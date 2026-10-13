// SPDX-License-Identifier: MIT OR Apache-2.0

//! The NEF reader from outside the crate, on a file laid out like a Nikon
//! Z 9's: what it opens as, which previews it lists, and what reading one
//! returns.

mod support;

use std::io::Cursor;

use raw_files::{Format, Orientation, Preview, PreviewFormat, RawFile};
use support::nef::{
    FULL_SIZE_JPEG, IFD0_JPEG, MEDIUM_JPEG, RAW_STRIP_START, stub, z9_file, z9_file_with_blobs,
};
use support::tiff_builder::Order;

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

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

#[test]
fn the_z9_file_opens_as_nef_with_its_three_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = z9_file_with_blobs(order, 1);
        let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
        let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
        for (how, format, previews) in [
            ("bytes", from_bytes.format(), from_bytes.previews()),
            ("reader", from_reader.format(), from_reader.previews()),
        ] {
            assert_eq!(format, Format::Nef, "{order:?}, from {how}");
            assert_eq!(
                sizes(previews),
                [wide(IFD0_JPEG), wide(MEDIUM_JPEG), wide(FULL_SIZE_JPEG)],
                "{order:?}, from {how}"
            );
            let expected = [
                (blobs.ifd0_jpeg, IFD0_JPEG),
                (blobs.medium_jpeg, MEDIUM_JPEG),
                (blobs.full_size_jpeg, FULL_SIZE_JPEG),
            ];
            for (preview, (blob, size)) in previews.iter().zip(expected) {
                assert_eq!(
                    preview.format(),
                    PreviewFormat::Jpeg,
                    "{order:?}, from {how}"
                );
                assert_eq!(
                    preview.byte_range().start,
                    built.blob_offset(blob),
                    "{order:?}, from {how}"
                );
                assert_eq!(preview.byte_len(), stub_len(size), "{order:?}, from {how}");
            }
        }
    }
}

#[test]
fn the_largest_preview_is_the_full_size_jpeg_and_at_least_picks_the_smallest_that_fits() {
    let built = z9_file(Order::Little, 1);
    let file = RawFile::from_bytes(&built.bytes).unwrap();
    let size = |preview: Option<Preview>| preview.map(|p| (p.width(), p.height()));
    assert_eq!(size(file.largest_preview()), Some(wide(FULL_SIZE_JPEG)));
    assert_eq!(
        size(file.preview_at_least(1000, 600)),
        Some(wide(MEDIUM_JPEG))
    );
    assert_eq!(size(file.preview_at_least(100, 100)), Some(wide(IFD0_JPEG)));
    // Wider than any preview: none fits, so the largest is given.
    assert_eq!(
        size(file.preview_at_least(9000, 100)),
        Some(wide(FULL_SIZE_JPEG))
    );
}

#[test]
fn every_preview_takes_ifd0s_orientation() {
    for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
        let built = z9_file(Order::Big, value);
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        assert_eq!(file.previews().len(), 3, "Orientation {value}");
        for preview in file.previews() {
            assert_eq!(preview.orientation(), orientation, "Orientation {value}");
        }
    }
}

#[test]
fn reading_the_largest_preview_returns_exactly_the_stubs_bytes() {
    let built = z9_file(Order::Little, 1);
    let mut file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
    let largest = file.largest_preview().unwrap();
    let bytes = file.read_preview(&largest).unwrap();
    assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF]);
    assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9]);
    assert_eq!(u64::try_from(bytes.len()).unwrap(), largest.byte_len());
    assert_eq!(bytes, stub(FULL_SIZE_JPEG));
}

#[test]
fn the_raw_strip_the_grey_image_and_the_empty_subifds_are_not_previews() {
    for order in ORDERS {
        let (built, blobs) = z9_file_with_blobs(order, 1);
        // The raw strip is where the fixture says, so that the check below
        // looks at the right offset.
        let raw = usize::try_from(built.blob_offset(blobs.raw)).unwrap();
        assert_eq!(built.bytes[raw..raw + 4], RAW_STRIP_START, "{order:?}");

        let file = RawFile::from_bytes(&built.bytes).unwrap();
        let starts: Vec<u64> = file
            .previews()
            .iter()
            .map(|p| p.byte_range().start)
            .collect();
        assert!(
            !starts.contains(&built.blob_offset(blobs.raw)),
            "{order:?}: {starts:?}"
        );
        assert!(
            !starts.contains(&built.blob_offset(blobs.grey)),
            "{order:?}: {starts:?}"
        );
        // The empty SubIFDs declare a strip at offset 0.
        assert!(!starts.contains(&0), "{order:?}: {starts:?}");
        assert_eq!(starts.len(), 3, "{order:?}");
    }
}
