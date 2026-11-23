// SPDX-License-Identifier: MIT OR Apache-2.0

//! The camera settings in the Nikon MakerNote: the white balance, the
//! shutter used, the image area, the raw size, the color space, the focus
//! mode, the AF-area mode, AF fine-tune and daylight saving time.
//!
//! The MakerNote records how the camera was set for the picture beyond
//! what the Exif tags record (ADR011). [`camera_settings`] reads each
//! setting from one tag, by its own rule:
//!
//! - the white balance mode from WhiteBalance's text
//!   ([`white_balance_of_text`]), finer than Exif's;
//! - the color temperature from ColorTemperatureAuto, else from a
//!   WhiteBalance text that records a chosen one ([`color_temperature`]);
//! - the fine-tuning from 0x003F, else from 0x000B ([`fine_tune`]);
//! - the shutter used from ShutterMode ([`shutter`]);
//! - the image area from CropHiSpeed's crop setting ([`image_area`]);
//! - the raw size from ImageSizeRAW ([`raw_image_size`]);
//! - the color space from ColorSpace ([`color_space`]);
//! - the focus mode from FocusMode's text ([`focus_mode_of_text`]);
//! - the AF-area mode from AFInfo2 ([`area_mode_of`]), else from AFInfo
//!   ([`af_info_area_mode`]);
//! - AF fine-tune from AFTune ([`af_fine_tune`]);
//! - daylight saving time from WorldTime ([`daylight_saving_time`]).
//!
//! A code or a text gives a value only where its meaning is known, as the
//! next paragraph says. Anything else costs that value alone.
//! Nothing the block holds is an error.
//!
//! The meaning of each code is ExifTool's, read in its published tag
//! documentation, or Nikon's for a camera's own text. The codes of the
//! settings of section 8.7 of the NEF notes were each confirmed on the
//! public samples and on frames I shot for these values with a Z 9 and a
//! D600. Those of sections 8.6 and 8.10, such as the focus modes, come
//! from lists the documentation gives, each read whole once the measured
//! files hold part of it: each code is confirmed by such a frame, observed
//! on files, or documented only, and the notes say which
//! (`docs/formats/nef/makernote-and-metadata.md`).

use std::io::{Read, Seek};

use super::af_info2::area_mode_of;
use super::block::MakerNote;
use super::crop::CropHiSpeed;
use super::rules;
use crate::camera_settings::{
    AfFineTune, CameraSettings, ColorSpace, FineTune, FocusAreaMode, FocusMode, ImageArea,
    RawImageSize, Shutter, WhiteBalanceMode,
};
use crate::error::Result;
use crate::formats::makernote::{Picture, checked_color_temperature};
use crate::formats::tiff_metadata;
use crate::io::Source;
use crate::rational::SignedRational;

/// How the camera was set for the picture: `settings`, what the standard
/// tags gave, with what `makernote` records. Each value is read by its own
/// rule, so that a damaged one costs only itself:
///
/// - **The white balance mode** is the one WhiteBalance's text names
///   ([`white_balance_of_text`]), in place of the Exif tags' mode, which
///   records only auto or the kind of light. The text is read once, by the
///   text rule ([`tiff_metadata::text`]). When it names no mode, the Exif
///   tags' mode stays.
/// - **The color temperature** is ColorTemperatureAuto's, or a chosen one
///   that the WhiteBalance text records ([`color_temperature`]).
/// - **The fine-tuning** is read from 0x003F, else from 0x000B
///   ([`fine_tune`]).
/// - **The shutter used** is ShutterMode's code ([`shutter`]).
/// - **The image area** is the crop setting of `crop_hi_speed`,
///   CropHiSpeed as [`crop_hi_speed`] read it ([`image_area`]).
/// - **The raw image size** is ImageSizeRAW's code ([`raw_image_size`]),
///   for a raw file's picture alone. In a JPEG, whose `picture` is
///   [`Picture::Jpeg`], the tag follows the JPEG's own size, and a JPEG
///   holds no raw data (ADR014).
/// - **The color space** is ColorSpace's code ([`color_space`]), only when
///   `exif_records_color_space` is `false`. Where the Exif IFD keeps a
///   ColorSpace, it alone decides
///   ([`Standard`](tiff_metadata::Standard)).
/// - **The focus mode** is the one FocusMode's text names
///   ([`focus_mode_of_text`]), read by the text rule. The focus point's
///   reader takes manual focus from it, and does not read the tag again
///   ([`focus`](super::focus)).
/// - **The AF-area mode** is the one `af_info2`, AFInfo2 as the MakerNote's
///   reader read it once, records ([`area_mode_of`]), whatever the focus
///   mode and whether or not the record gives a point. On a body without
///   AFInfo2 it is AFInfo's ([`af_info_area_mode`]). A body with AFInfo2
///   never falls back on AFInfo, which no file holds beside it.
/// - **AF fine-tune** is AFTune's ([`af_fine_tune`]).
/// - **Daylight saving time** is WorldTime's third byte
///   ([`daylight_saving_time`]), read from the tag that gives the capture
///   time its offset ([`world_time`](super::metadata::world_time)).
///
/// Nikon records no electronic front curtain setting, so it stays as
/// `settings` has it, `None`. The exposure program, the metering mode and
/// the flash are the Exif tags' alone.
///
/// # Errors
///
/// `Io` when the reader fails.
///
/// [`crop_hi_speed`]: super::crop::crop_hi_speed
pub(super) fn camera_settings<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    mut settings: CameraSettings,
    crop_hi_speed: Option<CropHiSpeed>,
    af_info2: Option<&[u8]>,
    exif_records_color_space: bool,
    picture: Picture,
) -> Result<CameraSettings> {
    let Some(ifd) = makernote.tagged() else {
        return Ok(settings);
    };
    // Read once, for the white balance mode and the color temperature.
    let white_balance = tiff_metadata::text(src, ifd, rules::WHITE_BALANCE)?;
    let white_balance = white_balance.as_deref();
    if let Some(mode) = white_balance.and_then(white_balance_of_text) {
        settings.white_balance.mode = Some(mode);
    }
    settings.white_balance.color_temperature = color_temperature(makernote, white_balance);
    settings.white_balance.fine_tune = fine_tune(src, makernote)?;
    settings.shutter = ifd
        .held(rules::SHUTTER_MODE)
        .and_then(|[code]| shutter(code));
    settings.image_area = crop_hi_speed.and_then(|crop| image_area(crop.mode));
    settings.raw_image_size = match picture {
        Picture::Raw => ifd
            .held(rules::IMAGE_SIZE_RAW)
            .and_then(|[code]| raw_image_size(code)),
        Picture::Jpeg => None,
    };
    if !exif_records_color_space {
        settings.color_space = ifd
            .held(rules::COLOR_SPACE)
            .and_then(|[code]| color_space(code));
    }
    settings.focus_mode = tiff_metadata::text(src, ifd, rules::FOCUS_MODE)?
        .and_then(|text| focus_mode_of_text(&text));
    settings.focus_area_mode = match ifd.entry(super::AF_INFO2) {
        Some(_) => af_info2.and_then(area_mode_of),
        None => ifd
            .held(rules::AF_INFO)
            .and_then(|[code, ..]| af_info_area_mode(code)),
    };
    settings.af_fine_tune = ifd.held(rules::AF_TUNE).and_then(af_fine_tune);
    settings.daylight_saving_time = ifd
        .held(rules::WORLD_TIME)
        .and_then(|[_, _, daylight_saving, _]| daylight_saving_time(daylight_saving));
    Ok(settings)
}

/// The color temperature, in kelvin, of the white balance `makernote`
/// records: ColorTemperatureAuto, when it gives one; otherwise the number
/// of `white_balance`, the WhiteBalance text, when that text records a
/// chosen color temperature ([`kelvin_of_text`]). Either gives one only
/// from 1 to 99,999 K ([`checked_color_temperature`]). So a
/// ColorTemperatureAuto of 0, which records none, gives way to the text.
///
/// ColorTemperatureAuto comes first, under whichever setting: it records
/// the temperature the camera applied, a chosen one included. The Z 9
/// writes both, 3230 beside `3230K`. The text is there for the bodies
/// before the D5 and the D850, which write no ColorTemperatureAuto: it is
/// all they record of a temperature, `3230K` on the D600 too.
fn color_temperature(makernote: &MakerNote, white_balance: Option<&str>) -> Option<u32> {
    let recorded = makernote
        .tagged()
        .and_then(|ifd| ifd.held(rules::COLOR_TEMPERATURE))
        .and_then(|[kelvin]| checked_color_temperature(kelvin));
    recorded.or_else(|| {
        white_balance
            .and_then(kelvin_of_text)
            .and_then(checked_color_temperature)
    })
}

