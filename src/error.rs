// SPDX-License-Identifier: MIT OR Apache-2.0

//! The error type: what went wrong, by category, and where in the file;
//! and the crate's rule for damage, [`skip_damage`], which turns damage in
//! the file into `None` and keeps a failure of the reader as an error.

use std::borrow::Cow;
use std::fmt;
use std::io;

/// A specialized [`Result`](std::result::Result) whose error defaults to
/// raw-files' [`Error`].
///
/// # Examples
///
/// A function that opens a file returns `Result<T>`, and `?` passes the
/// error on. The error type can also be named, as for a constructor that
/// refuses its arguments:
///
/// ```
/// use raw_files::{ErrorKind, Rational, RawFile, Result, ValueError};
///
/// /// How many previews a file holds.
/// fn preview_count(bytes: &[u8]) -> Result<usize> {
///     Ok(RawFile::from_bytes(bytes)?.previews().len())
/// }
///
/// let err = preview_count(b"not a camera file").unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::UnknownFormat);
///
/// let refused: Result<Rational, ValueError> = Rational::new(1, 0);
/// assert_eq!(refused, Err(ValueError::ZeroDenominator));
/// ```
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error from reading a camera file, a RAW file or a JPEG.
///
/// [`kind()`](Error::kind) says what went wrong, by category, and
/// [`offset()`](Error::offset) says where in the file, when that is known.
/// An error that came from the reader keeps the [`io::Error`] as its
/// [`source()`](std::error::Error::source).
///
/// `Error` is neither `Clone` nor `PartialEq`, since the [`io::Error`] it
/// may keep is neither. Compare errors by their [`kind()`](Error::kind).
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
///
/// As text, [`name`](Self::name) and [`Display`](fmt::Display), a
/// category is a few lowercase words with no final period, such as
/// "malformed file", so that it reads in front of an error's own text.
/// An [`Error`]'s own text is its message, followed by the byte offset
/// when it is known; it does not name the category. An I/O error's own
/// message is "I/O error", the name of its category: what the reader
/// reported is the error's [`source()`](std::error::Error::source).
///
/// # Examples
///
/// Name the category before the error's own text, as a log line would:
///
/// ```
/// use raw_files::{ErrorKind, RawFile};
///
/// // A TIFF header whose first directory starts at byte 8, where the
/// // file ends.
/// let err = RawFile::from_bytes(b"II*\0\x08\0\0\0").unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::Malformed);
/// assert_eq!(
///     format!("{}: {err}", err.kind()),
///     "malformed file: data runs past the end of the file at byte 8",
/// );
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorKind {
    /// The reader failed, including when the file shrank while it was being
    /// read.
    Io,
    /// The file is not a RAW file or a JPEG file that this version of
    /// raw-files recognizes.
    UnknownFormat,
    /// The file was recognized, but this variant of its format, or this
    /// feature of it, is not supported.
    Unsupported,
    /// The file contradicts itself or its format.
    Malformed,
    /// Reading the file would cross a limit set by `Limits`, or needs more
    /// memory than this platform can give.
    LimitExceeded,
}

impl ErrorKind {
    /// The category's name, in lowercase and with no final period, for a
    /// log line or a table of counts: "I/O error", "unknown format",
    /// "unsupported file", "malformed file" or "limit exceeded".
    /// [`Display`](fmt::Display) writes the same.
    pub fn name(self) -> &'static str {
        match self {
            Self::Io => "I/O error",
            Self::UnknownFormat => "unknown format",
            Self::Unsupported => "unsupported file",
            Self::Malformed => "malformed file",
            Self::LimitExceeded => "limit exceeded",
        }
    }
}

impl fmt::Display for ErrorKind {
    /// Writes [`name`](ErrorKind::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
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

    /// The file is not a RAW file or a JPEG file this version recognizes.
    pub(crate) fn unknown_format(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::UnknownFormat, message.into(), None, None)
    }

    /// The file is recognized, but this variant or feature is not supported.
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

