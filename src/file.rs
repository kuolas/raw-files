// SPDX-License-Identifier: MIT OR Apache-2.0

//! The file handle: a camera file opened, a RAW file or a JPEG, its format
//! detected, its previews listed and its metadata kept, ready to read a
//! preview.

use std::fmt;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use crate::container::Structure;
use crate::error::{Error, Result};
use crate::format::Format;
use crate::formats;
use crate::geometry::Size;
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::Metadata;
use crate::options::Options;
use crate::preview::{self, Preview};

/// An open camera file: its format, its previews, what the camera recorded
/// about the picture, and the reader to fetch a preview's bytes from.
///
/// It opens the camera files raw-files reads: RAW files, such as Nikon's
/// NEF and Sony's ARW, and the JPEG files cameras write beside them or
/// instead of them. [`format`](Self::format) says which a file is, and
/// [`Format::is_raw`] whether it holds raw data. A JPEG file is one of its
/// own previews, its whole byte range.
///
/// Opening reads only the file's structure, through a cache of 64 KiB
/// blocks, so a few blocks rather than the whole file, and finds the
/// previews and the metadata; no preview or raw data is read until asked
/// for. The previews are listed with their sizes, and a preview's bytes are
/// read only when asked for, so showing a preview costs the structure plus
/// that preview, whether the file is on a disk, on a network share or in
/// memory.
///
/// `R` is any reader that can seek: a [`File`] from [`open`](Self::open),
/// a `Cursor<&[u8]>` from [`from_bytes`](Self::from_bytes), or anything
/// else through [`from_reader`](Self::from_reader). To open with other
/// [`Limits`], use [`Options`].
///
/// A `RawFile<R>` is [`Send`] when `R` is, and [`Sync`] when `R` is: the
/// handle holds nothing else that would prevent it, and the library keeps no
/// global state, so files can be opened and read on a thread pool, one
/// handle per thread. Reading takes `&mut self`; a [`Preview`] is `Copy`, so
/// copy the one you want out of the handle before reading it.
///
/// # Examples
///
/// Save the smallest preview that fills a 1024 × 768 box, with the
/// orientation it must be shown in:
///
/// ```no_run
/// use std::fs;
///
/// use raw_files::{RawFile, Size};
///
/// fn main() -> raw_files::Result<()> {
///     let mut file = RawFile::open("DSC_0001.NEF")?;
///     println!("a {} file", file.format());
///     let Some(preview) = file.preview_at_least(Size { width: 1024, height: 768 }) else {
///         println!("no previews");
///         return Ok(());
///     };
///     let mut bytes = Vec::new();
///     file.read_preview_into(&preview, &mut bytes)?;
///     fs::write("preview.jpg", &bytes)?;
///     println!(
///         "{} pixels, shown upright by {:?}",
///         preview.size(),
///         preview.orientation(),
///     );
///     Ok(())
/// }
/// ```
pub struct RawFile<R> {
    src: Source<R>,
    format: Format,
    /// Smallest first, as [`preview::sort`] orders them.
    previews: Vec<Preview>,
    metadata: Metadata,
    limits: Limits,
}

impl RawFile<File> {
    /// Opens the camera file at `path`, a RAW file or a JPEG, with the
    /// default [`Limits`].
    ///
    /// The format is detected from the file's content, never from its
    /// extension, so a renamed file opens as what it is. The library opens
    /// the `File` itself; see [`from_reader`](Self::from_reader) on buffering.
    ///
    /// This is [`Options::new().open(path)`](Options::open).
    ///
    /// # Errors
    ///
    /// `Io` if the file cannot be opened, and otherwise as for
    /// [`from_reader`](Self::from_reader).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Options::new().open(path)
    }
}

impl<'a> RawFile<Cursor<&'a [u8]>> {
    /// Opens a camera file that is already in memory, such as a file read
    /// whole or mapped into memory, with the default [`Limits`].
    ///
    /// The same handle and the same code path as a file on disk. A preview's
    /// [`byte_range`](Preview::byte_range) lets a caller borrow its bytes
    /// from `bytes` instead of copying them.
    ///
    /// This is [`Options::new().from_bytes(bytes)`](Options::from_bytes).
    ///
    /// # Errors
    ///
    /// As for [`from_reader`](RawFile::from_reader).
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        Options::new().from_bytes(bytes)
    }
}

