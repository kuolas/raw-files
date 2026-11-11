// SPDX-License-Identifier: MIT OR Apache-2.0

//! raw-files: a camera RAW library in pure Rust.
//!
//! raw-files reads Nikon NEF and Sony ARW raw files, and the JPEG files
//! cameras write. It gives their previews, their metadata and where the
//! camera focused. It does not decode the raw sensor data yet. The
//! [README](https://github.com/kuolas/raw-files#readme) says what each
//! format is checked on, and the
//! [roadmap](https://github.com/kuolas/raw-files/blob/master/docs/ROADMAP.md)
//! says what comes next.
//!
//! # Opening a file
//!
//! [`RawFile`] is an open camera file. [`RawFile::open`] opens one by path,
//! [`RawFile::from_bytes`] from memory, and [`RawFile::from_reader`] from
//! any reader that can seek. The format is found from the file's content,
//! and [`RawFile::format`] says which [`Format`] it is. [`Options`] opens a
//! file under other [`Limits`].
//!
//! # What a file gives
//!
//! | What | Method | Type |
//! |---|---|---|
//! | The previews | [`RawFile::previews`], [`RawFile::largest_preview`], [`RawFile::preview_at_least`] | [`Preview`] |
//! | A preview's bytes | [`RawFile::read_preview`], [`RawFile::read_preview_into`] | bytes |
//! | What the camera recorded about the picture | [`RawFile::metadata`] | [`Metadata`] |
//! | Every entry of the file's TIFF directories | [`RawFile::entries`] | [`Entries`] of [`Entry`] |
//! | One entry's value | [`RawFile::read_value`] | [`Value`] |
//!
//! The metadata is read when the file is opened. Each of its values comes
//! from one method of [`Metadata`]:
//!
//! | What | Method | Type |
//! |---|---|---|
//! | The camera, its serial number and its owner | [`Metadata::camera`] | [`Camera`], with its [`Make`] |
//! | The software that wrote the file | [`Metadata::software`] | text |
//! | Who made the picture, and what they wrote about it | [`Metadata::authorship`] | [`Authorship`] |
//! | Where the picture was taken | [`Metadata::location`] | [`Location`] |
//! | When it was taken | [`Metadata::capture_time`] | [`DateTime`] |
//! | How to turn it upright | [`Metadata::orientation`] | [`Orientation`] |
//! | The exposure | [`Metadata::exposure`] | [`Exposure`] |
//! | How the camera was set | [`Metadata::camera_settings`] | [`CameraSettings`] |
//! | The lens | [`Metadata::lens`] | [`Lens`] |
//! | The size of the raw data | [`Metadata::raw_size`] | [`Size`] |
//! | The size of the visible image | [`Metadata::image_size`] | [`Size`] |
//! | Where the camera focused | [`Metadata::focus`] | [`Focus`] |
//! | The autofocus points it used | [`Metadata::focus_points`] | [`FocusPoints`] |
//! | The layout of the body's autofocus points | [`Metadata::focus_layout`] | [`FocusLayout`] |
//!
//! The `with_…` methods and the checked constructors, such as
//! [`Focus::new`], build every metadata value without a file, for a
//! program's tests. [`Metadata`] shows how. A constructor that refuses its
//! arguments returns a [`ValueError`] that names the rule they broke. An
//! [`Entry`] comes only from a file.
//!
//! # Examples
//!
//! Read the largest preview of a NEF file, with its size and the
//! orientation to show it in:
//!
//! ```no_run
//! use raw_files::RawFile;
//!
//! # fn main() -> raw_files::Result<()> {
//! let mut file = RawFile::open("photo.nef")?;
//! if let Some(preview) = file.largest_preview() {
//!     let jpeg = file.read_preview(&preview)?;
//!     println!(
//!         "{} pixels, shown upright by {:?}, {} bytes",
//!         preview.size(),
//!         preview.orientation(),
//!         jpeg.len(),
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Print when a picture was taken, with which camera and lens, and at what
//! exposure. Each value is optional, so each is handled on its own:
//!
//! ```no_run
//! use raw_files::RawFile;
//!
//! # fn main() -> raw_files::Result<()> {
//! let file = RawFile::open("photo.nef")?;
//! let metadata = file.metadata();
//! if let Some(time) = metadata.capture_time() {
//!     println!("taken {time}");
//! }
//! if let Some(name) = metadata.camera().display_name() {
//!     println!("camera: {name}");
//! }
//! if let Some(name) = metadata.lens().recorded_model() {
//!     println!("lens: {name}");
//! }
//! let exposure = metadata.exposure();
//! if let (Some(time), Some(f_number), Some(iso)) = (
//!     exposure.exposure_time(),
//!     exposure.f_number(),
//!     exposure.iso(),
//! ) {
//!     println!("{} s at f/{}, ISO {iso}", time.reduced(), f_number.to_f64());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Find where the camera focused on the largest preview. That is the
//! window in which to measure how sharp the picture is. A file that records
//! no focus point gives `None`, and a point recorded without its area has
//! no [`Focus::size`]:
//!
//! ```no_run
//! use raw_files::RawFile;
//!
//! # fn main() -> raw_files::Result<()> {
//! let file = RawFile::open("photo.nef")?;
//! // The camera measures the point in its own frame; scale it to the preview.
//! let on_preview = file
//!     .largest_preview()
//!     .zip(file.metadata().focus())
//!     .and_then(|(preview, focus)| focus.scaled_to(preview.size()).ok());
//! match on_preview {
//!     Some(focus) => {
//!         let (center, frame) = (focus.center(), focus.frame());
//!         println!("focused at {center} on the {frame} preview");
//!         if let Some(size) = focus.size() {
//!             println!("measure sharpness in the {size} area around it");
//!         }
//!     }
//!     None => println!("no focus point to show"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Say where a focus point came from, as a viewer's caption does.
//! [`FocusPoints::origin`] tells a point the camera recorded from one named
//! by number and placed from a table of the body's points.
//! [`FocusPoints::area_mode`] gives the area mode the camera was set to:
//!
//! ```
//! use raw_files::{
//!     Focus, FocusAreaMode, FocusOrigin, FocusPoint, FocusPoints, Metadata, Point, Size,
//! };
//!
//! /// A caption for the point the camera focused on, if the file names one.
//! fn caption(metadata: &Metadata) -> Option<String> {
//!     let points = metadata.focus_points()?;
//!     let primary = points.primary()?;
//!     let point = match primary.number() {
//!         Some(number) => format!("point {number} at {}", primary.center()),
//!         None => format!("at {}", primary.center()),
//!     };
//!     Some(match points.area_mode() {
//!         Some(mode) => format!("{}: {point}, {mode}", points.origin()),
//!         None => format!("{}: {point}", points.origin()),
//!     })
//! }
//!
//! // A Nikon Z 9 records the position it focused at.
//! let frame = Size { width: 8256, height: 5504 };
//! let recorded = Focus::new(frame, Point { x: 4905, y: 2461 }, FocusOrigin::Recorded)
//!     .expect("the center lies on the frame");
//! let z9 = Metadata::default().with_focus(recorded);
//! assert_eq!(caption(&z9).as_deref(), Some("Recorded: at (4905, 2461)"));
//!
//! // A Nikon D600 shot through the viewfinder names point 38, which
//! // raw-files places from its table of the D600's points.
//! let frame = Size { width: 6016, height: 4016 };
//! let point_38 = FocusPoint::new(Point { x: 1812, y: 1705 })
//!     .with_number(38)
//!     .with_size(Size { width: 164, height: 164 });
//! let placed = FocusPoints::new(frame, FocusOrigin::Table)
//!     .and_then(|points| points.with_point(point_38))
//!     .and_then(|points| points.with_primary_index(0))
//!     .expect("the point lies on the frame")
//!     .with_area_mode(FocusAreaMode::Single);
//! let d600 = Metadata::default().with_focus_points(placed);
//! assert_eq!(
//!     caption(&d600).as_deref(),
//!     Some("Placed from a table: point 38 at (1812, 1705), Single point"),
//! );
//! ```
//!
//! Show how the camera was set, as a viewer's information panel does.
//! Every setting is optional, and one the file does not record is left
//! out:
//!
//! ```
//! use raw_files::{
//!     CameraSettings, ExposureProgram, MeteringMode, WhiteBalance,
//!     WhiteBalanceMode,
//! };
//!
//! /// The lines an information panel shows for the settings a file records.
//! fn panel(settings: &CameraSettings) -> Vec<String> {
//!     let mut lines = Vec::new();
//!     if let Some(program) = settings.exposure_program() {
//!         lines.push(format!("Exposure program: {program}"));
//!     }
//!     if let Some(mode) = settings.metering_mode() {
//!         lines.push(format!("Metering: {mode}"));
//!     }
//!     let white_balance = settings.white_balance();
//!     if let Some(mode) = white_balance.mode() {
//!         lines.push(match white_balance.color_temperature() {
//!             Some(temperature) => format!("White balance: {mode}, {temperature} K"),
//!             None => format!("White balance: {mode}"),
//!         });
//!     }
//!     lines
//! }
//!
//! // A file's settings: panel(file.metadata().camera_settings())
//! let settings = CameraSettings::default()
//!     .with_exposure_program(ExposureProgram::AperturePriority)
//!     .with_metering_mode(MeteringMode::MultiSegment)
//!     .with_white_balance(
//!         WhiteBalance::default()
//!             .with_mode(WhiteBalanceMode::Auto)
//!             .with_color_temperature(5_200),
//!     );
//! for line in panel(&settings) {
//!     println!("{line}");
//! }
//! assert_eq!(
//!     panel(&settings),
//!     [
//!         "Exposure program: Aperture priority",
//!         "Metering: Multi-segment",
//!         "White balance: Auto, 5200 K",
//!     ]
//! );
//! ```
//!
//! List every entry of the MakerNote, the maker's own directory, though no
//! standard names its tags. Then say how many of its entries could not be
//! read, and read one standard entry's value:
//!
//! ```no_run
//! use raw_files::{Directory, RawFile};
//!
//! # fn main() -> raw_files::Result<()> {
//! let mut file = RawFile::open("photo.nef")?;
//! let entries = file.entries();
//! for entry in entries.in_directory(Directory::MakerNote) {
//!     println!("0x{:04X} {} x {}", entry.tag(), entry.field_type(), entry.count());
//! }
//! let left_out = entries.skipped(Directory::MakerNote);
//! if left_out > 0 {
//!     println!("{left_out} entries could not be read");
//! }
//! // IFD0's Model (tag 0x0110), as text.
//! if let Some(model) = entries.entry(Directory::Ifd(0), 0x0110) {
//!     if let Some(text) = file.read_value(&model)?.text() {
//!         println!("Model: {text}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Tell a raw file from the JPEG of the same shot by their names, before
//! opening either. A photo browser that shows the two as one picture does
//! this. [`Format::from_extension`] gives the format a name suggests, and
//! [`Format::is_raw`] whether that format holds raw data:
//!
//! ```
//! use std::path::Path;
//!
//! use raw_files::Format;
//!
//! /// The format a file's name says it holds, if raw-files reads it.
//! fn format_of(name: &str) -> Option<Format> {
//!     let extension = Path::new(name).extension()?.to_str()?;
//!     Format::from_extension(extension)
//! }
//!
//! for name in ["DSC_0001.NEF", "DSC_0001.JPG", "notes.txt"] {
//!     let kind = match format_of(name) {
//!         Some(format) if format.is_raw() => "a raw file",
//!         Some(_) => "a picture with no raw data",
//!         None => "not a file raw-files reads",
//!     };
//!     println!("{name}: {kind}");
//! }
//! assert_eq!(format_of("DSC_0001.NEF").map(Format::is_raw), Some(true));
//! assert_eq!(format_of("DSC_0001.JPG").map(Format::is_raw), Some(false));
//! assert_eq!(format_of("notes.txt"), None);
//! ```
//!
//! # Guarantees
//!
//! - **A malformed file returns an error, never a panic.** Every offset
//!   and length read from a file is checked before it is used. The tests
//!   feed the readers truncated and corrupted files, and fuzz targets
//!   exercise them.
//! - **Damaged metadata costs only what it damages.** A tag that is
//!   missing, of an unexpected type or out of range leaves its value
//!   `None`. The other values and the previews stay as they are. A damaged
//!   directory, such as an Exif IFD with a wrong pointer, loses only the
//!   values in it.
//! - **A JPEG whose Exif segment is missing or cannot be read still
//!   opens.** Its previews are the file itself and the images its MPF
//!   list holds after the main one. Its image size is the only value read from it, and its
//!   orientation is the default, no transform.
//! - **An untrusted file cannot make the library allocate or walk without
//!   bound.** [`Limits`] caps the number of directories, their entries and
//!   their nesting. It also caps the size of a preview and of an entry's
//!   value. Its defaults suit untrusted input.
//! - **A cap fails only what it must.** A cap the walk cannot go on without
//!   fails the open with [`LimitExceeded`](ErrorKind::LimitExceeded): the
//!   first directory's size, or the total number of directories. So does a
//!   Make longer than the cap on one value in a TIFF file other than a DNG,
//!   every RAW file among them, since the Make tells its format. A preview
//!   larger than allowed fails that read alone. A value larger than allowed
//!   fails its own [`RawFile::read_value`], is `None` to the metadata, and
//!   costs the previews and directories located by it. A directory past
//!   the first that crosses a cap, or one nested too deep, is left out like
//!   a damaged one. In a JPEG file, a cap crossed inside its Exif or MPF
//!   segment costs at most that segment, not the file.
//! - **The format comes from the file's content, never from its name.** A
//!   renamed file opens as what it is. A DNG is refused even when its Make
//!   names Nikon or Sony. [`Format::from_extension`] serves file pickers and
//!   folder scans, which choose before opening anything.
//! - **Opening reads little.** It reads the structure and each preview's
//!   frame header, a few blocks through a cache rather than the whole file.
//!   A preview's bytes are read only when asked for. Listing the entries
//!   reads nothing more, and [`RawFile::read_value`] reads a value each time
//!   it is asked for.
//! - **Pure Rust, with no dependencies outside the standard library.** The
//!   crate forbids `unsafe` code.
//! - **No serde feature,** since it would be a dependency. A caller that
//!   stores metadata serializes what the getters return: text, integers,
//!   booleans, small structs of these, and enums whose `name` gives their
//!   text. An [`Orientation`] gives its Exif number, a [`PreviewFormat`]
//!   its MIME type, and a [`Directory`] its text through `Display`. A
//!   [`Value`] holds its numbers or bytes in a vector.
//!
//! # Formats
//!
//! raw-files reads Nikon NEF, Sony ARW and JPEG, and [`Format`] says which
//! files each includes. Any other file, a DNG among them, is refused with
//! [`UnknownFormat`](ErrorKind::UnknownFormat). The
//! [README](https://github.com/kuolas/raw-files#formats) says what each
//! format is checked on.

