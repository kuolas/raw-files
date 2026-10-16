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
//!   one strip. Its ImageWidth and ImageLength are LONGs, and it holds no
//!   default crop, as on the public Z 9 sample 5147. The strip is 64 bytes
//!   that start with `FF 10 FF 50`, the start-of-codestream marker of JPEG
//!   XS, which is how the raw strips of the Z 9's HE and HE* modes begin;
//!   the bytes stand for the raw data.
//! - SubIFD2 is a reduced-resolution image that declares the medium JPEG,
//!   1620 by 1080 pixels, as IFD0 does.
//! - SubIFD3 is an uncompressed reduced-resolution grey image, 384 by 256
//!   pixels of 8-bit samples, in one strip of 64 zero bytes that stand for
//!   it.
//! - SubIFD4 and SubIFD5 are reduced-resolution images with no pixels:
//!   their width, height, strip offset and strip length are all 0. The
//!   first has Compression 34713, the second none (1).
//! - The Exif IFD holds the ExifVersion, `0232`; the capture time, as
//!   DateTimeOriginal `2022:02:05 13:46:41`, SubSecTimeOriginal `85` and
//!   OffsetTimeOriginal `-05:00`; the exposure, as ExposureTime `10/3200`,
//!   FNumber `900/100` and FocalLength `180/10` (RATIONAL), ExposureBiasValue
//!   `0/6` (SRATIONAL), PhotographicSensitivity 64 and SensitivityType 2
//!   (SHORT), and RecommendedExposureIndex 64 (LONG); these are the values
//!   of the public Z 9 sample 5147 of raw.pixls.us, each of the type it
//!   has there. It holds the lens of that sample too: LensMake `NIKON`;
//!   LensModel `NIKKOR Z 14-30mm f/4 S`, padded with NULs to 65 bytes as
//!   the Z 9 stores it; and LensSpecification `140/10`, `300/10`,
//!   `400/100`, `400/100`. The BodySerialNumber `1234567` and the
//!   LensSerialNumber `12345678` are made up, since no real serial number
//!   is copied here; the lens's has the two spaces and the NUL after its
//!   eight digits that the Z 9 writes, 11 bytes in all. Last, the Exif IFD
//!   holds the MakerNote, whose value is the block [`nikon_makernote`]
//!   writes: Nikon's own metadata, here only its version, its crop area
//!   and its PreviewIFD. The crop area is the visible image, the raw data
//!   inside its border: 8256 by 5504 pixels starting 12 columns and 8 rows
//!   into the raw, the CropArea `[12, 8, 8256, 5504]` of the public Z 9
//!   samples 5146 to 5148, as four SHORTs. The PreviewIFD declares a small
//!   JPEG of 640 by 424 pixels. The MakerNote is in the file's byte order
//!   unless the file is built with [`z9_file_with`], which may give it the
//!   other.
//!
//! Every text value is ASCII ending in one NUL, with no padding, as the
//! Z 9 writes it, but for the lens's model and serial number, padded as
//! above.
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
/// How long the shutter was open, in seconds, as a RATIONAL
/// (ExposureTime).
const EXPOSURE_TIME: u16 = 0x829A;
/// The f-number, as a RATIONAL (FNumber).
const F_NUMBER: u16 = 0x829D;
/// The ISO, as a SHORT (PhotographicSensitivity).
const PHOTOGRAPHIC_SENSITIVITY: u16 = 0x8827;
/// Which of the Exif sensitivities the camera records, as a SHORT
/// (SensitivityType): 2, the RecommendedExposureIndex.
const SENSITIVITY_TYPE: u16 = 0x8830;
/// The recommended exposure index, as a LONG (RecommendedExposureIndex).
const RECOMMENDED_EXPOSURE_INDEX: u16 = 0x8832;
/// The version of the Exif standard the file follows (ExifVersion).
const EXIF_VERSION: u16 = 0x9000;
/// When the picture was taken, as ASCII text (DateTimeOriginal).
const DATE_TIME_ORIGINAL: u16 = 0x9003;
/// The capture time's offset from UTC, as ASCII text
/// (OffsetTimeOriginal).
const OFFSET_TIME_ORIGINAL: u16 = 0x9011;
/// The exposure compensation in EV, as an SRATIONAL (ExposureBiasValue).
const EXPOSURE_BIAS_VALUE: u16 = 0x9204;
/// The focal length in millimetres, as a RATIONAL (FocalLength).
const FOCAL_LENGTH: u16 = 0x920A;
/// The capture time's fraction of a second, as ASCII digits
/// (SubSecTimeOriginal).
const SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;
/// The camera maker's own metadata block (MakerNote).
const MAKER_NOTE: u16 = 0x927C;
/// The camera body's serial number, as ASCII text (BodySerialNumber).
const BODY_SERIAL_NUMBER: u16 = 0xA431;
/// The lens's focal and aperture range, as four RATIONALs
/// (LensSpecification).
const LENS_SPECIFICATION: u16 = 0xA432;
/// The lens's manufacturer, as ASCII text (LensMake).
const LENS_MAKE: u16 = 0xA433;
/// The lens's model name, as ASCII text (LensModel).
const LENS_MODEL: u16 = 0xA434;
/// The lens's serial number, as ASCII text (LensSerialNumber).
const LENS_SERIAL_NUMBER: u16 = 0xA435;
/// The version of the Nikon MakerNote's layout, as four ASCII digits in an
/// UNDEFINED value (MakerNoteVersion, a tag of the MakerNote IFD).
const MAKER_NOTE_VERSION: u16 = 0x0001;
/// The offset of the MakerNote's PreviewIFD, the IFD that declares its
/// small JPEG (PreviewIFD, a tag of the MakerNote IFD).
const PREVIEW_IFD: u16 = 0x0011;
/// The visible image's rectangle in the raw data, as four SHORTs: its left
/// column, top row, width and height (CropArea, a tag of the MakerNote
/// IFD).
const CROP_AREA: u16 = 0x0045;
/// The Z 9's CropArea, as the public samples 5146 to 5148 record it.
const CROP_AREA_Z9: [u16; 4] = [12, 8, 8256, 5504];

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

