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
