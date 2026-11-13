// SPDX-License-Identifier: MIT OR Apache-2.0

//! The tables of a body's autofocus points: where each autofocus point of
//! a body lies on its picture, which no file the body writes records.
//!
//! A Nikon DSLR shot through the viewfinder names the point the camera
//! used by its number, and records no position for it. A table gives, for
//! one body, each point's number, the center of its box and the box's
//! size, in the body's full-size output frame. The parts:
//!
//! - [`find`] finds a body's table by the file's Model alone;
//! - [`layout`] gives the table as the layout of the body's points
//!   ([`FocusLayout`]), which detection gives every file of that Model,
//!   whatever the shot and the file's format;
//! - [`PointTable`] holds one body's points, with what its record confirms
//!   of the way a file names them: the layout code, the mask's bit order
//!   ([`BitOrder`]), the crops ([`ConfirmedCrop`]) and the JPEG sizes
//!   ([`ConfirmedJpegSize`]). The Nikon reader places a named point with
//!   them.
//!
//! Every table is measured by the project, or by a contributor, on frames
//! shot with the body itself, as the
//! [decision to measure per-body tables in the project](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)
//! (ADR017) sets out. Its values are written from the body's record in the
//! NEF notes alone: the readings rounded to the pixel, not a grid fitted
//! to them, and nothing from another project's table. Each table carries
//! its [`Provenance`].
//!
//! Each body's table is a file of its own in this folder, named after the
//! body's Model in lower case with its spaces as underscores, such as
//! `nikon_d600.rs` for `NIKON D600`. It is registered by hand in
//! [`TABLES`], and a test fails on a file here that is not.
//!
//! One Model can carry two layouts, each under a layout code of its own:
//! ExifTool's published tag documentation, read as documentation, lists
//! two for the D780. So placing a named point takes the file's layout code
//! too, as the
//! [decision to place focus points from a table](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)
//! (ADR018) says.
//!
//! A body whose uncropped files record a CropHiSpeed cropped size other
//! than the original, as the D810's do, would have those files read as a
//! crop when a named point is placed. Check the body's uncropped
//! CropHiSpeed before adding its table.
//!
//! The NEF notes hold each body's record, with the procedure, the readings
//! and their tolerance (`docs/formats/nef/autofocus-points.md`, sections 1
//! and 2).

use crate::focus::{FocusLayout, FocusLayoutPoint};
use crate::geometry::Size;

mod nikon_d600;

/// Every table in raw-files, one per body, registered by hand: a table's
/// file in this folder that is not listed here fails the tests.
static TABLES: [&PointTable; 1] = [&nikon_d600::TABLE];

/// How long the mask of the points used is, in bytes: the seven bytes 8 to
/// 14 of AFInfo2 version 0100, which a shot through a DSLR's viewfinder
/// writes beside the number of the point it names. Every bit a table's
/// [`BitOrder`] gives one of its points lies in these bytes, which the
/// table tests check.
pub(in crate::formats) const MASK_LEN: usize = 7;

/// How a file's mask of the points used names a table's points: which bit
/// of the mask stands for which point. The bits are counted from the least
/// significant bit of the mask's first byte, so that bit k is bit k % 8 of
/// byte k / 8.
///
/// It is a fact of the body's layout, read off the single-point frames of
/// the body's record, as the procedure in the NEF notes says. No file
/// names its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::formats) enum BitOrder {
    /// Point n is bit n − 1: point 1 is the least significant bit of the
    /// first byte, point 9 the least significant of the second. The D600's
    /// order (`docs/formats/nef/autofocus-points.md`, section 2.3).
    NumberLessOne,
}

impl BitOrder {
    /// The bit that stands for point `number`, counted as the type's
    /// documentation says. `None` for a number no bit stands for, such as 0.
    /// Whether the bit lies in the mask is the reader's to check.
    fn bit(self, number: u16) -> Option<usize> {
        match self {
            BitOrder::NumberLessOne => usize::from(number).checked_sub(1),
        }
    }
}

