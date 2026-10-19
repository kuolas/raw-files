// SPDX-License-Identifier: MIT OR Apache-2.0

//! The file handle on arbitrary bytes: opening, detection, the metadata,
//! the previews and reading each one.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| raw_files::fuzzing::open(data));
