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
//!   PreviewIFD, and gives the PreviewIFD's JPEG as a candidate preview;
//! - [`metadata`](fn@metadata) reads what the MakerNote adds to the
//!   standard tags, with the four files below;
//! - [`metadata`](mod@metadata) reads CropArea, the serial number, the ISO,
//!   the lens's range and WorldTime;
//! - [`settings`] reads the camera settings;
//! - [`af_info2`] reads AFInfo2, the focus point it records and the points
//!   it names;
//! - [`crop`] reads CropHiSpeed, and places a focus point on a crop or
//!   from the body's table;
//! - [`MakerNote::add_entries`] lists both IFDs' entries in the raw view,
//!   as the directories `MakerNote` and `MakerNoteSub(0x0011)`.
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
mod settings;
#[cfg(test)]
mod testing;

pub(in crate::formats) use block::{MakerNote, preview_candidate, read};

use std::io::{Read, Seek};

use crate::error::Result;
use crate::focus::FocusPoints;
use crate::formats::makernote::points::{self, PointTable};
use crate::formats::tiff_metadata::{self, Standard};
use crate::geometry::Size;
use crate::io::Source;
use crate::metadata::Metadata;
use af_info2::af_info2_record;
use crop::{CropHiSpeed, crop_hi_speed, in_crop, placed};
use metadata::{CropArea, crop_area, iso, lens_range, serial_number, world_time};
use settings::camera_settings;

/// The Nikon MakerNote tag that records the ISO the picture was taken at
/// (ISO): two SHORTs, the second of them the ISO.
const ISO: u16 = 0x0002;

/// The Nikon MakerNote tag that records the white balance setting
/// (WhiteBalance), as ASCII text. It is an upper-case word padded with
/// spaces, such as `AUTO1` or `SUNNY`, or a chosen color temperature as
/// its number, such as `3230K`.
const WHITE_BALANCE: u16 = 0x0005;

/// The Nikon MakerNote tag that records how the camera focused
/// (FocusMode), as ASCII text: `MANUAL` for manual focus, or an autofocus
/// mode padded with spaces to six characters, such as `AF-S  `.
const FOCUS_MODE: u16 = 0x0007;

/// The Nikon MakerNote tag that records the white balance fine-tuning in
/// whole steps (WhiteBalanceFineTune): two SSHORTs, amber to blue then
/// magenta to green. The oldest bodies write one, whose axis and sign no
/// file shows.
const FINE_TUNE_STEPS: u16 = 0x000B;

/// The Nikon MakerNote tag that points at the PreviewIFD, the sub-IFD
/// holding the small preview JPEG.
const PREVIEW_IFD: u16 = 0x0011;

/// The Nikon MakerNote tag that records the crop mode the picture was
/// taken in, and how large the picture was before and after the crop
/// (CropHiSpeed). It holds seven SHORTs: the mode, the original width and
/// height, the cropped width and height, and the left column and top row
/// where the crop starts.
const CROP_HI_SPEED: u16 = 0x001B;

/// The Nikon MakerNote tag that records the camera body's serial number
/// (SerialNumber), as ASCII text.
const SERIAL_NUMBER: u16 = 0x001D;

/// The Nikon MakerNote tag that records the color space the camera was
/// set to for its JPEG output (ColorSpace): one SHORT, 1 sRGB, 2 Adobe RGB.
const COLOR_SPACE: u16 = 0x001E;

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

/// The Nikon MakerNote tag that records the raw size the camera was set to
/// (ImageSizeRAW): one SHORT, 1 large, 2 medium, 3 small. In a JPEG it
/// follows the JPEG's own size instead.
const IMAGE_SIZE_RAW: u16 = 0x003E;

/// The Nikon MakerNote tag that records the white balance fine-tuning in
/// the camera's own steps (WhiteBalanceFineTune): two SRATIONALs, amber to
/// blue then magenta to green.
const FINE_TUNE: u16 = 0x003F;

/// The Nikon MakerNote tag that records the visible image, the rectangle
/// of the stored raw data inside its border (CropArea): its left column,
/// top row, width and height.
const CROP_AREA: u16 = 0x0045;

/// The Nikon MakerNote tag that records the color temperature of the
/// white balance the camera applied, in kelvin (ColorTemperatureAuto): one
/// SHORT, 0 when none was recorded.
const COLOR_TEMPERATURE: u16 = 0x004F;

/// The Nikon MakerNote tag that records the lens's range (Lens): its
/// shortest and longest focal length and the f-number at each, as four
/// RATIONALs in the order of the Exif LensSpecification.
const LENS: u16 = 0x0084;

