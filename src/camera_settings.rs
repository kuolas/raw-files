// SPDX-License-Identifier: MIT OR Apache-2.0

//! How the camera was set for a picture.
//!
//! Beside the exposure, a viewer's information panel shows how the camera
//! was set for the shot: the white balance, the exposure program, the
//! metering mode, the flash, the shutter, the image area, the raw image
//! size, the color space, the focus mode, the AF-area mode, AF fine-tune,
//! the stabilization, auto ISO, the exposure fine-tuning, the release
//! mode, bracketing, whether the clock was on daylight saving time, the
//! raw data's bit depth and compression, the JPEG quality and compression,
//! and how the camera processed the picture: the directions of contrast,
//! saturation and sharpness, the scene capture type, custom rendering,
//! gain control, the maker's picture style, its dynamic range
//! optimization and its high dynamic range setting, and the high ISO
//! noise reduction.
//! [`CameraSettings`] holds them all, and
//! [`Metadata::camera_settings`](crate::Metadata::camera_settings) gives
//! them for a file. [`WhiteBalance`], [`WhiteBalanceFineTune`], [`Flash`],
//! [`AfFineTune`], [`Stabilization`], [`Hdr`] and [`AutoHdr`] hold the
//! settings made of several parts; each enum, such as [`ExposureProgram`],
//! names one setting's choices in words that fit any maker, with a
//! [`name`](ExposureProgram::name) to show. The picture style keeps each
//! maker's own words instead: [`PictureStyle`] holds Nikon's
//! [`PictureControl`], Sony's [`CreativeStyle`] or its [`CreativeLook`],
//! each under its own name, [`DynamicRangeOptimization`] holds Nikon's
//! [`ActiveDLighting`] or Sony's [`DRangeOptimizer`], and
//! [`HighDynamicRange`] Nikon's [`Hdr`] or Sony's [`AutoHdr`].
//!
//! Every value is optional. A file may not record it, or may record it in
//! a way that cannot be read, such as a code no variant names; either way
//! the value is `None`, and a damaged value costs only itself.
//!
//! NEF, ARW and JPEG files give the settings that their standard Exif tags
//! record: the exposure program, the metering mode, the flash, the white
//! balance mode and the kind of fluorescent lamp it was set for, the color
//! space and exposure bracketing, the directions of contrast, saturation
//! and sharpness, the scene capture type, custom rendering and gain
//! control. Raw files give the raw data's bit depth from
//! its own standard tags, and its compression where those tags say it is
//! uncompressed. The others are recorded only in a maker's own tags, its
//! MakerNote, or in a maker's own codes:
//!
//! - Nikon's MakerNote, in NEF files and in the JPEGs of Nikon cameras,
//!   gives the white balance setting, finer than Exif's, in place of
//!   Exif's, with its color temperature, fine-tuning and preset slot; the
//!   shutter, the image area, the focus mode, the AF-area mode, AF
//!   fine-tune, the stabilization, auto ISO, the exposure fine-tuning, the
//!   release mode and bracketing, daylight saving time, and the flash's
//!   source, control, sync and compensation; the color space where the
//!   Exif tags record none, as in every Nikon raw file checked; in a raw
//!   file, the raw image size and how a compressed raw is compressed; and
//!   in a JPEG, its quality and the choice between size priority and
//!   optimal quality; the Picture Control, Nikon's picture style; Active
//!   D-Lighting; HDR; and the high ISO noise reduction.
//! - Sony's MakerNote, in ARW files and their family and in the JPEGs of
//!   Sony cameras, gives the white balance setting in place of Exif's,
//!   with its fine-tuning and the color temperature of a chosen color
//!   temperature or a custom white balance; the electronic front curtain
//!   setting, which only Sony records; the focus mode, the AF-area mode,
//!   the stabilization, without a mode, the exposure fine-tuning, the
//!   release mode and bracketing, the flash's source when it is external
//!   and its compensation, the JPEG quality, the Creative Style or
//!   Creative Look, Sony's picture style, the D-Range Optimizer, Auto HDR,
//!   and the high ISO noise reduction, which a raw file's 0 does not give.
//!   A Sony raw file says how a compressed raw is compressed in codes of
//!   Sony's own.
//!
//! Where a Nikon MakerNote records its crop as "off", or holds no crop,
//! raw-files gives the body's sensor format from a table of its own, for
//! the bodies that table holds; and where a Nikon raw file's MakerNote
//! holds no raw size, it gives large for a body the table says offers no
//! raw sizes.
//!
//! On Sony files seven settings are `None`:
//!
//! - the shutter and the image area: Sony records them only in enciphered
//!   blocks, which raw-files does not read;
//! - the raw image size: Sony's tag gives an APS-C raw the code of a
//!   medium one;
//! - AF fine-tune and auto ISO: Sony keeps its AF micro adjustment and
//!   ISO setting in enciphered or unread blocks;
//! - daylight saving time: Sony records none in a plain tag;
//! - the JPEG compression: it is a setting of Nikon's alone.
//!
//! A program that uses raw-files can build these values for its own
//! tests, without a camera file: [`CameraSettings`], [`WhiteBalance`],
//! [`PictureControl`], [`CreativeStyle`] and [`CreativeLook`] through
//! their `with_…` methods, and a
//! [`WhiteBalanceFineTune`], a [`Flash`], an [`AfFineTune`], a
//! [`Stabilization`], an [`Hdr`] and an [`AutoHdr`] through their
//! constructors.
//! [`CameraSettings`]'s docs show how.

use std::fmt;

use crate::rational::{Rational, SignedRational};

