// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests of the whole open path on damaged RAW and JPEG files, through the
//! public API; this file holds no code of the reader itself.
//!
//! A RAW library often meets truncated and corrupted files, and must
//! answer each with a result, never a panic (ADR008). The unit tests of
//! the TIFF reader hold the walk to that; the tests here hold everything
//! that opening a file runs after it to the same rule: the detection of
//! the format, the preview finder, and the metadata reader, whose values
//! come from tags all over the file. They damage the fixtures that carry
//! every metadata tag the readers read: a Nikon Z 9's NEF (with its
//! MakerNote, which records the focus point, in the file's byte order and
//! in the other), a Nikon Z6III's NEF on a 1:1 crop, whose MakerNote's
//! CropHiSpeed and CropArea place the focus point on the cropped picture, a
//! Nikon D300's NEF, whose MakerNote records the serial number, the lens's
//! range and the clock's time zone that its standard tags lack, a Nikon
//! D1H's NEF, whose MakerNote, of the headerless layout, its offsets
//! counted from the start of the file, records the ISO and the lens's
//! range and holds the file's only JPEG, and an ILCE-7M4's ARW 4, with its
//! MakerNote, which records the focus point, each in both byte orders.
//! They damage five real files too, committed with their pixels and
//! personal fields removed or zeroed: two NEF shots of the maintainer's
//! own Z 9 in DX and 1:1 crop modes, and three JPEG shots of the same
//! camera in FX and in DX and 16:9 crop modes, the last in portrait
//! orientation, whose Exif segment is the camera's own; they hold the
//! camera's own layout of tags, in little-endian byte order, where the
//! fixtures only imitate it. And they damage the JPEG files the JPEG
//! reader's tests open, whose segments the walk reads before the Exif
//! segment's TIFF structure and the MPF index: one with no Exif segment,
//! one with zeros after its end, one whose Exif segment holds IFD0 and
//! IFD1's thumbnail, a camera's, whose Exif segment holds every value the
//! metadata reads and whose MPF index lists two images after its main
//! one, a camera's with a JFIF segment first, whose Exif segment holds
//! only IFD0's orientation and IFD1's thumbnail, and five Nikon cameras'
//! JPEGs, whose Exif segment holds a Nikon MakerNote: a Z 9's, whose
//! MakerNote records its focus point, one of a Z 9's on a 1:1 crop, whose
//! point is placed on the JPEG's pixels, a D5300's, whose MakerNote
//! records the serial number, the lens's range and the clock's time zone
//! that its Exif segment lacks, a D70's, whose MakerNote, in the other
//! byte order from its Exif segment's, records the ISO and holds a
//! preview, and one laid out as a D1H's would be, whose MakerNote, of the
//! headerless layout, its offsets counted from the segment's TIFF header,
//! records the ISO and the lens's range and holds a preview;
//! and three Sony cameras' JPEGs, whose Exif segment holds a Sony
//! MakerNote after its `SONY DSC` header, its offsets counted from the
//! segment's TIFF header: an ILCE-7M3's, whose MakerNote records its focus
//! point, an ILCE-7M4's, whose MakerNote records the focus area's size too,
//! and a DSLR-A100's, with no MPF index, whose MakerNote holds tags of an
//! older layout and no focus point; those with a TIFF structure or an MPF
//! index in both byte orders. They
//! cut each file at every length, and overwrite each byte in turn with
//! 0x00, 0xFF and its complement.
//!
//! Each damaged file is opened as a caller would open it, with
//! [`RawFile::from_bytes`], which reads through the same source and block
//! cache as [`RawFile::from_reader`]. Opening may fail; when it does not,
//! every getter of the metadata is read, every fraction is shown, reduced
//! and turned into a decimal, the capture time is shown and taken apart,
//! the focus point is shown and scaled onto a small frame and onto its
//! own, and every preview is read, so that a panic anywhere on the way is
//! caught.
//! Tests run with debug assertions, so an arithmetic overflow panics too.
//!
//! The ARW 4 file is 65,780 bytes, most of them the zeros before its two
//! lossless raw tiles, which sit at a fixed offset, and the tiles
//! themselves. Nothing reads those bytes when the file is opened: the tiles
//! are raw data, which the preview finder passes over by its tags. So the
//! ARW 4 file is cut at every length and overwritten byte by byte only up
//! to the end of what the builder lays out before them (the header, the
//! IFDs, their values and the JPEG stubs), and cut at a few lengths past
//! that: in the zeros, at each tile's start and around it, and just short
//! of the end. That keeps the whole file to a few seconds in a debug build.
//!
//! The three stripped JPEG files are 69,998 bytes each, most of them the
//! camera's Exif segment of 65,536 bytes, kept whole so that no offset in
//! it moves. Three spans of it are zeros, which nothing parses when the
//! file is opened: the value of the MakerNote's ShotInfo entry, 35,785
//! bytes, an enciphered block that no reader reads, which the stripping
//! zeroed; the zeros after IFD1's stub thumbnail, up to the length IFD1
//! declares for it, 10,559 to 12,664 bytes, which the stripping wrote, and
//! which the preview finder never reaches, as it stops at the stub's frame
//! header, and reading the preview copies without looking at them; and
//! the zeros the camera wrote after that length, up to the end of the
//! segment, 12,876 to 14,981 bytes, at which no offset points and which
//! the walk over the segments steps over by the segment's length. So each
//! of the three is cut at every length, and overwritten byte by byte
//! everywhere but in those three spans, which the test finds from the
//! file's own structure ([`stripped_jpeg_unread`]) and checks are zeros:
//! every segment header, every IFD, every value a reader reads, the MPF
//! segment and every stub are still overwritten. That keeps the three
//! files to about 17 seconds in a debug build, most of the run;
//! overwriting every byte of them took more than two minutes.
//!
//! Every other file is swept whole: the two stripped NEF files are 4,270
//! bytes each, their raw data replaced by 64 bytes, and the largest JPEG
//! file built in code, the one with zeros after its end, is 4,218 bytes.
//!
//! A last test damages one value only, the capture time, in each of the
//! RAW files, and checks that the file gives every other value exactly as the
//! undamaged file does: damage to one value costs only that value
//! (ADR011).

