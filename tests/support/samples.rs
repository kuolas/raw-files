// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the integration tests find the real sample files of
//! `tests/samples.tsv`, which files of each format the list names, and the
//! checks that one opens with exactly the previews expected of it and
//! gives exactly the metadata expected of it, its focus point among it.
//! They are too large to commit, so `scripts/fetch-samples.sh` downloads
//! them, the RAW files from raw.pixls.us and the JPEG files from Wikimedia
//! Commons, and a test whose file is absent skips with a note rather than
//! failing: the suite runs offline.

use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};

use raw_files::{
    Camera, ColorSpace, Directory, ExposureProgram, FieldType, FlashMode, FocusAreaMode,
    FocusOrigin, Format, ImageArea, Make, MeteringMode, Orientation, Preview, PreviewFormat,
    Rational, RawFile, RawImageSize, Shutter, SignedRational, Size, WhiteBalanceMode,
};

/// The path of the sample file `name`, or `None`, with a note on stderr,
/// when it has not been downloaded. The files are in `RAW_FILES_SAMPLES`
/// (relative to the current directory, which under `cargo test` is the
/// crate's root), or in `raw-samples` at the crate's root when it is unset,
/// where `scripts/fetch-samples.sh` puts them: a folder that git ignores and
/// that `cargo clean` leaves alone.
pub fn path(name: &str) -> Option<PathBuf> {
    let dir = match env::var_os("RAW_FILES_SAMPLES") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("raw-samples"),
    };
    let path = dir.join(name);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!(
            "skipped: sample file {} is missing; scripts/fetch-samples.sh downloads it",
            path.display()
        );
        None
    }
}

/// The list of sample files, `tests/samples.tsv`, compiled in, so that a
/// test of the list runs without any download.
const LIST: &str = include_str!("../samples.tsv");

/// The columns of `tests/samples.tsv`, in order: the site a file comes
/// from, `raw.pixls.us` or `commons`; its id there, a number on
/// raw.pixls.us and the file's name on Wikimedia Commons; its format, the
/// name of the file in `raw-samples/`, its tier, `normal` or `large`; its
/// SHA-256; its license, with the author for a license that asks for
/// credit; and its URL.
const COLUMNS: [&str; 8] = [
    "source", "id", "format", "file", "tier", "sha256", "license", "url",
];

/// The files `tests/samples.tsv` lists of `format`, as its format column
/// names it (`nef`, `arw` or `jpeg`), each with its tier, for the tests
/// that check that the list and the tables of a format name the same
/// files. Checks that the list starts with the header line of [`COLUMNS`],
/// that every line has as many columns, from one of the two sources, and
/// that no file name is listed twice anywhere in the list, whatever its
/// format or tier: every file is saved under its name in one folder,
/// `raw-samples/` or the one `RAW_FILES_SAMPLES` names ([`path`]), so two
/// lines with one name would be one local file.
pub fn listed(format: &str) -> BTreeSet<(&'static str, &'static str)> {
    let mut lines = LIST.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    assert_eq!(header, COLUMNS);
    let mut names = BTreeSet::new();
    let mut listed = BTreeSet::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), COLUMNS.len(), "{line}");
        assert!(
            matches!(fields[0], "raw.pixls.us" | "commons"),
            "{line}: an unknown source"
        );
        assert!(names.insert(fields[3]), "{line}: listed twice");
        if fields[2] == format {
            listed.insert((fields[3], fields[4]));
        }
    }
    listed
}

/// One JPEG a sample file holds, as the notes on its format record it and
/// the reader must report it: its size in pixels from its own frame header,
/// where its bytes are, and how many bytes the declared length runs past
/// the end-of-image marker (a preview can be padded, as some Sony files'
/// are: the thumbnails of the DSLR-A200, A700 and A900 and the 1616 by
/// 1080 previews of the SLT-A55 and NEX-5, see the ARW notes; and a JPEG
/// file's own preview, the whole file, which runs through the zeros that
/// some Sony bodies write after the last image, see the JPEG notes).
pub struct Jpeg {
    pub width: u32,
    pub height: u32,
    pub offset: u64,
    pub len: u64,
    pub after_eoi: u64,
}

