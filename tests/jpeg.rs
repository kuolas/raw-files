// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG reader from outside the crate, on JPEG files built in code: what
//! they open as, which previews they list (the file itself, the thumbnail
//! of the Exif segment and the images of the MPF list), what reading one
//! returns, and the metadata it gives: every value the Exif segment
//! records, read as in a raw file, and the image size of the picture,
//! whatever size the tags give. Then the files it refuses, and the damage
//! that costs a file some of its previews or of its metadata but never the
//! file itself.

mod support;

use std::io::Cursor;
use std::ops::Range;

use raw_files::{
    DateTime, ErrorKind, Exposure, Format, Lens, Limits, Metadata, Options, Orientation, Preview,
    PreviewFormat, Rational, RawFile, SignedRational, Size,
};
use support::jpeg_builder::{
    Built, JpegBuilder, LONG, MP_ENTRY, MP_ENTRY_AT, MP_ENTRY_LEN, MP_INDEX_AT, MPF_ID,
    MPF_VERSION, NUMBER_OF_IMAGES, UNDEFINED,
};
use support::jpeg_files::{
    APERTURE, BODY_SERIAL_NUMBER, CAMERA_MAKE, CAMERA_MODEL, CAMERA_SERIAL_NUMBER,
    DATE_TIME_ORIGINAL, EXIF_IFD_TAGS, EXPOSURE_BIAS, EXPOSURE_BIAS_VALUE, EXPOSURE_SECONDS,
    EXPOSURE_TIME, F_NUMBER, FOCAL_LENGTH, FOCAL_MILLIMETRES, ISO, LENS, LENS_MAKE, LENS_MODEL,
    LENS_RANGE, LENS_SERIAL_NUMBER, LENS_SPECIFICATION, MAIN, MAKE, MODEL, MPF_IMAGES,
    PHOTOGRAPHIC_SENSITIVITY, ROTATE_90, THUMBNAIL, camera_exif, camera_file, camera_jpeg,
    camera_jpeg_with_metadata, exif, exif_jpeg, padded_jpeg, plain_jpeg,
};
use support::jpeg_stub::{jpeg_stub, segment};
use support::tiff_builder::{EXIF_IFD, Order};

/// The width of the image an IFD describes (ImageWidth).
const IMAGE_WIDTH: u16 = 0x0100;

/// The height of the image an IFD describes (ImageLength).
const IMAGE_LENGTH: u16 = 0x0101;

/// What the samples of the image an IFD describes mean
/// (PhotometricInterpretation).
const PHOTOMETRIC_INTERPRETATION: u16 = 0x0106;

/// The PhotometricInterpretation of raw data: a colour filter array.
const CFA: u16 = 32803;

/// Where the visible image starts in raw data, column then row
/// (DefaultCropOrigin).
const DEFAULT_CROP_ORIGIN: u16 = 0xC61F;

/// The size of the visible image in raw data (DefaultCropSize).
const DEFAULT_CROP_SIZE: u16 = 0xC620;

/// The width of the picture, as the Exif IFD records it
/// (PixelXDimension).
const PIXEL_X_DIMENSION: u16 = 0xA002;

/// The height of the picture, as the Exif IFD records it
/// (PixelYDimension).
const PIXEL_Y_DIMENSION: u16 = 0xA003;

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
    assert_opens_with(&Options::new(), bytes, previews, metadata, label);
}

