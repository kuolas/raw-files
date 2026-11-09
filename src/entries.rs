// SPDX-License-Identifier: MIT OR Apache-2.0

//! The raw view: every entry of every TIFF directory a file holds, as the
//! file records it, beneath the values raw-files reads from them.
//!
//! The NEF, ARW and camera JPEG files raw-files reads are built from TIFF
//! directories, each a table of entries: a tag number, a field type, a
//! count of values and the values themselves, or where they lie. Opening a
//! file keeps every entry of every directory it walks, as an [`Entry`] of
//! an [`Entries`]; an entry's [`Value`] is read only when asked for, as a
//! preview's bytes are. A [`Directory`] names which directory an entry lies
//! in, and a [`FieldType`] how its values are stored.
//!
//! An entry the directory's reader could not check is left out and counted
//! per directory: one whose type is none of those [`FieldType`] names, whose
//! length overflows, or whose value lies outside the file or the part of it
//! the directory was read in. So every entry listed can be read.

use std::fmt;
use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::{self, Ifd};
use crate::error::Result;
use crate::io::{ByteOrder, Source};

/// Which directory of a file an entry lies in, named by what led the reader
/// to it.
///
/// The directories that repeat are numbered in the order the reader finds
/// them, which is the same for every file a camera writes the same way, so
/// a directory and a tag name one entry across those files: IFD0 is
/// `Ifd(0)`, the directory chained after it `Ifd(1)`, and the SubIFDs are
/// numbered from 0 as they are found, each followed by its own SubIFDs.
///
/// A directory reached through two links, such as an IFD1 that is also the
/// Interoperability IFD, is listed once, under the first that led to it:
/// the chain, the SubIFDs and the Exif IFD are followed before the GPS and
/// Interoperability IFDs.
///
/// New variants may be added as raw-files reads more kinds of directory.
///
/// `Display` writes the name a viewer shows: "IFD0", "SubIFD0", "Exif",
/// "GPS", "Interop" and "MP Index".
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Directory {
    /// A directory of the chain that starts at the file's first: `Ifd(0)`
    /// is IFD0, which every TIFF structure has, and `Ifd(1)` the directory
    /// IFD0 points to as the next, often a thumbnail's.
    Ifd(u32),
    /// A SubIFD, a directory that the SubIFDs tag (0x014A) of a directory
    /// of the chain or of another SubIFD points to, numbered from 0 in the
    /// order they are found: the SubIFDs of IFD0, depth first, then those
    /// of IFD1, and so on. A RAW file keeps its raw data and its larger
    /// previews in them.
    SubIfd(u32),
    /// The Exif IFD, which IFD0's tag 0x8769 points to: how the picture was
    /// taken.
    Exif,
    /// The GPS IFD, which IFD0's tag 0x8825 points to: where the picture
    /// was taken.
    Gps,
    /// The Interoperability IFD, which the Exif IFD's tag 0xA005 points
    /// to: the rule the file follows.
    Interop,
    /// The MP Index IFD of a JPEG file's Multi-Picture Format segment: the
    /// images the file holds.
    MpIndex,
}

impl fmt::Display for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ifd(n) => write!(f, "IFD{n}"),
            Self::SubIfd(n) => write!(f, "SubIFD{n}"),
            Self::Exif => f.write_str("Exif"),
            Self::Gps => f.write_str("GPS"),
            Self::Interop => f.write_str("Interop"),
            Self::MpIndex => f.write_str("MP Index"),
        }
    }
}

/// How an entry's values are stored: what one value is and how many bytes
/// it takes.
///
/// TIFF 6.0 defines the first twelve; [`Ifd`](Self::Ifd) is the type some
/// files give the offsets of other directories, and [`Utf8`](Self::Utf8)
/// is Exif's type for text in UTF-8.
///
/// New variants may be added if a standard defines more types.
///
/// `Display` writes the type's name as the standards write it, such as
/// "SHORT" or "UTF-8"; [`Ifd`](Self::Ifd), which none of the standards
/// raw-files follows names, is "IFD".
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

    /// The type's code, as an entry records it.
    pub fn code(self) -> u16 {
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
    pub fn size(self) -> u32 {
        match self {
            Self::Byte | Self::Ascii | Self::SByte | Self::Undefined | Self::Utf8 => 1,
            Self::Short | Self::SShort => 2,
            Self::Long | Self::SLong | Self::Float | Self::Ifd => 4,
            Self::Rational | Self::SRational | Self::Double => 8,
        }
    }

    /// The type's name, as [`Display`](fmt::Display) writes it.
    fn name(self) -> &'static str {
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

/// One entry of a directory, as the file records it: its tag, its field
/// type, how many values it holds and where they lie.
///
/// An `Entry` describes the value; it does not hold it.
/// `RawFile::read_value` reads it. Like a
/// [`Preview`](crate::Preview), an entry is a small `Copy` value that only
/// a file gives.
///
/// Every entry listed has been checked when the file was opened: its type
/// is one [`FieldType`] names, its length did not overflow, and its value
/// lies inside the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entry {
    directory: Directory,
    /// The byte order of the directory's numbers, which its values share.
    order: ByteOrder,
    entry: tiff::Entry,
}