/// A sample file of `tests/samples.tsv` and what opening it must give: its
/// previews, smallest first, none for a file in which the reader finds
/// none, and the orientation IFD0 gives them all.
pub struct Sample {
    pub file: &'static str,
    pub orientation: Orientation,
    pub previews: &'static [Jpeg],
}

/// The stored size of each preview, in order.
pub fn sizes(previews: &[Preview]) -> Vec<Size> {
    previews.iter().map(Preview::size).collect()
}

/// Checks that `sample` opens as `format` with exactly its expected
/// previews, smallest first: each a JPEG at its offset and of its length,
/// with IFD0's orientation, that reads whole as stored, its padding after
/// the end-of-image marker included; and that the largest is the last, or
/// that there is none when the file has no preview. Does nothing when the
/// file has not been downloaded.
pub fn check(sample: &Sample, format: Format) {
    let Some(path) = path(sample.file) else {
        return;
    };
    let label = sample.file;
    let mut file = RawFile::open(&path).unwrap();
    assert_eq!(file.format(), format, "{label}");
    let expected_sizes: Vec<Size> = sample
        .previews
        .iter()
        .map(|jpeg| Size {
            width: jpeg.width,
            height: jpeg.height,
        })
        .collect();
    assert_eq!(sizes(file.previews()), expected_sizes, "{label}");

    let previews = file.previews().to_vec();
    for (preview, jpeg) in previews.iter().zip(sample.previews) {
        let size = preview.size();
        assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label} {size:?}");
        assert_eq!(
            preview.orientation(),
            sample.orientation,
            "{label} {size:?}"
        );
        assert_eq!(preview.byte_range().start, jpeg.offset, "{label} {size:?}");
        assert_eq!(preview.byte_len(), jpeg.len, "{label} {size:?}");

        // The declared length is returned as stored, so a preview padded
        // after its end-of-image marker comes back with the padding.
        let bytes = file.read_preview(preview).unwrap();
        assert_eq!(
            u64::try_from(bytes.len()).unwrap(),
            preview.byte_len(),
            "{label} {size:?}"
        );
        assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF], "{label} {size:?}");
        let eoi = usize::try_from(jpeg.len - jpeg.after_eoi - 2).unwrap();
        assert_eq!(
            bytes[eoi..eoi + 2],
            [0xFF, 0xD9],
            "{label} {size:?}: the end-of-image marker, followed by {} bytes",
            jpeg.after_eoi
        );
    }

    let place = |preview: &Preview| (preview.size(), preview.byte_range());
    assert_eq!(
        file.largest_preview().as_ref().map(place),
        previews.last().map(place),
        "{label}"
    );
}

/// What a sample file of `tests/samples.tsv` must give as its metadata:
/// every value of [`Metadata`](raw_files::Metadata), `None` where the file
/// must give none. The camera's maker is the [`Make`] the reader recognizes
/// from the recorded Make, and its model the name raw-files' table gives
/// for the maker and the recorded Model, `None` for every body the table
/// does not hold, as [`Camera::model`](raw_files::Camera::model) gives
/// them. Fractions are the numerator and denominator as stored,
/// so `(10, 3200)` is not `(1, 320)`. The software is its text, as the
/// reader cleans it, and the focal length in 35 mm terms its whole
/// millimeters. The capture time is its ISO 8601 form, as
/// [`DateTime`](raw_files::DateTime) displays it. The focus point is the
/// one the reader gives ([`SampleFocus`]), and so are the camera settings
/// ([`SampleSettings`]). The lens is `None` where the file describes none,
/// which the reader gives as a lens with every value `None`. The layout of
/// the body's autofocus points is the Model text of the layout the reader
/// must give
/// ([`FocusLayout::recorded_model`](raw_files::FocusLayout::recorded_model)),
/// `None` when it must give none, as for every body that raw-files has no
/// table of.
///
/// The files are public, but the repository holds no camera's or lens's
/// serial number, so a serial number is recorded by its shape alone and a
/// lens's by whether there is one. Nor does it hold the name or the words
/// of whoever took a public file: the texts of who made a picture and the
/// camera's owner are recorded by which of them the file gives
/// ([`Text`]), never by what they say, and the location by whether the
/// file gives one and which of its optional parts ([`LocationPart`]),
/// never by where it lies.
pub struct SampleMetadata {
    pub file: &'static str,
    pub make: Option<Make>,
    pub model: Option<&'static str>,
    pub recorded_make: Option<&'static str>,
    pub recorded_model: Option<&'static str>,
    pub software: Option<&'static str>,
    pub serial_number: Option<SerialShape>,
    pub texts: &'static [Text],
    pub location: Option<&'static [LocationPart]>,
    pub capture_time: Option<&'static str>,
    pub orientation: Orientation,
    pub iso: Option<u32>,
    pub exposure_time: Option<(u32, u32)>,
    pub f_number: Option<(u32, u32)>,
    pub focal_length: Option<(u32, u32)>,
    pub focal_length_in_35mm_film: Option<u32>,
    pub exposure_bias: Option<(i32, i32)>,
    pub lens: Option<SampleLens>,
    pub raw_size: Option<(u32, u32)>,
    pub image_size: Option<(u32, u32)>,
    pub focus: Option<SampleFocus>,
    pub focus_layout: Option<&'static str>,
    pub settings: SampleSettings,
}

