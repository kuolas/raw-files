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
//! - [`parse`] reads the file as every TIFF-based format does
//!   ([`parse_tiff`]), with [`nikon`], the reader every format whose files
//!   carry Nikon's MakerNote shares.
//! - `previews` says where a NEF keeps its previews.
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

use super::{FormatSpec, Parsed, is_dng, make_starts_with, parse_tiff};
use crate::container::tiff::Tree;
use crate::error::Result;
use crate::format::Format;
use crate::formats::makernote::{Reader, nikon};
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
/// not a DNG ([`is_dng`]), and its IFD0's Make starts with `NIKON`, the
/// Nikon reader's [`MAKE`](Reader::MAKE) ([`make_starts_with`]).
///
/// For a file that is not a DNG, Make alone decides. Nikon writes `NIKON`
/// on the Coolpix E5700 and P7800, and `NIKON CORPORATION` on every other
/// body checked: the prefix covers both
/// (`docs/formats/nef/makernote-and-metadata.md`, section 8.3). The
/// MakerNote's `Nikon` signature does not decide, as no MakerNote's look
/// does ([`Reader::MAKE`]). Neither does the file's name, nor the image
/// IFD0 describes, which differs between bodies.
///
/// # Errors
///
/// As for [`make_starts_with`].
fn claims<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<bool> {
    Ok(!is_dng(tree) && make_starts_with(src, tree, nikon::MakerNote::MAKE)?)
}

/// Reads the previews, the metadata and the entries of a NEF file that
/// [`claims`] accepted, as every TIFF-based format reads them
/// ([`parse_tiff`]), with the Nikon MakerNote reader. The MakerNote adds
/// the JPEG of its PreviewIFD to the previews, and its PreviewIFD to the
/// entries. A NEF has no private directory.
///
/// # Errors
///
/// `Io` when the reader fails.
fn parse<R: Read + Seek>(src: &mut Source<R>, tree: &Tree, limits: &Limits) -> Result<Parsed> {
    parse_tiff::<nikon::MakerNote, R>(src, tree, limits, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_settings::{
        CameraSettings, Contrast, FocusMode, Saturation, SceneCaptureType, Sharpness,
    };
    use crate::container::tiff::tags::{
        CONTRAST, DNG_PRIVATE_DATA, DNG_VERSION, MAKE, MAKER_NOTE, SATURATION, SCENE_CAPTURE_TYPE,
        SHARPNESS,
    };
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

    #[test]
    fn the_standard_processing_values_stand_beside_the_makernotes_settings() {
        // Exif's directions of contrast, saturation and sharpness and its
        // scene capture type beside a MakerNote whose FocusMode (0x0007)
        // says AF-C: the MakerNote's
        // reader adds its setting and leaves Exif's values as they are.
        let (mut src, tree) = file(|builder, ifd0| {
            let (exif, makernote) = (builder.ifd(), builder.ifd());
            builder
                .ascii(ifd0, MAKE, "NIKON CORPORATION")
                .exif(ifd0, exif)
                .short(exif, CONTRAST, &[1])
                .short(exif, SATURATION, &[2])
                .short(exif, SHARPNESS, &[2])
                .short(exif, SCENE_CAPTURE_TYPE, &[1])
                .ascii(makernote, 0x0007, "AF-C  ")
                .undefined_ifd(exif, MAKER_NOTE, makernote);
        });
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        let expected = CameraSettings::default()
            .with_focus_mode(FocusMode::Continuous)
            .with_contrast(Contrast::Soft)
            .with_saturation(Saturation::High)
            .with_sharpness(Sharpness::Hard)
            .with_scene_capture_type(SceneCaptureType::Landscape);
        assert_eq!(parsed.metadata.camera_settings, expected);
    }
}
