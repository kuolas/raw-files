// SPDX-License-Identifier: MIT OR Apache-2.0

//! The directories a file links to that the raw view does not list:
//! [`SkippedDirectory`], and [`Lookup`], what a reader found where one of
//! its own links leads.

use super::Directory;
use crate::error::ErrorKind;

/// A directory the raw view does not list: a link that a listed directory
/// holds, which led to no directory raw-files could read, and why.
///
/// [`Entries::skipped_directories`](crate::Entries::skipped_directories)
/// lists them, so a caller can tell that a directory was there. A
/// directory can be left out for one of these causes, each seen by its
/// [`tag`](Self::tag), [`offset`](Self::offset) and [`kind`](Self::kind):
///
/// - **A link into the TIFF header, or past the largest offset.** No
///   directory can start in the header's 8 bytes. `Malformed`; the offset
///   is where the link leads, or `None` for a position past the largest
///   offset, which no `u64` holds.
/// - **Bytes that are not a directory.** The link leads to bytes that
///   cannot be read as a table of entries: one that would pass the end of
///   the file, or of the part of it the directory is read in, such as a
///   Nikon PreviewIFD put past the MakerNote's declared length, or one with
///   no entries. `Malformed`, or `LimitExceeded` when a cap of
///   [`Limits`](crate::Limits) refused it, such as a count of entries over
///   [`Limits::max_ifd_entries`](crate::Limits::max_ifd_entries).
/// - **A directory with an entry that cannot be checked**, of an unknown
///   type or with its value outside the file: a SubIFD, or Sony's private
///   directory, is left out whole. `Malformed`.
/// - **A link whose value cannot be read**: the offset is `None`. A
///   SubIFDs value longer than
///   [`Limits::max_value_bytes`](crate::Limits::max_value_bytes) is
///   `LimitExceeded`. An Exif, GPS or Interoperability IFD tag that is not
///   one SHORT, LONG or IFD is `Malformed`: raw-files reads each only so.
///   So is a Nikon MakerNote's PreviewIFD tag (0x0011) that is not one
///   LONG, the one shape Nikon writes it in. An entry that cannot be read
///   at all, of a type TIFF does not define or with its value outside the
///   file, is no link: it is counted by
///   [`Entries::skipped_entries`](crate::Entries::skipped_entries).
/// - **The SubIFDs of a directory at the depth cap**,
///   [`Limits::max_ifd_depth`](crate::Limits::max_ifd_depth): each one is
///   listed, `LimitExceeded`. No camera file nests that deep.
///
/// The Sony DSLR-A100 writes the position of its raw data where IFD0's
/// SubIFDs tag (0x014A) and its pointer to Sony's private directory
/// (0xC634) lead, and the bytes there are no directory. Every file of that
/// body lists both: the default limits refuse them for their count of
/// entries (`LimitExceeded`), and limits that allow it find entries of
/// types TIFF does not define (`Malformed`).
///
/// A link to a directory already listed is not one: the directory is
/// listed once, under the first link that led to it. Within the TIFF
/// structure, a position is listed once, at the first link that led to it,
/// and a link with no position once for the directory and tag that hold
/// it. A MakerNote's directories and Sony's private directory are read
/// apart from that structure, so one of them is listed even when a link of
/// the structure leads to the same position: the DSLR-A100's two below
/// lead to the same bytes. Nor is a pointer to the next directory of 0 one,
/// which means there is none. Nor, but for the Exif, GPS and
/// Interoperability IFD tags and Nikon's PreviewIFD tag, is an entry whose
/// type or count is not the one raw-files reads that link in, such as an
/// IFD0 DNGPrivateData tag (0xC634) that is not the four BYTEs of Sony's
/// pointer to its private directory: a DNG writes a block of its maker's
/// own data, of any length, under that tag, which is no damage. The MP
/// Index IFD of a JPEG file is not listed when it cannot be read: no
/// directory links to it.
///
/// It has no public constructor: only a file gives one.
///
/// # Examples
///
/// ```no_run
/// use raw_files::{CameraFile, ErrorKind};
///
/// fn main() -> raw_files::Result<()> {
///     let file = CameraFile::open("DSC_0001.ARW")?;
///     for skipped in file.entries().skipped_directories() {
///         let link = match skipped.tag() {
///             Some(tag) => format!("tag {tag:#06X}"),
///             None => "its next-directory pointer".to_owned(),
///         };
///         let hint = match skipped.kind() {
///             ErrorKind::LimitExceeded => " (a larger limit may list it)",
///             _ => "",
///         };
///         println!("{} links through {link} to a directory left out{hint}", skipped.parent());
///     }
///     Ok(())
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkippedDirectory {
    parent: Directory,
    tag: Option<u16>,
    offset: Option<u64>,
    kind: ErrorKind,
}

