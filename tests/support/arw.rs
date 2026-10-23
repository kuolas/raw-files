// SPDX-License-Identifier: MIT OR Apache-2.0

//! Files laid out like each generation of Sony's RAW family, with stub
//! JPEGs in place of the real ones, so that the tests check the ARW reader
//! on every real layout without a real file.
//!
//! Every Sony RAW file is a TIFF file whose previews sit in the IFDs of
//! IFD0's chain, and whose raw data sits in IFD0's one SubIFD. What changes
//! from one generation to the next is which IFD holds which JPEG, and what
//! the raw data looks like. The layouts, as the ARW format notes record
//! them (`docs/formats/arw/`, sections 1.3, 1.4 and 5):
//!
//! - [`arw4_file`] is a file of the bodies that write version 4.0 or later
//!   of Sony's file format, an ILCE-7M4's. IFD0 is a reduced-resolution
//!   image (NewSubfileType 1) whose Make is `SONY` and Model `ILCE-7M4`,
//!   with the Orientation given; it declares a JPEG of 1616 by 1080 pixels
//!   through its JPEGInterchangeFormat and JPEGInterchangeFormatLength tags,
//!   with Compression 6 (old-style JPEG), and points at the raw SubIFD and
//!   at an Exif IFD that holds the ExifVersion, `0232`; the capture time,
//!   as DateTimeOriginal `2023:10:19 16:13:48`, SubSecTimeOriginal `776`
//!   and OffsetTimeOriginal `+02:00`; and the exposure, as ExposureTime
//!   `1/50`, FNumber `56/10` and FocalLength `500/10` (RATIONAL),
//!   ExposureBiasValue `0/10` (SRATIONAL), PhotographicSensitivity 250
//!   (SHORT) and RecommendedExposureIndex 250 (LONG); and the lens, as
//!   LensModel `FE 50mm F2.5 G` and LensSpecification `500/10`, `500/10`,
//!   `25/10`, `25/10`; and Sony's MakerNote, which records the focus point:
//!   a directory of Sony's tags at the start of its block, without a
//!   header, with its values inside the block, holding FocusMode 3 (AF-C),
//!   one BYTE; FocusLocation, four SHORTs, a frame of 7008 by 4672 pixels
//!   and a centre at (3832, 2102); FocusFrameSize, six UNDEFINED bytes that
//!   hold the area's 350 by 351 pixels and a third number, 257, as 16-bit
//!   numbers in the file's byte order; FileFormat, the four BYTEs 4, 0, 1,
//!   0; and SonyModelID 388, a SHORT. These are the values of the public
//!   ILCE-7M4 sample 6929 of raw.pixls.us, each of the type it has there;
//!   the sample has no serial number, of the body or of the lens, and no
//!   LensMake in its Exif tags, so neither has the fixture. IFD1, IFD0's
//!   next, repeats the Make, Model and Orientation and declares a
//!   thumbnail of 160 by 120 pixels the same way. IFD2, IFD1's next,
//!   declares a JPEG of the picture's full size, 7008 by 4672 pixels, and
//!   says so in its tags: Compression 7 (JPEG as TIFF Technical Note 2
//!   defines it), YCbCr samples (PhotometricInterpretation 6) of 8 bits
//!   each, three per pixel, subsampled 2 by 1. The raw SubIFD is a
//!   lossless raw: the full-resolution image (NewSubfileType 0) of 14-bit
//!   samples of a colour filter array (PhotometricInterpretation 32803),
//!   with Compression 7. Its ImageWidth and ImageLength, SHORTs, say 7168 by
//!   5120 pixels, and its default crop, LONGs, starts at DefaultCropOrigin
//!   12, 8 and is DefaultCropSize 7008 by 4672 pixels, the size of IFD2's
//!   JPEG: the values of the sample 6929, whose raw size is a whole number
//!   of its 512-pixel tiles. The fixture's raw data is two tiles of 512 by
//!   512 pixels that stand for them all, each a lossless JPEG stream,
//!   starting with the start-of-image marker and a lossless frame header.
//!   Like the real files, it also lists one strip that starts at the first
//!   tile and is as long as the tiles together.
//! - [`arw4_medium_file`] is the same file, with the same metadata, shot as
//!   a medium raw. Its MakerNote is the lossless file's, a simplification:
//!   the medium raw sample 6930 records the same frame and area, the
//!   full-size ones, with its centre elsewhere. IFD2's JPEG is 4608 by
//!   3072 pixels, and the raw SubIFD holds YCbCr samples
//!   (PhotometricInterpretation 6, the same as IFD2's) of 15 bits each,
//!   three per pixel, in the same lossless tiles. Its
//!   size, SHORTs, and default crop, LONGs, are those of the public
//!   ILCE-7M4 medium raw sample 6930: 5120 by 3584 pixels, cropped from 8,
//!   4 to 4608 by 3072.
//! - [`arw2_file`] is a file of an ARW 2 body, an SLT-A99V's: IFD0 and IFD1
//!   as above, with that Model, and no IFD2. The raw SubIFD is the
//!   full-resolution image, 6024 by 4024 pixels, of 12-bit colour filter
//!   array samples, with Sony's compressed raw (Compression 32767), in one
//!   strip of 64 bytes that are not a JPEG stream and stand for the raw
//!   data.
//! - [`arq_file`] is a pixel-shift composite that Sony's software made from
//!   the frames of an ILCE-7RM3: an ARW 2 file whose raw SubIFD holds
//!   linear raw data (PhotometricInterpretation 32892), every colour at
//!   every pixel, four 16-bit samples per pixel, uncompressed.
//! - [`sr2_file`] is a file of the DSC-R1: IFD0 declares a JPEG of 640 by
//!   424 pixels and IFD1 the thumbnail, both reduced-resolution images
//!   with Compression 6; the raw SubIFD is 14-bit colour filter array
//!   samples, uncompressed, in one strip of 64 bytes.
//! - [`srf_file`] is a file of the DSC-F828, whose IFD0 describes the raw
//!   data rather than an image a reader can use: 3360 by 2460 pixels of
//!   RGB samples (PhotometricInterpretation 2) of 14 bits each, three per
//!   pixel, uncompressed, with a StripByteCounts of 16,531,200 bytes and no
//!   StripOffsets, because the raw data, encrypted, sits at a fixed offset
//!   that the reader must know. IFD1 declares the file's only JPEG, 640 by
//!   480 pixels, with Compression 6. Neither IFD has a NewSubfileType, and
//!   there are no SubIFDs.
//! - [`a100_file`] is a file of the DSLR-A100, Sony's first ARW, version
//!   1.0. Its Make is `SONY` padded with eleven spaces. IFD0 declares a
//!   JPEG of 640 by 480 pixels, but its ImageWidth and ImageLength say 3872
//!   by 2592, the raw data's size; and its SubIFDs tag holds not the
//!   offset of an IFD but that of the raw data, whose bytes do not hold
//!   together as one (see [`a100_file`] for which rule of the walk
//!   rejects them). IFD1 declares the thumbnail.
//!
//! The metadata: every file holds its Make and Model in IFD0, and every
//! file but the SRF holds them again in IFD1, with the Orientation in both;
//! the SRF has no Orientation. Only the two ARW 4 files hold a capture
//! time, an exposure, a lens and a MakerNote, which records the focus
//! point, and no file holds a serial number. Every file but the A100 has
//! an IFD that describes its raw data and gives its size; only the two
//! ARW 4 files have a default crop. Every text value is ASCII ending in
//! one NUL.
//!
//! Every JPEG preview is a baseline stub. Each builder returns where it put
//! the JPEGs and the raw data, as an [`ArwBlobs`], so that a test can check
//! that no preview starts at the raw data. The tags that the shared builder
//! has no constant for are named here.

