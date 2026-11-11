// SPDX-License-Identifier: MIT OR Apache-2.0

//! The file handle: a camera file opened, a RAW file or a JPEG, its format
//! detected, its previews listed and its metadata kept, ready to read a
//! preview.

use std::fmt;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use crate::container::Structure;
use crate::entries::{Entries, Entry, Value};
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
/// about the picture, every entry of its directories, and the reader to
/// fetch a preview's bytes or an entry's value from.
///
/// It opens the camera files raw-files reads: RAW files, such as Nikon's
/// NEF and Sony's ARW, and the JPEG files cameras write beside them or
/// instead of them. [`format`](Self::format) says which a file is, and
/// [`Format::is_raw`] whether it holds raw data. A JPEG file is one of its
/// own previews, its whole byte range.
///
/// Opening reads only the file's structure, through a cache of 64 KiB
/// blocks, so a few blocks rather than the whole file, and finds the
/// previews, the metadata and the entries; no preview, entry's value or raw
/// data is read until asked for. The previews are listed with their sizes,
/// and a preview's bytes are read only when asked for, so showing a preview
/// costs the structure plus that preview, whether the file is on a disk, on
/// a network share or in memory.
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
    entries: Entries,
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
    /// version recognizes; `Malformed` if its structure is damaged;
    /// `LimitExceeded` if reading its structure would cross one of the
    /// [`Limits`], as a RAW file's Make longer than
    /// [`Limits::max_value_bytes`] does, though a limit crossed inside a
    /// JPEG file's Exif or MPF segment is not an error: it costs at most
    /// what that segment holds, the Exif values and thumbnail or the MPF
    /// images, as damage there does; `Io` if the reader fails.
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
    /// found in `parsed`: the previews, in any order, the metadata and the
    /// entries.
    fn from_parts(src: Source<R>, format: Format, parsed: formats::Parsed, limits: Limits) -> Self {
        let formats::Parsed {
            mut previews,
            metadata,
            entries,
        } = parsed;
        preview::sort(&mut previews);
        Self {
            src,
            format,
            previews,
            metadata,
            entries,
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
    ///     if let Some(name) = metadata.camera().display_name() {
    ///         println!("with a {name}");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Every entry of every TIFF directory of the file that raw-files walks,
    /// as the file records it: its directory, tag, field type and count,
    /// and where its value lies, beneath the values
    /// [`metadata`](Self::metadata) reads from them.
    ///
    /// The directories are those of a NEF's or an ARW's TIFF structure, or
    /// of a JPEG's Exif segment: IFD0 and the directories chained after it,
    /// the SubIFDs, and the Exif, GPS and Interoperability IFDs; then a
    /// Nikon or Sony MakerNote's IFD, with Nikon's preview IFD, and an ARW's
    /// private directory, SR2Private; and a JPEG's Multi-Picture Format
    /// index. Records a maker enciphers are entries like any other, whose
    /// bytes are given as stored. Each entry was checked when the
    /// file was opened; one that could not be is left out, and counted by
    /// [`Entries::skipped`].
    ///
    /// Reading the entries reads nothing from the file, so it never fails;
    /// [`read_value`](Self::read_value) reads an entry's value.
    ///
    /// # Examples
    ///
    /// List every entry, then print when the picture was taken, as the
    /// Exif IFD's DateTimeOriginal (tag 0x9003) records it:
    ///
    /// ```no_run
    /// use raw_files::{Directory, RawFile};
    ///
    /// fn main() -> raw_files::Result<()> {
    ///     let mut file = RawFile::open("DSC_0001.NEF")?;
    ///     for entry in file.entries() {
    ///         let name = entry.name().unwrap_or("");
    ///         println!(
    ///             "{} 0x{:04X} {name} {} x {}",
    ///             entry.directory(),
    ///             entry.tag(),
    ///             entry.field_type(),
    ///             entry.count(),
    ///         );
    ///     }
    ///     if let Some(entry) = file.entries().entry(Directory::Exif, 0x9003) {
    ///         if let Some(text) = file.read_value(&entry)?.text() {
    ///             println!("taken at {text}");
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn entries(&self) -> &Entries {
        &self.entries
    }

    /// Reads the value of `entry`, as the file records it, in the byte order
    /// of its directory.
    ///
    /// A value the entry holds itself, of four bytes or fewer, is read from
    /// the entry, without touching the reader. Any other is read from the
    /// file each time it is asked for: at most [`Limits::max_value_bytes`]
    /// bytes are read into memory, and a value of numbers, such as SHORTs or
    /// RATIONALs, is decoded into a vector of as many bytes again, so the
    /// most held at once is twice the cap. Bytes and text are kept as read.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` if the value is longer than
    /// [`Limits::max_value_bytes`] allows, at the value's offset, before
    /// anything is allocated or read; `Malformed` if the value does not lie
    /// inside this file, which happens when `entry` came from another file,
    /// since an `Entry` is `Copy` and carries no tie to its file, while the
    /// value of an entry of another file that does lie inside this one is
    /// read from this file; `Io` if the reader fails, as it does when the
    /// file has shrunk since it was opened.
    ///
    /// # Examples
    ///
    /// Print the camera's Make as text, and the GPS version as numbers:
    ///
    /// ```no_run
    /// use raw_files::{Directory, RawFile, Value};
    ///
    /// fn main() -> raw_files::Result<()> {
    ///     let mut file = RawFile::open("DSC_0001.NEF")?;
    ///     if let Some(make) = file.entries().entry(Directory::Ifd(0), 0x010F) {
    ///         println!("{:?}", file.read_value(&make)?.text());
    ///     }
    ///     if let Some(version) = file.entries().entry(Directory::Gps, 0x0000) {
    ///         if let Value::Byte(bytes) = file.read_value(&version)? {
    ///             println!("GPS version {bytes:?}");
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn read_value(&mut self, entry: &Entry) -> Result<Value> {
        entry.read(&mut self.src, self.limits.max_value_bytes())
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

/// Shows the format, the previews, the metadata and the number of
/// entries; never the reader, which need not be `Debug`, nor each entry,
/// which would bury the rest.
impl<R> fmt::Debug for RawFile<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawFile")
            .field("format", &self.format)
            .field("previews", &self.previews)
            .field("metadata", &self.metadata)
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::entries::{Directory, Entries};
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
            entries: Entries::new(),
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
        let metadata = Metadata::default()
            .with_camera(Camera::default().with_recorded_model("NIKON Z 9"))
            .with_orientation(Orientation::Rotate90);
        let parsed = formats::Parsed {
            previews: Vec::new(),
            metadata: metadata.clone(),
            entries: Entries::new(),
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

    /// A NEF in byte order `order`: IFD0 with its Make and its Exif IFD,
    /// which holds DateTimeOriginal (0x9003) and an UNDEFINED value of
    /// `undefined` bytes (0x927C), with an ExposureTime held in its entry.
    fn nef(order: Order, undefined: usize) -> Vec<u8> {
        let mut builder = TiffBuilder::new(order);
        let (ifd0, exif) = (builder.ifd(), builder.ifd());
        builder
            .ascii(ifd0, 0x010F, "NIKON CORPORATION")
            .exif(ifd0, exif)
            .rational(exif, 0x829A, &[(1, 250)])
            .ascii(exif, 0x9003, "2026:11:09 10:11:12")
            .short(exif, 0x9207, &[5])
            .undefined(exif, 0x927C, &vec![0xAB; undefined]);
        builder.build().bytes
    }

    #[test]
    fn the_entries_are_the_files_and_their_values_are_read_on_request() {
        for order in [Order::Little, Order::Big] {
            let bytes = nef(order, 64);
            let mut file = RawFile::from_bytes(&bytes).unwrap();
            let directories: Vec<Directory> = file.entries().directories().collect();
            assert_eq!(directories, [Directory::Ifd(0), Directory::Exif]);
            assert_eq!(file.entries().len(), 6, "{order:?}");
            let time = file.entries().entry(Directory::Exif, 0x9003).unwrap();
            assert_eq!(time.name(), Some("DateTimeOriginal"));
            let pointer = file.entries().entry(Directory::Ifd(0), 0x8769).unwrap();
            assert_eq!(pointer.name(), Some("Exif IFD Pointer"));
            let value = file.read_value(&time).unwrap();
            assert_eq!(value.text().as_deref(), Some("2026:11:09 10:11:12"));
            let metering = file.entries().entry(Directory::Exif, 0x9207).unwrap();
            assert_eq!(file.read_value(&metering).unwrap(), Value::Short(vec![5]));
            let undefined = file.entries().entry(Directory::Exif, 0x927C).unwrap();
            assert_eq!(
                file.read_value(&undefined).unwrap(),
                Value::Undefined(vec![0xAB; 64])
            );
        }
    }

    #[test]
    fn a_value_is_read_up_to_the_value_cap_and_refused_past_it() {
        let bytes = nef(Order::Little, 1000);
        let limits = |n| Limits::default().with_max_value_bytes(n);
        let mut file = Options::new()
            .with_limits(limits(1000))
            .from_bytes(&bytes)
            .unwrap();
        let undefined = file.entries().entry(Directory::Exif, 0x927C).unwrap();
        assert_eq!(file.read_value(&undefined).unwrap().len(), 1000);

        let mut file = Options::new()
            .with_limits(limits(999))
            .from_bytes(&bytes)
            .unwrap();
        let err = file.read_value(&undefined).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
        assert_eq!(err.offset(), Some(undefined.byte_range().unwrap().start));
        // The cap bounds one value, not the file: the others still read.
        let time = file.entries().entry(Directory::Exif, 0x9003).unwrap();
        assert_eq!(file.read_value(&time).unwrap().len(), 20);
        // A value held in its entry is never refused. The lowest cap the
        // file opens under is its Make's length, 18 bytes, which the
        // capture time's 20 pass.
        let mut file = Options::new()
            .with_limits(limits(18))
            .from_bytes(&bytes)
            .unwrap();
        let metering = file.entries().entry(Directory::Exif, 0x9207).unwrap();
        assert_eq!(file.read_value(&metering).unwrap(), Value::Short(vec![5]));
        let err = file.read_value(&time).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
    }

    /// A NEF whose IFD0 holds its Make, a short Artist and an
    /// ImageDescription of `description` bytes, which the metadata reads
    /// with no limit of its own on its length.
    fn nef_with_description(description: usize) -> Vec<u8> {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        builder
            .ascii(ifd0, 0x010F, "NIKON CORPORATION")
            .ascii(ifd0, 0x013B, "Claude Monet")
            .ascii(ifd0, 0x010E, &"a".repeat(description));
        builder.build().bytes
    }

    #[test]
    fn the_metadata_is_read_under_the_value_cap_of_the_limits() {
        let bytes = nef_with_description(300);
        let read = |limits: Limits| {
            Options::new()
                .with_limits(limits)
                .from_bytes(&bytes)
                .unwrap()
        };

        // The default cap and a cap of 301 bytes, the value with its NUL,
        // read the description; a cap of 200 does not, and still reads the
        // artist, whose value is shorter.
        for (cap, read_whole) in [(None, true), (Some(301), true), (Some(200), false)] {
            let limits = cap.map_or_else(Limits::default, |n| {
                Limits::default().with_max_value_bytes(n)
            });
            let mut file = read(limits);
            let authorship = file.metadata().authorship();
            assert_eq!(authorship.artist(), Some("Claude Monet"), "{cap:?}");
            let description = authorship.description().map(str::len);
            assert_eq!(description, read_whole.then_some(300), "{cap:?}");
            // The caller's read of the same value goes the same way.
            let entry = file.entries().entry(Directory::Ifd(0), 0x010E).unwrap();
            assert_eq!(file.read_value(&entry).is_ok(), read_whole, "{cap:?}");
        }
    }

    #[test]
    fn unlimited_limits_lift_the_cap_on_the_metadatas_values() {
        // One byte more than the default cap, with the NUL two.
        let bytes = nef_with_description(1024 * 1024 + 1);
        let default = RawFile::from_bytes(&bytes).unwrap();
        assert_eq!(default.metadata().authorship().description(), None);
        let unlimited = Options::new()
            .with_limits(Limits::unlimited())
            .from_bytes(&bytes)
            .unwrap();
        let description = unlimited.metadata().authorship().description();
        assert_eq!(description.map(str::len), Some(1024 * 1024 + 1));
    }

    #[test]
    fn a_make_longer_than_the_value_cap_fails_the_open() {
        // "NIKON CORPORATION" and its NUL: 18 bytes.
        let bytes = nef(Order::Little, 8);
        let make = RawFile::from_bytes(&bytes)
            .unwrap()
            .entries()
            .entry(Directory::Ifd(0), 0x010F)
            .unwrap();
        let at = make.byte_range().unwrap().start;
        for cap in [0, 17] {
            let limits = Limits::default().with_max_value_bytes(cap);
            let err = Options::new()
                .with_limits(limits)
                .from_bytes(&bytes)
                .unwrap_err();
            assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{cap}: {err:?}");
            assert_eq!(err.offset(), Some(at), "{cap}");
        }
        let limits = Limits::default().with_max_value_bytes(18);
        assert!(
            Options::new()
                .with_limits(limits)
                .from_bytes(&bytes)
                .is_ok()
        );
    }

    #[test]
    fn the_default_value_cap_reads_a_value_of_a_whole_makernote() {
        // A Nikon 1 NEF's MakerNote entry, the largest value measured on
        // camera files: 993,826 bytes.
        let bytes = nef(Order::Big, 993_826);
        let mut file = RawFile::from_bytes(&bytes).unwrap();
        let makernote = file.entries().entry(Directory::Exif, 0x927C).unwrap();
        assert_eq!(file.read_value(&makernote).unwrap().len(), 993_826);
    }

    #[test]
    fn an_entry_whose_value_lies_outside_this_file_is_malformed() {
        // An entry of a longer file, read through the handle of a shorter
        // one, as a `Copy` entry can be.
        let long = nef(Order::Little, 4000);
        let short = nef(Order::Little, 8);
        let undefined = RawFile::from_bytes(&long)
            .unwrap()
            .entries()
            .entry(Directory::Exif, 0x927C)
            .unwrap();
        let mut file = RawFile::from_bytes(&short).unwrap();
        let err = file.read_value(&undefined).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
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
        assert!(text.contains("entries: 0"), "{text}");
        assert!(text.ends_with(", .. }"), "{text}");
    }
}