/// How the camera of a sample file was set, as the reader must give it
/// ([`CameraSettings`](raw_files::CameraSettings)): every value, `None`
/// where the file must give none, each a variant of the library's own
/// enums, so that a row reads as what a caller gets. The white balance is
/// its three parts: its mode, its color temperature in kelvin, and its
/// fine-tuning ([`SampleFineTune`]). Then the exposure program, the metering
/// mode, the flash, whether it fired and then its mode, `None` when the
/// file records no mode, the shutter used, whether Sony's electronic front
/// curtain setting was on, the image area, the raw image size and the
/// color space.
///
/// Every row writes every field, `None` included, one to a line, so that
/// a value read later changes one line of a row.
pub struct SampleSettings {
    pub white_balance_mode: Option<WhiteBalanceMode>,
    pub color_temperature: Option<u32>,
    pub fine_tune: Option<SampleFineTune>,
    pub exposure_program: Option<ExposureProgram>,
    pub metering_mode: Option<MeteringMode>,
    pub flash: Option<(bool, Option<FlashMode>)>,
    pub shutter: Option<Shutter>,
    pub electronic_front_curtain_setting: Option<bool>,
    pub image_area: Option<ImageArea>,
    pub raw_image_size: Option<RawImageSize>,
    pub color_space: Option<ColorSpace>,
}

/// The white balance fine-tuning of a sample file, as the reader must give
/// it ([`FineTune`](raw_files::FineTune)): the shift along the amber to
/// blue axis, and the one along the green to magenta axis, each fraction
/// as the reader gives it, signed as its axis is named, and not reduced,
/// so `(-24, 4)` is not `(-6, 1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleFineTune {
    pub amber_blue: (i32, i32),
    pub green_magenta: (i32, i32),
}

/// Where the camera of a sample file focused, as the reader must give it
/// ([`Focus`](raw_files::Focus)): the frame the point is measured in, its
/// width and height in pixels; the center of the focus area in that frame,
/// x from its left and y from its top; the area's width and height, `None`
/// when the file records no area; and whether the camera confirmed focus,
/// `None` when the file does not say. The frame is the one the reader
/// gives, not always the one the file records: on a Nikon crop mode it is
/// the cropped picture, the size of the MakerNote's CropArea, and the
/// center and the area are placed in it.
///
/// Then what the reader gives of the autofocus points
/// ([`FocusPoints`](raw_files::FocusPoints)), whose primary the point above
/// is, by construction, so that the values above check it: where they
/// came from; how many there are, the length of the list; the primary's
/// number, `None` when the file gives it none, as for a recorded position;
/// and the area mode, `None` when the file names none in a code raw-files
/// knows.
pub struct SampleFocus {
    pub frame: (u32, u32),
    pub center: (u32, u32),
    pub size: Option<(u32, u32)>,
    pub in_focus: Option<bool>,
    pub origin: FocusOrigin,
    pub points: usize,
    pub primary_number: Option<u16>,
    pub area_mode: Option<FocusAreaMode>,
}

