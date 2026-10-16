// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fractions exactly as a file stores them.
//!
//! Exif records most exposure values as fractions: an exposure time of
//! 10/3200 s, an f-number of 900/100 for f/9, an exposure bias of -2/3 EV.
//! Cameras do not reduce them, and the numbers they write say something a
//! decimal would lose, so the library returns each fraction with the
//! numerator and denominator the file recorded. [`Rational`] holds an
//! unsigned one and [`SignedRational`] a signed one; a caller reduces them
//! with `reduced` to compare or show them in lowest terms, or turns them
//! into a decimal with `to_f64`.

use std::fmt;

/// A fraction of two unsigned numbers, exactly as the file stored it, such
/// as an exposure time of 10/3200 s.
///
/// Its fields are public because it is a small fixed value: a numerator and
/// a denominator, nothing more.
///
/// The values the library returns never have a zero denominator, but a
/// caller can build one; every method then still gives a result rather than
/// a panic.
///
/// # Equality
///
/// Two values are equal when they were stored the same way, so 10/3200 is
/// not equal to 1/320, although they are the same number. Compare their
/// [`reduced`](Self::reduced) forms to compare the numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    /// The number above the line, as stored.
    pub numerator: u32,
    /// The number below the line, as stored.
    pub denominator: u32,
}

impl Rational {
    /// The fraction as a decimal: the numerator divided by the denominator,
    /// so 10/3200 gives 0.003125.
    ///
    /// A zero denominator, which the library never returns, gives what
    /// floating-point division gives: infinity, or NaN for 0/0.
    pub fn to_f64(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }

    /// The same fraction in lowest terms, with the numerator and the
    /// denominator divided by their greatest common divisor: 10/3200 gives
    /// 1/320, 900/100 gives 9/1, and a zero numerator gives 0/1.
    ///
    /// A fraction with a zero denominator is returned unchanged.
    pub fn reduced(self) -> Self {
        if self.denominator == 0 {
            return self;
        }
        // At least 1, since the denominator is not zero.
        let divisor = gcd(self.numerator, self.denominator);
        Self {
            numerator: self.numerator / divisor,
            denominator: self.denominator / divisor,
        }
    }
}

impl fmt::Display for Rational {
    /// Writes the fraction as stored, numerator first: `10/3200`, not
    /// `1/320`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

/// A fraction of two signed numbers, exactly as the file stored it, such
/// as an exposure bias of -2/3 EV.
///
/// Its fields are public because it is a small fixed value: a numerator and
/// a denominator, nothing more. The sign may be on either number, or on
/// both, as the file stored it.
///
/// The values the library returns never have a zero denominator, but a
/// caller can build one; every method then still gives a result rather than
/// a panic.
///
/// # Equality
///
/// Two values are equal when they were stored the same way, so -2/6 is not
/// equal to -1/3, nor 2/-3 to -2/3, although they are the same number.
/// Compare their [`reduced`](Self::reduced) forms to compare the numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SignedRational {
    /// The number above the line, as stored.
    pub numerator: i32,
    /// The number below the line, as stored.
    pub denominator: i32,
}

impl SignedRational {
    /// The fraction as a decimal: the numerator divided by the denominator,
    /// so -2/3 gives about -0.667.
    ///
    /// A zero denominator, which the library never returns, gives what
    /// floating-point division gives: an infinity, or NaN for 0/0.
    pub fn to_f64(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }

    /// The same fraction in lowest terms, with the sign on the numerator:
    /// the numerator and the denominator are divided by their greatest
    /// common divisor, and the denominator is made positive. So -6/6 gives
    /// -1/1, 3/-10 gives -3/10, -3/-10 gives 3/10, and a zero numerator
    /// gives 0/1.
    ///
    /// The fraction is returned unchanged when its denominator is zero, or
    /// when the result does not fit in an `i32`: -2147483648/-1 would be
    /// 2147483648/1, and 1/-2147483648 would have a denominator of
    /// 2147483648.
    pub fn reduced(self) -> Self {
        if self.denominator == 0 {
            return self;
        }
        // The magnitudes, as unsigned numbers, so that `i32::MIN` has one.
        let numerator = self.numerator.unsigned_abs();
        let denominator = self.denominator.unsigned_abs();
        // At least 1, since the denominator is not zero.
        let divisor = gcd(numerator, denominator);
        let magnitude = i64::from(numerator / divisor);
        let negative = (self.numerator < 0) != (self.denominator < 0);
        let numerator = if negative { -magnitude } else { magnitude };
        match (
            i32::try_from(numerator),
            i32::try_from(denominator / divisor),
        ) {
            (Ok(numerator), Ok(denominator)) => Self {
                numerator,
                denominator,
            },
            _ => self,
        }
    }
}

