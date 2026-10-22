// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the stub JPEG from outside the crate, as the integration tests
//! use it.
//!
//! The unit tests compile the stub into the library's test build; this test
//! compiles it through `tests/support/`, the copy the integration tests
//! share, so that copy is built and linted on every test run even before
//! another integration test uses it.

mod support;

use support::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};

#[test]
fn a_stub_starts_ends_and_has_its_frame_header_where_documented() {
    let bytes = jpeg_stub(0x0102, 0x0304, 0xC0);
    assert_eq!(bytes.len(), 2 + 69 + 22 + 13 + 10 + 4 + 2);
    assert_eq!(bytes[..2], [0xFF, 0xD8]);
    #[rustfmt::skip]
    let frame = [
        0xFF, 0xC0, 0, 11,      // SOF0, length 11
        8,                      // precision
        0x03, 0x04,             // height
        0x01, 0x02,             // width
        1, 1, 0x11, 0,          // one component: id 1, 1x1, table 0
    ];
    assert_eq!(bytes[FRAME_HEADER_AT..FRAME_HEADER_AT + frame.len()], frame);
    // SOS follows the frame header.
    assert_eq!(bytes[FRAME_HEADER_AT + frame.len()..][..2], [0xFF, 0xDA]);
    assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9]);
}

#[test]
fn a_segment_length_counts_itself() {
    assert_eq!(segment(0xFE, b"hi"), [0xFF, 0xFE, 0, 4, b'h', b'i']);
    assert_eq!(segment(0xFE, &[]), [0xFF, 0xFE, 0, 2]);
}
