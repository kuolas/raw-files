// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG container: the marker segments of a JPEG stream, as far as
//! raw-files reads them. Nothing here decodes an image.
//!
//! A JPEG stream is a picture compressed as the JPEG standard, ITU-T T.81,
//! describes, and raw-files meets it in two places. RAW files carry their
//! previews as JPEG streams, often several, from a thumbnail to a full-size
//! image. And a JPEG file, the picture a camera writes beside its raw file
//! or instead of it, is one such stream, with the camera's metadata inside
//! it, which raw-files is to open as a format of its own, its metadata and
//! previews read as a raw file's are (ADR014). Both are read by walking the
//! stream's segments up to its compressed image:
//!
//! - [`probe`] sizes a preview. The tags that describe a preview are often
//!   missing, zero or wrong about its size, so the preview finder asks each
//!   candidate itself: `probe` finds the frame header in the bytes the tags
//!   give and reads the image's width and height from it.
//! - [`segments`] reads a JPEG file: it keeps the file's frame header, its
//!   Exif segment and its MPF segment, the parts a reader of the file needs.
//! - [`mpf::images`] reads the MPF segment's index: where the images the
//!   file holds after its first one lie, such as its large thumbnails.
//!
//! # Marker segments
//!
//! A JPEG stream is a sequence of markers, each the byte `FF` followed by a
//! code that says what comes next (T.81 B.1.1.2, p. 31).
//!
//! - Any number of `FF` fill bytes may come before a marker's code (T.81
//!   B.1.1.2, p. 31).
//! - A few markers stand alone, with nothing after them: SOI (`D8`), the
//!   start of the image, EOI (`D9`), its end, the restart markers RST0 to
//!   RST7 (`D0` to `D7`) and TEM (`01`) (T.81 B.1.1.3 and Table B.1,
//!   pp. 31 to 32).
//! - Every other marker starts a segment: a 2-byte big-endian length,
//!   which counts itself and the payload after it but not the marker, so
//!   that it is at least 2 (T.81 B.1.1.4, p. 33), and a payload of at most
//!   65,533 bytes (T.81 Table B.9, p. 44).
//!
//! A stream opens with SOI, then holds tables, application data such as
//! Exif, and comments, then the frame header (SOF), which gives the
//! image's size and how it is coded, then the scan, an SOS segment followed
//! by the compressed image data, and ends with EOI (T.81 B.2.1 and Figure
//! B.2, pp. 34 to 35).
//!
//! # The walk
//!
//! Both readers walk the segments from the marker after SOI, with one
//! walk, `Walk`, which yields each segment that has a length, with its
//! marker and where its payload lies. It takes one step per marker or fill
//! byte and gives up after [`MAX_STEPS`] of them, so it ends on any input.
//! It ends as well, keeping what came before for whoever reads it, at:
//!
//! - SOS. What follows it is the compressed image, not a sequence of
//!   segments: the coder stuffs a `00` after every `FF` byte it produces
//!   (T.81 B.1.1.5, note 2, p. 33), and restart markers may come between
//!   its parts, so an `FF` byte there can look like a marker without being
//!   one. A frame header that has not come by then cannot be found by
//!   walking;
//! - EOI, the end of the stream;
//! - a byte that is not a marker where a marker should be, a stuffed
//!   `FF 00` included, since the walk has then left the sequence of
//!   segments, and a length read after it could send it anywhere;
//! - a length below 2, which cannot count itself and would send the walk
//!   back or keep it in place;
//! - a segment that runs past the end of the bytes walked.
//!
//! The walk reads only inside the bytes it is given, never past their end,
//! and every step moves it forward, with checked arithmetic.
//!
//! # The frame header
//!
//! After its marker and its length, the frame header holds the sample
//! precision, 1 byte, then the image's height and width, 2 bytes each
//! (T.81 B.2.2 and Figure B.3, p. 35; Table B.2, p. 36). A height of 0 is
//! legal, meaning that a DNL segment after the first scan gives it (T.81
//! p. 36; B.2.5, p. 45), but it gives no size to a reader that stops at
//! SOS; a width of 0 is never legal. Either gives no [`Frame`].
//!
//! The frame header's marker says how the image is coded, and a lossless
//! frame (`C3`, `C7`, `CB`, `CF`; T.81 Table B.1, p. 32) is never a
//! preview: Sony stores the raw sensor data of its lossless ARW files as
//! tiles that are lossless JPEG streams, each starting `FF D8 FF C3`, and a
//! strip that points at the first tile looks to the tags much like a
//! preview. The walk reports every frame header as it finds it; the frame
//! says whether it is lossless, so that the reader can drop it.
//!
//! # A JPEG file
//!
//! A JPEG file starts with SOI and a marker, `FF D8 FF`, and [`segments`]
//! walks it from there to the end of the file, keeping three things:
//!
//! - **The first frame header**, the main image's, read as [`probe`]
//!   reads it. When it gives no size, none is kept, and a later frame
//!   header is not looked at.
//! - **The first Exif segment**: the first APP1 segment (`E1`) whose
//!   payload starts with `Exif` and two zero bytes (CIPA DC-008 4.5.4 and
//!   Figure 7, pp. 20 to 21; 4.7.2.3 and Figure 36, p. 105). A whole TIFF
//!   structure follows them, the camera's metadata, whose offsets count
//!   from its own header, the first byte after the identifier (DC-008
//!   4.6.2, p. 28; 4.7.2.5, p. 106), and all of which lies inside the
//!   segment (ADR014). It is kept as a [`Block`]: its base is that TIFF
//!   header, and its bounds run from there to the end of the segment. A
//!   later Exif segment is skipped, as XMP and every other application
//!   segment are.
//! - **The first MPF segment**: the first APP2 segment (`E2`) whose payload
//!   starts with `MPF` and a zero byte (CIPA DC-007 5.2.1 and Figure 4,
//!   pp. 10 to 11). An MP header in the shape of a TIFF header follows
//!   them, and the index of the file's images after it, whose offsets count
//!   from the MP header's first byte (DC-007 5.2, p. 10). It is kept as a
//!   [`Block`] too: its base is the MP header, and its bounds run from
//!   there to the end of the segment. The [`mpf`] module reads the index.
//!
//! A payload too short to hold its identifier is not that segment. One that
//! holds the identifier and too little after it is: its block may be too
//! small to read, which the reader of the block finds out, as damage.

pub(crate) mod mpf;

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::Block;
use crate::error::Result;
use crate::io::{ByteOrder, Source, within};

/// The most steps a walk over a stream's segments takes before it gives up.
///
/// A step reads one marker, or skips one fill byte before a marker. Real
/// previews reach their frame header in a few dozen steps, and camera JPEG
/// files their scan in fewer; the cap keeps a stream made of nothing but
/// tiny segments or fill bytes from making a walk read its whole range,
/// which can be most of a large file, one marker at a time. Both [`probe`]
/// and [`segments`] walk under it.
///
/// The JPEG standard allows any number of fill bytes before a marker, so a
/// stream with more of them than the cap allows loses what lies after them.
const MAX_STEPS: u32 = 1024;

/// The marker that opens every JPEG stream, SOI, with the `FF` of the
/// marker that follows it: no stream starts any other way.
const START: [u8; 3] = [0xFF, 0xD8, 0xFF];

/// The shortest frame header that holds the image's size: the length
/// itself, the sample precision, the height and the width, 2 + 1 + 2 + 2
/// bytes.
const MIN_FRAME_HEADER_LEN: u16 = 7;

/// The marker of an APP1 segment, which holds Exif, XMP or other
/// application data.
const APP1: u8 = 0xE1;

/// The marker of an APP2 segment, which holds MPF or other application
/// data.
const APP2: u8 = 0xE2;

/// What the payload of an Exif segment starts with: `Exif`, then a zero
/// byte and a zero byte of padding (CIPA DC-008 4.7.2.3 and Figure 36,
/// p. 105).
const EXIF_ID: &[u8] = b"Exif\0\0";

