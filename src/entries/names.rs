// SPDX-License-Identifier: MIT OR Apache-2.0

//! The name of an entry's tag, from the table of [`crate::tags`] for its
//! kind of directory.
//!
//! A tag's name depends on the kind of directory it lies in, not on its
//! number alone: the GPS IFD's numbers repeat other directories', and
//! Exif names its own tags in the Exif IFD only, not where a camera writes
//! one of them in IFD0. So an entry gets a name only where a specification
//! names its tag in its kind of directory.

use super::Directory;
use crate::tags;

/// The name a published specification gives `tag` in a directory of the
/// kind `directory` is; `None` when none of the documents names it there.
pub(super) fn name(directory: Directory, tag: u16) -> Option<&'static str> {
    let table = match directory {
        Directory::Ifd(_) | Directory::SubIfd(_) => tags::tiff::NAMES,
        Directory::Exif => tags::exif::NAMES,
        Directory::Gps => tags::gps::NAMES,
        Directory::Interop => tags::interop::NAMES,
        Directory::MpIndex => tags::mp_index::NAMES,
        // A maker's own directories: no standard names their tags.
        Directory::MakerNote | Directory::MakerNoteSub(_) | Directory::Private => return None,
    };
    let at = table.binary_search_by_key(&tag, |&(tag, _)| tag).ok()?;
    table.get(at).map(|&(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_names_its_constant_in_a_directory_of_its_kind() {
        for (directory, table) in [
            (Directory::Ifd(0), tags::tiff::NAMES),
            (Directory::SubIfd(0), tags::tiff::NAMES),
            (Directory::Exif, tags::exif::NAMES),
            (Directory::Gps, tags::gps::NAMES),
            (Directory::Interop, tags::interop::NAMES),
            (Directory::MpIndex, tags::mp_index::NAMES),
        ] {
            for &(tag, expected) in table {
                assert_eq!(
                    name(directory, tag),
                    Some(expected),
                    "{directory} {tag:#06X}"
                );
            }
        }
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
            (Directory::Ifd(0), 0x014A, "SubIFDs"),
            (Directory::SubIfd(0), 0x014A, "SubIFDs"),
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
            // Tags no document read names: XMP, Sony's and Nikon's own,
            // and the DNG tags raw-files does not read.
            (Directory::Ifd(0), 0x02BC),
            (Directory::SubIfd(0), 0x7000),
            (Directory::SubIfd(0), 0xC7D5),
            (Directory::Ifd(0), 0xC61D),
            (Directory::MpIndex, 0xB005),
            // A maker's directories, whatever the tag: not even one a
            // standard names elsewhere, as Make in IFD0.
            (Directory::MakerNote, 0x010F),
            (Directory::MakerNote, 0x0011),
            (Directory::MakerNoteSub(0x0011), 0x0201),
            (Directory::Private, 0x7200),
            (Directory::Private, 0x0100),
        ] {
            assert_eq!(name(directory, tag), None, "{directory} {tag:#06X}");
        }
    }
}
