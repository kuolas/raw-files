// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF field types: what kind of value an entry of an IFD holds.
//!
//! Every entry names the type of its values, and the type says how to
//! read them: how many bytes one value takes, and what those bytes mean,
//! a number, a fraction, text, or bytes that only the tag gives a meaning
//! to. A reader checks the type before it reads a value, so that it never
//! takes a fraction for a number or a number for text. TIFF 6.0 defines
//! types 1 to 12, TIFF Technical Note 1 adds type 13, and Exif 3.0 adds
//! type 129, text in UTF-8; an entry of any other type cannot be read.
//! [`FieldType`](crate::FieldType) holds the whole table, with each type's
//! size; these constants are the codes the readers compare an entry's type
//! with, one for each type a reader takes.

/// Field type BYTE: 8-bit unsigned integers.
pub(crate) const BYTE: u16 = 1;
/// Field type ASCII: text, ending in a NUL.
pub(crate) const ASCII: u16 = 2;
/// Field type SHORT: 16-bit unsigned integers.
pub(crate) const SHORT: u16 = 3;
/// Field type LONG: 32-bit unsigned integers.
pub(crate) const LONG: u16 = 4;
/// Field type RATIONAL: pairs of LONGs, numerator first.
pub(crate) const RATIONAL: u16 = 5;
/// Field type UNDEFINED: bytes that the tag gives a meaning to.
pub(crate) const UNDEFINED: u16 = 7;
/// Field type SSHORT: 16-bit signed integers.
pub(crate) const SSHORT: u16 = 8;
/// Field type SLONG: 32-bit signed integers.
pub(crate) const SLONG: u16 = 9;
/// Field type SRATIONAL: pairs of SLONGs, numerator first.
pub(crate) const SRATIONAL: u16 = 10;
/// Field type IFD: the offsets of IFDs, as LONGs (TIFF Technical Note 1).
pub(crate) const IFD: u16 = 13;
/// Field type UTF-8: text in UTF-8, ending in a NUL (Exif 3.0).
pub(crate) const UTF8: u16 = 129;
