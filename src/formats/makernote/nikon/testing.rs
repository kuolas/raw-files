// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the unit tests of more than one file of the Nikon reader
//! share, with the helpers those build on: MakerNote blocks and files
//! built in code, and the AFInfo2 records and shots built from them.

use std::io::Cursor;

use super::block::{HEADER_AT, MakerNote, read};
use super::{AF_INFO2, CROP_HI_SPEED, FOCUS_MODE, PREVIEW_IFD, Picture, metadata};
use crate::container::tiff::tags::{
    IMAGE_LENGTH, IMAGE_WIDTH, JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKE,
    MAKER_NOTE, MODEL, PHOTOMETRIC_INTERPRETATION,
};
use crate::container::tiff::{self, walk};
use crate::focus::{Focus, FocusAreaMode, FocusOrigin, FocusPoint, FocusPoints};
use crate::formats::tiff_metadata::{self, Standard};
use crate::formats::tiff_previews::Candidate;
use crate::geometry::{Point, Size};
use crate::io::Source;
use crate::jpeg_stub::jpeg_stub;
use crate::limits::Limits;
use crate::metadata::Metadata;
use crate::tiff_builder::{IfdId, LONG, Order, RawValue, TiffBuilder, UNDEFINED};

pub(super) const ORDERS: [Order; 2] = [Order::Little, Order::Big];

/// Where the tests put the MakerNote block in the file.
pub(super) const AT: u64 = 1024;

/// How far the file goes on past the block, so that what a test points
/// past the block still lies inside the file.
pub(super) const TAIL: u64 = 4096;

/// The Nikon MakerNote tag MakerNoteVersion.
pub(super) const MAKER_NOTE_VERSION: u16 = 0x0001;

/// What a test changes in the inner TIFF structure of a MakerNote
/// block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Change {
    /// Nothing: a sound block.
    Nothing,
    /// The inner header's 42 is 41.
    Magic41,
    /// The PreviewIFD declares no JPEG.
    NoJpegTags,
    /// The PreviewIFD declares the JPEG's offset, not its length.
    NoJpegLength,
    /// The PreviewIFD pointer is stored out of line, inside the block,
    /// as two LONGs whose first is the right offset.
    PointerOutOfLine,
    /// The PreviewIFD holds, before its JPEGInterchangeFormat, another
    /// one of two LONGs, stored out of line in the 8 bytes just past the
    /// block's end, which lie inside the file. The JPEG's length is then
    /// the PreviewIFD's third entry, not the one [`Block::length_at`]
    /// gives.
    JpegOffsetPastTheBlockFirst,
}

/// A MakerNote block and where its parts are, each counted from the
/// start of the block.
pub(super) struct Block {
    pub(super) bytes: Vec<u8>,
    order: Order,
    /// The MakerNote IFD.
    pub(super) makernote: u64,
    /// The PreviewIFD.
    pub(super) preview: u64,
    /// The JPEG: 10, where the inner structure starts, plus the JPEG's
    /// offset in it.
    pub(super) jpeg: u64,
    /// The JPEG's length.
    pub(super) jpeg_len: u64,
}

impl Block {
    /// The block's length.
    pub(super) fn len(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap()
    }

    /// Writes `value` at `at`, in the block's byte order.
    pub(super) fn put_u32(&mut self, at: u64, value: u32) {
        let at = usize::try_from(at).unwrap();
        self.bytes[at..at + 4].copy_from_slice(&self.order.u32(value));
    }

    /// Where the value field of the MakerNote IFD's PreviewIFD entry
    /// is: the second entry, after MakerNoteVersion, 8 bytes in.
    pub(super) fn pointer_at(&self) -> u64 {
        self.makernote + 2 + 12 + 8
    }

    /// Where the value field of the PreviewIFD's
    /// JPEGInterchangeFormatLength entry is: the second entry, after
    /// JPEGInterchangeFormat, 8 bytes in.
    pub(super) fn length_at(&self) -> u64 {
        self.preview + 2 + 12 + 8
    }

    /// Where the value field of the PreviewIFD's JPEGInterchangeFormat
    /// entry is: the first entry, 8 bytes in.
    pub(super) fn offset_at(&self) -> u64 {
        self.preview + 2 + 8
    }

