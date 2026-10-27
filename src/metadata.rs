// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the camera recorded about a picture.
//!
//! Besides the image, a camera records how the picture was taken: by which
//! camera and when, the software that wrote the file, the exposure it
//! chose, how the camera was set, the lens it was taken with, how large the
//! image is, and where the camera focused. [`Metadata`] holds all of it,
//! read when the file is opened; [`Exposure`] and [`Lens`] hold the
//! exposure and the lens as the file records them, fractions with the
//! numerator and denominator it stored, [`CameraSettings`] the camera's
//! settings, [`Focus`] where the camera's autofocus aimed, as the camera
//! recorded it, [`FocusPoints`] the autofocus points it used, and
//! [`FocusLayout`] every autofocus point of the body, where raw-files has
//! its table of them. [`LensBuilder`] builds a lens a value at a time, for
//! a caller's tests.
//!
//! Every value is optional. A file may not record it, or may record it in a
//! way that cannot be read, such as a tag of the wrong type; either way the
//! value is `None`, and the others are unaffected.
//!
//! A program that uses raw-files can build these values for its own tests,
//! without a camera file: [`Metadata`], [`Exposure`] and [`CameraSettings`]
//! through their `with_…` methods, a [`Lens`] through [`Lens::builder`],
//! [`FocusPoints`] through their builders, and a [`Focus`], a
//! [`FocusLayout`], a [`DateTime`] and a [`Rational`] through their checked
//! constructors. [`Metadata`]'s docs show how.

use crate::camera_settings::CameraSettings;
use crate::datetime::DateTime;
use crate::focus::{Focus, FocusLayout, FocusPoints};
use crate::geometry::Size;
use crate::orientation::Orientation;
use crate::rational::{Rational, SignedRational};

