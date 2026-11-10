// SPDX-License-Identifier: MIT OR Apache-2.0

//! The entry points of the fuzz targets in `fuzz/`.
//!
//! A fuzz target feeds a parser arbitrary bytes, again and again, and
//! reports any input that makes it panic, run without end or allocate past
//! the fuzzer's memory limit. Each function here reads its input as a
//! caller would, so that one call reaches the whole of a reader, and the
//! fuzz target is a single line.
//!
//! The module is compiled only with `--cfg fuzzing`, which cargo-fuzz sets:
//! normal builds and the documentation never see it, and it is not part of
//! the API. The file still ships in the package, because cargo-fuzz sets
//! that flag for every crate it builds: a project that fuzzes its own code
//! compiles this module in its copy of raw-files too.

use std::io::{Cursor, Read, Seek};

use crate::container::jpeg::{mpf, probe, segments};
use crate::container::tiff::{Block, Tree, walk};
use crate::datetime::DateTime;
use crate::entries::{Entries, Entry, Value};
use crate::focus::{Focus, FocusLayout, FocusPoints};
use crate::geometry::Size;
use crate::io::Source;
use crate::limits::Limits;
use crate::location::Location;
use crate::metadata::Metadata;
use crate::options::Options;

/// Reads `data` as a TIFF file: the walk with the default limits, header
/// first, then every value of every IFD the walk found, through each
/// accessor.
///
/// Whatever `data` holds, this must return, never panic, and do so in
/// bounded time and memory. What it reads is thrown away: an error is an
/// expected outcome for arbitrary bytes, so every `Result` is ignored.
pub fn tiff_walk(data: &[u8]) {
    let Ok(mut src) = Source::new(Cursor::new(data)) else {
        return;
    };
    let block = Block::file(src.len());
    let Ok(tree) = walk(&mut src, &block, &Limits::default()) else {
        return;
    };
    read_values(&mut src, &tree);
}

/// Reads every value of every IFD of `tree` through each accessor, as a
/// number, as text and as bytes, whatever the tag and its type; then
/// lists the tree's entries, as opening a file does, and reads and
/// formats each one's value under the default cap.
fn read_values<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) {
    for (_, ifd) in tree.iter() {
        for tag in ifd.tags() {
            let _ = ifd.u32s(src, tag);
            let _ = ifd.text_bytes(src, tag);
            let _ = ifd.bytes(src, tag);
        }
    }
    let cap = Limits::default().max_value_bytes();
    for entry in &Entries::of_tree(tree) {
        read_entry(entry);
        if let Ok(value) = entry.read(src, cap) {
            read_value(&value);
        }
    }
}

/// Reads `data` as a JPEG stream: the probe for its frame header over the
/// whole of `data`, then over a range inside it that the first two bytes
/// choose, and whether each frame found is lossless.
///
/// The second range tests the probe's bounds, which a caller sets from a
/// preview's tags: the first byte is where it starts, the second how many
/// bytes it leaves off the end, so that the fuzzer also probes ranges that
/// start inside the bytes or end before them.
///
/// Whatever `data` holds, this must return, never panic, and do so in
/// bounded time and memory. What it reads is thrown away: an error or no
/// frame is an expected outcome for arbitrary bytes, so every result is
/// ignored.
pub fn jpeg_probe(data: &[u8]) {
    let Ok(mut src) = Source::new(Cursor::new(data)) else {
        return;
    };
    let Ok(len) = u64::try_from(data.len()) else {
        return;
    };
    if let Ok(Some(frame)) = probe(&mut src, 0, len) {
        let _ = frame.is_lossless();
    }
    let [start, cut, ..] = data else {
        return;
    };
    let offset = u64::from(*start);
    let Some(sub_len) = len
        .checked_sub(u64::from(*cut))
        .and_then(|end| end.checked_sub(offset))
        .filter(|&sub_len| sub_len > 0)
    else {
        return;
    };
    if let Ok(Some(frame)) = probe(&mut src, offset, sub_len) {
        let _ = frame.is_lossless();
    }
}

