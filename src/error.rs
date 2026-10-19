// SPDX-License-Identifier: MIT OR Apache-2.0

//! The error type: what went wrong, by category, and where in the file.

use std::borrow::Cow;
use std::fmt;
use std::io;

/// A specialised [`Result`](std::result::Result) whose error defaults to
/// raw-files' [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error from reading a RAW file.
///
/// [`kind()`](Error::kind) says what went wrong, by category, and
/// [`offset()`](Error::offset) says where in the file, when that is known.
/// An error that came from the reader keeps the [`io::Error`] as its
/// [`source()`](std::error::Error::source).
///
/// # Examples
///
/// ```
/// use std::io::{self, Read};
///
/// use raw_files::{ErrorKind, Result};
///
/// fn first_byte(mut reader: impl Read) -> Result<u8> {
///     let mut byte = [0];
///     reader.read_exact(&mut byte)?;
///     Ok(byte[0])
/// }
///
/// let err = first_byte(io::empty()).unwrap_err();
/// match err.kind() {
///     ErrorKind::Io => println!("could not read the file: {err}"),
///     ErrorKind::Malformed => println!("the file is damaged: {err}"),
///     _ => println!("cannot read this file: {err}"),
/// }
/// assert_eq!(err.kind(), ErrorKind::Io);
/// assert_eq!(err.offset(), None);
/// ```
// Boxed so that the error is one pointer wide and `Result<T>` stays small.
pub struct Error(Box<Inner>);

struct Inner {
    kind: ErrorKind,
    message: Cow<'static, str>,
    offset: Option<u64>,
    source: Option<io::Error>,
}

/// The category of an [`Error`].
///
/// More categories may be added, so a `match` on this type needs a wildcard
/// arm. The categories order as they are listed, and a new category is
/// added after the existing ones, so an order once observed holds.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorKind {
    /// The reader failed, including when the file shrank while it was being
    /// read.
    Io,
    /// The file is not a RAW file that this version of raw-files recognises.
    UnknownFormat,
    /// The file was recognised, but this variant of its format, or this
    /// feature of it, is not supported.
    Unsupported,
    /// The file contradicts itself or its format.
    Malformed,
    /// Reading the file would cross a limit set by `Limits`, or needs more
    /// memory than this platform can give.
    LimitExceeded,
}

impl Error {
    fn new(
        kind: ErrorKind,
        message: Cow<'static, str>,
        offset: Option<u64>,
        source: Option<io::Error>,
    ) -> Self {
        Self(Box::new(Inner {
            kind,
            message,
            offset,
            source,
        }))
    }

    /// Returns the category of this error.
    pub fn kind(&self) -> ErrorKind {
        self.0.kind
    }

    /// Returns the byte offset in the file where the problem was found, when
    /// it is known.
    pub fn offset(&self) -> Option<u64> {
        self.0.offset
    }
}

// The constructors the rest of the crate builds errors with. No format
// module reports `Unsupported` yet; the `expect` fails the build once that
// constructor has a caller, so it is then removed rather than left behind.
impl Error {
    /// The file contradicts itself or its format at byte `offset`.
    pub(crate) fn malformed(message: impl Into<Cow<'static, str>>, offset: u64) -> Self {
        Self::new(ErrorKind::Malformed, message.into(), Some(offset), None)
    }

    /// The file is not a RAW file this version recognises.
    pub(crate) fn unknown_format(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::UnknownFormat, message.into(), None, None)
    }

    /// The file is recognised, but this variant or feature is not supported.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "no format module reports this error yet")
    )]
    pub(crate) fn unsupported(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Unsupported, message.into(), None, None)
    }

    /// Reading on from byte `offset` would cross a limit.
    pub(crate) fn limit(message: impl Into<Cow<'static, str>>, offset: u64) -> Self {
        Self::new(ErrorKind::LimitExceeded, message.into(), Some(offset), None)
    }

    /// The reader failed with `source` while reading at byte `offset`.
    ///
    /// For a reader error whose place in the file matters, such as the file
    /// ending early because it shrank after it was opened. Other reader
    /// errors convert with `From<io::Error>`, without an offset, so that they
    /// convert back to the original [`io::Error`].
    pub(crate) fn io_at(source: io::Error, offset: u64) -> Self {
        Self::new(
            ErrorKind::Io,
            Cow::Borrowed("I/O error"),
            Some(offset),
            Some(source),
        )
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.0.kind)
            .field("message", &self.0.message)
            .field("offset", &self.0.offset)
            .field("source", &self.0.source)
            .finish()
    }
}

