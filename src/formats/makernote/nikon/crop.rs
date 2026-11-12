// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote's CropHiSpeed, and placing a focus point on a
//! crop: a recorded position on the cropped picture, and the points
//! a record names, placed from the body's table.

use std::cmp::Ordering;
use std::io::{Read, Seek};

use super::af_info2::{FocusRecord, NamedPoints};
use super::block::MakerNote;
use super::metadata::CropArea;
use super::rules;
use crate::error::Result;
use crate::focus::{Focus, FocusOrigin, FocusPoint, FocusPoints, check_point, scale};
use crate::formats::makernote::points::PointTable;
use crate::geometry::{Point, Size};
use crate::io::Source;

/// What CropHiSpeed records of a crop: the crop setting, which gives the
/// image area ([`image_area`]) and says which crop a point placed from a
/// table may be placed on ([`placed_frame`]); the size of the picture
/// before the crop and after it, in the sensor's pixels, which [`in_crop`]
/// and [`placed_frame`] only compare with each other; and where the crop
/// starts, which a point placed from a table requires to be its window's
/// centered origin ([`placed_frame`]).
///
/// [`image_area`]: super::settings::image_area
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CropHiSpeed {
    /// The crop setting, its first value, such as 11 for FX uncropped.
    pub(super) mode: u32,
    /// The original width and height, those of the uncropped picture.
    original: Size,
    /// The cropped width and height.
    cropped: Size,
    /// The crop's left column and top row, its sixth and seventh values,
    /// in the sensor's pixels: (1040, 696) on a D600 DX frame, (0, 0)
    /// uncropped. On the D600 it is also the origin of the DX picture's
    /// window centered in the 6016 by 4016 output, which a point placed from
    /// a table requires of it ([`placed_frame`]).
    origin: Point,
}

impl CropHiSpeed {
    /// What this crop cut, by comparing its cropped size with its original
    /// size on each side: `None` for no crop, the two sizes equal, and for
    /// a crop that fits neither shape, such as one with a cropped side
    /// larger than its original, or one side cut and the other larger.
    fn cut(self) -> Option<Cut> {
        let (original, cropped) = (self.original, self.cropped);
        match (
            cropped.width.cmp(&original.width),
            cropped.height.cmp(&original.height),
        ) {
            (Ordering::Less, Ordering::Less) => Some(Cut::Both),
            (Ordering::Less, Ordering::Equal) => Some(Cut::Width),
            (Ordering::Equal, Ordering::Less) => Some(Cut::Height),
            _ => None,
        }
    }
}

/// What a crop cut from the camera's full output, as CropHiSpeed tells it
/// ([`CropHiSpeed::cut`]), which decides what its recorded focus point
/// means on the cropped picture ([`in_crop`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cut {
    /// Both sides, as a DX crop does: the point records fractions of the
    /// cropped picture.
    Both,
    /// The width, keeping the height, as a 1:1 crop of a landscape frame
    /// does: the point records a position in the full frame.
    Width,
    /// The height, keeping the width, as a 16:9 crop of a 3:2 frame does:
    /// the point records a position in the full frame.
    Height,
}

/// What [`in_crop`] places a crop's focus point on: the picture, in the
/// pixels it is measured in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Onto {
    /// CropArea's size: the visible rectangle of the stored raw data, in
    /// a file whose MakerNote holds a CropArea that gave one.
    CropArea(Size),
    /// The size of a JPEG's own pixels, as its frame header gives it, in a
    /// file whose MakerNote holds no CropArea: the `pixels` [`metadata`]
    /// gives [`in_crop`] for a JPEG's picture (ADR015).
    ///
    /// [`metadata`]: fn@super::metadata
    Pixels(Size),
}

/// What `makernote`'s CropHiSpeed records of the crop; `None` when the tag
/// is missing, is not of type SHORT, does not hold exactly seven values,
/// or stores them outside the block. A file without it is read as no crop,
/// and has no image area.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn crop_hi_speed<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<CropHiSpeed>> {
    let Some(
        [
            mode,
            original_width,
            original_height,
            cropped_width,
            cropped_height,
            left,
            top,
        ],
    ) = makernote.ifd.get(src, rules::CROP_HI_SPEED)?
    else {
        return Ok(None);
    };
    Ok(Some(CropHiSpeed {
        mode,
        original: Size {
            width: original_width,
            height: original_height,
        },
        cropped: Size {
            width: cropped_width,
            height: cropped_height,
        },
        origin: Point { x: left, y: top },
    }))
}

/// The focus point `record` describes, placed on the cropped picture when
/// the file is a crop, and as AFInfo2 records it otherwise; `None` when
/// the record is refused as recorded, or the file is a crop on which it
/// cannot be placed.
///
/// The record is checked first, as recorded, in its own frame, by
/// [`FocusRecord::focus`], on a crop as on any other file. A center outside
/// the recorded frame, or an area that does not fit around it there, gives
/// none, even where placing it on the cropped picture would move it inside.
///
/// A file is a crop when CropHiSpeed, `crop`, records a cropped size
/// that differs from its original size. On a crop, AFInfo2's frame is
/// still the camera's full output, and the point is placed on the cropped
/// picture ([`Onto`]): CropArea, when the file has one, which `crop_area`
/// gives; and, when it has none, `pixels`, a JPEG's own pixels, of the
/// size its frame header gives (ADR015). A crop without a CropArea tag
/// and without `pixels`, as in a raw file, such as the D810's, whose
/// cropped width is four pixels short on an uncropped file (public sample
/// 2036), has no picture to place the record on, and is read as recorded.
/// A crop whose CropArea cannot be used, or could not be checked against
/// a raw size, as in any JPEG, has a picture whose size is not known, and
/// gives none. What happens to the center and the area depends on what the
/// crop cut ([`CropHiSpeed::cut`]), as the module documentation shows:
///
/// - **A crop that cuts both sides** (DX) records fractions of the
///   cropped picture, in the full frame's units. The point is scaled onto
///   CropArea, or onto the JPEG's pixels, by [`Focus::scaled_to`]: x and
///   the width multiplied by the picture's width over the frame's, y and
///   the height by the heights, each rounded to the nearest pixel, a half
///   rounding up, and the frame becomes the picture's size. An area that
///   rounding leaves a pixel too wide for its center is trimmed to fit,
///   and an area side that rounds to nothing keeps one pixel. A center
///   that rounds onto an edge of the picture leaves no room for an area,
///   and keeps the point without one.
/// - **A crop that keeps one side** (1:1, 16:9), whose cropped size
///   equals the original on exactly one side and is smaller on the other,
///   records positions in the full frame, of which the cropped picture is
///   the centered window. The window is CropArea; on a JPEG's pixels, it
///   is the frame's length on the side the crop keeps, and on the side it
///   cuts, the kept length times the JPEG's cut side over its kept side
///   ([`window_of_shape`]), so that the window has the JPEG's shape in the
///   frame's pixels. The center moves back on each axis by the window's
///   origin, half the difference between the frame and the window, rounded
///   down ([`shifted`]), the area is kept, and the frame becomes the
///   window. That holds only when the window's kept side equals the
///   frame's, as a JPEG's does by its making: on a small raw, CropArea
///   counts stored pixels and the frame the full output, the window cannot
///   be placed, and the file gives none. A window side larger than the
///   frame's, or a center that would move past the window's left or top
///   edge, gives none too. The moved record is checked again by
///   [`FocusRecord::focus`], in the window: a center past its right or
///   bottom edge, or an area that no longer fits around the moved center,
///   gives none.
///
/// A CropHiSpeed that fits neither shape, such as one whose cropped size
/// is larger than its original on a side, records no crop the point can
/// be placed on, and gives none; except in a file with neither a CropArea
/// nor `pixels`, which is read as recorded whatever its CropHiSpeed
/// records.
pub(super) fn in_crop(
    record: FocusRecord,
    crop: Option<CropHiSpeed>,
    crop_area: CropArea,
    pixels: Option<Size>,
) -> Option<Focus> {
    // `Focus::new` refuses a frame with a zero side here, so the old frame
    // whose sides `scaled_to` divides by below has no zero side.
    let focus = record.focus()?;
    let Some(crop) = crop else {
        return Some(focus);
    };
    if crop.cropped == crop.original {
        return Some(focus);
    }
    let onto = match (crop_area, pixels) {
        (CropArea::Size(window), _) => Onto::CropArea(window),
        (CropArea::Unusable, _) => return None,
        (CropArea::Absent, None) => return Some(focus),
        (CropArea::Absent, Some(pixels)) => Onto::Pixels(pixels),
    };
    let cut = crop.cut()?;
    let FocusRecord { frame, center, .. } = record;
    let window = match (cut, onto) {
        (Cut::Both, Onto::CropArea(size) | Onto::Pixels(size)) => {
            return focus.scaled_to(size).ok();
        }
        (_, Onto::CropArea(window)) => window,
        (_, Onto::Pixels(pixels)) => window_of_shape(frame, cut, pixels)?,
    };
    if (cut == Cut::Height && window.width != frame.width)
        || (cut == Cut::Width && window.height != frame.height)
    {
        return None;
    }
    // `Focus` cannot move its center, so the moved point is built anew,
    // with the area and the in-focus byte as recorded.
    FocusRecord {
        frame: window,
        center: Point {
            x: shifted(center.x, frame.width, window.width)?,
            y: shifted(center.y, frame.height, window.height)?,
        },
        ..record
    }
    .focus()
}

/// The centered window of the recorded frame `frame` that a crop which cut
/// one side, `cut`, shows on a JPEG of `pixels`: the frame's length on the
/// side the crop keeps, and on the side it cuts, that length times the
/// JPEG's cut side over its kept side, rounded to the nearest pixel, a
/// half rounding up, by the rule [`Focus::scaled_to`] scales by
/// ([`scale`]) (ADR015). So the window has the JPEG's shape, in the frame's
/// pixels. It may be longer than the frame on the cut side, which
/// [`shifted`] then refuses. A cut side too long for a `u32` saturates at
/// `u32::MAX`, as [`scale`] does, and is refused the same way: the
/// frame's sides are read from SHORTs, so no frame is that long.
///
/// `None` for a crop that cut both sides, which has no window, and for a
/// JPEG side of zero, which [`scale`] cannot divide by.
fn window_of_shape(frame: Size, cut: Cut, pixels: Size) -> Option<Size> {
    if pixels.width == 0 || pixels.height == 0 {
        return None;
    }
    match cut {
        Cut::Both => None,
        Cut::Width => Some(Size {
            width: scale(frame.height, pixels.height, pixels.width),
            height: frame.height,
        }),
        Cut::Height => Some(Size {
            width: frame.width,
            height: scale(frame.width, pixels.width, pixels.height),
        }),
    }
}

