# ADR009: The shape of the public API

- Status: Accepted
- Date: 2026-10-11

## Context

Two kinds of program set the scope of raw-files. A culler, such as
[photography-workbench](https://github.com/kuolas/photography-workbench),
opens thousands of files to show their embedded previews, often from a
network share, and cannot afford to read a 50 MB file to show a 1 MB JPEG.
An editing application needs more of the same file later: metadata, raw
data, a developed image. Both open files from anyone, so every file is
untrusted input.

Before deciding, we read the APIs of other RAW and image libraries, in Rust
and in other languages. The ones that serve previews well open a file into a
handle and answer questions about it afterwards, instead of decoding
everything up front. They list the previews with their sizes and fetch the
chosen one's bytes separately, and a preview carries the orientation needed
to show it upright. The Rust image crates mark their enums
`#[non_exhaustive]`, cap allocations with explicit limits, and report errors
by category; callers mostly need to tell "not supported" from "corrupt".

For input, the options were a byte slice (simple, but the whole file must be
in memory), a `Read + Seek` source, or both through separate types.

## Decision

We will expose **one handle, `RawFile<R>`, generic over any
`R: Read + Seek`**.

- `RawFile::open(path)` returns a `RawFile<File>`. Small reads go through a
  block cache inside raw-files, so walking a file's scattered TIFF
  directories costs one read per block. Callers do not wrap the file in a
  `BufReader`, which discards its buffer on every seek.
- Bytes already in memory, including a memory-mapped file, are opened with
  `RawFile::from_bytes`, a `RawFile<Cursor<&[u8]>>`: the same handle and the
  same code path.
- Opening reads only the file's structure. Each capability (previews now;
  metadata, raw data and development later) then succeeds or fails on its
  own, so a file whose raw data is not supported still gives its previews.
- The format is detected from the file's content, never from its name.
  Format modules produce format-neutral public types: supporting a new
  format adds a `Format` variant and nothing else. There is no public
  decoder trait; dispatch stays inside the crate.
- No third-party types in the public API. Conversions to other crates' types
  can come later behind optional features.
- Errors are one opaque `Error` with a `#[non_exhaustive]` `ErrorKind` (I/O,
  unknown format, unsupported, malformed, limit exceeded) and, when known,
  the byte offset in the file. Something a file does not contain is `None`,
  not an error.
- Enums that may gain variants and records that may gain fields are
  `#[non_exhaustive]`, and structs expose getters; only small fixed value
  types such as `Size` have public fields.
- Every allocation whose size comes from the file is capped by `Limits`,
  whose defaults are safe for untrusted input.
- raw-files stays **one crate**. The raw-data and development APIs arrive
  behind an `unstable` Cargo feature, outside the compatibility promise,
  until an application has used them.

The first release has this surface:

- `RawFile`: `open`, `from_bytes`, `from_reader`, `format`, `previews`,
  `largest_preview`, `preview_at_least`, `read_preview`,
  `read_preview_into`, `into_inner`;
- `Options` and `Limits`, to open with other limits;
- `Format`, with a name and file extensions for file pickers;
- `Preview` (size, format, orientation, byte range) and `PreviewFormat`;
- `Orientation`, `Size`, `Error`, `ErrorKind` and `Result`.

Rejected:

- **Byte slices only.** The whole file would be read to show its preview:
  the cost a culler on a network share cannot pay.
- **Separate types for slices and readers.** Two code paths to keep in step.
  `Cursor<&[u8]>` covers slices through the one handle, and
  `Preview::byte_range` lets a slice caller borrow a preview's bytes instead
  of copying them.
- **Several crates**, such as one for containers and one for decoding.
  They would turn internal boundaries into public ones before those
  boundaries have settled.

## Consequences

- Callers can rely on getting an `Err`, never a panic, for any file
  ([ADR008](adr008-lints-and-panics.md)); on allocations bounded by
  `Limits`; and on detection by content, so a renamed file opens as what it
  is.
- Showing a preview reads a few kilobytes of structure plus the preview's
  own bytes, whether the file is on disk, on a network share or in memory.
- `RawFile<R>` is `Send` when `R` is, so files can be opened on a thread
  pool. Reading takes `&mut self`; a `Preview` is `Copy`, so a caller copies
  it out before reading its bytes.
- Metadata, raw data and development arrive as new methods and types, and
  new formats as new `Format` variants, without changing what exists.
- Before 1.0, a minor release may still break the API; `#[non_exhaustive]`
  and getters keep most additions from needing to.
- The `unstable` APIs carry no compatibility promise; a program using them
  pins an exact version of raw-files.
- Buffering is raw-files' job: the block cache is tuned on real files, and
  a caller's own buffering only adds a copy.