impl Entry {
    /// The directory the entry lies in.
    pub fn directory(&self) -> Directory {
        self.directory
    }

    /// The entry's tag: what the value means, by the number the file uses.
    pub fn tag(&self) -> u16 {
        self.entry.tag()
    }

    /// How the entry's values are stored.
    pub fn field_type(&self) -> FieldType {
        self.entry.field_type()
    }

    /// How many values the entry holds, as recorded: for text, its bytes,
    /// the NUL included.
    pub fn count(&self) -> u32 {
        self.entry.count()
    }

    /// The length of the value in bytes: the count times the size of one
    /// value of the type.
    pub fn value_len(&self) -> u32 {
        self.entry.value_len()
    }

    /// Where the value's bytes lie in the file; `None` when the entry holds
    /// its value itself, as it does a value of four bytes or fewer.
    ///
    /// A caller that holds the file in memory can borrow the bytes of a
    /// value from it by this range rather than read them.
    pub fn byte_range(&self) -> Option<Range<u64>> {
        let start = self.entry.value_offset()?;
        // Checked when the file was opened: the value lies inside it.
        Some(start..start.saturating_add(u64::from(self.value_len())))
    }

    /// The entry's value, read from `src` under a cap of `cap` bytes, in
    /// the byte order of its directory.
    ///
    /// # Errors
    ///
    /// As for [`tiff::Entry::read`].
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing reads values yet"))]
    pub(crate) fn read<R: Read + Seek>(&self, src: &mut Source<R>, cap: u64) -> Result<Value> {
        let bytes = self.entry.read(src, cap)?;
        Ok(Value::decode(self.field_type(), self.order, bytes))
    }
}

/// An entry's values, as the file records them, each variant holding the
/// values of one [`FieldType`] in the byte order of the entry's directory.
///
/// Nothing is interpreted: a RATIONAL or SRATIONAL is a pair, numerator
/// first, with a zero denominator kept; text is its bytes, with its NUL and
/// any padding, as [`bytes`](Self::bytes) gives them and
/// [`text`](Self::text) cleans them.
///
/// New variants may be added with new field types.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// BYTE values.
    Byte(Vec<u8>),
    /// ASCII text, as its bytes.
    Ascii(Vec<u8>),
    /// SHORT values.
    Short(Vec<u16>),
    /// LONG values.
    Long(Vec<u32>),
    /// RATIONAL values, as `(numerator, denominator)` pairs.
    Rational(Vec<(u32, u32)>),
    /// SBYTE values.
    SByte(Vec<i8>),
    /// UNDEFINED bytes.
    Undefined(Vec<u8>),
    /// SSHORT values.
    SShort(Vec<i16>),
    /// SLONG values.
    SLong(Vec<i32>),
    /// SRATIONAL values, as `(numerator, denominator)` pairs.
    SRational(Vec<(i32, i32)>),
    /// FLOAT values.
    Float(Vec<f32>),
    /// DOUBLE values.
    Double(Vec<f64>),
    /// IFD values: the offsets of directories, as written.
    Ifd(Vec<u32>),
    /// UTF-8 text, as its bytes.
    Utf8(Vec<u8>),
}

