// SPDX-License-Identifier: MIT OR Apache-2.0

//! A file laid out like the NEF files of a Nikon Z 9, with stub JPEGs in
//! place of the real ones, so that the tests check the NEF reader on the
//! real layout without a real file.
//!
//! The layout, which [`z9_file`] writes:
//!
//! - IFD0 is a reduced-resolution image (NewSubfileType 1) whose Make is
//!   `NIKON CORPORATION` and Model `NIKON Z 9`, with the Orientation
//!   given. It declares a small JPEG, 160 by 120 pixels, through its
//!   JPEGInterchangeFormat and JPEGInterchangeFormatLength tags, with
//!   Compression 6 (old-style JPEG); it points at six SubIFDs, in the
//!   order below, and at the Exif IFD.
//! - SubIFD0 is a reduced-resolution image that declares the full-size
//!   JPEG, 8256 by 5504 pixels, the same way.
//! - SubIFD1 is the raw data: the full-resolution image (NewSubfileType 0),
//!   8280 by 5520 pixels, of 14-bit samples of a colour filter array
//!   (PhotometricInterpretation 32803), with Nikon's Compression 34713, in
//!   one strip. The strip is 64 bytes that start with `FF 10 FF 50`, the
//!   start-of-codestream marker of JPEG XS, which is how the raw strips of
//!   the Z 9's HE and HE* modes begin; the bytes stand for the raw data.
//! - SubIFD2 is a reduced-resolution image that declares the medium JPEG,
//!   1620 by 1080 pixels, as IFD0 does.
//! - SubIFD3 is an uncompressed reduced-resolution grey image, 384 by 256
//!   pixels of 8-bit samples, in one strip of 64 zero bytes that stand for
//!   it.
//! - SubIFD4 and SubIFD5 are reduced-resolution images with no pixels:
//!   their width, height, strip offset and strip length are all 0. The
//!   first has Compression 34713, the second none (1).
//! - The Exif IFD holds only the ExifVersion, `0232`.
//!
//! Every stub JPEG is a baseline one. The tags that the shared builder has
//! no constant for are named here.

use super::jpeg::jpeg_stub;
use super::tiff_builder::{BlobId, Built, IfdId, Order, TiffBuilder};

/// Whether an IFD's image is the full-resolution one, 0, or a
/// reduced-resolution copy, 1 (NewSubfileType).
const NEW_SUBFILE_TYPE: u16 = 0x00FE;
/// The image's width in pixels, as the tags declare it (ImageWidth).
const IMAGE_WIDTH: u16 = 0x0100;
/// The image's height in pixels, as the tags declare it (ImageLength).
const IMAGE_LENGTH: u16 = 0x0101;
/// The bits of each sample (BitsPerSample).
const BITS_PER_SAMPLE: u16 = 0x0102;
/// How the image's data is compressed (Compression).
const COMPRESSION: u16 = 0x0103;
/// What the samples mean (PhotometricInterpretation).
const PHOTOMETRIC_INTERPRETATION: u16 = 0x0106;
/// The camera's manufacturer, as ASCII text (Make).
const MAKE: u16 = 0x010F;
/// The camera's model, as ASCII text (Model).
const MODEL: u16 = 0x0110;
/// Where each strip of the image's data starts (StripOffsets).
const STRIP_OFFSETS: u16 = 0x0111;
/// What a viewer must do to show the image upright (Orientation).
const ORIENTATION: u16 = 0x0112;
/// The number of samples per pixel (SamplesPerPixel).
const SAMPLES_PER_PIXEL: u16 = 0x0115;
/// How many bytes each strip takes (StripByteCounts).
const STRIP_BYTE_COUNTS: u16 = 0x0117;
/// Where an embedded JPEG stream starts (JPEGInterchangeFormat).
const JPEG_INTERCHANGE_FORMAT: u16 = 0x0201;
/// How many bytes the embedded JPEG stream takes
/// (JPEGInterchangeFormatLength).
const JPEG_INTERCHANGE_FORMAT_LENGTH: u16 = 0x0202;
/// The version of the Exif standard the file follows (ExifVersion).
const EXIF_VERSION: u16 = 0x9000;

/// NewSubfileType of the full-resolution image.
const FULL_RESOLUTION: u32 = 0;
/// NewSubfileType of a reduced-resolution image.
const REDUCED_RESOLUTION: u32 = 1;
/// Compression of no compression.
const UNCOMPRESSED: u16 = 1;
/// Compression of JPEG as TIFF 6.0 first defined it, old-style JPEG.
const OLD_STYLE_JPEG: u16 = 6;
/// Compression of Nikon's own raw data.
const NIKON_RAW: u16 = 34713;
/// PhotometricInterpretation of black is zero, a grey image.
const BLACK_IS_ZERO: u16 = 1;
/// PhotometricInterpretation of a colour filter array, raw sensor data.
const CFA: u16 = 32803;
/// The baseline frame header's marker, SOF0.
const BASELINE: u8 = 0xC0;

/// The size in pixels of each JPEG of the file: IFD0's small one, SubIFD0's
/// full-size one and SubIFD2's medium one.
pub const IFD0_JPEG: (u16, u16) = (160, 120);
/// See [`IFD0_JPEG`].
pub const FULL_SIZE_JPEG: (u16, u16) = (8256, 5504);
/// See [`IFD0_JPEG`].
pub const MEDIUM_JPEG: (u16, u16) = (1620, 1080);

