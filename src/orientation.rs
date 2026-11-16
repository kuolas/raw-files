// SPDX-License-Identifier: MIT OR Apache-2.0

//! Image orientation: how a viewer turns the stored pixels upright.

use std::fmt;

/// What a viewer must do to the stored pixels to show the image upright, as
/// the camera recorded it from its orientation sensor.
///
/// The stored preview is not rotated or flipped: the viewer applies this
/// itself. The variants are the eight values of the Exif `Orientation` tag,
/// numbered 1 to 8 in the variant docs, and carry the same names as the
/// `image` crate uses, so an application using it can map one to one.
///
/// `Orientation` is not `#[non_exhaustive]`: Exif defines exactly these eight
/// values. It orders as its variants are listed, by their Exif number, so
/// that orientations can key a map or be sorted.
///
/// As text, [`name`](Self::name) and [`Display`](fmt::Display), an
/// orientation says what a viewer does to the stored pixels, for an
/// information panel: "Rotate 90 degrees clockwise". `Debug` gives the
/// variant's name, `Rotate90`.
///
/// # Rotation and mirror
///
/// Each orientation is a turn clockwise, then a mirror left to right or
/// none, in that order. [`rotation_degrees`](Self::rotation_degrees) gives
/// the turn and [`is_mirrored`](Self::is_mirrored) the mirror:
///
/// | Variant | Exif | Rotation | Mirrored |
/// |---|---|---|---|
/// | [`NoTransforms`](Self::NoTransforms) | 1 | 0 | no |
/// | [`FlipHorizontal`](Self::FlipHorizontal) | 2 | 0 | yes |
/// | [`Rotate180`](Self::Rotate180) | 3 | 180 | no |
/// | [`FlipVertical`](Self::FlipVertical) | 4 | 180 | yes |
/// | [`Rotate90FlipH`](Self::Rotate90FlipH) | 5 | 90 | yes |
/// | [`Rotate90`](Self::Rotate90) | 6 | 90 | no |
/// | [`Rotate270FlipH`](Self::Rotate270FlipH) | 7 | 270 | yes |
/// | [`Rotate270`](Self::Rotate270) | 8 | 270 | no |
///
/// A mirror top to bottom, `FlipVertical`, is a turn of 180 degrees and a
/// mirror left to right. With the two parts, a program in another language
/// says what a viewer does without matching the eight variants, here in
/// French:
///
/// ```
/// use raw_files::Orientation;
///
/// fn en_francais(orientation: Orientation) -> String {
///     let turn = match orientation.rotation_degrees() {
///         0 => None,
///         degrees => Some(format!("pivoter de {degrees} degrés dans le sens horaire")),
///     };
///     let mirror = orientation.is_mirrored().then_some("retourner de gauche à droite");
///     match (turn, mirror) {
///         (None, None) => "aucun changement".to_owned(),
///         (Some(turn), None) => turn,
///         (None, Some(mirror)) => mirror.to_owned(),
///         (Some(turn), Some(mirror)) => format!("{turn}, puis {mirror}"),
///     }
/// }
///
/// assert_eq!(
///     en_francais(Orientation::Rotate90FlipH),
///     "pivoter de 90 degrés dans le sens horaire, puis retourner de gauche à droite"
/// );
/// assert_eq!(en_francais(Orientation::NoTransforms), "aucun changement");
/// ```
///
/// # Examples
///
/// ```
/// use raw_files::Orientation;
///
/// # fn main() {
/// #     example().expect("6 is an Exif orientation");
/// # }
/// # fn example() -> Option<()> {
/// let orientation = Orientation::from_exif(6)?;
/// assert_eq!(orientation, Orientation::Rotate90);
/// assert!(orientation.swaps_dimensions());
/// assert_eq!(orientation.to_string(), "Rotate 90 degrees clockwise");
/// // Exif defines no orientation 9.
/// assert_eq!(Orientation::from_exif(9), None);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Orientation {
    /// Apply nothing: the stored pixels are upright. Exif 1.
    #[default]
    NoTransforms,
    /// Mirror the image left to right. Exif 2.
    FlipHorizontal,
    /// Rotate the image by 180 degrees. Exif 3.
    Rotate180,
    /// Mirror the image top to bottom. Exif 4.
    FlipVertical,
    /// Rotate the image 90 degrees clockwise, then mirror it left to right.
    /// Exif 5.
    Rotate90FlipH,
    /// Rotate the image 90 degrees clockwise. Exif 6.
    Rotate90,
    /// Rotate the image 270 degrees clockwise, then mirror it left to right.
    /// Exif 7.
    Rotate270FlipH,
    /// Rotate the image 270 degrees clockwise (90 degrees counterclockwise).
    /// Exif 8.
    Rotate270,
}

