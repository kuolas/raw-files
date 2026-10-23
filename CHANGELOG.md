# Changelog

All notable changes to raw-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the version numbers follow [Semantic Versioning](https://semver.org/).
Before 1.0, a minor release (0.2.0, 0.3.0, ...) may change the public API,
and the changelog says what changed; a patch release (0.1.1, ...) never
does.

## [Unreleased]

### Added

- JPEG files open, as `Format::Jpeg`, detected by their content like
  every other format: the JPEG a camera writes beside its raw file or
  instead of it, or an editor exports. Their previews are the file
  itself, the thumbnail of the Exif segment and the images the
  Multi-Picture Format (MPF) list holds after the main one, each sized
  from its own frame header, with the orientation the Exif segment
  records. Their metadata comes from the Exif segment, read as in a NEF
  or an ARW file: the camera's make and model, its serial number, the
  capture time, the exposure, the lens and the orientation. The image
  size is the size of the picture, and there is no raw size. A JPEG
  whose Exif segment is missing or damaged still opens, with the file
  and its MPF images as previews and the image size as its only
  metadata. A Nikon camera's JPEG also gives what its MakerNote
  records, read as in a NEF: the serial number, the ISO, the lens's
  range and the capture time's offset from UTC where its Exif segment
  records none, the small JPEG of the MakerNote's PreviewIFD as a
  preview on the bodies that write one, such as the D70, and the focus
  point. On a crop mode (DX, 1:1, 16:9), whose
  JPEG records the point in the camera's full output, the point is
  placed on the JPEG's own picture. A Sony camera's JPEG gives the focus
  point its MakerNote records, read as in an ARW file, with the size of
  the focus area on the bodies that record one; the DSLR-A100's JPEG
  records none. A JPEG's focus point is given only when its frame has the
  picture's aspect ratio, so an export cut to another shape gives none,
  except one cut from a Nikon crop-mode JPEG, whose point is placed on
  the export's pixels and may be misplaced.
- `Format::is_raw`, whether a format's files hold a camera's raw sensor
  data, true for NEF and ARW and false for JPEG, so that a program can
  tell raw files from other images without matching every `Format`.
- JPEG previews and metadata checked on eight camera-original JPEGs from
  Wikimedia Commons, of five Sony bodies, two Nikon bodies and a Canon,
  under CC0, a public-domain dedication or CC BY, and on each the focus
  point it gives, or that it gives none; `scripts/fetch-samples.sh jpeg`
  downloads them.

### Changed

- `Format::from_extension("jpg")` and `Format::from_extension("jpeg")`
  return `Some(Format::Jpeg)` where they returned `None`, and
  `Format::all` lists `Format::Jpeg`. A program that took every file whose
  extension `from_extension` knows for a raw file should ask
  `Format::is_raw` as well.

## [0.3.0] - 2026-10-21

The third release: where the camera focused, read from Nikon NEF and Sony
ARW files next to the previews and the metadata, and public constructors so
that a program can build every metadata value in its own tests.

### Added

- `Metadata::focus`, where the camera focused, as it recorded it: the
  centre of the focus area, the area's size when the file records one,
  the frame the two are measured in, and whether the camera confirmed
  focus, with the `Focus` and `Point` types; `Focus::new`, `with_size`
  and `with_in_focus` build one, and `Focus::scaled_to` places it on any
  preview. Nikon mirrorless bodies and Nikon DSLRs in live view give a
  focus point, read from the MakerNote's AFInfo2 block and placed inside
  the cropped picture on a crop mode, and so do Sony bodies from the
  ILCE-7RM2 on, read from the MakerNote's FocusLocation, with the area's
  size from the ILCE-7SM3 on and nothing said about whether focus was
  confirmed. A file that records no position gives `None`: a Nikon DSLR
  shot through the viewfinder or a Sony body before the ILCE-7RM2, which
  record at most an autofocus point's number or a mask of points, a
  manual-focus shot, an AFInfo2 version raw-files does not know, and a
  Nikon crop whose point cannot be placed on the cropped picture.
- NEF previews and metadata checked on ten more CC0 Nikon sample files,
  of the D5 and the D5600 in live view, of the Z5II, Z50II and Z6III in
  their 1:1 and 16:9 crop modes, of a Z fc shot in manual focus and of a
  rotated Z 30 shot, 33 NEF files in all, and the focus point on every
  one of them.
- Public constructors for `DateTime` (`new`, `with_subseconds`,
  `with_offset_minutes`), `Rational` and `SignedRational` (`new`), a
  builder for `Lens` (`Lens::builder` and its `LensBuilder`), and `with_…`
  builders on `Metadata` and `Exposure`, so that a program can build the
  values raw-files reads in its own tests; the constructors give `None`
  for a date or time that does not exist, a fraction with a zero
  denominator or a lens with no value, and the builders store what they
  are given.
- `Size` and `Point` print as text (`Display`), `8256x5504` and
  `(4905, 2461)`, as the Rust API Guidelines ask of such types.

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

[Unreleased]: https://github.com/kuolas/raw-files/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/kuolas/raw-files/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kuolas/raw-files/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kuolas/raw-files/releases/tag/v0.1.0
