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
//! [`parse_ifd`] reads an IFD and checks each entry against the file once,
//! while reading it. An entry whose type is unknown, whose length overflows,
//! or whose value does not lie inside the file is counted and skipped, not an
//! error for the IFD: real files carry such entries and still hold good
//! previews. Every entry that is kept can be read. Values are read only when
//! asked for, through the same [`Source`], and no accessor reads more than
//! [`MAX_VALUE_LEN`] bytes for one value.
//!
//! The offsets written inside an IFD are relative to a base: the start of
//! the file for the file's own IFDs, an embedded TIFF header for the IFDs of
//! some MakerNotes. Every offset this module returns is absolute, except the
//! next-IFD pointer, which is returned as written.

use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::io::{ByteOrder, Source};
use crate::limits::Limits;

/// The longest value, in bytes, that one accessor of [`Ifd`] reads.
///
/// A cap on what one tag can make raw-files allocate, independent of
/// [`Limits`]: the tags read through these accessors are numbers and short
/// texts, far below it. A longer value is a payload, such as a preview,
/// which has its own path and its own limit.
const MAX_VALUE_LEN: u32 = 1024 * 1024;

/// Field type BYTE: 8-bit unsigned integers.
const BYTE: u16 = 1;
/// Field type ASCII: text, ending in a NUL.
const ASCII: u16 = 2;
/// Field type SHORT: 16-bit unsigned integers.
const SHORT: u16 = 3;
/// Field type LONG: 32-bit unsigned integers.
const LONG: u16 = 4;
/// Field type RATIONAL: pairs of LONGs, numerator first.
const RATIONAL: u16 = 5;
/// Field type SBYTE: 8-bit signed integers.
const SBYTE: u16 = 6;
/// Field type UNDEFINED: bytes that the tag gives a meaning to.
const UNDEFINED: u16 = 7;
/// Field type SSHORT: 16-bit signed integers.
const SSHORT: u16 = 8;
/// Field type SLONG: 32-bit signed integers.
const SLONG: u16 = 9;
/// Field type SRATIONAL: pairs of SLONGs, numerator first.
const SRATIONAL: u16 = 10;
/// Field type FLOAT: 32-bit IEEE floating point.
const FLOAT: u16 = 11;
/// Field type DOUBLE: 64-bit IEEE floating point.
const DOUBLE: u16 = 12;
/// Field type IFD: the offsets of IFDs, as LONGs (TIFF Technical Note 1).
const IFD: u16 = 13;

/// The size in bytes of one value of field type `typ`, or `None` for a type
/// that neither TIFF 6.0 (types 1 to 12) nor TIFF Technical Note 1 (type
/// 13) defines.
fn type_size(typ: u16) -> Option<u32> {
    match typ {
        BYTE | ASCII | SBYTE | UNDEFINED => Some(1),
        SHORT | SSHORT => Some(2),
        LONG | SLONG | FLOAT | IFD => Some(4),
        RATIONAL | SRATIONAL | DOUBLE => Some(8),
        _ => None,
    }
}

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
    base: u64,
    order: ByteOrder,
    entries: Vec<Entry>,
    invalid_entries: u32,
}

/// An entry of an [`Ifd`]: a field type that is known, and a value that lies
/// inside the file.
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
    /// At this absolute offset, with its `len` bytes inside the file.
    At(u64),
}

/// Reads the IFD at byte `offset` of `src`, in byte order `order`.
///
/// `offset` is absolute. `base` is added to every offset written inside the
/// IFD, which here means the offsets of values longer than four bytes: 0 for
/// the file's own IFDs, the position of an embedded TIFF header for the IFDs
/// of a MakerNote that has one.
///
/// Returns the IFD and its next-IFD pointer **as written**: relative to
/// `base`, and not checked against anything. 0 means there is no next IFD;
/// whoever follows the pointer adds `base` and decides whether it can be
/// read.
///
/// The IFD itself must be whole: its count and entries must lie inside the
/// file. A next-IFD pointer cut off by the end of the file counts as 0, as
/// other readers treat it. An entry that cannot be read is skipped and
/// counted in [`Ifd::invalid_entries`]: one whose field type is unknown,
/// whose length (count times type size) overflows 32 bits, or whose value is
/// out of line and overflows or does not lie inside the file.
///
/// # Errors
///
/// `Malformed` if the IFD has no entries, at `offset`, or its count or
/// entries do not lie inside the file (at `offset` too); `LimitExceeded` if it has more entries than
/// [`Limits::max_ifd_entries`] allows, at `offset`; `Io` if the reader
/// fails.
pub(crate) fn parse_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    base: u64,
    offset: u64,
    order: ByteOrder,
    limits: &Limits,
) -> Result<(Ifd, u32)> {
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
    let past_the_end = || Error::malformed("an IFD runs past the end of the file", offset);
    let table_len = 12 * u64::from(count);
    let table_at = offset
        .checked_add(2)
        .filter(|at| {
            at.checked_add(table_len)
                .is_some_and(|end| end <= src.len())
        })
        .ok_or_else(past_the_end)?;
    let table = src.bytes(table_at, table_len)?;
    // The table is exactly 12 × count bytes, so there is no remainder.
    let (raw_entries, _) = table.as_chunks::<12>();

    // The next-IFD pointer follows the entries. One cut off by the end of the
    // file is read as 0, no next IFD, as other readers do.
    let next = match table_at.checked_add(table_len).filter(|pointer_at| {
        pointer_at
            .checked_add(4)
            .is_some_and(|end| end <= src.len())
    }) {
        Some(pointer_at) => src.u32_at(pointer_at, order)?,
        None => 0,
    };

    let mut entries = Vec::new();
    let mut invalid_entries: u32 = 0;
    for raw in raw_entries {
        match parse_entry(raw, base, order, src.len()) {
            Some(entry) => entries.push(entry),
            // At most 65,535 entries: this cannot overflow.
            None => invalid_entries += 1,
        }
    }

    let ifd = Ifd {
        offset,
        base,
        order,
        entries,
        invalid_entries,
    };
    Ok((ifd, next))
}

