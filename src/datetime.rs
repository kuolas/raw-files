// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dates and times as a camera's clock records them.
//!
//! A camera stamps each picture with the time its clock showed when the
//! picture was taken: local time, in whatever zone the clock was set to.
//! Many cameras also record the fraction of a second and the offset of
//! that clock from UTC. [`DateTime`] holds such a time as it was recorded,
//! checked to be a real date and time, and shows it in ISO 8601 form.
//!
//! Exif stores the three parts as separate ASCII texts: the date and time
//! as `YYYY:MM:DD HH:MM:SS`, the fraction of a second as its digits alone,
//! and the offset as `+HH:MM` or `-HH:MM`. [`DateTime::from_exif`] reads
//! them.
//!
//! Each rule a time is held to lives in one public constructor:
//! [`DateTime::new`] for the date and time, [`DateTime::with_subseconds`]
//! for the fraction of a second and [`DateTime::with_offset_minutes`] for
//! the offset. The reader builds every time it returns through them, and a
//! caller builds one through them for its tests, so the two cannot differ.

use std::fmt;

/// The latest year a `DateTime` may have: the last that the four digits of
/// its ISO 8601 form can show, and the last that the four digits of an Exif
/// date can hold.
const MAX_YEAR: u16 = 9999;

/// The farthest an offset from UTC may lie from it, in minutes: 14 hours,
/// since no offset from UTC in use lies outside -14:00 to +14:00. An offset
/// beyond it is refused; from a file, it is taken for damage.
const MAX_OFFSET_MINUTES: i16 = 14 * 60;

/// A date and time as a camera's clock recorded it: local time, with the
/// fraction of a second and the offset from UTC when the file records them.
///
/// The date and time are always real ones: the year is 0 to 9999, the
/// month 1 to 12, the day exists in its month (29 February only in a leap
/// year), the hour is 0 to 23, and the minute and second are 0 to 59. A
/// clock that was never set, or an impossible date, gives no `DateTime` at
/// all rather than a wrong one.
///
/// A `DateTime` comes from a file, or a caller builds one with
/// [`new`](Self::new), adding a fraction of a second with
/// [`with_subseconds`](Self::with_subseconds) and an offset with
/// [`with_offset_minutes`](Self::with_offset_minutes), for its tests. The
/// same rules apply either way: the reader builds every time it returns
/// through those three.
///
/// Without an offset, the time is local to a zone the file does not name,
/// so it cannot be compared with times from other clocks.
///
/// # Equality and order
///
/// Two values are equal when they were recorded the same way: the same
/// date and time, the same offset or the same lack of one, and the same
/// number of sub-second digits. So `10:20:30.5` and `10:20:30.50` are not
/// equal, although they name the same moment.
///
/// `DateTime` has no ordering, because local times do not order by their
/// fields: 10:00 at an offset of `+02:00` came before 09:00 at `+00:00`,
/// and a time without an offset cannot be placed against either. Sort by
/// the getters in the order you need, such as `(year, month, day, hour,
/// minute, second, nanosecond)` for the order of one camera's clock.
///
/// # Display
///
/// Its [`Display`](fmt::Display) form is ISO 8601: the date and time as
/// `YYYY-MM-DDTHH:MM:SS`; then a `.` and the sub-second digits as recorded,
/// when there are any, up to nine of them (digits past the ninth are not
/// kept); then the offset as `+HH:MM` or `-HH:MM`, when it is known, never
/// as `Z`. A time taken at 10:20:30 and 85 hundredths, five hours behind
/// UTC, shows as `2022-01-31T10:20:30.85-05:00`.
///
/// # Examples
///
/// A file's capture time: print it, or take its parts to name the file by
/// the moment it was taken, milliseconds included, since the frames of a
/// burst share their second:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// if let Some(time) = file.metadata().capture_time() {
///     println!("taken {time}");
///     let name = format!(
///         "{:04}{:02}{:02}-{:02}{:02}{:02}.{:03}.nef",
///         time.year(),
///         time.month(),
///         time.day(),
///         time.hour(),
///         time.minute(),
///         time.second(),
///         time.nanosecond() / 1_000_000,
///     );
///     println!("could be renamed to {name}");
/// }
/// # Ok(())
/// # }
/// ```
///
/// A time built for a test, as a camera would record 85 hundredths of a
/// second past 10:20:30 on 1 May 2024, two hours ahead of UTC; a date that
/// does not exist gives none:
///
/// ```
/// use raw_files::DateTime;
///
/// # fn main() {
/// #     example().expect("every step accepts a real date and time");
/// # }
/// # fn example() -> Option<()> {
/// let time = DateTime::new(2024, 5, 1, 10, 20, 30)?
///     .with_subseconds(850_000_000, 2)?
///     .with_offset_minutes(120)?;
/// assert_eq!(time.to_string(), "2024-05-01T10:20:30.85+02:00");
///
/// // 2023 was not a leap year.
/// assert_eq!(DateTime::new(2023, 2, 29, 0, 0, 0), None);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
    /// How many sub-second digits were recorded, from 1 to 9; 0 when none
    /// were. `.5` and `.50` give the same `nanosecond` and differ here.
    subsec_digits: u8,
    /// Minutes east of UTC.
    offset: Option<i16>,
}