/// What the payload of an MPF segment starts with: `MPF` and a zero byte
/// (CIPA DC-007 5.2.1 and Figure 4, pp. 10 to 11).
const MPF_ID: &[u8] = b"MPF\0";

/// The frame header of a JPEG stream: the size of the image and how it is
/// coded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Frame {
    /// The width of the image in pixels; never 0.
    pub width: u16,
    /// The height of the image in pixels; never 0.
    pub height: u16,
    /// The code of the frame header's marker, from `C0` to `CF`, which says
    /// how the image is coded: `C0` for a baseline JPEG, `C2` for a
    /// progressive one, `C3` for a lossless one, and so on; see
    /// [`is_lossless`](Frame::is_lossless).
    pub marker: u8,
}

impl Frame {
    /// Whether the image is coded losslessly, which is to say it is raw data
    /// and not a preview.
    ///
    /// A lossless JPEG keeps every sample exactly as it was, which is how a
    /// camera stores sensor data without loss, and such a stream can look much
    /// like a preview to the tags that point at it: Sony stores the raw tiles
    /// of its lossless ARW files as lossless JPEG streams. The preview finder
    /// drops these frames.
    ///
    /// The lossless frame markers are `C3`, the lossless process with Huffman
    /// coding, `C7`, its differential variant, and `CB` and `CF`, the same two
    /// with arithmetic coding.
    pub(crate) fn is_lossless(self) -> bool {
        matches!(self.marker, 0xC3 | 0xC7 | 0xCB | 0xCF)
    }
}

/// What the walk over a JPEG file's segments found; see the
/// [module documentation](self#a-jpeg-file).
///
/// Every field is `None` when the walk found no such segment before it
/// ended, and all three when the file does not start with SOI and a
/// marker.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fields are read by the JPEG format, which is not in the format table yet"
    )
)]
pub(crate) struct Segments {
    /// The first frame header, if it gives a size: `None` when there is no
    /// frame header before the walk ends, and when the first gives none,
    /// being too short to hold the size or giving a width or height of 0.
    /// A later frame header is not looked at.
    pub frame: Option<Frame>,
    /// The TIFF structure of the first Exif segment: its base is the TIFF
    /// header, 6 bytes into the segment's payload, and its bounds run from
    /// there to the end of the segment.
    pub exif: Option<Block>,
    /// The MPF index of the first MPF segment: its base is the MP header, 4
    /// bytes into the segment's payload, and its bounds run from there to
    /// the end of the segment.
    pub mpf: Option<Block>,
}

/// Whether `code` is the code of a frame header (SOF) marker.
///
/// They are `C0` to `CF` but for three that sit among them and are not:
/// `C4` defines Huffman tables, `C8` is reserved for JPEG extensions and
/// `CC` defines arithmetic-coding conditioning.
fn is_frame_marker(code: u8) -> bool {
    matches!(code, 0xC0..=0xCF) && !matches!(code, 0xC4 | 0xC8 | 0xCC)
}

/// Whether `code` is the code of a marker that stands alone, with no length
/// and no payload: TEM (`01`), the restart markers RST0 to RST7 (`D0` to
/// `D7`), and SOI (`D8`), which may repeat.
///
/// EOI (`D9`) stands alone too, but it ends the walk, and is not here.
fn is_standalone(code: u8) -> bool {
    matches!(code, 0x01 | 0xD0..=0xD8)
}

/// A segment of a JPEG stream that has a length: its marker's code and
/// where its payload lies.
///
/// The marker's `FF` is always 4 bytes before the payload: fill bytes
/// come before a marker, never between its `FF`, its code and its length.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Segment {
    /// The code of the segment's marker, such as `E1` for APP1.
    marker: u8,
    /// Where the payload lies in the file: from the byte after the length
    /// to the end of the segment. Empty for a segment of length 2.
    payload: Range<u64>,
}

/// The walk over the marker segments of a JPEG stream, which both
/// [`probe`] and [`segments`] use; see the
/// [module documentation](self#the-walk).
///
/// It yields each segment that has a length, in the order of the stream.
/// It skips fill bytes and stand-alone markers, which have nothing to
/// yield, and ends, yielding nothing more, at SOS, EOI, a byte that is not
/// a marker where a marker should be, a length below 2, a segment that
/// runs past the end of its range, or after [`MAX_STEPS`] steps, counted
/// over the whole walk. When reading the source fails it yields that error
/// and then ends too, since it cannot know where the next marker is.
///
/// It reads only inside its range and holds the source while it walks;
/// [`source`](Walk::source) lends it out between steps, to read a payload.
struct Walk<'a, R> {
    src: &'a mut Source<R>,
    /// Where the next marker, or a fill byte before it, should be.
    pos: u64,
    /// The end of the range walked.
    end: u64,
    /// The steps taken so far.
    steps: u32,
    /// Whether the walk has ended.
    ended: bool,
}

impl<'a, R: Read + Seek> Walk<'a, R> {
    /// The walk over the stream in the `len` bytes at `offset`, from the
    /// marker after its SOI.
    ///
    /// `None` when those bytes do not start with SOI and a marker
    /// (`FF D8 FF`), and when their end does not fit in a `u64`.
    ///
    /// # Errors
    ///
    /// Those of reading the first 3 bytes from `src`.
    fn from_soi(src: &'a mut Source<R>, offset: u64, len: u64) -> Result<Option<Self>> {
        let Some(end) = offset.checked_add(len) else {
            return Ok(None);
        };
        if len < 3 || src.bytes(offset, 3)? != START {
            return Ok(None);
        }
        // The first marker after SOI: its `FF` is the third byte, checked
        // above.
        let Some(pos) = within(offset, 2, end) else {
            return Ok(None);
        };
        Ok(Some(Self {
            src,
            pos,
            end,
            steps: 0,
            ended: false,
        }))
    }

    /// The source, to read a segment's payload between steps.
    fn source(&mut self) -> &mut Source<R> {
        self.src
    }

    /// Takes steps up to the next segment that has a length, and returns
    /// it; `None` when the walk ends before one.
    fn step(&mut self) -> Result<Option<Segment>> {
        while self.steps < MAX_STEPS {
            self.steps += 1;
            // A marker is `FF` and a code, both inside the range.
            let Some(code_at) = within(self.pos, 1, self.end) else {
                return Ok(None);
            };
            let Some(after) = within(code_at, 1, self.end) else {
                return Ok(None);
            };
            if self.src.u8_at(self.pos)? != 0xFF {
                return Ok(None);
            }
            let code = self.src.u8_at(code_at)?;
            if code == 0xFF {
                // `pos` was a fill byte; the marker starts at the next `FF`.
                self.pos = code_at;
                continue;
            }
            if code == 0x00 {
                // `FF 00` is a stuffed byte: the way entropy-coded data
                // writes a data byte of `FF`, so that it is not read as a
                // marker. It names no segment and appears only inside the
                // scan, so bytes that hold one where a marker should be are
                // not a sequence of segments, and reading a length after it
                // could jump the walk anywhere.
                return Ok(None);
            }
            if is_standalone(code) {
                self.pos = after;
                continue;
            }
            if matches!(code, 0xD9 | 0xDA) {
                // EOI ends the stream; SOS starts the entropy-coded data,
                // which is not made of segments.
                return Ok(None);
            }
            let Some(payload_at) = within(after, 2, self.end) else {
                return Ok(None);
            };
            let length = self.src.u16_at(after, ByteOrder::Big)?;
            if length < 2 {
                return Ok(None);
            }
            let Some(next) = within(after, u64::from(length), self.end) else {
                return Ok(None);
            };
            self.pos = next;
            return Ok(Some(Segment {
                marker: code,
                payload: payload_at..next,
            }));
        }
        Ok(None)
    }
}

impl<R: Read + Seek> Iterator for Walk<'_, R> {
    type Item = Result<Segment>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let step = self.step();
        if !matches!(step, Ok(Some(_))) {
            self.ended = true;
        }
        step.transpose()
    }
}

