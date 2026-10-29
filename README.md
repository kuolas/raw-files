# raw-files

A camera RAW library in pure Rust. It reads camera RAW files and the JPEG
files cameras write: the previews they hold, their metadata and where the
camera focused. The RAW files it reads are Nikon NEF and Sony ARW, and
fuller RAW decoding is planned.

**Status: early.** What raw-files reads today:

- **Previews.** The JPEG previews embedded in Nikon NEF and Sony ARW
  files, the SRF, SR2 and ARQ files of Sony's family and the ARW files of
  the Sony-built Hasselblad bodies included. It gives the largest, or the
  smallest that fills a given size, with the orientation to show it in,
  and it never reads the raw data.
- **Metadata.** What a viewer or a file renamer needs next to a preview:
  the capture time, with sub-seconds and offset from UTC when recorded;
  the camera's make, model and serial number; the software that wrote the
  file; the lens; the exposure, with the focal length in 35 mm terms; the
  orientation; and the sizes of the raw data and of the visible image.
- **Camera settings.** The white balance with its colour temperature and
  fine-tuning, the exposure program, the metering mode, the flash, the
  shutter, Sony's electronic front curtain setting, the image area, the
  raw image size and the colour space.
- **The focus point.** Where the camera focused: the centre of the focus
  area, the frame it is measured in and, when recorded, the area's size
  and whether the camera confirmed focus, ready to be scaled onto any
  preview, with where the point came from. Nikon mirrorless bodies, Nikon
  DSLRs in live view and Sony bodies from the ILCE-7RM2 on record its
  position. A Nikon DSLR shot through the viewfinder names only the
  number of an autofocus point, which raw-files places on the picture
  from its own table of the body's points, measured on frames shot with
  that body: the Nikon D600 is the first body with a table, and further
  bodies join as their tables are measured. Beside the focus point come
  the autofocus points the camera used, the primary among them, with the
  area mode, and, for a body with a table, the layout of all its points,
  for a viewer to draw. Camera JPEGs are read for all of it as their raw
  files are, except that, through the viewfinder, a D600 Medium JPEG,
  and a D600 DX JPEG smaller than the crop's full size, give no point.
- **JPEG files.** The JPEG files cameras write beside their raw files or
  instead of them, read in the same way: a JPEG's previews are the file
  itself and the smaller images it holds, and its metadata is the same,
  with the picture's size and no raw size.
- **Not yet.** A shot through the viewfinder of a body without a table
  gives no focus point, nor does a Nikon mirrorless shot that records
  only a mask of the points used, a shot with a Sony body before the
  ILCE-7RM2, or a shot in manual focus; raw decoding comes later. See the
  [roadmap](docs/ROADMAP.md).

## Usage

Open a file, take its largest preview and read the JPEG:

```rust,no_run
use raw_files::RawFile;

fn main() -> raw_files::Result<()> {
    let mut file = RawFile::open("photo.nef")?;
    if let Some(preview) = file.largest_preview() {
        let jpeg = file.read_preview(&preview)?;
        println!(
            "{} pixels, shown upright by {:?}, {} bytes",
            preview.size(),
            preview.orientation(),
            jpeg.len(),
        );
    }
    Ok(())
}
```

Print when the picture was taken, and with which camera and lens:

```rust,no_run
use raw_files::RawFile;

fn main() -> raw_files::Result<()> {
    let file = RawFile::open("photo.nef")?;
    let metadata = file.metadata();
    if let Some(time) = metadata.capture_time() {
        println!("taken {time}");
    }
    if let (Some(make), Some(model)) = (metadata.make(), metadata.model()) {
        println!("camera: {make} {model}");
    }
    if let Some(name) = metadata.lens().and_then(|lens| lens.model()) {
        println!("lens: {name}");
    }
    Ok(())
}
```

Find where the camera focused on the largest preview, the window in which
to measure how sharp the picture is; a file that records no focus point
gives none:

```rust,no_run
use raw_files::RawFile;

fn main() -> raw_files::Result<()> {
    let file = RawFile::open("photo.nef")?;
    // The camera measures the point in its own frame; scale it to the preview.
    let on_preview = file
        .largest_preview()
        .zip(file.metadata().focus())
        .and_then(|(preview, focus)| focus.scaled_to(preview.size()));
    match on_preview {
        Some(focus) => {
            let (centre, frame) = (focus.center(), focus.frame());
            println!("focused at {centre} on the {frame} preview");
            if let Some(size) = focus.size() {
                println!("measure sharpness in the {size} area around it");
            }
        }
        None => println!("no focus point to show"),
    }
    Ok(())
}
```

