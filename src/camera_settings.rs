// SPDX-License-Identifier: MIT OR Apache-2.0

//! How the camera was set for a picture.
//!
//! Beside the exposure, a viewer's information panel shows how the camera
//! was set for the shot: the white balance, the exposure program, the
//! metering mode, the flash, the shutter, the image area, the raw image
//! size, the color space, the focus mode, the AF-area mode, AF fine-tune,
//! the release mode, bracketing and whether the clock was on daylight
//! saving time. [`CameraSettings`] holds them all, and
//! [`Metadata::camera_settings`](crate::Metadata::camera_settings) gives
//! them for a file. [`WhiteBalance`], [`FineTune`], [`Flash`] and
//! [`AfFineTune`] hold the settings made of several parts; each enum, such as [`ExposureProgram`],
//! names one setting's choices in words that fit any maker, with a
//! [`name`](ExposureProgram::name) to show.
//!
//! Every value is optional. A file may not record it, or may record it in
//! a way that cannot be read, such as a code no variant names; either way
//! the value is `None`, and a damaged value costs only itself.
//!
//! NEF, ARW and JPEG files give the settings that their standard Exif tags
//! record: the exposure program, the metering mode, the flash, the white
//! balance mode, the color space and exposure bracketing. The others are
//! recorded only in a
//! maker's own tags, its MakerNote. Nikon's, in NEF files and in the JPEGs
//! of Nikon cameras, gives the white balance's color temperature and
//! fine-tuning, the shutter, the image area, the focus mode, the AF-area
//! mode, AF fine-tune, the release mode and bracketing, daylight saving
//! time and, in a raw file, the raw image size; the white balance setting,
//! finer than Exif's,
//! in place of Exif's; and the color space where the Exif tags record
//! none, as in every Nikon raw file checked. Sony's, in ARW files and
//! their family and in the JPEGs of Sony cameras, gives the white
//! balance's fine-tuning, the electronic front curtain setting, which only
//! Sony records, the color temperature of a chosen color temperature or a
//! custom white balance, the focus mode, the AF-area mode, the release
//! mode and bracketing; and the white balance setting in place of Exif's. On Sony files the shutter,
//! the image area, the raw image size, AF fine-tune and daylight saving
//! time are `None`: Sony records the first two only in enciphered blocks,
//! which raw-files does not read, gives an APS-C raw the raw size code of
//! a medium one, keeps its AF micro adjustment in enciphered or unread
//! blocks, and records no daylight saving time in a plain tag.
//!
//! A program that uses raw-files can build these values for its own
//! tests, without a camera file: [`CameraSettings`] and [`WhiteBalance`]
//! through their `with_…` methods, and a [`FineTune`], a [`Flash`] and an
//! [`AfFineTune`] through their constructors. [`CameraSettings`]'s docs show how.

use std::fmt;

use crate::rational::SignedRational;

/// How the camera was set for a picture: its white balance, exposure
/// program, metering mode, flash, shutter, image area, raw image size,
/// color space, focus mode, AF-area mode, AF fine-tune, release mode,
/// bracketing and daylight saving time, for a viewer's information panel
/// beside the exposure.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read, such as a code that no variant names. A
/// damaged value costs only itself.
/// [`Metadata::camera_settings`](crate::Metadata::camera_settings) gives
/// the values a file's standard Exif tags record: the exposure program,
/// the metering mode, the flash, the white balance mode, the color space
/// and exposure bracketing. The others, which only a maker's MakerNote records, it gives
/// from Nikon's MakerNote and Sony's, each those its MakerNote records in
/// plain tags; the shutter, the image area, the raw image size, AF
/// fine-tune and daylight saving time are `None` for every Sony file. Each
/// getter says where its value comes from.
///
/// `CameraSettings::default()` is the empty record, with every value
/// `None` and an empty [`WhiteBalance`]. A caller's tests start from it and
/// set the values they need with the `with_…` methods, such as
/// [`with_metering_mode`](Self::with_metering_mode), each named after the
/// method that reads the value back and each replacing any value set
/// before. The empty record is the start of every chain.
///
/// The `with_…` methods store each value as given: none refuses or changes
/// one. `CameraSettings` is `Clone` and not `Copy`.
///
/// # Examples
///
/// The settings a file records, each on its own, since each is optional:
///
/// ```no_run
/// use raw_files::CameraFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = CameraFile::open("photo.nef")?;
/// let settings = file.metadata().camera_settings();
/// if let Some(program) = settings.exposure_program() {
///     println!("program: {program}");
/// }
/// if let Some(mode) = settings.white_balance().mode() {
///     println!("white balance: {mode}");
/// }
/// if let Some(flash) = settings.flash() {
///     println!("flash fired: {}", flash.fired());
/// }
/// # Ok(())
/// # }
/// ```
///
/// A program's tests build the settings without a camera file, from the
/// empty record, and read back what they set; what they do not set stays
/// `None`:
///
/// ```
/// use raw_files::{
///     CameraSettings, ExposureProgram, Flash, FlashMode, MeteringMode, WhiteBalance,
///     WhiteBalanceMode,
/// };
///
/// let settings = CameraSettings::default()
///     .with_exposure_program(ExposureProgram::AperturePriority)
///     .with_metering_mode(MeteringMode::MultiSegment)
///     .with_white_balance(
///         WhiteBalance::default()
///             .with_mode(WhiteBalanceMode::Auto)
///             .with_color_temperature(5_200),
///     )
///     .with_flash(Flash::new(false).with_mode(FlashMode::Off));
///
/// assert_eq!(
///     settings.exposure_program().map(ExposureProgram::name),
///     Some("Aperture priority")
/// );
/// assert_eq!(settings.metering_mode(), Some(MeteringMode::MultiSegment));
/// assert_eq!(settings.white_balance().color_temperature(), Some(5_200));
/// assert_eq!(settings.flash().map(|flash| flash.fired()), Some(false));
/// assert_eq!(settings.shutter(), None);
/// assert_eq!(settings.color_space(), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CameraSettings {
    pub(crate) white_balance: WhiteBalance,
    pub(crate) exposure_program: Option<ExposureProgram>,
    pub(crate) metering_mode: Option<MeteringMode>,
    pub(crate) flash: Option<Flash>,
    pub(crate) shutter: Option<Shutter>,
    pub(crate) electronic_front_curtain_setting: Option<bool>,
    pub(crate) image_area: Option<ImageArea>,
    pub(crate) raw_image_size: Option<RawImageSize>,
    pub(crate) color_space: Option<ColorSpace>,
    pub(crate) focus_mode: Option<FocusMode>,
    pub(crate) focus_area_mode: Option<FocusAreaMode>,
    pub(crate) af_fine_tune: Option<AfFineTune>,
    pub(crate) release_mode: Option<ReleaseMode>,
    pub(crate) bracketing: Option<Bracketing>,
    pub(crate) daylight_saving_time: Option<bool>,
}

impl CameraSettings {
    /// The white balance the camera was set to. Always present, though
    /// each of its parts is optional.
    pub fn white_balance(&self) -> &WhiteBalance {
        &self.white_balance
    }

    /// The exposure program: how the camera chose the exposure, such as
    /// [`AperturePriority`](ExposureProgram::AperturePriority).
    ///
    /// Read from Exif's ExposureProgram. `None` also when the camera
    /// records its 0, "not defined", as some cameras do.
    pub fn exposure_program(&self) -> Option<ExposureProgram> {
        self.exposure_program
    }

    /// How the camera metered the light, such as
    /// [`MultiSegment`](MeteringMode::MultiSegment).
    ///
    /// Read from Exif's MeteringMode. `None` also when the camera records
    /// its 0, "unknown".
    pub fn metering_mode(&self) -> Option<MeteringMode> {
        self.metering_mode
    }

    /// Whether the flash fired, and its mode when the file records one.
    ///
    /// Read from Exif's Flash. `None` also when the value contradicts
    /// itself, as a flash that fired on a camera without a flash function
    /// does, or sets bits Exif reserves.
    pub fn flash(&self) -> Option<Flash> {
        self.flash
    }

    /// The shutter the camera used for the picture: on an auto setting, the
    /// one the camera chose.
    ///
    /// Read from Nikon's MakerNote, its ShutterMode, which the Nikon 1 J1,
    /// the D850, the D780 and the Z bodies checked record; no checked DSLR
    /// older than the D850 does. A code for a movie's shutter, or one no
    /// checked file holds, gives `None`. A Sony body records it only in an
    /// enciphered block, which raw-files does not read. Its
    /// [`electronic_front_curtain_setting`](Self::electronic_front_curtain_setting)
    /// is a setting, not the shutter used.
    pub fn shutter(&self) -> Option<Shutter> {
        self.shutter
    }

    /// Whether Sony's electronic front curtain setting was on: `Some(true)`
    /// when on, `Some(false)` when off.
    ///
    /// It is the menu setting, not the shutter used, which
    /// [`shutter`](Self::shutter) gives: a body may use another shutter,
    /// such as a silent electronic one, with the setting on.
    ///
    /// Read from Sony's MakerNote, its ElectronicFrontCurtainShutter, 0 off
    /// and 1 on, which the SLT-A99V, the ILCE-7S and every later Sony body
    /// checked record; any other value gives `None`. Only Sony records it,
    /// so a file of any other maker gives `None`.
    pub fn electronic_front_curtain_setting(&self) -> Option<bool> {
        self.electronic_front_curtain_setting
    }

    /// The part of the sensor the picture uses, such as the whole of a
    /// full-frame sensor or its APS-C crop; [`ImageArea`] says which of a
    /// maker's settings gives each.
    ///
    /// Read from Nikon's MakerNote, the crop setting of its CropHiSpeed,
    /// which the D200 and every later body checked but the Coolpix P7800
    /// record. Nikon's "off" setting, which some of those bodies write,
    /// gives `None`, since it does not say which sensor the body has. A
    /// Sony body records its APS-C mode only in an enciphered block, which
    /// raw-files does not read.
    pub fn image_area(&self) -> Option<ImageArea> {
        self.image_area
    }

    /// The raw size the camera was set to: large, medium or small. The
    /// size in pixels is [`Metadata::raw_size`](crate::Metadata::raw_size).
    ///
    /// Read from Nikon's MakerNote, its ImageSizeRAW, which the D750, the
    /// D810, the D5600 and every later body checked record in their raw
    /// files. No raw file checked is a medium one, so no file gives
    /// [`Medium`](RawImageSize::Medium) yet. A JPEG file has none, whatever
    /// its MakerNote records, since it holds no raw data: the Z 9 writes
    /// the JPEG's own size there. A Sony file has none either, since its
    /// tag gives an APS-C raw the code of a medium one.
    pub fn raw_image_size(&self) -> Option<RawImageSize> {
        self.raw_image_size
    }

