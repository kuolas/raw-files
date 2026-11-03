// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sizes and positions in pixels.

use std::fmt;

/// The size of an image in pixels, as stored in the file, before any
/// [`Orientation`](crate::Orientation) is applied.
///
/// For an orientation that swaps the dimensions, the size a viewer shows is
/// this size with its width and height exchanged.
///
/// Sizes order by width, then by height, so that they can key a map or be
/// sorted; to compare how large two pictures are, compare their areas.
///
/// As text ([`Display`](fmt::Display)), a size is its width and its height
/// around a plain `x`, `8256x5504`, as command-line tools write sizes: the
/// text is ASCII, so it fits a file name or a log line as it is. The docs
/// write sizes with a multiplication sign in prose.
///
/// Its fields are public because it is a small fixed value: a width and a
/// height, nothing more. There is no `Default`, since a size of 0 × 0 is
/// no image, and [`Focus::new`](crate::Focus::new) refuses it as a frame.
///
/// # Examples
///
/// The size a viewer shows, once the orientation is applied:
///
/// ```
/// use raw_files::{Orientation, Size};
///
/// let stored = Size { width: 8256, height: 5504 };
/// let orientation = Orientation::Rotate270;
/// let shown = if orientation.swaps_dimensions() {
///     Size { width: stored.height, height: stored.width }
/// } else {
///     stored
/// };
/// assert_eq!(shown, Size { width: 5504, height: 8256 });
/// assert_eq!(shown.to_string(), "5504x8256");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Size {
    /// The number of pixel columns, as stored.
    pub width: u32,
    /// The number of pixel rows, as stored.
    pub height: u32,
}

impl Size {
    /// How many pixels an image of this size has. It is a `u64`, so it
    /// cannot overflow: two `u32`s multiplied always fit.
    pub(crate) fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Whether focus points measured in a frame of this size fit a picture
    /// of `picture` pixels: whether this size has the picture's aspect
    /// ratio, to within one pixel of rounding in either of the two sizes,
    /// the rule a JPEG's points are held to (ADR014).
    ///
    /// That is when the frame's width times the picture's height and the
    /// frame's height times the picture's width differ by at most the longest
    /// of the four sides. When one of the two sizes is the other scaled, each
    /// side rounded to the nearest pixel, the products differ by at most half
    /// the sum of the other size's two sides, which is within the longest
    /// side: a 640 by 427 frame, 3:2 rounded, against a 6000 by 4000 picture
    /// differs by 2,000, within 6,000. A picture cut to another shape, or
    /// turned upright in its pixels, does not pass. Both sizes are in the
    /// stored orientation: Orientation turns neither.
    ///
    /// The products are of two `u32`s, which a `u64` always holds, so nothing
    /// here can overflow.
    ///
    /// The tables of a body's points hold the window of each JPEG size they
    /// list to this rule against the size, in their tests, so that the
    /// points placed in such a window are kept.
    pub(crate) fn same_shape(self, picture: Size) -> bool {
        let [fw, fh, pw, ph] =
            [self.width, self.height, picture.width, picture.height].map(u64::from);
        let longest = fw.max(fh).max(pw).max(ph);
        (fw * ph).abs_diff(fh * pw) <= longest
    }
}

impl fmt::Display for Size {
    /// Writes the width, a plain `x` and the height, with no spaces:
    /// `8256x5504`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// A position on an image, in pixels, measured from its edges: `x` is the
/// distance from the left edge and `y` the distance from the top edge of
/// the image as stored in the file, before any
/// [`Orientation`](crate::Orientation) is applied. So the middle of an
/// image `width` pixels wide is at `x = width / 2`, and the middle of one
/// `height` pixels high at `y = height / 2`.
///
/// Points order by `x`, then by `y`, so that they can key a map or be
/// sorted; that order is only a convention, and says nothing more about
/// where two points lie on the picture.
///
/// As text ([`Display`](fmt::Display)), a point is its `x` and its `y` in
/// parentheses, `x` first: `(4128, 2752)`.
///
/// Its fields are public because it is a small fixed value: a distance
/// across and a distance down, nothing more. There is no `Default`, since
/// (0, 0) is a real place, the top-left corner, which a default would
/// silently choose.
///
/// # Examples
///
/// The centre of a stored image, half its width from the left edge and
/// half its height from the top:
///
/// ```
/// use raw_files::{Point, Size};
///
/// let stored = Size { width: 8256, height: 5504 };
/// let centre = Point { x: stored.width / 2, y: stored.height / 2 };
/// assert_eq!(centre, Point { x: 4128, y: 2752 });
/// assert_eq!(centre.to_string(), "(4128, 2752)");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    /// The distance in pixels from the left edge, as stored.
    pub x: u32,
    /// The distance in pixels from the top edge, as stored.
    pub y: u32,
}

