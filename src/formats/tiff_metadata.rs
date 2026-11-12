// SPDX-License-Identifier: MIT OR Apache-2.0

//! The metadata reader shared by the TIFF-based formats: it reads what the
//! camera recorded about a picture from the standard TIFF and Exif tags.
//!
//! Cameras of every maker record the same facts in the same standard tags:
//! the camera's make and model, the software that wrote the file and the
//! orientation in IFD0, the capture time, the body's serial number, the
//! exposure, the lens and some of the camera's settings in the Exif IFD,
//! with the Interoperability IFD for the color space, where the picture
//! was taken in the GPS IFD, and the sizes in the IFD of the raw data. So
//! [`read`] reads them once, over the tree, for every format, and a format
//! adds only what its own structures record and the standard tags lack
//! (ADR011, which chose the standard tags first). The JPEG reader shares
//! it too, over the tree of a JPEG file's Exif segment, and then decides
//! the sizes itself: the picture's own size in place of the ones read
//! here, and no raw size (ADR014).
//!
//! The rules:
//!
//! - A value is read only from a tag of the type and count its rule names
//!   (ADR011). Each tag's rule is declared once, in the `rules` module
//!   below, and the IFD reads it ([`Ifd::get`], [`Ifd::held`]), checking
//!   the type and count before anything is read.
//! - A damaged value costs only itself. A tag that is missing, of a type
//!   or count other than its rule's, or whose value cannot be read, such
//!   as one longer than the limits' cap on one value, gives `None` for its
//!   own value; every other value is still read, and the file still opens.
//!   Only a failure of the reader is an error: it says nothing about the
//!   file.
//! - The make, the model and the serial number, of the camera and of the
//!   lens, and the software are text, read by the one rule [`text`] gives:
//!   cut at the first NUL, `None` when longer than 255 bytes, trimmed of
//!   spaces at both ends, `None` when nothing is left, and with bytes that
//!   are not UTF-8 replaced by U+FFFD. The texts Exif 3.1 gives as "ASCII
//!   or UTF-8", the make, the model, the software, the lens's make and
//!   model, the camera's owner, the artist, the copyright notice, the
//!   photographer, the editor and the title, are read from a value of
//!   either type by that rule, and so is the
//!   description, as free text without the 255-byte limit (below); the
//!   serial numbers, which Exif gives as ASCII alone, as every MakerNote
//!   text, only from type ASCII.
//! - The camera's owner is the Exif CameraOwnerName, read by that rule, as
//!   a fact of the camera beside its serial number.
//! - The software is IFD0's Software, as written and cleaned by that rule:
//!   the camera's firmware version, such as `Ver.1.10` from the D300's
//!   `Ver.1.10 `, or the name of the program that last wrote the file.
//!   Exif 3.0's CameraFirmware, which no checked file writes, is not read.
//! - The authorship is seven texts ([`authorship`]): the artist, the
//!   copyright notice and the description from IFD0's Artist, Copyright
//!   and ImageDescription, never from the copies Sony writes in IFD1 and
//!   IFD2; the photographer, the editor and the title from the Exif IFD's
//!   Photographer, ImageEditor and ImageTitle; and the comment from its
//!   UserComment. Each but the comment is of type ASCII or UTF-8, as Exif
//!   3.1 gives them. The artist, the copyright notice, the photographer,
//!   the editor and the title are read by the text rule above. The
//!   description and the comment are free text, cleaned by that rule but
//!   with no limit of 255 bytes, only the cap on any one value, so their
//!   line breaks are kept. A description that is a text a camera writes
//!   by itself, `SONY DSC`, matched whole once cleaned, is `None`
//!   ([`description`]). The comment is the text after UserComment's
//!   8-byte code, of type UNDEFINED, when that code is `ASCII`; under any
//!   other code, and when the value holds no more than its code, it is
//!   `None` ([`comment`]). Nothing is read from a MakerNote.
//! - The location is read from the GPS IFD the walk found, wherever it
//!   placed it ([`location`]). It needs a sound position: GPSLatitudeRef
//!   `N` or `S` and GPSLongitudeRef `E` or `W`, each read by the text rule
//!   and nothing else, and GPSLatitude and GPSLongitude, each exactly three
//!   RATIONALs, degrees, minutes and seconds as stored, with no zero
//!   denominator, minutes and seconds below 60, and no more than 90° of
//!   latitude or 180° of longitude. Without one there is no location, and
//!   no other GPS value is read. A zero is a real value of every GPS
//!   fraction: 0 seconds, 0 meters and 0° are kept. The other parts each
//!   need both their tags, of the type and count Exif gives, and a part
//!   whose tags are missing or damaged is `None` alone: the altitude,
//!   GPSAltitude, one RATIONAL, above its reference for a GPSAltitudeRef,
//!   one BYTE, of 0 or 2 and below it for 1 or 3, without naming the
//!   surface it is measured from; the GPS time, GPSDateStamp, `YYYY:MM:DD`
//!   of type ASCII, and GPSTimeStamp, three RATIONALs with whole hours and
//!   minutes and the seconds over a power of ten up to 10⁹, whose zeros
//!   are the digits of the fraction kept, a date and time that exist, in
//!   UTC ([`DateTime::from_gps`]); the image direction, GPSImgDirection,
//!   one RATIONAL below 360°, from the north GPSImgDirectionRef names, `T`
//!   true or `M` magnetic, with no default; and the map datum,
//!   GPSMapDatum, of type ASCII as Exif gives it, read by the text rule,
//!   `None` when blank, as the Z 9 writes it. GPSVersionID, GPSStatus and the
//!   other GPS tags are not read.
//! - The capture time is the Exif DateTimeOriginal, with its own fraction
//!   of a second (SubSecTimeOriginal) and offset from UTC
//!   (OffsetTimeOriginal), each of type ASCII; [`DateTime::from_exif`]
//!   checks them. Without a DateTimeOriginal there is no capture time,
//!   whatever fraction or offset the file records. It is never taken from
//!   DateTimeDigitized or from IFD0's DateTime, which editors change.
//! - The ISO is the Exif PhotographicSensitivity, of type SHORT, LONG or
//!   IFD, its first value. When that is missing, unreadable, zero, which Exif
//!   writes for an unknown sensitivity, or 65535, which Exif writes for one
//!   of 65535 or more, the ISO is the RecommendedExposureIndex, or else the
//!   ISOSpeed, each its first value and only when above zero. A 65535 that
//!   neither replaces stands, as the least the sensitivity was; a zero
//!   that neither replaces is `None` ([`iso`]).
//! - The exposure time, the f-number and the focal length are the Exif
//!   ExposureTime, FNumber and FocalLength, of type RATIONAL, their first
//!   value as stored, read alone whatever the count, as the ISO's is.
//!   Each is `None` when its numerator or its denominator is zero, since
//!   Exif writes a zero for a value the camera did not know
//!   ([`Rational::known`]). The exposure bias is the Exif
//!   ExposureBiasValue, of type SRATIONAL, its first value as stored, read
//!   the same way; it is `None` only when its denominator is zero, since a
//!   zero bias, such as 0/6, is no compensation ([`SignedRational::new`]).
//! - The focal length in 35 mm terms is the Exif FocalLengthIn35mmFilm,
//!   exactly one SHORT, the type and count the standard gives it; it is
//!   `None` when zero, which Exif writes for one the
//!   camera did not know ([`focal_length_in_35mm_film`]).
//! - The lens is described by the Exif LensMake, LensModel and
//!   LensSerialNumber, and by the LensSpecification, of type RATIONAL and
//!   of exactly four values: the shortest and longest focal length and the
//!   f-number at each. A LensSpecification of any other count is ignored
//!   whole; each of its four values is `None` when its numerator or its
//!   denominator is zero, as a 0/0 for a value the camera did not know
//!   ([`Rational::known`]). A LensModel made only of `-` characters, which
//!   Sony writes for a manual lens, names no lens and is `None`. A file
//!   whose lens tags give no value gives a lens with every value `None`
//!   ([`lens`]).
//! - The camera settings that Exif records come from tags of the Exif IFD,
//!   each exactly one SHORT, as the standard gives them,
//!   and each code Exif defines and a variant names gives that variant;
//!   every other code, the ones Exif writes for "not defined" or "unknown"
//!   among them, gives `None` ([`camera_settings`]):
//!   - the exposure program is the ExposureProgram, 1 to 8; 0, "not
//!     defined", and 9, which Exif does not define, give `None`
//!     ([`exposure_program`]);
//!   - the metering mode is the MeteringMode, 1 to 6 and 255, "other"; 0,
//!     "unknown", gives `None` ([`metering_mode`]);
//!   - the flash is the Flash, read by its bits: whether it fired, and its
//!     mode, compulsory firing, compulsory suppression or auto, none when
//!     the camera has no flash function; return light and red-eye
//!     reduction are not read. A value with a reserved bit set, or that
//!     says a flash fired on a camera without one, gives `None`
//!     ([`flash`]);
//!   - the white balance mode is auto for a WhiteBalance of 0; for 1,
//!     "manual", it is the kind of light the LightSource names, such as
//!     shade, and `None` for a light no mode stands for, such as the
//!     standard light D65, for "unknown" and for "other"
//!     ([`white_balance_mode`]);
//!   - the color space is sRGB for a ColorSpace of 1, and Adobe RGB for
//!     65535, "uncalibrated", when the Interoperability IFD's
//!     InteropIndex, read by the text rule, is `R03`, a DCF option file;
//!     any other ColorSpace, an "uncalibrated" one beside another index or
//!     none, and no ColorSpace give `None` ([`color_space`]).
//!
//!   The other settings are recorded only in a maker's MakerNote, so they
//!   are `None` here: the white balance's color temperature and
//!   fine-tuning, the shutter, the electronic front curtain setting, the
//!   image area and the raw image size. A format's MakerNote reader fills
//!   them where its MakerNote records them, and puts the MakerNote's white
//!   balance setting, finer than Exif's, in place of the mode read here
//!   when the MakerNote names one, as Nikon's and Sony's readers both do.
//!   So that a MakerNote's color space never overrules Exif's, [`read`]
//!   also says whether the Exif IFD keeps a ColorSpace entry, of any known
//!   type and any count, whether or not it gives a color space
//!   ([`Standard`]): only when it keeps none may a MakerNote give the
//!   color space, as Nikon's raw files, which record none in Exif, need.
//!   An entry the IFD's reader skips as damaged, of an unknown type, of a
//!   length past 32 bits or with its value outside its block, the file or
//!   a JPEG's Exif segment, counts as none, so the MakerNote may stand in,
//!   as for any damaged value (ADR011).
//! - A file without an Exif IFD has no capture time, no serial number, no
//!   exposure, no lens, none of the camera settings, and no photographer,
//!   editor, title or comment. A file without a GPS IFD has no location.
//! - The orientation is IFD0's Orientation, read as the preview finder reads
//!   it ([`tiff_previews::orientation`]), so that the metadata and the
//!   previews never disagree.
//! - The raw size is the ImageWidth and ImageLength, of type SHORT, LONG
//!   or IFD, of the IFD of the tree, the Exif, GPS and Interoperability
//!   IFDs left out, that the preview finder takes for raw data
//!   ([`tiff_previews::is_raw`]), whose width and height are both above
//!   zero, and that has the most pixels of all such IFDs, the first in the
//!   tree's order on a tie: a file can hold a small full-resolution raw
//!   image beside the real one, as the public Coolpix E5700 sample 4456
//!   does. An IFD whose tags say nothing readable either way is passed
//!   over, as the finder passes over it. Without such an IFD there is no
//!   raw size, and no image size.
//! - The image size is the default crop of that same IFD, its
//!   DefaultCropSize, when it and its DefaultCropOrigin each hold exactly
//!   two whole numbers, the size is above zero, and the crop lies inside
//!   the raw size; otherwise it is the raw size ([`default_crop`]). A
//!   format whose own structures record a crop may then put it in place of
//!   the raw size, never of a default crop, which [`read`] says it has
//!   found ([`Standard`]): NEF puts the CropArea of Nikon's MakerNote there,
//!   checked by the same test of a crop ([`crop`]).
//!
//! Nothing here panics on any input: every value is read through the IFD's
//! checked readers, which check its offset and length against the file
//! and cap its size (ADR008).

use std::io::{Read, Seek};

use crate::authorship::Authorship;
use crate::camera::Camera;
use crate::camera_settings::{
    CameraSettings, ColorSpace, ExposureProgram, Flash, FlashMode, MeteringMode, WhiteBalance,
    WhiteBalanceMode,
};
use crate::container::tiff::tags::COLOR_SPACE;
use crate::container::tiff::{First, Ifd, Rule, Text, Tree, skip_damage};
use crate::datetime::DateTime;
use crate::entries::clean_text;
use crate::error::Result;
use crate::formats::tiff_previews;
use crate::geometry::Size;
use crate::io::Source;
use crate::location::{Altitude, Direction, Latitude, Location, Longitude, North};
use crate::metadata::{Exposure, Lens, Metadata};
use crate::rational::{Rational, SignedRational};

/// The rules of the standard tags this reader reads: each tag's number,
/// from [`tags`](crate::container::tiff::tags), and the types and count its
/// value must have to be read (ADR011), as the module documentation gives
/// them. [`Ifd::get`] and [`Ifd::held`] read them.
///
/// The ISO's three tags and the raw size's two are read by their first
/// value alone, whatever the count, a [`First`] rule, of type SHORT, LONG
/// or IFD, a LONG under another name; so are the exposure's four
/// fractions. The default crop's two tags each have two rules, for the two
/// forms the DNG specification allows them.
mod rules {
    use crate::container::tiff::tags;
    use crate::container::tiff::{FieldType, First, Held, Rule, Text};
    use crate::rational::{Rational, SignedRational};

    /// Text Exif 3.1 gives as "ASCII or UTF-8".
    const ASCII_OR_UTF8: &[FieldType] = &[FieldType::Ascii, FieldType::Utf8];
    /// Text Exif gives as ASCII alone.
    const ASCII: &[FieldType] = &[FieldType::Ascii];
    /// One number of a tag Exif gives as SHORT or LONG, IFD taken as a LONG.
    const NUMBER: &[FieldType] = &[FieldType::Short, FieldType::Long, FieldType::Ifd];
    const SHORT: &[FieldType] = &[FieldType::Short];
    const RATIONAL: &[FieldType] = &[FieldType::Rational];

    // IFD0.
    pub(super) const MAKE: Rule<Text> = Rule::new(tags::MAKE, ASCII_OR_UTF8);
    pub(super) const MODEL: Rule<Text> = Rule::new(tags::MODEL, ASCII_OR_UTF8);
    pub(super) const SOFTWARE: Rule<Text> = Rule::new(tags::SOFTWARE, ASCII_OR_UTF8);
    pub(super) const ARTIST: Rule<Text> = Rule::new(tags::ARTIST, ASCII_OR_UTF8);
    pub(super) const COPYRIGHT: Rule<Text> = Rule::new(tags::COPYRIGHT, ASCII_OR_UTF8);
    pub(super) const IMAGE_DESCRIPTION: Rule<Text> =
        Rule::new(tags::IMAGE_DESCRIPTION, ASCII_OR_UTF8);

    // The Exif IFD.
    pub(super) const BODY_SERIAL_NUMBER: Rule<Text> = Rule::new(tags::BODY_SERIAL_NUMBER, ASCII);
    pub(super) const CAMERA_OWNER_NAME: Rule<Text> =
        Rule::new(tags::CAMERA_OWNER_NAME, ASCII_OR_UTF8);
    pub(super) const PHOTOGRAPHER: Rule<Text> = Rule::new(tags::PHOTOGRAPHER, ASCII_OR_UTF8);
    pub(super) const IMAGE_EDITOR: Rule<Text> = Rule::new(tags::IMAGE_EDITOR, ASCII_OR_UTF8);
    pub(super) const IMAGE_TITLE: Rule<Text> = Rule::new(tags::IMAGE_TITLE, ASCII_OR_UTF8);
    pub(super) const USER_COMMENT: Rule<Vec<u8>> =
        Rule::new(tags::USER_COMMENT, &[FieldType::Undefined]);
    pub(super) const DATE_TIME_ORIGINAL: Rule<Text> = Rule::new(tags::DATE_TIME_ORIGINAL, ASCII);
    pub(super) const SUB_SEC_TIME_ORIGINAL: Rule<Text> =
        Rule::new(tags::SUB_SEC_TIME_ORIGINAL, ASCII);
    pub(super) const OFFSET_TIME_ORIGINAL: Rule<Text> =
        Rule::new(tags::OFFSET_TIME_ORIGINAL, ASCII);
    pub(super) const PHOTOGRAPHIC_SENSITIVITY: Rule<First<u32>> =
        Rule::new(tags::PHOTOGRAPHIC_SENSITIVITY, NUMBER);
    pub(super) const RECOMMENDED_EXPOSURE_INDEX: Rule<First<u32>> =
        Rule::new(tags::RECOMMENDED_EXPOSURE_INDEX, NUMBER);
    pub(super) const ISO_SPEED: Rule<First<u32>> = Rule::new(tags::ISO_SPEED, NUMBER);
    pub(super) const EXPOSURE_TIME: Rule<First<Rational>> =
        Rule::new(tags::EXPOSURE_TIME, RATIONAL);
    pub(super) const F_NUMBER: Rule<First<Rational>> = Rule::new(tags::F_NUMBER, RATIONAL);
    pub(super) const FOCAL_LENGTH: Rule<First<Rational>> = Rule::new(tags::FOCAL_LENGTH, RATIONAL);
    pub(super) const EXPOSURE_BIAS_VALUE: Rule<First<SignedRational>> =
        Rule::new(tags::EXPOSURE_BIAS_VALUE, &[FieldType::SRational]);
    pub(super) const FOCAL_LENGTH_IN_35MM_FILM: Held<[u32; 1]> =
        Held::new(tags::FOCAL_LENGTH_IN_35MM_FILM, SHORT);
    pub(super) const LENS_MAKE: Rule<Text> = Rule::new(tags::LENS_MAKE, ASCII_OR_UTF8);
    pub(super) const LENS_MODEL: Rule<Text> = Rule::new(tags::LENS_MODEL, ASCII_OR_UTF8);
    pub(super) const LENS_SERIAL_NUMBER: Rule<Text> = Rule::new(tags::LENS_SERIAL_NUMBER, ASCII);
    pub(super) const LENS_SPECIFICATION: Rule<[Rational; 4]> =
        Rule::new(tags::LENS_SPECIFICATION, RATIONAL);
    pub(super) const EXPOSURE_PROGRAM: Held<[u32; 1]> = Held::new(tags::EXPOSURE_PROGRAM, SHORT);
    pub(super) const METERING_MODE: Held<[u32; 1]> = Held::new(tags::METERING_MODE, SHORT);
    pub(super) const FLASH: Held<[u32; 1]> = Held::new(tags::FLASH, SHORT);
    pub(super) const WHITE_BALANCE: Held<[u32; 1]> = Held::new(tags::WHITE_BALANCE, SHORT);
    pub(super) const LIGHT_SOURCE: Held<[u32; 1]> = Held::new(tags::LIGHT_SOURCE, SHORT);
    pub(super) const COLOR_SPACE: Held<[u32; 1]> = Held::new(tags::COLOR_SPACE, SHORT);

    // The Interoperability IFD.
    pub(super) const INTEROP_INDEX: Rule<Text> = Rule::new(tags::INTEROP_INDEX, ASCII);

    // The GPS IFD.
    pub(super) const GPS_LATITUDE_REF: Rule<Text> = Rule::new(tags::GPS_LATITUDE_REF, ASCII);
    pub(super) const GPS_LATITUDE: Rule<[Rational; 3]> = Rule::new(tags::GPS_LATITUDE, RATIONAL);
    pub(super) const GPS_LONGITUDE_REF: Rule<Text> = Rule::new(tags::GPS_LONGITUDE_REF, ASCII);
    pub(super) const GPS_LONGITUDE: Rule<[Rational; 3]> = Rule::new(tags::GPS_LONGITUDE, RATIONAL);
    pub(super) const GPS_ALTITUDE_REF: Held<[u8; 1]> =
        Held::new(tags::GPS_ALTITUDE_REF, &[FieldType::Byte]);
    pub(super) const GPS_ALTITUDE: Rule<[Rational; 1]> = Rule::new(tags::GPS_ALTITUDE, RATIONAL);
    pub(super) const GPS_DATE_STAMP: Rule<Text> = Rule::new(tags::GPS_DATE_STAMP, ASCII);
    pub(super) const GPS_TIME_STAMP: Rule<[Rational; 3]> =
        Rule::new(tags::GPS_TIME_STAMP, RATIONAL);
    pub(super) const GPS_IMG_DIRECTION_REF: Rule<Text> =
        Rule::new(tags::GPS_IMG_DIRECTION_REF, ASCII);
    pub(super) const GPS_IMG_DIRECTION: Rule<[Rational; 1]> =
        Rule::new(tags::GPS_IMG_DIRECTION, RATIONAL);
    pub(super) const GPS_MAP_DATUM: Rule<Text> = Rule::new(tags::GPS_MAP_DATUM, ASCII);

    // The IFD of the raw data.
    pub(super) const IMAGE_WIDTH: Rule<First<u32>> = Rule::new(tags::IMAGE_WIDTH, NUMBER);
    pub(super) const IMAGE_LENGTH: Rule<First<u32>> = Rule::new(tags::IMAGE_LENGTH, NUMBER);
    pub(super) const DEFAULT_CROP_ORIGIN: Rule<[u32; 2]> =
        Rule::new(tags::DEFAULT_CROP_ORIGIN, NUMBER);
    pub(super) const DEFAULT_CROP_ORIGIN_FRACTIONS: Rule<[Rational; 2]> =
        Rule::new(tags::DEFAULT_CROP_ORIGIN, RATIONAL);
    pub(super) const DEFAULT_CROP_SIZE: Rule<[u32; 2]> = Rule::new(tags::DEFAULT_CROP_SIZE, NUMBER);
    pub(super) const DEFAULT_CROP_SIZE_FRACTIONS: Rule<[Rational; 2]> =
        Rule::new(tags::DEFAULT_CROP_SIZE, RATIONAL);
}

/// The longest text, in bytes as stored, that is taken for a value rather
/// than for damage.
///
/// The longest text value measured on the public sample files is 65 bytes,
/// a lens model, so a longer one than this is not a name or a serial
/// number a camera wrote.
const MAX_TEXT_LEN: usize = 255;

/// The texts that cameras write in ImageDescription by themselves, whatever
/// the picture shows, each as the whole text once cleaned: none is a
/// description.
///
/// Only a text measured files show a camera writes joins the list. Older
/// Sony bodies write `SONY DSC`: in 9 bytes, or padded with spaces to 32,
/// on 23 public ARW files and in a public sample JPEG of the DSLR-A100, as
/// the ARW notes (section 9.8) and the JPEG notes (section 4) list. Newer
/// Sony bodies write 32 spaces, which are no text.
const PLACEHOLDER_DESCRIPTIONS: [&str; 1] = ["SONY DSC"];

/// The 8 bytes that start a UserComment whose text is ASCII: the code
/// `ASCII` and three NULs (Exif 3.1, 4.6.4 and Table 4).
const ASCII_CODE: &[u8; 8] = b"ASCII\0\0\0";

/// The PhotographicSensitivity that Exif writes for a sensitivity of 65535
/// or more: a least value, which a RecommendedExposureIndex or an ISOSpeed
/// replaces with the real one when the file records it.
const ISO_65535_OR_MORE: u32 = 65535;

/// What [`read`] reads from the standard tags: the metadata, and, for a
/// format whose own structures record the same values, whether the
/// standard tags recorded two of them: the image size as a crop of the
/// raw IFD's, and the color space.
///
/// The standard tags come first (ADR011). A format's own crop, such as
/// Nikon's crop area, may replace the raw size the image size otherwise
/// falls back on, but never a default crop; comparing the image size with
/// the raw size cannot tell the two apart, since a default crop may cover
/// the whole raw. A format's own color space, such as the one in Nikon's
/// MakerNote, may stand in only where the Exif IFD holds no ColorSpace: a
/// ColorSpace that gives none, such as a program's "uncalibrated", is
/// still Exif's word, and a MakerNote the program left behind may describe
/// a picture that is no longer there. The color space `None` cannot tell
/// the two apart.
#[derive(Debug, Default)]
pub(crate) struct Standard {
    /// The metadata, every value of it from the standard tags.
    pub(crate) metadata: Metadata,
    /// Whether the image size is the raw IFD's default crop; `false` when
    /// it is the raw size, or there is no image size.
    pub(crate) from_default_crop: bool,
    /// Whether the Exif IFD keeps a ColorSpace entry, of any known type and
    /// any count, whether or not it gives a color space; `false` when
    /// there is no Exif IFD, and when the IFD's reader skipped the entry
    /// as damaged, of an unknown type, of a length past 32 bits or with its
    /// value outside its block, the file or a JPEG's Exif segment
    /// ([`Ifd::entry`]).
    pub(crate) exif_records_color_space: bool,
}

/// Reads the metadata that `tree` records, the IFDs of a TIFF-based file
/// in `src` or of the Exif segment of a JPEG file in `src`, with whether
/// its image size is a default crop and whether its Exif IFD holds a
/// ColorSpace.
///
/// See the [module documentation](self) for the rules.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
pub(crate) fn read<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Standard> {
    let ifd0 = tree.ifd0();
    let (serial_number, owner_name, capture_time, exposure, camera_settings, lens) =
        match tree.exif() {
            Some(exif) => (
                text(src, exif, rules::BODY_SERIAL_NUMBER)?,
                text(src, exif, rules::CAMERA_OWNER_NAME)?,
                capture_time(src, exif)?,
                exposure(src, exif)?,
                camera_settings(src, exif, tree.interop())?,
                lens(src, exif)?,
            ),
            None => (
                None,
                None,
                None,
                Exposure::default(),
                CameraSettings::default(),
                Lens::default(),
            ),
        };
    let (raw_size, default_crop) = sizes(src, tree)?;
    let metadata = Metadata {
        camera: Camera {
            recorded_make: text(src, ifd0, rules::MAKE)?,
            recorded_model: text(src, ifd0, rules::MODEL)?,
            serial_number,
            owner_name,
        },
        software: text(src, ifd0, rules::SOFTWARE)?,
        authorship: authorship(src, tree)?,
        location: match tree.gps() {
            Some(gps) => location(src, gps)?,
            None => None,
        },
        capture_time,
        orientation: tiff_previews::orientation(src, tree)?,
        exposure,
        camera_settings,
        lens,
        raw_size,
        image_size: default_crop.or(raw_size),
        // No standard tag records where the camera focused.
        focus_points: None,
        // None of the standard tags holds a layout: the readers give one
        // only from a table of the body's points.
        focus_layout: None,
    };
    Ok(Standard {
        metadata,
        from_default_crop: default_crop.is_some(),
        exif_records_color_space: tree
            .exif()
            .is_some_and(|exif| exif.entry(COLOR_SPACE).is_some()),
    })
}

