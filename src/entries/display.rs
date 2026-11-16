// SPDX-License-Identifier: MIT OR Apache-2.0

//! A short text form of a [`Value`], for people to read: [`ValueDisplay`],
//! which [`Value::display`] gives.
//!
//! A value may hold a version of four bytes or a table of a megabyte, so
//! the form is cut after a few values and gives the length of the rest.
//! It is written straight to the formatter, a value at a time, and never
//! built as a string: printing a value of a megabyte costs what printing
//! its first values does, plus a pass over text to count its characters.

use std::fmt::{self, Write as _};

use super::{Value, trim_text};

/// How many numbers, fractions or bytes are written when no precision is
/// given.
const VALUES: usize = 16;

/// How many characters of text are written when no precision is given.
const CHARACTERS: usize = 80;

/// A short form of a [`Value`] for people to read, which `{}` prints;
/// [`Value::display`] gives one.
///
/// | Type | Form |
/// |---|---|
/// | ASCII, UTF-8 | the text as [`Value::text`] cleans it, each control character as [`char::escape_debug`] writes it, such as `\n`; nothing when it cleans to nothing |
/// | BYTE, SBYTE, SHORT, SSHORT, LONG, SLONG, IFD | each number in decimal |
/// | FLOAT, DOUBLE | each number as Rust's `{}` writes it |
/// | RATIONAL, SRATIONAL | each fraction as its own `Display` writes it, such as `28/10` |
/// | UNDEFINED | each byte as two lowercase hexadecimal digits |
///
/// One space separates values. At most 16 numbers, fractions or bytes
/// are written, and 80 characters of text. When more remain, an ellipsis
/// and the length of the whole follow: `… (65432 bytes)` for UNDEFINED,
/// `… (3000 characters)` of cleaned text and `… (512 values)` for the
/// rest. A value with no values writes nothing.
///
/// A precision sets the number written: `{:.4}` writes 4 values, or 4
/// characters of text, and `{:.0}` only the length. Width, fill and
/// alignment are not applied.
///
/// The form is for people to read, as `Debug`'s is, and may change in any
/// release. A program that needs the values matches on the variant of
/// [`Value`].
///
/// # Examples
///
/// ```
/// use raw_files::Value;
///
/// let version = Value::Byte(vec![2, 3, 0, 0]);
/// assert_eq!(version.display().to_string(), "2 3 0 0");
///
/// let curve = Value::Short((0..1000).collect());
/// assert_eq!(format!("{:.3}", curve.display()), "0 1 2 … (1000 values)");
///
/// let comment = Value::Ascii(b"Shot at dawn from the north ridge\0".to_vec());
/// assert_eq!(format!("{:.7}", comment.display()), "Shot at … (33 characters)");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ValueDisplay<'a> {
    value: &'a Value,
}

impl<'a> ValueDisplay<'a> {
    /// The short form of `value`.
    pub(super) fn new(value: &'a Value) -> Self {
        Self { value }
    }
}

impl fmt::Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values = f.precision().unwrap_or(VALUES);
        match self.value {
            Value::Ascii(bytes) | Value::Utf8(bytes) => {
                text(f, bytes, f.precision().unwrap_or(CHARACTERS))
            }
            Value::Byte(v) => numbers(f, v, values),
            Value::SByte(v) => numbers(f, v, values),
            Value::Short(v) => numbers(f, v, values),
            Value::SShort(v) => numbers(f, v, values),
            Value::Long(v) | Value::Ifd(v) => numbers(f, v, values),
            Value::SLong(v) => numbers(f, v, values),
            Value::Float(v) => numbers(f, v, values),
            Value::Double(v) => numbers(f, v, values),
            Value::Rational(v) => numbers(f, v, values),
            Value::SRational(v) => numbers(f, v, values),
            Value::Undefined(v) => list(f, v, values, "byte", |f, b| write!(f, "{b:02x}")),
        }
    }
}

/// Writes the first `max` of `items`, each as its own `Display` writes
/// it, as [`list`] does.
///
/// Each is written with `write!`, which formats it afresh: the precision
/// that chose `max` is never passed on to a number or a fraction.
fn numbers<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T], max: usize) -> fmt::Result {
    list(f, items, max, "value", |f, n| write!(f, "{n}"))
}

