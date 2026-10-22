// SPDX-License-Identifier: MIT OR Apache-2.0

//! Opening files through the public API: what is detected, what fails and
//! how, and that the options reach the reader.

mod support;

use std::fs::File;
use std::io::{self, Cursor};

use raw_files::{ErrorKind, Focus, Format, Limits, Metadata, Options, Point, RawFile};
use support::jpeg_builder::JpegBuilder;
use support::tiff_builder::{Order, TiffBuilder};

/// The Make tag: the camera's manufacturer, as text.
const MAKE: u16 = 0x010F;

/// The DNGVersion tag: the version of the DNG specification a DNG follows.
const DNG_VERSION: u16 = 0xC612;

/// The DNGPrivateData tag: a block of the camera maker's own data.
const DNG_PRIVATE_DATA: u16 = 0xC634;

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
fn a_file_that_is_neither_a_tiff_nor_a_jpeg_file_is_of_unknown_format() {
    for bytes in [&b"not a tiff"[..], b"", b"I", &[0xFF, 0xD8]] {
        let err = RawFile::from_bytes(bytes).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{bytes:02X?}");
    }
}

#[test]
fn a_jpeg_file_opens_as_jpeg_whatever_its_make() {
    // A camera's JPEG: JFIF, an Exif segment with a Nikon Make, then the
    // image. The Make names the maker of NEF files, but the container is
    // JPEG, whose list holds JPEG alone.
    let mut tiff = TiffBuilder::new(Order::Little);
    let ifd = tiff.ifd();
    tiff.ascii(ifd, MAKE, "NIKON CORPORATION");
    let mut builder = JpegBuilder::new();
    builder.jfif().exif(&tiff.build().bytes);
    let bytes = builder.build().bytes;
    let file = RawFile::from_bytes(&bytes).unwrap();
    assert_eq!(file.format(), Format::Jpeg);
    assert!(!file.format().is_raw());
    let file = RawFile::from_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(file.format(), Format::Jpeg);
}

#[test]
fn a_nikon_file_that_holds_no_jpeg_opens_as_nef_with_no_previews() {
    let bytes = one_ifd(Some("NIKON CORPORATION"));
    let file = RawFile::from_bytes(&bytes).unwrap();
    assert_eq!(file.format(), Format::Nef);
    assert!(file.previews().is_empty());
    assert_eq!(file.largest_preview(), None);
    let file = RawFile::from_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(file.format(), Format::Nef);
    assert!(file.previews().is_empty());
    assert_eq!(file.largest_preview(), None);
}

#[test]
fn a_sony_file_that_holds_no_jpeg_opens_as_arw_with_no_previews() {
    let bytes = one_ifd(Some("SONY"));
    let file = RawFile::from_bytes(&bytes).unwrap();
    assert_eq!(file.format(), Format::Arw);
    assert!(file.previews().is_empty());
    assert_eq!(file.largest_preview(), None);
    let file = RawFile::from_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(file.format(), Format::Arw);
    assert!(file.previews().is_empty());
    assert_eq!(file.largest_preview(), None);
}

#[test]
fn a_dng_whose_make_names_a_known_maker_is_of_unknown_format() {
    for make in ["SONY", "NIKON CORPORATION"] {
        // As Adobe's converter writes it: the camera's Make, a DNG version
        // and a private data block.
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd = builder.ifd();
        builder
            .ascii(ifd, MAKE, make)
            .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
            .byte(ifd, DNG_PRIVATE_DATA, &[0; 32]);
        let bytes = builder.build().bytes;
        let err = RawFile::from_bytes(&bytes).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{make}");
        let err = RawFile::from_reader(Cursor::new(bytes)).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{make}");
    }
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

#[test]
fn metadata_values_are_send_and_sync() {
    // A culler reads the metadata of many files on a pool of threads.
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<Metadata>();
    assert_sync::<Metadata>();
    assert_send::<Focus>();
    assert_sync::<Focus>();
    assert_send::<Point>();
    assert_sync::<Point>();
}