/// How the camera was set for a picture: its white balance, exposure
/// program, metering mode, flash, shutter, image area, raw image size,
/// color space, focus mode, AF-area mode, AF fine-tune, stabilization, auto
/// ISO, exposure fine-tuning, release mode, bracketing, daylight saving
/// time, raw bit depth and compression, JPEG quality and compression, the
/// directions of contrast, saturation and sharpness it processed the
/// picture in, the scene capture type, custom rendering, gain control, the
/// maker's picture style, dynamic range optimization and high dynamic
/// range setting, and the high ISO noise reduction, for a viewer's
/// information panel beside the exposure.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read, such as a code that no variant names. A
/// damaged value costs only itself.
/// [`Metadata::camera_settings`](crate::Metadata::camera_settings) gives
/// the values a file's standard Exif tags record: the exposure program,
/// the metering mode, the flash, the white balance mode and its kind of
/// fluorescent lamp, the color space and exposure bracketing, the
/// directions of contrast, saturation and sharpness, the scene capture
/// type, custom rendering and gain control, and the raw bit depth and an
/// uncompressed raw's compression that the standard tags of a raw
/// file's raw data record. The others, which only a maker's
/// MakerNote or its own codes record, it gives from Nikon's MakerNote and
/// Sony's, each those its MakerNote records in plain tags, and from each
/// maker's compression codes; the shutter, the image area, the raw image
/// size, AF fine-tune, auto ISO, daylight saving time and the JPEG
/// compression are `None` for every Sony file. Each getter says where its
/// value comes from.
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
///     println!("flash fired: {}", flash.has_fired());
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
///     .with_flash(Flash::not_fired().with_mode(FlashMode::Off));
///
/// assert_eq!(
///     settings.exposure_program().map(ExposureProgram::name),
///     Some("Aperture priority")
/// );
/// assert_eq!(settings.metering_mode(), Some(MeteringMode::MultiSegment));
/// assert_eq!(settings.white_balance().color_temperature(), Some(5_200));
/// assert_eq!(settings.flash().map(|flash| flash.has_fired()), Some(false));
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
    pub(crate) stabilization: Option<Stabilization>,
    pub(crate) auto_iso: Option<bool>,
    pub(crate) exposure_fine_tune: Option<SignedRational>,
    pub(crate) release_mode: Option<ReleaseMode>,
    pub(crate) bracketing: Option<Bracketing>,
    pub(crate) daylight_saving_time: Option<bool>,
    pub(crate) raw_bit_depth: Option<u32>,
    pub(crate) raw_compression: Option<RawCompression>,
    pub(crate) jpeg_quality: Option<JpegQuality>,
    pub(crate) jpeg_compression: Option<JpegCompression>,
    pub(crate) contrast: Option<Contrast>,
    pub(crate) saturation: Option<Saturation>,
    pub(crate) sharpness: Option<Sharpness>,
    pub(crate) scene_capture_type: Option<SceneCaptureType>,
    pub(crate) custom_rendered: Option<bool>,
    pub(crate) gain_control: Option<GainControl>,
    pub(crate) picture_style: Option<PictureStyle>,
    pub(crate) dynamic_range_optimization: Option<DynamicRangeOptimization>,
    pub(crate) high_dynamic_range: Option<HighDynamicRange>,
    pub(crate) high_iso_noise_reduction: Option<HighIsoNoiseReduction>,
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

    /// Whether the flash fired, and the parts of its setting the file
    /// records, such as its mode.
    ///
    /// Read from Exif's Flash, to which a maker's tags add the parts Exif
    /// does not record; each getter of [`Flash`] says where its part comes
    /// from. `None` when the file has no Exif Flash, whatever the maker's
    /// tags hold, and when the value contradicts itself, as a flash that
    /// fired on a camera without a flash function does, or sets bits Exif
    /// reserves.
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
    /// does not say which sensor the body has. For it, and for a MakerNote
    /// that holds no CropHiSpeed, the value is the body's sensor format
    /// from raw-files' table of bodies, such as
    /// [`FullFrame`](ImageArea::FullFrame) for the D600 and
    /// [`ApsC`](ImageArea::ApsC) for the D7100. The table holds the bodies
    /// whose sensor format the
    /// [notes on camera names](https://github.com/kuolas/raw-files/blob/master/docs/formats/camera-names.md)
    /// give from the maker's manual, and a caller cannot tell an area the
    /// table gave from one the file recorded.
    ///
    /// `None` when:
    ///
    /// - the crop setting is "off", or the MakerNote holds no CropHiSpeed,
    ///   on a body the table does not hold;
    /// - the crop setting is "off" beside a cropped size smaller than the
    ///   original, which no measured file holds;
    /// - the CropHiSpeed cannot be read, since its setting may have named
    ///   a crop;
    /// - the MakerNote holds no CropHiSpeed but has an entry too damaged
    ///   to read, which may be it;
    /// - the crop setting is one that gives no area: the 5:4, 3:2 and
    ///   2.7x crops, which no file checked holds, a movie's crop other
    ///   than the two [`ImageArea`] lists, or a code nothing names;
    /// - the file is a Sony body's, whatever the body: Sony records its
    ///   APS-C mode only in an enciphered block, which raw-files does not
    ///   read.
    pub fn image_area(&self) -> Option<ImageArea> {
        self.image_area
    }

    /// The raw size the camera was set to: large, medium or small. The
    /// size in pixels is [`Metadata::raw_size`](crate::Metadata::raw_size).
    ///
    /// Read from Nikon's MakerNote, its ImageSizeRAW, which the D750, the
    /// D810, the D5600 and every later body checked record in their raw
    /// files. No raw file checked is a medium one, so no file gives
    /// [`Medium`](RawImageSize::Medium) yet.
    ///
    /// A raw file whose MakerNote holds no ImageSizeRAW, and which holds
    /// raw data, is [`Large`](RawImageSize::Large) when raw-files' table of
    /// bodies says the body offers no raw sizes, as for the D600. The table
    /// holds the bodies the
    /// [notes on camera names](https://github.com/kuolas/raw-files/blob/master/docs/formats/camera-names.md)
    /// say this of, from the maker's manual. So large also means a body
    /// that offers no other size, and a caller cannot tell a size the
    /// table gave from one the file recorded.
    ///
    /// `None` when:
    ///
    /// - a raw file's MakerNote holds no ImageSizeRAW, and the table does
    ///   not hold the body or says it offers raw sizes;
    /// - the MakerNote holds no ImageSizeRAW and the file holds no raw
    ///   data, as a TIFF image a Nikon body writes;
    /// - the MakerNote holds no ImageSizeRAW but has an entry too damaged
    ///   to read, which may be it;
    /// - ImageSizeRAW holds a code other than 1, large, and 3, small. The
    ///   medium size's 2 is among them, until a raw file shows it;
    /// - ImageSizeRAW has another type or count than Nikon writes;
    /// - the file is a JPEG, whatever its MakerNote records, since it
    ///   holds no raw data: the Z 9 writes the JPEG's own size there;
    /// - the file is a Sony body's, since its tag gives an APS-C raw the
    ///   code of a medium one.
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

    /// Whether the camera's image stabilization was on, and its mode when
    /// the file records one; [`Stabilization`] says what each holds.
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's VRInfo, at its versions 0100, 0101 and 0200, each of the
    ///   length the published documentation gives: its fifth byte says on
    ///   or off, and its seventh byte the mode, read only when on, in the
    ///   version's list of codes. Nikon calls it vibration reduction (VR);
    /// - Sony's ImageStabilization, 1 on and 0 off; no plain Sony tag
    ///   records a mode.
    ///
    /// Which kind of stabilization it was, in the lens, in the body or
    /// both, is not given.
    ///
    /// `None` when:
    ///
    /// - the file records none, as Nikon's Coolpix bodies but the Coolpix
    ///   A, its DSLRs from the D1 to the D200, and Sony's SR2 and SRF files
    ///   do;
    /// - Nikon's fifth byte is neither 1 nor 2, as some files hold 0, or
    ///   Sony's value is neither 0 nor 1, as its documented 4294967295,
    ///   "n/a";
    /// - VRInfo is of a version no measured file holds, or of another
    ///   length than its version's, or the tag has another type or count
    ///   than the maker writes.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(stabilization) = file.metadata().camera_settings().stabilization() {
    ///     match (stabilization.is_on(), stabilization.mode()) {
    ///         (true, Some(mode)) => println!("stabilization: {mode}"),
    ///         (true, None) => println!("stabilization: on"),
    ///         (false, _) => println!("stabilization: off"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn stabilization(&self) -> Option<Stabilization> {
        self.stabilization
    }

    /// Whether auto ISO was on: `Some(true)` when the camera chose the ISO,
    /// `Some(false)` when the photographer set it.
    ///
    /// The ISO the camera took the picture at is
    /// [`Exposure::iso`](crate::Exposure::iso), whichever chose it.
    ///
    /// Read from Nikon's MakerNote:
    ///
    /// - bit 5 of its ShootingMode, which every Nikon DSLR checked but the
    ///   D1 bodies, the D100 and the D2H records as one SHORT, as do the
    ///   Nikon 1 and Z bodies and the Coolpix A;
    /// - on the other Coolpix bodies, which record no ShootingMode, the
    ///   text of ISOSelection, `AUTO` or `MANUAL`.
    ///
    /// `None` when:
    ///
    /// - the file is a D70's, whose bit 5 has another meaning;
    /// - ISOSelection holds any other text, or is not a text;
    /// - ShootingMode is not one SHORT and the file records no
    ///   ISOSelection, as on the D1 bodies, the D100 and the D2H, which
    ///   write ShootingMode as one BYTE, which is not read;
    /// - the file is a Sony body's: the E-mount bodies record the ISO
    ///   setting only in enciphered blocks, and the A-mount bodies in a
    ///   plain block whose layout raw-files does not read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// let metadata = file.metadata();
    /// if let Some(iso) = metadata.exposure().iso() {
    ///     match metadata.camera_settings().auto_iso() {
    ///         Some(true) => println!("ISO {iso}, chosen by the camera"),
    ///         _ => println!("ISO {iso}"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn auto_iso(&self) -> Option<bool> {
        self.auto_iso
    }

    /// The photographer's fine-tuning of the camera's metering, in EV, as
    /// the file records it: a fraction with the numerator and denominator
    /// it stored, such as `-3/6`, half a stop under. A recorded zero, such
    /// as `0/6`, is a fine-tuning of zero.
    ///
    /// It shifts every exposure the camera meters, unlike the exposure
    /// compensation of one shot,
    /// [`Exposure::exposure_bias`](crate::Exposure::exposure_bias).
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's ExposureTuning, three bytes, a signed count, a factor and
    ///   a divisor, read as the count times the factor over the divisor:
    ///   `fd 01 06` is `-3/6`. The D600's manual sets the fine-tuning in
    ///   steps of 1/6 EV, and every file holds 1 and 6 after the count;
    /// - Sony's ExposureStandardAdjustment, one SRATIONAL, `0/6` on every
    ///   file checked.
    ///
    /// `None` when:
    ///
    /// - the file records none, as the Coolpix and Nikon 1 bodies, the
    ///   oldest Nikon DSLRs and the D40 to the D80, and many Sony bodies
    ///   older than the ILCE-9, the ILCE-7M2 among them, do;
    /// - the denominator is zero;
    /// - Nikon's factor is zero, which no file holds;
    /// - the tag has another type or count than the maker writes: some
    ///   JPEGs that a program made from a NEF file store Nikon's with four
    ///   bytes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(fine_tune) = file.metadata().camera_settings().exposure_fine_tune() {
    ///     if fine_tune.numerator != 0 {
    ///         println!("metering fine-tuned by {fine_tune} EV");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn exposure_fine_tune(&self) -> Option<SignedRational> {
        self.exposure_fine_tune
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
    /// - the file records no release mode, as the Coolpix bodies but the
    ///   Coolpix A and the oldest Sony bodies do;
    /// - Nikon's ShootingMode sets two of bits 0, 3 and 7, which no file
    ///   does, or bit 11, pre-capture, a burst that begins before the
    ///   shutter button is pressed all the way, which no file does either;
    /// - Sony's code records a bracketed series, which does not say
    ///   whether the frames were shot one at a time or as a burst, or is a
    ///   code the documentation does not give;
    /// - the tag has another type or count than the maker writes, as
    ///   Nikon's D1 bodies, D100 and D2H, which write ShootingMode as one
    ///   BYTE, do.
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
    /// the Coolpix A.
    ///
    /// `None` when:
    ///
    /// - the byte holds any other value;
    /// - the file records no WorldTime, as the older Nikon bodies and the
    ///   other Coolpix bodies do;
    /// - WorldTime has another type or count than Nikon writes;
    /// - the file is a Sony body's, since Sony records no such setting in
    ///   a plain tag.
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

    /// How many bits each sample of the raw data holds, as the file records
    /// it, such as 12 or 14.
    ///
    /// Read from the BitsPerSample of the raw data, in the IFD that gives
    /// [`Metadata::raw_size`](crate::Metadata::raw_size), when it is one
    /// SHORT above zero. It is the depth the file records, which is not
    /// always the depth of the stored samples: a lossy curve may store
    /// fewer bits.
    ///
    /// `None` when:
    ///
    /// - the file holds no raw data, as a JPEG;
    /// - BitsPerSample holds several values, as Sony's medium and small
    ///   raws in YCbCr do, which record three, one a sample;
    /// - it is missing, is zero or has another type than SHORT.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(bits) = file.metadata().camera_settings().raw_bit_depth() {
    ///     println!("{bits}-bit raw");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw_bit_depth(&self) -> Option<u32> {
        self.raw_bit_depth
    }

    /// How the raw data is compressed, such as
    /// [`Lossless`](RawCompression::Lossless); [`RawCompression`] says
    /// which of a maker's codes gives each.
    ///
    /// Read from the Compression of the raw data, in the IFD that gives
    /// [`Metadata::raw_size`](crate::Metadata::raw_size), one SHORT:
    ///
    /// - 1, uncompressed by TIFF 6.0, whatever the maker;
    /// - on a Nikon file, 34713, Nikon's compressed form, refined by its
    ///   MakerNote's NEFCompression or, on the Z bodies that record none,
    ///   by the code at byte 10 of its tag 0x0051;
    /// - on a Sony file, 7, lossless; 32767, lossy where the data is one
    ///   strip of one byte a pixel, Sony's compressed raw; and 32766, lossy,
    ///   the ILCE-7M5's compressed raws.
    ///
    /// `None` when:
    ///
    /// - the file is a JPEG, which holds no raw data;
    /// - Nikon's code is 8, which names a small raw, a size and not a
    ///   compression, or one the documentation does not give, or Nikon's
    ///   34713 comes without a code to refine it;
    /// - Sony's 32767 holds one of its two other layouts, the 12-bit
    ///   packed data of the DSLR-A850 and A900 and the stream of the
    ///   DSLR-A200 to A390;
    /// - the Compression is another code, or another type or count, or the
    ///   maker's MakerNote cannot be read.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, RawCompression};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// match file.metadata().camera_settings().raw_compression() {
    ///     Some(RawCompression::Uncompressed) => println!("uncompressed raw"),
    ///     Some(compression) => println!("raw: {compression}"),
    ///     None => println!("no raw compression recorded"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw_compression(&self) -> Option<RawCompression> {
        self.raw_compression
    }

    /// The quality step of the JPEG the camera wrote, such as
    /// [`Fine`](JpegQuality::Fine), in its maker's own words; on a Sony raw
    /// file shot with a JPEG beside it, the step of that JPEG.
    ///
    /// Read from the MakerNote:
    ///
    /// - in a Nikon JPEG, its Quality text: `FINE`, `NORMAL` and `BASIC`;
    /// - in a Sony file, the first of three tags that names a step: the
    ///   second value of its tag 0x202E, which the ILCE-7RM3, the RX100M6
    ///   and later bodies write, in raw files too; in a JPEG, its tag
    ///   0x0102; and its JPEGQuality.
    ///
    /// `None` when:
    ///
    /// - the file is a Nikon raw file, or a Nikon JPEG whose Quality says
    ///   `RAW`, one that a program made from a raw file;
    /// - Sony's 0x0102, or in a raw file its 0x202E, says the camera wrote
    ///   the raw file alone, so that JPEGQuality is not read;
    /// - the file records none of these tags. The ILCE-7M2 records its
    ///   JPEG quality in no plain tag the documentation names, so its
    ///   files give `None`;
    /// - the tag holds a text or a code the documentation does not give.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.jpg")?;
    /// if let Some(quality) = file.metadata().camera_settings().jpeg_quality() {
    ///     println!("JPEG quality: {quality}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn jpeg_quality(&self) -> Option<JpegQuality> {
        self.jpeg_quality
    }

    /// How Nikon's JPEG compression was set: for a file size kept even, or
    /// for the best quality the picture allows, which the Z bodies mark
    /// with a star.
    ///
    /// Read from the MakerNote of a Nikon JPEG, its JPGCompression, one
    /// SHORT: 1 size priority and 3 optimal quality.
    ///
    /// `None` when:
    ///
    /// - the code is any other, such as 0, which the JPEGs a program made
    ///   from a raw file hold;
    /// - JPGCompression is not one SHORT;
    /// - the file is a Nikon raw file, which gives no JPEG quality either:
    ///   every one checked holds 0;
    /// - the file is of any other maker, since only Nikon records it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, JpegCompression};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.jpg")?;
    /// let settings = file.metadata().camera_settings();
    /// if settings.jpeg_compression() == Some(JpegCompression::OptimalQuality) {
    ///     println!("JPEG compressed for quality");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn jpeg_compression(&self) -> Option<JpegCompression> {
        self.jpeg_compression
    }

    /// The direction of the contrast processing the camera applied, as
    /// Exif records it: [`Normal`](Contrast::Normal),
    /// [`Soft`](Contrast::Soft) or [`Hard`](Contrast::Hard).
    ///
    /// It gives a direction only. A maker's finer value, such as the steps
    /// of a Nikon Picture Control's contrast, is part of the maker's picture
    /// style ([`picture_style`](Self::picture_style)).
    ///
    /// Read from Exif's Contrast, one SHORT: 0 normal, 1 soft and 2 hard,
    /// for every maker, in a raw file and a JPEG alike. No MakerNote fills
    /// it.
    ///
    /// `None` when:
    ///
    /// - the file records none, as Sony's DSC-F828 does;
    /// - the code is one Exif reserves, 3 or above;
    /// - the tag is not one SHORT.
    ///
    /// What files record is in the notes on
    /// [the Exif segment](https://github.com/kuolas/raw-files/blob/master/docs/formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, Contrast};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.jpg")?;
    /// match file.metadata().camera_settings().contrast() {
    ///     Some(Contrast::Normal) | None => {}
    ///     Some(contrast) => println!("contrast: {contrast}"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn contrast(&self) -> Option<Contrast> {
        self.contrast
    }

    /// The direction of the saturation processing the camera applied, as
    /// Exif records it: [`Normal`](Saturation::Normal),
    /// [`Low`](Saturation::Low) or [`High`](Saturation::High).
    ///
    /// It gives a direction only. A maker's finer value, such as the steps
    /// of a Nikon Picture Control's saturation, is part of the maker's
    /// picture style ([`picture_style`](Self::picture_style)).
    ///
    /// Read from Exif's Saturation, one SHORT: 0 normal, 1 low saturation
    /// and 2 high saturation, for every maker, in a raw file and a JPEG
    /// alike. No MakerNote fills it.
    ///
    /// `None` when:
    ///
    /// - the file records none, as Sony's DSC-F828 does;
    /// - the code is one Exif reserves, 3 or above;
    /// - the tag is not one SHORT.
    ///
    /// What files record is in the notes on
    /// [the Exif segment](https://github.com/kuolas/raw-files/blob/master/docs/formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, Saturation};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.jpg")?;
    /// if file.metadata().camera_settings().saturation() == Some(Saturation::High) {
    ///     println!("colors heightened in the camera");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn saturation(&self) -> Option<Saturation> {
        self.saturation
    }

    /// The direction of the sharpening the camera applied, as Exif
    /// records it: [`Normal`](Sharpness::Normal),
    /// [`Soft`](Sharpness::Soft) or [`Hard`](Sharpness::Hard).
    ///
    /// It gives a direction only. A maker's finer value, such as the steps
    /// of a Nikon Picture Control's sharpening, is part of the maker's
    /// picture style ([`picture_style`](Self::picture_style)).
    ///
    /// Read from Exif's Sharpness, one SHORT: 0 normal, 1 soft and 2 hard,
    /// for every maker, in a raw file and a JPEG alike. No MakerNote fills
    /// it.
    ///
    /// `None` when:
    ///
    /// - the file records none, as Sony's DSC-F828 does;
    /// - the code is one Exif reserves, 3 or above;
    /// - the tag is not one SHORT.
    ///
    /// What files record is in the notes on
    /// [the Exif segment](https://github.com/kuolas/raw-files/blob/master/docs/formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(sharpness) = file.metadata().camera_settings().sharpness() {
    ///     println!("sharpness: {sharpness}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn sharpness(&self) -> Option<Sharpness> {
        self.sharpness
    }

    /// The type of scene the camera was set for, as Exif records it, such
    /// as [`Landscape`](SceneCaptureType::Landscape).
    /// [`Standard`](SceneCaptureType::Standard) is a type the camera
    /// recorded, not the absence of one.
    ///
    /// Exif says it may also record the mode the picture was shot in. On
    /// the Nikon files measured it follows the Picture Control more than
    /// the mode: files shot in a program mode under the Landscape and
    /// Portrait controls record landscape and portrait.
    ///
    /// Read from Exif's SceneCaptureType, one SHORT: 0 standard, 1
    /// landscape, 2 portrait and 3 night scene, for every maker, in a raw
    /// file and a JPEG alike. No MakerNote fills it.
    ///
    /// `None` when:
    ///
    /// - the file records none;
    /// - the code is one Exif reserves, 4 or above, as some phones write:
    ///   4, 5, 6 and 64 are on phones' JPEGs measured;
    /// - the tag is not one SHORT.
    ///
    /// What files record is in the notes on
    /// [the Exif segment](https://github.com/kuolas/raw-files/blob/master/docs/formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, SceneCaptureType};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// match file.metadata().camera_settings().scene_capture_type() {
    ///     Some(SceneCaptureType::Standard) | None => {}
    ///     Some(scene) => println!("scene: {scene}"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn scene_capture_type(&self) -> Option<SceneCaptureType> {
        self.scene_capture_type
    }

    /// Whether the camera processed the picture in a special way, as Exif
    /// records it: `Some(true)` for its custom process, `Some(false)` for
    /// its normal process.
    ///
    /// Exif means it to mark processing aimed at the picture's output: a
    /// reader that finds it is expected not to apply processing of its
    /// own. Bodies set it for different things. Every Nikon Z 9 file
    /// measured is marked, for a reason not known, while the D3200 and
    /// D7200 mark only the files whose long exposure noise reduction ran.
    ///
    /// Read from Exif's CustomRendered, one SHORT: 0 normal process and 1
    /// custom process, for every maker, in a raw file and a JPEG alike. No
    /// MakerNote fills it.
    ///
    /// `None` when:
    ///
    /// - the file records none;
    /// - the code is one Exif reserves, 2 or above, as some phones write:
    ///   3 and 4 are on phones' JPEGs measured;
    /// - the tag is not one SHORT.
    ///
    /// What files record is in the notes on
    /// [the Exif segment](https://github.com/kuolas/raw-files/blob/master/docs/formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.jpg")?;
    /// if file.metadata().camera_settings().custom_rendered() == Some(true) {
    ///     println!("processed in a custom way");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn custom_rendered(&self) -> Option<bool> {
        self.custom_rendered
    }

    /// The degree of overall gain adjustment the camera applied, as Exif
    /// records it, such as [`LowGainUp`](GainControl::LowGainUp), or
    /// [`NoAdjustment`](GainControl::NoAdjustment), whose name is Exif's
    /// "None".
    ///
    /// Read from Exif's GainControl, one SHORT: 0 none, 1 low gain up, 2
    /// high gain up, 3 low gain down and 4 high gain down, for every maker,
    /// in a raw file and a JPEG alike. No MakerNote fills it. In practice
    /// Nikon bodies write it; nearly every Sony file measured records
    /// none.
    ///
    /// `None` when:
    ///
    /// - the file records none;
    /// - the code is one Exif reserves, 5 or above;
    /// - the tag is not one SHORT.
    ///
    /// What files record is in the notes on
    /// [the Exif segment](https://github.com/kuolas/raw-files/blob/master/docs/formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, GainControl};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// match file.metadata().camera_settings().gain_control() {
    ///     Some(GainControl::NoAdjustment) | None => {}
    ///     Some(gain) => println!("gain control: {gain}"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn gain_control(&self) -> Option<GainControl> {
        self.gain_control
    }

    /// The picture style the camera processed the picture in, under its
    /// maker's own name: Nikon's [`PictureControl`], or Sony's
    /// [`CreativeStyle`] or [`CreativeLook`], with its name and its values
    /// ([`PictureStyle`]).
    ///
    /// Read from the maker's MakerNote, in a raw file and a JPEG alike:
    ///
    /// - Nikon's from the record PictureControlData (0x0023), or from the
    ///   same record under 0x00BD, which the Coolpix bodies write, where a
    ///   file holds no 0x0023. The record is read at the versions and
    ///   lengths that measured files hold: 0100, of 58 bytes; 0200, of 68;
    ///   0300, 0301, 0302 and 0310, of 108.
    /// - Sony's from CreativeStyle's name (0xB020) and the contrast,
    ///   saturation and sharpness beside it (0x2004 to 0x2006). It is a
    ///   Creative Look when the file also records any of the look's five
    ///   further values (0x2032 to 0x2036), and a Creative Style
    ///   otherwise.
    ///
    /// `None` when:
    ///
    /// - the file records no picture style, as every file of another
    ///   maker does, and five of Sony's oldest bodies, such as the
    ///   DSC-F828;
    /// - Nikon's record is of another version, or of another length than
    ///   its version's: no measured file shows which layout it takes;
    /// - Nikon's record is not of type UNDEFINED;
    /// - none of Sony's four tags gives a value.
    ///
    /// What files record is in the NEF notes on
    /// [the Picture Control](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#811-picture-style-and-processing)
    /// and the ARW notes on
    /// [the Creative Style](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#911-picture-style-and-processing).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, PictureStyle};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(style) = file.metadata().camera_settings().picture_style() {
    ///     let name = style.name().unwrap_or("unnamed");
    ///     println!("{}: {name}", style.setting_name());
    ///     if let PictureStyle::PictureControl(control) = style {
    ///         println!("sharpening: {:?}", control.sharpening());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn picture_style(&self) -> Option<&PictureStyle> {
        self.picture_style.as_ref()
    }

    /// How the camera optimized the picture's dynamic range, keeping
    /// detail in its highlights and shadows, under its maker's own name:
    /// Nikon's [`ActiveDLighting`] or Sony's [`DRangeOptimizer`]
    /// ([`DynamicRangeOptimization`]).
    ///
    /// Read from the maker's MakerNote, in a raw file and a JPEG alike, by
    /// the lists ExifTool's documentation gives, each read whole, since
    /// files hold part of it:
    ///
    /// - Nikon's Active D-Lighting from ActiveD-Lighting (0x0022), one
    ///   SHORT: 0 off, 1 low, 3 normal, 5 high, 7 extra high, 8 to 11
    ///   extra high 1 to 4, and 65535 auto.
    /// - Sony's D-Range Optimizer from DynamicRangeOptimizer (0xB025), one
    ///   LONG: 0 off, 1 standard, 2 advanced auto, 3 auto, 8 to 12
    ///   advanced levels 1 to 5, and 16 to 23 levels 1 to 8. 0xB04F, which
    ///   repeats it in three steps, is not read.
    ///
    /// `None` when:
    ///
    /// - the file records none, as every file of another maker does, and
    ///   two of Sony's oldest bodies, the DSC-F828 and the DSC-R1;
    /// - the code is one its list lacks, such as Nikon's 2 or Sony's 4;
    /// - the tag is not Nikon's one SHORT or Sony's one LONG.
    ///
    /// What files record is in the NEF notes on
    /// [Active D-Lighting](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#811-picture-style-and-processing)
    /// and the ARW notes on
    /// [the D-Range Optimizer](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#911-picture-style-and-processing).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{ActiveDLighting, CameraFile, DRangeOptimizer, DynamicRangeOptimization};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// let settings = file.metadata().camera_settings();
    /// if let Some(optimization) = settings.dynamic_range_optimization() {
    ///     println!("{}: {}", optimization.setting_name(), optimization.name());
    ///     match optimization {
    ///         DynamicRangeOptimization::ActiveDLighting(ActiveDLighting::Auto)
    ///         | DynamicRangeOptimization::DRangeOptimizer(DRangeOptimizer::Auto) => {
    ///             println!("the camera chose the strength");
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn dynamic_range_optimization(&self) -> Option<DynamicRangeOptimization> {
        self.dynamic_range_optimization
    }

    /// Whether the camera combined exposures into one picture of a high
    /// dynamic range, and how, under its maker's own name: Nikon's
    /// [`Hdr`] or Sony's [`AutoHdr`] ([`HighDynamicRange`]).
    ///
    /// Read from the maker's MakerNote:
    ///
    /// - Nikon's HDR from HDRInfo (0x0035), a record that starts with a
    ///   four-digit version, at the versions and lengths measured files
    ///   hold: 0100 and 0101, of 8 bytes, and 0200, of 6. Its byte 4 is 0
    ///   off and 1 on. Under on, byte 5 at 0200 gives the
    ///   [`strength`](Hdr::strength), and bytes 5 and 6 at 0100 and 0101
    ///   the [`exposure_differential`](Hdr::exposure_differential) and the
    ///   [`smoothing`](Hdr::smoothing), each by the list ExifTool's
    ///   documentation gives; a code its list lacks gives no part. Byte 7
    ///   of the 8-byte versions is not read. Nikon's DSLRs and Z bodies
    ///   write the record in their JPEGs alone; Coolpix and Nikon 1 bodies
    ///   write it in their raw files too.
    /// - Sony's Auto HDR from the first of the two 16-bit numbers of HDR
    ///   (0x200A), one LONG, in a raw file and a JPEG alike: 0 off, 1 on
    ///   with the [`exposure_difference`](AutoHdr::exposure_difference)
    ///   left to the camera, and 0x10 to 0x1A on with a difference of 1.0
    ///   to 6.0 EV in steps of half an EV. The second number, whether the
    ///   camera made the HDR picture, is not read. Every measured file
    ///   holds 0.
    ///
    /// `None` when:
    ///
    /// - the file records none, as every file of another maker does, and
    ///   every NEF file of a Nikon DSLR or Z body measured;
    /// - Nikon's record is of another version, or of another length than
    ///   its version's, or starts with no version, as a COOLPIX P7100's
    ///   does;
    /// - Nikon's byte 4 holds any other code, such as 48, which the
    ///   documentation calls auto and which says neither on nor off; no
    ///   file holds it;
    /// - Sony's first number is any other code, such as 2;
    /// - the tag is not Nikon's record of type UNDEFINED or Sony's one
    ///   LONG.
    ///
    /// What files record is in the NEF notes on
    /// [HDR](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#811-picture-style-and-processing)
    /// and the ARW notes on
    /// [Auto HDR](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#911-picture-style-and-processing).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{AutoHdrExposureDifference, CameraFile, HighDynamicRange};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.jpg")?;
    /// if let Some(setting) = file.metadata().camera_settings().high_dynamic_range() {
    ///     let state = if setting.is_on() { "on" } else { "off" };
    ///     println!("{}: {state}", setting.setting_name());
    ///     match setting {
    ///         HighDynamicRange::Hdr(hdr) => {
    ///             if let Some(strength) = hdr.strength() {
    ///                 println!("strength: {strength}");
    ///             }
    ///         }
    ///         HighDynamicRange::AutoHdr(auto_hdr) => {
    ///             if let Some(AutoHdrExposureDifference::Ev(ev)) = auto_hdr.exposure_difference() {
    ///                 println!("exposure difference: {} EV", ev.to_f64());
    ///             }
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn high_dynamic_range(&self) -> Option<HighDynamicRange> {
        self.high_dynamic_range
    }

    /// How strongly the camera reduced the noise of pictures taken at
    /// high sensitivities, the setting both makers call High ISO NR, such
    /// as [`Normal`](HighIsoNoiseReduction::Normal). One list holds both
    /// makers' steps ([`HighIsoNoiseReduction`]).
    ///
    /// Read from the maker's MakerNote, by the lists ExifTool's
    /// documentation gives, each read whole, since files hold part of it:
    ///
    /// - Nikon's from HighISONoiseReduction (0x00B1), one SHORT, in a raw
    ///   file and a JPEG alike: 0 off, 1 minimal, 2 low, 3 medium low, 4
    ///   normal, 5 medium high and 6 high.
    /// - Sony's from HighISONoiseReduction (0x2009), one SHORT: 0 off, 1
    ///   low, 2 normal, 3 high and 256 auto. In a raw file its 0 gives
    ///   `None`: on every shot of an ILCE-7M2 measured with both files, the
    ///   raw file holds 0 where its JPEG holds the step, so a raw file's 0
    ///   does not say the setting. A raw file's other codes are read.
    ///
    /// `None` when:
    ///
    /// - the file records none, as every file of another maker does;
    /// - the code is one its list lacks, such as Sony's 65535, which the
    ///   documentation calls "n/a";
    /// - a Sony raw file holds 0;
    /// - the tag is not one SHORT.
    ///
    /// Sony's DSC bodies record a second tag, 0xB050, with another list;
    /// it is not read. What files record is in the NEF notes on
    /// [the high ISO noise reduction](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#811-picture-style-and-processing)
    /// and the ARW notes on
    /// [the same](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#911-picture-style-and-processing).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::{CameraFile, HighIsoNoiseReduction};
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// match file.metadata().camera_settings().high_iso_noise_reduction() {
    ///     Some(HighIsoNoiseReduction::Off) => println!("high ISO NR: off"),
    ///     Some(step) => println!("high ISO NR: {step}"),
    ///     None => {}
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn high_iso_noise_reduction(&self) -> Option<HighIsoNoiseReduction> {
        self.high_iso_noise_reduction
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

    /// Sets the stabilization, in place of any set before;
    /// [`stabilization`](Self::stabilization) reads it.
    #[must_use]
    pub fn with_stabilization(mut self, stabilization: Stabilization) -> Self {
        self.stabilization = Some(stabilization);
        self
    }

    /// Sets whether auto ISO was on, in place of any value set before;
    /// [`auto_iso`](Self::auto_iso) reads it.
    #[must_use]
    pub fn with_auto_iso(mut self, on: bool) -> Self {
        self.auto_iso = Some(on);
        self
    }

    /// Sets the exposure fine-tuning, in place of any set before;
    /// [`exposure_fine_tune`](Self::exposure_fine_tune) reads it.
    #[must_use]
    pub fn with_exposure_fine_tune(mut self, fine_tune: SignedRational) -> Self {
        self.exposure_fine_tune = Some(fine_tune);
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

    /// Sets the raw bit depth, in place of any set before;
    /// [`raw_bit_depth`](Self::raw_bit_depth) reads it.
    #[must_use]
    pub fn with_raw_bit_depth(mut self, bits: u32) -> Self {
        self.raw_bit_depth = Some(bits);
        self
    }

    /// Sets the raw compression, in place of any set before;
    /// [`raw_compression`](Self::raw_compression) reads it.
    #[must_use]
    pub fn with_raw_compression(mut self, compression: RawCompression) -> Self {
        self.raw_compression = Some(compression);
        self
    }

    /// Sets the JPEG quality, in place of any set before;
    /// [`jpeg_quality`](Self::jpeg_quality) reads it.
    #[must_use]
    pub fn with_jpeg_quality(mut self, quality: JpegQuality) -> Self {
        self.jpeg_quality = Some(quality);
        self
    }

    /// Sets the JPEG compression, in place of any set before;
    /// [`jpeg_compression`](Self::jpeg_compression) reads it.
    #[must_use]
    pub fn with_jpeg_compression(mut self, compression: JpegCompression) -> Self {
        self.jpeg_compression = Some(compression);
        self
    }

    /// Sets the contrast, in place of any set before;
    /// [`contrast`](Self::contrast) reads it.
    #[must_use]
    pub fn with_contrast(mut self, contrast: Contrast) -> Self {
        self.contrast = Some(contrast);
        self
    }

    /// Sets the saturation, in place of any set before;
    /// [`saturation`](Self::saturation) reads it.
    #[must_use]
    pub fn with_saturation(mut self, saturation: Saturation) -> Self {
        self.saturation = Some(saturation);
        self
    }

    /// Sets the sharpness, in place of any set before;
    /// [`sharpness`](Self::sharpness) reads it.
    #[must_use]
    pub fn with_sharpness(mut self, sharpness: Sharpness) -> Self {
        self.sharpness = Some(sharpness);
        self
    }

    /// Sets the scene capture type, in place of any set before;
    /// [`scene_capture_type`](Self::scene_capture_type) reads it.
    #[must_use]
    pub fn with_scene_capture_type(mut self, scene: SceneCaptureType) -> Self {
        self.scene_capture_type = Some(scene);
        self
    }

    /// Sets whether the camera processed the picture in a special way, in
    /// place of any value set before;
    /// [`custom_rendered`](Self::custom_rendered) reads it.
    #[must_use]
    pub fn with_custom_rendered(mut self, custom: bool) -> Self {
        self.custom_rendered = Some(custom);
        self
    }

    /// Sets the gain control, in place of any set before;
    /// [`gain_control`](Self::gain_control) reads it.
    #[must_use]
    pub fn with_gain_control(mut self, gain: GainControl) -> Self {
        self.gain_control = Some(gain);
        self
    }

    /// Sets the picture style, in place of any set before;
    /// [`picture_style`](Self::picture_style) reads it. Each maker's type
    /// converts into a [`PictureStyle`] with `From`.
    #[must_use]
    pub fn with_picture_style(mut self, style: PictureStyle) -> Self {
        self.picture_style = Some(style);
        self
    }

    /// Sets the dynamic range optimization, in place of any set before;
    /// [`dynamic_range_optimization`](Self::dynamic_range_optimization)
    /// reads it. Each maker's type converts into a
    /// [`DynamicRangeOptimization`] with `From`.
    #[must_use]
    pub fn with_dynamic_range_optimization(
        mut self,
        optimization: DynamicRangeOptimization,
    ) -> Self {
        self.dynamic_range_optimization = Some(optimization);
        self
    }

    /// Sets the high dynamic range setting, in place of any set before;
    /// [`high_dynamic_range`](Self::high_dynamic_range) reads it. Each
    /// maker's type converts into a [`HighDynamicRange`] with `From`.
    #[must_use]
    pub fn with_high_dynamic_range(mut self, setting: HighDynamicRange) -> Self {
        self.high_dynamic_range = Some(setting);
        self
    }

    /// Sets the high ISO noise reduction, in place of any set before;
    /// [`high_iso_noise_reduction`](Self::high_iso_noise_reduction) reads
    /// it.
    #[must_use]
    pub fn with_high_iso_noise_reduction(mut self, step: HighIsoNoiseReduction) -> Self {
        self.high_iso_noise_reduction = Some(step);
        self
    }
}

/// The white balance the camera was set to: its mode, the color
/// temperature the file records, the photographer's fine-tuning, the
/// preset slot of a custom white balance and the kind of fluorescent lamp.
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
/// use raw_files::{SignedRational, WhiteBalance, WhiteBalanceFineTune, WhiteBalanceMode};
///
/// let zero = SignedRational { numerator: 0, denominator: 4 };
/// let white_balance = WhiteBalance::default()
///     .with_mode(WhiteBalanceMode::Auto)
///     .with_color_temperature(5_200)
///     .with_fine_tune(WhiteBalanceFineTune::new(zero, zero));
///
/// assert_eq!(white_balance.mode().map(WhiteBalanceMode::name), Some("Auto"));
/// assert_eq!(white_balance.color_temperature(), Some(5_200));
/// assert_eq!(
///     white_balance.fine_tune().map(|fine_tune| fine_tune.amber_blue()),
///     Some(zero)
/// );
/// assert_eq!(WhiteBalance::default().mode(), None);
/// ```
///
/// A Nikon body's custom white balance from its first preset slot, and a
/// white balance set for a cool white fluorescent lamp:
///
/// ```
/// use raw_files::{FluorescentLamp, WhiteBalance, WhiteBalanceMode};
///
/// let preset = WhiteBalance::default()
///     .with_mode(WhiteBalanceMode::Custom)
///     .with_preset_slot(1);
/// assert_eq!(preset.preset_slot(), Some(1));
///
/// let lamp = WhiteBalance::default()
///     .with_mode(WhiteBalanceMode::Fluorescent)
///     .with_fluorescent_lamp(FluorescentLamp::CoolWhite);
/// assert_eq!(lamp.fluorescent_lamp(), Some(FluorescentLamp::CoolWhite));
/// assert_eq!(lamp.preset_slot(), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WhiteBalance {
    pub(crate) mode: Option<WhiteBalanceMode>,
    pub(crate) color_temperature: Option<u32>,
    pub(crate) fine_tune: Option<WhiteBalanceFineTune>,
    pub(crate) preset_slot: Option<u32>,
    pub(crate) fluorescent_lamp: Option<FluorescentLamp>,
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
    /// [`ColorTemperature`](WhiteBalanceMode::ColorTemperature), and a
    /// preset slot, `PRESET` and its number, such as `PRESET1`, gives
    /// [`Custom`](WhiteBalanceMode::Custom). A text the table lacks gives
    /// way to Exif's mode, never to a guess from its letters.
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
    /// included, signed as [`WhiteBalanceFineTune`] says, whatever the
    /// maker.
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
    pub fn fine_tune(&self) -> Option<WhiteBalanceFineTune> {
        self.fine_tune
    }

    /// The slot of a custom white balance, the preset the photographer
    /// measured and saved under a number, such as 1.
    ///
    /// Read from Nikon's MakerNote, the number after `PRESET` in its
    /// WhiteBalance text: `PRESET1` is the slot NX Studio calls "d-1", and
    /// the D3X's `PRESET0` is slot 0. A bare `PRESET`, which most bodies
    /// write, gives the [`Custom`](WhiteBalanceMode::Custom) mode and no
    /// slot. Only Nikon records it, so a file of any other maker gives
    /// `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.nef")?;
    /// if let Some(slot) = file.metadata().camera_settings().white_balance().preset_slot() {
    ///     println!("white balance preset d-{slot}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn preset_slot(&self) -> Option<u32> {
        self.preset_slot
    }

    /// The kind of fluorescent lamp the white balance was set for, such as
    /// [`CoolWhite`](FluorescentLamp::CoolWhite).
    ///
    /// Read from Exif's LightSource, 12 to 16, the five kinds of
    /// fluorescent lamp Exif names, whatever the maker, under Exif's manual
    /// white balance: as for the [`mode`](Self::mode), LightSource gives
    /// the light a manual white balance was set for. Sony's MakerNote also
    /// records the lamp, in its tag 0x0112, which repeats Exif's on every
    /// frame checked and is not read.
    ///
    /// `None` when:
    ///
    /// - LightSource names any other light;
    /// - Exif's WhiteBalance says auto, or the file has none;
    /// - LightSource or Exif's WhiteBalance is not one SHORT;
    /// - the maker's MakerNote names a [`mode`](Self::mode) other than
    ///   [`Fluorescent`](WhiteBalanceMode::Fluorescent).
    pub fn fluorescent_lamp(&self) -> Option<FluorescentLamp> {
        self.fluorescent_lamp
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
    pub fn with_fine_tune(mut self, fine_tune: WhiteBalanceFineTune) -> Self {
        self.fine_tune = Some(fine_tune);
        self
    }

    /// Sets the preset slot, in place of any set before;
    /// [`preset_slot`](Self::preset_slot) reads it.
    #[must_use]
    pub fn with_preset_slot(mut self, slot: u32) -> Self {
        self.preset_slot = Some(slot);
        self
    }

    /// Sets the kind of fluorescent lamp, in place of any set before;
    /// [`fluorescent_lamp`](Self::fluorescent_lamp) reads it.
    #[must_use]
    pub fn with_fluorescent_lamp(mut self, lamp: FluorescentLamp) -> Self {
        self.fluorescent_lamp = Some(lamp);
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
/// use raw_files::{SignedRational, WhiteBalanceFineTune};
///
/// let shift = SignedRational { numerator: -24, denominator: 4 };
/// let zero = SignedRational { numerator: 0, denominator: 4 };
/// let fine_tune = WhiteBalanceFineTune::new(shift, zero);
/// assert_eq!(fine_tune.amber_blue(), shift);
/// assert_eq!(fine_tune.green_magenta(), zero);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WhiteBalanceFineTune {
    pub(crate) amber_blue: SignedRational,
    pub(crate) green_magenta: SignedRational,
}

impl WhiteBalanceFineTune {
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

/// The flash: whether it fired and, when the file records them, its
/// mode, whether red-eye reduction was on, whether the flash's return
/// light was detected, which flash it was, how its output was
/// controlled, when in the exposure it fired and its compensation.
///
/// There is no `Default`: a file that records no flash gives `None` for
/// [`CameraSettings::flash`].
///
/// # Examples
///
/// A flash unit that fired in fill mode, under TTL control, in slow sync,
/// with red-eye reduction and a compensation of -1/3 EV, its return light
/// detected, and one that did not fire, with nothing else recorded:
///
/// ```
/// use raw_files::{Flash, FlashControl, FlashMode, FlashSource, FlashSync, SignedRational};
///
/// # fn main() -> Result<(), raw_files::ValueError> {
/// let minus_a_third = SignedRational::new(-2, 6)?;
/// let fill = Flash::fired()
///     .with_mode(FlashMode::Fill)
///     .with_red_eye_reduction(true)
///     .with_return_light(true)
///     .with_source(FlashSource::External)
///     .with_control(FlashControl::Ttl)
///     .with_sync(FlashSync::Slow)
///     .with_compensation(minus_a_third);
/// assert!(fill.has_fired());
/// assert_eq!(fill.mode(), Some(FlashMode::Fill));
/// assert!(fill.red_eye_reduction());
/// assert_eq!(fill.return_light(), Some(true));
/// assert_eq!(fill.source(), Some(FlashSource::External));
/// assert_eq!(fill.control(), Some(FlashControl::Ttl));
/// assert_eq!(fill.sync(), Some(FlashSync::Slow));
/// assert_eq!(fill.compensation(), Some(minus_a_third));
///
/// let off = Flash::not_fired();
/// assert!(!off.has_fired());
/// assert_eq!(off.mode(), None);
/// assert!(!off.red_eye_reduction());
/// assert_eq!(off.return_light(), None);
/// assert_eq!(off.source(), None);
/// assert_eq!(off.compensation(), None);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flash {
    pub(crate) fired: bool,
    pub(crate) mode: Option<FlashMode>,
    pub(crate) red_eye_reduction: bool,
    pub(crate) return_light: Option<bool>,
    pub(crate) source: Option<FlashSource>,
    pub(crate) control: Option<FlashControl>,
    pub(crate) sync: Option<FlashSync>,
    pub(crate) compensation: Option<SignedRational>,
}

impl Flash {
    /// A flash that fired, with nothing else yet: red-eye reduction off,
    /// and no mode, return light, source, control, sync or compensation.
    /// The `with_…` methods add them.
    #[must_use]
    pub fn fired() -> Self {
        Self {
            fired: true,
            ..Self::not_fired()
        }
    }

    /// A flash that did not fire, with nothing else yet: red-eye reduction
    /// off, and no mode, return light, source, control, sync or
    /// compensation. The `with_…` methods add them.
    #[must_use]
    pub fn not_fired() -> Self {
        Self {
            fired: false,
            mode: None,
            red_eye_reduction: false,
            return_light: None,
            source: None,
            control: None,
            sync: None,
            compensation: None,
        }
    }

    /// This flash with the mode `mode`, in place of any it had.
    #[must_use]
    pub fn with_mode(self, mode: FlashMode) -> Self {
        Self {
            mode: Some(mode),
            ..self
        }
    }

    /// This flash with red-eye reduction on or off, as `on` says, in place
    /// of what it had.
    #[must_use]
    pub fn with_red_eye_reduction(self, on: bool) -> Self {
        Self {
            red_eye_reduction: on,
            ..self
        }
    }

    /// This flash with its return light detected or not, as `detected`
    /// says, in place of what it had.
    #[must_use]
    pub fn with_return_light(self, detected: bool) -> Self {
        Self {
            return_light: Some(detected),
            ..self
        }
    }

    /// This flash with the source `source`, in place of any it had.
    #[must_use]
    pub fn with_source(self, source: FlashSource) -> Self {
        Self {
            source: Some(source),
            ..self
        }
    }

    /// This flash with the control `control`, in place of any it had.
    #[must_use]
    pub fn with_control(self, control: FlashControl) -> Self {
        Self {
            control: Some(control),
            ..self
        }
    }

    /// This flash with the sync `sync`, in place of any it had.
    #[must_use]
    pub fn with_sync(self, sync: FlashSync) -> Self {
        Self {
            sync: Some(sync),
            ..self
        }
    }

    /// This flash with the compensation `compensation`, in EV, kept as
    /// given, in place of any it had.
    #[must_use]
    pub fn with_compensation(self, compensation: SignedRational) -> Self {
        Self {
            compensation: Some(compensation),
            ..self
        }
    }

    /// Whether the flash fired, from bit 0 of Exif's Flash.
    pub fn has_fired(&self) -> bool {
        self.fired
    }

    /// The flash mode, from bits 3 and 4 of Exif's Flash: 1
    /// [`Fill`](FlashMode::Fill), 2 [`Off`](FlashMode::Off) and 3
    /// [`Auto`](FlashMode::Auto). `None` for their 0, "unknown", and where
    /// Exif's Flash says the camera has no flash function, whose mode is
    /// not read.
    pub fn mode(&self) -> Option<FlashMode> {
        self.mode
    }

    /// Whether red-eye reduction was on: a light before the flash fires,
    /// from a lamp or weak flashes, so that the subject's pupils narrow.
    ///
    /// Read from bit 6 of Exif's Flash, whatever the maker. Nikon's
    /// MakerNote turns it on too, where its FlashSetting text is
    /// `RED-EYE`, as [`sync`](Self::sync) says.
    ///
    /// `false` also when the file does not say: Exif's bit reads "no
    /// red-eye reduction mode or unknown" when clear, and it is not read
    /// where Exif's Flash says the camera has no flash function.
    pub fn red_eye_reduction(&self) -> bool {
        self.red_eye_reduction
    }

    /// Whether the camera detected the light the flash sent back, which
    /// tells whether the flash reached the subject: `Some(true)` detected,
    /// `Some(false)` not detected.
    ///
    /// Read from bits 1 and 2 of Exif's Flash, whatever the maker: 2 not
    /// detected and 3 detected.
    ///
    /// `None` when:
    ///
    /// - the flash did not fire, since it sent no light back;
    /// - the two bits hold 0, a flash that cannot detect its return light;
    /// - they hold 1, which Exif reserves.
    pub fn return_light(&self) -> Option<bool> {
        self.return_light
    }

    /// Which flash it was: the camera's own or a flash unit.
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's FlashType text, the part before its comma: `Built-in`,
    ///   or `Optional`, a flash unit;
    /// - Sony's FlashAction code: 2, an external flash fired, and 3, a
    ///   wirelessly controlled flash fired, both
    ///   [`External`](FlashSource::External).
    ///
    /// `None` when:
    ///
    /// - the file records neither tag, as Sony's DSLRs, its NEX bodies and
    ///   its SLT bodies up to the SLT-A77V do;
    /// - Nikon's text is blank, as when no flash fired, or is one the table
    ///   of texts lacks, such as the D100's `NEW_TTL`. The text is looked
    ///   up whole, so it gives the source and the
    ///   [`control`](Self::control) together or neither;
    /// - Sony's code is 0, did not fire, or 1, fired, which name no flash,
    ///   or a code the documentation does not give;
    /// - the tag has another type or count than the maker writes.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    pub fn source(&self) -> Option<FlashSource> {
        self.source
    }

    /// How the flash's output was controlled, such as by the camera's
    /// metering through the lens.
    ///
    /// Read from Nikon's MakerNote, the part after the comma in its
    /// FlashType text: `TTL` and `TTL-BL`, `AA` and `M`, as
    /// [`FlashControl`] lists them, and in commander mode `TTL&Comdr.` and
    /// `RPT&Comdr.`, a repeating flash. Commander mode itself, in which
    /// the camera fires other units, is not given.
    ///
    /// `None` when:
    ///
    /// - the file records no FlashType, as a file of any other maker;
    /// - the text is blank, as when no flash fired, or is one the table of
    ///   texts lacks, such as the D100's `NEW_TTL`. The text is looked up
    ///   whole, so it gives the [`source`](Self::source) and the control
    ///   together or neither;
    /// - the tag has another type than Nikon writes.
    ///
    /// What the files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings).
    pub fn control(&self) -> Option<FlashControl> {
        self.control
    }

    /// When in the exposure the flash was set to fire, such as at its end.
    /// It is the setting, which a camera records whether or not the flash
    /// fired.
    ///
    /// Read from Nikon's MakerNote, its FlashSetting text: `NORMAL`,
    /// `REAR`, `SLOW` and `REAR SLOW`, as [`FlashSync`] lists them, and
    /// `RED-EYE`, Nikon's red-eye reduction, which fires the flash as the
    /// exposure begins: it gives
    /// [`FrontCurtain`](FlashSync::FrontCurtain) and turns
    /// [`red_eye_reduction`](Self::red_eye_reduction) on.
    ///
    /// `None` when:
    ///
    /// - the file records no FlashSetting, as a file of any other maker;
    /// - the text is blank, or is one the table of texts lacks;
    /// - the tag has another type than Nikon writes.
    ///
    /// What the files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings).
    pub fn sync(&self) -> Option<FlashSync> {
        self.sync
    }

    /// The flash compensation the photographer set, in EV, as the file
    /// records it: a fraction such as `-2/6`, which a caller reduces for
    /// display. A recorded zero, such as `0/10`, is a compensation of
    /// zero. A camera may record it whether or not the flash fired.
    ///
    /// Read from the MakerNote:
    ///
    /// - Nikon's FlashExposureComp, four bytes, a signed count, a factor,
    ///   a divisor and a zero, read as the count times the factor over the
    ///   divisor: `fa 01 06 00` is `-6/6`. It is the camera's setting for
    ///   whichever flash fires. A flash unit's own setting, which Nikon
    ///   records beside it, is not given;
    /// - Sony's FlashExposureComp, one SRATIONAL, such as `-7/10`.
    ///
    /// `None` when:
    ///
    /// - the file records none, as the Nikon bodies with no built-in
    ///   flash, the Z 9 among them, and Sony's DSLR-A100 and its SR2 and
    ///   SRF files do;
    /// - the denominator is zero, as the Z 50 and the Z50II write Nikon's
    ///   divisor;
    /// - Nikon's factor is zero or its fourth byte is not, which no file
    ///   holds;
    /// - the tag has another type or count than the maker writes.
    ///
    /// What each maker's files record is in the notes on
    /// [Nikon's capture settings](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#810-more-capture-settings)
    /// and
    /// [Sony's](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#910-more-capture-settings).
    pub fn compensation(&self) -> Option<SignedRational> {
        self.compensation
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

    /// Whether AF fine-tune was on.
    pub fn is_on(&self) -> bool {
        self.on
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

/// The camera's image stabilization: whether it was on, and, when it was,
/// its mode, when the file records one.
///
/// It is what the camera recorded of the setting, whichever part
/// stabilized the picture: the lens, the body or both. Nikon calls it
/// vibration reduction (VR).
///
/// There is no `Default`: a file that records no stabilization gives
/// `None` for [`CameraSettings::stabilization`]. A file that records it off
/// gives one that [`is_on`](Self::is_on) says is off, with no
/// [`mode`](Self::mode).
///
/// # Examples
///
/// Stabilization on in sport mode, on with no mode recorded, and off:
///
/// ```
/// use raw_files::{Stabilization, StabilizationMode};
///
/// let sport = Stabilization::on().with_mode(StabilizationMode::Sport);
/// assert!(sport.is_on());
/// assert_eq!(sport.mode(), Some(StabilizationMode::Sport));
///
/// let on = Stabilization::on();
/// assert_eq!(on.mode(), None);
///
/// let off = Stabilization::off();
/// assert!(!off.is_on());
/// assert_eq!(off.mode(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stabilization {
    pub(crate) on: bool,
    pub(crate) mode: Option<StabilizationMode>,
}

impl Stabilization {
    /// Stabilization that was on, with no mode yet;
    /// [`with_mode`](Self::with_mode) adds it.
    #[must_use]
    pub fn on() -> Self {
        Self {
            on: true,
            mode: None,
        }
    }

    /// Stabilization that was off, with no mode.
    #[must_use]
    pub fn off() -> Self {
        Self {
            on: false,
            mode: None,
        }
    }

    /// This stabilization with the mode `mode`, in place of any it had. It
    /// is stored as given, whether or not the stabilization is on: the
    /// readers give a mode only when it is.
    #[must_use]
    pub fn with_mode(self, mode: StabilizationMode) -> Self {
        Self {
            mode: Some(mode),
            ..self
        }
    }

    /// Whether the stabilization was on.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The stabilization's mode, or `None` when none is recorded: when it
    /// is off, or the file records no mode, as every Sony file does.
    pub fn mode(&self) -> Option<StabilizationMode> {
        self.mode
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

/// Which flash lit the picture: the camera's own, or a flash unit.
///
/// `FlashSource` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Built-in" and
/// "External".
///
/// # Examples
///
/// ```
/// use raw_files::FlashSource;
///
/// assert_eq!(FlashSource::BuiltIn.name(), "Built-in");
/// assert_eq!(FlashSource::External.to_string(), "External");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FlashSource {
    /// The camera's own flash. Nikon's `Built-in`.
    BuiltIn,
    /// A flash unit, on the camera or fired by it from a distance.
    /// Nikon's `Optional`, and Sony's external and wirelessly controlled
    /// flash.
    External,
}

impl FlashSource {
    /// The source's name, for an information panel: "Built-in" or
    /// "External".
    pub fn name(self) -> &'static str {
        match self {
            FlashSource::BuiltIn => "Built-in",
            FlashSource::External => "External",
        }
    }
}

impl fmt::Display for FlashSource {
    /// Writes [`name`](FlashSource::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the flash's output was controlled.
///
/// `FlashControl` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "TTL", "Auto
/// aperture", "Manual" and "Repeating".
///
/// # Examples
///
/// ```
/// use raw_files::FlashControl;
///
/// assert_eq!(FlashControl::Ttl.name(), "TTL");
/// assert_eq!(FlashControl::AutoAperture.to_string(), "Auto aperture");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FlashControl {
    /// The camera metered the flash through the lens and set its output:
    /// TTL. Nikon's i-TTL and i-TTL balanced fill-flash (`TTL`,
    /// `TTL-BL`).
    Ttl,
    /// The flash unit metered the light itself and set its output for the
    /// aperture: Nikon's auto aperture (`AA`).
    AutoAperture,
    /// The photographer set the flash's output: Nikon's `M`.
    Manual,
    /// The flash fired several times during one exposure, as the
    /// photographer set it: Nikon's repeating flash (`RPT`).
    Repeating,
}

impl FlashControl {
    /// The control's name, for an information panel: "TTL", "Auto
    /// aperture", "Manual" or "Repeating".
    pub fn name(self) -> &'static str {
        match self {
            FlashControl::Ttl => "TTL",
            FlashControl::AutoAperture => "Auto aperture",
            FlashControl::Manual => "Manual",
            FlashControl::Repeating => "Repeating",
        }
    }
}

impl fmt::Display for FlashControl {
    /// Writes [`name`](FlashControl::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// When in the exposure the flash fired, and whether the shutter stayed
/// open longer for the background: the flash sync mode.
///
/// `FlashSync` is `#[non_exhaustive]`, so match on it with a wildcard arm.
/// It orders as its variants are listed, and a new variant is added after
/// the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Front curtain",
/// "Rear curtain", "Slow sync" and "Slow rear-curtain sync".
///
/// # Examples
///
/// ```
/// use raw_files::FlashSync;
///
/// assert_eq!(FlashSync::FrontCurtain.name(), "Front curtain");
/// assert_eq!(FlashSync::SlowRearCurtain.to_string(), "Slow rear-curtain sync");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FlashSync {
    /// The flash fires as the exposure begins: front-curtain sync, the
    /// usual setting. Nikon's `NORMAL`, and its red-eye reduction
    /// (`RED-EYE`), which also gives
    /// [`red_eye_reduction`](Flash::red_eye_reduction).
    FrontCurtain,
    /// The flash fires as the exposure ends: rear-curtain sync. Nikon's
    /// `REAR`.
    RearCurtain,
    /// The flash fires as the exposure begins, with a slow shutter speed
    /// for the background: slow sync. Nikon's `SLOW`.
    Slow,
    /// The flash fires as the exposure ends, with a slow shutter speed for
    /// the background. Nikon's `REAR SLOW`.
    SlowRearCurtain,
}

impl FlashSync {
    /// The sync's name, for an information panel: "Front curtain", "Rear
    /// curtain", "Slow sync" or "Slow rear-curtain sync".
    pub fn name(self) -> &'static str {
        match self {
            FlashSync::FrontCurtain => "Front curtain",
            FlashSync::RearCurtain => "Rear curtain",
            FlashSync::Slow => "Slow sync",
            FlashSync::SlowRearCurtain => "Slow rear-curtain sync",
        }
    }
}

