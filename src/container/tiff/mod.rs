// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF container, as the TIFF-based RAW formats use it.
//!
//! Reading a RAW format built on TIFF starts with the [`header`],
//! which sets the byte order of the file and says where its first directory,
//! IFD0, starts. Both byte orders are accepted: files in the wild use both.
//! The directories the header leads to, IFD0 and on, are read by the
//! [`ifd`] module, and found by the [`mod@walk`] module: IFD0's chain of
//! next IFDs, their SubIFDs and IFD0's Exif IFD.
//!
//! This module is the container's face to the rest of the crate: it
//! re-exports what the format modules use, and the split into files below
//! stays private to this folder. One file holds only tests: `robustness`,
//! which reads truncated and corrupted files through the whole reader.

mod header;
mod ifd;
#[cfg(test)]
mod robustness;
mod walk;

// No format reads entries or locations yet. Each `expect` fails the build
// once its item has a user, so it is then removed rather than left behind.
pub(crate) use header::{Header, parse_header};
#[expect(
    unused_imports,
    reason = "nothing reads entries outside this folder yet"
)]
pub(crate) use ifd::Entry;
pub(crate) use ifd::{Ifd, parse_ifd};
#[expect(
    unused_imports,
    reason = "nothing reads locations outside this folder yet"
)]
pub(crate) use walk::Location;
pub(crate) use walk::{Tree, walk};
