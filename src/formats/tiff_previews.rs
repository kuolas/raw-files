// SPDX-License-Identifier: MIT OR Apache-2.0

//! The preview finder shared by the TIFF-based formats, and by the JPEG
//! format, whose Exif segment holds a TIFF structure: it finds the JPEG
//! previews a camera embedded in a file, and measures each from its own
//! frame header.
//!
//! A TIFF-based RAW file describes each of its images with an IFD: the raw
//! sensor data, a full-size JPEG preview, a thumbnail. The formats differ
//! in where they put which, but the way to tell a preview from the rest is
//! the same for all of them, so [`find`] does it once, over the whole tree,
//! and a format adds only the candidates it knows of outside the tree, such
//! as a preview its MakerNote points at.
//!
//! [`find`] takes two steps, which a format may also take apart:
//! [`tree_candidates`] gathers the candidates the tree's IFDs declare, and
//! [`check`] checks a list of candidates and measures the ones that pass.
//! A format may check candidates without a tree: a JPEG file with no usable
//! Exif segment still has the images of its MPF list.
//!
//! The rules, in order:
//!
//! - Every IFD of the tree is asked, in the tree's order, except the Exif
//!   IFD, which holds capture metadata, and the Interoperability IFD, which
//!   names the rule the file follows, since neither ever holds an image;
//!   then the format's own candidates, in the order it gives them.
//! - An IFD that describes raw data is skipped, whatever it points at: one
//!   whose samples are a colour filter array (CFA) or Sony's linear raw
//!   (ARQ), one whose samples are wider than 8 bits, one stored in tiles,
//!   and one that says it is the full-resolution image. No camera writes a
//!   preview so. The metadata reader takes the raw size by the same test
//!   ([`is_raw`]), so that the two agree on which IFD holds the raw data.
//! - An IFD is skipped too, with whatever it points at, when a tag these
//!   rules read in it cannot be read, such as a value longer than one tag
//!   may take: damage costs only the IFD it touches, never the other
//!   previews, as in the walk past IFD0.
//! - An IFD's preview is the JPEG stream its JPEGInterchangeFormat and
//!   JPEGInterchangeFormatLength tags declare, when it has both; otherwise
//!   its one strip, when it is JPEG-compressed and stored as exactly one
//!   strip. Two candidates, from IFDs or from the format, at one position
//!   in the file are the same preview: the first that passes the checks
//!   below is kept, and the other is dropped. A candidate that fails them
//!   claims nothing, so a wrong declaration never hides a right one of the
//!   same bytes.
//! - Inside a TIFF structure embedded in a file, such as a JPEG file's
//!   Exif segment, an IFD's offsets count from that structure's header,
//!   and its JPEG must lie inside the structure. So an IFD's candidate
//!   counts from the base of the block the IFD was read in, and is dropped
//!   unless it lies inside that block's bounds as well as the file. A NEF's
//!   or an ARW's own IFDs have the start of the file as their base and the
//!   whole file as their bounds. A format's own candidates are positions in
//!   the file already: the format bounds each by the block it read it in,
//!   as a MakerNote's preview must lie inside the MakerNote, while an MPF
//!   image of a JPEG file, stored after the main image, is bounded by the
//!   file alone, which the next rule checks for every candidate.
//! - A candidate must lie inside the file, as declared: one that is empty,
//!   or runs past the end of the file, is dropped, never cut to fit, since
//!   a preview cut short would not decode.
//! - The bytes must hold a JPEG stream with a frame header, which gives the
//!   preview's size, and the frame must not be lossless: a lossless JPEG is
//!   raw data, as in Sony's lossless ARW files.
//! - Every preview is shown upright by IFD0's Orientation, which describes
//!   the capture; the Orientation of any other IFD is ignored, and a
//!   missing, unknown or unreadable value means none.
//!
//! The previews come back in the order they were found; the file handle
//! sorts them. Nothing here trusts a size from a tag, and the finder never
//! panics on any input: every offset and length read from the file is
//! checked before it is used (ADR008). Nothing the file holds is an error
//! of the finder's; only a failure of the reader is.

use std::collections::BTreeSet;
use std::io::{Read, Seek};

use crate::container::jpeg;
use crate::container::tiff::tags::{
    BITS_PER_SAMPLE, COMPRESSION, JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH,
    NEW_SUBFILE_TYPE, ORIENTATION, PHOTOMETRIC_INTERPRETATION, STRIP_BYTE_COUNTS, STRIP_OFFSETS,
    TILE_BYTE_COUNTS, TILE_LENGTH, TILE_OFFSETS, TILE_WIDTH,
};
use crate::container::tiff::{Block, Ifd, Tree, skip_damage};
use crate::error::Result;
use crate::io::{Source, within};
use crate::orientation::Orientation;
use crate::preview::{Preview, PreviewFormat};

/// The Compression value of JPEG as TIFF 6.0 first defined it, "old-style"
/// JPEG, which cameras still write for their previews.
const OLD_STYLE_JPEG: u32 = 6;

/// The Compression value of JPEG as TIFF Technical Note 2 defines it.
const JPEG: u32 = 7;

/// The PhotometricInterpretation value of a colour filter array: raw
/// sensor data, one colour per pixel (CFA, from TIFF/EP).
const CFA: u32 = 32803;

/// The PhotometricInterpretation value of linear raw data, every colour at
/// every pixel (LinearRaw, from the DNG specification), which Sony's
/// pixel-shift ARQ files hold.
const LINEAR_RAW: u32 = 32892;

/// The widest sample a preview has: a JPEG preview holds 8-bit samples,
/// and raw data more.
const PREVIEW_BITS: u32 = 8;

