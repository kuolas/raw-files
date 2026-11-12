// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote's AFInfo2, read at each version's offsets, and
//! the points a shot through a DSLR's viewfinder names by number.
//!
//! AFInfo2 records where the camera's autofocus aimed, which no standard
//! tag records. Mirrorless bodies, and DSLRs in live view, fill in a
//! position in pixels: the size of a frame, the center of the focus area
//! in that frame, and the area's width and height. A DSLR shot through the
//! viewfinder records a zero frame and area instead, and names its points
//! by number, which only a table of the body's points, carried by no file,
//! places in pixels (public samples 928 and 1835): a body with a table
//! gives them placed from it, as [`crop`](super::crop) says, and any other
//! body no focus point. The layout of the body's points, which every file
//! of a body whose table is in raw-files ([`points`]), the D600, has
//! whatever the shot
//! ([`Metadata::focus_layout`](crate::Metadata::focus_layout)), is not
//! this reader's to give: detection gives it from the Model alone,
//! whatever the MakerNote holds, even when it cannot be read.
//! The position is the center of the area, not its corner: the Coolpix A,
//! focusing at the middle of its 4928 by 3264 frame, records (2464, 1632)
//! (sample 2238). The frame is the camera's full-size output, not the
//! stored raw: 8256 by 5504 on the Z 9, inside an 8280 by 5520 raw
//! (samples 5146 to 5148), and the full 6048 by 4024 on the Z 6's small
//! raw, whose stored image is 4548 by 3030 (sample 3587); the point is
//! reported in that frame, as recorded. AFInfo2 is a value of type
//! UNDEFINED that starts with a version, four ASCII digits, and each
//! version keeps its fields at offsets of its own, as ExifTool's tables
//! publish them. It is read at the exact versions 0100, 0101, 0300, 0301,
//! 0400, 0401 and 0402; any other, such as the Nikon 1's 0200, gives no
//! focus point, since its fields may lie elsewhere (ADR012). The frame,
//! the center and the area are SHORTs in the MakerNote's byte order. On
//! every version read but 0300 and 0301, a byte says whether the camera
//! confirmed focus: 1 for yes, 0 for a miss, and any other value says
//! nothing. On every version read, byte 5 records the area mode, how the
//! camera chose its points, by a code read with the detection method at
//! byte 4: the codes that frames shot in a known mode confirmed, under the
//! method recorded, name a [`FocusAreaMode`] ([`area_mode`]), and any other
//! names none. A recorded position is given only when the value is at
//! least as long as its version's layout, the frame and the area have no
//! zero side, the center lies on the frame, and the area fits inside the
//! frame around it ([`Focus::new`], [`Focus::with_size`]); it is given as
//! a list of one point, the primary, with the area mode ([`FocusPoints`]).
//! FocusMode, ASCII text, records how the camera focused: `MANUAL`, or an
//! autofocus mode padded with spaces, such as `AF-S  `. When it reads
//! exactly `MANUAL`, by the text rule, the file gives no focus point
//! ([`manual_focus`]), whatever AFInfo2 holds: the point of a
//! manual-focus shot is not one the camera focused on (ADR012).
//!
//! [`FocusPoints`]: crate::focus::FocusPoints
//! [`manual_focus`]: super::manual_focus

use crate::focus::{Focus, FocusAreaMode, FocusOrigin};
use crate::formats::makernote::points;
use crate::geometry::{Point, Size};
use crate::io::ByteOrder;

/// Where one AFInfo2 version keeps the focus point: the byte offsets, from
/// the start of the value, of the fields read, and the shortest value that
/// version writes.
///
/// Each of the frame, the center and the area is two SHORTs in the
/// MakerNote's byte order, the width or x at its offset and the height or y
/// two bytes after it. The offsets are those of ExifTool's published
/// tables, confirmed on the public samples whose fields hold values. The
/// area mode is not here: it is byte 5, read with the detection method at
/// byte 4, on every version, which [`af_info2_record`] reads.
///
/// The points a record with no position names, the layout code, the
/// primary point's number and the mask of the points used, are read on
/// version 0100 alone, where a table's frames checked them: the other
/// versions keep them at offsets of their own, which the NEF notes record
/// and no table yet checks.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    /// How long a value of this version is, at least: the length of the
    /// version's layout, whose last fields may not be read.
    min_len: usize,
    /// The frame's width, then its height.
    frame: usize,
    /// The center's x, then its y.
    center: usize,
    /// The area's width, then its height.
    area: usize,
    /// The byte that says whether focus was confirmed, on the versions
    /// that have one.
    in_focus: Option<usize>,
    /// The layout code, followed by the primary point's number and the
    /// mask of the points used ([`NamedPoints`]), on the version read for
    /// them, 0100.
    named: Option<usize>,
}

