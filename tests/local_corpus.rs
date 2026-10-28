// SPDX-License-Identifier: MIT OR Apache-2.0

//! The reader run over a folder of your own camera files, RAW and JPEG, to
//! see whether it copes with cameras and settings the committed samples do
//! not cover.
//!
//! The test is ignored by default. Run it, in release mode because it reads
//! every file, with
//!
//! ```sh
//! RAW_FILES_LOCAL_CORPUS=/path/to/photos cargo test --release --test local_corpus -- --ignored --nocapture
//! ```
//!
//! It walks the folder and its subfolders, opens every file whose extension
//! names a format raw-files reads, and reads the metadata, the previews and
//! the largest preview's bytes of each. It skips, and counts, the
//! AppleDouble files among them: a file whose name starts with `._`, which
//! macOS writes beside a file to hold its extended attributes on a disk
//! that cannot, such as one formatted exFAT, and which is no photo though
//! it keeps the photo's extension. It counts each file under the
//! format its extension names, failures included, and prints one section
//! of counts for each format the folder holds; a section also counts the
//! formats its files opened as, so that a renamed file shows. It prints
//! counts only: how many files opened, how many have each metadata field,
//! how many previews the files have, and how the sizes are distributed; it
//! never prints a path, a file name, or the value of a make, model,
//! software, serial number, capture time, exposure or lens, so its output
//! can be pasted into an issue without disclosing anything about the
//! photos.
//!
//! For each camera setting, from the exposure program to the colour space,
//! it counts, over the files of a section, how many have the setting and
//! how many have each kind of it, by the name the library gives the kind,
//! such as "Aperture priority"; how many flashes fired; how many files had
//! the electronic front curtain setting on and how many off; how many have
//! each white balance fine-tuning, by its pair of shifts, such as
//! `(-24/4, 0/4)`; and the lowest and the highest colour temperature
//! recorded. As for the sizes, these are totals over the section, tied to
//! no file.
//!
//! For the focus point, where the camera focused, it counts the files that
//! have one, how many of those record that the camera confirmed focus, that
//! it missed, or neither, the sizes of the frames the points are measured
//! in, how many different sizes the focus areas have, and how many frames
//! are not the file's image size. It counts the focus points by their
//! origin too, by the name the library gives it: "Recorded" for a position
//! the camera measured, "Table" for a point placed from the table of the
//! body's autofocus points.
//!
//! For the autofocus points the camera used, it counts the files with a
//! list of them and how many of those mark a primary, how many files have
//! each number of points in their list, how many points the lists hold and
//! how many of those have a number, the lists by area mode, by the name
//! the library gives the mode, such as "Auto area", and the lists of one
//! point placed from a table. For the layout of the body's autofocus
//! points, it counts the files with one, by its number of points, and the
//! files with one but no focus point. Two of these counts each gather cases
//! the library gives alike, which the output cannot tell apart: a list of
//! one point placed from a table is a single-point shot or a shot whose
//! mask of the points used names no point beside the primary, and a file
//! with a layout and no focus point is a manual-focus shot, a shot that
//! names point 0, a crop the body's table does not confirm, a crop-mode
//! JPEG smaller than the crop's window, a JPEG cut to another shape than
//! the frame's, such as the D600's Medium JPEG, which the JPEG shape rule
//! refuses, or a file whose MakerNote is missing or cannot be read, since
//! the layout comes from the Model alone. It never prints where a point
//! lies, how large one area is, or a point's number.
//!
//! It fails when the folder cannot be read or holds no file of a format
//! raw-files reads, and otherwise unless every entry could be read and, in
//! every format's section, every file opened, every file has a preview,
//! every largest preview is a whole JPEG, every file's metadata gives the
//! orientation its largest preview carries, and every focus point lies
//! inside its frame, its area included, both as read and once placed on the
//! largest preview, as does every point of a list of the points used. Nor
//! may the focus point, the list and the layout disagree: every list must
//! mark a primary, and the focus point must be that primary, in the list's
//! frame and with its origin; every recorded position must be a list of
//! one point without a number; and every list placed from a table must
//! come with a layout, for the Model the file names, and hold points of
//! that layout, numbered in rising order, each with the layout's box and
//! the layout's centre, moved on a crop into the centred window that is
//! the list's frame. These hold by how the library builds a list, and the
//! test checks them by hand rather than taking the library's word for it.
//! A whole JPEG starts with its start marker and ends with
//! its end marker, followed by nothing but the padding some Sony cameras
//! write: `FF` bytes, in any format, and zeros, only in the largest preview
//! of a file that opens as JPEG, which is the whole file. Such a file must
//! also hold its own image's end marker, after its first scan, not only
//! its thumbnail's. Other data after the last end marker fails a RAW
//! file's preview, and is only counted in a JPEG file's; except that a
//! NEF file's preview may end before its declared length in the shape
//! the D600 and the D3200 sometimes write, a length 2 more than a
//! multiple of 512 with up to 513 bytes of other data after the image's
//! own end marker, which is counted as a preview that ends early. The metadata
//! counts are printed for the reader of the output to judge, since what a
//! folder's cameras record varies: a body that writes no serial number, or
//! no sub-seconds, is not a failure of the library.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use raw_files::{
    DateTime, FocusLayout, FocusOrigin, FocusPoints, Format, Metadata, Point, RawFile, Size,
};

/// A file that counts the bytes its reads return, into a counter the caller
/// keeps, so that the count is still there once the library owns the file.
struct CountingReader {
    file: File,
    bytes_read: Rc<Cell<u64>>,
}

impl Read for CountingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(buf)?;
        self.bytes_read.set(self.bytes_read.get() + n as u64);
        Ok(n)
    }
}

impl Seek for CountingReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

/// How many files have each metadata field, and how the sizes are
/// distributed. Counts only: no value of a text, a time, an exposure or a
/// lens is kept, apart from the capture times, which are kept to count the
/// files that share one and are never printed.
#[derive(Default)]
struct MetadataTally {
    make: usize,
    model: usize,
    software: usize,
    serial_number: usize,
    capture_time: usize,
    with_subseconds: usize,
    with_offset: usize,
    iso: usize,
    exposure_time: usize,
    f_number: usize,
    focal_length: usize,
    focal_length_in_35mm_film: usize,
    exposure_bias: usize,
    lens: usize,
    lens_model: usize,
    lens_focal_range: usize,
    raw_sizes: BTreeMap<(u32, u32), usize>,
    image_sizes: BTreeMap<(u32, u32), usize>,
    /// How many files record each capture time, sub-seconds and offset
    /// included (`DateTime` equality counts both), to count the files whose
    /// time is not their own.
    capture_times: HashMap<DateTime, usize>,
}

impl MetadataTally {
    /// Counts what `metadata` has.
    fn record(&mut self, metadata: &Metadata) {
        self.make += usize::from(metadata.make().is_some());
        self.model += usize::from(metadata.model().is_some());
        self.software += usize::from(metadata.software().is_some());
        self.serial_number += usize::from(metadata.serial_number().is_some());
        if let Some(time) = metadata.capture_time() {
            self.capture_time += 1;
            self.with_subseconds += usize::from(time.subsecond_digits() > 0);
            self.with_offset += usize::from(time.offset_minutes().is_some());
            *self.capture_times.entry(time).or_default() += 1;
        }
        let exposure = metadata.exposure();
        self.iso += usize::from(exposure.iso().is_some());
        self.exposure_time += usize::from(exposure.exposure_time().is_some());
        self.f_number += usize::from(exposure.f_number().is_some());
        self.focal_length += usize::from(exposure.focal_length().is_some());
        self.focal_length_in_35mm_film +=
            usize::from(exposure.focal_length_in_35mm_film().is_some());
        self.exposure_bias += usize::from(exposure.exposure_bias().is_some());
        if let Some(lens) = metadata.lens() {
            self.lens += 1;
            self.lens_model += usize::from(lens.model().is_some());
            self.lens_focal_range +=
                usize::from(lens.min_focal_length().is_some() && lens.max_focal_length().is_some());
        }
        if let Some(size) = metadata.raw_size() {
            *self.raw_sizes.entry((size.width, size.height)).or_default() += 1;
        }
        if let Some(size) = metadata.image_size() {
            *self
                .image_sizes
                .entry((size.width, size.height))
                .or_default() += 1;
        }
    }

    /// How many files record a capture time that another file records too,
    /// sub-seconds and offset included. Two frames of a burst share their
    /// second, and a camera that records hundredths tells them apart: a
    /// change that lost or rounded the sub-seconds would show here.
    fn shared_capture_times(&self) -> usize {
        self.capture_times
            .values()
            .filter(|&&count| count > 1)
            .sum()
    }
}

