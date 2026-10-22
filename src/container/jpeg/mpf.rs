// SPDX-License-Identifier: MIT OR Apache-2.0

//! The MPF index of a JPEG file: where the images the file holds after its
//! first one lie.
//!
//! The Multi-Picture Format (MPF), CIPA DC-007, lets one JPEG file hold
//! several images, each a whole JPEG stream from SOI to EOI: the main
//! image first, then others, such as large thumbnails, smaller copies of
//! the picture for screens (DC-007 5.1 and Figure 1, pp. 7 to 8; 6.1.2.2
//! and Table 9, pp. 30 to 31). A decoder reads the first image and stops
//! at its EOI, so the others are found only through the index that the
//! first image carries. They are previews of the file (ADR014), checked as
//! every preview is; [`images`] says where they lie.
//!
//! # Where the index lies
//!
//! The index lies in the first image's MPF segment, the APP2 segment whose
//! payload starts with `MPF` and a zero byte, which [`segments`] finds
//! (DC-007 5.2.1 and Figure 4, pp. 10 to 11). After the identifier comes
//! an MP header in the shape of a TIFF header: a byte-order mark, 42, and
//! the offset of the first IFD (DC-007 5.2.2.1 and Figure 5, 5.2.2.2,
//! p. 11). That IFD is the MP Index IFD (DC-007 5.2.3 and Table 3, pp. 12
//! to 18).
//!
//! Every offset in the segment, an image's among them, counts from the MP
//! header's first byte (DC-007 5.2, p. 10), not from the start of the
//! file. So the segment is read as the [`Block`] that [`segments`] gives:
//! its base is the MP header, and its bounds run from there to the end of
//! the segment. The index is an IFD inside its own segment, read by the
//! TIFF container as a MakerNote's IFD is: the MP header at the block's
//! base, the MP Index IFD inside the block and under
//! [`Limits::max_ifd_entries`], and every value it holds inside the block.
//! A segment's payload holds at most 65,533 bytes, so nothing read from
//! it is longer.
//!
//! Of the index's tags, only MPEntry ([`MP_ENTRY`]) is read. The MP tags
//! are the Multi-Picture Format's own, not TIFF or Exif tags, so they are
//! named here, beside their one reader, and not with the TIFF container's
//! tags. NumberOfImages (0xB001), which says how many images the file
//! holds, is not trusted and not read: the entries are those MPEntry's
//! value holds, so a count that disagrees with the value can neither send
//! the reader past it nor hide an entry. MPFVersion (0xB000) is not read
//! either.
//!
//! # An MP Entry
//!
//! An MP Entry describes one image the file holds. MPEntry's value is one
//! entry per image, 16 bytes each, every number in the MP header's byte
//! order (DC-007 Figure 7, p. 13):
//!
//! - the image's attribute, 4 bytes: flags, and the MP type, which says
//!   what the image is for, such as a large thumbnail (DC-007 Figure 8,
//!   p. 14; Table 4, p. 15). It is not read;
//! - the image's size, 4 bytes: the specification counts the bytes
//!   "between the SOI and EOI markers" without saying whether the markers
//!   count (DC-007 5.2.3.3.2, p. 16); on the camera files the notes
//!   describe, they do, and the size runs to the end of EOI;
//! - the image's offset, 4 bytes: where its SOI lies, counted from the MP
//!   header, and 0 for the first image (DC-007 5.2.3.3.3, p. 16);
//! - the entry numbers of two images that depend on this one, 2 bytes
//!   each. They are not read.
//!
//! # What is kept
//!
//! [`images`] gives, for each entry in the order of the value, the range
//! from the MP header's position plus the entry's offset, over the
//! entry's size. It leaves out:
//!
//! - an entry whose offset is 0: the file's first image, which is the file
//!   itself, a preview of its own (ADR014). The specification gives 0 to
//!   the first image only, and an offset of 0 points at the MP header,
//!   inside the first image, where no other image can start. So an entry
//!   is taken for the first image by its offset, wherever it stands in the
//!   value; a first entry with another offset is kept as any other;
//! - an entry whose start or end would pass the largest offset. On any
//!   file a reader can hold, this cannot happen, since the MP header lies
//!   inside the file and the offset and the size are 32-bit numbers, but
//!   the sums are checked all the same.
//!
//! It keeps an entry whatever its MP type: a gain map or a frame of a
//! panorama as well as a large thumbnail. Whether images other than large
//! thumbnails should be previews is an open question of the JPEG notes;
//! every camera file they describe that has an MPF index lists large
//! thumbnails only after its first image.
//!
//! It keeps as well, for the preview finder to judge with every other
//! candidate, so that each rule is applied in one place:
//!
//! - an entry whose range runs past the end of the file, or lies wholly
//!   past it: the finder drops every candidate that does not lie inside
//!   the file;
//! - an entry of size 0: the finder drops an empty candidate;
//! - an entry whose range holds no image: the finder checks that each
//!   candidate starts with SOI and holds a frame header within its own
//!   bytes.
//!
//! The ranges are not checked against the block, which bounds the index
//! and not the images: the images lie after the first image's EOI,
//! outside the segment. Nor is an image searched for: on the camera files
//! the notes describe, an image starts after zeros up to a multiple of 512
//! bytes (the Sony files and the Nikon D5300's) or after 12 bytes that are
//! not zeros (the Canon G7 X's), and only its offset says where.
//!
//! # Damage
//!
//! Only a failure of the reader is an error. Whatever else the segment
//! holds costs only the images it touches, as damage costs the MakerNote
//! readers only their own block ([`skip_damage`]): an MP header that
//! cannot be read, an MP Index IFD that cannot be read or has more entries
//! than the limit allows, or an MPEntry that is missing, of a type other
//! than UNDEFINED or of a length that is not a multiple of 16, so not laid
//! out as entries, gives no images; an entry that cannot give a range
//! gives none.
//!
//! [`segments`]: super::segments