/// The layout of AFInfo2 version `version`, its first four bytes: `None`
/// for any version but the exact ones read, 0100, 0101, 0300, 0301, 0400,
/// 0401 and 0402. A version that shares its first two digits with these,
/// such as 0200 or a future 0403, may have moved its fields, and is not
/// read at the offsets of its neighbors (ADR012).
pub(super) fn layout(version: &[u8]) -> Option<Layout> {
    match version {
        b"0100" => Some(Layout {
            min_len: 30,
            frame: 16,
            center: 20,
            area: 24,
            in_focus: Some(28),
            named: Some(6),
        }),
        b"0101" => Some(Layout {
            min_len: 84,
            frame: 70,
            center: 74,
            area: 78,
            in_focus: Some(82),
            named: None,
        }),
        b"0300" | b"0301" => Some(Layout {
            min_len: 60,
            frame: 42,
            center: 46,
            area: 50,
            in_focus: None,
            named: None,
        }),
        b"0400" | b"0401" | b"0402" => Some(Layout {
            min_len: 80,
            frame: 62,
            center: 66,
            area: 70,
            in_focus: Some(74),
            named: None,
        }),
        _ => None,
    }
}

/// What an AFInfo2 value records of the focus point, read but not yet
/// checked: the frame, the center of the focus area in it, the area's
/// size, whether focus was confirmed, on the versions that say, the area
/// mode, when its code names one, and, on the version read for them, the
/// points the record names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FocusRecord {
    /// The frame the center and the area are measured in.
    pub(super) frame: Size,
    /// The center of the focus area.
    pub(super) center: Point,
    /// The focus area's width and height.
    pub(super) area: Size,
    /// Whether focus was confirmed: `None` when the version has no byte
    /// for it, or the byte is neither 1 nor 0.
    pub(super) in_focus: Option<bool>,
    /// How the camera chose its points: `None` when the area-mode byte,
    /// under the detection method recorded, holds a code [`area_mode`]
    /// does not name.
    pub(super) area_mode: Option<FocusAreaMode>,
    /// The points the record names, read on version 0100 alone: what a
    /// record with no position gives, placed from a table.
    pub(super) named: Option<NamedPoints>,
}

impl FocusRecord {
    /// Whether the record records a position: whether its frame and its
    /// area have no zero side. A shot through a DSLR's viewfinder writes a
    /// zero frame and area, and names its points instead
    /// ([`NamedPoints`]). A record of a position is checked further, as
    /// recorded, by [`focus`](Self::focus), and one that fails gives no
    /// point, whatever it names.
    pub(super) fn records_position(self) -> bool {
        let Self { frame, area, .. } = self;
        frame.width != 0 && frame.height != 0 && area.width != 0 && area.height != 0
    }

    /// The focus point this record describes: `None` when [`Focus::new`]
    /// or [`Focus::with_size`] refuses it, for a frame or an area with a
    /// zero side, which is how a shot through a DSLR's viewfinder says it
    /// records no position, a center outside the frame, or an area that
    /// does not fit inside the frame around its center.
    ///
    /// [`in_crop`] calls it on the record as recorded, on every record of
    /// a position ([`records_position`](Self::records_position)), and on a
    /// crop that keeps one side again, on the record moved into the cropped
    /// picture.
    ///
    /// [`in_crop`]: super::crop::in_crop
    pub(super) fn focus(self) -> Option<Focus> {
        // An area that `with_size` refuses costs the whole focus, not only
        // the area. A zero side is how a Nikon file says it records no
        // position; and an area of no zero side that does not fit around
        // the center is a record that cannot be used, which gives no focus
        // on every format, as ADR012 decides.
        let focus = Focus::new(self.frame, self.center, FocusOrigin::Recorded)
            .and_then(|focus| focus.with_size(self.area))
            .ok()?;
        Some(match self.in_focus {
            Some(in_focus) => focus.with_in_focus(in_focus),
            None => focus,
        })
    }
}

