// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the TIFF fixture builder from outside the crate, as the
//! integration tests use it.
//!
//! These are the builder's only tests. The builder,
//! `tests/support/tiff_builder.rs`, is compiled into the library's unit
//! tests and into every integration test that declares `mod support`, so a
//! test kept in it would run once in each of those test builds.

mod support;

use support::tiff_builder::{EXIF_IFD, LONG, Order, RawValue, SUB_IFDS, TiffBuilder, UNDEFINED};

/// One IFD, holding Make (ASCII, out of line) and Orientation (SHORT,
/// inline).
fn tiny(order: Order) -> Vec<u8> {
    let mut builder = TiffBuilder::new(order);
    let ifd = builder.ifd();
    builder.ascii(ifd, 0x010F, "Nikon").short(ifd, 0x0112, &[1]);
    builder.build().bytes
}

/// The little-endian 16-bit number at `at` of `bytes`.
fn u16_le(bytes: &[u8], at: u64) -> u16 {
    let at = usize::try_from(at).unwrap();
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

/// The little-endian 32-bit number at `at` of `bytes`.
fn u32_le(bytes: &[u8], at: u64) -> u32 {
    let at = usize::try_from(at).unwrap();
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// The tags of the IFD at `at` in a little-endian file, in file order.
fn tags_le(bytes: &[u8], at: u64) -> Vec<u16> {
    (0..u64::from(u16_le(bytes, at)))
        .map(|i| u16_le(bytes, at + 2 + 12 * i))
        .collect()
}

#[test]
fn a_tiny_little_endian_file() {
    #[rustfmt::skip]
    let expected = [
        b'I', b'I', 42, 0, 8, 0, 0, 0,                  // header
        2, 0,                                           // two entries
        0x0F, 0x01, 2, 0, 6, 0, 0, 0, 38, 0, 0, 0,      // Make, ASCII, 6, at 38
        0x12, 0x01, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0,       // Orientation, SHORT, 1, 1
        0, 0, 0, 0,                                     // no next IFD
        b'N', b'i', b'k', b'o', b'n', 0,                // Make's value
    ];
    assert_eq!(tiny(Order::Little), expected);
}

#[test]
fn a_tiny_big_endian_file() {
    #[rustfmt::skip]
    let expected = [
        b'M', b'M', 0, 42, 0, 0, 0, 8,                  // header
        0, 2,                                           // two entries
        0x01, 0x0F, 0, 2, 0, 0, 0, 6, 0, 0, 0, 38,      // Make, ASCII, 6, at 38
        0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 1, 0, 0,       // Orientation, SHORT, 1, 1
        0, 0, 0, 0,                                     // no next IFD
        b'N', b'i', b'k', b'o', b'n', 0,                // Make's value
    ];
    assert_eq!(tiny(Order::Big), expected);
}

#[test]
fn entries_are_sorted_unless_asked_not_to_be() {
    let mut builder = TiffBuilder::new(Order::Little);
    let sorted = builder.ifd();
    let unsorted = builder.ifd();
    builder.unsorted(unsorted);
    for ifd in [sorted, unsorted] {
        builder
            .short(ifd, 0x0112, &[1])
            .ascii(ifd, 0x010F, "a long make")
            .long(ifd, 0x0100, &[4000]);
    }
    let built = builder.build();
    assert_eq!(
        tags_le(&built.bytes, built.ifd_offset(sorted)),
        [0x0100, 0x010F, 0x0112]
    );
    assert_eq!(
        tags_le(&built.bytes, built.ifd_offset(unsorted)),
        [0x0112, 0x010F, 0x0100]
    );
}

#[test]
fn out_of_line_values_and_ifds_are_word_aligned() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let ifd1 = builder.ifd();
    // Five bytes, then eight: the rational starts one byte later than
    // the end of the text.
    builder
        .ascii(ifd0, 0x010F, "Sony")
        .rational(ifd0, 0x011A, &[(300, 1)]);
    builder.next(ifd0, Some(ifd1));
    let built = builder.build();
    let bytes = &built.bytes;

    // IFD0 at 8 holds two entries, so its values start at 8 + 30.
    assert_eq!(u32_le(bytes, 8 + 2 + 8), 38, "the text");
    assert_eq!(&bytes[38..43], b"Sony\0");
    assert_eq!(u32_le(bytes, 8 + 14 + 8), 44, "the rational");
    assert_eq!((u32_le(bytes, 44), u32_le(bytes, 48)), (300, 1));
    assert_eq!(built.ifd_offset(ifd1), 52);
    assert_eq!(u32_le(bytes, 8 + 26), 52, "the next-IFD pointer");
}

#[test]
fn blobs_are_where_the_offsets_say() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd = builder.ifd();
    let jpeg = builder.blob(&[0xFF, 0xD8, 0xFF, 0xD9, 0xAA]);
    let fixed = builder.blob_at(1000, b"far away");
    builder
        .long_blob(ifd, 0x0201, jpeg)
        .long_blob_len(ifd, 0x0202, jpeg)
        .long_blob(ifd, 0x0111, fixed);
    let built = builder.build();
    let bytes = &built.bytes;

    // One IFD of three entries at 8, then the first blob.
    let at = built.blob_offset(jpeg);
    assert_eq!(at, 8 + 2 + 36 + 4);
    let start = usize::try_from(at).unwrap();
    assert_eq!(&bytes[start..start + 5], [0xFF, 0xD8, 0xFF, 0xD9, 0xAA]);
    assert_eq!(built.blob_offset(fixed), 1000);
    assert_eq!(&bytes[1000..], b"far away");
    assert!(bytes[start + 5..1000].iter().all(|&b| b == 0));

    // Sorted: 0x0111, 0x0201, 0x0202.
    assert_eq!(u32_le(bytes, 8 + 2 + 8), 1000);
    assert_eq!(u64::from(u32_le(bytes, 8 + 14 + 8)), at);
    assert_eq!(u32_le(bytes, 8 + 26 + 8), 5);
}