/// `value`, a position on an axis `frame` pixels long, as a position in
/// the centered window `window` pixels long on it: less the window's
/// origin, half of `frame - window`, rounded down. `None` when the window
/// is longer than the frame, or the position lies before the window's
/// start.
fn shifted(value: u32, frame: u32, window: u32) -> Option<u32> {
    value.checked_sub(centered_origin(frame, window)?)
}

/// Where a window `window` pixels long starts when centered on a side
/// `frame` pixels long: half of `frame - window`, rounded down. `None`
/// when the window is longer than the frame.
fn centered_origin(frame: u32, window: u32) -> Option<u32> {
    Some(frame.checked_sub(window)? / 2)
}

/// The points that `named`, what a record with no position names, names,
/// placed from `table`, the table of the body the file's Model names, by
/// the decision to place a point that the camera names by number from the
/// body's table (ADR018); `None` when they cannot be placed.
///
/// The record names points of the table only when its layout code is the
/// table's; any other code, such as 0 on a manual-focus or live-view shot,
/// names none. The list then holds every point of the table that is the
/// primary, the one whose number is `named`'s, or whose bit the mask sets
/// ([`PointTable::bit`]), in the table's order, which is number order;
/// each with its number, the table's center and box, nothing said of
/// whether focus was confirmed, since version 0100's in-focus byte is the
/// contrast-detect result, and the origin [`FocusOrigin::Table`]. So a set
/// bit the table lacks is skipped; an empty mask, or one that names no
/// point of the table, gives the primary alone; and a mask that leaves out
/// the primary's own bit gives its points and the primary. The primary is
/// marked: `None` when the table has no point of its number, as for 0,
/// which names none, or a number past the table.
///
/// The frame is the table's, or on a crop the crop's window, or on an
/// uncropped JPEG of a size the table lists the window that size shows,
/// into which every center moves back by the window's origin, the box kept
/// ([`placed_frame`]). A point that no longer lies on the window by the
/// rule every focus point is checked by ([`check_point`]), its center outside
/// it or its box across an edge, is dropped; the primary dropped costs the
/// whole list.
pub(super) fn placed(
    named: NamedPoints,
    table: &PointTable,
    crop: Option<CropHiSpeed>,
    pixels: Option<Size>,
) -> Option<FocusPoints> {
    if named.layout_code != table.layout_code() {
        return None;
    }
    let (frame, origin) = placed_frame(table, crop, pixels)?;
    let mut points = FocusPoints::new(frame, FocusOrigin::Table).ok()?;
    let mut primary = None;
    for point in table.points() {
        let number = point.number();
        let is_primary = number == u16::from(named.primary);
        if !is_primary && !table.bit(number).is_some_and(|bit| named.marks(bit)) {
            continue;
        }
        let (center, size) = (point.center(), point.size());
        // A center left or above the window lies outside it, as one past
        // its right or bottom edge does.
        let (Some(x), Some(y)) = (
            center.x.checked_sub(origin.x),
            center.y.checked_sub(origin.y),
        ) else {
            continue;
        };
        let center = Point { x, y };
        if check_point(frame, center, Some(size)).is_err() {
            continue;
        }
        if is_primary {
            primary = Some(points.points().len());
        }
        // `check_point` is the rule `with_point` checks, so it refuses none
        // here: the `?` never returns.
        points = points
            .with_point(FocusPoint::new(center).with_number(number).with_size(size))
            .ok()?;
    }
    points.with_primary_index(primary?).ok()
}