/// A fine-tuning's two shifts as the library gives them, amber to blue
/// first, each signed as its axis is named and as its numerator and
/// denominator, so that the counts sort.
type Shifts = ((i32, i32), (i32, i32));

/// How the camera was set for the files, in counts: for each setting, how
/// many files have it, and how many have each of its kinds, by the name the
/// library gives the kind, such as "Aperture priority". The flash is
/// counted by whether it fired too. A setting the library names no kinds
/// of is counted by what sets it apart instead: the electronic front
/// curtain setting by whether it was on, and the fine-tuning by its pair
/// of shifts, since a shift is a step the camera's menu offers, not a
/// value of one picture. The colour temperature, a number, is given as the
/// lowest and the highest any file records. Like every other count, these
/// are totals over the files, tied to no file.
#[derive(Default)]
struct SettingsTally {
    exposure_program: usize,
    exposure_programs: BTreeMap<&'static str, usize>,
    metering_mode: usize,
    metering_modes: BTreeMap<&'static str, usize>,
    /// Files with a flash, of which `flash_fired` fired; `flash_modes`
    /// counts the ones with each mode, and `flash_without_mode` the ones
    /// that record none.
    flash: usize,
    flash_fired: usize,
    flash_modes: BTreeMap<&'static str, usize>,
    flash_without_mode: usize,
    white_balance_mode: usize,
    white_balance_modes: BTreeMap<&'static str, usize>,
    /// Files with a colour temperature, and the lowest and the highest
    /// recorded, `None` while no file has one.
    color_temperature: usize,
    color_temperature_range: Option<(u32, u32)>,
    fine_tune: usize,
    fine_tunes: BTreeMap<Shifts, usize>,
    shutter: usize,
    shutters: BTreeMap<&'static str, usize>,
    /// Files with an electronic front curtain setting, of which
    /// `electronic_front_curtain_on` had it on.
    electronic_front_curtain: usize,
    electronic_front_curtain_on: usize,
    image_area: usize,
    image_areas: BTreeMap<&'static str, usize>,
    raw_image_size: usize,
    raw_image_sizes: BTreeMap<&'static str, usize>,
    color_space: usize,
    color_spaces: BTreeMap<&'static str, usize>,
}

impl SettingsTally {
    /// Counts the camera settings `metadata` has.
    fn record(&mut self, metadata: &Metadata) {
        /// Counts one more file with the setting, in `total`, and one more
        /// with its kind `name`, in `by_kind`.
        fn count(
            total: &mut usize,
            by_kind: &mut BTreeMap<&'static str, usize>,
            name: &'static str,
        ) {
            *total += 1;
            *by_kind.entry(name).or_default() += 1;
        }
        let settings = metadata.camera_settings();
        if let Some(program) = settings.exposure_program() {
            count(
                &mut self.exposure_program,
                &mut self.exposure_programs,
                program.name(),
            );
        }
        if let Some(mode) = settings.metering_mode() {
            count(
                &mut self.metering_mode,
                &mut self.metering_modes,
                mode.name(),
            );
        }
        if let Some(flash) = settings.flash() {
            self.flash += 1;
            self.flash_fired += usize::from(flash.fired());
            match flash.mode() {
                Some(mode) => *self.flash_modes.entry(mode.name()).or_default() += 1,
                None => self.flash_without_mode += 1,
            }
        }
        let white_balance = settings.white_balance();
        if let Some(mode) = white_balance.mode() {
            count(
                &mut self.white_balance_mode,
                &mut self.white_balance_modes,
                mode.name(),
            );
        }
        if let Some(temperature) = white_balance.color_temperature() {
            self.color_temperature += 1;
            self.color_temperature_range = Some(match self.color_temperature_range {
                Some((lowest, highest)) => (lowest.min(temperature), highest.max(temperature)),
                None => (temperature, temperature),
            });
        }
        if let Some(fine_tune) = white_balance.fine_tune() {
            let (amber_blue, green_magenta) = (fine_tune.amber_blue(), fine_tune.green_magenta());
            self.fine_tune += 1;
            *self
                .fine_tunes
                .entry((
                    (amber_blue.numerator, amber_blue.denominator),
                    (green_magenta.numerator, green_magenta.denominator),
                ))
                .or_default() += 1;
        }
        if let Some(shutter) = settings.shutter() {
            count(&mut self.shutter, &mut self.shutters, shutter.name());
        }
        if let Some(on) = settings.electronic_front_curtain_setting() {
            self.electronic_front_curtain += 1;
            self.electronic_front_curtain_on += usize::from(on);
        }
        if let Some(area) = settings.image_area() {
            count(&mut self.image_area, &mut self.image_areas, area.name());
        }
        if let Some(size) = settings.raw_image_size() {
            count(
                &mut self.raw_image_size,
                &mut self.raw_image_sizes,
                size.name(),
            );
        }
        if let Some(color_space) = settings.color_space() {
            count(
                &mut self.color_space,
                &mut self.color_spaces,
                color_space.name(),
            );
        }
    }
}

/// What the files say about where the camera focused, in counts: the
/// focus point, the list of the points the camera used and the layout of
/// the body's points. No centre, no area size and no point number is
/// printed: the area sizes are kept only to count how many different ones
/// there are.
#[derive(Default)]
struct FocusTally {
    with_point: usize,
    without_point: usize,
    /// Focus points by where they came from, by the name the library gives
    /// the origin, such as "Table".
    origins: BTreeMap<&'static str, usize>,
    /// Files with a list of the points the camera used, of which
    /// `with_primary` mark a primary; `list_sizes` counts the files with
    /// each number of points in their list.
    with_list: usize,
    with_primary: usize,
    list_sizes: BTreeMap<usize, usize>,
    /// Points over every list, of which `numbered_points` have a number.
    list_points: usize,
    numbered_points: usize,
    /// Lists by area mode, by the name the library gives the mode, and the
    /// lists without one.
    area_modes: BTreeMap<&'static str, usize>,
    without_area_mode: usize,
    /// Lists placed from a table that hold one point: a shot whose mask
    /// names no point beside the primary, or a single-point shot. The
    /// library gives both the same list, so they are counted together.
    single_placed: usize,
    /// Files with a layout of the body's points; `layout_sizes` counts the
    /// files whose layout has each number of points.
    with_layout: usize,
    layout_sizes: BTreeMap<usize, usize>,
    /// Files with a layout and no focus point: a body with a table whose
    /// file names no point the library places, which a manual-focus shot,
    /// a point number of 0, a crop the table does not confirm, a
    /// crop-mode JPEG smaller than the crop's window, a JPEG cut to
    /// another shape than the frame's, such as the D600's Medium JPEG,
    /// which the JPEG shape rule refuses, and a file whose MakerNote is
    /// missing or cannot be read all give alike, so they are counted
    /// together.
    layout_without_point: usize,
    /// Files whose list and focus point disagree: a list without a
    /// primary, or a focus point that is not the list's primary, in its
    /// frame and with its origin. The focus point is the list's primary,
    /// so this must stay zero.
    list_not_focus: usize,
    /// Files whose recorded position is not a list of one point without
    /// a number, as the library gives every recorded position: this must
    /// stay zero.
    recorded_not_one_point: usize,
    /// Files whose points are placed from a table and have no layout: the
    /// list and the layout come from the same table, so this must stay
    /// zero.
    placed_without_layout: usize,
    /// Files whose layout is not for the Model they name: the library
    /// finds the table by the Model, so this must stay zero.
    layout_not_for_model: usize,
    /// Files whose points placed from a table are not the layout's, as
    /// [`placed_from`] checks them: this must stay zero.
    placed_not_from_layout: usize,
    /// Files with a point of their list outside the list's frame, area
    /// included, checked by hand as for `outside_frame`: this must stay
    /// zero.
    list_outside_frame: usize,
    /// Files whose list could not be placed on their largest preview, or
    /// has a point outside it once placed, area included: as for
    /// `not_scaled` and `scaled_outside_frame`, this must stay zero.
    list_scaled_outside_frame: usize,
    /// Files whose camera confirmed focus; `missed` and `unknown` count
    /// the ones that record a miss and the ones that do not say.
    confirmed: usize,
    missed: usize,
    unknown: usize,
    /// How many points are measured in each frame size.
    frames: BTreeMap<(u32, u32), usize>,
    /// The different sizes of the focus areas, kept to count them.
    area_sizes: BTreeSet<(u32, u32)>,
    /// Files whose point, checked by hand here rather than trusted to the
    /// library, lies outside its frame, area included: the library builds
    /// no focus that does, so this must stay zero.
    outside_frame: usize,
    /// Files whose frame is not their image size, or that record no image
    /// size. Printed, not asserted: a Sony picture taken at another aspect
    /// ratio than the sensor's, or a medium or small raw, differs by design.
    frame_not_image_size: usize,
    /// Files whose point could not be placed on their largest preview. The
    /// library refuses only a frame with a zero side, and a largest preview
    /// with a zero side would be a finding in itself, so this must stay
    /// zero.
    not_scaled: usize,
    /// Files whose point, once placed on their largest preview, lies
    /// outside the preview, area included, checked by hand as for
    /// `outside_frame`: this must stay zero too.
    scaled_outside_frame: usize,
}

