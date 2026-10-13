// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF header: the byte order of the file and the offset of its first
//! directory, IFD0.
//!
//! A TIFF structure embedded in a file, such as the one inside a Nikon
//! MakerNote, starts with a header of its own, laid out the same way and
//! read at its own position; the offsets inside that structure count from
//! its header.

use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::io::{ByteOrder, Source, within};

/// The length of the TIFF header in bytes.
pub(super) const HEADER_LEN: u64 = 8;

/// The number that follows the byte-order mark in a TIFF header.
const MAGIC: u16 = 42;

/// The number in its place in a BigTIFF header, a different container with
/// 64-bit offsets.
const BIGTIFF_MAGIC: u16 = 43;

/// The 8-byte header at the start of every TIFF file, and of a TIFF
/// structure embedded in one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    /// The order of the bytes in every multi-byte value of the file, or of
    /// the embedded structure.
    pub order: ByteOrder,
    /// The offset of IFD0 from the start of the header, as the header gives
    /// it: not yet checked against anything. For a file's own header, which
    /// is at byte 0, that is the offset from the start of the file.
    pub ifd0: u32,
}

/// Reads the TIFF header at the start of `src`: [`parse_header_at`] at
/// byte 0.
///
/// # Errors
///
/// As for [`parse_header_at`].
pub(crate) fn parse_header<R: Read + Seek>(src: &mut Source<R>) -> Result<Header> {
    parse_header_at(src, 0)
}

