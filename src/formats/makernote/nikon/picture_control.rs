// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote's Picture Control record, read at each version's
//! layout.
//!
//! Nikon's picture style, the Picture Control, is a preset of sharpening,
//! contrast, brightness, saturation and hue, which the photographer picks
//! and may adjust. The MakerNote records it in PictureControlData
//! ([`PICTURE_CONTROL`](super::PICTURE_CONTROL)), and on the Coolpix
//! bodies in the same record under another tag
//! ([`COOLPIX_PICTURE_CONTROL`](super::COOLPIX_PICTURE_CONTROL)): UNDEFINED
//! bytes that start with a version, four ASCII digits.
//!
//! The record's layout changes with its version. ExifTool's published tag
//! documentation gives the layouts and does not say which version takes
//! which; the measured files do, since each version has one length and
//! each length fits one layout. [`picture_control`] reads a record only
//! at a version and length the files hold ([`layout`]), so a version no
//! file shows gives no Picture Control.
//!
//! Every field is one byte, and is read the same way in every layout
//! ([`value`]): a count of steps from 0x80, "auto" at 0x00 in the fields
//! that offer it, and "not available" at 0xFF. The name and the base are
//! texts of 20 bytes ([`name`]).
//!
//! The NEF notes give each version, its bodies and the offsets of its
//! fields (`docs/formats/nef/makernote-and-metadata.md`, section 8.11).

use crate::camera_settings::{
    PictureControl, PictureControlAdjustment, PictureControlFilterEffect, PictureControlToning,
    PictureControlValue,
};
use crate::entries::clean_text;
use crate::rational::SignedRational;

/// Where one of the record's layouts keeps each field, as offsets from
/// the record's first byte. A field the layout lacks is `None`.
struct Layout {
    /// The record's length at the versions that take this layout.
    len: usize,
    /// The name, 20 bytes; the base follows it.
    name: usize,
    /// The adjustment kind, a code.
    adjustment: usize,
    /// Quick adjust, a value.
    quick_adjust: Option<usize>,
    /// Quick sharp, a value.
    quick_sharp: Option<usize>,
    /// Sharpening, a value that may be auto.
    sharpening: usize,
    /// Mid-range sharpening, a value.
    mid_range_sharpening: Option<usize>,
    /// Clarity, a value that may be auto.
    clarity: Option<usize>,
    /// Contrast, a value that may be auto.
    contrast: usize,
    /// Brightness, a value.
    brightness: usize,
    /// Saturation, a value that may be auto.
    saturation: usize,
    /// Hue, a value.
    hue: usize,
    /// The filter effect, a code.
    filter_effect: usize,
    /// The toning, a code.
    toning: usize,
    /// The toning's saturation, a value.
    toning_saturation: usize,
    /// Whether the byte after each value is its divisor. Without one, a
    /// value counts whole steps.
    divided: bool,
    /// Which values take "auto" at 0x00, beside contrast and saturation,
    /// which take it in every layout.
    auto: Auto,
}

/// The values beside contrast and saturation that a layout reads as
/// "auto" at 0x00: those the manuals of the bodies that write it offer
/// "A" for. The D600's and the D7200's offer it for sharpening, and files
/// of the second layout hold it in clarity; the Z 9's offers it for
/// Quick sharp, and not for sharpening or clarity (NEF notes, section
/// 8.11).
struct Auto {
    quick_sharp: bool,
    sharpening: bool,
    clarity: bool,
}

/// The first layout, of 58 bytes, at version 0100: one byte per value and
/// no divisor, so every value counts whole steps.
const FIRST: Layout = Layout {
    len: 58,
    name: 4,
    adjustment: 48,
    quick_adjust: Some(49),
    quick_sharp: None,
    sharpening: 50,
    mid_range_sharpening: None,
    clarity: None,
    contrast: 51,
    brightness: 52,
    saturation: 53,
    hue: 54,
    filter_effect: 55,
    toning: 56,
    toning_saturation: 57,
    divided: false,
    auto: Auto {
        quick_sharp: false,
        sharpening: true,
        clarity: false,
    },
};