/// What the AFInfo2 value `value` records, its SHORTs in byte order
/// `order`: the frame, the center and the area at its version's
/// [`layout`]; the in-focus byte, when the layout has one, as `true` for 1,
/// `false` for 0, and nothing said for any other value; the area mode
/// that byte 5 names under the detection method of byte 4, both the same
/// on every version ([`area_mode`]); and, when the layout keeps them, on
/// version 0100, the points the record names ([`named_points`]).
///
/// `None` when the version is not one read, or the value is shorter than
/// its layout. The values are not checked here: [`in_crop`] checks a
/// position, as recorded, with [`FocusRecord::focus`], before it places
/// it, and [`placed`] the points named against the body's table.
///
/// [`in_crop`]: super::crop::in_crop
/// [`placed`]: super::crop::placed
pub(super) fn af_info2_record(value: &[u8], order: ByteOrder) -> Option<FocusRecord> {
    let layout = layout(value.get(..4)?)?;
    if value.len() < layout.min_len {
        return None;
    }
    let (width, height) = shorts_at(value, layout.frame, order)?;
    let frame = Size { width, height };
    let (x, y) = shorts_at(value, layout.center, order)?;
    let (width, height) = shorts_at(value, layout.area, order)?;
    let area = Size { width, height };
    let in_focus = layout
        .in_focus
        .and_then(|at| value.get(at))
        .and_then(|&byte| match byte {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        });
    // Every layout is longer than six bytes, so both bytes are always
    // there.
    let area_mode = match (value.get(4), value.get(5)) {
        (Some(&detection), Some(&code)) => area_mode(detection, code),
        _ => None,
    };
    let named = layout.named.and_then(|at| named_points(value, at));
    Some(FocusRecord {
        frame,
        center: Point { x, y },
        area,
        in_focus,
        area_mode,
        named,
    })
}

/// What a record with no position names of the points the camera used: a
/// shot through a DSLR's viewfinder writes the layout of points the
/// numbers belong to, the primary point's number and a mask of the points
/// used, which only a table of the body's points places ([`placed`]).
/// Read on version 0100 alone ([`Layout::named`]), where the D600's frames
/// checked them.
///
/// [`placed`]: super::crop::placed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NamedPoints {
    /// The layout code, AFInfo2's FocusPointSchema, byte 6: 3 for the
    /// D600's 39 points, and 0, no layout, on a live-view or manual-focus
    /// shot.
    pub(super) layout_code: u8,
    /// The primary point's number, byte 7: the point the camera focused
    /// on, and 0 for none.
    pub(super) primary: u8,
    /// The mask of the points used, bytes 8 to 14, whose bits the table's
    /// bit order ties to its points ([`PointTable::bit`]).
    ///
    /// [`PointTable::bit`]: points::PointTable::bit
    pub(super) mask: [u8; points::MASK_LEN],
}

impl NamedPoints {
    /// Whether the mask sets bit `bit`, counted from the least significant
    /// bit of its first byte: bit `bit % 8` of byte `bit / 8`. `false` for
    /// a bit past the mask.
    pub(super) fn marks(self, bit: usize) -> bool {
        self.mask
            .get(bit / 8)
            .is_some_and(|&byte| (byte >> (bit % 8)) & 1 == 1)
    }
}

/// The points that the AFInfo2 value `value` names at `at`, where its
/// layout keeps them ([`Layout::named`]): the layout code at `at`, the
/// primary point's number after it, and the mask of the points used after
/// that; `None` when they do not all lie inside the value.
fn named_points(value: &[u8], at: usize) -> Option<NamedPoints> {
    let (&[layout_code, primary], rest) = value.get(at..)?.split_first_chunk::<2>()?;
    Some(NamedPoints {
        layout_code,
        primary,
        mask: *rest.first_chunk()?,
    })
}

/// The area mode, how the camera chose its points, that AFInfo2's
/// area-mode byte, byte 5, names with the code `code`, read under the
/// detection method that byte 4 records as `detection`; `None` for any
/// other pair, whose meaning no file shot in a known mode has shown, as
/// the decision to name only confirmed area modes requires (ADR018).
///
/// The detection method is how the camera found focus: 0 by phase
/// detection, through a DSLR's viewfinder; 1 by contrast detection, in a
/// DSLR's live view; and 2 by both, hybrid, on the mirrorless bodies.
/// ExifTool's published documentation, read as documentation, gives byte 5
/// two lists, told apart by that method. The codes a DSLR writes through
/// the viewfinder mean one thing under phase detection and another under
/// contrast detection, where 0 is plain contrast-detect, not a single
/// point. The codes 193 and 197 are on both lists with one meaning, which
/// is why they are named under any detection method. The code 207 is on
/// the contrast-detect list and in the one list of version 04xx only, so
/// it is named under contrast or hybrid detection, the detection the Z 9
/// files record.
///
/// Named, each on frames shot in its mode, as the NEF format notes record:
///
/// - under phase detection, 0 [`Single`](FocusAreaMode::Single), 5
///   [`Dynamic`](FocusAreaMode::Dynamic), 8 [`Auto`](FocusAreaMode::Auto)
///   and 9 [`Tracking`](FocusAreaMode::Tracking), on my D600 frames
///   through the viewfinder, in single-point, dynamic-area, auto-area and
///   3D-tracking AF;
/// - under any detection method, 193 [`Single`](FocusAreaMode::Single), on
///   the two Z 9 frames shot with a single point and committed as
///   fixtures, both under hybrid detection, and 197
///   [`Auto`](FocusAreaMode::Auto), on my Z 9 files shot in auto-area AF;
/// - under contrast or hybrid detection, 207
///   [`Tracking`](FocusAreaMode::Tracking), on my Z 9 files shot in
///   3D-tracking, under hybrid detection.
///
/// A DSLR code under contrast or hybrid detection, 207 under phase
/// detection, or any other code gives `None`.
pub(super) fn area_mode(detection: u8, code: u8) -> Option<FocusAreaMode> {
    match (detection, code) {
        (0, 0) | (_, 193) => Some(FocusAreaMode::Single),
        (0, 5) => Some(FocusAreaMode::Dynamic),
        (0, 8) | (_, 197) => Some(FocusAreaMode::Auto),
        (0, 9) | (1 | 2, 207) => Some(FocusAreaMode::Tracking),
        _ => None,
    }
}

