// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ARW reader from outside the crate, on files laid out like each
//! generation of Sony's RAW family (see [`support::arw`]): what each opens
//! as, which previews it lists, that none of them is the raw data, and
//! what reading one returns.

mod support;

use std::io::Cursor;

use raw_files::{Format, Orientation, Preview, PreviewFormat, RawFile};
use support::arw::{
    A100_JPEG, A100_RAW_AS_COUNT, A100_TAG_SIZE, ARW4_FULL_SIZE_JPEG, ArwBlobs, IFD0_JPEG,
    MEDIUM_FULL_SIZE_JPEG, SR2_JPEG, SRF_JPEG, THUMBNAIL, a100_file, arq_file, arw2_file,
    arw4_file, arw4_medium_file, sr2_file, srf_file, stub,
};
use support::tiff_builder::{BlobId, Built, Order};

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

/// The SubIFDs tag, whose value on the A100 is the offset of its raw data.
const SUB_IFDS: u16 = 0x014A;

/// The entries an IFD may have under the default limits.
const DEFAULT_MAX_IFD_ENTRIES: u16 = 1000;

/// The stored size of each preview, in order.
fn sizes(previews: &[Preview]) -> Vec<(u32, u32)> {
    previews.iter().map(|p| (p.width(), p.height())).collect()
}

/// `size` as a preview reports it.
fn wide((width, height): (u16, u16)) -> (u32, u32) {
    (u32::from(width), u32::from(height))
}

/// The length of the stub JPEG of `size`.
fn stub_len(size: (u16, u16)) -> u64 {
    u64::try_from(stub(size).len()).unwrap()
}

/// Checks that the Sony file `built` opens as ARW, from bytes and from a
/// reader, with exactly the JPEGs `expected`, smallest first: each the
/// stub of its size at its blob's offset. `label` names the file in a
/// failure.
fn assert_previews(built: &Built, expected: &[(BlobId, (u16, u16))], label: &str) {
    let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
    let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
    let expected_sizes: Vec<(u32, u32)> = expected.iter().map(|&(_, size)| wide(size)).collect();
    for (how, format, previews) in [
        ("bytes", from_bytes.format(), from_bytes.previews()),
        ("reader", from_reader.format(), from_reader.previews()),
    ] {
        assert_eq!(format, Format::Arw, "{label}, from {how}");
        assert_eq!(sizes(previews), expected_sizes, "{label}, from {how}");
        for (preview, &(blob, size)) in previews.iter().zip(expected) {
            assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label}, from {how}");
            assert_eq!(
                preview.byte_range().start,
                built.blob_offset(blob),
                "{label}, from {how}"
            );
            assert_eq!(preview.byte_len(), stub_len(size), "{label}, from {how}");
        }
    }
}

#[test]
fn the_arw4_file_opens_as_arw_with_its_three_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = arw4_file(order, 1);
        assert_previews(
            &built,
            &[
                (blobs.ifd1_jpeg, THUMBNAIL),
                (blobs.ifd0_jpeg.unwrap(), IFD0_JPEG),
                (blobs.ifd2_jpeg.unwrap(), ARW4_FULL_SIZE_JPEG),
            ],
            &format!("ARW 4, {order:?}"),
        );
    }
}

#[test]
fn the_arw4_file_with_a_medium_raw_opens_with_its_three_jpegs_and_not_the_ycbcr_raw() {
    for order in ORDERS {
        let (built, blobs) = arw4_medium_file(order);
        assert_previews(
            &built,
            &[
                (blobs.ifd1_jpeg, THUMBNAIL),
                (blobs.ifd0_jpeg.unwrap(), IFD0_JPEG),
                (blobs.ifd2_jpeg.unwrap(), MEDIUM_FULL_SIZE_JPEG),
            ],
            &format!("ARW 4 medium, {order:?}"),
        );
    }
}

#[test]
fn the_arw2_file_opens_as_arw_with_its_two_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = arw2_file(order, 1);
        assert_previews(
            &built,
            &[
                (blobs.ifd1_jpeg, THUMBNAIL),
                (blobs.ifd0_jpeg.unwrap(), IFD0_JPEG),
            ],
            &format!("ARW 2, {order:?}"),
        );
    }
}

