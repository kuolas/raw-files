// SPDX-License-Identifier: MIT OR Apache-2.0

//! The names published standards give the tags of each kind of directory.
//!
//! A tag's name depends on the kind of directory it lies in: the GPS IFD's
//! tags number from 0x0000 and the Interoperability IFD's from 0x0001, so
//! their numbers repeat other directories', and Exif names its own tags in
//! the Exif IFD only, not where a camera writes one of them in IFD0. So the
//! names are kept per kind of directory, one table each, and an entry gets
//! a name only where a standard names its tag in its kind of directory.
//!
//! The tables are written from `docs/formats/tag-names.md` alone, which
//! records each name with the document and the page that give it: TIFF
//! 6.0 and Exif 3.1 for IFD0, the IFDs chained after it and the SubIFDs,
//! with the DNG specification for the four DNG tags raw-files reads; Exif
//! 3.1 for the Exif and GPS IFDs; Exif 3.1 and DCF 2.0 for the
//! Interoperability IFD; and CIPA DC-007 for the Multi-Picture index. A
//! name is spelled as the document spells it, spaces included. No other
//! reader's tag list was used.

use super::Directory;

/// The name a published standard gives `tag` in a directory of the kind
/// `directory` is; `None` when none of the documents names it there.
pub(super) fn name(directory: Directory, tag: u16) -> Option<&'static str> {
    let table = match directory {
        Directory::Ifd(_) | Directory::SubIfd(_) => CHAIN,
        Directory::Exif => EXIF,
        Directory::Gps => GPS,
        Directory::Interop => INTEROP,
        Directory::MpIndex => MP_INDEX,
    };
    let at = table.binary_search_by_key(&tag, |&(tag, _)| tag).ok()?;
    table.get(at).map(|&(_, name)| name)
}

/// The names of the tags of IFD0, the IFDs chained after it and the
/// SubIFDs, by tag: TIFF 6.0's, Exif's for IFD0 and IFD1, and the DNG
/// specification's for the DNG tags raw-files reads.
const CHAIN: &[(u16, &str)] = &[
    (0x00FE, "NewSubfileType"),
    (0x00FF, "SubfileType"),
    (0x0100, "ImageWidth"),
    (0x0101, "ImageLength"),
    (0x0102, "BitsPerSample"),
    (0x0103, "Compression"),
    (0x0106, "PhotometricInterpretation"),
    (0x0107, "Threshholding"),
    (0x0108, "CellWidth"),
    (0x0109, "CellLength"),
    (0x010A, "FillOrder"),
    (0x010D, "DocumentName"),
    (0x010E, "ImageDescription"),
    (0x010F, "Make"),
    (0x0110, "Model"),
    (0x0111, "StripOffsets"),
    (0x0112, "Orientation"),
    (0x0115, "SamplesPerPixel"),
    (0x0116, "RowsPerStrip"),
    (0x0117, "StripByteCounts"),
    (0x0118, "MinSampleValue"),
    (0x0119, "MaxSampleValue"),
    (0x011A, "XResolution"),
    (0x011B, "YResolution"),
    (0x011C, "PlanarConfiguration"),
    (0x011D, "PageName"),
    (0x011E, "XPosition"),
    (0x011F, "YPosition"),
    (0x0120, "FreeOffsets"),
    (0x0121, "FreeByteCounts"),
    (0x0122, "GrayResponseUnit"),
    (0x0123, "GrayResponseCurve"),
    (0x0124, "T4Options"),
    (0x0125, "T6Options"),
    (0x0128, "ResolutionUnit"),
    (0x0129, "PageNumber"),
    (0x012D, "TransferFunction"),
    (0x0131, "Software"),
    (0x0132, "DateTime"),
    (0x013B, "Artist"),
    (0x013C, "HostComputer"),
    (0x013D, "Predictor"),
    (0x013E, "WhitePoint"),
    (0x013F, "PrimaryChromaticities"),
    (0x0140, "ColorMap"),
    (0x0141, "HalftoneHints"),
    (0x0142, "TileWidth"),
    (0x0143, "TileLength"),
    (0x0144, "TileOffsets"),
    (0x0145, "TileByteCounts"),
    (0x014C, "InkSet"),
    (0x014D, "InkNames"),
    (0x014E, "NumberOfInks"),
    (0x0150, "DotRange"),
    (0x0151, "TargetPrinter"),
    (0x0152, "ExtraSamples"),
    (0x0153, "SampleFormat"),
    (0x0154, "SMinSampleValue"),
    (0x0155, "SMaxSampleValue"),
    (0x0156, "TransferRange"),
    (0x0200, "JPEGProc"),
    (0x0201, "JPEGInterchangeFormat"),
    (0x0202, "JPEGInterchangeFormatLength"),
    (0x0203, "JPEGRestartInterval"),
    (0x0205, "JPEGLosslessPredictors"),
    (0x0206, "JPEGPointTransforms"),
    (0x0207, "JPEGQTables"),
    (0x0208, "JPEGDCTables"),
    (0x0209, "JPEGACTables"),
    (0x0211, "YCbCrCoefficients"),
    (0x0212, "YCbCrSubSampling"),
    (0x0213, "YCbCrPositioning"),
    (0x0214, "ReferenceBlackWhite"),
    (0x8298, "Copyright"),
    (0x8769, "Exif IFD Pointer"),
    (0x8825, "GPS Info IFD Pointer"),
    (0xC612, "DNGVersion"),
    (0xC61F, "DefaultCropOrigin"),
    (0xC620, "DefaultCropSize"),
    (0xC634, "DNGPrivateData"),
];