/// The second layout, of 68 bytes, at version 0200: each value is
/// followed by its divisor, 4 for a value counted in quarter steps and 1
/// for quick adjust, and clarity joins the values.
const SECOND: Layout = Layout {
    len: 68,
    name: 4,
    adjustment: 48,
    quick_adjust: Some(49),
    quick_sharp: None,
    sharpening: 51,
    mid_range_sharpening: None,
    clarity: Some(53),
    contrast: 55,
    brightness: 57,
    saturation: 59,
    hue: 61,
    filter_effect: 63,
    toning: 64,
    toning_saturation: 65,
    divided: true,
    auto: Auto {
        quick_sharp: false,
        sharpening: true,
        clarity: true,
    },
};

/// The third layout, of 108 bytes, at versions 0300, 0301, 0302 and
/// 0310: the version again at byte 4, so the name and the base four bytes
/// later; Quick sharp in quick adjust's place; mid-range sharpening among
/// the values, each followed by its divisor.
///
/// The adjustment kind and Quick sharp are at bytes 52 and 53, two bytes
/// before the places the documentation gives them. The files hold `01 FF`
/// at bytes 54 and 55 on every record, whatever the control, while bytes
/// 52 and 53 hold the documented codes and move the three sharpening
/// values with them; Nikon's NX Studio, run on a frame of mine as a reader
/// to compare with, shows Quick sharp at byte 53 (NEF notes, section
/// 8.11).
const THIRD: Layout = Layout {
    len: 108,
    name: 8,
    adjustment: 52,
    quick_adjust: None,
    quick_sharp: Some(53),
    sharpening: 57,
    mid_range_sharpening: Some(59),
    clarity: Some(61),
    contrast: 63,
    brightness: 65,
    saturation: 67,
    hue: 69,
    filter_effect: 71,
    toning: 72,
    toning_saturation: 73,
    divided: true,
    auto: Auto {
        quick_sharp: true,
        sharpening: false,
        clarity: false,
    },
};

/// The layout of a record whose first four bytes are `version`, among the
/// versions measured files hold: 0100, the first layout ([`FIRST`]); 0200,
/// the second ([`SECOND`]); 0300, 0301, 0302 and 0310, the third
/// ([`THIRD`]). `None` for every other version, such as 0320, whose
/// layout no file shows.
fn layout(version: &[u8]) -> Option<&'static Layout> {
    match version {
        b"0100" => Some(&FIRST),
        b"0200" => Some(&SECOND),
        b"0300" | b"0301" | b"0302" | b"0310" => Some(&THIRD),
        _ => None,
    }
}

/// The length of the name and of the base, each a text ended by a NUL.
const NAME_LEN: usize = 20;

/// The byte of a value or a code the camera marks not available, as quick
/// adjust under `NEUTRAL` and the filter effect outside a monochrome
/// control.
const NOT_AVAILABLE: u8 = 0xFF;

/// The byte of a value the camera chooses, "A (auto)" in Nikon's menus.
const AUTO: u8 = 0x00;

/// The byte of a value of zero steps, and of the first code of the filter
/// effect and the toning.
const ZERO: u8 = 0x80;

/// The Picture Control that `record`, PictureControlData's bytes, holds,
/// at a version whose layout the measured files show ([`layout`]) and at
/// that layout's length. Each field is read on its own, so that a byte a
/// field cannot hold costs that field alone:
///
/// - the name and the base by [`name`];
/// - the adjustment kind by [`adjustment`];
/// - each value by [`value`], over the divisor that follows it where the
///   layout records one, and over 1 where it does not. Contrast and
///   saturation take auto in every layout, and the values of the
///   layout's [`Auto`] in it; the others do not;
/// - the filter effect by [`filter_effect`] and the toning by
///   [`toning`].
///
/// `None` for a record of another version, or of another length than its
/// version's: no measured file shows which layout it takes. The four
/// bytes before the adjustment kind, a number for each preset that the
/// documentation does not name, are not read; neither are the second
/// layout's last byte and the third layout's bytes 55 and 56 and 75 on,
/// which it does not name either (NEF notes, section 8.11).
pub(super) fn picture_control(record: &[u8]) -> Option<PictureControl> {
    let layout = layout(record.get(..4)?)?;
    if record.len() != layout.len {
        return None;
    }
    let byte = |at: usize| record.get(at).copied();
    let read = |at: usize, takes_auto: bool| {
        let divisor = if layout.divided { byte(at + 1)? } else { 1 };
        value(byte(at)?, divisor, takes_auto)
    };
    let steps = |at: usize| read(at, false);
    let steps_or_auto = |at: usize| read(at, true);
    let auto = &layout.auto;
    Some(PictureControl {
        name: name(record, layout.name),
        base: name(record, layout.name + NAME_LEN),
        adjustment: byte(layout.adjustment).and_then(adjustment),
        quick_adjust: layout.quick_adjust.and_then(steps),
        quick_sharp: layout.quick_sharp.and_then(|at| read(at, auto.quick_sharp)),
        sharpening: read(layout.sharpening, auto.sharpening),
        mid_range_sharpening: layout.mid_range_sharpening.and_then(steps),
        clarity: layout.clarity.and_then(|at| read(at, auto.clarity)),
        contrast: steps_or_auto(layout.contrast),
        brightness: steps(layout.brightness),
        saturation: steps_or_auto(layout.saturation),
        hue: steps(layout.hue),
        filter_effect: byte(layout.filter_effect).and_then(filter_effect),
        toning: byte(layout.toning).and_then(toning),
        toning_saturation: steps(layout.toning_saturation),
    })
}