use super::jpeg_stub::jpeg_stub;
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
/// The width of each tile in pixels (TileWidth).
const TILE_WIDTH: u16 = 0x0142;
/// The height of each tile in pixels (TileLength).
const TILE_LENGTH: u16 = 0x0143;
/// Where each tile of the image's data starts (TileOffsets).
const TILE_OFFSETS: u16 = 0x0144;
/// How many bytes each tile takes (TileByteCounts).
const TILE_BYTE_COUNTS: u16 = 0x0145;
/// Where an embedded JPEG stream starts (JPEGInterchangeFormat).
const JPEG_INTERCHANGE_FORMAT: u16 = 0x0201;
/// How many bytes the embedded JPEG stream takes
/// (JPEGInterchangeFormatLength).
const JPEG_INTERCHANGE_FORMAT_LENGTH: u16 = 0x0202;
/// How much the chroma samples of a YCbCr image are subsampled,
/// horizontally then vertically (YCbCrSubSampling).
const YCBCR_SUB_SAMPLING: u16 = 0x0212;
/// How long the shutter was open, in seconds, as a RATIONAL
/// (ExposureTime).
const EXPOSURE_TIME: u16 = 0x829A;
/// The f-number, as a RATIONAL (FNumber).
const F_NUMBER: u16 = 0x829D;
/// The ISO, as a SHORT (PhotographicSensitivity).
const PHOTOGRAPHIC_SENSITIVITY: u16 = 0x8827;
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
/// The lens's focal and aperture range, as four RATIONALs
/// (LensSpecification).
const LENS_SPECIFICATION: u16 = 0xA432;
/// The lens's model name, as ASCII text (LensModel).
const LENS_MODEL: u16 = 0xA434;
/// Where the visible image starts in the raw data, the column then the row
/// (DefaultCropOrigin).
const DEFAULT_CROP_ORIGIN: u16 = 0xC61F;
/// The size of the visible image in the raw data, the width then the
/// height (DefaultCropSize).
const DEFAULT_CROP_SIZE: u16 = 0xC620;
/// The capture time's fraction of a second, as ASCII digits
/// (SubSecTimeOriginal).
const SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;
/// The maker's own block of metadata, as UNDEFINED bytes (MakerNote).
const MAKER_NOTE: u16 = 0x927C;
/// The Sony MakerNote tag that records how the camera focused, one BYTE
/// (FocusMode).
pub const FOCUS_MODE: u16 = 0x201B;
/// The Sony MakerNote tag that records where the camera focused, a frame's
/// width and height and the centre's x and y, four SHORTs (FocusLocation).
pub const FOCUS_LOCATION: u16 = 0x2027;
/// The Sony MakerNote tag that records the focus area's width and height,
/// then a third number, as six UNDEFINED bytes (FocusFrameSize).
pub const FOCUS_FRAME_SIZE: u16 = 0x2037;
/// The Sony MakerNote tag that records the version of Sony's file format,
/// four BYTEs (FileFormat).
const FILE_FORMAT: u16 = 0xB000;
/// The Sony MakerNote tag that records the camera body by number, a SHORT
/// (SonyModelID).
const SONY_MODEL_ID: u16 = 0xB001;

