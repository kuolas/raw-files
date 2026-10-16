// SPDX-License-Identifier: MIT OR Apache-2.0

//! A builder for small TIFF files, for tests.
//!
//! Tests describe the file they need — its directories, their entries, the
//! data the entries point at — and the builder lays it out and writes the
//! bytes. It writes a valid file unless told otherwise, and every rule it
//! follows can be broken on purpose: a wrong byte-order mark or magic
//! number, an IFD0 offset anywhere, entries out of order, a next-IFD
//! pointer to anywhere, an entry with any type, count and value field.
//!
//! The layout, in order:
//!
//! - the 8-byte header: the byte-order mark, 42 and the offset of IFD0,
//!   which by default is the first IFD created;
//! - every IFD, in the order it was created, each at an even offset: the
//!   2-byte entry count, the 12-byte entries, the 4-byte offset of the next
//!   IFD (0 by default), and then the entry values longer than 4 bytes, each
//!   at an even offset, in the order of the entries;
//! - the blobs added with [`TiffBuilder::blob`], each at an even offset, in
//!   the order they were added;
//! - the blobs added with [`TiffBuilder::blob_at`], each at its own offset,
//!   with zeros in any gap before it;
//! - zeros up to the length set with [`TiffBuilder::pad_to`], if the file is
//!   shorter.
//!
//! Every multi-byte number is written in the file's byte order. A value of 4
//! bytes or fewer sits in the entry's value field, left-justified, and the
//! rest of the field is zero.
//!
//! This file uses only `std`, because it is compiled twice: into the
//! library's unit tests and, through `tests/support/`, into the integration
//! tests.

/// The byte order of the file being built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Least significant byte first, marked `II`.
    Little,
    /// Most significant byte first, marked `MM`.
    Big,
}

impl Order {
    /// The byte-order mark of a file in this order.
    pub fn mark(self) -> [u8; 2] {
        match self {
            Order::Little => *b"II",
            Order::Big => *b"MM",
        }
    }

    /// `value` in this order.
    pub fn u16(self, value: u16) -> [u8; 2] {
        match self {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        }
    }

    /// `value` in this order.
    pub fn u32(self, value: u32) -> [u8; 4] {
        match self {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        }
    }
}

/// The TIFF field type BYTE: 8-bit unsigned integers.
pub const BYTE: u16 = 1;
/// The TIFF field type ASCII: text ending in a NUL.
pub const ASCII: u16 = 2;
/// The TIFF field type SHORT: 16-bit unsigned integers.
pub const SHORT: u16 = 3;
/// The TIFF field type LONG: 32-bit unsigned integers.
pub const LONG: u16 = 4;
/// The TIFF field type RATIONAL: pairs of LONGs, numerator first.
pub const RATIONAL: u16 = 5;
/// The TIFF field type UNDEFINED: bytes the tag gives a meaning to.
pub const UNDEFINED: u16 = 7;
/// The TIFF field type SRATIONAL: pairs of signed 32-bit integers,
/// numerator first.
pub const SRATIONAL: u16 = 10;

/// The SubIFDs tag: the offsets of child IFDs.
pub const SUB_IFDS: u16 = 0x014A;
/// The Exif IFD tag: the offset of the Exif IFD.
pub const EXIF_IFD: u16 = 0x8769;

/// An IFD of one builder, as [`TiffBuilder::ifd`] returned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IfdId(usize);

/// A blob of one builder, as [`TiffBuilder::blob`] or
/// [`TiffBuilder::blob_at`] returned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobId(usize);

/// The value field of an entry added with [`TiffBuilder::raw`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawValue {
    /// These four bytes, as they are, whatever the byte order.
    Inline([u8; 4]),
    /// This offset, in the file's byte order.
    Offset(u32),
}

/// What an entry's value is, before the layout gives offsets to IFDs and
/// blobs.
#[derive(Clone, Debug)]
enum Value {
    /// The value's bytes, already in the file's order.
    Bytes(Vec<u8>),
    /// The value field, as given.
    Raw(RawValue),
    /// The offsets of these IFDs, as LONGs.
    Ifds(Vec<IfdId>),
    /// The offset of this blob, as a LONG.
    BlobOffset(BlobId),
    /// The length of this blob, as a LONG.
    BlobLen(BlobId),
}