#[test]
fn the_arq_file_opens_as_arw_with_its_two_jpegs_and_not_the_linear_raw() {
    for order in ORDERS {
        let (built, blobs) = arq_file(order);
        assert_previews(
            &built,
            &[
                (blobs.ifd1_jpeg, THUMBNAIL),
                (blobs.ifd0_jpeg.unwrap(), IFD0_JPEG),
            ],
            &format!("ARQ, {order:?}"),
        );
    }
}

#[test]
fn the_sr2_file_opens_as_arw_with_its_two_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = sr2_file(order);
        assert_previews(
            &built,
            &[
                (blobs.ifd1_jpeg, THUMBNAIL),
                (blobs.ifd0_jpeg.unwrap(), SR2_JPEG),
            ],
            &format!("SR2, {order:?}"),
        );
    }
}

#[test]
fn the_srfs_ifd0_yields_nothing_and_ifd1s_jpeg_is_its_only_preview() {
    for order in ORDERS {
        let (built, blobs) = srf_file(order);
        assert_eq!(blobs.ifd0_jpeg, None);
        assert_previews(
            &built,
            &[(blobs.ifd1_jpeg, SRF_JPEG)],
            &format!("SRF, {order:?}"),
        );
    }
}

#[test]
fn the_a100s_ifd0_preview_is_sized_from_its_frame_header_not_its_tags() {
    for order in ORDERS {
        let (built, blobs) = a100_file(order);
        assert_previews(
            &built,
            &[
                (blobs.ifd1_jpeg, THUMBNAIL),
                (blobs.ifd0_jpeg.unwrap(), A100_JPEG),
            ],
            &format!("A100, {order:?}"),
        );
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        assert!(
            !sizes(file.previews()).contains(&wide(A100_TAG_SIZE)),
            "{order:?}"
        );
    }
}

/// Every layout of the family, in byte order `order`, with its name.
fn every_layout(order: Order) -> Vec<(&'static str, (Built, ArwBlobs))> {
    vec![
        ("ARW 4", arw4_file(order, 1)),
        ("ARW 4 medium", arw4_medium_file(order)),
        ("ARW 2", arw2_file(order, 1)),
        ("ARQ", arq_file(order)),
        ("SR2", sr2_file(order)),
        ("SRF", srf_file(order)),
        ("A100", a100_file(order)),
    ]
}

#[test]
fn no_preview_starts_at_the_raw_data_of_any_layout() {
    for order in ORDERS {
        for (name, (built, blobs)) in every_layout(order) {
            let file = RawFile::from_bytes(&built.bytes).unwrap();
            let raw: Vec<u64> = blobs
                .raw
                .iter()
                .map(|&blob| built.blob_offset(blob))
                .collect();
            for preview in file.previews() {
                assert!(
                    !raw.contains(&preview.byte_range().start),
                    "{name}, {order:?}: a preview at {} is raw data",
                    preview.byte_range().start
                );
            }
        }
    }
}

#[test]
fn orientation_8_gives_rotate270_on_every_preview_of_the_arw2_and_arw4_files() {
    for order in ORDERS {
        for (name, (built, _)) in [
            ("ARW 2", arw2_file(order, 8)),
            ("ARW 4", arw4_file(order, 8)),
        ] {
            let file = RawFile::from_bytes(&built.bytes).unwrap();
            assert!(!file.previews().is_empty(), "{name}, {order:?}");
            for preview in file.previews() {
                assert_eq!(
                    preview.orientation(),
                    Orientation::Rotate270,
                    "{name}, {order:?}"
                );
            }
        }
    }
}

