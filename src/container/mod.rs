// SPDX-License-Identifier: MIT OR Apache-2.0

//! The containers: file structures that several RAW formats share.
//!
//! A container module knows the structure and nothing about camera vendors:
//! the TIFF module reads a TIFF file's structure, the JPEG module the
//! marker segments of a JPEG stream, an embedded preview's or a JPEG
//! file's, and a format module, such as the one for NEF files, decides what
//! that structure means.
//!
//! A file's container is recognised by its first bytes, and [`Structure`]
//! is the structure read from it: a JPEG file starts with SOI and a marker,
//! `FF D8 FF`, and a TIFF file with a byte-order mark, `II` or `MM`.
//! [`Structure::read`] asks the JPEG container first, then reads a TIFF
//! structure from byte 0, and refuses a file that starts neither way. The
//! formats that the container can hold are then asked whether the
//! structure is theirs.
//!
//! Each container is a folder, even when it starts as one file, so a
//! container that grows never moves.

pub(crate) mod jpeg;
pub(crate) mod tiff;

use std::io::{Read, Seek};

use self::jpeg::Segments;
use self::tiff::{Block, Tree, walk};
use crate::error::{Error, Result};
use crate::io::Source;
use crate::limits::Limits;

/// The structure of a file, as the container its first bytes name reads
/// it: what the formats that container can hold are asked about.
#[derive(Debug)]
pub(crate) enum Structure {
    /// A TIFF structure: the file's IFDs, walked from its header at byte 0.
    Tiff(Tree),
    /// A JPEG file's marker segments, as `jpeg::segments` read them.
    Jpeg(Segments),
}

