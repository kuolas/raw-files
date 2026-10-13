// SPDX-License-Identifier: MIT OR Apache-2.0

//! Opening files through the public API: what is detected, what fails and
//! how, and that the options reach the reader.

mod support;

use std::fs::File;
use std::io::{self, Cursor};

use raw_files::{ErrorKind, Limits, Options, RawFile};
use support::tiff_builder::{Order, TiffBuilder};

/// The Make tag: the camera's manufacturer, as text.
const MAKE: u16 = 0x010F;

/// A TIFF file of one IFD, with Make `make` when there is one.
fn one_ifd(make: Option<&str>) -> Vec<u8> {
    let mut builder = TiffBuilder::new(Order::Little);
    let ifd = builder.ifd();
    match make {
        Some(make) => builder.ascii(ifd, MAKE, make),
        None => builder.short(ifd, 0x0112, &[1]),
    };
    builder.build().bytes
}

/// A TIFF file of two IFDs, IFD0 chained to IFD1.
fn two_ifds() -> Vec<u8> {
    let mut builder = TiffBuilder::new(Order::Little);
    let (ifd0, ifd1) = (builder.ifd(), builder.ifd());
    builder.ascii(ifd0, MAKE, "Canon").next(ifd0, Some(ifd1));
    builder.short(ifd1, 0x0112, &[1]);
    builder.build().bytes
}

#[test]
fn a_file_that_is_not_tiff_is_of_unknown_format() {
    let err = RawFile::from_bytes(b"not a tiff").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnknownFormat);
}

#[test]
fn a_tiff_file_of_no_known_format_is_of_unknown_format() {
    for make in [Some("Canon"), None] {
        let bytes = one_ifd(make);
        let err = RawFile::from_bytes(&bytes).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{make:?}");
        let err = RawFile::from_reader(Cursor::new(bytes)).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{make:?}");
    }
}

#[test]
fn a_missing_file_is_an_io_error_that_converts_back() {
    let path = std::env::temp_dir().join("raw-files-open-test-no-such-file.nef");
    let err = RawFile::open(&path).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Io);
    assert_eq!(err.offset(), None);
    assert_eq!(io::Error::from(err).kind(), io::ErrorKind::NotFound);
}

#[test]
fn options_with_room_for_the_file_still_find_no_format() {
    let bytes = one_ifd(Some("Canon"));
    let options = Options::new().limits(Limits::default().max_ifds(1));
    let err = options.from_bytes(&bytes).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnknownFormat);
}

#[test]
fn the_options_limits_reach_the_walk() {
    let bytes = two_ifds();
    // The default limits walk both IFDs, and find no format.
    let err = RawFile::from_bytes(&bytes).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnknownFormat);
    // One IFD allowed: the walk stops at the second.
    let options = Options::new().limits(Limits::default().max_ifds(1));
    let err = options.from_bytes(&bytes).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded);
    let err = options.from_reader(Cursor::new(&bytes)).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded);
}

#[test]
fn handles_are_send_and_sync_when_their_reader_is() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<RawFile<File>>();
    assert_send::<RawFile<Cursor<&[u8]>>>();
    assert_sync::<RawFile<Cursor<&[u8]>>>();
    assert_send::<Options>();
    assert_sync::<Options>();
}
