// SPDX-License-Identifier: MIT OR Apache-2.0

//! The NEF reader from outside the crate, on a file laid out like a Nikon
//! Z 9's: what it opens as, which previews it lists, what reading one
//! returns, and what metadata it gives, the focus point among it. Then the
//! previews and the metadata of a file laid out like a Z6III's on a 1:1
//! crop, whose focus point the reader moves from the full frame AFInfo2
//! records into the cropped picture. Then the previews and the metadata
//! of a file laid out like a D300's, an older body that records its serial
//! number and its lens's range only in its MakerNote, and no focus point,
//! shot through the viewfinder in manual focus. Then the one preview and
//! the metadata of a file laid out like a D1H's, whose MakerNote, of the
//! headerless layout, with its offsets counted from the start of the file,
//! holds the file's only JPEG and the ISO and the lens's range that its
//! Exif tags lack; and the damage and the limits that cost that MakerNote
//! what it gives, but not the file. Then two real files of my own Z 9, in
//! DX and 1:1 crop modes, committed with their pixels and personal fields
//! removed (see [`support::fixtures`]): their previews, every value of
//! their metadata, the camera settings among them, and their focus points,
//! which the reader scales onto the DX crop and moves into the 1:1 window.
//! Then the 56 real files of the session on which my own D600's
//! autofocus points were measured, committed stripped the same way: what
//! the reader gives each frame, the layout of the D600's points from its
//! table, its orientation, its image area and its focus points among it:
//! the points a frame shot through the viewfinder names, placed from the
//! table, in the crop's window on a DX frame, and the position a live-view
//! frame records; and, on frame 4, shot through the viewfinder with one
//! point, its previews and every value of its metadata, the point it names
//! placed from the table among them, and the layout it gives with its
//! MakerNote missing or unreadable. Then 35
//! real files too large to commit, three of the
//! Z 9 and 32 of 25 other bodies, when they have been downloaded (see
//! [`support::samples::path`]): the previews each gives, and its
//! metadata, every value of it, the focus point and the camera settings
//! among them.

mod support;

use std::collections::BTreeSet;
use std::io::Cursor;

use raw_files::{
    CameraSettings, ColorSpace, DateTime, Exposure, ExposureProgram, FineTune, Flash, FlashMode,
    Focus, FocusAreaMode, FocusOrigin, FocusPoint, FocusPoints, Format, ImageArea, Lens, Limits,
    Metadata, MeteringMode, Options, Orientation, Point, Preview, PreviewFormat, Rational, RawFile,
    RawImageSize, Shutter, SignedRational, Size, WhiteBalance, WhiteBalanceMode,
};
use support::nef::{
    D1H_ISO, D1H_LENS, D1H_PREVIEW_JPEG, D1H_RAW, D1hBlobs, d1h_file, d1h_file_with,
    d1h_file_with_blobs,
};
use support::nef::{
    D300_FULL_SIZE_JPEG, D300_LENS, D300_PREVIEW_JPEG, D300_RAW, D300_SERIAL_NUMBER,
    FULL_SIZE_JPEG, IFD0_JPEG, LENS_MODEL_Z14_30, LENS_MODEL_Z24_200, MEDIUM_JPEG, PREVIEW_JPEG,
    RAW_STRIP_START, Z6III_FOCUS_AREA, Z6III_FOCUS_CENTER, Z6III_FOCUS_FRAME, Z6III_FULL_SIZE_JPEG,
    Z6III_IMAGE, Z6III_IN_FOCUS, Z6III_MEDIUM_JPEG, Z6III_PREVIEW_JPEG, Z6III_RAW, Z9_FOCUS_AREA,
    Z9_FOCUS_CENTER, Z9_FOCUS_FRAME, Z9Blobs, d300_file, d300_file_with_blobs, stub, z6iii_file,
    z6iii_file_with_blobs, z9_file, z9_file_with, z9_file_with_blobs,
};
use support::samples::{
    Jpeg, Sample, SampleFineTune, SampleFocus, SampleLens, SampleMetadata, SampleSettings,
    SerialShape, check, check_metadata, listed, sizes,
};
use support::tiff_builder::{Built, Order};

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

/// `size` as a preview reports it.
fn wide((width, height): (u16, u16)) -> (u32, u32) {
    (u32::from(width), u32::from(height))
}

/// The length of the stub JPEG of `size`.
fn stub_len(size: (u16, u16)) -> u64 {
    u64::try_from(stub(size).len()).unwrap()
}

/// The sizes of the four JPEGs of a file laid out like the Z 9's: IFD0's,
/// the MakerNote's, SubIFD2's medium one and SubIFD0's full-size one,
/// smallest first.
type FourJpegs = [(u16, u16); 4];

/// The sizes of the Z 9 file's four JPEGs.
const Z9_JPEGS: FourJpegs = [IFD0_JPEG, PREVIEW_JPEG, MEDIUM_JPEG, FULL_SIZE_JPEG];

/// The sizes of the Z6III file's four JPEGs.
const Z6III_JPEGS: FourJpegs = [
    IFD0_JPEG,
    Z6III_PREVIEW_JPEG,
    Z6III_MEDIUM_JPEG,
    Z6III_FULL_SIZE_JPEG,
];

/// Checks that the file `built`, laid out like the Z 9's, whose blobs are
/// `blobs`, opens as a NEF with its four JPEGs of sizes `jpegs`, smallest
/// first, each where the fixture put it; `label` names the file in a
/// failure.
fn assert_the_four_jpegs(built: &Built, blobs: Z9Blobs, jpegs: FourJpegs, label: &str) {
    let [ifd0_jpeg, preview_jpeg, medium_jpeg, full_size_jpeg] = jpegs;
    let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
    let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
    for (how, format, previews) in [
        ("bytes", from_bytes.format(), from_bytes.previews()),
        ("reader", from_reader.format(), from_reader.previews()),
    ] {
        assert_eq!(format, Format::Nef, "{label}, from {how}");
        assert_eq!(sizes(previews), jpegs.map(wide), "{label}, from {how}");
        let expected = [
            (built.blob_offset(blobs.ifd0_jpeg), ifd0_jpeg),
            (
                built.blob_offset(blobs.makernote) + blobs.preview_in_makernote,
                preview_jpeg,
            ),
            (built.blob_offset(blobs.medium_jpeg), medium_jpeg),
            (built.blob_offset(blobs.full_size_jpeg), full_size_jpeg),
        ];
        for (preview, (offset, size)) in previews.iter().zip(expected) {
            assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label}, from {how}");
            assert_eq!(preview.byte_range().start, offset, "{label}, from {how}");
            assert_eq!(preview.byte_len(), stub_len(size), "{label}, from {how}");
        }
    }
}

#[test]
fn the_z9_file_opens_as_nef_with_its_four_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = z9_file_with_blobs(order, 1);
        assert_the_four_jpegs(&built, blobs, Z9_JPEGS, &format!("{order:?}"));
    }
}

#[test]
fn a_makernote_in_the_other_byte_order_from_the_files_gives_the_same_four_jpegs() {
    for (order, makernote_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
        let (built, blobs) = z9_file_with(order, 1, makernote_order);
        assert_the_four_jpegs(
            &built,
            blobs,
            Z9_JPEGS,
            &format!("{order:?}, MakerNote {makernote_order:?}"),
        );
    }
}

#[test]
fn reading_the_makernote_preview_returns_exactly_the_stubs_bytes() {
    let built = z9_file(Order::Big, 1);
    let mut file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
    let preview = file.previews()[1];
    assert_eq!((preview.width(), preview.height()), wide(PREVIEW_JPEG));
    assert_eq!(file.read_preview(&preview).unwrap(), stub(PREVIEW_JPEG));
}

#[test]
fn the_largest_preview_is_the_full_size_jpeg_and_at_least_picks_the_smallest_that_fits() {
    let built = z9_file(Order::Little, 1);
    let file = RawFile::from_bytes(&built.bytes).unwrap();
    let size = |preview: Option<Preview>| preview.map(|p| (p.width(), p.height()));
    assert_eq!(size(file.largest_preview()), Some(wide(FULL_SIZE_JPEG)));
    assert_eq!(
        size(file.preview_at_least(1000, 600)),
        Some(wide(MEDIUM_JPEG))
    );
    assert_eq!(
        size(file.preview_at_least(600, 400)),
        Some(wide(PREVIEW_JPEG))
    );
    assert_eq!(size(file.preview_at_least(100, 100)), Some(wide(IFD0_JPEG)));
    // Wider than any preview: none fits, so the largest is given.
    assert_eq!(
        size(file.preview_at_least(9000, 100)),
        Some(wide(FULL_SIZE_JPEG))
    );
}

#[test]
fn every_preview_takes_ifd0s_orientation() {
    for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
        let built = z9_file(Order::Big, value);
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        assert_eq!(file.previews().len(), 4, "Orientation {value}");
        for preview in file.previews() {
            assert_eq!(preview.orientation(), orientation, "Orientation {value}");
        }
    }
}

/// The fraction `numerator`/`denominator`, as stored.
fn fraction(numerator: u32, denominator: u32) -> Rational {
    Rational {
        numerator,
        denominator,
    }
}

/// Checks that the Z 9 file `built`, whose IFD0's Orientation is
/// `orientation`, gives the camera, the software, the serial number, the
/// capture time, the exposure, the lens, the sizes and the focus point the
/// fixture records, and that orientation, from bytes and from a reader,
/// and no camera settings, since it records none; `label` names the file
/// in a failure.
fn assert_the_z9_metadata(built: &Built, orientation: Orientation, label: &str) {
    let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
    let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
    for (how, metadata, previews) in [
        ("bytes", from_bytes.metadata(), from_bytes.previews()),
        ("reader", from_reader.metadata(), from_reader.previews()),
    ] {
        assert_eq!(
            metadata.make(),
            Some("NIKON CORPORATION"),
            "{label}, from {how}"
        );
        assert_eq!(metadata.model(), Some("NIKON Z 9"), "{label}, from {how}");
        // Read through the NULs that pad it.
        assert_eq!(
            metadata.software(),
            Some("Ver.01.00"),
            "{label}, from {how}"
        );
        assert_eq!(
            metadata.serial_number(),
            Some("1234567"),
            "{label}, from {how}"
        );
        let time = metadata.capture_time().unwrap();
        assert_eq!(
            time.to_string(),
            "2022-02-05T13:46:41.85-05:00",
            "{label}, from {how}"
        );
        assert_eq!(
            (time.year(), time.month(), time.day()),
            (2022, 2, 5),
            "{label}, from {how}"
        );
        assert_eq!(
            (time.hour(), time.minute(), time.second()),
            (13, 46, 41),
            "{label}, from {how}"
        );
        assert_eq!(time.nanosecond(), 850_000_000, "{label}, from {how}");
        assert_eq!(time.offset_minutes(), Some(-300), "{label}, from {how}");
        assert_eq!(metadata.orientation(), orientation, "{label}, from {how}");
        // The orientation is the one every preview takes.
        assert_eq!(previews.len(), 4, "{label}, from {how}");
        for preview in previews {
            assert_eq!(preview.orientation(), orientation, "{label}, from {how}");
        }
        // The exposure as stored: 1/320 s at f/9, 18 mm, 18 mm in 35 mm
        // terms too, and ISO 64, with no compensation.
        let exposure = metadata.exposure();
        assert_eq!(exposure.iso(), Some(64), "{label}, from {how}");
        assert_eq!(
            exposure.exposure_time(),
            Some(fraction(10, 3200)),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.f_number(),
            Some(fraction(900, 100)),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.focal_length(),
            Some(fraction(180, 10)),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.focal_length_in_35mm_film(),
            Some(18),
            "{label}, from {how}"
        );
        assert_eq!(
            exposure.exposure_bias(),
            Some(SignedRational {
                numerator: 0,
                denominator: 6,
            }),
            "{label}, from {how}"
        );
        // The lens, its model read whole through the NULs that pad it and
        // its serial number trimmed of the spaces that pad it.
        let lens = metadata.lens().unwrap();
        assert_eq!(lens.make(), Some("NIKON"), "{label}, from {how}");
        assert_eq!(lens.model(), Some(LENS_MODEL_Z14_30), "{label}, from {how}");
        assert_eq!(
            lens.serial_number(),
            Some("12345678"),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.min_focal_length(),
            Some(fraction(140, 10)),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.max_focal_length(),
            Some(fraction(300, 10)),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.f_number_at_min_focal_length(),
            Some(fraction(400, 100)),
            "{label}, from {how}"
        );
        assert_eq!(
            lens.f_number_at_max_focal_length(),
            Some(fraction(400, 100)),
            "{label}, from {how}"
        );
        // The raw size is SubIFD1's, the raw data's; with no default crop
        // in the file, the image size is the MakerNote's crop area, the
        // raw less its border.
        let raw = Size {
            width: 8280,
            height: 5520,
        };
        let image = Size {
            width: 8256,
            height: 5504,
        };
        assert_eq!(metadata.raw_size(), Some(raw), "{label}, from {how}");
        assert_eq!(metadata.image_size(), Some(image), "{label}, from {how}");
        // The focus point, from the MakerNote's AFInfo2, as recorded: the
        // centre and the area in the camera's full-size frame, with focus
        // confirmed.
        let focus = metadata.focus().unwrap();
        assert_eq!(
            focus.frame(),
            as_size(Z9_FOCUS_FRAME),
            "{label}, from {how}"
        );
        let (x, y) = Z9_FOCUS_CENTER;
        assert_eq!(focus.center(), Point { x, y }, "{label}, from {how}");
        assert_eq!(
            focus.size(),
            Some(as_size(Z9_FOCUS_AREA)),
            "{label}, from {how}"
        );
        assert_eq!(focus.in_focus(), Some(true), "{label}, from {how}");
        // The focus points: that point alone, the primary, with no number,
        // recorded, in the area mode 193 the fixture records, single point.
        assert_the_z9_points(metadata, &format!("{label}, from {how}"));
        // No layout: raw-files has no table of the Z 9's points.
        assert_eq!(metadata.focus_layout(), None, "{label}, from {how}");
        // The fixture writes no settings tag, so the settings are the empty
        // record: no value is made up.
        assert_eq!(
            metadata.camera_settings(),
            &CameraSettings::default(),
            "{label}, from {how}"
        );
    }
}

/// `(width, height)` as a [`Size`].
fn as_size((width, height): (u32, u32)) -> Size {
    Size { width, height }
}

/// Checks that `metadata`'s focus points are its focus alone, as a Z 9
/// file in single-point AF gives them: one point, the primary, with no
/// number, recorded, in the area mode single point, the code 193 that the
/// Z 9's AFInfo2 records. The focus is the list's primary by
/// construction, so the checks of the focus check the point. `label`
/// names the file in a failure.
fn assert_the_z9_points(metadata: &Metadata, label: &str) {
    let points = metadata
        .focus_points()
        .unwrap_or_else(|| panic!("{label}: no focus points"));
    assert_eq!(points.points().len(), 1, "{label}");
    let primary = points
        .primary()
        .unwrap_or_else(|| panic!("{label}: no primary"));
    assert_eq!(Some(primary), points.points().first(), "{label}");
    assert_eq!(primary.number(), None, "{label}");
    assert_eq!(points.origin(), FocusOrigin::Recorded, "{label}");
    assert_eq!(points.area_mode(), Some(FocusAreaMode::Single), "{label}");
}

#[test]
fn the_z9_file_gives_every_value_its_fixture_records() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let built = z9_file(order, value);
            assert_the_z9_metadata(
                &built,
                orientation,
                &format!("{order:?}, Orientation {value}"),
            );
        }
    }
}

