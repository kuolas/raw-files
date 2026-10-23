// SPDX-License-Identifier: MIT OR Apache-2.0

//! Files laid out like the NEF files of four Nikon bodies, the Z 9, the
//! D300, the Z6III and the D1H, with stub JPEGs in place of the real ones,
//! so that the tests check the NEF reader on the real layouts without a
//! real file. The Z 9 records its metadata in the standard tags; the D300,
//! an older body, records its serial number, its lens's range and its
//! clock's time zone only in its MakerNote; the Z6III is laid out like the
//! Z 9, shot on a 1:1 crop. The first three record where they focused only
//! in their MakerNote: the Z 9 a focus point, the Z6III one in its full
//! frame, which the reader places on the cropped picture, and the D300,
//! shot through the viewfinder in manual focus, none. The D1H, the oldest
//! body, records its ISO and its lens's range only in a MakerNote of the
//! headerless layout, whose offsets count from the start of the file, and
//! keeps its only JPEG there; it records no focus point.
//!
//! The Z 9 layout, which [`z9_file`] writes:
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
//!   holds the MakerNote, whose value is a block [`nikon_makernote`]
//!   writes: Nikon's own metadata, here only its version, `0211`, its
//!   focus mode, its crop area, its AFInfo2 and its PreviewIFD, and none of
//!   the serial number, ISO and lens tags of older bodies. The focus mode
//!   is the FocusMode `AF-S  ` (ASCII), single-servo autofocus padded with
//!   spaces to six characters, as the bodies write an autofocus mode. The
//!   crop area is the visible image, the raw data inside its border: 8256
//!   by 5504 pixels starting 12 columns and 8 rows into the raw, the
//!   CropArea `[12, 8, 8256, 5504]` of the public Z 9 samples 5146 to
//!   5148, as four SHORTs. AFInfo2 is the 80 UNDEFINED bytes of version
//!   `0400` that those samples record, which [`z9_af_info2`] writes: a
//!   focus area of 291 by 323 pixels centred at (4905, 2461) in a frame of
//!   8256 by 5504, with focus confirmed. The PreviewIFD declares a small
//!   JPEG of 640 by 424 pixels. The MakerNote, AFInfo2's numbers included,
//!   is in the file's byte order unless the file is built with
//!   [`z9_file_with`], which may give it the other.
//!
//! Every text value is ASCII ending in one NUL, with no padding, as the
//! Z 9 writes it, but for the lens's model and serial number and the focus
//! mode, padded as above.
//!
//! The D300 layout, which [`d300_file`] writes, is that of the public D300
//! sample 928 of raw.pixls.us, which is big-endian, file and MakerNote
//! alike; the builder takes the byte order, as the Z 9's does, so that the
//! tests and the fuzz seeds can have both:
//!
//! - IFD0 is a reduced-resolution image (NewSubfileType 1) whose Make is
//!   `NIKON CORPORATION` and Model `NIKON D300`, with the Orientation
//!   given. Its image is an uncompressed thumbnail (Compression 1), 160 by
//!   120 pixels of RGB (PhotometricInterpretation 2), three 8-bit samples
//!   per pixel, in one strip, which is not a JPEG and so not a preview. The
//!   strip is 64 zero bytes that stand for the sample's 57,600. IFD0 points
//!   at two SubIFDs, in the order below, and at the Exif IFD.
//! - SubIFD0 is a reduced-resolution image that declares the full-size
//!   JPEG, 4288 by 2848 pixels, through its JPEGInterchangeFormat and
//!   JPEGInterchangeFormatLength tags, with Compression 6.
//! - SubIFD1 is the raw data: the full-resolution image (NewSubfileType 0),
//!   4352 by 2868 pixels, of 12-bit samples of a colour filter array
//!   (PhotometricInterpretation 32803), with Nikon's Compression 34713, in
//!   one strip, of 64 zero bytes that stand for the raw data. Its
//!   ImageWidth and ImageLength are LONGs, and it holds no default crop.
//! - The Exif IFD holds no ExifVersion, as the sample has none. It holds
//!   the capture time, as DateTimeOriginal `2008:05:17 10:22:31`, a time
//!   made up for the fixture, and SubSecTimeOriginal `34`, with no
//!   OffsetTimeOriginal; the exposure, as ExposureTime `10/100`, FNumber
//!   `71/10` and FocalLength `500/10` (RATIONAL), ExposureBiasValue `0/6`
//!   (SRATIONAL) and PhotographicSensitivity 200 (SHORT), with no other
//!   sensitivity tag; these are the sample's values, each of the type it
//!   has there. Like the sample, it holds no BodySerialNumber and no lens
//!   tag. Last, it holds the MakerNote, a block [`nikon_makernote`]
//!   writes, of the version `0210`: its MakerNoteVersion, `0210`; its ISO,
//!   `[0, 200]` (SHORT); its FocusMode `MANUAL` (ASCII), the sample's, a
//!   manual-focus shot; its SerialNumber `7654321` (ASCII), made up, as
//!   no real serial number is copied here; its WorldTime, a time zone of
//!   60 minutes, no daylight saving time and the date format 0, the
//!   sample's; its Lens, `180/10`, `700/10`, `35/10`, `45/10` (RATIONAL),
//!   the sample's 18-70 mm lens of f/3.5 to f/4.5; its AFInfo2, the
//!   sample's version `0100` and 26 zero bytes (UNDEFINED), a shot through
//!   the viewfinder, which records a zero frame and no position; and its
//!   PreviewIFD, which declares a small JPEG of 570 by 375 pixels. The
//!   MakerNote is in the file's byte order.
//!
//! Every text value of the D300 file is ASCII ending in one NUL, with no
//! padding.
//!
//! The Z6III layout, which [`z6iii_file`] writes, is that of the public
//! Z6III sample 7808 of raw.pixls.us, a 1:1 crop, which is little-endian,
//! file and MakerNote alike; the builder takes the byte order, as the
//! others do, and lays the file out as the Z 9's, with these differences:
//!
//! - IFD0's Model is `NIKON Z6_3`. Its small JPEG is the Z 9's 160 by 120.
//! - SubIFD0 declares the full-size JPEG of the cropped picture, 4032 by
//!   4032 pixels, and SubIFD2 the medium one, 1080 by 1080.
//! - SubIFD1 is the raw data, 4048 by 4040 pixels, the stored pixels of the
//!   1:1 crop, of 14-bit samples with Compression 34713, in one strip of 64
//!   bytes that start with `FF 10 FF 50`, as the Z 9's.
//! - SubIFD3 is an uncompressed grey image of 320 by 256 pixels, in one
//!   strip of 64 zero bytes; SubIFD4 and SubIFD5 are the Z 9's empty ones.
//! - The Exif IFD holds the sample's values, each of the type it has there:
//!   ExifVersion `0232`; DateTimeOriginal `2024:06:29 15:38:53`,
//!   SubSecTimeOriginal `42` and OffsetTimeOriginal `-04:00`; ExposureTime
//!   `10/1000`, FNumber `900/100` and FocalLength `860/10`,
//!   ExposureBiasValue `0/6`, PhotographicSensitivity 200, SensitivityType
//!   2 and RecommendedExposureIndex 200; LensMake `NIKON`, LensModel
//!   `NIKKOR Z 24-200mm f/4-6.3 VR`, padded with NULs to 65 bytes as the
//!   Z 9's, and LensSpecification `240/10`, `2000/10`, `400/100`,
//!   `630/100`. The BodySerialNumber `2345678` and the LensSerialNumber
//!   `23456789`, padded as the Z 9's, are made up.
//! - The MakerNote, of the version `0211`, holds the FocusMode `AF-S  `;
//!   the CropHiSpeed `[17, 6064, 4040, 4048, 4040, 1008, 0]` (SHORT), the
//!   1:1 crop of a 6064 by 4040 picture; the CropArea `[8, 4, 4032, 4032]`;
//!   the sample's AFInfo2 of version `0401`, the 80 bytes the Z 9's layout
//!   has, with the area mode 192 (pinpoint), a frame of 6048 by 4032, the
//!   camera's full output, a focus area of 163 by 174 pixels centred at
//!   (3023, 2016) in it, and the in-focus byte 0, a missed focus; and a
//!   PreviewIFD that declares a small JPEG of 480 by 480 pixels.
//!
//! The D1H layout, which [`d1h_file`] writes, is that of the public D1H
//! sample 5364 of raw.pixls.us, which is big-endian, file and MakerNote
//! alike; the builder takes the byte order, as the others do:
//!
//! - IFD0 is a reduced-resolution image (NewSubfileType 1) whose Make is
//!   `NIKON CORPORATION` and Model `NIKON D1H`, with no Orientation. Its
//!   image is an uncompressed thumbnail, 160 by 120 pixels of RGB, three
//!   8-bit samples per pixel, in one strip of 64 zero bytes that stand for
//!   the sample's 57,600, which is not a JPEG and so not a preview. IFD0
//!   points at one SubIFD and at the Exif IFD.
//! - SubIFD0 is the raw data: the full-resolution image (NewSubfileType
//!   0), 2012 by 1324 pixels, of 12-bit samples of a colour filter array,
//!   uncompressed (Compression 1), in one strip of 64 zero bytes that stand
//!   for the sample's 67 strips. Its ImageWidth and ImageLength are LONGs,
//!   and it holds no default crop.
//! - The Exif IFD holds the sample's values, each of the type it has
//!   there: DateTimeOriginal `2022:03:24 10:09:27` and SubSecTimeOriginal
//!   `01`, with no OffsetTimeOriginal; ExposureTime `10/8000`, FNumber
//!   `45/10` and FocalLength `600/10` (RATIONAL) and ExposureBiasValue
//!   `4/6` (SRATIONAL). Like the sample, it holds no ExifVersion, no
//!   sensitivity tag, no BodySerialNumber and no lens tag. Last, it holds
//!   the MakerNote, of the headerless layout: no header, the MakerNote IFD
//!   at its first byte, written by the file's own builder, so that every
//!   offset in it, the PreviewIFD's and the JPEG's among them, counts from
//!   the start of the file, and every number is in the file's byte order,
//!   as on the sample. Its block runs from the MakerNote IFD through that
//!   IFD's values, the PreviewIFD and its values, to the end of the
//!   PreviewIFD's JPEG ([`TiffBuilder::block_through`]), so that all of
//!   them lie inside the length the MakerNote's entry declares, as on the
//!   sample. The MakerNote IFD holds the sample's MakerNoteVersion `0101`
//!   (UNDEFINED); its ISO, `[0, 200]` (SHORT); its FocusMode `AF-S  `
//!   (ASCII); its PreviewIFD pointer; its Lens, `180/10`, `700/10`,
//!   `35/10`, `45/10` (RATIONAL), an 18-70 mm lens of f/3.5 to f/4.5; its
//!   AFInfo `01 00 01 00`, a point number and no position, and no AFInfo2,
//!   so no focus point; and the text of nine more of its tags, which the
//!   reader does not read ([`D1H_TEXT_TAGS`]). It holds no SerialNumber,
//!   WorldTime, CropArea or CropHiSpeed, as the sample holds none. The
//!   PreviewIFD holds the sample's seven entries: Compression 6,
//!   XResolution and YResolution `300/1`, ResolutionUnit 2 (inches),
//!   JPEGInterchangeFormat and JPEGInterchangeFormatLength, which declare a
//!   small JPEG of 570 by 375 pixels, the file's only JPEG, and
//!   YCbCrPositioning 2. [`d1h_file_with`] may lay a header out before the
//!   MakerNote IFD, inside the block, and may make the MakerNote's entry
//!   declare less than the whole block.
//!
//! Every text value of the D1H file is ASCII, padded with spaces as the
//! sample pads it, ending in one NUL, or in two for the last of
//! [`D1H_TEXT_TAGS`], as on the sample.
//!
//! Every stub JPEG is a baseline one. The tags that the shared builder has
//! no constant for are named here.

