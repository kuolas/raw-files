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
};

/// The layout of a record whose first four bytes are `version`, among the
/// versions measured files hold: 0100, the first layout ([`FIRST`]).
/// `None` for every other version.
fn layout(version: &[u8]) -> Option<&'static Layout> {
    match version {
        b"0100" => Some(&FIRST),
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
///   layout records one, and over 1 where it does not. Sharpening,
///   clarity, contrast and saturation take auto; the others do not;
/// - the filter effect by [`filter_effect`] and the toning by
///   [`toning`].
///
/// `None` for a record of another version, or of another length than its
/// version's: no measured file shows which layout it takes. The four
/// bytes before the adjustment kind, a number for each preset that the
/// documentation does not name, are not read (NEF notes, section 8.11).
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
    Some(PictureControl {
        name: name(record, layout.name),
        base: name(record, layout.name + NAME_LEN),
        adjustment: byte(layout.adjustment).and_then(adjustment),
        quick_adjust: layout.quick_adjust.and_then(steps),
        quick_sharp: layout.quick_sharp.and_then(steps),
        sharpening: steps_or_auto(layout.sharpening),
        mid_range_sharpening: layout.mid_range_sharpening.and_then(steps),
        clarity: layout.clarity.and_then(steps_or_auto),
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