/// The text of the tag `rule` names in `ifd`, cleaned as every text value
/// of the metadata is, such as the camera's make and serial number, whether
/// a standard tag or a format's own, such as the serial number in Nikon's
/// MakerNote, records it.
///
/// The value must be of a type the rule names: ASCII, or ASCII or UTF-8
/// for the texts Exif 3.1 gives as "ASCII or UTF-8", such as the Make, the
/// Model and the LensModel. It ends at its first NUL, or with the value
/// when there is none. When what is before the NUL is longer than
/// [`MAX_TEXT_LEN`] bytes, counted as stored, before any trimming or
/// replacement, it is damage and `None`. Spaces at either end are padding
/// and are dropped, as in the DSLR-A100's Make, `SONY` and eleven spaces;
/// spaces inside the text, and any other blank such as a tab, are kept, as
/// the capture time's reading and the formats' Make test keep them. Text
/// that is empty once trimmed, as a value of NULs or of spaces only, is
/// `None`. Bytes that are not UTF-8 become U+FFFD, whatever the type says.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason, such as one longer than the limits' cap on one value
/// ([`Limits::max_value_bytes`](crate::Limits::max_value_bytes)), is `None`.
pub(crate) fn text<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    rule: Rule<Text>,
) -> Result<Option<String>> {
    Ok(short_text(ifd.get(src, rule)?))
}

/// The text `bytes` hold, up to their first NUL, cleaned by
/// [`clean_text`]; `None` when there are none, or when they are longer
/// than [`MAX_TEXT_LEN`].
fn short_text(bytes: Option<Vec<u8>>) -> Option<String> {
    bytes
        .filter(|bytes| bytes.len() <= MAX_TEXT_LEN)
        .and_then(|bytes| clean_text(&bytes))
}

/// Who made the picture and what they wrote about it, from `tree`: the
/// artist, the copyright notice and the description from IFD0, and the
/// photographer, the editor, the title and the comment from the Exif IFD.
///
/// IFD0's texts are read from IFD0 alone: the copies Sony writes in IFD1
/// and IFD2 are not read. The artist, the copyright notice, the
/// photographer, the editor and the title are read by [`text`]; the
/// description by [`description`] and the
/// comment by [`comment`], as free text. A file without an Exif IFD gives
/// no photographer, editor, title or comment.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
fn authorship<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Authorship> {
    let ifd0 = tree.ifd0();
    let mut authorship = Authorship {
        artist: text(src, ifd0, rules::ARTIST)?,
        copyright: text(src, ifd0, rules::COPYRIGHT)?,
        description: description(src, ifd0)?,
        ..Authorship::default()
    };
    if let Some(exif) = tree.exif() {
        authorship.photographer = text(src, exif, rules::PHOTOGRAPHER)?;
        authorship.editor = text(src, exif, rules::IMAGE_EDITOR)?;
        authorship.title = text(src, exif, rules::IMAGE_TITLE)?;
        authorship.comment = comment(src, exif)?;
    }
    Ok(authorship)
}

/// What the picture shows, from IFD0 `ifd0`'s ImageDescription, of type
/// ASCII or UTF-8: free text, cut at its first NUL and cleaned as every
/// text is, with no limit of 255 bytes, only the cap on any one value, and
/// with its line breaks kept.
///
/// `None` when the cleaned text is one of [`PLACEHOLDER_DESCRIPTIONS`],
/// whole, whatever the camera's Make: a text a camera writes by itself is
/// no description. A longer text that holds one, such as `SONY DSC` and a
/// caption, is a description.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason, such as one longer than the cap, is `None`.
fn description<R: Read + Seek>(src: &mut Source<R>, ifd0: &Ifd) -> Result<Option<String>> {
    let Some(bytes) = ifd0.get(src, rules::IMAGE_DESCRIPTION)? else {
        return Ok(None);
    };
    Ok(clean_text(&bytes).filter(|text| !PLACEHOLDER_DESCRIPTIONS.contains(&text.as_str())))
}

/// A comment on the picture, from the Exif IFD `exif`'s UserComment, of
/// type UNDEFINED: the text after its 8-byte code when the code is
/// [`ASCII_CODE`], cut at its first NUL and cleaned as every text is, with
/// no limit of 255 bytes, only the cap on any one value, and with its line
/// breaks kept.
///
/// `None` under any other code: the undefined code of eight NULs, `JIS`,
/// `UNICODE`, which Exif defines as UTF-8, and a code Exif does not list;
/// and when the value is no longer than its code, or of another type.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason, such as one longer than the cap, is `None`.
fn comment<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Option<String>> {
    let Some(bytes) = exif.get(src, rules::USER_COMMENT)? else {
        return Ok(None);
    };
    Ok(bytes.strip_prefix(ASCII_CODE).and_then(clean_text))
}

/// Where the picture was taken, from the GPS IFD `gps`: its position, with
/// the altitude, the GPS time, the image direction and the map datum.
///
/// The position needs GPSLatitudeRef `N` or `S` and GPSLongitudeRef `E` or
/// `W`, each read by [`text`], and GPSLatitude and GPSLongitude, each
/// exactly three RATIONALs that [`Latitude`] or
/// [`Longitude`] accepts; without one there is no location, and no other
/// GPS value is read. The other parts each need both their tags, and a
/// part whose tags are missing or damaged is `None` alone:
///
/// - the altitude, GPSAltitude, exactly one RATIONAL with a denominator
///   above zero, above its reference for a GPSAltitudeRef of 0 or 2 and
///   below it for 1 or 3, exactly one BYTE;
/// - the time, GPSDateStamp, of type ASCII, and GPSTimeStamp, exactly
///   three RATIONALs, as [`DateTime::from_gps`] reads them;
/// - the image direction, GPSImgDirection, exactly one RATIONAL that
///   [`Direction`] accepts, below 360°, from the north GPSImgDirectionRef
///   names, read by [`text`]: `T` true, `M` magnetic, and no default;
/// - the map datum, GPSMapDatum, read by [`text`]; blank is `None`.
///
/// GPSVersionID, GPSStatus and the other GPS tags are not read.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
fn location<R: Read + Seek>(src: &mut Source<R>, gps: &Ifd) -> Result<Option<Location>> {
    let latitude = match text(src, gps, rules::GPS_LATITUDE_REF)?.as_deref() {
        Some("N") => Latitude::north,
        Some("S") => Latitude::south,
        _ => return Ok(None),
    };
    let Some(latitude) = gps
        .get(src, rules::GPS_LATITUDE)?
        .and_then(|[degrees, minutes, seconds]| latitude(degrees, minutes, seconds).ok())
    else {
        return Ok(None);
    };
    let longitude = match text(src, gps, rules::GPS_LONGITUDE_REF)?.as_deref() {
        Some("E") => Longitude::east,
        Some("W") => Longitude::west,
        _ => return Ok(None),
    };
    let Some(longitude) = gps
        .get(src, rules::GPS_LONGITUDE)?
        .and_then(|[degrees, minutes, seconds]| longitude(degrees, minutes, seconds).ok())
    else {
        return Ok(None);
    };

    let altitude = match gps.held(rules::GPS_ALTITUDE_REF) {
        Some([0 | 2]) => gps
            .get(src, rules::GPS_ALTITUDE)?
            .and_then(|[m]| Altitude::above(m).ok()),
        Some([1 | 3]) => gps
            .get(src, rules::GPS_ALTITUDE)?
            .and_then(|[m]| Altitude::below(m).ok()),
        _ => None,
    };
    let time = match gps.get(src, rules::GPS_DATE_STAMP)? {
        Some(date) => gps.get(src, rules::GPS_TIME_STAMP)?.and_then(|time| {
            DateTime::from_gps(
                &date,
                time.map(|fraction| (fraction.numerator, fraction.denominator)),
            )
        }),
        None => None,
    };
    let north = match text(src, gps, rules::GPS_IMG_DIRECTION_REF)?.as_deref() {
        Some("T") => Some(North::True),
        Some("M") => Some(North::Magnetic),
        _ => None,
    };
    let image_direction = match north {
        Some(north) => gps
            .get(src, rules::GPS_IMG_DIRECTION)?
            .and_then(|[degrees]| Direction::new(degrees, north).ok()),
        None => None,
    };
    Ok(Some(Location {
        altitude,
        time,
        image_direction,
        map_datum: text(src, gps, rules::GPS_MAP_DATUM)?,
        ..Location::new(latitude, longitude)
    }))
}

/// When the picture was taken, from the Exif IFD `exif`: its
/// DateTimeOriginal, with its SubSecTimeOriginal and OffsetTimeOriginal
/// when the file records them, all three of type ASCII, as
/// [`DateTime::from_exif`] reads them.
///
/// `None` when there is no DateTimeOriginal that can be read, or when it
/// names no real date and time, such as an unset clock. A fraction of a
/// second or an offset that cannot be read is left out, and the time
/// stands without it.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn capture_time<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Option<DateTime>> {
    let Some(date) = exif.get(src, rules::DATE_TIME_ORIGINAL)? else {
        return Ok(None);
    };
    let subsec = exif.get(src, rules::SUB_SEC_TIME_ORIGINAL)?;
    let offset = exif.get(src, rules::OFFSET_TIME_ORIGINAL)?;
    Ok(DateTime::from_exif(
        &date,
        subsec.as_deref(),
        offset.as_deref(),
    ))
}

/// The exposure the picture was taken with, from the Exif IFD `exif`: the
/// ISO, as [`iso`] reads it, the four fractions, as [`rational`] and
/// [`signed_rational`] read them, and the focal length in 35 mm terms, as
/// [`focal_length_in_35mm_film`] reads it.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
fn exposure<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Exposure> {
    Ok(Exposure {
        iso: iso(src, exif)?,
        exposure_time: rational(src, exif, rules::EXPOSURE_TIME)?,
        f_number: rational(src, exif, rules::F_NUMBER)?,
        focal_length: rational(src, exif, rules::FOCAL_LENGTH)?,
        focal_length_in_35mm_film: focal_length_in_35mm_film(exif),
        exposure_bias: signed_rational(src, exif, rules::EXPOSURE_BIAS_VALUE)?,
    })
}

/// The focal length in 35 mm terms, in whole millimeters, from the Exif
/// IFD `exif`: its FocalLengthIn35mmFilm, which must be exactly one SHORT,
/// the type and count the standard gives it. `None` when the value is
/// zero, which Exif writes for a focal length the camera did not know, as
/// the ILCE-7S does with a manual lens (public sample 1582).
fn focal_length_in_35mm_film(exif: &Ifd) -> Option<u32> {
    let [millimeters] = exif.held(rules::FOCAL_LENGTH_IN_35MM_FILM)?;
    (millimeters > 0).then_some(millimeters)
}

/// The ISO the picture was taken at, from the Exif IFD `exif`.
///
/// It is the first value of the PhotographicSensitivity, of type SHORT or
/// LONG, or IFD, a LONG under another name, read alone whatever the count
/// ([`First`]). Exif gives two of its values a meaning of their own:
/// zero, a sensitivity the camera did not know, and 65535
/// ([`ISO_65535_OR_MORE`]), one of 65535 or more. For those, and when the
/// tag is missing or cannot be read, the ISO is the first value of the
/// RecommendedExposureIndex, or else that of the ISOSpeed, each read the
/// same way and taken only when above zero; the two are read only then.
/// When neither gives one, a 65535 stands, since the sensitivity was at
/// least that, and anything else is `None`.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn iso<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Option<u32>> {
    let sensitivity = exif.get(src, rules::PHOTOGRAPHIC_SENSITIVITY)?;
    if let Some(iso) = sensitivity.filter(|&iso| iso != 0 && iso != ISO_65535_OR_MORE) {
        return Ok(Some(iso));
    }
    for rule in [rules::RECOMMENDED_EXPOSURE_INDEX, rules::ISO_SPEED] {
        if let Some(iso) = exif.get(src, rule)?.filter(|&iso| iso > 0) {
            return Ok(Some(iso));
        }
    }
    Ok(sensitivity.filter(|&iso| iso == ISO_65535_OR_MORE))
}

/// The first fraction of the tag `rule` names in `ifd`, which must be of
/// type RATIONAL, as stored, read alone whatever the count; `None` when its
/// numerator or its denominator is zero ([`Rational::known`]).
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn rational<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    rule: Rule<First<Rational>>,
) -> Result<Option<Rational>> {
    let first = ifd.get(src, rule)?;
    Ok(first.and_then(|first| Rational::known(first.numerator, first.denominator)))
}

/// The first fraction of the tag `rule` names in `ifd`, which must be of
/// type SRATIONAL, as stored, read alone whatever the count; `None` when
/// its denominator is zero ([`SignedRational::new`]).
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn signed_rational<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    rule: Rule<First<SignedRational>>,
) -> Result<Option<SignedRational>> {
    let first = ifd.get(src, rule)?;
    Ok(first.and_then(|first| SignedRational::new(first.numerator, first.denominator).ok()))
}

/// The lens the picture was taken with, from the Exif IFD `exif`: its
/// LensMake, LensModel and LensSerialNumber, read by the rule [`text`]
/// gives, and its range from the LensSpecification, as
/// [`lens_specification`] reads it. A file with none of these tags, or
/// none that can be read, gives a lens with every value `None`.
///
/// A LensModel made only of `-` characters is `None`: Sony writes `----`
/// for a manual lens, as on the public ILCE-7S sample 1582, and that names
/// no lens. Only the model is read this way; a model with dashes among
/// other characters, such as `FE 24-70mm F2.8 GM`, is kept.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
fn lens<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Lens> {
    let recorded_make = text(src, exif, rules::LENS_MAKE)?;
    let recorded_model =
        text(src, exif, rules::LENS_MODEL)?.filter(|model| !model.chars().all(|c| c == '-'));
    let serial_number = text(src, exif, rules::LENS_SERIAL_NUMBER)?;
    let mut lens = Lens {
        recorded_make,
        recorded_model,
        serial_number,
        ..Lens::default()
    };
    lens.set_range(lens_specification(src, exif)?);
    Ok(lens)
}

/// The lens's range, from the Exif IFD `exif`'s LensSpecification: its
/// shortest and longest focal length, then the f-number at each, in the
/// order the tag stores them.
///
/// The tag must be of type RATIONAL and hold exactly four values; with any
/// other count it describes no range the reader can trust, and all four
/// are `None`, without a read.
/// Each value is `None` when its numerator or its denominator is zero
/// ([`Rational::known`]): Exif writes 0/0 for a value the camera did not
/// know, such as the apertures of a lens that does not report them.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason gives four `None`s.
fn lens_specification<R: Read + Seek>(
    src: &mut Source<R>,
    exif: &Ifd,
) -> Result<[Option<Rational>; 4]> {
    let values = exif.get(src, rules::LENS_SPECIFICATION)?;
    Ok(values.map_or([None; 4], |values| {
        values.map(|value| Rational::known(value.numerator, value.denominator))
    }))
}

/// The ColorSpace Exif writes for sRGB.
const SRGB: u32 = 1;

/// The ColorSpace Exif writes for "uncalibrated": any color space but
/// sRGB, which the Interoperability IFD's InteropIndex may then name.
const UNCALIBRATED: u32 = 0xFFFF;

/// The InteropIndex of a DCF option file, which names Adobe RGB beside a
/// ColorSpace of [`UNCALIBRATED`].
///
/// Exif gives the index only the meaning "DCF option file"; that such a
/// file is in Adobe RGB is from ExifTool's published documentation of the
/// tag, read as documentation, and the camera files the JPEG format notes
/// checked agree (section 4 of those notes). ADR011 reads it so.
const DCF_OPTION_FILE: &str = "R03";

/// How the camera was set for the picture, as the Exif IFD `exif` records
/// it, with the Interoperability IFD `interop`, when the tree has one, for
/// the color space: the exposure program, the metering mode, the flash,
/// the white balance mode and the color space, as the
/// [module documentation](self) gives their rules.
///
/// The other settings, which only a maker's MakerNote records, are `None`,
/// for a format's MakerNote reader to fill.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
fn camera_settings<R: Read + Seek>(
    src: &mut Source<R>,
    exif: &Ifd,
    interop: Option<&Ifd>,
) -> Result<CameraSettings> {
    Ok(CameraSettings {
        white_balance: WhiteBalance {
            mode: white_balance_mode(exif),
            // Only a maker's MakerNote records the color temperature and
            // the fine-tuning: Nikon's and Sony's readers fill them.
            color_temperature: None,
            fine_tune: None,
        },
        exposure_program: exif
            .held(rules::EXPOSURE_PROGRAM)
            .and_then(|[code]| exposure_program(code)),
        metering_mode: exif
            .held(rules::METERING_MODE)
            .and_then(|[code]| metering_mode(code)),
        flash: exif.held(rules::FLASH).and_then(|[bits]| flash(bits)),
        // Only a maker's MakerNote records these: Nikon's reader fills
        // all but the electronic front curtain setting, which Nikon does
        // not record, and Sony's reader that setting alone, since Sony
        // records the others only in enciphered blocks or, for the raw
        // size, in a code that does not tell APS-C from medium.
        shutter: None,
        electronic_front_curtain_setting: None,
        image_area: None,
        raw_image_size: None,
        color_space: color_space(src, exif, interop)?,
    })
}

/// The exposure program an Exif ExposureProgram `code` names: 1 manual,
/// 2 program ("normal program"), 3 aperture priority, 4 shutter priority,
/// 5 creative, 6 action, 7 portrait and 8 landscape. `None` for 0, "not
/// defined", and for every code Exif does not define, such as 9.
fn exposure_program(code: u32) -> Option<ExposureProgram> {
    Some(match code {
        1 => ExposureProgram::Manual,
        2 => ExposureProgram::Program,
        3 => ExposureProgram::AperturePriority,
        4 => ExposureProgram::ShutterPriority,
        5 => ExposureProgram::Creative,
        6 => ExposureProgram::Action,
        7 => ExposureProgram::Portrait,
        8 => ExposureProgram::Landscape,
        _ => return None,
    })
}

/// The metering mode an Exif MeteringMode `code` names: 1 average, 2
/// center-weighted average, 3 spot, 4 multi-spot, 5 multi-segment (Exif's
/// "pattern"), 6 partial and 255 other. `None` for 0, "unknown", and for
/// every code Exif does not define.
fn metering_mode(code: u32) -> Option<MeteringMode> {
    Some(match code {
        1 => MeteringMode::Average,
        2 => MeteringMode::CenterWeighted,
        3 => MeteringMode::Spot,
        4 => MeteringMode::MultiSpot,
        5 => MeteringMode::MultiSegment,
        6 => MeteringMode::Partial,
        255 => MeteringMode::Other,
        _ => return None,
    })
}

/// The flash an Exif Flash value `bits` records, by its bits: bit 0
/// whether it fired, bits 3 and 4 its mode, and bit 5 that the camera has
/// no flash function. Bits 1 and 2, whether the flash's return light was
/// detected, and bit 6, red-eye reduction, are not read.
///
/// The mode is 1 [`Fill`](FlashMode::Fill) ("compulsory flash firing"), 2
/// [`Off`](FlashMode::Off) ("compulsory flash suppression") and 3
/// [`Auto`](FlashMode::Auto); 0, "unknown", gives no mode, and so does
/// bit 5, since a camera without a flash function has no flash mode.
///
/// `None` when any of the reserved bits 7 to 15 is set, which no camera
/// writes, and when bits 0 and 5 are both set, a flash that fired on a
/// camera without one: the value contradicts itself. `bits` is a SHORT
/// widened, so no bit above 15 is set.
fn flash(bits: u32) -> Option<Flash> {
    const FIRED: u32 = 1;
    const NO_FLASH_FUNCTION: u32 = 1 << 5;
    const RESERVED: u32 = 0xFF80;
    if bits & RESERVED != 0 {
        return None;
    }
    let fired = bits & FIRED != 0;
    if bits & NO_FLASH_FUNCTION != 0 {
        return (!fired).then_some(Flash::new(false));
    }
    let flash = Flash::new(fired);
    Some(match (bits >> 3) & 0b11 {
        1 => flash.with_mode(FlashMode::Fill),
        2 => flash.with_mode(FlashMode::Off),
        3 => flash.with_mode(FlashMode::Auto),
        _ => flash,
    })
}

/// The white balance mode the Exif IFD `exif` records: its WhiteBalance,
/// 0 for auto, and, when it is 1, "manual", the kind of light its
/// LightSource names ([`white_balance_of_light`]). Each must be exactly
/// one SHORT.
///
/// `None` when the WhiteBalance is missing, cannot be read or holds
/// another code, and for a manual white balance whose LightSource is
/// missing, cannot be read or names no mode.
fn white_balance_mode(exif: &Ifd) -> Option<WhiteBalanceMode> {
    match exif.held(rules::WHITE_BALANCE) {
        Some([0]) => Some(WhiteBalanceMode::Auto),
        Some([1]) => exif
            .held(rules::LIGHT_SOURCE)
            .and_then(|[code]| white_balance_of_light(code)),
        _ => None,
    }
}

/// The white balance mode a manual white balance set for the light an
/// Exif LightSource `code` names: 1 daylight and 9 fine weather give
/// [`Daylight`](WhiteBalanceMode::Daylight), 10 cloudy weather
/// [`Cloudy`](WhiteBalanceMode::Cloudy), 11 shade
/// [`Shade`](WhiteBalanceMode::Shade), 3 tungsten and 24 ISO studio
/// tungsten [`Tungsten`](WhiteBalanceMode::Tungsten), 2 fluorescent and 12
/// to 16, its five kinds of lamp,
/// [`Fluorescent`](WhiteBalanceMode::Fluorescent), and 4 flash
/// [`Flash`](WhiteBalanceMode::Flash).
///
/// `None` for every other code: 0, "unknown"; 17 to 23, the standard
/// lights A to D50, and 25 to 34, light sources and LEDs by color
/// temperature, which name a light no mode stands for; 255, "other"; and
/// every code Exif does not define.
fn white_balance_of_light(code: u32) -> Option<WhiteBalanceMode> {
    Some(match code {
        1 | 9 => WhiteBalanceMode::Daylight,
        10 => WhiteBalanceMode::Cloudy,
        11 => WhiteBalanceMode::Shade,
        3 | 24 => WhiteBalanceMode::Tungsten,
        2 | 12..=16 => WhiteBalanceMode::Fluorescent,
        4 => WhiteBalanceMode::Flash,
        _ => return None,
    })
}

/// The color space the Exif IFD `exif` records in its ColorSpace, exactly
/// one SHORT: [`SRGB`] is sRGB, and [`UNCALIBRATED`] is
/// Adobe RGB when the Interoperability IFD `interop` holds the
/// InteropIndex [`DCF_OPTION_FILE`], read as every text is ([`text`]).
///
/// `None` when the ColorSpace is missing, cannot be read or holds another
/// code, such as 2, which Exif does not define; and for an "uncalibrated"
/// one without an Interoperability IFD, or whose InteropIndex is missing,
/// cannot be read or is another, such as `R98`, a DCF basic file. The
/// InteropIndex is read only for an "uncalibrated" ColorSpace: with sRGB,
/// it changes nothing.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn color_space<R: Read + Seek>(
    src: &mut Source<R>,
    exif: &Ifd,
    interop: Option<&Ifd>,
) -> Result<Option<ColorSpace>> {
    match exif.held(rules::COLOR_SPACE) {
        Some([SRGB]) => Ok(Some(ColorSpace::Srgb)),
        Some([UNCALIBRATED]) => {
            let Some(interop) = interop else {
                return Ok(None);
            };
            let index = text(src, interop, rules::INTEROP_INDEX)?;
            Ok((index.as_deref() == Some(DCF_OPTION_FILE)).then_some(ColorSpace::AdobeRgb))
        }
        _ => Ok(None),
    }
}

/// The raw size and the default crop of the file whose IFDs are `tree`:
/// both from the largest IFD that holds raw data and says how large it is,
/// or both `None` when no IFD does.
///
/// The IFDs are asked in the tree's order, except the Exif IFD, which holds
/// capture metadata, the GPS IFD, which holds where the picture was taken,
/// and the Interoperability IFD, which names the rule the file follows:
/// none of them ever holds an image. An IFD counts as raw data
/// when the preview finder says so ([`tiff_previews::is_raw`]); one whose
/// tags that test reads cannot be read is passed over, as the finder passes
/// over it. A raw IFD counts only when [`raw_size`] reads a size from it.
/// Of those, the one with the most pixels gives the raw size, the first on
/// a tie: the Coolpix E5700 (public sample 4456) writes a 656 by 484 raw
/// image before its 2576 by 1924 one, both full-resolution. The crop is
/// then that IFD's [`default_crop`], `None` when it has none.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason costs only what it would have given.
fn sizes<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<(Option<Size>, Option<Size>)> {
    let mut largest: Option<(&Ifd, Size)> = None;
    for (link, ifd) in tree.iter() {
        if link.holds_no_image() {
            continue;
        }
        if skip_damage(tiff_previews::is_raw(src, ifd))? != Some(true) {
            continue;
        }
        let Some(raw) = raw_size(src, ifd)? else {
            continue;
        };
        // Only a larger one replaces it, so the first wins a tie.
        if largest.is_none_or(|(_, best)| raw.area() > best.area()) {
            largest = Some((ifd, raw));
        }
    }
    let Some((ifd, raw)) = largest else {
        return Ok((None, None));
    };
    Ok((Some(raw), default_crop(src, ifd, raw)?))
}

