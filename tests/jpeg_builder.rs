// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the JPEG file builder from outside the crate, as the integration
//! tests use it.
//!
//! The unit tests compile the builder into the library's test build; this
//! test compiles it through `tests/support/`, the copy the integration tests
//! share, so that copy is built and linted on every test run even before
//! another integration test uses it.

mod support;

use support::jpeg_builder::{EXIF_ID, JFIF, JpegBuilder, XMP_ID};
use support::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};

/// The bytes of `bytes` in `range`.
fn at(bytes: &[u8], range: std::ops::Range<u64>) -> &[u8] {
    let start = usize::try_from(range.start).unwrap();
    let end = usize::try_from(range.end).unwrap();
    &bytes[start..end]
}

#[test]
fn an_empty_builder_writes_the_stub() {
    let built = JpegBuilder::new().build();
    assert_eq!(built.bytes, jpeg_stub(640, 480, 0xC0));
    assert_eq!(
        built.frame_header(),
        u64::try_from(FRAME_HEADER_AT).unwrap()
    );
}

#[test]
fn segments_follow_soi_in_the_order_they_were_added() {
    let tiff = [b'M', b'M', 0, 42, 0, 0, 0, 8];
    let mut builder = JpegBuilder::new();
    builder
        .jfif()
        .exif(&tiff)
        .xmp("<x/>")
        .app2(b"MPF\0abc")
        .segment(0xFE, b"hi")
        .fill(2)
        .marker(0xD0)
        .raw(&[0xFF, 0x01])
        .frame(0x0102, 0x0304, 0xC2)
        .after_eoi(b"next")
        .after_eoi(b"last")
        .pad(3);
    let built = builder.build();
    let bytes = &built.bytes;

    let mut expected = vec![0xFF, 0xD8];
    expected.extend(segment(0xE0, &JFIF));
    let exif_at = expected.len();
    let mut exif = EXIF_ID.to_vec();
    exif.extend_from_slice(&tiff);
    expected.extend(segment(0xE1, &exif));
    let mut xmp = XMP_ID.to_vec();
    xmp.extend_from_slice(b"<x/>");
    expected.extend(segment(0xE1, &xmp));
    let app2_at = expected.len();
    expected.extend(segment(0xE2, b"MPF\0abc"));
    expected.extend(segment(0xFE, b"hi"));
    expected.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xD0, 0xFF, 0x01]);
    let main_at = expected.len();
    expected.extend_from_slice(&jpeg_stub(0x0102, 0x0304, 0xC2)[2..]);
    let next_at = expected.len();
    expected.extend_from_slice(b"nextlast\0\0\0");
    assert_eq!(*bytes, expected);

    // The JFIF segment's length counts itself and its 14 bytes.
    assert_eq!(bytes[2..6], [0xFF, 0xE0, 0, 16]);
    assert_eq!(bytes[6..11], *b"JFIF\0");
    // The Exif segment: marker, length 2 + 6 + 8, the identifier, then the
    // TIFF structure where the builder says.
    let exif_at = u64::try_from(exif_at).unwrap();
    assert_eq!(at(bytes, exif_at..exif_at + 4), [0xFF, 0xE1, 0, 16]);
    assert_eq!(at(bytes, exif_at + 4..exif_at + 10), *b"Exif\0\0");
    assert_eq!(built.exif_tiff(0), exif_at + 10..exif_at + 18);
    assert_eq!(at(bytes, built.exif_tiff(0)), tiff);
    // The APP2 segment's payload, as given.
    let app2_at = u64::try_from(app2_at).unwrap();
    assert_eq!(built.app2_payload(0), app2_at + 4..app2_at + 11);
    assert_eq!(at(bytes, built.app2_payload(0)), *b"MPF\0abc");
    // The main image's frame header, big-endian height then width.
    let frame = built.frame_header();
    assert_eq!(frame, u64::try_from(main_at + FRAME_HEADER_AT - 2).unwrap());
    assert_eq!(
        at(bytes, frame..frame + 9),
        [0xFF, 0xC2, 0, 11, 8, 0x03, 0x04, 0x01, 0x02]
    );
    // The blocks after EOI, then the padding.
    let next_at = u64::try_from(next_at).unwrap();
    assert_eq!(at(bytes, next_at - 2..next_at), [0xFF, 0xD9]);
    assert_eq!(built.after_eoi(0), next_at);
    assert_eq!(built.after_eoi(1), next_at + 4);
    assert_eq!(built.truncated(3), [0xFF, 0xD8, 0xFF]);
}

#[test]
fn each_exif_and_app2_segment_is_reported() {
    let mut builder = JpegBuilder::new();
    builder.exif(b"II").app2(b"a").exif(b"MM").app2(b"bc");
    let built = builder.build();
    // SOI, then segments of 4 + 8, 4 + 1, 4 + 8 and 4 + 2 bytes.
    assert_eq!(built.exif_tiff(0), 12..14);
    assert_eq!(built.app2_payload(0), 18..19);
    assert_eq!(built.exif_tiff(1), 29..31);
    assert_eq!(built.app2_payload(1), 35..37);
}