impl Value {
    /// The length of the value's bytes, or `None` for a raw value field.
    fn len(&self) -> Option<usize> {
        match self {
            Value::Bytes(bytes) => Some(bytes.len()),
            Value::Raw(_) => None,
            Value::Ifds(ids) => Some(4 * ids.len()),
            Value::BlobOffset(_) | Value::BlobLen(_) => Some(4),
        }
    }

    /// Whether the value lives outside its entry.
    fn out_of_line(&self) -> bool {
        self.len().is_some_and(|len| len > 4)
    }
}

#[derive(Clone, Debug)]
struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    value: Value,
}

/// Where an IFD's next-IFD pointer points.
#[derive(Clone, Copy, Debug)]
enum Next {
    None,
    Ifd(IfdId),
    Raw(u32),
}

#[derive(Clone, Debug)]
struct Ifd {
    entries: Vec<Entry>,
    sorted: bool,
    next: Next,
}

impl Ifd {
    /// The entries in the order they are written.
    fn written(&self) -> Vec<&Entry> {
        let mut entries: Vec<&Entry> = self.entries.iter().collect();
        if self.sorted {
            // Stable, so entries with the same tag keep the order they were
            // added in.
            entries.sort_by_key(|entry| entry.tag);
        }
        entries
    }

    /// The bytes of the IFD itself, without its out-of-line values.
    fn len(&self) -> u64 {
        2 + 12 * to_u64(self.entries.len()) + 4
    }
}

#[derive(Clone, Debug)]
struct Blob {
    bytes: Vec<u8>,
    at: Option<u64>,
}

/// Builds a TIFF file; see the [module documentation](self) for the layout.
///
/// The methods that add to the file return `&mut Self`, so calls can be
/// chained. They panic on a value that no TIFF file can hold, such as more
/// than 65,535 entries in one IFD, and [`build`](Self::build) panics on a
/// layout that cannot be written, such as two blobs at the same offset: both
/// are mistakes in the test.
#[derive(Clone, Debug)]
pub struct TiffBuilder {
    order: Order,
    mark: Option<[u8; 2]>,
    magic: u16,
    ifd0_offset: Option<u32>,
    ifds: Vec<Ifd>,
    blobs: Vec<Blob>,
    min_len: u64,
}

impl TiffBuilder {
    /// An empty file in byte order `order`: a header, and IFD0 at offset 0
    /// until an IFD is created.
    pub fn new(order: Order) -> Self {
        Self {
            order,
            mark: None,
            magic: 42,
            ifd0_offset: None,
            ifds: Vec::new(),
            blobs: Vec::new(),
            min_len: 0,
        }
    }

    /// Writes `mark` as the byte-order mark, instead of the one for the
    /// file's order. Numbers are still written in the file's order.
    pub fn mark(&mut self, mark: [u8; 2]) -> &mut Self {
        self.mark = Some(mark);
        self
    }

    /// Writes `magic` in place of 42.
    pub fn magic(&mut self, magic: u16) -> &mut Self {
        self.magic = magic;
        self
    }

    /// Writes `offset` as the offset of IFD0, or, with `None`, the offset of
    /// the first IFD created (0 when there is none), which is the default.
    pub fn ifd0_offset(&mut self, offset: Option<u32>) -> &mut Self {
        self.ifd0_offset = offset;
        self
    }

    /// Creates an empty IFD, with its entries sorted by tag and no next IFD.
    ///
    /// The first IFD created is IFD0. The same call creates SubIFDs and the
    /// Exif IFD: what makes an IFD one of those is the entry that points to
    /// it.
    pub fn ifd(&mut self) -> IfdId {
        self.ifds.push(Ifd {
            entries: Vec::new(),
            sorted: true,
            next: Next::None,
        });
        IfdId(self.ifds.len() - 1)
    }

    /// Writes the entries of `ifd` in the order they were added, instead of
    /// sorted by tag.
    pub fn unsorted(&mut self, ifd: IfdId) -> &mut Self {
        self.ifd_mut(ifd).sorted = false;
        self
    }

