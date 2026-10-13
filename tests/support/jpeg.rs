// SPDX-License-Identifier: MIT OR Apache-2.0

//! A stub JPEG for tests: a small, well-formed JPEG stream with no image in
//! it.
//!
//! The preview finder reads a JPEG only to its frame header, the segment
//! that gives the image's size; it never decodes. A test needs a stream
//! whose segments are laid out as a real encoder lays them out, with a
//! frame header of a chosen size and marker, and does not need pixels: the
//! stub is that stream, and [`segment`] writes one more segment for a test
//! that wants to add its own.
//!
//! The layout, in order, every number big-endian:
//!
//! - SOI, the start of the image: `FF D8`;
//! - a DQT segment, one quantization table: `FF DB`, length 67, table 0,
//!   then 64 entries of 1;
//! - a DHT segment, one Huffman table holding a single one-bit code: `FF
//!   C4`, length 20, class and table 0, 16 counts (one code of length 1,
//!   none longer), then the code's one symbol, 0;
//! - the frame header, at [`FRAME_HEADER_AT`]: `FF`, the given marker,
//!   length 11, precision 8, the height, the width, and one component (id
//!   1, sampling 1×1, quantization table 0);
//! - an SOS segment, the start of the scan: `FF DA`, length 8, one
//!   component (id 1, Huffman tables 0 and 0), spectral selection 0 to 63
//!   and successive approximation 0;
//! - 4 bytes standing for the entropy-coded data, none of them `FF`;
//! - EOI, the end of the image: `FF D9`.
//!
//! This file uses only `std`, because it is compiled twice: into the
//! library's unit tests and, through `tests/support/`, into the integration
//! tests.

/// Where the frame header's marker starts in a stub: after the 2 bytes of
/// SOI, the 69 of the DQT segment and the 22 of the DHT segment.
pub const FRAME_HEADER_AT: usize = 2 + 69 + 22;

/// A stub JPEG whose frame header has marker `marker` and gives the image
/// as `width` by `height` pixels; see the [module documentation](self) for
/// the layout.
///
/// Any marker and any size are written as given, so a test can build a
/// frame header that is no frame header, or one with a zero dimension.
pub fn jpeg_stub(width: u16, height: u16, marker: u8) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];

    let mut dqt = vec![0];
    dqt.extend_from_slice(&[1; 64]);
    bytes.extend(segment(0xDB, &dqt));

    let mut dht = vec![0, 1];
    dht.extend_from_slice(&[0; 15]);
    dht.push(0);
    bytes.extend(segment(0xC4, &dht));

    let mut frame = vec![8];
    frame.extend_from_slice(&height.to_be_bytes());
    frame.extend_from_slice(&width.to_be_bytes());
    frame.extend_from_slice(&[1, 1, 0x11, 0]);
    bytes.extend(segment(marker, &frame));

    bytes.extend(segment(0xDA, &[1, 1, 0x00, 0, 63, 0]));
    bytes.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// A marker segment: `FF`, `marker`, the big-endian length, which counts
/// its own 2 bytes and the payload's, then `payload`.
///
/// Panics if the payload is longer than a segment can hold, 65,533 bytes: a
/// mistake in the test.
pub fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload
        .len()
        .checked_add(2)
        .and_then(|len| u16::try_from(len).ok())
        .expect("a segment's payload is at most 65,533 bytes");
    let mut bytes = vec![0xFF, marker];
    bytes.extend_from_slice(&len.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