impl FocusTally {
    /// Counts what `metadata` says about the focus.
    fn record(&mut self, metadata: &Metadata) {
        let layout = metadata.focus_layout();
        if let Some(layout) = layout {
            self.with_layout += 1;
            *self.layout_sizes.entry(layout.points().len()).or_default() += 1;
            self.layout_without_point += usize::from(metadata.focus().is_none());
            self.layout_not_for_model += usize::from(metadata.model() != Some(layout.model()));
        }
        if let Some(list) = metadata.focus_points() {
            self.record_list(list, layout);
        }
        self.list_not_focus += usize::from(!list_gives_focus(metadata));
        let Some(focus) = metadata.focus() else {
            self.without_point += 1;
            return;
        };
        self.with_point += 1;
        *self.origins.entry(focus.origin().name()).or_default() += 1;
        match focus.in_focus() {
            Some(true) => self.confirmed += 1,
            Some(false) => self.missed += 1,
            None => self.unknown += 1,
        }
        let frame = focus.frame();
        *self.frames.entry((frame.width, frame.height)).or_default() += 1;
        if let Some(size) = focus.size() {
            self.area_sizes.insert((size.width, size.height));
        }
        self.outside_frame += usize::from(!lies_inside(focus.center(), focus.size(), frame));
        self.frame_not_image_size += usize::from(metadata.image_size() != Some(frame));
    }

    /// Counts what `list`, a file's list of the points the camera used,
    /// holds, and checks it against `layout`, the file's layout of the
    /// body's points, when its points are placed from a table.
    fn record_list(&mut self, list: &FocusPoints, layout: Option<FocusLayout>) {
        let points = list.points();
        self.with_list += 1;
        self.with_primary += usize::from(list.primary().is_some());
        *self.list_sizes.entry(points.len()).or_default() += 1;
        self.list_points += points.len();
        self.numbered_points += points
            .iter()
            .filter(|point| point.number().is_some())
            .count();
        match list.area_mode() {
            Some(mode) => *self.area_modes.entry(mode.name()).or_default() += 1,
            None => self.without_area_mode += 1,
        }
        self.list_outside_frame += usize::from(
            !points
                .iter()
                .all(|point| lies_inside(point.center(), point.size(), list.frame())),
        );
        match list.origin() {
            FocusOrigin::Recorded => {
                let one_point = matches!(points, [point] if point.number().is_none());
                self.recorded_not_one_point += usize::from(!one_point);
            }
            FocusOrigin::Table => {
                self.single_placed += usize::from(points.len() == 1);
                match layout {
                    Some(layout) => {
                        self.placed_not_from_layout += usize::from(!placed_from(list, &layout));
                    }
                    None => self.placed_without_layout += 1,
                }
            }
            // An origin added to the library after this test is counted
            // above by its name and checked no further.
            _ => {}
        }
    }

    /// Places the focus point and the list of `metadata` on a largest
    /// preview of size `preview`, and counts whether that worked and kept
    /// every point inside the preview.
    fn record_scaled(&mut self, metadata: &Metadata, preview: Size) {
        if let Some(focus) = metadata.focus() {
            match focus.scaled_to(preview) {
                Some(scaled) => {
                    self.scaled_outside_frame +=
                        usize::from(!lies_inside(scaled.center(), scaled.size(), preview));
                }
                None => self.not_scaled += 1,
            }
        }
        if let Some(list) = metadata.focus_points() {
            let inside = list.scaled_to(preview).is_some_and(|scaled| {
                scaled
                    .points()
                    .iter()
                    .all(|point| lies_inside(point.center(), point.size(), preview))
            });
            self.list_scaled_outside_frame += usize::from(!inside);
        }
    }
}

/// Whether `metadata`'s focus point is its list's primary, as
/// `Metadata::focus` documents: both absent, or the focus point at the
/// primary's centre, with its area and whether focus was confirmed, in the
/// list's frame and with its origin. A list without a primary, which gives
/// no focus point, does not pass.
fn list_gives_focus(metadata: &Metadata) -> bool {
    match (metadata.focus(), metadata.focus_points()) {
        (None, None) => true,
        (Some(focus), Some(list)) => list.primary().is_some_and(|primary| {
            focus.center() == primary.center()
                && focus.size() == primary.size()
                && focus.in_focus() == primary.in_focus()
                && focus.frame() == list.frame()
                && focus.origin() == list.origin()
        }),
        _ => false,
    }
}

/// Whether the points of `list`, placed from a table, are points of
/// `layout`, the body's layout from the same table, as
/// `Metadata::focus_points` and `Metadata::focus_layout` document: each
/// with a number, in rising order, and with the layout's box, its centre
/// the layout's moved by the origin of the centred window that the list's
/// frame is in the layout's frame. On a file that is no crop the two
/// frames are the same and nothing moves; on a crop the window is the
/// layout's frame less twice the origin on each side, so the difference
/// of the two frames is even on both axes and the origin is half of it.
/// Written out here, as `lies_inside` is, so that the test does not take
/// the library's word for it.
fn placed_from(list: &FocusPoints, layout: &FocusLayout) -> bool {
    let (window, frame) = (list.frame(), layout.frame());
    let (Some(across), Some(down)) = (
        frame.width.checked_sub(window.width),
        frame.height.checked_sub(window.height),
    ) else {
        return false;
    };
    if across % 2 != 0 || down % 2 != 0 {
        return false;
    }
    let origin = Point {
        x: across / 2,
        y: down / 2,
    };
    // No point is numbered 0, so every first number rises above it.
    let mut previous = 0;
    list.points().iter().all(|point| {
        let Some(number) = point.number() else {
            return false;
        };
        let rises = number > previous;
        previous = number;
        rises
            && layout.point(number).is_some_and(|table| {
                let center = table.center();
                point.size() == Some(table.size())
                    && center.x.checked_sub(origin.x) == Some(point.center().x)
                    && center.y.checked_sub(origin.y) == Some(point.center().y)
            })
    })
}

/// Whether `center` lies inside `frame`, edges included, and so does the
/// area of `size` around it when there is one: on each axis, from the
/// centre less half the side, rounded up, to the centre plus that half
/// lies between 0 and the frame's side. Written out here, from the rule
/// that `Focus::new` and `Focus::with_size` document, rather than asked
/// of the library, so that the test does not take the library's word for
/// it.
fn lies_inside(center: Point, size: Option<Size>, frame: Size) -> bool {
    let fits = |center: u32, side: u32, frame: u32| {
        let half = side.div_ceil(2);
        half <= center && u64::from(center) + u64::from(half) <= u64::from(frame)
    };
    center.x <= frame.width
        && center.y <= frame.height
        && size.is_none_or(|size| {
            fits(center.x, size.width, frame.width) && fits(center.y, size.height, frame.height)
        })
}

/// What the run found in the files whose extension names one format, in
/// counts.
#[derive(Default)]
struct Tally {
    seen: usize,
    /// Files that could not be opened for reading.
    unreadable: usize,
    opened: usize,
    failures: BTreeMap<String, usize>,
    no_preview: usize,
    not_whole_jpeg: usize,
    /// NEF files whose largest preview ends before its declared length in
    /// the shape [`ends_early`] accepts: a whole JPEG with up to
    /// [`LEFTOVERS_AFTER_END`] bytes of other data after its end marker.
    /// Printed, not asserted: the D600 and the D3200 write such a preview
    /// about once in 500 files, and a person reads the count, as every
    /// other count here.
    ends_early: usize,
    /// Files that opened as JPEG whose largest preview, the whole file,
    /// holds other data than padding after its last end marker. Printed,
    /// not asserted: the data belongs to the file.
    other_data_after_end: usize,
    /// The formats the files opened as, which differ from the one their
    /// extension names when a file was renamed.
    formats: BTreeMap<Format, usize>,
    /// How many files have each number of previews.
    preview_counts: BTreeMap<usize, usize>,
    /// How many previews have each size, over every preview of every file.
    preview_sizes: BTreeMap<(u32, u32), usize>,
    sizes: BTreeMap<(u32, u32), usize>,
    orientations: BTreeMap<String, usize>,
    metadata: MetadataTally,
    settings: SettingsTally,
    focus: FocusTally,
    /// Files whose metadata gives an orientation other than their largest
    /// preview's: both come from the same tag, so this must stay zero.
    orientation_disagreements: usize,
    /// Bytes read by opening each file, before any preview was asked for.
    open_bytes: Vec<u64>,
    shares: Vec<f64>,
}

