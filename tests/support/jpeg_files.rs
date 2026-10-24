// SPDX-License-Identifier: MIT OR Apache-2.0

//! Whole JPEG files, laid out as cameras and editors write them, so that
//! the JPEG reader's tests, the robustness tests and the fuzz seeds open the
//! same files. This is to JPEG files what `nef.rs` and `arw.rs` are to raw
//! ones: `jpeg_stub.rs` writes one stream, the smallest that has a frame
//! header, and `jpeg_builder.rs` writes a file from whatever parts a test
//! names, in any order; the functions here name the parts of each layout,
//! once, for every test that needs it.
//!
//! The layouts:
//!
//! - [`plain_jpeg`]: the main image alone, of [`MAIN`], with no Exif
//!   segment, as an editor may save a picture.
//! - [`exif_jpeg`]: an Exif segment, then the main image. The segment
//!   holds the TIFF structure [`exif`] writes: IFD0 with Orientation 6, and
//!   IFD1 with a thumbnail of [`THUMBNAIL`] inside the segment, which its
//!   JPEGInterchangeFormat and JPEGInterchangeFormatLength point at.
//! - [`camera_jpeg`] and [`camera_file`]: a camera's JPEG, laid out as the
//!   Nikon D5300's JPEG files are: an Exif segment, an MPF index, the main
//!   image, then the two [`MPF_IMAGES`], each after zeros up to a multiple
//!   of 512 bytes; with a JFIF segment first when asked, as an editor
//!   writes it. [`camera_exif`] writes the TIFF structure of such a file
//!   with every value the metadata reads: IFD0's Make, Model and Software,
//!   and an Exif IFD holding each of [`EXIF_IFD_TAGS`], with values made up
//!   in the shape a Nikon Z 9 records them; [`camera_jpeg_with_metadata`]
//!   is the camera's JPEG with that structure in its Exif segment.
//! - [`padded_jpeg`]: the main image, then 4,096 zeros after its EOI, as
//!   some Sony bodies write their JPEG files.
//! - [`nikon_jpeg`]: a Nikon camera's JPEG, laid out as [`camera_file`]'s,
//!   with or without the MPF index and its images, a main image of any
//!   size, [`camera_exif`]'s values but those a test leaves out, and a
//!   Nikon MakerNote, which `nef.rs`'s [`nikon_makernote_with`] writes, in
//!   the Exif IFD, at [`MAKER_NOTE_AT`] in the segment's TIFF structure,
//!   declared whole or cut short. Three bodies' layouts are built on it:
//!   [`z9_jpeg`], a Z 9's in FX or a crop mode ([`Z9Mode`]), at any size,
//!   whose MakerNote records the mode's CropHiSpeed and focus point, and
//!   the raw file's CropArea when asked, as an export that copied it
//!   holds; [`d5300_jpeg`], a D5300's, shot through the viewfinder, whose
//!   MakerNote records the serial number, the lens's range and the clock's
//!   time zone that its Exif segment lacks; and [`d70_jpeg`], a D70's,
//!   with no MPF index, whose MakerNote, in the other byte order from the
//!   file's, records the ISO and the lens's range and holds a preview of
//!   564 by 372 pixels, inside its declared length or just past it.
//! - [`d1h_jpeg`]: a JPEG laid out as a Nikon D1H's would be, as
//!   [`nikon_jpeg`]'s with no MPF index, under any Make, whose MakerNote,
//!   of the headerless layout the D1H writes in its raw files, is written
//!   by the builder of the segment's TIFF structure, so that its offsets
//!   count from that structure's header; it records the ISO and the lens's
//!   range that the Exif segment lacks, with any tags a test adds, and
//!   holds a preview of 570 by 375 pixels. No JPEG file of the D1 family
//!   is known.
//! - [`sony_jpeg`]: a Sony camera's JPEG, laid out as the ILCE-7M3's: an
//!   Exif segment, an MPF index, the main image of any size, the one MPF
//!   image of [`SONY_MPF_IMAGE`] and zeros after it, or, without the MPF
//!   index, the main image alone after the Exif segment; [`camera_exif`]'s
//!   values but [`SONY_LOST`], with a Sony Make and Model, or any; and a
//!   Sony MakerNote in the Exif IFD, the [`SONY_DSC`] header and then an IFD
//!   written by the builder of the segment's TIFF structure, so that its
//!   offsets count from that structure's header, as on Sony's camera JPEGs.
//!   Five bodies' layouts are built on it: [`ilce7m3_jpeg`], the
//!   ILCE-7M3's, at any size, whose MakerNote records its focus point in a
//!   frame of the JPEG's size, and [`ilce7m3_jpeg_with`], the same with
//!   any Make and more MakerNote tags; [`ilce7m4_jpeg`], with the focus
//!   point and area of the ILCE-7M4's raw file; [`rx100m4_jpeg`], a
//!   DSC-RX100M4's in manual focus; [`ilce6000_jpeg`], an ILCE-6000's,
//!   with no FocusLocation; and [`a100_jpeg`], a DSLR-A100's, with no MPF
//!   index and a MakerNote of an older layout.
//!
//! The TIFF structure of an Exif segment and the MPF index are each in the
//! byte order a test gives, a Nikon MakerNote of the embedded layout in the
//! one its layout gives, and a Nikon MakerNote of the headerless layout
//! and a Sony MakerNote in the TIFF structure's.

use std::ops::Range;

use super::arw::{
    self, ARW4_FOCUS_CENTER, ARW4_FOCUS_FRAME, ARW4_FULL_SIZE_JPEG, arw4_focus_frame_size,
};
use super::jpeg_builder::{Built, JpegBuilder};
use super::jpeg_stub::jpeg_stub;
use super::nef::{
    self, AF_INFO2, CROP_AREA, CROP_HI_SPEED, D1H_ISO, D1H_LENS, D1H_PREVIEW_JPEG, D300_LENS,
    FOCUS_MODE, PREVIEW_IFD, SERIAL_NUMBER, WORLD_TIME, Z9_FOCUS_AREA, Z9_FOCUS_CENTER,
    Z9_FOCUS_FRAME, af_info2_04xx, nikon_makernote_with,
};
use super::tiff_builder::{self, IfdId, Order, RawValue, TiffBuilder};

/// The camera's maker, in IFD0 (Make).
pub const MAKE: u16 = 0x010F;

/// The camera's model, in IFD0 (Model).
pub const MODEL: u16 = 0x0110;

/// The Orientation tag, in IFD0.
const ORIENTATION: u16 = 0x0112;

/// The firmware or program that wrote the file, in IFD0 (Software).
pub const SOFTWARE: u16 = 0x0131;

