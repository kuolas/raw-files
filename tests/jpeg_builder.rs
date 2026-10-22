// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the JPEG file builder from outside the crate, as the integration
//! tests use it.
//!
//! The unit tests compile the builder into the library's test build; this
//! test compiles it through `tests/support/`, the copy the integration tests
//! share, so that copy is built and linted on every test run even before
//! another integration test uses it.

mod support;

use support::jpeg_builder::{
    Built, EXIF_ID, JFIF, JpegBuilder, LONG, MP_ENTRY, MP_ENTRY_AT, MP_INDEX_AT, MPF_ID,
    MPF_VERSION, NUMBER_OF_IMAGES, UNDEFINED, XMP_ID,
};
use support::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};
use support::tiff_builder::Order;

/// The bytes of `bytes` in `range`.
fn at(bytes: &[u8], range: std::ops::Range<u64>) -> &[u8] {
    let start = usize::try_from(range.start).unwrap();
    let end = usize::try_from(range.end).unwrap();
    &bytes[start..end]
}

#[test]
fn an_empty_builder_writes_the_stub() {
    let built = JpegBuilder::new().build();
    assert_eq!(built.bytes, jpeg_stub(640, 480, 0xC0));
    assert_eq!(
        built.frame_header(),
        u64::try_from(FRAME_HEADER_AT).unwrap()
    );
}

#[test]
fn segments_follow_soi_in_the_order_they_were_added() {
    let tiff = [b'M', b'M', 0, 42, 0, 0, 0, 8];
    let mut builder = JpegBuilder::new();
    builder
        .jfif()
        .exif(&tiff)
        .xmp("<x/>")
        .app2(b"MPF\0abc")
        .segment(0xFE, b"hi")
        .fill(2)
        .marker(0xD0)
        .raw(&[0xFF, 0x01])
        .frame(0x0102, 0x0304, 0xC2)
        .after_eoi(b"next")
        .after_eoi(b"last")
        .pad(3);
    let built = builder.build();
    let bytes = &built.bytes;

    let mut expected = vec![0xFF, 0xD8];
    expected.extend(segment(0xE0, &JFIF));
    let exif_at = expected.len();
    let mut exif = EXIF_ID.to_vec();
    exif.extend_from_slice(&tiff);
    expected.extend(segment(0xE1, &exif));
    let mut xmp = XMP_ID.to_vec();
    xmp.extend_from_slice(b"<x/>");
    expected.extend(segment(0xE1, &xmp));
    let app2_at = expected.len();
    expected.extend(segment(0xE2, b"MPF\0abc"));
    expected.extend(segment(0xFE, b"hi"));
    expected.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xD0, 0xFF, 0x01]);
    let main_at = expected.len();
    expected.extend_from_slice(&jpeg_stub(0x0102, 0x0304, 0xC2)[2..]);
    let next_at = expected.len();
    expected.extend_from_slice(b"nextlast\0\0\0");
    assert_eq!(*bytes, expected);

    // The JFIF segment's length counts itself and its 14 bytes.
    assert_eq!(bytes[2..6], [0xFF, 0xE0, 0, 16]);
    assert_eq!(bytes[6..11], *b"JFIF\0");
    // The Exif segment: marker, length 2 + 6 + 8, the identifier, then the
    // TIFF structure where the builder says.
    let exif_at = u64::try_from(exif_at).unwrap();
    assert_eq!(at(bytes, exif_at..exif_at + 4), [0xFF, 0xE1, 0, 16]);
    assert_eq!(at(bytes, exif_at + 4..exif_at + 10), *b"Exif\0\0");
    assert_eq!(built.exif_tiff(0), exif_at + 10..exif_at + 18);
    assert_eq!(at(bytes, built.exif_tiff(0)), tiff);
    // The APP2 segment's payload, as given.
    let app2_at = u64::try_from(app2_at).unwrap();
    assert_eq!(built.app2_payload(0), app2_at + 4..app2_at + 11);
    assert_eq!(at(bytes, built.app2_payload(0)), *b"MPF\0abc");
    // The main image's frame header, big-endian height then width.
    let frame = built.frame_header();
    assert_eq!(frame, u64::try_from(main_at + FRAME_HEADER_AT - 2).unwrap());
    assert_eq!(
        at(bytes, frame..frame + 9),
        [0xFF, 0xC2, 0, 11, 8, 0x03, 0x04, 0x01, 0x02]
    );
    // The blocks after EOI, then the padding.
    let next_at = u64::try_from(next_at).unwrap();
    assert_eq!(at(bytes, next_at - 2..next_at), [0xFF, 0xD9]);
    assert_eq!(built.after_eoi(0), next_at);
    assert_eq!(built.after_eoi(1), next_at + 4);
    assert_eq!(built.truncated(3), [0xFF, 0xD8, 0xFF]);
}