#[test]
fn the_largest_preview_is_ifd2s_on_the_arw4_file_and_ifd0s_on_the_arw2_file() {
    for order in ORDERS {
        let (built, blobs) = arw4_file(order, 1);
        let largest = RawFile::from_bytes(&built.bytes)
            .unwrap()
            .largest_preview()
            .unwrap();
        assert_eq!(
            largest.byte_range().start,
            built.blob_offset(blobs.ifd2_jpeg.unwrap()),
            "{order:?}"
        );

        let (built, blobs) = arw2_file(order, 1);
        let largest = RawFile::from_bytes(&built.bytes)
            .unwrap()
            .largest_preview()
            .unwrap();
        assert_eq!(
            largest.byte_range().start,
            built.blob_offset(blobs.ifd0_jpeg.unwrap()),
            "{order:?}"
        );
    }
}

#[test]
fn a_preview_at_least_2000_by_1500_on_the_arw4_file_is_ifd2s() {
    let (built, blobs) = arw4_file(Order::Little, 1);
    let file = RawFile::from_bytes(&built.bytes).unwrap();
    let preview = file.preview_at_least(2000, 1500).unwrap();
    assert_eq!(
        (preview.width(), preview.height()),
        wide(ARW4_FULL_SIZE_JPEG)
    );
    assert_eq!(
        preview.byte_range().start,
        built.blob_offset(blobs.ifd2_jpeg.unwrap())
    );
}

#[test]
fn reading_the_largest_preview_returns_exactly_the_stubs_bytes() {
    for order in ORDERS {
        let (built, _) = arw4_file(order, 1);
        let mut file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
        let largest = file.largest_preview().unwrap();
        let bytes = file.read_preview(&largest).unwrap();
        assert_eq!(bytes, stub(ARW4_FULL_SIZE_JPEG), "{order:?}");
    }
}

/// The 16-bit number at `at` of `bytes`, in byte order `order`.
fn u16_at(bytes: &[u8], at: usize, order: Order) -> u16 {
    let pair = [bytes[at], bytes[at + 1]];
    match order {
        Order::Little => u16::from_le_bytes(pair),
        Order::Big => u16::from_be_bytes(pair),
    }
}

/// The 32-bit number at `at` of `bytes`, in byte order `order`.
fn u32_at(bytes: &[u8], at: usize, order: Order) -> u32 {
    let quad = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
    match order {
        Order::Little => u32::from_le_bytes(quad),
        Order::Big => u32::from_be_bytes(quad),
    }
}

/// Whether `typ` is one of the field types TIFF defines, 1 to 13.
fn is_tiff_type(typ: u16) -> bool {
    (1..=13).contains(&typ)
}

#[test]
fn the_a100s_subifd_pointer_leads_to_bytes_that_are_not_an_ifd() {
    // The fixture is what it claims: read as the walk reads it, the target
    // of IFD0's SubIFDs tag is an IFD of more entries than the default cap
    // allows, whose table lies inside the file and whose every entry has a
    // field type no TIFF defines.
    for order in ORDERS {
        let (built, _) = a100_file(order);
        let bytes = &built.bytes;
        let ifd0 = usize::try_from(u32_at(bytes, 4, order)).unwrap();
        let entries = usize::from(u16_at(bytes, ifd0, order));
        let pointer = (0..entries)
            .map(|i| ifd0 + 2 + 12 * i)
            .find(|&entry| u16_at(bytes, entry, order) == SUB_IFDS)
            .map(|entry| u32_at(bytes, entry + 8, order))
            .expect("IFD0 has a SubIFDs tag");
        let at = usize::try_from(pointer).unwrap();

        let count = u16_at(bytes, at, order);
        assert_eq!(count, A100_RAW_AS_COUNT, "{order:?}");
        assert!(count > DEFAULT_MAX_IFD_ENTRIES, "{order:?}");
        let table_end = at + 2 + 12 * usize::from(count) + 4;
        assert!(table_end <= bytes.len(), "{order:?}");
        for i in 0..usize::from(count) {
            let typ = u16_at(bytes, at + 2 + 12 * i + 2, order);
            assert!(!is_tiff_type(typ), "{order:?}: entry {i} has type {typ}");
        }

        // And the walk leaves it out: the file opens, with its two JPEGs.
        let file = RawFile::from_bytes(bytes).unwrap();
        assert_eq!(file.format(), Format::Arw, "{order:?}");
        assert_eq!(file.previews().len(), 2, "{order:?}");
    }
}
