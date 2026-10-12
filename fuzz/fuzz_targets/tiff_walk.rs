// SPDX-License-Identifier: MIT OR Apache-2.0

//! The TIFF reader on arbitrary bytes: the header, the walk, and every
//! value of every IFD found.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| raw_files::fuzzing::tiff_walk(data));