impl Tally {
    fn fail(&mut self, kind: impl std::fmt::Debug) {
        *self.failures.entry(format!("{kind:?}")).or_default() += 1;
    }

    /// Prints the counts as one section, under a heading that names
    /// `format`, the format the files' extension names.
    fn print(&self, format: Format) {
        let mut shares = self.shares.clone();
        shares.sort_by(f64::total_cmp);
        let mut open_bytes = self.open_bytes.clone();
        open_bytes.sort_unstable();
        let m = &self.metadata;
        println!();
        println!(
            "== {format}: {} files whose extension names it ==",
            self.seen
        );
        println!("unreadable: {}", self.unreadable);
        println!("opened: {}", self.opened);
        println!("failures by kind: {:?}", self.failures);
        println!("no preview: {}", self.no_preview);
        println!("not a whole JPEG: {}", self.not_whole_jpeg);
        // Only a NEF file's preview can end early, and only a file that
        // opened as JPEG can have other data after the end.
        if self.formats.contains_key(&Format::Nef) {
            println!(
                "previews that end before their declared length, by up to {LEFTOVERS_AFTER_END} bytes: {}",
                self.ends_early
            );
        }
        if self.formats.contains_key(&Format::Jpeg) {
            println!(
                "files with other data after the last end marker: {}",
                self.other_data_after_end
            );
        }
        println!("formats the files opened as: {:?}", self.formats);
        println!("files by number of previews: {:?}", self.preview_counts);
        println!("sizes of all previews: {:?}", self.preview_sizes);
        println!("largest-preview sizes: {:?}", self.sizes);
        println!("largest-preview orientations: {:?}", self.orientations);
        println!(
            "files with a make: {}, a model: {}, software: {}, a serial number: {}",
            m.make, m.model, m.software, m.serial_number
        );
        println!(
            "files with a capture time: {}, of which with sub-seconds: {}, with an offset from UTC: {}",
            m.capture_time, m.with_subseconds, m.with_offset
        );
        println!(
            "files whose capture time, sub-seconds included, another file shares: {}",
            m.shared_capture_times()
        );
        println!(
            "files with an ISO: {}, an exposure time: {}, an f-number: {}, a focal length: {}, a focal length in 35 mm terms: {}, an exposure compensation: {}",
            m.iso,
            m.exposure_time,
            m.f_number,
            m.focal_length,
            m.focal_length_in_35mm_film,
            m.exposure_bias
        );
        println!(
            "files with a lens: {}, of which with a model: {}, with a focal-length range: {}",
            m.lens, m.lens_model, m.lens_focal_range
        );
        println!("raw sizes: {:?}", m.raw_sizes);
        println!("image sizes: {:?}", m.image_sizes);
        let s = &self.settings;
        println!(
            "files with an exposure program: {}, by program: {:?}",
            s.exposure_program, s.exposure_programs
        );
        println!(
            "files with a metering mode: {}, by mode: {:?}",
            s.metering_mode, s.metering_modes
        );
        println!(
            "files with a flash: {}, of which fired: {}, not fired: {}; by mode: {:?}, without a mode: {}",
            s.flash,
            s.flash_fired,
            s.flash - s.flash_fired,
            s.flash_modes,
            s.flash_without_mode
        );
        println!(
            "files with a white balance mode: {}, by mode: {:?}",
            s.white_balance_mode, s.white_balance_modes
        );
        match s.color_temperature_range {
            Some((lowest, highest)) => println!(
                "files with a colour temperature: {}, lowest: {lowest} K, highest: {highest} K",
                s.color_temperature
            ),
            None => println!("files with a colour temperature: {}", s.color_temperature),
        }
        // Each pair as the library gives it, written as fractions, such as
        // `(-24/4, 0/4): 3`, in the braces a map prints in.
        let fine_tunes: Vec<String> = s
            .fine_tunes
            .iter()
            .map(|(((ab_num, ab_den), (gm_num, gm_den)), count)| {
                format!("({ab_num}/{ab_den}, {gm_num}/{gm_den}): {count}")
            })
            .collect();
        println!(
            "files with a white balance fine-tuning: {}, by shifts, amber to blue then green to magenta: {{{}}}",
            s.fine_tune,
            fine_tunes.join(", ")
        );
        println!(
            "files with a shutter: {}, by shutter: {:?}",
            s.shutter, s.shutters
        );
        println!(
            "files with an electronic front curtain setting: {}, of which on: {}, off: {}",
            s.electronic_front_curtain,
            s.electronic_front_curtain_on,
            s.electronic_front_curtain - s.electronic_front_curtain_on
        );
        println!(
            "files with an image area: {}, by area: {:?}",
            s.image_area, s.image_areas
        );
        println!(
            "files with a raw image size: {}, by size: {:?}",
            s.raw_image_size, s.raw_image_sizes
        );
        println!(
            "files with a colour space: {}, by colour space: {:?}",
            s.color_space, s.color_spaces
        );
        let f = &self.focus;
        println!(
            "files with a focus point: {}, without: {}",
            f.with_point, f.without_point
        );
        println!("focus points by origin: {:?}", f.origins);
        println!(
            "focus confirmed: true {}, false {}, unknown {}",
            f.confirmed, f.missed, f.unknown
        );
        println!("focus frames: {:?}", f.frames);
        println!("distinct focus area sizes: {}", f.area_sizes.len());
        println!(
            "files with a list of the focus points used: {}, of which with a primary: {}",
            f.with_list, f.with_primary
        );
        println!(
            "files by number of points in their list: {:?}",
            f.list_sizes
        );
        println!(
            "points in the lists: {}, of which with a number: {}",
            f.list_points, f.numbered_points
        );
        println!(
            "lists of one point placed from a table, a single-point shot or a mask naming no other point, which the library does not tell apart: {}",
            f.single_placed
        );
        println!(
            "lists with an area mode: {}, by mode: {:?}",
            f.with_list - f.without_area_mode,
            f.area_modes
        );
        println!(
            "files with a layout of the body's autofocus points: {}, by number of points: {:?}",
            f.with_layout, f.layout_sizes
        );
        println!(
            "files with a layout and no focus point, from manual focus, a point number of 0, a crop the table does not confirm, a crop-mode JPEG smaller than the crop, a JPEG cut to another shape or a MakerNote missing or unreadable, which the library does not tell apart: {}",
            f.layout_without_point
        );
        println!(
            "files whose focus point lies outside its frame, area included: {}",
            f.outside_frame
        );
        println!(
            "files whose focus frame differs from their image size: {}",
            f.frame_not_image_size
        );
        println!(
            "files whose focus point could not be scaled onto their largest preview: {}",
            f.not_scaled
        );
        println!(
            "files whose focus point, scaled onto their largest preview, lies outside it, area included: {}",
            f.scaled_outside_frame
        );
        println!(
            "files whose focus point is not their list's primary, or whose list has none: {}",
            f.list_not_focus
        );
        println!(
            "files whose recorded position is not a list of one point without a number: {}",
            f.recorded_not_one_point
        );
        println!(
            "files with points placed from a table and no layout: {}",
            f.placed_without_layout
        );
        println!(
            "files whose layout is not for the Model they name: {}",
            f.layout_not_for_model
        );
        println!(
            "files whose points placed from a table are not their layout's: {}",
            f.placed_not_from_layout
        );
        println!(
            "files with a point of their list outside the list's frame, area included: {}",
            f.list_outside_frame
        );
        println!(
            "files whose list could not be scaled onto their largest preview, or lies outside it once scaled, area included: {}",
            f.list_scaled_outside_frame
        );
        println!(
            "files whose metadata orientation differs from their largest preview's: {}",
            self.orientation_disagreements
        );
        if let (Some(median), Some(max)) = (open_bytes.get(open_bytes.len() / 2), open_bytes.last())
        {
            println!(
                "bytes read by open alone, median: {median} ({} KiB), maximum: {max} ({} KiB)",
                median / 1024,
                max / 1024
            );
        }
        if let (Some(median), Some(max)) = (shares.get(shares.len() / 2), shares.last()) {
            println!(
                "bytes read by open and the largest preview, as a share of the file, median: {median:.2} %, maximum: {max:.2} %"
            );
        }
    }
}

