// SPDX-License-Identifier: MIT OR Apache-2.0

//! The metadata reader shared by the TIFF-based formats: it reads what the
//! camera recorded about a picture from the standard TIFF and Exif tags.
//!
//! Cameras of every maker record the same facts in the same standard tags:
//! the camera's make and model and the orientation in IFD0, the capture
//! time and the body's serial number in the Exif IFD. So [`read`] reads them
//! once, over the tree, for every format, and a format adds only what its
//! own structures record and the standard tags lack (ADR011, which chose
//! the standard tags first).
//!
//! The rules:
//!
//! - A damaged value costs only itself. A tag that is missing, of a type
//!   other than the standard's, or whose value cannot be read, such as one
//!   longer than one tag may take, gives `None` for its own value; every
//!   other value is still read, and the file still opens. Only a failure of
//!   the reader is an error: it says nothing about the file.
//! - The make, the model and the serial number are text, read by the one
//!   rule [`text`] gives: cut at the first NUL, `None` when longer than
//!   255 bytes, trimmed of spaces at both ends, `None` when
//!   nothing is left, and with bytes that are not UTF-8 replaced by U+FFFD.
//! - The capture time is the Exif DateTimeOriginal, with its own fraction
//!   of a second (SubSecTimeOriginal) and offset from UTC
//!   (OffsetTimeOriginal), each of type ASCII; [`DateTime::from_exif`]
//!   checks them. Without a DateTimeOriginal there is no capture time,
//!   whatever fraction or offset the file records. It is never taken from
//!   DateTimeDigitized or from IFD0's DateTime, which editors change.
//! - A file without an Exif IFD has no capture time and no serial number.
//! - The orientation is IFD0's Orientation, read as the preview finder reads
//!   it ([`tiff_previews::orientation`]), so that the metadata and the
//!   previews never disagree.
//!
//! The exposure, the lens and the sizes are not read yet, and are left
//! empty (see #13).
//!
//! Nothing here panics on any input: every value is read through the IFD's
//! checked accessors, which check its offset and length against the file
//! and cap its size (ADR008).

use std::io::{Read, Seek};

use crate::container::tiff::tags::{
    BODY_SERIAL_NUMBER, DATE_TIME_ORIGINAL, MAKE, MODEL, OFFSET_TIME_ORIGINAL,
    SUB_SEC_TIME_ORIGINAL,
};
use crate::container::tiff::{Ifd, Tree, skip_damage};
use crate::datetime::DateTime;
use crate::error::Result;
use crate::formats::tiff_previews;
use crate::io::Source;
use crate::metadata::Metadata;

/// The longest text, in bytes as stored, that is taken for a value rather
/// than for damage.
///
/// The longest text value measured on the public sample files is 65 bytes,
/// a lens model, so a longer one than this is not a name or a serial
/// number a camera wrote.
const MAX_TEXT_LEN: usize = 255;

/// Reads the metadata of the TIFF-based file in `src`, whose IFDs are
/// `tree`.
///
/// See the [module documentation](self) for the rules.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
pub(crate) fn read<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Metadata> {
    let ifd0 = tree.ifd0();
    let (serial_number, capture_time) = match tree.exif() {
        Some(exif) => (
            text(src, exif, BODY_SERIAL_NUMBER)?,
            capture_time(src, exif)?,
        ),
        None => (None, None),
    };
    Ok(Metadata {
        make: text(src, ifd0, MAKE)?,
        model: text(src, ifd0, MODEL)?,
        serial_number,
        capture_time,
        orientation: tiff_previews::orientation(src, tree)?,
        ..Metadata::default()
    })
}

