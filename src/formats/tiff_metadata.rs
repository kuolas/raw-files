// SPDX-License-Identifier: MIT OR Apache-2.0

//! The metadata reader shared by the TIFF-based formats: it reads what the
//! camera recorded about a picture from the standard TIFF and Exif tags.
//!
//! Cameras of every maker record the same facts in the same standard tags:
//! the camera's make and model, the software that wrote the file and the
//! orientation in IFD0, the capture time, the body's serial number, the
//! exposure, the lens and some of the camera's settings in the Exif IFD,
//! with the Interoperability IFD for the colour space, and the sizes in the
//! IFD of the raw data. So [`read`] reads them once, over the tree, for
//! every format, and a format adds only what its own structures record and
//! the standard tags lack (ADR011, which chose the standard tags first). The
//! JPEG reader shares it too, over the tree of a JPEG file's Exif segment,
//! and then decides the sizes itself: the picture's own size in place of
//! the ones read here, and no raw size (ADR014).
//!
//! The rules:
//!
//! - A damaged value costs only itself. A tag that is missing, of a type
//!   other than the standard's, of a count other than one where the rules
//!   below ask for one SHORT, or whose value cannot be read, such as one
//!   longer than one tag may take, gives `None` for its own value; every
//!   other value is still read, and the file still opens. Only a failure of
//!   the reader is an error: it says nothing about the file.
//! - The make, the model and the serial number, of the camera and of the
//!   lens, and the software are text, read by the one rule [`text`] gives:
//!   cut at the first NUL, `None` when longer than 255 bytes, trimmed of
//!   spaces at both ends, `None` when nothing is left, and with bytes that
//!   are not UTF-8 replaced by U+FFFD.
//! - The software is IFD0's Software, as written and cleaned by that rule:
//!   the camera's firmware version, such as `Ver.1.10` from the D300's
//!   `Ver.1.10 `, or the name of the program that last wrote the file.
//!   Exif 3.0's CameraFirmware, which no checked file writes, is not read.
//! - The capture time is the Exif DateTimeOriginal, with its own fraction
//!   of a second (SubSecTimeOriginal) and offset from UTC
//!   (OffsetTimeOriginal), each of type ASCII; [`DateTime::from_exif`]
//!   checks them. Without a DateTimeOriginal there is no capture time,
//!   whatever fraction or offset the file records. It is never taken from
//!   DateTimeDigitized or from IFD0's DateTime, which editors change.
//! - The ISO is the Exif PhotographicSensitivity, of type SHORT or LONG,
//!   its first value. When that is missing, unreadable, zero, which Exif
//!   writes for an unknown sensitivity, or 65535, which Exif writes for one
//!   of 65535 or more, the ISO is the RecommendedExposureIndex, or else the
//!   ISOSpeed, each its first value and only when above zero. A 65535 that
//!   neither replaces stands, as the least the sensitivity was; a zero
//!   that neither replaces is `None` ([`iso`]).
//! - The exposure time, the f-number and the focal length are the Exif
//!   ExposureTime, FNumber and FocalLength, of type RATIONAL, their first
//!   value as stored; each is `None` when its numerator or its denominator
//!   is zero, since Exif writes a zero for a value the camera did not know
//!   ([`Rational::known`]). The exposure bias is the Exif
//!   ExposureBiasValue, of type SRATIONAL, its first value as stored; it is
//!   `None` only when its denominator is zero, since a zero bias, such as
//!   0/6, is no compensation ([`SignedRational::new`]).
//! - The focal length in 35 mm terms is the Exif FocalLengthIn35mmFilm,
//!   exactly one SHORT, the type and count the standard gives it
//!   ([`one_short`]); it is `None` when zero, which Exif writes for one the
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
//!   each exactly one SHORT, as the standard gives them ([`one_short`]),
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
//!   - the colour space is sRGB for a ColorSpace of 1, and Adobe RGB for
//!     65535, "uncalibrated", when the Interoperability IFD's
//!     InteropIndex, read by the text rule, is `R03`, a DCF option file;
//!     any other ColorSpace, an "uncalibrated" one beside another index or
//!     none, and no ColorSpace give `None` ([`color_space`]).
//!
//!   The other settings are recorded only in a maker's MakerNote, so they
//!   are `None` here: the white balance's colour temperature and
//!   fine-tuning, the shutter, the electronic front curtain setting, the
//!   image area and the raw image size. A format's MakerNote reader fills
//!   them where its MakerNote records them, and puts the MakerNote's white
//!   balance setting, finer than Exif's, in place of the mode read here
//!   when the MakerNote names one, as Nikon's and Sony's readers both do.
//!   So that a MakerNote's colour space never overrules Exif's, [`read`]
//!   also says whether the Exif IFD keeps a ColorSpace entry, of any known
//!   type and any count, whether or not it gives a colour space
//!   ([`Standard`]): only when it keeps none may a MakerNote give the
//!   colour space, as Nikon's raw files, which record none in Exif, need.
//!   An entry the IFD's reader skips as damaged, of an unknown type, of a
//!   length past 32 bits or with its value outside its block, the file or
//!   a JPEG's Exif segment, counts as none, so the MakerNote may stand in,
//!   as for any damaged value (ADR011).
//! - A file without an Exif IFD has no capture time, no serial number, no
//!   exposure, no lens, and none of the camera settings.
//! - The orientation is IFD0's Orientation, read as the preview finder reads
//!   it ([`tiff_previews::orientation`]), so that the metadata and the
//!   previews never disagree.
//! - The raw size is the ImageWidth and ImageLength, of type SHORT or LONG,
//!   of the IFD of the tree, the Exif and Interoperability IFDs left out,
//!   that the preview finder takes for raw data
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
//! checked accessors, which check its offset and length against the file
//! and cap its size (ADR008).

