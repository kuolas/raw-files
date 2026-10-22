// SPDX-License-Identifier: MIT OR Apache-2.0

//! The containers: file structures that several RAW formats share.
//!
//! A container module knows the structure and nothing about camera vendors:
//! the TIFF module reads a TIFF file's structure, the JPEG module the
//! marker segments of a JPEG stream, an embedded preview's or a JPEG
//! file's, and a format module, such as the one for NEF files, decides what
//! that structure means.
//!
//! Each container is a folder, even when it starts as one file, so a
//! container that grows never moves.

pub(crate) mod jpeg;
pub(crate) mod tiff;
