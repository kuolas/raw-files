// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Sony MakerNote: where it lies in the file, how its directory is
//! found, what is checked before anything in it is read, and the focus
//! point read from it.
//!
//! The MakerNote is a block of Sony's own metadata, the value of the
//! MakerNote tag of the Exif IFD: a directory of Sony's tags, an IFD like
//! the file's own, with the values it points at. raw-files reads it for one
//! thing only, the point where the camera focused, which no standard tag
//! records and which Sony keeps in plain tags of this directory. ADR012
//! decided it, amending ADR011, under which no Sony MakerNote was read.
//!
//! Sony has written the block in several layouts, told apart by its first
//! bytes:
//!
//! - **A header, then the IFD 12 bytes in.** The DSLR-A100's ARW 1.0 starts
//!   its MakerNote with `SONY DSC `, a space and three NULs included
//!   (public sample 2346 of raw.pixls.us). ExifTool's documentation lists
//!   four more headers for Sony files, seen in no sample: `SONY CAM ` and
//!   three NULs; `SONY MOBILE`; two NULs, `SONY PIC` and a NUL; and `VHAB`,
//!   five spaces and a NUL. The last three are shorter than 12 bytes, and
//!   the bytes after them, up to the IFD, are not looked at.
//! - **No header: the IFD at the block's start.** Every other ARW, SR2 and
//!   ARQ sample writes this one, the Hasselblad Lunar's among them (sample
//!   4051). The SRF files of the earliest bodies keep a chain of IFDs
//!   there; the first is read as a plain IFD, with its single entry, and
//!   the chain is not followed.
//! - **No Sony directory at all.** A block that starts with `SONY PI` and a
//!   NUL, or `PREMI` and a NUL, is in Olympus's format, with Olympus's
//!   tags, and one that starts with `SONY PIC` and a NUL is text.
//!   ExifTool's documentation lists them for the JPEG files of Sony's
//!   compacts, and none of the RAW samples holds one. Such a block gives no
//!   MakerNote, even with an IFD after it, since what it holds is not
//!   Sony's directory.
//!
//! The headers are looked for first, then the blocks that hold no Sony
//! directory; any other block is taken to start with its IFD. The file's
//! Make is not looked at: the ARW reader's claim has already decided that
//! the file is Sony's.
//!
//! Every offset inside the block counts from the start of the file, where
//! its TIFF header is, as the offsets of the file's own IFDs do: not from
//! the block, nor from a header of its own, as Nikon's do. On the A100,
//! tag 0xB028 records the offset of a Minolta-format IFD, and the IFD lies
//! at that offset from the file's start (sample 2346). So the IFD is read
//! with base 0. The block's numbers are in the file's byte order, the one
//! the Exif IFD was read in. Every sample is little-endian; no big-endian
//! Sony file is known, so that its MakerNote would follow the file's order
//! is unchecked.
//!
//! The four bytes after the IFD's entries are not a next-IFD pointer to
//! rely on: they are zero on the older bodies, such as the A100, the NEX-5
//! and the DSC-R1, and the first bytes of the values on the RX100 and every
//! ILCE and RX body since 2012. They are neither used nor required, as the
//! ARW security notes ask (`docs/formats/arw/security.md`, item A5). Nor
//! need the tags be sorted: Sony writes other IFDs with tags out of order
//! (the raw SubIFD of the ILCE-9, samples 1541 and 1542), and the IFD keeps
//! the order of the file, each tag found wherever it is.
//!
//! Nothing in the block is trusted. Its IFD is read inside the block, as
//! the Nikon reader's are ([`ifd_in`]): the IFD must lie inside the
//! block with its count and its whole entry table, and every entry whose
//! value lies outside the block is dropped, as an entry whose value lies
//! outside the file is, so every value read from it lies inside the block.
//! The signatures are looked for in at most the block's first 12 bytes,
//! never past its end. A block too short for its header or for the IFD's
//! count, an IFD cut short by the block's end, one with no entries, and one
//! with more entries than the limits allow cost the MakerNote, never the
//! file. Only a failure of the reader itself is an error: that is not the
//! file's fault, and says nothing about the MakerNote.
//!
//! Nothing in the block is deciphered: Sony enciphers several of its tags'
//! values, and none of them is read. FileFormat (0xB000), SonyModelID
//! (0xB001) and the lens tags are not read either: the standard tags give
//! the camera and the lens (ADR011).
//!
//! The MakerNote is read once per file, into a [`MakerNote`]: its block
//! found and checked, its IFD parsed. Every use of the MakerNote is to read
//! from that same structure, as the Nikon reader's do, so that none of them
//! finds, checks or parses the block again.
//!
//! # The focus point
//!
//! The bodies from the ILCE-7RM2 (2015) on record where the camera focused
//! as a position in pixels, in plain tags of the MakerNote IFD, outside
//! the values Sony enciphers. The tags' meanings are those of ExifTool's
//! documentation, confirmed on the 36 public samples that the ARW notes
//! list (`docs/formats/arw/makernote-and-metadata.md`, section 9.6), and
//! every one of their values lies inside the block on every sample:
//!
//! - **FocusLocation** ([`FOCUS_LOCATION`]) says where the camera focused:
//!   the width and height of a frame, then the x and y of the focus area's
//!   centre in it. Every sample from the ILCE-7RM2 on has it; the ILCE-7S,
//!   the NEX-5, the RX100, the A-mount bodies, the Hasselblad Lunar and the
//!   older formats do not. The ILCE-9M3, the ILCE-1M2 and the ILCE-7M5
//!   repeat it in **FocusLocation2** ([`FOCUS_LOCATION2`]), equal to it on
//!   every sample. FocusLocation2 is read only when FocusLocation gives
//!   nothing; no sample has FocusLocation2 alone, so such a file is
//!   unchecked.
//! - **FocusFrameSize** ([`FOCUS_FRAME_SIZE`]) says how large the focus
//!   area is, on the bodies from the ILCE-7SM3 (2020) on: from 92 by 94
//!   pixels on the ILCE-7SM3 (public sample 5418) to 1386 by 998 on the
//!   ILCE-1M2 (file 7835). Its numbers are in the file's byte order, like
//!   every number in the block; every sample is little-endian, so that a
//!   big-endian file's are too is unchecked.
//! - **FocusMode** ([`FOCUS_MODE`]) says how the camera focused: manual
//!   focus, or one of the autofocus modes.
//!
//! The position is the centre of the area. Several autofocus shots with an
//! area sit at exactly half their frame, such as the ILCE-1's (4320, 2880)
//! in 8640 by 5760 pixels with an area of 1026 by 912 (file 4466) and the
//! ILCE-6700's (file 6736); and the ILCE-7RM3's agrees with the spot that
//! another tag, FlexibleSpotPosition, records on a grid of 640 by 480
//! (file 3251).
//!
//! The frame is the picture as the camera framed it, in the orientation of
//! the stored image and at the file's scale, which is not always the image
//! size raw-files reports. It is 6000 by 4000 on the ILCE-9 (files 1541
//! and 1542). On the RX100M6's 1:1 file it is 3648 by 3648, while the
//! image size is the 5472 by 3648 default crop (file 3012), and on its 16:9
//! file 5472 by 3080 (file 3015). On the ILCE-7M4's medium and small raws
//! it is the full-size 7008 by 4672, larger than the stored image (files
//! 6930 and 6931), and on the ILCE-9M3's small raw 3008 by 2000 (file
//! 7364). The point is given as recorded, built through [`Focus::new`] and
//! [`Focus::with_size`]: a frame with a zero side or a centre off the
//! frame gives none, and so does an area that does not fit inside the
//! frame around its centre, since such a record cannot be used, as on a
//! Nikon file (ADR012). A side of zero in FocusFrameSize says only that no
//! size is recorded, and the point is kept without one.
//!
//! A shot in manual focus still records a location, the frame's exact
//! centre, with FocusMode 0: the ILCE-7RM2's files 1310 and 1311, and the
//! ILCE-9M3's 7361 and 7364, whose FocusFrameSize is all zeros. The camera
//! did not focus on that point, so FocusMode 0 gives none, whatever
//! FocusLocation holds (ADR012). Nothing else in the plain tags says that
//! no autofocus took place, so any other FocusMode, or none, keeps the
//! point. The ILCE-7S's file is a manual-focus shot without FocusLocation
//! (file 1582). No plain tag says whether focus was confirmed either, so
//! [`Focus::in_focus`] is never set.

