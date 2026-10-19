# Changelog

All notable changes to raw-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the version numbers follow [Semantic Versioning](https://semver.org/).
Before 1.0, a minor release (0.2.0, 0.3.0, ...) may change the public API,
and the changelog says what changed; a patch release (0.1.1, ...) never
does.

## [Unreleased]

## [0.2.0] - 2026-10-19

The second release: the basic metadata of Nikon NEF and Sony ARW files,
read next to the previews when a file is opened.

### Added

- `RawFile::metadata`, the metadata of NEF and ARW files: camera make,
  model and serial number, capture time with sub-seconds and offset from
  UTC when recorded, orientation, the exposure (ISO, exposure time,
  f-number, focal length and exposure compensation, as stored), the lens
  (make, model, serial number, and focal-length and aperture range), and
  the sizes of the raw data and of the visible image (on Nikon files, the
  crop area the camera records), with the `Metadata`, `Exposure`, `Lens`,
  `DateTime`, `Rational` and `SignedRational` types; on Nikon bodies whose
  standard tags lack them, the serial number, ISO and lens range come from
  the MakerNote, and the offset from UTC from the camera's time-zone
  setting.
- `DateTime::subsecond_digits`, how many sub-second digits the file
  recorded, so that a time without sub-seconds can be told from one whose
  recorded digits are zeros without formatting them.
- NEF previews checked on 20 more sample files of 18 Nikon bodies, from
  the D1H to the Z6III, and the metadata of all 23 NEF sample files.
- The metadata of the 36 Sony-family sample files checked, as their
  previews were in 0.1.0.
- `Options` compares and hashes, `Limits` hashes, and `Format`,
  `PreviewFormat`, `ErrorKind`, `Orientation` and `Size` order, as the Rust
  API Guidelines ask of such types; the `Debug` form of `RawFile` shows the
  metadata.

## [0.1.0] - 2026-10-14

The first release: the embedded JPEG previews of Nikon NEF and Sony ARW
files.

### Added

- The project is licensed under the MIT license or the Apache License,
  version 2.0, at your option.
- Builds with Rust 1.95 or later.
- `Error`, with an `ErrorKind` category and the byte offset when known,
  and the `Result` alias.
- `Limits`, the caps on what a file can make the library walk or allocate.
- `Orientation`, the EXIF orientation a viewer must apply to a preview, and
  `Size`, a width and height in pixels.
- `Preview`, one embedded preview: where its bytes are in the file, its
  size, format and orientation; and `PreviewFormat`, how its bytes are
  encoded, with a media type and a file extension.
- `Format`, the RAW formats the library knows, with a name, the file
  extensions and a lookup by extension for file pickers.
- `RawFile`, an open RAW file: opened by path, from bytes or from any
  reader that can seek, with its format detected from its content. It
  lists the file's previews smallest first, picks the largest or the
  smallest at least a given size, and reads one, exactly as stored. A
  file of no format the library knows is refused with the `UnknownFormat` error.
- `Options`, to open files with custom `Limits`.
- Reading the embedded JPEG previews of Nikon NEF files: the ones in IFD0
  and the SubIFDs, and the one the MakerNote points at, sized from their
  own frame headers, with IFD0's orientation.
- Reading the embedded JPEG previews of Sony ARW files, and of the SRF, SR2
  and ARQ files of the same family: the ones in IFD0, IFD1 and, on bodies
  writing FileFormat 4.0 and later, the full-size one in IFD2, sized from
  their own frame headers, with IFD0's orientation.
- A DNG is of no known format, whatever camera maker its Make names:
  converters and the phones that write DNG keep the maker's name.

[Unreleased]: https://github.com/kuolas/raw-files/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/kuolas/raw-files/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kuolas/raw-files/releases/tag/v0.1.0