    /// The color space the camera was set to, such as sRGB.
    ///
    /// Read from Exif's ColorSpace, with the Interoperability IFD's index
    /// for Adobe RGB, as [`ColorSpace`] says. Only when the Exif tags hold
    /// no ColorSpace at all, as in every Nikon raw file checked, is it read
    /// from Nikon's MakerNote, which the D200 and every later body checked
    /// but the Coolpix P7800 record it in. A ColorSpace that gives none,
    /// such as the "uncalibrated" a program may write when it rewrites a
    /// JPEG, is never overruled by a MakerNote the program left behind,
    /// which may no longer describe the picture.
    pub fn color_space(&self) -> Option<ColorSpace> {
        self.color_space
    }

    /// How the camera focused: by hand, or in which autofocus mode, such
    /// as [`Continuous`](FocusMode::Continuous); [`FocusMode`] says which
    /// of a maker's settings gives each.
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's FocusMode text, without its padding, looked up whole:
    ///   `MANUAL`, `AF-S`, `AF-C`, `AF-A` and `AF-F`;
    /// - Sony's FocusMode code: 0 manual focus, 2 AF-S, 3 AF-C, 4 AF-A,
    ///   6 DMF and 7 AF-D, continuous AF with a depth map assisting.
    ///
    /// `None` when:
    ///
    /// - the file records no focus mode, as the Coolpix JPEGs whose
    ///   MakerNote has the short header do, and the Sony A-mount DSLRs,
    ///   the first NEX and SLT bodies, the RX100 and the SR2 and SRF files;
    /// - it records a text outside the list above, or a code the
    ///   documentation does not give, such as Sony's 1 or 5;
    /// - the tag has another type or count than the maker writes.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, FocusMode};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// match file.metadata().camera_settings().focus_mode() {
    ///     Some(FocusMode::Manual) => println!("focused by hand"),
    ///     Some(mode) => println!("focus: {mode}"),
    ///     None => println!("no focus mode recorded"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn focus_mode(&self) -> Option<FocusMode> {
        self.focus_mode
    }

    /// How the camera chose its autofocus points, the AF-area mode, such
    /// as [`Auto`](FocusAreaMode::Auto).
    ///
    /// It is a setting: it is given whatever the focus mode, manual focus
    /// included, and whether or not the file gives
    /// [focus points](crate::Metadata::focus_points). A viewer labels the
    /// points with it, or shows it beside the other settings.
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's AFInfo2, at the versions whose layout raw-files reads,
    ///   the Nikon 1 bodies' included, by the code and the detection
    ///   method the record names, in the version's list of codes;
    /// - on the Nikon bodies without AFInfo2, from the D1 to the D80 and
    ///   the Coolpix bodies, AFInfo's first byte;
    /// - Sony's AFAreaModeSetting, by the codes of the body's family,
    ///   which the Model gives: the SLT bodies, the ILCA bodies, or every
    ///   other body, the E-mount bodies and compacts among them.
    ///
    /// `None` when:
    ///
    /// - the file records no AF-area mode, as the Sony A-mount DSLRs, the
    ///   first NEX and SLT bodies and the SR2 and SRF files do;
    /// - it records a code that no variant names, such as Nikon's face
    ///   priority in a DSLR's live view, or a code that its list does not
    ///   hold;
    /// - Nikon's AFInfo2 records hybrid detection at a version from 0100 to
    ///   0301 with a DSLR code, from 0 to 18, which no list gives for that
    ///   method, or a detection method the documentation does not name
    ///   (byte 4 above 2) at those versions;
    /// - AFInfo2 is of a version whose layout raw-files does not read, is
    ///   shorter than its layout, or has another type than Nikon writes;
    /// - a Sony file has no Model, which gives the body's family.
    ///
    /// The codes named are in the notes on
    /// [Nikon's focus point](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#86-focus-point)
    /// and
    /// [Sony's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// let settings = file.metadata().camera_settings();
    /// if let Some(mode) = settings.focus_area_mode() {
    ///     println!("AF-area mode: {mode}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn focus_area_mode(&self) -> Option<FocusAreaMode> {
        self.focus_area_mode
    }

    /// AF fine-tune: whether the photographer's correction of where the
    /// autofocus lands was on, and its value when it was; [`AfFineTune`]
    /// says what each holds.
    ///
    /// Read from Nikon's MakerNote, its AFTune: four bytes, whether it is
    /// on, the saved lens's index, the value, and the value at a zoom's
    /// long end, which is given when the first byte says the fine-tune is
    /// on for a zoom. The saved lens's index is not given.
    ///
    /// `None` when:
    ///
    /// - the file records no AF fine-tune, as every Sony file does: the
    ///   E-mount bodies record their AF micro adjustment only in
    ///   enciphered blocks, and the A-mount bodies in a plain block whose
    ///   layout raw-files does not read;
    /// - the first byte is a code the documentation does not name;
    /// - AFTune has another type or count than Nikon writes.
    ///
    /// What the files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(fine_tune) = file.metadata().camera_settings().af_fine_tune() {
    ///     match fine_tune.value() {
    ///         Some(value) => println!("AF fine-tune: {value:+}"),
    ///         None => println!("AF fine-tune: off"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn af_fine_tune(&self) -> Option<AfFineTune> {
        self.af_fine_tune
    }

    /// How the shutter was released for the picture: one frame for each
    /// press, a burst, by the self-timer or by a remote, such as
    /// [`Continuous`](ReleaseMode::Continuous); [`ReleaseMode`] says which
    /// of a maker's settings gives each.
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's ShootingMode, a value of bits: bit 0 continuous, bit 3
    ///   the self-timer, bit 7 an infrared remote, and none of the three a
    ///   single frame;
    /// - Sony's ReleaseMode code: 0 a single frame, 2 continuous.
    ///
    /// `None` when:
    ///
    /// - the file records no release mode, as the Nikon bodies before the
    ///   D2X, the Coolpix bodies but the Coolpix A, and the oldest Sony
    ///   bodies do;
    /// - Nikon's ShootingMode sets two of bits 0, 3 and 7, which no file
    ///   does, or bit 11, pre-capture, a burst that begins before the
    ///   shutter button is pressed all the way, which no file does either;
    /// - Sony's code records a bracketed series, which does not say
    ///   whether the frames were shot one at a time or as a burst, or is a
    ///   code the documentation does not give;
    /// - the tag has another type or count than the maker writes.
    ///
    /// Sony records the self-timer and the remote only in blocks raw-files
    /// does not read, and whether a self-timer release writes 0 is not
    /// known: a Sony file gives [`Single`](ReleaseMode::Single) or
    /// [`Continuous`](ReleaseMode::Continuous) alone.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, ReleaseMode};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if file.metadata().camera_settings().release_mode() == Some(ReleaseMode::Continuous) {
    ///     println!("one frame of a burst");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn release_mode(&self) -> Option<ReleaseMode> {
        self.release_mode
    }

    /// Whether the picture is one of a bracketed series, and of which kind:
    /// [`Off`](Bracketing::Off) when it is not, or the setting the series
    /// varied, such as [`Exposure`](Bracketing::Exposure); [`Bracketing`]
    /// says which of a maker's settings gives each.
    ///
    /// Read from the MakerNote where it records the bracketing:
    ///
    /// - Nikon's ShootingMode, a value of bits: bit 4 exposure, bit 6
    ///   white balance and bit 8 Active D-Lighting bracketing, and none of
    ///   the three [`Off`](Bracketing::Off);
    /// - Sony's ReleaseMode code: 6 white balance and 8 DRO bracketing,
    ///   and 0 and 2, a single frame and continuous release,
    ///   [`Off`](Bracketing::Off).
    ///
    /// Where the MakerNote gives none, Exif's ExposureMode 2, auto
    /// bracket, gives [`Exposure`](Bracketing::Exposure); Exif records no
    /// other bracketing, so its other codes give none.
    ///
    /// The MakerNote gives no value, and Exif's stands, when:
    ///
    /// - Nikon's ShootingMode sets two of bits 4, 6 and 8, which no file
    ///   does;
    /// - Sony's code is 5, which ExifTool's documentation names exposure
    ///   bracketing but files contradict: the ILCE-7RM3 writes it on
    ///   pixel-shift frames too, while the RX100M3 and RX100M6 write it
    ///   with Exif's auto bracket, and so give
    ///   [`Exposure`](Bracketing::Exposure);
    /// - the tag has another type or count than its maker writes.
    ///
    /// `None` when neither the MakerNote nor Exif gives one.
    ///
    /// Nikon's bits 6 and 8 are named from the documentation, with the rest
    /// of their list, though no measured file holds either; the notes say
    /// which codes files hold.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{Bracketing, CameraFile};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.arw")?;
    /// match file.metadata().camera_settings().bracketing() {
    ///     Some(Bracketing::Off) | None => {}
    ///     Some(kind) => println!("one of a series: {kind}"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn bracketing(&self) -> Option<Bracketing> {
        self.bracketing
    }

    /// Whether daylight saving time was on in the camera's clock:
    /// `Some(true)` when on, `Some(false)` when off.
    ///
    /// It is a setting of the clock, as the time zone is. Where the
    /// capture time's offset from UTC comes from the clock's time zone, on
    /// the Nikon bodies that record no Exif offset, the offset already
    /// includes the daylight hour
    /// ([`Metadata::capture_time`](crate::Metadata::capture_time)).
    ///
    /// Read from Nikon's MakerNote, the third byte of its WorldTime, 0 off
    /// and 1 on, which the D300 and every later body checked record, and
    /// the Coolpix A; any other value gives `None`. The older Nikon bodies
    /// and the other Coolpix bodies record no WorldTime, and Sony records
    /// no such setting in a plain tag, so their files give `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if file.metadata().camera_settings().daylight_saving_time() == Some(true) {
    ///     println!("the clock was on daylight saving time");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn daylight_saving_time(&self) -> Option<bool> {
        self.daylight_saving_time
    }

    /// Sets the white balance, in place of the one it had;
    /// [`white_balance`](Self::white_balance) reads it.
    #[must_use]
    pub fn with_white_balance(mut self, white_balance: WhiteBalance) -> Self {
        self.white_balance = white_balance;
        self
    }

    /// Sets the exposure program, in place of any set before;
    /// [`exposure_program`](Self::exposure_program) reads it.
    #[must_use]
    pub fn with_exposure_program(mut self, program: ExposureProgram) -> Self {
        self.exposure_program = Some(program);
        self
    }

    /// Sets the metering mode, in place of any set before;
    /// [`metering_mode`](Self::metering_mode) reads it.
    #[must_use]
    pub fn with_metering_mode(mut self, mode: MeteringMode) -> Self {
        self.metering_mode = Some(mode);
        self
    }

    /// Sets the flash, in place of any set before; [`flash`](Self::flash)
    /// reads it.
    #[must_use]
    pub fn with_flash(mut self, flash: Flash) -> Self {
        self.flash = Some(flash);
        self
    }

    /// Sets the shutter used, in place of any set before;
    /// [`shutter`](Self::shutter) reads it.
    #[must_use]
    pub fn with_shutter(mut self, shutter: Shutter) -> Self {
        self.shutter = Some(shutter);
        self
    }

    /// Sets whether the electronic front curtain setting was on, in place
    /// of any value set before;
    /// [`electronic_front_curtain_setting`](Self::electronic_front_curtain_setting)
    /// reads it.
    #[must_use]
    pub fn with_electronic_front_curtain_setting(mut self, on: bool) -> Self {
        self.electronic_front_curtain_setting = Some(on);
        self
    }

    /// Sets the image area, in place of any set before;
    /// [`image_area`](Self::image_area) reads it.
    #[must_use]
    pub fn with_image_area(mut self, area: ImageArea) -> Self {
        self.image_area = Some(area);
        self
    }

    /// Sets the raw image size, in place of any set before;
    /// [`raw_image_size`](Self::raw_image_size) reads it.
    #[must_use]
    pub fn with_raw_image_size(mut self, size: RawImageSize) -> Self {
        self.raw_image_size = Some(size);
        self
    }

    /// Sets the color space, in place of any set before;
    /// [`color_space`](Self::color_space) reads it.
    #[must_use]
    pub fn with_color_space(mut self, color_space: ColorSpace) -> Self {
        self.color_space = Some(color_space);
        self
    }

    /// Sets the focus mode, in place of any set before;
    /// [`focus_mode`](Self::focus_mode) reads it.
    #[must_use]
    pub fn with_focus_mode(mut self, mode: FocusMode) -> Self {
        self.focus_mode = Some(mode);
        self
    }

    /// Sets the AF-area mode, in place of any set before;
    /// [`focus_area_mode`](Self::focus_area_mode) reads it.
    #[must_use]
    pub fn with_focus_area_mode(mut self, mode: FocusAreaMode) -> Self {
        self.focus_area_mode = Some(mode);
        self
    }

    /// Sets AF fine-tune, in place of any set before;
    /// [`af_fine_tune`](Self::af_fine_tune) reads it.
    #[must_use]
    pub fn with_af_fine_tune(mut self, fine_tune: AfFineTune) -> Self {
        self.af_fine_tune = Some(fine_tune);
        self
    }

    /// Sets the release mode, in place of any set before;
    /// [`release_mode`](Self::release_mode) reads it.
    #[must_use]
    pub fn with_release_mode(mut self, mode: ReleaseMode) -> Self {
        self.release_mode = Some(mode);
        self
    }

    /// Sets the bracketing, in place of any set before;
    /// [`bracketing`](Self::bracketing) reads it.
    #[must_use]
    pub fn with_bracketing(mut self, bracketing: Bracketing) -> Self {
        self.bracketing = Some(bracketing);
        self
    }

    /// Sets whether daylight saving time was on, in place of any value set
    /// before; [`daylight_saving_time`](Self::daylight_saving_time) reads
    /// it.
    #[must_use]
    pub fn with_daylight_saving_time(mut self, on: bool) -> Self {
        self.daylight_saving_time = Some(on);
        self
    }
}