impl fmt::Display for FlashSync {
    /// Writes [`name`](FlashSync::name).
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
/// Nikon's "off" setting has no variant of its own, since it does not say
/// which sensor the body has: for a body raw-files' table of bodies holds,
/// it gives the body's sensor format, [`FullFrame`](Self::FullFrame) or
/// [`ApsC`](Self::ApsC) ([`CameraSettings::image_area`]). Its 3:2 crop and
/// its other movie crops have no variant either. A Sony body records its
/// APS-C mode only in an enciphered block, which raw-files does not read.
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

/// The mode of a camera's image stabilization, in words that fit any
/// maker: Nikon's normal, active and sport vibration reduction.
///
/// A maker's code is named where it is in a list that its published
/// documentation gives and that the measured files agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives no mode.
///
/// `StabilizationMode` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It orders as its variants are listed, and a new variant is
/// added after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Normal",
/// "Active" and "Sport".
///
/// # Examples
///
/// ```
/// use raw_files::StabilizationMode;
///
/// assert_eq!(StabilizationMode::Active.name(), "Active");
/// assert_eq!(StabilizationMode::Sport.to_string(), "Sport");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum StabilizationMode {
    /// The mode for most pictures: Nikon's normal VR.
    Normal,
    /// The mode for pictures taken from a moving vehicle or another
    /// shaking stand: Nikon's active VR.
    Active,
    /// The mode for following moving subjects: Nikon's sport VR.
    Sport,
}

impl StabilizationMode {
    /// The mode's name, for an information panel, such as "Active"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            StabilizationMode::Normal => "Normal",
            StabilizationMode::Active => "Active",
            StabilizationMode::Sport => "Sport",
        }
    }
}

impl fmt::Display for StabilizationMode {
    /// Writes [`name`](StabilizationMode::name).
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

/// How the raw data is compressed, in words that fit any maker, each
/// maker's finer kinds folded in:
///
/// - stored as it is, TIFF's code 1 and Nikon's uncompressed raws, those
///   reduced to 12 bits and those packed into 12 or 14 bits:
///   [`Uncompressed`](Self::Uncompressed);
/// - compressed with no loss, Nikon's and Sony's lossless compressed raws:
///   [`Lossless`](Self::Lossless);
/// - compressed with a loss, Nikon's two lossy compressed forms, Sony's
///   compressed raw and the ILCE-7M5's compressed and compressed HQ raws:
///   [`Lossy`](Self::Lossy);
/// - Nikon's High Efficiency raws: [`HighEfficiency`](Self::HighEfficiency),
///   the smaller files of the two, and
///   [`HighEfficiencyStar`](Self::HighEfficiencyStar), marked with a star,
///   the higher quality.
///
/// A maker's code is named where it is in a list that its published
/// documentation gives and that the measured files agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives `None`.
///
/// `RawCompression` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Uncompressed",
/// "Lossless compressed", "Lossy compressed", "High efficiency" and "High
/// efficiency*".
///
/// # Examples
///
/// ```
/// use raw_files::RawCompression;
///
/// assert_eq!(RawCompression::Lossless.name(), "Lossless compressed");
/// assert_eq!(RawCompression::HighEfficiencyStar.to_string(), "High efficiency*");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum RawCompression {
    /// The raw data is stored without compression, though it may be packed.
    Uncompressed,
    /// The raw data is compressed without loss.
    Lossless,
    /// The raw data is compressed with a loss.
    Lossy,
    /// Nikon's High Efficiency compression, the smaller files of its two.
    HighEfficiency,
    /// Nikon's High Efficiency compression marked with a star, the higher
    /// quality of its two.
    HighEfficiencyStar,
}

impl RawCompression {
    /// The compression's name, for an information panel, such as
    /// "Lossless compressed"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            RawCompression::Uncompressed => "Uncompressed",
            RawCompression::Lossless => "Lossless compressed",
            RawCompression::Lossy => "Lossy compressed",
            RawCompression::HighEfficiency => "High efficiency",
            RawCompression::HighEfficiencyStar => "High efficiency*",
        }
    }
}

impl fmt::Display for RawCompression {
    /// Writes [`name`](RawCompression::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The quality step of a JPEG the camera wrote, in its maker's own words.
///
/// The makers cut the ladder from small files to fine detail into
/// different steps, so each keeps its own names, as variants of one enum:
///
/// - Nikon's steps, from its Quality text: [`Basic`](Self::Basic),
///   [`Normal`](Self::Normal) and [`Fine`](Self::Fine);
/// - Sony's steps: [`Light`](Self::Light), [`Standard`](Self::Standard),
///   [`Fine`](Self::Fine) and [`ExtraFine`](Self::ExtraFine), and on its
///   older compacts [`Economy`](Self::Economy), [`Standard`](Self::Standard),
///   [`Fine`](Self::Fine) and [`SuperFine`](Self::SuperFine).
///
/// The variants are listed so that each maker's ladder runs from its
/// smallest files to its finest detail, and `JpegQuality` orders as they
/// are listed. So two steps of one maker compare as their ladder ranks
/// them, Nikon's [`Basic`](Self::Basic) below its [`Fine`](Self::Fine).
/// Across makers the order is only the list's: Nikon's
/// [`Normal`](Self::Normal) and Sony's [`Standard`](Self::Standard) say
/// nothing of each other's size or detail.
///
/// A maker's code is named where it is in a list that its published
/// documentation gives and that the measured files agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives `None`.
///
/// `JpegQuality` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. A new variant is added where its maker's ladder puts it, between
/// the existing ones if need be, so two steps keep the order once observed
/// between them. Its [`name`](Self::name)s, which `Display` writes too,
/// are "Economy", "Light", "Basic", "Standard", "Normal", "Fine", "Super
/// fine" and "Extra fine".
///
/// # Examples
///
/// ```
/// use raw_files::JpegQuality;
///
/// assert_eq!(JpegQuality::ExtraFine.name(), "Extra fine");
/// assert_eq!(JpegQuality::Normal.to_string(), "Normal");
/// // Within one maker's ladder, the order ranks the steps.
/// assert!(JpegQuality::Basic < JpegQuality::Fine);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum JpegQuality {
    /// Sony's economy step, on its older compacts, the smallest of their
    /// files.
    Economy,
    /// Sony's light step, the smallest of its files.
    Light,
    /// Nikon's basic step, the smallest of its files.
    Basic,
    /// Sony's standard step.
    Standard,
    /// Nikon's normal step.
    Normal,
    /// Nikon's and Sony's fine step.
    Fine,
    /// Sony's super fine step, the finest of its older compacts.
    SuperFine,
    /// Sony's extra fine step, its finest.
    ExtraFine,
}