/// The names of the tags of the Exif IFD, by tag.
const EXIF: &[(u16, &str)] = &[
    (0x829A, "ExposureTime"),
    (0x829D, "FNumber"),
    (0x8822, "ExposureProgram"),
    (0x8824, "SpectralSensitivity"),
    (0x8827, "PhotographicSensitivity"),
    (0x8828, "OECF"),
    (0x8830, "SensitivityType"),
    (0x8831, "StandardOutputSensitivity"),
    (0x8832, "RecommendedExposureIndex"),
    (0x8833, "ISOSpeed"),
    (0x8834, "ISOSpeedLatitudeyyy"),
    (0x8835, "ISOSpeedLatitudezzz"),
    (0x9000, "ExifVersion"),
    (0x9003, "DateTimeOriginal"),
    (0x9004, "DateTimeDigitized"),
    (0x9010, "OffsetTime"),
    (0x9011, "OffsetTimeOriginal"),
    (0x9012, "OffsetTimeDigitized"),
    (0x9101, "ComponentsConfiguration"),
    (0x9102, "CompressedBitsPerPixel"),
    (0x9201, "ShutterSpeedValue"),
    (0x9202, "ApertureValue"),
    (0x9203, "BrightnessValue"),
    (0x9204, "ExposureBiasValue"),
    (0x9205, "MaxApertureValue"),
    (0x9206, "SubjectDistance"),
    (0x9207, "MeteringMode"),
    (0x9208, "LightSource"),
    (0x9209, "Flash"),
    (0x920A, "FocalLength"),
    (0x9214, "SubjectArea"),
    (0x927C, "MakerNote"),
    (0x9286, "UserComment"),
    (0x9287, "LearningOptOutIn"),
    (0x9290, "SubSecTime"),
    (0x9291, "SubSecTimeOriginal"),
    (0x9292, "SubSecTimeDigitized"),
    (0x9400, "Temperature"),
    (0x9401, "Humidity"),
    (0x9402, "Pressure"),
    (0x9403, "WaterDepth"),
    (0x9404, "Acceleration"),
    (0x9405, "CameraElevationAngle"),
    (0xA000, "FlashpixVersion"),
    (0xA001, "ColorSpace"),
    (0xA002, "PixelXDimension"),
    (0xA003, "PixelYDimension"),
    (0xA004, "RelatedSoundFile"),
    (0xA005, "Interoperability IFD Pointer"),
    (0xA20B, "FlashEnergy"),
    (0xA20C, "SpatialFrequencyResponse"),
    (0xA20E, "FocalPlaneXResolution"),
    (0xA20F, "FocalPlaneYResolution"),
    (0xA210, "FocalPlaneResolutionUnit"),
    (0xA214, "SubjectLocation"),
    (0xA215, "ExposureIndex"),
    (0xA217, "SensingMethod"),
    (0xA300, "FileSource"),
    (0xA301, "SceneType"),
    (0xA302, "CFAPattern"),
    (0xA401, "CustomRendered"),
    (0xA402, "ExposureMode"),
    (0xA403, "WhiteBalance"),
    (0xA404, "DigitalZoomRatio"),
    (0xA405, "FocalLengthIn35mmFilm"),
    (0xA406, "SceneCaptureType"),
    (0xA407, "GainControl"),
    (0xA408, "Contrast"),
    (0xA409, "Saturation"),
    (0xA40A, "Sharpness"),
    (0xA40B, "DeviceSettingDescription"),
    (0xA40C, "SubjectDistanceRange"),
    (0xA40D, "DevelopmentType"),
    (0xA40E, "DevelopmentTypeDescription"),
    (0xA40F, "DistortionCorrection"),
    (0xA410, "ChromaticAberrationCorrection"),
    (0xA411, "ShadingCorrection"),
    (0xA412, "NoiseReduction"),
    (0xA420, "ImageUniqueID"),
    (0xA430, "CameraOwnerName"),
    (0xA431, "BodySerialNumber"),
    (0xA432, "LensSpecification"),
    (0xA433, "LensMake"),
    (0xA434, "LensModel"),
    (0xA435, "LensSerialNumber"),
    (0xA436, "ImageTitle"),
    (0xA437, "Photographer"),
    (0xA438, "ImageEditor"),
    (0xA439, "CameraFirmware"),
    (0xA43A, "RAWDevelopingSoftware"),
    (0xA43B, "ImageEditingSoftware"),
    (0xA43C, "MetadataEditingSoftware"),
    (0xA460, "CompositeImage"),
    (0xA461, "SourceImageNumberOfCompositeImage"),
    (0xA462, "SourceExposureTimesOfCompositeImage"),
    (0xA500, "Gamma"),
];