mod support;

use std::fmt::Write as _;
use std::hint::black_box;
use std::ops::Range;
use std::panic;

use raw_files::{Focus, Metadata, Rational, RawFile, SignedRational, Size};
use support::arw::{ARW4_FULL_SIZE_JPEG, arw4_file, stub};
use support::fixtures::{Z9_1X1, Z9_16X9_JPEG, Z9_DX, Z9_DX_JPEG, Z9_FX_JPEG, read_fixture};
use support::jpeg_files::{
    CAMERA_MAKE, ILCE7M3_MAIN, MAIN, Z9Mode, a100_jpeg, camera_jpeg, camera_jpeg_with_metadata,
    d1h_jpeg, d70_jpeg, d5300_jpeg, exif_jpeg, ilce7m3_jpeg, ilce7m4_jpeg, padded_jpeg, plain_jpeg,
    z9_jpeg,
};
use support::nef::{d1h_file, d300_file, z6iii_file, z9_file, z9_file_with};
use support::tiff_builder::Order;

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

/// A fixture to damage: its name for failure messages, its bytes, and how
/// far into it the byte-by-byte damage goes.
struct Fixture {
    name: String,
    bytes: Vec<u8>,
    /// The bytes before this offset are cut at every length and
    /// overwritten one by one, but those of [`Fixture::unread`]; past it,
    /// only [`Fixture::more_lengths`] are tried.
    sweep_end: usize,
    /// Lengths past `sweep_end` to cut the file at.
    more_lengths: Vec<usize>,
    /// Spans before `sweep_end` that are cut at every length but not
    /// overwritten: zeros in a camera's stripped JPEG, which no reader
    /// parses ([`stripped_jpeg_unread`]).
    unread: Vec<Range<usize>>,
}

