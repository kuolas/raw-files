// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RAW formats: the table of the formats the crate can detect, and
//! detection over it.
//!
//! Each format lives in its own folder here (`nef/`, `arw/`) and
//! registers itself in [`spec`] as a [`FormatSpec`]: one function that says
//! whether a file is of that format, and one that reads its previews and
//! its metadata.
//! Detection goes by content, never by the file's name, so a renamed file
//! opens as what it is; [`Format::from_extension`] is for file pickers,
//! which must choose what to offer before opening anything. The preview
//! finder that the TIFF-based formats share lives in `tiff_previews`, and
//! the metadata reader they share in `tiff_metadata`. The Nikon and Sony
//! MakerNote readers live in `makernote`, apart from the formats, so that
//! every format whose files carry a maker's MakerNote reads it with the
//! same reader; they read their IFDs inside the MakerNote's block with the
//! TIFF container's [`ifd_in`](crate::container::tiff::ifd_in).
//!
//! The table holds NEF and ARW, in that order. A Nikon file is recognised
//! by its IFD0's Make, and opens with its previews, which the NEF reader
//! finds with the shared finder, and its metadata, which it reads with the
//! shared reader. A Sony file is recognised by its IFD0's Make too, helped
//! for a Hasselblad Make by the pointer to Sony's private directory, and
//! opens with its previews and its metadata, which the ARW reader finds and
//! reads the same way. A file no format claims is of no format the crate
//! recognises.
//!
//! A DNG is of no format the crate recognises either, whatever its Make
//! says: converters and the phones that write DNG keep the maker's name in
//! the Make, so each reader refuses a DNG before it looks at the Make
//! ([`is_dng`]), and [`make_starts_with`] reads the Make for them.

mod arw;
mod makernote;
mod nef;
mod tiff_metadata;
mod tiff_previews;

use std::io::{Read, Seek};

use crate::container::tiff::Tree;
use crate::container::tiff::tags::{DNG_VERSION, MAKE};
use crate::error::Result;
use crate::format::Format;
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::Metadata;
use crate::preview::Preview;

/// What a format's reader found in a file: its previews, in the order it
/// found them, and its metadata. The file handle sorts the previews and
/// keeps both.
pub(crate) struct Parsed {
    /// The file's embedded previews, unsorted.
    pub previews: Vec<Preview>,
    /// What the camera recorded about the picture, which a format reads
    /// from the same tree it reads the previews from.
    pub metadata: Metadata,
}

/// One format the crate can detect, as two functions over the TIFF tree a
/// file was read into.
///
/// A spec is a pair of plain functions rather than a trait object, so that
/// the table is data: detection walks it in order, and a format joins it by
/// giving it a [`spec`].
pub(crate) struct FormatSpec<R> {
    /// The format this spec detects and reads.
    pub format: Format,
    /// Whether this format owns the file. It looks at the tree, and may
    /// read tag values through the source, but never at the file's name.
    pub claims: fn(&mut Source<R>, &Tree) -> Result<bool>,
    /// Reads the previews and the metadata of a file that this format
    /// claimed.
    pub parse: fn(&mut Source<R>, &Tree, &Limits) -> Result<Parsed>,
}

/// The spec of `format`.
///
/// The match is exhaustive, so a format added to [`Format`] fails to compile
/// until it is handled here. A function, not a `static`, because the specs'
/// functions are generic over the reader, and a `static` cannot be.
pub(crate) fn spec<R: Read + Seek>(format: Format) -> FormatSpec<R> {
    match format {
        Format::Nef => nef::spec(),
        Format::Arw => arw::spec(),
    }
}

/// Every format's spec, in the order detection tries them: the first whose
/// `claims` says yes reads the file.
///
/// The order is [`Format::all`]'s, by construction.
pub(crate) fn all<R: Read + Seek>() -> Vec<FormatSpec<R>> {
    Format::all().iter().copied().map(spec).collect()
}

/// Whether the file whose TIFF structure is `tree` is a DNG: whether its
/// IFD0 holds DNGVersion, which every DNG carries there.
///
/// A maker's reader refuses a DNG whatever its Make says. DNG is Adobe's
/// format, which no reader of this release reads, yet a DNG keeps the
/// camera maker's name: Adobe's converter leaves `SONY` or
/// `NIKON CORPORATION` in IFD0's Make and adds DNGVersion, and Sony's Xperia
/// phones write DNG with `Sony` as the Make. Without this test such a file
/// would open as its maker's own format. A reader of DNG would claim a file
/// by DNGVersion itself.
///
/// An entry that the IFD parser skipped because it could not be read (an
/// unknown field type, a length that overflows, a value outside the file)
/// is absent from the tree, so a DNG whose version entry is damaged is
/// taken for a file without one. That is the crate's rule for damage, which
/// costs only what it touches: the preview finder takes an Orientation it
/// cannot read for none in the same way.
pub(super) fn is_dng(tree: &Tree) -> bool {
    tree.ifd0().entry(DNG_VERSION).is_some()
}