/// The JPEGInterchangeFormat tag: where a thumbnail's SOI lies, counted from
/// the TIFF header.
const JPEG_INTERCHANGE_FORMAT: u16 = 0x0201;

/// The JPEGInterchangeFormatLength tag: the thumbnail's length in bytes.
const JPEG_INTERCHANGE_FORMAT_LENGTH: u16 = 0x0202;

/// How long the shutter was open, in seconds, in the Exif IFD
/// (ExposureTime).
pub const EXPOSURE_TIME: u16 = 0x829A;

/// The f-number, in the Exif IFD (FNumber).
pub const F_NUMBER: u16 = 0x829D;

/// The ISO, in the Exif IFD (PhotographicSensitivity).
pub const PHOTOGRAPHIC_SENSITIVITY: u16 = 0x8827;

/// When the picture was taken, in the Exif IFD (DateTimeOriginal).
pub const DATE_TIME_ORIGINAL: u16 = 0x9003;

/// The capture time's offset from UTC, in the Exif IFD
/// (OffsetTimeOriginal).
pub const OFFSET_TIME_ORIGINAL: u16 = 0x9011;

/// The exposure compensation in EV, in the Exif IFD (ExposureBiasValue).
pub const EXPOSURE_BIAS_VALUE: u16 = 0x9204;

/// The focal length in millimetres, in the Exif IFD (FocalLength).
pub const FOCAL_LENGTH: u16 = 0x920A;

/// The focal length in 35 mm terms, in whole millimetres, in the Exif IFD
/// (FocalLengthIn35mmFilm).
pub const FOCAL_LENGTH_IN_35MM_FILM: u16 = 0xA405;

/// The capture time's fraction of a second, in the Exif IFD
/// (SubSecTimeOriginal).
pub const SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;

/// The camera body's serial number, in the Exif IFD (BodySerialNumber).
pub const BODY_SERIAL_NUMBER: u16 = 0xA431;

/// The lens's focal lengths and f-numbers, in the Exif IFD
/// (LensSpecification).
pub const LENS_SPECIFICATION: u16 = 0xA432;

/// The lens's maker, in the Exif IFD (LensMake).
pub const LENS_MAKE: u16 = 0xA433;

/// The lens's model, in the Exif IFD (LensModel).
pub const LENS_MODEL: u16 = 0xA434;

/// The lens's serial number, in the Exif IFD (LensSerialNumber).
pub const LENS_SERIAL_NUMBER: u16 = 0xA435;

/// The tags of the Exif IFD that [`camera_exif`] writes, each of which
/// gives a value of the metadata.
pub const EXIF_IFD_TAGS: [u16; 14] = [
    BODY_SERIAL_NUMBER,
    DATE_TIME_ORIGINAL,
    SUB_SEC_TIME_ORIGINAL,
    OFFSET_TIME_ORIGINAL,
    EXPOSURE_TIME,
    F_NUMBER,
    PHOTOGRAPHIC_SENSITIVITY,
    EXPOSURE_BIAS_VALUE,
    FOCAL_LENGTH,
    FOCAL_LENGTH_IN_35MM_FILM,
    LENS_MAKE,
    LENS_MODEL,
    LENS_SERIAL_NUMBER,
    LENS_SPECIFICATION,
];

/// The camera's make, in IFD0. This and the values below are those
/// [`camera_exif`] writes: made up, in the shape a Nikon Z 9 records them.
pub const CAMERA_MAKE: &str = "NIKON CORPORATION";
/// The camera's model, in IFD0.
pub const CAMERA_MODEL: &str = "NIKON Z 9";
/// The camera's firmware, in IFD0's Software.
pub const CAMERA_SOFTWARE: &str = "Ver.01.00";
/// The camera's serial number, in the Exif IFD.
pub const CAMERA_SERIAL_NUMBER: &str = "1234567";
/// The capture time: DateTimeOriginal, SubSecTimeOriginal and
/// OffsetTimeOriginal, in the Exif IFD; 15 June 2024 at 09:30:12, a
/// quarter of a second, two digits, and two hours ahead of UTC.
pub const CAPTURE_TIME: [&str; 3] = ["2024:06:15 09:30:12", "25", "+02:00"];
/// The exposure time, as a fraction, in the Exif IFD.
pub const EXPOSURE_SECONDS: (u32, u32) = (10, 2500);
/// The f-number, as a fraction, in the Exif IFD.
pub const APERTURE: (u32, u32) = (56, 10);
/// The ISO, in the Exif IFD.
pub const ISO: u16 = 400;
/// The exposure compensation, as a fraction, in the Exif IFD.
pub const EXPOSURE_BIAS: (i32, i32) = (-4, 6);
/// The focal length, as a fraction, in the Exif IFD.
pub const FOCAL_MILLIMETRES: (u32, u32) = (500, 10);
/// The focal length in 35 mm terms, in the Exif IFD: the same 50 mm, as on
/// the Z 9's full frame, FX.
pub const FOCAL_MILLIMETRES_IN_35MM_FILM: u16 = 50;
/// The lens's make, model and serial number, in the Exif IFD.
pub const LENS: [&str; 3] = ["NIKON", "NIKKOR Z 24-70mm f/2.8 S", "20012345"];
/// The lens's range, in the Exif IFD: the shortest and the longest focal
/// length, then the f-number at each.
pub const LENS_RANGE: [(u32, u32); 4] = [(240, 10), (700, 10), (28, 10), (28, 10)];

/// The Orientation value of a picture shown upright by a turn of 90
/// degrees clockwise.
pub const ROTATE_90: u16 = 6;

/// The size of the main image of every file here, a Z 9's.
pub const MAIN: (u16, u16) = (8256, 5504);

/// The size of the thumbnail in IFD1.
pub const THUMBNAIL: (u16, u16) = (160, 120);

/// The sizes of the two images of the MPF list after the main one, large
/// thumbnails of classes 1 and 2.
pub const MPF_IMAGES: [(u16, u16); 2] = [(640, 424), (1620, 1080)];

/// The length of the stub JPEG of `size`.
fn stub_len(size: (u16, u16)) -> u64 {
    u64::try_from(jpeg_stub(size.0, size.1, 0xC0).len()).unwrap()
}

/// A JPEG with no Exif segment: SOI, then the main image of [`MAIN`].
pub fn plain_jpeg() -> Built {
    let mut builder = JpegBuilder::new();
    builder.frame(MAIN.0, MAIN.1, 0xC0);
    builder.build()
}