/// NewSubfileType of the full-resolution image.
const FULL_RESOLUTION: u32 = 0;
/// NewSubfileType of a reduced-resolution image.
const REDUCED_RESOLUTION: u32 = 1;
/// Compression of no compression.
const UNCOMPRESSED: u16 = 1;
/// Compression of JPEG as TIFF 6.0 first defined it, old-style JPEG.
const OLD_STYLE_JPEG: u16 = 6;
/// Compression of JPEG as TIFF Technical Note 2 defines it.
const JPEG: u16 = 7;
/// Compression of Sony's compressed raw data.
const SONY_RAW: u16 = 32767;
/// PhotometricInterpretation of RGB samples.
const RGB: u16 = 2;
/// PhotometricInterpretation of YCbCr samples.
const YCBCR: u16 = 6;
/// PhotometricInterpretation of a colour filter array, raw sensor data.
const CFA: u16 = 32803;
/// PhotometricInterpretation of linear raw data, every colour at every
/// pixel.
const LINEAR_RAW: u16 = 32892;
/// The baseline frame header's marker, SOF0.
const BASELINE: u8 = 0xC0;
/// The lossless frame header's marker, SOF3.
const LOSSLESS: u8 = 0xC3;

/// The size in pixels of IFD0's JPEG on the ARW 2 and ARW 4 files, and on
/// the ARQ file.
pub const IFD0_JPEG: (u16, u16) = (1616, 1080);
/// The size in pixels of IFD1's thumbnail, on every file but the SRF.
pub const THUMBNAIL: (u16, u16) = (160, 120);
/// The size in pixels of IFD2's full-size JPEG on the ARW 4 file.
pub const ARW4_FULL_SIZE_JPEG: (u16, u16) = (7008, 4672);
/// The size of the ARW 4 file's raw data, as its raw SubIFD's ImageWidth
/// and ImageLength declare it, SHORTs: that of the public ILCE-7M4 sample
/// 6929, a whole number of its 512-pixel tiles, so larger than the
/// visible image.
pub const ARW4_RAW_SIZE: (u16, u16) = (7168, 5120);
/// The default crop of the ARW 4 file's raw SubIFD: where the visible
/// image starts, the column then the row, and its width and height. The
/// values of the public ILCE-7M4 sample 6929, which writes them as LONGs;
/// the size is that of IFD2's full-size JPEG.
pub const ARW4_CROP_ORIGIN: [u32; 2] = [12, 8];
/// See [`ARW4_CROP_ORIGIN`].
pub const ARW4_CROP_SIZE: [u32; 2] = [7008, 4672];
/// The size in pixels of IFD2's full-size JPEG on the ARW 4 file with a
/// medium raw.
pub const MEDIUM_FULL_SIZE_JPEG: (u16, u16) = (4608, 3072);
/// The size of the medium raw data, as its raw SubIFD declares it, SHORTs:
/// that of the public ILCE-7M4 sample 6930, a medium raw, tile-padded too.
pub const MEDIUM_RAW_SIZE: (u16, u16) = (5120, 3584);
/// The default crop of the medium raw SubIFD, as [`ARW4_CROP_ORIGIN`] and
/// [`ARW4_CROP_SIZE`] are the full raw's: the values of the public sample
/// 6930, as LONGs; the size is that of IFD2's JPEG.
pub const MEDIUM_CROP_ORIGIN: [u32; 2] = [8, 4];
/// See [`MEDIUM_CROP_ORIGIN`].
pub const MEDIUM_CROP_SIZE: [u32; 2] = [4608, 3072];
/// The frame of the focus point that the ARW 4 files' MakerNote records in
/// its FocusLocation, the width then the height: that of the public
/// ILCE-7M4 sample 6929.
pub const ARW4_FOCUS_FRAME: (u16, u16) = (7008, 4672);
/// The centre of the focus area in [`ARW4_FOCUS_FRAME`], x then y, as the
/// FocusLocation of the sample 6929 records it.
pub const ARW4_FOCUS_CENTER: (u16, u16) = (3832, 2102);
/// The focus area's width and height, as the FocusFrameSize of the sample
/// 6929 records them.
pub const ARW4_FOCUS_SIZE: (u16, u16) = (350, 351);
/// The third number of the FocusFrameSize of the sample 6929, whose
/// meaning is unknown.
const ARW4_FOCUS_FRAME_SIZE_THIRD: u16 = 257;
/// The size in pixels of IFD0's JPEG on the SR2 file.
pub const SR2_JPEG: (u16, u16) = (640, 424);
/// The size in pixels of IFD1's JPEG on the SRF file, its only one.
pub const SRF_JPEG: (u16, u16) = (640, 480);
/// The size in pixels of IFD0's JPEG on the A100 file.
pub const A100_JPEG: (u16, u16) = (640, 480);
/// The size that the A100's IFD0 declares in its ImageWidth and
/// ImageLength: the raw data's, not its JPEG's.
pub const A100_TAG_SIZE: (u16, u16) = (3872, 2592);
/// The size in pixels of each lossless raw tile.
pub const RAW_TILE: (u16, u16) = (512, 512);

