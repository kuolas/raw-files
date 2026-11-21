// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote's AFInfo2, read at each version's offsets, and the
//! points a shot through a DSLR's viewfinder names by number.
//!
//! AFInfo2 records where the camera's autofocus aimed, which no standard
//! tag records ([`AF_INFO2`](super::AF_INFO2)), and the AF-area mode the
//! camera was set to, a camera setting ([`area_mode_of`]). It records the
//! focus point in one of two ways:
//!
//! - **A position in pixels**, on mirrorless bodies and on DSLRs in live
//!   view. It is the size of a frame, the center of the focus area in that
//!   frame, and the area's width and height. The frame is the camera's
//!   full-size output, not the stored raw, and the point is reported in it
//!   as recorded.
//! - **Points named by number**, on a DSLR shot through the viewfinder,
//!   with a zero frame and area ([`NamedPoints`]). Only a table of the
//!   body's points places them, which [`crop`](super::crop) does.
//!
//! [`af_info2_record`] reads a value at its version's offsets ([`layout`])
//! into a [`FocusRecord`]. It checks nothing more: [`crop`](super::crop)
//! checks a record before it places it. [`area_mode_of`] reads the area
//! mode of the same value, whether or not the record gives a point.
//!
//! This reader does not give the layout of the body's points
//! ([`Metadata::focus_layout`](crate::Metadata::focus_layout)). Detection
//! gives it, from the Model alone, to every file of a body whose table is
//! in raw-files ([`points`]). So a file has its layout whatever the shot,
//! and even when its MakerNote cannot be read.
//!
//! The NEF notes give each version's offsets, the bodies that write it and
//! the samples that confirm it (`docs/formats/nef/makernote-and-metadata.md`,
//! section 8.6).

use crate::camera_settings::FocusAreaMode;
use crate::focus::{Focus, FocusOrigin};
use crate::formats::makernote::points;
use crate::geometry::{Point, Size};
use crate::io::ByteOrder;

/// Where one AFInfo2 version keeps the focus point: the byte offsets, from
/// the start of the value, of the fields read, and the shortest value that
/// version writes.
///
/// Each of the frame, the center and the area is two SHORTs in the
/// MakerNote's byte order, the width or x first. The offsets are those of
/// ExifTool's published tag documentation, read as documentation and
/// confirmed on the samples whose fields hold values. The area mode is at
/// the same offsets on every version, bytes 4 and 5, so only the list of
/// codes it takes is here.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    /// How long a value of this version is, at least.
    min_len: usize,
    /// The list of area-mode codes the version takes ([`area_mode`]).
    codes: Codes,
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
    /// mask of the points used ([`NamedPoints`]), on version 0100 alone.
    named: Option<usize>,
}