/// Reads the TIFF header at byte `at` of `src`.
///
/// The header is a byte-order mark, `II` for little-endian or `MM` for
/// big-endian; the number 42 in that order; and the offset of IFD0 in that
/// order. The offset is returned as it is: it may be zero, odd, inside the
/// header or past the end of the file, and whoever reads IFD0 decides
/// whether it can be read.
///
/// A file's own header is at byte 0, which [`parse_header`] reads. A TIFF
/// header embedded inside the file, such as the one a Nikon MakerNote
/// carries, is read with this function at its own position; its
/// [`Header::ifd0`] then counts from `at`, not from the start of the file,
/// as every offset inside the embedded structure does.
///
/// # Errors
///
/// `UnknownFormat` if the file ends before the 8 bytes from `at` do, the
/// header starts with neither byte-order mark, or has a number other than
/// 42 after it, which includes BigTIFF's 43; `Io` if the reader fails.
pub(crate) fn parse_header_at<R: Read + Seek>(src: &mut Source<R>, at: u64) -> Result<Header> {
    // Checked here, before reading, so that a short file is "not a TIFF
    // file" rather than the `Malformed` of a read past the end. Once the
    // header's end is inside the file, so is every byte before it, and the
    // offsets below cannot overflow.
    if within(at, HEADER_LEN, src.len()).is_none() {
        return Err(Error::unknown_format(
            "the file is too short to hold a TIFF header",
        ));
    }
    let order = match src.bytes(at, 2)?.as_slice() {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => {
            return Err(Error::unknown_format(
                "the file does not start with a TIFF byte-order mark",
            ));
        }
    };
    match src.u16_at(at + 2, order)? {
        MAGIC => {}
        BIGTIFF_MAGIC => {
            return Err(Error::unknown_format(
                "the file is a BigTIFF file, not a TIFF file",
            ));
        }
        _ => {
            return Err(Error::unknown_format(
                "the TIFF byte-order mark is not followed by the number 42",
            ));
        }
    }
    let ifd0 = src.u32_at(at + 4, order)?;
    Ok(Header { order, ifd0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::tiff_builder::{IfdId, Order, TiffBuilder};
    use std::io::{self, Cursor, SeekFrom};

    fn parse(bytes: &[u8]) -> Result<Header> {
        parse_header(&mut Source::new(Cursor::new(bytes)).unwrap())
    }

    fn assert_unknown_format(bytes: &[u8]) -> Error {
        let err = parse(bytes).expect_err("not a TIFF header");
        assert_eq!(
            err.kind(),
            ErrorKind::UnknownFormat,
            "{err:?} for {bytes:02X?}"
        );
        assert_eq!(err.offset(), None, "{err:?}");
        err
    }

    /// A file with one IFD that holds one entry, and that IFD.
    fn one_ifd(order: Order) -> (TiffBuilder, IfdId) {
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder.short(ifd, 0x0112, &[1]);
        (builder, ifd)
    }

    #[test]
    fn reads_both_byte_orders() {
        for (order, expected) in [
            (Order::Little, ByteOrder::Little),
            (Order::Big, ByteOrder::Big),
        ] {
            let (mut builder, _) = one_ifd(order);
            // An offset whose four bytes all differ, so that reading it in
            // the wrong order would show.
            builder.ifd0_offset(Some(0x0102_0304));
            let built = builder.build();
            let header = parse(&built.bytes).unwrap();
            assert_eq!(
                header,
                Header {
                    order: expected,
                    ifd0: 0x0102_0304
                }
            );
        }
    }

    #[test]
    fn reads_a_header_at_an_offset_in_both_byte_orders() {
        for (order, expected) in [
            (Order::Little, ByteOrder::Little),
            (Order::Big, ByteOrder::Big),
        ] {
            let (mut builder, _) = one_ifd(order);
            builder.ifd0_offset(Some(0x0102_0304));
            // Ten bytes before the header, as a Nikon MakerNote has, the
            // first two of them a byte-order mark of the other order, so
            // that reading at the wrong position would show.
            let mut bytes = b"MMII\0\0\x02\x11\0\0".to_vec();
            bytes.extend_from_slice(&builder.build().bytes);
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let header = parse_header_at(&mut src, 10).unwrap();
            assert_eq!(
                header,
                Header {
                    order: expected,
                    ifd0: 0x0102_0304
                },
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_header_at_an_offset_must_end_inside_the_file() {
        let built = one_ifd(Order::Little).0.build();
        let mut bytes = vec![0; 10];
        bytes.extend_from_slice(&built.bytes[..8]);
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        parse_header_at(&mut src, 10).unwrap();
        // One byte later, the header would end past the file; at the
        // largest offset, its end overflows.
        for at in [11, u64::MAX] {
            let err = parse_header_at(&mut src, at).expect_err("no room for a header");
            assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{at}: {err:?}");
        }
    }

    #[test]
    fn ifd0_is_where_the_header_says() {
        for order in [Order::Little, Order::Big] {
            let (mut builder, ifd) = one_ifd(order);
            let built = builder.build();
            let ifd0 = u32::try_from(built.ifd_offset(ifd)).unwrap();
            assert_eq!(parse(&built.bytes).unwrap().ifd0, ifd0);

            // Not validated here: zero and past the end come back as they are.
            for offset in [0, u32::MAX] {
                builder.ifd0_offset(Some(offset));
                assert_eq!(parse(&builder.build().bytes).unwrap().ifd0, offset);
            }
        }
    }

    #[test]
    fn a_bad_byte_order_mark_is_not_a_tiff_file() {
        for mark in [*b"XX", *b"IM", *b"MI", *b"ii", [0, 0]] {
            let (mut builder, _) = one_ifd(Order::Little);
            builder.mark(mark);
            assert_unknown_format(&builder.build().bytes);
        }
    }

    #[test]
    fn a_magic_other_than_42_is_not_a_tiff_file() {
        for order in [Order::Little, Order::Big] {
            for magic in [0, 41, 0x2A00, u16::MAX] {
                let (mut builder, _) = one_ifd(order);
                builder.magic(magic);
                assert_unknown_format(&builder.build().bytes);
            }
        }
    }

    #[test]
    fn bigtiff_is_not_a_tiff_file() {
        for order in [Order::Little, Order::Big] {
            let (mut builder, _) = one_ifd(order);
            builder.magic(43);
            let err = assert_unknown_format(&builder.build().bytes);
            assert!(err.to_string().contains("BigTIFF"), "{err}");
        }
    }

    #[test]
    fn a_file_shorter_than_the_header_is_not_a_tiff_file() {
        for order in [Order::Little, Order::Big] {
            let built = one_ifd(order).0.build();
            for len in 0..8 {
                assert_unknown_format(&built.truncated(len));
            }
            // Eight bytes are enough: the header does not need IFD0.
            parse(&built.truncated(8)).unwrap();
        }
    }

    #[test]
    fn a_jpeg_is_not_a_tiff_file() {
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10, b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0,
        ];
        assert_unknown_format(&jpeg);
    }

    /// A reader that seeks, so that the source can be created, but fails
    /// every read.
    struct Unreadable;

    impl Read for Unreadable {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        }
    }

    impl Seek for Unreadable {
        fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
            Ok(100)
        }
    }

    #[test]
    fn a_reader_error_stays_an_io_error() {
        let mut src = Source::new(Unreadable).unwrap();
        let err = parse_header(&mut src).expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }
}
