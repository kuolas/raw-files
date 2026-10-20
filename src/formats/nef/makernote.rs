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
//! Nikon has used three layouts for the block, told apart by their first
//! bytes. The one every body since the D70 and the D100 writes, and the one
//! read here, starts with the signature `Nikon` and a NUL, two version
//! bytes and two more; then, 10 bytes in, comes a TIFF header of its own: a
//! byte-order mark, 42 and the offset of the MakerNote IFD. That inner
//! header is the base of every offset inside the block, the MakerNote
//! IFD's, the PreviewIFD's and the JPEG's, never the start of the file, and
//! its byte order, which may differ from the file's, is the order of every
//! number inside the block. The two older layouts, a bare IFD with no
//! signature and an IFD after the 8 bytes `Nikon\0\1\0`, count their
//! offsets from the file's own header instead. They are not read in this
//! release: a file that has one gives no MakerNote preview.
//!
//! Nothing in the block is trusted. Every offset found inside it is checked
//! against the block, not only against the file: the MakerNote IFD and the
//! PreviewIFD must lie inside the block, with their whole entry tables; the
//! three numbers that lead to the JPEG must be held in their entries, not
//! stored elsewhere; and the whole JPEG, as declared, must lie inside the
//! block. When the block's IFDs are read, every entry whose value lies
//! outside the block is dropped, as an entry whose value lies outside the
//! file is, so every value read from them lies inside the block: the ISO's
//! two numbers and WorldTime's four bytes, which the entry holds, and
//! CropArea's four numbers, the serial number and the lens's range, which
//! take more than the four bytes an entry holds and so are stored
//! elsewhere. A value of another type than its tag's rule allows is not
//! read. A block that does not hold together, whether its inner header
//! is not one, an IFD in it is cut short or has more entries than the
//! limits allow, costs what the MakerNote would have given, the small
//! preview, the crop area and the values above, never the file: the NEF
//! security notes ask that bad entries be skipped rather than fail the
//! file (`docs/formats/nef/security.md`, item 4 of the checklist). Only a
//! failure of the reader itself is an error: that is not the file's fault,
//! and says nothing about the MakerNote.
//!
//! The MakerNote is read once per file, into a [`MakerNote`]: its block
//! found and checked, its MakerNote IFD parsed. Every use of the MakerNote
//! then reads from that same structure, so that none of them finds, checks
//! or parses the block again.

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::tags::{
    JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKER_NOTE,
};
use crate::container::tiff::types::{ASCII, BYTE, LONG, RATIONAL, SHORT, UNDEFINED};
use crate::container::tiff::{Entry, Ifd, Tree, parse_header_at, parse_ifd, skip_damage};
use crate::error::Result;
use crate::formats::tiff_metadata::{self, Standard};
use crate::formats::tiff_previews::Candidate;
use crate::geometry::Size;
use crate::io::{ByteOrder, Source, within};
use crate::limits::Limits;
use crate::metadata::{Lens, Metadata};
use crate::rational::Rational;

/// The Nikon MakerNote tag that records the ISO the picture was taken at
/// (ISO): two SHORTs, the second of them the ISO, on the bodies whose Exif
/// tags may lack one.
const ISO: u16 = 0x0002;

/// The Nikon MakerNote tag that points at the PreviewIFD, the sub-IFD
/// holding the small preview JPEG.
const PREVIEW_IFD: u16 = 0x0011;

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
pub(super) struct MakerNote {
    /// Where the block's bytes are in the file: the bounds that every
    /// offset found inside the block is checked against.
    block: Range<u64>,
    /// What every offset inside the block counts from: the inner TIFF
    /// header, 10 bytes into the block.
    base: u64,
    /// The byte order of every number inside the block, the inner
    /// header's, which may differ from the file's.
    order: ByteOrder,
    /// The MakerNote IFD, which holds Nikon's tags: the IFD the inner
    /// header points at, inside the block with its whole entry table. Its
    /// entries whose values lie outside the block were dropped when it was
    /// read, so every value read from it lies inside the block.
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
pub(super) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    let Some(block) = tree
        .exif()
        .and_then(|exif| exif.entry(MAKER_NOTE))
        .and_then(block_of)
    else {
        return Ok(None);
    };
    // The block's bytes lie inside the file, as every kept entry's value
    // does, so the reads below that stay inside the block can fail only
    // with the reader.
    if block
        .end
        .checked_sub(block.start)
        .is_none_or(|len| len < MIN_LEN)
        || src.bytes(block.start, 6)?.as_slice() != SIGNATURE
    {
        return Ok(None);
    }

