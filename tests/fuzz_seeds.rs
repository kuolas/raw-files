// SPDX-License-Identifier: MIT OR Apache-2.0

//! The seeds of the `open` and `jpeg_segments` fuzz targets: the
//! integration tests' fixtures of the RAW formats, committed as files in
//! `fuzz/seeds/open/`, and their JPEG files, committed as files in
//! `fuzz/seeds/jpeg_segments/`.
//!
//! A fuzzer reaches the Nikon or Sony reader only by building a layout that
//! the format detection accepts, which it does by luck. So the `open`
//! target starts from files laid out like the NEF of a Nikon Z 9, of a
//! Z6III on a 1:1 crop, whose MakerNote records the crop, and of a D300,
//! whose MakerNote records what the Z 9's standard tags hold, and one like
//! each generation of Sony's family, in both byte orders; and from the two
//! real files committed in `tests/fixtures/`, shots of the maintainer's own
//! Z 9 in DX and 1:1 crop modes with their pixels and personal fields
//! removed, copied as they are, in their own little-endian byte order.
//!
//! Likewise, a fuzzer builds an Exif segment that holds a TIFF structure,
//! or an MPF segment that holds an index, only by luck. So the
//! `jpeg_segments` target starts from JPEG files: one with no Exif segment,
//! one whose Exif segment holds IFD0 and IFD1's thumbnail, a camera's with
//! every value the metadata reads in its Exif segment and an MPF index of
//! the images after its main one, the same camera layout with a JFIF
//! segment first, and one with zeros after its end, the Exif segment's TIFF
//! structure and the MPF index in both byte orders. The `open` target takes
//! these seeds too, as JPEG files are something it must open.
//!
//! The fixtures live in `tests/`, outside the library, so this test writes
//! and checks the seed files, as the unit tests of the TIFF reader do for
//! theirs. [`open_seeds`] and [`jpeg_segments_seeds`] list them; the
//! ignored test `write_fuzz_seeds` regenerates the files after a fixture
//! changes, and `the_open_fuzz_seeds_match_the_fixtures` and
//! `the_jpeg_segments_fuzz_seeds_match_the_fixtures` fail when a file is
//! missing or has gone stale.

mod support;

use std::path::{Path, PathBuf};

use support::arw::{
    a100_file, arq_file, arw2_file, arw4_file, arw4_medium_file, sr2_file, srf_file,
};
use support::fixtures::{Z9_1X1, Z9_DX, read_fixture};
use support::jpeg_files::{
    camera_jpeg, camera_jpeg_with_metadata, exif_jpeg, padded_jpeg, plain_jpeg,
};
use support::nef::{d300_file, z6iii_file, z9_file};
use support::tiff_builder::Order;

/// A layout built in code, once: the stem and extension of its file names,
/// and a builder for the fixture in a given byte order.
type Layout = (&'static str, &'static str, fn(Order) -> Vec<u8>);

/// Each of `layouts` in both byte orders: each file name, the stem, `le`
/// or `be` and the extension, with the bytes of the fixture it holds.
fn in_both_orders(layouts: &[Layout]) -> Vec<(String, Vec<u8>)> {
    let mut seeds = Vec::new();
    for order in [Order::Little, Order::Big] {
        let suffix = match order {
            Order::Little => "le",
            Order::Big => "be",
        };
        for (stem, extension, build) in layouts {
            seeds.push((format!("{stem}-{suffix}.{extension}"), build(order)));
        }
    }
    seeds
}

