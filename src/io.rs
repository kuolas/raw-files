// SPDX-License-Identifier: MIT OR Apache-2.0

//! The bounds-checked reader: the one place raw-files reads bytes from a file.
//!
//! Every byte the crate reads passes through [`Source`], and every read takes
//! the same checked path: `offset + len` is computed with `checked_add` and
//! compared with the file's length, taken once when the source is created. A
//! read that overflows or runs past the end is a `Malformed` error at its
//! offset, found before the reader is touched, never a panic or a short read
//! (ADR008). A file that turns out shorter than that length, because it
//! shrank after it was opened, is an `Io` error at the offset of the read.
//!
//! Small reads go through a block cache. Walking a file's TIFF directories
//! reads a few bytes here and a few there, often close together; the cache
//! keeps the last few 64 KiB blocks, so these reads cost one underlying read
//! per block instead of one per value. Wrapping the reader in a `BufReader`
//! would not do this: it discards its buffer on every seek (ADR009). Payload
//! reads, such as a preview's bytes, bypass the cache: they go straight from
//! the reader into the caller's buffer, and leave the cached blocks as they
//! were.

use std::io::{self, Read, Seek, SeekFrom};

use crate::error::{Error, Result};

/// The size of one cached block, and the alignment of every block's start.
const BLOCK_SIZE: u64 = 64 * 1024;

/// How many blocks the cache holds.
const BLOCKS: usize = 4;

/// The longest read that [`Source::bytes`] serves from the block cache.
///
/// Tag values and headers are a few bytes to a few KiB, read many times from
/// the same neighbourhood, so they are worth caching. A longer read is a
/// payload, read once: caching it would only evict the directories around
/// it. Being far below `BLOCK_SIZE`, a cached read spans at most two blocks.
const SMALL_READ: u64 = 4096;

/// The order of the bytes in a file's multi-byte values, set by its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ByteOrder {
    /// Least significant byte first (TIFF `II`).
    Little,
    /// Most significant byte first (TIFF `MM`).
    Big,
}

/// A `Read + Seek` source whose every read is checked against its length.
///
/// The length is taken once, when the source is created. A read whose range
/// does not lie inside the file fails before the reader is touched. An empty
/// read is allowed at any offset up to and including the length, and returns
/// no bytes: a zero-length value that ends at the end of the file is not
/// damage.
///
/// Every call seeks before it reads, so the reader's position after any call
/// is unspecified.
pub(crate) struct Source<R> {
    inner: R,
    len: u64,
    cache: BlockCache,
}

impl<R: Read + Seek> Source<R> {
    /// Wraps `inner`, taking the file's length from the end of the reader.
    ///
    /// # Errors
    ///
    /// `Io` if the reader cannot seek to its end.
    pub(crate) fn new(mut inner: R) -> Result<Self> {
        let len = inner.seek(SeekFrom::End(0))?;
        Ok(Self {
            inner,
            len,
            cache: BlockCache::new(),
        })
    }

    /// The file's length in bytes, as it was when the source was created.
    pub(crate) fn len(&self) -> u64 {
        self.len
    }

    /// Returns the reader.
    pub(crate) fn into_inner(self) -> R {
        self.inner
    }

    /// Reads the `len` bytes at `offset` into `buf`, straight from the reader.
    ///
    /// This is the read for payloads. It bypasses the block cache and leaves
    /// it as it was. `buf` is cleared, then grows to exactly `len` bytes, so
    /// the most this allocates is `len` bytes, and only once `len` has been
    /// checked against the file's length; capping `len` by `Limits` is the
    /// caller's job. On error `buf` is left empty.
    ///
    /// # Errors
    ///
    /// `Malformed` if the range runs past the end of the file;
    /// `LimitExceeded` if `len` bytes cannot be held in memory on this
    /// platform; `Io` if the reader fails, with `offset` when the file is
    /// shorter than its length.
    pub(crate) fn read_into(&mut self, offset: u64, len: u64, buf: &mut Vec<u8>) -> Result<()> {
        buf.clear();
        self.check(offset, len)?;
        let n = to_usize(len, offset)?;
        // `len` is bounded only by the file's length, which comes from the
        // reader: `reserve_exact` would panic past `isize::MAX` bytes and
        // abort when the allocation fails.
        buf.try_reserve_exact(n).map_err(|_| too_large(offset))?;
        buf.resize(n, 0);
        let result = read_exact_at(&mut self.inner, offset, buf, offset);
        if result.is_err() {
            buf.clear();
        }
        result
    }

