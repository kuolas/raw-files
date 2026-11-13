// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF field types: what kind of value an entry of an IFD holds, and
//! how many bytes one value takes.
//!
//! A reader names the types it takes as [`FieldType`]s, not as codes, so
//! that a type cannot stand where a tag is wanted. The types are those of
//! TIFF 6.0 (codes 1 to 12), TIFF Technical Note 1 (13) and Exif 3.0 (129,
//! text in UTF-8).
//!
//! `FieldType` is public: the raw view gives every entry's type as one,
//! and `entries` and the crate's root re-export it from here.

use std::fmt;

/// How an entry's values are stored: what one value is and how many bytes
/// it takes.
///
/// TIFF 6.0 defines the first twelve; [`Ifd`](Self::Ifd) is the type some
/// files give the offsets of other directories, and [`Utf8`](Self::Utf8)
/// is Exif's type for text in UTF-8.
///
/// New variants may be added if a standard defines more types.
///
/// `Display` writes the type's [`name`](Self::name) as the standards write
/// it, such as "SHORT" or "UTF-8"; [`Ifd`](Self::Ifd), which none of the
/// standards raw-files follows names, is "IFD".
///
/// # Examples
///
/// An entry's type and count give the length of its value, and whether it
/// fits in the entry itself, four bytes, or lies elsewhere in the file:
///
/// ```
/// use raw_files::FieldType;
///
/// let rational = FieldType::from_code(5).expect("code 5 is RATIONAL");
/// assert_eq!(rational.to_string(), "RATIONAL");
/// // Three RATIONALs, such as GPS latitude's degrees, minutes and
/// // seconds, take 24 bytes: too many to fit in the entry.
/// assert_eq!(3 * rational.size(), 24);
/// assert_eq!(2 * FieldType::Short.size(), 4);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldType {
    /// BYTE, code 1: 8-bit unsigned integers.
    Byte,
    /// ASCII, code 2: text, a byte per character, ending in a NUL.
    Ascii,
    /// SHORT, code 3: 16-bit unsigned integers.
    Short,
    /// LONG, code 4: 32-bit unsigned integers.
    Long,
    /// RATIONAL, code 5: fractions of two LONGs, numerator first.
    Rational,
    /// SBYTE, code 6: 8-bit signed integers.
    SByte,
    /// UNDEFINED, code 7: bytes that only the tag gives a meaning to.
    Undefined,
    /// SSHORT, code 8: 16-bit signed integers.
    SShort,
    /// SLONG, code 9: 32-bit signed integers.
    SLong,
    /// SRATIONAL, code 10: fractions of two SLONGs, numerator first.
    SRational,
    /// FLOAT, code 11: 32-bit IEEE floating-point numbers.
    Float,
    /// DOUBLE, code 12: 64-bit IEEE floating-point numbers.
    Double,
    /// IFD, code 13: the offsets of directories, as 32-bit unsigned
    /// integers.
    Ifd,
    /// UTF-8, code 129: text in UTF-8, ending in a NUL, with no byte order
    /// mark; its count is in bytes, not characters.
    Utf8,
}

impl FieldType {
    /// The type that an entry's type code `code` names: 1 to 13, or 129.
    /// `None` for any other code, which raw-files cannot read.
    /// [`to_code`](Self::to_code) gives a type's code back.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::FieldType;
    ///
    /// assert_eq!(FieldType::from_code(3), Some(FieldType::Short));
    /// assert_eq!(FieldType::from_code(129), Some(FieldType::Utf8));
    /// assert_eq!(FieldType::from_code(14), None);
    /// ```
    pub fn from_code(code: u16) -> Option<Self> {
        Some(match code {
            1 => Self::Byte,
            2 => Self::Ascii,
            3 => Self::Short,
            4 => Self::Long,
            5 => Self::Rational,
            6 => Self::SByte,
            7 => Self::Undefined,
            8 => Self::SShort,
            9 => Self::SLong,
            10 => Self::SRational,
            11 => Self::Float,
            12 => Self::Double,
            13 => Self::Ifd,
            129 => Self::Utf8,
            _ => return None,
        })
    }