impl Orientation {
    /// The orientation for an Exif `Orientation` tag value.
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

    /// The Exif `Orientation` tag value for this orientation, from 1 to 8.
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

    /// What a viewer does to the stored pixels to show the image upright,
    /// in a person's words, for an information panel: "No change",
    /// "Mirror left to right", "Rotate 90 degrees clockwise" or "Rotate 90
    /// degrees counterclockwise, then mirror left to right".
    /// [`Display`](fmt::Display) writes the same.
    pub fn name(self) -> &'static str {
        match self {
            Self::NoTransforms => "No change",
            Self::FlipHorizontal => "Mirror left to right",
            Self::Rotate180 => "Rotate 180 degrees",
            Self::FlipVertical => "Mirror top to bottom",
            Self::Rotate90FlipH => "Rotate 90 degrees clockwise, then mirror left to right",
            Self::Rotate90 => "Rotate 90 degrees clockwise",
            Self::Rotate270FlipH => "Rotate 90 degrees counterclockwise, then mirror left to right",
            Self::Rotate270 => "Rotate 90 degrees counterclockwise",
        }
    }

    /// Whether applying this orientation exchanges the width and height.
    ///
    /// When true (Exif 5 to 8), the displayed width is the stored height and
    /// the displayed height is the stored width. This matters when choosing a
    /// preview that fits a box.
    pub fn swaps_dimensions(self) -> bool {
        matches!(self.rotation_degrees(), 90 | 270)
    }

    /// How far a viewer turns the stored pixels clockwise, in degrees: 0,
    /// 90, 180 or 270. The viewer turns them first, then mirrors them when
    /// [`is_mirrored`](Self::is_mirrored) says so, as the
    /// [table of the type](Self#rotation-and-mirror) shows.
    #[must_use]
    pub fn rotation_degrees(self) -> u16 {
        match self {
            Self::NoTransforms | Self::FlipHorizontal => 0,
            Self::Rotate90FlipH | Self::Rotate90 => 90,
            Self::Rotate180 | Self::FlipVertical => 180,
            Self::Rotate270FlipH | Self::Rotate270 => 270,
        }
    }

    /// Whether a viewer mirrors the stored pixels left to right, after
    /// turning them by [`rotation_degrees`](Self::rotation_degrees), as the
    /// [table of the type](Self#rotation-and-mirror) shows.
    #[must_use]
    pub fn is_mirrored(self) -> bool {
        matches!(
            self,
            Self::FlipHorizontal | Self::FlipVertical | Self::Rotate90FlipH | Self::Rotate270FlipH
        )
    }
}

