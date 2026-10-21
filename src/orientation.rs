// SPDX-License-Identifier: MIT OR Apache-2.0

//! Image orientation: how a viewer turns the stored pixels upright.

/// What a viewer must do to the stored pixels to show the image upright, as
/// the camera recorded it from its orientation sensor.
///
/// The stored preview is not rotated or flipped: the viewer applies this
/// itself. The variants are the eight values of the EXIF `Orientation` tag,
/// numbered 1 to 8 in the variant docs, and carry the same names as the
/// `image` crate uses, so an application using it can map one to one.
///
/// `Orientation` is not `#[non_exhaustive]`: EXIF defines exactly these eight
/// values. It orders as its variants are listed, by their EXIF number, so
/// that orientations can key a map or be sorted.
///
/// # Examples
///
/// ```
/// use raw_files::Orientation;
///
/// # fn main() {
/// #     example().expect("6 is an EXIF orientation");
/// # }
/// # fn example() -> Option<()> {
/// let orientation = Orientation::from_exif(6)?;
/// assert_eq!(orientation, Orientation::Rotate90);
/// assert!(orientation.swaps_dimensions());
/// // EXIF defines no orientation 9.
/// assert_eq!(Orientation::from_exif(9), None);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Orientation {
    /// Apply nothing: the stored pixels are upright. EXIF 1.
    #[default]
    NoTransforms,
    /// Mirror the image left to right. EXIF 2.
    FlipHorizontal,
    /// Rotate the image by 180 degrees. EXIF 3.
    Rotate180,
    /// Mirror the image top to bottom. EXIF 4.
    FlipVertical,
    /// Rotate the image 90 degrees clockwise, then mirror it left to right.
    /// EXIF 5.
    Rotate90FlipH,
    /// Rotate the image 90 degrees clockwise. EXIF 6.
    Rotate90,
    /// Rotate the image 270 degrees clockwise, then mirror it left to right.
    /// EXIF 7.
    Rotate270FlipH,
    /// Rotate the image 270 degrees clockwise (90 degrees counterclockwise).
    /// EXIF 8.
    Rotate270,
}

impl Orientation {
    /// The orientation for an EXIF `Orientation` tag value.
    ///
    /// Returns `None` for 0 and for 9 and above, which real files do contain;
    /// 0 is common for "unknown".
    #[must_use]
    pub fn from_exif(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::NoTransforms),
            2 => Some(Self::FlipHorizontal),
            3 => Some(Self::Rotate180),
            4 => Some(Self::FlipVertical),
            5 => Some(Self::Rotate90FlipH),
            6 => Some(Self::Rotate90),
            7 => Some(Self::Rotate270FlipH),
            8 => Some(Self::Rotate270),
            _ => None,
        }
    }

    /// The EXIF `Orientation` tag value for this orientation, from 1 to 8.
    pub fn to_exif(self) -> u16 {
        match self {
            Self::NoTransforms => 1,
            Self::FlipHorizontal => 2,
            Self::Rotate180 => 3,
            Self::FlipVertical => 4,
            Self::Rotate90FlipH => 5,
            Self::Rotate90 => 6,
            Self::Rotate270FlipH => 7,
            Self::Rotate270 => 8,
        }
    }

    /// Whether applying this orientation exchanges the width and height.
    ///
    /// When true (EXIF 5 to 8), the displayed width is the stored height and
    /// the displayed height is the stored width. This matters when choosing a
    /// preview that fits a box.
    pub fn swaps_dimensions(self) -> bool {
        matches!(
            self,
            Self::Rotate90FlipH | Self::Rotate90 | Self::Rotate270FlipH | Self::Rotate270
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: [(u16, Orientation); 8] = [
        (1, Orientation::NoTransforms),
        (2, Orientation::FlipHorizontal),
        (3, Orientation::Rotate180),
        (4, Orientation::FlipVertical),
        (5, Orientation::Rotate90FlipH),
        (6, Orientation::Rotate90),
        (7, Orientation::Rotate270FlipH),
        (8, Orientation::Rotate270),
    ];

    #[test]
    fn each_number_maps_to_its_variant() {
        for (number, orientation) in TABLE {
            assert_eq!(Orientation::from_exif(number), Some(orientation));
            assert_eq!(orientation.to_exif(), number);
        }
    }

    #[test]
    fn round_trip() {
        for n in 1..=8 {
            assert_eq!(Orientation::from_exif(n).unwrap().to_exif(), n);
        }
    }

    #[test]
    fn out_of_range_values_are_none() {
        for n in [0, 9, 255, u16::MAX] {
            assert_eq!(Orientation::from_exif(n), None);
        }
    }

    #[test]
    fn only_the_quarter_turns_swap_dimensions() {
        for n in 1..=8 {
            let orientation = Orientation::from_exif(n).unwrap();
            assert_eq!(orientation.swaps_dimensions(), n >= 5, "EXIF {n}");
        }
    }

    #[test]
    fn default_is_no_transforms() {
        assert_eq!(Orientation::default(), Orientation::NoTransforms);
    }
}
