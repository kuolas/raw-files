// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon NEF reader: how the crate recognises a NEF file, and, next,
//! how it finds the file's previews.
//!
//! A NEF file is a TIFF file written by a Nikon camera, with Nikon's own
//! additions inside: a MakerNote, compressed raw data. For this reader a
//! file is a NEF when the Make tag of its IFD0, which names the camera's
//! manufacturer, names Nikon. The NRW files of Coolpix cameras are the same
//! TIFF family and are recognised the same way. The file's name is never
//! looked at, and neither is the image IFD0 describes, which differs
//! between bodies and does not tell a Nikon file from another.
//!
//! This folder will also hold the search for the previews, in `previews.rs`,
//! and the reader of Nikon's MakerNote, in `makernote.rs`.

use std::io::{Read, Seek};

use super::{FormatSpec, Parsed};
use crate::container::tiff::Tree;
use crate::container::tiff::tags::MAKE;
use crate::error::Result;
use crate::format::Format;
use crate::io::Source;
use crate::limits::Limits;

/// The manufacturer's name that a Nikon camera's Make starts with,
/// compared without regard to case.
const NIKON: &[u8; 5] = b"NIKON";

/// The NEF format's entry in the format table.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R> {
    FormatSpec {
        format: Format::Nef,
        claims,
        parse,
    }
}

/// Whether the file whose TIFF structure is `tree` is a NEF: whether the
/// Make of its IFD0, trimmed of spaces at either end, starts with `NIKON`,
/// in any case.
///
/// Make alone decides. The NEF notes (`docs/formats/nef/`) record
/// `NIKON CORPORATION` as the Make of the DSLRs and `NIKON` as that of the
/// Coolpix compacts, and the public sample collection they cite files the
/// Z bodies, and a recent Coolpix, under `NIKON CORPORATION`: the
/// manufacturer's name comes first in each, so the prefix covers them all.
/// The notes' own test for a Nikon MakerNote compares Make the same way.
/// The MakerNote's `Nikon` signature does not decide, because Nikon's
/// editing software writes Nikon-style MakerNotes into the files of other
/// makes, whose Make it leaves as it was.
///
/// A missing Make, a Make that is not text (type ASCII), and the Make of any
/// other manufacturer are all "no".
///
/// # Errors
///
/// Whatever reading the Make returns: `LimitExceeded` for a value longer
/// than one tag may take, `Io` when the reader fails. A claim that cannot
/// read its tag stops detection rather than passing the file on.
fn claims<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<bool> {
    let Some(make) = tree.ifd0().ascii(src, MAKE)? else {
        return Ok(false);
    };
    Ok(make
        .trim_matches(' ')
        .as_bytes()
        .get(..NIKON.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(NIKON)))
}

/// Reads the previews of a NEF file that [`claims`] accepted: none yet.
///
/// The shared TIFF preview finder is to run here, over the whole tree;
/// until it does, a NEF file opens with no previews.
///
/// # Errors
///
/// None yet: the `Result` is the format table's signature.
fn parse<R: Read + Seek>(_src: &mut Source<R>, _tree: &Tree, _limits: &Limits) -> Result<Parsed> {
    Ok(Parsed {
        previews: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::ORIENTATION;
    use crate::container::tiff::{parse_header, walk};
    use crate::error::ErrorKind;
    use crate::tiff_builder::{ASCII, IfdId, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    type Src = Source<Cursor<Vec<u8>>>;

    /// The file that `build` describes in a one-IFD TIFF file, read into a
    /// source and a tree.
    fn file(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> (Src, Tree) {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        build(&mut builder, ifd);
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
        (src, tree)
    }

    /// Whether the NEF reader claims the file that `build` describes.
    fn claimed(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> bool {
        let (mut src, tree) = file(build);
        claims(&mut src, &tree).unwrap()
    }

    /// Whether the NEF reader claims a file whose Make is `make`, as ASCII.
    fn claimed_make(make: &str) -> bool {
        claimed(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        })
    }

    #[test]
    fn the_make_of_a_nikon_dslr_is_claimed() {
        assert!(claimed_make("NIKON CORPORATION"));
    }

    #[test]
    fn the_make_of_a_nikon_compact_is_claimed() {
        assert!(claimed_make("NIKON"));
    }

    #[test]
    fn a_make_in_lower_case_is_claimed() {
        assert!(claimed_make("nikon corporation"));
    }

    #[test]
    fn a_make_with_spaces_at_either_end_is_claimed() {
        assert!(claimed_make("  NIKON CORPORATION  "));
    }

    #[test]
    fn a_make_padded_with_nuls_is_claimed() {
        // The builder adds one NUL; the value holds three.
        assert!(claimed_make("NIKON\0\0"));
    }

    #[test]
    fn the_make_of_another_manufacturer_is_not_claimed() {
        assert!(!claimed_make("Canon"));
    }

    #[test]
    fn a_make_shorter_than_nikon_is_not_claimed() {
        assert!(!claimed_make("NIKO"));
    }

    #[test]
    fn a_make_with_nikon_inside_it_but_not_first_is_not_claimed() {
        assert!(!claimed_make("MYNIKON"));
    }

    #[test]
    fn an_empty_make_is_not_claimed() {
        assert!(!claimed_make(""));
    }

    #[test]
    fn a_file_without_a_make_is_not_claimed() {
        assert!(!claimed(|builder, ifd| {
            // A tag that is not Make, for an IFD0 that needs an entry and
            // nothing else.
            builder.short(ifd, ORIENTATION, &[1]);
        }));
    }

    #[test]
    fn a_make_that_is_not_text_is_not_claimed() {
        assert!(!claimed(|builder, ifd| {
            builder.byte(ifd, MAKE, b"NIKON\0");
        }));
    }

    #[test]
    fn a_make_too_long_to_read_is_an_error() {
        // A Make of more bytes than the 1 MiB one tag value may take, inside
        // the file.
        let count: u32 = 1024 * 1024 + 1;
        let (mut src, tree) = file(|builder, ifd| {
            builder.raw(ifd, MAKE, ASCII, count, RawValue::Offset(4096));
            builder.pad_to(4096 + u64::from(count));
        });
        let err = claims(&mut src, &tree).expect_err("the Make is too long");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
        assert_eq!(err.offset(), Some(4096));
    }

    #[test]
    fn a_claimed_file_has_no_previews_yet() {
        let (mut src, tree) = file(|builder, ifd| {
            builder.ascii(ifd, MAKE, "NIKON CORPORATION");
        });
        assert!(claims(&mut src, &tree).unwrap());
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        assert!(parsed.previews.is_empty());
    }
}
