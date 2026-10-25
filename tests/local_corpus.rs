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
//! the largest preview's bytes of each. It counts each file under the
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
//! For the camera settings that the Exif tags record, the exposure
//! program, the metering mode, the flash, the white balance mode and the
//! colour space, it counts, over the files of a section, how many have each
//! setting and how many have each kind of it, by the name the library
//! gives the kind, such as "Aperture priority", and how many flashes fired;
//! as for the sizes, these are totals over the section, tied to no file.
//!
//! For the focus point, where the camera focused, it counts the files that
//! have one, how many of those record that the camera confirmed focus, that
//! it missed, or neither, the sizes of the frames the points are measured
//! in, how many different sizes the focus areas have, and how many frames
//! are not the file's image size. It never prints where a point lies or how
//! large one area is.
//!
//! It fails when the folder cannot be read or holds no file of a format
//! raw-files reads, and otherwise unless every entry could be read and, in
//! every format's section, every file opened, every file has a preview,
//! every largest preview is a whole JPEG, every file's metadata gives the
//! orientation its largest preview carries, and every focus point lies
//! inside its frame, its area included, both as read and once placed on the
//! largest preview. A whole JPEG starts with its start marker and ends with
//! its end marker, followed by nothing but the padding some Sony cameras
//! write: `FF` bytes, in any format, and zeros, only in the largest preview
//! of a file that opens as JPEG, which is the whole file. Such a file must
//! also hold its own image's end marker, after its first scan, not only
//! its thumbnail's. Other data after the last end marker fails a RAW
//! file's preview, and is only counted in a JPEG file's. The metadata
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

use raw_files::{DateTime, Focus, Format, Metadata, RawFile, Size};

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

/// How the camera was set for the files, as the Exif tags record it, in
/// counts: for each setting, how many files have it, and how many have each
/// of its kinds, by the name the library gives the kind, such as
/// "Aperture priority". The settings that only a MakerNote records are not
/// counted yet. See #26
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
        if let Some(mode) = settings.white_balance().mode() {
            count(
                &mut self.white_balance_mode,
                &mut self.white_balance_modes,
                mode.name(),
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

/// What the files say about where the camera focused, in counts. No
/// centre and no area size is printed: the area sizes are kept only to
/// count how many different ones there are.
#[derive(Default)]
struct FocusTally {
    with_point: usize,
    without_point: usize,
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
        let Some(focus) = metadata.focus() else {
            self.without_point += 1;
            return;
        };
        self.with_point += 1;
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
        self.outside_frame += usize::from(!lies_inside(&focus, frame));
        self.frame_not_image_size += usize::from(metadata.image_size() != Some(frame));
    }

    /// Places `focus` on a largest preview of size `preview` and counts
    /// whether that worked and kept the point inside the preview.
    fn record_scaled(&mut self, focus: &Focus, preview: Size) {
        match focus.scaled_to(preview) {
            Some(scaled) => {
                self.scaled_outside_frame += usize::from(!lies_inside(&scaled, preview));
            }
            None => self.not_scaled += 1,
        }
    }
}

/// Whether the centre of `focus` lies inside `frame`, edges included, and
/// so does its area around the centre when it has one: on each axis, from
/// the centre less half the side, rounded up, to the centre plus that half
/// lies between 0 and the frame's side. Written out here, from the rule
/// that `Focus::new` and `Focus::with_size` document, rather than asked
/// of the library, so that the test does not take the library's word for
/// it.
fn lies_inside(focus: &Focus, frame: Size) -> bool {
    let center = focus.center();
    let fits = |center: u32, side: u32, frame: u32| {
        let half = side.div_ceil(2);
        half <= center && u64::from(center) + u64::from(half) <= u64::from(frame)
    };
    center.x <= frame.width
        && center.y <= frame.height
        && focus.size().is_none_or(|size| {
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
        // Only a file that opened as JPEG can have other data after the end.
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
        println!(
            "files with a colour space: {}, by colour space: {:?}",
            s.color_space, s.color_spaces
        );
        let f = &self.focus;
        println!(
            "files with a focus point: {}, without: {}",
            f.with_point, f.without_point
        );
        println!(
            "focus confirmed: true {}, false {}, unknown {}",
            f.confirmed, f.missed, f.unknown
        );
        println!("focus frames: {:?}", f.frames);
        println!("distinct focus area sizes: {}", f.area_sizes.len());
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
    /// A whole JPEG file whose last end marker is followed by data other
    /// than padding.
    OtherDataAfterEnd,
    /// Not a whole JPEG.
    NotWhole,
}

/// Whether `bytes`, the largest preview of a file that opened as `format`,
/// are a whole JPEG: `expected_len` bytes long, as the preview says,
/// starting with the start marker `FF D8 FF`, and holding the end marker
/// `FF D9` with nothing after its last one but padding, or, in a file that
/// opened as JPEG, with other data after it.
///
/// A file that opened as JPEG must also hold its own image's end marker,
/// which [`main_image_end`] looks for after the image's first scan. Its
/// Exif segment may hold a thumbnail, and its MakerNote a preview, each
/// with an end marker of its own; a file cut inside its image, or whose
/// bytes after the Exif segment were lost to zeros, still holds those, and
/// a thumbnail's end marker must not stand for the image's. A RAW file's
/// preview is checked by its last end marker alone: it is one JPEG that
/// the reader found by its offset and length, and its last bytes are the
/// ones that show whether that range holds all of it.
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
/// file embeds is known to end in zeros, so the check stays strict there.
///
/// Since that preview is the whole file, other data after its last end
/// marker, such as a record a website appends to the files it serves,
/// belongs to the file, not to a range the reader chose: it is counted
/// apart, not as a failure. In a RAW file's preview, other data is a
/// failure.
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
    if bytes[end_marker + 2..].iter().all(|&byte| is_padding(byte)) {
        Wholeness::Whole
    } else if jpeg_file {
        Wholeness::OtherDataAfterEnd
    } else {
        Wholeness::NotWhole
    }
}

/// Where the end marker of the image of a JPEG file, `bytes`, lies, or
/// `None` when it has none. Walked here rather than asked of the library,
/// so that the test does not take the library's word for it.
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
    if let Some(focus) = raw.metadata().focus() {
        tally.focus.record_scaled(&focus, preview.size());
    }
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
    let unreadable_entries = walk(Path::new(&dir), &mut |path| {
        let format = path
            .extension()
            .and_then(|ext| Format::from_extension(&ext.to_string_lossy()));
        if let Some(format) = format {
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

#[test]
fn zeros_after_a_raw_previews_end_are_not_padding() {
    let mut bytes = jpeg_file(SCAN);
    bytes.extend_from_slice(&[0x00; 64]);
    assert_eq!(wholeness_of(&bytes, Format::Nef), Wholeness::NotWhole);
}

#[test]
fn other_data_after_a_raw_previews_end_is_not_whole() {
    let mut bytes = jpeg_file(SCAN);
    bytes.extend_from_slice(b"other data");
    assert_eq!(wholeness_of(&bytes, Format::Nef), Wholeness::NotWhole);
}
