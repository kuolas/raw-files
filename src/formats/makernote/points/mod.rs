// SPDX-License-Identifier: MIT OR Apache-2.0

//! The tables of a body's autofocus points: where each autofocus point of
//! a body lies on its picture, which no file the body writes records.
//!
//! A Nikon DSLR shot through the viewfinder names the autofocus point the
//! camera used by its number, and records no position for it. A table
//! gives, for one body, every point's number, the centre of the box the
//! maker's software draws for it and the box's size, all in the body's
//! full-size output frame. Every file whose Model names a body with a
//! table is given the table as the layout of the body's points, a
//! [`FocusLayout`] ([`layout`]), whatever the shot and whatever the
//! file's format. A body with one layout gives it from the Model alone,
//! so detection looks it up once, after the format's reader, and a file
//! whose MakerNote is missing or cannot be read has it too.
//!
//! Every table is measured by the project, or by a contributor, on frames
//! shot with the body itself and read in the maker's own software, run as
//! a reader to compare with, as the
//! [decision to measure per-body tables in the project](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)
//! (ADR017) sets out. The body's record in the NEF notes
//! (`docs/formats/nef/autofocus-points.md`) holds the readings, with their
//! method and their tolerance, and a table's values are written from that
//! record alone: the readings rounded to the pixel, not a grid fitted to
//! them, and nothing from another project's table. Each table carries its
//! [`Provenance`]: who shot the frames and who measured them, when, in
//! which software, on which frames, and the folder of fixtures that holds
//! the frames, committed so that the table can be checked on them.
//!
//! Each body's table is a file of its own in this folder, named after the
//! body's Model in lower case with its spaces as underscores, such as
//! `nikon_d600.rs` for `NIKON D600`, and registered by hand in
//! [`TABLES`]. A test fails on a file here that is not registered, and on
//! one whose name is not its table's Model; others check each table
//! against itself and against the frame it is measured in.
//!
//! A table is found by the file's Model alone ([`find`]), and its layout is
//! given for every file of that Model. One Model can carry two layouts,
//! though: ExifTool's published tag documentation, read as documentation,
//! lists two for the D780, 51 points through the viewfinder and 81 in live
//! view, each under a layout code of its own, which AFInfo2 records. So the
//! Model alone does not say which of a body's layouts a point that a file
//! names belongs to: placing that point takes the file's layout code too,
//! as the
//! [decision to place focus points from a table](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)
//! (ADR018) says. Nor does it say which layout such a body's file has:
//! a body whose files name one of several layouts will give the layout
//! that the file's layout code names, and none when the file names none,
//! as [`Metadata::focus_layout`](crate::Metadata::focus_layout) states;
//! only a body with one layout gives it from the Model alone.

use crate::focus::{FocusLayout, LayoutPoint};
use crate::geometry::Size;

mod nikon_d600;

/// Every table in raw-files, one per body, registered by hand: a table's
/// file in this folder that is not listed here fails the tests.
static TABLES: [&PointTable; 1] = [&nikon_d600::TABLE];

/// One body's table of autofocus points: the body's Model, the frame the
/// points are measured in and every point, with where the values come
/// from.
#[derive(Debug)]
pub(in crate::formats) struct PointTable {
    /// The body's Model, as the camera writes it in the Model tag, trimmed
    /// of padding as [`Metadata::model`](crate::Metadata::model) gives it,
    /// such as `NIKON D600`: what [`find`] matches a file's Model against.
    model: &'static str,
    /// The frame every point is measured in: the body's full-size output,
    /// the size of its full-size JPEG.
    frame: Size,
    /// Every point, in number order, from 1: its number as the camera
    /// writes it, the centre of its box and the box's size, in `frame`.
    points: &'static [LayoutPoint],
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

/// Where a table's values come from, as the body's record in the NEF notes
/// gives it: a record for readers of the table, which the table tests
/// check is filled in and whose frames they check are committed.
#[derive(Debug)]
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
    /// stripped of their pixels and personal fields, on which the tests
    /// check the table, such as `nikon-d600/`.
    fixtures: &'static str,
}

impl PointTable {
    /// The layout of the table's points, as
    /// [`Metadata::focus_layout`](crate::Metadata::focus_layout) gives it:
    /// the table's Model, frame and points, through the checked
    /// [`FocusLayout::new`].
    ///
    /// It is `Some` for every registered table, which the table tests
    /// check: a frame with no zero side, every number above 0 and given
    /// once, every centre and box on the frame, among checks that go
    /// further than [`FocusLayout::new`]'s, such as numbers that run from
    /// 1 to the count. So a table that fails them gives a file no layout
    /// ([`layout`]), never a panic.
    fn layout(&'static self) -> Option<FocusLayout> {
        FocusLayout::new(self.model, self.frame, self.points)
    }
}

/// The table of the body whose Model is `model`, matched exactly against
/// the table's Model, such as `NIKON D600`; `None` for a body without a
/// table. `model` is the file's Model as
/// [`Metadata::model`](crate::Metadata::model) gives it, trimmed of
/// padding.
pub(in crate::formats) fn find(model: &str) -> Option<&'static PointTable> {
    TABLES.iter().copied().find(|table| table.model == model)
}

/// The layout of the autofocus points of the body whose Model is `model`,
/// as [`Metadata::focus_layout`](crate::Metadata::focus_layout) gives it:
/// its table's ([`find`]); `None` for a body without a table. Detection
/// gives it to every file, whatever its format, from the Model alone.
pub(in crate::formats) fn layout(model: &str) -> Option<FocusLayout> {
    find(model).and_then(PointTable::layout)
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
        let registered: BTreeSet<String> =
            TABLES.iter().map(|table| file_name(table.model)).collect();
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
    /// every number above 0 and given once, every centre on the frame and
    /// every box inside it. So the reader, which gives `None` for a table
    /// that fails them, gives every registered body its layout.
    #[test]
    fn every_table_is_a_layout() {
        for table in TABLES {
            let model = table.model;
            assert!(
                table.frame.width > 0 && table.frame.height > 0,
                "{model}: a side of the frame is zero"
            );
            let layout = table
                .layout()
                .unwrap_or_else(|| panic!("{model}: the points are not a layout on the frame"));
            assert_eq!(layout.model(), model);
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
                table.model
            );
        }
    }

    /// Every table's point 1 lies within 3 pixels of its frame's centre on
    /// each axis: Nikon numbers the centre point 1 on every layout the NEF
    /// notes record, and a frame of the wrong size, or a table read in
    /// another frame, moves the centre away from it.
    #[test]
    fn every_tables_point_1_lies_at_the_frames_centre() {
        for table in TABLES {
            let model = table.model;
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
                "{model}: point 1 at {center} is more than 3 pixels from the centre of the \
                 {} frame",
                table.frame
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
            let model = table.model;
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
    /// padding, which [`Metadata::model`](crate::Metadata::model) never
    /// gives, or for none at all.
    #[test]
    fn find_matches_the_model_exactly() {
        for table in TABLES {
            let found = find(table.model).map(|found| found.model);
            assert_eq!(found, Some(table.model));
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
