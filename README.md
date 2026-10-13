# raw-files

A camera RAW library in pure Rust. It extracts the JPEG previews embedded in
Nikon NEF files, with Sony ARW files next and fuller RAW decoding planned.

**Status: early.** Of the list below, the embedded previews work for Nikon
NEF files; the previews of Sony ARW files, metadata and RAW decoding are
planned. The public API is the file handle (`RawFile` and `Options`) with the
preview types (`Preview`, `PreviewFormat`, `Orientation`, `Size`), `Format`,
`Error` and `Limits`. A NEF file opens with the JPEG previews in its IFD0 and
SubIFDs; any other file that can be read opens as `UnknownFormat`. The NEF
previews are checked on a test file laid out like a Nikon Z 9's, with small
stand-in JPEGs; tests on real sample files are still to come. See the
[roadmap](docs/ROADMAP.md) for the plan.

## What it will do

- **Embedded previews** — return the full-size JPEG preview a camera stores
  in every RAW file, without decoding the raw data: the fast path for photo
  viewers and culling tools.
- **Metadata** — orientation, capture date, camera and image dimensions.
- **RAW decoding** — the sensor data itself, later.
- **Safe on untrusted files** — safe Rust only, and a malformed file returns
  an error rather than crashing the program that opened it.

Pure Rust means no C or C++ library to build, link or ship, on macOS, Linux
and Windows alike.

## Build and test

There are no releases yet. To build from source on macOS, Linux or Windows
you need Rust 1.95 or later:

```sh
cargo build
cargo test
```

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