/// Writes the first `max` of `items`, one space between, each by `write`;
/// then, when more remain, an ellipsis and how many there are in all,
/// counted in `unit`.
fn list<T>(
    f: &mut fmt::Formatter<'_>,
    items: &[T],
    max: usize,
    unit: &str,
    mut write: impl FnMut(&mut fmt::Formatter<'_>, &T) -> fmt::Result,
) -> fmt::Result {
    for (i, item) in items.iter().take(max).enumerate() {
        if i > 0 {
            f.write_char(' ')?;
        }
        write(f, item)?;
    }
    let written = max.min(items.len());
    rest(f, written, items.len(), unit, written > 0)
}

/// Writes the first `max` characters of the text `bytes` holds, cleaned
/// as [`Value::text`] cleans it; then, when more remain, an ellipsis and
/// how many characters there are in all.
///
/// A control character is written as [`char::escape_debug`] writes it,
/// such as `\n`, so a newline or an escape sequence in a file's text
/// neither breaks the line it is printed on nor reaches a terminal.
/// Bytes that are not UTF-8 become U+FFFD where
/// [`String::from_utf8_lossy`] puts one. Either counts as one character.
fn text(f: &mut fmt::Formatter<'_>, bytes: &[u8], max: usize) -> fmt::Result {
    let mut written = 0;
    let mut total = 0;
    let mut last = None;
    for chunk in trim_text(bytes).utf8_chunks() {
        let invalid = (!chunk.invalid().is_empty()).then_some(char::REPLACEMENT_CHARACTER);
        if written >= max {
            total += chunk.valid().chars().count() + usize::from(invalid.is_some());
            continue;
        }
        for c in chunk.valid().chars().chain(invalid) {
            if written < max {
                if c.is_control() {
                    write!(f, "{}", c.escape_debug())?;
                } else {
                    f.write_char(c)?;
                }
                written += 1;
                last = Some(c);
            }
            total += 1;
        }
    }
    // A text cut after a space has its space before the ellipsis already.
    let space = last.is_some_and(|c| c != ' ');
    rest(f, written, total, "character", space)
}

/// Writes, when `written` of `total` leaves some out, an ellipsis and
/// `total` counted in `unit`, after a space when `space` asks for one.
fn rest(
    f: &mut fmt::Formatter<'_>,
    written: usize,
    total: usize,
    unit: &str,
    space: bool,
) -> fmt::Result {
    if written >= total {
        return Ok(());
    }
    if space {
        f.write_char(' ')?;
    }
    let plural = if total == 1 { "" } else { "s" };
    write!(f, "… ({total} {unit}{plural})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rational, SignedRational};

    fn shown(value: &Value) -> String {
        value.display().to_string()
    }

    #[test]
    fn each_type_prints_its_form() {
        let cases = [
            (Value::Byte(vec![0, 2, 255]), "0 2 255"),
            (Value::SByte(vec![-128, 0, 127]), "-128 0 127"),
            (Value::Short(vec![8256, 5504]), "8256 5504"),
            (Value::SShort(vec![-1, 32767]), "-1 32767"),
            (Value::Long(vec![0, u32::MAX]), "0 4294967295"),
            (Value::SLong(vec![i32::MIN, 7]), "-2147483648 7"),
            (Value::Ifd(vec![2368]), "2368"),
            (Value::Float(vec![0.5, -1.0, f32::NAN]), "0.5 -1 NaN"),
            (Value::Double(vec![2.25, f64::INFINITY]), "2.25 inf"),
            (
                Value::Rational(vec![Rational {
                    numerator: 10,
                    denominator: 3200,
                }]),
                "10/3200",
            ),
            (
                Value::SRational(vec![
                    SignedRational {
                        numerator: -2,
                        denominator: 3,
                    },
                    SignedRational {
                        numerator: 2,
                        denominator: -3,
                    },
                ]),
                "-2/3 2/-3",
            ),
            (Value::Undefined(vec![0x30, 0x0A, 0xFF]), "30 0a ff"),
            (Value::Ascii(b"  NIKON Z 9 \0junk".to_vec()), "NIKON Z 9"),
            (Value::Utf8("Café\0".as_bytes().to_vec()), "Café"),
        ];
        for (value, text) in cases {
            assert_eq!(shown(&value), text, "{value:?}");
        }
    }

    #[test]
    fn a_zero_denominator_prints_as_the_fraction_prints_it() {
        let value = Value::Rational(vec![Rational {
            numerator: 0,
            denominator: 0,
        }]);
        assert_eq!(shown(&value), "0/0");
    }

    #[test]
    fn nothing_prints_for_no_values_or_text_that_cleans_to_nothing() {
        assert_eq!(shown(&Value::Short(Vec::new())), "");
        assert_eq!(shown(&Value::Undefined(Vec::new())), "");
        assert_eq!(shown(&Value::Ascii(Vec::new())), "");
        assert_eq!(shown(&Value::Ascii(b"    \0\0".to_vec())), "");
        assert_eq!(shown(&Value::Utf8(b"\0text after the NUL".to_vec())), "");
    }

    #[test]
    fn one_value_more_than_the_default_is_cut() {
        let value = Value::Short((1..=17).collect());
        assert_eq!(
            shown(&value),
            "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 … (17 values)"
        );
        let value = Value::Short((1..=16).collect());
        assert_eq!(shown(&value), "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16");
    }

    #[test]
    fn a_value_of_a_mebibyte_prints_its_first_values_and_its_length() {
        let value = Value::Undefined(vec![0xAB; 1 << 20]);
        let text = shown(&value);
        assert!(text.len() < 100, "{} bytes: {text}", text.len());
        assert!(text.ends_with(" … (1048576 bytes)"), "{text}");

        let text = shown(&Value::Ascii(vec![b'a'; 1 << 20]));
        assert_eq!(text, format!("{} … (1048576 characters)", "a".repeat(80)));
    }

    #[test]
    fn text_is_cut_at_eighty_characters_by_default() {
        let exact = "é".repeat(80);
        assert_eq!(shown(&Value::Utf8(exact.clone().into_bytes())), exact);
        let longer = Value::Utf8("é".repeat(81).into_bytes());
        assert_eq!(shown(&longer), format!("{exact} … (81 characters)"));
    }

    #[test]
    fn a_precision_sets_how_many_values_print() {
        let value = Value::Long(vec![10, 20, 30, 40, 50]);
        assert_eq!(format!("{:.0}", value.display()), "… (5 values)");
        assert_eq!(
            format!("{:.4}", value.display()),
            "10 20 30 40 … (5 values)"
        );
        assert_eq!(format!("{:.5}", value.display()), "10 20 30 40 50");
        assert_eq!(format!("{:.99}", value.display()), "10 20 30 40 50");
        assert_eq!(
            format!("{:.0}", Value::Byte(vec![7]).display()),
            "… (1 value)"
        );
        assert_eq!(format!("{:.0}", Value::Byte(Vec::new()).display()), "");

        let text = Value::Ascii(b"Shot at dawn\0".to_vec());
        assert_eq!(format!("{:.0}", text.display()), "… (12 characters)");
        assert_eq!(format!("{:.4}", text.display()), "Shot … (12 characters)");
        assert_eq!(format!("{:.99}", text.display()), "Shot at dawn");
        let one = Value::Ascii(b"A".to_vec());
        assert_eq!(format!("{:.0}", one.display()), "… (1 character)");
    }

    #[test]
    fn a_precision_is_not_passed_on_to_a_number() {
        let value = Value::Double(vec![1.0 / 3.0]);
        assert_eq!(
            format!("{:.1}", value.display()),
            (1.0f64 / 3.0).to_string()
        );
    }

    #[test]
    fn width_fill_and_alignment_are_not_applied() {
        let value = Value::Short(vec![1, 2]);
        assert_eq!(format!("{:>12}", value.display()), "1 2");
        assert_eq!(format!("{:*<12.1}", value.display()), "1 … (2 values)");
    }

    #[test]
    fn each_replacement_character_counts_as_one() {
        // As `Value::text` cleans it: a sequence cut short is one U+FFFD.
        let value = Value::Ascii(b"ab\xF0\x9F\x98cd\xC3".to_vec());
        assert_eq!(value.text().as_deref(), Some("ab\u{FFFD}cd\u{FFFD}"));
        assert_eq!(shown(&value), "ab\u{FFFD}cd\u{FFFD}");
        assert_eq!(
            format!("{:.3}", value.display()),
            "ab\u{FFFD} … (6 characters)"
        );
        assert_eq!(format!("{:.2}", value.display()), "ab … (6 characters)");
    }

    #[test]
    fn a_control_character_prints_escaped_and_counts_as_one() {
        let value = Value::Ascii(b"Rembrandt\r\nvan Rijn\t\x1b[31m\x7f".to_vec());
        assert_eq!(
            shown(&value),
            "Rembrandt\\r\\nvan Rijn\\t\\u{1b}[31m\\u{7f}"
        );
        assert_eq!(
            format!("{:.10}", value.display()),
            "Rembrandt\\r … (26 characters)"
        );
        // A C1 control, NEXT LINE, written in UTF-8.
        let value = Value::Utf8("a\u{85}b".as_bytes().to_vec());
        assert_eq!(shown(&value), "a\\u{85}b");
        // Other characters, a non-breaking space among them, print as they are.
        let value = Value::Utf8("Café\u{a0}Monet".as_bytes().to_vec());
        assert_eq!(shown(&value), "Café\u{a0}Monet");
    }

    #[test]
    fn a_text_cut_after_a_space_has_one_space_before_the_ellipsis() {
        let value = Value::Ascii(b"Shot at dawn\0".to_vec());
        assert_eq!(format!("{:.5}", value.display()), "Shot … (12 characters)");
        assert_eq!(format!("{:.4}", value.display()), "Shot … (12 characters)");
    }

    /// A writer that takes `room` bytes, then fails.
    struct Failing {
        room: usize,
    }

    impl fmt::Write for Failing {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.room = self.room.checked_sub(s.len()).ok_or(fmt::Error)?;
            Ok(())
        }
    }

    #[test]
    fn a_writer_that_fails_gives_its_error_back() {
        let values = [
            Value::Short((0..40).collect()),
            Value::Undefined(vec![1; 40]),
            Value::Rational(vec![
                Rational {
                    numerator: 1,
                    denominator: 3,
                };
                40
            ]),
            Value::Ascii(vec![b'x'; 200]),
            Value::Ascii(b"ab\xFFcd".to_vec()),
            Value::Ascii(b"a\nb\x1b".to_vec()),
        ];
        for value in &values {
            let whole = shown(value).len();
            for room in 0..whole {
                let mut out = Failing { room };
                assert_eq!(write!(out, "{}", value.display()), Err(fmt::Error));
            }
            let mut out = Failing { room: whole };
            assert_eq!(write!(out, "{}", value.display()), Ok(()));
        }
    }
}