    /// Points the next-IFD pointer of `ifd` at `next`, or, with `None`,
    /// writes 0 there. `next` may be `ifd` itself.
    pub fn next(&mut self, ifd: IfdId, next: Option<IfdId>) -> &mut Self {
        self.ifd_mut(ifd).next = next.map_or(Next::None, Next::Ifd);
        self
    }

    /// Writes `offset` as the next-IFD pointer of `ifd`.
    pub fn next_raw(&mut self, ifd: IfdId, offset: u32) -> &mut Self {
        self.ifd_mut(ifd).next = Next::Raw(offset);
        self
    }

    /// Adds a BYTE entry.
    pub fn byte(&mut self, ifd: IfdId, tag: u16, values: &[u8]) -> &mut Self {
        self.entry(ifd, tag, BYTE, values.len(), Value::Bytes(values.to_vec()))
    }

    /// Adds an ASCII entry: `text` and a NUL, which the count includes.
    pub fn ascii(&mut self, ifd: IfdId, tag: u16, text: &str) -> &mut Self {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        self.entry(ifd, tag, ASCII, bytes.len(), Value::Bytes(bytes))
    }

    /// Adds a SHORT entry.
    pub fn short(&mut self, ifd: IfdId, tag: u16, values: &[u16]) -> &mut Self {
        let order = self.order;
        let bytes = values.iter().flat_map(|&v| order.u16(v)).collect();
        self.entry(ifd, tag, SHORT, values.len(), Value::Bytes(bytes))
    }

    /// Adds a LONG entry.
    pub fn long(&mut self, ifd: IfdId, tag: u16, values: &[u32]) -> &mut Self {
        let order = self.order;
        let bytes = values.iter().flat_map(|&v| order.u32(v)).collect();
        self.entry(ifd, tag, LONG, values.len(), Value::Bytes(bytes))
    }

    /// Adds a RATIONAL entry of `(numerator, denominator)` pairs.
    pub fn rational(&mut self, ifd: IfdId, tag: u16, values: &[(u32, u32)]) -> &mut Self {
        let order = self.order;
        let bytes = values
            .iter()
            .flat_map(|&(num, den)| [order.u32(num), order.u32(den)])
            .flatten()
            .collect();
        self.entry(ifd, tag, RATIONAL, values.len(), Value::Bytes(bytes))
    }

    /// Adds an SRATIONAL entry of `(numerator, denominator)` pairs.
    pub fn srational(&mut self, ifd: IfdId, tag: u16, values: &[(i32, i32)]) -> &mut Self {
        let order = self.order;
        let bytes = values
            .iter()
            .flat_map(|&(num, den)| {
                [
                    order.u32(num.cast_unsigned()),
                    order.u32(den.cast_unsigned()),
                ]
            })
            .flatten()
            .collect();
        self.entry(ifd, tag, SRATIONAL, values.len(), Value::Bytes(bytes))
    }

    /// Adds an UNDEFINED entry.
    pub fn undefined(&mut self, ifd: IfdId, tag: u16, bytes: &[u8]) -> &mut Self {
        self.entry(
            ifd,
            tag,
            UNDEFINED,
            bytes.len(),
            Value::Bytes(bytes.to_vec()),
        )
    }

    /// Adds an entry with any type, count and value field, none of which
    /// need agree: for unknown types, and counts and offsets that are wrong
    /// on purpose.
    pub fn raw(
        &mut self,
        ifd: IfdId,
        tag: u16,
        typ: u16,
        count: u32,
        value: RawValue,
    ) -> &mut Self {
        self.ifd_mut(ifd).entries.push(Entry {
            tag,
            typ,
            count,
            value: Value::Raw(value),
        });
        self
    }

    /// Adds a SubIFDs entry (0x014A), a LONG for each child's offset.
    pub fn subifds(&mut self, ifd: IfdId, children: &[IfdId]) -> &mut Self {
        self.entry(
            ifd,
            SUB_IFDS,
            LONG,
            children.len(),
            Value::Ifds(children.to_vec()),
        )
    }

    /// Adds a SubIFDs entry (0x014A) with these offsets, whatever they point
    /// at.
    pub fn subifds_raw(&mut self, ifd: IfdId, offsets: &[u32]) -> &mut Self {
        self.long(ifd, SUB_IFDS, offsets)
    }

