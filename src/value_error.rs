// SPDX-License-Identifier: MIT OR Apache-2.0

//! The error of a value built by hand: which rule refused it.

use std::fmt;

/// Why a function refused its arguments: the one rule they broke.
///
/// A program builds values without a file for its tests, such as a
/// [`DateTime`](crate::DateTime), a [`Rational`](crate::Rational) or a
/// [`Focus`](crate::Focus). A function that checks its arguments returns
/// a `ValueError` when they break one of its rules. Each variant names
/// one rule, and each function's `# Errors` section lists the variants it
/// returns.
///
/// A function checks its rules in the order its `# Errors` section lists
/// them. When several rules are broken, the first in that order is
/// returned.
///
/// `ValueError` is not a [`raw_files::Error`](crate::Error), which comes
/// from reading a file, and does not convert into one. A function that
/// opens a file and builds values returns a `Box<dyn std::error::Error>`,
/// which `?` fills from either.
///
/// `ValueError` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`Display`](fmt::Display) text says the rule in lowercase, with no
/// final period, so that it reads inside a caller's own message.
///
/// # Examples
///
/// A focus area that does not fit around its center, as a test might
/// build by mistake, and the rule it broke:
///
/// ```
/// use raw_files::{Focus, FocusOrigin, Point, Size, ValueError};
///
/// # fn main() -> Result<(), ValueError> {
/// let frame = Size { width: 100, height: 100 };
/// let focus = Focus::new(frame, Point { x: 10, y: 50 }, FocusOrigin::Recorded)?;
/// let err = focus.with_size(Size { width: 30, height: 30 }).unwrap_err();
/// assert_eq!(err, ValueError::AreaOffFrame);
/// assert_eq!(
///     err.to_string(),
///     "the area does not fit inside the frame around its center"
/// );
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueError {
    /// A fraction's denominator is zero, which makes it no number.
    ZeroDenominator,
    /// The year is above 9999, which the four digits of a time's ISO 8601
    /// form cannot show.
    YearOutOfRange,
    /// The month is not 1 to 12.
    MonthOutOfRange,
    /// The day is 0, or past the last day of its month.
    DayOutOfRange,
    /// The hour is above 23.
    HourOutOfRange,
    /// The minute is above 59.
    MinuteOutOfRange,
    /// The second is above 59: a leap second is not accepted.
    SecondOutOfRange,
    /// The number of sub-second digits is not 1 to 9.
    SubsecondDigitsOutOfRange,
    /// The nanoseconds make a whole second or more: 1 000 000 000 or more.
    NanosecondOutOfRange,
    /// The nanoseconds do not fit in the sub-second digits: 855 000 000
    /// with 2 digits would lose its last 5.
    NanosecondFinerThanDigits,
    /// The offset lies more than 14 hours from UTC, beyond `-14:00` to
    /// `+14:00`.
    OffsetOutOfRange,
    /// The minutes of arc are 60 or more.
    ArcMinutesOutOfRange,
    /// The seconds of arc are 60 or more.
    ArcSecondsOutOfRange,
    /// The degrees, minutes and seconds come to more than 90°.
    LatitudeOutOfRange,
    /// The degrees, minutes and seconds come to more than 180°.
    LongitudeOutOfRange,
    /// The angle is 360° or more: a full turn is 0°.
    DirectionOutOfRange,
    /// A side of the frame is zero.
    ZeroFrameSide,
    /// The center lies outside the frame: its x is more than the frame's
    /// width, or its y more than its height.
    CenterOffFrame,
    /// A side of the area is zero.
    ZeroAreaSide,
    /// The area does not fit inside the frame around its center, by the
    /// rule [`Focus::with_size`](crate::Focus::with_size) states.
    AreaOffFrame,
    /// The list has no point at that index.
    PointIndexOutOfRange,
    /// The layout has no points.
    NoPoints,
    /// A point's number is 0, or not above the number of the point before
    /// it.
    PointNumbersNotRising,
}

