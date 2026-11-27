// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote reader: the small preview JPEG the MakerNote keeps,
//! and what it records of the picture that the standard tags lack.
//!
//! The MakerNote is read once per file, into a [`MakerNote`]: its block
//! found and checked, its MakerNote IFD and its PreviewIFD parsed. Every
//! use of the MakerNote then reads from that structure, and none finds,
//! checks or parses the block again. The parts, in the order a file is
//! read:
//!
//! - [`block`] reads the block in its layouts, the MakerNote IFD and the
//!   PreviewIFD, and gives the PreviewIFD's JPEG as a candidate preview.
//!   It holds the Nikon reader's side of the MakerNote readers' trait
//!   ([`Reader`](super::Reader)), whose `metadata` calls the function
//!   below;
//! - [`metadata`](fn@metadata) reads what the MakerNote adds to the
//!   standard tags, with the four files below;
//! - [`metadata`](mod@metadata) reads CropArea, the serial number, the ISO,
//!   the lens's range and WorldTime;
//! - [`settings`] reads the camera settings;
//! - [`af_info2`] reads AFInfo2: the AF-area mode it records, the focus
//!   point and the points it names;
//! - [`crop`] reads CropHiSpeed, and places a focus point on a crop or
//!   from the body's table;
//! - [`add_entries`](super::Reader::add_entries) lists both IFDs' entries
//!   in the raw view, as the directories `MakerNote` and
//!   `MakerNoteSub(0x0011)`.
//!
//! A tag's number and what it records are on its constant, such as
//! [`CROP_AREA`]. The type and count Nikon writes it in are its rule
//! ([`rules`]), and a value of another type or count is never read.
//! Nothing the block holds is an error: a tag that is missing or breaks
//! its rule gives no value.
//!
//! The NEF notes say what each tag records, and on which bodies
//! (`docs/formats/nef/makernote-and-metadata.md`, sections 1 and 8).

mod af_info2;
mod block;
mod crop;
mod metadata;
mod picture_control;
mod settings;
#[cfg(test)]
mod testing;

pub(in crate::formats) use block::MakerNote;
#[cfg(test)]
pub(in crate::formats) use block::read;

use std::io::{Read, Seek};

use crate::camera_settings::FocusMode;
use crate::error::Result;
use crate::focus::FocusPoints;
use crate::formats::makernote::Picture;
use crate::formats::makernote::points::{self, PointTable};
use crate::formats::tiff_metadata::Standard;
use crate::geometry::Size;
use crate::io::Source;
use crate::metadata::Metadata;
use af_info2::af_info2_record;
use crop::{CropHiSpeed, crop_hi_speed, in_crop, placed, placed_layout};
use metadata::{CropArea, crop_area, iso, lens_range, serial_number, world_time};
use settings::{FromStandardTags, camera_settings};

/// The Nikon MakerNote tag that records the ISO the picture was taken at
/// (ISO): two SHORTs, the second of them the ISO.
const ISO: u16 = 0x0002;

/// The Nikon MakerNote tag that records the image quality (Quality), as
/// ASCII text: in a camera JPEG its quality step, such as `FINE`, and in a
/// raw file `RAW`, or `NRW` on a Coolpix.
const QUALITY: u16 = 0x0004;

/// The Nikon MakerNote tag that records the white balance setting
/// (WhiteBalance), as ASCII text. It is an upper-case word padded with
/// spaces, such as `AUTO1` or `SUNNY`, or a chosen color temperature as
/// its number, such as `3230K`.
const WHITE_BALANCE: u16 = 0x0005;

/// The Nikon MakerNote tag that records how the camera focused
/// (FocusMode), as ASCII text: `MANUAL` for manual focus, or an autofocus
/// mode padded with spaces to six characters, such as `AF-S  `. Read once,
/// as a camera setting, which also says whether the shot is in manual
/// focus.
const FOCUS_MODE: u16 = 0x0007;

/// The Nikon MakerNote tag that records the flash sync mode
/// (FlashSetting), as ASCII text, such as `NORMAL` for front-curtain sync
/// or `REAR SLOW`, padded with spaces. Blank on some bodies.
const FLASH_SETTING: u16 = 0x0008;