/// The white balance the camera was set to: its mode, the color
/// temperature the file records, and the photographer's fine-tuning.
///
/// Every part is `None` when the file does not record it or records it in
/// a way that cannot be read. `WhiteBalance::default()` is the empty one,
/// with every part `None`; a caller's tests set the parts they need with
/// the `with_…` methods, each named after the method that reads the part
/// back and each replacing any value set before. The `with_…` methods
/// store each value as given, so a test reads back even a color
/// temperature of 0; the readers' rules that a recorded 0 or a number
/// above 99,999 K gives `None` are not the builders'. `WhiteBalance` is
/// `Clone` and not `Copy`.
///
/// # Examples
///
/// A Nikon body's auto white balance, with the color temperature it chose
/// and no shift:
///
/// ```
/// use raw_files::{FineTune, SignedRational, WhiteBalance, WhiteBalanceMode};
///
/// let zero = SignedRational { numerator: 0, denominator: 4 };
/// let white_balance = WhiteBalance::default()
///     .with_mode(WhiteBalanceMode::Auto)
///     .with_color_temperature(5_200)
///     .with_fine_tune(FineTune::new(zero, zero));
///
/// assert_eq!(white_balance.mode().map(WhiteBalanceMode::name), Some("Auto"));
/// assert_eq!(white_balance.color_temperature(), Some(5_200));
/// assert_eq!(
///     white_balance.fine_tune().map(|fine_tune| fine_tune.amber_blue()),
///     Some(zero)
/// );
/// assert_eq!(WhiteBalance::default().mode(), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WhiteBalance {
    pub(crate) mode: Option<WhiteBalanceMode>,
    pub(crate) color_temperature: Option<u32>,
    pub(crate) fine_tune: Option<FineTune>,
}

impl WhiteBalance {
    /// The white balance setting, such as [`Auto`](WhiteBalanceMode::Auto)
    /// or [`Shade`](WhiteBalanceMode::Shade).
    ///
    /// Read from the maker's MakerNote first, which records the setting
    /// itself, and otherwise from Exif's WhiteBalance, which says auto or
    /// manual, and, for a manual one, from its LightSource, the kind of
    /// light it was set for, such as shade. Nikon's MakerNote records the
    /// setting as text, which is looked up in a table of the texts files
    /// are known to hold, such as `AUTO1` or `COOL WHT FL`; a chosen color
    /// temperature, written as its number, such as `3230K`, gives
    /// [`ColorTemperature`](WhiteBalanceMode::ColorTemperature). A text the
    /// table lacks gives way to Exif's mode, never to a guess from its letters.
    /// Sony's MakerNote records the setting as a code, which gives a mode only
    /// where a file shows what the code means, such as
    /// [`Underwater`](WhiteBalanceMode::Underwater), which Exif calls auto, or
    /// [`ColorTemperature`](WhiteBalanceMode::ColorTemperature); any other code
    /// gives way to Exif's mode. The Sony DSLR-A100 keeps its code where
    /// raw-files does not read it, so its mode is Exif's. `None` when neither
    /// gives a mode: for a light no mode stands for, such as Exif's standard
    /// light D65, and when the camera records "unknown" or "other".
    pub fn mode(&self) -> Option<WhiteBalanceMode> {
        self.mode
    }

    /// The color temperature the file records, in kelvin. A recorded 0
    /// means that none was recorded, and is `None`; so is a number above
    /// 99,999 K, which is no temperature a camera is set to.
    ///
    /// What it means differs by maker, and only a maker's MakerNote records
    /// it. On Nikon, from its ColorTemperatureAuto, it is the color
    /// temperature of the white balance the camera applied, under any
    /// [`mode`](Self::mode): the temperature auto white balance chose, the
    /// one a chosen color temperature set, or the one a kind of light or a
    /// preset stands for. The D5, the D850, the D780 and the Z bodies
    /// checked record it; the Z 9 under every setting, while the D850 and
    /// the Z fc write 0 under daylight. The Nikon bodies before them record
    /// a temperature only under a chosen color temperature
    /// ([`ColorTemperature`](WhiteBalanceMode::ColorTemperature)), as the
    /// number of their white balance text, such as `3230K`, which gives it.
    /// On Sony, from its ColorTemperature, it is the color temperature the
    /// photographer chose under a chosen color temperature
    /// ([`ColorTemperature`](WhiteBalanceMode::ColorTemperature)), and the
    /// custom white balance's own under [`Custom`](WhiteBalanceMode::Custom),
    /// as the files checked show; the [`mode`](Self::mode) beside it says
    /// which. Under every other setting every Sony file checked records 0,
    /// which gives `None`.
    pub fn color_temperature(&self) -> Option<u32> {
        self.color_temperature
    }

    /// The photographer's shift of the white balance, a recorded zero shift
    /// included, signed as [`FineTune`] says, whatever the maker.
    ///
    /// Only a maker's MakerNote records it. Nikon's records it along both
    /// axes on the D300 and every later body checked, as fractions from the
    /// D810 on, of 4 on every file checked, and in whole steps before; the
    /// single value the D1 family, the D70, the D100 and the D200 record
    /// gives `None`, since no file shows which axis it is on or which way
    /// its sign points. Sony's records it along both axes, in whole steps,
    /// on every Sony body checked but the DSLR-A100, A200, A700 and A900,
    /// the SLT-A55V, the NEX-5, the DSC-F828 and the DSC-R1, which record
    /// none and give `None`.
    pub fn fine_tune(&self) -> Option<FineTune> {
        self.fine_tune
    }

