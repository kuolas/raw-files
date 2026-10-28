# Changelog

All notable changes to raw-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the version numbers follow [Semantic Versioning](https://semver.org/).
Before 1.0, a minor release (0.2.0, 0.3.0, ...) may change the public API,
and the changelog says what changed; a patch release (0.1.1, ...) never
does.

## [Unreleased]

### Added

- `Focus::origin` says where a focus point came from, as a `FocusOrigin`:
  `Recorded`, a position the camera measured and wrote, or `Table`, a
  point the camera named by number that raw-files placed from its table
  of the body's autofocus points.
  `Focus::new` gives `Recorded`, `Focus::with_origin` sets another and
  `Focus::scaled_to` keeps it; `FocusOrigin` has a `name` that `Display`
  writes, "Recorded" and "Placed from a table", and orders as its
  variants are listed.
- `Metadata::focus_points` gives the autofocus points the camera used,
  as a `FocusPoints`: every point in one frame, each a `FocusPoint` with
  its number when the file gives one, its centre, its area when recorded
  and whether focus was confirmed when the file says; which of them is
  the primary, which `Metadata::focus` gives as a `Focus`; the area mode,
  a `FocusAreaMode`, "Single point", "Dynamic area", "Auto area" or
  "Tracking", with a `name` that `Display` writes, ordered as its
  variants are listed; and where the points came from.
  `FocusPoints::scaled_to` places every point on a preview as
  `Focus::scaled_to` places one. A program's tests build a list with
  `FocusPoints::new`, `with_point`, `with_primary` and `with_area_mode`,
  each point with `FocusPoint::new` and its `with_number`, `with_size`
  and `with_in_focus`, or a list of one point from a `Focus` with `From`,
  and set it with `Metadata::with_focus_points`; `Metadata::with_focus`
  sets a list of that one point.
- NEF, ARW and JPEG files give the position the camera recorded as a
  list of one point, the primary, with no number. Nikon files give the
  area mode from AFInfo2's area-mode byte, on every version read, for
  the codes that frames shot in a known mode confirmed, under the
  detection method the file records: the DSLR codes 0 single point, 5
  dynamic area and 8 auto area under phase detection, through the
  viewfinder; the mirrorless codes 193 single point and 197 auto area
  under any detection method, and 207 tracking under contrast or hybrid
  detection. Any other code, or a DSLR code in live view, gives `None`.
  Sony files give no area mode.
- `Metadata::focus_layout` gives the layout of the body's autofocus
  points, as a `FocusLayout`: the Model it is for, the frame its points
  are measured in, the body's full-size output, and every point, each a
  `LayoutPoint` with the camera's number for it, the centre of its box
  and the box's size; `FocusLayout::point` finds a point by its number,
  so that a viewer can draw the grid and mark on it the points the
  camera used. `FocusLayout::new` builds one from a `static` array of
  points, which `LayoutPoint::new` can fill since it is `const`, and
  refuses a frame with a zero side, no points, a number of 0 or one
  given twice, and a point whose centre or box does not lie on the
  frame by the rules of `Focus::new` and `Focus::with_size`; a program's
  tests set it with `Metadata::with_focus_layout`. The readers give a
  layout only from a table of the body's points, and `None` for a body
  whose table is not in raw-files.
- NEF and JPEG files of the Nikon D600 give its layout of 39 autofocus
  points (`Metadata::focus_layout`), whatever the shot: through the
  viewfinder, in live view, in manual focus or on a crop. The layout
  depends on the Model alone, so a file whose MakerNote is missing or
  cannot be read, such as an editor's re-save, gives it too. The table was
  measured by the project on frames shot with a D600 and read in Nikon's
  NX Studio, as the NEF notes record it.
- NEF and JPEG files of the Nikon D600 shot through the viewfinder give
  the autofocus point the camera names by number, placed from that
  table. `Metadata::focus` gives the point's centre and its box of
  164x164 in the 6016x4016 frame, with the origin `Table` and nothing
  said of whether focus was confirmed; `Metadata::focus_points` gives
  every point the file's record of the points used, one bit per point,
  names, and the primary, each with its number, in number order, with
  the area mode: single point, dynamic area or auto area. On the DX crop
  the points are placed in the crop's window, 3936x2624, the size of the
  NEF's full-size preview and of its JPEG; a crop-mode JPEG at a smaller
  size gives none.
  A Medium JPEG, 4512x3008, gives none either: its shape, exactly 3:2,
  is further from the 6016x4016 frame than the one pixel of rounding by
  which a JPEG's point must fit its picture; a Small JPEG, 3008x2008,
  half the frame, keeps its point. A shot through the viewfinder of any
  other body gives no focus point, since raw-files has no table of its
  points.

## [0.5.0] - 2026-10-26

The fifth release: the camera's settings beside the exposure, for a
viewer's information panel, read from the Exif tags and from the plain
tags of Nikon's and Sony's MakerNotes in NEF, ARW and JPEG files, and
the focal length in 35 mm terms and the software that wrote the file.
Nothing in 0.4.0's API was removed or changed in signature, though
with a very low cap on its directories a raw file may now be refused,
and a JPEG lose its Exif metadata (see Changed).

### Added

- `Metadata::camera_settings` gives a `CameraSettings`: how the camera
  was set for the shot, each value optional. It holds the white balance
  (`WhiteBalance`: its mode, its colour temperature in kelvin and its
  fine-tuning, `FineTune`), the exposure program, the metering mode, the
  flash (`Flash`: whether it fired and its mode), the shutter used,
  Sony's electronic front curtain setting, the image area, the raw image
  size and the colour space. The enums `WhiteBalanceMode`,
  `ExposureProgram`, `MeteringMode`, `FlashMode`, `Shutter`, `ImageArea`,
  `RawImageSize` and `ColorSpace` name each setting's choices in words
  that fit any maker, with a `name` that `Display` writes, and order as
  their variants are listed, so a program can use them as sorted keys. A
  program's tests build all of these through their `with_…` methods,
  `Metadata::with_camera_settings`, `FineTune::new` and `Flash::new`.
- NEF, ARW and JPEG files give the settings their Exif tags record: the
  exposure program, the metering mode, the flash, the white balance mode,
  from Exif's auto or manual and the kind of light a manual one was set
  for, and the colour space, sRGB or, from the `R03` index of a DCF
  option file, Adobe RGB. A code Exif does not define, or its "not
  defined" and "unknown", gives `None`.
- NEF files and the JPEGs of Nikon cameras give the rest from Nikon's
  MakerNote: the white balance setting, such as a preset or a chosen
  colour temperature, in place of Exif's when the MakerNote names one;
  the colour temperature of the white balance the camera applied, which
  the D5, the D850, the D780 and the Z bodies record, or, where that is
  missing or 0, the number of a chosen colour temperature, such as
  `3230K`, the one temperature older bodies record; the
  fine-tuning; the shutter used; the image area; the raw image size, in a
  raw file only; and the colour space, when Exif records none, as in
  every Nikon raw file checked. The single fine-tuning value of the D1
  family, the D70, the D100 and the D200 gives `None`, since no file
  shows its axis.
- ARW files, with the SRF, SR2 and ARQ files of the family, and the
  JPEGs of Sony cameras give settings from Sony's MakerNote too: the
  white balance setting, such as custom, or underwater, which Exif calls
  auto, in place of Exif's when the MakerNote names one; the colour
  temperature under a chosen colour temperature or a custom white
  balance, the only settings under which the files checked record one;
  the fine-tuning; and the electronic front curtain setting, the menu
  setting, never the shutter used. The shutter used, the image area and
  the raw image size are `None` on Sony files: Sony records the first two
  only in enciphered blocks, and gives an APS-C raw the raw size code of
  a medium one. The DSLR-A100, A200, A700 and A900, the SLT-A55V, the
  NEX-5, the DSC-F828 and the DSC-R1 record no fine-tuning, and the
  white balance mode of the DSLR-A100, the DSC-F828 and the DSC-R1 is
  Exif's.
- Every fine-tuning is signed as its getters are named, whatever the
  maker: `FineTune::amber_blue` is positive towards blue and
  `FineTune::green_magenta` positive towards magenta, so a viewer can
  show or compare two files' shifts without knowing how each maker signs
  them. Every colour temperature is read from 1 to 99,999 K: a recorded
  0, which means none was recorded, and a larger number give `None`.
- `Exposure::focal_length_in_35mm_film` and `Metadata::software`, read
  from NEF, ARW and JPEG files. The first is the focal length in 35 mm
  terms the camera recorded, in whole millimetres, from Exif's
  FocalLengthIn35mmFilm: the focal length that would give the same field
  of view on a full-frame camera, so that pictures from bodies with
  different sensor sizes compare; a recorded 0, which a camera writes when
  it does not know it, gives `None`. The second is IFD0's Software,
  cleaned of padding as every text value: the camera's firmware version,
  such as `Ver.01.00` or `ILCE-7M4 v2.00`, or the name of a program that
  rewrote the file. `Exposure::with_focal_length_in_35mm_film` and
  `Metadata::with_software` build them for a program's tests.

### Changed

- The Exif IFD's Interoperability IFD, read for the colour space, counts
  as one more directory under `Limits::max_ifds`. A file that holds one,
  as most Sony-family sample files do, needs one directory more than
  under 0.4.0, so a cap set just high enough for it then fails a raw
  file with `LimitExceeded`. A JPEG still opens, as for any cap crossed
  in its Exif segment, but without what that segment holds: its Exif
  metadata, the MakerNote's settings and focus point, and the Exif
  thumbnail. The default cap, 256, is far above what any file checked
  needs.

## [0.4.0] - 2026-10-23

The fourth release: JPEG files, as cameras and editors write them, open as
a format of their own, with their previews, their Exif metadata and, from
Nikon and Sony cameras, the focus point; `Format::is_raw` tells raw formats
from the others; the headerless MakerNote of the Nikon D1 family is read.
Nothing in 0.3.0's API was removed or changed in signature, though opening
a JPEG now succeeds (see Changed).

### Added

- JPEG files open, as `Format::Jpeg`: the pictures a camera writes beside
  its raw files or instead of them, and the JPEGs an editor exports,
  recognised by their content like every other format. A program gets
  from one what it gets from a raw file. Its previews are the file itself
  and the smaller images it holds: the Exif thumbnail, the images of its
  Multi-Picture Format (MPF) list and, from Nikon bodies such as the D70,
  one more small JPEG. Its metadata is what its Exif segment records,
  with the picture's size and no raw size. In a Nikon or a Sony camera's
  JPEG, the maker's MakerNote adds the focus point, and in a Nikon
  camera's what Exif lacks, such as the serial number, the ISO, the
  lens's range and the offset from UTC; on a Nikon crop mode (DX, 1:1,
  16:9) the point is placed on the picture. A focus point is given only
  when its frame has the picture's shape: an export cut to another shape
  gives none, except one cut from a Nikon crop-mode JPEG, whose point may
  then be misplaced. A JPEG whose Exif segment is missing or cannot be
  read still opens, with the file itself and its MPF images as previews
  and its image size.