/// The Nikon MakerNote tag that records which flash fired and how its
/// output was controlled (FlashType), as ASCII text: the source, then a
/// comma and the control, such as `Built-in,TTL` or `Optional,M`. Blank
/// when no flash fired.
const FLASH_TYPE: u16 = 0x0009;

/// The Nikon MakerNote tag that records the white balance fine-tuning in
/// whole steps (WhiteBalanceFineTune): two SSHORTs, amber to blue then
/// magenta to green. The oldest bodies write one, whose axis and sign no
/// file shows.
const FINE_TUNE_STEPS: u16 = 0x000B;

/// The Nikon MakerNote tag that records the ISO setting on the Coolpix
/// bodies (ISOSelection), as ASCII text: `AUTO` for auto ISO, `MANUAL`
/// for a chosen ISO. The other bodies record auto ISO in ShootingMode.
const ISO_SELECTION: u16 = 0x000F;

/// The Nikon MakerNote tag that points at the PreviewIFD, the sub-IFD
/// holding the small preview JPEG.
const PREVIEW_IFD: u16 = 0x0011;

/// The Nikon MakerNote tag that records the flash compensation the
/// photographer set (FlashExposureComp): four UNDEFINED bytes, a signed
/// count, then a factor and a divisor, 1 and 6 on every file but the
/// Z 50's and the Z50II's, and a zero, so that `fa 01 06 00` is -6/6 EV.
const FLASH_EXPOSURE_COMP: u16 = 0x0012;

/// The Nikon MakerNote tag that records the crop mode the picture was
/// taken in, and how large the picture was before and after the crop
/// (CropHiSpeed). It holds seven SHORTs: the mode, the original width and
/// height, the cropped width and height, and the left column and top row
/// where the crop starts.
const CROP_HI_SPEED: u16 = 0x001B;

/// The Nikon MakerNote tag that records the photographer's fine-tuning of
/// the camera's metering (ExposureTuning): three UNDEFINED bytes, a signed
/// count, then a factor and a divisor, 1 and 6 on every file, so that
/// `fd 01 06` is -3/6 EV.
const EXPOSURE_TUNING: u16 = 0x001C;

/// The Nikon MakerNote tag that records the camera body's serial number
/// (SerialNumber), as ASCII text.
const SERIAL_NUMBER: u16 = 0x001D;

/// The Nikon MakerNote tag that records the color space the camera was
/// set to for its JPEG output (ColorSpace): one SHORT, 1 sRGB, 2 Adobe RGB.
const COLOR_SPACE: u16 = 0x001E;

/// The Nikon MakerNote tag that records the vibration reduction, Nikon's
/// image stabilization (VRInfo): an UNDEFINED value that starts with its
/// version, four ASCII digits. Its fifth byte says whether it was on, and
/// its seventh byte its mode, in a list of codes each version sets.
const VR_INFO: u16 = 0x001F;

/// The Nikon MakerNote tag that records Active D-Lighting
/// (ActiveD-Lighting): one SHORT, a code for its step, such as 0 off, 3
/// normal and 65535 auto.
const ACTIVE_D_LIGHTING: u16 = 0x0022;

/// The Nikon MakerNote tag that records the Picture Control, Nikon's
/// picture style (PictureControlData): an UNDEFINED value that starts with
/// its version, four ASCII digits, then the control's name and base and
/// its values, at offsets each version's layout sets
/// ([`picture_control`]).
const PICTURE_CONTROL: u16 = 0x0023;

/// The Nikon MakerNote tag that records the time zone and daylight-saving
/// setting of the camera's clock (WorldTime): four UNDEFINED bytes. The
/// first two are the zone in minutes east of UTC as a signed 16-bit number
/// in the MakerNote's byte order. Then comes 1 when daylight saving time is
/// on and 0 when it is off, then the date display format.
const WORLD_TIME: u16 = 0x0024;

/// The Nikon MakerNote tag that records which shutter took the picture
/// (ShutterMode): one SHORT, a code for the shutter, and on an auto
/// setting for the shutter the camera chose.
const SHUTTER_MODE: u16 = 0x0034;

