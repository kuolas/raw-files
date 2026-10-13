// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF container, as the TIFF-based RAW formats use it.
//!
//! Reading a RAW format built on TIFF starts with the [`header`],
//! which sets the byte order of the file and says where its first directory,
//! IFD0, starts. Both byte orders are accepted: files in the wild use both.
//! The directories the header leads to, IFD0 and on, are read by the
//! [`ifd`] module, and found by the [`mod@walk`] module: IFD0's chain of
//! next IFDs, their SubIFDs and IFD0's Exif IFD. The [`tags`] module names
//! the tags the crate reads, and the [`types`] module the field types of
//! their values.
//!
//! This module is the container's face to the rest of the crate: it
//! re-exports what the format modules use, and the split into files below
//! stays private to this folder, but for `tags` and `types`, whose
//! constants the format modules name as `tiff::tags::ORIENTATION` and
//! `tiff::types::UNDEFINED`. Besides the readers, it
//! re-exports the walk's rule for damage, [`skip_damage`], so that what
//! reads the tree after the walk loses to damage only what the damage
//! touches, as the walk does. One file holds only tests:
//! `robustness`, which reads truncated and corrupted files through the
//! whole reader.

mod header;
mod ifd;
#[cfg(test)]
mod robustness;
pub(crate) mod tags;
pub(crate) mod types;
mod walk;

pub(crate) use header::{Header, parse_header, parse_header_at};
pub(crate) use ifd::{Entry, Ifd, parse_ifd};
pub(crate) use walk::{Location, Tree, skip_damage, walk};
