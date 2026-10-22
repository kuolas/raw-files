// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG reader from outside the crate, on JPEG files built in code: what
//! they open as, which previews they list (the file itself, the thumbnail
//! of the Exif segment and the images of the MPF list), what reading one
//! returns, and the orientation and image size the metadata gives. Then
//! the files it refuses, and the damage that costs a file some of its
//! previews but never the file itself.

mod support;

use std::io::Cursor;
use std::ops::Range;

use raw_files::{ErrorKind, Format, Metadata, Orientation, Preview, PreviewFormat, RawFile, Size};
use support::jpeg_builder::{
    Built, JpegBuilder, LONG, MP_ENTRY, MP_ENTRY_AT, MP_ENTRY_LEN, MP_INDEX_AT, MPF_ID,
    MPF_VERSION, NUMBER_OF_IMAGES, UNDEFINED,
};
use support::jpeg_stub::{jpeg_stub, segment};
use support::tiff_builder::{Order, TiffBuilder};

/// The Orientation tag, in IFD0.
const ORIENTATION: u16 = 0x0112;

/// The JPEGInterchangeFormat tag: where a thumbnail's SOI lies, counted from
/// the TIFF header.
const JPEG_INTERCHANGE_FORMAT: u16 = 0x0201;

/// The JPEGInterchangeFormatLength tag: the thumbnail's length in bytes.
const JPEG_INTERCHANGE_FORMAT_LENGTH: u16 = 0x0202;

/// The Orientation value of a picture shown upright by a turn of 90
/// degrees clockwise.
const ROTATE_90: u16 = 6;

/// The size of the main image of the files built here, a Z 9's.
const MAIN: (u16, u16) = (8256, 5504);

/// The size of the thumbnail in IFD1.
const THUMBNAIL: (u16, u16) = (160, 120);

/// The sizes of the two images of the MPF list after the main one, large
/// thumbnails of classes 1 and 2.
const MPF_IMAGES: [(u16, u16); 2] = [(640, 424), (1620, 1080)];

/// `size` as a preview reports it.
fn wide((width, height): (u16, u16)) -> (u32, u32) {
    (u32::from(width), u32::from(height))
}

/// `size` as the metadata reports it.
fn size_of((width, height): (u16, u16)) -> Size {
    Size {
        width: u32::from(width),
        height: u32::from(height),
    }
}

/// `n` as a `u64`.
fn to_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap()
}

/// The length of the stub JPEG of `size`.
fn stub_len(size: (u16, u16)) -> u64 {
    to_u64(jpeg_stub(size.0, size.1, 0xC0).len())
}

/// A preview as the tests compare it: where its bytes are, its size, its
/// format and its orientation.
type Seen = (Range<u64>, (u32, u32), PreviewFormat, Orientation);

/// `preview` as the tests compare it.
fn seen(preview: &Preview) -> Seen {
    (
        preview.byte_range(),
        (preview.width(), preview.height()),
        preview.format(),
        preview.orientation(),
    )
}

/// The previews a test expects: each JPEG's range and size, smallest first,
/// all with `orientation`.
fn expected(jpegs: &[(Range<u64>, (u16, u16))], orientation: Orientation) -> Vec<Seen> {
    jpegs
        .iter()
        .map(|(range, size)| (range.clone(), wide(*size), PreviewFormat::Jpeg, orientation))
        .collect()
}

/// The whole of the file `built`, as its own preview gives it.
fn whole(built: &Built) -> Range<u64> {
    0..to_u64(built.bytes.len())
}

/// Opens `bytes` from bytes and from a reader, and checks that each opens
/// as a JPEG with the previews `previews`, smallest first, and the metadata
/// `metadata`; `label` names the file in a failure.
fn assert_opens(bytes: &[u8], previews: &[Seen], metadata: &Metadata, label: &str) {
    let from_bytes = RawFile::from_bytes(bytes).unwrap();
    let from_reader = RawFile::from_reader(Cursor::new(bytes.to_vec())).unwrap();
    for (how, format, found, read) in [
        (
            "bytes",
            from_bytes.format(),
            from_bytes.previews(),
            from_bytes.metadata(),
        ),
        (
            "reader",
            from_reader.format(),
            from_reader.previews(),
            from_reader.metadata(),
        ),
    ] {
        assert_eq!(format, Format::Jpeg, "{label}, from {how}");
        let found: Vec<Seen> = found.iter().map(seen).collect();
        assert_eq!(found, previews, "{label}, from {how}");
        assert_eq!(read, metadata, "{label}, from {how}");
    }
}

