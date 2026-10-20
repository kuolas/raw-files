// SPDX-License-Identifier: MIT OR Apache-2.0

//! The reading of an IFD inside a block: a bounded part of the file, with a
//! base of its own, whose IFDs and values must all lie inside it.
//!
//! A MakerNote is such a block. It is a camera maker's own metadata, stored
//! as the value of one tag, and so a range of the file with bounds of its
//! own; inside it, the maker lays out IFDs, their entries and their values.
//! Nothing the block holds is trusted, so every offset found inside it is
//! checked against the block, not only against the file: an IFD must lie
//! inside the block with its count and its whole entry table, and an entry
//! whose value lies outside the block is dropped when the IFD is read, as
//! an entry whose value lies outside the file is. So every value read from
//! the IFD lies inside the block. The IFD's next-IFD pointer is never used.
//!
//! Where the offsets inside a block count from, its base, and the byte order
//! of its numbers differ from maker to maker, and the caller gives both. The
//! Nikon MakerNote counts its offsets from the TIFF header it holds, 10
//! bytes in, in that header's byte order, which may differ from the file's.
//! A Sony MakerNote counts them from the file's own header, so from base 0,
//! in the file's byte order. The reading takes both as parameters, so that
//! one reading can serve any MakerNote, and lives here rather than in one
//! format's folder. The Nikon reader uses it; no Sony MakerNote is read.
//!
//! [`block_of`] gives where a block's bytes are from the entry whose value
//! it is, [`fits`] whether a range lies inside a block, and [`ifd_in`] the
//! IFD at an offset from the block's base, read without the entries whose
//! values lie outside the block.

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::types::{BYTE, UNDEFINED};
use crate::container::tiff::{Entry, Ifd, parse_ifd, skip_damage};
use crate::error::Result;
use crate::io::{ByteOrder, Source, within};
use crate::limits::Limits;

/// Where a block's bytes are in the file, from the entry whose value they
/// are, such as the Exif IFD's MakerNote: `None` unless the entry holds
/// bytes, of type UNDEFINED or BYTE, stored out of line, as a block longer
/// than the four bytes an entry holds itself always is.
///
/// The value of a kept entry lies inside the file, so the range does too.
pub(crate) fn block_of(entry: &Entry) -> Option<Range<u64>> {
    if !matches!(entry.typ(), UNDEFINED | BYTE) {
        return None;
    }
    let start = entry.value_offset()?;
    let end = start.checked_add(u64::from(entry.count()))?;
    Some(start..end)
}

/// The IFD at `offset` from `base`, read in byte order `order` with its
/// own offsets counted from `base`, without the entries whose value lies
/// outside `block`; `None` when it does not lie inside `block` with its
/// count and all its entries, or cannot be read.
///
/// An entry whose value is stored elsewhere is dropped unless the value
/// [`fits`] the block: [`parse_ifd`] checks it only against the file.
/// A value the entry holds is inside the block with the entry. So every
/// value read from the IFD lies inside the block.
///
/// The IFD's next-IFD pointer is not used, and not required: the D200
/// writes none in its MakerNote.
///
/// # Errors
///
/// `Io` when the reader fails.
pub(crate) fn ifd_in<R: Read + Seek>(
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
    let Some((mut ifd, _)) = skip_damage(parse_ifd(src, base, at, order, limits))? else {
        return Ok(None);
    };
    ifd.retain(|entry| {
        entry
            .value_offset()
            .is_none_or(|at| fits(block, at, u64::from(entry.value_len())))
    });
    Ok(Some(ifd))
}

