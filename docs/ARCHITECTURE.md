# Architecture

How the code is structured: which file holds what, and what each file
may depend on. This page changes in the same change as the code it
describes. The [decision records](architecture-decisions/README.md) say
why the code has this shape, and
[when a change needs a record](architecture-decisions/README.md#when-to-write-one).

raw-files is one Rust crate (edition 2024): a library, with no binary.
Its fuzz targets are a second crate, in `fuzz/`, that is never
published.

Where the comments in the code and the tests say *I* or *my own*, it is
me, Alvaro Kuolas, who maintains raw-files. I write there about my own
cameras and shots. The shots stay private, and are cited by counts or
by frame numbers.

## The layers

The library has four parts. They are listed from what a caller opens
down to the values every part shares:

1. **The handle**: `CameraFile` and `Options`, in `src/file.rs` and
   `src/options.rs`. A caller opens a file through them.
2. **The format readers**, in `src/formats/`. Each claims the files of
   its format, finds their previews and reads their metadata. NEF, ARW
   and JPEG share the preview finder and the reader of the standard
   tags, and NEF and ARW share `parse_tiff`. The readers of the
   MakerNote, where each maker records data of its own, are in
   `makernote/`, one per maker, each with the same entry points.
3. **The containers**, in `src/container/`. They read a file's
   structure: the header and the directories (IFDs) of a TIFF file, the
   marker segments of a JPEG file. They know no camera maker and no
   format. Every read goes through `Source`, the one checked reader, in
   `src/io.rs`.
4. **The values**: the other public types, such as `Metadata`,
   `Preview`, `Limits` and `Error`. A program can build a `Metadata`
   without a file.

A part depends only on the parts below it. So no format reader calls
the handle, and no container knows a format reader. There is one
exception: `src/entries.rs` is a value, and it reads an entry's value
through the container.

## The files

One row per file: what it holds, in one sentence, and what it may
depend on. A file's rules are in its module docs, not here. A new file
adds a row.

### The crate root and the handle

| File | Holds | May depend on |
|---|---|---|
| `src/lib.rs` | The crate root: the crate documentation and the re-exports of the public types. | `authorship`, `camera`, `camera_settings`, `datetime`, `entries`, `error`, `file`, `focus`, `format`, `geometry`, `limits`, `location`, `metadata`, `options`, `orientation`, `preview`, `rational`, `tags`, `value_error` |
| `src/options.rs` | `Options`: how a file is opened, and under which `Limits`. | `file`, `limits`, `error` |
| `src/file.rs` | `CameraFile`: the file handle, giving the format, the previews, the metadata and the entries, and reading a preview's bytes or an entry's value. | `container`, `formats`, `io`, `options`, `preview`, `metadata`, `entries`, `format`, `limits`, `error`, `geometry` |

### The format readers

| File | Holds | May depend on |
|---|---|---|
| `src/formats/mod.rs` | The format table, one list per container; detection, where the first format that claims a file reads it into a `Parsed`; and `parse_tiff`, the steps every TIFF-based format takes. | `container`, `io`, `format`, `preview`, `metadata`, `entries`, `limits`, `error`, `makernote/mod.rs` (for `Reader`, `Picture` and `add_entries_of`), `makernote/points/`, `tiff_metadata`, `tiff_previews` |
| `src/formats/tiff_previews.rs` | The preview finder the TIFF-based formats and the JPEG format share: which IFDs hold JPEG previews, each checked and measured from its own frame header. | `container`, `io`, `preview`, `orientation`, `error`, `geometry`, `limits` |
| `src/formats/tiff_metadata.rs` | The metadata reader the TIFF-based formats and the JPEG reader share: it reads what the standard TIFF, Exif and GPS tags record. | `container`, `io`, `metadata`, `authorship`, `location`, `camera`, `camera_settings`, `datetime`, `geometry`, `rational`, `entries` (for `clean_text`), `error`, `tiff_previews` (for `orientation` and `is_raw`) |
| `src/formats/makernote/mod.rs` | The MakerNote readers, one per maker, shared by every format whose files carry a MakerNote: the trait each implements (`Reader`), `Picture`, and the block and color temperature bound they share. | `container`, `io`, `limits`, `error`, `entries`, `metadata`, `tiff_metadata` (for `Standard`), `tiff_previews` (for `Candidate`) |
| `src/formats/makernote/nikon/mod.rs` | The Nikon MakerNote reader's `metadata`, which its side of the `Reader` trait calls, and the rules of the tags its parts read; a short-header MakerNote's tags have none. | the files of `makernote/nikon/`, `makernote/mod.rs` (for `Picture`), `makernote/points/`, `camera` (for the body's facts), `camera_settings` (for `FocusMode`), `container` (for the rules), `error`, `focus`, `geometry`, `io`, `metadata`, `rational` (for the rules), `tiff_metadata` (for what it read) |
| `src/formats/makernote/nikon/block.rs` | The Nikon MakerNote's block, read once per file into a `MakerNote`, which implements the `Reader` trait: its layout, told by its first bytes, its MakerNote IFD, its PreviewIFD and the PreviewIFD's JPEG as a candidate preview. | `makernote/nikon/mod.rs` (for `metadata` and the rules), `makernote/mod.rs`, `container`, `entries`, `error`, `io`, `limits`, `metadata`, `tiff_metadata` (for `Standard`), `tiff_previews` (for `Candidate`) |
| `src/formats/makernote/nikon/metadata.rs` | What the Nikon MakerNote records of the picture beside the standard tags: CropArea, the serial number, the ISO, the lens's range and WorldTime. | `makernote/nikon/mod.rs` (for the rules), `makernote/nikon/block.rs`, `error`, `geometry`, `io`, `rational`, `tiff_metadata` (for its test of a crop and its text rule) |
| `src/formats/makernote/nikon/settings.rs` | The camera settings the Nikon MakerNote records: the white balance, the shutter used, the image area, the raw size and compression, the JPEG quality and compression, the color space, the focus mode, the AF-area mode, AF fine-tune, the stabilization, auto ISO, the exposure fine-tuning, the release mode and bracketing, daylight saving time, the flash's parts, the picture style, Active D-Lighting, HDR, the high ISO and long exposure noise reductions, vignette control and auto distortion control. | `makernote/nikon/mod.rs` (for the rules), `makernote/nikon/block.rs`, `makernote/nikon/crop.rs` (for CropHiSpeed), `makernote/nikon/af_info2.rs` (for the AF-area mode), `makernote/nikon/picture_control.rs` (for the Picture Control), `makernote/mod.rs` (for `Picture`), `camera` (for `BodyFacts`), `camera_settings`, `error`, `io`, `rational`, `tiff_metadata` (for its text rule) |
| `src/formats/makernote/nikon/crop.rs` | The Nikon MakerNote's CropHiSpeed, and placing the focus point on a crop or, from the body's table, the points a record names and the table's whole layout. | `makernote/nikon/mod.rs` (for the rules), `makernote/nikon/block.rs`, `makernote/nikon/metadata.rs` (for CropArea), `makernote/nikon/af_info2.rs` (for the record), `makernote/points/`, `error`, `focus`, `geometry`, `io` |
| `src/formats/makernote/nikon/picture_control.rs` | The Nikon MakerNote's Picture Control record, read at the layout of each version measured files hold, each version at its one length: a record of another version or length gives no Picture Control. | `camera_settings` (for `PictureControl` and its parts), `entries` (for `clean_text`), `rational` |
| `src/formats/makernote/nikon/af_info2.rs` | The Nikon MakerNote's AFInfo2, read at each version's offsets: the AF-area mode it records, the focus point and the points a DSLR's viewfinder shot names by number. | `makernote/points/`, `camera_settings` (for `FocusAreaMode`), `focus`, `geometry`, `io` |
| `src/formats/makernote/nikon/testing.rs` | Tests only: the MakerNote blocks, files and AFInfo2 records that the unit tests of several files of the Nikon MakerNote reader share. | `makernote/nikon/mod.rs`, `makernote/nikon/block.rs`, `container`, `focus`, `geometry`, `io`, `limits`, `metadata`, `tiff_metadata`, `tiff_previews` (for `Candidate`), `tests/support` |
| `src/formats/makernote/points/mod.rs` | `PointTable` and its registry: the tables of a body's autofocus points, which no file records, found by Model through `find` and `layout`. | `focus`, `geometry` |
| `src/formats/makernote/points/nikon_d600.rs` | The Nikon D600's table of autofocus points, with its layout code, bit order, and crop and JPEG windows. | `makernote/points/mod.rs`, `focus`, `geometry` |
| `src/formats/makernote/sony.rs` | The Sony MakerNote reader: the block found and checked, read once into one structure, which implements the `Reader` trait, giving the focus point and the camera settings it alone records, and the raw compression Sony's own codes name. | `makernote/mod.rs`, `container`, `entries`, `io`, `limits`, `error`, `geometry`, `focus`, `metadata`, `camera_settings`, `rational`, `tiff_metadata` (for `Standard`, `RawStorage` and its text rule) |
| `src/formats/nef/mod.rs` | The Nikon NEF reader: which files it claims, and its parse, the shared `parse_tiff` with the Nikon MakerNote reader. | `formats/mod.rs` (for `parse_tiff`), `makernote/mod.rs` (for `Reader`), `makernote/nikon/`, `container`, `io`, `format`, `limits`, `error` |
| `src/formats/nef/previews.rs` | Where a NEF file keeps its previews, the shared finder over the whole tree with the MakerNote's preview as its one extra candidate, and the tests of what the reader finds there. | nothing outside its tests |
| `src/formats/arw/mod.rs` | The Sony ARW reader, for SRF, SR2 and ARQ files too: its claim by Make, and its parse, the shared `parse_tiff` with the Sony MakerNote reader and SR2Private. | `formats/mod.rs` (for `parse_tiff`), `makernote/mod.rs` (for `Reader`), `makernote/sony.rs`, `container`, `io`, `format`, `entries`, `limits`, `error` |
| `src/formats/arw/previews.rs` | Where a Sony file keeps its previews, the shared finder over the whole tree with no candidate of its own, and the tests of what the reader finds there. | nothing outside its tests |
| `src/formats/jpeg/mod.rs` | The JPEG reader: its claim by the first frame header, its parse of the Exif segment, MakerNote, previews and entries, and the list of makers whose MakerNote it reads, one line each. | `makernote/mod.rs` (for `Reader` and `Picture`), `makernote/nikon/`, `makernote/sony.rs`, `tiff_metadata`, `tiff_previews`, `container`, `io`, `format`, `preview`, `metadata`, `camera_settings` (for `CameraSettings`, to clear the raw values), `entries`, `geometry`, `orientation`, `limits`, `error` |

### The containers

| File | Holds | May depend on |
|---|---|---|
| `src/io.rs` | `Source`, the one checked reader, with bounded reads and a block cache, and `within`, the same check on a range of a reader's own. | `error` |
| `src/container/mod.rs` | The containers, file structures that several formats share, and `Structure`, a file's structure as the container its first bytes name reads it. | `io`, `error`, `limits`, `container/jpeg`, `container/tiff` |
| `src/container/tiff/mod.rs` | The TIFF container's face to the crate: the re-exports of what the format modules use. | — |
| `src/container/tiff/block.rs` | `Block`: the base a TIFF structure's offsets count from and the bounds it may use, with the one conversion of an offset to a position. | `io` |
| `src/container/tiff/header.rs` | The TIFF header: the byte order and the offset of IFD0, read at the base of a block. | `io`, `error` |
| `src/container/tiff/ifd.rs` | TIFF IFDs: their entries, checked against the block they are read in, and the readers of their values by tag rule and by tag number. | `io`, `error`, `limits` |
| `src/container/tiff/rule.rs` | Tag rules: `Rule` and `Held`, a tag's number and the field types its value may have, in a shape that sets its count. | `rational`, `io` |
| `src/container/tiff/tags.rs` | The TIFF and Exif tags the crate reads, each documented for what a reader does with it, its number taken from `tags`. | `tags` |
| `src/container/tiff/field_type.rs` | `FieldType`: the one table of the TIFF field types, each with its code, size and name. | — |
| `src/container/tiff/walk.rs` | The IFD walk inside a block: it follows the links from IFD0 and returns every IFD it finds as a `Tree`, with each link it did not follow, bounded on any input. | `io`, `error`, `limits` |
| `src/container/tiff/robustness.rs` | Tests only: the whole TIFF reader on truncated and corrupted files, which give an error or a tree within the limits, never a panic. | — |
| `src/container/jpeg/mod.rs` | The JPEG container: one walk over a stream's marker segments, the probe that sizes a preview and the walk over a JPEG file. | `io`, `error`, `limits`, `container/tiff` (for `Block`) |
| `src/container/jpeg/mpf.rs` | The MPF index of a JPEG file: where the large thumbnails the file holds after its first image lie, the first of each class. | `io`, `error`, `limits`, `container/tiff` |

### The values

| File | Holds | May depend on |
|---|---|---|
| `src/error.rs` | `Error`, `ErrorKind` and `Result`: what can go wrong, by category and byte offset, and `skip_damage`, which turns damage in the file into `None`. | — |
| `src/value_error.rs` | `ValueError`: why a function refused its arguments, one variant per rule, each with a lowercase text that `Display` writes. | — |
| `src/limits.rs` | `Limits`: the caps on what a file can make the library walk or allocate. | — |
| `src/orientation.rs` | `Orientation`: what a viewer must do to the stored pixels to show the image upright, from the Exif tag's eight values. | — |
| `src/geometry.rs` | `Size` and `Point`, in pixels as stored: a width and height, a position from the left and top, and whether two sizes have one shape. | — |
| `src/datetime.rs` | `DateTime`: the time a camera's clock showed, always a real date and time, with the fraction of a second and the UTC offset when recorded. | `value_error` |
| `src/rational.rs` | `Rational` and `SignedRational`: a fraction exactly as the file stored it, with its reduction to lowest terms and its value as a decimal. | `value_error` |
| `src/authorship.rs` | `Authorship`: who made a picture and what they wrote about it, seven texts as the file records them, each optional, built through `with_…` methods. | — |
| `src/location.rs` | `Location`, `Latitude`, `Longitude`, `Altitude`, `Direction` and `North`: where a picture was taken, as the file records it, built through checked constructors. | `datetime`, `rational`, `value_error` |
| `src/camera.rs` | `Camera` and `Make`: the camera body a picture was taken with, as the file names it, every value optional, built through `with_…` methods; and raw-files' table of bodies, with each one's name and, for some, its sensor format and whether it offers raw sizes. | `camera_settings` (for `ImageArea`) |
| `src/camera_settings.rs` | `CameraSettings`, `WhiteBalance`, `WhiteBalanceFineTune`, `Flash`, `AfFineTune`, `Stabilization`, `Hdr`, `AutoHdr` and the setting enums: how the camera was set for a picture, every value optional. | `rational` |
| `src/focus.rs` | `Focus`, `FocusPoints`, `FocusLayout` and `PlacedFocusLayout`, with their parts: where the camera's autofocus aimed, the points it used and the body's layout of points, in the table's frame and placed on the picture. | `geometry`, `value_error` |
| `src/metadata.rs` | `Metadata`, `Exposure` and `Lens`: what the camera recorded about a picture, every value optional, each built without a file through `with_…` methods. | `authorship`, `camera`, `camera_settings`, `datetime`, `focus`, `geometry`, `location`, `orientation`, `rational` |
| `src/preview.rs` | `Preview` and `PreviewFormat`: one preview in a file, with its byte range, size, format and orientation, and the choice among previews by size. | `geometry`, `orientation` |
| `src/entries.rs` | `Directory`, `Entry`, `Value` and `Entries`: the raw view, every entry of every TIFF directory a file holds, as recorded, with its value decoded on request, and each directory left out as a `SkippedDirectory`. | `container`, `io`, `error`, `rational` |
| `src/entries/display.rs` | `ValueDisplay`: the short form of a `Value` that `Value::display` gives, cut after a few values. | `entries` |
| `src/entries/names.rs` | The name of an entry's tag, from the table of `tags` for its kind of directory. | `entries`, `tags` |
| `src/entries/skipped.rs` | `SkippedDirectory`: a directory a file links to that could not be read, with the link and the cause; and what a reader found where one of its own links leads. | `entries`, `error` |
| `src/tags.rs` | `tags`: a constant for each tag `Entry::name` has a name for, one module per kind of directory, each row giving the constant and its name at once. | — |
| `src/format.rs` | `Format`: the camera file formats the library reads, NEF, ARW and JPEG, with their names, extensions and whether each holds raw data. | — |

### The tests

| File | Holds | May depend on |
|---|---|---|
| `src/testing.rs` | Tests only: a reader that fails past the first cached block, the checks of an enum's names and order, and values the tests build from, a fraction of each sign among them. | `focus`, `geometry`, `io`, `rational` |
| `tests/support/mod.rs` | For the integration tests: the module that gathers the helpers of `tests/support/`, with the size and fraction conversions that the tests of every format use. | the files of `tests/support/`, the library |
| `tests/support/tiff_builder.rs` | The fixture builder: small TIFF files described in code, with any rule broken on purpose, in either byte order or both, for unit and integration tests. | — (`std` only) |
| `tests/support/jpeg_stub.rs` | The stub JPEG: a small well-formed stream with a chosen frame header and no image, and the baseline stub that built files hold as previews, for unit and integration tests. | — (`std` only) |
| `tests/support/jpeg_builder.rs` | The JPEG file builder: segments, the stub as main image, an MPF index and the images after it, with where each part landed. | `tests/support/jpeg_stub.rs`, `tests/support/tiff_builder.rs` (for `Order`) (`std` only) |
| `tests/support/jpeg_files.rs` | Whole JPEG files: plain, Exif, padded, and Nikon and Sony camera layouts with their MakerNotes, for the JPEG reader's, robustness and fuzz-seed tests. | `tests/support/jpeg_builder.rs`, `tests/support/jpeg_stub.rs`, `tests/support/tiff_builder.rs`, `tests/support/nef.rs`, `tests/support/arw.rs` |
| `tests/support/nef.rs` | For the integration tests: files laid out like the NEF files of a Z 9, D300, Z6III and D1H, and the Nikon MakerNote builder. | `tests/support/tiff_builder.rs`, `tests/support/jpeg_stub.rs` |
| `tests/support/arw.rs` | For the integration tests: files laid out like each generation of Sony's RAW family, ARW 4 to the A100's ARW 1.0, with stub JPEGs. | `tests/support/tiff_builder.rs`, `tests/support/jpeg_stub.rs` |
| `tests/support/samples.rs` | For the integration tests: where the downloaded samples of `tests/samples.tsv` are found, the types of the tables of expected values, the checks against them for a sample or a committed fixture, and the check that a format's tables name the files the list names. | the library, `tests/support/fixtures.rs` |
| `tests/support/fixtures.rs` | For the integration tests: where the committed files of `tests/fixtures/` are found and read, a missing one being a failure. | — (`std` only) |
| `tests/fixtures/` | Real camera files of my own, stripped of their pixels and personal fields, which `tests/fixtures/README.md` lists. | — |
| `tests/tiff_builder.rs` | Tests only: the TIFF fixture builder, checked from outside the crate; its only tests. | `tests/support` |
| `tests/samples.rs` | Tests only: the helpers of `tests/support/samples.rs` that need no sample file: the collector, the require switch and the tiers. | `tests/support` |
| `tests/jpeg_stub.rs` | Tests only: the stub JPEG, checked from outside the crate. | `tests/support` |
| `tests/jpeg_builder.rs` | Tests only: the JPEG file builder, checked from outside the crate: its layout, the positions it reports and the MPF index it writes. | `tests/support` |
| `tests/open.rs` | Tests only: opening files through the public API: detection, limits, errors, `Send`, and authorship, camera and location built by hand against what opening reads. | `tests/support` |
| `tests/nef.rs` | Tests only: the NEF reader from outside the crate, on files built in code, the committed files of `tests/fixtures/` and the downloaded samples. | `tests/support`, `tests/fixtures`, the downloaded samples |
| `tests/arw.rs` | Tests only: the ARW reader from outside the crate, on each Sony layout built in code and on the downloaded Sony samples. | `tests/support`, the downloaded samples |
| `tests/jpeg.rs` | Tests only: the JPEG reader from outside the crate, on files built in code, on the committed JPEGs of `tests/fixtures/` and on the downloaded samples. | `tests/support`, `tests/fixtures`, the downloaded samples |
| `tests/fuzz_seeds.rs` | Tests only: writes the seeds of the `open` and `jpeg_segments` fuzz targets from the fixtures, and checks the committed seed files against them. | `tests/support`, `tests/fixtures` |
| `tests/robustness.rs` | Tests only: opening damaged RAW and JPEG files, cut at every length and overwritten byte by byte: each opens or fails, never panics. | `tests/support`, `tests/fixtures` |
| `tests/local_corpus.rs` | Tests only: the reader run over a folder of your own camera files, ignored by default, and unit tests of its checks and tallies. | the library |
| `tests/samples.tsv` | The real sample files the tests read when present, each pinned by its SHA-256, with its source, tier, license and URL. | — |
| `scripts/fetch-samples.sh` | Downloads the files of `tests/samples.tsv` into `raw-samples/` and checks each SHA-256. | `tests/samples.tsv`, `curl`, `sha256sum` or `shasum` |
| `scripts/check-series.sh` | Builds and tests every commit of a branch in a throwaway worktree, so every commit that lands on `master` is known to pass. | `git`, `cargo` |
| `scripts/install-hooks.sh` | Turns on the git hooks of `.githooks/` for a checkout, and sets `pull.ff=only`. | `git` |
| `scripts/test-hooks.sh` | Tests the `commit-msg` hook of `.githooks/` on message files and through a real `git commit`, and the `pre-push` hook through a real `git push`. | `.githooks/commit-msg`, `.githooks/pre-push`, `git`, `cargo` |

### The fuzz targets

| File | Holds | May depend on |
|---|---|---|
| `src/fuzzing.rs` | The entry points of the fuzz targets in `fuzz/`: each reads arbitrary bytes through a reader, or through the file handle as a caller would. | `io`, `container`, `entries`, `geometry`, `focus`, `limits`, `location`, `datetime`, `metadata`, `options`, `rational` |
| `fuzz/` | The fuzz targets and their committed seed inputs: a separate crate, never published, each target one call into `src/fuzzing.rs`. | the library, `libfuzzer-sys` |

## How a file is read

A change that moves data between modules updates this section.

A caller opens a file through `Options`, or through `CameraFile`'s
constructors, which use the default options
([ADR009](architecture-decisions/adr009-public-api-shape.md)). The file
can come from any `Read + Seek` source. Opening wraps it in an
`io::Source`, which takes the file's length.

The file's first bytes name its container: `FF D8 FF` a JPEG file, and
`II` or `MM` a TIFF file. The container then reads the file's
structure:

- of a JPEG file, its marker segments;
- of a TIFF file, its header, then its IFDs, which the walk finds under
  the `Limits` and returns as a tree.

Each format in that container's list in `formats` is asked in turn
whether it claims the structure. The first that does reads the
previews, the metadata and the entries from it. The handle keeps all
three, the previews sorted smallest first. A file of neither container,
or one that no format claims, is `UnknownFormat`.

The JPEG container's list holds one format, JPEG. Its reader walks the
TIFF structure of the file's Exif segment, inside that segment. It
reads the Nikon or the Sony MakerNote, chosen by IFD0's Make, and the
file's MPF list. It lists the file itself among the previews.

`metadata()` and `entries()` return what was read at opening and read
nothing more. `read_value()` reads an entry's value from the file,
unless the entry holds it. `read_preview()` and `read_preview_into()`
check a preview's length against `max_preview_bytes`. They then read its
bytes into a buffer with `Source::read_into`, bypassing the block cache.

## Untrusted input

Everything the library parses comes from a file. So every offset and
length read from one is checked before it is used, and a bad value is
an error, never a panic
([ADR008](architecture-decisions/adr008-lints-and-panics.md)). Every
read goes through `io::Source`, which checks its offset and length
against the file's length.

## Test fixtures

The tests build in code every file they read but the real camera files
below, with the builders listed under [The tests](#the-tests).
`src/lib.rs` includes the TIFF builder, the stub JPEG and the JPEG file
builder with `#[path]` under `#[cfg(test)]`. So the unit tests and the
integration tests share one copy of each, and none becomes part of the
library. The published package carries these three files of
`tests/support/` and no other, so its unit tests run from the package.

The unit tests of any module may use `src/testing.rs`. Its failing
reader lets a reader's tests tell a failure of the reader from damage in
the file. Its checks test an enum's names and their order.

The real files of my own cameras in
[`tests/fixtures/`](../tests/fixtures/README.md) are committed, and an
integration test never skips them. The other real files are not committed:
`tests/samples.tsv` pins each by its SHA-256. A test whose file has not
been downloaded skips. The ignored test of `tests/local_corpus.rs` reads
a folder of your own files instead.
