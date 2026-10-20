// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon NEF reader: how the crate recognises a NEF file, and how it
//! finds the file's previews and reads its metadata.
//!
//! A NEF file is a TIFF file written by a Nikon camera, with Nikon's own
//! additions inside: a MakerNote, compressed raw data. For this reader a
//! file is a NEF when the Make tag of its IFD0, which names the camera's
//! manufacturer, names Nikon, and the file is not a DNG: a DNG converted
//! from a NEF keeps Nikon's name in its Make, but it is Adobe's format. The
//! NRW files of Coolpix cameras are the same TIFF family and are recognised
//! the same way. The file's name is never looked at, and neither is the
//! image IFD0 describes, which differs between bodies and does not tell a
//! Nikon file from another.
//!
//! The reader of Nikon's MakerNote is in `makernote.rs`: it reads the
//! MakerNote once per file, finds the small preview the MakerNote points
//! at, and reads what the MakerNote records that the standard tags lack:
//! the visible image size, the focus point, and, on the older bodies whose
//! standard tags lack them, the serial number, the ISO, the lens's range
//! and the capture time's offset from UTC, from the time-zone setting of
//! the camera's clock. The search for the previews is in `previews.rs`,
//! and takes the MakerNote already read. The metadata comes from the
//! standard tags, through the reader the TIFF-based formats share, and then
//! from that MakerNote.

mod makernote;
mod previews;

use std::io::{Read, Seek};

use super::{FormatSpec, Parsed, is_dng, make_starts_with, tiff_metadata};
use crate::container::tiff::Tree;
use crate::error::Result;
use crate::format::Format;
use crate::io::Source;
use crate::limits::Limits;

/// The manufacturer's name that a Nikon camera's Make starts with,
/// compared without regard to case.
const NIKON: &[u8; 5] = b"NIKON";

/// The NEF format's entry in the format table.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R> {
    FormatSpec {
        format: Format::Nef,
        claims,
        parse,
    }
}

/// Whether the file whose TIFF structure is `tree` is a NEF: whether it is
/// not a DNG ([`is_dng`]), and the Make of its IFD0, trimmed of spaces at
/// either end, starts with `NIKON`, in any case ([`make_starts_with`]).
///
/// For a file that is not a DNG, Make alone decides. The NEF notes
/// (`docs/formats/nef/`) record `NIKON CORPORATION` as the Make of the
/// DSLRs and `NIKON` as that of the Coolpix compacts, and the public sample
/// collection they cite files the Z bodies, and a recent Coolpix, under
/// `NIKON CORPORATION`: the manufacturer's name comes first in each, so the
/// prefix covers them all.
/// The notes' own test for a Nikon MakerNote compares Make the same way.
/// The MakerNote's `Nikon` signature does not decide, because Nikon's
/// editing software writes Nikon-style MakerNotes into the files of other
/// makes, whose Make it leaves as it was.
///
/// # Errors
///
/// As for [`make_starts_with`].
fn claims<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<bool> {
    Ok(!is_dng(tree) && make_starts_with(src, tree, NIKON)?)
}

/// Reads the previews and the metadata of a NEF file that [`claims`]
/// accepted.
///
/// The MakerNote is read first, once for the whole file. The previews are
/// the JPEGs that IFD0 and the SubIFDs declare, found by the shared TIFF
/// preview finder over the whole tree, and the one the MakerNote points
/// at; the MakerNote and its preview are read within `limits`. The
/// metadata is what the shared TIFF metadata reader reads from the
/// standard tags: the camera, the capture time, the orientation, the
/// exposure, the lens and the sizes. When there is a MakerNote, its
/// CropArea then gives the visible image size, unless the raw IFD records
/// a default crop, and it gives the serial number, the ISO and the lens's
/// range when the standard tags gave none, as on bodies before the D780
/// and the Z 6, and the capture time's offset from UTC, from its WorldTime,
/// when the standard tags gave a capture time without one, as on bodies
/// before the D850; and it gives the focus point, from its AFInfo2, which
/// no standard tag records ([`makernote::metadata`]).
///
/// # Errors
///
/// As for [`makernote::read`], [`previews::previews`],
/// [`tiff_metadata::read`] and [`makernote::metadata`]: `Io` when the
/// reader fails. Nothing the file holds is an error here.
fn parse<R: Read + Seek>(src: &mut Source<R>, tree: &Tree, limits: &Limits) -> Result<Parsed> {
    let makernote = makernote::read(src, tree, limits)?;
    let previews = previews::previews(src, tree, makernote.as_ref(), limits)?;
    let standard = tiff_metadata::read(src, tree)?;
    let metadata = match &makernote {
        Some(makernote) => makernote::metadata(src, makernote, standard)?,
        None => standard.metadata,
    };
    Ok(Parsed { previews, metadata })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::{DNG_PRIVATE_DATA, DNG_VERSION, MAKE};
    use crate::formats::testing::file;
    use crate::tiff_builder::{IfdId, TiffBuilder};

    /// Whether the NEF reader claims the file that `build` describes.
    fn claimed(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> bool {
        let (mut src, tree) = file(build);
        claims(&mut src, &tree).unwrap()
    }

    /// Whether the NEF reader claims a file whose Make is `make`, as ASCII.
    fn claimed_make(make: &str) -> bool {
        claimed(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        })
    }

    #[test]
    fn the_make_of_a_nikon_dslr_is_claimed() {
        assert!(claimed_make("NIKON CORPORATION"));
    }

    #[test]
    fn the_make_of_a_nikon_compact_is_claimed() {
        assert!(claimed_make("NIKON"));
    }

    #[test]
    fn the_make_of_another_manufacturer_is_not_claimed() {
        assert!(!claimed_make("Canon"));
    }

    #[test]
    fn a_dng_from_a_nikon_camera_is_not_claimed() {
        // As Adobe's converter writes it: Nikon's Make, a DNG version and a
        // private data block.
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "NIKON CORPORATION")
                .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
                .byte(ifd, DNG_PRIVATE_DATA, &[0; 32]);
        }));
    }

    #[test]
    fn a_claimed_file_without_a_jpeg_has_no_previews() {
        let (mut src, tree) = file(|builder, ifd| {
            builder.ascii(ifd, MAKE, "NIKON CORPORATION");
        });
        assert!(claims(&mut src, &tree).unwrap());
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        assert!(parsed.previews.is_empty());
    }
}