/// The metadata [`assert_the_z9_metadata`] checks, built as a caller's
/// test would, through the public constructors and the `with_…` methods
/// alone, with the orientation `orientation`; `None` when a constructor
/// refuses a value.
fn the_z9_metadata_by_hand(orientation: Orientation) -> Option<Metadata> {
    let time = DateTime::new(2022, 2, 5, 13, 46, 41)?
        .with_subseconds(850_000_000, 2)?
        .with_offset_minutes(-300)?;
    let exposure = Exposure::default()
        .with_iso(64)
        .with_exposure_time(Rational::new(10, 3200)?)
        .with_f_number(Rational::new(900, 100)?)
        .with_focal_length(Rational::new(180, 10)?)
        .with_focal_length_in_35mm_film(18)
        .with_exposure_bias(SignedRational::new(0, 6)?);
    let lens = Lens::builder()
        .make("NIKON")
        .model(LENS_MODEL_Z14_30)
        .serial_number("12345678")
        .min_focal_length(Rational::new(140, 10)?)
        .max_focal_length(Rational::new(300, 10)?)
        .f_number_at_min_focal_length(Rational::new(400, 100)?)
        .f_number_at_max_focal_length(Rational::new(400, 100)?)
        .build()?;
    let (x, y) = Z9_FOCUS_CENTER;
    let focus = Focus::new(as_size(Z9_FOCUS_FRAME), Point { x, y })?
        .with_size(as_size(Z9_FOCUS_AREA))?
        .with_in_focus(true);
    let metadata = Metadata::default()
        .with_make("NIKON CORPORATION")
        .with_model("NIKON Z 9")
        .with_software("Ver.01.00")
        .with_serial_number("1234567")
        .with_capture_time(time)
        .with_orientation(orientation)
        .with_exposure(exposure)
        .with_lens(lens)
        .with_raw_size(Size {
            width: 8280,
            height: 5520,
        })
        .with_image_size(Size {
            width: 8256,
            height: 5504,
        })
        // The fixture's AFInfo2 records the area mode 193, single point.
        .with_focus_points(FocusPoints::from(focus).with_area_mode(FocusAreaMode::Single));
    Some(metadata)
}

#[test]
fn the_z9_metadata_built_by_hand_equals_what_open_reads() {
    // The one test that ties the builders to the reader: a builder that
    // wrote the wrong value, or a reader that read one, makes the two
    // differ.
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let label = format!("{order:?}, Orientation {value}");
            let expected = the_z9_metadata_by_hand(orientation)
                .expect("every value of the Z 9 fixture is one the constructors accept");
            let built = z9_file(order, value);
            let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
            let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
            assert_eq!(from_bytes.metadata(), &expected, "{label}, from bytes");
            assert_eq!(from_reader.metadata(), &expected, "{label}, from reader");
        }
    }
}

#[test]
fn a_makernote_in_the_other_byte_order_from_the_files_gives_the_same_metadata() {
    for (order, makernote_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
        let (built, _) = z9_file_with(order, 8, makernote_order);
        assert_the_z9_metadata(
            &built,
            Orientation::Rotate270,
            &format!("{order:?}, MakerNote {makernote_order:?}"),
        );
    }
}

#[test]
fn reading_the_largest_preview_returns_exactly_the_stubs_bytes() {
    let built = z9_file(Order::Little, 1);
    let mut file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
    let largest = file.largest_preview().unwrap();
    let bytes = file.read_preview(&largest).unwrap();
    assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF]);
    assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9]);
    assert_eq!(u64::try_from(bytes.len()).unwrap(), largest.byte_len());
    assert_eq!(bytes, stub(FULL_SIZE_JPEG));
}

#[test]
fn the_raw_strip_the_grey_image_and_the_empty_subifds_are_not_previews() {
    for order in ORDERS {
        let (built, blobs) = z9_file_with_blobs(order, 1);
        // The raw strip is where the fixture says, so that the check below
        // looks at the right offset.
        let raw = usize::try_from(built.blob_offset(blobs.raw)).unwrap();
        assert_eq!(built.bytes[raw..raw + 4], RAW_STRIP_START, "{order:?}");

        let file = RawFile::from_bytes(&built.bytes).unwrap();
        let starts: Vec<u64> = file
            .previews()
            .iter()
            .map(|p| p.byte_range().start)
            .collect();
        assert!(
            !starts.contains(&built.blob_offset(blobs.raw)),
            "{order:?}: {starts:?}"
        );
        assert!(
            !starts.contains(&built.blob_offset(blobs.grey)),
            "{order:?}: {starts:?}"
        );
        // The empty SubIFDs declare a strip at offset 0.
        assert!(!starts.contains(&0), "{order:?}: {starts:?}");
        assert_eq!(starts.len(), 4, "{order:?}");
    }
}

#[test]
fn the_z6iii_file_opens_as_nef_with_its_four_jpegs_smallest_first() {
    for order in ORDERS {
        let (built, blobs) = z6iii_file_with_blobs(order, 1);
        assert_the_four_jpegs(&built, blobs, Z6III_JPEGS, &format!("{order:?}"));
    }
}

#[test]
fn the_z6iii_file_gives_its_focus_point_on_the_cropped_picture() {
    // A 1:1 crop: AFInfo2 records the point in the camera's full 6048 by
    // 4032 frame, and the reader moves it into the 4032 by 4032 window
    // that CropArea is, keeping the area, as on the public sample 7808.
    for order in ORDERS {
        let built = z6iii_file(order, 1);
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        let metadata = file.metadata();
        assert_eq!(metadata.make(), Some("NIKON CORPORATION"), "{order:?}");
        assert_eq!(metadata.model(), Some("NIKON Z6_3"), "{order:?}");
        assert_eq!(metadata.raw_size(), Some(as_size(Z6III_RAW)), "{order:?}");
        assert_eq!(
            metadata.image_size(),
            Some(as_size(Z6III_IMAGE)),
            "{order:?}"
        );
        let focus = metadata.focus().unwrap();
        assert_eq!(focus.frame(), as_size(Z6III_FOCUS_FRAME), "{order:?}");
        let (x, y) = Z6III_FOCUS_CENTER;
        assert_eq!(focus.center(), Point { x, y }, "{order:?}");
        assert_eq!(focus.size(), Some(as_size(Z6III_FOCUS_AREA)), "{order:?}");
        assert_eq!(focus.in_focus(), Some(Z6III_IN_FOCUS), "{order:?}");
        // The frame is the picture's, so the point scales onto the square
        // previews as it is.
        assert_eq!(
            focus.frame(),
            as_size(wide(Z6III_FULL_SIZE_JPEG)),
            "{order:?}"
        );
        // No layout: raw-files has no table of the Z6III's points.
        assert_eq!(metadata.focus_layout(), None, "{order:?}");
    }
}

/// The metadata of the Z6III file, with the orientation `orientation`,
/// built as a caller's test would, through the public constructors and
/// the `with_…` methods alone; `None` when a constructor refuses a value.
fn the_z6iii_metadata_by_hand(orientation: Orientation) -> Option<Metadata> {
    let time = DateTime::new(2024, 6, 29, 15, 38, 53)?
        .with_subseconds(420_000_000, 2)?
        .with_offset_minutes(-240)?;
    let exposure = Exposure::default()
        .with_iso(200)
        .with_exposure_time(Rational::new(10, 1000)?)
        .with_f_number(Rational::new(900, 100)?)
        .with_focal_length(Rational::new(860, 10)?)
        .with_focal_length_in_35mm_film(111)
        .with_exposure_bias(SignedRational::new(0, 6)?);
    let lens = Lens::builder()
        .make("NIKON")
        .model(LENS_MODEL_Z24_200)
        .serial_number("23456789")
        .min_focal_length(Rational::new(240, 10)?)
        .max_focal_length(Rational::new(2000, 10)?)
        .f_number_at_min_focal_length(Rational::new(400, 100)?)
        .f_number_at_max_focal_length(Rational::new(630, 100)?)
        .build()?;
    let (x, y) = Z6III_FOCUS_CENTER;
    let focus = Focus::new(as_size(Z6III_FOCUS_FRAME), Point { x, y })?
        .with_size(as_size(Z6III_FOCUS_AREA))?
        .with_in_focus(Z6III_IN_FOCUS);
    // Its MakerNote's one settings tag is CropHiSpeed, whose crop setting,
    // 17, FX 1:1, gives the image area.
    let metadata = Metadata::default()
        .with_make("NIKON CORPORATION")
        .with_model("NIKON Z6_3")
        .with_software("Ver.01.00")
        .with_serial_number("2345678")
        .with_capture_time(time)
        .with_orientation(orientation)
        .with_exposure(exposure)
        .with_lens(lens)
        .with_raw_size(as_size(Z6III_RAW))
        .with_image_size(as_size(Z6III_IMAGE))
        .with_focus(focus)
        .with_camera_settings(CameraSettings::default().with_image_area(ImageArea::Square));
    Some(metadata)
}

#[test]
fn the_z6iii_metadata_built_by_hand_equals_what_open_reads() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let label = format!("{order:?}, Orientation {value}");
            let expected = the_z6iii_metadata_by_hand(orientation)
                .expect("every value of the Z6III fixture is one the constructors accept");
            let built = z6iii_file(order, value);
            let from_bytes = RawFile::from_bytes(&built.bytes).unwrap();
            let from_reader = RawFile::from_reader(Cursor::new(built.bytes.clone())).unwrap();
            assert_eq!(from_bytes.metadata(), &expected, "{label}, from bytes");
            assert_eq!(from_reader.metadata(), &expected, "{label}, from reader");
        }
    }
}

#[test]
fn the_d300_file_opens_as_nef_with_its_two_jpegs_smallest_first() {
    // The MakerNote's small JPEG and SubIFD0's full-size one; IFD0's
    // uncompressed thumbnail is not a JPEG, and the raw strip is not a
    // preview, as on the public D300 sample 928.
    for order in ORDERS {
        let (built, blobs) = d300_file_with_blobs(order, 1);
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        assert_eq!(file.format(), Format::Nef, "{order:?}");
        let previews = file.previews();
        assert_eq!(
            sizes(previews),
            [wide(D300_PREVIEW_JPEG), wide(D300_FULL_SIZE_JPEG)],
            "{order:?}"
        );
        let expected = [
            (
                built.blob_offset(blobs.makernote) + blobs.preview_in_makernote,
                D300_PREVIEW_JPEG,
            ),
            (built.blob_offset(blobs.full_size_jpeg), D300_FULL_SIZE_JPEG),
        ];
        for (preview, (offset, size)) in previews.iter().zip(expected) {
            assert_eq!(preview.format(), PreviewFormat::Jpeg, "{order:?}");
            assert_eq!(preview.byte_range().start, offset, "{order:?}");
            assert_eq!(preview.byte_len(), stub_len(size), "{order:?}");
        }
        let starts: Vec<u64> = previews.iter().map(|p| p.byte_range().start).collect();
        for strip in [blobs.thumbnail, blobs.raw] {
            assert!(
                !starts.contains(&built.blob_offset(strip)),
                "{order:?}: {starts:?}"
            );
        }
    }
}

#[test]
fn the_d300_file_gives_what_its_exif_tags_lack_from_the_makernote() {
    for order in ORDERS {
        for (value, orientation) in [(8, Orientation::Rotate270), (1, Orientation::NoTransforms)] {
            let label = format!("{order:?}, Orientation {value}");
            let built = d300_file(order, value);
            let file = RawFile::from_bytes(&built.bytes).unwrap();
            let metadata = file.metadata();
            assert_eq!(metadata.make(), Some("NIKON CORPORATION"), "{label}");
            assert_eq!(metadata.model(), Some("NIKON D300"), "{label}");
            // Trimmed of the trailing space the D300 writes.
            assert_eq!(metadata.software(), Some("Ver.1.10"), "{label}");
            // No BodySerialNumber in the Exif IFD: the MakerNote's.
            assert_eq!(
                metadata.serial_number(),
                Some(D300_SERIAL_NUMBER),
                "{label}"
            );
            // No OffsetTimeOriginal: the offset from UTC is the MakerNote's
            // WorldTime, a zone of 60 minutes without daylight saving time.
            let time = metadata.capture_time().unwrap();
            assert_eq!(time.to_string(), "2008-05-17T10:22:31.34+01:00", "{label}");
            assert_eq!(time.offset_minutes(), Some(60), "{label}");
            assert_eq!(metadata.orientation(), orientation, "{label}");
            // The exposure as stored: 1/10 s at f/7.1, 50 mm, 75 mm in 35 mm
            // terms, and ISO 200, with no compensation. The ISO is the Exif
            // one, which the MakerNote's `[0, 200]` agrees with.
            let exposure = metadata.exposure();
            assert_eq!(exposure.iso(), Some(200), "{label}");
            assert_eq!(exposure.exposure_time(), Some(fraction(10, 100)), "{label}");
            assert_eq!(exposure.f_number(), Some(fraction(71, 10)), "{label}");
            assert_eq!(exposure.focal_length(), Some(fraction(500, 10)), "{label}");
            assert_eq!(exposure.focal_length_in_35mm_film(), Some(75), "{label}");
            assert_eq!(
                exposure.exposure_bias(),
                Some(SignedRational {
                    numerator: 0,
                    denominator: 6,
                }),
                "{label}"
            );
            // No lens tag in the Exif IFD: a lens of the MakerNote's range
            // alone, with no name and no serial number.
            let lens = metadata.lens().unwrap();
            assert_eq!(
                (lens.make(), lens.model(), lens.serial_number()),
                (None, None, None),
                "{label}"
            );
            let [min_focal, max_focal, f_at_min, f_at_max] =
                D300_LENS.map(|(n, d)| Some(fraction(n, d)));
            assert_eq!(lens.min_focal_length(), min_focal, "{label}");
            assert_eq!(lens.max_focal_length(), max_focal, "{label}");
            assert_eq!(lens.f_number_at_min_focal_length(), f_at_min, "{label}");
            assert_eq!(lens.f_number_at_max_focal_length(), f_at_max, "{label}");
            // No CropArea and no default crop: the image size is the raw
            // size, border pixels included.
            let raw = Size {
                width: D300_RAW.0,
                height: D300_RAW.1,
            };
            assert_eq!(metadata.raw_size(), Some(raw), "{label}");
            assert_eq!(metadata.image_size(), Some(raw), "{label}");
            // A shot through the viewfinder in manual focus, as on the
            // sample: AFInfo2 records a zero frame, and FocusMode `MANUAL`,
            // so no focus point.
            assert_eq!(metadata.focus(), None, "{label}");
            // No layout: raw-files has no table of the D300's points.
            assert_eq!(metadata.focus_layout(), None, "{label}");
            // The fixture writes no settings tag, so the settings are the
            // empty record.
            assert_eq!(
                metadata.camera_settings(),
                &CameraSettings::default(),
                "{label}"
            );
        }
    }
}

/// The metadata of the D1H file, built as a caller's test would, through
/// the public constructors and the `with_…` methods alone: what its
/// standard tags give, with the ISO, the lens's range and the white
/// balance setting, `SUNNY`, daylight, that its MakerNote alone records
/// when `with_makernote`; `None` when a constructor refuses a value. It
/// records no serial number, no offset from UTC, no orientation, no focal
/// length in 35 mm terms, no focus point and no other camera setting
/// either way.
fn the_d1h_metadata_by_hand(with_makernote: bool) -> Option<Metadata> {
    let time = DateTime::new(2022, 3, 24, 10, 9, 27)?.with_subseconds(10_000_000, 2)?;
    let mut exposure = Exposure::default()
        .with_exposure_time(Rational::new(10, 8000)?)
        .with_f_number(Rational::new(45, 10)?)
        .with_focal_length(Rational::new(600, 10)?)
        .with_exposure_bias(SignedRational::new(4, 6)?);
    let mut metadata = Metadata::default()
        .with_make("NIKON CORPORATION")
        .with_model("NIKON D1H")
        .with_software("Ver.1.00")
        .with_capture_time(time)
        .with_raw_size(as_size(D1H_RAW))
        .with_image_size(as_size(D1H_RAW));
    if with_makernote {
        exposure = exposure.with_iso(u32::from(D1H_ISO));
        let [
            (min, min_d),
            (max, max_d),
            (at_min, at_min_d),
            (at_max, at_max_d),
        ] = D1H_LENS;
        let lens = Lens::builder()
            .min_focal_length(Rational::new(min, min_d)?)
            .max_focal_length(Rational::new(max, max_d)?)
            .f_number_at_min_focal_length(Rational::new(at_min, at_min_d)?)
            .f_number_at_max_focal_length(Rational::new(at_max, at_max_d)?)
            .build()?;
        metadata = metadata.with_lens(lens).with_camera_settings(
            CameraSettings::default()
                .with_white_balance(WhiteBalance::default().with_mode(WhiteBalanceMode::Daylight)),
        );
    }
    Some(metadata.with_exposure(exposure))
}

