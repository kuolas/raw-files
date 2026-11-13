# raw-files

A camera RAW library in pure Rust. It reads Nikon NEF and Sony ARW raw
files, and the JPEG files cameras write. It gives their previews, their
metadata and where the camera focused. Fuller RAW decoding is planned.

Pure Rust means no C or C++ library to build, link or ship, on macOS,
Linux and Windows alike.

**Status: early.** The [roadmap](docs/ROADMAP.md) says what comes next.

## What it reads

- **Previews.** The JPEG previews a file holds, read without touching
  the raw data. raw-files gives the largest, or the smallest that fills
  a given size, with the orientation to show it in.
- **Metadata.** What a viewer or a file renamer needs next to a preview:
  - when the picture was taken, to the sub-second and with the offset
    from UTC where the file records them;
  - the camera's make, model, serial number and owner's name;
  - the maker raw-files recognizes in them, such as Nikon;
  - the name the body is sold under, such as Z6III, for the bodies the
    [notes on camera names](docs/formats/camera-names.md) list;
  - the software that wrote the file;
  - who made the picture: the artist, the copyright notice, the
    photographer, the editor, the title, the description and the
    comment;
  - where it was taken: the latitude and longitude, in decimal degrees
    too, the altitude, the GPS time, the direction the camera faced and
    the map datum;
  - the lens;
  - the exposure, with the focal length in 35 mm terms;
  - the orientation;
  - the sizes of the raw data and of the visible image.