    /// Rewrites the entry whose value field is at `value_at` with type
    /// `typ`, count `count` and value field `value`, in the block's
    /// byte order; its tag is kept.
    pub(super) fn put_entry(&mut self, value_at: u64, typ: u16, count: u32, value: [u8; 4]) {
        let at = usize::try_from(value_at).unwrap();
        self.bytes[at - 6..at - 4].copy_from_slice(&self.order.u16(typ));
        self.bytes[at - 4..at].copy_from_slice(&self.order.u32(count));
        self.bytes[at..at + 4].copy_from_slice(&value);
    }

    /// The candidate the block gives when it sits at `at` in the file.
    pub(super) fn candidate(&self, at: u64) -> Candidate {
        Candidate {
            offset: at + self.jpeg,
            length: self.jpeg_len,
        }
    }
}

/// A Nikon MakerNote block of the embedded-TIFF-header layout, in byte
/// order `order`, changed as `change` says: `Nikon\0`, the version
/// `02 11`, two zero bytes, then an inner TIFF structure whose offsets
/// count from its own first byte, as the MakerNote's do. Its MakerNote
/// IFD holds MakerNoteVersion and the PreviewIFD pointer; its PreviewIFD
/// declares a 640 by 424 stub JPEG, the last bytes of the block.
pub(super) fn block(order: Order, change: Change) -> Block {
    let mut inner = TiffBuilder::new(order);
    let makernote = inner.ifd();
    let preview = inner.ifd();
    inner.undefined(makernote, MAKER_NOTE_VERSION, b"0211");
    if change == Change::PointerOutOfLine {
        // The PreviewIFD's offset depends only on the IFD before it,
        // so a copy with a stand-in value of the same length gives it.
        let mut probe = inner.clone();
        probe.long(makernote, PREVIEW_IFD, &[0, 0]);
        let offset = u32::try_from(probe.build().ifd_offset(preview)).unwrap();
        inner.long(makernote, PREVIEW_IFD, &[offset, 0]);
    } else {
        inner.ifd_pointer(makernote, PREVIEW_IFD, preview);
    }
    if change == Change::Magic41 {
        inner.magic(41);
    }
    let stub = jpeg_stub(640, 424, 0xC0);
    let jpeg = inner.blob(&stub);
    if change == Change::JpegOffsetPastTheBlockFirst {
        // Entries of the same tag keep the order they were added in, so
        // this one comes first. Its offset is set once the block's
        // length is known.
        inner.raw(
            preview,
            JPEG_INTERCHANGE_FORMAT,
            LONG,
            2,
            RawValue::Offset(0),
        );
    }
    if change != Change::NoJpegTags {
        inner.long_blob(preview, JPEG_INTERCHANGE_FORMAT, jpeg);
    }
    if !matches!(change, Change::NoJpegTags | Change::NoJpegLength) {
        inner.long_blob_len(preview, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg);
    }
    let built = inner.build();

    let mut bytes = b"Nikon\0\x02\x11\0\0".to_vec();
    bytes.extend_from_slice(&built.bytes);
    let mut block = Block {
        bytes,
        order,
        makernote: HEADER_AT + built.ifd_offset(makernote),
        preview: HEADER_AT + built.ifd_offset(preview),
        jpeg: HEADER_AT + built.blob_offset(jpeg),
        jpeg_len: u64::try_from(stub.len()).unwrap(),
    };
    if change == Change::JpegOffsetPastTheBlockFirst {
        // The block's end, counted from the inner header, written in
        // the value field of the PreviewIFD's first entry, 8 bytes in.
        let end = u32::try_from(block.len() - HEADER_AT).unwrap();
        block.put_u32(block.offset_at(), end);
    }
    block
}

/// A Nikon file in byte order `order`: IFD0 holds Make and points at an
/// Exif IFD, which holds ExifVersion and what `exif` adds. `bytes` lie
/// at `at`, and the file ends `tail` bytes after them.
pub(super) fn nikon_file(
    order: Order,
    at: u64,
    bytes: &[u8],
    tail: u64,
    exif: impl FnOnce(&mut TiffBuilder, IfdId),
) -> Vec<u8> {
    nikon_file_with(order, at, bytes, tail, |builder, _, exif_ifd| {
        exif(builder, exif_ifd);
    })
}

