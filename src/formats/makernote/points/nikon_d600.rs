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
//! table's layout, and frame 4, a shot with point 38, its place.
//!
//! Each point's name, in the comment beside it, is its row in the
//! viewfinder, A at the top to E at the bottom, and its column, 1 at the
//! left to 11 at the right; rows A and E have points over the three inner
//! columns only, and number them 1 to 3. The numbers follow the order in
//! which ExifTool's published tag documentation names them, read as
//! documentation and confirmed on the frames: 1 is the centre, 13 the top
//! of the inner columns' left one and 38 the top of the leftmost column.

use super::{PointTable, Provenance};
use crate::focus::LayoutPoint;
use crate::geometry::{Point, Size};

/// Every point's box, 164 by 164 pixels.
const BOX: Size = Size {
    width: 164,
    height: 164,
};

/// The D600's 39 points, in number order.
static POINTS: [LayoutPoint; 39] = [
    LayoutPoint::new(1, Point { x: 3008, y: 2007 }, BOX), // C6
    LayoutPoint::new(2, Point { x: 3008, y: 1755 }, BOX), // B6
    LayoutPoint::new(3, Point { x: 3008, y: 1503 }, BOX), // A2
    LayoutPoint::new(4, Point { x: 3008, y: 2258 }, BOX), // D6
    LayoutPoint::new(5, Point { x: 3008, y: 2510 }, BOX), // E2
    LayoutPoint::new(6, Point { x: 3247, y: 2007 }, BOX), // C7
    LayoutPoint::new(7, Point { x: 3247, y: 1755 }, BOX), // B7
    LayoutPoint::new(8, Point { x: 3247, y: 1503 }, BOX), // A3
    LayoutPoint::new(9, Point { x: 3247, y: 2258 }, BOX), // D7
    LayoutPoint::new(10, Point { x: 3247, y: 2510 }, BOX), // E3
    LayoutPoint::new(11, Point { x: 2768, y: 2007 }, BOX), // C5
    LayoutPoint::new(12, Point { x: 2768, y: 1755 }, BOX), // B5
    LayoutPoint::new(13, Point { x: 2768, y: 1503 }, BOX), // A1
    LayoutPoint::new(14, Point { x: 2768, y: 2258 }, BOX), // D5
    LayoutPoint::new(15, Point { x: 2768, y: 2510 }, BOX), // E1
    LayoutPoint::new(16, Point { x: 3486, y: 2007 }, BOX), // C8
    LayoutPoint::new(17, Point { x: 3486, y: 1705 }, BOX), // B8
    LayoutPoint::new(18, Point { x: 3486, y: 2308 }, BOX), // D8
    LayoutPoint::new(19, Point { x: 3724, y: 2007 }, BOX), // C9
    LayoutPoint::new(20, Point { x: 3724, y: 1705 }, BOX), // B9
    LayoutPoint::new(21, Point { x: 3724, y: 2308 }, BOX), // D9
    LayoutPoint::new(22, Point { x: 3963, y: 2007 }, BOX), // C10
    LayoutPoint::new(23, Point { x: 3963, y: 1705 }, BOX), // B10
    LayoutPoint::new(24, Point { x: 3963, y: 2308 }, BOX), // D10
    LayoutPoint::new(25, Point { x: 4203, y: 2007 }, BOX), // C11
    LayoutPoint::new(26, Point { x: 4203, y: 1705 }, BOX), // B11
    LayoutPoint::new(27, Point { x: 4203, y: 2308 }, BOX), // D11
    LayoutPoint::new(28, Point { x: 2529, y: 2007 }, BOX), // C4
    LayoutPoint::new(29, Point { x: 2529, y: 1705 }, BOX), // B4
    LayoutPoint::new(30, Point { x: 2529, y: 2308 }, BOX), // D4
    LayoutPoint::new(31, Point { x: 2289, y: 2007 }, BOX), // C3
    LayoutPoint::new(32, Point { x: 2289, y: 1705 }, BOX), // B3
    LayoutPoint::new(33, Point { x: 2289, y: 2308 }, BOX), // D3
    LayoutPoint::new(34, Point { x: 2050, y: 2007 }, BOX), // C2
    LayoutPoint::new(35, Point { x: 2050, y: 1705 }, BOX), // B2
    LayoutPoint::new(36, Point { x: 2050, y: 2308 }, BOX), // D2
    LayoutPoint::new(37, Point { x: 1812, y: 2007 }, BOX), // C1
    LayoutPoint::new(38, Point { x: 1812, y: 1705 }, BOX), // B1
    LayoutPoint::new(39, Point { x: 1812, y: 2308 }, BOX), // D1
];

/// The D600's table: its Model, its full-size output frame and its 39
/// points, with where they come from.
pub(super) static TABLE: PointTable = PointTable {
    model: "NIKON D600",
    frame: Size {
        width: 6016,
        height: 4016,
    },
    points: &POINTS,
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