/// The TIFF structure of an Exif segment, in byte order `order`: IFD0 with
/// Orientation `orientation`, and IFD1 with a thumbnail of [`THUMBNAIL`]
/// that JPEGInterchangeFormat and JPEGInterchangeFormatLength point at, at
/// `thumbnail_at` when given, counted from the TIFF header, and otherwise
/// where the builder puts it. Returns the bytes and where the builder put
/// the thumbnail in them.
pub fn exif(order: Order, orientation: u16, thumbnail_at: Option<u32>) -> (Vec<u8>, Range<u64>) {
    exif_with(order, orientation, thumbnail_at, |_, _| {})
}

/// As [`exif`], with what `tags` adds, given the builder and IFD0.
pub fn exif_with(
    order: Order,
    orientation: u16,
    thumbnail_at: Option<u32>,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Vec<u8>, Range<u64>) {
    let (built, thumbnail, ()) = exif_built(order, orientation, thumbnail_at, tags);
    (built.bytes, thumbnail)
}

/// As [`exif_with`], with what `tags` returns, and the built structure in
/// place of its bytes, so that a caller can find where the builder put
/// what `tags` added.
fn exif_built<T>(
    order: Order,
    orientation: u16,
    thumbnail_at: Option<u32>,
    tags: impl FnOnce(&mut TiffBuilder, IfdId) -> T,
) -> (tiff_builder::Built, Range<u64>, T) {
    let mut tiff = TiffBuilder::new(order);
    let (ifd0, ifd1) = (tiff.ifd(), tiff.ifd());
    tiff.short(ifd0, ORIENTATION, &[orientation])
        .next(ifd0, Some(ifd1));
    let added = tags(&mut tiff, ifd0);
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
    (built, at..at + stub_len(THUMBNAIL), added)
}

/// A JPEG whose Exif segment holds [`exif`]'s TIFF structure in byte order
/// `order`, with Orientation 6, then the main image of [`MAIN`]. Returns
/// the file and where its thumbnail lies in the TIFF structure.
pub fn exif_jpeg(order: Order) -> (Built, Range<u64>) {
    let (tiff, thumbnail) = exif(order, ROTATE_90, None);
    let mut builder = JpegBuilder::new();
    builder.exif(&tiff).frame(MAIN.0, MAIN.1, 0xC0);
    (builder.build(), thumbnail)
}

/// A camera's JPEG, laid out as [`camera_file`]'s, whose Exif segment holds
/// [`exif`]'s TIFF structure, with Orientation 6; that structure and the
/// MPF index are in byte order `order`. With `jfif`, a JFIF segment comes
/// first, as an editor writes it. Returns the file and where its thumbnail
/// lies in the TIFF structure.
pub fn camera_jpeg(order: Order, jfif: bool) -> (Built, Range<u64>) {
    let (tiff, thumbnail) = exif(order, ROTATE_90, None);
    (camera_file(order, jfif, &tiff, &[]), thumbnail)
}

/// A camera's JPEG: an Exif segment that holds the TIFF structure `tiff`,
/// an MPF index in byte order `order`, the main image of [`MAIN`], then the
/// two [`MPF_IMAGES`], each after zeros up to a multiple of 512 bytes, as
/// the Nikon D5300's JPEG files are laid out. With `jfif`, a JFIF segment
/// comes first, as an editor writes it. With `outside`, when not empty,
/// after the main image's EOI and before the MPF images: inside the file,
/// outside the segment.
pub fn camera_file(order: Order, jfif: bool, tiff: &[u8], outside: &[u8]) -> Built {
    camera_layout(order, jfif, tiff, outside, MAIN, true)
}

/// As [`camera_file`], with a main image of `main`, and with no MPF index
/// and no image after the main one unless `mpf`.
fn camera_layout(
    order: Order,
    jfif: bool,
    tiff: &[u8],
    outside: &[u8],
    main: (u16, u16),
    mpf: bool,
) -> Built {
    let mut builder = JpegBuilder::new();
    if jfif {
        builder.jfif();
    }
    builder.exif(tiff);
    if mpf {
        builder.mpf(order);
    }
    builder.frame(main.0, main.1, 0xC0);
    if !outside.is_empty() {
        builder.after_eoi(outside);
    }
    if mpf {
        for (width, height) in MPF_IMAGES {
            builder.mpf_image(width, height, &[], 512);
        }
    }
    builder.build()
}

/// The TIFF structure of a camera's Exif segment, in byte order `order`,
/// as [`exif`] writes it with Orientation 6, and with every value the
/// metadata reads: IFD0's Make, Model and Software, [`CAMERA_MAKE`],
/// [`CAMERA_MODEL`] and [`CAMERA_SOFTWARE`], and an Exif IFD holding each
/// of [`EXIF_IFD_TAGS`], with the values above, each of the type the Exif
/// standard gives it.
/// Then `change` adds what a test needs, given the builder, IFD0 and the
/// Exif IFD. Returns the bytes and where the builder put the thumbnail in
/// them.
pub fn camera_exif(
    order: Order,
    change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
) -> (Vec<u8>, Range<u64>) {
    let (built, thumbnail, ()) = camera_exif_built(order, &[], change);
    (built.bytes, thumbnail)
}

/// As [`camera_exif`], without the tags of `lost`, among IFD0's Make,
/// Model and Software and [`EXIF_IFD_TAGS`], and with what `change`
/// returns and the built structure in place of its bytes, so that a caller
/// can find where the builder put what `change` added.
fn camera_exif_built<T>(
    order: Order,
    lost: &[u16],
    change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId) -> T,
) -> (tiff_builder::Built, Range<u64>, T) {
    exif_built(order, ROTATE_90, None, |tiff, ifd0| {
        let exif = tiff.ifd();
        for tag in [MAKE, MODEL, SOFTWARE] {
            if !lost.contains(&tag) {
                camera_value(tiff, ifd0, tag);
            }
        }
        tiff.exif(ifd0, exif);
        for tag in EXIF_IFD_TAGS {
            if !lost.contains(&tag) {
                camera_value(tiff, exif, tag);
            }
        }
        change(tiff, ifd0, exif)
    })
}

