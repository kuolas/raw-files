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

/// The offset of the IFD that holds the Exif capture metadata (Exif IFD),
/// read from IFD0.
pub(crate) const EXIF_IFD: u16 = 0x8769;