use std::io::{Read, Seek};

use crate::container::tiff::tags::MAKER_NOTE;
use crate::container::tiff::types::{BYTE, SHORT, UNDEFINED};
use crate::container::tiff::{Block, Entry, Ifd, Tree, ifd_in, skip_damage};
use crate::error::Result;
use crate::geometry::{Point, Size};
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::Focus;

/// The Sony MakerNote tag that records how the camera focused
/// (FocusMode): one BYTE, 0 for manual focus, and 2, 3, 4, 6 or 7 for the
/// autofocus modes AF-S, AF-C, AF-A, DMF and AF-D.
pub(super) const FOCUS_MODE: u16 = 0x201B;

/// The Sony MakerNote tag that records where the camera focused
/// (FocusLocation): four SHORTs, the width and height of the frame, then
/// the x and y of the focus area's centre in it.
pub(super) const FOCUS_LOCATION: u16 = 0x2027;

/// The Sony MakerNote tag that records the size of the focus area
/// (FocusFrameSize): six UNDEFINED bytes, the area's width and height as
/// 16-bit numbers in the MakerNote's byte order, then a third, 257 or 0,
/// whose meaning is unknown.
const FOCUS_FRAME_SIZE: u16 = 0x2037;

/// The Sony MakerNote tag that repeats FocusLocation on the newest bodies
/// (FocusLocation2): the same four SHORTs, with the same values.
const FOCUS_LOCATION2: u16 = 0x204A;

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
/// the MakerNote starts from, so that the block is found, checked and
/// parsed once per file.
///
/// See the [module documentation](self) for the layouts.
#[derive(Debug)]
pub(super) struct MakerNote {
    /// The MakerNote IFD, which holds Sony's tags: at the block's start, or
    /// 12 bytes in after a header, inside the block with its whole entry
    /// table, its offsets counted from the start of the file. Its entries
    /// whose values lie outside the block were dropped when it was read, so
    /// every value read from it lies inside the block.
    ///
    /// It was read in the block, [`Ifd::block`]: of base 0, and with bounds
    /// where the block's bytes are in the file, which every offset found
    /// inside the block is checked against.
    ifd: Ifd,
}

