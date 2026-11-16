// SPDX-License-Identifier: MIT OR Apache-2.0

//! The numbers of the tags [`Entry::name`](crate::Entry::name) has a name
//! for, one module per kind of directory.
//!
//! [`Entries::entry`](crate::Entries::entry) finds an entry by its
//! directory and its tag number. Each constant here is one such number,
//! named after the tag, so that a lookup says what it looks for:
//! `tags::exif::DATE_TIME_ORIGINAL` rather than `0x9003`.
//!
//! A tag's number means something only in its kind of directory: the GPS
//! IFD's tags number from 0x0000 and the Interoperability IFD's from
//! 0x0001, so their numbers repeat other directories'. So the constants
//! are kept per kind of directory, in the module of that kind:
//!
//! | Module | Directories |
//! |---|---|
//! | [`tiff`] | [`Directory::Ifd`](crate::Directory::Ifd) and [`Directory::SubIfd`](crate::Directory::SubIfd) |
//! | [`exif`] | [`Directory::Exif`](crate::Directory::Exif) |
//! | [`gps`] | [`Directory::Gps`](crate::Directory::Gps) |
//! | [`interop`] | [`Directory::Interop`](crate::Directory::Interop) |
//! | [`mp_index`] | [`Directory::MpIndex`](crate::Directory::MpIndex) |
//!
//! A constant is the name [`Entry::name`](crate::Entry::name) gives the
//! tag in that kind of directory, in capitals, with an underscore where a
//! word starts; an acronym is one word. A maker's own directories have no
//! constants, since no maker publishes its tags.
//!
//! # Examples
//!
//! Read when a picture was taken, from the Exif IFD:
//!
//! ```
//! use std::path::Path;
//!
//! use raw_files::{CameraFile, Directory, tags};
//!
//! fn capture_time(path: &Path) -> raw_files::Result<Option<String>> {
//!     let file = CameraFile::open(path)?;
//!     let Some(entry) = file
//!         .entries()
//!         .entry(Directory::Exif, tags::exif::DATE_TIME_ORIGINAL)
//!     else {
//!         return Ok(None);
//!     };
//!     Ok(file.read_value(&entry)?.text())
//! }
//! # fn main() -> raw_files::Result<()> {
//! #     // A NEF file of the repository, which a packaged crate leaves out.
//! #     let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/nikon-z9-dx.nef");
//! #     if Path::new(path).exists() {
//! #         let time = capture_time(Path::new(path))?.expect("DateTimeOriginal");
//! #         assert_eq!(time.len(), "YYYY:MM:DD HH:MM:SS".len());
//! #     }
//! #     Ok(())
//! # }
//! ```
//!
//! Match an entry against the tags a program knows, as a pattern:
//!
//! ```
//! use raw_files::{Directory, Entry, tags};
//!
//! fn label(entry: &Entry) -> &'static str {
//!     match (entry.directory(), entry.tag()) {
//!         (Directory::Ifd(_), tags::tiff::MAKE) => "Camera maker",
//!         (Directory::Ifd(_), tags::tiff::MODEL) => "Camera model",
//!         (Directory::Exif, tags::exif::DATE_TIME_ORIGINAL) => "Taken",
//!         (Directory::Gps, tags::gps::GPS_LATITUDE) => "Latitude",
//!         _ => "Other",
//!     }
//! }
//! # let _ = label;
//! ```

