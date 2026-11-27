// SPDX-License-Identifier: MIT OR Apache-2.0

//! The camera settings in the Nikon MakerNote: the white balance with its
//! preset slot, the shutter used, the image area, the raw size and
//! compression, the JPEG quality and compression, the color space, the
//! focus mode, the AF-area mode, AF fine-tune, the stabilization, auto ISO,
//! the exposure fine-tuning, the release mode and bracketing, daylight
//! saving time, the flash's source, control, sync and compensation, the
//! Picture Control, Active D-Lighting, HDR, and the high ISO and long
//! exposure noise reductions.
//!
//! The MakerNote records how the camera was set for the picture beyond
//! what the Exif tags record (ADR011). [`camera_settings`] reads each
//! setting from one tag, by its own rule:
//!
//! - the white balance mode from WhiteBalance's text
//!   ([`white_balance_of_text`]), finer than Exif's, and the preset slot
//!   from the same text ([`preset_slot_of_text`]);
//! - the color temperature from ColorTemperatureAuto, else from a
//!   WhiteBalance text that records a chosen one ([`color_temperature`]);
//! - the fine-tuning from 0x003F, else from 0x000B ([`fine_tune`]);
//! - the shutter used from ShutterMode ([`shutter`]);
//! - the image area from CropHiSpeed's crop setting ([`image_area`]), or,
//!   where it is "off" or missing, from the body's sensor format in
//!   raw-files' table;
//! - the raw size from ImageSizeRAW ([`raw_image_size`]), or, where a raw
//!   file holds none, large for a body raw-files' table says offers no raw
//!   sizes;
//! - the raw compression from NEFCompression, else from tag 0x0051
//!   ([`nef_compression_code`], [`nef_compression`]);
//! - the JPEG quality from Quality's text ([`jpeg_quality_of_text`]), and
//!   the JPEG compression from JPGCompression ([`jpeg_compression`]);
//! - the color space from ColorSpace ([`color_space`]);
//! - the focus mode from FocusMode's text ([`focus_mode_of_text`]);
//! - the AF-area mode from AFInfo2 ([`area_mode_of`]), else from AFInfo
//!   ([`af_info_area_mode`]);
//! - AF fine-tune from AFTune ([`af_fine_tune`]);
//! - the stabilization from VRInfo ([`stabilization`]);
//! - auto ISO from ShootingMode's bit 5 ([`auto_iso`]), else from
//!   ISOSelection's text ([`auto_iso_of_text`]);
//! - the exposure fine-tuning from ExposureTuning
//!   ([`exposure_fine_tune`]);
//! - the release mode and bracketing from ShootingMode's bits
//!   ([`release_mode`], [`bracketing`]);
//! - daylight saving time from WorldTime ([`daylight_saving_time`]);
//! - the flash's source and control from FlashType's text
//!   ([`flash_type_of_text`]), its sync from FlashSetting's
//!   ([`flash_sync_of_text`]), and its compensation from
//!   FlashExposureComp ([`flash_compensation`]);
//! - the picture style from PictureControlData, else from the Coolpix
//!   bodies' copy of it ([`picture_control_record`], [`picture_control`]);
//! - the dynamic range optimization from ActiveD-Lighting
//!   ([`active_d_lighting`]);
//! - the high dynamic range setting from HDRInfo ([`hdr`]);
//! - the high ISO noise reduction from HighISONoiseReduction
//!   ([`high_iso_noise_reduction`]);
//! - the long exposure noise reduction from NoiseReduction's text
//!   ([`long_exposure_noise_reduction_of_text`]).
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
use super::picture_control::picture_control;
use super::rules;
use crate::camera::BodyFacts;
use crate::camera_settings::{
    ActiveDLighting, AfFineTune, Bracketing, CameraSettings, ColorSpace, DynamicRangeOptimization,
    Flash, FlashControl, FlashSource, FlashSync, FocusAreaMode, FocusMode, Hdr,
    HdrExposureDifferential, HdrSmoothing, HdrStrength, HighDynamicRange, HighIsoNoiseReduction,
    ImageArea, JpegCompression, JpegQuality, PictureStyle, RawCompression, RawImageSize,
    ReleaseMode, Shutter, Stabilization, StabilizationMode, WhiteBalanceFineTune, WhiteBalanceMode,
};
use crate::container::tiff::Ifd;
use crate::error::Result;
use crate::formats::makernote::{Picture, checked_color_temperature};
use crate::formats::tiff_metadata;
use crate::io::Source;
use crate::rational::{Rational, SignedRational};

/// What the standard tags gave that the camera settings build on.
pub(super) struct FromStandardTags<'a> {
    /// The camera settings the Exif tags record.
    pub(super) settings: CameraSettings,
    /// The Model, read by the text rule, if any.
    pub(super) model: Option<&'a str>,
    /// The published facts of the body the Make and Model name, from
    /// raw-files' table ([`Camera::body_facts`]), if it holds any.
    ///
    /// [`Camera::body_facts`]: crate::camera::Camera::body_facts
    pub(super) body: Option<BodyFacts>,
    /// Whether the file holds raw data: the standard tags gave a raw size.
    /// A TIFF image a Nikon body or program wrote is read as a raw file
    /// too, and holds none.
    pub(super) holds_raw_data: bool,
    /// Whether the Exif IFD keeps a ColorSpace entry
    /// ([`Standard`](tiff_metadata::Standard)).
    pub(super) exif_records_color_space: bool,
    /// The Compression of the raw data, as the IFD that gives the raw size
    /// records it ([`RawStorage`](tiff_metadata::RawStorage)).
    pub(super) raw_compression: Option<u32>,
}

/// The Compression of raw data in Nikon's compressed form
/// (`docs/formats/nef/raw-data.md`), which NEFCompression refines.
const NIKON_COMPRESSED: u32 = 34713;

/// How the camera was set for the picture: `standard.settings`, what the
/// standard tags gave, with what `makernote` records. Each value is read
/// by its own rule, so that a damaged one costs only itself:
///
/// - **The white balance mode** is the one WhiteBalance's text names
///   ([`white_balance_of_text`]), in place of the Exif tags' mode, which
///   records only auto or the kind of light. The text is read once, by the
///   text rule ([`tiff_metadata::text`]). When it names no mode, the Exif
///   tags' mode stays. When it names a mode other than fluorescent, the
///   lamp the Exif tags gave is dropped: a lamp belongs to a fluorescent
///   white balance alone. No measured file shows them disagreeing.
/// - **The preset slot** is the number a WhiteBalance text that names a
///   preset records ([`preset_slot_of_text`]).
/// - **The color temperature** is ColorTemperatureAuto's, or a chosen one
///   that the WhiteBalance text records ([`color_temperature`]).
/// - **The fine-tuning** is read from 0x003F, else from 0x000B
///   ([`fine_tune`]).
/// - **The shutter used** is ShutterMode's code ([`shutter`]).
/// - **The image area** is the crop setting of `crop_hi_speed`,
///   CropHiSpeed as [`crop_hi_speed`] read it ([`image_area`]). Where the
///   setting is 0, "off", or the MakerNote holds no CropHiSpeed at all, it
///   is the sensor format of `standard.body`, the body's facts from
///   raw-files' table, when the table holds them (ADR011). A CropHiSpeed
///   that breaks its rule gives none, since its setting may have named a
///   crop, and so does one the IFD skipped as damaged ([`holds_no`]); so
///   does a setting that names no area for another reason, such as a 3:2
///   crop or a movie crop, and a setting of "off" whose cropped size is
///   smaller than its original, which no measured file holds.
/// - **The raw image size** is ImageSizeRAW's code ([`raw_image_size`]),
///   for a raw file's picture alone. Where the MakerNote holds no
///   ImageSizeRAW, it is large when `standard.body` says the body offers
///   no raw sizes (ADR011), in a file that holds raw data
///   (`standard.holds_raw_data`): a TIFF image holds none, whatever its
///   body. An ImageSizeRAW that breaks its rule, that the IFD skipped as
///   damaged ([`holds_no`]), or whose code [`raw_image_size`] does not
///   read, such as 2, the medium size, gives none. In a JPEG, whose
///   `picture` is [`Picture::Jpeg`], the tag follows the JPEG's own size,
///   and a JPEG holds no raw data (ADR014).
/// - **The raw compression**, for a raw file's picture whose
///   `standard.raw_compression` is Nikon's compressed form, 34713, is the
///   one NEFCompression's code names, read from NEFCompression or, on the
///   bodies without it, from tag 0x0051 ([`nef_compression_code`],
///   [`nef_compression`]). Under any other Compression it stays as
///   `settings` has it: uncompressed for 1, else `None`.
/// - **The JPEG quality** is the one Quality's text names
///   ([`jpeg_quality_of_text`]), read by the text rule, for a JPEG's
///   picture alone: a raw file's Quality says `RAW`, whatever JPEG the
///   camera wrote beside it. **The JPEG compression** is JPGCompression's
///   code ([`jpeg_compression`]), for a JPEG's picture alone too.
/// - **The color space** is ColorSpace's code ([`color_space`]), only when
///   `standard.exif_records_color_space` is `false`. Where the Exif IFD
///   keeps a ColorSpace, it alone decides
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
/// - **The stabilization** is VRInfo's ([`stabilization`]).
/// - **Auto ISO** is ShootingMode's bit 5 ([`auto_iso`]), on a body other
///   than the D70, which `standard.model`, the Model the standard tags
///   read, names.
///   Only when the file has no ShootingMode that can be read is it the
///   text of ISOSelection ([`auto_iso_of_text`]), as the Coolpix bodies
///   record it.
/// - **The exposure fine-tuning** is ExposureTuning's
///   ([`exposure_fine_tune`]).
/// - **The release mode** is the one ShootingMode's bits name
///   ([`release_mode`]). **The bracketing** they name ([`bracketing`])
///   takes the place of the one the Exif tags gave, exposure bracketing
///   from ExposureMode 2, when it names one; when it names none, as for
///   two bracketing bits set together, the Exif tags' stays.
/// - **Daylight saving time** is WorldTime's third byte
///   ([`daylight_saving_time`]), read from the tag that gives the capture
///   time its offset ([`world_time`](super::metadata::world_time)).
/// - **The flash** the Exif tags gave gains the parts the MakerNote
///   records ([`flash`]). A file whose Exif tags give no flash gets none
///   from the MakerNote either: its parts have nothing to belong to.
/// - **The picture style** is the Picture Control that PictureControlData
///   records, or, on a body without it, the Coolpix bodies' copy
///   ([`picture_control_record`]), read at its version's layout
///   ([`picture_control`]), in a raw file and a JPEG alike.
/// - **The dynamic range optimization** is the Active D-Lighting step
///   that ActiveD-Lighting's code names ([`active_d_lighting`]).
/// - **The high dynamic range setting** is the HDR that HDRInfo records
///   ([`hdr`]), in a raw file and a JPEG alike.
/// - **The high ISO noise reduction** is the step HighISONoiseReduction's
///   code names ([`high_iso_noise_reduction`]), in a raw file and a JPEG
///   alike.
/// - **The long exposure noise reduction** is on where NoiseReduction's
///   text, read by the text rule, says it ran
///   ([`long_exposure_noise_reduction_of_text`]), and `None` otherwise.
///
/// Nikon records no multi-frame noise reduction, so it stays `None`.
///
/// Nikon records no electronic front curtain setting, so it stays as
/// `settings` has it, `None`. The exposure program and the metering mode
/// are the Exif tags' alone.
///
/// # Errors
///
/// `Io` when the reader fails.
///
/// [`crop_hi_speed`]: super::crop::crop_hi_speed
pub(super) fn camera_settings<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    standard: FromStandardTags<'_>,
    crop_hi_speed: Option<CropHiSpeed>,
    af_info2: Option<&[u8]>,
    picture: Picture,
) -> Result<CameraSettings> {
    let FromStandardTags {
        mut settings,
        model,
        body,
        holds_raw_data,
        exif_records_color_space,
        raw_compression,
    } = standard;
    let Some(ifd) = makernote.tagged() else {
        return Ok(settings);
    };
    // Read once, for the white balance mode, the preset slot and the color
    // temperature.
    let white_balance = tiff_metadata::text(src, ifd, rules::WHITE_BALANCE)?;
    let white_balance = white_balance.as_deref();
    if let Some(mode) = white_balance.and_then(white_balance_of_text) {
        settings.white_balance.mode = Some(mode);
        // A lamp belongs to a fluorescent white balance alone.
        if mode != WhiteBalanceMode::Fluorescent {
            settings.white_balance.fluorescent_lamp = None;
        }
    }
    settings.white_balance.preset_slot = white_balance.and_then(preset_slot_of_text);
    settings.white_balance.color_temperature = color_temperature(makernote, white_balance);
    settings.white_balance.fine_tune = fine_tune(src, makernote)?;
    settings.shutter = ifd
        .held(rules::SHUTTER_MODE)
        .and_then(|[code]| shutter(code));
    let sensor = body.map(|facts| facts.sensor);
    settings.image_area = match crop_hi_speed {
        // "Off" says nothing of the sensor; the body's facts do, for a
        // picture the crop kept whole.
        Some(crop @ CropHiSpeed { mode: 0, .. }) if crop.is_uncropped() => sensor,
        Some(CropHiSpeed { mode: 0, .. }) => None,
        Some(crop) => image_area(crop.mode),
        None if holds_no(ifd, super::CROP_HI_SPEED) => sensor,
        None => None,
    };
    settings.raw_image_size = match picture {
        // A body that offers no raw sizes writes every raw file at its
        // large one.
        Picture::Raw if holds_no(ifd, super::IMAGE_SIZE_RAW) => body
            .filter(|facts| holds_raw_data && !facts.offers_raw_sizes)
            .map(|_| RawImageSize::Large),
        Picture::Raw => ifd
            .held(rules::IMAGE_SIZE_RAW)
            .and_then(|[code]| raw_image_size(code)),
        Picture::Jpeg => None,
    };
    if picture == Picture::Raw && raw_compression == Some(NIKON_COMPRESSED) {
        settings.raw_compression = nef_compression_code(src, ifd)?.and_then(nef_compression);
    }
    settings.jpeg_quality = match picture {
        Picture::Jpeg => tiff_metadata::text(src, ifd, rules::QUALITY)?
            .and_then(|text| jpeg_quality_of_text(&text)),
        Picture::Raw => None,
    };
    settings.jpeg_compression = match picture {
        Picture::Jpeg => ifd
            .held(rules::JPG_COMPRESSION)
            .and_then(|[code]| jpeg_compression(code)),
        Picture::Raw => None,
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
    settings.stabilization = ifd
        .get(src, rules::VR_INFO)?
        .as_deref()
        .and_then(stabilization);
    settings.exposure_fine_tune = ifd
        .held(rules::EXPOSURE_TUNING)
        .and_then(exposure_fine_tune);
    if let Some([bits]) = ifd.held(rules::SHOOTING_MODE) {
        settings.auto_iso = auto_iso(bits, model);
        settings.release_mode = release_mode(bits);
        if let Some(bracketing) = bracketing(bits) {
            settings.bracketing = Some(bracketing);
        }
    } else {
        settings.auto_iso = tiff_metadata::text(src, ifd, rules::ISO_SELECTION)?
            .and_then(|text| auto_iso_of_text(&text));
    }
    settings.daylight_saving_time = ifd
        .held(rules::WORLD_TIME)
        .and_then(|[_, _, daylight_saving, _]| daylight_saving_time(daylight_saving));
    if let Some(exif_flash) = settings.flash {
        settings.flash = Some(flash(src, ifd, exif_flash)?);
    }
    settings.picture_style = picture_control_record(src, ifd)?
        .as_deref()
        .and_then(picture_control)
        .map(PictureStyle::PictureControl);
    settings.dynamic_range_optimization = ifd
        .held(rules::ACTIVE_D_LIGHTING)
        .and_then(|[code]| active_d_lighting(code))
        .map(DynamicRangeOptimization::ActiveDLighting);
    settings.high_dynamic_range = ifd
        .get(src, rules::HDR_INFO)?
        .as_deref()
        .and_then(hdr)
        .map(HighDynamicRange::Hdr);
    settings.high_iso_noise_reduction = ifd
        .held(rules::HIGH_ISO_NOISE_REDUCTION)
        .and_then(|[code]| high_iso_noise_reduction(code));
    settings.long_exposure_noise_reduction = tiff_metadata::text(src, ifd, rules::NOISE_REDUCTION)?
        .and_then(|text| long_exposure_noise_reduction_of_text(&text));
    Ok(settings)
}

/// The bytes of the Picture Control record that the MakerNote's IFD `ifd`
/// holds: PictureControlData (0x0023), else the copy the Coolpix bodies
/// write under 0x00BD, read only when the IFD keeps no 0x0023. No
/// measured file holds both. A PictureControlData that breaks its rule, a
/// type other than UNDEFINED, gives none, and does not give way to the
/// copy. An entry the IFD skipped as damaged does not stop the copy from
/// being read, unlike the tags [`holds_no`] guards: the two hold the same
/// record, so the copy can stand for a 0x0023 that was skipped.
///
/// # Errors
///
/// `Io` when the reader fails.
fn picture_control_record<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
) -> Result<Option<Vec<u8>>> {
    if ifd.entry(super::PICTURE_CONTROL).is_none() {
        ifd.get(src, rules::COOLPIX_PICTURE_CONTROL)
    } else {
        ifd.get(src, rules::PICTURE_CONTROL)
    }
}