/// The metadata of a JPEG whose main image is of size [`MAIN`], shown
/// upright by `orientation`: that and the image size, and nothing else.
fn main_metadata(orientation: Orientation) -> Metadata {
    Metadata::default()
        .with_orientation(orientation)
        .with_image_size(size_of(MAIN))
}

/// The TIFF structure of an Exif segment: IFD0 with Orientation
/// `orientation`, and IFD1 with a thumbnail of [`THUMBNAIL`] that
/// JPEGInterchangeFormat and JPEGInterchangeFormatLength point at, at
/// `thumbnail_at` when given, counted from the TIFF header, and otherwise
/// where the builder puts it. Returns the bytes and where the builder put
/// the thumbnail in them.
fn exif(orientation: u16, thumbnail_at: Option<u32>) -> (Vec<u8>, Range<u64>) {
    let mut tiff = TiffBuilder::new(Order::Little);
    let (ifd0, ifd1) = (tiff.ifd(), tiff.ifd());
    tiff.short(ifd0, ORIENTATION, &[orientation])
        .next(ifd0, Some(ifd1));
    let blob = tiff.blob(&jpeg_stub(THUMBNAIL.0, THUMBNAIL.1, 0xC0));
    match thumbnail_at {
        Some(at) => {
            let len = u32::try_from(stub_len(THUMBNAIL)).unwrap();
            tiff.long(ifd1, JPEG_INTERCHANGE_FORMAT, &[at]);
            tiff.long(ifd1, JPEG_INTERCHANGE_FORMAT_LENGTH, &[len]);
        }
        None => {
            tiff.long_blob(ifd1, JPEG_INTERCHANGE_FORMAT, blob)
                .long_blob_len(ifd1, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);
        }
    }
    let built = tiff.build();
    let at = built.blob_offset(blob);
    (built.bytes, at..at + stub_len(THUMBNAIL))
}

/// Where the thumbnail of the Exif segment of `built` lies in the file:
/// `in_tiff`, where it lies in the segment's TIFF structure, from that
/// structure's header.
fn in_file(built: &Built, in_tiff: &Range<u64>) -> Range<u64> {
    let base = built.exif_tiff(0).start;
    base + in_tiff.start..base + in_tiff.end
}

/// A camera's JPEG: an Exif segment whose IFD0 has Orientation 6 and whose
/// IFD1 holds a thumbnail of [`THUMBNAIL`], an MPF list, the main image of
/// [`MAIN`], then the two [`MPF_IMAGES`], each after zeros up to a
/// multiple of 512 bytes, as the Nikon D5300's JPEG files are laid out.
/// With `jfif`, a JFIF segment comes first, as an editor writes it.
/// Returns the file and where its thumbnail lies in the TIFF structure.
fn camera_jpeg(jfif: bool) -> (Built, Range<u64>) {
    let (tiff, thumbnail) = exif(ROTATE_90, None);
    let mut builder = JpegBuilder::new();
    if jfif {
        builder.jfif();
    }
    builder
        .exif(&tiff)
        .mpf(Order::Little)
        .frame(MAIN.0, MAIN.1, 0xC0);
    for (width, height) in MPF_IMAGES {
        builder.mpf_image(width, height, &[], 512);
    }
    (builder.build(), thumbnail)
}

/// The four previews of [`camera_jpeg`], smallest first: the thumbnail,
/// the two MPF images and the file.
fn camera_jpeg_previews(built: &Built, thumbnail: &Range<u64>) -> Vec<Seen> {
    expected(
        &[
            (in_file(built, thumbnail), THUMBNAIL),
            (built.mpf_image(0), MPF_IMAGES[0]),
            (built.mpf_image(1), MPF_IMAGES[1]),
            (whole(built), MAIN),
        ],
        Orientation::Rotate90,
    )
}

