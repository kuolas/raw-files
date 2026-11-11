// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where a picture was taken.
//!
//! Exif records a picture's position in its GPS IFD: a latitude and a
//! longitude, each as degrees, minutes and seconds north or south of the
//! equator, east or west of the prime meridian, and beside them the
//! altitude, the GPS time in UTC, the direction the camera faced
//! and the map datum the position is given in. [`Location`] holds them as
//! the file records them: [`Latitude`] and [`Longitude`] the two
//! coordinates, [`Altitude`] the height above or below its reference, and
//! [`Direction`] a bearing counted from the [`North`] the file names.
//!
//! A location always has its position, and its other parts are optional:
//! a file whose GPS IFD records no sound position has no location at all,
//! whatever else that IFD holds, and a damaged altitude, time, direction
//! or datum costs only itself.
//! [`Metadata::location`](crate::Metadata::location) gives a file's.
//!
//! A program that uses raw-files can build a [`Location`] for its own
//! tests through the checked constructors of its parts, which refuse a
//! value that names no place, such as a latitude beyond 90° or minutes of
//! 60 or more. [`Location`]'s docs show how.

use std::fmt;

use crate::datetime::DateTime;
use crate::rational::Rational;
use crate::value_error::ValueError;

/// Where a picture was taken: its latitude and longitude, with the
/// altitude, the GPS time, the direction the camera faced and the
/// map datum when the file records them.
///
/// The position is as the file records it: each coordinate keeps the
/// degrees, minutes and seconds the file wrote, as fractions, and its
/// hemisphere; [`Latitude::to_degrees`] and [`Longitude::to_degrees`] give
/// decimal degrees for a map. Exif's GPS tags may also hold a position
/// from another satellite system or from a mobile network: the file does
/// not say which gave it.
///
/// A location names a place, and raw-files gives it as the file records
/// it: nothing is rounded or hidden, and there is no option to hide it. A
/// program that shows or shares it decides what to keep.
///
/// # Equality
///
/// Two locations are equal when every part is the same as recorded: a
/// minute written as 30/1 is not equal to one written as 300000/10000,
/// although they are the same angle, as for [`Rational`]. There is no
/// order: no order of places would agree with that equality. There is no
/// `Default` either, since a location has a position: a file that records
/// none gives `None`, and a location built for a test starts from its
/// latitude and longitude ([`new`](Self::new)).
///
/// # Examples
///
/// A program's tests build the location of a picture taken under the
/// Eiffel Tower, in degrees and minutes to four decimals as a Nikon camera
/// writes them, with its altitude and the GPS time:
///
/// ```
/// use raw_files::{Altitude, DateTime, Latitude, Location, Longitude, Rational};
///
/// # fn main() -> Result<(), raw_files::ValueError> {
/// let latitude = Latitude::north(
///     Rational::new(48, 1)?,
///     Rational::new(514_933, 10_000)?,
///     Rational::new(0, 1)?,
/// )?;
/// let longitude = Longitude::east(
///     Rational::new(2, 1)?,
///     Rational::new(176_700, 10_000)?,
///     Rational::new(0, 1)?,
/// )?;
/// let location = Location::new(latitude, longitude)
///     .with_altitude(Altitude::above(Rational::new(35, 1)?)?)
///     .with_time(DateTime::new(2026, 6, 21, 10, 15, 30)?.with_offset_minutes(0)?);
///
/// assert_eq!(location.latitude().minutes(), Rational::new(514_933, 10_000)?);
/// assert!((location.latitude().to_degrees() - 48.858_22).abs() < 1e-5);
/// assert!((location.longitude().to_degrees() - 2.294_5).abs() < 1e-5);
/// assert_eq!(location.altitude().map(Altitude::to_meters), Some(35.0));
/// assert_eq!(location.map_datum(), None);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Location {
    pub(crate) latitude: Latitude,
    pub(crate) longitude: Longitude,
    pub(crate) altitude: Option<Altitude>,
    pub(crate) time: Option<DateTime>,
    pub(crate) image_direction: Option<Direction>,
    pub(crate) map_datum: Option<String>,
}

impl Location {
    /// The location at `latitude` and `longitude`, with no altitude, time,
    /// image direction or map datum; the `with_…` methods set those.
    ///
    /// This is how a caller starts a location for its tests. The two
    /// coordinates are already checked by their own constructors, so every
    /// pair names a place.
    #[must_use]
    pub fn new(latitude: Latitude, longitude: Longitude) -> Self {
        Self {
            latitude,
            longitude,
            altitude: None,
            time: None,
            image_direction: None,
            map_datum: None,
        }
    }

    /// The latitude, as the file records it: GPSLatitudeRef and
    /// GPSLatitude, or as given to [`new`](Self::new).
    pub fn latitude(&self) -> Latitude {
        self.latitude
    }

