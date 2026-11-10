# Changelog

All notable changes to raw-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the version numbers follow [Semantic Versioning](https://semver.org/).
Before 1.0, a minor release (0.2.0, 0.3.0, ...) may change the public API,
and the changelog says what changed; a patch release (0.1.1, ...) never
does.

## [Unreleased]

### Added

- `Metadata::authorship` gives who made the picture and what they wrote
  about it, in an `Authorship`: the artist, the copyright notice and the
  description from IFD0's Artist, Copyright and ImageDescription, and
  the photographer, the editor, the title and the comment from the Exif
  IFD's Photographer, ImageEditor, ImageTitle and UserComment, in NEF,
  ARW and JPEG files alike. Each text is given as the file records it,
  cleaned as every text is and nothing hidden; a blank field, as a
  camera writes one its owner has not set, is `None`. The description
  and the comment are free text, with no 255-byte limit and their line
  breaks kept. `SONY DSC`, which older Sony bodies write as the
  description of every picture, gives none. The comment is read only
  under UserComment's `ASCII` character code. Sony's copies of IFD0's
  texts in IFD1 and IFD2 are not read, nor anything in a MakerNote.
  `Metadata::with_authorship` and the `with_…` methods of `Authorship`
  build one for a program's tests.
- `Camera::owner_name` gives the name of the camera's owner, the Exif
  IFD's CameraOwnerName, of type ASCII or UTF-8, cleaned as every text
  is and held to 255 bytes, `None` when blank; `Camera::with_owner_name`
  sets it. It names whoever owns the body, not whoever took the
  picture.
- `Metadata::location` gives where the picture was taken, in a
  `Location`, read from the GPS IFD of NEF, ARW and JPEG files alike: a
  position, a `Latitude` and a `Longitude`, each the degrees, minutes
  and seconds the file records as fractions with its hemisphere, and
  `to_degrees` for signed decimal degrees; and, when the file records
  them, the `Altitude`, above or below its reference without naming the
  surface, the GPS time in UTC with the fraction of a second the file
  records, the image direction, a `Direction` from true or magnetic
  `North`, and the map datum. A GPS IFD without a sound position gives
  no location: both reference letters and both coordinates must name a
  place, so a GPS IFD of only its version, a Sony file without a fix and
  a Coolpix file's blank tags give none. Each other part needs both its
  tags, and a damaged one costs only itself; a GPS time needs a date and
  time that exist. Everything is given as recorded, nothing rounded or
  hidden. GPSVersionID, GPSStatus and the other GPS tags are not read;
  the view of every entry gives them. `Metadata::with_location`,
  `Location::new` and its `with_…` methods, and the checked
  constructors of the coordinates, the altitude and the direction build
  one for a program's tests.

### Changed

- The camera's Make and Model, the Software, and the lens's LensMake
  and LensModel are read from a value of type UTF-8 as from one of type
  ASCII, since Exif 3.1 gives each as "ASCII or UTF-8": the same
  cleaning, and the same 255 bytes at most, counted as stored. So a NEF
  or ARW whose Make is of type UTF-8 is now claimed by its reader, where
  it was refused as of no known format, and a camera JPEG's MakerNote is
  read. The serial numbers and the
  capture time, which Exif gives as ASCII alone, are read only from
  type ASCII, as is every text of a maker's MakerNote. No file checked
  writes the UTF-8 type, so no value read from one changes.

## [0.8.0] - 2026-11-09

The eighth release, the first step towards full metadata: every entry
of the TIFF directories raw-files walks in a NEF, ARW or camera JPEG,
a Nikon or Sony MakerNote's among them, as the file records it, with
the name a published standard gives each standard tag, and any
entry's value read when asked for. Nothing in 0.7.0's API was removed
or changed in signature, though opening a file now also reads its GPS
IFD, which counts towards the cap on directories, and, in an ARW, the
table of Sony's private directory (see Changed).

### Added

- `RawFile::entries` gives every entry of every TIFF directory of the
  file that raw-files walks, as the file records it, in an `Entries`:
  IFD0 and the directories chained after it, the SubIFDs, the Exif, GPS
  and Interoperability IFDs, a Nikon or Sony MakerNote's IFD in the
  layouts raw-files reads, in a raw file or a JPEG, with Nikon's preview
  IFD, an ARW's private directory, SR2Private, and a JPEG's
  Multi-Picture Format index. A record the maker enciphers is an entry
  like any other, whose bytes are given as stored, neither marked nor
  deciphered; no standard names a maker's tags, so those entries have
  no name.
  Each `Entry` gives its `Directory`, its tag, its `FieldType`, its count
  and where its value lies; `Entries` lists them directory by directory
  in the order they were found, looks one up by directory and tag, the
  first when a tag is repeated, and counts per directory the entries
  left out because they could not be checked when the file was opened:
  an unknown type, a length that overflows, or a value outside the file
  or the part of it the directory was read in. Reading the entries reads
  nothing from the file.
  `Directory` writes the name a viewer shows through `Display`, such as
  `IFD0` or `MakerNote 0x0011`, and `FieldType` has a `name` that
  `Display` writes, such as `SHORT`, spelled as the standards write it.
- `RawFile::read_value` reads an entry's value, as a `Value` with one
  variant per field type, its numbers in the byte order of the entry's
  directory, fractions as numerator and denominator pairs with a zero
  denominator kept, and text as its bytes, which `Value::text` cleans as
  the metadata's text is cleaned, without its 255-byte limit. A value is
  read from the file each time it is asked for.
- `Entry::name` gives the name a published standard gives the entry's
  tag in its kind of directory, such as `DateTimeOriginal` for tag
  0x9003 of the Exif IFD: TIFF 6.0's, Exif 3.1's, DCF 2.0's for the
  Interoperability IFD, CIPA DC-007's for the Multi-Picture index and
  the DNG specification's for the four DNG tags raw-files reads, 218 in
  all, spelled as each document spells them, each recorded with its
  document and page in `docs/formats/tag-names.md`. A tag no such
  document names in that kind of directory has none, as an Exif tag
  written in IFD0 has none there.
- `Limits::max_value_bytes`, set with `with_max_value_bytes`, bounds the
  bytes one entry's value may take, 1 MiB by default, the bound the
  metadata's values already have; a longer value fails only its own
  read, with `LimitExceeded`.

### Changed

- Opening a file now also reads IFD0's GPS IFD, after the chain of
  directories, so that its entries can be listed; an IFD it shares with
  the chain stays there. On most older NEFs this reads one more 64 KiB
  block. The GPS IFD counts towards `Limits::max_ifds`, so with a cap
  set at a file's count of directories under 0.7.0, a raw file may now
  be refused, and a JPEG lose what its Exif segment holds.
- Opening an ARW now also reads the table of Sony's private directory,
  SR2Private, at the offset IFD0's DNGPrivateData holds, so that its
  entries can be listed; its values are read only when asked for. On
  89 of the 245 public ARW-family files measured this reads one more
  64 KiB block.
- An entry of Exif 3.0's UTF-8 type (129) is kept, where it was left out
  as one of an unknown type. The metadata still reads text only from
  ASCII entries.

## [0.7.0] - 2026-11-03

The seventh release: the camera's maker and the name the body is sold
under, beside the Make and Model texts the file records, from
raw-files' own table of names; the camera's and the lens's values as
plain values, as the exposure's are; and a D600 Medium JPEG's
autofocus points, placed in the window of the frame its picture shows,
and a D600's 3D-tracking area mode named. This changes some of 0.6.0's
signatures: the camera's values move from `Metadata` to a `Camera`,
`Metadata::lens` gives a `&Lens`, the recorded texts are named
`recorded_make` and `recorded_model`, and `LensBuilder` is gone (see
Changed and Removed).

### Added

- `Camera::make` gives the camera's maker as a `Make`, `Nikon`, `Sony`,
  `Hasselblad` or `Canon`, recognised from the whole Make text the file
  records, case included: `NIKON CORPORATION` and `NIKON` are Nikon,
  `SONY` Sony, `HASSELBLAD` Hasselblad and `Canon` Canon, and any other
  text gives `None`. `Make` has a `name` that `Display` writes, spelled
  as each maker writes its own name, and orders as its variants are
  listed. `make()` is read from the recorded Make each time it is asked
  for, with no setter of its own, so a program's tests set the recorded
  text with `with_recorded_make`.
- `Camera::model` gives the name the maker sells the body under, without
  the brand, spelled as the maker writes it, such as `Z6III` for a file
  whose Model is `NIKON Z6_3` and `α7 II` for `ILCE-7M2`. It comes from
  raw-files' table of the bodies the notes on camera names list with
  their evidence: the Nikon D600, Z 9, Z 6, Z 30, Z f, Z fc, Z50II, Z5II
  and Z6III, and the Sony α1, α1 II, α6000, α6700, α7 II, α7 III, α7 IV,
  α7 V, α7R II, α7R III, α7S, α7S III, α9 and α9 III. Any other body
  gives `None`, never the recorded text, which `recorded_model` still
  gives. `model()` is read from the recorded make and Model each time it
  is asked for, with no setter of its own.

### Changed

- `Metadata::lens` gives a `&Lens`, as `Metadata::exposure` gives a
  `&Exposure`, instead of an `Option<&Lens>`. A file that describes no
  lens gives a lens whose every value is `None`, as `Lens::default()`
  is, so 0.6.0's `metadata.lens().and_then(|lens| lens.model())` is
  `metadata.lens().recorded_model()` now, and an `if let Some(lens)` on
  it fails to compile rather than change meaning. A program that asked
  whether the file describes a lens at all, with 0.6.0's
  `metadata.lens().is_some()`, compares with the empty lens:
  `*metadata.lens() != Lens::default()`.
- `Lens` is built as `Exposure` is, from the empty `Lens::default()`
  through `with_recorded_make`, `with_recorded_model`,
  `with_serial_number`, `with_min_focal_length`, `with_max_focal_length`,
  `with_f_number_at_min_focal_length` and
  `with_f_number_at_max_focal_length`, each storing its value as given.
- `Metadata::make`, `Metadata::model` and `Metadata::serial_number` move
  to a `Camera`, which `Metadata::camera` gives as `Metadata::lens` gives
  a `Lens`: they are `Camera::recorded_make`, `Camera::recorded_model`
  and `Camera::serial_number` there. The first two give the text as the
  file records it, cleaned as before, so 0.6.0's `metadata.model()` is
  `metadata.camera().recorded_model()` now. A program's tests build a
  camera from `Camera::default()` through `with_recorded_make`,
  `with_recorded_model` and `with_serial_number` and set it with
  `Metadata::with_camera`, in place of 0.6.0's `Metadata::with_make`,
  `Metadata::with_model` and `Metadata::with_serial_number`.
- `FocusLayout::model` is `FocusLayout::recorded_model`: its text is the
  Model as the file records it, such as `NIKON D600`, as
  `Camera::recorded_model` gives it.
- `Lens::make` and `Lens::model` are `Lens::recorded_make` and
  `Lens::recorded_model`, with their setters `with_recorded_make` and
  `with_recorded_model`: they give the text as the file records it, as
  `Camera`'s do, and leave the plain words for the lens's name that a
  table of lenses is to give. 0.6.0's `lens.model()` is
  `lens.recorded_model()`.
- Nikon files name the DSLR area-mode code 9 under phase detection,
  through the viewfinder, as `FocusAreaMode::Tracking`: D600 frames shot
  in 3D-tracking write it. `FocusPoints::area_mode` gave `None` for such
  a file before.

### Removed

- `LensBuilder` and `Lens::builder`: a program's tests start from
  `Lens::default()` and write `.with_recorded_model(…)` for `.model(…)`,
  and so on for every value, with no `build()` call and no `?` after it.

### Fixed

- A Nikon D600 JPEG at the Medium size, 4512x3008, shot through the
  viewfinder ([#62](https://github.com/kuolas/raw-files/issues/62)),
  gives the autofocus points it names, placed from the
  D600's table, where it gave none: its picture is not the whole
  6016x4016 frame scaled, but the 6010x4006 window centred in it, which
  the table now holds for that size, as measured on RAW+JPEG pairs. The
  frame of `Metadata::focus` and `Metadata::focus_points` is that
  window, 6010x4006, every centre moved back by its origin, (3, 5), so
  that a point scaled from that frame onto the JPEG lands where the
  camera's picture shows it; the NEF of the same shot
  still gives the 6016x4016 frame, and `Metadata::focus_layout` stays
  in it, as on a crop. A DX JPEG at a smaller size still gives none.
  An export of a D600 JPEG cut or scaled to exactly 4512x3008 is read
  as a Medium JPEG, its points placed in the window, by far misplaced
  on a cut; an export scaled from a Medium JPEG to another size gives
  none, unless it is small enough, about 750 pixels tall or less, for
  the whole frame to pass the shape rule, when its point is placed in
  the whole frame, off by under a pixel.

## [0.6.0] - 2026-10-29

The sixth release: a DSLR's autofocus point, which a shot through the
viewfinder names only by number, placed on the picture from raw-files'
own table of the body's points, the Nikon D600's first; and the focus
API at its full shape, with where a point came from, the points the
camera used with the area mode, and the layout of the body's points.
The API is also made more consistent before more is built on it, which
changes some of 0.5.0's signatures: `Focus::new` takes the origin,
`RawFile::preview_at_least` a `Size`, `Limits` names its setters
`with_…` and reads its caps back, and `Preview::width` and
`Preview::height` are gone (see Changed and Removed).

### Added

- `Focus::origin` says where a focus point came from, as a `FocusOrigin`:
  `Recorded`, a position the camera measured and wrote, or `Table`, a
  point the camera named by number that raw-files placed from its table
  of the body's autofocus points.
  `Focus::new` takes it (see Changed) and `Focus::scaled_to` keeps it;
  `FocusOrigin` has a `name` that `Display` writes, "Recorded" and
  "Placed from a table", and orders as its variants are listed.
- `Metadata::focus_points` gives the autofocus points the camera used,
  as a `FocusPoints`: every point in one frame, each a `FocusPoint` with
  its number when the file gives one, its centre, its area when recorded
  and whether focus was confirmed when the file says; which of them is
  the primary, which `Metadata::focus` gives as a `Focus` and
  `FocusPoints::primary_index` by its place in the list; the area mode,
  a `FocusAreaMode`, "Single point", "Dynamic area", "Auto area" or
  "Tracking", with a `name` that `Display` writes, ordered as its
  variants are listed; and where the points came from.
  `FocusPoints::scaled_to` places every point on a preview as
  `Focus::scaled_to` places one. A program's tests build a list with
  `FocusPoints::new`, `with_point`, `with_primary_index` and
  `with_area_mode`, each point with `FocusPoint::new` and its
  `with_number`, `with_size` and `with_in_focus`, or a list of one point
  from a `Focus` with `From`, and set it with
  `Metadata::with_focus_points`; `Metadata::with_focus` sets a list of
  that one point.
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
  `FocusLayoutPoint` with the camera's number for it, the centre of its
  box and the box's size; `FocusLayout::point` finds a point by its
  number, so that a viewer can draw the grid and mark on it the points
  the camera used. `FocusLayout::new` builds one from a `static` array
  of points, which `FocusLayoutPoint::new` can fill since it is `const`,
  and refuses a frame with a zero side, no points, a number of 0, one
  given twice or points out of rising number order, and a point whose
  centre or box does not lie on the frame by the rules of `Focus::new`
  and `Focus::with_size`; a program's tests set it with
  `Metadata::with_focus_layout`. The readers give a
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

### Changed

- `Focus::new` takes where the point came from, a `FocusOrigin`, as its
  third argument, so that every focus value says where it came from
  when it is built. A program that built a focus with 0.5.0's
  `Focus::new(frame, center)` writes
  `Focus::new(frame, center, FocusOrigin::Recorded)` for the same
  value.
- `RawFile::preview_at_least` takes the box as a `Size`, as
  `Focus::scaled_to` takes a frame, so that a size has one form across
  the API: `preview_at_least(Size { width: 1024, height: 768 })` for
  0.5.0's `preview_at_least(1024, 768)`.
- The setters of `Limits` are named after the cap they set, as every
  `with_…` method is: `with_max_ifds`, `with_max_ifd_entries`,
  `with_max_ifd_depth` and `with_max_preview_bytes`. The names they had,
  `max_ifds`, `max_ifd_entries`, `max_ifd_depth` and
  `max_preview_bytes`, now read each cap back and take no argument, so
  0.5.0's `Limits::default().max_ifds(64)`, which is
  `Limits::default().with_max_ifds(64)` now, fails to compile rather
  than set nothing.

### Removed

- `Preview::width` and `Preview::height`: `Preview::size` gives both,
  as `preview.size().width` and `preview.size().height`.

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

[Unreleased]: https://github.com/kuolas/raw-files/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/kuolas/raw-files/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/kuolas/raw-files/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/kuolas/raw-files/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/kuolas/raw-files/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/kuolas/raw-files/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/kuolas/raw-files/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kuolas/raw-files/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kuolas/raw-files/releases/tag/v0.1.0