use std::ops::Range;

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
/// The pixels per unit across, as a RATIONAL (XResolution).
const X_RESOLUTION: u16 = 0x011A;
/// The pixels per unit down, as a RATIONAL (YResolution).
const Y_RESOLUTION: u16 = 0x011B;
/// The unit of XResolution and YResolution, as a SHORT (ResolutionUnit).
const RESOLUTION_UNIT: u16 = 0x0128;
/// Where an embedded JPEG stream starts (JPEGInterchangeFormat).
const JPEG_INTERCHANGE_FORMAT: u16 = 0x0201;
/// How many bytes the embedded JPEG stream takes
/// (JPEGInterchangeFormatLength).
const JPEG_INTERCHANGE_FORMAT_LENGTH: u16 = 0x0202;
/// Where the chroma samples lie against the luma samples, as a SHORT
/// (YCbCrPositioning).
const Y_CB_CR_POSITIONING: u16 = 0x0213;
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
/// The ISO, as two SHORTs of which the second is the ISO (ISO, a tag of
/// the MakerNote IFD).
pub const ISO: u16 = 0x0002;
/// How the camera focused, as ASCII text: `MANUAL`, or an autofocus mode
/// padded with spaces to six characters (FocusMode, a tag of the MakerNote
/// IFD).
pub const FOCUS_MODE: u16 = 0x0007;
/// The offset of the MakerNote's PreviewIFD, the IFD that declares its
/// small JPEG (PreviewIFD, a tag of the MakerNote IFD).
pub const PREVIEW_IFD: u16 = 0x0011;
/// The crop mode and how large the picture was before and after the crop,
/// as seven SHORTs: the mode, the original width and height, the cropped
/// width and height, and the left column and top row where the crop starts
/// (CropHiSpeed, a tag of the MakerNote IFD).
pub const CROP_HI_SPEED: u16 = 0x001B;
/// The camera body's serial number, as ASCII text (SerialNumber, a tag of
/// the MakerNote IFD).
pub const SERIAL_NUMBER: u16 = 0x001D;
/// The time zone and daylight saving setting of the camera's clock, as
/// four UNDEFINED bytes: the zone in minutes from UTC as a signed 16-bit
/// number in the MakerNote's byte order, then whether daylight saving time
/// is on, then the date format (WorldTime, a tag of the MakerNote IFD).
pub const WORLD_TIME: u16 = 0x0024;
/// The visible image's rectangle in the raw data, as four SHORTs: its left
/// column, top row, width and height (CropArea, a tag of the MakerNote
/// IFD).
pub const CROP_AREA: u16 = 0x0045;
/// The Z 9's CropArea, as the public samples 5146 to 5148 record it.
const CROP_AREA_Z9: [u16; 4] = [12, 8, 8256, 5504];
/// The lens's range, as four RATIONALs in the order of the Exif
/// LensSpecification: the shortest and longest focal length, then the
/// f-number at each (Lens, a tag of the MakerNote IFD).
pub const LENS: u16 = 0x0084;
/// Which autofocus point the camera used, as four UNDEFINED bytes: the
/// area mode, the number of the point and a mask of the points in focus,
/// with no position (AFInfo, a tag of the MakerNote IFD of the bodies
/// without AFInfo2).
const AF_INFO: u16 = 0x0088;
/// Where the camera's autofocus aimed, as an UNDEFINED value that starts
/// with its version, four ASCII digits, and holds at the version's offsets
/// the size of a frame, the centre of the focus area and the area's size,
/// as SHORTs in the MakerNote's byte order, and on most versions whether
/// focus was confirmed (AFInfo2, a tag of the MakerNote IFD).
pub const AF_INFO2: u16 = 0x00B7;

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
/// PhotometricInterpretation of an RGB image.
const RGB: u16 = 2;
/// PhotometricInterpretation of a colour filter array, raw sensor data.
const CFA: u16 = 32803;
/// ResolutionUnit of inches.
const INCH: u16 = 2;
/// YCbCrPositioning of chroma samples that lie on the first luma sample
/// of each pair, co-sited.
const CO_SITED: u16 = 2;
/// The baseline frame header's marker, SOF0.
const BASELINE: u8 = 0xC0;