/// The two SHORTs at `at` in `value`, in byte order `order`; `None` when
/// they do not lie inside it.
fn shorts_at(value: &[u8], at: usize, order: ByteOrder) -> Option<(u32, u32)> {
    let &[first0, first1, second0, second1] = value.get(at..)?.first_chunk::<4>()?;
    Some((
        u32::from(order.u16([first0, first1])),
        u32::from(order.u16([second0, second1])),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::testing::{
        B1, D600_FX, FIELDS_03XX, FIELDS_04XX, FIELDS_0100, FIELDS_0101, Fields, ORDERS, Recorded,
        Z6, Z9, af_info2, af_info2_with, d600_viewfinder, exif_file, exif_metadata, expected,
        makernote_metadata, metadata_of, naming, placed_list, retyped, shot, tags_block, whole,
    };
    use super::super::{AF_INFO2, FOCUS_MODE};
    use super::*;
    use crate::focus::FocusPoints;
    use crate::tiff_builder::{BYTE, IfdId, Order, TiffBuilder, UNDEFINED};

    /// The Coolpix A's record, version 0100: a live-view shot focused at
    /// the middle of its frame, and confirmed (public sample 2238).
    const COOLPIX_A: Recorded = Recorded {
        frame: [4928, 3264],
        center: [2464, 1632],
        area: [408, 336],
        in_focus: 1,
    };

    /// A record for version 0101, made up, since no public sample fills in
    /// its position: a D850-sized frame, off-center, confirmed.
    const MADE_UP_0101: Recorded = Recorded {
        frame: [8256, 5504],
        center: [3000, 2000],
        area: [400, 400],
        in_focus: 1,
    };

    /// The focus of a file in byte order `order` whose MakerNote holds what
    /// `tags` adds to its MakerNote IFD.
    fn focus_of(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> Option<Focus> {
        makernote_metadata(order, tags).focus()
    }

    /// The focus of a file in byte order `order` whose MakerNote holds the
    /// AFInfo2 value `value`, as UNDEFINED, and no FocusMode.
    fn focus_from(order: Order, value: &[u8]) -> Option<Focus> {
        focus_of(order, |inner, ifd| {
            inner.undefined(ifd, AF_INFO2, value);
        })
    }

    #[test]
    fn each_af_info2_layout_gives_the_focus() {
        // The Coolpix A's 0100, a made-up 0101, the Z 6's 0300, which does
        // not say whether focus was confirmed, and the Z 9's 0400.
        for order in ORDERS {
            for (version, recorded, in_focus) in [
                ("0100", COOLPIX_A, Some(true)),
                ("0101", MADE_UP_0101, Some(true)),
                ("0300", Z6, None),
                ("0400", Z9, Some(true)),
            ] {
                assert_eq!(
                    focus_from(order, &af_info2(order, version, recorded)),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {version}"
                );
            }
        }
    }

    #[test]
    fn the_other_versions_of_a_layout_read_like_it() {
        // 0301, which the D6, the Z 5 and the Z 6II write, and 0401 and
        // 0402, which the Z f, the Z6III, the Z50II and the Z5II write.
        for order in ORDERS {
            for (version, recorded, in_focus) in [
                ("0301", Z6, None),
                ("0401", Z9, Some(true)),
                ("0402", Z9, Some(true)),
            ] {
                assert_eq!(
                    focus_from(order, &af_info2(order, version, recorded)),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {version}"
                );
            }
        }
    }

    #[test]
    fn an_af_info2_version_not_read_gives_none() {
        // The Nikon 1's 0200 and 0201, 42 bytes long as the J1 writes it
        // (public sample 2956), with a position at the 0100 offsets; then
        // versions no file shows, each laid out as the read version nearest
        // it, which a reader choosing by the first two digits would read;
        // then bytes that are no version at all.
        for order in ORDERS {
            let nikon_1 = Fields {
                len: 42,
                ..FIELDS_0100
            };
            for (version, fields, recorded) in [
                ("0200", nikon_1, COOLPIX_A),
                ("0201", nikon_1, COOLPIX_A),
                ("0102", FIELDS_0101, MADE_UP_0101),
                ("0302", FIELDS_03XX, Z6),
                ("0403", FIELDS_04XX, Z9),
                ("0500", FIELDS_04XX, Z9),
                ("\0\0\0\0", FIELDS_04XX, Z9),
                ("400 ", FIELDS_04XX, Z9),
            ] {
                let value = af_info2_with(order, version, fields, recorded);
                assert_eq!(focus_from(order, &value), None, "{order:?}, {version:?}");
            }
            // Too short to hold a version, held in the entry itself.
            assert_eq!(focus_from(order, b"040"), None, "{order:?}");
        }
    }

    #[test]
    fn an_af_info2_shorter_than_its_layout_gives_none() {
        // Each layout ends past the last field read, so one byte short, the
        // fields still lie inside the value; exactly its length gives the
        // focus.
        for order in ORDERS {
            for (version, recorded, in_focus) in [
                ("0100", COOLPIX_A, Some(true)),
                ("0101", MADE_UP_0101, Some(true)),
                ("0300", Z6, None),
                ("0400", Z9, Some(true)),
            ] {
                let value = af_info2(order, version, recorded);
                assert_eq!(
                    focus_from(order, &value),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {version}, {} bytes",
                    value.len()
                );
                let short = &value[..value.len() - 1];
                assert_eq!(
                    focus_from(order, short),
                    None,
                    "{order:?}, {version}, {} bytes",
                    short.len()
                );
            }
        }
    }

    #[test]
    fn an_af_info2_without_a_usable_position_gives_none() {
        for order in ORDERS {
            for (what, recorded) in [
                // As the D300 writes it through the viewfinder: zeros past
                // the version (public sample 928).
                (
                    "everything zero",
                    Recorded {
                        frame: [0, 0],
                        center: [0, 0],
                        area: [0, 0],
                        in_focus: 0,
                    },
                ),
                (
                    "a zero frame width",
                    Recorded {
                        frame: [0, 5504],
                        ..Z9
                    },
                ),
                (
                    "a zero frame height",
                    Recorded {
                        frame: [8256, 0],
                        ..Z9
                    },
                ),
                (
                    "a zero area width",
                    Recorded {
                        area: [0, 323],
                        ..Z9
                    },
                ),
                (
                    "a zero area height",
                    Recorded {
                        area: [291, 0],
                        ..Z9
                    },
                ),
                (
                    "a center right of the frame",
                    Recorded {
                        center: [8257, 2461],
                        ..Z9
                    },
                ),
                (
                    "a center below the frame",
                    Recorded {
                        center: [4905, 5505],
                        ..Z9
                    },
                ),
                // An area 291 pixels wide needs 146 on each side of the
                // center, and one 323 high 162 above and below it.
                (
                    "an area past the left edge",
                    Recorded {
                        center: [145, 2461],
                        ..Z9
                    },
                ),
                (
                    "an area past the bottom edge",
                    Recorded {
                        center: [4905, 5504 - 161],
                        ..Z9
                    },
                ),
            ] {
                let value = af_info2(order, "0400", recorded);
                assert_eq!(focus_from(order, &value), None, "{order:?}, {what}");
            }
            // The control: the same areas just fit one pixel further in.
            for center in [[146, 2461], [4905, 5504 - 162]] {
                let recorded = Recorded { center, ..Z9 };
                assert_eq!(
                    focus_from(order, &af_info2(order, "0400", recorded)),
                    Some(expected(recorded, Some(true))),
                    "{order:?}, {center:?}"
                );
            }
        }
    }

    #[test]
    fn an_af_info2_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &af_info2(inner_order, "0400", Z9));
            })
            .0;
            let metadata = exif_metadata(file, &block, |_, _| {});
            assert_eq!(metadata.focus(), Some(expected(Z9, Some(true))), "{file:?}");
        }
    }

    #[test]
    fn a_byte_swapped_af_info2_gives_none() {
        // As in the other byte order from the MakerNote's: the Coolpix A's
        // frame reads as 16403 by 49164, the Z 9's as 16416 by 32789, and
        // each center then lies outside it.
        for (order, swapped) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            for (version, recorded) in [("0100", COOLPIX_A), ("0400", Z9)] {
                let value = af_info2(swapped, version, recorded);
                assert_eq!(focus_from(order, &value), None, "{order:?}, {version}");
            }
        }
    }

    #[test]
    fn the_in_focus_byte_says_whether_focus_was_confirmed() {
        // 1 is confirmed, 0 a miss, as on the Z6III samples beside a valid
        // point (public samples 7808 to 7810); any other value says nothing,
        // and the point is kept.
        for order in ORDERS {
            for (version, recorded) in [("0100", COOLPIX_A), ("0101", MADE_UP_0101), ("0400", Z9)] {
                for (byte, in_focus) in [(1, Some(true)), (0, Some(false)), (2, None), (255, None)]
                {
                    let recorded = Recorded {
                        in_focus: byte,
                        ..recorded
                    };
                    assert_eq!(
                        focus_from(order, &af_info2(order, version, recorded)),
                        Some(expected(recorded, in_focus)),
                        "{order:?}, {version}, {byte}"
                    );
                }
            }
        }
    }

    #[test]
    fn versions_0300_and_0301_do_not_say_whether_focus_was_confirmed() {
        // Every byte but the version and the position set to 1, wherever an
        // in-focus byte might be looked for.
        for order in ORDERS {
            for version in ["0300", "0301"] {
                let mut value = af_info2(order, version, Z6);
                value[4..FIELDS_03XX.frame].fill(1);
                value[FIELDS_03XX.area + 4..].fill(1);
                assert_eq!(
                    focus_from(order, &value),
                    Some(expected(Z6, None)),
                    "{order:?}, {version}"
                );
            }
        }
    }

    #[test]
    fn an_af_info2_past_the_block_is_not_read() {
        for order in ORDERS {
            let value = af_info2(order, "0400", Z9);
            let block = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &value);
            })
            .0;
            // The value is the block's last 80 bytes.
            assert_eq!(block[block.len() - 80..], value, "{order:?}");

            // Declared one byte short: the value's last byte lies in the
            // file, after the block.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).focus(), None, "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            assert_eq!(
                metadata_of(file).focus(),
                Some(expected(Z9, Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_af_info2_of_another_type_is_not_read() {
        // The Z 9's 80 bytes, declared as BYTEs, which the reader of bytes
        // reads too.
        for order in ORDERS {
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9));
            });
            let block = retyped(order, block, entry, BYTE, 80);
            let metadata = exif_metadata(order, &block, |_, _| {});
            assert_eq!(metadata.focus(), None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_without_af_info2_gives_no_focus() {
        // As the D1H, D70, D100, D200 and older Coolpix bodies write it.
        for order in ORDERS {
            assert_eq!(focus_of(order, |_, _| {}), None, "{order:?}");
        }
    }

    #[test]
    fn a_focus_mode_of_manual_gives_no_focus() {
        // `MANUAL` and the NUL the builder adds, as the D300 and the D850
        // write it (public samples 928 and 1835), and padded with spaces as
        // an autofocus mode is, beside the Z 9's AFInfo2.
        for order in ORDERS {
            for mode in ["MANUAL", "MANUAL  "] {
                let focus = focus_of(order, |inner, ifd| {
                    inner.ascii(ifd, FOCUS_MODE, mode).undefined(
                        ifd,
                        AF_INFO2,
                        &af_info2(order, "0400", Z9),
                    );
                });
                assert_eq!(focus, None, "{order:?}, {mode:?}");
            }
        }
    }

    #[test]
    fn a_focus_mode_other_than_manual_keeps_the_focus() {
        // Autofocus modes, padded as the bodies write them; `manual` in
        // lower case, which the exact rule does not take for `MANUAL`; and
        // no FocusMode at all.
        for order in ORDERS {
            for mode in [Some("AF-S  "), Some("AF-C  "), Some("manual"), None] {
                let focus = focus_of(order, |inner, ifd| {
                    if let Some(mode) = mode {
                        inner.ascii(ifd, FOCUS_MODE, mode);
                    }
                    inner.undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9));
                });
                assert_eq!(focus, Some(expected(Z9, Some(true))), "{order:?}, {mode:?}");
            }
        }
    }

    #[test]
    fn a_focus_mode_of_another_type_is_not_read() {
        // `MANUAL` and its NUL as seven UNDEFINED bytes: not FocusMode's
        // type, so no manual focus is recorded, and the focus is kept.
        for order in ORDERS {
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, FOCUS_MODE, "MANUAL").undefined(
                    ifd,
                    AF_INFO2,
                    &af_info2(order, "0400", Z9),
                );
            });
            let block = retyped(order, block, entry, UNDEFINED, 7);
            let metadata = exif_metadata(order, &block, |_, _| {});
            assert_eq!(
                metadata.focus(),
                Some(expected(Z9, Some(true))),
                "{order:?}"
            );
        }
    }

    /// The focus points of a file in byte order `order` whose MakerNote
    /// holds the AFInfo2 value `value`, as UNDEFINED, and no FocusMode.
    fn points_from(order: Order, value: &[u8]) -> Option<FocusPoints> {
        makernote_metadata(order, |inner, ifd| {
            inner.undefined(ifd, AF_INFO2, value);
        })
        .focus_points
    }

    /// A record for each layout read: the Coolpix A's 0100, a made-up
    /// 0101, the Z 6's 0300 and the Z 9's 0400.
    const RECORDS: [(&str, Recorded); 4] = [
        ("0100", COOLPIX_A),
        ("0101", MADE_UP_0101),
        ("0300", Z6),
        ("0400", Z9),
    ];

    #[test]
    fn the_area_mode_is_named_for_the_codes_checked_files_show() {
        // Byte 5 on every version read, under the detection method byte 4
        // records: 0 phase detection, 1 contrast detection, 2 hybrid. The
        // codes that frames shot in a known mode confirmed name it: the
        // DSLR codes, 9 among them, only under phase detection, 193 and
        // 197 under any, and 207 under contrast or hybrid detection, the
        // lists that carry it. Any other names none: a DSLR code under
        // contrast or hybrid detection; 207 under phase detection, whose
        // list lacks it; the codes the public samples show in modes not
        // named, 1 and 2 under detection 1 in the live view of the D7000,
        // the Coolpix A, the D5600 and the D5, and 192, 196 and 199 under
        // detection 2; and codes no file shows. The point is kept either
        // way.
        use FocusAreaMode::{Auto, Dynamic, Single, Tracking};
        let mut named = vec![(0, 0, Single), (0, 5, Dynamic), (0, 8, Auto)];
        for detection in [0, 1, 2] {
            named.extend([(detection, 193, Single), (detection, 197, Auto)]);
        }
        named.extend([(0, 9, Tracking), (1, 207, Tracking), (2, 207, Tracking)]);
        let mut unnamed = vec![(1, 0), (2, 0), (1, 5), (2, 8), (1, 4), (0, 207)];
        unnamed.extend([(1, 9), (2, 9)]);
        unnamed.extend([1, 2, 3, 4, 192, 196, 199, 255].map(|code| (0, code)));
        unnamed.extend([1, 2, 192, 199].map(|code| (1, code)));
        let codes = named
            .into_iter()
            .map(|(detection, code, mode)| (detection, code, Some(mode)))
            .chain(
                unnamed
                    .into_iter()
                    .map(|(detection, code)| (detection, code, None)),
            );
        for order in ORDERS {
            for (version, recorded) in RECORDS {
                for (detection, code, mode) in codes.clone() {
                    let mut value = af_info2(order, version, recorded);
                    value[4] = detection;
                    value[5] = code;
                    let label = format!("{order:?}, {version}, detection {detection}, code {code}");
                    let points = points_from(order, &value).expect(&label);
                    assert_eq!(points.area_mode(), mode, "{label}");
                    assert_eq!(points.points().len(), 1, "{label}");
                }
            }
        }
    }

    #[test]
    fn a_recorded_position_is_a_list_of_one_point() {
        // One point with no number, the list's primary, in the recorded
        // frame, recorded. `focus` is the list's primary by construction,
        // so the tests above that check `focus` check the point. The
        // builder writes zeros past the version, so bytes 4 and 5 are 0,
        // code 0 under phase detection: single point.
        for order in ORDERS {
            for (version, recorded) in RECORDS {
                let label = format!("{order:?}, {version}");
                let metadata = makernote_metadata(order, |inner, ifd| {
                    inner.undefined(ifd, AF_INFO2, &af_info2(order, version, recorded));
                });
                let points = metadata.focus_points().expect(&label);
                let [only] = points.points() else {
                    panic!("{label}: {points:?}");
                };
                assert_eq!(only.number(), None, "{label}");
                assert_eq!(points.primary_index(), Some(0), "{label}");
                assert_eq!(points.primary(), Some(*only), "{label}");
                let [width, height] = recorded.frame.map(u32::from);
                assert_eq!(points.frame(), Size { width, height }, "{label}");
                assert_eq!(points.origin(), FocusOrigin::Recorded, "{label}");
                assert_eq!(points.area_mode(), Some(FocusAreaMode::Single), "{label}");
            }
        }
    }

    #[test]
    fn a_file_that_gives_no_focus_gives_no_points() {
        for order in ORDERS {
            // Manual focus beside the Z 9's AFInfo2.
            let manual = makernote_metadata(order, |inner, ifd| {
                inner.ascii(ifd, FOCUS_MODE, "MANUAL").undefined(
                    ifd,
                    AF_INFO2,
                    &af_info2(order, "0400", Z9),
                );
            });
            assert_eq!(manual.focus_points(), None, "{order:?}");
            // A zero frame, as a DSLR writes through the viewfinder, beside
            // an area mode that is named, in a file with no Model, so no
            // table to place a point from.
            let viewfinder = Recorded {
                frame: [0, 0],
                center: [0, 0],
                area: [0, 0],
                in_focus: 0,
            };
            let mut value = af_info2(order, "0100", viewfinder);
            value[5] = 8;
            assert_eq!(points_from(order, &value), None, "{order:?}");
            // No AFInfo2 at all.
            let none = makernote_metadata(order, |_, _| {});
            assert_eq!(none.focus_points(), None, "{order:?}");
        }
    }

    #[test]
    fn a_zero_side_of_the_frame_or_the_area_alone_names_the_points() {
        // Any zero side says the record records no position, and the
        // points it names are placed: the frame's width or height, or the
        // area's, with the other values a valid position's.
        let valid = Recorded {
            frame: [6016, 4016],
            center: [343, 343],
            area: [540, 540],
            in_focus: 1,
        };
        for order in ORDERS {
            for (what, recorded) in [
                (
                    "frame width",
                    Recorded {
                        frame: [0, 4016],
                        ..valid
                    },
                ),
                (
                    "frame height",
                    Recorded {
                        frame: [6016, 0],
                        ..valid
                    },
                ),
                (
                    "area width",
                    Recorded {
                        area: [0, 540],
                        ..valid
                    },
                ),
                (
                    "area height",
                    Recorded {
                        area: [540, 0],
                        ..valid
                    },
                ),
            ] {
                let value = naming(af_info2(order, "0100", recorded), 3, 38, &[37]);
                let metadata = shot(order, "NIKON D600", &value, Some(D600_FX));
                assert_eq!(
                    metadata.focus_points(),
                    Some(&placed_list([6016, 4016], &[B1], 38, FocusAreaMode::Single)),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_record_of_a_position_keeps_the_recorded_path_whatever_it_names() {
        // A live-view record of a D600, frame 54's position, beside the
        // bytes of a viewfinder shot naming point 38: the position, as
        // recorded. The same with its center off the frame: no point, the
        // named one not placed in its stead.
        let live_view = Recorded {
            frame: [6016, 4016],
            center: [343, 343],
            area: [540, 540],
            in_focus: 1,
        };
        let off_frame = Recorded {
            center: [7000, 343],
            ..live_view
        };
        for order in ORDERS {
            let value = naming(af_info2(order, "0100", live_view), 3, 38, &[37]);
            let metadata = shot(order, "NIKON D600", &value, Some(D600_FX));
            let focus = Some(expected(live_view, Some(true)));
            assert_eq!(metadata.focus(), focus, "{order:?}");
            assert_eq!(
                metadata.focus_points(),
                focus
                    .map(FocusPoints::from)
                    .map(|points| points.with_area_mode(FocusAreaMode::Single))
                    .as_ref(),
                "{order:?}"
            );
            let value = naming(af_info2(order, "0100", off_frame), 3, 38, &[37]);
            let metadata = shot(order, "NIKON D600", &value, Some(D600_FX));
            assert_eq!(metadata.focus_points(), None, "{order:?}");
        }
    }

    #[test]
    fn points_named_past_the_value_give_none_and_never_a_panic() {
        // A version 0100 value cut short of its 30 bytes, by 7 or more,
        // gives no record at all; the named points read at byte 6 need 9
        // bytes there.
        for order in ORDERS {
            let value = d600_viewfinder(order);
            for len in [23, 15, 8, 4] {
                let metadata = shot(order, "NIKON D600", &value[..len], Some(D600_FX));
                assert_eq!(metadata.focus_points(), None, "{order:?}, {len}");
            }
            // Every byte after the version set: a position that does not
            // hold. With a zero frame and area besides: layout code 255,
            // which names nothing.
            let mut garbage = value.clone();
            garbage[4..].fill(0xFF);
            assert_eq!(
                shot(order, "NIKON D600", &garbage, Some(D600_FX)).focus_points(),
                None,
                "{order:?}"
            );
            garbage[16..28].fill(0);
            assert_eq!(
                shot(order, "NIKON D600", &garbage, Some(D600_FX)).focus_points(),
                None,
                "{order:?}"
            );
        }
        let value = d600_viewfinder(Order::Little);
        assert_eq!(named_points(&value[..14], 6), None);
        assert_eq!(named_points(&value, usize::MAX), None);
        let named = named_points(&value[..15], 6).expect("nine bytes from 6");
        assert_eq!((named.layout_code, named.primary), (3, 38));
        assert!(named.marks(37));
        assert!(!named.marks(36));
        assert!(!named.marks(56));
        assert!(!named.marks(usize::MAX));
    }
}
