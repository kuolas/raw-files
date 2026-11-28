// SPDX-License-Identifier: MIT OR Apache-2.0

//! The XMP packet a camera file embeds, through the public API: where each
//! format holds it, what gives none, and how many reads it costs.

mod support;

use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

use raw_files::{CameraFile, Directory, Format, Limits, Options, Xmp, namespaces};
use support::jpeg_builder::JpegBuilder;
use support::tiff_builder::{Order, RawValue, TiffBuilder};
use support::xmp_packets;

/// The Make tag: the camera's manufacturer, as text.
const MAKE: u16 = 0x010F;

/// The Orientation tag, which every test file below records.
const ORIENTATION: u16 = 0x0112;

/// IFD0's tag for the XMP packet (XMP Part 3, Table 13).
const XMP: u16 = 0x02BC;

/// A tag no reader reads, which holds the packet's bytes in place of
/// [`XMP`] so that a file can be read with the same layout and no packet.
const UNREAD: u16 = 0x02BD;

/// The field types the tests write.
const BYTE: u16 = 1;
const UNDEFINED: u16 = 7;
const LONG: u16 = 4;

/// The two raw formats, each with the Make that its reader claims.
const RAW_MAKES: [(Format, &str); 2] = [(Format::Nef, "NIKON CORPORATION"), (Format::Arw, "SONY")];

/// A packet of one property, a rating of 3, in the wrapper cameras write.
fn packet() -> Vec<u8> {
    b"<?xpacket begin=\"\xEF\xBB\xBF\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
      <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
      <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
      <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" \
      xmp:Rating=\"3\"/></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>"
        .to_vec()
}

/// What [`packet`] gives: its one property. An empty `rdf:about` gives
/// no subject.
fn expected() -> Xmp {
    Xmp::default().with_property(namespaces::XMP, "Rating", "3")
}

/// A reader over bytes in memory that counts its `read` calls into a
/// counter the caller keeps.
struct Counting {
    inner: Cursor<Vec<u8>>,
    reads: Rc<Cell<usize>>,
}

impl Read for Counting {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        self.inner.read(buf)
    }
}