/// What the camera recorded about a picture: the camera and the software
/// that wrote the file, when the picture was taken, how to turn it upright,
/// its exposure, how the camera was set, its lens, the sizes of its raw
/// data and of its visible image, and where the camera focused, with the
/// layout of the body's autofocus points where raw-files has its table.
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
///     println!("{size} pixels");
/// }
/// # Ok(())
/// # }
/// ```
///
/// A program's tests build a `Metadata` without a camera file, from the
/// empty one, and read back what they set; what they do not set stays
/// `None`:
///
/// ```
/// use raw_files::{
///     CameraSettings, DateTime, Exposure, Focus, Lens, Metadata, MeteringMode, Point, Rational,
///     Size,
/// };
///
/// # fn main() {
/// #     example().expect("every step gives a value");
/// # }
/// # fn example() -> Option<()> {
/// let time = DateTime::new(2022, 2, 5, 13, 46, 41)?
///     .with_subseconds(850_000_000, 2)?
///     .with_offset_minutes(-300)?;
/// let exposure = Exposure::default()
///     .with_iso(64)
///     .with_exposure_time(Rational::new(10, 3200)?);
/// let frame = Size { width: 8256, height: 5504 };
/// let focus = Focus::new(frame, Point { x: 4905, y: 2461 })?
///     .with_size(Size { width: 291, height: 323 })?
///     .with_in_focus(true);
/// let metadata = Metadata::default()
///     .with_model("NIKON Z 9")
///     .with_capture_time(time)
///     .with_exposure(exposure)
///     .with_camera_settings(
///         CameraSettings::default().with_metering_mode(MeteringMode::MultiSegment),
///     )
///     .with_lens(Lens::builder().model("NIKKOR Z 14-30mm f/4 S").build()?)
///     .with_focus(focus);
///
/// assert_eq!(metadata.model(), Some("NIKON Z 9"));
/// assert_eq!(
///     metadata.capture_time().map(|time| time.to_string()).as_deref(),
///     Some("2022-02-05T13:46:41.85-05:00")
/// );
/// assert_eq!(metadata.exposure().iso(), Some(64));
/// assert_eq!(metadata.exposure().exposure_time(), Rational::new(10, 3200));
/// assert_eq!(
///     metadata.camera_settings().metering_mode(),
///     Some(MeteringMode::MultiSegment)
/// );
/// assert_eq!(metadata.camera_settings().white_balance().mode(), None);
/// assert_eq!(
///     metadata.lens().and_then(Lens::model),
///     Some("NIKKOR Z 14-30mm f/4 S")
/// );
/// assert_eq!(
///     metadata.focus().map(|focus| focus.center()),
///     Some(Point { x: 4905, y: 2461 })
/// );
/// // The focus is a list of that one point, its primary.
/// let points = metadata.focus_points()?;
/// assert_eq!(points.points().len(), 1);
/// assert_eq!(
///     points.primary().map(|point| point.center()),
///     Some(Point { x: 4905, y: 2461 })
/// );
/// assert_eq!(metadata.serial_number(), None);
/// assert_eq!(metadata.exposure().f_number(), None);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Metadata {
    pub(crate) make: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) software: Option<String>,
    pub(crate) serial_number: Option<String>,
    pub(crate) capture_time: Option<DateTime>,
    pub(crate) orientation: Orientation,
    pub(crate) exposure: Exposure,
    pub(crate) camera_settings: CameraSettings,
    pub(crate) lens: Option<Lens>,
    pub(crate) raw_size: Option<Size>,
    pub(crate) image_size: Option<Size>,
    pub(crate) focus_points: Option<FocusPoints>,
    pub(crate) focus_layout: Option<FocusLayout>,
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

    /// The software that wrote the file: IFD0's Software, as the file
    /// records it, cleaned of padding by the reader as every text value
    /// is, or as given to [`with_software`](Self::with_software).
    ///
    /// A camera writes its firmware version there, such as `Ver.01.00`
    /// (Nikon) or `ILCE-7M4 v2.00` (Sony), and a program that rewrites the
    /// file writes its own name, so a caller cannot tell which without
    /// reading the text. It is `None` on a file that records none, such as
    /// those of the Sony DSC-F828 and DSC-R1.
    pub fn software(&self) -> Option<&str> {
        self.software.as_deref()
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
    /// one: the raw.pixls.us sample 2956 of the Nikon 1 J1 records
    /// 2010-01-01 00:11, with its time-zone setting of +02:00.
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

    /// How the camera was set for the picture (see [`CameraSettings`]),
    /// for a viewer's information panel beside the exposure: the white
    /// balance, the exposure program, the metering mode, the flash, the
    /// shutter, the image area, the raw image size and the colour space.
    /// Always present, though each of its values is optional: `None` when
    /// the file does not record it or records it in a way that cannot be
    /// read, a damaged value costing only itself.
    ///
    /// NEF, ARW and JPEG files give the settings that their standard Exif
    /// tags record: the exposure program, the metering mode, the flash, the
    /// white balance mode and the colour space. NEF files and the JPEGs of
    /// Nikon cameras give, from Nikon's MakerNote, the white balance
    /// setting, finer than Exif's, in place of Exif's, the colour
    /// temperature, the fine-tuning, the shutter, the image area, in a NEF
    /// file alone the raw image size, and the colour space where Exif
    /// records none, as in every NEF file checked; Nikon records no
    /// electronic front curtain setting. ARW files and the JPEGs of Sony
    /// cameras give, from Sony's MakerNote, the white balance setting in
    /// place of Exif's, the fine-tuning, the electronic front curtain
    /// setting and the colour temperature of a chosen colour temperature
    /// or a custom white balance, and no shutter, image area or raw image
    /// size, which Sony does not record in a form raw-files reads. Each
    /// getter of [`CameraSettings`] says where its value comes from. A
    /// program's tests build the settings with the `with_…` methods of
    /// [`CameraSettings`] and set them with
    /// [`with_camera_settings`](Self::with_camera_settings).
    pub fn camera_settings(&self) -> &CameraSettings {
        &self.camera_settings
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
    /// size, not the sensor's. A JPEG file holds no raw data, so it has no
    /// raw size.
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
    /// record no crop, and on the raw.pixls.us samples their stored raw is
    /// up to 64 pixels wider and 24 taller than the camera's own full-size
    /// preview.
    ///
    /// It is the default crop, not an aspect ratio chosen in the camera. On
    /// a Nikon small raw it counts the stored pixels, which can differ from
    /// the size the camera's menu names. For a RAW file, it is `None` when
    /// there is no raw size.
    ///
    /// For a JPEG file, it is the size its frame header gives: the picture's
    /// own size.
    pub fn image_size(&self) -> Option<Size> {
        self.image_size
    }

    /// Where the camera focused, as the file records it (see [`Focus`]):
    /// the camera's own record only, never an estimate from the picture. It
    /// is the primary point of [`focus_points`](Self::focus_points) as a
    /// [`Focus`], with the list's frame and origin, and `None` when there is
    /// no list or the list names no primary.
    ///
    /// It is read on Nikon files, from the MakerNote's AFInfo2 block, as the
    /// camera recorded it. The exception, on a raw file, is a Nikon crop,
    /// any that CropHiSpeed records with a CropArea: there the point is
    /// placed inside the cropped picture, by the sides the crop cuts, as the
    /// [NEF format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md)
    /// describe. DX, 1:1 and 16:9 are the checked cases. It is `None` on a
    /// Nikon file that records no position, such as a DSLR shot through the
    /// viewfinder, which records only an autofocus point's number, and on a
    /// manual-focus shot. It is `None` too when the AFInfo2 version is one
    /// raw-files does not know, and on a crop whose point cannot be placed
    /// on the cropped picture: a 1:1 or 16:9 crop on a small raw, or a crop
    /// whose CropArea cannot be used.
    ///
    /// On Sony files it is read from the MakerNote's FocusLocation, on the
    /// bodies from the ILCE-7RM2 on, as the camera recorded it and as the
    /// [ARW format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md)
    /// describe. Its frame is the picture as the camera framed it, which
    /// differs from the image size on a picture taken at another aspect
    /// ratio than the sensor's, and can on a medium or small raw: the
    /// ILCE-7M4 records the full size there, the ILCE-9M3 the stored size.
    /// It is `None` on a manual-focus shot and on the older bodies, which
    /// record only an autofocus point's number or nothing at all, and its
    /// [`in_focus`](Focus::in_focus) is always `None`: Sony records it in
    /// no plain tag. The area's size is read from the MakerNote's
    /// FocusFrameSize, which the bodies record only from the ILCE-7SM3 on,
    /// so on the bodies before it [`size`](Focus::size) is `None`.
    ///
    /// A [`Format::Jpeg`](crate::Format::Jpeg) file of a Nikon or a Sony
    /// camera gives the point as its raw file would. A Nikon crop-mode JPEG
    /// (DX, 1:1 or 16:9), which has no CropArea, has its point placed on the
    /// JPEG's own picture. A JPEG's point is given only when its frame has
    /// the picture's shape, so an export cut to another shape, or turned
    /// upright, gives `None`; the exception is an export cut from a Nikon
    /// crop-mode JPEG, whose point may be misplaced.
    ///
    /// # Examples
    ///
    /// Where the camera focused, and whether it confirmed focus there:
    ///
    /// ```no_run
    /// use raw_files::RawFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = RawFile::open("photo.nef")?;
    /// if let Some(focus) = file.metadata().focus() {
    ///     let (centre, frame) = (focus.center(), focus.frame());
    ///     println!("focused at {centre} on a frame of {frame} pixels");
    ///     match focus.in_focus() {
    ///         Some(true) => println!("in focus"),
    ///         Some(false) => println!("missed focus"),
    ///         None => println!("focus not recorded"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn focus(&self) -> Option<Focus> {
        self.focus_points.as_ref()?.primary_focus()
    }

    /// The autofocus points the camera used for the picture, in one frame
    /// (see [`FocusPoints`]): each with its number when the file gives one,
    /// the centre of its area, the area's size when recorded, and whether
    /// the camera confirmed focus there when the file says; which of them
    /// is the primary, the one [`focus`](Self::focus) reports; the area
    /// mode, when a checked file names its code; and where the points came
    /// from.
    ///
    /// The Nikon and Sony readers give the position the camera recorded,
    /// the one [`focus`](Self::focus) gives, as a list of one point, its
    /// primary: with no number, with the position's area and whether focus
    /// was confirmed, and with the origin
    /// [`Recorded`](crate::FocusOrigin::Recorded). On a Nikon file the area
    /// mode is read from AFInfo2's area-mode byte, on every version
    /// raw-files reads, and named for the codes that frames shot in a known
    /// mode confirmed, under the detection method the file records: through
    /// the viewfinder, by phase detection, 0
    /// [`Single`](crate::AfAreaMode::Single), 5
    /// [`Dynamic`](crate::AfAreaMode::Dynamic) and 8
    /// [`Auto`](crate::AfAreaMode::Auto); under any detection method, 193
    /// [`Single`](crate::AfAreaMode::Single) and 197
    /// [`Auto`](crate::AfAreaMode::Auto); and under contrast or hybrid
    /// detection, 207 [`Tracking`](crate::AfAreaMode::Tracking). Any other
    /// code, or a DSLR code in live view, gives `None`.
    /// On a Sony file the area mode is always `None`, since the codes of its
    /// AFAreaModeSetting are not checked. It is `None` when
    /// [`focus`](Self::focus) is, for the same reasons.
    ///
    /// # Examples
    ///
    /// Every point the camera used, the primary marked, and the area mode:
    ///
    /// ```no_run
    /// use raw_files::RawFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = RawFile::open("photo.nef")?;
    /// if let Some(points) = file.metadata().focus_points() {
    ///     for point in points.points() {
    ///         let primary = Some(point) == points.primary();
    ///         let mark = if primary { ", the primary" } else { "" };
    ///         println!("a point at {}{mark}", point.center());
    ///     }
    ///     if let Some(mode) = points.area_mode() {
    ///         println!("area mode: {mode}");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn focus_points(&self) -> Option<&FocusPoints> {
        self.focus_points.as_ref()
    }

    /// The layout of the body's autofocus points (see [`FocusLayout`]):
    /// every point the body has, each with its number, the centre of its
    /// box and the box's size, in the frame of raw-files' table of that
    /// body, the body's full-size output.
    ///
    /// The readers give a layout only from a table of the body's points,
    /// for a body whose table is in raw-files, the body the file's Model
    /// names. The
    /// [NEF format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/autofocus-points.md)
    /// record each body measured, and the tables in raw-files are written
    /// from those records, so a body the notes record has a layout only
    /// when its table is in raw-files. It is `None` for every other body
    /// and when the file names no Model. It is given whatever the shot, so a
    /// manual-focus or live-view shot of such a body has it too, and its
    /// frame is always the table's: on a shot in a crop mode, the points
    /// of [`focus_points`](Self::focus_points) are in the crop's frame, not
    /// in the layout's.
    ///
    /// # Examples
    ///
    /// Every point of the body, with the points the camera used marked,
    /// matched by their numbers:
    ///
    /// ```no_run
    /// use raw_files::{FocusPoint, RawFile};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = RawFile::open("photo.nef")?;
    /// let metadata = file.metadata();
    /// if let Some(layout) = metadata.focus_layout() {
    ///     let used: Vec<u8> = metadata
    ///         .focus_points()
    ///         .map(|points| points.points().iter().filter_map(FocusPoint::number).collect())
    ///         .unwrap_or_default();
    ///     for point in layout.points() {
    ///         let number = point.number();
    ///         let mark = if used.contains(&number) { ", used" } else { "" };
    ///         println!("point {number} at {}{mark}", point.center());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn focus_layout(&self) -> Option<FocusLayout> {
        self.focus_layout
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

    /// Sets the software that wrote the file, in place of any set before;
    /// [`software`](Self::software) reads it.
    #[must_use]
    pub fn with_software(mut self, software: impl Into<String>) -> Self {
        self.software = Some(software.into());
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

    /// Sets the camera settings, in place of the ones it had;
    /// [`camera_settings`](Self::camera_settings) reads them.
    #[must_use]
    pub fn with_camera_settings(mut self, settings: CameraSettings) -> Self {
        self.camera_settings = settings;
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

    /// Sets where the camera focused, as a list of that one point, in
    /// place of any focus or list set before: the point with no number, as
    /// the primary, in the focus's frame and with its origin, and no area
    /// mode, as `FocusPoints::from(focus)` makes it.
    /// [`focus`](Self::focus) reads the focus back as given, and
    /// [`focus_points`](Self::focus_points) gives the list; a later
    /// `with_focus` or [`with_focus_points`](Self::with_focus_points)
    /// replaces it.
    #[must_use]
    pub fn with_focus(mut self, focus: Focus) -> Self {
        self.focus_points = Some(FocusPoints::from(focus));
        self
    }

    /// Sets the autofocus points the camera used, in place of any list or
    /// focus set before; [`focus_points`](Self::focus_points) reads it, and
    /// [`focus`](Self::focus) gives its primary.
    #[must_use]
    pub fn with_focus_points(mut self, points: FocusPoints) -> Self {
        self.focus_points = Some(points);
        self
    }

    /// Sets the layout of the body's autofocus points, in place of any set
    /// before; [`focus_layout`](Self::focus_layout) reads it. The focus and
    /// the points are left as they were, as [`with_focus`](Self::with_focus)
    /// and [`with_focus_points`](Self::with_focus_points) leave the layout.
    #[must_use]
    pub fn with_focus_layout(mut self, layout: FocusLayout) -> Self {
        self.focus_layout = Some(layout);
        self
    }
}

/// The exposure of a picture: its sensitivity, how long and how wide the
/// lens was open, at what focal length, also in 35 mm terms, and the
/// compensation chosen.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read. A zero ISO, exposure time, f-number, focal
/// length or focal length in 35 mm terms means the camera did not know it,
/// and the readers never return one: the value is `None`, unless, for the
/// ISO, another of the file's sensitivity tags gives it. A zero exposure
/// bias is a real value.
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
/// exposure time, f-number, focal length or focal length in 35 mm terms.
/// `Exposure` is not `Copy`, so that a later value that cannot be copied
/// does not break callers.
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
    pub(crate) focal_length_in_35mm_film: Option<u32>,
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
    /// stored: the lens's actual focal length, not its 35 mm equivalent,
    /// which [`focal_length_in_35mm_film`](Self::focal_length_in_35mm_film)
    /// gives.
    pub fn focal_length(&self) -> Option<Rational> {
        self.focal_length
    }

    /// The focal length in 35 mm terms the camera recorded, in whole
    /// millimetres: the focal length that would give the same field of
    /// view on a 35 mm film (full-frame) camera, so that pictures from
    /// bodies with different sensor sizes compare, such as 75 for a 50 mm
    /// lens on a Nikon DX body. It is Exif's FocalLengthIn35mmFilm.
    ///
    /// A recorded 0 means the camera did not know it, as with the manual
    /// lens of the public ILCE-7S sample 1582, and the readers give `None`.
    pub fn focal_length_in_35mm_film(&self) -> Option<u32> {
        self.focal_length_in_35mm_film
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

    /// Sets the focal length in 35 mm terms, in whole millimetres, in place
    /// of any set before;
    /// [`focal_length_in_35mm_film`](Self::focal_length_in_35mm_film) reads
    /// it.
    #[must_use]
    pub fn with_focal_length_in_35mm_film(mut self, millimetres: u32) -> Self {
        self.focal_length_in_35mm_film = Some(millimetres);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_settings::{
        ColorSpace, ExposureProgram, MeteringMode, RawImageSize, Shutter,
    };
    use crate::focus::{AfAreaMode, FocusOrigin, FocusPoint, LayoutPoint};
    use crate::geometry::Point;
    use crate::testing::{point, size, z9_focus};

    fn fraction(numerator: u32, denominator: u32) -> Rational {
        Rational {
            numerator,
            denominator,
        }
    }

    /// An exposure of 1/320 s at f/9, 14 mm, 14 mm in 35 mm terms too on a
    /// full-frame body, and ISO 64, with -2/3 EV of compensation, as a
    /// camera stores it.
    fn z9_exposure() -> Exposure {
        Exposure {
            iso: Some(64),
            exposure_time: Some(fraction(10, 3200)),
            f_number: Some(fraction(900, 100)),
            focal_length: Some(fraction(140, 10)),
            focal_length_in_35mm_film: Some(14),
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
        assert_eq!(exposure.focal_length_in_35mm_film(), None);
        assert_eq!(exposure.exposure_bias(), None);
    }

    #[test]
    fn exposure_getters_return_what_was_set() {
        let exposure = z9_exposure();
        assert_eq!(exposure.iso(), Some(64));
        assert_eq!(exposure.exposure_time(), Some(fraction(10, 3200)));
        assert_eq!(exposure.f_number(), Some(fraction(900, 100)));
        assert_eq!(exposure.focal_length(), Some(fraction(140, 10)));
        assert_eq!(exposure.focal_length_in_35mm_film(), Some(14));
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

    /// Camera settings of a Z 9 picture: aperture priority, matrix
    /// metering and the electronic front curtain.
    fn z9_camera_settings() -> CameraSettings {
        CameraSettings::default()
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_shutter(Shutter::ElectronicFrontCurtain)
    }

    /// The metadata of a Z 9 picture, every value set but the layout of the
    /// body's autofocus points, which a Z 9 has none of, and each one
    /// different from the others, so that a getter reading the wrong value
    /// is caught.
    fn z9_metadata() -> Metadata {
        Metadata {
            make: Some("NIKON CORPORATION".to_owned()),
            model: Some("NIKON Z 9".to_owned()),
            software: Some("Ver.01.00".to_owned()),
            serial_number: Some("3001234".to_owned()),
            capture_time: DateTime::from_exif(b"2024:05:01 10:20:30", Some(b"85"), Some(b"+02:00")),
            orientation: Orientation::Rotate90,
            exposure: z9_exposure(),
            camera_settings: z9_camera_settings(),
            lens: Lens::builder().model("NIKKOR Z 14-30mm f/4 S").build(),
            raw_size: Some(Size {
                width: 8_280,
                height: 5_520,
            }),
            image_size: Some(Size {
                width: 8_256,
                height: 5_504,
            }),
            focus_points: Some(FocusPoints::from(z9_focus())),
            focus_layout: None,
        }
    }

    /// Two points of the Nikon D600's layout, as the record of the
    /// maintainer's D600 in the NEF format notes gives them: the centre
    /// point, 1, and the top point of the leftmost column, 38.
    static D600_POINTS: [LayoutPoint; 2] = [
        LayoutPoint::new(1, D600_CENTRE, D600_BOX),
        LayoutPoint::new(38, Point { x: 1_812, y: 1_705 }, D600_BOX),
    ];

    /// The D600's centre point alone.
    static D600_CENTRE_POINT: [LayoutPoint; 1] = [LayoutPoint::new(1, D600_CENTRE, D600_BOX)];

    /// Where the D600's centre point lies, and the box of every point.
    const D600_CENTRE: Point = Point { x: 3_008, y: 2_007 };
    const D600_BOX: Size = Size {
        width: 164,
        height: 164,
    };

    /// A made-up layout of two of the D600's points, in its 6016 × 4016
    /// frame: unlike any other value of these tests.
    fn d600_layout() -> FocusLayout {
        FocusLayout::new("NIKON D600", size(6_016, 4_016), &D600_POINTS).unwrap()
    }

    /// A made-up layout of the D600's centre point alone: unlike
    /// [`d600_layout`].
    fn d600_centre_layout() -> FocusLayout {
        FocusLayout::new("NIKON D600", size(6_016, 4_016), &D600_CENTRE_POINT).unwrap()
    }

    /// Two points placed from a table in a D600-sized frame, the second
    /// the primary, in auto area: a list unlike the Z 9's in every value.
    fn two_points() -> FocusPoints {
        FocusPoints::new(size(6_016, 4_016), FocusOrigin::Table)
            .and_then(|list| list.with_point(FocusPoint::new(point(1_000, 1_000)).with_number(1)))
            .and_then(|list| {
                list.with_point(
                    FocusPoint::new(point(3_008, 2_008))
                        .with_number(21)
                        .with_size(size(164, 164)),
                )
            })
            .and_then(|list| list.with_primary(1))
            .unwrap()
            .with_area_mode(AfAreaMode::Auto)
    }

    #[test]
    fn default_metadata_is_empty() {
        let metadata = Metadata::default();
        assert_eq!(metadata.make(), None);
        assert_eq!(metadata.model(), None);
        assert_eq!(metadata.software(), None);
        assert_eq!(metadata.serial_number(), None);
        assert_eq!(metadata.capture_time(), None);
        assert_eq!(metadata.orientation(), Orientation::NoTransforms);
        assert_eq!(metadata.exposure(), &Exposure::default());
        assert_eq!(metadata.camera_settings(), &CameraSettings::default());
        assert_eq!(metadata.lens(), None);
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), None);
        assert_eq!(metadata.focus(), None);
        assert_eq!(metadata.focus_points(), None);
        assert_eq!(metadata.focus_layout(), None);
    }

    #[test]
    fn metadata_getters_return_what_was_set() {
        let metadata = z9_metadata();
        assert_eq!(metadata.make(), Some("NIKON CORPORATION"));
        assert_eq!(metadata.model(), Some("NIKON Z 9"));
        assert_eq!(metadata.software(), Some("Ver.01.00"));
        assert_eq!(metadata.serial_number(), Some("3001234"));
        let time = metadata.capture_time().unwrap();
        assert_eq!(time.to_string(), "2024-05-01T10:20:30.85+02:00");
        assert_eq!(metadata.orientation(), Orientation::Rotate90);
        assert_eq!(metadata.exposure(), &z9_exposure());
        assert_eq!(metadata.camera_settings(), &z9_camera_settings());
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
        assert_eq!(
            metadata.focus_points(),
            Some(&FocusPoints::from(z9_focus()))
        );
        // A Z 9 has no layout; one set beside its values is read back.
        assert_eq!(metadata.focus_layout(), None);
        let with_layout = Metadata {
            focus_layout: Some(d600_layout()),
            ..z9_metadata()
        };
        assert_eq!(with_layout.focus_layout(), Some(d600_layout()));
        assert_eq!(
            with_layout.focus_points(),
            Some(&FocusPoints::from(z9_focus()))
        );
    }

    #[test]
    fn focus_is_the_primary_of_the_points() {
        let metadata = Metadata::default().with_focus_points(two_points());
        let focus = Focus::new(size(6_016, 4_016), point(3_008, 2_008))
            .and_then(|focus| focus.with_size(size(164, 164)))
            .unwrap()
            .with_origin(FocusOrigin::Table);
        assert_eq!(metadata.focus(), Some(focus));
        assert_eq!(metadata.focus_points(), Some(&two_points()));

        // The first point as the primary gives that one instead.
        let first = two_points().with_primary(0).unwrap();
        let metadata = Metadata::default().with_focus_points(first);
        let focus = Focus::new(size(6_016, 4_016), point(1_000, 1_000))
            .unwrap()
            .with_origin(FocusOrigin::Table);
        assert_eq!(metadata.focus(), Some(focus));
    }

    #[test]
    fn a_list_without_a_primary_gives_no_focus() {
        let unmarked = FocusPoints::new(size(6_016, 4_016), FocusOrigin::Recorded)
            .and_then(|list| list.with_point(FocusPoint::new(point(3_008, 2_008))))
            .unwrap()
            .with_area_mode(AfAreaMode::Single);
        let metadata = Metadata::default().with_focus_points(unmarked.clone());
        assert_eq!(metadata.focus(), None);
        assert_eq!(metadata.focus_points(), Some(&unmarked));
        // Nor does an empty list.
        let empty = FocusPoints::new(size(6_016, 4_016), FocusOrigin::Recorded).unwrap();
        let metadata = Metadata::default().with_focus_points(empty);
        assert_eq!(metadata.focus(), None);
        assert!(metadata.focus_points().is_some());
    }

    #[test]
    fn with_focus_stores_a_list_of_that_one_point() {
        let without_area = Focus::new(size(7, 3), point(6, 0)).unwrap();
        for focus in [
            z9_focus(),
            z9_focus().with_origin(FocusOrigin::Table),
            without_area,
        ] {
            let metadata = Metadata::default().with_focus(focus);
            let points = metadata.focus_points().unwrap();
            assert_eq!(points.frame(), focus.frame(), "{focus:?}");
            assert_eq!(points.origin(), focus.origin(), "{focus:?}");
            assert_eq!(points.area_mode(), None, "{focus:?}");
            let only = FocusPoint::new(focus.center());
            let only = match focus.size() {
                Some(size) => only.with_size(size),
                None => only,
            };
            let only = match focus.in_focus() {
                Some(in_focus) => only.with_in_focus(in_focus),
                None => only,
            };
            assert_eq!(points.points(), [only], "{focus:?}");
            assert_eq!(points.primary(), Some(&only), "{focus:?}");
            assert_eq!(points.primary().unwrap().number(), None, "{focus:?}");
            assert_eq!(metadata.focus(), Some(focus), "{focus:?}");
        }
    }

    #[test]
    fn the_layout_and_the_points_used_are_set_apart() {
        let (layout, list, z9) = (d600_layout(), two_points(), z9_focus());
        // A layout alone gives no points and no focus.
        let alone = Metadata::default().with_focus_layout(layout);
        assert_eq!(alone.focus_layout(), Some(layout));
        assert_eq!(alone.focus_points(), None);
        assert_eq!(alone.focus(), None);
        // A layout set after the points, or after the focus, leaves them.
        let listed = Metadata::default()
            .with_focus_points(list.clone())
            .with_focus_layout(layout);
        assert_eq!(listed.focus_points(), Some(&list));
        assert_eq!(listed.focus_layout(), Some(layout));
        let focused = Metadata::default().with_focus(z9).with_focus_layout(layout);
        assert_eq!(focused.focus(), Some(z9));
        assert_eq!(focused.focus_layout(), Some(layout));
        // The points, or the focus, set after the layout leave it.
        let listed_after = Metadata::default()
            .with_focus_layout(layout)
            .with_focus_points(list.clone());
        assert_eq!(listed_after, listed);
        let focused_after = Metadata::default().with_focus_layout(layout).with_focus(z9);
        assert_eq!(focused_after, focused);
    }

    #[test]
    fn a_later_focus_or_list_replaces_the_earlier_one() {
        let one = Focus::new(size(1, 1), point(0, 0)).unwrap();
        let z9 = z9_focus();
        let list = two_points();
        // Each pair in turn, the later one kept whatever the earlier.
        let by_focus = |metadata: Metadata| metadata.with_focus(z9);
        let by_list = |metadata: Metadata| metadata.with_focus_points(list.clone());
        for earlier in [
            Metadata::default().with_focus(one),
            Metadata::default().with_focus_points(list.clone().with_primary(0).unwrap()),
        ] {
            let focused = by_focus(earlier.clone());
            assert_eq!(focused.focus(), Some(z9), "{earlier:?}");
            assert_eq!(
                focused.focus_points(),
                Some(&FocusPoints::from(z9)),
                "{earlier:?}"
            );
            let listed = by_list(earlier.clone());
            assert_eq!(listed.focus_points(), Some(&list), "{earlier:?}");
            assert_eq!(listed.focus(), list.primary_focus(), "{earlier:?}");
        }
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
                    start.clone().with_focal_length_in_35mm_film(36),
                    Exposure {
                        focal_length_in_35mm_film: Some(36),
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
        let points = two_points();
        let settings = CameraSettings::default().with_color_space(ColorSpace::Srgb);
        // From the empty metadata, and from a full one whose every value
        // differs from the one set, so that a builder writing another
        // value, or clearing one, is caught: the Z 9's, which has no
        // layout, and the Z 9's with a layout, so that a builder clearing
        // the layout is caught too. The expected value is built field by
        // field, not through the builders.
        let z9_with_layout = Metadata {
            focus_layout: Some(d600_centre_layout()),
            ..z9_metadata()
        };
        for start in [Metadata::default(), z9_metadata(), z9_with_layout] {
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
                    start.clone().with_software("Ver.01.10"),
                    Metadata {
                        software: Some("Ver.01.10".to_owned()),
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
                    start.clone().with_camera_settings(settings.clone()),
                    Metadata {
                        camera_settings: settings.clone(),
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
                        focus_points: Some(FocusPoints::from(focus)),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_focus_points(points.clone()),
                    Metadata {
                        focus_points: Some(points.clone()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_focus_layout(d600_layout()),
                    Metadata {
                        focus_layout: Some(d600_layout()),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
    }

    /// [`z9_metadata`] built through twelve of its fourteen builders, all
    /// but [`Metadata::with_focus_points`], whose list `with_focus` makes,
    /// and [`Metadata::with_focus_layout`], since a Z 9 has no layout.
    fn z9_metadata_built() -> Metadata {
        let lens = Lens::builder()
            .model("NIKKOR Z 14-30mm f/4 S")
            .build()
            .unwrap();
        Metadata::default()
            .with_make("NIKON CORPORATION")
            .with_model("NIKON Z 9")
            .with_software("Ver.01.00")
            .with_serial_number("3001234")
            .with_capture_time(
                DateTime::from_exif(b"2024:05:01 10:20:30", Some(b"85"), Some(b"+02:00")).unwrap(),
            )
            .with_orientation(Orientation::Rotate90)
            .with_exposure(z9_exposure_built())
            .with_camera_settings(z9_camera_settings())
            .with_lens(lens)
            .with_raw_size(size(8_280, 5_520))
            .with_image_size(size(8_256, 5_504))
            .with_focus(z9_focus())
    }

    /// [`z9_exposure`] built through the six builders.
    fn z9_exposure_built() -> Exposure {
        Exposure::default()
            .with_iso(64)
            .with_exposure_time(fraction(10, 3200))
            .with_f_number(fraction(900, 100))
            .with_focal_length(fraction(140, 10))
            .with_focal_length_in_35mm_film(14)
            .with_exposure_bias(SignedRational {
                numerator: -4,
                denominator: 6,
            })
    }

    #[test]
    fn every_builder_in_a_chain_equals_the_value_built_field_by_field() {
        assert_eq!(z9_exposure_built(), z9_exposure());
        assert_eq!(z9_metadata_built(), z9_metadata());
        // The fourteenth builder at the end of the chain.
        assert_eq!(
            z9_metadata_built().with_focus_layout(d600_layout()),
            Metadata {
                focus_layout: Some(d600_layout()),
                ..z9_metadata()
            }
        );
    }

    #[test]
    fn a_later_exposure_or_metadata_value_replaces_an_earlier_one() {
        let one = fraction(1, 1);
        let exposure = Exposure::default()
            .with_iso(1)
            .with_exposure_time(one)
            .with_f_number(one)
            .with_focal_length(one)
            .with_focal_length_in_35mm_film(1)
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
            .with_focal_length_in_35mm_film(14)
            .with_exposure_bias(SignedRational {
                numerator: -4,
                denominator: 6,
            });
        assert_eq!(exposure, z9_exposure());

        let earlier = Metadata::default()
            .with_make("A")
            .with_model("A")
            .with_software("A")
            .with_serial_number("A")
            .with_capture_time(DateTime::new(2000, 1, 1, 0, 0, 0).unwrap())
            .with_orientation(Orientation::Rotate180)
            .with_exposure(Exposure::default().with_iso(1))
            .with_camera_settings(
                CameraSettings::default().with_raw_image_size(RawImageSize::Small),
            )
            .with_lens(Lens::builder().model("A").build().unwrap())
            .with_raw_size(size(1, 1))
            .with_image_size(size(1, 1))
            .with_focus_points(two_points())
            .with_focus_layout(d600_centre_layout());
        // Every value of `earlier` differs from the Z 9's, and its layout
        // from the one set below, so each builder below has one to
        // replace.
        let z9 = Metadata {
            focus_layout: Some(d600_layout()),
            ..z9_metadata()
        };
        let metadata = earlier
            .with_make("NIKON CORPORATION")
            .with_model("NIKON Z 9")
            .with_software("Ver.01.00")
            .with_serial_number("3001234")
            .with_capture_time(z9.capture_time.unwrap())
            .with_orientation(z9.orientation)
            .with_exposure(z9.exposure.clone())
            .with_camera_settings(z9.camera_settings.clone())
            .with_lens(z9.lens.clone().unwrap())
            .with_raw_size(z9.raw_size.unwrap())
            .with_image_size(z9.image_size.unwrap())
            .with_focus(z9.focus().unwrap())
            .with_focus_layout(d600_layout());
        assert_eq!(metadata, z9);
    }

    #[test]
    fn the_metadata_builders_move_an_owned_text_rather_than_copy_it() {
        let make = String::from("NIKON CORPORATION");
        let model = String::from("NIKON Z 9");
        let software = String::from("Ver.01.00");
        let serial_number = String::from("3001234");
        let addresses = [
            make.as_ptr(),
            model.as_ptr(),
            software.as_ptr(),
            serial_number.as_ptr(),
        ];
        let metadata = Metadata::default()
            .with_make(make)
            .with_model(model)
            .with_software(software)
            .with_serial_number(serial_number);
        let texts = [
            metadata.make(),
            metadata.model(),
            metadata.software(),
            metadata.serial_number(),
        ];
        assert_eq!(
            texts,
            [
                Some("NIKON CORPORATION"),
                Some("NIKON Z 9"),
                Some("Ver.01.00"),
                Some("3001234")
            ]
        );
        // The very texts given, not copies of them.
        assert_eq!(texts.map(|text| text.map(str::as_ptr)), addresses.map(Some));
    }

    #[test]
    fn the_builders_store_values_as_given() {
        // Neither cleaned nor refused, as the readers would a file's: a
        // padded or empty text, a zero ISO, fraction or focal length in
        // 35 mm terms, and an image size with no raw size.
        let zero = Rational {
            numerator: 0,
            denominator: 1,
        };
        let exposure = Exposure::default()
            .with_iso(0)
            .with_exposure_time(zero)
            .with_f_number(zero)
            .with_focal_length(zero)
            .with_focal_length_in_35mm_film(0);
        assert_eq!(exposure.iso(), Some(0));
        assert_eq!(exposure.exposure_time(), Some(zero));
        assert_eq!(exposure.f_number(), Some(zero));
        assert_eq!(exposure.focal_length(), Some(zero));
        assert_eq!(exposure.focal_length_in_35mm_film(), Some(0));

        let metadata = Metadata::default()
            .with_make("  padded  ")
            .with_model("NIKON Z 9\0\0")
            .with_software("Ver.1.10 ")
            .with_serial_number("")
            .with_image_size(size(0, 0));
        assert_eq!(metadata.make(), Some("  padded  "));
        assert_eq!(metadata.model(), Some("NIKON Z 9\0\0"));
        assert_eq!(metadata.software(), Some("Ver.1.10 "));
        assert_eq!(metadata.serial_number(), Some(""));
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), Some(size(0, 0)));
    }
}