/// The signature a Nikon MakerNote starts with: `Nikon` and a NUL.
const MAKER_NOTE_SIGNATURE: &[u8; 6] = b"Nikon\0";
/// The version of the MakerNote's layout that the Z 9 writes, in the two
/// bytes after the signature: `02 11`.
const Z9_MAKER_NOTE_VERSION: [u8; 2] = [0x02, 0x11];
/// The version that the D300 writes: `02 10`.
const D300_MAKER_NOTE_VERSION: [u8; 2] = [0x02, 0x10];

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

/// The size in pixels of the D300 file's JPEGs, as on the public sample
/// 928: SubIFD0's full-size one.
pub const D300_FULL_SIZE_JPEG: (u16, u16) = (4288, 2848);
/// The D300 file's MakerNote's small JPEG; see [`D300_FULL_SIZE_JPEG`].
pub const D300_PREVIEW_JPEG: (u16, u16) = (570, 375);
/// The size in pixels of the D300 file's uncompressed thumbnail in IFD0,
/// which is not a JPEG.
pub const D300_THUMBNAIL: (u16, u16) = (160, 120);
/// The D300 file's raw size, SubIFD1's: width, then height.
pub const D300_RAW: (u32, u32) = (4352, 2868);
/// The D300 file's serial number, in its MakerNote only: made up, as no
/// real serial number is copied here.
pub const D300_SERIAL_NUMBER: &str = "7654321";
/// The D300 file's lens range, in its MakerNote only: that of the public
/// sample 928, an 18-70 mm lens of f/3.5 to f/4.5.
pub const D300_LENS: [(u32, u32); 4] = [(180, 10), (700, 10), (35, 10), (45, 10)];
/// The time zone of the D300 file's WorldTime, in minutes from UTC: 60,
/// with no daylight saving time and the date format 0, as on the public
/// sample 928.
const D300_TIME_ZONE: u16 = 60;