/// The NewSubfileType value of the full-resolution image, which in a RAW
/// file is the raw data.
const FULL_RESOLUTION: u32 = 0;

/// The tags of an image stored in tiles, which only raw data is.
const TILE_TAGS: [u16; 4] = [TILE_WIDTH, TILE_LENGTH, TILE_OFFSETS, TILE_BYTE_COUNTS];

/// Where a preview may be, as declared, to be checked like every other: an
/// IFD's, or one a format found outside the tree, such as the preview a
/// MakerNote points at or an image of a JPEG file's MPF list.
///
/// A format's own candidates are positions in the file, not offsets as a
/// tag writes them: the format has turned each offset it read into a
/// position. It bounds each candidate by the block it read it in, as a
/// MakerNote's preview must lie inside the MakerNote; an MPF image, stored
/// after the main image, outside the segment that lists it, is bounded by
/// the file alone. [`check`] then drops every candidate, an IFD's or a
/// format's, that does not lie inside the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// Where the preview starts: a position in the file, not an offset as
    /// a tag writes it.
    pub offset: u64,
    /// The preview's length in bytes, as declared.
    pub length: u64,
}

/// Finds the previews of the TIFF-based file in `src`, whose IFDs are
/// `tree`, and of the candidates `extra` that its format found outside the
/// tree.
///
/// See the [module documentation](self) for the rules. The previews are in
/// the order they were found: the tree's, in its order, then `extra`'s.
/// It takes two steps: [`tree_candidates`] gathers the tree's candidates,
/// and [`check`] checks them and `extra`, with IFD0's orientation
/// ([`orientation`]).
///
/// # Errors
///
/// `Io` when the reader fails, whether reading a tag value or probing a
/// candidate. A tag value that cannot be read for any other reason costs
/// its IFD's preview, or, for IFD0's Orientation, the orientation. Probing
/// returns no other error here: it can fail with `Malformed` only on a
/// range that runs past the end of the file, and no candidate that does is
/// probed.
pub(crate) fn find<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    extra: &[Candidate],
) -> Result<Vec<Preview>> {
    let orientation = orientation(src, tree)?;
    let mut candidates = tree_candidates(src, tree)?;
    candidates.extend_from_slice(extra);
    check(src, &candidates, orientation)
}

/// The candidates that the IFDs of `tree` declare, in the tree's order:
/// the first step of [`find`].
///
/// The Exif IFD and the Interoperability IFD are not asked, an IFD that
/// describes raw data is skipped, and so is an IFD whose tags these rules
/// read cannot be read; see the [module documentation](self). Each
/// candidate counts from the base of the block its IFD was read in, and
/// lies inside that block's bounds and the file.
///
/// # Errors
///
/// `Io` when the reader fails. A tag value that cannot be read for any
/// other reason costs its IFD's candidate.
pub(crate) fn tree_candidates<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    for (location, ifd) in tree.iter() {
        if location.holds_no_image() {
            continue;
        }
        // An IFD whose tags cannot be read is skipped as raw data is,
        // rather than read as if the tag were missing: what it says of
        // itself is unknown.
        if skip_damage(is_raw(src, ifd))? != Some(false) {
            continue;
        }
        candidates.extend(skip_damage(declared(src, ifd))?.flatten());
    }
    Ok(candidates)
}

/// The previews among `candidates`, in their order, each shown upright by
/// `orientation`: the second step of [`find`], which a format may also
/// take without a tree, as a JPEG file with no usable Exif segment does
/// for the images of its MPF list.
///
/// A candidate is dropped when it is empty or does not lie wholly inside
/// the file, when its bytes hold no JPEG stream with a frame header that
/// gives a size, when that frame is lossless, and when a candidate before
/// it at the same position became a preview; see the
/// [module documentation](self). Each preview's size is its frame
/// header's.
///
/// # Errors
///
/// `Io` when the reader fails probing a candidate. Probing returns no
/// other error here: it can fail with `Malformed` only on a range that
/// runs past the end of the file, and no candidate that does is probed.
pub(crate) fn check<R: Read + Seek>(
    src: &mut Source<R>,
    candidates: &[Candidate],
    orientation: Orientation,
) -> Result<Vec<Preview>> {
    let mut seen = BTreeSet::new();
    let mut previews = Vec::new();
    for &candidate in candidates {
        if seen.contains(&candidate.offset) || !inside(candidate, src.len()) {
            continue;
        }
        let Some(frame) = jpeg::probe(src, candidate.offset, candidate.length)? else {
            continue;
        };
        if frame.is_lossless() {
            continue;
        }
        // Only a candidate that became a preview claims its position, so a
        // wrong declaration never hides a right one of the same bytes.
        seen.insert(candidate.offset);
        previews.push(Preview::new(
            candidate.offset,
            candidate.length,
            u32::from(frame.width),
            u32::from(frame.height),
            PreviewFormat::Jpeg,
            orientation,
        ));
    }
    Ok(previews)
}

/// The orientation of every preview: IFD0's Orientation, or none when it
/// is missing, cannot be read or is not one of the eight EXIF values.
///
/// The metadata reader (`tiff_metadata`) gives the file's metadata this
/// orientation too, so that the metadata and the previews never disagree.
///
/// # Errors
///
/// `Io` when the reader fails; a value that cannot be read for any other
/// reason means none.
pub(crate) fn orientation<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Orientation> {
    Ok(skip_damage(tree.ifd0().u32(src, ORIENTATION))?
        .flatten()
        .and_then(|value| u16::try_from(value).ok())
        .and_then(Orientation::from_exif)
        .unwrap_or(Orientation::NoTransforms))
}