impl Value {
    /// The values of type `field_type` that `bytes` holds, in byte order
    /// `order`. `bytes` is a whole number of values, as an entry's value
    /// is; a partial value at the end would be left out.
    ///
    /// Bytes are kept as they are; numbers are decoded into one new vector
    /// each, as long as `bytes`, so the most held at once is twice the
    /// value's length.
    fn decode(field_type: FieldType, order: ByteOrder, bytes: Vec<u8>) -> Self {
        let shorts = |bytes: &[u8]| -> Vec<u16> {
            let (chunks, _) = bytes.as_chunks::<2>();
            chunks.iter().map(|&v| order.u16(v)).collect()
        };
        let longs = |bytes: &[u8]| -> Vec<u32> {
            let (chunks, _) = bytes.as_chunks::<4>();
            chunks.iter().map(|&v| order.u32(v)).collect()
        };
        let pairs = |bytes: &[u8]| -> Vec<(u32, u32)> {
            let (chunks, _) = bytes.as_chunks::<8>();
            chunks
                .iter()
                .map(|&[n0, n1, n2, n3, d0, d1, d2, d3]| {
                    (order.u32([n0, n1, n2, n3]), order.u32([d0, d1, d2, d3]))
                })
                .collect()
        };
        match field_type {
            FieldType::Byte => Self::Byte(bytes),
            FieldType::Ascii => Self::Ascii(bytes),
            FieldType::Undefined => Self::Undefined(bytes),
            FieldType::Utf8 => Self::Utf8(bytes),
            FieldType::SByte => Self::SByte(bytes.iter().map(|b| b.cast_signed()).collect()),
            FieldType::Short => Self::Short(shorts(&bytes)),
            FieldType::SShort => {
                let (chunks, _) = bytes.as_chunks::<2>();
                Self::SShort(chunks.iter().map(|&v| order.u16(v).cast_signed()).collect())
            }
            FieldType::Long => Self::Long(longs(&bytes)),
            FieldType::Ifd => Self::Ifd(longs(&bytes)),
            FieldType::SLong => {
                let (chunks, _) = bytes.as_chunks::<4>();
                Self::SLong(chunks.iter().map(|&v| order.u32(v).cast_signed()).collect())
            }
            FieldType::Float => {
                let (chunks, _) = bytes.as_chunks::<4>();
                Self::Float(
                    chunks
                        .iter()
                        .map(|&v| f32::from_bits(order.u32(v)))
                        .collect(),
                )
            }
            FieldType::Rational => Self::Rational(pairs(&bytes)),
            FieldType::SRational => Self::SRational(
                bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|&[n0, n1, n2, n3, d0, d1, d2, d3]| {
                        (
                            order.u32([n0, n1, n2, n3]).cast_signed(),
                            order.u32([d0, d1, d2, d3]).cast_signed(),
                        )
                    })
                    .collect(),
            ),
            FieldType::Double => {
                let (chunks, _) = bytes.as_chunks::<8>();
                Self::Double(
                    chunks
                        .iter()
                        .map(|&v| f64::from_bits(order.u64(v)))
                        .collect(),
                )
            }
        }
    }

    /// The field type of the values.
    pub fn field_type(&self) -> FieldType {
        match self {
            Self::Byte(_) => FieldType::Byte,
            Self::Ascii(_) => FieldType::Ascii,
            Self::Short(_) => FieldType::Short,
            Self::Long(_) => FieldType::Long,
            Self::Rational(_) => FieldType::Rational,
            Self::SByte(_) => FieldType::SByte,
            Self::Undefined(_) => FieldType::Undefined,
            Self::SShort(_) => FieldType::SShort,
            Self::SLong(_) => FieldType::SLong,
            Self::SRational(_) => FieldType::SRational,
            Self::Float(_) => FieldType::Float,
            Self::Double(_) => FieldType::Double,
            Self::Ifd(_) => FieldType::Ifd,
            Self::Utf8(_) => FieldType::Utf8,
        }
    }

    /// How many values there are: the entry's count. For text, its bytes.
    pub fn len(&self) -> usize {
        match self {
            Self::Byte(v) | Self::Ascii(v) | Self::Undefined(v) | Self::Utf8(v) => v.len(),
            Self::Short(v) => v.len(),
            Self::Long(v) | Self::Ifd(v) => v.len(),
            Self::Rational(v) => v.len(),
            Self::SByte(v) => v.len(),
            Self::SShort(v) => v.len(),
            Self::SLong(v) => v.len(),
            Self::SRational(v) => v.len(),
            Self::Float(v) => v.len(),
            Self::Double(v) => v.len(),
        }
    }

    /// Whether there are no values: an entry whose count is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bytes of a value made of bytes, as the file stores them: BYTE,
    /// UNDEFINED, ASCII and UTF-8, text with its NUL and any padding.
    /// `None` for the other types.
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Byte(v) | Self::Ascii(v) | Self::Undefined(v) | Self::Utf8(v) => Some(v),
            _ => None,
        }
    }

    /// The text of an ASCII or UTF-8 value, cleaned as raw-files cleans
    /// every text it reads: cut at the first NUL, spaces at either end
    /// dropped, and bytes that are not UTF-8 replaced by U+FFFD. `None` for
    /// the other types, and for text that is empty once cleaned.
    ///
    /// Unlike the text values of [`Metadata`](crate::Metadata), which
    /// refuse text longer than 255 bytes as damage, this gives text of any
    /// length.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::Value;
    ///
    /// let make = Value::Ascii(b"SONY       \0".to_vec());
    /// assert_eq!(make.text().as_deref(), Some("SONY"));
    /// assert_eq!(Value::Ascii(b"   \0".to_vec()).text(), None);
    /// assert_eq!(Value::Short(vec![1]).text(), None);
    /// ```
    pub fn text(&self) -> Option<String> {
        match self {
            Self::Ascii(bytes) | Self::Utf8(bytes) => clean_text(bytes),
            _ => None,
        }
    }
}

