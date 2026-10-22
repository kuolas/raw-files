// SPDX-License-Identifier: MIT OR Apache-2.0

//! What unit tests of any module share, whatever its layer: a reader that
//! fails past the first block the source caches, as a disk or a network
//! reader can fail, so that a test can tell a reader's failure from damage
//! in the file.
//!
//! It stands at the crate's top, outside `container` and `formats`, so
//! that tests of either may use it without a container module's tests
//! depending on `formats`.

use std::io::{self, Cursor, Read, Seek, SeekFrom};

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