/// Reads one 12-byte entry: `None` if its field type is unknown, its length
/// overflows 32 bits, or its value is out of line and does not lie inside a
/// file of `file_len` bytes once `base` is added to its offset.
fn parse_entry(raw: &[u8; 12], base: u64, order: ByteOrder, file_len: u64) -> Option<Entry> {
    let [t0, t1, y0, y1, c0, c1, c2, c3, f0, f1, f2, f3] = *raw;
    let tag = order.u16([t0, t1]);
    let typ = order.u16([y0, y1]);
    let count = order.u32([c0, c1, c2, c3]);
    let field = [f0, f1, f2, f3];

    let len = count.checked_mul(type_size(typ)?)?;
    let value = if len <= 4 {
        Value::Inline(field)
    } else {
        let at = base.checked_add(u64::from(order.u32(field)))?;
        let end = at.checked_add(u64::from(len))?;
        if end > file_len {
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

// The walk, the preview finder and the NEF reader's claim read IFDs
// through some of these accessors; the others wait for the format modules.
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

    /// The base that the offsets written inside the IFD are relative to,
    /// as given to [`parse_ifd`].
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn base(&self) -> u64 {
        self.base
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
    /// length overflows or their value lies outside the file.
    ///
    /// 0 means every entry of the IFD has a known type and lies inside the
    /// file.
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
        let Some((_, mut bytes)) = self.value(src, tag, &[ASCII])? else {
            return Ok(None);
        };
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// The bytes of tag `tag`, when its type is BYTE or UNDEFINED; `None`
    /// when the tag is missing or has another type. ASCII is not included:
    /// [`ascii`](Self::ascii) reads text.
    ///
    /// # Errors
    ///
    /// As for [`u32s`](Self::u32s).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing reads this accessor yet")
    )]
    pub(crate) fn bytes<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self
            .value(src, tag, &[BYTE, UNDEFINED])?
            .map(|(_, bytes)| bytes))
    }

    /// The first entry with tag `tag` and its value's bytes, when its type is
    /// one of `types`.
    fn value<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        tag: u16,
        types: &[u16],
    ) -> Result<Option<(&Entry, Vec<u8>)>> {
        let Some(entry) = self.entry(tag).filter(|entry| types.contains(&entry.typ)) else {
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

// Nothing reads an entry's fields yet. Each `expect` fails the build once
// its accessor has a caller, so it is then removed rather than left behind.
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
    #[cfg_attr(not(test), expect(dead_code, reason = "no format reads IFDs yet"))]
    pub(crate) fn typ(&self) -> u16 {
        self.typ
    }

    /// The number of values, as written.
    #[cfg_attr(not(test), expect(dead_code, reason = "no format reads IFDs yet"))]
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    /// The absolute offset of the value, when it is longer than four bytes
    /// and so stored out of line; `None` when it is in the entry.
    ///
    /// The value's bytes, from this offset, lie inside the file.
    #[cfg_attr(not(test), expect(dead_code, reason = "no format reads IFDs yet"))]
    pub(crate) fn value_offset(&self) -> Option<u64> {
        match self.value {
            Value::Inline(_) => None,
            Value::At(at) => Some(at),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
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

    /// Reads the IFD at `offset` of `bytes`, with `base` and `limits`.
    fn try_read(
        order: Order,
        bytes: &[u8],
        base: u64,
        offset: u64,
        limits: &Limits,
    ) -> Result<(Ifd, u32)> {
        parse_ifd(&mut open(bytes), base, offset, byte_order(order), limits)
    }

    /// Reads `ifd` of `built`, which must succeed, and returns the source to
    /// read its values through.
    fn read(order: Order, built: &Built, ifd: IfdId) -> (Source<Cursor<Vec<u8>>>, Ifd, u32) {
        let mut src = open(&built.bytes);
        let (parsed, next) = parse_ifd(
            &mut src,
            0,
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

    #[test]
    fn type_sizes() {
        let sizes = [1, 1, 2, 4, 8, 1, 1, 2, 4, 8, 4, 8, 4];
        for (typ, size) in (1..=13).zip(sizes) {
            assert_eq!(type_size(typ), Some(size), "type {typ}");
        }
        for typ in [0, 14, 99, u16::MAX] {
            assert_eq!(type_size(typ), None, "type {typ}");
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

            // A RATIONAL has no typed accessor yet: its entry says where it is.
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
            assert_eq!(ifd.base(), 0);
            assert_eq!(read(order, &built, b).2, 0xDEAD_BEEF);
            assert_eq!(read(order, &built, c).2, 0);
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
            let (parsed, next) = parse_ifd(
                &mut src,
                base,
                offset,
                byte_order(order),
                &Limits::default(),
            )
            .unwrap();
            assert_eq!(parsed.offset(), offset);
            assert_eq!(parsed.base(), base);
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
}
