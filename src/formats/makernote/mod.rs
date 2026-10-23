// SPDX-License-Identifier: MIT OR Apache-2.0

//! The MakerNote readers: one per camera maker whose MakerNote raw-files
//! reads.
//!
//! A MakerNote is a camera maker's own metadata, stored as the value of
//! one Exif tag, MakerNote, in a layout each maker sets. It records what
//! the standard tags lack, such as the point where the camera focused. The
//! readers here read the layouts raw-files knows, and are shared by every
//! format whose files carry them: NEF and ARW files, and JPEG files, whose
//! Exif segment holds the same MakerNote. The JPEG reader reads both,
//! Nikon's and Sony's, each by IFD0's Make.
//!
//! - [`nikon`] reads Nikon's MakerNote: its small preview JPEG, the visible
//!   image size, the focus point, and the values older bodies record only
//!   there.
//! - [`sony`] reads Sony's MakerNote: the focus point.

pub(in crate::formats) mod nikon;
pub(in crate::formats) mod sony;
