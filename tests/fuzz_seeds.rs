// SPDX-License-Identifier: MIT OR Apache-2.0

//! The seeds of the `open` fuzz target: the integration tests' fixtures of
//! the RAW formats, committed as files in `fuzz/seeds/open/`.
//!
//! A fuzzer reaches the Nikon or Sony reader only by building a layout that
//! the format detection accepts, which it does by luck. So the target
//! starts from one file laid out like a Nikon Z 9's NEF and one like each
//! generation of Sony's family, in both byte orders. The fixtures live in
//! `tests/support/`, outside the library, so this test writes and checks
//! the seed files, as the unit tests of the TIFF reader do for theirs.
//! [`seeds`] lists them; the ignored test `write_fuzz_seeds` regenerates the
//! files after a fixture changes, and `the_open_fuzz_seeds_match_the_fixtures`
//! fails when a file is missing or has gone stale.

mod support;

use std::path::{Path, PathBuf};

use support::arw::{
    a100_file, arq_file, arw2_file, arw4_file, arw4_medium_file, sr2_file, srf_file,
};
use support::nef::z9_file;
use support::tiff_builder::Order;

/// The fixtures as starting inputs for the fuzzer: each file name with the
/// bytes of the fixture it holds, in both byte orders.
fn seeds() -> Vec<(String, Vec<u8>)> {
    // Each layout once: the stem and extension of its file names, and a
    // builder for the fixture in a given byte order.
    type Layout = (&'static str, &'static str, fn(Order) -> Vec<u8>);
    let layouts: [Layout; 8] = [
        ("nikon-z9", "nef", |order| z9_file(order, 1).bytes),
        ("sony-arw4", "arw", |order| arw4_file(order, 1).0.bytes),
        ("sony-arw4-medium", "arw", |order| {
            arw4_medium_file(order).0.bytes
        }),
        ("sony-arw2", "arw", |order| arw2_file(order, 1).0.bytes),
        ("sony-arq", "arq", |order| arq_file(order).0.bytes),
        ("sony-sr2", "sr2", |order| sr2_file(order).0.bytes),
        ("sony-srf", "srf", |order| srf_file(order).0.bytes),
        ("sony-a100", "arw", |order| a100_file(order).0.bytes),
    ];
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

/// Where the seed files live, under the crate's root.
fn seeds_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/open")
}

/// Writes the fixtures to `fuzz/seeds/open/`, where the fuzz target starts
/// from them. Ignored, as it writes to the source tree; run it after
/// changing a fixture: `cargo test write_fuzz_seeds -- --ignored`, which
/// also writes the TIFF reader's seeds and the JPEG search's.
#[test]
#[ignore = "writes the seed files; run it to regenerate them"]
fn write_fuzz_seeds() {
    let dir = seeds_dir();
    std::fs::create_dir_all(&dir).unwrap();
    for (name, bytes) in seeds() {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
}

/// The committed seed files hold the fixtures' bytes, so that a changed
/// fixture cannot leave the fuzzer starting from a stale file.
///
/// Unlike the library's two seed tests, this one has no skip. Those ship in
/// the package, which leaves out `fuzz/`, so they skip when it is absent;
/// this file is not packaged, so it only ever runs in the repository, where
/// a missing `fuzz/` or seeds directory is a failure to report.
#[test]
fn the_open_fuzz_seeds_match_the_fixtures() {
    let dir = seeds_dir();
    assert!(
        dir.is_dir(),
        "{} does not exist: the seed files are committed there",
        dir.display()
    );
    for (name, bytes) in seeds() {
        let path = dir.join(&name);
        let committed = std::fs::read(&path).ok();
        assert!(
            committed.as_deref() == Some(&bytes[..]),
            "fuzz/seeds/open/{name} is missing or differs from its fixture: \
             regenerate it with `cargo test write_fuzz_seeds -- --ignored`"
        );
    }
}
