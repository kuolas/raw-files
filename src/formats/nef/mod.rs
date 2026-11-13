// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon NEF reader: how the crate recognizes a NEF file, and how it
//! finds the file's previews and reads its metadata.
//!
//! A NEF file is a TIFF file written by a Nikon camera, with Nikon's own
//! additions inside: a MakerNote, compressed raw data. The NRW files of
//! Coolpix cameras are of the same TIFF family, and are read the same way.
//!
//! The parts, in the order a file is read:
//!
//! - [`claims`] says whether a file is a NEF, by IFD0's Make.
//! - [`parse`] reads Nikon's MakerNote once, with [`nikon`], the reader
//!   every format whose files carry that MakerNote shares. Then it reads
//!   the previews, the metadata and the entries.
//! - `previews` finds the previews, with the MakerNote already read.
//!
//! Nothing the file holds is an error of this reader's. Damage costs only
//! what it touches ([`skip_damage`](crate::error::skip_damage)), and only
//! a failure of the reader is an error.
//!
//! The NEF notes say where a NEF keeps each of these
//! (`docs/formats/nef/container-and-previews.md` and
//! `docs/formats/nef/makernote-and-metadata.md`).

mod previews;

use std::io::{Read, Seek};

use super::{FormatSpec, Parsed, is_dng, make_starts_with, tiff_metadata};
use crate::container::tiff::Tree;
use crate::entries::Entries;
use crate::error::Result;
use crate::format::Format;
use crate::formats::makernote::nikon::{self, NIKON, Picture};
use crate::io::Source;
use crate::limits::Limits;

/// The NEF format's entry in the TIFF container's list of formats.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R, Tree> {
    FormatSpec {
        format: Format::Nef,
        claims,
        parse,
    }
}

/// Whether the file whose TIFF structure is `tree` is a NEF: whether it is
/// not a DNG ([`is_dng`]), and its IFD0's Make starts with `NIKON`
/// ([`make_starts_with`]).
///
/// For a file that is not a DNG, Make alone decides. Nikon writes `NIKON`
/// on the Coolpix E5700 and P7800, and `NIKON CORPORATION` on every other
/// body checked: the prefix covers both
/// (`docs/formats/nef/makernote-and-metadata.md`, section 8.3). The
/// MakerNote's `Nikon` signature does not decide: Nikon's editing software
/// writes Nikon-style MakerNotes into the files of other makes, and leaves
/// their Make as it was. Neither does the file's name, nor the image IFD0
/// describes, which differs between bodies.
///
/// # Errors
///
/// As for [`make_starts_with`].
fn claims<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<bool> {
    Ok(!is_dng(tree) && make_starts_with(src, tree, NIKON)?)
}

/// Reads the previews, the metadata and the entries of a NEF file that
/// [`claims`] accepted.
///
/// The MakerNote is read first, once for the whole file, with its
/// PreviewIFD, within `limits`. Then:
///
/// - **The previews** are the JPEGs that IFD0 and the SubIFDs declare,
///   and the one the MakerNote's PreviewIFD declares. The shared TIFF
///   preview finder finds those of IFD0 and the SubIFDs over the whole
///   tree.
/// - **The metadata** is what the shared TIFF metadata reader reads from
///   the standard tags. When there is a MakerNote, [`nikon::metadata`]
///   adds what it records and the standard tags lack, for a raw file's
///   picture ([`Picture::Raw`]).
/// - **The entries** are those of every directory of the tree, then the
///   MakerNote IFD's and its PreviewIFD's
///   ([`MakerNote::add_entries`](nikon::MakerNote::add_entries)).
///
/// # Errors
///
/// `Io` when the reader fails.
fn parse<R: Read + Seek>(src: &mut Source<R>, tree: &Tree, limits: &Limits) -> Result<Parsed> {
    let makernote = nikon::read(src, tree, limits)?;
    let previews = previews::previews(src, tree, makernote.as_ref())?;
    let standard = tiff_metadata::read(src, tree)?;
    let mut entries = Entries::of_tree(tree);
    let metadata = match &makernote {
        Some(makernote) => {
            makernote.add_entries(&mut entries);
            nikon::metadata(src, makernote, standard, Picture::Raw)?
        }
        None => standard.metadata,
    };
    Ok(Parsed {
        previews,
        metadata,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::{DNG_PRIVATE_DATA, DNG_VERSION, MAKE, MAKER_NOTE};
    use crate::entries::Directory;
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
    fn a_make_of_type_utf8_is_claimed() {
        // Exif 3.1 gives the Make as "ASCII or UTF-8".
        assert!(claimed(|builder, ifd| {
            builder.utf8(ifd, MAKE, "NIKON CORPORATION");
        }));
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

    #[test]
    fn the_entries_are_those_of_every_directory_of_the_tree() {
        let (mut src, tree) = file(|builder, ifd0| {
            let (exif, gps) = (builder.ifd(), builder.ifd());
            builder
                .ascii(ifd0, MAKE, "NIKON CORPORATION")
                .exif(ifd0, exif)
                .gps(ifd0, gps)
                .short(exif, 0x9207, &[5])
                .byte(gps, 0x0000, &[2, 3, 0, 0]);
        });
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        let entries = &parsed.entries;
        assert_eq!(
            entries.directories(),
            [Directory::Ifd(0), Directory::Exif, Directory::Gps]
        );
        assert!(entries.entry(Directory::Ifd(0), MAKE).is_some());
        assert!(entries.entry(Directory::Exif, 0x9207).is_some());
        assert!(entries.entry(Directory::Gps, 0x0000).is_some());
    }

    #[test]
    fn the_makernote_ifd_and_its_preview_ifd_follow_the_trees_directories() {
        // A MakerNote of the headerless layout, which the file's own builder
        // writes, its block running through its PreviewIFD.
        let (mut src, tree) = file(|builder, ifd0| {
            let (exif, gps, makernote, preview) =
                (builder.ifd(), builder.ifd(), builder.ifd(), builder.ifd());
            builder
                .ascii(ifd0, MAKE, "NIKON CORPORATION")
                .exif(ifd0, exif)
                .gps(ifd0, gps)
                .byte(gps, 0x0000, &[2, 3, 0, 0]);
            builder
                .short(makernote, 0x0002, &[0, 200])
                .ifd_pointer(makernote, 0x0011, preview)
                .short(preview, 0x0103, &[6]);
            let end = builder.blob(&[0; 4]);
            builder
                .block_through(makernote, end)
                .undefined_ifd(exif, MAKER_NOTE, makernote);
        });
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        let entries = &parsed.entries;
        assert_eq!(
            entries.directories(),
            [
                Directory::Ifd(0),
                Directory::Exif,
                Directory::Gps,
                Directory::MakerNote,
                Directory::MakerNoteSub(0x0011),
            ]
        );
        assert!(entries.entry(Directory::MakerNote, 0x0002).is_some());
        assert!(
            entries
                .entry(Directory::MakerNoteSub(0x0011), 0x0103)
                .is_some()
        );
    }
}