    /// The longitude, as the file records it: GPSLongitudeRef and
    /// GPSLongitude, or as given to [`new`](Self::new).
    pub fn longitude(&self) -> Longitude {
        self.longitude
    }

    /// The altitude, as the file records it: GPSAltitude, above or below
    /// its reference as GPSAltitudeRef says, or as given to
    /// [`with_altitude`](Self::with_altitude).
    ///
    /// `None` when the file does not record both tags, or records either
    /// in a way that cannot be read: a GPSAltitude that is not one
    /// fraction with a denominator above zero, or a GPSAltitudeRef that
    /// is not one byte of 0 to 3. [`Altitude`] says why the surface the
    /// height is measured from is not given.
    pub fn altitude(&self) -> Option<Altitude> {
        self.altitude
    }

    /// The GPS time, in UTC, as the file records it: GPSDateStamp and
    /// GPSTimeStamp together, with an offset of zero and the fraction of a
    /// second the file records, or as given to
    /// [`with_time`](Self::with_time).
    ///
    /// It is not the capture time, which the camera's own clock gives in
    /// [`Metadata::capture_time`](crate::Metadata::capture_time).
    ///
    /// `None` when the file does not record both tags, or records a date
    /// and time that do not exist, such as an hour of 24, or in a way that
    /// cannot be read: hours or minutes that are not whole, or seconds
    /// whose denominator is not a power of ten up to 10⁹ (1, 10, 100 and
    /// so on), whose zeros are the digits of the fraction of a second.
    pub fn time(&self) -> Option<DateTime> {
        self.time
    }

    /// The direction the camera faced, as the file records it:
    /// GPSImgDirection, counted from the north GPSImgDirectionRef names,
    /// or as given to [`with_image_direction`](Self::with_image_direction).
    ///
    /// `None` when the file does not record both tags, or records either
    /// in a way that cannot be read: a GPSImgDirection that is not one
    /// fraction below 360°, or a GPSImgDirectionRef that is neither `T`
    /// nor `M`. No north is assumed when the reference is missing.
    pub fn image_direction(&self) -> Option<Direction> {
        self.image_direction
    }

    /// The geodetic datum the position is given in, as the file records
    /// it: GPSMapDatum, such as `WGS-84`, or as given to
    /// [`with_map_datum`](Self::with_map_datum).
    ///
    /// The text is cleaned as every text value of the metadata is: cut at
    /// the first NUL and trimmed of spaces at both ends. `None` when the
    /// file does not record it, when it is blank, as the Nikon Z 9 writes
    /// it, or longer than 255 bytes, or not of type ASCII.
    pub fn map_datum(&self) -> Option<&str> {
        self.map_datum.as_deref()
    }

    /// This location with the altitude `altitude`, in place of any set
    /// before. [`altitude`](Self::altitude) reads it back.
    #[must_use]
    pub fn with_altitude(mut self, altitude: Altitude) -> Self {
        self.altitude = Some(altitude);
        self
    }

    /// This location with the GPS time `time`, in place of any set
    /// before. [`time`](Self::time) reads it back.
    ///
    /// The time is stored as given; the readers give it in UTC, with an
    /// offset of zero, and a test that builds one through
    /// [`DateTime::with_offset_minutes`]`(0)` gets the same value.
    #[must_use]
    pub fn with_time(mut self, time: DateTime) -> Self {
        self.time = Some(time);
        self
    }

    /// This location with the image direction `direction`, in place of
    /// any set before. [`image_direction`](Self::image_direction) reads it
    /// back.
    #[must_use]
    pub fn with_image_direction(mut self, direction: Direction) -> Self {
        self.image_direction = Some(direction);
        self
    }

    /// This location with the map datum `datum`, in place of any set
    /// before. [`map_datum`](Self::map_datum) reads it back.
    ///
    /// The text is stored as given: it is not trimmed or otherwise
    /// changed, though the readers clean a file's text before they store
    /// it. It may be a `&str`, which is copied, or a `String`, which is
    /// moved.
    #[must_use]
    pub fn with_map_datum(mut self, datum: impl Into<String>) -> Self {
        self.map_datum = Some(datum.into());
        self
    }
}