/// The Z 9 fixture, with IFD0's Orientation 8, in both byte orders, and
/// with its MakerNote in the other byte order from the file's; the Z6III
/// and the D300 fixtures, with Orientation 8, in both byte orders; the D1H
/// fixture, which records no orientation, in both byte orders; the
/// ARW 4 fixture, with Orientation 8, in both byte orders; the five files
/// stripped from the maintainer's own Z 9 shots, as they are committed: two
/// NEF files in DX and 1:1 crop modes, and three JPEG files in FX and in DX
/// and 16:9 crop modes; and the JPEG files: without Exif, with
/// zeros after its end, and, in both byte orders, with an Exif segment, a
/// camera's with every value the metadata reads, a camera's with a JFIF
/// segment first, whose Exif segment holds only the orientation and the
/// thumbnail, the Nikon JPEGs of a Z 9 in FX and on a 1:1 crop, of a
/// D5300, of a D70 and laid out as a D1H's would be, and the Sony JPEGs of
/// an ILCE-7M3, an ILCE-7M4 and a DSLR-A100.
fn fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();
    for order in ORDERS {
        let bytes = z9_file(order, 8).bytes;
        fixtures.push(Fixture {
            name: format!("Z 9, {order:?}"),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    for (order, makernote_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
        let bytes = z9_file_with(order, 8, makernote_order).0.bytes;
        fixtures.push(Fixture {
            name: format!("Z 9, {order:?}, MakerNote {makernote_order:?}"),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    for order in ORDERS {
        let bytes = z6iii_file(order, 8).bytes;
        fixtures.push(Fixture {
            name: format!("Z6III, {order:?}"),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    for order in ORDERS {
        let bytes = d300_file(order, 8).bytes;
        fixtures.push(Fixture {
            name: format!("D300, {order:?}"),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    for order in ORDERS {
        let bytes = d1h_file(order).bytes;
        fixtures.push(Fixture {
            name: format!("D1H, {order:?}"),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    for order in ORDERS {
        let (built, blobs) = arw4_file(order, 8);
        let at = |offset: u64| usize::try_from(offset).unwrap();
        // IFD2's JPEG is the last blob the builder lays out before the
        // tiles, which it places at their own offset.
        let ifd2_jpeg = blobs.ifd2_jpeg.unwrap();
        let laid_out = at(built.blob_offset(ifd2_jpeg)) + stub(ARW4_FULL_SIZE_JPEG).len();
        let tiles: Vec<usize> = blobs
            .raw
            .iter()
            .map(|&tile| at(built.blob_offset(tile)))
            .collect();
        assert!(laid_out < tiles[0], "ARW 4, {order:?}: the tiles come last");
        let len = built.bytes.len();
        let mut more_lengths = vec![(laid_out + tiles[0]) / 2, len - 1, len];
        for &tile in &tiles {
            more_lengths.extend([tile - 1, tile, tile + 1]);
        }
        fixtures.push(Fixture {
            name: format!("ARW 4, {order:?}"),
            bytes: built.bytes,
            sweep_end: laid_out,
            more_lengths,
            unread: Vec::new(),
        });
    }
    for (name, file) in [("Z 9 DX, stripped", Z9_DX), ("Z 9 1:1, stripped", Z9_1X1)] {
        let bytes = read_fixture(file);
        fixtures.push(Fixture {
            name: name.to_owned(),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    for (name, file) in [
        ("Z 9 FX JPEG, stripped", Z9_FX_JPEG),
        ("Z 9 DX JPEG, stripped", Z9_DX_JPEG),
        ("Z 9 16:9 JPEG, stripped", Z9_16X9_JPEG),
    ] {
        let bytes = read_fixture(file);
        fixtures.push(Fixture {
            name: name.to_owned(),
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: stripped_jpeg_unread(&bytes),
            bytes,
        });
    }
    let mut jpegs = vec![
        ("JPEG without Exif".to_owned(), plain_jpeg().bytes),
        ("JPEG with padding".to_owned(), padded_jpeg().bytes),
    ];
    for order in ORDERS {
        jpegs.extend([
            (
                format!("JPEG with Exif, {order:?}"),
                exif_jpeg(order).0.bytes,
            ),
            (
                format!("camera JPEG, {order:?}"),
                camera_jpeg_with_metadata(order).0.bytes,
            ),
            (
                format!("camera JPEG, JFIF first, {order:?}"),
                camera_jpeg(order, true).0.bytes,
            ),
            (
                format!("Z 9 JPEG, {order:?}"),
                z9_jpeg(order, order, Z9Mode::Fx, MAIN, false).built.bytes,
            ),
            (
                format!("Z 9 JPEG, 1:1, {order:?}"),
                z9_jpeg(order, order, Z9Mode::Square, (2752, 2752), false)
                    .built
                    .bytes,
            ),
            (
                format!("D5300 JPEG, {order:?}"),
                d5300_jpeg(order).built.bytes,
            ),
            (
                format!("D70 JPEG, {order:?}"),
                d70_jpeg(order, false).0.built.bytes,
            ),
            (
                format!("D1H JPEG, {order:?}"),
                d1h_jpeg(order, CAMERA_MAKE, |_, _| {}).0.built.bytes,
            ),
            (
                format!("ILCE-7M3 JPEG, {order:?}"),
                ilce7m3_jpeg(order, ILCE7M3_MAIN).built.bytes,
            ),
            (
                format!("ILCE-7M4 JPEG, {order:?}"),
                ilce7m4_jpeg(order).built.bytes,
            ),
            (
                format!("A100 JPEG, {order:?}"),
                a100_jpeg(order).built.bytes,
            ),
        ]);
    }
    for (name, bytes) in jpegs {
        fixtures.push(Fixture {
            name,
            sweep_end: bytes.len(),
            more_lengths: Vec::new(),
            unread: Vec::new(),
            bytes,
        });
    }
    fixtures
}

/// The spans of a JPEG stripped from a Z 9 shot that hold only zeros and
/// that no reader parses, found from the file's own structure: the value
/// of the MakerNote's ShotInfo entry (tag 0x0091), an enciphered block
/// that no reader reads, zeroed in place by the stripping; the zeros after
/// IFD1's stub thumbnail, up to the length IFD1 declares for it, which the
/// stripping kept so that no offset moves; and the zeros the camera wrote
/// after that length, up to the end of the Exif segment, which no offset
/// in the segment points at. The entries that point at the first two
/// spans, and the stub itself, stay outside them.
///
/// Panics when the file is not laid out as a stripped Z 9 JPEG is, or a
/// span holds a byte that is not zero: a mistake in the fixture.
fn stripped_jpeg_unread(bytes: &[u8]) -> Vec<Range<usize>> {
    // The Exif segment follows SOI, and its TIFF structure follows the
    // segment's marker, its length and `Exif` with two zero bytes. The
    // length, big-endian, counts itself and what follows it, so the
    // segment ends that many bytes after the marker.
    assert_eq!(bytes[..4], [0xFF, 0xD8, 0xFF, 0xE1]);
    assert_eq!(bytes[6..12], *b"Exif\0\0");
    let segment_end = 4 + usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let exif = Tiff::at(bytes, 12);
    let ifd0 = exif.u32(4);
    let ifd1 = exif.u32(ifd0 + 2 + 12 * u32::from(exif.u16(ifd0)));
    let thumbnail = exif.at_offset(exif.field(ifd1, 0x0201));
    let declared = thumbnail + usize::try_from(exif.field(ifd1, 0x0202)).unwrap();
    assert!(declared <= segment_end, "{declared} past {segment_end}");
    // The stub ends at its first end-of-image marker: no other byte pair of
    // a stub is `FF D9`.
    assert_eq!(bytes[thumbnail..thumbnail + 3], [0xFF, 0xD8, 0xFF]);
    let stub_end = thumbnail
        + bytes[thumbnail..declared]
            .windows(2)
            .position(|pair| pair == [0xFF, 0xD9])
            .unwrap()
        + 2;
    // The MakerNote: `Nikon`, a zero byte, its version and two zero bytes,
    // then a TIFF header of its own, from which its offsets count.
    let makernote = exif.field(exif.field(ifd0, 0x8769), 0x927C);
    let at = exif.at_offset(makernote);
    assert_eq!(bytes[at..at + 6], *b"Nikon\0");
    let inner = Tiff::at(bytes, at + 10);
    let shot_info = inner.at_offset(inner.field(inner.u32(4), 0x0091));
    let shot_info_len = usize::try_from(inner.count(inner.u32(4), 0x0091)).unwrap();
    let spans = vec![
        shot_info..shot_info + shot_info_len,
        stub_end..declared,
        declared..segment_end,
    ];
    for span in &spans {
        assert!(
            bytes[span.clone()].iter().all(|&byte| byte == 0),
            "{span:?}"
        );
    }
    spans
}

/// A TIFF structure inside a file: where its header lies, and its byte
/// order, read from that header. Its reads panic past the end of the file,
/// which only [`stripped_jpeg_unread`] makes, on committed fixtures.
struct Tiff<'a> {
    bytes: &'a [u8],
    base: usize,
    little: bool,
}

impl<'a> Tiff<'a> {
    /// The TIFF structure whose header lies at `base` in `bytes`.
    fn at(bytes: &'a [u8], base: usize) -> Self {
        let little = match &bytes[base..base + 2] {
            b"II" => true,
            b"MM" => false,
            order => panic!("no TIFF byte order at {base}: {order:?}"),
        };
        Self {
            bytes,
            base,
            little,
        }
    }

    /// Where `offset`, counted from the header, lies in the file.
    fn at_offset(&self, offset: u32) -> usize {
        self.base + usize::try_from(offset).unwrap()
    }

    /// The SHORT at `offset` from the header.
    fn u16(&self, offset: u32) -> u16 {
        let at = self.at_offset(offset);
        let pair = [self.bytes[at], self.bytes[at + 1]];
        if self.little {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    }

    /// The LONG at `offset` from the header.
    fn u32(&self, offset: u32) -> u32 {
        let at = self.at_offset(offset);
        let four = self.bytes[at..at + 4].try_into().unwrap();
        if self.little {
            u32::from_le_bytes(four)
        } else {
            u32::from_be_bytes(four)
        }
    }

    /// Where the entry of `tag` lies in the IFD at `ifd`, from the header.
    fn entry(&self, ifd: u32, tag: u16) -> u32 {
        (0..u32::from(self.u16(ifd)))
            .map(|index| ifd + 2 + 12 * index)
            .find(|&entry| self.u16(entry) == tag)
            .unwrap_or_else(|| panic!("no tag {tag:#06X} in the IFD at {ifd}"))
    }

    /// The count of the entry of `tag` in the IFD at `ifd`.
    fn count(&self, ifd: u32, tag: u16) -> u32 {
        self.u32(self.entry(ifd, tag) + 4)
    }

    /// The four-byte field of the entry of `tag` in the IFD at `ifd`, read
    /// as a LONG: the value of a LONG, or the offset of a value stored
    /// outside the entry.
    fn field(&self, ifd: u32, tag: u16) -> u32 {
        self.u32(self.entry(ifd, tag) + 8)
    }
}

/// Opens `bytes` and, when that succeeds, reads everything a caller can
/// read from the file: every value of its metadata, in every form the
/// library gives it, and every preview. Returns whether the file opened.
///
/// Reading a preview may fail; it must not panic.
fn open_and_read_everything(bytes: &[u8]) -> bool {
    let Ok(mut file) = RawFile::from_bytes(bytes) else {
        return false;
    };
    let mut seen = String::new();
    show_metadata(file.metadata(), &mut seen);
    black_box(seen);
    for preview in file.previews().to_vec() {
        let _ = black_box(file.read_preview(&preview));
    }
    true
}

/// Writes every value of `metadata` to `out`, through every getter and in
/// every form the library offers: each fraction shown, reduced and as a
/// decimal; the capture time shown and taken apart; the focus point shown,
/// and scaled onto a small frame and onto its own.
fn show_metadata(metadata: &Metadata, out: &mut String) {
    let _ = write!(
        out,
        "{:?} {:?} {:?} {:?}",
        metadata.make(),
        metadata.model(),
        metadata.serial_number(),
        metadata.orientation()
    );
    if let Some(time) = metadata.capture_time() {
        let _ = write!(
            out,
            " {time} {}-{}-{} {}:{}:{} {} {:?}",
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second(),
            time.nanosecond(),
            time.offset_minutes()
        );
    }
    let exposure = metadata.exposure();
    let _ = write!(out, " {:?}", exposure.iso());
    for fraction in [
        exposure.exposure_time(),
        exposure.f_number(),
        exposure.focal_length(),
    ] {
        show_fraction(fraction, out);
    }
    show_signed_fraction(exposure.exposure_bias(), out);
    if let Some(lens) = metadata.lens() {
        let _ = write!(
            out,
            " {:?} {:?} {:?}",
            lens.make(),
            lens.model(),
            lens.serial_number()
        );
        for fraction in [
            lens.min_focal_length(),
            lens.max_focal_length(),
            lens.f_number_at_min_focal_length(),
            lens.f_number_at_max_focal_length(),
        ] {
            show_fraction(fraction, out);
        }
    }
    for size in [metadata.raw_size(), metadata.image_size()]
        .into_iter()
        .flatten()
    {
        let _ = write!(out, " {}x{}", size.width, size.height);
    }
    if let Some(focus) = metadata.focus() {
        show_focus(focus, out);
        // Placed on a small preview's frame, and on its own, which scales
        // by one.
        for frame in [SMALL_FRAME, focus.frame()] {
            if let Some(scaled) = focus.scaled_to(frame) {
                show_focus(scaled, out);
            }
        }
    }
}

/// A small preview's frame, to place a focus point on: IFD0's JPEG of the
/// Z 9 fixture.
const SMALL_FRAME: Size = Size {
    width: 160,
    height: 120,
};

/// Writes `focus` to `out`: its frame, its centre, its size and whether
/// focus was confirmed.
fn show_focus(focus: Focus, out: &mut String) {
    let (frame, center) = (focus.frame(), focus.center());
    let _ = write!(
        out,
        " {}x{} ({}, {}) {:?} {:?}",
        frame.width,
        frame.height,
        center.x,
        center.y,
        focus.size(),
        focus.in_focus()
    );
}

/// Writes `fraction`, when there is one, to `out`: as stored, reduced, and
/// as a decimal.
fn show_fraction(fraction: Option<Rational>, out: &mut String) {
    if let Some(fraction) = fraction {
        let _ = write!(
            out,
            " {fraction} {} {}",
            fraction.reduced(),
            fraction.to_f64()
        );
    }
}

/// Writes `fraction`, when there is one, to `out`: as stored, reduced, and
/// as a decimal.
fn show_signed_fraction(fraction: Option<SignedRational>, out: &mut String) {
    if let Some(fraction) = fraction {
        let _ = write!(
            out,
            " {fraction} {} {}",
            fraction.reduced(),
            fraction.to_f64()
        );
    }
}

/// Opens `bytes` and reads everything from it, failing the test, with the
/// damage `what` names, if anything panics. Returns whether the file
/// opened.
fn check(bytes: &[u8], what: impl Fn() -> String) -> bool {
    match panic::catch_unwind(|| open_and_read_everything(bytes)) {
        Ok(opened) => opened,
        Err(_) => panic!("opening or reading {} panicked", what()),
    }
}

/// A file cut short anywhere, in its header, a JPEG segment, an IFD, a
/// value, a MakerNote or a JPEG stub, opens or fails, and never panics,
/// nor does reading what it gives.
#[test]
fn a_file_cut_short_opens_or_fails_without_a_panic() {
    for fixture in fixtures() {
        let (mut opened, mut failed) = (0, 0);
        let lengths = (0..=fixture.sweep_end).chain(fixture.more_lengths.iter().copied());
        for len in lengths {
            if check(&fixture.bytes[..len], || {
                format!("{} cut to {len} bytes", fixture.name)
            }) {
                opened += 1;
            } else {
                failed += 1;
            }
        }
        // Both outcomes occur: the sweep reaches past the header and the
        // walk, into what the readers read.
        assert!(opened > 0 && failed > 0, "{}", fixture.name);
        // Uncut, the file opens, by the same check as its cuts.
        assert!(
            check(&fixture.bytes, || fixture.name.clone()),
            "{}",
            fixture.name
        );
    }
}

/// A file with any one byte set to 0x00, to 0xFF or to its complement opens
/// or fails, and never panics, nor does reading what it gives.
#[test]
fn a_file_with_a_byte_overwritten_opens_or_fails_without_a_panic() {
    for fixture in fixtures() {
        let (mut opened, mut failed) = (0, 0);
        let mut damaged = fixture.bytes.clone();
        let swept = (0..fixture.sweep_end)
            .filter(|at| !fixture.unread.iter().any(|span| span.contains(at)));
        for at in swept {
            let original = fixture.bytes[at];
            for byte in [0x00, 0xFF, !original] {
                if byte == original {
                    continue;
                }
                damaged[at] = byte;
                let what = || format!("{} with byte {at} set to {byte:#04X}", fixture.name);
                if check(&damaged, what) {
                    opened += 1;
                } else {
                    failed += 1;
                }
            }
            damaged[at] = original;
        }
        assert!(opened > 0 && failed > 0, "{}", fixture.name);
    }
}

/// Asserts that `damaged` gives every value of `undamaged` but the capture
/// time, which it does not give; `what` names the file in a failure.
fn assert_all_but_the_capture_time(damaged: &Metadata, undamaged: &Metadata, what: &str) {
    assert_eq!(damaged.capture_time(), None, "{what}");
    assert!(undamaged.capture_time().is_some(), "{what}");
    assert_eq!(damaged.make(), undamaged.make(), "{what}");
    assert_eq!(damaged.model(), undamaged.model(), "{what}");
    assert_eq!(damaged.serial_number(), undamaged.serial_number(), "{what}");
    assert_eq!(damaged.orientation(), undamaged.orientation(), "{what}");
    assert_eq!(damaged.exposure(), undamaged.exposure(), "{what}");
    assert_eq!(damaged.lens(), undamaged.lens(), "{what}");
    assert_eq!(damaged.raw_size(), undamaged.raw_size(), "{what}");
    assert_eq!(damaged.image_size(), undamaged.image_size(), "{what}");
    assert_eq!(damaged.focus(), undamaged.focus(), "{what}");
}

/// A capture time that names no real date, in an otherwise undamaged file,
/// costs the capture time and nothing else: every other value is the one
/// the undamaged file gives, and so are the previews.
///
/// The damage goes into each copy of the time's text in turn. A file can
/// hold the same text in other tags: the two stripped Z 9 files hold it in
/// IFD0's DateTime and DateTimeOriginal and in the Exif IFD's
/// DateTimeOriginal and DateTimeDigitized. The capture time is read from
/// the Exif IFD's DateTimeOriginal alone, so damage to exactly one copy
/// costs it, and damage to any other copy changes nothing.
#[test]
fn an_impossible_capture_time_costs_only_the_capture_time() {
    let mut fixtures = Vec::new();
    for order in ORDERS {
        fixtures.extend([
            (
                format!("Z 9, {order:?}"),
                z9_file(order, 8).bytes,
                "2022:02:05 13:46:41",
            ),
            (
                format!("Z6III, {order:?}"),
                z6iii_file(order, 8).bytes,
                "2024:06:29 15:38:53",
            ),
            (
                format!("D300, {order:?}"),
                d300_file(order, 8).bytes,
                "2008:05:17 10:22:31",
            ),
            (
                format!("D1H, {order:?}"),
                d1h_file(order).bytes,
                "2022:03:24 10:09:27",
            ),
            (
                format!("ARW 4, {order:?}"),
                arw4_file(order, 8).0.bytes,
                "2023:10:19 16:13:48",
            ),
        ]);
    }
    fixtures.extend([
        (
            "Z 9 DX, stripped".to_owned(),
            read_fixture(Z9_DX),
            "2026:10:19 19:26:32",
        ),
        (
            "Z 9 1:1, stripped".to_owned(),
            read_fixture(Z9_1X1),
            "2026:10:19 19:26:52",
        ),
    ]);
    for (name, undamaged_bytes, time) in fixtures {
        let undamaged = RawFile::from_bytes(&undamaged_bytes).unwrap();
        let copies: Vec<usize> = undamaged_bytes
            .windows(time.len())
            .enumerate()
            .filter(|(_, window)| *window == time.as_bytes())
            .map(|(at, _)| at)
            .collect();
        assert!(!copies.is_empty(), "{name}: no DateTimeOriginal");
        // Where the damage goes in `YYYY:MM:DD HH:MM:SS`, and what it
        // writes there.
        for (damage, offset, replacement) in
            [("month 13", 5, b"13"), ("a letter in the day", 8, b"0x")]
        {
            let mut costly = 0;
            for &at in &copies {
                let what = format!("{name}, {damage} in the copy at byte {at}");
                let mut bytes = undamaged_bytes.clone();
                let start = at + offset;
                bytes[start..start + 2].copy_from_slice(replacement);
                let damaged = RawFile::from_bytes(&bytes).unwrap();
                if damaged.metadata().capture_time().is_none() {
                    costly += 1;
                    assert_all_but_the_capture_time(
                        damaged.metadata(),
                        undamaged.metadata(),
                        &what,
                    );
                } else {
                    // A copy the reader does not read.
                    assert_eq!(damaged.metadata(), undamaged.metadata(), "{what}");
                }
                assert_eq!(damaged.previews(), undamaged.previews(), "{what}");
            }
            assert_eq!(
                costly, 1,
                "{name}, {damage}: the copies at {copies:?} hold one capture time"
            );
        }
    }
}