/// Where the A100's SubIFDs tag points: the start of its raw data. The
/// notes record this value in one file's dump (and another value in
/// another file's): it is a data offset, not a constant.
pub const A100_RAW_AT: u32 = 0x1_0000;
/// The entry count that the A100's raw data gives when read as an IFD, as
/// the notes record it of a real file: far over the walk's cap of 1000
/// entries.
pub const A100_RAW_AS_COUNT: u16 = 7520;
/// Four bytes that, repeated, fill the A100's raw data after its first
/// two: read as IFD entries, in either byte order, every entry's field
/// type is 0xE2E5 or 0xE5E2, a type no TIFF defines.
pub const A100_RAW_NOISE: [u8; 4] = [0x20, 0x1D, 0xE2, 0xE5];

/// Where the lossless raw tiles of the ARW 4 files start: after everything
/// else the builder places.
const TILES_AT: u64 = 0x1_0000;

/// The baseline stub JPEG of `size`, as the files hold their previews.
pub fn stub((width, height): (u16, u16)) -> Vec<u8> {
    jpeg_stub(width, height, BASELINE)
}

/// The lossless stub JPEG of [`RAW_TILE`], as the ARW 4 files hold each
/// raw tile.
pub fn lossless_tile() -> Vec<u8> {
    jpeg_stub(RAW_TILE.0, RAW_TILE.1, LOSSLESS)
}

