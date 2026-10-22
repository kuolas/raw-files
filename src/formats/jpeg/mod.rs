// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG reader: how the crate recognises a JPEG file, and how it finds
//! the file's previews and reads its orientation and image size.
//!
//! A JPEG file is a picture as a camera writes it beside its raw file or
//! instead of it, or as an editor exports it: one JPEG stream, with the
//! camera's metadata in an Exif segment before the picture's frame header,
//! and often smaller copies of the picture stored after its end and listed
//! in an MPF segment. raw-files opens it as a format of its own, through
//! the same handle and into the same types as a raw file (ADR014). It holds
//! no raw data.
//!
//! The JPEG container has walked the file's segments up to its scan and
//! kept its first frame header, its first Exif segment and its first MPF
//! segment ([`Segments`]). For this reader a file is a JPEG when that frame
//! header gives a size, neither side zero, and is not lossless
//! ([`claims`]): a lossless frame keeps every sample value exactly, which is
//! how some raw files store their raw data, not a picture to show (T.81
//! Table B.1, p. 32). Nothing else decides: a file without an Exif segment,
//! or whose Exif segment is damaged, is a JPEG all the same, and the file's
//! name is never looked at.
//!
//! # The previews
//!
//! - **The file itself**: its whole byte range, exactly as stored, with the
//!   size its frame header gives. A viewer opens the whole file, and a
//!   decoder stops at the main image's end marker, so the MPF images and
//!   any padding after it, such as the zeros some Sony bodies write, do no
//!   harm (CIPA DC-008 4.7, p. 99). It is built from the frame header the
//!   claim checked, not probed again. No other preview starts at the
//!   file's first byte: the thumbnail lies inside the Exif segment, and an
//!   MPF image after the main image.
//! - **The thumbnail in IFD1** of the Exif segment's TIFF structure, which
//!   the shared TIFF preview finder (`tiff_previews`) finds as it finds a
//!   raw file's: counted from that structure's header, and inside the
//!   segment (DC-008 4.5.8, pp. 25 to 26).
//! - **Each image the MPF list holds after the first**, at the MP header's
//!   position plus the offset its entry gives, over the size its entry
//!   gives (CIPA DC-007 5.2.3.3.3, p. 16), whatever the type of image the
//!   entry names. These images lie after the main image, outside the
//!   segment that lists them, so they are bounded by the file alone
//!   ([`mpf::images`]).
//!
//! The thumbnail and the MPF images are checked as every preview is, by
//! the shared finder's [`check`](tiff_previews::check): each must lie
//! inside the file, start with SOI and hold, within its own bytes, a frame
//! header that gives a size and is not lossless, and is measured from that
//! frame header. Every preview, the file included, takes the orientation of
//! IFD0 of the Exif segment, or none without one.
//!
//! # Damage
//!
//! The Exif segment is read as a bounded block, whose TIFF header is the
//! base of every offset. A TIFF header or an IFD0 that cannot be read
//! gives no tree: it costs the orientation and the thumbnail, never the
//! file, which opens with its own preview and its MPF images, listed in a
//! segment of their own. Past IFD0, damage costs only what it touches, as
//! in a raw file, and an MPF list that does not hold together costs only
//! its images. Only a failure of the reader is an error.
//!
//! # The metadata
//!
//! Only the orientation and the image size are read. The image size is
//! the size in the frame header, the picture's own (T.81 B.2.2 and Figure
//! B.3, p. 35); the Exif pixel dimensions are not read, since an editor may
//! leave them unchanged. A JPEG holds no raw data, so it has no raw size.

use std::io::{Read, Seek};

use super::tiff_previews::{self, Candidate};
use super::{FormatSpec, JPEG_UNCLAIMED, Parsed};
use crate::container::jpeg::{Frame, Segments, mpf};
use crate::container::tiff::{skip_damage, walk};
use crate::error::{Error, Result};
use crate::format::Format;
use crate::geometry::Size;
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::Metadata;
use crate::orientation::Orientation;
use crate::preview::{Preview, PreviewFormat};

/// The JPEG format's entry in the JPEG container's list of formats.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R, Segments> {
    FormatSpec {
        format: Format::Jpeg,
        claims,
        parse,
    }
}

/// The frame header of the picture the file holds: the first frame header
/// of the file, when it gives a size and is not lossless; `None` otherwise.
///
/// [`segments`](crate::container::jpeg::segments) keeps a frame header
/// only when neither side is zero, so a frame header that gives no size
/// never reaches here.
fn picture(segments: &Segments) -> Option<Frame> {
    segments.frame.filter(|frame| !frame.is_lossless())
}

/// Whether the file whose segments are `segments` is a JPEG file: whether
/// its first frame header gives a size and is not lossless ([`picture`]).
///
/// It reads nothing: the container read the frame header already.
///
/// # Errors
///
/// None. It returns a `Result` because every format's claim does.
fn claims<R: Read + Seek>(_: &mut Source<R>, segments: &Segments) -> Result<bool> {
    Ok(picture(segments).is_some())
}