    /// Sets the white balance setting, in place of any set before;
    /// [`mode`](Self::mode) reads it.
    #[must_use]
    pub fn with_mode(mut self, mode: WhiteBalanceMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Sets the color temperature, in kelvin, in place of any set before;
    /// [`color_temperature`](Self::color_temperature) reads it.
    #[must_use]
    pub fn with_color_temperature(mut self, kelvin: u32) -> Self {
        self.color_temperature = Some(kelvin);
        self
    }

    /// Sets the fine-tuning, in place of any set before;
    /// [`fine_tune`](Self::fine_tune) reads it.
    #[must_use]
    pub fn with_fine_tune(mut self, fine_tune: FineTune) -> Self {
        self.fine_tune = Some(fine_tune);
        self
    }
}

/// The photographer's shift of the white balance along two axes, amber to
/// blue and green to magenta, in the camera's own steps.
///
/// Each shift is signed as its getter is named, whatever the maker:
/// [`amber_blue`](Self::amber_blue) is positive towards blue and negative
/// towards amber, and [`green_magenta`](Self::green_magenta) positive
/// towards magenta and negative towards green. So a viewer can show or
/// compare the shifts of any two files without knowing how each maker
/// signs them. A maker that signs an axis the other way has its value
/// turned around, its numerator negated, as frames shot with a known shift
/// show each maker's signs: Nikon stores a shift towards green as
/// positive, so its green to magenta value is negated; and Sony stores a
/// shift towards amber as positive, so its amber to blue value is negated.
///
/// The shifts are [`SignedRational`]s with the denominator the file
/// recorded, and the numerator too but for that sign, so a Nikon shift in
/// quarter steps stays, for example, `-24/4`; Sony records whole steps,
/// each read over 1, such as `-3/1`. How large a step is, no documentation
/// checked says. A recorded zero shift is a real setting, not an unknown
/// one.
///
/// A fine-tuning always has both shifts: a file that records a shift
/// along one axis only gives no fine-tuning, since no file shows which
/// axis it is on. There is no `Default`: a file that records none gives
/// `None` for [`WhiteBalance::fine_tune`].
///
/// # Equality
///
/// Two fine-tunings are equal when their shifts were stored the same way,
/// so `-24/4` is not equal to `-6/1`, although they are the same shift.
/// Compare the [`reduced`](SignedRational::reduced) forms of their shifts
/// to compare the shifts by value. Their signs follow the axes' names
/// whatever the maker, so the reduced shifts of any two files compare.
///
/// # Examples
///
/// A shift of six steps towards amber and none along the other axis, in a
/// Nikon body's quarter steps:
///
/// ```
/// use raw_files::{FineTune, SignedRational};
///
/// let shift = SignedRational { numerator: -24, denominator: 4 };
/// let zero = SignedRational { numerator: 0, denominator: 4 };
/// let fine_tune = FineTune::new(shift, zero);
/// assert_eq!(fine_tune.amber_blue(), shift);
/// assert_eq!(fine_tune.green_magenta(), zero);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FineTune {
    pub(crate) amber_blue: SignedRational,
    pub(crate) green_magenta: SignedRational,
}

impl FineTune {
    /// The fine-tuning with a shift of `amber_blue` along the amber to blue
    /// axis and one of `green_magenta` along the green to magenta axis,
    /// each signed as its getter is named and kept as given.
    #[must_use]
    pub fn new(amber_blue: SignedRational, green_magenta: SignedRational) -> Self {
        Self {
            amber_blue,
            green_magenta,
        }
    }

    /// The shift along the amber to blue axis, in the camera's steps:
    /// positive towards blue, negative towards amber.
    pub fn amber_blue(&self) -> SignedRational {
        self.amber_blue
    }

    /// The shift along the green to magenta axis, in the camera's steps:
    /// positive towards magenta, negative towards green.
    pub fn green_magenta(&self) -> SignedRational {
        self.green_magenta
    }
}

/// The flash: whether it fired, and its mode when the file records one.
///
/// Red-eye reduction and whether the flash's return light was detected,
/// which Exif records beside them, are not given.
///
/// There is no `Default`: a file that records no flash gives `None` for
/// [`CameraSettings::flash`].
///
/// # Examples
///
/// A flash that fired in fill mode, and one that did not fire, with no
/// mode recorded:
///
/// ```
/// use raw_files::{Flash, FlashMode};
///
/// let fill = Flash::new(true).with_mode(FlashMode::Fill);
/// assert!(fill.fired());
/// assert_eq!(fill.mode(), Some(FlashMode::Fill));
///
/// let off = Flash::new(false);
/// assert!(!off.fired());
/// assert_eq!(off.mode(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flash {
    pub(crate) fired: bool,
    pub(crate) mode: Option<FlashMode>,
}

impl Flash {
    /// The flash, fired or not, with no mode recorded;
    /// [`with_mode`](Self::with_mode) adds it.
    #[must_use]
    pub fn new(fired: bool) -> Self {
        Self { fired, mode: None }
    }

    /// This flash with the mode `mode`, in place of any it had.
    #[must_use]
    pub fn with_mode(self, mode: FlashMode) -> Self {
        Self {
            mode: Some(mode),
            ..self
        }
    }

    /// Whether the flash fired.
    pub fn fired(&self) -> bool {
        self.fired
    }

    /// The flash mode, or `None` when the file records none, as a camera
    /// without a flash function does.
    pub fn mode(&self) -> Option<FlashMode> {
        self.mode
    }
}

/// AF fine-tune: the photographer's correction of where the autofocus
/// lands, for a lens that focuses a little in front of or behind its
/// subject. Whether it was on, and, when it was, its value.
///
/// The value is in the camera's own steps, signed: the Nikon D600's menu
/// sets it from -20 to +20. It is the value in effect for the lens on the
/// camera; which saved lens it belongs to is not given. A zoom lens can
/// be tuned at both ends of its range: then [`value`](Self::value) is the
/// one at the short end, and [`tele_value`](Self::tele_value) the one at
/// the long end.
///
/// There is no `Default`: a file that records no AF fine-tune gives `None`
/// for [`CameraSettings::af_fine_tune`]. A file that records it off gives
/// one that [`is_on`](Self::is_on) says is off, with no
/// [`value`](Self::value).
///
/// # Examples
///
/// Fine-tune on, at -5; on for a zoom, at +2 and +4; and off:
///
/// ```
/// use raw_files::AfFineTune;
///
/// let on = AfFineTune::on().with_value(-5);
/// assert!(on.is_on());
/// assert_eq!(on.value(), Some(-5));
/// assert_eq!(on.tele_value(), None);
///
/// let zoom = AfFineTune::on().with_value(2).with_tele_value(4);
/// assert_eq!((zoom.value(), zoom.tele_value()), (Some(2), Some(4)));
///
/// let off = AfFineTune::off();
/// assert!(!off.is_on());
/// assert_eq!(off.value(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AfFineTune {
    pub(crate) on: bool,
    pub(crate) value: Option<i32>,
    pub(crate) tele_value: Option<i32>,
}

impl AfFineTune {
    /// AF fine-tune that was on, with no value yet;
    /// [`with_value`](Self::with_value) adds it.
    #[must_use]
    pub fn on() -> Self {
        Self {
            on: true,
            value: None,
            tele_value: None,
        }
    }

    /// AF fine-tune that was off, with no value.
    #[must_use]
    pub fn off() -> Self {
        Self {
            on: false,
            value: None,
            tele_value: None,
        }
    }

    /// This AF fine-tune with the value `value`, in place of any it had.
    /// It is stored as given, whether or not the fine-tune is on: the
    /// readers give a value only when it is.
    #[must_use]
    pub fn with_value(self, value: i32) -> Self {
        Self {
            value: Some(value),
            ..self
        }
    }

    /// Whether AF fine-tune was on.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// This AF fine-tune with the value `value` at a zoom's long end, in
    /// place of any it had. It is stored as given, whether or not the
    /// fine-tune is on: the readers give one only when the camera records
    /// a zoom tuned at both ends.
    #[must_use]
    pub fn with_tele_value(self, value: i32) -> Self {
        Self {
            tele_value: Some(value),
            ..self
        }
    }

    /// The fine-tune value, in the camera's steps, or `None` when none is
    /// recorded, as when the fine-tune is off. For a zoom tuned at both
    /// ends, it is the value at the short end.
    pub fn value(&self) -> Option<i32> {
        self.value
    }

    /// The fine-tune value at a zoom's long end, in the camera's steps, or
    /// `None` when none is recorded: when the fine-tune is off, or tunes
    /// the lens with one value.
    pub fn tele_value(&self) -> Option<i32> {
        self.tele_value
    }
}

/// How the flash was set to fire, in Exif's three modes.
///
/// `FlashMode` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Fill flash", "Flash off" and "Auto flash".
///
/// # Examples
///
/// ```
/// use raw_files::FlashMode;
///
/// assert_eq!(FlashMode::Fill.name(), "Fill flash");
/// assert_eq!(FlashMode::Off.to_string(), "Flash off");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FlashMode {
    /// The flash fires for every picture: Exif's "compulsory flash
    /// firing".
    Fill,
    /// The flash never fires: Exif's "compulsory flash suppression".
    Off,
    /// The camera decides whether the flash fires: Exif's "auto mode".
    Auto,
}

impl FlashMode {
    /// The mode's name, for an information panel: "Fill flash", "Flash
    /// off" or "Auto flash".
    pub fn name(self) -> &'static str {
        match self {
            FlashMode::Fill => "Fill flash",
            FlashMode::Off => "Flash off",
            FlashMode::Auto => "Auto flash",
        }
    }
}

impl fmt::Display for FlashMode {
    /// Writes [`name`](FlashMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the camera chose the exposure: Exif's exposure programs 1 to 8.
///
/// A code that no variant names, Exif's 0, "not defined", included, gives
/// `None`.
///
/// `ExposureProgram` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added after
/// the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Manual", "Program",
/// "Aperture priority", "Shutter priority", "Creative", "Action", "Portrait"
/// and "Landscape".
///
/// # Examples
///
/// ```
/// use raw_files::ExposureProgram;
///
/// assert_eq!(ExposureProgram::AperturePriority.name(), "Aperture priority");
/// assert_eq!(ExposureProgram::Program.to_string(), "Program");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ExposureProgram {
    /// The photographer set the exposure.
    Manual,
    /// The camera set the exposure: Exif's "normal program".
    Program,
    /// The photographer set the aperture, and the camera the shutter
    /// speed.
    AperturePriority,
    /// The photographer set the shutter speed, and the camera the
    /// aperture.
    ShutterPriority,
    /// A program biased toward depth of field.
    Creative,
    /// A program biased toward a fast shutter speed.
    Action,
    /// A program for close-ups with the background out of focus.
    Portrait,
    /// A program for landscapes, with the background in focus.
    Landscape,
}

impl ExposureProgram {
    /// The program's name, for an information panel, such as "Aperture
    /// priority"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            ExposureProgram::Manual => "Manual",
            ExposureProgram::Program => "Program",
            ExposureProgram::AperturePriority => "Aperture priority",
            ExposureProgram::ShutterPriority => "Shutter priority",
            ExposureProgram::Creative => "Creative",
            ExposureProgram::Action => "Action",
            ExposureProgram::Portrait => "Portrait",
            ExposureProgram::Landscape => "Landscape",
        }
    }
}