/// The text that `bytes` holds, cleaned: cut at the first NUL, spaces at
/// either end dropped, and bytes that are not UTF-8 replaced by U+FFFD;
/// `None` when nothing is left.
///
/// Only spaces are padding: any other blank, such as a tab, is kept, and
/// so are spaces inside the text.
pub(crate) fn clean_text(bytes: &[u8]) -> Option<String> {
    let bytes = bytes
        .iter()
        .position(|&b| b == 0)
        .map_or(bytes, |nul| bytes.get(..nul).unwrap_or_default());
    let start = bytes.iter().position(|&b| b != b' ').unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| b != b' ')
        .map_or(start, |i| i + 1);
    let text = bytes.get(start..end).unwrap_or_default();
    (!text.is_empty()).then(|| String::from_utf8_lossy(text).into_owned())
}

/// Every entry of every directory a file holds, in the file's order:
/// directory by directory, in the order they were found, and within one
/// directory, in the order it lists its entries, which need not be sorted
/// by tag.
///
/// `RawFile::entries` gives them. A tag that a
/// directory lists twice is listed twice; [`entry`](Self::entry) finds the
/// first, as raw-files' own readers do.
///
/// An entry that could not be checked when the file was opened is left
/// out, and [`skipped`](Self::skipped) counts it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entries {
    entries: Vec<Entry>,
    /// Each directory found, in the order it was found, with the range of
    /// its entries in `entries`.
    directories: Vec<Found>,
}

/// One directory of [`Entries`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Found {
    directory: Directory,
    /// Its entries' positions in [`Entries::entries`].
    entries: Range<usize>,
    /// How many of its entries were left out.
    skipped: u32,
}

impl Entries {
    /// No entries, from no directory.
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing lists the entries yet"))]
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            directories: Vec::new(),
        }
    }

    /// Adds the entries of `ifd`, a directory found as `directory`, after
    /// those of the directories added before, with the number its reader
    /// left out.
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing lists the entries yet"))]
    pub(crate) fn push(&mut self, directory: Directory, ifd: &Ifd) {
        let start = self.entries.len();
        let order = ifd.order();
        self.entries
            .extend(ifd.entries().iter().map(|&entry| Entry {
                directory,
                order,
                entry,
            }));
        self.directories.push(Found {
            directory,
            entries: start..self.entries.len(),
            skipped: ifd.invalid_entries(),
        });
    }

    /// Every entry, in the file's order.
    pub fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }

    /// The number of entries, of every directory.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no entries: the file holds no directory raw-files
    /// reads, as a JPEG without an Exif segment.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every directory found, in the order it was found, each once, and
    /// whether or not any of its entries could be listed.
    pub fn directories(&self) -> impl Iterator<Item = Directory> + '_ {
        self.directories.iter().map(|found| found.directory)
    }

    /// The entries of `directory`, in the order it lists them; none when
    /// the file has no such directory.
    pub fn in_directory(&self, directory: Directory) -> std::slice::Iter<'_, Entry> {
        self.found(directory)
            .and_then(|found| self.entries.get(found.entries.clone()))
            .unwrap_or_default()
            .iter()
    }

    /// The entry of `tag` in `directory`; when the directory lists the tag
    /// more than once, the first, in the order it lists them. `None` when
    /// the file has no such directory, or it has no such entry that could
    /// be listed.
    pub fn entry(&self, directory: Directory, tag: u16) -> Option<Entry> {
        self.in_directory(directory)
            .find(|entry| entry.tag() == tag)
            .copied()
    }

    /// How many entries of `directory` were left out because they could
    /// not be checked: an unknown type, a length that overflows, or a value
    /// outside the file or the part of it the directory was read in. 0 for
    /// a directory the file does not have.
    pub fn skipped(&self, directory: Directory) -> u32 {
        self.found(directory).map_or(0, |found| found.skipped)
    }

    /// The directory `directory`, if it was found.
    fn found(&self, directory: Directory) -> Option<&Found> {
        self.directories
            .iter()
            .find(|found| found.directory == directory)
    }
}