/// The white balance mode that the WhiteBalance text `text`, as read,
/// names. Its words cannot be guessed from one another, such as `COOL WHT
/// FL` for a cool white fluorescent lamp. So each is looked up whole in a
/// table of the texts that checked files hold:
///
/// - `AUTO`, `AUTO0`, `AUTO1`, `AUTO2` and `NATURAL AUTO` give
///   [`Auto`](WhiteBalanceMode::Auto);
/// - `SUNNY` [`Daylight`](WhiteBalanceMode::Daylight), `CLOUDY`
///   [`Cloudy`](WhiteBalanceMode::Cloudy) and `SHADE`
///   [`Shade`](WhiteBalanceMode::Shade);
/// - `INCANDESCENT` [`Tungsten`](WhiteBalanceMode::Tungsten) and `COOL WHT
///   FL` [`Fluorescent`](WhiteBalanceMode::Fluorescent);
/// - `FLASH` [`Flash`](WhiteBalanceMode::Flash);
/// - `PRESET` and `PRESET1` to `PRESET4`
///   [`Custom`](WhiteBalanceMode::Custom).
///
/// A chosen color temperature, written as its number, is the one text read
/// by its form ([`kelvin_of_text`]). Whatever the number, it gives
/// [`ColorTemperature`](WhiteBalanceMode::ColorTemperature).
///
/// `None` for every other text, such as `PRESET5`, which no checked file
/// holds. A text joins the table only once a file shows it, and until then
/// the Exif tags' mode stands.
fn white_balance_of_text(text: &str) -> Option<WhiteBalanceMode> {
    Some(match text {
        "AUTO" | "AUTO0" | "AUTO1" | "AUTO2" | "NATURAL AUTO" => WhiteBalanceMode::Auto,
        "SUNNY" => WhiteBalanceMode::Daylight,
        "CLOUDY" => WhiteBalanceMode::Cloudy,
        "SHADE" => WhiteBalanceMode::Shade,
        "INCANDESCENT" => WhiteBalanceMode::Tungsten,
        "COOL WHT FL" => WhiteBalanceMode::Fluorescent,
        "FLASH" => WhiteBalanceMode::Flash,
        "PRESET" | "PRESET1" | "PRESET2" | "PRESET3" | "PRESET4" => WhiteBalanceMode::Custom,
        _ if kelvin_of_text(text).is_some() => WhiteBalanceMode::ColorTemperature,
        _ => return None,
    })
}

/// The most digits a WhiteBalance text that records a chosen color
/// temperature may have: the rule's own bound, past the four of every such
/// text checked. Its largest number is the bound of every color
/// temperature a MakerNote gives
/// ([`MAX_COLOR_TEMPERATURE`](super::super::MAX_COLOR_TEMPERATURE)).
const MAX_KELVIN_DIGITS: usize = 5;

// The text's largest number is the shared bound, so that one bound holds
// whatever a WhiteBalance text gives.
const _: () =
    assert!(10_u32.pow(MAX_KELVIN_DIGITS as u32) - 1 == super::super::MAX_COLOR_TEMPERATURE);

/// The color temperature that the WhiteBalance text `text`, as read,
/// records as its number when the photographer chose one: `3230K` for
/// 3230 K. That is one to [`MAX_KELVIN_DIGITS`] ASCII digits, then an
/// upper-case `K`, and nothing else, so `K` alone, `3230 K` and `3230k`
/// give `None`.
///
/// `0K` has the form and gives 0, which records no temperature. Its caller
/// judges the number ([`color_temperature`]), while the form alone names
/// the mode ([`white_balance_of_text`]).
fn kelvin_of_text(text: &str) -> Option<u32> {
    let digits = text.strip_suffix('K')?;
    if !(1..=MAX_KELVIN_DIGITS).contains(&digits.len())
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    // At most five ASCII digits always parse into a u32.
    digits.parse().ok()
}

/// The white balance fine-tuning that `makernote` records, signed as
/// [`FineTune`] signs it ([`fine_tune_of`]):
///
/// - from 0x003F, two SRATIONALs in the camera's own steps, as stored,
///   such as `-12/4`;
/// - else, when 0x003F is missing, breaks its rule or gives none, from
///   0x000B, two SSHORTs in whole steps, each read as a fraction over 1.
///
/// Either holds the shift from amber to blue, then the one from magenta to
/// green. A recorded zero shift, such as `0/4, 0/4`, is a fine-tuning like
/// any other. A 0x000B of one value breaks its rule and gives none: no
/// file shows its axis or its sign.
///
/// # Errors
///
/// `Io` when the reader fails.
fn fine_tune<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<FineTune>> {
    let Some(ifd) = makernote.tagged() else {
        return Ok(None);
    };
    let fraction = |value: SignedRational| (value.numerator, value.denominator);
    if let Some([amber_blue, magenta_green]) = ifd.get(src, rules::FINE_TUNE)?
        && let Some(fine_tune) = fine_tune_of(fraction(amber_blue), fraction(magenta_green))
    {
        return Ok(Some(fine_tune));
    }
    let Some([amber_blue, magenta_green]) = ifd.held(rules::FINE_TUNE_STEPS) else {
        return Ok(None);
    };
    Ok(fine_tune_of(
        (i32::from(amber_blue), 1),
        (i32::from(magenta_green), 1),
    ))
}

/// The fine-tuning of Nikon's two values, `amber_blue` and
/// `magenta_green`, each a numerator and a denominator as stored. The first
/// is kept, and the second has its numerator negated.
///
/// Nikon stores a shift towards blue and one towards green as positive, as
/// frames shot on a Z 9 and a D600 with a known shift show. [`FineTune`]
/// signs each value as its axis is named, so its green to magenta shift is
/// positive towards magenta, and Nikon's second value is turned around
/// (ADR011).
///
/// `None` when either denominator is zero, which records no fraction, and
/// when the second numerator is `i32::MIN`, whose negation does not fit. No
/// file checked holds either.
fn fine_tune_of(
    (amber_blue, amber_blue_denominator): (i32, i32),
    (magenta_green, magenta_green_denominator): (i32, i32),
) -> Option<FineTune> {
    let amber_blue = SignedRational::new(amber_blue, amber_blue_denominator).ok()?;
    let green_magenta =
        SignedRational::new(magenta_green.checked_neg()?, magenta_green_denominator).ok()?;
    Some(FineTune::new(amber_blue, green_magenta))
}

/// The shutter a ShutterMode `code` names: 0 the mechanical shutter, 16 the
/// electronic one, and the auto setting's codes for the shutter the camera
/// chose, 80 the mechanical shutter and 81 the electronic front curtain.
///
/// `None` for every other code. 64 is a movie's electronic shutter, which a
/// still does not record. ExifTool's documentation names 48 the electronic
/// front curtain and 96 a high-speed electronic shutter, but no checked
/// file holds either.
fn shutter(code: u32) -> Option<Shutter> {
    Some(match code {
        0 | 80 => Shutter::Mechanical,
        16 => Shutter::Electronic,
        81 => Shutter::ElectronicFrontCurtain,
        _ => return None,
    })
}

/// The image area a CropHiSpeed crop setting `mode` names:
///
/// - 11, FX uncropped, [`FullFrame`](ImageArea::FullFrame);
/// - 2, a DX crop, and 12, a DX body uncropped, [`ApsC`](ImageArea::ApsC);
/// - 17, FX 1:1, [`Square`](ImageArea::Square), and 18, DX 1:1,
///   [`ApsCSquare`](ImageArea::ApsCSquare);
/// - 6, 16:9, [`Wide`](ImageArea::Wide);
/// - 9, which ExifTool's documentation names a DX movie's 16:9 crop and DX
///   bodies write on still pictures, [`ApsCWide`](ImageArea::ApsCWide);
/// - 1, the 1.3x crop, and 10, the 1.3x movie crop, which a DX body writes
///   on still pictures too, [`Crop1_3x`](ImageArea::Crop1_3x).
///
/// `None` for 0, "off", which FX and DX bodies both write, so it does not
/// say which sensor the body has. `None` too for 3, a 5:4 crop, 4, a 3:2
/// crop, and 8, a 2.7x crop, which no checked file holds; for the other
/// movie crops; and for every code nothing names.
pub(super) fn image_area(mode: u32) -> Option<ImageArea> {
    Some(match mode {
        11 => ImageArea::FullFrame,
        2 | 12 => ImageArea::ApsC,
        17 => ImageArea::Square,
        18 => ImageArea::ApsCSquare,
        6 => ImageArea::Wide,
        9 => ImageArea::ApsCWide,
        1 | 10 => ImageArea::Crop1_3x,
        _ => return None,
    })
}

/// The raw size an ImageSizeRAW `code` names: 1
/// [`Large`](RawImageSize::Large) and 3 [`Small`](RawImageSize::Small).
/// `None` for 2, the medium size, which no checked raw file holds, for 0,
/// which no table names, and for every other code.
fn raw_image_size(code: u32) -> Option<RawImageSize> {
    Some(match code {
        1 => RawImageSize::Large,
        3 => RawImageSize::Small,
        _ => return None,
    })
}