use std::io::{Read, Seek};

use crate::camera_settings::{
    CameraSettings, ColorSpace, ExposureProgram, Flash, FlashMode, MeteringMode, WhiteBalance,
    WhiteBalanceMode,
};
use crate::container::tiff::tags::{
    BODY_SERIAL_NUMBER, COLOR_SPACE, DATE_TIME_ORIGINAL, DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE,
    EXPOSURE_BIAS_VALUE, EXPOSURE_PROGRAM, EXPOSURE_TIME, F_NUMBER, FLASH, FOCAL_LENGTH,
    FOCAL_LENGTH_IN_35MM_FILM, IMAGE_LENGTH, IMAGE_WIDTH, INTEROP_INDEX, ISO_SPEED, LENS_MAKE,
    LENS_MODEL, LENS_SERIAL_NUMBER, LENS_SPECIFICATION, LIGHT_SOURCE, MAKE, METERING_MODE, MODEL,
    OFFSET_TIME_ORIGINAL, PHOTOGRAPHIC_SENSITIVITY, RECOMMENDED_EXPOSURE_INDEX, SOFTWARE,
    SUB_SEC_TIME_ORIGINAL, WHITE_BALANCE,
};
use crate::container::tiff::types::{LONG, SHORT};
use crate::container::tiff::{Ifd, Tree, skip_damage};
use crate::datetime::DateTime;
use crate::error::Result;
use crate::formats::tiff_previews;
use crate::geometry::Size;
use crate::io::Source;
use crate::metadata::{Exposure, Lens, Metadata};
use crate::rational::{Rational, SignedRational};

/// The longest text, in bytes as stored, that is taken for a value rather
/// than for damage.
///
/// The longest text value measured on the public sample files is 65 bytes,
/// a lens model, so a longer one than this is not a name or a serial
/// number a camera wrote.
const MAX_TEXT_LEN: usize = 255;

/// The PhotographicSensitivity that Exif writes for a sensitivity of 65535
/// or more: a least value, which a RecommendedExposureIndex or an ISOSpeed
/// replaces with the real one when the file records it.
const ISO_65535_OR_MORE: u32 = 65535;

