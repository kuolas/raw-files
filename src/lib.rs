// SPDX-License-Identifier: MIT OR Apache-2.0

//! raw-files: a camera RAW library in pure Rust.
//!
//! raw-files reads camera RAW files and the JPEG files cameras write: the
//! previews they hold, which in a raw file are the JPEG images it embeds
//! and in a JPEG file the file itself and the smaller copies it carries,
//! and the metadata a viewer or a file renamer needs next to them. The RAW
//! files it reads are Nikon NEF and Sony ARW, whose raw sensor data it will
//! later decode. A malformed file gives an error, never a panic.
//!
//! [`RawFile`] opens a file by path, from bytes in memory or from any reader
//! that can seek, detects its format from its content, lists its previews
//! and reads one; [`Options`] opens it with other [`Limits`].
//! [`RawFile::format`] says which [`Format`] the file is, and
//! [`Format::is_raw`] whether it holds raw data; a file of no format
//! raw-files reads is refused with the `UnknownFormat` error.
//! [`RawFile::metadata`] gives what the camera recorded about the picture,
//! read when the file is opened, from the standard TIFF and Exif tags, in a
//! JPEG those of its Exif segment, and, in the raw files and the JPEGs of
//! Nikon and Sony cameras, from what the maker's MakerNote records that
//! they lack: the camera's make, model and serial number, the software
//! that wrote the file, the capture time, the orientation, the exposure,
//! with the focal length in 35 mm terms, the camera's settings (the white
//! balance, the exposure program, the metering mode, the flash, the
//! shutter, the electronic front curtain setting, the image area, the raw
//! image size and the colour space, in [`CameraSettings`], those that only
//! a MakerNote records from the plain tags of Nikon's and Sony's), the
//! lens, the sizes of the raw data, which a JPEG lacks, and of the visible
//! image, and where the camera focused ([`Metadata::focus`]). See the
//! [roadmap](https://github.com/kuolas/raw-files/blob/master/docs/ROADMAP.md)
//! for the plan and the
//! [architecture](https://github.com/kuolas/raw-files/blob/master/docs/ARCHITECTURE.md)
//! for how the code is laid out.
//!
//! # Examples
//!
//! Read the largest preview of a NEF file, with its size and the
//! orientation it must be shown in:
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
//! exposure. Every value is optional: a file that does not record one, or
//! records it in a way that cannot be read, gives `None` for that value
//! alone.
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
//! if let (Some(make), Some(model)) = (metadata.make(), metadata.model()) {
//!     println!("camera: {make} {model}");
//! }
//! if let Some(name) = metadata.lens().and_then(|lens| lens.model()) {
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
//! Find where the camera focused on the largest preview: the window in
//! which to measure how sharp the picture is. A file that records no focus
//! point gives `None`, and one that records the point without its area
//! gives the point alone.
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
//!     .and_then(|(preview, focus)| focus.scaled_to(preview.size()));
//! match on_preview {
//!     Some(focus) => {
//!         let (centre, frame) = (focus.center(), focus.frame());
//!         println!("focused at {centre} on the {frame} preview");
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
//! A program's tests can build every metadata value without a file,
//! through the `with_…` methods of [`Metadata`], [`Exposure`],
//! [`CameraSettings`] and [`WhiteBalance`], [`Lens::builder`], the
//! constructors of [`FineTune`] and [`Flash`], and the checked
//! constructors, such as [`DateTime::new`]; [`Metadata`] shows how.
//!
//! Show how the camera was set ([`Metadata::camera_settings`]), as a
//! viewer's information panel does; every setting is optional, and one the
//! file does not record is left out:
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
//! Tell a raw file from the JPEG of the same shot by their names, before
//! opening either, as a photo browser that shows the two as one picture
//! does: [`Format::from_extension`] gives the format a name suggests, and
//! [`Format::is_raw`] whether files of that format hold raw data.
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
//! - A malformed or truncated file returns an error, never a panic,
//!   whatever its bytes. Every offset and length read from a file is checked
//!   before it is used; the tests feed the readers truncated and corrupted
//!   files, and fuzz targets exercise them.
//! - Damaged metadata costs only what it damages. Each value of
//!   [`Metadata`] is read on its own when the file is opened: a tag that is
//!   missing, of an unexpected type or out of range leaves that value
//!   `None`, and the other values and the previews as they are. A damaged
//!   directory, such as an Exif IFD whose pointer is wrong, loses the
//!   values in it and no others. A JPEG whose Exif segment is missing or
//!   cannot be read still opens: its image size is the only value read
//!   from it, its orientation is the default, no transform, and its
//!   previews are the file itself and the images after the main one.
//! - An untrusted file cannot make the library allocate or walk without
//!   bound. [`Limits`] caps the directories visited, their entries, their
//!   nesting and the size of a preview, with defaults that suit untrusted
//!   input. A cap the walk cannot go on without (the first directory's
//!   size, the total number of directories, a preview larger than allowed
//!   when it is read) fails with
//!   [`LimitExceeded`](ErrorKind::LimitExceeded). A directory past the first
//!   that crosses a cap, or one nested deeper than allowed, is left out as a
//!   damaged one is, so the file still opens with what was reachable. In a
//!   JPEG file, a cap crossed inside its Exif or MPF segment, the first
//!   directory's size included, costs what that segment holds, not the
//!   file.
//! - The format comes from the file's content, never from its name. A
//!   renamed file opens as what it is, and a DNG is refused the same way
//!   even when its Make names Nikon or Sony. [`Format::from_extension`]
//!   exists for file pickers and folder scans, which decide what to offer
//!   before opening anything.
//! - Opening reads the structure and the frame header of each preview
//!   through a block cache, a few blocks rather than the whole file, and a
//!   preview's bytes only when asked for.
//! - The crate is pure Rust with no dependencies outside the standard
//!   library, and it contains no `unsafe` code: it forbids it. So there is
//!   no serde feature either: a caller that stores metadata serialises what
//!   the getters return, which is text, integers and small structs of
//!   integers.
//!
//! # Formats
//!
//! Two RAW formats, Nikon NEF, including the NRW files of Coolpix cameras,
//! and Sony ARW, including the SRF, SR2 and ARQ files of the same family
//! and the ARW files of the Sony-built Hasselblad bodies; and JPEG, the
//! files cameras write beside their raw files or instead of them. Of a raw
//! file, raw-files reads the embedded previews, the metadata and the focus
//! point, not the raw data, each checked on the public-domain (CC0) sample
//! files of [raw.pixls.us](https://raw.pixls.us/): 35 Nikon files, from
//! the D1H to the Z5II, and 36 Sony-family files, from the DSC-F828 to the
//! ILCE-7M5.
//!
//! A JPEG file's previews are the file itself, the thumbnail of its Exif
//! segment, the images its Multi-Picture Format (MPF) list holds after the
//! main one and, from the Nikon bodies that write one, such as the D70,
//! the small JPEG its MakerNote points at. Its metadata is what its Exif
//! segment records, read as in a raw file, with the picture's size as the
//! image size and no raw size; from a Nikon or a Sony camera, also what
//! its MakerNote records, as in a NEF or an ARW: what the Exif segment
//! lacks, the camera settings and the focus point among it, and the white
//! balance setting, finer than the Exif segment's. No other maker's
//! MakerNote is read. These are checked on eight camera JPEGs from
//! [Wikimedia Commons](https://commons.wikimedia.org/), of five Sony
//! bodies, two Nikon bodies and a Canon; on three stripped JPEG files of
//! the maintainer's own Z 9, one in FX and two in the DX and 16:9 crop
//! modes, which check where a crop mode's focus point is placed; and on
//! JPEG files built in the tests.
//!
//! Nikon mirrorless bodies, Nikon DSLRs in live view and Sony bodies from
//! the ILCE-7RM2 on record where they focused as a position, which
//! [`Metadata::focus`] gives, from a raw file or a JPEG alike. A shot that
//! records at most the number of an autofocus point or a mask of the
//! points used, such as a Nikon DSLR's through the viewfinder or a Sony
//! body's before the ILCE-7RM2, gives `None`, and so does a shot in manual
//! focus. A Nikon JPEG shot in a crop mode (DX, 1:1, 16:9) records its
//! point in the camera's full output, and the point is placed on the
//! JPEG's picture. A JPEG gives its point only when the point's frame has
//! the picture's shape, so an export cut to another shape or turned
//! upright gives `None`; one cut from a Nikon crop-mode JPEG is the
//! exception, and its point may be misplaced.
//!
//! # What comes next
//!
//! The [roadmap](https://github.com/kuolas/raw-files/blob/master/docs/ROADMAP.md)
//! plans next the focus point of the shots that record only an autofocus
//! point's number, such as a DSLR's through the viewfinder, mapped to
//! pixels through a table of the body's points; then every metadata value
//! a file records, the ones that only enciphered or encrypted blocks hold
//! among them; then the raw sensor data; and last a developed linear
//! image.

// A fuzzing build compiles the reader for one caller, the fuzz entry points,
// which use only part of it. What is unused there, and so which `expect`s
// go unfulfilled, says nothing about the library: the normal and test
// builds keep both lints, and CI checks those.
#![cfg_attr(fuzzing, allow(dead_code, unfulfilled_lint_expectations))]

mod camera_settings;
mod container;
mod datetime;
mod error;
mod file;
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
// tests of every module that reads a file.
#[cfg(test)]
mod testing;

pub use camera_settings::{
    CameraSettings, ColorSpace, ExposureProgram, FineTune, Flash, FlashMode, ImageArea,
    MeteringMode, RawImageSize, Shutter, WhiteBalance, WhiteBalanceMode,
};
pub use datetime::DateTime;
pub use error::{Error, ErrorKind, Result};
pub use file::RawFile;
pub use format::Format;
pub use geometry::{Point, Size};
pub use limits::Limits;
pub use metadata::{Exposure, Focus, Lens, LensBuilder, Metadata};
pub use options::Options;
pub use orientation::Orientation;
pub use preview::{Preview, PreviewFormat};
pub use rational::{Rational, SignedRational};
