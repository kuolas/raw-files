// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG container, as far as sizing an embedded preview needs it.
//!
//! RAW files carry their previews as JPEG streams, and a file often holds
//! several, from a thumbnail to a full-size image. The tags that describe a
//! preview are often missing, zero or wrong about its size, so the preview
//! finder asks each candidate itself: [`probe`] finds the frame header of a
//! JPEG stream and reads the image's width and height from it. Nothing here
//! decodes an image.
//!
//! A JPEG stream is a sequence of marker segments. Each starts with a
//! marker, the byte `FF` followed by a code that says what the segment is;
//! most markers are followed by a 2-byte big-endian length, which counts
//! itself and the payload after it. A stream opens with SOI (`FF D8`), then
//! holds tables, application data such as Exif, and comments, then the
//! frame header (SOF), which gives the image's size, then the scan: an SOS
//! segment followed by the entropy-coded image data.
//!
//! The walk stops at SOS. What follows it is the compressed image, not a
//! sequence of segments, and an `FF` byte inside it can look like a marker
//! without being one; a stream whose frame header has not come by then has
//! none that can be found by walking.
//!
//! The frame header also says how the image is coded. A lossless frame
//! (`C3`, `C7`, `CB`, `CF`) is never a preview: Sony stores the raw sensor
//! data of its lossless ARW files as tiles that are lossless JPEG streams,
//! each starting `FF D8 FF C3`, and a strip that points at the first tile
//! looks to the tags much like a preview. The frame says whether it is
//! lossless, so that the finder can drop it.

use std::io::{Read, Seek};

use crate::error::Result;
use crate::io::{ByteOrder, Source};

/// The most steps the walk takes before it gives up on a stream.
///
/// A step reads one marker, or skips one fill byte before a marker. Real
/// previews reach their frame header in a few dozen steps; the cap keeps a
/// stream made of nothing but tiny segments or fill bytes from making the
/// probe read its whole range, which can be most of a large file, one
/// marker at a time.
const MAX_STEPS: u32 = 1024;

/// The marker that opens every JPEG stream, SOI, with the `FF` of the
/// marker that follows it: no stream starts any other way.
const START: [u8; 3] = [0xFF, 0xD8, 0xFF];

/// The shortest frame header that holds the image's size: the length
/// itself, the sample precision, the height and the width, 2 + 1 + 2 + 2
/// bytes.
const MIN_FRAME_HEADER_LEN: u16 = 7;

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
    /// of its lossless ARW files as lossless JPEG streams. The preview finder,
    /// once written, will drop these frames.
    ///
    /// The lossless frame markers are `C3`, the lossless process with Huffman
    /// coding, `C7`, its differential variant, and `CB` and `CF`, the same two
    /// with arithmetic coding.
    pub(crate) fn is_lossless(self) -> bool {
        matches!(self.marker, 0xC3 | 0xC7 | 0xCB | 0xCF)
    }
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
fn is_standalone(code: u8) -> bool {
    matches!(code, 0x01 | 0xD0..=0xD8)
}

/// `pos + n`, if it is at most `end`.
fn within(pos: u64, n: u64, end: u64) -> Option<u64> {
    pos.checked_add(n).filter(|&next| next <= end)
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
    let Some(end) = offset.checked_add(len) else {
        return Ok(None);
    };
    if len < 3 || src.bytes(offset, 3)? != START {
        return Ok(None);
    }
    // The first marker after SOI: its `FF` is the third byte, checked above.
    let Some(mut pos) = within(offset, 2, end) else {
        return Ok(None);
    };
    for _ in 0..MAX_STEPS {
        // A marker is `FF` and a code, both inside the range.
        let Some(code_at) = within(pos, 1, end) else {
            return Ok(None);
        };
        let Some(after) = within(code_at, 1, end) else {
            return Ok(None);
        };
        if src.u8_at(pos)? != 0xFF {
            return Ok(None);
        }
        let code = src.u8_at(code_at)?;
        if code == 0xFF {
            // `pos` was a fill byte; the marker starts at the next `FF`.
            pos = code_at;
            continue;
        }
        if code == 0x00 {
            // `FF 00` is a stuffed byte: the way entropy-coded data writes a
            // data byte of `FF`, so that it is not read as a marker. It
            // names no segment and appears only inside the scan, so bytes
            // that hold one where a marker should be are not a sequence of
            // segments, and reading a length after it could jump the walk
            // anywhere.
            return Ok(None);
        }
        if is_standalone(code) {
            pos = after;
            continue;
        }
        if matches!(code, 0xD9 | 0xDA) {
            // EOI ends the stream; SOS starts the entropy-coded data, which
            // is not made of segments.
            return Ok(None);
        }
        let Some(payload_at) = within(after, 2, end) else {
            return Ok(None);
        };
        let length = src.u16_at(after, ByteOrder::Big)?;
        if length < 2 {
            return Ok(None);
        }
        let Some(next) = within(after, u64::from(length), end) else {
            return Ok(None);
        };
        if is_frame_marker(code) {
            if length < MIN_FRAME_HEADER_LEN {
                return Ok(None);
            }
            // After the length: the sample precision, 1 byte, which sizing
            // does not need, then the height and the width.
            let (Some(height_at), Some(width_at)) =
                (payload_at.checked_add(1), payload_at.checked_add(3))
            else {
                return Ok(None);
            };
            let height = src.u16_at(height_at, ByteOrder::Big)?;
            let width = src.u16_at(width_at, ByteOrder::Big)?;
            // A height of 0 is legal JPEG, meaning that a DNL segment after
            // the first scan gives it, but it gives no size here; a width of
            // 0 is never legal.
            if width == 0 || height == 0 {
                return Ok(None);
            }
            return Ok(Some(Frame {
                width,
                height,
                marker: code,
            }));
        }
        pos = next;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};
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
    /// `cargo test --lib write_fuzz_seeds -- --ignored`, which also writes
    /// the TIFF reader's.
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
                 regenerate it with `cargo test --lib write_fuzz_seeds -- --ignored`"
            );
        }
    }
}