/// What [`read`] reads from the standard tags: the metadata, and, for a
/// format whose own structures record the same values, whether the
/// standard tags recorded two of them: the image size as a crop of the
/// raw IFD's, and the colour space.
///
/// The standard tags come first (ADR011). A format's own crop, such as
/// Nikon's crop area, may replace the raw size the image size otherwise
/// falls back on, but never a default crop; comparing the image size with
/// the raw size cannot tell the two apart, since a default crop may cover
/// the whole raw. A format's own colour space, such as the one in Nikon's
/// MakerNote, may stand in only where the Exif IFD holds no ColorSpace: a
/// ColorSpace that gives none, such as a program's "uncalibrated", is
/// still Exif's word, and a MakerNote the program left behind may describe
/// a picture that is no longer there. The colour space `None` cannot tell
/// the two apart.
#[derive(Debug, Default)]
pub(crate) struct Standard {
    /// The metadata, every value of it from the standard tags.
    pub(crate) metadata: Metadata,
    /// Whether the image size is the raw IFD's default crop; `false` when
    /// it is the raw size, or there is no image size.
    pub(crate) from_default_crop: bool,
    /// Whether the Exif IFD keeps a ColorSpace entry, of any known type and
    /// any count, whether or not it gives a colour space; `false` when
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
    let (serial_number, capture_time, exposure, camera_settings, lens) = match tree.exif() {
        Some(exif) => (
            text(src, exif, BODY_SERIAL_NUMBER)?,
            capture_time(src, exif)?,
            exposure(src, exif)?,
            camera_settings(src, exif, tree.interop())?,
            lens(src, exif)?,
        ),
        None => (
            None,
            None,
            Exposure::default(),
            CameraSettings::default(),
            Lens::default(),
        ),
    };
    let (raw_size, default_crop) = sizes(src, tree)?;
    let metadata = Metadata {
        make: text(src, ifd0, MAKE)?,
        model: text(src, ifd0, MODEL)?,
        software: text(src, ifd0, SOFTWARE)?,
        serial_number,
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

/// The text of tag `tag` of `ifd`, cleaned as every text value of the
/// metadata is, such as the camera's make and serial number, whether a
/// standard tag or a format's own, such as the serial number in Nikon's
/// MakerNote, records it.
///
/// The value must be of type ASCII. It ends at its first NUL, or with the
/// value when there is none. When what is before the NUL is longer than
/// [`MAX_TEXT_LEN`] bytes, counted as stored, before any trimming or
/// replacement, it is damage and `None`. Spaces at either end are padding
/// and are dropped, as in the DSLR-A100's Make, `SONY` and eleven spaces;
/// spaces inside the text, and any other blank such as a tab, are kept, as
/// the capture time's reading and the formats' Make test keep them. Text
/// that is empty once trimmed, as a value of NULs or of spaces only, is
/// `None`. Bytes that are not UTF-8 become U+FFFD.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason, such as one longer than one tag may take, is `None`.
pub(crate) fn text<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    tag: u16,
) -> Result<Option<String>> {
    let Some(bytes) = skip_damage(ifd.ascii_bytes(src, tag))?.flatten() else {
        return Ok(None);
    };
    if bytes.len() > MAX_TEXT_LEN {
        return Ok(None);
    }
    let start = bytes.iter().position(|&b| b != b' ').unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| b != b' ')
        .map_or(start, |i| i + 1);
    let text = bytes.get(start..end).unwrap_or_default();
    Ok((!text.is_empty()).then(|| String::from_utf8_lossy(text).into_owned()))
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
    let Some(date) = skip_damage(exif.ascii_bytes(src, DATE_TIME_ORIGINAL))?.flatten() else {
        return Ok(None);
    };
    let subsec = skip_damage(exif.ascii_bytes(src, SUB_SEC_TIME_ORIGINAL))?.flatten();
    let offset = skip_damage(exif.ascii_bytes(src, OFFSET_TIME_ORIGINAL))?.flatten();
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
        exposure_time: rational(src, exif, EXPOSURE_TIME)?,
        f_number: rational(src, exif, F_NUMBER)?,
        focal_length: rational(src, exif, FOCAL_LENGTH)?,
        focal_length_in_35mm_film: focal_length_in_35mm_film(src, exif)?,
        exposure_bias: signed_rational(src, exif, EXPOSURE_BIAS_VALUE)?,
    })
}

