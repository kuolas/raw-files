// SPDX-License-Identifier: MIT OR Apache-2.0

//! How the camera was set for a picture.
//!
//! Beside the exposure, a viewer's information panel shows how the camera
//! was set for the shot: the white balance, the exposure program, the
//! metering mode, the flash, the shutter, the image area, the raw image
//! size and the colour space. [`CameraSettings`] holds them all, and
//! [`Metadata::camera_settings`](crate::Metadata::camera_settings) gives
//! them for a file. [`WhiteBalance`], [`FineTune`] and [`Flash`] hold the
//! settings made of several parts; each enum, such as [`ExposureProgram`],
//! names one setting's choices in words that fit any maker, with a
//! [`name`](ExposureProgram::name) to show.
//!
//! Every value is optional. A file may not record it, or may record it in
//! a way that cannot be read, such as a code no variant names; either way
//! the value is `None`, and a damaged value costs only itself.
//!
//! NEF, ARW and JPEG files give the settings that their standard Exif tags
//! record: the exposure program, the metering mode, the flash, the white
//! balance mode and the colour space. The others are recorded only in a
//! maker's own tags, its MakerNote. Nikon's, in NEF files and in the JPEGs
//! of Nikon cameras, gives the white balance's colour temperature and
//! fine-tuning, the shutter, the image area and, in a raw file, the raw
//! image size; the white balance setting, finer than Exif's, in place of
//! Exif's; and the colour space where the Exif tags record none, as in
//! every Nikon raw file checked. Sony's, in ARW files and their family and
//! in the JPEGs of Sony cameras, gives the white balance's fine-tuning,
//! the electronic front curtain setting, which only Sony records, and the
//! colour temperature of a chosen colour temperature or a custom white
//! balance; and the white balance setting in place of Exif's. On Sony
//! files the shutter, the image area and the raw image size are `None`:
//! Sony records the first two only in enciphered blocks, which raw-files
//! does not read, and gives an APS-C raw the raw size code of a medium
//! one.
//!
//! A program that uses raw-files can build these values for its own
//! tests, without a camera file: [`CameraSettings`] and [`WhiteBalance`]
//! through their `with_…` methods, and a [`FineTune`] and a [`Flash`]
//! through their constructors. [`CameraSettings`]'s docs show how.

use std::fmt;

use crate::rational::SignedRational;

/// How the camera was set for a picture: its white balance, exposure
/// program, metering mode, flash, shutter, image area, raw image size and
/// colour space, for a viewer's information panel beside the exposure.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read, such as a code that no variant names. A
/// damaged value costs only itself.
/// [`Metadata::camera_settings`](crate::Metadata::camera_settings) gives
/// the values a file's standard Exif tags record: the exposure program,
/// the metering mode, the flash, the white balance mode and the colour
/// space. The others, which only a maker's MakerNote records, it gives
/// from Nikon's MakerNote and Sony's, each those its MakerNote records in
/// plain tags; the shutter, the image area and the raw image size are
/// `None` for every Sony file. Each getter says where its value comes
/// from.
///
/// `CameraSettings::default()` is the empty record, with every value
/// `None` and an empty [`WhiteBalance`]. A caller's tests start from it and
/// set the values they need with the `with_…` methods, such as
/// [`with_metering_mode`](Self::with_metering_mode), each named after the
/// method that reads the value back and each replacing any value set
/// before. As for [`Metadata`](crate::Metadata), every value is optional,
/// so the empty record is the start of every chain and there is no
/// separate builder type.
///
/// The `with_…` methods store each value as given: none refuses or changes
/// one. `CameraSettings` is not `Copy`, so that a later value that cannot
/// be copied does not break callers.
///
/// # Examples
///
/// The settings a file records, each on its own, since each is optional:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
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
///             .with_kelvin(5_200),
///     )
///     .with_flash(Flash::new(false).with_mode(FlashMode::Off));
///
/// assert_eq!(
///     settings.exposure_program().map(ExposureProgram::name),
///     Some("Aperture priority")
/// );
/// assert_eq!(settings.metering_mode(), Some(MeteringMode::MultiSegment));
/// assert_eq!(settings.white_balance().kelvin(), Some(5_200));
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

    /// The colour space the camera was set to, such as sRGB.
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

    /// Sets the colour space, in place of any set before;
    /// [`color_space`](Self::color_space) reads it.
    #[must_use]
    pub fn with_color_space(mut self, color_space: ColorSpace) -> Self {
        self.color_space = Some(color_space);
        self
    }
}