impl fmt::Display for Point {
    /// Writes `x`, then `y`, in parentheses: `(4128, 2752)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::size;

    #[test]
    fn a_frame_of_the_pictures_shape_has_its_shape() {
        for (frame, picture) in [
            // The same size, and a frame twice the picture's.
            (size(8256, 5504), size(8256, 5504)),
            (size(8256, 5504), size(4128, 2752)),
            // A 3:2 frame rounded to 640 by 427: 640 x 4000 and 427 x 6000
            // differ by 2,000, within the longest side, 6,000, which a
            // tolerance of the frame's longer side, 640, would refuse; and
            // the other way round.
            (size(640, 427), size(6000, 4000)),
            (size(6000, 4000), size(640, 427)),
            // A 1:1 window of a Z 9's frame, on a JPEG of its middle size.
            (size(5504, 5504), size(4128, 4128)),
            // 3 x 3 and 2 x 3 differ by 3, exactly the longest side.
            (size(3, 2), size(3, 3)),
        ] {
            assert!(frame.same_shape(picture), "{frame:?}, {picture:?}");
        }
    }

    #[test]
    fn a_frame_of_another_shape_has_not_the_pictures() {
        for (frame, picture) in [
            // A landscape frame on a portrait picture, as on an export
            // turned upright in its pixels or cut to portrait.
            (size(8256, 5504), size(5504, 8256)),
            (size(8256, 5504), size(3648, 5472)),
            // A landscape frame on a square export.
            (size(8256, 5504), size(1742, 1742)),
            // 7 x 4 and 5 x 4 differ by 8, one more than the longest side.
            (size(7, 5), size(4, 4)),
        ] {
            assert!(!frame.same_shape(picture), "{frame:?}, {picture:?}");
        }
    }

    #[test]
    fn the_largest_sides_do_not_overflow() {
        let largest = size(u32::MAX, u32::MAX);
        assert!(largest.same_shape(largest));
        assert!(!largest.same_shape(size(1, u32::MAX)));
    }

    #[test]
    fn construction_and_equality() {
        let size = Size {
            width: 640,
            height: 480,
        };
        assert_eq!(size.width, 640);
        assert_eq!(size.height, 480);
        assert_eq!(
            size,
            Size {
                width: 640,
                height: 480
            }
        );
        assert_ne!(
            size,
            Size {
                width: 480,
                height: 640
            }
        );
    }

    #[test]
    fn area_is_the_number_of_pixels() {
        let size = |width, height| Size { width, height };
        assert_eq!(size(640, 480).area(), 307_200);
        assert_eq!(size(0, 480).area(), 0);
        // 65537 × 65537 is past 2^32: computed in u32 it would wrap to
        // 131073.
        assert_eq!(size(65_537, 65_537).area(), 4_295_098_369);
        assert_eq!(size(u32::MAX, u32::MAX).area(), 18_446_744_065_119_617_025);
    }

    #[test]
    fn point_construction_equality_and_order() {
        let point = Point { x: 3, y: 7 };
        assert_eq!(point.x, 3);
        assert_eq!(point.y, 7);
        assert_eq!(point, Point { x: 3, y: 7 });
        assert_ne!(point, Point { x: 7, y: 3 });

        // By x first, whatever y...
        assert!(Point { x: 1, y: 9 } < Point { x: 2, y: 0 });
        // ...then by y for the same x.
        assert!(Point { x: 1, y: 2 } < Point { x: 1, y: 3 });
    }

    #[test]
    fn size_prints_width_x_height() {
        let size = Size {
            width: 8256,
            height: 5504,
        };
        assert_eq!(size.to_string(), "8256x5504");
        let zero = Size {
            width: 0,
            height: 0,
        };
        assert_eq!(zero.to_string(), "0x0");
        let largest = Size {
            width: u32::MAX,
            height: 1,
        };
        assert_eq!(largest.to_string(), "4294967295x1");
    }

    #[test]
    fn point_prints_x_then_y_in_parentheses() {
        assert_eq!(Point { x: 4905, y: 2461 }.to_string(), "(4905, 2461)");
        assert_eq!(Point { x: 0, y: 0 }.to_string(), "(0, 0)");
        assert_eq!(Point { x: 1, y: u32::MAX }.to_string(), "(1, 4294967295)");
    }
}
