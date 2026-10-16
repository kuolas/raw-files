// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ARW reader from outside the crate, on files laid out like each
//! generation of Sony's RAW family (see [`support::arw`]): what each opens
//! as, which previews it lists, that none of them is the raw data, what
//! reading one returns, and what metadata it gives. Then the previews of
//! the real Sony sample files, when they have been downloaded (see
//! [`support::samples::path`]).

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;

use raw_files::{
    Exposure, Format, Lens, Orientation, PreviewFormat, Rational, RawFile, SignedRational,
};
use support::arw::{
    A100_JPEG, A100_RAW_AS_COUNT, A100_TAG_SIZE, ARW4_FULL_SIZE_JPEG, ArwBlobs, IFD0_JPEG,
    MEDIUM_FULL_SIZE_JPEG, SR2_JPEG, SRF_JPEG, THUMBNAIL, a100_file, arq_file, arw2_file,
    arw4_file, arw4_medium_file, sr2_file, srf_file, stub,
};
use support::samples::{Jpeg, Sample, check, sizes};
use support::tiff_builder::{BlobId, Built, Order};

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

/// The SubIFDs tag, whose value on the A100 is the offset of its raw data.
const SUB_IFDS: u16 = 0x014A;

/// The entries an IFD may have under the default limits.
const DEFAULT_MAX_IFD_ENTRIES: u16 = 1000;

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

/// Checks that `exposure` is the one the ARW 4 files record, as stored:
/// 1/50 s at f/5.6, 50 mm and ISO 250, with no compensation; `label` names
/// the file in a failure.
fn assert_the_arw4_exposure(exposure: &Exposure, label: &str) {
    let fraction = |numerator, denominator| {
        Some(Rational {
            numerator,
            denominator,
        })
    };
    assert_eq!(exposure.iso(), Some(250), "{label}");
    assert_eq!(exposure.exposure_time(), fraction(1, 50), "{label}");
    assert_eq!(exposure.f_number(), fraction(56, 10), "{label}");
    assert_eq!(exposure.focal_length(), fraction(500, 10), "{label}");
    assert_eq!(
        exposure.exposure_bias(),
        Some(SignedRational {
            numerator: 0,
            denominator: 10,
        }),
        "{label}"
    );
}

/// Checks that `lens` is the one the ARW 4 files record: the model
/// `FE 50mm F2.5 G`, a 50 mm prime at f/2.5, with no make and no serial
/// number; `label` names the file in a failure.
fn assert_the_arw4_lens(lens: Option<&Lens>, label: &str) {
    let lens = lens.unwrap_or_else(|| panic!("{label}: no lens"));
    let fraction = |numerator, denominator| {
        Some(Rational {
            numerator,
            denominator,
        })
    };
    assert_eq!(lens.make(), None, "{label}");
    assert_eq!(lens.model(), Some("FE 50mm F2.5 G"), "{label}");
    assert_eq!(lens.serial_number(), None, "{label}");
    assert_eq!(lens.min_focal_length(), fraction(500, 10), "{label}");
    assert_eq!(lens.max_focal_length(), fraction(500, 10), "{label}");
    assert_eq!(
        lens.f_number_at_min_focal_length(),
        fraction(25, 10),
        "{label}"
    );
    assert_eq!(
        lens.f_number_at_max_focal_length(),
        fraction(25, 10),
        "{label}"
    );
}

#[test]
fn the_arw4_file_gives_its_camera_capture_time_exposure_lens_and_orientation() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let label = format!("{order:?}, Orientation {value}");
            let (built, _) = arw4_file(order, value);
            let file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
            let metadata = file.metadata();
            assert_eq!(metadata.make(), Some("SONY"), "{label}");
            assert_eq!(metadata.model(), Some("ILCE-7M4"), "{label}");
            // Like the sample it copies, the file records no serial number.
            assert_eq!(metadata.serial_number(), None, "{label}");
            let time = metadata.capture_time().unwrap();
            assert_eq!(time.to_string(), "2023-10-19T16:13:48.776+02:00", "{label}");
            assert_eq!(time.nanosecond(), 776_000_000, "{label}");
            assert_eq!(time.offset_minutes(), Some(120), "{label}");
            assert_eq!(metadata.orientation(), orientation, "{label}");
            assert_the_arw4_exposure(metadata.exposure(), &label);
            assert_the_arw4_lens(metadata.lens(), &label);
            // The sizes are not read yet.
            assert_eq!(metadata.raw_size(), None, "{label}");
            assert_eq!(metadata.image_size(), None, "{label}");
            for preview in file.previews() {
                assert_eq!(preview.orientation(), orientation, "{label}");
            }
        }
    }
}

