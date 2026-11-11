// SPDX-License-Identifier: MIT OR Apache-2.0

//! Opening files through the public API: what is detected, what fails and
//! how, and that the options reach the reader.

mod support;

use std::fs::File;
use std::io::{self, Cursor};

use raw_files::{
    Altitude, Authorship, Camera, DateTime, Direction, ErrorKind, Focus, Format, Latitude, Limits,
    Location, Longitude, Metadata, North, Options, Point, Rational, RawFile,
};
use support::jpeg_builder::JpegBuilder;
use support::tiff_builder::{Order, TiffBuilder};

/// The Make tag: the camera's manufacturer, as text.
const MAKE: u16 = 0x010F;

/// The DNGVersion tag: the version of the DNG specification a DNG follows.
const DNG_VERSION: u16 = 0xC612;

/// The DNGPrivateData tag: a block of the camera maker's own data.
const DNG_PRIVATE_DATA: u16 = 0xC634;

/// The authorship tags of IFD0, each with a text of a painter's.
const IFD0_AUTHORSHIP: [(u16, &str); 3] = [
    (0x013B, "Claude Monet"),                  // Artist
    (0x8298, "Copyright Claude Monet"),        // Copyright
    (0x010E, "The port of Le Havre\nat dawn"), // ImageDescription
];

/// The authorship texts of the Exif IFD, but the comment.
const EXIF_AUTHORSHIP: [(u16, &str); 3] = [
    (0xA437, "Camille Pissarro"),          // Photographer
    (0xA438, "Édouard Manet"),             // ImageEditor
    (0xA436, "Impression, soleil levant"), // ImageTitle
];

/// The UserComment tag of the Exif IFD.
const USER_COMMENT: u16 = 0x9286;

/// The CameraOwnerName tag of the Exif IFD.
const CAMERA_OWNER_NAME: u16 = 0xA430;

/// A TIFF file whose IFD0 holds the Make `make` and [`IFD0_AUTHORSHIP`],
/// and whose Exif IFD holds [`EXIF_AUTHORSHIP`], in UTF-8, a comment under
/// the ASCII code and the camera's owner, in UTF-8.
fn with_authorship(make: &str) -> Vec<u8> {
    let mut builder = TiffBuilder::new(Order::Little);
    let (ifd0, exif) = (builder.ifd(), builder.ifd());
    builder.ascii(ifd0, MAKE, make).exif(ifd0, exif);
    for (tag, text) in IFD0_AUTHORSHIP {
        builder.ascii(ifd0, tag, text);
    }
    for (tag, text) in EXIF_AUTHORSHIP {
        builder.utf8(exif, tag, text);
    }
    builder.undefined(exif, USER_COMMENT, b"ASCII\0\0\0Painted at dawn   ");
    builder.utf8(exif, CAMERA_OWNER_NAME, "Paul Cézanne");
    builder.build().bytes
}

/// A TIFF file whose IFD0 holds the Make `make` and links to a GPS IFD
/// that records every part of a location, in the form a Sony camera
/// writes it, with the image direction and an altitude below its
/// reference: the Statue of Liberty's position and a made-up time.
fn with_location(make: &str) -> Vec<u8> {
    let mut builder = TiffBuilder::new(Order::Big);
    let (ifd0, gps) = (builder.ifd(), builder.ifd());
    builder.ascii(ifd0, MAKE, make).gps(ifd0, gps);
    builder
        .byte(gps, 0x0000, &[2, 3, 0, 0]) // GPSVersionID
        .ascii(gps, 0x0001, "N") // GPSLatitudeRef
        .rational(gps, 0x0002, &[(40, 1), (41, 1), (21_400, 1000)]) // GPSLatitude
        .ascii(gps, 0x0003, "W") // GPSLongitudeRef
        .rational(gps, 0x0004, &[(74, 1), (2, 1), (40_200, 1000)]) // GPSLongitude
        .byte(gps, 0x0005, &[1]) // GPSAltitudeRef
        .rational(gps, 0x0006, &[(2_500, 1000)]) // GPSAltitude
        .rational(gps, 0x0007, &[(13, 1), (5, 1), (21_450, 1000)]) // GPSTimeStamp
        .ascii(gps, 0x0009, "A") // GPSStatus
        .ascii(gps, 0x0010, "T") // GPSImgDirectionRef
        .rational(gps, 0x0011, &[(4_550, 100)]) // GPSImgDirection
        .ascii(gps, 0x0012, "WGS-84") // GPSMapDatum
        .ascii(gps, 0x001D, "2024:05:01"); // GPSDateStamp
    builder.build().bytes
}

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
    let options = Options::new().with_limits(Limits::default().with_max_ifds(1));
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
    let options = Options::new().with_limits(Limits::default().with_max_ifds(1));
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
fn an_authorship_and_a_camera_built_by_hand_equal_what_open_reads() {
    let expected = Authorship::default()
        .with_artist("Claude Monet")
        .with_copyright("Copyright Claude Monet")
        .with_description("The port of Le Havre\nat dawn")
        .with_photographer("Camille Pissarro")
        .with_editor("Édouard Manet")
        .with_title("Impression, soleil levant")
        .with_comment("Painted at dawn");
    let mut jpeg = JpegBuilder::new();
    jpeg.jfif().exif(&with_authorship("NIKON CORPORATION"));
    let camera = |make| {
        Camera::default()
            .with_recorded_make(make)
            .with_owner_name("Paul Cézanne")
    };
    for (bytes, format, make) in [
        (
            with_authorship("NIKON CORPORATION"),
            Format::Nef,
            "NIKON CORPORATION",
        ),
        (with_authorship("SONY"), Format::Arw, "SONY"),
        (jpeg.build().bytes, Format::Jpeg, "NIKON CORPORATION"),
    ] {
        let file = RawFile::from_bytes(&bytes).unwrap();
        assert_eq!(file.format(), format);
        assert_eq!(file.metadata().authorship(), &expected, "{format:?}");
        assert_eq!(file.metadata().camera(), &camera(make), "{format:?}");
    }
}

#[test]
fn a_location_built_by_hand_equals_what_open_reads() {
    let fraction = |numerator, denominator| Rational::new(numerator, denominator).unwrap();
    let expected = Location::new(
        Latitude::north(fraction(40, 1), fraction(41, 1), fraction(21_400, 1000)).unwrap(),
        Longitude::west(fraction(74, 1), fraction(2, 1), fraction(40_200, 1000)).unwrap(),
    )
    .with_altitude(Altitude::below(fraction(2_500, 1000)).unwrap())
    .with_time(
        DateTime::new(2024, 5, 1, 13, 5, 21)
            .and_then(|time| time.with_subseconds(450_000_000, 3))
            .and_then(|time| time.with_offset_minutes(0))
            .unwrap(),
    )
    .with_image_direction(Direction::new(fraction(4_550, 100), North::True).unwrap())
    .with_map_datum("WGS-84");
    let mut jpeg = JpegBuilder::new();
    jpeg.jfif().exif(&with_location("SONY"));
    for (bytes, format) in [
        (with_location("NIKON CORPORATION"), Format::Nef),
        (with_location("SONY"), Format::Arw),
        (jpeg.build().bytes, Format::Jpeg),
    ] {
        let file = RawFile::from_bytes(&bytes).unwrap();
        assert_eq!(file.format(), format);
        assert_eq!(file.metadata().location(), Some(&expected), "{format:?}");
    }
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
    assert_send::<Location>();
    assert_sync::<Location>();
}
