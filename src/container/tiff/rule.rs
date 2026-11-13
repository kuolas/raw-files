// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tag rules: what type and count a tag's value must have to be read.
//!
//! A [`Rule`] names a tag and the field types its value may have. It is
//! declared once, beside the tag's number, and
//! [`Ifd::get`](super::Ifd::get) reads every rule the same way (ADR011). A
//! [`Held`] is a rule whose value always fits in the entry's four bytes,
//! which [`Ifd::held`](super::Ifd::held) reads with no source and no error.
//!
//! A rule's type parameter is its shape: how many values it takes, and what
//! a reader gets.
//!
//! | Shape | Count | A reader gets |
//! |---|---|---|
//! | `[T; N]` | exactly `N` | the `N` values |
//! | `Vec<T>` | any, zero included | every value |
//! | [`First<T>`] | one or more | the first value, read alone |
//! | [`Text`] | any | the bytes before the first NUL |
//!
//! `T` is an [`Element`], read from these field types:
//!
//! | Element | Field types |
//! |---|---|
//! | `u8` | BYTE, UNDEFINED |
//! | `u32` | SHORT, LONG, IFD, a SHORT widened |
//! | `i16` | SSHORT |
//! | `i32` | SLONG |
//! | [`Rational`] | RATIONAL |
//! | [`SignedRational`] | SRATIONAL |
//!
//! [`Text`] is read from ASCII and UTF-8. These are the forms the raw
//! view's values take too ([`crate::entries::Value`]).
//!
//! A rule may name only types its shape is read from: a rule of `u32`s
//! cannot name RATIONAL. [`Rule::new`] checks this, and [`Held::new`] also
//! checks that the value fits in four bytes. Both are `const fn`, and every
//! reader declares its rules as `const` items, so a reader's rule that
//! fails a check stops the build. The checks, [`decodes_to`] and
//! [`fits_in_entry`], are functions of their own so that the tests can
//! evaluate them at compile time.

use std::marker::PhantomData;

use super::FieldType;
use crate::io::ByteOrder;
use crate::rational::{Rational, SignedRational};

/// The rule of a tag: its number, and the field types its value may have,
/// in the shape `S`.
///
/// Declared as a `const` item beside the tag's number, so that
/// [`new`](Self::new)'s check runs at compile time. Read by
/// [`Ifd::get`](super::Ifd::get).
pub(crate) struct Rule<S: Shape> {
    tag: u16,
    types: &'static [FieldType],
    shape: PhantomData<fn() -> S>,
}

/// The rule of a tag whose value always fits in the entry's own four bytes,
/// such as one SHORT or four BYTEs.
///
/// Such a value is never stored elsewhere in the file, so reading it needs
/// no source and cannot fail. Read by [`Ifd::held`](super::Ifd::held).
pub(crate) struct Held<S: Shape>(Rule<S>);

// A derive would ask `S: Copy`, and a `Vec<T>` shape is not.
impl<S: Shape> Clone for Rule<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Shape> Copy for Rule<S> {}

impl<S: Shape> Clone for Held<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Shape> Copy for Held<S> {}

impl<S: Shape> Rule<S> {
    /// The rule of tag `tag`, whose value may have any of `types`.
    ///
    /// # Panics
    ///
    /// When `types` is empty, or names a type `S` is not read from
    /// ([`decodes_to`]). In a `const` item, as every reader's rule is, that
    /// panic stops the build.
    pub(crate) const fn new(tag: u16, types: &'static [FieldType]) -> Self {
        assert!(
            decodes_to(types, S::TYPES),
            "a rule names a type its shape is not read from"
        );
        Self {
            tag,
            types,
            shape: PhantomData,
        }
    }

    /// The tag's number.
    pub(crate) const fn tag(&self) -> u16 {
        self.tag
    }

    /// The field types the tag's value may have.
    pub(crate) const fn types(&self) -> &'static [FieldType] {
        self.types
    }
}

impl<S: Shape> Held<S> {
    /// The rule of tag `tag`, whose value may have any of `types` and always
    /// fits in the entry.
    ///
    /// # Panics
    ///
    /// As [`Rule::new`], and when a value of the shape and of one of `types`
    /// could be longer than four bytes ([`fits_in_entry`]). In a `const`
    /// item, as every reader's rule is, that panic stops the build.
    pub(crate) const fn new(tag: u16, types: &'static [FieldType]) -> Self {
        let rule = Rule::new(tag, types);
        assert!(
            fits_in_entry(types, S::MAX_COUNT),
            "a held rule's value may not fit in its entry"
        );
        Self(rule)
    }