/// The Nikon MakerNote tag that records where the camera's autofocus
/// aimed (AFInfo2): an UNDEFINED value that starts with its version, four
/// ASCII digits. At offsets each version sets, it holds the size of a
/// frame, the center of the focus area and the area's width and height,
/// as SHORTs in the MakerNote's byte order. Most versions add a byte that
/// says whether focus was confirmed.
const AF_INFO2: u16 = 0x00B7;

/// The rules of the tags this reader reads: each tag's number, from the
/// constants above, and the type and count Nikon writes its value in on
/// every file checked (ADR011). [`Ifd::get`] and [`Ifd::held`] read them.
/// Two are tags of the PreviewIFD, which Nikon writes as one LONG each.
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
    pub(super) const WHITE_BALANCE: Rule<Text> = Rule::new(super::WHITE_BALANCE, ASCII);
    pub(super) const FOCUS_MODE: Rule<Text> = Rule::new(super::FOCUS_MODE, ASCII);
    pub(super) const FINE_TUNE_STEPS: Held<[i16; 2]> =
        Held::new(super::FINE_TUNE_STEPS, &[FieldType::SShort]);
    pub(super) const PREVIEW_IFD: Held<[u32; 1]> = Held::new(super::PREVIEW_IFD, LONG);
    pub(super) const CROP_HI_SPEED: Rule<[u32; 7]> = Rule::new(super::CROP_HI_SPEED, SHORT);
    pub(super) const SERIAL_NUMBER: Rule<Text> = Rule::new(super::SERIAL_NUMBER, ASCII);
    pub(super) const COLOR_SPACE: Held<[u32; 1]> = Held::new(super::COLOR_SPACE, SHORT);
    pub(super) const WORLD_TIME: Held<[u8; 4]> =
        Held::new(super::WORLD_TIME, &[FieldType::Undefined]);
    pub(super) const SHUTTER_MODE: Held<[u32; 1]> = Held::new(super::SHUTTER_MODE, SHORT);
    pub(super) const IMAGE_SIZE_RAW: Held<[u32; 1]> = Held::new(super::IMAGE_SIZE_RAW, SHORT);
    pub(super) const FINE_TUNE: Rule<[SignedRational; 2]> =
        Rule::new(super::FINE_TUNE, &[FieldType::SRational]);
    pub(super) const CROP_AREA: Rule<[u32; 4]> =
        Rule::new(super::CROP_AREA, &[FieldType::Short, FieldType::Long]);
    pub(super) const COLOR_TEMPERATURE: Held<[u32; 1]> = Held::new(super::COLOR_TEMPERATURE, SHORT);
    pub(super) const LENS: Rule<[Rational; 4]> = Rule::new(super::LENS, &[FieldType::Rational]);
    pub(super) const AF_INFO2: Rule<Vec<u8>> = Rule::new(super::AF_INFO2, &[FieldType::Undefined]);

    // The PreviewIFD.
    pub(super) const JPEG_INTERCHANGE_FORMAT: Held<[u32; 1]> =
        Held::new(tags::JPEG_INTERCHANGE_FORMAT, LONG);
    pub(super) const JPEG_INTERCHANGE_FORMAT_LENGTH: Held<[u32; 1]> =
        Held::new(tags::JPEG_INTERCHANGE_FORMAT_LENGTH, LONG);
}

/// The manufacturer's name that a Nikon camera's Make starts with,
/// compared without regard to case. The NEF claim and the JPEG reader look
/// for it in IFD0's Make before they take a file for a Nikon one.
pub(in crate::formats) const NIKON: &[u8; 5] = b"NIKON";

/// What a file's picture is, as the format that read the MakerNote knows
/// it: what the focus point of a crop is placed on when the MakerNote
/// holds no CropArea. [`metadata`] turns it into the pixels it gives
/// [`focus`]: none for a raw file, the image size for a JPEG.
///
/// [`metadata`]: fn@metadata
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::formats) enum Picture {
    /// A raw file's, such as a NEF's: the picture is the part of the stored
    /// raw data that CropArea locates. A raw file has no pixels of its own,
    /// so a crop without a CropArea is read as recorded (ADR012).
    Raw,
    /// A JPEG's: the picture is the file's own pixels, of the size its
    /// frame header gives, which the metadata's image size already holds.
    /// A crop without a CropArea is placed on those pixels (ADR015).
    Jpeg,
}

