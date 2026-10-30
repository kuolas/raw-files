// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ARW reader from outside the crate, on files laid out like each
//! generation of Sony's RAW family (see [`support::arw`]): what each opens
//! as, which previews it lists, that none of them is the raw data, what
//! reading one returns, and what metadata it gives, the focus point that
//! the ARW 4 files' MakerNote records among it. Then the real Sony sample
//! files, when they have been downloaded (see [`support::samples::path`]):
//! the previews each gives, and its metadata, every value of it, the focus
//! point and the camera settings among them.

mod support;

use std::collections::BTreeSet;
use std::io::Cursor;

use raw_files::{
    CameraSettings, ColorSpace, Exposure, ExposureProgram, FlashMode, FocusOrigin, Format, Lens,
    Metadata, MeteringMode, Orientation, Point, PreviewFormat, Rational, RawFile, SignedRational,
    Size, WhiteBalanceMode,
};
use support::arw::{
    A100_JPEG, A100_RAW_AS_COUNT, A100_TAG_SIZE, ARW4_FOCUS_CENTER, ARW4_FOCUS_FRAME,
    ARW4_FOCUS_SIZE, ARW4_FULL_SIZE_JPEG, ArwBlobs, IFD0_JPEG, MEDIUM_FULL_SIZE_JPEG, SR2_JPEG,
    SRF_JPEG, THUMBNAIL, a100_file, arq_file, arw2_file, arw4_file, arw4_medium_file, sr2_file,
    srf_file, stub,
};
use support::samples::{
    Jpeg, Sample, SampleFineTune, SampleFocus, SampleLens, SampleMetadata, SampleSettings,
    SerialShape, check, check_metadata, listed, sizes,
};
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
/// 1/50 s at f/5.6, 50 mm, 50 mm in 35 mm terms too, and ISO 250, with no
/// compensation; `label` names the file in a failure.
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
    assert_eq!(exposure.focal_length_in_35mm_film(), Some(50), "{label}");
    assert_eq!(
        exposure.exposure_bias(),
        Some(SignedRational {
            numerator: 0,
            denominator: 10,
        }),
        "{label}"
    );
}

/// A size of `width` by `height` pixels.
fn size(width: u32, height: u32) -> Size {
    Size { width, height }
}

/// Checks that `lens` is the one the ARW 4 files record: the model
/// `FE 50mm F2.5 G`, a 50 mm prime at f/2.5, with no make and no serial
/// number; `label` names the file in a failure.
fn assert_the_arw4_lens(lens: &Lens, label: &str) {
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

/// Checks that `metadata`'s focus is the one the ARW 4 files' MakerNote
/// records: a frame of 7008 by 4672 pixels with the centre at (3832, 2102)
/// and an area of 350 by 351, as given, and nothing said about whether
/// focus was confirmed; and that its focus points are that point alone, as
/// the primary, with no number, recorded, with no area mode, since Sony's
/// is not read. The focus is the list's primary by construction, so the
/// checks of the focus check the point. `label` names the file in a
/// failure.
fn assert_the_arw4_focus(metadata: &Metadata, label: &str) {
    let focus = metadata
        .focus()
        .unwrap_or_else(|| panic!("{label}: no focus point"));
    let (width, height) = wide(ARW4_FOCUS_FRAME);
    let (x, y) = wide(ARW4_FOCUS_CENTER);
    let (area_width, area_height) = wide(ARW4_FOCUS_SIZE);
    assert_eq!(focus.frame(), size(width, height), "{label}");
    assert_eq!(focus.center(), Point { x, y }, "{label}");
    assert_eq!(focus.size(), Some(size(area_width, area_height)), "{label}");
    assert_eq!(focus.in_focus(), None, "{label}");

    let points = metadata
        .focus_points()
        .unwrap_or_else(|| panic!("{label}: no focus points"));
    assert_eq!(points.points().len(), 1, "{label}");
    let primary = points
        .primary()
        .unwrap_or_else(|| panic!("{label}: no primary"));
    assert_eq!(Some(primary), points.points().first(), "{label}");
    assert_eq!(primary.number(), None, "{label}");
    assert_eq!(points.origin(), FocusOrigin::Recorded, "{label}");
    assert_eq!(points.area_mode(), None, "{label}");
}

#[test]
fn the_arw4_file_gives_every_value_its_fixture_records() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let label = format!("{order:?}, Orientation {value}");
            let (built, _) = arw4_file(order, value);
            let file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
            let metadata = file.metadata();
            assert_eq!(metadata.make(), Some("SONY"), "{label}");
            assert_eq!(metadata.model(), Some("ILCE-7M4"), "{label}");
            assert_eq!(metadata.software(), Some("ILCE-7M4 v2.00"), "{label}");
            // Like the sample it copies, the file records no serial number.
            assert_eq!(metadata.serial_number(), None, "{label}");
            let time = metadata.capture_time().unwrap();
            assert_eq!(time.to_string(), "2023-10-19T16:13:48.776+02:00", "{label}");
            assert_eq!(time.nanosecond(), 776_000_000, "{label}");
            assert_eq!(time.offset_minutes(), Some(120), "{label}");
            assert_eq!(metadata.orientation(), orientation, "{label}");
            assert_the_arw4_exposure(metadata.exposure(), &label);
            assert_the_arw4_lens(metadata.lens(), &label);
            // The raw SubIFD's size, padded to whole tiles, and its
            // default crop, which lies inside it.
            assert_eq!(metadata.raw_size(), Some(size(7168, 5120)), "{label}");
            assert_eq!(metadata.image_size(), Some(size(7008, 4672)), "{label}");
            // From the MakerNote, in the stored orientation whatever the
            // Orientation says.
            assert_the_arw4_focus(metadata, &label);
            // No layout: raw-files has no table of the ILCE-7M4's points.
            assert_eq!(metadata.focus_layout(), None, "{label}");
            // The fixture writes no settings tag, so the settings are the
            // empty record.
            assert_eq!(
                metadata.camera_settings(),
                &CameraSettings::default(),
                "{label}"
            );
            for preview in file.previews() {
                assert_eq!(preview.orientation(), orientation, "{label}");
            }
        }
    }
}

