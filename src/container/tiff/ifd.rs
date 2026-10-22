// SPDX-License-Identifier: MIT OR Apache-2.0

//! TIFF directories (IFDs): their entries, and typed access to their values.
//!
//! An IFD (image file directory) is a table of tags that describes one image
//! in the file: its size, its compression, where its pixels or its embedded
//! JPEG are, and so on. In the file it is laid out as: a 2-byte number
//! saying how many entries follow; that many entries, 12 bytes each; and a
//! 4-byte offset pointing to the next IFD in the chain, 0 when there is
//! none. Each entry holds one tag: the tag's number, the type of its values
//! (byte, short, long, text, ...), how many values there are, and a 4-byte
//! field that holds the values themselves when they fit in four bytes,
//! left-justified, and otherwise the offset where they are stored.
//!
//! An IFD is read inside a [`Block`]: the offsets written inside it count
//! from the block's base, the start of the file for a TIFF file's own IFDs
//! and an embedded TIFF header for the IFDs of a JPEG file's Exif segment or
//! of some MakerNotes; and the IFD, with its values, must lie inside the
//! block's bounds, the whole file for a TIFF file's own IFDs and the
//! segment or the MakerNote for the others. Every offset this module
//! returns is absolute, a position in the file, except the next-IFD
//! pointer, which is returned as written.
//!
//! [`parse_ifd`] reads an IFD and checks each entry against the block and
//! the file once, while reading it. An entry whose type is unknown, whose
//! length overflows, or whose value does not lie inside the block and the
//! file is counted and skipped, not an error for the IFD: real files carry
//! such entries and still hold good previews. Every entry that is kept can
//! be read. Values are read only when asked for, through the same
//! [`Source`], and no accessor reads more than [`MAX_VALUE_LEN`] bytes for
//! one value.
//!
//! [`ifd_in`] reads the IFD at an offset written inside a block, as a
//! MakerNote's IFDs are found, and gives none when it is damaged.
//! [`Entry::byte_range`] says where a value made of bytes, such as a
//! MakerNote, lies in the file.

use std::io::{Read, Seek};
use std::ops::Range;

use super::types::{ASCII, BYTE, IFD, LONG, RATIONAL, SHORT, SRATIONAL, UNDEFINED, type_size};
use super::{Block, skip_damage};
use crate::error::{Error, Result};
use crate::io::{ByteOrder, Source, within};
use crate::limits::Limits;

/// The longest value, in bytes, that one accessor of [`Ifd`] reads.
///
/// A cap on what one tag can make raw-files allocate, independent of
/// [`Limits`]: the tags read through these accessors are numbers and short
/// texts, far below it. A longer value is a payload, such as a preview,
/// which has its own path and its own limit.
const MAX_VALUE_LEN: u32 = 1024 * 1024;

/// An IFD read by [`parse_ifd`]: the entries that can be read, and how many
/// could not.
///
/// Entries keep the order of the file, which need not be sorted by tag. A
/// tag that appears more than once is kept each time in
/// [`entries`](Self::entries), and the lookups by tag find its first
/// occurrence.
///
/// The accessors that read a value take the [`Source`] the IFD was read
/// from. Inline values are served from the entry without touching it.
#[derive(Clone, Debug)]
pub(crate) struct Ifd {
    offset: u64,
    /// The block it was read in, as given to [`parse_ifd`].
    block: Block,
    order: ByteOrder,
    entries: Vec<Entry>,
    invalid_entries: u32,
}

/// An entry of an [`Ifd`]: a field type that is known, and a value that lies
/// inside the IFD's block and the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    /// The length of the value in bytes: `count` values of the size of
    /// `typ`.
    len: u32,
    value: Value,
}

/// Where the value of an [`Entry`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Value {
    /// In the entry's value field, as written: the first `len` bytes are the
    /// value, and `len` is at most 4.
    Inline([u8; 4]),
    /// At this absolute offset, with its `len` bytes inside the block and
    /// the file.
    At(u64),
}

/// Reads the IFD at byte `offset` of `src`, inside `block`, in byte order
/// `order`.
///
/// `offset` is a position in the file. The block's base is added to every
/// offset written inside the IFD, which here means the offsets of values
/// longer than four bytes, and the IFD may use only the block's bounds:
/// the whole file for a TIFF file's own IFDs, and the MakerNote or the
/// Exif segment for the IFDs inside one.
///
/// Returns the IFD and its next-IFD pointer **as written**: relative to
/// the base, and not checked against anything. 0 means there is no next
/// IFD; whoever follows the pointer adds the base and decides whether it
/// can be read.
///
/// The IFD itself must be whole: its count and entries must lie inside the
/// block and the file. A next-IFD pointer cut off by the end of either
/// counts as 0, as other readers treat one cut off by the end of the file.
/// An entry that cannot be read is skipped and counted in
/// [`Ifd::invalid_entries`]: one whose field type is unknown, whose length
/// (count times type size) overflows 32 bits, or whose value is out of line
/// and overflows or does not lie inside the block and the file.
///
/// # Errors
///
/// `Malformed` if the IFD has no entries, or its count or entries do not
/// lie inside the block and the file; `LimitExceeded` if it has more
/// entries than [`Limits::max_ifd_entries`] allows; `Io` if the reader
/// fails. The `Malformed` and `LimitExceeded` errors are at `offset`.
/// Nothing is read before its bounds are checked, so a count or entries
/// outside the block or the file are `Malformed`, never `Io`, whatever
/// the reader would do there.
pub(crate) fn parse_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    block: &Block,
    offset: u64,
    order: ByteOrder,
    limits: &Limits,
) -> Result<(Ifd, u32)> {
    // Nothing is read before its bounds are checked, once, against the block
    // cut at the end of the file: what passes can be read, whatever bounds
    // the caller gave, and what does not is never read. A range that runs
    // past the end of the file is refused as one past the file, not the
    // block, so that a TIFF file's own IFDs, whose block is the file, fail
    // as they would without one; the count with the message of a read past
    // the end of the file.
    let file_len = src.len();
    let inside = block.within_file(file_len);
    check_inside(
        &inside,
        file_len,
        offset,
        2,
        offset,
        "data runs past the end of the file",
        "an IFD lies outside its block",
    )?;
    let count = src.u16_at(offset, order)?;
    if count == 0 {
        return Err(Error::malformed("an IFD with no entries", offset));
    }
    if u32::from(count) > limits.ifd_entries() {
        return Err(Error::limit(
            "an IFD has more entries than the limit allows",
            offset,
        ));
    }

    // The entries, in one read: one bounds check for all of them, so a
    // truncated IFD fails before any entry is read, and the entries are then
    // cut from memory with no offset arithmetic. The 16-bit count bounds the
    // read to 12 * 65,535 = 786,420 bytes: the product cannot overflow, and
    // the allocation is never larger than that.
    let past_the_file = "an IFD runs past the end of the file";
    let table_len = 12 * u64::from(count);
    let table_at = offset
        .checked_add(2)
        .ok_or_else(|| Error::malformed(past_the_file, offset))?;
    check_inside(
        &inside,
        file_len,
        table_at,
        table_len,
        offset,
        past_the_file,
        "an IFD runs past the end of its block",
    )?;
    let table = src.bytes(table_at, table_len)?;
    // The table is exactly 12 × count bytes, so there is no remainder.
    let (raw_entries, _) = table.as_chunks::<12>();

    // The next-IFD pointer follows the entries. One cut off by the end of the
    // block or the file is read as 0, no next IFD, as other readers do at
    // the end of the file.
    let next = match table_at
        .checked_add(table_len)
        .filter(|&pointer_at| inside.fits(pointer_at, 4))
    {
        Some(pointer_at) => src.u32_at(pointer_at, order)?,
        None => 0,
    };

    let mut entries = Vec::new();
    let mut invalid_entries: u32 = 0;
    for raw in raw_entries {
        match parse_entry(raw, &inside, order) {
            Some(entry) => entries.push(entry),
            // At most 65,535 entries: this cannot overflow.
            None => invalid_entries += 1,
        }
    }

    let ifd = Ifd {
        offset,
        block: block.clone(),
        order,
        entries,
        invalid_entries,
    };
    Ok((ifd, next))
}

/// Checks, before anything there is read, that the `len` bytes at `at` lie
/// inside `inside`, the block of the IFD at `offset` cut at the end of a
/// file of `file_len` bytes.
///
/// # Errors
///
/// `Malformed` at `offset` if they do not: `past_the_file` if they run past
/// the end of the file, or their end would pass the largest offset, and
/// `outside_the_block` if they lie inside the file but not inside the
/// block.
fn check_inside(
    inside: &Block,
    file_len: u64,
    at: u64,
    len: u64,
    offset: u64,
    past_the_file: &'static str,
    outside_the_block: &'static str,
) -> Result<()> {
    if inside.fits(at, len) {
        return Ok(());
    }
    let message = if within(at, len, file_len).is_none() {
        past_the_file
    } else {
        outside_the_block
    };
    Err(Error::malformed(message, offset))
}