/// Whether `ifd` describes raw data, by the role its tags give it, so that
/// nothing it points at is a preview.
///
/// The metadata reader (`tiff_metadata`) takes the raw size from the
/// largest IFD this says is raw data, so that the finder and the metadata
/// agree on which IFDs hold it.
///
/// A tag whose value the test reads says nothing either way when it is
/// missing or not of an integer type that [`Ifd::u32s`] reads; a tile tag
/// counts by its presence alone, whatever its type.
///
/// # Errors
///
/// As for [`Ifd::u32s`]: `LimitExceeded` for a value longer than one tag
/// may take, which [`find`] and the metadata reader take for damage and
/// skip the IFD; `Io` when the reader fails.
pub(crate) fn is_raw<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd) -> Result<bool> {
    if matches!(
        ifd.u32(src, PHOTOMETRIC_INTERPRETATION)?,
        Some(CFA | LINEAR_RAW)
    ) {
        return Ok(true);
    }
    // Samples wider than a JPEG preview's are raw data, whatever the
    // PhotometricInterpretation says: this also covers raw data that calls
    // itself YCbCr (6) with samples above 8 bits, while an 8-bit YCbCr image
    // is a preview.
    if ifd
        .u32s(src, BITS_PER_SAMPLE)?
        .is_some_and(|bits| bits.iter().any(|&b| b > PREVIEW_BITS))
    {
        return Ok(true);
    }
    // A tile tag of any type: its presence is what says the image is tiled.
    if TILE_TAGS.iter().any(|&tag| ifd.entry(tag).is_some()) {
        return Ok(true);
    }
    Ok(ifd.u32(src, NEW_SUBFILE_TYPE)? == Some(FULL_RESOLUTION))
}

/// Where `ifd` says its JPEG stream is, if it says so: the stream that
/// JPEGInterchangeFormat and JPEGInterchangeFormatLength declare, or else a
/// JPEG-compressed image's one strip; `None` when the stream it declares
/// does not lie inside the IFD's block and the file.
///
/// The offset is written as every offset inside the IFD's TIFF structure
/// is, counted from the structure's header, the base of the block the IFD
/// was read in ([`Ifd::block`]). That is the start of the file for a NEF's
/// or an ARW's own IFDs, and the TIFF header inside the Exif segment for a
/// JPEG file's, whose thumbnail must then lie inside the segment as well
/// as the file.
///
/// # Errors
///
/// As for [`is_raw`]: `LimitExceeded`, which [`find`] takes for damage and
/// skips the IFD, and `Io`.
fn declared<R: Read + Seek>(src: &mut Source<R>, ifd: &Ifd) -> Result<Option<Candidate>> {
    let block = ifd.block().within_file(src.len());
    let offset = ifd.u32(src, JPEG_INTERCHANGE_FORMAT)?;
    let length = ifd.u32(src, JPEG_INTERCHANGE_FORMAT_LENGTH)?;
    if let (Some(offset), Some(length)) = (offset, length) {
        return Ok(candidate(&block, offset, length));
    }
    if !matches!(ifd.u32(src, COMPRESSION)?, Some(OLD_STYLE_JPEG | JPEG)) {
        return Ok(None);
    }
    let offsets = ifd.u32s(src, STRIP_OFFSETS)?;
    let lengths = ifd.u32s(src, STRIP_BYTE_COUNTS)?;
    Ok(match (offsets.as_deref(), lengths.as_deref()) {
        (Some(&[offset]), Some(&[length])) => candidate(&block, offset, length),
        _ => None,
    })
}

/// The candidate of `length` bytes at `offset`, as a tag of an IFD read in
/// `block` writes them: at the position `offset` gives inside the block
/// ([`Block::at`]), when the whole of it lies inside the block's bounds,
/// an empty candidate included; `None` when it does not, or when that
/// position would pass the largest offset.
///
/// The bounds are used as given. A caller whose bounds may run past the
/// end of the file cuts them at it first ([`Block::within_file`]), as
/// [`declared`] does; one whose bounds lie inside the file, such as a
/// MakerNote's, passes its block as it is.
pub(crate) fn candidate(block: &Block, offset: u32, length: u32) -> Option<Candidate> {
    let offset = block.at(offset)?;
    let length = u64::from(length);
    block
        .fits(offset, length)
        .then_some(Candidate { offset, length })
}

