// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Sony MakerNote: where it lies in the file, how its directory is
//! found, and what is checked before anything in it is read.
//!
//! The MakerNote is a block of Sony's own metadata, the value of the
//! MakerNote tag of the Exif IFD: a directory of Sony's tags, an IFD like
//! the file's own, with the values it points at. raw-files reads it for one
//! thing only, the point where the camera focused, which no standard tag
//! records and which Sony keeps in a plain tag of this directory. ADR012
//! decided it, amending ADR011, under which no Sony MakerNote was read.
//! Nothing in the directory is used yet (#25).
//!
//! Sony has written the block in several layouts, told apart by its first
//! bytes:
//!
//! - **A header, then the IFD 12 bytes in.** The DSLR-A100's ARW 1.0 starts
//!   its MakerNote with `SONY DSC `, a space and three NULs included
//!   (public sample 2346 of raw.pixls.us). ExifTool's documentation lists
//!   four more headers for Sony files, seen in no sample: `SONY CAM ` and
//!   three NULs; `SONY MOBILE`; two NULs, `SONY PIC` and a NUL; and `VHAB`,
//!   five spaces and a NUL. The last three are shorter than 12 bytes, and
//!   the bytes after them, up to the IFD, are not looked at.
//! - **No header: the IFD at the block's start.** Every other ARW, SR2 and
//!   ARQ sample writes this one, the Hasselblad Lunar's among them (sample
//!   4051). The SRF files of the earliest bodies keep a chain of IFDs
//!   there; the first is read as a plain IFD, with its single entry, and
//!   the chain is not followed.
//! - **No Sony directory at all.** A block that starts with `SONY PI` and a
//!   NUL, or `PREMI` and a NUL, is in Olympus's format, with Olympus's
//!   tags, and one that starts with `SONY PIC` and a NUL is text.
//!   ExifTool's documentation lists them for the JPEG files of Sony's
//!   compacts, and none of the RAW samples holds one. Such a block gives no
//!   MakerNote, even with an IFD after it, since what it holds is not
//!   Sony's directory.
//!
//! The headers are looked for first, then the blocks that hold no Sony
//! directory; any other block is taken to start with its IFD. The file's
//! Make is not looked at: the ARW reader's claim has already decided that
//! the file is Sony's.
//!
//! Every offset inside the block counts from the start of the file, where
//! its TIFF header is, as the offsets of the file's own IFDs do: not from
//! the block, nor from a header of its own, as Nikon's do. On the A100,
//! tag 0xB028 records the offset of a Minolta-format IFD, and the IFD lies
//! at that offset from the file's start (sample 2346). So the IFD is read
//! with base 0. The block's numbers are in the file's byte order, the one
//! the Exif IFD was read in. Every sample is little-endian; no big-endian
//! Sony file is known, so that its MakerNote would follow the file's order
//! is unchecked.
//!
//! The four bytes after the IFD's entries are not a next-IFD pointer to
//! rely on: they are zero on the older bodies, such as the A100, the NEX-5
//! and the DSC-R1, and the first bytes of the values on the RX100 and every
//! ILCE and RX body since 2012. They are neither used nor required, as the
//! ARW security notes ask (`docs/formats/arw/security.md`, item A5). Nor
//! need the tags be sorted: Sony writes other IFDs with tags out of order
//! (the raw SubIFD of the ILCE-9, samples 1541 and 1542), and the IFD keeps
//! the order of the file, each tag found wherever it is.
//!
//! Nothing in the block is trusted. Its IFD is read by the reading of an
//! IFD inside a block that both MakerNote readers share
//! ([`block_ifd`](crate::formats::block_ifd)): the IFD must lie inside the
//! block with its count and its whole entry table, and every entry whose
//! value lies outside the block is dropped, as an entry whose value lies
//! outside the file is, so every value read from it lies inside the block.
//! The signatures are looked for in at most the block's first 12 bytes,
//! never past its end. A block too short for its header or for the IFD's
//! count, an IFD cut short by the block's end, one with no entries, and one
//! with more entries than the limits allow cost the MakerNote, never the
//! file. Only a failure of the reader itself is an error: that is not the
//! file's fault, and says nothing about the MakerNote.
//!
//! Nothing in the block is deciphered: Sony enciphers several of its tags'
//! values, and none of them is read. FileFormat (0xB000), SonyModelID
//! (0xB001) and the lens tags are not read either: the standard tags give
//! the camera and the lens (ADR011).
//!
//! The MakerNote is read once per file, into a [`MakerNote`]: its block
//! found and checked, its IFD parsed. Every use of the MakerNote is to read
//! from that same structure, as the Nikon reader's do, so that none of them
//! finds, checks or parses the block again.

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::tags::MAKER_NOTE;
use crate::container::tiff::{Ifd, Tree};
use crate::error::Result;
use crate::formats::block_ifd::{block_of, ifd_in};
use crate::io::Source;
use crate::limits::Limits;