/// The Sony MakerNote of the file in `src`, whose IFDs are `tree`, read
/// within `limits`; `None` when the file has no Exif IFD, no MakerNote, a
/// MakerNote that is not bytes stored out of line, one that holds no Sony
/// directory, or one whose IFD does not hold together inside the block.
///
/// See the [module documentation](self) for the layouts and the checks.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn read<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<MakerNote>> {
    let Some(exif) = tree.exif() else {
        return Ok(None);
    };
    let Some(range) = exif.entry(MAKER_NOTE).and_then(Entry::byte_range) else {
        return Ok(None);
    };
    // The block's bytes lie inside the file, as every kept entry's value
    // does, so a read that stays inside the block can fail only with the
    // reader. `Entry::byte_range` gives a range whose end is not before its
    // start.
    let len = range.end.saturating_sub(range.start);
    let head = src.bytes(range.start, len.min(HEADER_LEN))?;
    let Some(skip) = ifd_start(&head) else {
        return Ok(None);
    };
    // The IFD's offset from base 0, as a TIFF offset is written: a block
    // that starts near the top of the 32-bit range may put it past.
    let Some(offset) = range
        .start
        .checked_add(skip)
        .and_then(|at| u32::try_from(at).ok())
    else {
        return Ok(None);
    };
    // The block's numbers are in the file's byte order, the Exif IFD's.
    let block = Block::new(0, range);
    let Some(ifd) = ifd_in(src, &block, offset, exif.order(), limits)? else {
        return Ok(None);
    };
    Ok(Some(MakerNote { ifd }))
}