#[test]
fn ifd_pointers_resolve() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let (a, b, exif) = (builder.ifd(), builder.ifd(), builder.ifd());
    builder.subifds(ifd0, &[a, b]).exif(ifd0, exif);
    builder.next(a, Some(a)).next_raw(b, 0xDEAD_BEEF);
    let built = builder.build();
    let bytes = &built.bytes;

    // SubIFDs: two offsets, out of line.
    assert_eq!(u16_le(bytes, 8 + 2), SUB_IFDS);
    let array = u64::from(u32_le(bytes, 8 + 2 + 8));
    assert_eq!(u64::from(u32_le(bytes, array)), built.ifd_offset(a));
    assert_eq!(u64::from(u32_le(bytes, array + 4)), built.ifd_offset(b));
    // The Exif IFD: one offset, inline.
    assert_eq!(u16_le(bytes, 8 + 14), EXIF_IFD);
    assert_eq!(u64::from(u32_le(bytes, 8 + 14 + 8)), built.ifd_offset(exif));
    // Empty IFDs: the next pointer follows the count.
    let a_at = built.ifd_offset(a);
    assert_eq!(u64::from(u32_le(bytes, a_at + 2)), a_at, "points to itself");
    assert_eq!(u32_le(bytes, built.ifd_offset(b) + 2), 0xDEAD_BEEF);
    assert_eq!(u32_le(bytes, built.ifd_offset(exif) + 2), 0);
}

#[test]
fn an_ifd_pointer_holds_the_childs_offset_under_any_tag() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let (a, child) = (builder.ifd(), builder.ifd());
    builder.ifd_pointer(ifd0, 0x0011, child);
    let built = builder.build();
    let bytes = &built.bytes;

    // One entry at 8 + 2: the tag, LONG, one value, inline.
    assert_eq!(u16_le(bytes, 8 + 2), 0x0011);
    assert_eq!(u16_le(bytes, 8 + 2 + 2), LONG);
    assert_eq!(u32_le(bytes, 8 + 2 + 4), 1);
    assert_eq!(u64::from(u32_le(bytes, 8 + 2 + 8)), built.ifd_offset(child));
    assert_ne!(built.ifd_offset(child), built.ifd_offset(a));
}

