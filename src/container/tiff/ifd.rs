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
//! [`Source`]. An IFD keeps the cap on one value of the limits it was read
//! with, [`Limits::max_value_bytes`], and no accessor reads a longer value
//! stored out of line. The exceptions read a tag's first value alone,
//! whatever the count, and so never more than eight bytes: [`Ifd::u32`],
//! and [`Ifd::get`] with a [`First`](super::rule::First) rule. One number
//! bounds every other value, those raw-files reads for the metadata and
//! the previews as much as those a caller reads.
//!
//! There are two ways to read a value:
//!
//! - The metadata and MakerNote readers read by [`Rule`]: [`Ifd::get`] and
//!   [`Ifd::held`]. A rule names a tag's exact type and count, which are
//!   checked before anything is read (ADR011). Damage, such as a value
//!   longer than the cap, gives `None`, and a failure of the reader is the
//!   only error.
//! - The walk, the preview finder and format detection read by tag number,
//!   through the accessors such as [`Ifd::u32s`]. These pass damage up as
//!   an error, `LimitExceeded` for a value over the cap, since they skip a
//!   whole IFD on it, or decide a file's format by it.
//!
//! [`ifd_in`] reads the IFD at an offset written inside a block, as a
//! MakerNote's IFDs are found, and gives none when it is damaged.
//! [`Entry::byte_range`] says where a value made of bytes, such as a
//! MakerNote, lies in the file.

use std::io::{Read, Seek};
use std::ops::Range;

use super::rule::{Element, Held, Rule, Shape};
use super::{Block, FieldType, skip_damage};
use crate::error::{Error, Result};
use crate::io::{ByteOrder, Source, within};
use crate::limits::Limits;

/// An IFD read by [`parse_ifd`]: the entries that can be read, and how many
/// could not.
///
/// Entries keep the order of the file, which need not be sorted by tag. A
/// tag that appears more than once is kept each time in
/// [`entries`](Self::entries), and the lookups by tag find its first
/// occurrence.
///
/// The accessors that read a value take the [`Source`] the IFD was read
/// from. Inline values are served from the entry without touching it. A
/// value stored out of line is read only when it is no longer than the cap
/// the IFD was read under, but by [`u32`](Self::u32) and by
/// [`get`](Self::get) with a [`First`](super::rule::First) rule, which read
/// only its first value.
#[derive(Clone, Debug)]
pub(crate) struct Ifd {
    offset: u64,
    /// The block it was read in, as given to [`parse_ifd`].
    block: Block,
    order: ByteOrder,
    entries: Vec<Entry>,
    invalid_entries: u32,
    /// The most bytes one accessor reads for one value: the
    /// [`Limits::max_value_bytes`] of the limits given to [`parse_ifd`].
    value_cap: u64,
}

/// An entry of an [`Ifd`]: a field type that is known, and a value that lies
/// inside the IFD's block and the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Entry {
    tag: u16,
    field_type: FieldType,
    count: u32,
    /// The length of the value in bytes: `count` values of the size of
    /// `field_type`.
    len: u32,
    value: Value,
}