/// The size of the raw data `ifd` describes: its ImageWidth and
/// ImageLength, each of type SHORT, LONG or IFD, each its first value,
/// read alone whatever the count.
/// `None` when either is missing, cannot be read or is zero.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn raw_size<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd) -> Result<Option<Size>> {
    let width = ifd.get(src, rules::IMAGE_WIDTH)?;
    let height = ifd.get(src, rules::IMAGE_LENGTH)?;
    Ok(width
        .zip(height)
        .filter(|&(width, height)| width > 0 && height > 0)
        .map(|(width, height)| Size { width, height }))
}

/// The visible image of the raw data `ifd` describes, whose size is `raw`:
/// the size its DefaultCropSize gives, when the crop can be trusted.
///
/// The crop needs both its DefaultCropOrigin, the column and the row where
/// it starts, and its DefaultCropSize, its width and height, each exactly
/// two whole numbers as [`whole_pair`] reads them, and it must be one that
/// [`crop`] accepts. `None` otherwise, for the caller to take the raw size.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn default_crop<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd, raw: Size) -> Result<Option<Size>> {
    let origin = (
        rules::DEFAULT_CROP_ORIGIN,
        rules::DEFAULT_CROP_ORIGIN_FRACTIONS,
    );
    let Some([x, y]) = whole_pair(src, ifd, origin)? else {
        return Ok(None);
    };
    let size = (rules::DEFAULT_CROP_SIZE, rules::DEFAULT_CROP_SIZE_FRACTIONS);
    let Some(size) = whole_pair(src, ifd, size)? else {
        return Ok(None);
    };
    Ok(crop([x, y], size, raw))
}

/// The size of the crop whose first column and row are `x` and `y` and
/// whose size is `width` by `height`, in raw data of size `raw`, when the
/// crop can be trusted: its size is above zero, and it lies inside the raw
/// data, the start plus the size, in each direction, at most the raw size,
/// a sum that overflows being outside. `None` otherwise.
///
/// The one test for every crop a file records, the DNG default crop here
/// and a format's own, such as Nikon's crop area.
pub(crate) fn crop([x, y]: [u32; 2], [width, height]: [u32; 2], raw: Size) -> Option<Size> {
    let inside = |start: u32, length: u32, raw: u32| {
        length > 0 && start.checked_add(length).is_some_and(|end| end <= raw)
    };
    (inside(x, width, raw.width) && inside(y, height, raw.height)).then_some(Size { width, height })
}

/// The two whole numbers a tag of `ifd` holds, in the forms the DNG
/// specification allows its crop tags, each with its rule: two integers
/// of type SHORT or LONG, or IFD, a LONG under another name, by the first
/// rule of `rules`; or two fractions of type RATIONAL whose every
/// denominator is 1, by the second. `None` when the tag is missing, cannot
/// be read, holds a fraction with another denominator, or holds other
/// than two values, which is not read.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn whole_pair<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    (integers, fractions): (Rule<[u32; 2]>, Rule<[Rational; 2]>),
) -> Result<Option<[u32; 2]>> {
    if let Some(integers) = ifd.get(src, integers)? {
        return Ok(Some(integers));
    }
    let Some([first, second]) = ifd.get(src, fractions)? else {
        return Ok(None);
    };
    let whole = |fraction: Rational| (fraction.denominator == 1).then_some(fraction.numerator);
    Ok(whole(first)
        .zip(whole(second))
        .map(|(first, second)| [first, second]))
}

#[cfg(test)]
mod tests {
    use super::*;
    // The fixtures name each tag by its number, as a file stores it; the
    // reader above names it by its rule.
    use crate::container::tiff::tags::*;
    use crate::container::tiff::{Block, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing;
    use crate::limits::Limits;
    use crate::orientation::Orientation;
    use crate::testing::{FAR, FailsFar, size};
    use crate::tiff_builder::{
        ASCII, IfdId, LONG, Order, RATIONAL, RawValue, SHORT, SRATIONAL, TiffBuilder, UNDEFINED,
        UTF8,
    };
    use std::io::Cursor;

    /// When the picture was stored or digitized (DateTimeDigitized), an
    /// Exif tag the reader must never take the capture time from.
    const DATE_TIME_DIGITIZED: u16 = 0x9004;

    /// When the file was last changed (DateTime), an IFD0 tag the reader
    /// must never take the capture time from.
    const DATE_TIME: u16 = 0x0132;

    /// The text values of a Z 9 file, each with its tag: IFD0's, then the
    /// Exif IFD's. The software, the time and the lens are those of the
    /// public Z 9 sample 5147; the serial numbers are made up, the lens's
    /// with the two spaces the Z 9 pads its eight digits with.
    const IFD0_TEXT: [(u16, &str); 3] = [
        (MAKE, "NIKON CORPORATION"),
        (MODEL, "NIKON Z 9"),
        (SOFTWARE, "Ver.01.00"),
    ];
    /// See [`IFD0_TEXT`]. The LensModel is stored padded, as
    /// [`z9_bytes_except`] says.
    const EXIF_TEXT: [(u16, &str); 7] = [
        (BODY_SERIAL_NUMBER, "1234567"),
        (DATE_TIME_ORIGINAL, "2022:02:05 13:46:41"),
        (SUB_SEC_TIME_ORIGINAL, "85"),
        (OFFSET_TIME_ORIGINAL, "-05:00"),
        (LENS_MAKE, "NIKON"),
        (LENS_MODEL, "NIKKOR Z 14-30mm f/4 S"),
        (LENS_SERIAL_NUMBER, "12345678  "),
    ];

    /// The Z 9 file's LensSpecification: 14-30 mm, f/4 at both ends, as
    /// the public sample 5147 records it.
    const Z9_LENS_SPECIFICATION: [(u32, u32); 4] = [(140, 10), (300, 10), (400, 100), (400, 100)];

    /// How many bytes, with the NULs that end and pad it, the Z 9 stores
    /// its LensModel in: 65 on the public sample 5147, whatever the name.
    const Z9_LENS_MODEL_COUNT: usize = 65;

    /// `model` as the Z 9 stores a LensModel: followed by NULs up to
    /// [`Z9_LENS_MODEL_COUNT`] bytes, less the NUL the builder adds.
    fn padded_lens_model(model: &str) -> String {
        let mut padded = model.to_owned();
        padded.extend(std::iter::repeat_n(
            '\0',
            Z9_LENS_MODEL_COUNT - 1 - model.len(),
        ));
        padded
    }

    /// What adds one exposure value to the Exif IFD: given the builder,
    /// the Exif IFD and the value's tag.
    type AddExposure = fn(&mut TiffBuilder, IfdId, u16);

    /// The exposure values of the Z 9 file's Exif IFD, each with its tag
    /// and what adds it: those of the public Z 9 sample 5147, each of the
    /// type the Z 9 writes.
    const EXIF_EXPOSURE: [(u16, AddExposure); 7] = [
        (EXPOSURE_TIME, |builder, exif, tag| {
            builder.rational(exif, tag, &[(10, 3200)]);
        }),
        (F_NUMBER, |builder, exif, tag| {
            builder.rational(exif, tag, &[(900, 100)]);
        }),
        (PHOTOGRAPHIC_SENSITIVITY, |builder, exif, tag| {
            builder.short(exif, tag, &[64]);
        }),
        (RECOMMENDED_EXPOSURE_INDEX, |builder, exif, tag| {
            builder.long(exif, tag, &[64]);
        }),
        (EXPOSURE_BIAS_VALUE, |builder, exif, tag| {
            builder.srational(exif, tag, &[(0, 6)]);
        }),
        (FOCAL_LENGTH, |builder, exif, tag| {
            builder.rational(exif, tag, &[(180, 10)]);
        }),
        (FOCAL_LENGTH_IN_35MM_FILM, |builder, exif, tag| {
            builder.short(exif, tag, &[18]);
        }),
    ];

    /// The camera settings of the Z 9 file's Exif IFD, each with its tag,
    /// each one SHORT: those of the public Z 9 sample 5147, the program 2,
    /// center-weighted metering, a flash that did not fire and recorded no
    /// mode, and auto white balance, with the LightSource 0, "unknown",
    /// beside it. Like every Nikon raw file checked, the sample records no
    /// ColorSpace.
    const EXIF_SETTINGS: [(u16, u16); 5] = [
        (EXPOSURE_PROGRAM, 2),
        (METERING_MODE, 2),
        (FLASH, 0),
        (WHITE_BALANCE, 0),
        (LIGHT_SOURCE, 0),
    ];

    /// The camera settings of the Z 9 file, as [`EXIF_SETTINGS`] records
    /// them.
    fn z9_settings() -> CameraSettings {
        CameraSettings {
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some(Flash::new(false)),
            white_balance: WhiteBalance {
                mode: Some(WhiteBalanceMode::Auto),
                ..WhiteBalance::default()
            },
            ..CameraSettings::default()
        }
    }

    /// More bytes than the 1 MiB one value may take under the default
    /// limits.
    const TOO_LONG: u32 = 1024 * 1024 + 1;

    /// A little-endian file whose IFD0 holds the Orientation 8 and
    /// [`IFD0_TEXT`], and whose Exif IFD holds [`EXIF_TEXT`], with the
    /// LensModel padded by [`padded_lens_model`], [`EXIF_EXPOSURE`],
    /// [`EXIF_SETTINGS`] and [`Z9_LENS_SPECIFICATION`], all but tag
    /// `except`. `change` is then given the builder and the IDs of IFD0 and
    /// of the Exif IFD, to add what the test needs in its place.
    fn z9_bytes_except(
        except: u16,
        change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
    ) -> Vec<u8> {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder.exif(ifd0, exif).short(ifd0, ORIENTATION, &[8]);
        for (ifd, values) in [(ifd0, &IFD0_TEXT[..]), (exif, &EXIF_TEXT[..])] {
            for &(tag, text) in values.iter().filter(|(tag, _)| *tag != except) {
                if tag == LENS_MODEL {
                    builder.ascii(ifd, tag, &padded_lens_model(text));
                } else {
                    builder.ascii(ifd, tag, text);
                }
            }
        }
        for &(tag, add) in EXIF_EXPOSURE.iter().filter(|(tag, _)| *tag != except) {
            add(&mut builder, exif, tag);
        }
        for &(tag, code) in EXIF_SETTINGS.iter().filter(|(tag, _)| *tag != except) {
            builder.short(exif, tag, &[code]);
        }
        if except != LENS_SPECIFICATION {
            builder.rational(exif, LENS_SPECIFICATION, &Z9_LENS_SPECIFICATION);
        }
        change(&mut builder, ifd0, exif);
        builder.build().bytes
    }

    /// The metadata read from `bytes`; the reader must succeed.
    fn read_bytes(bytes: Vec<u8>) -> Metadata {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        read(&mut src, &tree).unwrap().metadata
    }

    /// The metadata of [`z9_bytes_except`]'s file.
    fn z9_except(except: u16, change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId)) -> Metadata {
        read_bytes(z9_bytes_except(except, change))
    }

    /// The metadata of the Z 9 file whole: tag 0, which it does not hold,
    /// is the one left out.
    fn z9_file() -> Metadata {
        z9_except(0, |_, _, _| {})
    }

    /// The capture time of the Z 9 file, with the fraction of a second and
    /// the offset from UTC each when `Some`.
    fn z9_time(subsec: Option<&[u8]>, offset: Option<&[u8]>) -> Option<DateTime> {
        DateTime::from_exif(b"2022:02:05 13:46:41", subsec, offset)
    }

    /// The fraction `numerator`/`denominator`, as stored.
    fn fraction(numerator: u32, denominator: u32) -> Rational {
        Rational {
            numerator,
            denominator,
        }
    }

    /// The signed fraction `numerator`/`denominator`, as stored.
    fn signed(numerator: i32, denominator: i32) -> SignedRational {
        SignedRational {
            numerator,
            denominator,
        }
    }

    /// The exposure of the Z 9 file, as [`EXIF_EXPOSURE`] records it:
    /// 1/320 s at f/9, 18 mm, 18 mm in 35 mm terms too, and ISO 64, with no
    /// compensation.
    fn z9_exposure() -> Exposure {
        Exposure {
            iso: Some(64),
            exposure_time: Some(fraction(10, 3200)),
            f_number: Some(fraction(900, 100)),
            focal_length: Some(fraction(180, 10)),
            focal_length_in_35mm_film: Some(18),
            exposure_bias: Some(signed(0, 6)),
        }
    }

    /// The lens of the Z 9 file, as [`EXIF_TEXT`] and
    /// [`Z9_LENS_SPECIFICATION`] record it, with `change` made to its
    /// values.
    fn z9_lens_with(change: impl FnOnce(&mut LensValues)) -> Lens {
        let mut values = LensValues {
            recorded_make: Some("NIKON".to_owned()),
            recorded_model: Some("NIKKOR Z 14-30mm f/4 S".to_owned()),
            serial_number: Some("12345678".to_owned()),
            range: [
                Some(fraction(140, 10)),
                Some(fraction(300, 10)),
                Some(fraction(400, 100)),
                Some(fraction(400, 100)),
            ],
        };
        change(&mut values);
        values.lens()
    }

    /// The lens of the Z 9 file, every value read.
    fn z9_lens() -> Lens {
        z9_lens_with(|_| {})
    }

    /// The values a [`Lens`] is built from, for a test to change one.
    struct LensValues {
        recorded_make: Option<String>,
        recorded_model: Option<String>,
        serial_number: Option<String>,
        /// The shortest and longest focal length, then the f-number at
        /// each.
        range: [Option<Rational>; 4],
    }

    impl LensValues {
        /// No value: the values of a file that describes no lens.
        fn none() -> Self {
            Self {
                recorded_make: None,
                recorded_model: None,
                serial_number: None,
                range: [None; 4],
            }
        }

        /// The lens these values describe.
        fn lens(self) -> Lens {
            let mut lens = Lens {
                recorded_make: self.recorded_make,
                recorded_model: self.recorded_model,
                serial_number: self.serial_number,
                ..Lens::default()
            };
            lens.set_range(self.range);
            lens
        }
    }

    /// The camera the Z 9 file names: its Make, its Model and its serial
    /// number.
    fn z9_camera() -> Camera {
        Camera {
            recorded_make: Some("NIKON CORPORATION".to_owned()),
            recorded_model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("1234567".to_owned()),
            owner_name: None,
        }
    }

    /// What the Z 9 file gives, every value read.
    fn z9_expected() -> Metadata {
        Metadata {
            camera: z9_camera(),
            software: Some("Ver.01.00".to_owned()),
            capture_time: z9_time(Some(b"85"), Some(b"-05:00")),
            orientation: Orientation::Rotate270,
            exposure: z9_exposure(),
            camera_settings: z9_settings(),
            lens: z9_lens(),
            ..Metadata::default()
        }
    }

    /// The Z 9 file's metadata with the lens `lens` instead of its own.
    fn z9_expected_with_lens(lens: Lens) -> Metadata {
        Metadata {
            lens,
            ..z9_expected()
        }
    }