/// How far north or south of the equator a picture was taken: degrees,
/// minutes and seconds as the file records them, and the hemisphere.
///
/// Exif writes each of the three as a fraction, and makers use them
/// differently: degrees and minutes with four decimals and no seconds
/// (`48/1, 514933/10000, 0/1`), as Nikon does, or degrees, minutes and
/// seconds with three decimals (`48/1, 51/1, 29600/1000`), as Sony does.
/// The fractions are kept as written; [`to_degrees`](Self::to_degrees)
/// gives the decimal degrees a map takes.
///
/// [`north`](Self::north) and [`south`](Self::south) build one and refuse
/// three fractions that name no latitude.
///
/// # Equality
///
/// Two latitudes are equal when their hemisphere and their three fractions
/// are the same as recorded, as for [`Rational`]: minutes of 30/1 are not
/// equal to 300000/10000, and 0° north is not equal to 0° south. There is
/// no `Ord`, since an order by angle would disagree with that equality;
/// sort by [`to_degrees`](Self::to_degrees) to order by angle. There is no
/// `Display` either: a program formats degrees as it shows them.
///
/// # Examples
///
/// The latitude of the Eiffel Tower, as a Sony camera would write it:
///
/// ```
/// use raw_files::{Latitude, Rational, ValueError};
///
/// # fn main() -> Result<(), ValueError> {
/// let latitude = Latitude::north(
///     Rational::new(48, 1)?,
///     Rational::new(51, 1)?,
///     Rational::new(29_600, 1000)?,
/// )?;
/// assert!(!latitude.is_south());
/// assert_eq!(latitude.seconds(), Rational::new(29_600, 1000)?);
/// assert!((latitude.to_degrees() - 48.858_22).abs() < 1e-5);
/// // Minutes of 60 name no latitude.
/// assert_eq!(
///     Latitude::north(Rational::new(48, 1)?, Rational::new(60, 1)?, Rational::new(0, 1)?),
///     Err(ValueError::ArcMinutesOutOfRange)
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Latitude {
    angle: Angle,
    south: bool,
}

impl Latitude {
    /// The latitude `degrees`, `minutes` and `seconds` north of the
    /// equator, kept as given.
    ///
    /// Zero is a real value of each: 0° is the equator, and 0 seconds is
    /// what Nikon writes beside minutes with decimals. 90° 0′ 0″ is the
    /// pole, the most there is.
    ///
    /// # Errors
    ///
    /// In the order they are checked:
    ///
    /// - [`ZeroDenominator`](ValueError::ZeroDenominator) when one of the
    ///   three fractions has a zero denominator;
    /// - [`ArcMinutesOutOfRange`](ValueError::ArcMinutesOutOfRange) when
    ///   `minutes` is 60 or more;
    /// - [`ArcSecondsOutOfRange`](ValueError::ArcSecondsOutOfRange) when
    ///   `seconds` is 60 or more;
    /// - [`LatitudeOutOfRange`](ValueError::LatitudeOutOfRange) when the
    ///   three together come to more than 90°.
    pub fn north(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
    ) -> Result<Self, ValueError> {
        let angle = Angle::latitude(degrees, minutes, seconds)?;
        Ok(Self {
            angle,
            south: false,
        })
    }

    /// The latitude `degrees`, `minutes` and `seconds` south of the
    /// equator, kept as given.
    ///
    /// # Errors
    ///
    /// As for [`north`](Self::north).
    pub fn south(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
    ) -> Result<Self, ValueError> {
        let angle = Angle::latitude(degrees, minutes, seconds)?;
        Ok(Self { angle, south: true })
    }

    /// The degrees, as recorded; never negative, since the hemisphere is
    /// [`is_south`](Self::is_south).
    pub fn degrees(&self) -> Rational {
        self.angle.degrees
    }

    /// The minutes of arc, as recorded, below 60.
    pub fn minutes(&self) -> Rational {
        self.angle.minutes
    }

    /// The seconds of arc, as recorded, below 60.
    pub fn seconds(&self) -> Rational {
        self.angle.seconds
    }

    /// Whether the latitude lies south of the equator: the file's
    /// GPSLatitudeRef `S`, or a latitude built by [`south`](Self::south).
    pub fn is_south(&self) -> bool {
        self.south
    }

    /// The latitude in decimal degrees, negative to the south, as a map
    /// takes it: the degrees, plus the minutes over 60 and the seconds
    /// over 3600. 48° 51.4933′ north gives about 48.85822.
    pub fn to_degrees(self) -> f64 {
        signed(self.angle.to_degrees(), self.south)
    }
}

/// How far east or west of the prime meridian a picture was taken:
/// degrees, minutes and seconds as the file records them, and the
/// hemisphere.
///
/// It is kept as [`Latitude`] is, as written, and
/// [`to_degrees`](Self::to_degrees) gives decimal degrees. [`east`](Self::east)
/// and [`west`](Self::west) build one and refuse three fractions that name
/// no longitude.
///
/// # Equality
///
/// As for [`Latitude`]: the hemisphere and the three fractions as
/// recorded, no `Ord` and no `Display`.
///
/// # Examples
///
/// The longitude of the Statue of Liberty, west of the prime meridian, in
/// degrees and minutes to four decimals:
///
/// ```
/// use raw_files::{Longitude, Rational};
///
/// # fn main() -> Result<(), raw_files::ValueError> {
/// let longitude = Longitude::west(
///     Rational::new(74, 1)?,
///     Rational::new(26_670, 10_000)?,
///     Rational::new(0, 1)?,
/// )?;
/// assert!(longitude.is_west());
/// assert!((longitude.to_degrees() + 74.044_45).abs() < 1e-5);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Longitude {
    angle: Angle,
    west: bool,
}

