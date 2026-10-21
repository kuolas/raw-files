// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the camera recorded about a picture.
//!
//! Besides the image, a camera records how the picture was taken: by which
//! camera and when, the exposure it chose, the lens it was taken with, how
//! large the image is, and where the camera focused. [`Metadata`] holds all
//! of it, read when the file is opened; [`Exposure`] and [`Lens`] hold the
//! exposure and the lens as the file records them, fractions with the
//! numerator and denominator it stored, and [`Focus`] where the camera's
//! autofocus aimed, as the camera recorded it. [`LensBuilder`] builds a
//! lens a value at a time, for a caller's tests.
//!
//! Every value is optional. A file may not record it, or may record it in a
//! way that cannot be read, such as a tag of the wrong type; either way the
//! value is `None`, and the others are unaffected.
//!
//! A program that uses raw-files can build these values for its own tests,
//! without a camera file for each case. [`Metadata`] and [`Exposure`] start
//! empty, from `default()`, and take each value through a `with_…` method
//! named after the method that reads it back, such as
//! [`Metadata::with_model`] for [`Metadata::model`]. A [`Lens`] is built
//! through [`Lens::builder`], and a [`Focus`], a [`DateTime`] and a
//! [`Rational`] through their checked constructors. The `with_…` methods
//! store each value as given, so a test reads back what it wrote, and what
//! it does not set stays `None`:
//!
//! ```
//! use raw_files::{DateTime, Exposure, Lens, Metadata, Rational};
//!
//! # fn main() {
//! #     example().expect("every step gives a value");
//! # }
//! # fn example() -> Option<()> {
//! let time = DateTime::new(2022, 2, 5, 13, 46, 41)?
//!     .with_subseconds(850_000_000, 2)?
//!     .with_offset_minutes(-300)?;
//! let exposure = Exposure::default()
//!     .with_iso(64)
//!     .with_exposure_time(Rational::new(10, 3200)?);
//! let metadata = Metadata::default()
//!     .with_model("NIKON Z 9")
//!     .with_capture_time(time)
//!     .with_exposure(exposure)
//!     .with_lens(Lens::builder().model("NIKKOR Z 14-30mm f/4 S").build()?);
//!
//! assert_eq!(metadata.model(), Some("NIKON Z 9"));
//! assert_eq!(
//!     metadata.capture_time().map(|time| time.to_string()).as_deref(),
//!     Some("2022-02-05T13:46:41.85-05:00")
//! );
//! assert_eq!(metadata.exposure().iso(), Some(64));
//! assert_eq!(metadata.exposure().exposure_time(), Rational::new(10, 3200));
//! assert_eq!(
//!     metadata.lens().and_then(Lens::model),
//!     Some("NIKKOR Z 14-30mm f/4 S")
//! );
//! assert_eq!(metadata.serial_number(), None);
//! assert_eq!(metadata.exposure().f_number(), None);
//! # Some(())
//! # }
//! ```

use crate::datetime::DateTime;
use crate::geometry::{Point, Size};
use crate::orientation::Orientation;
use crate::rational::{Rational, SignedRational};

/// What the camera recorded about a picture: the camera, when the picture
/// was taken, how to turn it upright, its exposure and lens, the sizes of
/// its raw data and of its visible image, and where the camera focused.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read. A damaged value costs only itself, and a
/// damaged directory only the values in it: the rest is still read, and
/// the file still opens with its previews.
///
/// `Metadata::default()` is an empty one, with every value `None` and the
/// orientation [`NoTransforms`](Orientation::NoTransforms). A caller's
/// tests start from it and set the values they need with the `with_…`
/// methods, such as [`with_model`](Self::with_model), each named after the
/// method that reads the value back and each replacing any value set
/// before. The builder pattern of the Rust API Guidelines (C-BUILDER)
/// uses a separate builder type, but `Metadata` needs none: every value is
/// optional, so the empty `Metadata` is already a valid start for every
/// chain. A [`Lens`], which has no empty value, is built through a
/// [`LensBuilder`] instead.
///
/// The `with_…` methods store each value as given: none refuses or changes
/// one. The docs of the methods that read a value say what the readers give
/// for a file, such as a make cleaned of padding; a test gets back what it
/// wrote. `Metadata` is not `Copy`, so that a later value that cannot be
/// copied does not break callers.
///
/// # Examples
///
/// Every value is optional, so each is handled on its own; a file that
/// lacks one still gives the others:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// let metadata = file.metadata();
/// println!("camera: {}", metadata.model().unwrap_or("unknown"));
/// println!(
///     "serial number: {}",
///     metadata.serial_number().unwrap_or("not recorded")
/// );
/// if let Some(size) = metadata.image_size() {
///     println!("{} × {} pixels", size.width, size.height);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Metadata {
    pub(crate) make: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) serial_number: Option<String>,
    pub(crate) capture_time: Option<DateTime>,
    pub(crate) orientation: Orientation,
    pub(crate) exposure: Exposure,
    pub(crate) lens: Option<Lens>,
    pub(crate) raw_size: Option<Size>,
    pub(crate) image_size: Option<Size>,
    pub(crate) focus: Option<Focus>,
}

impl Metadata {
    /// The camera's maker, such as `NIKON CORPORATION`: as the file records
    /// it, cleaned of padding by the reader, or as given to
    /// [`with_make`](Self::with_make).
    pub fn make(&self) -> Option<&str> {
        self.make.as_deref()
    }

    /// The camera's model, such as `NIKON Z 9`: as the file records it,
    /// cleaned of padding by the reader, or as given to
    /// [`with_model`](Self::with_model).
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The camera body's serial number, as text: as the file records it,
    /// cleaned of padding by the reader, or as given to
    /// [`with_serial_number`](Self::with_serial_number). Many bodies do not
    /// record one in a form the library reads, and then it is `None`.
    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }

    /// When the picture was taken, as the camera's clock showed it: the
    /// Exif DateTimeOriginal, with its fraction of a second and its offset
    /// from UTC when the file records them (see [`DateTime`]). The offset
    /// is the Exif one or, on older Nikon bodies, which record none, the
    /// time-zone setting of the camera's clock.
    ///
    /// Both are what the camera was set to. A clock that was never set
    /// gives its default date, which the library cannot tell from a real
    /// one: the public Nikon 1 J1 sample 2956 records 2010-01-01 00:11,
    /// with its time-zone setting of +02:00.
    pub fn capture_time(&self) -> Option<DateTime> {
        self.capture_time
    }

    /// What a viewer must do to the stored pixels to show the picture
    /// upright. For a file, it is the same value the previews carry
    /// ([`Preview::orientation`](crate::Preview::orientation)), and
    /// [`NoTransforms`](Orientation::NoTransforms) when the file records
    /// none.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// The exposure the picture was taken with. Always present, though each
    /// of its values is optional.
    pub fn exposure(&self) -> &Exposure {
        &self.exposure
    }

    /// The lens the picture was taken with, or `None` when the file
    /// describes no lens.
    pub fn lens(&self) -> Option<&Lens> {
        self.lens.as_ref()
    }

    /// The size of the raw data as stored, before any orientation is
    /// applied.
    ///
    /// It can include masked border pixels and padding beyond the visible
    /// image. On a camera's medium or small raw it is that smaller stored
    /// size, not the sensor's.
    pub fn raw_size(&self) -> Option<Size> {
        self.raw_size
    }

    /// The size of the visible image as stored, before any orientation is
    /// applied.
    ///
    /// It is the default crop the file records, when that lies inside the
    /// raw data: DNG's DefaultCropSize, which Sony writes, or Nikon's crop
    /// area. Otherwise it is the [raw size](Self::raw_size), which then
    /// includes the border pixels: older Nikon bodies, before the D850,
    /// record no crop, and on the public samples their stored raw is up to
    /// 64 pixels wider and 24 taller than the camera's own full-size
    /// preview.
    ///
    /// It is the default crop, not an aspect ratio chosen in the camera. On
    /// a Nikon small raw it counts the stored pixels, which can differ from
    /// the size the camera's menu names. For a file, it is `None` when there
    /// is no raw size.
    pub fn image_size(&self) -> Option<Size> {
        self.image_size
    }

    /// Where the camera focused, as the file records it (see [`Focus`]):
    /// the camera's own record only, never an estimate from the picture.
    ///
    /// It is read on Nikon files, from the MakerNote's AFInfo2 block, as the
    /// camera recorded it. The exception is a Nikon crop, any that
    /// CropHiSpeed records with a CropArea: there the point is placed inside
    /// the cropped picture, by the sides the crop cuts, as the
    /// [NEF format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md)
    /// describe. DX, 1:1 and 16:9 are the checked cases. It is `None` on a Nikon file that records no position,
    /// such as a DSLR shot through the viewfinder, which records only an
    /// autofocus point's number, and on a manual-focus shot. It is `None`
    /// too when the AFInfo2 version is one raw-files does not know, and on
    /// a crop whose point cannot be placed on the cropped picture: a 1:1 or
    /// 16:9 crop on a small raw, or a crop whose CropArea cannot be used.
    ///
    /// On Sony files it is read from the MakerNote's FocusLocation, with
    /// the area's size from its FocusFrameSize, on the bodies from the
    /// ILCE-7RM2 on, as the camera recorded it and as the
    /// [ARW format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md)
    /// describe. Its frame is the picture as the camera framed it, which
    /// differs from the image size on a picture taken at another aspect
    /// ratio than the sensor's, and can on a medium or small raw: the
    /// ILCE-7M4 records the full size there, the ILCE-9M3 the stored size.
    /// It is `None` on a manual-focus shot and on the older bodies, which
    /// record only an autofocus point's number or nothing at all, and its
    /// [`in_focus`](Focus::in_focus) is always `None`: Sony records it in
    /// no plain tag.
    pub fn focus(&self) -> Option<Focus> {
        self.focus
    }

    /// Sets the camera's maker, in place of any set before;
    /// [`make`](Self::make) reads it.
    #[must_use]
    pub fn with_make(mut self, make: impl Into<String>) -> Self {
        self.make = Some(make.into());
        self
    }

    /// Sets the camera's model, in place of any set before;
    /// [`model`](Self::model) reads it.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the camera body's serial number, in place of any set before;
    /// [`serial_number`](Self::serial_number) reads it.
    #[must_use]
    pub fn with_serial_number(mut self, serial_number: impl Into<String>) -> Self {
        self.serial_number = Some(serial_number.into());
        self
    }

    /// Sets when the picture was taken, in place of any time set before;
    /// [`capture_time`](Self::capture_time) reads it.
    #[must_use]
    pub fn with_capture_time(mut self, time: DateTime) -> Self {
        self.capture_time = Some(time);
        self
    }

    /// Sets how to turn the picture upright, in place of the orientation
    /// it had; [`orientation`](Self::orientation) reads it.
    #[must_use]
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the exposure, in place of the one it had;
    /// [`exposure`](Self::exposure) reads it.
    #[must_use]
    pub fn with_exposure(mut self, exposure: Exposure) -> Self {
        self.exposure = exposure;
        self
    }

    /// Sets the lens, in place of any set before; [`lens`](Self::lens)
    /// reads it.
    #[must_use]
    pub fn with_lens(mut self, lens: Lens) -> Self {
        self.lens = Some(lens);
        self
    }

    /// Sets the size of the raw data, in place of any set before;
    /// [`raw_size`](Self::raw_size) reads it.
    #[must_use]
    pub fn with_raw_size(mut self, size: Size) -> Self {
        self.raw_size = Some(size);
        self
    }

    /// Sets the size of the visible image, in place of any set before;
    /// [`image_size`](Self::image_size) reads it.
    #[must_use]
    pub fn with_image_size(mut self, size: Size) -> Self {
        self.image_size = Some(size);
        self
    }

    /// Sets where the camera focused, in place of any focus set before;
    /// [`focus`](Self::focus) reads it.
    #[must_use]
    pub fn with_focus(mut self, focus: Focus) -> Self {
        self.focus = Some(focus);
        self
    }
}

