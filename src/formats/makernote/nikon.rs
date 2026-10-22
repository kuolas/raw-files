// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote: where it lies in the file, where it keeps its small
//! preview JPEG, and what it records of the picture that the standard tags
//! lack.
//!
//! The MakerNote is a block of Nikon's own metadata, the value of the
//! MakerNote tag of the Exif IFD. Among much else, it points at a
//! PreviewIFD, a small IFD whose JPEGInterchangeFormat and
//! JPEGInterchangeFormatLength declare a small JPEG, 640 by 424 pixels or
//! smaller. Every Nikon body since the D70 writes one, and on the oldest
//! bodies it is the file's only JPEG.
//!
//! It also records the visible image, which no standard tag of a NEF
//! records: the part of the stored raw data that a viewer shows, without
//! the border pixels around it. Its CropArea, which the D850, the D780 and
//! the Z bodies write, is that rectangle: on the Z 9, the 8256 by 5504
//! pixels that start 12 columns and 8 rows into the 8280 by 5520 raw, as
//! large as the camera's full-size preview (public samples 5146 to 5148).
//! It is four numbers, the left column, the top row, the width and the
//! height, in stored raw pixels. It gives the image size only when it
//! holds exactly four SHORTs or LONGs, its width and height are above
//! zero, and it lies inside the raw size the standard tags give; and never
//! when the raw IFD records a default crop, a standard tag, which comes
//! first (ADR011).
//!
//! Older bodies record three more values only here, in plain form: Nikon
//! has written them in the standard Exif tags only since the D780 and the
//! Z 6 (public samples 3827 and 3587). SerialNumber is the body's serial
//! number, ASCII text, seven digits on every checked body from the D300 on
//! (sample 928) but the Nikon 1 J1, which writes eight (sample 2956); the
//! D200 writes seven spaces, which are none (sample 8079). ISO holds two
//! SHORTs, of which the second is the ISO, and gives the ISO of the D100
//! and the D70, which write none in the Exif tags (samples 2996 and 2060);
//! the D700 writes four zero bytes there instead, which are none (sample
//! 2255). Lens is the lens's range, four RATIONALs in the order of the
//! Exif LensSpecification, on every checked body with interchangeable
//! lenses. Each is used only when the standard tags give no value for it:
//! an Exif value always wins, even where the MakerNote records another. A
//! lens the Exif tags name but give no range for, such as the Nikon 1 J1's
//! (sample 2956), takes the MakerNote's range and keeps its name; an Exif
//! range with any value known is kept whole.
//!
//! WorldTime is the time zone and daylight-saving setting of the camera's
//! clock, and so gives the capture time's offset from UTC on bodies whose
//! Exif tags record none. It is four bytes: the zone in minutes east of
//! UTC, a signed 16-bit number in the MakerNote's byte order; a byte that
//! is 1 when daylight saving time is on and 0 when it is off; and one for
//! how the camera shows dates, which is not read. The offset is the zone,
//! plus 60 minutes when daylight saving time is on. On every public sample
//! that also records the Exif OffsetTimeOriginal, the two agree, with
//! daylight saving time off (samples 1835, 3827, 3587 and 5146 to 5148)
//! and on (samples 6887 and 7808 to 7810). The bodies before the D850
//! write no Exif offset, and most of them from the D300 on write
//! WorldTime: the D300, D700, D7000, D750, D810, Nikon 1 J1 and Coolpix A
//! (samples 928, 2255, 961, 895, 2036, 2956 and 2238); the D1H, D100, D70,
//! D200, Coolpix E5700 and P7800 do not. WorldTime gives an offset only
//! when it is of type UNDEFINED and exactly four bytes long, its
//! daylight-saving byte is 0 or 1, and the offset lies within 14 hours of
//! UTC, the bound an Exif offset is held to: a zone that editing software
//! has rewritten in the wrong byte order reads as hundreds of hours, 15360
//! minutes for 60. It is used only for a capture time the standard tags
//! give, when they give no offset for it: OffsetTimeOriginal is missing,
//! or cannot be read. An Exif offset always wins, even where WorldTime
//! records another.
//!
//! AFInfo2 records where the camera's autofocus aimed, which no standard
//! tag records. Mirrorless bodies, and DSLRs in live view, fill in a
//! position in pixels: the size of a frame, the centre of the focus area
//! in that frame, and the area's width and height. A DSLR shot through the
//! viewfinder records a zero frame instead, and only the number of an
//! autofocus point, which only a table of the body's points, carried by no
//! file, could place in pixels (public samples 928 and 1835): it gives no
//! focus point.
//! The position is the centre of the area, not its corner: the Coolpix A,
//! focusing at the middle of its 4928 by 3264 frame, records (2464, 1632)
//! (sample 2238). The frame is the camera's full-size output, not the
//! stored raw: 8256 by 5504 on the Z 9, inside an 8280 by 5520 raw
//! (samples 5146 to 5148), and the full 6048 by 4024 on the Z 6's small
//! raw, whose stored image is 4548 by 3030 (sample 3587); the point is
//! reported in that frame, as recorded. AFInfo2 is a value of type
//! UNDEFINED that starts with a version, four ASCII digits, and each
//! version keeps its fields at offsets of its own, as ExifTool's tables
//! publish them. It is read at the exact versions 0100, 0101, 0300, 0301,
//! 0400, 0401 and 0402; any other, such as the Nikon 1's 0200, gives no
//! focus point, since its fields may lie elsewhere (ADR012). The frame,
//! the centre and the area are SHORTs in the MakerNote's byte order. On
//! every version read but 0300 and 0301, a byte says whether the camera
//! confirmed focus: 1 for yes, 0 for a miss, and any other value says
//! nothing. A focus point is given only when the value is at least as long
//! as its version's layout, the frame and the area have no zero side, the
//! centre lies on the frame, and the area fits inside the frame around it
//! ([`Focus::new`], [`Focus::with_size`]). FocusMode, ASCII text, records
//! how the camera focused: `MANUAL`, or an autofocus mode padded with
//! spaces, such as `AF-S  `. When it reads exactly `MANUAL`, by the text
//! rule, the file gives no focus point, whatever AFInfo2 holds: the point
//! of a manual-focus shot is not one the camera focused on (ADR012).
//!
//! On a crop mode, DX, 1:1 or 16:9, the point is not reported as recorded,
//! for the frame AFInfo2 records stays the camera's full output while the
//! picture is cut from it: the Z6III's 1:1 file records a 6048 by 4032
//! frame for a picture of 4032 by 4032 (public sample 7808), and its 16:9
//! and DX files and the Z50II's 1:1 and 16:9 files do the same (samples
//! 7809, 7810, 7767 and 7770). What the values mean on the cropped picture
//! depends on what the crop cut. A DX crop, which cuts both sides, records
//! fractions of the cropped picture; a crop that keeps one side, 1:1 or
//! 16:9, records positions in the full frame. Shots made with the
//! maintainer's Z 9, with the point in the top-left corner of the picture,
//! showed it, with the same single point of 291 by 323 at (502, 424) in FX.
//! The DX shot records (636, 572) and an area of 420 by 468, which, scaled
//! onto its 5392 by 3592 picture, are (415, 373) and 274 by 305, near the
//! corner and about the FX area. The 1:1 shot records (1797, 424), which is
//! (421, 424) in its 5504 by 5504 window of the frame, and the 16:9 shot
//! (502, 715), which is (502, 283) in its 8256 by 4640 window, both with
//! the FX area. CropHiSpeed tells a crop apart: its cropped size differs
//! from its original size on a crop, and equals it on every other file
//! checked but the D810's, below. The record is checked first as recorded,
//! in the full frame, as on any file: one that does not hold there gives no
//! focus point on a crop either. On a crop, the frame becomes CropArea's
//! size. A DX crop's point is scaled onto it as [`Focus::scaled_to`]
//! scales: the centre and the area are each rounded to the nearest pixel,
//! a half rounding up. An area that rounding leaves a pixel too wide for
//! its centre is trimmed to fit, and an area side that rounds to nothing
//! is kept at one pixel. A centre that rounds onto an edge of the picture
//! keeps the point without an area. A 1:1 or 16:9 crop's centre moves
//! back by the window's origin, half the difference between the frame and
//! CropArea, keeping the area. That window holds only when CropArea's
//! kept side equals the frame's: a small raw's CropArea counts stored
//! pixels, smaller than the full output, so a 1:1 or 16:9 crop on one
//! gives no focus point, while a DX crop on one is scaled onto its
//! CropArea. A file whose CropHiSpeed records a crop but that has no
//! CropArea is read as recorded: the D810 writes a cropped width four
//! pixels short of its original on an uncropped file, and no CropArea
//! (sample 2036); one whose CropArea cannot be used, or cannot be checked
//! against a raw size, gives no focus point. A CropHiSpeed that fits
//! neither shape, cutting both sides or keeping exactly one, such as one
//! with a cropped side larger than its original, gives none too.
//! CropHiSpeed is read only when it holds exactly seven SHORTs; otherwise
//! the file is read as no crop (ADR012).
//!
//! Nikon has used three layouts for the block, told apart by their first
//! bytes. The one every body since the D70 and the D100 writes, and the one
//! read here, starts with the signature `Nikon` and a NUL, two version
//! bytes and two more; then, 10 bytes in, comes a TIFF header of its own: a
//! byte-order mark, 42 and the offset of the MakerNote IFD. That inner
//! header is the base of every offset inside the block, the MakerNote
//! IFD's, the PreviewIFD's and the JPEG's, never the start of the file, and
//! its byte order, which may differ from the file's, is the order of every
//! number inside the block. So the offsets are the same wherever the block
//! lies, in a NEF file or in a JPEG file's Exif segment: the header of the
//! TIFF structure that holds the MakerNote is not their base, as it is for
//! Sony's. The two older layouts, a bare IFD with no signature and an IFD
//! after the 8 bytes `Nikon\0\1\0`, count their offsets from the file's
//! own header instead. They are not read in this release: a file that has
//! one gives no MakerNote preview.
//!
//! Nothing in the block is trusted. Every offset found inside it is checked
//! against the block, not only against the file: the MakerNote IFD and the
//! PreviewIFD must lie inside the block, with their whole entry tables; the
//! three numbers that lead to the JPEG must be held in their entries, not
//! stored elsewhere; and the whole JPEG, as declared, must lie inside the
//! block. The block's IFDs are read inside the block ([`ifd_in`]): every
//! entry whose value lies outside the block is dropped, as an entry
//! whose value lies outside the file is, so every value read from them lies
//! inside the block: the ISO's two numbers and WorldTime's four bytes,
//! which the entry holds, and CropArea's four numbers, CropHiSpeed's seven,
//! the serial number, the lens's range, FocusMode's text and AFInfo2's
//! bytes, which take more than the four bytes an entry holds and so are
//! stored elsewhere. A value of another type than its tag's rule allows is
//! not read. The readers of text and of fractions read only ASCII and only
//! RATIONAL, but those of numbers and of bytes accept more types than a
//! tag's rule allows, such as [`Ifd::u32s`], which reads type IFD too, so a
//! reader of numbers or bytes first finds the tag's entry among the types
//! its rule allows ([`Ifd::entry_of_type`]). A block that does not hold
//! together, whether its inner header is not one, an IFD in it is cut
//! short or has more entries than the limits allow, costs what the
//! MakerNote would have given, the small preview, the crop area and the
//! values above, never the file: the NEF security notes ask that bad
//! entries be skipped rather than fail the file
//! (`docs/formats/nef/security.md`, item 4 of the checklist).
//! Only a failure of the reader itself is an error: that is not the file's
//! fault, and says nothing about the MakerNote.
//!
//! The MakerNote is read once per file, into a [`MakerNote`]: its block
//! found and checked, its MakerNote IFD parsed. Every use of the MakerNote
//! then reads from that same structure, so that none of them finds, checks
//! or parses the block again.

use std::io::{Read, Seek};

use crate::container::tiff::tags::{
    JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKER_NOTE,
};
use crate::container::tiff::types::{LONG, SHORT, UNDEFINED};
use crate::container::tiff::{Block, Entry, Ifd, Tree, ifd_in, parse_header, skip_damage};
use crate::error::Result;
use crate::formats::tiff_metadata::{self, Standard};
use crate::formats::tiff_previews::{self, Candidate};
use crate::geometry::{Point, Size};
use crate::io::{ByteOrder, Source};
use crate::limits::Limits;
use crate::metadata::{Focus, Lens, Metadata};
use crate::rational::Rational;

/// The Nikon MakerNote tag that records the ISO the picture was taken at
/// (ISO): two SHORTs, the second of them the ISO, on the bodies whose Exif
/// tags may lack one.
const ISO: u16 = 0x0002;

/// The Nikon MakerNote tag that records how the camera focused
/// (FocusMode), as ASCII text: `MANUAL` for manual focus, or an autofocus
/// mode padded with spaces to six characters, such as `AF-S  `.
const FOCUS_MODE: u16 = 0x0007;

/// The Nikon MakerNote tag that points at the PreviewIFD, the sub-IFD
/// holding the small preview JPEG.
const PREVIEW_IFD: u16 = 0x0011;

/// The Nikon MakerNote tag that records the crop mode the picture was
/// taken in, and how large the picture was before and after the crop
/// (CropHiSpeed): seven SHORTs, the mode, the original width and height,
/// the cropped width and height, and the left column and top row where
/// the crop starts.
const CROP_HI_SPEED: u16 = 0x001B;

/// The Nikon MakerNote tag that records the camera body's serial number
/// (SerialNumber), as ASCII text.
const SERIAL_NUMBER: u16 = 0x001D;

/// The Nikon MakerNote tag that records the time zone and daylight-saving
/// setting of the camera's clock (WorldTime): four UNDEFINED bytes, the
/// zone in minutes east of UTC as a signed 16-bit number in the
/// MakerNote's byte order, then 1 when daylight saving time is on and 0
/// when it is off, then the date display format.
const WORLD_TIME: u16 = 0x0024;

/// The Nikon MakerNote tag that records the visible image, the rectangle
/// of the stored raw data inside its border (CropArea): its left column,
/// top row, width and height.
const CROP_AREA: u16 = 0x0045;

/// The Nikon MakerNote tag that records the lens's range (Lens): its
/// shortest and longest focal length and the f-number at each, as four
/// RATIONALs in the order of the Exif LensSpecification.
const LENS: u16 = 0x0084;

/// The Nikon MakerNote tag that records where the camera's autofocus
/// aimed (AFInfo2): an UNDEFINED value that starts with its version, four
/// ASCII digits, and holds, at offsets each version sets, the size of a
/// frame, the centre of the focus area and the area's width and height,
/// as SHORTs in the MakerNote's byte order, and on most versions a byte
/// saying whether focus was confirmed.
const AF_INFO2: u16 = 0x00B7;

/// The signature a Nikon MakerNote starts with.
const SIGNATURE: &[u8; 6] = b"Nikon\0";

/// Where the inner TIFF header sits in the block: after the signature, two
/// version bytes and two more.
const HEADER_AT: u64 = 10;

/// The shortest block of the layout read here: everything up to the end of
/// the inner TIFF header, which is 8 bytes long.
const MIN_LEN: u64 = HEADER_AT + 8;

/// A Nikon MakerNote of the layout read here, its block found and its
/// MakerNote IFD read: what every use of the MakerNote starts from, so
/// that the block is found, checked and parsed once per file.
///
/// See the [module documentation](self) for the layout.
#[derive(Debug)]
pub(in crate::formats) struct MakerNote {
    /// The MakerNote IFD, which holds Nikon's tags: the IFD the inner
    /// header points at, inside the block with its whole entry table. Its
    /// entries whose values lie outside the block were dropped when it was
    /// read, so every value read from it lies inside the block.
    ///
    /// It was read in the block, [`Ifd::block`]: its bounds are where the
    /// block's bytes are in the file, which every offset found inside the
    /// block is checked against, and its base, which every such offset
    /// counts from, is the inner TIFF header, 10 bytes into the block. Its
    /// byte order, [`Ifd::order`], is the inner header's: the order of
    /// every number inside the block, which may differ from the file's.
    ifd: Ifd,
}

/// The Nikon MakerNote of the file in `src`, whose IFDs are `tree`, read
/// within `limits`; `None` when the file has no Exif IFD, no MakerNote, a
/// MakerNote of another layout or maker, or one whose inner header or
/// MakerNote IFD does not hold together.
///
/// See the [module documentation](self) for the layout and the checks.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(in crate::formats) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    let Some(range) = tree
        .exif()
        .and_then(|exif| exif.entry(MAKER_NOTE))
        .and_then(Entry::byte_range)
    else {
        return Ok(None);
    };
    // The block's bytes lie inside the file, as every kept entry's value
    // does, so the reads below that stay inside the block can fail only
    // with the reader.
    if range
        .end
        .checked_sub(range.start)
        .is_none_or(|len| len < MIN_LEN)
        || src.bytes(range.start, 6)?.as_slice() != SIGNATURE
    {
        return Ok(None);
    }

    let Some(base) = range.start.checked_add(HEADER_AT) else {
        return Ok(None);
    };
    let block = Block::new(base, range);
    let Some(header) = skip_damage(parse_header(src, &block))? else {
        return Ok(None);
    };
    let Some(ifd) = ifd_in(src, &block, header.ifd0, header.order, limits)? else {
        return Ok(None);
    };
    Ok(Some(MakerNote { ifd }))
}

/// Where the preview JPEG of `makernote` is, as a candidate for the shared
/// finder, with its absolute offset, read within `limits`; `None` when the
/// MakerNote has no PreviewIFD, or one whose structure does not hold
/// together.
///
/// See the [module documentation](self) for the checks.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(in crate::formats) fn preview_candidate<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    limits: &Limits,
) -> Result<Option<Candidate>> {
    let ifd = &makernote.ifd;
    let block = ifd.block();
    let Some(pointer) = inline_u32(src, ifd, PREVIEW_IFD)? else {
        return Ok(None);
    };
    let Some(preview) = ifd_in(src, block, pointer, ifd.order(), limits)? else {
        return Ok(None);
    };
    let offset = inline_u32(src, &preview, JPEG_INTERCHANGE_FORMAT)?;
    let length = inline_u32(src, &preview, JPEG_INTERCHANGE_FORMAT_LENGTH)?;
    let (Some(offset), Some(length)) = (offset, length) else {
        return Ok(None);
    };

    // An empty JPEG is passed on: the finder drops it with the others.
    Ok(tiff_previews::candidate(block, offset, length))
}