/// Calls `visit` for every file under `dir`, without following symlinks;
/// returns how many entries could not be read.
fn walk(dir: &Path, visit: &mut impl FnMut(PathBuf)) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 1;
    };
    let mut unreadable = 0;
    for entry in entries {
        let Ok(entry) = entry else {
            unreadable += 1;
            continue;
        };
        let Ok(kind) = entry.file_type() else {
            unreadable += 1;
            continue;
        };
        if kind.is_dir() {
            unreadable += walk(&entry.path(), visit);
        } else if kind.is_file() {
            visit(entry.path());
        }
    }
    unreadable
}

/// What the bytes of a largest preview hold, as `wholeness` finds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wholeness {
    /// A whole JPEG: the end marker, then nothing but padding.
    Whole,
    /// A whole JPEG in a NEF file's preview that ends before its declared
    /// length in the shape [`ends_early`] accepts: its own end marker is
    /// followed by other data, at most [`LEFTOVERS_AFTER_END`] bytes of it.
    EndsEarly,
    /// A whole JPEG file whose last end marker is followed by data other
    /// than padding.
    OtherDataAfterEnd,
    /// Not a whole JPEG.
    NotWhole,
}

/// How many bytes of other data may follow the end marker of a NEF file's
/// preview before the preview is not whole. The D600 and the D3200 write,
/// about once in 500 files, a preview whose declared length is 2 more
/// than a multiple of 512 while the image ends 513 bytes earlier, with
/// the write buffer's leftovers after it, as
/// `docs/formats/nef/container-and-previews.md` records in its section
/// on generational quirks; the image is whole and a decoder stops at the
/// marker. One byte more is not a case any file has shown.
const LEFTOVERS_AFTER_END: usize = 513;

/// Whether `bytes`, a RAW file's preview of `format` that holds other
/// data after its last end marker, has the shape of the D600's and the
/// D3200's habit, and so ends early rather than failing. Three things
/// must hold, each one a part of what those files show: the file is a
/// NEF; the declared length is 2 more than a multiple of 512, as a count
/// of 512-byte blocks plus 2 would be; and at most
/// [`LEFTOVERS_AFTER_END`] bytes follow the image's own end marker, the
/// one [`main_image_end`] finds by walking the segments from the start,
/// so that, within this check, an end marker lying among the leftovers,
/// which the write buffer may well hold, neither shortens the measure
/// nor passes a longer overrun. (Leftovers that end in an end marker, or
/// in one and then `FF` bytes, never reach this check: [`wholeness`]
/// judges a RAW preview whole by its last end marker, as before.) A scan
/// cut short and followed by leftovers that hold an end marker cannot be
/// told from a whole one without decoding the image; the three
/// conditions together make that a rare coincidence. Any other raw
/// preview, and any other shape, stays as strict as before.
fn ends_early(bytes: &[u8], format: Format) -> bool {
    format == Format::Nef
        && bytes.len() % 512 == 2
        && main_image_end(bytes)
            .is_some_and(|own_end| bytes.len() - (own_end + 2) <= LEFTOVERS_AFTER_END)
}

/// Whether `bytes`, the largest preview of a file that opened as `format`,
/// are a whole JPEG: `expected_len` bytes long, as the preview says,
/// starting with the start marker `FF D8 FF`, and holding the end marker
/// `FF D9` with nothing after its last one but padding, or, in a file that
/// opened as JPEG, with other data after it, or, in a NEF file's preview
/// of the shape [`ends_early`] accepts, with leftovers after it.
///
/// A file that opened as JPEG must also hold its own image's end marker,
/// which [`main_image_end`] looks for after the image's first scan. Its
/// Exif segment may hold a thumbnail, and its MakerNote a preview, each
/// with an end marker of its own; a file cut inside its image, or whose
/// bytes after the Exif segment were lost to zeros, still holds those, and
/// a thumbnail's end marker must not stand for the image's. A RAW file's
/// preview is judged whole by its last end marker alone: it is one JPEG
/// that the reader found by its offset and length, and its last bytes are
/// the ones that show whether that range holds all of it. Only when other
/// data follows that marker does [`ends_early`] look for the image's own
/// end marker, to measure the leftovers from it.
///
/// Two kinds of padding are known. Some Sony cameras write `FF` fill
/// bytes after the end marker, inside the length the file declares: the
/// main preview of the NEX-5's and SLT-A55V's ARW files is followed by 51
/// of them, as `docs/formats/arw/container-and-previews.md` records under
/// "Bytes after the end-of-image marker". So `FF` is padding in any
/// format. And a JPEG file's largest preview is, on camera files, the file
/// itself, every byte of it, which some Sony cameras pad with zeros after
/// the last end marker, up to a multiple of 64 KiB. So zeros are padding
/// too, for a file that opened as JPEG and only for one: no preview a RAW
/// file embeds is known to end in zeros, so zeros there are other data,
/// like any other byte.
///
/// Since that preview is the whole file, other data after its last end
/// marker, such as a record a website appends to the files it serves,
/// belongs to the file, not to a range the reader chose: it is counted
/// apart, not as a failure. In a RAW file's preview, other data is a
/// failure, unless the preview has the shape the D600 and the D3200
/// write, which [`ends_early`] judges: then the preview ends early, which
/// is counted apart too.
fn wholeness(bytes: &[u8], expected_len: u64, format: Format) -> Wholeness {
    let starts_whole = u64::try_from(bytes.len()).is_ok_and(|len| len == expected_len)
        && bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
    if !starts_whole {
        return Wholeness::NotWhole;
    }
    let jpeg_file = format == Format::Jpeg;
    if jpeg_file && main_image_end(bytes).is_none() {
        return Wholeness::NotWhole;
    }
    // In a JPEG file this is at or after its image's end marker.
    let Some(end_marker) = bytes.windows(2).rposition(|pair| pair == [0xFF, 0xD9]) else {
        return Wholeness::NotWhole;
    };
    let is_padding = |byte: u8| byte == 0xFF || (byte == 0x00 && jpeg_file);
    let after_end = &bytes[end_marker + 2..];
    if after_end.iter().all(|&byte| is_padding(byte)) {
        Wholeness::Whole
    } else if jpeg_file {
        Wholeness::OtherDataAfterEnd
    } else if ends_early(bytes, format) {
        Wholeness::EndsEarly
    } else {
        Wholeness::NotWhole
    }
}

/// Where the end marker of the image of `bytes`, a JPEG file or a RAW
/// file's preview, lies, or `None` when it has none. Walked here rather
/// than asked of the library, so that the test does not take the
/// library's word for it.
///
/// The walk goes from the start marker over the marker segments to the
/// first scan header (SOS, `FF DA`): each segment is `FF`, its code and a
/// two-byte big-endian length that counts itself and the payload, and
/// skipping a segment by its length steps over everything inside it, a
/// thumbnail's end marker included. `FF` fill bytes may come before a
/// marker, and a restart marker (`FF D0` to `FF D7`) or TEM (`FF 01`) has
/// no length. Anything else ends the walk with `None`: a byte other than
/// `FF` where a marker should be, a second start marker, an end marker
/// before any scan, a length below 2 or running past the end, or no scan
/// at all. Every step moves forward, so the walk ends.
///
/// After the scan header comes the entropy-coded data, in which an `FF`
/// byte is followed by a stuffed `00` or a restart marker, neither of them
/// `FF D9`; so the first `FF D9` after the scan header ends the image. A
/// progressive JPEG's later scans and their tables lie before it.
fn main_image_end(bytes: &[u8]) -> Option<usize> {
    let mut pos = 2;
    let scan_start = loop {
        if *bytes.get(pos)? != 0xFF {
            return None;
        }
        match *bytes.get(pos + 1)? {
            0xFF => pos += 1,
            0x01 | 0xD0..=0xD7 => pos += 2,
            0x00 | 0xD8 | 0xD9 => return None,
            code => {
                let len = usize::from(u16::from_be_bytes([
                    *bytes.get(pos + 2)?,
                    *bytes.get(pos + 3)?,
                ]));
                let next = pos + 2 + len;
                if len < 2 || next > bytes.len() {
                    return None;
                }
                if code == 0xDA {
                    break next;
                }
                pos = next;
            }
        }
    };
    let offset = bytes[scan_start..]
        .windows(2)
        .position(|pair| pair == [0xFF, 0xD9])?;
    Some(scan_start + offset)
}