impl<'a> IntoIterator for &'a Entries {
    type Item = &'a Entry;
    type IntoIter = std::slice::Iter<'a, Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::container::tiff::{Block, parse_ifd};
    use crate::limits::Limits;
    use crate::testing::assert_ordered;
    use crate::tiff_builder::{Built, IfdId, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

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
            assert_eq!(field_type.code(), codes[i], "{field_type:?}");
            assert_eq!(field_type.size(), sizes[i], "{field_type:?}");
            assert_eq!(field_type.to_string(), names[i], "{field_type:?}");
            assert_eq!(FieldType::from_code(codes[i]), Some(field_type));
        }
        assert_ordered(&TYPES);
    }

    #[test]
    fn only_the_known_codes_name_a_type() {
        let known = (0..=u16::MAX)
            .filter_map(FieldType::from_code)
            .map(FieldType::code);
        assert!(known.eq([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 129]));
    }

    #[test]
    fn directories_are_named_and_ordered() {
        let all = [
            Directory::Ifd(0),
            Directory::Ifd(1),
            Directory::Ifd(12),
            Directory::SubIfd(0),
            Directory::SubIfd(3),
            Directory::Exif,
            Directory::Gps,
            Directory::Interop,
            Directory::MpIndex,
        ];
        let names: Vec<String> = all.iter().map(ToString::to_string).collect();
        assert_eq!(
            names,
            [
                "IFD0", "IFD1", "IFD12", "SubIFD0", "SubIFD3", "Exif", "GPS", "Interop", "MP Index"
            ]
        );
        assert_ordered(&all);
    }

    /// The bytes of `value` in byte order `order`, as a file stores them,
    /// written here apart from [`Value::decode`], so that the two check each
    /// other.
    fn encode(value: &Value, order: Order) -> Vec<u8> {
        let little = order == Order::Little;
        let u16s = |v: u16| {
            if little {
                v.to_le_bytes()
            } else {
                v.to_be_bytes()
            }
        };
        let u32s = |v: u32| {
            if little {
                v.to_le_bytes()
            } else {
                v.to_be_bytes()
            }
        };
        let u64s = |v: u64| {
            if little {
                v.to_le_bytes()
            } else {
                v.to_be_bytes()
            }
        };
        match value {
            Value::Byte(v) | Value::Ascii(v) | Value::Undefined(v) | Value::Utf8(v) => v.clone(),
            Value::SByte(v) => v.iter().map(|b| b.cast_unsigned()).collect(),
            Value::Short(v) => v.iter().flat_map(|&n| u16s(n)).collect(),
            Value::SShort(v) => v.iter().flat_map(|&n| u16s(n.cast_unsigned())).collect(),
            Value::Long(v) | Value::Ifd(v) => v.iter().flat_map(|&n| u32s(n)).collect(),
            Value::SLong(v) => v.iter().flat_map(|&n| u32s(n.cast_unsigned())).collect(),
            Value::Rational(v) => v
                .iter()
                .flat_map(|&(n, d)| [u32s(n), u32s(d)])
                .flatten()
                .collect(),
            Value::SRational(v) => v
                .iter()
                .flat_map(|&(n, d)| [u32s(n.cast_unsigned()), u32s(d.cast_unsigned())])
                .flatten()
                .collect(),
            Value::Float(v) => v.iter().flat_map(|&n| u32s(n.to_bits())).collect(),
            Value::Double(v) => v.iter().flat_map(|&n| u64s(n.to_bits())).collect(),
        }
    }

    /// A value of every type held in the entry, where the type allows one
    /// of four bytes or fewer, and one stored out of line.
    fn values() -> Vec<Value> {
        vec![
            Value::Byte(vec![1, 0xFE]),
            Value::Byte((1..=9).collect()),
            Value::Ascii(b"ab\0".to_vec()),
            Value::Ascii(b"a longer text\0".to_vec()),
            Value::Short(vec![0x0102, 0xFFFE]),
            Value::Short(vec![1, 2, 0x0304]),
            Value::Long(vec![0x0102_0304]),
            Value::Long(vec![1, u32::MAX]),
            Value::Rational(vec![(1, 3), (5, 0)]),
            Value::SByte(vec![-1, 2, i8::MIN]),
            Value::SByte(vec![-128, -1, 0, 1, 127]),
            Value::Undefined(vec![0, 1, 2]),
            Value::Undefined((10..19).collect()),
            Value::SShort(vec![-2, 3]),
            Value::SShort(vec![-1, -2, i16::MIN]),
            Value::SLong(vec![-5]),
            Value::SLong(vec![-1, i32::MIN]),
            Value::SRational(vec![(-1, 3), (2, -5)]),
            Value::Float(vec![1.5]),
            Value::Float(vec![-0.25, f32::MAX]),
            Value::Double(vec![3.25]),
            Value::Double(vec![-1e300, 0.5]),
            Value::Ifd(vec![8]),
            Value::Ifd(vec![8, 0x0001_0000]),
            Value::Utf8("é\0".as_bytes().to_vec()),
            Value::Utf8("Zoë Lüthi\0".as_bytes().to_vec()),
        ]
    }

    /// A file in byte order `order` whose one IFD holds `values`, tagged
    /// from 1 in order, each in its entry when it fits there and otherwise
    /// at a place of its own.
    fn file_of(values: &[Value], order: Order) -> (Built, IfdId) {
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        for (i, value) in (1..).zip(values) {
            let bytes = encode(value, order);
            let count = u32::try_from(value.len()).unwrap();
            let code = value.field_type().code();
            if bytes.len() <= 4 {
                // Left-justified, as TIFF holds a value in its entry.
                let mut field = [0; 4];
                field[..bytes.len()].copy_from_slice(&bytes);
                builder.raw(ifd, i, code, count, RawValue::Inline(field));
            } else {
                let at = 1024 + 64 * u32::from(i);
                builder.raw(ifd, i, code, count, RawValue::Offset(at));
                builder.blob_at(u64::from(at), &bytes);
            }
        }
        (builder.build(), ifd)
    }

    fn source(bytes: &[u8]) -> Source<Cursor<Vec<u8>>> {
        Source::new(Cursor::new(bytes.to_vec())).unwrap()
    }

    /// The entries of the IFD `ifd` of `built`, listed as `directory`.
    fn entries_of(built: &Built, ifd: IfdId, directory: Directory, order: Order) -> Entries {
        let mut src = source(&built.bytes);
        let order = match order {
            Order::Little => ByteOrder::Little,
            Order::Big => ByteOrder::Big,
        };
        let block = Block::file(src.len());
        let at = built.ifd_offset(ifd);
        let (parsed, _) = parse_ifd(&mut src, &block, at, order, &Limits::default()).unwrap();
        let mut entries = Entries::new();
        entries.push(directory, &parsed);
        entries
    }

    #[test]
    fn values_of_every_type_in_both_byte_orders_in_and_out_of_the_entry() {
        let expected = values();
        for order in ORDERS {
            let (built, ifd) = file_of(&expected, order);
            let entries = entries_of(&built, ifd, Directory::Ifd(0), order);
            assert_eq!(entries.len(), expected.len(), "{order:?}");
            let mut src = source(&built.bytes);
            for (entry, value) in entries.iter().zip(&expected) {
                let what = format!("{value:?} ({order:?})");
                assert_eq!(entry.field_type(), value.field_type(), "{what}");
                assert_eq!(entry.count(), u32::try_from(value.len()).unwrap(), "{what}");
                let len = u32::try_from(encode(value, order).len()).unwrap();
                assert_eq!(entry.value_len(), len, "{what}");
                assert_eq!(entry.byte_range().is_none(), len <= 4, "{what}");
                assert_eq!(&entry.read(&mut src, 1024).unwrap(), value, "{what}");
            }
        }
    }

    #[test]
    fn an_inline_value_is_read_without_the_file() {
        let expected = Value::SShort(vec![-2, 3]);
        let (built, ifd) = file_of(std::slice::from_ref(&expected), Order::Big);
        let entries = entries_of(&built, ifd, Directory::Exif, Order::Big);
        let entry = entries.entry(Directory::Exif, 1).unwrap();
        assert_eq!(entry.read(&mut source(&[]), 0).unwrap(), expected);
    }

    #[test]
    fn a_value_is_read_up_to_the_cap_and_refused_past_it() {
        let value = Value::Long(vec![1, 2, 3]);
        let (built, ifd) = file_of(std::slice::from_ref(&value), Order::Little);
        let entry = entries_of(&built, ifd, Directory::Ifd(0), Order::Little)
            .entry(Directory::Ifd(0), 1)
            .unwrap();
        let mut src = source(&built.bytes);
        assert_eq!(entry.read(&mut src, 12).unwrap(), value);
        let err = entry.read(&mut src, 11).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
        let start = entry.byte_range().unwrap().start;
        assert_eq!(err.offset(), Some(start), "{err:?}");
        // A shorter file, as an entry of another file may meet: its value
        // lies past the end.
        let shrunk = &built.bytes[..usize::try_from(start).unwrap() + 4];
        let err = entry.read(&mut source(shrunk), 12).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
    }

    #[test]
    fn a_byte_range_is_where_any_out_of_line_value_lies() {
        let (built, ifd) = file_of(&[Value::Short(vec![1, 2, 3])], Order::Little);
        let entry = entries_of(&built, ifd, Directory::Ifd(0), Order::Little)
            .entry(Directory::Ifd(0), 1)
            .unwrap();
        let start = 1024 + 64;
        assert_eq!(entry.byte_range(), Some(start..start + 6));
        assert_eq!(entry.directory(), Directory::Ifd(0));
        assert_eq!(entry.tag(), 1);
    }

    #[test]
    fn text_is_cut_at_the_nul_trimmed_of_spaces_and_lossy() {
        let text = |bytes: &[u8]| Value::Ascii(bytes.to_vec()).text();
        assert_eq!(text(b"NIKON\0garbage").as_deref(), Some("NIKON"));
        assert_eq!(text(b"SONY       \0").as_deref(), Some("SONY"));
        assert_eq!(text(b"  a b  ").as_deref(), Some("a b"));
        assert_eq!(text(b"\ttab\t\0").as_deref(), Some("\ttab\t"));
        assert_eq!(text(b"caf\xE9\0").as_deref(), Some("caf\u{FFFD}"));
        assert_eq!(text(b"   \0"), None);
        assert_eq!(text(b"\0NIKON"), None);
        assert_eq!(text(b""), None);
        let utf8 = Value::Utf8("  Zoë\0".as_bytes().to_vec());
        assert_eq!(utf8.text().as_deref(), Some("Zoë"));
        // Text of any length, unlike the metadata's 255 bytes.
        let long = vec![b'a'; 1000];
        assert_eq!(text(&long).map(|t| t.len()), Some(1000));
        assert_eq!(Value::Byte(b"abc".to_vec()).text(), None);
        assert_eq!(Value::Undefined(b"abc".to_vec()).text(), None);
    }

    #[test]
    fn bytes_are_given_for_the_types_made_of_bytes() {
        for value in values() {
            let made_of_bytes = matches!(
                value.field_type(),
                FieldType::Byte | FieldType::Undefined | FieldType::Ascii | FieldType::Utf8
            );
            assert_eq!(value.bytes().is_some(), made_of_bytes, "{value:?}");
            if let Some(bytes) = value.bytes() {
                assert_eq!(bytes.len(), value.len(), "{value:?}");
            }
            assert!(!value.is_empty(), "{value:?}");
        }
        assert_eq!(Value::Ascii(b"ab\0".to_vec()).bytes(), Some(&b"ab\0"[..]));
        assert!(Value::Short(Vec::new()).is_empty());
        assert_eq!(Value::Rational(vec![(1, 2), (3, 4)]).len(), 2);
    }

    #[test]
    fn every_value_has_its_type() {
        let types: Vec<FieldType> = values().iter().map(Value::field_type).collect();
        for field_type in TYPES {
            assert!(types.contains(&field_type), "{field_type:?}");
        }
    }

    #[test]
    fn entries_keep_the_directories_in_order_with_their_skipped_counts() {
        for order in ORDERS {
            // A first IFD of two good entries beside three that cannot be
            // read: a type no standard defines, a length that overflows,
            // and a value past the end of the file; a second IFD whose
            // every entry is bad; a third with a tag repeated.
            let mut builder = TiffBuilder::new(order);
            let first = builder.ifd();
            builder
                .short(first, 0x0001, &[1])
                .raw(first, 0x0002, 99, 1, RawValue::Inline([0; 4]))
                .raw(first, 0x0003, 4, 0x4000_0000, RawValue::Offset(8))
                .raw(first, 0x0004, 1, 16, RawValue::Offset(0xFFFF_0000))
                .short(first, 0x0005, &[5]);
            let second = builder.ifd();
            builder.raw(second, 0x0001, 0, 1, RawValue::Inline([0; 4]));
            let third = builder.ifd();
            builder
                .short(third, 0x0007, &[1])
                .short(third, 0x0007, &[2])
                .short(third, 0x0006, &[3]);
            builder.unsorted(third);
            let built = builder.build();

            let mut entries = entries_of(&built, first, Directory::Exif, order);
            let mut src = source(&built.bytes);
            let block = Block::file(src.len());
            let byte_order = match order {
                Order::Little => ByteOrder::Little,
                Order::Big => ByteOrder::Big,
            };
            for (ifd, directory) in [(second, Directory::Gps), (third, Directory::SubIfd(0))] {
                let at = built.ifd_offset(ifd);
                let (parsed, _) =
                    parse_ifd(&mut src, &block, at, byte_order, &Limits::default()).unwrap();
                entries.push(directory, &parsed);
            }

            let what = format!("{order:?}");
            let found: Vec<Directory> = entries.directories().collect();
            assert_eq!(
                found,
                [Directory::Exif, Directory::Gps, Directory::SubIfd(0)],
                "{what}"
            );
            assert_eq!(entries.skipped(Directory::Exif), 3, "{what}");
            assert_eq!(entries.skipped(Directory::Gps), 1, "{what}");
            assert_eq!(entries.skipped(Directory::SubIfd(0)), 0, "{what}");
            assert_eq!(entries.skipped(Directory::Interop), 0, "{what}");
            let tags = |directory| -> Vec<u16> {
                entries.in_directory(directory).map(Entry::tag).collect()
            };
            assert_eq!(tags(Directory::Exif), [1, 5], "{what}");
            assert!(tags(Directory::Gps).is_empty(), "{what}");
            assert_eq!(tags(Directory::SubIfd(0)), [7, 7, 6], "{what}");
            assert!(tags(Directory::Interop).is_empty(), "{what}");
            assert_eq!(entries.len(), 5, "{what}");
            assert!(!entries.is_empty());
            let all: Vec<(Directory, u16)> = entries
                .iter()
                .map(|entry| (entry.directory(), entry.tag()))
                .collect();
            let by_ref: Vec<(Directory, u16)> = (&entries)
                .into_iter()
                .map(|entry| (entry.directory(), entry.tag()))
                .collect();
            assert_eq!(all, by_ref, "{what}");
            assert_eq!(all.first(), Some(&(Directory::Exif, 1)), "{what}");

            // The first of a repeated tag, in the order the IFD lists them.
            let repeated = entries.entry(Directory::SubIfd(0), 7).unwrap();
            assert_eq!(repeated.read(&mut src, 64).unwrap(), Value::Short(vec![1]));
            assert_eq!(entries.entry(Directory::Exif, 2), None, "{what}");
            assert_eq!(entries.entry(Directory::Interop, 1), None, "{what}");
        }
    }

    #[test]
    fn no_entries_from_no_directory() {
        let entries = Entries::new();
        assert!(entries.is_empty());
        assert_eq!(entries.len(), 0);
        assert_eq!(entries.directories().count(), 0);
        assert_eq!(entries.in_directory(Directory::Ifd(0)).len(), 0);
        assert_eq!(entries.iter().len(), 0);
        assert_eq!(entries.skipped(Directory::Ifd(0)), 0);
    }
}
