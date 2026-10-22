// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG file's segments on arbitrary bytes: the walk over them, the
//! TIFF structure of the Exif segment with every value of every IFD, and
//! the index of the MPF segment.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| raw_files::fuzzing::jpeg_segments(data));
