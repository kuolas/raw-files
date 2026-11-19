// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote's block: its layouts, the MakerNote IFD and the
//! PreviewIFD read from it, and the PreviewIFD's JPEG as a candidate
//! preview.
//!
//! Nikon has used three layouts for the block, told apart by their first
//! bytes:
//!
//! - **The embedded layout**, which every body since the D70 and the D100
//!   writes. It starts with the signature `Nikon` and a NUL, and holds a
//!   TIFF header of its own 10 bytes in. That inner header is the base of
//!   every offset inside the block, and its byte order is the order of
//!   every number there. So the offsets are the same wherever the block
//!   lies, in a NEF file or in a JPEG file's Exif segment.
//! - **The headerless layout**, which the D1 family writes. The block is a
//!   bare IFD at its first byte. Its offsets count from the header of the
//!   TIFF structure that holds the MakerNote, as Sony's do, and its numbers
//!   are in that structure's byte order ([`MakerNoteBlock::ifd_at`]).
//! - **The short-header layout**, which the Coolpix E700, E800, E900 and
//!   E950 write in their JPEGs. The block starts with the 8 bytes
//!   `Nikon\0\1\0`, and an IFD follows them, read as the headerless
//!   layout's is. It is read only when the structure that holds the
//!   MakerNote is little-endian ([`short_header_ifd`]).
//!
//! The short-header layout numbers its tags in a scheme of its own: its
//! tag 0x0002 holds a text, where the later layouts record the ISO. So the
//! MakerNote keeps the layout it was read in ([`Layout`]), and gives its
//! IFD to the readers of the tags' rules only for the later layouts
//! ([`MakerNote::tagged`]). A short-header MakerNote gives no PreviewIFD
//! and no metadata, only its entries in the raw view.
//!
//! A block that starts with the short header is of that layout. Any other
//! block that starts with the signature is of the embedded layout or of
//! none. Any other block is read as a bare IFD. The Make is not looked at
//! here: the format's reader has taken the file for a Nikon one by its
//! Make ([`MakerNote::MAKE`]). So a block of another maker's under a Nikon
//! Make is read as a bare IFD too, under the same checks. If it starts
//! with text, its first two bytes, taken for the number of entries, are
//! more than the limits or the block allow.
//!
//! Every offset found inside the block is checked against the block, not
//! only against the file. The MakerNote IFD and the PreviewIFD are read
//! inside the block ([`ifd_in`]), each with its whole entry table, and an
//! entry whose value lies outside the block is dropped. The PreviewIFD's
//! JPEG must lie inside the block too
//! ([`preview_candidate`](MakerNote::preview_candidate)).
//!
//! Nothing the block holds is an error. A block that does not hold
//! together costs what the MakerNote would have given, never the file, as
//! the NEF security notes ask (`docs/formats/nef/security.md`, item 4 of
//! the checklist).
//!
//! The NEF notes give each layout's bytes and the bodies that write it
//! (`docs/formats/nef/makernote-and-metadata.md`, section 1).

use std::io::{Read, Seek};

use super::{PREVIEW_IFD, rules};
use crate::container::tiff::{Block, Ifd, Tree, ifd_in, parse_header};
use crate::entries::{Directory, Entries, Lookup, SkippedDirectory};
use crate::error::{ErrorKind, Result, skip_damage};
use crate::formats::makernote::{MakerNoteBlock, Picture, Reader};
use crate::formats::tiff_metadata::Standard;
use crate::formats::tiff_previews::{self, Candidate};
use crate::io::{ByteOrder, Source};
use crate::limits::Limits;
use crate::metadata::Metadata;

/// The signature a Nikon MakerNote of the embedded layout starts with, as
/// does one of the short-header layout: what tells both from the
/// headerless layout.
const SIGNATURE: &[u8; 6] = b"Nikon\0";

/// The 8 bytes a Nikon MakerNote of the short-header layout starts with:
/// the signature, then 1 and 0, where the embedded layout's version starts
/// with 2.
const SHORT_HEADER: &[u8; 8] = b"Nikon\0\x01\0";

/// Where the MakerNote IFD starts in a block of the short-header layout:
/// right after its header.
const SHORT_HEADER_LEN: u64 = SHORT_HEADER.len() as u64;

/// Where the inner TIFF header sits in a block of the embedded layout:
/// after the signature, two version bytes and two more.
pub(super) const HEADER_AT: u64 = 10;

/// The shortest block of the embedded layout: everything up to the end of
/// the inner TIFF header, which is 8 bytes long.
const MIN_LEN: u64 = HEADER_AT + 8;

/// A Nikon MakerNote of one of the layouts read here, its block found and
/// its MakerNote IFD read: what every use of the MakerNote starts from.
#[derive(Debug)]
pub(in crate::formats) struct MakerNote {
    /// The MakerNote IFD, which holds Nikon's tags: the IFD the inner
    /// header points at, the one at the block's first byte in the
    /// headerless layout, or the one 8 bytes in, in the short-header
    /// layout. It was read inside the block, so every value read from it
    /// lies inside the block.
    ///
    /// Its block, [`Ifd::block`], has the MakerNote's bytes as bounds. Its
    /// base is the inner TIFF header, or, in the other two layouts, the
    /// Exif IFD's. Its byte order, [`Ifd::order`], is the inner header's,
    /// which may differ from the file's, or in the other two layouts the
    /// Exif IFD's: the order of every number inside the block.
    ///
    /// Private, so that the readers of the tags' rules reach it only
    /// through [`tagged`](Self::tagged).
    ifd: Ifd,
    /// The layout the block was read in, which says whose rules read the
    /// MakerNote IFD's tags: none of the later layouts' read a
    /// short-header MakerNote's.
    layout: Layout,
    /// The PreviewIFD, which the MakerNote IFD's entry 0x0011 points to,
    /// read in the same block, with the same base and byte order
    /// ([`preview_ifd`]); or the directory the raw view lists as skipped
    /// when the entry is not one LONG or leads to none that can be read.
    /// Read with the MakerNote IFD, so that the preview and the raw view
    /// both take it from here. Always absent in the short-header layout,
    /// whose tag 0x0011, if it had one, would not be the PreviewIFD's.
    preview: Lookup<Ifd>,
}

impl MakerNote {
    /// The MakerNote IFD, for the readers of the tags' rules
    /// (`super::rules`): `None` for a MakerNote of the short-header
    /// layout, whose tags are numbered in a scheme of their own. So no
    /// rule of the later layouts reads a short-header MakerNote's tags,
    /// whichever reader asks.
    pub(super) fn tagged(&self) -> Option<&Ifd> {
        match self.layout {
            Layout::Embedded | Layout::Headerless => Some(&self.ifd),
            Layout::ShortHeader => None,
        }
    }
}

/// The layouts of a Nikon MakerNote's block that are read here, as its
/// first bytes tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    /// The embedded layout: the signature, a version and a TIFF header of
    /// the block's own, 10 bytes in.
    Embedded,
    /// The headerless layout: a bare IFD at the block's first byte.
    Headerless,
    /// The short-header layout: an IFD after the 8 bytes `Nikon\0\1\0`,
    /// whose tags are numbered in a scheme of their own.
    ShortHeader,
}

impl Reader for MakerNote {
    /// `NIKON`, the manufacturer's name that a Nikon camera's Make starts
    /// with. The NEF claim and the JPEG reader look for it in IFD0's Make
    /// before they take a file for a Nikon one.
    const MAKE: &'static [u8] = b"NIKON";

