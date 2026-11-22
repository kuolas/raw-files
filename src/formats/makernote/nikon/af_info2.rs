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

/// What one AFInfo2 version holds and where: the shortest value that
/// version writes, the list of area-mode codes it takes, and where it
/// keeps the focus point, on the versions that record one.
///
/// The area mode is at the same offsets on every version, bytes 4 and 5,
/// so only the list of codes it takes is here. The offsets are those of
/// ExifTool's published tag documentation, read as documentation and
/// confirmed on the samples whose fields hold values.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    /// How long a value of this version is, at least.
    min_len: usize,
    /// The list of area-mode codes the version takes ([`area_mode`]).
    codes: Codes,
    /// Where the version keeps the focus point; `None` on the Nikon 1
    /// versions, which record no position.
    position: Option<Position>,
}

/// Where one AFInfo2 version keeps the focus point: the byte offsets, from
/// the start of the value, of the fields read. Each of the frame, the
/// center and the area is two SHORTs in the MakerNote's byte order, the
/// width or x first.
#[derive(Clone, Copy, Debug)]
struct Position {
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
/// for any version but the exact ones read, 0100, 0101, 0200, 0201, 0300,
/// 0301, 0400, 0401 and 0402. A version that shares its first two digits
/// with these, such as a future 0403, may have moved its fields, and is
/// not read at its neighbors' offsets (ADR012).
///
/// The Nikon 1 versions, 0200 and 0201, record the area mode and no
/// position: their point fields hold a number, which needs a table of the
/// body's points. 0200 writes 42 bytes and 0201 52, on every file
/// measured.
pub(super) fn layout(version: &[u8]) -> Option<Layout> {
    let position = |frame, in_focus, named| {
        Some(Position {
            frame,
            center: frame + 4,
            area: frame + 8,
            in_focus,
            named,
        })
    };
    let (min_len, codes, position) = match version {
        b"0100" => (30, Codes::TwoLists, position(16, Some(28), Some(6))),
        b"0101" => (84, Codes::TwoLists, position(70, Some(82), None)),
        b"0200" => (42, Codes::Nikon1, None),
        b"0201" => (52, Codes::Nikon1, None),
        b"0300" | b"0301" => (60, Codes::TwoLists, position(42, None, None)),
        b"0400" | b"0401" | b"0402" => (80, Codes::Mirrorless, position(62, Some(74), None)),
        _ => return None,
    };
    Some(Layout {
        min_len,
        codes,
        position,
    })
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
/// `None` when the version is not one read or records no position, as
/// the Nikon 1 versions do, or the value is shorter than its layout. The
/// values are not checked here.
pub(super) fn af_info2_record(value: &[u8], order: ByteOrder) -> Option<FocusRecord> {
    let layout = layout(value.get(..4)?)?;
    if value.len() < layout.min_len {
        return None;
    }
    let layout = layout.position?;
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
/// them ([`placed`]). Read on version 0100 alone ([`Position::named`]), where
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
/// layout keeps them ([`Position::named`]): the layout code, then the primary
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
    /// codes, the Nikon 1 codes and the mirrorless ones; hybrid detection
    /// takes what they share and the mirrorless codes of the second.
    TwoLists,
    /// Versions 0400 to 0402: one list of mirrorless codes, whatever the
    /// detection method.
    Mirrorless,
    /// The Nikon 1 versions, 0200 and 0201: a list of their own, with no
    /// detection method at byte 4.
    Nikon1,
}

/// The area mode, how the camera chose its points, that AFInfo2's byte 5
/// names with `code` in the list `codes`, under the detection method that
/// byte 4 records as `detection`: 0 phase detection, through a DSLR's
/// viewfinder; 1 contrast detection, in a DSLR's live view and in my
/// Z 9's pinpoint AF; 2 both, on the mirrorless bodies.
///
/// Each list is read whole, as ExifTool's published tag documentation
/// gives it: files hold part of every one, so every code it names is
/// named, those no measured file holds among them (ADR011). The NEF notes
/// (`docs/formats/nef/makernote-and-metadata.md`, section 8.6) list
/// every code with its variant and mark it confirmed by a frame, observed
/// on files, or documented only. Each maker's finer kind folds into a
/// variant:
///
/// - versions 0100 to 0301 under phase detection take the first list
///   ([`phase_detection_area_mode`]), under contrast detection the second
///   ([`contrast_detection_area_mode`]); under hybrid detection, which the
///   documentation gives neither, a code takes the meaning both lists
///   share, or the second list's for a mirrorless code it alone gives
///   ([`hybrid_area_mode`]);
/// - versions 0400 to 0402 have one list, whatever byte 4 holds
///   ([`mirrorless_area_mode`]);
/// - the Nikon 1 versions have a list of their own, with no detection
///   method at byte 4 ([`nikon_1_area_mode`]).
///
/// `None` for any other pair: a code its list does not hold, a detection
/// method the documentation does not name, the DSLR codes 0 to 18 under
/// hybrid detection, and the codes no neutral word fits (face priority,
/// and plain contrast detection, which names no area).
pub(super) fn area_mode(codes: Codes, detection: u8, code: u8) -> Option<FocusAreaMode> {
    match codes {
        Codes::TwoLists => match detection {
            0 => phase_detection_area_mode(code),
            1 => contrast_detection_area_mode(code),
            2 => hybrid_area_mode(code),
            _ => None,
        },
        Codes::Mirrorless => mirrorless_area_mode(code),
        Codes::Nikon1 => nikon_1_area_mode(code),
    }
}

/// The area mode a Nikon 1 code `code` names, whatever byte 4 holds: 128,
/// single, [`Single`](FocusAreaMode::Single); 129, auto, the camera
/// choosing among its 41 points, [`Auto`](FocusAreaMode::Auto); 130,
/// subject tracking, [`Tracking`](FocusAreaMode::Tracking). Files hold
/// 128 on both versions, 129 on 0200 and 130 on 0201. `None` for 131,
/// face priority, which no neutral word fits, and for every code the
/// list does not hold, such as the 0 that two 0201 files write.
fn nikon_1_area_mode(code: u8) -> Option<FocusAreaMode> {
    match code {
        128 => Some(FocusAreaMode::Single),
        129 => Some(FocusAreaMode::Auto),
        130 => Some(FocusAreaMode::Tracking),
        _ => None,
    }
}

/// The area mode that `code` names in the first list of versions 0100 to
/// 0301, the one phase detection takes, with the DSLR codes, the Nikon 1
/// codes and the mirrorless ones:
///
/// - [`Single`](FocusAreaMode::Single): 0 single area, 10 single area
///   with the wide focus frame, 128 and 193 single;
/// - [`Dynamic`](FocusAreaMode::Dynamic): 1 dynamic area, 4, 5, 6, 14,
///   15 and 18 its sizes of 9 to 72 points, 11 with the wide focus frame,
///   194 dynamic;
/// - [`Auto`](FocusAreaMode::Auto): 2 dynamic area with the closest
///   subject, the camera picking the point; 8 auto-area; 129, 197 and 199
///   auto;
/// - [`Tracking`](FocusAreaMode::Tracking): 7, 9 and 12 dynamic area with
///   3D-tracking, 130 subject tracking;
/// - [`Zone`](FocusAreaMode::Zone): 3 group dynamic, 13, 16 and 17 group
///   area, 195 and 196 wide area;
/// - [`Pinpoint`](FocusAreaMode::Pinpoint): 192.
///
/// Frames I shot in a known mode confirm 0, 5, 8 and 9; files hold 1, 4,
/// 6 and 7 too; the rest are documented only. `None` for 131, face
/// priority, and for every code the list does not hold.
fn phase_detection_area_mode(code: u8) -> Option<FocusAreaMode> {
    use FocusAreaMode::{Auto, Dynamic, Pinpoint, Single, Tracking, Zone};
    Some(match code {
        0 | 10 | 128 | 193 => Single,
        1 | 4..=6 | 11 | 14 | 15 | 18 | 194 => Dynamic,
        2 | 8 | 129 | 197 | 199 => Auto,
        7 | 9 | 12 | 130 => Tracking,
        3 | 13 | 16 | 17 | 195 | 196 => Zone,
        192 => Pinpoint,
        _ => return None,
    })
}

/// The area mode that `code` names in the second list of versions 0100
/// to 0301, the one contrast detection takes:
///
/// - [`Single`](FocusAreaMode::Single): 1 normal area, 128 and 193
///   single, 200 normal-area AF;
/// - [`Dynamic`](FocusAreaMode::Dynamic): 194 dynamic, 204, 205 and 206
///   dynamic area S, M and L;
/// - [`Auto`](FocusAreaMode::Auto): 129 and 197 auto, 198 auto with
///   people, 199 auto with animals;
/// - [`Tracking`](FocusAreaMode::Tracking): 4 subject tracking, 130 and
///   203 subject tracking, 207 3D-tracking;
/// - [`Zone`](FocusAreaMode::Zone): 2 wide area, 195 and 196 wide area S
///   and L, 201 wide-area AF, 208 wide area C1 and C2;
/// - [`Pinpoint`](FocusAreaMode::Pinpoint): 192.
///
/// Files hold 0 to 4; the rest are documented only. `None` for 0, plain
/// contrast detection, which names no area; for 3, 131 and 202, face
/// priority; and for every code the list does not hold.
fn contrast_detection_area_mode(code: u8) -> Option<FocusAreaMode> {
    use FocusAreaMode::{Auto, Dynamic, Pinpoint, Single, Tracking, Zone};
    Some(match code {
        1 | 128 | 193 | 200 => Single,
        194 | 204..=206 => Dynamic,
        129 | 197..=199 => Auto,
        4 | 130 | 203 | 207 => Tracking,
        2 | 195 | 196 | 201 | 208 => Zone,
        192 => Pinpoint,
        _ => return None,
    })
}

/// The area mode that `code` names under hybrid detection in versions
/// 0100 to 0301, for which the documentation gives neither list:
///
/// - the meaning both lists share for 128 to 130 and 192 to 197, and for
///   199, which folds auto and auto with animals into
///   [`Auto`](FocusAreaMode::Auto);
/// - the second list's for 198 and 200 to 208, the mirrorless codes it
///   alone gives: version 0400's list holds 204 to 208, which my Z 9's
///   frames write under hybrid detection, and 198 and 200 to 203 are
///   named with them, though no file shows them.
///
/// Files hold 193, 195, 197 and 199. `None` for the DSLR codes 0 to 18:
/// 0 to 4, which the two lists name differently (0 is a single area in the
/// first and plain contrast detection in the second), and 5 to 18,
/// viewfinder modes of the first list alone, which no file shows under
/// hybrid detection; and for every code neither list holds.
fn hybrid_area_mode(code: u8) -> Option<FocusAreaMode> {
    match code {
        0..=18 => None,
        _ => contrast_detection_area_mode(code),
    }
}

/// The area mode that `code` names in the one list of versions 0400 to
/// 0402, whatever byte 4 holds:
///
/// - 192, pinpoint, [`Pinpoint`](FocusAreaMode::Pinpoint);
/// - 193, single point, [`Single`](FocusAreaMode::Single);
/// - 195 and 196, wide area S and L, and 208, wide area C1 and C2,
///   [`Zone`](FocusAreaMode::Zone);
/// - 197, auto, [`Auto`](FocusAreaMode::Auto);
/// - 204, 205 and 206, dynamic area S, M and L,
///   [`Dynamic`](FocusAreaMode::Dynamic);
/// - 207, 3D-tracking, [`Tracking`](FocusAreaMode::Tracking).
///
/// Frames I shot with my Z 9 in each of its eleven modes confirm every
/// code. `None` for every code the list does not hold, the DSLR codes
/// among them.
fn mirrorless_area_mode(code: u8) -> Option<FocusAreaMode> {
    use FocusAreaMode::{Auto, Dynamic, Pinpoint, Single, Tracking, Zone};
    Some(match code {
        192 => Pinpoint,
        193 => Single,
        195 | 196 | 208 => Zone,
        197 => Auto,
        204..=206 => Dynamic,
        207 => Tracking,
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
        // The Nikon 1's 0200 and 0201, as long as each writes it, 42 and 52
        // bytes (public samples 2956 and 5922), with a position at the 0100
        // offsets, which those versions do not record; then versions no
        // file shows, each laid out as the read version nearest it, which a
        // reader choosing by the first two digits would read; then bytes
        // that are no version at all.
        for order in ORDERS {
            let nikon_1 = |len| Fields { len, ..FIELDS_0100 };
            for (version, fields, recorded) in [
                ("0200", nikon_1(42), COOLPIX_A),
                ("0201", nikon_1(52), COOLPIX_A),
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

    // Every code of each list, with the variant the NEF notes' tables of
    // section 8.6 give it, written out from those tables, not from the
    // reader. A code a list does not hold, and 131 and 202, face
    // priority, are absent: they name none.
    use FocusAreaMode::{Auto, Dynamic, Pinpoint, Single, Tracking, Zone};

    /// The first list of 0100 to 0301, which phase detection takes.
    const PHASE_DETECTION: [(u8, FocusAreaMode); 29] = [
        (0, Single),
        (1, Dynamic),
        (2, Auto),
        (3, Zone),
        (4, Dynamic),
        (5, Dynamic),
        (6, Dynamic),
        (7, Tracking),
        (8, Auto),
        (9, Tracking),
        (10, Single),
        (11, Dynamic),
        (12, Tracking),
        (13, Zone),
        (14, Dynamic),
        (15, Dynamic),
        (16, Zone),
        (17, Zone),
        (18, Dynamic),
        (128, Single),
        (129, Auto),
        (130, Tracking),
        (192, Pinpoint),
        (193, Single),
        (194, Dynamic),
        (195, Zone),
        (196, Zone),
        (197, Auto),
        (199, Auto),
    ];

    /// The second list of 0100 to 0301, which contrast detection takes;
    /// its 0, plain contrast detection, names no area.
    const CONTRAST_DETECTION: [(u8, FocusAreaMode); 22] = [
        (1, Single),
        (2, Zone),
        (4, Tracking),
        (128, Single),
        (129, Auto),
        (130, Tracking),
        (192, Pinpoint),
        (193, Single),
        (194, Dynamic),
        (195, Zone),
        (196, Zone),
        (197, Auto),
        (198, Auto),
        (199, Auto),
        (200, Single),
        (201, Zone),
        (203, Tracking),
        (204, Dynamic),
        (205, Dynamic),
        (206, Dynamic),
        (207, Tracking),
        (208, Zone),
    ];

    /// The one list of 0400 to 0402.
    const MIRRORLESS: [(u8, FocusAreaMode); 10] = [
        (192, Pinpoint),
        (193, Single),
        (195, Zone),
        (196, Zone),
        (197, Auto),
        (204, Dynamic),
        (205, Dynamic),
        (206, Dynamic),
        (207, Tracking),
        (208, Zone),
    ];

    /// The Nikon 1 list of 0200 and 0201.
    const NIKON_1: [(u8, FocusAreaMode); 3] = [(128, Single), (129, Auto), (130, Tracking)];

    fn in_list(list: &[(u8, FocusAreaMode)], code: u8) -> Option<FocusAreaMode> {
        list.iter()
            .find(|&&(c, _)| c == code)
            .map(|&(_, mode)| mode)
    }

    /// The variant the notes give `code` under `detection` in `codes`.
    /// Under hybrid detection on 0100 to 0301, a code above 18 takes the
    /// meaning of the list that gives it, both lists agreeing where both
    /// do, and the DSLR codes 0 to 18 take none.
    fn listed_mode(codes: Codes, detection: u8, code: u8) -> Option<FocusAreaMode> {
        match (codes, detection) {
            (Codes::TwoLists, 0) => in_list(&PHASE_DETECTION, code),
            (Codes::TwoLists, 1) => in_list(&CONTRAST_DETECTION, code),
            (Codes::TwoLists, 2) if code > 18 => {
                in_list(&PHASE_DETECTION, code).or(in_list(&CONTRAST_DETECTION, code))
            }
            (Codes::TwoLists, _) => None,
            (Codes::Mirrorless, _) => in_list(&MIRRORLESS, code),
            (Codes::Nikon1, _) => in_list(&NIKON_1, code),
        }
    }

    #[test]
    fn the_two_lists_agree_on_every_code_above_4_that_both_give() {
        // So hybrid detection can name each of them: both lists give
        // 128 to 130 and 192 to 199 one meaning, 199 folding auto and
        // auto with animals into one variant.
        for code in 5..=u8::MAX {
            if let (Some(phase), Some(contrast)) = (
                in_list(&PHASE_DETECTION, code),
                in_list(&CONTRAST_DETECTION, code),
            ) {
                assert_eq!(phase, contrast, "code {code}");
            }
        }
        // And 0 to 4 differ: 0 and 3 are named in one list alone, 1, 2
        // and 4 with two variants.
        for code in 0..=4 {
            assert_ne!(
                in_list(&PHASE_DETECTION, code),
                in_list(&CONTRAST_DETECTION, code),
                "code {code}"
            );
        }
    }

    #[test]
    fn every_code_takes_the_meaning_of_its_list() {
        // Every pair of a detection method and a code, in each kind of
        // list; detection 3, which the documentation does not name, names
        // none in the two lists, and the lists of one kind ignore byte 4.
        for codes in [Codes::TwoLists, Codes::Mirrorless, Codes::Nikon1] {
            for detection in 0..=3 {
                for code in 0..=u8::MAX {
                    assert_eq!(
                        area_mode(codes, detection, code),
                        listed_mode(codes, detection, code),
                        "{codes:?}, detection {detection}, code {code}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_area_mode_of_a_record_is_named_by_its_versions_list() {
        // Through a whole record of each version that records a position:
        // codes of each list and of none, under each detection method. The
        // point is kept either way.
        let codes_seen = [
            0, 1, 2, 3, 4, 5, 8, 10, 12, 13, 18, 128, 131, 192, 193, 194, 196, 198, 199, 200, 202,
            204, 207, 208, 255,
        ];
        let versions = [
            ("0100", COOLPIX_A, Codes::TwoLists),
            ("0101", MADE_UP_0101, Codes::TwoLists),
            ("0300", Z6, Codes::TwoLists),
            ("0301", Z6, Codes::TwoLists),
            ("0400", Z9, Codes::Mirrorless),
            ("0401", Z9, Codes::Mirrorless),
            ("0402", Z9, Codes::Mirrorless),
        ];
        for order in ORDERS {
            for (version, recorded, codes) in versions {
                for detection in 0..=2 {
                    for code in codes_seen {
                        let mut value = af_info2(order, version, recorded);
                        value[4] = detection;
                        value[5] = code;
                        let label =
                            format!("{order:?}, {version}, detection {detection}, code {code}");
                        let metadata = makernote_metadata(order, |inner, ifd| {
                            inner.undefined(ifd, AF_INFO2, &value);
                        });
                        assert_eq!(
                            metadata.camera_settings.focus_area_mode,
                            listed_mode(codes, detection, code),
                            "{label}"
                        );
                        let points = metadata.focus_points().expect(&label);
                        assert_eq!(points.points().len(), 1, "{label}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_nikon_1_versions_give_the_area_mode_and_no_point() {
        // 128 single, 129 auto and 130 subject tracking, whatever byte 4
        // holds, on 0200 at 42 bytes and on 0201 at 52; codes of the other
        // lists, and the 0 two 0201 files write, give none. No point
        // either way: those versions record a number, not a position.
        for order in ORDERS {
            for (version, len) in [("0200", 42), ("0201", 52)] {
                for (byte_4, code, mode) in [
                    (0, 128, Some(FocusAreaMode::Single)),
                    (0, 129, Some(FocusAreaMode::Auto)),
                    (0, 130, Some(FocusAreaMode::Tracking)),
                    (2, 128, Some(FocusAreaMode::Single)),
                    (0, 0, None),
                    (0, 8, None),
                    (2, 193, None),
                    (0, 131, None),
                ] {
                    let mut value = version.as_bytes().to_vec();
                    value.resize(len, 0);
                    value[4] = byte_4;
                    value[5] = code;
                    let label = format!("{order:?}, {version}, byte 4 {byte_4}, code {code}");
                    let metadata = makernote_metadata(order, |inner, ifd| {
                        inner.undefined(ifd, AF_INFO2, &value);
                    });
                    assert_eq!(metadata.camera_settings.focus_area_mode, mode, "{label}");
                    assert_eq!(metadata.focus_points(), None, "{label}");
                    // One byte short of its version's length: no mode.
                    let short = &value[..len - 1];
                    let metadata = makernote_metadata(order, |inner, ifd| {
                        inner.undefined(ifd, AF_INFO2, short);
                    });
                    assert_eq!(metadata.camera_settings.focus_area_mode, None, "{label}");
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