/// Where the value of an [`Entry`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
/// The IFD keeps `limits`' [`max_value_bytes`](Limits::max_value_bytes),
/// the cap its accessors read each value under. An entry whose value is
/// longer is kept: only reading that value fails.
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
    if u32::from(count) > limits.max_ifd_entries() {
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
        value_cap: limits.max_value_bytes(),
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
    let field_type = FieldType::from_code(order.u16([y0, y1]))?;
    let count = order.u32([c0, c1, c2, c3]);
    let field = [f0, f1, f2, f3];

    let len = count.checked_mul(field_type.size())?;
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
        field_type,
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
    /// [`get`](Self::get) and [`held`](Self::held) find a rule's entry with
    /// it, and a reader may check that a tag is there before it reads
    /// another. The value accessors accept more types than a tag's rule may
    /// allow: [`u32s`](Self::u32s) reads LONG and IFD as well as SHORT. The
    /// entry's value, as every kept entry's, lies inside the IFD's block
    /// and the file: for an IFD read by [`ifd_in`], such as a MakerNote's,
    /// inside the block.
    ///
    /// The entry is the tag's first occurrence, the one
    /// [`entry`](Self::entry) finds. When that one has another type, a
    /// later occurrence of an allowed type does not count: the lookups by
    /// tag always mean the first occurrence, and a value reader would read
    /// that one.
    pub(crate) fn entry_of_type(&self, tag: u16, types: &[FieldType]) -> Option<&Entry> {
        self.entry(tag)
            .filter(|entry| types.contains(&entry.field_type()))
    }

    /// The values of tag `tag`, widened to 32 bits, when its type is SHORT,
    /// LONG or IFD; `None` when the tag is missing or has another type.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` if the value is stored out of line and is longer than
    /// the cap the IFD was read under ([`parse_ifd`]), at the value's
    /// offset; `Io` if the reader fails.
    pub(crate) fn u32s<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<u32>>> {
        let Some((entry, bytes)) = self.value(
            src,
            tag,
            &[FieldType::Short, FieldType::Long, FieldType::Ifd],
        )?
        else {
            return Ok(None);
        };
        let order = self.order;
        let values = if entry.field_type() == FieldType::Short {
            let (shorts, _) = bytes.as_chunks::<2>();
            shorts.iter().map(|&v| u32::from(order.u16(v))).collect()
        } else {
            let (longs, _) = bytes.as_chunks::<4>();
            longs.iter().map(|&v| order.u32(v)).collect()
        };
        Ok(Some(values))
    }

    /// The first value of tag `tag`, widened to 32 bits, when its type is
    /// SHORT, LONG or IFD; `None` when the tag is missing, has another type
    /// or has no values.
    ///
    /// The first value is read alone: from the entry when the entry holds
    /// the values itself, and otherwise as the 2 bytes of a SHORT or the 4
    /// of a LONG at the values' offset. The other values are neither read
    /// nor checked, so a tag of one number that the file gives many values
    /// costs no more than one, and is never refused for its length as
    /// [`u32s`](Self::u32s) refuses it. A reader of a tag's rule reads its
    /// first value with [`get`](Self::get) and a
    /// [`First`](super::rule::First) rule, the same way.
    ///
    /// # Errors
    ///
    /// `Io` if the reader fails. The value lies inside the file, as every
    /// kept entry's does, so nothing else can go wrong.
    pub(crate) fn u32<R: Read + Seek>(&self, src: &mut Source<R>, tag: u16) -> Result<Option<u32>> {
        let Some(entry) = self.entry_of_type(tag, u32::TYPES) else {
            return Ok(None);
        };
        if entry.count == 0 {
            return Ok(None);
        }
        let bytes = entry.read_first(src)?;
        Ok(u32::decode(entry.field_type, &bytes, self.order))
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
        self.pairs(src, tag, FieldType::Rational)
    }

    /// The text of tag `tag`, when its type is ASCII; `None` when the tag is
    /// missing or has another type.
    ///
    /// The text ends at the first NUL, or with the value when there is none.
    /// Bytes that are not UTF-8 become U+FFFD; spaces are kept, for the
    /// caller to trim.
    ///
    /// Only the tests read text this way, as a `String`: the readers read
    /// text by a [`Text`](super::rule::Text) rule, and clean the bytes
    /// themselves.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    #[cfg(test)]
    pub(crate) fn ascii<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<String>> {
        Ok(self
            .text_of_type(src, tag, &[FieldType::Ascii])?
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// The text of tag `tag` as the file stores it, when its type is ASCII
    /// or UTF-8, the two types Exif 3.1 allows for many of its texts, such
    /// as the Make. `None` when the tag is missing or has another type.
    /// Format detection reads the Make this way; the readers read text by a
    /// [`Text`](super::rule::Text) rule.
    ///
    /// The bytes end at the first NUL, or with the value when there is
    /// none. Bytes that are not UTF-8 are kept as they are, whatever the
    /// type says.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    pub(crate) fn text_bytes<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<u8>>> {
        self.text_of_type(src, tag, &[FieldType::Ascii, FieldType::Utf8])
    }

    /// The bytes of tag `tag` up to the first NUL, when its type is one of
    /// `types`: what [`ascii`](Self::ascii) and
    /// [`text_bytes`](Self::text_bytes) read. `None` when the tag is
    /// missing or has another type.
    fn text_of_type<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
        types: &[FieldType],
    ) -> Result<Option<Vec<u8>>> {
        let Some((_, mut bytes)) = self.value(src, tag, types)? else {
            return Ok(None);
        };
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        Ok(Some(bytes))
    }

    /// The bytes of tag `tag`, when its type is BYTE or UNDEFINED; `None`
    /// when the tag is missing or has another type. ASCII and UTF-8 are not
    /// included: [`text_bytes`](Self::text_bytes) reads text.
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
            .value(src, tag, &[FieldType::Byte, FieldType::Undefined])?
            .map(|(_, bytes)| bytes))
    }

    /// The pairs of 32-bit numbers of tag `tag`, numerator first, both in
    /// the IFD's byte order, when its type is `field_type`: what
    /// [`rationals`](Self::rationals) reads. `None` when the tag is missing
    /// or has another type.
    fn pairs<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
        field_type: FieldType,
    ) -> Result<Option<Vec<(u32, u32)>>> {
        let Some((_, bytes)) = self.value(src, tag, &[field_type])? else {
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

    /// The value of the tag `rule` names, in the rule's shape, when its
    /// entry has one of the rule's types and a count the shape takes;
    /// `None` when the tag is missing, has another type or another count,
    /// or its value cannot be read.
    ///
    /// The type and the count are checked before anything is read, so a tag
    /// of another type or count costs no read, however long its value. That
    /// is the metadata decision's rule (ADR011), written once here for every
    /// reader of metadata. The entry is the tag's first occurrence, as for
    /// [`entry_of_type`](Self::entry_of_type).
    ///
    /// The value is read under the cap the IFD was read under, and a longer
    /// one is damage, `None`. A [`First`](super::rule::First) rule reads only
    /// the first value, at most eight bytes, whatever the count and the cap.
    /// A value held in the entry is served from it without a read: a rule
    /// whose value always fits there is a [`Held`], read by
    /// [`held`](Self::held) with no source.
    ///
    /// # Errors
    ///
    /// `Io` if the reader fails, the only error: anything else wrong with
    /// the value is damage in the file, and costs only that value.
    pub(crate) fn get<R: Read + Seek, S: Shape>(
        &self,
        src: &mut Source<R>,
        rule: Rule<S>,
    ) -> Result<Option<S::Output>> {
        let Some(entry) = self.entry_of_type(rule.tag(), rule.types()) else {
            return Ok(None);
        };
        if !S::takes(entry.count) {
            return Ok(None);
        }
        let bytes = if S::FIRST_ONLY {
            entry.read_first(src)
        } else {
            entry.read(src, self.value_cap)
        };
        let Some(bytes) = skip_damage(bytes)? else {
            return Ok(None);
        };
        Ok(S::decode(entry.field_type, bytes, self.order))
    }

    /// The value of the tag `rule` names, in the rule's shape, when its
    /// entry has one of the rule's types and a count the shape takes;
    /// `None` when the tag is missing or has another type or count.
    ///
    /// A [`Held`] rule's value is never longer than four bytes, which
    /// [`Held::new`] checks when the rule is declared. The entry holds such
    /// a value in its own field. So this reads nothing from the file, takes
    /// no source and cannot fail. A tag whose count the shape does not take,
    /// such as one SHORT given two, records nothing a reader can trust, and
    /// even its first value is not taken (ADR011).
    pub(crate) fn held<S: Shape>(&self, rule: Held<S>) -> Option<S::Output> {
        let entry = self.entry_of_type(rule.tag(), rule.types())?;
        if !S::takes(entry.count) {
            return None;
        }
        let Value::Inline(field) = entry.value else {
            return None;
        };
        let len = usize::try_from(entry.len).ok()?;
        S::decode(entry.field_type, field.get(..len)?.to_vec(), self.order)
    }

    /// The first entry with tag `tag` and its value's bytes, when its type is
    /// one of `types`, read under the cap the IFD was read under.
    fn value<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
        types: &[FieldType],
    ) -> Result<Option<(&Entry, Vec<u8>)>> {
        let Some(entry) = self.entry_of_type(tag, types) else {
            return Ok(None);
        };
        let bytes = entry.read(src, self.value_cap)?;
        Ok(Some((entry, bytes)))
    }
}

