# ADR009: The shape of the public API

- Status: Accepted
- Date: 2026-10-11
- Revision: 5, 2026-11-16
- Issue: #119; the handle's name #107; reads through `&self` #89

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

A program also builds values without a file, for its tests: a time, a
fraction, a location, a focus point. Their constructors check their
arguments. They first returned `None` for a refused value, which does
not say which rule refused it. `Focus::with_size` and
`FocusPoints::with_point` each have three rules, so a test that got
`None` had to work out which one it broke.

## Decision

We will expose **one handle, `CameraFile<R>`, generic over any
`R: Read + Seek`**. It is named for what it opens: a camera file, a raw
file or a JPEG ([ADR014](adr014-jpeg-files-as-a-format-of-their-own.md)).

- `CameraFile::open(path)` returns a `CameraFile<File>`. Small reads go
  through a block cache inside raw-files, so walking a file's scattered TIFF
  directories costs one read per block. Callers do not wrap the file in a
  `BufReader`, which discards its buffer on every seek.
- Bytes already in memory, including a memory-mapped file, are opened with
  `CameraFile::from_bytes`, a `CameraFile<Cursor<&[u8]>>`: the same handle
  and the same code path.
- Reading takes `&self`. A read names its own offset and length, those
  of an `Entry` or a `Preview`, and seeks before it reads, so it
  changes nothing a caller can see, and a program reads a value or a
  preview while it goes through the list it came from. The reader sits
  behind a `Mutex` inside the handle: threads that share a handle read
  in turn, and a value an entry holds itself is read without the lock.
  A lock that a panicking read left poisoned is taken all the same,
  since a read that fails leaves the block cache as it was.
- Opening reads the file's structure, and its metadata as
  [ADR011](adr011-metadata-from-standard-tags.md) decides. Each other
  capability, such as the previews or the raw data, then succeeds or fails
  on its own, so a file whose raw data is not supported still gives its
  previews.
- The format is detected from the file's content, never from its name.
  Format modules produce format-neutral public types: supporting a new
  format adds a `Format` variant and nothing else. There is no public
  decoder trait; dispatch stays inside the crate.
- No third-party types in the public API. Conversions to other crates' types
  can come later behind optional features.
- An error from reading a file is one opaque `Error` with a
  `#[non_exhaustive]` `ErrorKind` (I/O, unknown format, unsupported,
  malformed, limit exceeded) and, when known, the byte offset in the file.
  Something a file does not contain is `None`, not an error.
- A function that refuses its arguments returns `Result<_, ValueError>`.
  `ValueError` is a small `Copy` enum with one variant per rule, so a
  caller learns which rule refused the value. When several rules are
  broken, the first in the function's documented order is returned.
- A lookup, such as `Format::from_extension`, and a getter return
  `Option`: no value there is an answer, not a refusal.
- Enums that may gain variants and records that may gain fields are
  `#[non_exhaustive]`, and structs expose getters; only small fixed value
  types such as `Size` have public fields.
- A record whose every value is optional, such as `Metadata`,
  `Exposure` or `Lens`, starts from its empty `default()`, and a
  `with_…` method sets each value. It has no separate builder type.
- A record that may gain a value that cannot be copied is `Clone` and
  not `Copy`, even when every value it holds can be copied. Such a value
  would otherwise break callers. `Exposure`, `CameraSettings` and
  `Options` are such records. A record of plain numbers, such as
  `Limits`, `Preview` or `Focus`, is `Copy`.
- A value that keeps what the file stored is equal to another only
  when both were stored the same way. `Rational` and `Latitude` are
  such values. Such a value has no `Ord`, since an order by value would
  disagree with that equality. A caller sorts by a method that gives the
  value, such as `Rational::to_f64`.
- `DateTime` has no order, because local times do not order by their
  fields. 10:00 at an offset of `+02:00` came before 09:00 at `+00:00`.
  A time without an offset cannot be placed against either. `Direction`
  has none because a bearing turns round to its start.
- `Value` has no `Eq` or `Hash`. Its FLOAT and DOUBLE values are
  floating-point numbers, and a NaN is not equal to itself.
- A value with no empty state that means anything, such as a `Size`, a
  `Point`, a `Flash` or a `FineTune`, has no `Default`.
- Every allocation whose size comes from the file is capped by `Limits`,
  whose defaults are safe for untrusted input.