/// The text of the 20 bytes of `record` from `at`, the name or the base:
/// up to the first NUL, or all 20 bytes when there is none, cleaned as
/// every text ([`clean_text`]).
///
/// `None` when the text is blank, and when a byte before the NUL is not
/// ASCII: the documentation and every measured file hold ASCII there, so
/// such a byte is damage, and costs the text alone.
fn name(record: &[u8], at: usize) -> Option<String> {
    let field = record.get(at..at.checked_add(NAME_LEN)?)?;
    let text = field
        .iter()
        .position(|&b| b == 0)
        .map_or(field, |nul| field.get(..nul).unwrap_or_default());
    if !text.is_ascii() {
        return None;
    }
    clean_text(text)
}

/// The value that `byte` records over `divisor`:
///
/// - 0xFF, not available, gives `None`;
/// - 0x00 gives [`Auto`](PictureControlValue::Auto) where the field
///   `takes_auto`, and `None` elsewhere, where it would be 128 steps
///   down;
/// - any other byte gives [`Steps`](PictureControlValue::Steps): the byte
///   less 0x80 over `divisor`, as recorded, so 0x8C over 4 is 12/4, three
///   steps, and 0x7F over 1 one step down. A divisor of zero gives `None`.
fn value(byte: u8, divisor: u8, takes_auto: bool) -> Option<PictureControlValue> {
    match byte {
        NOT_AVAILABLE => None,
        AUTO if takes_auto => Some(PictureControlValue::Auto),
        AUTO => None,
        _ => SignedRational::new(i32::from(byte) - i32::from(ZERO), i32::from(divisor))
            .ok()
            .map(PictureControlValue::Steps),
    }
}

/// The adjustment kind that `code` names, in the list the documentation
/// gives, read whole since files hold part of it: 0
/// [`Default`](PictureControlAdjustment::Default), 1
/// [`QuickAdjust`](PictureControlAdjustment::QuickAdjust) and 2
/// [`FullControl`](PictureControlAdjustment::FullControl). `None` for
/// every other code.
fn adjustment(code: u8) -> Option<PictureControlAdjustment> {
    Some(match code {
        0 => PictureControlAdjustment::Default,
        1 => PictureControlAdjustment::QuickAdjust,
        2 => PictureControlAdjustment::FullControl,
        _ => return None,
    })
}

/// The filter effect that `code` names, in the list the documentation
/// gives, read whole since a file holds part of it: 0x80
/// [`Off`](PictureControlFilterEffect::Off), 0x81
/// [`Yellow`](PictureControlFilterEffect::Yellow), 0x82
/// [`Orange`](PictureControlFilterEffect::Orange), 0x83
/// [`Red`](PictureControlFilterEffect::Red) and 0x84
/// [`Green`](PictureControlFilterEffect::Green). `None` for every other
/// code, 0xFF, not available, included.
fn filter_effect(code: u8) -> Option<PictureControlFilterEffect> {
    use PictureControlFilterEffect::{Green, Off, Orange, Red, Yellow};
    Some(match code {
        0x80 => Off,
        0x81 => Yellow,
        0x82 => Orange,
        0x83 => Red,
        0x84 => Green,
        _ => return None,
    })
}