impl Longitude {
    /// The longitude `degrees`, `minutes` and `seconds` east of the prime
    /// meridian, kept as given.
    ///
    /// Zero is a real value of each, as for [`Latitude::north`]; 180° 0′
    /// 0″ is the antimeridian, the most there is.
    ///
    /// # Errors
    ///
    /// In the order they are checked:
    ///
    /// - [`ZeroDenominator`](ValueError::ZeroDenominator) when one of the
    ///   three fractions has a zero denominator;
    /// - [`ArcMinutesOutOfRange`](ValueError::ArcMinutesOutOfRange) when
    ///   `minutes` is 60 or more;
    /// - [`ArcSecondsOutOfRange`](ValueError::ArcSecondsOutOfRange) when
    ///   `seconds` is 60 or more;
    /// - [`LongitudeOutOfRange`](ValueError::LongitudeOutOfRange) when the
    ///   three together come to more than 180°.
    pub fn east(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
    ) -> Result<Self, ValueError> {
        let angle = Angle::longitude(degrees, minutes, seconds)?;
        Ok(Self { angle, west: false })
    }

    /// The longitude `degrees`, `minutes` and `seconds` west of the prime
    /// meridian, kept as given.
    ///
    /// # Errors
    ///
    /// As for [`east`](Self::east).
    pub fn west(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
    ) -> Result<Self, ValueError> {
        let angle = Angle::longitude(degrees, minutes, seconds)?;
        Ok(Self { angle, west: true })
    }

    /// The degrees, as recorded; never negative, since the hemisphere is
    /// [`is_west`](Self::is_west).
    pub fn degrees(&self) -> Rational {
        self.angle.degrees
    }

    /// The minutes of arc, as recorded, below 60.
    pub fn minutes(&self) -> Rational {
        self.angle.minutes
    }

    /// The seconds of arc, as recorded, below 60.
    pub fn seconds(&self) -> Rational {
        self.angle.seconds
    }

    /// Whether the longitude lies west of the prime meridian: the file's
    /// GPSLongitudeRef `W`, or a longitude built by [`west`](Self::west).
    pub fn is_west(&self) -> bool {
        self.west
    }

    /// The longitude in decimal degrees, negative to the west, as a map
    /// takes it: the degrees, plus the minutes over 60 and the seconds
    /// over 3600.
    pub fn to_degrees(self) -> f64 {
        signed(self.angle.to_degrees(), self.west)
    }
}

/// How high a picture was taken: meters as the file records them, and
/// whether they lie above or below the reference.
///
/// Exif records the meters as one fraction and the side in
/// GPSAltitudeRef: 0 and 2 above, 1 and 3 below. Which surface they are
/// measured from is not given: Exif called 0 and 1 sea level until its
/// version 2.32, and version 3.0 made them the reference ellipsoid and
/// added 2 and 3 for sea level, but a file does not say which rule its
/// writer followed.
///
/// # Equality
///
/// Two altitudes are equal when their side and their fraction are the
/// same as recorded, as for [`Rational`]. There is no `Ord`; sort by
/// [`to_meters`](Self::to_meters) to order by height.
///
/// # Examples
///
/// The shore of the Dead Sea, below sea level:
///
/// ```
/// use raw_files::{Altitude, Rational};
///
/// # fn main() -> Result<(), raw_files::ValueError> {
/// let shore = Altitude::below(Rational::new(43_050, 100)?)?;
/// assert!(shore.is_below());
/// assert_eq!(shore.meters(), Rational::new(43_050, 100)?);
/// assert_eq!(shore.to_meters(), -430.5);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Altitude {
    meters: Rational,
    below: bool,
}

impl Altitude {
    /// `meters` above the reference, kept as given. Zero meters is a real
    /// altitude.
    ///
    /// # Errors
    ///
    /// [`ZeroDenominator`](ValueError::ZeroDenominator) when the
    /// denominator of `meters` is zero.
    pub fn above(meters: Rational) -> Result<Self, ValueError> {
        Ok(Self {
            meters: checked(meters)?,
            below: false,
        })
    }

    /// `meters` below the reference, kept as given.
    ///
    /// # Errors
    ///
    /// [`ZeroDenominator`](ValueError::ZeroDenominator) when the
    /// denominator of `meters` is zero.
    pub fn below(meters: Rational) -> Result<Self, ValueError> {
        Ok(Self {
            meters: checked(meters)?,
            below: true,
        })
    }

    /// The meters from the reference, as recorded; never negative, since
    /// the side is [`is_below`](Self::is_below).
    pub fn meters(&self) -> Rational {
        self.meters
    }

    /// Whether the altitude lies below the reference: the file's
    /// GPSAltitudeRef 1 or 3, or an altitude built by
    /// [`below`](Self::below).
    pub fn is_below(&self) -> bool {
        self.below
    }