/// The shape of a serial number, in place of its text: how many bytes
/// long it is, and whether every one of them is an ASCII digit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerialShape {
    pub len: usize,
    pub digits: bool,
}

impl SerialShape {
    /// The shape of `serial_number`.
    pub fn of(serial_number: &str) -> Self {
        Self {
            len: serial_number.len(),
            digits: serial_number.bytes().all(|b| b.is_ascii_digit()),
        }
    }
}

/// A text of who made the picture, or the camera's owner, that a sample
/// file gives: the repository records which texts the reader gives for a
/// public file, never what they say. A row lists them in the order of
/// the variants, the order of the getters of
/// [`Authorship`](raw_files::Authorship), then
/// [`Camera::owner_name`](raw_files::Camera::owner_name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    Artist,
    Copyright,
    Photographer,
    Editor,
    Title,
    Description,
    Comment,
    OwnerName,
}

impl Text {
    /// Which texts `metadata` gives, in the order of the variants.
    pub fn given(metadata: &raw_files::Metadata) -> Vec<Self> {
        let authorship = metadata.authorship();
        [
            (Self::Artist, authorship.artist()),
            (Self::Copyright, authorship.copyright()),
            (Self::Photographer, authorship.photographer()),
            (Self::Editor, authorship.editor()),
            (Self::Title, authorship.title()),
            (Self::Description, authorship.description()),
            (Self::Comment, authorship.comment()),
            (Self::OwnerName, metadata.camera().owner_name()),
        ]
        .into_iter()
        .filter_map(|(text, value)| value.map(|_| text))
        .collect()
    }
}

/// An optional part of the location a sample file gives: the repository
/// records whether a public file has a location and which of these parts
/// it gives, never a coordinate, an altitude, a time or a datum. A row
/// lists them in the order of the variants, the order of the getters of
/// [`Location`](raw_files::Location); an empty list is a position alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationPart {
    Altitude,
    Time,
    ImageDirection,
    MapDatum,
}

impl LocationPart {
    /// Which parts `location` gives, in the order of the variants.
    pub fn given(location: &raw_files::Location) -> Vec<Self> {
        [
            (Self::Altitude, location.altitude().is_some()),
            (Self::Time, location.time().is_some()),
            (Self::ImageDirection, location.image_direction().is_some()),
            (Self::MapDatum, location.map_datum().is_some()),
        ]
        .into_iter()
        .filter_map(|(part, given)| given.then_some(part))
        .collect()
    }
}

/// Checks that the sample file `name` opens as `format`, records in IFD0's
/// ImageDescription the text `SONY DSC`, as older Sony bodies write it by
/// themselves whatever the picture, in 9 bytes or padded with spaces, and
/// gives no description for it. Does nothing when the file has not been
/// downloaded.
pub fn check_placeholder_description(name: &str, format: Format) {
    let Some(path) = path(name) else {
        return;
    };
    let mut file = RawFile::open(&path).unwrap();
    assert_eq!(file.format(), format, "{name}");
    let entry = file
        .entries()
        .entry(Directory::Ifd(0), 0x010E)
        .expect("an ImageDescription");
    let recorded = file.read_value(&entry).unwrap();
    assert_eq!(recorded.text().as_deref(), Some("SONY DSC"), "{name}");
    assert_eq!(file.metadata().authorship().description(), None, "{name}");
}

/// What the lens of a sample file must be: its recorded make and model,
/// whether it has a serial number, and its range, each fraction as stored.
pub struct SampleLens {
    pub recorded_make: Option<&'static str>,
    pub recorded_model: Option<&'static str>,
    pub serial_number: bool,
    pub min_focal_length: Option<(u32, u32)>,
    pub max_focal_length: Option<(u32, u32)>,
    pub f_number_at_min_focal_length: Option<(u32, u32)>,
    pub f_number_at_max_focal_length: Option<(u32, u32)>,
}

/// The numerator and denominator of `fraction`, as stored.
fn stored(fraction: Rational) -> (u32, u32) {
    (fraction.numerator, fraction.denominator)
}

/// The numerator and denominator of the signed `fraction`, as stored.
fn stored_signed(fraction: SignedRational) -> (i32, i32) {
    (fraction.numerator, fraction.denominator)
}