Build the same values without a camera file, as a program's tests do:

```rust
use raw_files::{Focus, Metadata, Point, Size};

fn main() {
    let frame = Size { width: 8256, height: 5504 };
    let focus = Focus::new(frame, Point { x: 4905, y: 2461 })
        .expect("the centre lies on the frame")
        .with_in_focus(true);
    let metadata = Metadata::default()
        .with_model("NIKON Z 9")
        .with_focus(focus);
    assert_eq!(metadata.focus(), Some(focus));
}
```

Say where a focus point came from, as a viewer's caption does: recorded by
the camera, or named by number and placed from a table of the body's
points, with the area mode the camera was set to:

```rust
use raw_files::{Focus, FocusAreaMode, FocusOrigin, FocusPoint, FocusPoints, Metadata, Point, Size};

/// A caption for the point the camera focused on, if the file names one.
fn caption(metadata: &Metadata) -> Option<String> {
    let points = metadata.focus_points()?;
    let primary = points.primary()?;
    let point = match primary.number() {
        Some(number) => format!("point {number} at {}", primary.center()),
        None => format!("at {}", primary.center()),
    };
    Some(match points.area_mode() {
        Some(mode) => format!("{}: {point}, {mode}", points.origin()),
        None => format!("{}: {point}", points.origin()),
    })
}

fn main() {
    // A Nikon Z 9 records the position it focused at.
    let frame = Size { width: 8256, height: 5504 };
    let recorded = Focus::new(frame, Point { x: 4905, y: 2461 })
        .expect("the centre lies on the frame");
    let z9 = Metadata::default().with_focus(recorded);
    assert_eq!(caption(&z9).as_deref(), Some("Recorded: at (4905, 2461)"));

    // A Nikon D600 shot through the viewfinder names point 38, which
    // raw-files places from its table of the D600's points.
    let frame = Size { width: 6016, height: 4016 };
    let point_38 = FocusPoint::new(Point { x: 1812, y: 1705 })
        .with_number(38)
        .with_size(Size { width: 164, height: 164 });
    let placed = FocusPoints::new(frame, FocusOrigin::Table)
        .and_then(|points| points.with_point(point_38))
        .and_then(|points| points.with_primary(0))
        .expect("the point lies on the frame")
        .with_area_mode(FocusAreaMode::Single);
    let d600 = Metadata::default().with_focus_points(placed);
    assert_eq!(
        caption(&d600).as_deref(),
        Some("Placed from a table: point 38 at (1812, 1705), Single point"),
    );
}
```

Show how the camera was set, as a viewer's information panel does; every
setting is optional, and one the file does not record is left out:

```rust
use raw_files::{CameraSettings, ExposureProgram, MeteringMode, WhiteBalance, WhiteBalanceMode};

/// The lines an information panel shows for the settings a file records.
fn panel(settings: &CameraSettings) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(program) = settings.exposure_program() {
        lines.push(format!("Exposure program: {program}"));
    }
    if let Some(mode) = settings.metering_mode() {
        lines.push(format!("Metering: {mode}"));
    }
    let white_balance = settings.white_balance();
    if let Some(mode) = white_balance.mode() {
        lines.push(match white_balance.color_temperature() {
            Some(temperature) => format!("White balance: {mode}, {temperature} K"),
            None => format!("White balance: {mode}"),
        });
    }
    lines
}

fn main() {
    // A file's settings: panel(file.metadata().camera_settings())
    let settings = CameraSettings::default()
        .with_exposure_program(ExposureProgram::AperturePriority)
        .with_metering_mode(MeteringMode::MultiSegment)
        .with_white_balance(
            WhiteBalance::default()
                .with_mode(WhiteBalanceMode::Auto)
                .with_color_temperature(5_200),
        );
    for line in panel(&settings) {
        println!("{line}");
    }
    assert_eq!(
        panel(&settings),
        [
            "Exposure program: Aperture priority",
            "Metering: Multi-segment",
            "White balance: Auto, 5200 K",
        ]
    );
}
```

