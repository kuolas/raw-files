// SPDX-License-Identifier: MIT OR Apache-2.0

//! The MakerNote readers: one per camera maker whose MakerNote raw-files
//! reads.
//!
//! A MakerNote is a camera maker's own metadata, stored as the value of
//! one Exif tag, MakerNote, in a layout each maker sets. It records what
//! the standard tags lack, such as the point where the camera focused. The
//! readers here read the layouts raw-files knows, and are shared by every
//! format whose files carry them: NEF and ARW files, and JPEG files, whose
//! Exif segment holds the same MakerNote. The JPEG reader reads both,
//! Nikon's and Sony's, each by IFD0's Make.
//!
//! - [`nikon`] reads Nikon's MakerNote: its small preview JPEG, the visible
//!   image size, the focus point, the camera settings that only it
//!   records, such as the white balance setting and the shutter used, and
//!   the values older bodies record only there.
//! - [`points`] holds the tables of a body's autofocus points, measured by
//!   the project, which no file records: detection gives every file whose
//!   Model names a body with a table that body's layout of points, after
//!   the format's reader, whatever the file's MakerNote holds.
//! - [`sony`] reads Sony's MakerNote: the focus point, and the camera
//!   settings that only it records, such as the white balance setting and
//!   the electronic front curtain setting.
//!
//! What the readers share lives here, in a [`MakerNoteBlock`]: the
//! MakerNote's block as the Exif IFD records it, where its bytes lie and
//! the Exif IFD whose MakerNote it is. A reader finds the block through it
//! and tells its maker's layout by the block's first bytes. A layout whose
//! offsets count from the header of the TIFF structure that holds the
//! MakerNote, as Sony's and the headerless layout of Nikon's D1 family do,
//! has its IFD read through it too, with that structure's base and byte
//! order. A layout with a TIFF header of its own, as Nikon's other one,
//! takes only where the block's bytes lie, and reads its IFD from that
//! header. One bound holds every colour temperature a MakerNote gives,
//! whatever its maker: 1 to 99,999 K ([`checked_color_temperature`]).

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::tags::MAKER_NOTE;
use crate::container::tiff::{Block, Entry, Ifd, Tree, ifd_in};
use crate::error::Result;
use crate::io::Source;
use crate::limits::Limits;

pub(in crate::formats) mod nikon;
pub(in crate::formats) mod points;
pub(in crate::formats) mod sony;

/// A MakerNote's block as the TIFF structure that holds it records it:
/// what every reader starts from, to tell its maker's layout by the
/// block's first bytes and to read the IFD that holds the maker's tags.
///
/// It keeps where the block's bytes lie and the Exif IFD whose MakerNote
/// it is. The Exif IFD gives what the offsets inside the block count from
/// in a layout without a TIFF header of its own, the header of the
/// structure that holds the MakerNote, and the byte order of that
/// structure's numbers.
#[derive(Debug)]
struct MakerNoteBlock<'a> {
    /// The Exif IFD whose MakerNote entry holds the block. The base of its
    /// block is the header of the TIFF structure that holds the MakerNote,
    /// the start of the file in a RAW file and the TIFF header inside the
    /// Exif segment in a JPEG file, and its byte order is that structure's.
    exif: &'a Ifd,
    /// Where the block's bytes lie in the file: the MakerNote entry's value,
    /// which lies inside the Exif IFD's block and the file, as every kept
    /// entry's value does, and whose end is not before its start.
    range: Range<u64>,
}

impl<'a> MakerNoteBlock<'a> {
    /// The MakerNote's block of the file whose IFDs are `tree`: the value of
    /// the Exif IFD's MakerNote tag; `None` when the file has no Exif IFD,
    /// no MakerNote, or a MakerNote that is not bytes stored out of line
    /// ([`Entry::byte_range`]).
    fn find(tree: &'a Tree) -> Option<Self> {
        let exif = tree.exif()?;
        let range = exif.entry(MAKER_NOTE).and_then(Entry::byte_range)?;
        Some(Self { exif, range })
    }

    /// Where the block's bytes lie in the file: what a layout with a TIFF
    /// header of its own reads its IFD in, with that header as the base.
    /// The range lies inside the file, and its end is not before its start.
    fn range(&self) -> &Range<u64> {
        &self.range
    }

    /// The block's first `max` bytes, or all of them when the block is
    /// shorter: where a reader looks for its maker's header, never past the
    /// block's end.
    ///
    /// # Errors
    ///
    /// `Io` when the reader fails. The block's bytes lie inside the file,
    /// so a read that stays inside the block can fail only with the reader.
    fn first_bytes<R: Read + Seek>(&self, src: &mut Source<R>, max: u64) -> Result<Vec<u8>> {
        let len = self.range.end.saturating_sub(self.range.start);
        src.bytes(self.range.start, len.min(max))
    }

