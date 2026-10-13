// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF field types: what kind of value an entry of an IFD holds.
//!
//! Every entry names the type of its values, and the type says how to
//! read them: how many bytes one value takes, and what those bytes mean,
//! a number, a fraction, text, or bytes that only the tag gives a meaning
//! to. A reader checks the type before it reads a value, so that it never
//! takes a fraction for a number or a number for text. TIFF 6.0 defines
//! types 1 to 12 and TIFF Technical Note 1 adds type 13; an entry of any
//! other type cannot be read.

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
/// Field type SBYTE: 8-bit signed integers.
pub(crate) const SBYTE: u16 = 6;
/// Field type UNDEFINED: bytes that the tag gives a meaning to.
pub(crate) const UNDEFINED: u16 = 7;
/// Field type SSHORT: 16-bit signed integers.
pub(crate) const SSHORT: u16 = 8;
/// Field type SLONG: 32-bit signed integers.
pub(crate) const SLONG: u16 = 9;
/// Field type SRATIONAL: pairs of SLONGs, numerator first.
pub(crate) const SRATIONAL: u16 = 10;
/// Field type FLOAT: 32-bit IEEE floating point.
pub(crate) const FLOAT: u16 = 11;
/// Field type DOUBLE: 64-bit IEEE floating point.
pub(crate) const DOUBLE: u16 = 12;
/// Field type IFD: the offsets of IFDs, as LONGs (TIFF Technical Note 1).
pub(crate) const IFD: u16 = 13;

/// The size in bytes of one value of field type `typ`, or `None` for a type
/// that neither TIFF 6.0 (types 1 to 12) nor TIFF Technical Note 1 (type
/// 13) defines.
pub(super) fn type_size(typ: u16) -> Option<u32> {
    match typ {
        BYTE | ASCII | SBYTE | UNDEFINED => Some(1),
        SHORT | SSHORT => Some(2),
        LONG | SLONG | FLOAT | IFD => Some(4),
        RATIONAL | SRATIONAL | DOUBLE => Some(8),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_sizes() {
        let sizes = [1, 1, 2, 4, 8, 1, 1, 2, 4, 8, 4, 8, 4];
        for (typ, size) in (1..=13).zip(sizes) {
            assert_eq!(type_size(typ), Some(size), "type {typ}");
        }
        for typ in [0, 14, 99, u16::MAX] {
            assert_eq!(type_size(typ), None, "type {typ}");
        }
    }
}
