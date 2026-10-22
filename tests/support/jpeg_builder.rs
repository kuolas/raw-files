// SPDX-License-Identifier: MIT OR Apache-2.0

//! A builder for small JPEG files, for tests.
//!
//! A JPEG file, as a camera writes it, is one JPEG stream: a picture, with
//! the camera's metadata in application segments before the picture's
//! frame header, and sometimes more images after the end of the first.
//! Tests describe the segments they need, and the builder writes the bytes
//! and says where the parts a reader looks for landed. It writes what it is
//! told, in the order it is told, so a test can also write a file no camera
//! would: two Exif segments, fill bytes or a stand-alone marker anywhere, a
//! segment with any marker, bytes that are no segment at all.
//!
//! The layout, in order:
//!
//! - SOI, the start of the image: `FF D8`;
//! - the parts added with these methods, in the order they were added:
//!   - [`JpegBuilder::jfif`]: an APP0 segment, `FF E0`, holding the
//!     smallest JFIF header: `JFIF` and a zero byte, version 1.02, no
//!     units, a density of 1 by 1, and no thumbnail;
//!   - [`JpegBuilder::exif`]: an APP1 segment, `FF E1`, holding `Exif`, two
//!     zero bytes, and then the given TIFF structure, whose first byte is
//!     its TIFF header;
//!   - [`JpegBuilder::xmp`]: an APP1 segment holding the XMP namespace,
//!     `http://ns.adobe.com/xap/1.0/` and a zero byte, then the given text;
//!   - [`JpegBuilder::app2`]: an APP2 segment, `FF E2`, holding the given
//!     payload as it is;
//!   - [`JpegBuilder::segment`]: a segment with any marker and payload;
//!   - [`JpegBuilder::fill`]: `FF` fill bytes, which the next marker
//!     follows;
//!   - [`JpegBuilder::marker`]: a marker alone, `FF` and a code, with no
//!     length;
//!   - [`JpegBuilder::raw`]: any bytes, as they are;
//! - the main image: the stub JPEG of `tests/support/jpeg_stub.rs` without its
//!   SOI, that is its two tables, its frame header with the size and marker
//!   set by [`JpegBuilder::frame`] (640 by 480 and baseline, `C0`, by
//!   default), its SOS segment, 4 bytes of data and EOI, `FF D9`;
//! - the blocks added with [`JpegBuilder::after_eoi`], such as other
//!   images, in the order they were added;
//! - the zeros added with [`JpegBuilder::pad`], as some cameras pad their
//!   files after the last image.
//!
//! A segment is written by the stub's `segment`: `FF`, the marker, the
//! 2-byte length, which counts itself and the payload, then the payload.
//! Every multi-byte number of the JPEG structure is big-endian; the TIFF
//! structure inside an Exif segment is written as the test gives it.
//!
//! [`Built`] holds the bytes and where things landed: each Exif segment's
//! TIFF structure, each APP2 segment's payload, the main image's frame
//! header and each block after its EOI.
//!
//! The stub and `segment` come from `tests/support/jpeg_stub.rs`, this
//! module's sibling under one name in both compilations: in
//! `tests/support/` for the integration tests, and at the root of the
//! crate for the unit tests.
//!
//! This file uses only `std`, because it is compiled twice: into the
//! library's unit tests and, through `tests/support/`, into the integration
//! tests.

use std::ops::Range;

use super::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};

/// The marker of APP0, the JFIF segment.
pub const APP0: u8 = 0xE0;
/// The marker of APP1, the Exif and XMP segments.
pub const APP1: u8 = 0xE1;
/// The marker of APP2, the MPF segment among others.
pub const APP2: u8 = 0xE2;

/// What a JFIF segment holds after its marker and length: `JFIF` and a
/// zero byte, version 1.02, units 0 (none), a density of 1 by 1, and a
/// thumbnail of 0 by 0 pixels.
pub const JFIF: [u8; 14] = *b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0";

/// What an Exif segment's payload starts with: `Exif` and two zero bytes.
pub const EXIF_ID: [u8; 6] = *b"Exif\0\0";

/// What an XMP segment's payload starts with: its namespace and a zero
/// byte.
pub const XMP_ID: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

/// Which position of a segment the builder reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Report {
    /// None.
    Nothing,
    /// Where its TIFF structure lies, after the Exif identifier.
    Exif,
    /// Where its payload lies.
    App2,
}

/// One part added before the main image.
#[derive(Clone, Debug)]
enum Part {
    /// A segment: its marker, its payload, and what to report of it.
    Segment(u8, Vec<u8>, Report),
    /// Bytes written as they are.
    Bytes(Vec<u8>),
}

/// Builds a JPEG file; see the [module documentation](self) for the
/// layout.
///
/// The methods that add to the file return `&mut Self`, so calls can be
/// chained. A method that writes a segment panics on a payload longer than
/// a segment can hold, 65,533 bytes, as `segment` does: a mistake in the
/// test.
#[derive(Clone, Debug)]
pub struct JpegBuilder {
    parts: Vec<Part>,
    width: u16,
    height: u16,
    marker: u8,
    after_eoi: Vec<Vec<u8>>,
    pad: usize,
}

