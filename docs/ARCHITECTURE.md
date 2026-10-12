# Architecture

How the code is structured, and why. Update this file in the same change as
any structural change.

## Today

raw-files is one Rust crate (edition 2024): a library, with no binary.

| File | Holds | May depend on |
|---|---|---|
| `src/lib.rs` | the library root: the crate documentation; no public API yet | — |

**Data flow.** None yet: there is no public API. The API to come reads any
`Read + Seek` source through one handle
([ADR009](architecture-decisions/adr009-public-api-shape.md)).

**Untrusted input.** Everything the library will parse comes from a file, so
every offset and length read from one is checked before it is used, and a bad
value is an error, never a panic
([ADR008](architecture-decisions/adr008-lints-and-panics.md)).

## As the code grows

Each new module adds a row to the table above, and a change that moves data
between modules updates the data flow. A constraint that forces the shape
gets [an ADR](architecture-decisions/README.md).
