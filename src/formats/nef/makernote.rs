// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote, as far as the preview reader needs it: where it
//! keeps its small preview JPEG.
//!
//! The MakerNote is a block of Nikon's own metadata, the value of the
//! MakerNote tag of the Exif IFD. Among much else, it points at a
//! PreviewIFD, a small IFD whose JPEGInterchangeFormat and
//! JPEGInterchangeFormatLength declare a small JPEG, 640 by 424 pixels or
//! smaller. Every Nikon body since the D70 writes one, and on the oldest
//! bodies it is the file's only JPEG.
//!
//! Nikon has used three layouts for the block, told apart by their first
//! bytes. The one every body since the D70 and the D100 writes, and the one
//! read here, starts with the signature `Nikon` and a NUL, two version
//! bytes and two more; then, 10 bytes in, comes a TIFF header of its own: a
//! byte-order mark, 42 and the offset of the MakerNote IFD. That inner
//! header is the base of every offset inside the block, the MakerNote
//! IFD's, the PreviewIFD's and the JPEG's, never the start of the file, and
//! its byte order, which may differ from the file's, is the order of every
//! number inside the block. The two older layouts, a bare IFD with no
//! signature and an IFD after the 8 bytes `Nikon\0\1\0`, count their
//! offsets from the file's own header instead. They are not read in this
//! release: a file that has one gives no MakerNote preview.
//!
//! Nothing in the block is trusted. Every offset found inside it is checked
//! against the block, not only against the file: the MakerNote IFD and the
//! PreviewIFD must lie inside the block, with their whole entry tables; the
//! three numbers that lead to the JPEG must be held in their entries, not
//! stored elsewhere; and the whole JPEG, as declared, must lie inside the
//! block. A block that does not hold together, whether its inner header
//! is not one, an IFD in it is cut short or has more entries than the
//! limits allow, costs the small preview, never the file: the NEF security
//! notes ask that bad entries be skipped rather than fail the file
//! (`docs/formats/nef/security.md`, item 4 of the checklist). Only a
//! failure of the reader itself is an error: that is not the file's fault,
//! and says nothing about the MakerNote.

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::tags::{
    JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKER_NOTE,
};
use crate::container::tiff::types::{BYTE, UNDEFINED};
use crate::container::tiff::{Entry, Ifd, Tree, parse_header_at, parse_ifd, skip_damage};
use crate::error::Result;
use crate::formats::tiff_previews::Candidate;
use crate::io::{ByteOrder, Source, within};
use crate::limits::Limits;

/// The Nikon MakerNote tag that points at the PreviewIFD, the sub-IFD
/// holding the small preview JPEG.
const PREVIEW_IFD: u16 = 0x0011;

/// The signature a Nikon MakerNote starts with.
const SIGNATURE: &[u8; 6] = b"Nikon\0";

/// Where the inner TIFF header sits in the block: after the signature, two
/// version bytes and two more.
const HEADER_AT: u64 = 10;

/// The shortest block of the layout read here: everything up to the end of
/// the inner TIFF header, which is 8 bytes long.
const MIN_LEN: u64 = HEADER_AT + 8;

/// Where the preview JPEG of the MakerNote is, as a candidate for the
/// shared finder, with its absolute offset; `None` when the file has no
/// Exif IFD, no MakerNote, a MakerNote of another layout or maker, or one
/// whose structure does not hold together.
///
/// See the [module documentation](self) for the layout and the checks.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn preview_candidate<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<Candidate>> {
    let Some(block) = tree
        .exif()
        .and_then(|exif| exif.entry(MAKER_NOTE))
        .and_then(block_of)
    else {
        return Ok(None);
    };
    // The block's bytes lie inside the file, as every kept entry's value
    // does, so the reads below that stay inside the block can fail only
    // with the reader.
    if block
        .end
        .checked_sub(block.start)
        .is_none_or(|len| len < MIN_LEN)
        || src.bytes(block.start, 6)?.as_slice() != SIGNATURE
    {
        return Ok(None);
    }

    let Some(base) = block.start.checked_add(HEADER_AT) else {
        return Ok(None);
    };
    let Some(header) = skip_damage(parse_header_at(src, base))? else {
        return Ok(None);
    };
    let Some(makernote) = ifd_in(src, &block, base, header.ifd0, header.order, limits)? else {
        return Ok(None);
    };
    let Some(pointer) = inline_u32(src, &makernote, PREVIEW_IFD)? else {
        return Ok(None);
    };
    let Some(preview) = ifd_in(src, &block, base, pointer, header.order, limits)? else {
        return Ok(None);
    };
    let offset = inline_u32(src, &preview, JPEG_INTERCHANGE_FORMAT)?;
    let length = inline_u32(src, &preview, JPEG_INTERCHANGE_FORMAT_LENGTH)?;
    let (Some(offset), Some(length)) = (offset, length) else {
        return Ok(None);
    };

    // An empty JPEG is passed on: the finder drops it with the others.
    let length = u64::from(length);
    match base.checked_add(u64::from(offset)) {
        Some(offset) if fits(&block, offset, length) => Ok(Some(Candidate { offset, length })),
        _ => Ok(None),
    }
}