impl Structure {
    /// Reads the structure of the file in `src`, by the container its first
    /// bytes name, under `limits`.
    ///
    /// A file that starts with SOI and a marker, `FF D8 FF`, is a JPEG
    /// file, and its segments are read; the JPEG container decides that,
    /// as [`jpeg::segments`] does not walk a file that starts otherwise. A
    /// file that starts with a byte-order mark, `II` or `MM`, is taken for
    /// a TIFF file, and walked from its header at byte 0: whatever follows
    /// the mark is the TIFF header's to judge, so that a file that only
    /// looks like TIFF is refused with the header's own reason.
    ///
    /// # Errors
    ///
    /// `UnknownFormat`, with no offset, when the file starts neither way, a
    /// file shorter than 2 bytes included; for a file that starts with a
    /// byte-order mark, whatever [`walk`] returns: `UnknownFormat` when the
    /// file is too short to hold a TIFF header, or the mark is not followed
    /// by the number 42, as in a BigTIFF file, and `Malformed` or
    /// `LimitExceeded` when the TIFF structure is damaged or crosses one of
    /// the `limits`; `Io` if the reader fails.
    pub(crate) fn read<R: Read + Seek>(src: &mut Source<R>, limits: &Limits) -> Result<Self> {
        if let Some(segments) = jpeg::segments(src)? {
            return Ok(Self::Jpeg(segments));
        }
        // Checked before reading, so that a file of 0 or 1 byte is of no
        // container rather than the `Malformed` of a read past the end.
        if src.len() >= 2 && matches!(src.bytes(0, 2)?.as_slice(), b"II" | b"MM") {
            let tree = walk(src, &Block::file(src.len()), limits)?;
            return Ok(Self::Tiff(tree));
        }
        Err(Error::unknown_format(
            "the file does not start as a TIFF or a JPEG file does",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::jpeg_builder::JpegBuilder;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{Order, TiffBuilder};
    use std::io::Cursor;

    /// The structure read from the file `bytes`, under the default limits.
    fn read(bytes: &[u8]) -> Result<Structure> {
        Structure::read(
            &mut Source::new(Cursor::new(bytes)).unwrap(),
            &Limits::default(),
        )
    }

    /// A TIFF file in byte order `order`, of one IFD that holds one entry,
    /// an Orientation.
    fn tiff_file(order: Order) -> TiffBuilder {
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder.short(ifd, 0x0112, &[1]);
        builder
    }

    /// The error reading `bytes` gives, which must be `UnknownFormat` with
    /// no offset, as its text.
    fn unknown(bytes: &[u8]) -> String {
        let err = read(bytes).expect_err("of no container");
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{err:?}");
        assert_eq!(err.offset(), None, "{err:?}");
        err.to_string()
    }

    /// What a file of neither container is refused with.
    const NEITHER: &str = "the file does not start as a TIFF or a JPEG file does";

    #[test]
    fn a_jpeg_file_is_read_as_its_segments() {
        let mut builder = JpegBuilder::new();
        builder.frame(640, 480, 0xC0);
        match read(&builder.build().bytes).unwrap() {
            Structure::Jpeg(segments) => {
                let frame = segments.frame.expect("the frame header");
                assert_eq!((frame.width, frame.height), (640, 480));
            }
            Structure::Tiff(_) => panic!("a JPEG file read as TIFF"),
        }
    }

    #[test]
    fn a_tiff_file_in_either_byte_order_is_read_as_its_tree() {
        for order in [Order::Little, Order::Big] {
            match read(&tiff_file(order).build().bytes).unwrap() {
                Structure::Tiff(tree) => assert_eq!(tree.len(), 1, "{order:?}"),
                Structure::Jpeg(_) => panic!("a TIFF file read as JPEG"),
            }
        }
    }

    #[test]
    fn a_bigtiff_file_is_refused_by_the_tiff_header() {
        let mut builder = tiff_file(Order::Little);
        builder.magic(43);
        let text = unknown(&builder.build().bytes);
        assert!(text.contains("BigTIFF"), "{text}");
    }

    #[test]
    fn a_mark_not_followed_by_42_is_refused_by_the_tiff_header() {
        let mut builder = tiff_file(Order::Big);
        builder.magic(41);
        let text = unknown(&builder.build().bytes);
        assert!(text.contains("not followed by the number 42"), "{text}");
    }

    #[test]
    fn a_byte_order_mark_alone_is_too_short_for_a_tiff_header() {
        for mark in [b"II", b"MM"] {
            let text = unknown(mark);
            assert!(text.contains("too short to hold a TIFF header"), "{text}");
        }
    }

    #[test]
    fn a_file_that_starts_neither_way_is_of_no_container() {
        for bytes in [
            &b""[..],
            b"I",
            &[0xFF],
            &[0xFF, 0xD8],
            &[0xFF, 0xD8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            b"IM\x2A\x00\x08\x00\x00\x00",
            b"not a tiff",
        ] {
            assert_eq!(unknown(bytes), NEITHER, "{bytes:02X?}");
        }
    }

    #[test]
    fn a_failing_reader_is_an_io_error() {
        // A JPEG file whose walk reads past `FAR` before its frame header:
        // two comments of 60,000 bytes after SOI.
        let big = vec![0x20; 60_000];
        let mut builder = JpegBuilder::new();
        builder.segment(0xFE, &big).segment(0xFE, &big);
        let jpeg = builder.build().bytes;
        // A TIFF file whose IFD0 lies past `FAR`, behind zeros.
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        builder
            .short(ifd, 0x0112, &[1])
            .prefix(ifd, &vec![0; usize::try_from(FAR).unwrap()]);
        let tiff = builder.build().bytes;
        for bytes in [jpeg, tiff] {
            assert!(u64::try_from(bytes.len()).unwrap() > FAR);
            let mut src = Source::new(FailsFar(Cursor::new(bytes.clone()))).unwrap();
            let err = Structure::read(&mut src, &Limits::default()).expect_err("the reader fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
            // Read without the failure, the same bytes are of a container.
            read(&bytes).unwrap();
        }
    }
}