impl DateTime {
    /// The date and time `year`-`month`-`day` `hour`:`minute`:`second`,
    /// with no fraction of a second and no offset from UTC; `None` when
    /// they are not a real date and time.
    ///
    /// This is how the reader builds every time it returns, and how a
    /// caller builds one for its tests. It gives `None` when:
    ///
    /// - `year` is above 9999, which the four digits of the
    ///   [`Display`](fmt::Display) form cannot show;
    /// - `month` is not 1 to 12;
    /// - `day` is 0, or past the last day of its month. 29 February exists
    ///   only in a leap year, by the Gregorian rule: every fourth year,
    ///   except the centuries that 400 does not divide, so 2000 was a leap
    ///   year and 1900 was not;
    /// - `hour` is above 23, or `minute` or `second` above 59. A leap
    ///   second is not accepted.
    ///
    /// Add a fraction of a second with
    /// [`with_subseconds`](Self::with_subseconds) and an offset with
    /// [`with_offset_minutes`](Self::with_offset_minutes).
    #[must_use]
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        if year > MAX_YEAR
            || !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return None;
        }
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond: 0,
            subsec_digits: 0,
            offset: None,
        })
    }

    /// This time with the fraction of a second `nanosecond`, in
    /// nanoseconds, recorded with `digits` digits, in place of the one it
    /// has, if any. A time recorded as `.85` is
    /// `with_subseconds(850_000_000, 2)`, and one recorded as `.850` is
    /// `with_subseconds(850_000_000, 3)`: they name the same moment, but
    /// differ in equality and in the [`Display`](fmt::Display) form, as
    /// they would from a file.
    ///
    /// It gives `None` when:
    ///
    /// - `digits` is 0 or above 9: a fraction of a second is recorded with
    ///   1 to 9 digits, the ninth being the nanosecond, and a time without
    ///   one is simply built without this;
    /// - `nanosecond` is 1 000 000 000 or more, a whole second;
    /// - `nanosecond` does not fit in `digits` digits: it must be a
    ///   multiple of 10 to the power of 9 minus `digits`, so that the
    ///   digits recorded hold all of it. 855 000 000 with 2 digits is
    ///   refused, since `.85` would lose its last 5.
    #[must_use]
    pub fn with_subseconds(self, nanosecond: u32, digits: u8) -> Option<Self> {
        if !(1..=9).contains(&digits) || nanosecond >= 1_000_000_000 {
            return None;
        }
        // The value of one unit of the last digit recorded: 1 for nine
        // digits, 100 000 000 for one.
        let unit = 10_u32.pow(9 - u32::from(digits));
        nanosecond.is_multiple_of(unit).then_some(Self {
            nanosecond,
            subsec_digits: digits,
            ..self
        })
    }

    /// This time with the offset from UTC `minutes`, in minutes east of
    /// UTC, in place of the one it has, if any: -300 for a clock five hours
    /// behind UTC (`-05:00`), 60 for one an hour ahead (`+01:00`).
    ///
    /// This is how the reader gives a time the offset its Exif text
    /// records, or one a file records apart from the Exif texts, such as
    /// the time-zone setting of a Nikon camera's clock; and how a caller
    /// gives one to a time for its tests.
    ///
    /// It gives `None` when `minutes` lies more than 14 hours from UTC,
    /// beyond `-14:00` to `+14:00`, since no offset from UTC in use lies
    /// outside that.
    #[must_use]
    pub fn with_offset_minutes(self, minutes: i16) -> Option<Self> {
        (-MAX_OFFSET_MINUTES..=MAX_OFFSET_MINUTES)
            .contains(&minutes)
            .then_some(Self {
                offset: Some(minutes),
                ..self
            })
    }

    /// The time read from the three Exif texts that record it: the date
    /// and time, such as DateTimeOriginal; the fraction of a second, such
    /// as SubSecTimeOriginal; and the offset from UTC, such as
    /// OffsetTimeOriginal. The last two are `None` when the file lacks
    /// them.
    ///
    /// Each text may still end in a NUL, or in several, and is cut at its
    /// first NUL. Spaces at either end of what remains are padding and are
    /// dropped; any other space, beyond the one between the date and the
    /// time, or any other blank such as a tab, is kept and makes the text
    /// malformed.
    ///
    /// Returns `None` when the date and time are not exactly
    /// `YYYY:MM:DD HH:MM:SS`, with digits in every digit place and those
    /// separators, or do not name a real date and time, by the rules of
    /// [`new`](Self::new). That covers a clock that was never set, which
    /// cameras record as spaces, as spaces between the separators, or as
    /// zeros.
    ///
    /// The fraction of a second is one or more ASCII digits; digits past the
    /// ninth are cut, not rounded. The offset is exactly `+HH:MM` or
    /// `-HH:MM`, with the minutes from 00 to 59, and is then held to the
    /// bound of [`with_offset_minutes`](Self::with_offset_minutes), at
    /// most 14 hours from UTC. A fraction or offset in any other form is
    /// left out, and the time stands without it.
    pub(crate) fn from_exif(
        date: &[u8],
        subsec: Option<&[u8]>,
        offset: Option<&[u8]>,
    ) -> Option<Self> {
        // The 19 bytes of `YYYY:MM:DD HH:MM:SS`, with the separators matched
        // in place.
        let &[
            y0,
            y1,
            y2,
            y3,
            b':',
            mo0,
            mo1,
            b':',
            d0,
            d1,
            b' ',
            h0,
            h1,
            b':',
            mi0,
            mi1,
            b':',
            s0,
            s1,
        ] = cleaned(date)
        else {
            return None;
        };
        let time = Self::new(
            u16::from(two_digits(y0, y1)?) * 100 + u16::from(two_digits(y2, y3)?),
            two_digits(mo0, mo1)?,
            two_digits(d0, d1)?,
            two_digits(h0, h1)?,
            two_digits(mi0, mi1)?,
            two_digits(s0, s1)?,
        )?;
        // A fraction or offset that its parser or its constructor refuses
        // is left out, and the time stands without it. The parser never
        // gives a fraction the constructor refuses.
        let time = subsec
            .and_then(parse_subsec)
            .and_then(|(nanosecond, digits)| time.with_subseconds(nanosecond, digits))
            .unwrap_or(time);
        let time = offset
            .and_then(parse_offset)
            .and_then(|minutes| time.with_offset_minutes(minutes))
            .unwrap_or(time);
        Some(time)
    }

    /// The year, from 0 to 9999.
    pub fn year(&self) -> u16 {
        self.year
    }

    /// The month, from 1 (January) to 12 (December).
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month, from 1 to the month's last day.
    pub fn day(&self) -> u8 {
        self.day
    }

    /// The hour, from 0 to 23.
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute, from 0 to 59.
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second, from 0 to 59. A leap second is not accepted.
    pub fn second(&self) -> u8 {
        self.second
    }

    /// The fraction of the second, in nanoseconds: the recorded sub-second
    /// digits read as a decimal fraction, so `.85` gives 850 000 000.
    ///
    /// 0 when the file records no sub-seconds, as when it records `.0`;
    /// the two differ in equality and in the [`Display`](fmt::Display)
    /// form.
    pub fn nanosecond(&self) -> u32 {
        self.nanosecond
    }

    /// How many sub-second digits the file recorded, up to the nine the
    /// library keeps; 0 when it recorded none. A time recorded as `.5` and
    /// one recorded as `.50` give the same [`nanosecond`](Self::nanosecond)
    /// and differ here, as they do in equality and in the
    /// [`Display`](fmt::Display) form; so do a time without sub-seconds and
    /// one whose recorded digits are zeros.
    pub fn subsecond_digits(&self) -> u8 {
        self.subsec_digits
    }

    /// The offset of the camera's clock from UTC, in minutes east of UTC:
    /// -300 for a clock five hours behind UTC (`-05:00`), 60 for one an
    /// hour ahead (`+01:00`).
    ///
    /// `None` when the file records no offset; the time is then local to a
    /// zone the file does not name.
    pub fn offset_minutes(&self) -> Option<i16> {
        self.offset
    }
}