/// Reads `data` as a JPEG file: the walk over its segments, then whether
/// the frame header it kept is lossless; the TIFF structure of its Exif
/// segment, walked with the default limits and every value of every IFD
/// read through each accessor, as [`tiff_walk`] reads a TIFF file; and the
/// index of its MPF segment, with the default limits.
///
/// Whatever `data` holds, this must return, never panic, and do so in
/// bounded time and memory. What it reads is thrown away: an error or a
/// missing segment is an expected outcome for arbitrary bytes, so every
/// result is ignored.
pub fn jpeg_segments(data: &[u8]) {
    let Ok(mut src) = Source::new(Cursor::new(data)) else {
        return;
    };
    let Ok(Some(found)) = segments(&mut src) else {
        return;
    };
    if let Some(frame) = found.frame {
        let _ = frame.is_lossless();
    }
    let limits = Limits::default();
    if let Some(exif) = &found.exif
        && let Ok(tree) = walk(&mut src, exif, &limits)
    {
        read_values(&mut src, &tree);
    }
    if let Some(index) = &found.mpf {
        let _ = mpf::index(&mut src, index, &limits);
    }
}

/// Opens `data` as a camera file, a RAW file or a JPEG, as a caller would,
/// with the default options, then asks it everything the handle answers:
/// its format, every value of its metadata, formatted as a caller would
/// print it, its largest preview and the focus point and the focus points
/// placed on it, the smallest preview at least 1 by 1, the bytes of every
/// preview, read into one buffer, and every entry, with its value read
/// and formatted.
///
/// Whatever `data` holds, this must return, never panic, and do so in
/// bounded time and memory. What it reads is thrown away: an error is an
/// expected outcome for arbitrary bytes, so every `Result` is ignored.
pub fn open(data: &[u8]) {
    let Ok(mut file) = Options::new().from_bytes(data) else {
        return;
    };
    let _ = file.format();
    read_metadata(file.metadata());
    let largest = file.largest_preview();
    if let (Some(focus), Some(preview)) = (file.metadata().focus(), largest)
        && let Some(on_preview) = focus.scaled_to(preview.size())
    {
        read_focus(&on_preview);
    }
    if let (Some(points), Some(preview)) = (file.metadata().focus_points(), largest)
        && let Some(on_preview) = points.scaled_to(preview.size())
    {
        read_focus_points(&on_preview);
    }
    let _ = file.preview_at_least(Size {
        width: 1,
        height: 1,
    });
    let mut buf = Vec::new();
    for preview in file.previews().to_vec() {
        let _ = file.read_preview_into(&preview, &mut buf);
    }
    let entries = file.entries().clone();
    for directory in entries.directories() {
        let _ = (directory.to_string(), entries.skipped(directory));
        let _ = entries.in_directory(directory).count();
    }
    for entry in &entries {
        read_entry(entry);
        let _ = entries.entry(entry.directory(), entry.tag());
        if let Ok(value) = file.read_value(entry) {
            read_value(&value);
        }
    }
}

/// Asks `entry` for each of its values and its `Debug` form, as a caller
/// listing it would.
fn read_entry(entry: &Entry) {
    let _ = entry.directory().to_string();
    let _ = (entry.tag(), entry.name(), entry.count(), entry.value_len());
    let _ = entry.field_type().to_string();
    let _ = entry.byte_range();
    let _ = format!("{entry:?}");
}

/// Asks `value` for each of its forms, as a caller showing it would: its
/// type, its length, its bytes, its text and its `Debug` form.
fn read_value(value: &Value) {
    let _ = value.field_type();
    let _ = (value.len(), value.is_empty());
    let _ = value.bytes();
    let _ = value.text();
    let _ = format!("{value:?}");
}

/// Asks `location` for every part, and converts each coordinate and the
/// altitude to a decimal as a map would, so that the conversions are
/// fuzzed on values the reader built from arbitrary bytes.
fn read_location(location: &Location) {
    let latitude = location.latitude();
    let _ = (
        latitude.degrees(),
        latitude.minutes(),
        latitude.seconds(),
        latitude.is_south(),
        latitude.to_degrees(),
    );
    let longitude = location.longitude();
    let _ = (
        longitude.degrees(),
        longitude.minutes(),
        longitude.seconds(),
        longitude.is_west(),
        longitude.to_degrees(),
    );
    if let Some(altitude) = location.altitude() {
        let _ = (altitude.meters(), altitude.is_below(), altitude.to_meters());
    }
    if let Some(time) = location.time() {
        read_time(&time);
    }
    if let Some(direction) = location.image_direction() {
        let north = direction.north();
        let _ = (direction.degrees(), north.name(), north.to_string());
    }
    let _ = location.map_datum();
}