    /// The altitude in meters as a decimal, negative below the reference.
    pub fn to_meters(self) -> f64 {
        signed(self.meters.to_f64(), self.below)
    }
}

/// A bearing, such as the direction the camera faced: degrees clockwise
/// from north as the file records them, and which north they count from.
///
/// Exif records the degrees as one fraction, from 0 to 359.99, and the
/// north in a reference letter, `T` for true north and `M` for magnetic
/// north.
///
/// # Equality
///
/// Two directions are equal when their north and their fraction are the
/// same as recorded, as for [`Rational`]. There is no `Ord`, since a
/// bearing turns round to its start.
///
/// # Examples
///
/// ```
/// use raw_files::{Direction, North, Rational, ValueError};
///
/// # fn main() -> Result<(), ValueError> {
/// let facing = Direction::new(Rational::new(27_550, 100)?, North::Magnetic)?;
/// assert_eq!(facing.degrees().to_f64(), 275.5);
/// assert_eq!(facing.north(), North::Magnetic);
/// // A full turn is no bearing: 360° is 0°.
/// assert_eq!(
///     Direction::new(Rational::new(360, 1)?, North::True),
///     Err(ValueError::DirectionOutOfRange)
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Direction {
    degrees: Rational,
    north: North,
}

impl Direction {
    /// `degrees` clockwise from `north`, kept as given. Zero is due north,
    /// a real bearing.
    ///
    /// # Errors
    ///
    /// In the order they are checked:
    ///
    /// - [`ZeroDenominator`](ValueError::ZeroDenominator) when the
    ///   denominator of `degrees` is zero;
    /// - [`DirectionOutOfRange`](ValueError::DirectionOutOfRange) when the
    ///   angle is 360° or more.
    pub fn new(degrees: Rational, north: North) -> Result<Self, ValueError> {
        let degrees = checked(degrees)?;
        if u64::from(degrees.numerator) >= 360 * u64::from(degrees.denominator) {
            return Err(ValueError::DirectionOutOfRange);
        }
        Ok(Self { degrees, north })
    }

    /// The degrees clockwise from [`north`](Self::north), as recorded,
    /// below 360.
    pub fn degrees(&self) -> Rational {
        self.degrees
    }

    /// The north the degrees count from.
    pub fn north(&self) -> North {
        self.north
    }
}

/// The north a [`Direction`] counts from: Exif's two, true and magnetic.
///
/// `North` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "True north" and
/// "Magnetic north".
///
/// # Examples
///
/// ```
/// use raw_files::North;
///
/// assert_eq!(North::True.name(), "True north");
/// assert_eq!(North::Magnetic.to_string(), "Magnetic north");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum North {
    /// The geographic north pole, where the meridians meet: Exif's `T`.
    True,
    /// Where a compass points: Exif's `M`.
    Magnetic,
}

impl North {
    /// The north's name, for an information panel: "True north" or
    /// "Magnetic north".
    pub fn name(self) -> &'static str {
        match self {
            North::True => "True north",
            North::Magnetic => "Magnetic north",
        }
    }
}

impl fmt::Display for North {
    /// Writes [`name`](North::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Degrees, minutes and seconds, each a fraction as recorded, that
/// [`Angle::new`] has checked: no zero denominator, minutes and seconds
/// below 60, and no more than its largest angle in all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Angle {
    degrees: Rational,
    minutes: Rational,
    seconds: Rational,
}

impl Angle {
    /// A latitude's angle: at most 90°, by the rules of [`new`](Self::new).
    fn latitude(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
    ) -> Result<Self, ValueError> {
        Self::new(
            degrees,
            minutes,
            seconds,
            90,
            ValueError::LatitudeOutOfRange,
        )
    }

    /// A longitude's angle: at most 180°, by the rules of [`new`](Self::new).
    fn longitude(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
    ) -> Result<Self, ValueError> {
        Self::new(
            degrees,
            minutes,
            seconds,
            180,
            ValueError::LongitudeOutOfRange,
        )
    }

    /// The angle `degrees`, `minutes` and `seconds`, checked in the order
    /// [`Latitude::north`] documents: the three denominators, then the
    /// minutes, the seconds, and `beyond` when the three together come to
    /// more than `max` degrees.
    fn new(
        degrees: Rational,
        minutes: Rational,
        seconds: Rational,
        max: u32,
        beyond: ValueError,
    ) -> Result<Self, ValueError> {
        let (degrees, minutes, seconds) = (checked(degrees)?, checked(minutes)?, checked(seconds)?);
        let below_60 =
            |part: Rational| u64::from(part.numerator) < 60 * u64::from(part.denominator);
        if !below_60(minutes) {
            return Err(ValueError::ArcMinutesOutOfRange);
        }
        if !below_60(seconds) {
            return Err(ValueError::ArcSecondsOutOfRange);
        }
        // degrees + minutes / 60 + seconds / 3600 <= max, multiplied out by
        // 3600 and the three denominators so that the test is exact: each
        // product is at most 3600 × 180 × 2^96, well inside a u128.
        let (dn, dd) = (
            u128::from(degrees.numerator),
            u128::from(degrees.denominator),
        );
        let (mn, md) = (
            u128::from(minutes.numerator),
            u128::from(minutes.denominator),
        );
        let (sn, sd) = (
            u128::from(seconds.numerator),
            u128::from(seconds.denominator),
        );
        let angle = 3600 * dn * md * sd + 60 * mn * dd * sd + sn * dd * md;
        let largest = 3600 * u128::from(max) * dd * md * sd;
        if angle > largest {
            return Err(beyond);
        }
        Ok(Self {
            degrees,
            minutes,
            seconds,
        })
    }