/// The size in pixels of the D1H file's one JPEG, the MakerNote's small
/// one, as on the public sample 5364.
pub const D1H_PREVIEW_JPEG: (u16, u16) = (570, 375);
/// The size in pixels of the D1H file's uncompressed thumbnail in IFD0,
/// which is not a JPEG.
pub const D1H_THUMBNAIL: (u16, u16) = (160, 120);
/// The D1H file's raw size, SubIFD0's: width, then height.
pub const D1H_RAW: (u32, u32) = (2012, 1324);
/// The D1H file's ISO, in its MakerNote only, the second of its two
/// SHORTs, as on the public sample 5364.
pub const D1H_ISO: u16 = 200;
/// The D1H file's lens range, in its MakerNote only: that of the public
/// sample 5364, an 18-70 mm lens of f/3.5 to f/4.5.
pub const D1H_LENS: [(u32, u32); 4] = [(180, 10), (700, 10), (35, 10), (45, 10)];
/// The D1H file's AFInfo, as on the public sample 5364: a point number, no
/// position.
const D1H_AF_INFO: [u8; 4] = [1, 0, 1, 0];
/// Nine more tags of the D1H file's MakerNote IFD, each with its ASCII
/// text, less the NUL the builder adds: those of the public sample 5364,
/// which the reader does not read. They make the MakerNote IFD, of 15
/// entries, longer than IFD0, of 13, the longest IFD outside it, as on the
/// sample, whose MakerNote IFD holds 27 and IFD0 26.
const D1H_TEXT_TAGS: [(u16, &str); 9] = [
    (0x0003, "COLOR"),
    (0x0004, "RAW    "),
    (0x0005, "SUNNY       "),
    (0x0006, "HIGH  "),
    (0x0008, "            "),
    (0x0009, "        "),
    (0x0081, "NORMAL  "),
    (0x008D, "MODE2   "),
    (0x0090, "NATURAL   \0"),
];

/// The frame of the Z 9 file's focus point, as its AFInfo2 records it:
/// width, then height, the camera's full-size output, as on the public Z 9
/// samples 5146 to 5148.
pub const Z9_FOCUS_FRAME: (u32, u32) = (8256, 5504);
/// The centre of the Z 9 file's focus area in [`Z9_FOCUS_FRAME`]: x, then
/// y, as on the public Z 9 samples 5146 to 5148.
pub const Z9_FOCUS_CENTER: (u32, u32) = (4905, 2461);
/// The size of the Z 9 file's focus area: width, then height, a single
/// point's, as on the public Z 9 samples 5146 to 5148.
pub const Z9_FOCUS_AREA: (u32, u32) = (291, 323);

/// An AFInfo2 of the layout of versions 0400, 0401 and 0402, as the Z
/// bodies write it, with its SHORTs in byte order `order`: 80 bytes, the
/// version `version`, the detection method 2 (hybrid), the area mode
/// `area_mode`, a zero byte and the coordinates flag 1; zeros up to byte
/// 62; then the frame, the centre and the area of `record`, each width or
/// x before its height or y; the in-focus byte `in_focus`; and five zero
/// bytes.
pub fn af_info2_04xx(
    order: Order,
    version: &[u8; 4],
    area_mode: u8,
    record: [(u32, u32); 3],
    in_focus: u8,
) -> Vec<u8> {
    let mut value = version.to_vec();
    value.extend_from_slice(&[2, area_mode, 0, 1]);
    value.resize(62, 0);
    for (first, second) in record {
        for number in [first, second] {
            let short = u16::try_from(number).expect("a SHORT");
            value.extend_from_slice(&order.u16(short));
        }
    }
    value.push(in_focus);
    value.resize(80, 0);
    value
}