/// The start of the raw strip: the start-of-codestream marker of JPEG XS.
pub const RAW_STRIP_START: [u8; 4] = [0xFF, 0x10, 0xFF, 0x50];

/// The stub JPEG of `size`, as the file holds it.
pub fn stub((width, height): (u16, u16)) -> Vec<u8> {
    jpeg_stub(width, height, BASELINE)
}

/// Where the blobs of a Z 9 file are, as [`z9_file_with_blobs`] laid them
/// out.
#[derive(Clone, Copy, Debug)]
pub struct Z9Blobs {
    /// IFD0's small JPEG.
    pub ifd0_jpeg: BlobId,
    /// SubIFD0's full-size JPEG.
    pub full_size_jpeg: BlobId,
    /// SubIFD1's raw strip.
    pub raw: BlobId,
    /// SubIFD2's medium JPEG.
    pub medium_jpeg: BlobId,
    /// SubIFD3's grey strip.
    pub grey: BlobId,
}

/// The Z 9 layout, in byte order `order`, with IFD0's Orientation
/// `orientation` (an EXIF value, 1 to 8).
pub fn z9_file(order: Order, orientation: u16) -> Built {
    z9_file_with_blobs(order, orientation).0
}

/// As [`z9_file`], with where its blobs are, so that a test can find them
/// with [`Built::blob_offset`].
pub fn z9_file_with_blobs(order: Order, orientation: u16) -> (Built, Z9Blobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let subs: Vec<IfdId> = (0..6).map(|_| builder.ifd()).collect();
    let exif = builder.ifd();

    builder
        .long(ifd0, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .ascii(ifd0, MAKE, "NIKON CORPORATION")
        .ascii(ifd0, MODEL, "NIKON Z 9")
        .short(ifd0, ORIENTATION, &[orientation])
        .short(ifd0, COMPRESSION, &[OLD_STYLE_JPEG]);
    let ifd0_jpeg = jpeg(&mut builder, ifd0, IFD0_JPEG);
    builder.subifds(ifd0, &subs).exif(ifd0, exif);

    builder
        .long(subs[0], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .short(subs[0], COMPRESSION, &[OLD_STYLE_JPEG]);
    let full_size_jpeg = jpeg(&mut builder, subs[0], FULL_SIZE_JPEG);

    let mut raw_strip = RAW_STRIP_START.to_vec();
    raw_strip.resize(64, 0);
    builder
        .long(subs[1], NEW_SUBFILE_TYPE, &[FULL_RESOLUTION])
        .long(subs[1], IMAGE_WIDTH, &[8280])
        .long(subs[1], IMAGE_LENGTH, &[5520])
        .short(subs[1], BITS_PER_SAMPLE, &[14])
        .short(subs[1], COMPRESSION, &[NIKON_RAW])
        .short(subs[1], PHOTOMETRIC_INTERPRETATION, &[CFA])
        .short(subs[1], SAMPLES_PER_PIXEL, &[1]);
    let raw = strip(&mut builder, subs[1], &raw_strip);

    builder
        .long(subs[2], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .short(subs[2], COMPRESSION, &[OLD_STYLE_JPEG]);
    let medium_jpeg = jpeg(&mut builder, subs[2], MEDIUM_JPEG);

    builder
        .long(subs[3], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .long(subs[3], IMAGE_WIDTH, &[384])
        .long(subs[3], IMAGE_LENGTH, &[256])
        .short(subs[3], BITS_PER_SAMPLE, &[8])
        .short(subs[3], COMPRESSION, &[UNCOMPRESSED])
        .short(subs[3], PHOTOMETRIC_INTERPRETATION, &[BLACK_IS_ZERO])
        .short(subs[3], SAMPLES_PER_PIXEL, &[1]);
    let grey = strip(&mut builder, subs[3], &[0; 64]);

    for (sub, compression) in [(subs[4], NIKON_RAW), (subs[5], UNCOMPRESSED)] {
        builder
            .long(sub, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
            .long(sub, IMAGE_WIDTH, &[0])
            .long(sub, IMAGE_LENGTH, &[0])
            .short(sub, COMPRESSION, &[compression])
            .long(sub, STRIP_OFFSETS, &[0])
            .long(sub, STRIP_BYTE_COUNTS, &[0]);
    }

    builder.undefined(exif, EXIF_VERSION, b"0232");

    let blobs = Z9Blobs {
        ifd0_jpeg,
        full_size_jpeg,
        raw,
        medium_jpeg,
        grey,
    };
    (builder.build(), blobs)
}

/// Adds the stub JPEG of `size` to the file, and points `ifd`'s
/// JPEGInterchangeFormat and JPEGInterchangeFormatLength at it.
fn jpeg(builder: &mut TiffBuilder, ifd: IfdId, size: (u16, u16)) -> BlobId {
    let blob = builder.blob(&stub(size));
    builder
        .long_blob(ifd, JPEG_INTERCHANGE_FORMAT, blob)
        .long_blob_len(ifd, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);
    blob
}

/// Adds `bytes` to the file, and makes them `ifd`'s one strip.
fn strip(builder: &mut TiffBuilder, ifd: IfdId, bytes: &[u8]) -> BlobId {
    let blob = builder.blob(bytes);
    builder
        .long_blob(ifd, STRIP_OFFSETS, blob)
        .long_blob_len(ifd, STRIP_BYTE_COUNTS, blob);
    blob
}