impl<R: Read + Seek> RawFile<R> {
    /// Opens the camera file that `reader` reads, with the default
    /// [`Limits`].
    ///
    /// The file starts at the reader's start, whatever its position, and
    /// the format is detected from its content.
    ///
    /// Reads go through the library's own block cache, which seeks before
    /// every read, so wrapping the reader in a [`std::io::BufReader`]
    /// gains nothing: the library seeks before every read and reads whole
    /// blocks, so the wrapper's buffer is never used. Pass the `File` or
    /// reader as it is.
    ///
    /// This is [`Options::new().from_reader(reader)`](Options::from_reader).
    ///
    /// # Errors
    ///
    /// `UnknownFormat` if the file is not a RAW file or a JPEG file this
    /// version recognises; `Malformed` if its structure is damaged;
    /// `LimitExceeded` if reading its structure would cross one of the
    /// [`Limits`], though a limit crossed inside a JPEG file's Exif or MPF
    /// segment is not an error: it costs what that segment holds, the Exif
    /// values and thumbnail or the MPF images, as damage there does; `Io` if
    /// the reader fails.
    pub fn from_reader(reader: R) -> Result<Self> {
        Options::new().from_reader(reader)
    }

    /// Opens the file that `reader` reads under `limits`: the one place a
    /// file is opened, which [`Options`] calls.
    ///
    /// Reads the structure of the file's container, which the file's first
    /// bytes name, and asks the formats that container can hold, in order,
    /// whether the file is of that format; the first that says yes reads
    /// its previews and its metadata.
    pub(crate) fn new(reader: R, limits: Limits) -> Result<Self> {
        let mut src = Source::new(reader)?;
        // The container first: a file of no container the crate knows is
        // refused before any format is asked, and each format is asked
        // only about a structure its own container read.
        let structure = Structure::read(&mut src, &limits)?;
        let (format, parsed) = formats::detect(&mut src, &structure, &limits)?;
        Ok(Self::from_parts(src, format, parsed, limits))
    }

    /// The handle over `src`, of format `format`, with what its reader
    /// found in `parsed`: the previews, in any order, and the metadata.
    fn from_parts(src: Source<R>, format: Format, parsed: formats::Parsed, limits: Limits) -> Self {
        let formats::Parsed {
            mut previews,
            metadata,
        } = parsed;
        preview::sort(&mut previews);
        Self {
            src,
            format,
            previews,
            metadata,
            limits,
        }
    }

    /// The file's format, as detected from its content.
    pub fn format(&self) -> Format {
        self.format
    }

    /// The file's previews, smallest first: by pixel area, then by byte
    /// length.
    ///
    /// A RAW file's are the JPEG images the camera embedded in it. A JPEG
    /// file's are the smaller images it carries and the file itself, its
    /// whole byte range, which on the camera files checked is the largest:
    /// reading it reads the whole file, under [`Limits::max_preview_bytes`].
    ///
    /// Empty when the file holds no preview the library can show. The sizes
    /// are measured from each preview's own data, not taken from the tags
    /// that describe it.
    pub fn previews(&self) -> &[Preview] {
        &self.previews
    }

    /// What the camera recorded about the picture, read when the file was
    /// opened: the camera and the software that wrote the file, when the
    /// picture was taken, its exposure, the camera's settings, the lens,
    /// the orientation, the sizes and where the camera focused.
    ///
    /// Reading it reads nothing from the file, so it never fails. A value
    /// the file does not record, or records in a way that cannot be read,
    /// is `None`.
    ///
    /// # Examples
    ///
    /// Print when a picture was taken, and with which camera:
    ///
    /// ```no_run
    /// use raw_files::RawFile;
    ///
    /// fn main() -> raw_files::Result<()> {
    ///     let file = RawFile::open("DSC_0001.NEF")?;
    ///     let metadata = file.metadata();
    ///     if let Some(time) = metadata.capture_time() {
    ///         println!("taken at {time}");
    ///     }
    ///     if let (Some(make), Some(model)) = (metadata.make(), metadata.model()) {
    ///         println!("with a {make} {model}");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// The largest preview, the last of [`previews`](Self::previews), or
    /// `None` when there are none.
    pub fn largest_preview(&self) -> Option<Preview> {
        self.previews.last().copied()
    }