/// How far into the block the MakerNote IFD starts, from `head`, the
/// block's first bytes, at most [`HEADER_LEN`] of them: after one of
/// [`HEADERS`], [`HEADER_LEN`]; at the start, without one; `None` when the
/// block starts as one of [`NOT_SONY`], and holds no Sony directory.
fn ifd_start(head: &[u8]) -> Option<u64> {
    if HEADERS.iter().any(|header| head.starts_with(header)) {
        Some(HEADER_LEN)
    } else if NOT_SONY.iter().any(|start| head.starts_with(start)) {
        None
    } else {
        Some(0)
    }
}

/// Where the camera focused, as `makernote` records it: the centre of the
/// focus area and the frame it is measured in, from FocusLocation, and the
/// area's size from FocusFrameSize when it records one. Whether focus was
/// confirmed is never set: no plain tag records it.
///
/// `None` when FocusMode records manual focus ([`manual_focus`]); when
/// neither FocusLocation nor FocusLocation2 gives a location
/// ([`location`]); when [`Focus::new`] refuses the location, for a frame
/// with a zero side or a centre off the frame; or when FocusFrameSize
/// records an area, both of its sides above zero, that [`Focus::with_size`]
/// refuses because it does not fit inside the frame around the centre.
///
/// FocusLocation2 is read only when FocusLocation gives no location: when
/// it is missing, and also when it is there but is not four SHORTs, a shape
/// no sample shows, which reads as missing. A location of the right shape
/// is FocusLocation's, whatever FocusLocation2 holds. A FocusFrameSize that
/// gives no area ([`area`]) keeps the point without a size.
///
/// See the [module documentation](self) for what the tags record.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn focus<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<Focus>> {
    if manual_focus(src, makernote)? {
        return Ok(None);
    }
    let mut found = location(src, makernote, FOCUS_LOCATION)?;
    if found.is_none() {
        found = location(src, makernote, FOCUS_LOCATION2)?;
    }
    let Some(focus) = found.and_then(|(frame, center)| Focus::new(frame, center)) else {
        return Ok(None);
    };
    // An area that `with_size` refuses costs the whole point: `area` gives
    // none with a zero side, so this one is a record that cannot be used,
    // which gives no focus on every format, as ADR012 decides.
    Ok(match area(src, makernote)? {
        Some(size) => focus.with_size(size),
        None => Some(focus),
    })
}

/// Whether `makernote`'s FocusMode records manual focus: whether it is one
/// BYTE, and that byte is 0. A FocusMode that is missing, of another type
/// or another count, or that holds any other value, such as 3 for AF-C,
/// records no manual focus.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn manual_focus<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<bool> {
    if !has_entry_of_shape(makernote, FOCUS_MODE, BYTE, 1) {
        return Ok(false);
    }
    let mode = skip_damage(makernote.ifd.bytes(src, FOCUS_MODE))?.flatten();
    Ok(mode.as_deref() == Some(&[0]))
}