/// Checks that the D1H file `built`, whose parts are where `blobs` says,
/// opens with `options` as a NEF, from bytes and from a reader, with the
/// MakerNote's JPEG as its one preview when `preview`, and with no preview
/// otherwise, and with the metadata [`the_d1h_metadata_by_hand`] gives
/// with or without what the MakerNote records, as `with_makernote` says;
/// `label` names the file in a failure.
fn assert_the_d1h_file(
    options: &Options,
    built: &Built,
    blobs: &D1hBlobs,
    preview: bool,
    with_makernote: bool,
    label: &str,
) {
    let expected_metadata = the_d1h_metadata_by_hand(with_makernote)
        .expect("every value of the D1H fixture is one the constructors accept");
    let start = built.blob_offset(blobs.preview_jpeg);
    let len = stub_len(D1H_PREVIEW_JPEG);
    let expected_previews: Vec<_> = [(start..start + len, wide(D1H_PREVIEW_JPEG))]
        .into_iter()
        .filter(|_| preview)
        .collect();
    let from_bytes = options.from_bytes(&built.bytes).unwrap();
    let from_reader = options
        .from_reader(Cursor::new(built.bytes.clone()))
        .unwrap();
    for (how, format, previews, metadata) in [
        (
            "bytes",
            from_bytes.format(),
            from_bytes.previews(),
            from_bytes.metadata(),
        ),
        (
            "reader",
            from_reader.format(),
            from_reader.previews(),
            from_reader.metadata(),
        ),
    ] {
        assert_eq!(format, Format::Nef, "{label}, from {how}");
        let found: Vec<_> = previews
            .iter()
            .map(|p| (p.byte_range(), (p.width(), p.height())))
            .collect();
        assert_eq!(found, expected_previews, "{label}, from {how}");
        for preview in previews {
            assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label}, from {how}");
            assert_eq!(
                preview.orientation(),
                Orientation::NoTransforms,
                "{label}, from {how}"
            );
        }
        assert_eq!(metadata, &expected_metadata, "{label}, from {how}");
    }
}

#[test]
fn the_d1h_file_opens_as_nef_with_its_makernotes_jpeg_alone() {
    // The one JPEG, in the PreviewIFD of the headerless MakerNote, whose
    // offsets count from the start of the file; IFD0's uncompressed
    // thumbnail is not a JPEG, and the raw strip is not a preview, as on
    // the public D1H sample 5364.
    for order in ORDERS {
        let (built, blobs) = d1h_file_with_blobs(order);
        // The MakerNote's block holds the PreviewIFD and the whole JPEG, as
        // the sample's does.
        let jpeg = built.blob_offset(blobs.preview_jpeg);
        assert!(blobs.makernote.contains(&blobs.preview_ifd), "{order:?}");
        assert_eq!(
            blobs.makernote.end,
            jpeg + stub_len(D1H_PREVIEW_JPEG),
            "{order:?}"
        );
        assert_the_d1h_file(
            &Options::new(),
            &built,
            &blobs,
            true,
            true,
            &format!("{order:?}"),
        );
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        let starts: Vec<u64> = file
            .previews()
            .iter()
            .map(|p| p.byte_range().start)
            .collect();
        for strip in [blobs.thumbnail, blobs.raw] {
            assert!(
                !starts.contains(&built.blob_offset(strip)),
                "{order:?}: {starts:?}"
            );
        }
    }
}

#[test]
fn reading_the_d1h_files_preview_returns_exactly_the_stubs_bytes() {
    for order in ORDERS {
        let built = d1h_file(order);
        let mut file = RawFile::from_reader(Cursor::new(built.bytes)).unwrap();
        let preview = file.largest_preview().unwrap();
        assert_eq!(
            file.read_preview(&preview).unwrap(),
            stub(D1H_PREVIEW_JPEG),
            "{order:?}"
        );
    }
}

#[test]
fn the_d1h_file_gives_what_its_exif_tags_lack_from_its_headerless_makernote() {
    for order in ORDERS {
        let built = d1h_file(order);
        let file = RawFile::from_bytes(&built.bytes).unwrap();
        let metadata = file.metadata();
        // No PhotographicSensitivity in the Exif IFD: the MakerNote's
        // second value.
        assert_eq!(
            metadata.exposure().iso(),
            Some(u32::from(D1H_ISO)),
            "{order:?}"
        );
        // No lens tag in the Exif IFD: a lens of the MakerNote's range
        // alone, with no name and no serial number.
        let lens = metadata.lens().unwrap();
        assert_eq!(
            (lens.make(), lens.model(), lens.serial_number()),
            (None, None, None),
            "{order:?}"
        );
        let [min_focal, max_focal, f_at_min, f_at_max] =
            D1H_LENS.map(|(n, d)| Some(fraction(n, d)));
        assert_eq!(lens.min_focal_length(), min_focal, "{order:?}");
        assert_eq!(lens.max_focal_length(), max_focal, "{order:?}");
        assert_eq!(lens.f_number_at_min_focal_length(), f_at_min, "{order:?}");
        assert_eq!(lens.f_number_at_max_focal_length(), f_at_max, "{order:?}");
        // No serial number in either, no WorldTime, and AFInfo's point
        // number alone, no AFInfo2.
        assert_eq!(metadata.serial_number(), None, "{order:?}");
        assert_eq!(
            metadata.capture_time().unwrap().offset_minutes(),
            None,
            "{order:?}"
        );
        assert_eq!(metadata.focus(), None, "{order:?}");
        // No layout: raw-files has no table of the D1H's points.
        assert_eq!(metadata.focus_layout(), None, "{order:?}");
    }
}