/// Adds to `ifd` the entry of `tag`, one of IFD0's Make, Model and
/// Software or of [`EXIF_IFD_TAGS`], with the value [`camera_exif`] gives
/// it, of the type the Exif standard gives it.
fn camera_value(tiff: &mut TiffBuilder, ifd: IfdId, tag: u16) {
    let [date_time, subseconds, offset] = CAPTURE_TIME;
    let [lens_make, lens_model, lens_serial_number] = LENS;
    match tag {
        MAKE => tiff.ascii(ifd, MAKE, CAMERA_MAKE),
        MODEL => tiff.ascii(ifd, MODEL, CAMERA_MODEL),
        SOFTWARE => tiff.ascii(ifd, SOFTWARE, CAMERA_SOFTWARE),
        BODY_SERIAL_NUMBER => tiff.ascii(ifd, tag, CAMERA_SERIAL_NUMBER),
        DATE_TIME_ORIGINAL => tiff.ascii(ifd, tag, date_time),
        SUB_SEC_TIME_ORIGINAL => tiff.ascii(ifd, tag, subseconds),
        OFFSET_TIME_ORIGINAL => tiff.ascii(ifd, tag, offset),
        EXPOSURE_TIME => tiff.rational(ifd, tag, &[EXPOSURE_SECONDS]),
        F_NUMBER => tiff.rational(ifd, tag, &[APERTURE]),
        PHOTOGRAPHIC_SENSITIVITY => tiff.short(ifd, tag, &[ISO]),
        EXPOSURE_BIAS_VALUE => tiff.srational(ifd, tag, &[EXPOSURE_BIAS]),
        FOCAL_LENGTH => tiff.rational(ifd, tag, &[FOCAL_MILLIMETRES]),
        FOCAL_LENGTH_IN_35MM_FILM => tiff.short(ifd, tag, &[FOCAL_MILLIMETRES_IN_35MM_FILM]),
        LENS_MAKE => tiff.ascii(ifd, tag, lens_make),
        LENS_MODEL => tiff.ascii(ifd, tag, lens_model),
        LENS_SERIAL_NUMBER => tiff.ascii(ifd, tag, lens_serial_number),
        LENS_SPECIFICATION => tiff.rational(ifd, tag, &LENS_RANGE),
        _ => panic!("no camera value for tag {tag:#06X}"),
    };
}

/// A camera's JPEG laid out as [`camera_file`]'s, with no JFIF segment,
/// whose Exif segment holds [`camera_exif`]'s TIFF structure, with every
/// value the metadata reads; that structure and the MPF index are in byte
/// order `order`.
/// Returns the file and where its thumbnail lies in the TIFF structure.
pub fn camera_jpeg_with_metadata(order: Order) -> (Built, Range<u64>) {
    let (tiff, thumbnail) = camera_exif(order, |_, _, _| {});
    (camera_file(order, false, &tiff, &[]), thumbnail)
}

/// The MakerNote tag, in the Exif IFD: the camera maker's own metadata.
pub const MAKER_NOTE: u16 = 0x927C;

/// Where a Nikon JPEG's MakerNote lies in its Exif segment's TIFF
/// structure, counted from that structure's header: past everything else
/// the structure holds, with zeros before it.
pub const MAKER_NOTE_AT: u32 = 1024;

/// A Nikon camera's JPEG, as [`nikon_jpeg`] lays it out, and where its
/// parts are.
pub struct NikonJpeg {
    /// The file.
    pub built: Built,
    /// Where IFD1's thumbnail lies in the Exif segment's TIFF structure,
    /// counted from its header.
    pub thumbnail: Range<u64>,
    /// Where the MakerNote's block lies in the file.
    pub makernote: Range<u64>,
}

/// A Nikon camera's JPEG: laid out as [`camera_file`]'s, with no JFIF
/// segment, a main image of `main`, and with the MPF index and the two
/// [`MPF_IMAGES`] only when `mpf`; its Exif segment holds the TIFF
/// structure [`camera_exif`] writes in byte order `order`, without the
/// tags of `lost`, among IFD0's Make and Model and [`EXIF_IFD_TAGS`], with
/// what `change` adds, given the builder, IFD0 and the Exif IFD, and with
/// the MakerNote `makernote`, an UNDEFINED value in the Exif IFD at
/// [`MAKER_NOTE_AT`], of which the MakerNote's entry declares the first
/// `declared` bytes. The MPF index is in byte order `order` too.
pub fn nikon_jpeg(
    order: Order,
    main: (u16, u16),
    mpf: bool,
    lost: &[u16],
    change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
    makernote: &[u8],
    declared: usize,
) -> NikonJpeg {
    let (tiff, thumbnail, ()) = camera_exif_built(order, lost, |tiff, ifd0, exif| {
        change(tiff, ifd0, exif);
        tiff.blob_at(u64::from(MAKER_NOTE_AT), makernote);
        let count = u32::try_from(declared).unwrap();
        tiff.raw(
            exif,
            MAKER_NOTE,
            UNDEFINED,
            count,
            RawValue::Offset(MAKER_NOTE_AT),
        );
    });
    let built = camera_layout(order, false, &tiff.bytes, &[], main, mpf);
    let at = built.exif_tiff(0).start + u64::from(MAKER_NOTE_AT);
    let len = u64::try_from(makernote.len()).unwrap();
    NikonJpeg {
        built,
        thumbnail,
        makernote: at..at + len,
    }
}

/// The type UNDEFINED, of the MakerNote's value.
const UNDEFINED: u16 = 7;

/// A mode a Nikon Z 9 takes a picture in, as its MakerNote records it in
/// its JPEGs, for [`z9_jpeg`]: the camera's full output, FX, or one of its
/// crops. The values of each are those the NEF notes record of shots made
/// with the maintainer's Z 9, and of the public Z 9 samples for FX.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Z9Mode {
    /// No crop: the full 8256 by 5504 output.
    Fx,
    /// The DX crop, which cuts both sides.
    Dx,
    /// The 1:1 crop, which keeps the height and cuts the width.
    Square,
    /// The 16:9 crop, which keeps the width and cuts the height.
    Wide,
}

impl Z9Mode {
    /// Every mode.
    pub const ALL: [Z9Mode; 4] = [Z9Mode::Fx, Z9Mode::Dx, Z9Mode::Square, Z9Mode::Wide];

    /// The mode's CropHiSpeed: the mode's number, the original width and
    /// height, the cropped width and height, and the left column and top
    /// row where the crop starts. FX's two sizes are equal: no crop.
    pub fn crop_hi_speed(self) -> [u16; 7] {
        match self {
            Z9Mode::Fx => [11, 8280, 5520, 8280, 5520, 0, 0],
            Z9Mode::Dx => [2, 8280, 5520, 5408, 3608, 1436, 956],
            Z9Mode::Square => [17, 8280, 5520, 5520, 5520, 1380, 0],
            Z9Mode::Wide => [6, 8280, 5520, 8280, 4656, 0, 432],
        }
    }

