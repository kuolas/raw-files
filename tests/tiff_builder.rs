// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the TIFF fixture builder from outside the crate, as the
//! integration tests use it.
//!
//! The unit tests compile the builder into the library's test build; this
//! test compiles it through `tests/support/`, the copy the integration tests
//! share, so that copy is built and linted on every test run even before
//! another integration test uses it.

mod support;

use support::tiff_builder::{EXIF_IFD, Order, SUB_IFDS, TiffBuilder};

/// The little-endian 32-bit number at `at` of `bytes`.
fn u32_le(bytes: &[u8], at: u64) -> u32 {
    let at = usize::try_from(at).unwrap();
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn a_tiny_little_endian_file() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd = builder.ifd();
    builder.ascii(ifd, 0x010F, "Nikon").short(ifd, 0x0112, &[1]);
    #[rustfmt::skip]
    let expected = [
        b'I', b'I', 42, 0, 8, 0, 0, 0,                  // header
        2, 0,                                           // two entries
        0x0F, 0x01, 2, 0, 6, 0, 0, 0, 38, 0, 0, 0,      // Make, ASCII, 6, at 38
        0x12, 0x01, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0,       // Orientation, SHORT, 1, 1
        0, 0, 0, 0,                                     // no next IFD
        b'N', b'i', b'k', b'o', b'n', 0,                // Make's value
    ];
    assert_eq!(builder.build().bytes, expected);
}

#[test]
fn subifd_and_exif_offsets_resolve() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let (sub, exif) = (builder.ifd(), builder.ifd());
    builder.subifds(ifd0, &[sub]).exif(ifd0, exif);
    let built = builder.build();
    let bytes = &built.bytes;

    // IFD0 at 8 holds two entries, sorted: SubIFDs, then the Exif IFD, each
    // a single offset, inline.
    assert_eq!(bytes[8 + 2..8 + 4], SUB_IFDS.to_le_bytes());
    assert_eq!(u64::from(u32_le(bytes, 8 + 2 + 8)), built.ifd_offset(sub));
    assert_eq!(bytes[8 + 14..8 + 16], EXIF_IFD.to_le_bytes());
    assert_eq!(u64::from(u32_le(bytes, 8 + 14 + 8)), built.ifd_offset(exif));
    // The two empty IFDs follow IFD0, in the order they were created.
    assert_eq!(built.ifd_offset(sub), 8 + 2 + 24 + 4);
    assert_eq!(built.ifd_offset(exif), built.ifd_offset(sub) + 2 + 4);
}