/// [`assert_opens`], with the options `options`.
fn assert_opens_with(
    options: &Options,
    bytes: &[u8],
    previews: &[Seen],
    metadata: &Metadata,
    label: &str,
) {
    let from_bytes = options.from_bytes(bytes).unwrap();
    let from_reader = options.from_reader(Cursor::new(bytes.to_vec())).unwrap();
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

/// Where the thumbnail of the Exif segment of `built` lies in the file:
/// `in_tiff`, where it lies in the segment's TIFF structure, from that
/// structure's header.
fn in_file(built: &Built, in_tiff: &Range<u64>) -> Range<u64> {
    let base = built.exif_tiff(0).start;
    base + in_tiff.start..base + in_tiff.end
}

/// The capture time that `CAPTURE_TIME`, in [`camera_exif`]'s TIFF
/// structure, records: 15 June 2024 at 09:30:12,
/// a quarter of a second, two digits, and two hours ahead of UTC.
fn capture_time() -> DateTime {
    DateTime::new(2024, 6, 15, 9, 30, 12)
        .and_then(|time| time.with_subseconds(250_000_000, 2))
        .and_then(|time| time.with_offset_minutes(120))
        .unwrap()
}

/// `(numerator, denominator)` as a fraction of the metadata.
fn fraction((numerator, denominator): (u32, u32)) -> Rational {
    Rational::new(numerator, denominator).unwrap()
}

/// The metadata of a JPEG whose main image is of size [`MAIN`] and whose
/// Exif segment holds [`camera_exif`]'s values, but the values of the
/// tags in `lost`: the image size, no raw size and no focus point, and
/// every value the segment records but those. The capture time goes with
/// its DateTimeOriginal, which no test loses apart from its fraction and
/// its offset.
fn camera_metadata_without(lost: &[u16]) -> Metadata {
    let kept = |tag: u16| !lost.contains(&tag);
    let mut metadata = main_metadata(Orientation::Rotate90);
    if kept(MAKE) {
        metadata = metadata.with_make(CAMERA_MAKE);
    }
    if kept(MODEL) {
        metadata = metadata.with_model(CAMERA_MODEL);
    }
    if kept(BODY_SERIAL_NUMBER) {
        metadata = metadata.with_serial_number(CAMERA_SERIAL_NUMBER);
    }
    if kept(DATE_TIME_ORIGINAL) {
        metadata = metadata.with_capture_time(capture_time());
    }
    let mut exposure = Exposure::default();
    if kept(EXPOSURE_TIME) {
        exposure = exposure.with_exposure_time(fraction(EXPOSURE_SECONDS));
    }
    if kept(F_NUMBER) {
        exposure = exposure.with_f_number(fraction(APERTURE));
    }
    if kept(PHOTOGRAPHIC_SENSITIVITY) {
        exposure = exposure.with_iso(u32::from(ISO));
    }
    if kept(EXPOSURE_BIAS_VALUE) {
        let (numerator, denominator) = EXPOSURE_BIAS;
        exposure =
            exposure.with_exposure_bias(SignedRational::new(numerator, denominator).unwrap());
    }
    if kept(FOCAL_LENGTH) {
        exposure = exposure.with_focal_length(fraction(FOCAL_MILLIMETRES));
    }
    let [lens_make, lens_model, lens_serial_number] = LENS;
    let mut lens = Lens::builder();
    if kept(LENS_MAKE) {
        lens = lens.make(lens_make);
    }
    if kept(LENS_MODEL) {
        lens = lens.model(lens_model);
    }
    if kept(LENS_SERIAL_NUMBER) {
        lens = lens.serial_number(lens_serial_number);
    }
    if kept(LENS_SPECIFICATION) {
        let [shortest, longest, at_shortest, at_longest] = LENS_RANGE.map(fraction);
        lens = lens
            .min_focal_length(shortest)
            .max_focal_length(longest)
            .f_number_at_min_focal_length(at_shortest)
            .f_number_at_max_focal_length(at_longest);
    }
    metadata = metadata.with_exposure(exposure);
    match lens.build() {
        Some(lens) => metadata.with_lens(lens),
        None => metadata,
    }
}

/// The little-endian 16-bit number at `at` in `bytes`.
fn le_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// The little-endian 32-bit number at `at` in `bytes`.
fn le_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// Where the IFD at offset `ifd` of the little-endian TIFF structure
/// `tiff` lies: its count of entries, its entries and its next-IFD
/// pointer. The values stored after it are not part of it.
fn ifd_range(tiff: &[u8], ifd: usize) -> Range<usize> {
    ifd..ifd + 2 + 12 * usize::from(le_u16(tiff, ifd)) + 4
}

/// Where the value field, the last four bytes, of the entry of tag `tag`
/// lies in the IFD at offset `ifd` of the little-endian TIFF structure
/// `tiff`.
fn field_of(tiff: &[u8], ifd: usize, tag: u16) -> Range<usize> {
    let entry = (0..usize::from(le_u16(tiff, ifd)))
        .map(|n| ifd + 2 + 12 * n)
        .find(|&entry| le_u16(tiff, entry) == tag)
        .expect("the tag is in the IFD");
    entry + 8..entry + 12
}

/// Where the ASCII value of the entry of tag `tag` lies in the IFD at
/// offset `ifd` of the little-endian TIFF structure `tiff`, a value longer
/// than its field, at the offset the field holds over the count the entry
/// gives.
fn ascii_of(tiff: &[u8], ifd: usize, tag: u16) -> Range<usize> {
    let field = field_of(tiff, ifd, tag);
    let count = usize::try_from(le_u32(tiff, field.start - 4)).unwrap();
    assert!(count > 4, "the value lies outside its entry");
    let at = usize::try_from(le_u32(tiff, field.start)).unwrap();
    at..at + count
}

/// The offsets of IFD0 and of the Exif IFD in the little-endian TIFF
/// structure `tiff`: as its header and IFD0's Exif IFD entry give them.
fn ifd0_and_exif(tiff: &[u8]) -> (usize, usize) {
    let ifd0 = usize::try_from(le_u32(tiff, 4)).unwrap();
    let exif = usize::try_from(le_u32(tiff, field_of(tiff, ifd0, EXIF_IFD).start)).unwrap();
    (ifd0, exif)
}

/// A file laid out as [`camera_file`]'s, whose Exif segment holds `tiff`,
/// a little-endian TIFF structure, but for the offset in its four bytes at
/// `pointer`, which points at a copy of `tiff[moved]` laid after the main
/// image's EOI: inside the file, outside the segment. The copy is the
/// bytes as they are, so any offset it holds still points inside the
/// segment, and reading it would give what reading the original gives.
/// The file is built once to learn where the copy lands, then with the
/// pointer: the segment's length does not depend on it.
fn moved_outside(tiff: &[u8], moved: Range<usize>, pointer: Range<usize>) -> Built {
    let copy = &tiff[moved];
    let first = camera_file(Order::Little, false, tiff, copy);
    let at = u32::try_from(first.after_eoi(0) - first.exif_tiff(0).start).unwrap();
    let mut changed = tiff.to_vec();
    changed[pointer].copy_from_slice(&at.to_le_bytes());
    let built = camera_file(Order::Little, false, &changed, copy);
    assert_eq!(built.after_eoi(0), first.after_eoi(0));
    assert!(built.after_eoi(0) >= built.exif_tiff(0).end);
    built
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
    let built = plain_jpeg();
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
    let (built, thumbnail) = exif_jpeg(Order::Little);
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
    let (built, thumbnail) = camera_jpeg(Order::Little, false);
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
    let (built, thumbnail) = camera_jpeg(Order::Little, true);
    let previews = camera_jpeg_previews(&built, &thumbnail);
    // The JFIF segment moved everything after it.
    let (without, _) = camera_jpeg(Order::Little, false);
    assert!(built.exif_tiff(0).start > without.exif_tiff(0).start);
    assert_opens(
        &built.bytes,
        &previews,
        &main_metadata(Orientation::Rotate90),
        "JFIF first",
    );
}

/// `fraction` as the numerator and the denominator it stores.
fn stored(fraction: Option<Rational>) -> Option<(u32, u32)> {
    fraction.map(|fraction| (fraction.numerator, fraction.denominator))
}

#[test]
fn a_camera_jpeg_gives_every_value_its_exif_segment_records() {
    let (built, thumbnail) = camera_jpeg_with_metadata(Order::Little);
    let previews = camera_jpeg_previews(&built, &thumbnail);
    assert_opens(
        &built.bytes,
        &previews,
        &camera_metadata_without(&[]),
        "camera JPEG with metadata",
    );

    // Each value, as the tags give it; the sizes, as the frame header
    // gives them; and no focus point, which no standard tag records.
    let file = RawFile::from_bytes(&built.bytes).unwrap();
    let read = file.metadata();
    assert_eq!(read.make(), Some(CAMERA_MAKE));
    assert_eq!(read.model(), Some(CAMERA_MODEL));
    assert_eq!(read.serial_number(), Some(CAMERA_SERIAL_NUMBER));
    assert_eq!(read.capture_time(), Some(capture_time()));
    assert_eq!(
        read.capture_time().map(|time| time.to_string()).as_deref(),
        Some("2024-06-15T09:30:12.25+02:00")
    );
    assert_eq!(read.orientation(), Orientation::Rotate90);
    let exposure = read.exposure();
    assert_eq!(stored(exposure.exposure_time()), Some(EXPOSURE_SECONDS));
    assert_eq!(stored(exposure.f_number()), Some(APERTURE));
    assert_eq!(exposure.iso(), Some(u32::from(ISO)));
    assert_eq!(
        exposure
            .exposure_bias()
            .map(|bias| (bias.numerator, bias.denominator)),
        Some(EXPOSURE_BIAS)
    );
    assert_eq!(stored(exposure.focal_length()), Some(FOCAL_MILLIMETRES));
    let lens = read.lens().expect("a lens");
    assert_eq!(
        [lens.make(), lens.model(), lens.serial_number()],
        LENS.map(Some)
    );
    assert_eq!(
        [
            lens.min_focal_length(),
            lens.max_focal_length(),
            lens.f_number_at_min_focal_length(),
            lens.f_number_at_max_focal_length(),
        ]
        .map(stored),
        LENS_RANGE.map(Some)
    );
    assert_eq!(read.raw_size(), None);
    assert_eq!(read.image_size(), Some(size_of(MAIN)));
    assert_eq!(read.focus(), None);
}

#[test]
fn the_image_size_is_the_pictures_whatever_the_tags_say() {
    // Every size the tags give is another than the picture's: IFD0
    // describes raw data, as a raw file's IFD does, with a size and a
    // default crop inside it, which in a raw file would give the raw size
    // and the image size; and the Exif pixel dimensions, which an editor
    // may leave unchanged, give a third.
    const RAW: [u32; 2] = [6064, 4040];
    const CROP_ORIGIN: [u32; 2] = [8, 4];
    const CROP: [u32; 2] = [6048, 4032];
    const PIXELS: [u32; 2] = [4000, 3000];
    let (tiff, thumbnail) = camera_exif(Order::Little, |tiff, ifd0, exif| {
        tiff.short(ifd0, PHOTOMETRIC_INTERPRETATION, &[CFA])
            .long(ifd0, IMAGE_WIDTH, &[RAW[0]])
            .long(ifd0, IMAGE_LENGTH, &[RAW[1]])
            .long(ifd0, DEFAULT_CROP_ORIGIN, &CROP_ORIGIN)
            .long(ifd0, DEFAULT_CROP_SIZE, &CROP)
            .long(exif, PIXEL_X_DIMENSION, &[PIXELS[0]])
            .long(exif, PIXEL_Y_DIMENSION, &[PIXELS[1]]);
    });
    let built = camera_file(Order::Little, false, &tiff, &[]);
    let previews = camera_jpeg_previews(&built, &thumbnail);
    assert_opens(
        &built.bytes,
        &previews,
        &camera_metadata_without(&[]),
        "sizes in the tags",
    );

    let file = RawFile::from_bytes(&built.bytes).unwrap();
    assert_eq!(file.metadata().image_size(), Some(size_of(MAIN)));
    assert_eq!(file.metadata().raw_size(), None);
}

#[test]
fn the_files_own_preview_runs_through_the_padding_after_its_end() {
    // As some Sony bodies write it: zeros after the last image's EOI.
    let built = padded_jpeg();
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
        let (tiff, _) = exif(Order::Little, ROTATE_90, Some(thumbnail_at));
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
fn a_damaged_exif_segment_costs_the_metadata_and_the_thumbnail_only() {
    let (sound, _) = camera_exif(Order::Little, |_, _, _| {});

    // A TIFF header whose byte-order mark is neither `II` nor `MM`.
    let mut no_header = sound.clone();
    no_header[..2].copy_from_slice(b"XX");
    let no_header = camera_file(Order::Little, false, &no_header, &[]);
    // IFD0 after the main image: inside the file, outside the segment. Its
    // copy there would read as IFD0 does, every value it points at lying
    // inside the segment. The header's offset of IFD0 is its bytes 4 to 8.
    let (ifd0, _) = ifd0_and_exif(&sound);
    let ifd0_outside = moved_outside(&sound, ifd_range(&sound, ifd0), 4..8);

    for (case, built) in [
        ("no TIFF header", no_header),
        ("IFD0 outside", ifd0_outside),
    ] {
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

/// The number of IFDs in the Exif segment of [`camera_jpeg_with_metadata`]:
/// IFD0, the Exif IFD and IFD1, which holds the thumbnail.
const EXIF_IFDS: u32 = 3;

#[test]
fn a_limit_crossed_inside_the_exif_segment_costs_its_metadata_only() {
    let (built, thumbnail) = camera_jpeg_with_metadata(Order::Little);

    // With the default limits, or with exactly the IFDs the segment holds,
    // it gives its thumbnail and values.
    for (options, label) in [
        (Options::new(), "default limits"),
        (
            Options::new().limits(Limits::default().max_ifds(EXIF_IFDS)),
            "max_ifds at the segment's IFDs",
        ),
    ] {
        assert_opens_with(
            &options,
            &built.bytes,
            &camera_jpeg_previews(&built, &thumbnail),
            &camera_metadata_without(&[]),
            label,
        );
    }

    // Walking the segment needs `EXIF_IFDS` IFDs, so one fewer crosses the
    // limit: the segment's values and thumbnail are lost, not the file.
    let options = Options::new().limits(Limits::default().max_ifds(EXIF_IFDS - 1));
    let previews = expected(
        &[
            (built.mpf_image(0), MPF_IMAGES[0]),
            (built.mpf_image(1), MPF_IMAGES[1]),
            (whole(&built), MAIN),
        ],
        Orientation::NoTransforms,
    );
    assert_opens_with(
        &options,
        &built.bytes,
        &previews,
        &main_metadata(Orientation::NoTransforms),
        "max_ifds below the segment's IFDs",
    );
}

#[test]
fn a_value_outside_the_exif_segment_costs_only_itself() {
    let (sound, thumbnail) = camera_exif(Order::Little, |_, _, _| {});
    let (ifd0, exif) = ifd0_and_exif(&sound);
    // The text of IFD0's Model, then of the Exif IFD's LensModel, after the
    // main image: inside the file, outside the segment. Its copy there
    // would read as the text the segment holds.
    for (case, ifd, tag) in [("Model", ifd0, MODEL), ("LensModel", exif, LENS_MODEL)] {
        let built = moved_outside(
            &sound,
            ascii_of(&sound, ifd, tag),
            field_of(&sound, ifd, tag),
        );
        let previews = camera_jpeg_previews(&built, &thumbnail);
        assert_opens(
            &built.bytes,
            &previews,
            &camera_metadata_without(&[tag]),
            case,
        );
    }
}

#[test]
fn an_exif_ifd_outside_the_exif_segment_costs_its_values_only() {
    // The Exif IFD after the main image: inside the file, outside the
    // segment. Its copy there would read as the Exif IFD does, every value
    // it points at lying inside the segment.
    let (sound, thumbnail) = camera_exif(Order::Little, |_, _, _| {});
    let (ifd0, exif) = ifd0_and_exif(&sound);
    let built = moved_outside(
        &sound,
        ifd_range(&sound, exif),
        field_of(&sound, ifd0, EXIF_IFD),
    );
    let previews = camera_jpeg_previews(&built, &thumbnail);
    // IFD0's make, model and orientation, and the thumbnail, remain.
    assert_opens(
        &built.bytes,
        &previews,
        &camera_metadata_without(&EXIF_IFD_TAGS),
        "Exif IFD outside",
    );
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