impl Default for JpegBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JpegBuilder {
    /// A file of SOI and the main image, a baseline frame of 640 by 480.
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            width: 640,
            height: 480,
            marker: 0xC0,
            after_eoi: Vec::new(),
            pad: 0,
        }
    }

    /// Gives the main image's frame header marker `marker` and the size
    /// `width` by `height`, written as given: a zero side, or a marker that
    /// is no frame marker, included.
    pub fn frame(&mut self, width: u16, height: u16, marker: u8) -> &mut Self {
        self.width = width;
        self.height = height;
        self.marker = marker;
        self
    }

    /// Adds an APP0 segment holding the smallest JFIF header, [`JFIF`].
    pub fn jfif(&mut self) -> &mut Self {
        self.add(APP0, JFIF.to_vec(), Report::Nothing)
    }

    /// Adds an APP1 segment holding [`EXIF_ID`] and then `tiff`, a TIFF
    /// structure; [`Built::exif_tiff`] says where `tiff` landed.
    pub fn exif(&mut self, tiff: &[u8]) -> &mut Self {
        let mut payload = EXIF_ID.to_vec();
        payload.extend_from_slice(tiff);
        self.add(APP1, payload, Report::Exif)
    }

    /// Adds an APP1 segment holding [`XMP_ID`] and then `text`.
    pub fn xmp(&mut self, text: &str) -> &mut Self {
        let mut payload = XMP_ID.to_vec();
        payload.extend_from_slice(text.as_bytes());
        self.add(APP1, payload, Report::Nothing)
    }

    /// Adds an APP2 segment holding `payload` as it is;
    /// [`Built::app2_payload`] says where it landed.
    pub fn app2(&mut self, payload: &[u8]) -> &mut Self {
        self.add(APP2, payload.to_vec(), Report::App2)
    }

    /// Adds a segment with marker `marker` holding `payload`.
    pub fn segment(&mut self, marker: u8, payload: &[u8]) -> &mut Self {
        self.add(marker, payload.to_vec(), Report::Nothing)
    }

    /// Adds `count` fill bytes, `FF` each, before what comes next.
    pub fn fill(&mut self, count: usize) -> &mut Self {
        self.raw(&vec![0xFF; count])
    }

    /// Adds the marker `code` alone: `FF` and `code`, with no length.
    pub fn marker(&mut self, code: u8) -> &mut Self {
        self.raw(&[0xFF, code])
    }

    /// Adds `bytes` as they are.
    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.parts.push(Part::Bytes(bytes.to_vec()));
        self
    }

    /// Adds `bytes` after the main image's EOI, after the blocks added
    /// before; [`Built::after_eoi`] says where they landed.
    pub fn after_eoi(&mut self, bytes: &[u8]) -> &mut Self {
        self.after_eoi.push(bytes.to_vec());
        self
    }

    /// Adds `count` zero bytes at the end of the file.
    pub fn pad(&mut self, count: usize) -> &mut Self {
        self.pad = self
            .pad
            .checked_add(count)
            .expect("the padding fits in memory");
        self
    }

    /// Writes the file.
    pub fn build(&self) -> Built {
        let mut bytes = vec![0xFF, 0xD8];
        let mut exif = Vec::new();
        let mut app2 = Vec::new();
        for part in &self.parts {
            match part {
                Part::Bytes(raw) => bytes.extend_from_slice(raw),
                Part::Segment(marker, payload, report) => {
                    // After the marker and the length.
                    let payload_at = to_u64(bytes.len()) + 4;
                    bytes.extend(segment(*marker, payload));
                    let end = to_u64(bytes.len());
                    match report {
                        Report::Nothing => {}
                        Report::Exif => exif.push(payload_at + to_u64(EXIF_ID.len())..end),
                        Report::App2 => app2.push(payload_at..end),
                    }
                }
            }
        }
        // The stub's frame header is FRAME_HEADER_AT bytes into it, SOI
        // included, and its SOI is left out here.
        let frame_header = to_u64(bytes.len() + FRAME_HEADER_AT - 2);
        bytes.extend_from_slice(&jpeg_stub(self.width, self.height, self.marker)[2..]);
        let mut after_eoi = Vec::new();
        for block in &self.after_eoi {
            after_eoi.push(to_u64(bytes.len()));
            bytes.extend_from_slice(block);
        }
        bytes.resize(bytes.len() + self.pad, 0);
        Built {
            bytes,
            exif,
            app2,
            frame_header,
            after_eoi,
        }
    }

    /// Adds a segment.
    fn add(&mut self, marker: u8, payload: Vec<u8>, report: Report) -> &mut Self {
        self.parts.push(Part::Segment(marker, payload, report));
        self
    }
}

/// A file written by [`JpegBuilder::build`], with where its parts landed.
///
/// The methods that take `n` count from 0, in the order the parts were
/// added, and panic when there are not that many: a mistake in the test.
#[derive(Clone, Debug)]
pub struct Built {
    /// The file.
    pub bytes: Vec<u8>,
    exif: Vec<Range<u64>>,
    app2: Vec<Range<u64>>,
    frame_header: u64,
    after_eoi: Vec<u64>,
}

impl Built {
    /// Where the TIFF structure of the `n`th Exif segment lies: from its
    /// TIFF header, 6 bytes into the segment's payload, to the end of the
    /// segment.
    pub fn exif_tiff(&self, n: usize) -> Range<u64> {
        self.exif[n].clone()
    }

    /// Where the payload of the `n`th segment added with
    /// [`JpegBuilder::app2`] lies: from the byte after its length to the end
    /// of the segment.
    pub fn app2_payload(&self, n: usize) -> Range<u64> {
        self.app2[n].clone()
    }

    /// Where the main image's frame header starts: the `FF` of its marker.
    pub fn frame_header(&self) -> u64 {
        self.frame_header
    }

    /// Where the `n`th block added with [`JpegBuilder::after_eoi`] starts.
    pub fn after_eoi(&self, n: usize) -> u64 {
        self.after_eoi[n]
    }

    /// The first `len` bytes of the file, which must be at most its length.
    pub fn truncated(&self, len: usize) -> Vec<u8> {
        self.bytes[..len].to_vec()
    }
}

/// `n` as a `u64`, which it always fits in where a test runs.
fn to_u64(n: usize) -> u64 {
    u64::try_from(n).expect("a length fits in a u64")
}