impl JpegQuality {
    /// The step's name, for an information panel, such as "Extra fine";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            JpegQuality::Economy => "Economy",
            JpegQuality::Light => "Light",
            JpegQuality::Basic => "Basic",
            JpegQuality::Standard => "Standard",
            JpegQuality::Normal => "Normal",
            JpegQuality::Fine => "Fine",
            JpegQuality::SuperFine => "Super fine",
            JpegQuality::ExtraFine => "Extra fine",
        }
    }
}

impl fmt::Display for JpegQuality {
    /// Writes [`name`](JpegQuality::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How Nikon's JPEG compression was set: for a file size kept even across
/// pictures, or for the best quality each picture allows.
///
/// The D600's menu names the two "size priority" and "optimal quality",
/// and the Z bodies mark optimal quality with a star beside the JPEG
/// quality step.
///
/// `JpegCompression` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It orders as its variants are listed, and a new variant is
/// added after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Size priority"
/// and "Optimal quality".
///
/// # Examples
///
/// ```
/// use raw_files::JpegCompression;
///
/// assert_eq!(JpegCompression::SizePriority.name(), "Size priority");
/// assert_eq!(JpegCompression::OptimalQuality.to_string(), "Optimal quality");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum JpegCompression {
    /// Compressed so that the files keep about the same size.
    SizePriority,
    /// Compressed for the best quality each picture allows, the Z bodies'
    /// starred step.
    OptimalQuality,
}

impl JpegCompression {
    /// The compression's name, for an information panel: "Size priority"
    /// or "Optimal quality".
    pub fn name(self) -> &'static str {
        match self {
            JpegCompression::SizePriority => "Size priority",
            JpegCompression::OptimalQuality => "Optimal quality",
        }
    }
}

impl fmt::Display for JpegCompression {
    /// Writes [`name`](JpegCompression::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The kind of fluorescent lamp a white balance was set for, as Exif's
/// LightSource names the five kinds, from the coolest light to the warmest
/// (Exif 3.1, 4.6.6.7.20):
///
/// - 12, daylight fluorescent, 5700 to 7100 K: [`Daylight`](Self::Daylight);
/// - 13, day white fluorescent, 4600 to 5500 K: [`DayWhite`](Self::DayWhite);
/// - 14, cool white fluorescent, 3800 to 4500 K:
///   [`CoolWhite`](Self::CoolWhite);
/// - 15, white fluorescent, 3250 to 3800 K: [`White`](Self::White);
/// - 16, warm white fluorescent, 2600 to 3250 K:
///   [`WarmWhite`](Self::WarmWhite).
///
/// `FluorescentLamp` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Daylight
/// fluorescent", "Day white fluorescent", "Cool white fluorescent", "White
/// fluorescent" and "Warm white fluorescent".
///
/// # Examples
///
/// ```
/// use raw_files::FluorescentLamp;
///
/// assert_eq!(FluorescentLamp::CoolWhite.name(), "Cool white fluorescent");
/// assert_eq!(FluorescentLamp::Daylight.to_string(), "Daylight fluorescent");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FluorescentLamp {
    /// A daylight fluorescent lamp, Exif's D, 5700 to 7100 K.
    Daylight,
    /// A day white fluorescent lamp, Exif's N, 4600 to 5500 K.
    DayWhite,
    /// A cool white fluorescent lamp, Exif's W, 3800 to 4500 K.
    CoolWhite,
    /// A white fluorescent lamp, Exif's WW, 3250 to 3800 K.
    White,
    /// A warm white fluorescent lamp, Exif's L, 2600 to 3250 K.
    WarmWhite,
}

impl FluorescentLamp {
    /// The lamp's name, for an information panel, such as "Cool white
    /// fluorescent"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            FluorescentLamp::Daylight => "Daylight fluorescent",
            FluorescentLamp::DayWhite => "Day white fluorescent",
            FluorescentLamp::CoolWhite => "Cool white fluorescent",
            FluorescentLamp::White => "White fluorescent",
            FluorescentLamp::WarmWhite => "Warm white fluorescent",
        }
    }
}

impl fmt::Display for FluorescentLamp {
    /// Writes [`name`](FluorescentLamp::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The direction of the contrast processing the camera applied, as Exif's
/// Contrast names it (Exif 3.1, 4.6.6.7.42): 0 normal, 1 soft and 2 hard.
/// A code Exif reserves gives `None`.
///
/// `Contrast` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Normal", "Soft" and "Hard".
///
/// # Examples
///
/// ```
/// use raw_files::Contrast;
///
/// assert_eq!(Contrast::Soft.name(), "Soft");
/// assert_eq!(Contrast::Hard.to_string(), "Hard");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Contrast {
    /// The camera's normal contrast.
    Normal,
    /// Lower contrast than normal.
    Soft,
    /// Higher contrast than normal.
    Hard,
}

impl Contrast {
    /// The direction's name, for an information panel: "Normal", "Soft" or
    /// "Hard".
    pub fn name(self) -> &'static str {
        match self {
            Contrast::Normal => "Normal",
            Contrast::Soft => "Soft",
            Contrast::Hard => "Hard",
        }
    }
}

impl fmt::Display for Contrast {
    /// Writes [`name`](Contrast::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The direction of the saturation processing the camera applied, as
/// Exif's Saturation names it (Exif 3.1, 4.6.6.7.43): 0 normal, 1 low
/// saturation and 2 high saturation. A code Exif reserves gives `None`.
///
/// `Saturation` is `#[non_exhaustive]`, so match on it with a wildcard arm.
/// It orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Normal", "Low" and "High".
///
/// # Examples
///
/// ```
/// use raw_files::Saturation;
///
/// assert_eq!(Saturation::Low.name(), "Low");
/// assert_eq!(Saturation::High.to_string(), "High");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Saturation {
    /// The camera's normal saturation.
    Normal,
    /// Lower saturation than normal: Exif's "low saturation".
    Low,
    /// Higher saturation than normal: Exif's "high saturation".
    High,
}

impl Saturation {
    /// The direction's name, for an information panel: "Normal", "Low" or
    /// "High".
    pub fn name(self) -> &'static str {
        match self {
            Saturation::Normal => "Normal",
            Saturation::Low => "Low",
            Saturation::High => "High",
        }
    }
}

impl fmt::Display for Saturation {
    /// Writes [`name`](Saturation::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The direction of the sharpening the camera applied, as Exif's Sharpness
/// names it (Exif 3.1, 4.6.6.7.44): 0 normal, 1 soft and 2 hard. A code
/// Exif reserves gives `None`.
///
/// `Sharpness` is `#[non_exhaustive]`, so match on it with a wildcard arm. It
/// orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "Normal", "Soft" and "Hard".
///
/// # Examples
///
/// ```
/// use raw_files::Sharpness;
///
/// assert_eq!(Sharpness::Soft.name(), "Soft");
/// assert_eq!(Sharpness::Hard.to_string(), "Hard");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Sharpness {
    /// The camera's normal sharpening.
    Normal,
    /// Less sharpening than normal.
    Soft,
    /// More sharpening than normal.
    Hard,
}

impl Sharpness {
    /// The direction's name, for an information panel: "Normal", "Soft" or
    /// "Hard".
    pub fn name(self) -> &'static str {
        match self {
            Sharpness::Normal => "Normal",
            Sharpness::Soft => "Soft",
            Sharpness::Hard => "Hard",
        }
    }
}

impl fmt::Display for Sharpness {
    /// Writes [`name`](Sharpness::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The type of scene the camera was set for, as Exif's SceneCaptureType
/// names it (Exif 3.1, 4.6.6.7.40): 0 standard, 1 landscape, 2 portrait
/// and 3 night scene. A code Exif reserves gives `None`.
///
/// `SceneCaptureType` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It orders as its variants are listed, and a new variant is
/// added after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Standard",
/// "Landscape", "Portrait" and "Night scene".
///
/// # Examples
///
/// ```
/// use raw_files::SceneCaptureType;
///
/// assert_eq!(SceneCaptureType::Landscape.name(), "Landscape");
/// assert_eq!(SceneCaptureType::NightScene.to_string(), "Night scene");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SceneCaptureType {
    /// A standard scene: a type the camera recorded, not the absence of
    /// one.
    Standard,
    /// A landscape.
    Landscape,
    /// A portrait.
    Portrait,
    /// A scene at night.
    NightScene,
}

impl SceneCaptureType {
    /// The type's name, for an information panel, such as "Night scene";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            SceneCaptureType::Standard => "Standard",
            SceneCaptureType::Landscape => "Landscape",
            SceneCaptureType::Portrait => "Portrait",
            SceneCaptureType::NightScene => "Night scene",
        }
    }
}

impl fmt::Display for SceneCaptureType {
    /// Writes [`name`](SceneCaptureType::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The degree of overall gain adjustment the camera applied, as Exif's
/// GainControl names it (Exif 3.1, 4.6.6.7.41): 0 none, 1 low gain up, 2
/// high gain up, 3 low gain down and 4 high gain down. A code Exif reserves
/// gives `None`.
///
/// Exif's "None" is the variant [`NoAdjustment`](Self::NoAdjustment), so
/// that a recorded value never reads `Some(GainControl::None)`; its
/// [`name`](Self::name) is the standard's word.
///
/// `GainControl` is `#[non_exhaustive]`, so match on it with a wildcard arm.
/// It orders as its variants are listed, and a new variant is added after the
/// existing ones, so an order once observed holds. Its [`name`](Self::name)s,
/// which `Display` writes too, are "None", "Low gain up", "High gain up",
/// "Low gain down" and "High gain down".
///
/// # Examples
///
/// ```
/// use raw_files::GainControl;
///
/// assert_eq!(GainControl::NoAdjustment.name(), "None");
/// assert_eq!(GainControl::LowGainUp.to_string(), "Low gain up");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum GainControl {
    /// No gain adjustment: Exif's "None".
    NoAdjustment,
    /// A small adjustment of the gain upward.
    LowGainUp,
    /// A large adjustment of the gain upward.
    HighGainUp,
    /// A small adjustment of the gain downward.
    LowGainDown,
    /// A large adjustment of the gain downward.
    HighGainDown,
}

impl GainControl {
    /// The adjustment's name, for an information panel, such as "Low gain
    /// up"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            GainControl::NoAdjustment => "None",
            GainControl::LowGainUp => "Low gain up",
            GainControl::HighGainUp => "High gain up",
            GainControl::LowGainDown => "Low gain down",
            GainControl::HighGainDown => "High gain down",
        }
    }
}

impl fmt::Display for GainControl {
    /// Writes [`name`](GainControl::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The picture style the camera processed the picture in, under its
/// maker's own name: the variant says which maker's setting it is, and
/// holds that setting's own type.
///
/// A picture style is a preset of the processing the camera gives its
/// JPEG, such as its sharpening, contrast and saturation, which the
/// photographer picks and may adjust. Each maker names it and cuts its
/// values in its own way, so each keeps its own type:
///
/// - [`PictureControl`](Self::PictureControl), Nikon's Picture Control;
/// - [`CreativeStyle`](Self::CreativeStyle), Sony's Creative Style;
/// - [`CreativeLook`](Self::CreativeLook), Sony's Creative Look, which its
///   later bodies have in the Creative Style's place.
///
/// [`setting_name`](Self::setting_name) gives the maker's word for the
/// setting, for a viewer's label, and [`name`](Self::name) the style's
/// own name, whichever maker's it is.
///
/// `PictureStyle` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm: another maker's setting is added as a variant. It is `Clone` and
/// not `Copy`, and has no `Default`: a style is always one maker's. Each
/// maker's type converts into it with `From`.
///
/// # Examples
///
/// Each maker's style, labeled for a viewer and read by its variant:
///
/// ```
/// use raw_files::{
///     CreativeStyle, PictureControl, PictureControlValue, PictureStyle, SignedRational,
/// };
///
/// let five = PictureControlValue::Steps(SignedRational { numerator: 20, denominator: 4 });
/// let nikon = PictureStyle::from(
///     PictureControl::default()
///         .with_name("VIVID")
///         .with_base("VIVID")
///         .with_sharpening(five),
/// );
/// let sony = PictureStyle::from(CreativeStyle::default().with_name("Vivid").with_sharpness(1));
///
/// for style in [&nikon, &sony] {
///     let name = style.name().unwrap_or("unnamed");
///     let sharpness = match style {
///         PictureStyle::PictureControl(control) => format!("{:?}", control.sharpening()),
///         PictureStyle::CreativeStyle(creative) => format!("{:?}", creative.sharpness()),
///         _ => String::new(),
///     };
///     println!("{}: {name} {sharpness}", style.setting_name());
/// }
/// assert_eq!(nikon.setting_name(), "Picture Control");
/// assert_eq!(sony.setting_name(), "Creative Style");
/// assert_eq!(sony.name(), Some("Vivid"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PictureStyle {
    /// Nikon's Picture Control.
    PictureControl(PictureControl),
    /// Sony's Creative Style.
    CreativeStyle(CreativeStyle),
    /// Sony's Creative Look.
    CreativeLook(CreativeLook),
}

impl PictureStyle {
    /// The maker's word for the setting, for a viewer's label: "Picture
    /// Control", "Creative Style" or "Creative Look".
    pub fn setting_name(&self) -> &'static str {
        match self {
            PictureStyle::PictureControl(_) => "Picture Control",
            PictureStyle::CreativeStyle(_) => "Creative Style",
            PictureStyle::CreativeLook(_) => "Creative Look",
        }
    }

    /// The style's name as the file records it, whichever maker's style
    /// it is, such as `STANDARD`, `Vivid` or a name the photographer
    /// typed: the `name` of the variant's own type,
    /// [`PictureControl::name`], [`CreativeStyle::name`] or
    /// [`CreativeLook::name`]. `None` when that type gives none.
    pub fn name(&self) -> Option<&str> {
        match self {
            PictureStyle::PictureControl(control) => control.name(),
            PictureStyle::CreativeStyle(style) => style.name(),
            PictureStyle::CreativeLook(look) => look.name(),
        }
    }
}

impl From<PictureControl> for PictureStyle {
    /// The Picture Control as a picture style, its
    /// [`PictureControl`](PictureStyle::PictureControl) variant.
    fn from(control: PictureControl) -> Self {
        PictureStyle::PictureControl(control)
    }
}

impl From<CreativeStyle> for PictureStyle {
    /// The Creative Style as a picture style, its
    /// [`CreativeStyle`](PictureStyle::CreativeStyle) variant.
    fn from(style: CreativeStyle) -> Self {
        PictureStyle::CreativeStyle(style)
    }
}

impl From<CreativeLook> for PictureStyle {
    /// The Creative Look as a picture style, its
    /// [`CreativeLook`](PictureStyle::CreativeLook) variant.
    fn from(look: CreativeLook) -> Self {
        PictureStyle::CreativeLook(look)
    }
}

/// Nikon's Picture Control: the preset of sharpening, contrast,
/// brightness, saturation and hue the camera processed the picture in,
/// with the photographer's adjustments.
///
/// Each part is in Nikon's own words, those of its cameras' manuals:
/// sharpening, quick adjust, filter effects, toning. The name and the
/// base, the preset the control was made from, are as the file records
/// them, `STANDARD` or a name its owner typed: raw-files keeps no list of
/// Picture Control names. Each value is a [`PictureControlValue`], auto
/// or a count of steps as recorded.
///
/// Read from Nikon's MakerNote, from the record PictureControlData
/// (0x0023), or from the same record under 0x00BD, which the Coolpix
/// bodies write, where a file holds no 0x0023. The record is read at the
/// versions and lengths that measured files hold: 0100, of 58 bytes, whose
/// values count whole steps; 0200, of 68, and 0300, 0301, 0302 and 0310,
/// of 108, whose values count quarter steps. A record of another version,
/// or of another length than its version's, gives no Picture Control.
///
/// Every part is `None` when the record marks it not available, as Nikon
/// does for quick adjust under `NEUTRAL` and for the filter effect
/// outside a monochrome control, or holds a byte its field cannot hold;
/// each getter lists its own cases. In the records that keep a divisor
/// beside each value, every value is also `None` where its divisor is 0.
/// A damaged part costs only itself.
/// What files record is in the
/// [NEF notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/makernote-and-metadata.md#811-picture-style-and-processing).
///
/// `PictureControl::default()` is the empty control, with every part
/// `None`; a caller's tests set the parts they need with the `with_…`
/// methods, each named after the method that reads the part back, each
/// replacing any value set before and storing it as given.
/// `PictureControl` is `Clone` and not `Copy`.
///
/// # Examples
///
/// The `VIVID` control of a Nikon Z 9 with Quick sharp at +1:
///
/// ```
/// use raw_files::{
///     PictureControl, PictureControlAdjustment, PictureControlValue, SignedRational,
/// };
///
/// let steps = |numerator, denominator| {
///     PictureControlValue::Steps(SignedRational { numerator, denominator })
/// };
/// let control = PictureControl::default()
///     .with_name("VIVID")
///     .with_base("VIVID")
///     .with_adjustment(PictureControlAdjustment::FullControl)
///     .with_quick_sharp(steps(1, 1))
///     .with_sharpening(steps(20, 4))
///     .with_mid_range_sharpening(steps(14, 4))
///     .with_clarity(steps(8, 4));
///
/// assert_eq!(control.name(), Some("VIVID"));
/// assert_eq!(control.sharpening(), Some(steps(20, 4)));
/// assert_eq!(control.quick_adjust(), None);
/// assert_eq!(control.filter_effect(), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PictureControl {
    pub(crate) name: Option<String>,
    pub(crate) base: Option<String>,
    pub(crate) adjustment: Option<PictureControlAdjustment>,
    pub(crate) quick_adjust: Option<PictureControlValue>,
    pub(crate) quick_sharp: Option<PictureControlValue>,
    pub(crate) sharpening: Option<PictureControlValue>,
    pub(crate) mid_range_sharpening: Option<PictureControlValue>,
    pub(crate) clarity: Option<PictureControlValue>,
    pub(crate) contrast: Option<PictureControlValue>,
    pub(crate) brightness: Option<PictureControlValue>,
    pub(crate) saturation: Option<PictureControlValue>,
    pub(crate) hue: Option<PictureControlValue>,
    pub(crate) filter_effect: Option<PictureControlFilterEffect>,
    pub(crate) toning: Option<PictureControlToning>,
    pub(crate) toning_saturation: Option<PictureControlValue>,
}

impl PictureControl {
    /// The control's name as the file records it, such as `STANDARD`,
    /// `VIVID` or a name the photographer typed, cleaned as every text.
    ///
    /// `None` when the name is blank, or holds a byte that is not ASCII.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The name of the preset the control was made from, as the file
    /// records it, such as `VIVID` under a name the photographer typed.
    /// Under a preset itself it equals [`name`](Self::name).
    ///
    /// `None` when the base is blank, or holds a byte that is not ASCII.
    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }

    /// How the photographer adjusted the control: not at all, by quick
    /// adjust or Quick sharp, or value by value.
    ///
    /// `None` when the record holds a code the list does not name.
    pub fn adjustment(&self) -> Option<PictureControlAdjustment> {
        self.adjustment
    }

    /// Quick adjust, which mutes or heightens the whole control, in
    /// whole steps.
    ///
    /// `None` when:
    ///
    /// - the record marks it not available, as under `NEUTRAL`, `FLAT`,
    ///   `MONOCHROME` and a name the photographer typed;
    /// - the record holds 0, which this value cannot hold;
    /// - the record is of a version that has Quick sharp in its place,
    ///   0300, 0301, 0302 or 0310.
    pub fn quick_adjust(&self) -> Option<PictureControlValue> {
        self.quick_adjust
    }

    /// Quick sharp, which raises or lowers sharpening, mid-range
    /// sharpening and clarity together, in whole steps, or
    /// [`Auto`](PictureControlValue::Auto).
    ///
    /// `None` when:
    ///
    /// - the record marks it not available, as under a name the
    ///   photographer typed;
    /// - the record is of a version that has quick adjust in its place,
    ///   0100 or 0200.
    pub fn quick_sharp(&self) -> Option<PictureControlValue> {
        self.quick_sharp
    }

    /// Sharpening, in steps, or [`Auto`](PictureControlValue::Auto).
    ///
    /// `None` when the record marks it not available, as under the
    /// `AUTO` control of the later bodies, or holds 0 at the 108-byte
    /// versions: the Z bodies offer no auto sharpening.
    pub fn sharpening(&self) -> Option<PictureControlValue> {
        self.sharpening
    }

    /// Mid-range sharpening, in steps.
    ///
    /// `None` when the record marks it not available, holds 0, or is of a
    /// version that records none, 0100 or 0200.
    pub fn mid_range_sharpening(&self) -> Option<PictureControlValue> {
        self.mid_range_sharpening
    }

    /// Clarity, in steps, or [`Auto`](PictureControlValue::Auto).
    ///
    /// `None` when the record marks it not available, holds 0 at the
    /// 108-byte versions, whose bodies offer no auto clarity, or is of a
    /// version that records none, as 0100 is.
    pub fn clarity(&self) -> Option<PictureControlValue> {
        self.clarity
    }

    /// Contrast, in steps, or [`Auto`](PictureControlValue::Auto).
    ///
    /// `None` when the record marks it not available.
    pub fn contrast(&self) -> Option<PictureControlValue> {
        self.contrast
    }

    /// Brightness, in steps.
    ///
    /// `None` when the record marks it not available, as on the Coolpix
    /// bodies, or holds 0.
    pub fn brightness(&self) -> Option<PictureControlValue> {
        self.brightness
    }

    /// Saturation, in steps, or [`Auto`](PictureControlValue::Auto).
    ///
    /// `None` when the record marks it not available, as under
    /// `MONOCHROME`.
    pub fn saturation(&self) -> Option<PictureControlValue> {
        self.saturation
    }

    /// Hue, in steps.
    ///
    /// `None` when the record marks it not available, as under
    /// `MONOCHROME` and on the Coolpix bodies, or holds 0.
    pub fn hue(&self) -> Option<PictureControlValue> {
        self.hue
    }

    /// The filter effect of a monochrome control, such as
    /// [`Yellow`](PictureControlFilterEffect::Yellow).
    ///
    /// `None` when the record marks it not available, as outside a
    /// monochrome control, or holds a code the list does not name.
    pub fn filter_effect(&self) -> Option<PictureControlFilterEffect> {
        self.filter_effect
    }

    /// The toning of a monochrome control, such as
    /// [`Sepia`](PictureControlToning::Sepia).
    ///
    /// `None` when the record marks it not available, as outside a
    /// monochrome control, or holds a code the list does not name.
    pub fn toning(&self) -> Option<PictureControlToning> {
        self.toning
    }

    /// The saturation of a monochrome control's toning, in steps.
    ///
    /// `None` when the record marks it not available, as every measured
    /// file does, or holds 0.
    pub fn toning_saturation(&self) -> Option<PictureControlValue> {
        self.toning_saturation
    }

    /// Sets the name, in place of any set before;
    /// [`name`](Self::name) reads it.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the base, in place of any set before;
    /// [`base`](Self::base) reads it.
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = Some(base.into());
        self
    }

    /// Sets how the control was adjusted, in place of any value set
    /// before; [`adjustment`](Self::adjustment) reads it.
    #[must_use]
    pub fn with_adjustment(mut self, adjustment: PictureControlAdjustment) -> Self {
        self.adjustment = Some(adjustment);
        self
    }

    /// Sets quick adjust, in place of any value set before;
    /// [`quick_adjust`](Self::quick_adjust) reads it.
    #[must_use]
    pub fn with_quick_adjust(mut self, value: PictureControlValue) -> Self {
        self.quick_adjust = Some(value);
        self
    }

    /// Sets Quick sharp, in place of any value set before;
    /// [`quick_sharp`](Self::quick_sharp) reads it.
    #[must_use]
    pub fn with_quick_sharp(mut self, value: PictureControlValue) -> Self {
        self.quick_sharp = Some(value);
        self
    }

    /// Sets sharpening, in place of any value set before;
    /// [`sharpening`](Self::sharpening) reads it.
    #[must_use]
    pub fn with_sharpening(mut self, value: PictureControlValue) -> Self {
        self.sharpening = Some(value);
        self
    }

    /// Sets mid-range sharpening, in place of any value set before;
    /// [`mid_range_sharpening`](Self::mid_range_sharpening) reads it.
    #[must_use]
    pub fn with_mid_range_sharpening(mut self, value: PictureControlValue) -> Self {
        self.mid_range_sharpening = Some(value);
        self
    }

    /// Sets clarity, in place of any value set before;
    /// [`clarity`](Self::clarity) reads it.
    #[must_use]
    pub fn with_clarity(mut self, value: PictureControlValue) -> Self {
        self.clarity = Some(value);
        self
    }

    /// Sets contrast, in place of any value set before;
    /// [`contrast`](Self::contrast) reads it.
    #[must_use]
    pub fn with_contrast(mut self, value: PictureControlValue) -> Self {
        self.contrast = Some(value);
        self
    }

    /// Sets brightness, in place of any value set before;
    /// [`brightness`](Self::brightness) reads it.
    #[must_use]
    pub fn with_brightness(mut self, value: PictureControlValue) -> Self {
        self.brightness = Some(value);
        self
    }

    /// Sets saturation, in place of any value set before;
    /// [`saturation`](Self::saturation) reads it.
    #[must_use]
    pub fn with_saturation(mut self, value: PictureControlValue) -> Self {
        self.saturation = Some(value);
        self
    }

    /// Sets hue, in place of any value set before;
    /// [`hue`](Self::hue) reads it.
    #[must_use]
    pub fn with_hue(mut self, value: PictureControlValue) -> Self {
        self.hue = Some(value);
        self
    }

    /// Sets the filter effect, in place of any set before;
    /// [`filter_effect`](Self::filter_effect) reads it.
    #[must_use]
    pub fn with_filter_effect(mut self, effect: PictureControlFilterEffect) -> Self {
        self.filter_effect = Some(effect);
        self
    }

    /// Sets the toning, in place of any set before;
    /// [`toning`](Self::toning) reads it.
    #[must_use]
    pub fn with_toning(mut self, toning: PictureControlToning) -> Self {
        self.toning = Some(toning);
        self
    }

    /// Sets the toning's saturation, in place of any value set before;
    /// [`toning_saturation`](Self::toning_saturation) reads it.
    #[must_use]
    pub fn with_toning_saturation(mut self, value: PictureControlValue) -> Self {
        self.toning_saturation = Some(value);
        self
    }
}