/// As [`nikon_file`], with `build` given the builder, IFD0 and the Exif
/// IFD, to add to either.
pub(super) fn nikon_file_with(
    order: Order,
    at: u64,
    bytes: &[u8],
    tail: u64,
    build: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
) -> Vec<u8> {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let exif_ifd = builder.ifd();
    builder
        .ascii(ifd0, MAKE, "NIKON CORPORATION")
        .exif(ifd0, exif_ifd);
    builder.undefined(exif_ifd, 0x9000, b"0232");
    build(&mut builder, ifd0, exif_ifd);
    builder.blob_at(at, bytes);
    builder.pad_to(at + u64::try_from(bytes.len()).unwrap() + tail);
    builder.build().bytes
}

/// Adds to `ifd` an entry of tag `tag` and type `typ` whose value is
/// the `count` bytes at `at`.
pub(super) fn entry_at(
    builder: &mut TiffBuilder,
    ifd: IfdId,
    tag: u16,
    typ: u16,
    at: u64,
    count: u64,
) {
    let count = u32::try_from(count).unwrap();
    let at = u32::try_from(at).unwrap();
    builder.raw(ifd, tag, typ, count, RawValue::Offset(at));
}

/// A Nikon file in byte order `order` holding `bytes` at [`AT`], whose
/// Exif IFD's MakerNote, of type `typ`, is the first `count` of them.
pub(super) fn with_makernote(order: Order, typ: u16, bytes: &[u8], count: u64) -> Vec<u8> {
    nikon_file(order, AT, bytes, TAIL, |builder, exif| {
        entry_at(builder, exif, MAKER_NOTE, typ, AT, count);
    })
}

/// The raw size of the files the CropArea tests build: the Z 9's.
pub(super) const RAW: Size = Size {
    width: 8280,
    height: 5520,
};

/// The PhotometricInterpretation of a color filter array: raw data, as
/// the preview finder takes it.
pub(super) const CFA: u16 = 32803;

/// A Nikon MakerNote block in byte order `order` whose MakerNote IFD
/// holds the MakerNoteVersion and what `tags` adds, given the inner
/// builder and that IFD. When that is one tag, such as a CropArea, whose
/// value is stored out of line, the value is the last bytes of the
/// block. Also where the IFD's second entry is, the one tag's when
/// `tags` adds one, counted from the start of the block.
pub(super) fn tags_block(
    order: Order,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Vec<u8>, u64) {
    let mut inner = TiffBuilder::new(order);
    let makernote = inner.ifd();
    inner.undefined(makernote, MAKER_NOTE_VERSION, b"0211");
    tags(&mut inner, makernote);
    let built = inner.build();
    let mut bytes = b"Nikon\0\x02\x11\0\0".to_vec();
    bytes.extend_from_slice(&built.bytes);
    (bytes, HEADER_AT + built.ifd_offset(makernote) + 2 + 12)
}

/// A Nikon file in byte order `order` whose IFD0 holds raw data of size
/// [`RAW`] and what `raw` adds, given the builder and IFD0, and whose
/// MakerNote, at [`AT`], is the first `count` bytes of `block`.
pub(super) fn raw_file(
    order: Order,
    block: &[u8],
    count: u64,
    raw: impl FnOnce(&mut TiffBuilder, IfdId),
) -> Vec<u8> {
    raw_file_of(order, RAW, block, count, raw)
}

/// As [`raw_file`], with raw data of size `size`.
pub(super) fn raw_file_of(
    order: Order,
    size: Size,
    block: &[u8],
    count: u64,
    raw: impl FnOnce(&mut TiffBuilder, IfdId),
) -> Vec<u8> {
    nikon_file_with(order, AT, block, TAIL, |builder, ifd0, exif| {
        builder
            .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[CFA])
            .long(ifd0, IMAGE_WIDTH, &[size.width])
            .long(ifd0, IMAGE_LENGTH, &[size.height]);
        raw(builder, ifd0);
        entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, count);
    })
}

/// The file `bytes`, read as a format's reader reads it before it
/// calls [`metadata`]: a source of it, its MakerNote, which must be
/// read, and what the standard tags give.
///
/// [`metadata`]: fn@super::metadata
pub(super) fn read_file(bytes: Vec<u8>) -> (Source<Cursor<Vec<u8>>>, MakerNote, Standard) {
    let mut src = Source::new(Cursor::new(bytes)).unwrap();
    let file_block = tiff::Block::file(src.len());
    let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
    let makernote = read(&mut src, &tree, &Limits::default())
        .unwrap()
        .expect("a MakerNote");
    let standard = tiff_metadata::read(&mut src, &tree).unwrap();
    (src, makernote, standard)
}