#[test]
fn a_jpeg_without_exif_opens_with_the_file_as_its_one_preview() {
    let mut builder = JpegBuilder::new();
    builder.frame(MAIN.0, MAIN.1, 0xC0);
    let built = builder.build();
    let previews = expected(&[(whole(&built), MAIN)], Orientation::NoTransforms);
    // The image size and nothing else: no raw size, no camera, no capture
    // time, exposure, lens or focus point.
    let metadata = main_metadata(Orientation::NoTransforms);
    assert_opens(&built.bytes, &previews, &metadata, "no Exif");

    let file = RawFile::from_bytes(&built.bytes).unwrap();
    let read = file.metadata();
    assert_eq!(read.image_size(), Some(size_of(MAIN)));
    assert_eq!(read.raw_size(), None);
    assert_eq!(read.orientation(), Orientation::NoTransforms);
    assert_eq!(read.make(), None);
    assert_eq!(read.model(), None);
    assert_eq!(read.capture_time(), None);
    assert_eq!(read.exposure().exposure_time(), None);
    assert_eq!(read.lens(), None);
    assert_eq!(read.focus(), None);
}

#[test]
fn the_thumbnail_in_ifd1_is_a_preview_and_ifd0s_orientation_turns_every_one() {
    let (tiff, thumbnail) = exif(ROTATE_90, None);
    let mut builder = JpegBuilder::new();
    builder.exif(&tiff).frame(MAIN.0, MAIN.1, 0xC0);
    let built = builder.build();
    let previews = expected(
        &[
            (in_file(&built, &thumbnail), THUMBNAIL),
            (whole(&built), MAIN),
        ],
        Orientation::Rotate90,
    );
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::Rotate90),
        "Exif",
    );
}

#[test]
fn a_camera_jpeg_lists_the_file_the_thumbnail_and_the_mpf_images() {
    let (built, thumbnail) = camera_jpeg(false);
    let previews = camera_jpeg_previews(&built, &thumbnail);
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::Rotate90),
        "camera JPEG",
    );

    let mut file = RawFile::from_bytes(&built.bytes).unwrap();
    for preview in file.previews().to_vec() {
        let bytes = file.read_preview(&preview).unwrap();
        let range = preview.byte_range();
        let (start, end) = (
            usize::try_from(range.start).unwrap(),
            usize::try_from(range.end).unwrap(),
        );
        assert_eq!(bytes[..2], [0xFF, 0xD8], "{range:?}");
        assert_eq!(bytes, built.bytes[start..end], "{range:?}");
    }
    let at_least = file.preview_at_least(1000, 700).unwrap();
    assert_eq!(at_least.byte_range(), built.mpf_image(1));
    assert_eq!(at_least.size(), size_of(MPF_IMAGES[1]));
}

#[test]
fn a_jfif_segment_before_the_exif_segment_changes_nothing() {
    let (built, thumbnail) = camera_jpeg(true);
    let previews = camera_jpeg_previews(&built, &thumbnail);
    // The JFIF segment moved everything after it.
    let (without, _) = camera_jpeg(false);
    assert!(built.exif_tiff(0).start > without.exif_tiff(0).start);
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::Rotate90),
        "JFIF first",
    );
}

#[test]
fn the_files_own_preview_runs_through_the_padding_after_its_end() {
    // As some Sony bodies write it: zeros after the last image's EOI.
    let mut builder = JpegBuilder::new();
    builder.frame(MAIN.0, MAIN.1, 0xC0).pad(4096);
    let built = builder.build();
    assert!(built.image_end() < to_u64(built.bytes.len()));
    let previews = expected(&[(whole(&built), MAIN)], Orientation::NoTransforms);
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::NoTransforms),
        "padded",
    );

    let mut file = RawFile::from_bytes(&built.bytes).unwrap();
    let preview = file.largest_preview().unwrap();
    assert_eq!(file.read_preview(&preview).unwrap(), built.bytes);
}

#[test]
fn a_thumbnail_outside_the_exif_segment_is_not_a_preview() {
    // IFD1 points at a whole JPEG that lies after the main image's EOI:
    // inside the file, outside the Exif segment. The TIFF structure's
    // length does not depend on the offset, so the file is built once to
    // learn where things land, then again with the offset that points
    // there.
    let build = |thumbnail_at: u32| {
        let (tiff, _) = exif(ROTATE_90, Some(thumbnail_at));
        let mut builder = JpegBuilder::new();
        builder
            .exif(&tiff)
            .frame(MAIN.0, MAIN.1, 0xC0)
            .after_eoi(&jpeg_stub(THUMBNAIL.0, THUMBNAIL.1, 0xC0));
        builder.build()
    };
    let first = build(0);
    let at = first.after_eoi(0) - first.exif_tiff(0).start;
    let built = build(u32::try_from(at).unwrap());
    assert_eq!(built.after_eoi(0), first.after_eoi(0));
    assert_eq!(built.exif_tiff(0), first.exif_tiff(0));
    assert!(built.after_eoi(0) >= built.exif_tiff(0).end);

    let previews = expected(&[(whole(&built), MAIN)], Orientation::Rotate90);
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::Rotate90),
        "thumbnail outside the segment",
    );
}

