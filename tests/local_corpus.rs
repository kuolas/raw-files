// SPDX-License-Identifier: MIT OR Apache-2.0

//! The reader run over a folder of your own RAW files, to see whether it
//! copes with cameras and settings the committed samples do not cover.
//!
//! The test is ignored by default. Run it, in release mode because it reads
//! every file, with
//!
//! ```sh
//! RAW_FILES_LOCAL_CORPUS=/path/to/photos cargo test --release --test local_corpus -- --ignored --nocapture
//! ```
//!
//! It walks the folder and its subfolders, opens every file whose extension
//! is a known RAW format, and reads the metadata and the largest preview of
//! each. It prints counts only: how many files opened, how many have each
//! metadata field, and how the sizes are distributed; it
//! never prints a path, a file name, or the value of a make, model, serial
//! number, capture time, exposure or lens, so its output can be pasted into
//! an issue without disclosing anything about the photos.
//!
//! For the focus point, where the camera focused, it counts the files that
//! have one, how many of those record that the camera confirmed focus, that
//! it missed, or neither, the sizes of the frames the points are measured
//! in, how many different sizes the focus areas have, and how many frames
//! are not the file's image size. It never prints where a point lies or how
//! large one area is.
//!
//! It fails when the folder cannot be read or holds no file with a known
//! RAW extension, and otherwise unless every entry could be read, every file
//! opened, every file has a preview, every largest preview is a whole JPEG,
//! every file's metadata gives the orientation its largest preview
//! carries, and every focus point lies inside its frame, its area included,
//! both as read and once placed on the largest preview. The metadata counts
//! are printed for the reader of the output to judge, since what a folder's
//! cameras record varies: a body that writes no serial number, or no
//! sub-seconds, is not a failure of the library.

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
    serial_number: usize,
    capture_time: usize,
    with_subseconds: usize,
    with_offset: usize,
    iso: usize,
    exposure_time: usize,
    f_number: usize,
    focal_length: usize,
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

/// What the run found, in counts.
#[derive(Default)]
struct Tally {
    seen: usize,
    unreadable: usize,
    opened: usize,
    failures: BTreeMap<String, usize>,
    no_preview: usize,
    not_whole_jpeg: usize,
    formats: BTreeMap<String, usize>,
    sizes: BTreeMap<(u32, u32), usize>,
    orientations: BTreeMap<String, usize>,
    metadata: MetadataTally,
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
    *tally
        .formats
        .entry(format!("{:?}", raw.format()))
        .or_default() += 1;
    tally.metadata.record(raw.metadata());
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
        Ok(bytes) => {
            let whole = u64::try_from(bytes.len()).is_ok_and(|len| len == preview.byte_len())
                && bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                && bytes.ends_with(&[0xFF, 0xD9]);
            if !whole {
                tally.not_whole_jpeg += 1;
            }
        }
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
#[ignore = "reads a folder of your own RAW files; see the module documentation"]
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
    let mut tally = Tally::default();
    let mut paths = Vec::new();
    tally.unreadable += walk(Path::new(&dir), &mut |path| {
        let known = path
            .extension()
            .is_some_and(|ext| Format::from_extension(&ext.to_string_lossy()).is_some());
        if known {
            paths.push(path);
        }
    });
    for path in paths {
        tally.seen += 1;
        check(&path, &mut tally);
    }

    let mut shares = std::mem::take(&mut tally.shares);
    shares.sort_by(f64::total_cmp);
    let mut open_bytes = std::mem::take(&mut tally.open_bytes);
    open_bytes.sort_unstable();
    let m = &tally.metadata;
    println!("files with a known extension: {}", tally.seen);
    println!("unreadable: {}", tally.unreadable);
    println!("opened: {}", tally.opened);
    println!("failures by kind: {:?}", tally.failures);
    println!("no preview: {}", tally.no_preview);
    println!("not a whole JPEG: {}", tally.not_whole_jpeg);
    println!("formats: {:?}", tally.formats);
    println!("largest-preview sizes: {:?}", tally.sizes);
    println!("largest-preview orientations: {:?}", tally.orientations);
    println!(
        "files with a make: {}, a model: {}, a serial number: {}",
        m.make, m.model, m.serial_number
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
        "files with an ISO: {}, an exposure time: {}, an f-number: {}, a focal length: {}, an exposure compensation: {}",
        m.iso, m.exposure_time, m.f_number, m.focal_length, m.exposure_bias
    );
    println!(
        "files with a lens: {}, of which with a model: {}, with a focal-length range: {}",
        m.lens, m.lens_model, m.lens_focal_range
    );
    println!("raw sizes: {:?}", m.raw_sizes);
    println!("image sizes: {:?}", m.image_sizes);
    let f = &tally.focus;
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
        tally.orientation_disagreements
    );
    if let (Some(median), Some(max)) = (open_bytes.get(open_bytes.len() / 2), open_bytes.last()) {
        println!(
            "bytes read by open alone, median: {median} ({} KiB), maximum: {max} ({} KiB)",
            median / 1024,
            max / 1024
        );
    }
    if let (Some(median), Some(max)) = (shares.get(shares.len() / 2), shares.last()) {
        println!("share of the file read, median: {median:.2} %, maximum: {max:.2} %");
    }

    assert!(
        tally.seen > 0,
        "no file with a known RAW extension under RAW_FILES_LOCAL_CORPUS"
    );
    assert_eq!(tally.unreadable, 0, "some entries could not be read");
    assert!(
        tally.failures.is_empty(),
        "some files failed to open or read"
    );
    assert_eq!(tally.no_preview, 0, "some files have no preview");
    assert_eq!(
        tally.orientation_disagreements, 0,
        "some files' metadata orientation differs from their preview's"
    );
    assert_eq!(
        tally.not_whole_jpeg, 0,
        "some largest previews are not whole JPEGs"
    );
    assert_eq!(
        tally.focus.outside_frame, 0,
        "some focus points lie outside their frame"
    );
    assert_eq!(
        tally.focus.not_scaled, 0,
        "some focus points could not be scaled onto their largest preview"
    );
    assert_eq!(
        tally.focus.scaled_outside_frame, 0,
        "some focus points, scaled onto their largest preview, lie outside it"
    );
}