#[test]
fn an_undefined_blob_counts_the_blobs_bytes_and_points_at_them() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd = builder.ifd();
    let first = builder.blob(b"another blob first");
    let value = b"Nikon\0, and more";
    let blob = builder.blob(value);
    builder.undefined_blob(ifd, 0x927C, blob);
    let built = builder.build();
    let bytes = &built.bytes;

    // One entry at 8 + 2: the tag, UNDEFINED, the blob's length, and
    // the blob's offset in the value field.
    assert_eq!(u16_le(bytes, 8 + 2), 0x927C);
    assert_eq!(u16_le(bytes, 8 + 2 + 2), UNDEFINED);
    assert_eq!(
        usize::try_from(u32_le(bytes, 8 + 2 + 4)).unwrap(),
        value.len()
    );
    let at = built.blob_offset(blob);
    assert_ne!(at, built.blob_offset(first));
    assert_eq!(u64::from(u32_le(bytes, 8 + 2 + 8)), at);
    let at = usize::try_from(at).unwrap();
    assert_eq!(&bytes[at..at + value.len()], value);
}

#[test]
#[should_panic(expected = "inline value")]
fn an_undefined_blob_of_four_bytes_is_a_mistake() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd = builder.ifd();
    let blob = builder.blob(b"four");
    builder.undefined_blob(ifd, 0x927C, blob);
}

#[test]
fn a_prefix_lies_just_before_its_ifd_which_stays_even() {
    // IFD0 of one inline entry ends at 8 + 18, an even offset; nine
    // bytes before an IFD at 26 + 9, rounded up to 36, start at 27.
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let with = builder.ifd();
    builder.short(ifd0, 0x0112, &[1]).short(with, 0x0112, &[6]);
    builder.prefix(with, b"SONY PIC\0");
    let built = builder.build();
    let bytes = &built.bytes;

    assert_eq!(built.ifd_offset(with), 36);
    assert_eq!(&bytes[27..36], b"SONY PIC\0");
    assert_eq!(bytes[26], 0, "the byte before the prefix is padding");
    assert_eq!(u16_le(bytes, 36), 1, "the IFD's count follows the prefix");
    assert_eq!(tags_le(bytes, 36), [0x0112]);
    assert_eq!(built.ifd_block(with), 27..36 + 2 + 12 + 4);
}

#[test]
fn an_undefined_ifd_holds_the_prefix_the_ifd_and_its_values() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let block = builder.ifd();
    let after = builder.ifd();
    builder.undefined_ifd(ifd0, 0x927C, block);
    builder.short(after, 0x0001, &[1]);
    // Added after the entry that holds the block, and still in it: an
    // inline SHORT, then three SHORTs, which round the values up to an
    // even end, and four LONGs.
    builder.prefix(block, b"SONY DSC \0\0\0");
    builder
        .short(block, 0x0001, &[7])
        .short(block, 0x0002, &[1, 2, 3])
        .long(block, 0x0003, &[4, 5, 6, 7]);
    let built = builder.build();
    let bytes = &built.bytes;

    // IFD0 at 8 holds one entry and ends at 8 + 18; the block's IFD
    // after its 12-byte prefix, at 38, its 42-byte table and pointer,
    // then six bytes and sixteen, at 80 and at 86.
    let ifd = built.ifd_offset(block);
    assert_eq!(ifd, 38);
    assert_eq!(built.ifd_block(block), 26..86 + 16);
    assert_eq!(&bytes[26..38], b"SONY DSC \0\0\0");
    assert_eq!(u32_le(bytes, ifd + 2 + 12 + 8), 80, "the SHORTs");
    assert_eq!(u32_le(bytes, ifd + 2 + 24 + 8), 86, "the LONGs");
    // The next IFD starts past the block.
    assert_eq!(built.ifd_offset(after), 102);

    // One entry at 8 + 2: the tag, UNDEFINED, the block's length, and
    // its offset in the value field.
    assert_eq!(u16_le(bytes, 8 + 2), 0x927C);
    assert_eq!(u16_le(bytes, 8 + 2 + 2), UNDEFINED);
    assert_eq!(u32_le(bytes, 8 + 2 + 4), 102 - 26);
    assert_eq!(u32_le(bytes, 8 + 2 + 8), 26);
}