impl Entry {
    /// The entry's tag.
    pub(crate) fn tag(&self) -> u16 {
        self.tag
    }

    /// The entry's field type.
    pub(crate) fn field_type(&self) -> FieldType {
        self.field_type
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
        if !matches!(self.field_type, FieldType::Undefined | FieldType::Byte) {
            return None;
        }
        let start = self.value_offset()?;
        let end = start.checked_add(u64::from(self.value_len()))?;
        Some(start..end)
    }

    /// The bytes of the entry's value, as the file stores them, from `src`,
    /// the source its IFD was read from: what every value reader of an
    /// [`Ifd`] reads, and the raw view's values too.
    ///
    /// A value held in the entry itself, at most four bytes, is served from
    /// the entry without touching `src`, and never refused. A value stored
    /// out of line is read only when its length is at most `cap` bytes, so
    /// the most this allocates is `cap` bytes, or four.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` if the value is stored out of line and is longer than
    /// `cap`, at the value's offset, before anything is read; `Malformed` if
    /// the value does not lie inside the file `src` reads, which cannot
    /// happen for the source the entry was read from; `Io` if the reader
    /// fails, as it does when the file has shrunk since its length was
    /// taken.
    pub(crate) fn read<R: Read + Seek>(&self, src: &mut Source<R>, cap: u64) -> Result<Vec<u8>> {
        match self.value {
            Value::Inline(field) => {
                let mut bytes = field.to_vec();
                // At most 4 for an inline value, so this only shortens.
                bytes.truncate(usize::try_from(self.len).unwrap_or(usize::MAX));
                Ok(bytes)
            }
            Value::At(at) => {
                if u64::from(self.len) > cap {
                    return Err(Error::limit(
                        "a tag value is longer than the limit allows",
                        at,
                    ));
                }
                src.bytes(at, u64::from(self.len))
            }
        }
    }