    /// Adds an entry under `tag` that points at `child`: a LONG holding
    /// the offset of `child`, as the Exif IFD tag does in IFD0, or the
    /// PreviewIFD tag in a Nikon MakerNote.
    pub fn ifd_pointer(&mut self, ifd: IfdId, tag: u16, child: IfdId) -> &mut Self {
        self.entry(ifd, tag, LONG, 1, Value::Ifds(vec![child]))
    }

    /// Adds an Exif IFD entry (0x8769), a LONG for the offset of `child`.
    pub fn exif(&mut self, ifd: IfdId, child: IfdId) -> &mut Self {
        self.ifd_pointer(ifd, EXIF_IFD, child)
    }

    /// Adds a blob, placed after the IFDs and after the blobs that this
    /// method added before it.
    pub fn blob(&mut self, bytes: &[u8]) -> BlobId {
        self.push_blob(bytes, None)
    }

    /// Adds a blob placed at `offset`, which must lie after the IFDs and
    /// the blobs placed by [`blob`](Self::blob), and must not overlap
    /// another blob.
    pub fn blob_at(&mut self, offset: u64, bytes: &[u8]) -> BlobId {
        self.push_blob(bytes, Some(offset))
    }

    /// Adds a LONG entry holding the offset of `blob`.
    pub fn long_blob(&mut self, ifd: IfdId, tag: u16, blob: BlobId) -> &mut Self {
        self.entry(ifd, tag, LONG, 1, Value::BlobOffset(blob))
    }

    /// Adds a LONG entry holding the length of `blob`.
    pub fn long_blob_len(&mut self, ifd: IfdId, tag: u16, blob: BlobId) -> &mut Self {
        self.entry(ifd, tag, LONG, 1, Value::BlobLen(blob))
    }

    /// Adds an UNDEFINED entry whose value is the bytes of `blob`: its count
    /// is the blob's length, and its value field holds the blob's offset, as
    /// the field of any value longer than four bytes does. This is how a
    /// MakerNote is written. The blob must be longer than four bytes, or
    /// its bytes would belong in the field itself.
    pub fn undefined_blob(&mut self, ifd: IfdId, tag: u16, blob: BlobId) -> &mut Self {
        let len = self.blobs[blob.0].bytes.len();
        assert!(len > 4, "a blob of {len} bytes would be an inline value");
        // The value field is the blob's offset in four bytes, exactly what
        // `long_blob` writes, so the two share `Value::BlobOffset` on
        // purpose: there the four bytes are the value, here they say where
        // the value is.
        self.entry(ifd, tag, UNDEFINED, len, Value::BlobOffset(blob))
    }

    /// Makes the file at least `len` bytes long, padding it with zeros.
    pub fn pad_to(&mut self, len: u64) -> &mut Self {
        self.min_len = len;
        self
    }

    /// Lays the file out and writes it.
    pub fn build(&self) -> Built {
        let layout = self.layout();
        let order = self.order;
        let mut bytes = Vec::new();

        let ifd0 = match (self.ifd0_offset, layout.ifds.first()) {
            (Some(offset), _) => offset,
            (None, Some(&offset)) => to_u32(offset),
            (None, None) => 0,
        };
        put(&mut bytes, 0, &self.mark.unwrap_or(order.mark()));
        put(&mut bytes, 2, &order.u16(self.magic));
        put(&mut bytes, 4, &order.u32(ifd0));

        for (ifd, (&at, values)) in self.ifds.iter().zip(layout.ifds.iter().zip(&layout.values)) {
            let count = u16::try_from(ifd.entries.len()).expect("at most 65,535 entries in an IFD");
            put(&mut bytes, at, &order.u16(count));
            let mut pos = at + 2;
            for (entry, value_at) in ifd.written().into_iter().zip(values) {
                put(&mut bytes, pos, &order.u16(entry.tag));
                put(&mut bytes, pos + 2, &order.u16(entry.typ));
                put(&mut bytes, pos + 4, &order.u32(entry.count));
                let field = match (&entry.value, value_at) {
                    (Value::Raw(RawValue::Inline(field)), _) => *field,
                    (Value::Raw(RawValue::Offset(offset)), _) => order.u32(*offset),
                    (value, Some(value_at)) => {
                        put(&mut bytes, *value_at, &self.resolve(value, &layout));
                        order.u32(to_u32(*value_at))
                    }
                    (value, None) => {
                        let mut field = [0; 4];
                        let data = self.resolve(value, &layout);
                        field[..data.len()].copy_from_slice(&data);
                        field
                    }
                };
                put(&mut bytes, pos + 8, &field);
                pos += 12;
            }
            let next = match ifd.next {
                Next::None => 0,
                Next::Ifd(next) => to_u32(layout.ifd(next)),
                Next::Raw(offset) => offset,
            };
            put(&mut bytes, pos, &order.u32(next));
        }

        for (blob, &at) in self.blobs.iter().zip(&layout.blobs) {
            put(&mut bytes, at, &blob.bytes);
        }
        let min_len = usize::try_from(self.min_len).expect("the file fits in memory");
        if bytes.len() < min_len {
            bytes.resize(min_len, 0);
        }

        Built {
            bytes,
            ifds: layout.ifds,
            blobs: layout.blobs,
        }
    }

