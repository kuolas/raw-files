// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote's block: its layouts, the MakerNote IFD and
//! the PreviewIFD read from it, and the PreviewIFD's JPEG as a
//! candidate preview.

use std::io::{Read, Seek};

use super::{PREVIEW_IFD, rules};
use crate::container::tiff::{Block, Ifd, Tree, ifd_in, parse_header, skip_damage};
use crate::entries::{Directory, Entries};
use crate::error::Result;
use crate::formats::makernote::MakerNoteBlock;
use crate::formats::tiff_previews::{self, Candidate};
use crate::io::Source;
use crate::limits::Limits;

/// The signature a Nikon MakerNote of the embedded layout starts with, and
/// the short-header layout too: what tells both from the headerless one.
const SIGNATURE: &[u8; 6] = b"Nikon\0";

/// Where the inner TIFF header sits in a block of the embedded layout:
/// after the signature, two version bytes and two more.
pub(super) const HEADER_AT: u64 = 10;

/// The shortest block of the embedded layout: everything up to the end of
/// the inner TIFF header, which is 8 bytes long.
const MIN_LEN: u64 = HEADER_AT + 8;

/// A Nikon MakerNote of one of the layouts read here, its block found and
/// its MakerNote IFD read: what every use of the MakerNote starts from, so
/// that the block is found, checked and parsed once per file.
///
/// See the [module documentation](super) for the layouts.
#[derive(Debug)]
pub(in crate::formats) struct MakerNote {
    /// The MakerNote IFD, which holds Nikon's tags: the IFD the inner
    /// header points at, or the one at the block's first byte in the
    /// headerless layout, inside the block with its whole entry table. Its
    /// entries whose values lie outside the block were dropped when it was
    /// read, so every value read from it lies inside the block.
    ///
    /// It was read in the block, [`Ifd::block`]: its bounds are where the
    /// block's bytes are in the file, which every offset found inside the
    /// block is checked against, and its base, which every such offset
    /// counts from, is the inner TIFF header, 10 bytes into the block, or,
    /// in the headerless layout, the header of the TIFF structure that
    /// holds the MakerNote, the Exif IFD's. Its byte order, [`Ifd::order`],
    /// is the inner header's, which may differ from the file's, or the
    /// Exif IFD's: the order of every number inside the block.
    pub(super) ifd: Ifd,
    /// The PreviewIFD, which the MakerNote IFD's entry 0x0011 points to,
    /// read in the same block, with the same base and byte order; `None`
    /// when that entry is missing, is not one number the entry holds, or
    /// leads to no IFD that holds together inside the block. Read with the
    /// MakerNote IFD, so that the preview and the raw view both take it
    /// from here.
    preview: Option<Ifd>,
}

impl MakerNote {
    /// Adds the MakerNote's directories to `entries`: the MakerNote IFD,
    /// then the PreviewIFD, under the tag of the entry that points to it,
    /// only when the MakerNote IFD was added, not already listed as a
    /// directory at the same position, so that no PreviewIFD is listed
    /// without its MakerNote.
    pub(in crate::formats) fn add_entries(&self, entries: &mut Entries) {
        if entries.push(Directory::MakerNote, &self.ifd)
            && let Some(preview) = &self.preview
        {
            entries.push(Directory::MakerNoteSub(PREVIEW_IFD), preview);
        }
    }
}

/// The Nikon MakerNote of the file in `src`, whose IFDs are `tree`, read
/// within `limits`, for a file that its format's reader has taken for a
/// Nikon one by its Make, with its PreviewIFD when it has one
/// ([`preview_ifd`]); `None` when the file has no Exif IFD, no MakerNote,
/// a MakerNote of the short-header layout, which is not read, or one whose
/// inner header or MakerNote IFD does not hold together.
///
/// The Make is not looked at here: the caller has. A block that starts
/// with the signature is of the embedded layout or none; any other block
/// is read as the headerless layout, another maker's under a Nikon Make
/// too, under the same checks.
///
/// See the [module documentation](super) for the layouts and the checks.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(in crate::formats) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    let Some(block) = MakerNoteBlock::find(tree) else {
        return Ok(None);
    };
    let head = block.first_bytes(src, 6)?;
    let ifd = if head.as_slice() == SIGNATURE {
        embedded_ifd(src, &block, limits)?
    } else {
        // The headerless layout: a bare IFD at the block's first byte, its
        // offsets counted from the header of the TIFF structure that holds
        // the MakerNote, and its numbers in that structure's byte order,
        // the Exif IFD's base and order.
        block.ifd_at(src, 0, limits)?
    };
    let Some(ifd) = ifd else {
        return Ok(None);
    };
    let preview = preview_ifd(src, &ifd, limits)?;
    Ok(Some(MakerNote { ifd, preview }))
}

/// The PreviewIFD of the MakerNote whose IFD is `ifd`, read within
/// `limits`: the IFD at the offset its entry 0x0011 holds, counted from the
/// MakerNote's base, in its byte order, inside its block; `None` when the
/// entry is missing or is not one LONG, as Nikon writes it, or the IFD does
/// not hold together inside the block.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn preview_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    limits: &Limits,
) -> Result<Option<Ifd>> {
    let Some([pointer]) = ifd.held(rules::PREVIEW_IFD) else {
        return Ok(None);
    };
    ifd_in(src, ifd.block(), pointer, ifd.order(), limits)
}

