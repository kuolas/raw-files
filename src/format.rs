// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RAW file formats raw-files knows.

use std::fmt;

/// A RAW file format that raw-files can open.
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
/// assert_eq!(Format::from_extension("jpg"), None);
/// assert_eq!(Format::Nef.to_string(), "Nikon NEF");
/// assert_eq!(Format::Arw.to_string(), "Sony ARW");
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
}

impl Format {
    /// Every format, in a fixed order.
    ///
    /// Detection tries the formats in this order, asking only those that
    /// the file's container, TIFF or JPEG, can hold.
    pub fn all() -> &'static [Format] {
        &[Format::Nef, Format::Arw]
    }

    /// The name of the format as a person would say it, for example in a
    /// file picker: "Nikon NEF".
    pub fn name(self) -> &'static str {
        match self {
            Format::Nef => "Nikon NEF",
            Format::Arw => "Sony ARW",
        }
    }

    /// The file extensions that files of this format carry, in lower case and
    /// without the leading dot.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Nef => &["nef", "nrw"],
            Format::Arw => &["arw", "srf", "sr2", "arq"],
        }
    }

    /// Whether files of this format hold a camera's raw sensor data, as NEF
    /// and ARW files do.
    ///
    /// A program that treats raw files and other images apart, such as a
    /// browser that pairs a raw file with the JPEG of the same shot, can ask
    /// this instead of matching every variant of this `#[non_exhaustive]`
    /// enum, which may gain formats that hold no raw data. It is `true` for
    /// both formats raw-files reads. It says nothing of the container or of
    /// the codecs that compress the data.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::Format;
    ///
    /// assert!(Format::Nef.is_raw());
    /// assert!(Format::from_extension("arw").is_some_and(Format::is_raw));
    /// ```
    #[must_use]
    pub fn is_raw(self) -> bool {
        // No wildcard arm: a format added later has to say here whether it
        // holds raw data.
        match self {
            Format::Nef | Format::Arw => true,
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
        assert_eq!(Format::all(), &[Format::Nef, Format::Arw]);
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
    }

    #[test]
    fn is_raw_holds_for_nef_arw_and_every_listed_format() {
        assert!(Format::Nef.is_raw());
        assert!(Format::Arw.is_raw());
        for format in Format::all() {
            assert!(format.is_raw(), "{format}");
        }
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
        for ext in ["", ".", "jpg", ".jpg", "nef ", "..nef", "arw ", "..arw"] {
            assert_eq!(Format::from_extension(ext), None, "{ext:?}");
        }
    }

    #[test]
    fn display_is_the_name() {
        assert_eq!(Format::Nef.to_string(), "Nikon NEF");
        assert_eq!(Format::Nef.to_string(), Format::Nef.name());
        assert_eq!(Format::Arw.to_string(), "Sony ARW");
        assert_eq!(Format::Arw.to_string(), Format::Arw.name());
    }
}