impl fmt::Display for DateTime {
    /// Writes the time in ISO 8601 form, such as
    /// `2022-01-31T10:20:30.85-05:00`, with the sub-second digits as
    /// recorded and the offset only when it is known.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second,
        )?;
        if self.subsec_digits > 0 {
            // The recorded digits are the leading ones of the nine; the
            // rest are zeros the reader added.
            let dropped = 9 - u32::from(self.subsec_digits);
            let digits = self.nanosecond / 10_u32.pow(dropped);
            let width = usize::from(self.subsec_digits);
            write!(f, ".{digits:0width$}")?;
        }
        if let Some(offset) = self.offset {
            let sign = if offset < 0 { '-' } else { '+' };
            let minutes = offset.unsigned_abs();
            write!(f, "{sign}{:02}:{:02}", minutes / 60, minutes % 60)?;
        }
        Ok(())
    }
}

/// The text before the first NUL, or all of it when there is none, without
/// the spaces at either end, which are padding. Only the space byte is
/// dropped, not tabs or other blanks, and not the spaces inside the text.
fn cleaned(text: &[u8]) -> &[u8] {
    let text = text.split(|&b| b == 0).next().unwrap_or(text);
    let start = text.iter().position(|&b| b != b' ').unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|&b| b != b' ')
        .map_or(start, |i| i + 1);
    text.get(start..end).unwrap_or_default()
}