#[test]
fn a_d1h_makernote_declared_too_short_for_its_ifd_costs_what_it_gives() {
    // The MakerNote IFD's count and its 15 entries take 182 bytes: one
    // byte short of them, the IFD is not read, and neither is anything in
    // it. The file opens with its standard tags' values.
    for order in ORDERS {
        let (built, blobs) = d1h_file_with(order, &[], Some(2 + 12 * 15 - 1));
        assert_the_d1h_file(
            &Options::new(),
            &built,
            &blobs,
            false,
            false,
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_d1h_preview_ifd_outside_the_declared_block_is_not_read() {
    // Declared up to where the PreviewIFD starts: the PreviewIFD lies
    // inside the file, outside the MakerNote, and gives no preview. The
    // MakerNote IFD and its values, the ISO and the lens's range among
    // them, lie inside, and are read.
    for order in ORDERS {
        let (_, sound) = d1h_file_with_blobs(order);
        let declared = sound.preview_ifd - sound.makernote.start;
        let (built, blobs) = d1h_file_with(order, &[], Some(declared));
        assert_the_d1h_file(
            &Options::new(),
            &built,
            &blobs,
            false,
            true,
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_d1h_preview_running_past_the_declared_block_is_not_a_preview() {
    // Declared one byte short of the JPEG's end, which lies inside the
    // file: the JPEG is not a preview. The rest of the MakerNote is read.
    for order in ORDERS {
        let (_, sound) = d1h_file_with_blobs(order);
        let declared = sound.makernote.end - sound.makernote.start - 1;
        let (built, blobs) = d1h_file_with(order, &[], Some(declared));
        let jpeg_end = built.blob_offset(blobs.preview_jpeg) + stub_len(D1H_PREVIEW_JPEG);
        assert!(
            jpeg_end <= u64::try_from(built.bytes.len()).unwrap(),
            "{order:?}"
        );
        assert_the_d1h_file(
            &Options::new(),
            &built,
            &blobs,
            false,
            true,
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_short_header_makernote_under_a_nikon_make_is_not_read() {
    // The D1H file's MakerNote IFD after the 8 bytes `Nikon\0\1\0`, with
    // its offsets counted from the start of the file, as the sources
    // describe the short-header layout of the Coolpix E700 to E950: the
    // layout is not read, so the file gives nothing from it, though its
    // IFD is sound 8 bytes in.
    for order in ORDERS {
        let (built, blobs) = d1h_file_with(order, b"Nikon\0\x01\0", None);
        assert_the_d1h_file(
            &Options::new(),
            &built,
            &blobs,
            false,
            false,
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_makernote_of_text_under_a_nikon_make_gives_nothing() {
    // A block that starts with text, read as a bare IFD: its first two
    // bytes count thousands of entries, and the IFD after the text is
    // never reached. The file opens with its standard tags' values.
    for order in ORDERS {
        let (built, blobs) = d1h_file_with(order, b"Not a MakerNote IFD\0", None);
        assert_the_d1h_file(
            &Options::new(),
            &built,
            &blobs,
            false,
            false,
            &format!("{order:?}"),
        );
    }
}

#[test]
fn a_limit_crossed_inside_the_d1h_makernote_costs_the_makernote_only() {
    // The MakerNote IFD holds 15 entries, IFD0 14, the most of any IFD
    // outside the MakerNote: with a limit of 15 the MakerNote gives its
    // preview, ISO and lens's range; with 14, none of them, and the rest
    // of the file is as without it.
    for order in ORDERS {
        let (built, blobs) = d1h_file_with_blobs(order);
        for (entries, read) in [(15, true), (14, false)] {
            assert_the_d1h_file(
                &Options::new().limits(Limits::default().max_ifd_entries(entries)),
                &built,
                &blobs,
                read,
                read,
                &format!("{entries} entries, {order:?}"),
            );
        }
    }
}

// Two real files, committed. Two shots of my own Z 9, in DX and 1:1
// crop modes, with a single focus point placed in the top-left corner
// of the picture, stripped of their pixels and personal fields with
// their structure and values kept (`tests/fixtures/README.md`). Their
// point lies far enough from the centre to tell a crop the reader scales
// from one it shifts, which no public sample's point does.

/// What one of the two stripped Z 9 files must give, beside what both
/// give: the values that differ between the DX and the 1:1 shot.
struct StrippedZ9 {
    /// The file's name in `tests/fixtures/`.
    file: &'static str,
    /// The capture time, as [`DateTime`] displays it.
    capture_time: &'static str,
    /// The exposure time, as stored.
    exposure_time: (u32, u32),
    /// The focal length in 35 mm terms, which the crop changes.
    focal_length_in_35mm_film: u32,
    /// The size of the raw data, which the crop already cut.
    raw_size: (u32, u32),
    /// The size of the picture, the MakerNote's CropArea.
    image_size: (u32, u32),
    /// The four previews, smallest first: IFD0's, the MakerNote's,
    /// SubIFD2's and SubIFD0's.
    previews: [(u32, u32); 4],
    /// The frame, the centre and the size of the focus point the reader
    /// gives, on the cropped picture.
    focus_frame: (u32, u32),
    focus_center: (u32, u32),
    focus_size: (u32, u32),
    /// The colour temperature of the white balance the camera applied, in
    /// kelvin: the MakerNote's ColorTemperatureAuto.
    color_temperature: u32,
    /// The image area, from the crop setting of the MakerNote's
    /// CropHiSpeed.
    image_area: ImageArea,
}

/// The DX shot. CropHiSpeed `[2, 8280, 5520, 5408, 3608, 1436, 956]`
/// records a crop that cuts both sides, so the centre (636, 572) and the
/// area 420 by 468 that AFInfo2 records in the full 8256 by 5504 frame are
/// fractions of the cropped picture, and the reader scales them onto
/// CropArea's 5392 by 3592, each rounded to the nearest pixel.
const STRIPPED_Z9_DX: StrippedZ9 = StrippedZ9 {
    file: support::fixtures::Z9_DX,
    capture_time: "2026-10-19T19:26:32.53-03:00",
    exposure_time: (10, 1000),
    focal_length_in_35mm_film: 75,
    raw_size: (5408, 3608),
    image_size: (5392, 3592),
    previews: [(160, 120), (640, 424), (1620, 1080), (5392, 3592)],
    focus_frame: (5392, 3592),
    focus_center: (415, 373),
    focus_size: (274, 305),
    color_temperature: 2830,
    image_area: ImageArea::ApsC,
};

/// The 1:1 shot. CropHiSpeed `[17, 8280, 5520, 5520, 5520, 1380, 0]`
/// records a crop that keeps the height, so the centre (1797, 424) that
/// AFInfo2 records in the full 8256 by 5504 frame is a position in that
/// frame, of which CropArea's 5504 by 5504 is the centred window: the
/// reader moves x back by the window's origin, (8256 - 5504) / 2 = 1376,
/// and keeps the area, 291 by 323.
const STRIPPED_Z9_1X1: StrippedZ9 = StrippedZ9 {
    file: support::fixtures::Z9_1X1,
    capture_time: "2026-10-19T19:26:52.73-03:00",
    exposure_time: (10, 600),
    focal_length_in_35mm_film: 65,
    raw_size: (5520, 5520),
    image_size: (5504, 5504),
    previews: [(160, 120), (480, 480), (1080, 1080), (5504, 5504)],
    focus_frame: (5504, 5504),
    focus_center: (421, 424),
    focus_size: (291, 323),
    color_temperature: 2860,
    image_area: ImageArea::Square,
};

/// Checks that the stripped Z 9 file `expected` names opens as a NEF with
/// its four previews, smallest first, each a stub JPEG of 122 bytes that
/// reads whole, and gives every value of its metadata, the focus point
/// placed on the cropped picture among them.
fn check_stripped_z9(expected: &StrippedZ9) {
    let label = expected.file;
    let mut file = RawFile::open(support::fixtures::fixture(expected.file)).unwrap();
    assert_eq!(file.format(), Format::Nef, "{label}");

    // The stripping replaced each JPEG by a stub of 122 bytes with the
    // original's frame header, so each keeps its size.
    assert_eq!(sizes(file.previews()), expected.previews, "{label}");
    let previews = file.previews().to_vec();
    for preview in &previews {
        let size = (preview.width(), preview.height());
        assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label} {size:?}");
        assert_eq!(
            preview.orientation(),
            Orientation::NoTransforms,
            "{label} {size:?}"
        );
        assert_eq!(preview.byte_len(), 122, "{label} {size:?}");
        let bytes = file.read_preview(preview).unwrap();
        assert_eq!(bytes.len(), 122, "{label} {size:?}");
        assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF], "{label} {size:?}");
        assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9], "{label} {size:?}");
    }
    assert_eq!(file.largest_preview(), previews.last().copied(), "{label}");

    let metadata = file.metadata();
    assert_eq!(metadata.make(), Some("NIKON CORPORATION"), "{label}");
    assert_eq!(metadata.model(), Some("NIKON Z 9"), "{label}");
    // The firmware my Z 9 ran.
    assert_eq!(metadata.software(), Some("Ver.05.30"), "{label}");
    // The stripping removed BodySerialNumber and the MakerNote's
    // SerialNumber.
    assert_eq!(metadata.serial_number(), None, "{label}");
    assert_eq!(
        metadata
            .capture_time()
            .map(|time| time.to_string())
            .as_deref(),
        Some(expected.capture_time),
        "{label}"
    );
    assert_eq!(metadata.orientation(), Orientation::NoTransforms, "{label}");
    // The exposure as stored: ISO 100 at f/1.4 and 50 mm, with no
    // compensation.
    let exposure = metadata.exposure();
    assert_eq!(exposure.iso(), Some(100), "{label}");
    let (numerator, denominator) = expected.exposure_time;
    assert_eq!(
        exposure.exposure_time(),
        Some(fraction(numerator, denominator)),
        "{label}"
    );
    assert_eq!(exposure.f_number(), Some(fraction(140, 100)), "{label}");
    assert_eq!(exposure.focal_length(), Some(fraction(500, 10)), "{label}");
    assert_eq!(
        exposure.focal_length_in_35mm_film(),
        Some(expected.focal_length_in_35mm_film),
        "{label}"
    );
    assert_eq!(
        exposure.exposure_bias(),
        Some(SignedRational {
            numerator: 0,
            denominator: 6,
        }),
        "{label}"
    );
    // The lens from the Exif IFD: a LensMake that is empty, which gives no
    // make, a model, no serial number, since the stripping removed it, and
    // the range of a 50 mm f/1.4 prime.
    let lens = metadata
        .lens()
        .unwrap_or_else(|| panic!("{label}: no lens"));
    assert_eq!(
        (lens.make(), lens.model(), lens.serial_number()),
        (None, Some("50mm f/1.4G"), None),
        "{label}"
    );
    assert_eq!(lens.min_focal_length(), Some(fraction(500, 10)), "{label}");
    assert_eq!(lens.max_focal_length(), Some(fraction(500, 10)), "{label}");
    assert_eq!(
        lens.f_number_at_min_focal_length(),
        Some(fraction(140, 100)),
        "{label}"
    );
    assert_eq!(
        lens.f_number_at_max_focal_length(),
        Some(fraction(140, 100)),
        "{label}"
    );
    // The raw size is the raw SubIFD's, already cropped; with no default
    // crop in the file, the image size is the MakerNote's CropArea.
    assert_eq!(
        metadata.raw_size(),
        Some(as_size(expected.raw_size)),
        "{label}"
    );
    assert_eq!(
        metadata.image_size(),
        Some(as_size(expected.image_size)),
        "{label}"
    );
    // The focus point, placed on the cropped picture by the crop rule (see
    // the two files' constants), with focus confirmed.
    let focus = metadata
        .focus()
        .unwrap_or_else(|| panic!("{label}: no focus point"));
    assert_eq!(focus.frame(), as_size(expected.focus_frame), "{label}");
    let (x, y) = expected.focus_center;
    assert_eq!(focus.center(), Point { x, y }, "{label}");
    assert_eq!(focus.size(), Some(as_size(expected.focus_size)), "{label}");
    assert_eq!(focus.in_focus(), Some(true), "{label}");
    // The frame is the picture's, so the point scales onto the full-size
    // preview as it is.
    assert_eq!(focus.frame(), as_size(expected.previews[3]), "{label}");
    // The focus points: that point alone, in the area mode 193 both files'
    // AFInfo2 records, single point.
    assert_the_z9_points(metadata, label);
    // No layout: raw-files has no table of the Z 9's points.
    assert_eq!(metadata.focus_layout(), None, "{label}");
    // The settings, the same on both shots but the colour temperature and
    // the image area. From the Exif IFD: aperture priority, matrix
    // metering, a Flash of 0, no flash fired and no mode recorded, and auto
    // white balance. From the MakerNote: the colour temperature auto white
    // balance chose; the electronic shutter; the image area of the crop;
    // the large raw size; and, since the Exif IFD records no ColorSpace, as
    // in every Nikon raw file checked, the MakerNote's colour space, sRGB.
    // The stripping removed the MakerNote's WhiteBalance text and its
    // fine-tuning, 0x003F, whose values are stored outside their entries,
    // so the white balance mode is Exif's, and since the Z 9 writes no
    // 0x000B, there is no fine-tuning.
    assert_eq!(
        metadata.camera_settings(),
        &CameraSettings::default()
            .with_white_balance(
                WhiteBalance::default()
                    .with_mode(WhiteBalanceMode::Auto)
                    .with_color_temperature(expected.color_temperature),
            )
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(Flash::new(false))
            .with_shutter(Shutter::Electronic)
            .with_image_area(expected.image_area)
            .with_raw_image_size(RawImageSize::Large)
            .with_color_space(ColorSpace::Srgb),
        "{label}"
    );
}

#[test]
fn the_stripped_z9_dx_file_gives_its_previews_metadata_and_scaled_focus_point() {
    check_stripped_z9(&STRIPPED_Z9_DX);
}

#[test]
fn the_stripped_z9_1x1_file_gives_its_previews_metadata_and_shifted_focus_point() {
    check_stripped_z9(&STRIPPED_Z9_1X1);
}

// Real files, committed: the 56 frames of the session on which the
// autofocus points of my own D600 were measured, numbered 1 to 56 in the
// order they were shot, stripped of their pixels and personal fields with
// their structure and values kept (`tests/fixtures/nikon-d600/README.md`),
// so that what the NEF notes' record of the session reads from the files'
// bytes can be checked on them (`docs/formats/nef/autofocus-points.md`,
// section 2). Every frame gives the layout of the D600's points from the
// table raw-files holds, written from that record, and every frame shot
// through the viewfinder in autofocus the points it names, placed from it.

/// What the reader gives of a D600 frame's focus.
#[derive(Clone, Copy, Debug)]
enum D600Focus {
    /// No focus point and no list: a manual-focus frame, whose AFInfo2
    /// names no point.
    ManualFocus,
    /// The points a viewfinder frame names, placed from the table: the
    /// frame they are measured in, the primary's number, the area mode,
    /// and every point's number and centre, in number order, each with the
    /// box of 164 by 164, from a table and with nothing said of focus.
    Placed((u32, u32), u16, FocusAreaMode, &'static [(u16, (u32, u32))]),
    /// The position a live-view frame records: the centre of its area in
    /// the 6016 by 4016 frame and whether the camera confirmed focus there.
    LiveView((u32, u32), bool),
}

use D600Focus::{LiveView, ManualFocus, Placed};

/// One frame of the D600 session, as the reader gives it: its number in
/// the session; the orientation IFD0 records; the image area that the crop
/// setting of the MakerNote's CropHiSpeed names; and its focus.
type D600Frame = (u8, Orientation, Option<ImageArea>, D600Focus);

/// A D600 frame shot in landscape orientation: Orientation 1.
const D600_LANDSCAPE: Orientation = Orientation::NoTransforms;

/// A D600 frame shot with the camera turned, its picture stored as the
/// sensor saw it: Orientation 8.
const D600_PORTRAIT: Orientation = Orientation::Rotate270;

/// CropHiSpeed's crop setting 0, the crop off, which names no image area,
/// since FX and DX bodies both write it.
const D600_FX: Option<ImageArea> = None;

/// CropHiSpeed's crop setting 2, the DX crop.
const D600_DX: Option<ImageArea> = Some(ImageArea::ApsC);

/// The frame of the points placed on an FX frame: the D600's full-size
/// output, the table's frame.
const D600_FULL: (u32, u32) = (6016, 4016);

/// The frame of the points placed on a DX frame: the crop's window, the
/// table's frame less twice CropHiSpeed's origin, (1040, 696), on each
/// side.
const D600_WINDOW: (u32, u32) = (3936, 2624);

/// The 56 frames of the D600 session, in order. The point numbers, the
/// layout code, the area mode, the mask's bits, FocusMode and CropHiSpeed
/// were read from each file's bytes with a script written from the notes,
/// and each point's centre is the record's, from the table of the NEF
/// notes (`docs/formats/nef/autofocus-points.md`, section 2.4), less
/// CropHiSpeed's origin on a DX frame: none of it is read from the
/// reader.
const D600_SESSION: [D600Frame; 56] = [
    // Frames 1 to 39: single-point AF through the viewfinder, one frame
    // per point, in landscape and FX. AFInfo2 names the point by its
    // number, with layout code 3, one bit of the mask set, the point's,
    // and a zero frame and area: the point, placed from the table.
    (
        1,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 13, FocusAreaMode::Single, &[(13, (2768, 1503))]),
    ),
    (
        2,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 3, FocusAreaMode::Single, &[(3, (3008, 1503))]),
    ),
    (
        3,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 8, FocusAreaMode::Single, &[(8, (3247, 1503))]),
    ),
    (
        4,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 38, FocusAreaMode::Single, &[(38, (1812, 1705))]),
    ),
    (
        5,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 35, FocusAreaMode::Single, &[(35, (2050, 1705))]),
    ),
    (
        6,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 32, FocusAreaMode::Single, &[(32, (2289, 1705))]),
    ),
    (
        7,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 29, FocusAreaMode::Single, &[(29, (2529, 1705))]),
    ),
    (
        8,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 12, FocusAreaMode::Single, &[(12, (2768, 1755))]),
    ),
    (
        9,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 2, FocusAreaMode::Single, &[(2, (3008, 1755))]),
    ),
    (
        10,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 7, FocusAreaMode::Single, &[(7, (3247, 1755))]),
    ),
    (
        11,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 17, FocusAreaMode::Single, &[(17, (3486, 1705))]),
    ),
    (
        12,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 20, FocusAreaMode::Single, &[(20, (3724, 1705))]),
    ),
    (
        13,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 23, FocusAreaMode::Single, &[(23, (3963, 1705))]),
    ),
    (
        14,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 26, FocusAreaMode::Single, &[(26, (4203, 1705))]),
    ),
    (
        15,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 37, FocusAreaMode::Single, &[(37, (1812, 2007))]),
    ),
    (
        16,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 34, FocusAreaMode::Single, &[(34, (2050, 2007))]),
    ),
    (
        17,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 31, FocusAreaMode::Single, &[(31, (2289, 2007))]),
    ),
    (
        18,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 28, FocusAreaMode::Single, &[(28, (2529, 2007))]),
    ),
    (
        19,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 11, FocusAreaMode::Single, &[(11, (2768, 2007))]),
    ),
    (
        20,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 1, FocusAreaMode::Single, &[(1, (3008, 2007))]),
    ),
    (
        21,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 6, FocusAreaMode::Single, &[(6, (3247, 2007))]),
    ),
    (
        22,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 16, FocusAreaMode::Single, &[(16, (3486, 2007))]),
    ),
    (
        23,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 19, FocusAreaMode::Single, &[(19, (3724, 2007))]),
    ),
    (
        24,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 22, FocusAreaMode::Single, &[(22, (3963, 2007))]),
    ),
    (
        25,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 25, FocusAreaMode::Single, &[(25, (4203, 2007))]),
    ),
    (
        26,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 39, FocusAreaMode::Single, &[(39, (1812, 2308))]),
    ),
    (
        27,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 36, FocusAreaMode::Single, &[(36, (2050, 2308))]),
    ),
    (
        28,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 33, FocusAreaMode::Single, &[(33, (2289, 2308))]),
    ),
    (
        29,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 30, FocusAreaMode::Single, &[(30, (2529, 2308))]),
    ),
    (
        30,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 14, FocusAreaMode::Single, &[(14, (2768, 2258))]),
    ),
    (
        31,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 4, FocusAreaMode::Single, &[(4, (3008, 2258))]),
    ),
    (
        32,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 9, FocusAreaMode::Single, &[(9, (3247, 2258))]),
    ),
    (
        33,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 18, FocusAreaMode::Single, &[(18, (3486, 2308))]),
    ),
    (
        34,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 21, FocusAreaMode::Single, &[(21, (3724, 2308))]),
    ),
    (
        35,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 24, FocusAreaMode::Single, &[(24, (3963, 2308))]),
    ),
    (
        36,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 27, FocusAreaMode::Single, &[(27, (4203, 2308))]),
    ),
    (
        37,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 15, FocusAreaMode::Single, &[(15, (2768, 2510))]),
    ),
    (
        38,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 5, FocusAreaMode::Single, &[(5, (3008, 2510))]),
    ),
    (
        39,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 10, FocusAreaMode::Single, &[(10, (3247, 2510))]),
    ),
    // Frames 40 and 41: points 1 and 13 shot again, the same as the
    // single-point frames.
    (
        40,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 1, FocusAreaMode::Single, &[(1, (3008, 2007))]),
    ),
    (
        41,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 13, FocusAreaMode::Single, &[(13, (2768, 1503))]),
    ),
    // Frames 42 to 44: auto-area AF, area mode 8, whose AFInfo2 names a
    // primary point by its number and marks the points used in its mask,
    // 12, 13 and 9 of them, the primary among them: every point the mask
    // names, placed.
    (
        42,
        D600_LANDSCAPE,
        D600_FX,
        Placed(
            D600_FULL,
            22,
            FocusAreaMode::Auto,
            &[
                (6, (3247, 2007)),
                (7, (3247, 1755)),
                (17, (3486, 1705)),
                (19, (3724, 2007)),
                (20, (3724, 1705)),
                (22, (3963, 2007)),
                (23, (3963, 1705)),
                (25, (4203, 2007)),
                (29, (2529, 1705)),
                (32, (2289, 1705)),
                (35, (2050, 1705)),
                (38, (1812, 1705)),
            ],
        ),
    ),
    (
        43,
        D600_LANDSCAPE,
        D600_FX,
        Placed(
            D600_FULL,
            22,
            FocusAreaMode::Auto,
            &[
                (1, (3008, 2007)),
                (2, (3008, 1755)),
                (12, (2768, 1755)),
                (17, (3486, 1705)),
                (19, (3724, 2007)),
                (20, (3724, 1705)),
                (22, (3963, 2007)),
                (23, (3963, 1705)),
                (29, (2529, 1705)),
                (32, (2289, 1705)),
                (35, (2050, 1705)),
                (37, (1812, 2007)),
                (38, (1812, 1705)),
            ],
        ),
    ),
    (
        44,
        D600_LANDSCAPE,
        D600_FX,
        Placed(
            D600_FULL,
            4,
            FocusAreaMode::Auto,
            &[
                (4, (3008, 2258)),
                (9, (3247, 2258)),
                (14, (2768, 2258)),
                (18, (3486, 2308)),
                (21, (3724, 2308)),
                (24, (3963, 2308)),
                (27, (4203, 2308)),
                (30, (2529, 2308)),
                (39, (1812, 2308)),
            ],
        ),
    ),
    // Frames 45 and 46: manual focus. FocusMode reads `MANUAL`, and
    // AFInfo2 names no point, layout code 0 and number 0, and records a
    // zero frame: no point.
    (45, D600_LANDSCAPE, D600_FX, ManualFocus),
    (46, D600_LANDSCAPE, D600_FX, ManualFocus),
    // Frames 47 and 48: points 13 and 38 with the camera turned, which
    // IFD0 records as Orientation 8; AFInfo2 holds the same bytes as in
    // landscape, and the point is placed in the stored orientation.
    (
        47,
        D600_PORTRAIT,
        D600_FX,
        Placed(D600_FULL, 13, FocusAreaMode::Single, &[(13, (2768, 1503))]),
    ),
    (
        48,
        D600_PORTRAIT,
        D600_FX,
        Placed(D600_FULL, 38, FocusAreaMode::Single, &[(38, (1812, 1705))]),
    ),
    // Frames 49 and 50: points 38 and 27 in the DX crop, which
    // CropHiSpeed records as `[2, 6080, 4028, 3968, 2636, 1040, 696]`:
    // placed in the crop's window, 3936 by 2624, each centre moved back by
    // the origin, (1040, 696).
    (
        49,
        D600_LANDSCAPE,
        D600_DX,
        Placed(D600_WINDOW, 38, FocusAreaMode::Single, &[(38, (772, 1009))]),
    ),
    (
        50,
        D600_LANDSCAPE,
        D600_DX,
        Placed(
            D600_WINDOW,
            27,
            FocusAreaMode::Single,
            &[(27, (3163, 1612))],
        ),
    ),
    // Frames 51 and 52: dynamic-area AF over 21 points, in AF-C, area mode
    // 5, whose AFInfo2 names the point chosen by its number and sets its
    // bit; 51 in the DX crop.
    (
        51,
        D600_LANDSCAPE,
        D600_DX,
        Placed(D600_WINDOW, 1, FocusAreaMode::Dynamic, &[(1, (1968, 1311))]),
    ),
    (
        52,
        D600_LANDSCAPE,
        D600_FX,
        Placed(D600_FULL, 1, FocusAreaMode::Dynamic, &[(1, (3008, 2007))]),
    ),
    // Frames 53 to 56: live view, the focus area moved to the four limits
    // of its travel: bottom left, top left, top right and bottom right.
    // AFInfo2 records the area as a position, which the reader gives as
    // recorded: a 6016 by 4016 frame, an area of 540 by 540 and these
    // centres, with the in-focus byte 1.
    (53, D600_LANDSCAPE, D600_FX, LiveView((343, 3673), true)),
    (54, D600_LANDSCAPE, D600_FX, LiveView((343, 343), true)),
    (55, D600_LANDSCAPE, D600_FX, LiveView((5671, 343), true)),
    (56, D600_LANDSCAPE, D600_FX, LiveView((5671, 3673), true)),
];