#[test]
fn an_undefined_ifd_without_a_prefix_or_values_is_the_ifd() {
    let mut builder = TiffBuilder::new(Order::Big);
    let ifd0 = builder.ifd();
    let block = builder.ifd();
    builder.undefined_ifd(ifd0, 0x927C, block);
    builder.short(block, 0x0001, &[7]);
    let built = builder.build();
    let at = built.ifd_offset(block);
    assert_eq!(built.ifd_block(block), at..at + 2 + 12 + 4);
    let bytes = &built.bytes;
    assert_eq!(bytes[8 + 2 + 4..8 + 2 + 8], [0, 0, 0, 18]);
    assert_eq!(
        bytes[8 + 2 + 8..8 + 2 + 12],
        u32::try_from(at).unwrap().to_be_bytes()
    );
}

#[test]
fn an_undefined_ifd_declaring_a_count_keeps_the_layout() {
    for order in [Order::Little, Order::Big] {
        // The same file twice: the entry that holds the block declares
        // its whole length in one and 5 bytes in the other. The block's
        // prefix puts its start before its IFD, and a blob follows it.
        let file = |count: Option<u32>| {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = builder.ifd();
            let block = builder.ifd();
            match count {
                None => builder.undefined_ifd(ifd0, 0x927C, block),
                Some(count) => builder.undefined_ifd_declaring(ifd0, 0x927C, block, count),
            };
            builder
                .prefix(block, b"SONY DSC \0\0\0")
                .short(block, 0x0001, &[1, 2, 3]);
            let blob = builder.blob(&[9; 6]);
            (builder.build(), block, blob)
        };
        let (whole, block, blob) = file(None);
        let (declared, _, _) = file(Some(5));
        let range = whole.ifd_block(block);
        assert_eq!(declared.ifd_block(block), range, "{order:?}");
        assert_eq!(declared.ifd_offset(block), whole.ifd_offset(block));
        assert_eq!(declared.blob_offset(blob), whole.blob_offset(blob));

        // One entry at 8 + 2: the tag, UNDEFINED, the count given, and
        // the block's offset in the value field. Every other byte is
        // as with `undefined_ifd`, which declares the block's length.
        let bytes = &declared.bytes;
        let count = 8 + 2 + 4..8 + 2 + 8;
        assert_eq!(bytes[8 + 2..8 + 2 + 2], order.u16(0x927C), "{order:?}");
        assert_eq!(bytes[8 + 2 + 2..8 + 2 + 4], order.u16(UNDEFINED));
        assert_eq!(bytes[count.clone()], order.u32(5), "{order:?}");
        let field = 8 + 2 + 8..8 + 2 + 12;
        assert_eq!(
            bytes[field],
            order.u32(u32::try_from(range.start).unwrap()),
            "{order:?}"
        );
        let len = u32::try_from(range.end - range.start).unwrap();
        assert_eq!(whole.bytes[count.clone()], order.u32(len), "{order:?}");
        let mut rest = bytes.clone();
        rest[count.clone()].copy_from_slice(&whole.bytes[count]);
        assert_eq!(rest, whole.bytes, "{order:?}");
    }
}

