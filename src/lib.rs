// SPDX-License-Identifier: MIT OR Apache-2.0

//! raw-files: a camera RAW library in pure Rust.
//!
//! raw-files will extract the JPEG previews embedded in Nikon NEF and Sony ARW
//! files, and later decode their raw sensor data. A malformed file will give
//! an error, never a panic.
//!
//! [`RawFile`] opens a file by path, from bytes in memory or from any reader
//! that can seek, detects its format from its content, lists its embedded
//! previews and reads one; [`Options`] opens it with other [`Limits`].
//! Nikon NEF files are recognised, though their previews are not read yet;
//! any other file that can be read opens as `UnknownFormat`. See the
//! [roadmap](https://github.com/kuolas/raw-files/blob/master/docs/ROADMAP.md)
//! for the plan and the
//! [architecture](https://github.com/kuolas/raw-files/blob/master/docs/ARCHITECTURE.md)
//! for how the code is laid out.

// A fuzzing build compiles the reader for one caller, the fuzz entry points,
// which use only part of it. What is unused there, and so which `expect`s
// go unfulfilled, says nothing about the library: the normal and test
// builds keep both lints, and CI checks those.
#![cfg_attr(fuzzing, allow(dead_code, unfulfilled_lint_expectations))]

mod container;
mod error;
mod file;
mod format;
mod formats;
// The entry points of the fuzz targets in `fuzz/`: compiled only with
// `--cfg fuzzing`, which cargo-fuzz sets, and hidden from the docs.
#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzzing;
mod geometry;
mod io;
// The stub JPEG lives with the integration tests and is shared with the
// unit tests, as the TIFF fixture builder below is.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/jpeg.rs"]
mod jpeg_stub;
mod limits;
mod options;
mod orientation;
mod preview;
// The TIFF fixture builder lives with the integration tests and is shared
// with the unit tests: compiled into each test build, never into the
// library. Each side uses only part of it.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/tiff_builder.rs"]
mod tiff_builder;

pub use error::{Error, ErrorKind, Result};
pub use file::RawFile;
pub use format::Format;
pub use geometry::Size;
pub use limits::Limits;
pub use options::Options;
pub use orientation::Orientation;
pub use preview::{Preview, PreviewFormat};