/// The header of the DSLR-A100's MakerNote (`SONY DSC `, then three NULs),
/// which ExifTool's documentation also lists for the DSC compacts' JPEG
/// files.
const SONY_DSC: &[u8; 12] = b"SONY DSC \0\0\0";

/// A header ExifTool's documentation lists for Sony files (`SONY CAM `,
/// then three NULs), seen in no sample.
const SONY_CAM: &[u8; 12] = b"SONY CAM \0\0\0";

/// A header ExifTool's documentation lists for Sony files (`SONY MOBILE`),
/// seen in no sample; the twelfth byte is not looked at.
const SONY_MOBILE: &[u8; 11] = b"SONY MOBILE";

/// A header ExifTool's documentation lists for Sony files (two NULs, then
/// `SONY PIC` and a NUL), seen in no sample; the twelfth byte is not looked
/// at.
const SONY_PIC_HEADER: &[u8; 11] = b"\0\0SONY PIC\0";

/// A header ExifTool's documentation lists for Sony files (`VHAB`, five
/// spaces and a NUL), seen in no sample; the last two of the twelve bytes
/// are not looked at.
const VHAB: &[u8; 10] = b"VHAB     \0";

/// The headers that put the MakerNote IFD [`HEADER_LEN`] bytes into the
/// block.
const HEADERS: [&[u8]; 5] = [SONY_DSC, SONY_CAM, SONY_MOBILE, SONY_PIC_HEADER, VHAB];

/// How far into the block the MakerNote IFD starts after a header.
const HEADER_LEN: u64 = 12;

/// The start of an Olympus-format MakerNote from a Sony compact's JPEG
/// file (`SONY PI` and a NUL), with Olympus's tags.
const SONY_PI: &[u8; 8] = b"SONY PI\0";

/// The start of another Olympus-format MakerNote from a Sony compact's
/// JPEG file (`PREMI` and a NUL), with Olympus's tags.
const PREMI: &[u8; 6] = b"PREMI\0";

/// The start of a MakerNote of text from a Sony compact's JPEG file
/// (`SONY PIC` and a NUL).
const SONY_PIC: &[u8; 9] = b"SONY PIC\0";

/// The starts of the blocks that hold no Sony directory.
const NOT_SONY: [&[u8]; 3] = [SONY_PI, PREMI, SONY_PIC];

/// A Sony MakerNote, its block found and its IFD read: what every use of
/// the MakerNote starts from, so that the block is found, checked and
/// parsed once per file.
///
/// See the [module documentation](self) for the layouts.
#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "nothing uses the Sony MakerNote's directory yet (#25)"
    )
)]
pub(super) struct MakerNote {
    /// Where the block's bytes are in the file: the bounds that every
    /// offset found inside the block is checked against.
    block: Range<u64>,
    /// The MakerNote IFD, which holds Sony's tags: at the block's start, or
    /// 12 bytes in after a header, inside the block with its whole entry
    /// table, its offsets counted from the start of the file. Its entries
    /// whose values lie outside the block were dropped when it was read, so
    /// every value read from it lies inside the block.
    ifd: Ifd,
}

