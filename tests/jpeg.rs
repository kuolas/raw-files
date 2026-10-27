// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG reader from outside the crate, on JPEG files built in code: what
//! they open as, which previews they list (the file itself, the thumbnail
//! of the Exif segment and the images of the MPF list), what reading one
//! returns, and the metadata it gives: every value the Exif segment
//! records, read as in a raw file, and the image size of the picture,
//! whatever size the tags give. Then the files it refuses, and the damage
//! that costs a file some of its previews or of its metadata but never the
//! file itself. Last, the Nikon MakerNote of a Nikon camera's JPEG, read by
//! IFD0's Make: the values it adds where the Exif segment has none, its
//! preview, and its focus point, placed on the JPEG's pixels on a crop mode
//! and kept only when its frame has the picture's shape; the MakerNote of
//! the headerless layout of the D1 family, with its offsets counted from
//! the Exif segment's TIFF header, its preview and its values; and the
//! damage and the limits that cost the MakerNote but not the file. Then
//! the Sony MakerNote of a Sony camera's JPEG, read by IFD0's Make too,
//! with its offsets counted from the Exif segment's TIFF header: the
//! camera settings it records, its focus point, as recorded and kept only
//! when its frame has the picture's shape, none where it records none, and
//! the damage and the limits that cost the point but not the file. Then three real JPEGs of the
//! maintainer's own Z 9, in FX and in DX and 16:9 crop modes, committed
//! with their images replaced by stubs and their personal fields zeroed (see
//! [`support::fixtures`]), read whole from a path and from bytes: their
//! previews, every value of their metadata, the camera settings among
//! them, and their focus points, as recorded in FX, scaled onto the DX
//! picture and moved into the 16:9 window. Then eight camera JPEGs of
//! Wikimedia Commons, too large to commit, when they have been downloaded
//! (see [`support::samples::path`]): the previews each gives, and its
//! metadata, every value of it, the focus point and the camera settings
//! among them.

mod support;

use std::collections::BTreeSet;
use std::io::Cursor;
use std::ops::Range;

use raw_files::{
    AfAreaMode, CameraSettings, ColorSpace, DateTime, ErrorKind, Exposure, ExposureProgram,
    FineTune, Flash, FlashMode, Focus, FocusOrigin, FocusPoints, Format, ImageArea, Lens, Limits,
    Metadata, MeteringMode, Options, Orientation, Point, Preview, PreviewFormat, Rational, RawFile,
    Shutter, SignedRational, Size, WhiteBalance, WhiteBalanceMode,
};
use support::arw::{
    ARW4_FOCUS_CENTER, ARW4_FOCUS_FRAME, ARW4_FOCUS_SIZE, ARW4_FULL_SIZE_JPEG,
    COLOR_TEMPERATURE as SONY_COLOR_TEMPERATURE,
    ELECTRONIC_FRONT_CURTAIN as SONY_ELECTRONIC_FRONT_CURTAIN, FINE_TUNE as SONY_FINE_TUNE,
    FOCUS_LOCATION as SONY_FOCUS_LOCATION, WHITE_BALANCE as SONY_WHITE_BALANCE,
};
use support::jpeg_builder::{
    Built, JpegBuilder, LONG, MP_ENTRY, MP_ENTRY_AT, MP_ENTRY_LEN, MP_INDEX_AT, MPF_ID,
    MPF_VERSION, NUMBER_OF_IMAGES, UNDEFINED,
};
use support::jpeg_files::{
    A100_MAIN, A100_MODEL, ILCE7M3_FOCUS_LOCATION, ILCE7M3_MAIN, ILCE7M3_MODEL, ILCE7M4_MODEL,
    ILCE6000_MAIN, ILCE6000_MODEL, RX100M4_MAIN, RX100M4_MODEL, SONY_LOST, SONY_MAKE,
    SONY_MPF_IMAGE, SONY_PADDING, SonyJpeg, a100_jpeg, ilce7m3_jpeg, ilce7m3_jpeg_with,
    ilce7m4_jpeg, ilce6000_jpeg, rx100m4_jpeg,
};
use support::jpeg_files::{
    APERTURE, BODY_SERIAL_NUMBER, CAMERA_MAKE, CAMERA_MODEL, CAMERA_SERIAL_NUMBER, CAMERA_SOFTWARE,
    DATE_TIME_ORIGINAL, EXIF_IFD_TAGS, EXPOSURE_BIAS, EXPOSURE_BIAS_VALUE, EXPOSURE_SECONDS,
    EXPOSURE_TIME, F_NUMBER, FOCAL_LENGTH, FOCAL_LENGTH_IN_35MM_FILM, FOCAL_MILLIMETRES,
    FOCAL_MILLIMETRES_IN_35MM_FILM, ISO, LENS, LENS_MAKE, LENS_MODEL, LENS_RANGE,
    LENS_SERIAL_NUMBER, LENS_SPECIFICATION, MAIN, MAKE, MODEL, MPF_IMAGES,
    PHOTOGRAPHIC_SENSITIVITY, ROTATE_90, SOFTWARE, THUMBNAIL, camera_exif, camera_file,
    camera_jpeg, camera_jpeg_with_metadata, exif, exif_jpeg, padded_jpeg, plain_jpeg,
};
use support::jpeg_files::{
    D1H_LOST, D1H_MAIN, D1H_MODEL, D70_ISO, D70_LOST, D70_MAIN, D70_MODEL, D70_PREVIEW, D5300_ISO,
    D5300_LENS, D5300_LOST, D5300_MAIN, D5300_MODEL, D5300_SERIAL_NUMBER, D5300_TIME_ZONE,
    NikonJpeg, Z9Mode, d1h_jpeg, d70_jpeg, d5300_jpeg, nikon_jpeg, z9_jpeg,
};
use support::jpeg_stub::{jpeg_stub, segment};
use support::nef::{
    AF_INFO2, CROP_HI_SPEED, D1H_ISO, D1H_LENS, D1H_PREVIEW_JPEG, D300_LENS, FOCUS_MODE,
    ISO as MAKER_NOTE_ISO, LENS as MAKER_NOTE_LENS, PREVIEW_JPEG, SERIAL_NUMBER, WORLD_TIME,
    nikon_makernote_with, z9_af_info2,
};
use support::samples::{
    Jpeg, Sample, SampleFineTune, SampleFocus, SampleLens, SampleMetadata, SampleSettings,
    SerialShape, check, check_metadata, listed,
};
use support::tiff_builder::{EXIF_IFD, IfdId, Order, TiffBuilder};