/// The frame of the points placed from `table`, and the origin their
/// centers move back by: on a file that is no crop, whose CropHiSpeed,
/// `crop`, records a cropped size equal to its original size or is
/// missing, the table's frame, the body's full-size output, and no move;
/// but in a JPEG whose own size, `pixels`, is one the table lists as
/// showing a window of the frame ([`PointTable::jpeg_window`]), that
/// window, 6010 by 4006 on a D600 Medium JPEG, and its origin centered in
/// the table's frame, (3, 5) on the D600, by the decision to keep a JPEG
/// size's window in the table (ADR018). A JPEG of any other size, which
/// the JPEG reader then holds to the picture's shape, and a raw file,
/// which has no `pixels`, keep the table's frame.
///
/// On a crop, only when the table's record confirms the crop's setting:
/// the crop's window, the size the record measured
/// ([`PointTable::crop_window`]), 3936 by 2624 on a D600 DX frame, and its
/// origin centered in the table's frame, half of the frame less the window
/// on each axis, (1040, 696) on the D600, by the decisions to measure a
/// placed point on a crop in the crop's window and to keep each crop's
/// window in the table (ADR018). CropHiSpeed's origin must be that
/// origin: one that is not records a crop placed elsewhere in the frame
/// than the record's frames show, which the window cannot be trusted on.
/// In a JPEG, whose own size is `pixels`, the window must be that size,
/// which on the D600 is the full image size: a crop-mode JPEG at a smaller
/// size gives none (ADR018), whatever JPEG sizes the table lists.
///
/// `None` on a crop the record does not confirm, for a CropHiSpeed origin
/// other than the window's, for a window that does not fit the frame,
/// which the table tests rule out, and on a crop for a JPEG whose size is
/// not the window's.
fn placed_frame(
    table: &PointTable,
    crop: Option<CropHiSpeed>,
    pixels: Option<Size>,
) -> Option<(Size, Point)> {
    let frame = table.frame();
    let centered = |window: Size| {
        Some(Point {
            x: centered_origin(frame.width, window.width)?,
            y: centered_origin(frame.height, window.height)?,
        })
    };
    match crop.filter(|crop| crop.cropped != crop.original) {
        Some(crop) => {
            let window = table.crop_window(crop.mode)?;
            let origin = centered(window)?;
            let confirmed = crop.origin == origin && pixels.is_none_or(|pixels| pixels == window);
            confirmed.then_some((window, origin))
        }
        None => match pixels.and_then(|pixels| table.jpeg_window(pixels)) {
            Some(window) => Some((window, centered(window)?)),
            None => Some((frame, Point { x: 0, y: 0 })),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{
        B1, D600_BOX, D600_FX, ORDERS, Recorded, Z6, af_info2, d600_viewfinder, expected,
        jpeg_metadata_of, metadata_of, placed_list, raw_file_of, shot, shot_file, tags_block,
        viewfinder, whole, with_makernote,
    };
    use super::super::{AF_INFO2, CROP_AREA, CROP_HI_SPEED};
    use super::*;
    use crate::container::tiff::tags::{DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE};
    use crate::focus::FocusAreaMode;
    use crate::formats::makernote::points;
    use crate::metadata::Metadata;
    use crate::tiff_builder::{Order, UNDEFINED};

    #[test]
    fn a_crop_lists_the_placed_point() {
        // A raw file's crop, placed on CropArea, moved or scaled, and a
        // JPEG's DX crop, scaled onto its pixels: the list is the placed
        // point alone, in the cropped picture's frame.
        for order in ORDERS {
            for (what, crop, version, recorded) in [
                ("Z6III 1:1", Z6III_1X1, "0401", Z6III_ASPECT),
                ("Z6III DX", Z6III_DX, "0401", Z6III_DX_RECORD),
                ("Z50II 16:9", Z50II_16X9, "0402", Z50II_16X9_RECORD),
            ] {
                let block = crop_block(order, crop, &af_info2(order, version, recorded));
                let [width, height] = crop.raw;
                let raw = Size { width, height };
                let file = raw_file_of(order, raw, &block, whole(&block), |_, _| {});
                let metadata = metadata_of(file);
                let placed = metadata.focus().expect(what);
                let expected = FocusPoints::from(placed).with_area_mode(FocusAreaMode::Single);
                assert_eq!(
                    metadata.focus_points(),
                    Some(&expected),
                    "{order:?}, {what}"
                );
                // Placed, so not in the frame AFInfo2 records.
                let [width, height] = recorded.frame.map(u32::from);
                assert_ne!(placed.frame(), Size { width, height }, "{order:?}, {what}");
            }
            let metadata =
                jpeg_metadata(order, order, in_a_jpeg(Z9_DX), Z9_DX_CORNER, [4032, 2688]);
            let placed = metadata.focus().expect("a JPEG's DX point");
            assert_eq!(
                placed.frame(),
                Size {
                    width: 4032,
                    height: 2688
                }
            );
            let expected = FocusPoints::from(placed).with_area_mode(FocusAreaMode::Single);
            assert_eq!(
                metadata.focus_points(),
                Some(&expected),
                "{order:?}, JPEG DX"
            );
        }
    }

    /// The D600's CropHiSpeed in the DX crop, as frames 49 to 51 record it.
    const D600_DX: [u16; 7] = [2, 6080, 4028, 3968, 2636, 1040, 696];

    /// The D600's DX CropHiSpeed with its origin moved to (`left`, `top`),
    /// its sizes kept.
    fn d600_dx_at(left: u32, top: u32) -> CropHiSpeed {
        CropHiSpeed {
            mode: 2,
            original: Size {
                width: 6080,
                height: 4028,
            },
            cropped: Size {
                width: 3968,
                height: 2636,
            },
            origin: Point { x: left, y: top },
        }
    }

    /// The points that the D600's table places for a viewfinder shot that
    /// names `primary` and sets the mask's `bits`, on the crop `crop`, as
    /// if the table's record confirmed the DX crop, setting 2, in a window
    /// of `window` instead of 3936 by 2624, so that a test can place points
    /// in a window no body has, in a raw file. The list is given the area
    /// mode single point, as the reader gives it.
    fn placed_in_window(
        window: [u32; 2],
        crop: CropHiSpeed,
        primary: u8,
        bits: &[usize],
    ) -> Option<FocusPoints> {
        let [width, height] = window;
        // A table's crops are `'static`; each call leaks the few bytes of
        // one, which a test can afford.
        let crops = Box::leak(Box::new([points::ConfirmedCrop::new(
            2,
            Size { width, height },
        )]));
        let table = points::find("NIKON D600")
            .expect("the D600's table")
            .with_crops(crops);
        let mut mask = [0; points::MASK_LEN];
        for &bit in bits {
            mask[bit / 8] |= 1 << (bit % 8);
        }
        let named = NamedPoints {
            layout_code: 3,
            primary,
            mask,
        };
        super::placed(named, &table, Some(crop), None)
            .map(|points| points.with_area_mode(FocusAreaMode::Single))
    }

    /// The metadata of the file [`shot_file`] builds for a D600, in
    /// autofocus, read as a JPEG whose frame header gives `pixels`.
    fn d600_jpeg(
        order: Order,
        af_info2: &[u8],
        crop_hi_speed: Option<[u16; 7]>,
        pixels: [u32; 2],
    ) -> Metadata {
        let file = shot_file(order, "NIKON D600", "AF-S  ", af_info2, crop_hi_speed);
        jpeg_metadata_of(file, pixels)
    }

    #[test]
    fn a_d600_viewfinder_shot_gives_the_point_it_names_placed_from_the_table() {
        // Frame 4 of the D600 session: a viewfinder shot that names point
        // 38, with the crop off or no CropHiSpeed at all. The point is
        // placed from the table, in its 6016 by 4016 frame, with the box
        // of 164 by 164, from a table, and nothing said of focus, though
        // the in-focus byte reads 0; the list is that point, numbered, the
        // primary, single point.
        let frame = Size {
            width: 6016,
            height: 4016,
        };
        let expected = Focus::new(frame, Point { x: 1812, y: 1705 }, FocusOrigin::Table)
            .and_then(|focus| focus.with_size(D600_BOX))
            .unwrap();
        for order in ORDERS {
            for crop in [Some(D600_FX), None] {
                let what = format!("{order:?}, {crop:?}");
                let metadata = shot(order, "NIKON D600", &d600_viewfinder(order), crop);
                assert_eq!(metadata.focus(), Some(expected), "{what}");
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list([6016, 4016], &[B1], 38, FocusAreaMode::Single)),
                    "{what}"
                );
            }
        }
    }

    #[test]
    fn a_named_point_is_placed_only_under_every_condition() {
        // Each condition failing alone, beside frame 4's bytes, gives no
        // point and no list: manual focus; version 0101, with the same
        // bytes where 0100 keeps them; a Model without a table, the D610,
        // whose layout is the D600's; layout code 2, the 11-point layout,
        // or 0, none; the number 0, none, 40, one past the table, or 255.
        for order in ORDERS {
            let mut cases = vec![
                (
                    "manual focus",
                    metadata_of(shot_file(
                        order,
                        "NIKON D600",
                        "MANUAL",
                        &d600_viewfinder(order),
                        Some(D600_FX),
                    )),
                ),
                (
                    "version 0101",
                    shot(
                        order,
                        "NIKON D600",
                        &viewfinder(order, "0101", 3, 38, &[37]),
                        Some(D600_FX),
                    ),
                ),
                (
                    "no table",
                    shot(order, "NIKON D610", &d600_viewfinder(order), Some(D600_FX)),
                ),
            ];
            for (what, code) in [("layout code 2", 2), ("layout code 0", 0)] {
                let value = viewfinder(order, "0100", code, 38, &[37]);
                cases.push((what, shot(order, "NIKON D600", &value, Some(D600_FX))));
            }
            for (what, number) in [("number 0", 0), ("number 40", 40), ("number 255", 255)] {
                let value = viewfinder(order, "0100", 3, number, &[]);
                cases.push((what, shot(order, "NIKON D600", &value, Some(D600_FX))));
            }
            for (what, metadata) in cases {
                assert_eq!(metadata.focus(), None, "{order:?}, {what}");
                assert_eq!(metadata.focus_points(), None, "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn the_mask_names_the_points_used_in_number_order() {
        let d600 = |order, primary, bits: &[usize]| {
            shot(
                order,
                "NIKON D600",
                &viewfinder(order, "0100", 3, primary, bits),
                Some(D600_FX),
            )
        };
        for order in ORDERS {
            // Frame 44, auto-area: area mode 8, primary point 4, nine bits
            // set out of number order. The list is the nine points in
            // number order, centers from the record.
            let mut value = viewfinder(order, "0100", 3, 4, &[3, 8, 13, 17, 20, 23, 26, 29, 38]);
            value[5] = 8;
            let metadata = shot(order, "NIKON D600", &value, Some(D600_FX));
            let frame_44 = [
                (4, [3008, 2258]),
                (9, [3247, 2258]),
                (14, [2768, 2258]),
                (18, [3486, 2308]),
                (21, [3724, 2308]),
                (24, [3963, 2308]),
                (27, [4203, 2308]),
                (30, [2529, 2308]),
                (39, [1812, 2308]),
            ];
            assert_eq!(
                metadata.focus_points(),
                Some(&placed_list(
                    [6016, 4016],
                    &frame_44,
                    4,
                    FocusAreaMode::Auto
                )),
                "{order:?}, frame 44"
            );
            // Every bit of the seven bytes set: the table's 39 points, in
            // number order, and the 17 bits past them skipped.
            let every_bit: Vec<usize> = (0..56).collect();
            let points = d600(order, 38, &every_bit).focus_points().cloned();
            let points = points.expect("every point");
            let numbers: Vec<Option<u16>> =
                points.points().iter().map(FocusPoint::number).collect();
            assert_eq!(numbers, (1..=39).map(Some).collect::<Vec<_>>(), "{order:?}");
            assert_eq!(points.primary().and_then(|point| point.number()), Some(38));
            // An empty mask, or one whose bits the table lacks: the primary
            // alone.
            let alone = placed_list([6016, 4016], &[B1], 38, FocusAreaMode::Single);
            for bits in [&[][..], &[39, 47, 55]] {
                let metadata = d600(order, 38, bits);
                assert_eq!(metadata.focus_points(), Some(&alone), "{order:?}, {bits:?}");
            }
            // A mask without the primary's own bit: its points and the
            // primary, in number order.
            let metadata = d600(order, 38, &[0, 38]);
            let expected = placed_list(
                [6016, 4016],
                &[(1, [3008, 2007]), B1, (39, [1812, 2308])],
                38,
                FocusAreaMode::Single,
            );
            assert_eq!(metadata.focus_points(), Some(&expected), "{order:?}");
            assert_eq!(
                metadata.focus().map(|focus| focus.center()),
                Some(Point { x: 1812, y: 1705 }),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_places_the_points_in_the_crop_window() {
        // CropHiSpeed's DX crop, setting 2 with the origin (1040, 696): the
        // frame is the window, 6016 - 2 * 1040 by 4016 - 2 * 696, 3936 by
        // 2624, and every center moves back by the origin, the box kept.
        // Frame 49's point 38 lands at (772, 1009) and frame 50's point 27
        // at (3163, 1612); a list of points 1, 27 and 38 moves whole.
        for order in ORDERS {
            for (primary, bits, points) in [
                (38, &[37][..], &[(38, [772, 1009])][..]),
                (27, &[26], &[(27, [3163, 1612])]),
                (
                    1,
                    &[0, 26, 37],
                    &[(1, [1968, 1311]), (27, [3163, 1612]), (38, [772, 1009])],
                ),
            ] {
                let value = viewfinder(order, "0100", 3, primary, bits);
                let metadata = shot(order, "NIKON D600", &value, Some(D600_DX));
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list(
                        [3936, 2624],
                        points,
                        u16::from(primary),
                        FocusAreaMode::Single
                    )),
                    "{order:?}, point {primary}"
                );
            }
            // Setting 2 with the two sizes equal is no crop: the table's
            // frame.
            let no_crop = [2, 6080, 4028, 6080, 4028, 1040, 696];
            let metadata = shot(order, "NIKON D600", &d600_viewfinder(order), Some(no_crop));
            assert_eq!(
                metadata.focus_points(),
                Some(&placed_list([6016, 4016], &[B1], 38, FocusAreaMode::Single)),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_the_table_does_not_confirm_gives_no_placed_point() {
        // The D600's record confirms the DX crop alone, setting 2: any
        // other crop setting with the DX crop's sizes and origin gives no
        // point, 1.3x, 16:9, 1:1 and the others CropHiSpeed records.
        for order in ORDERS {
            for code in [0, 1, 6, 9, 10, 11, 12, 17, 18] {
                let [_, rest @ ..] = D600_DX;
                let mut crop = [code; 7];
                crop[1..].copy_from_slice(&rest);
                let metadata = shot(order, "NIKON D600", &d600_viewfinder(order), Some(crop));
                assert_eq!(metadata.focus_points(), None, "{order:?}, setting {code}");
            }
        }
    }

    #[test]
    fn a_point_the_crop_window_leaves_out_is_dropped() {
        // No point of the D600's table lies outside its DX window, so these
        // windows are narrower than any the D600 has, each centered in the
        // frame with CropHiSpeed's origin at its own.
        //
        // A window of 2536 by 2624, at the origin (1740, 696): point 38 at
        // (72, 1009), whose box of 164 crosses the left edge, and point 25
        // at (2463, 1311), whose box crosses the right one, are dropped;
        // points 1 and 11 stay.
        let narrow = d600_dx_at(1740, 696);
        let expected = placed_list(
            [2536, 2624],
            &[(1, [1268, 1311]), (11, [1028, 1311])],
            1,
            FocusAreaMode::Single,
        );
        assert_eq!(
            placed_in_window([2536, 2624], narrow, 1, &[10, 24, 37]),
            Some(expected)
        );
        // A window of 2556 by 2624, at the origin (1730, 696): point 38's
        // box reaches the left edge exactly, at (82, 1009), and is kept.
        let expected = placed_list(
            [2556, 2624],
            &[(1, [1278, 1311]), (38, [82, 1009])],
            1,
            FocusAreaMode::Single,
        );
        assert_eq!(
            placed_in_window([2556, 2624], d600_dx_at(1730, 696), 1, &[37]),
            Some(expected)
        );
        // A window of 2216 by 2624, at the origin (1900, 696): point 38's
        // center lies left of the window, dropped.
        let narrowest = d600_dx_at(1900, 696);
        let expected = placed_list([2216, 2624], &[(1, [1108, 1311])], 1, FocusAreaMode::Single);
        assert_eq!(
            placed_in_window([2216, 2624], narrowest, 1, &[37]),
            Some(expected)
        );
        // The primary dropped, by its box or by its center: no point.
        assert_eq!(placed_in_window([2536, 2624], narrow, 38, &[37]), None);
        assert_eq!(placed_in_window([2216, 2624], narrowest, 38, &[37]), None);
    }

    #[test]
    fn a_crop_window_with_no_room_gives_no_placed_point() {
        // A window with a zero side, which no list can be measured in, and
        // one wider or taller than the frame, which has no origin: no
        // point. Each CropHiSpeed records the window's centered origin, with
        // 0 on a side where the window is longer than the frame, so that
        // the window alone is at fault. The table tests keep such a window out of
        // every table.
        for (window, crop) in [
            ([0, 2624], d600_dx_at(3008, 696)),
            ([3936, 0], d600_dx_at(1040, 2008)),
            ([6018, 2624], d600_dx_at(0, 696)),
            ([3936, 4018], d600_dx_at(1040, 0)),
        ] {
            assert_eq!(placed_in_window(window, crop, 1, &[0]), None, "{window:?}");
        }
    }

    #[test]
    fn a_crop_whose_origin_is_not_the_windows_gives_no_placed_point() {
        // The D600's DX window is 3936 by 2624, centered at (1040, 696),
        // the origin every DX frame's CropHiSpeed records. A CropHiSpeed
        // with another origin records the crop elsewhere in the frame, so
        // the window measured on the frames does not hold for it: no
        // point, in a raw file or in a JPEG of the window's size. The
        // origins are a pixel off on each side; (1024, 690), the centered
        // origin of CropHiSpeed's cropped size, 3968 by 2636, which counts
        // the sensor's border; a top of 0, as a crop at the top of the
        // frame would record; and the largest SHORTs.
        for order in ORDERS {
            for (left, top) in [
                (1041, 696),
                (1039, 696),
                (1040, 697),
                (1040, 695),
                (1024, 690),
                (1040, 0),
                (u16::MAX, u16::MAX),
            ] {
                let mut crop = D600_DX;
                crop[5] = left;
                crop[6] = top;
                let value = d600_viewfinder(order);
                let metadata = shot(order, "NIKON D600", &value, Some(crop));
                assert_eq!(metadata.focus_points(), None, "{order:?}, ({left}, {top})");
                let metadata = d600_jpeg(order, &value, Some(crop), [3936, 2624]);
                assert_eq!(
                    metadata.focus_points(),
                    None,
                    "{order:?}, ({left}, {top}) JPEG"
                );
            }
        }
    }

    #[test]
    fn a_d600_jpeg_gives_the_point_its_raw_file_gives() {
        for order in ORDERS {
            // Uncropped, at full size, the Large JPEG's, at half of it, the
            // Small JPEG's, and at a quarter of it: the table's frame, as at
            // every size the table does not list; the JPEG reader holds it
            // to the picture's shape. So do sizes a pixel off the Medium
            // JPEG's, 4512 by 3008, which the table lists, and the 4512 by
            // 3012 of the whole frame scaled as the Medium JPEG is.
            for pixels in [
                [6016, 4016],
                [3008, 2008],
                [1504, 1004],
                [4512, 3012],
                [4511, 3008],
                [4512, 3009],
            ] {
                let metadata = d600_jpeg(order, &d600_viewfinder(order), Some(D600_FX), pixels);
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list([6016, 4016], &[B1], 38, FocusAreaMode::Single)),
                    "{order:?}, {pixels:?}"
                );
            }
            // In DX at full size, the window's 3936 by 2624: frame 49's
            // point, as in its NEF.
            let metadata = d600_jpeg(order, &d600_viewfinder(order), Some(D600_DX), [3936, 2624]);
            assert_eq!(
                metadata.focus_points(),
                Some(&placed_list(
                    [3936, 2624],
                    &[(38, [772, 1009])],
                    38,
                    FocusAreaMode::Single
                )),
                "{order:?}"
            );
            // In DX at a smaller size, or a pixel off the window: none.
            for pixels in [[2944, 1968], [1968, 1312], [3936, 2625], [3935, 2624]] {
                let metadata = d600_jpeg(order, &d600_viewfinder(order), Some(D600_DX), pixels);
                assert_eq!(metadata.focus_points(), None, "{order:?}, {pixels:?}");
            }
        }
    }

    #[test]
    fn a_d600_medium_jpeg_places_the_points_in_its_window() {
        // The Medium JPEG, 4512 by 3008, shows the 6010 by 4006 window
        // centered in the 6016 by 4016 frame, at the origin (3, 5): the
        // frame is the window, and every center moves back by the origin,
        // the box kept. Point 38 lands at (1809, 1700), point 1 at (3005,
        // 2002) and point 27 at (4200, 2303), with the crop off or no
        // CropHiSpeed at all.
        for order in ORDERS {
            for crop in [Some(D600_FX), None] {
                let what = format!("{order:?}, {crop:?}");
                let metadata = d600_jpeg(order, &d600_viewfinder(order), crop, [4512, 3008]);
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list(
                        [6010, 4006],
                        &[(38, [1809, 1700])],
                        38,
                        FocusAreaMode::Single
                    )),
                    "{what}"
                );
                let value = viewfinder(order, "0100", 3, 1, &[0, 26, 37]);
                let metadata = d600_jpeg(order, &value, crop, [4512, 3008]);
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list(
                        [6010, 4006],
                        &[(1, [3005, 2002]), (27, [4200, 2303]), (38, [1809, 1700])],
                        1,
                        FocusAreaMode::Single
                    )),
                    "{what}"
                );
                // The same record read as a raw file, which has no pixels
                // of its own: the table's frame.
                let metadata = shot(order, "NIKON D600", &d600_viewfinder(order), crop);
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list([6016, 4016], &[B1], 38, FocusAreaMode::Single)),
                    "{what}, raw"
                );
            }
            // A DX crop-mode JPEG at the Medium JPEG's size: the crop's
            // window is 3936 by 2624, not the JPEG's size, so none, as at
            // any smaller size.
            let metadata = d600_jpeg(order, &d600_viewfinder(order), Some(D600_DX), [4512, 3008]);
            assert_eq!(metadata.focus_points(), None, "{order:?}, DX");
        }
    }

    /// The points that the D600's table places for a viewfinder shot that
    /// names `primary` and sets the mask's `bits`, with no CropHiSpeed, as
    /// a file that is no crop, in a JPEG of `pixels`, as if the table's
    /// record listed that size as showing a window of `window` of the
    /// frame, so that a test can place points in a window no body has. The
    /// list is given the area mode single point, as the reader gives it.
    fn placed_on_jpeg(
        pixels: [u32; 2],
        window: [u32; 2],
        primary: u8,
        bits: &[usize],
    ) -> Option<FocusPoints> {
        let [width, height] = window;
        let [pixels_width, pixels_height] = pixels;
        let pixels = Size {
            width: pixels_width,
            height: pixels_height,
        };
        // A table's JPEG sizes are `'static`; each call leaks the few bytes
        // of one, which a test can afford.
        let jpeg_sizes = Box::leak(Box::new([points::ConfirmedJpegSize::new(
            pixels,
            Size { width, height },
        )]));
        let table = points::find("NIKON D600")
            .expect("the D600's table")
            .with_jpeg_sizes(jpeg_sizes);
        let mut mask = [0; points::MASK_LEN];
        for &bit in bits {
            mask[bit / 8] |= 1 << (bit % 8);
        }
        let named = NamedPoints {
            layout_code: 3,
            primary,
            mask,
        };
        super::placed(named, &table, None, Some(pixels))
            .map(|points| points.with_area_mode(FocusAreaMode::Single))
    }

    #[test]
    fn a_point_a_jpeg_sizes_window_leaves_out_is_dropped() {
        // No point of the D600's table lies outside its Medium JPEG's
        // window, so this window is narrower than any the D600 has: 2536 by
        // 2624, centered in the frame at the origin (1740, 696), shown by a
        // JPEG of half its size. Point 38 at (72, 1009), whose box of 164
        // crosses the left edge, and point 25 at (2463, 1311), whose box
        // crosses the right one, are dropped; points 1 and 11 stay.
        let expected = placed_list(
            [2536, 2624],
            &[(1, [1268, 1311]), (11, [1028, 1311])],
            1,
            FocusAreaMode::Single,
        );
        assert_eq!(
            placed_on_jpeg([1268, 1312], [2536, 2624], 1, &[10, 24, 37]),
            Some(expected)
        );
        // The primary dropped: no point.
        assert_eq!(placed_on_jpeg([1268, 1312], [2536, 2624], 38, &[37]), None);
        // A window wider than the frame has no origin: no point.
        assert_eq!(placed_on_jpeg([3009, 2008], [6018, 4016], 1, &[0]), None);
    }

    /// What a crop test's file holds besides AFInfo2: the size of its raw
    /// data, its CropHiSpeed's seven values, when it has one, and its
    /// CropArea, when it has one, both as SHORTs.
    #[derive(Clone, Copy, Debug)]
    struct Crop {
        raw: [u32; 2],
        crop_hi_speed: Option<[u16; 7]>,
        crop_area: Option<[u16; 4]>,
    }

    /// The Z6III's 1:1 crop (public samples 7808 and 7812).
    const Z6III_1X1: Crop = Crop {
        raw: [4048, 4040],
        crop_hi_speed: Some([17, 6064, 4040, 4048, 4040, 1008, 0]),
        crop_area: Some([8, 4, 4032, 4032]),
    };

    /// The Z6III's 16:9 crop (public samples 7809 and 7813).
    const Z6III_16X9: Crop = Crop {
        raw: [6064, 3416],
        crop_hi_speed: Some([6, 6064, 4040, 6064, 3416, 0, 312]),
        crop_area: Some([8, 8, 6048, 3400]),
    };

    /// The Z6III's DX crop (public sample 7810).
    const Z6III_DX: Crop = Crop {
        raw: [4000, 2672],
        crop_hi_speed: Some([2, 6064, 4040, 4000, 2672, 1032, 684]),
        crop_area: Some([8, 8, 3984, 2656]),
    };

    /// The Z50II's 1:1 crop (public sample 7767).
    const Z50II_1X1: Crop = Crop {
        raw: [3728, 3728],
        crop_hi_speed: Some([18, 5600, 3728, 3728, 3728, 936, 0]),
        crop_area: Some([8, 8, 3712, 3712]),
    };

    /// The Z50II's 16:9 crop (public sample 7770).
    const Z50II_16X9: Crop = Crop {
        raw: [5600, 3144],
        crop_hi_speed: Some([9, 5600, 3728, 5600, 3144, 0, 292]),
        crop_area: Some([16, 8, 5568, 3128]),
    };

    /// The Z 6's small raw, which is no crop: CropHiSpeed's two sizes are
    /// equal, and CropArea counts the stored pixels (public sample 3587).
    const Z6_SMALL_RAW: Crop = Crop {
        raw: [4548, 3030],
        crop_hi_speed: Some([11, 6064, 4040, 6064, 4040, 0, 0]),
        crop_area: Some([10, 6, 4528, 3018]),
    };

    /// The Z 9's FX mode, which is no crop, as shots made with my Z 9
    /// record it.
    const Z9_FX: Crop = Crop {
        raw: [8280, 5520],
        crop_hi_speed: Some([11, 8280, 5520, 8280, 5520, 0, 0]),
        crop_area: Some([12, 8, 8256, 5504]),
    };

    /// The Z 9's DX crop, as shots made with my Z 9 record it.
    const Z9_DX: Crop = Crop {
        raw: [5408, 3608],
        crop_hi_speed: Some([2, 8280, 5520, 5408, 3608, 1436, 956]),
        crop_area: Some([8, 8, 5392, 3592]),
    };

    /// The Z 9's 1:1 crop, as shots made with my Z 9 record it.
    const Z9_1X1: Crop = Crop {
        raw: [5520, 5520],
        crop_hi_speed: Some([17, 8280, 5520, 5520, 5520, 1380, 0]),
        crop_area: Some([8, 8, 5504, 5504]),
    };

    /// The Z 9's 16:9 crop, as shots made with my Z 9 record it.
    const Z9_16X9: Crop = Crop {
        raw: [8280, 4656],
        crop_hi_speed: Some([6, 8280, 5520, 8280, 4656, 0, 432]),
        crop_area: Some([12, 8, 8256, 4640]),
    };

    /// The record of the Z6III's 1:1 and 16:9 files, version 0401, a valid
    /// point beside a missed focus (public samples 7808, 7809, 7812 and
    /// 7813).
    const Z6III_ASPECT: Recorded = Recorded {
        frame: [6048, 4032],
        center: [3023, 2016],
        area: [163, 174],
        in_focus: 0,
    };

    /// The record of the Z6III's DX file, version 0401 (public sample
    /// 7810).
    const Z6III_DX_RECORD: Recorded = Recorded {
        area: [222, 245],
        ..Z6III_ASPECT
    };

    /// The record of the Z50II's 1:1 file, version 0402 (public sample
    /// 7767).
    const Z50II_1X1_RECORD: Recorded = Recorded {
        frame: [5568, 3712],
        center: [2792, 1868],
        area: [278, 276],
        in_focus: 1,
    };

    /// The record of the Z50II's 16:9 file, version 0402 (public sample
    /// 7770).
    const Z50II_16X9_RECORD: Recorded = Recorded {
        frame: [5568, 3712],
        center: [2789, 1865],
        area: [280, 280],
        in_focus: 1,
    };

    /// The records of the shots made with my Z 9, version 0400, with
    /// a single point placed in the top-left corner of the picture in
    /// each mode: FX, DX, 1:1 and 16:9.
    const Z9_FX_CORNER: Recorded = Recorded {
        frame: [8256, 5504],
        center: [502, 424],
        area: [291, 323],
        in_focus: 1,
    };

    /// See [`Z9_FX_CORNER`].
    const Z9_DX_CORNER: Recorded = Recorded {
        center: [636, 572],
        area: [420, 468],
        ..Z9_FX_CORNER
    };

    /// See [`Z9_FX_CORNER`].
    const Z9_1X1_CORNER: Recorded = Recorded {
        center: [1797, 424],
        ..Z9_FX_CORNER
    };

    /// See [`Z9_FX_CORNER`].
    const Z9_16X9_CORNER: Recorded = Recorded {
        center: [502, 715],
        ..Z9_FX_CORNER
    };

    /// A MakerNote block in byte order `order` holding `crop`'s
    /// CropHiSpeed and CropArea, each when it has one, and the AFInfo2
    /// value `af_info2`, as UNDEFINED.
    fn crop_block(order: Order, crop: Crop, af_info2: &[u8]) -> Vec<u8> {
        tags_block(order, |inner, ifd| {
            if let Some(values) = crop.crop_hi_speed {
                inner.short(ifd, CROP_HI_SPEED, &values);
            }
            if let Some(values) = crop.crop_area {
                inner.short(ifd, CROP_AREA, &values);
            }
            inner.undefined(ifd, AF_INFO2, af_info2);
        })
        .0
    }

    /// The focus of a file in byte order `order` whose raw data is of size
    /// `raw` and whose MakerNote is the first `count` bytes of `block`; the
    /// raw size must be read, since CropArea is checked against it.
    fn focus_in(order: Order, raw: [u32; 2], block: &[u8], count: u64) -> Option<Focus> {
        let [width, height] = raw;
        let size = Size { width, height };
        let metadata = metadata_of(raw_file_of(order, size, block, count, |_, _| {}));
        assert_eq!(metadata.raw_size, Some(size), "{order:?}");
        metadata.focus()
    }

    /// The focus of a file in byte order `order` that holds `crop` and an
    /// AFInfo2 of version `version` recording `recorded`.
    fn crop_focus(order: Order, crop: Crop, version: &str, recorded: Recorded) -> Option<Focus> {
        let block = crop_block(order, crop, &af_info2(order, version, recorded));
        focus_in(order, crop.raw, &block, whole(&block))
    }

    /// The focus of `frame`, `center` and `area`, with `in_focus` as
    /// whether focus was confirmed.
    fn focus_at(
        frame: [u16; 2],
        center: [u16; 2],
        area: [u16; 2],
        in_focus: Option<bool>,
    ) -> Focus {
        let recorded = Recorded {
            frame,
            center,
            area,
            in_focus: 0,
        };
        expected(recorded, in_focus)
    }

    /// `value` on an axis `old` pixels long, on one `new` pixels long, by
    /// the rounding the DX crops are scaled with, written out here apart
    /// from [`Focus::scaled_to`]: to the nearest whole number, a half
    /// rounding up.
    fn scale_by_hand(value: u16, old: u16, new: u16) -> u16 {
        let (value, old, new) = (u64::from(value), u64::from(old), u64::from(new));
        u16::try_from((value * new + old / 2) / old).unwrap()
    }

    #[test]
    fn the_crop_samples_give_the_point_on_the_cropped_picture() {
        // On each, the frame becomes CropArea's size. The 1:1 and 16:9
        // crops keep one side: the center moves back on the other axis by
        // half the difference between the frame and CropArea, 1008 across
        // on the Z6III's 1:1 and 316 down on its 16:9, 928 and 292 on the
        // Z50II's, and the area is kept. The DX crop cuts both sides, and
        // the center and the area are scaled onto CropArea.
        for order in ORDERS {
            for (what, crop, version, recorded, focus) in [
                (
                    "Z6III 1:1",
                    Z6III_1X1,
                    "0401",
                    Z6III_ASPECT,
                    focus_at([4032, 4032], [2015, 2016], [163, 174], Some(false)),
                ),
                (
                    "Z6III 16:9",
                    Z6III_16X9,
                    "0401",
                    Z6III_ASPECT,
                    focus_at([6048, 3400], [3023, 1700], [163, 174], Some(false)),
                ),
                (
                    "Z6III DX",
                    Z6III_DX,
                    "0401",
                    Z6III_DX_RECORD,
                    focus_at([3984, 2656], [1991, 1328], [146, 161], Some(false)),
                ),
                (
                    "Z50II 1:1",
                    Z50II_1X1,
                    "0402",
                    Z50II_1X1_RECORD,
                    focus_at([3712, 3712], [1864, 1868], [278, 276], Some(true)),
                ),
                (
                    "Z50II 16:9",
                    Z50II_16X9,
                    "0402",
                    Z50II_16X9_RECORD,
                    focus_at([5568, 3128], [2789, 1573], [280, 280], Some(true)),
                ),
            ] {
                assert_eq!(
                    crop_focus(order, crop, version, recorded),
                    Some(focus),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn the_z9_shots_with_the_point_in_the_corner_keep_it_there_on_every_crop() {
        // Shots made with my Z 9, a single point placed in the
        // top-left corner of the picture in each mode. FX is no crop,
        // and is read as recorded. The DX shot, scaled onto its 5392 by
        // 3592 CropArea, lies as near the corner as FX, with about its
        // area; the 1:1 and 16:9 shots, moved back by 1376 across and 432
        // down, lie there too, with FX's area.
        for order in ORDERS {
            for (what, crop, recorded, focus) in [
                (
                    "FX",
                    Z9_FX,
                    Z9_FX_CORNER,
                    expected(Z9_FX_CORNER, Some(true)),
                ),
                (
                    "DX",
                    Z9_DX,
                    Z9_DX_CORNER,
                    focus_at([5392, 3592], [415, 373], [274, 305], Some(true)),
                ),
                (
                    "1:1",
                    Z9_1X1,
                    Z9_1X1_CORNER,
                    focus_at([5504, 5504], [421, 424], [291, 323], Some(true)),
                ),
                (
                    "16:9",
                    Z9_16X9,
                    Z9_16X9_CORNER,
                    focus_at([8256, 4640], [502, 283], [291, 323], Some(true)),
                ),
            ] {
                assert_eq!(
                    crop_focus(order, crop, "0400", recorded),
                    Some(focus),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_file_that_is_no_crop_gives_the_point_as_recorded() {
        // The D810 records a cropped width four pixels short of its
        // original on an uncropped file, and writes no CropArea (public
        // sample 2036); it records no position, so the record beside it is
        // made up, a centered point in its 7360 by 4912 frame.
        let d810 = Crop {
            raw: [7380, 4928],
            crop_hi_speed: Some([11, 7384, 4928, 7380, 4928, 0, 0]),
            crop_area: None,
        };
        let d810_record = Recorded {
            frame: [7360, 4912],
            center: [3680, 2456],
            area: [400, 400],
            in_focus: 1,
        };
        for order in ORDERS {
            for (what, crop, version, recorded, in_focus) in [
                // CropHiSpeed's two sizes equal: the Z 6's small raw, whose
                // CropArea is smaller than the frame on both sides (public
                // sample 3587).
                ("Z 6 small raw", Z6_SMALL_RAW, "0300", Z6, None),
                // The Z 9's DX values without a CropHiSpeed.
                (
                    "no CropHiSpeed",
                    Crop {
                        crop_hi_speed: None,
                        ..Z9_DX
                    },
                    "0400",
                    Z9_DX_CORNER,
                    Some(true),
                ),
                ("no CropArea", d810, "0100", d810_record, Some(true)),
            ] {
                assert_eq!(
                    crop_focus(order, crop, version, recorded),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_crop_whose_crop_area_cannot_be_used_gives_none() {
        // The Z6III's 1:1 crop with a CropArea the image-size check
        // refuses, in a raw 4039 wide, one column too narrow for its 4032
        // from column 8; with one of three values; and with one of type
        // UNDEFINED, the eight bytes of its four SHORTs. Then the Z 9's DX
        // crop with its CropArea in a raw one column too narrow. A crop is
        // not read as no crop when its CropArea is there but cannot be
        // used: each gives no point, not the point as recorded in the full
        // frame.
        let narrow = Crop {
            raw: [4039, 4040],
            ..Z6III_1X1
        };
        let narrow_z9 = Crop {
            raw: [5399, 3608],
            ..Z9_DX
        };
        let crop_hi_speed = Z6III_1X1.crop_hi_speed.unwrap();
        let [left, top, width, height] = Z6III_1X1.crop_area.unwrap();
        for order in ORDERS {
            let value = af_info2(order, "0401", Z6III_ASPECT);
            let bytes: Vec<u8> = [left, top, width, height]
                .iter()
                .flat_map(|&v| order.u16(v))
                .collect();
            for (what, raw, block) in [
                (
                    "outside the raw",
                    narrow.raw,
                    crop_block(order, narrow, &value),
                ),
                (
                    "three values",
                    Z6III_1X1.raw,
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &crop_hi_speed)
                            .short(ifd, CROP_AREA, &[left, top, width])
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                ),
                (
                    "UNDEFINED",
                    Z6III_1X1.raw,
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &crop_hi_speed)
                            .undefined(ifd, CROP_AREA, &bytes)
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                ),
                (
                    "the Z 9's DX outside the raw",
                    narrow_z9.raw,
                    crop_block(order, narrow_z9, &af_info2(order, "0400", Z9_DX_CORNER)),
                ),
            ] {
                assert_eq!(
                    focus_in(order, raw, &block, whole(&block)),
                    None,
                    "{order:?}, {what}"
                );
            }
            // The control: the same file with no CropArea tag is read as
            // recorded, as the D810's is.
            let none = Crop {
                crop_area: None,
                ..Z6III_1X1
            };
            assert_eq!(
                crop_focus(order, none, "0401", Z6III_ASPECT),
                Some(expected(Z6III_ASPECT, Some(false))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_without_a_raw_size_gives_none_when_it_has_a_crop_area() {
        // The Z6III's 1:1 crop in a file with no raw IFD, so no raw size
        // to check its CropArea against: a CropArea of SHORTs, and one of
        // type UNDEFINED, each give no point. The control, the same file
        // with no CropArea tag, is read as recorded.
        let crop_hi_speed = Z6III_1X1.crop_hi_speed.unwrap();
        let crop_area = Z6III_1X1.crop_area.unwrap();
        for order in ORDERS {
            let value = af_info2(order, "0401", Z6III_ASPECT);
            let bytes: Vec<u8> = crop_area.iter().flat_map(|&v| order.u16(v)).collect();
            let none = Crop {
                crop_area: None,
                ..Z6III_1X1
            };
            for (what, block, focus) in [
                ("SHORTs", crop_block(order, Z6III_1X1, &value), None),
                (
                    "UNDEFINED",
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &crop_hi_speed)
                            .undefined(ifd, CROP_AREA, &bytes)
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    None,
                ),
                (
                    "no CropArea",
                    crop_block(order, none, &value),
                    Some(expected(Z6III_ASPECT, Some(false))),
                ),
            ] {
                let metadata = metadata_of(with_makernote(order, UNDEFINED, &block, whole(&block)));
                assert_eq!(metadata.raw_size, None, "{order:?}, {what}");
                assert_eq!(metadata.focus(), focus, "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_that_fits_neither_crop_shape_gives_none() {
        // A crop cuts both sides, or keeps exactly one and cuts the other.
        // The Z 6's small raw with a cropped size one pixel larger than
        // its original on each side; the Z 9's DX crop with a cropped
        // height of 5600, larger than its original 5520; and the Z 9's FX
        // mode with a cropped width one pixel larger and the height kept,
        // where CropArea's height equals the frame's. Moved by the window,
        // each point would still lie in the picture; each gives none.
        let z6 = Crop {
            crop_hi_speed: Some([11, 6064, 4040, 6065, 4041, 0, 0]),
            ..Z6_SMALL_RAW
        };
        let z9_dx = Crop {
            crop_hi_speed: Some([2, 8280, 5520, 5408, 5600, 1436, 956]),
            ..Z9_DX
        };
        let z9_dx_record = Recorded {
            center: [6000, 3000],
            ..Z9_DX_CORNER
        };
        let z9_fx = Crop {
            crop_hi_speed: Some([11, 8280, 5520, 8281, 5520, 0, 0]),
            ..Z9_FX
        };
        for order in ORDERS {
            for (what, crop, version, recorded) in [
                ("both sides larger", z6, "0300", Z6),
                ("one side larger, one smaller", z9_dx, "0400", z9_dx_record),
                ("one side larger, one kept", z9_fx, "0400", Z9_FX_CORNER),
            ] {
                assert_eq!(
                    crop_focus(order, crop, version, recorded),
                    None,
                    "{order:?}, {what}"
                );
            }
            // The control: both sides equal, the Z 6's small raw as it is,
            // is no crop, and is read as recorded.
            assert_eq!(
                crop_focus(order, Z6_SMALL_RAW, "0300", Z6),
                Some(expected(Z6, None)),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_hi_speed_of_other_than_seven_shorts_is_no_crop() {
        // The Z 9's DX values short of their last, with one more, and as
        // LONGs, which the reader of numbers reads too; then the seven
        // SHORTs, the control, which scale the point.
        let values = Z9_DX.crop_hi_speed.unwrap();
        let mut eight = values.to_vec();
        eight.push(0);
        for order in ORDERS {
            let value = af_info2(order, "0400", Z9_DX_CORNER);
            for (what, block, focus) in [
                (
                    "six SHORTs",
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &values[..6])
                            .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    expected(Z9_DX_CORNER, Some(true)),
                ),
                (
                    "eight SHORTs",
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &eight)
                            .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    expected(Z9_DX_CORNER, Some(true)),
                ),
                (
                    "seven LONGs",
                    tags_block(order, |inner, ifd| {
                        inner
                            .long(ifd, CROP_HI_SPEED, &values.map(u32::from))
                            .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    expected(Z9_DX_CORNER, Some(true)),
                ),
                (
                    "seven SHORTs",
                    crop_block(order, Z9_DX, &value),
                    focus_at([5392, 3592], [415, 373], [274, 305], Some(true)),
                ),
            ] {
                assert_eq!(
                    focus_in(order, Z9_DX.raw, &block, whole(&block)),
                    Some(focus),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_past_the_block_is_no_crop() {
        for order in ORDERS {
            let values = Z9_DX.crop_hi_speed.unwrap();
            // Written in the order the tags are added, so that CropHiSpeed's
            // values, stored after the others, are the block's last bytes.
            let block = tags_block(order, |inner, ifd| {
                inner
                    .unsorted(ifd)
                    .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                    .undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9_DX_CORNER))
                    .short(ifd, CROP_HI_SPEED, &values);
            })
            .0;
            let bytes: Vec<u8> = values.iter().flat_map(|&v| order.u16(v)).collect();
            assert_eq!(block[block.len() - 14..], bytes, "{order:?}");

            // Declared one byte short: the last value's last byte lies in
            // the file, after the block, and the point is as recorded.
            assert_eq!(
                focus_in(order, Z9_DX.raw, &block, whole(&block) - 1),
                Some(expected(Z9_DX_CORNER, Some(true))),
                "{order:?}"
            );

            // The control: declared whole, the same bytes scale the point.
            assert_eq!(
                focus_in(order, Z9_DX.raw, &block, whole(&block)),
                Some(focus_at([5392, 3592], [415, 373], [274, 305], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_aspect_crop_on_a_small_raw_gives_none() {
        // The Z 9's 1:1 CropHiSpeed with a small raw's CropArea, which
        // counts stored pixels: 4128 high, not the frame's 5504, so the
        // window cannot be placed. The corner shot would move past the
        // window's edge anyway; a centered point would not, and gives none
        // all the same.
        let crop = Crop {
            raw: [4140, 4140],
            crop_area: Some([8, 8, 4128, 4128]),
            ..Z9_1X1
        };
        let centered = Recorded {
            center: [4128, 2752],
            ..Z9_1X1_CORNER
        };
        for order in ORDERS {
            for recorded in [Z9_1X1_CORNER, centered] {
                assert_eq!(
                    crop_focus(order, crop, "0400", recorded),
                    None,
                    "{order:?}, {recorded:?}"
                );
            }
        }
    }

    #[test]
    fn a_dx_crop_on_a_small_raw_is_scaled_onto_its_crop_area() {
        // The Z 9's DX CropHiSpeed with a small raw's CropArea, 4044 by
        // 2694 stored pixels inside a 4056 by 2706 raw.
        let crop = Crop {
            raw: [4056, 2706],
            crop_area: Some([6, 6, 4044, 2694]),
            ..Z9_DX
        };
        let [width, height] = Z9_DX_CORNER.frame;
        let [x, y] = Z9_DX_CORNER.center;
        let [area_width, area_height] = Z9_DX_CORNER.area;
        let by_hand = focus_at(
            [4044, 2694],
            [
                scale_by_hand(x, width, 4044),
                scale_by_hand(y, height, 2694),
            ],
            [
                scale_by_hand(area_width, width, 4044),
                scale_by_hand(area_height, height, 2694),
            ],
            Some(true),
        );
        // What the formula gives: (636 × 4044 + 4128) / 8256 = 312,
        // (572 × 2694 + 2752) / 5504 = 280, (420 × 4044 + 4128) / 8256 =
        // 206 and (468 × 2694 + 2752) / 5504 = 229.
        assert_eq!(
            by_hand,
            focus_at([4044, 2694], [312, 280], [206, 229], Some(true))
        );
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", Z9_DX_CORNER),
                Some(by_hand),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_window_that_cannot_be_placed_gives_none() {
        for order in ORDERS {
            // A CropArea 9000 wide, larger than the 8256 frame, on a crop
            // that keeps the height, inside a raw large enough for it.
            let wide = Crop {
                raw: [9000, 5520],
                crop_area: Some([0, 0, 9000, 5504]),
                ..Z9_1X1
            };
            assert_eq!(
                crop_focus(order, wide, "0400", Z9_1X1_CORNER),
                None,
                "{order:?}, wider than the frame"
            );
            // A center 100 pixels in, which the 1:1 window's origin, 1376
            // pixels in, would take below zero.
            let left = Recorded {
                center: [100, 424],
                ..Z9_1X1_CORNER
            };
            assert_eq!(
                crop_focus(order, Z9_1X1, "0400", left),
                None,
                "{order:?}, left of the window"
            );
        }
    }

    #[test]
    fn a_crop_from_a_frame_with_a_zero_side_gives_none() {
        // A frame with a zero side is refused as recorded, by `Focus::new`,
        // before anything is scaled from it.
        for order in ORDERS {
            for frame in [[0, 5504], [8256, 0]] {
                let recorded = Recorded {
                    frame,
                    ..Z9_DX_CORNER
                };
                assert_eq!(
                    crop_focus(order, Z9_DX, "0400", recorded),
                    None,
                    "{order:?}, {frame:?}"
                );
            }
        }
    }

    #[test]
    fn a_dx_crop_rounds_to_the_nearest_pixel_a_half_rounding_up() {
        // A 1000 by 1000 frame scaled onto a 500 by 400 CropArea: by a half
        // across and by 0.4 down. Across, 301 × 0.5 = 150.5 rounds up to
        // 151 and 101 × 0.5 = 50.5 to 51; down, 501 × 0.4 = 200.4 rounds
        // down to 200 and 101 × 0.4 = 40.4 to 40.
        let crop = Crop {
            raw: [500, 400],
            crop_hi_speed: Some([2, 1000, 1000, 500, 400, 0, 0]),
            crop_area: Some([0, 0, 500, 400]),
        };
        let recorded = Recorded {
            frame: [1000, 1000],
            center: [301, 501],
            area: [101, 101],
            in_focus: 1,
        };
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", recorded),
                Some(focus_at([500, 400], [151, 200], [51, 40], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_keeps_an_area_side_of_one_pixel() {
        // A single pixel in the Z 9's frame, scaled onto a 1000 by 1000
        // CropArea: 1000 / 8256 and 1000 / 5504 of a pixel round to
        // nothing, and each side keeps one.
        let crop = Crop {
            raw: [1000, 1000],
            crop_hi_speed: Some([2, 8280, 5520, 1000, 1000, 0, 0]),
            crop_area: Some([0, 0, 1000, 1000]),
        };
        let recorded = Recorded {
            center: [4128, 2752],
            area: [1, 1],
            ..Z9_FX_CORNER
        };
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", recorded),
                Some(focus_at([1000, 1000], [500, 500], [1, 1], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_trims_an_area_that_rounding_leaves_a_pixel_too_wide() {
        // The Z 9's DX crop, with a center 146 across and an area 292 wide,
        // which fits the frame as recorded: half of 292 is 146. Scaled onto
        // the 5392 by 3592 CropArea, x is (146 × 5392 + 4128) / 8256 = 95
        // and the width (292 × 5392 + 4128) / 8256 = 191, whose half
        // rounds up to 96, one more than 95: the width is trimmed to 190,
        // twice the center's distance from the left edge. Down, the corner
        // shot's (572 × 3592 + 2752) / 5504 = 373 and (468 × 3592 + 2752)
        // / 5504 = 305 fit as they are.
        let near_the_edge = Recorded {
            center: [146, 572],
            area: [292, 468],
            ..Z9_DX_CORNER
        };
        let [width, _] = Z9_DX_CORNER.frame;
        assert_eq!(
            (
                scale_by_hand(146, width, 5392),
                scale_by_hand(292, width, 5392)
            ),
            (95, 191)
        );
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, Z9_DX, "0400", near_the_edge),
                Some(focus_at([5392, 3592], [95, 373], [190, 305], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_whose_center_lies_outside_the_recorded_frame_gives_none() {
        // A center at x = 8257, one pixel past the right edge of the 8256
        // frame, which `Focus::new` refuses as recorded. On the Z 9's DX
        // CropArea it would scale past the right edge anyway, to 5393; on
        // a small raw's CropArea, 4044 wide, it would scale to (8257 × 4044
        // + 4128) / 8256 = 4044, on the right edge, and only its area would
        // be lost. The control, the corner shot, gives the point on both.
        let small_raw = Crop {
            raw: [4056, 2706],
            crop_area: Some([6, 6, 4044, 2694]),
            ..Z9_DX
        };
        let outside = Recorded {
            center: [8257, 572],
            ..Z9_DX_CORNER
        };
        let [width, _] = Z9_DX_CORNER.frame;
        assert_eq!(scale_by_hand(8257, width, 4044), 4044);
        for order in ORDERS {
            for crop in [Z9_DX, small_raw] {
                assert_eq!(
                    crop_focus(order, crop, "0400", outside),
                    None,
                    "{order:?}, {crop:?}"
                );
                assert!(
                    crop_focus(order, crop, "0400", Z9_DX_CORNER).is_some(),
                    "{order:?}, {crop:?}"
                );
            }
        }
    }

    #[test]
    fn a_dx_crop_whose_area_does_not_fit_as_recorded_gives_none() {
        // A center 145 across with an area 291 wide: half of 291 rounds up
        // to 146, one more than 145, so the area does not fit the frame as
        // recorded. Scaled onto the Z 9's DX CropArea first, it would:
        // (145 × 5392 + 4128) / 8256 = 95 and (291 × 5392 + 4128) / 8256 =
        // 190, whose half is 95.
        let too_wide = Recorded {
            center: [145, 572],
            area: [291, 468],
            ..Z9_DX_CORNER
        };
        let [width, _] = Z9_DX_CORNER.frame;
        assert_eq!(
            (
                scale_by_hand(145, width, 5392),
                scale_by_hand(291, width, 5392)
            ),
            (95, 190)
        );
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, Z9_DX, "0400", too_wide),
                None,
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_that_rounds_the_center_onto_an_edge_keeps_the_point_alone() {
        // A 1000 by 1000 frame scaled onto a 400 by 400 CropArea, by 0.4:
        // a center 1 across, with an area 2 wide, which fits as recorded,
        // rounds to (1 × 400 + 500) / 1000 = 0, on the left edge, where no
        // area fits, and the point is kept without one. Down, (500 × 400 +
        // 500) / 1000 = 200.
        let crop = Crop {
            raw: [400, 400],
            crop_hi_speed: Some([2, 1000, 1000, 400, 400, 0, 0]),
            crop_area: Some([0, 0, 400, 400]),
        };
        let recorded = Recorded {
            frame: [1000, 1000],
            center: [1, 500],
            area: [2, 2],
            in_focus: 1,
        };
        let frame = Size {
            width: 400,
            height: 400,
        };
        let point = Focus::new(frame, Point { x: 0, y: 200 }, FocusOrigin::Recorded)
            .expect("a center on the frame")
            .with_in_focus(true);
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", recorded),
                Some(point),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_whose_area_no_longer_fits_gives_none() {
        // The 16:9 shot's center, 715 down, with an area 900 high, which
        // fits the full frame: moved back to 283, the 450 above it would
        // leave the window. The control, the shot's own 323, fits.
        for order in ORDERS {
            let tall = Recorded {
                area: [291, 900],
                ..Z9_16X9_CORNER
            };
            assert_eq!(crop_focus(order, Z9_16X9, "0400", tall), None, "{order:?}");
            assert_eq!(
                crop_focus(order, Z9_16X9, "0400", Z9_16X9_CORNER),
                Some(focus_at([8256, 4640], [502, 283], [291, 323], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_default_crop_leaves_the_crop_area_to_the_focus() {
        // The image size is the default crop, which comes first; CropArea
        // still gives the cropped picture the point is placed on.
        for order in ORDERS {
            let block = crop_block(order, Z6III_1X1, &af_info2(order, "0401", Z6III_ASPECT));
            let [width, height] = Z6III_1X1.raw;
            let raw = Size { width, height };
            let file = raw_file_of(order, raw, &block, whole(&block), |builder, ifd0| {
                builder.long(ifd0, DEFAULT_CROP_ORIGIN, &[0, 0]).long(
                    ifd0,
                    DEFAULT_CROP_SIZE,
                    &[4000, 4000],
                );
            });
            let metadata = metadata_of(file);
            assert_eq!(
                metadata.image_size,
                Some(Size {
                    width: 4000,
                    height: 4000
                }),
                "{order:?}"
            );
            assert_eq!(
                metadata.focus(),
                Some(focus_at(
                    [4032, 4032],
                    [2015, 2016],
                    [163, 174],
                    Some(false)
                )),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = crop_block(
                inner_order,
                Z9_DX,
                &af_info2(inner_order, "0400", Z9_DX_CORNER),
            );
            assert_eq!(
                focus_in(file, Z9_DX.raw, &block, whole(&block)),
                Some(focus_at([5392, 3592], [415, 373], [274, 305], Some(true))),
                "{file:?}"
            );
        }
    }

    /// `crop` as a camera's JPEG records it: with the raw file's
    /// CropHiSpeed and no CropArea, as the Z 9's JPEGs do (ADR015).
    fn in_a_jpeg(crop: Crop) -> Crop {
        Crop {
            crop_area: None,
            ..crop
        }
    }

    /// The metadata of a JPEG whose MakerNote, in byte order `order`, holds
    /// `crop`'s CropHiSpeed and CropArea, each when it has one, and an
    /// AFInfo2 of version 0400 recording `recorded`, inside a file in byte
    /// order `file` that holds no raw data, read as [`jpeg_metadata_of`]
    /// reads it with the picture `pixels`.
    fn jpeg_metadata(
        file: Order,
        order: Order,
        crop: Crop,
        recorded: Recorded,
        pixels: [u32; 2],
    ) -> Metadata {
        let block = crop_block(order, crop, &af_info2(order, "0400", recorded));
        jpeg_metadata_of(
            with_makernote(file, UNDEFINED, &block, whole(&block)),
            pixels,
        )
    }

    /// The focus point of the JPEG [`jpeg_metadata`] reads, its file and
    /// its MakerNote both in byte order `order`.
    fn jpeg_focus(order: Order, crop: Crop, recorded: Recorded, pixels: [u32; 2]) -> Option<Focus> {
        jpeg_metadata(order, order, crop, recorded, pixels).focus()
    }

    #[test]
    fn a_jpegs_dx_crop_is_scaled_onto_its_pixels() {
        // A DX crop records fractions of the cropped picture in the full
        // frame's units, so its point is scaled onto the JPEG's pixels, as
        // onto CropArea in a raw file, and the frame becomes the JPEG's
        // size. At full size the JPEG is the raw file's CropArea, 5392 by
        // 3592, and the point is the raw file's (415, 373), 274 by 305; at
        // the middle size, 4032 by 2688, the recorded point is scaled onto
        // that, by the rounding written out in `scale_by_hand`.
        let [x, y] = Z9_DX_CORNER.center;
        let [width, height] = Z9_DX_CORNER.area;
        let middle = focus_at(
            [4032, 2688],
            [scale_by_hand(x, 8256, 4032), scale_by_hand(y, 5504, 2688)],
            [
                scale_by_hand(width, 8256, 4032),
                scale_by_hand(height, 5504, 2688),
            ],
            Some(true),
        );
        for (file, order) in [
            (Order::Little, Order::Little),
            (Order::Big, Order::Big),
            (Order::Little, Order::Big),
            (Order::Big, Order::Little),
        ] {
            for (pixels, focus) in [
                (
                    [5392, 3592],
                    focus_at([5392, 3592], [415, 373], [274, 305], Some(true)),
                ),
                ([4032, 2688], middle),
            ] {
                assert_eq!(
                    jpeg_metadata(file, order, in_a_jpeg(Z9_DX), Z9_DX_CORNER, pixels).focus(),
                    Some(focus),
                    "{file:?}, MakerNote {order:?}, {pixels:?}"
                );
            }
        }
    }

    #[test]
    fn a_jpegs_aspect_crop_is_moved_into_the_window_of_its_shape() {
        // A 1:1 or 16:9 crop records a position in the full 8256 by 5504
        // frame. The JPEG's picture is the centered window of that frame
        // with the frame's length on the kept side and the JPEG's shape.
        // On 1:1, the height is kept: the window is 5504 times the JPEG's
        // width over its height wide, 5504 at every size, so 5504 by 5504,
        // whose origin is (8256 - 5504) / 2 = 1376 across, and the recorded
        // (1797, 424) moves to (421, 424). On 16:9 the width is kept: the
        // window is 8256 times the JPEG's height over its width high, 4640
        // at every size (8256 x 4640 / 8256, x 3480 / 6192, x 2320 /
        // 4128), whose origin is (5504 - 4640) / 2 = 432 down, and the
        // recorded (502, 715) moves to (502, 283). The area is kept, and
        // the frame is the window, whatever the JPEG's size.
        let square = focus_at([5504, 5504], [421, 424], [291, 323], Some(true));
        let wide = focus_at([8256, 4640], [502, 283], [291, 323], Some(true));
        for order in ORDERS {
            for (what, crop, recorded, pixels, focus) in [
                ("1:1", Z9_1X1, Z9_1X1_CORNER, [5504, 5504], square),
                ("1:1", Z9_1X1, Z9_1X1_CORNER, [4128, 4128], square),
                ("1:1", Z9_1X1, Z9_1X1_CORNER, [2752, 2752], square),
                ("16:9", Z9_16X9, Z9_16X9_CORNER, [8256, 4640], wide),
                ("16:9", Z9_16X9, Z9_16X9_CORNER, [6192, 3480], wide),
                ("16:9", Z9_16X9, Z9_16X9_CORNER, [4128, 2320], wide),
            ] {
                assert_eq!(
                    jpeg_focus(order, in_a_jpeg(crop), recorded, pixels),
                    Some(focus),
                    "{order:?}, {what}, {pixels:?}"
                );
            }
        }
    }

    #[test]
    fn a_jpegs_window_rounds_to_the_nearest_pixel_a_half_rounding_up() {
        // A 16:9 crop, which keeps the frame's 8256 width. On a JPEG of
        // 1000 by 563, the window is 8256 x 563 / 1000 = 4648.128 high,
        // 4648 to the nearest pixel, whose origin is (5504 - 4648) / 2 =
        // 428; on one of 128 by 71, it is 8256 x 71 / 128 = 4579.5, a half,
        // 4580, whose origin is (5504 - 4580) / 2 = 462. The recorded y,
        // 715, moves back by the origin.
        for order in ORDERS {
            for (pixels, focus) in [
                (
                    [1000, 563],
                    focus_at([8256, 4648], [502, 287], [291, 323], Some(true)),
                ),
                (
                    [128, 71],
                    focus_at([8256, 4580], [502, 253], [291, 323], Some(true)),
                ),
            ] {
                assert_eq!(
                    jpeg_focus(order, in_a_jpeg(Z9_16X9), Z9_16X9_CORNER, pixels),
                    Some(focus),
                    "{order:?}, {pixels:?}"
                );
            }
        }
    }

    #[test]
    fn a_jpegs_window_longer_than_the_frame_gives_none() {
        // A 16:9 crop on a 4:3 JPEG of 4128 by 3096: the window would be
        // 8256 x 3096 / 4128 = 6192 high, more than the frame's 5504. A 1:1
        // crop on a 16:9 JPEG of 1920 by 1080: it would be 5504 x 1920 /
        // 1080, 9785 wide, more than the frame's 8256.
        for order in ORDERS {
            for (what, crop, recorded, pixels) in [
                ("16:9 on 4:3", Z9_16X9, Z9_16X9_CORNER, [4128, 3096]),
                ("1:1 on 16:9", Z9_1X1, Z9_1X1_CORNER, [1920, 1080]),
            ] {
                assert_eq!(
                    jpeg_focus(order, in_a_jpeg(crop), recorded, pixels),
                    None,
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_jpeg_side_of_zero_gives_no_window() {
        // The JPEG reader never passes a side of zero, since the claim
        // refuses such a frame header, but the window divides by the
        // JPEG's sides, so it refuses one itself rather than divide by
        // zero.
        let frame = Size {
            width: 8256,
            height: 5504,
        };
        for cut in [Cut::Width, Cut::Height] {
            for (width, height) in [(0, 2752), (2752, 0), (0, 0)] {
                assert_eq!(
                    window_of_shape(frame, cut, Size { width, height }),
                    None,
                    "{cut:?}, {width} x {height}"
                );
            }
        }
    }

    #[test]
    fn a_jpegs_crop_area_gives_no_image_size_and_no_point_on_a_crop() {
        // A MakerNote copied from a raw file into an export holds the raw
        // file's CropArea. A JPEG has no raw size to check it against: it
        // never gives the image size, which stays the JPEG's own, and a
        // crop gives no point, as in a raw file whose CropArea cannot be
        // checked. A file that is no crop gives its point as recorded.
        for order in ORDERS {
            for (what, crop, recorded, pixels, focus) in [
                ("DX", Z9_DX, Z9_DX_CORNER, [5392, 3592], None),
                ("1:1", Z9_1X1, Z9_1X1_CORNER, [5504, 5504], None),
                (
                    "FX, an export cut to portrait",
                    Z9_FX,
                    Z9_FX_CORNER,
                    [3648, 5472],
                    Some(expected(Z9_FX_CORNER, Some(true))),
                ),
            ] {
                let metadata = jpeg_metadata(order, order, crop, recorded, pixels);
                let [width, height] = pixels;
                assert_eq!(
                    metadata.image_size,
                    Some(Size { width, height }),
                    "{order:?}, {what}"
                );
                assert_eq!(metadata.focus(), focus, "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_that_fits_neither_shape_gives_none_in_a_jpeg_only() {
        // A cropped width one pixel larger than the original, and a
        // cropped height smaller, with no CropArea: a JPEG has nothing to
        // place the point on, while a raw file without a CropArea is read
        // as recorded, whatever its CropHiSpeed says.
        let neither = Crop {
            raw: [8280, 5520],
            crop_hi_speed: Some([2, 8280, 5520, 8281, 3608, 0, 0]),
            crop_area: None,
        };
        for order in ORDERS {
            assert_eq!(
                jpeg_focus(order, neither, Z9_FX_CORNER, [8256, 5504]),
                None,
                "{order:?}, JPEG"
            );
            assert_eq!(
                crop_focus(order, neither, "0400", Z9_FX_CORNER),
                Some(expected(Z9_FX_CORNER, Some(true))),
                "{order:?}, raw"
            );
        }
    }

    #[test]
    fn a_jpeg_that_is_no_crop_gives_the_point_as_recorded() {
        // The Z 9's FX JPEG: CropHiSpeed's two sizes equal, no CropArea.
        for order in ORDERS {
            assert_eq!(
                jpeg_focus(order, in_a_jpeg(Z9_FX), Z9_FX_CORNER, [8256, 5504]),
                Some(expected(Z9_FX_CORNER, Some(true))),
                "{order:?}"
            );
        }
    }
}