/// The focus mode that the FocusMode text `text`, as read, names. Each
/// text is looked up whole in a table of the texts that files hold, as the
/// white balance texts are ([`white_balance_of_text`]):
///
/// - `MANUAL` [`Manual`](FocusMode::Manual);
/// - `AF-S`, single-servo AF, [`Single`](FocusMode::Single);
/// - `AF-C`, continuous-servo AF, [`Continuous`](FocusMode::Continuous);
/// - `AF-A`, auto-servo AF, [`Automatic`](FocusMode::Automatic);
/// - `AF-F`, full-time AF, [`FullTime`](FocusMode::FullTime).
///
/// The bodies pad an autofocus mode with spaces to six characters, which
/// the text rule trims. `None` for every other text, such as `manual` in
/// lower case.
fn focus_mode_of_text(text: &str) -> Option<FocusMode> {
    Some(match text {
        "MANUAL" => FocusMode::Manual,
        "AF-S" => FocusMode::Single,
        "AF-C" => FocusMode::Continuous,
        "AF-A" => FocusMode::Automatic,
        "AF-F" => FocusMode::FullTime,
        _ => return None,
    })
}

/// The AF-area mode that AFInfo's first byte, `code`, names, in the list
/// ExifTool's published documentation gives, read as documentation and
/// read whole, since files hold part of it:
///
/// - 0, single area, and 4, single area with the wide focus frame,
///   [`Single`](FocusAreaMode::Single);
/// - 1, dynamic area, and 5, dynamic area with the wide focus frame,
///   [`Dynamic`](FocusAreaMode::Dynamic);
/// - 2, dynamic area with the closest subject, the camera picking the
///   point, [`Auto`](FocusAreaMode::Auto);
/// - 3, group dynamic, a group of points the photographer places, in
///   which the camera chooses, [`Zone`](FocusAreaMode::Zone).
///
/// The bodies without AFInfo2, from the D1 to the D80 and the Coolpix
/// bodies, write 0 or 1, and the public D70 JPEG writes 2; 3 to 5 are
/// documented only (NEF notes, section 8.6). `None` for every other code.
fn af_info_area_mode(code: u8) -> Option<FocusAreaMode> {
    Some(match code {
        0 | 4 => FocusAreaMode::Single,
        1 | 5 => FocusAreaMode::Dynamic,
        2 => FocusAreaMode::Auto,
        3 => FocusAreaMode::Zone,
        _ => return None,
    })
}

/// The AF fine-tune that AFTune's four bytes record: the first says
/// whether it is on, 0 off, 1 or 2 on, 3 on for a zoom tuned at both ends;
/// the third is the value, signed, given when it is on; the fourth is the
/// value at the zoom's long end, signed, given under 3 alone. The second,
/// the saved lens's index, is not given.
///
/// ExifTool's published documentation, read as documentation, names the
/// codes, and calls the fourth byte valid only under 3. The list is read
/// whole, since files hold part of it: my D600 writes 0, 1 and 2, and 3
/// is documented only (NEF notes, section 8.10). What tells 1 from 2 is
/// not documented, and both read as on. `None` for any other first byte.
fn af_fine_tune([on, _lens, value, tele_value]: [u8; 4]) -> Option<AfFineTune> {
    let value = i32::from(value.cast_signed());
    match on {
        0 => Some(AfFineTune::off()),
        1 | 2 => Some(AfFineTune::on().with_value(value)),
        3 => Some(
            AfFineTune::on()
                .with_value(value)
                .with_tele_value(i32::from(tele_value.cast_signed())),
        ),
        _ => None,
    }
}

