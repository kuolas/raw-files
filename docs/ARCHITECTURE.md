# Architecture

How the code is structured, and why. Update this file in the same change as
any structural change.

## Today

raw-files is one Rust crate (edition 2024): a library, with no binary. Its
fuzz targets are a second crate, in `fuzz/`, that is never published.

| File | Holds | May depend on |
|---|---|---|
| `src/lib.rs` | the library root: the crate documentation and the re-exports | `error`, `geometry`, `limits`, `orientation` |
| `src/error.rs` | `Error`, `ErrorKind`, `Result`: what can go wrong, by category, with the byte offset when known | — |
| `src/limits.rs` | `Limits`: the caps on what a file can make the library walk or allocate | — |
| `src/orientation.rs` | `Orientation`: what a viewer must do to the stored pixels to show the image upright, from the EXIF tag's eight values | — |
| `src/geometry.rs` | `Size`: a width and height in pixels, as stored | — |
| `src/io.rs` | `Source`: the one checked reader — bounded reads, checked offset arithmetic, a small block cache for the scattered reads of directory walking | `error` |
| `src/container/mod.rs` | the containers: file structures that several formats share, with no knowledge of camera vendors | — |
| `src/container/tiff/mod.rs` | the TIFF container's face to the crate: re-exports; one folder per container, so a container that grows never moves | — |
| `src/container/tiff/header.rs` | the TIFF header: the byte order and the offset of IFD0 | `io`, `error` |
| `src/container/tiff/ifd.rs` | TIFF IFDs: their entries, checked against the file and the entry limit, and typed access to tag values | `io`, `error`, `limits` |
| `src/container/tiff/walk.rs` | the IFD walk: IFD0's chain, SubIFDs and the Exif IFD, with a visited set, a depth cap and a total cap | `io`, `error`, `limits` |
| `src/container/tiff/robustness.rs` | tests only: the whole TIFF reader on truncated and corrupted files, which give an error or a tree within the limits, never a panic | — |
| `src/container/jpeg/mod.rs` | the JPEG container, as far as sizing a preview needs it: the walk over a stream's marker segments, inside a given range, to the frame header and the image's size; it never decodes | `io`, `error` |
| `src/fuzzing.rs` | the fuzz entry points, compiled only with `--cfg fuzzing`: each reads arbitrary bytes through a whole reader | `io`, `container`, `limits` |
| `tests/support/tiff_builder.rs` | the fixture builder: small TIFF files described in code, for unit and integration tests | — (`std` only) |
| `tests/support/jpeg.rs` | the stub JPEG: a small well-formed stream with a chosen frame header and no image, for unit and integration tests | — (`std` only) |
| `tests/tiff_builder.rs` | tests only: the fixture builder checked from outside the crate, which keeps `tests/support/` compiled | `tests/support` |
| `tests/jpeg_stub.rs` | tests only: the stub JPEG checked from outside the crate | `tests/support` |
| `fuzz/` | the fuzz targets: a separate crate, never published, each target one call into `src/fuzzing.rs`; `fuzz/seeds/` holds the committed starting inputs, generated from the test fixtures and checked against them by a test. Its one dependency besides the library, `libfuzzer-sys`, compiles libFuzzer's C++ runtime: the only native code in the repository, kept out of the library, which stays pure Rust ([ADR007](architecture-decisions/adr007-pure-rust-cross-platform.md)) | the library, `libfuzzer-sys` |

**Data flow.** None yet: nothing reads files so far. The API to come reads any
`Read + Seek` source through one handle
([ADR009](architecture-decisions/adr009-public-api-shape.md)).

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