// The source's own text is left out: callers reach it through `source()`.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.message)?;
        if let Some(offset) = self.0.offset {
            write!(f, " at byte {offset}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0
            .source
            .as_ref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

/// Converts a reader's error to an [`ErrorKind::Io`] error without an offset,
/// keeping the [`io::Error`] as its source.
///
/// An [`io::Error`] that wraps a raw-files error, as converting an [`Error`]
/// to an [`io::Error`] does, gives that error back unchanged, with its kind
/// and offset.
impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        match source.downcast::<Error>() {
            Ok(err) => err,
            Err(source) => Self::new(
                ErrorKind::Io,
                Cow::Borrowed("I/O error"),
                None,
                Some(source),
            ),
        }
    }
}

/// Converts the error to an [`io::Error`], for callers working in terms of
/// `io::Result`.
///
/// An error that came from the reader converts back to the original
/// [`io::Error`], unchanged, unless it carries a byte offset. Otherwise the
/// raw-files error is wrapped, and stays reachable through
/// [`io::Error::get_ref`]. The wrapper's [`io::ErrorKind`] is the reader's own
/// for [`ErrorKind::Io`], [`Unsupported`](io::ErrorKind::Unsupported) for
/// [`ErrorKind::Unsupported`], and
/// [`InvalidData`](io::ErrorKind::InvalidData) otherwise.
impl From<Error> for io::Error {
    fn from(mut err: Error) -> Self {
        if err.0.kind == ErrorKind::Io
            && err.0.offset.is_none()
            && let Some(source) = err.0.source.take()
        {
            return source;
        }
        // No wildcard arm: a new `ErrorKind` needs its own decision here.
        let kind = match err.0.kind {
            ErrorKind::Io => err
                .0
                .source
                .as_ref()
                .map_or(io::ErrorKind::Other, io::Error::kind),
            ErrorKind::Unsupported => io::ErrorKind::Unsupported,
            ErrorKind::UnknownFormat | ErrorKind::Malformed | ErrorKind::LimitExceeded => {
                io::ErrorKind::InvalidData
            }
        };
        io::Error::new(kind, err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    const fn assert_traits<T: Send + Sync + 'static>() {}
    const _: () = assert_traits::<Error>();

    #[test]
    fn result_is_small() {
        assert!(size_of::<Result<u64>>() <= 16);
    }

    #[test]
    fn display_without_offset() {
        let err = Error::unsupported("lossy compression");
        assert_eq!(err.to_string(), "lossy compression");
    }

    #[test]
    fn display_with_offset() {
        let err = Error::malformed(String::from("IFD runs past the end"), 4096);
        assert_eq!(err.to_string(), "IFD runs past the end at byte 4096");
    }

    #[test]
    fn display_leaves_out_the_source() {
        let err = Error::from(io::Error::other("disk on fire"));
        assert_eq!(err.to_string(), "I/O error");
    }

    #[test]
    fn getters() {
        let err = Error::malformed("bad tag", 12);
        assert_eq!(err.kind(), ErrorKind::Malformed);
        assert_eq!(err.offset(), Some(12));

        let err = Error::limit("preview too large", 1 << 40);
        assert_eq!(err.kind(), ErrorKind::LimitExceeded);
        assert_eq!(err.offset(), Some(1 << 40));

        let err = Error::unknown_format("not a TIFF file");
        assert_eq!(err.kind(), ErrorKind::UnknownFormat);
        assert_eq!(err.offset(), None);

        let err = Error::unsupported("lossy compression");
        assert_eq!(err.kind(), ErrorKind::Unsupported);
        assert_eq!(err.offset(), None);

        let err = Error::from(io::Error::from(io::ErrorKind::UnexpectedEof));
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.offset(), None);
    }

    #[test]
    fn source_is_the_io_error() {
        let err = Error::from(io::Error::other("disk on fire"));
        let source = err.source().expect("an I/O error keeps its source");
        let source = source
            .downcast_ref::<io::Error>()
            .expect("the source is the io::Error");
        assert_eq!(source.kind(), io::ErrorKind::Other);
        assert_eq!(source.to_string(), "disk on fire");
    }

    #[test]
    fn constructed_error_has_no_source() {
        assert!(Error::malformed("bad tag", 12).source().is_none());
    }

    fn into_io(err: Error) -> io::Error {
        err.into()
    }

    #[test]
    fn into_io_error_kinds() {
        let cases = [
            (Error::malformed("m", 1), io::ErrorKind::InvalidData),
            (Error::unknown_format("u"), io::ErrorKind::InvalidData),
            (Error::limit("l", 2), io::ErrorKind::InvalidData),
            (Error::unsupported("s"), io::ErrorKind::Unsupported),
            (
                Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)),
                io::ErrorKind::UnexpectedEof,
            ),
            (
                Error::new(ErrorKind::Io, Cow::Borrowed("i"), None, None),
                io::ErrorKind::Other,
            ),
        ];
        for (err, expected) in cases {
            let kind = err.kind();
            assert_eq!(into_io(err).kind(), expected, "{kind:?}");
        }
    }

    #[test]
    fn into_io_error_wraps_our_error() {
        let io_err = into_io(Error::malformed("bad tag", 12));
        let inner = io_err
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<Error>())
            .expect("the io::Error wraps our Error");
        assert_eq!(inner.kind(), ErrorKind::Malformed);
        assert_eq!(inner.offset(), Some(12));
        assert_eq!(io_err.to_string(), "bad tag at byte 12");
    }

    #[test]
    fn into_io_error_returns_the_original() {
        let io_err = into_io(Error::from(io::Error::other("disk on fire")));
        assert_eq!(io_err.to_string(), "disk on fire");
        assert_eq!(io_err.kind(), io::ErrorKind::Other);
        assert!(
            io_err
                .get_ref()
                .and_then(|inner| inner.downcast_ref::<Error>())
                .is_none(),
            "the original is not wrapped"
        );

        let io_err = into_io(Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)));
        assert_eq!(io_err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn io_at_keeps_the_offset_and_the_source() {
        let err = Error::io_at(io::Error::from(io::ErrorKind::UnexpectedEof), 7);
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.offset(), Some(7));
        assert_eq!(err.to_string(), "I/O error at byte 7");
        let source = err
            .source()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .expect("the source is the io::Error");
        assert_eq!(source.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn into_io_error_keeps_the_offset() {
        let err = Error::io_at(io::Error::from(io::ErrorKind::UnexpectedEof), 7);
        let io_err = into_io(err);
        assert_eq!(io_err.kind(), io::ErrorKind::UnexpectedEof);
        let inner = io_err
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<Error>())
            .expect("the io::Error wraps our Error");
        assert_eq!(inner.offset(), Some(7));
    }

    #[test]
    fn a_wrapped_error_converts_back_unchanged() {
        let err = Error::from(into_io(Error::malformed("IFD runs past the end", 4096)));
        assert_eq!(err.kind(), ErrorKind::Malformed);
        assert_eq!(err.offset(), Some(4096));
        assert_eq!(err.to_string(), "IFD runs past the end at byte 4096");
        assert!(err.source().is_none(), "{err:?}");
    }
    #[test]
    fn a_wrapped_io_error_keeps_its_offset() {
        let err = Error::io_at(io::Error::from(io::ErrorKind::UnexpectedEof), 7);
        let err = Error::from(into_io(err));
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.offset(), Some(7));
        assert_eq!(err.to_string(), "I/O error at byte 7");
        let source = err
            .source()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .expect("the source is still the reader's io::Error");
        assert_eq!(source.kind(), io::ErrorKind::UnexpectedEof);
    }
}
