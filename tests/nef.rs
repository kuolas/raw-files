// SPDX-License-Identifier: MIT OR Apache-2.0

//! The NEF reader from outside the crate, on a file laid out like a Nikon
//! Z 9's: what it opens as, which previews it lists, what reading one
//! returns, and what metadata it gives. Then the previews and the metadata
//! of a file laid out like a D300's, an older body that records its serial
//! number and its lens's range only in its MakerNote. Then the same on real
//! Z 9 files, when they have been downloaded (see
//! [`support::samples::path`]).

mod support;

use std::io::Cursor;

use raw_files::{
    Format, Orientation, Preview, PreviewFormat, Rational, RawFile, SignedRational, Size,
};
use support::nef::{
    D300_FULL_SIZE_JPEG, D300_LENS, D300_PREVIEW_JPEG, D300_RAW, D300_SERIAL_NUMBER,
    FULL_SIZE_JPEG, IFD0_JPEG, LENS_MODEL_Z14_30, MEDIUM_JPEG, PREVIEW_JPEG, RAW_STRIP_START,
    Z9Blobs, d300_file, d300_file_with_blobs, stub, z9_file, z9_file_with, z9_file_with_blobs,
};
use support::samples::{Jpeg, Sample, check, sizes};
use support::tiff_builder::{Built, Order};

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

/// `size` as a preview reports it.
fn wide((width, height): (u16, u16)) -> (u32, u32) {
    (u32::from(width), u32::from(height))
}

/// The length of the stub JPEG of `size`.
fn stub_len(size: (u16, u16)) -> u64 {
    u64::try_from(stub(size).len()).unwrap()
}

/// Checks that the Z 9 file `built`, whose blobs are `blobs`, opens as a
/// NEF with its four JPEGs, smallest first, each where the fixture put it;
/// `label` names the file in a failure.
fn assert_the_four_jpegs(built: &Built, blobs: Z9Blobs, label: &str) {
    let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
    let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
    for (how, format, previews) in [
        ("bytes", from_bytes.format(), from_bytes.previews()),
        ("reader", from_reader.format(), from_reader.previews()),
    ] {
        assert_eq!(format, Format::Nef, "{label}, from {how}");
        assert_eq!(
            sizes(previews),
            [
                wide(IFD0_JPEG),
                wide(PREVIEW_JPEG),
                wide(MEDIUM_JPEG),
                wide(FULL_SIZE_JPEG)
            ],
            "{label}, from {how}"
        );
        let expected = [
            (built.blob_offset(blobs.ifd0_jpeg), IFD0_JPEG),
            (
                built.blob_offset(blobs.makernote) + blobs.preview_in_makernote,
                PREVIEW_JPEG,
            ),
            (built.blob_offset(blobs.medium_jpeg), MEDIUM_JPEG),
            (built.blob_offset(blobs.full_size_jpeg), FULL_SIZE_JPEG),
        ];
        for (preview, (offset, size)) in previews.iter().zip(expected) {
            assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label}, from {how}");
            assert_eq!(preview.byte_range().start, offset, "{label}, from {how}");
            assert_eq!(preview.byte_len(), stub_len(size), "{label}, from {how}");
        }
    }
}

#[test]
fn the_z9_file_opens_as_nef_with_its_four_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = z9_file_with_blobs(order, 1);
        assert_the_four_jpegs(&built, blobs, &format!("{order:?}"));
    }
}

#[test]
fn a_makernote_in_the_other_byte_order_from_the_files_gives_the_same_four_jpegs() {
    for (order, makernote_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
        let (built, blobs) = z9_file_with(order, 1, makernote_order);
        assert_the_four_jpegs(
            &built,
            blobs,
            &format!("{order:?}, MakerNote {makernote_order:?}"),
        );
    }
}