/// The Nikon MakerNote tag that records the HDR setting (HDRInfo): an
/// UNDEFINED value that starts with its version, four ASCII digits. Its
/// fifth byte says whether HDR was on; the bytes after it hold the parts
/// its version sets, the strength or the exposure differential and the
/// smoothing.
const HDR_INFO: u16 = 0x0035;

/// The Nikon MakerNote tag that records the raw size the camera was set to
/// (ImageSizeRAW): one SHORT, 1 large, 2 medium, 3 small. In a JPEG it
/// follows the JPEG's own size instead.
const IMAGE_SIZE_RAW: u16 = 0x003E;

/// The Nikon MakerNote tag that records the white balance fine-tuning in
/// the camera's own steps (WhiteBalanceFineTune): two SRATIONALs, amber to
/// blue then magenta to green.
const FINE_TUNE: u16 = 0x003F;

/// The Nikon MakerNote tag that records how the JPEG compression was set
/// (JPGCompression): one SHORT, 1 for size priority and 3 for optimal
/// quality. Every raw file that records it holds 0.
const JPG_COMPRESSION: u16 = 0x0044;

/// The Nikon MakerNote tag that records the visible image, the rectangle
/// of the stored raw data inside its border (CropArea): its left column,
/// top row, width and height.
const CROP_AREA: u16 = 0x0045;

/// The Nikon MakerNote tag that records the color temperature of the
/// white balance the camera applied, in kelvin (ColorTemperatureAuto): one
/// SHORT, 0 when none was recorded.
const COLOR_TEMPERATURE: u16 = 0x004F;

/// The Nikon MakerNote tag the Z bodies but the Z fc and the Z 30 write in
/// place of NEFCompression, which ExifTool's documentation calls
/// FirmwareVersion51: an UNDEFINED value of 24 bytes that starts with eight
/// ASCII digits. Its bytes 10 and 11 hold NEFCompression's code, as a
/// 16-bit number in the MakerNote's byte order; what the rest records is
/// not known.
const COMPRESSION_RECORD: u16 = 0x0051;

/// The Nikon MakerNote tag that records the lens's range (Lens): its
/// shortest and longest focal length and the f-number at each, as four
/// RATIONALs in the order of the Exif LensSpecification.
const LENS: u16 = 0x0084;

/// The Nikon MakerNote tag that records the AF-area mode, the point the
/// camera focused on and the points in focus on the bodies older than
/// AFInfo2 (AFInfo): four UNDEFINED bytes, the area mode first. Only the
/// area mode is read: the point is a number, which needs a table of the
/// body's points.
const AF_INFO: u16 = 0x0088;

/// The Nikon MakerNote tag that records how the shutter was released and
/// more of the drive (ShootingMode): one SHORT of bits. Bit 0 is
/// continuous release, 3 the self-timer and 7 an infrared remote; bits 4,
/// 6 and 8 are exposure, white balance and Active D-Lighting bracketing;
/// bit 5 is auto ISO, on every body but the D70. Bits 1 and 2, the
/// exposure delay mode and a release from a computer, bit 9, which no
/// documentation names, and bit 11, pre-capture, are not read.
const SHOOTING_MODE: u16 = 0x0089;

/// The Nikon MakerNote tag that records how the raw data is compressed
/// (NEFCompression): one SHORT, a code of Nikon's own. The bodies before
/// the Z bodies write it, and the Z fc and the Z 30; the other Z bodies
/// write the same code in [`COMPRESSION_RECORD`].
const NEF_COMPRESSION: u16 = 0x0093;

/// The Nikon MakerNote tag that says whether the long exposure noise
/// reduction ran on the picture (NoiseReduction): ASCII text, `FPNR`
/// where it ran and `OFF` where it did not.
const NOISE_REDUCTION: u16 = 0x0095;

/// The Nikon MakerNote tag that records the high ISO noise reduction
/// (HighISONoiseReduction): one SHORT, a code for its step, such as 0
/// off and 4 normal.
const HIGH_ISO_NOISE_REDUCTION: u16 = 0x00B1;

/// The Nikon MakerNote tag that records where the camera's autofocus
/// aimed (AFInfo2): an UNDEFINED value that starts with its version, four
/// ASCII digits. At offsets each version sets, it holds the size of a
/// frame, the center of the focus area and the area's width and height,
/// as SHORTs in the MakerNote's byte order. Most versions add a byte that
/// says whether focus was confirmed.
const AF_INFO2: u16 = 0x00B7;