    /// The CropArea of the mode's raw file: what a MakerNote copied from
    /// that raw file into an exported JPEG holds, and the size of the
    /// mode's JPEG at full size.
    pub fn crop_area(self) -> [u16; 4] {
        match self {
            Z9Mode::Fx => [12, 8, 8256, 5504],
            Z9Mode::Dx => [8, 8, 5392, 3592],
            Z9Mode::Square => [8, 8, 5504, 5504],
            Z9Mode::Wide => [12, 8, 8256, 4640],
        }
    }

    /// What the mode's AFInfo2 records, in the full frame of 8256 by 5504:
    /// the frame, the centre and the area, each width or x first. On FX,
    /// the point of the public Z 9 samples 5146 to 5148; on a crop, the
    /// point of the maintainer's shot with a single point in the top-left
    /// corner of the picture.
    pub fn recorded(self) -> [(u32, u32); 3] {
        let frame = Z9_FOCUS_FRAME;
        match self {
            Z9Mode::Fx => [frame, Z9_FOCUS_CENTER, Z9_FOCUS_AREA],
            Z9Mode::Dx => [frame, (636, 572), (420, 468)],
            Z9Mode::Square => [frame, (1797, 424), (291, 323)],
            Z9Mode::Wide => [frame, (502, 715), (291, 323)],
        }
    }
}

/// A Nikon Z 9's JPEG taken in mode `mode`, with a main image of `main`:
/// laid out as [`nikon_jpeg`]'s, with the MPF index and its two images,
/// and every value [`camera_exif`] writes, in byte order `order`; and a
/// MakerNote in byte order `makernote_order`, of the version `0211`, as
/// the Z 9 writes it, which holds the FocusMode `AF-S  `, the mode's
/// CropHiSpeed, the mode's AFInfo2 of version `0400`, with the area mode
/// 193 (single point) and focus confirmed, and, only when `crop_area`, the
/// CropArea of the mode's raw file, as a MakerNote copied from that raw
/// file into an export holds it; and no PreviewIFD, as the D5300's JPEGs
/// hold none. A camera's JPEG of a crop mode has no CropArea (ADR015).
pub fn z9_jpeg(
    order: Order,
    makernote_order: Order,
    mode: Z9Mode,
    main: (u16, u16),
    crop_area: bool,
) -> NikonJpeg {
    let (block, _) = nikon_makernote_with(makernote_order, [0x02, 0x11], None, |inner, ifd| {
        inner
            .ascii(ifd, FOCUS_MODE, "AF-S  ")
            .short(ifd, CROP_HI_SPEED, &mode.crop_hi_speed());
        if crop_area {
            inner.short(ifd, CROP_AREA, &mode.crop_area());
        }
        let af_info2 = af_info2_04xx(makernote_order, b"0400", 0xC1, mode.recorded(), 1);
        inner.undefined(ifd, AF_INFO2, &af_info2);
    });
    nikon_jpeg(order, main, true, &[], |_, _, _| {}, &block, block.len())
}

/// The Model of the D5300's JPEG, in IFD0.
pub const D5300_MODEL: &str = "NIKON D5300";
/// The size of the D5300's JPEG.
pub const D5300_MAIN: (u16, u16) = (6000, 4000);
/// The tags of [`camera_exif`] that the D5300's JPEG leaves out: its own
/// Model takes the Z 9's place, and, as on the bodies before the D780 and
/// the Z 6, its Exif IFD holds no serial number and no lens tags, and, as
/// before the D850, no offset from UTC.
pub const D5300_LOST: [u16; 7] = [
    MODEL,
    BODY_SERIAL_NUMBER,
    OFFSET_TIME_ORIGINAL,
    LENS_MAKE,
    LENS_MODEL,
    LENS_SERIAL_NUMBER,
    LENS_SPECIFICATION,
];
/// The serial number in the D5300's MakerNote: made up, as no real serial
/// number is copied here.
pub const D5300_SERIAL_NUMBER: &str = "3456789";
/// The ISO in the D5300's MakerNote, the second of its two SHORTs: the
/// Exif ISO, [`ISO`].
pub const D5300_ISO: u16 = ISO;
/// The time zone of the D5300's WorldTime, in minutes east of UTC, with
/// daylight saving time off: made up.
pub const D5300_TIME_ZONE: i16 = 540;
/// The lens's range in the D5300's MakerNote: an 18-55 mm lens of f/3.5 to
/// f/5.6, made up.
pub const D5300_LENS: [(u32, u32); 4] = [(180, 10), (550, 10), (35, 10), (56, 10)];

/// A JPEG laid out as a Nikon D5300's, a DSLR shot through the viewfinder,
/// in byte order `order`, MakerNote and all: laid out as [`nikon_jpeg`]'s,
/// with the MPF index and its two images, as the D5300's JPEGs are, and a
/// main image of [`D5300_MAIN`]; the values [`camera_exif`] writes, but
/// [`D5300_LOST`], with IFD0's Model [`D5300_MODEL`]; and a MakerNote of
/// the version `0211` with no PreviewIFD, as the D5300's JPEG on Wikimedia
/// Commons has none. The MakerNote holds the ISO `[0, D5300_ISO]`, the
/// FocusMode `AF-A  `, the SerialNumber [`D5300_SERIAL_NUMBER`], the
/// WorldTime of [`D5300_TIME_ZONE`], daylight saving time off and the date
/// format 0, the Lens [`D5300_LENS`], and an AFInfo2 of version `0100`,
/// its 30 bytes zero after the version, as that JPEG's is: a zero frame,
/// which records no position.
pub fn d5300_jpeg(order: Order) -> NikonJpeg {
    let mut world_time = order.u16(D5300_TIME_ZONE.cast_unsigned()).to_vec();
    world_time.extend_from_slice(&[0, 0]);
    let mut af_info2 = b"0100".to_vec();
    af_info2.resize(30, 0);
    let (block, _) = nikon_makernote_with(order, [0x02, 0x11], None, |inner, ifd| {
        inner
            .short(ifd, nef::ISO, &[0, D5300_ISO])
            .ascii(ifd, FOCUS_MODE, "AF-A  ")
            .ascii(ifd, SERIAL_NUMBER, D5300_SERIAL_NUMBER)
            .undefined(ifd, WORLD_TIME, &world_time)
            .rational(ifd, nef::LENS, &D5300_LENS)
            .undefined(ifd, AF_INFO2, &af_info2);
    });
    nikon_jpeg(
        order,
        D5300_MAIN,
        true,
        &D5300_LOST,
        |tiff, ifd0, _| {
            tiff.ascii(ifd0, MODEL, D5300_MODEL);
        },
        &block,
        block.len(),
    )
}

