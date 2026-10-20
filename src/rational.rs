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
//! into a decimal with `to_f64`. Each type's `new` refuses a fraction with
//! a zero denominator, which is no number, so that a caller can build its
//! own for its tests. The readers build every signed fraction through
//! `new` too, and every unsigned one through a stricter check of their
//! own, which also refuses a zero numerator: Exif writes a zero for a
//! value the camera did not know.

use std::fmt;

/// A fraction of two unsigned numbers, exactly as the file stored it, such
/// as an exposure time of 10/3200 s.
///
/// Its fields are public because it is a small fixed value: a numerator and
/// a denominator, nothing more.
///
/// The values the library returns never have a zero denominator, nor do
/// the ones [`new`](Self::new) builds, but a struct literal can hold one;
/// every method then still gives a result rather than a panic.
///
/// # Equality
///
/// Two values are equal when they were stored the same way, so 10/3200 is
/// not equal to 1/320, although they are the same number. Compare their
/// [`reduced`](Self::reduced) forms to compare the numbers.
///
/// There is no `Ord` for the same reason: an order by value would disagree
/// with this equality. Sort by [`to_f64`](Self::to_f64) to order by value.
///
/// # Examples
///
/// An exposure time as a Nikon Z 9 stores it, shown as stored, in lowest
/// terms and as a decimal:
///
/// ```
/// use raw_files::Rational;
///
/// let time = Rational { numerator: 10, denominator: 3200 };
/// assert_eq!(time.to_string(), "10/3200");
/// assert_eq!(time.reduced(), Rational { numerator: 1, denominator: 320 });
/// assert_eq!(time.reduced().to_string(), "1/320");
/// assert_eq!(time.to_f64(), 0.003125);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    /// The number above the line, as stored.
    pub numerator: u32,
    /// The number below the line, as stored.
    pub denominator: u32,
}

impl Rational {
    /// The fraction `numerator`/`denominator`, kept as given, not reduced;
    /// `None` when the denominator is zero.
    ///
    /// This is how a caller builds a fraction for its tests. A zero
    /// denominator is refused because the fraction would be no number. A
    /// zero numerator is kept: for a fraction a caller builds, zero is a
    /// real value, such as an angle of 0° or a distance of 0. The readers
    /// never return a zero unsigned fraction, since Exif writes a zero for
    /// a value the camera did not know, and they read it as `None`.
    ///
    /// The fields are public, so a struct literal can still hold any two
    /// numbers, unchecked: that is why every method still gives a result,
    /// never a panic, for a zero denominator.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::Rational;
    ///
    /// # fn main() {
    /// #     example().expect("10/3200 is accepted");
    /// # }
    /// # fn example() -> Option<()> {
    /// let time = Rational::new(10, 3200)?;
    /// assert_eq!((time.numerator, time.denominator), (10, 3200));
    /// // A fraction that is no number.
    /// assert_eq!(Rational::new(1, 0), None);
    /// # Some(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        (denominator != 0).then_some(Self {
            numerator,
            denominator,
        })
    }

    /// The fraction a file stored as `numerator`/`denominator`, when it
    /// gives a value: as [`new`](Self::new) builds it, and also `None` when
    /// the numerator is zero.
    ///
    /// This is how the readers build every unsigned fraction they read.
    /// Exif writes a zero for a value the camera did not know: an f-number
    /// of 0/10 from a lens that does not report its aperture, as on the
    /// public ILCE-7S sample 1582, or a lens range of 0/0. That suits every
    /// unsigned fraction the library reads, such as an exposure time, an
    /// f-number or a focal length: none of them can be zero when known.
    pub(crate) fn known(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0 {
            return None;
        }
        Self::new(numerator, denominator)
    }

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
/// The values the library returns never have a zero denominator, nor do
/// the ones [`new`](Self::new) builds, but a struct literal can hold one;
/// every method then still gives a result rather than a panic.
///
/// # Equality
///
/// Two values are equal when they were stored the same way, so -2/6 is not
/// equal to -1/3, nor 2/-3 to -2/3, although they are the same number.
/// Compare their [`reduced`](Self::reduced) forms to compare the numbers.
///
/// There is no `Ord` for the same reason: an order by value would disagree
/// with this equality. Sort by [`to_f64`](Self::to_f64) to order by value.
///
/// # Examples
///
/// An exposure bias of two thirds of a stop under, as a camera stores it in
/// sixths:
///
/// ```
/// use raw_files::SignedRational;
///
/// let bias = SignedRational { numerator: -4, denominator: 6 };
/// assert_eq!(bias.to_string(), "-4/6");
/// assert_eq!(bias.reduced(), SignedRational { numerator: -2, denominator: 3 });
/// assert!((bias.to_f64() + 2.0 / 3.0).abs() < 1e-9);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SignedRational {
    /// The number above the line, as stored.
    pub numerator: i32,
    /// The number below the line, as stored.
    pub denominator: i32,
}