/// The A100's raw data, in byte order `order`: an entry count of
/// [`A100_RAW_AS_COUNT`], then [`A100_RAW_NOISE`] over as many bytes as
/// that many entries and a next-IFD pointer take, so that the whole IFD it
/// would be lies inside the file.
pub fn a100_raw(order: Order) -> Vec<u8> {
    let mut bytes = order.u16(A100_RAW_AS_COUNT).to_vec();
    let table = 12 * usize::from(A100_RAW_AS_COUNT) + 4;
    bytes.extend(A100_RAW_NOISE.iter().cycle().take(table));
    bytes
}

/// Where the blobs of a Sony file are, as its builder laid them out.
#[derive(Clone, Debug)]
pub struct ArwBlobs {
    /// IFD0's JPEG; none on the SRF, whose IFD0 is the raw data.
    pub ifd0_jpeg: Option<BlobId>,
    /// IFD1's JPEG: the thumbnail, or the SRF's only JPEG.
    pub ifd1_jpeg: BlobId,
    /// IFD2's full-size JPEG, on the ARW 4 files only.
    pub ifd2_jpeg: Option<BlobId>,
    /// The raw data: each tile, or the one strip, or the A100's raw data;
    /// none on the SRF, whose raw data no tag points at.
    pub raw: Vec<BlobId>,
}

/// The ARW 4 layout with a lossless raw, in byte order `order`, with
/// IFD0's Orientation `orientation` (an EXIF value, 1 to 8).
pub fn arw4_file(order: Order, orientation: u16) -> (Built, ArwBlobs) {
    arw4(order, orientation, ARW4_FULL_SIZE_JPEG, |builder, raw| {
        builder
            .short(raw, IMAGE_WIDTH, &[ARW4_RAW_SIZE.0])
            .short(raw, IMAGE_LENGTH, &[ARW4_RAW_SIZE.1])
            .short(raw, PHOTOMETRIC_INTERPRETATION, &[CFA])
            .short(raw, BITS_PER_SAMPLE, &[14])
            .short(raw, SAMPLES_PER_PIXEL, &[1])
            .long(raw, DEFAULT_CROP_ORIGIN, &ARW4_CROP_ORIGIN)
            .long(raw, DEFAULT_CROP_SIZE, &ARW4_CROP_SIZE);
    })
}

/// The ARW 4 layout with a medium raw, in byte order `order`, with no
/// rotation.
pub fn arw4_medium_file(order: Order) -> (Built, ArwBlobs) {
    arw4(order, 1, MEDIUM_FULL_SIZE_JPEG, |builder, raw| {
        builder
            .short(raw, IMAGE_WIDTH, &[MEDIUM_RAW_SIZE.0])
            .short(raw, IMAGE_LENGTH, &[MEDIUM_RAW_SIZE.1])
            .short(raw, PHOTOMETRIC_INTERPRETATION, &[YCBCR])
            .short(raw, BITS_PER_SAMPLE, &[15, 15, 15])
            .short(raw, SAMPLES_PER_PIXEL, &[3])
            .long(raw, DEFAULT_CROP_ORIGIN, &MEDIUM_CROP_ORIGIN)
            .long(raw, DEFAULT_CROP_SIZE, &MEDIUM_CROP_SIZE);
    })
}

/// The ARW 2 layout, in byte order `order`, with IFD0's Orientation
/// `orientation` (an EXIF value, 1 to 8).
pub fn arw2_file(order: Order, orientation: u16) -> (Built, ArwBlobs) {
    arw2(order, orientation, "SLT-A99V", |builder, raw| {
        builder
            .long(raw, IMAGE_WIDTH, &[6024])
            .long(raw, IMAGE_LENGTH, &[4024])
            .short(raw, COMPRESSION, &[SONY_RAW])
            .short(raw, PHOTOMETRIC_INTERPRETATION, &[CFA])
            .short(raw, BITS_PER_SAMPLE, &[12])
            .short(raw, SAMPLES_PER_PIXEL, &[1]);
    })
}

/// The ARQ layout, in byte order `order`, with no rotation.
pub fn arq_file(order: Order) -> (Built, ArwBlobs) {
    arw2(order, 1, "ILCE-7RM3", |builder, raw| {
        builder
            .long(raw, IMAGE_WIDTH, &[8000])
            .long(raw, IMAGE_LENGTH, &[5320])
            .short(raw, COMPRESSION, &[UNCOMPRESSED])
            .short(raw, PHOTOMETRIC_INTERPRETATION, &[LINEAR_RAW])
            .short(raw, BITS_PER_SAMPLE, &[16, 16, 16, 16])
            .short(raw, SAMPLES_PER_PIXEL, &[4]);
    })
}