/// Whether the `len` bytes at `at` lie inside `block`.
pub(crate) fn fits(block: &Range<u64>, at: u64, len: u64) -> bool {
    at >= block.start && within(at, len, block.end).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiff_builder::{ASCII, IfdId, LONG, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// Where the tests that place a block inside a larger file put it.
    const AT: u64 = 512;

    /// How far such a file goes on past the block, so that what a test
    /// points past the block still lies inside the file.
    const TAIL: u64 = 256;

    // Tags that mean nothing here, for entries the tests read back by tag,
    // in the order an IFD sorts them in.
    const FIRST: u16 = 0x0101;
    const SECOND: u16 = 0x0102;
    const THIRD: u16 = 0x0103;
    const FOURTH: u16 = 0x0104;

    fn byte_order(order: Order) -> ByteOrder {
        match order {
            Order::Little => ByteOrder::Little,
            Order::Big => ByteOrder::Big,
        }
    }

    fn open(bytes: Vec<u8>) -> Source<Cursor<Vec<u8>>> {
        Source::new(Cursor::new(bytes)).unwrap()
    }

    /// The file `bytes`, read through a source, and what [`ifd_in`] gives
    /// for the IFD at `offset` from `base` inside `block`, in byte order
    /// `order`, which must not be an error.
    fn read(
        bytes: Vec<u8>,
        block: &Range<u64>,
        base: u64,
        offset: u32,
        order: Order,
    ) -> (Source<Cursor<Vec<u8>>>, Option<Ifd>) {
        let mut src = open(bytes);
        let ifd = ifd_in(
            &mut src,
            block,
            base,
            offset,
            byte_order(order),
            &Limits::default(),
        )
        .unwrap();
        (src, ifd)
    }

    /// `bytes` placed at [`AT`] in a file of zeros that goes on [`TAIL`]
    /// bytes past them; and the range they take, the block.
    fn placed(bytes: &[u8]) -> (Vec<u8>, Range<u64>) {
        let start = usize::try_from(AT).unwrap();
        let mut file = vec![0; start];
        file.extend_from_slice(bytes);
        file.resize(file.len() + usize::try_from(TAIL).unwrap(), 0);
        (file, AT..AT + u64::try_from(bytes.len()).unwrap())
    }

    /// A block in byte order `order` laid out as a TIFF structure whose
    /// offsets count from its own first byte, as the Nikon MakerNote's
    /// count from its inner header: one IFD holding a SHORT, which the
    /// entry holds, and three LONGs, stored out of line; and that IFD's
    /// offset.
    fn sound_block(order: Order) -> (Vec<u8>, u32) {
        let mut inner = TiffBuilder::new(order);
        let ifd = inner.ifd();
        inner.short(ifd, FIRST, &[7]).long(ifd, SECOND, &[1, 2, 3]);
        let built = inner.build();
        let offset = u32::try_from(built.ifd_offset(ifd)).unwrap();
        (built.bytes, offset)
    }

    /// The entry of tag `tag` in IFD0 of the file `builder` builds, as
    /// [`parse_ifd`] reads it.
    fn entry_of(builder: &TiffBuilder, ifd0: IfdId, order: Order, tag: u16) -> Option<Entry> {
        let built = builder.build();
        let at = built.ifd_offset(ifd0);
        let mut src = open(built.bytes);
        let (ifd, _) = parse_ifd(&mut src, 0, at, byte_order(order), &Limits::default()).unwrap();
        ifd.entry(tag).cloned()
    }

    /// A file in byte order `order` whose IFD0 has one entry, of tag
    /// [`FIRST`] and type `typ`, whose value is the ten bytes at [`AT`];
    /// the builder and IFD0.
    fn ten_bytes_at(order: Order, typ: u16) -> (TiffBuilder, IfdId) {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let at = u32::try_from(AT).unwrap();
        builder.raw(ifd0, FIRST, typ, 10, RawValue::Offset(at));
        builder.blob_at(AT, b"012345678\0");
        builder.pad_to(AT + TAIL);
        (builder, ifd0)
    }

    #[test]
    fn a_range_inside_the_block_fits() {
        let block = 100..200;
        assert!(fits(&block, 100, 10), "at the block's start");
        assert!(fits(&block, 190, 10), "ending at the block's end");
        assert!(fits(&block, 100, 100), "the whole block");
    }

    #[test]
    fn an_empty_range_at_the_blocks_end_fits() {
        assert!(fits(&(100..200), 200, 0));
    }

    #[test]
    fn a_range_one_byte_past_the_blocks_end_does_not_fit() {
        let block = 100..200;
        assert!(!fits(&block, 191, 10));
        assert!(!fits(&block, 200, 1));
    }

    #[test]
    fn a_range_starting_before_the_block_does_not_fit() {
        // It ends inside the block, so only its start is wrong.
        assert!(!fits(&(100..200), 99, 10));
    }

    #[test]
    fn a_range_whose_end_overflows_does_not_fit() {
        // It starts inside a block that reaches the top of the u64 range,
        // and its end, wrapped around, would be small enough to fit.
        let block = 100..u64::MAX;
        assert!(!fits(&block, u64::MAX - 5, 10));
    }

    #[test]
    fn bytes_stored_out_of_line_are_the_block() {
        for order in ORDERS {
            for typ in [UNDEFINED, BYTE] {
                let (builder, ifd0) = ten_bytes_at(order, typ);
                let entry = entry_of(&builder, ifd0, order, FIRST).expect("the entry");
                assert_eq!(block_of(&entry), Some(AT..AT + 10), "{order:?}, {typ}");
            }
        }
    }

    #[test]
    fn text_is_not_a_block() {
        // The same ten bytes, as ASCII: stored out of line too.
        for order in ORDERS {
            let (builder, ifd0) = ten_bytes_at(order, ASCII);
            let entry = entry_of(&builder, ifd0, order, FIRST).expect("the entry");
            assert_eq!(entry.value_offset(), Some(AT), "{order:?}");
            assert_eq!(block_of(&entry), None, "{order:?}");
        }
    }

    #[test]
    fn bytes_the_entry_holds_are_not_a_block() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            builder.undefined(ifd0, FIRST, b"0211");
            let entry = entry_of(&builder, ifd0, order, FIRST).expect("the entry");
            assert_eq!(entry.count(), 4, "{order:?}");
            assert_eq!(block_of(&entry), None, "{order:?}");
        }
    }

    #[test]
    fn an_ifd_inside_the_block_is_read_with_its_entries() {
        // Base 0, as a Sony MakerNote counts its offsets: the block is the
        // file's second IFD and its values, which follow it, and IFD0 lies
        // before the block.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let inside = builder.ifd();
            builder.short(ifd0, FIRST, &[1]);
            builder
                .short(inside, FIRST, &[7])
                .long(inside, SECOND, &[1, 2, 3]);
            let built = builder.build();
            let start = built.ifd_offset(inside);
            let block = start..u64::try_from(built.bytes.len()).unwrap();
            let offset = u32::try_from(start).unwrap();

            let (mut src, ifd) = read(built.bytes, &block, 0, offset, order);
            let ifd = ifd.expect("the IFD");
            assert_eq!(ifd.len(), 2, "{order:?}");
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
            assert_eq!(ifd.u32(&mut src, FIRST).unwrap(), Some(7), "{order:?}");
            assert_eq!(
                ifd.u32s(&mut src, SECOND).unwrap(),
                Some(vec![1, 2, 3]),
                "{order:?}"
            );
        }
    }

    #[test]
    fn offsets_inside_the_block_count_from_its_base() {
        // As the Nikon MakerNote's count from its inner header: the block
        // lies at AT, and so does its base.
        for order in ORDERS {
            let (bytes, offset) = sound_block(order);
            let (file, block) = placed(&bytes);
            let (mut src, ifd) = read(file, &block, AT, offset, order);
            let ifd = ifd.expect("the IFD");
            assert_eq!(ifd.offset(), AT + u64::from(offset), "{order:?}");
            assert_eq!(ifd.base(), AT, "{order:?}");
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
            assert_eq!(ifd.u32(&mut src, FIRST).unwrap(), Some(7), "{order:?}");
            assert_eq!(
                ifd.u32s(&mut src, SECOND).unwrap(),
                Some(vec![1, 2, 3]),
                "{order:?}"
            );
            let longs = ifd.entry(SECOND).and_then(Entry::value_offset);
            assert!(longs.is_some_and(|at| block.contains(&at)), "{order:?}");
        }
    }

    #[test]
    fn a_count_that_straddles_the_blocks_end_has_none() {
        for order in ORDERS {
            let (bytes, offset) = sound_block(order);
            let (file, block) = placed(&bytes);
            let at = AT + u64::from(offset);

            // The block ends one byte into the count, which the file holds
            // whole.
            let cut = block.start..at + 1;
            let ifd = read(file.clone(), &cut, AT, offset, order).1;
            assert!(ifd.is_none(), "{order:?}");

            // The block ends where the file does, one byte into the count:
            // the count is not read at all, so this is not an error.
            let end = usize::try_from(at + 1).unwrap();
            let short = file[..end].to_vec();
            assert!(
                read(short, &cut, AT, offset, order).1.is_none(),
                "{order:?}"
            );

            // The control: the whole block gives the IFD.
            let ifd = read(file, &block, AT, offset, order).1;
            assert!(ifd.is_some(), "{order:?}");
        }
    }

    #[test]
    fn an_entry_table_that_runs_past_the_block_has_none() {
        for order in ORDERS {
            let (bytes, offset) = sound_block(order);
            let (file, block) = placed(&bytes);
            // Two entries: the table ends 2 + 24 bytes into the IFD, and
            // the block one byte before. The file goes on.
            let cut = block.start..AT + u64::from(offset) + 2 + 24 - 1;
            let ifd = read(file.clone(), &cut, AT, offset, order).1;
            assert!(ifd.is_none(), "{order:?}");

            // The control: a block that ends with the table, before the
            // next-IFD pointer, gives the IFD, without the LONGs, whose
            // value lies past it.
            let table = block.start..cut.end + 1;
            let ifd = read(file, &table, AT, offset, order).1.expect("the IFD");
            assert!(ifd.entry(FIRST).is_some(), "{order:?}");
            assert!(ifd.entry(SECOND).is_none(), "{order:?}");
        }
    }

    #[test]
    fn a_value_past_the_block_is_dropped() {
        for order in ORDERS {
            // A SHORT the entry holds and three LONGs inside the block;
            // then two LONGs stored just past the block's end, and two
            // that start 4 bytes before it and so end 4 bytes past it. The
            // block ends 8 bytes after the three LONGs, and the file goes
            // on 64 bytes past it.
            let mut inner = TiffBuilder::new(order);
            let ifd = inner.ifd();
            inner.short(ifd, FIRST, &[7]).long(ifd, SECOND, &[1, 2, 3]);
            // Offsets written into entries move nothing, so a copy with
            // stand-in ones gives where the three LONGs end.
            let mut probe = inner.clone();
            probe.raw(ifd, THIRD, LONG, 2, RawValue::Offset(0)).raw(
                ifd,
                FOURTH,
                LONG,
                2,
                RawValue::Offset(0),
            );
            let end = u32::try_from(probe.build().bytes.len()).unwrap() + 8;
            inner
                .raw(ifd, THIRD, LONG, 2, RawValue::Offset(end))
                .raw(ifd, FOURTH, LONG, 2, RawValue::Offset(end - 4))
                .pad_to(u64::from(end) + 64);
            let built = inner.build();
            let offset = u32::try_from(built.ifd_offset(ifd)).unwrap();

            let (file, _) = placed(&built.bytes);
            let block = AT..AT + u64::from(end);
            let (mut src, ifd) = read(file.clone(), &block, AT, offset, order);
            let ifd = ifd.expect("the IFD");
            assert_eq!(ifd.u32(&mut src, FIRST).unwrap(), Some(7), "{order:?}");
            assert_eq!(
                ifd.u32s(&mut src, SECOND).unwrap(),
                Some(vec![1, 2, 3]),
                "{order:?}"
            );
            assert!(ifd.entry(THIRD).is_none(), "{order:?}");
            assert!(ifd.entry(FOURTH).is_none(), "{order:?}");
            assert_eq!(ifd.invalid_entries(), 2, "{order:?}");

            // The control: a block that holds them keeps them all.
            let whole = AT..AT + u64::try_from(built.bytes.len()).unwrap();
            let ifd = read(file, &whole, AT, offset, order).1.expect("the IFD");
            assert_eq!(ifd.len(), 4, "{order:?}");
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
        }
    }

    #[test]
    fn an_ifd_past_the_end_of_the_file_has_none() {
        for order in ORDERS {
            let (bytes, _) = sound_block(order);
            let (file, block) = placed(&bytes);
            let ifd = read(file, &block, AT, u32::MAX, order).1;
            assert!(ifd.is_none(), "{order:?}");
        }
    }

    #[test]
    fn a_base_and_offset_whose_sum_overflows_have_none() {
        // The sum, wrapped around, would be the offset of a sound IFD at the
        // start of a block that starts the file.
        for order in ORDERS {
            let (bytes, offset) = sound_block(order);
            let block = 0..u64::try_from(bytes.len()).unwrap();
            let wrapping = offset + 1;
            let ifd = read(bytes, &block, u64::MAX, wrapping, order).1;
            assert!(ifd.is_none(), "{order:?}");
        }
    }
}