/// The frame and the centre that `makernote`'s tag `tag`, FocusLocation or
/// FocusLocation2, records: its four SHORTs, the frame's width and height,
/// then the centre's x and y, widened to 32 bits. `None` when the tag is
/// missing, is not of type SHORT, or does not hold exactly four values.
///
/// The values are not checked here: [`Focus::new`] checks them.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn location<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    tag: u16,
) -> Result<Option<(Size, Point)>> {
    if !has_entry_of_shape(makernote, tag, SHORT, 4) {
        return Ok(None);
    }
    let values = skip_damage(makernote.ifd.u32s(src, tag))?.flatten();
    let Some(&[width, height, x, y]) = values.as_deref() else {
        return Ok(None);
    };
    Ok(Some((Size { width, height }, Point { x, y })))
}

/// The size of the focus area that `makernote`'s FocusFrameSize records:
/// the first two of its three 16-bit numbers, the area's width and height,
/// in the MakerNote's byte order, widened to 32 bits. The third is not
/// read.
///
/// `None` when the tag is missing, is not of type UNDEFINED, does not hold
/// exactly six bytes, or records a width or a height of zero, which on
/// Sony says only that no size is recorded (ADR012).
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn area<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<Option<Size>> {
    if !has_entry_of_shape(makernote, FOCUS_FRAME_SIZE, UNDEFINED, 6) {
        return Ok(None);
    }
    let value = skip_damage(makernote.ifd.bytes(src, FOCUS_FRAME_SIZE))?.flatten();
    let Some(&[width0, width1, height0, height1, ..]) = value.as_deref() else {
        return Ok(None);
    };
    let order = makernote.ifd.order();
    let width = u32::from(order.u16([width0, width1]));
    let height = u32::from(order.u16([height0, height1]));
    Ok((width != 0 && height != 0).then_some(Size { width, height }))
}

/// Whether `makernote`'s MakerNote IFD has an entry of tag `tag`, of type
/// `typ`, holding exactly `count` values: the shape its tag's rule allows.
///
/// The readers of values accept more types than a tag's rule allows:
/// [`Ifd::u32s`] reads LONG and IFD as well as SHORT, and [`Ifd::bytes`]
/// BYTE as well as UNDEFINED. The value itself lies inside the block, as
/// every value of the IFD does ([`ifd_in`]).
fn has_entry_of_shape(makernote: &MakerNote, tag: u16, typ: u16, count: u32) -> bool {
    makernote
        .ifd
        .entry(tag)
        .is_some_and(|entry| entry.typ() == typ && entry.count() == count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::tags::MAKE;
    use crate::container::tiff::types::{ASCII, SHORT, UNDEFINED};
    use crate::container::tiff::{Block, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing::{FAR, FailsFar, Src, byte_order};
    use crate::tiff_builder::{Built, IfdId, Order, RawValue, TiffBuilder};
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
            let limits = Limits::default().max_ifd_entries(2);
            assert!(read_with(built.bytes.clone(), &limits).1.is_some());
            let limits = Limits::default().max_ifd_entries(1);
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

    /// The centre that the ILCE-7M4's FocusLocation records in
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

    /// FocusLocation's four SHORTs for a frame `frame` and a centre
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

    /// The focus centred at `center` in `frame`, with an area of `area`
    /// when there is one, and nothing said about whether focus was
    /// confirmed.
    fn expected(frame: [u16; 2], center: [u16; 2], area: Option<[u16; 2]>) -> Focus {
        let [width, height] = frame.map(u32::from);
        let [x, y] = center.map(u32::from);
        let focus = Focus::new(Size { width, height }, Point { x, y }).expect("a usable centre");
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

    #[test]
    fn focus_mode_0_gives_none() {
        // As the ILCE-9M3 writes a manual-focus shot: the frame's exact
        // centre and an area of zeros (public sample 7361).
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
    fn a_centre_on_the_far_edge_is_kept_and_one_past_it_gives_none() {
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
    fn an_area_that_does_not_fit_around_the_centre_gives_none() {
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
            let bytes = makernote.ifd.bytes(&mut src, FOCUS_FRAME_SIZE).unwrap();
            stored.push(bytes.expect("FocusFrameSize"));
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
}