/// One body's table of autofocus points: the body's Model, the frame the
/// points are measured in and every point, with what the body's record
/// confirms of the way a file names them, and where the values come from.
#[derive(Debug)]
pub(in crate::formats) struct PointTable {
    /// The body's Model, as the camera writes it in the Model tag, trimmed
    /// of padding as
    /// [`Camera::recorded_model`](crate::Camera::recorded_model) gives it,
    /// such as `NIKON D600`: what [`find`] matches a file's Model against.
    recorded_model: &'static str,
    /// The frame every point is measured in: the body's full-size output,
    /// the size of its full-size JPEG.
    frame: Size,
    /// Every point, in number order, from 1: its number as the camera
    /// writes it, the center of its box and the box's size, in `frame`.
    points: &'static [FocusLayoutPoint],
    /// The layout code the body writes beside a point number of this
    /// layout, AFInfo2's FocusPointSchema. A file names a point of this
    /// table only under this code, since one Model can carry two layouts.
    layout_code: u8,
    /// How the mask of the points used names each point by a bit.
    bit_order: BitOrder,
    /// The crops the record's crop frames confirm, each with its window. A
    /// file on any other crop gives no placed point.
    crops: &'static [ConfirmedCrop],
    /// The sizes of the body's JPEGs that show a window of the frame rather
    /// than the whole of it, each with its window. A JPEG of any other size
    /// shows the whole frame.
    jpeg_sizes: &'static [ConfirmedJpegSize],
    /// Where the table's values come from.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the provenance is a record for readers of the table and for its tests, \
                      never read by the reader of a file"
        )
    )]
    provenance: Provenance,
}

/// A crop that a body's record confirms a named point can be placed on.
/// It holds the crop's setting, as CropHiSpeed's first value records it,
/// and the window a point placed on the crop is measured in: the size of
/// the crop's picture.
///
/// The window is measured on the record's crop frames, as the size of
/// their full-size previews and JPEGs, and lies centered in the table's
/// frame. No tag gives it: CropHiSpeed's cropped size counts the sensor's
/// pixels, the border of the stored raw data among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::formats) struct ConfirmedCrop {
    /// The crop setting, CropHiSpeed's first value.
    code: u32,
    /// The size of the crop's picture, in the pixels of the table's frame.
    window: Size,
}

impl ConfirmedCrop {
    /// The crop of setting `code` whose picture is `window`, for tests
    /// that place points in a window no body has.
    #[cfg(test)]
    pub(in crate::formats) const fn new(code: u32, window: Size) -> ConfirmedCrop {
        ConfirmedCrop { code, window }
    }
}

/// A size of the body's uncropped JPEGs whose picture is not the whole
/// frame scaled: the JPEG's size, as its frame header gives it, and the
/// window of the table's frame that its picture shows.
///
/// The window is measured on the record's RAW+JPEG pairs at that size, by
/// fitting each JPEG onto the full-size preview of its own raw file, and
/// lies centered in the table's frame. No tag gives it, and the JPEG's
/// shape alone does not: the D600's Medium JPEG shows a window a few pixels
/// smaller than its frame (`docs/formats/nef/autofocus-points.md`, section
/// 2.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::formats) struct ConfirmedJpegSize {
    /// The JPEG's size, as its frame header gives it.
    size: Size,
    /// The window of the frame its picture shows, in the pixels of the
    /// table's frame.
    window: Size,
}

impl ConfirmedJpegSize {
    /// The JPEG size `size` whose picture shows `window`, for tests that
    /// place points in a window no body has.
    #[cfg(test)]
    pub(in crate::formats) const fn new(size: Size, window: Size) -> ConfirmedJpegSize {
        ConfirmedJpegSize { size, window }
    }
}

/// Where a table's values come from, as the body's record in the NEF notes
/// gives it: a record for readers of the table. The table tests check that
/// it is filled in and that its frames are committed.
#[derive(Debug)]
#[cfg_attr(test, derive(Clone, Copy))]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the provenance is a record for readers of the table and for its tests, \
                  never read by the reader of a file"
    )
)]
struct Provenance {
    /// Who shot the frames the table is measured on.
    shot_by: &'static str,
    /// Who measured them.
    measured_by: &'static str,
    /// When they were measured, as a date, `YYYY-MM-DD`.
    measured_on: &'static str,
    /// The maker's software the frames were read in, and its version.
    software: &'static str,
    /// Which frames of the session the table is read from, and which
    /// calibrate the reading, by their numbers in the record.
    frames: &'static str,
    /// The folder in `tests/fixtures/` that holds the session's frames,
    /// such as `nikon-d600/`.
    fixtures: &'static str,
}