/// Reads the image's size from `segment`, a frame header: `None` when it
/// is too short to hold the size, or gives a width or height of 0.
///
/// # Errors
///
/// Those of reading `src`.
fn frame_size<R: Read + Seek>(src: &mut Source<R>, segment: &Segment) -> Result<Option<Frame>> {
    // The payload holds what the length counts, but for the length's own
    // 2 bytes.
    let size_len = u64::from(MIN_FRAME_HEADER_LEN - 2);
    let at = segment.payload.start;
    if within(at, size_len, segment.payload.end).is_none() {
        return Ok(None);
    }
    // The sample precision, 1 byte, which sizing does not need, then the
    // height and the width.
    let (Some(height_at), Some(width_at)) = (at.checked_add(1), at.checked_add(3)) else {
        return Ok(None);
    };
    let height = src.u16_at(height_at, ByteOrder::Big)?;
    let width = src.u16_at(width_at, ByteOrder::Big)?;
    // A height of 0 is legal JPEG, meaning that a DNL segment after the
    // first scan gives it, but it gives no size here; a width of 0 is never
    // legal.
    if width == 0 || height == 0 {
        return Ok(None);
    }
    Ok(Some(Frame {
        width,
        height,
        marker: segment.marker,
    }))
}

/// The block that `segment` holds after `id`, its identifier: its base is
/// the first byte after the identifier, and its bounds run from there to
/// the end of the segment. `None` when the payload does not start with
/// `id`, being too short to hold it included.
///
/// # Errors
///
/// Those of reading `src`.
fn block_after<R: Read + Seek>(
    src: &mut Source<R>,
    segment: &Segment,
    id: &[u8],
) -> Result<Option<Block>> {
    let Ok(id_len) = u64::try_from(id.len()) else {
        return Ok(None);
    };
    let at = segment.payload.start;
    let Some(base) = within(at, id_len, segment.payload.end) else {
        return Ok(None);
    };
    if src.bytes(at, id_len)? != id {
        return Ok(None);
    }
    Ok(Some(Block::new(base, base..segment.payload.end)))
}

/// Finds the frame header of the JPEG stream in the `len` bytes at
/// `offset` and returns the image's size from it.
///
/// Returns `Ok(None)` when those bytes hold no frame header that can be
/// found: they do not start with SOI and a marker (`FF D8 FF`), or the walk
/// over the marker segments meets SOS or EOI first, a byte that is not a
/// marker where a marker should be (a stuffed `FF 00` included), a segment
/// length shorter than its own 2 bytes, a segment that runs past
/// `offset + len`, more than [`MAX_STEPS`] steps, or a frame header too
/// short to hold the size or giving a width or height of 0. A range whose
/// end does not fit in a `u64` holds no stream either.
///
/// The walk reads only inside the range, never past `offset + len`, so a
/// caller can pass the range that a preview's tags declare and trust that
/// nothing after it is taken for part of the stream. Fill bytes, any run of
/// `FF` before a marker's code, are skipped, as the JPEG standard allows.
/// Every frame header is reported, lossless ones included: telling a
/// preview from raw data is the caller's decision, for which there is
/// [`is_lossless`](Frame::is_lossless).
///
/// # Errors
///
/// Only the errors of reading `src`: `Malformed` if the walk reads past the
/// end of the file, which can happen only when the range does not lie
/// inside it; `Io` if the reader fails.
pub(crate) fn probe<R: Read + Seek>(
    src: &mut Source<R>,
    offset: u64,
    len: u64,
) -> Result<Option<Frame>> {
    let Some(mut walk) = Walk::from_soi(src, offset, len)? else {
        return Ok(None);
    };
    while let Some(segment) = walk.next() {
        let segment = segment?;
        if is_frame_marker(segment.marker) {
            return frame_size(walk.source(), &segment);
        }
    }
    Ok(None)
}

