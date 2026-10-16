// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Sony ARW reader: how the crate recognises a Sony RAW file, and how
//! it finds the file's previews and reads its metadata.
//!
//! Sony's RAW files are one family. Every Sony body since 2006 writes ARW;
//! Sony's earliest bodies wrote SRF and SR2 files; and Sony's software
//! makes ARQ files, composites of several pixel-shift frames. All of them
//! are one TIFF container with Sony's additions inside: a MakerNote, a
//! private directory, compressed or encrypted raw data. The MakerNote's
//! FileFormat tag gives a version, but the version follows the firmware
//! rather than the body, and the previews are found the same way whatever
//! it says, so this reader never needs it.
//!
//! One more maker's name turns up in this family. Hasselblad, which makes
//! medium-format cameras with RAW formats of its own, also sold a few Sony
//! cameras under its own name, with a new body shell around Sony's
//! electronics: the Lunar is a Sony NEX-7 inside. Those cameras write
//! ordinary Sony ARW files, with Hasselblad's name in the Make tag.
//! Hasselblad's own cameras write files of other kinds with the same name
//! in the Make: 3FR and FFF files, which are TIFF-based, and, from the
//! camera modules it makes for DJI's drones, DNG files.
//!
//! For this reader a file is a Sony file when it is not a DNG and the Make
//! tag of its IFD0, which names the camera's manufacturer, names Sony; or
//! names Hasselblad, and IFD0 shows Sony's structure: the pointer to Sony's
//! private directory, in the shape Sony writes it. A DNG keeps the maker's
//! name in its Make, whether Adobe's converter made it from an ARW file or
//! one of Sony's phones wrote it, but it is Adobe's format, not Sony's.
//! Hasselblad's 3FR and FFF files carry no such pointer, and its DNG files
//! carry a DNG version, as far as the published samples show; [`claims`]
//! says what was measured. The file's name is never looked at.
//!
//! The search for the previews is in `previews.rs`. The metadata comes
//! from the standard tags, through the reader the TIFF-based formats share;
//! no Sony MakerNote is read.

mod previews;

use std::io::{Read, Seek};

use super::{FormatSpec, Parsed, is_dng, make_starts_with, tiff_metadata};
use crate::container::tiff::Tree;
use crate::container::tiff::tags::DNG_PRIVATE_DATA;
use crate::container::tiff::types::BYTE;
use crate::error::Result;
use crate::format::Format;
use crate::io::Source;
use crate::limits::Limits;

/// The manufacturer's name that a Sony camera's Make starts with, compared
/// without regard to case.
const SONY: &[u8; 4] = b"SONY";

/// The manufacturer's name that a Hasselblad camera's Make starts with,
/// compared without regard to case.
const HASSELBLAD: &[u8; 10] = b"HASSELBLAD";

/// The ARW format's entry in the format table.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R> {
    FormatSpec {
        format: Format::Arw,
        claims,
        parse,
    }
}

/// Whether the file whose TIFF structure is `tree` is a Sony RAW file:
/// whether it is not a DNG ([`is_dng`]), and the Make of its IFD0, trimmed
/// of spaces at either end, starts with `SONY`, in any case
/// ([`make_starts_with`]); or starts with `HASSELBLAD`, in any case, in an
/// IFD0 that holds Sony's private pointer (DNGPrivateData, as four bytes).
///
/// For a file that is not a DNG, a Sony Make decides alone. The ARW notes
/// (`docs/formats/arw/`) record `SONY` as the Make of every Sony body,
/// padded with eleven spaces on the A100 and with one on the A200 to A900,
/// which the trim covers. The SRF files of the earliest bodies carry no
/// private pointer, but their Make is Sony's too, so the pointer is not
/// asked of a Sony Make.
///
/// A Hasselblad Make appears here because Hasselblad sold a few Sony
/// cameras under its own name, rebuilt around Sony's electronics (the
/// Lunar is a NEX-7 inside), and those write ordinary Sony ARW files with
/// `HASSELBLAD` as the Make: the Lunar's file has Model `Lunar` and every
/// Sony structure, the pointer to Sony's private directory among them
/// (file 4051 of raw.pixls.us). The files of Hasselblad's own cameras
/// carry the same Make and must not be claimed as Sony's: its cameras
/// write 3FR and FFF files, which are TIFF-based, and the camera modules
/// it makes for DJI's drones write DNG.
///
/// The structure tells them apart, as the metadata dumps of the 24
/// Hasselblad-made files on raw.pixls.us show. None of the 11 3FR and 11
/// FFF files carries DNGPrivateData or DNGVersion; both DNG files, from
/// the L1D-20c and L2D-20c drone modules, carry both; and the Lunar's file
/// carries DNGPrivateData and no DNGVersion, as Sony's ARW files do. So a
/// Hasselblad Make is claimed only with the pointer present and the
/// version absent. The version's absence is the test this claim makes
/// first, for every Make, so the Hasselblad branch asks only for the
/// pointer. The dumps are exiv2's output, read in place of the files
/// themselves; the Lunar's dump lists its DNGPrivateData, so the dumps do
/// list the tag when a file has it.
///
/// The pointer must also have Sony's shape. Sony writes DNGPrivateData as
/// four bytes (type BYTE, count 4) holding the offset of its private
/// directory, whereas a DNG's DNGPrivateData is a block of a maker's own
/// data, of any length. Asking for that shape keeps out a Hasselblad file
/// that holds such a block but no DNG version, should one exist.
///
/// The rule rests on those 24 dumps, the Lunar's dump and the one Lunar
/// sample. A counterexample, such as a 3FR or FFF file with the pointer, a Hasselblad
/// DNG without a version, or a Sony-built Hasselblad without the pointer,
/// would call for checking it again.
///
/// # Errors
///
/// As for [`make_starts_with`].
fn claims<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<bool> {
    if is_dng(tree) {
        return Ok(false);
    }
    if make_starts_with(src, tree, SONY)? {
        return Ok(true);
    }
    // Sony's pointer is four bytes; a DNG's private data block is not. The
    // DNG version, the other half of the Hasselblad rule, was ruled out
    // above for every Make. The pointer is tested first because it costs
    // no read, and the Make was read once already.
    let sony_pointer = tree
        .ifd0()
        .entry(DNG_PRIVATE_DATA)
        .is_some_and(|entry| entry.typ() == BYTE && entry.count() == 4);
    Ok(sony_pointer && make_starts_with(src, tree, HASSELBLAD)?)
}

