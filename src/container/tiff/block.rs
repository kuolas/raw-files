// SPDX-License-Identifier: MIT OR Apache-2.0

//! The block a TIFF structure is read in: where its offsets count from,
//! and which bytes of the file it may use.
//!
//! Every offset written inside a TIFF structure counts from a position in
//! the file, the structure's base, and everything the structure points at,
//! its IFDs and their values, must lie inside a range of the file, its
//! bounds. A [`Block`] is that pair. Some examples:
//!
//! - a TIFF file's own structure, as a NEF or an ARW file holds it: base 0,
//!   and the whole file as bounds;
//! - the TIFF structure of a JPEG file's Exif segment: base at its TIFF
//!   header, 6 bytes into the payload of the APP1 segment, and the segment
//!   as bounds;
//! - the MPF index of a JPEG file: base at its MP header, and the APP2
//!   segment that holds it as bounds;
//! - the Nikon MakerNote: base at the TIFF header it holds, 10 bytes in,
//!   and the MakerNote as bounds;
//! - the Sony MakerNote: base at the header of the TIFF structure that
//!   holds it, since its offsets count from there (0 in an ARW file), and
//!   the MakerNote as bounds. The base lies outside the bounds, which a
//!   block allows.
//!
//! [`Block::at`] is the one place where an offset written in the file
//! becomes a position in the file, and [`Block::fits`] says whether a range
//! of the file lies inside the bounds.

use std::ops::Range;

use crate::io::within;

/// Where the offsets of a TIFF structure count from, its base, and the
/// range of the file it may use, its bounds.
///
/// The base may lie outside the bounds. An empty or inverted range is a
/// block that no byte fits in. The bounds may also run past the end of
/// the file: whoever reads inside them checks the file as well.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Block {
    base: u64,
    bounds: Range<u64>,
}

impl Block {
    /// The block of a file's own TIFF structure, for a file of `len`
    /// bytes: base 0, and the whole file as bounds.
    pub(crate) fn file(len: u64) -> Self {
        Self::new(0, 0..len)
    }

    /// The block whose offsets count from `base` and whose bytes are
    /// `bounds`.
    pub(crate) fn new(base: u64, bounds: Range<u64>) -> Self {
        Self { base, bounds }
    }

    /// The position in the file that the offsets inside the block count
    /// from.
    pub(crate) fn base(&self) -> u64 {
        self.base
    }

    /// The range of the file the block may use.
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing reads this method yet"))]
    pub(crate) fn bounds(&self) -> &Range<u64> {
        &self.bounds
    }

    /// The same block with its bounds cut at `len`, the end of a file of
    /// `len` bytes: what fits it lies inside both the bounds and the file.
    pub(crate) fn within_file(&self, len: u64) -> Self {
        Self::new(self.base, self.bounds.start..self.bounds.end.min(len))
    }

    /// Whether the `len` bytes at position `at` lie inside the bounds.
    ///
    /// A range whose end would pass the largest offset lies inside no
    /// bounds.
    pub(crate) fn fits(&self, at: u64, len: u64) -> bool {
        at >= self.bounds.start && within(at, len, self.bounds.end).is_some()
    }

    /// The position in the file of `offset`, an offset written inside the
    /// block: the base plus the offset; `None` if that would pass the
    /// largest offset. The position need not lie inside the bounds.
    pub(crate) fn at(&self, offset: u32) -> Option<u64> {
        self.base.checked_add(u64::from(offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_block_counts_from_byte_0_and_spans_the_file() {
        let block = Block::file(100);
        assert_eq!(block.base(), 0);
        assert_eq!(block.bounds(), &(0..100));
        assert_eq!(block, Block::new(0, 0..100));
    }

    #[test]
    fn a_range_inside_the_bounds_fits() {
        let block = Block::new(0, 100..200);
        assert!(block.fits(100, 10), "at the start");
        assert!(block.fits(190, 10), "ending exactly at the end");
        assert!(block.fits(100, 100), "the whole bounds");
        assert!(block.fits(200, 0), "nothing, at the end");
    }

    #[test]
    fn a_range_one_byte_past_the_end_does_not_fit() {
        let block = Block::new(0, 100..200);
        assert!(!block.fits(191, 10));
        assert!(!block.fits(200, 1));
    }

    #[test]
    fn a_range_starting_before_the_bounds_does_not_fit() {
        // It ends inside the bounds, so only its start is wrong.
        assert!(!Block::new(0, 100..200).fits(99, 10));
    }

    #[test]
    fn a_range_whose_end_overflows_does_not_fit() {
        // It starts inside bounds that reach the top of the u64 range, and
        // its end, wrapped around, would be small enough to fit.
        let block = Block::new(0, 100..u64::MAX);
        assert!(block.fits(u64::MAX - 10, 10));
        assert!(!block.fits(u64::MAX - 5, 10));
    }

    #[test]
    fn no_byte_fits_an_empty_or_inverted_range() {
        let empty = Block::new(0, 100..100);
        assert!(!empty.fits(100, 1));
        assert!(!empty.fits(99, 1));
        #[expect(
            clippy::reversed_empty_ranges,
            reason = "an inverted range is what this test is about"
        )]
        let inverted = Block::new(0, 200..100);
        for at in [99, 100, 150, 199, 200] {
            assert!(!inverted.fits(at, 1), "{at}");
            assert!(!inverted.fits(at, 0), "{at}");
        }
    }

    #[test]
    fn within_file_cuts_the_bounds_at_the_end_of_the_file() {
        let block = Block::new(10, 100..200);
        assert_eq!(block.within_file(150), Block::new(10, 100..150));
        assert_eq!(block.within_file(300), block, "the file goes on past");
        assert!(block.fits(160, 10));
        assert!(!block.within_file(150).fits(160, 10));
        // A file that ends before the bounds start leaves nothing.
        assert!(!block.within_file(50).fits(100, 1));
    }

    #[test]
    fn an_offset_counts_from_the_base() {
        let block = Block::new(1000, 0..2000);
        assert_eq!(block.at(0), Some(1000));
        assert_eq!(block.at(24), Some(1024));
        assert_eq!(block.at(u32::MAX), Some(1000 + u64::from(u32::MAX)));
        // The position need not lie inside the bounds.
        assert_eq!(Block::new(1000, 0..10).at(5000), Some(6000));
    }

    #[test]
    fn an_offset_whose_position_would_pass_the_largest_offset_has_none() {
        let block = Block::new(u64::MAX - 10, 0..u64::MAX);
        assert_eq!(block.at(10), Some(u64::MAX));
        assert_eq!(block.at(11), None);
        assert_eq!(block.at(u32::MAX), None);
    }

    #[test]
    fn the_base_may_lie_outside_the_bounds() {
        // As a Sony MakerNote's in an ARW file: its offsets count from the
        // start of the file, and it lies further in.
        let block = Block::new(0, 512..1024);
        assert_eq!(block.base(), 0);
        assert!(!block.fits(0, 1));
        assert_eq!(block.at(600), Some(600));
        assert!(block.fits(block.at(600).unwrap(), 8));
    }
}