/// Turns damage in the file into `None`, keeping only a failure of the
/// reader as an error.
///
/// The rule wherever damage should cost only the part of the file it
/// touches. For example, past IFD0, an IFD that cannot be parsed or a tag
/// value that cannot be read costs the walk only what it would have led
/// to, the preview finder skips an IFD whose tags it cannot read, and a
/// NEF MakerNote that does not hold together costs only its own preview. An
/// `Io` error is not damage: it says nothing about the file, and the
/// reader may fail again on the next read.
pub(crate) fn skip_damage<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.kind() == ErrorKind::Io => Err(err),
        Err(_) => Ok(None),
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

    // Every category, in the order of its declaration. The `match` has no
    // wildcard arm, so a category added later does not compile until it is
    // listed here too, and the test of the names then fails until its name
    // is listed there.
    fn all_kinds() -> [ErrorKind; 5] {
        let all = [
            ErrorKind::Io,
            ErrorKind::UnknownFormat,
            ErrorKind::Unsupported,
            ErrorKind::Malformed,
            ErrorKind::LimitExceeded,
        ];
        for kind in all {
            match kind {
                ErrorKind::Io
                | ErrorKind::UnknownFormat
                | ErrorKind::Unsupported
                | ErrorKind::Malformed
                | ErrorKind::LimitExceeded => {}
            }
        }
        all
    }

    #[test]
    fn every_kind_name_is_the_one_documented_and_display_writes_it() {
        // The names are stable once released: a program may show, store or
        // count by them, so a changed one fails here. `assert_names` wants
        // sentence case; these are lowercase, to read before an error's
        // text, so they are checked here.
        let names: Vec<&str> = all_kinds().map(ErrorKind::name).to_vec();
        assert_eq!(
            names,
            [
                "I/O error",
                "unknown format",
                "unsupported file",
                "malformed file",
                "limit exceeded",
            ]
        );
        for kind in all_kinds() {
            let name = kind.name();
            assert_eq!(kind.to_string(), name, "{kind:?}");
            assert!(name.is_ascii(), "{kind:?}");
            assert!(!name.ends_with('.'), "{kind:?}");
            // No capital letter, but those of the abbreviation "I/O".
            let rest = name.strip_prefix("I/O").unwrap_or(name);
            assert!(!rest.contains(|c: char| c.is_ascii_uppercase()), "{kind:?}");
        }
        let distinct: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(distinct.len(), names.len(), "{names:?}");
    }

    #[test]
    fn kinds_order_as_they_are_listed() {
        crate::testing::assert_ordered(&all_kinds());
    }

    #[test]
    fn an_errors_text_does_not_name_its_category() {
        // An error's own text is its message and offset, with no category
        // put before it, so that a caller can write "category: text".
        let errors = [
            Error::unknown_format("not a TIFF file"),
            Error::unsupported("lossy compression"),
            Error::malformed("bad tag", 12),
            Error::limit("preview too large", 1 << 40),
        ];
        for err in errors {
            let text = err.to_string();
            assert!(!text.starts_with(err.kind().name()), "{text}");
        }
        // An I/O error's message is "I/O error", its category's name: its
        // text is that message alone, never "I/O error: I/O error".
        let err = Error::from(io::Error::other("disk on fire"));
        assert_eq!(err.to_string(), "I/O error");
        let err = Error::io_at(io::Error::from(io::ErrorKind::UnexpectedEof), 7);
        assert_eq!(err.to_string(), "I/O error at byte 7");
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

    #[test]
    fn skip_damage_keeps_a_value_and_a_failing_reader() {
        assert_eq!(skip_damage(Ok(7)).ok(), Some(Some(7)));
        let err = skip_damage::<u8>(Err(Error::io_at(io::Error::other("gone"), 3)))
            .expect_err("a failing reader stays an error");
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.offset(), Some(3));
    }

    #[test]
    fn skip_damage_turns_every_other_kind_into_none() {
        for kind in all_kinds() {
            if kind == ErrorKind::Io {
                continue;
            }
            let err = Error::new(kind, Cow::Borrowed("damage"), Some(5), None);
            assert_eq!(skip_damage::<u8>(Err(err)).ok(), Some(None), "{kind}");
        }
    }
}