    /// The bytes of the entry's first value alone, from `src`, the source
    /// its IFD was read from: one value's size, at most eight bytes, never
    /// refused for a cap; none when the entry has no values.
    ///
    /// # Errors
    ///
    /// As for [`read`](Self::read), but never `LimitExceeded`.
    fn read_first<R: Read + Seek>(&self, src: &mut Source<R>) -> Result<Vec<u8>> {
        let len = self.field_type.size().min(self.len);
        match self.value {
            Value::Inline(field) => {
                // At most 4 for an inline value, so this only shortens.
                let mut bytes = field.to_vec();
                bytes.truncate(usize::try_from(len).unwrap_or(usize::MAX));
                Ok(bytes)
            }
            Value::At(at) => src.bytes(at, u64::from(len)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::container::tiff::{First, Text};
    use crate::rational::{Rational, SignedRational};
    /// The code of field type DOUBLE, which no reader takes.
    const DOUBLE: u16 = 12;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{
        ASCII, BYTE, Built, IFD, IfdId, LONG, Order, RATIONAL, RawValue, SHORT, SLONG, SRATIONAL,
        SSHORT, TiffBuilder, UNDEFINED, UTF8,
    };
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

    /// Reads `ifd` of `built` with the default limits, which must succeed,
    /// and returns the source to read its values through.
    fn read(order: Order, built: &Built, ifd: IfdId) -> (Source<Cursor<Vec<u8>>>, Ifd, u32) {
        read_with(order, built, ifd, &Limits::default())
    }

    /// Reads `ifd` of `built` with `limits`, as [`read`] does with the
    /// default ones.
    fn read_with(
        order: Order,
        built: &Built,
        ifd: IfdId,
        limits: &Limits,
    ) -> (Source<Cursor<Vec<u8>>>, Ifd, u32) {
        let mut src = open(&built.bytes);
        let block = Block::file(src.len());
        let (parsed, next) = parse_ifd(
            &mut src,
            &block,
            built.ifd_offset(ifd),
            byte_order(order),
            limits,
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
    fn the_accessors_read_under_the_value_cap_of_the_limits_the_ifd_was_read_with() {
        for order in ORDERS {
            let at = 512;
            let mut builder = TiffBuilder::new(order);
            let id = builder.ifd();
            builder
                .raw(id, 0x0001, UNDEFINED, 100, RawValue::Offset(at))
                .short(id, 0x0002, &[7, 8]);
            builder.pad_to(u64::from(at) + 100);
            let built = builder.build();
            let limits = |n| Limits::default().with_max_value_bytes(n);

            // At the cap, and with no cap, the value is read whole.
            for limits in [limits(100), Limits::unlimited()] {
                let (mut src, ifd, _) = read_with(order, &built, id, &limits);
                assert_eq!(ifd.bytes(&mut src, 0x0001).unwrap().unwrap().len(), 100);
            }
            // A byte under it, refused at the value; the entry is still kept.
            let (mut src, ifd, _) = read_with(order, &built, id, &limits(99));
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
            let err = ifd.bytes(&mut src, 0x0001).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
            assert_eq!(err.offset(), Some(u64::from(at)), "{err:?}");
            // A value held in its entry is read under any cap, zero included.
            let (mut src, ifd, _) = read_with(order, &built, id, &limits(0));
            assert_eq!(ifd.u32s(&mut src, 0x0002).unwrap(), Some(vec![7, 8]));
            assert!(ifd.bytes(&mut src, 0x0001).is_err());
        }
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
                (rational.tag(), rational.field_type(), rational.count()),
                (0x000C, FieldType::Rational, 1)
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

            let limits = Limits::default().with_max_ifd_entries(3);
            let (ifd, _) = try_read(order, &built.bytes, 0, at, &limits).unwrap();
            assert_eq!(ifd.len(), 3);
            let limits = Limits::default().with_max_ifd_entries(2);
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
            for (tag, field_type) in [
                (0x0001, FieldType::Short),
                (0x0002, FieldType::Long),
                (0x0003, FieldType::Ascii),
            ] {
                let entry = ifd.entry_of_type(tag, &[field_type]);
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
            assert_eq!(
                ifd.entry_of_type(0x0002, &[FieldType::Short]),
                None,
                "{order:?}"
            );
            assert_eq!(
                ifd.entry_of_type(0x0001, &[FieldType::Long]),
                None,
                "{order:?}"
            );
            assert_eq!(
                ifd.entry_of_type(0x0003, &[FieldType::Byte, FieldType::Undefined]),
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
                let entry = ifd.entry_of_type(tag, &[FieldType::Short, FieldType::Long]);
                assert!(entry.is_some(), "{order:?}, tag {tag:#06X}");
                assert_eq!(entry, ifd.entry(tag), "{order:?}, tag {tag:#06X}");
            }
            assert_eq!(
                ifd.entry_of_type(0x0003, &[FieldType::Short, FieldType::Long]),
                None
            );
        }
    }

    #[test]
    fn entry_of_type_finds_none_for_a_missing_tag() {
        for order in ORDERS {
            let ifd = typed_ifd(order);
            assert_eq!(
                ifd.entry_of_type(
                    0x0099,
                    &[FieldType::Short, FieldType::Long, FieldType::Ascii]
                ),
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
            assert_eq!(
                ifd.entry_of_type(0x0112, &[FieldType::Long]),
                None,
                "{order:?}"
            );
            assert_eq!(
                ifd.entry_of_type(0x0112, &[FieldType::Short]),
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
        let max = u32::try_from(Limits::default().max_value_bytes()).unwrap();
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
            ifd.ascii(src, 0x0005).map(|_| ()),
        ] {
            let err = result.expect_err("the value is over the cap");
            assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
            assert_eq!(err.offset(), Some(u64::from(at)), "{err:?}");
        }
        // The first value alone is never over the cap.
        assert_eq!(ifd.u32(src, 0x0004).unwrap(), Some(0));
    }

    #[test]
    fn the_first_value_is_read_alone_whatever_the_count() {
        for order in ORDERS {
            // A cap of 16 bytes, which the out-of-line LONGs of 0x0001 and
            // the SHORTs of 0x0002 pass, each far longer.
            let longs: Vec<u8> = (1..=1000_u32).flat_map(|v| order.u32(v)).collect();
            let shorts: Vec<u8> = (11..=1010_u16).flat_map(|v| order.u16(v)).collect();
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, LONG, 1000, RawValue::Offset(1024))
                .raw(ifd, 0x0002, SHORT, 1000, RawValue::Offset(8192))
                .raw(ifd, 0x0003, IFD, 1000, RawValue::Offset(1024))
                .short(ifd, 0x0004, &[0x0102, 0x0304, 0x0506])
                .long(ifd, 0x0005, &[0x0102_0304, 0x0506_0708, 0x090A_0B0C])
                .short(ifd, 0x0006, &[0x0708, 0x090A])
                .raw(ifd, 0x0007, SHORT, 0, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0008, LONG, 0, RawValue::Inline([1, 2, 3, 4]))
                .raw(ifd, 0x0009, IFD, 0, RawValue::Inline([1, 2, 3, 4]));
            builder.blob_at(1024, &longs);
            builder.blob_at(8192, &shorts);
            let built = builder.build();
            let limits = Limits::default().with_max_value_bytes(16);
            let (mut src, ifd, _) = read_with(order, &built, ifd, &limits);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");

            for (tag, first) in [
                (0x0001, 1),
                (0x0002, 11),
                (0x0003, 1),
                (0x0004, 0x0102),
                (0x0005, 0x0102_0304),
                (0x0006, 0x0708),
            ] {
                assert_eq!(
                    ifd.u32(src, tag).unwrap(),
                    Some(first),
                    "{order:?} tag {tag:#06X}"
                );
            }
            // Read whole, the long values are over the cap.
            for tag in [0x0001, 0x0002, 0x0003] {
                let err = ifd.u32s(src, tag).expect_err("over the cap");
                assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{order:?} {err:?}");
            }
            // No values, whatever the entry's value field holds.
            for tag in [0x0007, 0x0008, 0x0009] {
                assert_eq!(ifd.u32(src, tag).unwrap(), None, "{order:?} {tag:#06X}");
            }
        }
    }

    #[test]
    fn the_first_value_is_read_without_the_bytes_after_it() {
        for order in ORDERS {
            // Two LONGs, and three SHORTs, whose first lies in the first block
            // the source caches and whose second runs past it, where the
            // reader fails: reading every value fails, reading the first
            // does not. The LONGs start 6 bytes before the block's end, the
            // SHORTs 2 bytes before it, just after the first LONG.
            let offset = |back: u64| RawValue::Offset(u32::try_from(FAR - back).unwrap());
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, LONG, 2, offset(6))
                .raw(ifd, 0x0002, SHORT, 3, offset(2));
            builder.blob_at(
                FAR - 6,
                &[order.u32(0x0102_0304).as_slice(), &order.u16(7)].concat(),
            );
            builder.pad_to(FAR + 16);
            let built = builder.build();
            let mut src = Source::new(FailsFar(Cursor::new(built.bytes.clone()))).unwrap();
            let block = Block::file(src.len());
            let at = built.ifd_offset(ifd);
            let (ifd, _) =
                parse_ifd(&mut src, &block, at, byte_order(order), &Limits::default()).unwrap();
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");

            assert_eq!(ifd.u32(&mut src, 0x0001).unwrap(), Some(0x0102_0304));
            assert_eq!(ifd.u32(&mut src, 0x0002).unwrap(), Some(7));
            for tag in [0x0001, 0x0002] {
                let err = ifd.u32s(&mut src, tag).expect_err("the reader fails");
                assert_eq!(err.kind(), ErrorKind::Io, "{order:?} {err:?}");
            }
        }
    }

    #[test]
    fn a_utf8_entry_is_kept_and_only_the_reader_of_either_text_takes_it() {
        for order in ORDERS {
            // Exif 3.0's UTF-8 type, inline and out of line: "é" is two
            // bytes, and the NUL is counted.
            let at = 256;
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x010F, UTF8, 3, RawValue::Inline([0xC3, 0xA9, 0, 0]))
                .raw(ifd, 0x0110, UTF8, 6, RawValue::Offset(at));
            builder.blob_at(u64::from(at), b"Z \xC3\xA9t\0");
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
            for (tag, expected) in [(0x010F, &b"\xC3\xA9\0"[..]), (0x0110, b"Z \xC3\xA9t\0")] {
                let entry = ifd.entry(tag).expect("kept");
                assert_eq!(entry.field_type(), FieldType::Utf8);
                assert_eq!(entry.read(src, 16).unwrap(), expected, "{order:?}");
                // Every typed reader checks the type: the reader of a text
                // of either type takes UTF-8, cut at its NUL; the ASCII
                // readers and the readers of numbers and bytes do not.
                let text = expected.strip_suffix(b"\0").expect("a NUL ends it");
                assert_eq!(ifd.text_bytes(src, tag).unwrap().as_deref(), Some(text));
                assert_eq!(ifd.ascii(src, tag).unwrap(), None);
                assert_eq!(
                    ifd.get(src, rule::<Text>(tag, &[FieldType::Ascii]))
                        .unwrap(),
                    None
                );
                assert_eq!(ifd.bytes(src, tag).unwrap(), None);
                assert_eq!(ifd.u32s(src, tag).unwrap(), None);
                assert_eq!(entry.byte_range(), None);
            }
        }
    }

    #[test]
    fn an_entry_is_read_under_the_cap_it_is_given() {
        for order in ORDERS {
            let at = 512;
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, UNDEFINED, 100, RawValue::Offset(at))
                .raw(ifd, 0x0002, DOUBLE, 2, RawValue::Offset(at))
                .short(ifd, 0x0003, &[7, 8]);
            builder.pad_to(u64::from(at) + 100);
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let [bytes, doubles, shorts] =
                [0x0001, 0x0002, 0x0003].map(|tag| *ifd.entry(tag).unwrap());

            // At the cap, read whole; a byte over it, refused at the value.
            assert_eq!(bytes.read(&mut src, 100).unwrap().len(), 100);
            let err = bytes.read(&mut src, 99).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
            assert_eq!(err.offset(), Some(u64::from(at)), "{err:?}");
            // Any type, DOUBLE among them, which no typed reader takes.
            assert_eq!(doubles.read(&mut src, 16).unwrap().len(), 16);
            assert!(doubles.read(&mut src, 15).is_err());
            // A value held in the entry is never refused, whatever the cap.
            let expected: Vec<u8> = [order.u16(7), order.u16(8)].concat();
            assert_eq!(shorts.read(&mut src, 0).unwrap(), expected);
        }
    }

    #[test]
    fn an_inline_value_is_read_without_touching_the_reader() {
        // The entry is read from its file, and its value then through a
        // source over no bytes, where every read fails: only the entry can
        // serve it.
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        builder.short(ifd, 0x0001, &[0x0102, 0x0304]);
        let (_, parsed, _) = read(Order::Little, &builder.build(), ifd);
        let entry = *parsed.entry(0x0001).unwrap();
        let mut empty = open(&[]);
        assert!(empty.bytes(0, 1).is_err());
        assert_eq!(entry.read(&mut empty, 0).unwrap(), [2, 1, 4, 3]);
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
        ifd.entry(tag).copied()
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
        // Base 0, as a Sony MakerNote in an ARW file counts its offsets: the
        // block is the file's second IFD and its values, which follow it,
        // and IFD0 lies before the block.
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

    // The rule readers. The tests build each rule at run time, for tags
    // that vary; the readers declare theirs as `const` items.

    /// A rule of shape `S` for tag `tag`, of `types`.
    fn rule<S: Shape>(tag: u16, types: &'static [FieldType]) -> Rule<S> {
        Rule::new(tag, types)
    }

    /// A held rule of shape `S` for tag `tag`, of `types`.
    fn held_rule<S: Shape>(tag: u16, types: &'static [FieldType]) -> Held<S> {
        Held::new(tag, types)
    }

    /// The field types a rule can name, each alone, for the rules the tests
    /// build: a rule's types live as long as the program.
    static NAMED: [FieldType; 11] = [
        FieldType::Byte,
        FieldType::Undefined,
        FieldType::Short,
        FieldType::Long,
        FieldType::Ifd,
        FieldType::SShort,
        FieldType::SLong,
        FieldType::Rational,
        FieldType::SRational,
        FieldType::Ascii,
        FieldType::Utf8,
    ];

    /// `field_type` alone, as a rule's types.
    fn only(field_type: FieldType) -> &'static [FieldType] {
        let i = NAMED
            .iter()
            .position(|&t| t == field_type)
            .expect("a type a rule can name");
        std::slice::from_ref(&NAMED[i])
    }

    /// Gives `ifd` tag `tag` of type `code` with `count` values whose bytes
    /// are `bytes`: in the entry when they fit, and otherwise at `at`.
    fn put(
        builder: &mut TiffBuilder,
        ifd: IfdId,
        tag: u16,
        code: u16,
        count: u32,
        bytes: &[u8],
        at: u32,
    ) {
        if bytes.len() <= 4 {
            let mut field = [0; 4];
            field[..bytes.len()].copy_from_slice(bytes);
            builder.raw(ifd, tag, code, count, RawValue::Inline(field));
        } else {
            builder.blob_at(u64::from(at), bytes);
            builder.raw(ifd, tag, code, count, RawValue::Offset(at));
        }
    }

    /// Checks every shape of element `T` over tags of `code`, which is
    /// `field_type`, holding the first 0, 1, 2 and 3 of `values`, each
    /// encoded by `encode`, in both byte orders: an array reads exactly its
    /// count and nothing else, `Vec` any count, zero included, and `First`
    /// the first value of any count but zero.
    fn check_shapes<T>(
        code: u16,
        field_type: FieldType,
        values: [T; 3],
        encode: fn(Order, T) -> Vec<u8>,
    ) where
        T: Element + Copy + PartialEq + std::fmt::Debug,
    {
        let types = only(field_type);
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            for k in 0..=3_u16 {
                let bytes: Vec<u8> = values[..usize::from(k)]
                    .iter()
                    .flat_map(|&v| encode(order, v))
                    .collect();
                put(
                    &mut builder,
                    ifd,
                    k,
                    code,
                    u32::from(k),
                    &bytes,
                    1024 + 64 * u32::from(k),
                );
            }
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0, "{field_type} {order:?}");
            let what = |k: u16| format!("{field_type} {order:?}: {k} values");
            let [a, b, c] = values;

            // An array: its exact count gives the values; one fewer and one
            // more give none.
            for k in 0..=3 {
                let one = ifd.get(src, rule::<[T; 1]>(k, types)).unwrap();
                let two = ifd.get(src, rule::<[T; 2]>(k, types)).unwrap();
                let three = ifd.get(src, rule::<[T; 3]>(k, types)).unwrap();
                assert_eq!(one, (k == 1).then_some([a]), "{}", what(k));
                assert_eq!(two, (k == 2).then_some([a, b]), "{}", what(k));
                assert_eq!(three, (k == 3).then_some([a, b, c]), "{}", what(k));
            }
            // `Vec`: any count, zero included.
            for k in 0..=3 {
                let all = ifd.get(src, rule::<Vec<T>>(k, types)).unwrap();
                assert_eq!(
                    all.as_deref(),
                    Some(&values[..usize::from(k)]),
                    "{}",
                    what(k)
                );
            }
            // `First`: the first value of any count but zero.
            for k in 0..=3 {
                let first = ifd.get(src, rule::<First<T>>(k, types)).unwrap();
                assert_eq!(first, (k > 0).then_some(a), "{}", what(k));
            }
            // A missing tag.
            assert_eq!(ifd.get(src, rule::<Vec<T>>(0x0099, types)).unwrap(), None);
        }
    }

    #[test]
    fn a_rule_reads_exactly_its_count_of_every_element_in_both_byte_orders() {
        check_shapes(BYTE, FieldType::Byte, [1_u8, 0xFE, 0x7F], |_, v| vec![v]);
        check_shapes(UNDEFINED, FieldType::Undefined, [0_u8, 0xFF, 9], |_, v| {
            vec![v]
        });
        let short = |order: Order, v: u32| order.u16(u16::try_from(v).unwrap()).to_vec();
        check_shapes(SHORT, FieldType::Short, [0x0102_u32, 0xFFFF, 0], short);
        let long = |order: Order, v: u32| order.u32(v).to_vec();
        check_shapes(LONG, FieldType::Long, [0x0102_0304_u32, u32::MAX, 0], long);
        check_shapes(IFD, FieldType::Ifd, [8_u32, 0x0A0B_0C0D, 0], long);
        let sshort = |order: Order, v: i16| order.u16(v.cast_unsigned()).to_vec();
        check_shapes(
            SSHORT,
            FieldType::SShort,
            [-6_i16, i16::MIN, i16::MAX],
            sshort,
        );
        let slong = |order: Order, v: i32| order.u32(v.cast_unsigned()).to_vec();
        check_shapes(SLONG, FieldType::SLong, [-3_i32, i32::MIN, i32::MAX], slong);
        let fraction = |numerator, denominator| Rational {
            numerator,
            denominator,
        };
        check_shapes(
            RATIONAL,
            FieldType::Rational,
            [fraction(10, 3200), fraction(u32::MAX, 1), fraction(5, 0)],
            |order, v| [order.u32(v.numerator), order.u32(v.denominator)].concat(),
        );
        let signed = |numerator, denominator| SignedRational {
            numerator,
            denominator,
        };
        check_shapes(
            SRATIONAL,
            FieldType::SRational,
            [signed(-2, 3), signed(i32::MIN, i32::MAX), signed(7, 0)],
            |order, v| srational_bytes(order, &[(v.numerator, v.denominator)]),
        );
    }

    #[test]
    fn a_rule_reads_only_a_type_it_names() {
        // One tag of each type a rule can name, each one value long, and
        // DOUBLE, which none can; a rule of each type, of any count, finds
        // the tag of its own type and no other.
        let types = [
            (BYTE, FieldType::Byte),
            (UNDEFINED, FieldType::Undefined),
            (SHORT, FieldType::Short),
            (LONG, FieldType::Long),
            (IFD, FieldType::Ifd),
            (SSHORT, FieldType::SShort),
            (SLONG, FieldType::SLong),
            (RATIONAL, FieldType::Rational),
            (SRATIONAL, FieldType::SRational),
            (ASCII, FieldType::Ascii),
            (UTF8, FieldType::Utf8),
            (DOUBLE, FieldType::Double),
        ];
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            for (tag, (code, field_type)) in (1..).zip(types) {
                let len = field_type.size();
                let at = 1024 + 16 * u32::from(tag);
                let bytes = vec![1; usize::try_from(len).unwrap()];
                put(&mut builder, ifd, tag, code, 1, &bytes, at);
            }
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");

            for (tag, (_, stored)) in (1..).zip(types) {
                let mut found = |named: FieldType| -> bool {
                    let types = only(named);
                    match named {
                        FieldType::Byte | FieldType::Undefined => {
                            ifd.get(src, rule::<Vec<u8>>(tag, types)).unwrap().is_some()
                        }
                        FieldType::Short | FieldType::Long | FieldType::Ifd => ifd
                            .get(src, rule::<Vec<u32>>(tag, types))
                            .unwrap()
                            .is_some(),
                        FieldType::SShort => ifd
                            .get(src, rule::<Vec<i16>>(tag, types))
                            .unwrap()
                            .is_some(),
                        FieldType::SLong => ifd
                            .get(src, rule::<Vec<i32>>(tag, types))
                            .unwrap()
                            .is_some(),
                        FieldType::Rational => ifd
                            .get(src, rule::<Vec<Rational>>(tag, types))
                            .unwrap()
                            .is_some(),
                        FieldType::SRational => ifd
                            .get(src, rule::<Vec<SignedRational>>(tag, types))
                            .unwrap()
                            .is_some(),
                        FieldType::Ascii | FieldType::Utf8 => {
                            ifd.get(src, rule::<Text>(tag, types)).unwrap().is_some()
                        }
                        _ => unreachable!("no rule names {named}"),
                    }
                };
                for (_, named) in types.iter().filter(|(_, t)| *t != FieldType::Double) {
                    assert_eq!(
                        found(*named),
                        *named == stored,
                        "{order:?}: {named} rule, {stored} tag"
                    );
                }
            }
        }
    }