/// The exposure of a picture: its sensitivity, how long and how wide the
/// lens was open, at what focal length, and the compensation chosen.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read. A zero ISO, exposure time, f-number or
/// focal length means the camera did not know it, and the readers never
/// return one: the value is `None`, unless, for the ISO, another of the
/// file's sensitivity tags gives it. A zero exposure bias is a real value.
///
/// `Exposure::default()` is an empty one, with every value `None`. A
/// caller's tests start from it and set the values they need with the
/// `with_…` methods, such as [`with_iso`](Self::with_iso), each named after
/// the method that reads the value back and each replacing any value set
/// before. As for [`Metadata`], every value is optional, so the empty
/// `Exposure` is the start of every chain and there is no separate builder
/// type.
///
/// The `with_…` methods store each value as given: none refuses or changes
/// one, so a test reads back even a zero it wrote, such as an ISO of 0 or
/// an f-number of 0/10, though the readers never return a zero ISO,
/// exposure time, f-number or focal length. `Exposure` is not `Copy`, so
/// that a later value that cannot be copied does not break callers.
///
/// # Examples
///
/// The exposure in lowest terms, such as `1/320 s, f/9, ISO 64` from the
/// fractions as stored (`10/3200` and `900/100`):
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// let exposure = file.metadata().exposure();
/// if let (Some(time), Some(f_number), Some(iso)) = (
///     exposure.exposure_time(),
///     exposure.f_number(),
///     exposure.iso(),
/// ) {
///     println!("{} s, f/{}, ISO {iso}", time.reduced(), f_number.to_f64());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Exposure {
    pub(crate) iso: Option<u32>,
    pub(crate) exposure_time: Option<Rational>,
    pub(crate) f_number: Option<Rational>,
    pub(crate) focal_length: Option<Rational>,
    pub(crate) exposure_bias: Option<SignedRational>,
}

impl Exposure {
    /// The ISO sensitivity, such as 64.
    ///
    /// For a file, 65535 means 65535 or more: Exif records that value for
    /// any higher sensitivity, and it is kept when the file records no
    /// exact one.
    pub fn iso(&self) -> Option<u32> {
        self.iso
    }

    /// How long the shutter was open, in seconds, as stored, such as
    /// 10/3200 for 1/320 s.
    pub fn exposure_time(&self) -> Option<Rational> {
        self.exposure_time
    }

    /// The f-number the picture was taken at, as stored, such as 900/100
    /// for f/9.
    pub fn f_number(&self) -> Option<Rational> {
        self.f_number
    }

    /// The focal length the picture was taken at, in millimetres, as
    /// stored: the lens's actual focal length, not its 35 mm equivalent.
    pub fn focal_length(&self) -> Option<Rational> {
        self.focal_length
    }

    /// The exposure compensation chosen, in EV, as stored, such as -2/3.
    /// A zero, such as 0/6, means no compensation, not an unknown one.
    pub fn exposure_bias(&self) -> Option<SignedRational> {
        self.exposure_bias
    }

    /// Sets the ISO sensitivity, in place of any set before;
    /// [`iso`](Self::iso) reads it.
    #[must_use]
    pub fn with_iso(mut self, iso: u32) -> Self {
        self.iso = Some(iso);
        self
    }

    /// Sets how long the shutter was open, in seconds, in place of any time
    /// set before; [`exposure_time`](Self::exposure_time) reads it.
    #[must_use]
    pub fn with_exposure_time(mut self, seconds: Rational) -> Self {
        self.exposure_time = Some(seconds);
        self
    }

    /// Sets the f-number, in place of any set before;
    /// [`f_number`](Self::f_number) reads it.
    #[must_use]
    pub fn with_f_number(mut self, f_number: Rational) -> Self {
        self.f_number = Some(f_number);
        self
    }

    /// Sets the focal length, in millimetres, in place of any set before;
    /// [`focal_length`](Self::focal_length) reads it.
    #[must_use]
    pub fn with_focal_length(mut self, millimetres: Rational) -> Self {
        self.focal_length = Some(millimetres);
        self
    }

    /// Sets the exposure compensation, in EV, in place of any set before;
    /// [`exposure_bias`](Self::exposure_bias) reads it.
    #[must_use]
    pub fn with_exposure_bias(mut self, ev: SignedRational) -> Self {
        self.exposure_bias = Some(ev);
        self
    }
}

/// The lens a picture was taken with, as the file describes it: its name
/// and serial number, and its range of focal lengths with the widest
/// aperture at each end.
///
/// A prime lens has the same shortest and longest focal length. A camera
/// with a fixed lens may describe it by its ranges alone, with no model.
/// At least one value is always present: a file that describes no lens has
/// no `Lens`.
///
/// Focal lengths are in millimetres, the lens's actual ones, not their 35 mm
/// equivalents; apertures are f-numbers. Both are fractions as stored, and
/// a zero, which a file writes for a value it does not know, is read as
/// `None`.
///
/// A `Lens` comes from a file, or a caller builds one a value at a time
/// with [`Lens::builder`] for its tests.
///
/// # Examples
///
/// The lens's name and its range of focal lengths, such as
/// `NIKKOR Z 14-30mm f/4 S, 14-30 mm`:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// if let Some(lens) = file.metadata().lens() {
///     let name = lens.model().unwrap_or("a lens without a name");
///     match (lens.min_focal_length(), lens.max_focal_length()) {
///         (Some(min), Some(max)) => {
///             println!("{name}, {}-{} mm", min.to_f64(), max.to_f64());
///         }
///         _ => println!("{name}"),
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lens {
    make: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    min_focal_length: Option<Rational>,
    max_focal_length: Option<Rational>,
    f_number_at_min_focal_length: Option<Rational>,
    f_number_at_max_focal_length: Option<Rational>,
}

impl Lens {
    /// An empty [`LensBuilder`], to build a lens a value at a time for a
    /// caller's tests. Its [`build`](LensBuilder::build) gives `None` until
    /// a value is set.
    #[must_use]
    pub fn builder() -> LensBuilder {
        LensBuilder::default()
    }

    /// The lens with these values; `None` when every one of them is `None`,
    /// since a lens is described by at least one.
    ///
    /// This is the one place that rule is checked: for
    /// [`LensBuilder::build`], and for the readers, which call it with the
    /// texts they read, moved rather than copied. The texts are stored as
    /// given, not trimmed or otherwise changed: the readers clean the text
    /// a file records before they build a lens, such as by trimming its
    /// spaces or by leaving out a model made only of `-` characters. The
    /// four fractions are in the order the Exif LensSpecification tag
    /// stores them: the shortest and longest focal length, then the
    /// f-number at each.
    pub(crate) fn from_parts(
        make: Option<String>,
        model: Option<String>,
        serial_number: Option<String>,
        min_focal_length: Option<Rational>,
        max_focal_length: Option<Rational>,
        f_number_at_min_focal_length: Option<Rational>,
        f_number_at_max_focal_length: Option<Rational>,
    ) -> Option<Self> {
        let lens = Self {
            make,
            model,
            serial_number,
            min_focal_length,
            max_focal_length,
            f_number_at_min_focal_length,
            f_number_at_max_focal_length,
        };
        let empty = lens.make.is_none()
            && lens.model.is_none()
            && lens.serial_number.is_none()
            && lens.min_focal_length.is_none()
            && lens.max_focal_length.is_none()
            && lens.f_number_at_min_focal_length.is_none()
            && lens.f_number_at_max_focal_length.is_none();
        (!empty).then_some(lens)
    }

    /// Whether the lens has a range: whether any of its four fractions, the
    /// shortest and longest focal length and the f-number at each, is
    /// known.
    pub(crate) fn has_range(&self) -> bool {
        self.min_focal_length.is_some()
            || self.max_focal_length.is_some()
            || self.f_number_at_min_focal_length.is_some()
            || self.f_number_at_max_focal_length.is_some()
    }

