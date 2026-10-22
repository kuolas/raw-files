// SPDX-License-Identifier: MIT OR Apache-2.0

//! raw-files: a camera RAW library in pure Rust.
//!
//! raw-files extracts the JPEG previews embedded in Nikon NEF and Sony ARW
//! files and reads the metadata a viewer or a file renamer needs next to
//! them, and will later decode their raw sensor data. A malformed file
//! gives an error, never a panic.
//!
//! [`RawFile`] opens a file by path, from bytes in memory or from any reader
//! that can seek, detects its format from its content, lists its embedded
//! previews and reads one; [`Options`] opens it with other [`Limits`].
//! Nikon NEF and Sony ARW files open with their previews; any other file is
//! refused with the `UnknownFormat` error.
//! [`RawFile::metadata`] gives what the camera recorded about the picture,
//! read when the file is opened, from the standard TIFF and Exif tags and,
//! on Nikon and Sony files, from what the maker's MakerNote records that
//! they lack: the camera's make, model and serial number, the capture
//! time, the orientation, the exposure, the lens, the sizes of the raw
//! data and of the visible image, and where the camera focused
//! ([`Metadata::focus`]). See the
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
//! through the `with_…` methods of [`Metadata`] and [`Exposure`],
//! [`Lens::builder`] and the checked constructors, such as
//! [`DateTime::new`]; [`Metadata`] shows how.
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
//!   values in it and no others.
//! - An untrusted file cannot make the library allocate or walk without
//!   bound. [`Limits`] caps the directories visited, their entries, their
//!   nesting and the size of a preview, with defaults that suit untrusted
//!   input. A cap the walk cannot go on without (the first directory's
//!   size, the total number of directories, a preview larger than allowed
//!   when it is read) fails with
//!   [`LimitExceeded`](ErrorKind::LimitExceeded). A directory past the first
//!   that crosses a cap, or one nested deeper than allowed, is left out as a
//!   damaged one is, so the file still opens with what was reachable.
//! - The format comes from the file's content, never from its name. A
//!   renamed file opens as what it is, and a DNG is refused the same way
//!   even when its Make names Nikon or Sony. [`Format::from_extension`]
//!   exists for file pickers only.
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
//! Nikon NEF, including the NRW files of Coolpix cameras, and Sony ARW,
//! including the SRF, SR2 and ARQ files of the same family and the ARW files
//! of the Sony-built Hasselblad bodies. Version 0.3.0 reads the embedded
//! previews, the metadata and the focus point of these files, not their raw
//! data. All three are checked on the public-domain (CC0) sample files of
//! [raw.pixls.us](https://raw.pixls.us/): 33 Nikon files, from the D1H to
//! the Z5II, and 36 Sony-family files, from the DSC-F828 to the ILCE-7M5.
//!
//! Nikon mirrorless bodies, Nikon DSLRs in live view and Sony bodies from
//! the ILCE-7RM2 on record where they focused as a position, which
//! [`Metadata::focus`] gives. A shot that records at most the number of an
//! autofocus point or a mask of the points used, such as a Nikon DSLR's
//! through the viewfinder or a Sony body's before the ILCE-7RM2, gives
//! `None`, and so does a shot in manual focus.
//!
//! # What comes next
//!
//! The [roadmap](https://github.com/kuolas/raw-files/blob/master/docs/ROADMAP.md)
//! plans plain JPEG files next, for the pictures a camera writes beside its
//! raw files; then the focus point of the shots that record only an
//! autofocus point's number, such as a DSLR's through the viewfinder,
//! mapped to pixels through a table of the body's points; then the raw
//! sensor data; and last a developed linear image.

// A fuzzing build compiles the reader for one caller, the fuzz entry points,
// which use only part of it. What is unused there, and so which `expect`s
// go unfulfilled, says nothing about the library: the normal and test
// builds keep both lints, and CI checks those.
#![cfg_attr(fuzzing, allow(dead_code, unfulfilled_lint_expectations))]

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
