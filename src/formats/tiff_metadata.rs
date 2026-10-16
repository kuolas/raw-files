// SPDX-License-Identifier: MIT OR Apache-2.0

//! The metadata reader shared by the TIFF-based formats: it reads what the
//! camera recorded about a picture from the standard TIFF and Exif tags.
//!
//! Cameras of every maker record the same facts in the same standard tags:
//! the camera's make and model and the orientation in IFD0, the capture
//! time, the body's serial number, the exposure and the lens in the Exif
//! IFD, and the sizes in the IFD of the raw data. So [`read`] reads them
//! once, over the tree, for every format, and a format adds only what its
//! own structures record and the standard tags lack (ADR011, which chose
//! the standard tags first).
//!
//! The rules:
//!
//! - A damaged value costs only itself. A tag that is missing, of a type
//!   other than the standard's, or whose value cannot be read, such as one
//!   longer than one tag may take, gives `None` for its own value; every
//!   other value is still read, and the file still opens. Only a failure of
//!   the reader is an error: it says nothing about the file.
//! - The make, the model and the serial number, of the camera and of the
//!   lens, are text, read by the one rule [`text`] gives: cut at the first
//!   NUL, `None` when longer than 255 bytes, trimmed of spaces at both
//!   ends, `None` when nothing is left, and with bytes that are not UTF-8
//!   replaced by U+FFFD.
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
//!   0/6, is no compensation ([`SignedRational::defined`]).
//! - The lens is described by the Exif LensMake, LensModel and
//!   LensSerialNumber, and by the LensSpecification, of type RATIONAL and
//!   of exactly four values: the shortest and longest focal length and the
//!   f-number at each. A LensSpecification of any other count is ignored
//!   whole; each of its four values is `None` when its numerator or its
//!   denominator is zero, as a 0/0 for a value the camera did not know
//!   ([`Rational::known`]). A LensModel made only of `-` characters, which
//!   Sony writes for a manual lens, names no lens and is `None`. A file
//!   whose lens tags give no value has no lens ([`lens`]).
//! - A file without an Exif IFD has no capture time, no serial number, no
//!   exposure and no lens.
//! - The orientation is IFD0's Orientation, read as the preview finder reads
//!   it ([`tiff_previews::orientation`]), so that the metadata and the
//!   previews never disagree.
//! - The raw size is the ImageWidth and ImageLength, of type SHORT or LONG,
//!   of the IFD of the tree, the Exif IFD left out, that the preview finder
//!   takes for raw data ([`tiff_previews::is_raw`]), whose width and height
//!   are both above zero, and that has the most pixels of all such IFDs,
//!   the first in the tree's order on a tie: a file can hold a small
//!   full-resolution raw image beside the real one, as the public Coolpix
//!   E5700 sample 4456 does. An IFD whose tags say nothing readable either
//!   way is passed over, as the finder passes over it. Without such an IFD
//!   there is no raw size, and no image size.
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

use crate::container::tiff::tags::{
    BODY_SERIAL_NUMBER, DATE_TIME_ORIGINAL, DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE,
    EXPOSURE_BIAS_VALUE, EXPOSURE_TIME, F_NUMBER, FOCAL_LENGTH, IMAGE_LENGTH, IMAGE_WIDTH,
    ISO_SPEED, LENS_MAKE, LENS_MODEL, LENS_SERIAL_NUMBER, LENS_SPECIFICATION, MAKE, MODEL,
    OFFSET_TIME_ORIGINAL, PHOTOGRAPHIC_SENSITIVITY, RECOMMENDED_EXPOSURE_INDEX,
    SUB_SEC_TIME_ORIGINAL,
};
use crate::container::tiff::{Ifd, Location, Tree, skip_damage};
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

/// What [`read`] reads from the standard tags: the metadata, and whether
/// its image size is a crop the raw IFD records, for a format whose own
/// structures record a crop too.
///
/// A format's own crop, such as Nikon's crop area, may replace the raw size
/// the image size otherwise falls back on, but never a default crop: the
/// standard tags come first (ADR011). Comparing the image size with the raw
/// size cannot tell the two apart, since a default crop may cover the whole
/// raw.
#[derive(Debug)]
pub(crate) struct Standard {
    /// The metadata, every value of it from the standard tags.
    pub(crate) metadata: Metadata,
    /// Whether the image size is the raw IFD's default crop; `false` when
    /// it is the raw size, or there is no image size.
    pub(crate) from_default_crop: bool,
}