/// The Sony MakerNote of the file in `src`, whose IFDs are `tree`, read
/// within `limits`; `None` when the file has no Exif IFD, no MakerNote, a
/// MakerNote that is not bytes stored out of line, one that holds no Sony
/// directory, or one whose IFD does not hold together inside the block.
///
/// See the [module documentation](self) for the layouts and the checks.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "nothing uses the Sony MakerNote's directory yet (#25)"
    )
)]
pub(super) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    let Some(exif) = tree.exif() else {
        return Ok(None);
    };
    let Some(block) = exif.entry(MAKER_NOTE).and_then(block_of) else {
        return Ok(None);
    };
    // The block's bytes lie inside the file, as every kept entry's value
    // does, so a read that stays inside the block can fail only with the
    // reader. `block_of` gives a range whose end is not before its start.
    let len = block.end.saturating_sub(block.start);
    let head = src.bytes(block.start, len.min(HEADER_LEN))?;
    let Some(skip) = ifd_start(&head) else {
        return Ok(None);
    };
    // The IFD's offset from base 0, as a TIFF offset is written: a block
    // that starts near the top of the 32-bit range may put it past.
    let Some(offset) = block
        .start
        .checked_add(skip)
        .and_then(|at| u32::try_from(at).ok())
    else {
        return Ok(None);
    };
    // The block's numbers are in the file's byte order, the Exif IFD's.
    let Some(ifd) = ifd_in(src, &block, 0, offset, exif.order(), limits)? else {
        return Ok(None);
    };
    Ok(Some(MakerNote { block, ifd }))
}