Tell a raw file from the JPEG of the same shot by their names, before
opening either, as a photo browser that shows the two as one picture does:

```rust
use std::path::Path;

use raw_files::Format;

/// The format a file's name says it holds, if raw-files reads it.
fn format_of(name: &str) -> Option<Format> {
    let extension = Path::new(name).extension()?.to_str()?;
    Format::from_extension(extension)
}

fn main() {
    for name in ["DSC_0001.NEF", "DSC_0001.JPG", "notes.txt"] {
        let kind = match format_of(name) {
            Some(format) if format.is_raw() => "a raw file",
            Some(_) => "a picture with no raw data",
            None => "not a file raw-files reads",
        };
        println!("{name}: {kind}");
    }
    assert_eq!(format_of("DSC_0001.NEF").map(Format::is_raw), Some(true));
    assert_eq!(format_of("DSC_0001.JPG").map(Format::is_raw), Some(false));
    assert_eq!(format_of("notes.txt"), None);
}
```

`RawFile` opens a raw file or a JPEG file from a path, bytes in memory or
any reader that can seek, and its `format` says which format the file is.
`Format::is_raw` tells the raw formats from the others, and
`Format::from_extension` gives the format a file's name suggests, for a file
picker or a folder scan; opening never looks at the name. A `Preview` says
where the bytes are, how large the picture is, its format and the
orientation to show it in. `preview_at_least` picks the smallest preview
that fills a box. `Metadata` holds what the camera recorded about the
picture, read when the file is opened; every value is optional, since not
every camera records every one, and a damaged value is `None` on its own.
`Metadata::camera_settings` says how the camera was set, as a
`CameraSettings`. `Focus` is where the camera focused, and its `scaled_to`
places it on a preview; `Focus::origin` says, as a `FocusOrigin`, whether
the camera recorded the position or raw-files placed it from a table of
the body's points. `Metadata::focus_points` gives the autofocus points the
camera used, a `FocusPoints` of `FocusPoint`s with the primary among them
and the `FocusAreaMode`, and `Metadata::focus_layout` the body's
`FocusLayout` of `LayoutPoint`s, where raw-files has its table. The
`with_…` methods of `Metadata`, `Exposure`, `CameraSettings`,
`WhiteBalance`, `FocusPoints` and `FocusPoint`, `Lens::builder` and the
constructors, such as `Focus::new` and `FocusLayout::new`, build every
value without a file, for a program's tests. `Options` and `Limits` open
an untrusted file with other caps. The API documentation is on
[docs.rs](https://docs.rs/raw-files).

## Formats

Where this README says *my own* files, it is me, Alvaro Kuolas, who maintains
raw-files, writing about my own cameras: they stay private.

**Nikon NEF**, with the NRW files of Coolpix cameras.

- **Previews**: found in IFD0, in its SubIFDs and in the small preview
  that the MakerNote points at.
- **Metadata**: from the standard TIFF and Exif tags and, for what Nikon
  bodies before 2018 record only there, from the MakerNote: the serial
  number, the ISO, the lens range and the time zone. The visible image
  size is the crop area the MakerNote records.
- **Camera settings**: the MakerNote also gives the settings that Exif
  lacks: the white balance setting, finer than Exif's, the colour
  temperature, the fine-tuning, the shutter used (on the bodies that
  record it), the image area, the raw image size and, since no Nikon raw
  file records an Exif colour space, the colour space.
- **Focus point**: from the MakerNote's AFInfo2 block, which mirrorless
  bodies and DSLRs in live view fill in with a position; on a crop mode
  it is placed inside the cropped picture. A D600 shot through the
  viewfinder names its point by number, with a mask of the points used,
  and the points are placed from the D600's table, in the crop's window
  on a DX frame. The area mode is named for the codes that frames shot
  in a known mode confirmed.
- **Checked on**: 35 CC0 Nikon files from raw.pixls.us that CI downloads,
  three of the Z 9 (lossless, HE and HE\*) and 32 of 25 other bodies from
  the D1H to the Z5II; test files laid out like a Z 9's, a Z6III's, a
  D300's and a D1H's; two stripped files of my own Z 9; and the 56
  frames, stripped of their pixels, on which I measured the autofocus
  points of my own D600.

**Sony ARW**, with the SRF, SR2 and ARQ files of the same family and the
ARW files of the Sony-built Hasselblad bodies.

- **Previews**: found in IFD0 and IFD1 and, on bodies from version 4.0 of
  Sony's format, the full-size JPEG in IFD2.
- **Metadata**: from the standard tags. Most Sony bodies keep their serial
  number in a form the library does not read, and of the public samples
  only the ILCE-1 and the ILCE-7M5 give one.
- **Camera settings**: from the MakerNote: the white balance setting,
  finer than Exif's, the colour temperature under a chosen colour
  temperature or a custom white balance, the fine-tuning and the
  electronic front curtain setting (the menu setting, never the shutter
  used). The shutter used, the image area (the APS-C mode) and the raw
  image size stay unread, since Sony records the first two only in
  enciphered blocks and gives an APS-C raw the size code of a medium one.
- **Focus point**: on the bodies from the ILCE-7RM2 on, from the
  MakerNote's focus tags: the position from FocusLocation, or from
  FocusLocation2 when that is missing, the area's size from
  FocusFrameSize from the ILCE-7SM3 on, and the focus mode, since a shot
  in manual focus gives none.
- **Checked on**: 36 CC0 Sony-family files from the same site, from the
  DSC-F828's SRF to the A7 V's ARW 6.0, 35 of which CI downloads, and
  test files laid out like each generation.

**JPEG**, as a camera writes it beside its raw file or instead of it, or
as an editor exports it.

- **Previews**: the file itself, the thumbnail in the Exif segment, the
  images that the Multi-Picture Format (MPF) list holds after the main one
  and, on the Nikon bodies that write one, such as the D70, the small JPEG
  that the MakerNote points at.
- **Metadata**: from the Exif segment, read as in a NEF or an ARW file;
  the image size is the picture's own, and there is no raw size. A Nikon
  camera's JPEG also gives what its MakerNote records where the Exif
  segment does not, as a NEF does; a Sony camera's JPEG gives what its
  MakerNote records, as an ARW does: the camera settings above and the
  focus point. No other maker's MakerNote is read.
- **Focus point**: given only when its frame has the picture's shape, so
  an export cut to another shape or turned upright gives none, except one
  cut from a Nikon crop-mode JPEG, whose point may then be misplaced.
  The point of a Nikon crop-mode JPEG (DX, 1:1, 16:9) is placed on the
  picture. A D600 JPEG shot through the viewfinder gives its points
  placed from the table, as its NEF does, on a DX frame only at the
  crop's full size; a Medium JPEG, 4512x3008, gives none, since its
  shape, exactly 3:2, misses the table's frame by more than a pixel
  ([#62](https://github.com/kuolas/raw-files/issues/62)).
- **A damaged Exif segment**: a JPEG whose Exif segment is missing or
  cannot be read still opens, with the file itself and its MPF images as
  previews, its image size as the only value read from it, and the
  default orientation, no transform.
- **Checked on**: eight camera JPEGs from Wikimedia Commons that CI
  downloads, of five Sony bodies, two Nikon bodies and a Canon; JPEG files
  built in the tests; and three stripped JPEG files of my own Z 9 and
  one of my own D600.

Any other file, a DNG among them whatever its Make says, is refused with the
`UnknownFormat` error.

## Guarantees

- A malformed file returns an error, never a panic.
- Damaged metadata costs only what it damages: a bad value leaves the other
  values and the previews as they are, and a bad directory loses only the
  values in it. A JPEG whose Exif segment cannot be read still opens.
- `Limits` keeps an untrusted file from making the library allocate or walk
  without bound: a cap the reader cannot go on without is an error; past the
  first directory a cap leaves a directory out. In a JPEG file, a cap crossed
  inside its Exif or MPF segment costs what that segment holds, not the file.
- The format comes from the file's content, never from its name.
- Opening reads the structure and each preview's frame header through a block
  cache, not the whole file; a preview's bytes are read only when asked for.
- Pure Rust with no dependencies and no `unsafe` code. There is no serde
  feature, since that would be a dependency: the getters return text,
  integers, booleans, enums whose `name` gives their text, and small
  structs of these.

Pure Rust means no C or C++ library to build, link or ship, on macOS, Linux
and Windows alike.

## Build and test

raw-files is on [crates.io](https://crates.io/crates/raw-files); add it to a
project with `cargo add raw-files`. It needs Rust 1.95 or later. To build
from source on macOS, Linux or Windows:

```sh
cargo build
cargo test
```

Some tests read real sample files, which `scripts/fetch-samples.sh`
downloads (about 1.6 GB). `scripts/fetch-samples.sh nef`, `arw` or `jpeg`
fetches one format only, and the 325 MiB ARQ file only with
`RAW_FILES_LARGE=1`. Without the files, those tests skip.

Another test runs the reader over a folder of your own camera files, RAW and
JPEG, and counts per format. It prints counts only, never a path, the value
of a make, model, software, serial number, time, exposure or lens, or where
the camera focused, so its output can go into an issue:

```sh
RAW_FILES_LOCAL_CORPUS=/path/to/photos cargo test --release --test local_corpus -- --ignored --nocapture
```

[CONTRIBUTING.md](CONTRIBUTING.md) lists the full checks.

## Get involved

- **Found a bug or want a feature?** Open an
  [issue](https://github.com/kuolas/raw-files/issues/new/choose).
- **Found a security problem?** Please don't open a public issue; see
  [SECURITY.md](SECURITY.md).
- **Want to contribute code or docs?** Read [CONTRIBUTING.md](CONTRIBUTING.md).

Everyone taking part is expected to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) — how to build, test and send a change.
- [docs/ROADMAP.md](docs/ROADMAP.md) — what is planned, in what order.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the code is structured.
- [docs/architecture-decisions/](docs/architecture-decisions/README.md) — why
  things are the way they are.
- [CHANGELOG.md](CHANGELOG.md) — user-visible changes.

## Acknowledgements

- **Phil Harvey**, the creator of [ExifTool](https://exiftool.org/). Its
  published tag documentation is where raw-files learnt most of what the
  format notes record of the Nikon and Sony MakerNotes, and the program,
  run on the sample files as an independent reader to compare with, has
  checked what raw-files reads again and again. None of its code or
  tables is in raw-files, which is written from the notes
  ([ADR013](docs/architecture-decisions/adr013-clean-room-and-third-party-code.md)).
- The **exiv2** project, whose [MakerNote documentation](https://exiv2.org/makernote.html)
  the notes also draw on, and whose metadata listings on raw.pixls.us
  say which version of a block each body writes.
- **Laurent Clévy**, whose [NEF file format notes](http://lclevy.free.fr/nef/)
  are the backbone of what the notes record of the NEF container, its
  byte order, its IFD tree and where its previews sit, and whose Sony
  raw notes serve the ARW pages the same way; and the other community
  notes the format pages cite, the pixls.us forum threads, ozhiker's
  Nikon tag notes and DPReview's article on Nikon's white balance
  encryption among them.
- The open-source decoders whose source was read to settle facts about
  the formats, **dcraw**, **LibRaw**, **rawspeed**, **rawler** and
  **libopenraw**. Reading them taught what a published description did
  not say; none of their code was copied or translated, whatever its
  licence, and every fact learnt from them went through the notes and a
  sample file before it was coded
  ([ADR013](docs/architecture-decisions/adr013-clean-room-and-third-party-code.md)).
- [**raw.pixls.us**](https://raw.pixls.us/) and the photographers who gave
  their raw files to it under CC0: the public samples that CI downloads,
  and that the format notes cite file by file, are theirs.
- **Wikimedia Commons** and the photographers whose camera JPEGs, under
  CC0, public-domain dedications and CC BY licences, check the JPEG
  reader;
  [`tests/samples.tsv`](tests/samples.tsv) names each author whose
  licence asks for credit.

## Trademarks

Nikon and NEF, Sony and ARW, Hasselblad, and the other camera and format
names in this repository are trademarks of their owners. raw-files is an
independent project: it is not affiliated with, endorsed by or sponsored by
Nikon, Sony, Hasselblad or any other camera maker, and the formats it reads
are described from public sources and public sample files, as the format
notes record.

## License

Copyright (C) 2026 Alvaro Kuolas and the raw-files contributors.

raw-files is licensed under either of

- the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>), or
- the MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>),

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in raw-files by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