/// Sony's Creative Style: the preset of image processing the camera
/// processed the picture in, with the photographer's contrast, saturation
/// and sharpness.
///
/// The name is as the file records it, such as `Standard`, `Vivid`, `BW`,
/// or `Off` and `None`, which files hold too: raw-files keeps no list of
/// Creative Style names. The three values are the camera's steps from the
/// style's own value, as recorded: 0 is the style's own, and Sony's later
/// bodies have a [`CreativeLook`] in its place, whose sharpness counts on
/// another scale.
///
/// Read from Sony's MakerNote, in a raw file and a JPEG alike: the name
/// from CreativeStyle (0xB020), and the contrast, saturation and sharpness
/// from Contrast, Saturation and Sharpness (0x2004 to 0x2006), each one
/// SLONG. A file gives a Creative Style when at least one of the four
/// gives a value and the file records none of a Creative Look's further
/// values. Each part is `None` when its tag is missing or breaks its rule,
/// and a damaged part costs only itself. What files record is in the
/// [ARW notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#911-picture-style-and-processing).
///
/// `CreativeStyle::default()` is the empty style, with every part `None`;
/// a caller's tests set the parts they need with the `with_…` methods,
/// each named after the method that reads the part back, each replacing
/// any value set before and storing it as given. `CreativeStyle` is
/// `Clone` and not `Copy`.
///
/// # Examples
///
/// ```
/// use raw_files::CreativeStyle;
///
/// let style = CreativeStyle::default()
///     .with_name("Vivid")
///     .with_contrast(0)
///     .with_saturation(1)
///     .with_sharpness(0);
/// assert_eq!(style.name(), Some("Vivid"));
/// assert_eq!(style.saturation(), Some(1));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CreativeStyle {
    pub(crate) name: Option<String>,
    pub(crate) contrast: Option<i32>,
    pub(crate) saturation: Option<i32>,
    pub(crate) sharpness: Option<i32>,
}

impl CreativeStyle {
    /// The style's name as the file records it, such as `Standard`,
    /// cleaned as every text.
    ///
    /// `None` when the file records none, as seven of Sony's DSLRs do,
    /// when the text is blank, or when the tag is not ASCII.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The contrast, in the camera's steps from the style's own, 0.
    ///
    /// `None` when the file records none, as the DSC-R1 and the DSLR-A100
    /// do, or when the tag is not one SLONG.
    pub fn contrast(&self) -> Option<i32> {
        self.contrast
    }

    /// The saturation, in the camera's steps from the style's own, 0.
    ///
    /// `None` when the file records none, as the DSC-R1 and the DSLR-A100
    /// do, or when the tag is not one SLONG.
    pub fn saturation(&self) -> Option<i32> {
        self.saturation
    }

    /// The sharpness, in the camera's steps from the style's own, 0.
    ///
    /// `None` when the file records none, as the DSC-R1 and the DSLR-A100
    /// do, or when the tag is not one SLONG.
    pub fn sharpness(&self) -> Option<i32> {
        self.sharpness
    }

    /// Sets the name, in place of any set before;
    /// [`name`](Self::name) reads it.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the contrast, in place of any set before;
    /// [`contrast`](Self::contrast) reads it.
    #[must_use]
    pub fn with_contrast(mut self, steps: i32) -> Self {
        self.contrast = Some(steps);
        self
    }

    /// Sets the saturation, in place of any set before;
    /// [`saturation`](Self::saturation) reads it.
    #[must_use]
    pub fn with_saturation(mut self, steps: i32) -> Self {
        self.saturation = Some(steps);
        self
    }

    /// Sets the sharpness, in place of any set before;
    /// [`sharpness`](Self::sharpness) reads it.
    #[must_use]
    pub fn with_sharpness(mut self, steps: i32) -> Self {
        self.sharpness = Some(steps);
        self
    }
}

/// Sony's Creative Look: the picture style that Sony's later bodies, from
/// the ILCE-7SM3 on, have in the [`CreativeStyle`]'s place, with the
/// photographer's contrast, saturation and sharpness.
///
/// The name is as the file records it, such as `Standard` or `Off`, and
/// written out where the camera's menu abbreviates it: a look the menu
/// calls VV is `Vivid`. raw-files keeps no list of names. The three values
/// are the camera's steps, as recorded, and the sharpness counts on
/// another scale than a Creative Style's: its default is 4, where a
/// Creative Style's is 0.
///
/// Read from Sony's MakerNote, in the same four tags as a Creative Style:
/// the name from CreativeStyle (0xB020), and the contrast, saturation and
/// sharpness from 0x2004 to 0x2006, each one SLONG. A file gives a
/// Creative Look in place of a Creative Style when it also records any of
/// the look's five further values, 0x2032 to 0x2036: highlights, shadows,
/// fade, sharpness range and clarity. Those five are not given. An entry
/// the MakerNote skipped as damaged is not counted among them, so a look
/// whose five entries were all damaged reads as a Creative Style. Each part
/// is `None` when its tag is missing or breaks its rule, and a damaged
/// part costs only itself. What files record is in the
/// [ARW notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/arw/makernote-and-metadata.md#911-picture-style-and-processing).
///
/// `CreativeLook::default()` is the empty look, with every part `None`;
/// a caller's tests set the parts they need with the `with_…` methods,
/// each named after the method that reads the part back, each replacing
/// any value set before and storing it as given. `CreativeLook` is
/// `Clone` and not `Copy`.
///
/// # Examples
///
/// ```
/// use raw_files::CreativeLook;
///
/// let look = CreativeLook::default().with_name("Standard").with_sharpness(4);
/// assert_eq!(look.sharpness(), Some(4));
/// assert_eq!(look.contrast(), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CreativeLook {
    pub(crate) name: Option<String>,
    pub(crate) contrast: Option<i32>,
    pub(crate) saturation: Option<i32>,
    pub(crate) sharpness: Option<i32>,
}

impl CreativeLook {
    /// The look's name as the file records it, such as `Standard`,
    /// cleaned as every text.
    ///
    /// `None` when the file records none, when the text is blank, or when
    /// the tag is not ASCII.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The contrast, in the camera's steps from the look's own, 0.
    ///
    /// `None` when the file records none, or when the tag is not one
    /// SLONG.
    pub fn contrast(&self) -> Option<i32> {
        self.contrast
    }

    /// The saturation, in the camera's steps from the look's own, 0.
    ///
    /// `None` when the file records none, or when the tag is not one
    /// SLONG.
    pub fn saturation(&self) -> Option<i32> {
        self.saturation
    }

    /// The sharpness, in the camera's steps, as recorded: 4 is the look's
    /// own, where a Creative Style's is 0, and Exif's Sharpness says
    /// normal beside both.
    ///
    /// `None` when the file records none, or when the tag is not one
    /// SLONG.
    pub fn sharpness(&self) -> Option<i32> {
        self.sharpness
    }

    /// Sets the name, in place of any set before;
    /// [`name`](Self::name) reads it.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the contrast, in place of any set before;
    /// [`contrast`](Self::contrast) reads it.
    #[must_use]
    pub fn with_contrast(mut self, steps: i32) -> Self {
        self.contrast = Some(steps);
        self
    }

    /// Sets the saturation, in place of any set before;
    /// [`saturation`](Self::saturation) reads it.
    #[must_use]
    pub fn with_saturation(mut self, steps: i32) -> Self {
        self.saturation = Some(steps);
        self
    }

    /// Sets the sharpness, in place of any set before;
    /// [`sharpness`](Self::sharpness) reads it.
    #[must_use]
    pub fn with_sharpness(mut self, steps: i32) -> Self {
        self.sharpness = Some(steps);
        self
    }
}

/// One value of a Picture Control: auto, where the camera chooses it, or
/// a count of steps.
///
/// The steps are kept as the record holds them: the byte less 128 over
/// the divisor recorded beside it. So five steps are `20/4` in the
/// records that count quarter steps, and `5/1` in the oldest, which
/// counts whole steps. Each is the value on the menu's own scale.
/// Contrast, brightness, saturation, hue, quick adjust, Quick sharp and
/// the toning's saturation are 0 under a preset's own settings.
/// Sharpening, mid-range sharpening and clarity hold the preset's level
/// instead, such as a sharpening of 3 under an unadjusted `STANDARD`. Two values of one count in different records are
/// so not equal; compare their [`reduced`](SignedRational::reduced)
/// forms, or [`to_f64`](SignedRational::to_f64), to compare counts.
///
/// `PictureControlValue` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It is `Copy`, and has no `Display`: how a quarter step
/// is written is the viewer's choice.
///
/// # Examples
///
/// ```
/// use raw_files::{PictureControlValue, SignedRational};
///
/// let value = PictureControlValue::Steps(SignedRational { numerator: 14, denominator: 4 });
/// match value {
///     PictureControlValue::Auto => println!("auto"),
///     PictureControlValue::Steps(steps) => assert_eq!(steps.to_f64(), 3.5),
///     _ => {}
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PictureControlValue {
    /// The camera chooses the value: "A (auto)" in Nikon's menus.
    Auto,
    /// A count of steps on the menu's scale, as recorded.
    Steps(SignedRational),
}

/// How the photographer adjusted a Picture Control, as Nikon's record
/// names it: 0 the default settings, 1 by quick adjust or Quick sharp, 2
/// value by value, full control. A code the list does not name gives
/// `None`.
///
/// `PictureControlAdjustment` is `#[non_exhaustive]`, so match on it with
/// a wildcard arm. It orders as its variants are listed, and a new
/// variant is added after the existing ones, so an order once observed
/// holds. Its [`name`](Self::name)s, which `Display` writes too, are
/// "Default settings", "Quick adjust" and "Full control".
///
/// # Examples
///
/// ```
/// use raw_files::PictureControlAdjustment;
///
/// assert_eq!(PictureControlAdjustment::FullControl.name(), "Full control");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PictureControlAdjustment {
    /// The preset's own values, unadjusted.
    Default,
    /// Adjusted by quick adjust or, on the Z bodies, Quick sharp.
    QuickAdjust,
    /// Adjusted value by value.
    FullControl,
}

impl PictureControlAdjustment {
    /// The adjustment's name, for an information panel, such as "Quick
    /// adjust"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            PictureControlAdjustment::Default => "Default settings",
            PictureControlAdjustment::QuickAdjust => "Quick adjust",
            PictureControlAdjustment::FullControl => "Full control",
        }
    }
}

impl fmt::Display for PictureControlAdjustment {
    /// Writes [`name`](PictureControlAdjustment::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The filter effect of a monochrome Picture Control, which imitates a
/// color filter on black-and-white film, as Nikon's manuals list them:
/// off, yellow, orange, red and green. A code the list does not name
/// gives `None`.
///
/// `PictureControlFilterEffect` is `#[non_exhaustive]`, so match on it
/// with a wildcard arm. It orders as its variants are listed, and a new
/// variant is added after the existing ones, so an order once observed
/// holds. Its [`name`](Self::name)s, which `Display` writes too, are
/// "Off", "Yellow", "Orange", "Red" and "Green".
///
/// # Examples
///
/// ```
/// use raw_files::PictureControlFilterEffect;
///
/// assert_eq!(PictureControlFilterEffect::Orange.to_string(), "Orange");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PictureControlFilterEffect {
    /// No filter effect.
    Off,
    /// A yellow filter.
    Yellow,
    /// An orange filter.
    Orange,
    /// A red filter.
    Red,
    /// A green filter.
    Green,
}

impl PictureControlFilterEffect {
    /// The filter effect's name, for an information panel, such as
    /// "Yellow"; the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            PictureControlFilterEffect::Off => "Off",
            PictureControlFilterEffect::Yellow => "Yellow",
            PictureControlFilterEffect::Orange => "Orange",
            PictureControlFilterEffect::Red => "Red",
            PictureControlFilterEffect::Green => "Green",
        }
    }
}

impl fmt::Display for PictureControlFilterEffect {
    /// Writes [`name`](PictureControlFilterEffect::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The toning of a monochrome Picture Control, as Nikon's manuals list
/// them: black-and-white, sepia, cyanotype, and seven tints. A code the
/// list does not name gives `None`.
///
/// `PictureControlToning` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It orders as its variants are listed, and a new variant
/// is added after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are
/// "Black-and-white", "Sepia", "Cyanotype", "Red", "Yellow", "Green",
/// "Blue green", "Blue", "Purple blue" and "Red purple".
///
/// # Examples
///
/// ```
/// use raw_files::PictureControlToning;
///
/// assert_eq!(PictureControlToning::BlueGreen.name(), "Blue green");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PictureControlToning {
    /// Black-and-white, untinted.
    BlackAndWhite,
    /// Sepia.
    Sepia,
    /// Cyanotype, a blue tint.
    Cyanotype,
    /// A red tint.
    Red,
    /// A yellow tint.
    Yellow,
    /// A green tint.
    Green,
    /// A blue-green tint.
    BlueGreen,
    /// A blue tint.
    Blue,
    /// A purple-blue tint.
    PurpleBlue,
    /// A red-purple tint.
    RedPurple,
}

impl PictureControlToning {
    /// The toning's name, for an information panel, such as "Sepia"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            PictureControlToning::BlackAndWhite => "Black-and-white",
            PictureControlToning::Sepia => "Sepia",
            PictureControlToning::Cyanotype => "Cyanotype",
            PictureControlToning::Red => "Red",
            PictureControlToning::Yellow => "Yellow",
            PictureControlToning::Green => "Green",
            PictureControlToning::BlueGreen => "Blue green",
            PictureControlToning::Blue => "Blue",
            PictureControlToning::PurpleBlue => "Purple blue",
            PictureControlToning::RedPurple => "Red purple",
        }
    }
}

impl fmt::Display for PictureControlToning {
    /// Writes [`name`](PictureControlToning::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the camera optimized the picture's dynamic range, under its
/// maker's own name: the variant says which maker's setting it is, and
/// holds that setting's own type.
///
/// The setting keeps detail in a picture's highlights and shadows. Each
/// maker names it and cuts its steps in its own way, so each keeps its own
/// type:
///
/// - [`ActiveDLighting`](Self::ActiveDLighting), Nikon's Active
///   D-Lighting;
/// - [`DRangeOptimizer`](Self::DRangeOptimizer), Sony's D-Range
///   Optimizer.
///
/// [`setting_name`](Self::setting_name) gives the maker's word for the
/// setting, for a viewer's label, and [`name`](Self::name) the step's
/// name, whichever maker's it is.
///
/// `DynamicRangeOptimization` is `#[non_exhaustive]`, so match on it with
/// a wildcard arm: another maker's setting is added as a variant. It is
/// `Copy`, and has no `Default`: a setting is always one maker's. It has
/// no `Display` either, as [`PictureStyle`] has none: a viewer writes the
/// setting's name and the step's together. Each maker's type converts into
/// it with `From`.
///
/// # Examples
///
/// Each maker's setting, labeled for a viewer and read by its variant:
///
/// ```
/// use raw_files::{ActiveDLighting, DRangeOptimizer, DynamicRangeOptimization};
///
/// let nikon = DynamicRangeOptimization::from(ActiveDLighting::Auto);
/// let sony = DynamicRangeOptimization::from(DRangeOptimizer::Level3);
///
/// for optimization in [nikon, sony] {
///     println!("{}: {}", optimization.setting_name(), optimization.name());
///     let chosen_by_the_camera = match optimization {
///         DynamicRangeOptimization::ActiveDLighting(step) => step == ActiveDLighting::Auto,
///         DynamicRangeOptimization::DRangeOptimizer(step) => step == DRangeOptimizer::Auto,
///         _ => false,
///     };
///     println!("chosen by the camera: {chosen_by_the_camera}");
/// }
/// assert_eq!(nikon.setting_name(), "Active D-Lighting");
/// assert_eq!(nikon.name(), "Auto");
/// assert_eq!(sony.setting_name(), "D-Range Optimizer");
/// assert_eq!(sony.name(), "Level 3");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DynamicRangeOptimization {
    /// Nikon's Active D-Lighting.
    ActiveDLighting(ActiveDLighting),
    /// Sony's D-Range Optimizer.
    DRangeOptimizer(DRangeOptimizer),
}

impl DynamicRangeOptimization {
    /// The maker's word for the setting, for a viewer's label: "Active
    /// D-Lighting" or "D-Range Optimizer".
    pub fn setting_name(&self) -> &'static str {
        match self {
            DynamicRangeOptimization::ActiveDLighting(_) => "Active D-Lighting",
            DynamicRangeOptimization::DRangeOptimizer(_) => "D-Range Optimizer",
        }
    }

    /// The step's name, whichever maker's setting it is, such as "Auto":
    /// the `name` of the variant's own type, [`ActiveDLighting::name`] or
    /// [`DRangeOptimizer::name`].
    pub fn name(&self) -> &'static str {
        match self {
            DynamicRangeOptimization::ActiveDLighting(step) => step.name(),
            DynamicRangeOptimization::DRangeOptimizer(step) => step.name(),
        }
    }
}

impl From<ActiveDLighting> for DynamicRangeOptimization {
    /// The Active D-Lighting step as a dynamic range optimization, its
    /// [`ActiveDLighting`](DynamicRangeOptimization::ActiveDLighting)
    /// variant.
    fn from(step: ActiveDLighting) -> Self {
        DynamicRangeOptimization::ActiveDLighting(step)
    }
}

/// Nikon's Active D-Lighting: how strongly the camera kept detail in the
/// highlights and shadows, by adjusting the exposure before the shot, or
/// that it was off.
///
/// The steps are those of Nikon's cameras: the D600's manual offers auto,
/// extra high, high, normal, low and off. ExifTool's documentation also
/// gives four numbered steps past extra high, Extra High 1 to Extra High
/// 4, which no Nikon manual read for raw-files names; their variants rest
/// on the documentation's words.
///
/// `ActiveDLighting` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. The steps are listed from the weakest to the strongest,
/// then auto, and `ActiveDLighting` orders as they are listed. A new step
/// is added where the ladder puts it, between the existing ones if need
/// be, so two steps keep the order once observed between them. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Off", "Low",
/// "Normal", "High", "Extra high", "Extra high 1" to "Extra high 4" and
/// "Auto".
///
/// # Examples
///
/// ```
/// use raw_files::ActiveDLighting;
///
/// assert_eq!(ActiveDLighting::ExtraHigh.name(), "Extra high");
/// assert_eq!(ActiveDLighting::ExtraHigh2.to_string(), "Extra high 2");
/// assert!(ActiveDLighting::Low < ActiveDLighting::High);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ActiveDLighting {
    /// Active D-Lighting off.
    Off,
    /// The low step.
    Low,
    /// The normal step.
    Normal,
    /// The high step.
    High,
    /// The extra high step.
    ExtraHigh,
    /// The documentation's Extra High 1, the first numbered step past
    /// extra high.
    ExtraHigh1,
    /// The documentation's Extra High 2.
    ExtraHigh2,
    /// The documentation's Extra High 3.
    ExtraHigh3,
    /// The documentation's Extra High 4, the strongest.
    ExtraHigh4,
    /// The camera chose the strength from the scene.
    Auto,
}

impl ActiveDLighting {
    /// The step's name, for an information panel, such as "Extra high";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            ActiveDLighting::Off => "Off",
            ActiveDLighting::Low => "Low",
            ActiveDLighting::Normal => "Normal",
            ActiveDLighting::High => "High",
            ActiveDLighting::ExtraHigh => "Extra high",
            ActiveDLighting::ExtraHigh1 => "Extra high 1",
            ActiveDLighting::ExtraHigh2 => "Extra high 2",
            ActiveDLighting::ExtraHigh3 => "Extra high 3",
            ActiveDLighting::ExtraHigh4 => "Extra high 4",
            ActiveDLighting::Auto => "Auto",
        }
    }
}

impl fmt::Display for ActiveDLighting {
    /// Writes [`name`](ActiveDLighting::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl From<DRangeOptimizer> for DynamicRangeOptimization {
    /// The D-Range Optimizer's step as a dynamic range optimization, its
    /// [`DRangeOptimizer`](DynamicRangeOptimization::DRangeOptimizer)
    /// variant.
    fn from(step: DRangeOptimizer) -> Self {
        DynamicRangeOptimization::DRangeOptimizer(step)
    }
}