/// Whether `candidate` is not empty and lies wholly inside a file of
/// `file_len` bytes. A non-empty candidate that ends inside the file also
/// starts inside it.
fn inside(candidate: Candidate, file_len: u64) -> bool {
    candidate.length != 0 && within(candidate.offset, candidate.length, file_len).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::{parse_ifd, walk};
    use crate::error::ErrorKind;
    use crate::formats::testing::byte_order;
    use crate::jpeg_stub::jpeg_stub;
    use crate::limits::Limits;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{BlobId, Built, IfdId, Order, RawValue, SHORT, TiffBuilder};
    use std::io::Cursor;
    use std::ops::Range;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// A tag no rule reads, for an IFD that needs an entry and nothing else:
    /// ImageWidth.
    const IMAGE_WIDTH: u16 = 0x0100;

    /// ImageLength, the height the tags declare, which the finder ignores.
    const IMAGE_LENGTH: u16 = 0x0101;

    /// The result of finding the previews of `bytes`, with `extra`.
    fn try_find(bytes: &[u8], extra: &[Candidate]) -> Result<Vec<Preview>> {
        let mut src = Source::new(Cursor::new(bytes.to_vec()))?;
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default())?;
        find(&mut src, &tree, extra)
    }

    /// The previews of `bytes`, with `extra`; the finder must succeed.
    fn found_with(bytes: &[u8], extra: &[Candidate]) -> Vec<Preview> {
        try_find(bytes, extra).unwrap()
    }

    /// The previews of `bytes`; the finder must succeed.
    fn found(bytes: &[u8]) -> Vec<Preview> {
        found_with(bytes, &[])
    }

    /// The stored size of each preview, in order: each test gives its JPEGs
    /// distinct sizes, so these name them.
    fn sizes(previews: &[Preview]) -> Vec<(u32, u32)> {
        previews
            .iter()
            .map(|p| (p.size().width, p.size().height))
            .collect()
    }

    /// An IFD holding only an ImageWidth, so that it can be read.
    fn plain_ifd(builder: &mut TiffBuilder) -> IfdId {
        let ifd = builder.ifd();
        builder.short(ifd, IMAGE_WIDTH, &[1]);
        ifd
    }

    /// Adds a stub JPEG of `width` by `height` with marker `marker` to the
    /// file, and points `ifd`'s JPEGInterchangeFormat and
    /// JPEGInterchangeFormatLength at it.
    fn jpeg_with_marker(
        builder: &mut TiffBuilder,
        ifd: IfdId,
        width: u16,
        height: u16,
        marker: u8,
    ) -> BlobId {
        let blob = builder.blob(&jpeg_stub(width, height, marker));
        builder.long_blob(ifd, JPEG_INTERCHANGE_FORMAT, blob);
        builder.long_blob_len(ifd, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);
        blob
    }

    /// As [`jpeg_with_marker`], with a baseline frame header.
    fn jpeg(builder: &mut TiffBuilder, ifd: IfdId, width: u16, height: u16) -> BlobId {
        jpeg_with_marker(builder, ifd, width, height, 0xC0)
    }

    /// The length of a stub JPEG.
    fn stub_len() -> u64 {
        u64::try_from(jpeg_stub(1, 1, 0xC0).len()).unwrap()
    }

    /// The previews of a little-endian file whose one IFD points at a
    /// 640 by 480 JPEG through JPEGInterchangeFormat and also holds what
    /// `tags` adds.
    fn one_ifd(tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> Vec<Preview> {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        jpeg(&mut builder, ifd0, 640, 480);
        tags(&mut builder, ifd0);
        found(&builder.build().bytes)
    }

    /// The previews of a little-endian file whose one IFD has compression
    /// `compression` and holds what `tags` adds.
    fn strips(compression: u16, tags: impl FnOnce(&mut TiffBuilder, IfdId)) -> Vec<Preview> {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        builder.short(ifd0, COMPRESSION, &[compression]);
        tags(&mut builder, ifd0);
        found(&builder.build().bytes)
    }

    /// Adds a stub JPEG of `width` by `height` and makes it `ifd`'s one
    /// strip.
    fn one_strip(builder: &mut TiffBuilder, ifd: IfdId, width: u16, height: u16) -> BlobId {
        let blob = builder.blob(&jpeg_stub(width, height, 0xC0));
        builder.long_blob(ifd, STRIP_OFFSETS, blob);
        builder.long_blob_len(ifd, STRIP_BYTE_COUNTS, blob);
        blob
    }

    #[test]
    fn a_jpeg_in_ifd0_is_found_with_its_measured_size() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            // Tags that declare another size, which the finder ignores.
            builder.short(ifd0, IMAGE_WIDTH, &[160]);
            builder.short(ifd0, IMAGE_LENGTH, &[120]);
            let blob = jpeg(&mut builder, ifd0, 640, 480);
            let built = builder.build();
            let previews = found(&built.bytes);

            assert_eq!(previews.len(), 1, "{order:?}");
            let preview = previews[0];
            assert_eq!((preview.size().width, preview.size().height), (640, 480));
            assert_eq!(preview.byte_range().start, built.blob_offset(blob));
            assert_eq!(preview.byte_len(), stub_len());
            assert_eq!(preview.format(), PreviewFormat::Jpeg);
            assert_eq!(preview.orientation(), Orientation::NoTransforms);
        }
    }

    #[test]
    fn the_exif_ifd_is_never_a_candidate() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = plain_ifd(&mut builder);
        let exif = builder.ifd();
        jpeg(&mut builder, exif, 640, 480);
        builder.exif(ifd0, exif);
        assert_eq!(found(&builder.build().bytes), []);
    }

    #[test]
    fn the_interoperability_ifd_is_never_a_candidate() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = plain_ifd(&mut builder);
        let (exif, interop) = (plain_ifd(&mut builder), builder.ifd());
        jpeg(&mut builder, interop, 640, 480);
        builder.exif(ifd0, exif).interop(exif, interop);
        assert_eq!(found(&builder.build().bytes), []);
    }

    #[test]
    fn chain_ifds_and_subifds_are_candidates() {
        for order in ORDERS {
            // IFD0 → IFD1, and a SubIFD of IFD0; the tree's order is IFD0,
            // its SubIFD, IFD1.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let (sub, ifd1) = (builder.ifd(), builder.ifd());
            jpeg(&mut builder, ifd0, 160, 120);
            jpeg(&mut builder, sub, 1620, 1080);
            jpeg(&mut builder, ifd1, 640, 480);
            builder.subifds(ifd0, &[sub]).next(ifd0, Some(ifd1));
            let previews = found(&builder.build().bytes);
            assert_eq!(
                sizes(&previews),
                [(160, 120), (1620, 1080), (640, 480)],
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_cfa_ifd_is_raw_data() {
        let previews = one_ifd(|builder, ifd| {
            builder.short(ifd, PHOTOMETRIC_INTERPRETATION, &[32803]);
        });
        assert_eq!(previews, []);
    }

    #[test]
    fn a_linear_raw_ifd_is_raw_data() {
        let previews = one_ifd(|builder, ifd| {
            builder.short(ifd, PHOTOMETRIC_INTERPRETATION, &[32892]);
        });
        assert_eq!(previews, []);
    }

    #[test]
    fn samples_wider_than_8_bits_are_raw_data() {
        for bits in [&[14, 14, 14][..], &[8, 8, 9], &[12]] {
            let previews = one_ifd(|builder, ifd| {
                builder.short(ifd, BITS_PER_SAMPLE, bits);
            });
            assert_eq!(previews, [], "{bits:?}");
        }
    }

    #[test]
    fn an_8_bit_ycbcr_ifd_is_a_preview() {
        // The ARW layout: an IFD of YCbCr samples, 8 bits each, whose JPEG
        // is the full-size preview.
        let previews = one_ifd(|builder, ifd| {
            builder.short(ifd, PHOTOMETRIC_INTERPRETATION, &[6]);
            builder.short(ifd, BITS_PER_SAMPLE, &[8, 8, 8]);
        });
        assert_eq!(sizes(&previews), [(640, 480)]);
    }

    #[test]
    fn a_tiled_ifd_is_raw_data() {
        for tag in TILE_TAGS {
            let previews = one_ifd(|builder, ifd| {
                builder.long(ifd, tag, &[256]);
            });
            assert_eq!(previews, [], "tag {tag:#06X}");
        }
    }

    #[test]
    fn the_full_resolution_ifd_is_raw_data() {
        let previews = one_ifd(|builder, ifd| {
            builder.long(ifd, NEW_SUBFILE_TYPE, &[0]);
        });
        assert_eq!(previews, []);
    }

    #[test]
    fn a_reduced_resolution_ifd_is_a_preview() {
        let previews = one_ifd(|builder, ifd| {
            builder.long(ifd, NEW_SUBFILE_TYPE, &[1]);
        });
        assert_eq!(sizes(&previews), [(640, 480)]);
    }

    #[test]
    fn a_single_jpeg_strip_is_found() {
        for compression in [6, 7] {
            let mut builder = TiffBuilder::new(Order::Big);
            let ifd0 = builder.ifd();
            builder.short(ifd0, COMPRESSION, &[compression]);
            let blob = one_strip(&mut builder, ifd0, 640, 480);
            let built = builder.build();
            let previews = found(&built.bytes);
            assert_eq!(sizes(&previews), [(640, 480)], "compression {compression}");
            assert_eq!(previews[0].byte_range().start, built.blob_offset(blob));
            assert_eq!(previews[0].byte_len(), stub_len());
        }
    }

    #[test]
    fn an_uncompressed_strip_is_not_a_preview() {
        let previews = strips(1, |builder, ifd| {
            one_strip(builder, ifd, 640, 480);
        });
        assert_eq!(previews, []);
    }

    #[test]
    fn two_jpeg_strips_are_not_a_preview() {
        let stub = jpeg_stub(640, 480, 0xC0);
        let len = u32::try_from(stub.len()).unwrap();
        let previews = strips(6, |builder, ifd| {
            builder.blob_at(4096, &stub);
            builder.blob_at(8192, &stub);
            builder.long(ifd, STRIP_OFFSETS, &[4096, 8192]);
            builder.long(ifd, STRIP_BYTE_COUNTS, &[len, len]);
        });
        assert_eq!(previews, []);
    }

    #[test]
    fn strip_offsets_without_strip_byte_counts_are_not_a_preview() {
        let previews = strips(6, |builder, ifd| {
            let blob = builder.blob(&jpeg_stub(640, 480, 0xC0));
            builder.long_blob(ifd, STRIP_OFFSETS, blob);
        });
        assert_eq!(previews, []);
    }

    #[test]
    fn jpeg_interchange_format_comes_before_the_strip() {
        let previews = strips(6, |builder, ifd| {
            one_strip(builder, ifd, 320, 240);
            jpeg(builder, ifd, 640, 480);
        });
        assert_eq!(sizes(&previews), [(640, 480)]);
    }

    #[test]
    fn two_ifds_naming_one_jpeg_give_one_preview() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let sub = builder.ifd();
        let blob = jpeg(&mut builder, ifd0, 640, 480);
        builder.long_blob(sub, JPEG_INTERCHANGE_FORMAT, blob);
        builder.long_blob_len(sub, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);
        builder.subifds(ifd0, &[sub]);
        assert_eq!(sizes(&found(&builder.build().bytes)), [(640, 480)]);
    }

    #[test]
    fn an_extra_candidate_is_found_after_the_tree() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        jpeg(&mut builder, ifd0, 160, 120);
        let blob = builder.blob(&jpeg_stub(640, 480, 0xC0));
        let built = builder.build();
        let extra = Candidate {
            offset: built.blob_offset(blob),
            length: stub_len(),
        };
        let previews = found_with(&built.bytes, &[extra]);
        assert_eq!(sizes(&previews), [(160, 120), (640, 480)]);
        assert_eq!(previews[1].byte_range().start, extra.offset);
        assert_eq!(previews[1].byte_len(), extra.length);
    }

    #[test]
    fn an_extra_candidate_at_an_offset_the_tree_gave_is_the_same_preview() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let blob = jpeg(&mut builder, ifd0, 640, 480);
        // Room for the extra's longer declared length.
        builder.pad_to(64 * 1024);
        let built = builder.build();
        let extra = Candidate {
            offset: built.blob_offset(blob),
            length: stub_len() + 100,
        };
        let previews = found_with(&built.bytes, &[extra]);
        // The tree's came first, and wins.
        assert_eq!(sizes(&previews), [(640, 480)]);
        assert_eq!(previews[0].byte_len(), stub_len());
    }

    #[test]
    fn a_dropped_candidate_does_not_hide_a_later_one_at_its_offset() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let blob = builder.blob(&jpeg_stub(640, 480, 0xC0));
        // IFD0 names the JPEG's offset with a length of 0: an empty candidate.
        builder.long_blob(ifd0, JPEG_INTERCHANGE_FORMAT, blob);
        builder.long(ifd0, JPEG_INTERCHANGE_FORMAT_LENGTH, &[0]);
        let built = builder.build();
        let extra = Candidate {
            offset: built.blob_offset(blob),
            length: stub_len(),
        };
        let previews = found_with(&built.bytes, &[extra]);
        assert_eq!(sizes(&previews), [(640, 480)]);
        assert_eq!(previews[0].byte_len(), stub_len());
    }

    #[test]
    fn candidates_are_checked_without_a_tree() {
        // Bytes that hold no TIFF structure, as a JPEG file without an Exif
        // segment does: zeros, a baseline JPEG, a lossless one, and a
        // baseline one that ends the file.
        let mut bytes = vec![0; 16];
        let mut place = |stub: Vec<u8>| {
            let at = u64::try_from(bytes.len()).unwrap();
            bytes.extend(stub);
            at
        };
        let baseline = place(jpeg_stub(640, 480, 0xC0));
        let lossless = place(jpeg_stub(512, 256, 0xC3));
        let last = place(jpeg_stub(160, 120, 0xC0));
        let candidates = [
            Candidate {
                offset: baseline,
                length: stub_len(),
            },
            Candidate {
                offset: lossless,
                length: stub_len(),
            },
            // One byte longer than the file holds.
            Candidate {
                offset: last,
                length: stub_len() + 1,
            },
            // At the position of a candidate that became a preview.
            Candidate {
                offset: baseline,
                length: stub_len(),
            },
        ];
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        let previews = check(&mut src, &candidates, Orientation::Rotate90).unwrap();
        assert_eq!(
            previews,
            [Preview::new(
                baseline,
                stub_len(),
                640,
                480,
                PreviewFormat::Jpeg,
                Orientation::Rotate90,
            )]
        );
    }

    #[test]
    fn an_empty_candidate_is_dropped() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let blob = builder.blob(&jpeg_stub(640, 480, 0xC0));
        builder.long_blob(ifd0, JPEG_INTERCHANGE_FORMAT, blob);
        builder.long(ifd0, JPEG_INTERCHANGE_FORMAT_LENGTH, &[0]);
        assert_eq!(found(&builder.build().bytes), []);
    }

    #[test]
    fn a_candidate_starting_past_the_end_of_the_file_is_dropped() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        builder.long(ifd0, JPEG_INTERCHANGE_FORMAT, &[0x0010_0000]);
        builder.long(ifd0, JPEG_INTERCHANGE_FORMAT_LENGTH, &[100]);
        let built = builder.build();
        let len = u64::try_from(built.bytes.len()).unwrap();
        // At the end of the file, past it, and so far past it that the end
        // overflows.
        let extra = [
            Candidate {
                offset: len,
                length: 10,
            },
            Candidate {
                offset: u64::MAX,
                length: 10,
            },
        ];
        assert_eq!(found_with(&built.bytes, &extra), []);
    }

    #[test]
    fn a_candidate_running_past_the_end_of_the_file_is_dropped_not_cut() {
        for order in ORDERS {
            // The JPEG is the last thing in the file; cutting the file short
            // keeps its frame header, so a candidate cut to fit would still
            // probe as a preview.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            jpeg(&mut builder, ifd0, 640, 480);
            let built = builder.build();
            assert_eq!(sizes(&found(&built.bytes)), [(640, 480)], "{order:?}");
            let short = built.truncated(built.bytes.len() - 2);
            assert_eq!(found(&short), [], "{order:?}");
        }
    }

    #[test]
    fn bytes_that_are_not_a_jpeg_are_not_a_preview() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let blob = builder.blob(b"not a JPEG stream, only text");
        builder.long_blob(ifd0, JPEG_INTERCHANGE_FORMAT, blob);
        builder.long_blob_len(ifd0, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);
        assert_eq!(found(&builder.build().bytes), []);
    }

    #[test]
    fn a_lossless_jpeg_is_raw_data_and_a_baseline_one_beside_it_a_preview() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let sub = builder.ifd();
        jpeg_with_marker(&mut builder, ifd0, 512, 256, 0xC3);
        jpeg(&mut builder, sub, 640, 480);
        builder.subifds(ifd0, &[sub]);
        assert_eq!(sizes(&found(&builder.build().bytes)), [(640, 480)]);
    }

    #[test]
    fn every_preview_takes_ifd0s_orientation() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let sub = builder.ifd();
            builder.short(ifd0, ORIENTATION, &[8]);
            // The SubIFD's own Orientation does not count.
            builder.short(sub, ORIENTATION, &[1]);
            jpeg(&mut builder, ifd0, 160, 120);
            jpeg(&mut builder, sub, 640, 480);
            builder.subifds(ifd0, &[sub]);
            let previews = found(&builder.build().bytes);
            assert_eq!(previews.len(), 2, "{order:?}");
            for preview in previews {
                assert_eq!(preview.orientation(), Orientation::Rotate270, "{order:?}");
            }
        }
    }

    #[test]
    fn a_missing_orientation_means_none() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let sub = builder.ifd();
        // Only the SubIFD has one, which does not count.
        builder.short(sub, ORIENTATION, &[6]);
        jpeg(&mut builder, sub, 640, 480);
        builder.subifds(ifd0, &[sub]);
        let previews = found(&builder.build().bytes);
        assert_eq!(sizes(&previews), [(640, 480)]);
        assert_eq!(previews[0].orientation(), Orientation::NoTransforms);
    }

    #[test]
    fn an_invalid_orientation_means_none() {
        // 0 and 9 are no EXIF orientation; 0x1_0006 is none either, though
        // its low 16 bits are 6.
        for value in [0, 9, 0x1_0006] {
            let previews = one_ifd(|builder, ifd| {
                builder.long(ifd, ORIENTATION, &[value]);
            });
            assert_eq!(sizes(&previews), [(640, 480)], "{value}");
            assert_eq!(
                previews[0].orientation(),
                Orientation::NoTransforms,
                "{value}"
            );
        }
    }

    #[test]
    fn the_previews_are_in_the_order_they_were_found() {
        // The large one first, in IFD0: sorting, which is the file
        // handle's job, would put the small one first.
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let sub = builder.ifd();
        jpeg(&mut builder, ifd0, 1620, 1080);
        jpeg(&mut builder, sub, 160, 120);
        builder.subifds(ifd0, &[sub]);
        let previews = found(&builder.build().bytes);
        assert_eq!(sizes(&previews), [(1620, 1080), (160, 120)]);
    }

    /// More SHORTs than the 1 MiB one tag value may take: a value that lies
    /// inside the file and cannot be read.
    const TOO_MANY_SHORTS: u32 = 512 * 1024 + 1;

    /// Gives `ifd` tag `tag` with [`TOO_MANY_SHORTS`] SHORTs at offset
    /// 4096, and makes the file long enough to hold them.
    fn unreadable(builder: &mut TiffBuilder, ifd: IfdId, tag: u16) {
        builder.raw(ifd, tag, SHORT, TOO_MANY_SHORTS, RawValue::Offset(4096));
        builder.pad_to(4096 + 2 * u64::from(TOO_MANY_SHORTS));
    }

    #[test]
    fn an_ifd_whose_tag_cannot_be_read_is_skipped() {
        for order in ORDERS {
            // IFD0's BitsPerSample cannot be read, so whether it describes
            // raw data is unknown: its JPEG is not taken, and the SubIFD's
            // still is.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let sub = builder.ifd();
            jpeg(&mut builder, ifd0, 640, 480);
            unreadable(&mut builder, ifd0, BITS_PER_SAMPLE);
            jpeg(&mut builder, sub, 160, 120);
            builder.subifds(ifd0, &[sub]);
            assert_eq!(
                sizes(&found(&builder.build().bytes)),
                [(160, 120)],
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_ifd_whose_jpeg_tags_cannot_be_read_is_skipped() {
        for order in ORDERS {
            // The SubIFD's StripOffsets holds 262,145 values, one megabyte
            // and four bytes: over the cap on a tag's size. Where the JPEG
            // is stored is unknown, so the SubIFD costs only its own
            // preview, and IFD0's is still found.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let sub = builder.ifd();
            jpeg(&mut builder, ifd0, 640, 480);
            builder.short(sub, COMPRESSION, &[6]);
            builder.long(sub, STRIP_OFFSETS, &vec![0u32; 262_145]);
            builder.long(sub, STRIP_BYTE_COUNTS, &[1]);
            builder.subifds(ifd0, &[sub]);
            assert_eq!(
                sizes(&found(&builder.build().bytes)),
                [(640, 480)],
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_unreadable_orientation_means_none() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let sub = builder.ifd();
            unreadable(&mut builder, ifd0, ORIENTATION);
            jpeg(&mut builder, sub, 640, 480);
            builder.subifds(ifd0, &[sub]);
            let previews = found(&builder.build().bytes);
            assert_eq!(sizes(&previews), [(640, 480)], "{order:?}");
            assert_eq!(
                previews[0].orientation(),
                Orientation::NoTransforms,
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_reader_failure_is_the_finders_error() {
        // A tag value the rules read, stored far away: reading it fails.
        let mut tag = TiffBuilder::new(Order::Little);
        let ifd0 = tag.ifd();
        jpeg(&mut tag, ifd0, 640, 480);
        // Three 8-bit samples, in the little-endian order of the file.
        tag.blob_at(2 * FAR, &[8, 0, 8, 0, 8, 0]);
        let far = u32::try_from(2 * FAR).unwrap();
        tag.raw(ifd0, BITS_PER_SAMPLE, SHORT, 3, RawValue::Offset(far));

        // The JPEG stored far away: probing it fails.
        let mut probe = TiffBuilder::new(Order::Little);
        let ifd0 = probe.ifd();
        let blob = probe.blob_at(2 * FAR, &jpeg_stub(640, 480, 0xC0));
        probe.long_blob(ifd0, JPEG_INTERCHANGE_FORMAT, blob);
        probe.long_blob_len(ifd0, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);

        for (case, builder) in [("tag", tag), ("probe", probe)] {
            let bytes = builder.build().bytes;
            // The file is sound, for a reader that does not fail.
            assert_eq!(sizes(&found(&bytes)), [(640, 480)], "{case}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = Block::file(src.len());
            let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
            let err = find(&mut src, &tree, &[]).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{case}: {err:?}");
        }
    }

    /// Where the tests of a TIFF structure inside a larger file place its
    /// header, as a JPEG file holds the structure of its Exif segment: far
    /// enough in that an offset counted from the start of the file lands
    /// somewhere else.
    const POS: u64 = 1000;

    /// The bytes of a file that holds `built` at [`POS`], after as many
    /// bytes of filler, and goes on `tail` bytes past it.
    fn placed(built: &Built, tail: usize) -> Vec<u8> {
        let mut bytes = vec![0; usize::try_from(POS).unwrap()];
        bytes.extend_from_slice(&built.bytes);
        bytes.resize(bytes.len() + tail, 0);
        bytes
    }

    /// The length of the structure `built`.
    fn len_of(built: &Built) -> u64 {
        u64::try_from(built.bytes.len()).unwrap()
    }

    /// The previews of `bytes`, whose TIFF structure has its header at
    /// [`POS`] and may use the bytes `bounds`; the finder must succeed.
    fn found_in(bytes: &[u8], bounds: Range<u64>) -> Vec<Preview> {
        let mut src = Source::new(Cursor::new(bytes.to_vec())).unwrap();
        let tree = walk(&mut src, &Block::new(POS, bounds), &Limits::default()).unwrap();
        find(&mut src, &tree, &[]).unwrap()
    }

    /// Writes into `bytes`, at `at`, a stub JPEG of 99 by 77: what a finder
    /// that counted an offset from the wrong place would find.
    fn decoy(bytes: &mut [u8], at: u64) {
        let stub = jpeg_stub(99, 77, 0xC0);
        bytes[usize::try_from(at).unwrap()..][..stub.len()].copy_from_slice(&stub);
    }

    #[test]
    fn a_thumbnail_inside_a_file_counts_from_its_structures_header() {
        for order in ORDERS {
            // IFD0, then IFD1 with a JPEG thumbnail, as a JPEG file's Exif
            // segment holds them. Counted from the start of the file, the
            // thumbnail's offset lands on a decoy, which the bounds, the
            // whole file, do not leave out: only the base is at stake.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = plain_ifd(&mut builder);
            let ifd1 = builder.ifd();
            let blob = jpeg(&mut builder, ifd1, 160, 120);
            builder.next(ifd0, Some(ifd1));
            let built = builder.build();
            let at = built.blob_offset(blob);
            assert!(at + stub_len() <= POS, "the decoy lies before the header");
            let mut bytes = placed(&built, 0);
            decoy(&mut bytes, at);

            let previews = found_in(&bytes, 0..POS + len_of(&built));
            assert_eq!(sizes(&previews), [(160, 120)], "{order:?}");
            assert_eq!(previews[0].byte_range().start, POS + at, "{order:?}");
            assert_eq!(previews[0].byte_len(), stub_len(), "{order:?}");
        }
    }

    #[test]
    fn a_jpeg_strip_inside_a_file_counts_from_its_structures_header() {
        for order in ORDERS {
            // As the thumbnail: counted from the start of the file, the
            // strip's offset lands on a decoy.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            builder.short(ifd0, COMPRESSION, &[7]);
            let blob = one_strip(&mut builder, ifd0, 640, 480);
            let built = builder.build();
            let at = built.blob_offset(blob);
            assert!(at + stub_len() <= POS, "the decoy lies before the header");
            let mut bytes = placed(&built, 0);
            decoy(&mut bytes, at);

            let previews = found_in(&bytes, 0..POS + len_of(&built));
            assert_eq!(sizes(&previews), [(640, 480)], "{order:?}");
            assert_eq!(previews[0].byte_range().start, POS + at, "{order:?}");
            assert_eq!(previews[0].byte_len(), stub_len(), "{order:?}");
        }
    }

    #[test]
    fn a_jpeg_past_the_end_of_its_ifds_block_is_dropped() {
        for order in ORDERS {
            // IFD0's JPEG is the last thing in the structure, and the file
            // goes on past the bounds, so that the bounds, not the file, are
            // what leave it out.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let blob = jpeg(&mut builder, ifd0, 640, 480);
            let built = builder.build();
            let (at, len) = (built.blob_offset(blob), len_of(&built));
            assert_eq!(at + stub_len(), len, "the JPEG ends the structure");
            let bytes = placed(&built, 100);

            for (end, expected, what) in [
                (len, &[(640, 480)][..], "ending exactly at the bounds' end"),
                (len - 1, &[], "ending one byte past it"),
                (at, &[], "wholly past it"),
            ] {
                let previews = found_in(&bytes, POS..POS + end);
                assert_eq!(sizes(&previews), expected, "{what} ({order:?})");
            }
        }
    }

    #[test]
    fn an_offset_whose_position_would_overflow_gives_no_candidate() {
        for order in ORDERS {
            for strip in [false, true] {
                // The IFD declares its JPEG at the largest offset, in a
                // block whose base that offset carries past the largest
                // position: wrapped around, it would land on the JPEG. No
                // walk reads a header so far in, so the IFD is read alone.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = builder.ifd();
                let blob = builder.blob(&jpeg_stub(640, 480, 0xC0));
                let length = u32::try_from(stub_len()).unwrap();
                if strip {
                    builder.short(ifd0, COMPRESSION, &[7]);
                    builder.long(ifd0, STRIP_OFFSETS, &[u32::MAX]);
                    builder.long(ifd0, STRIP_BYTE_COUNTS, &[length]);
                } else {
                    builder.long(ifd0, JPEG_INTERCHANGE_FORMAT, &[u32::MAX]);
                    builder.long(ifd0, JPEG_INTERCHANGE_FORMAT_LENGTH, &[length]);
                }
                let built = builder.build();
                let base = built.blob_offset(blob).wrapping_sub(u64::from(u32::MAX));
                assert_eq!(base.checked_add(u64::from(u32::MAX)), None);

                let mut src = Source::new(Cursor::new(built.bytes.clone())).unwrap();
                let block = Block::new(base, 0..src.len());
                let (ifd, _) = parse_ifd(
                    &mut src,
                    &block,
                    built.ifd_offset(ifd0),
                    byte_order(order),
                    &Limits::default(),
                )
                .unwrap();
                assert_eq!(
                    declared(&mut src, &ifd).unwrap(),
                    None,
                    "strip {strip} ({order:?})"
                );
            }
        }
    }
}