    /// `lens` with its range replaced by `range`, its make, model and
    /// serial number kept, or, when there is no `lens`, a lens of the range
    /// alone: for a format whose own structures record a range the standard
    /// tags lack, such as Nikon's MakerNote. `None` when that leaves no
    /// value, as for [`from_parts`](Self::from_parts); the texts are moved,
    /// not copied.
    ///
    /// The four fractions are in the order
    /// [`from_parts`](Self::from_parts) takes them.
    pub(crate) fn with_range(lens: Option<Self>, range: [Option<Rational>; 4]) -> Option<Self> {
        let (make, model, serial_number) = match lens {
            Some(lens) => (lens.make, lens.model, lens.serial_number),
            None => (None, None, None),
        };
        let [
            min_focal_length,
            max_focal_length,
            f_number_at_min_focal_length,
            f_number_at_max_focal_length,
        ] = range;
        Self::from_parts(
            make,
            model,
            serial_number,
            min_focal_length,
            max_focal_length,
            f_number_at_min_focal_length,
            f_number_at_max_focal_length,
        )
    }

    /// The name of the lens's maker.
    pub fn make(&self) -> Option<&str> {
        self.make.as_deref()
    }

    /// The lens's model name, such as `NIKKOR Z 14-30mm f/4 S`. In a file,
    /// a name made only of `-` characters, which Sony writes for a manual
    /// lens, names no lens and is read as `None`.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The lens's serial number, as the text the file records.
    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }

    /// The shortest focal length of the lens, in millimetres, as stored,
    /// such as 140/10 for 14 mm.
    pub fn min_focal_length(&self) -> Option<Rational> {
        self.min_focal_length
    }

    /// The longest focal length of the lens, in millimetres, as stored;
    /// equal to the shortest on a prime lens.
    pub fn max_focal_length(&self) -> Option<Rational> {
        self.max_focal_length
    }

    /// The widest aperture at the shortest focal length, as an f-number
    /// stored as a fraction, such as 400/100 for f/4.
    pub fn f_number_at_min_focal_length(&self) -> Option<Rational> {
        self.f_number_at_min_focal_length
    }

    /// The widest aperture at the longest focal length, as an f-number
    /// stored as a fraction; on a lens whose aperture does not change as
    /// it zooms, the same as at the shortest.
    pub fn f_number_at_max_focal_length(&self) -> Option<Rational> {
        self.f_number_at_max_focal_length
    }
}

/// Builds a [`Lens`] a value at a time, for a caller's tests.
///
/// A lens is described by at least one value, so there is no empty `Lens`
/// to start from and change, as there is an empty [`Metadata`] or
/// [`Exposure`]. A builder starts empty instead, from [`Lens::builder`].
/// Each of its methods sets one value, in place of any it had, and is
/// named after the [`Lens`] method that reads that value back;
/// [`build`](Self::build) then gives the lens, or `None` when no value was
/// set.
///
/// The values are stored as given. A text is not trimmed or otherwise
/// changed, and a model made only of `-` characters is kept, though the
/// readers leave such a model out of a file: they clean a file's text
/// before they build a lens, and a test gets back what it wrote. A text
/// may be a `&str`, which is copied, or a `String`, which is moved.
///
/// # Examples
///
/// A camera with a fixed 9-72 mm f/2.8-4.5 lens, which its files describe
/// by the range alone; a lens described by its name alone; and a builder
/// given no value, which builds no lens:
///
/// ```
/// use raw_files::{Lens, Rational};
///
/// # fn main() {
/// #     example().expect("every step gives a value");
/// # }
/// # fn example() -> Option<()> {
/// let fixed = Lens::builder()
///     .min_focal_length(Rational::new(90, 10)?)
///     .max_focal_length(Rational::new(720, 10)?)
///     .f_number_at_min_focal_length(Rational::new(28, 10)?)
///     .f_number_at_max_focal_length(Rational::new(45, 10)?)
///     .build()?;
/// assert_eq!(fixed.model(), None);
/// assert_eq!(fixed.min_focal_length(), Rational::new(90, 10));
/// assert_eq!(fixed.max_focal_length(), Rational::new(720, 10));
/// assert_eq!(fixed.f_number_at_min_focal_length(), Rational::new(28, 10));
/// assert_eq!(fixed.f_number_at_max_focal_length(), Rational::new(45, 10));
///
/// let named = Lens::builder().model("NIKKOR Z 14-30mm f/4 S").build()?;
/// assert_eq!(named.model(), Some("NIKKOR Z 14-30mm f/4 S"));
/// assert_eq!(named.min_focal_length(), None);
///
/// assert_eq!(Lens::builder().build(), None);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LensBuilder {
    make: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    min_focal_length: Option<Rational>,
    max_focal_length: Option<Rational>,
    f_number_at_min_focal_length: Option<Rational>,
    f_number_at_max_focal_length: Option<Rational>,
}

impl LensBuilder {
    /// Sets the name of the lens's maker, in place of any set before;
    /// [`Lens::make`] reads it.
    #[must_use]
    pub fn make(mut self, make: impl Into<String>) -> Self {
        self.make = Some(make.into());
        self
    }

    /// Sets the lens's model name, such as `NIKKOR Z 14-30mm f/4 S`, in
    /// place of any set before; [`Lens::model`] reads it.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the lens's serial number, in place of any set before;
    /// [`Lens::serial_number`] reads it.
    #[must_use]
    pub fn serial_number(mut self, serial_number: impl Into<String>) -> Self {
        self.serial_number = Some(serial_number.into());
        self
    }

    /// Sets the shortest focal length of the lens, in millimetres, in place
    /// of any set before; [`Lens::min_focal_length`] reads it.
    #[must_use]
    pub fn min_focal_length(mut self, millimetres: Rational) -> Self {
        self.min_focal_length = Some(millimetres);
        self
    }

    /// Sets the longest focal length of the lens, in millimetres, in place
    /// of any set before; [`Lens::max_focal_length`] reads it.
    #[must_use]
    pub fn max_focal_length(mut self, millimetres: Rational) -> Self {
        self.max_focal_length = Some(millimetres);
        self
    }

    /// Sets the widest aperture at the shortest focal length, as an
    /// f-number, in place of any set before;
    /// [`Lens::f_number_at_min_focal_length`] reads it.
    #[must_use]
    pub fn f_number_at_min_focal_length(mut self, f_number: Rational) -> Self {
        self.f_number_at_min_focal_length = Some(f_number);
        self
    }

    /// Sets the widest aperture at the longest focal length, as an
    /// f-number, in place of any set before;
    /// [`Lens::f_number_at_max_focal_length`] reads it.
    #[must_use]
    pub fn f_number_at_max_focal_length(mut self, f_number: Rational) -> Self {
        self.f_number_at_max_focal_length = Some(f_number);
        self
    }

    /// The lens with the values set; `None` when no value was set, since a
    /// lens is described by at least one.
    #[must_use]
    pub fn build(self) -> Option<Lens> {
        Lens::from_parts(
            self.make,
            self.model,
            self.serial_number,
            self.min_focal_length,
            self.max_focal_length,
            self.f_number_at_min_focal_length,
            self.f_number_at_max_focal_length,
        )
    }
}

/// Where the camera's autofocus aimed, as the camera recorded it: the
/// centre of the focus area, the area's size when the file records one,
/// the frame the two are measured in, and whether the camera confirmed
/// focus, when the file says.
///
/// It is the camera's record, never an estimate from the pixels, and not
/// proof of where the picture is sharpest: a body that focuses in front of
/// or behind the subject, or a subject that moved, puts the sharpest area
/// elsewhere.
///
/// # Coordinates
///
/// The centre is a [`Point`], and the frame and the area are [`Size`]s,
/// all in pixels and in the orientation of the stored image, like every
/// size and preview the library reports: a viewer that rotates a preview
/// to show it upright rotates the point with it.
///
/// The centre is a distance from the frame's left and top edges, so the
/// middle of the frame is at (width / 2, height / 2), as the cameras
/// record it: a Nikon Coolpix A focusing at the middle of its 4928 × 3264
/// frame records (2464, 1632), as on the public sample 2238. An area
/// `w` × `h` centred at (x, y) spans from x - w / 2 to x + w / 2 across
/// and from y - h / 2 to y + h / 2 down. The values are whole numbers, as
/// the cameras store them; to place the focus in fractions of the frame,
/// divide x and the area's width by the [`frame`](Self::frame)'s width,
/// and y and the area's height by its height.
///
/// The frame has the framing of the previews, at whatever scale the file
/// records. To place the focus on a preview, scale x and the width by the
/// preview's width over the frame's, and y and the height by the heights;
/// [`scaled_to`](Self::scaled_to) does it. Do not assume that the frame
/// equals [`Metadata::image_size`]. They differ where the camera's framing
/// differs from the default crop, as on a Sony picture taken at another
/// aspect ratio than the sensor's; where the stored raw is a smaller copy
/// of the camera's output, as on a small raw; and on the older Nikon
/// bodies that record no crop area, whose image size includes the border
/// pixels.
///
/// # Which files record one
///
/// Nikon mirrorless bodies, Nikon DSLRs in live view, and Sony bodies from
/// the ILCE-7RM2 on record a position. A DSLR shot through the viewfinder
/// and older bodies record only the number of an autofocus point, and a
/// Nikon mirrorless body in some auto-area modes only a mask of the points
/// it used, which the library does not map to pixels; a shot in manual
/// focus records none that means anything. Those files give no `Focus`.
///
/// On Nikon files, [`Metadata::focus`] is read from the MakerNote's AFInfo2
/// block, as the camera recorded it but placed inside the cropped picture
/// on a crop, and is `None` for a manual-focus shot.
/// On Sony files it is read from the MakerNote's FocusLocation and
/// FocusFrameSize, on the bodies from the ILCE-7RM2 on, in the frame the
/// camera framed the picture in: the picture at an aspect-ratio setting,
/// and, on a medium or small raw, the full size or the stored size, as
/// the body chooses. It is `None` for a manual-focus shot and for the
/// bodies that record only a point's number, and its
/// [`in_focus`](Self::in_focus) is always `None`.
///
/// # Equality
///
/// Two values are equal when every value is the same as stored. There is
/// no order: no order of focus records means anything, and sorting them
/// would suggest one. There is no `Default` either, since there is no
/// empty focus: a file that records none gives `None`.
///
/// # Examples
///
/// A Nikon Z 9's record, built a step at a time, the centre first, then
/// the area and whether focus was confirmed, and placed on a 1920 × 1280
/// preview:
///
/// ```
/// use raw_files::{Focus, Point, Size};
///
/// # fn main() {
/// #     example().expect("every step accepts the Z 9's values");
/// # }
/// # fn example() -> Option<()> {
/// let frame = Size { width: 8256, height: 5504 };
/// let focus = Focus::new(frame, Point { x: 4905, y: 2461 })?
///     .with_size(Size { width: 291, height: 323 })?
///     .with_in_focus(true);
/// let on_preview = focus.scaled_to(Size { width: 1920, height: 1280 })?;
/// assert_eq!(on_preview.center(), Point { x: 1141, y: 572 });
/// assert_eq!(on_preview.size(), Some(Size { width: 68, height: 75 }));
/// assert_eq!(on_preview.frame(), Size { width: 1920, height: 1280 });
/// assert_eq!(on_preview.in_focus(), Some(true));
/// # Some(())
/// # }
/// ```
///
/// Where the camera focused on a file's largest preview:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// let focus = file.metadata().focus();
/// if let (Some(focus), Some(preview)) = (focus, file.largest_preview()) {
///     if let Some(focus) = focus.scaled_to(preview.size()) {
///         let centre = focus.center();
///         println!("focused at ({}, {}) on the preview", centre.x, centre.y);
///         if let Some(size) = focus.size() {
///             println!("on an area of {} × {} pixels", size.width, size.height);
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Focus {
    frame: Size,
    center: Point,
    size: Option<Size>,
    in_focus: Option<bool>,
}