impl fmt::Display for SignedRational {
    /// Writes the fraction as stored, numerator first, with each sign
    /// where it was stored: `-2/3`, `0/6`, `2/-3`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

/// The greatest common divisor of `a` and `b`, by Euclid's algorithm; `a`
/// when `b` is zero, so 0 only when both are.
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unsigned(numerator: u32, denominator: u32) -> Rational {
        Rational {
            numerator,
            denominator,
        }
    }

    fn signed(numerator: i32, denominator: i32) -> SignedRational {
        SignedRational {
            numerator,
            denominator,
        }
    }

    #[test]
    fn unsigned_reduced_to_lowest_terms() {
        let cases = [
            ((10, 3200), (1, 320)),
            ((900, 100), (9, 1)),
            ((0, 6), (0, 1)),
            ((1, 320), (1, 320)),
            ((u32::MAX, u32::MAX), (1, 1)),
        ];
        for ((n, d), (rn, rd)) in cases {
            assert_eq!(unsigned(n, d).reduced(), unsigned(rn, rd), "{n}/{d}");
        }
    }

    #[test]
    fn unsigned_with_a_zero_denominator_stays_unchanged() {
        for n in [0, 7, u32::MAX] {
            assert_eq!(unsigned(n, 0).reduced(), unsigned(n, 0), "{n}/0");
        }
    }

    #[test]
    fn signed_reduced_to_lowest_terms_with_the_sign_on_the_numerator() {
        let cases = [
            ((-3, 10), (-3, 10)),
            ((-6, 6), (-1, 1)),
            ((3, -10), (-3, 10)),
            ((-3, -10), (3, 10)),
            ((-2, 6), (-1, 3)),
            ((0, 6), (0, 1)),
            ((0, -6), (0, 1)),
            ((i32::MIN, 1), (i32::MIN, 1)),
            ((i32::MIN, 2), (-1_073_741_824, 1)),
            ((i32::MIN, i32::MIN), (1, 1)),
        ];
        for ((n, d), (rn, rd)) in cases {
            assert_eq!(signed(n, d).reduced(), signed(rn, rd), "{n}/{d}");
        }
    }

    #[test]
    fn signed_that_would_not_fit_stays_unchanged() {
        // In lowest terms with the sign on the numerator, these would need
        // 2147483648 above or below the line.
        for (n, d) in [
            (i32::MIN, -1),
            (1, i32::MIN),
            (-1, i32::MIN),
            (i32::MAX, i32::MIN),
        ] {
            assert_eq!(signed(n, d).reduced(), signed(n, d), "{n}/{d}");
        }
    }

    #[test]
    fn signed_with_a_zero_denominator_stays_unchanged() {
        for n in [0, 7, -7, i32::MIN, i32::MAX] {
            assert_eq!(signed(n, 0).reduced(), signed(n, 0), "{n}/0");
        }
    }

    #[test]
    fn to_f64_divides() {
        assert_eq!(unsigned(10, 3200).to_f64(), 0.003125);
        assert_eq!(unsigned(0, 6).to_f64(), 0.0);
        assert!((signed(-2, 3).to_f64() + 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(signed(0, 6).to_f64(), 0.0);
        assert_eq!(signed(3, -10).to_f64(), -0.3);
    }

    #[test]
    fn to_f64_with_a_zero_denominator_follows_floating_point() {
        assert_eq!(unsigned(7, 0).to_f64(), f64::INFINITY);
        assert!(unsigned(0, 0).to_f64().is_nan());
        assert_eq!(signed(-7, 0).to_f64(), f64::NEG_INFINITY);
        assert!(signed(0, 0).to_f64().is_nan());
    }

    #[test]
    fn display_as_stored() {
        assert_eq!(unsigned(10, 3200).to_string(), "10/3200");
        assert_eq!(unsigned(0, 6).to_string(), "0/6");
        assert_eq!(signed(-2, 3).to_string(), "-2/3");
        assert_eq!(signed(0, 6).to_string(), "0/6");
        assert_eq!(signed(2, -3).to_string(), "2/-3");
    }

    #[test]
    fn equality_as_stored() {
        assert_ne!(unsigned(10, 3200), unsigned(1, 320));
        assert_eq!(unsigned(10, 3200).reduced(), unsigned(1, 320).reduced());
        assert_ne!(signed(2, -3), signed(-2, 3));
        assert_eq!(signed(2, -3).reduced(), signed(-2, 3).reduced());
    }
}