/// Reads the previews and the metadata of a file that [`claims`]
/// accepted.
///
/// The previews are the JPEGs that the IFDs of IFD0's chain declare, found
/// by the shared TIFF preview finder over the whole tree. The metadata is
/// what the shared TIFF metadata reader reads from the standard tags: the
/// camera, the capture time, the orientation, the exposure and the lens.
/// Neither takes limits of its own, so `_limits` goes unused here: the walk
/// that built `tree` applied them.
///
/// # Errors
///
/// As for [`previews::previews`] and [`tiff_metadata::read`]: `Io` when the
/// reader fails. Nothing the file holds is an error here.
fn parse<R: Read + Seek>(src: &mut Source<R>, tree: &Tree, _limits: &Limits) -> Result<Parsed> {
    Ok(Parsed {
        previews: previews::previews(src, tree)?,
        metadata: tiff_metadata::read(src, tree)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::{DNG_VERSION, MAKE};
    use crate::formats::testing::file;
    use crate::tiff_builder::{IfdId, TiffBuilder};

    /// Whether the ARW reader claims the file that `build` describes.
    fn claimed(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> bool {
        let (mut src, tree) = file(build);
        claims(&mut src, &tree).unwrap()
    }

    /// Whether the ARW reader claims a file whose Make is `make`, as ASCII.
    fn claimed_make(make: &str) -> bool {
        claimed(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        })
    }

    /// The pointer to Sony's private directory as Sony writes it: four
    /// bytes holding a little-endian offset.
    const PRIVATE_POINTER: [u8; 4] = [0x00, 0x10, 0x00, 0x00];

    /// Whether the ARW reader claims a file whose Make is `make`, with
    /// Sony's private pointer in IFD0 and no DNG version.
    fn claimed_make_with_pointer(make: &str) -> bool {
        claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, make)
                .byte(ifd, DNG_PRIVATE_DATA, &PRIVATE_POINTER);
        })
    }

    #[test]
    fn the_make_of_a_sony_body_is_claimed() {
        assert!(claimed_make("SONY"));
    }

    #[test]
    fn the_make_of_the_a100_padded_with_spaces_is_claimed() {
        // Eleven spaces after the name; the builder adds the NUL that makes
        // the value sixteen bytes.
        assert!(claimed_make("SONY           "));
    }

    #[test]
    fn a_hasselblad_make_with_sonys_private_pointer_is_claimed() {
        assert!(claimed_make_with_pointer("HASSELBLAD"));
    }

    #[test]
    fn a_hasselblad_make_in_mixed_case_with_the_pointer_is_claimed() {
        assert!(claimed_make_with_pointer("Hasselblad"));
    }

    #[test]
    fn a_hasselblad_make_without_the_pointer_is_not_claimed() {
        assert!(!claimed_make("HASSELBLAD"));
    }

    #[test]
    fn a_hasselblad_make_with_eight_bytes_of_private_data_is_not_claimed() {
        // A DNG's private data block, of a length Sony never writes.
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "HASSELBLAD")
                .byte(ifd, DNG_PRIVATE_DATA, &[0; 8]);
        }));
    }

    #[test]
    fn a_hasselblad_make_with_a_pointer_of_type_undefined_is_not_claimed() {
        // Four bytes, as Sony's pointer, but not of Sony's type BYTE.
        assert!(!claimed(|builder, ifd| {
            builder.ascii(ifd, MAKE, "HASSELBLAD");
            builder.undefined(ifd, DNG_PRIVATE_DATA, &PRIVATE_POINTER);
        }));
    }

    #[test]
    fn a_hasselblad_make_with_the_pointer_and_a_dng_version_is_not_claimed() {
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "HASSELBLAD")
                .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
                .byte(ifd, DNG_PRIVATE_DATA, &PRIVATE_POINTER);
        }));
    }

    #[test]
    fn the_make_of_a_nikon_body_is_not_claimed() {
        assert!(!claimed_make("NIKON CORPORATION"));
    }

    #[test]
    fn the_make_of_another_manufacturer_is_not_claimed() {
        assert!(!claimed_make("Canon"));
    }

    #[test]
    fn a_dng_from_a_sony_camera_is_not_claimed() {
        // As Adobe's converter writes it: Sony's Make, a DNG version and a
        // private data block.
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "SONY")
                .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
                .byte(ifd, DNG_PRIVATE_DATA, &[0; 32]);
        }));
    }

    #[test]
    fn a_claimed_file_without_a_jpeg_has_no_previews() {
        let (mut src, tree) = file(|builder, ifd| {
            builder.ascii(ifd, MAKE, "SONY");
        });
        assert!(claims(&mut src, &tree).unwrap());
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        assert!(parsed.previews.is_empty());
    }
}