#[test]
fn every_frame_of_the_d600_session_gives_what_its_bytes_record() {
    // The table lists every frame once, in order, and the folder holds
    // those frames and no other.
    let numbers: Vec<u8> = D600_SESSION.iter().map(|row| row.0).collect();
    assert_eq!(numbers, (1..=56).collect::<Vec<u8>>());
    let dir = support::fixtures::fixture("nikon-d600");
    let mut committed: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".nef"))
        .map(|name| format!("nikon-d600/{name}"))
        .collect();
    committed.sort();
    let listed: Vec<String> = numbers
        .iter()
        .map(|&frame| support::fixtures::d600_frame(frame))
        .collect();
    assert_eq!(committed, listed);

    for &(frame, orientation, image_area, focus) in &D600_SESSION {
        let name = support::fixtures::d600_frame(frame);
        let label = name.as_str();
        let file = RawFile::open(support::fixtures::fixture(label))
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(file.format(), Format::Nef, "{label}");
        let metadata = file.metadata();
        assert_eq!(metadata.model(), Some("NIKON D600"), "{label}");
        assert_eq!(metadata.orientation(), orientation, "{label}");
        assert_eq!(
            metadata.camera_settings().image_area(),
            image_area,
            "{label}"
        );
        // The D600's layout, from its table, by the Model, whatever the
        // shot: through the viewfinder, in live view, in manual focus, in
        // portrait orientation or in the DX crop.
        let layout = metadata
            .focus_layout()
            .unwrap_or_else(|| panic!("{label}: no layout"));
        assert_eq!(layout.model(), "NIKON D600", "{label}");
        assert_eq!(layout.frame(), as_size((6016, 4016)), "{label}");
        assert_eq!(layout.points().len(), 39, "{label}");
        check_d600_focus(label, metadata, focus);
    }
}

/// Checks that the focus point and the focus points `metadata` gives are
/// `focus`, for the D600 frame `label`.
fn check_d600_focus(label: &str, metadata: &Metadata, focus: D600Focus) {
    match focus {
        ManualFocus => {
            assert_eq!(metadata.focus(), None, "{label}");
            assert_eq!(metadata.focus_points(), None, "{label}");
        }
        Placed(frame, primary_number, mode, points) => {
            // The primary, as a focus point: the table's centre, moved on a
            // DX frame, with its box, from a table, and nothing said of
            // focus, though the in-focus byte reads 0.
            let &(_, (x, y)) = points
                .iter()
                .find(|(number, _)| *number == primary_number)
                .unwrap_or_else(|| panic!("{label}: the primary is not among the points"));
            let focus = metadata
                .focus()
                .unwrap_or_else(|| panic!("{label}: no focus point"));
            assert_eq!(focus.frame(), as_size(frame), "{label}");
            assert_eq!(focus.center(), Point { x, y }, "{label}");
            assert_eq!(focus.size(), Some(as_size((164, 164))), "{label}");
            assert_eq!(focus.in_focus(), None, "{label}");
            assert_eq!(focus.origin(), FocusOrigin::Table, "{label}");
            // Every point the mask names, and the primary, in number order,
            // each numbered, with its box and nothing said of focus.
            let list = metadata
                .focus_points()
                .unwrap_or_else(|| panic!("{label}: no focus points"));
            assert_eq!(list.frame(), as_size(frame), "{label}");
            assert_eq!(list.origin(), FocusOrigin::Table, "{label}");
            assert_eq!(list.area_mode(), Some(mode), "{label}");
            let placed: Vec<(u16, (u32, u32))> = list
                .points()
                .iter()
                .map(|point| {
                    let number = point
                        .number()
                        .unwrap_or_else(|| panic!("{label}: a point without a number"));
                    assert_eq!(point.size(), Some(as_size((164, 164))), "{label}");
                    assert_eq!(point.in_focus(), None, "{label}");
                    (number, (point.center().x, point.center().y))
                })
                .collect();
            assert_eq!(placed, points, "{label}");
            let primary = list
                .primary()
                .unwrap_or_else(|| panic!("{label}: no primary"));
            assert_eq!(primary.number(), Some(primary_number), "{label}");
        }
        LiveView((x, y), in_focus) => {
            let focus = metadata
                .focus()
                .unwrap_or_else(|| panic!("{label}: no focus point"));
            assert_eq!(focus.frame(), as_size((6016, 4016)), "{label}");
            assert_eq!(focus.center(), Point { x, y }, "{label}");
            assert_eq!(focus.size(), Some(as_size((540, 540))), "{label}");
            assert_eq!(focus.in_focus(), Some(in_focus), "{label}");
            assert_eq!(focus.origin(), FocusOrigin::Recorded, "{label}");
            // The focus points: that point alone, the primary, with no
            // number. AFInfo2 records detection method 1, live view, with
            // area mode 4, a code the reader names under no detection
            // method: no area mode.
            let points = metadata
                .focus_points()
                .unwrap_or_else(|| panic!("{label}: no focus points"));
            assert_eq!(points.frame(), focus.frame(), "{label}");
            assert_eq!(points.origin(), FocusOrigin::Recorded, "{label}");
            assert_eq!(points.area_mode(), None, "{label}");
            assert_eq!(points.points().len(), 1, "{label}");
            let primary = points
                .primary()
                .unwrap_or_else(|| panic!("{label}: no primary"));
            assert_eq!(Some(primary), points.points().first(), "{label}");
            assert_eq!(primary.number(), None, "{label}");
            assert_eq!(primary.center(), focus.center(), "{label}");
            assert_eq!(primary.size(), focus.size(), "{label}");
            assert_eq!(primary.in_focus(), focus.in_focus(), "{label}");
        }
    }
}

/// Checks that frame 4 of the D600 session, shot through the viewfinder
/// with the single point 38, B1 in the record, the top of the leftmost
/// column, opens with its three JPEG previews, smallest first, each a stub
/// JPEG of 122 bytes that reads whole, and gives every value of its
/// metadata: point 38 placed from the table among them, and point 38 of
/// the layout, far from the centre, where a wrong table, a wrong bit order
/// or a wrong frame shows. Its values were read from the file's bytes and
/// the record in the NEF notes, not from the reader.
#[test]
fn the_d600_frame_4_gives_its_previews_metadata_and_point_38_placed_from_the_table() {
    let name = support::fixtures::d600_frame(4);
    let label = name.as_str();
    let mut file = RawFile::open(support::fixtures::fixture(label)).unwrap();

    // The MakerNote's preview, SubIFD2's and SubIFD0's, each replaced by a
    // stub of 122 bytes with the original's frame header. IFD0's thumbnail
    // is no JPEG but 160 by 120 uncompressed pixels, which the stripping
    // replaced by 64 zero bytes, and is not a preview.
    assert_eq!(
        sizes(file.previews()),
        [(570, 375), (1632, 1080), (6016, 4016)],
        "{label}"
    );
    let previews = file.previews().to_vec();
    for preview in &previews {
        let size = (preview.width(), preview.height());
        assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label} {size:?}");
        assert_eq!(
            preview.orientation(),
            Orientation::NoTransforms,
            "{label} {size:?}"
        );
        assert_eq!(preview.byte_len(), 122, "{label} {size:?}");
        let bytes = file.read_preview(preview).unwrap();
        assert_eq!(bytes.len(), 122, "{label} {size:?}");
        assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF], "{label} {size:?}");
        assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9], "{label} {size:?}");
    }
    assert_eq!(file.largest_preview(), previews.last().copied(), "{label}");

    let metadata = file.metadata();
    assert_eq!(metadata.make(), Some("NIKON CORPORATION"), "{label}");
    // The firmware the D600 ran, `Ver.1.04 ` with its space trimmed.
    assert_eq!(metadata.software(), Some("Ver.1.04"), "{label}");
    // The D600 writes its serial number only in the MakerNote's
    // SerialNumber, which the stripping removed.
    assert_eq!(metadata.serial_number(), None, "{label}");
    // DateTimeOriginal and SubSecTimeOriginal, with the offset from
    // WorldTime, -180 minutes with daylight saving time off, since the
    // D600 writes no OffsetTimeOriginal.
    assert_eq!(
        metadata
            .capture_time()
            .map(|time| time.to_string())
            .as_deref(),
        Some("2026-10-26T13:15:08.10-03:00"),
        "{label}"
    );
    // The exposure as stored: ISO 2200, 1/40 s at f/4 and 35 mm, 35 mm in
    // 35 mm terms on the full-frame body, with no compensation.
    let exposure = metadata.exposure();
    assert_eq!(exposure.iso(), Some(2200), "{label}");
    assert_eq!(exposure.exposure_time(), Some(fraction(10, 400)), "{label}");
    assert_eq!(exposure.f_number(), Some(fraction(40, 10)), "{label}");
    assert_eq!(exposure.focal_length(), Some(fraction(350, 10)), "{label}");
    assert_eq!(exposure.focal_length_in_35mm_film(), Some(35), "{label}");
    assert_eq!(
        exposure.exposure_bias(),
        Some(SignedRational {
            numerator: 0,
            denominator: 6,
        }),
        "{label}"
    );
    // The D600's Exif IFD names no lens, so the lens is the MakerNote's
    // range alone, that of a 35 mm f/1.8 prime.
    let lens = metadata
        .lens()
        .unwrap_or_else(|| panic!("{label}: no lens"));
    assert_eq!(
        (lens.make(), lens.model(), lens.serial_number()),
        (None, None, None),
        "{label}"
    );
    assert_eq!(lens.min_focal_length(), Some(fraction(350, 10)), "{label}");
    assert_eq!(lens.max_focal_length(), Some(fraction(350, 10)), "{label}");
    assert_eq!(
        lens.f_number_at_min_focal_length(),
        Some(fraction(18, 10)),
        "{label}"
    );
    assert_eq!(
        lens.f_number_at_max_focal_length(),
        Some(fraction(18, 10)),
        "{label}"
    );
    // The raw SubIFD's size; with no default crop and no CropArea, the
    // image size is the raw size.
    assert_eq!(metadata.raw_size(), Some(as_size((6080, 4028))), "{label}");
    assert_eq!(
        metadata.image_size(),
        Some(as_size((6080, 4028))),
        "{label}"
    );
    // AFInfo2 names point 38 in layout 3, with bit 37 of its mask set,
    // area mode 0 under detection method 0, single point, and a zero frame
    // and area. The point is placed from the table: at (1812, 1705), with a
    // box of 164 by 164, in the 6016 by 4016 frame, as the record in the
    // NEF notes gives it, from a table, and with nothing said of focus,
    // though the in-focus byte reads 0. The list is that point alone,
    // numbered 38, the primary.
    let frame = as_size((6016, 4016));
    let point_38 = Point { x: 1812, y: 1705 };
    let placed = Focus::new(frame, point_38)
        .and_then(|focus| focus.with_size(as_size((164, 164))))
        .map(|focus| focus.with_origin(FocusOrigin::Table));
    assert_eq!(metadata.focus(), placed, "{label}");
    let points = FocusPoints::new(frame, FocusOrigin::Table)
        .and_then(|points| {
            points.with_point(
                FocusPoint::new(point_38)
                    .with_number(38)
                    .with_size(as_size((164, 164))),
            )
        })
        .and_then(|points| points.with_primary_index(0))
        .map(|points| points.with_area_mode(FocusAreaMode::Single));
    assert_eq!(metadata.focus_points(), points.as_ref(), "{label}");
    // Point 38 of the D600's layout, from its table, at the same place.
    let layout = metadata
        .focus_layout()
        .unwrap_or_else(|| panic!("{label}: no layout"));
    let layout_38 = layout
        .point(38)
        .unwrap_or_else(|| panic!("{label}: no point 38"));
    assert_eq!(layout_38.center(), point_38, "{label}");
    assert_eq!(layout_38.size(), as_size((164, 164)), "{label}");
    // The settings. From the Exif IFD: aperture priority, matrix metering,
    // auto white balance, and a Flash of 16, the flash off by the
    // camera's setting and not fired. From the MakerNote: the fine-tuning
    // 0x000B records, no shift, whole steps; and, since the Exif IFD
    // records no ColorSpace, the MakerNote's colour space, sRGB. The
    // stripping removed the WhiteBalance text, so the mode is Exif's; the
    // D600 writes no ColorTemperatureAuto, ShutterMode or ImageSizeRAW,
    // and CropHiSpeed's crop setting, 0, names no image area.
    let zero = SignedRational {
        numerator: 0,
        denominator: 1,
    };
    assert_eq!(
        metadata.camera_settings(),
        &CameraSettings::default()
            .with_white_balance(
                WhiteBalance::default()
                    .with_mode(WhiteBalanceMode::Auto)
                    .with_fine_tune(FineTune::new(zero, zero)),
            )
            .with_exposure_program(ExposureProgram::AperturePriority)
            .with_metering_mode(MeteringMode::MultiSegment)
            .with_flash(Flash::new(false).with_mode(FlashMode::Off))
            .with_color_space(ColorSpace::Srgb),
        "{label}"
    );
}

/// Checks that frame 4 of the D600 session gives the layout of the D600's
/// points when its MakerNote is missing, as after an editor drops it on a
/// re-save, and when it cannot be read, its inner TIFF header zeroed: the
/// layout depends on the Model alone. Neither file gives a focus point,
/// which only the MakerNote records.
#[test]
fn the_d600_frame_4_gives_the_layout_without_a_readable_makernote() {
    let name = support::fixtures::d600_frame(4);
    let label = name.as_str();
    let bytes = support::fixtures::read_fixture(label);
    // The one place each pattern starts in the file.
    let only = |pattern: &[u8]| {
        let mut starts = bytes
            .windows(pattern.len())
            .enumerate()
            .filter(|(_, window)| *window == pattern)
            .map(|(start, _)| start);
        let start = starts.next().expect("the pattern is in the file");
        assert_eq!(
            starts.next(),
            None,
            "{label}: the pattern is in the file twice"
        );
        start
    };
    // The MakerNote's block: Nikon's signature, the version 2.10, then the
    // inner TIFF header, big-endian, as the D600 writes it.
    let block = only(b"Nikon\0\x02\x10\0\0MM\0\x2A");
    // The MakerNote's entry in the Exif IFD: tag 0x927C, of type UNDEFINED,
    // big-endian.
    let entry = only(&[0x92, 0x7C, 0x00, 0x07]);
    let mut unreadable = bytes.clone();
    unreadable[block + 10..block + 14].fill(0);
    let mut missing = bytes;
    // Another tag, which no reader reads, still in tag order.
    missing[entry + 1] = 0x7D;
    for (what, bytes) in [("unreadable", unreadable), ("missing", missing)] {
        let file = RawFile::from_bytes(&bytes).unwrap();
        let metadata = file.metadata();
        assert_eq!(metadata.model(), Some("NIKON D600"), "{label}, {what}");
        assert_eq!(metadata.focus(), None, "{label}, {what}");
        assert_eq!(metadata.focus_points(), None, "{label}, {what}");
        let layout = metadata
            .focus_layout()
            .unwrap_or_else(|| panic!("{label}, {what}: no layout"));
        assert_eq!(layout.model(), "NIKON D600", "{label}, {what}");
        assert_eq!(layout.frame(), as_size((6016, 4016)), "{label}, {what}");
        assert_eq!(layout.points().len(), 39, "{label}, {what}");
        let point_38 = layout
            .point(38)
            .unwrap_or_else(|| panic!("{label}, {what}: no point 38"));
        assert_eq!(
            point_38.center(),
            Point { x: 1812, y: 1705 },
            "{label}, {what}"
        );
        assert_eq!(point_38.size(), as_size((164, 164)), "{label}, {what}");
    }
}

