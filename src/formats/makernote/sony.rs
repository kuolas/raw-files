// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Sony MakerNote reader: how its directory is found and checked, and
//! the focus point and the camera settings read from it.
//!
//! The MakerNote is a directory of Sony's tags, an IFD like the file's
//! own. raw-files reads it for what no standard tag records and Sony keeps
//! in plain tags: the point where the camera focused (ADR012), and the
//! camera settings that only the MakerNote records (ADR011). Nothing is
//! deciphered: Sony enciphers several tags' values, and none of them is
//! read. FileFormat (0xB000), SonyModelID (0xB001) and the lens tags are
//! not read either, since the standard tags give the camera and the lens.
//!
//! The parts, in the order a file is read:
//!
//! - [`read`] tells the block's layout by its first bytes ([`ifd_start`]),
//!   and reads its IFD once, into a [`MakerNote`];
//! - [`metadata`] reads the camera settings ([`camera_settings`]) and the
//!   focus point ([`focus`]);
//! - [`MakerNote::add_entries`] lists the IFD's entries in the raw view.
//!
//! The Make is not looked at here: the format's reader has taken the file
//! for a Sony one by [`SONY`]. Every offset inside the block counts from
//! the TIFF header of the structure that holds the MakerNote, as in Nikon's
//! headerless layout. Every number is in that structure's byte order. So
//! the IFD is read through the block ([`MakerNoteBlock::ifd_at`]).
//!
//! The four bytes after the IFD's entries are not a next-IFD pointer to
//! rely on. They are zero on the older bodies, such as the A100, the NEX-5
//! and the DSC-R1, and the first bytes of the values on the newer ones.
//! They are neither used nor required, as the ARW security notes ask
//! (`docs/formats/arw/security.md`, item A5). Nor need the tags be sorted:
//! the IFD keeps the file's order, and each tag is found wherever it is.
//!
//! Nothing the block holds is an error. A block too short for its header
//! or for the IFD's count, or an IFD cut short, with no entries or with
//! more than the limits allow, costs the MakerNote, never the file.
//!
//! The meaning of each code is ExifTool's, read in its published tag
//! documentation. Each was confirmed on the public samples and on frames I
//! shot with an ILCE-7M2 under each setting, also read in Sony's Imaging
//! Edge Desktop, run as a reader to compare with. The ARW notes record
//! them, and give the layouts (`docs/formats/arw/makernote-and-metadata.md`,
//! section 1), what each focus tag records (section 9.6) and each settings
//! tag (section 9.7). A sample cited here by number is the file of that
//! number on raw.pixls.us.

use std::io::{Read, Seek};

use super::{MakerNoteBlock, checked_color_temperature};
use crate::camera_settings::{CameraSettings, FineTune, WhiteBalanceMode};
use crate::container::tiff::{Ifd, Rule, Tree};
use crate::entries::{Directory, Entries};
use crate::error::Result;
use crate::focus::{Focus, FocusOrigin, FocusPoints};
use crate::geometry::{Point, Size};
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::Metadata;
use crate::rational::SignedRational;

/// The manufacturer's name that a Sony camera's Make starts with, compared
/// without regard to case. The ARW claim and the JPEG reader look for it in
/// IFD0's Make before they take a file for a Sony one.
pub(in crate::formats) const SONY: &[u8; 4] = b"SONY";

/// The Sony MakerNote tag that records the white balance setting
/// (WhiteBalance): one LONG, a code for the setting, such as 0 for auto,
/// 48 for shade or 1 for a chosen color temperature.
pub(in crate::formats) const WHITE_BALANCE: u16 = 0x0115;

/// The Sony MakerNote tag that records the white balance fine-tuning
/// (WBShiftAB_GM): two SLONGs, whole steps, the shift from amber to blue,
/// positive towards amber, then the one from green to magenta, positive
/// towards magenta.
pub(in crate::formats) const FINE_TUNE: u16 = 0x2014;

/// The Sony MakerNote tag that records the electronic front curtain
/// setting (ElectronicFrontCurtainShutter): one LONG, 0 off and 1 on. It is
/// the menu setting, not the shutter used: on four samples it contradicts
/// the release an enciphered block records, as ExifTool, run on the files
/// as a reader to compare with, decodes it.
pub(in crate::formats) const ELECTRONIC_FRONT_CURTAIN: u16 = 0x201A;

/// The Sony MakerNote tag that records how the camera focused
/// (FocusMode): one BYTE, 0 for manual focus, and 2, 3, 4, 6 or 7 for the
/// autofocus modes AF-S, AF-C, AF-A, DMF and AF-D.
pub(in crate::formats) const FOCUS_MODE: u16 = 0x201B;

/// The Sony MakerNote tag that records where the camera focused
/// (FocusLocation): four SHORTs, the width and height of the frame, then
/// the x and y of the focus area's center in it.
pub(in crate::formats) const FOCUS_LOCATION: u16 = 0x2027;

/// The Sony MakerNote tag that records the size of the focus area
/// (FocusFrameSize): six UNDEFINED bytes, the area's width and height as
/// 16-bit numbers in the MakerNote's byte order, then a third, 257 or 0,
/// whose meaning is unknown.
const FOCUS_FRAME_SIZE: u16 = 0x2037;

/// The Sony MakerNote tag that repeats FocusLocation on the newest bodies
/// (FocusLocation2): the same four SHORTs, with the same values.
const FOCUS_LOCATION2: u16 = 0x204A;

/// The Sony MakerNote tag that records a color temperature, in kelvin
/// (ColorTemperature): one LONG, the one chosen under a chosen color
/// temperature and the custom white balance's own under custom, and 0 when
/// none was recorded.
const COLOR_TEMPERATURE: u16 = 0xB021;

/// The rules of the tags this reader reads: each tag's number, from the
/// constants above, and the type and count Sony writes its value in on
/// every file checked (ADR011). [`Ifd::get`] and [`Ifd::held`] read them.
/// The fine-tuning's rule names SLONG alone, so that a negative value is
/// never read as a large one.
mod rules {
    use crate::container::tiff::{FieldType, Held, Rule};

    const LONG: &[FieldType] = &[FieldType::Long];
    const SHORT: &[FieldType] = &[FieldType::Short];

    pub(super) const WHITE_BALANCE: Held<[u32; 1]> = Held::new(super::WHITE_BALANCE, LONG);
    pub(super) const FINE_TUNE: Rule<[i32; 2]> = Rule::new(super::FINE_TUNE, &[FieldType::SLong]);
    pub(super) const ELECTRONIC_FRONT_CURTAIN: Held<[u32; 1]> =
        Held::new(super::ELECTRONIC_FRONT_CURTAIN, LONG);
    pub(super) const FOCUS_MODE: Held<[u8; 1]> = Held::new(super::FOCUS_MODE, &[FieldType::Byte]);
    pub(super) const FOCUS_LOCATION: Rule<[u32; 4]> = Rule::new(super::FOCUS_LOCATION, SHORT);
    pub(super) const FOCUS_FRAME_SIZE: Rule<[u8; 6]> =
        Rule::new(super::FOCUS_FRAME_SIZE, &[FieldType::Undefined]);
    pub(super) const FOCUS_LOCATION2: Rule<[u32; 4]> = Rule::new(super::FOCUS_LOCATION2, SHORT);
    pub(super) const COLOR_TEMPERATURE: Held<[u32; 1]> = Held::new(super::COLOR_TEMPERATURE, LONG);
}

/// The header of the DSLR-A100's MakerNote (`SONY DSC `, then three NULs),
/// which ExifTool's documentation also lists for the DSC compacts' JPEG
/// files.
const SONY_DSC: &[u8; 12] = b"SONY DSC \0\0\0";

/// A header ExifTool's documentation lists for Sony files (`SONY CAM `,
/// then three NULs), seen in no sample.
const SONY_CAM: &[u8; 12] = b"SONY CAM \0\0\0";

/// A header ExifTool's documentation lists for Sony files (`SONY MOBILE`),
/// seen in no sample; the twelfth byte is not looked at.
const SONY_MOBILE: &[u8; 11] = b"SONY MOBILE";

/// A header ExifTool's documentation lists for Sony files (two NULs, then
/// `SONY PIC` and a NUL), seen in no sample; the twelfth byte is not looked
/// at.
const SONY_PIC_HEADER: &[u8; 11] = b"\0\0SONY PIC\0";

/// A header ExifTool's documentation lists for Sony files (`VHAB`, five
/// spaces and a NUL), seen in no sample; the last two of the twelve bytes
/// are not looked at.
const VHAB: &[u8; 10] = b"VHAB     \0";

/// The headers that put the MakerNote IFD [`HEADER_LEN`] bytes into the
/// block.
const HEADERS: [&[u8]; 5] = [SONY_DSC, SONY_CAM, SONY_MOBILE, SONY_PIC_HEADER, VHAB];

/// How far into the block the MakerNote IFD starts after a header.
const HEADER_LEN: u64 = 12;

/// The start of an Olympus-format MakerNote from a Sony compact's JPEG
/// file (`SONY PI` and a NUL), with Olympus's tags.
const SONY_PI: &[u8; 8] = b"SONY PI\0";

/// The start of another Olympus-format MakerNote from a Sony compact's
/// JPEG file (`PREMI` and a NUL), with Olympus's tags.
const PREMI: &[u8; 6] = b"PREMI\0";

/// The start of a MakerNote of text from a Sony compact's JPEG file
/// (`SONY PIC` and a NUL).
const SONY_PIC: &[u8; 9] = b"SONY PIC\0";

/// The starts of the blocks that hold no Sony directory.
const NOT_SONY: [&[u8]; 3] = [SONY_PI, PREMI, SONY_PIC];

/// A Sony MakerNote, its block found and its IFD read: what every use of
/// the MakerNote starts from.
#[derive(Debug)]
pub(in crate::formats) struct MakerNote {
    /// The MakerNote IFD, which holds Sony's tags: at the block's start, or
    /// 12 bytes in after a header. It was read inside the block, so every
    /// value read from it lies inside the block.
    ///
    /// Its block, [`Ifd::block`], has the MakerNote's bytes as bounds, and
    /// the Exif IFD's base.
    ifd: Ifd,
}

impl MakerNote {
    /// Adds the MakerNote IFD to `entries`, as the directory `MakerNote`.
    /// Its next-IFD pointer is never followed: on most files the slot holds
    /// the first value instead, and an SRF file's chain is enciphered.
    ///
    /// The records Sony enciphers, such as 0x2010 and 0x9050, are plain
    /// UNDEFINED entries, listed as recorded, neither marked nor deciphered
    /// (ADR023).
    pub(in crate::formats) fn add_entries(&self, entries: &mut Entries) {
        entries.push(Directory::MakerNote, &self.ifd);
    }
}

