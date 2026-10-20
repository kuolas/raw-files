// SPDX-License-Identifier: MIT OR Apache-2.0

//! The NEF reader from outside the crate, on a file laid out like a Nikon
//! Z 9's: what it opens as, which previews it lists, what reading one
//! returns, and what metadata it gives, the focus point among it. Then the
//! previews and the metadata of a file laid out like a Z6III's on a 1:1
//! crop, whose focus point the reader moves from the full frame AFInfo2
//! records into the cropped picture. Then the previews and the metadata
//! of a file laid out like a D300's, an older body that records its serial
//! number and its lens's range only in its MakerNote, and no focus point,
//! shot through the viewfinder in manual focus. Then real files, three of
//! the Z 9 and 20 of 18 other bodies, when they have been downloaded (see
//! [`support::samples::path`]): the previews each gives, and its metadata,
//! every value of it.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;

use raw_files::{
    DateTime, Exposure, Focus, Format, Lens, Metadata, Orientation, Point, Preview, PreviewFormat,
    Rational, RawFile, SignedRational, Size,
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
    Jpeg, Sample, SampleLens, SampleMetadata, SerialShape, check, check_metadata, sizes,
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
/// `orientation`, gives the camera, the serial number, the capture time,
/// the exposure, the lens, the sizes and the focus point the fixture
/// records, and that orientation, from bytes and from a reader; `label`
/// names the file in a failure.
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
        // The exposure as stored: 1/320 s at f/9, 18 mm and ISO 64, with
        // no compensation.
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
    }
}

/// `(width, height)` as a [`Size`].
fn as_size((width, height): (u32, u32)) -> Size {
    Size { width, height }
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
        .with_offset(-300)?;
    let exposure = Exposure::default()
        .with_iso(64)
        .with_exposure_time(Rational::new(10, 3200)?)
        .with_f_number(Rational::new(900, 100)?)
        .with_focal_length(Rational::new(180, 10)?)
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
        .with_focus(focus);
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
    }
}

/// The metadata of the Z6III file, with the orientation `orientation`,
/// built as a caller's test would, through the public constructors and
/// the `with_…` methods alone; `None` when a constructor refuses a value.
fn the_z6iii_metadata_by_hand(orientation: Orientation) -> Option<Metadata> {
    let time = DateTime::new(2024, 6, 29, 15, 38, 53)?
        .with_subseconds(420_000_000, 2)?
        .with_offset(-240)?;
    let exposure = Exposure::default()
        .with_iso(200)
        .with_exposure_time(Rational::new(10, 1000)?)
        .with_f_number(Rational::new(900, 100)?)
        .with_focal_length(Rational::new(860, 10)?)
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
    let metadata = Metadata::default()
        .with_make("NIKON CORPORATION")
        .with_model("NIKON Z6_3")
        .with_serial_number("2345678")
        .with_capture_time(time)
        .with_orientation(orientation)
        .with_exposure(exposure)
        .with_lens(lens)
        .with_raw_size(as_size(Z6III_RAW))
        .with_image_size(as_size(Z6III_IMAGE))
        .with_focus(focus);
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
            // The exposure as stored: 1/10 s at f/7.1, 50 mm and ISO 200,
            // with no compensation. The ISO is the Exif one, which the
            // MakerNote's `[0, 200]` agrees with.
            let exposure = metadata.exposure();
            assert_eq!(exposure.iso(), Some(200), "{label}");
            assert_eq!(exposure.exposure_time(), Some(fraction(10, 100)), "{label}");
            assert_eq!(exposure.f_number(), Some(fraction(71, 10)), "{label}");
            assert_eq!(exposure.focal_length(), Some(fraction(500, 10)), "{label}");
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
        }
    }
}

// Real files. The Z 9 files of raw.pixls.us, one per compression mode, and
// 20 files of 18 other Nikon bodies, all CC0, are listed with their hashes
// in `tests/samples.tsv`; `scripts/fetch-samples.sh` downloads them. They
// are too large to commit, so a test whose file is absent says so and
// passes: the suite runs offline.

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