impl SignedRational {
    /// The fraction `numerator`/`denominator`, kept as given, not reduced,
    /// with each sign where it is given; `None` when the denominator is
    /// zero.
    ///
    /// This is how the readers build every signed fraction they return,
    /// and how a caller builds one for its tests. A zero denominator is
    /// refused because the fraction would be no number. A zero numerator is
    /// kept: the signed fraction the library reads, the exposure bias, is
    /// zero when no compensation was chosen, which cameras write as 0/6 or
    /// 0/10.
    ///
    /// The fields are public, so a struct literal can still hold any two
    /// numbers, unchecked: that is why every method still gives a result,
    /// never a panic, for a zero denominator.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::SignedRational;
    ///
    /// # fn main() {
    /// #     example().expect("0/6 is accepted");
    /// # }
    /// # fn example() -> Option<()> {
    /// // No exposure compensation, as a camera writes it in sixths.
    /// let bias = SignedRational::new(0, 6)?;
    /// assert_eq!((bias.numerator, bias.denominator), (0, 6));
    /// assert_eq!(SignedRational::new(-2, 0), None);
    /// # Some(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(numerator: i32, denominator: i32) -> Option<Self> {
        (denominator != 0).then_some(Self {
            numerator,
            denominator,
        })
    }

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
    fn unsigned_new_refuses_only_a_zero_denominator() {
        // Kept as given, not reduced.
        assert_eq!(Rational::new(10, 3200), Some(unsigned(10, 3200)));
        assert_eq!(Rational::new(u32::MAX, 1), Some(unsigned(u32::MAX, 1)));
        // A real zero, such as an angle of 0°, kept as given.
        assert_eq!(Rational::new(0, 1), Some(unsigned(0, 1)));
        assert_eq!(Rational::new(0, 10), Some(unsigned(0, 10)));
        // Fractions that are no number.
        assert_eq!(Rational::new(1, 0), None);
        assert_eq!(Rational::new(7, 0), None);
        assert_eq!(Rational::new(0, 0), None);
    }

    #[test]
    fn known_refuses_a_zero_numerator_or_denominator() {
        // Kept as given, not reduced.
        assert_eq!(Rational::known(10, 3200), Some(unsigned(10, 3200)));
        assert_eq!(Rational::known(u32::MAX, 1), Some(unsigned(u32::MAX, 1)));
        // An unknown f-number, as Sony writes it, then an unknown lens
        // range and fractions that are no number.
        assert_eq!(Rational::known(0, 10), None);
        assert_eq!(Rational::known(0, 1), None);
        assert_eq!(Rational::known(0, 0), None);
        assert_eq!(Rational::known(1, 0), None);
        assert_eq!(Rational::known(7, 0), None);
    }

    #[test]
    fn signed_new_refuses_only_a_zero_denominator() {
        // Kept as given, not reduced, each sign where it is given.
        assert_eq!(SignedRational::new(-4, 6), Some(signed(-4, 6)));
        assert_eq!(SignedRational::new(-3, 10), Some(signed(-3, 10)));
        assert_eq!(SignedRational::new(2, -3), Some(signed(2, -3)));
        // No compensation: a real zero, kept as stored.
        assert_eq!(SignedRational::new(0, 6), Some(signed(0, 6)));
        assert_eq!(
            SignedRational::new(i32::MIN, -1),
            Some(signed(i32::MIN, -1))
        );
        assert_eq!(SignedRational::new(0, 0), None);
        assert_eq!(SignedRational::new(-4, 0), None);
        assert_eq!(SignedRational::new(-7, 0), None);
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
