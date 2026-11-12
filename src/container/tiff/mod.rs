// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF container, as the TIFF-based RAW formats use it.
//!
//! A TIFF structure is read inside a [`block`]: the position its offsets
//! count from and the range of the file it may use, the whole file for a
//! TIFF file's own structure. Reading it starts with the [`header`], at the
//! block's base, which sets the byte order of the structure and says where
//! its first directory, IFD0, starts. Both byte orders are accepted: files
//! in the wild use both. The directories the header leads to, IFD0 and on,
//! are read by the [`ifd`] module, and found by the [`mod@walk`] module:
//! IFD0's chain of next IFDs, their SubIFDs, IFD0's Exif and GPS IFDs and
//! the Exif IFD's Interoperability IFD. The [`tags`] module names the tags
//! the crate reads, and the [`field_type`] module the field types of their
//! values. The [`rule`] module declares the type and count each tag must
//! have to be read, and the IFD reads a value by its rule.
//!
//! This module is the container's face to the rest of the crate: it
//! re-exports what the format modules use, and the split into files below
//! stays private to this folder, but for `tags`, whose constants the
//! format modules name as `tiff::tags::ORIENTATION`. Besides the readers,
//! it re-exports the rules, [`Rule`], [`Held`] and their shapes [`First`]
//! and [`Text`], and [`FieldType`], which the crate's root re-exports in
//! turn.
//! It also re-exports the walk's rule for damage, [`skip_damage`], so that
//! what reads the tree after the walk loses to damage only what the damage
//! touches, as the walk does. One file holds only tests:
//! `robustness`, which reads truncated and corrupted files through the
//! whole reader.

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
pub(crate) use walk::{Link, Tree, skip_damage, walk};