- raw-files stays **one crate**. The raw-data and development APIs arrive
  behind an `unstable` Cargo feature, outside the compatibility promise,
  until an application has used them.

The first release has this surface:

- `CameraFile`: `open`, `from_bytes`, `from_reader`, `format`, `previews`,
  `largest_preview`, `preview_at_least`, `read_preview`,
  `read_preview_into`, `into_inner`;
- `Options` and `Limits`, to open with other limits;
- `Format`, with a name and file extensions for file pickers;
- `Preview` (size, format, orientation, byte range) and `PreviewFormat`;
- `Orientation`, `Size`, `Error`, `ErrorKind` and `Result`.

Rejected:

- **Byte slices only.** The whole file would be read to show its preview:
  the cost a culler on a network share cannot pay.
- **The name `RawFile`**, which the handle had while raw-files read only
  raw files. It also opens JPEG files, for which `Format::is_raw` is
  false, so "raw" had two senses in one API. The rename costs a caller
  an edit wherever the type is written, and gives a name that is true of
  every file the handle opens.
- **`RawFile` kept as a deprecated alias.** One type would have two
  names until it was removed, and the changelog already says what a
  caller changes.
- **Reads through `&mut self`**, as the handle had them before. The
  lists of entries and previews borrow the handle, so a program could
  not read from one while it went through it, and every caller copied
  the list first.
- **A public trait of reads at an offset as the bound**, whose reads
  take `&self`, as std's `FileExt::read_at`, parquet's `ChunkReader`
  and positioned-io's `ReadAt` do. Two threads could read one handle at
  once, which no program asks for, at the cost of a public trait and
  code for each platform, and the block cache would still need a lock.
  Those three APIs' docs were read for their signatures; none of their
  code was.
- **An iterator that yields each entry with its value.** It serves only
  a program that reads every value, and leaves the previews as they
  were.
- **`entries()` returning an owned list**, shared and cheap to clone.
  The getter would give an owned value for a reason no caller sees.
- **Separate types for slices and readers.** Two code paths to keep in step.
  `Cursor<&[u8]>` covers slices through the one handle, and
  `Preview::byte_range` lets a slice caller borrow a preview's bytes instead
  of copying them.
- **Several crates**, such as one for containers and one for decoding.
  They would turn internal boundaries into public ones before those
  boundaries have settled.
- **`Option` from every constructor**, as before. It cannot say which
  rule refused a value, and after 1.0 a reason could not be added.
- **`Result` for the focus types alone.** `?` cannot mix `Option` and
  `Result` in one chain, and the fractions feed every other type.
- **An error type per family of values.** `?` could not mix them in one
  chain either.
- **`Error` with a new `ErrorKind`.** Each refusal would allocate, and
  its rules could be told apart only by their text.
- **A conversion from `ValueError` into `Error`.** `Error` would then
  mean something that no reading of a file returns. A function that
  opens a file and builds values returns `Box<dyn std::error::Error>`.
- **List methods that borrow the list**, so that a refused point keeps
  the list built so far. They would leave the `with_…` convention, and
  the error now tells a test's author what to fix.
- **A separate builder type**, as the builder pattern of the Rust API
  Guidelines has it (C-BUILDER). Every value of such a record is
  optional, so the empty record is already a valid start for every
  chain. A builder would be a second type with the same methods.

## Consequences

- Callers can rely on getting an `Err`, never a panic, for any file
  ([ADR008](adr008-lints-and-panics.md)); on allocations bounded by
  `Limits`; and on detection by content, so a renamed file opens as what it
  is.
- Showing a preview reads the file's structure, a few blocks of 64 KiB,
  plus the preview's own bytes, whether the file is on disk, on a network
  share or in memory.
- `CameraFile<R>` is `Send` when `R` is, and `Sync` when `R` is `Send`,
  so files can be opened and read on a thread pool. Threads that share
  one handle read from the file one at a time; a program that wants two
  reads at once opens the file twice.
- Raw data and development arrive as new methods and types, as metadata
  did, and new formats as new `Format` variants, without changing what
  exists.
- Before 1.0, a minor release may still break the API; `#[non_exhaustive]`
  and getters keep most additions from needing to.
- The `unstable` APIs carry no compatibility promise; a program using them
  pins an exact version of raw-files.
- Buffering is raw-files' job: the block cache is tuned on real files, and
  a caller's own buffering only adds a copy.