- NEF files of the Nikon D1 family give the ISO and the lens's range
  where their Exif tags lack them, and those of the D1H and the D1X a
  preview, their only JPEG, where they gave none. A JPEG file whose Nikon
  MakerNote has the D1 family's layout is read the same way: on the early
  Coolpix JPEGs checked, from Wikimedia Commons, that gives nothing beyond
  the Exif values, and the small JPEG some of them hold is not given, as
  it lies outside the MakerNote.
- `Format::is_raw`: true for NEF and ARW and false for JPEG, so that a
  program can tell raw files from other pictures without matching every
  `Format`.
- Eight camera JPEGs from Wikimedia Commons, of five Sony bodies, two
  Nikon bodies and a Canon, under CC0, a public-domain dedication or
  CC BY, check the JPEG reader: every preview, the metadata, and the
  focus point or that there is none. `scripts/fetch-samples.sh jpeg`
  downloads them.

### Changed

- `Format::from_extension` returns `Some(Format::Jpeg)` for `jpg` and
  `jpeg` where it returned `None`, and `Format::all` lists `Format::Jpeg`.
  A program that took every file whose extension `from_extension` knows
  for a raw file now takes JPEGs too, and should ask `Format::is_raw` as
  well.
- Opening a JPEG file (`RawFile::open`, `from_bytes` and `from_reader`,
  and the same on `Options`) succeeds, as `Format::Jpeg`, where 0.3.0
  returned the `UnknownFormat` error. A program that took every file that
  opens for a raw file, or that matched `format()` assuming NEF or ARW,
  should ask `Format::is_raw`.

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

[Unreleased]: https://github.com/kuolas/raw-files/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/kuolas/raw-files/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/kuolas/raw-files/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/kuolas/raw-files/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kuolas/raw-files/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kuolas/raw-files/releases/tag/v0.1.0