/// Asks `time` for every value, and formats it as a caller showing it
/// would.
fn read_time(time: &DateTime) {
    let _ = time.to_string();
    let _ = (
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second(),
        time.nanosecond(),
        time.subsecond_digits(),
        time.offset_minutes(),
    );
}

/// Asks `metadata` for every value, and formats, reduces and converts
/// each time and fraction as a caller showing it would, so that what those
/// types do with values the reader built from arbitrary bytes is fuzzed
/// too, not only the reading. The focus point and the focus points are
/// read, then placed on a frame of 1 by 1 pixel and read again, since a
/// tiny frame is where their rounding and trimming do the most; the layout
/// of the body's points is read as it is. A `Debug` form of the whole
/// covers whatever a later value adds.
fn read_metadata(metadata: &Metadata) {
    let camera = metadata.camera();
    let _ = camera.make().map(|make| (make.name(), make.to_string()));
    let _ = camera.model();
    let _ = camera.recorded_make();
    let _ = camera.recorded_model();
    let _ = camera.serial_number();
    let _ = camera.owner_name();
    let authorship = metadata.authorship();
    let _ = (
        authorship.artist(),
        authorship.copyright(),
        authorship.photographer(),
        authorship.editor(),
        authorship.title(),
        authorship.description(),
        authorship.comment(),
    );
    if let Some(location) = metadata.location() {
        read_location(location);
    }
    if let Some(time) = metadata.capture_time() {
        read_time(&time);
    }
    let _ = metadata.orientation();
    let exposure = metadata.exposure();
    let _ = exposure.iso();
    let mut fractions = vec![
        exposure.exposure_time(),
        exposure.f_number(),
        exposure.focal_length(),
    ];
    if let Some(bias) = exposure.exposure_bias() {
        let _ = bias.to_string();
        let _ = bias.reduced();
        let _ = bias.to_f64();
    }
    let lens = metadata.lens();
    let _ = lens.recorded_make();
    let _ = lens.recorded_model();
    let _ = lens.serial_number();
    fractions.extend([
        lens.min_focal_length(),
        lens.max_focal_length(),
        lens.f_number_at_min_focal_length(),
        lens.f_number_at_max_focal_length(),
    ]);
    for fraction in fractions.into_iter().flatten() {
        let _ = fraction.to_string();
        let _ = fraction.reduced();
        let _ = fraction.to_f64();
    }
    let _ = metadata.raw_size();
    let _ = metadata.image_size();
    let tiny = Size {
        width: 1,
        height: 1,
    };
    if let Some(focus) = metadata.focus() {
        read_focus(&focus);
        if let Some(on_tiny) = focus.scaled_to(tiny) {
            read_focus(&on_tiny);
        }
    }
    if let Some(points) = metadata.focus_points() {
        read_focus_points(points);
        if let Some(on_tiny) = points.scaled_to(tiny) {
            read_focus_points(&on_tiny);
        }
    }
    if let Some(layout) = metadata.focus_layout() {
        read_focus_layout(layout);
    }
    let _ = format!("{metadata:?}");
}

/// Asks `focus` for each of its values and its `Debug` form, as a caller
/// showing it would.
fn read_focus(focus: &Focus) {
    let _ = focus.frame();
    let _ = focus.center();
    let _ = focus.size();
    let _ = focus.in_focus();
    let _ = format!("{focus:?}");
}

/// Asks `points` for each of its values, each point's and its primary's
/// among them, and its `Debug` form, as a caller showing them would.
fn read_focus_points(points: &FocusPoints) {
    let _ = points.frame();
    let _ = points.origin();
    let _ = points.area_mode().map(|mode| mode.to_string());
    for point in points.points() {
        let _ = point.number();
        let _ = point.center();
        let _ = point.size();
        let _ = point.in_focus();
    }
    let _ = points.primary();
    let _ = format!("{points:?}");
}

/// Asks `layout` for each of its values, each point's among them, the
/// point numbered 1, and its `Debug` form, as a caller drawing it would.
fn read_focus_layout(layout: FocusLayout) {
    let _ = layout.recorded_model();
    let _ = layout.frame();
    for point in layout.points() {
        let _ = point.number();
        let _ = point.center();
        let _ = point.size();
    }
    let _ = layout.point(1);
    let _ = format!("{layout:?}");
}