    /// The IFD `skip` bytes into the block, read within `limits` as the
    /// TIFF structure that holds the MakerNote reads its own: its offsets
    /// counted from that structure's header, the base of the Exif IFD's
    /// block, and its numbers in that structure's byte order, the Exif
    /// IFD's. `None` when no offset counted from that header reaches the
    /// IFD ([`ifd_offset`]), or when the IFD does not hold together inside
    /// the block.
    ///
    /// It serves the layouts whose offsets count from that header, Sony's
    /// and the headerless layout of Nikon's D1 family; a layout with a TIFF
    /// header of its own, as Nikon's other one, counts from that header
    /// instead, in the block's bytes ([`range`](Self::range)). The IFD is
    /// read inside the block ([`ifd_in`]): the bounds of the block it is
    /// read in are the MakerNote's bytes, so every value read from it lies
    /// inside them.
    ///
    /// # Errors
    ///
    /// `Io` when the reader fails. Nothing the block holds is an error.
    fn ifd_at<R: Read + Seek>(
        &self,
        src: &mut Source<R>,
        skip: u64,
        limits: &Limits,
    ) -> Result<Option<Ifd>> {
        let base = self.exif.block().base();
        let Some(offset) = ifd_offset(base, self.range.start, skip) else {
            return Ok(None);
        };
        let block = Block::new(base, self.range.clone());
        ifd_in(src, &block, offset, self.exif.order(), limits)
    }
}

/// The offset of the MakerNote IFD as a TIFF offset is written, counted
/// from `base`, the header of the TIFF structure that holds the MakerNote,
/// for an IFD `skip` bytes into a MakerNote that starts at position
/// `start`; `None` when the IFD lies before the base or more than the
/// 32-bit range past it.
///
/// No offset written in the structure counts back from its header, so an
/// IFD that lies before the header is one the offsets could not reach,
/// and gives no MakerNote. A MakerNote the Exif IFD points at starts at the
/// base or past it, since the entry's offset counts from the same base, so
/// that check only keeps the subtraction from wrapping. A MakerNote that
/// starts near the top of the 32-bit range past the base may put its IFD
/// out of reach too.
fn ifd_offset(base: u64, start: u64, skip: u64) -> Option<u32> {
    start
        .checked_add(skip)
        .and_then(|at| at.checked_sub(base))
        .and_then(|offset| u32::try_from(offset).ok())
}

/// The highest colour temperature, in kelvin, that a MakerNote's value
/// gives: 99,999 K, the largest number of five digits, the most that
/// Nikon's white balance text may hold for a chosen colour temperature
/// ([`nikon`]'s `MAX_KELVIN_DIGITS`).
const MAX_COLOR_TEMPERATURE: u32 = 99_999;