    /// The smallest preview at least `size`, as wide as its width and as
    /// tall as its height, for filling a box of that size without reading
    /// more than it needs.
    ///
    /// The comparison uses the size as stored, before the preview's
    /// [`orientation`](Preview::orientation) is applied: for a box that the
    /// orientation turns, ask for the box turned, its height as the width
    /// and its width as the height. When no preview is that large, the
    /// largest is returned, so asking for more than the file holds still
    /// gives the best there is. `None` only when the file has no previews.
    pub fn preview_at_least(&self, size: Size) -> Option<Preview> {
        preview::at_least(&self.previews, size)
    }

    /// Reads the bytes of `preview`, exactly as they are stored in the
    /// file, into a new buffer.
    ///
    /// To read several previews, or the previews of many files, reuse one
    /// buffer with [`read_preview_into`](Self::read_preview_into) instead.
    ///
    /// # Errors
    ///
    /// As for [`read_preview_into`](Self::read_preview_into).
    pub fn read_preview(&mut self, preview: &Preview) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.read_preview_into(preview, &mut buf)?;
        Ok(buf)
    }

    /// Reads the bytes of `preview`, exactly as they are stored in the
    /// file, into `buf`.
    ///
    /// `buf` is cleared first, then holds the preview's bytes and nothing
    /// else. Its capacity is kept, so a caller filling a grid of thumbnails
    /// can pass the same buffer for every file and stop reallocating once
    /// it has grown to the largest preview. On error, `buf` is left empty.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` if the preview is longer than
    /// [`Limits::max_preview_bytes`] allows, at the preview's offset, before
    /// anything is allocated or read; `Malformed` if the preview does not
    /// lie inside this file, which happens when `preview` came from another
    /// file, since a `Preview` is `Copy` and carries no tie to its file;
    /// `Io` if the reader fails. `LimitExceeded` is also returned when the
    /// preview's bytes cannot be held in memory on this platform (a length
    /// past what a `Vec` can hold, or an allocation that fails), whatever the
    /// limits are: `max_preview_bytes` cannot make such a preview fit.
    pub fn read_preview_into(&mut self, preview: &Preview, buf: &mut Vec<u8>) -> Result<()> {
        let offset = preview.byte_range().start;
        if preview.byte_len() > self.limits.max_preview_bytes() {
            buf.clear();
            return Err(Error::limit(
                "the preview is longer than max_preview_bytes allows",
                offset,
            ));
        }
        self.src.read_into(offset, preview.byte_len(), buf)
    }

    /// Returns the reader, giving up the handle.
    ///
    /// The reader's position is unspecified: seek before reading from it.
    pub fn into_inner(self) -> R {
        self.src.into_inner()
    }
}