/// Where the MakerNote's bytes are in the file, from its entry: `None`
/// unless the entry holds bytes, of type UNDEFINED or BYTE, stored out of
/// line, as a block of 18 bytes or more always is.
///
/// The value of a kept entry lies inside the file, so the range does too.
fn block_of(entry: &Entry) -> Option<Range<u64>> {
    if !matches!(entry.typ(), UNDEFINED | BYTE) {
        return None;
    }
    let start = entry.value_offset()?;
    let end = start.checked_add(u64::from(entry.count()))?;
    Some(start..end)
}

/// The IFD at `offset` from `base`, read in byte order `order` with its
/// own offsets counted from `base`; `None` when it does not lie inside
/// `block` with its count and all its entries, or cannot be read.
///
/// The IFD's next-IFD pointer is not used, and not required: the D200
/// writes none.
///
/// # Errors
///
/// `Io` when the reader fails.
fn ifd_in<R: Read + Seek>(
    src: &mut Source<R>,
    block: &Range<u64>,
    base: u64,
    offset: u32,
    order: ByteOrder,
    limits: &Limits,
) -> Result<Option<Ifd>> {
    let Some(at) = base.checked_add(u64::from(offset)) else {
        return Ok(None);
    };
    // The count first, inside the block and so inside the file, then the
    // entries it announces. The 16-bit count bounds the table to
    // 2 + 12 * 65,535 bytes: this cannot overflow.
    if !fits(block, at, 2) {
        return Ok(None);
    }
    let table = 2 + 12 * u64::from(src.u16_at(at, order)?);
    if !fits(block, at, table) {
        return Ok(None);
    }
    Ok(skip_damage(parse_ifd(src, base, at, order, limits))?.map(|(ifd, _)| ifd))
}

/// The value of tag `tag` of `ifd`, as [`Ifd::u32`] reads it, when the
/// entry holds it in its own value field; `None` when the tag is missing,
/// of another type, or stored out of line.
///
/// A PreviewIFD pointer, a JPEG offset and a JPEG length are each one
/// number, which an entry always holds itself. One stored out of line
/// could be read from anywhere in the file, outside the block.
///
/// # Errors
///
/// `Io` when the reader fails.
fn inline_u32<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd, tag: u16) -> Result<Option<u32>> {
    if ifd
        .entry(tag)
        .is_none_or(|entry| entry.value_offset().is_some())
    {
        return Ok(None);
    }
    Ok(skip_damage(ifd.u32(src, tag))?.flatten())
}