    fn ifd_mut(&mut self, ifd: IfdId) -> &mut Ifd {
        &mut self.ifds[ifd.0]
    }

    fn entry(&mut self, ifd: IfdId, tag: u16, typ: u16, count: usize, value: Value) -> &mut Self {
        let count = u32::try_from(count).expect("a TIFF count fits in 32 bits");
        self.ifd_mut(ifd).entries.push(Entry {
            tag,
            typ,
            count,
            value,
        });
        self
    }

    fn push_blob(&mut self, bytes: &[u8], at: Option<u64>) -> BlobId {
        self.blobs.push(Blob {
            bytes: bytes.to_vec(),
            at,
        });
        BlobId(self.blobs.len() - 1)
    }

    /// Places every IFD, out-of-line value and blob.
    fn layout(&self) -> Layout {
        let mut pos = 8;
        let mut ifds = Vec::new();
        let mut values = Vec::new();
        for ifd in &self.ifds {
            pos = even(pos);
            ifds.push(pos);
            pos += ifd.len();
            let mut placed = Vec::new();
            for entry in ifd.written() {
                if entry.value.out_of_line() {
                    pos = even(pos);
                    placed.push(Some(pos));
                    pos += to_u64(entry.value.len().unwrap_or(0));
                } else {
                    placed.push(None);
                }
            }
            values.push(placed);
        }

        let mut blobs = vec![0; self.blobs.len()];
        for (blob, at) in self.blobs.iter().zip(&mut blobs) {
            if blob.at.is_none() {
                pos = even(pos);
                *at = pos;
                pos += to_u64(blob.bytes.len());
            }
        }
        let mut fixed: Vec<(u64, usize)> = self
            .blobs
            .iter()
            .enumerate()
            .filter_map(|(i, blob)| blob.at.map(|at| (at, i)))
            .collect();
        fixed.sort_unstable();
        for (at, i) in fixed {
            assert!(
                at >= pos,
                "the blob placed at {at} overlaps the bytes before {pos}"
            );
            blobs[i] = at;
            pos = at + to_u64(self.blobs[i].bytes.len());
        }

        Layout {
            ifds,
            values,
            blobs,
        }
    }

    /// The bytes of `value`, with the offsets and lengths it refers to.
    fn resolve(&self, value: &Value, layout: &Layout) -> Vec<u8> {
        let order = self.order;
        match value {
            Value::Bytes(bytes) => bytes.clone(),
            Value::Raw(_) => unreachable!("a raw value is its own field"),
            Value::Ifds(ids) => ids
                .iter()
                .flat_map(|&id| order.u32(to_u32(layout.ifd(id))))
                .collect(),
            Value::BlobOffset(blob) => order.u32(to_u32(layout.blobs[blob.0])).to_vec(),
            Value::BlobLen(blob) => order
                .u32(to_u32(to_u64(self.blobs[blob.0].bytes.len())))
                .to_vec(),
        }
    }
}