/// The samples of 18 other bodies, from the D1H of 2001 to the Z6III of
/// 2024, each with the previews the reader finds in it, smallest first.
/// They are the MakerNote's JPEG; the largest JPEG, in SubIFD0, on every
/// file but the D1H's, the D100's and the E5700's; a JPEG 1080 pixels
/// high in SubIFD2 on the D750, D810, D850, D780, Z 6, Z f and Z6III; and
/// a 160 by 120 JPEG in IFD0 on the Z f, the Z6III and the P7800. The
/// tables were made from a walk of each file's IFDs and MakerNote, by the
/// reader's rules. Two files give fewer JPEGs than they hold: the D1H's
/// one JPEG is in a MakerNote of the bare layout, which the reader does
/// not read, so it gives none; and the P7800's MakerNote JPEG starts where
/// the MakerNote's declared length ends, outside it, so it gives two. None
/// is padded after its end-of-image marker, and no file's IFD0 asks for a
/// rotation.
const OTHER_SAMPLES: &[Sample] = &[
    // D1H, 12-bit uncompressed (raw.pixls.us 5364)
    Sample {
        file: "nikon-d1h-12bit-uncompressed.nef",
        orientation: Orientation::NoTransforms,
        previews: &[],
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
/// MakerNote. The D1H's MakerNote is of the bare layout, which the reader
/// does not read, so that file gives the values of its standard tags
/// alone.
const METADATA: &[SampleMetadata] = &[
    // Z 9, 14-bit lossless compression (raw.pixls.us 5146)
    SampleMetadata {
        file: "nikon-z9-lossless.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 9"),
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
    },
    // Z 9, High Efficiency* (raw.pixls.us 5147)
    SampleMetadata {
        file: "nikon-z9-he-star.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 9"),
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
    },
    // Z 9, High Efficiency (raw.pixls.us 5148)
    SampleMetadata {
        file: "nikon-z9-he.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 9"),
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
    },
    // D1H, 12-bit uncompressed (raw.pixls.us 5364)
    SampleMetadata {
        file: "nikon-d1h-12bit-uncompressed.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D1H"),
        serial_number: None,
        capture_time: Some("2022-03-24T10:09:27.01"),
        orientation: Orientation::NoTransforms,
        iso: None,
        exposure_time: Some((10, 8000)),
        f_number: Some((45, 10)),
        focal_length: Some((600, 10)),
        exposure_bias: Some((4, 6)),
        lens: None,
        raw_size: Some((2012, 1324)),
        image_size: Some((2012, 1324)),
    },
    // D100, 12-bit uncompressed (raw.pixls.us 2996)
    SampleMetadata {
        file: "nikon-d100-12bit-uncompressed.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D100"),
        serial_number: None,
        capture_time: Some("2013-08-02T19:12:06.20"),
        orientation: Orientation::NoTransforms,
        iso: Some(400),
        exposure_time: Some((10, 1800)),
        f_number: Some((45, 10)),
        focal_length: Some((900, 10)),
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
    },
    // D70, 12-bit lossy (raw.pixls.us 2060)
    SampleMetadata {
        file: "nikon-d70-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D70"),
        serial_number: None,
        capture_time: Some("2017-09-02T16:55:01.80"),
        orientation: Orientation::NoTransforms,
        iso: Some(200),
        exposure_time: Some((10, 2000)),
        f_number: Some((32, 10)),
        focal_length: Some((500, 10)),
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
    },
    // D200, 12-bit lossy (raw.pixls.us 8079)
    SampleMetadata {
        file: "nikon-d200-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D200"),
        serial_number: None,
        capture_time: Some("2007-10-13T10:27:30.16"),
        orientation: Orientation::NoTransforms,
        iso: Some(100),
        exposure_time: Some((10, 2000)),
        f_number: Some((80, 10)),
        focal_length: Some((700, 10)),
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
    },
    // D300, 12-bit lossy (raw.pixls.us 928)
    SampleMetadata {
        file: "nikon-d300-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D300"),
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
    },
    // D700, 12-bit lossy (raw.pixls.us 2255)
    SampleMetadata {
        file: "nikon-d700-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D700"),
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
    },
    // D7000, 12-bit lossy (raw.pixls.us 961)
    SampleMetadata {
        file: "nikon-d7000-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D7000"),
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
    },
    // D750, 12-bit lossy (raw.pixls.us 895)
    SampleMetadata {
        file: "nikon-d750-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D750"),
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
    },
    // D810, 12-bit small raw (raw.pixls.us 2036)
    SampleMetadata {
        file: "nikon-d810-12bit-small.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D810"),
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
    },
    // D850, 12-bit lossless small raw (raw.pixls.us 1835)
    SampleMetadata {
        file: "nikon-d850-12bit-lossless-small.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D850"),
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
    },
    // D780, 12-bit lossy (raw.pixls.us 3827)
    SampleMetadata {
        file: "nikon-d780-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON D780"),
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
    },
    // Z 6, 12-bit small raw (raw.pixls.us 3587)
    SampleMetadata {
        file: "nikon-z6-12bit-small.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z 6"),
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
    },
    // Z f, 14-bit (raw.pixls.us 6887)
    SampleMetadata {
        file: "nikon-zf-14bit.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z f"),
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
    },
    // Z6III, 14-bit, 3:2 (raw.pixls.us 7810)
    SampleMetadata {
        file: "nikon-z6iii-14bit-3x2.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
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
    },
    // Z6III, 14-bit, 16:9 (raw.pixls.us 7809)
    SampleMetadata {
        file: "nikon-z6iii-14bit-16x9.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
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
    },
    // Z6III, 14-bit, 1:1 (raw.pixls.us 7808)
    SampleMetadata {
        file: "nikon-z6iii-14bit-1x1.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON Z6_3"),
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
    },
    // Coolpix A, 14-bit lossy (raw.pixls.us 2238)
    SampleMetadata {
        file: "nikon-coolpix-a-14bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("COOLPIX A"),
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
        exposure_bias: Some((0, 6)),
        lens: None,
        raw_size: Some((4992, 3280)),
        image_size: Some((4992, 3280)),
    },
    // Coolpix P7800, 12-bit, an NRW file (raw.pixls.us 1425)
    SampleMetadata {
        file: "nikon-coolpix-p7800-12bit.nrw",
        make: Some("NIKON"),
        model: Some("COOLPIX P7800"),
        serial_number: None,
        capture_time: Some("2017-05-20T11:55:14"),
        orientation: Orientation::NoTransforms,
        iso: Some(80),
        exposure_time: Some((10, 400)),
        f_number: Some((20, 10)),
        focal_length: Some((60, 10)),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((4032, 3024)),
        image_size: Some((4032, 3024)),
    },
    // E5700, 12-bit uncompressed (raw.pixls.us 4456)
    SampleMetadata {
        file: "nikon-e5700-12bit-uncompressed.nef",
        make: Some("NIKON"),
        model: Some("E5700"),
        serial_number: None,
        capture_time: Some("2008-01-15T11:15:41"),
        orientation: Orientation::NoTransforms,
        iso: None,
        exposure_time: Some((10, 4366)),
        f_number: Some((80, 10)),
        focal_length: Some((89, 10)),
        exposure_bias: Some((0, 10)),
        lens: None,
        raw_size: Some((2576, 1924)),
        image_size: Some((2576, 1924)),
    },
    // Nikon 1 J1, 12-bit lossy (raw.pixls.us 2956)
    SampleMetadata {
        file: "nikon-1-j1-12bit-lossy.nef",
        make: Some("NIKON CORPORATION"),
        model: Some("NIKON 1 J1"),
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
    // The list is in the repository, so this runs without any download.
    let list =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples.tsv")).unwrap();
    let mut lines = list.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    assert_eq!(
        header,
        ["id", "format", "file", "tier", "sha256", "licence", "url"]
    );
    let mut listed = BTreeSet::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), header.len(), "{line}");
        if fields[1] == "nef" {
            listed.insert((fields[2], fields[3]));
        }
    }

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