    #[test]
    fn text_is_the_bytes_before_the_first_nul_as_stored() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder.blob_at(512, &[b'a', 0xFF, 0xFE, b'b', 0, b'c']);
            builder.blob_at(520, b"  spaced  \0\0");
            builder.blob_at(540, b"Z \xC3\xA9t\0");
            builder
                .raw(ifd, 0x0001, ASCII, 6, RawValue::Offset(512))
                .raw(ifd, 0x0002, ASCII, 12, RawValue::Offset(520))
                .raw(ifd, 0x0003, ASCII, 4, RawValue::Inline([b'x', 0, b'y', 0]))
                .raw(ifd, 0x0004, ASCII, 4, RawValue::Inline(*b"wxyz"))
                .raw(ifd, 0x0005, ASCII, 0, RawValue::Inline(*b"wxyz"))
                .raw(ifd, 0x0006, UTF8, 6, RawValue::Offset(540))
                .raw(ifd, 0x0007, UNDEFINED, 4, RawValue::Inline(*b"wxyz"));
            let built = builder.build();
            let (mut src, ifd, _) = read(order, &built, ifd);
            let src = &mut src;
            let ascii: &[FieldType] = &[FieldType::Ascii];
            let either: &[FieldType] = &[FieldType::Ascii, FieldType::Utf8];
            let text =
                |src: &mut Source<_>, tag, types| ifd.get(src, rule::<Text>(tag, types)).unwrap();