/// The Nikon MakerNote tag that records AF fine-tune (AFTune): four
/// UNDEFINED bytes, whether it is on, the index of the saved lens, the
/// value, signed, and the value at a zoom's long end, signed.
const AF_TUNE: u16 = 0x00B9;

/// The Nikon MakerNote tag under which the Coolpix bodies record the
/// Picture Control, in the same record as [`PICTURE_CONTROL`], which they
/// do not write. No measured file holds both.
const COOLPIX_PICTURE_CONTROL: u16 = 0x00BD;

/// The rules of the tags this reader reads: each tag's number, from the
/// constants above, and the type and count Nikon writes its value in on
/// every file checked (ADR011). [`Ifd::get`] and [`Ifd::held`] read them.
/// Two are tags of the PreviewIFD, which Nikon writes as one LONG each.
///
/// They are the rules of the embedded and headerless layouts' tags. The
/// short-header layout's tags have none: they are numbered in a scheme of
/// their own, and the files confirm none of their values (the NEF notes,
/// section 1).
///
/// [`Ifd::get`]: crate::container::tiff::Ifd::get
/// [`Ifd::held`]: crate::container::tiff::Ifd::held
mod rules {
    use crate::container::tiff::tags;
    use crate::container::tiff::{FieldType, Held, Rule, Text};
    use crate::rational::{Rational, SignedRational};

    const SHORT: &[FieldType] = &[FieldType::Short];
    const LONG: &[FieldType] = &[FieldType::Long];
    const ASCII: &[FieldType] = &[FieldType::Ascii];