    /// The Nikon MakerNote of the file in `src`, whose IFDs are `tree`,
    /// read within `limits`, with its PreviewIFD when it has one.
    ///
    /// [`Lookup::Absent`] when the file has no Exif IFD or no MakerNote,
    /// when the MakerNote's inner header does not hold together
    /// ([`embedded_ifd`]), or when it is of the short-header layout in a
    /// big-endian structure ([`short_header_ifd`]). [`Lookup::Skipped`]
    /// when the MakerNote IFD does not hold together: the raw view lists it
    /// as skipped.
    ///
    /// # Errors
    ///
    /// `Io` when the reader fails.
    fn lookup<R: Read + Seek>(
        src: &mut Source<R>,
        tree: &Tree,
        limits: &Limits,
    ) -> Result<Lookup<MakerNote>> {
        let Some(block) = MakerNoteBlock::find(tree) else {
            return Ok(Lookup::Absent);
        };
        let head = block.first_bytes(src, SHORT_HEADER_LEN)?;
        let (layout, ifd) = if head.as_slice() == SHORT_HEADER {
            (Layout::ShortHeader, short_header_ifd(src, &block, limits)?)
        } else if head.starts_with(SIGNATURE) {
            (Layout::Embedded, embedded_ifd(src, &block, limits)?)
        } else {
            // A bare IFD at the block's first byte.
            (Layout::Headerless, block.makernote_ifd_at(src, 0, limits)?)
        };
        let ifd = match ifd {
            Lookup::Found(ifd) => ifd,
            Lookup::Skipped(skipped) => return Ok(Lookup::Skipped(skipped)),
            Lookup::Absent => return Ok(Lookup::Absent),
        };
        let preview = match layout {
            Layout::Embedded | Layout::Headerless => preview_ifd(src, &ifd, limits)?,
            // Its tags are not the later layouts': no tag 0x0011 is followed.
            Layout::ShortHeader => Lookup::Absent,
        };
        Ok(Lookup::Found(MakerNote {
            ifd,
            layout,
            preview,
        }))
    }

    /// What the MakerNote adds to the standard tags, as
    /// [`metadata`](fn@super::metadata) reads it.
    ///
    /// # Errors
    ///
    /// `Io` when the reader fails.
    fn metadata<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        standard: Standard,
        picture: Picture,
    ) -> Result<Metadata> {
        super::metadata(src, self, standard, picture)
    }

    /// Where the MakerNote's preview JPEG is, as a candidate for the shared
    /// finder: the JPEG its PreviewIFD declares, held to the block
    /// ([`tiff_previews::candidate`]).
    ///
    /// `None` when the MakerNote has no PreviewIFD, when its
    /// JPEGInterchangeFormat or JPEGInterchangeFormatLength breaks its rule,
    /// one LONG each, as Nikon writes them, or when the JPEG they declare
    /// does not lie inside the block.
    fn preview_candidate(&self) -> Option<Candidate> {
        let preview = self.preview.as_ref().found()?;
        let [offset] = preview.held(rules::JPEG_INTERCHANGE_FORMAT)?;
        let [length] = preview.held(rules::JPEG_INTERCHANGE_FORMAT_LENGTH)?;
        // An empty JPEG is passed on: the finder drops it with the others.
        tiff_previews::candidate(preview.block(), offset, length)
    }

    /// Adds the MakerNote's directories to `entries`: the MakerNote IFD,
    /// then the PreviewIFD, under the tag of the entry that points to it,
    /// or the PreviewIFD as skipped when it could not be read. The
    /// PreviewIFD is added only when the MakerNote IFD was, so that no
    /// PreviewIFD is listed without its MakerNote.
    fn add_entries(&self, entries: &mut Entries) {
        if entries.push(Directory::MakerNote, &self.ifd) {
            match &self.preview {
                Lookup::Found(preview) => {
                    entries.push(Directory::MakerNoteSub(PREVIEW_IFD), preview);
                }
                Lookup::Skipped(skipped) => entries.skip(*skipped),
                Lookup::Absent => {}
            }
        }
    }
}

/// The Nikon MakerNote of the file in `src`, as
/// [`lookup`](Reader::lookup) finds it; `None` when it gives none.
///
/// # Errors
///
/// As for [`lookup`](Reader::lookup).
#[cfg(test)]
pub(in crate::formats) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    Ok(MakerNote::lookup(src, tree, limits)?.found())
}

/// The PreviewIFD of the MakerNote whose IFD is `ifd`, read within
/// `limits`: the IFD at the offset its entry 0x0011 holds, inside the
/// MakerNote's block, with its base and byte order.
///
/// [`Lookup::Absent`] when the entry is missing. [`Lookup::Skipped`] when
/// the entry breaks its rule, one LONG as Nikon writes it, with no offset,
/// since its value is not read as one; or when the IFD does not hold
/// together inside the block, as when it lies past the MakerNote's
/// declared length. Neither counts toward [`Limits::max_ifds`]: the
/// MakerNote's IFDs are read apart from the walk, which counts its own.
///
/// # Errors
///
/// `Io` when the reader fails.
fn preview_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    limits: &Limits,
) -> Result<Lookup<Ifd>> {
    let Some([pointer]) = ifd.held(rules::PREVIEW_IFD) else {
        if ifd.entry(PREVIEW_IFD).is_none() {
            return Ok(Lookup::Absent);
        }
        return Ok(Lookup::Skipped(SkippedDirectory::new(
            Directory::MakerNote,
            Some(PREVIEW_IFD),
            None,
            ErrorKind::Malformed,
        )));
    };
    Ok(
        match ifd_in(src, ifd.block(), pointer, ifd.order(), limits)? {
            Ok(preview) => Lookup::Found(preview),
            Err(kind) => Lookup::Skipped(SkippedDirectory::new(
                Directory::MakerNote,
                Some(PREVIEW_IFD),
                ifd.block().at(pointer),
                kind,
            )),
        },
    )
}

/// The MakerNote IFD of `block`, a block that starts with the signature,
/// read within `limits`: the IFD the inner TIFF header points at, with that
/// header as the base.
///
/// [`Lookup::Absent`] when the block is too short for the header, or the
/// header does not hold together.
///
/// [`Lookup::Skipped`] when the header holds together and the IFD it
/// points at does not.
///
/// # Errors
///
/// `Io` when the reader fails.
fn embedded_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNoteBlock<'_>,
    limits: &Limits,
) -> Result<Lookup<Ifd>> {
    let range = makernote.range();
    if range.end.saturating_sub(range.start) < MIN_LEN {
        return Ok(Lookup::Absent);
    }
    let Some(base) = range.start.checked_add(HEADER_AT) else {
        return Ok(Lookup::Absent);
    };
    let block = Block::new(base, range.clone());
    let Some(header) = skip_damage(parse_header(src, &block))? else {
        return Ok(Lookup::Absent);
    };
    let found = ifd_in(src, &block, header.ifd0, header.order, limits)?;
    Ok(makernote.looked(found, block.at(header.ifd0)))
}

/// The MakerNote IFD of `makernote`, a block of the short-header layout,
/// read within `limits`: the IFD right after its 8 bytes, read as the
/// structure that holds the MakerNote reads its own IFDs, with that
/// structure's header as the base ([`MakerNoteBlock::makernote_ifd_at`]).
///
/// [`Lookup::Absent`] when that structure is big-endian. Every file known
/// to have the layout is little-endian, outside the MakerNote and inside.
/// The sources disagree on a big-endian one: one says its numbers are
/// always little-endian, another reads them in the structure's byte
/// order. So such a block is left unread rather than read by a guess
/// (`docs/formats/nef/makernote-and-metadata.md`, section 1).
///
/// [`Lookup::Skipped`] when the IFD does not hold together inside the
/// block.
///
/// # Errors
///
/// `Io` when the reader fails.
fn short_header_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNoteBlock<'_>,
    limits: &Limits,
) -> Result<Lookup<Ifd>> {
    if makernote.order() != ByteOrder::Little {
        return Ok(Lookup::Absent);
    }
    makernote.makernote_ifd_at(src, SHORT_HEADER_LEN, limits)
}

#[cfg(test)]
mod tests {
    use super::super::testing::{
        AT, Block, Change, MAKER_NOTE_VERSION, ORDERS, SERIAL, TAIL, block, entry_at, exif_file,
        nikon_file, read_file, tags_block, whole, with_makernote,
    };
    use super::super::{ISO, Picture, SERIAL_NUMBER, metadata};
    use super::*;
    use crate::container::tiff::tags::{
        JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKE, MAKER_NOTE,
    };
    use crate::container::tiff::{self, parse_ifd, walk};
    use crate::entries::Value;
    use crate::formats::testing::byte_order;
    use crate::formats::tiff_metadata;
    use crate::jpeg_stub::jpeg_stub;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{ASCII, BYTE, IFD, LONG, Order, SHORT, TiffBuilder, UNDEFINED};
    use std::io::Cursor;
    use std::ops::Range;