/// The Active D-Lighting step an ActiveD-Lighting `code` names, in the
/// list ExifTool's published documentation gives, read whole, since files
/// hold part of it (NEF notes, section 8.11):
///
/// - 0 [`Off`](ActiveDLighting::Off), 1 [`Low`](ActiveDLighting::Low), 3
///   [`Normal`](ActiveDLighting::Normal), 5 [`High`](ActiveDLighting::High)
///   and 65535 [`Auto`](ActiveDLighting::Auto), each on files;
/// - 7 [`ExtraHigh`](ActiveDLighting::ExtraHigh), and 8 to 11
///   [`ExtraHigh1`](ActiveDLighting::ExtraHigh1) to
///   [`ExtraHigh4`](ActiveDLighting::ExtraHigh4), documented only.
///
/// `None` for every other code, such as 2, 4 or 6, which the list lacks.
fn active_d_lighting(code: u32) -> Option<ActiveDLighting> {
    Some(match code {
        0 => ActiveDLighting::Off,
        1 => ActiveDLighting::Low,
        3 => ActiveDLighting::Normal,
        5 => ActiveDLighting::High,
        7 => ActiveDLighting::ExtraHigh,
        8 => ActiveDLighting::ExtraHigh1,
        9 => ActiveDLighting::ExtraHigh2,
        10 => ActiveDLighting::ExtraHigh3,
        11 => ActiveDLighting::ExtraHigh4,
        65535 => ActiveDLighting::Auto,
        _ => return None,
    })
}

/// The HDR that HDRInfo's `record` holds, at the versions whose layout
/// ExifTool's published documentation gives and measured files hold,
/// each of its length: 0100, on the D600 and a Nikon 1 J2, and 0101, on
/// Coolpix bodies, of 8 bytes; 0200, on the D7100 and later bodies, of 6
/// bytes (NEF notes, section 8.11).
///
/// Byte 4 is 0 off and 1 on. Under on, the parts are read by the lists
/// the documentation gives, each read whole, since files hold part of
/// each: at 0200, byte 5 the strength ([`hdr_strength`]); at 0100 and
/// 0101, byte 5 the exposure differential
/// ([`hdr_exposure_differential`]) and byte 6 the smoothing
/// ([`hdr_smoothing`]). A code a list lacks gives no part. Under off, the
/// record gives no parts: the D600's files hold 0 in both bytes there,
/// which would read as a smoothing of off, a step its menu does not
/// offer.
///
/// `None` for any other version, a record of another length than its
/// version's, and any other byte 4, such as 48, which the documentation
/// calls auto: it says neither on nor off, and no file holds it. Byte 7 of
/// the 8-byte versions, which the documentation calls a second level, is
/// not read.
fn hdr(record: &[u8]) -> Option<Hdr> {
    let len = match record.get(..4)? {
        b"0100" | b"0101" => 8,
        b"0200" => 6,
        _ => return None,
    };
    if record.len() != len {
        return None;
    }
    let &[_, _, _, _, on, fifth, ref rest @ ..] = record else {
        return None;
    };
    match on {
        0 => Some(Hdr::off()),
        1 => {
            let mut hdr = Hdr::on();
            if let [sixth, _] = *rest {
                if let Some(differential) = hdr_exposure_differential(fifth) {
                    hdr = hdr.with_exposure_differential(differential);
                }
                if let Some(smoothing) = hdr_smoothing(sixth) {
                    hdr = hdr.with_smoothing(smoothing);
                }
            } else if let Some(strength) = hdr_strength(fifth) {
                hdr = hdr.with_strength(strength);
            }
            Some(hdr)
        }
        _ => None,
    }
}

/// The HDR strength that byte 5 of HDRInfo's version 0200, `code`, names:
/// 1 [`Normal`](HdrStrength::Normal), 2 [`Low`](HdrStrength::Low), 3
/// [`High`](HdrStrength::High), 4, the documentation's "High+", the
/// D7100's extra high, [`ExtraHigh`](HdrStrength::ExtraHigh), and 5
/// [`Auto`](HdrStrength::Auto), which my D7100's files hold. `None` for
/// 0, which the documentation calls not available, and every other code.
fn hdr_strength(code: u8) -> Option<HdrStrength> {
    Some(match code {
        1 => HdrStrength::Normal,
        2 => HdrStrength::Low,
        3 => HdrStrength::High,
        4 => HdrStrength::ExtraHigh,
        5 => HdrStrength::Auto,
        _ => return None,
    })
}

/// The exposure differential that byte 5 of HDRInfo's versions 0100 and
/// 0101, `code`, names: 0 [`Auto`](HdrExposureDifferential::Auto), and 1
/// to 3 that many EV, [`Ev`](HdrExposureDifferential::Ev) of `1/1` to
/// `3/1`. `None` for every other code.
fn hdr_exposure_differential(code: u8) -> Option<HdrExposureDifferential> {
    match code {
        0 => Some(HdrExposureDifferential::Auto),
        1..=3 => Some(HdrExposureDifferential::Ev(Rational {
            numerator: u32::from(code),
            denominator: 1,
        })),
        _ => None,
    }
}

/// The smoothing that byte 6 of HDRInfo's versions 0100 and 0101, `code`,
/// names: 0 [`Off`](HdrSmoothing::Off), 1 [`Normal`](HdrSmoothing::Normal),
/// 2 [`Low`](HdrSmoothing::Low), 3 [`High`](HdrSmoothing::High) and 48
/// [`Auto`](HdrSmoothing::Auto). `None` for every other code.
fn hdr_smoothing(code: u8) -> Option<HdrSmoothing> {
    Some(match code {
        0 => HdrSmoothing::Off,
        1 => HdrSmoothing::Normal,
        2 => HdrSmoothing::Low,
        3 => HdrSmoothing::High,
        48 => HdrSmoothing::Auto,
        _ => return None,
    })
}

/// The high ISO noise reduction step a HighISONoiseReduction `code`
/// names, in the list ExifTool's published documentation gives, read
/// whole, since files hold part of it (NEF notes, section 8.11):
///
/// - 0 [`Off`](HighIsoNoiseReduction::Off), 1
///   [`Minimal`](HighIsoNoiseReduction::Minimal), 2
///   [`Low`](HighIsoNoiseReduction::Low), 4
///   [`Normal`](HighIsoNoiseReduction::Normal) and 6
///   [`High`](HighIsoNoiseReduction::High), each on files, 6 on mine
///   alone;
/// - 3 [`MediumLow`](HighIsoNoiseReduction::MediumLow) and 5
///   [`MediumHigh`](HighIsoNoiseReduction::MediumHigh), documented only.
///
/// `None` for every other code.
fn high_iso_noise_reduction(code: u32) -> Option<HighIsoNoiseReduction> {
    Some(match code {
        0 => HighIsoNoiseReduction::Off,
        1 => HighIsoNoiseReduction::Minimal,
        2 => HighIsoNoiseReduction::Low,
        3 => HighIsoNoiseReduction::MediumLow,
        4 => HighIsoNoiseReduction::Normal,
        5 => HighIsoNoiseReduction::MediumHigh,
        6 => HighIsoNoiseReduction::High,
        _ => return None,
    })
}

/// Whether the long exposure noise reduction was on, as a NoiseReduction
/// `text`, as read, shows it, looked up whole: `FPNR`, which says the
/// camera took a dark frame and subtracted it, gives `Some(true)`.
///
/// The text says whether the reduction ran on the picture, not how the
/// camera was set, as the files show: on my D3200 every exposure over 1 s
/// says `FPNR` and every shorter one `OFF`, and no file of mine or of the
/// public set says `FPNR` under 1 s (NEF notes, section 8.11). So `OFF`
/// proves nothing of the setting and gives `None`, and so does every other
/// text. ExifTool's published documentation gives the tag as a string and
/// lists no values; the D600's manual says the reduction runs at shutter
/// speeds slower than 1 s.
fn long_exposure_noise_reduction_of_text(text: &str) -> Option<bool> {
    (text == "FPNR").then_some(true)
}

/// `exif_flash`, the flash the Exif tags gave, with the parts the
/// MakerNote's IFD `ifd` records, each by its own rule, so that a damaged
/// one costs only itself and leaves the part as `exif_flash` has it:
///
/// - the source and the control, from FlashType's text, read by the text
///   rule ([`flash_type_of_text`]);
/// - the sync, from FlashSetting's text, read the same way
///   ([`flash_sync_of_text`]). Its red-eye reduction text turns red-eye
///   reduction on, which Exif's bit for it, when clear, leaves unknown;
/// - the compensation, from FlashExposureComp
///   ([`flash_compensation`]). ExternalFlashExposureComp (0x0017), which
///   holds a flash unit's own setting beside it, is not read.
///
/// The parts are added to any flash Exif gives, one from a camera without
/// a flash function included: they are what the MakerNote records.
///
/// # Errors
///
/// `Io` when the reader fails.
fn flash<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd, exif_flash: Flash) -> Result<Flash> {
    let mut flash = exif_flash;
    if let Some((source, control)) =
        tiff_metadata::text(src, ifd, rules::FLASH_TYPE)?.and_then(|text| flash_type_of_text(&text))
    {
        flash = flash.with_source(source).with_control(control);
    }
    if let Some((sync, red_eye_reduction)) = tiff_metadata::text(src, ifd, rules::FLASH_SETTING)?
        .and_then(|text| flash_sync_of_text(&text))
    {
        flash = flash.with_sync(sync);
        if red_eye_reduction {
            flash = flash.with_red_eye_reduction(true);
        }
    }
    if let Some(compensation) = ifd
        .held(rules::FLASH_EXPOSURE_COMP)
        .and_then(flash_compensation)
    {
        flash = flash.with_compensation(compensation);
    }
    Ok(flash)
}

/// The flash compensation, in EV, that FlashExposureComp's four bytes
/// record: a signed count, then a factor and a divisor, read as the count
/// times the factor over the divisor, as ExposureTuning is read
/// ([`exposure_fine_tune`]), then a zero. So `fa 01 06 00` is -6/6 EV, and
/// a recorded zero, `00 01 06 00`, is a compensation of zero, as stored.
///
/// ExifTool's published documentation gives the tag's name and its four
/// bytes alone, and says the camera may record it whether or not the flash
/// fires. The reading rests on the D600's manual, which sets the
/// compensation from -3 to +1 EV in steps of 1/3 EV, and on the files:
/// every count is a multiple of two, from -18 to +6, with 1 and 6 after it
/// (NEF notes, section 8.10). It is the camera's setting for whichever
/// flash fires: files whose flash unit fired hold counts other than zero.
///
/// `None` when the divisor is zero, as the Z 50 and the Z50II write it
/// (`00 01 00 00`), and when the factor or the fourth byte breaks the
/// shape, a factor of zero, which would read any count as zero, or a
/// fourth byte other than zero; no file holds either.
fn flash_compensation([count, factor, divisor, zero]: [u8; 4]) -> Option<SignedRational> {
    if factor == 0 || zero != 0 {
        return None;
    }
    let numerator = i32::from(count.cast_signed()) * i32::from(factor);
    SignedRational::new(numerator, i32::from(divisor)).ok()
}

/// The flash sync that a FlashSetting `text`, as read, names, and whether
/// it also says red-eye reduction was on, looked up whole among the texts
/// files hold:
///
/// - `NORMAL`, front-curtain sync, [`FrontCurtain`](FlashSync::FrontCurtain);
/// - `REAR`, rear-curtain sync, [`RearCurtain`](FlashSync::RearCurtain);
/// - `SLOW`, slow sync, [`Slow`](FlashSync::Slow);
/// - `REAR SLOW`, rear-curtain with slow sync,
///   [`SlowRearCurtain`](FlashSync::SlowRearCurtain);
/// - `RED-EYE`, red-eye reduction, under which a lamp lights before the
///   flash fires as the shutter opens,
///   [`FrontCurtain`](FlashSync::FrontCurtain) with red-eye reduction on.
///
/// ExifTool's published documentation gives the tag as a string and lists
/// no values; each text's meaning is Nikon's, as the D600's manual names
/// the flash modes (NEF notes, section 8.10). `None` for every other text,
/// and for the blank some bodies write. A mode the manual offers but no
/// file shows, such as red-eye reduction with slow sync, gives none until
/// a file shows its text.
fn flash_sync_of_text(text: &str) -> Option<(FlashSync, bool)> {
    Some(match text {
        "NORMAL" => (FlashSync::FrontCurtain, false),
        "REAR" => (FlashSync::RearCurtain, false),
        "SLOW" => (FlashSync::Slow, false),
        "REAR SLOW" => (FlashSync::SlowRearCurtain, false),
        "RED-EYE" => (FlashSync::FrontCurtain, true),
        _ => return None,
    })
}