/// Opens `path`, counts its metadata, reads its largest preview and records
/// what happened.
fn check(path: &Path, tally: &mut Tally) {
    let (Ok(file), Ok(metadata)) = (File::open(path), fs::metadata(path)) else {
        tally.unreadable += 1;
        return;
    };
    let bytes_read = Rc::new(Cell::new(0));
    let reader = CountingReader {
        file,
        bytes_read: Rc::clone(&bytes_read),
    };
    let mut raw = match RawFile::from_reader(reader) {
        Ok(raw) => raw,
        Err(error) => {
            tally.fail(error.kind());
            return;
        }
    };
    tally.opened += 1;
    tally.open_bytes.push(bytes_read.get());
    let format = raw.format();
    *tally.formats.entry(format).or_default() += 1;
    *tally
        .preview_counts
        .entry(raw.previews().len())
        .or_default() += 1;
    for preview in raw.previews() {
        *tally
            .preview_sizes
            .entry((preview.width(), preview.height()))
            .or_default() += 1;
    }
    tally.metadata.record(raw.metadata());
    tally.settings.record(raw.metadata());
    tally.focus.record(raw.metadata());
    let Some(preview) = raw.largest_preview() else {
        tally.no_preview += 1;
        return;
    };
    if preview.orientation() != raw.metadata().orientation() {
        tally.orientation_disagreements += 1;
    }
    tally.focus.record_scaled(raw.metadata(), preview.size());
    *tally
        .sizes
        .entry((preview.width(), preview.height()))
        .or_default() += 1;
    *tally
        .orientations
        .entry(format!("{:?}", preview.orientation()))
        .or_default() += 1;
    match raw.read_preview(&preview) {
        Ok(bytes) => match wholeness(&bytes, preview.byte_len(), format) {
            Wholeness::Whole => {}
            Wholeness::EndsEarly => tally.ends_early += 1,
            Wholeness::OtherDataAfterEnd => tally.other_data_after_end += 1,
            Wholeness::NotWhole => tally.not_whole_jpeg += 1,
        },
        Err(error) => {
            tally.fail(error.kind());
            return;
        }
    }
    if metadata.len() > 0 {
        tally
            .shares
            .push(100.0 * bytes_read.get() as f64 / metadata.len() as f64);
    }
}

#[test]
#[ignore = "reads a folder of your own camera files; see the module documentation"]
fn every_file_of_the_local_corpus_opens_and_gives_a_whole_jpeg() {
    let Some(dir) = env::var_os("RAW_FILES_LOCAL_CORPUS").filter(|dir| !dir.is_empty()) else {
        println!("RAW_FILES_LOCAL_CORPUS is not set; nothing to check");
        return;
    };
    // The walk counts a folder it cannot read and goes on, which for the
    // folder itself would leave nothing to check and every count at zero.
    if let Err(error) = fs::read_dir(&dir) {
        panic!("the folder RAW_FILES_LOCAL_CORPUS names cannot be read: {error}");
    }
    let mut paths = Vec::new();
    let mut apple_double = 0;
    let unreadable_entries = walk(Path::new(&dir), &mut |path| {
        let format = path
            .extension()
            .and_then(|ext| Format::from_extension(&ext.to_string_lossy()));
        let Some(format) = format else {
            return;
        };
        // macOS writes a file's extended attributes, on a disk that cannot
        // hold them, to a file of the same name with `._` before it: not a
        // photo, though its extension names a format.
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("._"))
        {
            apple_double += 1;
        } else {
            paths.push((format, path));
        }
    });
    // One tally for each format the extensions name, in the order of the
    // formats, so that a folder of one format prints one section.
    let mut tallies: BTreeMap<Format, Tally> = BTreeMap::new();
    for (format, path) in paths {
        let tally = tallies.entry(format).or_default();
        tally.seen += 1;
        check(&path, tally);
    }

    println!(
        "files of a format raw-files reads: {}",
        tallies.values().map(|tally| tally.seen).sum::<usize>()
    );
    println!(
        "AppleDouble files skipped, whose name starts with `._` and whose extension names a format: {apple_double}"
    );
    println!("entries that could not be read: {unreadable_entries}");
    for (format, tally) in &tallies {
        tally.print(*format);
    }

    assert!(
        !tallies.is_empty(),
        "no file of a format raw-files reads under RAW_FILES_LOCAL_CORPUS"
    );
    assert_eq!(unreadable_entries, 0, "some entries could not be read");
    for (format, tally) in &tallies {
        assert_eq!(tally.unreadable, 0, "some {format} files could not be read");
        assert!(
            tally.failures.is_empty(),
            "some {format} files failed to open or read"
        );
        assert_eq!(tally.no_preview, 0, "some {format} files have no preview");
        assert_eq!(
            tally.orientation_disagreements, 0,
            "some {format} files' metadata orientation differs from their preview's"
        );
        assert_eq!(
            tally.not_whole_jpeg, 0,
            "some largest previews of {format} files are not whole JPEGs"
        );
        assert_eq!(
            tally.focus.outside_frame, 0,
            "some focus points of {format} files lie outside their frame"
        );
        assert_eq!(
            tally.focus.not_scaled, 0,
            "some focus points of {format} files could not be scaled onto their largest preview"
        );
        assert_eq!(
            tally.focus.scaled_outside_frame, 0,
            "some focus points of {format} files, scaled onto their largest preview, lie outside it"
        );
        let f = &tally.focus;
        assert_eq!(
            f.list_outside_frame, 0,
            "some {format} files have a point of their list outside the list's frame"
        );
        assert_eq!(
            f.list_scaled_outside_frame, 0,
            "some {format} files' list could not be scaled onto their largest preview, or lies outside it"
        );
        assert_eq!(
            f.list_not_focus, 0,
            "some {format} files' focus point is not their list's primary"
        );
        assert_eq!(
            f.recorded_not_one_point, 0,
            "some recorded positions of {format} files are not a list of one point without a number"
        );
        assert_eq!(
            f.placed_without_layout, 0,
            "some {format} files have points placed from a table and no layout"
        );
        assert_eq!(
            f.layout_not_for_model, 0,
            "some {format} files' layout is not for the Model they name"
        );
        assert_eq!(
            f.placed_not_from_layout, 0,
            "some {format} files' points placed from a table are not their layout's"
        );
    }
}

#[test]
fn the_settings_tally_counts_the_makernote_settings() {
    use raw_files::{
        CameraSettings, FineTune, ImageArea, RawImageSize, Shutter, SignedRational, WhiteBalance,
    };

    let shift = |numerator, denominator| SignedRational {
        numerator,
        denominator,
    };
    let zero = shift(0, 4);
    let towards_amber = FineTune::new(shift(-24, 4), zero);
    // Three files with a colour temperature each, neither the first the
    // lowest nor the highest, so that both ends of the range must move;
    // the first two with the same shift, the third with another; the
    // electronic front curtain setting on, off and on, so that on and off
    // differ in count. A fourth file has none of the settings.
    let first = CameraSettings::default()
        .with_white_balance(
            WhiteBalance::default()
                .with_color_temperature(4_000)
                .with_fine_tune(towards_amber),
        )
        .with_electronic_front_curtain_setting(true)
        .with_shutter(Shutter::Mechanical)
        .with_image_area(ImageArea::ApsC)
        .with_raw_image_size(RawImageSize::Small);
    let second = CameraSettings::default()
        .with_white_balance(
            WhiteBalance::default()
                .with_color_temperature(3_050)
                .with_fine_tune(towards_amber),
        )
        .with_electronic_front_curtain_setting(false);
    let third = CameraSettings::default()
        .with_white_balance(
            WhiteBalance::default()
                .with_color_temperature(5_200)
                .with_fine_tune(FineTune::new(zero, zero)),
        )
        .with_electronic_front_curtain_setting(true);
    let fourth = CameraSettings::default();
    let mut tally = SettingsTally::default();
    for settings in [first, second, third, fourth.clone()] {
        tally.record(&Metadata::default().with_camera_settings(settings));
    }

    assert_eq!(tally.color_temperature, 3);
    assert_eq!(tally.color_temperature_range, Some((3_050, 5_200)));

    assert_eq!(tally.fine_tune, 3);
    assert_eq!(
        tally.fine_tunes,
        BTreeMap::from([(((-24, 4), (0, 4)), 2), (((0, 4), (0, 4)), 1)])
    );

    assert_eq!(tally.electronic_front_curtain, 3);
    assert_eq!(tally.electronic_front_curtain_on, 2);

    assert_eq!(
        (tally.shutter, tally.image_area, tally.raw_image_size),
        (1, 1, 1)
    );
    assert_eq!(tally.shutters, BTreeMap::from([("Mechanical", 1)]));
    assert_eq!(tally.image_areas, BTreeMap::from([("APS-C", 1)]));
    assert_eq!(tally.raw_image_sizes, BTreeMap::from([("Small", 1)]));

    // Files without a colour temperature leave the range empty.
    let mut without_color_temperature = SettingsTally::default();
    without_color_temperature.record(&Metadata::default().with_camera_settings(fourth));
    assert_eq!(without_color_temperature.color_temperature, 0);
    assert_eq!(without_color_temperature.color_temperature_range, None);
}