- **Camera settings.**
  - the white balance, with its color temperature and fine-tuning;
  - the exposure program;
  - the metering mode;
  - the flash;
  - the shutter;
  - Sony's electronic front curtain setting;
  - the image area;
  - the raw image size;
  - the color space.
  - [`CameraSettings`](https://docs.rs/raw-files/latest/raw_files/struct.CameraSettings.html)
    says which a maker's files leave out: Sony's give no shutter, image
    area or raw image size.
- **The focus point.** Where the camera focused, ready to place on any
  preview.
  - Nikon mirrorless bodies, Nikon DSLRs in live view and Sony bodies
    from the ILCE-7RM2 on record its position.
  - A Nikon DSLR shot through the viewfinder names only the number of an
    autofocus point. raw-files places that point from its own table of
    the body's points, measured on frames shot with that body.
  - The Nikon D600 is the first body with a table. Further bodies join
    as their tables are measured.
  - With the point come the autofocus points the camera used and the
    area mode. A body with a table also gives the layout of all its
    points, for a viewer to draw.
  - Some shots give no point, such as one in manual focus.
    [`Metadata::focus`](https://docs.rs/raw-files/latest/raw_files/struct.Metadata.html#method.focus)
    says which.
- **JPEG files.** The JPEG files cameras write beside their raw files or
  instead of them. A JPEG's previews are the file itself and the smaller
  images it holds. A Nikon or Sony camera's JPEG also gives what its
  MakerNote records, the focus point among it. Any other camera's JPEG
  gives what its Exif segment records. A JPEG gives no point when its
  picture's shape differs from the frame the point is measured in, and
  [`Metadata::focus`](https://docs.rs/raw-files/latest/raw_files/struct.Metadata.html#method.focus)
  names the exceptions.
- **Every entry.** Every entry of the TIFF directories raw-files walks,
  as the file records it. Each comes with its directory, tag, type and
  count, and the name a published standard gives its tag. Its value is
  read when asked for.
  [`RawFile::entries`](https://docs.rs/raw-files/latest/raw_files/struct.RawFile.html#method.entries)
  lists the directories.
- **Not yet.** Decoding the raw sensor data. The
  [roadmap](docs/ROADMAP.md) says what comes after.

## Example

Open a file, print when the picture was taken, and read its largest
preview:

```rust,no_run
use raw_files::RawFile;

fn main() -> raw_files::Result<()> {
    let mut file = RawFile::open("photo.nef")?;
    if let Some(time) = file.metadata().capture_time() {
        println!("taken {time}");
    }
    if let Some(preview) = file.largest_preview() {
        let jpeg = file.read_preview(&preview)?;
        println!("a {} preview of {} bytes", preview.size(), jpeg.len());
    }
    Ok(())
}
```

The [API documentation](https://docs.rs/raw-files) guides through the
rest of the library, with seven examples.

## Guarantees

A malformed file returns an error, never a panic. The
[API documentation](https://docs.rs/raw-files/latest/raw_files/#guarantees)
lists every guarantee, from damaged metadata to the caps on an
untrusted file.

## Formats

Where this README says *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files. They come from my own cameras and stay private.

- **Nikon NEF**, with the NRW files of Coolpix cameras
  ([format notes](docs/formats/nef/README.md)). Checked on:
  - 35 CC0 files from raw.pixls.us, of 26 bodies from the D1H to the
    Z5II, which CI downloads;
  - among them, three of the Z 9: lossless, HE and HE\*;
  - test files laid out like a Z 9's, a Z6III's, a D300's and a D1H's;
  - two stripped files of my own Z 9;
  - the frames on which I measured my own D600's autofocus points,
    stripped of their pixels
    ([the fixtures](tests/fixtures/README.md#the-d600-session));
  - the NEF of one more D600 frame, shot as a pair with a Medium JPEG.
- **Sony ARW**, with the SRF, SR2 and ARQ files of the same family and
  the ARW files of the Sony-built Hasselblad bodies
  ([format notes](docs/formats/arw/README.md)). Checked on:
  - 36 CC0 files from raw.pixls.us, from the DSC-F828's SRF to the
    ILCE-7M5's ARW 6.0, of which CI downloads all but the large ARQ;
  - test files laid out like each generation of the format.
- **JPEG**, as a camera writes it beside its raw file or instead of it,
  or as an editor exports it
  ([format notes](docs/formats/jpeg/README.md)). Checked on:
  - eight camera JPEGs from Wikimedia Commons, which CI downloads: five
    of Sony bodies, two of Nikon bodies and one of a Canon;
  - three stripped JPEG files of my own Z 9 and three of my own D600;
  - JPEG files built in the tests.

Any other file, a DNG among them whatever its Make says, is refused with
the `UnknownFormat` error.

## Build and test

raw-files is on [crates.io](https://crates.io/crates/raw-files). Add it
to a project with `cargo add raw-files`. It needs Rust 1.95 or later. To
build from source on macOS, Linux or Windows:

```sh
cargo build
cargo test
```

Some tests read real sample files, which `scripts/fetch-samples.sh`
downloads (about 1.6 GB). Without the files, those tests skip.
`scripts/fetch-samples.sh nef`, `arw` or `jpeg` fetches one format only.
The 325 MiB ARQ file comes only with `RAW_FILES_LARGE=1`.

Another test runs the reader over a folder of your own camera files, RAW
and JPEG. It prints totals only, tied to no file, so its output can go
into an issue:

```sh
RAW_FILES_LOCAL_CORPUS=/path/to/photos cargo test --release --test local_corpus -- --ignored --nocapture
```

[docs/TESTING.md](docs/TESTING.md#the-local-corpus-test) lists what the
output never holds.

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

- **To use the library:** the [API documentation](https://docs.rs/raw-files),
  with examples and the guarantees.
- **To change it:**
  - [CONTRIBUTING.md](CONTRIBUTING.md): how to build, test and send a
    change;
  - [docs/TESTING.md](docs/TESTING.md): the sample files, the ignored
    tests, fuzzing and what CI runs;
  - [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): which file holds what;
  - the [decision records](docs/architecture-decisions/README.md): why
    the code has its shape.
- **To know what a file holds:** the
  [format notes](docs/formats/README.md) record what NEF, ARW and camera
  JPEG files hold, and where each fact comes from.

[docs/ROADMAP.md](docs/ROADMAP.md) says what raw-files aims for, what it
reads so far, and what comes next. [CHANGELOG.md](CHANGELOG.md) lists the
user-visible changes.

## Acknowledgments

- **Phil Harvey**, the creator of [ExifTool](https://exiftool.org/).
  raw-files learned most of what the format notes record of the Nikon
  and Sony MakerNotes from its published tag documentation. The program,
  run on the sample files as an independent reader, has checked what
  raw-files reads again and again. None of its code or tables is in
  raw-files, which is written from the notes under its clean-room rule
  ([ADR013](docs/architecture-decisions/adr013-clean-room-and-third-party-code.md)).
- The **exiv2** project. The notes also draw on its
  [MakerNote documentation](https://exiv2.org/makernote.html). Its
  metadata listings on raw.pixls.us say which version of a block each
  body writes.
- **Laurent Clévy**, for the
  [NEF file format notes](http://lclevy.free.fr/nef/). They are the
  backbone of what the notes record of the NEF container: its byte
  order, its IFD tree and where its previews sit. The Sony raw notes of
  the same site serve the ARW pages the same way.
- The other community notes the format pages cite, among them the
  pixls.us forum threads, ozhiker's Nikon tag notes and DPReview's
  article on Nikon's white balance encryption.
- The open-source decoders whose source was read to settle facts about
  the formats: **dcraw**, **LibRaw**, **rawspeed**, **rawler** and
  **libopenraw**. Reading them taught what a published description did
  not say. None of their code was copied or translated, whatever its
  license. Every fact learned from them went through the notes and a
  sample file before it was coded, under the same clean-room rule
  ([ADR013](docs/architecture-decisions/adr013-clean-room-and-third-party-code.md)).
- [**raw.pixls.us**](https://raw.pixls.us/) and the photographers who
  gave their raw files to it under CC0. The public samples that CI
  downloads, and that the format notes cite file by file, are theirs.
- **Wikimedia Commons** and the photographers whose camera JPEGs check
  the JPEG reader, under CC0, public-domain dedications and CC BY
  licenses. [`tests/samples.tsv`](tests/samples.tsv) names each author
  whose license asks for credit.

## Trademarks

Nikon and NEF, Sony and ARW, Hasselblad, and the other camera and format
names in this repository are trademarks of their owners. raw-files is an
independent project. It is not affiliated with, endorsed by or sponsored
by Nikon, Sony, Hasselblad or any other camera maker. The formats it
reads are described from public sources and public sample files, as the
format notes record.

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