/// Checks the rules of `camera`'s text to show: it is there exactly when
/// a Model is recorded; it is the maker's name and the body's, joined by
/// a space, where raw-files has both; and it is the recorded Model
/// wherever raw-files has no name for the body.
pub fn check_display_name(camera: &Camera, label: &str) {
    let shown = camera.display_name();
    assert_eq!(
        shown.is_some(),
        camera.recorded_model().is_some(),
        "{label}: a text to show without a recorded model, or the reverse"
    );
    match (camera.make(), camera.model()) {
        (Some(make), Some(model)) => assert_eq!(
            shown,
            Some(format!("{make} {model}")),
            "{label}: the text to show of a body with a name"
        ),
        _ => assert_eq!(
            shown.as_deref(),
            camera.recorded_model(),
            "{label}: the text to show of a body with no name"
        ),
    }
}

/// Checks that the sample file `expected` names opens as `format` and
/// gives exactly the metadata `expected` records: every value compared, the
/// focus point's and the camera settings' among them, so that a value the
/// file must not give fails as surely as a wrong or a missing one. Does
/// nothing when the file has not been downloaded.
pub fn check_metadata(expected: &SampleMetadata, format: Format) {
    let Some(path) = path(expected.file) else {
        return;
    };
    let label = expected.file;
    let file = RawFile::open(&path).unwrap();
    assert_eq!(file.format(), format, "{label}");
    let metadata = file.metadata();
    assert_eq!(metadata.camera().make(), expected.make, "{label}: make");
    assert_eq!(metadata.camera().model(), expected.model, "{label}: model");
    assert_eq!(
        metadata.camera().recorded_make(),
        expected.recorded_make,
        "{label}: recorded make"
    );
    assert_eq!(
        metadata.camera().recorded_model(),
        expected.recorded_model,
        "{label}: recorded model"
    );
    check_display_name(metadata.camera(), label);
    assert_eq!(metadata.software(), expected.software, "{label}: software");
    assert_eq!(
        metadata.camera().serial_number().map(SerialShape::of),
        expected.serial_number,
        "{label}: serial number"
    );
    assert_eq!(Text::given(metadata), expected.texts, "{label}: texts");
    assert_eq!(
        metadata.location().map(LocationPart::given),
        expected.location.map(<[LocationPart]>::to_vec),
        "{label}: location"
    );
    assert_eq!(
        metadata
            .capture_time()
            .map(|time| time.to_string())
            .as_deref(),
        expected.capture_time,
        "{label}: capture time"
    );
    assert_eq!(
        metadata.orientation(),
        expected.orientation,
        "{label}: orientation"
    );

    let exposure = metadata.exposure();
    assert_eq!(exposure.iso(), expected.iso, "{label}: ISO");
    assert_eq!(
        exposure.exposure_time().map(stored),
        expected.exposure_time,
        "{label}: exposure time"
    );
    assert_eq!(
        exposure.f_number().map(stored),
        expected.f_number,
        "{label}: f-number"
    );
    assert_eq!(
        exposure.focal_length().map(stored),
        expected.focal_length,
        "{label}: focal length"
    );
    assert_eq!(
        exposure.focal_length_in_35mm_film(),
        expected.focal_length_in_35mm_film,
        "{label}: focal length in 35 mm terms"
    );
    assert_eq!(
        exposure
            .exposure_bias()
            .map(|bias| (bias.numerator, bias.denominator)),
        expected.exposure_bias,
        "{label}: exposure bias"
    );

    let lens = metadata.lens();
    let lens = (
        lens.recorded_make(),
        lens.recorded_model(),
        lens.serial_number().is_some(),
        [
            lens.min_focal_length(),
            lens.max_focal_length(),
            lens.f_number_at_min_focal_length(),
            lens.f_number_at_max_focal_length(),
        ]
        .map(|value| value.map(stored)),
    );
    let expected_lens = match &expected.lens {
        Some(lens) => (
            lens.recorded_make,
            lens.recorded_model,
            lens.serial_number,
            [
                lens.min_focal_length,
                lens.max_focal_length,
                lens.f_number_at_min_focal_length,
                lens.f_number_at_max_focal_length,
            ],
        ),
        None => (None, None, false, [None; 4]),
    };
    assert_eq!(lens, expected_lens, "{label}: lens");

    let size = |size: Size| (size.width, size.height);
    assert_eq!(
        metadata.raw_size().map(size),
        expected.raw_size,
        "{label}: raw size"
    );
    assert_eq!(
        metadata.image_size().map(size),
        expected.image_size,
        "{label}: image size"
    );

    let focus = metadata.focus().map(|focus| {
        let center = focus.center();
        (
            size(focus.frame()),
            (center.x, center.y),
            focus.size().map(size),
            focus.in_focus(),
        )
    });
    let expected_focus = expected
        .focus
        .as_ref()
        .map(|focus| (focus.frame, focus.center, focus.size, focus.in_focus));
    assert_eq!(focus, expected_focus, "{label}: focus");

    // The points: a list exactly when there is a focus. `focus` is the
    // list's primary by construction, so the row's `focus` values above
    // check the primary.
    let points = metadata.focus_points();
    assert_eq!(
        points.is_some(),
        metadata.focus().is_some(),
        "{label}: focus points beside the focus"
    );
    let points = points.map(|points| {
        (
            points.origin(),
            points.points().len(),
            points.primary().and_then(|point| point.number()),
            points.area_mode(),
        )
    });
    let expected_points = expected.focus.as_ref().map(|focus| {
        (
            focus.origin,
            focus.points,
            focus.primary_number,
            focus.area_mode,
        )
    });
    assert_eq!(points, expected_points, "{label}: focus points");
    assert_eq!(
        metadata
            .focus_layout()
            .map(|layout| layout.recorded_model()),
        expected.focus_layout,
        "{label}: focus layout"
    );

    let settings = metadata.camera_settings();
    let expected = &expected.settings;
    let white_balance = settings.white_balance();
    assert_eq!(
        white_balance.mode(),
        expected.white_balance_mode,
        "{label}: white balance mode"
    );
    assert_eq!(
        white_balance.color_temperature(),
        expected.color_temperature,
        "{label}: white balance color temperature"
    );
    assert_eq!(
        white_balance.fine_tune().map(|fine_tune| SampleFineTune {
            amber_blue: stored_signed(fine_tune.amber_blue()),
            green_magenta: stored_signed(fine_tune.green_magenta()),
        }),
        expected.fine_tune,
        "{label}: white balance fine-tuning"
    );
    assert_eq!(
        settings.exposure_program(),
        expected.exposure_program,
        "{label}: exposure program"
    );
    assert_eq!(
        settings.metering_mode(),
        expected.metering_mode,
        "{label}: metering mode"
    );
    assert_eq!(
        settings.flash().map(|flash| (flash.fired(), flash.mode())),
        expected.flash,
        "{label}: flash"
    );
    assert_eq!(settings.shutter(), expected.shutter, "{label}: shutter");
    assert_eq!(
        settings.electronic_front_curtain_setting(),
        expected.electronic_front_curtain_setting,
        "{label}: electronic front curtain setting"
    );
    assert_eq!(
        settings.image_area(),
        expected.image_area,
        "{label}: image area"
    );
    assert_eq!(
        settings.raw_image_size(),
        expected.raw_image_size,
        "{label}: raw image size"
    );
    assert_eq!(
        settings.color_space(),
        expected.color_space,
        "{label}: color space"
    );
}