/// The layout of AFInfo2 version `version`, its first four bytes. `None`
/// for any version but the exact ones read, 0100, 0101, 0300, 0301, 0400,
/// 0401 and 0402. A version that shares its first two digits with these,
/// such as 0200 or a future 0403, may have moved its fields, and is not
/// read at its neighbors' offsets (ADR012).
pub(super) fn layout(version: &[u8]) -> Option<Layout> {
    match version {
        b"0100" => Some(Layout {
            min_len: 30,
            codes: Codes::TwoLists,
            frame: 16,
            center: 20,
            area: 24,
            in_focus: Some(28),
            named: Some(6),
        }),
        b"0101" => Some(Layout {
            min_len: 84,
            codes: Codes::TwoLists,
            frame: 70,
            center: 74,
            area: 78,
            in_focus: Some(82),
            named: None,
        }),
        b"0300" | b"0301" => Some(Layout {
            min_len: 60,
            codes: Codes::TwoLists,
            frame: 42,
            center: 46,
            area: 50,
            in_focus: None,
            named: None,
        }),
        b"0400" | b"0401" | b"0402" => Some(Layout {
            min_len: 80,
            codes: Codes::Mirrorless,
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
/// checked.
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
    /// The points the record names, read on version 0100 alone.
    pub(super) named: Option<NamedPoints>,
}

impl FocusRecord {
    /// Whether the record records a position: whether its frame and its
    /// area have no zero side. A record with no position names its points
    /// instead ([`NamedPoints`]).
    pub(super) fn records_position(self) -> bool {
        let Self { frame, area, .. } = self;
        frame.width != 0 && frame.height != 0 && area.width != 0 && area.height != 0
    }

    /// The focus point this record describes, with whether focus was
    /// confirmed. `None` when [`Focus::new`] or [`Focus::with_size`] refuses
    /// it: a frame or an area with a zero side, a center outside the frame,
    /// or an area that does not fit inside the frame around its center.
    pub(super) fn focus(self) -> Option<Focus> {
        // An area that `with_size` refuses costs the whole focus, not only
        // the area. Such a record cannot be used, and gives no focus on
        // every format (ADR012).
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
/// `order`, at its version's [`layout`]. The in-focus byte, when the layout
/// has one, is `true` for 1 and `false` for 0, and says nothing otherwise.
/// The points the record names are read where the layout keeps them
/// ([`named_points`]).
///
/// `None` when the version is not one read, or the value is shorter than
/// its layout. The values are not checked here.
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
    let named = layout.named.and_then(|at| named_points(value, at));
    Some(FocusRecord {
        frame,
        center: Point { x, y },
        area,
        in_focus,
        named,
    })
}

/// The AF-area mode that the AFInfo2 value `value` records: the mode
/// [`area_mode`] names for the code at byte 5, under the detection method
/// at byte 4, in the list of codes of the value's version. It is read at
/// every version [`layout`] reads, from a value
/// at least as long as its layout, whatever the record holds besides: a
/// record of no usable position gives its area mode too.
///
/// `None` when the version is not one read, or the value is shorter than
/// its layout.
pub(super) fn area_mode_of(value: &[u8]) -> Option<FocusAreaMode> {
    let layout = layout(value.get(..4)?)?;
    if value.len() < layout.min_len {
        return None;
    }
    // Every layout is longer than six bytes, so both bytes are there.
    let &[detection, code] = value.get(4..6)?.first_chunk::<2>()?;
    area_mode(layout.codes, detection, code)
}

/// What a record with no position names of the points the camera used:
/// the layout of points the numbers belong to, the primary point's number
/// and a mask of the points used. Only a table of the body's points places
/// them ([`placed`]). Read on version 0100 alone ([`Layout::named`]), where
/// the D600's frames checked them.
///
/// [`placed`]: super::crop::placed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NamedPoints {
    /// The layout code, AFInfo2's FocusPointSchema, byte 6. It is 0, no
    /// layout, on a live-view or manual-focus shot.
    pub(super) layout_code: u8,
    /// The primary point's number, byte 7: the point the camera focused
    /// on, and 0 for none.
    pub(super) primary: u8,
    /// The mask of the points used, bytes 8 to 14. The table's bit order
    /// ties its bits to the table's points ([`PointTable::bit`]).
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
/// layout keeps them ([`Layout::named`]): the layout code, then the primary
/// point's number, then the mask of the points used. `None` when they do
/// not all lie inside the value.
fn named_points(value: &[u8], at: usize) -> Option<NamedPoints> {
    let (&[layout_code, primary], rest) = value.get(at..)?.split_first_chunk::<2>()?;
    Some(NamedPoints {
        layout_code,
        primary,
        mask: *rest.first_chunk()?,
    })
}

/// The list of area-mode codes an AFInfo2 version takes at byte 5, as
/// ExifTool's published tag documentation, read as documentation, gives
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Codes {
    /// Versions 0100, 0101, 0300 and 0301: two lists, the first for phase
    /// detection, the second for contrast detection, each with the DSLR
    /// codes and the mirrorless ones.
    TwoLists,
    /// Versions 0400 to 0402: the mirrorless list alone.
    Mirrorless,
}

/// The area mode, how the camera chose its points, that AFInfo2's byte 5
/// names with `code` in the list `codes`, under the detection method that
/// byte 4 records as `detection`: 0 phase detection, through a DSLR's
/// viewfinder; 1 contrast detection, in a DSLR's live view and in my
/// Z 9's pinpoint AF; 2 both, on the mirrorless bodies.
///
/// A pair is named when the documentation names its code under that
/// method, in that version's list, and a measured file holds it under
/// that method in a version of the same list, though not always the
/// version at hand: a single area under phase detection is named in a
/// 0300 record too, as in the 0100 records that hold it (ADR011). The
/// pairs are those of the NEF notes' table of the codes by detection method
/// (`docs/formats/nef/makernote-and-metadata.md`, section 8.6), which
/// marks each as confirmed by a frame shot in that mode or documented and
/// agreed with by the files. Each maker's finer kind folds into a variant:
///
/// - the DSLR codes, under [`TwoLists`](Codes::TwoLists) alone
///   ([`dslr_area_mode`]);
/// - the mirrorless codes, under either list ([`mirrorless_area_mode`]).
///
/// `None` for every other pair: a code the documentation does not name,
/// or one no measured file holds under that method, such as 192 under
/// phase detection or 194; 0 under contrast detection, which names no
/// area; and 3, face priority, which no neutral word fits.
pub(super) fn area_mode(codes: Codes, detection: u8, code: u8) -> Option<FocusAreaMode> {
    match codes {
        Codes::TwoLists => {
            dslr_area_mode(detection, code).or_else(|| mirrorless_area_mode(detection, code))
        }
        Codes::Mirrorless => mirrorless_area_mode(detection, code),
    }
}

/// The area mode a DSLR code `code` names under the detection method
/// `detection`, in the two lists of versions 0100 to 0301:
///
/// - under phase detection, 0, single area,
///   [`Single`](FocusAreaMode::Single); 1, 4, 5 and 6, dynamic area of
///   any size, [`Dynamic`](FocusAreaMode::Dynamic); 7 and 9, dynamic area
///   with 3D-tracking, [`Tracking`](FocusAreaMode::Tracking); 8, auto-area,
///   [`Auto`](FocusAreaMode::Auto);
/// - under contrast detection, 1, normal area,
///   [`Single`](FocusAreaMode::Single); 2, wide area,
///   [`Zone`](FocusAreaMode::Zone); 4, subject tracking,
///   [`Tracking`](FocusAreaMode::Tracking).
///
/// Frames I shot in a known mode confirm 0, 5, 8 and 9 under phase
/// detection; the others are documented, and the files agree.
fn dslr_area_mode(detection: u8, code: u8) -> Option<FocusAreaMode> {
    use FocusAreaMode::{Auto, Dynamic, Single, Tracking, Zone};
    Some(match (detection, code) {
        (0, 0) | (1, 1) => Single,
        (0, 1 | 4 | 5 | 6) => Dynamic,
        (0, 7 | 9) | (1, 4) => Tracking,
        (0, 8) => Auto,
        (1, 2) => Zone,
        _ => return None,
    })
}

/// The area mode a mirrorless code `code` names under the detection
/// method `detection`, in the lists of versions 0100 to 0402, under the
/// methods the files hold it:
///
/// - 192, pinpoint, under contrast or hybrid detection,
///   [`Pinpoint`](FocusAreaMode::Pinpoint);
/// - 193, single point, under hybrid detection,
///   [`Single`](FocusAreaMode::Single);
/// - 195 and 196, wide area S and L, and 208, wide area C1 and C2, under
///   contrast or hybrid detection, 196 under hybrid alone,
///   [`Zone`](FocusAreaMode::Zone);
/// - 197, auto, under contrast or hybrid detection, and 199, auto in one
///   list and animal auto in the other, both folding into one variant,
///   under hybrid detection, [`Auto`](FocusAreaMode::Auto);
/// - 204, 205 and 206, dynamic area S, M and L, under hybrid detection,
///   [`Dynamic`](FocusAreaMode::Dynamic);
/// - 207, 3D-tracking, under contrast or hybrid detection,
///   [`Tracking`](FocusAreaMode::Tracking).
///
/// Frames I shot with my Z 9 in each of its eleven modes confirm every
/// code but 199, which is documented, and the files agree.
fn mirrorless_area_mode(detection: u8, code: u8) -> Option<FocusAreaMode> {
    use FocusAreaMode::{Auto, Dynamic, Pinpoint, Single, Tracking, Zone};
    Some(match (detection, code) {
        (1 | 2, 192) => Pinpoint,
        (2, 193) => Single,
        (1 | 2, 195 | 208) | (2, 196) => Zone,
        (1 | 2, 197) | (2, 199) => Auto,
        (2, 204..=206) => Dynamic,
        (1 | 2, 207) => Tracking,
        _ => return None,
    })
}

/// The two SHORTs at `at` in `value`, in byte order `order`. `None` when
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
    fn the_area_mode_is_named_for_the_pairs_the_notes_name() {
        // Byte 5 under the detection method byte 4 records: 0 phase
        // detection, 1 contrast detection, 2 hybrid. The DSLR codes are
        // named in the two lists of 0100 to 0301 alone; the mirrorless
        // codes in every list, under the methods the files hold them. Any
        // other pair names none: a mirrorless code under phase detection,
        // which no file holds; 0 under contrast detection, which names no
        // area; 3, face priority; documented codes no file holds, such as
        // 194 and 200; and codes nothing names. The point is kept either
        // way.
        use FocusAreaMode::{Auto, Dynamic, Pinpoint, Single, Tracking, Zone};
        let dslr = [
            (0, 0, Single),
            (0, 1, Dynamic),
            (0, 4, Dynamic),
            (0, 5, Dynamic),
            (0, 6, Dynamic),
            (0, 7, Tracking),
            (0, 8, Auto),
            (0, 9, Tracking),
            (1, 1, Single),
            (1, 2, Zone),
            (1, 4, Tracking),
        ];
        let mut mirrorless = vec![
            (2, 193, Single),
            (2, 196, Zone),
            (2, 199, Auto),
            (2, 204, Dynamic),
            (2, 205, Dynamic),
            (2, 206, Dynamic),
        ];
        for detection in [1, 2] {
            mirrorless.extend([
                (detection, 192, Pinpoint),
                (detection, 195, Zone),
                (detection, 197, Auto),
                (detection, 207, Tracking),
                (detection, 208, Zone),
            ]);
        }
        let unnamed = [
            (1, 0),
            (2, 0),
            (1, 3),
            (2, 1),
            (2, 4),
            (1, 5),
            (2, 8),
            (0, 2),
            (0, 3),
            (0, 10),
            (0, 192),
            (0, 193),
            (0, 197),
            (0, 207),
            (1, 193),
            (1, 196),
            (1, 199),
            (1, 204),
            (0, 194),
            (2, 194),
            (2, 200),
            (2, 255),
        ];
        // Every version that records a position, each with the list its
        // byte 5 takes, written out here.
        let versions = [
            ("0100", COOLPIX_A, true),
            ("0101", MADE_UP_0101, true),
            ("0300", Z6, true),
            ("0301", Z6, true),
            ("0400", Z9, false),
            ("0401", Z9, false),
            ("0402", Z9, false),
        ];
        for order in ORDERS {
            for (version, recorded, two_lists) in versions {
                let named = mirrorless
                    .iter()
                    .chain(dslr.iter().filter(|_| two_lists))
                    .map(|&(detection, code, mode)| (detection, code, Some(mode)));
                // On 0400 to 0402, the DSLR codes too name none.
                let none = unnamed
                    .iter()
                    .copied()
                    .chain(
                        dslr.iter()
                            .filter(|_| !two_lists)
                            .map(|&(detection, code, _)| (detection, code)),
                    )
                    .map(|(detection, code)| (detection, code, None));
                for (detection, code, mode) in named.chain(none) {
                    let mut value = af_info2(order, version, recorded);
                    value[4] = detection;
                    value[5] = code;
                    let label = format!("{order:?}, {version}, detection {detection}, code {code}");
                    let metadata = makernote_metadata(order, |inner, ifd| {
                        inner.undefined(ifd, AF_INFO2, &value);
                    });
                    assert_eq!(metadata.camera_settings.focus_area_mode, mode, "{label}");
                    let points = metadata.focus_points().expect(&label);
                    assert_eq!(points.points().len(), 1, "{label}");
                }
            }
        }
    }

    #[test]
    fn the_area_mode_is_given_whether_or_not_the_file_gives_a_point() {
        // Auto-area: code 197 under hybrid detection beside the Z 9's
        // position, under `MANUAL`, which gives no point, and with the
        // position off the frame; code 8 under phase detection in a record
        // of no position, through the viewfinder of a body with no table,
        // which gives none either. Each gives the area mode.
        let viewfinder = Recorded {
            frame: [0, 0],
            center: [0, 0],
            area: [0, 0],
            in_focus: 0,
        };
        let off_frame = Recorded {
            center: [8257, 2461],
            ..Z9
        };
        for order in ORDERS {
            for (what, version, recorded, focus_mode, [detection, code]) in [
                ("manual focus", "0400", Z9, Some("MANUAL"), [2, 197]),
                ("no position", "0100", viewfinder, None, [0, 8]),
                (
                    "a position off the frame",
                    "0400",
                    off_frame,
                    None,
                    [2, 197],
                ),
            ] {
                let mut value = af_info2(order, version, recorded);
                value[4] = detection;
                value[5] = code;
                let metadata = makernote_metadata(order, |inner, ifd| {
                    if let Some(text) = focus_mode {
                        inner.ascii(ifd, FOCUS_MODE, text);
                    }
                    inner.undefined(ifd, AF_INFO2, &value);
                });
                assert_eq!(metadata.focus_points(), None, "{order:?}, {what}");
                assert_eq!(
                    metadata.camera_settings.focus_area_mode,
                    Some(FocusAreaMode::Auto),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_damaged_af_info2_costs_the_area_mode_and_nothing_else() {
        // The Z 9's record with code 193, single point, beside FocusMode
        // `AF-S`: one byte short of its layout, of a version not read, or
        // stored as BYTEs, it gives no area mode and no point, and the
        // focus mode is still read.
        for order in ORDERS {
            let mut value = af_info2(order, "0400", Z9);
            value[4] = 2;
            value[5] = 193;
            let mut other_version = value.clone();
            other_version[..4].copy_from_slice(b"0403");
            for (what, value) in [
                ("one byte short", value[..value.len() - 1].to_vec()),
                ("a version not read", other_version),
            ] {
                let metadata = makernote_metadata(order, |inner, ifd| {
                    inner
                        .ascii(ifd, FOCUS_MODE, "AF-S  ")
                        .undefined(ifd, AF_INFO2, &value);
                });
                let settings = &metadata.camera_settings;
                assert_eq!(settings.focus_area_mode, None, "{order:?}, {what}");
                assert_eq!(
                    settings.focus_mode,
                    Some(crate::camera_settings::FocusMode::Single),
                    "{order:?}, {what}"
                );
                assert_eq!(metadata.focus_points(), None, "{order:?}, {what}");
            }
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &value);
            });
            let block = retyped(order, block, entry, BYTE, 80);
            let metadata = exif_metadata(order, &block, |_, _| {});
            assert_eq!(metadata.camera_settings.focus_area_mode, None, "{order:?}");
            // The control: the sound record gives the mode.
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &value);
            });
            assert_eq!(
                metadata.camera_settings.focus_area_mode,
                Some(FocusAreaMode::Single),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_recorded_position_is_a_list_of_one_point() {
        // One point with no number, the list's primary, in the recorded
        // frame, recorded. `focus` is the list's primary by construction,
        // so the tests above that check `focus` check the point. The
        // builder writes zeros past the version, so bytes 4 and 5 are 0,
        // code 0 under phase detection: single point where the version's
        // list names it, given as a camera setting beside the list.
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
                // Zeros at bytes 4 and 5: single area under phase
                // detection in the two lists, no code of 0400's.
                let mode = (version != "0400").then_some(FocusAreaMode::Single);
                assert_eq!(metadata.camera_settings.focus_area_mode, mode, "{label}");
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
                    Some(&placed_list([6016, 4016], &[B1], 38)),
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
                focus.map(FocusPoints::from).as_ref(),
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