#[test]
fn every_layout_gives_the_metadata_its_fixture_records() {
    let arw4_time = Some("2023-10-19T16:13:48.776+02:00");
    // Each layout's name, model, software, capture time, whether it
    // records an exposure, a lens and a focus point, and its raw size and
    // image size.
    // The medium file's MakerNote is the lossless file's, so its focus
    // point is too, in the full-size frame.
    let expected = [
        (
            "ARW 4",
            "ILCE-7M4",
            Some("ILCE-7M4 v2.00"),
            arw4_time,
            true,
            // Padded to whole tiles, then the default crop.
            Some(size(7168, 5120)),
            Some(size(7008, 4672)),
        ),
        (
            "ARW 4 medium",
            "ILCE-7M4",
            Some("ILCE-7M4 v2.00"),
            arw4_time,
            true,
            Some(size(5120, 3584)),
            Some(size(4608, 3072)),
        ),
        (
            "ARW 2",
            "SLT-A99V",
            Some("SLT-A99V v1.02"),
            None,
            false,
            Some(size(6024, 4024)),
            Some(size(6024, 4024)),
        ),
        (
            "ARQ",
            "ILCE-7RM3",
            Some("ILCE-7RM3 v3.00"),
            None,
            false,
            Some(size(8000, 5320)),
            Some(size(8000, 5320)),
        ),
        (
            "SR2",
            "DSC-R1",
            None,
            None,
            false,
            Some(size(3984, 2608)),
            Some(size(3984, 2608)),
        ),
        // IFD0 is the raw data, by its 14-bit samples.
        (
            "SRF",
            "DSC-F828",
            None,
            None,
            false,
            Some(size(3360, 2460)),
            Some(size(3360, 2460)),
        ),
        // Its Make is `SONY` and eleven spaces, which are padding. Its
        // SubIFD pointer leads to no IFD, and its IFDs are previews, so no
        // IFD describes the raw data, whatever size IFD0's tags declare.
        (
            "A100",
            "DSLR-A100",
            Some("DSLR-A100 v1.04"),
            None,
            false,
            None,
            None,
        ),
    ];
    for order in ORDERS {
        let layouts = every_layout(order);
        assert_eq!(layouts.len(), expected.len());
        for (
            (name, (built, _)),
            (
                expected_name,
                model,
                software,
                time,
                has_exposure_lens_and_focus,
                raw_size,
                image_size,
            ),
        ) in layouts.into_iter().zip(expected)
        {
            assert_eq!(name, expected_name);
            let label = format!("{name}, {order:?}");
            let file = RawFile::from_bytes(&built.bytes).unwrap();
            assert_eq!(file.format(), Format::Arw, "{label}");
            let metadata = file.metadata();
            assert_eq!(metadata.make(), Some("SONY"), "{label}");
            assert_eq!(metadata.model(), Some(model), "{label}");
            assert_eq!(metadata.software(), software, "{label}");
            assert_eq!(metadata.serial_number(), None, "{label}");
            assert_eq!(
                metadata.capture_time().map(|t| t.to_string()).as_deref(),
                time,
                "{label}"
            );
            if has_exposure_lens_and_focus {
                assert_the_arw4_exposure(metadata.exposure(), &label);
                assert_the_arw4_lens(metadata.lens(), &label);
                assert_the_arw4_focus(metadata, &label);
            } else {
                assert_eq!(metadata.exposure(), &Exposure::default(), "{label}");
                assert_eq!(metadata.lens(), &Lens::default(), "{label}");
                assert_eq!(metadata.focus(), None, "{label}");
                assert_eq!(metadata.focus_points(), None, "{label}");
            }
            assert_eq!(metadata.raw_size(), raw_size, "{label}");
            assert_eq!(metadata.image_size(), image_size, "{label}");
            // No layout of autofocus points: raw-files has no table of a
            // Sony body's points.
            assert_eq!(metadata.focus_layout(), None, "{label}");
            // No layout writes a settings tag.
            assert_eq!(
                metadata.camera_settings(),
                &CameraSettings::default(),
                "{label}"
            );
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
    let box_2000_by_1500 = Size {
        width: 2000,
        height: 1500,
    };
    let preview = file.preview_at_least(box_2000_by_1500).unwrap();
    assert_eq!(
        (preview.size().width, preview.size().height),
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
// which the script fetches only with `RAW_FILES_LARGE=1`, so its tests are
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

/// The metadata each Sony sample of the list's normal tier must give, in
/// the order of `tests/samples.tsv`, its serial numbers by their shape
/// alone (see [`SampleMetadata`]). The table was generated from a walk of
/// each file's IFDs and Exif IFD, and its focus points from each file's
/// MakerNote, by the reader's rules, which read the standard tags on Sony
/// files and Sony's MakerNote for what no standard tag records, the focus
/// point and the camera settings that only it records (ADR011, ADR012).
/// So only the ILCE-1 and the
/// ILCE-7M5 give a serial number, the only bodies of the set that write
/// the Exif one; the DSLR-A100 gives no raw size and no image
/// size, since its SubIFDs tag holds the offset of its raw data rather
/// than that of an IFD; the ILCE-7S, with a manual lens whose model is
/// `----`, whose f-number and focal length are 0/10 and whose focal length
/// in 35 mm terms is 0, gives no lens, no f-number and no focal length of
/// either kind; and the RX100 and the RX100M6 give a lens with a range and
/// no model. A file without a default crop gives its raw size as its image
/// size. The software, the body's name and firmware version, and the
/// focal length in 35 mm terms were generated from each file's IFD0 and
/// Exif IFD by the same rules; the DSC-F828 and the DSC-R1 record neither.
/// The bodies from the ILCE-7RM2 on give a focus point from the
/// MakerNote's FocusLocation, with an area from its FocusFrameSize from
/// the ILCE-7SM3 on, and never say whether focus was confirmed; a
/// manual-focus shot and the files without a FocusLocation give none, each
/// row saying why. The camera settings were generated by the same rules,
/// from each file's Exif IFD, Interoperability IFD and MakerNote, by a
/// script written apart from the reader: the DSC-R1 and the DSLR-A900
/// record an "uncalibrated" ColorSpace with the index `R03`, which gives
/// Adobe RGB, and the DSLR-A100 one with an empty index, which gives no
/// colour space. The white balance mode is the MakerNote's WhiteBalance
/// on every file that records one: custom on the two files of the
/// ILCE-9M3, whose Exif records a manual white balance for the LightSource
/// 255, "other", which gives no mode, and shade on the ILCE-7S; the
/// DSC-F828 and the DSC-R1 record none, and the DSLR-A100 records it where
/// the reader does not read it, so theirs is Exif's auto. The ILCE-9M3
/// records the colour temperature 5500 under custom, its custom white
/// balance's own, and every other file 0, which gives none. The
/// fine-tuning is `0, 0` on every file that records one; the DSLR-A100,
/// A200, A700 and A900, the SLT-A55, the NEX-5, the DSC-F828 and the
/// DSC-R1 record none. The electronic front
/// curtain setting is the MakerNote's on the SLT-A99, the ILCE-7S and every
/// later body. No file gives the shutter used, the image area or the raw
/// image size, which Sony records only in enciphered blocks or, for the
/// raw size, in a code that does not tell APS-C from medium.
const METADATA: &[SampleMetadata] = &[
    // SLT-A55 (raw.pixls.us 964)
    SampleMetadata {
        file: "sony-slt-a55.arw",
        make: Some("SONY"),
        model: Some("SLT-A55V"),
        software: Some("SLT-A55V v1.10"),
        serial_number: None,
        capture_time: Some("2015-08-21T17:35:57"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 125)),
        f_number: Some((100, 10)),
        focal_length: Some((100, 10)),
        focal_length_in_35mm_film: Some(15),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("28-80mm F3.5-5.6"),
            serial_number: false,
            min_focal_length: Some((280, 10)),
            max_focal_length: Some((800, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((4928, 3280)),
        image_size: Some((4928, 3280)),
        focus: None, // no FocusLocation: a point number in CameraInfo only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7RM2 (raw.pixls.us 1310)
    SampleMetadata {
        file: "sony-ilce-7rm2-compressed.arw",
        make: Some("SONY"),
        model: Some("ILCE-7RM2"),
        software: Some("ILCE-7RM2 v3.30"),
        serial_number: None,
        capture_time: Some("2017-04-06T18:44:31"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 6)),
        f_number: Some((130, 10)),
        focal_length: Some((510, 10)),
        focal_length_in_35mm_film: Some(51),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 24-70mm F2.8 GM"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((8000, 5320)),
        image_size: Some((8000, 5320)),
        focus: None, // manual focus
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7RM2 (raw.pixls.us 1311)
    SampleMetadata {
        file: "sony-ilce-7rm2-uncompressed.arw",
        make: Some("SONY"),
        model: Some("ILCE-7RM2"),
        software: Some("ILCE-7RM2 v3.30"),
        serial_number: None,
        capture_time: Some("2017-04-06T18:45:20"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 6)),
        f_number: Some((130, 10)),
        focal_length: Some((510, 10)),
        focal_length_in_35mm_film: Some(51),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 24-70mm F2.8 GM"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((8000, 5320)),
        image_size: Some((8000, 5320)),
        focus: None, // manual focus
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSC-F828 (raw.pixls.us 1351)
    SampleMetadata {
        file: "sony-dsc-f828.srf",
        make: Some("SONY"),
        model: Some("DSC-F828"),
        software: None,
        serial_number: None,
        capture_time: Some("2017-03-27T15:02:34"),
        orientation: Orientation::NoTransforms,
        iso: Some(64),
        exposure_time: Some((10, 1600)),
        f_number: Some((40, 10)),
        focal_length: Some((71, 10)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((3360, 2460)),
        image_size: Some((3360, 2460)),
        focus: None, // no FocusLocation: no focus tag at all
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // NEX-5 (raw.pixls.us 1457)
    SampleMetadata {
        file: "sony-nex-5.arw",
        make: Some("SONY"),
        model: Some("NEX-5"),
        software: Some("NEX-5 Ver.05"),
        serial_number: None,
        capture_time: Some("2015-09-24T09:28:02"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((1, 200)),
        f_number: Some((71, 10)),
        focal_length: Some((180, 10)),
        focal_length_in_35mm_film: Some(27),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("E 18-55mm F3.5-5.6 OSS"),
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((4608, 3072)),
        image_size: Some((4608, 3072)),
        focus: None, // no FocusLocation: a point number in CameraInfo only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-9 (raw.pixls.us 1541)
    SampleMetadata {
        file: "sony-ilce-9-compressed.arw",
        make: Some("SONY"),
        model: Some("ILCE-9"),
        software: Some("ILCE-9 v1.00"),
        serial_number: None,
        capture_time: Some("2017-06-25T16:02:57+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 400)),
        f_number: Some((40, 10)),
        focal_length: Some((550, 10)),
        focal_length_in_35mm_film: Some(55),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 55mm F1.8 ZA"),
            serial_number: false,
            min_focal_length: Some((550, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((6048, 4024)),
        image_size: Some((6000, 4000)),
        focus: Some(SampleFocus {
            frame: (6000, 4000),
            center: (2662, 2000),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-9 (raw.pixls.us 1542)
    SampleMetadata {
        file: "sony-ilce-9-uncompressed.arw",
        make: Some("SONY"),
        model: Some("ILCE-9"),
        software: Some("ILCE-9 v1.00"),
        serial_number: None,
        capture_time: Some("2017-06-25T16:04:05+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 400)),
        f_number: Some((40, 10)),
        focal_length: Some((550, 10)),
        focal_length_in_35mm_film: Some(55),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 55mm F1.8 ZA"),
            serial_number: false,
            min_focal_length: Some((550, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((6048, 4024)),
        image_size: Some((6000, 4000)),
        focus: Some(SampleFocus {
            frame: (6000, 4000),
            center: (2493, 1808),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7S (raw.pixls.us 1582)
    SampleMetadata {
        file: "sony-ilce-7s.arw",
        make: Some("SONY"),
        model: Some("ILCE-7S"),
        software: Some("ILCE-7S v3.20"),
        serial_number: None,
        capture_time: Some("2017-07-13T16:58:28"),
        orientation: Orientation::NoTransforms,
        iso: Some(400),
        exposure_time: Some((1, 250)),
        f_number: None,
        focal_length: None,
        focal_length_in_35mm_film: None,
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((2816, 1872)),
        image_size: Some((2816, 1872)),
        focus: None, // manual focus; no FocusLocation either
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Shade),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSLR-A200 (raw.pixls.us 2271)
    SampleMetadata {
        file: "sony-dslr-a200.arw",
        make: Some("SONY"),
        model: Some("DSLR-A200"),
        software: Some("DSLR-A200 v1.00"),
        serial_number: None,
        capture_time: Some("2017-11-17T22:44:20"),
        orientation: Orientation::NoTransforms,
        iso: Some(800),
        exposure_time: Some((1, 25)),
        f_number: Some((45, 10)),
        focal_length: Some((800, 10)),
        focal_length_in_35mm_film: Some(120),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((3880, 2600)),
        image_size: Some((3880, 2600)),
        focus: None, // no FocusLocation: a point number in CameraInfo only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Action),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSLR-A100 (raw.pixls.us 2346)
    SampleMetadata {
        file: "sony-dslr-a100.arw",
        make: Some("SONY"),
        model: Some("DSLR-A100"),
        software: Some("DSLR-A100 v1.04"),
        serial_number: None,
        capture_time: Some("2018-02-19T17:55:25"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 125)),
        f_number: Some((80, 10)),
        focal_length: Some((600, 10)),
        focal_length_in_35mm_film: Some(90),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: None,
        image_size: None,
        focus: None, // no FocusLocation: only the A100's Minolta block, which is not read
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // DSLR-A700 (raw.pixls.us 2521)
    SampleMetadata {
        file: "sony-dslr-a700.arw",
        make: Some("SONY"),
        model: Some("DSLR-A700"),
        software: Some("DSLR-A700 v04"),
        serial_number: None,
        capture_time: Some("2014-04-18T15:53:21"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((1, 500)),
        f_number: Some((50, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((4288, 2856)),
        image_size: Some((4288, 2856)),
        focus: None, // no FocusLocation: a point number in CameraInfo only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSC-RX100M6 (raw.pixls.us 3012)
    SampleMetadata {
        file: "sony-dsc-rx100m6-1x1.arw",
        make: Some("SONY"),
        model: Some("DSC-RX100M6"),
        software: Some("DSC-RX100M6 v1.00"),
        serial_number: None,
        capture_time: Some("2019-03-06T14:33:05+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(125),
        exposure_time: Some((1, 80)),
        f_number: Some((71, 10)),
        focal_length: Some((5892, 100)),
        focal_length_in_35mm_film: Some(163),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((900, 100)),
            max_focal_length: Some((7200, 100)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((45, 10)),
        }),
        raw_size: Some((5504, 3672)),
        image_size: Some((5472, 3648)),
        focus: Some(SampleFocus {
            frame: (3648, 3648),
            center: (1824, 1824),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::Spot),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSC-RX100M6 (raw.pixls.us 3015)
    SampleMetadata {
        file: "sony-dsc-rx100m6-16x9.arw",
        make: Some("SONY"),
        model: Some("DSC-RX100M6"),
        software: Some("DSC-RX100M6 v1.00"),
        serial_number: None,
        capture_time: Some("2019-03-06T14:32:40+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(125),
        exposure_time: Some((1, 80)),
        f_number: Some((71, 10)),
        focal_length: Some((5892, 100)),
        focal_length_in_35mm_film: Some(163),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((900, 100)),
            max_focal_length: Some((7200, 100)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((45, 10)),
        }),
        raw_size: Some((5504, 3672)),
        image_size: Some((5472, 3648)),
        focus: Some(SampleFocus {
            frame: (5472, 3080),
            center: (2736, 936),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::Spot),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSC-R1 (raw.pixls.us 3221)
    SampleMetadata {
        file: "sony-dsc-r1.sr2",
        make: Some("SONY"),
        model: Some("DSC-R1"),
        software: None,
        serial_number: None,
        capture_time: Some("2019-06-17T18:45:07"),
        orientation: Orientation::NoTransforms,
        iso: Some(160),
        exposure_time: Some((10, 4000)),
        f_number: Some((48, 10)),
        focal_length: Some((715, 10)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((-3, 10)),
        lens: None,
        raw_size: Some((3984, 2608)),
        image_size: Some((3984, 2608)),
        focus: None, // no FocusLocation: no focus tag at all
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // ILCE-7RM3 (raw.pixls.us 3251)
    SampleMetadata {
        file: "sony-ilce-7rm3-pixel-shift.arw",
        make: Some("SONY"),
        model: Some("ILCE-7RM3"),
        software: Some("ILCE-7RM3 v3.00"),
        serial_number: None,
        capture_time: Some("2019-07-12T09:41:02-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 30)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 24mm F1.4 GM"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((240, 10)),
            f_number_at_min_focal_length: Some((14, 10)),
            f_number_at_max_focal_length: Some((14, 10)),
        }),
        raw_size: Some((8000, 5320)),
        image_size: Some((7952, 5304)),
        focus: Some(SampleFocus {
            frame: (7952, 5304),
            center: (3976, 1911),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // SLT-A99 (raw.pixls.us 3751)
    SampleMetadata {
        file: "sony-slt-a99.arw",
        make: Some("SONY"),
        model: Some("SLT-A99V"),
        software: Some("SLT-A99V v1.02"),
        serial_number: None,
        capture_time: Some("2019-10-26T15:58:28"),
        orientation: Orientation::Rotate270,
        iso: Some(100),
        exposure_time: Some((1, 250)),
        f_number: Some((56, 10)),
        focal_length: Some((850, 10)),
        focal_length_in_35mm_film: Some(85),
        exposure_bias: Some((7, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("85mm F1.4 ZA"),
            serial_number: false,
            min_focal_length: Some((850, 10)),
            max_focal_length: Some((850, 10)),
            f_number_at_min_focal_length: Some((14, 10)),
            f_number_at_max_focal_length: Some((14, 10)),
        }),
        raw_size: Some((6048, 4024)),
        image_size: Some((6048, 4024)),
        focus: None, // no FocusLocation: a mask of A-mount points (AFPointsUsed), no position
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::Spot),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Hasselblad Lunar (raw.pixls.us 4051)
    SampleMetadata {
        file: "hasselblad-lunar.arw",
        make: Some("HASSELBLAD"),
        model: Some("Lunar"),
        software: Some("Lunar v1.02"),
        serial_number: None,
        capture_time: Some("2016-05-31T15:21:03"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 125)),
        f_number: Some((56, 10)),
        focal_length: Some((410, 10)),
        focal_length_in_35mm_film: Some(61),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("E 18-55mm F3.5-5.6 OSS"),
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((6048, 4024)),
        image_size: Some((6048, 4024)),
        focus: None, // no FocusLocation: focus data only in an enciphered block
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-1 (raw.pixls.us 4466)
    SampleMetadata {
        file: "sony-ilce-1-lossless.arw",
        make: Some("SONY"),
        model: Some("ILCE-1"),
        software: Some("ILCE-1 v1.00"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2021-04-05T15:29:40.959+00:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(500),
        exposure_time: Some((1, 160)),
        f_number: Some((80, 10)),
        focal_length: Some((1350, 10)),
        focal_length_in_35mm_film: Some(135),
        exposure_bias: Some((10, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 135mm F1.8 GM"),
            serial_number: false,
            min_focal_length: Some((1350, 10)),
            max_focal_length: Some((1350, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((8704, 6144)),
        image_size: Some((8640, 5760)),
        focus: Some(SampleFocus {
            frame: (8640, 5760),
            center: (4320, 2880),
            size: Some((1026, 912)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7SM3 (raw.pixls.us 5418)
    SampleMetadata {
        file: "sony-ilce-7sm3.arw",
        make: Some("SONY"),
        model: Some("ILCE-7SM3"),
        software: Some("ILCE-7SM3 v2.01"),
        serial_number: None,
        capture_time: Some("2022-04-10T17:25:57.888+08:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(80),
        exposure_time: Some((1, 160)),
        f_number: Some((40, 10)),
        focal_length: Some((530, 10)),
        focal_length_in_35mm_film: Some(53),
        exposure_bias: Some((-3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 24-105mm F4 G OSS"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((1050, 10)),
            f_number_at_min_focal_length: Some((40, 10)),
            f_number_at_max_focal_length: Some((40, 10)),
        }),
        raw_size: Some((4288, 2848)),
        image_size: Some((4240, 2832)),
        focus: Some(SampleFocus {
            frame: (4240, 2832),
            center: (2120, 1180),
            size: Some((92, 94)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSC-RX100 (raw.pixls.us 5811)
    SampleMetadata {
        file: "sony-dsc-rx100.arw",
        make: Some("SONY"),
        model: Some("DSC-RX100"),
        software: Some("DSC-RX100 v1.00"),
        serial_number: None,
        capture_time: Some("2012-12-24T12:27:30"),
        orientation: Orientation::NoTransforms,
        iso: Some(125),
        exposure_time: Some((1, 250)),
        f_number: Some((18, 10)),
        focal_length: Some((1040, 100)),
        focal_length_in_35mm_film: Some(28),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((1040, 100)),
            max_focal_length: Some((3710, 100)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((49, 10)),
        }),
        raw_size: Some((5504, 3672)),
        image_size: Some((5504, 3672)),
        focus: None, // no FocusLocation: no focus tag at all
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // DSLR-A900 (raw.pixls.us 5858)
    SampleMetadata {
        file: "sony-dslr-a900.arw",
        make: Some("SONY"),
        model: Some("DSLR-A900"),
        software: Some("DSLR-A900 v2.00"),
        serial_number: None,
        capture_time: Some("2022-08-14T19:36:44"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((1, 80)),
        f_number: Some((40, 10)),
        focal_length: Some((280, 10)),
        focal_length_in_35mm_film: Some(28),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((6080, 4048)),
        image_size: Some((6080, 4048)),
        focus: None, // no FocusLocation: a point number in CameraInfo only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::Spot),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // ILCE-6700 (raw.pixls.us 6736)
    SampleMetadata {
        file: "sony-ilce-6700-lossless.arw",
        make: Some("SONY"),
        model: Some("ILCE-6700"),
        software: Some("ILCE-6700 v1.00"),
        serial_number: None,
        capture_time: Some("2023-08-01T17:00:23.272+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 50)),
        f_number: Some((110, 10)),
        focal_length: Some((160, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("E 16-55mm F2.8 G"),
            serial_number: false,
            min_focal_length: Some((160, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((6656, 4608)),
        image_size: Some((6192, 4128)),
        focus: Some(SampleFocus {
            frame: (6192, 4128),
            center: (3096, 2064),
            size: Some((135, 138)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M4 (raw.pixls.us 6929)
    SampleMetadata {
        file: "sony-ilce-7m4-lossless-l.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M4"),
        software: Some("ILCE-7M4 v2.00"),
        serial_number: None,
        capture_time: Some("2023-10-19T16:13:48.776+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(250),
        exposure_time: Some((1, 50)),
        f_number: Some((56, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(50),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 50mm F2.5 G"),
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((25, 10)),
            f_number_at_max_focal_length: Some((25, 10)),
        }),
        raw_size: Some((7168, 5120)),
        image_size: Some((7008, 4672)),
        focus: Some(SampleFocus {
            frame: (7008, 4672),
            center: (3832, 2102),
            size: Some((350, 351)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M4 (raw.pixls.us 6930)
    SampleMetadata {
        file: "sony-ilce-7m4-lossless-m.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M4"),
        software: Some("ILCE-7M4 v2.00"),
        serial_number: None,
        capture_time: Some("2023-10-19T16:14:16.647+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(250),
        exposure_time: Some((1, 50)),
        f_number: Some((56, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(50),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 50mm F2.5 G"),
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((25, 10)),
            f_number_at_max_focal_length: Some((25, 10)),
        }),
        raw_size: Some((5120, 3584)),
        image_size: Some((4608, 3072)),
        focus: Some(SampleFocus {
            frame: (7008, 4672),
            center: (3482, 2306),
            size: Some((350, 351)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M4 (raw.pixls.us 6931)
    SampleMetadata {
        file: "sony-ilce-7m4-lossless-s.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M4"),
        software: Some("ILCE-7M4 v2.00"),
        serial_number: None,
        capture_time: Some("2023-10-19T16:15:00.101+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(250),
        exposure_time: Some((1, 50)),
        f_number: Some((56, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(50),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 50mm F2.5 G"),
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((25, 10)),
            f_number_at_max_focal_length: Some((25, 10)),
        }),
        raw_size: Some((3584, 2560)),
        image_size: Some((3504, 2336)),
        focus: Some(SampleFocus {
            frame: (7008, 4672),
            center: (3591, 2306),
            size: Some((350, 351)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M4 (raw.pixls.us 6933)
    SampleMetadata {
        file: "sony-ilce-7m4-uncompressed-apsc.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M4"),
        software: Some("ILCE-7M4 v2.00"),
        serial_number: None,
        capture_time: Some("2023-10-19T16:16:39.709+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(640),
        exposure_time: Some((1, 80)),
        f_number: Some((56, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 50mm F2.5 G"),
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((25, 10)),
            f_number_at_max_focal_length: Some((25, 10)),
        }),
        raw_size: Some((4736, 3132)),
        image_size: Some((4608, 3072)),
        focus: Some(SampleFocus {
            frame: (4608, 3072),
            center: (2304, 1529),
            size: Some((230, 231)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M4 (raw.pixls.us 6936)
    SampleMetadata {
        file: "sony-ilce-7m4-compressed-apsc.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M4"),
        software: Some("ILCE-7M4 v2.00"),
        serial_number: None,
        capture_time: Some("2023-10-19T16:18:54.794+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(640),
        exposure_time: Some((1, 80)),
        f_number: Some((56, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 50mm F2.5 G"),
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((25, 10)),
            f_number_at_max_focal_length: Some((25, 10)),
        }),
        raw_size: Some((4736, 3132)),
        image_size: Some((4608, 3072)),
        focus: Some(SampleFocus {
            frame: (4608, 3072),
            center: (2275, 1516),
            size: Some((230, 231)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-9M3 (raw.pixls.us 7361)
    SampleMetadata {
        file: "sony-ilce-9m3-compressed.arw",
        make: Some("SONY"),
        model: Some("ILCE-9M3"),
        software: Some("ILCE-9M3 v1.00"),
        serial_number: None,
        capture_time: Some("2024-03-26T12:47:24.654+00:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(1000),
        exposure_time: Some((1, 50)),
        f_number: Some((18, 10)),
        focal_length: Some((550, 10)),
        focal_length_in_35mm_film: Some(55),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 55mm F1.8 ZA"),
            serial_number: false,
            min_focal_length: Some((550, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((6048, 4020)),
        image_size: Some((6000, 4000)),
        focus: None, // manual focus
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Custom),
            color_temperature: Some(5500),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-9M3 (raw.pixls.us 7364)
    SampleMetadata {
        file: "sony-ilce-9m3-lossless-s.arw",
        make: Some("SONY"),
        model: Some("ILCE-9M3"),
        software: Some("ILCE-9M3 v1.00"),
        serial_number: None,
        capture_time: Some("2024-03-26T12:47:59.397+00:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(1000),
        exposure_time: Some((1, 50)),
        f_number: Some((18, 10)),
        focal_length: Some((550, 10)),
        focal_length_in_35mm_film: Some(55),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 55mm F1.8 ZA"),
            serial_number: false,
            min_focal_length: Some((550, 10)),
            max_focal_length: Some((550, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((3072, 2048)),
        image_size: Some((3008, 2000)),
        focus: None, // manual focus
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Custom),
            color_temperature: Some(5500),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-1M2 (raw.pixls.us 7835)
    SampleMetadata {
        file: "sony-ilce-1m2-lossy-apsc.arw",
        make: Some("SONY"),
        model: Some("ILCE-1M2"),
        software: Some("ILCE-1M2 v2.00"),
        serial_number: None,
        capture_time: Some("2025-05-09T09:36:24.457+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 1000)),
        f_number: Some((28, 10)),
        focal_length: Some((1330, 10)),
        focal_length_in_35mm_film: Some(199),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 70-200mm F2.8 GM OSS II"),
            serial_number: false,
            min_focal_length: Some((700, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((5664, 3768)),
        image_size: Some((5616, 3744)),
        focus: Some(SampleFocus {
            frame: (5616, 3744),
            center: (2808, 1872),
            size: Some((1386, 998)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(false),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M5 (raw.pixls.us 8846)
    SampleMetadata {
        file: "sony-ilce-7m5-compressed-hq.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M5"),
        software: Some("ILCE-7M5 v1.00"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2025-12-17T16:05:02.978+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 8)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("24-70mm F2.8 DG DN II | Art 024"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((7040, 4688)),
        image_size: Some((7008, 4672)),
        focus: Some(SampleFocus {
            frame: (7008, 4672),
            center: (3504, 2336),
            size: Some((745, 594)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M5 (raw.pixls.us 8847)
    SampleMetadata {
        file: "sony-ilce-7m5-compressed.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M5"),
        software: Some("ILCE-7M5 v1.00"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2025-12-17T16:05:52.741+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 8)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("24-70mm F2.8 DG DN II | Art 024"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((7040, 4688)),
        image_size: Some((7008, 4672)),
        focus: Some(SampleFocus {
            frame: (7008, 4672),
            center: (3504, 2336),
            size: Some((745, 594)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M5 (raw.pixls.us 8848)
    SampleMetadata {
        file: "sony-ilce-7m5-lossless-apsc.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M5"),
        software: Some("ILCE-7M5 v1.00"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2025-12-17T16:06:25.008+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 6)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(36),
        exposure_bias: Some((3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("24-70mm F2.8 DG DN II | Art 024"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((5120, 3584)),
        image_size: Some((4608, 3072)),
        focus: Some(SampleFocus {
            frame: (4608, 3072),
            center: (2304, 1536),
            size: Some((490, 390)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M5 (raw.pixls.us 8849)
    SampleMetadata {
        file: "sony-ilce-7m5-compressed-hq-apsc.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M5"),
        software: Some("ILCE-7M5 v1.00"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2025-12-17T16:06:35.524+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 8)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(36),
        exposure_bias: Some((3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("24-70mm F2.8 DG DN II | Art 024"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((4640, 3088)),
        image_size: Some((4608, 3072)),
        focus: Some(SampleFocus {
            frame: (4608, 3072),
            center: (2304, 1536),
            size: Some((490, 390)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // ILCE-7M5 (raw.pixls.us 8850)
    SampleMetadata {
        file: "sony-ilce-7m5-compressed-apsc.arw",
        make: Some("SONY"),
        model: Some("ILCE-7M5"),
        software: Some("ILCE-7M5 v1.00"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2025-12-17T16:06:47.125+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 8)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(36),
        exposure_bias: Some((3, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("24-70mm F2.8 DG DN II | Art 024"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((4640, 3088)),
        image_size: Some((4608, 3072)),
        focus: Some(SampleFocus {
            frame: (4608, 3072),
            center: (2304, 1536),
            size: Some((490, 390)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
];

/// The metadata the ARQ of the list's large tier must give, fetched only
/// on request: the same as that of the pixel-shift frame 3251.
const LARGE_METADATA: &[SampleMetadata] = &[
    // ILCE-7RM3 (raw.pixls.us 3255), large tier
    SampleMetadata {
        file: "sony-ilce-7rm3.arq",
        make: Some("SONY"),
        model: Some("ILCE-7RM3"),
        software: Some("ILCE-7RM3 v3.00"),
        serial_number: None,
        capture_time: Some("2019-07-12T09:41:02-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((1, 30)),
        f_number: Some((28, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((0, 10)),
        lens: Some(SampleLens {
            make: None,
            model: Some("FE 24mm F1.4 GM"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((240, 10)),
            f_number_at_min_focal_length: Some((14, 10)),
            f_number_at_max_focal_length: Some((14, 10)),
        }),
        raw_size: Some((8000, 5320)),
        image_size: Some((7952, 5304)),
        focus: Some(SampleFocus {
            frame: (7952, 5304),
            center: (3976, 1911),
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None,
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: Some(true),
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
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
fn every_sony_sample_gives_its_metadata() {
    for expected in METADATA {
        check_metadata(expected, Format::Arw);
    }
}

#[test]
fn the_arq_sample_gives_the_metadata_of_its_pixel_shift_frame() {
    for expected in LARGE_METADATA {
        check_metadata(expected, Format::Arw);
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
fn the_sample_list_and_the_tables_of_previews_and_metadata_name_the_same_sony_files() {
    // The list is compiled in, so this runs without any download.
    let listed = listed("arw");

    let previews: Vec<(&str, &str)> = SAMPLES
        .iter()
        .map(|sample| (sample.file, "normal"))
        .chain(LARGE_SAMPLES.iter().map(|sample| (sample.file, "large")))
        .collect();
    let metadata: Vec<(&str, &str)> = METADATA
        .iter()
        .map(|expected| (expected.file, "normal"))
        .chain(
            LARGE_METADATA
                .iter()
                .map(|expected| (expected.file, "large")),
        )
        .collect();
    for (what, tables) in [("previews", previews), ("metadata", metadata)] {
        let expected: BTreeSet<(&str, &str)> = tables.iter().copied().collect();
        assert_eq!(
            expected.len(),
            tables.len(),
            "a file is in the tables of {what} twice"
        );
        let unexpected: Vec<_> = listed.difference(&expected).collect();
        let unlisted: Vec<_> = expected.difference(&listed).collect();
        assert!(
            unexpected.is_empty() && unlisted.is_empty(),
            "listed without expected {what}: {unexpected:?}; \
             expected {what} but not listed: {unlisted:?}"
        );
    }
}