/// The text of tag `tag` of `ifd`, cleaned as every text value of the
/// metadata is, such as the camera's make and serial number.
///
/// The value must be of type ASCII. It ends at its first NUL, or with the
/// value when there is none. When what is before the NUL is longer than
/// [`MAX_TEXT_LEN`] bytes, counted as stored, before any trimming or
/// replacement, it is damage and `None`. Spaces at either end are padding
/// and are dropped, as in the DSLR-A100's Make, `SONY` and eleven spaces;
/// spaces inside the text, and any other blank such as a tab, are kept, as
/// the capture time's reading and the formats' Make test keep them. Text
/// that is empty once trimmed, as a value of NULs or of spaces only, is
/// `None`. Bytes that are not UTF-8 become U+FFFD.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason, such as one longer than one tag may take, is `None`.
fn text<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd, tag: u16) -> Result<Option<String>> {
    let Some(bytes) = skip_damage(ifd.ascii_bytes(src, tag))?.flatten() else {
        return Ok(None);
    };
    if bytes.len() > MAX_TEXT_LEN {
        return Ok(None);
    }
    let start = bytes.iter().position(|&b| b != b' ').unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| b != b' ')
        .map_or(start, |i| i + 1);
    let text = bytes.get(start..end).unwrap_or_default();
    Ok((!text.is_empty()).then(|| String::from_utf8_lossy(text).into_owned()))
}