/// The Sony MakerNote of the file in `src`, whose IFDs are `tree`, read
/// within `limits`.
///
/// `None` when the file has no Exif IFD or no MakerNote, when the
/// MakerNote is not bytes stored out of line or holds no Sony directory,
/// or when its IFD does not hold together inside the block.
///
/// # Errors
///
/// `Io` when the reader fails.
pub(in crate::formats) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    let Some(block) = MakerNoteBlock::find(tree) else {
        return Ok(None);
    };
    let head = block.first_bytes(src, HEADER_LEN)?;
    let Some(skip) = ifd_start(&head) else {
        return Ok(None);
    };
    // The Exif IFD's base and byte order.
    let ifd = block.ifd_at(src, skip, limits)?;
    Ok(ifd.map(|ifd| MakerNote { ifd }))
}

/// How far into the block the MakerNote IFD starts, from `head`, the
/// block's first [`HEADER_LEN`] bytes at most. It is [`HEADER_LEN`] after
/// one of [`HEADERS`], and 0 without one. `None` when the block starts as
/// one of [`NOT_SONY`], and holds no Sony directory.
fn ifd_start(head: &[u8]) -> Option<u64> {
    if HEADERS.iter().any(|header| head.starts_with(header)) {
        Some(HEADER_LEN)
    } else if NOT_SONY.iter().any(|start| head.starts_with(start)) {
        None
    } else {
        Some(0)
    }
}

/// The metadata of a file whose Sony MakerNote is `makernote`: `metadata`,
/// what the standard tags gave, with what the MakerNote records that they
/// lack. It is the same in every format whose files carry it, raw or JPEG:
///
/// - **the camera settings** that only the MakerNote records, and the white
///   balance setting, in place of the Exif tags' mode
///   ([`camera_settings`]);
/// - **the focus point**, as recorded ([`focus`]), always read, since the
///   standard tags never give one. It is a list of one point, the primary,
///   with no area mode: AFAreaModeSetting is not read, since no file shot
///   in a known mode has checked what its codes mean.
///
/// The other values are the standard tags' alone.
///
/// # Errors
///
/// `Io` when the reader fails.
pub(in crate::formats) fn metadata<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    mut metadata: Metadata,
) -> Result<Metadata> {
    metadata.camera_settings = camera_settings(src, makernote, metadata.camera_settings)?;
    metadata.focus_points = focus(src, makernote)?.map(FocusPoints::from);
    Ok(metadata)
}

/// Where the camera focused, as `makernote` records it: the frame and the
/// center of the focus area from FocusLocation ([`location`]), and the
/// area's size from FocusFrameSize when it records one ([`area`]).
///
/// The point is given as recorded, in the frame as the camera framed it,
/// which is not always the image size raw-files reports. Whether focus
/// was confirmed is never set: no plain tag records it.
///
/// FocusLocation2 is read only when FocusLocation gives no location,
/// missing or of another shape. No sample has FocusLocation2 alone, so
/// such a file is unchecked.
///
/// `None` when FocusMode records manual focus ([`manual_focus`]), or when
/// neither tag gives a location. `None` too when [`Focus::new`] refuses the
/// location, for a frame with a zero side or a center off the frame, or
/// when [`Focus::with_size`] refuses an area that does not fit inside the
/// frame around the center.
///
/// # Errors
///
/// `Io` when the reader fails.
fn focus<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<Option<Focus>> {
    if manual_focus(makernote) {
        return Ok(None);
    }
    let mut found = location(src, makernote, rules::FOCUS_LOCATION)?;
    if found.is_none() {
        found = location(src, makernote, rules::FOCUS_LOCATION2)?;
    }
    let Some(focus) =
        found.and_then(|(frame, center)| Focus::new(frame, center, FocusOrigin::Recorded).ok())
    else {
        return Ok(None);
    };
    // An area that `with_size` refuses costs the whole point. Such a
    // record cannot be used, and gives no focus on every format (ADR012).
    Ok(match area(src, makernote)? {
        Some(size) => focus.with_size(size).ok(),
        None => Some(focus),
    })
}

/// Whether `makernote`'s FocusMode records manual focus: whether it is one
/// BYTE, 0. A FocusMode that is missing, breaks its rule or holds any other
/// value, such as 3 for AF-C, records none.
///
/// A shot in manual focus still records a location, the frame's exact
/// center, with FocusMode 0 (samples 1310 and 1311). The camera did not
/// focus on that point, so FocusMode 0 gives none, whatever FocusLocation
/// holds (ADR012). Nothing else in the plain tags says that no autofocus
/// took place, so any other FocusMode, or none, keeps the point.
fn manual_focus(makernote: &MakerNote) -> bool {
    makernote.ifd.held(rules::FOCUS_MODE) == Some([0])
}

/// The frame and the center that `makernote`'s tag `tag`, FocusLocation or
/// FocusLocation2, records: its four SHORTs, the frame's width and height,
/// then the center's x and y. `None` when the tag is missing or breaks its
/// rule. The values are checked by [`Focus::new`].
///
/// # Errors
///
/// `Io` when the reader fails.
fn location<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    rule: Rule<[u32; 4]>,
) -> Result<Option<(Size, Point)>> {
    let Some([width, height, x, y]) = makernote.ifd.get(src, rule)? else {
        return Ok(None);
    };
    Ok(Some((Size { width, height }, Point { x, y })))
}

/// The size of the focus area that `makernote`'s FocusFrameSize records:
/// the first two of its three 16-bit numbers, the area's width and height,
/// in the MakerNote's byte order. The third is not read.
///
/// `None` when the tag is missing or breaks its rule, and when it records a
/// width or a height of zero, which on Sony says only that no size is
/// recorded (ADR012). The point is then kept without a size.
///
/// # Errors
///
/// `Io` when the reader fails.
fn area<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<Option<Size>> {
    let Some([width0, width1, height0, height1, ..]) =
        makernote.ifd.get(src, rules::FOCUS_FRAME_SIZE)?
    else {
        return Ok(None);
    };
    let order = makernote.ifd.order();
    let width = u32::from(order.u16([width0, width1]));
    let height = u32::from(order.u16([height0, height1]));
    Ok((width != 0 && height != 0).then_some(Size { width, height }))
}

/// How the camera was set for the picture: `settings`, what the standard
/// tags gave, with what `makernote` records. Each value is read by its own
/// rule, so that a damaged one costs only itself:
///
/// - **The white balance mode** is the one WhiteBalance's code names
///   ([`white_balance_mode`]), in place of the Exif tags' mode, which
///   records only auto or the kind of light, and calls the underwater
///   setting auto. When the code names none, the Exif tags' mode stays.
/// - **The color temperature** is ColorTemperature, from 1 to 99,999 K
///   ([`checked_color_temperature`]), under whichever white balance
///   setting: the chosen color temperature under that setting, and the
///   custom white balance's own under custom. 0, which every other setting
///   holds on every file checked, records none.
/// - **The fine-tuning** is WBShiftAB_GM's ([`fine_tune`]).
/// - **The electronic front curtain setting** is
///   ElectronicFrontCurtainShutter's ([`electronic_front_curtain_setting`]).
///
/// The shutter used, the image area and the raw image size stay as
/// `settings` has them, `None`. Sony records the first two only in
/// enciphered blocks, and gives an APS-C raw the raw size code of a medium
/// one. The exposure program, the metering mode, the flash and the color
/// space are the Exif tags' alone. Neither 0xB054 nor 0x0112, which
/// ExifTool's documentation names for the white balance, is read: on the
/// files checked, neither records the setting.
///
/// # Errors
///
/// `Io` when the reader fails.
fn camera_settings<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    mut settings: CameraSettings,
) -> Result<CameraSettings> {
    let ifd = &makernote.ifd;
    let mode = ifd
        .held(rules::WHITE_BALANCE)
        .and_then(|[code]| white_balance_mode(code));
    if let Some(mode) = mode {
        settings.white_balance.mode = Some(mode);
    }
    settings.white_balance.color_temperature = ifd
        .held(rules::COLOR_TEMPERATURE)
        .and_then(|[kelvin]| checked_color_temperature(kelvin));
    settings.white_balance.fine_tune = fine_tune(src, makernote)?;
    settings.electronic_front_curtain_setting = ifd
        .held(rules::ELECTRONIC_FRONT_CURTAIN)
        .and_then(|[code]| electronic_front_curtain_setting(code));
    Ok(settings)
}

/// The white balance mode a WhiteBalance `code` names:
///
/// - 0 [`Auto`](WhiteBalanceMode::Auto);
/// - 1, a chosen color temperature,
///   [`ColorTemperature`](WhiteBalanceMode::ColorTemperature);
/// - 16 [`Daylight`](WhiteBalanceMode::Daylight), 32
///   [`Cloudy`](WhiteBalanceMode::Cloudy) and 48
///   [`Shade`](WhiteBalanceMode::Shade);
/// - 64 [`Tungsten`](WhiteBalanceMode::Tungsten) and 80
///   [`Flash`](WhiteBalanceMode::Flash);
/// - 96, each of the four fluorescent lamps,
///   [`Fluorescent`](WhiteBalanceMode::Fluorescent);
/// - 112 [`Custom`](WhiteBalanceMode::Custom) and 128
///   [`Underwater`](WhiteBalanceMode::Underwater).
///
/// ExifTool's documentation names 1 a color temperature or a color filter.
/// No checked file was shot with a color filter, so whether one writes 1
/// too is unchecked. `None` for every other code: a code joins the table
/// only once a file shows it, and until then the Exif tags' mode stands.
fn white_balance_mode(code: u32) -> Option<WhiteBalanceMode> {
    Some(match code {
        0 => WhiteBalanceMode::Auto,
        1 => WhiteBalanceMode::ColorTemperature,
        16 => WhiteBalanceMode::Daylight,
        32 => WhiteBalanceMode::Cloudy,
        48 => WhiteBalanceMode::Shade,
        64 => WhiteBalanceMode::Tungsten,
        80 => WhiteBalanceMode::Flash,
        96 => WhiteBalanceMode::Fluorescent,
        112 => WhiteBalanceMode::Custom,
        128 => WhiteBalanceMode::Underwater,
        _ => return None,
    })
}

/// The white balance fine-tuning that `makernote`'s WBShiftAB_GM records,
/// signed as [`FineTune`] signs it ([`fine_tune_of`]): its two SLONGs,
/// whole steps, the shift from amber to blue and then the one from green
/// to magenta. A recorded zero shift, `0, 0`, is a fine-tuning like any
/// other.
///
/// `None` when the tag is missing, as on the oldest bodies, breaks its rule
/// or stores its values outside the block, and when [`fine_tune_of`] gives
/// none.
///
/// # Errors
///
/// `Io` when the reader fails.
fn fine_tune<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<FineTune>> {
    let Some([blue_amber, green_magenta]) = makernote.ifd.get(src, rules::FINE_TUNE)? else {
        return Ok(None);
    };
    Ok(fine_tune_of(blue_amber, green_magenta))
}