#[test]
fn each_exif_and_app2_segment_is_reported() {
    let mut builder = JpegBuilder::new();
    builder.exif(b"II").app2(b"a").exif(b"MM").app2(b"bc");
    let built = builder.build();
    // SOI, then segments of 4 + 8, 4 + 1, 4 + 8 and 4 + 2 bytes.
    assert_eq!(built.exif_tiff(0), 12..14);
    assert_eq!(built.app2_payload(0), 18..19);
    assert_eq!(built.exif_tiff(1), 29..31);
    assert_eq!(built.app2_payload(1), 35..37);
}

/// The 16-bit number at `at` in `bytes`, in byte order `order`.
fn u16_at(bytes: &[u8], at: u64, order: Order) -> u16 {
    let raw: [u8; 2] = self::at(bytes, at..at + 2).try_into().unwrap();
    match order {
        Order::Little => u16::from_le_bytes(raw),
        Order::Big => u16::from_be_bytes(raw),
    }
}

/// The 32-bit number at `at` in `bytes`, in byte order `order`.
fn u32_at(bytes: &[u8], at: u64, order: Order) -> u32 {
    let raw: [u8; 4] = self::at(bytes, at..at + 4).try_into().unwrap();
    match order {
        Order::Little => u32::from_le_bytes(raw),
        Order::Big => u32::from_be_bytes(raw),
    }
}

/// The `n`th entry of the MP Index IFD of `built`: its tag, type, count
/// and value field, the field's numbers read in byte order `order`.
fn index_entry(built: &Built, n: u64, order: Order) -> (u16, u16, u32, u32) {
    let at = built.mpf_header() + u64::from(MP_INDEX_AT) + 2 + 12 * n;
    let bytes = &built.bytes;
    (
        u16_at(bytes, at, order),
        u16_at(bytes, at + 2, order),
        u32_at(bytes, at + 4, order),
        u32_at(bytes, at + 8, order),
    )
}

/// The `n`th MP Entry of `built`: its attribute, size and offset, and its
/// two dependent images' entry numbers, read in byte order `order`.
fn mp_entry(built: &Built, n: u64, order: Order) -> (u32, u32, u32, u16, u16) {
    let at = built.mpf_header() + u64::from(MP_ENTRY_AT) + 16 * n;
    let bytes = &built.bytes;
    (
        u32_at(bytes, at, order),
        u32_at(bytes, at + 4, order),
        u32_at(bytes, at + 8, order),
        u16_at(bytes, at + 12, order),
        u16_at(bytes, at + 14, order),
    )
}

/// The three MP Entries of a file built by [`with_mpf`], from where its
/// images landed, every number in byte order `order`.
fn entries(built: &Built, order: Order) -> Vec<u8> {
    let header = built.mpf_header();
    let mut entries = Vec::new();
    let mut entry = |attribute: u32, range: std::ops::Range<u64>, offset: u64| {
        let size = u32::try_from(range.end - range.start).unwrap();
        let offset = u32::try_from(offset).unwrap();
        entries.extend_from_slice(&order.u32(attribute));
        entries.extend_from_slice(&order.u32(size));
        entries.extend_from_slice(&order.u32(offset));
        entries.extend_from_slice(&[0; 4]);
    };
    entry(0xA003_0000, 0..built.image_end(), 0);
    for (n, attribute) in [0x4001_0001, 0x4001_0002].into_iter().enumerate() {
        let image = built.mpf_image(n);
        entry(attribute, image.clone(), image.start - header);
    }
    entries
}