/// The focal length in 35 mm terms, in whole millimetres, from the Exif
/// IFD `exif`: its FocalLengthIn35mmFilm, which must be exactly one SHORT,
/// the type and count the standard gives it, as [`one_short`] reads it.
/// `None` when the value is zero, which Exif writes for a focal length the
/// camera did not know, as the ILCE-7S does with a manual lens (public
/// sample 1582).
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn focal_length_in_35mm_film<R: Read + Seek>(
    src: &mut Source<R>,
    exif: &Ifd,
) -> Result<Option<u32>> {
    Ok(one_short(src, exif, FOCAL_LENGTH_IN_35MM_FILM)?
        .filter(|&millimetres| millimetres > 0)
        .map(u32::from))
}

/// The value of tag `tag` of `ifd`, when the tag is of type SHORT and
/// holds exactly one value, as [`one_of_type`] reads it: the type and
/// count the standard gives each Exif tag read this way, a single number
/// or code such as the focal length in 35 mm terms or the metering mode,
/// and that a MakerNote's reader holds its own such tags to, such as
/// Nikon's shutter mode.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
pub(crate) fn one_short<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    tag: u16,
) -> Result<Option<u16>> {
    let value = one_of_type(src, ifd, tag, SHORT)?;
    // A SHORT, widened to 32 bits, always fits back.
    Ok(value.and_then(|value| u16::try_from(value).ok()))
}

/// The value of tag `tag` of `ifd`, when the tag is of type LONG and holds
/// exactly one value, as [`one_of_type`] reads it: the type and count a
/// MakerNote's reader holds its tags of one number to where the maker
/// writes them as LONGs, such as each of Sony's settings tags of one
/// number.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
pub(crate) fn one_long<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    tag: u16,
) -> Result<Option<u32>> {
    one_of_type(src, ifd, tag, LONG)
}

/// The value of tag `tag` of `ifd`, widened to 32 bits, when the tag is of
/// type `typ`, SHORT or LONG, and holds exactly one value: the one rule
/// behind [`one_short`] and [`one_long`].
///
/// `None` when the tag is missing, of another type or of another count.
/// The type is checked first ([`Ifd::entry_of_type`]), since [`Ifd::u32`]
/// would take a SHORT, a LONG and an IFD alike; a tag of one value that
/// holds several, or none, records nothing the reader can trust, so even
/// its first value is not taken (ADR011).
///
/// One SHORT or one LONG is held in its entry, so reading it reads
/// nothing more from the file.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn one_of_type<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    tag: u16,
    typ: u16,
) -> Result<Option<u32>> {
    if ifd
        .entry_of_type(tag, &[typ])
        .is_none_or(|entry| entry.count() != 1)
    {
        return Ok(None);
    }
    Ok(skip_damage(ifd.u32(src, tag))?.flatten())
}