/// The start of a Nikon MakerNote of the layout with an embedded TIFF
/// header: the signature `Nikon` and a NUL, the version `02 11`, which
/// the Z 9 writes, and two zero bytes. The inner TIFF header follows.
const MAKER_NOTE_START: [u8; 10] = *b"Nikon\0\x02\x11\0\0";
/// The MakerNote's version as its MakerNoteVersion tag gives it.
const MAKER_NOTE_VERSION_0211: &[u8; 4] = b"0211";

/// The size in pixels of each JPEG of the file: IFD0's small one, SubIFD0's
/// full-size one and SubIFD2's medium one.
pub const IFD0_JPEG: (u16, u16) = (160, 120);
/// See [`IFD0_JPEG`].
pub const FULL_SIZE_JPEG: (u16, u16) = (8256, 5504);
/// See [`IFD0_JPEG`].
pub const MEDIUM_JPEG: (u16, u16) = (1620, 1080);
/// The size in pixels of the MakerNote's small JPEG, the one its
/// PreviewIFD declares.
pub const PREVIEW_JPEG: (u16, u16) = (640, 424);

/// The model name of the Z 9 file's lens, as its LensModel records it.
pub const LENS_MODEL_Z14_30: &str = "NIKKOR Z 14-30mm f/4 S";

/// How many bytes the Z 9 stores its LensModel in, with the NUL that ends
/// it and the NULs that pad it: 65 on the public sample 5147.
const LENS_MODEL_COUNT: usize = 65;

/// `model` padded with NULs as the Z 9 stores a LensModel, less the NUL
/// the builder adds, so that the value is [`LENS_MODEL_COUNT`] bytes.
fn padded_lens_model(model: &str) -> String {
    let mut padded = model.to_owned();
    padded.extend(std::iter::repeat_n(
        '\0',
        LENS_MODEL_COUNT - 1 - model.len(),
    ));
    padded
}

/// The start of the raw strip: the start-of-codestream marker of JPEG XS.
pub const RAW_STRIP_START: [u8; 4] = [0xFF, 0x10, 0xFF, 0x50];

/// The stub JPEG of `size`, as the file holds it.
pub fn stub((width, height): (u16, u16)) -> Vec<u8> {
    jpeg_stub(width, height, BASELINE)
}

/// Where the blobs of a Z 9 file are, as [`z9_file_with`] laid them
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
    /// The Exif IFD's MakerNote block.
    pub makernote: BlobId,
    /// Where the MakerNote's small JPEG starts, counted from the start of
    /// the MakerNote block: the JPEG is at `blob_offset(makernote)` plus
    /// this.
    pub preview_in_makernote: u64,
}