/// The SR2 layout of the DSC-R1, in byte order `order`, with no rotation.
pub fn sr2_file(order: Order) -> (Built, ArwBlobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let raw = builder.ifd();
    let ifd1 = builder.ifd();

    preview_ifd(&mut builder, ifd0, "SONY", "DSC-R1", 1);
    let ifd0_jpeg = jpeg(&mut builder, ifd0, SR2_JPEG);
    builder.subifds(ifd0, &[raw]).next(ifd0, Some(ifd1));

    preview_ifd(&mut builder, ifd1, "SONY", "DSC-R1", 1);
    let ifd1_jpeg = jpeg(&mut builder, ifd1, THUMBNAIL);

    builder
        .long(raw, NEW_SUBFILE_TYPE, &[FULL_RESOLUTION])
        .long(raw, IMAGE_WIDTH, &[3984])
        .long(raw, IMAGE_LENGTH, &[2608])
        .short(raw, COMPRESSION, &[UNCOMPRESSED])
        .short(raw, PHOTOMETRIC_INTERPRETATION, &[CFA])
        .short(raw, BITS_PER_SAMPLE, &[14])
        .short(raw, SAMPLES_PER_PIXEL, &[1]);
    let strip = raw_strip(&mut builder, raw);

    let blobs = ArwBlobs {
        ifd0_jpeg: Some(ifd0_jpeg),
        ifd1_jpeg,
        ifd2_jpeg: None,
        raw: vec![strip],
    };
    (builder.build(), blobs)
}

/// The SRF layout of the DSC-F828, in byte order `order`.
pub fn srf_file(order: Order) -> (Built, ArwBlobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let ifd1 = builder.ifd();

    builder
        .long(ifd0, IMAGE_WIDTH, &[3360])
        .long(ifd0, IMAGE_LENGTH, &[2460])
        .short(ifd0, BITS_PER_SAMPLE, &[14, 14, 14])
        .short(ifd0, COMPRESSION, &[UNCOMPRESSED])
        .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[RGB])
        .ascii(ifd0, MAKE, "SONY")
        .ascii(ifd0, MODEL, "DSC-F828")
        .short(ifd0, SAMPLES_PER_PIXEL, &[3])
        .long(ifd0, STRIP_BYTE_COUNTS, &[16_531_200])
        .next(ifd0, Some(ifd1));

    builder.short(ifd1, COMPRESSION, &[OLD_STYLE_JPEG]);
    let ifd1_jpeg = jpeg(&mut builder, ifd1, SRF_JPEG);

    let blobs = ArwBlobs {
        ifd0_jpeg: None,
        ifd1_jpeg,
        ifd2_jpeg: None,
        raw: Vec::new(),
    };
    (builder.build(), blobs)
}

/// The ARW 1.0 layout of the DSLR-A100, in byte order `order`, with no
/// rotation.
///
/// IFD0's SubIFDs tag holds [`A100_RAW_AT`], where the raw data, written
/// by [`a100_raw`], starts. Read as an IFD, those bytes give an entry
/// count of 7520, whose entries all lie inside the file, so only two rules
/// of the walk can reject them, as the notes say of the real file: the cap
/// on an IFD's entries, 1000 by default, which makes the parse a
/// `LimitExceeded` error that the walk takes for damage, leaving the
/// SubIFD out; and, under a raised cap, the rule that a SubIFD must have
/// no entry of an unknown field type. With the default limits it is the
/// cap that rejects them.
pub fn a100_file(order: Order) -> (Built, ArwBlobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let ifd1 = builder.ifd();

    // `SONY` and eleven spaces; the builder adds the NUL that makes the
    // value sixteen bytes.
    preview_ifd(&mut builder, ifd0, "SONY           ", "DSLR-A100", 1);
    builder
        .long(ifd0, IMAGE_WIDTH, &[u32::from(A100_TAG_SIZE.0)])
        .long(ifd0, IMAGE_LENGTH, &[u32::from(A100_TAG_SIZE.1)]);
    let ifd0_jpeg = jpeg(&mut builder, ifd0, A100_JPEG);
    builder
        .subifds_raw(ifd0, &[A100_RAW_AT])
        .next(ifd0, Some(ifd1));

    preview_ifd(&mut builder, ifd1, "SONY           ", "DSLR-A100", 1);
    let ifd1_jpeg = jpeg(&mut builder, ifd1, THUMBNAIL);

    let raw = builder.blob_at(u64::from(A100_RAW_AT), &a100_raw(order));

    let blobs = ArwBlobs {
        ifd0_jpeg: Some(ifd0_jpeg),
        ifd1_jpeg,
        ifd2_jpeg: None,
        raw: vec![raw],
    };
    (builder.build(), blobs)
}