use std::io::{Read, Seek};
use std::ops::Range;

use crate::container::tiff::types::UNDEFINED;
use crate::container::tiff::{Block, ifd_in, parse_header, skip_damage};
use crate::error::Result;
use crate::io::{ByteOrder, Source};
use crate::limits::Limits;

/// The MP tag that lists the images the file holds (MPEntry): an
/// UNDEFINED value of one MP Entry per image, 16 bytes each, the first
/// image's first (DC-007 Table 3, p. 12; Figure 7, p. 13).
const MP_ENTRY: u16 = 0xB002;

/// The length of one MP Entry: the attribute, the size and the offset, 4
/// bytes each, then two entry numbers of 2 bytes (DC-007 Figure 7, p. 13).
const ENTRY_LEN: usize = 16;

/// The byte ranges in the file of the images that the MPF index in `mpf`
/// lists after the file's first image, in the order of their entries; see
/// the [module documentation](self).
///
/// `mpf` is the block [`segments`](super::segments) gives: its base is the
/// MP header, and its bounds run from there to the end of the MPF segment.
/// A range may run past the end of the file, be empty, or hold no image:
/// the preview finder checks each one, as every candidate. An index that
/// does not hold together gives no ranges.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the segment holds is an error.
pub(crate) fn images<R: Read + Seek>(
    src: &mut Source<R>,
    mpf: &Block,
    limits: &Limits,
) -> Result<Vec<Range<u64>>> {
    let Some(header) = skip_damage(parse_header(src, mpf))? else {
        return Ok(Vec::new());
    };
    let Some(index) = ifd_in(src, mpf, header.ifd0, header.order, limits)? else {
        return Ok(Vec::new());
    };
    // The value readers take BYTE as well as UNDEFINED; MPEntry's rule
    // allows UNDEFINED only.
    if index.entry_of_type(MP_ENTRY, &[UNDEFINED]).is_none() {
        return Ok(Vec::new());
    }
    let Some(value) = skip_damage(index.bytes(src, MP_ENTRY))?.flatten() else {
        return Ok(Vec::new());
    };
    Ok(ranges(mpf, header.order, &value))
}

