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