/// The metadata of the file `bytes`, a raw file: what the standard
/// tags give, with what [`metadata`] adds from the file's MakerNote,
/// which must be read.
///
/// [`metadata`]: fn@super::metadata
pub(super) fn metadata_of(bytes: Vec<u8>) -> Metadata {
    let (mut src, makernote, standard) = read_file(bytes);
    metadata(&mut src, &makernote, standard, Picture::Raw).unwrap()
}

/// The metadata of the file `bytes` read as a JPEG whose frame header
/// gives `pixels`: what the standard tags give, with no raw size and
/// `pixels` as the image size, as the JPEG reader builds it, then what
/// [`metadata`] adds from the file's MakerNote, which must be read,
/// for a JPEG's picture. The file must hold no raw data.
///
/// [`metadata`]: fn@super::metadata
pub(super) fn jpeg_metadata_of(bytes: Vec<u8>, [width, height]: [u32; 2]) -> Metadata {
    let (mut src, makernote, mut standard) = read_file(bytes);
    assert_eq!(standard.metadata.raw_size, None);
    standard.metadata.image_size = Some(Size { width, height });
    metadata(&mut src, &makernote, standard, Picture::Jpeg).unwrap()
}

/// A made-up serial number of seven digits, as the bodies from the
/// D300 on write theirs; no camera's serial number is copied here.
pub(super) const SERIAL: &str = "7654321";

/// The count of a MakerNote that is all of `block`.
pub(super) fn whole(block: &[u8]) -> u64 {
    u64::try_from(block.len()).unwrap()
}

/// A Nikon file in byte order `order` whose Exif IFD holds what `exif`
/// adds, given the builder and that IFD, and whose MakerNote, at
/// [`AT`], is the first `count` bytes of `block`.
pub(super) fn exif_file(
    order: Order,
    block: &[u8],
    count: u64,
    exif: impl FnOnce(&mut TiffBuilder, IfdId),
) -> Vec<u8> {
    nikon_file_with(order, AT, block, TAIL, |builder, _, exif_ifd| {
        exif(builder, exif_ifd);
        entry_at(builder, exif_ifd, MAKER_NOTE, UNDEFINED, AT, count);
    })
}

/// The metadata of the file [`exif_file`] builds in byte order `order`
/// with the MakerNote `block`, declared whole, and what `exif` adds to
/// the Exif IFD.
pub(super) fn exif_metadata(
    order: Order,
    block: &[u8],
    exif: impl FnOnce(&mut TiffBuilder, IfdId),
) -> Metadata {
    metadata_of(exif_file(order, block, whole(block), exif))
}