/// What a sample file of `tests/samples.tsv` must give as its entries:
/// each directory, in the order the reader found it, by the name the
/// library gives it, with the number of its entries listed and the number
/// left out.
pub struct SampleEntries {
    pub file: &'static str,
    pub directories: &'static [(&'static str, usize, u32)],
}

/// Checks that `expected`'s file lists exactly its directories, in order,
/// with their counts; that every entry's value reads under the default
/// cap, the whole MakerNote's included; and that the entries agree with the
/// typed values read from them: IFD0's Make and Model are the recorded make
/// and model, the Exif IFD's DateTimeOriginal names the capture time's
/// year, and ExifVersion is four bytes of digits. The serial number is
/// checked by presence, type and count only, never by its value. Does
/// nothing when the file has not been downloaded.
pub fn check_entries(expected: &SampleEntries) {
    let Some(path) = path(expected.file) else {
        return;
    };
    let label = expected.file;
    let mut file = RawFile::open(&path).unwrap();
    let entries = file.entries().clone();
    let found: Vec<(String, usize, u32)> = entries
        .directories()
        .iter()
        .map(|&d| {
            (
                d.to_string(),
                entries.in_directory(d).len(),
                entries.skipped(d),
            )
        })
        .collect();
    let want: Vec<(String, usize, u32)> = expected
        .directories
        .iter()
        .map(|&(name, listed, skipped)| (name.to_string(), listed, skipped))
        .collect();
    assert_eq!(found, want, "{label}");

    for entry in &entries {
        let value = file.read_value(entry);
        assert!(value.is_ok(), "{label}: {entry:?}: {:?}", value.err());
    }

    let text = |file: &mut RawFile<std::fs::File>, directory, tag| -> Option<String> {
        let entry = entries.entry(directory, tag)?;
        file.read_value(&entry).unwrap().text()
    };
    let camera = file.metadata().camera().clone();
    assert_eq!(
        text(&mut file, Directory::Ifd(0), 0x010F).as_deref(),
        camera.recorded_make(),
        "{label}: Make"
    );
    assert_eq!(
        text(&mut file, Directory::Ifd(0), 0x0110).as_deref(),
        camera.recorded_model(),
        "{label}: Model"
    );
    if let Some(time) = file.metadata().capture_time() {
        let original = text(&mut file, Directory::Exif, 0x9003).expect("DateTimeOriginal");
        assert_eq!(
            original.get(..4),
            Some(&*time.year().to_string()),
            "{label}"
        );
    }
    if let Some(version) = entries.entry(Directory::Exif, 0x9000) {
        assert_eq!(version.name(), Some("ExifVersion"), "{label}");
        let value = file.read_value(&version).unwrap();
        assert_eq!(value.field_type(), FieldType::Undefined, "{label}");
        let bytes = value.bytes().unwrap();
        assert!(
            bytes.len() == 4 && bytes.iter().all(u8::is_ascii_digit),
            "{label}"
        );
    }
    if let Some(serial) = entries.entry(Directory::Exif, 0xA431) {
        assert_eq!(serial.field_type(), FieldType::Ascii, "{label}: serial");
        assert!(serial.count() > 0, "{label}: serial");
    }
}

