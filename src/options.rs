// SPDX-License-Identifier: MIT OR Apache-2.0

//! The options a file is opened with.

use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use crate::error::Result;
use crate::file::RawFile;
use crate::limits::Limits;

/// How to open a camera file, a RAW file or a JPEG: today, with which
/// [`Limits`].
///
/// The defaults suit untrusted files, and opening with them is what
/// [`RawFile::open`], [`RawFile::from_bytes`] and [`RawFile::from_reader`]
/// do: `RawFile::open(path)` is `Options::new().open(path)`. Build an
/// `Options` to open files with other limits; more options will join them
/// as the library grows.
///
/// # Examples
///
/// ```
/// use raw_files::{ErrorKind, Limits, Options};
///
/// let limits = Limits::default().with_max_preview_bytes(16 * 1024 * 1024);
/// let options = Options::new().limits(limits);
/// let err = options.from_bytes(b"not a RAW file").unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::UnknownFormat);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Options {
    limits: Limits,
}

impl Options {
    /// The default options: [`Limits::default()`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the limits that files opened with these options are read
    /// under.
    #[must_use]
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Opens the file at `path` with these options.
    ///
    /// See [`RawFile::open`], which this is with the default options.
    ///
    /// # Errors
    ///
    /// `Io` if the file cannot be opened, and otherwise as for
    /// [`from_reader`](Self::from_reader).
    pub fn open(&self, path: impl AsRef<Path>) -> Result<RawFile<File>> {
        self.from_reader(File::open(path)?)
    }

    /// Opens a file that is already in memory with these options.
    ///
    /// See [`RawFile::from_bytes`], which this is with the default options.
    ///
    /// # Errors
    ///
    /// As for [`from_reader`](Self::from_reader).
    pub fn from_bytes<'a>(&self, bytes: &'a [u8]) -> Result<RawFile<Cursor<&'a [u8]>>> {
        self.from_reader(Cursor::new(bytes))
    }

    /// Opens the file that `reader` reads with these options.
    ///
    /// See [`RawFile::from_reader`], which this is with the default options.
    ///
    /// Reads go through the library's own block cache, which seeks before
    /// every read, so wrapping the reader in a [`std::io::BufReader`]
    /// gains nothing: the library seeks before every read and reads whole
    /// blocks, so the wrapper's buffer is never used. Pass the `File` or
    /// reader as it is.
    ///
    /// # Errors
    ///
    /// `UnknownFormat` if the file is not a RAW file or a JPEG file this
    /// version recognises; `Malformed` if its structure is damaged;
    /// `LimitExceeded` if reading its structure would cross one of the
    /// limits, though a limit crossed inside a JPEG file's Exif or MPF
    /// segment is not an error: it costs what that segment holds, the Exif
    /// values and thumbnail or the MPF images, as damage there does; `Io` if
    /// the reader fails.
    pub fn from_reader<R: Read + Seek>(&self, reader: R) -> Result<RawFile<R>> {
        RawFile::new(reader, self.limits.clone())
    }
}
