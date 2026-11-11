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
//!   bytes set with [`TiffBuilder::prefix`], if any, immediately before it;
//!   the 2-byte entry count, the 12-byte entries, the 4-byte offset of the
//!   next IFD (0 by default), and then the entry values longer than 4 bytes,
//!   each at an even offset, in the order of the entries;
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
//! An IFD with its prefix and its values is also a block that an entry can
//! hold as its value, as a Sony MakerNote is held: an IFD that no other IFD
//! points at, whose offsets count from the start of the file, like every
//! offset the builder writes. [`TiffBuilder::undefined_ifd`] adds the entry,
//! [`TiffBuilder::undefined_ifd_declaring`] one that declares a count of
//! its own, and [`Built::ifd_block`] says where the block is. The block can
//! run on past the IFD's values to the end of a blob, holding what lies
//! between, as a Nikon MakerNote of the D1's layout holds its PreviewIFD
//! and that IFD's JPEG: [`TiffBuilder::block_through`].
//!
//! This file uses only `std`, because it is compiled both into the
//! library's unit tests and, through `tests/support/`, into every
//! integration test that declares `mod support`. Its own tests are in
//! `tests/tiff_builder.rs`, so that they run once and not in each of those
//! test builds.

use std::ops::Range;

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
/// The TIFF field type SSHORT: 16-bit signed integers.
pub const SSHORT: u16 = 8;
/// The TIFF field type SLONG: 32-bit signed integers.
pub const SLONG: u16 = 9;
/// The TIFF field type SRATIONAL: pairs of signed 32-bit integers,
/// numerator first.
pub const SRATIONAL: u16 = 10;
/// The TIFF field type IFD: the offsets of IFDs, as LONGs (TIFF Technical
/// Note 1).
pub const IFD: u16 = 13;
/// The Exif field type UTF-8: text in UTF-8 ending in a NUL.
pub const UTF8: u16 = 129;

/// The SubIFDs tag: the offsets of child IFDs.
pub const SUB_IFDS: u16 = 0x014A;
/// The Exif IFD tag: the offset of the Exif IFD.
pub const EXIF_IFD: u16 = 0x8769;
/// The GPS IFD tag: the offset of the GPS IFD, in IFD0.
pub const GPS_IFD: u16 = 0x8825;
/// The Interoperability IFD tag: the offset of the Interoperability IFD,
/// in the Exif IFD.
pub const INTEROP_IFD: u16 = 0xA005;

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
    /// The offset of this IFD's block, as the field of an UNDEFINED value
    /// whose count is the block's length; see [`Built::ifd_block`].
    IfdBlock(IfdId),
    /// The offset of this IFD's block, as the field of an UNDEFINED value
    /// whose count is the entry's own; see
    /// [`TiffBuilder::undefined_ifd_declaring`].
    IfdBlockOffset(IfdId),
}

