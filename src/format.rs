// SPDX-License-Identifier: MIT OR Apache-2.0

//! The formats of the camera files raw-files reads: RAW files, and the
//! JPEG files cameras write.

use std::fmt;

/// A format of the camera files raw-files can open: a RAW format, or JPEG,
/// the format of the pictures cameras write beside their raw files or
/// instead of them.
///
/// `Format` is `#[non_exhaustive]`: formats are added in later releases, so
/// match on it with a wildcard arm. It orders as its variants are listed,
/// and a new variant is added after the existing ones, so an order once
/// observed holds.
///
/// Opening a file never looks at its extension: the content is sniffed, so a
/// renamed file opens as what it is. [`from_extension`](Format::from_extension)
/// exists for file pickers and folder scans, which must decide what to offer
/// before opening anything.
///
/// # Examples
///
/// ```
/// use raw_files::Format;
///
/// assert_eq!(Format::from_extension(".NEF"), Some(Format::Nef));
/// assert_eq!(Format::from_extension(".ARW"), Some(Format::Arw));
/// assert_eq!(Format::from_extension("sr2"), Some(Format::Arw));
/// assert_eq!(Format::from_extension("jpg"), Some(Format::Jpeg));
/// assert_eq!(Format::Nef.to_string(), "Nikon NEF");
/// assert_eq!(Format::Arw.to_string(), "Sony ARW");
/// assert_eq!(Format::Jpeg.to_string(), "JPEG");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Nikon NEF, including the NRW files of Coolpix cameras: a TIFF file,
    /// not a DNG, whose first directory's Make names Nikon.
    Nef,
    /// Sony ARW, with the SRF and SR2 files of Sony's earliest bodies and
    /// the ARQ pixel-shift composites made by Sony's software: a TIFF file,
    /// not a DNG, whose first directory's Make names Sony, or names
    /// Hasselblad for the Sony-built Hasselblad bodies (the Lunar among
    /// them), which write Sony's structure.
    Arw,
    /// JPEG: a JPEG file as a camera writes it beside its raw files or
    /// instead of them, or as an editor exports it. It holds no raw data.
    ///
    /// Its previews are the file itself and the smaller images it carries:
    /// the thumbnail of its Exif segment, the images its Multi-Picture
    /// Format (MPF) list holds after the main one, and, from a Nikon
    /// camera that writes one, such as the D70, the small preview of its
    /// MakerNote.
    ///
    /// Its metadata is what its Exif segment records, read as in a raw
    /// file: the camera, its serial number, the capture time, the
    /// exposure, the lens and the orientation; from a Nikon camera, also
    /// what its MakerNote records where the Exif segment records nothing,
    /// as in a NEF. Its image size is the size of the picture, and it has
    /// no raw size.
    ///
    /// Its focus point comes from the MakerNote of a Nikon camera, placed
    /// on the picture on a crop mode, or of a Sony camera, as in an ARW. It
    /// is given only when its frame has the picture's shape.
    Jpeg,
}