/// The Z 9's AFInfo2, as the public samples 5146 to 5148 record it, with
/// its SHORTs in byte order `order`, as [`af_info2_04xx`] writes it: the
/// version `0400`, the area mode 193 (single point), [`Z9_FOCUS_FRAME`],
/// [`Z9_FOCUS_CENTER`] and [`Z9_FOCUS_AREA`], and the in-focus byte 1,
/// focus confirmed.
pub fn z9_af_info2(order: Order) -> Vec<u8> {
    af_info2_04xx(
        order,
        b"0400",
        0xC1,
        [Z9_FOCUS_FRAME, Z9_FOCUS_CENTER, Z9_FOCUS_AREA],
        1,
    )
}

/// The size in pixels of each JPEG of the Z6III file but IFD0's, which is
/// [`IFD0_JPEG`], as on the public sample 7808: SubIFD0's full-size one,
/// the size of the cropped picture.
pub const Z6III_FULL_SIZE_JPEG: (u16, u16) = (4032, 4032);
/// SubIFD2's medium JPEG; see [`Z6III_FULL_SIZE_JPEG`].
pub const Z6III_MEDIUM_JPEG: (u16, u16) = (1080, 1080);
/// The MakerNote's small JPEG; see [`Z6III_FULL_SIZE_JPEG`].
pub const Z6III_PREVIEW_JPEG: (u16, u16) = (480, 480);
/// The Z6III file's raw size, SubIFD1's: width, then height, the stored
/// pixels of the 1:1 crop, as on the public sample 7808.
pub const Z6III_RAW: (u32, u32) = (4048, 4040);
/// The Z6III file's image size, its CropArea's: width, then height.
pub const Z6III_IMAGE: (u32, u32) = (4032, 4032);

/// The Z6III's CropHiSpeed, as the public sample 7808 records it: the mode
/// 17 (FX 1:1), the original size 6064 by 4040, the cropped size 4048 by
/// 4040, and the crop's left column, 1008, and top row, 0.
const CROP_HI_SPEED_Z6III_1X1: [u16; 7] = [17, 6064, 4040, 4048, 4040, 1008, 0];
/// The Z6III's CropArea, as the public sample 7808 records it.
const CROP_AREA_Z6III_1X1: [u16; 4] = [8, 4, 4032, 4032];

/// The focus point that the Z6III file's AFInfo2 records, as on the public
/// sample 7808: the frame, the camera's full 6048 by 4032 output, the
/// centre and the area in it, a pinpoint area, each width or x first.
const Z6III_RECORDED_FOCUS: [(u32, u32); 3] = [(6048, 4032), (3023, 2016), (163, 174)];

/// The frame of the Z6III file's focus point, as the reader gives it: the
/// cropped picture, CropArea's size, not the frame AFInfo2 records.
pub const Z6III_FOCUS_FRAME: (u32, u32) = (4032, 4032);
/// The centre of the Z6III file's focus area in [`Z6III_FOCUS_FRAME`]: the
/// recorded (3023, 2016) moved back across by the 1:1 window's origin,
/// half of 6048 less 4032.
pub const Z6III_FOCUS_CENTER: (u32, u32) = (2015, 2016);
/// The size of the Z6III file's focus area, kept as recorded.
pub const Z6III_FOCUS_AREA: (u32, u32) = (163, 174);
/// Whether the Z6III file records that focus was confirmed: no, a miss
/// beside a valid point, as on every public Z6III sample.
pub const Z6III_IN_FOCUS: bool = false;

/// The model name of the Z 9 file's lens, as its LensModel records it.
pub const LENS_MODEL_Z14_30: &str = "NIKKOR Z 14-30mm f/4 S";

/// The model name of the Z6III file's lens, as its LensModel records it on
/// the public sample 7808.
pub const LENS_MODEL_Z24_200: &str = "NIKKOR Z 24-200mm f/4-6.3 VR";

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
/// out, or of a Z6III file, laid out alike by [`z6iii_file_with_blobs`].
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
/// order `order` and of the version `version`, whose MakerNote IFD holds
/// what `tags` adds, given the inner builder and that IFD, and whose
/// PreviewIFD declares `preview`; and where `preview` starts, counted from
/// the start of the block.
///
/// The block is the signature `Nikon` and a NUL, the two bytes of
/// `version`, two zero bytes, and then a TIFF structure of its own: a TIFF
/// header in `order`, then the MakerNote IFD, which holds the
/// MakerNoteVersion, `version` as four ASCII digits, the PreviewIFD tag and
/// the tags `tags` adds, each value too long for its entry following the
/// IFD, then the PreviewIFD, which declares `preview` through
/// JPEGInterchangeFormat and JPEGInterchangeFormatLength, then `preview`'s
/// bytes. Every offset inside the block counts from the inner TIFF header,
/// 10 bytes in, which is exactly how a second [`TiffBuilder`] counts them:
/// from its own first byte, which is that header.
pub fn nikon_makernote(
    order: Order,
    version: [u8; 2],
    preview: &[u8],
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Vec<u8>, u64) {
    let (block, at) = nikon_makernote_with(order, version, Some(preview), tags);
    (block, at.expect("a PreviewIFD"))
}