/// Checks that the entry of `tag` in the MakerNote of the sample `name`,
/// a record its maker enciphers, is a plain UNDEFINED entry, lying inside
/// the MakerNote's bytes, whose value the raw view gives byte for byte as
/// the file stores it, neither marked nor deciphered (ADR023); returns
/// those bytes, or `None` when the sample has not been downloaded. That
/// the offset counts from the right base the unit tests check, and the
/// sample metadata tests, which read the same MakerNote IFD.
pub fn check_enciphered(name: &str, tag: u16) -> Option<Vec<u8>> {
    let path = path(name)?;
    let stored = std::fs::read(&path).unwrap();
    let mut file = RawFile::open(&path).unwrap();
    let entry = file
        .entries()
        .entry(Directory::MakerNote, tag)
        .unwrap_or_else(|| panic!("{name}: no MakerNote entry {tag:#06X}"));
    assert_eq!(entry.field_type(), FieldType::Undefined, "{name}");
    let range = entry.byte_range().expect("stored out of line");
    // Inside the MakerNote's own bytes, the Exif IFD's 0x927C entry's
    // value, which the standard walk located apart from the MakerNote's
    // reader: a base counted wrongly would most likely leave them.
    let makernote = file
        .entries()
        .entry(Directory::Exif, 0x927C)
        .and_then(|entry| entry.byte_range())
        .expect("the MakerNote's bytes");
    assert!(
        makernote.start <= range.start && range.end <= makernote.end,
        "{name}: {range:?} outside the MakerNote, {makernote:?}"
    );
    let range = usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap();
    let value = file.read_value(&entry).unwrap();
    assert_eq!(value.bytes(), stored.get(range), "{name}");
    assert_eq!(
        value.len(),
        usize::try_from(entry.count()).unwrap(),
        "{name}"
    );
    value.bytes().map(<[u8]>::to_vec)
}