/// The ISO the picture was taken at, from the Exif IFD `exif`.
///
/// It is the first value of the PhotographicSensitivity, of type SHORT or
/// LONG, as [`Ifd::u32`] reads it (which takes the type IFD too, a LONG
/// under another name). Exif gives two of its values a meaning of their
/// own: zero, a sensitivity the camera did not know, and 65535
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
    let sensitivity = skip_damage(exif.u32(src, PHOTOGRAPHIC_SENSITIVITY))?.flatten();
    if let Some(iso) = sensitivity.filter(|&iso| iso != 0 && iso != ISO_65535_OR_MORE) {
        return Ok(Some(iso));
    }
    for tag in [RECOMMENDED_EXPOSURE_INDEX, ISO_SPEED] {
        if let Some(iso) = skip_damage(exif.u32(src, tag))?
            .flatten()
            .filter(|&iso| iso > 0)
        {
            return Ok(Some(iso));
        }
    }
    Ok(sensitivity.filter(|&iso| iso == ISO_65535_OR_MORE))
}

/// The first fraction of tag `tag` of `ifd`, which must be of type
/// RATIONAL, as stored; `None` when its numerator or its denominator is
/// zero ([`Rational::known`]).
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn rational<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd, tag: u16) -> Result<Option<Rational>> {
    let first = skip_damage(ifd.rationals(src, tag))?
        .flatten()
        .and_then(|values| values.first().copied());
    Ok(first.and_then(|(numerator, denominator)| Rational::known(numerator, denominator)))
}

