# Architecture

How the code is structured, and why. Update this file in the same change as
any structural change.

## Today

raw-files is one Rust crate (edition 2024): a library, with no binary.

| File | Holds | May depend on |
|---|---|---|
| `src/lib.rs` | the library root: the crate documentation and the re-exports | `error`, `limits` |
| `src/error.rs` | `Error`, `ErrorKind`, `Result`: what can go wrong, by category, with the byte offset when known | — |
| `src/limits.rs` | `Limits`: the caps on what a file can make the library walk or allocate | — |
| `src/io.rs` | `Source`: the one checked reader — bounded reads, checked offset arithmetic, a small block cache for the scattered reads of directory walking | `error` |
| `src/container/mod.rs` | the containers: file structures that several formats share, with no knowledge of camera vendors | — |
| `src/container/tiff/mod.rs` | the TIFF container's face to the crate: re-exports; one folder per container, so a container that grows never moves | — |
| `src/container/tiff/header.rs` | the TIFF header: the byte order and the offset of IFD0 | `io`, `error` |
| `tests/support/tiff_builder.rs` | the fixture builder: small TIFF files described in code, for unit and integration tests | — (`std` only) |
| `tests/tiff_builder.rs` | tests only: the fixture builder checked from outside the crate, which keeps `tests/support/` compiled | `tests/support` |

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
`tests/support/tiff_builder.rs`; `src/lib.rs` includes that file with
`#[path]` under `#[cfg(test)]`, so the unit tests and the integration tests
share one builder without it becoming part of the library.

## As the code grows

Each new module adds a row to the table above, and a change that moves data
between modules updates the data flow. A constraint that forces the shape
gets [an ADR](architecture-decisions/README.md).