impl SkippedDirectory {
    /// A directory left out: the link lies in `parent`, in the entry of
    /// `tag` or, with `None`, in its next-directory pointer; it leads to
    /// `offset` in the file, if that is known; `kind` says why.
    pub(crate) fn new(
        parent: Directory,
        tag: Option<u16>,
        offset: Option<u64>,
        kind: ErrorKind,
    ) -> Self {
        Self {
            parent,
            tag,
            offset,
            kind,
        }
    }

    /// The listed directory that holds the link, such as
    /// [`Directory::Ifd(0)`](Directory::Ifd) for IFD0's SubIFDs tag.
    pub fn parent(&self) -> Directory {
        self.parent
    }

    /// The tag of the entry that holds the link, such as
    /// [`tags::tiff::SUB_IFDS`](crate::tags::tiff::SUB_IFDS). `None` for
    /// the pointer to the next directory, which follows a directory's
    /// entries and has no tag.
    pub fn tag(&self) -> Option<u16> {
        self.tag
    }

    /// Where the link leads in the file: the position the directory was to
    /// start at. `None` when the link's value could not be read, or names a
    /// position past the largest offset.
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Why the directory is left out: `LimitExceeded` when a cap of
    /// [`Limits`](crate::Limits) stopped the reader, so that a larger limit
    /// may list it; `Malformed` when the file contradicts its format there.
    /// The same kind as opening a file gives for the same cause.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

/// What a reader found where one of its own links leads, such as a
/// MakerNote entry or Sony's pointer to its private directory.
#[derive(Debug)]
pub(crate) enum Lookup<T> {
    /// No such link: there is nothing to list.
    Absent,
    /// The link led to no directory the reader could read: the raw view
    /// lists it as skipped.
    Skipped(SkippedDirectory),
    /// What the link led to, read.
    Found(T),
}

impl<T> Lookup<T> {
    /// What the link led to; `None` when there is no link, or it led to
    /// nothing that could be read.
    pub(crate) fn found(self) -> Option<T> {
        match self {
            Self::Found(found) => Some(found),
            Self::Absent | Self::Skipped(_) => None,
        }
    }

    /// The same lookup, borrowing what it found.
    pub(crate) fn as_ref(&self) -> Lookup<&T> {
        match self {
            Self::Absent => Lookup::Absent,
            Self::Skipped(skipped) => Lookup::Skipped(*skipped),
            Self::Found(found) => Lookup::Found(found),
        }
    }

    /// The same lookup, with `f` applied to what it found.
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> Lookup<U> {
        match self {
            Self::Absent => Lookup::Absent,
            Self::Skipped(skipped) => Lookup::Skipped(skipped),
            Self::Found(found) => Lookup::Found(f(found)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skipped_directory_gives_what_it_was_built_with() {
        let skipped = SkippedDirectory::new(
            Directory::Ifd(0),
            Some(0x014A),
            Some(1024),
            ErrorKind::Malformed,
        );
        assert_eq!(skipped.parent(), Directory::Ifd(0));
        assert_eq!(skipped.tag(), Some(0x014A));
        assert_eq!(skipped.offset(), Some(1024));
        assert_eq!(skipped.kind(), ErrorKind::Malformed);

        let next = SkippedDirectory::new(Directory::Ifd(1), None, None, ErrorKind::LimitExceeded);
        assert_eq!(next.tag(), None);
        assert_eq!(next.offset(), None);
        assert_eq!(next.kind(), ErrorKind::LimitExceeded);
    }

    #[test]
    fn a_lookup_gives_what_it_found() {
        let skipped =
            SkippedDirectory::new(Directory::Exif, Some(0x927C), None, ErrorKind::Malformed);
        assert_eq!(Lookup::Found(3).found(), Some(3));
        assert_eq!(Lookup::<u8>::Absent.found(), None);
        assert_eq!(Lookup::<u8>::Skipped(skipped).found(), None);
        assert_eq!(Lookup::Found(3).map(|n| n * 2).found(), Some(6));
        assert!(matches!(
            Lookup::<u8>::Skipped(skipped).map(|n| n * 2),
            Lookup::Skipped(s) if s == skipped
        ));
        assert!(matches!(Lookup::<u8>::Absent.as_ref(), Lookup::Absent));
        assert_eq!(Lookup::Found(3).as_ref().found(), Some(&3));
    }
}