/// The first fraction of tag `tag` of `ifd`, which must be of type
/// SRATIONAL, as stored; `None` when its denominator is zero
/// ([`SignedRational::new`]).
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn signed_rational<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    tag: u16,
) -> Result<Option<SignedRational>> {
    let first = skip_damage(ifd.srationals(src, tag))?
        .flatten()
        .and_then(|values| values.first().copied());
    Ok(first.and_then(|(numerator, denominator)| SignedRational::new(numerator, denominator)))
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
    let make = text(src, exif, LENS_MAKE)?;
    let model = text(src, exif, LENS_MODEL)?.filter(|model| !model.chars().all(|c| c == '-'));
    let serial_number = text(src, exif, LENS_SERIAL_NUMBER)?;
    let mut lens = Lens {
        make,
        model,
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
/// are `None`. Each value is `None` when its numerator or its denominator
/// is zero ([`Rational::known`]): Exif writes 0/0 for a value the camera
/// did not know, such as the apertures of a lens that does not report
/// them.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason gives four `None`s.
fn lens_specification<R: Read + Seek>(
    src: &mut Source<R>,
    exif: &Ifd,
) -> Result<[Option<Rational>; 4]> {
    let values = skip_damage(exif.rationals(src, LENS_SPECIFICATION))?.flatten();
    let Some(&[first, second, third, fourth]) = values.as_deref() else {
        return Ok([None; 4]);
    };
    Ok([first, second, third, fourth]
        .map(|(numerator, denominator)| Rational::known(numerator, denominator)))
}

/// The ColorSpace Exif writes for sRGB.
const SRGB: u16 = 1;

/// The ColorSpace Exif writes for "uncalibrated": any colour space but
/// sRGB, which the Interoperability IFD's InteropIndex may then name.
const UNCALIBRATED: u16 = 0xFFFF;

/// The InteropIndex of a DCF option file, which names Adobe RGB beside a
/// ColorSpace of [`UNCALIBRATED`].
///
/// Exif gives the index only the meaning "DCF option file"; that such a
/// file is in Adobe RGB is from ExifTool's published documentation of the
/// tag, read as documentation, and the camera files the JPEG format notes
/// checked agree (section 4 of those notes, and ADR011).
const DCF_OPTION_FILE: &str = "R03";

/// How the camera was set for the picture, as the Exif IFD `exif` records
/// it, with the Interoperability IFD `interop`, when the tree has one, for
/// the colour space: the exposure program, the metering mode, the flash,
/// the white balance mode and the colour space, as the
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
            mode: white_balance_mode(src, exif)?,
            // Only a maker's MakerNote records the colour temperature and
            // the fine-tuning: Nikon's and Sony's readers fill them.
            color_temperature: None,
            fine_tune: None,
        },
        exposure_program: one_short(src, exif, EXPOSURE_PROGRAM)?.and_then(exposure_program),
        metering_mode: one_short(src, exif, METERING_MODE)?.and_then(metering_mode),
        flash: one_short(src, exif, FLASH)?.and_then(flash),
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
fn exposure_program(code: u16) -> Option<ExposureProgram> {
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
/// centre-weighted average, 3 spot, 4 multi-spot, 5 multi-segment (Exif's
/// "pattern"), 6 partial and 255 other. `None` for 0, "unknown", and for
/// every code Exif does not define.
fn metering_mode(code: u16) -> Option<MeteringMode> {
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
/// camera without one: the value contradicts itself.
fn flash(bits: u16) -> Option<Flash> {
    const FIRED: u16 = 1;
    const NO_FLASH_FUNCTION: u16 = 1 << 5;
    const RESERVED: u16 = 0xFF80;
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
/// one SHORT ([`one_short`]).
///
/// `None` when the WhiteBalance is missing, cannot be read or holds
/// another code, and for a manual white balance whose LightSource is
/// missing, cannot be read or names no mode.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`.
fn white_balance_mode<R: Read + Seek>(
    src: &mut Source<R>,
    exif: &Ifd,
) -> Result<Option<WhiteBalanceMode>> {
    match one_short(src, exif, WHITE_BALANCE)? {
        Some(0) => Ok(Some(WhiteBalanceMode::Auto)),
        Some(1) => Ok(one_short(src, exif, LIGHT_SOURCE)?.and_then(white_balance_of_light)),
        _ => Ok(None),
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
/// lights A to D50, and 25 to 34, light sources and LEDs by colour
/// temperature, which name a light no mode stands for; 255, "other"; and
/// every code Exif does not define.
fn white_balance_of_light(code: u16) -> Option<WhiteBalanceMode> {
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

/// The colour space the Exif IFD `exif` records in its ColorSpace, exactly
/// one SHORT ([`one_short`]): [`SRGB`] is sRGB, and [`UNCALIBRATED`] is
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
    match one_short(src, exif, COLOR_SPACE)? {
        Some(SRGB) => Ok(Some(ColorSpace::Srgb)),
        Some(UNCALIBRATED) => {
            let Some(interop) = interop else {
                return Ok(None);
            };
            let index = text(src, interop, INTEROP_INDEX)?;
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
/// capture metadata, and the Interoperability IFD, which names the rule the
/// file follows: neither ever holds an image. An IFD counts as raw data
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
    // The pixels of a size, which cannot overflow: two `u32`s multiplied
    // fit in a `u64`.
    let pixels = |size: Size| u64::from(size.width) * u64::from(size.height);
    let mut largest: Option<(&Ifd, Size)> = None;
    for (location, ifd) in tree.iter() {
        if location.holds_no_image() {
            continue;
        }
        if skip_damage(tiff_previews::is_raw(src, ifd))? != Some(true) {
            continue;
        }
        let Some(raw) = raw_size(src, ifd)? else {
            continue;
        };
        // Only a larger one replaces it, so the first wins a tie.
        if largest.is_none_or(|(_, best)| pixels(raw) > pixels(best)) {
            largest = Some((ifd, raw));
        }
    }
    let Some((ifd, raw)) = largest else {
        return Ok((None, None));
    };
    Ok((Some(raw), default_crop(src, ifd, raw)?))
}

/// The size of the raw data `ifd` describes: its ImageWidth and
/// ImageLength, each of type SHORT or LONG, as [`Ifd::u32`] reads them.
/// `None` when either is missing, cannot be read or is zero.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn raw_size<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd) -> Result<Option<Size>> {
    let width = skip_damage(ifd.u32(src, IMAGE_WIDTH))?.flatten();
    let height = skip_damage(ifd.u32(src, IMAGE_LENGTH))?.flatten();
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
    let Some([x, y]) = whole_pair(src, ifd, DEFAULT_CROP_ORIGIN)? else {
        return Ok(None);
    };
    let Some(size) = whole_pair(src, ifd, DEFAULT_CROP_SIZE)? else {
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

/// The two whole numbers tag `tag` of `ifd` holds, in the forms the DNG
/// specification allows its crop tags: integers of type SHORT or LONG, as
/// [`Ifd::u32s`] reads them, or fractions of type RATIONAL whose every
/// denominator is 1. `None` when the tag is missing, cannot be read, holds
/// a fraction with another denominator, or holds other than two values.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is missing.
fn whole_pair<R: Read + Seek>(
    src: &mut Source<R>,
    ifd: &Ifd,
    tag: u16,
) -> Result<Option<[u32; 2]>> {
    let values = match skip_damage(ifd.u32s(src, tag))?.flatten() {
        Some(integers) => Some(integers),
        None => skip_damage(ifd.rationals(src, tag))?
            .flatten()
            .and_then(|fractions| {
                fractions
                    .into_iter()
                    .map(|(numerator, denominator)| (denominator == 1).then_some(numerator))
                    .collect()
            }),
    };
    Ok(values.and_then(|values: Vec<u32>| <[u32; 2]>::try_from(values).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::{NEW_SUBFILE_TYPE, ORIENTATION, PHOTOMETRIC_INTERPRETATION};
    use crate::container::tiff::{Block, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing;
    use crate::limits::Limits;
    use crate::orientation::Orientation;
    use crate::testing::{FAR, FailsFar, size};
    use crate::tiff_builder::{
        ASCII, IfdId, LONG, Order, RATIONAL, RawValue, SHORT, SRATIONAL, TiffBuilder,
    };
    use std::io::Cursor;

    /// When the picture was stored or digitised (DateTimeDigitized), an
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
    /// centre-weighted metering, a flash that did not fire and recorded no
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

    /// More bytes than the 1 MiB one tag value may take.
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
            make: Some("NIKON".to_owned()),
            model: Some("NIKKOR Z 14-30mm f/4 S".to_owned()),
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
        make: Option<String>,
        model: Option<String>,
        serial_number: Option<String>,
        /// The shortest and longest focal length, then the f-number at
        /// each.
        range: [Option<Rational>; 4],
    }

    impl LensValues {
        /// No value: the values of a file that describes no lens.
        fn none() -> Self {
            Self {
                make: None,
                model: None,
                serial_number: None,
                range: [None; 4],
            }
        }

        /// The lens these values describe.
        fn lens(self) -> Lens {
            let mut lens = Lens {
                make: self.make,
                model: self.model,
                serial_number: self.serial_number,
                ..Lens::default()
            };
            lens.set_range(self.range);
            lens
        }
    }

    /// What the Z 9 file gives, every value read.
    fn z9_expected() -> Metadata {
        Metadata {
            make: Some("NIKON CORPORATION".to_owned()),
            model: Some("NIKON Z 9".to_owned()),
            software: Some("Ver.01.00".to_owned()),
            serial_number: Some("1234567".to_owned()),
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
        read(&mut src, &tree).unwrap().metadata.make
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
        assert_eq!(lens.make(), Some("NIKON"));
        // Read whole, without the NULs that pad it, nor the spaces that pad
        // the serial number.
        assert_eq!(lens.model(), Some("NIKKOR Z 14-30mm f/4 S"));
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
        assert_eq!(model(255).model, Some("M".repeat(255)));
        // Too long, and costs only the model.
        assert_eq!(
            model(256),
            Metadata {
                model: None,
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
        read(&mut src, &tree).unwrap().metadata.make
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
                make: Some("SONY".to_owned()),
                model: Some("DSC-F828".to_owned()),
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
            // No EXIF orientation: none.
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
        let cases: [(&str, u16, Change, Metadata); 24] = [
            (
                "a Model that is not text",
                MODEL,
                |builder, ifd0, _| {
                    builder.byte(ifd0, MODEL, b"NIKON Z 9\0");
                },
                Metadata {
                    model: None,
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
                    make: None,
                    ..z9_expected()
                },
            ),
            (
                "a serial number of 256 bytes",
                BODY_SERIAL_NUMBER,
                |builder, _, exif| {
                    builder.ascii(exif, BODY_SERIAL_NUMBER, &"1".repeat(256));
                },
                Metadata {
                    serial_number: None,
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
                "an ExposureTime too long to read",
                EXPOSURE_TIME,
                |builder, _, exif| too_long_of(builder, exif, EXPOSURE_TIME, RATIONAL, 8),
                z9_expected_with(Exposure {
                    exposure_time: None,
                    ..z9_exposure()
                }),
            ),
            (
                "an FNumber too long to read",
                F_NUMBER,
                |builder, _, exif| too_long_of(builder, exif, F_NUMBER, RATIONAL, 8),
                z9_expected_with(Exposure {
                    f_number: None,
                    ..z9_exposure()
                }),
            ),
            (
                "a FocalLength too long to read",
                FOCAL_LENGTH,
                |builder, _, exif| too_long_of(builder, exif, FOCAL_LENGTH, RATIONAL, 8),
                z9_expected_with(Exposure {
                    focal_length: None,
                    ..z9_exposure()
                }),
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
                "an ExposureBiasValue too long to read",
                EXPOSURE_BIAS_VALUE,
                |builder, _, exif| too_long_of(builder, exif, EXPOSURE_BIAS_VALUE, SRATIONAL, 8),
                z9_expected_with(Exposure {
                    exposure_bias: None,
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
                z9_expected_with_lens(z9_lens_with(|lens| lens.make = None)),
            ),
            (
                "a LensModel that is not text",
                LENS_MODEL,
                |builder, _, exif| {
                    builder.undefined(exif, LENS_MODEL, b"NIKKOR Z 14-30mm f/4 S\0");
                },
                z9_expected_with_lens(z9_lens_with(|lens| lens.model = None)),
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
                "a LensSpecification too long to read",
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
    fn an_unknown_f_number_is_none() {
        // Sony's ILCE-7S with a manual lens, as on the public sample 1582.
        let metadata = z9_except(F_NUMBER, |builder, _, exif| {
            builder.rational(exif, F_NUMBER, &[(0, 10)]);
        });
        assert_eq!(
            metadata,
            z9_expected_with(Exposure {
                f_number: None,
                ..z9_exposure()
            })
        );
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
            z9_expected_with_lens(z9_lens_with(|lens| lens.make = None))
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
            model: Some("VR 24-120mm f/4G".to_owned()),
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
            z9_expected_with_lens(z9_lens_with(|lens| lens.model = None))
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
            assert_eq!(read.model(), Some(model), "{model:?}");
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
            make: Some("----".to_owned()),
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

    /// The colour space read from [`settings`]'s file with the ColorSpace
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
    fn a_colour_space_of_srgb_is_srgb_whatever_the_index() {
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
    fn an_uncalibrated_colour_space_with_the_dcf_option_files_index_is_adobe_rgb() {
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
    fn an_uncalibrated_colour_space_without_the_dcf_option_files_index_is_none() {
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
    fn the_index_alone_or_beside_an_undefined_colour_space_is_none() {
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
            let no_image = tree
                .iter()
                .filter(|(location, _)| location.holds_no_image());
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
        // sRGB, "uncalibrated" with no index, which gives no colour space,
        // a code Exif does not define, a LONG and two SHORTs, which are
        // not read: each is the Exif IFD's word on the colour space.
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
    /// fill mode, a manual white balance for shade, and a colour space
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
            make: Some("SONY".to_owned()),
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
    fn a_damaged_index_costs_only_the_colour_space() {
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

    /// The PhotometricInterpretation of a colour filter array: raw data,
    /// as the preview finder takes it.
    const CFA: u16 = 32803;

    /// Gives `ifd` the tags of raw data: samples of a colour filter array,
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
                    make: Some("SONY".to_owned()),
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
}