/// The metadata of a file in byte order `order` whose MakerNote holds
/// what `tags` adds to its MakerNote IFD, and whose Exif IFD holds no
/// serial number, ISO or lens.
pub(super) fn makernote_metadata(
    order: Order,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> Metadata {
    exif_metadata(order, &tags_block(order, tags).0, |_, _| {})
}

/// `block` with the type and count of the entry at `entry` changed to
/// `typ` and `count`, its value left as it was.
pub(super) fn retyped(
    order: Order,
    mut block: Vec<u8>,
    entry: u64,
    typ: u16,
    count: u32,
) -> Vec<u8> {
    let entry = usize::try_from(entry).unwrap();
    block[entry + 2..entry + 4].copy_from_slice(&order.u16(typ));
    block[entry + 4..entry + 8].copy_from_slice(&order.u32(count));
    block
}

/// Where a test's AFInfo2 value puts each field: the value's length,
/// then the byte offsets of the frame, the center, the area and the
/// in-focus byte, as the NEF notes' table gives them for each version.
/// Written out here, apart from [`layout`], so that a wrong offset there
/// reads a field from the wrong place.
///
/// [`layout`]: super::af_info2::layout
#[derive(Clone, Copy, Debug)]
pub(super) struct Fields {
    pub(super) len: usize,
    pub(super) frame: usize,
    pub(super) center: usize,
    pub(super) area: usize,
    pub(super) in_focus: Option<usize>,
}

/// The fields of version 0100.
pub(super) const FIELDS_0100: Fields = Fields {
    len: 30,
    frame: 16,
    center: 20,
    area: 24,
    in_focus: Some(28),
};

/// The fields of version 0101.
pub(super) const FIELDS_0101: Fields = Fields {
    len: 84,
    frame: 70,
    center: 74,
    area: 78,
    in_focus: Some(82),
};

/// The fields of versions 0300 and 0301, which have no in-focus byte.
pub(super) const FIELDS_03XX: Fields = Fields {
    len: 60,
    frame: 42,
    center: 46,
    area: 50,
    in_focus: None,
};

/// The fields of versions 0400, 0401 and 0402.
pub(super) const FIELDS_04XX: Fields = Fields {
    len: 80,
    frame: 62,
    center: 66,
    area: 70,
    in_focus: Some(74),
};

/// The fields of the AFInfo2 version `version`, one of those read.
fn fields_of(version: &str) -> Fields {
    match version {
        "0100" => FIELDS_0100,
        "0101" => FIELDS_0101,
        "0300" | "0301" => FIELDS_03XX,
        "0400" | "0401" | "0402" => FIELDS_04XX,
        _ => panic!("no fields for version {version}"),
    }
}

/// What an AFInfo2 value records: the frame's width and height, the
/// center's x and y, the area's width and height, and the in-focus
/// byte, written only where the version has one.
#[derive(Clone, Copy, Debug)]
pub(super) struct Recorded {
    pub(super) frame: [u16; 2],
    pub(super) center: [u16; 2],
    pub(super) area: [u16; 2],
    pub(super) in_focus: u8,
}

/// The Z 6's record, version 0300, which has no in-focus byte (public
/// sample 3587).
pub(super) const Z6: Recorded = Recorded {
    frame: [6048, 4024],
    center: [3024, 1160],
    area: [285, 308],
    in_focus: 1,
};

/// The Z 9's record, version 0400, confirmed (public samples 5146 to
/// 5148).
pub(super) const Z9: Recorded = Recorded {
    frame: [8256, 5504],
    center: [4905, 2461],
    area: [291, 323],
    in_focus: 1,
};

/// An AFInfo2 value in byte order `order`, `fields.len` bytes long: the
/// four bytes of `version`, then zeros, with `recorded` at the offsets
/// of `fields`, its in-focus byte only where `fields` has one.
pub(super) fn af_info2_with(
    order: Order,
    version: &str,
    fields: Fields,
    recorded: Recorded,
) -> Vec<u8> {
    let mut value = vec![0; fields.len];
    value[..4].copy_from_slice(version.as_bytes());
    for (at, [first, second]) in [
        (fields.frame, recorded.frame),
        (fields.center, recorded.center),
        (fields.area, recorded.area),
    ] {
        value[at..at + 2].copy_from_slice(&order.u16(first));
        value[at + 2..at + 4].copy_from_slice(&order.u16(second));
    }
    if let Some(at) = fields.in_focus {
        value[at] = recorded.in_focus;
    }
    value
}

/// An AFInfo2 value of version `version`, one of those read, in byte
/// order `order`, as long as its layout, recording `recorded`.
pub(super) fn af_info2(order: Order, version: &str, recorded: Recorded) -> Vec<u8> {
    af_info2_with(order, version, fields_of(version), recorded)
}

/// The focus that `recorded` describes, with `in_focus` as whether
/// focus was confirmed.
pub(super) fn expected(recorded: Recorded, in_focus: Option<bool>) -> Focus {
    let [width, height] = recorded.frame.map(u32::from);
    let [x, y] = recorded.center.map(u32::from);
    let [area_width, area_height] = recorded.area.map(u32::from);
    let focus = Focus::new(
        Size { width, height },
        Point { x, y },
        FocusOrigin::Recorded,
    )
    .and_then(|focus| {
        focus.with_size(Size {
            width: area_width,
            height: area_height,
        })
    })
    .expect("a usable record");
    match in_focus {
        Some(in_focus) => focus.with_in_focus(in_focus),
        None => focus,
    }
}

/// A record of no position: a zero frame, center and area, and an
/// in-focus byte of 0, as a D600 shot through the viewfinder writes it.
const NO_POSITION: Recorded = Recorded {
    frame: [0, 0],
    center: [0, 0],
    area: [0, 0],
    in_focus: 0,
};

/// `value`, an AFInfo2 value, naming points where version 0100 keeps
/// them, written out here apart from [`Layout`]: the layout code
/// `layout_code` at byte 6, the primary point `primary` at byte 7, and
/// the mask's `bits` set in bytes 8 to 14, bit k as bit k % 8 of byte
/// 8 + k / 8.
///
/// [`Layout`]: super::af_info2::Layout
pub(super) fn naming(mut value: Vec<u8>, layout_code: u8, primary: u8, bits: &[usize]) -> Vec<u8> {
    value[6] = layout_code;
    value[7] = primary;
    for &bit in bits {
        value[8 + bit / 8] |= 1 << (bit % 8);
    }
    value
}

/// An AFInfo2 value of version `version`, one of those read, in byte
/// order `order`, that records no position and names points as
/// [`naming`] writes them, with detection method 0 and area mode 0.
pub(super) fn viewfinder(
    order: Order,
    version: &str,
    layout_code: u8,
    primary: u8,
    bits: &[usize],
) -> Vec<u8> {
    naming(
        af_info2(order, version, NO_POSITION),
        layout_code,
        primary,
        bits,
    )
}

/// The AFInfo2 value, version 0100, in byte order `order`, of a D600
/// shot through the viewfinder in single-point autofocus with point 38,
/// as frame 4 of the D600 session in the NEF notes records it: detection
/// method 0, area mode 0, layout code 3, primary point 38, bit 37 of the
/// mask set, and a zero frame and area.
pub(super) fn d600_viewfinder(order: Order) -> Vec<u8> {
    viewfinder(order, "0100", 3, 38, &[37])
}

/// The D600's CropHiSpeed with the crop off, as its FX frames record it.
pub(super) const D600_FX: [u16; 7] = [0, 6080, 4028, 6080, 4028, 0, 0];

/// A file in byte order `order` that holds no raw data, whose IFD0's
/// Model is `model` and whose MakerNote holds the FocusMode
/// `focus_mode`, the AFInfo2 value `af_info2` and, when given, the
/// CropHiSpeed `crop_hi_speed`, as SHORTs.
pub(super) fn shot_file(
    order: Order,
    model: &str,
    focus_mode: &str,
    af_info2: &[u8],
    crop_hi_speed: Option<[u16; 7]>,
) -> Vec<u8> {
    let block = tags_block(order, |inner, ifd| {
        inner.ascii(ifd, FOCUS_MODE, focus_mode);
        if let Some(values) = crop_hi_speed {
            inner.short(ifd, CROP_HI_SPEED, &values);
        }
        inner.undefined(ifd, AF_INFO2, af_info2);
    })
    .0;
    let count = whole(&block);
    nikon_file_with(order, AT, &block, TAIL, |builder, ifd0, exif| {
        builder.ascii(ifd0, MODEL, model);
        entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, count);
    })
}