    let Some(base) = block.start.checked_add(HEADER_AT) else {
        return Ok(None);
    };
    let Some(header) = skip_damage(parse_header_at(src, base))? else {
        return Ok(None);
    };
    let Some(ifd) = ifd_in(src, &block, base, header.ifd0, header.order, limits)? else {
        return Ok(None);
    };
    Ok(Some(MakerNote {
        block,
        base,
        order: header.order,
        ifd,
    }))
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
pub(super) fn preview_candidate<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    limits: &Limits,
) -> Result<Option<Candidate>> {
    let MakerNote {
        block,
        base,
        order,
        ifd,
    } = makernote;
    let Some(pointer) = inline_u32(src, ifd, PREVIEW_IFD)? else {
        return Ok(None);
    };
    let Some(preview) = ifd_in(src, block, *base, pointer, *order, limits)? else {
        return Ok(None);
    };
    let offset = inline_u32(src, &preview, JPEG_INTERCHANGE_FORMAT)?;
    let length = inline_u32(src, &preview, JPEG_INTERCHANGE_FORMAT_LENGTH)?;
    let (Some(offset), Some(length)) = (offset, length) else {
        return Ok(None);
    };

    // An empty JPEG is passed on: the finder drops it with the others.
    let length = u64::from(length);
    match base.checked_add(u64::from(offset)) {
        Some(offset) if fits(block, offset, length) => Ok(Some(Candidate { offset, length })),
        _ => Ok(None),
    }
}

/// The metadata of the file whose MakerNote is `makernote`: what the
/// standard tags gave, `standard`, with what the MakerNote records that the
/// standard tags lack.
///
/// That is, first, the image size, from CropArea, which replaces the raw
/// size the image size otherwise falls back on: when the raw IFD records no
/// default crop, which comes first, and when there is a raw size for the
/// crop area to lie inside ([`crop_area`]). Then, each only when the
/// standard tags gave none, the serial number ([`serial_number`]), the ISO
/// ([`iso`]), the lens's range ([`lens_range`]) and the capture time's
/// offset from UTC ([`world_time`]): a range fills a lens whose standard
/// tags gave no value of its range, keeping their make, model and serial
/// number, or makes a lens of the range alone when they described none;
/// an offset is given only to a capture time the standard tags gave, one
/// whose OffsetTimeOriginal is missing or could not be read, and only
/// within the bound every offset is held to
/// ([`DateTime::with_offset`](crate::datetime::DateTime::with_offset)).
/// Every other value is the standard tags'.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn metadata<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    standard: Standard,
) -> Result<Metadata> {
    let Standard {
        mut metadata,
        from_default_crop,
    } = standard;
    if !from_default_crop
        && let Some(raw) = metadata.raw_size
        && let Some(image) = crop_area(src, makernote, raw)?
    {
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
        && let Some(time) = time.with_offset(offset)
    {
        metadata.capture_time = Some(time);
    }
    Ok(metadata)
}

/// The size of the visible image that `makernote`'s CropArea records, in
/// raw data of size `raw`; `None` when the tag is missing, is not of type
/// SHORT or LONG, does not hold exactly four values, stores them outside
/// the block, or describes a rectangle that [`tiff_metadata::crop`] does
/// not accept: one of zero width or height, or not inside the raw data.
///
/// See the [module documentation](self) for what CropArea is.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn crop_area<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    raw: Size,
) -> Result<Option<Size>> {
    if !has_entry_of_type(makernote, CROP_AREA, &[SHORT, LONG]) {
        return Ok(None);
    }
    let values = skip_damage(makernote.ifd.u32s(src, CROP_AREA))?.flatten();
    let Some(&[left, top, width, height]) = values.as_deref() else {
        return Ok(None);
    };
    Ok(tiff_metadata::crop([left, top], [width, height], raw))
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
    if !has_entry_of_type(makernote, SERIAL_NUMBER, &[ASCII]) {
        return Ok(None);
    }
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
    if !has_entry_of_type(makernote, ISO, &[SHORT]) {
        return Ok(None);
    }
    let values = skip_damage(makernote.ifd.u32s(src, ISO))?.flatten();
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
    if !has_entry_of_type(makernote, LENS, &[RATIONAL]) {
        return Ok([None; 4]);
    }
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
/// bounded here: [`DateTime::with_offset`](crate::datetime::DateTime::with_offset),
/// which takes it, refuses one beyond 14 hours, as a zone rewritten in the
/// wrong byte order is.
///
/// See the [module documentation](self) for what WorldTime is.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn world_time<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<Option<i16>> {
    if !has_entry_of_type(makernote, WORLD_TIME, &[UNDEFINED]) {
        return Ok(None);
    }
    let bytes = skip_damage(makernote.ifd.bytes(src, WORLD_TIME))?.flatten();
    let Some(&[zone0, zone1, daylight_saving, _date_format]) = bytes.as_deref() else {
        return Ok(None);
    };
    let zone = i32::from(makernote.order.u16([zone0, zone1]).cast_signed());
    let offset = match daylight_saving {
        0 => zone,
        1 => zone + 60,
        _ => return Ok(None),
    };
    // A zone near the end of the 16-bit range, plus the daylight hour, may
    // not fit; it is far beyond any offset in use anyway.
    Ok(i16::try_from(offset).ok())
}