#[test]
fn reading_the_makernote_preview_returns_exactly_the_stubs_bytes() {
    let built = z9_file(Order::Big, 1);
    let mut file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
    let preview = file.previews()[1];
    assert_eq!((preview.width(), preview.height()), wide(PREVIEW_JPEG));
    assert_eq!(file.read_preview(&preview).unwrap(), stub(PREVIEW_JPEG));
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
    assert_eq!(
        size(file.preview_at_least(600, 400)),
        Some(wide(PREVIEW_JPEG))
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
        assert_eq!(file.previews().len(), 4, "Orientation {value}");
        for preview in file.previews() {
            assert_eq!(preview.orientation(), orientation, "Orientation {value}");
        }
    }
}

/// The fraction `numerator`/`denominator`, as stored.
fn fraction(numerator: u32, denominator: u32) -> Rational {
    Rational {
        numerator,
        denominator,
    }
}

/// Checks that the Z 9 file `built`, whose IFD0's Orientation is
/// `orientation`, gives the camera, the serial number, the capture time,
/// the exposure, the lens and the sizes the fixture records, and that
/// orientation, from bytes and from a reader; `label` names the file in a
/// failure.
fn assert_the_z9_metadata(built: &Built, orientation: Orientation, label: &str) {
    let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
    let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
    for (how, metadata, previews) in [
        ("bytes", from_bytes.metadata(), from_bytes.previews()),
        ("reader", from_reader.metadata(), from_reader.previews()),
    ] {
        assert_eq!(
            metadata.make(),
            Some("NIKON CORPORATION"),
            "{label}, from {how}"
        );
        assert_eq!(metadata.model(), Some("NIKON Z 9"), "{label}, from {how}");
        assert_eq!(
            metadata.serial_number(),
            Some("1234567"),
            "{label}, from {how}"
        );
        let time = metadata.capture_time().unwrap();
        assert_eq!(
            time.to_string(),
            "2022-02-05T13:46:41.85-05:00",
            "{label}, from {how}"
        );
        assert_eq!(
            (time.year(), time.month(), time.day()),
            (2022, 2, 5),
            "{label}, from {how}"
        );
        assert_eq!(
            (time.hour(), time.minute(), time.second()),
            (13, 46, 41),
            "{label}, from {how}"
        );
        assert_eq!(time.nanosecond(), 850_000_000, "{label}, from {how}");
        assert_eq!(time.offset_minutes(), Some(-300), "{label}, from {how}");
        assert_eq!(metadata.orientation(), orientation, "{label}, from {how}");
        // The orientation is the one every preview takes.
        assert_eq!(previews.len(), 4, "{label}, from {how}");
        for preview in previews {
            assert_eq!(preview.orientation(), orientation, "{label}, from {how}");
        }
        // The exposure as stored: 1/320 s at f/9, 18 mm and ISO 64, with
        // no compensation.
        let exposure = metadata.exposure();
        assert_eq!(exposure.iso(), Some(64), "{label}, from {how}");
        assert_eq!(
            exposure.exposure_time(),
            Some(fraction(10, 3200)),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.f_number(),
            Some(fraction(900, 100)),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.focal_length(),
            Some(fraction(180, 10)),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.exposure_bias(),
            Some(SignedRational {
                numerator: 0,
                denominator: 6,
            }),
            "{label}, from {how}"
        );
        // The lens, its model read whole through the NULs that pad it and
        // its serial number trimmed of the spaces that pad it.
        let lens = metadata.lens().unwrap();
        assert_eq!(lens.make(), Some("NIKON"), "{label}, from {how}");
        assert_eq!(lens.model(), Some(LENS_MODEL_Z14_30), "{label}, from {how}");
        assert_eq!(
            lens.serial_number(),
            Some("12345678"),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.min_focal_length(),
            Some(fraction(140, 10)),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.max_focal_length(),
            Some(fraction(300, 10)),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.f_number_at_min_focal_length(),
            Some(fraction(400, 100)),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.f_number_at_max_focal_length(),
            Some(fraction(400, 100)),
            "{label}, from {how}"
        );
        // The raw size is SubIFD1's, the raw data's; with no default crop
        // in the file, the image size is the MakerNote's crop area, the
        // raw less its border.
        let raw = Size {
            width: 8280,
            height: 5520,
        };
        let image = Size {
            width: 8256,
            height: 5504,
        };
        assert_eq!(metadata.raw_size(), Some(raw), "{label}, from {how}");
        assert_eq!(metadata.image_size(), Some(image), "{label}, from {how}");
    }
}