impl fmt::Display for Orientation {
    /// Writes [`name`](Orientation::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_names, assert_ordered};

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
            assert_eq!(orientation.swaps_dimensions(), n >= 5, "Exif {n}");
        }
    }

    #[test]
    fn every_orientation_name_is_the_one_documented_and_display_writes_it() {
        // The names are stable once released: a program may show or store
        // them, so a changed one fails here. `TABLE` lists every variant,
        // in Exif order, which is their declaration order.
        let all = TABLE.map(|(_, orientation)| orientation);
        assert_ordered(&all);
        assert_names(
            &all,
            Orientation::name,
            &[
                "No change",
                "Mirror left to right",
                "Rotate 180 degrees",
                "Mirror top to bottom",
                "Rotate 90 degrees clockwise, then mirror left to right",
                "Rotate 90 degrees clockwise",
                "Rotate 90 degrees counterclockwise, then mirror left to right",
                "Rotate 90 degrees counterclockwise",
            ],
        );
    }

    #[test]
    fn default_is_no_transforms() {
        assert_eq!(Orientation::default(), Orientation::NoTransforms);
    }

    #[test]
    fn each_orientation_gives_the_rotation_and_mirror_of_the_table() {
        let table = [
            (Orientation::NoTransforms, 0, false),
            (Orientation::FlipHorizontal, 0, true),
            (Orientation::Rotate180, 180, false),
            (Orientation::FlipVertical, 180, true),
            (Orientation::Rotate90FlipH, 90, true),
            (Orientation::Rotate90, 90, false),
            (Orientation::Rotate270FlipH, 270, true),
            (Orientation::Rotate270, 270, false),
        ];
        for (orientation, degrees, mirrored) in table {
            assert_eq!(orientation.rotation_degrees(), degrees, "{orientation:?}");
            assert_eq!(orientation.is_mirrored(), mirrored, "{orientation:?}");
            assert_eq!(
                orientation.swaps_dimensions(),
                degrees == 90 || degrees == 270,
                "{orientation:?}"
            );
        }
    }

    /// A side of the picture as a viewer shows it.
    #[derive(Debug, PartialEq)]
    enum Side {
        Top,
        Bottom,
        Left,
        Right,
    }

    /// The grid a viewer shows from `stored`, a grid of the stored pixels'
    /// places: turned clockwise by the rotation, then mirrored left to
    /// right when the orientation says so.
    fn shown(orientation: Orientation, stored: &[Vec<(usize, usize)>]) -> Vec<Vec<(usize, usize)>> {
        let mut grid = stored.to_vec();
        for _ in 0..orientation.rotation_degrees() / 90 {
            // A quarter turn clockwise: the first column, read from the
            // bottom up, becomes the first row.
            let rows = grid.len();
            let columns = grid[0].len();
            grid = (0..columns)
                .map(|column| (0..rows).rev().map(|row| grid[row][column]).collect())
                .collect();
        }
        if orientation.is_mirrored() {
            for row in &mut grid {
                row.reverse();
            }
        }
        grid
    }

    /// The side of `grid` along which every place lies that `stored`
    /// accepts, and no other place does.
    fn side_of(grid: &[Vec<(usize, usize)>], stored: impl Fn((usize, usize)) -> bool) -> Side {
        let last_row = grid.len() - 1;
        let last_column = grid[0].len() - 1;
        let on_side = |side: &Side, row: usize, column: usize| match side {
            Side::Top => row == 0,
            Side::Bottom => row == last_row,
            Side::Left => column == 0,
            Side::Right => column == last_column,
        };
        [Side::Top, Side::Bottom, Side::Left, Side::Right]
            .into_iter()
            .find(|side| {
                grid.iter().enumerate().all(|(row, places)| {
                    places
                        .iter()
                        .enumerate()
                        .all(|(column, &place)| stored(place) == on_side(side, row, column))
                })
            })
            .expect("the places lie along one side")
    }

    #[test]
    fn the_rotation_then_the_mirror_put_row_0_and_column_0_where_exif_says() {
        // CIPA DC-008 (Exif), the Orientation tag, names for each value the
        // side of the shown picture on which the stored picture's 0th row
        // and 0th column lie.
        let exif = [
            (1, Side::Top, Side::Left),
            (2, Side::Top, Side::Right),
            (3, Side::Bottom, Side::Right),
            (4, Side::Bottom, Side::Left),
            (5, Side::Left, Side::Top),
            (6, Side::Right, Side::Top),
            (7, Side::Right, Side::Bottom),
            (8, Side::Left, Side::Bottom),
        ];
        // Two rows of three, so that a row cannot pass for a column.
        let stored: Vec<Vec<(usize, usize)>> = (0..2)
            .map(|row| (0..3).map(|column| (row, column)).collect())
            .collect();
        for (number, row_0, column_0) in exif {
            let orientation = Orientation::from_exif(number).unwrap();
            let grid = shown(orientation, &stored);
            assert_eq!(
                side_of(&grid, |(row, _)| row == 0),
                row_0,
                "Exif {number}, row 0"
            );
            assert_eq!(
                side_of(&grid, |(_, column)| column == 0),
                column_0,
                "Exif {number}, column 0"
            );
            assert_eq!(
                grid.len() == 3,
                orientation.swaps_dimensions(),
                "Exif {number}"
            );
        }
    }
}