/// The names of the tags of the GPS IFD, by tag.
const GPS: &[(u16, &str)] = &[
    (0x0000, "GPSVersionID"),
    (0x0001, "GPSLatitudeRef"),
    (0x0002, "GPSLatitude"),
    (0x0003, "GPSLongitudeRef"),
    (0x0004, "GPSLongitude"),
    (0x0005, "GPSAltitudeRef"),
    (0x0006, "GPSAltitude"),
    (0x0007, "GPSTimeStamp"),
    (0x0008, "GPSSatellites"),
    (0x0009, "GPSStatus"),
    (0x000A, "GPSMeasureMode"),
    (0x000B, "GPSDOP"),
    (0x000C, "GPSSpeedRef"),
    (0x000D, "GPSSpeed"),
    (0x000E, "GPSTrackRef"),
    (0x000F, "GPSTrack"),
    (0x0010, "GPSImgDirectionRef"),
    (0x0011, "GPSImgDirection"),
    (0x0012, "GPSMapDatum"),
    (0x0013, "GPSDestLatitudeRef"),
    (0x0014, "GPSDestLatitude"),
    (0x0015, "GPSDestLongitudeRef"),
    (0x0016, "GPSDestLongitude"),
    (0x0017, "GPSDestBearingRef"),
    (0x0018, "GPSDestBearing"),
    (0x0019, "GPSDestDistanceRef"),
    (0x001A, "GPSDestDistance"),
    (0x001B, "GPSProcessingMethod"),
    (0x001C, "GPSAreaInformation"),
    (0x001D, "GPSDateStamp"),
    (0x001E, "GPSDifferential"),
    (0x001F, "GPSHPositioningError"),
];