// Real files, downloaded: 35 NEF files of raw.pixls.us, all CC0, the
// three of the Z 9, one per compression mode, and 32 files of 25 other
// Nikon bodies. They are listed with their hashes in `tests/samples.tsv`;
// `scripts/fetch-samples.sh` downloads them. They are too large to commit,
// so a test whose file is absent says so and passes: the suite runs
// offline.

/// The Z 9 samples, each with its four previews, smallest first: the IFD0
/// thumbnail, the MakerNote's preview, the medium JPEG in SubIFD2 and the
/// full-size JPEG in SubIFD0. None is padded after its end-of-image marker.
const SAMPLES: &[Sample] = &[
    // Z 9, 14-bit lossless compression (raw.pixls.us 5146)
    Sample {
        file: "nikon-z9-lossless.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 222_540,
                len: 9_354,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 99_432,
                len: 122_857,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 332_288,
                len: 1_075_885,
                after_eoi: 0,
            },
            Jpeg {
                width: 8256,
                height: 5504,
                offset: 1_408_512,
                len: 6_492_228,
                after_eoi: 0,
            },
        ],
    },
    // Z 9, High Efficiency* (raw.pixls.us 5147)
    Sample {
        file: "nikon-z9-he-star.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 229_076,
                len: 9_823,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 99_372,
                len: 129_452,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 338_944,
                len: 927_453,
                after_eoi: 0,
            },
            Jpeg {
                width: 8256,
                height: 5504,
                offset: 1_266_688,
                len: 5_760_370,
                after_eoi: 0,
            },
        ],
    },
    // Z 9, High Efficiency (raw.pixls.us 5148)
    Sample {
        file: "nikon-z9-he.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 219_300,
                len: 10_165,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 99_372,
                len: 119_678,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 329_728,
                len: 1_081_068,
                after_eoi: 0,
            },
            Jpeg {
                width: 8256,
                height: 5504,
                offset: 1_411_072,
                len: 6_288_717,
                after_eoi: 0,
            },
        ],
    },
];

#[test]
fn every_z9_sample_opens_with_its_four_previews() {
    for sample in SAMPLES {
        check(sample, Format::Nef);
    }
}

/// The samples of 25 other bodies, from the D1H of 2001 to the Z5II of
/// 2025, each with the previews the reader finds in it, smallest first.
/// They are the MakerNote's JPEG; the largest JPEG, in SubIFD0, on every
/// file but the D1H's, the D1X's, the D100's and the E5700's; a JPEG 1080
/// pixels high in SubIFD2 on the D750, D810, D850, D780, D5600, Z 6, Z f,
/// Z6III, Z5II, Z50II, D5, Z 30 and Z fc; and a 160 by 120 JPEG in IFD0 on
/// the Z f, the Z6III, the Z5II, the Z50II and the P7800. The tables were
/// made from a walk of each file's IFDs and MakerNote, by the reader's
/// rules. The one JPEG of the D1H and of the D1X is in a MakerNote of the
/// headerless layout, whose offsets count from the start of the file. One
/// file gives fewer JPEGs than it holds: the P7800's MakerNote JPEG starts
/// where the MakerNote's declared length ends, outside it, so it gives
/// two. None is padded after its end-of-image marker. Only the Z 30's
/// IFD0 asks for a rotation, of 270 degrees clockwise (Orientation 8),
/// which each of its previews reports.
const OTHER_SAMPLES: &[Sample] = &[
    // D1H, 12-bit uncompressed (raw.pixls.us 5364)
    Sample {
        file: "nikon-d1h-12bit-uncompressed.nef",
        orientation: Orientation::NoTransforms,
        previews: &[Jpeg {
            width: 570,
            height: 375,
            offset: 3_688,
            len: 24_567,
            after_eoi: 0,
        }],
    },
    // D1X, 12-bit lossy (raw.pixls.us 1785)
    Sample {
        file: "nikon-d1x-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[Jpeg {
            width: 570,
            height: 375,
            offset: 4_878,
            len: 27_540,
            after_eoi: 0,
        }],
    },
    // D1X, 12-bit uncompressed (raw.pixls.us 1786)
    Sample {
        file: "nikon-d1x-12bit-uncompressed.nef",
        orientation: Orientation::NoTransforms,
        previews: &[Jpeg {
            width: 570,
            height: 375,
            offset: 3_688,
            len: 26_984,
            after_eoi: 0,
        }],
    },
    // D100, 12-bit uncompressed (raw.pixls.us 2996)
    Sample {
        file: "nikon-d100-12bit-uncompressed.nef",
        orientation: Orientation::NoTransforms,
        previews: &[Jpeg {
            width: 570,
            height: 375,
            offset: 6_260,
            len: 107_689,
            after_eoi: 0,
        }],
    },
    // D70, 12-bit lossy (raw.pixls.us 2060)
    Sample {
        file: "nikon-d70-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 564,
                height: 372,
                offset: 8_112,
                len: 26_618,
                after_eoi: 0,
            },
            Jpeg {
                width: 3008,
                height: 2000,
                offset: 131_072,
                len: 703_881,
                after_eoi: 0,
            },
        ],
    },
    // D200, 12-bit lossy (raw.pixls.us 8079)
    Sample {
        file: "nikon-d200-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 570,
                height: 375,
                offset: 9_096,
                len: 112_612,
                after_eoi: 0,
            },
            Jpeg {
                width: 3872,
                height: 2592,
                offset: 179_712,
                len: 1_107_857,
                after_eoi: 0,
            },
        ],
    },
    // D300, 12-bit lossy (raw.pixls.us 928)
    Sample {
        file: "nikon-d300-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 570,
                height: 375,
                offset: 10_496,
                len: 88_176,
                after_eoi: 0,
            },
            Jpeg {
                width: 4288,
                height: 2848,
                offset: 156_672,
                len: 543_086,
                after_eoi: 0,
            },
        ],
    },
    // D700, 12-bit lossy (raw.pixls.us 2255)
    Sample {
        file: "nikon-d700-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 570,
                height: 375,
                offset: 12_064,
                len: 105_573,
                after_eoi: 0,
            },
            Jpeg {
                width: 4256,
                height: 2832,
                offset: 175_616,
                len: 1_747_382,
                after_eoi: 0,
            },
        ],
    },
    // D7000, 12-bit lossy (raw.pixls.us 961)
    Sample {
        file: "nikon-d7000-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 570,
                height: 375,
                offset: 17_336,
                len: 106_160,
                after_eoi: 0,
            },
            Jpeg {
                width: 4928,
                height: 3264,
                offset: 181_504,
                len: 1_175_901,
                after_eoi: 0,
            },
        ],
    },
    // D750, 12-bit lossy (raw.pixls.us 895)
    Sample {
        file: "nikon-d750-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 45_636,
                len: 115_449,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 219_648,
                len: 751_668,
                after_eoi: 0,
            },
            Jpeg {
                width: 6016,
                height: 4016,
                offset: 971_776,
                len: 981_911,
                after_eoi: 0,
            },
        ],
    },
    // D810, 12-bit small raw (raw.pixls.us 2036)
    Sample {
        file: "nikon-d810-12bit-small.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 44_964,
                len: 161_928,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 265_216,
                len: 914_381,
                after_eoi: 0,
            },
            Jpeg {
                width: 3680,
                height: 2456,
                offset: 1_179_648,
                len: 717_240,
                after_eoi: 0,
            },
        ],
    },
    // D850, 12-bit lossless small raw (raw.pixls.us 1835)
    Sample {
        file: "nikon-d850-12bit-lossless-small.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 54_768,
                len: 148_517,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 261_632,
                len: 1_108_506,
                after_eoi: 0,
            },
            Jpeg {
                width: 4128,
                height: 2752,
                offset: 1_370_624,
                len: 1_325_032,
                after_eoi: 0,
            },
        ],
    },
    // D780, 12-bit lossy (raw.pixls.us 3827)
    Sample {
        file: "nikon-d780-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 74_200,
                len: 141_716,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 274_944,
                len: 904_449,
                after_eoi: 0,
            },
            Jpeg {
                width: 6048,
                height: 4024,
                offset: 1_179_648,
                len: 3_347_446,
                after_eoi: 0,
            },
        ],
    },
    // Z 6, 12-bit small raw (raw.pixls.us 3587)
    Sample {
        file: "nikon-z6-12bit-small.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 76_256,
                len: 137_675,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 1_428_992,
                len: 737_043,
                after_eoi: 0,
            },
            Jpeg {
                width: 3024,
                height: 2016,
                offset: 2_166_272,
                len: 432_488,
                after_eoi: 0,
            },
        ],
    },
    // Z f, 14-bit (raw.pixls.us 6887)
    Sample {
        file: "nikon-zf-14bit.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 257_992,
                len: 11_483,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 104_444,
                len: 153_528,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 370_176,
                len: 1_078_641,
                after_eoi: 0,
            },
            Jpeg {
                width: 6048,
                height: 4032,
                offset: 1_448_960,
                len: 2_825_901,
                after_eoi: 0,
            },
        ],
    },
    // Z6III, 14-bit, 3:2 (raw.pixls.us 7810)
    Sample {
        file: "nikon-z6iii-14bit-3x2.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 230_632,
                len: 10_676,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 424,
                offset: 104_680,
                len: 125_933,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 367_616,
                len: 803_433,
                after_eoi: 0,
            },
            Jpeg {
                width: 3984,
                height: 2656,
                offset: 1_171_456,
                len: 1_340_854,
                after_eoi: 0,
            },
        ],
    },
    // Z6III, 14-bit, 16:9 (raw.pixls.us 7809)
    Sample {
        file: "nikon-z6iii-14bit-16x9.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 213_984,
                len: 13_103,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 360,
                offset: 104_680,
                len: 109_283,
                after_eoi: 0,
            },
            Jpeg {
                width: 1920,
                height: 1080,
                offset: 312_320,
                len: 989_322,
                after_eoi: 0,
            },
            Jpeg {
                width: 6048,
                height: 3400,
                offset: 1_302_016,
                len: 2_546_957,
                after_eoi: 0,
            },
        ],
    },
    // Z6III, 14-bit, 1:1 (raw.pixls.us 7808)
    Sample {
        file: "nikon-z6iii-14bit-1x1.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 210_680,
                len: 12_538,
                after_eoi: 0,
            },
            Jpeg {
                width: 480,
                height: 480,
                offset: 104_680,
                len: 105_980,
                after_eoi: 0,
            },
            Jpeg {
                width: 1080,
                height: 1080,
                offset: 307_200,
                len: 553_902,
                after_eoi: 0,
            },
            Jpeg {
                width: 4032,
                height: 4032,
                offset: 861_184,
                len: 2_027_564,
                after_eoi: 0,
            },
        ],
    },
    // Coolpix A, 14-bit lossy (raw.pixls.us 2238)
    Sample {
        file: "nikon-coolpix-a-14bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 570,
                height: 375,
                offset: 18_092,
                len: 105_404,
                after_eoi: 0,
            },
            Jpeg {
                width: 4928,
                height: 3264,
                offset: 181_504,
                len: 1_772_400,
                after_eoi: 0,
            },
        ],
    },
    // Coolpix P7800, 12-bit, an NRW file (raw.pixls.us 1425)
    Sample {
        file: "nikon-coolpix-p7800-12bit.nrw",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 30_830,
                len: 4_178,
                after_eoi: 0,
            },
            Jpeg {
                width: 4000,
                height: 3000,
                offset: 24_420_544,
                len: 2_571_557,
                after_eoi: 0,
            },
        ],
    },
    // E5700, 12-bit uncompressed (raw.pixls.us 4456)
    Sample {
        file: "nikon-e5700-12bit-uncompressed.nef",
        orientation: Orientation::NoTransforms,
        previews: &[Jpeg {
            width: 640,
            height: 480,
            offset: 14_409,
            len: 38_758,
            after_eoi: 0,
        }],
    },
    // Nikon 1 J1, 12-bit lossy (raw.pixls.us 2956)
    Sample {
        file: "nikon-1-j1-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 1616,
                height: 1080,
                offset: 23_612,
                len: 788_523,
                after_eoi: 0,
            },
            Jpeg {
                width: 3872,
                height: 2592,
                offset: 870_144,
                len: 633_831,
                after_eoi: 0,
            },
        ],
    },
    // D5600, 12-bit lossy, live view (raw.pixls.us 1415)
    Sample {
        file: "nikon-d5600-12bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 39_528,
                len: 128_239,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 226_304,
                len: 841_313,
                after_eoi: 0,
            },
            Jpeg {
                width: 6000,
                height: 4000,
                offset: 1_068_032,
                len: 2_582_334,
                after_eoi: 0,
            },
        ],
    },
    // Z5II, 14-bit, 1:1 (raw.pixls.us 7735)
    Sample {
        file: "nikon-z5ii-14bit-1x1.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 231_396,
                len: 7_778,
                after_eoi: 0,
            },
            Jpeg {
                width: 480,
                height: 480,
                offset: 105_152,
                len: 126_225,
                after_eoi: 0,
            },
            Jpeg {
                width: 1080,
                height: 1080,
                offset: 323_072,
                len: 642_001,
                after_eoi: 0,
            },
            Jpeg {
                width: 4032,
                height: 4032,
                offset: 965_120,
                len: 1_777_421,
                after_eoi: 0,
            },
        ],
    },
    // Z5II, 14-bit, 16:9 (raw.pixls.us 7736)
    Sample {
        file: "nikon-z5ii-14bit-16x9.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 230_180,
                len: 7_805,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 360,
                offset: 105_152,
                len: 125_010,
                after_eoi: 0,
            },
            Jpeg {
                width: 1920,
                height: 1080,
                offset: 323_072,
                len: 1_095_945,
                after_eoi: 0,
            },
            Jpeg {
                width: 6048,
                height: 3400,
                offset: 1_419_264,
                len: 2_127_142,
                after_eoi: 0,
            },
        ],
    },
    // Z50II, 14-bit, 1:1 (raw.pixls.us 7767)
    Sample {
        file: "nikon-z50ii-14bit-1x1.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 204_092,
                len: 12_678,
                after_eoi: 0,
            },
            Jpeg {
                width: 480,
                height: 480,
                offset: 105_092,
                len: 98_979,
                after_eoi: 0,
            },
            Jpeg {
                width: 1080,
                height: 1080,
                offset: 301_056,
                len: 730_293,
                after_eoi: 0,
            },
            Jpeg {
                width: 3712,
                height: 3712,
                offset: 1_031_680,
                len: 2_199_584,
                after_eoi: 0,
            },
        ],
    },
    // Z50II, 14-bit, 16:9 (raw.pixls.us 7770)
    Sample {
        file: "nikon-z50ii-14bit-16x9.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 212_828,
                len: 10_731,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 360,
                offset: 105_092,
                len: 107_717,
                after_eoi: 0,
            },
            Jpeg {
                width: 1920,
                height: 1080,
                offset: 308_736,
                len: 1_192_916,
                after_eoi: 0,
            },
            Jpeg {
                width: 5568,
                height: 3128,
                offset: 1_501_696,
                len: 2_259_600,
                after_eoi: 0,
            },
        ],
    },
    // Z6III, 14-bit High Efficiency*, 1:1 (raw.pixls.us 7812)
    Sample {
        file: "nikon-z6iii-14bit-1x1-he-star.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 210_748,
                len: 12_595,
                after_eoi: 0,
            },
            Jpeg {
                width: 480,
                height: 480,
                offset: 104_680,
                len: 106_050,
                after_eoi: 0,
            },
            Jpeg {
                width: 1080,
                height: 1080,
                offset: 307_712,
                len: 551_924,
                after_eoi: 0,
            },
            Jpeg {
                width: 4032,
                height: 4032,
                offset: 859_648,
                len: 2_049_047,
                after_eoi: 0,
            },
        ],
    },
    // Z6III, 14-bit High Efficiency*, 16:9 (raw.pixls.us 7813)
    Sample {
        file: "nikon-z6iii-14bit-16x9-he-star.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 160,
                height: 120,
                offset: 214_952,
                len: 13_115,
                after_eoi: 0,
            },
            Jpeg {
                width: 640,
                height: 360,
                offset: 104_680,
                len: 110_253,
                after_eoi: 0,
            },
            Jpeg {
                width: 1920,
                height: 1080,
                offset: 313_344,
                len: 992_802,
                after_eoi: 0,
            },
            Jpeg {
                width: 6048,
                height: 3400,
                offset: 1_306_624,
                len: 2_572_041,
                after_eoi: 0,
            },
        ],
    },
    // D5, 14-bit lossy, live view (raw.pixls.us 2218)
    Sample {
        file: "nikon-d5-14bit-lossy.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 51_968,
                len: 164_946,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 275_456,
                len: 979_610,
                after_eoi: 0,
            },
            Jpeg {
                width: 5568,
                height: 3712,
                offset: 1_255_424,
                len: 2_623_669,
                after_eoi: 0,
            },
        ],
    },
    // Z 30, 12-bit lossless, rotated (raw.pixls.us 5812)
    Sample {
        file: "nikon-z30-12bit-lossless.nef",
        orientation: Orientation::Rotate270,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 95_044,
                len: 103_247,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 257_024,
                len: 714_194,
                after_eoi: 0,
            },
            Jpeg {
                width: 5568,
                height: 3712,
                offset: 971_264,
                len: 1_350_653,
                after_eoi: 0,
            },
        ],
    },
    // Z fc, 12-bit lossless, manual focus (raw.pixls.us 8559)
    Sample {
        file: "nikon-zfc-12bit-lossless.nef",
        orientation: Orientation::NoTransforms,
        previews: &[
            Jpeg {
                width: 640,
                height: 424,
                offset: 96_684,
                len: 114_159,
                after_eoi: 0,
            },
            Jpeg {
                width: 1620,
                height: 1080,
                offset: 269_824,
                len: 1_078_447,
                after_eoi: 0,
            },
            Jpeg {
                width: 5568,
                height: 3712,
                offset: 1_348_608,
                len: 2_827_393,
                after_eoi: 0,
            },
        ],
    },
];