/// When the picture was taken, from the Exif IFD `exif`: its
/// DateTimeOriginal, with its SubSecTimeOriginal and OffsetTimeOriginal
/// when the file records them, all three of type ASCII, as
/// [`DateTime::from_exif`] reads them.
///
/// `None` when there is no DateTimeOriginal that can be read, or when it
/// names no real date and time, such as an unset clock. A fraction of a
/// second or an offset that cannot be read is left out, and the time
/// stands without it.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn capture_time<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Option<DateTime>> {
    let Some(date) = skip_damage(exif.ascii_bytes(src, DATE_TIME_ORIGINAL))?.flatten() else {
        return Ok(None);
    };
    let subsec = skip_damage(exif.ascii_bytes(src, SUB_SEC_TIME_ORIGINAL))?.flatten();
    let offset = skip_damage(exif.ascii_bytes(src, OFFSET_TIME_ORIGINAL))?.flatten();
    Ok(DateTime::from_exif(
        &date,
        subsec.as_deref(),
        offset.as_deref(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::ORIENTATION;
    use crate::container::tiff::{parse_header, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing::{self, FAR, FailsFar};
    use crate::limits::Limits;
    use crate::orientation::Orientation;
    use crate::tiff_builder::{ASCII, IfdId, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    /// When the picture was stored or digitised (DateTimeDigitized), an
    /// Exif tag the reader must never take the capture time from.
    const DATE_TIME_DIGITIZED: u16 = 0x9004;

    /// When the file was last changed (DateTime), an IFD0 tag the reader
    /// must never take the capture time from.
    const DATE_TIME: u16 = 0x0132;

    /// The text values of a Z 9 file, each with its tag: IFD0's, then the
    /// Exif IFD's. The time is that of a public sample file; the serial
    /// number is made up.
    const IFD0_TEXT: [(u16, &str); 2] = [(MAKE, "NIKON CORPORATION"), (MODEL, "NIKON Z 9")];
    /// See [`IFD0_TEXT`].
    const EXIF_TEXT: [(u16, &str); 4] = [
        (BODY_SERIAL_NUMBER, "1234567"),
        (DATE_TIME_ORIGINAL, "2022:02:05 13:46:41"),
        (SUB_SEC_TIME_ORIGINAL, "85"),
        (OFFSET_TIME_ORIGINAL, "-05:00"),
    ];

    /// More bytes than the 1 MiB one tag value may take.
    const TOO_LONG: u32 = 1024 * 1024 + 1;

    /// A little-endian file whose IFD0 holds the Orientation 8 and
    /// [`IFD0_TEXT`], and whose Exif IFD holds [`EXIF_TEXT`], all but tag
    /// `except`. `change` is then given the builder and the IDs of IFD0 and
    /// of the Exif IFD, to add what the test needs in its place.
    fn z9_bytes_except(
        except: u16,
        change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
    ) -> Vec<u8> {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder.exif(ifd0, exif).short(ifd0, ORIENTATION, &[8]);
        for (ifd, values) in [(ifd0, &IFD0_TEXT[..]), (exif, &EXIF_TEXT[..])] {
            for &(tag, text) in values.iter().filter(|(tag, _)| *tag != except) {
                builder.ascii(ifd, tag, text);
            }
        }
        change(&mut builder, ifd0, exif);
        builder.build().bytes
    }

    /// The metadata read from `bytes`; the reader must succeed.
    fn read_bytes(bytes: Vec<u8>) -> Metadata {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
        read(&mut src, &tree).unwrap()
    }

    /// The metadata of [`z9_bytes_except`]'s file.
    fn z9_except(except: u16, change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId)) -> Metadata {
        read_bytes(z9_bytes_except(except, change))
    }

    /// The metadata of the Z 9 file whole: tag 0, which it does not hold,
    /// is the one left out.
    fn z9_file() -> Metadata {
        z9_except(0, |_, _, _| {})
    }

    /// The capture time of the Z 9 file, with the fraction of a second and
    /// the offset from UTC each when `Some`.
    fn z9_time(subsec: Option<&[u8]>, offset: Option<&[u8]>) -> Option<DateTime> {
        DateTime::from_exif(b"2022:02:05 13:46:41", subsec, offset)
    }

    /// What the Z 9 file gives, every value read.
    fn z9_expected() -> Metadata {
        Metadata {
            make: Some("NIKON CORPORATION".to_owned()),
            model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("1234567".to_owned()),
            capture_time: z9_time(Some(b"85"), Some(b"-05:00")),
            orientation: Orientation::Rotate270,
            ..Metadata::default()
        }
    }

    /// The make read from a file whose IFD0's Make is `make`, as ASCII with
    /// the NUL the builder adds.
    fn make(make: &str) -> Option<String> {
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        });
        read(&mut src, &tree).unwrap().make
    }

    #[test]
    fn the_z9_file_gives_every_value_it_records() {
        let metadata = z9_file();
        assert_eq!(metadata, z9_expected());
        let time = metadata.capture_time().unwrap();
        assert_eq!(time.to_string(), "2022-02-05T13:46:41.85-05:00");
        assert_eq!(time.offset_minutes(), Some(-300));
        assert_eq!(time.nanosecond(), 850_000_000);
        // Not read yet.
        assert_eq!(metadata.exposure(), &crate::metadata::Exposure::default());
        assert_eq!(metadata.lens(), None);
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), None);
    }

    #[test]
    fn a_make_padded_with_spaces_is_trimmed() {
        // The DSLR-A100's: `SONY` and eleven spaces.
        assert_eq!(make("SONY           ").as_deref(), Some("SONY"));
        assert_eq!(
            make("  NIKON CORPORATION ").as_deref(),
            Some("NIKON CORPORATION")
        );
    }

    #[test]
    fn only_spaces_are_padding() {
        // A tab or a line break is not padding, at an end or inside.
        assert_eq!(make(" \tSONY\r\n ").as_deref(), Some("\tSONY\r\n"));
    }

    #[test]
    fn spaces_inside_a_make_are_kept() {
        assert_eq!(
            make("NIKON  CORPORATION").as_deref(),
            Some("NIKON  CORPORATION")
        );
    }

    #[test]
    fn a_make_is_cut_at_its_first_nul() {
        // A space, then a NUL, then bytes past it, then the builder's NUL.
        assert_eq!(make("SONY \0rest of the value").as_deref(), Some("SONY"));
    }

    #[test]
    fn a_make_of_nul_bytes_or_spaces_only_is_none() {
        assert_eq!(make("\0\0\0"), None);
        assert_eq!(make("    "), None);
        assert_eq!(make(""), None);
    }

    #[test]
    fn a_model_of_256_bytes_is_refused_and_one_of_255_kept() {
        let model = |len: usize| {
            z9_except(MODEL, |builder, ifd0, _| {
                builder.ascii(ifd0, MODEL, &"M".repeat(len));
            })
        };
        assert_eq!(model(255).model, Some("M".repeat(255)));
        // Too long, and costs only the model.
        assert_eq!(
            model(256),
            Metadata {
                model: None,
                ..z9_expected()
            }
        );
    }

    #[test]
    fn the_length_limit_counts_the_padding() {
        // 256 bytes, which trimming would make 4.
        let padded = format!("SONY{}", " ".repeat(252));
        assert_eq!(make(&padded), None);
    }

    /// The make of a little-endian file whose IFD0's Make is `bytes`, of
    /// type ASCII, stored at offset 4096.
    fn make_bytes(bytes: &[u8]) -> Option<String> {
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.blob_at(4096, bytes);
            let count = u32::try_from(bytes.len()).unwrap();
            builder.raw(ifd, MAKE, ASCII, count, RawValue::Offset(4096));
        });
        read(&mut src, &tree).unwrap().make
    }

    #[test]
    fn bytes_that_are_not_utf8_become_the_replacement_character() {
        assert_eq!(make_bytes(b"SO\xFFNY\0").as_deref(), Some("SO\u{FFFD}NY"));
    }

    #[test]
    fn the_length_limit_counts_the_bytes_stored_not_the_text_they_become() {
        // 200 bytes that are not UTF-8: 600 bytes of text once each is
        // replaced, yet within the limit as stored.
        let mut bytes = vec![0xFF; 200];
        bytes.push(0);
        assert_eq!(make_bytes(&bytes), Some("\u{FFFD}".repeat(200)));
    }

    #[test]
    fn a_date_time_original_of_type_undefined_is_ignored() {
        let metadata = z9_except(DATE_TIME_ORIGINAL, |builder, _, exif| {
            builder.undefined(exif, DATE_TIME_ORIGINAL, b"2022:02:05 13:46:41\0");
        });
        assert_eq!(
            metadata,
            Metadata {
                capture_time: None,
                ..z9_expected()
            }
        );
    }

    #[test]
    fn sub_seconds_without_a_date_time_original_give_no_time() {
        // The Z 9 file without its DateTimeOriginal, which keeps its
        // SubSecTimeOriginal and its OffsetTimeOriginal; then a file whose
        // Exif IFD holds the SubSecTimeOriginal alone.
        let metadata = z9_except(DATE_TIME_ORIGINAL, |_, _, _| {});
        assert_eq!(metadata.capture_time(), None);
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            builder.ascii(exif, SUB_SEC_TIME_ORIGINAL, "85");
        });
        assert_eq!(read(&mut src, &tree).unwrap().capture_time(), None);
    }

    #[test]
    fn an_offset_without_a_date_time_original_gives_no_time() {
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            builder.ascii(exif, OFFSET_TIME_ORIGINAL, "-05:00");
        });
        assert_eq!(read(&mut src, &tree).unwrap().capture_time(), None);
    }

    #[test]
    fn the_time_is_never_taken_from_date_time_digitized_or_ifd0s_date_time() {
        let metadata = z9_except(DATE_TIME_ORIGINAL, |builder, ifd0, exif| {
            builder.ascii(exif, DATE_TIME_DIGITIZED, "2022:02:05 13:46:41");
            builder.ascii(ifd0, DATE_TIME, "2022:02:05 13:46:41");
        });
        assert_eq!(metadata.capture_time(), None);
    }

    #[test]
    fn a_file_without_an_exif_ifd_has_no_time_or_serial_but_its_camera_and_orientation() {
        // The Exif tags in IFD0, where they do not belong, are not read.
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "SONY")
                .ascii(ifd, MODEL, "DSC-F828")
                .short(ifd, ORIENTATION, &[6])
                .ascii(ifd, BODY_SERIAL_NUMBER, "1234567")
                .ascii(ifd, DATE_TIME_ORIGINAL, "2022:02:05 13:46:41");
        });
        assert!(tree.exif().is_none());
        let metadata = read(&mut src, &tree).unwrap();
        assert_eq!(
            metadata,
            Metadata {
                make: Some("SONY".to_owned()),
                model: Some("DSC-F828".to_owned()),
                orientation: Orientation::Rotate90,
                ..Metadata::default()
            }
        );
    }

    #[test]
    fn the_orientation_is_the_one_the_previews_take() {
        for (value, orientation) in [
            (8, Orientation::Rotate270),
            (6, Orientation::Rotate90),
            (1, Orientation::NoTransforms),
            // No EXIF orientation: none.
            (9, Orientation::NoTransforms),
        ] {
            let (mut src, tree) = testing::file(|builder, ifd| {
                builder.short(ifd, ORIENTATION, &[value]);
            });
            let metadata = read(&mut src, &tree).unwrap();
            assert_eq!(metadata.orientation(), orientation, "{value}");
            assert_eq!(
                tiff_previews::orientation(&mut src, &tree).unwrap(),
                orientation,
                "{value}"
            );
        }
    }

    /// Gives `ifd` tag `tag` as ASCII with [`TOO_LONG`] bytes at offset
    /// 4096, and makes the file long enough to hold them: a value that lies
    /// inside the file and cannot be read.
    fn too_long(builder: &mut TiffBuilder, ifd: IfdId, tag: u16) {
        builder.raw(ifd, tag, ASCII, TOO_LONG, RawValue::Offset(4096));
        builder.pad_to(4096 + u64::from(TOO_LONG));
    }

    #[test]
    fn a_damaged_value_costs_only_itself() {
        type Change = fn(&mut TiffBuilder, IfdId, IfdId);
        let cases: [(&str, u16, Change, Metadata); 7] = [
            (
                "a Model that is not text",
                MODEL,
                |builder, ifd0, _| {
                    builder.byte(ifd0, MODEL, b"NIKON Z 9\0");
                },
                Metadata {
                    model: None,
                    ..z9_expected()
                },
            ),
            (
                "a Make too long to read",
                MAKE,
                |builder, ifd0, _| too_long(builder, ifd0, MAKE),
                Metadata {
                    make: None,
                    ..z9_expected()
                },
            ),
            (
                "a serial number of 256 bytes",
                BODY_SERIAL_NUMBER,
                |builder, _, exif| {
                    builder.ascii(exif, BODY_SERIAL_NUMBER, &"1".repeat(256));
                },
                Metadata {
                    serial_number: None,
                    ..z9_expected()
                },
            ),
            (
                "a DateTimeOriginal too long to read",
                DATE_TIME_ORIGINAL,
                |builder, _, exif| too_long(builder, exif, DATE_TIME_ORIGINAL),
                Metadata {
                    capture_time: None,
                    ..z9_expected()
                },
            ),
            (
                "an impossible date",
                DATE_TIME_ORIGINAL,
                |builder, _, exif| {
                    builder.ascii(exif, DATE_TIME_ORIGINAL, "2022:02:30 13:46:41");
                },
                Metadata {
                    capture_time: None,
                    ..z9_expected()
                },
            ),
            (
                "sub-seconds that are not text",
                SUB_SEC_TIME_ORIGINAL,
                |builder, _, exif| {
                    builder.undefined(exif, SUB_SEC_TIME_ORIGINAL, b"85\0");
                },
                Metadata {
                    capture_time: z9_time(None, Some(b"-05:00")),
                    ..z9_expected()
                },
            ),
            (
                "an offset too long to read",
                OFFSET_TIME_ORIGINAL,
                |builder, _, exif| too_long(builder, exif, OFFSET_TIME_ORIGINAL),
                Metadata {
                    capture_time: z9_time(Some(b"85"), None),
                    ..z9_expected()
                },
            ),
        ];
        for (case, tag, change, expected) in cases {
            assert_eq!(z9_except(tag, change), expected, "{case}");
        }
    }

    #[test]
    fn a_failure_of_the_reader_is_the_readers_error() {
        let far = u32::try_from(2 * FAR).unwrap();
        for (tag, text) in IFD0_TEXT.into_iter().chain(EXIF_TEXT) {
            // The value stored far away, padded with NULs so that it is
            // longer than four bytes and so not held in its entry.
            let mut value = text.as_bytes().to_vec();
            value.extend_from_slice(&[0; 8]);
            let count = u32::try_from(value.len()).unwrap();
            let in_ifd0 = IFD0_TEXT.iter().any(|&(t, _)| t == tag);
            let bytes = z9_bytes_except(tag, |builder, ifd0, exif| {
                builder.blob_at(2 * FAR, &value);
                let ifd = if in_ifd0 { ifd0 } else { exif };
                builder.raw(ifd, tag, ASCII, count, RawValue::Offset(far));
            });
            // The file is sound, for a reader that does not fail.
            assert_eq!(read_bytes(bytes.clone()), z9_expected(), "{tag:#06X}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{tag:#06X}: {err:?}");
        }
    }
}