    /// The lens read from a file whose Exif IFD holds only what `build`
    /// adds, given the builder and the Exif IFD's ID.
    fn lens(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> Lens {
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            build(builder, exif);
        });
        read(&mut src, &tree).unwrap().metadata.lens
    }

    /// The Z 9 file's metadata with the exposure `exposure` instead of its
    /// own.
    fn z9_expected_with(exposure: Exposure) -> Metadata {
        Metadata {
            exposure,
            ..z9_expected()
        }
    }

    /// The exposure read from a file whose Exif IFD holds only what `build`
    /// adds, given the builder and the Exif IFD's ID.
    fn exposure(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> Exposure {
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            build(builder, exif);
        });
        read(&mut src, &tree).unwrap().metadata.exposure
    }

    /// The ISO read from a file whose Exif IFD holds only what `build` adds,
    /// as [`exposure`] builds it.
    fn iso(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> Option<u32> {
        exposure(build).iso
    }

    /// The make read from a file whose IFD0's Make is `make`, as ASCII with
    /// the NUL the builder adds.
    fn make(make: &str) -> Option<String> {
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        });
        read(&mut src, &tree).unwrap().metadata.camera.recorded_make
    }

    #[test]
    fn the_z9_file_gives_every_value_it_records() {
        let metadata = z9_file();
        assert_eq!(metadata, z9_expected());
        assert_eq!(metadata.software(), Some("Ver.01.00"));
        let time = metadata.capture_time().unwrap();
        assert_eq!(time.to_string(), "2022-02-05T13:46:41.85-05:00");
        assert_eq!(time.offset_minutes(), Some(-300));
        assert_eq!(time.nanosecond(), 850_000_000);
        let exposure = metadata.exposure();
        assert_eq!(exposure.iso(), Some(64));
        assert_eq!(exposure.exposure_time(), Some(fraction(10, 3200)));
        assert_eq!(exposure.f_number(), Some(fraction(900, 100)));
        assert_eq!(exposure.focal_length(), Some(fraction(180, 10)));
        assert_eq!(exposure.focal_length_in_35mm_film(), Some(18));
        assert_eq!(exposure.exposure_bias(), Some(signed(0, 6)));
        let settings = metadata.camera_settings();
        assert_eq!(settings.exposure_program(), Some(ExposureProgram::Program));
        assert_eq!(settings.metering_mode(), Some(MeteringMode::CenterWeighted));
        assert_eq!(settings.flash(), Some(Flash::new(false)));
        assert_eq!(
            settings.white_balance().mode(),
            Some(WhiteBalanceMode::Auto)
        );
        // No ColorSpace, and what only a MakerNote records is not read.
        assert_eq!(settings.color_space(), None);
        assert_eq!(settings.white_balance().color_temperature(), None);
        assert_eq!(settings.shutter(), None);
        let lens = metadata.lens();
        assert_eq!(lens.recorded_make(), Some("NIKON"));
        // Read whole, without the NULs that pad it, nor the spaces that pad
        // the serial number.
        assert_eq!(lens.recorded_model(), Some("NIKKOR Z 14-30mm f/4 S"));
        assert_eq!(lens.serial_number(), Some("12345678"));
        assert_eq!(lens.min_focal_length(), Some(fraction(140, 10)));
        assert_eq!(lens.max_focal_length(), Some(fraction(300, 10)));
        assert_eq!(
            lens.f_number_at_min_focal_length(),
            Some(fraction(400, 100))
        );
        assert_eq!(
            lens.f_number_at_max_focal_length(),
            Some(fraction(400, 100))
        );
        // This file holds no IFD of raw data, so it has no sizes; the
        // tests of the sizes build files that do.
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), None);
    }

    #[test]
    fn a_make_padded_with_spaces_is_trimmed() {
        // The DSLR-A100's: `SONY` and eleven spaces.
        assert_eq!(make("SONY           ").as_deref(), Some("SONY"));
        assert_eq!(
            make("  NIKON CORPORATION ").as_deref(),
            Some("NIKON CORPORATION")
        );
    }

    #[test]
    fn only_spaces_are_padding() {
        // A tab or a line break is not padding, at an end or inside.
        assert_eq!(make(" \tSONY\r\n ").as_deref(), Some("\tSONY\r\n"));
    }

    #[test]
    fn spaces_inside_a_make_are_kept() {
        assert_eq!(
            make("NIKON  CORPORATION").as_deref(),
            Some("NIKON  CORPORATION")
        );
    }

    #[test]
    fn a_make_is_cut_at_its_first_nul() {
        // A space, then a NUL, then bytes past it, then the builder's NUL.
        assert_eq!(make("SONY \0rest of the value").as_deref(), Some("SONY"));
    }

    #[test]
    fn a_make_of_nul_bytes_or_spaces_only_is_none() {
        assert_eq!(make("\0\0\0"), None);
        assert_eq!(make("    "), None);
        assert_eq!(make(""), None);
    }

    /// The metadata of the Z 9 file with IFD0's Software `software`, as
    /// ASCII with the NUL the builder adds, in place of its own.
    fn z9_with_software(software: &str) -> Metadata {
        z9_except(SOFTWARE, |builder, ifd0, _| {
            builder.ascii(ifd0, SOFTWARE, software);
        })
    }

    #[test]
    fn a_software_is_read_as_text_trimmed_of_its_padding() {
        // The Nikon DSLRs' trailing space, as the D300's `Ver.1.10 ` on the
        // public sample 928.
        assert_eq!(
            z9_with_software("Ver.1.10 "),
            Metadata {
                software: Some("Ver.1.10".to_owned()),
                ..z9_expected()
            }
        );
        // The two spaces inside the Coolpix P7800's are kept.
        assert_eq!(
            z9_with_software("COOLPIX P7800  V1.0").software(),
            Some("COOLPIX P7800  V1.0")
        );
    }

    #[test]
    fn a_software_of_nul_bytes_or_spaces_only_is_none() {
        for software in ["\0\0\0", "    ", ""] {
            assert_eq!(
                z9_with_software(software),
                Metadata {
                    software: None,
                    ..z9_expected()
                },
                "{software:?}"
            );
        }
    }

    #[test]
    fn the_software_is_read_from_ifd0_alone() {
        // A Software in the Exif IFD, where it does not belong, is not read.
        let metadata = z9_except(SOFTWARE, |builder, _, exif| {
            builder.ascii(exif, SOFTWARE, "Ver.01.00");
        });
        assert_eq!(
            metadata,
            Metadata {
                software: None,
                ..z9_expected()
            }
        );
    }

    #[test]
    fn a_model_of_256_bytes_is_refused_and_one_of_255_kept() {
        let model = |len: usize| {
            z9_except(MODEL, |builder, ifd0, _| {
                builder.ascii(ifd0, MODEL, &"M".repeat(len));
            })
        };
        assert_eq!(model(255).camera.recorded_model, Some("M".repeat(255)));
        // Too long, and costs only the model.
        assert_eq!(
            model(256),
            Metadata {
                camera: Camera {
                    recorded_model: None,
                    ..z9_camera()
                },
                ..z9_expected()
            }
        );
    }

    #[test]
    fn the_length_limit_counts_the_padding() {
        // 256 bytes, which trimming would make 4.
        let padded = format!("SONY{}", " ".repeat(252));
        assert_eq!(make(&padded), None);
    }

    /// The make of a little-endian file whose IFD0's Make is `bytes`, of
    /// type ASCII, stored at offset 4096.
    fn make_bytes(bytes: &[u8]) -> Option<String> {
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.blob_at(4096, bytes);
            let count = u32::try_from(bytes.len()).unwrap();
            builder.raw(ifd, MAKE, ASCII, count, RawValue::Offset(4096));
        });
        read(&mut src, &tree).unwrap().metadata.camera.recorded_make
    }

    /// Where a text is read from: IFD0 or the Exif IFD.
    #[derive(Clone, Copy, Debug)]
    enum In {
        Ifd0,
        Exif,
    }

    /// Reads a text of the metadata, for the tests that check each text by
    /// the same rule.
    type GetText = fn(&Metadata) -> Option<&str>;

    /// The texts that Exif 3.1 gives as "ASCII or UTF-8" and the reader
    /// holds to 255 bytes, each with its tag, where it is read from and
    /// what reads it back. The description, of either type too, is free
    /// text, and is checked beside them where the rules agree.
    const ASCII_OR_UTF8_TEXTS: [(u16, In, GetText); 11] = [
        (MAKE, In::Ifd0, |m| m.camera().recorded_make()),
        (MODEL, In::Ifd0, |m| m.camera().recorded_model()),
        (SOFTWARE, In::Ifd0, |m| m.software()),
        (LENS_MAKE, In::Exif, |m| m.lens().recorded_make()),
        (LENS_MODEL, In::Exif, |m| m.lens().recorded_model()),
        (ARTIST, In::Ifd0, |m| m.authorship().artist()),
        (COPYRIGHT, In::Ifd0, |m| m.authorship().copyright()),
        (PHOTOGRAPHER, In::Exif, |m| m.authorship().photographer()),
        (IMAGE_EDITOR, In::Exif, |m| m.authorship().editor()),
        (IMAGE_TITLE, In::Exif, |m| m.authorship().title()),
        (CAMERA_OWNER_NAME, In::Exif, |m| m.camera().owner_name()),
    ];

    /// The description, with its tag, where it is read from and what
    /// reads it back, as [`ASCII_OR_UTF8_TEXTS`] lists the others.
    const DESCRIPTION: (u16, In, GetText) = (IMAGE_DESCRIPTION, In::Ifd0, |m| {
        m.authorship().description()
    });

    /// The texts Exif 3.1 gives as ASCII alone that the reader reads from
    /// the standard tags, each with its tag and what reads it back: the
    /// serial numbers. The capture time, ASCII alone too, is checked on its
    /// own.
    const ASCII_ONLY_TEXTS: [(u16, GetText); 2] = [
        (BODY_SERIAL_NUMBER, |m| m.camera().serial_number()),
        (LENS_SERIAL_NUMBER, |m| m.lens().serial_number()),
    ];

    /// The metadata of a file whose IFD or Exif IFD, as `at` says, holds
    /// what `add` adds, given the builder and that IFD's ID.
    fn with_text(at: In, add: impl FnOnce(&mut TiffBuilder, IfdId)) -> Metadata {
        let (mut src, tree) = testing::file(|builder, ifd0| {
            let exif = builder.ifd();
            builder.exif(ifd0, exif);
            add(builder, if matches!(at, In::Ifd0) { ifd0 } else { exif });
        });
        read(&mut src, &tree).unwrap().metadata
    }

    #[test]
    fn a_text_exif_gives_as_ascii_or_utf8_is_read_from_either_type() {
        // A name with letters past ASCII, which only UTF-8 can hold, padded
        // as the cameras pad their ASCII texts.
        for (tag, at, get) in ASCII_OR_UTF8_TEXTS.into_iter().chain([DESCRIPTION]) {
            let ascii = with_text(at, |builder, ifd| {
                builder.ascii(ifd, tag, "  Zeiss Ikon ");
            });
            assert_eq!(get(&ascii), Some("Zeiss Ikon"), "{tag:#06X}");
            let utf8 = with_text(at, |builder, ifd| {
                builder.utf8(ifd, tag, "  Zeiß Ikon \0\0");
            });
            assert_eq!(get(&utf8), Some("Zeiß Ikon"), "{tag:#06X}");
        }
    }

    #[test]
    fn a_text_of_another_type_is_none() {
        for (tag, at, get) in ASCII_OR_UTF8_TEXTS.into_iter().chain([DESCRIPTION]) {
            for metadata in [
                with_text(at, |builder, ifd| {
                    builder.undefined(ifd, tag, b"Zeiss Ikon\0");
                }),
                with_text(at, |builder, ifd| {
                    builder.byte(ifd, tag, b"Zeiss Ikon\0");
                }),
            ] {
                assert_eq!(get(&metadata), None, "{tag:#06X}");
            }
        }
    }

    #[test]
    fn a_text_keeps_the_255_byte_limit_counted_as_stored() {
        for (tag, at, get) in ASCII_OR_UTF8_TEXTS {
            let ascii = |len| {
                with_text(at, |builder, ifd| {
                    builder.ascii(ifd, tag, &"M".repeat(len));
                })
            };
            assert_eq!(
                get(&ascii(255)),
                Some("M".repeat(255).as_str()),
                "{tag:#06X}"
            );
            assert_eq!(get(&ascii(256)), None, "{tag:#06X}");
            // "é" is two bytes: 127 of them and one more byte are 255.
            let kept = format!("{}e", "é".repeat(127));
            let metadata = with_text(at, |builder, ifd| {
                builder.utf8(ifd, tag, &kept);
            });
            assert_eq!(get(&metadata), Some(kept.as_str()), "{tag:#06X}");
            let refused = "é".repeat(128);
            let metadata = with_text(at, |builder, ifd| {
                builder.utf8(ifd, tag, &refused);
            });
            assert_eq!(get(&metadata), None, "{tag:#06X}");
        }
    }

    /// The authorship read from a file whose IFD0 holds what `ifd0` adds
    /// and whose Exif IFD holds what `exif` adds, given the builder and
    /// that IFD's ID.
    fn authorship_of(
        ifd0: impl FnOnce(&mut TiffBuilder, IfdId),
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Authorship {
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif_ifd = builder.ifd();
            builder.exif(ifd, exif_ifd);
            ifd0(builder, ifd);
            exif(builder, exif_ifd);
        });
        read(&mut src, &tree).unwrap().metadata.authorship
    }

    /// The artist read from a file whose IFD0's Artist is `artist`, as
    /// ASCII with the NUL the builder adds.
    fn artist(artist: &str) -> Option<String> {
        authorship_of(
            |builder, ifd0| {
                builder.ascii(ifd0, ARTIST, artist);
            },
            |_, _| {},
        )
        .artist
    }

    /// The description read from a file whose IFD0's ImageDescription is
    /// `description`, as ASCII with the NUL the builder adds.
    fn description_of(description: &str) -> Option<String> {
        authorship_of(
            |builder, ifd0| {
                builder.ascii(ifd0, IMAGE_DESCRIPTION, description);
            },
            |_, _| {},
        )
        .description
    }

    /// The comment read from a file whose Exif IFD's UserComment is
    /// `bytes`, of type UNDEFINED.
    fn comment_of(bytes: &[u8]) -> Option<String> {
        authorship_of(
            |_, _| {},
            |builder, exif| {
                builder.undefined(exif, USER_COMMENT, bytes);
            },
        )
        .comment
    }

    /// `text` after the code `code`, padded with `pad` to 36 bytes, as
    /// Nikon stores a UserComment in 44 bytes.
    fn nikon_comment(code: &[u8; 8], text: &str, pad: u8) -> Vec<u8> {
        let mut bytes = code.to_vec();
        bytes.extend_from_slice(text.as_bytes());
        bytes.resize(44, pad);
        bytes
    }

    #[test]
    fn a_file_with_every_authorship_text_gives_each_in_its_place() {
        let authorship = authorship_of(
            |builder, ifd0| {
                builder
                    .ascii(ifd0, ARTIST, "Claude Monet")
                    .ascii(ifd0, COPYRIGHT, "Copyright Claude Monet")
                    .ascii(ifd0, IMAGE_DESCRIPTION, "The port of Le Havre");
            },
            |builder, exif| {
                builder
                    .ascii(exif, PHOTOGRAPHER, "Camille Pissarro")
                    .ascii(exif, IMAGE_EDITOR, "Alfred Sisley")
                    .utf8(exif, IMAGE_TITLE, "Impression, soleil levant")
                    .undefined(exif, USER_COMMENT, b"ASCII\0\0\0Painted at dawn");
            },
        );
        assert_eq!(
            authorship,
            Authorship::default()
                .with_artist("Claude Monet")
                .with_copyright("Copyright Claude Monet")
                .with_photographer("Camille Pissarro")
                .with_editor("Alfred Sisley")
                .with_title("Impression, soleil levant")
                .with_description("The port of Le Havre")
                .with_comment("Painted at dawn")
        );
    }

    #[test]
    fn an_artist_is_read_from_the_fields_nikon_and_sony_write() {
        // Nikon's 37 bytes, with the builder's NUL: unset, a NUL first or
        // all spaces; set, the text padded with spaces.
        let nikon = |text: &str| format!("{text:<36}");
        assert_eq!(artist(&nikon("\0")), None);
        assert_eq!(artist(&nikon("")), None);
        assert_eq!(
            artist(&nikon("Claude Monet")).as_deref(),
            Some("Claude Monet")
        );
        // Sony's: exactly the text and its NUL.
        assert_eq!(artist("Claude Monet").as_deref(), Some("Claude Monet"));
        // A painter's name past ASCII, in UTF-8.
        let utf8 = authorship_of(
            |builder, ifd0| {
                builder.utf8(ifd0, ARTIST, "Édouard Manet");
            },
            |_, _| {},
        );
        assert_eq!(utf8.artist(), Some("Édouard Manet"));
    }

    #[test]
    fn a_description_is_free_text_with_its_line_breaks() {
        let long = "A".repeat(300);
        assert_eq!(description_of(&long), Some(long.clone()));
        let utf8 = authorship_of(
            |builder, ifd0| {
                builder.utf8(ifd0, IMAGE_DESCRIPTION, &"é".repeat(300));
            },
            |_, _| {},
        );
        assert_eq!(utf8.description(), Some("é".repeat(300).as_str()));
        assert_eq!(
            description_of("  The port of Le Havre\r\nat sunrise\n  ").as_deref(),
            Some("The port of Le Havre\r\nat sunrise\n")
        );
        // Newer Sony bodies' 32 spaces are no text.
        assert_eq!(description_of(&" ".repeat(31)), None);
    }

    #[test]
    fn a_cameras_placeholder_is_no_description() {
        // In 9 bytes, as the DSLR-A200 to A900 write it, and padded with
        // spaces to 32, as the DSLR-A100, the NEX and SLT bodies do.
        assert_eq!(description_of("SONY DSC"), None);
        assert_eq!(description_of(&format!("{:<31}", "SONY DSC")), None);
        // Whatever the Make: the file says nothing of it here.
        let in_utf8 = authorship_of(
            |builder, ifd0| {
                builder.utf8(ifd0, IMAGE_DESCRIPTION, "SONY DSC");
            },
            |_, _| {},
        );
        assert_eq!(in_utf8.description(), None);
        // Matched whole and with its case: inside a longer text, or in
        // another case, it is a description.
        assert_eq!(
            description_of("SONY DSC The port of Le Havre").as_deref(),
            Some("SONY DSC The port of Le Havre")
        );
        assert_eq!(description_of("Sony DSC").as_deref(), Some("Sony DSC"));
    }

    #[test]
    fn ifd0s_texts_are_read_from_ifd0_alone() {
        // Sony copies ImageDescription, Artist and Copyright into IFD1 and
        // IFD2; a file holding them in IFD1 alone gives none.
        let (mut src, tree) = testing::file(|builder, ifd0| {
            let ifd1 = builder.ifd();
            builder
                .next(ifd0, Some(ifd1))
                .short(ifd0, ORIENTATION, &[1]);
            builder
                .ascii(ifd1, IMAGE_DESCRIPTION, "The port of Le Havre")
                .ascii(ifd1, ARTIST, "Claude Monet")
                .ascii(ifd1, COPYRIGHT, "Copyright Claude Monet");
        });
        let metadata = read(&mut src, &tree).unwrap().metadata;
        assert_eq!(metadata.authorship(), &Authorship::default());
    }

    #[test]
    fn the_exif_texts_are_not_read_from_ifd0() {
        let authorship = authorship_of(
            |builder, ifd0| {
                builder
                    .ascii(ifd0, PHOTOGRAPHER, "Camille Pissarro")
                    .ascii(ifd0, IMAGE_EDITOR, "Alfred Sisley")
                    .ascii(ifd0, IMAGE_TITLE, "Impression, soleil levant")
                    .undefined(ifd0, USER_COMMENT, b"ASCII\0\0\0Painted at dawn");
            },
            |_, _| {},
        );
        assert_eq!(authorship, Authorship::default());
    }

    #[test]
    fn a_comment_under_the_ascii_code_is_its_text() {
        // Nikon's 44 bytes: the text ending at a NUL, or padded with
        // spaces; unset, 36 NULs or 36 spaces.
        let text = "Painted at dawn";
        assert_eq!(
            comment_of(&nikon_comment(ASCII_CODE, text, 0)).as_deref(),
            Some(text)
        );
        assert_eq!(
            comment_of(&nikon_comment(ASCII_CODE, &format!("{text}\0junk"), b' ')).as_deref(),
            Some(text)
        );
        assert_eq!(
            comment_of(&nikon_comment(ASCII_CODE, text, b' ')).as_deref(),
            Some(text)
        );
        assert_eq!(comment_of(&nikon_comment(ASCII_CODE, "", 0)), None);
        assert_eq!(comment_of(&nikon_comment(ASCII_CODE, "", b' ')), None);
        // Free text: past 255 bytes, with its line breaks.
        let long = format!("{}\n{}", "A".repeat(200), "B".repeat(200));
        let mut bytes = ASCII_CODE.to_vec();
        bytes.extend_from_slice(long.as_bytes());
        assert_eq!(comment_of(&bytes), Some(long));
    }

    #[test]
    fn a_comment_under_any_other_code_is_none() {
        // The undefined code of eight NULs, JIS, UNICODE, and a code Exif
        // does not list, each followed by a text.
        for code in [
            b"\0\0\0\0\0\0\0\0",
            b"JIS\0\0\0\0\0",
            b"UNICODE\0",
            b"ASCII\0\0X",
            b"ascii\0\0\0",
        ] {
            assert_eq!(
                comment_of(&nikon_comment(code, "Painted at dawn", 0)),
                None,
                "{code:?}"
            );
        }
    }

    #[test]
    fn a_comment_no_longer_than_its_code_or_of_another_type_is_none() {
        assert_eq!(comment_of(b"ASCII\0\0"), None);
        assert_eq!(comment_of(ASCII_CODE), None);
        assert_eq!(comment_of(b""), None);
        for metadata in [
            with_text(In::Exif, |builder, exif| {
                builder.ascii(exif, USER_COMMENT, "ASCII\0\0\0Painted at dawn");
            }),
            with_text(In::Exif, |builder, exif| {
                builder.byte(exif, USER_COMMENT, b"ASCII\0\0\0Painted at dawn");
            }),
        ] {
            assert_eq!(metadata.authorship().comment(), None);
        }
    }

    #[test]
    fn a_damaged_authorship_text_costs_no_other_value() {
        type Damage = fn(&mut TiffBuilder, IfdId, IfdId);
        let texts: [(u16, Damage); 7] = [
            (ARTIST, |builder, ifd0, _| too_long(builder, ifd0, ARTIST)),
            (COPYRIGHT, |builder, ifd0, _| {
                too_long(builder, ifd0, COPYRIGHT)
            }),
            (IMAGE_DESCRIPTION, |builder, ifd0, _| {
                too_long(builder, ifd0, IMAGE_DESCRIPTION);
            }),
            (PHOTOGRAPHER, |builder, _, exif| {
                too_long(builder, exif, PHOTOGRAPHER)
            }),
            (IMAGE_EDITOR, |builder, _, exif| {
                too_long(builder, exif, IMAGE_EDITOR)
            }),
            (IMAGE_TITLE, |builder, _, exif| {
                too_long(builder, exif, IMAGE_TITLE)
            }),
            (USER_COMMENT, |builder, _, exif| {
                too_long_of(builder, exif, USER_COMMENT, UNDEFINED, 1);
            }),
        ];
        let whole = Authorship::default()
            .with_artist("Claude Monet")
            .with_copyright("Copyright Claude Monet")
            .with_description("The port of Le Havre")
            .with_photographer("Camille Pissarro")
            .with_editor("Alfred Sisley")
            .with_title("Impression, soleil levant")
            .with_comment("Painted at dawn");
        for (damaged, damage) in texts {
            let metadata = z9_except(0, |builder, ifd0, exif| {
                let add = |tag| tag != damaged;
                if add(ARTIST) {
                    builder.ascii(ifd0, ARTIST, "Claude Monet");
                }
                if add(COPYRIGHT) {
                    builder.ascii(ifd0, COPYRIGHT, "Copyright Claude Monet");
                }
                if add(IMAGE_DESCRIPTION) {
                    builder.ascii(ifd0, IMAGE_DESCRIPTION, "The port of Le Havre");
                }
                if add(PHOTOGRAPHER) {
                    builder.ascii(exif, PHOTOGRAPHER, "Camille Pissarro");
                }
                if add(IMAGE_EDITOR) {
                    builder.ascii(exif, IMAGE_EDITOR, "Alfred Sisley");
                }
                if add(IMAGE_TITLE) {
                    builder.ascii(exif, IMAGE_TITLE, "Impression, soleil levant");
                }
                if add(USER_COMMENT) {
                    builder.undefined(exif, USER_COMMENT, b"ASCII\0\0\0Painted at dawn");
                }
                damage(builder, ifd0, exif);
            });
            let mut expected = whole.clone();
            match damaged {
                ARTIST => expected.artist = None,
                COPYRIGHT => expected.copyright = None,
                IMAGE_DESCRIPTION => expected.description = None,
                PHOTOGRAPHER => expected.photographer = None,
                IMAGE_EDITOR => expected.editor = None,
                IMAGE_TITLE => expected.title = None,
                _ => expected.comment = None,
            }
            assert_eq!(
                metadata,
                Metadata {
                    authorship: expected,
                    ..z9_expected()
                },
                "{damaged:#06X}"
            );
        }
    }

    #[test]
    fn a_failure_of_the_reader_reading_an_authorship_text_or_the_owner_is_the_readers_error() {
        let far = u32::try_from(2 * FAR).unwrap();
        let texts = [
            (ARTIST, In::Ifd0, ASCII, &b"Claude Monet\0"[..]),
            (COPYRIGHT, In::Ifd0, ASCII, b"Copyright Claude Monet\0"),
            (
                IMAGE_DESCRIPTION,
                In::Ifd0,
                ASCII,
                b"The port of Le Havre\0",
            ),
            (PHOTOGRAPHER, In::Exif, UTF8, b"Camille Pissarro\0"),
            (IMAGE_EDITOR, In::Exif, UTF8, b"Alfred Sisley\0"),
            (IMAGE_TITLE, In::Exif, UTF8, b"Impression, soleil levant\0"),
            (
                USER_COMMENT,
                In::Exif,
                UNDEFINED,
                b"ASCII\0\0\0Painted at dawn",
            ),
            (CAMERA_OWNER_NAME, In::Exif, ASCII, b"Paul Cezanne\0"),
        ];
        for (tag, at, typ, value) in texts {
            // The value stored far away, longer than four bytes and so not
            // held in its entry.
            let count = u32::try_from(value.len()).unwrap();
            let bytes = z9_bytes_except(tag, |builder, ifd0, exif| {
                builder.blob_at(2 * FAR, value);
                let ifd = if matches!(at, In::Ifd0) { ifd0 } else { exif };
                builder.raw(ifd, tag, typ, count, RawValue::Offset(far));
            });
            // The file is sound, for a reader that does not fail.
            let sound = read_bytes(bytes.clone());
            assert!(
                sound.authorship != Authorship::default() || sound.camera.owner_name.is_some(),
                "{tag:#06X}"
            );

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{tag:#06X}: {err:?}");
        }
    }

    #[test]
    fn bytes_of_a_utf8_text_that_are_not_utf8_become_the_replacement_character() {
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.blob_at(4096, b"SO\xFFNY\0");
            builder.raw(ifd, MAKE, UTF8, 6, RawValue::Offset(4096));
        });
        let metadata = read(&mut src, &tree).unwrap().metadata;
        assert_eq!(metadata.camera().recorded_make(), Some("SO\u{FFFD}NY"));
    }

    #[test]
    fn a_text_exif_gives_as_ascii_alone_is_none_in_utf8() {
        for (tag, get) in ASCII_ONLY_TEXTS {
            let ascii = with_text(In::Exif, |builder, exif| {
                builder.ascii(exif, tag, "1234567");
            });
            assert_eq!(get(&ascii), Some("1234567"), "{tag:#06X}");
            let utf8 = with_text(In::Exif, |builder, exif| {
                builder.utf8(exif, tag, "1234567");
            });
            assert_eq!(get(&utf8), None, "{tag:#06X}");
        }
        let utf8_time = with_text(In::Exif, |builder, exif| {
            builder.utf8(exif, DATE_TIME_ORIGINAL, "2022:02:05 13:46:41");
        });
        assert_eq!(utf8_time.capture_time(), None);
    }

    #[test]
    fn bytes_that_are_not_utf8_become_the_replacement_character() {
        assert_eq!(make_bytes(b"SO\xFFNY\0").as_deref(), Some("SO\u{FFFD}NY"));
    }

    #[test]
    fn the_length_limit_counts_the_bytes_stored_not_the_text_they_become() {
        // 200 bytes that are not UTF-8: 600 bytes of text once each is
        // replaced, yet within the limit as stored.
        let mut bytes = vec![0xFF; 200];
        bytes.push(0);
        assert_eq!(make_bytes(&bytes), Some("\u{FFFD}".repeat(200)));
    }

    #[test]
    fn a_date_time_original_of_type_undefined_is_ignored() {
        let metadata = z9_except(DATE_TIME_ORIGINAL, |builder, _, exif| {
            builder.undefined(exif, DATE_TIME_ORIGINAL, b"2022:02:05 13:46:41\0");
        });
        assert_eq!(
            metadata,
            Metadata {
                capture_time: None,
                ..z9_expected()
            }
        );
    }

    #[test]
    fn sub_seconds_without_a_date_time_original_give_no_time() {
        // The Z 9 file without its DateTimeOriginal, which keeps its
        // SubSecTimeOriginal and its OffsetTimeOriginal; then a file whose
        // Exif IFD holds the SubSecTimeOriginal alone.
        let metadata = z9_except(DATE_TIME_ORIGINAL, |_, _, _| {});
        assert_eq!(metadata.capture_time(), None);
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            builder.ascii(exif, SUB_SEC_TIME_ORIGINAL, "85");
        });
        assert_eq!(read(&mut src, &tree).unwrap().metadata.capture_time(), None);
    }

    #[test]
    fn an_offset_without_a_date_time_original_gives_no_time() {
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            builder.ascii(exif, OFFSET_TIME_ORIGINAL, "-05:00");
        });
        assert_eq!(read(&mut src, &tree).unwrap().metadata.capture_time(), None);
    }

    #[test]
    fn the_time_is_never_taken_from_date_time_digitized_or_ifd0s_date_time() {
        let metadata = z9_except(DATE_TIME_ORIGINAL, |builder, ifd0, exif| {
            builder.ascii(exif, DATE_TIME_DIGITIZED, "2022:02:05 13:46:41");
            builder.ascii(ifd0, DATE_TIME, "2022:02:05 13:46:41");
        });
        assert_eq!(metadata.capture_time(), None);
    }

    #[test]
    fn a_file_without_an_exif_ifd_has_only_its_camera_and_orientation() {
        // The Exif tags in IFD0, where they do not belong, are not read:
        // its camera settings are the empty record.
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "SONY")
                .ascii(ifd, MODEL, "DSC-F828")
                .short(ifd, ORIENTATION, &[6])
                .ascii(ifd, BODY_SERIAL_NUMBER, "1234567")
                .ascii(ifd, DATE_TIME_ORIGINAL, "2022:02:05 13:46:41");
            for (tag, add) in EXIF_EXPOSURE {
                add(builder, ifd, tag);
            }
            for (tag, code) in EXIF_SETTINGS {
                builder.short(ifd, tag, &[code]);
            }
            builder.short(ifd, COLOR_SPACE, &[1]);
            builder.ascii(ifd, LENS_MODEL, "FE 50mm F2.5 G").rational(
                ifd,
                LENS_SPECIFICATION,
                &Z9_LENS_SPECIFICATION,
            );
        });
        assert!(tree.exif().is_none());
        let metadata = read(&mut src, &tree).unwrap().metadata;
        assert_eq!(
            metadata,
            Metadata {
                camera: Camera::default()
                    .with_recorded_make("SONY")
                    .with_recorded_model("DSC-F828"),
                orientation: Orientation::Rotate90,
                ..Metadata::default()
            }
        );
    }

    #[test]
    fn the_orientation_is_the_one_the_previews_take() {
        for (value, orientation) in [
            (8, Orientation::Rotate270),
            (6, Orientation::Rotate90),
            (1, Orientation::NoTransforms),
            // No Exif orientation: none.
            (9, Orientation::NoTransforms),
        ] {
            let (mut src, tree) = testing::file(|builder, ifd| {
                builder.short(ifd, ORIENTATION, &[value]);
            });
            let metadata = read(&mut src, &tree).unwrap().metadata;
            assert_eq!(metadata.orientation(), orientation, "{value}");
            assert_eq!(
                tiff_previews::orientation(&mut src, &tree).unwrap(),
                orientation,
                "{value}"
            );
        }
    }

    /// Gives `ifd` tag `tag` as ASCII with [`TOO_LONG`] bytes at offset
    /// 4096, and makes the file long enough to hold them: a value that lies
    /// inside the file and cannot be read.
    fn too_long(builder: &mut TiffBuilder, ifd: IfdId, tag: u16) {
        too_long_of(builder, ifd, tag, ASCII, 1);
    }

    /// As [`too_long`], with values of type `typ`, of `size` bytes each:
    /// as many as take at least [`TOO_LONG`] bytes.
    fn too_long_of(builder: &mut TiffBuilder, ifd: IfdId, tag: u16, typ: u16, size: u32) {
        let count = TOO_LONG.div_ceil(size);
        builder.raw(ifd, tag, typ, count, RawValue::Offset(4096));
        builder.pad_to(4096 + u64::from(count) * u64::from(size));
    }

    #[test]
    fn a_damaged_value_costs_only_itself() {
        type Change = fn(&mut TiffBuilder, IfdId, IfdId);
        /// The Z 9 file's metadata with `change` made to its settings.
        fn with_settings(change: fn(&mut CameraSettings)) -> Metadata {
            let mut expected = z9_expected();
            change(&mut expected.camera_settings);
            expected
        }
        let cases: [(&str, u16, Change, Metadata); 21] = [
            (
                "a Model that is not text",
                MODEL,
                |builder, ifd0, _| {
                    builder.byte(ifd0, MODEL, b"NIKON Z 9\0");
                },
                Metadata {
                    camera: Camera {
                        recorded_model: None,
                        ..z9_camera()
                    },
                    ..z9_expected()
                },
            ),
            (
                "a Software that is not text",
                SOFTWARE,
                |builder, ifd0, _| {
                    builder.byte(ifd0, SOFTWARE, b"Ver.01.00\0");
                },
                Metadata {
                    software: None,
                    ..z9_expected()
                },
            ),
            (
                "a Software too long to read",
                SOFTWARE,
                |builder, ifd0, _| too_long(builder, ifd0, SOFTWARE),
                Metadata {
                    software: None,
                    ..z9_expected()
                },
            ),
            (
                "a Make too long to read",
                MAKE,
                |builder, ifd0, _| too_long(builder, ifd0, MAKE),
                Metadata {
                    camera: Camera {
                        recorded_make: None,
                        ..z9_camera()
                    },
                    ..z9_expected()
                },
            ),
            (
                "an owner's name too long to read",
                CAMERA_OWNER_NAME,
                |builder, _, exif| too_long(builder, exif, CAMERA_OWNER_NAME),
                z9_expected(),
            ),
            (
                "a serial number of 256 bytes",
                BODY_SERIAL_NUMBER,
                |builder, _, exif| {
                    builder.ascii(exif, BODY_SERIAL_NUMBER, &"1".repeat(256));
                },
                Metadata {
                    camera: Camera {
                        serial_number: None,
                        ..z9_camera()
                    },
                    ..z9_expected()
                },
            ),
            (
                "a DateTimeOriginal too long to read",
                DATE_TIME_ORIGINAL,
                |builder, _, exif| too_long(builder, exif, DATE_TIME_ORIGINAL),
                Metadata {
                    capture_time: None,
                    ..z9_expected()
                },
            ),
            (
                "an impossible date",
                DATE_TIME_ORIGINAL,
                |builder, _, exif| {
                    builder.ascii(exif, DATE_TIME_ORIGINAL, "2022:02:30 13:46:41");
                },
                Metadata {
                    capture_time: None,
                    ..z9_expected()
                },
            ),
            (
                "sub-seconds that are not text",
                SUB_SEC_TIME_ORIGINAL,
                |builder, _, exif| {
                    builder.undefined(exif, SUB_SEC_TIME_ORIGINAL, b"85\0");
                },
                Metadata {
                    capture_time: z9_time(None, Some(b"-05:00")),
                    ..z9_expected()
                },
            ),
            (
                "an offset too long to read",
                OFFSET_TIME_ORIGINAL,
                |builder, _, exif| too_long(builder, exif, OFFSET_TIME_ORIGINAL),
                Metadata {
                    capture_time: z9_time(Some(b"85"), None),
                    ..z9_expected()
                },
            ),
            (
                "a FocalLengthIn35mmFilm too long to read",
                FOCAL_LENGTH_IN_35MM_FILM,
                |builder, _, exif| too_long_of(builder, exif, FOCAL_LENGTH_IN_35MM_FILM, SHORT, 2),
                z9_expected_with(Exposure {
                    focal_length_in_35mm_film: None,
                    ..z9_exposure()
                }),
            ),
            (
                // The RecommendedExposureIndex, 64 too, gives the ISO.
                "a PhotographicSensitivity too long to read",
                PHOTOGRAPHIC_SENSITIVITY,
                |builder, _, exif| too_long_of(builder, exif, PHOTOGRAPHIC_SENSITIVITY, SHORT, 2),
                z9_expected(),
            ),
            (
                "a LensMake too long to read",
                LENS_MAKE,
                |builder, _, exif| too_long(builder, exif, LENS_MAKE),
                z9_expected_with_lens(z9_lens_with(|lens| lens.recorded_make = None)),
            ),
            (
                "a LensModel that is not text",
                LENS_MODEL,
                |builder, _, exif| {
                    builder.undefined(exif, LENS_MODEL, b"NIKKOR Z 14-30mm f/4 S\0");
                },
                z9_expected_with_lens(z9_lens_with(|lens| lens.recorded_model = None)),
            ),
            (
                "a LensSerialNumber of 256 bytes",
                LENS_SERIAL_NUMBER,
                |builder, _, exif| {
                    builder.ascii(exif, LENS_SERIAL_NUMBER, &"1".repeat(256));
                },
                z9_expected_with_lens(z9_lens_with(|lens| lens.serial_number = None)),
            ),
            (
                "a LensSpecification of more values than the default cap allows",
                LENS_SPECIFICATION,
                |builder, _, exif| too_long_of(builder, exif, LENS_SPECIFICATION, RATIONAL, 8),
                z9_expected_with_lens(z9_lens_with(|lens| lens.range = [None; 4])),
            ),
            (
                "an ExposureProgram too long to read",
                EXPOSURE_PROGRAM,
                |builder, _, exif| too_long_of(builder, exif, EXPOSURE_PROGRAM, SHORT, 2),
                with_settings(|settings| settings.exposure_program = None),
            ),
            (
                "a MeteringMode too long to read",
                METERING_MODE,
                |builder, _, exif| too_long_of(builder, exif, METERING_MODE, SHORT, 2),
                with_settings(|settings| settings.metering_mode = None),
            ),
            (
                "a Flash too long to read",
                FLASH,
                |builder, _, exif| too_long_of(builder, exif, FLASH, SHORT, 2),
                with_settings(|settings| settings.flash = None),
            ),
            (
                "a WhiteBalance too long to read",
                WHITE_BALANCE,
                |builder, _, exif| too_long_of(builder, exif, WHITE_BALANCE, SHORT, 2),
                with_settings(|settings| settings.white_balance.mode = None),
            ),
            (
                // An auto white balance does not read the LightSource.
                "a LightSource too long to read",
                LIGHT_SOURCE,
                |builder, _, exif| too_long_of(builder, exif, LIGHT_SOURCE, SHORT, 2),
                z9_expected(),
            ),
        ];
        for (case, tag, change, expected) in cases {
            assert_eq!(z9_except(tag, change), expected, "{case}");
        }
    }

    #[test]
    fn a_failure_of_the_reader_is_the_readers_error() {
        let far = u32::try_from(2 * FAR).unwrap();
        for (tag, text) in IFD0_TEXT.into_iter().chain(EXIF_TEXT) {
            // The value stored far away, padded with NULs so that it is
            // longer than four bytes and so not held in its entry.
            let mut value = text.as_bytes().to_vec();
            value.extend_from_slice(&[0; 8]);
            let count = u32::try_from(value.len()).unwrap();
            let in_ifd0 = IFD0_TEXT.iter().any(|&(t, _)| t == tag);
            let bytes = z9_bytes_except(tag, |builder, ifd0, exif| {
                builder.blob_at(2 * FAR, &value);
                let ifd = if in_ifd0 { ifd0 } else { exif };
                builder.raw(ifd, tag, ASCII, count, RawValue::Offset(far));
            });
            // The file is sound, for a reader that does not fail.
            assert_eq!(read_bytes(bytes.clone()), z9_expected(), "{tag:#06X}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{tag:#06X}: {err:?}");
        }
    }

    #[test]
    fn a_failure_of_the_reader_reading_the_exposure_is_the_readers_error() {
        /// `values` as little-endian LONGs.
        fn longs(values: &[u32]) -> Vec<u8> {
            values.iter().flat_map(|v| v.to_le_bytes()).collect()
        }
        let only = |change: fn(&mut Exposure)| {
            let mut exposure = Exposure::default();
            change(&mut exposure);
            exposure
        };
        // Each value stored far away, so that it is read through the
        // failing part of the reader: of its tag's type, its first value
        // the one expected, and longer than four bytes, so not held in its
        // entry. The ISO's fallbacks are read only when the
        // PhotographicSensitivity does not give it, so their files hold an
        // inline one that does not. The focal length in 35 mm terms is not
        // among them: it is read only as one SHORT, which its entry holds,
        // so reading it never reads the file.
        let cases = [
            (
                EXPOSURE_TIME,
                RATIONAL,
                1,
                longs(&[10, 3200]),
                None,
                only(|e| e.exposure_time = Some(fraction(10, 3200))),
            ),
            (
                F_NUMBER,
                RATIONAL,
                1,
                longs(&[900, 100]),
                None,
                only(|e| e.f_number = Some(fraction(900, 100))),
            ),
            (
                FOCAL_LENGTH,
                RATIONAL,
                1,
                longs(&[180, 10]),
                None,
                only(|e| e.focal_length = Some(fraction(180, 10))),
            ),
            (
                EXPOSURE_BIAS_VALUE,
                SRATIONAL,
                1,
                longs(&[(-3_i32).cast_unsigned(), 10]),
                None,
                only(|e| e.exposure_bias = Some(signed(-3, 10))),
            ),
            (
                PHOTOGRAPHIC_SENSITIVITY,
                SHORT,
                3,
                vec![64, 0, 0, 0, 0, 0],
                None,
                only(|e| e.iso = Some(64)),
            ),
            (
                RECOMMENDED_EXPOSURE_INDEX,
                LONG,
                2,
                longs(&[100, 0]),
                Some(65535),
                only(|e| e.iso = Some(100)),
            ),
            (
                ISO_SPEED,
                LONG,
                2,
                longs(&[200, 0]),
                Some(0),
                only(|e| e.iso = Some(200)),
            ),
        ];
        let far = u32::try_from(2 * FAR).unwrap();
        for (tag, typ, count, value, sensitivity, expected) in cases {
            let mut builder = TiffBuilder::new(Order::Little);
            let ifd0 = builder.ifd();
            let exif = builder.ifd();
            builder.exif(ifd0, exif).blob_at(2 * FAR, &value);
            builder.raw(exif, tag, typ, count, RawValue::Offset(far));
            if let Some(sensitivity) = sensitivity {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[sensitivity]);
            }
            let bytes = builder.build().bytes;
            // The file is sound, for a reader that does not fail.
            assert_eq!(read_bytes(bytes.clone()).exposure, expected, "{tag:#06X}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{tag:#06X}: {err:?}");
        }
    }

    #[test]
    fn an_iso_of_65535_is_replaced_by_the_recommended_exposure_index_or_else_the_iso_speed() {
        // Exif writes 65535 for a sensitivity of 65535 or more.
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[65535])
                    .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[102_400])
                    .long(exif, ISO_SPEED, &[204_800]);
            }),
            Some(102_400)
        );
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[65535])
                    .long(exif, ISO_SPEED, &[204_800]);
            }),
            Some(204_800)
        );
    }

    #[test]
    fn an_iso_of_65535_that_nothing_replaces_stands() {
        assert_eq!(
            iso(|builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[65535]);
            }),
            Some(65535)
        );
        // Zeros replace nothing.
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[65535])
                    .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[0])
                    .long(exif, ISO_SPEED, &[0]);
            }),
            Some(65535)
        );
    }

    #[test]
    fn a_missing_iso_is_the_recommended_exposure_index_or_else_the_iso_speed() {
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[64])
                    .long(exif, ISO_SPEED, &[100]);
            }),
            Some(64)
        );
        assert_eq!(
            iso(|builder, exif| {
                builder.long(exif, ISO_SPEED, &[100]);
            }),
            Some(100)
        );
        // An Exif IFD with another tag: one with no entries at all is
        // damage, which the walk leaves out of the tree.
        assert_eq!(
            iso(|builder, exif| {
                builder.rational(exif, EXPOSURE_TIME, &[(10, 3200)]);
            }),
            None
        );
    }

    #[test]
    fn a_recommended_exposure_index_of_zero_is_skipped_for_the_iso_speed() {
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[0])
                    .long(exif, ISO_SPEED, &[200]);
            }),
            Some(200)
        );
    }

    #[test]
    fn an_iso_of_zero_is_unknown() {
        assert_eq!(
            iso(|builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[0]).long(
                    exif,
                    RECOMMENDED_EXPOSURE_INDEX,
                    &[100],
                );
            }),
            Some(100)
        );
        assert_eq!(
            iso(|builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[0]);
            }),
            None
        );
        assert_eq!(
            iso(|builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[0]).long(
                    exif,
                    RECOMMENDED_EXPOSURE_INDEX,
                    &[0],
                );
            }),
            None
        );
    }

    #[test]
    fn a_known_iso_is_taken_over_the_other_tags() {
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .short(exif, PHOTOGRAPHIC_SENSITIVITY, &[64])
                    .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[100])
                    .long(exif, ISO_SPEED, &[200]);
            }),
            Some(64)
        );
    }

    #[test]
    fn an_iso_is_read_as_a_short_or_a_long_its_first_value() {
        assert_eq!(
            iso(|builder, exif| {
                builder.long(exif, PHOTOGRAPHIC_SENSITIVITY, &[64]);
            }),
            Some(64)
        );
        assert_eq!(
            iso(|builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[200, 100]);
            }),
            Some(200)
        );
    }

    #[test]
    fn an_iso_of_another_type_is_missing() {
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .rational(exif, PHOTOGRAPHIC_SENSITIVITY, &[(64, 1)])
                    .long(exif, RECOMMENDED_EXPOSURE_INDEX, &[100]);
            }),
            Some(100)
        );
        assert_eq!(
            iso(|builder, exif| {
                builder.rational(exif, PHOTOGRAPHIC_SENSITIVITY, &[(64, 1)]);
            }),
            None
        );
        // A fallback of another type is missing too.
        assert_eq!(
            iso(|builder, exif| {
                builder
                    .rational(exif, RECOMMENDED_EXPOSURE_INDEX, &[(100, 1)])
                    .long(exif, ISO_SPEED, &[200]);
            }),
            Some(200)
        );
    }

    #[test]
    fn an_iso_that_cannot_be_read_is_missing() {
        assert_eq!(
            iso(|builder, exif| {
                too_long_of(builder, exif, PHOTOGRAPHIC_SENSITIVITY, SHORT, 2);
                builder.long(exif, RECOMMENDED_EXPOSURE_INDEX, &[100]);
            }),
            Some(100)
        );
        assert_eq!(
            iso(|builder, exif| {
                too_long_of(builder, exif, PHOTOGRAPHIC_SENSITIVITY, SHORT, 2);
            }),
            None
        );
        assert_eq!(
            iso(|builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[65535]);
                too_long_of(builder, exif, RECOMMENDED_EXPOSURE_INDEX, LONG, 4);
            }),
            Some(65535)
        );
    }

    #[test]
    fn a_fraction_of_zero_is_none() {
        type Unset = fn(&mut Exposure);
        let unknown: [(u16, (u32, u32), Unset); 3] = [
            (EXPOSURE_TIME, (0, 1), |exposure| {
                exposure.exposure_time = None
            }),
            // These two as Sony's ILCE-7S with a manual lens records them,
            // on the public sample 1582.
            (F_NUMBER, (0, 10), |exposure| exposure.f_number = None),
            (FOCAL_LENGTH, (0, 10), |exposure| {
                exposure.focal_length = None
            }),
        ];
        for (tag, value, unset) in unknown {
            let mut expected = z9_exposure();
            unset(&mut expected);
            let metadata = z9_except(tag, |builder, _, exif| {
                builder.rational(exif, tag, &[value]);
            });
            assert_eq!(metadata, z9_expected_with(expected), "{tag:#06X}");
        }
    }

    #[test]
    fn a_fraction_with_a_zero_denominator_is_none() {
        type Unset = fn(&mut Exposure);
        let unsigned: [(u16, Unset); 3] = [
            (EXPOSURE_TIME, |exposure| exposure.exposure_time = None),
            (F_NUMBER, |exposure| exposure.f_number = None),
            (FOCAL_LENGTH, |exposure| exposure.focal_length = None),
        ];
        for (tag, unset) in unsigned {
            let mut expected = z9_exposure();
            unset(&mut expected);
            for value in [(10, 0), (0, 0)] {
                let metadata = z9_except(tag, |builder, _, exif| {
                    builder.rational(exif, tag, &[value]);
                });
                assert_eq!(metadata.exposure, expected, "{tag:#06X}: {value:?}");
            }
        }
        for value in [(-3, 0), (0, 0)] {
            let metadata = z9_except(EXPOSURE_BIAS_VALUE, |builder, _, exif| {
                builder.srational(exif, EXPOSURE_BIAS_VALUE, &[value]);
            });
            assert_eq!(
                metadata.exposure,
                Exposure {
                    exposure_bias: None,
                    ..z9_exposure()
                },
                "{value:?}"
            );
        }
    }

    #[test]
    fn no_compensation_is_a_real_zero_kept_as_stored() {
        assert_eq!(z9_file().exposure.exposure_bias, Some(signed(0, 6)));
        let bias = exposure(|builder, exif| {
            builder.srational(exif, EXPOSURE_BIAS_VALUE, &[(0, 10)]);
        })
        .exposure_bias;
        assert_eq!(bias, Some(signed(0, 10)));
    }

    #[test]
    fn a_negative_bias_is_kept_as_stored() {
        for value in [(-3, 10), (3, -10), (-2, 3)] {
            let bias = exposure(|builder, exif| {
                builder.srational(exif, EXPOSURE_BIAS_VALUE, &[value]);
            })
            .exposure_bias;
            assert_eq!(bias, Some(signed(value.0, value.1)), "{value:?}");
        }
    }

    #[test]
    fn a_fraction_of_the_other_type_is_none() {
        let read = exposure(|builder, exif| {
            builder
                .srational(exif, EXPOSURE_TIME, &[(10, 3200)])
                .srational(exif, F_NUMBER, &[(900, 100)])
                .srational(exif, FOCAL_LENGTH, &[(180, 10)])
                .rational(exif, EXPOSURE_BIAS_VALUE, &[(0, 6)]);
        });
        assert_eq!(read, Exposure::default());
    }

    #[test]
    fn a_fraction_with_several_values_gives_its_first() {
        let read = exposure(|builder, exif| {
            builder
                .rational(exif, EXPOSURE_TIME, &[(1, 50), (1, 100)])
                .rational(exif, F_NUMBER, &[(56, 10), (80, 10)])
                .rational(exif, FOCAL_LENGTH, &[(180, 10), (500, 10)])
                .srational(exif, EXPOSURE_BIAS_VALUE, &[(-3, 10), (0, 6)]);
        });
        assert_eq!(
            read,
            Exposure {
                iso: None,
                exposure_time: Some(fraction(1, 50)),
                f_number: Some(fraction(56, 10)),
                focal_length: Some(fraction(180, 10)),
                focal_length_in_35mm_film: None,
                exposure_bias: Some(signed(-3, 10)),
            }
        );
    }

    #[test]
    fn a_fraction_too_long_to_read_whole_gives_its_first() {
        // Each of the exposure's fractions with more values than the cap on
        // one value lets through, the first of them a real one: the first
        // is read alone, as the ISO's is, and the others are not read.
        let count = TOO_LONG.div_ceil(8);
        let too_long = |builder: &mut TiffBuilder, exif, tag, typ, first: [u32; 2]| {
            builder.blob_at(
                4096,
                &[first[0].to_le_bytes(), first[1].to_le_bytes()].concat(),
            );
            builder.raw(exif, tag, typ, count, RawValue::Offset(4096));
            builder.pad_to(4096 + u64::from(count) * 8);
        };
        let read =
            exposure(|builder, exif| too_long(builder, exif, EXPOSURE_TIME, RATIONAL, [1, 250]));
        assert_eq!(read.exposure_time, Some(fraction(1, 250)));
        let read = exposure(|builder, exif| too_long(builder, exif, F_NUMBER, RATIONAL, [28, 10]));
        assert_eq!(read.f_number, Some(fraction(28, 10)));
        let read =
            exposure(|builder, exif| too_long(builder, exif, FOCAL_LENGTH, RATIONAL, [500, 10]));
        assert_eq!(read.focal_length, Some(fraction(500, 10)));
        let minus_one_third = [(-1_i32).cast_unsigned(), 3];
        let read = exposure(|builder, exif| {
            too_long(
                builder,
                exif,
                EXPOSURE_BIAS_VALUE,
                SRATIONAL,
                minus_one_third,
            );
        });
        assert_eq!(read.exposure_bias, Some(signed(-1, 3)));
    }

    #[test]
    fn a_fraction_whose_first_value_is_unknown_is_none_whatever_follows() {
        // Only the first value is the picture's.
        let read = exposure(|builder, exif| {
            builder.rational(exif, F_NUMBER, &[(0, 10), (56, 10)]);
        });
        assert_eq!(read.f_number, None);
    }

    #[test]
    fn a_focal_length_in_35mm_film_of_zero_is_unknown() {
        // Sony's ILCE-7S with a manual lens, as on the public sample 1582.
        let metadata = z9_except(FOCAL_LENGTH_IN_35MM_FILM, |builder, _, exif| {
            builder.short(exif, FOCAL_LENGTH_IN_35MM_FILM, &[0]);
        });
        assert_eq!(
            metadata,
            z9_expected_with(Exposure {
                focal_length_in_35mm_film: None,
                ..z9_exposure()
            })
        );
    }

    #[test]
    fn a_missing_focal_length_in_35mm_film_is_none() {
        // As on the D1H's, the D1X's and the Coolpix E5700's public samples.
        let metadata = z9_except(FOCAL_LENGTH_IN_35MM_FILM, |_, _, _| {});
        assert_eq!(
            metadata,
            z9_expected_with(Exposure {
                focal_length_in_35mm_film: None,
                ..z9_exposure()
            })
        );
    }

    #[test]
    fn a_focal_length_in_35mm_film_of_another_type_than_short_is_none() {
        // Unlike the ISO, not even a LONG, which the standard does not allow
        // this tag.
        type Add = fn(&mut TiffBuilder, IfdId);
        let cases: [(&str, Add); 3] = [
            ("LONG", |builder, exif| {
                builder.long(exif, FOCAL_LENGTH_IN_35MM_FILM, &[18]);
            }),
            ("RATIONAL", |builder, exif| {
                builder.rational(exif, FOCAL_LENGTH_IN_35MM_FILM, &[(18, 1)]);
            }),
            ("ASCII", |builder, exif| {
                builder.ascii(exif, FOCAL_LENGTH_IN_35MM_FILM, "18");
            }),
        ];
        for (case, add) in cases {
            let read = exposure(|builder, exif| {
                builder.rational(exif, FOCAL_LENGTH, &[(180, 10)]);
                add(builder, exif);
            });
            assert_eq!(read.focal_length_in_35mm_film, None, "{case}");
            // The other values of the Exif IFD are still read.
            assert_eq!(read.focal_length, Some(fraction(180, 10)), "{case}");
        }
    }

    #[test]
    fn a_focal_length_in_35mm_film_of_other_than_one_value_is_none() {
        // The standard gives it one value: with several, even the first is
        // not trusted, and with none there is nothing to read.
        for values in [&[18, 50][..], &[18, 18, 18][..], &[][..]] {
            let read = exposure(|builder, exif| {
                builder.rational(exif, FOCAL_LENGTH, &[(180, 10)]);
                builder.short(exif, FOCAL_LENGTH_IN_35MM_FILM, values);
            });
            assert_eq!(read.focal_length_in_35mm_film, None, "{values:?}");
            // The other values of the Exif IFD are still read.
            assert_eq!(read.focal_length, Some(fraction(180, 10)), "{values:?}");
        }
    }

    #[test]
    fn a_failure_of_the_reader_reading_the_lens_range_is_the_readers_error() {
        // The lens's text values are covered with the others, by
        // `a_failure_of_the_reader_is_the_readers_error`; its range, stored
        // far away, is read here.
        let far = u32::try_from(2 * FAR).unwrap();
        let value: Vec<u8> = Z9_LENS_SPECIFICATION
            .iter()
            .flat_map(|&(n, d)| [n.to_le_bytes(), d.to_le_bytes()])
            .flatten()
            .collect();
        let bytes = z9_bytes_except(LENS_SPECIFICATION, |builder, _, exif| {
            builder.blob_at(2 * FAR, &value);
            builder.raw(exif, LENS_SPECIFICATION, RATIONAL, 4, RawValue::Offset(far));
        });
        // The file is sound, for a reader that does not fail.
        assert_eq!(read_bytes(bytes.clone()), z9_expected());

        let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        let err = read(&mut src, &tree).expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }

    #[test]
    fn a_lens_make_of_nul_bytes_is_none() {
        // As the D780 writes it for an adapted F-mount lens (sample 3827):
        // six NULs, the builder adding the last.
        let metadata = z9_except(LENS_MAKE, |builder, _, exif| {
            builder.ascii(exif, LENS_MAKE, "\0\0\0\0\0");
        });
        assert_eq!(
            metadata,
            z9_expected_with_lens(z9_lens_with(|lens| lens.recorded_make = None))
        );
    }

    #[test]
    fn the_d780s_adapted_lens_gives_its_model_and_range_only() {
        // The lens tags of the public D780 sample 3827: a LensMake and a
        // LensSerialNumber of NULs only, a padded LensModel and a range.
        let read = lens(|builder, exif| {
            builder
                .ascii(exif, LENS_MAKE, "\0\0\0\0\0")
                .ascii(exif, LENS_MODEL, &padded_lens_model("VR 24-120mm f/4G"))
                .ascii(exif, LENS_SERIAL_NUMBER, &"\0".repeat(10))
                .rational(
                    exif,
                    LENS_SPECIFICATION,
                    &[(240, 10), (1200, 10), (400, 100), (400, 100)],
                );
        });
        let expected = LensValues {
            recorded_model: Some("VR 24-120mm f/4G".to_owned()),
            range: [
                Some(fraction(240, 10)),
                Some(fraction(1200, 10)),
                Some(fraction(400, 100)),
                Some(fraction(400, 100)),
            ],
            ..LensValues::none()
        };
        assert_eq!(read, expected.lens());
    }

    #[test]
    fn a_lens_model_of_dashes_only_names_no_lens() {
        // Sony's manual lens, as on the public ILCE-7S sample 1582.
        let metadata = z9_except(LENS_MODEL, |builder, _, exif| {
            builder.ascii(exif, LENS_MODEL, "----");
        });
        assert_eq!(
            metadata,
            z9_expected_with_lens(z9_lens_with(|lens| lens.recorded_model = None))
        );
        // With no other lens tag, as on that sample, the lens has no value.
        for model in ["----", "-", " -- "] {
            let read = lens(|builder, exif| {
                builder.ascii(exif, LENS_MODEL, model);
            });
            assert_eq!(read, Lens::default(), "{model:?}");
        }
    }

    #[test]
    fn dashes_among_other_characters_are_kept() {
        for model in ["FE 24-70mm F2.8 GM", "-A-", "--x"] {
            let read = lens(|builder, exif| {
                builder.ascii(exif, LENS_MODEL, model);
            });
            assert_eq!(read.recorded_model(), Some(model), "{model:?}");
        }
    }

    #[test]
    fn only_the_lens_model_is_refused_for_dashes() {
        let read = lens(|builder, exif| {
            builder
                .ascii(exif, LENS_MAKE, "----")
                .ascii(exif, LENS_SERIAL_NUMBER, "--");
        });
        let expected = LensValues {
            recorded_make: Some("----".to_owned()),
            serial_number: Some("--".to_owned()),
            ..LensValues::none()
        };
        assert_eq!(read, expected.lens());
    }

    #[test]
    fn a_lens_specification_of_other_than_four_values_is_ignored_whole() {
        for values in [
            &[(140, 10), (300, 10), (400, 100)][..],
            &[(140, 10), (300, 10), (400, 100), (400, 100), (400, 100)][..],
            &[][..],
        ] {
            let metadata = z9_except(LENS_SPECIFICATION, |builder, _, exif| {
                builder.rational(exif, LENS_SPECIFICATION, values);
            });
            assert_eq!(
                metadata,
                z9_expected_with_lens(z9_lens_with(|lens| lens.range = [None; 4])),
                "{} values",
                values.len()
            );
        }
        // With no other lens tag the lens has no value.
        let read = lens(|builder, exif| {
            builder.rational(
                exif,
                LENS_SPECIFICATION,
                &[(140, 10), (300, 10), (400, 100)],
            );
        });
        assert_eq!(read, Lens::default());
    }

    #[test]
    fn a_prime_lens_has_the_same_range_at_both_ends() {
        let read = lens(|builder, exif| {
            builder.rational(
                exif,
                LENS_SPECIFICATION,
                &[(550, 10), (550, 10), (18, 10), (18, 10)],
            );
        });
        let expected = LensValues {
            range: [
                Some(fraction(550, 10)),
                Some(fraction(550, 10)),
                Some(fraction(18, 10)),
                Some(fraction(18, 10)),
            ],
            ..LensValues::none()
        };
        assert_eq!(read, expected.lens());
    }

    #[test]
    fn an_unknown_value_of_the_range_is_none_and_costs_no_other() {
        // Unknown apertures, as Exif writes them: 0/0.
        let metadata = z9_except(LENS_SPECIFICATION, |builder, _, exif| {
            builder.rational(
                exif,
                LENS_SPECIFICATION,
                &[(140, 10), (300, 10), (0, 0), (0, 0)],
            );
        });
        assert_eq!(
            metadata,
            z9_expected_with_lens(z9_lens_with(|lens| lens.range[2..].fill(None)))
        );
        // A zero numerator, or a zero denominator, in any one place.
        for place in 0..4 {
            for zero in [(0, 10), (10, 0)] {
                let mut values = Z9_LENS_SPECIFICATION;
                values[place] = zero;
                let metadata = z9_except(LENS_SPECIFICATION, |builder, _, exif| {
                    builder.rational(exif, LENS_SPECIFICATION, &values);
                });
                assert_eq!(
                    metadata,
                    z9_expected_with_lens(z9_lens_with(|lens| lens.range[place] = None)),
                    "{place}: {zero:?}"
                );
            }
        }
    }

    #[test]
    fn a_lens_specification_of_type_srational_is_ignored() {
        let metadata = z9_except(LENS_SPECIFICATION, |builder, _, exif| {
            builder.srational(
                exif,
                LENS_SPECIFICATION,
                &[(140, 10), (300, 10), (400, 100), (400, 100)],
            );
        });
        assert_eq!(
            metadata,
            z9_expected_with_lens(z9_lens_with(|lens| lens.range = [None; 4]))
        );
    }

    #[test]
    fn a_lens_with_only_a_serial_number_keeps_it() {
        let read = lens(|builder, exif| {
            builder.ascii(exif, LENS_SERIAL_NUMBER, "12345678  ");
        });
        let expected = LensValues {
            serial_number: Some("12345678".to_owned()),
            ..LensValues::none()
        };
        assert_eq!(read, expected.lens());
    }

    #[test]
    fn an_exif_ifd_without_lens_tags_gives_an_empty_lens() {
        // An IFD without entries is damage and not read at all, so the Exif
        // IFD holds a tag other than the lens's.
        let read = lens(|builder, exif| {
            builder.ascii(exif, DATE_TIME_ORIGINAL, "2022:02:05 13:46:41");
        });
        assert_eq!(read, Lens::default());
    }

    /// The InteroperabilityVersion, a tag of the Interoperability IFD the
    /// reader does not read, which cameras write beside the InteropIndex:
    /// it keeps the IFD from being empty, which would be damage, in the
    /// tests that leave the index out.
    const INTEROP_VERSION: u16 = 0x0002;

    /// The camera settings read from a file whose Exif IFD holds the
    /// ExposureTime 1/320 s, so that it is never empty, and what `build`
    /// adds, given the builder and the Exif IFD's ID.
    fn settings(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> CameraSettings {
        let (mut src, tree) = testing::file(|builder, ifd| {
            let exif = builder.ifd();
            builder
                .exif(ifd, exif)
                .rational(exif, EXPOSURE_TIME, &[(10, 3200)]);
            build(builder, exif);
        });
        read(&mut src, &tree).unwrap().metadata.camera_settings
    }

    /// The camera settings of [`settings`]'s file with the one SHORT `code`
    /// in tag `tag` of its Exif IFD.
    fn settings_of(tag: u16, code: u16) -> CameraSettings {
        settings(|builder, exif| {
            builder.short(exif, tag, &[code]);
        })
    }

    #[test]
    fn every_exposure_program_exif_defines_gives_its_program() {
        let named = [
            (1, ExposureProgram::Manual),
            (2, ExposureProgram::Program),
            (3, ExposureProgram::AperturePriority),
            (4, ExposureProgram::ShutterPriority),
            (5, ExposureProgram::Creative),
            (6, ExposureProgram::Action),
            (7, ExposureProgram::Portrait),
            (8, ExposureProgram::Landscape),
        ];
        for (code, program) in named {
            let read = settings_of(EXPOSURE_PROGRAM, code);
            assert_eq!(read.exposure_program, Some(program), "{code}");
        }
    }

    #[test]
    fn an_exposure_program_exif_does_not_define_is_none() {
        // 0 is Exif's "not defined", as on recent Z bodies' samples; 9 is
        // the "bulb" that ExifTool's documentation lists and Exif does not
        // define.
        for code in [0, 9, 10, 255, 65535] {
            let read = settings_of(EXPOSURE_PROGRAM, code);
            assert_eq!(read.exposure_program, None, "{code}");
        }
    }

    #[test]
    fn every_metering_mode_exif_defines_gives_its_mode() {
        let named = [
            (1, MeteringMode::Average),
            (2, MeteringMode::CenterWeighted),
            (3, MeteringMode::Spot),
            (4, MeteringMode::MultiSpot),
            (5, MeteringMode::MultiSegment),
            (6, MeteringMode::Partial),
            (255, MeteringMode::Other),
        ];
        for (code, mode) in named {
            let read = settings_of(METERING_MODE, code);
            assert_eq!(read.metering_mode, Some(mode), "{code}");
        }
    }

    #[test]
    fn a_metering_mode_exif_does_not_define_is_none() {
        // 0 is Exif's "unknown".
        for code in [0, 7, 254, 256, 65535] {
            let read = settings_of(METERING_MODE, code);
            assert_eq!(read.metering_mode, None, "{code}");
        }
    }

    #[test]
    fn a_flash_gives_whether_it_fired_and_its_mode() {
        let fired = |mode| Some(Flash { fired: true, mode });
        let not_fired = |mode| Some(Flash { fired: false, mode });
        let cases = [
            // The four values of the public samples: the Z 9's and many
            // more, the D100's, the D750's and the D810's, and most Sony
            // bodies'. Bits 1 and 2, the return light, are not read.
            (0x00, not_fired(None)),
            (0x07, fired(None)),
            (0x0F, fired(Some(FlashMode::Fill))),
            (0x10, not_fired(Some(FlashMode::Off))),
            // Each mode, fired and not.
            (0x01, fired(None)),
            (0x08, not_fired(Some(FlashMode::Fill))),
            (0x09, fired(Some(FlashMode::Fill))),
            (0x11, fired(Some(FlashMode::Off))),
            (0x18, not_fired(Some(FlashMode::Auto))),
            (0x19, fired(Some(FlashMode::Auto))),
            // Red-eye reduction, bit 6, is not read either: 0x5F is every
            // defined bit but bit 5.
            (0x40, not_fired(None)),
            (0x59, fired(Some(FlashMode::Auto))),
            (0x5F, fired(Some(FlashMode::Auto))),
        ];
        for (bits, flash) in cases {
            assert_eq!(settings_of(FLASH, bits).flash, flash, "{bits:#06X}");
        }
    }

    #[test]
    fn a_camera_without_a_flash_function_has_a_flash_that_did_not_fire_with_no_mode() {
        // Bit 5 alone, and with each mode and the bits that are not read.
        for bits in [0x20, 0x28, 0x30, 0x38, 0x26, 0x60, 0x7E] {
            assert_eq!(
                settings_of(FLASH, bits).flash,
                Some(Flash::new(false)),
                "{bits:#06X}"
            );
        }
    }

    #[test]
    fn a_flash_that_contradicts_itself_or_sets_a_reserved_bit_is_none() {
        // A flash that fired on a camera without a flash function: alone,
        // with a mode, and as every defined bit (0x7F).
        for bits in [0x21, 0x39, 0x7F] {
            assert_eq!(settings_of(FLASH, bits).flash, None, "{bits:#06X}");
        }
        // Each reserved bit, 7 to 15, alone and beside a sound value.
        for bit in 7..16 {
            for sound in [0x00, 0x10, 0x0F] {
                let bits = sound | 1 << bit;
                assert_eq!(settings_of(FLASH, bits).flash, None, "{bits:#06X}");
            }
        }
    }

    /// The white balance mode read from [`settings`]'s file with the
    /// WhiteBalance `white_balance` and the LightSource `light`, each one
    /// SHORT when `Some`.
    fn white_balance(white_balance: Option<u16>, light: Option<u16>) -> Option<WhiteBalanceMode> {
        settings(|builder, exif| {
            if let Some(code) = white_balance {
                builder.short(exif, WHITE_BALANCE, &[code]);
            }
            if let Some(code) = light {
                builder.short(exif, LIGHT_SOURCE, &[code]);
            }
        })
        .white_balance
        .mode
    }

    #[test]
    fn an_auto_white_balance_is_auto_whatever_the_light_source() {
        for light in [None, Some(0), Some(1), Some(11), Some(17), Some(255)] {
            assert_eq!(
                white_balance(Some(0), light),
                Some(WhiteBalanceMode::Auto),
                "{light:?}"
            );
        }
    }

    #[test]
    fn a_manual_white_balance_is_the_mode_its_light_source_names() {
        let named = [
            (1, WhiteBalanceMode::Daylight),
            (9, WhiteBalanceMode::Daylight),
            (10, WhiteBalanceMode::Cloudy),
            (11, WhiteBalanceMode::Shade),
            (3, WhiteBalanceMode::Tungsten),
            (24, WhiteBalanceMode::Tungsten),
            (2, WhiteBalanceMode::Fluorescent),
            (12, WhiteBalanceMode::Fluorescent),
            (13, WhiteBalanceMode::Fluorescent),
            (14, WhiteBalanceMode::Fluorescent),
            (15, WhiteBalanceMode::Fluorescent),
            (16, WhiteBalanceMode::Fluorescent),
            (4, WhiteBalanceMode::Flash),
        ];
        for (light, mode) in named {
            assert_eq!(white_balance(Some(1), Some(light)), Some(mode), "{light}");
        }
        // Every other code: 0, "unknown", the standard lights 17 to 23,
        // the light sources and LEDs 25 to 34, 255, "other", and codes
        // Exif does not define.
        let others = (0..=40).chain([254, 255, 256, 65535]);
        for light in others.filter(|light| named.iter().all(|&(code, _)| code != *light)) {
            assert_eq!(white_balance(Some(1), Some(light)), None, "{light}");
        }
    }

    #[test]
    fn a_manual_white_balance_without_a_light_source_it_can_read_is_none() {
        assert_eq!(white_balance(Some(1), None), None);
        let read = settings(|builder, exif| {
            builder.short(exif, WHITE_BALANCE, &[1]);
            builder.long(exif, LIGHT_SOURCE, &[11]);
        });
        assert_eq!(read.white_balance.mode, None);
    }

    #[test]
    fn a_white_balance_exif_does_not_define_or_none_is_none() {
        for code in [2, 255, 65535] {
            assert_eq!(white_balance(Some(code), Some(11)), None, "{code}");
        }
        // A LightSource alone says nothing of the white balance.
        assert_eq!(white_balance(None, Some(11)), None);
    }

    /// What the InteropIndex of [`color_space`]'s file is.
    #[derive(Clone, Copy, Debug)]
    enum Index {
        /// No Interoperability IFD at all.
        NoIfd,
        /// An Interoperability IFD without an InteropIndex.
        Missing,
        /// This text, as ASCII with the NUL the builder adds.
        Text(&'static str),
        /// These bytes, of type UNDEFINED.
        Undefined(&'static [u8]),
    }

    /// The color space read from [`settings`]'s file with the ColorSpace
    /// `code`, one SHORT, when `Some`, and the Interoperability IFD
    /// `index` says, which holds the InteroperabilityVersion too.
    fn color_space(code: Option<u16>, index: Index) -> Option<ColorSpace> {
        settings(|builder, exif| {
            if let Some(code) = code {
                builder.short(exif, COLOR_SPACE, &[code]);
            }
            if matches!(index, Index::NoIfd) {
                return;
            }
            let interop = builder.ifd();
            builder
                .interop(exif, interop)
                .undefined(interop, INTEROP_VERSION, b"0100");
            match index {
                Index::NoIfd | Index::Missing => {}
                Index::Text(text) => {
                    builder.ascii(interop, INTEROP_INDEX, text);
                }
                Index::Undefined(bytes) => {
                    builder.undefined(interop, INTEROP_INDEX, bytes);
                }
            }
        })
        .color_space
    }

    #[test]
    fn a_color_space_of_srgb_is_srgb_whatever_the_index() {
        // `R98` beside it, as on 39 of the public samples; no index, as
        // on two; and `R03`, which does not overrule it.
        for index in [
            Index::Text("R98"),
            Index::NoIfd,
            Index::Missing,
            Index::Text("R03"),
        ] {
            assert_eq!(
                color_space(Some(1), index),
                Some(ColorSpace::Srgb),
                "{index:?}"
            );
        }
    }

    #[test]
    fn an_uncalibrated_color_space_with_the_dcf_option_files_index_is_adobe_rgb() {
        // As on the public A900 and DSC-R1 samples. The index is text,
        // cleaned as every text is, so the spaces that pad it are dropped.
        for text in ["R03", "R03 ", " R03  "] {
            assert_eq!(
                color_space(Some(65535), Index::Text(text)),
                Some(ColorSpace::AdobeRgb),
                "{text:?}"
            );
        }
    }

    #[test]
    fn an_uncalibrated_color_space_without_the_dcf_option_files_index_is_none() {
        let cases = [
            // Another index, an empty one, as on the public DSLR-A100
            // sample, or one that is not exactly `R03`.
            Index::Text("R98"),
            Index::Text(""),
            Index::Text("r03"),
            Index::Text("R030"),
            Index::Text("R0 3"),
            // An index of the wrong type, none, and no Interoperability
            // IFD, as on a JPEG a program rewrote.
            Index::Undefined(b"R03\0"),
            Index::Missing,
            Index::NoIfd,
        ];
        for index in cases {
            assert_eq!(color_space(Some(65535), index), None, "{index:?}");
        }
    }

    #[test]
    fn the_index_alone_or_beside_an_undefined_color_space_is_none() {
        // No ColorSpace, as in every Nikon raw file checked, and codes Exif
        // does not define: `R03` does not make them Adobe RGB.
        for code in [None, Some(0), Some(2), Some(65534)] {
            assert_eq!(color_space(code, Index::Text("R03")), None, "{code:?}");
        }
    }

    #[test]
    fn the_index_is_read_from_the_interoperability_ifd_alone() {
        // Tag 0x0001 means the InteropIndex only there: in the Exif IFD it
        // is not read.
        let read = settings(|builder, exif| {
            builder
                .short(exif, COLOR_SPACE, &[65535])
                .ascii(exif, INTEROP_INDEX, "R03");
        });
        assert_eq!(read.color_space, None);
    }

    #[test]
    fn the_index_is_read_from_an_interoperability_ifd_that_is_also_ifd1() {
        // The Nikon D70's layout: IFD0's next-IFD pointer and the Exif
        // IFD's Interoperability IFD tag both lead to IFD1, which the walk
        // keeps in the chain. Its InteropIndex is still read.
        for (code, expected) in [(65535, ColorSpace::AdobeRgb), (1, ColorSpace::Srgb)] {
            let (mut src, tree) = testing::file(|builder, ifd0| {
                let (exif, ifd1) = (builder.ifd(), builder.ifd());
                builder
                    .exif(ifd0, exif)
                    .short(exif, COLOR_SPACE, &[code])
                    .next(ifd0, Some(ifd1))
                    .interop(exif, ifd1)
                    .undefined(ifd1, INTEROP_VERSION, b"0100")
                    .ascii(ifd1, INTEROP_INDEX, "R03");
            });
            // IFD0, the Exif IFD and IFD1, which is not also placed as an
            // Interoperability IFD: the Exif IFD alone holds no image.
            assert_eq!(tree.iter().count(), 3);
            let no_image = tree.iter().filter(|(link, _)| link.holds_no_image());
            assert_eq!(no_image.count(), 1);
            let read = read(&mut src, &tree).unwrap().metadata.camera_settings;
            assert_eq!(read.color_space, Some(expected), "{code}");
        }
    }

    #[test]
    fn a_color_space_entry_in_the_exif_ifd_is_reported_whatever_it_holds() {
        // Whether the Exif IFD holds a ColorSpace, of [`settings`]'s file
        // with what `build` adds to that IFD.
        let records = |build: fn(&mut TiffBuilder, IfdId)| {
            let (mut src, tree) = testing::file(|builder, ifd| {
                let exif = builder.ifd();
                builder
                    .exif(ifd, exif)
                    .rational(exif, EXPOSURE_TIME, &[(10, 3200)]);
                build(builder, exif);
            });
            read(&mut src, &tree).unwrap().exif_records_color_space
        };
        // sRGB, "uncalibrated" with no index, which gives no color space,
        // a code Exif does not define, a LONG and two SHORTs, which are
        // not read: each is the Exif IFD's word on the color space.
        assert!(records(|builder, exif| {
            builder.short(exif, COLOR_SPACE, &[1]);
        }));
        assert!(records(|builder, exif| {
            builder.short(exif, COLOR_SPACE, &[65535]);
        }));
        assert!(records(|builder, exif| {
            builder.short(exif, COLOR_SPACE, &[2]);
        }));
        assert!(records(|builder, exif| {
            builder.long(exif, COLOR_SPACE, &[1]);
        }));
        assert!(records(|builder, exif| {
            builder.short(exif, COLOR_SPACE, &[1, 1]);
        }));
        // No ColorSpace, as in every Nikon raw file checked.
        assert!(!records(|_, _| {}));
        // No Exif IFD at all.
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.short(ifd, COLOR_SPACE, &[1]);
        });
        assert!(!read(&mut src, &tree).unwrap().exif_records_color_space);
    }

    #[test]
    fn a_color_space_entry_the_exif_ifd_skips_as_damaged_is_not_reported() {
        // A ColorSpace of a type TIFF does not define, and one of four
        // SHORTs whose value lies past the end of the file: the IFD's
        // reader skips each, as any damaged entry, so the Exif IFD keeps
        // no ColorSpace, and a MakerNote may stand in.
        type Damage = fn(&mut TiffBuilder, IfdId);
        let damaged: [(&str, Damage); 2] = [
            ("unknown type", |builder, exif| {
                builder.raw(exif, COLOR_SPACE, 99, 1, RawValue::Inline([1, 0, 0, 0]));
            }),
            ("past the file", |builder, exif| {
                builder.raw(exif, COLOR_SPACE, SHORT, 4, RawValue::Offset(4096));
            }),
        ];
        for (what, build) in damaged {
            let (mut src, tree) = testing::file(|builder, ifd| {
                let exif = builder.ifd();
                builder
                    .exif(ifd, exif)
                    .rational(exif, EXPOSURE_TIME, &[(10, 3200)]);
                build(builder, exif);
            });
            assert!(src.len() < 4096, "{what}");
            let exif = tree.exif().expect("an Exif IFD");
            assert_eq!(exif.invalid_entries(), 1, "{what}");
            let standard = read(&mut src, &tree).unwrap();
            assert!(!standard.exif_records_color_space, "{what}");
        }
    }

    /// The settings of [`full_settings_except`]'s file, each one SHORT:
    /// aperture priority, multi-segment metering, a flash that fired in
    /// fill mode, a manual white balance for shade, and a color space
    /// that is "uncalibrated", which the InteropIndex `R03` beside it makes
    /// Adobe RGB.
    const FULL_SETTINGS: [(u16, u16); 6] = [
        (EXPOSURE_PROGRAM, 3),
        (METERING_MODE, 5),
        (FLASH, 15),
        (WHITE_BALANCE, 1),
        (LIGHT_SOURCE, 11),
        (COLOR_SPACE, 65535),
    ];

    /// The metadata of a file whose IFD0 holds the Make `SONY`, whose Exif
    /// IFD holds the ExposureTime 1/320 s and [`FULL_SETTINGS`], and whose
    /// Interoperability IFD holds the InteroperabilityVersion and the
    /// InteropIndex `R03`, all but tag `except`. `change` is then given
    /// the builder and the IDs of the Exif and Interoperability IFDs, to
    /// add what the test needs in its place.
    fn full_settings_except(
        except: u16,
        change: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
    ) -> Metadata {
        metadata_of(|builder, ifd0| {
            let (exif, interop) = (builder.ifd(), builder.ifd());
            builder
                .exif(ifd0, exif)
                .interop(exif, interop)
                .rational(exif, EXPOSURE_TIME, &[(10, 3200)])
                .undefined(interop, INTEROP_VERSION, b"0100");
            for &(tag, code) in FULL_SETTINGS.iter().filter(|(tag, _)| *tag != except) {
                builder.short(exif, tag, &[code]);
            }
            if except != INTEROP_INDEX {
                builder.ascii(interop, INTEROP_INDEX, "R03");
            }
            change(builder, exif, interop);
        })
    }

    /// What [`full_settings_except`]'s whole file gives, with `change`
    /// made to its settings.
    fn full_settings_expected(change: impl FnOnce(&mut CameraSettings)) -> Metadata {
        let mut settings = CameraSettings {
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some(Flash::new(true).with_mode(FlashMode::Fill)),
            white_balance: WhiteBalance {
                mode: Some(WhiteBalanceMode::Shade),
                ..WhiteBalance::default()
            },
            color_space: Some(ColorSpace::AdobeRgb),
            ..CameraSettings::default()
        };
        change(&mut settings);
        Metadata {
            camera: Camera::default().with_recorded_make("SONY"),
            exposure: Exposure {
                exposure_time: Some(fraction(10, 3200)),
                ..Exposure::default()
            },
            camera_settings: settings,
            ..Metadata::default()
        }
    }

    #[test]
    fn every_setting_exif_records_is_read() {
        assert_eq!(
            full_settings_except(0, |_, _, _| {}),
            full_settings_expected(|_| {})
        );
    }

    #[test]
    fn a_setting_missing_or_of_another_type_or_count_is_none_and_costs_no_other() {
        type Unset = fn(&mut CameraSettings);
        let settings: [(u16, Unset); 6] = [
            (EXPOSURE_PROGRAM, |settings| {
                settings.exposure_program = None
            }),
            (METERING_MODE, |settings| settings.metering_mode = None),
            (FLASH, |settings| settings.flash = None),
            (WHITE_BALANCE, |settings| settings.white_balance.mode = None),
            (LIGHT_SOURCE, |settings| settings.white_balance.mode = None),
            (COLOR_SPACE, |settings| settings.color_space = None),
        ];
        // Each with the code that gives a value as one SHORT.
        type Damage = fn(&mut TiffBuilder, IfdId, u16, u16);
        let damages: [(&str, Damage); 6] = [
            ("missing", |_, _, _, _| {}),
            ("a LONG", |builder, exif, tag, code| {
                builder.long(exif, tag, &[u32::from(code)]);
            }),
            ("a BYTE", |builder, exif, tag, code| {
                builder.byte(exif, tag, &[u8::try_from(code).unwrap_or(u8::MAX)]);
            }),
            ("two SHORTs", |builder, exif, tag, code| {
                builder.short(exif, tag, &[code, code]);
            }),
            ("no SHORT", |builder, exif, tag, _| {
                builder.short(exif, tag, &[]);
            }),
            ("too long to read", |builder, exif, tag, _| {
                too_long_of(builder, exif, tag, SHORT, 2);
            }),
        ];
        for (tag, unset) in settings {
            let code = FULL_SETTINGS.iter().find(|&&(t, _)| t == tag).unwrap().1;
            for (damage, add) in damages {
                let metadata = full_settings_except(tag, |builder, exif, _| {
                    add(builder, exif, tag, code);
                });
                assert_eq!(
                    metadata,
                    full_settings_expected(unset),
                    "{tag:#06X}: {damage}"
                );
            }
        }
    }

    #[test]
    fn a_damaged_index_costs_only_the_color_space() {
        type Damage = fn(&mut TiffBuilder, IfdId);
        let damages: [(&str, Damage); 5] = [
            ("missing", |_, _| {}),
            ("UNDEFINED", |builder, interop| {
                builder.undefined(interop, INTEROP_INDEX, b"R03\0");
            }),
            ("BYTE", |builder, interop| {
                builder.byte(interop, INTEROP_INDEX, b"R03\0");
            }),
            ("of 256 bytes", |builder, interop| {
                builder.ascii(interop, INTEROP_INDEX, &format!("R03{}", " ".repeat(253)));
            }),
            ("too long to read", |builder, interop| {
                too_long(builder, interop, INTEROP_INDEX);
            }),
        ];
        for (damage, add) in damages {
            let metadata = full_settings_except(INTEROP_INDEX, |builder, _, interop| {
                add(builder, interop);
            });
            assert_eq!(
                metadata,
                full_settings_expected(|settings| settings.color_space = None),
                "{damage}"
            );
        }
    }

    #[test]
    fn a_failure_of_the_reader_reading_the_index_is_the_readers_error() {
        // The index stored far away, padded with NULs so that it is longer
        // than four bytes and so not held in its entry. The settings read
        // as one SHORT are held in their entries, so reading them never
        // reads the file, and no failure of the reader can reach them.
        let far = u32::try_from(2 * FAR).unwrap();
        let value = b"R03\0\0\0\0\0";
        let count = u32::try_from(value.len()).unwrap();
        let mut builder = TiffBuilder::new(Order::Little);
        let (ifd0, exif, interop) = (builder.ifd(), builder.ifd(), builder.ifd());
        builder
            .exif(ifd0, exif)
            .interop(exif, interop)
            .short(exif, COLOR_SPACE, &[65535])
            .blob_at(2 * FAR, value);
        builder.raw(interop, INTEROP_INDEX, ASCII, count, RawValue::Offset(far));
        let bytes = builder.build().bytes;
        // The file is sound, for a reader that does not fail.
        assert_eq!(
            read_bytes(bytes.clone()).camera_settings.color_space,
            Some(ColorSpace::AdobeRgb)
        );

        let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        let err = read(&mut src, &tree).expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }

    /// The size of the raw data in the files of the size tests.
    const RAW: Size = Size {
        width: 6000,
        height: 4000,
    };

    /// The PhotometricInterpretation of a color filter array: raw data,
    /// as the preview finder takes it.
    const CFA: u16 = 32803;

    /// Gives `ifd` the tags of raw data: samples of a color filter array,
    /// and the ImageWidth `width` and ImageLength `height`, as LONGs.
    fn raw_tags(builder: &mut TiffBuilder, ifd: IfdId, width: u32, height: u32) {
        builder
            .short(ifd, PHOTOMETRIC_INTERPRETATION, &[CFA])
            .long(ifd, IMAGE_WIDTH, &[width])
            .long(ifd, IMAGE_LENGTH, &[height]);
    }

    /// What [`read`] reads from a little-endian file whose IFD0 holds the
    /// Make `SONY` and what `build` adds, given the builder and IFD0.
    fn standard_of(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> Standard {
        let (mut src, tree) = testing::file(|builder, ifd| {
            builder.ascii(ifd, MAKE, "SONY");
            build(builder, ifd);
        });
        read(&mut src, &tree).unwrap()
    }

    /// The metadata of [`standard_of`]'s file.
    fn metadata_of(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> Metadata {
        standard_of(build).metadata
    }

    /// The image size of a file whose IFD0 holds raw data of size [`RAW`]
    /// and what `crop` adds; the raw size must be read.
    fn image_size(crop: impl FnOnce(&mut TiffBuilder, IfdId)) -> Option<Size> {
        let metadata = metadata_of(|builder, ifd| {
            raw_tags(builder, ifd, RAW.width, RAW.height);
            crop(builder, ifd);
        });
        assert_eq!(metadata.raw_size, Some(RAW));
        metadata.image_size
    }

    /// The image size of a file whose IFD0 holds raw data of size [`RAW`]
    /// and the default crop `origin` and `size`, as LONGs.
    fn cropped(origin: [u32; 2], size: [u32; 2]) -> Option<Size> {
        image_size(|builder, ifd| {
            builder
                .long(ifd, DEFAULT_CROP_ORIGIN, &origin)
                .long(ifd, DEFAULT_CROP_SIZE, &size);
        })
    }

    #[test]
    fn a_crop_inside_the_raw_is_the_image_size() {
        assert_eq!(
            cropped([12, 8], [5976, 3984]),
            Some(Size {
                width: 5976,
                height: 3984
            })
        );
    }

    #[test]
    fn a_crop_that_reaches_the_raws_edge_is_inside_it() {
        assert_eq!(
            cropped([24, 0], [5976, 4000]),
            Some(Size {
                width: 5976,
                height: 4000
            })
        );
        assert_eq!(
            cropped([0, 16], [6000, 3984]),
            Some(Size {
                width: 6000,
                height: 3984
            })
        );
        assert_eq!(cropped([0, 0], [6000, 4000]), Some(RAW));
    }

    #[test]
    fn a_crop_outside_the_raw_gives_the_raw_size() {
        assert_eq!(cropped([12, 8], [5990, 3984]), Some(RAW));
        assert_eq!(cropped([12, 8], [5976, 3993]), Some(RAW));
        assert_eq!(cropped([0, 0], [6001, 4000]), Some(RAW));
        assert_eq!(cropped([6000, 0], [1, 4000]), Some(RAW));
    }

    #[test]
    fn a_crop_whose_end_overflows_gives_the_raw_size() {
        // The sum would wrap round to 1, inside the raw.
        assert_eq!(cropped([u32::MAX, 0], [2, 4000]), Some(RAW));
        assert_eq!(cropped([0, u32::MAX], [6000, 2]), Some(RAW));
    }

    #[test]
    fn a_crop_of_zero_size_gives_the_raw_size() {
        assert_eq!(cropped([12, 8], [0, 3984]), Some(RAW));
        assert_eq!(cropped([12, 8], [5976, 0]), Some(RAW));
    }

    #[test]
    fn a_crop_of_shorts_or_of_whole_fractions_is_read() {
        let expected = Some(Size {
            width: 5976,
            height: 3984,
        });
        let shorts = image_size(|builder, ifd| {
            builder.short(ifd, DEFAULT_CROP_ORIGIN, &[12, 8]).short(
                ifd,
                DEFAULT_CROP_SIZE,
                &[5976, 3984],
            );
        });
        assert_eq!(shorts, expected);
        let fractions = image_size(|builder, ifd| {
            builder
                .rational(ifd, DEFAULT_CROP_ORIGIN, &[(12, 1), (8, 1)])
                .rational(ifd, DEFAULT_CROP_SIZE, &[(5976, 1), (3984, 1)]);
        });
        assert_eq!(fractions, expected);
    }

    #[test]
    fn a_crop_of_fractions_that_are_not_whole_gives_the_raw_size() {
        let origin = image_size(|builder, ifd| {
            builder
                .rational(ifd, DEFAULT_CROP_ORIGIN, &[(24, 2), (8, 1)])
                .long(ifd, DEFAULT_CROP_SIZE, &[5976, 3984]);
        });
        assert_eq!(origin, Some(RAW));
        let size = image_size(|builder, ifd| {
            builder.long(ifd, DEFAULT_CROP_ORIGIN, &[12, 8]).rational(
                ifd,
                DEFAULT_CROP_SIZE,
                &[(5976, 1), (7968, 2)],
            );
        });
        assert_eq!(size, Some(RAW));
    }

    #[test]
    fn a_crop_of_other_than_two_values_gives_the_raw_size() {
        assert_eq!(cropped_counts(&[12, 8, 0], &[5976, 3984]), Some(RAW));
        assert_eq!(cropped_counts(&[12, 8], &[5976, 3984, 1]), Some(RAW));
        assert_eq!(cropped_counts(&[12], &[5976, 3984]), Some(RAW));
        assert_eq!(cropped_counts(&[12, 8], &[5976]), Some(RAW));
        let fractions = image_size(|builder, ifd| {
            builder.long(ifd, DEFAULT_CROP_ORIGIN, &[12, 8]).rational(
                ifd,
                DEFAULT_CROP_SIZE,
                &[(5976, 1), (3984, 1), (1, 1)],
            );
        });
        assert_eq!(fractions, Some(RAW));
    }

    /// The image size of a file whose IFD0 holds raw data of size [`RAW`]
    /// and the crop tags with these values, as LONGs, of any count.
    fn cropped_counts(origin: &[u32], size: &[u32]) -> Option<Size> {
        image_size(|builder, ifd| {
            builder
                .long(ifd, DEFAULT_CROP_ORIGIN, origin)
                .long(ifd, DEFAULT_CROP_SIZE, size);
        })
    }

    #[test]
    fn half_a_crop_gives_the_raw_size() {
        let origin_only = image_size(|builder, ifd| {
            builder.long(ifd, DEFAULT_CROP_ORIGIN, &[12, 8]);
        });
        assert_eq!(origin_only, Some(RAW));
        let size_only = image_size(|builder, ifd| {
            builder.long(ifd, DEFAULT_CROP_SIZE, &[5976, 3984]);
        });
        assert_eq!(size_only, Some(RAW));
    }

    #[test]
    fn no_crop_gives_the_raw_size() {
        assert_eq!(image_size(|_, _| {}), Some(RAW));
    }

    #[test]
    fn only_a_default_crop_is_reported_as_one() {
        let crop = |origin: [u32; 2], size: [u32; 2]| {
            move |builder: &mut TiffBuilder, ifd| {
                raw_tags(builder, ifd, RAW.width, RAW.height);
                builder
                    .long(ifd, DEFAULT_CROP_ORIGIN, &origin)
                    .long(ifd, DEFAULT_CROP_SIZE, &size);
            }
        };
        // A crop as large as the raw is a crop all the same, which the
        // image size alone could not tell from the raw size.
        for (origin, size) in [([12, 8], [5976, 3984]), ([0, 0], [6000, 4000])] {
            let standard = standard_of(crop(origin, size));
            assert!(standard.from_default_crop, "{origin:?}, {size:?}");
            assert_eq!(
                standard.metadata.image_size,
                Some(Size {
                    width: size[0],
                    height: size[1]
                })
            );
        }
        // A crop outside the raw, no crop, and no raw data.
        let outside = standard_of(crop([12, 8], [5990, 3984]));
        assert!(!outside.from_default_crop);
        assert_eq!(outside.metadata.image_size, Some(RAW));
        let none = standard_of(|builder, ifd| raw_tags(builder, ifd, RAW.width, RAW.height));
        assert!(!none.from_default_crop);
        assert_eq!(none.metadata.image_size, Some(RAW));
        let no_raw = standard_of(|_, _| {});
        assert!(!no_raw.from_default_crop);
        assert_eq!(no_raw.metadata.image_size, None);
    }

    #[test]
    fn a_file_without_raw_data_has_no_sizes() {
        // A preview's IFD, which declares a size of its own.
        let metadata = metadata_of(|builder, ifd| {
            builder
                .long(ifd, NEW_SUBFILE_TYPE, &[1])
                .long(ifd, IMAGE_WIDTH, &[640])
                .long(ifd, IMAGE_LENGTH, &[480]);
        });
        assert_eq!(metadata.raw_size, None);
        assert_eq!(metadata.image_size, None);
    }

    #[test]
    fn a_raw_ifd_without_a_size_is_passed_over_for_the_next() {
        type Size0 = fn(&mut TiffBuilder, IfdId);
        let cases: [(&str, Size0); 4] = [
            ("a width of zero", |builder, ifd| {
                raw_tags(builder, ifd, 0, 4000);
            }),
            ("a height of zero", |builder, ifd| {
                raw_tags(builder, ifd, 6000, 0);
            }),
            ("no ImageLength", |builder, ifd| {
                builder.short(ifd, PHOTOMETRIC_INTERPRETATION, &[CFA]).long(
                    ifd,
                    IMAGE_WIDTH,
                    &[6000],
                );
            }),
            ("an ImageWidth too long to read", |builder, ifd| {
                builder.short(ifd, PHOTOMETRIC_INTERPRETATION, &[CFA]).long(
                    ifd,
                    IMAGE_LENGTH,
                    &[4000],
                );
                too_long_of(builder, ifd, IMAGE_WIDTH, LONG, 4);
            }),
        ];
        // A size with a zero side would lose to the next IFD's anyway, by
        // its pixel count; `a_wider_raw_ifd_of_no_height_is_not_the_largest`
        // shows the zero test itself. What each case shows here is that
        // the IFD gives nothing: neither its size nor its crop.
        for (case, first) in cases {
            // The first raw IFD, IFD0, also holds a crop, which is not
            // taken for the next one's.
            let metadata = metadata_of(|builder, ifd| {
                first(builder, ifd);
                builder.long(ifd, DEFAULT_CROP_ORIGIN, &[0, 0]).long(
                    ifd,
                    DEFAULT_CROP_SIZE,
                    &[100, 100],
                );
                let sub = builder.ifd();
                builder.subifds(ifd, &[sub]);
                raw_tags(builder, sub, 3000, 2000);
            });
            let next = Size {
                width: 3000,
                height: 2000,
            };
            assert_eq!(metadata.raw_size, Some(next), "{case}");
            assert_eq!(metadata.image_size, Some(next), "{case}");
        }
    }

    /// The metadata of a file whose IFD0 is a preview's and whose two
    /// SubIFDs hold full-resolution raw data of the sizes `first` and
    /// `second`, as LONGs, in that order, each with the default crop at
    /// 0, 0 of the size its `crops` entry gives, when it gives one.
    fn two_raw_ifds(first: Size, second: Size, crops: [Option<[u32; 2]>; 2]) -> Metadata {
        metadata_of(|builder, ifd| {
            builder.long(ifd, NEW_SUBFILE_TYPE, &[1]);
            let subs = [builder.ifd(), builder.ifd()];
            builder.subifds(ifd, &subs);
            for ((sub, size), crop) in subs.into_iter().zip([first, second]).zip(crops) {
                builder.long(sub, NEW_SUBFILE_TYPE, &[0]);
                raw_tags(builder, sub, size.width, size.height);
                if let Some(crop) = crop {
                    builder.long(sub, DEFAULT_CROP_ORIGIN, &[0, 0]).long(
                        sub,
                        DEFAULT_CROP_SIZE,
                        &crop,
                    );
                }
            }
        })
    }

    #[test]
    fn the_largest_raw_ifd_gives_the_sizes() {
        // The Coolpix E5700's layout (public sample 4456): a small
        // full-resolution raw image, then the real one.
        let (small, large) = (size(656, 484), size(2576, 1924));
        for (first, second) in [(small, large), (large, small)] {
            let metadata = two_raw_ifds(first, second, [None, None]);
            assert_eq!(metadata.raw_size, Some(large), "{first:?} then {second:?}");
            assert_eq!(
                metadata.image_size,
                Some(large),
                "{first:?} then {second:?}"
            );
        }
    }

    #[test]
    fn the_crop_is_the_largest_raw_ifds() {
        let (small, large) = (size(656, 484), size(2576, 1924));
        let metadata = two_raw_ifds(small, large, [Some([600, 400]), Some([2560, 1920])]);
        assert_eq!(metadata.raw_size, Some(large));
        assert_eq!(metadata.image_size, Some(size(2560, 1920)));
        let metadata = two_raw_ifds(large, small, [None, Some([600, 400])]);
        assert_eq!(metadata.image_size, Some(large));
    }

    #[test]
    fn the_first_of_two_raw_ifds_of_equal_size_gives_the_sizes() {
        // Told apart by their crops.
        let metadata = two_raw_ifds(RAW, RAW, [Some([100, 100]), Some([200, 200])]);
        assert_eq!(metadata.raw_size, Some(RAW));
        assert_eq!(metadata.image_size, Some(size(100, 100)));
        // The same pixel count in another shape is a tie too.
        let metadata = two_raw_ifds(size(4000, 6000), RAW, [Some([100, 100]), Some([200, 200])]);
        assert_eq!(metadata.raw_size, Some(size(4000, 6000)));
        assert_eq!(metadata.image_size, Some(size(100, 100)));
    }

    #[test]
    fn a_wider_raw_ifd_of_no_height_is_not_the_largest() {
        let metadata = two_raw_ifds(RAW, size(60_000, 0), [None, None]);
        assert_eq!(metadata.raw_size, Some(RAW));
        let metadata = two_raw_ifds(size(0, 40_000), RAW, [None, None]);
        assert_eq!(metadata.raw_size, Some(RAW));
        // Alone, it gives no sizes: it has no pixels.
        for (width, height) in [(60_000, 0), (0, 40_000)] {
            let metadata = metadata_of(|builder, ifd| raw_tags(builder, ifd, width, height));
            assert_eq!(metadata.raw_size, None, "{width} by {height}");
            assert_eq!(metadata.image_size, None, "{width} by {height}");
        }
    }

    #[test]
    fn pixel_counts_beyond_32_bits_are_compared_whole() {
        // 65536 by 65536 is 2^32 pixels, which in 32 bits would wrap to 0
        // and lose to the smaller one.
        let huge = size(65_536, 65_536);
        let metadata = two_raw_ifds(RAW, huge, [None, None]);
        assert_eq!(metadata.raw_size, Some(huge));
    }

    #[test]
    fn the_exif_ifd_never_gives_the_sizes() {
        // Raw-looking tags in the Exif IFD, where no image is.
        let metadata = metadata_of(|builder, ifd| {
            let exif = builder.ifd();
            builder.exif(ifd, exif);
            raw_tags(builder, exif, RAW.width, RAW.height);
        });
        assert_eq!(metadata.raw_size, None);
        assert_eq!(metadata.image_size, None);
    }

    #[test]
    fn the_interoperability_ifd_never_gives_the_sizes() {
        // Raw-looking tags in the Interoperability IFD, where no image is.
        let metadata = metadata_of(|builder, ifd| {
            let (exif, interop) = (builder.ifd(), builder.ifd());
            builder.exif(ifd, exif).interop(exif, interop);
            raw_tags(builder, interop, RAW.width, RAW.height);
        });
        assert_eq!(metadata.raw_size, None);
        assert_eq!(metadata.image_size, None);
    }

    #[test]
    fn an_ifd_whose_role_cannot_be_read_is_passed_over() {
        // IFD0's PhotometricInterpretation cannot be read, so whether it
        // holds raw data is unknown, as for the preview finder.
        let metadata = metadata_of(|builder, ifd| {
            too_long_of(builder, ifd, PHOTOMETRIC_INTERPRETATION, SHORT, 2);
            builder
                .long(ifd, IMAGE_WIDTH, &[RAW.width])
                .long(ifd, IMAGE_LENGTH, &[RAW.height]);
            let sub = builder.ifd();
            builder.subifds(ifd, &[sub]);
            raw_tags(builder, sub, 3000, 2000);
        });
        let next = Size {
            width: 3000,
            height: 2000,
        };
        assert_eq!(metadata.raw_size, Some(next));
        assert_eq!(metadata.image_size, Some(next));
    }

    #[test]
    fn damage_to_the_crop_costs_only_the_crop() {
        type Damage = fn(&mut TiffBuilder, IfdId);
        let cases: [(&str, Damage); 2] = [
            ("a DefaultCropOrigin too long to read", |builder, ifd| {
                too_long_of(builder, ifd, DEFAULT_CROP_ORIGIN, LONG, 4);
                builder.long(ifd, DEFAULT_CROP_SIZE, &[5976, 3984]);
            }),
            ("a DefaultCropSize too long to read", |builder, ifd| {
                builder.long(ifd, DEFAULT_CROP_ORIGIN, &[12, 8]);
                too_long_of(builder, ifd, DEFAULT_CROP_SIZE, RATIONAL, 8);
            }),
        ];
        for (case, damage) in cases {
            let metadata = metadata_of(|builder, ifd| {
                raw_tags(builder, ifd, RAW.width, RAW.height);
                builder.short(ifd, ORIENTATION, &[6]);
                damage(builder, ifd);
            });
            assert_eq!(
                metadata,
                Metadata {
                    camera: Camera::default().with_recorded_make("SONY"),
                    orientation: Orientation::Rotate90,
                    raw_size: Some(RAW),
                    image_size: Some(RAW),
                    ..Metadata::default()
                },
                "{case}"
            );
        }
    }

    #[test]
    fn a_failure_of_the_reader_reading_the_sizes_is_the_readers_error() {
        /// `values` as little-endian LONGs.
        fn longs(values: &[u32]) -> Vec<u8> {
            values.iter().flat_map(|v| v.to_le_bytes()).collect()
        }
        let crop = Size {
            width: 5976,
            height: 3984,
        };
        // Each value stored far away, so that it is read through the
        // failing part of the reader, with more than four bytes, so not
        // held in its entry; the other tags of the raw IFD are inline.
        let cases = [
            (IMAGE_WIDTH, LONG, 2, longs(&[6000, 0])),
            (IMAGE_LENGTH, LONG, 2, longs(&[4000, 0])),
            (DEFAULT_CROP_ORIGIN, LONG, 2, longs(&[12, 8])),
            (DEFAULT_CROP_SIZE, LONG, 2, longs(&[5976, 3984])),
            (DEFAULT_CROP_SIZE, RATIONAL, 2, longs(&[5976, 1, 3984, 1])),
        ];
        let far = u32::try_from(2 * FAR).unwrap();
        for (tag, typ, count, value) in cases {
            let label = format!("{tag:#06X}, type {typ}");
            let mut builder = TiffBuilder::new(Order::Little);
            let ifd = builder.ifd();
            builder
                .short(ifd, PHOTOMETRIC_INTERPRETATION, &[CFA])
                .blob_at(2 * FAR, &value);
            builder.raw(ifd, tag, typ, count, RawValue::Offset(far));
            let inline: [(u16, [u32; 2]); 4] = [
                (IMAGE_WIDTH, [6000, 0]),
                (IMAGE_LENGTH, [4000, 0]),
                (DEFAULT_CROP_ORIGIN, [12, 8]),
                (DEFAULT_CROP_SIZE, [5976, 3984]),
            ];
            for (other, values) in inline.into_iter().filter(|&(other, _)| other != tag) {
                if other == IMAGE_WIDTH || other == IMAGE_LENGTH {
                    builder.long(ifd, other, &values[..1]);
                } else {
                    builder.short(ifd, other, &values.map(|v| u16::try_from(v).unwrap()));
                }
            }
            let bytes = builder.build().bytes;
            // The file is sound, for a reader that does not fail.
            let metadata = read_bytes(bytes.clone());
            assert_eq!(metadata.raw_size, Some(RAW), "{label}");
            assert_eq!(metadata.image_size, Some(crop), "{label}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{label}: {err:?}");
        }
    }

    #[test]
    fn a_default_crop_of_another_count_than_two_is_not_read() {
        /// `values` as little-endian LONGs.
        fn longs(values: &[u32]) -> Vec<u8> {
            values.iter().flat_map(|v| v.to_le_bytes()).collect()
        }
        // Each of the default crop's tags of three values, as integers or
        // as fractions, stored far away, where every read fails. Its count
        // is checked before its values are read, so it gives no crop, and
        // reading the metadata succeeds, with the raw size as the image
        // size.
        let cases = [
            (DEFAULT_CROP_ORIGIN, LONG, longs(&[12, 8, 0])),
            (DEFAULT_CROP_SIZE, LONG, longs(&[5976, 3984, 0])),
            (DEFAULT_CROP_ORIGIN, RATIONAL, longs(&[12, 1, 8, 1, 0, 1])),
            (
                DEFAULT_CROP_SIZE,
                RATIONAL,
                longs(&[5976, 1, 3984, 1, 0, 1]),
            ),
        ];
        let far = u32::try_from(2 * FAR).unwrap();
        for (tag, typ, value) in cases {
            let label = format!("{tag:#06X}, type {typ}");
            let mut builder = TiffBuilder::new(Order::Little);
            let ifd = builder.ifd();
            builder
                .short(ifd, PHOTOMETRIC_INTERPRETATION, &[CFA])
                .long(ifd, IMAGE_WIDTH, &[RAW.width])
                .long(ifd, IMAGE_LENGTH, &[RAW.height])
                .blob_at(2 * FAR, &value);
            builder.raw(ifd, tag, typ, 3, RawValue::Offset(far));
            let other = if tag == DEFAULT_CROP_ORIGIN {
                (DEFAULT_CROP_SIZE, [5976, 3984])
            } else {
                (DEFAULT_CROP_ORIGIN, [12, 8])
            };
            builder.short(ifd, other.0, &other.1);
            let bytes = builder.build().bytes;
            // A reader that does not fail gives the same.
            let metadata = read_bytes(bytes.clone());
            assert_eq!(metadata.image_size, Some(RAW), "{label}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let read = read(&mut src, &tree).unwrap_or_else(|err| panic!("{label}: {err:?}"));
            assert_eq!(read.metadata.raw_size, Some(RAW), "{label}");
            assert_eq!(read.metadata.image_size, Some(RAW), "{label}");
        }
    }

    /// The GPS tags the reader never reads, written by the files below as
    /// cameras write them: GPSVersionID, GPSSatellites, GPSStatus,
    /// GPSMeasureMode and GPSDifferential.
    const GPS_VERSION_ID: u16 = 0x0000;
    const GPS_SATELLITES: u16 = 0x0008;
    const GPS_STATUS: u16 = 0x0009;
    const GPS_MEASURE_MODE: u16 = 0x000A;
    const GPS_DIFFERENTIAL: u16 = 0x001E;

    /// A value of a GPS tag, as the builder writes it.
    #[derive(Clone, Copy, Debug)]
    enum Gps {
        Byte(&'static [u8]),
        Ascii(&'static str),
        Rationals(&'static [(u32, u32)]),
        Short(&'static [u16]),
    }

    /// The GPS IFD a Nikon camera writes with a position, in the form the
    /// NEF notes record: degrees and minutes with four decimals and no
    /// seconds, an altitude above the reference, the time with seconds
    /// over 100, the satellites and a datum of 16 spaces, in 17 bytes. The
    /// position is the Eiffel Tower's; the time is made up.
    const NIKON_GPS: [(u16, Gps); 11] = [
        (GPS_VERSION_ID, Gps::Byte(&[2, 3, 0, 0])),
        (GPS_LATITUDE_REF, Gps::Ascii("N")),
        (
            GPS_LATITUDE,
            Gps::Rationals(&[(48, 1), (514_933, 10_000), (0, 1)]),
        ),
        (GPS_LONGITUDE_REF, Gps::Ascii("E")),
        (
            GPS_LONGITUDE,
            Gps::Rationals(&[(2, 1), (176_700, 10_000), (0, 1)]),
        ),
        (GPS_ALTITUDE_REF, Gps::Byte(&[0])),
        (GPS_ALTITUDE, Gps::Rationals(&[(35_050, 1000)])),
        (
            GPS_TIME_STAMP,
            Gps::Rationals(&[(8, 1), (20, 1), (3_045, 100)]),
        ),
        (GPS_SATELLITES, Gps::Ascii("07")),
        (GPS_MAP_DATUM, Gps::Ascii("                ")),
        (GPS_DATE_STAMP, Gps::Ascii("2024:05:01")),
    ];

    /// The GPS IFD a Sony camera writes with a fix, in the form the ARW
    /// notes record: degrees, minutes and seconds with three decimals, no
    /// altitude, the time with seconds over 1000, the status `A`, the
    /// measure mode, the datum `WGS-84` and the differential correction.
    /// The position is the Statue of Liberty's; the time is made up.
    const SONY_GPS: [(u16, Gps); 10] = [
        (GPS_VERSION_ID, Gps::Byte(&[2, 3, 0, 0])),
        (GPS_LATITUDE_REF, Gps::Ascii("N")),
        (
            GPS_LATITUDE,
            Gps::Rationals(&[(40, 1), (41, 1), (21_400, 1000)]),
        ),
        (GPS_LONGITUDE_REF, Gps::Ascii("W")),
        (
            GPS_LONGITUDE,
            Gps::Rationals(&[(74, 1), (2, 1), (40_200, 1000)]),
        ),
        (
            GPS_TIME_STAMP,
            Gps::Rationals(&[(13, 1), (5, 1), (21_450, 1000)]),
        ),
        (GPS_STATUS, Gps::Ascii("A")),
        (GPS_MEASURE_MODE, Gps::Ascii("2")),
        (GPS_MAP_DATUM, Gps::Ascii("WGS-84")),
        (GPS_DATE_STAMP, Gps::Ascii("2024:05:01")),
    ];

    /// A phone's GPS IFD: whole seconds, no time, and an altitude
    /// reference without an altitude. The position is the Sydney Opera
    /// House's.
    const PHONE_GPS: [(u16, Gps); 6] = [
        (GPS_VERSION_ID, Gps::Byte(&[2, 2, 0, 0])),
        (GPS_LATITUDE_REF, Gps::Ascii("S")),
        (GPS_LATITUDE, Gps::Rationals(&[(33, 1), (51, 1), (25, 1)])),
        (GPS_LONGITUDE_REF, Gps::Ascii("E")),
        (GPS_LONGITUDE, Gps::Rationals(&[(151, 1), (12, 1), (55, 1)])),
        (GPS_ALTITUDE_REF, Gps::Byte(&[0])),
    ];

    /// The four entries a Sony camera without a fix writes: the version,
    /// the status `V`, the datum and the differential correction.
    const SONY_NO_FIX_GPS: [(u16, Gps); 4] = [
        (GPS_VERSION_ID, Gps::Byte(&[2, 3, 0, 0])),
        (GPS_STATUS, Gps::Ascii("V")),
        (GPS_MAP_DATUM, Gps::Ascii("WGS-84")),
        (GPS_DIFFERENTIAL, Gps::Short(&[0])),
    ];

    /// The GPS IFD of the Coolpix files the NEF notes list: every position
    /// tag, the altitude, the time and the image direction, with blank
    /// references and zero denominators, and a blank datum of 10 bytes.
    const COOLPIX_GPS: [(u16, Gps); 12] = [
        (GPS_VERSION_ID, Gps::Byte(&[2, 3, 0, 0])),
        (GPS_LATITUDE_REF, Gps::Ascii(" ")),
        (GPS_LATITUDE, Gps::Rationals(&[(0, 0); 3])),
        (GPS_LONGITUDE_REF, Gps::Ascii(" ")),
        (GPS_LONGITUDE, Gps::Rationals(&[(0, 0); 3])),
        (GPS_ALTITUDE_REF, Gps::Byte(&[0])),
        (GPS_ALTITUDE, Gps::Rationals(&[(0, 0)])),
        (GPS_TIME_STAMP, Gps::Rationals(&[(0, 0); 3])),
        (GPS_IMG_DIRECTION_REF, Gps::Ascii(" ")),
        (GPS_IMG_DIRECTION, Gps::Rationals(&[(0, 0)])),
        (GPS_MAP_DATUM, Gps::Ascii("         ")),
        (GPS_DATE_STAMP, Gps::Ascii("          ")),
    ];

    /// Both byte orders.
    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// Writes `tags` into the GPS IFD `gps`.
    fn write_gps(builder: &mut TiffBuilder, gps: IfdId, tags: &[(u16, Gps)]) {
        for &(tag, value) in tags {
            match value {
                Gps::Byte(values) => builder.byte(gps, tag, values),
                Gps::Ascii(text) => builder.ascii(gps, tag, text),
                Gps::Rationals(values) => builder.rational(gps, tag, values),
                Gps::Short(values) => builder.short(gps, tag, values),
            };
        }
    }

    /// `tags` with tag `tag` set to `value`, added when it is not there,
    /// or left out when `value` is `None`.
    fn with_gps(tags: &[(u16, Gps)], tag: u16, value: Option<Gps>) -> Vec<(u16, Gps)> {
        let mut tags: Vec<_> = tags.iter().copied().filter(|&(t, _)| t != tag).collect();
        tags.extend(value.map(|value| (tag, value)));
        tags
    }

    /// The metadata of a file in byte order `order` whose IFD0 holds a
    /// Make and links to a GPS IFD holding `tags`.
    fn gps_file(order: Order, tags: &[(u16, Gps)]) -> Metadata {
        let mut builder = TiffBuilder::new(order);
        let (ifd0, gps) = (builder.ifd(), builder.ifd());
        builder
            .ascii(ifd0, MAKE, "NIKON CORPORATION")
            .gps(ifd0, gps);
        write_gps(&mut builder, gps, tags);
        read_bytes(builder.build().bytes)
    }

    /// The location `tags` give, in both byte orders, which must agree.
    fn location_of(tags: &[(u16, Gps)]) -> Option<Location> {
        let [little, big] = ORDERS.map(|order| gps_file(order, tags).location);
        assert_eq!(little, big, "{tags:?}");
        little
    }

    /// The location [`NIKON_GPS`] records.
    fn nikon_location() -> Location {
        Location {
            altitude: Altitude::above(fraction(35_050, 1000)).ok(),
            time: DateTime::new(2024, 5, 1, 8, 20, 30)
                .and_then(|time| time.with_subseconds(450_000_000, 2))
                .and_then(|time| time.with_offset_minutes(0))
                .ok(),
            ..Location::new(
                Latitude::north(fraction(48, 1), fraction(514_933, 10_000), fraction(0, 1))
                    .unwrap(),
                Longitude::east(fraction(2, 1), fraction(176_700, 10_000), fraction(0, 1)).unwrap(),
            )
        }
    }

    #[test]
    fn a_location_is_read_from_each_makers_gps_ifd() {
        assert_eq!(location_of(&NIKON_GPS), Some(nikon_location()));
        let sony = Location {
            time: DateTime::new(2024, 5, 1, 13, 5, 21)
                .and_then(|time| time.with_subseconds(450_000_000, 3))
                .and_then(|time| time.with_offset_minutes(0))
                .ok(),
            map_datum: Some("WGS-84".to_owned()),
            ..Location::new(
                Latitude::north(fraction(40, 1), fraction(41, 1), fraction(21_400, 1000)).unwrap(),
                Longitude::west(fraction(74, 1), fraction(2, 1), fraction(40_200, 1000)).unwrap(),
            )
        };
        assert_eq!(location_of(&SONY_GPS), Some(sony));
        let phone = Location::new(
            Latitude::south(fraction(33, 1), fraction(51, 1), fraction(25, 1)).unwrap(),
            Longitude::east(fraction(151, 1), fraction(12, 1), fraction(55, 1)).unwrap(),
        );
        assert_eq!(location_of(&PHONE_GPS), Some(phone));
        // The Nikon time keeps its two digits and the Sony time its three.
        let nikon_time = location_of(&NIKON_GPS).and_then(|location| location.time());
        assert_eq!(
            nikon_time.map(|time| time.to_string()).as_deref(),
            Some("2024-05-01T08:20:30.45+00:00")
        );
    }

    #[test]
    fn a_gps_ifd_without_a_sound_position_gives_no_location() {
        // No GPS IFD at all.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            builder.ascii(ifd0, MAKE, "NIKON CORPORATION");
            assert_eq!(read_bytes(builder.build().bytes).location, None);
        }
        let cases: [(&str, Vec<(u16, Gps)>); 18] = [
            ("the version alone", NIKON_GPS[..1].to_vec()),
            ("a Sony camera without a fix", SONY_NO_FIX_GPS.to_vec()),
            ("the Coolpix blanks", COOLPIX_GPS.to_vec()),
            (
                "no latitude reference",
                with_gps(&NIKON_GPS, GPS_LATITUDE_REF, None),
            ),
            (
                "no longitude reference",
                with_gps(&NIKON_GPS, GPS_LONGITUDE_REF, None),
            ),
            ("no latitude", with_gps(&NIKON_GPS, GPS_LATITUDE, None)),
            ("no longitude", with_gps(&NIKON_GPS, GPS_LONGITUDE, None)),
            (
                "a latitude reference X",
                with_gps(&NIKON_GPS, GPS_LATITUDE_REF, Some(Gps::Ascii("X"))),
            ),
            (
                "a longitude reference N",
                with_gps(&NIKON_GPS, GPS_LONGITUDE_REF, Some(Gps::Ascii("N"))),
            ),
            (
                "a lower-case reference",
                with_gps(&NIKON_GPS, GPS_LATITUDE_REF, Some(Gps::Ascii("n"))),
            ),
            (
                "a reference of two letters",
                with_gps(&NIKON_GPS, GPS_LATITUDE_REF, Some(Gps::Ascii("NS"))),
            ),
            (
                "a reference of type BYTE",
                with_gps(&NIKON_GPS, GPS_LATITUDE_REF, Some(Gps::Byte(b"N\0"))),
            ),
            (
                "two RATIONALs for a coordinate",
                with_gps(
                    &NIKON_GPS,
                    GPS_LATITUDE,
                    Some(Gps::Rationals(&[(48, 1), (514_933, 10_000)])),
                ),
            ),
            (
                "four RATIONALs for a coordinate",
                with_gps(
                    &NIKON_GPS,
                    GPS_LONGITUDE,
                    Some(Gps::Rationals(&[(2, 1), (17, 1), (40, 1), (0, 1)])),
                ),
            ),
            (
                "SHORTs in place of RATIONALs",
                with_gps(&NIKON_GPS, GPS_LATITUDE, Some(Gps::Short(&[48, 51, 29]))),
            ),
            (
                "a zero denominator",
                with_gps(
                    &NIKON_GPS,
                    GPS_LONGITUDE,
                    Some(Gps::Rationals(&[(2, 1), (176_700, 0), (0, 1)])),
                ),
            ),
            (
                "minutes of 60",
                with_gps(
                    &NIKON_GPS,
                    GPS_LATITUDE,
                    Some(Gps::Rationals(&[(48, 1), (600_000, 10_000), (0, 1)])),
                ),
            ),
            (
                "a latitude past 90 degrees",
                with_gps(
                    &NIKON_GPS,
                    GPS_LATITUDE,
                    Some(Gps::Rationals(&[(90, 1), (1, 10_000), (0, 1)])),
                ),
            ),
        ];
        for (label, tags) in cases {
            assert_eq!(location_of(&tags), None, "{label}");
        }
    }

    #[test]
    fn the_altitude_needs_both_its_tags() {
        let altitude = |reference: Option<Gps>, altitude: Option<Gps>| {
            let tags = with_gps(&NIKON_GPS, GPS_ALTITUDE_REF, reference);
            let tags = with_gps(&tags, GPS_ALTITUDE, altitude);
            location_of(&tags)
                .expect("the position is sound")
                .altitude()
        };
        let meters = Some(Gps::Rationals(&[(35_050, 1000)]));
        let above = Altitude::above(fraction(35_050, 1000)).ok();
        let below = Altitude::below(fraction(35_050, 1000)).ok();
        assert_eq!(altitude(Some(Gps::Byte(&[0])), meters), above);
        assert_eq!(altitude(Some(Gps::Byte(&[1])), meters), below);
        assert_eq!(altitude(Some(Gps::Byte(&[2])), meters), above);
        assert_eq!(altitude(Some(Gps::Byte(&[3])), meters), below);
        // Zero meters is a real altitude.
        assert_eq!(
            altitude(Some(Gps::Byte(&[0])), Some(Gps::Rationals(&[(0, 1)]))),
            Altitude::above(fraction(0, 1)).ok()
        );
        for (label, reference, meters) in [
            ("a reference of 4", Some(Gps::Byte(&[4])), meters),
            ("a reference of 255", Some(Gps::Byte(&[255])), meters),
            ("no reference", None, meters),
            ("no altitude", Some(Gps::Byte(&[1])), None),
            ("a reference of two BYTEs", Some(Gps::Byte(&[0, 0])), meters),
            ("a reference of type SHORT", Some(Gps::Short(&[0])), meters),
            (
                "a zero denominator",
                Some(Gps::Byte(&[0])),
                Some(Gps::Rationals(&[(35, 0)])),
            ),
            (
                "two RATIONALs",
                Some(Gps::Byte(&[0])),
                Some(Gps::Rationals(&[(35, 1), (1, 1)])),
            ),
        ] {
            assert_eq!(altitude(reference, meters), None, "{label}");
        }
    }

    #[test]
    fn the_gps_time_needs_a_date_and_time_that_exist() {
        let time = |date: Option<Gps>, stamp: Option<Gps>| {
            let tags = with_gps(&NIKON_GPS, GPS_DATE_STAMP, date);
            let tags = with_gps(&tags, GPS_TIME_STAMP, stamp);
            location_of(&tags)
                .expect("the position is sound")
                .time()
                .map(|time| time.to_string())
        };
        let date = Some(Gps::Ascii("2024:05:01"));
        let stamp = |seconds: (u32, u32)| {
            // Leaked so that the slice lives as long as the constants'.
            let values: &'static [(u32, u32)] = Box::leak(Box::new([(8, 1), (20, 1), seconds]));
            Some(Gps::Rationals(values))
        };
        for (seconds, expected) in [
            ((30, 1), "2024-05-01T08:20:30+00:00"),
            ((305, 10), "2024-05-01T08:20:30.5+00:00"),
            ((3_000, 100), "2024-05-01T08:20:30.00+00:00"),
            ((30_450, 1000), "2024-05-01T08:20:30.450+00:00"),
            ((0, 1), "2024-05-01T08:20:00+00:00"),
            (
                (599_999_999, 10_000_000),
                "2024-05-01T08:20:59.9999999+00:00",
            ),
            (
                (1_000_000_000, 1_000_000_000),
                "2024-05-01T08:20:01.000000000+00:00",
            ),
        ] {
            assert_eq!(
                time(date, stamp(seconds)).as_deref(),
                Some(expected),
                "{seconds:?}"
            );
        }
        // The date is cleaned as every text: padding spaces are dropped.
        assert_eq!(
            time(Some(Gps::Ascii(" 2024:05:01 ")), stamp((30, 1))).as_deref(),
            Some("2024-05-01T08:20:30+00:00")
        );
        let whole = |hours: (u32, u32), minutes: (u32, u32)| {
            let values: &'static [(u32, u32)] = Box::leak(Box::new([hours, minutes, (30, 1)]));
            Some(Gps::Rationals(values))
        };
        for (label, date, stamp) in [
            ("seconds over 3", date, stamp((91, 3))),
            ("seconds over 0", date, stamp((30, 0))),
            ("seconds over 20", date, stamp((600, 20))),
            ("a second of 60", date, stamp((60, 1))),
            ("a second of 60.000", date, stamp((60_000, 1000))),
            ("an hour of 24", date, whole((24, 1), (0, 1))),
            ("a minute of 60", date, whole((8, 1), (60, 1))),
            ("an hour over 10", date, whole((80, 10), (20, 1))),
            ("minutes over 100", date, whole((8, 1), (2_000, 100))),
            ("an hour past a byte", date, whole((264, 1), (20, 1))),
            (
                "a blank date",
                Some(Gps::Ascii("          ")),
                stamp((30, 1)),
            ),
            ("a date of NULs", Some(Gps::Ascii("")), stamp((30, 1))),
            (
                "30 February",
                Some(Gps::Ascii("2024:02:30")),
                stamp((30, 1)),
            ),
            (
                "a date with dashes",
                Some(Gps::Ascii("2024-05-01")),
                stamp((30, 1)),
            ),
            (
                "a date and a time",
                Some(Gps::Ascii("2024:05:01 08:20:30")),
                stamp((30, 1)),
            ),
            ("no date", None, stamp((30, 1))),
            ("no time stamp", date, None),
            (
                "two RATIONALs",
                date,
                Some(Gps::Rationals(&[(8, 1), (20, 1)])),
            ),
        ] {
            assert_eq!(time(date, stamp), None, "{label}");
        }
    }

    #[test]
    fn the_image_direction_needs_both_its_tags() {
        let direction = |reference: Option<Gps>, degrees: Option<Gps>| {
            let tags = with_gps(&NIKON_GPS, GPS_IMG_DIRECTION_REF, reference);
            let tags = with_gps(&tags, GPS_IMG_DIRECTION, degrees);
            location_of(&tags)
                .expect("the position is sound")
                .image_direction()
        };
        let degrees = Some(Gps::Rationals(&[(27_550, 100)]));
        assert_eq!(
            direction(Some(Gps::Ascii("M")), degrees),
            Direction::new(fraction(27_550, 100), North::Magnetic).ok()
        );
        assert_eq!(
            direction(
                Some(Gps::Ascii("T")),
                Some(Gps::Rationals(&[(35_999, 100)]))
            ),
            Direction::new(fraction(35_999, 100), North::True).ok()
        );
        assert_eq!(
            direction(Some(Gps::Ascii("T")), Some(Gps::Rationals(&[(0, 1)]))),
            Direction::new(fraction(0, 1), North::True).ok()
        );
        for (label, reference, degrees) in [
            ("no reference", None, degrees),
            ("a blank reference", Some(Gps::Ascii(" ")), degrees),
            ("a reference X", Some(Gps::Ascii("X")), degrees),
            ("no direction", Some(Gps::Ascii("T")), None),
            (
                "360 degrees",
                Some(Gps::Ascii("T")),
                Some(Gps::Rationals(&[(360, 1)])),
            ),
            (
                "a zero denominator",
                Some(Gps::Ascii("M")),
                Some(Gps::Rationals(&[(90, 0)])),
            ),
            (
                "two RATIONALs",
                Some(Gps::Ascii("M")),
                Some(Gps::Rationals(&[(90, 1), (0, 1)])),
            ),
        ] {
            assert_eq!(direction(reference, degrees), None, "{label}");
        }
    }

    #[test]
    fn the_map_datum_is_cleaned_text_and_none_when_blank() {
        let datum = |value: Option<Gps>| {
            location_of(&with_gps(&NIKON_GPS, GPS_MAP_DATUM, value))
                .expect("the position is sound")
                .map_datum()
                .map(str::to_owned)
        };
        assert_eq!(datum(Some(Gps::Ascii("WGS-84"))).as_deref(), Some("WGS-84"));
        assert_eq!(
            datum(Some(Gps::Ascii(" TOKYO  "))).as_deref(),
            Some("TOKYO")
        );
        // Nikon's 17 bytes of spaces, the Coolpix files' 10, and no datum.
        assert_eq!(datum(Some(Gps::Ascii("                "))), None);
        assert_eq!(datum(Some(Gps::Ascii("         "))), None);
        assert_eq!(datum(None), None);
        // Exif gives the datum as ASCII alone.
        assert_eq!(datum(Some(Gps::Byte(b"WGS-84\0"))), None);
        let utf8 = ORDERS.map(|order| {
            let mut builder = TiffBuilder::new(order);
            let (ifd0, gps) = (builder.ifd(), builder.ifd());
            builder.gps(ifd0, gps);
            write_gps(
                &mut builder,
                gps,
                &with_gps(&NIKON_GPS, GPS_MAP_DATUM, None),
            );
            builder.utf8(gps, GPS_MAP_DATUM, "WGS-84");
            read_bytes(builder.build().bytes)
                .location
                .unwrap()
                .map_datum
        });
        assert_eq!(utf8, [None, None]);
        let long = ORDERS.map(|order| {
            let mut builder = TiffBuilder::new(order);
            let (ifd0, gps) = (builder.ifd(), builder.ifd());
            builder.gps(ifd0, gps);
            write_gps(
                &mut builder,
                gps,
                &with_gps(&NIKON_GPS, GPS_MAP_DATUM, None),
            );
            builder.ascii(gps, GPS_MAP_DATUM, &"W".repeat(MAX_TEXT_LEN + 1));
            read_bytes(builder.build().bytes)
                .location
                .unwrap()
                .map_datum
        });
        assert_eq!(long, [None, None]);
    }

    #[test]
    fn the_gps_ifd_is_found_wherever_the_walk_placed_it_and_nothing_else_is_read_from_it() {
        // A GPS IFD that is also IFD1, as an IFD shared with the chain:
        // its position is still read.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let (ifd0, gps) = (builder.ifd(), builder.ifd());
            builder
                .ascii(ifd0, MAKE, "NIKON CORPORATION")
                .next(ifd0, Some(gps))
                .gps(ifd0, gps);
            write_gps(&mut builder, gps, &NIKON_GPS);
            let metadata = read_bytes(builder.build().bytes);
            assert_eq!(metadata.location, Some(nikon_location()), "{order:?}");
        }
        // The GPS tags in IFD0 or in the Exif IFD are not a location.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let (ifd0, exif) = (builder.ifd(), builder.ifd());
            builder.exif(ifd0, exif);
            write_gps(
                &mut builder,
                exif,
                &with_gps(&NIKON_GPS, GPS_VERSION_ID, None),
            );
            let metadata = read_bytes(builder.build().bytes);
            assert_eq!(metadata.location, None, "{order:?}");
        }
    }

    #[test]
    fn a_damaged_gps_value_costs_only_itself() {
        // Every part of a location in the Z 9 file, each part damaged in
        // turn: a text longer than one value may take, which the reader
        // reads and refuses, or a fraction of another type or count. Only
        // that part is lost; the position, the other parts and every value
        // outside the GPS IFD stay.
        let tags = with_gps(&NIKON_GPS, GPS_MAP_DATUM, Some(Gps::Ascii("WGS-84")));
        let tags = with_gps(&tags, GPS_IMG_DIRECTION_REF, Some(Gps::Ascii("T")));
        let tags = with_gps(&tags, GPS_IMG_DIRECTION, Some(Gps::Rationals(&[(275, 1)])));
        let whole = Location {
            image_direction: Direction::new(fraction(275, 1), North::True).ok(),
            map_datum: Some("WGS-84".to_owned()),
            ..nikon_location()
        };
        type Damage = fn(&mut TiffBuilder, IfdId, u16);
        type Lose = fn(&mut Location);
        let too_long: Damage = |builder, gps, tag| too_long_of(builder, gps, tag, ASCII, 1);
        let short: Damage = |builder, gps, tag| {
            builder.short(gps, tag, &[35]);
        };
        let two: Damage = |builder, gps, tag| {
            builder.rational(gps, tag, &[(8, 1), (20, 1)]);
        };
        let cases: [(u16, Damage, Lose); 7] = [
            (GPS_MAP_DATUM, too_long, |l| l.map_datum = None),
            (GPS_DATE_STAMP, too_long, |l| l.time = None),
            (GPS_IMG_DIRECTION_REF, too_long, |l| {
                l.image_direction = None
            }),
            (GPS_ALTITUDE, short, |l| l.altitude = None),
            (GPS_ALTITUDE_REF, short, |l| l.altitude = None),
            (GPS_TIME_STAMP, two, |l| l.time = None),
            (GPS_IMG_DIRECTION, two, |l| l.image_direction = None),
        ];
        let (sound, z9) = (
            read_bytes(z9_bytes_except(0, |builder, ifd0, _| {
                let gps = builder.ifd();
                builder.gps(ifd0, gps);
                write_gps(builder, gps, &tags);
            })),
            z9_file(),
        );
        assert_eq!(sound.location, Some(whole.clone()));
        for (damaged, damage, lose) in cases {
            let metadata = read_bytes(z9_bytes_except(0, |builder, ifd0, _| {
                let gps = builder.ifd();
                builder.gps(ifd0, gps);
                write_gps(builder, gps, &with_gps(&tags, damaged, None));
                damage(builder, gps, damaged);
            }));
            let mut expected = whole.clone();
            lose(&mut expected);
            assert_eq!(metadata.location, Some(expected), "{damaged:#06X}");
            assert_eq!(
                Metadata {
                    location: None,
                    ..metadata
                },
                z9,
                "{damaged:#06X}"
            );
        }
        // A damaged reference costs the location, and nothing else.
        for damaged in [GPS_LATITUDE_REF, GPS_LONGITUDE_REF] {
            let metadata = read_bytes(z9_bytes_except(0, |builder, ifd0, _| {
                let gps = builder.ifd();
                builder.gps(ifd0, gps);
                write_gps(builder, gps, &with_gps(&tags, damaged, None));
                too_long(builder, gps, damaged);
            }));
            assert_eq!(metadata, z9, "{damaged:#06X}");
        }
    }

    #[test]
    fn a_failure_of_the_reader_reading_a_gps_value_is_the_readers_error() {
        let far = u32::try_from(2 * FAR).unwrap();
        // Each value stored far away, longer than four bytes and so not
        // held in its entry, in the little-endian order of the Z 9 file.
        let rationals = |values: &[(u32, u32)]| -> Vec<u8> {
            values
                .iter()
                .flat_map(|&(n, d)| n.to_le_bytes().into_iter().chain(d.to_le_bytes()))
                .collect()
        };
        let values: [(u16, u16, Vec<u8>); 7] = [
            (
                GPS_LATITUDE,
                RATIONAL,
                rationals(&[(48, 1), (51, 1), (0, 1)]),
            ),
            (
                GPS_LONGITUDE,
                RATIONAL,
                rationals(&[(2, 1), (17, 1), (0, 1)]),
            ),
            (GPS_ALTITUDE, RATIONAL, rationals(&[(35, 1)])),
            (
                GPS_TIME_STAMP,
                RATIONAL,
                rationals(&[(8, 1), (20, 1), (30, 1)]),
            ),
            (GPS_IMG_DIRECTION, RATIONAL, rationals(&[(275, 1)])),
            (GPS_MAP_DATUM, ASCII, b"WGS-84\0".to_vec()),
            (GPS_DATE_STAMP, ASCII, b"2024:05:01\0".to_vec()),
        ];
        let tags = with_gps(&NIKON_GPS, GPS_IMG_DIRECTION_REF, Some(Gps::Ascii("T")));
        let tags = with_gps(&tags, GPS_IMG_DIRECTION, Some(Gps::Rationals(&[(275, 1)])));
        for (tag, typ, value) in values {
            let size = if typ == RATIONAL { 8 } else { 1 };
            let count = u32::try_from(value.len()).unwrap() / size;
            let bytes = z9_bytes_except(0, |builder, ifd0, _| {
                let gps = builder.ifd();
                builder.gps(ifd0, gps).blob_at(2 * FAR, &value);
                write_gps(builder, gps, &with_gps(&tags, tag, None));
                builder.raw(gps, tag, typ, count, RawValue::Offset(far));
            });
            // The file is sound, for a reader that does not fail.
            let sound = read_bytes(bytes.clone());
            assert!(sound.location.is_some(), "{tag:#06X}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{tag:#06X}: {err:?}");
        }
    }
}