/// The number written by two ASCII digits, from 0 to 99; `None` when
/// either byte is not an ASCII digit.
fn two_digits(tens: u8, ones: u8) -> Option<u8> {
    Some(digit(tens)? * 10 + digit(ones)?)
}

/// The value of one ASCII digit; `None` for any other byte.
fn digit(byte: u8) -> Option<u8> {
    byte.is_ascii_digit().then(|| byte - b'0')
}

/// How many days month `month` (1 to 12) of year `year` has.
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Whether `year` has a 29 February, by the Gregorian rule: every fourth
/// year, except the centuries that 400 does not divide, so 2000 was a leap
/// year and 1900 was not.
fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// The fraction of a second from its Exif text, as nanoseconds and the
/// number of digits kept; `None` when the text, cut at its first NUL and
/// without the spaces at either end, is not one or more ASCII digits.
fn parse_subsec(text: &[u8]) -> Option<(u32, u8)> {
    let digits = cleaned(text);
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    // Nanoseconds have nine places; further digits are cut, not rounded.
    let kept = digits.get(..9).unwrap_or(digits);
    let value = kept
        .iter()
        .fold(0_u32, |value, &b| value * 10 + u32::from(b - b'0'));
    // `kept` holds 1 to 9 digits, so both conversions are exact.
    let count = u8::try_from(kept.len()).ok()?;
    Some((value * 10_u32.pow(9 - u32::from(count)), count))
}