    pub(super) const ISO: Held<[u32; 2]> = Held::new(super::ISO, SHORT);
    pub(super) const QUALITY: Rule<Text> = Rule::new(super::QUALITY, ASCII);
    pub(super) const WHITE_BALANCE: Rule<Text> = Rule::new(super::WHITE_BALANCE, ASCII);
    pub(super) const FOCUS_MODE: Rule<Text> = Rule::new(super::FOCUS_MODE, ASCII);
    pub(super) const FLASH_SETTING: Rule<Text> = Rule::new(super::FLASH_SETTING, ASCII);
    pub(super) const FLASH_TYPE: Rule<Text> = Rule::new(super::FLASH_TYPE, ASCII);
    pub(super) const FINE_TUNE_STEPS: Held<[i16; 2]> =
        Held::new(super::FINE_TUNE_STEPS, &[FieldType::SShort]);
    pub(super) const ISO_SELECTION: Rule<Text> = Rule::new(super::ISO_SELECTION, ASCII);
    pub(super) const PREVIEW_IFD: Held<[u32; 1]> = Held::new(super::PREVIEW_IFD, LONG);
    pub(super) const FLASH_EXPOSURE_COMP: Held<[u8; 4]> =
        Held::new(super::FLASH_EXPOSURE_COMP, &[FieldType::Undefined]);
    pub(super) const CROP_HI_SPEED: Rule<[u32; 7]> = Rule::new(super::CROP_HI_SPEED, SHORT);
    pub(super) const EXPOSURE_TUNING: Held<[u8; 3]> =
        Held::new(super::EXPOSURE_TUNING, &[FieldType::Undefined]);
    pub(super) const SERIAL_NUMBER: Rule<Text> = Rule::new(super::SERIAL_NUMBER, ASCII);
    pub(super) const COLOR_SPACE: Held<[u32; 1]> = Held::new(super::COLOR_SPACE, SHORT);
    pub(super) const VR_INFO: Rule<Vec<u8>> = Rule::new(super::VR_INFO, &[FieldType::Undefined]);
    pub(super) const ACTIVE_D_LIGHTING: Held<[u32; 1]> = Held::new(super::ACTIVE_D_LIGHTING, SHORT);
    pub(super) const PICTURE_CONTROL: Rule<Vec<u8>> =
        Rule::new(super::PICTURE_CONTROL, &[FieldType::Undefined]);
    pub(super) const WORLD_TIME: Held<[u8; 4]> =
        Held::new(super::WORLD_TIME, &[FieldType::Undefined]);
    pub(super) const SHUTTER_MODE: Held<[u32; 1]> = Held::new(super::SHUTTER_MODE, SHORT);
    pub(super) const HDR_INFO: Rule<Vec<u8>> = Rule::new(super::HDR_INFO, &[FieldType::Undefined]);
    pub(super) const IMAGE_SIZE_RAW: Held<[u32; 1]> = Held::new(super::IMAGE_SIZE_RAW, SHORT);
    pub(super) const JPG_COMPRESSION: Held<[u32; 1]> = Held::new(super::JPG_COMPRESSION, SHORT);
    pub(super) const FINE_TUNE: Rule<[SignedRational; 2]> =
        Rule::new(super::FINE_TUNE, &[FieldType::SRational]);
    pub(super) const CROP_AREA: Rule<[u32; 4]> =
        Rule::new(super::CROP_AREA, &[FieldType::Short, FieldType::Long]);
    pub(super) const COLOR_TEMPERATURE: Held<[u32; 1]> = Held::new(super::COLOR_TEMPERATURE, SHORT);
    pub(super) const LENS: Rule<[Rational; 4]> = Rule::new(super::LENS, &[FieldType::Rational]);
    pub(super) const AF_INFO: Held<[u8; 4]> = Held::new(super::AF_INFO, &[FieldType::Undefined]);
    pub(super) const SHOOTING_MODE: Held<[u32; 1]> = Held::new(super::SHOOTING_MODE, SHORT);
    pub(super) const AF_TUNE: Held<[u8; 4]> = Held::new(super::AF_TUNE, &[FieldType::Undefined]);
    pub(super) const COOLPIX_PICTURE_CONTROL: Rule<Vec<u8>> =
        Rule::new(super::COOLPIX_PICTURE_CONTROL, &[FieldType::Undefined]);
    pub(super) const HIGH_ISO_NOISE_REDUCTION: Held<[u32; 1]> =
        Held::new(super::HIGH_ISO_NOISE_REDUCTION, SHORT);
    pub(super) const AF_INFO2: Rule<Vec<u8>> = Rule::new(super::AF_INFO2, &[FieldType::Undefined]);
    pub(super) const NEF_COMPRESSION: Held<[u32; 1]> = Held::new(super::NEF_COMPRESSION, SHORT);
    pub(super) const NOISE_REDUCTION: Rule<Text> = Rule::new(super::NOISE_REDUCTION, ASCII);
    pub(super) const COMPRESSION_RECORD: Rule<Vec<u8>> =
        Rule::new(super::COMPRESSION_RECORD, &[FieldType::Undefined]);

    // The PreviewIFD.
    pub(super) const JPEG_INTERCHANGE_FORMAT: Held<[u32; 1]> =
        Held::new(tags::JPEG_INTERCHANGE_FORMAT, LONG);
    pub(super) const JPEG_INTERCHANGE_FORMAT_LENGTH: Held<[u32; 1]> =
        Held::new(tags::JPEG_INTERCHANGE_FORMAT_LENGTH, LONG);
}