/// The fine-tuning of Sony's two values, `blue_amber` and `green_magenta`,
/// whole steps, each as a fraction over 1. The first has its sign turned
/// around, and the second is kept.
///
/// Sony stores a shift towards amber and one towards magenta as positive,
/// as frames shot on an ILCE-7M2 with a known shift show. [`FineTune`]
/// signs each value as its axis is named, so its amber to blue shift is
/// positive towards blue, and Sony's first value is turned around
/// (ADR011).
///
/// `None` when the first value is `i32::MIN`, whose negation does not fit.
/// No file checked holds it.
fn fine_tune_of(blue_amber: i32, green_magenta: i32) -> Option<FineTune> {
    let amber_blue = SignedRational::new(blue_amber.checked_neg()?, 1).ok()?;
    let green_magenta = SignedRational::new(green_magenta, 1).ok()?;
    Some(FineTune::new(amber_blue, green_magenta))
}

/// The electronic front curtain setting an ElectronicFrontCurtainShutter
/// `code` records: 0 off, `false`, and 1 on, `true`. `None` for every other
/// code, which no checked file holds.
fn electronic_front_curtain_setting(code: u32) -> Option<bool> {
    match code {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::camera_settings::{
        ColorSpace, ExposureProgram, Flash, FlashMode, MeteringMode, WhiteBalance,
    };
    use crate::container::tiff::tags::{MAKE, MAKER_NOTE};
    use crate::container::tiff::{Block, ifd_in, walk};
    use crate::error::ErrorKind;
    use crate::focus::FocusOrigin;
    use crate::formats::testing::{Src, byte_order};
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{
        ASCII, Built, IfdId, Order, RawValue, SHORT, SSHORT, TiffBuilder, UNDEFINED,
    };
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    // Tags that mean nothing here, for entries the tests read back by tag,
    // in the order an IFD sorts them in.
    const FIRST: u16 = 0x0101;
    const SECOND: u16 = 0x0102;
    const THIRD: u16 = 0x0103;
    const FOURTH: u16 = 0x0104;

    /// The Exif tag UserComment, UNDEFINED like the MakerNote: a tag that
    /// may hold any bytes and is not the MakerNote.
    const USER_COMMENT: u16 = 0x9286;

    /// The four SHORTs of the sound MakerNote's second entry, stored out of
    /// line.
    const SHORTS: [u16; 4] = [10, 20, 30, 40];

    /// A Sony file in byte order `order`, as a builder: IFD0 holds Make and
    /// points at an Exif IFD, which holds ExifVersion; and IFD0 and the
    /// Exif IFD.
    fn sony(order: Order) -> (TiffBuilder, IfdId, IfdId) {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder.ascii(ifd0, MAKE, "SONY").exif(ifd0, exif);
        builder.undefined(exif, 0x9000, b"0231");
        (builder, ifd0, exif)
    }

    /// Adds to `builder` a MakerNote IFD after `prefix`, that nothing points
    /// at, holding a SHORT under [`FIRST`], which the entry holds, and
    /// [`SHORTS`] under [`SECOND`], stored out of line; and returns it.
    fn sound_ifd(builder: &mut TiffBuilder, prefix: &[u8]) -> IfdId {
        let makernote = builder.ifd();
        builder
            .prefix(makernote, prefix)
            .short(makernote, FIRST, &[7])
            .short(makernote, SECOND, &SHORTS);
        makernote
    }

    /// A Sony file in byte order `order` whose MakerNote is a sound IFD
    /// after `prefix`; the file, and the MakerNote IFD.
    fn sound_file(order: Order, prefix: &[u8]) -> (Built, IfdId) {
        let (mut builder, _, exif) = sony(order);
        let makernote = sound_ifd(&mut builder, prefix);
        builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        (builder.build(), makernote)
    }

    /// Adds to `exif` a MakerNote of type `typ` and `count` values, whose
    /// value field holds `at`.
    fn makernote_at(builder: &mut TiffBuilder, exif: IfdId, typ: u16, count: u64, at: u64) {
        let count = u32::try_from(count).unwrap();
        let at = u32::try_from(at).unwrap();
        builder.raw(exif, MAKER_NOTE, typ, count, RawValue::Offset(at));
    }

    /// The file `bytes`, read into a source and a tree as opening a file
    /// reads it.
    fn opened(bytes: Vec<u8>) -> (Src, Tree) {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        (src, tree)
    }

    /// What [`read`] gives for the file `bytes` under `limits`, which must
    /// not be an error, and the source to read its values through.
    fn read_with(bytes: Vec<u8>, limits: &Limits) -> (Src, Option<MakerNote>) {
        let (mut src, tree) = opened(bytes);
        let makernote = read(&mut src, &tree, limits).unwrap();
        (src, makernote)
    }

    /// As [`read_with`], under the default limits.
    fn read_file(bytes: Vec<u8>) -> (Src, Option<MakerNote>) {
        read_with(bytes, &Limits::default())
    }

    /// Checks that `makernote` is the sound MakerNote of `built`, whose
    /// IFD is `ifd`, in byte order `order`, with both its entries.
    fn assert_sound(src: &mut Src, makernote: &MakerNote, built: &Built, ifd: IfdId, order: Order) {
        let found = &makernote.ifd;
        assert_eq!(found.block().bounds(), &built.ifd_block(ifd), "{order:?}");
        assert_eq!(found.order(), byte_order(order), "{order:?}");
        assert_eq!(found.offset(), built.ifd_offset(ifd), "{order:?}");
        assert_eq!(found.block().base(), 0, "{order:?}");
        assert_eq!(found.len(), 2, "{order:?}");
        assert_eq!(found.invalid_entries(), 0, "{order:?}");
        assert_eq!(found.u32(src, FIRST).unwrap(), Some(7), "{order:?}");
        assert_eq!(
            found.u32s(src, SECOND).unwrap(),
            Some(SHORTS.map(u32::from).to_vec()),
            "{order:?}"
        );
    }

    /// `start`, filled with NULs up to `len` bytes.
    fn padded(start: &[u8], len: usize) -> Vec<u8> {
        let mut bytes = start.to_vec();
        bytes.resize(len, 0);
        bytes
    }

    #[test]
    fn a_headerless_makernote_is_read_with_its_entries() {
        for order in ORDERS {
            let (built, ifd) = sound_file(order, b"");
            // The block starts with the IFD.
            assert_eq!(built.ifd_block(ifd).start, built.ifd_offset(ifd));
            let (mut src, found) = read_file(built.bytes.clone());
            let found = found.expect("a MakerNote");
            assert_sound(&mut src, &found, &built, ifd, order);
        }
    }

    #[test]
    fn each_of_sonys_headers_puts_the_ifd_12_bytes_in() {
        // The headers shorter than 12 bytes are followed by bytes that are
        // not looked at, here not NULs.
        let headers: [&[u8; 12]; 5] = [
            b"SONY DSC \0\0\0",
            b"SONY CAM \0\0\0",
            b"SONY MOBILE?",
            b"\0\0SONY PIC\0?",
            b"VHAB     \0??",
        ];
        for header in headers {
            for order in ORDERS {
                let (built, ifd) = sound_file(order, header);
                assert_eq!(
                    built.ifd_offset(ifd),
                    built.ifd_block(ifd).start + 12,
                    "{header:?}"
                );
                let (mut src, found) = read_file(built.bytes.clone());
                let found = found.expect("a MakerNote");
                assert_sound(&mut src, &found, &built, ifd, order);
            }
        }
    }

    /// The entries `makernote` adds to the raw view, alone.
    fn view_of(makernote: &MakerNote) -> Entries {
        let mut entries = Entries::new();
        makernote.add_entries(&mut entries);
        entries
    }

    /// The tags `entries` lists in the MakerNote, in order.
    fn makernote_tags(entries: &Entries) -> Vec<u16> {
        entries
            .in_directory(Directory::MakerNote)
            .iter()
            .map(crate::Entry::tag)
            .collect()
    }

    #[test]
    fn the_view_lists_the_makernote_ifd_in_every_layout() {
        // At the block's start, and 12 bytes in after each header; the
        // value stored out of line counted from the file's header.
        let layouts: [&[u8]; 6] = [
            b"",
            b"SONY DSC \0\0\0",
            b"SONY CAM \0\0\0",
            b"SONY MOBILE?",
            b"\0\0SONY PIC\0?",
            b"VHAB     \0??",
        ];
        for prefix in layouts {
            for order in ORDERS {
                let (built, _) = sound_file(order, prefix);
                let (mut src, found) = read_file(built.bytes);
                let view = view_of(&found.expect("a MakerNote"));
                let what = format!("{prefix:?}, {order:?}");
                let directories: Vec<Directory> = view.directories().to_vec();
                assert_eq!(directories, [Directory::MakerNote], "{what}");
                assert_eq!(makernote_tags(&view), [FIRST, SECOND], "{what}");
                assert_eq!(view.skipped(Directory::MakerNote), 0, "{what}");
                let second = view.entry(Directory::MakerNote, SECOND).unwrap();
                assert_eq!(
                    second.read(&mut src, 64).unwrap(),
                    crate::Value::Short(SHORTS.to_vec()),
                    "{what}"
                );
            }
        }
    }

    #[test]
    fn a_makernote_value_past_its_declared_length_is_skipped_and_counted() {
        // SHORTS, stored out of line, are the block's last 8 bytes: a
        // MakerNote declared one byte short leaves them out, as the
        // DSLR-A200 to A900 leave five of their values.
        for order in ORDERS {
            let (whole, makernote) = sound_file(order, b"");
            let len =
                u32::try_from(whole.ifd_block(makernote).end - whole.ifd_block(makernote).start)
                    .unwrap();
            for (count, skipped) in [(len - 1, 1), (len, 0)] {
                let (mut builder, _, exif) = sony(order);
                let ifd = sound_ifd(&mut builder, b"");
                builder.undefined_ifd_declaring(exif, MAKER_NOTE, ifd, count);
                let (_, found) = read_file(builder.build().bytes);
                let view = view_of(&found.expect("a MakerNote"));
                let what = format!("{order:?}, declared {count} bytes");
                assert_eq!(view.skipped(Directory::MakerNote), skipped, "{what}");
                let listed = if skipped == 0 {
                    vec![FIRST, SECOND]
                } else {
                    vec![FIRST]
                };
                assert_eq!(makernote_tags(&view), listed, "{what}");
            }
        }
    }

    #[test]
    fn the_makernotes_next_ifd_pointer_is_never_followed() {
        // A next-IFD pointer that leads to a sound IFD after the block.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let makernote = sound_ifd(&mut builder, b"");
            let next = builder.ifd();
            builder.short(next, THIRD, &[3]).short(next, FOURTH, &[4]);
            builder
                .next(makernote, Some(next))
                .undefined_ifd(exif, MAKER_NOTE, makernote);
            let (_, found) = read_file(builder.build().bytes);
            let view = view_of(&found.expect("a MakerNote"));
            assert_eq!(view.directories().len(), 1, "{order:?}");
            assert_eq!(makernote_tags(&view), [FIRST, SECOND], "{order:?}");
        }
    }

    #[test]
    fn a_block_that_holds_no_sony_directory_gives_none() {
        // Each start with a sound IFD right after it, inside a block long
        // enough that the start's first two bytes, read as an IFD's entry
        // count, would announce a table that fits in it: so neither a
        // header's 12 bytes nor the headerless rule may read it, and the
        // limits do not stop it.
        const LEN: u64 = 256 * 1024;
        let starts: [(&[u8], usize); 3] = [(b"SONY PI\0", 12), (b"PREMI\0", 8), (b"SONY PIC\0", 9)];
        for (start, ifd_at) in starts {
            for order in ORDERS {
                let (mut builder, _, exif) = sony(order);
                let ifd = sound_ifd(&mut builder, &padded(start, ifd_at));
                // Offsets written into entries move nothing, so a copy with
                // a stand-in one gives where the block starts.
                let mut probe = builder.clone();
                makernote_at(&mut probe, exif, UNDEFINED, LEN, 0);
                let at = probe.build().ifd_block(ifd).start;
                makernote_at(&mut builder, exif, UNDEFINED, LEN, at);
                builder.pad_to(at + LEN);
                let built = builder.build();
                assert_eq!(built.ifd_offset(ifd), at + u64::try_from(ifd_at).unwrap());

                let offset = u32::try_from(built.ifd_offset(ifd)).unwrap();
                let (mut src, found) = read_with(built.bytes, &Limits::unlimited());
                assert!(found.is_none(), "{start:?}, {order:?}");

                // The IFD after the start is sound, and so ignored on
                // purpose.
                let block = Block::new(0, at..at + LEN);
                let limits = Limits::unlimited();
                let sound = ifd_in(&mut src, &block, offset, byte_order(order), &limits);
                assert_eq!(sound.unwrap().map(|ifd| ifd.len()), Some(2), "{start:?}");
            }
        }
    }

    #[test]
    fn a_file_without_an_exif_ifd_gives_none() {
        // A sound MakerNote, but in IFD0, where no MakerNote belongs.
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            builder.ascii(ifd0, MAKE, "SONY");
            let ifd = sound_ifd(&mut builder, b"");
            builder.undefined_ifd(ifd0, MAKER_NOTE, ifd);
            assert!(read_file(builder.build().bytes).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn an_exif_ifd_without_a_makernote_gives_none() {
        // A sound block, but under another tag.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let ifd = sound_ifd(&mut builder, b"");
            builder.undefined_ifd(exif, USER_COMMENT, ifd);
            assert!(read_file(builder.build().bytes).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_that_is_not_bytes_gives_none() {
        // The bytes of a sound block, declared as text and as SHORTs.
        for order in ORDERS {
            let (built, ifd) = sound_file(order, b"");
            let block = built.ifd_block(ifd);
            let len = block.end - block.start;
            for (typ, count) in [(ASCII, len), (SHORT, len / 2)] {
                let (mut builder, _, exif) = sony(order);
                sound_ifd(&mut builder, b"");
                makernote_at(&mut builder, exif, typ, count, block.start);
                let bytes = builder.build().bytes;
                assert!(read_file(bytes).1.is_none(), "{order:?}, {typ}");
            }
        }
    }

    #[test]
    fn a_block_too_short_for_its_ifds_count_gives_none() {
        // Each block the last bytes of the file, so that a read past its end
        // would fail: a header alone, a header and one byte of a count, the
        // shortest block stored out of line, which holds an IFD's count of
        // 1 and no entry, and a header of 11 bytes alone.
        for order in ORDERS {
            let mut count_of_one = order.u16(1).to_vec();
            count_of_one.extend_from_slice(&[0; 3]);
            let blocks: [&[u8]; 4] = [
                b"SONY DSC \0\0\0",
                b"SONY DSC \0\0\0\x01",
                &count_of_one,
                b"SONY MOBILE",
            ];
            for block in blocks {
                let (mut builder, _, exif) = sony(order);
                let blob = builder.blob(block);
                builder.undefined_blob(exif, MAKER_NOTE, blob);
                let bytes = builder.build().bytes;
                assert!(read_file(bytes).1.is_none(), "{block:?}, {order:?}");
            }

            // One byte, which the entry holds: not a block at all.
            let (mut builder, _, exif) = sony(order);
            builder.undefined(exif, MAKER_NOTE, &[1]);
            assert!(read_file(builder.build().bytes).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn an_entry_table_that_runs_past_the_block_gives_none() {
        for order in ORDERS {
            // Two entries whose values the entries hold: the table ends
            // 2 + 24 bytes into the IFD. The file goes on past it.
            let (mut builder, _, exif) = sony(order);
            let ifd = builder.ifd();
            builder.short(ifd, FIRST, &[7]).long(ifd, SECOND, &[8]);
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let at = probe.build().ifd_offset(ifd);
            let table = 2 + 24;

            // The block ends one byte before the table does.
            let mut cut = builder.clone();
            makernote_at(&mut cut, exif, UNDEFINED, table - 1, at);
            assert!(read_file(cut.build().bytes).1.is_none(), "{order:?}");

            // The control: a block that ends with the table, before the
            // next-IFD pointer, gives the IFD.
            makernote_at(&mut builder, exif, UNDEFINED, table, at);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.ifd.block().bounds(), &(at..at + table), "{order:?}");
            assert_eq!(found.ifd.u32(&mut src, SECOND).unwrap(), Some(8));
        }
    }

    #[test]
    fn an_entry_whose_value_runs_past_the_block_is_dropped() {
        for order in ORDERS {
            // The sound block's last value is its four SHORTs: a block one
            // byte shorter cuts it.
            let (mut builder, _, exif) = sony(order);
            let ifd = sound_ifd(&mut builder, b"");
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let block = probe.build().ifd_block(ifd);
            let len = block.end - block.start;

            let mut cut = builder.clone();
            makernote_at(&mut cut, exif, UNDEFINED, len - 1, block.start);
            let (mut src, found) = read_file(cut.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.ifd.u32(&mut src, FIRST).unwrap(), Some(7));
            assert!(found.ifd.entry(SECOND).is_none(), "{order:?}");
            assert_eq!(found.ifd.invalid_entries(), 1, "{order:?}");

            // The control: the whole block keeps it.
            makernote_at(&mut builder, exif, UNDEFINED, len, block.start);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.ifd.len(), 2, "{order:?}");
            assert_eq!(found.ifd.invalid_entries(), 0, "{order:?}");
            assert_eq!(
                found.ifd.u32s(&mut src, SECOND).unwrap(),
                Some(SHORTS.map(u32::from).to_vec())
            );
        }
    }

    #[test]
    fn the_four_bytes_after_the_entries_do_not_matter() {
        // A third entry's four SHORTs are the four bytes after the entries
        // and the first four of the second entry's values, which follow
        // them: so those four bytes are values, as on the newer bodies.
        // Written there: a zero pointer, one to nowhere, and two SHORTs.
        for order in ORDERS {
            let two_shorts = match order {
                Order::Little => 0x0002_0001,
                Order::Big => 0x0001_0002,
            };
            for (next, first) in [
                (0, [0, 0]),
                (0xFFFF_FFFF, [0xFFFF, 0xFFFF]),
                (two_shorts, [1, 2]),
            ] {
                let (mut builder, _, exif) = sony(order);
                let ifd = sound_ifd(&mut builder, b"");
                builder.undefined_ifd(exif, MAKER_NOTE, ifd);
                builder.next_raw(ifd, next);
                let mut probe = builder.clone();
                probe.raw(ifd, THIRD, SHORT, 4, RawValue::Offset(0));
                let after = probe.build().ifd_offset(ifd) + 2 + 3 * 12;
                let after = u32::try_from(after).unwrap();
                builder.raw(ifd, THIRD, SHORT, 4, RawValue::Offset(after));
                let built = builder.build();

                let (mut src, found) = read_file(built.bytes);
                let found = found.expect("a MakerNote");
                let ifd = &found.ifd;
                assert_eq!(ifd.len(), 3, "{order:?}, {next:#X}");
                assert_eq!(ifd.invalid_entries(), 0, "{order:?}, {next:#X}");
                assert_eq!(ifd.u32(&mut src, FIRST).unwrap(), Some(7));
                assert_eq!(
                    ifd.u32s(&mut src, SECOND).unwrap(),
                    Some(SHORTS.map(u32::from).to_vec())
                );
                let [a, b] = first;
                let [c, d, ..] = SHORTS;
                assert_eq!(
                    ifd.u32s(&mut src, THIRD).unwrap(),
                    Some([a, b, c, d].map(u32::from).to_vec()),
                    "{order:?}, {next:#X}"
                );
            }
        }
    }

    #[test]
    fn unsorted_tags_are_each_found() {
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let ifd = builder.ifd();
            builder
                .unsorted(ifd)
                .short(ifd, FOURTH, &[4])
                .short(ifd, THIRD, &[3])
                .short(ifd, SECOND, &[2])
                .short(ifd, FIRST, &[1]);
            builder.undefined_ifd(exif, MAKER_NOTE, ifd);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            let tags: Vec<u16> = found.ifd.tags().collect();
            assert_eq!(tags, [FOURTH, THIRD, SECOND, FIRST], "{order:?}");
            for (tag, value) in [(FIRST, 1), (SECOND, 2), (THIRD, 3), (FOURTH, 4)] {
                assert_eq!(
                    found.ifd.u32(&mut src, tag).unwrap(),
                    Some(value),
                    "{order:?}, {tag:#06X}"
                );
            }
        }
    }

    #[test]
    fn a_makernote_ifd_with_more_entries_than_the_limit_gives_none() {
        // The sound IFD holds two entries; the walk is not limited.
        for order in ORDERS {
            let (built, _) = sound_file(order, b"");
            let limits = Limits::default().with_max_ifd_entries(2);
            assert!(read_with(built.bytes.clone(), &limits).1.is_some());
            let limits = Limits::default().with_max_ifd_entries(1);
            assert!(read_with(built.bytes, &limits).1.is_none(), "{order:?}");
        }
    }

    #[test]
    fn the_first_ifd_of_an_srf_chain_is_read_alone() {
        // An SRF's MakerNote: an IFD of one LONG under tag 0x0003, whose
        // next-IFD pointer leads to the next IFD of the chain, inside the
        // block.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let srf0 = builder.ifd();
            let srf1 = builder.ifd();
            builder.long(srf0, 0x0003, &[1234]).next(srf0, Some(srf1));
            builder.short(srf1, FIRST, &[1]).short(srf1, SECOND, &[2]);
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let probe = probe.build();
            let block = probe.ifd_block(srf0).start..probe.ifd_block(srf1).end;
            makernote_at(
                &mut builder,
                exif,
                UNDEFINED,
                block.end - block.start,
                block.start,
            );

            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(found.ifd.block().bounds(), &block, "{order:?}");
            assert_eq!(found.ifd.len(), 1, "{order:?}");
            assert_eq!(found.ifd.u32(&mut src, 0x0003).unwrap(), Some(1234));
            assert!(found.ifd.entry(FIRST).is_none(), "{order:?}");
        }
    }

    /// A Sony file in byte order `order` whose MakerNote, a sound IFD after
    /// the `SONY DSC` header, starts at `start`, an even offset: an IFD
    /// that nothing points at, before the MakerNote, holds bytes that push
    /// it there.
    fn far_file(order: Order, start: u64) -> (Built, IfdId) {
        far_file_with(order, start, |builder| {
            sound_ifd(builder, b"SONY DSC \0\0\0")
        })
    }

    /// As [`far_file`], with the MakerNote IFD that `makernote` adds to the
    /// builder and returns, after its prefix, if it has one.
    fn far_file_with(
        order: Order,
        start: u64,
        makernote: impl FnOnce(&mut TiffBuilder) -> IfdId,
    ) -> (Built, IfdId) {
        let filler_entry = |builder: &mut TiffBuilder, filler: IfdId, len: u64| {
            let len = usize::try_from(len).unwrap();
            builder.undefined(filler, FIRST, &vec![0; len]);
        };
        let (mut builder, _, exif) = sony(order);
        let filler = builder.ifd();
        let ifd = makernote(&mut builder);
        builder.undefined_ifd(exif, MAKER_NOTE, ifd);
        let mut probe = builder.clone();
        filler_entry(&mut probe, filler, 8);
        // The filler's bytes start after its one entry, at an even offset,
        // and the MakerNote's block right after them.
        let bytes_at = probe.build().ifd_offset(filler) + 2 + 12 + 4;
        filler_entry(&mut builder, filler, start - bytes_at);
        let built = builder.build();
        assert_eq!(built.ifd_block(ifd).start, start);
        (built, ifd)
    }

    #[test]
    fn a_failure_of_the_reader_is_an_error() {
        // The block at FAR and past it, so that reading its header fails;
        // and its header just before FAR, its IFD at FAR, so that reading
        // the IFD fails.
        for start in [FAR, 2 * FAR, FAR - 12] {
            let order = Order::Little;
            let (built, ifd) = far_file(order, start);
            // The file is sound, for a reader that does not fail.
            let (mut src, found) = read_file(built.bytes.clone());
            assert_sound(&mut src, &found.expect("a MakerNote"), &built, ifd, order);

            let mut src = Source::new(FailsFar(Cursor::new(built.bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = read(&mut src, &tree, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{start}: {err:?}");
        }
    }

    /// What a test adds to a MakerNote IFD, one of several cases.
    type Tags<'a> = &'a dyn Fn(&mut TiffBuilder, IfdId);

    /// The frame that the ILCE-7M4's FocusLocation records, its width and
    /// height (public sample 6929).
    const FRAME_7M4: [u16; 2] = [7008, 4672];

    /// The center that the ILCE-7M4's FocusLocation records in
    /// [`FRAME_7M4`], its x and y (sample 6929).
    const CENTER_7M4: [u16; 2] = [3832, 2102];

    /// The area that the ILCE-7M4's FocusFrameSize records, its width and
    /// height (sample 6929).
    const AREA_7M4: [u16; 2] = [350, 351];

    /// FocusFrameSize's six bytes for an area of `width` by `height`, then
    /// `third`, as 16-bit numbers in byte order `order`.
    fn frame_size(order: Order, [width, height]: [u16; 2], third: u16) -> Vec<u8> {
        [order.u16(width), order.u16(height), order.u16(third)].concat()
    }

    /// FocusLocation's four SHORTs for a frame `frame` and a center
    /// `center`.
    fn location_of([width, height]: [u16; 2], [x, y]: [u16; 2]) -> [u16; 4] {
        [width, height, x, y]
    }

    /// Adds to `makernote`, in a file of byte order `order`, the ILCE-7M4's
    /// tags as the sample 6929 has them: FocusMode 3 (AF-C), FocusLocation,
    /// FocusFrameSize with 257 as its third number, FileFormat and
    /// SonyModelID.
    fn tags_7m4(builder: &mut TiffBuilder, makernote: IfdId, order: Order) {
        tags_7m4_with(builder, makernote, order, Some(3), AREA_7M4);
    }

    /// As [`tags_7m4`], with FocusMode `mode`, or none, and an area of
    /// `area` in FocusFrameSize. An IFD keeps every entry it is given, so a
    /// test that wants another value of a tag writes it here, never a
    /// second entry of the same tag.
    fn tags_7m4_with(
        builder: &mut TiffBuilder,
        makernote: IfdId,
        order: Order,
        mode: Option<u8>,
        area: [u16; 2],
    ) {
        if let Some(mode) = mode {
            builder.byte(makernote, FOCUS_MODE, &[mode]);
        }
        builder
            .short(
                makernote,
                FOCUS_LOCATION,
                &location_of(FRAME_7M4, CENTER_7M4),
            )
            .undefined(makernote, FOCUS_FRAME_SIZE, &frame_size(order, area, 257))
            // FileFormat and SonyModelID, which are not read.
            .byte(makernote, 0xB000, &[4, 0, 1, 0])
            .short(makernote, 0xB001, &[388]);
    }

    /// A Sony file in byte order `order` whose MakerNote, after `prefix`,
    /// holds what `tags` adds to its IFD: the file, read into a source, and
    /// its MakerNote, which must be one.
    fn focus_file(
        order: Order,
        prefix: &[u8],
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> (Src, MakerNote) {
        let (mut builder, _, exif) = sony(order);
        let makernote = builder.ifd();
        builder.prefix(makernote, prefix);
        tags(&mut builder, makernote);
        builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        let (src, found) = read_file(builder.build().bytes);
        (src, found.expect("a MakerNote"))
    }

    /// What [`focus`] gives for a Sony file in byte order `order` whose
    /// headerless MakerNote holds what `tags` adds to its IFD.
    fn focus_of(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> Option<Focus> {
        let (mut src, makernote) = focus_file(order, b"", tags);
        focus(&mut src, &makernote).unwrap()
    }

    /// The focus centered at `center` in `frame`, with an area of `area`
    /// when there is one, and nothing said about whether focus was
    /// confirmed.
    fn expected(frame: [u16; 2], center: [u16; 2], area: Option<[u16; 2]>) -> Focus {
        let [width, height] = frame.map(u32::from);
        let [x, y] = center.map(u32::from);
        let focus = Focus::new(
            Size { width, height },
            Point { x, y },
            FocusOrigin::Recorded,
        )
        .expect("a usable center");
        match area.map(|area| area.map(u32::from)) {
            Some([width, height]) => focus
                .with_size(Size { width, height })
                .expect("a usable area"),
            None => focus,
        }
    }

    /// The ILCE-7M4's focus point: [`FRAME_7M4`], [`CENTER_7M4`] and
    /// [`AREA_7M4`].
    fn focus_7m4() -> Focus {
        expected(FRAME_7M4, CENTER_7M4, Some(AREA_7M4))
    }

    #[test]
    fn the_7m4s_tags_give_its_focus_point() {
        // Headerless, as on the ILCE-7M4, and after the A100's header, as
        // no body with FocusLocation writes it.
        for order in ORDERS {
            for prefix in [&b""[..], b"SONY DSC \0\0\0"] {
                let (mut src, makernote) = focus_file(order, prefix, |builder, ifd| {
                    tags_7m4(builder, ifd, order);
                });
                let found = focus(&mut src, &makernote).unwrap();
                let found = found.unwrap_or_else(|| panic!("{order:?}, {prefix:?}"));
                let label = format!("{order:?}, {prefix:?}");
                let size = |width, height| Size { width, height };
                assert_eq!(found.frame(), size(7008, 4672), "{label}");
                assert_eq!(found.center(), Point { x: 3832, y: 2102 }, "{label}");
                assert_eq!(found.size(), Some(size(350, 351)), "{label}");
                assert_eq!(found.in_focus(), None, "{label}");
            }
        }
    }

    /// Where the tests of a TIFF structure inside a larger file place its
    /// header, as a JPEG file holds the structure of its Exif segment.
    const POS: u64 = 1000;

    #[test]
    fn a_makernote_inside_a_file_counts_from_its_structures_header() {
        // The Sony file's structure at POS in a larger file, walked with its
        // header as the base and its bytes as the bounds. Counted from the
        // start of the file, the offsets of the values stored out of line,
        // FocusLocation's among them, would lie before the block.
        for order in ORDERS {
            for prefix in [&b""[..], b"SONY DSC \0\0\0"] {
                let (mut builder, _, exif) = sony(order);
                let ifd = builder.ifd();
                builder.prefix(ifd, prefix);
                tags_7m4(&mut builder, ifd, order);
                builder.undefined_ifd(exif, MAKER_NOTE, ifd);
                let built = builder.build();
                let mut bytes = vec![0; usize::try_from(POS).unwrap()];
                bytes.extend_from_slice(&built.bytes);

                let mut src = Source::new(Cursor::new(bytes)).unwrap();
                let block = Block::new(POS, POS..src.len());
                let tree = walk(&mut src, &block, &Limits::default()).unwrap();
                let makernote = read(&mut src, &tree, &Limits::default())
                    .unwrap()
                    .unwrap_or_else(|| panic!("a MakerNote ({order:?}, {prefix:?})"));
                let label = format!("{order:?}, {prefix:?}");
                let found = &makernote.ifd;
                assert_eq!(found.offset(), POS + built.ifd_offset(ifd), "{label}");
                assert_eq!(found.block().base(), POS, "{label}");
                let range = built.ifd_block(ifd);
                assert_eq!(
                    found.block().bounds(),
                    &(POS + range.start..POS + range.end),
                    "{label}"
                );
                assert_eq!(found.invalid_entries(), 0, "{label}");
                let focus = focus(&mut src, &makernote).unwrap();
                assert_eq!(focus, Some(focus_7m4()), "{label}");
            }
        }
    }

    #[test]
    fn focus_mode_0_gives_none() {
        // As the ILCE-9M3 writes a manual-focus shot: the frame's exact
        // center and an area of zeros (public sample 7361).
        for order in ORDERS {
            let manual = focus_of(order, |builder, ifd| {
                builder
                    .byte(ifd, FOCUS_MODE, &[0])
                    .short(
                        ifd,
                        FOCUS_LOCATION,
                        &location_of([6000, 4000], [3000, 2000]),
                    )
                    .undefined(ifd, FOCUS_FRAME_SIZE, &[0; 6]);
            });
            assert_eq!(manual, None, "{order:?}");
            // With the ILCE-7M4's location and area too.
            let manual = focus_of(order, |builder, ifd| {
                tags_7m4_with(builder, ifd, order, Some(0), AREA_7M4);
            });
            assert_eq!(manual, None, "{order:?}");
        }
    }

    #[test]
    fn every_other_focus_mode_or_none_keeps_the_point() {
        // AF-S, AF-C, AF-A, DMF and AF-D, and values no mode has.
        for order in ORDERS {
            for mode in [2, 3, 4, 6, 7, 1, 5, 255] {
                let found = focus_of(order, |builder, ifd| {
                    tags_7m4_with(builder, ifd, order, Some(mode), AREA_7M4);
                });
                assert_eq!(found, Some(focus_7m4()), "{order:?}, {mode}");
            }
            let found = focus_of(order, |builder, ifd| {
                tags_7m4_with(builder, ifd, order, None, AREA_7M4);
            });
            assert_eq!(found, Some(focus_7m4()), "{order:?}, no FocusMode");
        }
    }

    #[test]
    fn a_focus_mode_of_another_shape_is_not_read() {
        // A 0 that is not one BYTE says nothing of manual focus: a SHORT,
        // two BYTEs, and one UNDEFINED byte.
        for order in ORDERS {
            for (what, mode) in [
                (
                    "a SHORT",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.short(ifd, FOCUS_MODE, &[0]);
                    }) as Tags,
                ),
                ("two BYTEs", &|builder, ifd| {
                    builder.byte(ifd, FOCUS_MODE, &[0, 0]);
                }),
                ("UNDEFINED", &|builder, ifd| {
                    builder.undefined(ifd, FOCUS_MODE, &[0]);
                }),
            ] {
                let found = focus_of(order, |builder, ifd| {
                    tags_7m4_with(builder, ifd, order, None, AREA_7M4);
                    mode(builder, ifd);
                });
                assert_eq!(found, Some(focus_7m4()), "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn a_focus_location_of_another_shape_gives_none() {
        // Three values, five, and four LONGs, with no FocusLocation2.
        for order in ORDERS {
            let [width, height, x, y] = location_of(FRAME_7M4, CENTER_7M4);
            for (what, location) in [
                (
                    "three SHORTs",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.short(ifd, FOCUS_LOCATION, &[width, height, x]);
                    }) as Tags,
                ),
                ("five SHORTs", &|builder, ifd| {
                    builder.short(ifd, FOCUS_LOCATION, &[width, height, x, y, 0]);
                }),
                ("four LONGs", &|builder, ifd| {
                    let longs = [width, height, x, y].map(u32::from);
                    builder.long(ifd, FOCUS_LOCATION, &longs);
                }),
            ] {
                let found = focus_of(order, |builder, ifd| {
                    builder.byte(ifd, FOCUS_MODE, &[3]);
                    location(builder, ifd);
                });
                assert_eq!(found, None, "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn without_focus_location_focus_location2_gives_the_point() {
        for order in ORDERS {
            let found = focus_of(order, |builder, ifd| {
                builder
                    .byte(ifd, FOCUS_MODE, &[3])
                    .short(ifd, FOCUS_LOCATION2, &location_of(FRAME_7M4, CENTER_7M4))
                    .undefined(ifd, FOCUS_FRAME_SIZE, &frame_size(order, AREA_7M4, 257));
            });
            assert_eq!(found, Some(focus_7m4()), "{order:?}");
        }
    }

    #[test]
    fn focus_location_wins_over_focus_location2() {
        for order in ORDERS {
            let found = focus_of(order, |builder, ifd| {
                tags_7m4(builder, ifd, order);
                builder.short(ifd, FOCUS_LOCATION2, &location_of(FRAME_7M4, [3504, 2336]));
            });
            assert_eq!(found, Some(focus_7m4()), "{order:?}");
        }
    }

    #[test]
    fn a_focus_location_that_cannot_be_used_does_not_give_way_to_focus_location2() {
        // FocusLocation of the right shape but with a zero frame is the
        // location, refused as a whole: a sound FocusLocation2 beside it
        // does not stand in.
        for order in ORDERS {
            let found = focus_of(order, |builder, ifd| {
                builder
                    .short(ifd, FOCUS_LOCATION, &location_of([0, 4672], [0, 0]))
                    .short(ifd, FOCUS_LOCATION2, &location_of(FRAME_7M4, CENTER_7M4));
            });
            assert_eq!(found, None, "{order:?}");
        }
    }

    #[test]
    fn a_focus_location_of_another_shape_gives_way_to_focus_location2() {
        // FocusLocation read as missing: three SHORTs, and four LONGs.
        for order in ORDERS {
            let [width, height, x, y] = location_of(FRAME_7M4, [3504, 2336]);
            for (what, location) in [
                (
                    "three SHORTs",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.short(ifd, FOCUS_LOCATION, &[width, height, x]);
                    }) as Tags,
                ),
                ("four LONGs", &|builder, ifd| {
                    let longs = [width, height, x, y].map(u32::from);
                    builder.long(ifd, FOCUS_LOCATION, &longs);
                }),
            ] {
                let found = focus_of(order, |builder, ifd| {
                    builder
                        .byte(ifd, FOCUS_MODE, &[3])
                        .short(ifd, FOCUS_LOCATION2, &location_of(FRAME_7M4, CENTER_7M4))
                        .undefined(ifd, FOCUS_FRAME_SIZE, &frame_size(order, AREA_7M4, 257));
                    location(builder, ifd);
                });
                assert_eq!(found, Some(focus_7m4()), "{order:?}, {what}");
            }
        }
    }

    /// What [`focus`] gives for a Sony file in byte order `order` whose
    /// MakerNote holds FocusMode 3 and a FocusLocation of `frame` and
    /// `center`, and no FocusFrameSize, so that no area is checked.
    fn location_focus(order: Order, frame: [u16; 2], center: [u16; 2]) -> Option<Focus> {
        focus_of(order, |builder, ifd| {
            builder.byte(ifd, FOCUS_MODE, &[3]).short(
                ifd,
                FOCUS_LOCATION,
                &location_of(frame, center),
            );
        })
    }

    #[test]
    fn a_frame_with_a_zero_side_gives_none() {
        for order in ORDERS {
            for frame in [[0, 4672], [7008, 0], [0, 0]] {
                let found = location_focus(order, frame, [0, 0]);
                assert_eq!(found, None, "{order:?}, {frame:?}");
            }
        }
    }

    #[test]
    fn a_center_on_the_far_edge_is_kept_and_one_past_it_gives_none() {
        for order in ORDERS {
            for center in [[7008, 2102], [3832, 4672], [7008, 4672], [0, 0]] {
                assert_eq!(
                    location_focus(order, FRAME_7M4, center),
                    Some(expected(FRAME_7M4, center, None)),
                    "{order:?}, {center:?}"
                );
            }
            for center in [[7009, 2102], [3832, 4673]] {
                let found = location_focus(order, FRAME_7M4, center);
                assert_eq!(found, None, "{order:?}, {center:?}");
            }
        }
    }

    #[test]
    fn a_focus_frame_size_that_gives_no_area_keeps_the_point_without_one() {
        // Missing; all zeros, as on the ILCE-9M3's manual-focus files; a
        // zero side, either one; four or eight bytes; and three SHORTs or
        // six BYTEs, holding the area as numbers or as bytes.
        for order in ORDERS {
            let [width, height] = AREA_7M4;
            let sound = frame_size(order, AREA_7M4, 257);
            let cases: [(&str, Tags); 8] = [
                ("missing", &|_, _| {}),
                ("all zeros", &|builder, ifd| {
                    builder.undefined(ifd, FOCUS_FRAME_SIZE, &[0; 6]);
                }),
                ("a zero width", &|builder, ifd| {
                    let value = frame_size(order, [0, height], 257);
                    builder.undefined(ifd, FOCUS_FRAME_SIZE, &value);
                }),
                ("a zero height", &|builder, ifd| {
                    let value = frame_size(order, [width, 0], 257);
                    builder.undefined(ifd, FOCUS_FRAME_SIZE, &value);
                }),
                ("four bytes", &|builder, ifd| {
                    builder.undefined(ifd, FOCUS_FRAME_SIZE, &sound[..4]);
                }),
                ("eight bytes", &|builder, ifd| {
                    let value = [&sound[..], &[0, 0]].concat();
                    builder.undefined(ifd, FOCUS_FRAME_SIZE, &value);
                }),
                ("three SHORTs", &|builder, ifd| {
                    builder.short(ifd, FOCUS_FRAME_SIZE, &[width, height, 257]);
                }),
                ("six BYTEs", &|builder, ifd| {
                    builder.byte(ifd, FOCUS_FRAME_SIZE, &sound);
                }),
            ];
            for (what, frame_size) in cases {
                let found = focus_of(order, |builder, ifd| {
                    builder.byte(ifd, FOCUS_MODE, &[3]).short(
                        ifd,
                        FOCUS_LOCATION,
                        &location_of(FRAME_7M4, CENTER_7M4),
                    );
                    frame_size(builder, ifd);
                });
                assert_eq!(
                    found,
                    Some(expected(FRAME_7M4, CENTER_7M4, None)),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn an_area_that_does_not_fit_around_the_center_gives_none() {
        // At (3832, 2102) in 7008 by 4672, an area fits up to twice the
        // distance to the nearer edge: 2 × (7008 - 3832) = 6352 wide and
        // 2 × 2102 = 4204 high.
        for order in ORDERS {
            let with_area = |area: [u16; 2]| {
                focus_of(order, |builder, ifd| {
                    tags_7m4_with(builder, ifd, order, Some(3), area);
                })
            };
            for area in [[6352, 351], [350, 4204], [6352, 4204]] {
                assert_eq!(
                    with_area(area),
                    Some(expected(FRAME_7M4, CENTER_7M4, Some(area))),
                    "{order:?}, {area:?}"
                );
            }
            for area in [[6353, 351], [350, 4205]] {
                assert_eq!(with_area(area), None, "{order:?}, {area:?}");
            }
        }
    }

    #[test]
    fn focus_frame_sizes_numbers_are_read_in_the_files_byte_order() {
        // The same area gives other bytes in each order: 5E 01 5F 01 01 01
        // little-endian, 01 5E 01 5F 01 01 big-endian.
        let mut stored = Vec::new();
        for order in ORDERS {
            let (mut src, makernote) = focus_file(order, b"", |builder, ifd| {
                tags_7m4(builder, ifd, order);
            });
            let bytes = makernote
                .ifd
                .get(&mut src, rules::FOCUS_FRAME_SIZE)
                .unwrap();
            stored.push(bytes.expect("FocusFrameSize").to_vec());
            let found = focus(&mut src, &makernote).unwrap();
            assert_eq!(found, Some(focus_7m4()), "{order:?}");
        }
        assert_eq!(
            stored,
            [
                vec![0x5E, 0x01, 0x5F, 0x01, 0x01, 0x01],
                vec![0x01, 0x5E, 0x01, 0x5F, 0x01, 0x01]
            ]
        );
    }

    #[test]
    fn a_focus_location_whose_value_runs_past_the_block_gives_none() {
        // FocusLocation's eight bytes are the block's last: a block one
        // byte shorter drops the entry.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let ifd = builder.ifd();
            builder.byte(ifd, FOCUS_MODE, &[3]).short(
                ifd,
                FOCUS_LOCATION,
                &location_of(FRAME_7M4, CENTER_7M4),
            );
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let block = probe.build().ifd_block(ifd);
            let len = block.end - block.start;

            let mut cut = builder.clone();
            makernote_at(&mut cut, exif, UNDEFINED, len - 1, block.start);
            let (mut src, found) = read_file(cut.build().bytes);
            let found = found.expect("a MakerNote");
            assert!(found.ifd.entry(FOCUS_LOCATION).is_none(), "{order:?}");
            assert_eq!(focus(&mut src, &found).unwrap(), None, "{order:?}");

            // The control: the whole block gives the point.
            makernote_at(&mut builder, exif, UNDEFINED, len, block.start);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            assert_eq!(
                focus(&mut src, &found).unwrap(),
                Some(expected(FRAME_7M4, CENTER_7M4, None)),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_failure_of_the_reader_while_reading_the_focus_is_an_error() {
        // A headerless MakerNote of two entries whose table ends just
        // before FAR, so that the MakerNote is read, and FocusLocation's
        // value, stored out of line, starts at FAR, so that reading it
        // fails.
        let order = Order::Little;
        let start = FAR - (2 + 2 * 12 + 4);
        let (built, ifd) = far_file_with(order, start, |builder| {
            let ifd = builder.ifd();
            builder.byte(ifd, FOCUS_MODE, &[3]).short(
                ifd,
                FOCUS_LOCATION,
                &location_of(FRAME_7M4, CENTER_7M4),
            );
            ifd
        });
        assert_eq!(built.ifd_block(ifd), start..FAR + 8);
        // The file is sound, for a reader that does not fail.
        let (mut src, found) = read_file(built.bytes.clone());
        let found = found.expect("a MakerNote");
        assert_eq!(
            focus(&mut src, &found).unwrap(),
            Some(expected(FRAME_7M4, CENTER_7M4, None))
        );

        let limits = Limits::default();
        let mut src = Source::new(FailsFar(Cursor::new(built.bytes))).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &limits).unwrap();
        let makernote = read(&mut src, &tree, &limits)
            .unwrap()
            .expect("a MakerNote");
        let err = focus(&mut src, &makernote).expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }

    // The camera settings the MakerNote records. Each test starts from the
    // settings the standard tags gave, empty unless it says so, so that a
    // value it reads comes from the MakerNote.

    /// The Sony MakerNote tag that ExifTool's documentation also calls
    /// WhiteBalance (0xB054): one SHORT, which says auto on files set to
    /// shade, and is never read.
    const SECOND_WHITE_BALANCE: u16 = 0xB054;

    /// The Sony MakerNote tag that ExifTool's documentation calls
    /// WhiteBalanceFineTune (0x0112): one LONG, which holds the kind of
    /// fluorescent lamp, not the fine-tuning, and is never read.
    const FLUORESCENT_LAMP: u16 = 0x0112;

    /// The Sony MakerNote tag that records the kind of raw and the JPEG
    /// quality (Quality, 0x202E): two SHORTs, whose first value gives an
    /// APS-C raw the code of a medium one, and which is never read.
    const QUALITY: u16 = 0x202E;

    /// The settings that the standard tags of a file set to daylight might
    /// give: Exif's daylight white balance, aperture priority,
    /// multi-segment metering, a flash that did not fire, set off, and
    /// sRGB.
    fn exif_settings() -> CameraSettings {
        CameraSettings::default()
            .with_white_balance(WhiteBalance::default().with_mode(WhiteBalanceMode::Daylight))
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(Flash::new(false).with_mode(FlashMode::Off))
            .with_color_space(ColorSpace::Srgb)
    }

    /// What [`camera_settings`] gives, from `settings`, for a Sony file in
    /// byte order `order` whose headerless MakerNote holds what `tags` adds
    /// to its IFD.
    fn settings_from(
        order: Order,
        settings: CameraSettings,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> CameraSettings {
        let (mut src, makernote) = focus_file(order, b"", tags);
        camera_settings(&mut src, &makernote, settings).unwrap()
    }

    /// As [`settings_from`], from the empty settings.
    fn settings_of(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> CameraSettings {
        settings_from(order, CameraSettings::default(), tags)
    }

    /// The fine-tuning of `amber_blue` and `green_magenta` whole steps,
    /// each over 1, as Sony's are read.
    fn steps(amber_blue: i32, green_magenta: i32) -> FineTune {
        let step = |n| SignedRational::new(n, 1).expect("a denominator of 1");
        FineTune::new(step(amber_blue), step(green_magenta))
    }

    #[test]
    fn each_white_balance_code_a_file_shows_gives_its_mode() {
        // As the frames shot on the ILCE-7M2 record each setting.
        let codes = [
            (0, WhiteBalanceMode::Auto),
            (1, WhiteBalanceMode::ColorTemperature),
            (16, WhiteBalanceMode::Daylight),
            (32, WhiteBalanceMode::Cloudy),
            (48, WhiteBalanceMode::Shade),
            (64, WhiteBalanceMode::Tungsten),
            (80, WhiteBalanceMode::Flash),
            (96, WhiteBalanceMode::Fluorescent),
            (112, WhiteBalanceMode::Custom),
            (128, WhiteBalanceMode::Underwater),
        ];
        for order in ORDERS {
            for (code, mode) in codes {
                let settings = settings_of(order, |builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[code]);
                });
                assert_eq!(settings.white_balance.mode, Some(mode), "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn the_makernotes_white_balance_mode_takes_the_place_of_exifs() {
        // Exif calls Sony's underwater setting auto; and a MakerNote's auto
        // stands too.
        for order in ORDERS {
            for (exif, code, mode) in [
                (WhiteBalanceMode::Auto, 128, WhiteBalanceMode::Underwater),
                (WhiteBalanceMode::Daylight, 0, WhiteBalanceMode::Auto),
                (WhiteBalanceMode::Daylight, 112, WhiteBalanceMode::Custom),
            ] {
                let exif_settings = CameraSettings::default()
                    .with_white_balance(WhiteBalance::default().with_mode(exif));
                let settings = settings_from(order, exif_settings, |builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[code]);
                });
                assert_eq!(settings.white_balance.mode, Some(mode), "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn a_white_balance_code_no_file_shows_leaves_exifs_mode() {
        // Codes between those named, past them, and the largest.
        for order in ORDERS {
            for code in [2, 15, 17, 113, 144, u32::MAX] {
                let settings = settings_from(order, exif_settings(), |builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[code]);
                });
                assert_eq!(settings, exif_settings(), "{order:?}, {code}");
                let settings = settings_of(order, |builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[code]);
                });
                assert_eq!(settings.white_balance.mode, None, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn a_white_balance_of_another_shape_leaves_exifs_mode() {
        // Shade as a SHORT, as an SLONG, and as two LONGs; and no tag.
        for order in ORDERS {
            for (what, white_balance) in [
                (
                    "a SHORT",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.short(ifd, WHITE_BALANCE, &[48]);
                    }) as Tags,
                ),
                ("an SLONG", &|builder, ifd| {
                    builder.slong(ifd, WHITE_BALANCE, &[48]);
                }),
                ("two LONGs", &|builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[48, 48]);
                }),
                // A MakerNote IFD of no entries is none, so a tag that
                // means nothing here.
                ("missing", &|builder, ifd| {
                    builder.short(ifd, FIRST, &[48]);
                }),
            ] {
                let settings = settings_from(order, exif_settings(), white_balance);
                assert_eq!(settings, exif_settings(), "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn the_color_temperature_is_read_under_any_white_balance_setting() {
        // 3200 under a chosen color temperature, as on the ILCE-7M2's
        // later frames; 5500 and 6300 under custom, as on its custom
        // frames; and a value above 0 under auto, shade, a code no file
        // shows, or with no white balance tag at all, which no file
        // checked holds but the rule reads all the same.
        for order in ORDERS {
            for (code, temperature) in [
                (Some(1), 3200),
                (Some(112), 5500),
                (Some(112), 6300),
                (Some(0), 4000),
                (Some(48), 7500),
                (Some(2), 3200),
                (None, 3200),
            ] {
                let settings = settings_of(order, |builder, ifd| {
                    if let Some(code) = code {
                        builder.long(ifd, WHITE_BALANCE, &[code]);
                    }
                    builder.long(ifd, COLOR_TEMPERATURE, &[temperature]);
                });
                let found = settings.white_balance.color_temperature;
                assert_eq!(
                    found,
                    Some(temperature),
                    "{order:?}, {code:?}, {temperature}"
                );
            }
        }
    }

    #[test]
    fn a_color_temperature_of_zero_or_of_another_shape_gives_none() {
        // Under a chosen color temperature: 0, which records none, as
        // every other setting holds on every file checked; a SHORT; an
        // SLONG; and two LONGs.
        for order in ORDERS {
            for (what, tag) in [
                (
                    "zero",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.long(ifd, COLOR_TEMPERATURE, &[0]);
                    }) as Tags,
                ),
                ("a SHORT", &|builder, ifd| {
                    builder.short(ifd, COLOR_TEMPERATURE, &[3200]);
                }),
                ("an SLONG", &|builder, ifd| {
                    builder.slong(ifd, COLOR_TEMPERATURE, &[3200]);
                }),
                ("two LONGs", &|builder, ifd| {
                    builder.long(ifd, COLOR_TEMPERATURE, &[3200, 3200]);
                }),
            ] {
                let settings = settings_of(order, |builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[1]);
                    tag(builder, ifd);
                });
                assert_eq!(
                    settings.white_balance.color_temperature, None,
                    "{order:?}, {what}"
                );
                let mode = settings.white_balance.mode;
                assert_eq!(
                    mode,
                    Some(WhiteBalanceMode::ColorTemperature),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_color_temperature_past_99_999_kelvin_gives_none() {
        // 99,999 K, the bound, is given; past it, numbers no camera is set
        // to, up to the largest LONG, give none.
        for order in ORDERS {
            for (temperature, found) in [(99_999, Some(99_999)), (100_000, None), (u32::MAX, None)]
            {
                let settings = settings_of(order, |builder, ifd| {
                    builder.long(ifd, WHITE_BALANCE, &[1]).long(
                        ifd,
                        COLOR_TEMPERATURE,
                        &[temperature],
                    );
                });
                assert_eq!(
                    settings.white_balance.color_temperature, found,
                    "{order:?}, {temperature}"
                );
            }
        }
    }

    #[test]
    fn the_fine_tunings_first_value_is_turned_around_and_its_second_kept() {
        // As the ILCE-7M2's frames record a shift of three steps towards
        // amber, blue, green and magenta, and one along both axes.
        for order in ORDERS {
            for (stored, expected) in [
                ([3, 0], steps(-3, 0)),
                ([-3, 0], steps(3, 0)),
                ([0, -3], steps(0, -3)),
                ([0, 3], steps(0, 3)),
                ([-7, 5], steps(7, 5)),
            ] {
                let settings = settings_of(order, |builder, ifd| {
                    builder.slong(ifd, FINE_TUNE, &stored);
                });
                let fine_tune = settings.white_balance.fine_tune;
                assert_eq!(fine_tune, Some(expected), "{order:?}, {stored:?}");
            }
        }
    }

    #[test]
    fn a_recorded_zero_fine_tuning_is_a_fine_tuning() {
        // As every public sample that writes the tag records it.
        for order in ORDERS {
            let settings = settings_of(order, |builder, ifd| {
                builder.slong(ifd, FINE_TUNE, &[0, 0]);
            });
            assert_eq!(settings.white_balance.fine_tune, Some(steps(0, 0)));
        }
    }

    #[test]
    fn a_first_fine_tuning_value_that_cannot_be_turned_around_gives_none() {
        // i32::MIN has no negation; the second value, kept as stored, may
        // be anything.
        for order in ORDERS {
            let settings = settings_of(order, |builder, ifd| {
                builder.slong(ifd, FINE_TUNE, &[i32::MIN, 0]);
            });
            assert_eq!(settings.white_balance.fine_tune, None, "{order:?}");
            let settings = settings_of(order, |builder, ifd| {
                builder.slong(ifd, FINE_TUNE, &[i32::MAX, i32::MIN]);
            });
            let expected = steps(-i32::MAX, i32::MIN);
            assert_eq!(settings.white_balance.fine_tune, Some(expected));
        }
    }

    #[test]
    fn a_fine_tuning_of_another_shape_gives_none() {
        // One SLONG, three, two LONGs, two SRATIONALs and two SSHORTs; and
        // no tag, as on the bodies that write none.
        for order in ORDERS {
            let two_sshorts = [order.u16(3), order.u16(0)].concat();
            let two_sshorts: [u8; 4] = two_sshorts.try_into().unwrap();
            for (what, fine_tune) in [
                (
                    "one SLONG",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.slong(ifd, FINE_TUNE, &[3]);
                    }) as Tags,
                ),
                ("three SLONGs", &|builder, ifd| {
                    builder.slong(ifd, FINE_TUNE, &[3, 0, 0]);
                }),
                ("two LONGs", &|builder, ifd| {
                    builder.long(ifd, FINE_TUNE, &[3, 0]);
                }),
                ("two SRATIONALs", &|builder, ifd| {
                    builder.srational(ifd, FINE_TUNE, &[(3, 1), (0, 1)]);
                }),
                ("two SSHORTs", &|builder, ifd| {
                    let field = RawValue::Inline(two_sshorts);
                    builder.raw(ifd, FINE_TUNE, SSHORT, 2, field);
                }),
                ("missing", &|builder, ifd| {
                    builder.short(ifd, FIRST, &[3]);
                }),
            ] {
                let settings = settings_of(order, fine_tune);
                assert_eq!(settings.white_balance.fine_tune, None, "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn the_electronic_front_curtain_setting_is_off_at_0_and_on_at_1() {
        for order in ORDERS {
            for (code, setting) in [
                (0, Some(false)),
                (1, Some(true)),
                (2, None),
                (u32::MAX, None),
            ] {
                let settings = settings_of(order, |builder, ifd| {
                    builder.long(ifd, ELECTRONIC_FRONT_CURTAIN, &[code]);
                });
                let found = settings.electronic_front_curtain_setting;
                assert_eq!(found, setting, "{order:?}, {code}");
            }
        }
    }

    #[test]
    fn an_electronic_front_curtain_setting_of_another_shape_gives_none() {
        // On as a SHORT, a BYTE, an SLONG and two LONGs.
        for order in ORDERS {
            for (what, setting) in [
                (
                    "a SHORT",
                    &(|builder: &mut TiffBuilder, ifd| {
                        builder.short(ifd, ELECTRONIC_FRONT_CURTAIN, &[1]);
                    }) as Tags,
                ),
                ("a BYTE", &|builder, ifd| {
                    builder.byte(ifd, ELECTRONIC_FRONT_CURTAIN, &[1]);
                }),
                ("an SLONG", &|builder, ifd| {
                    builder.slong(ifd, ELECTRONIC_FRONT_CURTAIN, &[1]);
                }),
                ("two LONGs", &|builder, ifd| {
                    builder.long(ifd, ELECTRONIC_FRONT_CURTAIN, &[1, 1]);
                }),
            ] {
                let settings = settings_of(order, setting);
                let found = settings.electronic_front_curtain_setting;
                assert_eq!(found, None, "{order:?}, {what}");
            }
        }
    }

    /// Adds to `ifd` every settings tag the reader reads, as a frame shot
    /// on the ILCE-7M2 under a chosen color temperature of 3200 K might
    /// record them, shifted three steps towards amber, with the electronic
    /// front curtain on.
    fn settings_tags(builder: &mut TiffBuilder, ifd: IfdId) {
        builder
            .long(ifd, WHITE_BALANCE, &[1])
            .slong(ifd, FINE_TUNE, &[3, 0])
            .long(ifd, ELECTRONIC_FRONT_CURTAIN, &[1])
            .long(ifd, COLOR_TEMPERATURE, &[3200]);
    }

    /// What [`settings_tags`] gives, from `settings`.
    fn with_settings_tags(settings: CameraSettings) -> CameraSettings {
        let white_balance = WhiteBalance::default()
            .with_mode(WhiteBalanceMode::ColorTemperature)
            .with_color_temperature(3200)
            .with_fine_tune(steps(-3, 0));
        settings
            .with_white_balance(white_balance)
            .with_electronic_front_curtain_setting(true)
    }

    #[test]
    fn the_settings_the_makernote_does_not_record_are_the_standard_tags() {
        // The exposure program, the metering mode, the flash and the
        // color space stay as Exif gave them; and the shutter, the image
        // area and the raw image size stay `None`, whatever Quality says.
        for order in ORDERS {
            let settings = settings_from(order, exif_settings(), |builder, ifd| {
                settings_tags(builder, ifd);
                builder.short(ifd, QUALITY, &[3, 2]);
            });
            assert_eq!(settings, with_settings_tags(exif_settings()), "{order:?}");
            assert_eq!(settings.shutter, None);
            assert_eq!(settings.image_area, None);
            assert_eq!(settings.raw_image_size, None);
        }
    }

    #[test]
    fn tags_0xb054_and_0x0112_are_never_read() {
        // 0xB054 saying custom (4) and 0x0112 holding a fluorescent lamp
        // (1) change nothing, whether WhiteBalance gives a mode or not.
        for order in ORDERS {
            let unread = |builder: &mut TiffBuilder, ifd| {
                builder
                    .short(ifd, SECOND_WHITE_BALANCE, &[4])
                    .long(ifd, FLUORESCENT_LAMP, &[1]);
            };
            let settings = settings_from(order, exif_settings(), unread);
            assert_eq!(settings, exif_settings(), "{order:?}");
            let settings = settings_from(order, exif_settings(), |builder, ifd| {
                settings_tags(builder, ifd);
                unread(builder, ifd);
            });
            assert_eq!(settings, with_settings_tags(exif_settings()), "{order:?}");
        }
    }

    #[test]
    fn a_fine_tuning_whose_value_runs_past_the_block_gives_none() {
        // The fine-tuning's eight bytes are the block's last: a block one
        // byte shorter drops the entry, and the values the entries hold
        // are still read.
        for order in ORDERS {
            let (mut builder, _, exif) = sony(order);
            let ifd = builder.ifd();
            settings_tags(&mut builder, ifd);
            let mut probe = builder.clone();
            makernote_at(&mut probe, exif, UNDEFINED, 0, 0);
            let block = probe.build().ifd_block(ifd);
            let len = block.end - block.start;

            let mut cut = builder.clone();
            makernote_at(&mut cut, exif, UNDEFINED, len - 1, block.start);
            let (mut src, found) = read_file(cut.build().bytes);
            let found = found.expect("a MakerNote");
            assert!(found.ifd.entry(FINE_TUNE).is_none(), "{order:?}");
            let settings = camera_settings(&mut src, &found, exif_settings()).unwrap();
            let mut expected = with_settings_tags(exif_settings());
            expected.white_balance.fine_tune = None;
            assert_eq!(settings, expected, "{order:?}");

            // The control: the whole block gives it.
            makernote_at(&mut builder, exif, UNDEFINED, len, block.start);
            let (mut src, found) = read_file(builder.build().bytes);
            let found = found.expect("a MakerNote");
            let settings = camera_settings(&mut src, &found, exif_settings()).unwrap();
            assert_eq!(settings, with_settings_tags(exif_settings()), "{order:?}");
        }
    }

    #[test]
    fn metadata_gives_the_makernotes_settings_and_focus_point() {
        // The ILCE-7M4's focus tags beside the settings tags, read into
        // metadata whose other values stay as the standard tags gave them.
        for order in ORDERS {
            let (mut src, makernote) = focus_file(order, b"", |builder, ifd| {
                tags_7m4(builder, ifd, order);
                settings_tags(builder, ifd);
            });
            let standard = Metadata::default()
                .with_camera(Camera::default().with_recorded_make("SONY"))
                .with_camera_settings(exif_settings());
            let found = metadata(&mut src, &makernote, standard.clone()).unwrap();
            let expected = standard
                .with_camera_settings(with_settings_tags(exif_settings()))
                .with_focus(focus_7m4());
            assert_eq!(found, expected, "{order:?}");
            // The point as a list of one, its primary, recorded, with no
            // area mode.
            let points = found.focus_points().expect("a list");
            assert_eq!(points.points().len(), 1, "{order:?}");
            assert_eq!(
                points.primary(),
                points.points().first().copied(),
                "{order:?}"
            );
            assert_eq!(points.primary().unwrap().number(), None, "{order:?}");
            assert_eq!(points.origin(), FocusOrigin::Recorded, "{order:?}");
            assert_eq!(points.area_mode(), None, "{order:?}");
            assert_eq!(found.focus(), Some(focus_7m4()), "{order:?}");
        }
    }

    #[test]
    fn a_failure_of_the_reader_while_reading_the_settings_is_an_error() {
        // A headerless MakerNote of two entries whose table ends just
        // before FAR, so that the MakerNote is read, and the fine-tuning's
        // value, stored out of line, starts at FAR, so that reading it
        // fails.
        let order = Order::Little;
        let start = FAR - (2 + 2 * 12 + 4);
        let (built, ifd) = far_file_with(order, start, |builder| {
            let ifd = builder.ifd();
            builder
                .long(ifd, WHITE_BALANCE, &[0])
                .slong(ifd, FINE_TUNE, &[3, 0]);
            ifd
        });
        assert_eq!(built.ifd_block(ifd), start..FAR + 8);
        // The file is sound, for a reader that does not fail.
        let (mut src, found) = read_file(built.bytes.clone());
        let found = found.expect("a MakerNote");
        let settings = camera_settings(&mut src, &found, CameraSettings::default()).unwrap();
        assert_eq!(settings.white_balance.fine_tune, Some(steps(-3, 0)));

        let limits = Limits::default();
        let mut src = Source::new(FailsFar(Cursor::new(built.bytes))).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &limits).unwrap();
        let makernote = read(&mut src, &tree, &limits)
            .unwrap()
            .expect("a MakerNote");
        let err = camera_settings(&mut src, &makernote, CameraSettings::default())
            .expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }
}