/// As [`nikon_makernote`], with a PreviewIFD and its JPEG only when
/// `preview` is given, and where that JPEG starts then; without one, the
/// MakerNote IFD holds no PreviewIFD tag, as the MakerNote of the Nikon
/// D5300's JPEG on Wikimedia Commons holds none.
pub fn nikon_makernote_with(
    order: Order,
    version: [u8; 2],
    preview: Option<&[u8]>,
    tags: impl FnOnce(&mut TiffBuilder, IfdId),
) -> (Vec<u8>, Option<u64>) {
    let mut inner = TiffBuilder::new(order);
    let makernote = inner.ifd();
    let preview_ifd = preview.map(|_| inner.ifd());
    let digits = format!("{:02x}{:02x}", version[0], version[1]);
    inner.undefined(makernote, MAKER_NOTE_VERSION, digits.as_bytes());
    if let Some(preview_ifd) = preview_ifd {
        inner.ifd_pointer(makernote, PREVIEW_IFD, preview_ifd);
    }
    tags(&mut inner, makernote);
    let jpeg = preview.zip(preview_ifd).map(|(preview, preview_ifd)| {
        let jpeg = inner.blob(preview);
        inner
            .long_blob(preview_ifd, JPEG_INTERCHANGE_FORMAT, jpeg)
            .long_blob_len(preview_ifd, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg);
        jpeg
    });
    let inner = inner.build();

    let mut block = MAKER_NOTE_SIGNATURE.to_vec();
    block.extend_from_slice(&version);
    block.extend_from_slice(&[0, 0]);
    let header_at = u64::try_from(block.len()).expect("ten bytes");
    block.extend_from_slice(&inner.bytes);
    (block, jpeg.map(|jpeg| header_at + inner.blob_offset(jpeg)))
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
    let (block, preview_in_makernote) = nikon_makernote(
        makernote_order,
        Z9_MAKER_NOTE_VERSION,
        &stub(PREVIEW_JPEG),
        |inner, ifd| {
            inner
                .ascii(ifd, FOCUS_MODE, "AF-S  ")
                .short(ifd, CROP_AREA, &CROP_AREA_Z9)
                .undefined(ifd, AF_INFO2, &z9_af_info2(makernote_order));
        },
    );
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

/// The Z6III layout, in byte order `order`, with IFD0's Orientation
/// `orientation` (an EXIF value, 1 to 8).
pub fn z6iii_file(order: Order, orientation: u16) -> Built {
    z6iii_file_with_blobs(order, orientation).0
}

/// As [`z6iii_file`], with where its blobs are, so that a test can find
/// them with [`Built::blob_offset`]: the Z6III file is laid out like the
/// Z 9's, so the same fields name its blobs.
pub fn z6iii_file_with_blobs(order: Order, orientation: u16) -> (Built, Z9Blobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let subs: Vec<IfdId> = (0..6).map(|_| builder.ifd()).collect();
    let exif = builder.ifd();

    builder
        .long(ifd0, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .ascii(ifd0, MAKE, "NIKON CORPORATION")
        .ascii(ifd0, MODEL, "NIKON Z6_3")
        .short(ifd0, ORIENTATION, &[orientation])
        .short(ifd0, COMPRESSION, &[OLD_STYLE_JPEG]);
    let ifd0_jpeg = jpeg(&mut builder, ifd0, IFD0_JPEG);
    builder.subifds(ifd0, &subs).exif(ifd0, exif);

    builder
        .long(subs[0], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .short(subs[0], COMPRESSION, &[OLD_STYLE_JPEG]);
    let full_size_jpeg = jpeg(&mut builder, subs[0], Z6III_FULL_SIZE_JPEG);

    let mut raw_strip = RAW_STRIP_START.to_vec();
    raw_strip.resize(64, 0);
    let (raw_width, raw_height) = Z6III_RAW;
    builder
        .long(subs[1], NEW_SUBFILE_TYPE, &[FULL_RESOLUTION])
        .long(subs[1], IMAGE_WIDTH, &[raw_width])
        .long(subs[1], IMAGE_LENGTH, &[raw_height])
        .short(subs[1], BITS_PER_SAMPLE, &[14])
        .short(subs[1], COMPRESSION, &[NIKON_RAW])
        .short(subs[1], PHOTOMETRIC_INTERPRETATION, &[CFA])
        .short(subs[1], SAMPLES_PER_PIXEL, &[1]);
    let raw = strip(&mut builder, subs[1], &raw_strip);

    builder
        .long(subs[2], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .short(subs[2], COMPRESSION, &[OLD_STYLE_JPEG]);
    let medium_jpeg = jpeg(&mut builder, subs[2], Z6III_MEDIUM_JPEG);

    builder
        .long(subs[3], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .long(subs[3], IMAGE_WIDTH, &[320])
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
        .ascii(exif, DATE_TIME_ORIGINAL, "2024:06:29 15:38:53")
        .ascii(exif, SUB_SEC_TIME_ORIGINAL, "42")
        .ascii(exif, OFFSET_TIME_ORIGINAL, "-04:00")
        .rational(exif, EXPOSURE_TIME, &[(10, 1000)])
        .rational(exif, F_NUMBER, &[(900, 100)])
        .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[200])
        .short(exif, SENSITIVITY_TYPE, &[2])
        .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[200])
        .srational(exif, EXPOSURE_BIAS_VALUE, &[(0, 6)])
        .rational(exif, FOCAL_LENGTH, &[(860, 10)])
        .ascii(exif, BODY_SERIAL_NUMBER, "2345678")
        .rational(
            exif,
            LENS_SPECIFICATION,
            &[(240, 10), (2000, 10), (400, 100), (630, 100)],
        )
        .ascii(exif, LENS_MAKE, "NIKON")
        .ascii(exif, LENS_MODEL, &padded_lens_model(LENS_MODEL_Z24_200))
        .ascii(exif, LENS_SERIAL_NUMBER, "23456789  ");
    let (block, preview_in_makernote) = nikon_makernote(
        order,
        Z9_MAKER_NOTE_VERSION,
        &stub(Z6III_PREVIEW_JPEG),
        |inner, ifd| {
            inner
                .ascii(ifd, FOCUS_MODE, "AF-S  ")
                .short(ifd, CROP_HI_SPEED, &CROP_HI_SPEED_Z6III_1X1)
                .short(ifd, CROP_AREA, &CROP_AREA_Z6III_1X1)
                .undefined(
                    ifd,
                    AF_INFO2,
                    &af_info2_04xx(order, b"0401", 0xC0, Z6III_RECORDED_FOCUS, 0),
                );
        },
    );
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

/// Where the blobs of a D300 file are, as [`d300_file_with_blobs`] laid
/// them out.
#[derive(Clone, Copy, Debug)]
pub struct D300Blobs {
    /// IFD0's uncompressed thumbnail strip.
    pub thumbnail: BlobId,
    /// SubIFD0's full-size JPEG.
    pub full_size_jpeg: BlobId,
    /// SubIFD1's raw strip.
    pub raw: BlobId,
    /// The Exif IFD's MakerNote block.
    pub makernote: BlobId,
    /// Where the MakerNote's small JPEG starts, counted from the start of
    /// the MakerNote block: the JPEG is at `blob_offset(makernote)` plus
    /// this.
    pub preview_in_makernote: u64,
}

/// The D300 layout, in byte order `order`, with IFD0's Orientation
/// `orientation` (an EXIF value, 1 to 8).
pub fn d300_file(order: Order, orientation: u16) -> Built {
    d300_file_with_blobs(order, orientation).0
}

/// As [`d300_file`], with where its blobs are, so that a test can find
/// them with [`Built::blob_offset`].
pub fn d300_file_with_blobs(order: Order, orientation: u16) -> (Built, D300Blobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let subs = [builder.ifd(), builder.ifd()];
    let exif = builder.ifd();

    let (width, height) = D300_THUMBNAIL;
    builder
        .long(ifd0, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .long(ifd0, IMAGE_WIDTH, &[u32::from(width)])
        .long(ifd0, IMAGE_LENGTH, &[u32::from(height)])
        .short(ifd0, BITS_PER_SAMPLE, &[8, 8, 8])
        .short(ifd0, COMPRESSION, &[UNCOMPRESSED])
        .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[RGB])
        .ascii(ifd0, MAKE, "NIKON CORPORATION")
        .ascii(ifd0, MODEL, "NIKON D300")
        .short(ifd0, ORIENTATION, &[orientation])
        .short(ifd0, SAMPLES_PER_PIXEL, &[3]);
    let thumbnail = strip(&mut builder, ifd0, &[0; 64]);
    builder.subifds(ifd0, &subs).exif(ifd0, exif);

    builder
        .long(subs[0], NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .short(subs[0], COMPRESSION, &[OLD_STYLE_JPEG]);
    let full_size_jpeg = jpeg(&mut builder, subs[0], D300_FULL_SIZE_JPEG);

    let (raw_width, raw_height) = D300_RAW;
    builder
        .long(subs[1], NEW_SUBFILE_TYPE, &[FULL_RESOLUTION])
        .long(subs[1], IMAGE_WIDTH, &[raw_width])
        .long(subs[1], IMAGE_LENGTH, &[raw_height])
        .short(subs[1], BITS_PER_SAMPLE, &[12])
        .short(subs[1], COMPRESSION, &[NIKON_RAW])
        .short(subs[1], PHOTOMETRIC_INTERPRETATION, &[CFA])
        .short(subs[1], SAMPLES_PER_PIXEL, &[1]);
    let raw = strip(&mut builder, subs[1], &[0; 64]);

    builder
        .ascii(exif, DATE_TIME_ORIGINAL, "2008:05:17 10:22:31")
        .ascii(exif, SUB_SEC_TIME_ORIGINAL, "34")
        .rational(exif, EXPOSURE_TIME, &[(10, 100)])
        .rational(exif, F_NUMBER, &[(71, 10)])
        .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[200])
        .srational(exif, EXPOSURE_BIAS_VALUE, &[(0, 6)])
        .rational(exif, FOCAL_LENGTH, &[(500, 10)]);
    let mut world_time = order.u16(D300_TIME_ZONE).to_vec();
    world_time.extend_from_slice(&[0, 0]);
    let mut af_info2 = b"0100".to_vec();
    af_info2.resize(30, 0);
    let (block, preview_in_makernote) = nikon_makernote(
        order,
        D300_MAKER_NOTE_VERSION,
        &stub(D300_PREVIEW_JPEG),
        |inner, ifd| {
            inner
                .short(ifd, ISO, &[0, 200])
                .ascii(ifd, FOCUS_MODE, "MANUAL")
                .ascii(ifd, SERIAL_NUMBER, D300_SERIAL_NUMBER)
                .undefined(ifd, WORLD_TIME, &world_time)
                .rational(ifd, LENS, &D300_LENS)
                .undefined(ifd, AF_INFO2, &af_info2);
        },
    );
    let makernote = builder.blob(&block);
    builder.undefined_blob(exif, MAKER_NOTE, makernote);

    let blobs = D300Blobs {
        thumbnail,
        full_size_jpeg,
        raw,
        makernote,
        preview_in_makernote,
    };
    (builder.build(), blobs)
}

/// Where the parts of a D1H file are, as [`d1h_file_with`] laid them out.
#[derive(Clone, Debug)]
pub struct D1hBlobs {
    /// IFD0's uncompressed thumbnail strip.
    pub thumbnail: BlobId,
    /// SubIFD0's raw strip.
    pub raw: BlobId,
    /// The MakerNote's small JPEG, the one its PreviewIFD declares: the
    /// file's only JPEG, the last bytes of the MakerNote's block.
    pub preview_jpeg: BlobId,
    /// Where the MakerNote's block lies in the file: from its first byte,
    /// the MakerNote IFD's, or the header's when one is laid out before
    /// it, to the end of the JPEG.
    pub makernote: Range<u64>,
    /// Where the PreviewIFD lies in the file.
    pub preview_ifd: u64,
}

/// The D1H layout, in byte order `order`.
pub fn d1h_file(order: Order) -> Built {
    d1h_file_with_blobs(order).0
}

/// As [`d1h_file`], with where its parts are, so that a test can find
/// them.
pub fn d1h_file_with_blobs(order: Order) -> (Built, D1hBlobs) {
    d1h_file_with(order, &[], None)
}

/// As [`d1h_file_with_blobs`], with `header` laid out before the MakerNote
/// IFD, inside the MakerNote's block, when it is not empty, such as the
/// `Nikon\0\1\0` of the short-header layout; and with the MakerNote's
/// entry declaring the first `declared` bytes of the block when given,
/// rather than the whole block. Neither moves anything but the MakerNote
/// IFD and what follows it, by the header's length.
pub fn d1h_file_with(order: Order, header: &[u8], declared: Option<u64>) -> (Built, D1hBlobs) {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let sub = builder.ifd();
    let exif = builder.ifd();
    let makernote = builder.ifd();
    let preview_ifd = builder.ifd();

    let (width, height) = D1H_THUMBNAIL;
    builder
        .long(ifd0, NEW_SUBFILE_TYPE, &[REDUCED_RESOLUTION])
        .long(ifd0, IMAGE_WIDTH, &[u32::from(width)])
        .long(ifd0, IMAGE_LENGTH, &[u32::from(height)])
        .short(ifd0, BITS_PER_SAMPLE, &[8, 8, 8])
        .short(ifd0, COMPRESSION, &[UNCOMPRESSED])
        .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[RGB])
        .ascii(ifd0, MAKE, "NIKON CORPORATION")
        .ascii(ifd0, MODEL, "NIKON D1H")
        .short(ifd0, SAMPLES_PER_PIXEL, &[3]);
    builder.subifds(ifd0, &[sub]).exif(ifd0, exif);

    let (raw_width, raw_height) = D1H_RAW;
    builder
        .long(sub, NEW_SUBFILE_TYPE, &[FULL_RESOLUTION])
        .long(sub, IMAGE_WIDTH, &[raw_width])
        .long(sub, IMAGE_LENGTH, &[raw_height])
        .short(sub, BITS_PER_SAMPLE, &[12])
        .short(sub, COMPRESSION, &[UNCOMPRESSED])
        .short(sub, PHOTOMETRIC_INTERPRETATION, &[CFA])
        .short(sub, SAMPLES_PER_PIXEL, &[1]);

    builder
        .rational(exif, EXPOSURE_TIME, &[(10, 8000)])
        .rational(exif, F_NUMBER, &[(45, 10)])
        .ascii(exif, DATE_TIME_ORIGINAL, "2022:03:24 10:09:27")
        .srational(exif, EXPOSURE_BIAS_VALUE, &[(4, 6)])
        .rational(exif, FOCAL_LENGTH, &[(600, 10)])
        .ascii(exif, SUB_SEC_TIME_ORIGINAL, "01");

    builder
        .prefix(makernote, header)
        .undefined(makernote, MAKER_NOTE_VERSION, b"0101")
        .short(makernote, ISO, &[0, D1H_ISO])
        .ascii(makernote, FOCUS_MODE, "AF-S  ")
        .ifd_pointer(makernote, PREVIEW_IFD, preview_ifd)
        .rational(makernote, LENS, &D1H_LENS)
        .undefined(makernote, AF_INFO, &D1H_AF_INFO);
    for (tag, text) in D1H_TEXT_TAGS {
        builder.ascii(makernote, tag, text);
    }
    builder
        .short(preview_ifd, COMPRESSION, &[OLD_STYLE_JPEG])
        .rational(preview_ifd, X_RESOLUTION, &[(300, 1)])
        .rational(preview_ifd, Y_RESOLUTION, &[(300, 1)])
        .short(preview_ifd, RESOLUTION_UNIT, &[INCH])
        .short(preview_ifd, Y_CB_CR_POSITIONING, &[CO_SITED]);
    // The first blob, so that it follows the PreviewIFD, inside the block.
    let preview_jpeg = jpeg(&mut builder, preview_ifd, D1H_PREVIEW_JPEG);
    builder.block_through(makernote, preview_jpeg);
    let thumbnail = strip(&mut builder, ifd0, &[0; 64]);
    let raw = strip(&mut builder, sub, &[0; 64]);

    match declared {
        None => builder.undefined_ifd(exif, MAKER_NOTE, makernote),
        Some(declared) => {
            let count = u32::try_from(declared).expect("a TIFF count");
            builder.undefined_ifd_declaring(exif, MAKER_NOTE, makernote, count)
        }
    };
    let built = builder.build();
    let blobs = D1hBlobs {
        thumbnail,
        raw,
        preview_jpeg,
        makernote: built.ifd_block(makernote),
        preview_ifd: built.ifd_offset(preview_ifd),
    };
    (built, blobs)
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