#[test]
fn an_mpf_image_that_runs_past_the_end_of_the_file_is_dropped() {
    let mut builder = JpegBuilder::new();
    builder.mpf(Order::Little).frame(MAIN.0, MAIN.1, 0xC0);
    for (width, height) in MPF_IMAGES {
        builder.mpf_image(width, height, &[], 512);
    }
    // The second image's entry, after the main image's and the first's,
    // declares more bytes than the file holds after its start.
    builder.mpf_entry_size(2, u32::MAX);
    let built = builder.build();
    let previews = expected(
        &[(built.mpf_image(0), MPF_IMAGES[0]), (whole(&built), MAIN)],
        Orientation::NoTransforms,
    );
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::NoTransforms),
        "MPF image past the end",
    );
}

#[test]
fn a_jpeg_without_a_picture_to_show_is_of_unknown_format() {
    // A lossless frame header: raw data, not a picture.
    let mut lossless = JpegBuilder::new();
    lossless.frame(MAIN.0, MAIN.1, 0xC3);
    // SOS before the frame header: the walk ends there.
    let mut sos_first = JpegBuilder::new();
    sos_first.segment(0xDA, &[1, 1, 0x00, 0, 63, 0]);
    // A frame header with a zero side, which gives no size.
    let mut zero_width = JpegBuilder::new();
    zero_width.frame(0, MAIN.1, 0xC0);
    for (case, builder) in [
        ("lossless", lossless),
        ("SOS first", sos_first),
        ("zero width", zero_width),
    ] {
        let bytes = builder.build().bytes;
        let err = RawFile::from_bytes(&bytes).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{case}: {err:?}");
        assert_eq!(err.offset(), None, "{case}");
        if case == "lossless" {
            assert_eq!(
                err.to_string(),
                "the file is a JPEG file, but holds no frame header that gives the picture's \
                 size and is not lossless"
            );
        }
        let err = RawFile::from_reader(Cursor::new(bytes)).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{case}: {err:?}");
    }
}

#[test]
fn a_damaged_exif_segment_costs_the_thumbnail_and_the_orientation_only() {
    // A file laid out as `camera_jpeg`, whose Exif segment holds the TIFF
    // structure `tiff`.
    let build = |tiff: &[u8]| {
        let mut builder = JpegBuilder::new();
        builder
            .exif(tiff)
            .mpf(Order::Little)
            .frame(MAIN.0, MAIN.1, 0xC0);
        for (width, height) in MPF_IMAGES {
            builder.mpf_image(width, height, &[], 512);
        }
        builder.build()
    };
    let (sound, _) = exif(ROTATE_90, None);

    // A TIFF header whose byte-order mark is neither `II` nor `MM`.
    let mut no_header = sound.clone();
    no_header[..2].copy_from_slice(b"XX");
    // IFD0 at the main image's frame header: inside the file, outside the
    // segment. The structure's length does not depend on the offset.
    let first = build(&sound);
    let ifd0_at = u32::try_from(first.frame_header() - first.exif_tiff(0).start).unwrap();
    let mut ifd0_outside = sound.clone();
    ifd0_outside[4..8].copy_from_slice(&ifd0_at.to_le_bytes());

    for (case, tiff) in [
        ("no TIFF header", no_header),
        ("IFD0 outside", ifd0_outside),
    ] {
        let built = build(&tiff);
        assert_eq!(built.frame_header(), first.frame_header(), "{case}");
        let previews = expected(
            &[
                (built.mpf_image(0), MPF_IMAGES[0]),
                (built.mpf_image(1), MPF_IMAGES[1]),
                (whole(&built), MAIN),
            ],
            Orientation::NoTransforms,
        );
        assert_opens(
            &built.bytes,
            &previews,
            &main_metadata(Orientation::NoTransforms),
            case,
        );
    }
}