/// The ranges of the images that `value`, MPEntry's value in byte order
/// `order`, lists in `mpf`, the first image's left out; none when the
/// value is not a whole number of entries.
fn ranges(mpf: &Block, order: ByteOrder, value: &[u8]) -> Vec<Range<u64>> {
    let (entries, rest) = value.as_chunks::<ENTRY_LEN>();
    if !rest.is_empty() {
        return Vec::new();
    }
    entries
        .iter()
        .filter_map(|entry| range(mpf, order, entry))
        .collect()
}

/// The range of the image that `entry` describes, in byte order `order`,
/// from the MP header of `mpf`: `None` for the file's first image, whose
/// offset is 0, and when the range's start or end would pass the largest
/// offset.
fn range(mpf: &Block, order: ByteOrder, entry: &[u8; ENTRY_LEN]) -> Option<Range<u64>> {
    // The attribute and the two dependent images' entry numbers are not
    // read.
    let [_, _, _, _, s0, s1, s2, s3, o0, o1, o2, o3, _, _, _, _] = *entry;
    let size = order.u32([s0, s1, s2, s3]);
    let offset = order.u32([o0, o1, o2, o3]);
    if offset == 0 {
        return None;
    }
    let start = mpf.at(offset)?;
    let end = start.checked_add(u64::from(size))?;
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use crate::container::jpeg::segments;
    use crate::jpeg_builder::{Built, JpegBuilder};
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::Order;
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// The TIFF structure of the Exif segments the tests build: a
    /// little-endian header with IFD0 at 8, and an empty IFD0. Nothing here
    /// reads it.
    const TIFF: [u8; 14] = [b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /// Bytes that stand for the 12 before the large thumbnail of the Canon
    /// G7 X file, which are not zeros, and whose meaning is unknown. These
    /// hold `FF D8 FF`, as if an image started there, so that a reader that
    /// searched for one instead of following the offset would land on
    /// them.
    const G7_X_BYTES: [u8; 12] = [
        0x01, 0x02, 0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10, 0x0A, 0x0B, 0x0C, 0x0D,
    ];

    /// The MPF block that [`segments`] finds in `bytes`.
    fn mpf_block(bytes: &[u8]) -> Block {
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        segments(&mut src)
            .unwrap()
            .expect("a JPEG file")
            .mpf
            .expect("an MPF segment")
    }

    /// The images of the file `bytes`, read within `limits`, from the MPF
    /// block that [`segments`] finds in it.
    fn images_within(bytes: &[u8], limits: &Limits) -> Vec<Range<u64>> {
        let block = mpf_block(bytes);
        let mut src = Source::new(Cursor::new(bytes)).unwrap();
        images(&mut src, &block, limits).expect("only a reader's failure is an error")
    }

    /// The images of the file `bytes`, read within the default limits.
    fn images_of(bytes: &[u8]) -> Vec<Range<u64>> {
        images_within(bytes, &Limits::default())
    }

    /// The plainest layout of three entries, the main image's first: an
    /// Exif segment and the MPF index, then the main image and two large
    /// thumbnails right after its EOI, one after the other, of 640 by 424
    /// (class 1) and 1620 by 1080 (class 2), the sizes the D5300 file
    /// holds.
    fn back_to_back(order: Order) -> JpegBuilder {
        let mut builder = JpegBuilder::new();
        builder
            .exif(&TIFF)
            .mpf(order)
            .mpf_image(640, 424, &[], 1)
            .mpf_image(1620, 1080, &[], 1);
        builder
    }

    /// A file laid out as the D5300's: JFIF, Exif and the MPF index, then
    /// the main image and two large thumbnails, of 640 by 424 and 1620 by
    /// 1080, each after zeros up to a multiple of 512 bytes.
    fn d5300(order: Order) -> JpegBuilder {
        let mut builder = JpegBuilder::new();
        builder
            .jfif()
            .exif(&TIFF)
            .mpf(order)
            .mpf_image(640, 424, &[], 512)
            .mpf_image(1620, 1080, &[], 512);
        builder
    }

    /// A file laid out as the Sony files are: Exif and the MPF
    /// index, then the main image and one large thumbnail of 1616 by 1080
    /// after zeros up to a multiple of 512 bytes, then zeros.
    fn sony(order: Order) -> JpegBuilder {
        let mut builder = JpegBuilder::new();
        builder
            .exif(&TIFF)
            .mpf(order)
            .mpf_image(1616, 1080, &[], 512)
            .pad(1000);
        builder
    }

    /// A file laid out as the Canon G7 X's: Exif and the MPF index, then
    /// the main image and one large thumbnail of 1620 by 1080 at a multiple
    /// of 512 bytes, after 12 bytes that are not zeros, [`G7_X_BYTES`].
    fn g7_x(order: Order) -> JpegBuilder {
        let mut builder = JpegBuilder::new();
        builder
            .exif(&TIFF)
            .mpf(order)
            .mpf_image(1620, 1080, &G7_X_BYTES, 512);
        builder
    }

    /// Checks that the images of `built` are those the builder placed, each
    /// a whole stub from SOI to EOI.
    fn assert_images(built: &Built, what: &str) {
        let found = images_of(&built.bytes);
        assert_eq!(found, built.mpf_images(), "{what}");
        assert!(!found.is_empty(), "{what}");
        for range in found {
            let start = usize::try_from(range.start).unwrap();
            let end = usize::try_from(range.end).unwrap();
            assert_eq!(built.bytes[start..start + 2], [0xFF, 0xD8], "{what}");
            assert_eq!(built.bytes[end - 2..end], [0xFF, 0xD9], "{what}");
        }
    }

    /// Where the `n`th image of `built` lies, counted from the MP header,
    /// as its entry writes it.
    fn offset_of(built: &Built, n: usize) -> u32 {
        u32::try_from(built.mpf_image(n).start - built.mpf_header()).unwrap()
    }

    #[test]
    fn the_large_thumbnails_after_the_main_image_are_found() {
        for order in ORDERS {
            let built = back_to_back(order).build();
            assert_eq!(built.mpf_images().len(), 2);
            // Right after the main image's EOI, one after the other.
            assert_eq!(built.mpf_image(0).start, built.image_end());
            assert_eq!(built.mpf_image(1).start, built.mpf_image(0).end);
            assert_images(&built, &format!("back to back {order:?}"));
            // Each after zeros, at a multiple of 512 bytes.
            let built = d5300(order).build();
            for image in built.mpf_images() {
                assert_eq!(image.start % 512, 0);
            }
            assert_images(&built, &format!("D5300 {order:?}"));
        }
    }

    #[test]
    fn an_image_after_zeros_is_found_at_its_offset() {
        for order in ORDERS {
            let built = sony(order).build();
            let image = built.mpf_image(0);
            assert_eq!(image.start % 512, 0);
            let start = usize::try_from(image.start).unwrap();
            let image_end = usize::try_from(built.image_end()).unwrap();
            assert!(built.bytes[image_end..start].iter().all(|&b| b == 0));
            assert!(start > image_end, "zeros before the image");
            assert_images(&built, &format!("Sony {order:?}"));
        }
    }

    #[test]
    fn an_image_after_bytes_that_are_not_zeros_is_found_at_its_offset() {
        for order in ORDERS {
            let built = g7_x(order).build();
            let image = built.mpf_image(0);
            assert_eq!(image.start % 512, 0);
            let start = usize::try_from(image.start).unwrap();
            assert_eq!(built.bytes[start - 12..start], G7_X_BYTES);
            assert_images(&built, &format!("G7 X {order:?}"));
        }
    }

    #[test]
    fn a_file_without_images_after_the_first_lists_none() {
        // The index lists the main image alone.
        for order in ORDERS {
            let mut builder = JpegBuilder::new();
            builder.exif(&TIFF).mpf(order);
            assert_eq!(images_of(&builder.build().bytes), []);
        }
    }

    #[test]
    fn an_mp_header_that_is_not_one_gives_no_images() {
        let headers: [[u8; 8]; 6] = [
            // No byte-order mark.
            *b"XX\x2A\0\x08\0\0\0",
            *b"IM\x2A\0\x08\0\0\0",
            // Another number than 42, BigTIFF's 43 among them.
            *b"II\x2B\0\x08\0\0\0",
            *b"MM\0\x2B\0\0\0\x08",
            // 42 in the other byte order.
            *b"II\0\x2A\0\0\0\x08",
            [0; 8],
        ];
        for order in ORDERS {
            for header in headers {
                let mut builder = back_to_back(order);
                builder.mpf_header(header);
                assert_eq!(images_of(&builder.build().bytes), [], "{header:02X?}");
            }
        }
        // A segment too short to hold the MP header: the identifier and 7
        // bytes of a header.
        let mut builder = JpegBuilder::new();
        builder
            .app2(b"MPF\0II\x2A\0\x08\0\0")
            .mpf_image(640, 424, &[], 1);
        assert_eq!(images_of(&builder.build().bytes), []);
    }

    #[test]
    fn an_index_outside_the_block_gives_no_images() {
        /// An MP header in byte order `order` whose index lies at `at`.
        fn header_to(order: Order, at: u32) -> [u8; 8] {
            let mut header = order.mark().to_vec();
            header.extend_from_slice(&order.u16(42));
            header.extend_from_slice(&order.u32(at));
            header.try_into().unwrap()
        }
        for order in ORDERS {
            let built = back_to_back(order).build();
            let block = mpf_block(&built.bytes);
            let len = u32::try_from(block.bounds().end - block.base()).unwrap();
            // At the end of the segment, with its count inside but its
            // entries not, and far past it; the main image and the
            // thumbnails follow the segment, so a reader that left it would
            // find bytes there.
            for at in [len, len - 2, len - 10, 0x00FF_FFFF, u32::MAX] {
                let mut builder = back_to_back(order);
                builder.mpf_header(header_to(order, at));
                assert_eq!(images_of(&builder.build().bytes), [], "{order:?} {at}");
            }

            // A copy of the MP Index IFD in a comment right after the
            // segment, past its marker and length: an index that would list
            // the images, its MPEntry value inside the segment, were it not
            // outside the block.
            let header = usize::try_from(block.base()).unwrap();
            let ifd = built.bytes[header + 8..header + 50].to_vec();
            let mut builder = back_to_back(order);
            builder
                .segment(0xFE, &ifd)
                .mpf_header(header_to(order, len + 4));
            let built = builder.build();
            assert_eq!(images_of(&built.bytes), [], "{order:?}");
            // Read in a block that does not end with the segment, the copy
            // gives the images.
            let block = mpf_block(&built.bytes);
            let unbounded = Block::new(block.base(), block.base()..u64::MAX);
            let mut src = Source::new(Cursor::new(&built.bytes[..])).unwrap();
            let found = images(&mut src, &unbounded, &Limits::default()).unwrap();
            assert_eq!(found, built.mpf_images(), "{order:?}");
        }
    }

    #[test]
    fn an_index_with_more_entries_than_the_limit_gives_no_images() {
        // The MP Index IFD has 3 entries.
        for order in ORDERS {
            let built = back_to_back(order).build();
            let limits = Limits::default().max_ifd_entries(2);
            assert_eq!(images_within(&built.bytes, &limits), []);
            let limits = Limits::default().max_ifd_entries(3);
            assert_eq!(images_within(&built.bytes, &limits), built.mpf_images());
        }
    }

    #[test]
    fn a_missing_mp_entry_gives_no_images() {
        for order in ORDERS {
            let mut builder = back_to_back(order);
            builder.mpf_entry_tag(0xB00F);
            assert_eq!(images_of(&builder.build().bytes), []);
        }
    }

    #[test]
    fn an_mp_entry_of_another_type_gives_no_images() {
        // BYTE values are read as bytes too, and ASCII ones have 1-byte
        // values as well, so the value lies where it would.
        const BYTE: u16 = 1;
        const ASCII: u16 = 2;
        for order in ORDERS {
            for typ in [BYTE, ASCII] {
                let mut builder = back_to_back(order);
                builder.mpf_entry_type(typ);
                assert_eq!(images_of(&builder.build().bytes), [], "type {typ}");
            }
        }
    }

    #[test]
    fn an_mp_entry_not_a_multiple_of_16_bytes_gives_no_images() {
        // Three entries are 48 bytes. Lengths that hold whole entries and
        // some more, or fewer bytes than one.
        for order in ORDERS {
            for len in [1, 4, 15, 17, 40, 47, 49] {
                let mut builder = back_to_back(order);
                builder.mpf_entry_len(len);
                assert_eq!(images_of(&builder.build().bytes), [], "{len}");
            }
        }
    }

    #[test]
    fn the_entries_are_those_the_value_holds() {
        // The segment's length follows the value's, so each file places its
        // images where its own build says.
        for order in ORDERS {
            // Two entries: the main image's and the first thumbnail's.
            let mut builder = back_to_back(order);
            builder.mpf_entry_len(32);
            let built = builder.build();
            assert_eq!(images_of(&built.bytes), [built.mpf_image(0)]);
            // A fourth entry of zeros, which has offset 0.
            let mut builder = back_to_back(order);
            builder.mpf_entry_len(64);
            let built = builder.build();
            assert_eq!(images_of(&built.bytes), built.mpf_images());
            // No entry at all.
            let mut builder = back_to_back(order);
            builder.mpf_entry_len(0);
            assert_eq!(images_of(&builder.build().bytes), []);
        }
    }

    #[test]
    fn number_of_images_is_not_trusted() {
        for order in ORDERS {
            let built = back_to_back(order).build();
            for count in [0, 1, 2, 4, 1000, u32::MAX] {
                let mut builder = back_to_back(order);
                builder.mpf_number_of_images(count);
                assert_eq!(
                    images_of(&builder.build().bytes),
                    built.mpf_images(),
                    "{order:?} {count}"
                );
            }
        }
    }

    #[test]
    fn an_offset_of_0_is_the_first_image_wherever_it_stands() {
        for order in ORDERS {
            let built = back_to_back(order).build();
            // The last entry given offset 0 is taken for the first image.
            let mut builder = back_to_back(order);
            builder.mpf_entry_offset(2, 0);
            assert_eq!(images_of(&builder.build().bytes), [built.mpf_image(0)]);
            // The first entry given another offset is kept as any other:
            // here the second thumbnail's, with the main image's size.
            let mut builder = back_to_back(order);
            builder.mpf_entry_offset(0, offset_of(&built, 1));
            let start = built.mpf_image(1).start;
            let first = start..start + built.image_end();
            let mut expected = vec![first];
            expected.extend(built.mpf_images());
            assert_eq!(images_of(&builder.build().bytes), expected);
        }
    }

    #[test]
    fn an_image_past_the_end_of_the_file_is_kept() {
        for order in ORDERS {
            let built = back_to_back(order).build();
            let len = u64::try_from(built.bytes.len()).unwrap();
            let header = built.mpf_header();
            // Wholly past the end.
            let mut builder = back_to_back(order);
            builder.mpf_entry_offset(1, 0x7FFF_0000);
            let found = images_of(&builder.build().bytes);
            let size = built.mpf_image(0).end - built.mpf_image(0).start;
            let start = header + 0x7FFF_0000;
            assert_eq!(found, [start..start + size, built.mpf_image(1)]);
            assert!(found[0].start > len);
            // Starting inside the file and running past its end.
            let mut builder = back_to_back(order);
            builder.mpf_entry_size(2, u32::MAX);
            let found = images_of(&builder.build().bytes);
            let start = built.mpf_image(1).start;
            let end = start + u64::from(u32::MAX);
            assert_eq!(found, [built.mpf_image(0), start..end]);
            assert!(found[1].end > len);
        }
    }

    #[test]
    fn an_image_of_size_0_is_kept() {
        // The finder drops it as an empty candidate.
        for order in ORDERS {
            let built = back_to_back(order).build();
            let mut builder = back_to_back(order);
            builder.mpf_entry_size(1, 0);
            let start = built.mpf_image(0).start;
            assert_eq!(
                images_of(&builder.build().bytes),
                [start..start, built.mpf_image(1)]
            );
        }
    }

    #[test]
    fn the_ranges_follow_the_entries_order() {
        // The two thumbnails' entries swapped: the second image is listed
        // first.
        for order in ORDERS {
            let built = back_to_back(order).build();
            let first = built.mpf_image(0);
            let second = built.mpf_image(1);
            let mut builder = back_to_back(order);
            builder
                .mpf_entry_offset(1, offset_of(&built, 1))
                .mpf_entry_size(1, u32::try_from(second.end - second.start).unwrap())
                .mpf_entry_offset(2, offset_of(&built, 0))
                .mpf_entry_size(2, u32::try_from(first.end - first.start).unwrap());
            assert_eq!(images_of(&builder.build().bytes), [second, first]);
        }
    }

    /// An MP Entry with `size` and `offset` in byte order `order`, and an
    /// attribute and entry numbers that are not zeros, as no reader of the
    /// size and the offset may take them for either.
    fn entry(order: ByteOrder, size: u32, offset: u32) -> [u8; ENTRY_LEN] {
        let mut entry = [0xEE; ENTRY_LEN];
        let (size, offset) = match order {
            ByteOrder::Little => (size.to_le_bytes(), offset.to_le_bytes()),
            ByteOrder::Big => (size.to_be_bytes(), offset.to_be_bytes()),
        };
        entry[4..8].copy_from_slice(&size);
        entry[8..12].copy_from_slice(&offset);
        entry
    }

    #[test]
    fn an_entry_whose_start_or_end_would_overflow_is_dropped() {
        // A file cannot put the MP header high enough for the sums to
        // overflow, so a block whose base is there stands for it.
        let base = u64::MAX - 100;
        let block = Block::new(base, base..u64::MAX);
        for order in [ByteOrder::Little, ByteOrder::Big] {
            let value = [
                // The end at the largest offset: kept.
                entry(order, 50, 50),
                // The start past it.
                entry(order, 1, 101),
                entry(order, 0, u32::MAX),
                // The end past it.
                entry(order, 51, 50),
                entry(order, u32::MAX, 1),
                // At the largest offset, empty: kept.
                entry(order, 0, 100),
            ]
            .concat();
            assert_eq!(
                ranges(&block, order, &value),
                [base + 50..u64::MAX, u64::MAX..u64::MAX],
                "{order:?}"
            );
        }
    }

    #[test]
    fn the_size_and_the_offset_are_read_from_their_own_fields() {
        // Every field different, in both byte orders: the size 0x0102_0304
        // and the offset 0x0A0B_0C0D.
        let block = Block::new(1000, 1000..2000);
        for order in [ByteOrder::Little, ByteOrder::Big] {
            let value = entry(order, 0x0102_0304, 0x0A0B_0C0D);
            let start = 1000 + 0x0A0B_0C0D;
            let image = start..start + 0x0102_0304;
            assert_eq!(ranges(&block, order, &value), [image], "{order:?}");
        }
    }

    /// A file whose MP header lies at `header`, behind a comment that fills
    /// the bytes before it.
    fn mpf_at(header: u64, order: Order) -> Built {
        // SOI, the comment's marker and length, the MPF segment's marker,
        // length and identifier.
        let comment = usize::try_from(header - 2 - 4 - 4 - 4).unwrap();
        let mut builder = JpegBuilder::new();
        builder
            .segment(0xFE, &vec![0x20; comment])
            .mpf(order)
            .mpf_image(640, 424, &[], 1);
        let built = builder.build();
        assert_eq!(built.mpf_header(), header);
        built
    }

    #[test]
    fn a_failing_reader_is_an_error() {
        // The reader fails at the MP header, at the MP Index IFD, which
        // follows its 8 bytes, and at MPEntry's value, which follows the
        // IFD's 42.
        for order in ORDERS {
            for header in [FAR, FAR - 8, FAR - 50] {
                let built = mpf_at(header, order);
                let block = mpf_block(&built.bytes);
                let mut src = Source::new(FailsFar(Cursor::new(built.bytes.clone()))).unwrap();
                let err =
                    images(&mut src, &block, &Limits::default()).expect_err("the reader fails");
                assert_eq!(err.kind(), ErrorKind::Io, "{order:?} {header}: {err:?}");
                // Read without the failure, the same bytes give the image.
                assert_eq!(images_of(&built.bytes), built.mpf_images());
            }
            // With the whole index before the failure, the image is found,
            // although it lies past it.
            let built = mpf_at(FAR - 50 - 16 * 2, order);
            let block = mpf_block(&built.bytes);
            let mut src = Source::new(FailsFar(Cursor::new(built.bytes.clone()))).unwrap();
            let found = images(&mut src, &block, &Limits::default()).unwrap();
            assert_eq!(found, built.mpf_images());
            assert!(found[0].start >= FAR);
        }
    }

    #[test]
    fn every_truncated_file_gives_images_or_none() {
        for order in ORDERS {
            let built = d5300(order).build();
            let whole = mpf_block(&built.bytes);
            for cut in 0..=built.bytes.len() {
                let bytes = built.truncated(cut);
                let mut src = Source::new(Cursor::new(&bytes[..])).unwrap();
                // With the block of the whole file, which runs past the end
                // of the cut one, and with the block found in the cut one.
                let found = images(&mut src, &whole, &Limits::default())
                    .expect("only a reader's failure is an error");
                // The images are not checked against the file: once the
                // index is whole, every one is listed.
                let index_end = usize::try_from(whole.bounds().end).unwrap();
                if cut >= index_end {
                    assert_eq!(found, built.mpf_images(), "cut at {cut}");
                } else {
                    assert_eq!(found, [], "cut at {cut}");
                }
                if let Some(block) = segments(&mut src).unwrap().and_then(|found| found.mpf) {
                    images(&mut src, &block, &Limits::default())
                        .expect("only a reader's failure is an error");
                }
            }
        }
    }

    #[test]
    fn every_damaged_byte_of_the_segment_gives_images_or_none() {
        for order in ORDERS {
            let built = d5300(order).build();
            let block = mpf_block(&built.bytes);
            // From the segment's marker to its end.
            let start = usize::try_from(block.base()).unwrap() - 8;
            let end = usize::try_from(block.bounds().end).unwrap();
            for at in start..end {
                for value in [0x00, 0xFF, !built.bytes[at]] {
                    let mut bytes = built.bytes.clone();
                    bytes[at] = value;
                    let mut src = Source::new(Cursor::new(&bytes[..])).unwrap();
                    if let Some(block) = segments(&mut src).unwrap().and_then(|found| found.mpf) {
                        images(&mut src, &block, &Limits::default())
                            .expect("only a reader's failure is an error");
                    }
                }
            }
        }
    }
}