impl Focus {
    /// The focus centred at `center`, measured in `frame`, with no area and
    /// nothing said about whether the camera confirmed focus; `None` when
    /// the centre cannot lie on the frame. [`with_size`](Self::with_size)
    /// adds the area and [`with_in_focus`](Self::with_in_focus) whether
    /// focus was confirmed.
    ///
    /// This is how the readers start every focus they return, and how a
    /// caller starts one for its tests. It gives `None` when:
    ///
    /// - either side of `frame` is zero;
    /// - `center` lies outside the frame: its x is more than the frame's
    ///   width, or its y more than its height. The edges are on the frame,
    ///   so a centre at x = width, on its right edge, is accepted.
    #[must_use]
    pub fn new(frame: Size, center: Point) -> Option<Self> {
        if frame.width == 0 || frame.height == 0 {
            return None;
        }
        let center_on_frame = center.x <= frame.width && center.y <= frame.height;
        center_on_frame.then_some(Self {
            frame,
            center,
            size: None,
            in_focus: None,
        })
    }

    /// This focus with an area of `size` around its centre, in place of
    /// any area it had; `None` when the area cannot lie on the frame
    /// around the centre, which is when:
    ///
    /// - either side of `size` is zero;
    /// - the area does not fit inside the frame around its centre. An area
    ///   `w` pixels wide centred at x spans from x - w / 2 to x + w / 2,
    ///   which must lie between 0 and the frame's width; in whole numbers,
    ///   `w` is at most 2 × x and at most 2 × (width - x). The same holds
    ///   down, for the height, y and the frame's height. So an area 10
    ///   pixels wide fits a centre from x = 5 to x = width - 5, and one 11
    ///   pixels wide, whose half is 5.5, from x = 6 to x = width - 6.
    ///
    /// A focus is `Copy`, and a refused area leaves the one it was offered
    /// to as it was, so whoever builds a focus chooses what an area that
    /// cannot be used costs. `Focus::new(frame, center)?.with_size(size)?`
    /// gives no focus at all, and `focus.with_size(size).unwrap_or(focus)`
    /// keeps the point without an area. The readers choose by what the
    /// file means by an area with a zero side: no focus where the format
    /// writes it to say that it recorded no position, and the point alone
    /// where it writes it to say only that it recorded no size.
    #[must_use]
    pub fn with_size(self, size: Size) -> Option<Self> {
        let size_fits = size.width != 0
            && size.height != 0
            && fits(self.center.x, size.width, self.frame.width)
            && fits(self.center.y, size.height, self.frame.height);
        size_fits.then_some(Self {
            size: Some(size),
            ..self
        })
    }

    /// This focus with whether the camera confirmed focus, in place of
    /// what it had: `true` when the file records that the camera found
    /// focus, and `false` when it records a miss.
    ///
    /// A focus built without it says nothing, and its
    /// [`in_focus`](Self::in_focus) is `None`, as for a file that does not
    /// say. Once set, it can be replaced but not taken back to `None`.
    #[must_use]
    pub fn with_in_focus(self, in_focus: bool) -> Self {
        Self {
            in_focus: Some(in_focus),
            ..self
        }
    }

    /// The size of the frame the centre and the area are measured in. It
    /// never has a zero side.
    pub fn frame(&self) -> Size {
        self.frame
    }

    /// The centre of the focus area, as its distances from the left and top
    /// edges of the frame. It always lies within the frame, from 0 to the
    /// frame's width across and from 0 to its height down, the edges
    /// included.
    ///
    /// A caller reading the pixel at the centre takes the pixel at column x
    /// and row y; for a centre on the right edge, x = width, that is the
    /// last column, x - 1, and likewise the last row, y - 1, for a centre
    /// on the bottom edge, y = height.
    pub fn center(&self) -> Point {
        self.center
    }

    /// The width and height of the focus area, or `None` when the file
    /// records no size.
    ///
    /// It never has a zero side, and it fits inside the frame around the
    /// centre: across, from x - width / 2 to x + width / 2 lies between 0
    /// and the frame's width, so the width is at most 2 × x and at most
    /// 2 × (frame width - x); down, the same holds for the height, y and
    /// the frame's height.
    pub fn size(&self) -> Option<Size> {
        self.size
    }

    /// Whether the camera confirmed focus: `Some(true)` when it records that
    /// it found focus, `Some(false)` when it records a miss, and `None` when
    /// the file does not say.
    pub fn in_focus(&self) -> Option<bool> {
        self.in_focus
    }

    /// The same focus measured in `frame`, such as a preview's size: `None`
    /// when either side of `frame` is zero, and otherwise always a focus,
    /// whose frame is `frame` and whose [`in_focus`](Self::in_focus) is
    /// this one's.
    ///
    /// The centre's x and the area's width are multiplied by the new width
    /// over the old, and y and the height by the heights, each rounded to
    /// the nearest pixel, a half rounding up, so the centre stays within the
    /// new frame, from 0 to its width and height. An area that rounds to
    /// nothing keeps a side of one pixel.
    ///
    /// Rounding can leave the area too large to fit around the centre.
    /// The centre is kept where it rounded to, and the area is trimmed on
    /// that axis, by as little as it takes, to fit by the rule of
    /// [`with_size`](Self::with_size). A centre on an edge of the frame
    /// leaves no room for an area on that axis, and the size becomes
    /// `None`; on a frame only a few pixels across, rounding often puts the
    /// centre there.
    ///
    /// Each call rounds to whole pixels. To place the focus on several
    /// frames, scale it from the focus as read each time, not from one
    /// already scaled, so that it is not rounded twice.
    #[must_use]
    pub fn scaled_to(&self, frame: Size) -> Option<Self> {
        if frame.width == 0 || frame.height == 0 {
            return None;
        }
        let center = Point {
            x: scale(self.center.x, self.frame.width, frame.width),
            y: scale(self.center.y, self.frame.height, frame.height),
        };
        // The old centre is at most the old side, so the scaled one is at
        // most the new side: rounding cannot pass it, as the old side
        // scales to exactly the new one. The frame has no zero side either,
        // so this never gives `None`.
        let mut focus = Self::new(frame, center)?;
        if let Some(size) = self.size {
            let width = scale(size.width, self.frame.width, frame.width).max(1);
            let height = scale(size.height, self.frame.height, frame.height).max(1);
            let area = Size {
                width: trim(width, center.x, frame.width),
                height: trim(height, center.y, frame.height),
            };
            // The trimmed area fits, so `with_size` refuses it only for the
            // zero side a centre on an edge leaves, and the focus then keeps
            // its point without an area.
            focus = focus.with_size(area).unwrap_or(focus);
        }
        if let Some(in_focus) = self.in_focus {
            focus = focus.with_in_focus(in_focus);
        }
        Some(focus)
    }
}

/// Whether an area `side` pixels across, centred at `center`, lies inside
/// a frame `frame` pixels across, on one axis: from `center - side / 2` to
/// `center + side / 2` lies between 0 and `frame`, that is, `side` is at
/// most `2 * center` and at most `2 * (frame - center)`. Half the side is
/// rounded up, which in whole numbers is the same test. Summed in 64 bits,
/// so that no `u32` can overflow it.
fn fits(center: u32, side: u32, frame: u32) -> bool {
    let half = side.div_ceil(2);
    half <= center && u64::from(center) + u64::from(half) <= u64::from(frame)
}

/// `value`, measured on an axis `old` pixels long, measured on one `new`
/// pixels long instead: multiplied by `new` over `old` and rounded to the
/// nearest whole number, a half rounding up.
///
/// The product and the half of `old` added to it fit in 64 bits for any
/// `u32`s. `old` is a side of a [`Focus`]'s frame, which is never zero. A
/// result too large for a `u32`, which a value on the old frame cannot
/// give, saturates.
fn scale(value: u32, old: u32, new: u32) -> u32 {
    let old = u64::from(old);
    let scaled = (u64::from(value) * u64::from(new) + old / 2) / old;
    u32::try_from(scaled).unwrap_or(u32::MAX)
}

