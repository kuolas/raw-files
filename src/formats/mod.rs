// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RAW formats: the table of the formats the crate can detect, and
//! detection over it.
//!
//! Each format will live in its own folder here (`nef/`, `arw/`) and
//! register itself in [`spec`] as a [`FormatSpec`]: one function that says
//! whether a file is of that format, and one that reads its previews.
//! Detection goes by content, never by the file's name, so a renamed file
//! opens as what it is; [`Format::from_extension`] is for file pickers,
//! which must choose what to offer before opening anything. The preview
//! finder that the TIFF-based formats share lives in `tiff_previews`.
//!
//! The table is empty until the NEF reader registers itself: for now
//! detection claims no file, so a well-formed one is of no format the crate
//! recognises.

mod tiff_previews;

use std::io::{Read, Seek};

use crate::container::tiff::Tree;
use crate::error::Result;
use crate::format::Format;
use crate::io::Source;
use crate::limits::Limits;
use crate::preview::Preview;

/// What a format's reader found in a file: its previews, in the order it
/// found them. The file handle sorts them.
pub(crate) struct Parsed {
    /// The file's embedded previews, unsorted.
    pub previews: Vec<Preview>,
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
    /// Reads the previews of a file that this format claimed.
    pub parse: fn(&mut Source<R>, &Tree, &Limits) -> Result<Parsed>,
}

/// The spec of `format`.
///
/// The match is exhaustive, so a format added to [`Format`] fails to compile
/// until it is handled here. A function, not a `static`, because the specs'
/// functions are generic over the reader, and a `static` cannot be.
// `None` is temporary: once the NEF reader registers its spec the `Option`
// goes away, and a format without a spec fails to compile.
pub(crate) fn spec<R: Read + Seek>(format: Format) -> Option<FormatSpec<R>> {
    match format {
        Format::Nef => None,
    }
}

/// Every format's spec, in the order detection tries them: the first whose
/// `claims` says yes reads the file.
///
/// The order is [`Format::all`]'s, by construction.
pub(crate) fn all<R: Read + Seek>() -> Vec<FormatSpec<R>> {
    Format::all().iter().copied().filter_map(spec).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::{parse_header, walk};
    use crate::error::{Error, ErrorKind};
    use crate::orientation::Orientation;
    use crate::preview::PreviewFormat;
    use crate::tiff_builder::{Order, TiffBuilder};
    use std::io::Cursor;

    type Src = Source<Cursor<Vec<u8>>>;

    /// The Make tag: the camera's manufacturer, as text.
    const MAKE: u16 = 0x010F;

    /// A one-IFD TIFF file whose Make is `make`, read into a source and a
    /// tree.
    fn file(make: &str) -> (Src, Tree) {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        builder.ascii(ifd, MAKE, make);
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
        (src, tree)
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
        // Vacuous while every spec is `None`; it starts to bite with the first reader.
        for format in Format::all() {
            if let Some(s) = super::spec::<Cursor<Vec<u8>>>(*format) {
                assert_eq!(s.format, *format);
            }
        }
        let specs = all::<Cursor<Vec<u8>>>();
        assert!(specs.len() <= Format::all().len());
        let mut listed = Format::all().iter();
        for spec in &specs {
            assert!(listed.any(|format| *format == spec.format));
        }
    }

    #[test]
    fn the_table_is_empty_so_nothing_is_detected() {
        let specs = all::<Cursor<Vec<u8>>>();
        assert!(specs.is_empty());
        assert_eq!(detected(&specs, "Nikon").unwrap(), None);
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
}