/// How far into the block the MakerNote IFD starts, from `head`, the
/// block's first bytes, at most [`HEADER_LEN`] of them: after one of
/// [`HEADERS`], [`HEADER_LEN`]; at the start, without one; `None` when the
/// block starts as one of [`NOT_SONY`], and holds no Sony directory.
fn ifd_start(head: &[u8]) -> Option<u64> {
    if HEADERS.iter().any(|header| head.starts_with(header)) {
        Some(HEADER_LEN)
    } else if NOT_SONY.iter().any(|start| head.starts_with(start)) {
        None
    } else {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::MAKE;
    use crate::container::tiff::types::{ASCII, SHORT, UNDEFINED};
    use crate::container::tiff::{parse_header, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing::{FAR, FailsFar, Src, byte_order};
    use crate::tiff_builder::{Built, IfdId, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    // Tags that mean nothing here, for entries the tests read back by tag,
    // in the order an IFD sorts them in.
    const FIRST: u16 = 0x0101;
    const SECOND: u16 = 0x0102;
    const THIRD: u16 = 0x0103;
    const FOURTH: u16 = 0x0104;

    /// The Exif tag UserComment, UNDEFINED like the MakerNote: a tag that
    /// may hold any bytes and is not the MakerNote.
    const USER_COMMENT: u16 = 0x9286;

    /// The four SHORTs of the sound MakerNote's second entry, stored out of
    /// line.
    const SHORTS: [u16; 4] = [10, 20, 30, 40];

    /// A Sony file in byte order `order`, as a builder: IFD0 holds Make and
    /// points at an Exif IFD, which holds ExifVersion; and IFD0 and the
    /// Exif IFD.
    fn sony(order: Order) -> (TiffBuilder, IfdId, IfdId) {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder.ascii(ifd0, MAKE, "SONY").exif(ifd0, exif);
        builder.undefined(exif, 0x9000, b"0231");
        (builder, ifd0, exif)
    }

    /// Adds to `builder` a MakerNote IFD after `prefix`, that nothing points
    /// at, holding a SHORT under [`FIRST`], which the entry holds, and
    /// [`SHORTS`] under [`SECOND`], stored out of line; and returns it.
    fn sound_ifd(builder: &mut TiffBuilder, prefix: &[u8]) -> IfdId {
        let makernote = builder.ifd();
        builder
            .prefix(makernote, prefix)
            .short(makernote, FIRST, &[7])
            .short(makernote, SECOND, &SHORTS);
        makernote
    }

    /// A Sony file in byte order `order` whose MakerNote is a sound IFD
    /// after `prefix`; the file, and the MakerNote IFD.
    fn sound_file(order: Order, prefix: &[u8]) -> (Built, IfdId) {
        let (mut builder, _, exif) = sony(order);
        let makernote = sound_ifd(&mut builder, prefix);
        builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        (builder.build(), makernote)
    }

    /// Adds to `exif` a MakerNote of type `typ` and `count` values, whose
    /// value field holds `at`.
    fn makernote_at(builder: &mut TiffBuilder, exif: IfdId, typ: u16, count: u64, at: u64) {
        let count = u32::try_from(count).unwrap();
        let at = u32::try_from(at).unwrap();
        builder.raw(exif, MAKER_NOTE, typ, count, RawValue::Offset(at));
    }

    /// The file `bytes`, read into a source and a tree as opening a file
    /// reads it.
    fn opened(bytes: Vec<u8>) -> (Src, Tree) {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
        (src, tree)
    }

    /// What [`read`] gives for the file `bytes` under `limits`, which must
    /// not be an error, and the source to read its values through.
    fn read_with(bytes: Vec<u8>, limits: &Limits) -> (Src, Option<MakerNote>) {
        let (mut src, tree) = opened(bytes);
        let makernote = read(&mut src, &tree, limits).unwrap();
        (src, makernote)
    }

    /// As [`read_with`], under the default limits.
    fn read_file(bytes: Vec<u8>) -> (Src, Option<MakerNote>) {
        read_with(bytes, &Limits::default())
    }

    /// Checks that `makernote` is the sound MakerNote of `built`, whose
    /// IFD is `ifd`, in byte order `order`, with both its entries.
    fn assert_sound(src: &mut Src, makernote: &MakerNote, built: &Built, ifd: IfdId, order: Order) {
        assert_eq!(makernote.block, built.ifd_block(ifd), "{order:?}");
        let found = &makernote.ifd;
        assert_eq!(found.order(), byte_order(order), "{order:?}");
        assert_eq!(found.offset(), built.ifd_offset(ifd), "{order:?}");
        assert_eq!(found.base(), 0, "{order:?}");
        assert_eq!(found.len(), 2, "{order:?}");
        assert_eq!(found.invalid_entries(), 0, "{order:?}");
        assert_eq!(found.u32(src, FIRST).unwrap(), Some(7), "{order:?}");
        assert_eq!(
            found.u32s(src, SECOND).unwrap(),
            Some(SHORTS.map(u32::from).to_vec()),
            "{order:?}"
        );
    }

    /// `start`, filled with NULs up to `len` bytes.
    fn padded(start: &[u8], len: usize) -> Vec<u8> {
        let mut bytes = start.to_vec();
        bytes.resize(len, 0);
        bytes
    }

    #[test]
    fn a_headerless_makernote_is_read_with_its_entries() {
        for order in ORDERS {
            let (built, ifd) = sound_file(order, b"");
            // The block starts with the IFD.
            assert_eq!(built.ifd_block(ifd).start, built.ifd_offset(ifd));
            let (mut src, found) = read_file(built.bytes.clone());
            let found = found.expect("a MakerNote");
            assert_sound(&mut src, &found, &built, ifd, order);
        }
    }

    #[test]
    fn each_of_sonys_headers_puts_the_ifd_12_bytes_in() {
        // The headers shorter than 12 bytes are followed by bytes that are
        // not looked at, here not NULs.
        let headers: [&[u8; 12]; 5] = [
            b"SONY DSC \0\0\0",
            b"SONY CAM \0\0\0",
            b"SONY MOBILE?",
            b"\0\0SONY PIC\0?",
            b"VHAB     \0??",
        ];
        for header in headers {
            for order in ORDERS {
                let (built, ifd) = sound_file(order, header);
                assert_eq!(
                    built.ifd_offset(ifd),
                    built.ifd_block(ifd).start + 12,
                    "{header:?}"
                );
                let (mut src, found) = read_file(built.bytes.clone());
                let found = found.expect("a MakerNote");
                assert_sound(&mut src, &found, &built, ifd, order);
            }
        }
    }

    #[test]
    fn a_block_that_holds_no_sony_directory_gives_none() {
        // Each start with a sound IFD right after it, inside a block long
        // enough that the start's first two bytes, read as an IFD's entry
        // count, would announce a table that fits in it: so neither a
        // header's 12 bytes nor the headerless rule may read it, and the
        // limits do not stop it.
        const LEN: u64 = 256 * 1024;
        let starts: [(&[u8], usize); 3] = [(b"SONY PI\0", 12), (b"PREMI\0", 8), (b"SONY PIC\0", 9)];
        for (start, ifd_at) in starts {
            for order in ORDERS {
                let (mut builder, _, exif) = sony(order);
                let ifd = sound_ifd(&mut builder, &padded(start, ifd_at));
                // Offsets written into entries move nothing, so a copy with
                // a stand-in one gives where the block starts.
                let mut probe = builder.clone();
                makernote_at(&mut probe, exif, UNDEFINED, LEN, 0);
                let at = probe.build().ifd_block(ifd).start;
                makernote_at(&mut builder, exif, UNDEFINED, LEN, at);
                builder.pad_to(at + LEN);
                let built = builder.build();
                assert_eq!(built.ifd_offset(ifd), at + u64::try_from(ifd_at).unwrap());

                let offset = u32::try_from(built.ifd_offset(ifd)).unwrap();
                let (mut src, found) = read_with(built.bytes, &Limits::unlimited());
                assert!(found.is_none(), "{start:?}, {order:?}");

                // The IFD after the start is sound, and so ignored on
                // purpose.
                let block = at..at + LEN;
                let limits = Limits::unlimited();
                let sound = ifd_in(&mut src, &block, 0, offset, byte_order(order), &limits);
                assert_eq!(sound.unwrap().map(|ifd| ifd.len()), Some(2), "{start:?}");
            }
        }
    }

    #[test]
    fn a_file_without_an_exif_ifd_gives_none() {
        // A sound MakerNote, but in IFD0, where no MakerNote belongs.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            builder.ascii(ifd0, MAKE, "SONY");
            let ifd = sound_ifd(&mut builder, b"");
            builder.undefined_ifd(ifd0, MAKER_NOTE, ifd);
            assert!(read_file(builder.build().bytes).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn an_exif_ifd_without_a_makernote_gives_none() {
        // A sound block, but under another tag.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let ifd = sound_ifd(&mut builder, b"");
            builder.undefined_ifd(exif, USER_COMMENT, ifd);
            assert!(read_file(builder.build().bytes).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_that_is_not_bytes_gives_none() {
        // The bytes of a sound block, declared as text and as SHORTs.
        for order in ORDERS {
            let (built, ifd) = sound_file(order, b"");
            let block = built.ifd_block(ifd);
            let len = block.end - block.start;
            for (typ, count) in [(ASCII, len), (SHORT, len / 2)] {
                let (mut builder, _, exif) = sony(order);
                sound_ifd(&mut builder, b"");
                makernote_at(&mut builder, exif, typ, count, block.start);
                let bytes = builder.build().bytes;
                assert!(read_file(bytes).1.is_none(), "{order:?}, {typ}");
            }
        }
    }

    #[test]
    fn a_block_too_short_for_its_ifds_count_gives_none() {
        // Each block the last bytes of the file, so that a read past its end
        // would fail: a header alone, a header and one byte of a count, the
        // shortest block stored out of line, which holds an IFD's count of
        // 1 and no entry, and a header of 11 bytes alone.
        for order in ORDERS {
            let mut count_of_one = order.u16(1).to_vec();
            count_of_one.extend_from_slice(&[0; 3]);
            let blocks: [&[u8]; 4] = [
                b"SONY DSC \0\0\0",
                b"SONY DSC \0\0\0\x01",
                &count_of_one,
                b"SONY MOBILE",
            ];
            for block in blocks {
                let (mut builder, _, exif) = sony(order);
                let blob = builder.blob(block);
                builder.undefined_blob(exif, MAKER_NOTE, blob);
                let bytes = builder.build().bytes;
                assert!(read_file(bytes).1.is_none(), "{block:?}, {order:?}");
            }

            // One byte, which the entry holds: not a block at all.
            let (mut builder, _, exif) = sony(order);
            builder.undefined(exif, MAKER_NOTE, &[1]);
            assert!(read_file(builder.build().bytes).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn an_entry_table_that_runs_past_the_block_gives_none() {
        for order in ORDERS {
            // Two entries whose values the entries hold: the table ends
            // 2 + 24 bytes into the IFD. The file goes on past it.
            let (mut builder, _, exif) = sony(order);
            let ifd = builder.ifd();
            builder.short(ifd, FIRST, &[7]).long(ifd, SECOND, &[8]);
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let at = probe.build().ifd_offset(ifd);
            let table = 2 + 24;

            // The block ends one byte before the table does.
            let mut cut = builder.clone();
            makernote_at(&mut cut, exif, UNDEFINED, table - 1, at);
            assert!(read_file(cut.build().bytes).1.is_none(), "{order:?}");

            // The control: a block that ends with the table, before the
            // next-IFD pointer, gives the IFD.
            makernote_at(&mut builder, exif, UNDEFINED, table, at);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.block, at..at + table, "{order:?}");
            assert_eq!(found.ifd.u32(&mut src, SECOND).unwrap(), Some(8));
        }
    }

    #[test]
    fn an_entry_whose_value_runs_past_the_block_is_dropped() {
        for order in ORDERS {
            // The sound block's last value is its four SHORTs: a block one
            // byte shorter cuts it.
            let (mut builder, _, exif) = sony(order);
            let ifd = sound_ifd(&mut builder, b"");
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let block = probe.build().ifd_block(ifd);
            let len = block.end - block.start;

            let mut cut = builder.clone();
            makernote_at(&mut cut, exif, UNDEFINED, len - 1, block.start);
            let (mut src, found) = read_file(cut.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.ifd.u32(&mut src, FIRST).unwrap(), Some(7));
            assert!(found.ifd.entry(SECOND).is_none(), "{order:?}");
            assert_eq!(found.ifd.invalid_entries(), 1, "{order:?}");

            // The control: the whole block keeps it.
            makernote_at(&mut builder, exif, UNDEFINED, len, block.start);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.ifd.len(), 2, "{order:?}");
            assert_eq!(found.ifd.invalid_entries(), 0, "{order:?}");
            assert_eq!(
                found.ifd.u32s(&mut src, SECOND).unwrap(),
                Some(SHORTS.map(u32::from).to_vec())
            );
        }
    }

    #[test]
    fn the_four_bytes_after_the_entries_do_not_matter() {
        // A third entry's four SHORTs are the four bytes after the entries
        // and the first four of the second entry's values, which follow
        // them: so those four bytes are values, as on the newer bodies.
        // Written there: a zero pointer, one to nowhere, and two SHORTs.
        for order in ORDERS {
            let two_shorts = match order {
                Order::Little => 0x0002_0001,
                Order::Big => 0x0001_0002,
            };
            for (next, first) in [
                (0, [0, 0]),
                (0xFFFF_FFFF, [0xFFFF, 0xFFFF]),
                (two_shorts, [1, 2]),
            ] {
                let (mut builder, _, exif) = sony(order);
                let ifd = sound_ifd(&mut builder, b"");
                builder.undefined_ifd(exif, MAKER_NOTE, ifd);
                builder.next_raw(ifd, next);
                let mut probe = builder.clone();
                probe.raw(ifd, THIRD, SHORT, 4, RawValue::Offset(0));
                let after = probe.build().ifd_offset(ifd) + 2 + 3 * 12;
                let after = u32::try_from(after).unwrap();
                builder.raw(ifd, THIRD, SHORT, 4, RawValue::Offset(after));
                let built = builder.build();

                let (mut src, found) = read_file(built.bytes);
                let found = found.expect("a MakerNote");
                let ifd = &found.ifd;
                assert_eq!(ifd.len(), 3, "{order:?}, {next:#X}");
                assert_eq!(ifd.invalid_entries(), 0, "{order:?}, {next:#X}");
                assert_eq!(ifd.u32(&mut src, FIRST).unwrap(), Some(7));
                assert_eq!(
                    ifd.u32s(&mut src, SECOND).unwrap(),
                    Some(SHORTS.map(u32::from).to_vec())
                );
                let [a, b] = first;
                let [c, d, ..] = SHORTS;
                assert_eq!(
                    ifd.u32s(&mut src, THIRD).unwrap(),
                    Some([a, b, c, d].map(u32::from).to_vec()),
                    "{order:?}, {next:#X}"
                );
            }
        }
    }

    #[test]
    fn unsorted_tags_are_each_found() {
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let ifd = builder.ifd();
            builder
                .unsorted(ifd)
                .short(ifd, FOURTH, &[4])
                .short(ifd, THIRD, &[3])
                .short(ifd, SECOND, &[2])
                .short(ifd, FIRST, &[1]);
            builder.undefined_ifd(exif, MAKER_NOTE, ifd);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            let tags: Vec<u16> = found.ifd.tags().collect();
            assert_eq!(tags, [FOURTH, THIRD, SECOND, FIRST], "{order:?}");
            for (tag, value) in [(FIRST, 1), (SECOND, 2), (THIRD, 3), (FOURTH, 4)] {
                assert_eq!(
                    found.ifd.u32(&mut src, tag).unwrap(),
                    Some(value),
                    "{order:?}, {tag:#06X}"
                );
            }
        }
    }

    #[test]
    fn a_makernote_ifd_with_more_entries_than_the_limit_gives_none() {
        // The sound IFD holds two entries; the walk is not limited.
        for order in ORDERS {
            let (built, _) = sound_file(order, b"");
            let limits = Limits::default().max_ifd_entries(2);
            assert!(read_with(built.bytes.clone(), &limits).1.is_some());
            let limits = Limits::default().max_ifd_entries(1);
            assert!(read_with(built.bytes, &limits).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn the_first_ifd_of_an_srf_chain_is_read_alone() {
        // An SRF's MakerNote: an IFD of one LONG under tag 0x0003, whose
        // next-IFD pointer leads to the next IFD of the chain, inside the
        // block.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let srf0 = builder.ifd();
            let srf1 = builder.ifd();
            builder.long(srf0, 0x0003, &[1234]).next(srf0, Some(srf1));
            builder.short(srf1, FIRST, &[1]).short(srf1, SECOND, &[2]);
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let probe = probe.build();
            let block = probe.ifd_block(srf0).start..probe.ifd_block(srf1).end;
            makernote_at(
                &mut builder,
                exif,
                UNDEFINED,
                block.end - block.start,
                block.start,
            );

            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.block, block, "{order:?}");
            assert_eq!(found.ifd.len(), 1, "{order:?}");
            assert_eq!(found.ifd.u32(&mut src, 0x0003).unwrap(), Some(1234));
            assert!(found.ifd.entry(FIRST).is_none(), "{order:?}");
        }
    }

    /// A Sony file in byte order `order` whose MakerNote, a sound IFD after
    /// the `SONY DSC` header, starts at `start`, an even offset: an IFD
    /// that nothing points at, before the MakerNote, holds bytes that push
    /// it there.
    fn far_file(order: Order, start: u64) -> (Built, IfdId) {
        let filler_entry = |builder: &mut TiffBuilder, filler: IfdId, len: u64| {
            let len = usize::try_from(len).unwrap();
            builder.undefined(filler, FIRST, &vec![0; len]);
        };
        let (mut builder, _, exif) = sony(order);
        let filler = builder.ifd();
        let ifd = sound_ifd(&mut builder, b"SONY DSC \0\0\0");
        builder.undefined_ifd(exif, MAKER_NOTE, ifd);
        let mut probe = builder.clone();
        filler_entry(&mut probe, filler, 8);
        // The filler's bytes start after its one entry, at an even offset,
        // and the MakerNote's header right after them.
        let bytes_at = probe.build().ifd_offset(filler) + 2 + 12 + 4;
        filler_entry(&mut builder, filler, start - bytes_at);
        let built = builder.build();
        assert_eq!(built.ifd_block(ifd).start, start);
        (built, ifd)
    }

    #[test]
    fn a_failure_of_the_reader_is_an_error() {
        // The block at FAR and past it, so that reading its header fails;
        // and its header just before FAR, its IFD at FAR, so that reading
        // the IFD fails.
        for start in [FAR, 2 * FAR, FAR - 12] {
            let order = Order::Little;
            let (built, ifd) = far_file(order, start);
            // The file is sound, for a reader that does not fail.
            let (mut src, found) = read_file(built.bytes.clone());
            assert_sound(&mut src, &found.expect("a MakerNote"), &built, ifd, order);

            let mut src = Source::new(FailsFar(Cursor::new(built.bytes))).unwrap();
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
            let err = read(&mut src, &tree, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{start}: {err:?}");
        }
    }
}