/// The metadata of the file whose MakerNote is `makernote`: what the
/// standard tags gave, `standard`, with what the MakerNote records that
/// they lack. `picture` says what kind of picture the file holds, and so
/// what a crop without a CropArea is placed on: nothing for a raw file,
/// the image size for a JPEG. The Nikon reader's
/// [`Reader::metadata`](crate::formats::makernote::Reader::metadata).
///
/// In this order:
///
/// - **The image size**, from CropArea, which replaces the raw size the
///   image size otherwise falls back on ([`crop_area`]). Only when the raw
///   IFD records no default crop, which comes first, and when there is a
///   raw size for the crop area to lie inside. A JPEG's reader passes no
///   raw size, so a JPEG keeps the image size `standard` gives, its
///   picture's.
/// - **The serial number, the ISO, the lens's range and the offset from
///   UTC**, each only when the standard tags gave none ([`serial_number`],
///   [`iso`], [`lens_range`], [`world_time`]). A range fills a lens whose
///   standard tags gave no value of its range, keeping their recorded make
///   and model and their serial number. An offset is given only to a
///   capture time or a digitized time the standard tags gave without one:
///   its OffsetTimeOriginal or OffsetTimeDigitized is missing or could not
///   be read. Each of the two times takes it on its own, and it is held to
///   the bound of every offset
///   ([`DateTime::with_offset_minutes`](crate::datetime::DateTime::with_offset_minutes)).
///   The camera's own JPEGs whose digitized time is later than the capture
///   time, made from a raw file the camera held, name the camera's
///   firmware as their software, so the zone of the camera's clock is the
///   zone of that time too (`docs/formats/jpeg/container-and-metadata.md`,
///   section 4).
/// - **The camera settings** that only the MakerNote records, and its white
///   balance setting, finer than Exif's, in place of the mode the Exif tags
///   gave when it names one ([`camera_settings`]). The color space is read
///   only when the Exif IFD holds no ColorSpace, the raw image size and the
///   raw compression only for a raw file's picture, and the JPEG quality
///   and compression only for a JPEG's. Where the crop setting is "off",
///   or the MakerNote holds no CropHiSpeed, the image area is the sensor
///   format of the body the Make and Model name, when raw-files' table
///   holds it ([`Camera::body_facts`]). Where a raw file's MakerNote holds
///   no ImageSizeRAW, the raw size is large for a body the table says
///   offers no raw sizes, when the file holds raw data. The AF-area mode
///   is AFInfo2's, whatever the focus mode and whether or not the record
///   gives a point, and on a body without AFInfo2 AFInfo's.
/// - **The focus points**, from AFInfo2, always read, since the standard
///   tags never give one ([`focus`]); a shot whose focus mode, read with
///   the settings, is manual gives none. A recorded position is
///   placed on a crop's picture. The points a record with no position names
///   are placed from the table of the body the Model names, matched as
///   [`points::find`] matches it. A file without a Model has no table.
/// - **The layout placed on the picture**: every point of that table, in
///   the frame the named points take, whatever the shot
///   ([`placed_layout`]).
///
/// CropArea and CropHiSpeed are each read once, for the image size or the
/// image area and for the focus point, and AFInfo2 once, for the AF-area
/// mode and the focus point. Every other value is the standard
/// tags'. The layout of the body's autofocus points in the table's frame
/// is not given here: detection gives it from the Model alone.
///
/// A MakerNote of the short-header layout adds nothing: its tags are
/// numbered in a scheme of their own, which none of the rules above
/// reads, and none of them records a value the files confirm.
///
/// # Errors
///
/// `Io` when the reader fails.
///
/// [`Camera::body_facts`]: crate::camera::Camera::body_facts
fn metadata<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    standard: Standard,
    picture: Picture,
) -> Result<Metadata> {
    let Standard {
        mut metadata,
        from_default_crop,
        exif_records_color_space,
        raw_storage,
    } = standard;
    // None of the rules below is the short-header layout's.
    let Some(ifd) = makernote.tagged() else {
        return Ok(metadata);
    };
    // Read once, for the image size and the focus point. Without a raw
    // size, as in a JPEG, CropArea is never a size (ADR014).
    let crop = crop_area(src, makernote, metadata.raw_size)?;
    if !from_default_crop && let CropArea::Size(image) = crop {
        metadata.image_size = Some(image);
    }
    if metadata.camera.serial_number.is_none() {
        metadata.camera.serial_number = serial_number(src, makernote)?;
    }
    if metadata.exposure.iso.is_none() {
        metadata.exposure.iso = iso(makernote);
    }
    if !metadata.lens.has_range() {
        metadata.lens.set_range(lens_range(src, makernote)?);
    }
    // An offset belongs to a time: without a capture or digitized time,
    // WorldTime gives nothing.
    if let Some(offset) = world_time(makernote) {
        for time in [&mut metadata.capture_time, &mut metadata.digitized_time] {
            if let Some(recorded) = *time
                && recorded.offset_minutes().is_none()
                && let Ok(with_offset) = recorded.with_offset_minutes(offset)
            {
                *time = Some(with_offset);
            }
        }
    }
    // Read once, for the image area and the focus point.
    let crop_hi_speed = crop_hi_speed(src, makernote)?;
    // Read once, for the AF-area mode and the focus point.
    let af_info2 = ifd.get(src, rules::AF_INFO2)?;
    let standard = FromStandardTags {
        settings: metadata.camera_settings,
        model: metadata.camera.recorded_model.as_deref(),
        body: metadata.camera.body_facts(),
        holds_raw_data: metadata.raw_size.is_some(),
        exif_records_color_space,
        raw_compression: raw_storage.compression,
    };
    metadata.camera_settings = camera_settings(
        src,
        makernote,
        standard,
        crop_hi_speed,
        af_info2.as_deref(),
        picture,
    )?;
    // What a crop without a CropArea is placed on: a raw file has no
    // pixels of its own for it, a JPEG has the image size its reader gave.
    let pixels = match picture {
        Picture::Raw => None,
        Picture::Jpeg => metadata.image_size,
    };
    // The table that places the points a viewfinder shot names.
    let table = metadata
        .camera
        .recorded_model
        .as_deref()
        .and_then(points::find);
    let manual = metadata.camera_settings.focus_mode == Some(FocusMode::Manual);
    metadata.focus_points = focus(
        makernote,
        af_info2.as_deref(),
        manual,
        crop_hi_speed,
        crop,
        pixels,
        table,
    );
    // The table's layout on the picture, whatever the shot.
    metadata.placed_focus_layout =
        table.and_then(|table| placed_layout(table, crop_hi_speed, pixels));
    Ok(metadata)
}