    /// Returns the `len` bytes at `offset`.
    ///
    /// This is the read for tag values and headers. Up to [`SMALL_READ`]
    /// bytes are served from the block cache; a longer read goes straight to
    /// the reader, as [`read_into`](Self::read_into) does.
    ///
    /// # Errors
    ///
    /// As for [`read_into`](Self::read_into).
    pub(crate) fn bytes(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        if len > SMALL_READ {
            self.read_into(offset, len, &mut buf)?;
        } else {
            // At most `SMALL_READ` bytes, a bound set here and not by the file,
            // so this allocation need not be fallible as `read_into` is.
            buf.resize(to_usize(len, offset)?, 0);
            self.read_cached(offset, &mut buf)?;
        }
        Ok(buf)
    }

    /// Returns the byte at `offset`, through the block cache.
    ///
    /// # Errors
    ///
    /// As for [`read_into`](Self::read_into).
    pub(crate) fn u8_at(&mut self, offset: u64) -> Result<u8> {
        let mut byte = [0];
        self.read_cached(offset, &mut byte)?;
        let [byte] = byte;
        Ok(byte)
    }

    /// Returns the 16-bit value at `offset` in byte order `order`, through
    /// the block cache.
    ///
    /// # Errors
    ///
    /// As for [`read_into`](Self::read_into).
    pub(crate) fn u16_at(&mut self, offset: u64, order: ByteOrder) -> Result<u16> {
        let mut bytes = [0; 2];
        self.read_cached(offset, &mut bytes)?;
        Ok(match order {
            ByteOrder::Little => u16::from_le_bytes(bytes),
            ByteOrder::Big => u16::from_be_bytes(bytes),
        })
    }

    /// Returns the 32-bit value at `offset` in byte order `order`, through
    /// the block cache.
    ///
    /// # Errors
    ///
    /// As for [`read_into`](Self::read_into).
    pub(crate) fn u32_at(&mut self, offset: u64, order: ByteOrder) -> Result<u32> {
        let mut bytes = [0; 4];
        self.read_cached(offset, &mut bytes)?;
        Ok(match order {
            ByteOrder::Little => u32::from_le_bytes(bytes),
            ByteOrder::Big => u32::from_be_bytes(bytes),
        })
    }

    /// Checks that the `len` bytes at `offset` lie inside the file.
    fn check(&self, offset: u64, len: u64) -> Result<()> {
        match offset.checked_add(len) {
            Some(end) if end <= self.len => Ok(()),
            _ => Err(Error::malformed(
                "data runs past the end of the file",
                offset,
            )),
        }
    }

    /// Fills `out` with the bytes at `offset`, block by block, through the
    /// cache.
    ///
    /// A block that cannot be read whole makes the read fail, even when the
    /// bytes asked for were there: the file has changed since it was opened.
    fn read_cached(&mut self, offset: u64, out: &mut [u8]) -> Result<()> {
        let len = u64::try_from(out.len()).map_err(|_| too_large(offset))?;
        self.check(offset, len)?;
        let mut pos = offset;
        let mut rest = out;
        while !rest.is_empty() {
            let start = pos - pos % BLOCK_SIZE;
            let block = self.cache.get_or_load(start, || {
                load_block(&mut self.inner, self.len, start, offset)
            })?;
            let src = block
                .get(to_usize(pos - start, offset)?..)
                .unwrap_or_default();
            if src.is_empty() {
                // Cannot happen while the check above holds and blocks are
                // loaded whole; an error rather than an endless loop if it
                // ever does.
                return Err(Error::io_at(
                    io::Error::from(io::ErrorKind::UnexpectedEof),
                    offset,
                ));
            }
            let n = src.len().min(rest.len());
            let (head, tail) = rest.split_at_mut(n);
            head.copy_from_slice(&src[..n]);
            rest = tail;
            // Either `rest` is now empty, or this block is used up and the
            // read goes on at the start of the next one.
            pos = start.saturating_add(BLOCK_SIZE);
        }
        Ok(())
    }
}