/// The white balance the camera was set to: its mode, the colour
/// temperature the file records, and the photographer's fine-tuning.
///
/// Every part is `None` when the file does not record it or records it in
/// a way that cannot be read. `WhiteBalance::default()` is the empty one,
/// with every part `None`; a caller's tests set the parts they need with
/// the `with_…` methods, each named after the method that reads the part
/// back and each replacing any value set before. `WhiteBalance` is not
/// `Copy`, so that a later part that cannot be copied does not break
/// callers.
///
/// # Examples
///
/// A Nikon body's auto white balance, with the colour temperature it chose
/// and no shift:
///
/// ```
/// use raw_files::{FineTune, SignedRational, WhiteBalance, WhiteBalanceMode};
///
/// let zero = SignedRational { numerator: 0, denominator: 4 };
/// let white_balance = WhiteBalance::default()
///     .with_mode(WhiteBalanceMode::Auto)
///     .with_kelvin(5_200)
///     .with_fine_tune(FineTune::new(zero, zero));
///
/// assert_eq!(white_balance.mode().map(WhiteBalanceMode::name), Some("Auto"));
/// assert_eq!(white_balance.kelvin(), Some(5_200));
/// assert_eq!(
///     white_balance.fine_tune().map(|fine_tune| fine_tune.amber_blue()),
///     Some(zero)
/// );
/// assert_eq!(WhiteBalance::default().mode(), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WhiteBalance {
    pub(crate) mode: Option<WhiteBalanceMode>,
    pub(crate) kelvin: Option<u32>,
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
    /// are known to hold, such as `AUTO1` or `COOL WHT FL`; a chosen colour
    /// temperature, written as its number, such as `3230K`, gives
    /// [`Kelvin`](WhiteBalanceMode::Kelvin). A text the table lacks gives
    /// way to Exif's mode, never to a guess from its letters. Sony's
    /// MakerNote records the setting as a code, which gives a mode only
    /// where a file shows what the code means, such as
    /// [`Underwater`](WhiteBalanceMode::Underwater), which Exif calls auto,
    /// or [`Kelvin`](WhiteBalanceMode::Kelvin); any other code gives way to
    /// Exif's mode. The Sony DSLR-A100 keeps its code where raw-files does
    /// not read it, so its mode is Exif's. `None` when neither gives a
    /// mode: for a light no mode stands for, such as Exif's standard light
    /// D65, and when the camera records "unknown" or "other".
    pub fn mode(&self) -> Option<WhiteBalanceMode> {
        self.mode
    }

    /// The colour temperature the file records, in kelvin. A recorded 0
    /// means that none was recorded, and is `None`; so is a number above
    /// 99,999 K, which is no temperature a camera is set to.
    ///
    /// What it means differs by maker, and only a maker's MakerNote records
    /// it. On Nikon, from its ColorTemperatureAuto, it is the colour
    /// temperature of the white balance the camera applied, under any
    /// [`mode`](Self::mode): the temperature auto white balance chose, the
    /// one a chosen colour temperature set, or the one a kind of light or a
    /// preset stands for. The D5, the D850, the D780 and the Z bodies
    /// checked record it; the Z 9 under every setting, while the D850 and
    /// the Z fc write 0 under daylight. The Nikon bodies before them record
    /// a temperature only under a chosen colour temperature
    /// ([`Kelvin`](WhiteBalanceMode::Kelvin)), as the number of their white
    /// balance text, such as `3230K`, which gives it. On Sony, from its
    /// ColorTemperature, it is the colour temperature the photographer
    /// chose under a chosen colour temperature
    /// ([`Kelvin`](WhiteBalanceMode::Kelvin)), and the custom white
    /// balance's own under [`Custom`](WhiteBalanceMode::Custom), as the
    /// files checked show; the [`mode`](Self::mode) beside it says which.
    /// Under every other setting every Sony file checked records 0, which
    /// gives `None`.
    pub fn kelvin(&self) -> Option<u32> {
        self.kelvin
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

    /// Sets the colour temperature, in kelvin, in place of any set before;
    /// [`kelvin`](Self::kelvin) reads it.
    #[must_use]
    pub fn with_kelvin(mut self, kelvin: u32) -> Self {
        self.kelvin = Some(kelvin);
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
/// axis it is on. There is no `Default`, since there is no empty
/// fine-tuning: a file that records none gives `None` for
/// [`WhiteBalance::fine_tune`].
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
    /// The whole frame, weighted toward its centre: Exif's centre-weighted
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
/// assert_eq!(WhiteBalanceMode::Kelvin.to_string(), "Color temperature");
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
    /// A white balance measured from a reference, such as a grey card:
    /// Nikon's preset manual, Sony's custom.
    Custom,
    /// A colour temperature the photographer chose.
    Kelvin,
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
            WhiteBalanceMode::Kelvin => "Color temperature",
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
    /// A 1.3x crop: the centre of the sensor, by a crop factor of 1.3.
    Crop1_3x,
    /// A 2.7x crop: the centre of the sensor, by a crop factor of 2.7.
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

/// The colour space the camera was set to.
///
/// Exif's ColorSpace names sRGB alone; for any other colour space it says
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
    /// The colour space's name, for an information panel: "sRGB" or "Adobe
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fraction(numerator: i32, denominator: i32) -> SignedRational {
        SignedRational {
            numerator,
            denominator,
        }
    }

    /// A white balance with every part set: Nikon's auto, the temperature
    /// it chose, and a shift in quarter steps along both axes, each value
    /// different from the others.
    fn nikon_white_balance() -> WhiteBalance {
        WhiteBalance {
            mode: Some(WhiteBalanceMode::Auto),
            kelvin: Some(5_200),
            fine_tune: Some(FineTune {
                amber_blue: fraction(-24, 4),
                green_magenta: fraction(4, 4),
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

        let white_balance = WhiteBalance::default();
        assert_eq!(white_balance.mode(), None);
        assert_eq!(white_balance.kelvin(), None);
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

        let white_balance = nikon_white_balance();
        assert_eq!(white_balance.mode(), Some(WhiteBalanceMode::Auto));
        assert_eq!(white_balance.kelvin(), Some(5_200));
        let fine_tune = white_balance.fine_tune().unwrap();
        assert_eq!(fine_tune.amber_blue(), fraction(-24, 4));
        assert_eq!(fine_tune.green_magenta(), fraction(4, 4));
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
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
    }

    #[test]
    fn each_white_balance_builder_sets_its_own_value_and_no_other() {
        let fine_tune = FineTune {
            amber_blue: fraction(1, 2),
            green_magenta: fraction(-1, 2),
        };
        for start in [WhiteBalance::default(), nikon_white_balance()] {
            let cases = [
                (
                    start.clone().with_mode(WhiteBalanceMode::Kelvin),
                    WhiteBalance {
                        mode: Some(WhiteBalanceMode::Kelvin),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_kelvin(3_200),
                    WhiteBalance {
                        kelvin: Some(3_200),
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
                    .with_kelvin(5_200)
                    .with_fine_tune(FineTune::new(fraction(-24, 4), fraction(4, 4))),
            )
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(Flash::new(true).with_mode(FlashMode::Fill))
            .with_shutter(Shutter::ElectronicFrontCurtain)
            .with_electronic_front_curtain_setting(true)
            .with_image_area(ImageArea::FullFrame)
            .with_raw_image_size(RawImageSize::Large)
            .with_color_space(ColorSpace::AdobeRgb);
        assert_eq!(settings, full_settings());
    }

    #[test]
    fn a_later_value_replaces_an_earlier_one() {
        let earlier = CameraSettings::default()
            .with_white_balance(WhiteBalance::default().with_kelvin(1))
            .with_exposure_program(ExposureProgram::Manual)
            .with_metering_mode(MeteringMode::Average)
            .with_flash(Flash::new(false))
            .with_shutter(Shutter::Mechanical)
            .with_electronic_front_curtain_setting(false)
            .with_image_area(ImageArea::Crop2_7x)
            .with_raw_image_size(RawImageSize::Medium)
            .with_color_space(ColorSpace::Srgb);
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
            .with_color_space(ColorSpace::AdobeRgb);
        assert_eq!(settings, full);

        let white_balance = WhiteBalance::default()
            .with_mode(WhiteBalanceMode::Daylight)
            .with_kelvin(1)
            .with_fine_tune(FineTune::new(fraction(1, 1), fraction(1, 1)))
            .with_mode(WhiteBalanceMode::Auto)
            .with_kelvin(5_200)
            .with_fine_tune(FineTune::new(fraction(-24, 4), fraction(4, 4)));
        assert_eq!(white_balance, nikon_white_balance());
    }

    #[test]
    fn a_fine_tune_keeps_both_shifts_as_given() {
        // Two different shifts, so that one given for the other is caught.
        let fine_tune = FineTune::new(fraction(-6, 1), fraction(2, 4));
        assert_eq!(fine_tune.amber_blue(), fraction(-6, 1));
        assert_eq!(fine_tune.green_magenta(), fraction(2, 4));
        // As stored: -24/4 is not -6/1, and a zero shift is a value.
        assert_ne!(FineTune::new(fraction(-24, 4), fraction(2, 4)), fine_tune);
        let zero = FineTune::new(fraction(0, 4), fraction(0, 4));
        assert_eq!(zero.amber_blue(), fraction(0, 4));
        assert_eq!(zero.green_magenta(), fraction(0, 4));
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
            Kelvin,
        ];
        for mode in all {
            match mode {
                Auto | Daylight | Cloudy | Shade | Tungsten | Fluorescent | Flash | Underwater
                | Custom | Kelvin => {}
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

    /// Checks that `all`'s names are `expected`, in order, and so as many,
    /// that `Display` writes each name, and that every name is distinct,
    /// ASCII and in sentence case.
    fn assert_names<T: Copy + fmt::Debug + fmt::Display>(
        all: &[T],
        name: fn(T) -> &'static str,
        expected: &[&str],
    ) {
        let names: Vec<&str> = all.iter().map(|&value| name(value)).collect();
        assert_eq!(names, expected);
        for &value in all {
            let text = name(value);
            assert_eq!(value.to_string(), text, "{value:?}");
            assert!(text.is_ascii(), "{value:?}");
            // Sentence case: a capital first, or a digit. "sRGB" is the one
            // name whose own spelling starts in lower case.
            let capital = text.starts_with(|c: char| c.is_ascii_uppercase() || c.is_ascii_digit());
            assert!(capital || text == "sRGB", "{value:?}");
        }
        let distinct: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(distinct.len(), names.len(), "{names:?}");
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
    }

    /// Checks that `all`, the variants in declaration order, is strictly
    /// increasing.
    fn assert_ordered<T: Copy + Ord + fmt::Debug>(all: &[T]) {
        assert!(all.windows(2).all(|pair| pair[0] < pair[1]), "{all:?}");
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