/// Where everything goes.
struct Layout {
    /// The offset of each IFD.
    ifds: Vec<u64>,
    /// For each IFD, in the order its entries are written, the offset of each
    /// out-of-line value.
    values: Vec<Vec<Option<u64>>>,
    /// The offset of each blob.
    blobs: Vec<u64>,
}

impl Layout {
    fn ifd(&self, id: IfdId) -> u64 {
        self.ifds[id.0]
    }
}

/// A file written by [`TiffBuilder::build`], and where things are in it.
#[derive(Clone, Debug)]
pub struct Built {
    /// The file.
    pub bytes: Vec<u8>,
    ifds: Vec<u64>,
    blobs: Vec<u64>,
}

impl Built {
    /// The offset of `ifd` in the file.
    pub fn ifd_offset(&self, ifd: IfdId) -> u64 {
        self.ifds[ifd.0]
    }

    /// The offset of `blob` in the file.
    pub fn blob_offset(&self, blob: BlobId) -> u64 {
        self.blobs[blob.0]
    }

    /// The first `len` bytes of the file, which must be at most its length.
    pub fn truncated(&self, len: usize) -> Vec<u8> {
        self.bytes[..len].to_vec()
    }
}

/// Writes `data` at `at`, growing `bytes` with zeros as needed.
fn put(bytes: &mut Vec<u8>, at: u64, data: &[u8]) {
    let at = usize::try_from(at).expect("the file fits in memory");
    let end = at + data.len();
    if bytes.len() < end {
        bytes.resize(end, 0);
    }
    bytes[at..end].copy_from_slice(data);
}

/// `pos`, rounded up to a word boundary.
fn even(pos: u64) -> u64 {
    pos + pos % 2
}

fn to_u64(n: usize) -> u64 {
    u64::try_from(n).expect("a length fits in 64 bits")
}