/// The Model of the D70's JPEG, in IFD0.
pub const D70_MODEL: &str = "NIKON D70";
/// The size of the D70's JPEG.
pub const D70_MAIN: (u16, u16) = (3008, 2000);
/// The size of the JPEG the D70's MakerNote's PreviewIFD declares, as in
/// the D70's JPEG on Wikimedia Commons.
pub const D70_PREVIEW: (u16, u16) = (564, 372);
/// The tags of [`camera_exif`] that the D70's JPEG leaves out: its own
/// Model takes the Z 9's place, and, as on the D70, its Exif IFD holds no
/// ISO, no serial number, no lens tags and no offset from UTC.
pub const D70_LOST: [u16; 8] = [
    MODEL,
    BODY_SERIAL_NUMBER,
    OFFSET_TIME_ORIGINAL,
    PHOTOGRAPHIC_SENSITIVITY,
    LENS_MAKE,
    LENS_MODEL,
    LENS_SERIAL_NUMBER,
    LENS_SPECIFICATION,
];
/// The ISO in the D70's MakerNote, the second of its two SHORTs.
pub const D70_ISO: u16 = 200;

/// The byte order other than `order`.
fn other(order: Order) -> Order {
    match order {
        Order::Little => Order::Big,
        Order::Big => Order::Little,
    }
}

/// A JPEG laid out as a Nikon D70's, in byte order `order`, with its
/// MakerNote in the other byte order, as the D70's JPEG on Wikimedia
/// Commons is, little-endian with a big-endian MakerNote: laid out as
/// [`nikon_jpeg`]'s, with no MPF index, as the D70's JPEGs have none, and
/// a main image of [`D70_MAIN`]; the values [`camera_exif`] writes, but
/// [`D70_LOST`], with IFD0's Model [`D70_MODEL`]; and a MakerNote of the
/// version `0210`, which holds the ISO `[0, D70_ISO]`, the FocusMode
/// `AF-S  `, the Lens `D300_LENS`, an 18-70 mm lens of f/3.5 to f/4.5, and
/// a PreviewIFD that declares a JPEG of [`D70_PREVIEW`], the block's last
/// bytes, and no AFInfo2, as the D70 writes none. The MakerNote's entry
/// declares its whole block unless `preview_outside`, when it declares the
/// block up to where the preview's JPEG starts, so that the JPEG lies
/// inside the Exif segment but outside the MakerNote. Returns the file and
/// where the preview's JPEG lies in it.
pub fn d70_jpeg(order: Order, preview_outside: bool) -> (NikonJpeg, Range<u64>) {
    let makernote_order = other(order);
    let preview = jpeg_stub(D70_PREVIEW.0, D70_PREVIEW.1, 0xC0);
    let (block, at) = nikon_makernote_with(
        makernote_order,
        [0x02, 0x10],
        Some(&preview),
        |inner, ifd| {
            inner
                .short(ifd, nef::ISO, &[0, D70_ISO])
                .ascii(ifd, FOCUS_MODE, "AF-S  ")
                .rational(ifd, nef::LENS, &D300_LENS);
        },
    );
    let at = at.expect("a PreviewIFD");
    let declared = if preview_outside {
        usize::try_from(at).unwrap()
    } else {
        block.len()
    };
    let file = nikon_jpeg(
        order,
        D70_MAIN,
        false,
        &D70_LOST,
        |tiff, ifd0, _| {
            tiff.ascii(ifd0, MODEL, D70_MODEL);
        },
        &block,
        declared,
    );
    let start = file.makernote.start + at;
    let end = start + u64::try_from(preview.len()).unwrap();
    (file, start..end)
}

/// The Model of the D1H's JPEG, in IFD0.
pub const D1H_MODEL: &str = "NIKON D1H";
/// The size of the D1H's JPEG: made up, as no JPEG file of the D1 family
/// is known.
pub const D1H_MAIN: (u16, u16) = (2000, 1312);
/// The tags of [`camera_exif`] that the D1H's JPEG leaves out: its own
/// Make and Model take the Z 9's place, and, as in the D1H's raw file, its
/// Exif IFD holds no ISO, no focal length in 35 mm terms, no serial number,
/// no lens tags and no offset from UTC.
pub const D1H_LOST: [u16; 10] = [
    MAKE,
    MODEL,
    BODY_SERIAL_NUMBER,
    OFFSET_TIME_ORIGINAL,
    PHOTOGRAPHIC_SENSITIVITY,
    FOCAL_LENGTH_IN_35MM_FILM,
    LENS_MAKE,
    LENS_MODEL,
    LENS_SERIAL_NUMBER,
    LENS_SPECIFICATION,
];