/// Three points of a 39-point layout in a 6016 × 4016 frame, from the
/// record of the Nikon D600 in the NEF format notes: 1, the centre, 13 and
/// 38, each with its 164 × 164 box.
static THREE_POINTS: [raw_files::LayoutPoint; 3] = {
    use raw_files::LayoutPoint;
    const BOX: Size = Size {
        width: 164,
        height: 164,
    };
    [
        LayoutPoint::new(1, Point { x: 3008, y: 2007 }, BOX),
        LayoutPoint::new(13, Point { x: 2768, y: 1503 }, BOX),
        LayoutPoint::new(38, Point { x: 1812, y: 1705 }, BOX),
    ]
};

/// A list placed from a table in `frame`: one point for each number and
/// centre of `points`, in their order, each with the 164 × 164 box, the
/// one at index `primary` the primary.
fn placed_list(frame: Size, points: &[(u8, Point)], primary: usize) -> FocusPoints {
    use raw_files::FocusPoint;
    let size = Size {
        width: 164,
        height: 164,
    };
    let mut list = FocusPoints::new(frame, FocusOrigin::Table).unwrap();
    for &(number, center) in points {
        list = list
            .with_point(FocusPoint::new(center).with_number(number).with_size(size))
            .unwrap();
    }
    list.with_primary(primary).unwrap()
}

#[test]
fn the_focus_tally_counts_the_lists_and_layouts_and_checks_them_against_each_other() {
    use raw_files::{AfAreaMode, Focus, FocusPoint};

    let full = Size {
        width: 6016,
        height: 4016,
    };
    let layout = FocusLayout::new("NIKON D600", full, &THREE_POINTS).unwrap();
    let d600 = || {
        Metadata::default()
            .with_model("NIKON D600")
            .with_focus_layout(layout)
    };
    // An auto-area shot naming points 13 and 38, the primary 38; a DX
    // shot naming 38 alone, in the 3936 × 2624 window of origin
    // (1040, 696); a recorded position on a body without a table; and a
    // shot of the body with a table that names no point.
    let auto_area = placed_list(
        full,
        &[
            (13, Point { x: 2768, y: 1503 }),
            (38, Point { x: 1812, y: 1705 }),
        ],
        1,
    )
    .with_area_mode(AfAreaMode::Auto);
    let window = Size {
        width: 3936,
        height: 2624,
    };
    let dx = placed_list(window, &[(38, Point { x: 772, y: 1009 })], 0)
        .with_area_mode(AfAreaMode::Single);
    let recorded = Focus::new(full, Point { x: 343, y: 343 })
        .unwrap()
        .with_size(Size {
            width: 540,
            height: 540,
        })
        .unwrap();
    let files = [
        d600().with_focus_points(auto_area),
        d600().with_focus_points(dx),
        Metadata::default().with_focus(recorded),
        d600(),
    ];
    let mut tally = FocusTally::default();
    for metadata in &files {
        tally.record(metadata);
        tally.record_scaled(metadata, full);
    }

    assert_eq!((tally.with_point, tally.without_point), (3, 1));
    assert_eq!(
        tally.origins,
        BTreeMap::from([("Recorded", 1), ("Table", 2)])
    );
    assert_eq!((tally.with_list, tally.with_primary), (3, 3));
    assert_eq!(tally.list_sizes, BTreeMap::from([(1, 2), (2, 1)]));
    assert_eq!((tally.list_points, tally.numbered_points), (4, 3));
    assert_eq!(tally.single_placed, 1);
    assert_eq!(
        tally.area_modes,
        BTreeMap::from([("Auto area", 1), ("Single point", 1)])
    );
    assert_eq!(tally.without_area_mode, 1);
    assert_eq!(
        (tally.with_layout, tally.layout_sizes.clone()),
        (3, BTreeMap::from([(3, 3)]))
    );
    assert_eq!(tally.layout_without_point, 1);
    let failures = |tally: &FocusTally| {
        [
            tally.list_not_focus,
            tally.recorded_not_one_point,
            tally.placed_without_layout,
            tally.layout_not_for_model,
            tally.placed_not_from_layout,
            tally.list_outside_frame,
            tally.list_scaled_outside_frame,
        ]
    };
    assert_eq!(failures(&tally), [0; 7]);

    // Each of these breaks one rule, and is counted under it alone. The
    // rules of the points inside the frame and the preview cannot be
    // broken here: `FocusPoints::with_point` refuses such a point.
    let one = |center| [(38, center)];
    let at_38 = Point { x: 1812, y: 1705 };
    let unmarked = FocusPoints::new(full, FocusOrigin::Table)
        .unwrap()
        .with_point(FocusPoint::new(at_38).with_number(38).with_size(Size {
            width: 164,
            height: 164,
        }))
        .unwrap();
    let two_recorded = FocusPoints::new(full, FocusOrigin::Recorded)
        .unwrap()
        .with_point(FocusPoint::new(at_38))
        .unwrap()
        .with_point(FocusPoint::new(at_38))
        .unwrap()
        .with_primary(0)
        .unwrap();
    let numbered_recorded = FocusPoints::new(full, FocusOrigin::Recorded)
        .unwrap()
        .with_point(FocusPoint::new(at_38).with_number(38))
        .unwrap()
        .with_primary(0)
        .unwrap();
    let off_by_one = Point { x: 1813, y: 1705 };
    let out_of_order = placed_list(full, &[(38, at_38), (13, Point { x: 2768, y: 1503 })], 0);
    // A window whose difference from the frame is odd across, which no
    // origin gives.
    let odd_window = Size {
        width: 3937,
        height: 2624,
    };
    let broken: [(Metadata, usize); 9] = [
        (d600().with_focus_points(unmarked), 0),
        (Metadata::default().with_focus_points(two_recorded), 1),
        (Metadata::default().with_focus_points(numbered_recorded), 1),
        (
            Metadata::default().with_focus_points(placed_list(full, &one(at_38), 0)),
            2,
        ),
        (
            Metadata::default()
                .with_model("NIKON D610")
                .with_focus_layout(layout),
            3,
        ),
        (
            d600().with_focus_points(placed_list(full, &one(off_by_one), 0)),
            4,
        ),
        (d600().with_focus_points(out_of_order), 4),
        (
            d600().with_focus_points(placed_list(odd_window, &one(Point { x: 772, y: 1009 }), 0)),
            4,
        ),
        (
            d600().with_focus_points(
                FocusPoints::new(full, FocusOrigin::Table)
                    .unwrap()
                    .with_point(FocusPoint::new(at_38).with_size(Size {
                        width: 164,
                        height: 164,
                    }))
                    .unwrap()
                    .with_primary(0)
                    .unwrap(),
            ),
            4,
        ),
    ];
    for (index, (metadata, rule)) in broken.iter().enumerate() {
        let mut tally = FocusTally::default();
        tally.record(metadata);
        tally.record_scaled(metadata, full);
        let mut expected = [0; 7];
        expected[*rule] = 1;
        assert_eq!(failures(&tally), expected, "broken file {index}");
    }
}

// The tests of `wholeness` and `main_image_end`, on JPEG files in
// miniature built here: run with every `cargo test`, since they need no
// folder of files.