/// Reads the previews and the metadata of a JPEG file that [`claims`]
/// accepted.
///
/// The Exif segment's TIFF structure, when there is one, is walked within
/// `limits`, and gives the orientation and the thumbnail's candidate; the
/// MPF list, read within `limits` as well, gives a candidate for each image
/// after the first. The shared finder checks them all, and the file itself
/// joins them as a preview of its own, with its frame header's size. The
/// metadata is the orientation and that size; see the
/// [module documentation](self).
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the file holds is an error here.
/// `UnknownFormat`, with the message detection gives a JPEG file no format
/// claims, for segments that [`claims`] refuses, which detection never
/// passes here.
fn parse<R: Read + Seek>(
    src: &mut Source<R>,
    segments: &Segments,
    limits: &Limits,
) -> Result<Parsed> {
    let Some(frame) = picture(segments) else {
        return Err(Error::unknown_format(JPEG_UNCLAIMED));
    };
    // A TIFF header or an IFD0 that cannot be read costs the tree, not the
    // file.
    let tree = match &segments.exif {
        Some(exif) => skip_damage(walk(src, exif, limits))?,
        None => None,
    };
    let (orientation, mut candidates) = match &tree {
        Some(tree) => (
            tiff_previews::orientation(src, tree)?,
            tiff_previews::tree_candidates(src, tree)?,
        ),
        None => (Orientation::NoTransforms, Vec::new()),
    };
    if let Some(index) = &segments.mpf {
        let images = mpf::images(src, index, limits)?;
        candidates.extend(images.into_iter().filter_map(|image| {
            Some(Candidate {
                offset: image.start,
                length: image.end.checked_sub(image.start)?,
            })
        }));
    }
    let mut previews = tiff_previews::check(src, &candidates, orientation)?;

    let size = Size {
        width: u32::from(frame.width),
        height: u32::from(frame.height),
    };
    previews.push(Preview::new(
        0,
        src.len(),
        size.width,
        size.height,
        PreviewFormat::Jpeg,
        orientation,
    ));
    let metadata = Metadata {
        orientation,
        image_size: Some(size),
        ..Metadata::default()
    };
    Ok(Parsed { previews, metadata })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::jpeg::segments;
    use crate::error::ErrorKind;
    use crate::jpeg_builder::JpegBuilder;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::Order;
    use std::io::Cursor;

    /// The bytes of the file `builder` describes, with its segments as the
    /// JPEG container reads them.
    fn read(builder: &JpegBuilder) -> (Vec<u8>, Segments) {
        let bytes = builder.build().bytes;
        let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
        let found = segments(&mut src).unwrap().expect("a JPEG file");
        (bytes, found)
    }

    /// Whether the JPEG reader claims the file `builder` describes.
    fn claimed(builder: &JpegBuilder) -> bool {
        let (bytes, found) = read(builder);
        claims(&mut Source::new(Cursor::new(bytes)).unwrap(), &found).unwrap()
    }

    /// A file whose main image has the frame header marker `marker`.
    fn with_marker(marker: u8) -> JpegBuilder {
        let mut builder = JpegBuilder::new();
        builder.frame(640, 480, marker);
        builder
    }

    #[test]
    fn a_frame_header_that_gives_a_size_is_claimed() {
        for marker in [0xC0, 0xC1, 0xC2] {
            assert!(claimed(&with_marker(marker)), "{marker:02X}");
        }
    }

    #[test]
    fn a_lossless_frame_header_is_not_claimed() {
        for marker in [0xC3, 0xC7, 0xCB, 0xCF] {
            assert!(!claimed(&with_marker(marker)), "{marker:02X}");
        }
    }

    #[test]
    fn a_file_without_a_frame_header_before_its_scan_is_not_claimed() {
        // SOS before the main image's frame header: the walk ends there.
        let mut builder = JpegBuilder::new();
        builder.segment(0xDA, &[1, 1, 0x00, 0, 63, 0]);
        assert!(!claimed(&builder));
    }

    #[test]
    fn parse_refuses_what_the_claim_refuses() {
        let (bytes, lossless) = read(&with_marker(0xC3));
        for found in [Segments::default(), lossless] {
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            let Err(err) = parse(&mut src, &found, &Limits::default()) else {
                panic!("parsed segments the claim refuses: {found:?}");
            };
            assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{err:?}");
            assert_eq!(err.to_string(), JPEG_UNCLAIMED);
        }
    }

    #[test]
    fn a_failing_reader_is_an_io_error() {
        // The Exif segment past `FAR`, behind a comment: walking its TIFF
        // structure fails.
        let tiff = [b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut exif = JpegBuilder::new();
        exif.segment(0xFE, &[0x20; 60_000])
            .segment(0xFE, &[0x20; 60_000])
            .exif(&tiff);
        // An MPF image past `FAR`, behind zeros: probing it fails.
        let mut mpf = JpegBuilder::new();
        mpf.mpf(Order::Little)
            .after_eoi(&vec![0; usize::try_from(FAR).unwrap()])
            .mpf_image(640, 424, &[], 1);
        for (case, builder) in [("Exif", exif), ("MPF", mpf)] {
            let (bytes, found) = read(&builder);
            assert!(u64::try_from(bytes.len()).unwrap() > FAR, "{case}");
            // Read without the failure, the same bytes give their previews.
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            parse(&mut src, &found, &Limits::default()).unwrap();

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let Err(err) = parse(&mut src, &found, &Limits::default()) else {
                panic!("{case}: the reader fails");
            };
            assert_eq!(err.kind(), ErrorKind::Io, "{case}: {err:?}");
        }
    }
}
