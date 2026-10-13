// SPDX-License-Identifier: MIT OR Apache-2.0

//! The NEF reader from outside the crate, on a file laid out like a Nikon
//! Z 9's: what it opens as, which previews it lists, and what reading one
//! returns. Then the same on real Z 9 files, when they have been downloaded
//! (see [`sample`]).

mod support;

use std::env;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use raw_files::{Format, Orientation, Preview, PreviewFormat, RawFile};
use support::nef::{
    FULL_SIZE_JPEG, IFD0_JPEG, MEDIUM_JPEG, PREVIEW_JPEG, RAW_STRIP_START, Z9Blobs, stub, z9_file,
    z9_file_with, z9_file_with_blobs,
};
use support::tiff_builder::{Built, Order};

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

// Real files. The Z 9 files of raw.pixls.us, one per compression mode, are
// listed with their hashes in `tests/samples.tsv`; `scripts/fetch-samples.sh`
// downloads them. They are too large to commit, so a test whose file is
// absent says so and passes: the suite runs offline.

/// The path of the sample file `name`, or `None`, with a note on stderr,
/// when it has not been downloaded. The files are in `RAW_FILES_SAMPLES`
/// (relative to the current directory, which under `cargo test` is the
/// crate's root), or in `target/raw-samples` when it is unset, where
/// `scripts/fetch-samples.sh` puts them.
fn sample(name: &str) -> Option<PathBuf> {
    let dir = match env::var_os("RAW_FILES_SAMPLES") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("target/raw-samples"),
    };
    let path = dir.join(name);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!(
            "skipped: sample file {} is missing; scripts/fetch-samples.sh downloads it",
            path.display()
        );
        None
    }
}

/// The four previews each Z 9 sample holds, smallest first, by their size
/// in pixels: the IFD0 thumbnail, the MakerNote's preview, the medium JPEG in
/// SubIFD2 and the full-size JPEG in SubIFD0.
const Z9_PREVIEW_SIZES: [(u32, u32); 4] = [(160, 120), (640, 424), (1620, 1080), (8256, 5504)];

/// A sample Z 9 file and where its four previews are, in the order of
/// [`Z9_PREVIEW_SIZES`]: the byte offset of each and its length.
struct Z9Sample {
    file: &'static str,
    previews: [(u64, u64); 4],
}

/// 14-bit lossless compression (raw.pixls.us 5146).
const Z9_LOSSLESS: Z9Sample = Z9Sample {
    file: "nikon-z9-lossless.nef",
    previews: [
        (222_540, 9_354),
        (99_432, 122_857),
        (332_288, 1_075_885),
        (1_408_512, 6_492_228),
    ],
};

/// High Efficiency* (raw.pixls.us 5147).
const Z9_HE_STAR: Z9Sample = Z9Sample {
    file: "nikon-z9-he-star.nef",
    previews: [
        (229_076, 9_823),
        (99_372, 129_452),
        (338_944, 927_453),
        (1_266_688, 5_760_370),
    ],
};

/// High Efficiency (raw.pixls.us 5148).
const Z9_HE: Z9Sample = Z9Sample {
    file: "nikon-z9-he.nef",
    previews: [
        (219_300, 10_165),
        (99_372, 119_678),
        (329_728, 1_081_068),
        (1_411_072, 6_288_717),
    ],
};

/// Checks that `sample` opens as a NEF with its four JPEGs, smallest first,
/// each where the file has it and upright, and that the largest reads as a
/// whole JPEG stream of its declared length. Does nothing when the file has
/// not been downloaded.
fn assert_the_z9_sample(sample_file: &Z9Sample) {
    let Some(path) = sample(sample_file.file) else {
        return;
    };
    let label = sample_file.file;
    let mut file = RawFile::open(&path).unwrap();
    assert_eq!(file.format(), Format::Nef, "{label}");
    assert_eq!(sizes(file.previews()), Z9_PREVIEW_SIZES, "{label}");
    for (preview, (offset, len)) in file.previews().iter().zip(sample_file.previews) {
        let size = (preview.width(), preview.height());
        assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label} {size:?}");
        assert_eq!(
            preview.orientation(),
            Orientation::NoTransforms,
            "{label} {size:?}"
        );
        assert_eq!(preview.byte_range().start, offset, "{label} {size:?}");
        assert_eq!(preview.byte_len(), len, "{label} {size:?}");
    }

    let largest = file.largest_preview().unwrap();
    assert_eq!((largest.width(), largest.height()), (8256, 5504), "{label}");
    let bytes = file.read_preview(&largest).unwrap();
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        largest.byte_len(),
        "{label}"
    );
    assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF], "{label}");
    assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9], "{label}");
}

#[test]
fn the_lossless_z9_sample_opens_with_its_four_previews() {
    assert_the_z9_sample(&Z9_LOSSLESS);
}

#[test]
fn the_he_star_z9_sample_opens_with_its_four_previews() {
    assert_the_z9_sample(&Z9_HE_STAR);
}

#[test]
fn the_he_z9_sample_opens_with_its_four_previews() {
    assert_the_z9_sample(&Z9_HE);
}