/// Both byte orders.
const ORDERS: [Order; 2] = [Order::Little, Order::Big];

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
    if kept(SOFTWARE) {
        metadata = metadata.with_software(CAMERA_SOFTWARE);
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
    if kept(FOCAL_LENGTH_IN_35MM_FILM) {
        exposure =
            exposure.with_focal_length_in_35mm_film(u32::from(FOCAL_MILLIMETRES_IN_35MM_FILM));
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
    // The file writes no settings tag, so the settings are the empty
    // record.
    assert_eq!(read.camera_settings(), &CameraSettings::default());
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

/// The focus centred at `center` in `frame`, with an area of `area`, each
/// width or x first, and focus confirmed.
fn confirmed(frame: (u32, u32), center: (u32, u32), area: (u32, u32)) -> Focus {
    let size = |(width, height)| Size { width, height };
    let (x, y) = center;
    Focus::new(size(frame), Point { x, y })
        .and_then(|focus| focus.with_size(size(area)))
        .expect("a usable focus point")
        .with_in_focus(true)
}

/// The focus point a Z 9's JPEG taken in mode `mode` records, as recorded,
/// in the full frame of 8256 by 5504.
fn recorded(mode: Z9Mode) -> Focus {
    let [frame, center, area] = mode.recorded();
    confirmed(frame, center, area)
}

/// The focus points of a Z 9's JPEG built here whose focus point is
/// `focus`: that point alone, the primary, in the area mode its AFInfo2
/// records, 193, single point.
fn z9_points(focus: Focus) -> FocusPoints {
    FocusPoints::from(focus).with_area_mode(AfAreaMode::Single)
}

/// The previews of the Nikon JPEG `file`, laid out with the MPF index and
/// its two images and a main image of `main`, smallest first: the
/// thumbnail, the two MPF images and the file, all turned by IFD0's
/// Orientation 6.
fn nikon_previews(file: &NikonJpeg, main: (u16, u16)) -> Vec<Seen> {
    let built = &file.built;
    expected(
        &[
            (in_file(built, &file.thumbnail), THUMBNAIL),
            (built.mpf_image(0), MPF_IMAGES[0]),
            (built.mpf_image(1), MPF_IMAGES[1]),
            (whole(built), main),
        ],
        Orientation::Rotate90,
    )
}

/// `metadata` with the image area that the crop setting of `mode`'s
/// CropHiSpeed gives, the one camera setting the Z 9's JPEGs built here
/// record in their MakerNote: 11, FX uncropped, the full frame; 2, DX,
/// APS-C; 17, FX 1:1, the square; and 6, the 16:9 crop.
fn with_image_area(metadata: Metadata, mode: Z9Mode) -> Metadata {
    let area = match mode {
        Z9Mode::Fx => ImageArea::FullFrame,
        Z9Mode::Dx => ImageArea::ApsC,
        Z9Mode::Square => ImageArea::Square,
        Z9Mode::Wide => ImageArea::Wide,
    };
    let settings = metadata.camera_settings().clone().with_image_area(area);
    metadata.with_camera_settings(settings)
}

/// The metadata of a Z 9's JPEG that [`z9_jpeg`] lays out in mode `mode`
/// with a main image of `main`: every value [`camera_exif`] writes, the
/// image area of `mode`, the image size `main`, and the focus point
/// `focus`, as the focus points [`z9_points`] gives.
fn z9_metadata(mode: Z9Mode, main: (u16, u16), focus: Option<Focus>) -> Metadata {
    let metadata =
        with_image_area(camera_metadata_without(&[]), mode).with_image_size(size_of(main));
    match focus {
        Some(focus) => metadata.with_focus_points(z9_points(focus)),
        None => metadata,
    }
}

/// The capture time that [`camera_exif`] records, but without its offset
/// from UTC, which a Nikon JPEG without OffsetTimeOriginal takes from its
/// MakerNote's WorldTime when it records one: `offset`, in minutes east of
/// UTC, or none.
fn capture_time_at(offset: Option<i16>) -> DateTime {
    let time = DateTime::new(2024, 6, 15, 9, 30, 12)
        .and_then(|time| time.with_subseconds(250_000_000, 2))
        .unwrap();
    match offset {
        Some(offset) => time.with_offset_minutes(offset).unwrap(),
        None => time,
    }
}

/// A lens that only its range describes: `range`, the shortest and the
/// longest focal length, then the f-number at each.
fn lens_of_range(range: [(u32, u32); 4]) -> Lens {
    let [shortest, longest, at_shortest, at_longest] = range.map(fraction);
    Lens::builder()
        .min_focal_length(shortest)
        .max_focal_length(longest)
        .f_number_at_min_focal_length(at_shortest)
        .f_number_at_max_focal_length(at_longest)
        .build()
        .unwrap()
}

/// The pairs of byte orders the Nikon JPEGs are built in: the Exif
/// segment's, then the MakerNote's, each in both, and each with the other.
const NIKON_ORDERS: [(Order, Order); 4] = [
    (Order::Little, Order::Little),
    (Order::Big, Order::Big),
    (Order::Little, Order::Big),
    (Order::Big, Order::Little),
];

#[test]
fn a_nikon_jpeg_gives_the_focus_point_its_makernote_records() {
    // A Z 9's JPEG in FX, the full output: CropHiSpeed records no crop, so
    // the point is as recorded, in the frame of 8256 by 5504, which has the
    // picture's shape. Every value the Exif segment records is kept.
    for (order, makernote_order) in NIKON_ORDERS {
        let file = z9_jpeg(order, makernote_order, Z9Mode::Fx, MAIN, false);
        assert_opens(
            &file.built.bytes,
            &nikon_previews(&file, MAIN),
            &z9_metadata(Z9Mode::Fx, MAIN, Some(recorded(Z9Mode::Fx))),
            &format!("Z 9, {order:?}, MakerNote {makernote_order:?}"),
        );
    }
}

#[test]
fn the_exif_values_win_over_the_makernotes() {
    // The Z 9's FX JPEG, whose MakerNote also records a serial number, an
    // ISO, a lens's range and a time zone, all other than the Exif
    // segment's: the Exif values are kept.
    for (order, makernote_order) in NIKON_ORDERS {
        let mut world_time = makernote_order.u16((-300_i16).cast_unsigned()).to_vec();
        world_time.extend_from_slice(&[0, 0]);
        let (block, _) = nikon_makernote_with(makernote_order, [0x02, 0x11], None, |inner, ifd| {
            inner
                .short(ifd, MAKER_NOTE_ISO, &[0, 6400])
                .ascii(ifd, FOCUS_MODE, "AF-S  ")
                .short(ifd, CROP_HI_SPEED, &Z9Mode::Fx.crop_hi_speed())
                .ascii(ifd, SERIAL_NUMBER, "7654321")
                .undefined(ifd, WORLD_TIME, &world_time)
                .rational(ifd, MAKER_NOTE_LENS, &D300_LENS)
                .undefined(ifd, AF_INFO2, &z9_af_info2(makernote_order));
        });
        let file = nikon_jpeg(order, MAIN, true, &[], |_, _, _| {}, &block, block.len());
        assert_opens(
            &file.built.bytes,
            &nikon_previews(&file, MAIN),
            &z9_metadata(Z9Mode::Fx, MAIN, Some(recorded(Z9Mode::Fx))),
            &format!("{order:?}, MakerNote {makernote_order:?}"),
        );
    }
}

#[test]
fn an_older_nikon_jpeg_takes_what_its_exif_lacks_from_its_makernote() {
    // The D5300's JPEG records no serial number, lens range or offset from
    // UTC in its Exif segment: the MakerNote gives them, the offset from
    // its WorldTime's zone. Its AFInfo2 records a zero frame, a shot
    // through the viewfinder: no focus point.
    for order in ORDERS {
        let file = d5300_jpeg(order);
        let metadata = camera_metadata_without(&D5300_LOST)
            .with_model(D5300_MODEL)
            .with_serial_number(D5300_SERIAL_NUMBER)
            .with_capture_time(capture_time_at(Some(D5300_TIME_ZONE)))
            .with_lens(lens_of_range(D5300_LENS))
            .with_image_size(size_of(D5300_MAIN));
        assert_eq!(metadata.exposure().iso(), Some(u32::from(D5300_ISO)));
        assert_opens(
            &file.built.bytes,
            &nikon_previews(&file, D5300_MAIN),
            &metadata,
            &format!("D5300, {order:?}"),
        );
    }
}

/// The metadata of the D70's JPEG that [`d70_jpeg`] lays out: what
/// [`camera_exif`] writes but [`D70_LOST`], its own Model, and from its
/// MakerNote the ISO and the lens's range, which its Exif segment lacks.
/// Its MakerNote records no time zone and no AFInfo2.
fn d70_metadata() -> Metadata {
    let metadata = camera_metadata_without(&D70_LOST);
    let exposure = metadata.exposure().clone().with_iso(u32::from(D70_ISO));
    metadata
        .with_model(D70_MODEL)
        .with_exposure(exposure)
        .with_capture_time(capture_time_at(None))
        .with_lens(lens_of_range(D300_LENS))
        .with_image_size(size_of(D70_MAIN))
}

#[test]
fn the_makernotes_preview_inside_its_declared_length_is_a_preview() {
    // The D70's JPEG, whose MakerNote's byte order is the other from its
    // Exif segment's: its PreviewIFD's JPEG of 564 by 372 lies inside the
    // length the MakerNote declares, and is a preview, turned by IFD0's
    // Orientation like every other.
    for order in ORDERS {
        let (file, preview) = d70_jpeg(order, false);
        let previews = expected(
            &[
                (in_file(&file.built, &file.thumbnail), THUMBNAIL),
                (preview, D70_PREVIEW),
                (whole(&file.built), D70_MAIN),
            ],
            Orientation::Rotate90,
        );
        let label = format!("D70, {order:?}");
        assert_opens(&file.built.bytes, &previews, &d70_metadata(), &label);
    }
}

#[test]
fn a_makernote_preview_outside_the_declared_length_is_not_a_preview() {
    // The D70's JPEG with its MakerNote declared up to where the
    // PreviewIFD's JPEG starts: the JPEG lies inside the Exif segment but
    // outside the MakerNote, and is not a preview. The values the
    // MakerNote holds before it are still read.
    for order in ORDERS {
        let (file, preview) = d70_jpeg(order, true);
        assert!(preview.start >= file.makernote.start, "{order:?}");
        assert!(preview.end <= file.built.exif_tiff(0).end, "{order:?}");
        let previews = expected(
            &[
                (in_file(&file.built, &file.thumbnail), THUMBNAIL),
                (whole(&file.built), D70_MAIN),
            ],
            Orientation::Rotate90,
        );
        let label = format!("D70, preview outside, {order:?}");
        assert_opens(&file.built.bytes, &previews, &d70_metadata(), &label);
    }
}

#[test]
fn an_export_cut_to_another_shape_gives_no_focus_point() {
    // An export of the Z 9's FX shot that keeps the camera's MakerNote,
    // copied from the raw file with its CropArea of 8256 by 5504. The
    // image size is the export's, never CropArea's (a JPEG has no raw size
    // for CropArea to lie inside). The point, recorded in 8256 by 5504, is
    // kept only on an export of that shape: not on one cut to portrait or
    // to a square.
    for order in ORDERS {
        for (main, focus) in [
            ((4128, 2752), Some(recorded(Z9Mode::Fx))),
            ((3648, 5472), None),
            ((1742, 1742), None),
        ] {
            let file = z9_jpeg(order, order, Z9Mode::Fx, main, true);
            assert_opens(
                &file.built.bytes,
                &nikon_previews(&file, main),
                &z9_metadata(Z9Mode::Fx, main, focus),
                &format!("export of {main:?}, {order:?}"),
            );
        }
    }
}

#[test]
fn a_crop_mode_jpeg_places_its_point_on_its_pixels() {
    // The Z 9's crop modes, whose JPEGs hold no CropArea and record their
    // point in the full frame of 8256 by 5504, at each size, as the NEF
    // notes record the maintainer's shots with a single point in the
    // top-left corner of the picture (ADR015):
    //
    // - DX scales the recorded (636, 572), 420 by 468, onto the JPEG: at
    //   full size, the raw file's CropArea, 5392 by 3592, that is the raw
    //   file's (415, 373), 274 by 305; at 4032 by 2688, the recorded point
    //   scaled by `Focus::scaled_to`, as onto CropArea.
    // - 1:1 keeps the height: the window is 5504 by 5504 at every size,
    //   whose origin is (8256 - 5504) / 2 = 1376 across, so the recorded
    //   (1797, 424) is (421, 424), with the recorded area.
    // - 16:9 keeps the width: the window is 8256 by 8256 x 4640 / 8256 =
    //   4640, and 8256 x 3480 / 6192 = 4640, whose origin is (5504 - 4640)
    //   / 2 = 432 down, so the recorded (502, 715) is (502, 283).
    let middle_dx = recorded(Z9Mode::Dx)
        .scaled_to(Size {
            width: 4032,
            height: 2688,
        })
        .unwrap();
    let square = confirmed((5504, 5504), (421, 424), (291, 323));
    let wide = confirmed((8256, 4640), (502, 283), (291, 323));
    for (order, makernote_order) in NIKON_ORDERS {
        for (mode, main, focus) in [
            (
                Z9Mode::Dx,
                (5392, 3592),
                confirmed((5392, 3592), (415, 373), (274, 305)),
            ),
            (Z9Mode::Dx, (4032, 2688), middle_dx),
            (Z9Mode::Square, (5504, 5504), square),
            (Z9Mode::Square, (2752, 2752), square),
            (Z9Mode::Wide, (8256, 4640), wide),
            (Z9Mode::Wide, (6192, 3480), wide),
        ] {
            let file = z9_jpeg(order, makernote_order, mode, main, false);
            assert_opens(
                &file.built.bytes,
                &nikon_previews(&file, main),
                &z9_metadata(mode, main, Some(focus)),
                &format!("{mode:?} at {main:?}, {order:?}, MakerNote {makernote_order:?}"),
            );
        }
    }
}

#[test]
fn a_crop_mode_jpeg_that_cannot_be_placed_gives_no_focus_point() {
    // A crop whose MakerNote holds the raw file's CropArea, as a MakerNote
    // copied into an export does: a JPEG has no raw size to check it
    // against. And a 16:9 crop on a 4:3 JPEG, whose window would be 8256
    // x 3096 / 4128 = 6192 high, more than the frame's 5504.
    for order in ORDERS {
        for (mode, main, crop_area) in [
            (Z9Mode::Dx, (5392, 3592), true),
            (Z9Mode::Square, (5504, 5504), true),
            (Z9Mode::Wide, (8256, 4640), true),
            (Z9Mode::Wide, (4128, 3096), false),
        ] {
            let file = z9_jpeg(order, order, mode, main, crop_area);
            assert_opens(
                &file.built.bytes,
                &nikon_previews(&file, main),
                &z9_metadata(mode, main, None),
                &format!("{mode:?} at {main:?}, CropArea {crop_area}, {order:?}"),
            );
        }
    }
}

/// A Nikon MakerNote in byte order `order` that holds a serial number, the
/// Z 9's FX CropHiSpeed and AFInfo2, and what `tags` adds, with a
/// PreviewIFD that declares a JPEG of 640 by 424 pixels; and where that
/// JPEG starts in the block.
fn makernote_with_everything(
    order: Order,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Vec<u8>, u64) {
    let preview = jpeg_stub(PREVIEW_JPEG.0, PREVIEW_JPEG.1, 0xC0);
    let (block, at) = nikon_makernote_with(order, [0x02, 0x11], Some(&preview), |inner, ifd| {
        inner
            .ascii(ifd, FOCUS_MODE, "AF-S  ")
            .short(ifd, CROP_HI_SPEED, &Z9Mode::Fx.crop_hi_speed())
            .ascii(ifd, SERIAL_NUMBER, "7654321")
            .undefined(ifd, AF_INFO2, &z9_af_info2(order));
        tags(inner, ifd);
    });
    (block, at.unwrap())
}

#[test]
fn a_nikon_makernote_under_another_makers_make_is_not_read() {
    // A file of another make, whose Exif segment records no serial number,
    // with a sound Nikon MakerNote that records one, a focus point, a
    // preview and a crop setting: nothing of it is read. The same file with
    // Nikon's Make reads all four, the crop setting as the image area.
    for order in ORDERS {
        let (block, at) = makernote_with_everything(order, |_, _| {});
        for make in ["Canon", "NIKON CORPORATION"] {
            let file = nikon_jpeg(
                order,
                MAIN,
                true,
                &[MAKE, BODY_SERIAL_NUMBER],
                |tiff, ifd0, _| {
                    tiff.ascii(ifd0, MAKE, make);
                },
                &block,
                block.len(),
            );
            let mut metadata = camera_metadata_without(&[MAKE, BODY_SERIAL_NUMBER]).with_make(make);
            let mut previews = nikon_previews(&file, MAIN);
            if make != "Canon" {
                metadata = with_image_area(metadata, Z9Mode::Fx)
                    .with_serial_number("7654321")
                    .with_focus_points(z9_points(recorded(Z9Mode::Fx)));
                let start = file.makernote.start + at;
                let preview = start..start + to_u64(jpeg_stub(640, 424, 0xC0).len());
                previews.insert(
                    1,
                    (
                        preview,
                        wide(PREVIEW_JPEG),
                        PreviewFormat::Jpeg,
                        Orientation::Rotate90,
                    ),
                );
            }
            assert_opens(
                &file.built.bytes,
                &previews,
                &metadata,
                &format!("{make}, {order:?}"),
            );
        }
    }
}

/// The previews of the D1H's JPEG `file`, whose MakerNote's preview lies
/// at `preview`, smallest first: the thumbnail, the MakerNote's preview
/// when `read`, and the file, all turned by IFD0's Orientation 6.
fn d1h_previews(file: &NikonJpeg, preview: Range<u64>, read: bool) -> Vec<Seen> {
    let mut jpegs = vec![(in_file(&file.built, &file.thumbnail), THUMBNAIL)];
    if read {
        jpegs.push((preview, D1H_PREVIEW_JPEG));
    }
    jpegs.push((whole(&file.built), D1H_MAIN));
    expected(&jpegs, Orientation::Rotate90)
}

/// The metadata of the D1H's JPEG that [`d1h_jpeg`] lays out under the
/// Make `make`: what [`camera_exif`] writes but [`D1H_LOST`], that Make
/// and its own Model, and, when `read`, the ISO and the lens's range that
/// its MakerNote alone records. Its MakerNote records no time zone and no
/// AFInfo2.
fn d1h_metadata(make: &str, read: bool) -> Metadata {
    let metadata = camera_metadata_without(&D1H_LOST)
        .with_make(make)
        .with_model(D1H_MODEL)
        .with_capture_time(capture_time_at(None))
        .with_image_size(size_of(D1H_MAIN));
    if !read {
        return metadata;
    }
    let exposure = metadata.exposure().clone().with_iso(u32::from(D1H_ISO));
    metadata
        .with_exposure(exposure)
        .with_lens(lens_of_range(D1H_LENS))
}

#[test]
fn a_headerless_nikon_makernote_is_read_at_the_exif_segments_tiff_header() {
    // The D1H's layout in a JPEG: the MakerNote IFD at the block's first
    // byte, its offsets counted from the Exif segment's TIFF header, its
    // numbers in that structure's byte order. The PreviewIFD's JPEG, of
    // 570 by 375, is a preview, and the MakerNote gives the ISO and the
    // lens's range the Exif segment lacks.
    for order in ORDERS {
        let (file, preview) = d1h_jpeg(order, CAMERA_MAKE, |_, _| {});
        assert!(file.makernote.start <= preview.start, "{order:?}");
        assert_eq!(file.makernote.end, preview.end, "{order:?}");
        assert_opens(
            &file.built.bytes,
            &d1h_previews(&file, preview, true),
            &d1h_metadata(CAMERA_MAKE, true),
            &format!("D1H, {order:?}"),
        );
    }
}

/// A tag of Nikon's MakerNote that the reader does not read, numbered
/// between FocusMode and SerialNumber, with no tag between it and
/// SerialNumber whose value is stored out of line, so that its value lies
/// right before SerialNumber's in the IFD's values.
const UNREAD_BEFORE_SERIAL: u16 = 0x0010;

#[test]
fn a_headerless_nikon_makernotes_offsets_count_from_the_exif_segments_tiff_header() {
    // The D1H's JPEG, whose MakerNote holds 32 bytes of a tag the reader
    // does not read, just before the value of SerialNumber, which the
    // Exif segment lacks. The Exif segment comes first, so its TIFF header
    // lies 12 bytes into the file, and the offset SerialNumber's entry
    // holds, counted from the file's start, lands 12 bytes early, inside
    // those 32 bytes, where a decoy lies: another serial number. Counted
    // from the Exif segment's TIFF header, the serial number is the
    // camera's; from the file's start it would be the decoy's.
    let (serial, decoy) = (b"7654321\0", b"1111111\0");
    for order in ORDERS {
        let mut unread = vec![0; 32];
        unread[20..28].copy_from_slice(decoy);
        let (file, preview) = d1h_jpeg(order, CAMERA_MAKE, |tiff, ifd| {
            tiff.undefined(ifd, UNREAD_BEFORE_SERIAL, &unread)
                .ascii(ifd, SERIAL_NUMBER, "7654321");
        });
        let bytes = &file.built.bytes;
        let base = file.built.exif_tiff(0).start;
        assert_eq!(base, 12, "{order:?}");
        let offset = u64::from(field_in(bytes, file.makernote.start, order, SERIAL_NUMBER));
        let at = |pos: u64| &bytes[usize::try_from(pos).unwrap()..][..8];
        assert_eq!(at(base + offset), serial, "{order:?}");
        assert_eq!(at(offset), decoy, "{order:?}");
        assert_opens(
            bytes,
            &d1h_previews(&file, preview, true),
            &d1h_metadata(CAMERA_MAKE, true).with_serial_number("7654321"),
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_headerless_nikon_makernote_under_another_makers_make_is_not_read() {
    // The D1H's JPEG under Canon's Make: nothing of its MakerNote is read,
    // neither its preview nor its ISO nor its lens's range. Nikon's Make,
    // in any case, reads all three.
    for order in ORDERS {
        for (make, read) in [
            ("Canon", false),
            ("NIKON CORPORATION", true),
            ("Nikon", true),
        ] {
            let (file, preview) = d1h_jpeg(order, make, |_, _| {});
            assert_opens(
                &file.built.bytes,
                &d1h_previews(&file, preview, read),
                &d1h_metadata(make, read),
                &format!("{make}, {order:?}"),
            );
        }
    }
}

#[test]
fn a_damaged_makernote_costs_only_what_it_gives() {
    // A MakerNote whose inner TIFF header has no byte-order mark: the file
    // opens with every value of its Exif segment and its other previews,
    // and without the MakerNote's serial number, focus point and preview.
    for order in ORDERS {
        let (mut block, _) = makernote_with_everything(order, |_, _| {});
        block[10..12].copy_from_slice(b"XX");
        let file = nikon_jpeg(order, MAIN, true, &[], |_, _, _| {}, &block, block.len());
        assert_opens(
            &file.built.bytes,
            &nikon_previews(&file, MAIN),
            &camera_metadata_without(&[]),
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_limit_crossed_inside_the_makernote_costs_the_makernote_only() {
    // A MakerNote IFD of 20 entries: the version, the PreviewIFD's pointer,
    // the four tags of `makernote_with_everything` and 14 more of no
    // meaning, beside an Exif IFD of 14, the most any other IFD of the file
    // holds. With a limit of 20 entries the MakerNote gives its serial
    // number, focus point, preview and image area; with 19 it gives
    // nothing, and the rest of the file is as without it.
    let fillers = |inner: &mut TiffBuilder, ifd: IfdId| {
        for tag in 0x1000..0x100E {
            inner.undefined(ifd, tag, &[0; 4]);
        }
    };
    for order in ORDERS {
        let (block, at) = makernote_with_everything(order, fillers);
        let file = nikon_jpeg(
            order,
            MAIN,
            true,
            &[BODY_SERIAL_NUMBER],
            |_, _, _| {},
            &block,
            block.len(),
        );
        let start = file.makernote.start + at;
        let preview = start..start + to_u64(jpeg_stub(640, 424, 0xC0).len());
        let mut read = nikon_previews(&file, MAIN);
        read.insert(
            1,
            (
                preview,
                wide(PREVIEW_JPEG),
                PreviewFormat::Jpeg,
                Orientation::Rotate90,
            ),
        );
        let without = camera_metadata_without(&[BODY_SERIAL_NUMBER]);
        let with = with_image_area(without.clone(), Z9Mode::Fx)
            .with_serial_number("7654321")
            .with_focus_points(z9_points(recorded(Z9Mode::Fx)));
        for (entries, previews, metadata) in
            [(20, read, with), (19, nikon_previews(&file, MAIN), without)]
        {
            assert_opens_with(
                &Options::new().limits(Limits::default().max_ifd_entries(entries)),
                &file.built.bytes,
                &previews,
                &metadata,
                &format!("{entries} entries, {order:?}"),
            );
        }
    }
}

/// The focus point a Sony MakerNote's FocusLocation `location` records:
/// the frame's width and height, then the centre's x and y. Sony records
/// no confirmation of focus.
fn sony_focus([width, height, x, y]: [u16; 4]) -> Focus {
    let frame = size_of((width, height));
    Focus::new(
        frame,
        Point {
            x: u32::from(x),
            y: u32::from(y),
        },
    )
    .expect("a usable focus point")
}

/// The previews of the Sony JPEG `file`, laid out with the MPF index and
/// its image and a main image of `main`, smallest first: the thumbnail,
/// the MPF image and the file, padding included, all turned by IFD0's
/// Orientation 6.
fn sony_previews(file: &SonyJpeg, main: (u16, u16)) -> Vec<Seen> {
    let built = &file.built;
    expected(
        &[
            (in_file(built, &file.thumbnail), THUMBNAIL),
            (built.mpf_image(0), SONY_MPF_IMAGE),
            (whole(built), main),
        ],
        Orientation::Rotate90,
    )
}

/// The metadata of a Sony JPEG that `sony_jpeg` lays out with IFD0's
/// Make `make` and Model `model` and a main image of `main`: every value
/// [`camera_exif`] writes but [`SONY_LOST`], that Make and Model, the image
/// size `main`, and the focus point `focus`.
fn sony_metadata(make: &str, model: &str, main: (u16, u16), focus: Option<Focus>) -> Metadata {
    let metadata = camera_metadata_without(&SONY_LOST)
        .with_make(make)
        .with_model(model)
        .with_image_size(size_of(main));
    match focus {
        Some(focus) => metadata.with_focus(focus),
        None => metadata,
    }
}

#[test]
fn a_sony_jpeg_gives_the_focus_point_its_makernote_records() {
    // The ILCE-7M3's JPEG at the camera's M size, whose MakerNote, after
    // the `SONY DSC` header, records its point off centre in a frame of
    // the JPEG's own size, in AF-C: the point as recorded, with no area
    // and no confirmation. Every value the Exif segment records is kept,
    // and the file's own preview runs through the zeros after its end.
    for order in ORDERS {
        let file = ilce7m3_jpeg(order, ILCE7M3_MAIN);
        let bytes = &file.built.bytes;
        assert!(
            bytes.ends_with(&[0; SONY_PADDING]),
            "{order:?}: the file ends with zeros"
        );
        assert_opens(
            bytes,
            &sony_previews(&file, ILCE7M3_MAIN),
            &sony_metadata(
                SONY_MAKE,
                ILCE7M3_MODEL,
                ILCE7M3_MAIN,
                Some(sony_focus(ILCE7M3_FOCUS_LOCATION)),
            ),
            &format!("ILCE-7M3, {order:?}"),
        );
    }
}

#[test]
fn a_sony_jpeg_gives_the_camera_settings_its_makernote_records() {
    // The ILCE-7M3's JPEG, whose MakerNote also records a chosen colour
    // temperature of 3200 K, a shift of three steps towards amber, which
    // Sony stores as 3, and the electronic front curtain on: the settings
    // as the Sony MakerNote reader gives them, the shift positive towards
    // blue, beside the focus point. The Exif segment records no setting.
    for order in ORDERS {
        let file = ilce7m3_jpeg_with(order, SONY_MAKE, ILCE7M3_MAIN, |tiff, ifd| {
            tiff.long(ifd, SONY_WHITE_BALANCE, &[1])
                .slong(ifd, SONY_FINE_TUNE, &[3, 0])
                .long(ifd, SONY_ELECTRONIC_FRONT_CURTAIN, &[1])
                .long(ifd, SONY_COLOR_TEMPERATURE, &[3200]);
        });
        let step = |n| SignedRational::new(n, 1).unwrap();
        let white_balance = WhiteBalance::default()
            .with_mode(WhiteBalanceMode::ColorTemperature)
            .with_color_temperature(3200)
            .with_fine_tune(FineTune::new(step(-3), step(0)));
        let settings = CameraSettings::default()
            .with_white_balance(white_balance)
            .with_electronic_front_curtain_setting(true);
        let metadata = sony_metadata(
            SONY_MAKE,
            ILCE7M3_MODEL,
            ILCE7M3_MAIN,
            Some(sony_focus(ILCE7M3_FOCUS_LOCATION)),
        )
        .with_camera_settings(settings);
        assert_opens(
            &file.built.bytes,
            &sony_previews(&file, ILCE7M3_MAIN),
            &metadata,
            &format!("ILCE-7M3, {order:?}"),
        );
    }
}

#[test]
fn a_sony_jpeg_gives_the_area_its_makernote_records() {
    // A JPEG whose MakerNote holds the ILCE-7M4 raw file's FocusLocation
    // and FocusFrameSize, in the file's byte order: the point with its
    // area of 350 by 351.
    let (frame_width, frame_height) = ARW4_FOCUS_FRAME;
    let (x, y) = ARW4_FOCUS_CENTER;
    let focus = sony_focus([frame_width, frame_height, x, y])
        .with_size(size_of(ARW4_FOCUS_SIZE))
        .expect("an area that fits");
    for order in ORDERS {
        let file = ilce7m4_jpeg(order);
        assert_opens(
            &file.built.bytes,
            &sony_previews(&file, ARW4_FULL_SIZE_JPEG),
            &sony_metadata(SONY_MAKE, ILCE7M4_MODEL, ARW4_FULL_SIZE_JPEG, Some(focus)),
            &format!("ILCE-7M4, {order:?}"),
        );
    }
}

#[test]
fn a_smaller_sony_jpeg_of_the_same_shape_keeps_its_point() {
    // The ILCE-7M3's MakerNote in a JPEG of half its size, 1968 by 1312,
    // as an export scaled from it: the frame of 3936 by 2624 has the
    // picture's shape, and the point is kept as recorded, in that frame.
    let main = (1968, 1312);
    for order in ORDERS {
        let file = ilce7m3_jpeg(order, main);
        assert_opens(
            &file.built.bytes,
            &sony_previews(&file, main),
            &sony_metadata(
                SONY_MAKE,
                ILCE7M3_MODEL,
                main,
                Some(sony_focus(ILCE7M3_FOCUS_LOCATION)),
            ),
            &format!("{main:?}, {order:?}"),
        );
    }
}

#[test]
fn a_sony_export_cut_to_another_shape_gives_no_focus_point() {
    // The ILCE-7M3's MakerNote, kept by an export cut to portrait, or to a
    // square: the frame of 3936 by 2624 has not the picture's shape, and
    // the point is dropped (ADR014).
    for order in ORDERS {
        for main in [(2624, 3936), (2624, 2624)] {
            let file = ilce7m3_jpeg(order, main);
            assert_opens(
                &file.built.bytes,
                &sony_previews(&file, main),
                &sony_metadata(SONY_MAKE, ILCE7M3_MODEL, main, None),
                &format!("{main:?}, {order:?}"),
            );
        }
    }
}

#[test]
fn a_sony_jpeg_in_manual_focus_gives_no_focus_point() {
    // The DSC-RX100M4's JPEG, whose FocusMode is the one BYTE 0, manual
    // focus, and whose FocusLocation is the frame's centre: no point.
    for order in ORDERS {
        let file = rx100m4_jpeg(order);
        assert_opens(
            &file.built.bytes,
            &sony_previews(&file, RX100M4_MAIN),
            &sony_metadata(SONY_MAKE, RX100M4_MODEL, RX100M4_MAIN, None),
            &format!("RX100M4, {order:?}"),
        );
    }
}

#[test]
fn a_sony_jpeg_without_a_focus_location_gives_no_focus_point() {
    // The ILCE-6000's JPEG, whose MakerNote holds a FocusMode of AF-A and
    // no FocusLocation, as the bodies before the ILCE-7RM2 write it.
    for order in ORDERS {
        let file = ilce6000_jpeg(order);
        assert_opens(
            &file.built.bytes,
            &sony_previews(&file, ILCE6000_MAIN),
            &sony_metadata(SONY_MAKE, ILCE6000_MODEL, ILCE6000_MAIN, None),
            &format!("ILCE-6000, {order:?}"),
        );
    }
}

#[test]
fn an_a100_jpeg_opens_with_its_standard_metadata_and_no_focus_point() {
    // The DSLR-A100's JPEG, with no MPF index, whose MakerNote, after the
    // same header, holds tags of an older layout and no FocusLocation:
    // the standard metadata, the thumbnail and the file, and no point.
    for order in ORDERS {
        let file = a100_jpeg(order);
        let previews = expected(
            &[
                (in_file(&file.built, &file.thumbnail), THUMBNAIL),
                (whole(&file.built), A100_MAIN),
            ],
            Orientation::Rotate90,
        );
        assert_opens(
            &file.built.bytes,
            &previews,
            &sony_metadata(SONY_MAKE, A100_MODEL, A100_MAIN, None),
            &format!("A100, {order:?}"),
        );
    }
}

/// The 16-bit number at `at` in `bytes`, in byte order `order`.
fn u16_in(bytes: &[u8], at: u64, order: Order) -> u16 {
    let at = usize::try_from(at).unwrap();
    let pair = [bytes[at], bytes[at + 1]];
    match order {
        Order::Little => u16::from_le_bytes(pair),
        Order::Big => u16::from_be_bytes(pair),
    }
}

/// The 32-bit number at `at` in `bytes`, in byte order `order`.
fn u32_in(bytes: &[u8], at: u64, order: Order) -> u32 {
    let (high, low) = (u16_in(bytes, at, order), u16_in(bytes, at + 2, order));
    match order {
        Order::Little => u32::from(low) << 16 | u32::from(high),
        Order::Big => u32::from(high) << 16 | u32::from(low),
    }
}

/// The value field of the entry of tag `tag` in the IFD that lies at `ifd`
/// in `bytes`, in byte order `order`: for a value stored out of line, its
/// offset.
fn field_in(bytes: &[u8], ifd: u64, order: Order, tag: u16) -> u32 {
    let entry = (0..u64::from(u16_in(bytes, ifd, order)))
        .map(|n| ifd + 2 + 12 * n)
        .find(|&entry| u16_in(bytes, entry, order) == tag)
        .expect("the tag is in the IFD");
    u32_in(bytes, entry + 8, order)
}

/// The four SHORTs at `at` in `bytes`, in byte order `order`.
fn shorts_in(bytes: &[u8], at: u64, order: Order) -> [u16; 4] {
    [0, 2, 4, 6].map(|n| u16_in(bytes, at + n, order))
}

/// A tag of Sony's MakerNote that the JPEG reader does not read, numbered
/// below FocusLocation, so that its value lies right before FocusLocation's
/// in the IFD's values.
const UNREAD_BEFORE_FOCUS: u16 = 0x0010;

#[test]
fn a_sony_makernotes_offsets_count_from_the_exif_segments_tiff_header() {
    // The ILCE-7M3's JPEG, whose MakerNote holds 32 bytes of a tag the
    // reader does not read, just before FocusLocation's value. The Exif
    // segment comes first, so its TIFF header lies 12 bytes into the file,
    // and the offset FocusLocation's entry holds, counted from the file's
    // start, lands 12 bytes early, inside those 32 bytes, where a decoy
    // lies: a FocusLocation of the same frame with another centre. Counted
    // from the Exif segment's TIFF header, the point is the camera's; from
    // the file's start it would be the decoy's, and from the MakerNote's
    // own start, as Nikon's count, past the MakerNote, and none.
    let decoy = [3936, 2624, 100, 200];
    for order in ORDERS {
        let mut unread = vec![0; 32];
        unread[20..28].copy_from_slice(&decoy.map(|n| order.u16(n)).concat());
        let file = ilce7m3_jpeg_with(order, SONY_MAKE, ILCE7M3_MAIN, |tiff, ifd| {
            tiff.undefined(ifd, UNREAD_BEFORE_FOCUS, &unread);
        });
        let bytes = &file.built.bytes;
        let base = file.built.exif_tiff(0).start;
        assert_eq!(base, 12, "{order:?}");
        let offset = u64::from(field_in(
            bytes,
            file.makernote_ifd,
            order,
            SONY_FOCUS_LOCATION,
        ));
        assert_eq!(
            shorts_in(bytes, base + offset, order),
            ILCE7M3_FOCUS_LOCATION,
            "{order:?}"
        );
        assert_eq!(shorts_in(bytes, offset, order), decoy, "{order:?}");
        assert_opens(
            bytes,
            &sony_previews(&file, ILCE7M3_MAIN),
            &sony_metadata(
                SONY_MAKE,
                ILCE7M3_MODEL,
                ILCE7M3_MAIN,
                Some(sony_focus(ILCE7M3_FOCUS_LOCATION)),
            ),
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_sony_makernote_under_another_makers_make_is_not_read() {
    // The ILCE-7M3's JPEG under other Makes: another maker's, Hasselblad's,
    // whose Sony-built bodies the ARW claim tells apart by a raw file's
    // structure, which a JPEG lacks, and Nikon's, whose reader takes
    // Sony's block for a bare IFD, of Nikon's headerless layout, and finds
    // in its first two bytes, `SO`, a count of thousands of entries. None
    // gives the point. Sony's Make in any case reads it.
    for order in ORDERS {
        for (make, read) in [
            ("Canon", false),
            ("HASSELBLAD", false),
            ("NIKON CORPORATION", false),
            ("SONY", true),
            ("sony", true),
        ] {
            let file = ilce7m3_jpeg_with(order, make, ILCE7M3_MAIN, |_, _| {});
            let focus = read.then(|| sony_focus(ILCE7M3_FOCUS_LOCATION));
            assert_opens(
                &file.built.bytes,
                &sony_previews(&file, ILCE7M3_MAIN),
                &sony_metadata(make, ILCE7M3_MODEL, ILCE7M3_MAIN, focus),
                &format!("{make}, {order:?}"),
            );
        }
    }
}

#[test]
fn a_damaged_sony_makernote_costs_only_its_focus_point() {
    // The ILCE-7M3's JPEG whose MakerNote IFD counts 200 entries, whose
    // table would run far past the MakerNote's end: the file opens with
    // every value of its Exif segment and every preview, and no point.
    for order in ORDERS {
        let mut file = ilce7m3_jpeg(order, ILCE7M3_MAIN);
        let at = usize::try_from(file.makernote_ifd).unwrap();
        assert_eq!(u16_in(&file.built.bytes, file.makernote_ifd, order), 2);
        file.built.bytes[at..at + 2].copy_from_slice(&order.u16(200));
        assert_opens(
            &file.built.bytes,
            &sony_previews(&file, ILCE7M3_MAIN),
            &sony_metadata(SONY_MAKE, ILCE7M3_MODEL, ILCE7M3_MAIN, None),
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_limit_crossed_inside_the_sony_makernote_costs_its_focus_point_only() {
    // A MakerNote IFD of 12 entries: FocusMode, FocusLocation and 10 more
    // of no meaning, beside an Exif IFD of 11, the most any other IFD of
    // the file holds. With a limit of 12 entries the MakerNote gives its
    // point; with 11 it gives none, and the rest of the file is as without
    // it.
    for order in ORDERS {
        let file = ilce7m3_jpeg_with(order, SONY_MAKE, ILCE7M3_MAIN, |tiff, ifd| {
            for tag in 0x1000..0x100A {
                tiff.undefined(ifd, tag, &[0; 4]);
            }
        });
        assert_eq!(u16_in(&file.built.bytes, file.makernote_ifd, order), 12);
        for (entries, focus) in [(12, Some(sony_focus(ILCE7M3_FOCUS_LOCATION))), (11, None)] {
            assert_opens_with(
                &Options::new().limits(Limits::default().max_ifd_entries(entries)),
                &file.built.bytes,
                &sony_previews(&file, ILCE7M3_MAIN),
                &sony_metadata(SONY_MAKE, ILCE7M3_MODEL, ILCE7M3_MAIN, focus),
                &format!("{entries} entries, {order:?}"),
            );
        }
    }
}

// Three real files, committed. Three JPEGs of the maintainer's own Z 9,
// shot in FX, in DX crop mode and in 16:9 crop mode, with every image
// replaced by a stub JPEG of its size and their personal fields zeroed in
// place, their Exif segment otherwise as the camera wrote it
// (`tests/fixtures/README.md`). They hold what the files built in code only
// imitate: the camera's own Exif segment, every tag at the camera's
// offset, with its MakerNote, IFD1's thumbnail, and the MPF list of the
// images after the main one.
//
// The values below were read from each file's bytes by a walk of its
// structure that shares no code with the reader, with the rules of ADR011,
// ADR014 and ADR015 applied to what it read. Each focus point is, to the
// pixel, the point the reader gives on the raw file of the same shot,
// scaled onto the same frame.

/// What one of the three stripped Z 9 JPEGs must give.
struct StrippedZ9Jpeg {
    /// The file's name in `tests/fixtures/`.
    file: &'static str,
    /// IFD0's Make, Model and Software.
    make: &'static str,
    model: &'static str,
    software: &'static str,
    /// The capture time, as [`DateTime`] displays it: DateTimeOriginal,
    /// SubSecTimeOriginal and OffsetTimeOriginal.
    capture_time: &'static str,
    /// The exposure, each value as stored.
    exposure_time: (u32, u32),
    f_number: (u32, u32),
    iso: u32,
    focal_length: (u32, u32),
    focal_length_in_35mm_film: u32,
    exposure_bias: (i32, i32),
    /// The lens from the Exif IFD: its make, its model, and its range, the
    /// shortest and the longest focal length, then the f-number at each.
    lens_make: Option<&'static str>,
    lens_model: &'static str,
    lens_range: [(u32, u32); 4],
    /// IFD0's Orientation, which turns every preview.
    orientation: Orientation,
    /// The four previews, smallest first, each its byte range and size:
    /// IFD1's thumbnail, the length IFD1 declares for it, the two images
    /// of the MPF list, and the file itself, of its frame header's size.
    previews: [(Range<u64>, (u16, u16)); 4],
    /// The frame, the centre and the area of the focus point the reader
    /// gives, and whether the camera confirmed focus.
    focus_frame: (u32, u32),
    focus_center: (u32, u32),
    focus_size: (u32, u32),
    in_focus: bool,
    /// The area mode the reader names from AFInfo2's byte 5, under the
    /// detection method at byte 4, `None` for a code it does not name.
    area_mode: Option<AfAreaMode>,
    /// The colour temperature of the white balance the camera applied, in
    /// kelvin: the MakerNote's ColorTemperatureAuto.
    color_temperature: u32,
    /// The image area, from the crop setting of the MakerNote's
    /// CropHiSpeed.
    image_area: ImageArea,
}

/// The FX shot, the camera's full output, at full size: CropHiSpeed
/// `[11, 8280, 5520, 8280, 5520, 0, 0]` records no crop, so the point is as
/// AFInfo2 records it, the centre (4225, 2614) and the area 406 by 406 in
/// the frame of 8256 by 5504, which is the JPEG's own size.
const STRIPPED_Z9_FX_JPEG: StrippedZ9Jpeg = StrippedZ9Jpeg {
    file: support::fixtures::Z9_FX_JPEG,
    make: "NIKON CORPORATION",
    model: "NIKON Z 9",
    software: "Ver.05.30",
    capture_time: "2026-10-22T09:26:56.75-03:00",
    exposure_time: (10, 2500),
    f_number: (800, 100),
    iso: 64,
    focal_length: (700, 10),
    focal_length_in_35mm_film: 70,
    exposure_bias: (0, 6),
    lens_make: None,
    lens_model: "VR 70-300mm f/4.5-5.6E",
    lens_range: [(700, 10), (3000, 10), (450, 100), (560, 100)],
    orientation: Orientation::NoTransforms,
    previews: [
        (39876..52662, (160, 120)),
        (69754..69876, (640, 424)),
        (69876..69998, (1620, 1080)),
        (0..69998, (8256, 5504)),
    ],
    focus_frame: (8256, 5504),
    focus_center: (4225, 2614),
    focus_size: (406, 406),
    in_focus: true,
    // Code 207, 3D-tracking.
    area_mode: Some(AfAreaMode::Tracking),
    color_temperature: 5670,
    image_area: ImageArea::FullFrame,
};

/// The DX shot, at full size: CropHiSpeed `[2, 8280, 5520, 5408, 3608,
/// 1436, 956]` records a crop that cuts both sides, so the centre (1024,
/// 718) and the area 226 by 258 that AFInfo2 records in the full 8256 by
/// 5504 frame are fractions of the picture, and the reader scales them
/// onto the JPEG's 5392 by 3592, each rounded to the nearest pixel.
const STRIPPED_Z9_DX_JPEG: StrippedZ9Jpeg = StrippedZ9Jpeg {
    file: support::fixtures::Z9_DX_JPEG,
    make: "NIKON CORPORATION",
    model: "NIKON Z 9",
    software: "Ver.05.30",
    capture_time: "2026-10-22T09:40:59.19-03:00",
    exposure_time: (10, 2000),
    f_number: (800, 100),
    iso: 64,
    focal_length: (780, 10),
    focal_length_in_35mm_film: 117,
    exposure_bias: (0, 6),
    lens_make: None,
    lens_model: "VR 70-300mm f/4.5-5.6E",
    lens_range: [(700, 10), (3000, 10), (450, 100), (560, 100)],
    orientation: Orientation::NoTransforms,
    previews: [
        (39876..50557, (160, 120)),
        (69754..69876, (640, 424)),
        (69876..69998, (1620, 1080)),
        (0..69998, (5392, 3592)),
    ],
    focus_frame: (5392, 3592),
    focus_center: (669, 469),
    focus_size: (148, 168),
    in_focus: true,
    // Code 192, which no file shot in a known mode names.
    area_mode: None,
    color_temperature: 5300,
    image_area: ImageArea::ApsC,
};

/// The 16:9 shot, at the middle size, 6192 by 3480, taken in portrait
/// orientation and stored on its side with Orientation 8, which turns
/// every preview. CropHiSpeed `[6, 8280, 5520, 8280, 4656, 0, 432]`
/// records a crop that keeps the width, so the centre (878, 4504) that
/// AFInfo2 records in the full 8256 by 5504 frame is a position in that
/// frame. The window of the JPEG's shape in it is 8256 wide and 8256 x
/// 3480 / 6192 = 4640 high, whose origin is (5504 - 4640) / 2 = 432 down:
/// the reader moves y back to 4072 and keeps the area, 162 by 178.
const STRIPPED_Z9_16X9_JPEG: StrippedZ9Jpeg = StrippedZ9Jpeg {
    file: support::fixtures::Z9_16X9_JPEG,
    make: "NIKON CORPORATION",
    model: "NIKON Z 9",
    software: "Ver.05.30",
    capture_time: "2026-10-22T19:17:01.01-03:00",
    exposure_time: (10, 200),
    f_number: (450, 100),
    iso: 1400,
    focal_length: (700, 10),
    focal_length_in_35mm_film: 70,
    exposure_bias: (0, 6),
    lens_make: None,
    lens_model: "VR 70-300mm f/4.5-5.6E",
    lens_range: [(700, 10), (3000, 10), (450, 100), (560, 100)],
    orientation: Orientation::Rotate270,
    previews: [
        (39876..51224, (160, 120)),
        (69754..69876, (640, 360)),
        (69876..69998, (1920, 1080)),
        (0..69998, (6192, 3480)),
    ],
    focus_frame: (8256, 4640),
    focus_center: (878, 4072),
    focus_size: (162, 178),
    in_focus: true,
    // Code 192, which no file shot in a known mode names.
    area_mode: None,
    color_temperature: 6130,
    image_area: ImageArea::Wide,
};

/// Checks that the stripped Z 9 JPEG `stripped` names, read whole from its
/// path and from its bytes, opens as a JPEG with its four previews,
/// smallest first, each read whole, and gives every value of its metadata,
/// the focus point among them.
fn check_stripped_z9_jpeg(stripped: &StrippedZ9Jpeg) {
    let label = stripped.file;
    let bytes = support::fixtures::read_fixture(stripped.file);
    let mut file = RawFile::open(support::fixtures::fixture(stripped.file)).unwrap();
    let from_bytes = RawFile::from_bytes(&bytes).unwrap();
    for (how, format, previews) in [
        ("path", file.format(), file.previews()),
        ("bytes", from_bytes.format(), from_bytes.previews()),
    ] {
        assert_eq!(format, Format::Jpeg, "{label}, from {how}");
        let found: Vec<Seen> = previews.iter().map(seen).collect();
        assert_eq!(
            found,
            expected(&stripped.previews, stripped.orientation),
            "{label}, from {how}"
        );
    }
    assert_eq!(file.metadata(), from_bytes.metadata(), "{label}");

    // The file itself is the largest preview and reads as the whole file.
    // Every other preview is a stub JPEG of its size: an MPF image exactly
    // that, and the thumbnail the stub, then zeros to the length IFD1
    // declares, which the stripping kept so that no offset moved.
    let previews = file.previews().to_vec();
    let largest = file.largest_preview().unwrap();
    assert_eq!(Some(largest), previews.last().copied(), "{label}");
    assert_eq!(largest.byte_range(), 0..to_u64(bytes.len()), "{label}");
    for preview in &previews {
        let size = (preview.width(), preview.height());
        let read = file.read_preview(preview).unwrap();
        if *preview == largest {
            assert_eq!(read, bytes, "{label} {size:?}");
            continue;
        }
        let stub = jpeg_stub(
            u16::try_from(size.0).unwrap(),
            u16::try_from(size.1).unwrap(),
            0xC0,
        );
        assert_eq!(read[..stub.len()], stub[..], "{label} {size:?}");
        assert!(
            read[stub.len()..].iter().all(|&byte| byte == 0),
            "{label} {size:?}"
        );
    }

    let metadata = file.metadata();
    assert_eq!(metadata.make(), Some(stripped.make), "{label}");
    assert_eq!(metadata.model(), Some(stripped.model), "{label}");
    assert_eq!(metadata.software(), Some(stripped.software), "{label}");
    // The stripping zeroed BodySerialNumber and the MakerNote's
    // SerialNumber in place, so neither gives a serial number.
    assert_eq!(metadata.serial_number(), None, "{label}");
    assert_eq!(
        metadata
            .capture_time()
            .map(|time| time.to_string())
            .as_deref(),
        Some(stripped.capture_time),
        "{label}"
    );
    assert_eq!(metadata.orientation(), stripped.orientation, "{label}");
    let (numerator, denominator) = stripped.exposure_bias;
    assert_eq!(
        metadata.exposure(),
        &Exposure::default()
            .with_exposure_time(fraction(stripped.exposure_time))
            .with_f_number(fraction(stripped.f_number))
            .with_iso(stripped.iso)
            .with_focal_length(fraction(stripped.focal_length))
            .with_focal_length_in_35mm_film(stripped.focal_length_in_35mm_film)
            .with_exposure_bias(SignedRational::new(numerator, denominator).unwrap()),
        "{label}"
    );
    // The lens from the Exif IFD: a LensMake that is empty gives no make,
    // and the LensSerialNumber, empty and then zeroed, no serial number.
    let [shortest, longest, at_shortest, at_longest] = stripped.lens_range.map(fraction);
    let mut lens = Lens::builder()
        .model(stripped.lens_model)
        .min_focal_length(shortest)
        .max_focal_length(longest)
        .f_number_at_min_focal_length(at_shortest)
        .f_number_at_max_focal_length(at_longest);
    if let Some(make) = stripped.lens_make {
        lens = lens.make(make);
    }
    assert_eq!(metadata.lens(), Some(&lens.build().unwrap()), "{label}");
    // A JPEG holds no raw data, and its image size is its frame header's.
    assert_eq!(metadata.raw_size(), None, "{label}");
    assert_eq!(
        metadata.image_size(),
        Some(size_of(stripped.previews[3].1)),
        "{label}"
    );
    let focus = confirmed(
        stripped.focus_frame,
        stripped.focus_center,
        stripped.focus_size,
    )
    .with_in_focus(stripped.in_focus);
    assert_eq!(metadata.focus(), Some(focus), "{label}");
    // The focus points: that point alone, the primary, with no number,
    // recorded, in the area mode the file's AFInfo2 records.
    let points = metadata
        .focus_points()
        .unwrap_or_else(|| panic!("{label}: no focus points"));
    let mut expected = FocusPoints::from(focus);
    if let Some(mode) = stripped.area_mode {
        expected = expected.with_area_mode(mode);
    }
    assert_eq!(points, &expected, "{label}");
    assert_eq!(points.points().len(), 1, "{label}");
    assert_eq!(
        points.primary().and_then(|primary| primary.number()),
        None,
        "{label}"
    );
    assert_eq!(points.origin(), FocusOrigin::Recorded, "{label}");
    assert_eq!(points.area_mode(), stripped.area_mode, "{label}");
    // The settings, the same on the three shots but the colour
    // temperature and the image area. From the Exif IFD: aperture
    // priority, matrix metering, a Flash of 0, no flash fired and no mode
    // recorded, and the ColorSpace 1, sRGB, which keeps the MakerNote's
    // out. From the MakerNote: auto white balance, its text `AUTO0`, which
    // Exif's auto agrees with; the colour temperature it chose; a recorded
    // zero fine-tuning, `0/4, 0/4`; the electronic shutter; and the image
    // area of the crop. A JPEG has no raw image size, whatever its
    // MakerNote's ImageSizeRAW says.
    let zero = SignedRational {
        numerator: 0,
        denominator: 4,
    };
    assert_eq!(
        metadata.camera_settings(),
        &CameraSettings::default()
            .with_white_balance(
                WhiteBalance::default()
                    .with_mode(WhiteBalanceMode::Auto)
                    .with_color_temperature(stripped.color_temperature)
                    .with_fine_tune(FineTune::new(zero, zero)),
            )
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(Flash::new(false))
            .with_shutter(Shutter::Electronic)
            .with_image_area(stripped.image_area)
            .with_color_space(ColorSpace::Srgb),
        "{label}"
    );
}

#[test]
fn the_stripped_z9_fx_jpeg_gives_its_previews_metadata_and_recorded_focus_point() {
    check_stripped_z9_jpeg(&STRIPPED_Z9_FX_JPEG);
}

#[test]
fn the_stripped_z9_dx_jpeg_gives_its_previews_metadata_and_scaled_focus_point() {
    check_stripped_z9_jpeg(&STRIPPED_Z9_DX_JPEG);
}

#[test]
fn the_stripped_z9_16x9_jpeg_gives_its_previews_metadata_and_moved_focus_point() {
    check_stripped_z9_jpeg(&STRIPPED_Z9_16X9_JPEG);
}

// Real files, downloaded: eight camera-original JPEGs of Wikimedia
// Commons, of five Sony bodies, two Nikon bodies and a Canon, under CC0, a
// public-domain dedication or CC BY. They are listed with their licences,
// their authors and their hashes in `tests/samples.tsv`;
// `scripts/fetch-samples.sh` downloads them. They are too large to commit,
// so a test whose file is absent says so and passes: the suite runs
// offline.
//
// The tables below were generated from each file's bytes, by a walk of its
// segments, its Exif segment's TIFF structure, its MakerNote and its MPF
// list that shares no code with the reader, with the rules of ADR011,
// ADR014 and ADR015 applied to what it read; the software and the focal
// length in 35 mm terms too, from IFD0 and the Exif IFD, and the camera
// settings, from the Exif IFD and the Interoperability IFD.

/// The JPEG samples, each with the previews the reader finds in it,
/// smallest first: IFD1's thumbnail, of 160 by 120 on every file but the
/// D70's, of 128 by 85 in an IFD1 that is also the file's Interoperability
/// IFD; the images of the MPF list after the first, one on the ILCE-7M3,
/// ILCE-7RM2, DSC-RX100M4, ILCE-6000 and G7 X and two on the D5300; the
/// D70's MakerNote preview, of 564 by 372, inside the length the MakerNote
/// declares; and the file itself, of its frame header's size, whose bytes
/// run on past the main image, through the MPF images and, on the
/// ILCE-7M3, the ILCE-7RM2 and the ILCE-6000, through the zeros after the
/// last image. Every file has orientation 1.
const SAMPLES: &[Sample] = &[
    // ILCE-7M3 (Wikimedia Commons Acanthodactylus_erythrurus_2025.jpg)
    Sample {
        file: "sony-ilce-7m3.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 39_172,
                len: 7_574,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 7_469_056,
                len: 825_078,
                after_eoi: 0,
            },
            Jpeg {
                width: 3936,
                height: 2624,
                offset: 0,
                len: 8_323_072,
                after_eoi: 28_938,
            },
        ],
    },
    // ILCE-7RM2 (Wikimedia Commons Ulpu_Pajari_puhumassa_vastaanotettuaan_tiedonjulkistamisen_valtionpalkinnon_2.jpg)
    Sample {
        file: "sony-ilce-7rm2.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_778,
                len: 9_806,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 13_115_392,
                len: 615_548,
                after_eoi: 0,
            },
            Jpeg {
                width: 7952,
                height: 5304,
                offset: 0,
                len: 13_762_560,
                after_eoi: 31_620,
            },
        ],
    },
    // DSC-RX100M4 (Wikimedia Commons 여의도한강공원샛강생태공원문화다리_09.jpg)
    Sample {
        file: "sony-dsc-rx100m4.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 34_646,
                len: 10_107,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 7_483_392,
                len: 1_005_576,
                after_eoi: 0,
            },
            Jpeg {
                width: 5472,
                height: 3648,
                offset: 0,
                len: 8_488_968,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-6000 (Wikimedia Commons 140319_MS_Queen_Elizabeth_Kobe_Japan01o.JPG)
    Sample {
        file: "sony-ilce-6000.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_466,
                len: 5_872,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 7_220_736,
                len: 549_648,
                after_eoi: 0,
            },
            Jpeg {
                width: 6000,
                height: 4000,
                offset: 0,
                len: 7_798_784,
                after_eoi: 28_400,
            },
        ],
    },
    // DSLR-A100 (Wikimedia Commons "Krone"_flag_on_line_5_tram_(2564030031).jpg)
    Sample {
        file: "sony-dslr-a100.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 40_306,
                len: 4_129,
                after_eoi: 0,
            },
            Jpeg {
                width: 3872,
                height: 2592,
                offset: 0,
                len: 4_002_446,
                after_eoi: 0,
            },
        ],
    },
    // NIKON D5300 (Wikimedia Commons "Tours"_French_restaurant_in_Omotesando.JPG)
    Sample {
        file: "nikon-d5300.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 15_872,
                len: 7_745,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 7_301_632,
                len: 35_624,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 7_337_472,
                len: 485_513,
                after_eoi: 0,
            },
            Jpeg {
                width: 6000,
                height: 4000,
                offset: 0,
                len: 7_822_985,
                after_eoi: 0,
            },
        ],
    },
    // NIKON D70 (Wikimedia Commons "Stadswandeling".JPG)
    Sample {
        file: "nikon-d70.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 128,
                height: 85,
                offset: 29_228,
                len: 4_554,
                after_eoi: 0,
            },
            Jpeg {
                width: 564,
                height: 372,
                offset: 2_330,
                len: 26_731,
                after_eoi: 0,
            },
            Jpeg {
                width: 3008,
                height: 2000,
                offset: 0,
                len: 4_178_961,
                after_eoi: 0,
            },
        ],
    },
    // Canon PowerShot G7 X (Wikimedia Commons DC-3_OH-LCH_at_EFJM_20160717.jpg)
    Sample {
        file: "canon-powershot-g7x.jpg",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 19_456,
                len: 5_325,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 8_156_672,
                len: 510_383,
                after_eoi: 0,
            },
            Jpeg {
                width: 5472,
                height: 3648,
                offset: 0,
                len: 8_667_055,
                after_eoi: 0,
            },
        ],
    },
];

#[test]
fn every_jpeg_sample_opens_with_its_previews() {
    for sample in SAMPLES {
        check(sample, Format::Jpeg);
    }
}

/// The metadata each JPEG sample must give, in the order of
/// `tests/samples.tsv`, its serial numbers by their shape alone (see
/// [`SampleMetadata`]): the standard tags first (ADR011), then the Nikon
/// MakerNote, read by IFD0's Make, for what they lack, the D5300's serial
/// number, lens range and offset from UTC and the D70's ISO and lens
/// range; no raw size, and the frame header's size as the image size
/// (ADR014). The focus point is the one Sony's MakerNote records, as
/// recorded, on the ILCE-7M3, off centre in a frame of its picture's size,
/// and on the ILCE-7RM2, at the centre; where there is none, a comment
/// says why. The camera settings are those the Exif IFD records, with,
/// on the Nikon and Sony files, those their MakerNote records: every file
/// gives sRGB, from its Exif ColorSpace; the G7 X records no
/// ExposureProgram and the D5300 the ExposureProgram 0, "not defined", so
/// neither gives an exposure program; the D5300 gives the fine-tuning
/// `0, 0` of its 0x000B, while the D70's 0x000B holds one value, which
/// gives none; and neither Nikon file gives a raw image size, since a JPEG
/// holds no raw data. The Sony files but the DSLR-A100's give the
/// fine-tuning `0, 0` and the electronic front curtain setting their
/// MakerNote records, off on the RX100M4 and on on the others, and the
/// white balance mode of its WhiteBalance, shade on the RX100M4, as Exif
/// says too, and auto on the others; the DSLR-A100's MakerNote records
/// none of them. The settings only a MakerNote records are `None` on the
/// Canon file, since the reader reads no Canon MakerNote.
const METADATA: &[SampleMetadata] = &[
    // ILCE-7M3 (Wikimedia Commons Acanthodactylus_erythrurus_2025.jpg)
    SampleMetadata {
        file: "sony-ilce-7m3.jpg",
        make: Some("SONY"),
        model: Some("ILCE-7M3"),
        software: Some("ILCE-7M3 v4.01"),
        serial_number: None,
        capture_time: Some("2025-06-05T10:47:33+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(320),
        exposure_time: Some((1, 200)),
        f_number: Some((160, 10)),
        focal_length: Some((160, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("16mm F1.4 DC DN | Contemporary 017"),
            serial_number: false,
            min_focal_length: Some((160, 10)),
            max_focal_length: Some((160, 10)),
            f_number_at_min_focal_length: Some((14, 10)),
            f_number_at_max_focal_length: Some((14, 10)),
        }),
        raw_size: None,
        image_size: Some((3936, 2624)),
        focus: Some(SampleFocus {
            frame: (3936, 2624),
            center: (1783, 1804),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7RM2 (Wikimedia Commons Ulpu_Pajari_puhumassa_vastaanotettuaan_tiedonjulkistamisen_valtionpalkinnon_2.jpg)
    SampleMetadata {
        file: "sony-ilce-7rm2.jpg",
        make: Some("SONY"),
        model: Some("ILCE-7RM2"),
        software: Some("ILCE-7RM2 v4.00"),
        serial_number: None,
        capture_time: Some("2026-06-08T20:03:02"),
        orientation: Orientation::NoTransforms,
        iso: Some(5000),
        exposure_time: Some((1, 160)),
        f_number: Some((40, 10)),
        focal_length: Some((1050, 10)),
        focal_length_in_35mm_film: Some(105),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 24-105mm F4 G OSS"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((1050, 10)),
            f_number_at_min_focal_length: Some((40, 10)),
            f_number_at_max_focal_length: Some((40, 10)),
        }),
        raw_size: None,
        image_size: Some((7952, 5304)),
        focus: Some(SampleFocus {
            frame: (7952, 5304),
            center: (3976, 2652),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSC-RX100M4 (Wikimedia Commons 여의도한강공원샛강생태공원문화다리_09.jpg)
    SampleMetadata {
        file: "sony-dsc-rx100m4.jpg",
        make: Some("SONY"),
        model: Some("DSC-RX100M4"),
        software: Some("DSC-RX100M4 v2.00"),
        serial_number: None,
        capture_time: Some("2019-11-19T10:51:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(80),
        exposure_time: Some((1, 320)),
        f_number: Some((40, 10)),
        focal_length: Some((880, 100)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((880, 100)),
            max_focal_length: Some((2570, 100)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: None,
        image_size: Some((5472, 3648)),
        focus: None, // manual focus
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Shade),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-6000 (Wikimedia Commons 140319_MS_Queen_Elizabeth_Kobe_Japan01o.JPG)
    SampleMetadata {
        file: "sony-ilce-6000.jpg",
        make: Some("SONY"),
        model: Some("ILCE-6000"),
        software: Some("ILCE-6000 v1.00"),
        serial_number: None,
        capture_time: Some("2014-03-19T07:55:02"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 200)),
        f_number: Some((80, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(36),
        exposure_bias: Some((-3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("E 16-70mm F4 ZA OSS"),
            serial_number: false,
            min_focal_length: Some((160, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((40, 10)),
            f_number_at_max_focal_length: Some((40, 10)),
        }),
        raw_size: None,
        image_size: Some((6000, 4000)),
        focus: None, // no FocusLocation: a point number only (AFPointSelected)
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSLR-A100 (Wikimedia Commons "Krone"_flag_on_line_5_tram_(2564030031).jpg)
    SampleMetadata {
        file: "sony-dslr-a100.jpg",
        make: Some("SONY"),
        model: Some("DSLR-A100"),
        software: Some("DSLR-A100 v1.04"),
        serial_number: None,
        capture_time: Some("2008-06-09T15:44:33"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((1, 640)),
        f_number: Some((80, 10)),
        focal_length: Some((3000, 10)),
        focal_length_in_35mm_film: Some(450),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: None,
        image_size: Some((3872, 2592)),
        focus: None, // no FocusLocation: only the A100's Minolta block, which is not read
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::ShutterPriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // NIKON D5300 (Wikimedia Commons "Tours"_French_restaurant_in_Omotesando.JPG)
    SampleMetadata {
        file: "nikon-d5300.jpg",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D5300"),
        software: Some("Ver.1.02"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2018-12-02T12:42:13.30+09:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(400),
        exposure_time: Some((10, 1250)),
        f_number: Some((56, 10)),
        focal_length: Some((180, 10)),
        focal_length_in_35mm_film: Some(27),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: None,
        image_size: Some((6000, 4000)),
        focus: None, // a viewfinder shot: a zero frame and only a point number
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // NIKON D70 (Wikimedia Commons "Stadswandeling".JPG)
    SampleMetadata {
        file: "nikon-d70.jpg",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D70"),
        software: Some("Ver.1.02"),
        serial_number: None,
        capture_time: Some("2009-06-07T11:52:30"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 2500)),
        f_number: Some((80, 10)),
        focal_length: Some((620, 10)),
        focal_length_in_35mm_film: Some(93),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((280, 10)),
            max_focal_length: Some((800, 10)),
            f_number_at_min_focal_length: Some((33, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: None,
        image_size: Some((3008, 2000)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Canon PowerShot G7 X (Wikimedia Commons DC-3_OH-LCH_at_EFJM_20160717.jpg)
    SampleMetadata {
        file: "canon-powershot-g7x.jpg",
        make: Some("Canon"),
        model: Some("Canon PowerShot G7 X"),
        software: None,
        serial_number: None,
        capture_time: Some("2016-07-17T17:30:26"),
        orientation: Orientation::NoTransforms,
        iso: Some(125),
        exposure_time: Some((1, 200)),
        f_number: Some((40, 10)),
        focal_length: Some((23008, 1000)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((0, 3)),
        lens: None,
        raw_size: None,
        image_size: Some((5472, 3648)),
        focus: None, // the MakerNote of a maker no reader reads
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
];

#[test]
fn every_jpeg_sample_gives_its_metadata() {
    for expected in METADATA {
        check_metadata(expected, Format::Jpeg);
    }
}

#[test]
fn the_sample_list_and_the_tables_of_previews_and_metadata_name_the_same_jpeg_files() {
    // The list is compiled in, so this runs without any download.
    let listed = listed("jpeg");
    let previews: Vec<&str> = SAMPLES.iter().map(|sample| sample.file).collect();
    let metadata: Vec<&str> = METADATA.iter().map(|expected| expected.file).collect();
    for (what, files) in [("previews", previews), ("metadata", metadata)] {
        let expected: BTreeSet<(&str, &str)> = files.iter().map(|&file| (file, "normal")).collect();
        assert_eq!(
            expected.len(),
            files.len(),
            "a file is in the tables of {what} twice"
        );
        let unexpected: Vec<_> = listed.difference(&expected).collect();
        let unlisted: Vec<_> = expected.difference(&listed).collect();
        assert!(
            unexpected.is_empty() && unlisted.is_empty(),
            "listed without expected {what}: {unexpected:?}; \
             expected {what} but not listed: {unlisted:?}"
        );
    }
}