/// Sony's D-Range Optimizer: how the camera set the picture's brightness
/// and gradation from its contrast, analyzed area by area, or that it was
/// off.
///
/// Sony's manual for the SLT-A99 names the setting and gives no list of
/// its levels. The variants follow ExifTool's documentation, whose words
/// for the levels are "Advanced Lv1" to "Advanced Lv5" and "Lv1" to
/// "Lv8":
///
/// - [`Off`](Self::Off), and [`Auto`](Self::Auto), under which the camera
///   chose the level;
/// - [`Standard`](Self::Standard) and [`AdvancedAuto`](Self::AdvancedAuto),
///   which the measured files hold on A-mount DSLRs alone;
/// - [`AdvancedLevel1`](Self::AdvancedLevel1) to
///   [`AdvancedLevel5`](Self::AdvancedLevel5);
/// - [`Level1`](Self::Level1) to [`Level8`](Self::Level8), a level the
///   photographer chose, from the weakest to the strongest.
///
/// `DRangeOptimizer` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It orders as its variants are listed, which is the order
/// of Sony's codes, and a new variant is added after the existing ones,
/// so an order once observed holds. Its [`name`](Self::name)s, which
/// `Display` writes too, are "Off", "Standard", "Advanced auto", "Auto",
/// "Advanced level 1" to "Advanced level 5" and "Level 1" to "Level 8".
///
/// # Examples
///
/// ```
/// use raw_files::DRangeOptimizer;
///
/// assert_eq!(DRangeOptimizer::AdvancedAuto.name(), "Advanced auto");
/// assert_eq!(DRangeOptimizer::Level5.to_string(), "Level 5");
/// assert!(DRangeOptimizer::Level1 < DRangeOptimizer::Level8);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DRangeOptimizer {
    /// The D-Range Optimizer off.
    Off,
    /// The standard step of the A-mount DSLRs.
    Standard,
    /// The advanced auto step of the A-mount DSLRs.
    AdvancedAuto,
    /// The camera chose the level.
    Auto,
    /// The documentation's Advanced Lv1, the weakest advanced level.
    AdvancedLevel1,
    /// The documentation's Advanced Lv2.
    AdvancedLevel2,
    /// The documentation's Advanced Lv3.
    AdvancedLevel3,
    /// The documentation's Advanced Lv4.
    AdvancedLevel4,
    /// The documentation's Advanced Lv5, the strongest advanced level.
    AdvancedLevel5,
    /// The documentation's Lv1, the weakest level.
    Level1,
    /// The documentation's Lv2.
    Level2,
    /// The documentation's Lv3.
    Level3,
    /// The documentation's Lv4.
    Level4,
    /// The documentation's Lv5.
    Level5,
    /// The documentation's Lv6.
    Level6,
    /// The documentation's Lv7.
    Level7,
    /// The documentation's Lv8, the strongest level.
    Level8,
}

impl DRangeOptimizer {
    /// The step's name, for an information panel, such as "Level 3"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            DRangeOptimizer::Off => "Off",
            DRangeOptimizer::Standard => "Standard",
            DRangeOptimizer::AdvancedAuto => "Advanced auto",
            DRangeOptimizer::Auto => "Auto",
            DRangeOptimizer::AdvancedLevel1 => "Advanced level 1",
            DRangeOptimizer::AdvancedLevel2 => "Advanced level 2",
            DRangeOptimizer::AdvancedLevel3 => "Advanced level 3",
            DRangeOptimizer::AdvancedLevel4 => "Advanced level 4",
            DRangeOptimizer::AdvancedLevel5 => "Advanced level 5",
            DRangeOptimizer::Level1 => "Level 1",
            DRangeOptimizer::Level2 => "Level 2",
            DRangeOptimizer::Level3 => "Level 3",
            DRangeOptimizer::Level4 => "Level 4",
            DRangeOptimizer::Level5 => "Level 5",
            DRangeOptimizer::Level6 => "Level 6",
            DRangeOptimizer::Level7 => "Level 7",
            DRangeOptimizer::Level8 => "Level 8",
        }
    }
}

impl fmt::Display for DRangeOptimizer {
    /// Writes [`name`](DRangeOptimizer::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether the camera combined exposures into one picture of a high
/// dynamic range, and how, under its maker's own name: the variant says
/// which maker's setting it is, and holds that setting's own type.
///
/// Each maker names the setting and records its parts in its own way, so
/// each keeps its own type:
///
/// - [`Hdr`](Self::Hdr), Nikon's HDR (high dynamic range), which combines
///   two exposures;
/// - [`AutoHdr`](Self::AutoHdr), Sony's Auto HDR, which combines three.
///
/// [`setting_name`](Self::setting_name) gives the maker's word for the
/// setting, for a viewer's label, and [`is_on`](Self::is_on) whether it
/// was on, whichever maker's it is.
///
/// `HighDynamicRange` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm: another maker's setting is added as a variant. It is
/// `Copy`, and has no `Default`: a setting is always one maker's. It has
/// no `Display` either: its parts hold numbers, which a viewer writes in
/// its own way. Each maker's type converts into it with `From`.
///
/// # Examples
///
/// Each maker's setting, labeled for a viewer and read by its variant:
///
/// ```
/// use raw_files::{AutoHdr, Hdr, HdrStrength, HighDynamicRange};
///
/// let nikon = HighDynamicRange::from(Hdr::on().with_strength(HdrStrength::Auto));
/// let sony = HighDynamicRange::from(AutoHdr::off());
///
/// for setting in [nikon, sony] {
///     let state = if setting.is_on() { "on" } else { "off" };
///     let detail = match setting {
///         HighDynamicRange::Hdr(hdr) => format!("{:?}", hdr.strength()),
///         HighDynamicRange::AutoHdr(auto_hdr) => format!("{:?}", auto_hdr.exposure_difference()),
///         _ => String::new(),
///     };
///     println!("{}: {state} {detail}", setting.setting_name());
/// }
/// assert_eq!(nikon.setting_name(), "HDR");
/// assert!(nikon.is_on());
/// assert_eq!(sony.setting_name(), "Auto HDR");
/// assert!(!sony.is_on());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HighDynamicRange {
    /// Nikon's HDR.
    Hdr(Hdr),
    /// Sony's Auto HDR.
    AutoHdr(AutoHdr),
}

impl HighDynamicRange {
    /// The maker's word for the setting, for a viewer's label: "HDR" or
    /// "Auto HDR".
    pub fn setting_name(&self) -> &'static str {
        match self {
            HighDynamicRange::Hdr(_) => "HDR",
            HighDynamicRange::AutoHdr(_) => "Auto HDR",
        }
    }

    /// Whether the setting was on, whichever maker's it is: the `is_on`
    /// of the variant's own type, [`Hdr::is_on`] or [`AutoHdr::is_on`].
    pub fn is_on(&self) -> bool {
        match self {
            HighDynamicRange::Hdr(hdr) => hdr.is_on(),
            HighDynamicRange::AutoHdr(auto_hdr) => auto_hdr.is_on(),
        }
    }
}

impl From<Hdr> for HighDynamicRange {
    /// Nikon's HDR as a high dynamic range setting, its
    /// [`Hdr`](HighDynamicRange::Hdr) variant.
    fn from(hdr: Hdr) -> Self {
        HighDynamicRange::Hdr(hdr)
    }
}

/// Nikon's HDR (high dynamic range): whether the camera combined two
/// exposures into one JPEG, and, when it did, the parts of the setting the
/// file records.
///
/// Nikon's DSLRs and Z bodies record no NEF under HDR, so their JPEGs
/// alone hold it. The parts are Nikon's words, and which a file records
/// depends on its body:
///
/// - the D600 sets an [`exposure_differential`](Self::exposure_differential)
///   and a [`smoothing`](Self::smoothing);
/// - the D7100 and later bodies set a [`strength`](Self::strength).
///
/// There is no `Default`: a file that records no HDR gives `None` for
/// [`CameraSettings::high_dynamic_range`]. A file that records it off
/// gives one that [`is_on`](Self::is_on) says is off, with no parts.
///
/// # Examples
///
/// HDR on at an auto strength, as a D7100 records it, on as a D600
/// records it, and off:
///
/// ```
/// use raw_files::{Hdr, HdrExposureDifferential, HdrSmoothing, HdrStrength, Rational};
///
/// let d7100 = Hdr::on().with_strength(HdrStrength::Auto);
/// assert!(d7100.is_on());
/// assert_eq!(d7100.strength(), Some(HdrStrength::Auto));
/// assert_eq!(d7100.smoothing(), None);
///
/// let two_ev = Rational { numerator: 2, denominator: 1 };
/// let d600 = Hdr::on()
///     .with_exposure_differential(HdrExposureDifferential::Ev(two_ev))
///     .with_smoothing(HdrSmoothing::Normal);
/// assert_eq!(d600.exposure_differential(), Some(HdrExposureDifferential::Ev(two_ev)));
///
/// let off = Hdr::off();
/// assert!(!off.is_on());
/// assert_eq!(off.strength(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hdr {
    pub(crate) on: bool,
    pub(crate) strength: Option<HdrStrength>,
    pub(crate) exposure_differential: Option<HdrExposureDifferential>,
    pub(crate) smoothing: Option<HdrSmoothing>,
}

impl Hdr {
    /// HDR that was on, with no parts yet; the `with_…` methods add them.
    #[must_use]
    pub fn on() -> Self {
        Self {
            on: true,
            strength: None,
            exposure_differential: None,
            smoothing: None,
        }
    }

    /// HDR that was off, with no parts.
    #[must_use]
    pub fn off() -> Self {
        Self {
            on: false,
            ..Self::on()
        }
    }

    /// This HDR with the strength `strength`, in place of any it had. It
    /// is stored as given, whether or not HDR is on: the readers give a
    /// part only when it is.
    #[must_use]
    pub fn with_strength(self, strength: HdrStrength) -> Self {
        Self {
            strength: Some(strength),
            ..self
        }
    }

    /// This HDR with the exposure differential `differential`, in place of
    /// any it had, stored as given, as
    /// [`with_strength`](Self::with_strength) stores its part.
    #[must_use]
    pub fn with_exposure_differential(self, differential: HdrExposureDifferential) -> Self {
        Self {
            exposure_differential: Some(differential),
            ..self
        }
    }

    /// This HDR with the smoothing `smoothing`, in place of any it had,
    /// stored as given, as [`with_strength`](Self::with_strength) stores
    /// its part.
    #[must_use]
    pub fn with_smoothing(self, smoothing: HdrSmoothing) -> Self {
        Self {
            smoothing: Some(smoothing),
            ..self
        }
    }

    /// Whether HDR was on.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// How strongly the camera combined the exposures, the D7100's HDR
    /// strength, or `None` when none is recorded: when HDR is off, the
    /// file records the D600's parts instead, or the record marks the
    /// strength not available.
    pub fn strength(&self) -> Option<HdrStrength> {
        self.strength
    }

    /// The difference in exposure between the two shots, the D600's
    /// exposure differential, or `None` when none is recorded: when HDR is
    /// off, or the file records a strength instead.
    pub fn exposure_differential(&self) -> Option<HdrExposureDifferential> {
        self.exposure_differential
    }

    /// How much the camera smoothed the boundaries between the two shots,
    /// the D600's smoothing, or `None` when none is recorded: when HDR is
    /// off, or the file records a strength instead.
    pub fn smoothing(&self) -> Option<HdrSmoothing> {
        self.smoothing
    }
}

/// How strongly Nikon's HDR combined the exposures, in the steps of the
/// D7100's menu: low, normal, high, extra high or auto.
///
/// ExifTool's documentation calls extra high "High+"; the D7100's manual
/// names it extra high.
///
/// `HdrStrength` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. The steps are listed from the weakest to the strongest, then
/// auto, and `HdrStrength` orders as they are listed. A new step is added
/// where the ladder puts it, between the existing ones if need be, so two
/// steps keep the order once observed between them. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Low",
/// "Normal", "High", "Extra high" and "Auto".
///
/// # Examples
///
/// ```
/// use raw_files::HdrStrength;
///
/// assert_eq!(HdrStrength::ExtraHigh.name(), "Extra high");
/// assert!(HdrStrength::Low < HdrStrength::High);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum HdrStrength {
    /// The low step.
    Low,
    /// The normal step.
    Normal,
    /// The high step.
    High,
    /// The extra high step, the documentation's "High+".
    ExtraHigh,
    /// The camera chose the strength from the scene.
    Auto,
}

impl HdrStrength {
    /// The step's name, for an information panel, such as "Extra high";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            HdrStrength::Low => "Low",
            HdrStrength::Normal => "Normal",
            HdrStrength::High => "High",
            HdrStrength::ExtraHigh => "Extra high",
            HdrStrength::Auto => "Auto",
        }
    }
}

impl fmt::Display for HdrStrength {
    /// Writes [`name`](HdrStrength::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The difference in exposure between the two shots of Nikon's HDR, the
/// D600's exposure differential: chosen by the camera, or a number of EV.
///
/// The D600 offers auto and 1, 2 or 3 EV. The value is one or the other,
/// as a [`PictureControlValue`] is auto or a count of steps.
///
/// `HdrExposureDifferential` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. It has no `Display`: how a number of EV is written is the
/// viewer's choice.
///
/// # Examples
///
/// ```
/// use raw_files::{HdrExposureDifferential, Rational};
///
/// let differential = HdrExposureDifferential::Ev(Rational { numerator: 3, denominator: 1 });
/// let text = match differential {
///     HdrExposureDifferential::Auto => "auto".to_string(),
///     HdrExposureDifferential::Ev(ev) => format!("{} EV", ev.to_f64()),
///     _ => String::new(),
/// };
/// assert_eq!(text, "3 EV");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HdrExposureDifferential {
    /// The camera chose the differential from the scene.
    Auto,
    /// A differential of this many EV, in whole steps, such as `2/1`.
    Ev(Rational),
}

/// How much Nikon's HDR smoothed the boundaries between the two shots, in
/// the steps of the D600's menu: low, normal or high, with auto and off,
/// which the documentation also lists.
///
/// `HdrSmoothing` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. The steps are listed from none to the strongest, then auto, and
/// `HdrSmoothing` orders as they are listed. A new step is added where the
/// ladder puts it, between the existing ones if need be, so two steps keep
/// the order once observed between them. Its [`name`](Self::name)s, which
/// `Display` writes too, are "Off", "Low", "Normal", "High" and "Auto".
///
/// # Examples
///
/// ```
/// use raw_files::HdrSmoothing;
///
/// assert_eq!(HdrSmoothing::Normal.to_string(), "Normal");
/// assert!(HdrSmoothing::Off < HdrSmoothing::Low);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum HdrSmoothing {
    /// No smoothing, which the D600's menu does not offer.
    Off,
    /// The low step.
    Low,
    /// The normal step.
    Normal,
    /// The high step.
    High,
    /// The camera chose the smoothing.
    Auto,
}

impl HdrSmoothing {
    /// The step's name, for an information panel, such as "Normal"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            HdrSmoothing::Off => "Off",
            HdrSmoothing::Low => "Low",
            HdrSmoothing::Normal => "Normal",
            HdrSmoothing::High => "High",
            HdrSmoothing::Auto => "Auto",
        }
    }
}

impl fmt::Display for HdrSmoothing {
    /// Writes [`name`](HdrSmoothing::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl From<AutoHdr> for HighDynamicRange {
    /// Sony's Auto HDR as a high dynamic range setting, its
    /// [`AutoHdr`](HighDynamicRange::AutoHdr) variant.
    fn from(auto_hdr: AutoHdr) -> Self {
        HighDynamicRange::AutoHdr(auto_hdr)
    }
}

/// Sony's Auto HDR: whether the camera shot three exposures and overlaid
/// them into one picture, and, when it did, the exposure difference
/// between them.
///
/// There is no `Default`: a file that records no Auto HDR gives `None`
/// for [`CameraSettings::high_dynamic_range`]. A file that records it off
/// gives one that [`is_on`](Self::is_on) says is off, with no exposure
/// difference.
///
/// # Examples
///
/// Auto HDR on with a difference of 2.5 EV, on with the difference left to
/// the camera, and off:
///
/// ```
/// use raw_files::{AutoHdr, AutoHdrExposureDifference, Rational};
///
/// let halves = Rational { numerator: 5, denominator: 2 };
/// let chosen = AutoHdr::on().with_exposure_difference(AutoHdrExposureDifference::Ev(halves));
/// assert!(chosen.is_on());
/// assert_eq!(chosen.exposure_difference(), Some(AutoHdrExposureDifference::Ev(halves)));
///
/// let auto = AutoHdr::on().with_exposure_difference(AutoHdrExposureDifference::Auto);
/// assert_eq!(auto.exposure_difference(), Some(AutoHdrExposureDifference::Auto));
///
/// let off = AutoHdr::off();
/// assert!(!off.is_on());
/// assert_eq!(off.exposure_difference(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AutoHdr {
    pub(crate) on: bool,
    pub(crate) exposure_difference: Option<AutoHdrExposureDifference>,
}

impl AutoHdr {
    /// Auto HDR that was on, with no exposure difference yet;
    /// [`with_exposure_difference`](Self::with_exposure_difference) adds
    /// it.
    #[must_use]
    pub fn on() -> Self {
        Self {
            on: true,
            exposure_difference: None,
        }
    }

    /// Auto HDR that was off, with no exposure difference.
    #[must_use]
    pub fn off() -> Self {
        Self {
            on: false,
            exposure_difference: None,
        }
    }

    /// This Auto HDR with the exposure difference `difference`, in place
    /// of any it had. It is stored as given, whether or not Auto HDR is
    /// on: the readers give it only when it is.
    #[must_use]
    pub fn with_exposure_difference(self, difference: AutoHdrExposureDifference) -> Self {
        Self {
            exposure_difference: Some(difference),
            ..self
        }
    }

    /// Whether Auto HDR was on.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The difference in exposure between the three shots, or `None`
    /// when none is recorded, as when Auto HDR is off.
    pub fn exposure_difference(&self) -> Option<AutoHdrExposureDifference> {
        self.exposure_difference
    }
}

/// The difference in exposure between the three shots of Sony's Auto
/// HDR: chosen by the camera, or a number of EV, from 1.0 to 6.0 EV in
/// steps of half an EV.
///
/// The value is one or the other, as a [`PictureControlValue`] is auto or
/// a count of steps. A number of EV is kept in halves, as the camera
/// counts its steps: 1.0 EV is `2/2` and 2.5 EV `5/2`.
///
/// `AutoHdrExposureDifference` is `#[non_exhaustive]`, so match on it with
/// a wildcard arm. It has no `Display`: how a number of EV is written is
/// the viewer's choice.
///
/// # Examples
///
/// ```
/// use raw_files::{AutoHdrExposureDifference, Rational};
///
/// let difference = AutoHdrExposureDifference::Ev(Rational { numerator: 5, denominator: 2 });
/// let text = match difference {
///     AutoHdrExposureDifference::Auto => "auto".to_string(),
///     AutoHdrExposureDifference::Ev(ev) => format!("{} EV", ev.to_f64()),
///     _ => String::new(),
/// };
/// assert_eq!(text, "2.5 EV");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AutoHdrExposureDifference {
    /// The camera chose the difference from the scene.
    Auto,
    /// A difference of this many EV, in halves, from `2/2` to `12/2`.
    Ev(Rational),
}

/// How strongly the camera reduced the noise of pictures taken at high
/// sensitivities, in its maker's steps, which one list holds for both
/// makers, as their menus name the setting alike, High ISO NR:
///
/// - Nikon's steps: [`Off`](Self::Off), [`Minimal`](Self::Minimal),
///   [`Low`](Self::Low), [`MediumLow`](Self::MediumLow),
///   [`Normal`](Self::Normal), [`MediumHigh`](Self::MediumHigh) and
///   [`High`](Self::High);
/// - Sony's steps: [`Off`](Self::Off), [`Low`](Self::Low),
///   [`Normal`](Self::Normal), [`High`](Self::High) and
///   [`Auto`](Self::Auto).
///
/// The D600's manual offers high, normal, low and off, and says that under
/// off the camera still reduces noise at high sensitivities, by less than
/// under low. Sony's SLT-A99 manual offers normal, low and off. Nikon's
/// minimal step follows off as the documentation numbers Nikon's codes, 0
/// and 1; which of the two reduces more is not known.
///
/// A maker's code is named where it is in a list that its published
/// documentation gives and that the measured files agree with, as the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md)
/// says; any other code gives `None`.
///
/// `HighIsoNoiseReduction` is `#[non_exhaustive]`, so match on it with a
/// wildcard arm. The steps are listed from the weakest to the strongest,
/// as the documentation numbers each maker's codes, then auto, so that
/// each maker's ladder runs in that order, and
/// `HighIsoNoiseReduction` orders as they are listed. A new step is added
/// where its maker's ladder puts it, between the existing ones if need
/// be, so two steps keep the order once observed between them. Across
/// makers the order is only the list's: Nikon's
/// [`Normal`](Self::Normal) and Sony's are one variant, but not the same
/// strength. Its [`name`](Self::name)s, which `Display` writes too, are
/// "Off", "Minimal", "Low", "Medium low", "Normal", "Medium high", "High"
/// and "Auto".
///
/// # Examples
///
/// ```
/// use raw_files::HighIsoNoiseReduction;
///
/// assert_eq!(HighIsoNoiseReduction::MediumLow.name(), "Medium low");
/// assert_eq!(HighIsoNoiseReduction::Normal.to_string(), "Normal");
/// assert!(HighIsoNoiseReduction::Low < HighIsoNoiseReduction::High);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum HighIsoNoiseReduction {
    /// Nikon's and Sony's off.
    Off,
    /// Nikon's minimal step, which its files of the D3200, the D600 and
    /// the D7100 hold beside off; what sets the two apart is not known.
    Minimal,
    /// Nikon's and Sony's low step.
    Low,
    /// Nikon's medium low step.
    MediumLow,
    /// Nikon's and Sony's normal step.
    Normal,
    /// Nikon's medium high step.
    MediumHigh,
    /// Nikon's and Sony's high step.
    High,
    /// Sony's auto: the camera chose the step.
    Auto,
}

impl HighIsoNoiseReduction {
    /// The step's name, for an information panel, such as "Medium low";
    /// the type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            HighIsoNoiseReduction::Off => "Off",
            HighIsoNoiseReduction::Minimal => "Minimal",
            HighIsoNoiseReduction::Low => "Low",
            HighIsoNoiseReduction::MediumLow => "Medium low",
            HighIsoNoiseReduction::Normal => "Normal",
            HighIsoNoiseReduction::MediumHigh => "Medium high",
            HighIsoNoiseReduction::High => "High",
            HighIsoNoiseReduction::Auto => "Auto",
        }
    }
}

impl fmt::Display for HighIsoNoiseReduction {
    /// Writes [`name`](HighIsoNoiseReduction::name).
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
    /// it chose, a shift in quarter steps along both axes, each value
    /// different from the others, and a preset slot and a lamp, which no
    /// camera records beside an auto setting.
    fn nikon_white_balance() -> WhiteBalance {
        WhiteBalance {
            mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(5_200),
            fine_tune: Some(WhiteBalanceFineTune {
                amber_blue: signed_fraction(-24, 4),
                green_magenta: signed_fraction(4, 4),
            }),
            preset_slot: Some(2),
            fluorescent_lamp: Some(FluorescentLamp::CoolWhite),
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
                red_eye_reduction: true,
                return_light: Some(true),
                source: Some(FlashSource::External),
                control: Some(FlashControl::Ttl),
                sync: Some(FlashSync::Slow),
                compensation: Some(signed_fraction(-2, 6)),
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
            stabilization: Some(Stabilization {
                on: true,
                mode: Some(StabilizationMode::Active),
            }),
            auto_iso: Some(true),
            exposure_fine_tune: Some(signed_fraction(-3, 6)),
            release_mode: Some(ReleaseMode::Continuous),
            bracketing: Some(Bracketing::Exposure),
            daylight_saving_time: Some(true),
            raw_bit_depth: Some(14),
            raw_compression: Some(RawCompression::Lossless),
            jpeg_quality: Some(JpegQuality::Fine),
            jpeg_compression: Some(JpegCompression::OptimalQuality),
            contrast: Some(Contrast::Hard),
            saturation: Some(Saturation::Low),
            sharpness: Some(Sharpness::Soft),
            scene_capture_type: Some(SceneCaptureType::Portrait),
            custom_rendered: Some(true),
            gain_control: Some(GainControl::HighGainUp),
            picture_style: Some(PictureStyle::PictureControl(full_control())),
            dynamic_range_optimization: Some(DynamicRangeOptimization::ActiveDLighting(
                ActiveDLighting::High,
            )),
            high_dynamic_range: Some(HighDynamicRange::Hdr(full_hdr())),
            high_iso_noise_reduction: Some(HighIsoNoiseReduction::MediumHigh),
        }
    }

    /// Nikon's HDR on, with every part set, each different from what the
    /// builders set in the tests below. No record holds a strength beside
    /// the other two.
    fn full_hdr() -> Hdr {
        Hdr {
            on: true,
            strength: Some(HdrStrength::ExtraHigh),
            exposure_differential: Some(HdrExposureDifferential::Ev(Rational {
                numerator: 2,
                denominator: 1,
            })),
            smoothing: Some(HdrSmoothing::High),
        }
    }

    /// `numerator` over `denominator` steps of a Picture Control value.
    fn steps(numerator: i32, denominator: i32) -> PictureControlValue {
        PictureControlValue::Steps(signed_fraction(numerator, denominator))
    }