/// A file with an MPF index in byte order `order`, a block after the main
/// image, then a large thumbnail of class 1 right after it, then one of
/// class 2 after 12 bytes that are not zeros, at a multiple of 512 bytes,
/// then padding.
fn with_mpf(order: Order) -> JpegBuilder {
    let mut builder = JpegBuilder::new();
    builder
        .exif(&[b'I', b'I', 42, 0, 8, 0, 0, 0])
        .mpf(order)
        .segment(0xFE, b"hi")
        .after_eoi(b"next")
        .mpf_image(640, 424, &[], 1)
        .mpf_image(1620, 1080, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 512)
        .pad(3);
    builder
}

#[test]
fn the_mpf_index_lists_the_main_image_and_the_images_after_it() {
    for order in [Order::Little, Order::Big] {
        let built = with_mpf(order).build();
        let bytes = &built.bytes;
        let header = built.mpf_header();
        // The segment: its marker, its length, which counts itself, the
        // identifier, the header, the IFD and three entries, then the
        // identifier, right after the Exif segment.
        assert_eq!(header - 8, built.exif_tiff(0).end);
        assert_eq!(
            at(bytes, header - 8..header - 4),
            [0xFF, 0xE2, 0, 2 + 4 + 50 + 48]
        );
        assert_eq!(at(bytes, header - 4..header), MPF_ID);
        let segment_end = header + 50 + 48;
        assert_eq!(at(bytes, segment_end..segment_end + 2), [0xFF, 0xFE]);
        // The MP header, with the IFD right after it.
        assert_eq!(at(bytes, header..header + 2), order.mark());
        assert_eq!(u16_at(bytes, header + 2, order), 42);
        assert_eq!(u32_at(bytes, header + 4, order), MP_INDEX_AT);
        // The MP Index IFD: three entries and no next IFD.
        let ifd = header + u64::from(MP_INDEX_AT);
        assert_eq!(u16_at(bytes, ifd, order), 3);
        // The version's 4 bytes, `0100`, as the field holds them.
        let version = u32_at(b"0100", 0, order);
        assert_eq!(
            index_entry(&built, 0, order),
            (MPF_VERSION, UNDEFINED, 4, version)
        );
        assert_eq!(
            index_entry(&built, 1, order),
            (NUMBER_OF_IMAGES, LONG, 1, 3)
        );
        assert_eq!(
            index_entry(&built, 2, order),
            (MP_ENTRY, UNDEFINED, 48, MP_ENTRY_AT)
        );
        assert_eq!(u32_at(bytes, ifd + 2 + 36, order), 0);
        assert_eq!(header + u64::from(MP_ENTRY_AT), ifd + 2 + 36 + 4);

        // The main image's entry: offset 0, its size up to and including
        // its EOI.
        let image_end = built.image_end();
        assert_eq!(at(bytes, image_end - 2..image_end), [0xFF, 0xD9]);
        assert_eq!(built.after_eoi(0), image_end);
        let size = u32::try_from(image_end).unwrap();
        assert_eq!(mp_entry(&built, 0, order), (0xA003_0000, size, 0, 0, 0));
        // Each other image's: its offset from the MP header lands on its
        // SOI, and its size ends after its EOI.
        let stubs = [jpeg_stub(640, 424, 0xC0), jpeg_stub(1620, 1080, 0xC0)];
        for (n, (attribute, stub)) in [0x4001_0001, 0x4001_0002].iter().zip(&stubs).enumerate() {
            let image = built.mpf_image(n);
            let (got_attribute, size, offset, first, second) =
                mp_entry(&built, u64::try_from(n).unwrap() + 1, order);
            assert_eq!((got_attribute, first, second), (*attribute, 0, 0), "{n}");
            let start = header + u64::from(offset);
            assert_eq!(start, image.start, "{n}");
            assert_eq!(at(bytes, start..start + 2), [0xFF, 0xD8], "{n}");
            assert_eq!(start + u64::from(size), image.end, "{n}");
            assert_eq!(at(bytes, image.clone()), stub.as_slice(), "{n}");
        }
        // The first right after the block after EOI, the second after its
        // 12 bytes, at a multiple of 512, with zeros before them.
        assert_eq!(built.mpf_image(0).start, image_end + 4);
        let second = built.mpf_image(1).start;
        assert_eq!(second % 512, 0);
        assert_eq!(
            at(bytes, second - 12..second),
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        );
        let first_end = built.mpf_image(0).end;
        assert!(at(bytes, first_end..second - 12).iter().all(|&b| b == 0));
        assert_eq!(built.mpf_images(), [built.mpf_image(0), built.mpf_image(1)]);
        // Then the padding.
        assert_eq!(
            bytes.len(),
            usize::try_from(built.mpf_image(1).end).unwrap() + 3
        );
    }
}

