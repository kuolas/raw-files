// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF container, as the TIFF-based RAW formats use it.
//!
//! A TIFF structure is read inside a [`Block`]: where its offsets count
//! from, and the range of the file it may use. Its parts, in the order a
//! file is read:
//!
//! - [`header`] reads the TIFF header at the block's base: the byte order
//!   of the structure, and where IFD0 starts;
//! - [`mod@walk`] finds the directories: IFD0, and the IFDs it leads to;
//! - [`ifd`] reads one directory, and a value of it by tag or by rule;
//! - [`rule`] declares the type and count a tag must have to be read;
//! - [`tags`] names the tags the crate reads, and [`field_type`] the types
//!   of their values.
//!
//! This module is the container's face to the rest of the crate. It
//! re-exports what the format modules use, and its files stay private but
//! for `tags`, whose constants the format modules name as
//! `tiff::tags::ORIENTATION`. One file holds only tests: `robustness`,
//! which reads truncated and corrupted files through the whole reader.

mod block;
mod field_type;
mod header;
mod ifd;
#[cfg(test)]
mod robustness;
mod rule;
pub(crate) mod tags;
mod walk;

pub(crate) use block::Block;
pub use field_type::FieldType;
pub(crate) use header::parse_header;
pub(crate) use ifd::{Entry, Ifd, ifd_in, parse_ifd};
pub(crate) use rule::{First, Held, Rule, Text};
pub(crate) use walk::{Link, Tree, walk};
