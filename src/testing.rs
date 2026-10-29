// SPDX-License-Identifier: MIT OR Apache-2.0

//! What unit tests of any module share, whatever its layer: a reader that
//! fails past the first block the source caches, as a disk or a network
//! reader can fail, so that a test can tell a reader's failure from damage
//! in the file; the checks of an enum's names and order, that each name is
//! the one documented, that `Display` writes it, that they are distinct,
//! ASCII and in sentence case, and that the variants order as they are
//! listed; and the fixtures the tests of the focus, the metadata and the
//! readers share: a size, a point and a Z 9's focus.
//!
//! It stands at the crate's top, outside `container` and `formats`, so
//! that tests of either may use it without a container module's tests
//! depending on `formats`.

use std::fmt;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

use crate::focus::{Focus, FocusOrigin};
use crate::geometry::{Point, Size};
use crate::io::BLOCK_SIZE;

/// The first byte of the second block the source caches: a [`FailsFar`]
/// still serves everything the first block holds, such as a file's header
/// and its first IFD, and fails on the next block it must read.
pub(crate) const FAR: u64 = BLOCK_SIZE;

/// A reader over bytes in memory that fails every read from [`FAR`] on
/// with an I/O error, for the tests that a failure of the reader is passed
/// up as an I/O error, never taken for damage or for the end of the data.
pub(crate) struct FailsFar(pub(crate) Cursor<Vec<u8>>);

impl Read for FailsFar {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.0.position() >= FAR {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        self.0.read(buf)
    }
}

impl Seek for FailsFar {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}

/// Checks that `all`'s names are `expected`, in order, and so as many,
/// that `Display` writes each name, and that every name is distinct,
/// ASCII and in sentence case.
pub(crate) fn assert_names<T: Copy + fmt::Debug + fmt::Display>(
    all: &[T],
    name: fn(T) -> &'static str,
    expected: &[&str],
) {
    let names: Vec<&str> = all.iter().map(|&value| name(value)).collect();
    assert_eq!(names, expected);
    for &value in all {
        let text = name(value);
        assert_eq!(value.to_string(), text, "{value:?}");
        assert!(text.is_ascii(), "{value:?}");
        // Sentence case: a capital first, or a digit. "sRGB" is the one
        // name whose own spelling starts in lower case.
        let capital = text.starts_with(|c: char| c.is_ascii_uppercase() || c.is_ascii_digit());
        assert!(capital || text == "sRGB", "{value:?}");
    }
    let distinct: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(distinct.len(), names.len(), "{names:?}");
}

/// Checks that `all`, the variants in declaration order, is strictly
/// increasing.
pub(crate) fn assert_ordered<T: Copy + Ord + fmt::Debug>(all: &[T]) {
    assert!(all.windows(2).all(|pair| pair[0] < pair[1]), "{all:?}");
}

/// A size of `width` by `height` pixels.
pub(crate) fn size(width: u32, height: u32) -> Size {
    Size { width, height }
}

/// The point `x` pixels from the left and `y` from the top.
pub(crate) fn point(x: u32, y: u32) -> Point {
    Point { x, y }
}

/// Where a Nikon Z 9 focused: a 291 × 323 area right of and above the
/// centre of its 8256 × 5504 frame, with focus confirmed, as the camera
/// recorded it.
pub(crate) fn z9_focus() -> Focus {
    z9_focus_from(FocusOrigin::Recorded)
}

/// The Z 9's focus of [`z9_focus`], from `origin`, for the tests that
/// check a value of each origin.
pub(crate) fn z9_focus_from(origin: FocusOrigin) -> Focus {
    Focus::new(size(8_256, 5_504), point(4_905, 2_461), origin)
        .unwrap()
        .with_size(size(291, 323))
        .unwrap()
        .with_in_focus(true)
}
