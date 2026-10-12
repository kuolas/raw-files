// SPDX-License-Identifier: MIT OR Apache-2.0

//! raw-files: a camera RAW library in pure Rust.
//!
//! raw-files will extract the JPEG previews embedded in Nikon NEF and Sony ARW
//! files, and later decode their raw sensor data. A malformed file will give
//! an error, never a panic. Nothing is implemented yet: see the
//! [roadmap](https://github.com/kuolas/raw-files/blob/master/docs/ROADMAP.md)
//! for the plan and the
//! [architecture](https://github.com/kuolas/raw-files/blob/master/docs/ARCHITECTURE.md)
//! for how the code is laid out.

// Nothing uses the containers yet, and they read through `io`: compiled only
// for tests until the file handle uses them, for the same reason as `io`.
#[cfg(test)]
mod container;
mod error;
// Nothing reads files yet; tests already do. Compiled only for tests until the
// first caller arrives: an `expect(dead_code)` on the module would mark the
// error constructors it calls as used on some compilers and leave their own
// expectations unfulfilled.
#[cfg(test)]
mod io;
mod limits;
// The TIFF fixture builder lives with the integration tests and is shared
// with the unit tests: compiled into each test build, never into the
// library. Each side uses only part of it.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/tiff_builder.rs"]
mod tiff_builder;

pub use error::{Error, ErrorKind, Result};
pub use limits::Limits;