// A fuzzing build compiles the reader for one caller, the fuzz entry points,
// which use only part of it. What is unused there, and so which `expect`s
// go unfulfilled, says nothing about the library: the normal and test
// builds keep both lints, and CI checks those.
#![cfg_attr(fuzzing, allow(dead_code, unfulfilled_lint_expectations))]

mod authorship;
mod camera;
mod camera_settings;
mod container;
mod datetime;
mod entries;
mod error;
mod file;
mod focus;
mod format;
mod formats;
// The entry points of the fuzz targets in `fuzz/`: compiled only with
// `--cfg fuzzing`, which cargo-fuzz sets, and hidden from the docs.
#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzzing;
mod geometry;
mod io;
// The stub JPEG and the JPEG file builder live with the integration tests
// and are shared with the unit tests, as the TIFF fixture builder below is.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/jpeg_builder.rs"]
mod jpeg_builder;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/jpeg_stub.rs"]
mod jpeg_stub;
mod limits;
mod location;
mod metadata;
mod options;
mod orientation;
mod preview;
mod rational;
// The README's code samples, compiled by `cargo test` and run where they
// open no file: only a doc-test build sees this module, so it is never part
// of the library or its docs.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme {}
// The TIFF fixture builder lives with the integration tests and is shared
// with the unit tests: compiled into each test build, never into the
// library. Each side uses only part of it.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/tiff_builder.rs"]
mod tiff_builder;
// The reader that fails past the first cached block, shared by the unit
// tests of every module that reads a file, the checks of an enum's names
// and order, shared by the tests of every module with a named enum, and the
// size, point and focus fixtures the focus, metadata and reader tests share.
#[cfg(test)]
mod testing;
mod value_error;

pub use authorship::Authorship;
pub use camera::{Camera, Make};
pub use camera_settings::{
    CameraSettings, ColorSpace, ExposureProgram, FineTune, Flash, FlashMode, ImageArea,
    MeteringMode, RawImageSize, Shutter, WhiteBalance, WhiteBalanceMode,
};
pub use datetime::DateTime;
pub use entries::{Directory, Entries, Entry, FieldType, Value};
pub use error::{Error, ErrorKind, Result};
pub use file::RawFile;
pub use focus::{
    Focus, FocusAreaMode, FocusLayout, FocusLayoutPoint, FocusOrigin, FocusPoint, FocusPoints,
};
pub use format::Format;
pub use geometry::{Point, Size};
pub use limits::Limits;
pub use location::{Altitude, Direction, Latitude, Location, Longitude, North};
pub use metadata::{Exposure, Lens, Metadata};
pub use options::Options;
pub use orientation::Orientation;
pub use preview::{Preview, PreviewFormat};
pub use rational::{Rational, SignedRational};
pub use value_error::ValueError;
