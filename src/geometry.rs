// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sizes and positions in pixels.

/// The size of an image in pixels, as stored in the file, before any
/// [`Orientation`](crate::Orientation) is applied.
///
/// For an orientation that swaps the dimensions, the size a viewer shows is
/// this size with its width and height exchanged.
///
/// Sizes order by width, then by height, so that they can key a map or be
/// sorted; to compare how large two pictures are, compare their areas.
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
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Size {
    /// The number of pixel columns, as stored.
    pub width: u32,
    /// The number of pixel rows, as stored.
    pub height: u32,
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
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    /// The distance in pixels from the left edge, as stored.
    pub x: u32,
    /// The distance in pixels from the top edge, as stored.
    pub y: u32,
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
}