/// The metadata of the file whose MakerNote is `makernote`: what the
/// standard tags gave, `standard`, with what the MakerNote records that the
/// standard tags lack, the focus point among it.
///
/// That is, first, the image size, from CropArea, which replaces the raw
/// size the image size otherwise falls back on: when the raw IFD records no
/// default crop, which comes first, and when there is a raw size for the
/// crop area to lie inside ([`crop_area`]). CropArea is read once,
/// whether or not it gives the image size, and the focus point below
/// takes what it gives too: on a crop, a CropArea that cannot be used, or
/// that there is no raw size to check against, gives no focus point.
/// Then, each only when the standard tags gave none, the serial number
/// ([`serial_number`]), the ISO ([`iso`]), the lens's range
/// ([`lens_range`]) and the capture time's offset from UTC
/// ([`world_time`]): a range fills a lens whose standard tags gave no
/// value of its range, keeping their make, model and serial number, or
/// makes a lens of the range alone when they described none; an offset is
/// given only to a capture time the standard tags gave, one whose
/// OffsetTimeOriginal is missing or could not be read, and only within
/// the bound every offset is held to
/// ([`DateTime::with_offset_minutes`](crate::datetime::DateTime::with_offset_minutes)).
/// Last, the focus point, from AFInfo2 and FocusMode, placed on the
/// cropped picture by CropHiSpeed and CropArea on a crop ([`focus`]),
/// always read: the standard tags never give one, so it is not a fallback.
/// Every other value is the standard tags'.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(in crate::formats) fn metadata<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    standard: Standard,
) -> Result<Metadata> {
    let Standard {
        mut metadata,
        from_default_crop,
    } = standard;
    // Read once, for the image size and the focus point.
    let crop = crop_area(src, makernote, metadata.raw_size)?;
    if !from_default_crop && let CropArea::Size(image) = crop {
        metadata.image_size = Some(image);
    }
    if metadata.serial_number.is_none() {
        metadata.serial_number = serial_number(src, makernote)?;
    }
    if metadata.exposure.iso.is_none() {
        metadata.exposure.iso = iso(src, makernote)?;
    }
    if !metadata.lens.as_ref().is_some_and(Lens::has_range) {
        let range = lens_range(src, makernote)?;
        metadata.lens = Lens::with_range(metadata.lens.take(), range);
    }
    // An offset belongs to a time: without a capture time, WorldTime gives
    // nothing.
    if let Some(time) = metadata.capture_time
        && time.offset_minutes().is_none()
        && let Some(offset) = world_time(src, makernote)?
        && let Some(time) = time.with_offset_minutes(offset)
    {
        metadata.capture_time = Some(time);
    }
    metadata.focus = focus(src, makernote, crop)?;
    Ok(metadata)
}

/// What `makernote`'s CropArea gives, as [`crop_area`] reads it: the size
/// of the visible image, or why there is none. The image size takes only
/// a size; on a crop, the focus point tells the other two apart
/// ([`in_crop`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CropArea {
    /// There is no CropArea tag, as on the D810 (public sample 2036).
    Absent,
    /// There is a CropArea tag, but its value cannot be used, or there is
    /// no raw size to check it against.
    Unusable,
    /// The size of the visible image that CropArea records.
    Size(Size),
}

/// The size of the visible image that `makernote`'s CropArea records, in
/// raw data of size `raw`, as [`CropArea::Size`].
///
/// [`CropArea::Absent`] when the tag is missing, or stores its value
/// outside the block, where the reading of the block drops it. Otherwise
/// [`CropArea::Unusable`] when there is no raw size, whatever the tag's
/// type, or when the tag is not of type SHORT or LONG, does not hold
/// exactly four values, or describes a rectangle that
/// [`tiff_metadata::crop`] does not accept: one of zero width or height,
/// or not inside the raw data.
///
/// See the [module documentation](self) for what CropArea is.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn crop_area<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    raw: Option<Size>,
) -> Result<CropArea> {
    let ifd = &makernote.ifd;
    if ifd.entry(CROP_AREA).is_none() {
        return Ok(CropArea::Absent);
    }
    // Without a raw size, the rectangle has nothing to lie inside.
    let Some(raw) = raw else {
        return Ok(CropArea::Unusable);
    };
    if ifd.entry_of_type(CROP_AREA, &[SHORT, LONG]).is_none() {
        return Ok(CropArea::Unusable);
    }
    let values = skip_damage(ifd.u32s(src, CROP_AREA))?.flatten();
    let Some(&[left, top, width, height]) = values.as_deref() else {
        return Ok(CropArea::Unusable);
    };
    Ok(tiff_metadata::crop([left, top], [width, height], raw)
        .map_or(CropArea::Unusable, CropArea::Size))
}

/// The camera body's serial number that `makernote`'s SerialNumber
/// records, read by the rule every text value of the metadata is read by
/// ([`tiff_metadata::text`]); `None` when the tag is missing, is not of
/// type ASCII, stores its value outside the block, or holds text that rule
/// gives none for, such as the seven spaces the D200 writes (public sample
/// 8079).
///
/// A serial number that is not all digits is returned as written.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn serial_number<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<String>> {
    tiff_metadata::text(src, &makernote.ifd, SERIAL_NUMBER)
}

/// The ISO that `makernote`'s ISO tag records: the second of its values,
/// when it holds exactly two of type SHORT and that second one is above
/// zero; `None` otherwise.
///
/// Most bodies that write the tag write `[0, ISO]`, such as `[0, 200]` on
/// the D300 (public sample 928). Any other shape records no ISO: the D700
/// writes four zero bytes of type UNDEFINED (sample 2255), the Coolpix
/// E5700 `[0, 0]` (sample 4456).
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn iso<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<Option<u32>> {
    let ifd = &makernote.ifd;
    if ifd.entry_of_type(ISO, &[SHORT]).is_none() {
        return Ok(None);
    }
    let values = skip_damage(ifd.u32s(src, ISO))?.flatten();
    let Some(&[_, iso]) = values.as_deref() else {
        return Ok(None);
    };
    Ok((iso > 0).then_some(iso))
}

/// The lens's range that `makernote`'s Lens tag records: its shortest and
/// longest focal length, then the f-number at each, in the order of the
/// Exif LensSpecification, as [`Lens::from_parts`] takes them.
///
/// The tag must be of type RATIONAL, hold exactly four values and store
/// them inside the block; otherwise all four are `None`. Each value is
/// `None` when its numerator or its denominator is zero, as in the Exif
/// tag ([`Rational::known`]).
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn lens_range<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<[Option<Rational>; 4]> {
    let values = skip_damage(makernote.ifd.rationals(src, LENS))?.flatten();
    let Some(&[first, second, third, fourth]) = values.as_deref() else {
        return Ok([None; 4]);
    };
    Ok([first, second, third, fourth]
        .map(|(numerator, denominator)| Rational::known(numerator, denominator)))
}

/// The offset of the camera's clock from UTC, in minutes east of UTC, that
/// `makernote`'s WorldTime records: its time zone, plus 60 minutes when
/// its daylight-saving byte is 1; `None` when the tag is missing, is not of
/// type UNDEFINED, does not hold exactly four bytes, or has a
/// daylight-saving byte other than 0 or 1.
///
/// The zone is read in the MakerNote's byte order. The offset is not
/// bounded here:
/// [`DateTime::with_offset_minutes`](crate::datetime::DateTime::with_offset_minutes),
/// which takes it, refuses one beyond 14 hours, as a zone rewritten in the
/// wrong byte order is.
///
/// See the [module documentation](self) for what WorldTime is.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn world_time<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<Option<i16>> {
    let ifd = &makernote.ifd;
    if ifd.entry_of_type(WORLD_TIME, &[UNDEFINED]).is_none() {
        return Ok(None);
    }
    let bytes = skip_damage(ifd.bytes(src, WORLD_TIME))?.flatten();
    let Some(&[zone0, zone1, daylight_saving, _date_format]) = bytes.as_deref() else {
        return Ok(None);
    };
    let zone = i32::from(ifd.order().u16([zone0, zone1]).cast_signed());
    let offset = match daylight_saving {
        0 => zone,
        1 => zone + 60,
        _ => return Ok(None),
    };
    // A zone near the end of the 16-bit range, plus the daylight hour, may
    // not fit; it is far beyond any offset in use anyway.
    Ok(i16::try_from(offset).ok())
}

/// Where the camera's autofocus aimed, as `makernote`'s AFInfo2 records
/// it: the centre of the focus area, the area's size and the frame the two
/// are measured in, with whether focus was confirmed on the versions that
/// say, placed on the cropped picture when the file is a crop, by
/// `crop_area`, what CropArea gives, already read ([`crop_area`]).
///
/// `None` when FocusMode records manual focus ([`manual_focus`]); when
/// AFInfo2 is missing, is not of type UNDEFINED, stores its value outside
/// the block, or holds a value [`af_info2_record`] gives none for; or when
/// [`in_crop`] gives none, because [`FocusRecord::focus`] refuses the
/// record as recorded or the record cannot be placed on the cropped
/// picture. CropHiSpeed is read only once AFInfo2 has given a record
/// ([`crop_hi_speed`]).
///
/// See the [module documentation](self) for what AFInfo2 and CropHiSpeed
/// are.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn focus<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    crop_area: CropArea,
) -> Result<Option<Focus>> {
    let ifd = &makernote.ifd;
    // Without AFInfo2 there is nothing to place, so FocusMode is not read.
    if ifd.entry_of_type(AF_INFO2, &[UNDEFINED]).is_none() {
        return Ok(None);
    }
    if manual_focus(src, makernote)? {
        return Ok(None);
    }
    let value = skip_damage(ifd.bytes(src, AF_INFO2))?.flatten();
    let order = ifd.order();
    let Some(record) = value.and_then(|value| af_info2_record(&value, order)) else {
        return Ok(None);
    };
    let crop = crop_hi_speed(src, makernote)?;
    Ok(in_crop(record, crop, crop_area))
}

/// What CropHiSpeed records of a crop, as [`in_crop`] uses it: the size of
/// the picture before the crop and after it, in the sensor's pixels,
/// which [`in_crop`] only compares with each other. The mode and where the
/// crop starts are not used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CropHiSpeed {
    /// The original width and height, those of the uncropped picture.
    original: Size,
    /// The cropped width and height.
    cropped: Size,
}

/// What `makernote`'s CropHiSpeed records of the crop; `None` when the tag
/// is missing, is not of type SHORT, does not hold exactly seven values,
/// or stores them outside the block. A file without it is read as no crop.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn crop_hi_speed<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<CropHiSpeed>> {
    let ifd = &makernote.ifd;
    if ifd.entry_of_type(CROP_HI_SPEED, &[SHORT]).is_none() {
        return Ok(None);
    }
    let values = skip_damage(ifd.u32s(src, CROP_HI_SPEED))?.flatten();
    let Some(
        &[
            _mode,
            original_width,
            original_height,
            cropped_width,
            cropped_height,
            _left,
            _top,
        ],
    ) = values.as_deref()
    else {
        return Ok(None);
    };
    Ok(Some(CropHiSpeed {
        original: Size {
            width: original_width,
            height: original_height,
        },
        cropped: Size {
            width: cropped_width,
            height: cropped_height,
        },
    }))
}

/// The focus point `record` describes, placed on the cropped picture when
/// the file is a crop, and as AFInfo2 records it otherwise; `None` when
/// the record is refused as recorded, or the file is a crop on which it
/// cannot be placed.
///
/// The record is checked first, as recorded, in its own frame, by
/// [`FocusRecord::focus`], on a crop as on any other file. A centre outside
/// the recorded frame, or an area that does not fit around it there, gives
/// none, even where placing it on the cropped picture would move it inside.
///
/// A file is a crop when CropHiSpeed, `crop`, records a cropped size
/// that differs from its original size, and the file has a CropArea,
/// which `crop_area` gives: a crop without a CropArea tag, such as the
/// D810's, whose cropped width is four pixels short on an uncropped file
/// (public sample 2036), has no picture to place the record on, and is
/// read as recorded. A crop whose CropArea cannot be used, or could not be
/// checked against a raw size, has a picture whose size is not known, and
/// gives none. On a crop, AFInfo2's frame is still the camera's full
/// output, and the frame becomes CropArea's size. What happens to the
/// centre and the area depends on what the crop cut, as the module
/// documentation shows:
///
/// - **A crop that cuts both sides** (DX) records fractions of the
///   cropped picture, in the full frame's units. The point is scaled onto
///   CropArea by [`Focus::scaled_to`]: x and the width multiplied by
///   CropArea's width over the frame's, y and the height by the heights,
///   each rounded to the nearest pixel, a half rounding up. An area that
///   rounding leaves a pixel too wide for its centre is trimmed to fit,
///   and an area side that rounds to nothing keeps one pixel. A centre
///   that rounds onto an edge of CropArea leaves no room for an area, and
///   keeps the point without one.
/// - **A crop that keeps one side** (1:1, 16:9), whose cropped size
///   equals the original on exactly one side and is smaller on the other,
///   records positions in the full frame, of which the cropped picture is
///   the centred window. The centre moves back on each axis by the
///   window's origin, half the difference between the frame and CropArea,
///   rounded down, and the area is kept. That holds only when CropArea's
///   kept side equals the frame's: on a small raw, CropArea counts stored
///   pixels and the frame the full output, the window cannot be placed,
///   and the file gives none. A CropArea side larger than the frame's, or
///   a centre that would move past the window's left or top edge, gives
///   none too. The moved record is checked again by
///   [`FocusRecord::focus`], in the window: a centre past its right or
///   bottom edge, or an area that no longer fits around the moved centre,
///   gives none.
///
/// A CropHiSpeed that fits neither shape, such as one whose cropped size
/// is larger than its original on a side, records no crop the point can
/// be placed on, and gives none.
fn in_crop(record: FocusRecord, crop: Option<CropHiSpeed>, crop_area: CropArea) -> Option<Focus> {
    // `Focus::new` refuses a frame with a zero side here, so the old frame
    // whose sides `scaled_to` divides by below has no zero side.
    let focus = record.focus()?;
    let Some(CropHiSpeed { original, cropped }) = crop else {
        return Some(focus);
    };
    if cropped == original {
        return Some(focus);
    }
    let window = match crop_area {
        CropArea::Absent => return Some(focus),
        CropArea::Unusable => return None,
        CropArea::Size(window) => window,
    };
    if cropped.width < original.width && cropped.height < original.height {
        return focus.scaled_to(window);
    }
    let kept_width = cropped.width == original.width;
    let kept_height = cropped.height == original.height;
    // A window keeps exactly one side and cuts the other; the sizes differ
    // here, so they cannot both be kept.
    if !((kept_width && cropped.height < original.height)
        || (kept_height && cropped.width < original.width))
    {
        return None;
    }
    let FocusRecord { frame, center, .. } = record;
    if (kept_width && window.width != frame.width) || (kept_height && window.height != frame.height)
    {
        return None;
    }
    // `Focus` cannot move its centre, so the moved point is built anew,
    // with the area and the in-focus byte as recorded.
    FocusRecord {
        frame: window,
        center: Point {
            x: shifted(center.x, frame.width, window.width)?,
            y: shifted(center.y, frame.height, window.height)?,
        },
        ..record
    }
    .focus()
}

/// `value`, a position on an axis `frame` pixels long, as a position in
/// the centred window `window` pixels long on it: less the window's
/// origin, half of `frame - window`, rounded down. `None` when the window
/// is longer than the frame, or the position lies before the window's
/// start.
fn shifted(value: u32, frame: u32, window: u32) -> Option<u32> {
    value.checked_sub(frame.checked_sub(window)? / 2)
}

/// Whether `makernote`'s FocusMode records manual focus: whether it is of
/// type ASCII and reads `MANUAL`, exactly, by the rule every text value of
/// the metadata is read by ([`tiff_metadata::text`]), which cuts it at its
/// first NUL and trims the spaces that pad it. A FocusMode that is
/// missing, of another type, or that reads anything else, such as `AF-S`,
/// records no manual focus.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn manual_focus<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<bool> {
    let mode = tiff_metadata::text(src, &makernote.ifd, FOCUS_MODE)?;
    Ok(mode.as_deref() == Some("MANUAL"))
}

/// Where one AFInfo2 version keeps the focus point: the byte offsets, from
/// the start of the value, of the fields read, and the shortest value that
/// version writes.
///
/// Each of the frame, the centre and the area is two SHORTs in the
/// MakerNote's byte order, the width or x at its offset and the height or y
/// two bytes after it. The offsets are those of ExifTool's published
/// tables, confirmed on the public samples whose fields hold values.
#[derive(Clone, Copy, Debug)]
struct Layout {
    /// How long a value of this version is, at least: the length of the
    /// version's layout, whose last fields may not be read.
    min_len: usize,
    /// The frame's width, then its height.
    frame: usize,
    /// The centre's x, then its y.
    center: usize,
    /// The area's width, then its height.
    area: usize,
    /// The byte that says whether focus was confirmed, on the versions
    /// that have one.
    in_focus: Option<usize>,
}