/// The metadata of the file whose MakerNote is `makernote`: what the
/// standard tags gave, `standard`, with what the MakerNote records that
/// they lack. `picture` says what kind of picture the file holds.
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
///   capture time the standard tags gave without one: its
///   OffsetTimeOriginal is missing or could not be read. It is held to the
///   bound of every offset
///   ([`DateTime::with_offset_minutes`](crate::datetime::DateTime::with_offset_minutes)).
/// - **The camera settings** that only the MakerNote records, and its
///   white balance setting, finer than Exif's, in place of the mode the
///   Exif tags gave when it names one ([`camera_settings`]). The color space is read only
///   when the Exif IFD holds no ColorSpace, and the raw image size only for
///   a raw file's picture.
/// - **The focus points**, from AFInfo2 and FocusMode, always read, since
///   the standard tags never give one ([`focus`]). A recorded position is
///   placed on a crop's picture. The points a record with no position names
///   are placed from the table of the body the Model names, matched as
///   [`points::find`] matches it. A file without a Model has no table.
///
/// CropArea and CropHiSpeed are each read once, for the image size or the
/// image area and for the focus point. Every other value is the standard
/// tags'. The layout of the body's autofocus points is not given here:
/// detection gives it from the Model alone.
///
/// # Errors
///
/// `Io` when the reader fails.
pub(in crate::formats) fn metadata<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    standard: Standard,
    picture: Picture,
) -> Result<Metadata> {
    let Standard {
        mut metadata,
        from_default_crop,
        exif_records_color_space,
    } = standard;
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
    // An offset belongs to a time: without a capture time, WorldTime gives
    // nothing.
    if let Some(time) = metadata.capture_time
        && time.offset_minutes().is_none()
        && let Some(offset) = world_time(makernote)
        && let Ok(time) = time.with_offset_minutes(offset)
    {
        metadata.capture_time = Some(time);
    }
    // Read once, for the image area and the focus point.
    let crop_hi_speed = crop_hi_speed(src, makernote)?;
    metadata.camera_settings = camera_settings(
        src,
        makernote,
        metadata.camera_settings,
        crop_hi_speed,
        exif_records_color_space,
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
    metadata.focus_points = focus(src, makernote, crop_hi_speed, crop, pixels, table)?;
    Ok(metadata)
}

/// Where the camera's autofocus aimed, as `makernote`'s AFInfo2 records
/// it, with the area mode the record names ([`area_mode`]):
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
/// `None` when FocusMode records manual focus ([`manual_focus`]), when
/// AFInfo2 is missing or breaks its rule, or when its record cannot be
/// read ([`af_info2_record`]), checked or placed.
///
/// # Errors
///
/// `Io` when the reader fails.
///
/// [`area_mode`]: af_info2::area_mode
/// [`FocusRecord::records_position`]: af_info2::FocusRecord::records_position
/// [`NamedPoints`]: af_info2::NamedPoints
fn focus<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    crop: Option<CropHiSpeed>,
    crop_area: CropArea,
    pixels: Option<Size>,
    table: Option<&PointTable>,
) -> Result<Option<FocusPoints>> {
    let ifd = &makernote.ifd;
    // Without AFInfo2 there is nothing to place, so FocusMode is not read;
    // with manual focus there is nothing to place either, so AFInfo2 is not
    // read.
    if ifd.entry(AF_INFO2).is_none() || manual_focus(src, makernote)? {
        return Ok(None);
    }
    let Some(value) = ifd.get(src, rules::AF_INFO2)? else {
        return Ok(None);
    };
    let Some(record) = af_info2_record(&value, ifd.order()) else {
        return Ok(None);
    };
    let points = if record.records_position() {
        in_crop(record, crop, crop_area, pixels).map(FocusPoints::from)
    } else {
        record
            .named
            .zip(table)
            .and_then(|(named, table)| placed(named, table, crop, pixels))
    };
    Ok(points.map(|points| match record.area_mode {
        Some(mode) => points.with_area_mode(mode),
        None => points,
    }))
}

/// Whether `makernote`'s FocusMode reads `MANUAL`, exactly, by the text
/// rule ([`tiff_metadata::text`]). A FocusMode that is missing, breaks its
/// rule or reads anything else, such as `AF-S`, records no manual focus.
///
/// The point of a manual-focus shot is not one the camera focused on, so
/// such a shot gives none, whatever AFInfo2 holds (ADR012).
///
/// # Errors
///
/// `Io` when the reader fails.
fn manual_focus<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<bool> {
    let mode = tiff_metadata::text(src, &makernote.ifd, rules::FOCUS_MODE)?;
    Ok(mode.as_deref() == Some("MANUAL"))
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