/// The toning that `code` names, in the list the documentation gives,
/// read whole since a file holds part of it: 0x80
/// [`BlackAndWhite`](PictureControlToning::BlackAndWhite), 0x81
/// [`Sepia`](PictureControlToning::Sepia), 0x82
/// [`Cyanotype`](PictureControlToning::Cyanotype), and 0x83 to 0x89 the
/// tints red, yellow, green, blue green, blue, purple blue and red
/// purple. `None` for every other code, 0xFF, not available, included.
fn toning(code: u8) -> Option<PictureControlToning> {
    use PictureControlToning::{
        BlackAndWhite, Blue, BlueGreen, Cyanotype, Green, PurpleBlue, Red, RedPurple, Sepia, Yellow,
    };
    Some(match code {
        0x80 => BlackAndWhite,
        0x81 => Sepia,
        0x82 => Cyanotype,
        0x83 => Red,
        0x84 => Yellow,
        0x85 => Green,
        0x86 => BlueGreen,
        0x87 => Blue,
        0x88 => PurpleBlue,
        0x89 => RedPurple,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::signed_fraction;

    /// `text` as a 20-byte field, padded with NULs.
    fn field(text: &[u8]) -> Vec<u8> {
        let mut field = text.to_vec();
        field.resize(NAME_LEN, 0);
        field
    }

    /// The control's number of `STANDARD`, as the four bytes before the
    /// adjustment kind hold it in a little-endian MakerNote.
    const NUMBER: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

    /// A record of version 0100, the first layout, named `name` on base
    /// `base`, whose ten bytes from the adjustment kind on are `tail`:
    /// the adjustment kind, quick adjust, sharpening, contrast,
    /// brightness, saturation, hue, the filter effect, the toning and the
    /// toning's saturation.
    fn first(name: &[u8], base: &[u8], tail: [u8; 10]) -> Vec<u8> {
        let mut record = b"0100".to_vec();
        record.extend(field(name));
        record.extend(field(base));
        record.extend(NUMBER);
        record.extend(tail);
        assert_eq!(record.len(), 58);
        record
    }

    /// A `STANDARD` record of the first layout whose ten bytes from the
    /// adjustment kind on are `tail`.
    fn standard(tail: [u8; 10]) -> Vec<u8> {
        first(b"STANDARD", b"STANDARD", tail)
    }

    /// A record of version 0200, the second layout, named `name` on base
    /// `base`. `values` are the adjustment kind and then each value with
    /// the byte after it: quick adjust, sharpening, clarity, contrast,
    /// brightness, saturation and hue; then the filter effect, the
    /// toning, the toning's saturation with its divisor, and the last
    /// byte.
    fn second(name: &[u8], base: &[u8], values: [u8; 20]) -> Vec<u8> {
        let mut record = b"0200".to_vec();
        record.extend(field(name));
        record.extend(field(base));
        record.extend(NUMBER);
        record.extend(values);
        assert_eq!(record.len(), 68);
        record
    }

    /// A record of `version`, of the third layout, named `name` on base
    /// `base`. `values` are its 56 bytes from the adjustment kind on:
    /// the adjustment kind, Quick sharp and its divisor, two bytes the
    /// documentation does not name, then each value with the byte after
    /// it, sharpening, mid-range sharpening, clarity, contrast,
    /// brightness, saturation and hue; then the filter effect, the toning,
    /// the toning's saturation with its divisor, and 34 bytes no field
    /// takes.
    fn third(version: &[u8; 4], name: &[u8], base: &[u8], values: [u8; 56]) -> Vec<u8> {
        let mut record = version.to_vec();
        record.extend(version);
        record.extend(field(name));
        record.extend(field(base));
        record.extend(NUMBER);
        record.extend(values);
        assert_eq!(record.len(), 108);
        record
    }

    /// The 56 bytes of the third layout from the adjustment kind on that
    /// begin with `head` and are 0xFF after it, as the bytes past the
    /// documented fields are on the files.
    fn third_values(head: &[u8]) -> [u8; 56] {
        let mut values = [0xFF; 56];
        values[..head.len()].copy_from_slice(head);
        values
    }

    /// `numerator` over `denominator` steps.
    fn steps(numerator: i32, denominator: i32) -> Option<PictureControlValue> {
        Some(PictureControlValue::Steps(signed_fraction(
            numerator,
            denominator,
        )))
    }

    #[test]
    fn each_field_of_the_first_layout_is_read_at_its_place() {
        // Every byte different, so a field read from its neighbor's place
        // is caught.
        let record = first(
            b"VIVID-KR",
            b"VIVID",
            [2, 0x81, 0x83, 0x7F, 0x82, 0x84, 0x7E, 0x81, 0x82, 0x85],
        );
        let control = picture_control(&record).unwrap();
        assert_eq!(control.name(), Some("VIVID-KR"));
        assert_eq!(control.base(), Some("VIVID"));
        assert_eq!(
            control.adjustment(),
            Some(PictureControlAdjustment::FullControl)
        );
        assert_eq!(control.quick_adjust(), steps(1, 1));
        assert_eq!(control.sharpening(), steps(3, 1));
        assert_eq!(control.contrast(), steps(-1, 1));
        assert_eq!(control.brightness(), steps(2, 1));
        assert_eq!(control.saturation(), steps(4, 1));
        assert_eq!(control.hue(), steps(-2, 1));
        assert_eq!(
            control.filter_effect(),
            Some(PictureControlFilterEffect::Yellow)
        );
        assert_eq!(control.toning(), Some(PictureControlToning::Cyanotype));
        assert_eq!(control.toning_saturation(), steps(5, 1));
        // The first layout records none of these.
        assert_eq!(control.quick_sharp(), None);
        assert_eq!(control.mid_range_sharpening(), None);
        assert_eq!(control.clarity(), None);
    }

    #[test]
    fn each_field_of_the_second_layout_is_read_at_its_place_over_its_divisor() {
        // Every byte different, so a field read from its neighbor's place,
        // or over its neighbor's divisor, is caught; the last byte is not
        // read.
        #[rustfmt::skip]
        let values = [
            1,          // adjustment kind
            0x81, 1,    // quick adjust
            0x8C, 4,    // sharpening
            0x84, 4,    // clarity
            0x7C, 4,    // contrast
            0x82, 2,    // brightness
            0x86, 4,    // saturation
            0x7E, 4,    // hue
            0x84,       // filter effect
            0x89,       // toning
            0x85, 4,    // toning saturation
            0x7B,       // not documented
        ];
        let control = picture_control(&second(b"STANDARD-02", b"STANDARD", values)).unwrap();
        assert_eq!(control.name(), Some("STANDARD-02"));
        assert_eq!(control.base(), Some("STANDARD"));
        assert_eq!(
            control.adjustment(),
            Some(PictureControlAdjustment::QuickAdjust)
        );
        assert_eq!(control.quick_adjust(), steps(1, 1));
        assert_eq!(control.sharpening(), steps(12, 4));
        assert_eq!(control.clarity(), steps(4, 4));
        assert_eq!(control.contrast(), steps(-4, 4));
        assert_eq!(control.brightness(), steps(2, 2));
        assert_eq!(control.saturation(), steps(6, 4));
        assert_eq!(control.hue(), steps(-2, 4));
        assert_eq!(
            control.filter_effect(),
            Some(PictureControlFilterEffect::Green)
        );
        assert_eq!(control.toning(), Some(PictureControlToning::RedPurple));
        assert_eq!(control.toning_saturation(), steps(5, 4));
        assert_eq!(control.quick_sharp(), None);
        assert_eq!(control.mid_range_sharpening(), None);
    }

    #[test]
    fn the_second_layouts_values_are_auto_not_available_or_none_by_their_bytes() {
        // 0x00 is auto in sharpening, clarity, contrast and saturation,
        // whatever the divisor; 0xFF is not available; a divisor of zero
        // gives no value.
        #[rustfmt::skip]
        let values = [
            0,
            0x00, 1,    // quick adjust: 0 takes no auto
            0x00, 4,    // sharpening
            0x00, 0,    // clarity
            0x00, 4,    // contrast
            0x80, 0,    // brightness: a divisor of zero
            0x00, 4,    // saturation
            0xFF, 4,    // hue: not available
            0xFF,
            0xFF,
            0xFF, 4,
            0x00,
        ];
        let control = picture_control(&second(b"AUTO", b"AUTO", values)).unwrap();
        let auto = Some(PictureControlValue::Auto);
        assert_eq!(control.quick_adjust(), None);
        assert_eq!(control.sharpening(), auto);
        assert_eq!(control.clarity(), auto);
        assert_eq!(control.contrast(), auto);
        assert_eq!(control.brightness(), None);
        assert_eq!(control.saturation(), auto);
        assert_eq!(control.hue(), None);
        assert_eq!(control.filter_effect(), None);
        assert_eq!(control.toning(), None);
        assert_eq!(control.toning_saturation(), None);
        // A zero count over 4 is kept over 4.
        let mut zero = values;
        zero[3] = 0x80;
        let control = picture_control(&second(b"AUTO", b"AUTO", zero)).unwrap();
        assert_eq!(control.sharpening(), steps(0, 4));
    }

    #[test]
    fn a_version_at_another_versions_length_gives_none() {
        let first = standard([0, 0x80, 0x83, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF]);
        let second = second(b"STANDARD", b"STANDARD", [0x80; 20]);
        assert!(picture_control(&first).is_some());
        assert!(picture_control(&second).is_some());
        for (record, version) in [(&first, b"0200"), (&second, b"0100")] {
            let mut other = record.clone();
            other[..4].copy_from_slice(version);
            assert_eq!(picture_control(&other), None, "{version:?}");
        }
        assert_eq!(picture_control(&second[..67]), None);
    }

    #[test]
    fn each_field_of_the_third_layout_is_read_at_its_place_over_its_divisor() {
        #[rustfmt::skip]
        let head = [
            2,          // adjustment kind
            0x7F, 1,    // Quick sharp
            0x81, 0x01, // not documented: neither quick adjust nor a value
            0x8C, 4,    // sharpening
            0x87, 2,    // mid-range sharpening
            0x83, 4,    // clarity
            0x7D, 4,    // contrast
            0x85, 4,    // brightness
            0x7A, 4,    // saturation
            0x86, 4,    // hue
            0x83,       // filter effect
            0x87,       // toning
            0x89, 4,    // toning saturation
            0x82, 4,    // not documented
        ];
        for version in [b"0300", b"0301", b"0302", b"0310"] {
            let record = third(version, b"VIVID-KR", b"VIVID", third_values(&head));
            let control = picture_control(&record).unwrap();
            assert_eq!(control.name(), Some("VIVID-KR"), "{version:?}");
            assert_eq!(control.base(), Some("VIVID"), "{version:?}");
            assert_eq!(
                control.adjustment(),
                Some(PictureControlAdjustment::FullControl),
                "{version:?}"
            );
            assert_eq!(control.quick_sharp(), steps(-1, 1), "{version:?}");
            assert_eq!(control.sharpening(), steps(12, 4), "{version:?}");
            assert_eq!(control.mid_range_sharpening(), steps(7, 2), "{version:?}");
            assert_eq!(control.clarity(), steps(3, 4), "{version:?}");
            assert_eq!(control.contrast(), steps(-3, 4), "{version:?}");
            assert_eq!(control.brightness(), steps(5, 4), "{version:?}");
            assert_eq!(control.saturation(), steps(-6, 4), "{version:?}");
            assert_eq!(control.hue(), steps(6, 4), "{version:?}");
            assert_eq!(
                control.filter_effect(),
                Some(PictureControlFilterEffect::Red),
                "{version:?}"
            );
            assert_eq!(
                control.toning(),
                Some(PictureControlToning::Blue),
                "{version:?}"
            );
            assert_eq!(control.toning_saturation(), steps(9, 4), "{version:?}");
            // The third layout has Quick sharp in quick adjust's place.
            assert_eq!(control.quick_adjust(), None, "{version:?}");
        }
    }

    #[test]
    fn the_z_9_fixtures_bytes_read_as_nx_studio_shows_them() {
        // Bytes 52 to 54 of `nikon-z9-fx.jpg`'s record are `02 81 01` and
        // bytes 57, 59 and 61 `94 8E 88`, each over 4: NX Studio shows
        // VIVID with Quick sharp +1, sharpening +5, mid-range sharpening
        // +3.5 and clarity +2, and 0 for the four values after them.
        #[rustfmt::skip]
        let head = [
            0x02, 0x81, 0x01, 0xFF, 0x0A,
            0x94, 4, 0x8E, 4, 0x88, 4,
            0x80, 4, 0x80, 4, 0x80, 4, 0x80, 4,
            0xFF, 0xFF, 0xFF, 4,
        ];
        let record = third(b"0310", b"VIVID", b"VIVID", third_values(&head));
        let control = picture_control(&record).unwrap();
        assert_eq!(control.name(), Some("VIVID"));
        assert_eq!(
            control.adjustment(),
            Some(PictureControlAdjustment::FullControl)
        );
        assert_eq!(control.quick_sharp(), steps(1, 1));
        assert_eq!(control.sharpening(), steps(20, 4));
        assert_eq!(control.mid_range_sharpening(), steps(14, 4));
        assert_eq!(control.clarity(), steps(8, 4));
        for value in [
            control.contrast(),
            control.brightness(),
            control.saturation(),
            control.hue(),
        ] {
            assert_eq!(value, steps(0, 4));
        }
        assert_eq!(control.filter_effect(), None);
        assert_eq!(control.toning(), None);
        assert_eq!(control.toning_saturation(), None);
    }

    #[test]
    fn the_third_layout_reads_auto_where_the_menu_offers_it() {
        // No 108-byte record holds 0x00. The Z 9's guide offers "A" for
        // Quick sharp, contrast and saturation, and not for sharpening or
        // clarity, where 0x00 would be 128 steps down.
        #[rustfmt::skip]
        let head = [
            0, 0x00, 1, 0xFF, 0x0A,
            0x00, 4, 0x00, 4, 0x00, 4, 0x00, 4, 0x00, 4, 0x00, 4, 0x00, 4,
        ];
        let record = third(b"0310", b"AUTO", b"AUTO", third_values(&head));
        let control = picture_control(&record).unwrap();
        let auto = Some(PictureControlValue::Auto);
        assert_eq!(control.quick_sharp(), auto);
        assert_eq!(control.sharpening(), None);
        assert_eq!(control.mid_range_sharpening(), None);
        assert_eq!(control.clarity(), None);
        assert_eq!(control.contrast(), auto);
        assert_eq!(control.brightness(), None);
        assert_eq!(control.saturation(), auto);
        assert_eq!(control.hue(), None);
    }

    #[test]
    fn a_third_layout_version_no_file_holds_or_of_another_length_gives_none() {
        let record = third(b"0310", b"STANDARD", b"STANDARD", third_values(&[0]));
        assert!(picture_control(&record).is_some());
        for version in [b"0320", b"0303", b"0311", b"0400"] {
            let mut other = record.clone();
            other[..4].copy_from_slice(version);
            assert_eq!(picture_control(&other), None, "{version:?}");
        }
        assert_eq!(picture_control(&record[..107]), None);
        assert_eq!(picture_control(&record[..68]), None);
        assert_eq!(picture_control(&record[..58]), None);
    }

    #[test]
    fn a_value_is_its_byte_less_0x80_in_whole_steps() {
        for (byte, expected) in [
            (0x80, steps(0, 1)),
            (0x8C, steps(12, 1)),
            (0x7F, steps(-1, 1)),
            (0x01, steps(-127, 1)),
            (0xFE, steps(126, 1)),
        ] {
            let record = standard([0, byte, byte, byte, byte, byte, byte, 0x80, 0x80, byte]);
            let control = picture_control(&record).unwrap();
            for (field, read) in [
                ("quick adjust", control.quick_adjust()),
                ("sharpening", control.sharpening()),
                ("contrast", control.contrast()),
                ("brightness", control.brightness()),
                ("saturation", control.saturation()),
                ("hue", control.hue()),
                ("toning saturation", control.toning_saturation()),
            ] {
                assert_eq!(read, expected, "{field}, byte {byte:#04x}");
            }
        }
    }

    #[test]
    fn zero_is_auto_where_the_menu_offers_it_and_none_elsewhere() {
        let control = picture_control(&standard([0; 10])).unwrap();
        let auto = Some(PictureControlValue::Auto);
        assert_eq!(control.sharpening(), auto);
        assert_eq!(control.contrast(), auto);
        assert_eq!(control.saturation(), auto);
        assert_eq!(control.quick_adjust(), None);
        assert_eq!(control.brightness(), None);
        assert_eq!(control.hue(), None);
        assert_eq!(control.toning_saturation(), None);
        // 0 is a code of the adjustment kind, and none of the filter
        // effect's or the toning's.
        assert_eq!(
            control.adjustment(),
            Some(PictureControlAdjustment::Default)
        );
        assert_eq!(control.filter_effect(), None);
        assert_eq!(control.toning(), None);
    }

    #[test]
    fn not_available_gives_no_value_and_costs_that_field_alone() {
        let control = picture_control(&standard([0xFF; 10])).unwrap();
        assert_eq!(
            control,
            PictureControl {
                name: Some("STANDARD".into()),
                base: Some("STANDARD".into()),
                ..PictureControl::default()
            }
        );
    }

    #[test]
    fn each_code_of_the_three_lists_gives_its_variant() {
        use PictureControlToning::*;
        for (code, expected) in [
            (0, PictureControlAdjustment::Default),
            (1, PictureControlAdjustment::QuickAdjust),
            (2, PictureControlAdjustment::FullControl),
        ] {
            assert_eq!(adjustment(code), Some(expected), "{code}");
        }
        assert_eq!(adjustment(3), None);
        for (code, expected) in [
            (0x80, PictureControlFilterEffect::Off),
            (0x81, PictureControlFilterEffect::Yellow),
            (0x82, PictureControlFilterEffect::Orange),
            (0x83, PictureControlFilterEffect::Red),
            (0x84, PictureControlFilterEffect::Green),
        ] {
            assert_eq!(filter_effect(code), Some(expected), "{code:#04x}");
        }
        for code in [0x00, 0x7F, 0x85, 0xFF] {
            assert_eq!(filter_effect(code), None, "{code:#04x}");
        }
        let tonings = [
            BlackAndWhite,
            Sepia,
            Cyanotype,
            Red,
            Yellow,
            Green,
            BlueGreen,
            Blue,
            PurpleBlue,
            RedPurple,
        ];
        for (code, expected) in (0x80..).zip(tonings) {
            assert_eq!(toning(code), Some(expected), "{code:#04x}");
        }
        for code in [0x00, 0x7F, 0x8A, 0xFF] {
            assert_eq!(toning(code), None, "{code:#04x}");
        }
    }

    #[test]
    fn a_divisor_of_zero_gives_no_value() {
        assert_eq!(value(0x84, 0, false), None);
        assert_eq!(value(0x84, 0, true), None);
        // Auto and not available need no divisor.
        assert_eq!(value(AUTO, 0, true), Some(PictureControlValue::Auto));
        assert_eq!(value(NOT_AVAILABLE, 4, true), None);
    }

    #[test]
    fn a_record_of_another_version_or_length_gives_none() {
        let record = standard([0, 0x80, 0x83, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF]);
        assert!(picture_control(&record).is_some());
        // One byte short, one byte long.
        assert_eq!(picture_control(&record[..57]), None);
        let mut long = record.clone();
        long.push(0);
        assert_eq!(picture_control(&long), None);
        // A version of other digits, at the first layout's length.
        for version in [b"0320", b"0101", b"0000", b"ABCD"] {
            let mut other = record.clone();
            other[..4].copy_from_slice(version);
            assert_eq!(picture_control(&other), None, "{version:?}");
        }
        // Too short for a version.
        assert_eq!(picture_control(b"010"), None);
        assert_eq!(picture_control(b""), None);
    }

    #[test]
    fn a_name_that_is_blank_or_not_ascii_is_none_and_the_rest_is_read() {
        let tail = [0, 0x80, 0x83, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF];
        for name in [&b""[..], b"    ", b"\0STANDARD", b"STAND\xC9RD", b"\xFF"] {
            let control = picture_control(&first(name, b"STANDARD", tail)).unwrap();
            assert_eq!(control.name(), None, "{name:?}");
            assert_eq!(control.base(), Some("STANDARD"), "{name:?}");
            assert_eq!(control.sharpening(), steps(3, 1), "{name:?}");
        }
        // The base by the same rule, the name kept.
        let control = picture_control(&first(b"STANDARD", b"STAND\x80RD", tail)).unwrap();
        assert_eq!(control.name(), Some("STANDARD"));
        assert_eq!(control.base(), None);
    }

    #[test]
    fn a_name_is_kept_as_typed_up_to_its_nul_or_its_field() {
        let tail = [0, 0xFF, 0x83, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF];
        let control = picture_control(&first(b"STANDARD-02", b"STANDARD", tail)).unwrap();
        assert_eq!(control.name(), Some("STANDARD-02"));
        // Padding spaces dropped, inner spaces and case kept.
        let control = picture_control(&first(b" my Vivid  ", b"VIVID", tail)).unwrap();
        assert_eq!(control.name(), Some("my Vivid"));
        // Bytes after the NUL are not part of the name.
        let control = picture_control(&first(b"FLAT\0XYZ", b"FLAT", tail)).unwrap();
        assert_eq!(control.name(), Some("FLAT"));
        // A name that fills its 20 bytes ends with the field: the base
        // after it is not part of it.
        let control = picture_control(&first(b"ABCDEFGHIJKLMNOPQRST", b"VIVID", tail)).unwrap();
        assert_eq!(control.name(), Some("ABCDEFGHIJKLMNOPQRST"));
        assert_eq!(control.base(), Some("VIVID"));
    }
}