/// The metadata of the file [`shot_file`] builds, read as a raw file,
/// in autofocus, `AF-S  `.
pub(super) fn shot(
    order: Order,
    model: &str,
    af_info2: &[u8],
    crop_hi_speed: Option<[u16; 7]>,
) -> Metadata {
    metadata_of(shot_file(order, model, "AF-S  ", af_info2, crop_hi_speed))
}

/// The box of every D600 point, 164 by 164.
pub(super) const D600_BOX: Size = Size {
    width: 164,
    height: 164,
};

/// The points placed from a table in a frame of `frame`: each of
/// `points`, its number and its center, in that order, with the D600's
/// box and nothing said of focus; the one numbered `primary` the
/// primary; the area mode `mode`; the origin a table. Built here from
/// the record's values in the NEF notes, apart from [`placed`].
///
/// [`placed`]: super::crop::placed
pub(super) fn placed_list(
    frame: [u32; 2],
    points: &[(u16, [u32; 2])],
    primary: u16,
    mode: FocusAreaMode,
) -> FocusPoints {
    let [width, height] = frame;
    let mut list = FocusPoints::new(Size { width, height }, FocusOrigin::Table).unwrap();
    for &(number, [x, y]) in points {
        let point = FocusPoint::new(Point { x, y })
            .with_number(number)
            .with_size(D600_BOX);
        list = list.with_point(point).expect("a point on the frame");
    }
    let index = points
        .iter()
        .position(|&(number, _)| number == primary)
        .expect("the primary among the points");
    list.with_primary_index(index).unwrap().with_area_mode(mode)
}

/// Point 38, B1, at (1812, 1705) in the D600's 6016 by 4016 frame, as
/// the record in the NEF notes gives it.
pub(super) const B1: (u16, [u32; 2]) = (38, [1812, 1705]);

/// What a test adds to an IFD, given the builder and the IFD: a tag of
/// the MakerNote IFD or of the Exif IFD.
pub(super) type Tags = fn(&mut TiffBuilder, IfdId);
