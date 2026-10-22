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
//!   with every value the metadata reads: IFD0's Make and Model, and an
//!   Exif IFD holding each of [`EXIF_IFD_TAGS`], with values made up in the
//!   shape a Nikon Z 9 records them; [`camera_jpeg_with_metadata`] is the
//!   camera's JPEG with that structure in its Exif segment.
//! - [`padded_jpeg`]: the main image, then 4,096 zeros after its EOI, as
//!   some Sony bodies write their JPEG files.
//!
//! The TIFF structure of an Exif segment and the MPF index are each in the
//! byte order a test gives.

use std::ops::Range;

use super::jpeg_builder::{Built, JpegBuilder};
use super::jpeg_stub::jpeg_stub;
use super::tiff_builder::{IfdId, Order, TiffBuilder};

/// The camera's maker, in IFD0 (Make).
pub const MAKE: u16 = 0x010F;

/// The camera's model, in IFD0 (Model).
pub const MODEL: u16 = 0x0110;

/// The Orientation tag, in IFD0.
const ORIENTATION: u16 = 0x0112;

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
pub const EXIF_IFD_TAGS: [u16; 13] = [
    BODY_SERIAL_NUMBER,
    DATE_TIME_ORIGINAL,
    SUB_SEC_TIME_ORIGINAL,
    OFFSET_TIME_ORIGINAL,
    EXPOSURE_TIME,
    F_NUMBER,
    PHOTOGRAPHIC_SENSITIVITY,
    EXPOSURE_BIAS_VALUE,
    FOCAL_LENGTH,
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
    let mut tiff = TiffBuilder::new(order);
    let (ifd0, ifd1) = (tiff.ifd(), tiff.ifd());
    tiff.short(ifd0, ORIENTATION, &[orientation])
        .next(ifd0, Some(ifd1));
    tags(&mut tiff, ifd0);
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
    let mut builder = JpegBuilder::new();
    if jfif {
        builder.jfif();
    }
    builder.exif(tiff).mpf(order).frame(MAIN.0, MAIN.1, 0xC0);
    if !outside.is_empty() {
        builder.after_eoi(outside);
    }
    for (width, height) in MPF_IMAGES {
        builder.mpf_image(width, height, &[], 512);
    }
    builder.build()
}

/// The TIFF structure of a camera's Exif segment, in byte order `order`,
/// as [`exif`] writes it with Orientation 6, and with every value the
/// metadata reads: IFD0's Make and Model, [`CAMERA_MAKE`] and
/// [`CAMERA_MODEL`], and an Exif IFD holding each of [`EXIF_IFD_TAGS`],
/// with the values above, each of the type the Exif standard gives it.
/// Then `change` adds what a test needs, given the builder, IFD0 and the
/// Exif IFD. Returns the bytes and where the builder put the thumbnail in
/// them.
pub fn camera_exif(
    order: Order,
    change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
) -> (Vec<u8>, Range<u64>) {
    let [date_time, subseconds, offset] = CAPTURE_TIME;
    let [lens_make, lens_model, lens_serial_number] = LENS;
    exif_with(order, ROTATE_90, None, |tiff, ifd0| {
        let exif = tiff.ifd();
        tiff.ascii(ifd0, MAKE, CAMERA_MAKE)
            .ascii(ifd0, MODEL, CAMERA_MODEL)
            .exif(ifd0, exif);
        tiff.ascii(exif, BODY_SERIAL_NUMBER, CAMERA_SERIAL_NUMBER)
            .ascii(exif, DATE_TIME_ORIGINAL, date_time)
            .ascii(exif, SUB_SEC_TIME_ORIGINAL, subseconds)
            .ascii(exif, OFFSET_TIME_ORIGINAL, offset)
            .rational(exif, EXPOSURE_TIME, &[EXPOSURE_SECONDS])
            .rational(exif, F_NUMBER, &[APERTURE])
            .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[ISO])
            .srational(exif, EXPOSURE_BIAS_VALUE, &[EXPOSURE_BIAS])
            .rational(exif, FOCAL_LENGTH, &[FOCAL_MILLIMETRES])
            .ascii(exif, LENS_MAKE, lens_make)
            .ascii(exif, LENS_MODEL, lens_model)
            .ascii(exif, LENS_SERIAL_NUMBER, lens_serial_number)
            .rational(exif, LENS_SPECIFICATION, &LENS_RANGE);
        change(tiff, ifd0, exif);
    })
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

/// A JPEG with no Exif segment whose main image, of [`MAIN`], is followed
/// by 4,096 zeros after its EOI, as some Sony bodies write it.
pub fn padded_jpeg() -> Built {
    let mut builder = JpegBuilder::new();
    builder.frame(MAIN.0, MAIN.1, 0xC0).pad(4096);
    builder.build()
}