/// Reads the metadata of the TIFF-based file in `src`, whose IFDs are
/// `tree`, with whether its image size is a default crop.
///
/// See the [module documentation](self) for the rules.
///
/// # Errors
///
/// `Io` when the reader fails. A value that cannot be read for any other
/// reason is `None`, and costs no other value.
pub(crate) fn read<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Standard> {
    let ifd0 = tree.ifd0();
    let (serial_number, capture_time, exposure, lens) = match tree.exif() {
        Some(exif) => (
            text(src, exif, BODY_SERIAL_NUMBER)?,
            capture_time(src, exif)?,
            exposure(src, exif)?,
            lens(src, exif)?,
        ),
        None => (None, None, Exposure::default(), None),
    };
    let (raw_size, default_crop) = sizes(src, tree)?;
    let metadata = Metadata {
        make: text(src, ifd0, MAKE)?,
        model: text(src, ifd0, MODEL)?,
        serial_number,
        capture_time,
        orientation: tiff_previews::orientation(src, tree)?,
        exposure,
        lens,
        raw_size,
        image_size: default_crop.or(raw_size),
    };
    Ok(Standard {
        metadata,
        from_default_crop: default_crop.is_some(),
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
/// ISO, as [`iso`] reads it, and the four fractions, as [`rational`] and
/// [`signed_rational`] read them.
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
        exposure_bias: signed_rational(src, exif, EXPOSURE_BIAS_VALUE)?,
    })
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
/// ([`SignedRational::defined`]).
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
    Ok(first.and_then(|(numerator, denominator)| SignedRational::defined(numerator, denominator)))
}

