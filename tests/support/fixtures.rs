// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the integration tests find the files committed in
//! `tests/fixtures/`: two NEF files from the maintainer's own Z 9, in DX
//! and 1:1 crop modes, with their pixels and personal fields removed and
//! their structure and values kept, as `tests/fixtures/README.md` says.
//! Unlike the downloaded samples, they are in the repository, so a test
//! never skips one: a missing fixture is a failure.

use std::path::{Path, PathBuf};

/// The Z 9 file shot in DX crop mode, whose focus point the reader scales
/// onto the cropped picture.
pub const Z9_DX: &str = "nikon-z9-dx.nef";

/// The Z 9 file shot in 1:1 crop mode, whose focus point the reader moves
/// into the centred window the cropped picture is.
pub const Z9_1X1: &str = "nikon-z9-1x1.nef";

/// The path of the committed fixture `name`, in `tests/fixtures/` under
/// the crate's root.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The bytes of the committed fixture `name`.
///
/// # Panics
///
/// When the file cannot be read: it is committed, so that is a failure.
pub fn read_fixture(name: &str) -> Vec<u8> {
    let path = fixture(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}