/// Whether daylight saving time was on, as WorldTime's third byte,
/// `daylight_saving`, records it: 0 off, `false`, and 1 on, `true`
/// (ExifTool's published tag documentation, read as documentation; NEF
/// notes, section 8.2). The files agree: where the byte is 1, the zone
/// plus an hour equals the Exif offset (section 8.10). `None` for every
/// other value, which no file holds, as the offset's reader refuses it.
fn daylight_saving_time(daylight_saving: u8) -> Option<bool> {
    match daylight_saving {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// The color space a ColorSpace `code` of the MakerNote names: 1
/// [`Srgb`](ColorSpace::Srgb) and 2 [`AdobeRgb`](ColorSpace::AdobeRgb).
/// `None` for 4, BT.2100, which no variant names and no checked file
/// holds, and for every other code.
fn color_space(code: u32) -> Option<ColorSpace> {
    Some(match code {
        1 => ColorSpace::Srgb,
        2 => ColorSpace::AdobeRgb,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::block::read;
    use super::super::testing::{
        ORDERS, Tags, entry_at, exif_file, exif_metadata, jpeg_metadata_of, metadata_of,
        nikon_file, retyped, tags_block, whole,
    };
    use super::super::{
        AF_INFO, AF_INFO2, AF_TUNE, COLOR_SPACE, COLOR_TEMPERATURE, CROP_HI_SPEED, FINE_TUNE,
        FINE_TUNE_STEPS, FOCUS_MODE, IMAGE_SIZE_RAW, SHUTTER_MODE, WHITE_BALANCE, WORLD_TIME,
        metadata,
    };
    use super::*;
    use crate::camera_settings::{ExposureProgram, WhiteBalance};
    use crate::container::tiff::tags::{
        COLOR_SPACE as EXIF_COLOR_SPACE, EXPOSURE_PROGRAM, INTEROP_INDEX, LIGHT_SOURCE, MAKER_NOTE,
        WHITE_BALANCE as EXIF_WHITE_BALANCE,
    };
    use crate::container::tiff::{self, walk};
    use crate::error::ErrorKind;
    use crate::limits::Limits;
    use crate::testing::{FAR, FailsFar, signed_fraction};
    use crate::tiff_builder::{BYTE, IfdId, Order, RawValue, SSHORT, TiffBuilder, UNDEFINED};
    use std::io::Cursor;

    // The camera settings the MakerNote records. Each test builds a file
    // whose Exif IFD holds no settings tag unless it says so, so that a
    // value it reads comes from the MakerNote.

    /// The camera settings of a raw file in byte order `order` whose
    /// MakerNote holds what `tags` adds to its MakerNote IFD, and whose
    /// Exif IFD holds what `exif` adds.
    fn settings_with_exif(
        order: Order,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> CameraSettings {
        exif_metadata(order, &tags_block(order, tags).0, exif).camera_settings
    }

    /// The camera settings of a raw file in byte order `order` whose
    /// MakerNote holds what `tags` adds to its MakerNote IFD, and whose
    /// Exif IFD holds no settings tag.
    fn settings_of(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> CameraSettings {
        settings_with_exif(order, tags, |_, _| {})
    }

    /// The camera settings of a JPEG of 6048 by 4024 pixels in byte order
    /// `order` whose MakerNote holds what `tags` adds to its MakerNote
    /// IFD, and whose Exif IFD holds what `exif` adds.
    fn jpeg_settings(
        order: Order,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> CameraSettings {
        let block = tags_block(order, tags).0;
        let file = exif_file(order, &block, whole(&block), exif);
        jpeg_metadata_of(file, [6048, 4024]).camera_settings
    }

    /// `text` padded with spaces to 12 characters, as Nikon writes its
    /// white balance texts; the builder adds the NUL.
    fn padded(text: &str) -> String {
        format!("{text:<12}")
    }

    /// The white balance mode of a raw file in byte order `order` whose
    /// MakerNote's WhiteBalance is `text`, padded, and whose Exif IFD holds
    /// what `exif` adds.
    fn white_balance_of(
        order: Order,
        text: &str,
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Option<WhiteBalanceMode> {
        settings_with_exif(
            order,
            |inner, ifd| {
                inner.ascii(ifd, WHITE_BALANCE, &padded(text));
            },
            exif,
        )
        .white_balance
        .mode
    }

    /// Gives the Exif IFD an auto white balance: WhiteBalance 0, with the
    /// LightSource 0, "unknown", beside it, as Nikon writes it.
    fn exif_auto(builder: &mut TiffBuilder, exif: IfdId) {
        builder
            .short(exif, EXIF_WHITE_BALANCE, &[0])
            .short(exif, LIGHT_SOURCE, &[0]);
    }

    /// Gives the Exif IFD a manual white balance for shade: WhiteBalance 1
    /// and LightSource 11.
    fn exif_shade(builder: &mut TiffBuilder, exif: IfdId) {
        builder
            .short(exif, EXIF_WHITE_BALANCE, &[1])
            .short(exif, LIGHT_SOURCE, &[11]);
    }

    #[test]
    fn each_white_balance_text_of_the_table_gives_its_mode() {
        let table = [
            ("AUTO", WhiteBalanceMode::Auto),
            ("AUTO0", WhiteBalanceMode::Auto),
            ("AUTO1", WhiteBalanceMode::Auto),
            ("AUTO2", WhiteBalanceMode::Auto),
            ("NATURAL AUTO", WhiteBalanceMode::Auto),
            ("SUNNY", WhiteBalanceMode::Daylight),
            ("CLOUDY", WhiteBalanceMode::Cloudy),
            ("SHADE", WhiteBalanceMode::Shade),
            ("INCANDESCENT", WhiteBalanceMode::Tungsten),
            ("COOL WHT FL", WhiteBalanceMode::Fluorescent),
            ("FLASH", WhiteBalanceMode::Flash),
            ("PRESET", WhiteBalanceMode::Custom),
            ("PRESET1", WhiteBalanceMode::Custom),
            ("PRESET2", WhiteBalanceMode::Custom),
            ("PRESET3", WhiteBalanceMode::Custom),
            ("PRESET4", WhiteBalanceMode::Custom),
        ];
        for order in ORDERS {
            for (text, mode) in table {
                let read = white_balance_of(order, text, |_, _| {});
                assert_eq!(read, Some(mode), "{order:?}, {text:?}");
            }
        }
    }

    #[test]
    fn the_makernotes_white_balance_mode_wins_over_the_exif_tags() {
        // Exif's manual white balance for shade, beside a MakerNote that
        // names another setting: the MakerNote records the setting itself.
        for order in ORDERS {
            for (text, mode) in [
                ("SUNNY", WhiteBalanceMode::Daylight),
                ("PRESET1", WhiteBalanceMode::Custom),
                ("3230K", WhiteBalanceMode::ColorTemperature),
            ] {
                let read = white_balance_of(order, text, exif_shade);
                assert_eq!(read, Some(mode), "{order:?}, {text:?}");
            }
        }
    }

    #[test]
    fn a_chosen_color_temperature_is_read_by_its_form() {
        for order in ORDERS {
            for text in ["3230K", "2500K", "10000K", "1K", "99999K"] {
                let read = white_balance_of(order, text, exif_auto);
                assert_eq!(
                    read,
                    Some(WhiteBalanceMode::ColorTemperature),
                    "{order:?}, {text:?}"
                );
            }
        }
    }

    #[test]
    fn a_white_balance_text_not_in_the_table_leaves_the_exif_tags_mode() {
        let unknown = [
            // A preset slot no checked file shows, and an auto setting.
            "PRESET5", "AUTO3",
            // The table's texts in other cases, or cut short: no prefix
            // or case rule.
            "Auto1", "SUN", "PRESET1X", // Near a color temperature, but not its form.
            "K", "123456K", "3230k", "3230 K", "K3230", "32.5K",
        ];
        for order in ORDERS {
            for text in unknown {
                let what = format!("{order:?}, {text:?}");
                assert_eq!(
                    white_balance_of(order, text, exif_auto),
                    Some(WhiteBalanceMode::Auto),
                    "{what}"
                );
                assert_eq!(
                    white_balance_of(order, text, exif_shade),
                    Some(WhiteBalanceMode::Shade),
                    "{what}"
                );
                assert_eq!(white_balance_of(order, text, |_, _| {}), None, "{what}");
            }
        }
    }

    #[test]
    fn a_white_balance_text_is_trimmed_and_cut_at_its_nul() {
        // Spaces on either side are padding, as the text rule says, and
        // what follows a NUL is not part of the text.
        for order in ORDERS {
            for text in ["  SUNNY  ", "SUNNY\0CLOUDY", "SUNNY"] {
                let read = settings_of(order, |inner, ifd| {
                    inner.ascii(ifd, WHITE_BALANCE, text);
                });
                assert_eq!(
                    read.white_balance.mode,
                    Some(WhiteBalanceMode::Daylight),
                    "{order:?}, {text:?}"
                );
            }
        }
    }

    #[test]
    fn a_white_balance_of_another_type_is_not_read() {
        for order in ORDERS {
            let text = padded("SUNNY");
            let count = u32::try_from(text.len() + 1).unwrap();
            for typ in [UNDEFINED, BYTE] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.ascii(ifd, WHITE_BALANCE, &text);
                });
                let block = retyped(order, block, entry, typ, count);
                let read = exif_metadata(order, &block, exif_shade).camera_settings;
                assert_eq!(
                    read.white_balance.mode,
                    Some(WhiteBalanceMode::Shade),
                    "{order:?}, type {typ}"
                );
            }
        }
    }

    #[test]
    fn a_white_balance_past_the_block_is_not_read() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, WHITE_BALANCE, &padded("SUNNY"));
            })
            .0;
            // The text and its NUL are the block's last 13 bytes.
            assert_eq!(block[block.len() - 13..], *b"SUNNY       \0", "{order:?}");

            // Declared one byte short: the NUL lies in the file, after the
            // block, so the entry is dropped and Exif's mode stays.
            let file = exif_file(order, &block, whole(&block) - 1, exif_shade);
            let read = metadata_of(file).camera_settings.white_balance.mode;
            assert_eq!(read, Some(WhiteBalanceMode::Shade), "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), exif_shade);
            let read = metadata_of(file).camera_settings.white_balance.mode;
            assert_eq!(read, Some(WhiteBalanceMode::Daylight), "{order:?}");
        }
    }

    #[test]
    fn a_color_temperature_above_zero_is_read_under_any_white_balance() {
        // The Z 9's frames: the temperature auto chose, and the one
        // applied under daylight, a preset and a chosen temperature.
        for order in ORDERS {
            for (text, temperature) in [
                ("AUTO1", 4730),
                ("SUNNY", 5210),
                ("PRESET1", 5200),
                ("3230K", 3230),
                ("AUTO0", u16::MAX),
            ] {
                let read = settings_of(order, |inner, ifd| {
                    inner.ascii(ifd, WHITE_BALANCE, &padded(text)).short(
                        ifd,
                        COLOR_TEMPERATURE,
                        &[temperature],
                    );
                });
                assert_eq!(
                    read.white_balance.color_temperature,
                    Some(u32::from(temperature)),
                    "{order:?}, {text:?}"
                );
            }
        }
    }

    #[test]
    fn a_color_temperature_of_zero_is_none() {
        // As the D850 and the Z fc write it under daylight.
        for order in ORDERS {
            let read = settings_of(order, |inner, ifd| {
                inner.ascii(ifd, WHITE_BALANCE, &padded("SUNNY")).short(
                    ifd,
                    COLOR_TEMPERATURE,
                    &[0],
                );
            });
            assert_eq!(read.white_balance.color_temperature, None, "{order:?}");
            assert_eq!(read.white_balance.mode, Some(WhiteBalanceMode::Daylight));
        }
    }

    #[test]
    fn without_a_color_temperature_tag_a_chosen_one_is_read_from_the_text() {
        // As the D600 records a chosen 3230 K, by its text alone; and the
        // ends of the text's form, which counts leading zeros as digits.
        for order in ORDERS {
            for (text, temperature) in [
                ("3230K", 3230),
                ("1K", 1),
                ("99999K", 99_999),
                ("00500K", 500),
            ] {
                let read = settings_of(order, |inner, ifd| {
                    inner.ascii(ifd, WHITE_BALANCE, &padded(text));
                });
                let what = format!("{order:?}, {text:?}");
                assert_eq!(
                    read.white_balance.color_temperature,
                    Some(temperature),
                    "{what}"
                );
                assert_eq!(
                    read.white_balance.mode,
                    Some(WhiteBalanceMode::ColorTemperature),
                    "{what}"
                );
            }
        }
    }

    #[test]
    fn the_color_temperature_tag_wins_over_the_text() {
        // The Z 9 writes both, and they agree; were they to differ, the
        // temperature the camera applied is the tag's.
        for order in ORDERS {
            let read = settings_of(order, |inner, ifd| {
                inner.ascii(ifd, WHITE_BALANCE, &padded("3230K")).short(
                    ifd,
                    COLOR_TEMPERATURE,
                    &[4630],
                );
            });
            assert_eq!(
                read.white_balance.color_temperature,
                Some(4630),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_color_temperature_tag_that_gives_none_gives_way_to_the_text() {
        // 0, which records none, and a LONG and two SHORTs, which are not
        // read: each costs only itself, and the text stands in.
        let unusable: [(&str, Tags); 3] = [
            ("0", |inner, ifd| {
                inner.short(ifd, COLOR_TEMPERATURE, &[0]);
            }),
            ("a LONG", |inner, ifd| {
                inner.long(ifd, COLOR_TEMPERATURE, &[4630]);
            }),
            ("two SHORTs", |inner, ifd| {
                inner.short(ifd, COLOR_TEMPERATURE, &[4630, 4630]);
            }),
        ];
        for order in ORDERS {
            for (what, tag) in unusable {
                let read = settings_of(order, |inner, ifd| {
                    inner.ascii(ifd, WHITE_BALANCE, &padded("3230K"));
                    tag(inner, ifd);
                });
                assert_eq!(
                    read.white_balance.color_temperature,
                    Some(3230),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_text_of_zero_kelvin_or_of_another_form_gives_no_color_temperature() {
        // `0K` has the form of a chosen color temperature, and names the
        // mode, but records no temperature; a text of another form records
        // none either, and without the tag there is none.
        for order in ORDERS {
            for (text, mode) in [
                ("0K", Some(WhiteBalanceMode::ColorTemperature)),
                ("00000K", Some(WhiteBalanceMode::ColorTemperature)),
                ("SUNNY", Some(WhiteBalanceMode::Daylight)),
                ("123456K", None),
                ("3230 K", None),
                ("3230k", None),
            ] {
                let read = settings_of(order, |inner, ifd| {
                    inner.ascii(ifd, WHITE_BALANCE, &padded(text));
                });
                let what = format!("{order:?}, {text:?}");
                assert_eq!(read.white_balance.color_temperature, None, "{what}");
                assert_eq!(read.white_balance.mode, mode, "{what}");
            }
            // And with a tag of 0 beside `0K`.
            let read = settings_of(order, |inner, ifd| {
                inner
                    .ascii(ifd, WHITE_BALANCE, &padded("0K"))
                    .short(ifd, COLOR_TEMPERATURE, &[0]);
            });
            assert_eq!(read.white_balance.color_temperature, None, "{order:?}");
        }
    }

    #[test]
    fn each_one_short_setting_of_another_type_or_count_is_not_read() {
        // ColorTemperatureAuto, ShutterMode, ImageSizeRAW and ColorSpace,
        // each with a value its code names, as a LONG and as two SHORTs.
        let tags = [
            (COLOR_TEMPERATURE, 4630),
            (SHUTTER_MODE, 16),
            (IMAGE_SIZE_RAW, 1),
            (COLOR_SPACE, 2),
        ];
        // What is read of each, as a control with one SHORT, and with the
        // value of another shape.
        let read = |settings: &CameraSettings| {
            (
                settings.white_balance.color_temperature,
                settings.shutter,
                settings.raw_image_size,
                settings.color_space,
            )
        };
        for order in ORDERS {
            let control = settings_of(order, |inner, ifd| {
                for (tag, value) in tags {
                    inner.short(ifd, tag, &[value]);
                }
            });
            assert_eq!(
                read(&control),
                (
                    Some(4630),
                    Some(Shutter::Electronic),
                    Some(RawImageSize::Large),
                    Some(ColorSpace::AdobeRgb)
                ),
                "{order:?}"
            );
            let longs = settings_of(order, |inner, ifd| {
                for (tag, value) in tags {
                    inner.long(ifd, tag, &[u32::from(value)]);
                }
            });
            assert_eq!(read(&longs), (None, None, None, None), "{order:?}, LONG");
            let two = settings_of(order, |inner, ifd| {
                for (tag, value) in tags {
                    inner.short(ifd, tag, &[value, value]);
                }
            });
            assert_eq!(read(&two), (None, None, None, None), "{order:?}, two");
            let none = settings_of(order, |inner, ifd| {
                for (tag, _) in tags {
                    inner.short(ifd, tag, &[]);
                }
            });
            assert_eq!(read(&none), (None, None, None, None), "{order:?}, none");
        }
    }

    /// A shift of `amber_blue` and `green_magenta`, each a numerator and a
    /// denominator, as [`FineTune`] holds it.
    fn shift(amber_blue: (i32, i32), green_magenta: (i32, i32)) -> FineTune {
        let (amber_blue, green_magenta) = (
            signed_fraction(amber_blue.0, amber_blue.1),
            signed_fraction(green_magenta.0, green_magenta.1),
        );
        FineTune::new(amber_blue, green_magenta)
    }

    /// Gives `ifd` the tag `tag` as one or two SSHORTs, `values`, which its
    /// entry holds, in the builder's byte order `order`.
    fn sshorts(inner: &mut TiffBuilder, order: Order, ifd: IfdId, tag: u16, values: &[i16]) {
        assert!(values.len() <= 2, "an entry holds two SSHORTs at most");
        let mut field = [0; 4];
        for (at, &value) in values.iter().enumerate() {
            field[2 * at..2 * at + 2].copy_from_slice(&order.u16(value.cast_unsigned()));
        }
        let count = u32::try_from(values.len()).unwrap();
        inner.raw(ifd, tag, SSHORT, count, RawValue::Inline(field));
    }

    /// The fine-tuning of a raw file in byte order `order` whose MakerNote
    /// holds what `tags` adds, given the builder, the byte order and the
    /// MakerNote IFD.
    fn fine_tune_of_file(
        order: Order,
        tags: impl FnOnce(&mut TiffBuilder, Order, IfdId),
    ) -> Option<FineTune> {
        settings_of(order, |inner, ifd| tags(inner, order, ifd))
            .white_balance
            .fine_tune
    }

    #[test]
    fn a_fine_tune_in_fractions_keeps_amber_blue_and_turns_green_magenta_around() {
        // The Z 9's frames: `-12/4, 8/4` warmer and greener than the frame
        // without a shift, `8/4, -8/4` cooler and more magenta. Towards
        // amber and green is negative on both axes as raw-files signs them.
        for order in ORDERS {
            for (stored, expected) in [
                ([(-12, 4), (8, 4)], shift((-12, 4), (-8, 4))),
                ([(8, 4), (-8, 4)], shift((8, 4), (8, 4))),
                // A numerator and denominator are kept as stored, signs
                // included, and the first value is never negated.
                ([(i32::MIN, 4), (3, -4)], shift((i32::MIN, 4), (-3, -4))),
            ] {
                let read = fine_tune_of_file(order, |inner, _, ifd| {
                    inner.srational(ifd, FINE_TUNE, &stored);
                });
                assert_eq!(read, Some(expected), "{order:?}, {stored:?}");
            }
        }
    }

    #[test]
    fn a_recorded_zero_shift_is_a_fine_tune() {
        for order in ORDERS {
            let fractions = fine_tune_of_file(order, |inner, _, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(0, 4), (0, 4)]);
            });
            assert_eq!(fractions, Some(shift((0, 4), (0, 4))), "{order:?}");
            let steps = fine_tune_of_file(order, |inner, order, ifd| {
                sshorts(inner, order, ifd, FINE_TUNE_STEPS, &[0, 0]);
            });
            assert_eq!(steps, Some(shift((0, 1), (0, 1))), "{order:?}");
        }
    }

    #[test]
    fn without_fractions_a_fine_tune_in_whole_steps_is_read() {
        // The D600's frames: `3, 0` cooler than `-3, 0`, `0, 3` greener
        // than `0, -3`; and the extremes of an SSHORT, which negate
        // within 32 bits.
        for order in ORDERS {
            for (stored, expected) in [
                ([3, 0], shift((3, 1), (0, 1))),
                ([0, 3], shift((0, 1), (-3, 1))),
                ([-6, -3], shift((-6, 1), (3, 1))),
                (
                    [i16::MIN, i16::MIN],
                    shift((i32::from(i16::MIN), 1), (32768, 1)),
                ),
            ] {
                let read = fine_tune_of_file(order, |inner, order, ifd| {
                    sshorts(inner, order, ifd, FINE_TUNE_STEPS, &stored);
                });
                assert_eq!(read, Some(expected), "{order:?}, {stored:?}");
            }
        }
    }

    #[test]
    fn a_fine_tune_in_fractions_wins_over_whole_steps() {
        // As the D7200 writes both: `-24/4, 0/4` beside `-6, 0`.
        for order in ORDERS {
            let read = fine_tune_of_file(order, |inner, order, ifd| {
                sshorts(inner, order, ifd, FINE_TUNE_STEPS, &[-6, 0]);
                inner.srational(ifd, FINE_TUNE, &[(-24, 4), (0, 4)]);
            });
            assert_eq!(read, Some(shift((-24, 4), (0, 4))), "{order:?}");
        }
    }

    #[test]
    fn fractions_that_give_no_fine_tune_give_way_to_whole_steps() {
        let whole_steps = shift((1, 1), (-2, 1));
        // Each a 0x003F that gives none, as the closure adds it.
        let unusable: [(&str, Tags); 7] = [
            ("one value", |inner, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(-12, 4)]);
            }),
            ("three values", |inner, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(-12, 4), (8, 4), (0, 4)]);
            }),
            ("RATIONAL", |inner, ifd| {
                inner.rational(ifd, FINE_TUNE, &[(12, 4), (8, 4)]);
            }),
            ("SSHORT", |inner, ifd| {
                inner.raw(ifd, FINE_TUNE, SSHORT, 2, RawValue::Inline([0; 4]));
            }),
            ("a zero first denominator", |inner, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(-12, 0), (8, 4)]);
            }),
            ("a zero second denominator", |inner, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(-12, 4), (8, 0)]);
            }),
            ("a second numerator that cannot be negated", |inner, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(-12, 4), (i32::MIN, 4)]);
            }),
        ];
        for order in ORDERS {
            for (what, fractions) in unusable {
                let read = fine_tune_of_file(order, |inner, order, ifd| {
                    fractions(inner, ifd);
                    sshorts(inner, order, ifd, FINE_TUNE_STEPS, &[1, 2]);
                });
                assert_eq!(read, Some(whole_steps), "{order:?}, {what}");
                // And without whole steps, none at all.
                let alone = fine_tune_of_file(order, |inner, _, ifd| fractions(inner, ifd));
                assert_eq!(alone, None, "{order:?}, {what} alone");
            }
        }
    }

    #[test]
    fn a_fine_tune_in_fractions_past_the_block_gives_way_to_whole_steps() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                sshorts(inner, order, ifd, FINE_TUNE_STEPS, &[1, 2]);
                inner.srational(ifd, FINE_TUNE, &[(-12, 4), (8, 4)]);
            })
            .0;
            // The two fractions are the block's last 16 bytes.
            let fractions: Vec<u8> = [-12_i32, 4, 8, 4]
                .into_iter()
                .flat_map(|v| order.u32(v.cast_unsigned()))
                .collect();
            assert_eq!(block[block.len() - 16..], fractions, "{order:?}");

            // Declared one byte short: the fractions' last byte lies in
            // the file, after the block, so the entry is dropped.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            let read = metadata_of(file).camera_settings.white_balance.fine_tune;
            assert_eq!(read, Some(shift((1, 1), (-2, 1))), "{order:?}");

            // The control: declared whole, the fractions are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            let read = metadata_of(file).camera_settings.white_balance.fine_tune;
            assert_eq!(read, Some(shift((-12, 4), (-8, 4))), "{order:?}");
        }
    }

    #[test]
    fn a_fine_tune_of_one_whole_step_value_is_none() {
        // As the D1 family, the D70, the D100 and the D200 write it: no
        // file shows which axis the one value is on, or its sign.
        for order in ORDERS {
            for value in [0, -3, 2] {
                let read = fine_tune_of_file(order, |inner, order, ifd| {
                    sshorts(inner, order, ifd, FINE_TUNE_STEPS, &[value]);
                });
                assert_eq!(read, None, "{order:?}, {value}");
            }
        }
    }

    #[test]
    fn whole_steps_of_another_type_are_not_read() {
        for order in ORDERS {
            let shorts = fine_tune_of_file(order, |inner, _, ifd| {
                inner.short(ifd, FINE_TUNE_STEPS, &[1, 2]);
            });
            assert_eq!(shorts, None, "{order:?}, SHORT");
            let bytes = fine_tune_of_file(order, |inner, _, ifd| {
                inner.byte(ifd, FINE_TUNE_STEPS, &[1, 2]);
            });
            assert_eq!(bytes, None, "{order:?}, BYTE");
        }
    }

    #[test]
    fn a_fine_tune_of_another_count_is_refused_before_it_is_read() {
        // 0x003F of three SRATIONALs and 0x000B of three SSHORTs, each
        // stored outside its entry, alone in the block, whose last bytes
        // it is, placed from FAR on, with the inner header and the
        // MakerNote IFD before FAR: reading the values would fail, so the
        // count must be checked first, and either gives none.
        let order = Order::Little;
        let fractions = tags_block(order, |inner, ifd| {
            inner.srational(ifd, FINE_TUNE, &[(-12, 4), (8, 4), (0, 4)]);
        })
        .0;
        let (steps, entry) = tags_block(order, |inner, ifd| {
            inner.short(ifd, FINE_TUNE_STEPS, &[1, 2, 3]);
        });
        let steps = retyped(order, steps, entry, SSHORT, 3);
        for (what, len, block) in [("0x003F", 24, fractions), ("0x000B", 6, steps)] {
            let at = FAR - (whole(&block) - len);
            let bytes = nikon_file(order, at, &block, 0, |builder, exif| {
                entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, whole(&block));
            });
            let limits = Limits::default();
            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &limits).unwrap();
            let makernote = read(&mut src, &tree, &limits)
                .unwrap()
                .expect("a MakerNote");
            let standard = tiff_metadata::read(&mut src, &tree).unwrap();
            let read = metadata(&mut src, &makernote, standard, Picture::Raw).unwrap();
            assert_eq!(read.camera_settings.white_balance.fine_tune, None, "{what}");
        }
    }

    #[test]
    fn each_shutter_code_named_gives_its_shutter() {
        let named = [
            (0, Shutter::Mechanical),
            (16, Shutter::Electronic),
            (80, Shutter::Mechanical),
            (81, Shutter::ElectronicFrontCurtain),
        ];
        for order in ORDERS {
            for (code, shutter) in named {
                let read = settings_of(order, |inner, ifd| {
                    inner.short(ifd, SHUTTER_MODE, &[code]);
                });
                assert_eq!(read.shutter, Some(shutter), "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn a_shutter_code_not_named_is_none() {
        // 48 and 96, which no checked file holds; 64, a movie's; and codes
        // nothing names.
        for order in ORDERS {
            for code in [48, 64, 96, 1, 17, 82, u16::MAX] {
                let read = settings_of(order, |inner, ifd| {
                    inner.short(ifd, SHUTTER_MODE, &[code]);
                });
                assert_eq!(read.shutter, None, "{order:?}, {code}");
            }
        }
    }

    /// The image area of a raw file in byte order `order` whose MakerNote's
    /// CropHiSpeed is the Z 9's uncropped one with the crop setting `mode`.
    fn image_area_of(order: Order, mode: u16) -> Option<ImageArea> {
        settings_of(order, |inner, ifd| {
            inner.short(ifd, CROP_HI_SPEED, &[mode, 8280, 5520, 8280, 5520, 0, 0]);
        })
        .image_area
    }

    #[test]
    fn each_crop_setting_named_gives_its_image_area() {
        let named = [
            (11, ImageArea::FullFrame),
            (2, ImageArea::ApsC),
            (12, ImageArea::ApsC),
            (17, ImageArea::Square),
            (18, ImageArea::ApsCSquare),
            (6, ImageArea::Wide),
            (9, ImageArea::ApsCWide),
            (1, ImageArea::Crop1_3x),
            (10, ImageArea::Crop1_3x),
        ];
        for order in ORDERS {
            for (mode, area) in named {
                assert_eq!(image_area_of(order, mode), Some(area), "{order:?}, {mode}");
            }
        }
    }

    #[test]
    fn a_crop_setting_not_named_gives_no_image_area() {
        // "Off", which does not say which sensor the body has; 5:4, 3:2
        // and 2.7x, which no checked file holds; other movie crops; and
        // codes nothing names.
        for order in ORDERS {
            for mode in [0, 3, 4, 8, 13, 14, 15, 5, 7, 16, 19, u16::MAX] {
                assert_eq!(image_area_of(order, mode), None, "{order:?}, {mode}");
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_of_other_than_seven_shorts_gives_no_image_area() {
        let z9 = [11, 8280, 5520, 8280, 5520, 0, 0];
        for order in ORDERS {
            for values in [
                &z9[..6],
                &[11, 8280, 5520, 8280, 5520, 0, 0, 0][..],
                &z9[..1],
            ] {
                let read = settings_of(order, |inner, ifd| {
                    inner.short(ifd, CROP_HI_SPEED, values);
                });
                assert_eq!(read.image_area, None, "{order:?}, {values:?}");
            }
            let longs = settings_of(order, |inner, ifd| {
                inner.long(ifd, CROP_HI_SPEED, &z9.map(u32::from));
            });
            assert_eq!(longs.image_area, None, "{order:?}, LONG");
        }
    }

    #[test]
    fn a_crop_hi_speed_past_the_block_gives_no_image_area() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &[11, 8280, 5520, 8280, 5520, 0, 0]);
            })
            .0;
            // The seven values are the block's last 14 bytes.
            assert_eq!(block[block.len() - 14..][..2], order.u16(11), "{order:?}");
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).camera_settings.image_area, None);
            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            assert_eq!(
                metadata_of(file).camera_settings.image_area,
                Some(ImageArea::FullFrame),
                "{order:?}"
            );
        }
    }

    #[test]
    fn each_raw_size_code_named_gives_its_size_in_a_raw_file() {
        for order in ORDERS {
            for (code, size) in [(1, RawImageSize::Large), (3, RawImageSize::Small)] {
                let read = settings_of(order, |inner, ifd| {
                    inner.short(ifd, IMAGE_SIZE_RAW, &[code]);
                });
                assert_eq!(read.raw_image_size, Some(size), "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn a_raw_size_code_not_named_is_none() {
        // 2, the medium size, which no checked raw file holds; 0, which
        // the D7200 writes in JPEGs and no table names; and others.
        for order in ORDERS {
            for code in [0, 2, 4, u16::MAX] {
                let read = settings_of(order, |inner, ifd| {
                    inner.short(ifd, IMAGE_SIZE_RAW, &[code]);
                });
                assert_eq!(read.raw_image_size, None, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn a_jpeg_has_no_raw_image_size_and_the_other_settings() {
        // ImageSizeRAW follows a JPEG's own size, and a JPEG holds no raw
        // data. Its other settings are read as in a raw file.
        for order in ORDERS {
            for code in [1, 2, 3] {
                let read = jpeg_settings(
                    order,
                    |inner, ifd| {
                        inner
                            .short(ifd, IMAGE_SIZE_RAW, &[code])
                            .short(ifd, SHUTTER_MODE, &[16]);
                    },
                    |_, _| {},
                );
                assert_eq!(read.raw_image_size, None, "{order:?}, {code}");
                assert_eq!(read.shutter, Some(Shutter::Electronic), "{order:?}");
            }
        }
    }

    /// The focus mode of a raw file in byte order `order` whose
    /// MakerNote's FocusMode is `text`, which the builder ends with a NUL.
    fn focus_mode_of(order: Order, text: &str) -> Option<FocusMode> {
        settings_of(order, |inner, ifd| {
            inner.ascii(ifd, FOCUS_MODE, text);
        })
        .focus_mode
    }

    #[test]
    fn each_focus_mode_text_of_the_table_gives_its_mode() {
        // Padded with spaces to six characters, as the bodies write an
        // autofocus mode, and `MANUAL`, which fills the six.
        for order in ORDERS {
            for (text, mode) in [
                ("MANUAL", FocusMode::Manual),
                ("AF-S  ", FocusMode::Single),
                ("AF-C  ", FocusMode::Continuous),
                ("AF-A  ", FocusMode::Automatic),
                ("AF-F  ", FocusMode::FullTime),
            ] {
                assert_eq!(
                    focus_mode_of(order, text),
                    Some(mode),
                    "{order:?}, {text:?}"
                );
                // The same without its padding.
                let trimmed = text.trim_end();
                assert_eq!(
                    focus_mode_of(order, trimmed),
                    Some(mode),
                    "{order:?}, {trimmed:?}"
                );
            }
        }
    }

    #[test]
    fn a_focus_mode_text_not_in_the_table_is_none() {
        // Other letters, the right letters in lower case, a mode no file
        // holds, a text cut short, one with more after it, and spaces.
        for order in ORDERS {
            for text in [
                "manual", "af-s  ", "AF-D  ", "AF-", "AF-S1 ", "MANUAL1", "      ",
            ] {
                assert_eq!(focus_mode_of(order, text), None, "{order:?}, {text:?}");
            }
        }
    }

    #[test]
    fn a_focus_mode_of_another_type_is_not_read() {
        for order in ORDERS {
            for typ in [UNDEFINED, BYTE] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.ascii(ifd, FOCUS_MODE, "AF-C  ");
                });
                let block = retyped(order, block, entry, typ, 7);
                let read = exif_metadata(order, &block, |_, _| {}).camera_settings;
                assert_eq!(read.focus_mode, None, "{order:?}, type {typ}");
            }
        }
    }

    #[test]
    fn a_jpeg_gives_the_focus_mode_as_a_raw_file_does() {
        for order in ORDERS {
            let read = jpeg_settings(
                order,
                |inner, ifd| {
                    inner.ascii(ifd, FOCUS_MODE, "AF-A  ");
                },
                |_, _| {},
            );
            assert_eq!(read.focus_mode, Some(FocusMode::Automatic), "{order:?}");
        }
    }

    #[test]
    fn without_af_info2_the_area_mode_is_afinfos_first_byte() {
        // Every code of the list: 0 single area, 1 dynamic area, 2 the
        // closest subject, which one D70 JPEG writes, 3 group dynamic, and
        // 4 and 5 the wide focus frame's single and dynamic area. Codes
        // past the list give none. The other three bytes, the point and
        // the points in focus, play no part.
        for order in ORDERS {
            for (code, mode) in [
                (0, Some(FocusAreaMode::Single)),
                (1, Some(FocusAreaMode::Dynamic)),
                (2, Some(FocusAreaMode::Auto)),
                (3, Some(FocusAreaMode::Zone)),
                (4, Some(FocusAreaMode::Single)),
                (5, Some(FocusAreaMode::Dynamic)),
                (6, None),
                (128, None),
                (255, None),
            ] {
                let read = settings_of(order, |inner, ifd| {
                    inner.undefined(ifd, AF_INFO, &[code, 3, 0xFF, 0x07]);
                });
                assert_eq!(read.focus_area_mode, mode, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn afinfo_is_not_read_beside_af_info2_or_in_another_shape() {
        for order in ORDERS {
            // Beside an AFInfo2 of a version not read, AFInfo's 1 gives
            // nothing: a body with AFInfo2 never falls back on AFInfo.
            let mut af_info2 = b"0403".to_vec();
            af_info2.resize(80, 0);
            let read = settings_of(order, |inner, ifd| {
                inner
                    .undefined(ifd, AF_INFO, &[1, 0, 0, 0])
                    .undefined(ifd, AF_INFO2, &af_info2);
            });
            assert_eq!(read.focus_area_mode, None, "{order:?}");
            // As BYTEs, and of three or five bytes.
            for (typ, bytes) in [
                (BYTE, &[1_u8, 0, 0, 0][..]),
                (UNDEFINED, &[1, 0, 0][..]),
                (UNDEFINED, &[1, 0, 0, 0, 0][..]),
            ] {
                let read = settings_of(order, |inner, ifd| {
                    match typ {
                        BYTE => inner.byte(ifd, AF_INFO, bytes),
                        _ => inner.undefined(ifd, AF_INFO, bytes),
                    };
                });
                let label = format!("{order:?}, type {typ}, {} bytes", bytes.len());
                assert_eq!(read.focus_area_mode, None, "{label}");
            }
        }
    }

    /// The AF fine-tune of a raw file in byte order `order` whose
    /// MakerNote's AFTune is `bytes`, as UNDEFINED.
    fn af_fine_tune_of(order: Order, bytes: &[u8]) -> Option<AfFineTune> {
        settings_of(order, |inner, ifd| {
            inner.undefined(ifd, AF_TUNE, bytes);
        })
        .af_fine_tune
    }

    #[test]
    fn af_tune_gives_whether_fine_tune_is_on_and_its_value() {
        // Off, with a value it does not give; on at 1 and at 2, each with
        // its value, negative ones as signed bytes, and at 0; the saved
        // lens's index plays no part, nor does the long end's byte under
        // 0, 1 and 2, where the documentation calls it not valid.
        for order in ORDERS {
            for (bytes, expected) in [
                ([0, 0xFF, 0, 0], Some(AfFineTune::off())),
                ([0, 3, 0xF6, 0], Some(AfFineTune::off())),
                ([1, 0xFF, 0, 0], Some(AfFineTune::on().with_value(0))),
                ([2, 0, 0xF6, 0], Some(AfFineTune::on().with_value(-10))),
                ([2, 8, 0xEC, 0], Some(AfFineTune::on().with_value(-20))),
                ([1, 2, 20, 0], Some(AfFineTune::on().with_value(20))),
                ([2, 2, 0x80, 0x7F], Some(AfFineTune::on().with_value(-128))),
                ([1, 0, 5, 7], Some(AfFineTune::on().with_value(5))),
                ([0, 0, 5, 7], Some(AfFineTune::off())),
                // 3, on for a zoom: the value at the short end, then the
                // long end's, each signed.
                (
                    [3, 0, 5, 7],
                    Some(AfFineTune::on().with_value(5).with_tele_value(7)),
                ),
                (
                    [3, 1, 0xFE, 0xF6],
                    Some(AfFineTune::on().with_value(-2).with_tele_value(-10)),
                ),
                (
                    [3, 0xFF, 0, 0],
                    Some(AfFineTune::on().with_value(0).with_tele_value(0)),
                ),
            ] {
                assert_eq!(
                    af_fine_tune_of(order, &bytes),
                    expected,
                    "{order:?}, {bytes:?}"
                );
            }
        }
    }

    #[test]
    fn af_tune_of_a_code_the_list_lacks_or_another_shape_is_none() {
        for order in ORDERS {
            // Codes past the documented list.
            for code in [4, 128, 255] {
                let bytes = [code, 0, 5, 7];
                assert_eq!(af_fine_tune_of(order, &bytes), None, "{order:?}, {code}");
            }
            // Three or five bytes, and a zoom's three.
            assert_eq!(af_fine_tune_of(order, &[1, 0, 5]), None, "{order:?}");
            assert_eq!(af_fine_tune_of(order, &[3, 0, 5]), None, "{order:?}");
            assert_eq!(af_fine_tune_of(order, &[1, 0, 5, 0, 0]), None, "{order:?}");
            // Four BYTEs.
            let read = settings_of(order, |inner, ifd| {
                inner.byte(ifd, AF_TUNE, &[1, 0, 5, 0]);
            });
            assert_eq!(read.af_fine_tune, None, "{order:?}");
        }
    }

    /// Whether daylight saving time was on, in the settings of a raw file
    /// in byte order `order` whose MakerNote's WorldTime is `bytes`, of
    /// type UNDEFINED.
    fn daylight_saving_of(order: Order, bytes: &[u8]) -> Option<bool> {
        settings_of(order, |inner, ifd| {
            inner.undefined(ifd, WORLD_TIME, bytes);
        })
        .daylight_saving_time
    }

    #[test]
    fn world_time_says_whether_daylight_saving_time_was_on() {
        // The zones of public samples 928 and 6887 (60 and -300 minutes),
        // and one past the offset's bound, which the setting does not
        // depend on; the date format plays no part either.
        for order in ORDERS {
            for zone in [60_i16, -300, 15_360] {
                let [zone0, zone1] = order.u16(zone.cast_unsigned());
                for (byte, expected) in [(0, Some(false)), (1, Some(true))] {
                    for format in [0, 2] {
                        assert_eq!(
                            daylight_saving_of(order, &[zone0, zone1, byte, format]),
                            expected,
                            "{order:?}, {zone}, {byte}, {format}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_daylight_saving_byte_of_another_value_or_another_shape_is_none() {
        for order in ORDERS {
            for byte in [2, 128, 255] {
                assert_eq!(
                    daylight_saving_of(order, &[60, 0, byte, 0]),
                    None,
                    "{order:?}, {byte}"
                );
            }
            // Three bytes, and five; four BYTEs.
            assert_eq!(daylight_saving_of(order, &[60, 0, 1]), None, "{order:?}");
            assert_eq!(
                daylight_saving_of(order, &[60, 0, 1, 0, 0]),
                None,
                "{order:?}"
            );
            let read = settings_of(order, |inner, ifd| {
                inner.byte(ifd, WORLD_TIME, &[60, 0, 1, 0]);
            });
            assert_eq!(read.daylight_saving_time, None, "{order:?}");
        }
    }

    #[test]
    fn without_an_exif_color_space_the_makernotes_is_read() {
        // As in every Nikon raw file checked, and in a JPEG without one.
        let named = [
            (1, Some(ColorSpace::Srgb)),
            (2, Some(ColorSpace::AdobeRgb)),
            // BT.2100, which no variant names, and codes nothing names.
            (4, None),
            (0, None),
            (3, None),
        ];
        for order in ORDERS {
            for (code, expected) in named {
                let tags = |inner: &mut TiffBuilder, ifd| {
                    inner.short(ifd, COLOR_SPACE, &[code]);
                };
                let raw = settings_of(order, tags);
                assert_eq!(raw.color_space, expected, "{order:?}, {code}");
                let jpeg = jpeg_settings(order, tags, |_, _| {});
                assert_eq!(jpeg.color_space, expected, "{order:?}, {code}, JPEG");
            }
        }
    }

    #[test]
    fn an_exif_color_space_keeps_the_makernotes_out() {
        // Each Exif ColorSpace, as the closure adds it, and the color
        // space it gives: sRGB; "uncalibrated" without the index `R03`,
        // as a program that rewrote a JPEG may leave it; and one of
        // another type, which gives none. The MakerNote's Adobe RGB is
        // never read beside it.
        let exif: [(Tags, Option<ColorSpace>); 4] = [
            (
                |builder, exif| {
                    builder.short(exif, EXIF_COLOR_SPACE, &[1]);
                },
                Some(ColorSpace::Srgb),
            ),
            (
                |builder, exif| {
                    builder.short(exif, EXIF_COLOR_SPACE, &[65535]);
                },
                None,
            ),
            (
                |builder, exif| {
                    let interop = builder.ifd();
                    builder
                        .short(exif, EXIF_COLOR_SPACE, &[65535])
                        .interop(exif, interop)
                        .ascii(interop, INTEROP_INDEX, "R98");
                },
                None,
            ),
            (
                |builder, exif| {
                    builder.long(exif, EXIF_COLOR_SPACE, &[1]);
                },
                None,
            ),
        ];
        let tags = |inner: &mut TiffBuilder, ifd| {
            inner.short(ifd, COLOR_SPACE, &[2]);
        };
        for order in ORDERS {
            for (place, (exif, expected)) in exif.into_iter().enumerate() {
                let raw = settings_with_exif(order, tags, exif);
                assert_eq!(raw.color_space, expected, "{order:?}, {place}");
                let jpeg = jpeg_settings(order, tags, exif);
                assert_eq!(jpeg.color_space, expected, "{order:?}, {place}, JPEG");
            }
        }
    }

    #[test]
    fn a_makernote_without_settings_tags_keeps_the_exif_settings() {
        // Exif's aperture priority and manual white balance for shade, and
        // a MakerNote that records no setting: nothing is made up, and
        // nothing the Exif tags gave is lost.
        let exif = |builder: &mut TiffBuilder, exif: IfdId| {
            exif_shade(builder, exif);
            builder.short(exif, EXPOSURE_PROGRAM, &[3]);
        };
        let expected = CameraSettings::default()
            .with_white_balance(WhiteBalance::default().with_mode(WhiteBalanceMode::Shade))
            .with_exposure_program(ExposureProgram::AperturePriority);
        for order in ORDERS {
            assert_eq!(
                settings_with_exif(order, |_, _| {}, exif),
                expected,
                "{order:?}"
            );
            assert_eq!(
                jpeg_settings(order, |_, _| {}, exif),
                expected,
                "{order:?}, JPEG"
            );
        }
    }

    #[test]
    fn the_settings_in_the_other_byte_order_from_the_files_are_read() {
        // The Z 9's settings, in a MakerNote whose byte order is not the
        // file's.
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner
                    .ascii(ifd, WHITE_BALANCE, &padded("AUTO0"))
                    .short(ifd, CROP_HI_SPEED, &[11, 8280, 5520, 8280, 5520, 0, 0])
                    .short(ifd, COLOR_SPACE, &[1])
                    .short(ifd, SHUTTER_MODE, &[16])
                    .short(ifd, IMAGE_SIZE_RAW, &[1])
                    .srational(ifd, FINE_TUNE, &[(-12, 4), (8, 4)])
                    .short(ifd, COLOR_TEMPERATURE, &[4630]);
                sshorts(inner, inner_order, ifd, FINE_TUNE_STEPS, &[-3, 2]);
            })
            .0;
            let read = exif_metadata(file, &block, |_, _| {}).camera_settings;
            let expected = CameraSettings::default()
                .with_white_balance(
                    WhiteBalance::default()
                        .with_mode(WhiteBalanceMode::Auto)
                        .with_color_temperature(4630)
                        .with_fine_tune(shift((-12, 4), (-8, 4))),
                )
                .with_shutter(Shutter::Electronic)
                .with_image_area(ImageArea::FullFrame)
                .with_raw_image_size(RawImageSize::Large)
                .with_color_space(ColorSpace::Srgb);
            assert_eq!(read, expected, "{file:?}");
        }
    }

    #[test]
    fn a_reader_failure_while_reading_a_setting_is_an_error() {
        // Each setting whose value is stored outside its entry, alone in
        // the block, whose last bytes it is, placed from FAR on, with the
        // inner header and the MakerNote IFD before FAR.
        let order = Order::Little;
        let settings: [(u64, Tags); 3] = [
            (13, |inner, ifd| {
                inner.ascii(ifd, WHITE_BALANCE, &padded("SUNNY"));
            }),
            (16, |inner, ifd| {
                inner.srational(ifd, FINE_TUNE, &[(-12, 4), (8, 4)]);
            }),
            (14, |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &[11, 8280, 5520, 8280, 5520, 0, 0]);
            }),
        ];
        for (len, tags) in settings {
            let block = tags_block(order, tags).0;
            let at = FAR - (whole(&block) - len);
            let bytes = nikon_file(order, at, &block, 0, |builder, exif| {
                entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, whole(&block));
            });
            // The file is sound, for a reader that does not fail.
            assert_ne!(
                metadata_of(bytes.clone()).camera_settings,
                CameraSettings::default(),
                "{len}"
            );

            let limits = Limits::default();
            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &limits).unwrap();
            let makernote = read(&mut src, &tree, &limits)
                .unwrap()
                .expect("a MakerNote");
            let standard = tiff_metadata::read(&mut src, &tree).unwrap();
            let err =
                metadata(&mut src, &makernote, standard, Picture::Raw).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{len}: {err:?}");
        }
    }
}
