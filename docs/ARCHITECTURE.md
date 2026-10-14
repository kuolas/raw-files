# Architecture

How the code is structured, and why. Update this file in the same change as
any structural change.

## Today

raw-files is one Rust crate (edition 2024): a library, with no binary. Its
fuzz targets are a second crate, in `fuzz/`, that is never published.

| File | Holds | May depend on |
|---|---|---|
| `src/lib.rs` | the library root: the crate documentation and the re-exports | `error`, `file`, `format`, `geometry`, `limits`, `options`, `orientation`, `preview` |
| `src/error.rs` | `Error`, `ErrorKind`, `Result`: what can go wrong, by category, with the byte offset when known | — |
| `src/limits.rs` | `Limits`: the caps on what a file can make the library walk or allocate | — |
| `src/orientation.rs` | `Orientation`: what a viewer must do to the stored pixels to show the image upright, from the EXIF tag's eight values | — |
| `src/geometry.rs` | `Size`: a width and height in pixels, as stored | — |
| `src/preview.rs` | `Preview`, `PreviewFormat`: one embedded preview — its byte range in the file, its size measured from its frame header, its format and orientation — and the crate's ordering of previews, smallest first, and choice of the smallest at least a given size | `geometry`, `orientation` |
| `src/format.rs` | `Format`: the RAW formats the library knows, with their names and file extensions, and the lookup of a format by extension; opening a file never uses the extension | — |
| `src/options.rs` | `Options`: how a file is opened, today with which `Limits`; the one way a `RawFile` is built | `file`, `limits`, `error` |
| `src/file.rs` | `RawFile`: the file handle. Opening reads the TIFF structure, detects the format through the table in `formats` and keeps the previews sorted; reading a preview checks it against `max_preview_bytes` before reading it through the `Source` | `container`, `formats`, `io`, `options`, `preview`, `format`, `limits`, `error` |
| `src/formats/mod.rs` | the format table: one `FormatSpec` per format, a `claims` and a `parse` function over the TIFF tree, and detection, which asks each in order and lets the first that claims the file read it; and the two tests the readers' claims share, whether a file is a DNG and what its IFD0's Make starts with. The table holds NEF and ARW; each format is a folder here | `container`, `io`, `format`, `preview`, `limits`, `error` |
| `src/formats/tiff_previews.rs` | the preview finder the TIFF-based formats share: which IFDs of the tree describe raw data, or have a tag that cannot be read, and are skipped, where the others declare their JPEG, the checks on each candidate, the tree's and the ones a format adds (inside the file, a frame header, not lossless), the size measured from that frame header, and IFD0's orientation given to every preview | `container`, `io`, `preview`, `orientation`, `error` |
| `src/formats/nef/mod.rs` | the Nikon NEF reader: its claim, by IFD0's Make for a file that is not a DNG, and its spec in the format table, whose parse reads the previews through `nef/previews.rs` | `container`, `io`, `format`, `preview`, `limits`, `error` |
| `src/formats/nef/previews.rs` | where a NEF file keeps its previews, and their search: the shared finder over the whole tree, with the MakerNote's preview as its one extra candidate | `nef/makernote.rs`, `tiff_previews`, `container`, `io`, `limits`, `preview`, `error` |
| `src/formats/nef/makernote.rs` | the Nikon MakerNote, as far as its preview needs it: the layout with an embedded TIFF header, whose offsets count from that header, read as a block that every offset is checked against; the PreviewIFD's JPEG, as a candidate for the shared finder. A damaged block gives no candidate; only a failing reader is an error | `container`, `io`, `limits`, `error`, `tiff_previews` (for `Candidate`) |
| `src/formats/arw/mod.rs` | the Sony ARW reader, for the SRF, SR2 and ARQ files too: its claim, for a file that is not a DNG, by IFD0's Make and, for a Hasselblad Make, the pointer to Sony's private directory in Sony's four-byte shape, and its spec in the format table, whose parse reads the previews through `arw/previews.rs` | `container`, `io`, `format`, `limits`, `error` |
| `src/formats/arw/previews.rs` | where a Sony file keeps its previews (IFD0, IFD1 and, from FileFormat 4.0, IFD2; IFD1 alone on the SRF), and their search: the shared finder over the whole tree, with no candidate of its own | `tiff_previews`, `container`, `io`, `preview`, `error` |
| `src/io.rs` | `Source`: the one checked reader — bounded reads, checked offset arithmetic, a small block cache for the scattered reads of directory walking; `within`, the same bounds check on a range of a reader's own | `error` |
| `src/container/mod.rs` | the containers: file structures that several formats share, with no knowledge of camera vendors | — |
| `src/container/tiff/mod.rs` | the TIFF container's face to the crate: re-exports; one folder per container, so a container that grows never moves | — |
| `src/container/tiff/header.rs` | the TIFF header: the byte order and the offset of IFD0, read at the start of the file or at any offset, for a TIFF header embedded in the file, such as a MakerNote's | `io`, `error` |
| `src/container/tiff/ifd.rs` | TIFF IFDs: their entries, checked against the file and the entry limit, and typed access to tag values | `io`, `error`, `limits` |
| `src/container/tiff/tags.rs` | the numbers of the TIFF and Exif tags the crate reads, each named and documented for what it means | — |
| `src/container/tiff/types.rs` | the TIFF field types, each named and documented for the values it holds, and the size of one value of each | — |
| `src/container/tiff/walk.rs` | the IFD walk: IFD0's chain, SubIFDs and the Exif IFD, with a visited set, a depth cap and a total cap | `io`, `error`, `limits` |
| `src/container/tiff/robustness.rs` | tests only: the whole TIFF reader on truncated and corrupted files, which give an error or a tree within the limits, never a panic | — |
| `src/container/jpeg/mod.rs` | the JPEG container, as far as sizing a preview needs it: the walk over a stream's marker segments, inside a given range, to the frame header and the image's size; it never decodes | `io`, `error` |
| `src/fuzzing.rs` | the fuzz entry points, compiled only with `--cfg fuzzing`: each reads arbitrary bytes through a whole reader, or through the file handle as a caller would | `io`, `container`, `limits`, `options` |
| `tests/support/tiff_builder.rs` | the fixture builder: small TIFF files described in code, for unit and integration tests | — (`std` only) |
| `tests/support/jpeg.rs` | the stub JPEG: a small well-formed stream with a chosen frame header and no image, for unit and integration tests | — (`std` only) |
| `tests/support/nef.rs` | the Z 9 fixture: a file laid out like a Nikon Z 9's NEF files, with stub JPEGs and a Nikon MakerNote holding the small one, in the file's byte order or the other, for integration tests | `tests/support/tiff_builder.rs`, `tests/support/jpeg.rs` |
| `tests/support/arw.rs` | the Sony fixtures: files laid out like each generation of Sony's RAW family (ARW 4 with a lossless or a medium raw, ARW 2, ARQ, SR2, SRF, and the A100's ARW 1.0), with stub JPEGs and stand-in raw data, in either byte order, each with where its JPEGs and raw data are, for integration tests | `tests/support/tiff_builder.rs`, `tests/support/jpeg.rs` |
| `tests/support/samples.rs` | where the downloaded sample files of `tests/samples.tsv` are found: `RAW_FILES_SAMPLES` or `target/raw-samples`, with a note on stderr when a file is absent; the types of the table of what each sample must give, and the check that a sample opens with exactly its previews, each read whole; for integration tests | the library |
| `tests/tiff_builder.rs` | tests only: the fixture builder checked from outside the crate, which keeps `tests/support/` compiled | `tests/support` |
| `tests/jpeg_stub.rs` | tests only: the stub JPEG checked from outside the crate | `tests/support` |
| `tests/open.rs` | tests only: the public API from outside the crate (opening, detection, limits, errors, `Send`) | `tests/support` |
| `tests/nef.rs` | tests only: the NEF reader from outside the crate, on the Z 9 fixture (the previews listed, chosen and read, in both byte orders, with the MakerNote in either), and on the three Z 9 sample files of `tests/samples.tsv` (each preview's size, place, format and orientation, read whole), each skipped when its file is absent | `tests/support`, the downloaded samples |
| `tests/arw.rs` | tests only: the ARW reader from outside the crate, on the Sony fixtures (each layout's previews listed, chosen and read, in both byte orders; none of them the raw data; the A100's sized from its frame header and its SubIFD pointer shown not to be an IFD), and on the 36 Sony-family sample files of `tests/samples.tsv` (each preview's size, place, format, orientation and padding after its end marker, read whole), each skipped when its file is absent | `tests/support`, the downloaded samples |
| `tests/fuzz_seeds.rs` | tests only: writes the `open` fuzz target's seeds from the NEF and Sony fixtures and checks the committed files against them | `tests/support` |
| `tests/local_corpus.rs` | tests only: the reader run over a folder of your own RAW files (every file opens, has a preview, and its largest preview is a whole JPEG); ignored by default, run by hand on a folder named by `RAW_FILES_LOCAL_CORPUS`; prints counts only | the library |
| `tests/samples.tsv` | the real sample files the tests read when present: for each, its raw.pixls.us id, format, file name, tier, SHA-256, licence and URL. Not committed themselves; `scripts/fetch-samples.sh` downloads them into `target/raw-samples`, or `RAW_FILES_SAMPLES`, and checks each hash; a file of the large tier, the ARQ, is fetched only on request ([NEF test data](formats/nef/test-data.md#21-the-ci-set), [ARW test data](formats/arw/test-data.md#21-the-ci-set)) | — |
| `fuzz/` | the fuzz targets: a separate crate, never published, each target one call into `src/fuzzing.rs`; `fuzz/seeds/` holds the committed starting inputs, generated from the test fixtures and checked against them by a test. Its one dependency besides the library, `libfuzzer-sys`, compiles libFuzzer's C++ runtime: the only native code in the repository, kept out of the library, which stays pure Rust ([ADR007](architecture-decisions/adr007-pure-rust-cross-platform.md)) | the library, `libfuzzer-sys` |

**Data flow.** A file is opened through `Options`, or `RawFile`'s
constructors, which use the default options, from any `Read + Seek` source
([ADR009](architecture-decisions/adr009-public-api-shape.md)). The reader is
wrapped in an `io::Source`, which takes the file's length; the TIFF header
is read from it, and the walk finds the file's IFDs under the `Limits` and
returns them as a tree. Each format of the table in `formats` is then asked,
in order, whether it claims the tree; the first that does reads the
previews from it, and the handle keeps them sorted, smallest first. A file
that is not TIFF, or that no format claims, is `UnknownFormat`. Reading a
preview checks its length against `max_preview_bytes`, then reads its bytes
with `Source::read_into` into the caller's buffer, bypassing the block
cache.

**Untrusted input.** Everything the library will parse comes from a file, so
every offset and length read from one is checked before it is used, and a bad
value is an error, never a panic
([ADR008](architecture-decisions/adr008-lints-and-panics.md)). Every read
goes through `io::Source`, where its offset and length are checked against
the file's length.

**Test fixtures.** Tests describe the TIFF files they read in code, with
`tests/support/tiff_builder.rs`, and take their JPEG streams from
`tests/support/jpeg.rs`; `src/lib.rs` includes both files with `#[path]`
under `#[cfg(test)]`, so the unit tests and the integration tests share one
copy of each without it becoming part of the library. Whole files of one
camera's layout, built from those two, live beside them for the
integration tests: `tests/support/nef.rs`, the Z 9's, and
`tests/support/arw.rs`, one for each generation of Sony's family. Real
files are not committed: `tests/samples.tsv` pins the ones the tests read
by their SHA-256, and a test whose file has not been downloaded skips.

## As the code grows

Each new module adds a row to the table above, and a change that moves data
between modules updates the data flow. A constraint that forces the shape
gets [an ADR](architecture-decisions/README.md).