    /// The tag's number.
    pub(crate) const fn tag(&self) -> u16 {
        self.0.tag()
    }

    /// The field types the tag's value may have.
    pub(crate) const fn types(&self) -> &'static [FieldType] {
        self.0.types()
    }
}

/// Whether `types` names at least one type, and only types of `family`:
/// the check [`Rule::new`] makes, with `family` the shape's types.
const fn decodes_to(types: &[FieldType], family: &[FieldType]) -> bool {
    if types.is_empty() {
        return false;
    }
    let mut i = 0;
    while i < types.len() {
        let mut found = false;
        let mut j = 0;
        while j < family.len() {
            if types[i].to_code() == family[j].to_code() {
                found = true;
            }
            j += 1;
        }
        if !found {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether a value of at most `max_count` values, each of one of `types`,
/// always fits in an entry's four bytes: the check [`Held::new`] adds.
/// `None` for `max_count` is a shape of any count, which never fits.
const fn fits_in_entry(types: &[FieldType], max_count: Option<u32>) -> bool {
    let Some(count) = max_count else {
        return false;
    };
    let mut i = 0;
    while i < types.len() {
        match count.checked_mul(types[i].size()) {
            Some(len) if len <= 4 => {}
            _ => return false,
        }
        i += 1;
    }
    true
}

mod sealed {
    /// Keeps the shapes and elements to those this module defines.
    pub(crate) trait Sealed {}
}

/// The shape of a rule's value: which counts it takes, and how a reader
/// gets the value from its bytes. Implemented for `[T; N]`, `Vec<T>`,
/// [`First<T>`] and [`Text`] only.
pub(crate) trait Shape: sealed::Sealed {
    /// What a reader gets.
    type Output;

    /// The field types a value of this shape may be read from.
    const TYPES: &'static [FieldType];

    /// The most values the shape takes; `None` when it takes any count.
    const MAX_COUNT: Option<u32>;

    /// Whether only the first value is read, whatever the count.
    const FIRST_ONLY: bool;

    /// Whether an entry of `count` values has this shape.
    fn takes(count: u32) -> bool;

    /// The value that `bytes`, values of `field_type` in byte order `order`,
    /// hold: all of them, or the first alone when
    /// [`FIRST_ONLY`](Self::FIRST_ONLY). `None` when the bytes do not hold
    /// a whole value, which cannot happen for the bytes of an entry whose
    /// count [`takes`](Self::takes) took. The bytes are given owned, so
    /// that a shape of bytes keeps them rather than copy them.
    fn decode(field_type: FieldType, bytes: Vec<u8>, order: ByteOrder) -> Option<Self::Output>;
}

/// The value of a rule's element, one of the numbers or fractions a field
/// type holds. Implemented for `u8`, `u32`, `i16`, `i32`, [`Rational`] and
/// [`SignedRational`] only.
pub(crate) trait Element: sealed::Sealed + Sized {
    /// The field types the element is read from.
    const TYPES: &'static [FieldType];

    /// The element that `bytes`, one value of `field_type` in byte order
    /// `order`, hold; `None` when they are not one value of that type.
    fn decode(field_type: FieldType, bytes: &[u8], order: ByteOrder) -> Option<Self>;

    /// Every element that `bytes`, values of `field_type` in byte order
    /// `order`, hold; `None` when they are not a whole number of values.
    fn decode_all(field_type: FieldType, bytes: Vec<u8>, order: ByteOrder) -> Option<Vec<Self>> {
        let size = usize::try_from(field_type.size()).ok()?;
        let chunks = bytes.chunks_exact(size);
        if !chunks.remainder().is_empty() {
            return None;
        }
        chunks
            .map(|value| Self::decode(field_type, value, order))
            .collect()
    }
}

/// The shape of a rule whose tag holds one value or more, of which only the
/// first is read: the 2 bytes of a SHORT, say, wherever the others lie.
///
/// A tag of one number that the file gives many values thus costs no more
/// than one, and is never refused for its length.
pub(crate) struct First<T>(PhantomData<fn() -> T>);

/// The shape of a text rule: the bytes of an ASCII or UTF-8 value before
/// its first NUL, or all of them when there is none.
///
/// The bytes are kept as stored, valid UTF-8 or not, for the reader to
/// clean.
pub(crate) enum Text {}

impl<T: Element, const N: usize> sealed::Sealed for [T; N] {}

impl<T: Element, const N: usize> Shape for [T; N] {
    type Output = Self;
    const TYPES: &'static [FieldType] = T::TYPES;
    // An array of more than 2^32 values could match no entry's count.
    const MAX_COUNT: Option<u32> = Some(if N > u32::MAX as usize {
        u32::MAX
    } else {
        N as u32
    });
    const FIRST_ONLY: bool = false;

    fn takes(count: u32) -> bool {
        usize::try_from(count).is_ok_and(|count| count == N)
    }

    fn decode(field_type: FieldType, bytes: Vec<u8>, order: ByteOrder) -> Option<Self> {
        T::decode_all(field_type, bytes, order)?.try_into().ok()
    }
}

impl<T: Element> sealed::Sealed for Vec<T> {}

impl<T: Element> Shape for Vec<T> {
    type Output = Self;
    const TYPES: &'static [FieldType] = T::TYPES;
    const MAX_COUNT: Option<u32> = None;
    const FIRST_ONLY: bool = false;

    fn takes(_count: u32) -> bool {
        true
    }

    fn decode(field_type: FieldType, bytes: Vec<u8>, order: ByteOrder) -> Option<Self> {
        T::decode_all(field_type, bytes, order)
    }
}

impl<T: Element> sealed::Sealed for First<T> {}

impl<T: Element> Shape for First<T> {
    type Output = T;
    const TYPES: &'static [FieldType] = T::TYPES;
    const MAX_COUNT: Option<u32> = None;
    const FIRST_ONLY: bool = true;

    fn takes(count: u32) -> bool {
        count > 0
    }

    fn decode(field_type: FieldType, bytes: Vec<u8>, order: ByteOrder) -> Option<T> {
        let size = usize::try_from(field_type.size()).ok()?;
        T::decode(field_type, bytes.get(..size)?, order)
    }
}

impl sealed::Sealed for Text {}

impl Shape for Text {
    type Output = Vec<u8>;
    const TYPES: &'static [FieldType] = &[FieldType::Ascii, FieldType::Utf8];
    const MAX_COUNT: Option<u32> = None;
    const FIRST_ONLY: bool = false;

    fn takes(_count: u32) -> bool {
        true
    }

    fn decode(_field_type: FieldType, mut bytes: Vec<u8>, _order: ByteOrder) -> Option<Vec<u8>> {
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        Some(bytes)
    }
}

impl sealed::Sealed for u8 {}

impl Element for u8 {
    const TYPES: &'static [FieldType] = &[FieldType::Byte, FieldType::Undefined];

    fn decode(_field_type: FieldType, bytes: &[u8], _order: ByteOrder) -> Option<Self> {
        let &[byte] = bytes else {
            return None;
        };
        Some(byte)
    }

    /// The bytes themselves, one element each: nothing to decode.
    fn decode_all(_field_type: FieldType, bytes: Vec<u8>, _order: ByteOrder) -> Option<Vec<u8>> {
        Some(bytes)
    }
}

impl sealed::Sealed for u32 {}

impl Element for u32 {
    const TYPES: &'static [FieldType] = &[FieldType::Short, FieldType::Long, FieldType::Ifd];

    fn decode(field_type: FieldType, bytes: &[u8], order: ByteOrder) -> Option<Self> {
        if field_type == FieldType::Short {
            Some(u32::from(order.u16(bytes.try_into().ok()?)))
        } else {
            Some(order.u32(bytes.try_into().ok()?))
        }
    }
}

impl sealed::Sealed for i16 {}

impl Element for i16 {
    const TYPES: &'static [FieldType] = &[FieldType::SShort];

    fn decode(_field_type: FieldType, bytes: &[u8], order: ByteOrder) -> Option<Self> {
        Some(order.u16(bytes.try_into().ok()?).cast_signed())
    }
}

impl sealed::Sealed for i32 {}

impl Element for i32 {
    const TYPES: &'static [FieldType] = &[FieldType::SLong];

    fn decode(_field_type: FieldType, bytes: &[u8], order: ByteOrder) -> Option<Self> {
        Some(order.u32(bytes.try_into().ok()?).cast_signed())
    }
}

impl sealed::Sealed for Rational {}

impl Element for Rational {
    const TYPES: &'static [FieldType] = &[FieldType::Rational];

    /// The fraction as stored, numerator first: a zero denominator is kept,
    /// for the reader to judge.
    fn decode(_field_type: FieldType, bytes: &[u8], order: ByteOrder) -> Option<Self> {
        let &[n0, n1, n2, n3, d0, d1, d2, d3] = bytes else {
            return None;
        };
        Some(Rational {
            numerator: order.u32([n0, n1, n2, n3]),
            denominator: order.u32([d0, d1, d2, d3]),
        })
    }
}

impl sealed::Sealed for SignedRational {}

impl Element for SignedRational {
    const TYPES: &'static [FieldType] = &[FieldType::SRational];

    /// The fraction as stored, numerator first: a zero denominator is kept,
    /// for the reader to judge.
    fn decode(_field_type: FieldType, bytes: &[u8], order: ByteOrder) -> Option<Self> {
        let &[n0, n1, n2, n3, d0, d1, d2, d3] = bytes else {
            return None;
        };
        Some(SignedRational {
            numerator: order.u32([n0, n1, n2, n3]).cast_signed(),
            denominator: order.u32([d0, d1, d2, d3]).cast_signed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The checks, evaluated at compile time as a rule's are: if one gives
    // the wrong answer, the tests do not build.
    const _: () = {
        assert!(decodes_to(&[FieldType::Short], u32::TYPES));
        assert!(decodes_to(&[FieldType::Short, FieldType::Long], u32::TYPES));
        assert!(!decodes_to(&[FieldType::Rational], u32::TYPES));
        assert!(!decodes_to(
            &[FieldType::Short, FieldType::SShort],
            u32::TYPES
        ));
        assert!(!decodes_to(&[], u32::TYPES));
        assert!(decodes_to(&[FieldType::Utf8], Text::TYPES));
        assert!(!decodes_to(&[FieldType::Undefined], Text::TYPES));
        assert!(fits_in_entry(&[FieldType::Short, FieldType::Long], Some(1)));
        assert!(fits_in_entry(&[FieldType::Short], Some(2)));
        assert!(fits_in_entry(&[FieldType::Byte], Some(4)));
        assert!(!fits_in_entry(&[FieldType::Short], Some(3)));
        assert!(!fits_in_entry(
            &[FieldType::Short, FieldType::Long],
            Some(2)
        ));
        assert!(!fits_in_entry(&[FieldType::Rational], Some(1)));
        assert!(!fits_in_entry(&[FieldType::Byte], None));
        assert!(!fits_in_entry(&[FieldType::Byte], Some(u32::MAX)));
    };

    /// Every field type there is.
    const ALL: [FieldType; 14] = [
        FieldType::Byte,
        FieldType::Ascii,
        FieldType::Short,
        FieldType::Long,
        FieldType::Rational,
        FieldType::SByte,
        FieldType::Undefined,
        FieldType::SShort,
        FieldType::SLong,
        FieldType::SRational,
        FieldType::Float,
        FieldType::Double,
        FieldType::Ifd,
        FieldType::Utf8,
    ];

    #[test]
    fn each_shape_is_read_from_its_elements_types_alone() {
        // Each family, and the types of each, as the module's table gives
        // them; every other type is outside it.
        let families: [(&str, &[FieldType], &[FieldType]); 7] = [
            ("u8", u8::TYPES, &[FieldType::Byte, FieldType::Undefined]),
            (
                "u32",
                u32::TYPES,
                &[FieldType::Short, FieldType::Long, FieldType::Ifd],
            ),
            ("i16", i16::TYPES, &[FieldType::SShort]),
            ("i32", i32::TYPES, &[FieldType::SLong]),
            ("Rational", Rational::TYPES, &[FieldType::Rational]),
            (
                "SignedRational",
                SignedRational::TYPES,
                &[FieldType::SRational],
            ),
            ("Text", Text::TYPES, &[FieldType::Ascii, FieldType::Utf8]),
        ];
        for (name, family, expected) in families {
            for field_type in ALL {
                let inside = expected.contains(&field_type);
                assert_eq!(
                    decodes_to(&[field_type], family),
                    inside,
                    "{name} {field_type}"
                );
            }
        }
        assert_eq!(<[u32; 2]>::TYPES, u32::TYPES);
        assert_eq!(<Vec<i16>>::TYPES, i16::TYPES);
        assert_eq!(<First<Rational>>::TYPES, Rational::TYPES);
    }

    #[test]
    fn a_held_rule_fits_four_bytes_whatever_type_it_names() {
        // A shape of exactly `count` values fits when `count` values of its
        // largest type do.
        for field_type in ALL {
            for count in 0..=5 {
                let fits = count * field_type.size() <= 4;
                assert_eq!(
                    fits_in_entry(&[field_type], Some(count)),
                    fits,
                    "{count} {field_type}"
                );
            }
        }
        assert_eq!(<[u8; 4]>::MAX_COUNT, Some(4));
        for max in [
            <Vec<u8>>::MAX_COUNT,
            <First<u8>>::MAX_COUNT,
            Text::MAX_COUNT,
        ] {
            assert_eq!(max, None);
        }
    }

    #[test]
    #[should_panic(expected = "a rule names a type its shape is not read from")]
    fn a_rule_of_another_family_is_refused() {
        let _ = Rule::<[u32; 4]>::new(0x0001, &[FieldType::Rational]);
    }

    #[test]
    #[should_panic(expected = "a rule names a type its shape is not read from")]
    fn a_rule_of_no_type_is_refused() {
        let _ = Rule::<Vec<u8>>::new(0x0001, &[]);
    }

    #[test]
    #[should_panic(expected = "a rule names a type its shape is not read from")]
    fn a_held_rule_of_another_family_is_refused() {
        let _ = Held::<[u8; 4]>::new(0x0001, &[FieldType::Ascii]);
    }

    #[test]
    #[should_panic(expected = "a held rule's value may not fit in its entry")]
    fn a_held_rule_longer_than_four_bytes_is_refused() {
        let _ = Held::<[u32; 2]>::new(0x0001, &[FieldType::Short, FieldType::Long]);
    }

    #[test]
    #[should_panic(expected = "a held rule's value may not fit in its entry")]
    fn a_held_rule_of_any_count_is_refused() {
        let _ = Held::<First<u32>>::new(0x0001, &[FieldType::Short]);
    }

    #[test]
    fn a_rule_keeps_its_tag_and_types() {
        const RULE: Rule<Text> = Rule::new(0x010F, &[FieldType::Ascii, FieldType::Utf8]);
        const HELD: Held<[u32; 1]> = Held::new(0x0112, &[FieldType::Short]);
        assert_eq!((RULE.tag(), RULE.types()), (0x010F, Text::TYPES));
        assert_eq!(
            (HELD.tag(), HELD.types()),
            (0x0112, &[FieldType::Short][..])
        );
    }

    #[test]
    fn values_decode_in_both_byte_orders() {
        use ByteOrder::{Big, Little};
        let short = u32::decode(FieldType::Short, &[0x01, 0x02], Little);
        assert_eq!(short, Some(0x0201));
        assert_eq!(
            u32::decode(FieldType::Short, &[0x01, 0x02], Big),
            Some(0x0102)
        );
        let long = [0x01, 0x02, 0x03, 0x04];
        assert_eq!(u32::decode(FieldType::Long, &long, Big), Some(0x0102_0304));
        assert_eq!(
            u32::decode(FieldType::Ifd, &long, Little),
            Some(0x0403_0201)
        );
        assert_eq!(i16::decode(FieldType::SShort, &[0xFF, 0xFE], Big), Some(-2));
        assert_eq!(
            i32::decode(FieldType::SLong, &[0xFE, 0xFF, 0xFF, 0xFF], Little),
            Some(-2)
        );
        let fraction = [0, 0, 0, 1, 0, 0, 0, 0];
        assert_eq!(
            Rational::decode(FieldType::Rational, &fraction, Big),
            Some(Rational {
                numerator: 1,
                denominator: 0
            }),
            "a zero denominator is kept"
        );
        let signed = [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 3];
        assert_eq!(
            SignedRational::decode(FieldType::SRational, &signed, Big),
            Some(SignedRational {
                numerator: -1,
                denominator: 3
            })
        );
        // Bytes that are not one value give none.
        assert_eq!(u32::decode(FieldType::Short, &[1], Little), None);
        assert_eq!(u8::decode(FieldType::Byte, &[1, 2], Little), None);
        assert_eq!(
            <[u32; 2]>::decode(FieldType::Short, vec![1, 0, 2], Little),
            None
        );
        assert_eq!(
            <Vec<u32>>::decode(FieldType::Long, vec![1, 0, 0, 0, 2], Little),
            None
        );
        assert_eq!(
            <First<u32>>::decode(FieldType::Long, vec![1, 0], Little),
            None
        );
        // Bytes, kept as they are, of any count.
        let bytes = <Vec<u8>>::decode(FieldType::Undefined, vec![0, 0xFF, 7], Little);
        assert_eq!(bytes, Some(vec![0, 0xFF, 7]));
        assert_eq!(
            <[u8; 2]>::decode(FieldType::Byte, vec![1, 2], Big),
            Some([1, 2])
        );
        // Text: the bytes before the first NUL, or all of them.
        assert_eq!(
            Text::decode(FieldType::Ascii, b"ab\0c".to_vec(), Little),
            Some(b"ab".to_vec())
        );
        assert_eq!(
            Text::decode(FieldType::Utf8, b"\xC3\xA9".to_vec(), Big),
            Some(b"\xC3\xA9".to_vec())
        );
        assert_eq!(
            Text::decode(FieldType::Ascii, Vec::new(), Big),
            Some(Vec::new())
        );
    }
}