#[test]
fn the_z9_file_gives_every_value_its_fixture_records() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let built = z9_file(order, value);
            assert_the_z9_metadata(
                &built,
                orientation,
                &format!("{order:?}, Orientation {value}"),
            );
        }
    }
}

#[test]
fn a_makernote_in_the_other_byte_order_from_the_files_gives_the_same_metadata() {
    for (order, makernote_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
        let (built, _) = z9_file_with(order, 8, makernote_order);
        assert_the_z9_metadata(
            &built,
            Orientation::Rotate270,
            &format!("{order:?}, MakerNote {makernote_order:?}"),
        );
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
        assert_eq!(starts.len(), 4, "{order:?}");
    }
}

#[test]
fn the_d300_file_opens_as_nef_with_its_two_jpegs_smallest_first() {
    // The MakerNote's small JPEG and SubIFD0's full-size one; IFD0's
    // uncompressed thumbnail is not a JPEG, and the raw strip is not a
    // preview, as on the public D300 sample 928.
    for order in ORDERS {
        let (built, blobs) = d300_file_with_blobs(order, 1);
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        assert_eq!(file.format(), Format::Nef, "{order:?}");
        let previews = file.previews();
        assert_eq!(
            sizes(previews),
            [wide(D300_PREVIEW_JPEG), wide(D300_FULL_SIZE_JPEG)],
            "{order:?}"
        );
        let expected = [
            (
                built.blob_offset(blobs.makernote) + blobs.preview_in_makernote,
                D300_PREVIEW_JPEG,
            ),
            (built.blob_offset(blobs.full_size_jpeg), D300_FULL_SIZE_JPEG),
        ];
        for (preview, (offset, size)) in previews.iter().zip(expected) {
            assert_eq!(preview.format(), PreviewFormat::Jpeg, "{order:?}");
            assert_eq!(preview.byte_range().start, offset, "{order:?}");
            assert_eq!(preview.byte_len(), stub_len(size), "{order:?}");
        }
        let starts: Vec<u64> = previews.iter().map(|p| p.byte_range().start).collect();
        for strip in [blobs.thumbnail, blobs.raw] {
            assert!(
                !starts.contains(&built.blob_offset(strip)),
                "{order:?}: {starts:?}"
            );
        }
    }
}