#[test]
fn every_sample_of_the_other_nikon_bodies_opens_with_its_previews() {
    for sample in OTHER_SAMPLES {
        check(sample, Format::Nef);
    }
}

/// The metadata each NEF sample must give, in the order of
/// `tests/samples.tsv`, its serial numbers by their shape alone (see
/// [`SampleMetadata`]). The table was generated from a walk of each file's
/// IFDs, Exif IFD and MakerNote, by the reader's rules, which read the
/// standard tags first and the MakerNote only for what they lack
/// (ADR011): on the bodies before the D780 and the Z 6, the serial number,
/// the lens's range, the ISO or the offset from UTC can come from the
/// MakerNote, and the ISO and lens range of the D1H and the D1X come from
/// one of the headerless layout. The focus point is the one the rules of
/// ADR012 give from each file's AFInfo2, FocusMode, CropHiSpeed and
/// CropArea, on the cropped picture on a crop mode, and in the stored
/// orientation on the Z 30's rotated shot; where there is none, a comment
/// says why. The software and the focal length in 35 mm terms were
/// generated from each file's IFD0 and Exif IFD by the reader's rules too:
/// every file records its firmware in Software, and all but the D1H's, the
/// D1X's and the E5700's a FocalLengthIn35mmFilm. So were the camera
/// settings, from each file's Exif IFD and MakerNote, by the rules of
/// ADR011. The white balance mode is the MakerNote's text on every file,
/// the D1H's, the D1X's and the E5700's among them, which record no Exif
/// WhiteBalance. No file records an Exif ColorSpace, so the colour space
/// is the MakerNote's, which the bodies before the D200 and the Coolpix
/// E5700 and P7800 do not record. The fine-tuning is 0x003F's, else that
/// of a 0x000B of two values, so the bodies whose 0x000B holds one value,
/// and the E5700, which writes neither, give none. The D850 and the Z fc
/// record a colour temperature of 0 under daylight, which gives none. The
/// D1H and the D1X record no Flash; the Z5II and the Z6III record the
/// ExposureProgram 0, "not defined", which gives no exposure program; and
/// the D850 records the MeteringMode 255, "other". No Nikon file gives the
/// electronic front curtain setting, which only Sony records.
const METADATA: &[SampleMetadata] = &[
    // Z 9, 14-bit lossless compression (raw.pixls.us 5146)
    SampleMetadata {
        file: "nikon-z9-lossless.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 9"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2022-02-05T13:45:03.04-05:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(64),
        exposure_time: Some((10, 3200)),
        f_number: Some((900, 100)),
        focal_length: Some((180, 10)),
        focal_length_in_35mm_film: Some(18),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 14-30mm f/4 S"),
            serial_number: true,
            min_focal_length: Some((140, 10)),
            max_focal_length: Some((300, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((8280, 5520)),
        image_size: Some((8256, 5504)),
        focus: Some(SampleFocus {
            frame: (8256, 5504),
            center: (4905, 2461),
            size: Some((291, 323)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: Some(FocusAreaMode::Single), // code 193
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4650),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, None)),
            shutter: Some(Shutter::Electronic),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z 9, High Efficiency* (raw.pixls.us 5147)
    SampleMetadata {
        file: "nikon-z9-he-star.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 9"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2022-02-05T13:46:41.85-05:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(64),
        exposure_time: Some((10, 3200)),
        f_number: Some((900, 100)),
        focal_length: Some((180, 10)),
        focal_length_in_35mm_film: Some(18),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 14-30mm f/4 S"),
            serial_number: true,
            min_focal_length: Some((140, 10)),
            max_focal_length: Some((300, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((8280, 5520)),
        image_size: Some((8256, 5504)),
        focus: Some(SampleFocus {
            frame: (8256, 5504),
            center: (4905, 2461),
            size: Some((291, 323)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: Some(FocusAreaMode::Single), // code 193
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4630),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, None)),
            shutter: Some(Shutter::Electronic),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z 9, High Efficiency (raw.pixls.us 5148)
    SampleMetadata {
        file: "nikon-z9-he.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 9"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2022-02-05T13:48:11.45-05:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(64),
        exposure_time: Some((10, 3200)),
        f_number: Some((900, 100)),
        focal_length: Some((180, 10)),
        focal_length_in_35mm_film: Some(18),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 14-30mm f/4 S"),
            serial_number: true,
            min_focal_length: Some((140, 10)),
            max_focal_length: Some((300, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((8280, 5520)),
        image_size: Some((8256, 5504)),
        focus: Some(SampleFocus {
            frame: (8256, 5504),
            center: (4905, 2461),
            size: Some((291, 323)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: Some(FocusAreaMode::Single), // code 193
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4630),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, None)),
            shutter: Some(Shutter::Electronic),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D1H, 12-bit uncompressed (raw.pixls.us 5364)
    SampleMetadata {
        file: "nikon-d1h-12bit-uncompressed.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D1H"),
        software: Some("Ver.1.00"),
        serial_number: None,
        capture_time: Some("2022-03-24T10:09:27.01"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 8000)),
        f_number: Some((45, 10)),
        focal_length: Some((600, 10)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((4, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((45, 10)),
        }),
        raw_size: Some((2012, 1324)),
        image_size: Some((2012, 1324)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Daylight),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: None,
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // D1X, 12-bit lossy (raw.pixls.us 1785)
    SampleMetadata {
        file: "nikon-d1x-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D1X"),
        software: Some("Ver.1.01"),
        serial_number: None,
        capture_time: Some("2004-07-30T12:23:24.51"),
        orientation: Orientation::NoTransforms,
        iso: Some(125),
        exposure_time: Some((10, 1800)),
        f_number: Some((110, 10)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((120, 10)),
            max_focal_length: Some((240, 10)),
            f_number_at_min_focal_length: Some((45, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((4028, 1324)),
        image_size: Some((4028, 1324)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: None,
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // D1X, 12-bit uncompressed (raw.pixls.us 1786)
    SampleMetadata {
        file: "nikon-d1x-12bit-uncompressed.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D1X"),
        software: Some("Ver.1.01"),
        serial_number: None,
        capture_time: Some("2006-07-22T20:31:52.61"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 1250)),
        f_number: Some((80, 10)),
        focal_length: Some((700, 10)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((45, 10)),
        }),
        raw_size: Some((4028, 1324)),
        image_size: Some((4028, 1324)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Flash),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: None,
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // D100, 12-bit uncompressed (raw.pixls.us 2996)
    SampleMetadata {
        file: "nikon-d100-12bit-uncompressed.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D100"),
        software: Some("Ver.2.00"),
        serial_number: None,
        capture_time: Some("2013-08-02T19:12:06.20"),
        orientation: Orientation::NoTransforms,
        iso: Some(400),
        exposure_time: Some((10, 1800)),
        f_number: Some((45, 10)),
        focal_length: Some((900, 10)),
        focal_length_in_35mm_film: Some(135),
        exposure_bias: Some((-6, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((700, 10)),
            max_focal_length: Some((3000, 10)),
            f_number_at_min_focal_length: Some((40, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((3034, 2024)),
        image_size: Some((3034, 2024)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Shade),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::Spot),
            flash: Some((true, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // D70, 12-bit lossy (raw.pixls.us 2060)
    SampleMetadata {
        file: "nikon-d70-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D70"),
        software: Some("Ver.2.00"),
        serial_number: None,
        capture_time: Some("2017-09-02T16:55:01.80"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 2000)),
        f_number: Some((32, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((3040, 2014)),
        image_size: Some((3040, 2014)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // D200, 12-bit lossy (raw.pixls.us 8079)
    SampleMetadata {
        file: "nikon-d200-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D200"),
        software: Some("Ver.2.00"),
        serial_number: None,
        capture_time: Some("2007-10-13T10:27:30.16"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 2000)),
        f_number: Some((80, 10)),
        focal_length: Some((700, 10)),
        focal_length_in_35mm_film: Some(105),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((3904, 2616)),
        image_size: Some((3904, 2616)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // D300, 12-bit lossy (raw.pixls.us 928)
    SampleMetadata {
        file: "nikon-d300-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D300"),
        software: Some("Ver.1.10"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2017-01-17T21:27:12.34+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 100)),
        f_number: Some((71, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((45, 10)),
        }),
        raw_size: Some((4352, 2868)),
        image_size: Some((4352, 2868)),
        focus: None, // FocusMode MANUAL, and a zero frame
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D700, 12-bit lossy (raw.pixls.us 2255)
    SampleMetadata {
        file: "nikon-d700-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D700"),
        software: Some("Ver.1.03"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2018-01-17T12:31:01.00+01:00"),
        orientation: Orientation::NoTransforms,
        iso: None,
        exposure_time: Some((10, 20)),
        f_number: Some((80, 10)),
        focal_length: Some((200, 10)),
        focal_length_in_35mm_film: Some(20),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((200, 10)),
            max_focal_length: Some((200, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((4288, 2844)),
        image_size: Some((4288, 2844)),
        focus: None, // a viewfinder shot that names a point: no table for this body
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::ShutterPriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // D7000, 12-bit lossy (raw.pixls.us 961)
    SampleMetadata {
        file: "nikon-d7000-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D7000"),
        software: Some("Ver.1.05"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2017-01-22T14:19:23.70-07:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 2000)),
        f_number: Some((18, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((4992, 3280)),
        image_size: Some((4992, 3280)),
        focus: Some(SampleFocus {
            frame: (4928, 3264),
            center: (2816, 2336),
            size: Some((408, 336)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 1: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D750, 12-bit lossy (raw.pixls.us 895)
    SampleMetadata {
        file: "nikon-d750-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D750"),
        software: Some("Ver.1.10"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2017-01-15T21:38:43.42+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 300)),
        f_number: Some((18, 10)),
        focal_length: Some((350, 10)),
        focal_length_in_35mm_film: Some(35),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((350, 10)),
            max_focal_length: Some((350, 10)),
            f_number_at_min_focal_length: Some((18, 10)),
            f_number_at_max_focal_length: Some((18, 10)),
        }),
        raw_size: Some((6032, 4032)),
        image_size: Some((6032, 4032)),
        focus: None, // a viewfinder shot that names a point: no table for this body
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((true, Some(FlashMode::Fill))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D810, 12-bit small raw (raw.pixls.us 2036)
    SampleMetadata {
        file: "nikon-d810-12bit-small.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D810"),
        software: Some("Ver.1.12"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2017-12-05T23:50:41.81+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 600)),
        f_number: Some((80, 10)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(50),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((500, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((14, 10)),
            f_number_at_max_focal_length: Some((14, 10)),
        }),
        raw_size: Some((3680, 2456)),
        image_size: Some((3680, 2456)),
        focus: None, // a viewfinder shot that names a point: no table for this body
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((true, Some(FlashMode::Fill))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Small),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D850, 12-bit lossless small raw (raw.pixls.us 1835)
    SampleMetadata {
        file: "nikon-d850-12bit-lossless-small.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D850"),
        software: Some("Ver.1.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2017-10-15T17:35:49.60+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(64),
        exposure_time: Some((10, 1600)),
        f_number: Some((90, 10)),
        focal_length: Some((350, 10)),
        focal_length_in_35mm_film: Some(35),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((350, 10)),
            max_focal_length: Some((350, 10)),
            f_number_at_min_focal_length: Some((14, 10)),
            f_number_at_max_focal_length: Some((14, 10)),
        }),
        raw_size: Some((6216, 4136)),
        image_size: Some((6192, 4128)),
        focus: None, // FocusMode MANUAL, and a zero frame
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Daylight),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::Other),
            flash: Some((false, None)),
            shutter: Some(Shutter::Mechanical),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Small),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D780, 12-bit lossy (raw.pixls.us 3827)
    SampleMetadata {
        file: "nikon-d780-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D780"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2020-03-28T14:14:56.69+00:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 4000)),
        f_number: Some((400, 100)),
        focal_length: Some((700, 10)),
        focal_length_in_35mm_film: Some(70),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: Some("VR 24-120mm f/4G"),
            serial_number: false,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((1200, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((6064, 4040)),
        image_size: Some((6048, 4024)),
        focus: None, // a viewfinder shot that names a point: no table for this body
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4420),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::Mechanical),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z 6, 12-bit small raw (raw.pixls.us 3587)
    SampleMetadata {
        file: "nikon-z6-12bit-small.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 6"),
        software: Some("Ver.02.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2019-11-29T17:15:20.24+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 10)),
        f_number: Some((670, 100)),
        focal_length: Some((520, 10)),
        focal_length_in_35mm_film: Some(52),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-70mm f/4 S"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((4548, 3030)),
        image_size: Some((4528, 3018)),
        focus: Some(SampleFocus {
            frame: (6048, 4024),
            center: (3024, 1160),
            size: Some((285, 308)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: Some(FocusAreaMode::Single), // code 193
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(3050),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Small),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z f, 14-bit (raw.pixls.us 6887)
    SampleMetadata {
        file: "nikon-zf-14bit.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z f"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2023-10-18T17:19:29.68-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(250),
        exposure_time: Some((10, 600)),
        f_number: Some((630, 100)),
        focal_length: Some((1500, 10)),
        focal_length_in_35mm_film: Some(150),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: Some("TAMRON 35-150mm F/2-2.8 Di III VXD A058Z"),
            serial_number: true,
            min_focal_length: Some((350, 10)),
            max_focal_length: Some((1500, 10)),
            f_number_at_min_focal_length: Some((200, 100)),
            f_number_at_max_focal_length: Some((280, 100)),
        }),
        raw_size: Some((6064, 4040)),
        image_size: Some((6048, 4032)),
        focus: Some(SampleFocus {
            frame: (6048, 4032),
            center: (3023, 2016),
            size: Some((1859, 1452)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 196: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(5750),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::Mechanical),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z6III, 14-bit, 3:2 (raw.pixls.us 7810)
    SampleMetadata {
        file: "nikon-z6iii-14bit-3x2.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-06-29T15:38:47.93-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 600)),
        f_number: Some((800, 100)),
        focal_length: Some((860, 10)),
        focal_length_in_35mm_film: Some(129),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-200mm f/4-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((4000, 2672)),
        image_size: Some((3984, 2656)),
        focus: Some(SampleFocus {
            frame: (3984, 2656),
            center: (1991, 1328),
            size: Some((146, 161)),
            in_focus: Some(false),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 192: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4910),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::ApsC),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z6III, 14-bit, 16:9 (raw.pixls.us 7809)
    SampleMetadata {
        file: "nikon-z6iii-14bit-16x9.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-06-29T15:39:01.13-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 1000)),
        f_number: Some((900, 100)),
        focal_length: Some((860, 10)),
        focal_length_in_35mm_film: Some(86),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-200mm f/4-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((6064, 3416)),
        image_size: Some((6048, 3400)),
        focus: Some(SampleFocus {
            frame: (6048, 3400),
            center: (3023, 1700),
            size: Some((163, 174)),
            in_focus: Some(false),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 192: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4830),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::Wide),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z6III, 14-bit, 1:1 (raw.pixls.us 7808)
    SampleMetadata {
        file: "nikon-z6iii-14bit-1x1.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-06-29T15:38:53.42-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 1000)),
        f_number: Some((900, 100)),
        focal_length: Some((860, 10)),
        focal_length_in_35mm_film: Some(111),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-200mm f/4-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((4048, 4040)),
        image_size: Some((4032, 4032)),
        focus: Some(SampleFocus {
            frame: (4032, 4032),
            center: (2015, 2016),
            size: Some((163, 174)),
            in_focus: Some(false),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 192: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4870),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::Square),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Coolpix A, 14-bit lossy (raw.pixls.us 2238)
    SampleMetadata {
        file: "nikon-coolpix-a-14bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("COOLPIX A"),
        software: Some("Ver.1.12"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2015-12-23T11:41:13.50+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 6400)),
        f_number: Some((40, 10)),
        focal_length: Some((185, 10)),
        focal_length_in_35mm_film: Some(28),
        exposure_bias: Some((0, 6)),
        lens: None,
        raw_size: Some((4992, 3280)),
        image_size: Some((4992, 3280)),
        focus: Some(SampleFocus {
            frame: (4928, 3264),
            center: (2464, 1632),
            size: Some((408, 336)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 1: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // Coolpix P7800, 12-bit, an NRW file (raw.pixls.us 1425)
    SampleMetadata {
        file: "nikon-coolpix-p7800-12bit.nrw",
        make: Some("NIKON"),
        model: Some("COOLPIX P7800"),
        software: Some("COOLPIX P7800  V1.0"),
        serial_number: None,
        capture_time: Some("2017-05-20T11:55:14"),
        orientation: Orientation::NoTransforms,
        iso: Some(80),
        exposure_time: Some((10, 400)),
        f_number: Some((20, 10)),
        focal_length: Some((60, 10)),
        focal_length_in_35mm_film: Some(28),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((4032, 3024)),
        image_size: Some((4032, 3024)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // E5700, 12-bit uncompressed (raw.pixls.us 4456)
    SampleMetadata {
        file: "nikon-e5700-12bit-uncompressed.nef",
        make: Some("NIKON"),
        model: Some("E5700"),
        software: Some("E5700v1.1"),
        serial_number: None,
        capture_time: Some("2008-01-15T11:15:41"),
        orientation: Orientation::NoTransforms,
        iso: None,
        exposure_time: Some((10, 4366)),
        f_number: Some((80, 10)),
        focal_length: Some((89, 10)),
        focal_length_in_35mm_film: None,
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((2576, 1924)),
        image_size: Some((2576, 1924)),
        focus: None, // no AFInfo2: AFInfo holds a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: None,
            exposure_program: Some(ExposureProgram::Program),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: None,
        },
    },
    // Nikon 1 J1, 12-bit lossy (raw.pixls.us 2956)
    SampleMetadata {
        file: "nikon-1-j1-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON 1 J1"),
        software: Some("Ver.1.40"),
        serial_number: Some(SerialShape {
            len: 8,
            digits: true,
        }),
        capture_time: Some("2010-01-01T00:11:05.00+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 130)),
        f_number: Some((63, 10)),
        focal_length: Some((189, 10)),
        focal_length_in_35mm_film: Some(51),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: Some("1 NIKKOR VR 10-30mm f/3.5-5.6"),
            serial_number: false,
            min_focal_length: Some((100, 10)),
            max_focal_length: Some((300, 10)),
            f_number_at_min_focal_length: Some((35, 10)),
            f_number_at_max_focal_length: Some((56, 10)),
        }),
        raw_size: Some((3904, 2606)),
        image_size: Some((3904, 2606)),
        focus: None, // version 0200: a point number only
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 1),
                green_magenta: (0, 1),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::Electronic),
            electronic_front_curtain_setting: None,
            image_area: None,
            raw_image_size: None,
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // D5600, 12-bit lossy, live view (raw.pixls.us 1415)
    SampleMetadata {
        file: "nikon-d5600-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D5600"),
        software: Some("Ver.1.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2017-05-12T18:35:56.99-05:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 2000)),
        f_number: Some((56, 10)),
        focal_length: Some((170, 10)),
        focal_length_in_35mm_film: Some(25),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((170, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((40, 10)),
        }),
        raw_size: Some((6016, 4016)),
        image_size: Some((6016, 4016)),
        focus: Some(SampleFocus {
            frame: (6000, 4000),
            center: (3368, 2000),
            size: Some((984, 816)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 2: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::ApsC),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z5II, 14-bit, 1:1 (raw.pixls.us 7735)
    SampleMetadata {
        file: "nikon-z5ii-14bit-1x1.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z5_2"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2025-04-27T17:57:35.99+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 4000)),
        f_number: Some((710, 100)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(31),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-70mm f/4 S"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((4048, 4040)),
        image_size: Some((4032, 4032)),
        focus: None, // auto-area mode: a mask of points only, zero frame
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4820),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::Mechanical),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::Square),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z5II, 14-bit, 16:9 (raw.pixls.us 7736)
    SampleMetadata {
        file: "nikon-z5ii-14bit-16x9.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z5_2"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2025-04-27T17:58:04.37+02:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 4000)),
        f_number: Some((710, 100)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(24),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-70mm f/4 S"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((400, 100)),
        }),
        raw_size: Some((6064, 3416)),
        image_size: Some((6048, 3400)),
        focus: None, // auto-area mode: a mask of points only, zero frame
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4720),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::Mechanical),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::Wide),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z50II, 14-bit, 1:1 (raw.pixls.us 7767)
    SampleMetadata {
        file: "nikon-z50ii-14bit-1x1.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z50_2"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-12-08T16:53:48.99+07:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 500)),
        f_number: Some((420, 100)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(45),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z DX 16-50mm f/3.5-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((160, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((350, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((3728, 3728)),
        image_size: Some((3712, 3712)),
        focus: Some(SampleFocus {
            frame: (3712, 3712),
            center: (1864, 1868),
            size: Some((278, 276)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: Some(FocusAreaMode::Tracking), // code 207
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(6590),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::ApsCSquare),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z50II, 14-bit, 16:9 (raw.pixls.us 7770)
    SampleMetadata {
        file: "nikon-z50ii-14bit-16x9.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z50_2"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-12-08T16:54:06.82+07:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 500)),
        f_number: Some((420, 100)),
        focal_length: Some((240, 10)),
        focal_length_in_35mm_film: Some(36),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z DX 16-50mm f/3.5-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((160, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((350, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((5600, 3144)),
        image_size: Some((5568, 3128)),
        focus: Some(SampleFocus {
            frame: (5568, 3128),
            center: (2789, 1573),
            size: Some((280, 280)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: Some(FocusAreaMode::Tracking), // code 207
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(7000),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::CenterWeighted),
            flash: Some((false, Some(FlashMode::Off))),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::ApsCWide),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z6III, 14-bit High Efficiency*, 1:1 (raw.pixls.us 7812)
    SampleMetadata {
        file: "nikon-z6iii-14bit-1x1-he-star.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-06-29T15:38:03.76-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 1000)),
        f_number: Some((900, 100)),
        focal_length: Some((860, 10)),
        focal_length_in_35mm_film: Some(111),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-200mm f/4-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((4048, 4040)),
        image_size: Some((4032, 4032)),
        focus: Some(SampleFocus {
            frame: (4032, 4032),
            center: (2015, 2016),
            size: Some((163, 174)),
            in_focus: Some(false),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 192: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4820),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::Square),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z6III, 14-bit High Efficiency*, 16:9 (raw.pixls.us 7813)
    SampleMetadata {
        file: "nikon-z6iii-14bit-16x9-he-star.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2024-06-29T15:38:14.16-04:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 1000)),
        f_number: Some((900, 100)),
        focal_length: Some((860, 10)),
        focal_length_in_35mm_film: Some(86),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z 24-200mm f/4-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((240, 10)),
            max_focal_length: Some((2000, 10)),
            f_number_at_min_focal_length: Some((400, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((6064, 3416)),
        image_size: Some((6048, 3400)),
        focus: Some(SampleFocus {
            frame: (6048, 3400),
            center: (3023, 1700),
            size: Some((163, 174)),
            in_focus: Some(false),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 192: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(4740),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: None,
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::Wide),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // D5, 14-bit lossy, live view (raw.pixls.us 2218)
    SampleMetadata {
        file: "nikon-d5-14bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D5"),
        software: Some("Ver.1.20"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2018-01-12T13:15:24.15+01:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(400),
        exposure_time: Some((10, 1250)),
        f_number: Some((56, 10)),
        focal_length: Some((140, 10)),
        focal_length_in_35mm_film: Some(14),
        exposure_bias: Some((6, 6)),
        lens: Some(SampleLens {
            make: None,
            model: None,
            serial_number: false,
            min_focal_length: Some((140, 10)),
            max_focal_length: Some((240, 10)),
            f_number_at_min_focal_length: Some((28, 10)),
            f_number_at_max_focal_length: Some((28, 10)),
        }),
        raw_size: Some((5584, 3728)),
        image_size: Some((5568, 3712)),
        focus: Some(SampleFocus {
            frame: (5568, 3712),
            center: (3077, 2427),
            size: Some((588, 500)),
            in_focus: Some(true),
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 2: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(6020),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::AperturePriority),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: None,
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::FullFrame),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
    // Z 30, 12-bit lossless, rotated (raw.pixls.us 5812)
    SampleMetadata {
        file: "nikon-z30-12bit-lossless.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 30"),
        software: Some("Ver.01.00"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2022-07-26T15:56:46.86+02:00"),
        orientation: Orientation::Rotate270,
        iso: Some(1800),
        exposure_time: Some((10, 600)),
        f_number: Some((630, 100)),
        focal_length: Some((500, 10)),
        focal_length_in_35mm_film: Some(75),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: Some("NIKON"),
            model: Some("NIKKOR Z DX 16-50mm f/3.5-6.3 VR"),
            serial_number: true,
            min_focal_length: Some((160, 10)),
            max_focal_length: Some((500, 10)),
            f_number_at_min_focal_length: Some((350, 100)),
            f_number_at_max_focal_length: Some((630, 100)),
        }),
        raw_size: Some((5600, 3728)),
        image_size: Some((5568, 3712)),
        focus: Some(SampleFocus {
            frame: (5568, 3712),
            center: (3776, 1741),
            size: Some((552, 552)),
            in_focus: None,
            origin: FocusOrigin::Recorded,
            points: 1,
            primary_number: None,
            area_mode: None, // code 199: no file shot in a known mode names it
        }),
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Auto),
            color_temperature: Some(5240),
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::ApsC),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::AdobeRgb),
        },
    },
    // Z fc, 12-bit lossless, manual focus (raw.pixls.us 8559)
    SampleMetadata {
        file: "nikon-zfc-12bit-lossless.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z fc"),
        software: Some("Ver.01.70"),
        serial_number: Some(SerialShape {
            len: 7,
            digits: true,
        }),
        capture_time: Some("2025-09-05T18:15:05.79+00:00"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 10)),
        f_number: Some((900, 100)),
        focal_length: Some((460, 10)),
        focal_length_in_35mm_film: Some(69),
        exposure_bias: Some((0, 6)),
        lens: Some(SampleLens {
            make: None,
            model: Some("18-70mm f/3.5-4.5G"),
            serial_number: false,
            min_focal_length: Some((180, 10)),
            max_focal_length: Some((700, 10)),
            f_number_at_min_focal_length: Some((350, 100)),
            f_number_at_max_focal_length: Some((450, 100)),
        }),
        raw_size: Some((5600, 3728)),
        image_size: Some((5568, 3712)),
        focus: None, // FocusMode MANUAL; AFInfo2 records a position, (2520, 1856) and 288 by 319, which the rule discards
        focus_layout: None,
        settings: SampleSettings {
            white_balance_mode: Some(WhiteBalanceMode::Daylight),
            color_temperature: None,
            fine_tune: Some(SampleFineTune {
                amber_blue: (0, 4),
                green_magenta: (0, 4),
            }),
            exposure_program: Some(ExposureProgram::Manual),
            metering_mode: Some(MeteringMode::MultiSegment),
            flash: Some((false, None)),
            shutter: Some(Shutter::ElectronicFrontCurtain),
            electronic_front_curtain_setting: None,
            image_area: Some(ImageArea::ApsC),
            raw_image_size: Some(RawImageSize::Large),
            color_space: Some(ColorSpace::Srgb),
        },
    },
];

#[test]
fn every_nikon_sample_gives_its_metadata() {
    for expected in METADATA {
        check_metadata(expected, Format::Nef);
    }
}

#[test]
fn the_sample_list_and_the_tables_of_previews_and_metadata_name_the_same_nikon_files() {
    // The list is compiled in, so this runs without any download.
    let listed = listed("nef");

    let previews: Vec<&str> = SAMPLES
        .iter()
        .chain(OTHER_SAMPLES)
        .map(|sample| sample.file)
        .collect();
    let metadata: Vec<&str> = METADATA.iter().map(|expected| expected.file).collect();
    for (what, files) in [("previews", previews), ("metadata", metadata)] {
        let expected: BTreeSet<(&str, &str)> = files.iter().map(|&file| (file, "normal")).collect();
        assert_eq!(
            expected.len(),
            files.len(),
            "a file is in the tables of {what} twice"
        );
        let unexpected: Vec<_> = listed.difference(&expected).collect();
        let unlisted: Vec<_> = expected.difference(&listed).collect();
        assert!(
            unexpected.is_empty() && unlisted.is_empty(),
            "listed without expected {what}: {unexpected:?}; \
             expected {what} but not listed: {unlisted:?}"
        );
    }
}