/// A Nikon MakerNote of the layout every body since the D70 writes, in byte
/// order `order`, whose PreviewIFD declares `preview`; and where `preview`
/// starts, counted from the start of the block.
///
/// The block is [`MAKER_NOTE_START`] and then a TIFF structure of its own:
/// a TIFF header in `order`, then the MakerNote IFD, which holds the
/// MakerNoteVersion `0211`, the PreviewIFD tag and the Z 9's CropArea, whose
/// four SHORTs follow the IFD, then the PreviewIFD,
/// which declares `preview` through JPEGInterchangeFormat and
/// JPEGInterchangeFormatLength, then `preview`'s bytes. Every offset
/// inside the block counts from the inner TIFF header, 10 bytes in, which
/// is exactly how a second [`TiffBuilder`] counts them: from its own first
/// byte, which is that header.
pub fn nikon_makernote(order: Order, preview: &[u8]) -> (Vec<u8>, u64) {
    let mut inner = TiffBuilder::new(order);
    let makernote = inner.ifd();
    let preview_ifd = inner.ifd();
    inner
        .undefined(makernote, MAKER_NOTE_VERSION, MAKER_NOTE_VERSION_0211)
        .ifd_pointer(makernote, PREVIEW_IFD, preview_ifd)
        .short(makernote, CROP_AREA, &CROP_AREA_Z9);
    let jpeg = inner.blob(preview);
    inner
        .long_blob(preview_ifd, JPEG_INTERCHANGE_FORMAT, jpeg)
        .long_blob_len(preview_ifd, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg);
    let inner = inner.build();

    let mut block = MAKER_NOTE_START.to_vec();
    block.extend_from_slice(&inner.bytes);
    let header_at = u64::try_from(MAKER_NOTE_START.len()).expect("ten bytes");
    (block, header_at + inner.blob_offset(jpeg))
}

/// The Z 9 layout, in byte order `order`, with IFD0's Orientation
/// `orientation` (an EXIF value, 1 to 8).
pub fn z9_file(order: Order, orientation: u16) -> Built {
    z9_file_with_blobs(order, orientation).0
}

/// As [`z9_file`], with where its blobs are, so that a test can find them
/// with [`Built::blob_offset`].
pub fn z9_file_with_blobs(order: Order, orientation: u16) -> (Built, Z9Blobs) {
    z9_file_with(order, orientation, order)
}

/// As [`z9_file_with_blobs`], with the MakerNote in byte order
/// `makernote_order`, which may differ from the file's: a MakerNote's
/// numbers follow its own TIFF header.
pub fn z9_file_with(order: Order, orientation: u16, makernote_order: Order) -> (Built, Z9Blobs) {
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

    builder
        .undefined(exif, EXIF_VERSION, b"0232")
        .ascii(exif, DATE_TIME_ORIGINAL, "2022:02:05 13:46:41")
        .ascii(exif, SUB_SEC_TIME_ORIGINAL, "85")
        .ascii(exif, OFFSET_TIME_ORIGINAL, "-05:00")
        .rational(exif, EXPOSURE_TIME, &[(10, 3200)])
        .rational(exif, F_NUMBER, &[(900, 100)])
        .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[64])
        .short(exif, SENSITIVITY_TYPE, &[2])
        .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[64])
        .srational(exif, EXPOSURE_BIAS_VALUE, &[(0, 6)])
        .rational(exif, FOCAL_LENGTH, &[(180, 10)])
        .ascii(exif, BODY_SERIAL_NUMBER, "1234567")
        .rational(
            exif,
            LENS_SPECIFICATION,
            &[(140, 10), (300, 10), (400, 100), (400, 100)],
        )
        .ascii(exif, LENS_MAKE, "NIKON")
        .ascii(exif, LENS_MODEL, &padded_lens_model(LENS_MODEL_Z14_30))
        .ascii(exif, LENS_SERIAL_NUMBER, "12345678  ");
    let (block, preview_in_makernote) = nikon_makernote(makernote_order, &stub(PREVIEW_JPEG));
    let makernote = builder.blob(&block);
    builder.undefined_blob(exif, MAKER_NOTE, makernote);

    let blobs = Z9Blobs {
        ifd0_jpeg,
        full_size_jpeg,
        raw,
        medium_jpeg,
        grey,
        makernote,
        preview_in_makernote,
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