            assert_eq!(
                text(src, 0x0001, ascii).as_deref(),
                Some(&[b'a', 0xFF, 0xFE, b'b'][..])
            );
            assert_eq!(
                text(src, 0x0002, ascii).as_deref(),
                Some(&b"  spaced  "[..])
            );
            assert_eq!(text(src, 0x0003, ascii).as_deref(), Some(&b"x"[..]));
            // No NUL: the whole value.
            assert_eq!(text(src, 0x0004, ascii).as_deref(), Some(&b"wxyz"[..]));
            // No values: an empty text, whatever the entry's field holds.
            assert_eq!(text(src, 0x0005, ascii).as_deref(), Some(&b""[..]));
            // UTF-8 only where the rule names it; UNDEFINED never.
            assert_eq!(text(src, 0x0006, ascii), None, "{order:?}");
            assert_eq!(
                text(src, 0x0006, either).as_deref(),
                Some(&b"Z \xC3\xA9t"[..])
            );
            assert_eq!(text(src, 0x0007, either), None, "{order:?}");
        }
    }

    #[test]
    fn first_reads_the_first_value_alone_whatever_the_count_and_the_cap() {
        for order in ORDERS {
            // The fixture of the test of `u32` above, with fractions: under a
            // cap of 16 bytes, each value is far over it read whole.
            let longs: Vec<u8> = (1..=1000_u32).flat_map(|v| order.u32(v)).collect();
            let shorts: Vec<u8> = (11..=1010_u16).flat_map(|v| order.u16(v)).collect();
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .raw(ifd, 0x0001, LONG, 1000, RawValue::Offset(1024))
                .raw(ifd, 0x0002, SHORT, 1000, RawValue::Offset(8192))
                .raw(ifd, 0x0003, RATIONAL, 500, RawValue::Offset(1024))
                .raw(ifd, 0x0004, SRATIONAL, 500, RawValue::Offset(1024));
            builder.blob_at(1024, &longs);
            builder.blob_at(8192, &shorts);
            let built = builder.build();
            let limits = Limits::default().with_max_value_bytes(16);
            let (mut src, ifd, _) = read_with(order, &built, ifd, &limits);
            let src = &mut src;
            assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
            let numbers: &[FieldType] = &[FieldType::Short, FieldType::Long];

            assert_eq!(
                ifd.get(src, rule::<First<u32>>(0x0001, numbers)).unwrap(),
                Some(1)
            );
            assert_eq!(
                ifd.get(src, rule::<First<u32>>(0x0002, numbers)).unwrap(),
                Some(11)
            );
            assert_eq!(
                ifd.get(src, rule::<First<Rational>>(0x0003, &[FieldType::Rational]))
                    .unwrap(),
                Some(Rational {
                    numerator: 1,
                    denominator: 2
                })
            );
            assert_eq!(
                ifd.get(
                    src,
                    rule::<First<SignedRational>>(0x0004, &[FieldType::SRational])
                )
                .unwrap(),
                Some(SignedRational {
                    numerator: 1,
                    denominator: 2
                })
            );
            // Read whole, each is over the cap: damage, no value.
            assert_eq!(
                ifd.get(src, rule::<Vec<u32>>(0x0001, numbers)).unwrap(),
                None
            );
            assert_eq!(
                ifd.get(src, rule::<Vec<Rational>>(0x0003, &[FieldType::Rational]))
                    .unwrap(),
                None
            );
        }
    }

    /// The file of the tests that the reader's failure is the only error of
    /// [`Ifd::get`], read through a [`FailsFar`]: tag 0x0001, three SHORTs
    /// from [`FAR`] on, where every read fails; tag 0x0002, the same with
    /// four SHORTs; tag 0x0003, two SHORTs at the start of the file, read
    /// well; and tag 0x0004, a cap's worth of BYTEs and one more.
    fn failing_far(order: Order) -> (Source<FailsFar>, Ifd) {
        let far = RawValue::Offset(u32::try_from(FAR).unwrap());
        let cap = u32::try_from(Limits::default().max_value_bytes()).unwrap();
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder
            .raw(ifd, 0x0001, SHORT, 3, far)
            .raw(ifd, 0x0002, SHORT, 4, far)
            .short(ifd, 0x0003, &[7, 8])
            .raw(ifd, 0x0004, BYTE, cap + 1, RawValue::Offset(16));
        builder.pad_to(FAR + u64::from(cap) + 16);
        let built = builder.build();
        let mut src = Source::new(FailsFar(Cursor::new(built.bytes.clone()))).unwrap();
        let block = Block::file(src.len());
        let at = built.ifd_offset(ifd);
        let (ifd, _) =
            parse_ifd(&mut src, &block, at, byte_order(order), &Limits::default()).unwrap();
        assert_eq!(ifd.invalid_entries(), 0, "{order:?}");
        (src, ifd)
    }

    #[test]
    fn get_fails_only_when_the_reader_fails_reading_a_value_of_the_rules_shape() {
        let short: &[FieldType] = &[FieldType::Short];
        for order in ORDERS {
            let (mut src, ifd) = failing_far(order);
            let src = &mut src;
            // The rule's count, from FAR on: the reader fails.
            let err = ifd
                .get(src, rule::<[u32; 3]>(0x0001, short))
                .expect_err("the reader fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{order:?} {err:?}");
            // Another count, from FAR on: nothing is read, so nothing fails.
            assert_eq!(
                ifd.get(src, rule::<[u32; 3]>(0x0002, short)).unwrap(),
                None,
                "{order:?}"
            );
            // Another type, from FAR on: the same.
            assert_eq!(
                ifd.get(src, rule::<Vec<u32>>(0x0001, &[FieldType::Long]))
                    .unwrap(),
                None,
                "{order:?}"
            );
            // Over the cap: damage, never read.
            assert_eq!(
                ifd.get(src, rule::<Vec<u8>>(0x0004, &[FieldType::Byte]))
                    .unwrap(),
                None,
                "{order:?}"
            );
            assert_eq!(
                ifd.get(src, rule::<[u32; 2]>(0x0003, short)).unwrap(),
                Some([7, 8])
            );
        }
    }

    #[test]
    fn a_rules_value_past_the_end_of_the_file_is_damage() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .long(ifd, 0x0001, &[1, 3])
                .rational(ifd, 0x0002, &[(1, 3)])
                .srational(ifd, 0x0003, &[(-1, 3)]);
            let built = builder.build();
            let (_, parsed, _) = read(order, &built, ifd);
            let value_at = |tag| parsed.entry(tag).unwrap().value_offset().unwrap();
            let first = [0x0001, 0x0002, 0x0003].map(value_at).into_iter().min();
            let cut = usize::try_from(first.unwrap()).unwrap();

            // Read through a shorter file than the IFD was read from: the raw
            // accessor fails, and a rule gives no value.
            let mut src = open(&built.truncated(cut));
            let err = parsed.u32s(&mut src, 0x0001).expect_err("past the end");
            assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
            let longs = parsed.get(&mut src, rule::<[u32; 2]>(0x0001, &[FieldType::Long]));
            assert_eq!(longs.unwrap(), None, "{order:?}");
            let fractions = parsed.get(
                &mut src,
                rule::<[Rational; 1]>(0x0002, &[FieldType::Rational]),
            );
            assert_eq!(fractions.unwrap(), None, "{order:?}");
            let signed = parsed.get(
                &mut src,
                rule::<First<SignedRational>>(0x0003, &[FieldType::SRational]),
            );
            assert_eq!(signed.unwrap(), None, "{order:?}");
        }
    }

    #[test]
    fn held_reads_what_the_entry_holds_of_the_rules_shape_and_nothing_else() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd = builder.ifd();
            builder
                .long(ifd, 0x0001, &[0x0102_0304])
                .short(ifd, 0x0002, &[0x0506])
                .raw(
                    ifd,
                    0x0003,
                    IFD,
                    1,
                    RawValue::Inline(order.u32(0x0708_090A)),
                )
                .short(ifd, 0x0004, &[1, 2])
                .long(ifd, 0x0005, &[1, 2])
                .raw(ifd, 0x0006, LONG, 0, RawValue::Inline([1, 2, 3, 4]))
                .byte(ifd, 0x0007, &[9, 8, 7, 6])
                .undefined(ifd, 0x0008, &[9, 8, 7, 6])
                .byte(ifd, 0x0009, &[9, 8, 7])
                .raw(ifd, 0x000A, SSHORT, 2, RawValue::Inline([0xFF; 4]));
            let built = builder.build();
            // Read from the file, then held with no source at all.
            let (_, ifd, _) = read(order, &built, ifd);
            let any: &[FieldType] = &[FieldType::Short, FieldType::Long, FieldType::Ifd];
            let one = |tag, types| ifd.held(held_rule::<[u32; 1]>(tag, types));

            assert_eq!(one(0x0001, &[FieldType::Long]), Some([0x0102_0304]));
            assert_eq!(one(0x0001, any), Some([0x0102_0304]));
            assert_eq!(one(0x0002, &[FieldType::Short]), Some([0x0506]));
            assert_eq!(one(0x0003, &[FieldType::Ifd]), Some([0x0708_090A]));
            // A type the rule does not name.
            assert_eq!(one(0x0002, &[FieldType::Long]), None, "{order:?}");
            assert_eq!(one(0x0001, &[FieldType::Short]), None, "{order:?}");
            assert_eq!(one(0x0003, &[FieldType::Long]), None, "{order:?}");
            // Two values, or none, where the rule takes one: not even the
            // first is taken.
            for tag in [0x0004, 0x0005, 0x0006] {
                assert_eq!(one(tag, any), None, "{order:?} {tag:#06X}");
            }
            // Two SHORTs, where the rule takes two.
            let two = ifd.held(held_rule::<[u32; 2]>(0x0004, &[FieldType::Short]));
            assert_eq!(two, Some([1, 2]), "{order:?}");
            // Four bytes of BYTE, of UNDEFINED, and three where four are
            // wanted.
            let bytes = |tag, types| ifd.held(held_rule::<[u8; 4]>(tag, types));
            assert_eq!(bytes(0x0007, &[FieldType::Byte]), Some([9, 8, 7, 6]));
            assert_eq!(bytes(0x0008, &[FieldType::Undefined]), Some([9, 8, 7, 6]));
            assert_eq!(bytes(0x0008, &[FieldType::Byte]), None, "{order:?}");
            assert_eq!(bytes(0x0009, &[FieldType::Byte]), None, "{order:?}");
            // Two SSHORTs, read signed.
            let signed = ifd.held(held_rule::<[i16; 2]>(0x000A, &[FieldType::SShort]));
            assert_eq!(signed, Some([-1, -1]), "{order:?}");
            // A missing tag.
            assert_eq!(one(0x0099, any), None, "{order:?}");
        }
    }
}