#[test]
fn every_layout_gives_its_camera_and_a_time_exposure_and_lens_only_if_it_has_them() {
    // Each layout's name, model, capture time, and whether it records an
    // exposure and a lens.
    let expected = [
        (
            "ARW 4",
            "ILCE-7M4",
            Some("2023-10-19T16:13:48.776+02:00"),
            true,
        ),
        (
            "ARW 4 medium",
            "ILCE-7M4",
            Some("2023-10-19T16:13:48.776+02:00"),
            true,
        ),
        ("ARW 2", "SLT-A99V", None, false),
        ("ARQ", "ILCE-7RM3", None, false),
        ("SR2", "DSC-R1", None, false),
        ("SRF", "DSC-F828", None, false),
        // Its Make is `SONY` and eleven spaces, which are padding.
        ("A100", "DSLR-A100", None, false),
    ];
    for order in ORDERS {
        let layouts = every_layout(order);
        assert_eq!(layouts.len(), expected.len());
        for ((name, (built, _)), (expected_name, model, time, has_exposure_and_lens)) in
            layouts.into_iter().zip(expected)
        {
            assert_eq!(name, expected_name);
            let label = format!("{name}, {order:?}");
            let file = RawFile::from_bytes(&built.bytes).unwrap();
            assert_eq!(file.format(), Format::Arw, "{label}");
            let metadata = file.metadata();
            assert_eq!(metadata.make(), Some("SONY"), "{label}");
            assert_eq!(metadata.model(), Some(model), "{label}");
            assert_eq!(metadata.serial_number(), None, "{label}");
            assert_eq!(
                metadata.capture_time().map(|t| t.to_string()).as_deref(),
                time,
                "{label}"
            );
            if has_exposure_and_lens {
                assert_the_arw4_exposure(metadata.exposure(), &label);
                assert_the_arw4_lens(metadata.lens(), &label);
            } else {
                assert_eq!(metadata.exposure(), &Exposure::default(), "{label}");
                assert_eq!(metadata.lens(), None, "{label}");
            }
            // Every layout is built upright, and the SRF has no
            // Orientation at all; the previews agree.
            assert_eq!(metadata.orientation(), Orientation::NoTransforms, "{label}");
            assert!(!file.previews().is_empty(), "{label}");
            for preview in file.previews() {
                assert_eq!(preview.orientation(), metadata.orientation(), "{label}");
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

// Real files. The 36 CC0 Sony-family files of raw.pixls.us that the ARW
// notes were checked against are listed with their hashes in
// `tests/samples.tsv`; `scripts/fetch-samples.sh` downloads them. They are
// too large to commit, so a test whose file is absent says so and passes:
// the suite runs offline. The ARQ, at 325 MiB, is in the list's large tier,
// which the script fetches only with `RAW_FILES_LARGE=1`, so its test is
// nearly always skipped.

/// The Sony samples of the list's normal tier, which CI downloads, each
/// with the previews the ARW notes predict for it.
const SAMPLES: &[Sample] = &[
    // SLT-A55 (raw.pixls.us 964)
    Sample {
        file: "sony-slt-a55.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 47_796,
                len: 3_028,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 163_891,
                len: 608_696,
                after_eoi: 51,
            },
        ],
    },
    // ILCE-7RM2 (raw.pixls.us 1310)
    Sample {
        file: "sony-ilce-7rm2-compressed.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_908,
                len: 10_962,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 146_082,
                len: 564_782,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7RM2 (raw.pixls.us 1311)
    Sample {
        file: "sony-ilce-7rm2-uncompressed.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_908,
                len: 10_931,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 146_082,
                len: 559_709,
                after_eoi: 0,
            },
        ],
    },
    // DSC-F828 (raw.pixls.us 1351)
    Sample {
        file: "sony-dsc-f828.srf",
        orientation: Orientation::NoTransforms,
        previews: &[Jpeg {
            width: 640,
            height: 480,
            offset: 740,
            len: 151_046,
            after_eoi: 0,
        }],
    },
    // NEX-5 (raw.pixls.us 1457)
    Sample {
        file: "sony-nex-5.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 47_796,
                len: 2_790,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 163_891,
                len: 469_514,
                after_eoi: 51,
            },
        ],
    },
    // ILCE-9 (raw.pixls.us 1541)
    Sample {
        file: "sony-ilce-9-compressed.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_056,
                len: 9_432,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 151_714,
                len: 398_026,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-9 (raw.pixls.us 1542)
    Sample {
        file: "sony-ilce-9-uncompressed.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_056,
                len: 9_334,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 151_714,
                len: 393_347,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7S (raw.pixls.us 1582)
    Sample {
        file: "sony-ilce-7s.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_516,
                len: 8_908,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 144_034,
                len: 734_118,
                after_eoi: 0,
            },
        ],
    },
    // DSLR-A200 (raw.pixls.us 2271)
    Sample {
        file: "sony-dslr-a200.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 32_770,
                len: 3_726,
                after_eoi: 15,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 155_683,
                len: 552_749,
                after_eoi: 0,
            },
        ],
    },
    // DSLR-A100 (raw.pixls.us 2346)
    Sample {
        file: "sony-dslr-a100.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 68_535,
                len: 4_071,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 480,
                offset: 25_683,
                len: 42_850,
                after_eoi: 0,
            },
        ],
    },
    // DSLR-A700 (raw.pixls.us 2521)
    Sample {
        file: "sony-dslr-a700.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 32_770,
                len: 4_686,
                after_eoi: 4,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 155_683,
                len: 533_647,
                after_eoi: 0,
            },
        ],
    },
    // DSC-RX100M6 (raw.pixls.us 3012)
    Sample {
        file: "sony-dsc-rx100m6-1x1.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_978,
                len: 6_918,
                after_eoi: 0,
            },
            Jpeg {
                width: 1088,
                height: 1080,
                offset: 143_522,
                len: 316_756,
                after_eoi: 0,
            },
        ],
    },
    // DSC-RX100M6 (raw.pixls.us 3015)
    Sample {
        file: "sony-dsc-rx100m6-16x9.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 38_978,
                len: 6_326,
                after_eoi: 0,
            },
            Jpeg {
                width: 1920,
                height: 1080,
                offset: 143_522,
                len: 476_154,
                after_eoi: 0,
            },
        ],
    },
    // DSC-R1 (raw.pixls.us 3221)
    Sample {
        file: "sony-dsc-r1.sr2",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 165_198,
                len: 28_372,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 1_358,
                len: 145_173,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7RM3 (raw.pixls.us 3251)
    Sample {
        file: "sony-ilce-7rm3-pixel-shift.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_246,
                len: 6_053,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 135_330,
                len: 254_293,
                after_eoi: 0,
            },
        ],
    },
    // SLT-A99 (raw.pixls.us 3751)
    Sample {
        file: "sony-slt-a99.arw",
        orientation: Orientation::Rotate270,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 37_158,
                len: 10_091,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 143_522,
                len: 905_337,
                after_eoi: 0,
            },
        ],
    },
    // Hasselblad Lunar (raw.pixls.us 4051)
    Sample {
        file: "hasselblad-lunar.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 40_526,
                len: 8_913,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 144_546,
                len: 379_829,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-1 (raw.pixls.us 4466)
    Sample {
        file: "sony-ilce-1-lossless.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 44_008,
                len: 13_144,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 143_522,
                len: 484_164,
                after_eoi: 0,
            },
            Jpeg {
                width: 8640,
                height: 5760,
                offset: 630_784,
                len: 9_751_756,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7SM3 (raw.pixls.us 5418)
    Sample {
        file: "sony-ilce-7sm3.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_808,
                len: 10_649,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 143_522,
                len: 804_619,
                after_eoi: 0,
            },
            Jpeg {
                width: 4240,
                height: 2832,
                offset: 950_272,
                len: 2_302_638,
                after_eoi: 0,
            },
        ],
    },
    // DSC-RX100 (raw.pixls.us 5811)
    Sample {
        file: "sony-dsc-rx100.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 27_030,
                len: 9_550,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 105_122,
                len: 838_931,
                after_eoi: 0,
            },
        ],
    },
    // DSLR-A900 (raw.pixls.us 5858)
    Sample {
        file: "sony-dslr-a900.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 32_770,
                len: 4_046,
                after_eoi: 15,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 155_683,
                len: 845_435,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-6700 (raw.pixls.us 6736)
    Sample {
        file: "sony-ilce-6700-lossless.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_872,
                len: 10_150,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 534_901,
                after_eoi: 0,
            },
            Jpeg {
                width: 6192,
                height: 4128,
                offset: 733_184,
                len: 7_218_228,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M4 (raw.pixls.us 6929)
    Sample {
        file: "sony-ilce-7m4-lossless-l.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_946,
                len: 12_285,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 584_555,
                after_eoi: 0,
            },
            Jpeg {
                width: 7008,
                height: 4672,
                offset: 782_336,
                len: 7_424_248,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M4 (raw.pixls.us 6930)
    Sample {
        file: "sony-ilce-7m4-lossless-m.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_946,
                len: 12_326,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 492_638,
                after_eoi: 0,
            },
            Jpeg {
                width: 4608,
                height: 3072,
                offset: 692_224,
                len: 3_586_860,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M4 (raw.pixls.us 6931)
    Sample {
        file: "sony-ilce-7m4-lossless-s.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_946,
                len: 12_235,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 483_370,
                after_eoi: 0,
            },
            Jpeg {
                width: 3504,
                height: 2336,
                offset: 684_032,
                len: 2_354_346,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M4 (raw.pixls.us 6933)
    Sample {
        file: "sony-ilce-7m4-uncompressed-apsc.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_946,
                len: 12_281,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 192_674,
                len: 479_036,
                after_eoi: 0,
            },
            Jpeg {
                width: 4608,
                height: 3072,
                offset: 671_744,
                len: 3_837_614,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M4 (raw.pixls.us 6936)
    Sample {
        file: "sony-ilce-7m4-compressed-apsc.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_946,
                len: 12_903,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 192_674,
                len: 515_116,
                after_eoi: 0,
            },
            Jpeg {
                width: 4608,
                height: 3072,
                offset: 708_608,
                len: 4_087_916,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-9M3 (raw.pixls.us 7361)
    Sample {
        file: "sony-ilce-9m3-compressed.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 44_030,
                len: 10_700,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 478_764,
                after_eoi: 0,
            },
            Jpeg {
                width: 6000,
                height: 4000,
                offset: 675_840,
                len: 4_086_721,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-9M3 (raw.pixls.us 7364)
    Sample {
        file: "sony-ilce-9m3-lossless-s.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 44_030,
                len: 10_767,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 485_378,
                after_eoi: 0,
            },
            Jpeg {
                width: 3008,
                height: 2000,
                offset: 684_032,
                len: 1_379_719,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-1M2 (raw.pixls.us 7835)
    Sample {
        file: "sony-ilce-1m2-lossy-apsc.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 44_228,
                len: 4_170,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 196_770,
                len: 107_146,
                after_eoi: 0,
            },
            Jpeg {
                width: 5616,
                height: 3744,
                offset: 307_200,
                len: 1_102_572,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M5 (raw.pixls.us 8846)
    Sample {
        file: "sony-ilce-7m5-compressed-hq.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_992,
                len: 6_339,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 200_866,
                len: 333_230,
                after_eoi: 0,
            },
            Jpeg {
                width: 7008,
                height: 4672,
                offset: 536_576,
                len: 4_259_249,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M5 (raw.pixls.us 8847)
    Sample {
        file: "sony-ilce-7m5-compressed.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_992,
                len: 6_360,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 200_866,
                len: 333_372,
                after_eoi: 0,
            },
            Jpeg {
                width: 7008,
                height: 4672,
                offset: 536_576,
                len: 4_270_440,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M5 (raw.pixls.us 8848)
    Sample {
        file: "sony-ilce-7m5-lossless-apsc.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_992,
                len: 7_374,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 200_866,
                len: 440_228,
                after_eoi: 0,
            },
            Jpeg {
                width: 4608,
                height: 3072,
                offset: 643_072,
                len: 2_359_452,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M5 (raw.pixls.us 8849)
    Sample {
        file: "sony-ilce-7m5-compressed-hq-apsc.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_992,
                len: 7_329,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 200_866,
                len: 433_383,
                after_eoi: 0,
            },
            Jpeg {
                width: 4608,
                height: 3072,
                offset: 634_880,
                len: 2_312_397,
                after_eoi: 0,
            },
        ],
    },
    // ILCE-7M5 (raw.pixls.us 8850)
    Sample {
        file: "sony-ilce-7m5-compressed-apsc.arw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_992,
                len: 7_283,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 200_866,
                len: 452_187,
                after_eoi: 0,
            },
            Jpeg {
                width: 4608,
                height: 3072,
                offset: 655_360,
                len: 2_555_667,
                after_eoi: 0,
            },
        ],
    },
];

/// The Sony samples of the list's large tier, fetched only on request: the
/// ARQ, whose container is that of the pixel-shift frame 3251.
const LARGE_SAMPLES: &[Sample] = &[
    // ILCE-7RM3 (raw.pixls.us 3255), large tier
    Sample {
        file: "sony-ilce-7rm3.arq",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 43_246,
                len: 6_053,
                after_eoi: 0,
            },
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 135_330,
                len: 254_293,
                after_eoi: 0,
            },
        ],
    },
];

#[test]
fn every_sony_sample_opens_with_the_previews_the_notes_predict() {
    for sample in SAMPLES {
        check(sample, Format::Arw);
    }
}

#[test]
fn the_arq_sample_opens_with_the_two_previews_of_its_pixel_shift_frame() {
    for sample in LARGE_SAMPLES {
        check(sample, Format::Arw);
    }
}

#[test]
fn the_hasselblad_lunar_sample_opens_as_a_sony_file() {
    // The Lunar is a Sony NEX-7 sold under Hasselblad's name. Its Make is
    // not Sony's, so it is the one sample the reader claims by its other
    // rule: a Hasselblad Make with Sony's private pointer in the four-byte
    // shape Sony writes it, in a file that is not a DNG, which every reader
    // refuses first. The loop above covers it too; this test names that
    // rule.
    let lunar = SAMPLES
        .iter()
        .find(|sample| sample.file == "hasselblad-lunar.arw")
        .unwrap();
    check(lunar, Format::Arw);
}

#[test]
fn the_sample_list_and_the_expected_previews_name_the_same_sony_files() {
    // The list is in the repository, so this runs without any download.
    let list =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples.tsv")).unwrap();
    let mut lines = list.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    assert_eq!(
        header,
        ["id", "format", "file", "tier", "sha256", "licence", "url"]
    );
    let mut listed = BTreeSet::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), header.len(), "{line}");
        if fields[1] == "arw" {
            listed.insert((fields[2], fields[3]));
        }
    }

    let tables = SAMPLES
        .iter()
        .map(|sample| (sample.file, "normal"))
        .chain(LARGE_SAMPLES.iter().map(|sample| (sample.file, "large")));
    let expected: BTreeSet<(&str, &str)> = tables.clone().collect();
    assert_eq!(
        expected.len(),
        tables.count(),
        "a file is in the tables twice"
    );
    let unexpected: Vec<_> = listed.difference(&expected).collect();
    let unlisted: Vec<_> = expected.difference(&listed).collect();
    assert!(
        unexpected.is_empty() && unlisted.is_empty(),
        "listed without expected previews: {unexpected:?}; \
         expected previews but not listed: {unlisted:?}"
    );
}