impl fmt::Display for ExposureProgram {
    /// Writes [`name`](ExposureProgram::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the camera metered the light: Exif's metering modes.
///
/// Exif's 0, "unknown", and a code that no variant names give `None`.
///
/// `MeteringMode` is `#[non_exhaustive]`, so match on it with a wildcard arm.
/// It orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Average", "Center-weighted average",
/// "Spot", "Multi-spot", "Multi-segment", "Partial" and "Other".
///
/// # Examples
///
/// ```
/// use raw_files::MeteringMode;
///
/// assert_eq!(MeteringMode::MultiSegment.name(), "Multi-segment");
/// assert_eq!(MeteringMode::CenterWeighted.to_string(), "Center-weighted average");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MeteringMode {
    /// The average of the whole frame.
    Average,
    /// The whole frame, weighted toward its center: Exif's center-weighted
    /// average.
    CenterWeighted,
    /// A small spot of the frame.
    Spot,
    /// Several spots of the frame.
    MultiSpot,
    /// The frame metered in segments: what Exif calls "pattern", Nikon
    /// "matrix" and Sony "multi" metering.
    MultiSegment,
    /// A part of the frame, larger than a spot.
    Partial,
    /// A mode Exif does not name: its "other" (255).
    Other,
}

impl MeteringMode {
    /// The mode's name, for an information panel, such as "Multi-segment";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            MeteringMode::Average => "Average",
            MeteringMode::CenterWeighted => "Center-weighted average",
            MeteringMode::Spot => "Spot",
            MeteringMode::MultiSpot => "Multi-spot",
            MeteringMode::MultiSegment => "Multi-segment",
            MeteringMode::Partial => "Partial",
            MeteringMode::Other => "Other",
        }
    }
}

impl fmt::Display for MeteringMode {
    /// Writes [`name`](MeteringMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The white balance setting, in words that fit any maker.
///
/// Each maker's finer kinds fold into one variant: every auto setting into
/// [`Auto`](Self::Auto), every fluorescent lamp into
/// [`Fluorescent`](Self::Fluorescent).
///
/// `WhiteBalanceMode` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added after
/// the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Auto", "Daylight",
/// "Cloudy", "Shade", "Tungsten", "Fluorescent", "Flash", "Underwater",
/// "Custom" and "Color temperature".
///
/// # Examples
///
/// ```
/// use raw_files::WhiteBalanceMode;
///
/// assert_eq!(WhiteBalanceMode::Shade.name(), "Shade");
/// assert_eq!(WhiteBalanceMode::ColorTemperature.to_string(), "Color temperature");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum WhiteBalanceMode {
    /// The camera chose the white balance, in any of its auto settings.
    Auto,
    /// Direct sunlight.
    Daylight,
    /// A cloudy sky.
    Cloudy,
    /// Shade.
    Shade,
    /// Tungsten, or incandescent, light.
    Tungsten,
    /// Fluorescent light, of any kind of lamp.
    Fluorescent,
    /// Flash.
    Flash,
    /// Under water.
    Underwater,
    /// A white balance measured from a reference, such as a gray card:
    /// Nikon's preset manual, Sony's custom.
    Custom,
    /// A color temperature the photographer chose.
    ColorTemperature,
}

impl WhiteBalanceMode {
    /// The setting's name, for an information panel, such as "Daylight";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            WhiteBalanceMode::Auto => "Auto",
            WhiteBalanceMode::Daylight => "Daylight",
            WhiteBalanceMode::Cloudy => "Cloudy",
            WhiteBalanceMode::Shade => "Shade",
            WhiteBalanceMode::Tungsten => "Tungsten",
            WhiteBalanceMode::Fluorescent => "Fluorescent",
            WhiteBalanceMode::Flash => "Flash",
            WhiteBalanceMode::Underwater => "Underwater",
            WhiteBalanceMode::Custom => "Custom",
            WhiteBalanceMode::ColorTemperature => "Color temperature",
        }
    }
}

impl fmt::Display for WhiteBalanceMode {
    /// Writes [`name`](WhiteBalanceMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The shutter that took the picture.
///
/// `Shutter` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Mechanical", "Electronic" and "Electronic
/// front curtain".
///
/// # Examples
///
/// ```
/// use raw_files::Shutter;
///
/// assert_eq!(Shutter::Mechanical.name(), "Mechanical");
/// assert_eq!(
///     Shutter::ElectronicFrontCurtain.to_string(),
///     "Electronic front curtain"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Shutter {
    /// The mechanical shutter, which starts and ends the exposure.
    Mechanical,
    /// A fully electronic shutter, which starts and ends the exposure
    /// without moving.
    Electronic,
    /// The electronic front curtain: an electronic start of the exposure
    /// and a mechanical end.
    ElectronicFrontCurtain,
}

impl Shutter {
    /// The shutter's name, for an information panel: "Mechanical",
    /// "Electronic" or "Electronic front curtain".
    pub fn name(self) -> &'static str {
        match self {
            Shutter::Mechanical => "Mechanical",
            Shutter::Electronic => "Electronic",
            Shutter::ElectronicFrontCurtain => "Electronic front curtain",
        }
    }
}

impl fmt::Display for Shutter {
    /// Writes [`name`](Shutter::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The part of the sensor the picture uses, in words that fit any maker.
///
/// Nikon records it as a crop setting. Its settings that files show give:
///
/// - FX uncropped: [`FullFrame`](Self::FullFrame);
/// - a DX crop of an FX sensor, and a DX body uncropped, its whole sensor:
///   [`ApsC`](Self::ApsC);
/// - an FX 1:1 crop: [`Square`](Self::Square);
/// - a DX 1:1 crop: [`ApsCSquare`](Self::ApsCSquare);
/// - the 16:9 crop of an FX body: [`Wide`](Self::Wide);
/// - on a DX body, the setting for a DX movie's 16:9 crop, which DX bodies
///   write on still pictures too: [`ApsCWide`](Self::ApsCWide);
/// - the 1.3x crop, and the setting for a 1.3x movie crop, which a DX body
///   writes on still pictures too: [`Crop1_3x`](Self::Crop1_3x).
///
/// No file read so far gives [`FiveByFour`](Self::FiveByFour) or
/// [`Crop2_7x`](Self::Crop2_7x): they name crops that some bodies offer, a
/// 5:4 one and a 2.7x one. Nikon's user manual for the D850, for one,
/// lists a 5:4 image area.
///
/// Nikon's "off" setting has no variant, since it does not say which
/// sensor the body has; nor have its 3:2 crop and its other movie crops. A
/// Sony body records its APS-C mode only in an enciphered block, which
/// raw-files does not read.
///
/// `ImageArea` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Full frame", "APS-C", "1:1", "1:1 (APS-C)",
/// "16:9", "16:9 (APS-C)", "5:4", "1.3x crop" and "2.7x crop".
///
/// # Examples
///
/// ```
/// use raw_files::ImageArea;
///
/// assert_eq!(ImageArea::ApsC.name(), "APS-C");
/// assert_eq!(ImageArea::ApsCSquare.to_string(), "1:1 (APS-C)");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ImageArea {
    /// The whole of a full-frame sensor.
    FullFrame,
    /// The APS-C area: a full-frame sensor's crop to that size, or the
    /// whole of an APS-C sensor.
    ApsC,
    /// A 1:1 crop of a full-frame sensor.
    Square,
    /// A 1:1 crop of the APS-C area.
    ApsCSquare,
    /// A 16:9 crop of a full-frame sensor.
    Wide,
    /// A 16:9 crop of the APS-C area.
    ApsCWide,
    /// A 5:4 crop: a picture 5 wide for 4 high.
    FiveByFour,
    /// A 1.3x crop: the center of the sensor, by a crop factor of 1.3.
    Crop1_3x,
    /// A 2.7x crop: the center of the sensor, by a crop factor of 2.7.
    Crop2_7x,
}

impl ImageArea {
    /// The area's name, for an information panel, such as "Full frame";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            ImageArea::FullFrame => "Full frame",
            ImageArea::ApsC => "APS-C",
            ImageArea::Square => "1:1",
            ImageArea::ApsCSquare => "1:1 (APS-C)",
            ImageArea::Wide => "16:9",
            ImageArea::ApsCWide => "16:9 (APS-C)",
            ImageArea::FiveByFour => "5:4",
            ImageArea::Crop1_3x => "1.3x crop",
            ImageArea::Crop2_7x => "2.7x crop",
        }
    }
}

impl fmt::Display for ImageArea {
    /// Writes [`name`](ImageArea::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The raw size the camera was set to. Its size in pixels is
/// [`Metadata::raw_size`](crate::Metadata::raw_size).
///
/// `RawImageSize` is `#[non_exhaustive]`, so match on it with a wildcard arm.
/// It orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Large", "Medium" and "Small".
///
/// # Examples
///
/// ```
/// use raw_files::RawImageSize;
///
/// assert_eq!(RawImageSize::Large.name(), "Large");
/// assert_eq!(RawImageSize::Small.to_string(), "Small");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum RawImageSize {
    /// The large raw size, the camera's largest.
    Large,
    /// The medium raw size.
    Medium,
    /// The small raw size.
    Small,
}

impl RawImageSize {
    /// The size's name, for an information panel: "Large", "Medium" or
    /// "Small".
    pub fn name(self) -> &'static str {
        match self {
            RawImageSize::Large => "Large",
            RawImageSize::Medium => "Medium",
            RawImageSize::Small => "Small",
        }
    }
}

impl fmt::Display for RawImageSize {
    /// Writes [`name`](RawImageSize::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The color space the camera was set to.
///
/// Exif's ColorSpace names sRGB alone; for any other color space it says
/// "uncalibrated". A file in Adobe RGB then names it in its
/// Interoperability IFD, with the index `R03` of a DCF option file, so
/// "uncalibrated" with that index is [`AdobeRgb`](Self::AdobeRgb), and
/// "uncalibrated" with another index, or none, is `None`. That the index
/// means Adobe RGB is from ExifTool's published documentation of the tag,
/// read as documentation, and the camera files checked agree.
///
/// `ColorSpace` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "sRGB" and "Adobe RGB".
///
/// # Examples
///
/// ```
/// use raw_files::ColorSpace;
///
/// assert_eq!(ColorSpace::Srgb.name(), "sRGB");
/// assert_eq!(ColorSpace::AdobeRgb.to_string(), "Adobe RGB");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ColorSpace {
    /// sRGB.
    Srgb,
    /// Adobe RGB.
    AdobeRgb,
}