/// The IFD at `offset`, an offset written inside `block` and so counted
/// from its base ([`Block::at`]), read in byte order `order`; `None` when
/// that position would pass the largest offset, or when [`parse_ifd`]
/// refuses the IFD as damaged ([`skip_damage`]).
///
/// For the IFDs a reader finds by offsets of its own rather than by the
/// walk, such as a MakerNote's: where the offsets inside the block count
/// from and the byte order of its numbers differ from maker to maker, and
/// the caller gives both. Nothing the block holds is trusted. [`parse_ifd`]
/// reads the IFD inside the block: it refuses an IFD that does not lie
/// inside the block with its count and its whole entry table, and leaves
/// out an entry whose value does not, as one whose value lies outside the
/// file. So every value read from the IFD lies inside the block.
///
/// The IFD's next-IFD pointer is not used, and not required: the D200
/// writes none in its MakerNote.
///
/// # Errors
///
/// `Io` when the reader fails.
pub(crate) fn ifd_in<R: Read + Seek>(
    src: &mut Source<R>,
    block: &Block,
    offset: u32,
    order: ByteOrder,
    limits: &Limits,
) -> Result<Option<Ifd>> {
    let Some(at) = block.at(offset) else {
        return Ok(None);
    };
    let parsed = skip_damage(parse_ifd(src, block, at, order, limits))?;
    Ok(parsed.map(|(ifd, _)| ifd))
}

/// Reads one 12-byte entry: `None` if its field type is unknown, its length
/// overflows 32 bits, or its value is out of line and does not fit `inside`,
/// the block cut at the end of the file, once the base is added to its
/// offset.
fn parse_entry(raw: &[u8; 12], inside: &Block, order: ByteOrder) -> Option<Entry> {
    let [t0, t1, y0, y1, c0, c1, c2, c3, f0, f1, f2, f3] = *raw;
    let tag = order.u16([t0, t1]);
    let typ = order.u16([y0, y1]);
    let count = order.u32([c0, c1, c2, c3]);
    let field = [f0, f1, f2, f3];

    let len = count.checked_mul(type_size(typ)?)?;
    let value = if len <= 4 {
        Value::Inline(field)
    } else {
        let at = inside.at(order.u32(field))?;
        if !inside.fits(at, u64::from(len)) {
            return None;
        }
        Value::At(at)
    };
    Some(Entry {
        tag,
        typ,
        count,
        len,
        value,
    })
}

// The walk and the format readers read IFDs through some of these
// accessors; the others wait for a reader that needs them.
// Each `expect` fails the build once its accessor has a caller, so it is
// then removed rather than left behind.
impl Ifd {
    /// The absolute offset of the IFD: where its entry count is.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn offset(&self) -> u64 {
        self.offset
    }

    /// The block the IFD was read in, as given to [`parse_ifd`]: the base
    /// that the offsets written inside it count from, and the bounds that
    /// it and its values lie inside.
    pub(crate) fn block(&self) -> &Block {
        &self.block
    }

    /// The byte order of the IFD's numbers, the one it was read in.
    pub(crate) fn order(&self) -> ByteOrder {
        self.order
    }

    /// The number of entries that can be read; skipped entries are not
    /// counted.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entry can be read: every entry of the IFD was skipped.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of entries skipped because their type is unknown, their
    /// length overflows or their value lies outside the IFD's block or the
    /// file.
    ///
    /// 0 means every entry of the IFD has a known type and lies inside the
    /// block and the file.
    pub(crate) fn invalid_entries(&self) -> u32 {
        self.invalid_entries
    }

    /// The entries that can be read, in the order of the file.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The tags of the entries that can be read, in the order of the file,
    /// a repeated tag each time it appears.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn tags(&self) -> impl Iterator<Item = u16> {
        self.entries.iter().map(|entry| entry.tag)
    }

    /// The first entry with tag `tag`, if there is one that can be read.
    pub(crate) fn entry(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.tag == tag)
    }

    /// The entry of tag `tag` when it has a type its tag's rule allows, one
    /// of `types`; `None` when the tag is missing or has another type.
    ///
    /// A reader checks the type with it before it reads a value, because
    /// the value readers accept more types than a tag's rule allows:
    /// [`u32s`](Self::u32s) reads LONG and IFD as well as SHORT, and
    /// [`bytes`](Self::bytes) reads BYTE as well as UNDEFINED. The entry's
    /// value, as every kept entry's, lies inside the IFD's block and the
    /// file: for an IFD read by [`ifd_in`], such as a MakerNote's, inside
    /// the block.
    ///
    /// The entry is the tag's first occurrence, the one
    /// [`entry`](Self::entry) finds. When that one has another type, a
    /// later occurrence of an allowed type does not count: the lookups by
    /// tag always mean the first occurrence, and a value reader would read
    /// that one.
    pub(crate) fn entry_of_type(&self, tag: u16, types: &[u16]) -> Option<&Entry> {
        self.entry(tag).filter(|entry| types.contains(&entry.typ))
    }

    /// The values of tag `tag`, widened to 32 bits, when its type is SHORT,
    /// LONG or IFD; `None` when the tag is missing or has another type.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` if the value is longer than [`MAX_VALUE_LEN`], at the
    /// value's offset; `Io` if the reader fails.
    pub(crate) fn u32s<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<u32>>> {
        let Some((entry, bytes)) = self.value(src, tag, &[SHORT, LONG, IFD])? else {
            return Ok(None);
        };
        let order = self.order;
        let values = if entry.typ == SHORT {
            let (shorts, _) = bytes.as_chunks::<2>();
            shorts.iter().map(|&v| u32::from(order.u16(v))).collect()
        } else {
            let (longs, _) = bytes.as_chunks::<4>();
            longs.iter().map(|&v| order.u32(v)).collect()
        };
        Ok(Some(values))
    }

    /// The first value of tag `tag`, as [`u32s`](Self::u32s) reads it;
    /// `None` also when the tag has no values.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s), which reads every value of the tag.
    pub(crate) fn u32<R: Read + Seek>(&self, src: &mut Source<R>, tag: u16) -> Result<Option<u32>> {
        Ok(self
            .u32s(src, tag)?
            .and_then(|values| values.first().copied()))
    }

    /// The fractions of tag `tag`, such as an exposure time of 10/3200 s,
    /// when its type is RATIONAL; `None` when the tag is missing or has
    /// another type.
    ///
    /// Each value is a `(numerator, denominator)` pair of unsigned 32-bit
    /// numbers, numerator first, both in the IFD's byte order. The pairs
    /// are returned as written: a denominator of 0 is kept, for the caller
    /// to judge.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    pub(crate) fn rationals<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<(u32, u32)>>> {
        self.pairs(src, tag, RATIONAL)
    }

    /// The signed fractions of tag `tag`, such as an exposure compensation
    /// of -2/3 EV, when its type is SRATIONAL; `None` when the tag is
    /// missing or has another type.
    ///
    /// Each value is a `(numerator, denominator)` pair of signed 32-bit
    /// numbers, numerator first, both in the IFD's byte order. The pairs
    /// are returned as written: a denominator of 0 is kept, for the caller
    /// to judge.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    pub(crate) fn srationals<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<(i32, i32)>>> {
        Ok(self.pairs(src, tag, SRATIONAL)?.map(|pairs| {
            pairs
                .into_iter()
                .map(|(n, d)| (n.cast_signed(), d.cast_signed()))
                .collect()
        }))
    }

    /// The text of tag `tag`, when its type is ASCII; `None` when the tag is
    /// missing or has another type.
    ///
    /// The text ends at the first NUL, or with the value when there is none.
    /// Bytes that are not UTF-8 become U+FFFD; spaces are kept, for the
    /// caller to trim.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    pub(crate) fn ascii<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<String>> {
        Ok(self
            .ascii_bytes(src, tag)?
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// The text of tag `tag` as the file stores it, when its type is ASCII:
    /// the bytes [`ascii`](Self::ascii) reads, before they are turned into
    /// a `String`, for a caller that measures or parses the bytes
    /// themselves. `None` when the tag is missing or has another type.
    ///
    /// The bytes end at the first NUL, or with the value when there is
    /// none. Bytes that are not UTF-8 are kept as they are.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    pub(crate) fn ascii_bytes<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<u8>>> {
        let Some((_, mut bytes)) = self.value(src, tag, &[ASCII])? else {
            return Ok(None);
        };
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        Ok(Some(bytes))
    }

    /// The bytes of tag `tag`, when its type is BYTE or UNDEFINED; `None`
    /// when the tag is missing or has another type. ASCII is not included:
    /// [`ascii`](Self::ascii) reads text.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    pub(crate) fn bytes<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self
            .value(src, tag, &[BYTE, UNDEFINED])?
            .map(|(_, bytes)| bytes))
    }

    /// The pairs of 32-bit numbers of tag `tag`, numerator first, both in
    /// the IFD's byte order, when its type is `typ`: what
    /// [`rationals`](Self::rationals) and [`srationals`](Self::srationals)
    /// read, before the signed ones are cast. `None` when the tag is
    /// missing or has another type.
    fn pairs<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
        typ: u16,
    ) -> Result<Option<Vec<(u32, u32)>>> {
        let Some((_, bytes)) = self.value(src, tag, &[typ])? else {
            return Ok(None);
        };
        let order = self.order;
        let (pairs, _) = bytes.as_chunks::<8>();
        let values = pairs
            .iter()
            .map(|&[n0, n1, n2, n3, d0, d1, d2, d3]| {
                (order.u32([n0, n1, n2, n3]), order.u32([d0, d1, d2, d3]))
            })
            .collect();
        Ok(Some(values))
    }

    /// The first entry with tag `tag` and its value's bytes, when its type is
    /// one of `types`.
    fn value<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
        types: &[u16],
    ) -> Result<Option<(&Entry, Vec<u8>)>> {
        let Some(entry) = self.entry_of_type(tag, types) else {
            return Ok(None);
        };
        let bytes = match entry.value {
            Value::Inline(field) => {
                let mut bytes = field.to_vec();
                // At most 4 for an inline value, so this only shortens.
                bytes.truncate(usize::try_from(entry.len).unwrap_or(usize::MAX));
                bytes
            }
            Value::At(at) => {
                if entry.len > MAX_VALUE_LEN {
                    return Err(Error::limit("a tag value is longer than 1 MiB", at));
                }
                src.bytes(at, u64::from(entry.len))?
            }
        };
        Ok(Some((entry, bytes)))
    }
}