/// The ARW 4 layout, whose IFD2 declares a JPEG of `full_size` and whose
/// raw SubIFD, a lossless one in two tiles, holds what `raw_tags` adds.
fn arw4(
    order: Order,
    orientation: u16,
    full_size: (u16, u16),
    raw_tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Built, ArwBlobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let raw = builder.ifd();
    let exif = builder.ifd();
    let ifd1 = builder.ifd();
    let ifd2 = builder.ifd();

    preview_ifd(&mut builder, ifd0, "SONY", "ILCE-7M4", orientation);
    let ifd0_jpeg = jpeg(&mut builder, ifd0, IFD0_JPEG);
    builder
        .subifds(ifd0, &[raw])
        .exif(ifd0, exif)
        .next(ifd0, Some(ifd1));
    builder
        .undefined(exif, EXIF_VERSION, b"0232")
        .ascii(exif, DATE_TIME_ORIGINAL, "2023:10:19 16:13:48")
        .ascii(exif, SUB_SEC_TIME_ORIGINAL, "776")
        .ascii(exif, OFFSET_TIME_ORIGINAL, "+02:00")
        .rational(exif, EXPOSURE_TIME, &[(1, 50)])
        .rational(exif, F_NUMBER, &[(56, 10)])
        .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[250])
        .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[250])
        .srational(exif, EXPOSURE_BIAS_VALUE, &[(0, 10)])
        .rational(exif, FOCAL_LENGTH, &[(500, 10)])
        .rational(
            exif,
            LENS_SPECIFICATION,
            &[(500, 10), (500, 10), (25, 10), (25, 10)],
        )
        .ascii(exif, LENS_MODEL, "FE 50mm F2.5 G");
    let makernote = arw4_makernote(&mut builder, order);
    builder.undefined_ifd(exif, MAKER_NOTE, makernote);

    preview_ifd(&mut builder, ifd1, "SONY", "ILCE-7M4", orientation);
    let ifd1_jpeg = jpeg(&mut builder, ifd1, THUMBNAIL);
    builder.next(ifd1, Some(ifd2));

    builder
        .long(ifd2, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .long(ifd2, IMAGE_WIDTH, &[u32::from(full_size.0)])
        .long(ifd2, IMAGE_LENGTH, &[u32::from(full_size.1)])
        .short(ifd2, BITS_PER_SAMPLE, &[8, 8, 8])
        .short(ifd2, COMPRESSION, &[JPEG])
        .short(ifd2, PHOTOMETRIC_INTERPRETATION, &[YCBCR])
        .short(ifd2, ORIENTATION, &[orientation])
        .short(ifd2, SAMPLES_PER_PIXEL, &[3])
        .short(ifd2, YCBCR_SUB_SAMPLING, &[2, 1]);
    let ifd2_jpeg = jpeg(&mut builder, ifd2, full_size);

    // Two tiles, one after the other at a known offset, so that the tile
    // and strip tags can name them: the builder places a blob of its own
    // choosing only for a one-value tag.
    let tile = lossless_tile();
    let tile_len = u32::try_from(tile.len()).expect("a stub is short");
    let first_at = u32::try_from(TILES_AT).expect("the tiles start low");
    let second_at = first_at + tile_len;
    let first = builder.blob_at(u64::from(first_at), &tile);
    let second = builder.blob_at(u64::from(second_at), &tile);
    builder
        .long(raw, NEW_SUBFILE_TYPE, &[FULL_RESOLUTION])
        .short(raw, COMPRESSION, &[JPEG])
        .long(raw, TILE_WIDTH, &[u32::from(RAW_TILE.0)])
        .long(raw, TILE_LENGTH, &[u32::from(RAW_TILE.1)])
        .long(raw, TILE_OFFSETS, &[first_at, second_at])
        .long(raw, TILE_BYTE_COUNTS, &[tile_len, tile_len])
        .long(raw, STRIP_OFFSETS, &[first_at])
        .long(raw, STRIP_BYTE_COUNTS, &[2 * tile_len]);
    raw_tags(&mut builder, raw);

    let blobs = ArwBlobs {
        ifd0_jpeg: Some(ifd0_jpeg),
        ifd1_jpeg,
        ifd2_jpeg: Some(ifd2_jpeg),
        raw: vec![first, second],
    };
    (builder.build(), blobs)
}