/// Where the camera's autofocus aimed, as `af_info2`, `makernote`'s
/// AFInfo2 read by its rule, records it:
///
/// - a record of a position ([`FocusRecord::records_position`]) gives one
///   point, the primary. On a crop, which `crop` records, it is placed on
///   the crop's picture: CropArea, from `crop_area`, or a JPEG's `pixels`
///   ([`in_crop`]). A raw file's crop without a CropArea is read as
///   recorded.
/// - a record with no position, as a shot through a DSLR's viewfinder
///   writes, gives the points it names ([`NamedPoints`]), placed from
///   `table` ([`placed`]). CropArea plays no part there.
///
/// `None` when `manual` says that the shot is in manual focus: the point
/// of such a shot is not one the camera focused on, so it gives none,
/// whatever AFInfo2 holds (ADR012). `None` too when AFInfo2 is missing or
/// breaks its rule, or when its record cannot be read
/// ([`af_info2_record`]), checked or placed.
///
/// [`FocusRecord::records_position`]: af_info2::FocusRecord::records_position
/// [`NamedPoints`]: af_info2::NamedPoints
fn focus(
    makernote: &MakerNote,
    af_info2: Option<&[u8]>,
    manual: bool,
    crop: Option<CropHiSpeed>,
    crop_area: CropArea,
    pixels: Option<Size>,
    table: Option<&PointTable>,
) -> Option<FocusPoints> {
    let ifd = makernote.tagged()?;
    if manual {
        return None;
    }
    let record = af_info2_record(af_info2?, ifd.order())?;
    if record.records_position() {
        in_crop(record, crop, crop_area, pixels).map(FocusPoints::from)
    } else {
        record
            .named
            .zip(table)
            .and_then(|(named, table)| placed(named, table, crop, pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{
        Z9, af_info2, entry_at, expected, metadata_of, nikon_file, tags_block, whole,
    };
    use super::*;
    use crate::container::tiff::tags::MAKER_NOTE;
    use crate::container::tiff::{self, walk};
    use crate::error::ErrorKind;
    use crate::formats::tiff_metadata;
    use crate::limits::Limits;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{Order, UNDEFINED};
    use std::io::Cursor;

    #[test]
    fn a_reader_failure_while_reading_af_info2_is_an_error() {
        // The block placed so that its inner header and MakerNote IFD lie
        // before FAR and AFInfo2's value, its last 80 bytes, from FAR on.
        let order = Order::Little;
        let block = tags_block(order, |inner, ifd| {
            inner.undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9));
        })
        .0;
        let at = FAR - (whole(&block) - 80);
        let bytes = nikon_file(order, at, &block, 0, |builder, exif| {
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, whole(&block));
        });
        // The file is sound, for a reader that does not fail.
        assert_eq!(
            metadata_of(bytes.clone()).focus(),
            Some(expected(Z9, Some(true)))
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
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }
}