impl ColorSpace {
    /// The color space's name, for an information panel: "sRGB" or "Adobe
    /// RGB".
    pub fn name(self) -> &'static str {
        match self {
            ColorSpace::Srgb => "sRGB",
            ColorSpace::AdobeRgb => "Adobe RGB",
        }
    }
}

impl fmt::Display for ColorSpace {
    /// Writes [`name`](ColorSpace::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the camera focused, in words that fit any maker: by hand, or in
/// which autofocus mode.
///
/// Each maker's setting folds into one variant:
///
/// - Nikon's `MANUAL` and Sony's manual focus:
///   [`Manual`](Self::Manual);
/// - AF-S, Nikon's single-servo AF and Sony's: [`Single`](Self::Single);
/// - AF-C, Nikon's continuous-servo AF and Sony's, and Sony's AF-D,
///   continuous AF with a depth map assisting:
///   [`Continuous`](Self::Continuous);
/// - AF-A, Nikon's auto-servo AF and Sony's:
///   [`Automatic`](Self::Automatic);
/// - Nikon's AF-F, full-time AF: [`FullTime`](Self::FullTime);
/// - Sony's DMF, direct manual focus: [`DirectManual`](Self::DirectManual).
///
/// `FocusMode` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Manual focus", "Single AF", "Continuous
/// AF", "Automatic AF", "Full-time AF" and "Direct manual focus".
///
/// # Examples
///
/// ```
/// use raw_files::FocusMode;
///
/// assert_eq!(FocusMode::Continuous.name(), "Continuous AF");
/// assert_eq!(FocusMode::DirectManual.to_string(), "Direct manual focus");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FocusMode {
    /// The photographer focused by hand.
    Manual,
    /// The camera focused once, and held that focus while the shutter
    /// button stayed half-pressed: for a still subject.
    Single,
    /// The camera kept focusing while the shutter button stayed
    /// half-pressed: for a moving subject.
    Continuous,
    /// The camera chose between single and continuous autofocus, by
    /// whether the subject moved.
    Automatic,
    /// The camera kept focusing without the shutter button pressed.
    FullTime,
    /// The camera focused once, and the photographer could then correct
    /// focus by hand.
    DirectManual,
}

impl FocusMode {
    /// The mode's name, for an information panel, such as "Continuous
    /// AF"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            FocusMode::Manual => "Manual focus",
            FocusMode::Single => "Single AF",
            FocusMode::Continuous => "Continuous AF",
            FocusMode::Automatic => "Automatic AF",
            FocusMode::FullTime => "Full-time AF",
            FocusMode::DirectManual => "Direct manual focus",
        }
    }
}

impl fmt::Display for FocusMode {
    /// Writes [`name`](FocusMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the camera chose its autofocus points: the AF-area mode the
/// photographer set, in words that fit any maker.
///
/// A viewer labels the points of [`FocusPoints`](crate::FocusPoints) with
/// it, or shows it beside the other settings: one point the photographer
/// placed, a placed point that its neighbors stood in for while the
/// subject left it, points the camera chose itself, a subject the camera
/// followed across the frame, or an area the photographer placed in which
/// the camera chose.
///
/// Each maker's finer kinds fold into one variant, as each variant's docs
/// list. A maker's code is named where a frame shot in a known mode shows
/// what it means, or where it is in a list that the maker's or another
/// reader's published documentation gives and that the measured files
/// agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives `None`. The notes list the codes named, with
/// where each meaning comes from: on
/// [Nikon's focus point](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#86-focus-point)
/// and on
/// [Sony's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
///
/// `FocusAreaMode` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Single point",
/// "Dynamic area", "Auto area", "Tracking", "Pinpoint" and "Zone".
///
/// # Examples
///
/// ```
/// use raw_files::FocusAreaMode;
///
/// assert_eq!(FocusAreaMode::Single.name(), "Single point");
/// assert_eq!(FocusAreaMode::Zone.to_string(), "Zone");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FocusAreaMode {
    /// One point, which the photographer placed: Nikon's single-point AF,
    /// single area, with the normal or the wide focus frame, and, in a
    /// DSLR's live view, normal area; Sony's center, flexible spot, and on
    /// its SLT bodies local and spot.
    Single,
    /// A point the photographer placed, with the points around it standing
    /// in for it when the subject briefly leaves it: Nikon's dynamic-area
    /// AF, of every size and with either focus frame; Sony's expanded
    /// flexible spot.
    Dynamic,
    /// Points the camera chose itself: Nikon's auto-area AF, with people or
    /// animal detection on some bodies, and the older dynamic area with
    /// the closest subject; Sony's wide.
    Auto,
    /// A subject the camera followed from the point where focus started:
    /// Nikon's 3D-tracking and, in a DSLR's live view, subject tracking.
    Tracking,
    /// One point, which the photographer placed, smaller than a single
    /// point, for a precise spot: Nikon's pinpoint AF.
    Pinpoint,
    /// An area the photographer placed, larger than a single point,
    /// inside which the camera chose the points: Nikon's wide-area AF,
    /// group-area AF and the older group dynamic, and, in a DSLR's live
    /// view, wide area; Sony's zone and custom AF area.
    Zone,
}

impl FocusAreaMode {
    /// The mode's name, for an information panel, such as "Auto area"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            FocusAreaMode::Single => "Single point",
            FocusAreaMode::Dynamic => "Dynamic area",
            FocusAreaMode::Auto => "Auto area",
            FocusAreaMode::Tracking => "Tracking",
            FocusAreaMode::Pinpoint => "Pinpoint",
            FocusAreaMode::Zone => "Zone",
        }
    }
}

impl fmt::Display for FocusAreaMode {
    /// Writes [`name`](FocusAreaMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the shutter was released for a picture, in words that fit any
/// maker. Each maker's finer kinds fold into one variant:
///
/// - one frame for each press of the shutter button, Nikon's single
///   frame, quiet release and mirror up, which its ShootingMode records
///   alike, and Sony's normal release: [`Single`](Self::Single);
/// - frames for as long as the button is held, Nikon's continuous low and
///   high speed and Sony's continuous release:
///   [`Continuous`](Self::Continuous);
/// - Nikon's self-timer: [`SelfTimer`](Self::SelfTimer);
/// - Nikon's infrared remote: [`Remote`](Self::Remote).
///
/// A maker's code is named where it is in a list that its published
/// documentation gives and that the measured files agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives `None`.
///
/// `ReleaseMode` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Single frame",
/// "Continuous", "Self-timer" and "Remote".
///
/// # Examples
///
/// ```
/// use raw_files::ReleaseMode;
///
/// assert_eq!(ReleaseMode::Single.name(), "Single frame");
/// assert_eq!(ReleaseMode::SelfTimer.to_string(), "Self-timer");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ReleaseMode {
    /// One frame for each press of the shutter button.
    Single,
    /// Frames for as long as the shutter button was held.
    Continuous,
    /// The camera's self-timer released the shutter, after a delay.
    SelfTimer,
    /// A remote control released the shutter.
    Remote,
}

impl ReleaseMode {
    /// The mode's name, for an information panel, such as "Self-timer";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            ReleaseMode::Single => "Single frame",
            ReleaseMode::Continuous => "Continuous",
            ReleaseMode::SelfTimer => "Self-timer",
            ReleaseMode::Remote => "Remote",
        }
    }
}

impl fmt::Display for ReleaseMode {
    /// Writes [`name`](ReleaseMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether a picture is one of a bracketed series, a few frames shot one
/// after the other with one setting varied, and which setting: in words
/// that fit any maker. Each maker's kinds fold into one variant:
///
/// - not part of a series: [`Off`](Self::Off);
/// - the exposure varied, Nikon's exposure bracketing and Exif's auto
///   bracket: [`Exposure`](Self::Exposure);
/// - the white balance varied, Nikon's and Sony's white balance
///   bracketing: [`WhiteBalance`](Self::WhiteBalance);
/// - how far shadows and highlights were brought in, Nikon's Active
///   D-Lighting bracketing and Sony's DRO bracketing:
///   [`DynamicRange`](Self::DynamicRange).
///
/// A maker's code is named where it is in a list that its published
/// documentation gives and that the measured files agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives `None`.
///
/// `Bracketing` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Off",
/// "Exposure", "White balance" and "Dynamic range".
///
/// # Examples
///
/// ```
/// use raw_files::Bracketing;
///
/// assert_eq!(Bracketing::WhiteBalance.name(), "White balance");
/// assert_eq!(Bracketing::Off.to_string(), "Off");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Bracketing {
    /// The picture is not one of a bracketed series.
    Off,
    /// One of a series shot at different exposures.
    Exposure,
    /// One of a series shot at different white balances.
    WhiteBalance,
    /// One of a series shot with shadows and highlights brought in by
    /// different amounts.
    DynamicRange,
}

impl Bracketing {
    /// The bracketing's name, for an information panel, such as "White
    /// balance"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            Bracketing::Off => "Off",
            Bracketing::Exposure => "Exposure",
            Bracketing::WhiteBalance => "White balance",
            Bracketing::DynamicRange => "Dynamic range",
        }
    }
}

impl fmt::Display for Bracketing {
    /// Writes [`name`](Bracketing::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::signed_fraction;
    use crate::testing::{assert_names, assert_ordered};

    /// A white balance with every part set: Nikon's auto, the temperature
    /// it chose, and a shift in quarter steps along both axes, each value
    /// different from the others.
    fn nikon_white_balance() -> WhiteBalance {
        WhiteBalance {
            mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(5_200),
            fine_tune: Some(FineTune {
                amber_blue: signed_fraction(-24, 4),
                green_magenta: signed_fraction(4, 4),
            }),
        }
    }