    /// The type's code, as an entry records it: the code that
    /// [`from_code`](Self::from_code) takes.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::FieldType;
    ///
    /// assert_eq!(FieldType::Short.to_code(), 3);
    /// assert_eq!(FieldType::from_code(129).map(FieldType::to_code), Some(129));
    /// ```
    pub const fn to_code(self) -> u16 {
        match self {
            Self::Byte => 1,
            Self::Ascii => 2,
            Self::Short => 3,
            Self::Long => 4,
            Self::Rational => 5,
            Self::SByte => 6,
            Self::Undefined => 7,
            Self::SShort => 8,
            Self::SLong => 9,
            Self::SRational => 10,
            Self::Float => 11,
            Self::Double => 12,
            Self::Ifd => 13,
            Self::Utf8 => 129,
        }
    }

    /// The number of bytes one value of the type takes: 1, 2, 4 or 8.
    pub const fn size(self) -> u32 {
        match self {
            Self::Byte | Self::Ascii | Self::SByte | Self::Undefined | Self::Utf8 => 1,
            Self::Short | Self::SShort => 2,
            Self::Long | Self::SLong | Self::Float | Self::Ifd => 4,
            Self::Rational | Self::SRational | Self::Double => 8,
        }
    }

    /// The type's name as the standards write it, for a viewer that lists
    /// entries: "SHORT", "RATIONAL" or "UTF-8"; [`Ifd`](Self::Ifd), which
    /// none of the standards raw-files follows names, is "IFD".
    /// [`Display`](fmt::Display) writes the same.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::FieldType;
    ///
    /// assert_eq!(FieldType::SRational.name(), "SRATIONAL");
    /// assert_eq!(FieldType::Utf8.to_string(), FieldType::Utf8.name());
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Self::Byte => "BYTE",
            Self::Ascii => "ASCII",
            Self::Short => "SHORT",
            Self::Long => "LONG",
            Self::Rational => "RATIONAL",
            Self::SByte => "SBYTE",
            Self::Undefined => "UNDEFINED",
            Self::SShort => "SSHORT",
            Self::SLong => "SLONG",
            Self::SRational => "SRATIONAL",
            Self::Float => "FLOAT",
            Self::Double => "DOUBLE",
            Self::Ifd => "IFD",
            Self::Utf8 => "UTF-8",
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_ordered;

    /// Every field type, in declaration order.
    const TYPES: [FieldType; 14] = [
        FieldType::Byte,
        FieldType::Ascii,
        FieldType::Short,
        FieldType::Long,
        FieldType::Rational,
        FieldType::SByte,
        FieldType::Undefined,
        FieldType::SShort,
        FieldType::SLong,
        FieldType::SRational,
        FieldType::Float,
        FieldType::Double,
        FieldType::Ifd,
        FieldType::Utf8,
    ];

    #[test]
    fn field_types_have_their_codes_sizes_and_names() {
        let codes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 129];
        let sizes = [1, 1, 2, 4, 8, 1, 1, 2, 4, 8, 4, 8, 4, 1];
        let names = [
            "BYTE",
            "ASCII",
            "SHORT",
            "LONG",
            "RATIONAL",
            "SBYTE",
            "UNDEFINED",
            "SSHORT",
            "SLONG",
            "SRATIONAL",
            "FLOAT",
            "DOUBLE",
            "IFD",
            "UTF-8",
        ];
        for (i, field_type) in TYPES.into_iter().enumerate() {
            assert_eq!(field_type.to_code(), codes[i], "{field_type:?}");
            assert_eq!(field_type.size(), sizes[i], "{field_type:?}");
            assert_eq!(field_type.name(), names[i], "{field_type:?}");
            assert_eq!(field_type.to_string(), names[i], "{field_type:?}");
            assert_eq!(FieldType::from_code(codes[i]), Some(field_type));
        }
        assert_ordered(&TYPES);
    }

    #[test]
    fn only_the_known_codes_name_a_type() {
        let known = (0..=u16::MAX)
            .filter_map(FieldType::from_code)
            .map(FieldType::to_code);
        assert!(known.eq([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 129]));
    }
}