/// `side`, shortened as little as it takes for an area centred at `center`
/// to fit a frame `frame` pixels across by the rule of [`fits`]: to no more
/// than twice the smaller of `center` and `frame - center`, the centre's
/// distances from the two edges. Zero when the centre is on either edge,
/// at 0 or at `frame`, where no area fits.
fn trim(side: u32, center: u32, frame: u32) -> u32 {
    let room = 2 * u64::from(center.min(frame.saturating_sub(center)));
    u32::try_from(u64::from(side).min(room)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fraction(numerator: u32, denominator: u32) -> Rational {
        Rational {
            numerator,
            denominator,
        }
    }

    /// An exposure of 1/320 s at f/9, 14 mm and ISO 64, with -2/3 EV of
    /// compensation, as a camera stores it.
    fn z9_exposure() -> Exposure {
        Exposure {
            iso: Some(64),
            exposure_time: Some(fraction(10, 3200)),
            f_number: Some(fraction(900, 100)),
            focal_length: Some(fraction(140, 10)),
            exposure_bias: Some(SignedRational {
                numerator: -4,
                denominator: 6,
            }),
        }
    }

    #[test]
    fn default_exposure_is_empty() {
        let exposure = Exposure::default();
        assert_eq!(exposure.iso(), None);
        assert_eq!(exposure.exposure_time(), None);
        assert_eq!(exposure.f_number(), None);
        assert_eq!(exposure.focal_length(), None);
        assert_eq!(exposure.exposure_bias(), None);
    }

    #[test]
    fn exposure_getters_return_what_was_set() {
        let exposure = z9_exposure();
        assert_eq!(exposure.iso(), Some(64));
        assert_eq!(exposure.exposure_time(), Some(fraction(10, 3200)));
        assert_eq!(exposure.f_number(), Some(fraction(900, 100)));
        assert_eq!(exposure.focal_length(), Some(fraction(140, 10)));
        assert_eq!(
            exposure.exposure_bias(),
            Some(SignedRational {
                numerator: -4,
                denominator: 6,
            })
        );
    }

    #[test]
    fn exposure_equality_is_as_stored() {
        let reduced = Exposure {
            exposure_time: Some(fraction(1, 320)),
            ..z9_exposure()
        };
        assert_ne!(z9_exposure(), reduced);
        assert_eq!(z9_exposure(), z9_exposure());
    }

    #[test]
    fn a_lens_from_parts_with_no_value_is_none() {
        // The readers' way to build a lens.
        assert_eq!(
            Lens::from_parts(None, None, None, None, None, None, None),
            None
        );
    }

    #[test]
    fn an_empty_lens_builder_builds_no_lens() {
        assert_eq!(Lens::builder().build(), None);
    }

    #[test]
    fn each_lens_setter_alone_builds_a_lens_of_that_value_only() {
        let value = fraction(1, 1);
        let builders = [
            Lens::builder().make("x"),
            Lens::builder().model("x"),
            Lens::builder().serial_number("x"),
            Lens::builder().min_focal_length(value),
            Lens::builder().max_focal_length(value),
            Lens::builder().f_number_at_min_focal_length(value),
            Lens::builder().f_number_at_max_focal_length(value),
        ];
        for (place, builder) in builders.into_iter().enumerate() {
            let lens = builder
                .build()
                .unwrap_or_else(|| panic!("setter {place} alone gave no lens"));
            let texts = [lens.make(), lens.model(), lens.serial_number()];
            let fractions = [
                lens.min_focal_length(),
                lens.max_focal_length(),
                lens.f_number_at_min_focal_length(),
                lens.f_number_at_max_focal_length(),
            ];
            // Only the value set is there, so each setter writes its own
            // and each getter reads its own.
            let expected_texts: Vec<_> = (0..3).map(|i| (i == place).then_some("x")).collect();
            let expected_fractions: Vec<_> =
                (3..7).map(|i| (i == place).then_some(value)).collect();
            assert_eq!(texts.to_vec(), expected_texts, "setter {place}");
            assert_eq!(fractions.to_vec(), expected_fractions, "setter {place}");
        }
    }

    #[test]
    fn a_fixed_lens_described_by_its_ranges_alone() {
        let lens = Lens::builder()
            .min_focal_length(fraction(90, 10))
            .max_focal_length(fraction(720, 10))
            .f_number_at_min_focal_length(fraction(28, 10))
            .f_number_at_max_focal_length(fraction(45, 10))
            .build()
            .unwrap();
        assert_eq!(lens.model(), None);
        assert_eq!(lens.min_focal_length(), Some(fraction(90, 10)));
        assert_eq!(lens.max_focal_length(), Some(fraction(720, 10)));
        assert_eq!(lens.f_number_at_min_focal_length(), Some(fraction(28, 10)));
        assert_eq!(lens.f_number_at_max_focal_length(), Some(fraction(45, 10)));
    }

    #[test]
    fn a_lens_has_a_range_when_any_of_its_four_fractions_is_known() {
        let value = fraction(1, 1);
        // A lens named but with no range, as the Nikon 1 J1's Exif tags
        // describe it (public sample 2956).
        let named = Lens::builder().model("x").build().unwrap();
        assert!(!named.has_range());
        let ranges = [
            Lens::builder().min_focal_length(value),
            Lens::builder().max_focal_length(value),
            Lens::builder().f_number_at_min_focal_length(value),
            Lens::builder().f_number_at_max_focal_length(value),
        ];
        for (place, builder) in ranges.into_iter().enumerate() {
            assert!(builder.build().unwrap().has_range(), "value {place}");
        }
    }

    #[test]
    fn a_lens_with_a_range_keeps_its_text_and_takes_the_new_range() {
        let range = [
            Some(fraction(100, 10)),
            Some(fraction(300, 10)),
            Some(fraction(35, 10)),
            Some(fraction(56, 10)),
        ];
        let named = Lens::builder()
            .make("Nikon")
            .model("1 NIKKOR VR 10-30mm f/3.5-5.6")
            .serial_number("20012345")
            .min_focal_length(fraction(1, 1))
            .build();
        let lens = Lens::with_range(named, range).unwrap();
        assert_eq!(lens.make(), Some("Nikon"));
        assert_eq!(lens.model(), Some("1 NIKKOR VR 10-30mm f/3.5-5.6"));
        assert_eq!(lens.serial_number(), Some("20012345"));
        assert_eq!(lens.min_focal_length(), range[0]);
        assert_eq!(lens.max_focal_length(), range[1]);
        assert_eq!(lens.f_number_at_min_focal_length(), range[2]);
        assert_eq!(lens.f_number_at_max_focal_length(), range[3]);

        // No lens and a range: a lens of the range alone.
        let alone = Lens::with_range(None, range).unwrap();
        assert_eq!((alone.make(), alone.model()), (None, None));
        assert_eq!(alone.serial_number(), None);
        assert_eq!(alone.min_focal_length(), range[0]);
        assert_eq!(alone.f_number_at_max_focal_length(), range[3]);

        // A range of nothing on a lens of a range alone, or on no lens,
        // leaves no lens.
        let bare = Lens::builder().min_focal_length(fraction(1, 1)).build();
        assert_eq!(Lens::with_range(bare, [None; 4]), None);
        assert_eq!(Lens::with_range(None, [None; 4]), None);
    }

    /// A NIKKOR Z 14-30mm f/4 S, every value set, the fractions as a Z 9
    /// stores them.
    fn z_lens() -> Lens {
        Lens::builder()
            .make("Nikon")
            .model("NIKKOR Z 14-30mm f/4 S")
            .serial_number("20012345")
            .min_focal_length(fraction(140, 10))
            .max_focal_length(fraction(300, 10))
            .f_number_at_min_focal_length(fraction(400, 100))
            .f_number_at_max_focal_length(fraction(400, 100))
            .build()
            .unwrap()
    }

    #[test]
    fn a_z_lens_reads_back_exactly() {
        let lens = z_lens();
        assert_eq!(lens.make(), Some("Nikon"));
        assert_eq!(lens.model(), Some("NIKKOR Z 14-30mm f/4 S"));
        assert_eq!(lens.serial_number(), Some("20012345"));
        assert_eq!(lens.min_focal_length(), Some(fraction(140, 10)));
        assert_eq!(lens.max_focal_length(), Some(fraction(300, 10)));
        assert_eq!(
            lens.f_number_at_min_focal_length(),
            Some(fraction(400, 100))
        );
        assert_eq!(
            lens.f_number_at_max_focal_length(),
            Some(fraction(400, 100))
        );
        // As stored, not reduced: 140/10 is not 14/1.
        assert_ne!(lens.min_focal_length(), Some(fraction(14, 1)));
    }

    #[test]
    fn a_lens_builder_keeps_its_texts_as_given() {
        // Neither trimmed nor left out, as the readers would a file's.
        let lens = Lens::builder()
            .make("  padded  ")
            .model("----")
            .serial_number("")
            .build()
            .unwrap();
        assert_eq!(lens.make(), Some("  padded  "));
        assert_eq!(lens.model(), Some("----"));
        assert_eq!(lens.serial_number(), Some(""));
    }

    #[test]
    fn a_lens_builder_moves_an_owned_text_rather_than_copy_it() {
        let make = String::from("Nikon");
        let model = String::from("NIKKOR Z 14-30mm f/4 S");
        let serial_number = String::from("20012345");
        let addresses = [make.as_ptr(), model.as_ptr(), serial_number.as_ptr()];
        let lens = Lens::builder()
            .make(make)
            .model(model)
            .serial_number(serial_number)
            .build()
            .unwrap();
        let texts = [lens.make(), lens.model(), lens.serial_number()];
        assert_eq!(
            texts,
            [
                Some("Nikon"),
                Some("NIKKOR Z 14-30mm f/4 S"),
                Some("20012345")
            ]
        );
        // The very texts given, not copies of them.
        assert_eq!(texts.map(|text| text.map(str::as_ptr)), addresses.map(Some));
    }

    #[test]
    fn a_later_lens_value_replaces_an_earlier_one() {
        let one = fraction(1, 1);
        let lens = Lens::builder()
            .make("A")
            .model("A")
            .serial_number("A")
            .min_focal_length(one)
            .max_focal_length(one)
            .f_number_at_min_focal_length(one)
            .f_number_at_max_focal_length(one)
            .make("Nikon")
            .model("NIKKOR Z 14-30mm f/4 S")
            .serial_number("20012345")
            .min_focal_length(fraction(140, 10))
            .max_focal_length(fraction(300, 10))
            .f_number_at_min_focal_length(fraction(400, 100))
            .f_number_at_max_focal_length(fraction(400, 100))
            .build()
            .unwrap();
        assert_eq!(lens, z_lens());
    }

    #[test]
    fn a_lens_builder_starts_empty_and_compares_by_its_values() {
        assert_eq!(LensBuilder::default(), Lens::builder());
        let named = Lens::builder().model("x");
        assert_ne!(named, Lens::builder());
        assert_eq!(named.clone(), named);
        assert!(format!("{named:?}").contains("\"x\""));
        let builders: std::collections::HashSet<LensBuilder> =
            [Lens::builder(), named.clone(), Lens::builder().model("x")]
                .into_iter()
                .collect();
        assert_eq!(builders.len(), 2);
    }

    #[test]
    fn lenses_and_exposures_collect_in_a_set() {
        // A culler listing the lenses of a folder puts them in a set.
        let lens = |model: &str| Lens::builder().model(model).build().unwrap();
        let lenses: std::collections::HashSet<Lens> =
            [lens("A"), lens("B"), lens("A")].into_iter().collect();
        assert_eq!(lenses.len(), 2);

        let exposure = |iso| Exposure {
            iso: Some(iso),
            ..Exposure::default()
        };
        let exposures: std::collections::HashSet<Exposure> =
            [exposure(64), exposure(64), exposure(100)]
                .into_iter()
                .collect();
        assert_eq!(exposures.len(), 2);
    }

    fn size(width: u32, height: u32) -> Size {
        Size { width, height }
    }

    fn point(x: u32, y: u32) -> Point {
        Point { x, y }
    }

    /// The focus a reader builds from these values, a step at a time: the
    /// centre in the frame, then the area when there is one, then whether
    /// focus was confirmed when that is known. `None` when a step refuses
    /// its value.
    fn built(
        frame: Size,
        center: Point,
        area: Option<Size>,
        in_focus: Option<bool>,
    ) -> Option<Focus> {
        let mut focus = Focus::new(frame, center)?;
        if let Some(area) = area {
            focus = focus.with_size(area)?;
        }
        if let Some(in_focus) = in_focus {
            focus = focus.with_in_focus(in_focus);
        }
        Some(focus)
    }

    /// Where a Nikon Z 9 focused: a 291 × 323 area right of and above the
    /// centre of its 8256 × 5504 frame, with focus confirmed.
    fn z9_focus() -> Focus {
        Focus::new(size(8_256, 5_504), point(4_905, 2_461))
            .unwrap()
            .with_size(size(291, 323))
            .unwrap()
            .with_in_focus(true)
    }

    #[test]
    fn a_focus_needs_a_frame_with_no_zero_side() {
        // Refused even with the centre at 0, which lies on both edges of a
        // zero side.
        assert_eq!(Focus::new(size(0, 100), point(0, 0)), None);
        assert_eq!(Focus::new(size(100, 0), point(0, 0)), None);
        assert_eq!(Focus::new(size(0, 0), point(0, 0)), None);
        assert!(Focus::new(size(1, 1), point(0, 0)).is_some());
    }

    #[test]
    fn a_focus_needs_its_centre_on_the_frame() {
        let frame = size(100, 50);
        assert_eq!(Focus::new(frame, point(101, 0)), None);
        assert_eq!(Focus::new(frame, point(0, 51)), None);
        assert_eq!(Focus::new(frame, point(101, 51)), None);
        // The far edges are on the frame.
        assert!(Focus::new(frame, point(100, 50)).is_some());
    }

    #[test]
    fn a_focus_without_a_size_may_be_anywhere_on_the_frame() {
        let frame = size(100, 50);
        let corners = [point(0, 0), point(100, 50), point(0, 50), point(100, 0)];
        for center in corners.into_iter().chain([point(99, 49)]) {
            let focus = Focus::new(frame, center);
            assert_eq!(focus.map(|focus| focus.center()), Some(center));
        }
    }

    #[test]
    fn no_area_fits_around_a_centre_on_an_edge() {
        let frame = size(100, 100);
        for center in [point(0, 50), point(100, 50), point(50, 0), point(50, 100)] {
            let focus = Focus::new(frame, center).unwrap();
            assert_eq!(focus.with_size(size(1, 1)), None, "{center:?}");
            // One pixel in from the edge, an area of 2 fits.
            let inward = point(center.x.clamp(1, 99), center.y.clamp(1, 99));
            let focus = Focus::new(frame, inward).unwrap();
            assert!(focus.with_size(size(2, 2)).is_some(), "{inward:?}");
        }
    }

    #[test]
    fn a_focus_area_needs_no_zero_side() {
        let focus = Focus::new(size(100, 100), point(50, 50)).unwrap();
        assert_eq!(focus.with_size(size(0, 10)), None);
        assert_eq!(focus.with_size(size(10, 0)), None);
        assert!(focus.with_size(size(10, 10)).is_some());
    }

    #[test]
    fn a_focus_area_fits_around_its_centre_by_half_its_side_rounded_up() {
        // In a frame of 100, an area of 10 (half 5) fits a centre from 5 to
        // 95, and one of 11 (half 6) from 6 to 94: (side, centre, accepted).
        let cases = [
            (10, 5, true),
            (10, 4, false),
            (10, 95, true),
            (10, 96, false),
            (11, 6, true),
            (11, 5, false),
            (11, 94, true),
            (11, 95, false),
        ];
        let frame = size(100, 100);
        for (side, at, accepted) in cases {
            let across = Focus::new(frame, point(at, 50)).unwrap();
            let across = across.with_size(size(side, 1));
            assert_eq!(across.is_some(), accepted, "width {side} at x = {at}");
            let down = Focus::new(frame, point(50, at)).unwrap();
            let down = down.with_size(size(1, side));
            assert_eq!(down.is_some(), accepted, "height {side} at y = {at}");
        }
    }

    #[test]
    fn a_refused_area_costs_the_focus_or_only_the_area_as_the_builder_chooses() {
        let (frame, center) = (size(100, 100), point(50, 50));
        let empty = size(0, 10);
        // With `?` on every step, the whole focus is refused.
        let whole = || Focus::new(frame, center)?.with_size(empty);
        assert_eq!(whole(), None);
        // Falling back to the focus before the area keeps the point alone.
        let focus = Focus::new(frame, center).unwrap().with_in_focus(true);
        let point_only = focus.with_size(empty).unwrap_or(focus);
        assert_eq!(point_only.center(), center);
        assert_eq!(point_only.size(), None);
        assert_eq!(point_only.in_focus(), Some(true));
    }

    #[test]
    fn a_focus_near_the_largest_u32_follows_the_same_rules() {
        const MAX: u32 = u32::MAX;
        let frame = size(MAX, MAX);
        let new = |x, y, area: Option<u32>| {
            built(frame, point(x, y), area.map(|side| size(side, side)), None).is_some()
        };
        // No `u32` lies past a frame of MAX, so every centre is on it, the
        // far edges included; no area fits around a centre there.
        assert!(new(MAX - 1, MAX - 1, None));
        assert!(new(MAX, 0, None));
        assert!(new(0, MAX, None));
        assert!(new(MAX, MAX, None));
        assert!(!new(MAX, MAX, Some(1)));

        // An area of MAX - 1, half 2^31 - 1, fits a centre from 2^31 - 1 to
        // 2^31; past that, the centre plus the half overflows a u32.
        let half = (MAX - 1) / 2;
        assert!(new(half, half, Some(MAX - 1)));
        assert!(new(half + 1, half + 1, Some(MAX - 1)));
        assert!(!new(half - 1, half, Some(MAX - 1)));
        assert!(!new(half + 2, half, Some(MAX - 1)));
        assert!(!new(half, half + 2, Some(MAX - 1)));

        // An area of MAX, half 2^31, fits no centre.
        assert!(!new(half, half, Some(MAX)));
        assert!(!new(half + 1, half + 1, Some(MAX)));

        // One pixel from the far edges, an area of 2 (half 1) reaches them,
        // and one of 3 (half 2) would sum past MAX.
        assert!(new(MAX - 1, MAX - 1, Some(2)));
        assert!(!new(MAX - 1, MAX - 1, Some(3)));
        assert!(!new(MAX - 1, MAX - 1, Some(MAX)));
    }

    #[test]
    fn focus_getters_return_what_was_built() {
        let unconfirmed = Focus::new(size(8_256, 5_504), point(4_905, 2_461))
            .unwrap()
            .with_size(size(291, 323))
            .unwrap();
        // `with_in_focus` never called is `None`.
        let all = [
            (unconfirmed.with_in_focus(true), Some(true)),
            (unconfirmed.with_in_focus(false), Some(false)),
            (unconfirmed, None),
        ];
        for (focus, in_focus) in all {
            assert_eq!(focus.frame(), size(8_256, 5_504));
            assert_eq!(focus.center(), point(4_905, 2_461));
            assert_eq!(focus.size(), Some(size(291, 323)));
            assert_eq!(focus.in_focus(), in_focus);
        }
        let without_size = Focus::new(size(6_000, 4_000), point(3_000, 2_000)).unwrap();
        assert_eq!(without_size.size(), None);
        assert_eq!(without_size.in_focus(), None);
    }

    #[test]
    fn a_later_area_or_confirmation_replaces_the_earlier_one() {
        let focus = Focus::new(size(100, 100), point(50, 50))
            .unwrap()
            .with_size(size(10, 10))
            .unwrap()
            .with_in_focus(true);

        let resized = focus.with_size(size(20, 30)).unwrap();
        assert_eq!(resized.size(), Some(size(20, 30)));
        assert_eq!(resized.frame(), size(100, 100));
        assert_eq!(resized.center(), point(50, 50));
        assert_eq!(resized.in_focus(), Some(true));

        let missed = focus.with_in_focus(false);
        assert_eq!(missed.in_focus(), Some(false));
        assert_eq!(missed.frame(), size(100, 100));
        assert_eq!(missed.center(), point(50, 50));
        assert_eq!(missed.size(), Some(size(10, 10)));
        assert_eq!(missed.with_in_focus(true), focus);
    }

    #[test]
    fn a_z9_focus_scales_onto_its_preview() {
        let preview = z9_focus().scaled_to(size(1_920, 1_280)).unwrap();
        // 4905 × 1920 / 8256 = 1140.70 and 2461 × 1280 / 5504 = 572.33;
        // 291 × 1920 / 8256 = 67.67 and 323 × 1280 / 5504 = 75.12.
        assert_eq!(preview.center(), point(1_141, 572));
        assert_eq!(preview.size(), Some(size(68, 75)));
        assert_eq!(preview.frame(), size(1_920, 1_280));
        assert_eq!(preview.in_focus(), Some(true));
    }

    #[test]
    fn a_focus_scaled_to_its_own_frame_is_unchanged() {
        let z9 = z9_focus();
        assert_eq!(z9.scaled_to(z9.frame()), Some(z9));
        let odd = Focus::new(size(7, 3), point(6, 0))
            .unwrap()
            .with_in_focus(false);
        assert_eq!(odd.scaled_to(odd.frame()), Some(odd));
        let corner = Focus::new(size(7, 3), point(7, 3)).unwrap();
        assert_eq!(corner.scaled_to(corner.frame()), Some(corner));
    }

    #[test]
    fn a_focus_scaled_to_one_pixel_is_on_it_without_an_area() {
        let pixel = z9_focus().scaled_to(size(1, 1)).unwrap();
        assert_eq!(pixel.frame(), size(1, 1));
        // (4905 + 4128) / 8256 rounds to 1, on the right edge, and
        // (2461 + 2752) / 5504 to 0, on the top edge: no room for an area.
        assert_eq!(pixel.center(), point(1, 0));
        assert_eq!(pixel.size(), None);
        assert_eq!(pixel.in_focus(), Some(true));
    }

    #[test]
    fn a_focus_scales_to_no_frame_with_a_zero_side() {
        let z9 = z9_focus();
        assert_eq!(z9.scaled_to(size(0, 1_280)), None);
        assert_eq!(z9.scaled_to(size(1_920, 0)), None);
        assert_eq!(z9.scaled_to(size(0, 0)), None);
    }

    #[test]
    fn a_scaled_focus_rounds_to_the_nearest_pixel_a_half_up() {
        let scaled = |frame, center, area, to| {
            let focus = built(frame, center, area, None).unwrap();
            let scaled = focus.scaled_to(to).unwrap();
            (scaled.center(), scaled.size())
        };
        // 1 × 2 / 4 = 0.5 rounds up to 1, on both axes.
        assert_eq!(
            scaled(size(4, 4), point(1, 1), None, size(2, 2)),
            (point(1, 1), None)
        );
        // 2 × 4 / 3 = 2.67 rounds up to 3, where cutting off the fraction
        // would give 2.
        assert_eq!(
            scaled(size(3, 3), point(2, 2), None, size(4, 4)),
            (point(3, 3), None)
        );
        // 1 × 4 / 10 = 0.4 rounds down to 0.
        assert_eq!(
            scaled(size(10, 10), point(1, 1), None, size(4, 4)),
            (point(0, 0), None)
        );
        // An area of 3 × 4 / 8 = 1.5 rounds up to 2.
        assert_eq!(
            scaled(size(8, 8), point(4, 4), Some(size(3, 3)), size(4, 4)),
            (point(2, 2), Some(size(2, 2)))
        );
    }

    #[test]
    fn a_scaled_focus_keeps_its_centre_and_trims_its_area_to_fit() {
        // The area rounds to 3, and the centre to 2, which an area of 3
        // (half 2) would overflow in a frame of 3: the area is trimmed to 2.
        let focus = built(size(4, 4), point(2, 2), Some(size(4, 4)), None).unwrap();
        let scaled = focus.scaled_to(size(3, 3)).unwrap();
        assert_eq!(scaled.center(), point(2, 2));
        assert_eq!(scaled.size(), Some(size(2, 2)));

        // Only the axis that needs it is trimmed.
        let focus = built(size(4, 8), point(2, 4), Some(size(4, 4)), None).unwrap();
        let scaled = focus.scaled_to(size(3, 8)).unwrap();
        assert_eq!(scaled.center(), point(2, 4));
        assert_eq!(scaled.size(), Some(size(2, 4)));
    }

    #[test]
    fn a_focus_scales_up_and_between_the_largest_frames() {
        const MAX: u32 = u32::MAX;
        let small = built(size(2, 2), point(1, 1), Some(size(2, 2)), None).unwrap();
        let large = small.scaled_to(size(2_000, 2_000)).unwrap();
        assert_eq!(large.center(), point(1_000, 1_000));
        assert_eq!(large.size(), Some(size(2_000, 2_000)));

        // 1 × MAX / 2 rounds up to 2^31; the area of MAX that 2 × MAX / 2
        // gives is trimmed to MAX - 1 to fit around it.
        let largest = small.scaled_to(size(MAX, MAX)).unwrap();
        assert_eq!(largest.center(), point(1 << 31, 1 << 31));
        assert_eq!(largest.size(), Some(size(MAX - 1, MAX - 1)));

        let corner = built(
            size(MAX, MAX),
            point(MAX - 1, MAX - 1),
            Some(size(2, 2)),
            None,
        )
        .unwrap();
        assert_eq!(corner.scaled_to(size(MAX, MAX)), Some(corner));
        // (MAX - 1)² / MAX is MAX - 2 and a sliver; 2 × (MAX - 1) / MAX
        // is just under 2.
        let shrunk = corner.scaled_to(size(MAX - 1, MAX - 1)).unwrap();
        assert_eq!(shrunk.center(), point(MAX - 2, MAX - 2));
        assert_eq!(shrunk.size(), Some(size(2, 2)));

        let half = (MAX - 1) / 2;
        let widest = built(
            size(MAX, MAX),
            point(half, half),
            Some(size(MAX - 1, MAX - 1)),
            None,
        )
        .unwrap();
        let down = widest.scaled_to(size(1_000, 1_000)).unwrap();
        assert_eq!(down.center(), point(500, 500));
        assert_eq!(down.size(), Some(size(1_000, 1_000)));
    }

    #[test]
    fn a_scaled_focus_always_satisfies_the_rules_of_its_builders() {
        // On one axis, every frame from 1 to 12 pixels, every centre on it,
        // edges included, no area and every area `with_size` accepts around
        // it, scaled to every frame from 1 to 12 pixels. The other axis
        // stays a frame of 3 with the centre at 1 and an area of 1.
        let mut checked = 0;
        for vertical in [false, true] {
            let on_axis = |frame: u32, center: u32, area: Option<u32>| {
                if vertical {
                    (
                        size(3, frame),
                        point(1, center),
                        area.map(|side| size(1, side)),
                    )
                } else {
                    (
                        size(frame, 3),
                        point(center, 1),
                        area.map(|side| size(side, 1)),
                    )
                }
            };
            let axis = |focus: Focus| {
                let size = focus.size();
                if vertical {
                    (
                        focus.center().y,
                        size.map(|size| size.height),
                        focus.center().x,
                    )
                } else {
                    (
                        focus.center().x,
                        size.map(|size| size.width),
                        focus.center().y,
                    )
                }
            };
            for old in 1..=12_u32 {
                for center in 0..=old {
                    for area in std::iter::once(None).chain((1..=old).map(Some)) {
                        let (frame, at, area_size) = on_axis(old, center, area);
                        let Some(focus) = built(frame, at, area_size, Some(false)) else {
                            continue;
                        };
                        for new in 1..=12_u32 {
                            let (target, _, _) = on_axis(new, 0, None);
                            let context = format!("{focus:?} to {target:?}");
                            let scaled = focus.scaled_to(target).expect(&context);
                            assert_eq!(scaled.frame(), target, "{context}");
                            assert_eq!(scaled.in_focus(), Some(false), "{context}");
                            // Built again a step at a time, it is the same.
                            assert_eq!(
                                built(
                                    scaled.frame(),
                                    scaled.center(),
                                    scaled.size(),
                                    scaled.in_focus()
                                ),
                                Some(scaled),
                                "{context}"
                            );

                            // The nearest whole number, a half up, which is
                            // on the new frame: never moved by the trimming
                            // of the area.
                            let (got_center, got_area, other) = axis(scaled);
                            let nearest = |value: u32| (2 * value * new + old) / (2 * old);
                            assert_eq!(got_center, nearest(center), "{context}");
                            assert!(got_center <= new, "{context}");
                            assert_eq!(other, 1, "{context}");

                            // The area, at least 1, trimmed only as far as
                            // it must be to fit, and dropped only for a
                            // centre on an edge.
                            match (area, got_area) {
                                (None, got) => assert_eq!(got, None, "{context}"),
                                (Some(side), Some(got)) => {
                                    let rounded = nearest(side).max(1);
                                    assert!(got <= rounded, "{context}");
                                    let one_more = fits(got_center, got + 1, new);
                                    assert!(got == rounded || !one_more, "{context}");
                                }
                                (Some(_), None) => {
                                    assert!(got_center == 0 || got_center == new, "{context}");
                                }
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
        // Every case was reached: 412 starting focuses on each axis, each
        // scaled 12 ways.
        assert_eq!(checked, 9_888);
    }

    #[test]
    fn focus_equality_is_by_every_value_and_collects_in_a_set() {
        let z9 = z9_focus();
        assert_eq!(z9, z9_focus());
        let frame = size(8_256, 5_504);
        let center = point(4_905, 2_461);
        let area = Some(size(291, 323));
        let others = [
            built(size(8_257, 5_504), center, area, Some(true)),
            built(size(8_256, 5_505), center, area, Some(true)),
            built(frame, point(4_906, 2_461), area, Some(true)),
            built(frame, point(4_905, 2_462), area, Some(true)),
            built(frame, center, Some(size(292, 323)), Some(true)),
            built(frame, center, Some(size(291, 324)), Some(true)),
            built(frame, center, None, Some(true)),
            built(frame, center, area, Some(false)),
            built(frame, center, area, None),
        ];
        let others: Vec<Focus> = others.into_iter().map(Option::unwrap).collect();
        for (place, other) in others.iter().enumerate() {
            assert_ne!(&z9, other, "value {place}");
        }

        let mut all = others.clone();
        all.extend([z9, z9_focus()]);
        all.extend(others.iter().copied());
        let set: std::collections::HashSet<Focus> = all.into_iter().collect();
        assert_eq!(set.len(), others.len() + 1);
    }

    /// The metadata of a Z 9 picture, every value set and each one
    /// different from the others, so that a getter reading the wrong value
    /// is caught.
    fn z9_metadata() -> Metadata {
        Metadata {
            make: Some("NIKON CORPORATION".to_owned()),
            model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("3001234".to_owned()),
            capture_time: DateTime::from_exif(b"2024:05:01 10:20:30", Some(b"85"), Some(b"+02:00")),
            orientation: Orientation::Rotate90,
            exposure: z9_exposure(),
            lens: Lens::builder().model("NIKKOR Z 14-30mm f/4 S").build(),
            raw_size: Some(Size {
                width: 8_280,
                height: 5_520,
            }),
            image_size: Some(Size {
                width: 8_256,
                height: 5_504,
            }),
            focus: Some(z9_focus()),
        }
    }

    #[test]
    fn default_metadata_is_empty() {
        let metadata = Metadata::default();
        assert_eq!(metadata.make(), None);
        assert_eq!(metadata.model(), None);
        assert_eq!(metadata.serial_number(), None);
        assert_eq!(metadata.capture_time(), None);
        assert_eq!(metadata.orientation(), Orientation::NoTransforms);
        assert_eq!(metadata.exposure(), &Exposure::default());
        assert_eq!(metadata.lens(), None);
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), None);
        assert_eq!(metadata.focus(), None);
    }

    #[test]
    fn metadata_getters_return_what_was_set() {
        let metadata = z9_metadata();
        assert_eq!(metadata.make(), Some("NIKON CORPORATION"));
        assert_eq!(metadata.model(), Some("NIKON Z 9"));
        assert_eq!(metadata.serial_number(), Some("3001234"));
        let time = metadata.capture_time().unwrap();
        assert_eq!(time.to_string(), "2024-05-01T10:20:30.85+02:00");
        assert_eq!(metadata.orientation(), Orientation::Rotate90);
        assert_eq!(metadata.exposure(), &z9_exposure());
        assert_eq!(
            metadata.lens().and_then(Lens::model),
            Some("NIKKOR Z 14-30mm f/4 S")
        );
        assert_eq!(
            metadata.raw_size(),
            Some(Size {
                width: 8_280,
                height: 5_520,
            })
        );
        assert_eq!(
            metadata.image_size(),
            Some(Size {
                width: 8_256,
                height: 5_504,
            })
        );
        assert_eq!(metadata.focus(), Some(z9_focus()));
    }

    #[test]
    fn metadata_collects_in_a_set() {
        // A renamer grouping a folder's pictures by what they record puts
        // their metadata in a set.
        let metadata = |model: &str| Metadata {
            model: Some(model.to_owned()),
            ..Metadata::default()
        };
        let all: std::collections::HashSet<Metadata> =
            [metadata("A"), metadata("B"), metadata("A")]
                .into_iter()
                .collect();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn each_exposure_builder_sets_its_own_value_and_no_other() {
        let bias = SignedRational {
            numerator: 3,
            denominator: 3,
        };
        // From the empty exposure, and from a full one whose every value
        // differs from the one set, so that a builder writing another
        // value, or clearing one, is caught. The expected value is built
        // field by field, not through the builders.
        for start in [Exposure::default(), z9_exposure()] {
            let cases = [
                (
                    start.clone().with_iso(100),
                    Exposure {
                        iso: Some(100),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_exposure_time(fraction(1, 250)),
                    Exposure {
                        exposure_time: Some(fraction(1, 250)),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_f_number(fraction(56, 10)),
                    Exposure {
                        f_number: Some(fraction(56, 10)),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_focal_length(fraction(240, 10)),
                    Exposure {
                        focal_length: Some(fraction(240, 10)),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_exposure_bias(bias),
                    Exposure {
                        exposure_bias: Some(bias),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
    }

    #[test]
    fn each_metadata_builder_sets_its_own_value_and_no_other() {
        let time = DateTime::new(2022, 2, 5, 13, 46, 41).unwrap();
        let exposure = Exposure {
            iso: Some(100),
            ..Exposure::default()
        };
        let focus = Focus::new(size(6_000, 4_000), point(3_000, 2_000)).unwrap();
        // From the empty metadata, and from a full one whose every value
        // differs from the one set, so that a builder writing another
        // value, or clearing one, is caught. The expected value is built
        // field by field, not through the builders.
        for start in [Metadata::default(), z9_metadata()] {
            let cases = [
                (
                    start.clone().with_make("Nikon"),
                    Metadata {
                        make: Some("Nikon".to_owned()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_model("NIKON Z 8"),
                    Metadata {
                        model: Some("NIKON Z 8".to_owned()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_serial_number("7654321"),
                    Metadata {
                        serial_number: Some("7654321".to_owned()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_capture_time(time),
                    Metadata {
                        capture_time: Some(time),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_orientation(Orientation::Rotate270),
                    Metadata {
                        orientation: Orientation::Rotate270,
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_exposure(exposure.clone()),
                    Metadata {
                        exposure: exposure.clone(),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_lens(z_lens()),
                    Metadata {
                        lens: Some(z_lens()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_raw_size(size(6_048, 4_024)),
                    Metadata {
                        raw_size: Some(size(6_048, 4_024)),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_image_size(size(6_000, 4_000)),
                    Metadata {
                        image_size: Some(size(6_000, 4_000)),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_focus(focus),
                    Metadata {
                        focus: Some(focus),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
    }

    /// [`z9_metadata`] built through the ten builders.
    fn z9_metadata_built() -> Metadata {
        let lens = Lens::builder()
            .model("NIKKOR Z 14-30mm f/4 S")
            .build()
            .unwrap();
        Metadata::default()
            .with_make("NIKON CORPORATION")
            .with_model("NIKON Z 9")
            .with_serial_number("3001234")
            .with_capture_time(
                DateTime::from_exif(b"2024:05:01 10:20:30", Some(b"85"), Some(b"+02:00")).unwrap(),
            )
            .with_orientation(Orientation::Rotate90)
            .with_exposure(z9_exposure_built())
            .with_lens(lens)
            .with_raw_size(size(8_280, 5_520))
            .with_image_size(size(8_256, 5_504))
            .with_focus(z9_focus())
    }

    /// [`z9_exposure`] built through the five builders.
    fn z9_exposure_built() -> Exposure {
        Exposure::default()
            .with_iso(64)
            .with_exposure_time(fraction(10, 3200))
            .with_f_number(fraction(900, 100))
            .with_focal_length(fraction(140, 10))
            .with_exposure_bias(SignedRational {
                numerator: -4,
                denominator: 6,
            })
    }

    #[test]
    fn every_builder_in_a_chain_equals_the_value_built_field_by_field() {
        assert_eq!(z9_exposure_built(), z9_exposure());
        assert_eq!(z9_metadata_built(), z9_metadata());
    }

    #[test]
    fn a_later_exposure_or_metadata_value_replaces_an_earlier_one() {
        let one = fraction(1, 1);
        let exposure = Exposure::default()
            .with_iso(1)
            .with_exposure_time(one)
            .with_f_number(one)
            .with_focal_length(one)
            .with_exposure_bias(SignedRational {
                numerator: 1,
                denominator: 1,
            });
        // The Z 9's values, each set after a value of 1.
        let exposure = exposure
            .with_iso(64)
            .with_exposure_time(fraction(10, 3200))
            .with_f_number(fraction(900, 100))
            .with_focal_length(fraction(140, 10))
            .with_exposure_bias(SignedRational {
                numerator: -4,
                denominator: 6,
            });
        assert_eq!(exposure, z9_exposure());

        let earlier = Metadata::default()
            .with_make("A")
            .with_model("A")
            .with_serial_number("A")
            .with_capture_time(DateTime::new(2000, 1, 1, 0, 0, 0).unwrap())
            .with_orientation(Orientation::Rotate180)
            .with_exposure(Exposure::default().with_iso(1))
            .with_lens(Lens::builder().model("A").build().unwrap())
            .with_raw_size(size(1, 1))
            .with_image_size(size(1, 1))
            .with_focus(Focus::new(size(1, 1), point(0, 0)).unwrap());
        // Every value of `earlier` differs from the Z 9's, so each builder
        // below has one to replace.
        let z9 = z9_metadata();
        let metadata = earlier
            .with_make("NIKON CORPORATION")
            .with_model("NIKON Z 9")
            .with_serial_number("3001234")
            .with_capture_time(z9.capture_time.unwrap())
            .with_orientation(z9.orientation)
            .with_exposure(z9.exposure.clone())
            .with_lens(z9.lens.clone().unwrap())
            .with_raw_size(z9.raw_size.unwrap())
            .with_image_size(z9.image_size.unwrap())
            .with_focus(z9.focus.unwrap());
        assert_eq!(metadata, z9);
    }

    #[test]
    fn the_metadata_builders_move_an_owned_text_rather_than_copy_it() {
        let make = String::from("NIKON CORPORATION");
        let model = String::from("NIKON Z 9");
        let serial_number = String::from("3001234");
        let addresses = [make.as_ptr(), model.as_ptr(), serial_number.as_ptr()];
        let metadata = Metadata::default()
            .with_make(make)
            .with_model(model)
            .with_serial_number(serial_number);
        let texts = [metadata.make(), metadata.model(), metadata.serial_number()];
        assert_eq!(
            texts,
            [
                Some("NIKON CORPORATION"),
                Some("NIKON Z 9"),
                Some("3001234")
            ]
        );
        // The very texts given, not copies of them.
        assert_eq!(texts.map(|text| text.map(str::as_ptr)), addresses.map(Some));
    }

    #[test]
    fn the_builders_store_values_as_given() {
        // Neither cleaned nor refused, as the readers would a file's: a
        // padded or empty text, a zero ISO or fraction, and an image size
        // with no raw size.
        let zero = Rational {
            numerator: 0,
            denominator: 1,
        };
        let exposure = Exposure::default()
            .with_iso(0)
            .with_exposure_time(zero)
            .with_f_number(zero)
            .with_focal_length(zero);
        assert_eq!(exposure.iso(), Some(0));
        assert_eq!(exposure.exposure_time(), Some(zero));
        assert_eq!(exposure.f_number(), Some(zero));
        assert_eq!(exposure.focal_length(), Some(zero));

        let metadata = Metadata::default()
            .with_make("  padded  ")
            .with_model("NIKON Z 9\0\0")
            .with_serial_number("")
            .with_image_size(size(0, 0));
        assert_eq!(metadata.make(), Some("  padded  "));
        assert_eq!(metadata.model(), Some("NIKON Z 9\0\0"));
        assert_eq!(metadata.serial_number(), Some(""));
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), Some(size(0, 0)));
    }
}