    /// A Picture Control with every part set, each value different from
    /// the others and from what the builders set in the tests below. No
    /// record holds quick adjust beside Quick sharp, nor a toning beside a
    /// color control's saturation.
    fn full_control() -> PictureControl {
        PictureControl {
            name: Some("VIVID-KR".into()),
            base: Some("VIVID".into()),
            adjustment: Some(PictureControlAdjustment::FullControl),
            quick_adjust: Some(steps(1, 1)),
            quick_sharp: Some(steps(2, 1)),
            sharpening: Some(steps(20, 4)),
            mid_range_sharpening: Some(steps(14, 4)),
            clarity: Some(steps(8, 4)),
            contrast: Some(PictureControlValue::Auto),
            brightness: Some(steps(-4, 4)),
            saturation: Some(steps(3, 4)),
            hue: Some(steps(-1, 4)),
            filter_effect: Some(PictureControlFilterEffect::Red),
            toning: Some(PictureControlToning::Sepia),
            toning_saturation: Some(steps(5, 4)),
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
        assert_eq!(settings.stabilization(), None);
        assert_eq!(settings.auto_iso(), None);
        assert_eq!(settings.exposure_fine_tune(), None);
        assert_eq!(settings.release_mode(), None);
        assert_eq!(settings.bracketing(), None);
        assert_eq!(settings.daylight_saving_time(), None);
        assert_eq!(settings.raw_bit_depth(), None);
        assert_eq!(settings.raw_compression(), None);
        assert_eq!(settings.jpeg_quality(), None);
        assert_eq!(settings.jpeg_compression(), None);
        assert_eq!(settings.contrast(), None);
        assert_eq!(settings.saturation(), None);
        assert_eq!(settings.sharpness(), None);
        assert_eq!(settings.scene_capture_type(), None);
        assert_eq!(settings.custom_rendered(), None);
        assert_eq!(settings.gain_control(), None);
        assert_eq!(settings.picture_style(), None);
        assert_eq!(settings.dynamic_range_optimization(), None);
        assert_eq!(settings.high_dynamic_range(), None);
        assert_eq!(settings.high_iso_noise_reduction(), None);

        let control = PictureControl::default();
        assert_eq!(control.name(), None);
        assert_eq!(control.base(), None);
        assert_eq!(control.adjustment(), None);
        assert_eq!(control.quick_adjust(), None);
        assert_eq!(control.quick_sharp(), None);
        assert_eq!(control.sharpening(), None);
        assert_eq!(control.mid_range_sharpening(), None);
        assert_eq!(control.clarity(), None);
        assert_eq!(control.contrast(), None);
        assert_eq!(control.brightness(), None);
        assert_eq!(control.saturation(), None);
        assert_eq!(control.hue(), None);
        assert_eq!(control.filter_effect(), None);
        assert_eq!(control.toning(), None);
        assert_eq!(control.toning_saturation(), None);

        let white_balance = WhiteBalance::default();
        assert_eq!(white_balance.mode(), None);
        assert_eq!(white_balance.color_temperature(), None);
        assert_eq!(white_balance.fine_tune(), None);
        assert_eq!(white_balance.preset_slot(), None);
        assert_eq!(white_balance.fluorescent_lamp(), None);
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
                red_eye_reduction: true,
                return_light: Some(true),
                source: Some(FlashSource::External),
                control: Some(FlashControl::Ttl),
                sync: Some(FlashSync::Slow),
                compensation: Some(signed_fraction(-2, 6)),
            })
        );
        let flash = settings.flash().unwrap();
        assert!(flash.has_fired());
        assert_eq!(flash.mode(), Some(FlashMode::Fill));
        assert!(flash.red_eye_reduction());
        assert_eq!(flash.return_light(), Some(true));
        assert_eq!(flash.source(), Some(FlashSource::External));
        assert_eq!(flash.control(), Some(FlashControl::Ttl));
        assert_eq!(flash.sync(), Some(FlashSync::Slow));
        assert_eq!(flash.compensation(), Some(signed_fraction(-2, 6)));
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
        assert_eq!(
            settings.stabilization(),
            Some(Stabilization {
                on: true,
                mode: Some(StabilizationMode::Active),
            })
        );
        assert_eq!(settings.auto_iso(), Some(true));
        assert_eq!(settings.exposure_fine_tune(), Some(signed_fraction(-3, 6)));
        assert_eq!(settings.release_mode(), Some(ReleaseMode::Continuous));
        assert_eq!(settings.bracketing(), Some(Bracketing::Exposure));
        assert_eq!(settings.daylight_saving_time(), Some(true));
        assert_eq!(settings.raw_bit_depth(), Some(14));
        assert_eq!(settings.raw_compression(), Some(RawCompression::Lossless));
        assert_eq!(settings.jpeg_quality(), Some(JpegQuality::Fine));
        assert_eq!(
            settings.jpeg_compression(),
            Some(JpegCompression::OptimalQuality)
        );
        assert_eq!(settings.contrast(), Some(Contrast::Hard));
        assert_eq!(settings.saturation(), Some(Saturation::Low));
        assert_eq!(settings.sharpness(), Some(Sharpness::Soft));
        assert_eq!(
            settings.scene_capture_type(),
            Some(SceneCaptureType::Portrait)
        );
        assert_eq!(settings.custom_rendered(), Some(true));
        assert_eq!(settings.gain_control(), Some(GainControl::HighGainUp));
        assert_eq!(
            settings.picture_style(),
            Some(&PictureStyle::PictureControl(full_control()))
        );
        assert_eq!(
            settings.dynamic_range_optimization(),
            Some(DynamicRangeOptimization::ActiveDLighting(
                ActiveDLighting::High
            ))
        );
        assert_eq!(
            settings.high_dynamic_range(),
            Some(HighDynamicRange::Hdr(full_hdr()))
        );
        let hdr = full_hdr();
        assert!(hdr.is_on());
        assert_eq!(hdr.strength(), Some(HdrStrength::ExtraHigh));
        assert_eq!(
            hdr.exposure_differential(),
            Some(HdrExposureDifferential::Ev(Rational {
                numerator: 2,
                denominator: 1
            }))
        );
        assert_eq!(hdr.smoothing(), Some(HdrSmoothing::High));
        assert_eq!(
            settings.high_iso_noise_reduction(),
            Some(HighIsoNoiseReduction::MediumHigh)
        );

        let control = full_control();
        assert_eq!(control.name(), Some("VIVID-KR"));
        assert_eq!(control.base(), Some("VIVID"));
        assert_eq!(
            control.adjustment(),
            Some(PictureControlAdjustment::FullControl)
        );
        assert_eq!(control.quick_adjust(), Some(steps(1, 1)));
        assert_eq!(control.quick_sharp(), Some(steps(2, 1)));
        assert_eq!(control.sharpening(), Some(steps(20, 4)));
        assert_eq!(control.mid_range_sharpening(), Some(steps(14, 4)));
        assert_eq!(control.clarity(), Some(steps(8, 4)));
        assert_eq!(control.contrast(), Some(PictureControlValue::Auto));
        assert_eq!(control.brightness(), Some(steps(-4, 4)));
        assert_eq!(control.saturation(), Some(steps(3, 4)));
        assert_eq!(control.hue(), Some(steps(-1, 4)));
        assert_eq!(
            control.filter_effect(),
            Some(PictureControlFilterEffect::Red)
        );
        assert_eq!(control.toning(), Some(PictureControlToning::Sepia));
        assert_eq!(control.toning_saturation(), Some(steps(5, 4)));

        let white_balance = nikon_white_balance();
        assert_eq!(white_balance.mode(), Some(WhiteBalanceMode::Auto));
        assert_eq!(white_balance.color_temperature(), Some(5_200));
        let fine_tune = white_balance.fine_tune().unwrap();
        assert_eq!(fine_tune.amber_blue(), signed_fraction(-24, 4));
        assert_eq!(fine_tune.green_magenta(), signed_fraction(4, 4));
        assert_eq!(white_balance.preset_slot(), Some(2));
        assert_eq!(
            white_balance.fluorescent_lamp(),
            Some(FluorescentLamp::CoolWhite)
        );
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
            red_eye_reduction: false,
            return_light: Some(false),
            source: Some(FlashSource::BuiltIn),
            control: Some(FlashControl::Manual),
            sync: Some(FlashSync::RearCurtain),
            compensation: Some(signed_fraction(7, 10)),
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
                    start.clone().with_stabilization(Stabilization::off()),
                    CameraSettings {
                        stabilization: Some(Stabilization {
                            on: false,
                            mode: None,
                        }),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_auto_iso(false),
                    CameraSettings {
                        auto_iso: Some(false),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_exposure_fine_tune(signed_fraction(0, 6)),
                    CameraSettings {
                        exposure_fine_tune: Some(signed_fraction(0, 6)),
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
                (
                    start.clone().with_raw_bit_depth(12),
                    CameraSettings {
                        raw_bit_depth: Some(12),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_raw_compression(RawCompression::HighEfficiency),
                    CameraSettings {
                        raw_compression: Some(RawCompression::HighEfficiency),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_jpeg_quality(JpegQuality::Basic),
                    CameraSettings {
                        jpeg_quality: Some(JpegQuality::Basic),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_jpeg_compression(JpegCompression::SizePriority),
                    CameraSettings {
                        jpeg_compression: Some(JpegCompression::SizePriority),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_contrast(Contrast::Normal),
                    CameraSettings {
                        contrast: Some(Contrast::Normal),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_saturation(Saturation::High),
                    CameraSettings {
                        saturation: Some(Saturation::High),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_sharpness(Sharpness::Hard),
                    CameraSettings {
                        sharpness: Some(Sharpness::Hard),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_scene_capture_type(SceneCaptureType::Standard),
                    CameraSettings {
                        scene_capture_type: Some(SceneCaptureType::Standard),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_custom_rendered(false),
                    CameraSettings {
                        custom_rendered: Some(false),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_gain_control(GainControl::NoAdjustment),
                    CameraSettings {
                        gain_control: Some(GainControl::NoAdjustment),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_picture_style(PictureControl::default().into()),
                    CameraSettings {
                        picture_style: Some(
                            PictureStyle::PictureControl(PictureControl::default()),
                        ),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_dynamic_range_optimization(ActiveDLighting::Off.into()),
                    CameraSettings {
                        dynamic_range_optimization: Some(
                            DynamicRangeOptimization::ActiveDLighting(ActiveDLighting::Off),
                        ),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_high_dynamic_range(Hdr::off().into()),
                    CameraSettings {
                        high_dynamic_range: Some(HighDynamicRange::Hdr(Hdr::off())),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_high_iso_noise_reduction(HighIsoNoiseReduction::Auto),
                    CameraSettings {
                        high_iso_noise_reduction: Some(HighIsoNoiseReduction::Auto),
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
        let fine_tune = WhiteBalanceFineTune {
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
                (
                    start.clone().with_preset_slot(0),
                    WhiteBalance {
                        preset_slot: Some(0),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_fluorescent_lamp(FluorescentLamp::WarmWhite),
                    WhiteBalance {
                        fluorescent_lamp: Some(FluorescentLamp::WarmWhite),
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
    fn each_picture_control_builder_sets_its_own_value_and_no_other() {
        let zero = steps(0, 1);
        for start in [PictureControl::default(), full_control()] {
            let cases = [
                (
                    start.clone().with_name("STANDARD"),
                    PictureControl {
                        name: Some("STANDARD".into()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_base("STANDARD"),
                    PictureControl {
                        base: Some("STANDARD".into()),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_adjustment(PictureControlAdjustment::QuickAdjust),
                    PictureControl {
                        adjustment: Some(PictureControlAdjustment::QuickAdjust),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_quick_adjust(zero),
                    PictureControl {
                        quick_adjust: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_quick_sharp(zero),
                    PictureControl {
                        quick_sharp: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_sharpening(zero),
                    PictureControl {
                        sharpening: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_mid_range_sharpening(zero),
                    PictureControl {
                        mid_range_sharpening: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_clarity(zero),
                    PictureControl {
                        clarity: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_contrast(zero),
                    PictureControl {
                        contrast: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_brightness(zero),
                    PictureControl {
                        brightness: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_saturation(zero),
                    PictureControl {
                        saturation: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_hue(zero),
                    PictureControl {
                        hue: Some(zero),
                        ..start.clone()
                    },
                ),
                (
                    start
                        .clone()
                        .with_filter_effect(PictureControlFilterEffect::Green),
                    PictureControl {
                        filter_effect: Some(PictureControlFilterEffect::Green),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_toning(PictureControlToning::RedPurple),
                    PictureControl {
                        toning: Some(PictureControlToning::RedPurple),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_toning_saturation(zero),
                    PictureControl {
                        toning_saturation: Some(zero),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
    }

    /// A Creative Style with every part set, each value different.
    fn full_creative_style() -> CreativeStyle {
        CreativeStyle {
            name: Some("Vivid".into()),
            contrast: Some(-3),
            saturation: Some(2),
            sharpness: Some(1),
        }
    }

    /// A Creative Look with every part set, each value different.
    fn full_creative_look() -> CreativeLook {
        CreativeLook {
            name: Some("Standard".into()),
            contrast: Some(1),
            saturation: Some(-3),
            sharpness: Some(4),
        }
    }

    #[test]
    fn a_picture_style_gives_its_makers_word_and_its_name() {
        let cases = [
            (
                PictureStyle::from(full_control()),
                PictureStyle::PictureControl(full_control()),
                "Picture Control",
                "VIVID-KR",
            ),
            (
                PictureStyle::from(full_creative_style()),
                PictureStyle::CreativeStyle(full_creative_style()),
                "Creative Style",
                "Vivid",
            ),
            (
                PictureStyle::from(full_creative_look()),
                PictureStyle::CreativeLook(full_creative_look()),
                "Creative Look",
                "Standard",
            ),
        ];
        for (style, variant, setting, name) in cases {
            assert_eq!(style, variant);
            assert_eq!(style.setting_name(), setting);
            assert_eq!(style.name(), Some(name));
        }
        for unnamed in [
            PictureStyle::from(PictureControl::default()),
            PictureStyle::from(CreativeStyle::default()),
            PictureStyle::from(CreativeLook::default()),
        ] {
            assert_eq!(unnamed.name(), None, "{unnamed:?}");
        }
    }

    #[test]
    fn a_dynamic_range_optimization_gives_its_makers_word_and_its_step() {
        for step in all_active_d_lightings() {
            let optimization = DynamicRangeOptimization::from(step);
            assert_eq!(
                optimization,
                DynamicRangeOptimization::ActiveDLighting(step)
            );
            assert_eq!(optimization.setting_name(), "Active D-Lighting");
            assert_eq!(optimization.name(), step.name());
        }
        for step in all_d_range_optimizers() {
            let optimization = DynamicRangeOptimization::from(step);
            assert_eq!(
                optimization,
                DynamicRangeOptimization::DRangeOptimizer(step)
            );
            assert_eq!(optimization.setting_name(), "D-Range Optimizer");
            assert_eq!(optimization.name(), step.name());
        }
    }

    #[test]
    fn on_and_off_build_an_hdr_that_says_so_with_no_parts() {
        for (hdr, on) in [(Hdr::on(), true), (Hdr::off(), false)] {
            assert_eq!(hdr.is_on(), on);
            assert_eq!(hdr.strength(), None);
            assert_eq!(hdr.exposure_differential(), None);
            assert_eq!(hdr.smoothing(), None);
        }
        // A part is stored as given, even on an HDR that was off.
        let off = Hdr::off().with_strength(HdrStrength::Normal);
        assert!(!off.is_on());
        assert_eq!(off.strength(), Some(HdrStrength::Normal));
    }

    #[test]
    fn on_and_off_build_an_auto_hdr_that_says_so_with_no_difference() {
        for (auto_hdr, on) in [(AutoHdr::on(), true), (AutoHdr::off(), false)] {
            assert_eq!(auto_hdr.is_on(), on);
            assert_eq!(auto_hdr.exposure_difference(), None);
        }
        // The difference is stored as given, even on an Auto HDR that was
        // off, and a later one replaces an earlier one.
        let halves = AutoHdrExposureDifference::Ev(Rational {
            numerator: 5,
            denominator: 2,
        });
        let off = AutoHdr::off()
            .with_exposure_difference(AutoHdrExposureDifference::Auto)
            .with_exposure_difference(halves);
        assert!(!off.is_on());
        assert_eq!(off.exposure_difference(), Some(halves));
    }

    #[test]
    fn a_high_dynamic_range_gives_its_makers_word_and_whether_it_was_on() {
        for hdr in [Hdr::on(), Hdr::off(), full_hdr()] {
            let setting = HighDynamicRange::from(hdr);
            assert_eq!(setting, HighDynamicRange::Hdr(hdr));
            assert_eq!(setting.setting_name(), "HDR");
            assert_eq!(setting.is_on(), hdr.is_on());
        }
        let auto = AutoHdr::on().with_exposure_difference(AutoHdrExposureDifference::Auto);
        for auto_hdr in [AutoHdr::on(), AutoHdr::off(), auto] {
            let setting = HighDynamicRange::from(auto_hdr);
            assert_eq!(setting, HighDynamicRange::AutoHdr(auto_hdr));
            assert_eq!(setting.setting_name(), "Auto HDR");
            assert_eq!(setting.is_on(), auto_hdr.is_on());
        }
    }

    #[test]
    fn a_creative_style_and_a_creative_look_give_what_was_set() {
        let style = full_creative_style();
        assert_eq!(style.name(), Some("Vivid"));
        assert_eq!(style.contrast(), Some(-3));
        assert_eq!(style.saturation(), Some(2));
        assert_eq!(style.sharpness(), Some(1));
        let look = full_creative_look();
        assert_eq!(look.name(), Some("Standard"));
        assert_eq!(look.contrast(), Some(1));
        assert_eq!(look.saturation(), Some(-3));
        assert_eq!(look.sharpness(), Some(4));
        let empty = CreativeStyle::default();
        assert_eq!(
            (
                empty.name(),
                empty.contrast(),
                empty.saturation(),
                empty.sharpness()
            ),
            (None, None, None, None)
        );
        let empty = CreativeLook::default();
        assert_eq!(
            (
                empty.name(),
                empty.contrast(),
                empty.saturation(),
                empty.sharpness()
            ),
            (None, None, None, None)
        );
    }

    #[test]
    fn each_creative_style_and_look_builder_sets_its_own_value_and_no_other() {
        for start in [CreativeStyle::default(), full_creative_style()] {
            let cases = [
                (
                    start.clone().with_name("Neutral"),
                    CreativeStyle {
                        name: Some("Neutral".into()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_contrast(0),
                    CreativeStyle {
                        contrast: Some(0),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_saturation(0),
                    CreativeStyle {
                        saturation: Some(0),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_sharpness(0),
                    CreativeStyle {
                        sharpness: Some(0),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
        for start in [CreativeLook::default(), full_creative_look()] {
            let cases = [
                (
                    start.clone().with_name("Off"),
                    CreativeLook {
                        name: Some("Off".into()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_contrast(0),
                    CreativeLook {
                        contrast: Some(0),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_saturation(0),
                    CreativeLook {
                        saturation: Some(0),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_sharpness(0),
                    CreativeLook {
                        sharpness: Some(0),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
        // A chain equals the value built field by field, and a later value
        // replaces an earlier one.
        let style = CreativeStyle::default()
            .with_name("BW")
            .with_contrast(9)
            .with_saturation(9)
            .with_sharpness(9)
            .with_name("Vivid")
            .with_contrast(-3)
            .with_saturation(2)
            .with_sharpness(1);
        assert_eq!(style, full_creative_style());
        let look = CreativeLook::default()
            .with_name("Off")
            .with_contrast(9)
            .with_saturation(9)
            .with_sharpness(9)
            .with_name("Standard")
            .with_contrast(1)
            .with_saturation(-3)
            .with_sharpness(4);
        assert_eq!(look, full_creative_look());
    }

    #[test]
    fn every_builder_in_a_chain_equals_the_value_built_field_by_field() {
        let settings = CameraSettings::default()
            .with_white_balance(
                WhiteBalance::default()
                    .with_mode(WhiteBalanceMode::Auto)
                    .with_color_temperature(5_200)
                    .with_fine_tune(WhiteBalanceFineTune::new(
                        signed_fraction(-24, 4),
                        signed_fraction(4, 4),
                    ))
                    .with_preset_slot(2)
                    .with_fluorescent_lamp(FluorescentLamp::CoolWhite),
            )
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(
                Flash::fired()
                    .with_mode(FlashMode::Fill)
                    .with_red_eye_reduction(true)
                    .with_return_light(true)
                    .with_source(FlashSource::External)
                    .with_control(FlashControl::Ttl)
                    .with_sync(FlashSync::Slow)
                    .with_compensation(signed_fraction(-2, 6)),
            )
            .with_shutter(Shutter::ElectronicFrontCurtain)
            .with_electronic_front_curtain_setting(true)
            .with_image_area(ImageArea::FullFrame)
            .with_raw_image_size(RawImageSize::Large)
            .with_color_space(ColorSpace::AdobeRgb)
            .with_focus_mode(FocusMode::Continuous)
            .with_focus_area_mode(FocusAreaMode::Tracking)
            .with_af_fine_tune(AfFineTune::on().with_value(-5))
            .with_stabilization(Stabilization::on().with_mode(StabilizationMode::Active))
            .with_auto_iso(true)
            .with_exposure_fine_tune(signed_fraction(-3, 6))
            .with_release_mode(ReleaseMode::Continuous)
            .with_bracketing(Bracketing::Exposure)
            .with_daylight_saving_time(true)
            .with_raw_bit_depth(14)
            .with_raw_compression(RawCompression::Lossless)
            .with_jpeg_quality(JpegQuality::Fine)
            .with_jpeg_compression(JpegCompression::OptimalQuality)
            .with_contrast(Contrast::Hard)
            .with_saturation(Saturation::Low)
            .with_sharpness(Sharpness::Soft)
            .with_scene_capture_type(SceneCaptureType::Portrait)
            .with_custom_rendered(true)
            .with_gain_control(GainControl::HighGainUp)
            .with_picture_style(PictureStyle::from(
                PictureControl::default()
                    .with_name("VIVID-KR")
                    .with_base("VIVID")
                    .with_adjustment(PictureControlAdjustment::FullControl)
                    .with_quick_adjust(steps(1, 1))
                    .with_quick_sharp(steps(2, 1))
                    .with_sharpening(steps(20, 4))
                    .with_mid_range_sharpening(steps(14, 4))
                    .with_clarity(steps(8, 4))
                    .with_contrast(PictureControlValue::Auto)
                    .with_brightness(steps(-4, 4))
                    .with_saturation(steps(3, 4))
                    .with_hue(steps(-1, 4))
                    .with_filter_effect(PictureControlFilterEffect::Red)
                    .with_toning(PictureControlToning::Sepia)
                    .with_toning_saturation(steps(5, 4)),
            ))
            .with_dynamic_range_optimization(DynamicRangeOptimization::from(ActiveDLighting::High))
            .with_high_dynamic_range(HighDynamicRange::from(
                Hdr::on()
                    .with_strength(HdrStrength::ExtraHigh)
                    .with_exposure_differential(HdrExposureDifferential::Ev(Rational {
                        numerator: 2,
                        denominator: 1,
                    }))
                    .with_smoothing(HdrSmoothing::High),
            ))
            .with_high_iso_noise_reduction(HighIsoNoiseReduction::MediumHigh);
        assert_eq!(settings, full_settings());
    }

    #[test]
    fn a_later_value_replaces_an_earlier_one() {
        let earlier = CameraSettings::default()
            .with_white_balance(WhiteBalance::default().with_color_temperature(1))
            .with_exposure_program(ExposureProgram::Manual)
            .with_metering_mode(MeteringMode::Average)
            .with_flash(Flash::not_fired())
            .with_shutter(Shutter::Mechanical)
            .with_electronic_front_curtain_setting(false)
            .with_image_area(ImageArea::Crop2_7x)
            .with_raw_image_size(RawImageSize::Medium)
            .with_color_space(ColorSpace::Srgb)
            .with_focus_mode(FocusMode::Single)
            .with_focus_area_mode(FocusAreaMode::Pinpoint)
            .with_af_fine_tune(AfFineTune::off().with_value(3))
            .with_stabilization(Stabilization::off().with_mode(StabilizationMode::Sport))
            .with_auto_iso(false)
            .with_exposure_fine_tune(signed_fraction(1, 6))
            .with_release_mode(ReleaseMode::SelfTimer)
            .with_bracketing(Bracketing::DynamicRange)
            .with_daylight_saving_time(false)
            .with_raw_bit_depth(12)
            .with_raw_compression(RawCompression::Lossy)
            .with_jpeg_quality(JpegQuality::Light)
            .with_jpeg_compression(JpegCompression::SizePriority)
            .with_contrast(Contrast::Soft)
            .with_saturation(Saturation::High)
            .with_sharpness(Sharpness::Hard)
            .with_scene_capture_type(SceneCaptureType::NightScene)
            .with_custom_rendered(false)
            .with_gain_control(GainControl::LowGainDown)
            .with_picture_style(PictureControl::default().with_name("FLAT").into())
            .with_dynamic_range_optimization(ActiveDLighting::Low.into())
            .with_high_dynamic_range(Hdr::off().into())
            .with_high_iso_noise_reduction(HighIsoNoiseReduction::Off);
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
            .with_stabilization(Stabilization::on().with_mode(StabilizationMode::Active))
            .with_auto_iso(true)
            .with_exposure_fine_tune(signed_fraction(-3, 6))
            .with_release_mode(ReleaseMode::Continuous)
            .with_bracketing(Bracketing::Exposure)
            .with_daylight_saving_time(true)
            .with_raw_bit_depth(14)
            .with_raw_compression(RawCompression::Lossless)
            .with_jpeg_quality(JpegQuality::Fine)
            .with_jpeg_compression(JpegCompression::OptimalQuality)
            .with_contrast(Contrast::Hard)
            .with_saturation(Saturation::Low)
            .with_sharpness(Sharpness::Soft)
            .with_scene_capture_type(SceneCaptureType::Portrait)
            .with_custom_rendered(true)
            .with_gain_control(GainControl::HighGainUp)
            .with_picture_style(full_control().into())
            .with_dynamic_range_optimization(ActiveDLighting::High.into())
            .with_high_dynamic_range(full_hdr().into())
            .with_high_iso_noise_reduction(HighIsoNoiseReduction::MediumHigh);
        assert_eq!(settings, full);

        let hdr = Hdr::on()
            .with_strength(HdrStrength::Low)
            .with_exposure_differential(HdrExposureDifferential::Auto)
            .with_smoothing(HdrSmoothing::Off)
            .with_strength(HdrStrength::ExtraHigh)
            .with_exposure_differential(HdrExposureDifferential::Ev(Rational {
                numerator: 2,
                denominator: 1,
            }))
            .with_smoothing(HdrSmoothing::High);
        assert_eq!(hdr, full_hdr());

        let control = PictureControl::default()
            .with_name("FLAT")
            .with_base("FLAT")
            .with_adjustment(PictureControlAdjustment::Default)
            .with_quick_adjust(steps(-1, 1))
            .with_quick_sharp(steps(-2, 1))
            .with_sharpening(PictureControlValue::Auto)
            .with_mid_range_sharpening(steps(0, 4))
            .with_clarity(PictureControlValue::Auto)
            .with_contrast(steps(0, 4))
            .with_brightness(steps(0, 4))
            .with_saturation(PictureControlValue::Auto)
            .with_hue(steps(0, 4))
            .with_filter_effect(PictureControlFilterEffect::Off)
            .with_toning(PictureControlToning::BlackAndWhite)
            .with_toning_saturation(steps(0, 4))
            .with_name("VIVID-KR")
            .with_base("VIVID")
            .with_adjustment(PictureControlAdjustment::FullControl)
            .with_quick_adjust(steps(1, 1))
            .with_quick_sharp(steps(2, 1))
            .with_sharpening(steps(20, 4))
            .with_mid_range_sharpening(steps(14, 4))
            .with_clarity(steps(8, 4))
            .with_contrast(PictureControlValue::Auto)
            .with_brightness(steps(-4, 4))
            .with_saturation(steps(3, 4))
            .with_hue(steps(-1, 4))
            .with_filter_effect(PictureControlFilterEffect::Red)
            .with_toning(PictureControlToning::Sepia)
            .with_toning_saturation(steps(5, 4));
        assert_eq!(control, full_control());

        let white_balance = WhiteBalance::default()
            .with_mode(WhiteBalanceMode::Daylight)
            .with_color_temperature(1)
            .with_fine_tune(WhiteBalanceFineTune::new(
                signed_fraction(1, 1),
                signed_fraction(1, 1),
            ))
            .with_preset_slot(4)
            .with_fluorescent_lamp(FluorescentLamp::Daylight)
            .with_mode(WhiteBalanceMode::Auto)
            .with_color_temperature(5_200)
            .with_fine_tune(WhiteBalanceFineTune::new(
                signed_fraction(-24, 4),
                signed_fraction(4, 4),
            ))
            .with_preset_slot(2)
            .with_fluorescent_lamp(FluorescentLamp::CoolWhite);
        assert_eq!(white_balance, nikon_white_balance());
    }

    #[test]
    fn a_fine_tune_keeps_both_shifts_as_given() {
        // Two different shifts, so that one given for the other is caught.
        let fine_tune = WhiteBalanceFineTune::new(signed_fraction(-6, 1), signed_fraction(2, 4));
        assert_eq!(fine_tune.amber_blue(), signed_fraction(-6, 1));
        assert_eq!(fine_tune.green_magenta(), signed_fraction(2, 4));
        // As stored: -24/4 is not -6/1, and a zero shift is a value.
        assert_ne!(
            WhiteBalanceFineTune::new(signed_fraction(-24, 4), signed_fraction(2, 4)),
            fine_tune
        );
        let zero = WhiteBalanceFineTune::new(signed_fraction(0, 4), signed_fraction(0, 4));
        assert_eq!(zero.amber_blue(), signed_fraction(0, 4));
        assert_eq!(zero.green_magenta(), signed_fraction(0, 4));
    }

    #[test]
    fn fired_and_not_fired_build_a_flash_that_says_so() {
        assert!(Flash::fired().has_fired());
        assert!(!Flash::not_fired().has_fired());
    }

    #[test]
    fn a_flash_starts_without_a_mode_and_takes_one() {
        for (flash, fired) in [(Flash::not_fired(), false), (Flash::fired(), true)] {
            assert_eq!(flash.has_fired(), fired);
            assert_eq!(flash.mode(), None);
            let auto = flash.with_mode(FlashMode::Auto);
            assert_eq!(auto.has_fired(), fired);
            assert_eq!(auto.mode(), Some(FlashMode::Auto));
            // A second mode replaces the first.
            let off = auto.with_mode(FlashMode::Off);
            assert_eq!(off.has_fired(), fired);
            assert_eq!(off.mode(), Some(FlashMode::Off));
            // Nothing else was set.
            assert!(!off.red_eye_reduction());
            assert_eq!(off.return_light(), None);
            assert_eq!(off.source(), None);
            assert_eq!(off.control(), None);
            assert_eq!(off.sync(), None);
            assert_eq!(off.compensation(), None);
        }
    }

    #[test]
    fn a_flash_takes_its_compensation_as_given() {
        let flash = Flash::fired().with_sync(FlashSync::RearCurtain);
        // Set, then set again, which replaces it; each kept as given, so
        // -10/10 is not -1/1, and a zero is a value.
        let minus = flash.with_compensation(signed_fraction(-10, 10));
        assert_eq!(minus.compensation(), Some(signed_fraction(-10, 10)));
        assert_ne!(minus.compensation(), Some(signed_fraction(-1, 1)));
        let zero = minus.with_compensation(signed_fraction(0, 6));
        assert_eq!(zero.compensation(), Some(signed_fraction(0, 6)));
        // The other parts stay as they were.
        assert!(zero.has_fired());
        assert_eq!(zero.sync(), Some(FlashSync::RearCurtain));
        assert_eq!(zero.mode(), None);
    }

    #[test]
    fn a_flash_takes_its_sync() {
        let flash = Flash::not_fired().with_source(FlashSource::BuiltIn);
        // Set, then set again, which replaces it.
        let rear = flash.with_sync(FlashSync::RearCurtain);
        assert_eq!(rear.sync(), Some(FlashSync::RearCurtain));
        let slow = rear.with_sync(FlashSync::SlowRearCurtain);
        assert_eq!(slow.sync(), Some(FlashSync::SlowRearCurtain));
        // The other parts stay as they were.
        assert!(!slow.has_fired());
        assert_eq!(slow.source(), Some(FlashSource::BuiltIn));
        assert_eq!(slow.control(), None);
    }

    #[test]
    fn a_flash_takes_its_source_and_control() {
        let flash = Flash::fired().with_mode(FlashMode::Auto);
        // Each set, then set again, which replaces it.
        let built_in = flash.with_source(FlashSource::BuiltIn);
        assert_eq!(built_in.source(), Some(FlashSource::BuiltIn));
        let unit = built_in.with_source(FlashSource::External);
        assert_eq!(unit.source(), Some(FlashSource::External));
        let manual = unit.with_control(FlashControl::Manual);
        assert_eq!(manual.control(), Some(FlashControl::Manual));
        let repeating = manual.with_control(FlashControl::Repeating);
        assert_eq!(repeating.control(), Some(FlashControl::Repeating));
        // The other parts stay as they were.
        assert!(repeating.has_fired());
        assert_eq!(repeating.mode(), Some(FlashMode::Auto));
        assert_eq!(repeating.source(), Some(FlashSource::External));
        assert_eq!(repeating.return_light(), None);
    }

    #[test]
    fn a_flash_takes_red_eye_reduction_and_its_return_light() {
        for (flash, fired) in [(Flash::not_fired(), false), (Flash::fired(), true)] {
            assert!(!flash.red_eye_reduction());
            assert_eq!(flash.return_light(), None);
            // Each set, then set again the other way, which replaces it.
            let on = flash.with_red_eye_reduction(true);
            assert!(on.red_eye_reduction());
            assert!(!on.with_red_eye_reduction(false).red_eye_reduction());
            let detected = on.with_return_light(true);
            assert_eq!(detected.return_light(), Some(true));
            let missed = detected.with_return_light(false);
            assert_eq!(missed.return_light(), Some(false));
            // The other parts stay as they were.
            assert_eq!(missed.has_fired(), fired);
            assert_eq!(missed.mode(), None);
            assert!(missed.red_eye_reduction());
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

    #[test]
    fn on_and_off_build_a_stabilization_that_says_so() {
        assert!(Stabilization::on().is_on());
        assert!(!Stabilization::off().is_on());
    }

    #[test]
    fn a_stabilization_starts_without_a_mode_and_takes_one() {
        for (stabilization, on) in [(Stabilization::off(), false), (Stabilization::on(), true)] {
            assert_eq!(stabilization.is_on(), on);
            assert_eq!(stabilization.mode(), None);
            // A mode, then a second that replaces it, each kept as given,
            // whether on or off.
            let normal = stabilization.with_mode(StabilizationMode::Normal);
            assert_eq!(
                (normal.is_on(), normal.mode()),
                (on, Some(StabilizationMode::Normal))
            );
            let sport = normal.with_mode(StabilizationMode::Sport);
            assert_eq!(
                (sport.is_on(), sport.mode()),
                (on, Some(StabilizationMode::Sport))
            );
        }
        assert_ne!(Stabilization::on(), Stabilization::off());
        assert_ne!(
            Stabilization::on(),
            Stabilization::on().with_mode(StabilizationMode::Normal)
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

    fn all_flash_sources() -> [FlashSource; 2] {
        let all = [FlashSource::BuiltIn, FlashSource::External];
        for source in all {
            match source {
                FlashSource::BuiltIn | FlashSource::External => {}
            }
        }
        all
    }

    fn all_flash_controls() -> [FlashControl; 4] {
        use FlashControl::*;
        let all = [Ttl, AutoAperture, Manual, Repeating];
        for control in all {
            match control {
                Ttl | AutoAperture | Manual | Repeating => {}
            }
        }
        all
    }

    fn all_flash_syncs() -> [FlashSync; 4] {
        use FlashSync::*;
        let all = [FrontCurtain, RearCurtain, Slow, SlowRearCurtain];
        for sync in all {
            match sync {
                FrontCurtain | RearCurtain | Slow | SlowRearCurtain => {}
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

    fn all_stabilization_modes() -> [StabilizationMode; 3] {
        use StabilizationMode::*;
        let all = [Normal, Active, Sport];
        for mode in all {
            match mode {
                Normal | Active | Sport => {}
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

    fn all_raw_compressions() -> [RawCompression; 5] {
        use RawCompression::*;
        let all = [
            Uncompressed,
            Lossless,
            Lossy,
            HighEfficiency,
            HighEfficiencyStar,
        ];
        for compression in all {
            match compression {
                Uncompressed | Lossless | Lossy | HighEfficiency | HighEfficiencyStar => {}
            }
        }
        all
    }

    fn all_jpeg_qualities() -> [JpegQuality; 8] {
        use JpegQuality::*;
        let all = [
            Economy, Light, Basic, Standard, Normal, Fine, SuperFine, ExtraFine,
        ];
        for quality in all {
            match quality {
                Economy | Light | Basic | Standard | Normal | Fine | SuperFine | ExtraFine => {}
            }
        }
        all
    }

    fn all_jpeg_compressions() -> [JpegCompression; 2] {
        use JpegCompression::*;
        let all = [SizePriority, OptimalQuality];
        for compression in all {
            match compression {
                SizePriority | OptimalQuality => {}
            }
        }
        all
    }

    fn all_fluorescent_lamps() -> [FluorescentLamp; 5] {
        use FluorescentLamp::*;
        let all = [Daylight, DayWhite, CoolWhite, White, WarmWhite];
        for lamp in all {
            match lamp {
                Daylight | DayWhite | CoolWhite | White | WarmWhite => {}
            }
        }
        all
    }

    fn all_contrasts() -> [Contrast; 3] {
        use Contrast::*;
        let all = [Normal, Soft, Hard];
        for contrast in all {
            match contrast {
                Normal | Soft | Hard => {}
            }
        }
        all
    }

    fn all_saturations() -> [Saturation; 3] {
        use Saturation::*;
        let all = [Normal, Low, High];
        for saturation in all {
            match saturation {
                Normal | Low | High => {}
            }
        }
        all
    }

    fn all_sharpnesses() -> [Sharpness; 3] {
        use Sharpness::*;
        let all = [Normal, Soft, Hard];
        for sharpness in all {
            match sharpness {
                Normal | Soft | Hard => {}
            }
        }
        all
    }

    fn all_scene_capture_types() -> [SceneCaptureType; 4] {
        use SceneCaptureType::*;
        let all = [Standard, Landscape, Portrait, NightScene];
        for scene in all {
            match scene {
                Standard | Landscape | Portrait | NightScene => {}
            }
        }
        all
    }

    fn all_picture_control_adjustments() -> [PictureControlAdjustment; 3] {
        use PictureControlAdjustment::*;
        let all = [Default, QuickAdjust, FullControl];
        for adjustment in all {
            match adjustment {
                Default | QuickAdjust | FullControl => {}
            }
        }
        all
    }

    fn all_picture_control_filter_effects() -> [PictureControlFilterEffect; 5] {
        use PictureControlFilterEffect::*;
        let all = [Off, Yellow, Orange, Red, Green];
        for effect in all {
            match effect {
                Off | Yellow | Orange | Red | Green => {}
            }
        }
        all
    }

    fn all_picture_control_tonings() -> [PictureControlToning; 10] {
        use PictureControlToning::*;
        let all = [
            BlackAndWhite,
            Sepia,
            Cyanotype,
            Red,
            Yellow,
            Green,
            BlueGreen,
            Blue,
            PurpleBlue,
            RedPurple,
        ];
        for toning in all {
            match toning {
                BlackAndWhite | Sepia | Cyanotype | Red | Yellow | Green | BlueGreen | Blue
                | PurpleBlue | RedPurple => {}
            }
        }
        all
    }

    fn all_gain_controls() -> [GainControl; 5] {
        use GainControl::*;
        let all = [
            NoAdjustment,
            LowGainUp,
            HighGainUp,
            LowGainDown,
            HighGainDown,
        ];
        for gain in all {
            match gain {
                NoAdjustment | LowGainUp | HighGainUp | LowGainDown | HighGainDown => {}
            }
        }
        all
    }

    fn all_active_d_lightings() -> [ActiveDLighting; 10] {
        use ActiveDLighting::*;
        let all = [
            Off, Low, Normal, High, ExtraHigh, ExtraHigh1, ExtraHigh2, ExtraHigh3, ExtraHigh4, Auto,
        ];
        for step in all {
            match step {
                Off | Low | Normal | High | ExtraHigh | ExtraHigh1 | ExtraHigh2 | ExtraHigh3
                | ExtraHigh4 | Auto => {}
            }
        }
        all
    }

    fn all_d_range_optimizers() -> [DRangeOptimizer; 17] {
        use DRangeOptimizer::*;
        let all = [
            Off,
            Standard,
            AdvancedAuto,
            Auto,
            AdvancedLevel1,
            AdvancedLevel2,
            AdvancedLevel3,
            AdvancedLevel4,
            AdvancedLevel5,
            Level1,
            Level2,
            Level3,
            Level4,
            Level5,
            Level6,
            Level7,
            Level8,
        ];
        for step in all {
            match step {
                Off | Standard | AdvancedAuto | Auto | AdvancedLevel1 | AdvancedLevel2
                | AdvancedLevel3 | AdvancedLevel4 | AdvancedLevel5 | Level1 | Level2 | Level3
                | Level4 | Level5 | Level6 | Level7 | Level8 => {}
            }
        }
        all
    }

    fn all_hdr_strengths() -> [HdrStrength; 5] {
        use HdrStrength::*;
        let all = [Low, Normal, High, ExtraHigh, Auto];
        for step in all {
            match step {
                Low | Normal | High | ExtraHigh | Auto => {}
            }
        }
        all
    }

    fn all_hdr_smoothings() -> [HdrSmoothing; 5] {
        use HdrSmoothing::*;
        let all = [Off, Low, Normal, High, Auto];
        for step in all {
            match step {
                Off | Low | Normal | High | Auto => {}
            }
        }
        all
    }

    fn all_high_iso_noise_reductions() -> [HighIsoNoiseReduction; 8] {
        use HighIsoNoiseReduction::*;
        let all = [Off, Minimal, Low, MediumLow, Normal, MediumHigh, High, Auto];
        for step in all {
            match step {
                Off | Minimal | Low | MediumLow | Normal | MediumHigh | High | Auto => {}
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
            &all_flash_sources(),
            FlashSource::name,
            &["Built-in", "External"],
        );
        assert_names(
            &all_flash_controls(),
            FlashControl::name,
            &["TTL", "Auto aperture", "Manual", "Repeating"],
        );
        assert_names(
            &all_flash_syncs(),
            FlashSync::name,
            &[
                "Front curtain",
                "Rear curtain",
                "Slow sync",
                "Slow rear-curtain sync",
            ],
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
            &all_stabilization_modes(),
            StabilizationMode::name,
            &["Normal", "Active", "Sport"],
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
        assert_names(
            &all_raw_compressions(),
            RawCompression::name,
            &[
                "Uncompressed",
                "Lossless compressed",
                "Lossy compressed",
                "High efficiency",
                "High efficiency*",
            ],
        );
        assert_names(
            &all_jpeg_qualities(),
            JpegQuality::name,
            &[
                "Economy",
                "Light",
                "Basic",
                "Standard",
                "Normal",
                "Fine",
                "Super fine",
                "Extra fine",
            ],
        );
        assert_names(
            &all_jpeg_compressions(),
            JpegCompression::name,
            &["Size priority", "Optimal quality"],
        );
        assert_names(
            &all_fluorescent_lamps(),
            FluorescentLamp::name,
            &[
                "Daylight fluorescent",
                "Day white fluorescent",
                "Cool white fluorescent",
                "White fluorescent",
                "Warm white fluorescent",
            ],
        );
        assert_names(
            &all_contrasts(),
            Contrast::name,
            &["Normal", "Soft", "Hard"],
        );
        assert_names(
            &all_saturations(),
            Saturation::name,
            &["Normal", "Low", "High"],
        );
        assert_names(
            &all_sharpnesses(),
            Sharpness::name,
            &["Normal", "Soft", "Hard"],
        );
        assert_names(
            &all_scene_capture_types(),
            SceneCaptureType::name,
            &["Standard", "Landscape", "Portrait", "Night scene"],
        );
        assert_names(
            &all_gain_controls(),
            GainControl::name,
            &[
                "None",
                "Low gain up",
                "High gain up",
                "Low gain down",
                "High gain down",
            ],
        );
        assert_names(
            &all_picture_control_adjustments(),
            PictureControlAdjustment::name,
            &["Default settings", "Quick adjust", "Full control"],
        );
        assert_names(
            &all_picture_control_filter_effects(),
            PictureControlFilterEffect::name,
            &["Off", "Yellow", "Orange", "Red", "Green"],
        );
        assert_names(
            &all_picture_control_tonings(),
            PictureControlToning::name,
            &[
                "Black-and-white",
                "Sepia",
                "Cyanotype",
                "Red",
                "Yellow",
                "Green",
                "Blue green",
                "Blue",
                "Purple blue",
                "Red purple",
            ],
        );
        assert_names(
            &all_active_d_lightings(),
            ActiveDLighting::name,
            &[
                "Off",
                "Low",
                "Normal",
                "High",
                "Extra high",
                "Extra high 1",
                "Extra high 2",
                "Extra high 3",
                "Extra high 4",
                "Auto",
            ],
        );
        assert_names(
            &all_d_range_optimizers(),
            DRangeOptimizer::name,
            &[
                "Off",
                "Standard",
                "Advanced auto",
                "Auto",
                "Advanced level 1",
                "Advanced level 2",
                "Advanced level 3",
                "Advanced level 4",
                "Advanced level 5",
                "Level 1",
                "Level 2",
                "Level 3",
                "Level 4",
                "Level 5",
                "Level 6",
                "Level 7",
                "Level 8",
            ],
        );
        assert_names(
            &all_hdr_strengths(),
            HdrStrength::name,
            &["Low", "Normal", "High", "Extra high", "Auto"],
        );
        assert_names(
            &all_hdr_smoothings(),
            HdrSmoothing::name,
            &["Off", "Low", "Normal", "High", "Auto"],
        );
        assert_names(
            &all_high_iso_noise_reductions(),
            HighIsoNoiseReduction::name,
            &[
                "Off",
                "Minimal",
                "Low",
                "Medium low",
                "Normal",
                "Medium high",
                "High",
                "Auto",
            ],
        );
    }

    #[test]
    fn every_enum_orders_as_its_variants_are_listed() {
        // A program may key a sorted map by these enums, and the order is
        // a promise: a variant moved, or added before the others, fails.
        assert_ordered(&all_flash_modes());
        assert_ordered(&all_flash_sources());
        assert_ordered(&all_flash_controls());
        assert_ordered(&all_flash_syncs());
        assert_ordered(&all_exposure_programs());
        assert_ordered(&all_metering_modes());
        assert_ordered(&all_white_balance_modes());
        assert_ordered(&all_shutters());
        assert_ordered(&all_image_areas());
        assert_ordered(&all_raw_image_sizes());
        assert_ordered(&all_color_spaces());
        assert_ordered(&all_focus_modes());
        assert_ordered(&all_focus_area_modes());
        assert_ordered(&all_stabilization_modes());
        assert_ordered(&all_release_modes());
        assert_ordered(&all_bracketings());
        assert_ordered(&all_raw_compressions());
        assert_ordered(&all_jpeg_qualities());
        assert_ordered(&all_jpeg_compressions());
        assert_ordered(&all_fluorescent_lamps());
        assert_ordered(&all_contrasts());
        assert_ordered(&all_saturations());
        assert_ordered(&all_sharpnesses());
        assert_ordered(&all_scene_capture_types());
        assert_ordered(&all_gain_controls());
        assert_ordered(&all_picture_control_adjustments());
        assert_ordered(&all_picture_control_filter_effects());
        assert_ordered(&all_picture_control_tonings());
        assert_ordered(&all_active_d_lightings());
        assert_ordered(&all_d_range_optimizers());
        assert_ordered(&all_hdr_strengths());
        assert_ordered(&all_hdr_smoothings());
        assert_ordered(&all_high_iso_noise_reductions());
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