/// The layout of AFInfo2 version `version`, its first four bytes: `None`
/// for any version but the exact ones read, 0100, 0101, 0300, 0301, 0400,
/// 0401 and 0402. A version that shares its first two digits with these,
/// such as 0200 or a future 0403, may have moved its fields, and is not
/// read at the offsets of its neighbours (ADR012).
fn layout(version: &[u8]) -> Option<Layout> {
    match version {
        b"0100" => Some(Layout {
            min_len: 30,
            frame: 16,
            center: 20,
            area: 24,
            in_focus: Some(28),
        }),
        b"0101" => Some(Layout {
            min_len: 84,
            frame: 70,
            center: 74,
            area: 78,
            in_focus: Some(82),
        }),
        b"0300" | b"0301" => Some(Layout {
            min_len: 60,
            frame: 42,
            center: 46,
            area: 50,
            in_focus: None,
        }),
        b"0400" | b"0401" | b"0402" => Some(Layout {
            min_len: 80,
            frame: 62,
            center: 66,
            area: 70,
            in_focus: Some(74),
        }),
        _ => None,
    }
}

/// What an AFInfo2 value records of the focus point, read but not yet
/// checked: the frame, the centre of the focus area in it, the area's
/// size, and whether focus was confirmed, on the versions that say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FocusRecord {
    /// The frame the centre and the area are measured in.
    frame: Size,
    /// The centre of the focus area.
    center: Point,
    /// The focus area's width and height.
    area: Size,
    /// Whether focus was confirmed: `None` when the version has no byte
    /// for it, or the byte is neither 1 nor 0.
    in_focus: Option<bool>,
}

impl FocusRecord {
    /// The focus point this record describes: `None` when [`Focus::new`]
    /// or [`Focus::with_size`] refuses it, for a frame or an area with a
    /// zero side, which is how a shot through a DSLR's viewfinder says it
    /// records no position, a centre outside the frame, or an area that
    /// does not fit inside the frame around its centre.
    ///
    /// [`in_crop`] calls it on the record as recorded, on every file, and
    /// on a crop that keeps one side again, on the record moved into the
    /// cropped picture.
    fn focus(self) -> Option<Focus> {
        // An area that `with_size` refuses costs the whole focus, not only
        // the area. A zero side is how a Nikon file says it records no
        // position; and an area of no zero side that does not fit around
        // the centre is a record that cannot be used, which gives no focus
        // on every format, as ADR012 decides.
        let focus = Focus::new(self.frame, self.center)?.with_size(self.area)?;
        Some(match self.in_focus {
            Some(in_focus) => focus.with_in_focus(in_focus),
            None => focus,
        })
    }
}

/// What the AFInfo2 value `value` records, its SHORTs in byte order
/// `order`: the frame, the centre and the area at its version's
/// [`layout`], and the in-focus byte, when the layout has one, as `true`
/// for 1, `false` for 0, and nothing said for any other value.
///
/// `None` when the version is not one read, or the value is shorter than
/// its layout. The values are not checked here: [`in_crop`] checks them,
/// as recorded, with [`FocusRecord::focus`], before it places them.
fn af_info2_record(value: &[u8], order: ByteOrder) -> Option<FocusRecord> {
    let layout = layout(value.get(..4)?)?;
    if value.len() < layout.min_len {
        return None;
    }
    let (width, height) = shorts_at(value, layout.frame, order)?;
    let frame = Size { width, height };
    let (x, y) = shorts_at(value, layout.center, order)?;
    let (width, height) = shorts_at(value, layout.area, order)?;
    let area = Size { width, height };
    let in_focus = layout
        .in_focus
        .and_then(|at| value.get(at))
        .and_then(|&byte| match byte {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        });
    Some(FocusRecord {
        frame,
        center: Point { x, y },
        area,
        in_focus,
    })
}

/// The two SHORTs at `at` in `value`, in byte order `order`; `None` when
/// they do not lie inside it.
fn shorts_at(value: &[u8], at: usize, order: ByteOrder) -> Option<(u32, u32)> {
    let &[first0, first1, second0, second1] = value.get(at..)?.first_chunk::<4>()?;
    Some((
        u32::from(order.u16([first0, first1])),
        u32::from(order.u16([second0, second1])),
    ))
}