// The format readers read an entry's count, value offset and byte range,
// which reads its value length; nothing reads its tag or its type yet.
// Each `expect` fails the build once its accessor has a caller, so it is
// then removed rather than left behind.
impl Entry {
    /// The entry's tag.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn tag(&self) -> u16 {
        self.tag
    }

    /// The entry's field type, one of the types 1 to 13.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn typ(&self) -> u16 {
        self.typ
    }

    /// The number of values, as written.
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    /// The absolute offset of the value, when it is longer than four bytes
    /// and so stored out of line; `None` when it is in the entry.
    ///
    /// The value's bytes, from this offset, lie inside the IFD's block and
    /// the file.
    pub(crate) fn value_offset(&self) -> Option<u64> {
        match self.value {
            Value::Inline(_) => None,
            Value::At(at) => Some(at),
        }
    }

    /// The length of the value in bytes: its count times the size of its
    /// type, which the entry was checked not to overflow when it was read.
    pub(crate) fn value_len(&self) -> u32 {
        self.len
    }

    /// Where the bytes of the entry's value lie in the file, when the value
    /// is made of bytes, such as a MakerNote, a maker's own metadata stored
    /// as the value of one tag: from the value's offset to its end. `None`
    /// unless the entry's type is UNDEFINED or BYTE and its value is stored
    /// out of line, as a value longer than the four bytes an entry holds
    /// itself always is.
    ///
    /// The value of a kept entry lies inside its IFD's block and the file,
    /// so the range does too.
    pub(crate) fn byte_range(&self) -> Option<Range<u64>> {
        if !matches!(self.typ, UNDEFINED | BYTE) {
            return None;
        }
        let start = self.value_offset()?;
        let end = start.checked_add(u64::from(self.value_len()))?;
        Some(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::container::tiff::types::DOUBLE;
    use crate::tiff_builder::{Built, IfdId, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    fn byte_order(order: Order) -> ByteOrder {
        match order {
            Order::Little => ByteOrder::Little,
            Order::Big => ByteOrder::Big,
        }
    }

    fn open(bytes: &[u8]) -> Source<Cursor<Vec<u8>>> {
        Source::new(Cursor::new(bytes.to_vec())).unwrap()
    }

    /// Reads the IFD at `offset` of `bytes`, with `limits`, in a block whose
    /// base is `base` and whose bounds are the whole file.
    fn try_read(
        order: Order,
        bytes: &[u8],
        base: u64,
        offset: u64,
        limits: &Limits,
    ) -> Result<(Ifd, u32)> {
        let block = Block::new(base, 0..u64::try_from(bytes.len()).unwrap());
        parse_ifd(&mut open(bytes), &block, offset, byte_order(order), limits)
    }

    /// Reads the IFD at `offset` of `bytes` in `block`, with the default
    /// limits.
    fn try_read_in(order: Order, bytes: &[u8], block: &Block, offset: u64) -> Result<(Ifd, u32)> {
        parse_ifd(
            &mut open(bytes),
            block,
            offset,
            byte_order(order),
            &Limits::default(),
        )
    }

    /// Reads `ifd` of `built`, which must succeed, and returns the source to
    /// read its values through.
    fn read(order: Order, built: &Built, ifd: IfdId) -> (Source<Cursor<Vec<u8>>>, Ifd, u32) {
        let mut src = open(&built.bytes);
        let block = Block::file(src.len());
        let (parsed, next) = parse_ifd(
            &mut src,
            &block,
            built.ifd_offset(ifd),
            byte_order(order),
            &Limits::default(),
        )
        .unwrap();
        (src, parsed, next)
    }

    fn assert_error(result: Result<(Ifd, u32)>, kind: ErrorKind, offset: u64) {
        let err = result.expect_err("the IFD cannot be read");
        assert_eq!(err.kind(), kind, "{err:?}");
        assert_eq!(err.offset(), Some(offset), "{err:?}");
    }

    /// The bytes of `built` at `at`.
    fn file_bytes(built: &Built, at: u64, len: usize) -> &[u8] {
        let at = usize::try_from(at).unwrap();
        &built.bytes[at..at + len]
    }

    /// The bytes of SRATIONAL `values` in byte order `order`.
    fn srational_bytes(order: Order, values: &[(i32, i32)]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|&(num, den)| {
                [
                    order.u32(num.cast_unsigned()),
                    order.u32(den.cast_unsigned()),
                ]
            })
            .flatten()
            .collect()
    }

    #[test]
    fn values_inline_and_out_of_line_in_both_byte_orders() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .short(ifd, 0x0001, &[0x0102])
                .short(ifd, 0x0002, &[0x0102, 0x0304])
                .short(ifd, 0x0003, &[0x0102, 0x0304, 0x0506])
                .long(ifd, 0x0004, &[0x0102_0304])
                .long(ifd, 0x0005, &[0x0102_0304, 0x0506_0708])
                .ascii(ifd, 0x0006, "abc")
                .ascii(ifd, 0x0007, "abcd")
                .ascii(ifd, 0x0008, "a longer text")
                .undefined(ifd, 0x0009, &[1, 2, 3])
                .undefined(ifd, 0x000A, &[1, 2, 3, 4, 5, 6, 7, 8, 9])
                .byte(ifd, 0x000B, &[0xFE])
                .rational(ifd, 0x000C, &[(300, 1)]);
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.len(), 12);
            assert_eq!(ifd.invalid_entries(), 0);

            assert_eq!(ifd.u32s(src, 0x0001).unwrap(), Some(vec![0x0102]));
            assert_eq!(ifd.u32s(src, 0x0002).unwrap(), Some(vec![0x0102, 0x0304]));
            assert_eq!(
                ifd.u32s(src, 0x0003).unwrap(),
                Some(vec![0x0102, 0x0304, 0x0506])
            );
            assert_eq!(ifd.u32(src, 0x0004).unwrap(), Some(0x0102_0304));
            assert_eq!(
                ifd.u32s(src, 0x0005).unwrap(),
                Some(vec![0x0102_0304, 0x0506_0708])
            );
            assert_eq!(ifd.u32(src, 0x0005).unwrap(), Some(0x0102_0304));
            assert_eq!(ifd.ascii(src, 0x0006).unwrap().as_deref(), Some("abc"));
            assert_eq!(ifd.ascii(src, 0x0007).unwrap().as_deref(), Some("abcd"));
            assert_eq!(
                ifd.ascii(src, 0x0008).unwrap().as_deref(),
                Some("a longer text")
            );
            assert_eq!(ifd.bytes(src, 0x0009).unwrap(), Some(vec![1, 2, 3]));
            assert_eq!(
                ifd.bytes(src, 0x000A).unwrap(),
                Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9])
            );
            assert_eq!(ifd.bytes(src, 0x000B).unwrap(), Some(vec![0xFE]));

            // Four bytes or fewer are inline; five or more are out of line,
            // where the file holds them.
            for (tag, inline) in [
                (0x0001, true),
                (0x0002, true),
                (0x0003, false),
                (0x0004, true),
                (0x0005, false),
                (0x0006, true),
                (0x0007, false),
                (0x0008, false),
                (0x0009, true),
                (0x000A, false),
                (0x000B, true),
                (0x000C, false),
            ] {
                let entry = ifd.entry(tag).unwrap();
                assert_eq!(entry.value_offset().is_none(), inline, "tag {tag:#06X}");
            }
            let at = ifd.entry(0x0003).unwrap().value_offset().unwrap();
            let shorts: Vec<u8> = [0x0102, 0x0304, 0x0506]
                .into_iter()
                .flat_map(|v| order.u16(v))
                .collect();
            assert_eq!(file_bytes(&built, at, 6), shorts);
            let at = ifd.entry(0x0007).unwrap().value_offset().unwrap();
            assert_eq!(file_bytes(&built, at, 5), b"abcd\0");

            // A RATIONAL's entry says where its pairs are, numerator first.
            let rational = ifd.entry(0x000C).unwrap();
            assert_eq!(
                (rational.tag(), rational.typ(), rational.count()),
                (0x000C, RATIONAL, 1)
            );
            let at = rational.value_offset().unwrap();
            assert_eq!(file_bytes(&built, at, 4), order.u32(300));
            assert_eq!(file_bytes(&built, at + 4, 4), order.u32(1));
        }
    }

    #[test]
    fn a_values_length_is_its_count_times_its_types_size() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        builder
            .byte(ifd, 0x0001, &[0xFE])
            .short(ifd, 0x0002, &[1, 2, 3])
            .long(ifd, 0x0003, &[1, 2])
            .ascii(ifd, 0x0004, "abcd")
            .undefined(ifd, 0x0005, &[1, 2, 3, 4, 5, 6, 7, 8, 9])
            .rational(ifd, 0x0006, &[(300, 1), (72, 1)])
            .raw(ifd, 0x0007, SHORT, 0, RawValue::Inline([1, 2, 3, 4]));
        let built = builder.build();
        let (_, ifd, _) = read(Order::Little, &built, ifd);
        // Inline and out of line alike; the ASCII value counts its NUL.
        for (tag, len) in [
            (0x0001, 1),
            (0x0002, 6),
            (0x0003, 8),
            (0x0004, 5),
            (0x0005, 9),
            (0x0006, 16),
            (0x0007, 0),
        ] {
            assert_eq!(ifd.entry(tag).unwrap().value_len(), len, "tag {tag:#06X}");
        }
    }

    #[test]
    fn an_inline_value_is_only_its_count() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, UNDEFINED, 2, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0002, ASCII, 3, RawValue::Inline(*b"xyzw"))
                .raw(ifd, 0x0003, SHORT, 0, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0004, ASCII, 0, RawValue::Inline(*b"wxyz"));
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.bytes(src, 0x0001).unwrap(), Some(vec![1, 2]));
            assert_eq!(ifd.ascii(src, 0x0002).unwrap().as_deref(), Some("xyz"));
            // Count 0: an empty value, not a missing tag.
            assert_eq!(ifd.u32s(src, 0x0003).unwrap(), Some(vec![]));
            assert_eq!(ifd.u32(src, 0x0003).unwrap(), None);
            assert_eq!(ifd.ascii(src, 0x0004).unwrap().as_deref(), Some(""));
        }
    }

    #[test]
    fn u32s_reads_short_long_and_ifd_only() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            let blob = builder.blob_at(512, &[order.u32(512), order.u32(0xDEAD_BEEF)].concat());
            builder
                .raw(
                    ifd,
                    0x0001,
                    IFD,
                    1,
                    RawValue::Inline(order.u32(0x0102_0304)),
                )
                .raw(ifd, 0x0002, IFD, 2, RawValue::Offset(512))
                .ascii(ifd, 0x0003, "text")
                .rational(ifd, 0x0004, &[(1, 2)])
                .byte(ifd, 0x0005, &[1])
                .short(ifd, 0x0006, &[7]);
            let built = builder.build();
            assert_eq!(built.blob_offset(blob), 512);
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;

            assert_eq!(ifd.u32s(src, 0x0001).unwrap(), Some(vec![0x0102_0304]));
            assert_eq!(ifd.u32s(src, 0x0002).unwrap(), Some(vec![512, 0xDEAD_BEEF]));
            for tag in [0x0003, 0x0004, 0x0005, 0x0099] {
                assert_eq!(ifd.u32s(src, tag).unwrap(), None, "tag {tag:#06X}");
                assert_eq!(ifd.u32(src, tag).unwrap(), None, "tag {tag:#06X}");
            }
            // Each accessor reads its own types only.
            assert_eq!(ifd.ascii(src, 0x0006).unwrap(), None);
            assert_eq!(ifd.ascii(src, 0x0005).unwrap(), None);
            assert_eq!(ifd.bytes(src, 0x0003).unwrap(), None, "not ASCII");
            assert_eq!(ifd.bytes(src, 0x0006).unwrap(), None);
            assert_eq!(ifd.ascii(src, 0x0099).unwrap(), None);
            assert_eq!(ifd.bytes(src, 0x0099).unwrap(), None);
        }
    }

    #[test]
    fn rationals_and_srationals_in_both_byte_orders() {
        let signed = [
            (-2, 3),
            (0x0102_0304, -0x0506_0708),
            (i32::MIN, i32::MAX),
            (7, 0),
        ];
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.blob_at(512, &srational_bytes(order, &signed));
            builder
                .rational(
                    ifd,
                    0x0001,
                    &[
                        (10, 3200),
                        (0x0102_0304, 0x0506_0708),
                        (u32::MAX, 1),
                        (5, 0),
                    ],
                )
                .raw(ifd, 0x0002, SRATIONAL, 4, RawValue::Offset(512))
                .raw(ifd, 0x0003, RATIONAL, 0, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0004, SRATIONAL, 0, RawValue::Inline([1, 2, 3, 4]));
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0);

            assert_eq!(
                ifd.rationals(src, 0x0001).unwrap(),
                Some(vec![
                    (10, 3200),
                    (0x0102_0304, 0x0506_0708),
                    (u32::MAX, 1),
                    (5, 0)
                ])
            );
            assert_eq!(ifd.srationals(src, 0x0002).unwrap(), Some(signed.to_vec()));
            // Count 0: an empty value, not a missing tag.
            assert_eq!(ifd.rationals(src, 0x0003).unwrap(), Some(vec![]));
            assert_eq!(ifd.srationals(src, 0x0004).unwrap(), Some(vec![]));
        }
    }

    #[test]
    fn rationals_and_srationals_read_their_own_type_only() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.blob_at(512, &srational_bytes(order, &[(-1, 3)]));
            builder.blob_at(520, &[0x3F, 0xF0, 0, 0, 0, 0, 0, 0]);
            builder
                .rational(ifd, 0x0001, &[(1, 3)])
                .raw(ifd, 0x0002, SRATIONAL, 1, RawValue::Offset(512))
                .short(ifd, 0x0003, &[1, 3])
                .long(ifd, 0x0004, &[1, 3])
                .raw(ifd, 0x0005, DOUBLE, 1, RawValue::Offset(520));
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0);

            assert_eq!(ifd.rationals(src, 0x0001).unwrap(), Some(vec![(1, 3)]));
            assert_eq!(ifd.srationals(src, 0x0002).unwrap(), Some(vec![(-1, 3)]));
            assert_eq!(ifd.srationals(src, 0x0001).unwrap(), None, "RATIONAL");
            assert_eq!(ifd.rationals(src, 0x0002).unwrap(), None, "SRATIONAL");
            for tag in [0x0003, 0x0004, 0x0005, 0x0099] {
                assert_eq!(ifd.rationals(src, tag).unwrap(), None, "tag {tag:#06X}");
                assert_eq!(ifd.srationals(src, tag).unwrap(), None, "tag {tag:#06X}");
            }
            // And the other accessors do not read them.
            assert_eq!(ifd.u32s(src, 0x0002).unwrap(), None);
            assert_eq!(ifd.bytes(src, 0x0002).unwrap(), None);
        }
    }

    #[test]
    fn a_rational_past_the_end_of_the_file_is_read_as_u32s_reads_a_long() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.blob_at(512, &srational_bytes(order, &[(-1, 3)]));
            builder
                .long(ifd, 0x0001, &[1, 3])
                .rational(ifd, 0x0002, &[(1, 3)])
                .raw(ifd, 0x0003, SRATIONAL, 1, RawValue::Offset(512));
            let built = builder.build();
            let (_, parsed, _) = read(order, &built, ifd);
            let value_at = |tag| parsed.entry(tag).unwrap().value_offset().unwrap();
            let first = [0x0001, 0x0002, 0x0003].map(value_at).into_iter().min();
            let cut = usize::try_from(first.unwrap()).unwrap();
            let at = built.ifd_offset(ifd);
            assert!(
                cut >= usize::try_from(at).unwrap() + 2 + 36 + 4,
                "after the IFD"
            );

            // In a file cut before the values, the entries are skipped: each
            // accessor finds no tag.
            let short = built.truncated(cut);
            let (cut_ifd, _) = try_read(order, &short, 0, at, &Limits::default()).unwrap();
            assert_eq!(cut_ifd.invalid_entries(), 3);
            let mut src = open(&short);
            assert_eq!(cut_ifd.u32s(&mut src, 0x0001).unwrap(), None);
            assert_eq!(cut_ifd.rationals(&mut src, 0x0002).unwrap(), None);
            assert_eq!(cut_ifd.srationals(&mut src, 0x0003).unwrap(), None);

            // Read through a shorter file than the IFD was read from, each
            // value fails as `u32s` fails, at the value's offset.
            let longs = parsed.u32s(&mut src, 0x0001).expect_err("past the end");
            let rationals = parsed.rationals(&mut src, 0x0002).map(|_| ());
            let srationals = parsed.srationals(&mut src, 0x0003).map(|_| ());
            assert_eq!(longs.offset(), Some(value_at(0x0001)), "{longs:?}");
            for (tag, result) in [(0x0002, rationals), (0x0003, srationals)] {
                let err = result.expect_err("past the end");
                assert_eq!(err.kind(), longs.kind(), "{err:?}");
                assert_eq!(err.offset(), Some(value_at(tag)), "{err:?}");
            }
        }
    }

    #[test]
    fn a_rational_longer_than_the_cap_exceeds_a_limit_as_u32s_does() {
        let order = Order::Little;
        let at = 512;
        let max = MAX_VALUE_LEN;
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder
            .raw(ifd, 0x0001, RATIONAL, max / 8, RawValue::Offset(at))
            .raw(ifd, 0x0002, RATIONAL, max / 8 + 1, RawValue::Offset(at))
            .raw(ifd, 0x0003, SRATIONAL, max / 8, RawValue::Offset(at))
            .raw(ifd, 0x0004, SRATIONAL, max / 8 + 1, RawValue::Offset(at))
            .raw(ifd, 0x0005, LONG, max / 4 + 1, RawValue::Offset(at));
        builder.pad_to(u64::from(at) + u64::from(max) + 8);
        let built = builder.build();
        let (mut src, ifd, _) = read(order, &built, ifd);
        let src = &mut src;
        assert_eq!(ifd.invalid_entries(), 0, "every value is inside the file");

        assert_eq!(
            ifd.rationals(src, 0x0001).unwrap().unwrap().len(),
            128 * 1024
        );
        assert_eq!(
            ifd.srationals(src, 0x0003).unwrap().unwrap().len(),
            128 * 1024
        );
        let longs = ifd
            .u32s(src, 0x0005)
            .expect_err("the value is over the cap");
        assert_eq!(longs.kind(), ErrorKind::LimitExceeded, "{longs:?}");
        assert_eq!(longs.offset(), Some(u64::from(at)), "{longs:?}");
        for result in [
            ifd.rationals(src, 0x0002).map(|_| ()),
            ifd.srationals(src, 0x0004).map(|_| ()),
        ] {
            let err = result.expect_err("the value is over the cap");
            assert_eq!(err.kind(), longs.kind(), "{err:?}");
            assert_eq!(err.offset(), longs.offset(), "{err:?}");
        }
    }

    #[test]
    fn ascii_ends_at_the_first_nul() {
        let order = Order::Little;
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder.blob_at(512, b"ab\0cd\0\0");
        builder.blob_at(520, b"abcdef");
        builder.blob_at(530, &[b'a', 0xFF, b'b', 0, b'c']);
        builder.blob_at(540, b"  spaced  \0");
        builder
            .raw(ifd, 0x0001, ASCII, 7, RawValue::Offset(512))
            .raw(ifd, 0x0002, ASCII, 6, RawValue::Offset(520))
            .raw(ifd, 0x0003, ASCII, 5, RawValue::Offset(530))
            .raw(ifd, 0x0004, ASCII, 11, RawValue::Offset(540))
            .raw(ifd, 0x0005, ASCII, 4, RawValue::Inline(*b"wxyz"))
            .raw(ifd, 0x0006, ASCII, 4, RawValue::Inline([b'a', 0, b'b', 0]));
        let built = builder.build();
        let (mut src, ifd, _) = read(order, &built, ifd);
        let src = &mut src;

        assert_eq!(ifd.ascii(src, 0x0001).unwrap().as_deref(), Some("ab"));
        assert_eq!(ifd.ascii(src, 0x0002).unwrap().as_deref(), Some("abcdef"));
        assert_eq!(
            ifd.ascii(src, 0x0003).unwrap().as_deref(),
            Some("a\u{FFFD}b")
        );
        assert_eq!(
            ifd.ascii(src, 0x0004).unwrap().as_deref(),
            Some("  spaced  ")
        );
        assert_eq!(ifd.ascii(src, 0x0005).unwrap().as_deref(), Some("wxyz"));
        assert_eq!(ifd.ascii(src, 0x0006).unwrap().as_deref(), Some("a"));
    }

    #[test]
    fn ascii_bytes_end_at_the_first_nul_and_keep_bytes_that_are_not_utf8() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.blob_at(512, &[b'a', 0xFF, 0xFE, b'b', 0, b'c']);
            builder.blob_at(520, b"  spaced  \0\0");
            builder
                .raw(ifd, 0x0001, ASCII, 6, RawValue::Offset(512))
                .raw(ifd, 0x0002, ASCII, 12, RawValue::Offset(520))
                .raw(ifd, 0x0003, ASCII, 4, RawValue::Inline([b'x', 0, b'y', 0]))
                .raw(ifd, 0x0004, UNDEFINED, 4, RawValue::Inline(*b"wxyz"));
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;

            let bytes = |src: &mut Source<_>, tag| ifd.ascii_bytes(src, tag).unwrap();
            assert_eq!(
                bytes(src, 0x0001).as_deref(),
                Some(&[b'a', 0xFF, 0xFE, b'b'][..]),
                "{order:?}"
            );
            assert_eq!(
                bytes(src, 0x0002).as_deref(),
                Some(&b"  spaced  "[..]),
                "{order:?}"
            );
            assert_eq!(bytes(src, 0x0003).as_deref(), Some(&b"x"[..]), "{order:?}");
            // Of type UNDEFINED, which is not text, and missing.
            assert_eq!(bytes(src, 0x0004), None, "{order:?}");
            assert_eq!(bytes(src, 0x0005), None, "{order:?}");
        }
    }

    #[test]
    fn an_ifd_with_no_entries_is_malformed() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            let built = builder.build();
            let at = built.ifd_offset(ifd);
            assert_error(
                try_read(order, &built.bytes, 0, at, &Limits::default()),
                ErrorKind::Malformed,
                at,
            );
        }
    }

    #[test]
    fn the_entry_count_is_limited() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .short(ifd, 0x0001, &[1])
                .short(ifd, 0x0002, &[2])
                .short(ifd, 0x0003, &[3]);
            let built = builder.build();
            let at = built.ifd_offset(ifd);

            let limits = Limits::default().max_ifd_entries(3);
            let (ifd, _) = try_read(order, &built.bytes, 0, at, &limits).unwrap();
            assert_eq!(ifd.len(), 3);
            let limits = Limits::default().max_ifd_entries(2);
            assert_error(
                try_read(order, &built.bytes, 0, at, &limits),
                ErrorKind::LimitExceeded,
                at,
            );
        }
    }

    #[test]
    fn a_length_that_overflows_makes_the_entry_invalid() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, LONG, u32::MAX, RawValue::Offset(8))
                // 4 × 0x4000_0001 wraps to 4 bytes: an inline value, if the
                // product were not checked.
                .raw(
                    ifd,
                    0x0002,
                    LONG,
                    0x4000_0001,
                    RawValue::Inline([1, 2, 3, 4]),
                )
                .raw(ifd, 0x0003, DOUBLE, 0x2000_0001, RawValue::Offset(8))
                .short(ifd, 0x0004, &[1]);
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            assert_eq!(ifd.invalid_entries(), 3);
            assert_eq!(ifd.len(), 1);
            assert_eq!(ifd.tags().collect::<Vec<_>>(), [0x0004]);
            for tag in [0x0001, 0x0002, 0x0003] {
                assert!(ifd.entry(tag).is_none(), "tag {tag:#06X}");
                assert_eq!(ifd.u32s(&mut src, tag).unwrap(), None);
            }
        }
    }

    #[test]
    fn a_value_must_lie_inside_the_file() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            // The only out-of-line value, so the last bytes of the file.
            builder
                .long(ifd, 0x0005, &[0x0A, 0x0B])
                .short(ifd, 0x0112, &[1]);
            let built = builder.build();
            let len = built.bytes.len();
            let (mut src, parsed, _) = read(order, &built, ifd);
            let end = parsed.entry(0x0005).unwrap().value_offset().unwrap() + 8;
            assert_eq!(end, u64::try_from(len).unwrap(), "ends at the end");
            assert_eq!(parsed.invalid_entries(), 0);
            assert_eq!(
                parsed.u32s(&mut src, 0x0005).unwrap(),
                Some(vec![0x0A, 0x0B])
            );

            // One byte short: the value is skipped, the rest of the IFD stays.
            let at = built.ifd_offset(ifd);
            let short = built.truncated(len - 1);
            let (parsed, _) = try_read(order, &short, 0, at, &Limits::default()).unwrap();
            assert_eq!(parsed.invalid_entries(), 1);
            assert!(parsed.entry(0x0005).is_none());
            assert_eq!(parsed.tags().collect::<Vec<_>>(), [0x0112]);
        }
    }

    #[test]
    fn a_value_offset_that_overflows_makes_the_entry_invalid() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, LONG, 2, RawValue::Offset(u32::MAX))
                .raw(ifd, 0x0002, LONG, 2, RawValue::Offset(8))
                .short(ifd, 0x0003, &[1]);
            let built = builder.build();
            let at = built.ifd_offset(ifd);

            let (parsed, _) = try_read(order, &built.bytes, 0, at, &Limits::default()).unwrap();
            assert_eq!(parsed.invalid_entries(), 1, "past the end");
            assert!(parsed.entry(0x0002).is_some());

            // `base + offset` overflows for the first two, and an inline value
            // does not use the base.
            let base = u64::MAX - 4;
            let (parsed, _) = try_read(order, &built.bytes, base, at, &Limits::default()).unwrap();
            assert_eq!(parsed.invalid_entries(), 2);
            assert_eq!(parsed.tags().collect::<Vec<_>>(), [0x0003]);
        }
    }

    #[test]
    fn unknown_types_are_skipped() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, 99, 1, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0002, 0, 1, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0003, 14, 1, RawValue::Inline([1, 2, 3, 4]))
                .short(ifd, 0x0004, &[1]);
            let built = builder.build();
            let (_, ifd, _) = read(order, &built, ifd);
            assert_eq!(ifd.invalid_entries(), 3);
            assert_eq!(ifd.tags().collect::<Vec<_>>(), [0x0004]);
            assert!(!ifd.is_empty());
        }
    }

    #[test]
    fn an_ifd_of_only_invalid_entries_is_empty() {
        let order = Order::Big;
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder
            .raw(ifd, 0x0001, 99, 1, RawValue::Inline([0; 4]))
            .raw(ifd, 0x0002, LONG, 2, RawValue::Offset(u32::MAX));
        let built = builder.build();
        let (_, ifd, _) = read(order, &built, ifd);
        assert!(ifd.is_empty());
        assert_eq!(ifd.len(), 0);
        assert_eq!(ifd.entries(), []);
        assert_eq!(ifd.invalid_entries(), 2);
    }

    #[test]
    fn tags_need_not_be_sorted() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .unsorted(ifd)
                .short(ifd, 0x0112, &[6])
                .ascii(ifd, 0x010F, "Nikon")
                .long(ifd, 0x0100, &[4000]);
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            assert_eq!(ifd.tags().collect::<Vec<_>>(), [0x0112, 0x010F, 0x0100]);
            assert_eq!(ifd.u32(&mut src, 0x0112).unwrap(), Some(6));
            assert_eq!(
                ifd.ascii(&mut src, 0x010F).unwrap().as_deref(),
                Some("Nikon")
            );
            assert_eq!(ifd.u32(&mut src, 0x0100).unwrap(), Some(4000));
        }
    }

    #[test]
    fn the_first_of_a_repeated_tag_wins() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .unsorted(ifd)
                .short(ifd, 0x0112, &[1])
                .short(ifd, 0x0100, &[9])
                .short(ifd, 0x0112, &[2]);
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            assert_eq!(ifd.u32(&mut src, 0x0112).unwrap(), Some(1));
            assert_eq!(ifd.len(), 3, "both are kept");
            assert_eq!(ifd.tags().collect::<Vec<_>>(), [0x0112, 0x0100, 0x0112]);
            let entries = ifd.entries();
            assert_eq!(ifd.entry(0x0112), Some(&entries[0]));
            assert_ne!(entries[0], entries[2]);
        }
    }

    /// An IFD of a SHORT at 0x0001, a LONG at 0x0002 and an ASCII text at
    /// 0x0003, read in byte order `order`.
    fn typed_ifd(order: Order) -> Ifd {
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder
            .short(ifd, 0x0001, &[7])
            .long(ifd, 0x0002, &[7])
            .ascii(ifd, 0x0003, "text");
        let built = builder.build();
        read(order, &built, ifd).1
    }

    #[test]
    fn entry_of_type_finds_an_entry_of_the_allowed_type() {
        for order in ORDERS {
            let ifd = typed_ifd(order);
            for (tag, typ) in [(0x0001, SHORT), (0x0002, LONG), (0x0003, ASCII)] {
                let entry = ifd.entry_of_type(tag, &[typ]);
                assert!(entry.is_some(), "{order:?}, tag {tag:#06X}");
                assert_eq!(entry, ifd.entry(tag), "{order:?}, tag {tag:#06X}");
            }
        }
    }

    #[test]
    fn entry_of_type_finds_none_of_another_type() {
        for order in ORDERS {
            let ifd = typed_ifd(order);
            // A LONG where only SHORT is allowed, which `u32s` would read.
            assert_eq!(ifd.entry_of_type(0x0002, &[SHORT]), None, "{order:?}");
            assert_eq!(ifd.entry_of_type(0x0001, &[LONG]), None, "{order:?}");
            assert_eq!(
                ifd.entry_of_type(0x0003, &[BYTE, UNDEFINED]),
                None,
                "{order:?}"
            );
            assert_eq!(ifd.entry_of_type(0x0001, &[]), None, "{order:?}");
        }
    }

    #[test]
    fn entry_of_type_takes_any_of_several_types() {
        for order in ORDERS {
            let ifd = typed_ifd(order);
            for tag in [0x0001, 0x0002] {
                let entry = ifd.entry_of_type(tag, &[SHORT, LONG]);
                assert!(entry.is_some(), "{order:?}, tag {tag:#06X}");
                assert_eq!(entry, ifd.entry(tag), "{order:?}, tag {tag:#06X}");
            }
            assert_eq!(ifd.entry_of_type(0x0003, &[SHORT, LONG]), None);
        }
    }

    #[test]
    fn entry_of_type_finds_none_for_a_missing_tag() {
        for order in ORDERS {
            let ifd = typed_ifd(order);
            assert_eq!(
                ifd.entry_of_type(0x0099, &[SHORT, LONG, ASCII]),
                None,
                "{order:?}"
            );
        }
    }

    #[test]
    fn entry_of_type_reads_only_the_first_of_a_repeated_tag() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .unsorted(ifd)
                .short(ifd, 0x0112, &[1])
                .long(ifd, 0x0112, &[2]);
            let built = builder.build();
            let (_, ifd, _) = read(order, &built, ifd);
            assert_eq!(ifd.len(), 2, "both are kept");
            // The first is a SHORT: the LONG after it is not found.
            assert_eq!(ifd.entry_of_type(0x0112, &[LONG]), None, "{order:?}");
            assert_eq!(
                ifd.entry_of_type(0x0112, &[SHORT]),
                Some(&ifd.entries()[0]),
                "{order:?}"
            );
        }
    }

    #[test]
    fn the_next_pointer_is_returned_as_written() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let (a, b, c) = (builder.ifd(), builder.ifd(), builder.ifd());
            for ifd in [a, b, c] {
                builder.short(ifd, 0x0112, &[1]);
            }
            builder.next(a, Some(b)).next_raw(b, 0xDEAD_BEEF);
            let built = builder.build();

            let (_, ifd, next) = read(order, &built, a);
            assert_eq!(u64::from(next), built.ifd_offset(b));
            assert_eq!(ifd.offset(), built.ifd_offset(a));
            let len = u64::try_from(built.bytes.len()).unwrap();
            assert_eq!(ifd.block(), &Block::file(len));
            assert_eq!(read(order, &built, b).2, 0xDEAD_BEEF);
            assert_eq!(read(order, &built, c).2, 0);
        }
    }

    #[test]
    fn the_byte_order_is_the_one_the_ifd_was_read_in() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.short(ifd, 0x0112, &[1]);
            let (_, ifd, _) = read(order, &builder.build(), ifd);
            assert_eq!(ifd.order(), byte_order(order), "{order:?}");
        }
    }

    #[test]
    fn an_ifd_that_runs_past_the_end_is_malformed() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.short(ifd, 0x0001, &[1]).short(ifd, 0x0002, &[2]);
            let built = builder.build();
            let at = built.ifd_offset(ifd);
            let end = usize::try_from(at).unwrap() + 2 + 24 + 4;
            assert_eq!(built.bytes.len(), end, "the IFD ends the file");
            try_read(order, &built.bytes, 0, at, &Limits::default()).unwrap();

            // Mid-entry, inside the count. (A next-IFD pointer cut off by the
            // end of the file is not malformed: see
            // `a_missing_next_pointer_means_no_next_ifd`.)
            for len in [end - 4 - 12 - 5, end - 4 - 24 - 1] {
                assert_error(
                    try_read(order, &built.truncated(len), 0, at, &Limits::default()),
                    ErrorKind::Malformed,
                    at,
                );
            }
            // At or past the end, and where `offset + 2` overflows.
            for at in [u64::try_from(end).unwrap(), 1 << 40, u64::MAX, u64::MAX - 1] {
                assert_error(
                    try_read(order, &built.bytes, 0, at, &Limits::default()),
                    ErrorKind::Malformed,
                    at,
                );
            }
        }
    }

    /// Asserts that `result` is the `Malformed` error `message` at `at`.
    fn assert_malformed(result: Result<(Ifd, u32)>, message: &str, at: u64) {
        let err = result.expect_err("the IFD cannot be read");
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
        assert_eq!(err.offset(), Some(at), "{err:?}");
        assert_eq!(err.to_string(), format!("{message} at byte {at}"));
    }

    #[test]
    fn an_ifd_past_the_end_of_the_file_is_refused_as_past_the_file() {
        // A TIFF file's own IFDs, whose block is the file: a count or an
        // entry table past its end is refused as one past the end of the
        // file, not of the block.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.short(ifd, 0x0001, &[1]).short(ifd, 0x0002, &[2]);
            let built = builder.build();
            let at = built.ifd_offset(ifd);
            let len = u64::try_from(built.bytes.len()).unwrap();

            // The count one byte short, at the end, and where `offset + 2`
            // overflows.
            for at in [len - 1, len, u64::MAX - 1, u64::MAX] {
                assert_malformed(
                    try_read(order, &built.bytes, 0, at, &Limits::default()),
                    "data runs past the end of the file",
                    at,
                );
            }

            // The count whole, and the file cut one byte into the entries.
            let cut = built.truncated(usize::try_from(at).unwrap() + 2 + 1);
            assert_malformed(
                try_read(order, &cut, 0, at, &Limits::default()),
                "an IFD runs past the end of the file",
                at,
            );
        }
    }

    /// Where reads start to fail in a [`FailsFar`]: the start of the block
    /// cache's second block, so that the reads of the first all succeed.
    const FAR: u64 = 64 * 1024;

    /// A reader that fails every read from [`FAR`] on.
    struct FailsFar(Cursor<Vec<u8>>);

    impl Read for FailsFar {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0.position() >= FAR {
                return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
            }
            self.0.read(buf)
        }
    }

    impl Seek for FailsFar {
        fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
            self.0.seek(pos)
        }
    }

    #[test]
    fn an_ifd_outside_its_block_is_refused_before_it_is_read() {
        // The IFD lies in the file, past the block and in the second block
        // of the cache, where the reader fails: nothing there is read, so
        // the IFD is refused as outside its block, not as a read that
        // failed.
        let at = FAR + 100;
        let bytes = vec![0; usize::try_from(at).unwrap() + 100];
        let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
        let block = Block::new(0, 0..FAR);
        let result = parse_ifd(&mut src, &block, at, ByteOrder::Little, &Limits::default());
        assert_malformed(result, "an IFD lies outside its block", at);

        // The count, 1, lies in the block, at its last 2 bytes, and its one
        // entry past it, where the reader fails: the entries are refused as
        // running past the block before they are read.
        let at = FAR - 2;
        let mut bytes = vec![0; usize::try_from(FAR).unwrap() + 100];
        bytes[usize::try_from(at).unwrap()] = 1;
        let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
        let result = parse_ifd(&mut src, &block, at, ByteOrder::Little, &Limits::default());
        assert_malformed(result, "an IFD runs past the end of its block", at);
    }

    #[test]
    fn a_missing_next_pointer_means_no_next_ifd() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.short(ifd, 0x0001, &[1]).short(ifd, 0x0002, &[2]);
            builder.next_raw(ifd, 0xDEAD_BEEF);
            let built = builder.build();
            let at = built.ifd_offset(ifd);
            let end = usize::try_from(at).unwrap() + 2 + 24 + 4;
            assert_eq!(built.bytes.len(), end, "the IFD ends the file");
            assert_eq!(read(order, &built, ifd).2, 0xDEAD_BEEF);

            // Cut right after the last entry, and 1 to 3 bytes into the pointer.
            for len in [end - 4, end - 3, end - 2, end - 1] {
                let cut = built.truncated(len);
                let (parsed, next) = try_read(order, &cut, 0, at, &Limits::default()).unwrap();
                assert_eq!(next, 0, "cut at {len}");
                assert_eq!(parsed.invalid_entries(), 0, "cut at {len}");
                assert_eq!(parsed.tags().collect::<Vec<_>>(), [0x0001, 0x0002]);
                let mut src = open(&cut);
                assert_eq!(parsed.u32(&mut src, 0x0001).unwrap(), Some(1));
                assert_eq!(parsed.u32(&mut src, 0x0002).unwrap(), Some(2));
            }
        }
    }

    #[test]
    fn offsets_are_relative_to_the_base() {
        for order in ORDERS {
            // An embedded TIFF file, as some MakerNotes hold, whose offsets
            // are relative to its own header.
            let mut inner = TiffBuilder::new(order);
            let ifd = inner.ifd();
            inner
                .long(ifd, 0x0001, &[0x0A, 0x0B])
                .ascii(ifd, 0x0002, "embedded")
                .short(ifd, 0x0003, &[7]);
            inner.next_raw(ifd, 0x1234);
            let inner = inner.build();

            let base = 1000;
            let mut outer = TiffBuilder::new(order);
            outer.blob_at(base, &inner.bytes);
            let outer = outer.build();

            let offset = base + inner.ifd_offset(ifd);
            let mut src = open(&outer.bytes);
            let block = Block::new(base, 0..src.len());
            let (parsed, next) = parse_ifd(
                &mut src,
                &block,
                offset,
                byte_order(order),
                &Limits::default(),
            )
            .unwrap();
            assert_eq!(parsed.offset(), offset);
            assert_eq!(parsed.block(), &block);
            assert_eq!(next, 0x1234, "relative, as written");
            assert_eq!(parsed.invalid_entries(), 0);
            assert_eq!(
                parsed.u32s(&mut src, 0x0001).unwrap(),
                Some(vec![0x0A, 0x0B])
            );
            assert_eq!(
                parsed.ascii(&mut src, 0x0002).unwrap().as_deref(),
                Some("embedded")
            );
            assert_eq!(parsed.u32(&mut src, 0x0003).unwrap(), Some(7));

            // The absolute offset is the base plus the offset as written.
            let (_, inner_ifd, _) = read(order, &inner, ifd);
            let written = inner_ifd.entry(0x0001).unwrap().value_offset().unwrap();
            assert_eq!(
                parsed.entry(0x0001).unwrap().value_offset(),
                Some(base + written)
            );
        }
    }

    #[test]
    fn a_value_longer_than_the_cap_exceeds_a_limit() {
        let order = Order::Little;
        let at = 512;
        let max = MAX_VALUE_LEN;
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder
            .raw(ifd, 0x0001, UNDEFINED, max, RawValue::Offset(at))
            .raw(ifd, 0x0002, UNDEFINED, max + 1, RawValue::Offset(at))
            .raw(ifd, 0x0003, LONG, max / 4, RawValue::Offset(at))
            .raw(ifd, 0x0004, LONG, max / 4 + 1, RawValue::Offset(at))
            .raw(ifd, 0x0005, ASCII, max + 1, RawValue::Offset(at));
        builder.pad_to(u64::from(at) + u64::from(max) + 4);
        let built = builder.build();
        let (mut src, ifd, _) = read(order, &built, ifd);
        let src = &mut src;
        assert_eq!(ifd.invalid_entries(), 0, "every value is inside the file");

        let bytes = ifd.bytes(src, 0x0001).unwrap().unwrap();
        assert_eq!(bytes.len(), 1024 * 1024);
        assert_eq!(ifd.u32s(src, 0x0003).unwrap().unwrap().len(), 256 * 1024);
        for result in [
            ifd.bytes(src, 0x0002).map(|_| ()),
            ifd.u32s(src, 0x0004).map(|_| ()),
            ifd.u32(src, 0x0004).map(|_| ()),
            ifd.ascii(src, 0x0005).map(|_| ()),
        ] {
            let err = result.expect_err("the value is over the cap");
            assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
            assert_eq!(err.offset(), Some(u64::from(at)), "{err:?}");
        }
    }

    /// A file in byte order `order` whose IFD, at 8, holds two SHORTs, which
    /// its entries hold, and the next-IFD pointer 0xDEAD_BEEF, and which goes
    /// on 100 bytes past the IFD; and the IFD's offset and end, just past its
    /// next-IFD pointer.
    fn inline_ifd(order: Order) -> (Vec<u8>, u64, u64) {
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder.short(ifd, 0x0001, &[1]).short(ifd, 0x0002, &[2]);
        builder.next_raw(ifd, 0xDEAD_BEEF);
        let at = builder.build().ifd_offset(ifd);
        let end = at + 2 + 24 + 4;
        builder.pad_to(end + 100);
        (builder.build().bytes, at, end)
    }

    #[test]
    fn an_ifd_ending_exactly_at_the_blocks_end_is_read_with_its_next_pointer() {
        for order in ORDERS {
            let (bytes, at, end) = inline_ifd(order);
            let block = Block::new(0, at..end);
            let (parsed, next) = try_read_in(order, &bytes, &block, at).unwrap();
            assert_eq!(next, 0xDEAD_BEEF, "{order:?}");
            assert_eq!(parsed.tags().collect::<Vec<_>>(), [0x0001, 0x0002]);
            assert_eq!(parsed.block(), &block, "{order:?}");
        }
    }

    #[test]
    fn a_next_pointer_cut_off_by_the_blocks_end_means_no_next_ifd() {
        for order in ORDERS {
            let (bytes, at, end) = inline_ifd(order);
            // Right after the last entry, and 1 to 3 bytes into the pointer.
            for cut in [end - 4, end - 3, end - 2, end - 1] {
                let (parsed, next) = try_read_in(order, &bytes, &Block::new(0, at..cut), at)
                    .unwrap_or_else(|err| panic!("cut at {cut} ({order:?}): {err:?}"));
                assert_eq!(next, 0, "cut at {cut} ({order:?})");
                assert_eq!(parsed.invalid_entries(), 0, "cut at {cut} ({order:?})");
                assert_eq!(parsed.tags().collect::<Vec<_>>(), [0x0001, 0x0002]);
            }
        }
    }

    #[test]
    fn an_ifd_that_runs_past_the_blocks_end_is_malformed() {
        for order in ORDERS {
            let (bytes, at, end) = inline_ifd(order);
            let table_end = end - 4;
            // Its entries one byte past the end, its count past it, and the
            // IFD starting one byte before the block.
            for block in [
                Block::new(0, at..table_end - 1),
                Block::new(0, at..at + 1),
                Block::new(0, at + 1..end),
            ] {
                let result = try_read_in(order, &bytes, &block, at);
                let err = result.expect_err("the IFD lies outside the block");
                assert_eq!(err.kind(), ErrorKind::Malformed, "{block:?}: {err:?}");
                assert_eq!(err.offset(), Some(at), "{block:?}: {err:?}");
                assert!(err.to_string().contains("block"), "{block:?}: {err}");
            }
        }
    }

    #[test]
    fn a_value_must_lie_inside_the_block() {
        for order in ORDERS {
            // A SHORT the entry holds, and two LONGs stored out of line,
            // right after the IFD, in a file that goes on past them.
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.short(ifd, 0x0001, &[1]).long(ifd, 0x0002, &[2, 3]);
            let built = builder.build();
            let at = built.ifd_offset(ifd);
            let value_end = built.ifd_block(ifd).end;
            assert_eq!(value_end, at + 2 + 24 + 4 + 8);
            builder.pad_to(value_end + 100);
            let bytes = builder.build().bytes;

            // The value ending exactly at the block's end: kept.
            let (parsed, _) =
                try_read_in(order, &bytes, &Block::new(0, at..value_end), at).unwrap();
            assert_eq!(parsed.invalid_entries(), 0, "{order:?}");
            let mut src = open(&bytes);
            assert_eq!(parsed.u32s(&mut src, 0x0002).unwrap(), Some(vec![2, 3]));

            // One byte past it: the entry is invalid, the other kept.
            let block = Block::new(0, at..value_end - 1);
            let (parsed, _) = try_read_in(order, &bytes, &block, at).unwrap();
            assert_eq!(parsed.invalid_entries(), 1, "{order:?}");
            assert_eq!(parsed.tags().collect::<Vec<_>>(), [0x0001], "{order:?}");
        }
    }

    #[test]
    fn bounds_past_the_end_of_the_file_are_cut_at_it() {
        for order in ORDERS {
            // Bounds that run to the largest offset, past the end of the file:
            // what lies past the end of the file is refused all the same, so
            // that every entry kept can be read.
            let block = Block::new(0, 0..u64::MAX);

            // A value that runs past the end of the file.
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.short(ifd, 0x0001, &[1]);
            builder.raw(ifd, 0x0002, LONG, 2, RawValue::Offset(1000));
            builder.pad_to(1004);
            let built = builder.build();
            let at = built.ifd_offset(ifd);
            let (parsed, _) = try_read_in(order, &built.bytes, &block, at).unwrap();
            assert_eq!(parsed.invalid_entries(), 1, "{order:?}");
            assert_eq!(parsed.block(), &block, "{order:?}");

            // A next-IFD pointer cut off by the end of the file.
            let (bytes, at, end) = inline_ifd(order);
            let cut = &bytes[..usize::try_from(end - 2).unwrap()];
            let (_, next) = try_read_in(order, cut, &block, at).unwrap();
            assert_eq!(next, 0, "{order:?}");
        }
    }

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

    /// The file `bytes`, read through a source, and what [`ifd_in`] gives
    /// for the IFD at `offset` from `base` inside a block of bounds
    /// `bounds`, in byte order `order`, which must not be an error.
    fn read_ifd_in(
        bytes: Vec<u8>,
        bounds: &Range<u64>,
        base: u64,
        offset: u32,
        order: Order,
    ) -> (Source<Cursor<Vec<u8>>>, Option<Ifd>) {
        let mut src = open(&bytes);
        let block = Block::new(base, bounds.clone());
        let ifd = ifd_in(
            &mut src,
            &block,
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
        let mut src = open(&built.bytes);
        let block = Block::file(src.len());
        let (ifd, _) =
            parse_ifd(&mut src, &block, at, byte_order(order), &Limits::default()).unwrap();
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
    fn bytes_stored_out_of_line_have_a_byte_range() {
        for order in ORDERS {
            for typ in [UNDEFINED, BYTE] {
                let (builder, ifd0) = ten_bytes_at(order, typ);
                let entry = entry_of(&builder, ifd0, order, FIRST).expect("the entry");
                assert_eq!(entry.byte_range(), Some(AT..AT + 10), "{order:?}, {typ}");
            }
        }
    }

    #[test]
    fn text_has_no_byte_range() {
        // The same ten bytes, as ASCII: stored out of line too.
        for order in ORDERS {
            let (builder, ifd0) = ten_bytes_at(order, ASCII);
            let entry = entry_of(&builder, ifd0, order, FIRST).expect("the entry");
            assert_eq!(entry.value_offset(), Some(AT), "{order:?}");
            assert_eq!(entry.byte_range(), None, "{order:?}");
        }
    }

    #[test]
    fn bytes_the_entry_holds_have_no_byte_range() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            builder.undefined(ifd0, FIRST, b"0211");
            let entry = entry_of(&builder, ifd0, order, FIRST).expect("the entry");
            assert_eq!(entry.count(), 4, "{order:?}");
            assert_eq!(entry.byte_range(), None, "{order:?}");
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

            let (mut src, ifd) = read_ifd_in(built.bytes, &block, 0, offset, order);
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
            let (mut src, ifd) = read_ifd_in(file, &block, AT, offset, order);
            let ifd = ifd.expect("the IFD");
            assert_eq!(ifd.offset(), AT + u64::from(offset), "{order:?}");
            assert_eq!(ifd.block().base(), AT, "{order:?}");
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
            let ifd = read_ifd_in(file.clone(), &cut, AT, offset, order).1;
            assert!(ifd.is_none(), "{order:?}");

            // The block ends where the file does, one byte into the count:
            // the count is not read at all, so this is not an error.
            let end = usize::try_from(at + 1).unwrap();
            let short = file[..end].to_vec();
            assert!(
                read_ifd_in(short, &cut, AT, offset, order).1.is_none(),
                "{order:?}"
            );

            // The control: the whole block gives the IFD.
            let ifd = read_ifd_in(file, &block, AT, offset, order).1;
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
            let ifd = read_ifd_in(file.clone(), &cut, AT, offset, order).1;
            assert!(ifd.is_none(), "{order:?}");

            // The control: a block that ends with the table, before the
            // next-IFD pointer, gives the IFD, without the LONGs, whose
            // value lies past it.
            let table = block.start..cut.end + 1;
            let ifd = read_ifd_in(file, &table, AT, offset, order)
                .1
                .expect("the IFD");
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
            let (mut src, ifd) = read_ifd_in(file.clone(), &block, AT, offset, order);
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
            let ifd = read_ifd_in(file, &whole, AT, offset, order)
                .1
                .expect("the IFD");
            assert_eq!(ifd.len(), 4, "{order:?}");
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
        }
    }

    #[test]
    fn an_ifd_past_the_end_of_the_file_has_none() {
        for order in ORDERS {
            let (bytes, _) = sound_block(order);
            let (file, block) = placed(&bytes);
            let ifd = read_ifd_in(file, &block, AT, u32::MAX, order).1;
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
            let ifd = read_ifd_in(bytes, &block, u64::MAX, wrapping, order).1;
            assert!(ifd.is_none(), "{order:?}");
        }
    }
}