/// The six bytes of the FocusFrameSize of the sample 6929, in byte order
/// `order`: [`ARW4_FOCUS_SIZE`]'s width and height, then the third number,
/// as 16-bit numbers.
pub fn arw4_focus_frame_size(order: Order) -> Vec<u8> {
    let (width, height) = ARW4_FOCUS_SIZE;
    [
        order.u16(width),
        order.u16(height),
        order.u16(ARW4_FOCUS_FRAME_SIZE_THIRD),
    ]
    .concat()
}

/// Adds to `builder`, which writes byte order `order`, the MakerNote IFD
/// of the ARW 4 files, without a header, and returns it: the five tags of
/// the sample 6929, each of the type it has there, FocusFrameSize's
/// numbers in the file's byte order.
fn arw4_makernote(builder: &mut TiffBuilder, order: Order) -> IfdId {
    let makernote = builder.ifd();
    let (frame_width, frame_height) = ARW4_FOCUS_FRAME;
    let (x, y) = ARW4_FOCUS_CENTER;
    builder
        .byte(makernote, FOCUS_MODE, &[3])
        .short(
            makernote,
            FOCUS_LOCATION,
            &[frame_width, frame_height, x, y],
        )
        .undefined(makernote, FOCUS_FRAME_SIZE, &arw4_focus_frame_size(order))
        .byte(makernote, FILE_FORMAT, &[4, 0, 1, 0])
        .short(makernote, SONY_MODEL_ID, &[388]);
    makernote
}

/// The ARW 2 layout of a body whose Model is `model`, whose raw SubIFD, in
/// one strip, holds what `raw_tags` adds.
fn arw2(
    order: Order,
    orientation: u16,
    model: &str,
    raw_tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Built, ArwBlobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let raw = builder.ifd();
    let exif = builder.ifd();
    let ifd1 = builder.ifd();

    preview_ifd(&mut builder, ifd0, "SONY", model, orientation);
    let ifd0_jpeg = jpeg(&mut builder, ifd0, IFD0_JPEG);
    builder
        .subifds(ifd0, &[raw])
        .exif(ifd0, exif)
        .next(ifd0, Some(ifd1));
    builder.undefined(exif, EXIF_VERSION, b"0230");

    preview_ifd(&mut builder, ifd1, "SONY", model, orientation);
    let ifd1_jpeg = jpeg(&mut builder, ifd1, THUMBNAIL);

    builder.long(raw, NEW_SUBFILE_TYPE, &[FULL_RESOLUTION]);
    raw_tags(&mut builder, raw);
    let strip = raw_strip(&mut builder, raw);

    let blobs = ArwBlobs {
        ifd0_jpeg: Some(ifd0_jpeg),
        ifd1_jpeg,
        ifd2_jpeg: None,
        raw: vec![strip],
    };
    (builder.build(), blobs)
}

/// Gives `ifd` the tags that IFD0 and IFD1 share on every Sony body but
/// the SRF: NewSubfileType 1, Compression 6, the Make, Model and
/// Orientation.
fn preview_ifd(builder: &mut TiffBuilder, ifd: IfdId, make: &str, model: &str, orientation: u16) {
    builder
        .long(ifd, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .short(ifd, COMPRESSION, &[OLD_STYLE_JPEG])
        .ascii(ifd, MAKE, make)
        .ascii(ifd, MODEL, model)
        .short(ifd, ORIENTATION, &[orientation]);
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

/// Adds 64 bytes that stand for raw data, and are no JPEG stream, and
/// makes them `ifd`'s one strip.
fn raw_strip(builder: &mut TiffBuilder, ifd: IfdId) -> BlobId {
    let blob = builder.blob(&[0x5A; 64]);
    builder
        .long_blob(ifd, STRIP_OFFSETS, blob)
        .long_blob_len(ifd, STRIP_BYTE_COUNTS, blob);
    blob
}