    /// The Exif tag UserComment, UNDEFINED like the MakerNote: a tag that
    /// may hold any bytes and is not the MakerNote.
    const USER_COMMENT: u16 = 0x9286;

    /// A Nikon file in byte order `order` whose MakerNote is `block`.
    fn with_block(order: Order, block: &Block) -> Vec<u8> {
        with_makernote(order, UNDEFINED, &block.bytes, block.len())
    }

    /// The MakerNote's preview candidate in the file in `src`, whose IFDs
    /// are `tree`: what [`read`], under `limits`, and then
    /// [`preview_candidate`](MakerNote::preview_candidate) return.
    fn read_candidate<R: Read + Seek>(
        src: &mut Source<R>,
        tree: &Tree,
        limits: &Limits,
    ) -> Result<Option<Candidate>> {
        let Some(makernote) = read(src, tree, limits)? else {
            return Ok(None);
        };
        Ok(makernote.preview_candidate())
    }

    /// The MakerNote's preview candidate in the file `bytes`, whose IFDs
    /// are walked with the default limits, as [`read_candidate`] finds it
    /// under `limits`.
    fn try_candidate(bytes: Vec<u8>, limits: &Limits) -> Result<Option<Candidate>> {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let file_block = tiff::Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        read_candidate(&mut src, &tree, limits)
    }

    /// The MakerNote's preview candidate in the file `bytes`, which must
    /// not be an error.
    fn candidate(bytes: Vec<u8>) -> Option<Candidate> {
        try_candidate(bytes, &Limits::default()).unwrap()
    }

    #[test]
    fn the_preview_is_at_the_blocks_offset_plus_10_plus_its_own() {
        for order in ORDERS {
            let block = block(order, Change::Nothing);
            let bytes = with_block(order, &block);
            let found = candidate(bytes.clone()).expect("a candidate");
            assert_eq!(found, block.candidate(AT), "{order:?}");
            // The JPEG's own first bytes are there.
            let start = usize::try_from(found.offset).unwrap();
            assert_eq!(bytes[start..start + 3], [0xFF, 0xD8, 0xFF], "{order:?}");
        }
    }

    #[test]
    fn a_makernote_of_type_byte_is_read() {
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_makernote(Order::Little, BYTE, &block.bytes, block.len());
        assert_eq!(candidate(bytes), Some(block.candidate(AT)));
    }