/// Declares a module of tag constants, and the table of their names that
/// [`Entry::name`](crate::Entry::name) searches, from one row per tag:
/// its number, its constant and its name as the specification spells it.
/// So a constant and its name cannot disagree, and no number is written
/// twice.
macro_rules! tags {
    (
        $(#[$meta:meta])*
        $module:ident, $kind:literal {
            $($tag:literal $constant:ident $name:literal,)*
        }
    ) => {
        $(#[$meta])*
        pub mod $module {
            $(
                #[doc = concat!("Tag ", stringify!($tag), " of ", $kind, ", named `", $name, "`.")]
                pub const $constant: u16 = $tag;
            )*

            /// The name of every tag of the module, by number.
            pub(crate) const NAMES: &[(u16, &str)] = &[$(($constant, $name)),*];
        }
    };
}

// The rows are written from `docs/formats/tag-names.md` alone, which
// records each name with the document and the page that give it: TIFF
// 6.0 and Exif 3.1 for the IFD chain and the SubIFDs, with the DNG
// specification for the four DNG tags raw-files reads and Adobe's
// PageMaker 6.0 TIFF Technical Notes for the SubIFDs tag; Exif 3.1 for
// the Exif and GPS IFDs; Exif 3.1 and DCF 2.0 for the Interoperability
// IFD; and CIPA DC-007 for the Multi-Picture index. A name is spelled as
// the document spells it, spaces included. No other reader's tag list
// was used. Within a module the rows are sorted by number, each number
// once, since `Entry::name` searches them in halves.

tags! {
    /// The tags of IFD0, the IFDs chained after it and the SubIFDs: TIFF
    /// 6.0's, Exif's for IFD0 and IFD1, the DNG specification's for the
    /// DNG tags raw-files reads, and Adobe's TIFF notes' for the SubIFDs
    /// tag.
    tiff, "the IFD chain and the SubIFDs" {
        0x00FE NEW_SUBFILE_TYPE "NewSubfileType",
        0x00FF SUBFILE_TYPE "SubfileType",
        0x0100 IMAGE_WIDTH "ImageWidth",
        0x0101 IMAGE_LENGTH "ImageLength",
        0x0102 BITS_PER_SAMPLE "BitsPerSample",
        0x0103 COMPRESSION "Compression",
        0x0106 PHOTOMETRIC_INTERPRETATION "PhotometricInterpretation",
        0x0107 THRESHHOLDING "Threshholding",
        0x0108 CELL_WIDTH "CellWidth",
        0x0109 CELL_LENGTH "CellLength",
        0x010A FILL_ORDER "FillOrder",
        0x010D DOCUMENT_NAME "DocumentName",
        0x010E IMAGE_DESCRIPTION "ImageDescription",
        0x010F MAKE "Make",
        0x0110 MODEL "Model",
        0x0111 STRIP_OFFSETS "StripOffsets",
        0x0112 ORIENTATION "Orientation",
        0x0115 SAMPLES_PER_PIXEL "SamplesPerPixel",
        0x0116 ROWS_PER_STRIP "RowsPerStrip",
        0x0117 STRIP_BYTE_COUNTS "StripByteCounts",
        0x0118 MIN_SAMPLE_VALUE "MinSampleValue",
        0x0119 MAX_SAMPLE_VALUE "MaxSampleValue",
        0x011A X_RESOLUTION "XResolution",
        0x011B Y_RESOLUTION "YResolution",
        0x011C PLANAR_CONFIGURATION "PlanarConfiguration",
        0x011D PAGE_NAME "PageName",
        0x011E X_POSITION "XPosition",
        0x011F Y_POSITION "YPosition",
        0x0120 FREE_OFFSETS "FreeOffsets",
        0x0121 FREE_BYTE_COUNTS "FreeByteCounts",
        0x0122 GRAY_RESPONSE_UNIT "GrayResponseUnit",
        0x0123 GRAY_RESPONSE_CURVE "GrayResponseCurve",
        0x0124 T4_OPTIONS "T4Options",
        0x0125 T6_OPTIONS "T6Options",
        0x0128 RESOLUTION_UNIT "ResolutionUnit",
        0x0129 PAGE_NUMBER "PageNumber",
        0x012D TRANSFER_FUNCTION "TransferFunction",
        0x0131 SOFTWARE "Software",
        0x0132 DATE_TIME "DateTime",
        0x013B ARTIST "Artist",
        0x013C HOST_COMPUTER "HostComputer",
        0x013D PREDICTOR "Predictor",
        0x013E WHITE_POINT "WhitePoint",
        0x013F PRIMARY_CHROMATICITIES "PrimaryChromaticities",
        0x0140 COLOR_MAP "ColorMap",
        0x0141 HALFTONE_HINTS "HalftoneHints",
        0x0142 TILE_WIDTH "TileWidth",
        0x0143 TILE_LENGTH "TileLength",
        0x0144 TILE_OFFSETS "TileOffsets",
        0x0145 TILE_BYTE_COUNTS "TileByteCounts",
        0x014A SUB_IFDS "SubIFDs",
        0x014C INK_SET "InkSet",
        0x014D INK_NAMES "InkNames",
        0x014E NUMBER_OF_INKS "NumberOfInks",
        0x0150 DOT_RANGE "DotRange",
        0x0151 TARGET_PRINTER "TargetPrinter",
        0x0152 EXTRA_SAMPLES "ExtraSamples",
        0x0153 SAMPLE_FORMAT "SampleFormat",
        0x0154 S_MIN_SAMPLE_VALUE "SMinSampleValue",
        0x0155 S_MAX_SAMPLE_VALUE "SMaxSampleValue",
        0x0156 TRANSFER_RANGE "TransferRange",
        0x0200 JPEG_PROC "JPEGProc",
        0x0201 JPEG_INTERCHANGE_FORMAT "JPEGInterchangeFormat",
        0x0202 JPEG_INTERCHANGE_FORMAT_LENGTH "JPEGInterchangeFormatLength",
        0x0203 JPEG_RESTART_INTERVAL "JPEGRestartInterval",
        0x0205 JPEG_LOSSLESS_PREDICTORS "JPEGLosslessPredictors",
        0x0206 JPEG_POINT_TRANSFORMS "JPEGPointTransforms",
        0x0207 JPEG_Q_TABLES "JPEGQTables",
        0x0208 JPEG_DC_TABLES "JPEGDCTables",
        0x0209 JPEG_AC_TABLES "JPEGACTables",
        0x0211 Y_CB_CR_COEFFICIENTS "YCbCrCoefficients",
        0x0212 Y_CB_CR_SUB_SAMPLING "YCbCrSubSampling",
        0x0213 Y_CB_CR_POSITIONING "YCbCrPositioning",
        0x0214 REFERENCE_BLACK_WHITE "ReferenceBlackWhite",
        0x8298 COPYRIGHT "Copyright",
        0x8769 EXIF_IFD_POINTER "Exif IFD Pointer",
        0x8825 GPS_INFO_IFD_POINTER "GPS Info IFD Pointer",
        0xC612 DNG_VERSION "DNGVersion",
        0xC61F DEFAULT_CROP_ORIGIN "DefaultCropOrigin",
        0xC620 DEFAULT_CROP_SIZE "DefaultCropSize",
        0xC634 DNG_PRIVATE_DATA "DNGPrivateData",
    }
}

tags! {
    /// The tags of the Exif IFD, as Exif names them.
    exif, "the Exif IFD" {
        0x829A EXPOSURE_TIME "ExposureTime",
        0x829D F_NUMBER "FNumber",
        0x8822 EXPOSURE_PROGRAM "ExposureProgram",
        0x8824 SPECTRAL_SENSITIVITY "SpectralSensitivity",
        0x8827 PHOTOGRAPHIC_SENSITIVITY "PhotographicSensitivity",
        0x8828 OECF "OECF",
        0x8830 SENSITIVITY_TYPE "SensitivityType",
        0x8831 STANDARD_OUTPUT_SENSITIVITY "StandardOutputSensitivity",
        0x8832 RECOMMENDED_EXPOSURE_INDEX "RecommendedExposureIndex",
        0x8833 ISO_SPEED "ISOSpeed",
        0x8834 ISO_SPEED_LATITUDE_YYY "ISOSpeedLatitudeyyy",
        0x8835 ISO_SPEED_LATITUDE_ZZZ "ISOSpeedLatitudezzz",
        0x9000 EXIF_VERSION "ExifVersion",
        0x9003 DATE_TIME_ORIGINAL "DateTimeOriginal",
        0x9004 DATE_TIME_DIGITIZED "DateTimeDigitized",
        0x9010 OFFSET_TIME "OffsetTime",
        0x9011 OFFSET_TIME_ORIGINAL "OffsetTimeOriginal",
        0x9012 OFFSET_TIME_DIGITIZED "OffsetTimeDigitized",
        0x9101 COMPONENTS_CONFIGURATION "ComponentsConfiguration",
        0x9102 COMPRESSED_BITS_PER_PIXEL "CompressedBitsPerPixel",
        0x9201 SHUTTER_SPEED_VALUE "ShutterSpeedValue",
        0x9202 APERTURE_VALUE "ApertureValue",
        0x9203 BRIGHTNESS_VALUE "BrightnessValue",
        0x9204 EXPOSURE_BIAS_VALUE "ExposureBiasValue",
        0x9205 MAX_APERTURE_VALUE "MaxApertureValue",
        0x9206 SUBJECT_DISTANCE "SubjectDistance",
        0x9207 METERING_MODE "MeteringMode",
        0x9208 LIGHT_SOURCE "LightSource",
        0x9209 FLASH "Flash",
        0x920A FOCAL_LENGTH "FocalLength",
        0x9214 SUBJECT_AREA "SubjectArea",
        0x927C MAKER_NOTE "MakerNote",
        0x9286 USER_COMMENT "UserComment",
        0x9287 LEARNING_OPT_OUT_IN "LearningOptOutIn",
        0x9290 SUB_SEC_TIME "SubSecTime",
        0x9291 SUB_SEC_TIME_ORIGINAL "SubSecTimeOriginal",
        0x9292 SUB_SEC_TIME_DIGITIZED "SubSecTimeDigitized",
        0x9400 TEMPERATURE "Temperature",
        0x9401 HUMIDITY "Humidity",
        0x9402 PRESSURE "Pressure",
        0x9403 WATER_DEPTH "WaterDepth",
        0x9404 ACCELERATION "Acceleration",
        0x9405 CAMERA_ELEVATION_ANGLE "CameraElevationAngle",
        0xA000 FLASHPIX_VERSION "FlashpixVersion",
        0xA001 COLOR_SPACE "ColorSpace",
        0xA002 PIXEL_X_DIMENSION "PixelXDimension",
        0xA003 PIXEL_Y_DIMENSION "PixelYDimension",
        0xA004 RELATED_SOUND_FILE "RelatedSoundFile",
        0xA005 INTEROPERABILITY_IFD_POINTER "Interoperability IFD Pointer",
        0xA20B FLASH_ENERGY "FlashEnergy",
        0xA20C SPATIAL_FREQUENCY_RESPONSE "SpatialFrequencyResponse",
        0xA20E FOCAL_PLANE_X_RESOLUTION "FocalPlaneXResolution",
        0xA20F FOCAL_PLANE_Y_RESOLUTION "FocalPlaneYResolution",
        0xA210 FOCAL_PLANE_RESOLUTION_UNIT "FocalPlaneResolutionUnit",
        0xA214 SUBJECT_LOCATION "SubjectLocation",
        0xA215 EXPOSURE_INDEX "ExposureIndex",
        0xA217 SENSING_METHOD "SensingMethod",
        0xA300 FILE_SOURCE "FileSource",
        0xA301 SCENE_TYPE "SceneType",
        0xA302 CFA_PATTERN "CFAPattern",
        0xA401 CUSTOM_RENDERED "CustomRendered",
        0xA402 EXPOSURE_MODE "ExposureMode",
        0xA403 WHITE_BALANCE "WhiteBalance",
        0xA404 DIGITAL_ZOOM_RATIO "DigitalZoomRatio",
        0xA405 FOCAL_LENGTH_IN_35MM_FILM "FocalLengthIn35mmFilm",
        0xA406 SCENE_CAPTURE_TYPE "SceneCaptureType",
        0xA407 GAIN_CONTROL "GainControl",
        0xA408 CONTRAST "Contrast",
        0xA409 SATURATION "Saturation",
        0xA40A SHARPNESS "Sharpness",
        0xA40B DEVICE_SETTING_DESCRIPTION "DeviceSettingDescription",
        0xA40C SUBJECT_DISTANCE_RANGE "SubjectDistanceRange",
        0xA40D DEVELOPMENT_TYPE "DevelopmentType",
        0xA40E DEVELOPMENT_TYPE_DESCRIPTION "DevelopmentTypeDescription",
        0xA40F DISTORTION_CORRECTION "DistortionCorrection",
        0xA410 CHROMATIC_ABERRATION_CORRECTION "ChromaticAberrationCorrection",
        0xA411 SHADING_CORRECTION "ShadingCorrection",
        0xA412 NOISE_REDUCTION "NoiseReduction",
        0xA420 IMAGE_UNIQUE_ID "ImageUniqueID",
        0xA430 CAMERA_OWNER_NAME "CameraOwnerName",
        0xA431 BODY_SERIAL_NUMBER "BodySerialNumber",
        0xA432 LENS_SPECIFICATION "LensSpecification",
        0xA433 LENS_MAKE "LensMake",
        0xA434 LENS_MODEL "LensModel",
        0xA435 LENS_SERIAL_NUMBER "LensSerialNumber",
        0xA436 IMAGE_TITLE "ImageTitle",
        0xA437 PHOTOGRAPHER "Photographer",
        0xA438 IMAGE_EDITOR "ImageEditor",
        0xA439 CAMERA_FIRMWARE "CameraFirmware",
        0xA43A RAW_DEVELOPING_SOFTWARE "RAWDevelopingSoftware",
        0xA43B IMAGE_EDITING_SOFTWARE "ImageEditingSoftware",
        0xA43C METADATA_EDITING_SOFTWARE "MetadataEditingSoftware",
        0xA460 COMPOSITE_IMAGE "CompositeImage",
        0xA461 SOURCE_IMAGE_NUMBER_OF_COMPOSITE_IMAGE "SourceImageNumberOfCompositeImage",
        0xA462 SOURCE_EXPOSURE_TIMES_OF_COMPOSITE_IMAGE "SourceExposureTimesOfCompositeImage",
        0xA500 GAMMA "Gamma",
    }
}

tags! {
    /// The tags of the GPS IFD, as Exif names them.
    gps, "the GPS IFD" {
        0x0000 GPS_VERSION_ID "GPSVersionID",
        0x0001 GPS_LATITUDE_REF "GPSLatitudeRef",
        0x0002 GPS_LATITUDE "GPSLatitude",
        0x0003 GPS_LONGITUDE_REF "GPSLongitudeRef",
        0x0004 GPS_LONGITUDE "GPSLongitude",
        0x0005 GPS_ALTITUDE_REF "GPSAltitudeRef",
        0x0006 GPS_ALTITUDE "GPSAltitude",
        0x0007 GPS_TIME_STAMP "GPSTimeStamp",
        0x0008 GPS_SATELLITES "GPSSatellites",
        0x0009 GPS_STATUS "GPSStatus",
        0x000A GPS_MEASURE_MODE "GPSMeasureMode",
        0x000B GPS_DOP "GPSDOP",
        0x000C GPS_SPEED_REF "GPSSpeedRef",
        0x000D GPS_SPEED "GPSSpeed",
        0x000E GPS_TRACK_REF "GPSTrackRef",
        0x000F GPS_TRACK "GPSTrack",
        0x0010 GPS_IMG_DIRECTION_REF "GPSImgDirectionRef",
        0x0011 GPS_IMG_DIRECTION "GPSImgDirection",
        0x0012 GPS_MAP_DATUM "GPSMapDatum",
        0x0013 GPS_DEST_LATITUDE_REF "GPSDestLatitudeRef",
        0x0014 GPS_DEST_LATITUDE "GPSDestLatitude",
        0x0015 GPS_DEST_LONGITUDE_REF "GPSDestLongitudeRef",
        0x0016 GPS_DEST_LONGITUDE "GPSDestLongitude",
        0x0017 GPS_DEST_BEARING_REF "GPSDestBearingRef",
        0x0018 GPS_DEST_BEARING "GPSDestBearing",
        0x0019 GPS_DEST_DISTANCE_REF "GPSDestDistanceRef",
        0x001A GPS_DEST_DISTANCE "GPSDestDistance",
        0x001B GPS_PROCESSING_METHOD "GPSProcessingMethod",
        0x001C GPS_AREA_INFORMATION "GPSAreaInformation",
        0x001D GPS_DATE_STAMP "GPSDateStamp",
        0x001E GPS_DIFFERENTIAL "GPSDifferential",
        0x001F GPS_H_POSITIONING_ERROR "GPSHPositioningError",
    }
}

tags! {
    /// The tags of the Interoperability IFD, as Exif and DCF name them.
    interop, "the Interoperability IFD" {
        0x0001 INTEROPERABILITY_INDEX "InteroperabilityIndex",
        0x0002 INTEROPERABILITY_VERSION "InteroperabilityVersion",
        0x1000 RELATED_IMAGE_FILE_FORMAT "RelatedImageFileFormat",
        0x1001 RELATED_IMAGE_WIDTH "RelatedImageWidth",
        0x1002 RELATED_IMAGE_LENGTH "RelatedImageLength",
    }
}

tags! {
    /// The tags of the Multi-Picture Format's index IFD, as CIPA DC-007
    /// names them.
    mp_index, "the MP Index IFD" {
        0xB000 MPF_VERSION "MPFVersion",
        0xB001 NUMBER_OF_IMAGES "NumberOfImages",
        0xB002 MP_ENTRY "MPEntry",
        0xB003 IMAGE_UID_LIST "ImageUIDList",
        0xB004 TOTAL_FRAMES "TotalFrames",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: [(&[(u16, &str)], usize); 5] = [
        (tiff::NAMES, 81),
        (exif::NAMES, 96),
        (gps::NAMES, 32),
        (interop::NAMES, 5),
        (mp_index::NAMES, 5),
    ];

    #[test]
    fn every_table_is_sorted_by_tag_each_tag_once_with_the_notes_counts() {
        for (table, count) in TABLES {
            assert_eq!(table.len(), count);
            assert!(table.windows(2).all(|pair| pair[0].0 < pair[1].0));
            for &(tag, name) in table {
                assert!(!name.is_empty() && name.is_ascii(), "{tag:#06X}");
                assert_eq!(name, name.trim(), "{tag:#06X}");
            }
        }
        assert_eq!(TABLES.iter().map(|(_, count)| count).sum::<usize>(), 219);
    }

    #[test]
    fn a_constant_is_the_number_its_specification_prints() {
        // As TIFF 6.0, Adobe's TIFF notes, Exif 3.1, DCF 2.0 and CIPA
        // DC-007 print them, so that a shifted row fails.
        assert_eq!(tiff::MAKE, 0x010F);
        assert_eq!(tiff::SUB_IFDS, 0x014A);
        assert_eq!(tiff::JPEG_INTERCHANGE_FORMAT, 0x0201);
        assert_eq!(tiff::EXIF_IFD_POINTER, 0x8769);
        assert_eq!(exif::DATE_TIME_ORIGINAL, 0x9003);
        assert_eq!(exif::MAKER_NOTE, 0x927C);
        assert_eq!(gps::GPS_LATITUDE, 0x0002);
        assert_eq!(gps::GPS_H_POSITIONING_ERROR, 0x001F);
        assert_eq!(interop::INTEROPERABILITY_INDEX, 0x0001);
        assert_eq!(mp_index::MP_ENTRY, 0xB002);
    }
}