    /// Settings with every value set, each one different from what the
    /// builders set in the tests below, so that a builder writing another
    /// value, or clearing one, is caught.
    fn full_settings() -> CameraSettings {
        CameraSettings {
            white_balance: nikon_white_balance(),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some(Flash {
                fired: true,
                mode: Some(FlashMode::Fill),
            }),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: Some(true),
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::AdobeRgb),
            focus_mode: Some(FocusMode::Continuous),
            focus_area_mode: Some(FocusAreaMode::Tracking),
            af_fine_tune: Some(AfFineTune {
                on: true,
                value: Some(-5),
                tele_value: None,
            }),
            release_mode: Some(ReleaseMode::Continuous),
            bracketing: Some(Bracketing::Exposure),
            daylight_saving_time: Some(true),
        }
    }

    #[test]
    fn the_empty_record_has_no_value() {
        let settings = CameraSettings::default();
        assert_eq!(settings.white_balance(), &WhiteBalance::default());
        assert_eq!(settings.exposure_program(), None);
        assert_eq!(settings.metering_mode(), None);
        assert_eq!(settings.flash(), None);
        assert_eq!(settings.shutter(), None);
        assert_eq!(settings.electronic_front_curtain_setting(), None);
        assert_eq!(settings.image_area(), None);
        assert_eq!(settings.raw_image_size(), None);
        assert_eq!(settings.color_space(), None);
        assert_eq!(settings.focus_mode(), None);
        assert_eq!(settings.focus_area_mode(), None);
        assert_eq!(settings.af_fine_tune(), None);
        assert_eq!(settings.release_mode(), None);
        assert_eq!(settings.bracketing(), None);
        assert_eq!(settings.daylight_saving_time(), None);

        let white_balance = WhiteBalance::default();
        assert_eq!(white_balance.mode(), None);
        assert_eq!(white_balance.color_temperature(), None);
        assert_eq!(white_balance.fine_tune(), None);
    }

    #[test]
    fn getters_return_what_was_set() {
        let settings = full_settings();
        assert_eq!(settings.white_balance(), &nikon_white_balance());
        assert_eq!(
            settings.exposure_program(),
            Some(ExposureProgram::AperturePriority)
        );
        assert_eq!(settings.metering_mode(), Some(MeteringMode::MultiSegment));
        assert_eq!(
            settings.flash(),
            Some(Flash {
                fired: true,
                mode: Some(FlashMode::Fill),
            })
        );
        assert_eq!(settings.shutter(), Some(Shutter::ElectronicFrontCurtain));
        assert_eq!(settings.electronic_front_curtain_setting(), Some(true));
        assert_eq!(settings.image_area(), Some(ImageArea::FullFrame));
        assert_eq!(settings.raw_image_size(), Some(RawImageSize::Large));
        assert_eq!(settings.color_space(), Some(ColorSpace::AdobeRgb));
        assert_eq!(settings.focus_mode(), Some(FocusMode::Continuous));
        assert_eq!(settings.focus_area_mode(), Some(FocusAreaMode::Tracking));
        assert_eq!(
            settings.af_fine_tune(),
            Some(AfFineTune {
                on: true,
                value: Some(-5),
                tele_value: None,
            })
        );
        assert_eq!(settings.release_mode(), Some(ReleaseMode::Continuous));
        assert_eq!(settings.bracketing(), Some(Bracketing::Exposure));
        assert_eq!(settings.daylight_saving_time(), Some(true));

        let white_balance = nikon_white_balance();
        assert_eq!(white_balance.mode(), Some(WhiteBalanceMode::Auto));
        assert_eq!(white_balance.color_temperature(), Some(5_200));
        let fine_tune = white_balance.fine_tune().unwrap();
        assert_eq!(fine_tune.amber_blue(), signed_fraction(-24, 4));
        assert_eq!(fine_tune.green_magenta(), signed_fraction(4, 4));
    }

    #[test]
    fn each_settings_builder_sets_its_own_value_and_no_other() {
        let white_balance = WhiteBalance {
            mode: Some(WhiteBalanceMode::Shade),
            ..WhiteBalance::default()
        };
        let flash = Flash {
            fired: false,
            mode: Some(FlashMode::Off),
        };
        // From the empty record, and from a full one whose every value
        // differs from the one set. The expected value is built field by
        // field, not through the builders.
        for start in [CameraSettings::default(), full_settings()] {
            let cases = [
                (
                    start.clone().with_white_balance(white_balance.clone()),
                    CameraSettings {
                        white_balance: white_balance.clone(),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_exposure_program(ExposureProgram::Landscape),
                    CameraSettings {
                        exposure_program: Some(ExposureProgram::Landscape),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_metering_mode(MeteringMode::Spot),
                    CameraSettings {
                        metering_mode: Some(MeteringMode::Spot),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_flash(flash),
                    CameraSettings {
                        flash: Some(flash),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_shutter(Shutter::Electronic),
                    CameraSettings {
                        shutter: Some(Shutter::Electronic),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_electronic_front_curtain_setting(false),
                    CameraSettings {
                        electronic_front_curtain_setting: Some(false),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_image_area(ImageArea::ApsCWide),
                    CameraSettings {
                        image_area: Some(ImageArea::ApsCWide),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_raw_image_size(RawImageSize::Small),
                    CameraSettings {
                        raw_image_size: Some(RawImageSize::Small),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_color_space(ColorSpace::Srgb),
                    CameraSettings {
                        color_space: Some(ColorSpace::Srgb),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_focus_mode(FocusMode::Manual),
                    CameraSettings {
                        focus_mode: Some(FocusMode::Manual),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_focus_area_mode(FocusAreaMode::Zone),
                    CameraSettings {
                        focus_area_mode: Some(FocusAreaMode::Zone),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_af_fine_tune(AfFineTune::off()),
                    CameraSettings {
                        af_fine_tune: Some(AfFineTune {
                            on: false,
                            value: None,
                            tele_value: None,
                        }),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_release_mode(ReleaseMode::Remote),
                    CameraSettings {
                        release_mode: Some(ReleaseMode::Remote),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_bracketing(Bracketing::Off),
                    CameraSettings {
                        bracketing: Some(Bracketing::Off),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_daylight_saving_time(false),
                    CameraSettings {
                        daylight_saving_time: Some(false),
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
    fn each_white_balance_builder_sets_its_own_value_and_no_other() {
        let fine_tune = FineTune {
            amber_blue: signed_fraction(1, 2),
            green_magenta: signed_fraction(-1, 2),
        };
        for start in [WhiteBalance::default(), nikon_white_balance()] {
            let cases = [
                (
                    start.clone().with_mode(WhiteBalanceMode::ColorTemperature),
                    WhiteBalance {
                        mode: Some(WhiteBalanceMode::ColorTemperature),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_color_temperature(3_200),
                    WhiteBalance {
                        color_temperature: Some(3_200),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_fine_tune(fine_tune),
                    WhiteBalance {
                        fine_tune: Some(fine_tune),
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
    fn every_builder_in_a_chain_equals_the_value_built_field_by_field() {
        let settings = CameraSettings::default()
            .with_white_balance(
                WhiteBalance::default()
                    .with_mode(WhiteBalanceMode::Auto)
                    .with_color_temperature(5_200)
                    .with_fine_tune(FineTune::new(
                        signed_fraction(-24, 4),
                        signed_fraction(4, 4),
                    )),
            )
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(Flash::new(true).with_mode(FlashMode::Fill))
            .with_shutter(Shutter::ElectronicFrontCurtain)
            .with_electronic_front_curtain_setting(true)
            .with_image_area(ImageArea::FullFrame)
            .with_raw_image_size(RawImageSize::Large)
            .with_color_space(ColorSpace::AdobeRgb)
            .with_focus_mode(FocusMode::Continuous)
            .with_focus_area_mode(FocusAreaMode::Tracking)
            .with_af_fine_tune(AfFineTune::on().with_value(-5))
            .with_release_mode(ReleaseMode::Continuous)
            .with_bracketing(Bracketing::Exposure)
            .with_daylight_saving_time(true);
        assert_eq!(settings, full_settings());
    }

    #[test]
    fn a_later_value_replaces_an_earlier_one() {
        let earlier = CameraSettings::default()
            .with_white_balance(WhiteBalance::default().with_color_temperature(1))
            .with_exposure_program(ExposureProgram::Manual)
            .with_metering_mode(MeteringMode::Average)
            .with_flash(Flash::new(false))
            .with_shutter(Shutter::Mechanical)
            .with_electronic_front_curtain_setting(false)
            .with_image_area(ImageArea::Crop2_7x)
            .with_raw_image_size(RawImageSize::Medium)
            .with_color_space(ColorSpace::Srgb)
            .with_focus_mode(FocusMode::Single)
            .with_focus_area_mode(FocusAreaMode::Pinpoint)
            .with_af_fine_tune(AfFineTune::off().with_value(3))
            .with_release_mode(ReleaseMode::SelfTimer)
            .with_bracketing(Bracketing::DynamicRange)
            .with_daylight_saving_time(false);
        // Every value of `earlier` differs from the full record's, so each
        // builder below has one to replace.
        let full = full_settings();
        let settings = earlier
            .with_white_balance(full.white_balance.clone())
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(full.flash.unwrap())
            .with_shutter(Shutter::ElectronicFrontCurtain)
            .with_electronic_front_curtain_setting(true)
            .with_image_area(ImageArea::FullFrame)
            .with_raw_image_size(RawImageSize::Large)
            .with_color_space(ColorSpace::AdobeRgb)
            .with_focus_mode(FocusMode::Continuous)
            .with_focus_area_mode(FocusAreaMode::Tracking)
            .with_af_fine_tune(AfFineTune::on().with_value(-5))
            .with_release_mode(ReleaseMode::Continuous)
            .with_bracketing(Bracketing::Exposure)
            .with_daylight_saving_time(true);
        assert_eq!(settings, full);

        let white_balance = WhiteBalance::default()
            .with_mode(WhiteBalanceMode::Daylight)
            .with_color_temperature(1)
            .with_fine_tune(FineTune::new(signed_fraction(1, 1), signed_fraction(1, 1)))
            .with_mode(WhiteBalanceMode::Auto)
            .with_color_temperature(5_200)
            .with_fine_tune(FineTune::new(
                signed_fraction(-24, 4),
                signed_fraction(4, 4),
            ));
        assert_eq!(white_balance, nikon_white_balance());
    }

    #[test]
    fn a_fine_tune_keeps_both_shifts_as_given() {
        // Two different shifts, so that one given for the other is caught.
        let fine_tune = FineTune::new(signed_fraction(-6, 1), signed_fraction(2, 4));
        assert_eq!(fine_tune.amber_blue(), signed_fraction(-6, 1));
        assert_eq!(fine_tune.green_magenta(), signed_fraction(2, 4));
        // As stored: -24/4 is not -6/1, and a zero shift is a value.
        assert_ne!(
            FineTune::new(signed_fraction(-24, 4), signed_fraction(2, 4)),
            fine_tune
        );
        let zero = FineTune::new(signed_fraction(0, 4), signed_fraction(0, 4));
        assert_eq!(zero.amber_blue(), signed_fraction(0, 4));
        assert_eq!(zero.green_magenta(), signed_fraction(0, 4));
    }

    #[test]
    fn a_flash_starts_without_a_mode_and_takes_one() {
        for fired in [false, true] {
            let flash = Flash::new(fired);
            assert_eq!(flash.fired(), fired);
            assert_eq!(flash.mode(), None);
            let auto = flash.with_mode(FlashMode::Auto);
            assert_eq!(auto.fired(), fired);
            assert_eq!(auto.mode(), Some(FlashMode::Auto));
            // A second mode replaces the first.
            let off = auto.with_mode(FlashMode::Off);
            assert_eq!(off.fired(), fired);
            assert_eq!(off.mode(), Some(FlashMode::Off));
        }
    }

    #[test]
    fn on_and_off_build_an_af_fine_tune_that_says_so() {
        assert!(AfFineTune::on().is_on());
        assert!(!AfFineTune::off().is_on());
    }

    #[test]
    fn an_af_fine_tune_starts_without_a_value_and_takes_one() {
        for (fine_tune, on) in [(AfFineTune::off(), false), (AfFineTune::on(), true)] {
            assert_eq!(fine_tune.is_on(), on);
            assert_eq!(fine_tune.value(), None);
            // A negative value, then a second that replaces it, each kept
            // as given, whether on or off.
            let nearer = fine_tune.with_value(-20);
            assert_eq!((nearer.is_on(), nearer.value()), (on, Some(-20)));
            let farther = nearer.with_value(7);
            assert_eq!((farther.is_on(), farther.value()), (on, Some(7)));
            assert_eq!(farther.tele_value(), None);
            // A long end's value beside it, then a second that replaces
            // it, the value at the short end kept.
            let zoom = farther.with_tele_value(-3).with_tele_value(4);
            assert_eq!(
                (zoom.is_on(), zoom.value(), zoom.tele_value()),
                (on, Some(7), Some(4))
            );
        }
        assert_ne!(AfFineTune::on(), AfFineTune::off());
        assert_ne!(AfFineTune::on(), AfFineTune::on().with_value(0));
        assert_ne!(
            AfFineTune::on().with_value(0),
            AfFineTune::on().with_value(0).with_tele_value(0)
        );
    }

    // Every variant of each enum, in the order of its declaration. Each
    // `match` has no wildcard arm, so a variant added later does not
    // compile until it is listed here too, and the test below then fails
    // until its name is listed there.

    fn all_flash_modes() -> [FlashMode; 3] {
        let all = [FlashMode::Fill, FlashMode::Off, FlashMode::Auto];
        for mode in all {
            match mode {
                FlashMode::Fill | FlashMode::Off | FlashMode::Auto => {}
            }
        }
        all
    }

    fn all_exposure_programs() -> [ExposureProgram; 8] {
        use ExposureProgram::*;
        let all = [
            Manual,
            Program,
            AperturePriority,
            ShutterPriority,
            Creative,
            Action,
            Portrait,
            Landscape,
        ];
        for program in all {
            match program {
                Manual | Program | AperturePriority | ShutterPriority | Creative | Action
                | Portrait | Landscape => {}
            }
        }
        all
    }

    fn all_metering_modes() -> [MeteringMode; 7] {
        use MeteringMode::*;
        let all = [
            Average,
            CenterWeighted,
            Spot,
            MultiSpot,
            MultiSegment,
            Partial,
            Other,
        ];
        for mode in all {
            match mode {
                Average | CenterWeighted | Spot | MultiSpot | MultiSegment | Partial | Other => {}
            }
        }
        all
    }

    fn all_white_balance_modes() -> [WhiteBalanceMode; 10] {
        use WhiteBalanceMode::*;
        let all = [
            Auto,
            Daylight,
            Cloudy,
            Shade,
            Tungsten,
            Fluorescent,
            Flash,
            Underwater,
            Custom,
            ColorTemperature,
        ];
        for mode in all {
            match mode {
                Auto | Daylight | Cloudy | Shade | Tungsten | Fluorescent | Flash | Underwater
                | Custom | ColorTemperature => {}
            }
        }
        all
    }

    fn all_shutters() -> [Shutter; 3] {
        let all = [
            Shutter::Mechanical,
            Shutter::Electronic,
            Shutter::ElectronicFrontCurtain,
        ];
        for shutter in all {
            match shutter {
                Shutter::Mechanical | Shutter::Electronic | Shutter::ElectronicFrontCurtain => {}
            }
        }
        all
    }

    fn all_image_areas() -> [ImageArea; 9] {
        use ImageArea::*;
        let all = [
            FullFrame, ApsC, Square, ApsCSquare, Wide, ApsCWide, FiveByFour, Crop1_3x, Crop2_7x,
        ];
        for area in all {
            match area {
                FullFrame | ApsC | Square | ApsCSquare | Wide | ApsCWide | FiveByFour
                | Crop1_3x | Crop2_7x => {}
            }
        }
        all
    }

    fn all_raw_image_sizes() -> [RawImageSize; 3] {
        let all = [
            RawImageSize::Large,
            RawImageSize::Medium,
            RawImageSize::Small,
        ];
        for size in all {
            match size {
                RawImageSize::Large | RawImageSize::Medium | RawImageSize::Small => {}
            }
        }
        all
    }

    fn all_color_spaces() -> [ColorSpace; 2] {
        let all = [ColorSpace::Srgb, ColorSpace::AdobeRgb];
        for color_space in all {
            match color_space {
                ColorSpace::Srgb | ColorSpace::AdobeRgb => {}
            }
        }
        all
    }

    fn all_focus_modes() -> [FocusMode; 6] {
        use FocusMode::*;
        let all = [
            Manual,
            Single,
            Continuous,
            Automatic,
            FullTime,
            DirectManual,
        ];
        for mode in all {
            match mode {
                Manual | Single | Continuous | Automatic | FullTime | DirectManual => {}
            }
        }
        all
    }

    fn all_focus_area_modes() -> [FocusAreaMode; 6] {
        use FocusAreaMode::*;
        let all = [Single, Dynamic, Auto, Tracking, Pinpoint, Zone];
        for mode in all {
            match mode {
                Single | Dynamic | Auto | Tracking | Pinpoint | Zone => {}
            }
        }
        all
    }

    fn all_release_modes() -> [ReleaseMode; 4] {
        use ReleaseMode::*;
        let all = [Single, Continuous, SelfTimer, Remote];
        for mode in all {
            match mode {
                Single | Continuous | SelfTimer | Remote => {}
            }
        }
        all
    }

    fn all_bracketings() -> [Bracketing; 4] {
        use Bracketing::*;
        let all = [Off, Exposure, WhiteBalance, DynamicRange];
        for bracketing in all {
            match bracketing {
                Off | Exposure | WhiteBalance | DynamicRange => {}
            }
        }
        all
    }

    #[test]
    fn every_name_is_the_one_documented_and_display_writes_it() {
        // The names are stable once released: a program may show or store
        // them, so a changed one fails here.
        assert_names(
            &all_flash_modes(),
            FlashMode::name,
            &["Fill flash", "Flash off", "Auto flash"],
        );
        assert_names(
            &all_exposure_programs(),
            ExposureProgram::name,
            &[
                "Manual",
                "Program",
                "Aperture priority",
                "Shutter priority",
                "Creative",
                "Action",
                "Portrait",
                "Landscape",
            ],
        );
        assert_names(
            &all_metering_modes(),
            MeteringMode::name,
            &[
                "Average",
                "Center-weighted average",
                "Spot",
                "Multi-spot",
                "Multi-segment",
                "Partial",
                "Other",
            ],
        );
        assert_names(
            &all_white_balance_modes(),
            WhiteBalanceMode::name,
            &[
                "Auto",
                "Daylight",
                "Cloudy",
                "Shade",
                "Tungsten",
                "Fluorescent",
                "Flash",
                "Underwater",
                "Custom",
                "Color temperature",
            ],
        );
        assert_names(
            &all_shutters(),
            Shutter::name,
            &["Mechanical", "Electronic", "Electronic front curtain"],
        );
        assert_names(
            &all_image_areas(),
            ImageArea::name,
            &[
                "Full frame",
                "APS-C",
                "1:1",
                "1:1 (APS-C)",
                "16:9",
                "16:9 (APS-C)",
                "5:4",
                "1.3x crop",
                "2.7x crop",
            ],
        );
        assert_names(
            &all_raw_image_sizes(),
            RawImageSize::name,
            &["Large", "Medium", "Small"],
        );
        assert_names(
            &all_color_spaces(),
            ColorSpace::name,
            &["sRGB", "Adobe RGB"],
        );
        assert_names(
            &all_focus_modes(),
            FocusMode::name,
            &[
                "Manual focus",
                "Single AF",
                "Continuous AF",
                "Automatic AF",
                "Full-time AF",
                "Direct manual focus",
            ],
        );
        assert_names(
            &all_focus_area_modes(),
            FocusAreaMode::name,
            &[
                "Single point",
                "Dynamic area",
                "Auto area",
                "Tracking",
                "Pinpoint",
                "Zone",
            ],
        );
        assert_names(
            &all_release_modes(),
            ReleaseMode::name,
            &["Single frame", "Continuous", "Self-timer", "Remote"],
        );
        assert_names(
            &all_bracketings(),
            Bracketing::name,
            &["Off", "Exposure", "White balance", "Dynamic range"],
        );
    }

    #[test]
    fn every_enum_orders_as_its_variants_are_listed() {
        // A program may key a sorted map by these enums, and the order is
        // a promise: a variant moved, or added before the others, fails.
        assert_ordered(&all_flash_modes());
        assert_ordered(&all_exposure_programs());
        assert_ordered(&all_metering_modes());
        assert_ordered(&all_white_balance_modes());
        assert_ordered(&all_shutters());
        assert_ordered(&all_image_areas());
        assert_ordered(&all_raw_image_sizes());
        assert_ordered(&all_color_spaces());
        assert_ordered(&all_focus_modes());
        assert_ordered(&all_focus_area_modes());
        assert_ordered(&all_release_modes());
        assert_ordered(&all_bracketings());
    }

    #[test]
    fn settings_collect_in_a_set() {
        let all: std::collections::HashSet<CameraSettings> =
            [full_settings(), CameraSettings::default(), full_settings()]
                .into_iter()
                .collect();
        assert_eq!(all.len(), 2);
    }
}