fn to_u32(n: u64) -> u32 {
    u32::try_from(n).expect("a TIFF offset or length fits in 32 bits")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One IFD, holding Make (ASCII, out of line) and Orientation (SHORT,
    /// inline).
    fn tiny(order: Order) -> Vec<u8> {
        let mut builder = TiffBuilder::new(order);
        let ifd = builder.ifd();
        builder.ascii(ifd, 0x010F, "Nikon").short(ifd, 0x0112, &[1]);
        builder.build().bytes
    }

    fn u16_le(bytes: &[u8], at: u64) -> u16 {
        let at = usize::try_from(at).unwrap();
        u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
    }

    fn u32_le(bytes: &[u8], at: u64) -> u32 {
        let at = usize::try_from(at).unwrap();
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    /// The tags of the IFD at `at` in a little-endian file, in file order.
    fn tags_le(bytes: &[u8], at: u64) -> Vec<u16> {
        (0..u64::from(u16_le(bytes, at)))
            .map(|i| u16_le(bytes, at + 2 + 12 * i))
            .collect()
    }

    #[test]
    fn a_tiny_little_endian_file() {
        #[rustfmt::skip]
        let expected = [
            b'I', b'I', 42, 0, 8, 0, 0, 0,                  // header
            2, 0,                                           // two entries
            0x0F, 0x01, 2, 0, 6, 0, 0, 0, 38, 0, 0, 0,      // Make, ASCII, 6, at 38
            0x12, 0x01, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0,       // Orientation, SHORT, 1, 1
            0, 0, 0, 0,                                     // no next IFD
            b'N', b'i', b'k', b'o', b'n', 0,                // Make's value
        ];
        assert_eq!(tiny(Order::Little), expected);
    }

    #[test]
    fn a_tiny_big_endian_file() {
        #[rustfmt::skip]
        let expected = [
            b'M', b'M', 0, 42, 0, 0, 0, 8,                  // header
            0, 2,                                           // two entries
            0x01, 0x0F, 0, 2, 0, 0, 0, 6, 0, 0, 0, 38,      // Make, ASCII, 6, at 38
            0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 1, 0, 0,       // Orientation, SHORT, 1, 1
            0, 0, 0, 0,                                     // no next IFD
            b'N', b'i', b'k', b'o', b'n', 0,                // Make's value
        ];
        assert_eq!(tiny(Order::Big), expected);
    }

    #[test]
    fn entries_are_sorted_unless_asked_not_to_be() {
        let mut builder = TiffBuilder::new(Order::Little);
        let sorted = builder.ifd();
        let unsorted = builder.ifd();
        builder.unsorted(unsorted);
        for ifd in [sorted, unsorted] {
            builder
                .short(ifd, 0x0112, &[1])
                .ascii(ifd, 0x010F, "a long make")
                .long(ifd, 0x0100, &[4000]);
        }
        let built = builder.build();
        assert_eq!(
            tags_le(&built.bytes, built.ifd_offset(sorted)),
            [0x0100, 0x010F, 0x0112]
        );
        assert_eq!(
            tags_le(&built.bytes, built.ifd_offset(unsorted)),
            [0x0112, 0x010F, 0x0100]
        );
    }

    #[test]
    fn out_of_line_values_and_ifds_are_word_aligned() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let ifd1 = builder.ifd();
        // Five bytes, then eight: the rational starts one byte later than
        // the end of the text.
        builder
            .ascii(ifd0, 0x010F, "Sony")
            .rational(ifd0, 0x011A, &[(300, 1)]);
        builder.next(ifd0, Some(ifd1));
        let built = builder.build();
        let bytes = &built.bytes;

        // IFD0 at 8 holds two entries, so its values start at 8 + 30.
        assert_eq!(u32_le(bytes, 8 + 2 + 8), 38, "the text");
        assert_eq!(&bytes[38..43], b"Sony\0");
        assert_eq!(u32_le(bytes, 8 + 14 + 8), 44, "the rational");
        assert_eq!((u32_le(bytes, 44), u32_le(bytes, 48)), (300, 1));
        assert_eq!(built.ifd_offset(ifd1), 52);
        assert_eq!(u32_le(bytes, 8 + 26), 52, "the next-IFD pointer");
    }

    #[test]
    fn blobs_are_where_the_offsets_say() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        let jpeg = builder.blob(&[0xFF, 0xD8, 0xFF, 0xD9, 0xAA]);
        let fixed = builder.blob_at(1000, b"far away");
        builder
            .long_blob(ifd, 0x0201, jpeg)
            .long_blob_len(ifd, 0x0202, jpeg)
            .long_blob(ifd, 0x0111, fixed);
        let built = builder.build();
        let bytes = &built.bytes;

        // One IFD of three entries at 8, then the first blob.
        let at = built.blob_offset(jpeg);
        assert_eq!(at, 8 + 2 + 36 + 4);
        let start = usize::try_from(at).unwrap();
        assert_eq!(&bytes[start..start + 5], [0xFF, 0xD8, 0xFF, 0xD9, 0xAA]);
        assert_eq!(built.blob_offset(fixed), 1000);
        assert_eq!(&bytes[1000..], b"far away");
        assert!(bytes[start + 5..1000].iter().all(|&b| b == 0));

        // Sorted: 0x0111, 0x0201, 0x0202.
        assert_eq!(u32_le(bytes, 8 + 2 + 8), 1000);
        assert_eq!(u64::from(u32_le(bytes, 8 + 14 + 8)), at);
        assert_eq!(u32_le(bytes, 8 + 26 + 8), 5);
    }

    #[test]
    fn ifd_pointers_resolve() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let (a, b, exif) = (builder.ifd(), builder.ifd(), builder.ifd());
        builder.subifds(ifd0, &[a, b]).exif(ifd0, exif);
        builder.next(a, Some(a)).next_raw(b, 0xDEAD_BEEF);
        let built = builder.build();
        let bytes = &built.bytes;

        // SubIFDs: two offsets, out of line.
        assert_eq!(u16_le(bytes, 8 + 2), SUB_IFDS);
        let array = u64::from(u32_le(bytes, 8 + 2 + 8));
        assert_eq!(u64::from(u32_le(bytes, array)), built.ifd_offset(a));
        assert_eq!(u64::from(u32_le(bytes, array + 4)), built.ifd_offset(b));
        // The Exif IFD: one offset, inline.
        assert_eq!(u16_le(bytes, 8 + 14), EXIF_IFD);
        assert_eq!(u64::from(u32_le(bytes, 8 + 14 + 8)), built.ifd_offset(exif));
        // Empty IFDs: the next pointer follows the count.
        let a_at = built.ifd_offset(a);
        assert_eq!(u64::from(u32_le(bytes, a_at + 2)), a_at, "points to itself");
        assert_eq!(u32_le(bytes, built.ifd_offset(b) + 2), 0xDEAD_BEEF);
        assert_eq!(u32_le(bytes, built.ifd_offset(exif) + 2), 0);
    }

    #[test]
    fn an_ifd_pointer_holds_the_childs_offset_under_any_tag() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let (a, child) = (builder.ifd(), builder.ifd());
        builder.ifd_pointer(ifd0, 0x0011, child);
        let built = builder.build();
        let bytes = &built.bytes;

        // One entry at 8 + 2: the tag, LONG, one value, inline.
        assert_eq!(u16_le(bytes, 8 + 2), 0x0011);
        assert_eq!(u16_le(bytes, 8 + 2 + 2), LONG);
        assert_eq!(u32_le(bytes, 8 + 2 + 4), 1);
        assert_eq!(u64::from(u32_le(bytes, 8 + 2 + 8)), built.ifd_offset(child));
        assert_ne!(built.ifd_offset(child), built.ifd_offset(a));
    }

    #[test]
    fn an_undefined_blob_counts_the_blobs_bytes_and_points_at_them() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        let first = builder.blob(b"another blob first");
        let value = b"Nikon\0, and more";
        let blob = builder.blob(value);
        builder.undefined_blob(ifd, 0x927C, blob);
        let built = builder.build();
        let bytes = &built.bytes;

        // One entry at 8 + 2: the tag, UNDEFINED, the blob's length, and
        // the blob's offset in the value field.
        assert_eq!(u16_le(bytes, 8 + 2), 0x927C);
        assert_eq!(u16_le(bytes, 8 + 2 + 2), UNDEFINED);
        assert_eq!(
            usize::try_from(u32_le(bytes, 8 + 2 + 4)).unwrap(),
            value.len()
        );
        let at = built.blob_offset(blob);
        assert_ne!(at, built.blob_offset(first));
        assert_eq!(u64::from(u32_le(bytes, 8 + 2 + 8)), at);
        let at = usize::try_from(at).unwrap();
        assert_eq!(&bytes[at..at + value.len()], value);
    }

    #[test]
    #[should_panic(expected = "inline value")]
    fn an_undefined_blob_of_four_bytes_is_a_mistake() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        let blob = builder.blob(b"four");
        builder.undefined_blob(ifd, 0x927C, blob);
    }

    #[test]
    fn raw_entries_are_written_as_given() {
        let mut builder = TiffBuilder::new(Order::Big);
        let ifd = builder.ifd();
        builder
            .unsorted(ifd)
            .raw(ifd, 0x9999, 99, 7, RawValue::Inline([1, 2, 3, 4]))
            .raw(ifd, 0x0001, LONG, u32::MAX, RawValue::Offset(0x0A0B_0C0D));
        let bytes = builder.build().bytes;
        assert_eq!(
            &bytes[10..34],
            [
                0x99, 0x99, 0, 99, 0, 0, 0, 7, 1, 2, 3, 4, //
                0, 1, 0, 4, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A, 0x0B, 0x0C, 0x0D,
            ]
        );
    }

    #[test]
    fn header_overrides_take_effect() {
        let mut builder = TiffBuilder::new(Order::Big);
        builder.ifd();
        builder
            .magic(43)
            .ifd0_offset(Some(0x0102_0304))
            .mark(*b"XY");
        assert_eq!(builder.build().bytes[..8], [b'X', b'Y', 0, 43, 1, 2, 3, 4]);

        builder.ifd0_offset(None);
        assert_eq!(builder.build().bytes[4..8], [0, 0, 0, 8]);
        assert_eq!(
            TiffBuilder::new(Order::Little).build().bytes,
            b"II*\0\0\0\0\0"
        );
    }

    #[test]
    fn pad_to_and_truncated() {
        let mut builder = TiffBuilder::new(Order::Little);
        builder.pad_to(100);
        let built = builder.build();
        assert_eq!(built.bytes.len(), 100);
        assert!(built.bytes[8..].iter().all(|&b| b == 0));
        assert_eq!(built.truncated(3), b"II*");
    }
}