/// The flash's source and control that a FlashType `text`, as read,
/// names, looked up whole among the texts files hold. The part before the
/// comma is the source, `Built-in` [`BuiltIn`](FlashSource::BuiltIn) and
/// `Optional`, a flash unit, [`External`](FlashSource::External); the
/// part after it the control:
///
/// - `TTL` and `TTL-BL`, i-TTL and i-TTL balanced fill-flash,
///   [`Ttl`](FlashControl::Ttl);
/// - `AA`, auto aperture, [`AutoAperture`](FlashControl::AutoAperture);
/// - `M` [`Manual`](FlashControl::Manual);
/// - `RPT&Comdr.`, repeating flash in commander mode,
///   [`Repeating`](FlashControl::Repeating), and `TTL&Comdr.`, i-TTL in
///   commander mode, [`Ttl`](FlashControl::Ttl). Commander mode, in which
///   the camera fires other units, is not given.
///
/// ExifTool's published documentation gives the tag as a string and lists
/// no values; each text's meaning is Nikon's, as the D600's manual names
/// the settings (NEF notes, section 8.10). `None` for every other text:
/// the D100's `NEW_TTL`, which names neither a source nor a control, a
/// text the table lacks, and the blank a body writes when no flash fired.
/// A setting the manual offers but no file shows, such as the built-in
/// flash under manual control, gives none until a file shows its text.
fn flash_type_of_text(text: &str) -> Option<(FlashSource, FlashControl)> {
    use FlashControl::{AutoAperture, Manual, Repeating, Ttl};
    use FlashSource::{BuiltIn, External};
    Some(match text {
        "Built-in,TTL" | "Built-in,TTL-BL" => (BuiltIn, Ttl),
        "Optional,TTL" | "Optional,TTL-BL" | "Optional,TTL&Comdr." => (External, Ttl),
        "Optional,AA" => (External, AutoAperture),
        "Optional,M" => (External, Manual),
        "Optional,RPT&Comdr." => (External, Repeating),
        _ => return None,
    })
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
/// - `PRESET`, a custom white balance whose slot the text does not give,
///   [`Custom`](WhiteBalanceMode::Custom).
///
/// Two texts are read by their form, since no table could list every
/// number:
///
/// - a chosen color temperature, written as its number
///   ([`kelvin_of_text`]), gives
///   [`ColorTemperature`](WhiteBalanceMode::ColorTemperature);
/// - a preset slot, `PRESET` and the slot's number, such as `PRESET1` or
///   the D3X's `PRESET0` ([`preset_slot_of_text`]), gives
///   [`Custom`](WhiteBalanceMode::Custom).
///
/// `None` for every other text, such as `AUTO3`, which no checked file
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
        "PRESET" => WhiteBalanceMode::Custom,
        _ if kelvin_of_text(text).is_some() => WhiteBalanceMode::ColorTemperature,
        _ if preset_slot_of_text(text).is_some() => WhiteBalanceMode::Custom,
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

/// The most digits the number of a WhiteBalance text that records a preset
/// slot may have: the rule's own bound, past the one digit of every such
/// text checked. The bodies number their slots from 0 or 1 to at most 6.
const MAX_PRESET_DIGITS: usize = 2;

/// The preset slot that the WhiteBalance text `text`, as read, records as
/// the number after `PRESET`: `PRESET1` for slot 1, the one NX Studio
/// calls "d-1", and the D3X's `PRESET0` for slot 0. That is `PRESET`, then
/// one to [`MAX_PRESET_DIGITS`] ASCII digits, and nothing else, so `PRESET`
/// alone, `PRESETX`, `PRESET 1` and `preset1` give `None`.
///
/// The NEF notes mark `PRESET1` to `PRESET4` confirmed by my frames, and
/// `PRESET0` read by its form from the D3X's file (section 8.10).
fn preset_slot_of_text(text: &str) -> Option<u32> {
    let digits = text.strip_prefix("PRESET")?;
    if !(1..=MAX_PRESET_DIGITS).contains(&digits.len())
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    // At most two ASCII digits always parse into a u32.
    digits.parse().ok()
}

/// The white balance fine-tuning that `makernote` records, signed as
/// [`WhiteBalanceFineTune`] signs it ([`fine_tune_of`]):
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
) -> Result<Option<WhiteBalanceFineTune>> {
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
/// frames shot on a Z 9 and a D600 with a known shift show.
/// [`WhiteBalanceFineTune`] signs each value as its axis is named, so its
/// green to magenta shift is positive towards magenta, and Nikon's second
/// value is turned around (ADR011).
///
/// `None` when either denominator is zero, which records no fraction, and
/// when the second numerator is `i32::MIN`, whose negation does not fit. No
/// file checked holds either.
fn fine_tune_of(
    (amber_blue, amber_blue_denominator): (i32, i32),
    (magenta_green, magenta_green_denominator): (i32, i32),
) -> Option<WhiteBalanceFineTune> {
    let amber_blue = SignedRational::new(amber_blue, amber_blue_denominator).ok()?;
    let green_magenta =
        SignedRational::new(magenta_green.checked_neg()?, magenta_green_denominator).ok()?;
    Some(WhiteBalanceFineTune::new(amber_blue, green_magenta))
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

/// Whether the MakerNote's IFD `ifd` holds no entry of tag `tag` at all.
/// An entry the IFD skipped as damaged, such as one of an unknown type or
/// whose value lies outside the block, is not kept by its tag, so `true`
/// only when the IFD skipped none: a skipped entry might be the tag's, and
/// what it recorded unknown.
fn holds_no(ifd: &Ifd, tag: u16) -> bool {
    ifd.entry(tag).is_none() && ifd.invalid_entries() == 0
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
/// say which sensor the body has: [`camera_settings`] then takes the
/// body's sensor format from raw-files' table. `None` too for 3, a 5:4
/// crop, 4, a 3:2 crop, and 8, a 2.7x crop, which no checked file holds;
/// for the other movie crops; and for every code nothing names.
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

/// The code of Nikon's NEFCompression list that the MakerNote's IFD `ifd`
/// records for a compressed raw: NEFCompression's one SHORT, or, on a body
/// that writes no NEFCompression, the 16-bit number at bytes 10 and 11 of
/// tag 0x0051, in the MakerNote's byte order, as the NEF notes give it
/// (section 2). A body that writes NEFCompression never falls back on
/// 0x0051, which no file holds beside it.
///
/// `None` when NEFCompression breaks its rule, and when 0x0051 is missing,
/// is not UNDEFINED, stores its value outside the block or is shorter
/// than 12 bytes.
///
/// # Errors
///
/// `Io` when the reader fails.
fn nef_compression_code<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd) -> Result<Option<u32>> {
    if ifd.entry(super::NEF_COMPRESSION).is_some() {
        return Ok(ifd.held(rules::NEF_COMPRESSION).map(|[code]| code));
    }
    let Some(record) = ifd.get(src, rules::COMPRESSION_RECORD)? else {
        return Ok(None);
    };
    Ok(record
        .get(10..12)
        .map(|code| u32::from(ifd.order().u16([code[0], code[1]]))))
}

/// The raw compression a NEFCompression `code` names, in the one list
/// ExifTool's published documentation gives for NEFCompression and for
/// tag 0x0051, read whole, since files hold part of it in each tag (NEF
/// notes, section 8.10):
///
/// - 1, lossy (type 1), and 4, lossy (type 2),
///   [`Lossy`](RawCompression::Lossy);
/// - 2, uncompressed; 5, striped packed 12 bits; 6, uncompressed reduced
///   to 12 bits; 7, unpacked 12 bits; 9, packed 12 bits; and 10, packed 14
///   bits, [`Uncompressed`](RawCompression::Uncompressed): packings of
///   uncompressed data;
/// - 3, lossless, [`Lossless`](RawCompression::Lossless);
/// - 13, High Efficiency, [`HighEfficiency`](RawCompression::HighEfficiency);
/// - 14, High Efficiency\*,
///   [`HighEfficiencyStar`](RawCompression::HighEfficiencyStar).
///
/// Code 5 is documented only; the others are on files. `None` for 8,
/// small, which names a size and not a compression, and for every code the
/// list lacks.
fn nef_compression(code: u32) -> Option<RawCompression> {
    Some(match code {
        1 | 4 => RawCompression::Lossy,
        2 | 5 | 6 | 7 | 9 | 10 => RawCompression::Uncompressed,
        3 => RawCompression::Lossless,
        13 => RawCompression::HighEfficiency,
        14 => RawCompression::HighEfficiencyStar,
        _ => return None,
    })
}

/// The JPEG quality step that a JPEG's Quality text `text`, as read,
/// names, looked up whole among the texts files hold, as the D600's
/// manual and the Z 9's Reference Guide name the steps (NEF notes,
/// section 8.10):
///
/// - `FINE`, JPEG fine, [`Fine`](JpegQuality::Fine);
/// - `NORMAL`, JPEG normal, [`Normal`](JpegQuality::Normal);
/// - `BASIC`, JPEG basic, [`Basic`](JpegQuality::Basic).
///
/// `None` for every other text: `RAW`, which a JPEG holds when a program
/// made it from a raw file and kept the raw file's MakerNote, and a text
/// the table lacks.
fn jpeg_quality_of_text(text: &str) -> Option<JpegQuality> {
    Some(match text {
        "FINE" => JpegQuality::Fine,
        "NORMAL" => JpegQuality::Normal,
        "BASIC" => JpegQuality::Basic,
        _ => return None,
    })
}

/// The JPEG compression a JPGCompression `code` names, in the list
/// ExifTool's published documentation gives, read whole: 1
/// [`SizePriority`](JpegCompression::SizePriority) and 3
/// [`OptimalQuality`](JpegCompression::OptimalQuality), both on files.
/// `None` for 0, which the documentation does not give and every raw file
/// that records the tag holds, and for every other code. On every measured
/// file, 3 comes with a JPEG fine and 1 with a JPEG basic; that the code
/// is the star and not the step rests on the documentation and the Z 9's
/// Reference Guide (NEF notes, section 8.10).
fn jpeg_compression(code: u32) -> Option<JpegCompression> {
    Some(match code {
        1 => JpegCompression::SizePriority,
        3 => JpegCompression::OptimalQuality,
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

/// The stabilization that VRInfo's `record` holds, at the versions whose
/// layout ExifTool's published documentation gives and measured files
/// hold, each of its length: 0100, on the DSLRs from the D2Hs and the D2X
/// on, and 0101, on the Nikon 1 bodies, of 8 bytes; 0200, on the Z
/// bodies, the D6 and the D780, of 9 bytes.
///
/// Byte 4 is 1 on and 2 off. Under on, byte 6 gives the mode by the
/// version's list ([`stabilization_mode`]), and a code the list does not
/// name gives none. Under off, the record gives no mode.
///
/// `None` for any other version, for a record of another length than its
/// version's, and for any other byte 4, such as the 0 four public files
/// hold. Byte 5, which repeats byte 4 on the files, and byte 8 of version
/// 0200, the kind of stabilization, are not read (NEF notes, section
/// 8.10).
fn stabilization(record: &[u8]) -> Option<Stabilization> {
    let len = match record.get(..4)? {
        b"0100" | b"0101" => 8,
        b"0200" => 9,
        _ => return None,
    };
    if record.len() != len {
        return None;
    }
    let &[v0, v1, v2, v3, on, _, mode, ..] = record else {
        return None;
    };
    match on {
        1 => {
            let on = Stabilization::on();
            Some(match stabilization_mode([v0, v1, v2, v3], mode) {
                Some(mode) => on.with_mode(mode),
                None => on,
            })
        }
        2 => Some(Stabilization::off()),
        _ => None,
    }
}

/// The stabilization mode VRInfo's seventh byte, `code`, names in the list
/// the documentation gives for the record's `version`, each read whole,
/// since files hold part of each:
///
/// - at 0100 and 0101, 0 [`Normal`](StabilizationMode::Normal), 2
///   [`Active`](StabilizationMode::Active) and 3
///   [`Sport`](StabilizationMode::Sport); 1, which the documentation calls
///   "On (1)", names no mode;
/// - at 0200, the list for the Z bodies, 1
///   [`Normal`](StabilizationMode::Normal) and 3
///   [`Sport`](StabilizationMode::Sport); 0, which it calls "Off", names
///   no mode, since byte 4 says whether it was on.
///
/// `None` for every other code.
fn stabilization_mode(version: [u8; 4], code: u8) -> Option<StabilizationMode> {
    Some(match (&version, code) {
        (b"0100" | b"0101", 0) | (b"0200", 1) => StabilizationMode::Normal,
        (b"0100" | b"0101", 2) => StabilizationMode::Active,
        (b"0100" | b"0101" | b"0200", 3) => StabilizationMode::Sport,
        _ => return None,
    })
}

/// The exposure fine-tuning, in EV, that ExposureTuning's three bytes
/// record: a signed count, then a factor and a divisor, read as the count
/// times the factor over the divisor, so that `fd 01 06` is -3/6 EV. A
/// recorded zero, `00 01 06`, is a fine-tuning of zero, as stored.
///
/// ExifTool's published documentation gives the tag's name and its three
/// bytes alone. The reading rests on the D600's manual, which sets the
/// fine-tuning from -1 to +1 EV in steps of 1/6 EV, and on the files:
/// every one holds 1 and 6 after the count, and the four public D750 files
/// that tune it hold -3 (NEF notes, section 8.10).
///
/// `None` when the divisor is zero, which records no fraction, and when
/// the factor is zero, which would read any count as zero, as for the
/// flash compensation ([`flash_compensation`]); no file holds either.
fn exposure_fine_tune([count, factor, divisor]: [u8; 3]) -> Option<SignedRational> {
    if factor == 0 {
        return None;
    }
    let numerator = i32::from(count.cast_signed()) * i32::from(factor);
    SignedRational::new(numerator, i32::from(divisor)).ok()
}

/// ShootingMode's bit for continuous release.
const CONTINUOUS: u32 = 1 << 0;
/// ShootingMode's bit for the self-timer.
const SELF_TIMER: u32 = 1 << 3;
/// ShootingMode's bit for exposure bracketing.
const EXPOSURE_BRACKETING: u32 = 1 << 4;
/// ShootingMode's bit for auto ISO, on every body but the D70.
const AUTO_ISO: u32 = 1 << 5;
/// ShootingMode's bit for white balance bracketing.
const WHITE_BALANCE_BRACKETING: u32 = 1 << 6;
/// ShootingMode's bit for a release by an infrared remote, which the
/// documentation names IR Control.
const REMOTE: u32 = 1 << 7;
/// ShootingMode's bit for Active D-Lighting bracketing.
const ACTIVE_D_LIGHTING_BRACKETING: u32 = 1 << 8;
/// ShootingMode's bit for pre-capture, a burst that begins before the
/// shutter button is pressed all the way.
const PRE_CAPTURE: u32 = 1 << 11;

/// The Model of the one body whose ShootingMode gives bit 5 another
/// meaning than auto ISO, as ExifTool's published documentation says.
const D70: &str = "NIKON D70";

/// Whether auto ISO was on, as ShootingMode's `bits` record it in bit 5:
/// `true` when set, `false` when clear.
///
/// `None` when `model`, the file's Model, is the D70's: the documentation
/// gives its bit 5 another meaning, and the one public D70 file sets it.
/// A file without a Model takes the meaning every other body's bit has
/// (NEF notes, section 8.10).
fn auto_iso(bits: u32, model: Option<&str>) -> Option<bool> {
    (model != Some(D70)).then_some(bits & AUTO_ISO != 0)
}

/// Whether auto ISO was on, as the Coolpix bodies' ISOSelection `text`,
/// as read, records it: `AUTO` `true`, and `MANUAL`, a chosen ISO,
/// `false`. Both are on public files. `None` for every other text, such
/// as `auto` in lower case.
fn auto_iso_of_text(text: &str) -> Option<bool> {
    match text {
        "AUTO" => Some(true),
        "MANUAL" => Some(false),
        _ => None,
    }
}

/// The release mode that ShootingMode's `bits` record, in the list of bits
/// ExifTool's published documentation gives, read as documentation and
/// read whole, since files hold part of it:
///
/// - bit 0, continuous, [`Continuous`](ReleaseMode::Continuous);
/// - bit 3, the self-timer, [`SelfTimer`](ReleaseMode::SelfTimer);
/// - bit 7, IR Control, an infrared remote,
///   [`Remote`](ReleaseMode::Remote);
/// - none of the three, [`Single`](ReleaseMode::Single): no bit names a
///   single frame, and Nikon's single frame, quiet release and mirror up
///   all set none, so the three fold into one (NEF notes, section 8.10).
///
/// `None` when two of the three are set, which names two modes and which
/// no file holds, and when bit 11, pre-capture, is set: a burst, which
/// would otherwise read as a single frame, and which no file holds. The
/// other bits play no part: the exposure delay mode and a release from a
/// computer, bits 1 and 2, come with any of the modes.
fn release_mode(bits: u32) -> Option<ReleaseMode> {
    if bits & PRE_CAPTURE != 0 {
        return None;
    }
    match bits & (CONTINUOUS | SELF_TIMER | REMOTE) {
        0 => Some(ReleaseMode::Single),
        CONTINUOUS => Some(ReleaseMode::Continuous),
        SELF_TIMER => Some(ReleaseMode::SelfTimer),
        REMOTE => Some(ReleaseMode::Remote),
        _ => None,
    }
}

/// The bracketing that ShootingMode's `bits` record, in the same list:
///
/// - bit 4 [`Exposure`](Bracketing::Exposure);
/// - bit 6 [`WhiteBalance`](Bracketing::WhiteBalance);
/// - bit 8, Active D-Lighting bracketing,
///   [`DynamicRange`](Bracketing::DynamicRange);
/// - none of the three [`Off`](Bracketing::Off).
///
/// Files hold bit 4; bits 6 and 8 are documented only, and named with the
/// list. `None` when two of the three are set, which no file holds.
fn bracketing(bits: u32) -> Option<Bracketing> {
    match bits & (EXPOSURE_BRACKETING | WHITE_BALANCE_BRACKETING | ACTIVE_D_LIGHTING_BRACKETING) {
        0 => Some(Bracketing::Off),
        EXPOSURE_BRACKETING => Some(Bracketing::Exposure),
        WHITE_BALANCE_BRACKETING => Some(Bracketing::WhiteBalance),
        ACTIVE_D_LIGHTING_BRACKETING => Some(Bracketing::DynamicRange),
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
        AT, CFA, D600_FX, ORDERS, RAW, TAIL, Tags, entry_at, exif_file, exif_metadata,
        jpeg_metadata_of, metadata_of, nikon_file, nikon_file_with, raw_file, read_file, retyped,
        tags_block, whole,
    };
    use super::super::{
        ACTIVE_D_LIGHTING, AF_INFO, AF_INFO2, AF_TUNE, COLOR_SPACE, COLOR_TEMPERATURE,
        COMPRESSION_RECORD, COOLPIX_PICTURE_CONTROL, CROP_HI_SPEED, EXPOSURE_TUNING, FINE_TUNE,
        FINE_TUNE_STEPS, FLASH_EXPOSURE_COMP, FLASH_SETTING, FLASH_TYPE, FOCUS_MODE, HDR_INFO,
        HIGH_ISO_NOISE_REDUCTION, IMAGE_SIZE_RAW, ISO_SELECTION, JPG_COMPRESSION, NEF_COMPRESSION,
        NOISE_REDUCTION, PICTURE_CONTROL, QUALITY, SHOOTING_MODE, SHUTTER_MODE, VR_INFO,
        WHITE_BALANCE, WORLD_TIME, metadata,
    };
    use super::*;
    use crate::camera_settings::{
        ExposureProgram, FlashMode, FluorescentLamp, PictureControlValue, WhiteBalance,
    };
    use crate::container::tiff::tags::{
        COLOR_SPACE as EXIF_COLOR_SPACE, COMPRESSION, EXPOSURE_MODE, EXPOSURE_PROGRAM,
        FLASH as EXIF_FLASH, IMAGE_LENGTH, IMAGE_WIDTH, INTEROP_INDEX, LIGHT_SOURCE, MAKE,
        MAKER_NOTE, MODEL, PHOTOMETRIC_INTERPRETATION, WHITE_BALANCE as EXIF_WHITE_BALANCE,
    };
    use crate::container::tiff::{self, walk};
    use crate::error::ErrorKind;
    use crate::limits::Limits;
    use crate::testing::{FAR, FailsFar, signed_fraction};
    use crate::tiff_builder::{
        ASCII, BYTE, IfdId, Order, RawValue, SSHORT, TiffBuilder, UNDEFINED,
    };
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
            // An auto setting no checked file shows.
            "AUTO3",
            // The table's texts in other cases, or cut short: no prefix
            // or case rule.
            "Auto1",
            "SUN",
            // Near a preset slot, but not its form.
            "PRESET1X",
            "PRESETX",
            "PRESET 1",
            "preset1",
            "PRESET123",
            // Near a color temperature, but not its form.
            "K",
            "123456K",
            "3230k",
            "3230 K",
            "K3230",
            "32.5K",
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
    fn a_white_balance_text_other_than_fluorescent_drops_exifs_lamp() {
        // Exif's manual white balance for a cool white lamp, beside each
        // text: the lamp stays under `COOL WHT FL` and goes under a text
        // that names another mode. Under a text the table lacks, Exif's
        // fluorescent mode and lamp stay.
        let cool_white = |builder: &mut TiffBuilder, exif: IfdId| {
            builder
                .short(exif, EXIF_WHITE_BALANCE, &[1])
                .short(exif, LIGHT_SOURCE, &[14]);
        };
        for order in ORDERS {
            for (text, mode, lamp) in [
                (
                    "COOL WHT FL",
                    WhiteBalanceMode::Fluorescent,
                    Some(FluorescentLamp::CoolWhite),
                ),
                ("SUNNY", WhiteBalanceMode::Daylight, None),
                ("PRESET1", WhiteBalanceMode::Custom, None),
                ("3230K", WhiteBalanceMode::ColorTemperature, None),
                (
                    "AUTO3",
                    WhiteBalanceMode::Fluorescent,
                    Some(FluorescentLamp::CoolWhite),
                ),
            ] {
                let read = settings_with_exif(
                    order,
                    |inner, ifd| {
                        inner.ascii(ifd, WHITE_BALANCE, &padded(text));
                    },
                    cool_white,
                )
                .white_balance;
                assert_eq!(read.mode, Some(mode), "{order:?}, {text:?}");
                assert_eq!(read.fluorescent_lamp, lamp, "{order:?}, {text:?}");
            }
        }
    }

    #[test]
    fn a_preset_texts_number_is_its_slot() {
        // A bare `PRESET` is custom with no slot; `PRESET` and a number of
        // one or two digits is custom with that slot, whatever the number,
        // as a chosen color temperature is read by its form. Beside Exif's
        // shade, to show the MakerNote's mode wins.
        let cases = [
            ("PRESET", None),
            ("PRESET0", Some(0)),
            ("PRESET1", Some(1)),
            ("PRESET4", Some(4)),
            ("PRESET5", Some(5)),
            ("PRESET12", Some(12)),
            ("PRESET07", Some(7)),
        ];
        for order in ORDERS {
            for (text, slot) in cases {
                let read = settings_with_exif(
                    order,
                    |inner, ifd| {
                        inner.ascii(ifd, WHITE_BALANCE, &padded(text));
                    },
                    exif_shade,
                )
                .white_balance;
                let what = format!("{order:?}, {text:?}");
                assert_eq!(read.mode, Some(WhiteBalanceMode::Custom), "{what}");
                assert_eq!(read.preset_slot, slot, "{what}");
            }
            // Not the form: no slot, and Exif's mode stands.
            for text in ["PRESETX", "PRESET123", "PRESET 1", "SUNNY", "3230K"] {
                let read = settings_with_exif(
                    order,
                    |inner, ifd| {
                        inner.ascii(ifd, WHITE_BALANCE, &padded(text));
                    },
                    exif_shade,
                )
                .white_balance;
                assert_eq!(read.preset_slot, None, "{order:?}, {text:?}");
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
    /// denominator, as [`WhiteBalanceFineTune`] holds it.
    fn shift(amber_blue: (i32, i32), green_magenta: (i32, i32)) -> WhiteBalanceFineTune {
        let (amber_blue, green_magenta) = (
            signed_fraction(amber_blue.0, amber_blue.1),
            signed_fraction(green_magenta.0, green_magenta.1),
        );
        WhiteBalanceFineTune::new(amber_blue, green_magenta)
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
    ) -> Option<WhiteBalanceFineTune> {
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

    /// A file in byte order `order` whose IFD0 holds the Make `make` and
    /// the Model `model`, and raw data of size [`RAW`] when `raw` is true,
    /// and whose MakerNote holds what `tags` adds to its MakerNote IFD.
    fn body_file(order: Order, make: &str, model: &str, raw: bool, tags: Tags) -> Vec<u8> {
        let block = tags_block(order, tags).0;
        let count = whole(&block);
        body_file_of(order, make, model, raw, &block, count)
    }

    /// As [`body_file`], with the MakerNote the first `count` bytes of
    /// `block`.
    fn body_file_of(
        order: Order,
        make: &str,
        model: &str,
        raw: bool,
        block: &[u8],
        count: u64,
    ) -> Vec<u8> {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder
            .ascii(ifd0, MAKE, make)
            .ascii(ifd0, MODEL, model)
            .exif(ifd0, exif);
        if raw {
            builder
                .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[CFA])
                .long(ifd0, IMAGE_WIDTH, &[RAW.width])
                .long(ifd0, IMAGE_LENGTH, &[RAW.height]);
        }
        builder.undefined(exif, 0x9000, b"0232");
        entry_at(&mut builder, exif, MAKER_NOTE, UNDEFINED, AT, count);
        builder.blob_at(AT, block);
        builder.pad_to(AT + whole(block) + TAIL);
        builder.build().bytes
    }

    /// The camera settings of the file [`body_file`] builds, of a Nikon
    /// body, with raw data, read as a raw file.
    fn body_settings(order: Order, model: &str, tags: Tags) -> CameraSettings {
        metadata_of(body_file(order, "NIKON CORPORATION", model, true, tags)).camera_settings
    }

    /// The CropHiSpeed of the D600's DX crop, as its DX frames record it.
    const D600_DX: [u16; 7] = [2, 6080, 4028, 3968, 2636, 1040, 696];

    #[test]
    fn a_crop_setting_of_off_gives_the_sensor_format_of_a_body_the_table_holds() {
        // The D600's FX frames record 0 and the D300S's frames too: the
        // table says which sensor each has.
        let cases: [(&str, Tags, ImageArea); 4] = [
            (
                "NIKON D600",
                |inner, ifd| {
                    inner.short(ifd, CROP_HI_SPEED, &D600_FX);
                },
                ImageArea::FullFrame,
            ),
            (
                "NIKON D300S",
                |inner, ifd| {
                    inner.short(ifd, CROP_HI_SPEED, &[0, 4352, 2868, 4352, 2868, 0, 0]);
                },
                ImageArea::ApsC,
            ),
            // No CropHiSpeed at all.
            ("NIKON D600", |_, _| {}, ImageArea::FullFrame),
            ("NIKON D7100", |_, _| {}, ImageArea::ApsC),
        ];
        for order in ORDERS {
            for (model, tags, area) in cases {
                let read = body_settings(order, model, tags);
                assert_eq!(read.image_area, Some(area), "{order:?}, {model}");
            }
        }
    }

    #[test]
    fn a_crop_setting_that_names_an_area_comes_before_the_table() {
        // The D600's DX frames record 2: the crop, not the sensor.
        for order in ORDERS {
            let read = body_settings(order, "NIKON D600", |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &D600_DX);
            });
            assert_eq!(read.image_area, Some(ImageArea::ApsC), "{order:?}");
        }
    }

    #[test]
    fn a_crop_setting_that_names_no_area_for_another_reason_gives_none_from_the_table() {
        // A 3:2 crop, a movie crop and a code nothing names say something
        // other than "off", so the sensor's format would not be the
        // picture's area.
        let cases: [(u16, Tags); 3] = [
            (4, |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &[4, 6080, 4028, 6080, 4028, 0, 0]);
            }),
            (13, |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &[13, 6080, 4028, 6080, 4028, 0, 0]);
            }),
            (u16::MAX, |inner, ifd| {
                inner.short(
                    ifd,
                    CROP_HI_SPEED,
                    &[u16::MAX, 6080, 4028, 6080, 4028, 0, 0],
                );
            }),
        ];
        for order in ORDERS {
            for (mode, tags) in cases {
                let read = body_settings(order, "NIKON D600", tags);
                assert_eq!(read.image_area, None, "{order:?}, {mode}");
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_that_breaks_its_rule_gives_none_from_the_table() {
        // Six values and seven LONGs: the setting cannot be read, and may
        // have named a crop.
        let cases: [Tags; 2] = [
            |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &D600_FX[..6]);
            },
            |inner, ifd| {
                inner.long(ifd, CROP_HI_SPEED, &D600_FX.map(u32::from));
            },
        ];
        for order in ORDERS {
            for (place, tags) in cases.into_iter().enumerate() {
                let read = body_settings(order, "NIKON D600", tags);
                assert_eq!(read.image_area, None, "{order:?}, case {place}");
            }
        }
    }

    #[test]
    fn a_crop_setting_of_off_with_a_smaller_cropped_size_gives_none_from_the_table() {
        // "Off" beside the sizes of the D600's DX crop: the picture was
        // cut, so the whole sensor's format would not be its area. On the
        // measured files of the table's bodies, "off" never comes so.
        for order in ORDERS {
            let read = body_settings(order, "NIKON D600", |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &[0, 6080, 4028, 3968, 2636, 1040, 696]);
            });
            assert_eq!(read.image_area, None, "{order:?}");
        }
    }

    #[test]
    fn a_crop_hi_speed_the_ifd_skipped_as_damaged_gives_none_from_the_table() {
        // The D600's DX crop with an unknown type, or with its value, the
        // block's last 14 bytes, cut off by the MakerNote's end: the IFD
        // skips it, so it is not seen by its tag, and the shot was a crop.
        for order in ORDERS {
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &D600_DX);
            });
            let unknown = retyped(order, block.clone(), entry, 99, 7);
            for (what, block, count) in [
                ("unknown type", &unknown, whole(&unknown)),
                ("past the block", &block, whole(&block) - 1),
            ] {
                let file =
                    body_file_of(order, "NIKON CORPORATION", "NIKON D600", true, block, count);
                let read = metadata_of(file).camera_settings;
                assert_eq!(read.image_area, None, "{order:?}, {what}");
            }
            // The control: the same block, whole, gives the crop.
            let file = body_file_of(
                order,
                "NIKON CORPORATION",
                "NIKON D600",
                true,
                &block,
                whole(&block),
            );
            let read = metadata_of(file).camera_settings;
            assert_eq!(read.image_area, Some(ImageArea::ApsC), "{order:?}");
        }
    }

    #[test]
    fn a_body_the_table_holds_no_facts_for_gives_no_area_for_off() {
        // A body outside the table, one with a name and no facts, and no
        // Model at all.
        for order in ORDERS {
            for model in ["NIKON D7200", "NIKON Z 9"] {
                let read = body_settings(order, model, |inner, ifd| {
                    inner.short(ifd, CROP_HI_SPEED, &D600_FX);
                });
                assert_eq!(read.image_area, None, "{order:?}, {model}");
                let read = body_settings(order, model, |_, _| {});
                assert_eq!(read.image_area, None, "{order:?}, {model}, no tag");
            }
            assert_eq!(image_area_of(order, 0), None, "{order:?}, no Model");
        }
    }

    #[test]
    fn the_table_is_looked_up_by_the_maker_and_the_model_together() {
        // A Make no maker is recognized from: the Nikon reader still reads
        // the file, by its looser test, but the table names no body.
        for order in ORDERS {
            let tags: Tags = |inner, ifd| {
                inner.short(ifd, CROP_HI_SPEED, &D600_FX);
            };
            let file = body_file(order, "Nikon", "NIKON D600", true, tags);
            assert_eq!(
                metadata_of(file).camera_settings.image_area,
                None,
                "{order:?}"
            );
            // The control: the Make the D600 writes.
            let file = body_file(order, "NIKON CORPORATION", "NIKON D600", true, tags);
            assert_eq!(
                metadata_of(file).camera_settings.image_area,
                Some(ImageArea::FullFrame),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_d600_jpeg_gives_the_sensor_format_too() {
        // The image area is the shot's, in a JPEG as in a raw file.
        for order in ORDERS {
            let file = body_file(
                order,
                "NIKON CORPORATION",
                "NIKON D600",
                false,
                |inner, ifd| {
                    inner.short(ifd, CROP_HI_SPEED, &D600_FX);
                },
            );
            let read = jpeg_metadata_of(file, [6016, 4016]).camera_settings;
            assert_eq!(read.image_area, Some(ImageArea::FullFrame), "{order:?}");
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
    fn a_raw_file_of_a_body_that_offers_no_raw_sizes_is_large() {
        // The D600 and the D3200 write no ImageSizeRAW, and offer one raw
        // size.
        for order in ORDERS {
            for model in ["NIKON D600", "NIKON D3200"] {
                let read = body_settings(order, model, |_, _| {});
                assert_eq!(
                    read.raw_image_size,
                    Some(RawImageSize::Large),
                    "{order:?}, {model}"
                );
            }
        }
    }

    #[test]
    fn a_recorded_raw_size_comes_before_the_table() {
        // A code read gives its size; 2, the medium size, which no checked
        // raw file holds and so is not read, 0, which nothing names, and a
        // tag that breaks its rule give none: the file said something.
        let cases: [(Tags, Option<RawImageSize>); 4] = [
            (
                |inner, ifd| {
                    inner.short(ifd, IMAGE_SIZE_RAW, &[3]);
                },
                Some(RawImageSize::Small),
            ),
            (
                |inner, ifd| {
                    inner.short(ifd, IMAGE_SIZE_RAW, &[2]);
                },
                None,
            ),
            (
                |inner, ifd| {
                    inner.short(ifd, IMAGE_SIZE_RAW, &[0]);
                },
                None,
            ),
            (
                |inner, ifd| {
                    inner.long(ifd, IMAGE_SIZE_RAW, &[1]);
                },
                None,
            ),
        ];
        for order in ORDERS {
            for (place, (tags, size)) in cases.into_iter().enumerate() {
                let read = body_settings(order, "NIKON D600", tags);
                assert_eq!(read.raw_image_size, size, "{order:?}, case {place}");
            }
        }
    }

    #[test]
    fn an_image_size_raw_the_ifd_skipped_as_damaged_gives_none_from_the_table() {
        // ImageSizeRAW 3, small, of an unknown type: the IFD skips it, so it
        // is not seen by its tag, and the raw was not large.
        for order in ORDERS {
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.short(ifd, IMAGE_SIZE_RAW, &[3]);
            });
            let unknown = retyped(order, block, entry, 99, 1);
            let file = body_file_of(
                order,
                "NIKON CORPORATION",
                "NIKON D600",
                true,
                &unknown,
                whole(&unknown),
            );
            let read = metadata_of(file).camera_settings;
            assert_eq!(read.raw_image_size, None, "{order:?}");
        }
    }

    #[test]
    fn a_body_the_table_holds_no_facts_for_gives_no_raw_size_without_the_tag() {
        // A body outside the table, one with a name and no facts, a Make no
        // maker is recognized from, and no Model at all.
        for order in ORDERS {
            for model in ["NIKON D7200", "NIKON Z 9"] {
                let read = body_settings(order, model, |_, _| {});
                assert_eq!(read.raw_image_size, None, "{order:?}, {model}");
            }
            let file = body_file(order, "Nikon", "NIKON D600", true, |_, _| {});
            assert_eq!(metadata_of(file).camera_settings.raw_image_size, None);
            assert_eq!(settings_of(order, |_, _| {}).raw_image_size, None);
        }
    }

    #[test]
    fn a_file_without_raw_data_of_a_body_the_table_holds_has_no_raw_size() {
        // A TIFF image a D300S or a program wrote is read as a raw file, as
        // this one is, and holds no raw data to have a size. Its image area
        // is still the shot's.
        for order in ORDERS {
            let file = body_file(
                order,
                "NIKON CORPORATION",
                "NIKON D600",
                false,
                |inner, ifd| {
                    inner.short(ifd, CROP_HI_SPEED, &D600_FX);
                },
            );
            let read = metadata_of(file).camera_settings;
            assert_eq!(read.raw_image_size, None, "{order:?}");
            assert_eq!(read.image_area, Some(ImageArea::FullFrame), "{order:?}");
        }
    }

    #[test]
    fn a_jpeg_of_a_body_the_table_holds_has_no_raw_size() {
        // A JPEG holds no raw data, whatever the body offers.
        for order in ORDERS {
            let file = body_file(order, "NIKON CORPORATION", "NIKON D600", false, |_, _| {});
            let read = jpeg_metadata_of(file, [6016, 4016]).camera_settings;
            assert_eq!(read.raw_image_size, None, "{order:?}");
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

    /// The raw compression of a raw file in byte order `order` whose raw
    /// IFD's Compression is `compression`, when there is one, and whose
    /// MakerNote holds what `tags` adds to its MakerNote IFD.
    fn compression_of(
        order: Order,
        compression: Option<u16>,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Option<RawCompression> {
        let block = tags_block(order, tags).0;
        let file = raw_file(order, &block, whole(&block), |builder, ifd0| {
            if let Some(code) = compression {
                builder.short(ifd0, COMPRESSION, &[code]);
            }
        });
        metadata_of(file).camera_settings.raw_compression
    }

    /// The 24 bytes of a tag 0x0051 as the Z 9 writes it, in byte order
    /// `order`, with NEFCompression's `code` at bytes 10 and 11.
    fn compression_record(order: Order, code: u16) -> Vec<u8> {
        let mut record = b"01010500".to_vec();
        record.extend_from_slice(&[0x18, 0x00]);
        record.extend_from_slice(&order.u16(code));
        record.resize(24, 0);
        record
    }

    /// Each code of NEFCompression's list with the raw compression it
    /// names, and the codes that name none: 8, small, and codes the list
    /// lacks.
    const NEF_COMPRESSIONS: [(u16, Option<RawCompression>); 16] = [
        (1, Some(RawCompression::Lossy)),
        (2, Some(RawCompression::Uncompressed)),
        (3, Some(RawCompression::Lossless)),
        (4, Some(RawCompression::Lossy)),
        (5, Some(RawCompression::Uncompressed)),
        (6, Some(RawCompression::Uncompressed)),
        (7, Some(RawCompression::Uncompressed)),
        (8, None),
        (9, Some(RawCompression::Uncompressed)),
        (10, Some(RawCompression::Uncompressed)),
        (13, Some(RawCompression::HighEfficiency)),
        (14, Some(RawCompression::HighEfficiencyStar)),
        (0, None),
        (11, None),
        (12, None),
        (15, None),
    ];

    #[test]
    fn each_nef_compression_code_refines_nikons_compressed_form() {
        for order in ORDERS {
            for (code, expected) in NEF_COMPRESSIONS {
                let read = compression_of(order, Some(34713), |inner, ifd| {
                    inner.short(ifd, NEF_COMPRESSION, &[code]);
                });
                assert_eq!(read, expected, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn without_nef_compression_the_code_is_at_byte_10_of_0x0051() {
        // In the MakerNote's byte order: 13 is `0d 00` in a little-endian
        // MakerNote, as on the Z 9's HE file, and `00 0d` in a big-endian
        // one.
        for order in ORDERS {
            for (code, expected) in NEF_COMPRESSIONS {
                let read = compression_of(order, Some(34713), |inner, ifd| {
                    inner.undefined(ifd, COMPRESSION_RECORD, &compression_record(order, code));
                });
                assert_eq!(read, expected, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn byte_10_of_0x0051_is_read_in_the_makernotes_byte_order() {
        // A MakerNote whose byte order is not the file's: read in the
        // file's, 13 would be 3328.
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner.undefined(
                    ifd,
                    COMPRESSION_RECORD,
                    &compression_record(inner_order, 13),
                );
            })
            .0;
            let built = raw_file(file, &block, whole(&block), |builder, ifd0| {
                builder.short(ifd0, COMPRESSION, &[34713]);
            });
            assert_eq!(
                metadata_of(built).camera_settings.raw_compression,
                Some(RawCompression::HighEfficiency),
                "{file:?}"
            );
        }
    }

    #[test]
    fn a_0x0051_too_short_or_of_another_type_gives_no_compression() {
        for order in ORDERS {
            let record = compression_record(order, 13);
            // 12 bytes hold the code; 11 do not.
            let read = compression_of(order, Some(34713), |inner, ifd| {
                inner.undefined(ifd, COMPRESSION_RECORD, &record[..12]);
            });
            assert_eq!(read, Some(RawCompression::HighEfficiency), "{order:?}");
            let read = compression_of(order, Some(34713), |inner, ifd| {
                inner.undefined(ifd, COMPRESSION_RECORD, &record[..11]);
            });
            assert_eq!(read, None, "{order:?}, 11 bytes");
            let read = compression_of(order, Some(34713), |inner, ifd| {
                inner.byte(ifd, COMPRESSION_RECORD, &record);
            });
            assert_eq!(read, None, "{order:?}, BYTEs");
        }
    }

    #[test]
    fn nef_compression_wins_over_0x0051_even_when_it_gives_none() {
        for order in ORDERS {
            let record = compression_record(order, 13);
            let read = compression_of(order, Some(34713), |inner, ifd| {
                inner
                    .short(ifd, NEF_COMPRESSION, &[3])
                    .undefined(ifd, COMPRESSION_RECORD, &record);
            });
            assert_eq!(read, Some(RawCompression::Lossless), "{order:?}");
            // A NEFCompression of another type gives none, and 0x0051 is
            // not read in its place.
            let read = compression_of(order, Some(34713), |inner, ifd| {
                inner
                    .long(ifd, NEF_COMPRESSION, &[3])
                    .undefined(ifd, COMPRESSION_RECORD, &record);
            });
            assert_eq!(read, None, "{order:?}, a LONG");
        }
    }

    #[test]
    fn only_nikons_compressed_form_is_refined() {
        for order in ORDERS {
            let lossy = |inner: &mut TiffBuilder, ifd| {
                inner.short(ifd, NEF_COMPRESSION, &[4]);
            };
            // TIFF's 1 is uncompressed, whatever NEFCompression says.
            assert_eq!(
                compression_of(order, Some(1), lossy),
                Some(RawCompression::Uncompressed),
                "{order:?}"
            );
            // Another maker's code, or none, is not Nikon's to refine.
            for compression in [Some(7), Some(32767), Some(6), None] {
                assert_eq!(
                    compression_of(order, compression, lossy),
                    None,
                    "{order:?}, {compression:?}"
                );
            }
            // Nikon's compressed form with nothing to refine it.
            assert_eq!(
                compression_of(order, Some(34713), |_, _| {}),
                None,
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_jpeg_has_no_raw_compression() {
        // Even if the standard tags gave Nikon's compressed form, the
        // MakerNote of a JPEG's picture refines nothing: a JPEG holds no
        // raw data.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.short(ifd, NEF_COMPRESSION, &[3]);
            })
            .0;
            let (mut src, makernote, mut standard) =
                read_file(exif_file(order, &block, whole(&block), |_, _| {}));
            standard.raw_storage.compression = Some(34713);
            let read = metadata(&mut src, &makernote, standard, Picture::Jpeg).unwrap();
            assert_eq!(read.camera_settings.raw_compression, None, "{order:?}");
        }
    }

    #[test]
    fn each_quality_text_gives_its_step_in_a_jpeg() {
        // Padded with spaces, as the bodies write it, or not.
        let cases = [
            ("FINE", Some(JpegQuality::Fine)),
            ("NORMAL", Some(JpegQuality::Normal)),
            ("BASIC", Some(JpegQuality::Basic)),
            ("FINE   ", Some(JpegQuality::Fine)),
            // A JPEG a program made from a raw file, a raw file's texts,
            // and texts the table lacks.
            ("RAW", None),
            ("NRW", None),
            ("RAW2.7M", None),
            ("fine", None),
            ("FINE*", None),
        ];
        for order in ORDERS {
            for (text, expected) in cases {
                let read = jpeg_settings(
                    order,
                    |inner, ifd| {
                        inner.ascii(ifd, QUALITY, text);
                    },
                    |_, _| {},
                );
                assert_eq!(read.jpeg_quality, expected, "{order:?}, {text:?}");
            }
        }
    }

    #[test]
    fn a_raw_files_quality_gives_no_jpeg_quality() {
        // Whatever the text, even a JPEG's step: a raw file's picture is
        // its raw data.
        for order in ORDERS {
            for text in ["RAW", "FINE"] {
                let read = settings_of(order, |inner, ifd| {
                    inner.ascii(ifd, QUALITY, text);
                });
                assert_eq!(read.jpeg_quality, None, "{order:?}, {text:?}");
            }
        }
    }

    #[test]
    fn a_quality_of_another_type_gives_no_jpeg_quality() {
        // The E800, E900 and E950 write a SHORT.
        for order in ORDERS {
            let read = jpeg_settings(
                order,
                |inner, ifd| {
                    inner.short(ifd, QUALITY, &[2]);
                },
                |_, _| {},
            );
            assert_eq!(read.jpeg_quality, None, "{order:?}");
        }
    }

    #[test]
    fn each_jpg_compression_code_gives_its_compression_in_a_jpeg_alone() {
        let cases = [
            (1, Some(JpegCompression::SizePriority)),
            (3, Some(JpegCompression::OptimalQuality)),
            (0, None),
            (2, None),
            (4, None),
        ];
        for order in ORDERS {
            for (code, expected) in cases {
                let tags = |inner: &mut TiffBuilder, ifd| {
                    inner.short(ifd, JPG_COMPRESSION, &[code]);
                };
                let read = jpeg_settings(order, tags, |_, _| {});
                assert_eq!(read.jpeg_compression, expected, "{order:?}, {code}");
                // A raw file gives none, as it gives no JPEG quality.
                let read = settings_of(order, tags);
                assert_eq!(read.jpeg_compression, None, "{order:?}, {code}, raw");
            }
            let read = jpeg_settings(
                order,
                |inner, ifd| {
                    inner.long(ifd, JPG_COMPRESSION, &[3]);
                },
                |_, _| {},
            );
            assert_eq!(read.jpeg_compression, None, "{order:?}, a LONG");
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

    /// The release mode and the bracketing of a raw file in byte order
    /// `order` whose MakerNote's ShootingMode is the one SHORT `bits`, and
    /// whose Exif IFD holds what `exif` adds.
    fn drive_of(
        order: Order,
        bits: u16,
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> (Option<ReleaseMode>, Option<Bracketing>) {
        let settings = settings_with_exif(
            order,
            |inner, ifd| {
                inner.short(ifd, SHOOTING_MODE, &[bits]);
            },
            exif,
        );
        (settings.release_mode, settings.bracketing)
    }

    #[test]
    fn each_shooting_mode_bit_alone_and_together_gives_its_release_and_bracketing() {
        use Bracketing::{DynamicRange, Exposure, Off, WhiteBalance};
        use ReleaseMode::{Continuous, Remote, SelfTimer, Single};
        let cases = [
            // No bit: a single frame, as quiet release and mirror up too.
            (0, Some(Single), Some(Off)),
            (1, Some(Continuous), Some(Off)),
            (8, Some(SelfTimer), Some(Off)),
            (128, Some(Remote), Some(Off)),
            (16, Some(Single), Some(Exposure)),
            // Bits 6 and 8, which no file holds.
            (64, Some(Single), Some(WhiteBalance)),
            (256, Some(Single), Some(DynamicRange)),
            // Values files hold: continuous with exposure bracketing, the
            // self-timer with it, continuous with auto ISO (bit 5) and
            // with both; bits 1, 2 and 9 beside the modes.
            (17, Some(Continuous), Some(Exposure)),
            (24, Some(SelfTimer), Some(Exposure)),
            (33, Some(Continuous), Some(Off)),
            (49, Some(Continuous), Some(Exposure)),
            (2, Some(Single), Some(Off)),
            (4, Some(Single), Some(Off)),
            (160, Some(Remote), Some(Off)),
            (512, Some(Single), Some(Off)),
            (520, Some(SelfTimer), Some(Off)),
            (528, Some(Single), Some(Exposure)),
            // A release bit beside a bracketing bit of the documented two.
            (65, Some(Continuous), Some(WhiteBalance)),
            (264, Some(SelfTimer), Some(DynamicRange)),
        ];
        for order in ORDERS {
            for (bits, release, bracketing) in cases {
                assert_eq!(
                    drive_of(order, bits, |_, _| {}),
                    (release, bracketing),
                    "{order:?}, {bits}"
                );
            }
        }
    }

    #[test]
    fn two_release_bits_or_pre_capture_give_no_release_mode() {
        for order in ORDERS {
            // Two of bits 0, 3 and 7, and all three; pre-capture, alone and
            // beside continuous release.
            for bits in [9, 129, 136, 137, 2048, 2049] {
                let (release, bracketing) = drive_of(order, bits, |_, _| {});
                assert_eq!(release, None, "{order:?}, {bits}");
                assert_eq!(bracketing, Some(Bracketing::Off), "{order:?}, {bits}");
            }
        }
    }

    #[test]
    fn two_bracketing_bits_give_none_and_leave_the_exif_tags_bracketing() {
        for order in ORDERS {
            for bits in [80, 272, 320, 336] {
                assert_eq!(
                    drive_of(order, bits, |_, _| {}),
                    (Some(ReleaseMode::Single), None),
                    "{order:?}, {bits}"
                );
                // Exif's auto bracket stays where the MakerNote names none.
                let with_exif = drive_of(order, bits, |builder, exif| {
                    builder.short(exif, EXPOSURE_MODE, &[2]);
                });
                assert_eq!(
                    with_exif,
                    (Some(ReleaseMode::Single), Some(Bracketing::Exposure)),
                    "{order:?}, {bits}, Exif"
                );
            }
        }
    }

    #[test]
    fn the_makernotes_bracketing_wins_over_the_exif_tags() {
        // Exif's auto bracket beside a ShootingMode that sets no
        // bracketing bit, which gives off, and one that names white balance
        // bracketing: the MakerNote's word stands.
        for order in ORDERS {
            for (bits, expected) in [(0, Bracketing::Off), (64, Bracketing::WhiteBalance)] {
                let (_, bracketing) = drive_of(order, bits, |builder, exif| {
                    builder.short(exif, EXPOSURE_MODE, &[2]);
                });
                assert_eq!(bracketing, Some(expected), "{order:?}, {bits}");
            }
        }
    }

    #[test]
    fn a_shooting_mode_of_another_type_or_count_gives_neither() {
        for order in ORDERS {
            // One BYTE, as the D1 bodies write it; one LONG; two SHORTs.
            let byte = settings_of(order, |inner, ifd| {
                inner.byte(ifd, SHOOTING_MODE, &[1]);
            });
            let long = settings_of(order, |inner, ifd| {
                inner.long(ifd, SHOOTING_MODE, &[1]);
            });
            let two = settings_of(order, |inner, ifd| {
                inner.short(ifd, SHOOTING_MODE, &[1, 1]);
            });
            for (what, settings) in [("BYTE", byte), ("LONG", long), ("two", two)] {
                assert_eq!(
                    (settings.release_mode, settings.bracketing),
                    (None, None),
                    "{order:?}, {what}"
                );
            }
            // Without ShootingMode, Exif's auto bracket alone gives one.
            let exif = settings_with_exif(
                order,
                |_, _| {},
                |builder, exif| {
                    builder.short(exif, EXPOSURE_MODE, &[2]);
                },
            );
            assert_eq!(
                (exif.release_mode, exif.bracketing),
                (None, Some(Bracketing::Exposure)),
                "{order:?}"
            );
        }
    }

    /// Whether auto ISO was on, in the settings of a raw file in byte order
    /// `order` whose IFD0 holds the Model `model`, when there is one, and
    /// whose MakerNote holds what `tags` adds to its MakerNote IFD.
    fn auto_iso_of(
        order: Order,
        model: Option<&str>,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Option<bool> {
        let block = tags_block(order, tags).0;
        let count = whole(&block);
        let bytes = nikon_file_with(order, AT, &block, TAIL, |builder, ifd0, exif| {
            if let Some(model) = model {
                builder.ascii(ifd0, MODEL, model);
            }
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, count);
        });
        metadata_of(bytes).camera_settings.auto_iso
    }

    #[test]
    fn shooting_modes_bit_5_says_whether_auto_iso_was_on() {
        // Bit 5 alone, beside continuous release as the D70's file and
        // others set it (33), and beside exposure bracketing too (49);
        // clear, alone and beside other bits.
        for order in ORDERS {
            for model in [None, Some("NIKON D600"), Some("NIKON D70s")] {
                for (bits, expected) in [
                    (32, true),
                    (33, true),
                    (49, true),
                    (544, true),
                    (0, false),
                    (1, false),
                    (16, false),
                    (512, false),
                ] {
                    let read = auto_iso_of(order, model, |inner, ifd| {
                        inner.short(ifd, SHOOTING_MODE, &[bits]);
                    });
                    assert_eq!(read, Some(expected), "{order:?}, {model:?}, {bits}");
                }
            }
        }
    }

    #[test]
    fn the_d70s_bit_5_gives_no_auto_iso() {
        for order in ORDERS {
            for bits in [0, 32, 33] {
                let read = auto_iso_of(order, Some("NIKON D70"), |inner, ifd| {
                    inner.short(ifd, SHOOTING_MODE, &[bits]);
                });
                assert_eq!(read, None, "{order:?}, {bits}");
            }
        }
    }

    #[test]
    fn without_shooting_mode_the_coolpix_iso_selection_text_gives_auto_iso() {
        for order in ORDERS {
            for (text, expected) in [
                ("AUTO", Some(true)),
                ("MANUAL", Some(false)),
                ("auto", None),
                ("AUTO1", None),
            ] {
                let read = auto_iso_of(order, Some("COOLPIX P7800"), |inner, ifd| {
                    inner.ascii(ifd, ISO_SELECTION, text);
                });
                assert_eq!(read, expected, "{order:?}, {text}");
            }
            // As one SHORT, the text is not read.
            let read = auto_iso_of(order, None, |inner, ifd| {
                inner.short(ifd, ISO_SELECTION, &[1]);
            });
            assert_eq!(read, None, "{order:?}, a SHORT");
        }
    }

    #[test]
    fn a_shooting_mode_that_can_be_read_wins_over_iso_selection() {
        for order in ORDERS {
            // ShootingMode says off, ISOSelection on: ShootingMode stands.
            let read = auto_iso_of(order, None, |inner, ifd| {
                inner
                    .short(ifd, SHOOTING_MODE, &[0])
                    .ascii(ifd, ISO_SELECTION, "AUTO");
            });
            assert_eq!(read, Some(false), "{order:?}");
            // A ShootingMode of one BYTE cannot be read: the text gives it.
            let read = auto_iso_of(order, None, |inner, ifd| {
                inner
                    .byte(ifd, SHOOTING_MODE, &[0])
                    .ascii(ifd, ISO_SELECTION, "AUTO");
            });
            assert_eq!(read, Some(true), "{order:?}, BYTE");
            // On the D70, the bit gives none, and the text is not read.
            let read = auto_iso_of(order, Some("NIKON D70"), |inner, ifd| {
                inner
                    .short(ifd, SHOOTING_MODE, &[32])
                    .ascii(ifd, ISO_SELECTION, "AUTO");
            });
            assert_eq!(read, None, "{order:?}, D70");
        }
    }

    /// A VRInfo record of version `version`: byte 4 `on`, byte 5 repeating
    /// it, as the files do, byte 6 `mode`, then zeros to `len` bytes.
    fn vr_info(version: &[u8; 4], on: u8, mode: u8, len: usize) -> Vec<u8> {
        let mut record = version.to_vec();
        record.extend_from_slice(&[on, on, mode]);
        record.resize(len, 0);
        record
    }

    /// The stabilization of a raw file in byte order `order` whose
    /// MakerNote's VRInfo is `record`, of type UNDEFINED.
    fn stabilization_of(order: Order, record: &[u8]) -> Option<Stabilization> {
        settings_of(order, |inner, ifd| {
            inner.undefined(ifd, VR_INFO, record);
        })
        .stabilization
    }

    #[test]
    fn vr_info_gives_on_or_off_and_the_mode_of_its_versions_list() {
        use StabilizationMode::{Active, Normal, Sport};
        let on = Stabilization::on;
        let cases = [
            // The DSLRs' 0100 and the Nikon 1 bodies' 0101, of 8 bytes: 0
            // normal, 1 "On (1)", no mode, 2 active, 3 sport.
            (vr_info(b"0100", 1, 0, 8), on().with_mode(Normal)),
            (vr_info(b"0100", 1, 1, 8), on()),
            (vr_info(b"0100", 1, 2, 8), on().with_mode(Active)),
            (vr_info(b"0100", 1, 3, 8), on().with_mode(Sport)),
            (vr_info(b"0100", 1, 4, 8), on()),
            (vr_info(b"0101", 1, 0, 8), on().with_mode(Normal)),
            (vr_info(b"0101", 1, 2, 8), on().with_mode(Active)),
            (vr_info(b"0101", 1, 3, 8), on().with_mode(Sport)),
            // The Z bodies' 0200, of 9 bytes: 0 "Off", no mode, 1 normal, 3
            // sport, and 2, which its list lacks.
            (vr_info(b"0200", 1, 0, 9), on()),
            (vr_info(b"0200", 1, 1, 9), on().with_mode(Normal)),
            (vr_info(b"0200", 1, 2, 9), on()),
            (vr_info(b"0200", 1, 3, 9), on().with_mode(Sport)),
            // Off, whatever byte 6 holds.
            (vr_info(b"0100", 2, 0, 8), Stabilization::off()),
            (vr_info(b"0100", 2, 3, 8), Stabilization::off()),
            (vr_info(b"0200", 2, 1, 9), Stabilization::off()),
        ];
        for order in ORDERS {
            for (record, expected) in &cases {
                assert_eq!(
                    stabilization_of(order, record),
                    Some(*expected),
                    "{order:?}, {record:?}"
                );
            }
        }
    }

    #[test]
    fn vr_info_of_another_byte_4_version_or_length_is_none() {
        let records = [
            // Byte 4 zero, as four public files hold, and 3.
            vr_info(b"0100", 0, 0, 8),
            vr_info(b"0200", 3, 1, 9),
            // A record of 7 bytes, and 0100 of 9 and 0200 of 8.
            vr_info(b"0100", 1, 0, 7),
            vr_info(b"0100", 1, 0, 9),
            vr_info(b"0200", 1, 1, 8),
            // Versions no measured file holds.
            vr_info(b"0102", 1, 0, 8),
            vr_info(b"0201", 1, 1, 9),
            vr_info(b"0300", 1, 1, 9),
            b"010".to_vec(),
        ];
        for order in ORDERS {
            for record in &records {
                assert_eq!(
                    stabilization_of(order, record),
                    None,
                    "{order:?}, {record:?}"
                );
            }
            // Eight BYTEs.
            let read = settings_of(order, |inner, ifd| {
                inner.byte(ifd, VR_INFO, &vr_info(b"0100", 1, 0, 8));
            });
            assert_eq!(read.stabilization, None, "{order:?}");
        }
    }

    /// The exposure fine-tuning of a raw file in byte order `order` whose
    /// MakerNote's ExposureTuning is `bytes`, of type UNDEFINED.
    fn exposure_tuning_of(order: Order, bytes: &[u8]) -> Option<SignedRational> {
        settings_of(order, |inner, ifd| {
            inner.undefined(ifd, EXPOSURE_TUNING, bytes);
        })
        .exposure_fine_tune
    }

    #[test]
    fn exposure_tuning_gives_the_count_times_the_factor_over_the_divisor() {
        for order in ORDERS {
            for (bytes, expected) in [
                // The four public D750 files, and every other file.
                ([0xFD, 1, 6], (-3, 6)),
                ([0, 1, 6], (0, 6)),
                // The ends of the D600's menu, -1 and +1 EV; the ends of a
                // signed byte; another factor and divisor.
                ([0xFA, 1, 6], (-6, 6)),
                ([6, 1, 6], (6, 6)),
                ([0x80, 1, 6], (-128, 6)),
                ([0x7F, 1, 6], (127, 6)),
                ([3, 2, 6], (6, 6)),
                ([0xFF, 1, 3], (-1, 3)),
            ] {
                assert_eq!(
                    exposure_tuning_of(order, &bytes),
                    Some(signed_fraction(expected.0, expected.1)),
                    "{order:?}, {bytes:?}"
                );
            }
        }
    }

    #[test]
    fn exposure_tuning_with_a_zero_divisor_or_of_another_shape_is_none() {
        for order in ORDERS {
            assert_eq!(exposure_tuning_of(order, &[0xFD, 1, 0]), None, "{order:?}");
            // A zero factor would read any count as zero, so -6 is not a
            // fine-tuning of 0/6, as for the flash compensation. No file
            // holds one (#225).
            assert_eq!(exposure_tuning_of(order, &[0xFA, 0, 6]), None, "{order:?}");
            assert_eq!(exposure_tuning_of(order, &[0, 0, 6]), None, "{order:?}");
            // Four bytes, as some JPEGs a program made from a NEF hold;
            // two; three BYTEs.
            assert_eq!(
                exposure_tuning_of(order, &[0xFD, 1, 6, 0]),
                None,
                "{order:?}"
            );
            assert_eq!(exposure_tuning_of(order, &[0xFD, 1]), None, "{order:?}");
            let read = settings_of(order, |inner, ifd| {
                inner.byte(ifd, EXPOSURE_TUNING, &[0xFD, 1, 6]);
            });
            assert_eq!(read.exposure_fine_tune, None, "{order:?}");
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

    /// The flash of a raw file in byte order `order` whose Exif Flash is
    /// `exif_flash`, when `Some`, and whose MakerNote holds what `tags`
    /// adds to its MakerNote IFD.
    fn flash_of(
        order: Order,
        exif_flash: Option<u16>,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Option<Flash> {
        settings_with_exif(order, tags, |builder, exif| {
            if let Some(bits) = exif_flash {
                builder.short(exif, EXIF_FLASH, &[bits]);
            }
        })
        .flash
    }

    /// Exif's Flash 15: fired in fill mode, the return light detected, as
    /// most files whose flash fired hold.
    const FIRED: u16 = 0x0F;

    /// The flash [`FIRED`] reads as.
    fn fired() -> Flash {
        Flash::fired()
            .with_mode(FlashMode::Fill)
            .with_return_light(true)
    }

    /// The flash of a raw file in byte order `order` whose Exif Flash is
    /// [`FIRED`] and whose MakerNote's FlashType is `text`, padded with
    /// spaces to 19 characters, as Nikon writes it; the builder adds the
    /// NUL.
    fn flash_type_of(order: Order, text: &str) -> Option<Flash> {
        flash_of(order, Some(FIRED), |inner, ifd| {
            inner.ascii(ifd, FLASH_TYPE, &format!("{text:<19}"));
        })
    }

    #[test]
    fn each_flash_type_text_of_the_table_gives_its_source_and_control() {
        use FlashControl::{AutoAperture, Manual, Repeating, Ttl};
        use FlashSource::{BuiltIn, External};
        let table = [
            ("Built-in,TTL", BuiltIn, Ttl),
            ("Built-in,TTL-BL", BuiltIn, Ttl),
            ("Optional,TTL", External, Ttl),
            ("Optional,TTL-BL", External, Ttl),
            ("Optional,AA", External, AutoAperture),
            ("Optional,M", External, Manual),
            ("Optional,RPT&Comdr.", External, Repeating),
            ("Optional,TTL&Comdr.", External, Ttl),
        ];
        for order in ORDERS {
            for (text, source, control) in table {
                let expected = fired().with_source(source).with_control(control);
                assert_eq!(
                    flash_type_of(order, text),
                    Some(expected),
                    "{order:?}, {text:?}"
                );
            }
        }
    }

    #[test]
    fn a_flash_type_text_not_in_the_table_adds_nothing() {
        // The D100's NEW_TTL, which names neither part; the blank of a
        // flash that did not fire; the right letters in lower case; a
        // text no file holds, though the built-in flash offers manual
        // control, so its form is not known; a text cut short, and one
        // with more after it.
        for order in ORDERS {
            for text in [
                "NEW_TTL",
                "",
                "optional,ttl",
                "Built-in,M",
                "Optional",
                "Optional,",
                "Optional,TTL1",
                "TTL",
            ] {
                assert_eq!(
                    flash_type_of(order, text),
                    Some(fired()),
                    "{order:?}, {text:?}"
                );
            }
        }
    }

    #[test]
    fn a_flash_type_of_another_type_adds_nothing() {
        for order in ORDERS {
            for typ in [UNDEFINED, BYTE] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.ascii(ifd, FLASH_TYPE, "Optional,TTL       ");
                });
                let block = retyped(order, block, entry, typ, 20);
                let read = exif_metadata(order, &block, |builder, exif| {
                    builder.short(exif, EXIF_FLASH, &[FIRED]);
                })
                .camera_settings;
                assert_eq!(read.flash, Some(fired()), "{order:?}, type {typ}");
            }
        }
    }

    #[test]
    fn without_an_exif_flash_the_makernote_gives_no_flash() {
        // As on the D1 family's files: the MakerNote's parts have nothing
        // to belong to.
        for order in ORDERS {
            let read = flash_of(order, None, |inner, ifd| {
                inner.ascii(ifd, FLASH_SETTING, "NORMAL      ").ascii(
                    ifd,
                    FLASH_TYPE,
                    "Optional,TTL       ",
                );
            });
            assert_eq!(read, None, "{order:?}");
        }
    }

    /// The flash of a raw file in byte order `order` whose Exif Flash is
    /// `exif_flash` and whose MakerNote's FlashSetting is `text`, padded
    /// with spaces to 12 characters, as Nikon writes it; the builder adds
    /// the NUL.
    fn flash_setting_of(order: Order, exif_flash: u16, text: &str) -> Option<Flash> {
        flash_of(order, Some(exif_flash), |inner, ifd| {
            inner.ascii(ifd, FLASH_SETTING, &padded(text));
        })
    }

    #[test]
    fn each_flash_setting_text_of_the_table_gives_its_sync() {
        let table = [
            ("NORMAL", FlashSync::FrontCurtain),
            ("REAR", FlashSync::RearCurtain),
            ("SLOW", FlashSync::Slow),
            ("REAR SLOW", FlashSync::SlowRearCurtain),
        ];
        for order in ORDERS {
            for (text, sync) in table {
                assert_eq!(
                    flash_setting_of(order, FIRED, text),
                    Some(fired().with_sync(sync)),
                    "{order:?}, {text:?}"
                );
                // A flash that did not fire still records its setting, as
                // the Z 9 writes NORMAL on every file.
                assert_eq!(
                    flash_setting_of(order, 0, text),
                    Some(Flash::not_fired().with_sync(sync)),
                    "{order:?}, {text:?}, not fired"
                );
            }
        }
    }

    #[test]
    fn the_red_eye_text_gives_front_curtain_sync_with_red_eye_reduction() {
        // The Nikon 1 AW1's file: Exif's Flash 0, red-eye bit clear.
        for order in ORDERS {
            assert_eq!(
                flash_setting_of(order, 0, "RED-EYE"),
                Some(
                    Flash::not_fired()
                        .with_sync(FlashSync::FrontCurtain)
                        .with_red_eye_reduction(true)
                ),
                "{order:?}"
            );
            // Exif's bit already set: it stays on.
            assert_eq!(
                flash_setting_of(order, 0x4F, "RED-EYE"),
                Some(
                    fired()
                        .with_sync(FlashSync::FrontCurtain)
                        .with_red_eye_reduction(true)
                ),
                "{order:?}, Exif's bit set"
            );
            // Another text leaves Exif's bit as it is.
            assert_eq!(
                flash_setting_of(order, 0x4F, "NORMAL"),
                Some(
                    fired()
                        .with_sync(FlashSync::FrontCurtain)
                        .with_red_eye_reduction(true)
                ),
                "{order:?}, NORMAL beside Exif's bit"
            );
        }
    }

    /// The flash of a raw file in byte order `order` whose Exif Flash is
    /// [`FIRED`] and whose MakerNote's FlashExposureComp is `bytes`, of
    /// type UNDEFINED.
    fn flash_compensation_of(order: Order, bytes: &[u8]) -> Option<Flash> {
        flash_of(order, Some(FIRED), |inner, ifd| {
            inner.undefined(ifd, FLASH_EXPOSURE_COMP, bytes);
        })
    }

    #[test]
    fn flash_exposure_comp_gives_the_count_times_the_factor_over_the_divisor() {
        for order in ORDERS {
            for (bytes, expected) in [
                // A zero, as on most files; the D100's -6/6; the D80's
                // +6/6; the D5500's -10/6 and a D600's -2/6.
                ([0, 1, 6, 0], (0, 6)),
                ([0xFA, 1, 6, 0], (-6, 6)),
                ([6, 1, 6, 0], (6, 6)),
                ([0xF6, 1, 6, 0], (-10, 6)),
                ([0xFE, 1, 6, 0], (-2, 6)),
                // The ends of the counts seen, -18 and +6, and of a signed
                // byte; another factor and divisor.
                ([0xEE, 1, 6, 0], (-18, 6)),
                ([0x80, 1, 6, 0], (-128, 6)),
                ([0x7F, 1, 6, 0], (127, 6)),
                ([3, 2, 6, 0], (6, 6)),
                ([0xFF, 1, 3, 0], (-1, 3)),
            ] {
                assert_eq!(
                    flash_compensation_of(order, &bytes),
                    Some(fired().with_compensation(signed_fraction(expected.0, expected.1))),
                    "{order:?}, {bytes:?}"
                );
            }
        }
    }

    #[test]
    fn flash_exposure_comp_with_a_zero_divisor_or_of_another_shape_adds_nothing() {
        for order in ORDERS {
            // The Z 50's and the Z50II's zero divisor; a zero factor; a
            // fourth byte that is not zero.
            for bytes in [[0, 1, 0, 0], [0xFA, 0, 6, 0], [0, 0, 6, 0], [0xFA, 1, 6, 1]] {
                assert_eq!(
                    flash_compensation_of(order, &bytes),
                    Some(fired()),
                    "{order:?}, {bytes:?}"
                );
            }
            // Three bytes, five, and four BYTEs or ASCII characters.
            for bytes in [&[0xFA, 1, 6][..], &[0xFA, 1, 6, 0, 0]] {
                assert_eq!(
                    flash_compensation_of(order, bytes),
                    Some(fired()),
                    "{order:?}, {bytes:?}"
                );
            }
            let byte = flash_of(order, Some(FIRED), |inner, ifd| {
                inner.byte(ifd, FLASH_EXPOSURE_COMP, &[0xFA, 1, 6, 0]);
            });
            assert_eq!(byte, Some(fired()), "{order:?}, BYTE");
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, FLASH_EXPOSURE_COMP, &[0xFA, 1, 6, 0]);
            });
            let block = retyped(order, block, entry, ASCII, 4);
            let read = exif_metadata(order, &block, |builder, exif| {
                builder.short(exif, EXIF_FLASH, &[FIRED]);
            })
            .camera_settings;
            assert_eq!(read.flash, Some(fired()), "{order:?}, ASCII");
        }
    }

    #[test]
    fn a_damaged_flash_part_costs_only_itself() {
        // Every part sound, then each damaged in turn: the others stay.
        let sound = fired()
            .with_source(FlashSource::BuiltIn)
            .with_control(FlashControl::Ttl)
            .with_sync(FlashSync::Slow)
            .with_compensation(signed_fraction(-2, 6));
        for order in ORDERS {
            let read = |flash_type: &str, flash_setting: &str, compensation: [u8; 4]| {
                flash_of(order, Some(FIRED), |inner, ifd| {
                    inner
                        .ascii(ifd, FLASH_SETTING, &padded(flash_setting))
                        .ascii(ifd, FLASH_TYPE, &format!("{flash_type:<19}"))
                        .undefined(ifd, FLASH_EXPOSURE_COMP, &compensation);
                })
            };
            let good = [0xFE, 1, 6, 0];
            let bad = [0xFE, 1, 0, 0];
            assert_eq!(read("Built-in,TTL", "SLOW", good), Some(sound), "{order:?}");
            assert_eq!(
                read("Built-in,XX", "SLOW", good),
                Some(
                    fired()
                        .with_sync(FlashSync::Slow)
                        .with_compensation(signed_fraction(-2, 6))
                ),
                "{order:?}, FlashType"
            );
            assert_eq!(
                read("Built-in,TTL", "SLOWER", good),
                Some(
                    fired()
                        .with_source(FlashSource::BuiltIn)
                        .with_control(FlashControl::Ttl)
                        .with_compensation(signed_fraction(-2, 6))
                ),
                "{order:?}, FlashSetting"
            );
            assert_eq!(
                read("Built-in,TTL", "SLOW", bad),
                Some(
                    fired()
                        .with_source(FlashSource::BuiltIn)
                        .with_control(FlashControl::Ttl)
                        .with_sync(FlashSync::Slow)
                ),
                "{order:?}, FlashExposureComp"
            );
        }
    }

    #[test]
    fn a_flash_setting_text_not_in_the_table_adds_nothing() {
        // The blank of 35 public files; the right letters in lower case;
        // a text cut short, one with more after it, and two words run
        // together.
        for order in ORDERS {
            for text in ["", "normal", "REA", "NORMAL1", "REARSLOW", "SLOW REAR"] {
                assert_eq!(
                    flash_setting_of(order, FIRED, text),
                    Some(fired()),
                    "{order:?}, {text:?}"
                );
            }
            // Of another type.
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, FLASH_SETTING, &padded("REAR"));
            });
            let block = retyped(order, block, entry, UNDEFINED, 13);
            let read = exif_metadata(order, &block, |builder, exif| {
                builder.short(exif, EXIF_FLASH, &[FIRED]);
            })
            .camera_settings;
            assert_eq!(read.flash, Some(fired()), "{order:?}, UNDEFINED");
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

    // The picture style. The record's own fields are tested on its
    // reader's built records (`picture_control`); these tests check where
    // the settings take it from.

    /// A Picture Control record of version 0100 named `name`, with
    /// sharpening `sharpening` and every other value 0, as the D600 writes
    /// one.
    fn record_0100(name: &[u8], sharpening: u8) -> Vec<u8> {
        let mut record = b"0100".to_vec();
        for _ in 0..2 {
            let mut field = name.to_vec();
            field.resize(20, 0);
            record.extend(field);
        }
        record.extend([1, 0, 0, 0]);
        record.extend([
            0, 0x80, sharpening, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF,
        ]);
        record
    }

    /// The picture style's name and sharpening, if a Picture Control.
    fn style_of(settings: &CameraSettings) -> Option<(Option<&str>, Option<PictureControlValue>)> {
        match settings.picture_style()? {
            PictureStyle::PictureControl(control) => Some((control.name(), control.sharpening())),
            _ => None,
        }
    }

    fn whole_steps(count: i32) -> Option<PictureControlValue> {
        Some(PictureControlValue::Steps(signed_fraction(count, 1)))
    }

    #[test]
    fn the_picture_control_is_read_in_a_raw_file_and_a_jpeg_in_both_byte_orders() {
        for order in ORDERS {
            let tags = |inner: &mut TiffBuilder, ifd: IfdId| {
                inner.undefined(ifd, PICTURE_CONTROL, &record_0100(b"STANDARD", 0x83));
            };
            let expected = Some((Some("STANDARD"), whole_steps(3)));
            assert_eq!(style_of(&settings_of(order, tags)), expected, "{order:?}");
            let jpeg = jpeg_settings(order, tags, |_, _| {});
            assert_eq!(style_of(&jpeg), expected, "{order:?}");
        }
    }

    #[test]
    fn the_coolpix_tag_is_read_only_without_picture_control_data() {
        for order in ORDERS {
            let coolpix = settings_of(order, |inner, ifd| {
                inner.undefined(
                    ifd,
                    COOLPIX_PICTURE_CONTROL,
                    &record_0100(b"STANDARD", 0x83),
                );
            });
            assert_eq!(
                style_of(&coolpix),
                Some((Some("STANDARD"), whole_steps(3))),
                "{order:?}"
            );
            // Beside PictureControlData, the copy is not read.
            let both = settings_of(order, |inner, ifd| {
                inner
                    .undefined(ifd, PICTURE_CONTROL, &record_0100(b"VIVID", 0x84))
                    .undefined(
                        ifd,
                        COOLPIX_PICTURE_CONTROL,
                        &record_0100(b"STANDARD", 0x83),
                    );
            });
            assert_eq!(
                style_of(&both),
                Some((Some("VIVID"), whole_steps(4))),
                "{order:?}"
            );
            // A PictureControlData of another type gives none, and does not
            // give way to the copy.
            let retyped = settings_of(order, |inner, ifd| {
                inner
                    .byte(ifd, PICTURE_CONTROL, &record_0100(b"VIVID", 0x84))
                    .undefined(
                        ifd,
                        COOLPIX_PICTURE_CONTROL,
                        &record_0100(b"STANDARD", 0x83),
                    );
            });
            assert_eq!(retyped.picture_style(), None, "{order:?}");
            // An unrelated entry the IFD skips as damaged, of a type no
            // reader knows, does not stop the copy from being read.
            let damaged = settings_of(order, |inner, ifd| {
                inner
                    .raw(ifd, 0x0099, 99, 1, RawValue::Inline([0; 4]))
                    .undefined(
                        ifd,
                        COOLPIX_PICTURE_CONTROL,
                        &record_0100(b"STANDARD", 0x83),
                    );
            });
            assert_eq!(
                style_of(&damaged),
                Some((Some("STANDARD"), whole_steps(3))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_picture_control_record_that_cannot_be_read_gives_no_style_and_costs_nothing_else() {
        for order in ORDERS {
            let mut short = record_0100(b"STANDARD", 0x83);
            short.pop();
            let settings = settings_of(order, |inner, ifd| {
                inner.undefined(ifd, PICTURE_CONTROL, &short).ascii(
                    ifd,
                    WHITE_BALANCE,
                    &padded("SUNNY"),
                );
            });
            assert_eq!(settings.picture_style(), None, "{order:?}");
            assert_eq!(
                settings.white_balance.mode,
                Some(WhiteBalanceMode::Daylight),
                "{order:?}"
            );
        }
    }

    // Active D-Lighting.

    /// Each code of ActiveD-Lighting's list with the step it names, and
    /// codes the list lacks.
    const ACTIVE_D_LIGHTINGS: [(u16, Option<ActiveDLighting>); 15] = [
        (0, Some(ActiveDLighting::Off)),
        (1, Some(ActiveDLighting::Low)),
        (3, Some(ActiveDLighting::Normal)),
        (5, Some(ActiveDLighting::High)),
        (7, Some(ActiveDLighting::ExtraHigh)),
        (8, Some(ActiveDLighting::ExtraHigh1)),
        (9, Some(ActiveDLighting::ExtraHigh2)),
        (10, Some(ActiveDLighting::ExtraHigh3)),
        (11, Some(ActiveDLighting::ExtraHigh4)),
        (65535, Some(ActiveDLighting::Auto)),
        (2, None),
        (4, None),
        (6, None),
        (12, None),
        (65534, None),
    ];

    #[test]
    fn each_active_d_lighting_code_gives_its_step_in_a_raw_file_and_a_jpeg() {
        for order in ORDERS {
            for (code, step) in ACTIVE_D_LIGHTINGS {
                let tags = |inner: &mut TiffBuilder, ifd: IfdId| {
                    inner.short(ifd, ACTIVE_D_LIGHTING, &[code]);
                };
                let expected = step.map(DynamicRangeOptimization::ActiveDLighting);
                let raw = settings_of(order, tags);
                assert_eq!(
                    raw.dynamic_range_optimization, expected,
                    "{order:?}, {code}"
                );
                let jpeg = jpeg_settings(order, tags, |_, _| {});
                assert_eq!(
                    jpeg.dynamic_range_optimization, expected,
                    "{order:?}, {code}"
                );
            }
        }
    }

    #[test]
    fn an_active_d_lighting_of_another_shape_gives_none_and_costs_nothing_else() {
        for order in ORDERS {
            // One LONG, two SHORTs, and two UNDEFINED bytes.
            for place in 0..3 {
                let settings = settings_of(order, |inner, ifd| {
                    match place {
                        0 => inner.long(ifd, ACTIVE_D_LIGHTING, &[3]),
                        1 => inner.short(ifd, ACTIVE_D_LIGHTING, &[3, 3]),
                        _ => inner.undefined(ifd, ACTIVE_D_LIGHTING, &[3, 0]),
                    };
                    inner.ascii(ifd, WHITE_BALANCE, &padded("SUNNY"));
                });
                assert_eq!(
                    settings.dynamic_range_optimization, None,
                    "{order:?}, {place}"
                );
                assert_eq!(
                    settings.white_balance.mode,
                    Some(WhiteBalanceMode::Daylight),
                    "{order:?}, {place}"
                );
            }
        }
    }

    // HDR.

    /// An HDRInfo record of version `version`, `len` bytes long, whose
    /// bytes 4 to 6 are `on`, `fifth` and `sixth`, padded with zeros.
    fn hdr_info(version: &[u8; 4], on: u8, fifth: u8, sixth: u8, len: usize) -> Vec<u8> {
        let mut record = version.to_vec();
        record.extend_from_slice(&[on, fifth, sixth]);
        record.resize(len, 0);
        record
    }

    /// The high dynamic range setting of a raw file and a JPEG in byte
    /// order `order` whose MakerNote's HDRInfo is `record`, of type
    /// UNDEFINED, checked to be the same in both.
    fn hdr_of(order: Order, record: &[u8]) -> Option<HighDynamicRange> {
        let tags = |inner: &mut TiffBuilder, ifd: IfdId| {
            inner.undefined(ifd, HDR_INFO, record);
        };
        let raw = settings_of(order, tags).high_dynamic_range;
        let jpeg = jpeg_settings(order, tags, |_, _| {}).high_dynamic_range;
        assert_eq!(raw, jpeg, "{order:?}, {record:?}");
        raw
    }

    fn ev(numerator: u32) -> HdrExposureDifferential {
        HdrExposureDifferential::Ev(Rational {
            numerator,
            denominator: 1,
        })
    }

    #[test]
    fn hdr_info_0200_gives_on_or_off_and_the_strength_where_on() {
        use HdrStrength::{Auto, ExtraHigh, High, Low, Normal};
        let on = Hdr::on;
        let cases = [
            // My D7100's four JPEGs under HDR, `01 05`, and the rest of the
            // list; 0, "not available", and 6, which it lacks, name none.
            (hdr_info(b"0200", 1, 5, 0, 6), on().with_strength(Auto)),
            (hdr_info(b"0200", 1, 1, 0, 6), on().with_strength(Normal)),
            (hdr_info(b"0200", 1, 2, 0, 6), on().with_strength(Low)),
            (hdr_info(b"0200", 1, 3, 0, 6), on().with_strength(High)),
            (hdr_info(b"0200", 1, 4, 0, 6), on().with_strength(ExtraHigh)),
            (hdr_info(b"0200", 1, 0, 0, 6), on()),
            (hdr_info(b"0200", 1, 6, 0, 6), on()),
            // Off, whatever byte 5 holds.
            (hdr_info(b"0200", 0, 0, 0, 6), Hdr::off()),
            (hdr_info(b"0200", 0, 5, 0, 6), Hdr::off()),
        ];
        for order in ORDERS {
            for (record, expected) in &cases {
                assert_eq!(
                    hdr_of(order, record),
                    Some(HighDynamicRange::Hdr(*expected)),
                    "{order:?}, {record:?}"
                );
            }
        }
    }

    #[test]
    fn hdr_info_0100_and_0101_give_the_exposure_differential_and_smoothing_where_on() {
        use HdrSmoothing::{Auto, High, Low, Normal, Off};
        let on = Hdr::on;
        for version in [b"0100", b"0101"] {
            let cases = [
                (
                    hdr_info(version, 1, 0, 48, 8),
                    on().with_exposure_differential(HdrExposureDifferential::Auto)
                        .with_smoothing(Auto),
                ),
                (
                    hdr_info(version, 1, 1, 0, 8),
                    on().with_exposure_differential(ev(1)).with_smoothing(Off),
                ),
                (
                    hdr_info(version, 1, 2, 1, 8),
                    on().with_exposure_differential(ev(2))
                        .with_smoothing(Normal),
                ),
                (
                    hdr_info(version, 1, 3, 2, 8),
                    on().with_exposure_differential(ev(3)).with_smoothing(Low),
                ),
                (
                    hdr_info(version, 1, 3, 3, 8),
                    on().with_exposure_differential(ev(3)).with_smoothing(High),
                ),
                // A code a list lacks costs its part alone.
                (hdr_info(version, 1, 4, 3, 8), on().with_smoothing(High)),
                (
                    hdr_info(version, 1, 2, 4, 8),
                    on().with_exposure_differential(ev(2)),
                ),
                // Off, as on my D600's JPEGs, `00 00 00 FF` after the
                // version, and on the public files: no parts.
                ([version.as_slice(), &[0, 0, 0, 0xFF]].concat(), Hdr::off()),
                (hdr_info(version, 0, 2, 1, 8), Hdr::off()),
            ];
            for order in ORDERS {
                for (record, expected) in &cases {
                    assert_eq!(
                        hdr_of(order, record),
                        Some(HighDynamicRange::Hdr(*expected)),
                        "{order:?}, {record:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn hdr_info_of_another_byte_4_version_or_length_is_none() {
        let records = [
            // Byte 4's 48, "auto", which says neither on nor off, and 2.
            hdr_info(b"0100", 48, 0, 0, 8),
            hdr_info(b"0101", 48, 0, 0, 8),
            hdr_info(b"0200", 2, 5, 0, 6),
            // One byte short and one too many, at each version.
            hdr_info(b"0100", 0, 0, 0, 7),
            hdr_info(b"0101", 0, 0, 0, 9),
            hdr_info(b"0200", 1, 5, 0, 5),
            hdr_info(b"0200", 1, 5, 0, 7),
            // The other versions' lengths swapped.
            hdr_info(b"0100", 1, 2, 1, 6),
            hdr_info(b"0200", 1, 5, 0, 8),
            // Versions no measured file holds, and eight bytes with no
            // version, as a COOLPIX P7100's.
            hdr_info(b"0102", 1, 2, 1, 8),
            hdr_info(b"0201", 1, 5, 0, 6),
            hdr_info(b"0300", 1, 5, 0, 6),
            vec![0, 0, 0, 0, 0, 0, 0, 0],
            b"020".to_vec(),
        ];
        for order in ORDERS {
            for record in &records {
                assert_eq!(hdr_of(order, record), None, "{order:?}, {record:?}");
            }
        }
    }

    #[test]
    fn an_hdr_info_of_another_type_gives_none_and_costs_nothing_else() {
        for order in ORDERS {
            let settings = settings_of(order, |inner, ifd| {
                inner
                    .byte(ifd, HDR_INFO, &hdr_info(b"0200", 1, 5, 0, 6))
                    .ascii(ifd, WHITE_BALANCE, &padded("SUNNY"));
            });
            assert_eq!(settings.high_dynamic_range, None, "{order:?}");
            assert_eq!(
                settings.white_balance.mode,
                Some(WhiteBalanceMode::Daylight),
                "{order:?}"
            );
        }
    }

    // The high ISO noise reduction.

    /// Each code of HighISONoiseReduction's list with the step it names,
    /// and codes the list lacks.
    const HIGH_ISO_NOISE_REDUCTIONS: [(u16, Option<HighIsoNoiseReduction>); 10] = [
        (0, Some(HighIsoNoiseReduction::Off)),
        (1, Some(HighIsoNoiseReduction::Minimal)),
        (2, Some(HighIsoNoiseReduction::Low)),
        (3, Some(HighIsoNoiseReduction::MediumLow)),
        (4, Some(HighIsoNoiseReduction::Normal)),
        (5, Some(HighIsoNoiseReduction::MediumHigh)),
        (6, Some(HighIsoNoiseReduction::High)),
        (7, None),
        (256, None),
        (65535, None),
    ];

    #[test]
    fn each_high_iso_noise_reduction_code_gives_its_step_in_a_raw_file_and_a_jpeg() {
        for order in ORDERS {
            for (code, expected) in HIGH_ISO_NOISE_REDUCTIONS {
                let tags = |inner: &mut TiffBuilder, ifd: IfdId| {
                    inner.short(ifd, HIGH_ISO_NOISE_REDUCTION, &[code]);
                };
                let raw = settings_of(order, tags);
                assert_eq!(raw.high_iso_noise_reduction, expected, "{order:?}, {code}");
                let jpeg = jpeg_settings(order, tags, |_, _| {});
                assert_eq!(jpeg.high_iso_noise_reduction, expected, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn a_high_iso_noise_reduction_of_another_shape_gives_none_and_costs_nothing_else() {
        for order in ORDERS {
            // One LONG, two SHORTs, and two UNDEFINED bytes.
            for place in 0..3 {
                let settings = settings_of(order, |inner, ifd| {
                    match place {
                        0 => inner.long(ifd, HIGH_ISO_NOISE_REDUCTION, &[4]),
                        1 => inner.short(ifd, HIGH_ISO_NOISE_REDUCTION, &[4, 4]),
                        _ => inner.undefined(ifd, HIGH_ISO_NOISE_REDUCTION, &[4, 0]),
                    };
                    inner.ascii(ifd, WHITE_BALANCE, &padded("SUNNY"));
                });
                assert_eq!(
                    settings.high_iso_noise_reduction, None,
                    "{order:?}, {place}"
                );
                assert_eq!(
                    settings.white_balance.mode,
                    Some(WhiteBalanceMode::Daylight),
                    "{order:?}, {place}"
                );
            }
        }
    }

    // The long exposure noise reduction.

    #[test]
    fn fpnr_gives_the_long_exposure_noise_reduction_on_and_every_other_text_none() {
        // `OFF` says the reduction did not run on this picture, not that it
        // was set off, so a Nikon file never gives `Some(false)`.
        let cases = [
            ("FPNR", Some(true)),
            ("OFF", None),
            ("fpnr", None),
            ("ON", None),
            ("FPNR1", None),
            ("", None),
        ];
        for order in ORDERS {
            for (text, expected) in cases {
                let tags = |inner: &mut TiffBuilder, ifd: IfdId| {
                    inner.ascii(ifd, NOISE_REDUCTION, text);
                };
                let raw = settings_of(order, tags);
                assert_eq!(
                    raw.long_exposure_noise_reduction, expected,
                    "{order:?}, {text:?}"
                );
                let jpeg = jpeg_settings(order, tags, |_, _| {});
                assert_eq!(
                    jpeg.long_exposure_noise_reduction, expected,
                    "{order:?}, {text:?}"
                );
            }
        }
    }

    #[test]
    fn a_noise_reduction_text_of_another_type_gives_none_and_costs_nothing_else() {
        for order in ORDERS {
            let settings = settings_of(order, |inner, ifd| {
                inner.undefined(ifd, NOISE_REDUCTION, b"FPNR\0").ascii(
                    ifd,
                    WHITE_BALANCE,
                    &padded("SUNNY"),
                );
            });
            assert_eq!(settings.long_exposure_noise_reduction, None, "{order:?}");
            assert_eq!(settings.multi_frame_noise_reduction, None, "{order:?}");
            assert_eq!(
                settings.white_balance.mode,
                Some(WhiteBalanceMode::Daylight),
                "{order:?}"
            );
        }
    }
}
