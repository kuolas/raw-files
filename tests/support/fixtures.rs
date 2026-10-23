// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the integration tests find the files committed in
//! `tests/fixtures/`: two NEF files from the maintainer's own Z 9, in DX
//! and 1:1 crop modes, with their pixels and personal fields removed and
//! their structure and values kept, and three JPEG files from the same
//! camera, in FX and in DX and 16:9 crop modes, with their images replaced
//! by stubs and their personal fields zeroed and their Exif segment kept
//! as the camera wrote it, as `tests/fixtures/README.md` says.
//! Unlike the downloaded samples, they are in the repository, so a test
//! never skips one: a missing fixture is a failure.

use std::path::{Path, PathBuf};

/// The Z 9 file shot in DX crop mode, whose focus point the reader scales
/// onto the cropped picture.
pub const Z9_DX: &str = "nikon-z9-dx.nef";

/// The Z 9 file shot in 1:1 crop mode, whose focus point the reader moves
/// into the centred window the cropped picture is.
pub const Z9_1X1: &str = "nikon-z9-1x1.nef";

/// The Z 9 JPEG shot in FX, the camera's full output, at full size, whose
/// focus point the reader gives as recorded.
pub const Z9_FX_JPEG: &str = "nikon-z9-fx.jpg";

/// The Z 9 JPEG shot in DX crop mode at full size, whose focus point the
/// reader scales onto the JPEG's picture.
pub const Z9_DX_JPEG: &str = "nikon-z9-dx.jpg";

/// The Z 9 JPEG shot in 16:9 crop mode at the middle size, in portrait
/// orientation, whose focus point the reader moves into the centred window
/// of the recorded frame that has the JPEG's shape.
pub const Z9_16X9_JPEG: &str = "nikon-z9-16x9.jpg";

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