/// The last few blocks read from a file.
///
/// Each block holds the `BLOCK_SIZE` bytes from a multiple of `BLOCK_SIZE`,
/// except the last block of a file, which holds what is there.
struct BlockCache {
    blocks: Vec<Block>,
    /// Counts lookups; a block's `last_used` is the count when it was last
    /// used.
    clock: u64,
}

struct Block {
    start: u64,
    data: Vec<u8>,
    last_used: u64,
}

impl BlockCache {
    fn new() -> Self {
        Self {
            blocks: Vec::with_capacity(BLOCKS),
            clock: 0,
        }
    }

    /// Returns the data of the block that starts at `start`, loading it with
    /// `load` when it is not cached.
    ///
    /// A loaded block takes a free slot or, when there is none, the slot of
    /// the least recently used block. When `load` fails, the cache is left as
    /// it was.
    fn get_or_load(&mut self, start: u64, load: impl FnOnce() -> Result<Vec<u8>>) -> Result<&[u8]> {
        // Saturates rather than wraps: 2^64 lookups do not happen, and a
        // stuck clock would make eviction less apt, never a read wrong.
        self.clock = self.clock.saturating_add(1);
        let slot = match self.blocks.iter().position(|block| block.start == start) {
            Some(hit) => hit,
            None => {
                let block = Block {
                    start,
                    data: load()?,
                    last_used: 0,
                };
                if self.blocks.len() < BLOCKS {
                    self.blocks.push(block);
                    self.blocks.len() - 1
                } else {
                    let lru = self
                        .blocks
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, block)| block.last_used)
                        .map_or(0, |(slot, _)| slot);
                    self.blocks[lru] = block;
                    lru
                }
            }
        };
        let block = &mut self.blocks[slot];
        block.last_used = self.clock;
        Ok(&block.data)
    }
}

/// Reads the block that starts at `start`: `BLOCK_SIZE` bytes, or what is
/// left of a file of `file_len` bytes, never more. Errors carry `at`, the
/// offset of the read the block is for.
fn load_block<R: Read + Seek>(
    inner: &mut R,
    file_len: u64,
    start: u64,
    at: u64,
) -> Result<Vec<u8>> {
    let len = file_len.saturating_sub(start).min(BLOCK_SIZE);
    // At most `BLOCK_SIZE` bytes, a bound set here and not by the file, so
    // this allocation need not be fallible as `read_into` is.
    let mut data = vec![0; to_usize(len, at)?];
    read_exact_at(inner, start, &mut data, at)?;
    Ok(data)
}

/// Seeks to `start` and fills `buf`.
///
/// A reader that ends early means the file shrank after its length was
/// taken: that is an `Io` error at `at`, the offset of the caller's read,
/// with the reader's error as its source. Other reader errors become `Io`
/// errors as they are.
fn read_exact_at<R: Read + Seek>(inner: &mut R, start: u64, buf: &mut [u8], at: u64) -> Result<()> {
    inner.seek(SeekFrom::Start(start))?;
    inner.read_exact(buf).map_err(|err| {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            Error::io_at(err, at)
        } else {
            Error::from(err)
        }
    })
}

/// Converts the length of a read at `offset` to `usize`.
fn to_usize(len: u64, offset: u64) -> Result<usize> {
    usize::try_from(len).map_err(|_| too_large(offset))
}