/// Whether the Make of the IFD0 of the file whose TIFF structure is `tree`,
/// which names the camera's manufacturer, starts with `name` once trimmed
/// of spaces at either end, compared without regard to case.
///
/// The readers' claims decide by it. A missing Make, a Make that is not
/// text (type ASCII), and a Make that starts with anything else are all
/// "no".
///
/// # Errors
///
/// Whatever reading the Make returns: `LimitExceeded` for a value longer
/// than one tag may take, `Io` when the reader fails. A claim that cannot
/// read its tag stops detection rather than passing the file on.
pub(super) fn make_starts_with<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    name: &[u8],
) -> Result<bool> {
    let Some(make) = tree.ifd0().ascii(src, MAKE)? else {
        return Ok(false);
    };
    Ok(make
        .trim_matches(' ')
        .as_bytes()
        .get(..name.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(name)))
}

/// Detects the format of the file in `src`, whose TIFF structure is `tree`,
/// from `specs` in order, and reads it with the first spec that claims it.
///
/// Returns the format and what its reader found, or `None` when no spec
/// claims the file. Specs after the one that claims the file are not
/// consulted.
///
/// # Errors
///
/// Whatever the `claims` of a spec tried before the one that claims the
/// file returns, which stops detection; whatever the claiming spec's
/// `parse` returns.
pub(crate) fn detect<R: Read + Seek>(
    specs: &[FormatSpec<R>],
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<(Format, Parsed)>> {
    for spec in specs {
        if (spec.claims)(src, tree)? {
            return Ok(Some((spec.format, (spec.parse)(src, tree, limits)?)));
        }
    }
    Ok(None)
}

/// What the unit tests of detection and of the readers share: a file
/// described in code, read into a source and a tree as opening a file
/// reads it; and a reader that fails past the first block, for the tests
/// that a failure of the reader is an error.
#[cfg(test)]
pub(super) mod testing {
    use std::io::{self, Cursor, Read, Seek, SeekFrom};

    use crate::container::tiff::{Block, Tree, walk};
    use crate::io::{ByteOrder, Source};
    use crate::limits::Limits;
    use crate::tiff_builder::{IfdId, Order, TiffBuilder};

    /// A source over a file held in memory.
    pub(super) type Src = Source<Cursor<Vec<u8>>>;

    /// `order`, the fixture builder's byte order, as the reader names it.
    pub(super) fn byte_order(order: Order) -> ByteOrder {
        match order {
            Order::Little => ByteOrder::Little,
            Order::Big => ByteOrder::Big,
        }
    }

    /// The first byte of the second block the source caches: a reader that
    /// fails from here on still serves the header and the walk, which read
    /// only the first.
    pub(super) const FAR: u64 = 64 * 1024;

    /// A reader that fails every read from [`FAR`] on, for the tests that
    /// a failure of the reader is passed up, never taken for damage.
    pub(super) struct FailsFar(pub(super) Cursor<Vec<u8>>);

    impl Read for FailsFar {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0.position() >= FAR {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            self.0.read(buf)
        }
    }

    impl Seek for FailsFar {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.0.seek(pos)
        }
    }

    /// The file that `build` describes in a one-IFD little-endian TIFF
    /// file, read into a source and a tree.
    pub(super) fn file(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> (Src, Tree) {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        build(&mut builder, ifd);
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        (src, tree)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{self, Src};
    use super::*;
    use crate::container::tiff::tags::{DNG_PRIVATE_DATA, ORIENTATION};
    use crate::error::{Error, ErrorKind};
    use crate::orientation::Orientation;
    use crate::preview::PreviewFormat;
    use crate::tiff_builder::{ASCII, IfdId, RawValue, TiffBuilder};
    use std::io::Cursor;

    /// A one-IFD TIFF file whose Make is `make`, read into a source and a
    /// tree.
    fn file(make: &str) -> (Src, Tree) {
        testing::file(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        })
    }

    fn yes(_: &mut Src, _: &Tree) -> Result<bool> {
        Ok(true)
    }

    fn no(_: &mut Src, _: &Tree) -> Result<bool> {
        Ok(false)
    }

    fn fails(_: &mut Src, _: &Tree) -> Result<bool> {
        Err(Error::malformed("claims failed", 3))
    }

    /// Claims a file whose Make is "Nikon", reading the tag's value through
    /// the source, as a real format does.
    fn nikon(src: &mut Src, tree: &Tree) -> Result<bool> {
        Ok(tree.ifd0().ascii(src, MAKE)?.as_deref() == Some("Nikon"))
    }

    /// One preview, its offset `offset`, so that each spec's result can be
    /// told apart.
    fn one_preview(offset: u64) -> Parsed {
        Parsed {
            previews: vec![Preview::new(
                offset,
                1,
                1,
                1,
                PreviewFormat::Jpeg,
                Orientation::NoTransforms,
            )],
            metadata: Metadata::default(),
        }
    }

    fn parse_1(_: &mut Src, _: &Tree, _: &Limits) -> Result<Parsed> {
        Ok(one_preview(1))
    }

    fn parse_2(_: &mut Src, _: &Tree, _: &Limits) -> Result<Parsed> {
        Ok(one_preview(2))
    }

    fn parse_fails(_: &mut Src, _: &Tree, _: &Limits) -> Result<Parsed> {
        Err(Error::unsupported("parse failed"))
    }

    fn spec(
        claims: fn(&mut Src, &Tree) -> Result<bool>,
        parse: fn(&mut Src, &Tree, &Limits) -> Result<Parsed>,
    ) -> FormatSpec<Cursor<Vec<u8>>> {
        FormatSpec {
            format: Format::Nef,
            claims,
            parse,
        }
    }

    /// The offset of the one preview a detection found, naming the spec
    /// that read the file; `None` when no spec claimed it.
    fn detected(specs: &[FormatSpec<Cursor<Vec<u8>>>], make: &str) -> Result<Option<u64>> {
        let (mut src, tree) = file(make);
        let found = detect(specs, &mut src, &tree, &Limits::default())?;
        Ok(found.map(|(format, parsed)| {
            assert_eq!(format, Format::Nef);
            parsed.previews[0].byte_range().start
        }))
    }

    #[test]
    fn the_table_follows_format_all() {
        for format in Format::all() {
            assert_eq!(super::spec::<Cursor<Vec<u8>>>(*format).format, *format);
        }
        let formats: Vec<Format> = all::<Cursor<Vec<u8>>>()
            .iter()
            .map(|spec| spec.format)
            .collect();
        assert_eq!(formats, Format::all());
    }

    #[test]
    fn the_table_detects_each_vendors_file_as_its_format() {
        let specs = all::<Cursor<Vec<u8>>>();
        let format = |make: &str| {
            let (mut src, tree) = file(make);
            detect(&specs, &mut src, &tree, &Limits::default())
                .unwrap()
                .map(|(format, _)| format)
        };
        assert_eq!(format("SONY"), Some(Format::Arw));
        assert_eq!(format("NIKON CORPORATION"), Some(Format::Nef));
        assert_eq!(format("Canon"), None);
    }

    #[test]
    fn the_first_spec_that_claims_the_file_reads_it() {
        let specs = [
            spec(no, parse_fails),
            spec(yes, parse_1),
            spec(yes, parse_2),
        ];
        assert_eq!(detected(&specs, "Nikon").unwrap(), Some(1));
    }

    #[test]
    fn a_spec_after_the_claiming_one_is_not_consulted() {
        let specs = [spec(yes, parse_1), spec(fails, parse_2)];
        assert_eq!(detected(&specs, "Nikon").unwrap(), Some(1));
    }

    #[test]
    fn no_claim_is_none() {
        let specs = [spec(no, parse_1), spec(no, parse_2)];
        assert_eq!(detected(&specs, "Nikon").unwrap(), None);
    }

    #[test]
    fn a_claim_reads_tag_values_through_the_source() {
        let specs = [spec(nikon, parse_1), spec(yes, parse_2)];
        assert_eq!(detected(&specs, "Nikon").unwrap(), Some(1));
        assert_eq!(detected(&specs, "Canon").unwrap(), Some(2));
    }

    #[test]
    fn an_error_from_a_claim_stops_detection() {
        let specs = [spec(fails, parse_1), spec(yes, parse_2)];
        let err = detected(&specs, "Nikon").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Malformed);
        assert_eq!(err.offset(), Some(3));
    }

    #[test]
    fn an_error_from_the_claiming_parse_is_the_result() {
        let specs = [spec(yes, parse_fails), spec(yes, parse_2)];
        let err = detected(&specs, "Nikon").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unsupported);
    }

    #[test]
    fn a_dng_is_claimed_by_no_format() {
        let specs = all::<Cursor<Vec<u8>>>();
        for make in ["SONY", "NIKON CORPORATION"] {
            // As Adobe's converter writes it: the camera's Make, a DNG
            // version and a private data block far longer than Sony's
            // four-byte pointer.
            let (mut src, tree) = testing::file(|builder, ifd| {
                builder
                    .ascii(ifd, MAKE, make)
                    .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
                    .byte(ifd, DNG_PRIVATE_DATA, &[0; 32]);
            });
            assert!(is_dng(&tree), "{make}");
            let found = detect(&specs, &mut src, &tree, &Limits::default()).unwrap();
            assert!(found.is_none(), "{make}");
        }
    }

    #[test]
    fn a_file_without_a_dng_version_is_not_a_dng() {
        let (_, tree) = file("SONY");
        assert!(!is_dng(&tree));
    }

    /// Whether the Make of the file that `build` describes starts with
    /// `name`, as [`make_starts_with`] reads it.
    fn make_of(build: impl FnOnce(&mut TiffBuilder, IfdId), name: &[u8]) -> Result<bool> {
        let (mut src, tree) = testing::file(build);
        make_starts_with(&mut src, &tree, name)
    }

    /// Whether the Make `make`, as ASCII, starts with `name`.
    fn starts_with(make: &str, name: &[u8]) -> bool {
        make_of(
            |builder, ifd| {
                builder.ascii(ifd, MAKE, make);
            },
            name,
        )
        .unwrap()
    }

    #[test]
    fn a_make_that_starts_with_the_name_starts_with_it() {
        assert!(starts_with("NIKON CORPORATION", b"NIKON"));
        assert!(starts_with("NIKON", b"NIKON"));
    }

    #[test]
    fn a_make_is_compared_without_regard_to_case() {
        assert!(starts_with("nikon corporation", b"NIKON"));
        assert!(starts_with("Hasselblad", b"HASSELBLAD"));
    }

    #[test]
    fn a_make_with_spaces_at_either_end_starts_with_the_name() {
        assert!(starts_with("  NIKON CORPORATION  ", b"NIKON"));
    }

    #[test]
    fn a_make_padded_with_nuls_starts_with_the_name() {
        // The builder adds one NUL; the value holds three. Cutting the text
        // at the first NUL cannot change a prefix test, so this checks only
        // that the padding does no harm.
        assert!(starts_with("NIKON\0\0", b"NIKON"));
    }

    #[test]
    fn a_make_of_another_manufacturer_does_not_start_with_the_name() {
        assert!(!starts_with("Canon", b"NIKON"));
    }

    #[test]
    fn a_make_shorter_than_the_name_does_not_start_with_it() {
        assert!(!starts_with("NIKO", b"NIKON"));
    }

    #[test]
    fn a_make_with_the_name_inside_it_but_not_first_does_not_start_with_it() {
        assert!(!starts_with("MYNIKON", b"NIKON"));
    }

    #[test]
    fn an_empty_make_does_not_start_with_the_name() {
        assert!(!starts_with("", b"NIKON"));
    }

    #[test]
    fn a_missing_make_does_not_start_with_the_name() {
        let starts = make_of(
            |builder, ifd| {
                // A tag that is not Make, for an IFD0 that needs an entry
                // and nothing else.
                builder.short(ifd, ORIENTATION, &[1]);
            },
            b"NIKON",
        );
        assert!(!starts.unwrap());
    }

    #[test]
    fn a_make_that_is_not_text_does_not_start_with_the_name() {
        let starts = make_of(
            |builder, ifd| {
                builder.byte(ifd, MAKE, b"NIKON\0");
            },
            b"NIKON",
        );
        assert!(!starts.unwrap());
    }

    #[test]
    fn a_make_too_long_to_read_is_an_error() {
        // A Make of more bytes than the 1 MiB one tag value may take, inside
        // the file.
        let count: u32 = 1024 * 1024 + 1;
        let err = make_of(
            |builder, ifd| {
                builder.raw(ifd, MAKE, ASCII, count, RawValue::Offset(4096));
                builder.pad_to(4096 + u64::from(count));
            },
            b"NIKON",
        )
        .expect_err("the Make is too long");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
        assert_eq!(err.offset(), Some(4096));
    }
}