/// Whether `makernote`'s MakerNote IFD has an entry of tag `tag`, of one
/// of the types `types`: the type its tag's rule allows.
///
/// The readers of values accept more types than a tag's rule allows:
/// [`Ifd::u32s`] reads type IFD too. The value itself lies inside the
/// block, as every value of the IFD does ([`ifd_in`]).
fn has_entry_of_type(makernote: &MakerNote, tag: u16, types: &[u16]) -> bool {
    makernote
        .ifd
        .entry(tag)
        .is_some_and(|entry| types.contains(&entry.typ()))
}

/// Where the MakerNote's bytes are in the file, from its entry: `None`
/// unless the entry holds bytes, of type UNDEFINED or BYTE, stored out of
/// line, as a block of 18 bytes or more always is.
///
/// The value of a kept entry lies inside the file, so the range does too.
fn block_of(entry: &Entry) -> Option<Range<u64>> {
    if !matches!(entry.typ(), UNDEFINED | BYTE) {
        return None;
    }
    let start = entry.value_offset()?;
    let end = start.checked_add(u64::from(entry.count()))?;
    Some(start..end)
}

/// The IFD at `offset` from `base`, read in byte order `order` with its
/// own offsets counted from `base`, without the entries whose value lies
/// outside `block`; `None` when it does not lie inside `block` with its
/// count and all its entries, or cannot be read.
///
/// An entry whose value is stored elsewhere is dropped unless the value
/// [`fits`] the block: [`parse_ifd`] checks it only against the file.
/// A value the entry holds is inside the block with the entry. So every
/// value read from the IFD lies inside the block.
///
/// The IFD's next-IFD pointer is not used, and not required: the D200
/// writes none.
///
/// # Errors
///
/// `Io` when the reader fails.
fn ifd_in<R: Read + Seek>(
    src: &mut Source<R>,
    block: &Range<u64>,
    base: u64,
    offset: u32,
    order: ByteOrder,
    limits: &Limits,
) -> Result<Option<Ifd>> {
    let Some(at) = base.checked_add(u64::from(offset)) else {
        return Ok(None);
    };
    // The count first, inside the block and so inside the file, then the
    // entries it announces. The 16-bit count bounds the table to
    // 2 + 12 * 65,535 bytes: this cannot overflow.
    if !fits(block, at, 2) {
        return Ok(None);
    }
    let table = 2 + 12 * u64::from(src.u16_at(at, order)?);
    if !fits(block, at, table) {
        return Ok(None);
    }
    let Some((mut ifd, _)) = skip_damage(parse_ifd(src, base, at, order, limits))? else {
        return Ok(None);
    };
    ifd.retain(|entry| {
        entry
            .value_offset()
            .is_none_or(|at| fits(block, at, u64::from(entry.value_len())))
    });
    Ok(Some(ifd))
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

/// Whether the `len` bytes at `at` lie inside `block`.
fn fits(block: &Range<u64>, at: u64, len: u64) -> bool {
    at >= block.start && within(at, len, block.end).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::{
        BODY_SERIAL_NUMBER, DATE_TIME_ORIGINAL, DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE,
        IMAGE_LENGTH, IMAGE_WIDTH, LENS_MODEL, LENS_SPECIFICATION, MAKE, OFFSET_TIME_ORIGINAL,
        PHOTOGRAPHIC_SENSITIVITY, PHOTOMETRIC_INTERPRETATION, SUB_SEC_TIME_ORIGINAL,
    };
    use crate::container::tiff::types::{IFD, SRATIONAL};
    use crate::container::tiff::{parse_header, walk};
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
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
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

            // The PreviewIFD as the file holds it: the out-of-line entry
            // first, its value the 8 bytes just past the block.
            let limits = Limits::default();
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &limits).unwrap();
            let makernote = read(&mut src, &tree, &limits)
                .unwrap()
                .expect("a MakerNote");
            let at = AT + block.preview;
            let (table, _) = parse_ifd(&mut src, makernote.base, at, makernote.order, &limits)
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
        nikon_file_with(order, AT, block, TAIL, |builder, ifd0, exif| {
            builder
                .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[CFA])
                .long(ifd0, IMAGE_WIDTH, &[RAW.width])
                .long(ifd0, IMAGE_LENGTH, &[RAW.height]);
            raw(builder, ifd0);
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, AT, count);
        })
    }

    /// The metadata of the file `bytes`: what the standard tags give, with
    /// what [`metadata`] adds from the file's MakerNote, which must be
    /// read.
    fn metadata_of(bytes: Vec<u8>) -> Metadata {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();
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
                let header = parse_header(&mut src).unwrap();
                let tree = walk(&mut src, header, &Limits::default()).unwrap();
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
            let header = parse_header(&mut src).unwrap();
            let tree = walk(&mut src, header, &Limits::default()).unwrap();
            let err =
                read_candidate(&mut src, &tree, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{at}: {err:?}");
        }
    }
}