/// A read at `offset` too large to hold in memory on this platform.
fn too_large(offset: u64) -> Error {
    Error::limit("read is too large for this platform", offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use std::cell::Cell;
    use std::error::Error as _;
    use std::io::Cursor;
    use std::rc::Rc;

    /// Longer than the four cached blocks, with a short fifth block.
    const FILE_LEN: usize = 300_000;
    const FILE_LEN_U64: u64 = 300_000;
    const LAST_BLOCK: u64 = 4 * BLOCK_SIZE;

    fn data() -> Vec<u8> {
        (0..FILE_LEN)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect()
    }

    fn at(data: &[u8], offset: u64, len: usize) -> &[u8] {
        let start = usize::try_from(offset).unwrap();
        &data[start..start + len]
    }

    fn open(data: &[u8]) -> Source<Cursor<&[u8]>> {
        Source::new(Cursor::new(data)).unwrap()
    }

    /// A reader that counts its `read` calls and the bytes they return.
    struct Counting {
        inner: Cursor<Vec<u8>>,
        reads: Rc<Cell<usize>>,
        bytes: Rc<Cell<usize>>,
    }

    impl Read for Counting {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.inner.read(buf)?;
            self.reads.set(self.reads.get() + 1);
            self.bytes.set(self.bytes.get() + n);
            Ok(n)
        }
    }

    impl Seek for Counting {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.inner.seek(pos)
        }
    }

    struct Counters {
        reads: Rc<Cell<usize>>,
        bytes: Rc<Cell<usize>>,
    }

    impl Counters {
        fn reads(&self) -> usize {
            self.reads.get()
        }

        fn bytes(&self) -> usize {
            self.bytes.get()
        }
    }

    fn counting() -> (Source<Counting>, Counters) {
        let reads = Rc::new(Cell::new(0));
        let bytes = Rc::new(Cell::new(0));
        let reader = Counting {
            inner: Cursor::new(data()),
            reads: Rc::clone(&reads),
            bytes: Rc::clone(&bytes),
        };
        (Source::new(reader).unwrap(), Counters { reads, bytes })
    }

    /// A file that shrank after it was opened: seeking to its end gives its
    /// full length, but only the first `present` bytes can be read.
    struct Shrunk {
        inner: Cursor<Vec<u8>>,
        present: Rc<Cell<u64>>,
    }

    impl Read for Shrunk {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let left = self.present.get().saturating_sub(self.inner.position());
            let n = buf.len().min(usize::try_from(left).unwrap());
            self.inner.read(&mut buf[..n])
        }
    }

    impl Seek for Shrunk {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.inner.seek(pos)
        }
    }

    /// A reader whose every call fails with `kind`, except the seeks
    /// `Source::new` needs when `seeks` is true.
    struct Failing {
        kind: io::ErrorKind,
        seeks: bool,
    }

    impl Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from(self.kind))
        }
    }

    impl Seek for Failing {
        fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
            if self.seeks {
                Ok(100)
            } else {
                Err(io::Error::from(self.kind))
            }
        }
    }

    /// A reader that claims to be `u64::MAX` bytes long and holds nothing.
    struct Endless;

    impl Read for Endless {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Ok(0)
        }
    }

    impl Seek for Endless {
        fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
            Ok(u64::MAX)
        }
    }

    fn assert_malformed_at(result: Result<impl std::fmt::Debug>, offset: u64) {
        let err = result.expect_err("the read is out of bounds");
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err:?}");
        assert_eq!(err.offset(), Some(offset), "{err:?}");
    }

    fn assert_shrank_at(result: Result<impl std::fmt::Debug>, offset: u64) {
        let err = result.expect_err("the file is shorter than it claims");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
        assert_eq!(err.offset(), Some(offset), "{err:?}");
        let source = err
            .source()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .expect("the source is the io::Error");
        assert_eq!(source.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn length_and_into_inner() {
        let data = data();
        let source = open(&data);
        assert_eq!(source.len(), FILE_LEN_U64);
        let reader = source.into_inner();
        assert_eq!(reader.get_ref().len(), FILE_LEN);

        let empty = Source::new(Cursor::new(Vec::new())).unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn reads_in_bounds() {
        let data = data();
        let mut source = open(&data);

        assert_eq!(source.bytes(1000, 16).unwrap(), at(&data, 1000, 16));
        let mut buf = Vec::new();
        source.read_into(70_000, 5000, &mut buf).unwrap();
        assert_eq!(buf, at(&data, 70_000, 5000));
        assert_eq!(source.u8_at(1234).unwrap(), data[1234]);
        assert_eq!(source.u8_at(0).unwrap(), data[0]);

        let two = [data[500], data[501]];
        assert_eq!(
            source.u16_at(500, ByteOrder::Little).unwrap(),
            u16::from_le_bytes(two)
        );
        assert_eq!(
            source.u16_at(500, ByteOrder::Big).unwrap(),
            u16::from_be_bytes(two)
        );
        let four = [data[900], data[901], data[902], data[903]];
        assert_eq!(
            source.u32_at(900, ByteOrder::Little).unwrap(),
            u32::from_le_bytes(four)
        );
        assert_eq!(
            source.u32_at(900, ByteOrder::Big).unwrap(),
            u32::from_be_bytes(four)
        );
        // The two orders differ on these bytes, so a swap would show.
        assert_ne!(u32::from_le_bytes(four), u32::from_be_bytes(four));
    }

    #[test]
    fn reads_past_the_end_are_malformed() {
        let data = data();
        let mut source = open(&data);
        let len = FILE_LEN_U64;
        let mut buf = Vec::new();

        // Starting at the end.
        assert_malformed_at(source.bytes(len, 1), len);
        assert_malformed_at(source.read_into(len, 1, &mut buf), len);
        assert_malformed_at(source.u8_at(len), len);
        assert_malformed_at(source.u16_at(len, ByteOrder::Little), len);
        assert_malformed_at(source.u32_at(len, ByteOrder::Big), len);

        // Ending one past the end.
        assert_malformed_at(source.bytes(len - 9, 10), len - 9);
        assert_malformed_at(source.read_into(len - 9, 10, &mut buf), len - 9);
        assert_malformed_at(
            source.bytes(len - SMALL_READ, SMALL_READ + 1),
            len - SMALL_READ,
        );
        assert_malformed_at(source.u16_at(len - 1, ByteOrder::Little), len - 1);
        assert_malformed_at(source.u32_at(len - 3, ByteOrder::Big), len - 3);

        // Starting past the end, even with nothing to read.
        assert_malformed_at(source.bytes(len + 1, 0), len + 1);
        assert_malformed_at(source.read_into(len + 1, 0, &mut buf), len + 1);
        assert_malformed_at(source.u8_at(len + 100), len + 100);

        // Nothing to read in an empty file.
        let mut empty = Source::new(Cursor::new(Vec::new())).unwrap();
        assert_malformed_at(empty.u8_at(0), 0);
    }

    #[test]
    fn empty_reads_are_allowed_up_to_the_end() {
        let data = data();
        let mut source = open(&data);
        assert_eq!(source.bytes(FILE_LEN_U64, 0).unwrap(), b"");
        assert_eq!(source.bytes(0, 0).unwrap(), b"");
        let mut buf = vec![1, 2, 3];
        source.read_into(FILE_LEN_U64, 0, &mut buf).unwrap();
        assert!(buf.is_empty());

        let mut empty = Source::new(Cursor::new(Vec::new())).unwrap();
        assert_eq!(empty.bytes(0, 0).unwrap(), b"");
    }

    #[test]
    fn offsets_near_the_top_of_u64_do_not_overflow() {
        let data = data();
        let mut source = open(&data);
        let mut buf = Vec::new();
        for (offset, len) in [
            (u64::MAX, 1),
            (u64::MAX - 1, 2),
            (1, u64::MAX),
            (u64::MAX, u64::MAX),
        ] {
            assert_malformed_at(source.bytes(offset, len), offset);
            assert_malformed_at(source.read_into(offset, len, &mut buf), offset);
        }
        assert_malformed_at(source.u8_at(u64::MAX), u64::MAX);
        assert_malformed_at(source.u16_at(u64::MAX, ByteOrder::Little), u64::MAX);
        assert_malformed_at(source.u32_at(u64::MAX - 2, ByteOrder::Big), u64::MAX - 2);
    }

    #[test]
    fn reads_across_block_boundaries() {
        let data = data();
        let mut source = open(&data);

        let offset = BLOCK_SIZE - 2;
        let four = at(&data, offset, 4).try_into().unwrap();
        assert_eq!(
            source.u32_at(offset, ByteOrder::Little).unwrap(),
            u32::from_le_bytes(four)
        );
        assert_eq!(
            source.u32_at(offset, ByteOrder::Big).unwrap(),
            u32::from_be_bytes(four)
        );
        let two = at(&data, 2 * BLOCK_SIZE - 1, 2).try_into().unwrap();
        assert_eq!(
            source.u16_at(2 * BLOCK_SIZE - 1, ByteOrder::Big).unwrap(),
            u16::from_be_bytes(two)
        );

        // A cached read across two blocks.
        let offset = 3 * BLOCK_SIZE - 100;
        assert_eq!(
            source.bytes(offset, SMALL_READ).unwrap(),
            at(&data, offset, 4096)
        );

        // A payload read across several blocks.
        let mut buf = Vec::new();
        source.read_into(1000, 200_000, &mut buf).unwrap();
        assert_eq!(buf, at(&data, 1000, 200_000));
    }

    #[test]
    fn reads_in_the_short_last_block() {
        let data = data();
        let mut source = open(&data);
        let len = FILE_LEN_U64;

        // Across into the last block, then within it, then ending at the end.
        let offset = LAST_BLOCK - 10;
        assert_eq!(source.bytes(offset, 20).unwrap(), at(&data, offset, 20));
        assert_eq!(source.u8_at(LAST_BLOCK + 5).unwrap(), data[262_149]);
        let four = at(&data, len - 4, 4).try_into().unwrap();
        assert_eq!(
            source.u32_at(len - 4, ByteOrder::Little).unwrap(),
            u32::from_le_bytes(four)
        );
        assert_eq!(source.u8_at(len - 1).unwrap(), data[FILE_LEN - 1]);
        assert_eq!(
            source.bytes(len - SMALL_READ, SMALL_READ).unwrap(),
            at(&data, len - SMALL_READ, 4096)
        );
        let mut buf = Vec::new();
        source.read_into(len - 50_000, 50_000, &mut buf).unwrap();
        assert_eq!(buf, at(&data, len - 50_000, 50_000));

        // A file shorter than one block.
        let short = &data[..1000];
        let mut source = open(short);
        assert_eq!(source.bytes(990, 10).unwrap(), &short[990..]);
        assert_malformed_at(source.bytes(990, 11), 990);
    }

    #[test]
    fn small_reads_in_one_block_cost_one_read() {
        let (mut source, counters) = counting();
        for offset in (0..60_000).step_by(997) {
            source.u8_at(offset).unwrap();
            source.u16_at(offset + 1, ByteOrder::Little).unwrap();
            source.u32_at(offset + 3, ByteOrder::Big).unwrap();
            source.bytes(offset + 7, 100).unwrap();
        }
        assert_eq!(counters.reads(), 1);
        assert_eq!(counters.bytes(), 65_536);
    }

    #[test]
    fn the_cache_holds_four_blocks_and_evicts_the_least_recently_used() {
        let (mut source, counters) = counting();
        let block = |n: u64| n * BLOCK_SIZE + 10;

        for n in 0..4 {
            source.u8_at(block(n)).unwrap();
        }
        assert_eq!(counters.reads(), 4, "four blocks, four loads");
        for n in 0..4 {
            source.u8_at(block(n)).unwrap();
        }
        assert_eq!(counters.reads(), 4, "all four stay cached");

        // Block 0 was loaded first but is now the most recently used, so a
        // fifth block evicts block 1.
        source.u8_at(block(0)).unwrap();
        source.u8_at(block(4)).unwrap();
        assert_eq!(counters.reads(), 5, "a fifth block is loaded");
        source.u8_at(block(0)).unwrap();
        assert_eq!(counters.reads(), 5, "the recently used block stays");
        source.u8_at(block(1)).unwrap();
        assert_eq!(counters.reads(), 6, "the evicted block is loaded again");
    }

    #[test]
    fn payload_reads_bypass_the_cache() {
        let (mut source, counters) = counting();
        let data = data();
        for n in 0..4 {
            source.u8_at(n * BLOCK_SIZE).unwrap();
        }
        let (reads, bytes) = (counters.reads(), counters.bytes());

        let mut buf = Vec::new();
        source.read_into(1000, 100 * 1024, &mut buf).unwrap();
        assert_eq!(buf, at(&data, 1000, 100 * 1024));
        assert_eq!(
            counters.bytes() - bytes,
            100 * 1024,
            "only the payload is read"
        );
        assert_eq!(counters.reads(), reads + 1, "in one read");
        let reads = counters.reads();

        for n in 0..4 {
            source.u8_at(n * BLOCK_SIZE + 1).unwrap();
        }
        assert_eq!(counters.reads(), reads, "the cached blocks still serve");
    }

    #[test]
    fn a_read_longer_than_small_read_bypasses_the_cache() {
        let (mut source, counters) = counting();
        let data = data();

        let bytes = source.bytes(10, SMALL_READ + 1).unwrap();
        assert_eq!(bytes, at(&data, 10, 4097));
        assert_eq!((counters.reads(), counters.bytes()), (1, 4097));
        source.u8_at(10).unwrap();
        assert_eq!(counters.reads(), 2, "the bytes were not cached");

        let (mut source, counters) = counting();
        let bytes = source.bytes(10, SMALL_READ).unwrap();
        assert_eq!(bytes, at(&data, 10, 4096));
        assert_eq!(counters.bytes(), 65_536, "a small read loads its block");
        source.u8_at(10).unwrap();
        assert_eq!(counters.reads(), 1, "and the block is cached");
    }

    #[test]
    fn a_file_that_shrank_is_an_io_error() {
        let present = Rc::new(Cell::new(1000));
        let mut source = Source::new(Shrunk {
            inner: Cursor::new(vec![7; 10_000]),
            present: Rc::clone(&present),
        })
        .unwrap();
        assert_eq!(source.len(), 10_000);

        let mut buf = vec![1, 2, 3];
        assert_shrank_at(source.read_into(1500, 10, &mut buf), 1500);
        assert!(buf.is_empty(), "a failed read leaves the buffer empty");
        assert_shrank_at(source.read_into(990, 20, &mut buf), 990);
        assert!(buf.is_empty());
        assert_shrank_at(source.u16_at(1500, ByteOrder::Little), 1500);
        assert_shrank_at(source.bytes(1990, 10), 1990);
        assert_shrank_at(source.bytes(1990, SMALL_READ + 1), 1990);
        // The block that holds byte 10 cannot be read whole either.
        assert_shrank_at(source.u8_at(10), 10);

        // A failed load leaves nothing behind in the cache: once the bytes
        // are back, the same reads succeed.
        present.set(10_000);
        assert_eq!(source.u8_at(10).unwrap(), 7);
        assert_eq!(source.u16_at(1500, ByteOrder::Little).unwrap(), 0x0707);
    }

    #[test]
    fn reader_errors_are_io_errors() {
        let err = Source::new(Failing {
            kind: io::ErrorKind::PermissionDenied,
            seeks: false,
        })
        .err()
        .expect("the seek fails");
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.offset(), None);

        let mut source = Source::new(Failing {
            kind: io::ErrorKind::PermissionDenied,
            seeks: true,
        })
        .unwrap();
        let mut buf = Vec::new();
        for err in [
            source.read_into(10, 5, &mut buf).unwrap_err(),
            source.u8_at(10).unwrap_err(),
        ] {
            assert_eq!(err.kind(), ErrorKind::Io);
            // Only a file that ended early carries an offset; any other
            // reader error passes through as it is.
            assert_eq!(err.offset(), None);
            let source = err
                .source()
                .and_then(|source| source.downcast_ref::<io::Error>())
                .expect("the source is the io::Error");
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
    }

    #[test]
    fn a_read_too_large_to_allocate_exceeds_a_limit() {
        // The reader's length passes the bounds check, but no `Vec` can hold
        // more than `isize::MAX` bytes: an error, not a "capacity overflow"
        // panic or an aborted allocation.
        let mut source = Source::new(Endless).unwrap();
        let len = u64::try_from(isize::MAX).unwrap() + 1;
        let mut buf = vec![1, 2, 3];
        let err = source
            .read_into(0, len, &mut buf)
            .expect_err("the read cannot be allocated");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err:?}");
        assert_eq!(err.offset(), Some(0), "{err:?}");
        assert!(buf.is_empty(), "a failed read leaves the buffer empty");
    }

    #[test]
    fn read_into_replaces_the_buffer() {
        let data = data();
        let mut source = open(&data);

        // `read_into` reserves exactly `len` bytes, but std does not promise
        // that the capacity then equals `len`, so only a lower bound is
        // asserted.
        let mut buf = Vec::new();
        source.read_into(100, 10_000, &mut buf).unwrap();
        assert_eq!(buf.len(), 10_000);
        assert_eq!(buf, at(&data, 100, 10_000));
        assert!(buf.capacity() >= 10_000);

        let mut buf = vec![0xAA; 5];
        buf.shrink_to_fit();
        source.read_into(200, 10, &mut buf).unwrap();
        assert_eq!(buf, at(&data, 200, 10));
        assert!(buf.capacity() >= 10);

        let mut buf = vec![0xAA; 50];
        source.read_into(300, 10, &mut buf).unwrap();
        assert_eq!(buf, at(&data, 300, 10));

        let mut buf = vec![0xAA; 50];
        assert_malformed_at(source.read_into(FILE_LEN_U64, 1, &mut buf), FILE_LEN_U64);
        assert!(buf.is_empty(), "a failed read leaves the buffer empty");
    }
}