/// The RAW fixtures as starting inputs for the `open` target: each file
/// name with the bytes of the fixture it holds, in both byte orders for
/// those built in code, and once for the two stripped Z 9 files.
fn open_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = in_both_orders(&[
        ("nikon-z9", "nef", |order| z9_file(order, 1).bytes),
        ("nikon-z6iii-1x1", "nef", |order| z6iii_file(order, 1).bytes),
        ("nikon-d300", "nef", |order| d300_file(order, 1).bytes),
        ("sony-arw4", "arw", |order| arw4_file(order, 1).0.bytes),
        ("sony-arw4-medium", "arw", |order| {
            arw4_medium_file(order).0.bytes
        }),
        ("sony-arw2", "arw", |order| arw2_file(order, 1).0.bytes),
        ("sony-arq", "arq", |order| arq_file(order).0.bytes),
        ("sony-sr2", "sr2", |order| sr2_file(order).0.bytes),
        ("sony-srf", "srf", |order| srf_file(order).0.bytes),
        ("sony-a100", "arw", |order| a100_file(order).0.bytes),
    ]);
    // Real files, so in one byte order only, and under the names they are
    // committed by.
    for name in [Z9_DX, Z9_1X1] {
        seeds.push((name.to_owned(), read_fixture(name)));
    }
    seeds
}

/// The JPEG files as starting inputs for the `jpeg_segments` target, and
/// for the `open` target too: each file name with the bytes of the file it
/// holds, in both byte orders for those with a TIFF structure and an MPF
/// index, and once for those with neither.
fn jpeg_segments_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = in_both_orders(&[
        ("exif", "jpg", |order| exif_jpeg(order).0.bytes),
        ("camera", "jpg", |order| {
            camera_jpeg_with_metadata(order).0.bytes
        }),
        ("camera-jfif-first", "jpg", |order| {
            camera_jpeg(order, true).0.bytes
        }),
    ]);
    seeds.push(("plain.jpg".to_owned(), plain_jpeg().bytes));
    seeds.push(("padded.jpg".to_owned(), padded_jpeg().bytes));
    seeds
}

/// Where the seed files of the fuzz target `target` live, under the
/// crate's root.
fn seeds_dir(target: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz/seeds")
        .join(target)
}

/// Writes the fixtures to `fuzz/seeds/open/` and the JPEG files to
/// `fuzz/seeds/jpeg_segments/`, where the fuzz targets start from them.
/// Ignored, as it writes to the source tree; run it after changing a
/// fixture: `cargo test write_fuzz_seeds -- --ignored`, which also writes
/// the TIFF reader's seeds and the JPEG search's.
#[test]
#[ignore = "writes the seed files; run it to regenerate them"]
fn write_fuzz_seeds() {
    for (target, seeds) in [
        ("open", open_seeds()),
        ("jpeg_segments", jpeg_segments_seeds()),
    ] {
        let dir = seeds_dir(target);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, bytes) in seeds {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
    }
}

/// Checks that each of `seeds` is committed in the seeds directory of the
/// fuzz target `target` and holds the bytes of its fixture.
///
/// Unlike the library's two seed tests, this has no skip. Those ship in the
/// package, which leaves out `fuzz/`, so they skip when it is absent; this
/// file is not packaged, so it only ever runs in the repository, where a
/// missing `fuzz/` or seeds directory is a failure to report.
fn assert_committed(target: &str, seeds: Vec<(String, Vec<u8>)>) {
    let dir = seeds_dir(target);
    assert!(
        dir.is_dir(),
        "{} does not exist: the seed files are committed there",
        dir.display()
    );
    for (name, bytes) in seeds {
        let path = dir.join(&name);
        let committed = std::fs::read(&path).ok();
        assert!(
            committed.as_deref() == Some(&bytes[..]),
            "fuzz/seeds/{target}/{name} is missing or differs from its fixture: \
             regenerate it with `cargo test write_fuzz_seeds -- --ignored`"
        );
    }
}

/// The committed seed files of the `open` target hold the fixtures' bytes,
/// so that a changed fixture cannot leave the fuzzer starting from a stale
/// file.
#[test]
fn the_open_fuzz_seeds_match_the_fixtures() {
    assert_committed("open", open_seeds());
}

/// The committed seed files of the `jpeg_segments` target hold the JPEG
/// files' bytes, so that a changed file cannot leave the fuzzer starting
/// from a stale one.
#[test]
fn the_jpeg_segments_fuzz_seeds_match_the_fixtures() {
    assert_committed("jpeg_segments", jpeg_segments_seeds());
}