/// The offset from UTC, in minutes east of UTC, from its Exif text; `None`
/// when the text, cut at its first NUL and without the spaces at either
/// end, is not exactly `+HH:MM` or `-HH:MM` with the minutes from 00 to 59.
///
/// Any hours from 00 to 99 are read: the bound of 14 hours from UTC is
/// [`DateTime::with_offset_minutes`]'s, which the offset is given through.
fn parse_offset(text: &[u8]) -> Option<i16> {
    let &[sign, h0, h1, b':', m0, m1] = cleaned(text) else {
        return None;
    };
    let sign = match sign {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours = two_digits(h0, h1)?;
    let minutes = two_digits(m0, m1)?;
    // At most 99 hours and 99 minutes, so this cannot overflow.
    let total = i16::from(hours) * 60 + i16::from(minutes);
    (minutes <= 59).then_some(sign * total)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The date and time every test starts from, with a NUL as in a file.
    const DATE: &[u8] = b"2022:01:31 10:20:30\0";

    /// The time read from the three texts, which the test expects to be
    /// accepted.
    fn read(date: &[u8], subsec: Option<&[u8]>, offset: Option<&[u8]>) -> DateTime {
        DateTime::from_exif(date, subsec, offset).unwrap_or_else(|| {
            panic!(
                "{:?} {:?} {:?} refused",
                String::from_utf8_lossy(date),
                subsec.map(String::from_utf8_lossy),
                offset.map(String::from_utf8_lossy),
            )
        })
    }

    /// Checks every getter of `time`, the date and time as one tuple, and
    /// its [`Display`](fmt::Display) form.
    fn check(
        time: DateTime,
        fields: (u16, u8, u8, u8, u8, u8),
        nanosecond: u32,
        offset: Option<i16>,
        display: &str,
    ) {
        assert_eq!(
            (
                time.year(),
                time.month(),
                time.day(),
                time.hour(),
                time.minute(),
                time.second(),
            ),
            fields,
            "{display}"
        );
        assert_eq!(time.nanosecond(), nanosecond, "{display}");
        assert_eq!(time.offset_minutes(), offset, "{display}");
        assert_eq!(time.to_string(), display);
    }

    /// Checks that `date` is refused, whatever the fraction and offset.
    fn refused(date: &[u8]) {
        assert_eq!(
            DateTime::from_exif(date, Some(b"85\0"), Some(b"+01:00\0")),
            None,
            "{:?}",
            String::from_utf8_lossy(date)
        );
    }

    const FIELDS: (u16, u8, u8, u8, u8, u8) = (2022, 1, 31, 10, 20, 30);

    #[test]
    fn the_z9_form() {
        let time = read(DATE, Some(b"85\0"), Some(b"-05:00\0"));
        check(
            time,
            FIELDS,
            850_000_000,
            Some(-300),
            "2022-01-31T10:20:30.85-05:00",
        );
    }

    #[test]
    fn without_subseconds_or_offset() {
        check(
            read(DATE, None, None),
            FIELDS,
            0,
            None,
            "2022-01-31T10:20:30",
        );
    }

    #[test]
    fn one_to_nine_subsecond_digits_are_kept_as_recorded() {
        let cases: [(&[u8], u32, &str); 4] = [
            (b"5", 500_000_000, "2022-01-31T10:20:30.5"),
            (b"050", 50_000_000, "2022-01-31T10:20:30.050"),
            (b"000000001", 1, "2022-01-31T10:20:30.000000001"),
            (b"123456789", 123_456_789, "2022-01-31T10:20:30.123456789"),
        ];
        for (subsec, nanosecond, display) in cases {
            check(
                read(DATE, Some(subsec), None),
                FIELDS,
                nanosecond,
                None,
                display,
            );
        }
    }

    #[test]
    fn subsecond_digits_past_nine_are_cut_not_rounded() {
        let cases: [(&[u8], u32, &str); 2] = [
            (b"1234567890", 123_456_789, "2022-01-31T10:20:30.123456789"),
            (b"1234567899", 123_456_789, "2022-01-31T10:20:30.123456789"),
        ];
        for (subsec, nanosecond, display) in cases {
            check(
                read(DATE, Some(subsec), None),
                FIELDS,
                nanosecond,
                None,
                display,
            );
        }
    }

    #[test]
    fn equality_counts_the_subsecond_digits() {
        let half = read(DATE, Some(b"5"), None);
        let fifty = read(DATE, Some(b"50"), None);
        assert_eq!(half.nanosecond(), fifty.nanosecond());
        assert_ne!(half, fifty);
        assert_eq!(half, read(DATE, Some(b"5\0"), None));
        assert_ne!(read(DATE, Some(b"0"), None), read(DATE, None, None));
    }

    #[test]
    fn subseconds_padded_with_spaces() {
        let cases: [&[u8]; 4] = [b"85  \0", b" 85", b"85 ", b" 85 "];
        for subsec in cases {
            check(
                read(DATE, Some(subsec), None),
                FIELDS,
                850_000_000,
                None,
                "2022-01-31T10:20:30.85",
            );
        }
    }

    #[test]
    fn malformed_subseconds_leave_the_time_without_them() {
        let cases: [&[u8]; 8] = [b"8 5", b"", b"\0", b"   ", b"ab", b"\t85", b"85\t", b"-5"];
        for subsec in cases {
            check(
                read(DATE, Some(subsec), Some(b"+01:00")),
                FIELDS,
                0,
                Some(60),
                "2022-01-31T10:20:30+01:00",
            );
            // The time alone cannot tell an empty fraction from a refused
            // one, since both leave it without sub-seconds; the reader of
            // the fraction can.
            assert_eq!(parse_subsec(subsec), None, "{subsec:?}");
        }
    }

    #[test]
    fn offsets_accepted() {
        let cases: [(&[u8], i16, &str); 8] = [
            (b"+00:00", 0, "2022-01-31T10:20:30+00:00"),
            (b"-00:00", 0, "2022-01-31T10:20:30+00:00"),
            (b"+14:00", 840, "2022-01-31T10:20:30+14:00"),
            (b"-14:00", -840, "2022-01-31T10:20:30-14:00"),
            (b"-12:00", -720, "2022-01-31T10:20:30-12:00"),
            (b"+05:30\0", 330, "2022-01-31T10:20:30+05:30"),
            (b"-09:30", -570, "2022-01-31T10:20:30-09:30"),
            (b" -05:00 \0", -300, "2022-01-31T10:20:30-05:00"),
        ];
        for (offset, minutes, display) in cases {
            check(
                read(DATE, None, Some(offset)),
                FIELDS,
                0,
                Some(minutes),
                display,
            );
        }
    }

    #[test]
    fn malformed_offsets_leave_the_time_without_one() {
        let cases: [&[u8]; 12] = [
            b"+5:00",
            b"05:00",
            b"+05:60",
            b"Z",
            b"+0500",
            b"+05:000",
            b" 05:00",
            b"+05 :00",
            b"\t+05:00",
            b"+05-00",
            b"",
            b"\0",
        ];
        for offset in cases {
            check(
                read(DATE, Some(b"85"), Some(offset)),
                FIELDS,
                850_000_000,
                None,
                "2022-01-31T10:20:30.85",
            );
        }
    }

    #[test]
    fn offsets_beyond_fourteen_hours_leave_the_time_without_one() {
        let cases: [&[u8]; 5] = [b"+14:01", b"-14:30", b"+15:00", b"+23:30", b"+24:00"];
        for offset in cases {
            check(
                read(DATE, Some(b"85"), Some(offset)),
                FIELDS,
                850_000_000,
                None,
                "2022-01-31T10:20:30.85",
            );
        }
    }

    #[test]
    fn an_offset_given_apart_is_taken_up_to_fourteen_hours() {
        let cases: [(i16, &str); 5] = [
            (60, "2022-01-31T10:20:30.85+01:00"),
            (0, "2022-01-31T10:20:30.85+00:00"),
            (-420, "2022-01-31T10:20:30.85-07:00"),
            (840, "2022-01-31T10:20:30.85+14:00"),
            (-840, "2022-01-31T10:20:30.85-14:00"),
        ];
        for (minutes, display) in cases {
            // On a time without an offset, and in place of an Exif one.
            for offset in [None, Some(&b"-05:00"[..])] {
                let time = read(DATE, Some(b"85"), offset).with_offset_minutes(minutes);
                check(
                    time.expect("an offset in range"),
                    FIELDS,
                    850_000_000,
                    Some(minutes),
                    display,
                );
            }
        }
    }

    #[test]
    fn an_offset_given_apart_beyond_fourteen_hours_is_refused() {
        for minutes in [841, -841, 15360, i16::MAX, i16::MIN] {
            let time = read(DATE, Some(b"85"), None);
            assert_eq!(time.with_offset_minutes(minutes), None, "{minutes}");
        }
    }

    #[test]
    fn unset_clocks_are_none() {
        refused(b"                   \0");
        refused(b"    :  :     :  :  \0");
        refused(b"0000:00:00 00:00:00\0");
        refused(b"\0");
        refused(b"");
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        let accepted: [(&[u8], u16, &str); 2] = [
            (b"2024:02:29 00:00:00", 2024, "2024-02-29T00:00:00"),
            (b"2000:02:29 00:00:00", 2000, "2000-02-29T00:00:00"),
        ];
        for (date, year, display) in accepted {
            check(
                read(date, None, None),
                (year, 2, 29, 0, 0, 0),
                0,
                None,
                display,
            );
        }
        refused(b"2023:02:29 00:00:00");
        refused(b"1900:02:29 00:00:00");
        refused(b"2024:02:30 00:00:00");
    }

    #[test]
    fn each_field_is_checked_against_its_range() {
        check(
            read(b"2022:12:31 23:59:59", None, None),
            (2022, 12, 31, 23, 59, 59),
            0,
            None,
            "2022-12-31T23:59:59",
        );
        check(
            read(b"2022:04:30 00:00:00", None, None),
            (2022, 4, 30, 0, 0, 0),
            0,
            None,
            "2022-04-30T00:00:00",
        );
        refused(b"2022:04:31 00:00:00");
        refused(b"2022:01:32 00:00:00");
        refused(b"2022:13:01 00:00:00");
        refused(b"2022:00:01 00:00:00");
        refused(b"2022:01:00 00:00:00");
        refused(b"2022:01:31 24:00:00");
        refused(b"2022:01:31 10:60:00");
        refused(b"2022:01:31 10:20:60");
    }

    #[test]
    fn wrong_separators_are_none() {
        refused(b"2022-01-31 10:20:30");
        refused(b"2022:01:31T10:20:30");
        refused(b"2022:01:31 10-20-30");
        refused(b"2022:01:31:10:20:30");
        // Each separator on its own, so that none goes unchecked.
        for place in [4, 7, 10, 13, 16] {
            let mut date = *b"2022:01:31 10:20:30";
            date[place] = b'/';
            refused(&date);
        }
    }

    #[test]
    fn wrong_lengths_are_none() {
        refused(b"2022:01:31 10:20:3");
        refused(b"2022:01:31 10:20:300");
        refused(b"2022:01:31  10:20:30");
        refused(b"2022:01:31 10:20:30\t");
        refused(b"2022:1:31 10:20:30\0");
    }

    #[test]
    fn a_date_padded_with_spaces_is_read() {
        let cases: [&[u8]; 3] = [
            b" 2022:01:31 10:20:30",
            b"2022:01:31 10:20:30 ",
            b"  2022:01:31 10:20:30  \0",
        ];
        for date in cases {
            check(
                read(date, None, None),
                FIELDS,
                0,
                None,
                "2022-01-31T10:20:30",
            );
        }
    }

    #[test]
    fn non_digits_in_digit_places_are_none() {
        // U+0662, an Arabic-Indic two, takes two bytes in UTF-8 and here
        // stands for "20", so the length is still 19.
        refused("\u{662}22:01:31 10:20:30".as_bytes());
        refused(b"2022:01:31 10:20:3\xff");
        refused(b"\xb2022:01:31 10:20:30");
        refused(b"2022:01:31 1 :20:30");
        refused(b"2022:01:31  1:20:30");
        refused(b"2022:0a:31 10:20:30");
    }

    #[test]
    fn bytes_after_the_first_nul_are_ignored() {
        check(
            read(
                b"2022:01:31 10:20:30\0garbage",
                Some(b"85\09"),
                Some(b"-05:00\0+01:00"),
            ),
            FIELDS,
            850_000_000,
            Some(-300),
            "2022-01-31T10:20:30.85-05:00",
        );
    }

    #[test]
    fn subsecond_digits_count_what_was_recorded() {
        assert_eq!(read(DATE, None, None).subsecond_digits(), 0);
        assert_eq!(read(DATE, Some(b"5"), None).subsecond_digits(), 1);
        assert_eq!(read(DATE, Some(b"50"), None).subsecond_digits(), 2);
        assert_eq!(read(DATE, Some(b"00"), None).subsecond_digits(), 2);
        assert_eq!(read(DATE, Some(b"1234567890"), None).subsecond_digits(), 9);
    }

    /// The time `new` builds from these fields, which the test expects to
    /// be accepted.
    fn built(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> DateTime {
        DateTime::new(year, month, day, hour, minute, second).unwrap_or_else(|| {
            panic!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} refused")
        })
    }

    #[test]
    fn new_takes_29_february_only_in_a_gregorian_leap_year() {
        assert_eq!(built(2024, 2, 29, 0, 0, 0).day(), 29);
        assert_eq!(DateTime::new(2023, 2, 29, 0, 0, 0), None);
        // The century rule: 400 divides 2000, not 1900.
        assert_eq!(built(2000, 2, 29, 0, 0, 0).day(), 29);
        assert_eq!(DateTime::new(1900, 2, 29, 0, 0, 0), None);
        assert_eq!(DateTime::new(2024, 2, 30, 0, 0, 0), None);
    }

    #[test]
    fn new_checks_each_field_against_its_range() {
        assert_eq!(built(2022, 4, 30, 0, 0, 0).day(), 30);
        assert_eq!(DateTime::new(2022, 4, 31, 0, 0, 0), None);
        assert_eq!(DateTime::new(2022, 0, 1, 0, 0, 0), None);
        assert_eq!(DateTime::new(2022, 13, 1, 0, 0, 0), None);
        assert_eq!(DateTime::new(2022, 1, 0, 0, 0, 0), None);
        let last = built(2022, 12, 31, 23, 59, 59);
        assert_eq!(last.to_string(), "2022-12-31T23:59:59");
        assert_eq!(DateTime::new(2022, 1, 31, 24, 0, 0), None);
        assert_eq!(DateTime::new(2022, 1, 31, 10, 60, 0), None);
        assert_eq!(DateTime::new(2022, 1, 31, 10, 20, 60), None);
    }

    #[test]
    fn new_takes_the_years_four_digits_can_show() {
        assert_eq!(built(0, 1, 1, 0, 0, 0).to_string(), "0000-01-01T00:00:00");
        assert_eq!(
            built(9999, 12, 31, 0, 0, 0).to_string(),
            "9999-12-31T00:00:00"
        );
        assert_eq!(DateTime::new(10000, 1, 1, 0, 0, 0), None);
        assert_eq!(DateTime::new(u16::MAX, 1, 1, 0, 0, 0), None);
    }

    #[test]
    fn new_builds_a_time_without_subseconds_or_offset_as_a_file_gives_it() {
        let time = built(2022, 1, 31, 10, 20, 30);
        check(time, FIELDS, 0, None, "2022-01-31T10:20:30");
        assert_eq!(time.subsecond_digits(), 0);
        assert_eq!(time, read(DATE, None, None));
    }

    #[test]
    fn with_subseconds_takes_one_to_nine_digits() {
        let time = built(2022, 1, 31, 10, 20, 30);
        assert_eq!(time.with_subseconds(0, 0), None);
        assert_eq!(time.with_subseconds(500_000_000, 0), None);
        assert_eq!(time.with_subseconds(0, 10), None);
        assert_eq!(time.with_subseconds(500_000_000, 10), None);
        let nine = time.with_subseconds(999_999_999, 9).unwrap();
        assert_eq!(nine.to_string(), "2022-01-31T10:20:30.999999999");
        assert_eq!(nine.subsecond_digits(), 9);
    }

    #[test]
    fn with_subseconds_refuses_a_whole_second() {
        let time = built(2022, 1, 31, 10, 20, 30);
        assert_eq!(time.with_subseconds(1_000_000_000, 9), None);
        assert_eq!(time.with_subseconds(1_000_000_000, 1), None);
        assert_eq!(time.with_subseconds(u32::MAX, 9), None);
    }

    #[test]
    fn with_subseconds_refuses_what_its_digits_cannot_hold() {
        let time = built(2022, 1, 31, 10, 20, 30);
        let hundredths = time.with_subseconds(850_000_000, 2).unwrap();
        check(
            hundredths,
            FIELDS,
            850_000_000,
            None,
            "2022-01-31T10:20:30.85",
        );
        assert_eq!(time.with_subseconds(855_000_000, 2), None);
        assert_eq!(time.with_subseconds(1, 8), None);
        assert_eq!(
            time.with_subseconds(1, 9).unwrap().to_string(),
            "2022-01-31T10:20:30.000000001"
        );
    }

    #[test]
    fn with_subseconds_of_zero_differs_from_none() {
        let time = built(2022, 1, 31, 10, 20, 30);
        let zero = time.with_subseconds(0, 1).unwrap();
        assert_eq!(zero.to_string(), "2022-01-31T10:20:30.0");
        assert_eq!((zero.nanosecond(), zero.subsecond_digits()), (0, 1));
        assert_ne!(zero, time);
    }

    #[test]
    fn with_subseconds_replaces_earlier_ones_as_a_file_gives_them() {
        let time = built(2022, 1, 31, 10, 20, 30);
        let hundredths = time.with_subseconds(850_000_000, 2).unwrap();
        assert_eq!(hundredths, read(DATE, Some(b"85"), None));
        let tenths = hundredths.with_subseconds(500_000_000, 1).unwrap();
        assert_eq!(tenths, read(DATE, Some(b"5"), None));
        assert_eq!(tenths.subsecond_digits(), 1);
    }

    #[test]
    fn with_offset_minutes_takes_fourteen_hours_either_way() {
        let time = built(2022, 1, 31, 10, 20, 30);
        let east = time.with_offset_minutes(840).unwrap();
        assert_eq!(east.to_string(), "2022-01-31T10:20:30+14:00");
        let west = time.with_offset_minutes(-840).unwrap();
        assert_eq!(west.to_string(), "2022-01-31T10:20:30-14:00");
        assert_eq!(time.with_offset_minutes(841), None);
        assert_eq!(time.with_offset_minutes(-841), None);
    }

    #[test]
    fn with_offset_minutes_replaces_an_earlier_one_as_a_file_gives_it() {
        let time = built(2022, 1, 31, 10, 20, 30)
            .with_subseconds(850_000_000, 2)
            .and_then(|time| time.with_offset_minutes(60))
            .and_then(|time| time.with_offset_minutes(-300))
            .unwrap();
        assert_eq!(time.offset_minutes(), Some(-300));
        assert_eq!(time, read(DATE, Some(b"85"), Some(b"-05:00")));
    }
}
