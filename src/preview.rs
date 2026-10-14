// SPDX-License-Identifier: MIT OR Apache-2.0

//! Embedded previews: the images a camera writes into a RAW file next to the
//! raw data, and how to choose among them.

use std::ops::Range;

use crate::{Orientation, Size};

/// One embedded preview a camera wrote into the file: where its bytes are,
/// its measured size, its format and the orientation a viewer must apply.
///
/// A `Preview` describes the image; it does not hold its bytes. It is a
/// small `Copy` value, so a caller copies the one it wants out of the file
/// handle and then reads it, since reading needs the handle mutably.
///
/// The size comes from the JPEG's own frame header, never from the tags
/// that describe the preview: those are often missing, zero or wrong.
///
/// # Examples
///
/// A `Preview` describes where the bytes are, so a caller that holds the file
/// in memory need not copy them: this borrows the largest preview from the
/// buffer and saves it under the extension its format uses.
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes = std::fs::read("photo.nef")?;
/// let file = RawFile::from_bytes(&bytes)?;
/// if let Some(preview) = file.largest_preview() {
///     let range = preview.byte_range();
///     let (start, end) = (usize::try_from(range.start)?, usize::try_from(range.end)?);
///     let jpeg = &bytes[start..end];
///     let name = format!("preview.{}", preview.format().extension());
///     std::fs::write(name, jpeg)?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Preview {
    offset: u64,
    length: u64,
    width: u32,
    height: u32,
    format: PreviewFormat,
    orientation: Orientation,
}

impl Preview {
    /// A preview of `length` bytes starting at byte `offset` of the file,
    /// `width` by `height` pixels as stored, in `format`, shown upright by
    /// applying `orientation`.
    ///
    /// The caller has checked that `offset + length` lies inside the file,
    /// so the sum does not overflow.
    pub(crate) fn new(
        offset: u64,
        length: u64,
        width: u32,
        height: u32,
        format: PreviewFormat,
        orientation: Orientation,
    ) -> Self {
        Self {
            offset,
            length,
            width,
            height,
            format,
            orientation,
        }
    }

    /// The number of pixel columns, as stored, read from the JPEG's frame
    /// header rather than from the file's tags.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The number of pixel rows, as stored, read from the JPEG's frame
    /// header rather than from the file's tags.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The width and height together, as stored, before the
    /// [`orientation`](Self::orientation) is applied.
    pub fn size(&self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    /// How the preview's bytes are encoded.
    pub fn format(&self) -> PreviewFormat {
        self.format
    }

    /// What a viewer must apply to the stored pixels to show the preview
    /// upright. The preview's bytes are not rotated.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Where the preview's bytes are in the file, as a range of byte
    /// offsets.
    ///
    /// A caller that already holds the whole file in memory can borrow the
    /// preview from it with this range instead of having it copied.
    pub fn byte_range(&self) -> Range<u64> {
        // Never saturates for a range the preview finder produced.
        self.offset..self.offset.saturating_add(self.length)
    }

    /// How many bytes the preview takes in the file.
    pub fn byte_len(&self) -> u64 {
        self.length
    }
}

/// How a preview's bytes are encoded.
///
/// `PreviewFormat` is `#[non_exhaustive]`: other encodings may be added as
/// more RAW formats are supported, so a `match` on it needs a wildcard arm.
///
/// # Examples
///
/// ```
/// use raw_files::PreviewFormat;
///
/// assert_eq!(PreviewFormat::Jpeg.mime_type(), "image/jpeg");
/// assert_eq!(PreviewFormat::Jpeg.extension(), "jpg");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PreviewFormat {
    /// A baseline or progressive JPEG stream, which any image viewer shows.
    Jpeg,
}

impl PreviewFormat {
    /// The media type of the encoding, for an HTTP header or a clipboard.
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
        }
    }

    /// The usual file name extension for the encoding, in lower case and
    /// without the dot, for saving a preview to its own file.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
        }
    }
}

/// Orders `previews` smallest first: by pixel area, then by byte length.
///
/// The area is computed in `u64`, so it cannot overflow. The sort is
/// stable: previews equal in both keep their order.
pub(crate) fn sort(previews: &mut [Preview]) {
    previews.sort_by_key(|preview| {
        (
            u64::from(preview.width) * u64::from(preview.height),
            preview.length,
        )
    });
}