    #[test]
    fn a_block_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = block(inner, Change::Nothing);
            let bytes = with_block(file, &block);
            assert_eq!(candidate(bytes), Some(block.candidate(AT)), "{file:?}");
        }
    }

    /// Where the tests of a TIFF structure inside a larger file place its
    /// header, as a JPEG file holds the structure of its Exif segment.
    const POS: u64 = 1000;

    /// The file `bytes`, placed at [`POS`] in a larger file, after as many
    /// bytes of filler: a source of that file, and the tree walked with the
    /// header at POS as its base and the bytes from it as its bounds.
    fn placed(bytes: &[u8]) -> (Source<Cursor<Vec<u8>>>, Tree) {
        let mut file = vec![0; usize::try_from(POS).unwrap()];
        file.extend_from_slice(bytes);
        let mut src = Source::new(Cursor::new(file)).unwrap();
        let block = tiff::Block::new(POS, POS..src.len());
        let tree = walk(&mut src, &block, &Limits::default()).unwrap();
        (src, tree)
    }

    #[test]
    fn a_makernote_inside_a_file_counts_from_its_own_header() {
        for order in ORDERS {
            // The MakerNote lies POS bytes further into the file, with its
            // inner header, so its preview does too.
            let block = block(order, Change::Nothing);
            let (mut src, tree) = placed(&with_block(order, &block));
            let found = read_candidate(&mut src, &tree, &Limits::default()).unwrap();
            assert_eq!(found, Some(block.candidate(POS + AT)), "{order:?}");

            // A value stored out of line, read through the inner header.
            let tags = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            let (mut src, tree) = placed(&exif_file(order, &tags, whole(&tags), |_, _| {}));
            let makernote = read(&mut src, &tree, &Limits::default())
                .unwrap()
                .expect("a MakerNote");
            assert_eq!(
                makernote.ifd.block().base(),
                POS + AT + HEADER_AT,
                "{order:?}"
            );
            let standard = tiff_metadata::read(&mut src, &tree).unwrap();
            let metadata = metadata(&mut src, &makernote, standard, Picture::Raw).unwrap();
            assert_eq!(
                metadata.camera.serial_number.as_deref(),
                Some(SERIAL),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_file_without_an_exif_ifd_has_none() {
        // A sound MakerNote, but in IFD0, where no MakerNote belongs.
        let block = block(Order::Little, Change::Nothing);
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        builder.ascii(ifd0, MAKE, "NIKON CORPORATION");
        entry_at(&mut builder, ifd0, MAKER_NOTE, UNDEFINED, AT, block.len());
        builder.blob_at(AT, &block.bytes);
        assert_eq!(candidate(builder.build().bytes), None);
    }

    #[test]
    fn an_exif_ifd_without_a_makernote_has_none() {
        // A sound block, but under another tag.
        let block = block(Order::Little, Change::Nothing);
        let bytes = nikon_file(Order::Little, AT, &block.bytes, TAIL, |builder, exif| {
            entry_at(builder, exif, USER_COMMENT, UNDEFINED, AT, block.len());
        });
        assert_eq!(candidate(bytes), None);
    }

    #[test]
    fn a_makernote_that_is_not_bytes_has_none() {
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_makernote(Order::Little, ASCII, &block.bytes, block.len());
        assert_eq!(candidate(bytes), None);
    }

    #[test]
    fn a_block_shorter_than_the_inner_header_has_none() {
        // The first 17 bytes of a sound block, which the file goes on after.
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_makernote(Order::Little, UNDEFINED, &block.bytes, 17);
        assert_eq!(candidate(bytes), None);

        // Five bytes at the very end of the file, where reading the 6-byte
        // signature would run past the end.
        let bytes = nikon_file(Order::Little, AT, b"Nikon", 0, |builder, exif| {
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, 5);
        });
        assert_eq!(candidate(bytes), None);
    }

    #[test]
    fn another_makers_signature_has_none() {
        // Read as a bare IFD, its first two bytes, `Ca`, count thousands of
        // entries.
        let mut block = block(Order::Little, Change::Nothing);
        block.bytes[..6].copy_from_slice(b"Canon\0");
        assert_eq!(candidate(with_block(Order::Little, &block)), None);
    }

    /// A Nikon file in byte order `order` whose MakerNote is of the
    /// headerless layout, as the D1H writes it (public sample 5364): IFD0
    /// holds Make and points at an Exif IFD, which holds ExifVersion and
    /// the MakerNote, whose block is the MakerNote IFD, after `header`
    /// when it is not empty, then that IFD's values, the PreviewIFD with
    /// its values and the 570 by 375 stub JPEG the PreviewIFD declares.
    /// The file's own builder writes the block, so that every offset in it
    /// counts from the start of the file, in the file's byte order. The
    /// MakerNote IFD holds the ISO, `[0, 200]`, and the PreviewIFD pointer.
    /// The MakerNote's entry declares the first `declared` bytes of the
    /// block when given, and the whole block otherwise; the file goes on
    /// [`TAIL`] bytes past the block. Also where the block and the JPEG
    /// are in the file.
    fn headerless_file(
        order: Order,
        header: &[u8],
        declared: Option<u64>,
    ) -> (Vec<u8>, Range<u64>, Candidate) {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        let makernote = builder.ifd();
        let preview = builder.ifd();
        builder
            .ascii(ifd0, MAKE, "NIKON CORPORATION")
            .exif(ifd0, exif);
        builder.undefined(exif, 0x9000, b"0232");
        builder
            .prefix(makernote, header)
            .short(makernote, ISO, &[0, 200])
            .ifd_pointer(makernote, PREVIEW_IFD, preview);
        let stub = jpeg_stub(570, 375, 0xC0);
        let jpeg = builder.blob(&stub);
        builder
            .long_blob(preview, JPEG_INTERCHANGE_FORMAT, jpeg)
            .long_blob_len(preview, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg)
            .block_through(makernote, jpeg);

        match declared {
            None => builder.undefined_ifd(exif, MAKER_NOTE, makernote),
            Some(count) => {
                let count = u32::try_from(count).unwrap();
                builder.undefined_ifd_declaring(exif, MAKER_NOTE, makernote, count)
            }
        };
        let layout = builder.build();
        let range = layout.ifd_block(makernote);
        builder.pad_to(range.end + TAIL);
        let found = Candidate {
            offset: layout.blob_offset(jpeg),
            length: u64::try_from(stub.len()).unwrap(),
        };
        (builder.build().bytes, range, found)
    }

    #[test]
    fn a_headerless_makernote_counts_from_the_files_header() {
        // In the file's byte order: offsets read in the other one would
        // lead nowhere.
        for order in ORDERS {
            let (bytes, range, expected) = headerless_file(order, b"", None);
            let (mut src, makernote, standard) = read_file(bytes.clone());
            assert_eq!(
                makernote.ifd.block(),
                &tiff::Block::new(0, range),
                "{order:?}"
            );
            assert_eq!(makernote.ifd.order(), byte_order(order), "{order:?}");
            let found = makernote.preview_candidate();
            assert_eq!(found, Some(expected), "{order:?}");
            let start = usize::try_from(expected.offset).unwrap();
            assert_eq!(bytes[start..start + 3], [0xFF, 0xD8, 0xFF], "{order:?}");
            let metadata = metadata(&mut src, &makernote, standard, Picture::Raw).unwrap();
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    #[test]
    fn a_headerless_makernote_inside_a_file_counts_from_that_files_header() {
        // The file placed POS bytes further in, as a JPEG file's Exif
        // segment holds its TIFF structure: the offsets count from the
        // structure's header, at POS, so the preview lies POS bytes
        // further in too. Counted from the start of the file, they would
        // lead POS bytes short of the block.
        for order in ORDERS {
            let (bytes, range, expected) = headerless_file(order, b"", None);
            let (mut src, tree) = placed(&bytes);
            let makernote = read(&mut src, &tree, &Limits::default())
                .unwrap()
                .expect("a MakerNote");
            let block = tiff::Block::new(POS, POS + range.start..POS + range.end);
            assert_eq!(makernote.ifd.block(), &block, "{order:?}");
            let found = makernote.preview_candidate();
            assert_eq!(
                found,
                Some(Candidate {
                    offset: POS + expected.offset,
                    length: expected.length,
                }),
                "{order:?}"
            );
            let standard = tiff_metadata::read(&mut src, &tree).unwrap();
            let metadata = metadata(&mut src, &makernote, standard, Picture::Raw).unwrap();
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    /// The file `bytes`, walked with the default limits at position `pos`
    /// of a larger file as [`placed`] places it, or as the whole file at 0:
    /// a source of the file and its tree.
    fn walked_at(bytes: &[u8], pos: u64) -> (Source<Cursor<Vec<u8>>>, Tree) {
        if pos == 0 {
            let mut src = Source::new(Cursor::new(bytes.to_vec())).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            (src, tree)
        } else {
            assert_eq!(pos, POS);
            placed(bytes)
        }
    }

    /// What [`lookup`](MakerNote::lookup) finds, with the default limits,
    /// in the file `bytes`, walked as [`walked_at`] walks it.
    fn looked_up_at(bytes: &[u8], pos: u64) -> Lookup<MakerNote> {
        let (mut src, tree) = walked_at(bytes, pos);
        MakerNote::lookup(&mut src, &tree, &Limits::default()).unwrap()
    }

    #[test]
    fn a_short_header_makernote_is_read_in_a_little_endian_structure() {
        // `Nikon\0\1\0`, then the IFD, its offsets counted from the header
        // of the structure that holds the MakerNote: in a NEF file, at 0,
        // and in a JPEG file's Exif segment, at POS.
        for pos in [0, POS] {
            let (bytes, range, _) = headerless_file(Order::Little, SHORT_HEADER, None);
            let Lookup::Found(makernote) = looked_up_at(&bytes, pos) else {
                panic!("{pos}: a MakerNote");
            };
            assert_eq!(makernote.layout, Layout::ShortHeader, "{pos}");
            let block = tiff::Block::new(pos, pos + range.start..pos + range.end);
            assert_eq!(makernote.ifd.block(), &block, "{pos}");
            assert_eq!(makernote.ifd.order(), ByteOrder::Little, "{pos}");
            let view = view_of(&makernote);
            assert_eq!(tags_in(&view, Directory::MakerNote), [ISO, PREVIEW_IFD]);

            // The same layout in a big-endian structure is not read, and
            // nothing is listed as skipped.
            let (bytes, _, _) = headerless_file(Order::Big, SHORT_HEADER, None);
            let found = looked_up_at(&bytes, pos);
            assert!(matches!(found, Lookup::Absent), "{pos}: {found:?}");
        }
    }

    /// Tags of the short-header layout as the Coolpix files write them:
    /// a text of 6 bytes, so stored out of line, and a SHORT.
    const SHORT_HEADER_TEXT: u16 = 0x0002;
    const SHORT_HEADER_SHORT: u16 = 0x0003;

    /// A little-endian file whose MakerNote is of the short-header layout,
    /// and where its block lies. The file's own builder writes the block,
    /// so its offsets count from the start of the file. Its IFD holds:
    ///
    /// - the text `08.00` under [`SHORT_HEADER_TEXT`], the only value
    ///   stored out of line, so the block's last bytes;
    /// - 11 under [`SHORT_HEADER_SHORT`];
    /// - one SHORT under 0x0011, which the later layouts would take for a
    ///   PreviewIFD tag of the wrong type.
    ///
    /// The MakerNote's entry declares the first `declared` bytes of the
    /// block when given, and the whole block otherwise.
    fn short_header_file(declared: Option<u64>) -> (Vec<u8>, Range<u64>) {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        let makernote = builder.ifd();
        builder.ascii(ifd0, MAKE, "NIKON").exif(ifd0, exif);
        builder.undefined(exif, 0x9000, b"0210");
        builder
            .prefix(makernote, SHORT_HEADER)
            .ascii(makernote, SHORT_HEADER_TEXT, "08.00")
            .short(makernote, SHORT_HEADER_SHORT, &[11])
            .short(makernote, PREVIEW_IFD, &[1]);
        match declared {
            None => builder.undefined_ifd(exif, MAKER_NOTE, makernote),
            Some(count) => {
                let count = u32::try_from(count).unwrap();
                builder.undefined_ifd_declaring(exif, MAKER_NOTE, makernote, count)
            }
        };
        let built = builder.build();
        let range = built.ifd_block(makernote);
        builder.pad_to(range.end + TAIL);
        (builder.build().bytes, range)
    }

    #[test]
    fn a_short_header_value_stored_out_of_line_counts_from_the_outer_header() {
        // Counted from the MakerNote's start, or from the end of its 8
        // bytes, the text's offset would lead past the block.
        let (bytes, range) = short_header_file(None);
        for pos in [0, POS] {
            let (mut src, makernote) = makernote_at(&bytes, pos);
            let view = view_of(&makernote);
            assert_eq!(
                tags_in(&view, Directory::MakerNote),
                [SHORT_HEADER_TEXT, SHORT_HEADER_SHORT, PREVIEW_IFD],
                "{pos}"
            );
            let entry = view.entry(Directory::MakerNote, SHORT_HEADER_TEXT).unwrap();
            assert_eq!(
                entry.byte_range(),
                Some(pos + range.end - 6..pos + range.end),
                "{pos}"
            );
            let text = Value::Ascii(b"08.00\0".to_vec());
            assert_eq!(entry.read(&mut src, 64).unwrap(), text, "{pos}");
            let short = view
                .entry(Directory::MakerNote, SHORT_HEADER_SHORT)
                .unwrap();
            assert_eq!(short.read(&mut src, 64).unwrap(), Value::Short(vec![11]));
        }
    }

    #[test]
    fn a_short_header_value_past_the_declared_length_is_dropped_alone() {
        let (_, range) = short_header_file(None);
        let whole = range.end - range.start;
        for (count, skipped) in [(whole - 1, 1), (whole, 0)] {
            let (bytes, _) = short_header_file(Some(count));
            let (_, makernote) = makernote_at(&bytes, 0);
            let view = view_of(&makernote);
            let listed = if skipped == 0 {
                vec![SHORT_HEADER_TEXT, SHORT_HEADER_SHORT, PREVIEW_IFD]
            } else {
                vec![SHORT_HEADER_SHORT, PREVIEW_IFD]
            };
            assert_eq!(tags_in(&view, Directory::MakerNote), listed, "{count}");
            assert_eq!(
                view.skipped_entries(Directory::MakerNote),
                skipped,
                "{count}"
            );
        }
    }

    #[test]
    fn a_short_header_ifd_running_past_the_block_is_listed_as_skipped() {
        // The header, the count and three entries take 46 bytes.
        let table = SHORT_HEADER_LEN + 2 + 3 * 12;
        for (count, found) in [(table - 1, false), (table, true)] {
            let (bytes, range) = short_header_file(Some(count));
            match looked_up_at(&bytes, 0) {
                Lookup::Skipped(skipped) if !found => {
                    let expected = SkippedDirectory::new(
                        Directory::Exif,
                        Some(MAKER_NOTE),
                        Some(range.start + SHORT_HEADER_LEN),
                        ErrorKind::Malformed,
                    );
                    assert_eq!(skipped, expected, "{count}");
                }
                Lookup::Found(makernote) if found => {
                    // Its next-IFD pointer is not needed, nor its text.
                    let view = view_of(&makernote);
                    let listed = [SHORT_HEADER_SHORT, PREVIEW_IFD];
                    assert_eq!(tags_in(&view, Directory::MakerNote), listed);
                }
                other => panic!("{count}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_short_header_tag_0x0011_leads_to_no_preview_ifd() {
        // One LONG pointing at a PreviewIFD, which the headerless layout
        // follows to its JPEG; and one SHORT, which the later layouts list
        // as a skipped link. Neither is a link here.
        let (sound, _, expected) = headerless_file(Order::Little, b"", None);
        assert_eq!(candidate(sound), Some(expected), "the control");
        let (pointer, _, _) = headerless_file(Order::Little, SHORT_HEADER, None);
        let (misshaped, _) = short_header_file(None);
        for (what, bytes) in [("one LONG", pointer), ("one SHORT", misshaped)] {
            for pos in [0, POS] {
                let (_, makernote) = makernote_at(&bytes, pos);
                assert_eq!(makernote.preview_candidate(), None, "{what}, {pos}");
                let view = view_of(&makernote);
                assert_eq!(view.directories(), [Directory::MakerNote], "{what}");
                assert_eq!(view.skipped_directories(), [], "{what}, {pos}");
            }
        }
    }

    #[test]
    fn a_short_header_tag_0x0002_gives_no_iso() {
        // The headerless layout's ISO tag, two SHORTs, gives 200 there; in
        // the short-header layout its tag 0x0002 is a text, and no later
        // layout's rule reads it. Nothing else changes.
        for (header, iso) in [(&b""[..], Some(200)), (&SHORT_HEADER[..], None)] {
            let (bytes, _, _) = headerless_file(Order::Little, header, None);
            let (mut src, makernote, standard) = read_file(bytes);
            let expected = standard.metadata.clone();
            let metadata = makernote
                .metadata(&mut src, standard, Picture::Jpeg)
                .unwrap();
            assert_eq!(metadata.exposure.iso, iso, "{header:?}");
            if iso.is_none() {
                assert_eq!(metadata, expected);
            }
        }
    }

    #[test]
    fn a_headerless_makernote_whose_ifd_runs_past_the_block_has_none() {
        // The MakerNote IFD's count and two entries take 26 bytes, and
        // its next-IFD pointer is not needed.
        for order in ORDERS {
            let (bytes, _, _) = headerless_file(order, b"", Some(2 + 24 - 1));
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let makernote = read(&mut src, &tree, &Limits::default()).unwrap();
            assert!(makernote.is_none(), "{order:?}");

            // The control: declared to hold the IFD, it is read, its ISO
            // with it; the PreviewIFD now lies past the block, and gives
            // no preview.
            let (bytes, _, _) = headerless_file(order, b"", Some(2 + 24));
            let (mut src, makernote, standard) = read_file(bytes);
            let found = makernote.preview_candidate();
            assert_eq!(found, None, "{order:?}");
            let metadata = metadata(&mut src, &makernote, standard, Picture::Raw).unwrap();
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    #[test]
    fn a_block_of_text_under_a_nikon_make_has_none() {
        // Read as a bare IFD, the text's first two bytes, `No`, count
        // 20,079 or 28,494 entries: more than the limits allow, and, with
        // no limit, more than the block holds. The IFD after the text is
        // never reached.
        for order in ORDERS {
            let (bytes, _, _) = headerless_file(order, b"Not an IFD at all\0", None);
            for limits in [
                Limits::default(),
                Limits::default().with_max_ifd_entries(u32::MAX),
            ] {
                let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
                let file_block = tiff::Block::file(src.len());
                let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
                let makernote = read(&mut src, &tree, &limits).unwrap();
                assert!(makernote.is_none(), "{order:?}, {limits:?}");
            }
        }
    }

    #[test]
    fn an_inner_header_without_42_has_none() {
        for order in ORDERS {
            let block = block(order, Change::Magic41);
            assert_eq!(candidate(with_block(order, &block)), None, "{order:?}");
        }
    }

    /// Which IFD of the block [`relocated`] moves.
    #[derive(Clone, Copy, Debug)]
    enum Part {
        MakerNoteIfd,
        PreviewIfd,
    }

    /// A sound block in byte order `order` whose `part` is read from a copy
    /// starting `overlap` bytes before the block's end, the offset that
    /// leads to it changed to match; and the block's length, the count
    /// the MakerNote declares, which the copy runs past.
    ///
    /// The copy is a whole IFD inside the file, and its own offsets lead
    /// back into the block, so the block's end is the one thing that can
    /// reject it: a MakerNote declared long enough to hold the copy gives
    /// the preview.
    fn relocated(order: Order, part: Part, overlap: u64) -> (Vec<u8>, u64, Candidate) {
        let mut block = block(order, Change::Nothing);
        let (range, pointer_at) = match part {
            // The inner header's IFD offset, 4 bytes into it.
            Part::MakerNoteIfd => (block.makernote..block.preview, HEADER_AT + 4),
            Part::PreviewIfd => (block.preview..block.jpeg, block.pointer_at()),
        };
        let copy = block.bytes[usize::try_from(range.start).unwrap()..]
            [..usize::try_from(range.end - range.start).unwrap()]
            .to_vec();
        let copy_at = block.len() - overlap;
        block.put_u32(pointer_at, u32::try_from(copy_at - HEADER_AT).unwrap());
        let declared = block.len();
        let expected = block.candidate(AT);
        let mut bytes = block.bytes;
        bytes.truncate(usize::try_from(copy_at).unwrap());
        bytes.extend_from_slice(&copy);
        (bytes, declared, expected)
    }

    #[test]
    fn an_ifd_past_the_block_has_none() {
        for order in ORDERS {
            for part in [Part::MakerNoteIfd, Part::PreviewIfd] {
                let (bytes, declared, expected) = relocated(order, part, 0);
                let file = with_makernote(order, UNDEFINED, &bytes, declared);
                assert_eq!(candidate(file), None, "{order:?}, {part:?}");

                // The control: declared to hold the copy, it gives the
                // preview, so the block's end alone rejected it above.
                let whole = u64::try_from(bytes.len()).unwrap();
                let file = with_makernote(order, UNDEFINED, &bytes, whole);
                assert_eq!(candidate(file), Some(expected), "{order:?}, {part:?}");
            }
        }
    }

    /// What [`lookup`](MakerNote::lookup) finds, under `limits`, in the
    /// file `bytes`, walked with the default limits.
    fn looked_up(bytes: Vec<u8>, limits: &Limits) -> Lookup<MakerNote> {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let file_block = tiff::Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        MakerNote::lookup(&mut src, &tree, limits).unwrap()
    }

    #[test]
    fn an_ifd_past_the_block_is_listed_as_skipped() {
        // The copy starts at the declared end of the block: where each link
        // leads in the file.
        for order in ORDERS {
            for part in [Part::MakerNoteIfd, Part::PreviewIfd] {
                let (bytes, declared, _) = relocated(order, part, 0);
                let file = with_makernote(order, UNDEFINED, &bytes, declared);
                let what = format!("{order:?}, {part:?}");
                let at = Some(AT + declared);
                match (part, looked_up(file, &Limits::default())) {
                    (Part::MakerNoteIfd, Lookup::Skipped(skipped)) => {
                        let expected = SkippedDirectory::new(
                            Directory::Exif,
                            Some(MAKER_NOTE),
                            at,
                            ErrorKind::Malformed,
                        );
                        assert_eq!(skipped, expected, "{what}");
                    }
                    (Part::PreviewIfd, Lookup::Found(makernote)) => {
                        let view = view_of(&makernote);
                        assert_eq!(view.directories(), [Directory::MakerNote], "{what}");
                        let expected = SkippedDirectory::new(
                            Directory::MakerNote,
                            Some(PREVIEW_IFD),
                            at,
                            ErrorKind::Malformed,
                        );
                        assert_eq!(view.skipped_directories(), [expected], "{what}");
                    }
                    (_, other) => panic!("{what}: {other:?}"),
                }
            }
        }
    }

    #[test]
    fn a_makernote_ifd_over_the_entry_cap_is_listed_as_skipped_for_the_cap() {
        for order in ORDERS {
            let block = block(order, Change::Nothing);
            let file = with_block(order, &block);
            let limits = Limits::default().with_max_ifd_entries(1);
            let Lookup::Skipped(skipped) = looked_up(file, &limits) else {
                panic!("{order:?}: the MakerNote IFD has two entries");
            };
            assert_eq!(skipped.parent(), Directory::Exif, "{order:?}");
            assert_eq!(skipped.tag(), Some(MAKER_NOTE), "{order:?}");
            assert_eq!(skipped.offset(), Some(AT + block.makernote), "{order:?}");
            assert_eq!(skipped.kind(), ErrorKind::LimitExceeded, "{order:?}");
        }
    }

    #[test]
    fn a_layout_not_read_is_not_listed_as_skipped() {
        // An inner header without 42, in both byte orders, and the
        // short-header layout in a big-endian structure, the one order it
        // is not read in: no IFD of a layout read here was found to be
        // damaged.
        let (short_header, _, _) = headerless_file(Order::Big, SHORT_HEADER, None);
        let files = ORDERS
            .map(|order| (order, with_block(order, &block(order, Change::Magic41))))
            .into_iter()
            .map(|(order, file)| (format!("{order:?}, no 42"), file))
            .chain([("Big, short header".to_owned(), short_header)]);
        for (what, file) in files {
            let found = looked_up(file, &Limits::default());
            assert!(matches!(found, Lookup::Absent), "{what}: {found:?}");
        }
    }

    #[test]
    fn an_ifd_whose_entries_run_past_the_block_has_none() {
        // The IFD's count is the block's last two bytes; its entries follow
        // it in the file.
        for order in ORDERS {
            for part in [Part::MakerNoteIfd, Part::PreviewIfd] {
                let (bytes, declared, expected) = relocated(order, part, 2);
                let file = with_makernote(order, UNDEFINED, &bytes, declared);
                assert_eq!(candidate(file), None, "{order:?}, {part:?}");

                let whole = u64::try_from(bytes.len()).unwrap();
                let file = with_makernote(order, UNDEFINED, &bytes, whole);
                assert_eq!(candidate(file), Some(expected), "{order:?}, {part:?}");
            }
        }
    }

    #[test]
    fn an_ifd_offset_past_the_end_of_the_file_has_none() {
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            for pointer_at in [HEADER_AT + 4, sound.pointer_at()] {
                let mut block = block(order, Change::Nothing);
                block.put_u32(pointer_at, u32::MAX);
                assert_eq!(
                    candidate(with_block(order, &block)),
                    None,
                    "{order:?}, the offset at {pointer_at}"
                );
            }
        }
    }

    #[test]
    fn a_preview_ifd_pointer_stored_out_of_line_has_none() {
        for order in ORDERS {
            let block = block(order, Change::PointerOutOfLine);
            assert_eq!(candidate(with_block(order, &block)), None, "{order:?}");
        }
    }

    #[test]
    fn a_preview_ifd_without_the_jpegs_offset_and_length_has_none() {
        for change in [Change::NoJpegTags, Change::NoJpegLength] {
            let block = block(Order::Little, change);
            assert_eq!(
                candidate(with_block(Order::Little, &block)),
                None,
                "{change:?}"
            );
        }
    }

    #[test]
    fn a_jpeg_running_past_the_block_has_none() {
        for order in ORDERS {
            // Declared one byte short of the JPEG's end, which is the end
            // of the bytes: the JPEG lies inside the file, not the block.
            let sound = block(order, Change::Nothing);
            let file = with_makernote(order, UNDEFINED, &sound.bytes, sound.len() - 1);
            assert_eq!(candidate(file), None, "{order:?}");

            // A JPEG length so long that its end passes the file's too.
            let mut long = block(order, Change::Nothing);
            long.put_u32(long.length_at(), u32::MAX);
            assert_eq!(candidate(with_block(order, &long)), None, "{order:?}");
        }
    }

    #[test]
    fn a_preview_number_that_is_not_one_long_gives_no_preview() {
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            let pointer = u32::try_from(sound.preview - HEADER_AT).unwrap();
            let offset = u32::try_from(sound.jpeg - HEADER_AT).unwrap();
            let length = u32::try_from(sound.jpeg_len).unwrap();
            let long = |value: u32| order.u32(value);
            // The value first, as two SHORTs or one SHORT write it, then 0.
            let shorts = |value: u32| {
                let value = u16::try_from(value).unwrap();
                let mut field = [0; 4];
                field[..2].copy_from_slice(&order.u16(value));
                field
            };
            let cases = [
                (sound.pointer_at(), pointer, "the PreviewIFD pointer"),
                (sound.offset_at(), offset, "the JPEG's offset"),
                (sound.length_at(), length, "the JPEG's length"),
            ];
            for (at, value, what) in cases {
                // Rewritten as the one LONG it is, the preview stands.
                let mut same = block(order, Change::Nothing);
                same.put_entry(at, LONG, 1, long(value));
                let found = candidate(with_block(order, &same));
                assert_eq!(found, Some(sound.candidate(AT)), "{order:?} {what}");

                for (typ, count, field, shape) in [
                    (SHORT, 2, shorts(value), "two SHORTs, the first right"),
                    (SHORT, 1, shorts(value), "one SHORT"),
                    (IFD, 1, long(value), "one IFD"),
                    (LONG, 0, long(value), "no LONG"),
                ] {
                    let mut changed = block(order, Change::Nothing);
                    changed.put_entry(at, typ, count, field);
                    let found = candidate(with_block(order, &changed));
                    assert_eq!(found, None, "{order:?} {what} as {shape}");
                }
            }
        }
    }

    #[test]
    fn a_preview_ifd_entry_whose_value_lies_past_the_block_is_dropped() {
        // The first of the PreviewIFD's two JPEGInterchangeFormat entries
        // stores its value past the block, inside the file. Dropped when
        // the PreviewIFD is read, it leaves the second, which the entry
        // holds, to give the JPEG's offset. Kept, it would be the one
        // found, and refused as stored out of line: no preview.
        for order in ORDERS {
            let block = block(order, Change::JpegOffsetPastTheBlockFirst);
            let bytes = with_block(order, &block);

            // The PreviewIFD as the file holds it, read from the MakerNote's
            // base with the whole file as bounds: the out-of-line entry
            // first, its value the 8 bytes just past the block.
            let limits = Limits::default();
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &limits).unwrap();
            let makernote = read(&mut src, &tree, &limits)
                .unwrap()
                .expect("a MakerNote");
            let at = AT + block.preview;
            let whole_file = tiff::Block::new(makernote.ifd.block().base(), 0..src.len());
            let (table, _) = parse_ifd(&mut src, &whole_file, at, makernote.ifd.order(), &limits)
                .expect("the PreviewIFD");
            let first = &table.entries()[0];
            assert_eq!(first.tag(), JPEG_INTERCHANGE_FORMAT, "{order:?}");
            assert_eq!(first.value_offset(), Some(AT + block.len()), "{order:?}");
            assert_eq!(table.invalid_entries(), 0, "{order:?}");

            assert_eq!(candidate(bytes), Some(block.candidate(AT)), "{order:?}");
        }
    }

    #[test]
    fn an_empty_jpeg_is_passed_on() {
        let mut block = block(Order::Little, Change::Nothing);
        block.put_u32(block.length_at(), 0);
        let found = candidate(with_block(Order::Little, &block));
        assert_eq!(found.map(|c| c.length), Some(0));
    }

    #[test]
    fn a_makernote_ifd_with_more_entries_than_the_limit_has_none() {
        // The MakerNote IFD holds two entries, the PreviewIFD two.
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_block(Order::Little, &block);
        let found = try_candidate(bytes.clone(), &Limits::default().with_max_ifd_entries(2));
        assert_eq!(found.unwrap(), Some(block.candidate(AT)));
        let found = try_candidate(bytes, &Limits::default().with_max_ifd_entries(1));
        assert_eq!(found.unwrap(), None);
    }

    /// The entries `makernote` adds to the raw view, alone.
    fn view_of(makernote: &MakerNote) -> Entries {
        let mut entries = Entries::new();
        makernote.add_entries(&mut entries);
        entries
    }

    /// The tags `entries` lists in `directory`, in order.
    fn tags_in(entries: &Entries, directory: Directory) -> Vec<u16> {
        entries
            .in_directory(directory)
            .iter()
            .map(crate::Entry::tag)
            .collect()
    }

    /// The MakerNote of the file `bytes`, walked as [`walked_at`] walks
    /// it: a source of the file and the MakerNote, which must be read.
    fn makernote_at(bytes: &[u8], pos: u64) -> (Source<Cursor<Vec<u8>>>, MakerNote) {
        let (mut src, tree) = walked_at(bytes, pos);
        let makernote = read(&mut src, &tree, &Limits::default())
            .unwrap()
            .expect("a MakerNote");
        (src, makernote)
    }

    /// The Nikon PreviewIFD, as the raw view names it.
    const PREVIEW: Directory = Directory::MakerNoteSub(PREVIEW_IFD);

    #[test]
    fn the_view_lists_the_makernote_ifd_then_its_preview_ifd() {
        // The embedded layout, whose offsets count from its inner header
        // wherever the block lies, in both byte orders.
        for order in ORDERS {
            for pos in [0, POS] {
                let block = block(order, Change::Nothing);
                let (mut src, makernote) = makernote_at(&with_block(order, &block), pos);
                let view = view_of(&makernote);
                let what = format!("{order:?}, {pos}");
                let directories: Vec<Directory> = view.directories().to_vec();
                assert_eq!(directories, [Directory::MakerNote, PREVIEW], "{what}");
                assert_eq!(
                    tags_in(&view, Directory::MakerNote),
                    [MAKER_NOTE_VERSION, PREVIEW_IFD],
                    "{what}"
                );
                assert_eq!(
                    tags_in(&view, PREVIEW),
                    [JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH],
                    "{what}"
                );
                assert_eq!(view.skipped_entries(Directory::MakerNote), 0, "{what}");
                assert_eq!(view.skipped_entries(PREVIEW), 0, "{what}");
                // Values as written: the version, and the JPEG's offset
                // counted from the inner header.
                let mut value = |directory, tag| {
                    let entry = view.entry(directory, tag).expect("an entry");
                    entry.read(&mut src, 64).unwrap()
                };
                assert_eq!(
                    value(Directory::MakerNote, MAKER_NOTE_VERSION),
                    Value::Undefined(b"0211".to_vec()),
                    "{what}"
                );
                let offset = u32::try_from(block.jpeg - HEADER_AT).unwrap();
                assert_eq!(
                    value(PREVIEW, JPEG_INTERCHANGE_FORMAT),
                    Value::Long(vec![offset]),
                    "{what}"
                );
            }
        }
    }

    #[test]
    fn a_makernote_value_stored_out_of_line_lies_where_its_base_says() {
        // The serial number, counted from the inner header: read from the
        // start of the file or of the structure, it would miss.
        for order in ORDERS {
            for pos in [0, POS] {
                let tags = tags_block(order, |inner, ifd| {
                    inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
                })
                .0;
                let file = exif_file(order, &tags, whole(&tags), |_, _| {});
                let (mut src, makernote) = makernote_at(&file, pos);
                let view = view_of(&makernote);
                let entry = view
                    .entry(Directory::MakerNote, SERIAL_NUMBER)
                    .expect("the serial number");
                let range = entry.byte_range().expect("stored out of line");
                let block = makernote.ifd.block();
                assert!(block.bounds().start + HEADER_AT <= range.start, "{order:?}");
                assert!(range.end <= block.bounds().end, "{order:?}");
                let mut text = SERIAL.as_bytes().to_vec();
                text.push(0);
                assert_eq!(entry.read(&mut src, 64).unwrap(), Value::Ascii(text));
            }
        }
    }

    #[test]
    fn a_headerless_makernote_and_its_preview_ifd_are_listed() {
        // Offsets counted from the header of the structure that holds the
        // MakerNote, in its byte order, wherever that structure lies.
        for order in ORDERS {
            for pos in [0, POS] {
                let (bytes, _, expected) = headerless_file(order, b"", None);
                let (mut src, makernote) = makernote_at(&bytes, pos);
                let view = view_of(&makernote);
                let what = format!("{order:?}, {pos}");
                assert_eq!(
                    tags_in(&view, Directory::MakerNote),
                    [ISO, PREVIEW_IFD],
                    "{what}"
                );
                assert_eq!(
                    tags_in(&view, PREVIEW),
                    [JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH],
                    "{what}"
                );
                let iso = view.entry(Directory::MakerNote, ISO).unwrap();
                assert_eq!(iso.read(&mut src, 64).unwrap(), Value::Short(vec![0, 200]));
                let jpeg = view.entry(PREVIEW, JPEG_INTERCHANGE_FORMAT).unwrap();
                let offset = u32::try_from(expected.offset).unwrap();
                assert_eq!(jpeg.read(&mut src, 64).unwrap(), Value::Long(vec![offset]));
            }
        }
    }

    #[test]
    fn a_value_past_the_makernotes_declared_length_is_skipped_and_counted() {
        for order in ORDERS {
            // In the MakerNote IFD: the serial number's last byte lies
            // past the length the MakerNote's entry declares.
            let tags = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            for (count, skipped) in [(whole(&tags) - 1, 1), (whole(&tags), 0)] {
                let file = exif_file(order, &tags, count, |_, _| {});
                let (_, makernote) = makernote_at(&file, 0);
                let view = view_of(&makernote);
                let what = format!("{order:?}, declared {count} bytes");
                assert_eq!(
                    view.skipped_entries(Directory::MakerNote),
                    skipped,
                    "{what}"
                );
                let listed = if skipped == 0 {
                    vec![MAKER_NOTE_VERSION, SERIAL_NUMBER]
                } else {
                    vec![MAKER_NOTE_VERSION]
                };
                assert_eq!(tags_in(&view, Directory::MakerNote), listed, "{what}");
            }

            // In the PreviewIFD: its first entry's value lies just past the
            // block, inside the file.
            let block = block(order, Change::JpegOffsetPastTheBlockFirst);
            let (_, makernote) = makernote_at(&with_block(order, &block), 0);
            let view = view_of(&makernote);
            assert_eq!(view.skipped_entries(PREVIEW), 1, "{order:?}");
            assert_eq!(view.skipped_entries(Directory::MakerNote), 0, "{order:?}");
            assert_eq!(
                tags_in(&view, PREVIEW),
                [JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH],
                "{order:?}"
            );
        }
    }

    /// What the raw view lists for a PreviewIFD tag that breaks its rule:
    /// the link, with no offset, as damage.
    fn misshaped_link() -> SkippedDirectory {
        SkippedDirectory::new(
            Directory::MakerNote,
            Some(PREVIEW_IFD),
            None,
            ErrorKind::Malformed,
        )
    }

    #[test]
    fn a_makernote_without_a_preview_ifd_is_listed_alone() {
        // The pointer stored out of line, as two LONGs, leads to no
        // PreviewIFD: the link is listed as skipped, and the entry is one
        // of the MakerNote IFD's like any other.
        for order in ORDERS {
            let block = block(order, Change::PointerOutOfLine);
            let (_, makernote) = makernote_at(&with_block(order, &block), 0);
            let view = view_of(&makernote);
            let directories: Vec<Directory> = view.directories().to_vec();
            assert_eq!(directories, [Directory::MakerNote], "{order:?}");
            assert_eq!(
                tags_in(&view, Directory::MakerNote),
                [MAKER_NOTE_VERSION, PREVIEW_IFD],
                "{order:?}"
            );
            assert_eq!(view.in_directory(PREVIEW).len(), 0, "{order:?}");
            assert_eq!(view.skipped_directories(), [misshaped_link()], "{order:?}");
        }
    }

    /// The layouts that read a PreviewIFD, for the tests that run on both.
    #[derive(Clone, Copy, Debug)]
    enum PreviewLayout {
        Embedded,
        Headerless,
    }

    /// A sound file of `layout` in byte order `order`, and where in it the
    /// value field of the MakerNote IFD's PreviewIFD entry lies: the
    /// second entry in both, after MakerNoteVersion or the ISO.
    fn with_preview_entry(order: Order, layout: PreviewLayout) -> (Vec<u8>, usize) {
        let (bytes, at) = match layout {
            PreviewLayout::Embedded => {
                let block = block(order, Change::Nothing);
                (with_block(order, &block), AT + block.pointer_at())
            }
            PreviewLayout::Headerless => {
                let (bytes, range, _) = headerless_file(order, b"", None);
                (bytes, range.start + 2 + 12 + 8)
            }
        };
        (bytes, usize::try_from(at).unwrap())
    }

    #[test]
    fn a_preview_ifd_tag_of_another_type_or_count_is_listed_as_skipped() {
        // Both layouts, in a NEF file, at 0, and in a JPEG file's Exif
        // segment, at POS.
        for order in ORDERS {
            for layout in [PreviewLayout::Embedded, PreviewLayout::Headerless] {
                for pos in [0, POS] {
                    let (sound, at) = with_preview_entry(order, layout);
                    let field: [u8; 4] = sound[at..at + 4].try_into().unwrap();
                    // The MakerNote of the file with the PreviewIFD entry
                    // rewritten with `tag`, `typ`, `count` and `field`.
                    let rewritten = |tag: u16, typ: u16, count: u32, field: [u8; 4]| {
                        let mut bytes = sound.clone();
                        bytes[at - 8..at - 6].copy_from_slice(&order.u16(tag));
                        bytes[at - 6..at - 4].copy_from_slice(&order.u16(typ));
                        bytes[at - 4..at].copy_from_slice(&order.u32(count));
                        bytes[at..at + 4].copy_from_slice(&field);
                        makernote_at(&bytes, pos).1
                    };
                    let what = format!("{order:?}, {layout:?}, {pos}");

                    // One LONG, as Nikon writes it, is followed.
                    let makernote = rewritten(PREVIEW_IFD, LONG, 1, field);
                    let view = view_of(&makernote);
                    assert_eq!(
                        view.directories(),
                        [Directory::MakerNote, PREVIEW],
                        "{what}"
                    );
                    assert_eq!(view.skipped_directories(), [], "{what}");
                    assert!(makernote.preview_candidate().is_some(), "{what}");

                    // The offset as one SHORT holds it, then two zeros.
                    let offset = u16::try_from(byte_order(order).u32(field)).unwrap();
                    let mut short = [0; 4];
                    short[..2].copy_from_slice(&order.u16(offset));
                    for (typ, count, field, shape) in [
                        (SHORT, 1, short, "one SHORT"),
                        (LONG, 2, field, "two LONGs"),
                        (LONG, 0, field, "no value"),
                        (BYTE, 1, field, "one BYTE"),
                        (IFD, 1, field, "one IFD"),
                    ] {
                        let makernote = rewritten(PREVIEW_IFD, typ, count, field);
                        let view = view_of(&makernote);
                        let what = format!("{what}, {shape}");
                        assert_eq!(view.directories(), [Directory::MakerNote], "{what}");
                        let tags = tags_in(&view, Directory::MakerNote);
                        assert!(tags.contains(&PREVIEW_IFD), "{what}");
                        assert_eq!(view.skipped_directories(), [misshaped_link()], "{what}");
                        assert_eq!(makernote.preview_candidate(), None, "{what}");
                    }

                    // The entry under the next tag: the MakerNote has no
                    // PreviewIFD tag, and nothing is left out.
                    let makernote = rewritten(PREVIEW_IFD + 1, LONG, 1, field);
                    let view = view_of(&makernote);
                    assert_eq!(view.directories(), [Directory::MakerNote], "{what}");
                    assert_eq!(view.skipped_directories(), [], "{what}");
                }
            }
        }
    }

    #[test]
    fn a_makernote_ifd_already_listed_takes_its_preview_ifd_with_it() {
        // A MakerNote IFD at the position of a directory listed before it,
        // as a damaged file may point its MakerNote at IFD1: neither it nor
        // the PreviewIFD it leads to is listed, so that no PreviewIFD is
        // listed without its MakerNote. Nor is a PreviewIFD tag that is not
        // one LONG listed as skipped.
        for order in ORDERS {
            for change in [Change::Nothing, Change::PointerOutOfLine] {
                let block = block(order, change);
                let (_, makernote) = makernote_at(&with_block(order, &block), 0);
                let mut entries = Entries::new();
                entries.push(Directory::Ifd(1), &makernote.ifd);
                makernote.add_entries(&mut entries);
                let directories: Vec<Directory> = entries.directories().to_vec();
                assert_eq!(directories, [Directory::Ifd(1)], "{order:?}, {change:?}");
                assert_eq!(entries.skipped_directories(), [], "{order:?}, {change:?}");
            }
        }
    }

    #[test]
    fn a_preview_ifd_that_is_the_makernote_ifd_is_listed_once() {
        // A PreviewIFD pointer that leads back to the MakerNote IFD: one
        // directory, two links, listed under the first.
        for order in ORDERS {
            let mut block = block(order, Change::Nothing);
            let at = block.pointer_at();
            block.put_u32(at, u32::try_from(block.makernote - HEADER_AT).unwrap());
            let (_, makernote) = makernote_at(&with_block(order, &block), 0);
            assert!(makernote.preview.as_ref().found().is_some(), "{order:?}");
            let view = view_of(&makernote);
            let directories: Vec<Directory> = view.directories().to_vec();
            assert_eq!(directories, [Directory::MakerNote], "{order:?}");
        }
    }

    #[test]
    fn a_reader_failure_is_an_error() {
        // The block far away, so that reading its signature fails; and
        // with its signature before FAR and its inner header from FAR on,
        // so that reading the header fails.
        for at in [2 * FAR, FAR - HEADER_AT] {
            let block = block(Order::Little, Change::Nothing);
            let bytes = nikon_file(Order::Little, at, &block.bytes, 0, |builder, exif| {
                entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, block.len());
            });
            // The file is sound, for a reader that does not fail.
            assert_eq!(candidate(bytes.clone()), Some(block.candidate(at)), "{at}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err =
                read_candidate(&mut src, &tree, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{at}: {err:?}");
        }
    }
}
