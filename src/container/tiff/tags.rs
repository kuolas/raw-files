// SPDX-License-Identifier: MIT OR Apache-2.0

//! The numbers of the TIFF and Exif tags the crate reads.
//!
//! A tag says what an entry of an IFD means; its number is how the file
//! names it. Each constant here is documented with what the tag is for, so
//! that the code reading it can use the name alone. Only the tags something
//! reads are listed, in the order of their numbers: a tag joins this list
//! with its first reader.

/// Whether the image an IFD describes is the full-resolution image, a
/// reduced-resolution copy of it, or a page of several (NewSubfileType): a
/// bit field, 0 for the full-resolution image.
pub(crate) const NEW_SUBFILE_TYPE: u16 = 0x00FE;

/// How many bits each sample of the image's pixels holds, one value per
/// sample (BitsPerSample).
pub(crate) const BITS_PER_SAMPLE: u16 = 0x0102;

/// How the image's data is compressed (Compression): 1 for none, 6 and 7
/// for JPEG, and values of the vendors' own.
pub(crate) const COMPRESSION: u16 = 0x0103;

/// What the image's samples mean (PhotometricInterpretation): RGB, YCbCr,
/// a colour filter array of sensor data, and so on.
pub(crate) const PHOTOMETRIC_INTERPRETATION: u16 = 0x0106;

/// The manufacturer of the camera that wrote the file, as text (Make), read
/// from IFD0: what tells one vendor's files from another's.
pub(crate) const MAKE: u16 = 0x010F;

/// The model of the camera that wrote the file, as text (Model), read from
/// IFD0, such as `NIKON Z 9`.
pub(crate) const MODEL: u16 = 0x0110;

/// Where each strip of the image's data starts, as absolute offsets
/// (StripOffsets).
pub(crate) const STRIP_OFFSETS: u16 = 0x0111;

/// What a viewer must do to the stored pixels to show the image upright
/// (Orientation): one of the eight EXIF values.
pub(crate) const ORIENTATION: u16 = 0x0112;

/// How many bytes each strip of the image's data takes, one value per
/// strip (StripByteCounts).
pub(crate) const STRIP_BYTE_COUNTS: u16 = 0x0117;

/// The width in pixels of each tile of an image stored in tiles
/// (TileWidth).
pub(crate) const TILE_WIDTH: u16 = 0x0142;

/// The height in pixels of each tile of an image stored in tiles
/// (TileLength).
pub(crate) const TILE_LENGTH: u16 = 0x0143;

/// Where each tile of an image stored in tiles starts (TileOffsets).
pub(crate) const TILE_OFFSETS: u16 = 0x0144;

/// How many bytes each tile of an image stored in tiles takes
/// (TileByteCounts).
pub(crate) const TILE_BYTE_COUNTS: u16 = 0x0145;

/// The offsets of an IFD's child IFDs (SubIFDs), which describe further
/// images of the file.
pub(crate) const SUB_IFDS: u16 = 0x014A;

/// Where an embedded JPEG stream starts, as an absolute offset
/// (JPEGInterchangeFormat): how most RAW files point at a preview.
pub(crate) const JPEG_INTERCHANGE_FORMAT: u16 = 0x0201;

/// How many bytes the embedded JPEG stream that
/// [`JPEG_INTERCHANGE_FORMAT`] points at takes
/// (JPEGInterchangeFormatLength).
pub(crate) const JPEG_INTERCHANGE_FORMAT_LENGTH: u16 = 0x0202;

/// How long the shutter was open, in seconds, as a fraction of type
/// RATIONAL (ExposureTime), read from the Exif IFD, such as 10/3200 for
/// 1/320 s.
pub(crate) const EXPOSURE_TIME: u16 = 0x829A;

/// The f-number the picture was taken at, as a fraction of type RATIONAL
/// (FNumber), read from the Exif IFD, such as 900/100 for f/9.
pub(crate) const F_NUMBER: u16 = 0x829D;

/// The offset of the IFD that holds the Exif capture metadata (Exif IFD),
/// read from IFD0.
pub(crate) const EXIF_IFD: u16 = 0x8769;

/// The sensitivity the picture was taken at, as a SHORT or a LONG
/// (PhotographicSensitivity, called ISOSpeedRatings before Exif 2.3), read
/// from the Exif IFD: the ISO. Exif writes 65535 for a sensitivity of 65535
/// or more.
pub(crate) const PHOTOGRAPHIC_SENSITIVITY: u16 = 0x8827;

/// The recommended exposure index of the camera as ISO 12232 defines it,
/// as a LONG (RecommendedExposureIndex), read from the Exif IFD: what
/// gives the ISO when PhotographicSensitivity cannot.
pub(crate) const RECOMMENDED_EXPOSURE_INDEX: u16 = 0x8832;

/// The ISO speed of the camera as ISO 12232 defines it, as a LONG
/// (ISOSpeed), read from the Exif IFD: what gives the ISO when neither
/// PhotographicSensitivity nor RecommendedExposureIndex can.
pub(crate) const ISO_SPEED: u16 = 0x8833;

/// When the picture was taken, as the camera's clock showed it, as text in
/// the form `YYYY:MM:DD HH:MM:SS` (DateTimeOriginal), read from the Exif
/// IFD: the capture time.
pub(crate) const DATE_TIME_ORIGINAL: u16 = 0x9003;

/// The offset from UTC of the camera's clock when the picture was taken,
/// as text in the form `+HH:MM` or `-HH:MM` (OffsetTimeOriginal), read from
/// the Exif IFD.
pub(crate) const OFFSET_TIME_ORIGINAL: u16 = 0x9011;

/// The exposure compensation chosen, in EV, as a fraction of type
/// SRATIONAL (ExposureBiasValue), read from the Exif IFD, such as -3/10;
/// 0/6 means no compensation.
pub(crate) const EXPOSURE_BIAS_VALUE: u16 = 0x9204;

/// The focal length the picture was taken at, in millimetres, as a
/// fraction of type RATIONAL (FocalLength), read from the Exif IFD: the
/// lens's actual focal length, such as 180/10, not its 35 mm equivalent.
pub(crate) const FOCAL_LENGTH: u16 = 0x920A;

/// The fraction of a second of the capture time, as text holding its
/// digits alone (SubSecTimeOriginal), read from the Exif IFD.
pub(crate) const SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;

/// The camera maker's own metadata block, whose layout each vendor defines
/// (MakerNote): an Exif tag, read from the Exif IFD, whose value is the
/// block's bytes.
pub(crate) const MAKER_NOTE: u16 = 0x927C;

/// The serial number of the camera body, as text (BodySerialNumber), read
/// from the Exif IFD.
pub(crate) const BODY_SERIAL_NUMBER: u16 = 0xA431;

/// The version of the DNG specification a DNG file follows (DNGVersion),
/// read from IFD0: present in every DNG and in the DNG-based files of some
/// makers, absent from a Sony RAW file.
pub(crate) const DNG_VERSION: u16 = 0xC612;

/// A maker's private data block (DNGPrivateData), read from IFD0: Sony
/// writes it in every ARW, SR2 and ARQ file as the offset of its
/// SR2Private directory, or, on the A100, of a Minolta block.
pub(crate) const DNG_PRIVATE_DATA: u16 = 0xC634;
