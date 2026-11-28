// SPDX-License-Identifier: MIT OR Apache-2.0

//! An XMP packet on arbitrary bytes: the XML reader, the RDF walk, and
//! every value of the packet read, through each getter.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| raw_files::fuzzing::xmp(data));