/// The lens the picture was taken with, from the Exif IFD `exif`: its
/// LensMake, LensModel and LensSerialNumber, read by the rule [`text`]
/// gives, and its range from the LensSpecification, as
/// [`lens_specification`] reads it. `None` when none of them gives a
/// value ([`Lens::new`]).
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
fn lens<R: Read + Seek>(src: &mut Source<R>, exif: &Ifd) -> Result<Option<Lens>> {
    let make = text(src, exif, LENS_MAKE)?;
    let model = text(src, exif, LENS_MODEL)?.filter(|model| !model.chars().all(|c| c == '-'));
    let serial_number = text(src, exif, LENS_SERIAL_NUMBER)?;
    let [min_focal, max_focal, f_number_at_min, f_number_at_max] = lens_specification(src, exif)?;
    Ok(Lens::new(
        make,
        model,
        serial_number,
        min_focal,
        max_focal,
        f_number_at_min,
        f_number_at_max,
    ))
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

/// The raw size and the default crop of the file whose IFDs are `tree`:
/// both from the largest IFD that holds raw data and says how large it is,
/// or both `None` when no IFD does.
///
/// The IFDs are asked in the tree's order, the Exif IFD left out, since it
/// holds capture metadata and never an image. An IFD counts as raw data
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
        if location == Location::Exif {
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
    use crate::container::tiff::{parse_header, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing::{self, FAR, FailsFar};
    use crate::limits::Limits;
    use crate::orientation::Orientation;
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
    /// Exif IFD's. The time and the lens are those of the public Z 9
    /// sample 5147; the serial numbers are made up, the lens's with the two
    /// spaces the Z 9 pads its eight digits with.
    const IFD0_TEXT: [(u16, &str); 2] = [(MAKE, "NIKON CORPORATION"), (MODEL, "NIKON Z 9")];
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
    const EXIF_EXPOSURE: [(u16, AddExposure); 6] = [
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
    ];

    /// More bytes than the 1 MiB one tag value may take.
    const TOO_LONG: u32 = 1024 * 1024 + 1;

    /// A little-endian file whose IFD0 holds the Orientation 8 and
    /// [`IFD0_TEXT`], and whose Exif IFD holds [`EXIF_TEXT`], with the
    /// LensModel padded by [`padded_lens_model`], [`EXIF_EXPOSURE`] and
    /// [`Z9_LENS_SPECIFICATION`], all but tag `except`. `change` is then
    /// given the builder and the IDs of IFD0 and of the Exif IFD, to add
    /// what the test needs in its place.
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
        if except != LENS_SPECIFICATION {
            builder.rational(exif, LENS_SPECIFICATION, &Z9_LENS_SPECIFICATION);
        }
        change(&mut builder, ifd0, exif);
        builder.build().bytes
    }

    /// The metadata read from `bytes`; the reader must succeed.
    fn read_bytes(bytes: Vec<u8>) -> Metadata {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
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
    /// 1/320 s at f/9, 18 mm and ISO 64, with no compensation.
    fn z9_exposure() -> Exposure {
        Exposure {
            iso: Some(64),
            exposure_time: Some(fraction(10, 3200)),
            f_number: Some(fraction(900, 100)),
            focal_length: Some(fraction(180, 10)),
            exposure_bias: Some(signed(0, 6)),
        }
    }

    /// The lens of the Z 9 file, as [`EXIF_TEXT`] and
    /// [`Z9_LENS_SPECIFICATION`] record it, with `change` made to its
    /// values in the order [`Lens::new`] takes them.
    fn z9_lens_with(change: impl FnOnce(&mut LensValues)) -> Option<Lens> {
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
    fn z9_lens() -> Option<Lens> {
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
        /// No value: the lens of a file that describes none.
        fn none() -> Self {
            Self {
                make: None,
                model: None,
                serial_number: None,
                range: [None; 4],
            }
        }

        /// The lens these values describe, as [`Lens::new`] builds it.
        fn lens(self) -> Option<Lens> {
            let [min, max, f_min, f_max] = self.range;
            Lens::new(
                self.make,
                self.model,
                self.serial_number,
                min,
                max,
                f_min,
                f_max,
            )
        }
    }

    /// What the Z 9 file gives, every value read.
    fn z9_expected() -> Metadata {
        Metadata {
            make: Some("NIKON CORPORATION".to_owned()),
            model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("1234567".to_owned()),
            capture_time: z9_time(Some(b"85"), Some(b"-05:00")),
            orientation: Orientation::Rotate270,
            exposure: z9_exposure(),
            lens: z9_lens(),
            ..Metadata::default()
        }
    }

    /// The Z 9 file's metadata with the lens `lens` instead of its own.
    fn z9_expected_with_lens(lens: Option<Lens>) -> Metadata {
        Metadata {
            lens,
            ..z9_expected()
        }
    }

    /// The lens read from a file whose Exif IFD holds only what `build`
    /// adds, given the builder and the Exif IFD's ID.
    fn lens(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> Option<Lens> {
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
        let time = metadata.capture_time().unwrap();
        assert_eq!(time.to_string(), "2022-02-05T13:46:41.85-05:00");
        assert_eq!(time.offset_minutes(), Some(-300));
        assert_eq!(time.nanosecond(), 850_000_000);
        let exposure = metadata.exposure();
        assert_eq!(exposure.iso(), Some(64));
        assert_eq!(exposure.exposure_time(), Some(fraction(10, 3200)));
        assert_eq!(exposure.f_number(), Some(fraction(900, 100)));
        assert_eq!(exposure.focal_length(), Some(fraction(180, 10)));
        assert_eq!(exposure.exposure_bias(), Some(signed(0, 6)));
        let lens = metadata.lens().unwrap();
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
        // The Exif tags in IFD0, where they do not belong, are not read.
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
        let cases: [(&str, u16, Change, Metadata); 16] = [
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
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
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
        // inline one that does not.
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
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
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
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
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
        // With no other lens tag, as on that sample, there is no lens.
        for model in ["----", "-", " -- "] {
            let read = lens(|builder, exif| {
                builder.ascii(exif, LENS_MODEL, model);
            });
            assert_eq!(read, None, "{model:?}");
        }
    }

    #[test]
    fn dashes_among_other_characters_are_kept() {
        for model in ["FE 24-70mm F2.8 GM", "-A-", "--x"] {
            let read = lens(|builder, exif| {
                builder.ascii(exif, LENS_MODEL, model);
            });
            assert_eq!(
                read.as_ref().and_then(Lens::model),
                Some(model),
                "{model:?}"
            );
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
        // With no other lens tag there is no lens.
        let read = lens(|builder, exif| {
            builder.rational(
                exif,
                LENS_SPECIFICATION,
                &[(140, 10), (300, 10), (400, 100)],
            );
        });
        assert_eq!(read, None);
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
    fn a_lens_with_only_a_serial_number_is_a_lens() {
        let read = lens(|builder, exif| {
            builder.ascii(exif, LENS_SERIAL_NUMBER, "12345678  ");
        });
        let expected = LensValues {
            serial_number: Some("12345678".to_owned()),
            ..LensValues::none()
        };
        assert_eq!(read, expected.lens());
        assert!(read.is_some());
    }

    #[test]
    fn an_exif_ifd_without_lens_tags_gives_no_lens() {
        // An IFD without entries is damage and not read at all, so the Exif
        // IFD holds a tag other than the lens's.
        let read = lens(|builder, exif| {
            builder.ascii(exif, DATE_TIME_ORIGINAL, "2022:02:05 13:46:41");
        });
        assert_eq!(read, None);
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

    /// A size of `width` by `height` pixels.
    fn size(width: u32, height: u32) -> Size {
        Size { width, height }
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
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
            let err = read(&mut src, &tree).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{label}: {err:?}");
        }
    }
}