impl Format {
    /// Every format, in a fixed order.
    ///
    /// Detection tries the formats in this order, asking only those that
    /// the file's container, TIFF or JPEG, can hold.
    pub fn all() -> &'static [Format] {
        &[Format::Nef, Format::Arw, Format::Jpeg]
    }

    /// The name of the format as a person would say it, for example in a
    /// file picker: "Nikon NEF".
    pub fn name(self) -> &'static str {
        match self {
            Format::Nef => "Nikon NEF",
            Format::Arw => "Sony ARW",
            Format::Jpeg => "JPEG",
        }
    }

    /// The file extensions that files of this format carry, in lower case and
    /// without the leading dot.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Nef => &["nef", "nrw"],
            Format::Arw => &["arw", "srf", "sr2", "arq"],
            Format::Jpeg => &["jpg", "jpeg"],
        }
    }

    /// Whether files of this format hold a camera's raw sensor data, as NEF
    /// and ARW files do and JPEG files do not.
    ///
    /// A program that treats raw files and other images apart, such as a
    /// browser that pairs a raw file with the JPEG of the same shot, can ask
    /// this instead of matching every variant of this `#[non_exhaustive]`
    /// enum, which may gain formats that hold no raw data. It is `true` for
    /// NEF and ARW, and `false` for JPEG. It says nothing of the container or
    /// of the codecs that compress the data.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::Format;
    ///
    /// assert!(Format::Nef.is_raw());
    /// assert!(Format::from_extension("arw").is_some_and(Format::is_raw));
    /// assert!(!Format::Jpeg.is_raw());
    /// assert_eq!(Format::from_extension("jpg").map(Format::is_raw), Some(false));
    /// ```
    #[must_use]
    pub fn is_raw(self) -> bool {
        // No wildcard arm: a format added later has to say here whether it
        // holds raw data.
        match self {
            Format::Nef | Format::Arw => true,
            Format::Jpeg => false,
        }
    }

    /// The format whose files carry the given extension, if any.
    ///
    /// The comparison ignores case, and the extension may be given with or
    /// without its leading dot: `"NEF"`, `".nef"` and `"nef"` all name the
    /// same format. Anything else, including the empty string and a lone dot,
    /// gives `None`.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Format> {
        let ext = ext.strip_prefix('.').unwrap_or(ext);
        Format::all().iter().copied().find(|format| {
            format
                .extensions()
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
        })
    }
}

impl fmt::Display for Format {
    /// Writes [`name`](Format::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_format_once_with_well_formed_names() {
        assert_eq!(Format::all(), &[Format::Nef, Format::Arw, Format::Jpeg]);
        for format in Format::all() {
            assert!(!format.name().is_empty());
            assert!(!format.extensions().is_empty());
            for ext in format.extensions() {
                assert!(!ext.is_empty());
                assert_eq!(*ext, ext.to_ascii_lowercase());
                assert!(!ext.contains('.'));
            }
        }
    }

    #[test]
    fn extensions_are_those_of_each_formats_files() {
        assert_eq!(Format::Nef.extensions(), &["nef", "nrw"]);
        assert_eq!(Format::Arw.extensions(), &["arw", "srf", "sr2", "arq"]);
        assert_eq!(Format::Jpeg.extensions(), &["jpg", "jpeg"]);
    }

    #[test]
    fn is_raw_holds_for_nef_and_arw_and_not_for_jpeg() {
        assert!(Format::Nef.is_raw());
        assert!(Format::Arw.is_raw());
        assert!(!Format::Jpeg.is_raw());
    }

    #[test]
    fn from_extension_round_trips_every_listed_extension() {
        for &format in Format::all() {
            for ext in format.extensions() {
                assert_eq!(Format::from_extension(ext), Some(format));
                assert_eq!(Format::from_extension(&ext.to_uppercase()), Some(format));
                assert_eq!(Format::from_extension(&format!(".{ext}")), Some(format));
            }
        }
    }

    #[test]
    fn from_extension_rejects_everything_else() {
        for ext in [
            "", ".", "jp", "jpgx", ".jpe", "nef ", "..nef", "arw ", "..arw", "jpg ", "..jpg",
        ] {
            assert_eq!(Format::from_extension(ext), None, "{ext:?}");
        }
    }

    #[test]
    fn display_is_the_name() {
        assert_eq!(Format::Nef.to_string(), "Nikon NEF");
        assert_eq!(Format::Nef.to_string(), Format::Nef.name());
        assert_eq!(Format::Arw.to_string(), "Sony ARW");
        assert_eq!(Format::Arw.to_string(), Format::Arw.name());
        assert_eq!(Format::Jpeg.to_string(), "JPEG");
        assert_eq!(Format::Jpeg.to_string(), Format::Jpeg.name());
    }
}