#[test]
fn a_block_through_a_blob_holds_the_ifds_and_blobs_before_it() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let block = builder.ifd();
    let child = builder.ifd();
    builder.undefined_ifd(ifd0, 0x927C, block);
    builder.ifd_pointer(block, 0x0011, child);
    builder.long(child, 0x0001, &[1, 2]);
    let first = builder.blob(&[1, 2, 3]);
    let last = builder.blob(&[4, 5, 6, 7, 8]);
    let after = builder.blob(&[9; 6]);
    builder.block_through(block, last);
    let built = builder.build();
    let bytes = &built.bytes;

    // IFD0 at 8, one entry, ends at 26; the block's IFD there, one
    // entry, to 44; the child at 44, its two LONGs at 62, to 70; the
    // blobs at 70, 74 (rounded up from 73) and 80.
    assert_eq!(built.ifd_offset(block), 26);
    assert_eq!(built.ifd_offset(child), 44);
    assert_eq!(built.blob_offset(first), 70);
    assert_eq!(built.blob_offset(last), 74);
    assert_eq!(built.blob_offset(after), 80);
    assert_eq!(built.ifd_block(block), 26..79);
    // The entry declares the whole of it.
    assert_eq!(u32_le(bytes, 8 + 2 + 4), 79 - 26);
    assert_eq!(u32_le(bytes, 8 + 2 + 8), 26);
}

#[test]
fn raw_entries_are_written_as_given() {
    let mut builder = TiffBuilder::new(Order::Big);
    let ifd = builder.ifd();
    builder
        .unsorted(ifd)
        .raw(ifd, 0x9999, 99, 7, RawValue::Inline([1, 2, 3, 4]))
        .raw(ifd, 0x0001, LONG, u32::MAX, RawValue::Offset(0x0A0B_0C0D));
    let bytes = builder.build().bytes;
    assert_eq!(
        &bytes[10..34],
        [
            0x99, 0x99, 0, 99, 0, 0, 0, 7, 1, 2, 3, 4, //
            0, 1, 0, 4, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A, 0x0B, 0x0C, 0x0D,
        ]
    );
}

#[test]
fn header_overrides_take_effect() {
    let mut builder = TiffBuilder::new(Order::Big);
    builder.ifd();
    builder
        .magic(43)
        .ifd0_offset(Some(0x0102_0304))
        .mark(*b"XY");
    assert_eq!(builder.build().bytes[..8], [b'X', b'Y', 0, 43, 1, 2, 3, 4]);

    builder.ifd0_offset(None);
    assert_eq!(builder.build().bytes[4..8], [0, 0, 0, 8]);
    assert_eq!(
        TiffBuilder::new(Order::Little).build().bytes,
        b"II*\0\0\0\0\0"
    );
}

#[test]
fn pad_to_and_truncated() {
    let mut builder = TiffBuilder::new(Order::Little);
    builder.pad_to(100);
    let built = builder.build();
    assert_eq!(built.bytes.len(), 100);
    assert!(built.bytes[8..].iter().all(|&b| b == 0));
    assert_eq!(built.truncated(3), b"II*");
}

#[test]
fn subifd_and_exif_offsets_resolve() {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd0 = builder.ifd();
    let (sub, exif) = (builder.ifd(), builder.ifd());
    builder.subifds(ifd0, &[sub]).exif(ifd0, exif);
    let built = builder.build();
    let bytes = &built.bytes;

    // IFD0 at 8 holds two entries, sorted: SubIFDs, then the Exif IFD, each
    // a single offset, inline.
    assert_eq!(bytes[8 + 2..8 + 4], SUB_IFDS.to_le_bytes());
    assert_eq!(u64::from(u32_le(bytes, 8 + 2 + 8)), built.ifd_offset(sub));
    assert_eq!(bytes[8 + 14..8 + 16], EXIF_IFD.to_le_bytes());
    assert_eq!(u64::from(u32_le(bytes, 8 + 14 + 8)), built.ifd_offset(exif));
    // The two empty IFDs follow IFD0, in the order they were created.
    assert_eq!(built.ifd_offset(sub), 8 + 2 + 24 + 4);
    assert_eq!(built.ifd_offset(exif), built.ifd_offset(sub) + 2 + 4);
}
