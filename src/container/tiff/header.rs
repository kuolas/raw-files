// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF header: the byte order of the file and the offset of its first
//! directory, IFD0.

use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::io::{ByteOrder, Source};

/// The length of the TIFF header in bytes.
const HEADER_LEN: u64 = 8;

/// The number that follows the byte-order mark in a TIFF header.
const MAGIC: u16 = 42;

/// The number in its place in a BigTIFF header, a different container with
/// 64-bit offsets.
const BIGTIFF_MAGIC: u16 = 43;

/// The 8-byte header at the start of every TIFF file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    /// The order of the bytes in every multi-byte value of the file.
    pub order: ByteOrder,
    /// The offset of IFD0 from the start of the file, as the header gives it:
    /// not yet checked against anything.
    pub ifd0: u32,
}

/// Reads the TIFF header at the start of `src`.
///
/// The header is a byte-order mark, `II` for little-endian or `MM` for
/// big-endian; the number 42 in that order; and the offset of IFD0 in that
/// order. The offset is returned as it is: it may be zero, odd, inside the
/// header or past the end of the file, and whoever reads IFD0 decides
/// whether it can be read.
///
/// # Errors
///
/// `UnknownFormat` if the file is shorter than the header, starts with
/// neither byte-order mark, or has a number other than 42 after it, which
/// includes BigTIFF's 43; `Io` if the reader fails.
pub(crate) fn parse_header<R: Read + Seek>(src: &mut Source<R>) -> Result<Header> {
    // Checked here, before reading, so that a short file is "not a TIFF
    // file" rather than the `Malformed` of a read past the end.
    if src.len() < HEADER_LEN {
        return Err(Error::unknown_format(
            "the file is too short to hold a TIFF header",
        ));
    }
    let order = match src.bytes(0, 2)?.as_slice() {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => {
            return Err(Error::unknown_format(
                "the file does not start with a TIFF byte-order mark",
            ));
        }
    };
    match src.u16_at(2, order)? {
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
    let ifd0 = src.u32_at(4, order)?;
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
