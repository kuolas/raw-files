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
| `src/formats/mod.rs` | the format table: one `FormatSpec` per format, a `claims` and a `parse` function over the TIFF tree, and detection, which asks each in order and lets the first that claims the file read it. The table is empty until the first format reader; each format will be a folder here | `container`, `io`, `format`, `preview`, `limits`, `error` |
| `src/io.rs` | `Source`: the one checked reader — bounded reads, checked offset arithmetic, a small block cache for the scattered reads of directory walking | `error` |
| `src/container/mod.rs` | the containers: file structures that several formats share, with no knowledge of camera vendors | — |
| `src/container/tiff/mod.rs` | the TIFF container's face to the crate: re-exports; one folder per container, so a container that grows never moves | — |
| `src/container/tiff/header.rs` | the TIFF header: the byte order and the offset of IFD0 | `io`, `error` |
| `src/container/tiff/ifd.rs` | TIFF IFDs: their entries, checked against the file and the entry limit, and typed access to tag values | `io`, `error`, `limits` |
| `src/container/tiff/walk.rs` | the IFD walk: IFD0's chain, SubIFDs and the Exif IFD, with a visited set, a depth cap and a total cap | `io`, `error`, `limits` |
| `src/container/tiff/robustness.rs` | tests only: the whole TIFF reader on truncated and corrupted files, which give an error or a tree within the limits, never a panic | — |
| `src/container/jpeg/mod.rs` | the JPEG container, as far as sizing a preview needs it: the walk over a stream's marker segments, inside a given range, to the frame header and the image's size; it never decodes | `io`, `error` |
| `src/fuzzing.rs` | the fuzz entry points, compiled only with `--cfg fuzzing`: each reads arbitrary bytes through a whole reader, or through the file handle as a caller would | `io`, `container`, `limits`, `options` |
| `tests/support/tiff_builder.rs` | the fixture builder: small TIFF files described in code, for unit and integration tests | — (`std` only) |
| `tests/support/jpeg.rs` | the stub JPEG: a small well-formed stream with a chosen frame header and no image, for unit and integration tests | — (`std` only) |
| `tests/tiff_builder.rs` | tests only: the fixture builder checked from outside the crate, which keeps `tests/support/` compiled | `tests/support` |
| `tests/jpeg_stub.rs` | tests only: the stub JPEG checked from outside the crate | `tests/support` |
| `tests/open.rs` | tests only: the public API from outside the crate (opening, detection, limits, errors, `Send`) | `tests/support` |
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
copy of each without it becoming part of the library.

## As the code grows

Each new module adds a row to the table above, and a change that moves data
between modules updates the data flow. A constraint that forces the shape
gets [an ADR](architecture-decisions/README.md).