impl Seek for Counting {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// Opens `bytes` through a reader that counts its reads, and returns the
/// file and how many reads opening it took.
fn open_counting(bytes: Vec<u8>) -> (CameraFile<Counting>, usize) {
    let reads = Rc::new(Cell::new(0));
    let reader = Counting {
        inner: Cursor::new(bytes),
        reads: Rc::clone(&reads),
    };
    let file = CameraFile::from_reader(reader).expect("the file opens");
    (file, reads.get())
}

/// A little-endian TIFF file whose IFD0 holds the Make `make`, an
/// orientation, and `bytes` under `tag` with field type `typ` and count
/// `count`, placed after the IFD.
fn with_value(make: &str, tag: u16, typ: u16, count: u32, bytes: &[u8]) -> Vec<u8> {
    // Built twice: once to learn where the bytes lie, then with the entry
    // pointing there, which takes the same room as the stand-in.
    let build = |offset: u32| {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        builder
            .ascii(ifd0, MAKE, make)
            .short(ifd0, ORIENTATION, &[1]);
        builder.raw(ifd0, tag, typ, count, RawValue::Offset(offset));
        let blob = builder.blob(bytes);
        let built = builder.build();
        (u32::try_from(built.blob_offset(blob)).unwrap(), built.bytes)
    };
    let (offset, _) = build(0);
    let (again, bytes) = build(offset);
    assert_eq!(again, offset);
    bytes
}

/// [`with_value`] with `bytes` under `tag` as `typ`, one value per byte.
fn with_packet(make: &str, tag: u16, typ: u16, bytes: &[u8]) -> Vec<u8> {
    with_value(make, tag, typ, u32::try_from(bytes.len()).unwrap(), bytes)
}

/// A little-endian TIFF file whose IFD0 holds the Make `make` and the
/// packet under [`XMP`] as BYTEs, both at `at`, where an editor that
/// rewrote IFD0 leaves them: the header, zeros up to `at`, then IFD0 and
/// its values, the packet last.
fn ifd0_at(make: &str, tag: u16, at: u32) -> Vec<u8> {
    let packet = packet();
    let mut make_bytes = make.as_bytes().to_vec();
    make_bytes.push(0);
    // IFD0: three entries, then the next-IFD pointer, then the Make's
    // value and the packet.
    let entries = 3;
    let make_at = at + 2 + 12 * entries + 4;
    let packet_at = make_at + u32::try_from(make_bytes.len()).unwrap();
    let mut bytes = vec![b'I', b'I', 42, 0];
    bytes.extend_from_slice(&at.to_le_bytes());
    bytes.resize(usize::try_from(at).unwrap(), 0);
    bytes.extend_from_slice(&u16::try_from(entries).unwrap().to_le_bytes());
    let mut entry = |tag: u16, typ: u16, count: u32, field: [u8; 4]| {
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&typ.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        bytes.extend_from_slice(&field);
    };
    let ascii = 2;
    let make_len = u32::try_from(make_bytes.len()).unwrap();
    entry(MAKE, ascii, make_len, make_at.to_le_bytes());
    entry(ORIENTATION, 3, 1, [1, 0, 0, 0]);
    let packet_len = u32::try_from(packet.len()).unwrap();
    entry(tag, BYTE, packet_len, packet_at.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&make_bytes);
    bytes.extend_from_slice(&packet);
    bytes
}

#[test]
fn a_raw_files_packet_of_type_byte_or_undefined_is_given() {
    for (format, make) in RAW_MAKES {
        for typ in [BYTE, UNDEFINED] {
            let bytes = with_packet(make, XMP, typ, &packet());
            let file = CameraFile::from_bytes(&bytes).unwrap();
            assert_eq!(file.format(), format);
            assert_eq!(file.metadata().xmp(), Some(&expected()), "{make} {typ}");
            // The entry is listed as any other, its value the packet.
            let entry = file.entries().entry(Directory::Ifd(0), XMP).unwrap();
            assert_eq!(entry.byte_range().map(|range| range.end - range.start), {
                Some(u64::try_from(packet().len()).unwrap())
            });
        }
    }
}

#[test]
fn a_packet_of_another_type_gives_none() {
    // Four bytes of the packet per LONG, and as ASCII: the file opens with
    // the rest of its metadata.
    for (_, make) in RAW_MAKES {
        let mut packet = packet();
        packet.resize(packet.len().next_multiple_of(4), b' ');
        let longs = with_long_packet(make, &packet);
        for bytes in [longs, with_packet(make, XMP, 2, &packet)] {
            let file = CameraFile::from_bytes(&bytes).unwrap();
            assert_eq!(file.metadata().xmp(), None, "{make}");
            assert_eq!(file.metadata().camera().recorded_make(), Some(make));
        }
    }
}

/// [`with_packet`] with the packet's bytes under [`XMP`] as LONGs, its
/// count a quarter of its length.
fn with_long_packet(make: &str, packet: &[u8]) -> Vec<u8> {
    let count = u32::try_from(packet.len() / 4).unwrap();
    with_value(make, XMP, LONG, count, packet)
}

#[test]
fn a_packet_that_runs_past_the_end_of_the_file_gives_none_and_the_file_opens() {
    for (format, make) in RAW_MAKES {
        let mut bytes = with_packet(make, XMP, BYTE, &packet());
        bytes.truncate(bytes.len() - 1);
        let file = CameraFile::from_bytes(&bytes).unwrap();
        assert_eq!(file.format(), format);
        assert_eq!(file.metadata().xmp(), None, "{make}");
        assert_eq!(file.metadata().camera().recorded_make(), Some(make));
    }
}

#[test]
fn a_packet_that_does_not_parse_gives_none_and_stays_an_entry() {
    for (_, make) in RAW_MAKES {
        let mut broken = packet();
        // `</rdf:RDF>` written `</rdf:RDX>`: the end tag no longer matches.
        let at = broken.windows(9).position(|w| w == b"</rdf:RDF").unwrap();
        broken[at + 8] = b'X';
        let bytes = with_packet(make, XMP, BYTE, &broken);
        let file = CameraFile::from_bytes(&bytes).unwrap();
        assert_eq!(file.metadata().xmp(), None, "{make}");
        assert!(Xmp::from_bytes(&broken).is_err());
        let entry = file.entries().entry(Directory::Ifd(0), XMP).unwrap();
        let value = file.read_value(&entry).unwrap();
        assert_eq!(value.bytes(), Some(&broken[..]), "{make}");
    }
}

#[test]
fn a_packet_longer_than_the_cap_on_one_value_gives_none() {
    for (_, make) in RAW_MAKES {
        let packet = packet();
        let bytes = with_packet(make, XMP, BYTE, &packet);
        let len = u64::try_from(packet.len()).unwrap();
        let open = |cap: u64| {
            Options::new()
                .with_limits(Limits::default().with_max_value_bytes(cap))
                .from_reader(Cursor::new(bytes.clone()))
                .unwrap()
        };
        assert_eq!(open(len).metadata().xmp(), Some(&expected()), "{make}");
        assert_eq!(open(len - 1).metadata().xmp(), None, "{make}");
    }
}

#[test]
fn a_packet_among_the_directories_costs_no_read() {
    // The packet right after IFD0, as cameras write it, and IFD0 with the
    // packet in the last bytes of a file of 300,000, as an editor that
    // rewrote IFD0 leaves them. Each is opened again with the packet under
    // a tag no reader reads, which costs the same reads but the packet's.
    for (_, make) in RAW_MAKES {
        for (case, with, without) in [
            (
                "after IFD0",
                with_packet(make, XMP, BYTE, &packet()),
                with_packet(make, UNREAD, BYTE, &packet()),
            ),
            (
                "at the end",
                ifd0_at(make, XMP, 300_000),
                ifd0_at(make, UNREAD, 300_000),
            ),
        ] {
            let (file, reads) = open_counting(with);
            assert_eq!(file.metadata().xmp(), Some(&expected()), "{make} {case}");
            let (file, reads_without) = open_counting(without);
            assert_eq!(file.metadata().xmp(), None, "{make} {case}");
            assert_eq!(reads, reads_without, "{make} {case}");
        }
    }
}

#[test]
fn a_packet_far_from_the_directories_costs_one_read() {
    // IFD0 at the start, the packet 200,000 bytes in, in a block that
    // opening the file does not otherwise read.
    for (_, make) in RAW_MAKES {
        let build = |tag: u16| {
            let mut builder = TiffBuilder::new(Order::Little);
            let ifd0 = builder.ifd();
            builder
                .ascii(ifd0, MAKE, make)
                .short(ifd0, ORIENTATION, &[1]);
            let blob = builder.blob_at(200_000, &packet());
            builder.raw(
                ifd0,
                tag,
                BYTE,
                u32::try_from(packet().len()).unwrap(),
                RawValue::Offset(200_000),
            );
            let built = builder.build();
            assert_eq!(built.blob_offset(blob), 200_000);
            built.bytes
        };
        let (file, reads) = open_counting(build(XMP));
        assert_eq!(file.metadata().xmp(), Some(&expected()), "{make}");
        let (_, reads_without) = open_counting(build(UNREAD));
        assert_eq!(reads, reads_without + 1, "{make}");
    }
}

#[test]
fn the_z9_packet_is_given_whole() {
    let packet = xmp_packets::nikon_z9();
    let bytes = with_packet("NIKON CORPORATION", XMP, BYTE, &packet);
    let file = CameraFile::from_bytes(&bytes).unwrap();
    let xmp = file.metadata().xmp().expect("the Z 9's packet");
    assert_eq!(xmp, &Xmp::from_bytes(&packet).unwrap());
    let rating = xmp.property(namespaces::XMP, "Rating");
    assert_eq!(rating.and_then(raw_files::XmpValue::text), Some("0"));
}

/// The JPEG file `builder` describes, opened.
fn opened(builder: &JpegBuilder) -> CameraFile<Cursor<Vec<u8>>> {
    CameraFile::from_reader(Cursor::new(builder.build().bytes)).expect("the file opens")
}

/// The bytes of an Exif segment's TIFF structure whose IFD0 holds the Make
/// `make`.
fn exif(make: &str) -> Vec<u8> {
    let mut tiff = TiffBuilder::new(Order::Little);
    let ifd0 = tiff.ifd();
    tiff.ascii(ifd0, MAKE, make);
    tiff.build().bytes
}

/// A packet like [`packet`] with a rating of `rating`.
fn rated(rating: u8) -> Vec<u8> {
    let mut packet = packet();
    let at = packet
        .windows(10)
        .position(|w| w == b"Rating=\"3\"")
        .unwrap();
    packet[at + 8] = b'0' + rating;
    packet
}

#[test]
fn a_jpegs_packet_is_given_before_or_after_its_exif_segment() {
    for xmp_first in [false, true] {
        let mut builder = JpegBuilder::new();
        if xmp_first {
            builder.xmp_bytes(&packet()).exif(&exif("Canon"));
        } else {
            builder.exif(&exif("Canon")).xmp_bytes(&packet());
        }
        let file = opened(&builder);
        assert_eq!(file.format(), Format::Jpeg);
        assert_eq!(file.metadata().xmp(), Some(&expected()), "{xmp_first}");
        assert_eq!(file.metadata().camera().recorded_make(), Some("Canon"));
    }
    // Without an Exif segment too.
    let mut builder = JpegBuilder::new();
    builder.xmp_bytes(&packet());
    let file = opened(&builder);
    assert_eq!(file.metadata().xmp(), Some(&expected()));
}

#[test]
fn a_jpeg_with_two_xmp_segments_gives_the_first() {
    let mut builder = JpegBuilder::new();
    builder
        .xmp_bytes(&rated(1))
        .exif(&exif("NIKON CORPORATION"))
        .xmp_bytes(&rated(5));
    let file = opened(&builder);
    let rating = file
        .metadata()
        .xmp()
        .and_then(|xmp| xmp.property(namespaces::XMP, "Rating"));
    assert_eq!(rating.and_then(raw_files::XmpValue::text), Some("1"));
    // The first stands even when it does not parse.
    let mut builder = JpegBuilder::new();
    builder.xmp("<x:xmpmeta>").xmp_bytes(&rated(5));
    let file = opened(&builder);
    assert_eq!(file.metadata().xmp(), None);
}

#[test]
fn a_jpeg_with_an_extended_xmp_segment_alone_gives_no_packet() {
    // The extension's identifier, then a GUID of 32 hexadecimal digits,
    // the full length, the offset and the portion (XMP Part 3, Table 7).
    let mut extended = b"http://ns.adobe.com/xmp/extension/\0".to_vec();
    extended.extend_from_slice(b"0123456789ABCDEF0123456789ABCDEF");
    let portion = packet();
    let len = u32::try_from(portion.len()).unwrap();
    extended.extend_from_slice(&len.to_be_bytes());
    extended.extend_from_slice(&0_u32.to_be_bytes());
    extended.extend_from_slice(&portion);
    let mut builder = JpegBuilder::new();
    builder.exif(&exif("SONY")).segment(0xE1, &extended);
    let file = opened(&builder);
    assert_eq!(file.metadata().xmp(), None);
    assert_eq!(file.metadata().camera().recorded_make(), Some("SONY"));
}

#[test]
fn a_jpegs_damaged_packet_gives_none_and_the_rest_of_the_metadata() {
    let mut broken = packet();
    let at = broken.windows(9).position(|w| w == b"</rdf:RDF").unwrap();
    broken[at + 8] = b'X';
    let mut builder = JpegBuilder::new();
    builder.exif(&exif("NIKON CORPORATION")).xmp_bytes(&broken);
    let file = opened(&builder);
    assert_eq!(file.metadata().xmp(), None);
    assert_eq!(
        file.metadata().camera().recorded_make(),
        Some("NIKON CORPORATION")
    );
    assert_eq!(file.previews().len(), 1);
}

#[test]
fn a_jpegs_packet_longer_than_the_cap_on_one_value_gives_none() {
    let packet = packet();
    let mut builder = JpegBuilder::new();
    builder.xmp_bytes(&packet);
    let bytes = builder.build().bytes;
    let len = u64::try_from(packet.len()).unwrap();
    let open = |cap: u64| {
        Options::new()
            .with_limits(Limits::default().with_max_value_bytes(cap))
            .from_reader(Cursor::new(bytes.clone()))
            .unwrap()
    };
    assert_eq!(open(len).metadata().xmp(), Some(&expected()));
    assert_eq!(open(len - 1).metadata().xmp(), None);
}

#[test]
fn a_packet_in_a_jpegs_exif_segment_is_not_read() {
    // IFD0 of the Exif segment holds a packet under 0x02BC, which no
    // camera writes there: a JPEG's packet is its XMP segment's alone.
    let mut tiff = TiffBuilder::new(Order::Little);
    let ifd0 = tiff.ifd();
    tiff.ascii(ifd0, MAKE, "NIKON CORPORATION")
        .byte(ifd0, XMP, &packet());
    let mut builder = JpegBuilder::new();
    builder.exif(&tiff.build().bytes);
    let file = opened(&builder);
    assert_eq!(file.metadata().xmp(), None);
    builder.xmp_bytes(&rated(5));
    let file = opened(&builder);
    let rating = file
        .metadata()
        .xmp()
        .and_then(|xmp| xmp.property(namespaces::XMP, "Rating"));
    assert_eq!(rating.and_then(raw_files::XmpValue::text), Some("5"));
}

#[test]
fn the_lightroom_packet_is_given_whole() {
    let packet = xmp_packets::lightroom_jpeg();
    let mut builder = JpegBuilder::new();
    builder.exif(&exif("NIKON CORPORATION")).xmp_bytes(&packet);
    let file = opened(&builder);
    let xmp = file.metadata().xmp().expect("the Lightroom packet");
    assert_eq!(xmp, &Xmp::from_bytes(&packet).unwrap());
    assert!(!xmp.is_empty());
}