impl PointTable {
    /// The layout of the table's points, as
    /// [`Metadata::focus_layout`](crate::Metadata::focus_layout) gives it,
    /// through the checked [`FocusLayout::new`].
    ///
    /// It is `Some` for every registered table: the table tests check more
    /// than [`FocusLayout::new`] does. A table that failed them would give a
    /// file no layout ([`layout`]), never a panic.
    fn layout(&'static self) -> Option<FocusLayout> {
        FocusLayout::new(self.recorded_model, self.frame, self.points).ok()
    }

    /// The frame every point is measured in: the body's full-size output.
    pub(in crate::formats) fn frame(&self) -> Size {
        self.frame
    }

    /// Every point, in number order, from 1, as the table tests check.
    pub(in crate::formats) fn points(&self) -> &'static [FocusLayoutPoint] {
        self.points
    }

    /// The layout code a file writes beside a point number of this table.
    pub(in crate::formats) fn layout_code(&self) -> u8 {
        self.layout_code
    }

    /// The bit of the mask of the points used that stands for point
    /// `number`, by the table's [`BitOrder`]. `None` for a number no bit
    /// stands for. The table tests check that every point's bit lies inside
    /// the mask's [`MASK_LEN`] bytes.
    pub(in crate::formats) fn bit(&self, number: u16) -> Option<usize> {
        self.bit_order.bit(number)
    }

    /// The window of the crop whose setting, CropHiSpeed's first value, is
    /// `code`, in the pixels of the table's frame. `None` for a crop the
    /// record does not confirm. The table tests check that every window
    /// fits the frame and can be centered in it to the pixel.
    pub(in crate::formats) fn crop_window(&self, code: u32) -> Option<Size> {
        self.crops
            .iter()
            .find(|crop| crop.code == code)
            .map(|crop| crop.window)
    }

    /// The window of the frame that an uncropped JPEG of `size` shows, in
    /// the pixels of the table's frame. `None` for a size whose picture is
    /// the whole frame, or that the record did not measure. The table tests
    /// check that every window fits the frame, can be centered in it to the
    /// pixel, and has the shape of its JPEG size.
    pub(in crate::formats) fn jpeg_window(&self, size: Size) -> Option<Size> {
        self.jpeg_sizes
            .iter()
            .find(|jpeg| jpeg.size == size)
            .map(|jpeg| jpeg.window)
    }

    /// This table with `crops` as its confirmed crops, for tests that place
    /// points in a window no body has.
    #[cfg(test)]
    pub(in crate::formats) fn with_crops(&self, crops: &'static [ConfirmedCrop]) -> PointTable {
        PointTable { crops, ..*self }
    }

    /// This table with `jpeg_sizes` as its confirmed JPEG sizes, for tests
    /// that place points in a window no body has.
    #[cfg(test)]
    pub(in crate::formats) fn with_jpeg_sizes(
        &self,
        jpeg_sizes: &'static [ConfirmedJpegSize],
    ) -> PointTable {
        PointTable {
            jpeg_sizes,
            ..*self
        }
    }
}

/// The table of the body whose Model is `recorded_model`, matched exactly
/// against the table's Model, such as `NIKON D600`. `None` for a body
/// without a table. `recorded_model` is the file's Model as
/// [`Camera::recorded_model`](crate::Camera::recorded_model) gives it.
pub(in crate::formats) fn find(recorded_model: &str) -> Option<&'static PointTable> {
    TABLES
        .iter()
        .copied()
        .find(|table| table.recorded_model == recorded_model)
}

