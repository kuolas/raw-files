// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon D600's 39 autofocus points, where each lies on the D600's
//! full-size output, 6016 by 4016 pixels.
//!
//! I shot the frames with my own D600 and measured them on 2026-10-26,
//! reading them in Nikon NX Studio 1.10.0, run as a reader to compare
//! with: frames 1 to 39 of the session, one per point in single-point
//! autofocus, with frames 40 and 41 repeating points 1 and 13,
//! which read the same. I calibrated the reading on the four live-view
//! frames, 53 to 56, which record their focus area in the file: NX Studio
//! draws a point's box with its outer edge at the area's bounds. Each
//! centre is the reading of NX Studio's box rounded to the pixel, good to
//! ±3 pixels, and every box reads 164 by 164. The record, with every
//! reading, is the D600's section of the NEF notes on autofocus points
//! ([`docs/formats/nef/autofocus-points.md`, section 2](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/autofocus-points.md#2-nikon-d600)),
//! and the 56 frames of the session are committed in
//! `tests/fixtures/nikon-d600/`, stripped of their pixels and personal
//! fields, so that the table can be checked on them: every frame gives the
//! table's layout, and every frame shot through the viewfinder in autofocus
//! the points it names, placed from the table, frame 4, a shot with point
//! 38, among them.
//!
//! Beside the points, the table carries what the same frames' bytes show
//! of the way a file names them: layout code 3, a mask whose bit number − 1
//! stands for each point, and the DX crop, on which frames 49 and 50 put
//! their points where the table's place, less CropHiSpeed's origin, says.
//!
//! Each point's name, in the comment beside it, is its row in the
//! viewfinder, A at the top to E at the bottom, and its column, 1 at the
//! left to 11 at the right; rows A and E have points over the three inner
//! columns only, and number them 1 to 3. The numbers follow the order in
//! which ExifTool's published tag documentation names them, read as
//! documentation and confirmed on the frames: 1 is the centre, 13 the top
//! of the inner columns' left one and 38 the top of the leftmost column.

use super::{BitOrder, PointTable, Provenance};
use crate::focus::FocusLayoutPoint;
use crate::geometry::{Point, Size};

/// Every point's box, 164 by 164 pixels.
const BOX: Size = Size {
    width: 164,
    height: 164,
};

/// The D600's 39 points, in number order.
static POINTS: [FocusLayoutPoint; 39] = [
    FocusLayoutPoint::new(1, Point { x: 3008, y: 2007 }, BOX), // C6
    FocusLayoutPoint::new(2, Point { x: 3008, y: 1755 }, BOX), // B6
    FocusLayoutPoint::new(3, Point { x: 3008, y: 1503 }, BOX), // A2
    FocusLayoutPoint::new(4, Point { x: 3008, y: 2258 }, BOX), // D6
    FocusLayoutPoint::new(5, Point { x: 3008, y: 2510 }, BOX), // E2
    FocusLayoutPoint::new(6, Point { x: 3247, y: 2007 }, BOX), // C7
    FocusLayoutPoint::new(7, Point { x: 3247, y: 1755 }, BOX), // B7
    FocusLayoutPoint::new(8, Point { x: 3247, y: 1503 }, BOX), // A3
    FocusLayoutPoint::new(9, Point { x: 3247, y: 2258 }, BOX), // D7
    FocusLayoutPoint::new(10, Point { x: 3247, y: 2510 }, BOX), // E3
    FocusLayoutPoint::new(11, Point { x: 2768, y: 2007 }, BOX), // C5
    FocusLayoutPoint::new(12, Point { x: 2768, y: 1755 }, BOX), // B5
    FocusLayoutPoint::new(13, Point { x: 2768, y: 1503 }, BOX), // A1
    FocusLayoutPoint::new(14, Point { x: 2768, y: 2258 }, BOX), // D5
    FocusLayoutPoint::new(15, Point { x: 2768, y: 2510 }, BOX), // E1
    FocusLayoutPoint::new(16, Point { x: 3486, y: 2007 }, BOX), // C8
    FocusLayoutPoint::new(17, Point { x: 3486, y: 1705 }, BOX), // B8
    FocusLayoutPoint::new(18, Point { x: 3486, y: 2308 }, BOX), // D8
    FocusLayoutPoint::new(19, Point { x: 3724, y: 2007 }, BOX), // C9
    FocusLayoutPoint::new(20, Point { x: 3724, y: 1705 }, BOX), // B9
    FocusLayoutPoint::new(21, Point { x: 3724, y: 2308 }, BOX), // D9
    FocusLayoutPoint::new(22, Point { x: 3963, y: 2007 }, BOX), // C10
    FocusLayoutPoint::new(23, Point { x: 3963, y: 1705 }, BOX), // B10
    FocusLayoutPoint::new(24, Point { x: 3963, y: 2308 }, BOX), // D10
    FocusLayoutPoint::new(25, Point { x: 4203, y: 2007 }, BOX), // C11
    FocusLayoutPoint::new(26, Point { x: 4203, y: 1705 }, BOX), // B11
    FocusLayoutPoint::new(27, Point { x: 4203, y: 2308 }, BOX), // D11
    FocusLayoutPoint::new(28, Point { x: 2529, y: 2007 }, BOX), // C4
    FocusLayoutPoint::new(29, Point { x: 2529, y: 1705 }, BOX), // B4
    FocusLayoutPoint::new(30, Point { x: 2529, y: 2308 }, BOX), // D4
    FocusLayoutPoint::new(31, Point { x: 2289, y: 2007 }, BOX), // C3
    FocusLayoutPoint::new(32, Point { x: 2289, y: 1705 }, BOX), // B3
    FocusLayoutPoint::new(33, Point { x: 2289, y: 2308 }, BOX), // D3
    FocusLayoutPoint::new(34, Point { x: 2050, y: 2007 }, BOX), // C2
    FocusLayoutPoint::new(35, Point { x: 2050, y: 1705 }, BOX), // B2
    FocusLayoutPoint::new(36, Point { x: 2050, y: 2308 }, BOX), // D2
    FocusLayoutPoint::new(37, Point { x: 1812, y: 2007 }, BOX), // C1
    FocusLayoutPoint::new(38, Point { x: 1812, y: 1705 }, BOX), // B1
    FocusLayoutPoint::new(39, Point { x: 1812, y: 2308 }, BOX), // D1
];

/// The D600's table: its Model, its full-size output frame and its 39
/// points, with what its record confirms of the way a file names them, and
/// where they come from.
pub(super) static TABLE: PointTable = PointTable {
    model: "NIKON D600",
    frame: Size {
        width: 6016,
        height: 4016,
    },
    points: &POINTS,
    // Every frame through the viewfinder with a point number writes layout
    // code 3 (section 2.2 of the record).
    layout_code: 3,
    // On all 45 single-point frames with a point number, exactly one bit
    // is set, bit number − 1 (section 2.3).
    bit_order: BitOrder::NumberLessOne,
    // The DX crop, CropHiSpeed's setting 2: frames 49 and 50 put points 38
    // and 27 within 2 pixels of the table's place less CropHiSpeed's
    // origin, (1040, 696), in the 3936 by 2624 picture of the NEF's
    // preview and the JPEG alike (section 2.6).
    crop_codes: &[2],
    provenance: Provenance {
        shot_by: "Alvaro Kuolas",
        measured_by: "Alvaro Kuolas",
        measured_on: "2026-10-26",
        software: "Nikon NX Studio 1.10.0",
        frames: "1 to 39, one per point, with the repeats 40 and 41; calibrated on the \
                 live-view frames 53 to 56",
        fixtures: "nikon-d600/",
    },
};
