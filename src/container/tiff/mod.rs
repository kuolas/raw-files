// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF container, as the TIFF-based RAW formats use it.
//!
//! Reading a RAW format built on TIFF starts with the [`header`],
//! which sets the byte order of the file and says where its first directory,
//! IFD0, starts. Both byte orders are accepted: files in the wild use both.
//!
//! This module is the container's face to the rest of the crate: it
//! re-exports what the format modules use, and the split into files below
//! stays private to this folder.

mod header;

// Nothing outside this folder reads TIFF files yet. Each `expect` fails the
// build once its item has a user, so it is then removed rather than left
// behind.
#[expect(unused_imports, reason = "nothing walks the IFDs yet")]
pub(crate) use header::Header;
#[expect(unused_imports, reason = "nothing opens files yet")]
pub(crate) use header::parse_header;
