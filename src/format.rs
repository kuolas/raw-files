// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RAW file formats raw-files knows.

use std::fmt;

/// A RAW file format that raw-files can open.
///
/// `Format` is `#[non_exhaustive]`: formats are added in later releases, so
/// match on it with a wildcard arm.
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
/// assert_eq!(Format::from_extension("jpg"), None);
/// assert_eq!(Format::Nef.to_string(), "Nikon NEF");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Nikon NEF, including the NRW files of Coolpix cameras.
    ///
    /// Recognition of NEF files arrives with the NEF reader.
    Nef,
}

impl Format {
    /// Every format, in a fixed order.
    ///
    /// Detection tries the formats in this order.
    pub fn all() -> &'static [Format] {
        &[Format::Nef]
    }

    /// The name of the format as a person would say it, for example in a
    /// file picker: "Nikon NEF".
    pub fn name(self) -> &'static str {
        match self {
            Format::Nef => "Nikon NEF",
        }
    }

    /// The file extensions that files of this format carry, in lower case and
    /// without the leading dot.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Nef => &["nef", "nrw"],
        }
    }

    /// The format whose files carry the given extension, if any.
    ///
    /// The comparison ignores case, and the extension may be given with or
    /// without its leading dot: `"NEF"`, `".nef"` and `"nef"` all name the
    /// same format. Anything else, including the empty string and a lone dot,
    /// gives `None`.
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
        assert_eq!(Format::all(), &[Format::Nef]);
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
        for ext in ["", ".", "jpg", ".jpg", "nef ", "..nef"] {
            assert_eq!(Format::from_extension(ext), None, "{ext:?}");
        }
    }

    #[test]
    fn display_is_the_name() {
        assert_eq!(Format::Nef.to_string(), "Nikon NEF");
        assert_eq!(Format::Nef.to_string(), Format::Nef.name());
    }
}