/// Shows the format, the previews and the metadata; never the reader, which
/// need not be `Debug`.
impl<R> fmt::Debug for RawFile<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawFile")
            .field("format", &self.format)
            .field("previews", &self.previews)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::orientation::Orientation;
    use crate::preview::PreviewFormat;
    use crate::testing::size;
    use crate::tiff_builder::{Order, TiffBuilder};

    /// A JPEG preview of `width` by `height` pixels, `length` bytes at
    /// `offset`, upright.
    fn preview(offset: u64, length: u64, width: u32, height: u32) -> Preview {
        Preview::new(
            offset,
            length,
            size(width, height),
            PreviewFormat::Jpeg,
            Orientation::NoTransforms,
        )
    }

    /// The offsets of `previews`, in order, which name them.
    fn offsets(previews: &[Preview]) -> Vec<u64> {
        previews.iter().map(|p| p.byte_range().start).collect()
    }

    /// A handle over `reader`, of format NEF, with `previews` in the order
    /// given and no metadata, under `limits`.
    fn handle<R: Read + Seek>(reader: R, previews: Vec<Preview>, limits: Limits) -> RawFile<R> {
        let parsed = formats::Parsed {
            previews,
            metadata: Metadata::default(),
        };
        RawFile::from_parts(Source::new(reader).unwrap(), Format::Nef, parsed, limits)
    }

    /// The bytes of a stand-in preview: what reading must return.
    const BLOB: &[u8] = b"\xFF\xD8 a preview's bytes \xFF\xD9";

    /// A one-IFD TIFF file holding [`BLOB`], and the blob's offset.
    fn file_with_blob() -> (Vec<u8>, u64) {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        builder.ascii(ifd, 0x010F, "Nikon");
        let blob = builder.blob(BLOB);
        let built = builder.build();
        let offset = built.blob_offset(blob);
        (built.bytes, offset)
    }

    /// Three previews over the blob's bytes, out of order.
    fn three(offset: u64) -> Vec<Preview> {
        let len = u64::try_from(BLOB.len()).unwrap();
        vec![
            preview(offset, len, 1_620, 1_080),
            preview(offset + 1, len - 1, 160, 120),
            preview(offset + 2, len - 2, 640, 480),
        ]
    }

    #[test]
    fn previews_are_the_ones_given_smallest_first() {
        let (bytes, at) = file_with_blob();
        let file = handle(Cursor::new(bytes), three(at), Limits::default());
        assert_eq!(offsets(file.previews()), [at + 1, at + 2, at]);
    }

    #[test]
    fn the_largest_preview_is_the_last() {
        let (bytes, at) = file_with_blob();
        let file = handle(Cursor::new(bytes), three(at), Limits::default());
        assert_eq!(file.largest_preview(), file.previews().last().copied());
        assert_eq!(
            file.largest_preview().map(|p| p.size()),
            Some(size(1_620, 1_080))
        );
    }

    #[test]
    fn no_previews_gives_none() {
        let (bytes, _) = file_with_blob();
        let file = handle(Cursor::new(bytes), Vec::new(), Limits::default());
        assert!(file.previews().is_empty());
        assert_eq!(file.largest_preview(), None);
        assert_eq!(file.preview_at_least(size(0, 0)), None);
    }

    #[test]
    fn preview_at_least_gives_the_smallest_that_fits_or_the_largest() {
        let (bytes, at) = file_with_blob();
        let file = handle(Cursor::new(bytes), three(at), Limits::default());
        let preview_size = |p: Option<Preview>| p.map(|p| p.size());
        assert_eq!(
            preview_size(file.preview_at_least(size(300, 200))),
            Some(size(640, 480))
        );
        assert_eq!(
            preview_size(file.preview_at_least(size(4_000, 3_000))),
            Some(size(1_620, 1_080))
        );
    }

    #[test]
    fn format_is_the_one_detected() {
        let (bytes, _) = file_with_blob();
        let file = handle(Cursor::new(bytes), Vec::new(), Limits::default());
        assert_eq!(file.format(), Format::Nef);
    }

    #[test]
    fn metadata_is_the_one_the_reader_found() {
        let (bytes, _) = file_with_blob();
        let metadata = Metadata {
            model: Some("NIKON Z 9".to_owned()),
            orientation: Orientation::Rotate90,
            ..Metadata::default()
        };
        let parsed = formats::Parsed {
            previews: Vec::new(),
            metadata: metadata.clone(),
        };
        let file = RawFile::from_parts(
            Source::new(Cursor::new(bytes)).unwrap(),
            Format::Nef,
            parsed,
            Limits::default(),
        );
        assert_eq!(file.metadata(), &metadata);
    }

    #[test]
    fn read_preview_returns_exactly_the_stored_bytes() {
        let (bytes, at) = file_with_blob();
        let len = u64::try_from(BLOB.len()).unwrap();
        let blob = preview(at, len, 1, 1);
        let mut file = handle(Cursor::new(bytes), vec![blob], Limits::default());
        assert_eq!(file.read_preview(&blob).unwrap(), BLOB);
        // A preview inside the blob: its own bytes and no more.
        let inner = preview(at + 2, len - 4, 1, 1);
        assert_eq!(file.read_preview(&inner).unwrap(), BLOB[2..BLOB.len() - 2]);
    }

    #[test]
    fn read_preview_into_clears_the_buffer_first() {
        let (bytes, at) = file_with_blob();
        let len = u64::try_from(BLOB.len()).unwrap();
        let blob = preview(at, len, 1, 1);
        let mut file = handle(Cursor::new(bytes), vec![blob], Limits::default());
        let mut buf = b"left over from the last preview, and longer than this one".to_vec();
        file.read_preview_into(&blob, &mut buf).unwrap();
        assert_eq!(buf, BLOB);
    }

    #[test]
    fn a_preview_over_the_limit_is_refused_before_allocating() {
        let (bytes, at) = file_with_blob();
        let len = u64::try_from(BLOB.len()).unwrap();
        let limits = Limits::default().with_max_preview_bytes(4);
        // Both far longer than the buffer's capacity, so reading either
        // would grow it: the blob, which lies in the file and would be read
        // whole without the limit, and a preview longer than the file.
        let blob = preview(at, len, 1, 1);
        let huge = preview(at + 1, 1 << 40, 1, 1);
        let mut file = handle(Cursor::new(bytes), vec![blob, huge], limits);
        for preview in [blob, huge] {
            let mut buf = Vec::with_capacity(4);
            buf.extend_from_slice(b"old");
            let capacity = buf.capacity();
            assert!(preview.byte_len() > 4 * u64::try_from(capacity).unwrap());
            let err = file.read_preview_into(&preview, &mut buf).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{preview:?}");
            assert_eq!(err.offset(), Some(preview.byte_range().start));
            assert!(buf.is_empty(), "{preview:?}");
            assert_eq!(buf.capacity(), capacity, "{preview:?}");
        }
    }

    #[test]
    fn a_preview_at_the_limit_is_read() {
        let (bytes, at) = file_with_blob();
        let len = u64::try_from(BLOB.len()).unwrap();
        let blob = preview(at, len, 1, 1);
        let mut file = handle(
            Cursor::new(bytes),
            vec![blob],
            Limits::default().with_max_preview_bytes(len),
        );
        assert_eq!(file.read_preview(&blob).unwrap(), BLOB);
    }

    #[test]
    fn a_preview_past_the_end_of_the_file_is_malformed() {
        let (bytes, _) = file_with_blob();
        let end = u64::try_from(bytes.len()).unwrap();
        // As a preview of another, longer file would be.
        let elsewhere = preview(end - 4, 8, 1, 1);
        let mut file = handle(Cursor::new(bytes), vec![], Limits::default());
        let mut buf = b"stale".to_vec();
        let err = file.read_preview_into(&elsewhere, &mut buf).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Malformed);
        assert_eq!(err.offset(), Some(end - 4));
        assert!(buf.is_empty());
    }

    #[test]
    fn into_inner_returns_the_reader() {
        let (bytes, at) = file_with_blob();
        let mut file = handle(Cursor::new(bytes.clone()), three(at), Limits::default());
        // Move the reader, so that seeking back is the test's job.
        let first = file.previews()[0];
        file.read_preview(&first).unwrap();
        let mut reader = file.into_inner();
        reader.seek(std::io::SeekFrom::Start(at)).unwrap();
        let mut byte = [0];
        reader.read_exact(&mut byte).unwrap();
        assert_eq!(byte[0], BLOB[0]);
        assert_eq!(reader.into_inner(), bytes);
    }

    /// A reader that is not `Debug`.
    struct Opaque(Cursor<Vec<u8>>);

    impl Read for Opaque {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }

    impl Seek for Opaque {
        fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
            self.0.seek(pos)
        }
    }

    #[test]
    fn debug_shows_the_format_and_previews_without_the_reader() {
        let (bytes, at) = file_with_blob();
        let file = handle(Opaque(Cursor::new(bytes)), three(at), Limits::default());
        let text = format!("{file:?}");
        assert!(
            text.starts_with("RawFile { format: Nef, previews: ["),
            "{text}"
        );
        assert!(text.contains("width: 1620"), "{text}");
        assert!(text.ends_with(", .. }"), "{text}");
    }
}