/// The MakerNote IFD of `block`, a block of the embedded layout, which
/// starts with the signature, read within `limits`: the IFD the inner
/// TIFF header points at, in the block whose base is that header; `None`
/// when the block is too short for the header, or the header or the IFD
/// does not hold together. A block of the short-header layout, which
/// starts with the signature too, is read the same way: its IFD starts 8
/// bytes in, so the bytes 10 bytes in are the start of the IFD's first
/// entry, not a TIFF header, and it gives none unless that entry's tag
/// and type happen to read as a byte-order mark and 42.
///
/// # Errors
///
/// `Io` when the reader fails.
fn embedded_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    block: &MakerNoteBlock<'_>,
    limits: &Limits,
) -> Result<Option<Ifd>> {
    let range = block.range();
    if range.end.saturating_sub(range.start) < MIN_LEN {
        return Ok(None);
    }
    let Some(base) = range.start.checked_add(HEADER_AT) else {
        return Ok(None);
    };
    let block = Block::new(base, range.clone());
    let Some(header) = skip_damage(parse_header(src, &block))? else {
        return Ok(None);
    };
    ifd_in(src, &block, header.ifd0, header.order, limits)
}

/// Where the preview JPEG of `makernote` is, as a candidate for the shared
/// finder, with its absolute offset: the JPEG its PreviewIFD declares, read
/// with the MakerNote; `None` when the MakerNote has no PreviewIFD, or one
/// that does not declare a JPEG inside the block, its JPEGInterchangeFormat
/// and JPEGInterchangeFormatLength each one LONG, as Nikon writes them.
///
/// See the [module documentation](super) for the checks. The two numbers
/// are held in their entries, so this reads nothing from the file.
pub(in crate::formats) fn preview_candidate(makernote: &MakerNote) -> Option<Candidate> {
    let preview = makernote.preview.as_ref()?;
    let [offset] = preview.held(rules::JPEG_INTERCHANGE_FORMAT)?;
    let [length] = preview.held(rules::JPEG_INTERCHANGE_FORMAT_LENGTH)?;
    // An empty JPEG is passed on: the finder drops it with the others.
    tiff_previews::candidate(preview.block(), offset, length)
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
    use crate::error::ErrorKind;
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
    /// [`preview_candidate`] return.
    fn read_candidate<R: Read + Seek>(
        src: &mut Source<R>,
        tree: &Tree,
        limits: &Limits,
    ) -> Result<Option<Candidate>> {
        let Some(makernote) = read(src, tree, limits)? else {
            return Ok(None);
        };
        Ok(preview_candidate(&makernote))
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
            let found = preview_candidate(&makernote);
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
            let found = preview_candidate(&makernote);
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

    #[test]
    fn the_short_header_layout_has_none() {
        // `Nikon\0\1\0`, then the IFD, its offsets counted from the start
        // of the file, as the sources describe the layout: read 8 bytes in,
        // it would give the preview.
        for order in ORDERS {
            let (bytes, _, _) = headerless_file(order, b"Nikon\0\x01\0", None);
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let makernote = read(&mut src, &tree, &Limits::default()).unwrap();
            assert!(makernote.is_none(), "{order:?}");

            // The control: the same IFD, with no header, is read.
            let (bytes, _, expected) = headerless_file(order, b"", None);
            assert_eq!(candidate(bytes), Some(expected), "{order:?}");
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
            let found = preview_candidate(&makernote);
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

    /// The MakerNote of the file `bytes`, walked at position `pos` of a
    /// larger file as [`placed`] places it, or as the whole file at 0: a
    /// source of the file and the MakerNote, which must be read.
    fn makernote_at(bytes: &[u8], pos: u64) -> (Source<Cursor<Vec<u8>>>, MakerNote) {
        let (mut src, tree) = if pos == 0 {
            let mut src = Source::new(Cursor::new(bytes.to_vec())).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            (src, tree)
        } else {
            assert_eq!(pos, POS);
            placed(bytes)
        };
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
                assert_eq!(view.skipped(Directory::MakerNote), 0, "{what}");
                assert_eq!(view.skipped(PREVIEW), 0, "{what}");
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
                assert_eq!(view.skipped(Directory::MakerNote), skipped, "{what}");
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
            assert_eq!(view.skipped(PREVIEW), 1, "{order:?}");
            assert_eq!(view.skipped(Directory::MakerNote), 0, "{order:?}");
            assert_eq!(
                tags_in(&view, PREVIEW),
                [JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH],
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_makernote_without_a_preview_ifd_is_listed_alone() {
        // The pointer stored out of line leads to no PreviewIFD, but is an
        // entry of the MakerNote IFD like any other.
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
        }
    }

    #[test]
    fn a_makernote_ifd_already_listed_takes_its_preview_ifd_with_it() {
        // A MakerNote IFD at the position of a directory listed before it,
        // as a damaged file may point its MakerNote at IFD1: neither it nor
        // the PreviewIFD it leads to is listed, so that no PreviewIFD is
        // listed without its MakerNote.
        for order in ORDERS {
            let block = block(order, Change::Nothing);
            let (_, makernote) = makernote_at(&with_block(order, &block), 0);
            let mut entries = Entries::new();
            entries.push(Directory::Ifd(1), &makernote.ifd);
            makernote.add_entries(&mut entries);
            let directories: Vec<Directory> = entries.directories().to_vec();
            assert_eq!(directories, [Directory::Ifd(1)], "{order:?}");
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
            assert!(makernote.preview.is_some(), "{order:?}");
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