/// Walks the segments of the JPEG file in `src`, from SOI, and keeps its
/// first frame header, its first Exif segment and its first MPF segment;
/// see the [module documentation](self#a-jpeg-file).
///
/// What the walk found before it ended is kept, whatever ended it: SOS,
/// EOI, a length below 2, a segment that runs past the end of the file, a
/// byte that is not a marker, or the cap of [`MAX_STEPS`] steps. A file
/// that does not start with SOI and a marker (`FF D8 FF`) gives
/// [`Segments`] with every field `None`.
///
/// # Errors
///
/// Only `Io`, if the reader fails: the walk reads only inside the file.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the JPEG format reads a file's segments, and it is not in the format table yet"
    )
)]
pub(crate) fn segments<R: Read + Seek>(src: &mut Source<R>) -> Result<Segments> {
    let mut found = Segments::default();
    let len = src.len();
    let Some(mut walk) = Walk::from_soi(src, 0, len)? else {
        return Ok(found);
    };
    let mut frame_seen = false;
    while let Some(segment) = walk.next() {
        let segment = segment?;
        if is_frame_marker(segment.marker) {
            if !frame_seen {
                frame_seen = true;
                found.frame = frame_size(walk.source(), &segment)?;
            }
        } else if segment.marker == APP1 && found.exif.is_none() {
            found.exif = block_after(walk.source(), &segment, EXIF_ID)?;
        } else if segment.marker == APP2 && found.mpf.is_none() {
            found.mpf = block_after(walk.source(), &segment, MPF_ID)?;
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::jpeg_builder::{Built, JpegBuilder};
    use crate::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};
    use crate::testing::{FAR, FailsFar};
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    /// Every frame header marker.
    const FRAME_MARKERS: [u8; 13] = [
        0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF,
    ];

    /// The frame header a stub of `width` by `height` with SOF0 gives.
    fn baseline(width: u16, height: u16) -> Option<Frame> {
        Some(Frame {
            width,
            height,
            marker: 0xC0,
        })
    }

    /// Probes the `len` bytes at `offset` of `bytes`.
    fn probe_in(bytes: &[u8], offset: u64, len: u64) -> Result<Option<Frame>> {
        probe(&mut Source::new(Cursor::new(bytes)).unwrap(), offset, len)
    }

    /// Probes all of `bytes`.
    fn probe_all(bytes: &[u8]) -> Result<Option<Frame>> {
        probe_in(bytes, 0, u64::try_from(bytes.len()).unwrap())
    }

    /// `bytes` with `extra` inserted at `at`.
    fn insert(bytes: &[u8], at: usize, extra: &[u8]) -> Vec<u8> {
        let mut out = bytes[..at].to_vec();
        out.extend_from_slice(extra);
        out.extend_from_slice(&bytes[at..]);
        out
    }

    /// SOI, then `segments`, then a baseline frame header for 640 by 480.
    fn soi_then(segments: &[Vec<u8>]) -> Vec<u8> {
        let stub = jpeg_stub(640, 480, 0xC0);
        let mut bytes = vec![0xFF, 0xD8];
        for s in segments {
            bytes.extend_from_slice(s);
        }
        bytes.extend_from_slice(&stub[FRAME_HEADER_AT..]);
        bytes
    }

    /// A stream as a camera writes a preview: SOI, a JFIF segment (APP0), an
    /// Exif segment (APP1) and a comment, then a baseline frame header for
    /// 640 by 480.
    fn with_app_segments() -> Vec<u8> {
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(&[b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0]);
        soi_then(&[
            segment(0xE0, b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0"),
            segment(0xE1, &exif),
            segment(0xFE, b"a comment"),
        ])
    }

    /// The streams the fuzz target `jpeg_probe` starts from: each file name
    /// with the bytes it holds.
    ///
    /// A fuzzer finds the `FF D8 FF` that opens a stream only by luck, and
    /// a frame header behind well-formed segments hardly ever, so it starts
    /// from these, committed as files in `fuzz/seeds/jpeg_probe/`: the stub
    /// with a baseline, a progressive and a lossless frame header, a stream
    /// with application segments and a comment before its frame header, and
    /// one whose frame header comes first, as in a Sony raw tile.
    ///
    /// The fuzz target reads the first two bytes of its input as a range
    /// inside it to probe as well, and every stream starts `FF D8`, which
    /// names a range no seed is long enough to hold. The last seed is not a
    /// stream but a fuzz input: the two bytes that choose the range, 2 and
    /// 0, then the baseline stub, so that the range they choose is exactly
    /// the stub and the range probe starts from a valid stream too.
    fn seeds() -> Vec<(&'static str, Vec<u8>)> {
        let mut in_a_range = vec![2, 0];
        in_a_range.extend(jpeg_stub(640, 480, 0xC0));
        vec![
            ("baseline.jpg", jpeg_stub(640, 480, 0xC0)),
            ("progressive.jpg", jpeg_stub(640, 480, 0xC2)),
            ("lossless.jpg", jpeg_stub(640, 480, 0xC3)),
            ("with-app-segments.jpg", with_app_segments()),
            ("sof-first.jpg", sof_first(512, 256, 0xC3)),
            ("in-a-range.bin", in_a_range),
        ]
    }

    /// Where the seed files live, under the crate's root.
    fn seeds_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/jpeg_probe")
    }

    /// A stream whose first marker after SOI is the frame header, with
    /// marker `marker`, for `width` by `height`: the layout of the lossless
    /// JPEG tiles of a Sony raw file, which start `FF D8 FF C3`. Then a scan
    /// of 4 bytes, and EOI.
    fn sof_first(width: u16, height: u16, marker: u8) -> Vec<u8> {
        let mut frame = vec![8];
        frame.extend_from_slice(&height.to_be_bytes());
        frame.extend_from_slice(&width.to_be_bytes());
        frame.extend_from_slice(&[1, 1, 0x11, 0]);
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend(segment(marker, &frame));
        bytes.extend(segment(0xDA, &[1, 1, 0x00, 0, 63, 0]));
        bytes.extend_from_slice(&[0x12, 0x34, 0x56, 0x78, 0xFF, 0xD9]);
        bytes
    }

    #[test]
    fn every_frame_marker_gives_the_size() {
        for marker in FRAME_MARKERS {
            let frame = probe_all(&jpeg_stub(6048, 4024, marker))
                .unwrap()
                .unwrap_or_else(|| panic!("no frame for marker {marker:02X}"));
            assert_eq!(frame.width, 6048, "marker {marker:02X}");
            assert_eq!(frame.height, 4024, "marker {marker:02X}");
            assert_eq!(frame.marker, marker);
        }
    }

    #[test]
    fn lossless_frames_are_told_apart() {
        const LOSSLESS: [u8; 4] = [0xC3, 0xC7, 0xCB, 0xCF];
        for marker in LOSSLESS {
            assert!(
                FRAME_MARKERS.contains(&marker),
                "marker {marker:02X} is not found as a frame"
            );
            let frame = probe_all(&jpeg_stub(640, 480, marker))
                .unwrap()
                .unwrap_or_else(|| panic!("no frame for marker {marker:02X}"));
            assert!(frame.is_lossless(), "marker {marker:02X}");
        }
        let others: Vec<u8> = FRAME_MARKERS
            .into_iter()
            .filter(|marker| !LOSSLESS.contains(marker))
            .collect();
        assert_eq!(others.len(), 9);
        for marker in others {
            let frame = probe_all(&jpeg_stub(640, 480, marker))
                .unwrap()
                .unwrap_or_else(|| panic!("no frame for marker {marker:02X}"));
            assert!(!frame.is_lossless(), "marker {marker:02X}");
        }
    }

    #[test]
    fn markers_among_the_frame_markers_that_are_not_frames() {
        for marker in [0xC4, 0xC8, 0xCC] {
            assert_eq!(
                probe_all(&jpeg_stub(640, 480, marker)).unwrap(),
                None,
                "marker {marker:02X}"
            );
        }
    }

    #[test]
    fn app_and_comment_segments_are_walked_over() {
        assert_eq!(probe_all(&with_app_segments()).unwrap(), baseline(640, 480));
    }

    #[test]
    fn fill_bytes_before_a_marker_are_skipped() {
        let stub = jpeg_stub(640, 480, 0xC0);
        // Before the first marker after SOI, and before the frame header.
        let bytes = insert(&stub, FRAME_HEADER_AT, &[0xFF, 0xFF, 0xFF]);
        let bytes = insert(&bytes, 2, &[0xFF, 0xFF, 0xFF]);
        assert_eq!(probe_all(&bytes).unwrap(), baseline(640, 480));
    }

    #[test]
    fn standalone_markers_are_skipped() {
        let mut standalone = vec![0xFF, 0x01];
        for code in 0xD0..=0xD8 {
            standalone.extend_from_slice(&[0xFF, code]);
        }
        let stub = jpeg_stub(640, 480, 0xC0);
        let bytes = insert(&stub, FRAME_HEADER_AT, &standalone);
        assert_eq!(probe_all(&bytes).unwrap(), baseline(640, 480));
    }

    #[test]
    fn sos_before_the_frame_header_means_no_frame() {
        let sos = segment(0xDA, &[1, 1, 0x00, 0, 63, 0]);
        assert_eq!(probe_all(&soi_then(&[sos])).unwrap(), None);
    }

    #[test]
    fn eoi_before_the_frame_header_means_no_frame() {
        // EOI followed by the frame header, and followed by two bytes that
        // would make it a 2-byte segment if it had a length: either way the
        // walk ends at EOI.
        let stub = jpeg_stub(640, 480, 0xC0);
        for eoi in [&[0xFF, 0xD9][..], &[0xFF, 0xD9, 0x00, 0x02]] {
            let bytes = insert(&stub, FRAME_HEADER_AT, eoi);
            assert_eq!(probe_all(&bytes).unwrap(), None, "{eoi:02X?}");
        }
    }

    #[test]
    fn a_byte_other_than_ff_where_a_marker_starts_means_no_frame() {
        let stub = jpeg_stub(640, 480, 0xC0);
        let bytes = insert(&stub, FRAME_HEADER_AT, &[0x00]);
        assert_eq!(probe_all(&bytes).unwrap(), None);
    }

    #[test]
    fn a_stuffed_byte_where_a_marker_starts_means_no_frame() {
        // `FF 00`, then 2 bytes that, read as a segment length, would land
        // the walk exactly on the frame header after them.
        let stub = jpeg_stub(640, 480, 0xC0);
        let bytes = insert(&stub, FRAME_HEADER_AT, &[0xFF, 0x00, 0x00, 0x02]);
        assert_eq!(probe_all(&bytes).unwrap(), None);
    }

    #[test]
    fn a_frame_header_right_after_soi_is_found() {
        for marker in [0xC3, 0xC0] {
            let frame = probe_all(&sof_first(512, 256, marker))
                .unwrap()
                .unwrap_or_else(|| panic!("no frame for marker {marker:02X}"));
            assert_eq!((frame.width, frame.height), (512, 256), "{marker:02X}");
            assert_eq!(frame.marker, marker);
        }
    }

    #[test]
    fn a_segment_length_below_2_means_no_frame() {
        for length in [0, 1] {
            let stub = jpeg_stub(640, 480, 0xC0);
            let bytes = insert(&stub, FRAME_HEADER_AT, &[0xFF, 0xFE, 0, length]);
            assert_eq!(probe_all(&bytes).unwrap(), None, "length {length}");
        }
    }

    #[test]
    fn a_segment_running_past_the_range_means_no_frame() {
        // A comment whose length, counted from its own 2 bytes, claims every
        // byte after it, and one more.
        let stub = jpeg_stub(640, 480, 0xC0);
        let rest = u16::try_from(stub.len() - FRAME_HEADER_AT).unwrap();
        let mut comment = vec![0xFF, 0xFE];
        comment.extend_from_slice(&(rest + 2 + 1).to_be_bytes());
        let bytes = insert(&stub, FRAME_HEADER_AT, &comment);
        assert_eq!(probe_all(&bytes).unwrap(), None);
        // Ending exactly at the range's end is not past it, but leaves no
        // room for a frame header.
        let mut comment = vec![0xFF, 0xFE];
        comment.extend_from_slice(&(rest + 2).to_be_bytes());
        let bytes = insert(&stub, FRAME_HEADER_AT, &comment);
        assert_eq!(probe_all(&bytes).unwrap(), None);
        // A frame header that claims more than the range holds, although
        // the size fits in it.
        let mut bytes = stub.clone();
        bytes[FRAME_HEADER_AT + 2..FRAME_HEADER_AT + 4].copy_from_slice(&rest.to_be_bytes());
        assert_eq!(probe_all(&bytes).unwrap(), None);
        bytes[FRAME_HEADER_AT + 2..FRAME_HEADER_AT + 4].copy_from_slice(&(rest - 2).to_be_bytes());
        assert_eq!(probe_all(&bytes).unwrap(), baseline(640, 480));
    }

    #[test]
    fn a_frame_header_cut_by_the_range_end_means_no_frame() {
        let stub = jpeg_stub(640, 480, 0xC0);
        let frame_end = u64::try_from(FRAME_HEADER_AT + 13).unwrap();
        for len in u64::try_from(FRAME_HEADER_AT).unwrap()..frame_end {
            assert_eq!(probe_in(&stub, 0, len).unwrap(), None, "len {len}");
        }
        assert_eq!(probe_in(&stub, 0, frame_end).unwrap(), baseline(640, 480));
    }

    #[test]
    fn a_frame_header_too_short_for_the_size_means_no_frame() {
        // Length 6: the precision, the height and one byte of the width.
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend(segment(0xC0, &[8, 0x01, 0xE0, 0x02]));
        bytes.extend(segment(0xDA, &[1, 1, 0x00, 0, 63, 0]));
        bytes.extend_from_slice(&[0x12, 0x34, 0xFF, 0xD9]);
        assert_eq!(probe_all(&bytes).unwrap(), None);
    }

    #[test]
    fn a_zero_width_or_height_means_no_frame() {
        for (width, height) in [(0, 480), (640, 0), (0, 0)] {
            assert_eq!(
                probe_all(&jpeg_stub(width, height, 0xC0)).unwrap(),
                None,
                "{width}x{height}"
            );
        }
    }

    #[test]
    fn bytes_not_starting_with_soi_and_a_marker_are_no_jpeg() {
        let stub = jpeg_stub(640, 480, 0xC0);
        for len in 0..3 {
            assert_eq!(probe_in(&stub, 0, len).unwrap(), None, "len {len}");
        }
        let tiff = [b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(probe_all(&tiff).unwrap(), None);
        for start in [[0xFF, 0xD8, 0x00], [0xFF, 0xD9, 0xFF], [0x00, 0xD8, 0xFF]] {
            let mut bytes = stub.clone();
            bytes[..3].copy_from_slice(&start);
            assert_eq!(probe_all(&bytes).unwrap(), None, "{start:02X?}");
        }
    }

    #[test]
    fn only_the_range_is_read() {
        let stub = jpeg_stub(640, 480, 0xC0);
        // Junk before the range, and after it a frame header that the range
        // does not include.
        let before = [
            0xFF, 0xD8, 0xFF, 0xC0, 0, 11, 8, 0, 9, 0, 9, 1, 1, 0x11, 0, 0,
        ];
        let after = jpeg_stub(1234, 567, 0xC0)[FRAME_HEADER_AT..].to_vec();
        let at = before.len();
        let offset = u64::try_from(at).unwrap();

        let mut file = before.to_vec();
        file.extend_from_slice(&stub);
        file.extend_from_slice(&after);
        let len = u64::try_from(stub.len()).unwrap();
        assert_eq!(probe_in(&file, offset, len).unwrap(), baseline(640, 480));

        // The stub without its frame header: the walk reaches the end of the
        // range, where the other frame header starts, and stops there.
        let mut file = before.to_vec();
        file.extend_from_slice(&stub[..FRAME_HEADER_AT]);
        file.extend_from_slice(&after);
        let len = u64::try_from(FRAME_HEADER_AT).unwrap();
        assert_eq!(probe_in(&file, offset, len).unwrap(), None);
        // With the range grown over it, it is found.
        let len = u64::try_from(file.len() - at).unwrap();
        assert_eq!(probe_in(&file, offset, len).unwrap(), baseline(1234, 567));
    }

    #[test]
    fn the_walk_gives_up_after_max_steps() {
        let comment = segment(0xFE, &[]);
        // Each comment and the frame header take one step each.
        let within_cap = vec![comment.clone(); 1023];
        assert_eq!(
            probe_all(&soi_then(&within_cap)).unwrap(),
            baseline(640, 480)
        );
        for count in [1024, 1025] {
            let past_cap = vec![comment.clone(); count];
            assert_eq!(probe_all(&soi_then(&past_cap)).unwrap(), None, "{count}");
        }
        // Fill bytes count as steps too.
        let stub = jpeg_stub(640, 480, 0xC0);
        let fill = vec![0xFF; 2000];
        assert_eq!(probe_all(&insert(&stub, 2, &fill)).unwrap(), None);
    }

    #[test]
    fn a_range_whose_end_overflows_holds_no_jpeg() {
        let stub = jpeg_stub(640, 480, 0xC0);
        assert_eq!(probe_in(&stub, 1, u64::MAX).unwrap(), None);
        assert_eq!(probe_in(&stub, u64::MAX, 3).unwrap(), None);
    }

    #[test]
    fn a_range_past_the_end_of_the_file_is_a_read_error() {
        let stub = jpeg_stub(640, 480, 0xC0);
        let len = u64::try_from(stub.len()).unwrap();
        // The range starts past the end of the file.
        let err = probe_in(&stub, len + 10, 10).expect_err("past the end");
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
        assert_eq!(err.offset(), Some(len + 10));
        // The range covers the whole stub, but the file ends before the
        // frame header.
        let err = probe_in(&stub[..FRAME_HEADER_AT], 0, len).expect_err("past the end");
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
        assert_eq!(err.offset(), Some(u64::try_from(FRAME_HEADER_AT).unwrap()));
    }

    #[test]
    fn damaged_stubs_never_panic() {
        for marker in [0xC0, 0xC3] {
            let stub = jpeg_stub(640, 480, marker);
            let len = u64::try_from(stub.len()).unwrap();
            for cut in 0..=stub.len() {
                let short = &stub[..cut];
                // Probed whole, and with the range of the stub before it was
                // cut, which runs past the end of the file.
                let _ = probe_all(short);
                let _ = probe_in(short, 0, len);
            }
            for at in 0..stub.len() {
                for value in [0x00, 0xFF, !stub[at]] {
                    let mut bytes = stub.clone();
                    bytes[at] = value;
                    let _ = probe_all(&bytes);
                }
            }
        }
    }

    /// Writes the seeds to `fuzz/seeds/jpeg_probe/`, where the fuzz target
    /// starts from them. Ignored, as it writes to the source tree; run it
    /// after changing a seed or the stub:
    /// `cargo test write_fuzz_seeds -- --ignored`, which also writes the
    /// TIFF reader's seeds and the `open` target's.
    #[test]
    #[ignore = "writes the seed files; run it to regenerate them"]
    fn write_fuzz_seeds() {
        let dir = seeds_dir();
        std::fs::create_dir_all(&dir).unwrap();
        for (name, bytes) in seeds() {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
    }

    /// The committed seed files hold the seeds' bytes, so that a changed
    /// stub cannot leave the fuzzer starting from a stale file.
    ///
    /// The skip is keyed on `fuzz/Cargo.toml`, which the repository has and
    /// the packaged crate leaves out: without it the test is skipped with a
    /// note. With it, every seed file must exist and match, so a renamed or
    /// deleted seeds directory fails instead of silently skipping.
    #[test]
    fn the_jpeg_fuzz_seeds_match_the_stubs() {
        let fuzz_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/Cargo.toml");
        if !fuzz_manifest.is_file() {
            eprintln!(
                "skipped: {} does not exist, as in the packaged crate",
                fuzz_manifest.display()
            );
            return;
        }
        let dir = seeds_dir();
        for (name, bytes) in seeds() {
            let path = dir.join(name);
            let committed = std::fs::read(&path).ok();
            assert!(
                committed.as_deref() == Some(&bytes[..]),
                "fuzz/seeds/jpeg_probe/{name} is missing or differs from its stub: \
                 regenerate it with `cargo test write_fuzz_seeds -- --ignored`"
            );
        }
    }

    /// The TIFF structure of the Exif segments the tests build: a
    /// little-endian header with IFD0 at 8, and an empty IFD0. The walk
    /// never reads it; it only has to be there.
    const TIFF: [u8; 14] = [b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /// What the MPF segments the tests build hold: the identifier, then an
    /// MP header, big-endian with the index at 8, as a TIFF header is laid
    /// out, and an empty IFD. The walk never reads past the identifier.
    const MPF: [u8; 18] = [
        b'M', b'P', b'F', 0, b'M', b'M', 0, 42, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0,
    ];

    /// The SOS segment of the stub, which starts the scan.
    const SOS: [u8; 6] = [1, 1, 0x00, 0, 63, 0];

    /// The segments the walk finds in `bytes`, walked from SOI over all of
    /// them; `None` when they do not start with SOI and a marker.
    fn walked(bytes: &[u8]) -> Option<Vec<Segment>> {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let len = src.len();
        let walk = Walk::from_soi(&mut src, 0, len).unwrap()?;
        Some(walk.map(Result::unwrap).collect())
    }

    /// The markers of `segments`, in order.
    fn markers(segments: &[Segment]) -> Vec<u8> {
        segments.iter().map(|segment| segment.marker).collect()
    }

    /// The segments of the file `bytes`.
    fn segments_of(bytes: &[u8]) -> Segments {
        segments(&mut Source::new(Cursor::new(bytes)).unwrap()).unwrap()
    }

    /// The block of the `n`th Exif segment of `built`, from where the
    /// builder says its TIFF structure lies.
    fn exif_block(built: &Built, n: usize) -> Block {
        let tiff = built.exif_tiff(n);
        Block::new(tiff.start, tiff)
    }

    /// The block of the MPF index in the `n`th APP2 segment of `built`:
    /// from the MP header, 4 bytes into the payload where the builder says
    /// it starts, to the end of the segment.
    fn mpf_block(built: &Built, n: usize) -> Block {
        let payload = built.app2_payload(n);
        let base = payload.start + 4;
        Block::new(base, base..payload.end)
    }

    /// A camera's JPEG: SOI, JFIF, Exif, an XMP segment, MPF, a comment, the
    /// main image of 640 by 480, a smaller image after it and zeros.
    fn camera_jpeg() -> Built {
        let mut builder = JpegBuilder::new();
        builder
            .jfif()
            .exif(&TIFF)
            .xmp("<x:xmpmeta/>")
            .app2(&MPF)
            .segment(0xFE, b"a comment")
            .after_eoi(&jpeg_stub(160, 120, 0xC0))
            .pad(64);
        builder.build()
    }

    /// The segments found in a camera's JPEG: its frame header, its Exif
    /// segment and its MPF segment.
    fn camera_jpeg_segments(built: &Built) -> Segments {
        Segments {
            frame: baseline(640, 480),
            exif: Some(exif_block(built, 0)),
            mpf: Some(mpf_block(built, 0)),
        }
    }

    /// A frame header's payload for `width` by `height`, with one
    /// component.
    fn frame_payload(width: u16, height: u16) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&height.to_be_bytes());
        payload.extend_from_slice(&width.to_be_bytes());
        payload.extend_from_slice(&[1, 1, 0x11, 0]);
        payload
    }

    /// A file whose walk reads past [`FAR`] before its Exif segment and
    /// frame header: two comments of 60,000 bytes after SOI.
    fn far_jpeg() -> Vec<u8> {
        let big = vec![0x20; 60_000];
        let mut builder = JpegBuilder::new();
        builder.segment(0xFE, &big).segment(0xFE, &big).exif(&TIFF);
        let bytes = builder.build().bytes;
        assert!(u64::try_from(bytes.len()).unwrap() > FAR);
        bytes
    }

    #[test]
    fn the_walk_yields_each_segment_with_its_payload() {
        let built = camera_jpeg();
        let found = walked(&built.bytes).unwrap();
        // JFIF, Exif, XMP, MPF, the comment, then the stub's DQT, DHT and
        // frame header; the walk ends at SOS.
        assert_eq!(
            markers(&found),
            [0xE0, 0xE1, 0xE1, 0xE2, 0xFE, 0xDB, 0xC4, 0xC0]
        );
        // Each payload follows its marker and its length, and each segment
        // starts where the one before it ends.
        let mut marker_at = 2;
        for segment in &found {
            assert_eq!(segment.payload.start, marker_at + 4, "{segment:?}");
            let at = usize::try_from(marker_at).unwrap();
            assert_eq!(built.bytes[at..at + 2], [0xFF, segment.marker]);
            let length = u16::from_be_bytes([built.bytes[at + 2], built.bytes[at + 3]]);
            assert_eq!(segment.payload.end, marker_at + 2 + u64::from(length));
            marker_at = segment.payload.end;
        }
        assert_eq!(found[1].payload.start + 6, built.exif_tiff(0).start);
        assert_eq!(found[1].payload.end, built.exif_tiff(0).end);
        assert_eq!(found[3].payload, built.app2_payload(0));
        assert_eq!(found[7].payload.start - 4, built.frame_header());
    }

    #[test]
    fn a_camera_jpeg_gives_its_frame_exif_and_mpf() {
        let built = camera_jpeg();
        let found = segments_of(&built.bytes);
        assert_eq!(found, camera_jpeg_segments(&built));
        // Exactly where the builder says, which the blocks above are built
        // from: the TIFF header after the Exif segment's identifier, at 2
        // bytes of SOI, 18 of JFIF and 10 of the Exif segment's marker,
        // length and identifier, and the MP header after `MPF` and a zero.
        let exif = found.exif.unwrap();
        assert_eq!(exif.base(), 2 + 18 + 10);
        assert_eq!(exif.bounds(), &(30..30 + 14));
        let at = usize::try_from(exif.base()).unwrap();
        assert_eq!(built.bytes[at..at + 4], [b'I', b'I', 42, 0]);
        let mpf = found.mpf.unwrap();
        let at = usize::try_from(mpf.base()).unwrap();
        assert_eq!(built.bytes[at - 4..at + 4], MPF[..8]);
        assert_eq!(mpf.bounds().end, built.app2_payload(0).end);
    }

    #[test]
    fn fill_bytes_before_markers_are_skipped() {
        let mut builder = JpegBuilder::new();
        builder
            .fill(1)
            .exif(&TIFF)
            .fill(3)
            .app2(&MPF)
            .fill(7)
            .segment(0xFE, b"a comment")
            .fill(2);
        let built = builder.build();
        assert_eq!(
            segments_of(&built.bytes),
            Segments {
                frame: baseline(640, 480),
                exif: Some(exif_block(&built, 0)),
                mpf: Some(mpf_block(&built, 0)),
            }
        );
        // The fill bytes come before the markers, so each payload is still
        // the 4 bytes after its marker's `FF`.
        let walked = walked(&built.bytes).unwrap();
        for segment in &walked {
            let at = usize::try_from(segment.payload.start - 4).unwrap();
            assert_eq!(built.bytes[at..at + 2], [0xFF, segment.marker]);
            assert_ne!(built.bytes[at + 2], 0xFF, "{segment:?}");
        }
    }

    #[test]
    fn standalone_markers_are_taken_as_they_are() {
        let mut builder = JpegBuilder::new();
        builder.marker(0x01).exif(&TIFF);
        for code in 0xD0..=0xD7 {
            builder.marker(code);
        }
        // SOI again, which may repeat, then MPF.
        builder.marker(0xD8).app2(&MPF).marker(0xD3);
        let built = builder.build();
        assert_eq!(
            segments_of(&built.bytes),
            Segments {
                frame: baseline(640, 480),
                exif: Some(exif_block(&built, 0)),
                mpf: Some(mpf_block(&built, 0)),
            }
        );
        // They have no length and nothing to yield.
        assert_eq!(
            markers(&walked(&built.bytes).unwrap()),
            [0xE1, 0xE2, 0xDB, 0xC4, 0xC0]
        );
    }

    #[test]
    fn a_length_below_2_ends_the_walk_keeping_what_came_before() {
        for length in [0, 1] {
            let mut builder = JpegBuilder::new();
            builder.exif(&TIFF).raw(&[0xFF, 0xFE, 0, length]).app2(&MPF);
            let built = builder.build();
            assert_eq!(
                segments_of(&built.bytes),
                Segments {
                    frame: None,
                    exif: Some(exif_block(&built, 0)),
                    mpf: None,
                },
                "length {length}"
            );
            // The walk yields nothing from the bad segment on.
            assert_eq!(
                markers(&walked(&built.bytes).unwrap()),
                [0xE1],
                "length {length}"
            );
        }
        // A length of 2, an empty comment, is a segment.
        let mut builder = JpegBuilder::new();
        builder.exif(&TIFF).raw(&[0xFF, 0xFE, 0, 2]).app2(&MPF);
        let built = builder.build();
        assert_eq!(
            segments_of(&built.bytes),
            Segments {
                frame: baseline(640, 480),
                exif: Some(exif_block(&built, 0)),
                mpf: Some(mpf_block(&built, 0)),
            }
        );
    }

    #[test]
    fn a_segment_running_past_the_end_of_the_file_ends_the_walk() {
        // MPF, then an Exif segment cut by the end of the file: the segment
        // is not taken, and what came before it is kept.
        let mut builder = JpegBuilder::new();
        builder.app2(&MPF).exif(&TIFF);
        let built = builder.build();
        let tiff = built.exif_tiff(0);
        let mpf = Some(mpf_block(&built, 0));
        for cut in tiff.start..tiff.end {
            let bytes = built.truncated(usize::try_from(cut).unwrap());
            assert_eq!(
                segments_of(&bytes),
                Segments {
                    frame: None,
                    exif: None,
                    mpf: mpf.clone(),
                },
                "cut at {cut}"
            );
        }
        // Ending exactly at the end of the file is not past it.
        let bytes = built.truncated(usize::try_from(tiff.end).unwrap());
        assert_eq!(
            segments_of(&bytes),
            Segments {
                frame: None,
                exif: Some(exif_block(&built, 0)),
                mpf,
            }
        );
    }

    #[test]
    fn a_second_exif_segment_is_skipped() {
        let mut other = TIFF;
        other[..2].copy_from_slice(b"MM");
        let mut builder = JpegBuilder::new();
        builder.exif(&TIFF).exif(&other);
        let built = builder.build();
        let found = segments_of(&built.bytes);
        assert_eq!(found.exif, Some(exif_block(&built, 0)));
        assert_ne!(exif_block(&built, 0), exif_block(&built, 1));
        assert_eq!(found.frame, baseline(640, 480));
    }

    #[test]
    fn an_exif_segment_behind_jfif_is_found() {
        let mut builder = JpegBuilder::new();
        builder.jfif().exif(&TIFF);
        let built = builder.build();
        let found = segments_of(&built.bytes);
        // After SOI's 2 bytes and the 18 of the JFIF segment.
        assert_eq!(built.exif_tiff(0).start, 2 + 18 + 10);
        assert_eq!(found.exif, Some(exif_block(&built, 0)));
        assert_eq!(found.frame, baseline(640, 480));
    }

    #[test]
    fn an_xmp_segment_is_not_exif() {
        let mut builder = JpegBuilder::new();
        builder.xmp("<x:xmpmeta/>");
        assert_eq!(
            segments_of(&builder.build().bytes),
            Segments {
                frame: baseline(640, 480),
                exif: None,
                mpf: None,
            }
        );
        // XMP first, then Exif: the Exif segment is the one taken.
        builder.exif(&TIFF);
        let built = builder.build();
        assert_eq!(segments_of(&built.bytes).exif, Some(exif_block(&built, 0)));
    }

    #[test]
    fn an_app1_segment_without_the_whole_exif_identifier_is_not_exif() {
        // Too short to hold the identifier, then with another byte where
        // its second zero should be; each followed by an Exif segment, which
        // is taken.
        let mut not_exif = b"Exif\0\x01".to_vec();
        not_exif.extend_from_slice(&TIFF);
        for payload in [&b"Exif\0"[..], b"Exif", b"", &not_exif[..]] {
            let mut builder = JpegBuilder::new();
            builder.segment(0xE1, payload).exif(&TIFF);
            let built = builder.build();
            assert_eq!(
                segments_of(&built.bytes).exif,
                Some(exif_block(&built, 0)),
                "{payload:02X?}"
            );
            // Alone, it gives no Exif segment.
            let mut builder = JpegBuilder::new();
            builder.segment(0xE1, payload);
            assert_eq!(
                segments_of(&builder.build().bytes).exif,
                None,
                "{payload:02X?}"
            );
        }
    }

    #[test]
    fn an_exif_segment_with_nothing_after_its_identifier_is_taken() {
        // Its block is empty: the reader of the block finds the TIFF header
        // missing.
        let mut builder = JpegBuilder::new();
        builder.exif(&[]);
        let built = builder.build();
        let tiff = built.exif_tiff(0);
        assert_eq!(tiff.start, tiff.end);
        assert_eq!(
            segments_of(&built.bytes).exif,
            Some(Block::new(tiff.start, tiff))
        );
    }

    #[test]
    fn the_first_mpf_segment_is_found_among_app2_segments() {
        // A colour profile first, an APP2 segment of another kind, and a
        // second MPF segment after the first.
        let mut builder = JpegBuilder::new();
        builder.app2(b"ICC_PROFILE\0\x01\x01").app2(&MPF).app2(&MPF);
        let built = builder.build();
        let found = segments_of(&built.bytes);
        let mpf = found.mpf.unwrap();
        assert_eq!(mpf, mpf_block(&built, 1));
        assert_eq!(mpf.base(), built.app2_payload(1).start + 4);
        assert_eq!(mpf.bounds(), &(mpf.base()..mpf.base() + 14));
        let at = usize::try_from(mpf.base()).unwrap();
        assert_eq!(built.bytes[at..at + 4], [b'M', b'M', 0, 42]);
        assert_eq!(found.exif, None);
    }

    #[test]
    fn an_app2_segment_without_the_whole_mpf_identifier_is_not_mpf() {
        for payload in [&b"MPF"[..], b"MPF\x01", b""] {
            let mut builder = JpegBuilder::new();
            builder.app2(payload).app2(&MPF);
            let built = builder.build();
            assert_eq!(
                segments_of(&built.bytes).mpf,
                Some(mpf_block(&built, 1)),
                "{payload:02X?}"
            );
        }
        // The identifier alone is MPF, with an empty block.
        let mut builder = JpegBuilder::new();
        builder.app2(b"MPF\0");
        let built = builder.build();
        let end = built.app2_payload(0).end;
        assert_eq!(
            segments_of(&built.bytes).mpf,
            Some(Block::new(end, end..end))
        );
    }

    #[test]
    fn no_frame_header_gives_no_frame() {
        // The stub's frame header given each marker among the frame markers
        // that is no frame marker: the walk goes on to SOS.
        for marker in [0xC4, 0xC8, 0xCC] {
            let mut builder = JpegBuilder::new();
            builder.exif(&TIFF).frame(640, 480, marker);
            let built = builder.build();
            assert_eq!(
                segments_of(&built.bytes),
                Segments {
                    frame: None,
                    exif: Some(exif_block(&built, 0)),
                    mpf: None,
                },
                "marker {marker:02X}"
            );
        }
    }

    #[test]
    fn sos_first_gives_nothing() {
        let mut builder = JpegBuilder::new();
        builder.segment(0xDA, &SOS).exif(&TIFF).app2(&MPF);
        assert_eq!(segments_of(&builder.build().bytes), Segments::default());
    }

    #[test]
    fn eoi_before_the_frame_header_keeps_what_came_before() {
        // EOI, then MPF, and EOI followed by two bytes that would make it a
        // 2-byte segment if it had a length: either way the walk ends at
        // EOI.
        for eoi in [&[0xFF, 0xD9][..], &[0xFF, 0xD9, 0x00, 0x02]] {
            let mut builder = JpegBuilder::new();
            builder.exif(&TIFF).raw(eoi).app2(&MPF);
            let built = builder.build();
            assert_eq!(
                segments_of(&built.bytes),
                Segments {
                    frame: None,
                    exif: Some(exif_block(&built, 0)),
                    mpf: None,
                },
                "{eoi:02X?}"
            );
        }
    }

    #[test]
    fn a_lossless_frame_is_kept_as_found() {
        let mut builder = JpegBuilder::new();
        builder.frame(512, 256, 0xC3);
        let frame = segments_of(&builder.build().bytes).frame.unwrap();
        assert_eq!(
            frame,
            Frame {
                width: 512,
                height: 256,
                marker: 0xC3
            }
        );
        assert!(frame.is_lossless());
    }

    #[test]
    fn only_the_first_frame_header_is_looked_at() {
        // A frame header of 100 by 50 before the main image's: it is the
        // one kept.
        let mut builder = JpegBuilder::new();
        builder.segment(0xC2, &frame_payload(100, 50));
        assert_eq!(
            segments_of(&builder.build().bytes).frame,
            Some(Frame {
                width: 100,
                height: 50,
                marker: 0xC2
            })
        );
        // One with a zero side, or too short to hold the size, gives no
        // frame, and the main image's, which has a size, is not taken.
        let short = &frame_payload(100, 50)[..4];
        for payload in [frame_payload(0, 50), frame_payload(100, 0), short.to_vec()] {
            let mut builder = JpegBuilder::new();
            builder.exif(&TIFF).segment(0xC0, &payload);
            let built = builder.build();
            assert_eq!(
                segments_of(&built.bytes),
                Segments {
                    frame: None,
                    exif: Some(exif_block(&built, 0)),
                    mpf: None,
                },
                "{payload:02X?}"
            );
        }
        // The main image's own with a zero side.
        for (width, height) in [(0, 480), (640, 0)] {
            let mut builder = JpegBuilder::new();
            builder.frame(width, height, 0xC0);
            assert_eq!(segments_of(&builder.build().bytes).frame, None);
        }
    }

    #[test]
    fn the_cap_counts_every_fill_byte_and_marker() {
        // After SOI: `fill` fill bytes, one step each, then the stub's DQT,
        // DHT and frame header, one step each, so the frame header is the
        // step `fill + 3`.
        let at_cap = usize::try_from(MAX_STEPS).unwrap() - 3;
        for (fill, found) in [(at_cap, true), (at_cap + 1, false)] {
            let mut builder = JpegBuilder::new();
            builder.fill(fill);
            let built = builder.build();
            let frame = found.then(|| baseline(640, 480)).flatten();
            assert_eq!(segments_of(&built.bytes).frame, frame, "fill {fill}");
            assert_eq!(probe_all(&built.bytes).unwrap(), frame, "fill {fill}");
        }
        // Segments count across what the walk yields: an Exif segment and
        // empty comments, a step each, then the stub's three.
        let comments = usize::try_from(MAX_STEPS).unwrap() - 4;
        for (count, found) in [(comments, true), (comments + 1, false)] {
            let mut builder = JpegBuilder::new();
            builder.exif(&TIFF);
            for _ in 0..count {
                builder.segment(0xFE, &[]);
            }
            let built = builder.build();
            let segments = segments_of(&built.bytes);
            assert_eq!(segments.exif, Some(exif_block(&built, 0)), "{count}");
            let frame = found.then(|| baseline(640, 480)).flatten();
            assert_eq!(segments.frame, frame, "{count} comments");
            assert_eq!(probe_all(&built.bytes).unwrap(), frame, "{count}");
        }
    }

    #[test]
    fn a_file_not_starting_with_soi_and_a_marker_gives_nothing() {
        let bytes = camera_jpeg().bytes;
        for len in 0..3 {
            assert_eq!(segments_of(&bytes[..len]), Segments::default(), "{len}");
        }
        let tiff = [b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(segments_of(&tiff), Segments::default());
        for start in [[0xFF, 0xD8, 0x00], [0xFF, 0xD9, 0xFF], [0x00, 0xD8, 0xFF]] {
            let mut bytes = bytes.clone();
            bytes[..3].copy_from_slice(&start);
            assert_eq!(segments_of(&bytes), Segments::default(), "{start:02X?}");
        }
    }

    #[test]
    fn a_failing_reader_is_an_error() {
        let bytes = far_jpeg();
        let mut src = Source::new(FailsFar(Cursor::new(bytes.clone()))).unwrap();
        let err = segments(&mut src).expect_err("the reader fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
        let mut src = Source::new(FailsFar(Cursor::new(bytes.clone()))).unwrap();
        let len = u64::try_from(bytes.len()).unwrap();
        let err = probe(&mut src, 0, len).expect_err("the reader fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
        // Read without the failure, the same bytes give the Exif segment.
        assert!(segments_of(&bytes).exif.is_some());
    }

    #[test]
    fn the_walk_ends_after_an_error() {
        let bytes = far_jpeg();
        let mut src = Source::new(FailsFar(Cursor::new(bytes.clone()))).unwrap();
        let len = u64::try_from(bytes.len()).unwrap();
        let mut walk = Walk::from_soi(&mut src, 0, len).unwrap().unwrap();
        // The two comments, read from the first block.
        for _ in 0..2 {
            assert_eq!(walk.next().unwrap().unwrap().marker, 0xFE);
        }
        let err = walk.next().unwrap().expect_err("the reader fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
        assert!(walk.next().is_none());
        assert!(walk.next().is_none());
    }

    #[test]
    fn every_prefix_of_a_file_gives_what_lies_wholly_inside_it() {
        let mut builder = JpegBuilder::new();
        builder
            .jfif()
            .fill(2)
            .exif(&TIFF)
            .marker(0xD0)
            .xmp("<x:xmpmeta/>")
            .app2(&MPF)
            .segment(0xFE, b"a comment")
            .after_eoi(&jpeg_stub(160, 120, 0xC0))
            .pad(16);
        let built = builder.build();
        let whole = segments_of(&built.bytes);
        assert_eq!(whole.frame, baseline(640, 480));
        for cut in 0..=built.bytes.len() {
            let found = segments_of(&built.truncated(cut));
            let cut = u64::try_from(cut).unwrap();
            // What lies inside the prefix is found, as in the whole file.
            let exif = built.exif_tiff(0);
            let expected = Segments {
                frame: (cut >= built.frame_header() + 13)
                    .then_some(whole.frame)
                    .flatten(),
                exif: (cut >= exif.end).then(|| whole.exif.clone()).flatten(),
                mpf: (cut >= built.app2_payload(0).end)
                    .then(|| whole.mpf.clone())
                    .flatten(),
            };
            assert_eq!(found, expected, "cut at {cut}");
        }
    }

    #[test]
    fn damaged_files_never_panic() {
        let built = camera_jpeg();
        // The image after the main image's EOI is not walked, so the bytes
        // up to it are the ones to damage.
        let end = usize::try_from(built.after_eoi(0)).unwrap();
        for at in 0..end {
            for value in [0x00, 0xFF, !built.bytes[at]] {
                let mut bytes = built.bytes.clone();
                bytes[at] = value;
                let mut src = Source::new(Cursor::new(&bytes[..])).unwrap();
                segments(&mut src).expect("only a reader's failure is an error");
                let _ = probe_all(&bytes);
            }
        }
    }
}