impl Value {
    /// The length of the value's bytes, or `None` for a raw value field.
    fn len(&self) -> Option<usize> {
        match self {
            Value::Bytes(bytes) => Some(bytes.len()),
            Value::Raw(_) => None,
            Value::Ifds(ids) => Some(4 * ids.len()),
            Value::BlobOffset(_)
            | Value::BlobLen(_)
            | Value::IfdBlock(_)
            | Value::IfdBlockOffset(_) => Some(4),
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
    /// The bytes laid out immediately before the IFD.
    prefix: Vec<u8>,
    entries: Vec<Entry>,
    sorted: bool,
    next: Next,
    /// The blob the IFD's block runs on to the end of, when it does not
    /// end with the IFD's own values; see [`TiffBuilder::block_through`].
    through: Option<BlobId>,
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
            prefix: Vec::new(),
            entries: Vec::new(),
            sorted: true,
            next: Next::None,
            through: None,
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

    /// Lays `bytes` out immediately before `ifd`, such as the header a
    /// MakerNote puts before its IFD; the IFD still starts at an even
    /// offset, so the bytes may start at an odd one.
    pub fn prefix(&mut self, ifd: IfdId, bytes: &[u8]) -> &mut Self {
        self.ifd_mut(ifd).prefix = bytes.to_vec();
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

    /// Adds a UTF-8 entry: `text` and a NUL, which the count includes.
    pub fn utf8(&mut self, ifd: IfdId, tag: u16, text: &str) -> &mut Self {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        self.entry(ifd, tag, UTF8, bytes.len(), Value::Bytes(bytes))
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

    /// Adds an SLONG entry.
    pub fn slong(&mut self, ifd: IfdId, tag: u16, values: &[i32]) -> &mut Self {
        let order = self.order;
        let bytes = values
            .iter()
            .flat_map(|&v| order.u32(v.cast_unsigned()))
            .collect();
        self.entry(ifd, tag, SLONG, values.len(), Value::Bytes(bytes))
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

    /// Adds a GPS IFD entry (0x8825), a LONG for the offset of `child`: in
    /// IFD0, the offset of its GPS IFD.
    pub fn gps(&mut self, ifd: IfdId, child: IfdId) -> &mut Self {
        self.ifd_pointer(ifd, GPS_IFD, child)
    }

    /// Adds an Interoperability IFD entry (0xA005), a LONG for the offset
    /// of `child`: in the Exif IFD, the offset of its Interoperability IFD.
    pub fn interop(&mut self, ifd: IfdId, child: IfdId) -> &mut Self {
        self.ifd_pointer(ifd, INTEROP_IFD, child)
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

    /// Adds an UNDEFINED entry whose value is the block of `block_ifd`:
    /// its prefix, the IFD with its next-IFD pointer, and the IFD's values
    /// stored out of line, the bytes [`Built::ifd_block`] gives. Its count
    /// is the block's length and its value field holds the block's offset,
    /// both resolved when the file is built, so entries added to
    /// `block_ifd` afterwards are in the block too. This is how a Sony
    /// MakerNote is written. `block_ifd` should be an IFD that nothing
    /// else points at.
    pub fn undefined_ifd(&mut self, ifd: IfdId, tag: u16, block_ifd: IfdId) -> &mut Self {
        // The count written here is a stand-in: `build` writes the block's.
        self.entry(ifd, tag, UNDEFINED, 0, Value::IfdBlock(block_ifd))
    }

    /// Adds an UNDEFINED entry that holds the block of `block_ifd` as
    /// [`undefined_ifd`](Self::undefined_ifd) does, but declares `count`
    /// bytes rather than the block's length: a MakerNote whose entry says
    /// it is shorter than its block, or longer. Its value field holds the
    /// block's offset, resolved when the file is built. The entry takes the
    /// same 12 bytes either way, so the file is laid out as with
    /// `undefined_ifd`.
    pub fn undefined_ifd_declaring(
        &mut self,
        ifd: IfdId,
        tag: u16,
        block_ifd: IfdId,
        count: u32,
    ) -> &mut Self {
        self.ifd_mut(ifd).entries.push(Entry {
            tag,
            typ: UNDEFINED,
            count,
            value: Value::IfdBlockOffset(block_ifd),
        });
        self
    }

    /// Makes the block of `block_ifd` run on to the end of `blob`, so that
    /// everything the builder lays out between them lies inside it too:
    /// the IFDs created after `block_ifd`, with their values, and the
    /// blobs [`blob`](Self::blob) placed before `blob`. This is how a Nikon
    /// MakerNote of the D1's headerless layout is written, a bare IFD whose
    /// block holds its PreviewIFD and that IFD's JPEG: `block_ifd` is the
    /// MakerNote IFD, the PreviewIFD the last IFD created, and `blob` the
    /// JPEG, the first blob. `blob` must end past the block's own bytes.
    pub fn block_through(&mut self, block_ifd: IfdId, blob: BlobId) -> &mut Self {
        self.ifd_mut(block_ifd).through = Some(blob);
        self
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
            put(&mut bytes, at - to_u64(ifd.prefix.len()), &ifd.prefix);
            let count = u16::try_from(ifd.entries.len()).expect("at most 65,535 entries in an IFD");
            put(&mut bytes, at, &order.u16(count));
            let mut pos = at + 2;
            for (entry, value_at) in ifd.written().into_iter().zip(values) {
                let count = match entry.value {
                    Value::IfdBlock(block) => {
                        let range = &layout.blocks[block.0];
                        to_u32(range.end - range.start)
                    }
                    _ => entry.count,
                };
                put(&mut bytes, pos, &order.u16(entry.tag));
                put(&mut bytes, pos + 2, &order.u16(entry.typ));
                put(&mut bytes, pos + 4, &order.u32(count));
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
            blocks: layout.blocks,
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
        let mut blocks = Vec::new();
        for ifd in &self.ifds {
            // The prefix ends where the IFD starts, at an even offset; it
            // starts at or after `pos`.
            let prefix = to_u64(ifd.prefix.len());
            pos = even(pos + prefix);
            ifds.push(pos);
            let start = pos - prefix;
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
            blocks.push(start..pos);
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

        for (ifd, block) in self.ifds.iter().zip(&mut blocks) {
            if let Some(blob) = ifd.through {
                let end = blobs[blob.0] + to_u64(self.blobs[blob.0].bytes.len());
                assert!(
                    end >= block.end,
                    "the blob ends at {end}, inside the block it should close"
                );
                block.end = end;
            }
        }

        Layout {
            ifds,
            values,
            blobs,
            blocks,
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
            Value::IfdBlock(ifd) | Value::IfdBlockOffset(ifd) => {
                order.u32(to_u32(layout.blocks[ifd.0].start)).to_vec()
            }
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
    /// For each IFD, the bytes from the start of its prefix to the end of
    /// its last out-of-line value, or of its next-IFD pointer, or of the
    /// blob its block runs through.
    blocks: Vec<Range<u64>>,
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
    blocks: Vec<Range<u64>>,
}

impl Built {
    /// The offset of `ifd` in the file.
    pub fn ifd_offset(&self, ifd: IfdId) -> u64 {
        self.ifds[ifd.0]
    }

    /// Where the block of `ifd` is in the file: from the start of its
    /// prefix, or of the IFD when it has none, to the end of its last value
    /// stored out of line, or of its next-IFD pointer when it has none, or
    /// to the end of the blob set with [`TiffBuilder::block_through`]. The
    /// value of an entry added with [`TiffBuilder::undefined_ifd`].
    pub fn ifd_block(&self, ifd: IfdId) -> Range<u64> {
        self.blocks[ifd.0].clone()
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