/// A marker segment: `FF`, `code`, the two-byte length and `payload`.
fn segment(code: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len() + 2).expect("a payload of a segment's size");
    let mut bytes = vec![0xFF, code];
    bytes.extend_from_slice(&len.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

/// An Exif segment's payload in miniature: its header, then a thumbnail
/// with a scan header and an end marker of its own, as a camera's holds.
const EXIF_WITH_THUMBNAIL: &[u8] = &[
    b'E', b'x', b'i', b'f', 0, 0, // the Exif header
    0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0x12, 0x34, 0xFF, 0xD9, // the thumbnail
];

/// A JPEG file in miniature: the start marker, an Exif segment holding a
/// thumbnail, a scan header, `scan` as the entropy-coded data, and the end
/// marker.
fn jpeg_file(scan: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];
    bytes.extend(segment(0xE1, EXIF_WITH_THUMBNAIL));
    bytes.extend(segment(0xDA, &[1, 1, 0x00, 0, 63, 0]));
    bytes.extend_from_slice(scan);
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// Where the scan header starts in the bytes of `jpeg_file`: after the
/// start marker and the Exif segment, its marker and length included.
const SCAN_HEADER_AT: usize = 2 + 4 + EXIF_WITH_THUMBNAIL.len();

/// Entropy-coded data in miniature.
const SCAN: &[u8] = &[0x12, 0x34, 0x56, 0x78];

/// `wholeness` of all of `bytes`, which are as long as expected.
fn wholeness_of(bytes: &[u8], format: Format) -> Wholeness {
    wholeness(bytes, bytes.len() as u64, format)
}

#[test]
fn a_whole_jpeg_file_is_whole() {
    assert_eq!(
        wholeness_of(&jpeg_file(SCAN), Format::Jpeg),
        Wholeness::Whole
    );
}

#[test]
fn zeros_after_a_jpeg_files_end_are_padding() {
    let mut bytes = jpeg_file(SCAN);
    bytes.extend_from_slice(&[0x00; 64]);
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::Whole);
}

#[test]
fn ff_fill_after_a_jpeg_files_end_is_padding() {
    let mut bytes = jpeg_file(SCAN);
    bytes.extend_from_slice(&[0xFF; 51]);
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::Whole);
}

#[test]
fn ff_fill_before_a_marker_is_stepped_over() {
    let whole = jpeg_file(SCAN);
    // Two fill bytes before the scan header.
    let (before, after) = whole.split_at(SCAN_HEADER_AT);
    let bytes = [before, &[0xFF, 0xFF], after].concat();
    assert_eq!(main_image_end(&bytes), Some(bytes.len() - 2));
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::Whole);
}

#[test]
fn other_data_after_a_jpeg_files_end_is_counted_apart() {
    let mut bytes = jpeg_file(SCAN);
    bytes.extend_from_slice(b"a record appended to the file");
    assert_eq!(
        wholeness_of(&bytes, Format::Jpeg),
        Wholeness::OtherDataAfterEnd
    );
}

#[test]
fn a_jpeg_file_cut_inside_its_scan_is_not_whole_despite_its_thumbnails_end_marker() {
    let whole = jpeg_file(SCAN);
    // Cut two bytes into the entropy-coded data, which follows the scan
    // header's 10 bytes; the thumbnail's end marker lies before.
    let cut = &whole[..SCAN_HEADER_AT + 10 + 2];
    assert!(cut.windows(2).any(|pair| pair == [0xFF, 0xD9]));
    assert_eq!(wholeness_of(cut, Format::Jpeg), Wholeness::NotWhole);
}

#[test]
fn a_jpeg_file_zero_filled_after_its_exif_segment_is_not_whole() {
    let mut bytes = jpeg_file(SCAN);
    bytes[SCAN_HEADER_AT..].fill(0x00);
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::NotWhole);
}

#[test]
fn stuffed_bytes_and_restart_markers_do_not_end_the_scan() {
    // A stuffed `FF 00`, then a restart marker, inside the scan.
    let scan = [0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56];
    let bytes = jpeg_file(&scan);
    assert_eq!(main_image_end(&bytes), Some(bytes.len() - 2));
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::Whole);
}

#[test]
fn the_image_ends_at_its_own_end_marker_not_a_later_images() {
    // An image after the main one, as an MPF list holds, ends later.
    let main = jpeg_file(SCAN);
    let bytes = [&main[..], &jpeg_file(SCAN)].concat();
    assert_eq!(main_image_end(&bytes), Some(main.len() - 2));
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::Whole);
}

#[test]
fn bytes_without_an_end_marker_are_not_whole() {
    let mut bytes = vec![0xFF, 0xD8];
    bytes.extend(segment(0xDA, &[1, 1, 0x00, 0, 63, 0]));
    bytes.extend_from_slice(SCAN);
    assert!(!bytes.windows(2).any(|pair| pair == [0xFF, 0xD9]));
    assert_eq!(wholeness_of(&bytes, Format::Jpeg), Wholeness::NotWhole);
    assert_eq!(wholeness_of(&bytes, Format::Nef), Wholeness::NotWhole);
}

#[test]
fn bytes_of_another_length_than_expected_are_not_whole() {
    let bytes = jpeg_file(SCAN);
    for len in [bytes.len() - 1, bytes.len() + 1] {
        for format in [Format::Jpeg, Format::Nef] {
            assert_eq!(wholeness(&bytes, len as u64, format), Wholeness::NotWhole);
        }
    }
}

#[test]
fn bytes_without_a_start_marker_are_not_whole() {
    let bytes = jpeg_file(SCAN);
    for format in [Format::Jpeg, Format::Nef] {
        assert_eq!(wholeness_of(&bytes[2..], format), Wholeness::NotWhole);
    }
}

#[test]
fn ff_fill_after_a_raw_previews_end_is_padding() {
    let mut bytes = jpeg_file(SCAN);
    bytes.extend_from_slice(&[0xFF; 51]);
    assert_eq!(wholeness_of(&bytes, Format::Nef), Wholeness::Whole);
}

/// `count` bytes of other data, neither `FF` nor zero, as a write
/// buffer's leftovers might read.
fn leftovers(count: usize) -> Vec<u8> {
    (0..count).map(|i| 1 + (i % 254) as u8).collect()
}

/// A raw preview in the shape the D600 and the D3200 write: a whole JPEG,
/// its scan padded so that the image's bytes plus `after` make a length
/// `residue` more than a multiple of 512, then `after` as the leftovers.
/// The nine files have `residue` 2, as a count of blocks plus 2 gives.
fn preview_ending_early(after: &[u8], residue: usize) -> Vec<u8> {
    let image = jpeg_file(SCAN);
    let padding = (512 + residue - (image.len() + after.len()) % 512) % 512;
    let mut bytes = jpeg_file(&[SCAN, &vec![0x12; padding]].concat());
    bytes.extend_from_slice(after);
    assert_eq!(bytes.len() % 512, residue);
    bytes
}

#[test]
fn a_nef_preview_with_leftovers_after_its_end_ends_early_up_to_the_bound() {
    // Zeros are other data in a raw preview, not padding, so they are
    // leftovers too.
    for count in [10, LEFTOVERS_AFTER_END] {
        for after in [leftovers(count), vec![0x00; count]] {
            let bytes = preview_ending_early(&after, 2);
            assert_eq!(
                wholeness_of(&bytes, Format::Nef),
                Wholeness::EndsEarly,
                "{count} bytes after the end"
            );
            // Another raw format's preview stays strict.
            assert_eq!(wholeness_of(&bytes, Format::Arw), Wholeness::NotWhole);
        }
        // A JPEG file's own bytes after the end are other data, as before.
        assert_eq!(
            wholeness_of(&preview_ending_early(&leftovers(count), 2), Format::Jpeg),
            Wholeness::OtherDataAfterEnd
        );
    }
}

#[test]
fn a_nef_preview_with_more_leftovers_than_the_bound_is_not_whole() {
    let bytes = preview_ending_early(&leftovers(LEFTOVERS_AFTER_END + 1), 2);
    assert_eq!(wholeness_of(&bytes, Format::Nef), Wholeness::NotWhole);
}

#[test]
fn a_nef_preview_whose_declared_length_is_not_2_past_a_block_is_not_whole() {
    for residue in [1, 3] {
        let bytes = preview_ending_early(&leftovers(LEFTOVERS_AFTER_END), residue);
        assert_eq!(
            wholeness_of(&bytes, Format::Nef),
            Wholeness::NotWhole,
            "a length {residue} more than a multiple of 512"
        );
    }
}

#[test]
fn the_leftovers_are_measured_from_the_images_own_end_marker() {
    // Leftovers that hold an end marker of their own, near their end:
    // within the bound from the image's own marker they end early, and
    // one byte beyond it they fail, though the last marker lies nearer.
    for (count, expected) in [
        (LEFTOVERS_AFTER_END, Wholeness::EndsEarly),
        (LEFTOVERS_AFTER_END + 1, Wholeness::NotWhole),
    ] {
        let mut after = leftovers(count);
        after.splice(count - 4..count - 2, [0xFF, 0xD9]);
        let bytes = preview_ending_early(&after, 2);
        assert_eq!(
            wholeness_of(&bytes, Format::Nef),
            expected,
            "{count} bytes after the end"
        );
    }
}