/// Whether the `len` bytes at `at` lie inside `block`.
fn fits(block: &Range<u64>, at: u64, len: u64) -> bool {
    at >= block.start && within(at, len, block.end).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::MAKE;
    use crate::container::tiff::{parse_header, walk};
    use crate::error::ErrorKind;
    use crate::jpeg_stub::jpeg_stub;
    use crate::tiff_builder::{ASCII, IfdId, Order, RawValue, TiffBuilder};
    use std::io::{self, Cursor, SeekFrom};

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// Where the tests put the MakerNote block in the file.
    const AT: u64 = 1024;

    /// How far the file goes on past the block, so that what a test points
    /// past the block still lies inside the file.
    const TAIL: u64 = 4096;

    /// The Exif tag UserComment, UNDEFINED like the MakerNote: a tag that
    /// may hold any bytes and is not the MakerNote.
    const USER_COMMENT: u16 = 0x9286;

    /// The Nikon MakerNote tag MakerNoteVersion.
    const MAKER_NOTE_VERSION: u16 = 0x0001;

    /// What a test changes in the inner TIFF structure of a MakerNote
    /// block.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Change {
        /// Nothing: a sound block.
        Nothing,
        /// The inner header's 42 is 41.
        Magic41,
        /// The PreviewIFD declares no JPEG.
        NoJpegTags,
        /// The PreviewIFD declares the JPEG's offset, not its length.
        NoJpegLength,
        /// The PreviewIFD pointer is stored out of line, inside the block,
        /// as two LONGs whose first is the right offset.
        PointerOutOfLine,
    }

    /// A MakerNote block and where its parts are, each counted from the
    /// start of the block.
    struct Block {
        bytes: Vec<u8>,
        order: Order,
        /// The MakerNote IFD.
        makernote: u64,
        /// The PreviewIFD.
        preview: u64,
        /// The JPEG: 10, where the inner structure starts, plus the JPEG's
        /// offset in it.
        jpeg: u64,
        /// The JPEG's length.
        jpeg_len: u64,
    }

    impl Block {
        /// The block's length.
        fn len(&self) -> u64 {
            u64::try_from(self.bytes.len()).unwrap()
        }

        /// Writes `value` at `at`, in the block's byte order.
        fn put_u32(&mut self, at: u64, value: u32) {
            let at = usize::try_from(at).unwrap();
            self.bytes[at..at + 4].copy_from_slice(&self.order.u32(value));
        }

        /// Where the value field of the MakerNote IFD's PreviewIFD entry
        /// is: the second entry, after MakerNoteVersion, 8 bytes in.
        fn pointer_at(&self) -> u64 {
            self.makernote + 2 + 12 + 8
        }

        /// Where the value field of the PreviewIFD's
        /// JPEGInterchangeFormatLength entry is: the second entry, after
        /// JPEGInterchangeFormat, 8 bytes in.
        fn length_at(&self) -> u64 {
            self.preview + 2 + 12 + 8
        }

        /// The candidate the block gives when it sits at `at` in the file.
        fn candidate(&self, at: u64) -> Candidate {
            Candidate {
                offset: at + self.jpeg,
                length: self.jpeg_len,
            }
        }
    }

    /// A Nikon MakerNote block of the embedded-TIFF-header layout, in byte
    /// order `order`, changed as `change` says: `Nikon\0`, the version
    /// `02 11`, two zero bytes, then an inner TIFF structure whose offsets
    /// count from its own first byte, as the MakerNote's do. Its MakerNote
    /// IFD holds MakerNoteVersion and the PreviewIFD pointer; its PreviewIFD
    /// declares a 640 by 424 stub JPEG, the last bytes of the block.
    fn block(order: Order, change: Change) -> Block {
        let mut inner = TiffBuilder::new(order);
        let makernote = inner.ifd();
        let preview = inner.ifd();
        inner.undefined(makernote, MAKER_NOTE_VERSION, b"0211");
        if change == Change::PointerOutOfLine {
            // The PreviewIFD's offset depends only on the IFD before it,
            // so a copy with a stand-in value of the same length gives it.
            let mut probe = inner.clone();
            probe.long(makernote, PREVIEW_IFD, &[0, 0]);
            let offset = u32::try_from(probe.build().ifd_offset(preview)).unwrap();
            inner.long(makernote, PREVIEW_IFD, &[offset, 0]);
        } else {
            inner.ifd_pointer(makernote, PREVIEW_IFD, preview);
        }
        if change == Change::Magic41 {
            inner.magic(41);
        }
        let stub = jpeg_stub(640, 424, 0xC0);
        let jpeg = inner.blob(&stub);
        if change != Change::NoJpegTags {
            inner.long_blob(preview, JPEG_INTERCHANGE_FORMAT, jpeg);
        }
        if !matches!(change, Change::NoJpegTags | Change::NoJpegLength) {
            inner.long_blob_len(preview, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg);
        }
        let built = inner.build();

        let mut bytes = b"Nikon\0\x02\x11\0\0".to_vec();
        bytes.extend_from_slice(&built.bytes);
        Block {
            bytes,
            order,
            makernote: HEADER_AT + built.ifd_offset(makernote),
            preview: HEADER_AT + built.ifd_offset(preview),
            jpeg: HEADER_AT + built.blob_offset(jpeg),
            jpeg_len: u64::try_from(stub.len()).unwrap(),
        }
    }

    /// A Nikon file in byte order `order`: IFD0 holds Make and points at an
    /// Exif IFD, which holds ExifVersion and what `exif` adds. `bytes` lie
    /// at `at`, and the file ends `tail` bytes after them.
    fn nikon_file(
        order: Order,
        at: u64,
        bytes: &[u8],
        tail: u64,
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Vec<u8> {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif_ifd = builder.ifd();
        builder
            .ascii(ifd0, MAKE, "NIKON CORPORATION")
            .exif(ifd0, exif_ifd);
        builder.undefined(exif_ifd, 0x9000, b"0232");
        exif(&mut builder, exif_ifd);
        builder.blob_at(at, bytes);
        builder.pad_to(at + u64::try_from(bytes.len()).unwrap() + tail);
        builder.build().bytes
    }

    /// Adds to `ifd` an entry of tag `tag` and type `typ` whose value is
    /// the `count` bytes at `at`.
    fn entry_at(builder: &mut TiffBuilder, ifd: IfdId, tag: u16, typ: u16, at: u64, count: u64) {
        let count = u32::try_from(count).unwrap();
        let at = u32::try_from(at).unwrap();
        builder.raw(ifd, tag, typ, count, RawValue::Offset(at));
    }

    /// A Nikon file in byte order `order` holding `bytes` at [`AT`], whose
    /// Exif IFD's MakerNote, of type `typ`, is the first `count` of them.
    fn with_makernote(order: Order, typ: u16, bytes: &[u8], count: u64) -> Vec<u8> {
        nikon_file(order, AT, bytes, TAIL, |builder, exif| {
            entry_at(builder, exif, MAKER_NOTE, typ, AT, count);
        })
    }

    /// A Nikon file in byte order `order` whose MakerNote is `block`.
    fn with_block(order: Order, block: &Block) -> Vec<u8> {
        with_makernote(order, UNDEFINED, &block.bytes, block.len())
    }

    /// What [`preview_candidate`] returns for the file `bytes`, read with
    /// the default limits, under `limits`.
    fn try_candidate(bytes: Vec<u8>, limits: &Limits) -> Result<Option<Candidate>> {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
        preview_candidate(&mut src, &tree, limits)
    }

    /// What [`preview_candidate`] returns for the file `bytes`, which must
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
        let mut block = block(Order::Little, Change::Nothing);
        block.bytes[..6].copy_from_slice(b"Canon\0");
        assert_eq!(candidate(with_block(Order::Little, &block)), None);
    }

    #[test]
    fn the_short_header_layout_has_none() {
        // `Nikon\0\1\0`, then an IFD: the sound block's MakerNote IFD and
        // what follows it.
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            let mut bytes = b"Nikon\0\x01\0".to_vec();
            bytes.extend_from_slice(&sound.bytes[usize::try_from(sound.makernote).unwrap()..]);
            let count = u64::try_from(bytes.len()).unwrap();
            let file = with_makernote(order, UNDEFINED, &bytes, count);
            assert_eq!(candidate(file), None, "{order:?}");
        }
    }

    #[test]
    fn the_headerless_layout_has_none() {
        // A bare IFD: the sound block's MakerNote IFD and what follows it.
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            let bytes = &sound.bytes[usize::try_from(sound.makernote).unwrap()..];
            let count = u64::try_from(bytes.len()).unwrap();
            let file = with_makernote(order, UNDEFINED, bytes, count);
            assert_eq!(candidate(file), None, "{order:?}");
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
        let found = try_candidate(bytes.clone(), &Limits::default().max_ifd_entries(2));
        assert_eq!(found.unwrap(), Some(block.candidate(AT)));
        let found = try_candidate(bytes, &Limits::default().max_ifd_entries(1));
        assert_eq!(found.unwrap(), None);
    }

    /// The first byte of the second block the source caches: a reader that
    /// fails from here on still serves the walk, which reads only the
    /// first.
    const FAR: u64 = 64 * 1024;

    /// A reader that fails every read from [`FAR`] on.
    struct FailsFar(Cursor<Vec<u8>>);

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
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
            let err =
                preview_candidate(&mut src, &tree, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{at}: {err:?}");
        }
    }
}
