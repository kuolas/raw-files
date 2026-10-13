// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG probe on arbitrary bytes: the walk to the frame header, over
//! the whole input and over a range inside it.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| raw_files::fuzzing::jpeg_probe(data));