    /// The angle in decimal degrees.
    fn to_degrees(self) -> f64 {
        self.degrees.to_f64() + self.minutes.to_f64() / 60.0 + self.seconds.to_f64() / 3600.0
    }
}

/// `fraction` when [`Rational::new`] takes it, the one function that
/// decides whether a fraction is a number, so that a struct literal with a
/// zero denominator is refused as `new` refuses one.
fn checked(fraction: Rational) -> Result<Rational, ValueError> {
    Rational::new(fraction.numerator, fraction.denominator)
}

/// `value`, negated when `negative`. Only a value above zero is negated,
/// so that 0° south or 0 m below gives 0.0, not -0.0, which would print
/// with a minus sign.
fn signed(value: f64, negative: bool) -> f64 {
    if negative && value > 0.0 {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(numerator: u32, denominator: u32) -> Rational {
        Rational {
            numerator,
            denominator,
        }
    }

    /// The Eiffel Tower's latitude as Nikon writes a position: degrees,
    /// minutes with four decimals, no seconds.
    fn tower_latitude() -> Latitude {
        Latitude::north(r(48, 1), r(514_933, 10_000), r(0, 1)).unwrap()
    }

    fn tower_longitude() -> Longitude {
        Longitude::east(r(2, 1), r(176_700, 10_000), r(0, 1)).unwrap()
    }

    #[test]
    fn a_coordinate_keeps_its_fractions_as_recorded() {
        let latitude = tower_latitude();
        assert_eq!(latitude.degrees(), r(48, 1));
        assert_eq!(latitude.minutes(), r(514_933, 10_000));
        assert_eq!(latitude.seconds(), r(0, 1));
        assert!(!latitude.is_south());
        let longitude = Longitude::west(r(74, 1), r(2, 1), r(40_002, 1000)).unwrap();
        assert_eq!(longitude.degrees(), r(74, 1));
        assert_eq!(longitude.minutes(), r(2, 1));
        assert_eq!(longitude.seconds(), r(40_002, 1000));
        assert!(longitude.is_west());
    }

    #[test]
    fn a_coordinate_with_a_zero_denominator_is_refused() {
        for (d, m, s) in [
            (r(48, 0), r(0, 1), r(0, 1)),
            // 0/0 degrees would pass the range test on its own.
            (r(0, 0), r(0, 1), r(0, 1)),
            (r(48, 1), r(0, 0), r(0, 1)),
            (r(48, 1), r(0, 1), r(0, 0)),
            (r(0, 0), r(0, 0), r(0, 0)),
        ] {
            let refused = ValueError::ZeroDenominator;
            assert_eq!(Latitude::north(d, m, s), Err(refused), "{d} {m} {s}");
            assert_eq!(Latitude::south(d, m, s), Err(refused), "{d} {m} {s}");
            assert_eq!(Longitude::east(d, m, s), Err(refused), "{d} {m} {s}");
            assert_eq!(Longitude::west(d, m, s), Err(refused), "{d} {m} {s}");
        }
    }

    #[test]
    fn minutes_and_seconds_must_be_below_60() {
        // 59.9999 minutes and 59.999 seconds are the most there are.
        assert!(Latitude::north(r(10, 1), r(599_999, 10_000), r(0, 1)).is_ok());
        assert!(Latitude::north(r(10, 1), r(0, 1), r(59_999, 1000)).is_ok());
        assert!(Longitude::east(r(10, 1), r(59, 1), r(59_999, 1000)).is_ok());
        let minutes = ValueError::ArcMinutesOutOfRange;
        let seconds = ValueError::ArcSecondsOutOfRange;
        for (m, s, err) in [
            (r(60, 1), r(0, 1), minutes),
            (r(600_000, 10_000), r(0, 1), minutes),
            (r(0, 1), r(60, 1), seconds),
            (r(0, 1), r(60_000, 1000), seconds),
            (r(u32::MAX, 1), r(0, 1), minutes),
        ] {
            assert_eq!(Latitude::north(r(10, 1), m, s), Err(err), "{m} {s}");
            assert_eq!(Longitude::west(r(10, 1), m, s), Err(err), "{m} {s}");
        }
    }

    #[test]
    fn a_coordinate_reports_the_first_rule_broken_in_its_documented_order() {
        // Every rule broken, then one rule fewer each time: the
        // denominators, the minutes, the seconds, the whole angle.
        let cases = [
            (r(500, 1), r(60, 1), r(60, 0), ValueError::ZeroDenominator),
            (
                r(500, 1),
                r(60, 1),
                r(60, 1),
                ValueError::ArcMinutesOutOfRange,
            ),
            (
                r(500, 1),
                r(0, 1),
                r(60, 1),
                ValueError::ArcSecondsOutOfRange,
            ),
        ];
        for (d, m, s, err) in cases {
            assert_eq!(Latitude::north(d, m, s), Err(err), "{d} {m} {s}");
            assert_eq!(Latitude::south(d, m, s), Err(err), "{d} {m} {s}");
            assert_eq!(Longitude::east(d, m, s), Err(err), "{d} {m} {s}");
            assert_eq!(Longitude::west(d, m, s), Err(err), "{d} {m} {s}");
        }
        assert_eq!(
            Latitude::south(r(500, 1), r(0, 1), r(0, 1)),
            Err(ValueError::LatitudeOutOfRange)
        );
        assert_eq!(
            Longitude::west(r(500, 1), r(0, 1), r(0, 1)),
            Err(ValueError::LongitudeOutOfRange)
        );
    }

    #[test]
    fn a_latitude_goes_up_to_90_degrees() {
        assert!(Latitude::north(r(90, 1), r(0, 1), r(0, 1)).is_ok());
        assert!(Latitude::south(r(900_000, 10_000), r(0, 1), r(0, 1)).is_ok());
        assert!(Latitude::north(r(89, 1), r(59, 1), r(60_000, 1001)).is_ok());
        let refused = Err(ValueError::LatitudeOutOfRange);
        assert_eq!(Latitude::north(r(90, 1), r(0, 1), r(1, 1)), refused);
        assert_eq!(Latitude::south(r(90, 1), r(1, 10_000), r(0, 1)), refused);
        assert_eq!(
            Latitude::north(r(900_001, 10_000), r(0, 1), r(0, 1)),
            refused
        );
        assert_eq!(Latitude::north(r(91, 1), r(0, 1), r(0, 1)), refused);
        assert_eq!(Latitude::north(r(u32::MAX, 1), r(0, 1), r(0, 1)), refused);
        // 180 degrees is a longitude, not a latitude.
        assert_eq!(Latitude::north(r(180, 1), r(0, 1), r(0, 1)), refused);
    }

    #[test]
    fn a_longitude_goes_up_to_180_degrees() {
        assert!(Longitude::east(r(180, 1), r(0, 1), r(0, 1)).is_ok());
        assert!(Longitude::west(r(179, 1), r(599_999, 10_000), r(0, 1)).is_ok());
        let refused = Err(ValueError::LongitudeOutOfRange);
        assert_eq!(Longitude::east(r(180, 1), r(0, 1), r(1, 1000)), refused);
        assert_eq!(Longitude::west(r(180, 1), r(1, 1), r(0, 1)), refused);
        assert_eq!(Longitude::east(r(181, 1), r(0, 1), r(0, 1)), refused);
        // The largest fractions: the products stay exact.
        assert_eq!(
            Longitude::east(r(u32::MAX, u32::MAX), r(59, 1), r(59, 1)),
            Ok(Longitude {
                angle: Angle {
                    degrees: r(u32::MAX, u32::MAX),
                    minutes: r(59, 1),
                    seconds: r(59, 1),
                },
                west: false,
            })
        );
        assert_eq!(Longitude::east(r(u32::MAX, 1), r(0, 1), r(0, 1)), refused);
    }

    #[test]
    fn zero_is_a_real_coordinate() {
        let equator = Latitude::north(r(0, 1), r(0, 1), r(0, 1)).unwrap();
        assert_eq!(equator.to_degrees(), 0.0);
        let meridian = Longitude::west(r(0, 1), r(0, 1), r(0, 1)).unwrap();
        assert_eq!(meridian.to_degrees(), 0.0);
        // Zero to the south or west, or below, is no negative zero: it
        // prints without a minus sign.
        let south = Latitude::south(r(0, 1), r(0, 1), r(0, 1)).unwrap();
        for zero in [
            south.to_degrees(),
            meridian.to_degrees(),
            Altitude::below(r(0, 1)).unwrap().to_meters(),
        ] {
            assert!(zero.is_sign_positive(), "{zero}");
            assert_eq!(format!("{zero:.5}"), "0.00000");
        }
        // Recorded with another hemisphere, it is another value.
        assert_ne!(Latitude::south(r(0, 1), r(0, 1), r(0, 1)), Ok(equator));
    }

    #[test]
    fn decimal_degrees_are_negative_to_the_south_and_west() {
        let north = tower_latitude().to_degrees();
        assert!((north - 48.858_221_666).abs() < 1e-9, "{north}");
        let south = Latitude::south(r(33, 1), r(51, 1), r(31_000, 1000)).unwrap();
        assert!((south.to_degrees() + 33.858_611_111).abs() < 1e-9);
        let east = tower_longitude().to_degrees();
        assert!((east - 2.294_5).abs() < 1e-9, "{east}");
        let west = Longitude::west(r(74, 1), r(2, 1), r(40_002, 1000)).unwrap();
        assert!((west.to_degrees() + 74.044_445).abs() < 1e-9);
    }

    #[test]
    fn equality_is_of_what_is_recorded() {
        let thirty = Latitude::north(r(10, 1), r(30, 1), r(0, 1)).unwrap();
        let decimals = Latitude::north(r(10, 1), r(300_000, 10_000), r(0, 1)).unwrap();
        assert_eq!(thirty.to_degrees(), decimals.to_degrees());
        assert_ne!(thirty, decimals);
        assert_ne!(
            Latitude::north(r(10, 1), r(30, 1), r(0, 1)),
            Latitude::south(r(10, 1), r(30, 1), r(0, 1))
        );
        assert_ne!(
            Altitude::above(r(10, 1)),
            Altitude::above(r(100, 10)),
            "meters as recorded"
        );
    }

    #[test]
    fn an_altitude_keeps_its_side() {
        let above = Altitude::above(r(3505, 100)).unwrap();
        assert_eq!(above.meters(), r(3505, 100));
        assert!(!above.is_below());
        assert_eq!(above.to_meters(), 35.05);
        let below = Altitude::below(r(4305, 100)).unwrap();
        assert!(below.is_below());
        assert_eq!(below.to_meters(), -43.05);
        // Zero meters is a real altitude, on either side.
        assert_eq!(Altitude::above(r(0, 1)).unwrap().to_meters(), 0.0);
        assert!(Altitude::below(r(0, 1)).is_ok());
        assert_eq!(Altitude::above(r(35, 0)), Err(ValueError::ZeroDenominator));
        assert_eq!(Altitude::below(r(0, 0)), Err(ValueError::ZeroDenominator));
    }

    #[test]
    fn a_direction_is_below_360_degrees() {
        let facing = Direction::new(r(35_999, 100), North::True).unwrap();
        assert_eq!(facing.degrees(), r(35_999, 100));
        assert_eq!(facing.north(), North::True);
        assert!(Direction::new(r(0, 1), North::Magnetic).is_ok());
        let refused = Err(ValueError::DirectionOutOfRange);
        assert_eq!(Direction::new(r(360, 1), North::True), refused);
        assert_eq!(Direction::new(r(36_000, 100), North::Magnetic), refused);
        assert_eq!(Direction::new(r(u32::MAX, 1), North::True), refused);
        assert_eq!(
            Direction::new(r(90, 0), North::True),
            Err(ValueError::ZeroDenominator)
        );
        // 360/0 breaks both rules: the denominator is checked first.
        assert_eq!(
            Direction::new(r(360, 0), North::True),
            Err(ValueError::ZeroDenominator)
        );
    }

    #[test]
    fn north_has_a_name() {
        assert_eq!(North::True.name(), "True north");
        assert_eq!(North::Magnetic.name(), "Magnetic north");
        assert_eq!(North::Magnetic.to_string(), "Magnetic north");
        assert!(North::True < North::Magnetic);
    }

    #[test]
    fn a_location_starts_from_its_position() {
        let location = Location::new(tower_latitude(), tower_longitude());
        assert_eq!(location.latitude(), tower_latitude());
        assert_eq!(location.longitude(), tower_longitude());
        assert_eq!(location.altitude(), None);
        assert_eq!(location.time(), None);
        assert_eq!(location.image_direction(), None);
        assert_eq!(location.map_datum(), None);
    }

    #[test]
    fn each_builder_sets_its_part() {
        let altitude = Altitude::above(r(35, 1)).unwrap();
        let time = DateTime::new(2026, 6, 21, 10, 15, 30)
            .unwrap()
            .with_offset_minutes(0)
            .unwrap();
        let direction = Direction::new(r(275, 1), North::Magnetic).unwrap();
        let location = Location::new(tower_latitude(), tower_longitude())
            .with_altitude(altitude)
            .with_time(time)
            .with_image_direction(direction)
            .with_map_datum("WGS-84");
        assert_eq!(location.altitude(), Some(altitude));
        assert_eq!(location.time(), Some(time));
        assert_eq!(location.image_direction(), Some(direction));
        assert_eq!(location.map_datum(), Some("WGS-84"));
        // A later value replaces an earlier one, and a text is stored as
        // given.
        let below = Altitude::below(r(2, 1)).unwrap();
        let location = location.with_altitude(below).with_map_datum(" TOKYO ");
        assert_eq!(location.altitude(), Some(below));
        assert_eq!(location.map_datum(), Some(" TOKYO "));
    }
}