/// A JPEG laid out as a Nikon D1H's would be, in byte order `order`,
/// MakerNote and all, with IFD0's Make `make`: no JPEG file of the D1
/// family is known, so this one gives the MakerNote of the D1H's raw file
/// to a JPEG laid out as [`nikon_jpeg`]'s, with no MPF index and a main
/// image of [`D1H_MAIN`]; the values [`camera_exif`] writes, but
/// [`D1H_LOST`], with IFD0's Model [`D1H_MODEL`]. The MakerNote is of the
/// headerless layout: the MakerNote IFD at the block's first byte, written
/// by the builder of the segment's TIFF structure, so that its offsets
/// count from that structure's header, and its numbers are in byte order
/// `order`, as in the D1H's raw file they count from the file's own. The
/// MakerNote IFD holds the ISO `[0, D1H_ISO]`, the FocusMode `AF-S  `, the
/// Lens `D1H_LENS`, an 18-70 mm lens of f/3.5 to f/4.5, the PreviewIFD
/// pointer, and what `tags` adds, given the builder and that IFD; its
/// PreviewIFD declares a JPEG of `D1H_PREVIEW_JPEG`, 570 by 375. The block
/// runs from the MakerNote IFD through its values and the PreviewIFD to
/// the end of that JPEG, and the MakerNote's entry declares it whole.
/// Returns the file and where the preview's JPEG lies in it.
pub fn d1h_jpeg(
    order: Order,
    make: &str,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (NikonJpeg, Range<u64>) {
    let preview = jpeg_stub(D1H_PREVIEW_JPEG.0, D1H_PREVIEW_JPEG.1, 0xC0);
    let (tiff, thumbnail, (makernote, jpeg)) =
        camera_exif_built(order, &D1H_LOST, |tiff, ifd0, exif| {
            tiff.ascii(ifd0, MAKE, make).ascii(ifd0, MODEL, D1H_MODEL);
            let makernote = tiff.ifd();
            let preview_ifd = tiff.ifd();
            tiff.short(makernote, nef::ISO, &[0, D1H_ISO])
                .ascii(makernote, FOCUS_MODE, "AF-S  ")
                .ifd_pointer(makernote, PREVIEW_IFD, preview_ifd)
                .rational(makernote, nef::LENS, &D1H_LENS);
            tags(tiff, makernote);
            // The first blob, so that it follows the PreviewIFD, inside the
            // block.
            let jpeg = tiff.blob(&preview);
            tiff.long_blob(preview_ifd, JPEG_INTERCHANGE_FORMAT, jpeg)
                .long_blob_len(preview_ifd, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg)
                .block_through(makernote, jpeg)
                .undefined_ifd(exif, MAKER_NOTE, makernote);
            (makernote, jpeg)
        });
    let built = camera_layout(order, false, &tiff.bytes, &[], D1H_MAIN, false);
    let base = built.exif_tiff(0).start;
    let block = tiff.ifd_block(makernote);
    let at = base + tiff.blob_offset(jpeg);
    let file = NikonJpeg {
        built,
        thumbnail,
        makernote: base + block.start..base + block.end,
    };
    (file, at..at + u64::try_from(preview.len()).unwrap())
}

/// The Make of a Sony camera's JPEG, in IFD0.
pub const SONY_MAKE: &str = "SONY";
/// The header a Sony camera writes before the MakerNote IFD of its JPEGs
/// (`SONY DSC `, then three NULs), on every Sony camera JPEG the notes
/// record.
pub const SONY_DSC: &[u8; 12] = b"SONY DSC \0\0\0";
/// The tags of [`camera_exif`] that a Sony camera's JPEG leaves out: its
/// own Make and Model take the Z 9's place, and the lens tags, which name
/// a Nikon lens, are left out.
pub const SONY_LOST: [u16; 6] = [
    MAKE,
    MODEL,
    LENS_MAKE,
    LENS_MODEL,
    LENS_SERIAL_NUMBER,
    LENS_SPECIFICATION,
];
/// The size of the one image the MPF list of a Sony camera's JPEG holds
/// after the main one, a large thumbnail of class 2.
pub const SONY_MPF_IMAGE: (u16, u16) = (1616, 1080);
/// The zeros after the last image of a Sony camera's JPEG with an MPF
/// list: a few, where the cameras write tens of thousands, up to a
/// multiple of 64 KiB, so that the robustness tests sweep the file
/// quickly.
pub const SONY_PADDING: usize = 512;

/// A Sony camera's JPEG, as [`sony_jpeg`] lays it out, and where its parts
/// are.
pub struct SonyJpeg {
    /// The file.
    pub built: Built,
    /// Where IFD1's thumbnail lies in the Exif segment's TIFF structure,
    /// counted from its header.
    pub thumbnail: Range<u64>,
    /// Where the MakerNote IFD lies in the file: its count of entries,
    /// right after the [`SONY_DSC`] header.
    pub makernote_ifd: u64,
}

/// A Sony camera's JPEG: an Exif segment, then, only when `mpf`, an MPF
/// index in byte order `order`; the main image of `main`; and, only when
/// `mpf`, the image of [`SONY_MPF_IMAGE`] after zeros up to a multiple of
/// 512 bytes, then [`SONY_PADDING`] zeros, as the ILCE-7M3, ILCE-7RM2 and
/// ILCE-6000 lay their JPEGs out (the ILCE-7M3's XMP segment left out).
/// The Exif segment holds the TIFF structure [`camera_exif`] writes in byte
/// order `order`, but [`SONY_LOST`], with IFD0's Make `make` and Model
/// `model`, and with a Sony MakerNote in the Exif IFD: [`SONY_DSC`], then
/// the MakerNote IFD with what `tags` adds, given the builder and that IFD,
/// and the IFD's values, all written by the builder of the TIFF structure,
/// so that every offset counts from that structure's header, as on the
/// Sony camera JPEGs the notes record, and every number is in byte order
/// `order`.
pub fn sony_jpeg(
    order: Order,
    make: &str,
    model: &str,
    main: (u16, u16),
    mpf: bool,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> SonyJpeg {
    let (tiff, thumbnail, makernote) = camera_exif_built(order, &SONY_LOST, |tiff, ifd0, exif| {
        tiff.ascii(ifd0, MAKE, make).ascii(ifd0, MODEL, model);
        let makernote = tiff.ifd();
        tiff.prefix(makernote, SONY_DSC);
        tags(tiff, makernote);
        tiff.undefined_ifd(exif, MAKER_NOTE, makernote);
        makernote
    });
    let mut builder = JpegBuilder::new();
    builder.exif(&tiff.bytes);
    if mpf {
        builder.mpf(order);
    }
    builder.frame(main.0, main.1, 0xC0);
    if mpf {
        builder
            .mpf_image(SONY_MPF_IMAGE.0, SONY_MPF_IMAGE.1, &[], 512)
            .pad(SONY_PADDING);
    }
    let built = builder.build();
    let makernote_ifd = built.exif_tiff(0).start + tiff.ifd_offset(makernote);
    SonyJpeg {
        built,
        thumbnail,
        makernote_ifd,
    }
}

/// The Model of the ILCE-7M3's JPEG, in IFD0.
pub const ILCE7M3_MODEL: &str = "ILCE-7M3";
/// The size of the ILCE-7M3's JPEG at the camera's M size, as in the
/// ILCE-7M3's JPEG on Wikimedia Commons.
pub const ILCE7M3_MAIN: (u16, u16) = (3936, 2624);
/// The FocusLocation of the ILCE-7M3's JPEG on Wikimedia Commons, as ADR014
/// records it: the frame, the JPEG's own size, then the centre, off
/// centre.
pub const ILCE7M3_FOCUS_LOCATION: [u16; 4] = [3936, 2624, 1783, 1804];

/// A JPEG laid out as the Sony ILCE-7M3's, in byte order `order`, with a
/// main image of `main`, its MakerNote and all: laid out as
/// [`sony_jpeg`]'s, with the MPF index, its image and the padding, with
/// IFD0's Make [`SONY_MAKE`] and Model [`ILCE7M3_MODEL`]; its MakerNote
/// holds FocusMode 3 (AF-C), one BYTE, and FocusLocation
/// [`ILCE7M3_FOCUS_LOCATION`], four SHORTs, as the ILCE-7M3's JPEG on
/// Wikimedia Commons records them. A `main` other than [`ILCE7M3_MAIN`]
/// stands for an export that kept the camera's MakerNote.
pub fn ilce7m3_jpeg(order: Order, main: (u16, u16)) -> SonyJpeg {
    ilce7m3_jpeg_with(order, SONY_MAKE, main, |_, _| {})
}

/// As [`ilce7m3_jpeg`], with IFD0's Make `make`, and with what `tags` adds
/// to the MakerNote IFD, given the builder and that IFD.
pub fn ilce7m3_jpeg_with(
    order: Order,
    make: &str,
    main: (u16, u16),
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> SonyJpeg {
    sony_jpeg(order, make, ILCE7M3_MODEL, main, true, |tiff, ifd| {
        tiff.byte(ifd, arw::FOCUS_MODE, &[3]).short(
            ifd,
            arw::FOCUS_LOCATION,
            &ILCE7M3_FOCUS_LOCATION,
        );
        tags(tiff, ifd);
    })
}

/// The Model of the ILCE-7M4's JPEG, in IFD0.
pub const ILCE7M4_MODEL: &str = "ILCE-7M4";

/// A JPEG laid out as the Sony ILCE-7M3's, in byte order `order`, but of
/// an ILCE-7M4, a body that records the size of its focus area: IFD0's
/// Model [`ILCE7M4_MODEL`], a main image of `ARW4_FULL_SIZE_JPEG`, 7008 by
/// 4672, and a MakerNote that holds the focus tags of the ARW 4 fixtures,
/// those of the public ILCE-7M4 sample 6929 of raw.pixls.us: FocusMode 3
/// (AF-C), FocusLocation of `ARW4_FOCUS_FRAME` and `ARW4_FOCUS_CENTER`, and
/// FocusFrameSize of `ARW4_FOCUS_SIZE` and a third number, its six bytes
/// in byte order `order`. No ILCE-7M4 JPEG is recorded in the notes; this
/// one gives its raw file's values to a JPEG of the raw file's size.
pub fn ilce7m4_jpeg(order: Order) -> SonyJpeg {
    let (frame_width, frame_height) = ARW4_FOCUS_FRAME;
    let (x, y) = ARW4_FOCUS_CENTER;
    sony_jpeg(
        order,
        SONY_MAKE,
        ILCE7M4_MODEL,
        ARW4_FULL_SIZE_JPEG,
        true,
        |tiff, ifd| {
            tiff.byte(ifd, arw::FOCUS_MODE, &[3])
                .short(ifd, arw::FOCUS_LOCATION, &[frame_width, frame_height, x, y])
                .undefined(ifd, arw::FOCUS_FRAME_SIZE, &arw4_focus_frame_size(order));
        },
    )
}

/// The Model of the DSC-RX100M4's JPEG, in IFD0.
pub const RX100M4_MODEL: &str = "DSC-RX100M4";
/// The size of the DSC-RX100M4's JPEG, the frame its FocusLocation records.
pub const RX100M4_MAIN: (u16, u16) = (5472, 3648);

/// A JPEG laid out as the Sony ILCE-7M3's, in byte order `order`, but of
/// a DSC-RX100M4 shot in manual focus: IFD0's Model [`RX100M4_MODEL`], a
/// main image of [`RX100M4_MAIN`], and a MakerNote that holds FocusMode 0,
/// manual focus, one BYTE, and the FocusLocation of the frame's exact
/// centre, as the DSC-RX100M4's JPEG on Wikimedia Commons records them.
pub fn rx100m4_jpeg(order: Order) -> SonyJpeg {
    let (width, height) = RX100M4_MAIN;
    sony_jpeg(
        order,
        SONY_MAKE,
        RX100M4_MODEL,
        RX100M4_MAIN,
        true,
        |tiff, ifd| {
            tiff.byte(ifd, arw::FOCUS_MODE, &[0]).short(
                ifd,
                arw::FOCUS_LOCATION,
                &[width, height, width / 2, height / 2],
            );
        },
    )
}

/// The Model of the ILCE-6000's JPEG, in IFD0.
pub const ILCE6000_MODEL: &str = "ILCE-6000";
/// The size of the ILCE-6000's JPEG.
pub const ILCE6000_MAIN: (u16, u16) = (6000, 4000);

/// A JPEG laid out as the Sony ILCE-7M3's, in byte order `order`, but of
/// an ILCE-6000, a body before the ILCE-7RM2 that writes no FocusLocation:
/// IFD0's Model [`ILCE6000_MODEL`], a main image of [`ILCE6000_MAIN`], and
/// a MakerNote that holds FocusMode 4 (AF-A), one BYTE, and no
/// FocusLocation, as the ILCE-6000's JPEG on Wikimedia Commons records.
pub fn ilce6000_jpeg(order: Order) -> SonyJpeg {
    sony_jpeg(
        order,
        SONY_MAKE,
        ILCE6000_MODEL,
        ILCE6000_MAIN,
        true,
        |tiff, ifd| {
            tiff.byte(ifd, arw::FOCUS_MODE, &[4]);
        },
    )
}

/// The Model of the DSLR-A100's JPEG, in IFD0.
pub const A100_MODEL: &str = "DSLR-A100";
/// The size of the DSLR-A100's JPEG: the image size its raw file records.
pub const A100_MAIN: (u16, u16) = (3872, 2592);
/// Tags of the DSLR-A100's MakerNote IFD, in its older layout, that the
/// notes record: SceneMode, ZoneMatching, DynamicRangeOptimizer and
/// ImageStabilization, then the offset of a Minolta-format IFD
/// (MinoltaMakerNote), whose 0 means none.
pub const A100_MAKER_NOTE_TAGS: [u16; 5] = [0xB023, 0xB024, 0xB025, 0xB026, 0xB028];

/// A JPEG laid out as the Sony DSLR-A100's, in byte order `order` (the
/// DSLR-A100's JPEG on Wikimedia Commons is big-endian): laid out as
/// [`sony_jpeg`]'s, with no MPF index and no zeros after its end, as that
/// JPEG has none, and a main image of [`A100_MAIN`], IFD0's Make
/// [`SONY_MAKE`] and Model [`A100_MODEL`]; its MakerNote IFD, after the
/// same header, holds [`A100_MAKER_NOTE_TAGS`], each one LONG of 0, made
/// up, and neither FocusMode nor FocusLocation.
pub fn a100_jpeg(order: Order) -> SonyJpeg {
    sony_jpeg(
        order,
        SONY_MAKE,
        A100_MODEL,
        A100_MAIN,
        false,
        |tiff, ifd| {
            for tag in A100_MAKER_NOTE_TAGS {
                tiff.long(ifd, tag, &[0]);
            }
        },
    )
}

/// A JPEG with no Exif segment whose main image, of [`MAIN`], is followed
/// by 4,096 zeros after its EOI, as some Sony bodies write it.
pub fn padded_jpeg() -> Built {
    let mut builder = JpegBuilder::new();
    builder.frame(MAIN.0, MAIN.1, 0xC0).pad(4096);
    builder.build()
}