#[test]
fn the_d300_file_gives_its_serial_number_and_lens_range_from_the_makernote() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let label = format!("{order:?}, Orientation {value}");
            let built = d300_file(order, value);
            let file = RawFile::from_bytes(&built.bytes).unwrap();
            let metadata = file.metadata();
            assert_eq!(metadata.make(), Some("NIKON CORPORATION"), "{label}");
            assert_eq!(metadata.model(), Some("NIKON D300"), "{label}");
            // No BodySerialNumber in the Exif IFD: the MakerNote's.
            assert_eq!(
                metadata.serial_number(),
                Some(D300_SERIAL_NUMBER),
                "{label}"
            );
            // No OffsetTimeOriginal, so no offset from UTC.
            let time = metadata.capture_time().unwrap();
            assert_eq!(time.to_string(), "2008-05-17T10:22:31.34", "{label}");
            assert_eq!(time.offset_minutes(), None, "{label}");
            assert_eq!(metadata.orientation(), orientation, "{label}");
            // The exposure as stored: 1/10 s at f/7.1, 50 mm and ISO 200,
            // with no compensation. The ISO is the Exif one, which the
            // MakerNote's `[0, 200]` agrees with.
            let exposure = metadata.exposure();
            assert_eq!(exposure.iso(), Some(200), "{label}");
            assert_eq!(exposure.exposure_time(), Some(fraction(10, 100)), "{label}");
            assert_eq!(exposure.f_number(), Some(fraction(71, 10)), "{label}");
            assert_eq!(exposure.focal_length(), Some(fraction(500, 10)), "{label}");
            assert_eq!(
                exposure.exposure_bias(),
                Some(SignedRational {
                    numerator: 0,
                    denominator: 6,
                }),
                "{label}"
            );
            // No lens tag in the Exif IFD: a lens of the MakerNote's range
            // alone, with no name and no serial number.
            let lens = metadata.lens().unwrap();
            assert_eq!(
                (lens.make(), lens.model(), lens.serial_number()),
                (None, None, None),
                "{label}"
            );
            let [min_focal, max_focal, f_at_min, f_at_max] =
                D300_LENS.map(|(n, d)| Some(fraction(n, d)));
            assert_eq!(lens.min_focal_length(), min_focal, "{label}");
            assert_eq!(lens.max_focal_length(), max_focal, "{label}");
            assert_eq!(lens.f_number_at_min_focal_length(), f_at_min, "{label}");
            assert_eq!(lens.f_number_at_max_focal_length(), f_at_max, "{label}");
            // No CropArea and no default crop: the image size is the raw
            // size, border pixels included.
            let raw = Size {
                width: D300_RAW.0,
                height: D300_RAW.1,
            };
            assert_eq!(metadata.raw_size(), Some(raw), "{label}");
            assert_eq!(metadata.image_size(), Some(raw), "{label}");
        }
    }
}

// Real files. The Z 9 files of raw.pixls.us, one per compression mode, are
// listed with their hashes in `tests/samples.tsv`; `scripts/fetch-samples.sh`
// downloads them. They are too large to commit, so a test whose file is
// absent says so and passes: the suite runs offline.

/// The Z 9 samples, each with its four previews, smallest first: the IFD0
/// thumbnail, the MakerNote's preview, the medium JPEG in SubIFD2 and the
/// full-size JPEG in SubIFD0. None is padded after its end-of-image marker.
const SAMPLES: &[Sample] = &[
    // Z 9, 14-bit lossless compression (raw.pixls.us 5146)
    Sample {
        file: "nikon-z9-lossless.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 222_540,
                len: 9_354,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 99_432,
                len: 122_857,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 332_288,
                len: 1_075_885,
                after_eoi: 0,
            },
            Jpeg {
                width: 8256,
                height: 5504,
                offset: 1_408_512,
                len: 6_492_228,
                after_eoi: 0,
            },
        ],
    },
    // Z 9, High Efficiency* (raw.pixls.us 5147)
    Sample {
        file: "nikon-z9-he-star.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 229_076,
                len: 9_823,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 99_372,
                len: 129_452,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 338_944,
                len: 927_453,
                after_eoi: 0,
            },
            Jpeg {
                width: 8256,
                height: 5504,
                offset: 1_266_688,
                len: 5_760_370,
                after_eoi: 0,
            },
        ],
    },
    // Z 9, High Efficiency (raw.pixls.us 5148)
    Sample {
        file: "nikon-z9-he.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 219_300,
                len: 10_165,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 99_372,
                len: 119_678,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 329_728,
                len: 1_081_068,
                after_eoi: 0,
            },
            Jpeg {
                width: 8256,
                height: 5504,
                offset: 1_411_072,
                len: 6_288_717,
                after_eoi: 0,
            },
        ],
    },
];

#[test]
fn every_z9_sample_opens_with_its_four_previews() {
    for sample in SAMPLES {
        check(sample, Format::Nef);
    }
}