/// The names of the tags of the Interoperability IFD, by tag.
const INTEROP: &[(u16, &str)] = &[
    (0x0001, "InteroperabilityIndex"),
    (0x0002, "InteroperabilityVersion"),
    (0x1000, "RelatedImageFileFormat"),
    (0x1001, "RelatedImageWidth"),
    (0x1002, "RelatedImageLength"),
];

/// The names of the tags of the MP Index IFD, by tag.
const MP_INDEX: &[(u16, &str)] = &[
    (0xB000, "MPFVersion"),
    (0xB001, "NumberOfImages"),
    (0xB002, "MPEntry"),
    (0xB003, "ImageUIDList"),
    (0xB004, "TotalFrames"),
];

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: [(&[(u16, &str)], usize); 5] = [
        (CHAIN, 80),
        (EXIF, 96),
        (GPS, 32),
        (INTEROP, 5),
        (MP_INDEX, 5),
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
        assert_eq!(TABLES.iter().map(|(_, count)| count).sum::<usize>(), 218);
    }

    #[test]
    fn a_tag_is_named_in_its_kind_of_directory() {
        for (directory, tag, expected) in [
            (Directory::Ifd(0), 0x010F, "Make"),
            (Directory::Ifd(1), 0x0201, "JPEGInterchangeFormat"),
            (Directory::SubIfd(2), 0x0202, "JPEGInterchangeFormatLength"),
            (Directory::Ifd(0), 0x0107, "Threshholding"),
            (Directory::Ifd(0), 0x8769, "Exif IFD Pointer"),
            (Directory::Ifd(0), 0x8825, "GPS Info IFD Pointer"),
            (Directory::Ifd(0), 0xC634, "DNGPrivateData"),
            (Directory::Exif, 0x9003, "DateTimeOriginal"),
            (Directory::Exif, 0x927C, "MakerNote"),
            (Directory::Exif, 0xA005, "Interoperability IFD Pointer"),
            (Directory::Exif, 0xA500, "Gamma"),
            (Directory::Gps, 0x0000, "GPSVersionID"),
            (Directory::Gps, 0x001F, "GPSHPositioningError"),
            (Directory::Interop, 0x0001, "InteroperabilityIndex"),
            (Directory::Interop, 0x1002, "RelatedImageLength"),
            (Directory::MpIndex, 0xB002, "MPEntry"),
        ] {
            assert_eq!(
                name(directory, tag),
                Some(expected),
                "{directory} {tag:#06X}"
            );
        }
    }

    #[test]
    fn a_tag_no_document_names_in_that_kind_of_directory_has_no_name() {
        for (directory, tag) in [
            // Exif's tags written in IFD0, and TIFF's in the Exif IFD.
            (Directory::Ifd(0), 0x9003),
            (Directory::Ifd(0), 0xA401),
            (Directory::Exif, 0x010D),
            // Numbers shared between kinds: a GPS tag outside the GPS IFD,
            // an Interoperability tag outside its IFD.
            (Directory::Exif, 0x0001),
            (Directory::Ifd(0), 0x0000),
            (Directory::Gps, 0x1001),
            // Tags no document read names: SubIFDs, XMP, Sony's and
            // Nikon's own, and the DNG tags raw-files does not read.
            (Directory::Ifd(0), 0x014A),
            (Directory::Ifd(0), 0x02BC),
            (Directory::SubIfd(0), 0x7000),
            (Directory::SubIfd(0), 0xC7D5),
            (Directory::Ifd(0), 0xC61D),
            (Directory::MpIndex, 0xB005),
        ] {
            assert_eq!(name(directory, tag), None, "{directory} {tag:#06X}");
        }
    }
}