/// `kelvin`, a colour temperature a MakerNote records, when it records
/// one: from 1 to [`MAX_COLOR_TEMPERATURE`]. 0, which a maker writes where it
/// records no temperature, gives `None`, and so does a number past the bound,
/// which is no temperature a camera is set to: the reader refuses it rather
/// than report it.
fn checked_color_temperature(kelvin: u32) -> Option<u32> {
    (1..=MAX_COLOR_TEMPERATURE)
        .contains(&kelvin)
        .then_some(kelvin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::types::{ASCII, UNDEFINED};
    use crate::container::tiff::walk;
    use crate::formats::testing::{Src, byte_order};
    use crate::tiff_builder::{Built, IfdId, Order, RawValue, TiffBuilder};
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    // Tags that mean nothing here, for entries the tests read back by tag.
    const FIRST: u16 = 0x0101;
    const SECOND: u16 = 0x0102;

    /// The four SHORTs of the MakerNote IFD's second entry, stored out of
    /// line, so that reading them back takes an offset counted from the
    /// right base.
    const SHORTS: [u16; 4] = [10, 20, 30, 40];

    /// A header of 12 bytes before a MakerNote IFD, as Sony's `SONY DSC`
    /// header is.
    const HEADER: &[u8; 12] = b"SONY DSC \0\0\0";

    /// Where the tests of a TIFF structure inside a larger file place its
    /// header, as a JPEG file holds the structure of its Exif segment.
    const POS: u64 = 1000;

    /// A TIFF structure in byte order `order`, as a builder: IFD0 points at
    /// an Exif IFD, which holds ExifVersion; and the Exif IFD.
    fn structure(order: Order) -> (TiffBuilder, IfdId) {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder.exif(ifd0, exif);
        builder.undefined(exif, 0x9000, b"0231");
        (builder, exif)
    }

    /// A TIFF structure in byte order `order` whose MakerNote is an IFD
    /// after `prefix`, holding a SHORT under [`FIRST`], which the entry
    /// holds, and [`SHORTS`] under [`SECOND`]; the structure, and the
    /// MakerNote IFD.
    fn with_makernote(order: Order, prefix: &[u8]) -> (Built, IfdId) {
        let (mut builder, exif) = structure(order);
        let makernote = builder.ifd();
        builder
            .prefix(makernote, prefix)
            .short(makernote, FIRST, &[7])
            .short(makernote, SECOND, &SHORTS);
        builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        (builder.build(), makernote)
    }

    /// The structure `bytes` at position `pos` of a larger file, after
    /// `pos` NULs, read into a source and walked with its header at `pos`
    /// as the base and the bytes from there as the bounds, as a JPEG
    /// file's Exif segment is.
    fn opened_at(bytes: &[u8], pos: u64) -> (Src, Tree) {
        let mut file = vec![0; usize::try_from(pos).unwrap()];
        file.extend_from_slice(bytes);
        let mut src = Source::new(Cursor::new(file)).unwrap();
        let block = Block::new(pos, pos..src.len());
        let tree = walk(&mut src, &block, &Limits::default()).unwrap();
        (src, tree)
    }

    #[test]
    fn the_block_is_the_value_of_the_exif_ifds_makernote() {
        for order in ORDERS {
            for pos in [0, POS] {
                let (built, ifd) = with_makernote(order, HEADER);
                let (mut src, tree) = opened_at(&built.bytes, pos);
                let block = MakerNoteBlock::find(&tree).expect("a block");
                let range = built.ifd_block(ifd);
                let label = format!("{order:?}, {pos}");
                assert_eq!(
                    block.range(),
                    &(pos + range.start..pos + range.end),
                    "{label}"
                );
                assert!(std::ptr::eq(block.exif, tree.exif().unwrap()), "{label}");
                assert_eq!(block.first_bytes(&mut src, 12).unwrap(), HEADER, "{label}");
                assert_eq!(block.first_bytes(&mut src, 4).unwrap(), b"SONY");
            }
        }
    }

    #[test]
    fn the_first_bytes_stop_at_the_blocks_end() {
        for order in ORDERS {
            let (mut builder, exif) = structure(order);
            let blob = builder.blob(b"12345");
            builder.undefined_blob(exif, MAKER_NOTE, blob);
            let (mut src, tree) = opened_at(&builder.build().bytes, 0);
            let block = MakerNoteBlock::find(&tree).expect("a block");
            assert_eq!(block.first_bytes(&mut src, 12).unwrap(), b"12345");
            assert_eq!(block.first_bytes(&mut src, 5).unwrap(), b"12345");
            assert_eq!(block.first_bytes(&mut src, 0).unwrap(), b"");
        }
    }

    #[test]
    fn a_file_without_a_makernote_of_bytes_has_no_block() {
        for order in ORDERS {
            // No Exif IFD, a MakerNote in IFD0 instead.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let blob = builder.blob(b"12345");
            builder.undefined_blob(ifd0, MAKER_NOTE, blob);
            let (_, tree) = opened_at(&builder.build().bytes, 0);
            assert!(MakerNoteBlock::find(&tree).is_none(), "{order:?}");

            // An Exif IFD without a MakerNote.
            let (builder, _) = structure(order);
            let (_, tree) = opened_at(&builder.build().bytes, 0);
            assert!(MakerNoteBlock::find(&tree).is_none(), "{order:?}");

            // A MakerNote of text, and one held in its entry: entries the
            // walk keeps, but whose values are not bytes stored out of line.
            // Offsets written into entries move nothing, so a copy with a
            // stand-in one gives where the text lies.
            let (mut text, exif) = structure(order);
            let blob = text.blob(b"12345");
            let mut probe = text.clone();
            probe.raw(exif, MAKER_NOTE, ASCII, 5, RawValue::Offset(0));
            let at = u32::try_from(probe.build().blob_offset(blob)).unwrap();
            text.raw(exif, MAKER_NOTE, ASCII, 5, RawValue::Offset(at));
            let (mut inline, exif) = structure(order);
            inline.undefined(exif, MAKER_NOTE, b"1234");
            for builder in [text, inline] {
                let (_, tree) = opened_at(&builder.build().bytes, 0);
                let entry = tree.exif().and_then(|exif| exif.entry(MAKER_NOTE));
                assert!(entry.is_some(), "{order:?}");
                assert!(MakerNoteBlock::find(&tree).is_none(), "{order:?}");
            }
        }
    }

    #[test]
    fn the_ifd_counts_from_the_structures_header_in_its_byte_order() {
        // The base at the start of the file and further in, the IFD at the
        // block's start and 12 bytes in. Counted from anywhere but the
        // structure's header, the offset of SHORTS would miss them.
        for order in ORDERS {
            for pos in [0, POS] {
                for prefix in [&b""[..], HEADER] {
                    let (built, ifd) = with_makernote(order, prefix);
                    let (mut src, tree) = opened_at(&built.bytes, pos);
                    let block = MakerNoteBlock::find(&tree).expect("a block");
                    let skip = u64::try_from(prefix.len()).unwrap();
                    let limits = Limits::default();
                    let found = block.ifd_at(&mut src, skip, &limits).unwrap();
                    let found = found.expect("an IFD");
                    let label = format!("{order:?}, {pos}, {skip}");
                    let range = built.ifd_block(ifd);
                    assert_eq!(found.offset(), pos + built.ifd_offset(ifd), "{label}");
                    assert_eq!(found.block().base(), pos, "{label}");
                    assert_eq!(
                        found.block().bounds(),
                        &(pos + range.start..pos + range.end),
                        "{label}"
                    );
                    assert_eq!(found.order(), byte_order(order), "{label}");
                    assert_eq!(found.invalid_entries(), 0, "{label}");
                    assert_eq!(found.u32(&mut src, FIRST).unwrap(), Some(7), "{label}");
                    assert_eq!(
                        found.u32s(&mut src, SECOND).unwrap(),
                        Some(SHORTS.map(u32::from).to_vec()),
                        "{label}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_ifd_that_does_not_hold_together_inside_the_block_gives_none() {
        for order in ORDERS {
            // Two entries whose values the entries hold: the table ends
            // 2 + 24 bytes into the IFD. The file goes on past it, so only
            // the block's end can cut it.
            let (mut builder, exif) = structure(order);
            let ifd = builder.ifd();
            builder.short(ifd, FIRST, &[7]).long(ifd, SECOND, &[8]);
            let mut probe = builder.clone();
            probe.raw(exif, MAKER_NOTE, UNDEFINED, 0, RawValue::Offset(0));
            let at = u32::try_from(probe.build().ifd_offset(ifd)).unwrap();
            let table = 2 + 24;
            let declared = |len: u32| {
                let mut builder = builder.clone();
                builder.raw(exif, MAKER_NOTE, UNDEFINED, len, RawValue::Offset(at));
                opened_at(&builder.build().bytes, 0)
            };
            let limits = Limits::default();

            // The block ends one byte before the table does.
            let (mut src, tree) = declared(table - 1);
            let block = MakerNoteBlock::find(&tree).expect("a block");
            let found = block.ifd_at(&mut src, 0, &limits).unwrap();
            assert!(found.is_none(), "{order:?}");

            // The control: a block that ends with the table gives the IFD,
            // unless it holds more entries than the limits allow.
            let (mut src, tree) = declared(table);
            let block = MakerNoteBlock::find(&tree).expect("a block");
            let found = block.ifd_at(&mut src, 0, &limits).unwrap();
            let found = found.expect("an IFD");
            assert_eq!(found.u32(&mut src, SECOND).unwrap(), Some(8), "{order:?}");
            let limits = Limits::default().max_ifd_entries(1);
            let found = block.ifd_at(&mut src, 0, &limits).unwrap();
            assert!(found.is_none(), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_ifd_before_the_base_has_no_offset() {
        // An offset as a TIFF offset is written, counted from the base.
        assert_eq!(ifd_offset(0, 600, 12), Some(612));
        assert_eq!(ifd_offset(1000, 1000, 0), Some(0));
        assert_eq!(ifd_offset(1000, 1600, 12), Some(612));
        // The IFD before the base, whether its block starts there or only
        // the header before it does.
        assert_eq!(ifd_offset(1000, 900, 0), None);
        assert_eq!(ifd_offset(1000, 988, 11), None);
        assert_eq!(ifd_offset(1000, 988, 12), Some(0));
        // The IFD past the 32-bit range from the base, and past the largest
        // position.
        let top = 1000 + u64::from(u32::MAX);
        assert_eq!(ifd_offset(1000, top, 0), Some(u32::MAX));
        assert_eq!(ifd_offset(1000, top, 1), None);
        assert_eq!(ifd_offset(0, u64::MAX, 12), None);
    }
}