/// The value of tag `tag` of `ifd`, as [`Ifd::u32`] reads it, when the
/// entry holds it in its own value field; `None` when the tag is missing,
/// of another type, or stored out of line.
///
/// A PreviewIFD pointer, a JPEG offset and a JPEG length are each one
/// number, which an entry always holds itself. One stored out of line
/// could be read from anywhere in the file, outside the block.
///
/// # Errors
///
/// `Io` when the reader fails.
fn inline_u32<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd, tag: u16) -> Result<Option<u32>> {
    if ifd
        .entry(tag)
        .is_none_or(|entry| entry.value_offset().is_some())
    {
        return Ok(None);
    }
    Ok(skip_damage(ifd.u32(src, tag))?.flatten())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::{
        BODY_SERIAL_NUMBER, DATE_TIME_ORIGINAL, DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE,
        IMAGE_LENGTH, IMAGE_WIDTH, LENS_MODEL, LENS_SPECIFICATION, MAKE, OFFSET_TIME_ORIGINAL,
        PHOTOGRAPHIC_SENSITIVITY, PHOTOMETRIC_INTERPRETATION, SUB_SEC_TIME_ORIGINAL,
    };
    use crate::container::tiff::types::{BYTE, IFD, RATIONAL, SRATIONAL};
    use crate::container::tiff::{self, parse_ifd, walk};
    use crate::datetime::DateTime;
    use crate::error::ErrorKind;
    use crate::jpeg_stub::jpeg_stub;
    use crate::tiff_builder::{ASCII, IfdId, Order, RawValue, TiffBuilder};
    use std::io::{self, Cursor, SeekFrom};

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// Where the tests put the MakerNote block in the file.
    const AT: u64 = 1024;

    /// How far the file goes on past the block, so that what a test points
    /// past the block still lies inside the file.
    const TAIL: u64 = 4096;

    /// The Exif tag UserComment, UNDEFINED like the MakerNote: a tag that
    /// may hold any bytes and is not the MakerNote.
    const USER_COMMENT: u16 = 0x9286;

    /// The Nikon MakerNote tag MakerNoteVersion.
    const MAKER_NOTE_VERSION: u16 = 0x0001;

    /// What a test changes in the inner TIFF structure of a MakerNote
    /// block.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Change {
        /// Nothing: a sound block.
        Nothing,
        /// The inner header's 42 is 41.
        Magic41,
        /// The PreviewIFD declares no JPEG.
        NoJpegTags,
        /// The PreviewIFD declares the JPEG's offset, not its length.
        NoJpegLength,
        /// The PreviewIFD pointer is stored out of line, inside the block,
        /// as two LONGs whose first is the right offset.
        PointerOutOfLine,
        /// The PreviewIFD holds, before its JPEGInterchangeFormat, another
        /// one of two LONGs, stored out of line in the 8 bytes just past the
        /// block's end, which lie inside the file. The JPEG's length is then
        /// the PreviewIFD's third entry, not the one [`Block::length_at`]
        /// gives.
        JpegOffsetPastTheBlockFirst,
    }

    /// A MakerNote block and where its parts are, each counted from the
    /// start of the block.
    struct Block {
        bytes: Vec<u8>,
        order: Order,
        /// The MakerNote IFD.
        makernote: u64,
        /// The PreviewIFD.
        preview: u64,
        /// The JPEG: 10, where the inner structure starts, plus the JPEG's
        /// offset in it.
        jpeg: u64,
        /// The JPEG's length.
        jpeg_len: u64,
    }

    impl Block {
        /// The block's length.
        fn len(&self) -> u64 {
            u64::try_from(self.bytes.len()).unwrap()
        }

        /// Writes `value` at `at`, in the block's byte order.
        fn put_u32(&mut self, at: u64, value: u32) {
            let at = usize::try_from(at).unwrap();
            self.bytes[at..at + 4].copy_from_slice(&self.order.u32(value));
        }

        /// Where the value field of the MakerNote IFD's PreviewIFD entry
        /// is: the second entry, after MakerNoteVersion, 8 bytes in.
        fn pointer_at(&self) -> u64 {
            self.makernote + 2 + 12 + 8
        }

        /// Where the value field of the PreviewIFD's
        /// JPEGInterchangeFormatLength entry is: the second entry, after
        /// JPEGInterchangeFormat, 8 bytes in.
        fn length_at(&self) -> u64 {
            self.preview + 2 + 12 + 8
        }

        /// The candidate the block gives when it sits at `at` in the file.
        fn candidate(&self, at: u64) -> Candidate {
            Candidate {
                offset: at + self.jpeg,
                length: self.jpeg_len,
            }
        }
    }

    /// A Nikon MakerNote block of the embedded-TIFF-header layout, in byte
    /// order `order`, changed as `change` says: `Nikon\0`, the version
    /// `02 11`, two zero bytes, then an inner TIFF structure whose offsets
    /// count from its own first byte, as the MakerNote's do. Its MakerNote
    /// IFD holds MakerNoteVersion and the PreviewIFD pointer; its PreviewIFD
    /// declares a 640 by 424 stub JPEG, the last bytes of the block.
    fn block(order: Order, change: Change) -> Block {
        let mut inner = TiffBuilder::new(order);
        let makernote = inner.ifd();
        let preview = inner.ifd();
        inner.undefined(makernote, MAKER_NOTE_VERSION, b"0211");
        if change == Change::PointerOutOfLine {
            // The PreviewIFD's offset depends only on the IFD before it,
            // so a copy with a stand-in value of the same length gives it.
            let mut probe = inner.clone();
            probe.long(makernote, PREVIEW_IFD, &[0, 0]);
            let offset = u32::try_from(probe.build().ifd_offset(preview)).unwrap();
            inner.long(makernote, PREVIEW_IFD, &[offset, 0]);
        } else {
            inner.ifd_pointer(makernote, PREVIEW_IFD, preview);
        }
        if change == Change::Magic41 {
            inner.magic(41);
        }
        let stub = jpeg_stub(640, 424, 0xC0);
        let jpeg = inner.blob(&stub);
        if change == Change::JpegOffsetPastTheBlockFirst {
            // Entries of the same tag keep the order they were added in, so
            // this one comes first. Its offset is set once the block's
            // length is known.
            inner.raw(
                preview,
                JPEG_INTERCHANGE_FORMAT,
                LONG,
                2,
                RawValue::Offset(0),
            );
        }
        if change != Change::NoJpegTags {
            inner.long_blob(preview, JPEG_INTERCHANGE_FORMAT, jpeg);
        }
        if !matches!(change, Change::NoJpegTags | Change::NoJpegLength) {
            inner.long_blob_len(preview, JPEG_INTERCHANGE_FORMAT_LENGTH, jpeg);
        }
        let built = inner.build();

        let mut bytes = b"Nikon\0\x02\x11\0\0".to_vec();
        bytes.extend_from_slice(&built.bytes);
        let mut block = Block {
            bytes,
            order,
            makernote: HEADER_AT + built.ifd_offset(makernote),
            preview: HEADER_AT + built.ifd_offset(preview),
            jpeg: HEADER_AT + built.blob_offset(jpeg),
            jpeg_len: u64::try_from(stub.len()).unwrap(),
        };
        if change == Change::JpegOffsetPastTheBlockFirst {
            // The block's end, counted from the inner header, written in
            // the value field of the PreviewIFD's first entry, 8 bytes in.
            let end = u32::try_from(block.len() - HEADER_AT).unwrap();
            block.put_u32(block.preview + 2 + 8, end);
        }
        block
    }

    /// A Nikon file in byte order `order`: IFD0 holds Make and points at an
    /// Exif IFD, which holds ExifVersion and what `exif` adds. `bytes` lie
    /// at `at`, and the file ends `tail` bytes after them.
    fn nikon_file(
        order: Order,
        at: u64,
        bytes: &[u8],
        tail: u64,
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Vec<u8> {
        nikon_file_with(order, at, bytes, tail, |builder, _, exif_ifd| {
            exif(builder, exif_ifd);
        })
    }

    /// As [`nikon_file`], with `build` given the builder, IFD0 and the Exif
    /// IFD, to add to either.
    fn nikon_file_with(
        order: Order,
        at: u64,
        bytes: &[u8],
        tail: u64,
        build: impl FnOnce(&mut TiffBuilder, IfdId, IfdId),
    ) -> Vec<u8> {
        let mut builder = TiffBuilder::new(order);
        let ifd0 = builder.ifd();
        let exif_ifd = builder.ifd();
        builder
            .ascii(ifd0, MAKE, "NIKON CORPORATION")
            .exif(ifd0, exif_ifd);
        builder.undefined(exif_ifd, 0x9000, b"0232");
        build(&mut builder, ifd0, exif_ifd);
        builder.blob_at(at, bytes);
        builder.pad_to(at + u64::try_from(bytes.len()).unwrap() + tail);
        builder.build().bytes
    }

    /// Adds to `ifd` an entry of tag `tag` and type `typ` whose value is
    /// the `count` bytes at `at`.
    fn entry_at(builder: &mut TiffBuilder, ifd: IfdId, tag: u16, typ: u16, at: u64, count: u64) {
        let count = u32::try_from(count).unwrap();
        let at = u32::try_from(at).unwrap();
        builder.raw(ifd, tag, typ, count, RawValue::Offset(at));
    }

    /// A Nikon file in byte order `order` holding `bytes` at [`AT`], whose
    /// Exif IFD's MakerNote, of type `typ`, is the first `count` of them.
    fn with_makernote(order: Order, typ: u16, bytes: &[u8], count: u64) -> Vec<u8> {
        nikon_file(order, AT, bytes, TAIL, |builder, exif| {
            entry_at(builder, exif, MAKER_NOTE, typ, AT, count);
        })
    }

    /// A Nikon file in byte order `order` whose MakerNote is `block`.
    fn with_block(order: Order, block: &Block) -> Vec<u8> {
        with_makernote(order, UNDEFINED, &block.bytes, block.len())
    }

    /// The MakerNote's preview candidate in the file in `src`, whose IFDs
    /// are `tree`: what [`read`] and then [`preview_candidate`] return,
    /// each under `limits`.
    fn read_candidate<R: Read + Seek>(
        src: &mut Source<R>,
        tree: &Tree,
        limits: &Limits,
    ) -> Result<Option<Candidate>> {
        let Some(makernote) = read(src, tree, limits)? else {
            return Ok(None);
        };
        preview_candidate(src, &makernote, limits)
    }

    /// The MakerNote's preview candidate in the file `bytes`, whose IFDs
    /// are walked with the default limits, as [`read_candidate`] finds it
    /// under `limits`.
    fn try_candidate(bytes: Vec<u8>, limits: &Limits) -> Result<Option<Candidate>> {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let file_block = tiff::Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        read_candidate(&mut src, &tree, limits)
    }

    /// The MakerNote's preview candidate in the file `bytes`, which must
    /// not be an error.
    fn candidate(bytes: Vec<u8>) -> Option<Candidate> {
        try_candidate(bytes, &Limits::default()).unwrap()
    }

    #[test]
    fn the_preview_is_at_the_blocks_offset_plus_10_plus_its_own() {
        for order in ORDERS {
            let block = block(order, Change::Nothing);
            let bytes = with_block(order, &block);
            let found = candidate(bytes.clone()).expect("a candidate");
            assert_eq!(found, block.candidate(AT), "{order:?}");
            // The JPEG's own first bytes are there.
            let start = usize::try_from(found.offset).unwrap();
            assert_eq!(bytes[start..start + 3], [0xFF, 0xD8, 0xFF], "{order:?}");
        }
    }

    #[test]
    fn a_makernote_of_type_byte_is_read() {
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_makernote(Order::Little, BYTE, &block.bytes, block.len());
        assert_eq!(candidate(bytes), Some(block.candidate(AT)));
    }

    #[test]
    fn a_block_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = block(inner, Change::Nothing);
            let bytes = with_block(file, &block);
            assert_eq!(candidate(bytes), Some(block.candidate(AT)), "{file:?}");
        }
    }

    /// Where the tests of a TIFF structure inside a larger file place its
    /// header, as a JPEG file holds the structure of its Exif segment.
    const POS: u64 = 1000;

    /// The file `bytes`, placed at [`POS`] in a larger file, after as many
    /// bytes of filler: a source of that file, and the tree walked with the
    /// header at POS as its base and the bytes from it as its bounds.
    fn placed(bytes: &[u8]) -> (Source<Cursor<Vec<u8>>>, Tree) {
        let mut file = vec![0; usize::try_from(POS).unwrap()];
        file.extend_from_slice(bytes);
        let mut src = Source::new(Cursor::new(file)).unwrap();
        let block = tiff::Block::new(POS, POS..src.len());
        let tree = walk(&mut src, &block, &Limits::default()).unwrap();
        (src, tree)
    }

    #[test]
    fn a_makernote_inside_a_file_counts_from_its_own_header() {
        for order in ORDERS {
            // The MakerNote lies POS bytes further into the file, with its
            // inner header, so its preview does too.
            let block = block(order, Change::Nothing);
            let (mut src, tree) = placed(&with_block(order, &block));
            let found = read_candidate(&mut src, &tree, &Limits::default()).unwrap();
            assert_eq!(found, Some(block.candidate(POS + AT)), "{order:?}");

            // A value stored out of line, read through the inner header.
            let tags = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            let (mut src, tree) = placed(&exif_file(order, &tags, whole(&tags), |_, _| {}));
            let makernote = read(&mut src, &tree, &Limits::default())
                .unwrap()
                .expect("a MakerNote");
            assert_eq!(
                makernote.ifd.block().base(),
                POS + AT + HEADER_AT,
                "{order:?}"
            );
            let standard = tiff_metadata::read(&mut src, &tree).unwrap();
            let metadata = metadata(&mut src, &makernote, standard).unwrap();
            assert_eq!(metadata.serial_number.as_deref(), Some(SERIAL), "{order:?}");
        }
    }

    #[test]
    fn a_file_without_an_exif_ifd_has_none() {
        // A sound MakerNote, but in IFD0, where no MakerNote belongs.
        let block = block(Order::Little, Change::Nothing);
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        builder.ascii(ifd0, MAKE, "NIKON CORPORATION");
        entry_at(&mut builder, ifd0, MAKER_NOTE, UNDEFINED, AT, block.len());
        builder.blob_at(AT, &block.bytes);
        assert_eq!(candidate(builder.build().bytes), None);
    }

    #[test]
    fn an_exif_ifd_without_a_makernote_has_none() {
        // A sound block, but under another tag.
        let block = block(Order::Little, Change::Nothing);
        let bytes = nikon_file(Order::Little, AT, &block.bytes, TAIL, |builder, exif| {
            entry_at(builder, exif, USER_COMMENT, UNDEFINED, AT, block.len());
        });
        assert_eq!(candidate(bytes), None);
    }

    #[test]
    fn a_makernote_that_is_not_bytes_has_none() {
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_makernote(Order::Little, ASCII, &block.bytes, block.len());
        assert_eq!(candidate(bytes), None);
    }

    #[test]
    fn a_block_shorter_than_the_inner_header_has_none() {
        // The first 17 bytes of a sound block, which the file goes on after.
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_makernote(Order::Little, UNDEFINED, &block.bytes, 17);
        assert_eq!(candidate(bytes), None);

        // Five bytes at the very end of the file, where reading the 6-byte
        // signature would run past the end.
        let bytes = nikon_file(Order::Little, AT, b"Nikon", 0, |builder, exif| {
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, 5);
        });
        assert_eq!(candidate(bytes), None);
    }

    #[test]
    fn another_makers_signature_has_none() {
        let mut block = block(Order::Little, Change::Nothing);
        block.bytes[..6].copy_from_slice(b"Canon\0");
        assert_eq!(candidate(with_block(Order::Little, &block)), None);
    }

    #[test]
    fn the_short_header_layout_has_none() {
        // `Nikon\0\1\0`, then an IFD: the sound block's MakerNote IFD and
        // what follows it.
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            let mut bytes = b"Nikon\0\x01\0".to_vec();
            bytes.extend_from_slice(&sound.bytes[usize::try_from(sound.makernote).unwrap()..]);
            let count = u64::try_from(bytes.len()).unwrap();
            let file = with_makernote(order, UNDEFINED, &bytes, count);
            assert_eq!(candidate(file), None, "{order:?}");
        }
    }

    #[test]
    fn the_headerless_layout_has_none() {
        // A bare IFD: the sound block's MakerNote IFD and what follows it.
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            let bytes = &sound.bytes[usize::try_from(sound.makernote).unwrap()..];
            let count = u64::try_from(bytes.len()).unwrap();
            let file = with_makernote(order, UNDEFINED, bytes, count);
            assert_eq!(candidate(file), None, "{order:?}");
        }
    }

    #[test]
    fn an_inner_header_without_42_has_none() {
        for order in ORDERS {
            let block = block(order, Change::Magic41);
            assert_eq!(candidate(with_block(order, &block)), None, "{order:?}");
        }
    }

    /// Which IFD of the block [`relocated`] moves.
    #[derive(Clone, Copy, Debug)]
    enum Part {
        MakerNoteIfd,
        PreviewIfd,
    }

    /// A sound block in byte order `order` whose `part` is read from a copy
    /// starting `overlap` bytes before the block's end, the offset that
    /// leads to it changed to match; and the block's length, the count
    /// the MakerNote declares, which the copy runs past.
    ///
    /// The copy is a whole IFD inside the file, and its own offsets lead
    /// back into the block, so the block's end is the one thing that can
    /// reject it: a MakerNote declared long enough to hold the copy gives
    /// the preview.
    fn relocated(order: Order, part: Part, overlap: u64) -> (Vec<u8>, u64, Candidate) {
        let mut block = block(order, Change::Nothing);
        let (range, pointer_at) = match part {
            // The inner header's IFD offset, 4 bytes into it.
            Part::MakerNoteIfd => (block.makernote..block.preview, HEADER_AT + 4),
            Part::PreviewIfd => (block.preview..block.jpeg, block.pointer_at()),
        };
        let copy = block.bytes[usize::try_from(range.start).unwrap()..]
            [..usize::try_from(range.end - range.start).unwrap()]
            .to_vec();
        let copy_at = block.len() - overlap;
        block.put_u32(pointer_at, u32::try_from(copy_at - HEADER_AT).unwrap());
        let declared = block.len();
        let expected = block.candidate(AT);
        let mut bytes = block.bytes;
        bytes.truncate(usize::try_from(copy_at).unwrap());
        bytes.extend_from_slice(&copy);
        (bytes, declared, expected)
    }

    #[test]
    fn an_ifd_past_the_block_has_none() {
        for order in ORDERS {
            for part in [Part::MakerNoteIfd, Part::PreviewIfd] {
                let (bytes, declared, expected) = relocated(order, part, 0);
                let file = with_makernote(order, UNDEFINED, &bytes, declared);
                assert_eq!(candidate(file), None, "{order:?}, {part:?}");

                // The control: declared to hold the copy, it gives the
                // preview, so the block's end alone rejected it above.
                let whole = u64::try_from(bytes.len()).unwrap();
                let file = with_makernote(order, UNDEFINED, &bytes, whole);
                assert_eq!(candidate(file), Some(expected), "{order:?}, {part:?}");
            }
        }
    }

    #[test]
    fn an_ifd_whose_entries_run_past_the_block_has_none() {
        // The IFD's count is the block's last two bytes; its entries follow
        // it in the file.
        for order in ORDERS {
            for part in [Part::MakerNoteIfd, Part::PreviewIfd] {
                let (bytes, declared, expected) = relocated(order, part, 2);
                let file = with_makernote(order, UNDEFINED, &bytes, declared);
                assert_eq!(candidate(file), None, "{order:?}, {part:?}");

                let whole = u64::try_from(bytes.len()).unwrap();
                let file = with_makernote(order, UNDEFINED, &bytes, whole);
                assert_eq!(candidate(file), Some(expected), "{order:?}, {part:?}");
            }
        }
    }

    #[test]
    fn an_ifd_offset_past_the_end_of_the_file_has_none() {
        for order in ORDERS {
            let sound = block(order, Change::Nothing);
            for pointer_at in [HEADER_AT + 4, sound.pointer_at()] {
                let mut block = block(order, Change::Nothing);
                block.put_u32(pointer_at, u32::MAX);
                assert_eq!(
                    candidate(with_block(order, &block)),
                    None,
                    "{order:?}, the offset at {pointer_at}"
                );
            }
        }
    }

    #[test]
    fn a_preview_ifd_pointer_stored_out_of_line_has_none() {
        for order in ORDERS {
            let block = block(order, Change::PointerOutOfLine);
            assert_eq!(candidate(with_block(order, &block)), None, "{order:?}");
        }
    }

    #[test]
    fn a_preview_ifd_without_the_jpegs_offset_and_length_has_none() {
        for change in [Change::NoJpegTags, Change::NoJpegLength] {
            let block = block(Order::Little, change);
            assert_eq!(
                candidate(with_block(Order::Little, &block)),
                None,
                "{change:?}"
            );
        }
    }

    #[test]
    fn a_jpeg_running_past_the_block_has_none() {
        for order in ORDERS {
            // Declared one byte short of the JPEG's end, which is the end
            // of the bytes: the JPEG lies inside the file, not the block.
            let sound = block(order, Change::Nothing);
            let file = with_makernote(order, UNDEFINED, &sound.bytes, sound.len() - 1);
            assert_eq!(candidate(file), None, "{order:?}");

            // A JPEG length so long that its end passes the file's too.
            let mut long = block(order, Change::Nothing);
            long.put_u32(long.length_at(), u32::MAX);
            assert_eq!(candidate(with_block(order, &long)), None, "{order:?}");
        }
    }

    #[test]
    fn a_preview_ifd_entry_whose_value_lies_past_the_block_is_dropped() {
        // The first of the PreviewIFD's two JPEGInterchangeFormat entries
        // stores its value past the block, inside the file. Dropped when
        // the PreviewIFD is read, it leaves the second, which the entry
        // holds, to give the JPEG's offset. Kept, it would be the one
        // found, and refused as stored out of line: no preview.
        for order in ORDERS {
            let block = block(order, Change::JpegOffsetPastTheBlockFirst);
            let bytes = with_block(order, &block);

            // The PreviewIFD as the file holds it, read from the MakerNote's
            // base with the whole file as bounds: the out-of-line entry
            // first, its value the 8 bytes just past the block.
            let limits = Limits::default();
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &limits).unwrap();
            let makernote = read(&mut src, &tree, &limits)
                .unwrap()
                .expect("a MakerNote");
            let at = AT + block.preview;
            let whole_file = tiff::Block::new(makernote.ifd.block().base(), 0..src.len());
            let (table, _) = parse_ifd(&mut src, &whole_file, at, makernote.ifd.order(), &limits)
                .expect("the PreviewIFD");
            let first = &table.entries()[0];
            assert_eq!(first.tag(), JPEG_INTERCHANGE_FORMAT, "{order:?}");
            assert_eq!(first.value_offset(), Some(AT + block.len()), "{order:?}");
            assert_eq!(table.invalid_entries(), 0, "{order:?}");

            assert_eq!(candidate(bytes), Some(block.candidate(AT)), "{order:?}");
        }
    }

    #[test]
    fn an_empty_jpeg_is_passed_on() {
        let mut block = block(Order::Little, Change::Nothing);
        block.put_u32(block.length_at(), 0);
        let found = candidate(with_block(Order::Little, &block));
        assert_eq!(found.map(|c| c.length), Some(0));
    }

    #[test]
    fn a_makernote_ifd_with_more_entries_than_the_limit_has_none() {
        // The MakerNote IFD holds two entries, the PreviewIFD two.
        let block = block(Order::Little, Change::Nothing);
        let bytes = with_block(Order::Little, &block);
        let found = try_candidate(bytes.clone(), &Limits::default().max_ifd_entries(2));
        assert_eq!(found.unwrap(), Some(block.candidate(AT)));
        let found = try_candidate(bytes, &Limits::default().max_ifd_entries(1));
        assert_eq!(found.unwrap(), None);
    }

    /// The raw size of the files the CropArea tests build: the Z 9's.
    const RAW: Size = Size {
        width: 8280,
        height: 5520,
    };

    /// The Z 9's CropArea: the 8256 by 5504 pixels that start 12 columns
    /// and 8 rows into its raw (public samples 5146 to 5148).
    const Z9_CROP_AREA: [u16; 4] = [12, 8, 8256, 5504];

    /// The image size [`Z9_CROP_AREA`] gives.
    const Z9_IMAGE: Size = Size {
        width: 8256,
        height: 5504,
    };

    /// The PhotometricInterpretation of a colour filter array: raw data, as
    /// the preview finder takes it.
    const CFA: u16 = 32803;

    /// A Nikon MakerNote block in byte order `order` whose MakerNote IFD
    /// holds the MakerNoteVersion and what `tags` adds, given the inner
    /// builder and that IFD. When that is one tag, such as a CropArea, whose
    /// value is stored out of line, the value is the last bytes of the
    /// block. Also where the IFD's second entry is, the one tag's when
    /// `tags` adds one, counted from the start of the block.
    fn tags_block(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> (Vec<u8>, u64) {
        let mut inner = TiffBuilder::new(order);
        let makernote = inner.ifd();
        inner.undefined(makernote, MAKER_NOTE_VERSION, b"0211");
        tags(&mut inner, makernote);
        let built = inner.build();
        let mut bytes = b"Nikon\0\x02\x11\0\0".to_vec();
        bytes.extend_from_slice(&built.bytes);
        (bytes, HEADER_AT + built.ifd_offset(makernote) + 2 + 12)
    }

    /// A MakerNote block in byte order `order` whose CropArea is `values`,
    /// as SHORTs.
    fn shorts_block(order: Order, values: &[u16]) -> Vec<u8> {
        tags_block(order, |inner, ifd| {
            inner.short(ifd, CROP_AREA, values);
        })
        .0
    }

    /// A MakerNote block in byte order `order` whose CropArea is `values`,
    /// as LONGs.
    fn longs_block(order: Order, values: &[u32]) -> Vec<u8> {
        tags_block(order, |inner, ifd| {
            inner.long(ifd, CROP_AREA, values);
        })
        .0
    }

    /// A Nikon file in byte order `order` whose IFD0 holds raw data of size
    /// [`RAW`] and what `raw` adds, given the builder and IFD0, and whose
    /// MakerNote, at [`AT`], is the first `count` bytes of `block`.
    fn raw_file(
        order: Order,
        block: &[u8],
        count: u64,
        raw: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Vec<u8> {
        raw_file_of(order, RAW, block, count, raw)
    }

    /// As [`raw_file`], with raw data of size `size`.
    fn raw_file_of(
        order: Order,
        size: Size,
        block: &[u8],
        count: u64,
        raw: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Vec<u8> {
        nikon_file_with(order, AT, block, TAIL, |builder, ifd0, exif| {
            builder
                .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[CFA])
                .long(ifd0, IMAGE_WIDTH, &[size.width])
                .long(ifd0, IMAGE_LENGTH, &[size.height]);
            raw(builder, ifd0);
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, count);
        })
    }

    /// The metadata of the file `bytes`: what the standard tags give, with
    /// what [`metadata`] adds from the file's MakerNote, which must be
    /// read.
    fn metadata_of(bytes: Vec<u8>) -> Metadata {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let file_block = tiff::Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
        let makernote = read(&mut src, &tree, &Limits::default())
            .unwrap()
            .expect("a MakerNote");
        let standard = tiff_metadata::read(&mut src, &tree).unwrap();
        metadata(&mut src, &makernote, standard).unwrap()
    }

    /// The image size of the file [`raw_file`] builds in byte order `order`
    /// with the MakerNote `block`, declared whole, and nothing more in
    /// IFD0; the raw size must be read.
    fn image_size(order: Order, block: &[u8]) -> Option<Size> {
        let count = u64::try_from(block.len()).unwrap();
        let metadata = metadata_of(raw_file(order, block, count, |_, _| {}));
        assert_eq!(metadata.raw_size, Some(RAW), "{order:?}");
        metadata.image_size
    }

    #[test]
    fn a_crop_area_gives_the_image_size() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            assert_eq!(image_size(order, &block), Some(Z9_IMAGE), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_of_longs_is_read() {
        for order in ORDERS {
            let block = longs_block(order, &Z9_CROP_AREA.map(u32::from));
            assert_eq!(image_size(order, &block), Some(Z9_IMAGE), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = shorts_block(inner, &Z9_CROP_AREA);
            assert_eq!(image_size(file, &block), Some(Z9_IMAGE), "{file:?}");
        }
    }

    #[test]
    fn a_makernote_without_a_crop_area_gives_the_raw_size() {
        // As the bodies before the D850 write it.
        for order in ORDERS {
            let block = tags_block(order, |_, _| {}).0;
            assert_eq!(image_size(order, &block), Some(RAW), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_whose_values_run_past_the_block_gives_the_raw_size() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            // The values are the block's last 8 bytes.
            let values: Vec<u8> = Z9_CROP_AREA.iter().flat_map(|&v| order.u16(v)).collect();
            assert_eq!(block[block.len() - 8..], values, "{order:?}");

            // Declared one byte short: the values' last byte lies in the
            // file, after the block.
            let whole = u64::try_from(block.len()).unwrap();
            let file = raw_file(order, &block, whole - 1, |_, _| {});
            assert_eq!(metadata_of(file).image_size, Some(RAW), "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = raw_file(order, &block, whole, |_, _| {});
            assert_eq!(metadata_of(file).image_size, Some(Z9_IMAGE), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_outside_the_raw_gives_the_raw_size() {
        for order in ORDERS {
            for values in [
                // Wider or taller than the raw by itself.
                [0, 0, 8281, 5520],
                [0, 0, 8280, 5521],
                // One pixel past the raw's edge, with the left column or
                // the top row.
                [12, 8, 8269, 5504],
                [12, 17, 8256, 5504],
            ] {
                let block = shorts_block(order, &values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
            // Sums that would wrap round to 1, inside the raw.
            for values in [[u32::MAX, 8, 2, 5504], [12, u32::MAX, 8256, 2]] {
                let block = longs_block(order, &values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
        }
    }

    #[test]
    fn a_crop_area_of_zero_width_or_height_gives_the_raw_size() {
        for order in ORDERS {
            for values in [[12, 8, 0, 5504], [12, 8, 8256, 0]] {
                let block = shorts_block(order, &values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
        }
    }

    #[test]
    fn a_crop_area_of_other_than_four_values_gives_the_raw_size() {
        for order in ORDERS {
            // Three and five values, stored out of line; and two, which the
            // entry holds itself.
            for values in [&[12, 8, 8256][..], &[12, 8, 8256, 5504, 0], &[12, 8]] {
                let block = shorts_block(order, values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
        }
    }

    #[test]
    fn a_crop_area_of_another_type_gives_the_raw_size() {
        for order in ORDERS {
            // The 16 bytes of four LONGs that would give the Z 9's image,
            // declared as two RATIONALs, as 16 UNDEFINED bytes, or as four
            // IFD offsets, which are numbers too.
            for (typ, count) in [(RATIONAL, 2), (UNDEFINED, 16), (IFD, 4)] {
                let (mut block, entry) = tags_block(order, |inner, ifd| {
                    inner.long(ifd, CROP_AREA, &Z9_CROP_AREA.map(u32::from));
                });
                let entry = usize::try_from(entry).unwrap();
                block[entry + 2..entry + 4].copy_from_slice(&order.u16(typ));
                block[entry + 4..entry + 8].copy_from_slice(&order.u32(count));
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, type {typ}"
                );
            }
        }
    }

    #[test]
    fn without_a_raw_size_a_crop_area_gives_no_image_size() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            let count = u64::try_from(block.len()).unwrap();
            let metadata = metadata_of(with_makernote(order, UNDEFINED, &block, count));
            assert_eq!(metadata.raw_size, None, "{order:?}");
            assert_eq!(metadata.image_size, None, "{order:?}");
        }
    }

    #[test]
    fn a_default_crop_comes_before_the_crop_area() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            let count = u64::try_from(block.len()).unwrap();
            // One smaller than the crop area, and one of the whole raw,
            // which the image size alone cannot tell from no crop at all.
            for (origin, size) in [([4, 4], [8000, 5000]), ([0, 0], [RAW.width, RAW.height])] {
                let file = raw_file(order, &block, count, |builder, ifd0| {
                    builder.long(ifd0, DEFAULT_CROP_ORIGIN, &origin).long(
                        ifd0,
                        DEFAULT_CROP_SIZE,
                        &size,
                    );
                });
                let expected = Size {
                    width: size[0],
                    height: size[1],
                };
                assert_eq!(
                    metadata_of(file).image_size,
                    Some(expected),
                    "{order:?}, {size:?}"
                );
            }
        }
    }

    /// A made-up serial number of seven digits, as the bodies from the
    /// D300 on write theirs; no camera's serial number is copied here.
    const SERIAL: &str = "7654321";

    /// Another made-up serial number, for the Exif tag, which must win.
    const EXIF_SERIAL: &str = "1234567";

    /// The D300's Lens tag: an 18-70 mm lens of f/3.5 to f/4.5 (public
    /// sample 928).
    const D300_LENS: [(u32, u32); 4] = [(180, 10), (700, 10), (35, 10), (45, 10)];

    /// Another lens's range, a 24-120 mm f/4, for the Exif tag, which must
    /// win.
    const EXIF_LENS: [(u32, u32); 4] = [(240, 10), (1200, 10), (400, 100), (400, 100)];

    /// `values` as the reader returns a lens's range, each fraction as
    /// stored, and `None` where either of its numbers is zero, by the
    /// readers' own check ([`Rational::known`]).
    fn known(values: [(u32, u32); 4]) -> [Option<Rational>; 4] {
        values.map(|(numerator, denominator)| Rational::known(numerator, denominator))
    }

    /// The four fractions of `lens`'s range, in the order
    /// [`Lens::from_parts`] takes them.
    fn range_of(lens: &Lens) -> [Option<Rational>; 4] {
        [
            lens.min_focal_length(),
            lens.max_focal_length(),
            lens.f_number_at_min_focal_length(),
            lens.f_number_at_max_focal_length(),
        ]
    }

    /// The count of a MakerNote that is all of `block`.
    fn whole(block: &[u8]) -> u64 {
        u64::try_from(block.len()).unwrap()
    }

    /// A Nikon file in byte order `order` whose Exif IFD holds what `exif`
    /// adds, given the builder and that IFD, and whose MakerNote, at
    /// [`AT`], is the first `count` bytes of `block`.
    fn exif_file(
        order: Order,
        block: &[u8],
        count: u64,
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Vec<u8> {
        nikon_file_with(order, AT, block, TAIL, |builder, _, exif_ifd| {
            exif(builder, exif_ifd);
            entry_at(builder, exif_ifd, MAKER_NOTE, UNDEFINED, AT, count);
        })
    }

    /// The metadata of the file [`exif_file`] builds in byte order `order`
    /// with the MakerNote `block`, declared whole, and what `exif` adds to
    /// the Exif IFD.
    fn exif_metadata(
        order: Order,
        block: &[u8],
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Metadata {
        metadata_of(exif_file(order, block, whole(block), exif))
    }

    /// The metadata of a file in byte order `order` whose MakerNote holds
    /// what `tags` adds to its MakerNote IFD, and whose Exif IFD holds no
    /// serial number, ISO or lens.
    fn makernote_metadata(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> Metadata {
        exif_metadata(order, &tags_block(order, tags).0, |_, _| {})
    }

    /// `block` with the type and count of the entry at `entry` changed to
    /// `typ` and `count`, its value left as it was.
    fn retyped(order: Order, mut block: Vec<u8>, entry: u64, typ: u16, count: u32) -> Vec<u8> {
        let entry = usize::try_from(entry).unwrap();
        block[entry + 2..entry + 4].copy_from_slice(&order.u16(typ));
        block[entry + 4..entry + 8].copy_from_slice(&order.u32(count));
        block
    }

    #[test]
    fn without_an_exif_serial_number_the_makernotes_is_used() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            });
            assert_eq!(metadata.serial_number.as_deref(), Some(SERIAL), "{order:?}");
        }
    }

    #[test]
    fn without_a_serial_number_in_either_there_is_none() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |_, _| {});
            assert_eq!(metadata.serial_number, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_serial_number_of_another_type_is_not_read() {
        for order in ORDERS {
            // The bytes of the serial number and its NUL, as UNDEFINED and
            // as BYTE.
            let mut bytes = SERIAL.as_bytes().to_vec();
            bytes.push(0);
            for typ in [UNDEFINED, BYTE] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
                });
                let count = u32::try_from(bytes.len()).unwrap();
                let block = retyped(order, block, entry, typ, count);
                let metadata = exif_metadata(order, &block, |_, _| {});
                assert_eq!(metadata.serial_number, None, "{order:?}, type {typ}");
            }
        }
    }

    #[test]
    fn a_makernote_serial_number_past_the_block_is_not_read() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            // The seven digits and the NUL are the block's last 8 bytes.
            assert_eq!(block[block.len() - 8..], *b"7654321\0", "{order:?}");

            // Declared one byte short: the NUL lies in the file, after the
            // block.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).serial_number, None, "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            assert_eq!(
                metadata_of(file).serial_number.as_deref(),
                Some(SERIAL),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_entry_whose_value_runs_past_the_block_is_dropped_when_read() {
        // As in the test above, but on the MakerNote IFD itself: no reader
        // of the IFD can reach the value, whatever it checks.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            for (count, kept) in [(whole(&block) - 1, false), (whole(&block), true)] {
                let file = exif_file(order, &block, count, |_, _| {});
                let mut src = Source::new(Cursor::new(file)).unwrap();
                let file_block = tiff::Block::file(src.len());
                let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
                let makernote = read(&mut src, &tree, &Limits::default())
                    .unwrap()
                    .expect("a MakerNote");
                let what = format!("{order:?}, declared {count} bytes");
                assert_eq!(makernote.ifd.entry(SERIAL_NUMBER).is_some(), kept, "{what}");
                assert_eq!(makernote.ifd.invalid_entries(), u32::from(!kept), "{what}");
                // MakerNoteVersion, whose four bytes the entry holds, stays.
                assert!(makernote.ifd.entry(MAKER_NOTE_VERSION).is_some(), "{what}");
            }
        }
    }

    #[test]
    fn an_exif_serial_number_wins_over_the_makernotes() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.ascii(exif, BODY_SERIAL_NUMBER, EXIF_SERIAL);
            });
            assert_eq!(
                metadata.serial_number.as_deref(),
                Some(EXIF_SERIAL),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_makernote_serial_number_of_spaces_is_none() {
        // As the D200 writes it: seven spaces and a NUL (public sample
        // 8079).
        let metadata = makernote_metadata(Order::Big, |inner, ifd| {
            inner.ascii(ifd, SERIAL_NUMBER, "       ");
        });
        assert_eq!(metadata.serial_number, None);
    }

    #[test]
    fn a_makernote_serial_number_that_is_not_digits_is_returned_as_written() {
        let metadata = makernote_metadata(Order::Big, |inner, ifd| {
            inner.ascii(ifd, SERIAL_NUMBER, "ABC123");
        });
        assert_eq!(metadata.serial_number.as_deref(), Some("ABC123"));
    }

    #[test]
    fn without_an_exif_iso_the_makernotes_second_value_is_used() {
        // As most bodies write it: `[0, ISO]`, here the D300's (public
        // sample 928).
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 200]);
            });
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    #[test]
    fn without_an_iso_in_either_there_is_none() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |_, _| {});
            assert_eq!(metadata.exposure.iso, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_iso_of_another_type_is_not_read() {
        for order in ORDERS {
            // `[0, 200]` as LONGs, stored out of line inside the block,
            // which the reader of numbers reads; and the same two numbers
            // declared as IFD offsets, which it reads too.
            let long = tags_block(order, |inner, ifd| {
                inner.long(ifd, ISO, &[0, 200]);
            });
            let ifds = retyped(order, long.0.clone(), long.1, IFD, 2);
            for (label, block) in [("LONG", long.0), ("IFD", ifds)] {
                let metadata = exif_metadata(order, &block, |_, _| {});
                assert_eq!(metadata.exposure.iso, None, "{order:?}, {label}");
            }
        }
    }

    #[test]
    fn a_makernote_iso_of_four_zero_bytes_is_none() {
        // As the D700 writes it: four zero bytes of type UNDEFINED (public
        // sample 2255).
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.undefined(ifd, ISO, &[0; 4]);
            });
            assert_eq!(metadata.exposure.iso, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_iso_of_zero_is_none() {
        // As the Coolpix E5700 writes it: `[0, 0]` (public sample 4456).
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 0]);
            });
            assert_eq!(metadata.exposure.iso, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_iso_of_other_than_two_values_is_none() {
        for order in ORDERS {
            // One value, which the entry holds, and three, stored out of
            // line inside the block.
            for values in [&[200][..], &[0, 200, 400]] {
                let metadata = makernote_metadata(order, |inner, ifd| {
                    inner.short(ifd, ISO, values);
                });
                assert_eq!(metadata.exposure.iso, None, "{order:?}, {values:?}");
            }
        }
    }

    #[test]
    fn a_makernote_isos_first_value_is_not_read() {
        // Most bodies write 0 first; the ISO is the second value whatever
        // the first is.
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.short(ifd, ISO, &[1, 3200]);
            });
            assert_eq!(metadata.exposure.iso, Some(3200), "{order:?}");
        }
    }

    #[test]
    fn an_exif_iso_wins_over_the_makernotes() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 200]);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[100]);
            });
            assert_eq!(metadata.exposure.iso, Some(100), "{order:?}");
        }
    }

    #[test]
    fn an_exif_iso_of_zero_alone_gives_the_makernotes() {
        // The standard tags give no ISO for a PhotographicSensitivity of 0,
        // an unknown sensitivity, without another sensitivity tag.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 200]);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[0]);
            });
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    #[test]
    fn without_an_exif_lens_the_makernotes_range_makes_one() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            });
            let lens = metadata.lens.expect("a lens");
            assert_eq!(range_of(&lens), known(D300_LENS), "{order:?}");
            assert_eq!(
                (lens.make(), lens.model(), lens.serial_number()),
                (None, None, None),
                "{order:?}"
            );
        }
    }

    #[test]
    fn without_a_lens_in_either_there_is_none() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |_, _| {});
            assert_eq!(metadata.lens, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_lens_of_another_type_is_not_read() {
        for order in ORDERS {
            // The 32 bytes of the four fractions, declared as SRATIONALs,
            // as eight LONGs, or as 32 UNDEFINED bytes.
            for (typ, count) in [(SRATIONAL, 4), (LONG, 8), (UNDEFINED, 32)] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.rational(ifd, LENS, &D300_LENS);
                });
                let block = retyped(order, block, entry, typ, count);
                let metadata = exif_metadata(order, &block, |_, _| {});
                assert_eq!(metadata.lens, None, "{order:?}, type {typ}");
            }
        }
    }

    #[test]
    fn a_makernote_lens_past_the_block_is_not_read() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            })
            .0;
            // The four fractions are the block's last 32 bytes.
            let values: Vec<u8> = D300_LENS
                .iter()
                .flat_map(|&(n, d)| [order.u32(n), order.u32(d)])
                .flatten()
                .collect();
            assert_eq!(block[block.len() - 32..], values, "{order:?}");

            // Declared one byte short: the last fraction's last byte lies
            // in the file, after the block.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).lens, None, "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            let lens = metadata_of(file).lens.expect("a lens");
            assert_eq!(range_of(&lens), known(D300_LENS), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_range_wins_over_the_makernotes() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.rational(exif, LENS_SPECIFICATION, &EXIF_LENS);
            });
            let lens = metadata.lens.expect("a lens");
            assert_eq!(range_of(&lens), known(EXIF_LENS), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_range_with_one_known_value_wins_whole() {
        // Not merged value by value: the three unknown values stay unknown.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            })
            .0;
            let exif_range = [(240, 10), (0, 0), (0, 0), (0, 0)];
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.rational(exif, LENS_SPECIFICATION, &exif_range);
            });
            let lens = metadata.lens.expect("a lens");
            assert_eq!(range_of(&lens), known(exif_range), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_with_a_model_alone_takes_the_makernotes_range() {
        // As the Nikon 1 J1 writes it: a LensModel and no LensSpecification
        // in the Exif tags, the range in the MakerNote (public sample 2956).
        for order in ORDERS {
            let j1_range = [(100, 10), (300, 10), (35, 10), (56, 10)];
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &j1_range);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.ascii(exif, LENS_MODEL, "1 NIKKOR VR 10-30mm f/3.5-5.6");
            });
            let lens = metadata.lens.expect("a lens");
            assert_eq!(
                lens.model(),
                Some("1 NIKKOR VR 10-30mm f/3.5-5.6"),
                "{order:?}"
            );
            assert_eq!(range_of(&lens), known(j1_range), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_lens_of_other_than_four_values_is_not_read() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS[..3]);
            });
            assert_eq!(metadata.lens, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_lens_value_of_zero_is_none_and_the_others_are_kept() {
        // An unknown value written as 0/0, and as a zero over a denominator.
        for zero in [(0, 0), (0, 10)] {
            for order in ORDERS {
                let values = [(180, 10), (700, 10), zero, (45, 10)];
                let metadata = makernote_metadata(order, |inner, ifd| {
                    inner.rational(ifd, LENS, &values);
                });
                let lens = metadata.lens.expect("a lens");
                assert_eq!(
                    range_of(&lens),
                    [
                        Rational::new(180, 10),
                        Rational::new(700, 10),
                        None,
                        Rational::new(45, 10)
                    ],
                    "{zero:?} {order:?}"
                );
            }
        }
    }

    #[test]
    fn the_fallbacks_in_the_other_byte_order_from_the_files_are_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner
                    .short(ifd, ISO, &[0, 200])
                    .ascii(ifd, SERIAL_NUMBER, SERIAL)
                    .rational(ifd, LENS, &D300_LENS);
            })
            .0;
            let metadata = exif_metadata(file, &block, |_, _| {});
            assert_eq!(metadata.exposure.iso, Some(200), "{file:?}");
            assert_eq!(metadata.serial_number.as_deref(), Some(SERIAL), "{file:?}");
            let lens = metadata.lens.expect("a lens");
            assert_eq!(range_of(&lens), known(D300_LENS), "{file:?}");
        }
    }

    /// The DateTimeOriginal the WorldTime tests give the Exif IFD: the
    /// D300 fixture's, made up.
    const DATE: &str = "2008:05:17 10:22:31";

    /// [`DATE`] with the SubSecTimeOriginal `34`, as shown without an
    /// offset.
    const LOCAL: &str = "2008-05-17T10:22:31.34";

    /// WorldTime's four bytes in byte order `order`: the time zone `zone`,
    /// in minutes east of UTC, the daylight-saving byte `daylight_saving`,
    /// and the date format 0.
    fn world_time_bytes(order: Order, zone: i16, daylight_saving: u8) -> Vec<u8> {
        let mut bytes = order.u16(zone.cast_unsigned()).to_vec();
        bytes.extend_from_slice(&[daylight_saving, 0]);
        bytes
    }

    /// The capture time of a file in byte order `file` whose MakerNote is
    /// `block`, declared whole, and whose Exif IFD holds what `exif` adds.
    fn capture_time_of(
        file: Order,
        block: &[u8],
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Option<DateTime> {
        exif_metadata(file, block, exif).capture_time
    }

    /// The capture time of a file in byte order `order`, MakerNote alike,
    /// whose Exif IFD holds [`DATE`] and its sub-seconds, with the
    /// OffsetTimeOriginal `offset` when there is one, and whose MakerNote
    /// holds what `tags` adds; the time must be read.
    fn time_with(
        order: Order,
        offset: Option<&str>,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> DateTime {
        let block = tags_block(order, tags).0;
        capture_time_of(order, &block, |builder, exif| {
            builder
                .ascii(exif, DATE_TIME_ORIGINAL, DATE)
                .ascii(exif, SUB_SEC_TIME_ORIGINAL, "34");
            if let Some(offset) = offset {
                builder.ascii(exif, OFFSET_TIME_ORIGINAL, offset);
            }
        })
        .expect("a capture time")
    }

    /// The capture time of a file in byte order `order` with [`DATE`], no
    /// OffsetTimeOriginal, and a WorldTime of the time zone `zone` and the
    /// daylight-saving byte `daylight_saving`.
    fn zone_time(order: Order, zone: i16, daylight_saving: u8) -> DateTime {
        time_with(order, None, |inner, ifd| {
            inner.undefined(
                ifd,
                WORLD_TIME,
                &world_time_bytes(order, zone, daylight_saving),
            );
        })
    }

    #[test]
    fn without_an_exif_offset_world_time_gives_it() {
        // The D300's zone, as on public sample 928; the zone of samples
        // 6887 and 7808 to 7810 with daylight saving time on, which agrees
        // with their Exif -04:00; one of daylight saving time on west of
        // UTC, as on sample 961; and UTC itself.
        for order in ORDERS {
            for (zone, daylight_saving, minutes, display) in [
                (60, 0, 60, "+01:00"),
                (-300, 1, -240, "-04:00"),
                (-480, 1, -420, "-07:00"),
                (0, 0, 0, "+00:00"),
            ] {
                let time = zone_time(order, zone, daylight_saving);
                let what = format!("{order:?}, {zone}, {daylight_saving}");
                assert_eq!(time.offset_minutes(), Some(minutes), "{what}");
                assert_eq!(time.to_string(), format!("{LOCAL}{display}"), "{what}");
            }
        }
    }

    #[test]
    fn a_world_time_offset_of_up_to_fourteen_hours_is_taken() {
        for order in ORDERS {
            for (zone, daylight_saving, display) in
                [(840, 0, "+14:00"), (780, 1, "+14:00"), (-840, 0, "-14:00")]
            {
                let time = zone_time(order, zone, daylight_saving);
                assert_eq!(
                    time.to_string(),
                    format!("{LOCAL}{display}"),
                    "{order:?}, {zone}, {daylight_saving}"
                );
            }
        }
    }

    #[test]
    fn a_world_time_offset_beyond_fourteen_hours_gives_none() {
        // With the daylight hour too: 840 plus 60 is 15 hours. And at the
        // end of the 16-bit range, where adding the hour would overflow it.
        for order in ORDERS {
            for (zone, daylight_saving) in
                [(841, 0), (840, 1), (-841, 0), (i16::MAX, 1), (i16::MIN, 0)]
            {
                let time = zone_time(order, zone, daylight_saving);
                let what = format!("{order:?}, {zone}, {daylight_saving}");
                assert_eq!(time.offset_minutes(), None, "{what}");
                assert_eq!(time.to_string(), LOCAL, "{what}");
            }
        }
    }

    #[test]
    fn a_byte_swapped_world_time_gives_none() {
        // As editing software can rewrite it: the zone's two bytes in the
        // other order from the MakerNote's, so 60 reads as 15360, and
        // -300 as -11010.
        for (order, swapped) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            for zone in [60, -300] {
                let time = time_with(order, None, |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, &world_time_bytes(swapped, zone, 0));
                });
                assert_eq!(time.offset_minutes(), None, "{order:?}, {zone}");
                assert_eq!(time.to_string(), LOCAL, "{order:?}, {zone}");
            }
        }
    }

    #[test]
    fn a_world_time_daylight_saving_byte_of_another_value_gives_none() {
        for order in ORDERS {
            for daylight_saving in [2, 255] {
                let time = zone_time(order, 60, daylight_saving);
                assert_eq!(time.offset_minutes(), None, "{order:?}, {daylight_saving}");
            }
        }
    }

    #[test]
    fn an_exif_offset_wins_over_world_time() {
        for order in ORDERS {
            let time = time_with(order, Some("-05:00"), |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
            });
            assert_eq!(time.to_string(), format!("{LOCAL}-05:00"), "{order:?}");
        }
    }

    #[test]
    fn an_unreadable_exif_offset_gives_way_to_world_time() {
        // The standard tags keep the time and drop an offset they cannot
        // read, so they give none, and WorldTime gives it.
        for order in ORDERS {
            let time = time_with(order, Some("+5:00"), |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
            });
            assert_eq!(time.to_string(), format!("{LOCAL}+01:00"), "{order:?}");
        }
    }

    #[test]
    fn without_a_date_time_original_world_time_gives_no_capture_time() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
            })
            .0;
            let time = capture_time_of(order, &block, |builder, exif| {
                builder.ascii(exif, SUB_SEC_TIME_ORIGINAL, "34");
            });
            assert_eq!(time, None, "{order:?}");
        }
    }

    #[test]
    fn a_world_time_of_another_type_or_length_gives_none() {
        for order in ORDERS {
            // The four bytes of a zone of 60 without daylight saving time,
            // declared as four BYTEs or as two SHORTs.
            for (typ, count) in [(BYTE, 4), (SHORT, 2)] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
                });
                let block = retyped(order, block, entry, typ, count);
                let time = capture_time_of(order, &block, |builder, exif| {
                    builder.ascii(exif, DATE_TIME_ORIGINAL, DATE);
                })
                .expect("a capture time");
                assert_eq!(time.offset_minutes(), None, "{order:?}, type {typ}");
            }
            // Three of those bytes, held in the entry, and the four and one
            // more, stored elsewhere inside the block.
            let bytes = world_time_bytes(order, 60, 0);
            let mut five = bytes.clone();
            five.push(0);
            for value in [&bytes[..3], &five] {
                let time = time_with(order, None, |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, value);
                });
                assert_eq!(time.offset_minutes(), None, "{order:?}, {value:?}");
            }
        }
    }

    #[test]
    fn without_world_time_the_time_has_no_offset() {
        // As the D200 and older bodies write it (public sample 8079).
        for order in ORDERS {
            let time = time_with(order, None, |_, _| {});
            assert_eq!(time.offset_minutes(), None, "{order:?}");
            assert_eq!(time.to_string(), LOCAL, "{order:?}");
        }
    }

    #[test]
    fn a_world_time_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(inner_order, -300, 1));
            })
            .0;
            let time = capture_time_of(file, &block, |builder, exif| {
                builder.ascii(exif, DATE_TIME_ORIGINAL, DATE);
            })
            .expect("a capture time");
            assert_eq!(time.to_string(), "2008-05-17T10:22:31-04:00", "{file:?}");
        }
    }

    /// Where a test's AFInfo2 value puts each field: the value's length,
    /// then the byte offsets of the frame, the centre, the area and the
    /// in-focus byte, as the NEF notes' table gives them for each version.
    /// Written out here, apart from [`layout`], so that a wrong offset there
    /// reads a field from the wrong place.
    #[derive(Clone, Copy, Debug)]
    struct Fields {
        len: usize,
        frame: usize,
        center: usize,
        area: usize,
        in_focus: Option<usize>,
    }

    /// The fields of version 0100.
    const FIELDS_0100: Fields = Fields {
        len: 30,
        frame: 16,
        center: 20,
        area: 24,
        in_focus: Some(28),
    };

    /// The fields of version 0101.
    const FIELDS_0101: Fields = Fields {
        len: 84,
        frame: 70,
        center: 74,
        area: 78,
        in_focus: Some(82),
    };

    /// The fields of versions 0300 and 0301, which have no in-focus byte.
    const FIELDS_03XX: Fields = Fields {
        len: 60,
        frame: 42,
        center: 46,
        area: 50,
        in_focus: None,
    };

    /// The fields of versions 0400, 0401 and 0402.
    const FIELDS_04XX: Fields = Fields {
        len: 80,
        frame: 62,
        center: 66,
        area: 70,
        in_focus: Some(74),
    };

    /// The fields of the AFInfo2 version `version`, one of those read.
    fn fields_of(version: &str) -> Fields {
        match version {
            "0100" => FIELDS_0100,
            "0101" => FIELDS_0101,
            "0300" | "0301" => FIELDS_03XX,
            "0400" | "0401" | "0402" => FIELDS_04XX,
            _ => panic!("no fields for version {version}"),
        }
    }

    /// What an AFInfo2 value records: the frame's width and height, the
    /// centre's x and y, the area's width and height, and the in-focus
    /// byte, written only where the version has one.
    #[derive(Clone, Copy, Debug)]
    struct Recorded {
        frame: [u16; 2],
        center: [u16; 2],
        area: [u16; 2],
        in_focus: u8,
    }

    /// The Coolpix A's record, version 0100: a live-view shot focused at
    /// the middle of its frame, and confirmed (public sample 2238).
    const COOLPIX_A: Recorded = Recorded {
        frame: [4928, 3264],
        center: [2464, 1632],
        area: [408, 336],
        in_focus: 1,
    };

    /// A record for version 0101, made up, since no public sample fills in
    /// its position: a D850-sized frame, off-centre, confirmed.
    const MADE_UP_0101: Recorded = Recorded {
        frame: [8256, 5504],
        center: [3000, 2000],
        area: [400, 400],
        in_focus: 1,
    };

    /// The Z 6's record, version 0300, which has no in-focus byte (public
    /// sample 3587).
    const Z6: Recorded = Recorded {
        frame: [6048, 4024],
        center: [3024, 1160],
        area: [285, 308],
        in_focus: 1,
    };

    /// The Z 9's record, version 0400, confirmed (public samples 5146 to
    /// 5148).
    const Z9: Recorded = Recorded {
        frame: [8256, 5504],
        center: [4905, 2461],
        area: [291, 323],
        in_focus: 1,
    };

    /// An AFInfo2 value in byte order `order`, `fields.len` bytes long: the
    /// four bytes of `version`, then zeros, with `recorded` at the offsets
    /// of `fields`, its in-focus byte only where `fields` has one.
    fn af_info2_with(order: Order, version: &str, fields: Fields, recorded: Recorded) -> Vec<u8> {
        let mut value = vec![0; fields.len];
        value[..4].copy_from_slice(version.as_bytes());
        for (at, [first, second]) in [
            (fields.frame, recorded.frame),
            (fields.center, recorded.center),
            (fields.area, recorded.area),
        ] {
            value[at..at + 2].copy_from_slice(&order.u16(first));
            value[at + 2..at + 4].copy_from_slice(&order.u16(second));
        }
        if let Some(at) = fields.in_focus {
            value[at] = recorded.in_focus;
        }
        value
    }

    /// An AFInfo2 value of version `version`, one of those read, in byte
    /// order `order`, as long as its layout, recording `recorded`.
    fn af_info2(order: Order, version: &str, recorded: Recorded) -> Vec<u8> {
        af_info2_with(order, version, fields_of(version), recorded)
    }

    /// The focus that `recorded` describes, with `in_focus` as whether
    /// focus was confirmed.
    fn expected(recorded: Recorded, in_focus: Option<bool>) -> Focus {
        let [width, height] = recorded.frame.map(u32::from);
        let [x, y] = recorded.center.map(u32::from);
        let [area_width, area_height] = recorded.area.map(u32::from);
        let focus = Focus::new(Size { width, height }, Point { x, y })
            .and_then(|focus| {
                focus.with_size(Size {
                    width: area_width,
                    height: area_height,
                })
            })
            .expect("a usable record");
        match in_focus {
            Some(in_focus) => focus.with_in_focus(in_focus),
            None => focus,
        }
    }

    /// The focus of a file in byte order `order` whose MakerNote holds what
    /// `tags` adds to its MakerNote IFD.
    fn focus_of(order: Order, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> Option<Focus> {
        makernote_metadata(order, tags).focus
    }

    /// The focus of a file in byte order `order` whose MakerNote holds the
    /// AFInfo2 value `value`, as UNDEFINED, and no FocusMode.
    fn focus_from(order: Order, value: &[u8]) -> Option<Focus> {
        focus_of(order, |inner, ifd| {
            inner.undefined(ifd, AF_INFO2, value);
        })
    }

    #[test]
    fn each_af_info2_layout_gives_the_focus() {
        // The Coolpix A's 0100, a made-up 0101, the Z 6's 0300, which does
        // not say whether focus was confirmed, and the Z 9's 0400.
        for order in ORDERS {
            for (version, recorded, in_focus) in [
                ("0100", COOLPIX_A, Some(true)),
                ("0101", MADE_UP_0101, Some(true)),
                ("0300", Z6, None),
                ("0400", Z9, Some(true)),
            ] {
                assert_eq!(
                    focus_from(order, &af_info2(order, version, recorded)),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {version}"
                );
            }
        }
    }

    #[test]
    fn the_other_versions_of_a_layout_read_like_it() {
        // 0301, which the D6, the Z 5 and the Z 6II write, and 0401 and
        // 0402, which the Z f, the Z6III, the Z50II and the Z5II write.
        for order in ORDERS {
            for (version, recorded, in_focus) in [
                ("0301", Z6, None),
                ("0401", Z9, Some(true)),
                ("0402", Z9, Some(true)),
            ] {
                assert_eq!(
                    focus_from(order, &af_info2(order, version, recorded)),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {version}"
                );
            }
        }
    }

    #[test]
    fn an_af_info2_version_not_read_gives_none() {
        // The Nikon 1's 0200 and 0201, 42 bytes long as the J1 writes it
        // (public sample 2956), with a position at the 0100 offsets; then
        // versions no file shows, each laid out as the read version nearest
        // it, which a reader choosing by the first two digits would read;
        // then bytes that are no version at all.
        for order in ORDERS {
            let nikon_1 = Fields {
                len: 42,
                ..FIELDS_0100
            };
            for (version, fields, recorded) in [
                ("0200", nikon_1, COOLPIX_A),
                ("0201", nikon_1, COOLPIX_A),
                ("0102", FIELDS_0101, MADE_UP_0101),
                ("0302", FIELDS_03XX, Z6),
                ("0403", FIELDS_04XX, Z9),
                ("0500", FIELDS_04XX, Z9),
                ("\0\0\0\0", FIELDS_04XX, Z9),
                ("400 ", FIELDS_04XX, Z9),
            ] {
                let value = af_info2_with(order, version, fields, recorded);
                assert_eq!(focus_from(order, &value), None, "{order:?}, {version:?}");
            }
            // Too short to hold a version, held in the entry itself.
            assert_eq!(focus_from(order, b"040"), None, "{order:?}");
        }
    }

    #[test]
    fn an_af_info2_shorter_than_its_layout_gives_none() {
        // Each layout ends past the last field read, so one byte short, the
        // fields still lie inside the value; exactly its length gives the
        // focus.
        for order in ORDERS {
            for (version, recorded, in_focus) in [
                ("0100", COOLPIX_A, Some(true)),
                ("0101", MADE_UP_0101, Some(true)),
                ("0300", Z6, None),
                ("0400", Z9, Some(true)),
            ] {
                let value = af_info2(order, version, recorded);
                assert_eq!(
                    focus_from(order, &value),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {version}, {} bytes",
                    value.len()
                );
                let short = &value[..value.len() - 1];
                assert_eq!(
                    focus_from(order, short),
                    None,
                    "{order:?}, {version}, {} bytes",
                    short.len()
                );
            }
        }
    }

    #[test]
    fn an_af_info2_without_a_usable_position_gives_none() {
        for order in ORDERS {
            for (what, recorded) in [
                // As the D300 writes it through the viewfinder: zeros past
                // the version (public sample 928).
                (
                    "everything zero",
                    Recorded {
                        frame: [0, 0],
                        center: [0, 0],
                        area: [0, 0],
                        in_focus: 0,
                    },
                ),
                (
                    "a zero frame width",
                    Recorded {
                        frame: [0, 5504],
                        ..Z9
                    },
                ),
                (
                    "a zero frame height",
                    Recorded {
                        frame: [8256, 0],
                        ..Z9
                    },
                ),
                (
                    "a zero area width",
                    Recorded {
                        area: [0, 323],
                        ..Z9
                    },
                ),
                (
                    "a zero area height",
                    Recorded {
                        area: [291, 0],
                        ..Z9
                    },
                ),
                (
                    "a centre right of the frame",
                    Recorded {
                        center: [8257, 2461],
                        ..Z9
                    },
                ),
                (
                    "a centre below the frame",
                    Recorded {
                        center: [4905, 5505],
                        ..Z9
                    },
                ),
                // An area 291 pixels wide needs 146 on each side of the
                // centre, and one 323 high 162 above and below it.
                (
                    "an area past the left edge",
                    Recorded {
                        center: [145, 2461],
                        ..Z9
                    },
                ),
                (
                    "an area past the bottom edge",
                    Recorded {
                        center: [4905, 5504 - 161],
                        ..Z9
                    },
                ),
            ] {
                let value = af_info2(order, "0400", recorded);
                assert_eq!(focus_from(order, &value), None, "{order:?}, {what}");
            }
            // The control: the same areas just fit one pixel further in.
            for center in [[146, 2461], [4905, 5504 - 162]] {
                let recorded = Recorded { center, ..Z9 };
                assert_eq!(
                    focus_from(order, &af_info2(order, "0400", recorded)),
                    Some(expected(recorded, Some(true))),
                    "{order:?}, {center:?}"
                );
            }
        }
    }

    #[test]
    fn an_af_info2_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &af_info2(inner_order, "0400", Z9));
            })
            .0;
            let metadata = exif_metadata(file, &block, |_, _| {});
            assert_eq!(metadata.focus, Some(expected(Z9, Some(true))), "{file:?}");
        }
    }

    #[test]
    fn a_byte_swapped_af_info2_gives_none() {
        // As in the other byte order from the MakerNote's: the Coolpix A's
        // frame reads as 16403 by 49164, the Z 9's as 16416 by 32789, and
        // each centre then lies outside it.
        for (order, swapped) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            for (version, recorded) in [("0100", COOLPIX_A), ("0400", Z9)] {
                let value = af_info2(swapped, version, recorded);
                assert_eq!(focus_from(order, &value), None, "{order:?}, {version}");
            }
        }
    }

    #[test]
    fn the_in_focus_byte_says_whether_focus_was_confirmed() {
        // 1 is confirmed, 0 a miss, as on the Z6III samples beside a valid
        // point (public samples 7808 to 7810); any other value says nothing,
        // and the point is kept.
        for order in ORDERS {
            for (version, recorded) in [("0100", COOLPIX_A), ("0101", MADE_UP_0101), ("0400", Z9)] {
                for (byte, in_focus) in [(1, Some(true)), (0, Some(false)), (2, None), (255, None)]
                {
                    let recorded = Recorded {
                        in_focus: byte,
                        ..recorded
                    };
                    assert_eq!(
                        focus_from(order, &af_info2(order, version, recorded)),
                        Some(expected(recorded, in_focus)),
                        "{order:?}, {version}, {byte}"
                    );
                }
            }
        }
    }

    #[test]
    fn versions_0300_and_0301_do_not_say_whether_focus_was_confirmed() {
        // Every byte but the version and the position set to 1, wherever an
        // in-focus byte might be looked for.
        for order in ORDERS {
            for version in ["0300", "0301"] {
                let mut value = af_info2(order, version, Z6);
                value[4..FIELDS_03XX.frame].fill(1);
                value[FIELDS_03XX.area + 4..].fill(1);
                assert_eq!(
                    focus_from(order, &value),
                    Some(expected(Z6, None)),
                    "{order:?}, {version}"
                );
            }
        }
    }

    #[test]
    fn an_af_info2_past_the_block_is_not_read() {
        for order in ORDERS {
            let value = af_info2(order, "0400", Z9);
            let block = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &value);
            })
            .0;
            // The value is the block's last 80 bytes.
            assert_eq!(block[block.len() - 80..], value, "{order:?}");

            // Declared one byte short: the value's last byte lies in the
            // file, after the block.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).focus, None, "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            assert_eq!(
                metadata_of(file).focus,
                Some(expected(Z9, Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_af_info2_of_another_type_is_not_read() {
        // The Z 9's 80 bytes, declared as BYTEs, which the reader of bytes
        // reads too.
        for order in ORDERS {
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9));
            });
            let block = retyped(order, block, entry, BYTE, 80);
            let metadata = exif_metadata(order, &block, |_, _| {});
            assert_eq!(metadata.focus, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_without_af_info2_gives_no_focus() {
        // As the D1H, D70, D100, D200 and older Coolpix bodies write it.
        for order in ORDERS {
            assert_eq!(focus_of(order, |_, _| {}), None, "{order:?}");
        }
    }

    #[test]
    fn a_focus_mode_of_manual_gives_no_focus() {
        // `MANUAL` and the NUL the builder adds, as the D300 and the D850
        // write it (public samples 928 and 1835), and padded with spaces as
        // an autofocus mode is, beside the Z 9's AFInfo2.
        for order in ORDERS {
            for mode in ["MANUAL", "MANUAL  "] {
                let focus = focus_of(order, |inner, ifd| {
                    inner.ascii(ifd, FOCUS_MODE, mode).undefined(
                        ifd,
                        AF_INFO2,
                        &af_info2(order, "0400", Z9),
                    );
                });
                assert_eq!(focus, None, "{order:?}, {mode:?}");
            }
        }
    }

    #[test]
    fn a_focus_mode_other_than_manual_keeps_the_focus() {
        // Autofocus modes, padded as the bodies write them; `manual` in
        // lower case, which the exact rule does not take for `MANUAL`; and
        // no FocusMode at all.
        for order in ORDERS {
            for mode in [Some("AF-S  "), Some("AF-C  "), Some("manual"), None] {
                let focus = focus_of(order, |inner, ifd| {
                    if let Some(mode) = mode {
                        inner.ascii(ifd, FOCUS_MODE, mode);
                    }
                    inner.undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9));
                });
                assert_eq!(focus, Some(expected(Z9, Some(true))), "{order:?}, {mode:?}");
            }
        }
    }

    #[test]
    fn a_focus_mode_of_another_type_is_not_read() {
        // `MANUAL` and its NUL as seven UNDEFINED bytes: not FocusMode's
        // type, so no manual focus is recorded, and the focus is kept.
        for order in ORDERS {
            let (block, entry) = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, FOCUS_MODE, "MANUAL").undefined(
                    ifd,
                    AF_INFO2,
                    &af_info2(order, "0400", Z9),
                );
            });
            let block = retyped(order, block, entry, UNDEFINED, 7);
            let metadata = exif_metadata(order, &block, |_, _| {});
            assert_eq!(metadata.focus, Some(expected(Z9, Some(true))), "{order:?}");
        }
    }

    /// What a crop test's file holds besides AFInfo2: the size of its raw
    /// data, its CropHiSpeed's seven values, when it has one, and its
    /// CropArea, when it has one, both as SHORTs.
    #[derive(Clone, Copy, Debug)]
    struct Crop {
        raw: [u32; 2],
        crop_hi_speed: Option<[u16; 7]>,
        crop_area: Option<[u16; 4]>,
    }

    /// The Z6III's 1:1 crop (public samples 7808 and 7812).
    const Z6III_1X1: Crop = Crop {
        raw: [4048, 4040],
        crop_hi_speed: Some([17, 6064, 4040, 4048, 4040, 1008, 0]),
        crop_area: Some([8, 4, 4032, 4032]),
    };

    /// The Z6III's 16:9 crop (public samples 7809 and 7813).
    const Z6III_16X9: Crop = Crop {
        raw: [6064, 3416],
        crop_hi_speed: Some([6, 6064, 4040, 6064, 3416, 0, 312]),
        crop_area: Some([8, 8, 6048, 3400]),
    };

    /// The Z6III's DX crop (public sample 7810).
    const Z6III_DX: Crop = Crop {
        raw: [4000, 2672],
        crop_hi_speed: Some([2, 6064, 4040, 4000, 2672, 1032, 684]),
        crop_area: Some([8, 8, 3984, 2656]),
    };

    /// The Z50II's 1:1 crop (public sample 7767).
    const Z50II_1X1: Crop = Crop {
        raw: [3728, 3728],
        crop_hi_speed: Some([18, 5600, 3728, 3728, 3728, 936, 0]),
        crop_area: Some([8, 8, 3712, 3712]),
    };

    /// The Z50II's 16:9 crop (public sample 7770).
    const Z50II_16X9: Crop = Crop {
        raw: [5600, 3144],
        crop_hi_speed: Some([9, 5600, 3728, 5600, 3144, 0, 292]),
        crop_area: Some([16, 8, 5568, 3128]),
    };

    /// The Z 6's small raw, which is no crop: CropHiSpeed's two sizes are
    /// equal, and CropArea counts the stored pixels (public sample 3587).
    const Z6_SMALL_RAW: Crop = Crop {
        raw: [4548, 3030],
        crop_hi_speed: Some([11, 6064, 4040, 6064, 4040, 0, 0]),
        crop_area: Some([10, 6, 4528, 3018]),
    };

    /// The Z 9's FX mode, which is no crop, as shots made with the
    /// maintainer's Z 9 record it.
    const Z9_FX: Crop = Crop {
        raw: [8280, 5520],
        crop_hi_speed: Some([11, 8280, 5520, 8280, 5520, 0, 0]),
        crop_area: Some([12, 8, 8256, 5504]),
    };

    /// The Z 9's DX crop, as shots made with the maintainer's Z 9 record
    /// it.
    const Z9_DX: Crop = Crop {
        raw: [5408, 3608],
        crop_hi_speed: Some([2, 8280, 5520, 5408, 3608, 1436, 956]),
        crop_area: Some([8, 8, 5392, 3592]),
    };

    /// The Z 9's 1:1 crop, as shots made with the maintainer's Z 9 record
    /// it.
    const Z9_1X1: Crop = Crop {
        raw: [5520, 5520],
        crop_hi_speed: Some([17, 8280, 5520, 5520, 5520, 1380, 0]),
        crop_area: Some([8, 8, 5504, 5504]),
    };

    /// The Z 9's 16:9 crop, as shots made with the maintainer's Z 9 record
    /// it.
    const Z9_16X9: Crop = Crop {
        raw: [8280, 4656],
        crop_hi_speed: Some([6, 8280, 5520, 8280, 4656, 0, 432]),
        crop_area: Some([12, 8, 8256, 4640]),
    };

    /// The record of the Z6III's 1:1 and 16:9 files, version 0401, a valid
    /// point beside a missed focus (public samples 7808, 7809, 7812 and
    /// 7813).
    const Z6III_ASPECT: Recorded = Recorded {
        frame: [6048, 4032],
        center: [3023, 2016],
        area: [163, 174],
        in_focus: 0,
    };

    /// The record of the Z6III's DX file, version 0401 (public sample
    /// 7810).
    const Z6III_DX_RECORD: Recorded = Recorded {
        area: [222, 245],
        ..Z6III_ASPECT
    };

    /// The record of the Z50II's 1:1 file, version 0402 (public sample
    /// 7767).
    const Z50II_1X1_RECORD: Recorded = Recorded {
        frame: [5568, 3712],
        center: [2792, 1868],
        area: [278, 276],
        in_focus: 1,
    };

    /// The record of the Z50II's 16:9 file, version 0402 (public sample
    /// 7770).
    const Z50II_16X9_RECORD: Recorded = Recorded {
        frame: [5568, 3712],
        center: [2789, 1865],
        area: [280, 280],
        in_focus: 1,
    };

    /// The records of the shots made with the maintainer's Z 9, version
    /// 0400, with a single point placed in the top-left corner of the
    /// picture in each mode: FX, DX, 1:1 and 16:9.
    const Z9_FX_CORNER: Recorded = Recorded {
        frame: [8256, 5504],
        center: [502, 424],
        area: [291, 323],
        in_focus: 1,
    };

    /// See [`Z9_FX_CORNER`].
    const Z9_DX_CORNER: Recorded = Recorded {
        center: [636, 572],
        area: [420, 468],
        ..Z9_FX_CORNER
    };

    /// See [`Z9_FX_CORNER`].
    const Z9_1X1_CORNER: Recorded = Recorded {
        center: [1797, 424],
        ..Z9_FX_CORNER
    };

    /// See [`Z9_FX_CORNER`].
    const Z9_16X9_CORNER: Recorded = Recorded {
        center: [502, 715],
        ..Z9_FX_CORNER
    };

    /// A MakerNote block in byte order `order` holding `crop`'s
    /// CropHiSpeed and CropArea, each when it has one, and the AFInfo2
    /// value `af_info2`, as UNDEFINED.
    fn crop_block(order: Order, crop: Crop, af_info2: &[u8]) -> Vec<u8> {
        tags_block(order, |inner, ifd| {
            if let Some(values) = crop.crop_hi_speed {
                inner.short(ifd, CROP_HI_SPEED, &values);
            }
            if let Some(values) = crop.crop_area {
                inner.short(ifd, CROP_AREA, &values);
            }
            inner.undefined(ifd, AF_INFO2, af_info2);
        })
        .0
    }

    /// The focus of a file in byte order `order` whose raw data is of size
    /// `raw` and whose MakerNote is the first `count` bytes of `block`; the
    /// raw size must be read, since CropArea is checked against it.
    fn focus_in(order: Order, raw: [u32; 2], block: &[u8], count: u64) -> Option<Focus> {
        let [width, height] = raw;
        let size = Size { width, height };
        let metadata = metadata_of(raw_file_of(order, size, block, count, |_, _| {}));
        assert_eq!(metadata.raw_size, Some(size), "{order:?}");
        metadata.focus
    }

    /// The focus of a file in byte order `order` that holds `crop` and an
    /// AFInfo2 of version `version` recording `recorded`.
    fn crop_focus(order: Order, crop: Crop, version: &str, recorded: Recorded) -> Option<Focus> {
        let block = crop_block(order, crop, &af_info2(order, version, recorded));
        focus_in(order, crop.raw, &block, whole(&block))
    }

    /// The focus of `frame`, `center` and `area`, with `in_focus` as
    /// whether focus was confirmed.
    fn focus_at(
        frame: [u16; 2],
        center: [u16; 2],
        area: [u16; 2],
        in_focus: Option<bool>,
    ) -> Focus {
        let recorded = Recorded {
            frame,
            center,
            area,
            in_focus: 0,
        };
        expected(recorded, in_focus)
    }

    /// `value` on an axis `old` pixels long, on one `new` pixels long, by
    /// the rounding the DX crops are scaled with, written out here apart
    /// from [`Focus::scaled_to`]: to the nearest whole number, a half
    /// rounding up.
    fn scale_by_hand(value: u16, old: u16, new: u16) -> u16 {
        let (value, old, new) = (u64::from(value), u64::from(old), u64::from(new));
        u16::try_from((value * new + old / 2) / old).unwrap()
    }

    #[test]
    fn the_crop_samples_give_the_point_on_the_cropped_picture() {
        // On each, the frame becomes CropArea's size. The 1:1 and 16:9
        // crops keep one side: the centre moves back on the other axis by
        // half the difference between the frame and CropArea, 1008 across
        // on the Z6III's 1:1 and 316 down on its 16:9, 928 and 292 on the
        // Z50II's, and the area is kept. The DX crop cuts both sides, and
        // the centre and the area are scaled onto CropArea.
        for order in ORDERS {
            for (what, crop, version, recorded, focus) in [
                (
                    "Z6III 1:1",
                    Z6III_1X1,
                    "0401",
                    Z6III_ASPECT,
                    focus_at([4032, 4032], [2015, 2016], [163, 174], Some(false)),
                ),
                (
                    "Z6III 16:9",
                    Z6III_16X9,
                    "0401",
                    Z6III_ASPECT,
                    focus_at([6048, 3400], [3023, 1700], [163, 174], Some(false)),
                ),
                (
                    "Z6III DX",
                    Z6III_DX,
                    "0401",
                    Z6III_DX_RECORD,
                    focus_at([3984, 2656], [1991, 1328], [146, 161], Some(false)),
                ),
                (
                    "Z50II 1:1",
                    Z50II_1X1,
                    "0402",
                    Z50II_1X1_RECORD,
                    focus_at([3712, 3712], [1864, 1868], [278, 276], Some(true)),
                ),
                (
                    "Z50II 16:9",
                    Z50II_16X9,
                    "0402",
                    Z50II_16X9_RECORD,
                    focus_at([5568, 3128], [2789, 1573], [280, 280], Some(true)),
                ),
            ] {
                assert_eq!(
                    crop_focus(order, crop, version, recorded),
                    Some(focus),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn the_z9_shots_with_the_point_in_the_corner_keep_it_there_on_every_crop() {
        // Shots made with the maintainer's Z 9, a single point placed in
        // the top-left corner of the picture in each mode. FX is no crop,
        // and is read as recorded. The DX shot, scaled onto its 5392 by
        // 3592 CropArea, lies as near the corner as FX, with about its
        // area; the 1:1 and 16:9 shots, moved back by 1376 across and 432
        // down, lie there too, with FX's area.
        for order in ORDERS {
            for (what, crop, recorded, focus) in [
                (
                    "FX",
                    Z9_FX,
                    Z9_FX_CORNER,
                    expected(Z9_FX_CORNER, Some(true)),
                ),
                (
                    "DX",
                    Z9_DX,
                    Z9_DX_CORNER,
                    focus_at([5392, 3592], [415, 373], [274, 305], Some(true)),
                ),
                (
                    "1:1",
                    Z9_1X1,
                    Z9_1X1_CORNER,
                    focus_at([5504, 5504], [421, 424], [291, 323], Some(true)),
                ),
                (
                    "16:9",
                    Z9_16X9,
                    Z9_16X9_CORNER,
                    focus_at([8256, 4640], [502, 283], [291, 323], Some(true)),
                ),
            ] {
                assert_eq!(
                    crop_focus(order, crop, "0400", recorded),
                    Some(focus),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_file_that_is_no_crop_gives_the_point_as_recorded() {
        // The D810 records a cropped width four pixels short of its
        // original on an uncropped file, and writes no CropArea (public
        // sample 2036); it records no position, so the record beside it is
        // made up, a centred point in its 7360 by 4912 frame.
        let d810 = Crop {
            raw: [7380, 4928],
            crop_hi_speed: Some([11, 7384, 4928, 7380, 4928, 0, 0]),
            crop_area: None,
        };
        let d810_record = Recorded {
            frame: [7360, 4912],
            center: [3680, 2456],
            area: [400, 400],
            in_focus: 1,
        };
        for order in ORDERS {
            for (what, crop, version, recorded, in_focus) in [
                // CropHiSpeed's two sizes equal: the Z 6's small raw, whose
                // CropArea is smaller than the frame on both sides (public
                // sample 3587).
                ("Z 6 small raw", Z6_SMALL_RAW, "0300", Z6, None),
                // The Z 9's DX values without a CropHiSpeed.
                (
                    "no CropHiSpeed",
                    Crop {
                        crop_hi_speed: None,
                        ..Z9_DX
                    },
                    "0400",
                    Z9_DX_CORNER,
                    Some(true),
                ),
                ("no CropArea", d810, "0100", d810_record, Some(true)),
            ] {
                assert_eq!(
                    crop_focus(order, crop, version, recorded),
                    Some(expected(recorded, in_focus)),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_crop_whose_crop_area_cannot_be_used_gives_none() {
        // The Z6III's 1:1 crop with a CropArea the image-size check
        // refuses, in a raw 4039 wide, one column too narrow for its 4032
        // from column 8; with one of three values; and with one of type
        // UNDEFINED, the eight bytes of its four SHORTs. Then the Z 9's DX
        // crop with its CropArea in a raw one column too narrow. A crop is
        // not read as no crop when its CropArea is there but cannot be
        // used: each gives no point, not the point as recorded in the full
        // frame.
        let narrow = Crop {
            raw: [4039, 4040],
            ..Z6III_1X1
        };
        let narrow_z9 = Crop {
            raw: [5399, 3608],
            ..Z9_DX
        };
        let crop_hi_speed = Z6III_1X1.crop_hi_speed.unwrap();
        let [left, top, width, height] = Z6III_1X1.crop_area.unwrap();
        for order in ORDERS {
            let value = af_info2(order, "0401", Z6III_ASPECT);
            let bytes: Vec<u8> = [left, top, width, height]
                .iter()
                .flat_map(|&v| order.u16(v))
                .collect();
            for (what, raw, block) in [
                (
                    "outside the raw",
                    narrow.raw,
                    crop_block(order, narrow, &value),
                ),
                (
                    "three values",
                    Z6III_1X1.raw,
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &crop_hi_speed)
                            .short(ifd, CROP_AREA, &[left, top, width])
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                ),
                (
                    "UNDEFINED",
                    Z6III_1X1.raw,
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &crop_hi_speed)
                            .undefined(ifd, CROP_AREA, &bytes)
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                ),
                (
                    "the Z 9's DX outside the raw",
                    narrow_z9.raw,
                    crop_block(order, narrow_z9, &af_info2(order, "0400", Z9_DX_CORNER)),
                ),
            ] {
                assert_eq!(
                    focus_in(order, raw, &block, whole(&block)),
                    None,
                    "{order:?}, {what}"
                );
            }
            // The control: the same file with no CropArea tag is read as
            // recorded, as the D810's is.
            let none = Crop {
                crop_area: None,
                ..Z6III_1X1
            };
            assert_eq!(
                crop_focus(order, none, "0401", Z6III_ASPECT),
                Some(expected(Z6III_ASPECT, Some(false))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_without_a_raw_size_gives_none_when_it_has_a_crop_area() {
        // The Z6III's 1:1 crop in a file with no raw IFD, so no raw size
        // to check its CropArea against: a CropArea of SHORTs, and one of
        // type UNDEFINED, each give no point. The control, the same file
        // with no CropArea tag, is read as recorded.
        let crop_hi_speed = Z6III_1X1.crop_hi_speed.unwrap();
        let crop_area = Z6III_1X1.crop_area.unwrap();
        for order in ORDERS {
            let value = af_info2(order, "0401", Z6III_ASPECT);
            let bytes: Vec<u8> = crop_area.iter().flat_map(|&v| order.u16(v)).collect();
            let none = Crop {
                crop_area: None,
                ..Z6III_1X1
            };
            for (what, block, focus) in [
                ("SHORTs", crop_block(order, Z6III_1X1, &value), None),
                (
                    "UNDEFINED",
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &crop_hi_speed)
                            .undefined(ifd, CROP_AREA, &bytes)
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    None,
                ),
                (
                    "no CropArea",
                    crop_block(order, none, &value),
                    Some(expected(Z6III_ASPECT, Some(false))),
                ),
            ] {
                let metadata = metadata_of(with_makernote(order, UNDEFINED, &block, whole(&block)));
                assert_eq!(metadata.raw_size, None, "{order:?}, {what}");
                assert_eq!(metadata.focus, focus, "{order:?}, {what}");
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_that_fits_neither_crop_shape_gives_none() {
        // A crop cuts both sides, or keeps exactly one and cuts the other.
        // The Z 6's small raw with a cropped size one pixel larger than
        // its original on each side; the Z 9's DX crop with a cropped
        // height of 5600, larger than its original 5520; and the Z 9's FX
        // mode with a cropped width one pixel larger and the height kept,
        // where CropArea's height equals the frame's. Moved by the window,
        // each point would still lie in the picture; each gives none.
        let z6 = Crop {
            crop_hi_speed: Some([11, 6064, 4040, 6065, 4041, 0, 0]),
            ..Z6_SMALL_RAW
        };
        let z9_dx = Crop {
            crop_hi_speed: Some([2, 8280, 5520, 5408, 5600, 1436, 956]),
            ..Z9_DX
        };
        let z9_dx_record = Recorded {
            center: [6000, 3000],
            ..Z9_DX_CORNER
        };
        let z9_fx = Crop {
            crop_hi_speed: Some([11, 8280, 5520, 8281, 5520, 0, 0]),
            ..Z9_FX
        };
        for order in ORDERS {
            for (what, crop, version, recorded) in [
                ("both sides larger", z6, "0300", Z6),
                ("one side larger, one smaller", z9_dx, "0400", z9_dx_record),
                ("one side larger, one kept", z9_fx, "0400", Z9_FX_CORNER),
            ] {
                assert_eq!(
                    crop_focus(order, crop, version, recorded),
                    None,
                    "{order:?}, {what}"
                );
            }
            // The control: both sides equal, the Z 6's small raw as it is,
            // is no crop, and is read as recorded.
            assert_eq!(
                crop_focus(order, Z6_SMALL_RAW, "0300", Z6),
                Some(expected(Z6, None)),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_hi_speed_of_other_than_seven_shorts_is_no_crop() {
        // The Z 9's DX values short of their last, with one more, and as
        // LONGs, which the reader of numbers reads too; then the seven
        // SHORTs, the control, which scale the point.
        let values = Z9_DX.crop_hi_speed.unwrap();
        let mut eight = values.to_vec();
        eight.push(0);
        for order in ORDERS {
            let value = af_info2(order, "0400", Z9_DX_CORNER);
            for (what, block, focus) in [
                (
                    "six SHORTs",
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &values[..6])
                            .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    expected(Z9_DX_CORNER, Some(true)),
                ),
                (
                    "eight SHORTs",
                    tags_block(order, |inner, ifd| {
                        inner
                            .short(ifd, CROP_HI_SPEED, &eight)
                            .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    expected(Z9_DX_CORNER, Some(true)),
                ),
                (
                    "seven LONGs",
                    tags_block(order, |inner, ifd| {
                        inner
                            .long(ifd, CROP_HI_SPEED, &values.map(u32::from))
                            .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                            .undefined(ifd, AF_INFO2, &value);
                    })
                    .0,
                    expected(Z9_DX_CORNER, Some(true)),
                ),
                (
                    "seven SHORTs",
                    crop_block(order, Z9_DX, &value),
                    focus_at([5392, 3592], [415, 373], [274, 305], Some(true)),
                ),
            ] {
                assert_eq!(
                    focus_in(order, Z9_DX.raw, &block, whole(&block)),
                    Some(focus),
                    "{order:?}, {what}"
                );
            }
        }
    }

    #[test]
    fn a_crop_hi_speed_past_the_block_is_no_crop() {
        for order in ORDERS {
            let values = Z9_DX.crop_hi_speed.unwrap();
            // Written in the order the tags are added, so that CropHiSpeed's
            // values, stored after the others, are the block's last bytes.
            let block = tags_block(order, |inner, ifd| {
                inner
                    .unsorted(ifd)
                    .short(ifd, CROP_AREA, &Z9_DX.crop_area.unwrap())
                    .undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9_DX_CORNER))
                    .short(ifd, CROP_HI_SPEED, &values);
            })
            .0;
            let bytes: Vec<u8> = values.iter().flat_map(|&v| order.u16(v)).collect();
            assert_eq!(block[block.len() - 14..], bytes, "{order:?}");

            // Declared one byte short: the last value's last byte lies in
            // the file, after the block, and the point is as recorded.
            assert_eq!(
                focus_in(order, Z9_DX.raw, &block, whole(&block) - 1),
                Some(expected(Z9_DX_CORNER, Some(true))),
                "{order:?}"
            );

            // The control: declared whole, the same bytes scale the point.
            assert_eq!(
                focus_in(order, Z9_DX.raw, &block, whole(&block)),
                Some(focus_at([5392, 3592], [415, 373], [274, 305], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_aspect_crop_on_a_small_raw_gives_none() {
        // The Z 9's 1:1 CropHiSpeed with a small raw's CropArea, which
        // counts stored pixels: 4128 high, not the frame's 5504, so the
        // window cannot be placed. The corner shot would move past the
        // window's edge anyway; a centred point would not, and gives none
        // all the same.
        let crop = Crop {
            raw: [4140, 4140],
            crop_area: Some([8, 8, 4128, 4128]),
            ..Z9_1X1
        };
        let centred = Recorded {
            center: [4128, 2752],
            ..Z9_1X1_CORNER
        };
        for order in ORDERS {
            for recorded in [Z9_1X1_CORNER, centred] {
                assert_eq!(
                    crop_focus(order, crop, "0400", recorded),
                    None,
                    "{order:?}, {recorded:?}"
                );
            }
        }
    }

    #[test]
    fn a_dx_crop_on_a_small_raw_is_scaled_onto_its_crop_area() {
        // The Z 9's DX CropHiSpeed with a small raw's CropArea, 4044 by
        // 2694 stored pixels inside a 4056 by 2706 raw.
        let crop = Crop {
            raw: [4056, 2706],
            crop_area: Some([6, 6, 4044, 2694]),
            ..Z9_DX
        };
        let [width, height] = Z9_DX_CORNER.frame;
        let [x, y] = Z9_DX_CORNER.center;
        let [area_width, area_height] = Z9_DX_CORNER.area;
        let by_hand = focus_at(
            [4044, 2694],
            [
                scale_by_hand(x, width, 4044),
                scale_by_hand(y, height, 2694),
            ],
            [
                scale_by_hand(area_width, width, 4044),
                scale_by_hand(area_height, height, 2694),
            ],
            Some(true),
        );
        // What the formula gives: (636 × 4044 + 4128) / 8256 = 312,
        // (572 × 2694 + 2752) / 5504 = 280, (420 × 4044 + 4128) / 8256 =
        // 206 and (468 × 2694 + 2752) / 5504 = 229.
        assert_eq!(
            by_hand,
            focus_at([4044, 2694], [312, 280], [206, 229], Some(true))
        );
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", Z9_DX_CORNER),
                Some(by_hand),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_window_that_cannot_be_placed_gives_none() {
        for order in ORDERS {
            // A CropArea 9000 wide, larger than the 8256 frame, on a crop
            // that keeps the height, inside a raw large enough for it.
            let wide = Crop {
                raw: [9000, 5520],
                crop_area: Some([0, 0, 9000, 5504]),
                ..Z9_1X1
            };
            assert_eq!(
                crop_focus(order, wide, "0400", Z9_1X1_CORNER),
                None,
                "{order:?}, wider than the frame"
            );
            // A centre 100 pixels in, which the 1:1 window's origin, 1376
            // pixels in, would take below zero.
            let left = Recorded {
                center: [100, 424],
                ..Z9_1X1_CORNER
            };
            assert_eq!(
                crop_focus(order, Z9_1X1, "0400", left),
                None,
                "{order:?}, left of the window"
            );
        }
    }

    #[test]
    fn a_crop_from_a_frame_with_a_zero_side_gives_none() {
        // A frame with a zero side is refused as recorded, by `Focus::new`,
        // before anything is scaled from it.
        for order in ORDERS {
            for frame in [[0, 5504], [8256, 0]] {
                let recorded = Recorded {
                    frame,
                    ..Z9_DX_CORNER
                };
                assert_eq!(
                    crop_focus(order, Z9_DX, "0400", recorded),
                    None,
                    "{order:?}, {frame:?}"
                );
            }
        }
    }

    #[test]
    fn a_dx_crop_rounds_to_the_nearest_pixel_a_half_rounding_up() {
        // A 1000 by 1000 frame scaled onto a 500 by 400 CropArea: by a half
        // across and by 0.4 down. Across, 301 × 0.5 = 150.5 rounds up to
        // 151 and 101 × 0.5 = 50.5 to 51; down, 501 × 0.4 = 200.4 rounds
        // down to 200 and 101 × 0.4 = 40.4 to 40.
        let crop = Crop {
            raw: [500, 400],
            crop_hi_speed: Some([2, 1000, 1000, 500, 400, 0, 0]),
            crop_area: Some([0, 0, 500, 400]),
        };
        let recorded = Recorded {
            frame: [1000, 1000],
            center: [301, 501],
            area: [101, 101],
            in_focus: 1,
        };
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", recorded),
                Some(focus_at([500, 400], [151, 200], [51, 40], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_keeps_an_area_side_of_one_pixel() {
        // A single pixel in the Z 9's frame, scaled onto a 1000 by 1000
        // CropArea: 1000 / 8256 and 1000 / 5504 of a pixel round to
        // nothing, and each side keeps one.
        let crop = Crop {
            raw: [1000, 1000],
            crop_hi_speed: Some([2, 8280, 5520, 1000, 1000, 0, 0]),
            crop_area: Some([0, 0, 1000, 1000]),
        };
        let recorded = Recorded {
            center: [4128, 2752],
            area: [1, 1],
            ..Z9_FX_CORNER
        };
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", recorded),
                Some(focus_at([1000, 1000], [500, 500], [1, 1], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_trims_an_area_that_rounding_leaves_a_pixel_too_wide() {
        // The Z 9's DX crop, with a centre 146 across and an area 292 wide,
        // which fits the frame as recorded: half of 292 is 146. Scaled onto
        // the 5392 by 3592 CropArea, x is (146 × 5392 + 4128) / 8256 = 95
        // and the width (292 × 5392 + 4128) / 8256 = 191, whose half
        // rounds up to 96, one more than 95: the width is trimmed to 190,
        // twice the centre's distance from the left edge. Down, the corner
        // shot's (572 × 3592 + 2752) / 5504 = 373 and (468 × 3592 + 2752)
        // / 5504 = 305 fit as they are.
        let near_the_edge = Recorded {
            center: [146, 572],
            area: [292, 468],
            ..Z9_DX_CORNER
        };
        let [width, _] = Z9_DX_CORNER.frame;
        assert_eq!(
            (
                scale_by_hand(146, width, 5392),
                scale_by_hand(292, width, 5392)
            ),
            (95, 191)
        );
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, Z9_DX, "0400", near_the_edge),
                Some(focus_at([5392, 3592], [95, 373], [190, 305], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_whose_centre_lies_outside_the_recorded_frame_gives_none() {
        // A centre at x = 8257, one pixel past the right edge of the 8256
        // frame, which `Focus::new` refuses as recorded. On the Z 9's DX
        // CropArea it would scale past the right edge anyway, to 5393; on
        // a small raw's CropArea, 4044 wide, it would scale to (8257 × 4044
        // + 4128) / 8256 = 4044, on the right edge, and only its area would
        // be lost. The control, the corner shot, gives the point on both.
        let small_raw = Crop {
            raw: [4056, 2706],
            crop_area: Some([6, 6, 4044, 2694]),
            ..Z9_DX
        };
        let outside = Recorded {
            center: [8257, 572],
            ..Z9_DX_CORNER
        };
        let [width, _] = Z9_DX_CORNER.frame;
        assert_eq!(scale_by_hand(8257, width, 4044), 4044);
        for order in ORDERS {
            for crop in [Z9_DX, small_raw] {
                assert_eq!(
                    crop_focus(order, crop, "0400", outside),
                    None,
                    "{order:?}, {crop:?}"
                );
                assert!(
                    crop_focus(order, crop, "0400", Z9_DX_CORNER).is_some(),
                    "{order:?}, {crop:?}"
                );
            }
        }
    }

    #[test]
    fn a_dx_crop_whose_area_does_not_fit_as_recorded_gives_none() {
        // A centre 145 across with an area 291 wide: half of 291 rounds up
        // to 146, one more than 145, so the area does not fit the frame as
        // recorded. Scaled onto the Z 9's DX CropArea first, it would:
        // (145 × 5392 + 4128) / 8256 = 95 and (291 × 5392 + 4128) / 8256 =
        // 190, whose half is 95.
        let too_wide = Recorded {
            center: [145, 572],
            area: [291, 468],
            ..Z9_DX_CORNER
        };
        let [width, _] = Z9_DX_CORNER.frame;
        assert_eq!(
            (
                scale_by_hand(145, width, 5392),
                scale_by_hand(291, width, 5392)
            ),
            (95, 190)
        );
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, Z9_DX, "0400", too_wide),
                None,
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_dx_crop_that_rounds_the_centre_onto_an_edge_keeps_the_point_alone() {
        // A 1000 by 1000 frame scaled onto a 400 by 400 CropArea, by 0.4:
        // a centre 1 across, with an area 2 wide, which fits as recorded,
        // rounds to (1 × 400 + 500) / 1000 = 0, on the left edge, where no
        // area fits, and the point is kept without one. Down, (500 × 400 +
        // 500) / 1000 = 200.
        let crop = Crop {
            raw: [400, 400],
            crop_hi_speed: Some([2, 1000, 1000, 400, 400, 0, 0]),
            crop_area: Some([0, 0, 400, 400]),
        };
        let recorded = Recorded {
            frame: [1000, 1000],
            center: [1, 500],
            area: [2, 2],
            in_focus: 1,
        };
        let frame = Size {
            width: 400,
            height: 400,
        };
        let point = Focus::new(frame, Point { x: 0, y: 200 })
            .expect("a centre on the frame")
            .with_in_focus(true);
        for order in ORDERS {
            assert_eq!(
                crop_focus(order, crop, "0400", recorded),
                Some(point),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_whose_area_no_longer_fits_gives_none() {
        // The 16:9 shot's centre, 715 down, with an area 900 high, which
        // fits the full frame: moved back to 283, the 450 above it would
        // leave the window. The control, the shot's own 323, fits.
        for order in ORDERS {
            let tall = Recorded {
                area: [291, 900],
                ..Z9_16X9_CORNER
            };
            assert_eq!(crop_focus(order, Z9_16X9, "0400", tall), None, "{order:?}");
            assert_eq!(
                crop_focus(order, Z9_16X9, "0400", Z9_16X9_CORNER),
                Some(focus_at([8256, 4640], [502, 283], [291, 323], Some(true))),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_default_crop_leaves_the_crop_area_to_the_focus() {
        // The image size is the default crop, which comes first; CropArea
        // still gives the cropped picture the point is placed on.
        for order in ORDERS {
            let block = crop_block(order, Z6III_1X1, &af_info2(order, "0401", Z6III_ASPECT));
            let [width, height] = Z6III_1X1.raw;
            let raw = Size { width, height };
            let file = raw_file_of(order, raw, &block, whole(&block), |builder, ifd0| {
                builder.long(ifd0, DEFAULT_CROP_ORIGIN, &[0, 0]).long(
                    ifd0,
                    DEFAULT_CROP_SIZE,
                    &[4000, 4000],
                );
            });
            let metadata = metadata_of(file);
            assert_eq!(
                metadata.image_size,
                Some(Size {
                    width: 4000,
                    height: 4000
                }),
                "{order:?}"
            );
            assert_eq!(
                metadata.focus,
                Some(focus_at(
                    [4032, 4032],
                    [2015, 2016],
                    [163, 174],
                    Some(false)
                )),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_crop_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = crop_block(
                inner_order,
                Z9_DX,
                &af_info2(inner_order, "0400", Z9_DX_CORNER),
            );
            assert_eq!(
                focus_in(file, Z9_DX.raw, &block, whole(&block)),
                Some(focus_at([5392, 3592], [415, 373], [274, 305], Some(true))),
                "{file:?}"
            );
        }
    }

    /// The first byte of the second block the source caches: a reader that
    /// fails from here on still serves the walk, which reads only the
    /// first.
    const FAR: u64 = 64 * 1024;

    /// A reader that fails every read from [`FAR`] on.
    struct FailsFar(Cursor<Vec<u8>>);

    impl Read for FailsFar {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0.position() >= FAR {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            self.0.read(buf)
        }
    }

    impl Seek for FailsFar {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.0.seek(pos)
        }
    }

    #[test]
    fn a_reader_failure_is_an_error() {
        // The block far away, so that reading its signature fails; and
        // with its signature before FAR and its inner header from FAR on,
        // so that reading the header fails.
        for at in [2 * FAR, FAR - HEADER_AT] {
            let block = block(Order::Little, Change::Nothing);
            let bytes = nikon_file(Order::Little, at, &block.bytes, 0, |builder, exif| {
                entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, block.len());
            });
            // The file is sound, for a reader that does not fail.
            assert_eq!(candidate(bytes.clone()), Some(block.candidate(at)), "{at}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err =
                read_candidate(&mut src, &tree, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{at}: {err:?}");
        }
    }

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
            metadata_of(bytes.clone()).focus,
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
        let err = metadata(&mut src, &makernote, standard).expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }
}