impl fmt::Display for ValueError {
    /// Writes the rule that was broken, such as "the hour is above 23".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ZeroDenominator => "the denominator of a fraction is zero",
            Self::YearOutOfRange => "the year is above 9999",
            Self::MonthOutOfRange => "the month is not 1 to 12",
            Self::DayOutOfRange => "the day is 0 or past the last day of its month",
            Self::HourOutOfRange => "the hour is above 23",
            Self::MinuteOutOfRange => "the minute is above 59",
            Self::SecondOutOfRange => "the second is above 59",
            Self::SubsecondDigitsOutOfRange => "the number of sub-second digits is not 1 to 9",
            Self::NanosecondOutOfRange => "the nanoseconds make a whole second or more",
            Self::NanosecondFinerThanDigits => {
                "the nanoseconds do not fit in the sub-second digits"
            }
            Self::OffsetOutOfRange => "the offset lies more than 14 hours from UTC",
            Self::ArcMinutesOutOfRange => "the minutes of arc are 60 or more",
            Self::ArcSecondsOutOfRange => "the seconds of arc are 60 or more",
            Self::LatitudeOutOfRange => "the latitude is more than 90°",
            Self::LongitudeOutOfRange => "the longitude is more than 180°",
            Self::DirectionOutOfRange => "the direction is 360° or more",
            Self::ZeroFrameSide => "a side of the frame is zero",
            Self::CenterOffFrame => "the center lies outside the frame",
            Self::ZeroAreaSide => "a side of the area is zero",
            Self::AreaOffFrame => "the area does not fit inside the frame around its center",
            Self::PointIndexOutOfRange => "the list has no point at that index",
            Self::NoPoints => "the layout has no points",
            Self::PointNumbersNotRising => {
                "a point's number is 0 or not above the number before it"
            }
        })
    }
}

impl std::error::Error for ValueError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_ordered;
    use std::collections::HashSet;

    const fn assert_traits<T: Send + Sync + Copy + 'static>() {}
    const _: () = assert_traits::<ValueError>();

    /// Every variant, in the order they are declared.
    const ALL: [ValueError; 23] = [
        ValueError::ZeroDenominator,
        ValueError::YearOutOfRange,
        ValueError::MonthOutOfRange,
        ValueError::DayOutOfRange,
        ValueError::HourOutOfRange,
        ValueError::MinuteOutOfRange,
        ValueError::SecondOutOfRange,
        ValueError::SubsecondDigitsOutOfRange,
        ValueError::NanosecondOutOfRange,
        ValueError::NanosecondFinerThanDigits,
        ValueError::OffsetOutOfRange,
        ValueError::ArcMinutesOutOfRange,
        ValueError::ArcSecondsOutOfRange,
        ValueError::LatitudeOutOfRange,
        ValueError::LongitudeOutOfRange,
        ValueError::DirectionOutOfRange,
        ValueError::ZeroFrameSide,
        ValueError::CenterOffFrame,
        ValueError::ZeroAreaSide,
        ValueError::AreaOffFrame,
        ValueError::PointIndexOutOfRange,
        ValueError::NoPoints,
        ValueError::PointNumbersNotRising,
    ];

    #[test]
    fn every_variant_has_a_text_of_its_own() {
        let texts: HashSet<String> = ALL.iter().map(ToString::to_string).collect();
        assert_eq!(texts.len(), ALL.len(), "{texts:?}");
    }

    #[test]
    fn every_text_is_lowercase_with_no_final_period() {
        for err in ALL {
            let text = err.to_string();
            assert!(
                text.starts_with(|c: char| c.is_ascii_lowercase()),
                "{err:?}: {text}"
            );
            assert!(!text.ends_with('.'), "{err:?}: {text}");
        }
    }

    #[test]
    fn orders_as_listed() {
        assert_ordered(&ALL);
    }

    #[test]
    fn is_one_byte() {
        assert_eq!(size_of::<ValueError>(), 1);
    }

    #[test]
    fn display_writes_the_rule() {
        assert_eq!(
            ValueError::HourOutOfRange.to_string(),
            "the hour is above 23"
        );
        assert_eq!(
            ValueError::AreaOffFrame.to_string(),
            "the area does not fit inside the frame around its center"
        );
    }
}