/// The smallest preview at least `width` by `height`, from `previews`
/// already ordered by [`sort`].
///
/// The comparison uses the stored size, before the orientation is applied:
/// the caller knows the orientation and can swap its box, and the preview
/// chosen is the same bytes either way. When none is large enough, the
/// largest is returned, so a caller asking for more than the file has still
/// gets the best there is. `None` only when there are no previews.
pub(crate) fn at_least(previews: &[Preview], width: u32, height: u32) -> Option<Preview> {
    previews
        .iter()
        .find(|preview| preview.width >= width && preview.height >= height)
        .or(previews.last())
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A JPEG preview of `width` by `height` pixels, `length` bytes at
    /// `offset`, upright.
    fn preview(offset: u64, length: u64, width: u32, height: u32) -> Preview {
        Preview::new(
            offset,
            length,
            width,
            height,
            PreviewFormat::Jpeg,
            Orientation::NoTransforms,
        )
    }

    /// The offsets of `previews`, in order: each test gives its previews
    /// distinct offsets, so these name them.
    fn offsets(previews: &[Preview]) -> Vec<u64> {
        previews.iter().map(|preview| preview.offset).collect()
    }

    #[test]
    fn getters_return_what_was_given() {
        let preview = Preview::new(
            100,
            2_000,
            640,
            480,
            PreviewFormat::Jpeg,
            Orientation::Rotate90,
        );
        assert_eq!(preview.width(), 640);
        assert_eq!(preview.height(), 480);
        assert_eq!(
            preview.size(),
            Size {
                width: 640,
                height: 480
            }
        );
        assert_eq!(preview.format(), PreviewFormat::Jpeg);
        assert_eq!(preview.orientation(), Orientation::Rotate90);
        assert_eq!(preview.byte_range(), 100..2_100);
        assert_eq!(preview.byte_len(), 2_000);
    }

    #[test]
    fn jpeg_mime_type_and_extension() {
        assert_eq!(PreviewFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(PreviewFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn sort_orders_by_area_ascending() {
        let mut previews = [
            preview(1, 10, 1_620, 1_080),
            preview(2, 10, 160, 120),
            preview(3, 10, 8_256, 5_504),
            preview(4, 10, 640, 480),
        ];
        sort(&mut previews);
        assert_eq!(offsets(&previews), [2, 4, 1, 3]);
    }

    #[test]
    fn sort_breaks_an_area_tie_by_byte_length() {
        let mut previews = [preview(1, 900, 640, 480), preview(2, 500, 640, 480)];
        sort(&mut previews);
        assert_eq!(offsets(&previews), [2, 1]);
    }

    #[test]
    fn sort_keeps_the_order_of_equal_previews() {
        // Two sizes, interleaved, and enough of them that an unstable sort
        // would reorder the equal ones; a handful would not show it.
        let mut previews: Vec<Preview> = (0..64)
            .map(|offset| match offset % 2 {
                0 => preview(offset, 500, 640, 480),
                _ => preview(offset, 100, 160, 120),
            })
            .collect();
        sort(&mut previews);
        let odd = (0..64).filter(|offset| offset % 2 == 1);
        let even = (0..64).filter(|offset| offset % 2 == 0);
        assert_eq!(offsets(&previews), odd.chain(even).collect::<Vec<_>>());
    }

    #[test]
    fn sort_orders_equal_areas_by_byte_length_not_by_width() {
        // 1600 × 100 and 400 × 400 cover the same area. The wide one is the
        // smaller file, so it comes first, though ordering by width would
        // put it last.
        let mut previews = [preview(1, 900, 400, 400), preview(2, 500, 1_600, 100)];
        sort(&mut previews);
        assert_eq!(offsets(&previews), [2, 1]);
    }

    #[test]
    fn sort_computes_areas_past_u32_without_overflow() {
        // 65535 × 65535 still fits in u32; 65537 × 65537 is past 2^32, and
        // computed in u32 would wrap to 131073, smaller than the former.
        let mut previews = [
            preview(1, 10, 65_537, 65_537),
            preview(2, 10, 65_535, 65_535),
            preview(3, 10, 160, 120),
        ];
        sort(&mut previews);
        assert_eq!(offsets(&previews), [3, 2, 1]);
    }

    /// Three previews, sorted, as a NEF commonly holds them.
    fn sorted() -> [Preview; 3] {
        let mut previews = [
            preview(1, 10, 160, 120),
            preview(2, 20, 640, 480),
            preview(3, 30, 1_620, 1_080),
        ];
        sort(&mut previews);
        previews
    }

    #[test]
    fn at_least_gives_the_smallest_that_fits() {
        // Both 640 × 480 and 1620 × 1080 fit; the smaller is chosen.
        assert_eq!(at_least(&sorted(), 300, 200).map(|p| p.offset), Some(2));
    }

    #[test]
    fn at_least_accepts_an_exact_fit() {
        assert_eq!(at_least(&sorted(), 640, 480).map(|p| p.offset), Some(2));
    }

    #[test]
    fn at_least_falls_back_to_the_largest() {
        assert_eq!(at_least(&sorted(), 4_000, 3_000).map(|p| p.offset), Some(3));
    }

    #[test]
    fn at_least_of_nothing_is_none() {
        assert_eq!(at_least(&[], 0, 0), None);
        assert_eq!(at_least(&[], 640, 480), None);
    }

    #[test]
    fn at_least_zero_gives_the_smallest() {
        assert_eq!(at_least(&sorted(), 0, 0).map(|p| p.offset), Some(1));
    }

    #[test]
    fn at_least_compares_the_stored_size() {
        // Turned upright, 480 × 640 would be 640 × 480 and fit; as stored it
        // is too narrow, so the larger preview is chosen.
        let mut previews = [preview(1, 10, 480, 640), preview(2, 20, 1_620, 1_080)];
        sort(&mut previews);
        assert_eq!(at_least(&previews, 640, 480).map(|p| p.offset), Some(2));
    }
}