#[test]
fn the_mpf_index_can_be_damaged() {
    for order in [Order::Little, Order::Big] {
        let whole = with_mpf(order).build();

        // The MP header, NumberOfImages, and an entry's offset and size.
        let mut builder = with_mpf(order);
        builder
            .mpf_header(*b"XYZ\0\0\0\0\x01")
            .mpf_number_of_images(7)
            .mpf_entry_offset(2, 0x1234_5678)
            .mpf_entry_size(1, 9);
        let built = builder.build();
        let header = built.mpf_header();
        assert_eq!(header, whole.mpf_header());
        assert_eq!(at(&built.bytes, header..header + 8), *b"XYZ\0\0\0\0\x01");
        assert_eq!(index_entry(&built, 1, order).3, 7);
        assert_eq!(mp_entry(&built, 2, order).2, 0x1234_5678);
        assert_eq!(mp_entry(&built, 1, order).1, 9);
        // The other fields as in the whole file.
        assert_eq!(mp_entry(&built, 1, order).2, mp_entry(&whole, 1, order).2);
        assert_eq!(mp_entry(&built, 2, order).1, mp_entry(&whole, 2, order).1);
        assert_eq!(mp_entry(&built, 0, order), mp_entry(&whole, 0, order));

        // MPEntry's tag and type.
        let mut builder = with_mpf(order);
        builder.mpf_entry_tag(0xB00F).mpf_entry_type(1);
        let built = builder.build();
        assert_eq!(index_entry(&built, 2, order), (0xB00F, 1, 48, MP_ENTRY_AT));

        // A value that is not a multiple of 16 bytes: cut, or followed by
        // zeros; the segment is as long as it needs to be, and the images
        // move with it.
        for len in [17, 40, 49] {
            let mut builder = with_mpf(order);
            builder.mpf_entry_len(len);
            let built = builder.build();
            let header = built.mpf_header();
            let count = u32::try_from(len).unwrap();
            assert_eq!(
                index_entry(&built, 2, order),
                (MP_ENTRY, UNDEFINED, count, MP_ENTRY_AT)
            );
            let value = header + u64::from(MP_ENTRY_AT);
            let end = value + u64::try_from(len).unwrap();
            let mut expected = entries(&built, order);
            expected.resize(len, 0);
            assert_eq!(at(&built.bytes, value..end), expected, "{len}");
            assert_eq!(at(&built.bytes, end..end + 2), [0xFF, 0xFE], "{len}");
        }
        // One of 4 bytes or fewer lies in the entry, as TIFF has it.
        let mut builder = with_mpf(order);
        builder.mpf_entry_len(3);
        let built = builder.build();
        let header = built.mpf_header();
        // The third entry's value field, after the count, two entries, and
        // the third's tag, type and count.
        let field = header + u64::from(MP_INDEX_AT) + 2 + 24 + 8;
        assert_eq!(index_entry(&built, 2, order).2, 3);
        let mut expected = entries(&built, order);
        expected.truncate(3);
        expected.push(0);
        assert_eq!(at(&built.bytes, field..field + 4), expected);
        // The segment ends with the IFD.
        let end = header + u64::from(MP_ENTRY_AT);
        assert_eq!(at(&built.bytes, end..end + 2), [0xFF, 0xFE]);
    }
}