/// The layout of the autofocus points of the body whose Model is
/// `recorded_model`, as
/// [`Metadata::focus_layout`](crate::Metadata::focus_layout) gives it: its
/// table's ([`find`]). `None` for a body without a table.
pub(in crate::formats) fn layout(recorded_model: &str) -> Option<FocusLayout> {
    find(recorded_model).and_then(PointTable::layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Where the table files are, under the crate's root.
    const FOLDER: &str = "src/formats/makernote/points";

    /// The name of the file that holds the table of the body whose Model is
    /// `model`: the Model in lower case with its spaces as underscores, and
    /// `.rs`, such as `nikon_d600.rs` for `NIKON D600`.
    fn file_name(model: &str) -> String {
        format!("{}.rs", model.to_lowercase().replace(' ', "_"))
    }

    /// The names of the `.rs` files in [`FOLDER`] but `mod.rs`: the table
    /// files.
    fn table_files() -> BTreeSet<String> {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(FOLDER);
        let entries = std::fs::read_dir(&folder)
            .unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
        entries
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".rs") && name != "mod.rs")
            .collect()
    }

    /// Every table file in the folder is registered, and every registered
    /// table is in the file named after its Model: so a contributor's new
    /// table cannot sit in the folder unused, and a table cannot be filed
    /// under another body's name.
    #[test]
    fn every_table_file_is_registered_under_its_models_name() {
        let files = table_files();
        let registered: BTreeSet<String> = TABLES
            .iter()
            .map(|table| file_name(table.recorded_model))
            .collect();
        assert_eq!(
            registered.len(),
            TABLES.len(),
            "two registered tables have the same Model: one body has one table"
        );
        let unregistered: Vec<&String> = files.difference(&registered).collect();
        assert!(
            unregistered.is_empty(),
            "{unregistered:?} in {FOLDER} register no table: add a `mod` for each and its \
             `TABLE` to `TABLES` in {FOLDER}/mod.rs, and name each file after its table's \
             Model, in lower case with its spaces as underscores"
        );
        let missing: Vec<&String> = registered.difference(&files).collect();
        assert!(
            missing.is_empty(),
            "the registered tables' Models ask for {missing:?} in {FOLDER}, which do not \
             exist: name each table's file after its Model, in lower case with its spaces \
             as underscores"
        );
    }

    /// Every registered table can be a layout, by the checks of
    /// [`FocusLayout::new`]: a frame with no zero side, at least one point,
    /// every number above 0 and above the one before it, every center on
    /// the frame and every box inside it. So the reader, which gives `None`
    /// for a table that fails them, gives every registered body its layout.
    #[test]
    fn every_table_is_a_layout() {
        for table in TABLES {
            let model = table.recorded_model;
            assert!(
                table.frame.width > 0 && table.frame.height > 0,
                "{model}: a side of the frame is zero"
            );
            let layout = table
                .layout()
                .unwrap_or_else(|| panic!("{model}: the points are not a layout on the frame"));
            assert_eq!(layout.recorded_model(), model);
            assert_eq!(layout.frame(), table.frame);
            assert_eq!(layout.points(), table.points);
        }
    }

    /// Every table's points are numbered 1 to their count, in that order:
    /// the camera numbers a layout's points from 1 with no gap, and a point
    /// left out or out of order is a mistake in copying the record.
    #[test]
    fn every_table_numbers_its_points_from_1_in_order() {
        for table in TABLES {
            let numbers: Vec<usize> = table
                .points
                .iter()
                .map(|point| usize::from(point.number()))
                .collect();
            let expected: Vec<usize> = (1..=table.points.len()).collect();
            assert_eq!(
                numbers, expected,
                "{}: the points are not numbered 1 to their count, in order",
                table.recorded_model
            );
        }
    }

    /// Every table's point 1 lies within 3 pixels of its frame's center on
    /// each axis: Nikon numbers the center point 1 on every layout the NEF
    /// notes record, and a frame of the wrong size, or a table read in
    /// another frame, moves the center away from it.
    #[test]
    fn every_tables_point_1_lies_at_the_frames_center() {
        for table in TABLES {
            let model = table.recorded_model;
            let first = table
                .points
                .first()
                .unwrap_or_else(|| panic!("{model}: no points"));
            assert_eq!(first.number(), 1, "{model}: the first point is not 1");
            let center = first.center();
            // Twice the distance, so that an odd side's half pixel counts.
            let off_x = (2 * i64::from(center.x) - i64::from(table.frame.width)).abs();
            let off_y = (2 * i64::from(center.y) - i64::from(table.frame.height)).abs();
            assert!(
                off_x <= 6 && off_y <= 6,
                "{model}: point 1 at {center} is more than 3 pixels from the center of the \
                 {} frame",
                table.frame
            );
        }
    }

    /// How many points the layout of each layout code a table carries has,
    /// as ExifTool's published tag documentation names the layouts, read as
    /// documentation, and as the frames of each table's record confirm: 3,
    /// the 39-point layout, the D600's, whose 39 single-point frames name
    /// 39 points. A new table's code is added here from the same sources.
    const POINTS_BY_LAYOUT_CODE: [(u8, usize); 1] = [(3, 39)];

    /// Every table has as many points as its layout code's layout: a table
    /// that lost a point, or that is filed under another layout's code,
    /// would place the numbers a file names on the wrong layout.
    #[test]
    fn every_tables_point_count_is_its_layout_codes() {
        for table in TABLES {
            let model = table.recorded_model;
            let count = POINTS_BY_LAYOUT_CODE
                .iter()
                .find(|(code, _)| *code == table.layout_code)
                .map(|&(_, count)| count)
                .unwrap_or_else(|| {
                    panic!(
                        "{model}: no point count for layout code {}: add it to \
                         POINTS_BY_LAYOUT_CODE from the documentation and the frames",
                        table.layout_code
                    )
                });
            assert_eq!(
                table.points.len(),
                count,
                "{model}: layout code {} has {count} points",
                table.layout_code
            );
        }
    }

    /// Every point of every table has a bit of the mask of the points used
    /// of its own, inside the mask's [`MASK_LEN`] bytes: a point without
    /// one could never be read from a file's mask, and two points on one
    /// bit could not be told apart.
    #[test]
    fn every_tables_points_have_a_bit_each_inside_the_mask() {
        for table in TABLES {
            let model = table.recorded_model;
            let mut bits = BTreeSet::new();
            for point in table.points {
                let number = point.number();
                let bit = table
                    .bit(number)
                    .unwrap_or_else(|| panic!("{model}: no bit for point {number}"));
                assert!(
                    bit < MASK_LEN * 8,
                    "{model}: point {number}'s bit {bit} lies past the {MASK_LEN} bytes of the mask"
                );
                assert!(
                    bits.insert(bit),
                    "{model}: point {number}'s bit {bit} stands for another point too"
                );
            }
        }
    }

    /// Every table's confirmed crops are each confirmed once, and each
    /// window has no zero side, fits the frame, and leaves an even margin,
    /// the frame less the window, on each axis: a window that does not fit
    /// could hold no point, and one with an odd margin cannot be centered
    /// to the pixel, so no file's CropHiSpeed could ever match it.
    #[test]
    fn every_tables_crop_windows_fit_its_frame_centered() {
        for table in TABLES {
            let model = table.recorded_model;
            let mut codes = BTreeSet::new();
            for crop in table.crops {
                let ConfirmedCrop { code, window } = *crop;
                assert!(codes.insert(code), "{model}: crop setting {code} twice");
                let what = format!("{model}: crop setting {code}'s window");
                assert_fits_centered(&what, table.frame, window);
            }
        }
    }

    /// Checks that `window`, which `what` names, has no zero side, fits
    /// `frame` and leaves an even margin, the frame less the window, on
    /// each axis, so that it can be centered to the pixel.
    fn assert_fits_centered(what: &str, frame: Size, window: Size) {
        assert!(
            window.width > 0 && window.height > 0,
            "{what} {window} has a zero side"
        );
        for (side, frame, window) in [
            ("width", frame.width, window.width),
            ("height", frame.height, window.height),
        ] {
            let margin = frame
                .checked_sub(window)
                .unwrap_or_else(|| panic!("{what} is longer than the frame in {side}"));
            assert!(
                margin % 2 == 0,
                "{what} leaves an odd margin in {side}, {margin} pixels, so it cannot be \
                 centered"
            );
        }
    }

    /// Every table's confirmed JPEG sizes are each confirmed once, and each
    /// window fits the frame centered, as a crop's window must; and has the
    /// shape of its JPEG size, by the rule the JPEG reader holds a JPEG's
    /// points to ([`Size::same_shape`]): a window of another shape would give
    /// points that the reader then refuses, so the size would gain
    /// nothing.
    #[test]
    fn every_tables_jpeg_windows_fit_its_frame_centered_in_their_jpegs_shape() {
        for table in TABLES {
            let model = table.recorded_model;
            let mut sizes = BTreeSet::new();
            for jpeg in table.jpeg_sizes {
                let ConfirmedJpegSize { size, window } = *jpeg;
                assert!(
                    sizes.insert((size.width, size.height)),
                    "{model}: JPEG size {size} twice"
                );
                let what = format!("{model}: JPEG size {size}'s window");
                assert_fits_centered(&what, table.frame, window);
                assert!(
                    window.same_shape(size),
                    "{what} {window} has not the JPEG's shape"
                );
            }
        }
    }

    /// The D600's table carries what its record confirms of the way a file
    /// names its points, by values written here from the record rather than
    /// read from the table: layout code 3; point 1 on bit 0, point 13 on
    /// bit 12 and point 38 on bit 37, the bits frames 20, 1 and 4 set; and
    /// the DX crop alone, setting 2, of the crops CropHiSpeed records, in
    /// the 3936 by 2624 window of frames 49 to 51's previews and JPEGs; and
    /// the Medium JPEG alone, 4512 by 3008, of the JPEG sizes, in the 6010
    /// by 4006 window that the RAW+JPEG pairs of section 2.9 measure: the
    /// Large and Small JPEGs show the whole frame, and a size a pixel off
    /// the Medium one, or the Medium size turned, is no size of the D600.
    #[test]
    fn the_d600_table_names_its_points_as_its_record_confirms() {
        let d600 = find("NIKON D600").expect("the D600's table");
        assert_eq!(d600.layout_code(), 3);
        for (number, bit) in [(1, 0), (13, 12), (38, 37)] {
            assert_eq!(d600.bit(number), Some(bit), "point {number}");
        }
        assert_eq!(d600.bit(0), None);
        assert_eq!(
            d600.crop_window(2),
            Some(Size {
                width: 3936,
                height: 2624
            })
        );
        for code in [0, 1, 6, 9, 10, 11, 12, 17, 18] {
            assert_eq!(d600.crop_window(code), None, "crop setting {code}");
        }
        let size = |width, height| Size { width, height };
        assert_eq!(d600.jpeg_window(size(4512, 3008)), Some(size(6010, 4006)));
        for (width, height) in [
            (6016, 4016),
            (3008, 2008),
            (4512, 3012),
            (4511, 3008),
            (3008, 4512),
        ] {
            assert_eq!(
                d600.jpeg_window(size(width, height)),
                None,
                "{width} by {height}"
            );
        }
    }

    /// Every table's provenance is filled in, and names a folder of
    /// `tests/fixtures/` that holds the session's frames as NEF files: a
    /// table whose origin cannot be read, or whose frames are missing, has
    /// nothing a reader could check it against.
    ///
    /// The frames are checked only in the repository: the packaged crate
    /// leaves `tests/fixtures/` out. The skip is keyed on `fuzz/Cargo.toml`,
    /// which the repository has and the packaged crate leaves out too:
    /// without it the frames are not checked, with a note. With it, every
    /// table's folder must exist and hold frames, so a renamed or deleted
    /// folder fails instead of silently skipping.
    #[test]
    fn every_tables_provenance_is_filled_in_and_its_frames_committed() {
        let fuzz_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/Cargo.toml");
        let in_repository = fuzz_manifest.is_file();
        if !in_repository {
            eprintln!(
                "frames skipped: {} does not exist, as in the packaged crate",
                fuzz_manifest.display()
            );
        }
        for table in TABLES {
            let model = table.recorded_model;
            let Provenance {
                shot_by,
                measured_by,
                measured_on,
                software,
                frames,
                fixtures,
            } = table.provenance;
            for (field, value) in [
                ("shot_by", shot_by),
                ("measured_by", measured_by),
                ("measured_on", measured_on),
                ("software", software),
                ("frames", frames),
                ("fixtures", fixtures),
            ] {
                assert!(
                    !value.trim().is_empty(),
                    "{model}: the provenance's {field} is empty"
                );
            }
            if !in_repository {
                continue;
            }
            let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(fixtures);
            let entries = std::fs::read_dir(&folder).unwrap_or_else(|error| {
                panic!(
                    "{model}: the provenance names the folder {fixtures}, which is not in \
                     tests/fixtures/: {error}"
                )
            });
            let nefs = entries
                .map(|entry| entry.unwrap().file_name())
                .filter(|name| name.to_string_lossy().ends_with(".nef"))
                .count();
            assert!(
                nefs > 0,
                "{model}: the folder {fixtures} in tests/fixtures/ holds no frames"
            );
        }
    }

    /// [`find`] gives a registered table for its Model, exactly as written,
    /// and none for another Model, for the same Model with other case or
    /// padding, which
    /// [`Camera::recorded_model`](crate::Camera::recorded_model) never
    /// gives, or for none at all.
    #[test]
    fn find_matches_the_model_exactly() {
        for table in TABLES {
            let found = find(table.recorded_model).map(|found| found.recorded_model);
            assert_eq!(found, Some(table.recorded_model));
        }
        let d600 = find("NIKON D600").expect("the D600's table");
        assert_eq!(
            d600.frame,
            Size {
                width: 6016,
                height: 4016
            }
        );
        for other in [
            "NIKON D610",
            "nikon d600",
            "NIKON D600 ",
            " NIKON D600",
            "NIKON",
            "",
        ] {
            assert!(find(other).is_none(), "{other:?}");
        }
    }
}