#[test]
fn a_full_mpf_list_of_images_that_never_reach_a_frame_header_costs_only_time() {
    // The most work an MPF list can ask of the reader, near enough: a
    // segment as full of entries as its length allows beside the tags the
    // standard requires (an index with MPEntry alone has room for one
    // more), every one after the first pointing at the same bytes, which
    // hold a frame header further in than the walk's 1,024 steps reach.
    // Each entry is probed in full, since only one that becomes a preview
    // claims its position, and none is a preview. The test checks that
    // such a file still opens, as the JPEG it is, with the file as its one
    // preview.
    let mut hostile = vec![0xFF, 0xD8];
    for _ in 0..1100 {
        // An empty comment: one step each.
        hostile.extend(segment(0xFE, &[]));
    }
    hostile.extend_from_slice(&jpeg_stub(MPF_IMAGES[1].0, MPF_IMAGES[1].1, 0xC0)[2..]);

    // The MPF payload: the identifier, the MP header and the MP Index IFD,
    // then as many entries as the rest of a segment's 65,533 bytes holds.
    let most = usize::from(u16::MAX) - 2;
    let entries = (most - MPF_ID.len() - usize::try_from(MP_ENTRY_AT).unwrap()) / MP_ENTRY_LEN;
    let payload_len = MPF_ID.len() + usize::try_from(MP_ENTRY_AT).unwrap() + entries * MP_ENTRY_LEN;
    let payload = |header: u64, image_end: u64, hostile_at: u64| {
        let entry_len = u32::try_from(entries * MP_ENTRY_LEN).unwrap();
        let mut payload = MPF_ID.to_vec();
        payload.extend_from_slice(b"II");
        payload.extend_from_slice(&42_u16.to_le_bytes());
        payload.extend_from_slice(&MP_INDEX_AT.to_le_bytes());
        payload.extend_from_slice(&3_u16.to_le_bytes());
        for (tag, typ, count, field) in [
            (MPF_VERSION, UNDEFINED, 4, *b"0100"),
            (
                NUMBER_OF_IMAGES,
                LONG,
                1,
                u32::try_from(entries).unwrap().to_le_bytes(),
            ),
            (MP_ENTRY, UNDEFINED, entry_len, MP_ENTRY_AT.to_le_bytes()),
        ] {
            payload.extend_from_slice(&tag.to_le_bytes());
            payload.extend_from_slice(&typ.to_le_bytes());
            payload.extend_from_slice(&u32::to_le_bytes(count));
            payload.extend_from_slice(&field);
        }
        payload.extend_from_slice(&[0; 4]);
        let mut entry = |attribute: u32, size: u64, offset: u64| {
            payload.extend_from_slice(&attribute.to_le_bytes());
            payload.extend_from_slice(&u32::try_from(size).unwrap().to_le_bytes());
            payload.extend_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
            payload.extend_from_slice(&[0; 4]);
        };
        entry(0xA003_0000, image_end, 0);
        for _ in 1..entries {
            entry(0x4001_0002, to_u64(hostile.len()), hostile_at - header);
        }
        payload
    };
    let build = |payload: &[u8]| {
        let mut builder = JpegBuilder::new();
        builder
            .app2(payload)
            .frame(MAIN.0, MAIN.1, 0xC0)
            .after_eoi(&hostile);
        builder.build()
    };
    // Built once to learn where things land, then with the payload.
    let first = build(&vec![0; payload_len]);
    let header = first.app2_payload(0).start + to_u64(MPF_ID.len());
    let full = payload(header, first.image_end(), first.after_eoi(0));
    assert_eq!(full.len(), payload_len);
    assert!(
        most - payload_len < MP_ENTRY_LEN,
        "no room for one more entry"
    );
    let built = build(&full);
    assert_eq!(built.after_eoi(0), first.after_eoi(0));

    let previews = expected(&[(whole(&built), MAIN)], Orientation::NoTransforms);
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::NoTransforms),
        "full MPF list",
    );
}

#[test]
fn a_nefs_embedded_jpeg_saved_alone_opens_as_a_jpeg() {
    let stub = jpeg_stub(MPF_IMAGES[1].0, MPF_IMAGES[1].1, 0xC0);
    let previews = expected(
        &[(0..to_u64(stub.len()), MPF_IMAGES[1])],
        Orientation::NoTransforms,
    );
    let metadata = Metadata::default().with_image_size(size_of(MPF_IMAGES[1]));
    assert_opens(&stub, &previews, &metadata, "embedded JPEG");
}
