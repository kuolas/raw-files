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

use crate::container::tiff::{parse_header, walk};
use crate::io::Source;
use crate::limits::Limits;

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
