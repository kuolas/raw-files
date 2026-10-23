# raw-files

A camera RAW library in pure Rust. It extracts the JPEG previews embedded in
Nikon NEF and Sony ARW files and reads their metadata and where the camera
focused, with fuller RAW decoding planned.

**Status: early.** Release 0.3.0 reads the JPEG previews embedded in Nikon
NEF and Sony ARW files, the SRF, SR2 and ARQ files of Sony's family and the
ARW files of the Sony-built Hasselblad bodies included, and the metadata a
viewer or a file renamer needs next to them: the capture time, with
sub-seconds and offset from UTC when recorded, the camera's make, model and
serial number, the lens, the exposure, the orientation, and the sizes of the
raw data and of the visible image. It gives the largest preview, or the
smallest that fills a given size, with the orientation to show it in, and it
never reads the raw data. It also reads where the camera focused, as the
file records it: the centre of the focus area, the frame it is measured in
and, when recorded, the area's size and whether the camera confirmed focus,
ready to be scaled onto any preview. Nikon mirrorless bodies, Nikon DSLRs in
live view and Sony bodies from the ILCE-7RM2 on record it. The shots that
record only an autofocus point's number or a mask of points, such as a
DSLR's through the viewfinder, give no focus point yet, and a shot in manual
focus gives none; plain JPEG files and raw decoding come later; see the
[roadmap](docs/ROADMAP.md).

## Usage

Open a file, take its largest preview and read the JPEG:

```rust
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

```rust
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

```rust
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

`RawFile` opens a path, bytes in memory or any reader that can seek. A
`Preview` says where the bytes are, how large the picture is, its format and
the orientation to show it in. `preview_at_least` picks the smallest preview
that fills a box. `Metadata` holds what the camera recorded about the
picture, read when the file is opened; every value is optional, since not
every camera records every one, and a damaged value is `None` on its own.
`Focus` is where the camera focused, and its `scaled_to` places it on a
preview. The `with_…` methods of `Metadata` and `Exposure`, `Lens::builder`
and the checked constructors, such as `Focus::new`, build every value
without a file, for a program's tests. `Options` and `Limits` open an
untrusted file with other caps. The API documentation is on
[docs.rs](https://docs.rs/raw-files).

## Formats

- **Nikon NEF**, with the NRW files of Coolpix cameras. The previews are
  found in IFD0, in its SubIFDs and in the small preview that the MakerNote
  points at. The metadata comes from the standard TIFF and Exif tags and,
  for what Nikon bodies before 2018 record only there, from the MakerNote:
  the serial number, the ISO, the lens range and the time zone; the visible
  image size is the crop area the MakerNote records. The focus point comes
  from the MakerNote's AFInfo2 block, which mirrorless bodies and DSLRs in
  live view fill in, and on a crop mode it is placed inside the cropped
  picture. Previews and metadata are checked on 33 CC0 Nikon files from
  raw.pixls.us that CI downloads, three of the Z 9 (lossless, HE and HE\*)
  and 30 of 24 other bodies from the D1H to the Z5II, on test files laid
  out like a Z 9's, a Z6III's, a D300's and a D1H's, and on two stripped
  files of the maintainer's own Z 9.
- **Sony ARW**, with the SRF, SR2 and ARQ files of the same family and the
  ARW files of the Sony-built Hasselblad bodies. The previews are found in
  IFD0 and IFD1 and, on bodies from version 4.0 of Sony's format, the
  full-size JPEG in IFD2. The metadata comes from the standard tags, and
  the focus point, on the bodies from the ILCE-7RM2 on, from the
  MakerNote's focus tags alone: the position from FocusLocation, or from
  FocusLocation2 when that is missing, the area's size from FocusFrameSize
  from the ILCE-7SM3 on, and the focus mode, since a shot in manual focus
  gives none; nothing else in the MakerNote is read. Most Sony bodies keep
  their serial number in a form the library does not read, and of the
  public samples only the ILCE-1 and the ILCE-7M5 give one. Previews and
  metadata are checked on 36 CC0 Sony-family files from the same site, from
  the DSC-F828's SRF to the A7 V's ARW 6.0, 35 of which CI downloads, and on
  test files laid out like each generation.

Any other file, a DNG among them whatever its Make says, is refused with the
`UnknownFormat` error.

## Guarantees

- A malformed file returns an error, never a panic.
- Damaged metadata costs only what it damages: a bad value leaves the other
  values and the previews as they are, and a bad directory loses only the
  values in it.
- `Limits` keeps an untrusted file from making the library allocate or walk
  without bound: a cap the reader cannot go on without is an error; past the
  first directory a cap leaves a directory out.
- The format comes from the file's content, never from its name.
- Opening reads the structure and each preview's frame header through a block
  cache, not the whole file; a preview's bytes are read only when asked for.
- Pure Rust with no dependencies and no `unsafe` code. There is no serde
  feature, since that would be a dependency: the getters return text,
  integers and small structs of integers.

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
downloads (about 1.6 GB). `scripts/fetch-samples.sh nef` or `arw` fetches one
format only, and the 325 MiB ARQ file only with `RAW_FILES_LARGE=1`. Without
the files, those tests skip.

Another test runs the reader over a folder of your own RAW files. It prints
counts only, never a path, the value of a make, model, serial number,
time, exposure or lens, or where the camera focused, so its output can go
into an issue:

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
