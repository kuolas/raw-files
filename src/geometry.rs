// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sizes in pixels.

/// The size of an image in pixels, as stored in the file, before any
/// [`Orientation`](crate::Orientation) is applied.
///
/// For an orientation that swaps the dimensions, the size a viewer shows is
/// this size with its width and height exchanged.
///
/// Sizes order by width, then by height, so that they can key a map or be
/// sorted; to compare how large two pictures are, compare their areas.
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
}
