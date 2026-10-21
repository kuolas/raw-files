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

use std::io::Cursor;

use crate::container::jpeg::probe;
use crate::container::tiff::{parse_header, walk};
use crate::geometry::Size;
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::{Focus, Metadata};
use crate::options::Options;

/// Reads `data` as a TIFF file: the header, the walk with the default
/// limits, then every value of every IFD the walk found, through each
/// accessor.
///
/// Whatever `data` holds, this must return, never panic, and do so in
/// bounded time and memory. What it reads is thrown away: an error is an
/// expected outcome for arbitrary bytes, so every `Result` is ignored.
pub fn tiff_walk(data: &[u8]) {
    let Ok(mut src) = Source::new(Cursor::new(data)) else {
        return;
    };
    let Ok(header) = parse_header(&mut src) else {
        return;
    };
    let Ok(tree) = walk(&mut src, header, &Limits::default()) else {
        return;
    };
    for (_, ifd) in tree.iter() {
        for tag in ifd.tags() {
            let _ = ifd.u32s(&mut src, tag);
            let _ = ifd.ascii(&mut src, tag);
            let _ = ifd.bytes(&mut src, tag);
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

/// Opens `data` as a RAW file, as a caller would, with the default options,
/// then asks it everything the handle answers: its format, every value of
/// its metadata, formatted as a caller would print it, its largest
/// preview and the focus point placed on it, the smallest preview at least
/// 1 by 1, and the bytes of every preview, read into one buffer.
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
    let _ = file.preview_at_least(1, 1);
    let mut buf = Vec::new();
    for preview in file.previews().to_vec() {
        let _ = file.read_preview_into(&preview, &mut buf);
    }
}

/// Asks `metadata` for every value, and formats, reduces and converts
/// each time and fraction as a caller showing it would, so that what those
/// types do with values the reader built from arbitrary bytes is fuzzed
/// too, not only the reading. The focus point is read, then placed on a
/// frame of 1 by 1 pixel and read again, since a tiny frame is where its
/// rounding and trimming do the most. A `Debug` form of the whole covers
/// whatever a later value adds.
fn read_metadata(metadata: &Metadata) {
    let _ = metadata.make();
    let _ = metadata.model();
    let _ = metadata.serial_number();
    if let Some(time) = metadata.capture_time() {
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
    if let Some(lens) = metadata.lens() {
        let _ = lens.make();
        let _ = lens.model();
        let _ = lens.serial_number();
        fractions.extend([
            lens.min_focal_length(),
            lens.max_focal_length(),
            lens.f_number_at_min_focal_length(),
            lens.f_number_at_max_focal_length(),
        ]);
    }
    for fraction in fractions.into_iter().flatten() {
        let _ = fraction.to_string();
        let _ = fraction.reduced();
        let _ = fraction.to_f64();
    }
    let _ = metadata.raw_size();
    let _ = metadata.image_size();
    if let Some(focus) = metadata.focus() {
        read_focus(&focus);
        let tiny = Size {
            width: 1,
            height: 1,
        };
        if let Some(on_tiny) = focus.scaled_to(tiny) {
            read_focus(&on_tiny);
        }
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
