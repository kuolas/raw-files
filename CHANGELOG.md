# Changelog

All notable changes to raw-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the version numbers follow [Semantic Versioning](https://semver.org/).
Before 1.0, a minor release (0.2.0, 0.3.0, ...) may change the public API,
and the changelog says what changed; a patch release (0.1.1, ...) never
does. An entry that can break a caller opens with **Breaking:**.

## [Unreleased]

### Added

- `DateTime::to_local_nanoseconds` and `DateTime::to_unix_nanoseconds`
  each give a number to sort times by
  ([#103](https://github.com/kuolas/raw-files/issues/103)). A caller
  built one from seven getters before:
  - the local number orders the pictures of one camera's clock;
  - the Unix number orders times from any clocks, and is `None` for a
    time without an offset from UTC.

  `DateTime` itself still has no order.
- `Orientation::rotation_degrees` and `Orientation::is_mirrored` give
  an orientation's two parts: a turn clockwise of 0, 90, 180 or 270
  degrees, then a mirror left to right or none
  ([#149](https://github.com/kuolas/raw-files/issues/149)). A program
  applies an orientation, or names it in its own language, without
  matching the eight variants.
- The module `tags` gives a constant for each tag `Entry::name` has a
  name for, 219 in all, one module per kind of directory
  ([#106](https://github.com/kuolas/raw-files/issues/106)). A lookup
  reads as what it looks for:

  ```rust
  // 0.10.0
  entries.entry(Directory::Exif, 0x9003) // DateTimeOriginal
  // 0.11.0
  entries.entry(Directory::Exif, tags::exif::DATE_TIME_ORIGINAL)
  ```

  Each constant comes from the same row as the name `Entry::name`
  gives, so the two cannot disagree.
- `Value::display` gives a short form of a value to print, so a list of
  entries is one `println!` per entry
  ([#148](https://github.com/kuolas/raw-files/issues/148)):

  ```rust
  for entry in file.entries() {
      println!("{} {}", entry.name().unwrap_or("?"), file.read_value(entry)?.display());
  }
  ```

  What each type prints:
  - text: as `Value::text` cleans it, each control character escaped;
  - numbers: in decimal;
  - fractions: as they print;
  - UNDEFINED bytes: in hexadecimal.

  A long value stops after 16 values, or 80 characters of text, and
  gives its length; a precision such as `{:.4}` sets the number. The
  form is for people and may change. `Value` itself has no `Display`.
- `Entries::skipped_directories` lists each directory a file links to
  that raw-files could not read, as a `SkippedDirectory`
  ([#92](https://github.com/kuolas/raw-files/issues/92)). Such a
  directory left no trace before, so a caller could not tell it was
  there. Each one gives:
  - the listed directory that holds the link, and the link's tag;
  - where the link leads, when that is known;
  - why, as an `ErrorKind`: `LimitExceeded` when a cap of `Limits`
    stopped the reader, so that a larger limit may list it, and
    `Malformed` for damage.

  ```rust
  for skipped in file.entries().skipped_directories() {
      println!("{} links to a directory left out: {}", skipped.parent(), skipped.kind());
  }
  ```

  Of the public files measured, the Sony DSLR-A100's list two, some
  Nikon Coolpix NRWs and COOLSCAN NEFs one, and one damaged JPEG one. A
  Nikon MakerNote's PreviewIFD tag that is not one LONG is listed too,
  with no offset; no file measured writes one
  ([#185](https://github.com/kuolas/raw-files/issues/185)).
- `Limits::max_jpeg_markers` caps how many markers one walk over a JPEG
  stream reads, which was a fixed 1,024
  ([#144](https://github.com/kuolas/raw-files/issues/144)):
  - its default is still 1,024, so nothing changes under the default;
  - `Limits::unlimited()` now raises it to `u32::MAX`, and
    `with_max_jpeg_markers` sets it;
  - each fill byte before a marker counts as one marker. The JPEG
    standard allows any number of them. A JPEG file or a preview with
    more before its frame header than the cap allows is now read under
    a raised cap.
- A JPEG from a Nikon Coolpix E700, E800, E900 or E950 now lists its
  MakerNote's entries in `CameraFile::entries`, under
  `Directory::MakerNote`
  ([#42](https://github.com/kuolas/raw-files/issues/42)). Those bodies
  write a MakerNote of their own layout, after the 8 bytes
  `Nikon\0\1\0`, which raw-files did not read. Its tags are numbered
  in a scheme of their own, and no file confirms what they record, so
  `Metadata` gains no value from them. The layout is read only in a
  little-endian Exif segment, the case of every file known.
- `Metadata::placed_focus_layout` gives the layout of the body's
  autofocus points placed on the file's picture, as a
  `PlacedFocusLayout`
  ([#65](https://github.com/kuolas/raw-files/issues/65)):
  - a viewer draws the grid on any preview with its `scaled_to`, a
    crop's included, where `Metadata::focus_layout` stays in the
    table's frame;
  - a crop shot in manual focus or in live view has it too, where the
    crop's window was known before only from the points the camera
    used.
- `FocusAreaMode::Pinpoint`, named "Pinpoint": Nikon files whose
  AFInfo2 records area-mode code 192 give it through
  `CameraSettings::focus_area_mode`, where they gave no mode before
  ([#63](https://github.com/kuolas/raw-files/issues/63)). A code is
  named where the list for the record's version and detection method
  holds it, and every list but the Nikon 1 bodies' holds 192. Z 9
  frames shot in pinpoint AF write the code, NEF and JPEG alike; the
  public Z6III samples write it too.
- `CameraSettings::focus_mode` gives how the camera focused, a
  `FocusMode`: by hand, or in which autofocus mode, such as continuous
  AF ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon
  and Sony files give it from their MakerNote, raw and JPEG alike:
  - Nikon's FocusMode text: `MANUAL`, `AF-S`, `AF-C`, `AF-A` and
    `AF-F`;
  - Sony's FocusMode code: manual focus, AF-S, AF-C, AF-A, DMF and
    AF-D, continuous AF with a depth map assisting.

  Its names are "Manual focus", "Single AF", "Continuous AF",
  "Automatic AF", "Full-time AF" and "Direct manual focus".
- `CameraSettings::focus_area_mode` names many more modes, and
  `FocusAreaMode::Zone` is given, from Nikon's and Sony's MakerNotes
  ([#214](https://github.com/kuolas/raw-files/issues/214)):
  - Nikon's AFInfo2: the codes of the list its published documentation
    gives for the version and the detection method, such as the
    dynamic-area sizes, group area, wide area and live view's subject
    tracking, and at the Nikon 1 versions, 0200 and 0201, which record
    no focus point. Face priority and plain contrast detection, which
    no variant names, give `None`. Under hybrid detection at versions
    0100 to 0301, for which the documentation gives no list, the codes
    both lists share and the mirrorless codes are named, and the DSLR
    codes 0 to 18 give `None`;
  - Nikon's AFInfo, on the bodies before AFInfo2: single and dynamic
    area, the closest subject, group dynamic, and single and dynamic
    area with the wide focus frame;
  - Sony's AFAreaModeSetting, by the codes of the body's family: wide,
    center, flexible spot, zone, expanded flexible spot, the A-mount
    adapter's flexible spot and center, a custom AF area, and on the SLT
    bodies local and spot.

  A list is read whole once the measured files hold part of it, so
  codes no measured file holds are named with their list. A few pairs
  named before now give `None`, and no measured file records any of
  them:
  - the DSLR codes 0, 5, 8 and 9 in versions 0400 to 0402, whose list
    holds the mirrorless codes alone;
  - 193 and 197 in versions 0100 to 0301 under a detection method the
    documentation does not name (byte 4 above 2).

  The notes mark each code as confirmed by a frame, observed on files,
  or documented only.
- `CameraSettings::af_fine_tune` gives AF fine-tune, an `AfFineTune`:
  whether it was on and, when it was, its value in the camera's steps
  ([#214](https://github.com/kuolas/raw-files/issues/214)). It is built
  with `AfFineTune::on()` or `AfFineTune::off()`. For a zoom tuned at
  both ends, `AfFineTune::tele_value` gives the value at the long end,
  and `with_tele_value` sets it. Nikon files give it from their
  MakerNote's AFTune, the long end's value under its "On (Zoom)" code;
  which saved lens the value belongs to is not given. Sony files give
  none: they record their AF micro adjustment only in enciphered or
  unread blocks.
- `CameraSettings::stabilization` gives the camera's image
  stabilization, a `Stabilization`: whether it was on and, when the file
  records one, its mode, a `StabilizationMode`: normal, active or
  sport. It is built with `Stabilization::on()` or
  `Stabilization::off()`, and `with_stabilization` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give it from their MakerNote's VRInfo at versions 0100, 0101 and 0200,
  the mode from each version's list of codes. Sony files give it from
  their ImageStabilization, on or off, with no mode. Which kind of
  stabilization it was, in the lens or the body, is not given.
- `CameraSettings::auto_iso` says whether auto ISO was on, and
  `with_auto_iso` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give it from bit 5 of their MakerNote's ShootingMode, but the D70's,
  whose bit 5 has another meaning; the Coolpix bodies from their
  ISOSelection text, `AUTO` or `MANUAL`. Sony files give none: they
  record the ISO setting only in enciphered or unread blocks.
- `CameraSettings::exposure_fine_tune` gives the photographer's
  fine-tuning of the camera's metering, in EV as recorded, and
  `with_exposure_fine_tune` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give it from their MakerNote's ExposureTuning, three bytes read as a
  count times a factor over a divisor, so that `fd 01 06` is -3/6 EV; a
  zero factor or divisor gives `None`
  ([#225](https://github.com/kuolas/raw-files/issues/225)). Sony files
  give it from their ExposureStandardAdjustment, one SRATIONAL.
- `CameraSettings::release_mode` gives how the shutter was released, a
  `ReleaseMode`: a single frame, continuous, the self-timer or a remote.
  `CameraSettings::bracketing` gives whether the picture is one of a
  bracketed series, a `Bracketing`: off, or the exposure, the white
  balance or the dynamic range varied. Each has its `with_…` builder
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give both from their MakerNote's ShootingMode. Nikon's single frame,
  quiet release and mirror up set none of its release bits, and read
  as `Single`. Sony files give both from their ReleaseMode: a bracket
  gives no release mode, and its code 5, which files contradict, gives
  neither. Where the MakerNote gives no bracketing, Exif's ExposureMode
  2, auto bracket, gives `Exposure`.
- `CameraSettings::daylight_saving_time` says whether the camera's
  clock was on daylight saving time, and `with_daylight_saving_time`
  sets it ([#214](https://github.com/kuolas/raw-files/issues/214)).
  Nikon files from the D300 on give it from their MakerNote's
  WorldTime, whose time zone already gives older bodies' capture time
  its offset. Sony files give none.
- `Metadata::digitized_time` gives when the picture was stored as
  digital data, and `with_digitized_time` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). It comes
  from Exif's DateTimeDigitized, with its own fraction of a second and
  offset, by the capture time's rules. On older Nikon bodies the offset
  is the time-zone setting of the camera's clock, as for the capture
  time. A camera writes the capture time here too, unless it stores the
  picture later, as when it makes a JPEG from a raw file. The capture
  time never falls back on it.
- `Flash::red_eye_reduction` says whether red-eye reduction was on, and
  `Flash::return_light` whether the camera detected the light the
  flash sent back, `None` when the file does not say or the flash did
  not fire; each has its `with_…` builder
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Both come
  from Exif's Flash, beside whether the flash fired and its mode. Exif
  gives red-eye reduction one bit, whose clear value reads "no red-eye
  reduction mode or unknown", so `false` also means not known.
- `Flash::source` gives which flash fired, a `FlashSource`: the
  camera's own or an external unit. `Flash::control` gives how its
  output was controlled, a `FlashControl`: TTL, auto aperture, manual
  or repeating. Each has its `with_…` builder
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give both from their MakerNote's FlashType text, such as
  `Built-in,TTL-BL` or `Optional,M`; commander mode is not given. Sony
  files give an external flash from their FlashAction. A file without
  Exif's Flash still gives no flash.
- `Flash::sync` gives when in the exposure the flash fired, a
  `FlashSync`: front curtain, rear curtain, slow sync or slow
  rear-curtain sync, and `with_sync` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give it from their MakerNote's FlashSetting text, such as `NORMAL` or
  `REAR SLOW`. Its red-eye reduction text, `RED-EYE`, gives front-curtain
  sync and turns `Flash::red_eye_reduction` on.
- `Flash::compensation` gives the flash compensation the photographer
  set, in EV as recorded, and `with_compensation` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give it from their MakerNote's FlashExposureComp, four bytes read as a
  count times a factor over a divisor, so that `fa 01 06 00` is -6/6 EV;
  a flash unit's own setting, which Nikon records beside it, is not
  given. Sony files give it from their FlashExposureComp, one
  SRATIONAL, such as `-7/10`. A recorded zero is a compensation of
  zero.
- `CameraSettings::raw_bit_depth` gives how many bits each sample of the
  raw data holds, as the file records it, and `with_raw_bit_depth` sets
  it ([#214](https://github.com/kuolas/raw-files/issues/214)). It is the
  BitsPerSample of the IFD that gives the raw size, when that is one
  value: 12 or 14 on most Nikon and Sony raw files. Sony's medium and
  small raws in YCbCr record three values and give none, and a JPEG
  gives none.
- `CameraSettings::raw_compression` gives how the raw data is
  compressed, a `RawCompression`: uncompressed, lossless, lossy, or
  Nikon's High Efficiency and High Efficiency with its star, and
  `with_raw_compression` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). It comes from
  the raw data's Compression: TIFF's 1 is uncompressed whatever the
  maker. Nikon's compressed form is refined by the MakerNote's
  NEFCompression, or on the Z bodies that write none by the code at byte
  10 of its tag 0x0051; NEFCompression's code 8, a small raw, gives none.
  Sony's 7 is lossless, its 32767 lossy where the data takes one byte a
  pixel, and the ILCE-7M5's 32766 lossy. A JPEG gives none.
- `CameraSettings::jpeg_quality` gives the quality step of the JPEG the
  camera wrote, a `JpegQuality`, in its maker's own words, and
  `with_jpeg_quality` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Its variants
  list each maker's steps from the smallest files to the finest detail,
  so the order ranks one maker's steps and says nothing across makers.
  Nikon JPEGs give it from their MakerNote's Quality text, `FINE`,
  `NORMAL` or `BASIC`; a Nikon raw file gives none, and so does a JPEG
  whose Quality says `RAW`. Sony files give it from the first of their
  tag 0x202E's second value, tag 0x0102 in a JPEG and JPEGQuality that
  names a step, so a Sony raw file shot with a JPEG gives that JPEG's
  step. The ILCE-7M2 records it in no plain tag and gives none.
- `CameraSettings::jpeg_compression` gives how Nikon's JPEG compression
  was set, a `JpegCompression`: size priority or optimal quality, the Z
  bodies' starred step. `with_jpeg_compression` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon JPEGs
  give it from their MakerNote's JPGCompression; a raw file gives none,
  as it gives no JPEG quality.
- `WhiteBalance::preset_slot` gives the slot of a custom white balance,
  and `with_preset_slot` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Nikon files
  give it from their MakerNote's WhiteBalance text, the number after
  `PRESET`: `PRESET1` is slot 1, which NX Studio calls "d-1". The text is
  read by its form, as a chosen color temperature's is, so the D3X's
  `PRESET0` gives the custom mode and slot 0; a bare `PRESET` gives the
  custom mode and no slot.
- `WhiteBalance::fluorescent_lamp` gives the kind of fluorescent lamp a
  white balance was set for, a `FluorescentLamp`, and
  `with_fluorescent_lamp` sets it
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Every file
  gives it from Exif's LightSource, 12 to 16, the five kinds Exif names,
  from daylight to warm white, under a manual white balance, as the mode
  reads it; a MakerNote that names a mode other than fluorescent drops
  it. Sony's MakerNote tag 0x0112, which repeats it, is not read.
- `CameraSettings::contrast`, `saturation` and `sharpness` give the
  direction of the contrast, saturation and sharpening the camera
  applied, as Exif records them
  ([#230](https://github.com/kuolas/raw-files/issues/230)). Each has its
  `with_…` builder and its enum:
  - `Contrast` and `Sharpness`: normal, soft or hard;
  - `Saturation`: normal, low or high.

  They are read from Exif's tags of the same names for every maker, raw
  and JPEG alike, and a code Exif reserves gives `None`. Each gives a
  direction only: a maker's finer value belongs to its picture style.
- `CameraSettings::scene_capture_type` gives the type of scene the
  camera was set for, a `SceneCaptureType`: standard, landscape,
  portrait or night scene. `with_scene_capture_type` sets it
  ([#230](https://github.com/kuolas/raw-files/issues/230)).
  - It is read from Exif's SceneCaptureType for every maker, raw and
    JPEG alike.
  - Standard is a type the camera recorded, not the absence of one.
  - A code Exif reserves gives `None`, as the 4, 5, 6 and 64 that some
    phones write do.
  - On the Nikon files measured it follows the Picture Control more
    than the mode.
- `CameraSettings::custom_rendered` says whether the camera processed
  the picture in a special way, and `gain_control` gives its gain
  adjustment, a `GainControl`
  ([#230](https://github.com/kuolas/raw-files/issues/230)). Each has its
  `with_…` builder.
  - They are read from Exif's CustomRendered and GainControl for every
    maker, raw and JPEG alike. A code Exif reserves gives `None`,
    as the 3 and 4 that Apple's phones write in CustomRendered do.
  - `GainControl` names none, low gain up, high gain up, low gain down
    and high gain down. Exif's "None" is the variant `NoAdjustment`,
    whose name is "None".
  - Bodies mark a custom process for different things. Every Nikon Z 9
    file measured is marked. The D3200 and D7200 mark only the files
    whose long exposure noise reduction ran.
- `CameraSettings::picture_style` gives the picture style the camera
  processed the picture in, a `PictureStyle` under its maker's own name.
  `with_picture_style` sets it
  ([#230](https://github.com/kuolas/raw-files/issues/230)).
  - Its first variant is Nikon's `PictureControl`. It gives the name and
    the base as the file records them, with no list of names. It also
    gives the adjustment kind, quick adjust, sharpening, contrast,
    brightness, saturation, hue, the filter effect, the toning and the
    toning's saturation.
  - Each value is a `PictureControlValue`: `Auto`, or `Steps` as
    recorded. A value the camera marks not available gives `None`.
  - It is read from Nikon's PictureControlData, or from the Coolpix
    bodies' copy under 0x00BD, at version 0100 with its 58 bytes. A
    record of another version or length gives no style.
  - The 68-byte record of version 0200 is read too, with its clarity.
    Its values count quarter steps, as `Steps` keeps them: five steps
    are `20/4`.
  - `PictureStyle::setting_name` gives the maker's word for the setting,
    and `PictureStyle::name` the style's name. Each maker's type
    converts into a `PictureStyle` with `From`.

### Changed

- **Breaking:** `RawFile` is now `CameraFile`. The handle opens JPEG
  files too, for which `Format::is_raw` is false, and the docs already
  called it an open camera file. Rename the type wherever it is
  imported or written, such as `RawFile<File>` in a field. No method's
  name or arguments changed, and `Debug` writes the new name:

  ```rust
  // 0.10.0
  use raw_files::RawFile;
  let file = RawFile::open("photo.nef")?;

  // 0.11.0
  use raw_files::CameraFile;
  let file = CameraFile::open("photo.nef")?;
  ```

  A file opens and reads as before
  ([#107](https://github.com/kuolas/raw-files/issues/107)).
- **Breaking:** `CameraFile::read_value`, `read_preview` and
  `read_preview_into` take `&self`, no longer `&mut self`, so a program
  reads a value or a preview while it goes through `entries()` or
  `previews()`, with no copy of the list:

  ```rust
  let file = CameraFile::open("photo.nef")?;
  for entry in file.entries() {
      let value = file.read_value(entry)?;
  }
  ```

  Remove the `mut` on the handle that the compiler now reports as
  unused, and any copy such as `file.previews().to_vec()` made only to
  read. The reader sits behind a lock inside the handle, so threads
  that share a handle read in turn, and the handle is `Sync` when its
  reader is `Send`: a handle over a reader that is `Sync` and not
  `Send` is no longer `Sync`
  ([#89](https://github.com/kuolas/raw-files/issues/89)).
- **Breaking:** `Entries::skipped` is now `Entries::skipped_entries`,
  beside the new `skipped_directories`
  ([#92](https://github.com/kuolas/raw-files/issues/92)). It still
  counts the entries left out of one directory. Rename the call:

  ```rust
  // 0.10.0
  let left_out = entries.skipped(Directory::MakerNote);
  // 0.11.0
  let left_out = entries.skipped_entries(Directory::MakerNote);
  ```
- **Breaking:** each link the walk does not follow now counts toward
  `Limits::max_ifds`, as a directory it reads does, so the list of
  directories left out stays as bounded as the directories read
  ([#92](https://github.com/kuolas/raw-files/issues/92)). These were free
  before, and each now counts once per position it leads to:
  - a link into the TIFF header;
  - a link past the largest offset, or a SubIFDs value that cannot be
    read, once per directory and tag;
  - a SubIFD of a directory at the depth cap.

  A file whose directories and dead links together pass the cap now
  fails with `LimitExceeded`, where it opened before. No camera file
  comes near: the most measured in one file is 9, under the default 256.
- **Breaking:** the AF-area mode is now a camera setting,
  `CameraSettings::focus_area_mode`; `FocusPoints::area_mode` and
  `FocusPoints::with_area_mode` are removed
  ([#214](https://github.com/kuolas/raw-files/issues/214)). The mode is
  how the camera was set, not a part of the points:
  - it is given whether or not the file gives focus points, and under
    manual focus too, where a viewfinder shot of a body without a table,
    a manual-focus shot and a point that cannot be placed gave none
    before;
  - `FocusAreaMode` keeps its path, `raw_files::FocusAreaMode`, and
    gains `Zone`, named "Zone": an area the photographer placed, in
    which the camera chose.

  Read the mode from the settings, and build it there in a test:

  ```rust
  // 0.10.0
  let mode = metadata.focus_points().and_then(|points| points.area_mode());
  // 0.11.0
  let mode = metadata.camera_settings().focus_area_mode();
  ```
- **Breaking:** `FineTune`, the white balance's shift, is now
  `WhiteBalanceFineTune`
  ([#214](https://github.com/kuolas/raw-files/issues/214)). The crate
  now gives two more fine-tunings, `AfFineTune` and
  `CameraSettings::exposure_fine_tune`, so the name says which one it
  is. Rename the type wherever it is imported or written. Its methods
  keep their names, and so do `WhiteBalance::fine_tune` and
  `with_fine_tune`:

  ```rust
  // 0.10.0
  let shift = FineTune::new(amber_blue, green_magenta);
  // 0.11.0
  let shift = WhiteBalanceFineTune::new(amber_blue, green_magenta);
  ```
- **Breaking:** a `Flash` is built with `Flash::fired()` or
  `Flash::not_fired()`, no longer `Flash::new(bool)`, and whether it
  fired is `Flash::has_fired`, no longer `Flash::fired`
  ([#214](https://github.com/kuolas/raw-files/issues/214)). The call
  says what it builds, as `Stabilization::on()` does:

  ```rust
  // 0.10.0
  let flash = Flash::new(true).with_mode(FlashMode::Fill);
  assert!(flash.fired());
  // 0.11.0
  let flash = Flash::fired().with_mode(FlashMode::Fill);
  assert!(flash.has_fired());
  ```
- A tag that raw-files reads only at an exact count is now skipped,
  unread, when it holds another count
  ([#109](https://github.com/kuolas/raw-files/issues/109)). A tag of
  another type was already skipped before its value was read. These
  tags were read whole and then counted: Nikon's CropArea, ISO, Lens,
  WorldTime and CropHiSpeed, and a DNG default crop. A hostile file now
  costs less reading. A source that fails at such a value no longer
  makes opening the file fail.
- Nine standard tags of one number are now read only as one SHORT or
  LONG, or one IFD for a pointer
  ([#158](https://github.com/kuolas/raw-files/issues/158)). Each was
  read by its first value, whatever its count. A tag of another count
  or type is no longer read:
  - an Exif, GPS or Interoperability IFD tag: its directory is not
    read, and `Entries::skipped_directories` lists the link, with no
    offset, counted toward `Limits::max_ifds` as every link it lists;
  - JPEGInterchangeFormat or JPEGInterchangeFormatLength: the directory
    gives no preview through them;
  - PhotometricInterpretation or NewSubfileType: unless another tag
    says the directory holds raw data, what it holds is unknown, so it
    gives no preview and no raw size, as a directory whose tags cannot
    be read;
  - Compression: the directory gives no preview from its strip;
  - IFD0's Orientation: the previews and the metadata take
    `Orientation::NoTransforms`, as for a missing tag.

  No file measured writes one of the nine at another count, so none
  changes.
- `Entry::name` now gives "SubIFDs" for tag 0x014A in IFD0, the IFDs
  chained after it and the SubIFDs
  ([#90](https://github.com/kuolas/raw-files/issues/90)). The name comes
  from Adobe's *PageMaker 6.0 TIFF Technical Notes* (1995), read for it
  as the other documents were: no other reader's tables were used. The
  same notes define field type 13, which `FieldType::Ifd` still shows
  as "IFD".
- A JPEG's previews now take only the large thumbnails of its
  Multi-Picture Format (MPF) list, the first of each class
  ([#53](https://github.com/kuolas/raw-files/issues/53)). Every other
  image the list holds was a preview before:
  - a gain map, so that `CameraFile::preview_at_least` could give a
    phone's gain map in place of the picture;
  - a frame of a panorama, of a stereo pair or of a set of views;
  - an original kept from before an edit, which could be the largest
    preview;
  - an image of undefined type, and a second large thumbnail of a
    class.

  A list costs at most five images to check, where a hostile one cost
  4,091. No camera JPEG measured lists another image, so none changes.
- `Camera::model` now names four more Nikon bodies: the D300S, D3200,
  D3300 and D7100
  ([#214](https://github.com/kuolas/raw-files/issues/214)). Public files
  show each one's Model text, and Nikon's manual gives its name. Their
  files now show as "Nikon D300S" and so on through
  `Camera::display_name`, where they showed the recorded text, such as
  "NIKON D300S", before.
- `CameraSettings::image_area` now gives the body's sensor format where
  a Nikon file records its crop as "off", or records no crop
  ([#214](https://github.com/kuolas/raw-files/issues/214)). It does so
  for the bodies raw-files' table holds: full frame on the D600, APS-C
  on the D300S, D3200, D3300 and D7100. The format
  comes from Nikon's manual for each body, as the notes on camera names
  record. Such a file gave `None` before, so a caller that took `None`
  for an older body now gets a value, and cannot tell it from one the
  file recorded. A crop the file names still comes first, such as the
  D600's DX crop, and a Sony file still gives `None`. So does a file
  whose MakerNote holds no crop and an entry too damaged to read, and
  one that records "off" beside a cropped size smaller than the
  original.
- `CameraSettings::raw_image_size` now gives `Large` for a Nikon raw
  file that records no raw size, when the body offers only one
  ([#214](https://github.com/kuolas/raw-files/issues/214)). raw-files'
  table says so of the D600, D300S, D3200, D3300 and D7100. Such a file
  gave `None` before. `Large` then also means a body that offers no
  other size, and a caller cannot tell it from a size the file
  recorded. A JPEG still gives `None`, since it holds no raw data, and
  so do a TIFF image a Nikon body writes and a file whose MakerNote holds
  an entry too damaged to read.

### Fixed

- A tag that raw-files reads by its first value is now read from that
  value alone when the file gives it many
  ([#114](https://github.com/kuolas/raw-files/issues/114),
  [#109](https://github.com/kuolas/raw-files/issues/109)). Such tags
  are the three tags of the ISO, the exposure time, the f-number, the
  focal length, the exposure bias, and the width and height of the raw
  size.
  One longer than the cap on one value (`Limits::max_value_bytes`) was
  refused, taking its value with it. A shorter one was copied whole to
  use its first value.
- A Nikon MakerNote's small preview is found only when the pointer to
  it, its offset and its length are each one LONG
  ([#110](https://github.com/kuolas/raw-files/issues/110)). That is how
  Nikon writes them. Each was read as the first value of any SHORT,
  LONG or IFD entry. So a pointer written as two SHORTs was read as its
  first SHORT. Every other value of a MakerNote already had its type
  and count checked.

## [0.10.0] - 2026-11-11

The tenth release makes the API consistent before full metadata is
built on it. A function that refuses its arguments now says which rule
they broke, in a `ValueError`. Every setter is a `with_…` method.
`Entries` gives its lists as slices, as `RawFile::previews` does. A
fraction of the raw view is a `Rational`, as a fraction of the metadata
is. `Camera::display_name` gives a camera's name to show, and
`Orientation`, `PreviewFormat` and `ErrorKind` each gain a name.

Nine entries under Changed are marked **Breaking**. Eight change
0.9.0's signatures, and each says what a caller does:

- The functions that refuse their arguments return
  `Result<_, ValueError>`, not `Option`; `.ok()` gives the `Option`
  back.
- `Value::Rational` and `Value::SRational` hold `Rational` and
  `SignedRational`, not pairs; code that matches them names the fields.
- `Options::limits(limits)` is `Options::with_limits(limits)`.
- `FocusPoints::primary` gives `Option<FocusPoint>`, not a reference.
- `FieldType::code` is `FieldType::to_code`.
- `Latitude::to_degrees` and `Longitude::to_degrees` are
  `to_decimal_degrees`.
- `Entries::directories` and `Entries::in_directory` return slices, not
  iterators.
- `Limits` is `Copy`, and its getters take `self`.

The ninth marked entry says that `Limits::max_value_bytes` now bounds
every value raw-files reads. Only a caller who changes the cap sees it.
The [API docs of 0.10.0](https://docs.rs/raw-files/0.10.0/raw_files/) hold
the rules of each item named below.

### Added

- `Camera::display_name` gives a camera's name to show a person, for
  every file that records a Model. It is the maker's name and the
  body's, such as `Nikon Z6III`, where raw-files has both. For any
  other body it is the Model as the file records it, such as
  `NIKON D7200`. `Camera::model` is unchanged: `None` for a body the
  table of names does not hold
  ([#104](https://github.com/kuolas/raw-files/issues/104)).
- `ValueError` is the error of a function that refuses its arguments.
  It has one variant per rule, such as `ValueError::CenterOffFrame`,
  and its `Display` text says the rule
  ([#105](https://github.com/kuolas/raw-files/issues/105)).
- `Options::limits()` reads back the limits that files are opened
  under: `Limits::default()` until `Options::with_limits` sets others
  ([#107](https://github.com/kuolas/raw-files/issues/107)).
- `Orientation`, `PreviewFormat` and `ErrorKind` each have a `name`,
  which `Display` writes. Fourteen other enums of the crate, such as
  `Format` and `Make`, already do. Their docs sent a caller to `Debug`
  before ([#107](https://github.com/kuolas/raw-files/issues/107)):
  - An orientation says what a viewer does to the stored pixels, such
    as `Rotate 90 degrees clockwise`, for an information panel.
  - A preview's encoding is `JPEG`, as `Format::Jpeg`'s name is.
  - An error's category is lowercase with no final period, such as
    `malformed file`, so that it reads in front of an error's own
    text: `malformed file: bad tag at byte 12`. An error's own text
    still does not name its category.
- `Size::area` gives how many pixels a size has, as a `u64` that
  cannot overflow. `Size`'s docs already told a caller to compare areas
  to tell which of two pictures is larger, since sizes order by width
  first ([#150](https://github.com/kuolas/raw-files/issues/150)).

### Changed

- **Breaking:** every function that refuses its arguments returns
  `Result<_, ValueError>` where it returned `Option`. A refusal now
  says which rule it broke. The functions are `Rational::new`,
  `SignedRational::new`, `DateTime::new`, `DateTime::with_subseconds`,
  `DateTime::with_offset_minutes`, `Latitude::north` and `south`,
  `Longitude::east` and `west`, `Altitude::above` and `below`,
  `Direction::new`, `Focus::new`, `Focus::with_size`,
  `Focus::scaled_to`, `FocusPoints::new`, `FocusPoints::with_point`,
  `FocusPoints::with_primary_index`, `FocusPoints::scaled_to` and
  `FocusLayout::new`. A test that builds values with `?` returns a
  `Result` in place of an `Option`:

  ```rust
  // 0.9.0
  fn focus(frame: Size, center: Point, area: Size) -> Option<Focus> {
      Focus::new(frame, center, FocusOrigin::Recorded)?.with_size(area)
  }
  assert_eq!(Rational::new(1, 0), None);

  // 0.10.0
  fn focus(frame: Size, center: Point, area: Size) -> Result<Focus, ValueError> {
      Focus::new(frame, center, FocusOrigin::Recorded)?.with_size(area)
  }
  assert_eq!(Rational::new(1, 0), Err(ValueError::ZeroDenominator));
  ```

  Code that wants the old `Option` calls `.ok()`. When several rules
  are broken, the first in the order the function's docs list is
  returned. Lookups and getters, such as `Format::from_extension` and
  `FocusLayout::point`, still return `Option`. What the readers give
  for a file is unchanged
  ([#105](https://github.com/kuolas/raw-files/issues/105)).
- **Breaking:** `Value::Rational` holds a `Vec<Rational>` and
  `Value::SRational` a `Vec<SignedRational>`, where they held pairs of
  numbers. A fraction of the raw view now has the type a fraction of
  the metadata has, with `Display`, `reduced` and `to_f64`. Code that
  read a pair names the fields, or lets the fraction print itself:

  ```rust
  // 0.9.0
  let shown: Vec<String> = match &value {
      Value::Rational(pairs) => pairs.iter().map(|(n, d)| format!("{n}/{d}")).collect(),
      _ => Vec::new(),
  };

  // 0.10.0
  let shown: Vec<String> = match &value {
      Value::Rational(fractions) => fractions.iter().map(Rational::to_string).collect(),
      _ => Vec::new(),
  };
  ```

  Each fraction is still as the file records it: a zero denominator is
  kept, and each sign stays on the number that carries it. The bytes
  read are unchanged. So are the values of `Metadata`, which never hold
  a zero denominator. Only `{:?}` of a value changes, writing each
  fraction's fields
  ([#102](https://github.com/kuolas/raw-files/issues/102)).
- **Breaking:** `Limits::max_value_bytes` now bounds the values
  raw-files reads for itself, for the metadata and the previews, as
  well as those `RawFile::read_value` reads. Before, those values had a
  fixed cap of their own, 1 MiB, the default of `max_value_bytes`. Only
  a caller who changes the cap sees a difference:
  - `Limits::unlimited()` now lifts the cap on every value.
  - Under a lowered cap, a longer value of the metadata is `None`, and
    a preview or a directory located by one is left out. A RAW file, or
    any TIFF file but a DNG, whose Make is longer than the cap fails to
    open with `LimitExceeded`. A JPEG file still opens, without what its
    MakerNote gives
    ([#113](https://github.com/kuolas/raw-files/issues/113)).
- **Breaking:** `Options::limits(limits)` is now
  `Options::with_limits(limits)`. Every setter of the crate is now a
  `with_…` method, and `Options::limits()` reads the limits back:

  ```rust
  // 0.9.0
  let options = Options::new().limits(Limits::unlimited());

  // 0.10.0
  let options = Options::new().with_limits(Limits::unlimited());
  ```

  A file opened with the same limits is read as before
  ([#107](https://github.com/kuolas/raw-files/issues/107)).
- **Breaking:** `FocusPoints::primary` returns `Option<FocusPoint>`
  where it returned `Option<&FocusPoint>`. This breaks code that
  compares it with a reference or passes it to a function taking one.
  Every lookup of a `Copy` value now gives it by value, as
  `Metadata::focus` does:

  ```rust
  // 0.9.0
  assert_eq!(points.primary(), points.points().first());
  let number = points.primary().and_then(FocusPoint::number);

  // 0.10.0
  assert_eq!(points.primary(), points.points().first().copied());
  let number = points.primary().and_then(|point| point.number());
  ```

  The point given is the same
  ([#150](https://github.com/kuolas/raw-files/issues/150)).
- **Breaking:** `FieldType::code()` is now `FieldType::to_code()`. It
  pairs with `FieldType::from_code`, as `Orientation::to_exif` pairs
  with `from_exif`:

  ```rust
  // 0.9.0
  let code = entry.field_type().code();

  // 0.10.0
  let code = entry.field_type().to_code();
  ```

  The codes are the same
  ([#150](https://github.com/kuolas/raw-files/issues/150)).
- **Breaking:** `Latitude::to_degrees()` and `Longitude::to_degrees()`
  are now `to_decimal_degrees()`. `degrees()` beside them gives only
  the degrees part as recorded. The `f64` returned has std's own
  `to_degrees`, which converts radians:

  ```rust
  // 0.9.0
  let (lat, lon) = (location.latitude().to_degrees(), location.longitude().to_degrees());

  // 0.10.0
  let (lat, lon) = (
      location.latitude().to_decimal_degrees(),
      location.longitude().to_decimal_degrees(),
  );
  ```

  The numbers are the same. `Altitude::to_meters` keeps its name, since
  `meters()` beside it is the whole value
  ([#150](https://github.com/kuolas/raw-files/issues/150)).
- **Breaking:** `Entries::directories` returns `&[Directory]` and
  `Entries::in_directory` returns `&[Entry]`, where they returned
  iterators. This breaks code that calls an iterator's methods on them
  or iterates the directories by value. Both lists are now slices, as
  `RawFile::previews` is, with a length and indexing:

  ```rust
  // 0.9.0
  for directory in entries.directories() {
      let count = entries.in_directory(directory).count();
  }

  // 0.10.0
  for &directory in entries.directories() {
      let count = entries.in_directory(directory).len();
  }
  ```

  The directories and their entries are the same, in the same order.
  `in_directory` of a directory the file lacks is an empty slice
  ([#150](https://github.com/kuolas/raw-files/issues/150)).
- **Breaking:** `Limits` is `Copy`, and its five getters, such as
  `Limits::max_ifds`, take `self` where they took `&self`. A getter
  called by path with a reference breaks:

  ```rust
  // 0.9.0
  let caps: Vec<u32> = all_limits.iter().map(Limits::max_ifds).collect();

  // 0.10.0
  let caps: Vec<u32> = all_limits.iter().copied().map(Limits::max_ifds).collect();
  ```

  So, rarely, does a closure without `move` that uses a `Limits` by
  value. It now borrows the value where it moved it, so it cannot
  outlive it. `Limits` holds only numbers, so a caller no longer clones
  it to open files with the same caps through several `Options`.
  `Options::limits()`, new in this release, gives it by value.
  `Options` stays only `Clone`, so that an option that cannot be copied
  can join it ([#150](https://github.com/kuolas/raw-files/issues/150)).
- The `UnknownFormat` error for a TIFF file of an unknown format has a
  message that ends in "recognizes". It ended in "recognises". The docs
  and the messages are now spelled the American way, as the API's
  names are.

## [0.9.0] - 2026-11-10

The ninth release is the second step towards full metadata: who made a
picture and where it was taken. `Metadata::authorship` gives the
artist, the copyright notice, the photographer, the editor, the title,
the description and the comment. `Camera::owner_name` gives the name of
the camera's owner. `Metadata::location` gives the position, with the
altitude, the GPS time, the image direction and the map datum.

Each is given as the file records it, a person's name and the place a
picture was taken among them. Nothing is hidden, and there is no option
to hide it: a program that shows or shares them decides what to keep.
A GPS IFD is no location: many cameras write one that holds only its
version. A file has a location only when it records a sound position.
No file checked records a photographer, an editor, a title or the name
of a camera's owner, so those readers are checked on built files only.

Nothing in 0.8.0's API was removed or changed in signature, and no
entry is marked **Breaking**. A NEF or ARW whose Make is of type UTF-8
is now read where it was refused. So is the MakerNote of a camera JPEG
with such a Make. On the NEFs checked, opening one that records a
position reads one more block (see Changed). The
[API docs of 0.9.0](https://docs.rs/raw-files/0.9.0/raw_files/) hold
the rules of each item named below.

### Added

- `Metadata::authorship` gives who made the picture and what they wrote
  about it, in an `Authorship`, from NEF, ARW and JPEG files alike. The
  artist, the copyright notice and the description come from IFD0. The
  photographer, the editor, the title and the comment come from the
  Exif IFD. Each text is given as the file records it, cleaned as every
  text is, with nothing hidden. A blank field, as a camera writes one
  its owner has not set, is `None`. The description and the comment are
  free text, with no 255-byte limit and their line breaks kept. Sony's
  copies of IFD0's texts in IFD1 and IFD2 are not read, nor anything in
  a MakerNote. The docs of each getter name its tag and say what else
  gives `None`. `Metadata::with_authorship` and the `with_…` methods of
  `Authorship` build one for a program's tests.
- `Camera::owner_name` gives the name of the camera's owner, from the
  Exif IFD's CameraOwnerName, of type ASCII or UTF-8. It names whoever
  owns the body, not whoever took the picture. Its text is cleaned as
  every text is and held to 255 bytes, and it is `None` when blank.
  `Camera::with_owner_name` sets it.
- `Metadata::location` gives where the picture was taken, in a
  `Location`, from the GPS IFD of NEF, ARW and JPEG files alike. Its
  position is a `Latitude` and a `Longitude`. Each holds the degrees,
  minutes and seconds the file records as fractions, with its
  hemisphere, and `to_degrees` gives signed decimal degrees. When the
  file records them, a location also has the `Altitude`, the GPS time
  in UTC, the image direction, a `Direction` from true or magnetic
  `North`, and the map datum. Everything is given as recorded, nothing
  rounded or hidden.
  A GPS IFD without a sound position gives no location. So a GPS IFD of
  only its version, a Sony file without a fix and a Coolpix file's
  blank tags give none. Each other part needs both its tags, and a
  damaged one costs only itself. The docs of `Metadata::location` say
  what a sound position is, and the getters of `Location` what each
  part needs. GPSVersionID, GPSStatus and the other GPS tags are not
  read; `RawFile::entries` gives them. `Metadata::with_location`,
  `Location::new` and its `with_…` methods, and the checked
  constructors of the coordinates, the altitude and the direction build
  one for a program's tests.

### Changed

- The camera's Make and Model, the Software, LensMake and LensModel are
  now read from type UTF-8 as from type ASCII. Exif 3.1 gives each as
  "ASCII or UTF-8". The cleaning is the same, and so is the limit of
  255 bytes, counted as stored. So a NEF or ARW whose Make is of type
  UTF-8 is now claimed by its reader, where it was refused as of no
  known format. A camera JPEG's MakerNote with such a Make is read too.
  The serial numbers and the capture time, which Exif gives as ASCII
  alone, are read only from type ASCII, as is every text of a maker's
  MakerNote. No file checked writes the UTF-8 type, so no value read
  from one changes.
- Opening a file whose GPS IFD records a position now reads the
  location's values. On the NEFs checked they lie past the blocks
  opening read before, so opening reads one more 64 KiB block. That is
  448 KiB in all on a Nikon Z 9's NEF, where 0.8.0 read 384 KiB. It is
  one more block on each of the 6 NEFs of raw.pixls.us that record a
  position. On the ARW and JPEG files checked, and on the files without
  a position, it reads nothing more, and neither do the authorship's
  texts. No `Limits` cap counts these reads. Each value is read under
  the bound on any one value, as every value of the metadata is.

## [0.8.0] - 2026-11-09

The eighth release is the first step towards full metadata. It gives
every entry of the TIFF directories that raw-files walks in a NEF, an
ARW or a camera JPEG, as the file records it. A Nikon or Sony
MakerNote's entries are among them. A standard tag comes with the name
a published standard gives it, and an entry's value is read when asked
for.

Nothing in 0.7.0's API was removed or changed in signature. Opening a
file reads more than it did. One entry under Changed is marked
**Breaking**: the GPS IFD now counts towards the cap on directories.
The [API docs of 0.8.0](https://docs.rs/raw-files/0.8.0/raw_files/)
hold the rules of each item named below.

### Added

- `RawFile::entries` gives every entry of every TIFF directory that
  raw-files walks, as the file records it. Four types come with it:
  `Entries`, `Entry`, `Directory` and `FieldType`. Listing the entries
  reads nothing from the file. The method's docs name the directories,
  and those of `Entries` say which entries are left out.
- `RawFile::read_value` reads an entry's value from the file, as a
  `Value` with one variant per field type. A value is read each time it
  is asked for. `Value::text` gives a text value cleaned as the
  metadata's text is, without its 255-byte limit. The docs of `Value`
  say how numbers and fractions are given.
- `Entry::name` gives the name a published standard gives the entry's
  tag in its kind of directory. Tag 0x9003 of the Exif IFD is
  `DateTimeOriginal`. The 218 names come from five documents, which the
  method's docs list, and
  [`docs/formats/tag-names.md`](https://github.com/kuolas/raw-files/blob/v0.8.0/docs/formats/tag-names.md)
  records each with its document and page. A tag has no name where no
  document names it. An Exif tag written in IFD0 has none there, and a
  maker's tags have none anywhere.
- `Limits::max_value_bytes` bounds the bytes one entry's value may
  take. The default is 1 MiB, the bound the metadata's values already
  have, and `Limits::with_max_value_bytes` sets another. A longer value
  fails only its own read, with `LimitExceeded`.

### Changed

- **Breaking:** the GPS IFD now counts towards `Limits::max_ifds`,
  since opening a file now also reads it. This matters with a cap set
  at a file's count of directories under 0.7.0. A raw file may then be
  refused, and a JPEG may lose what its Exif segment holds. The GPS IFD
  is read after the chain of directories, so that its entries can be
  listed. An IFD it shares with the chain stays there. On most older
  NEFs this reads one more 64 KiB block.
- Opening an ARW now also reads the table of Sony's private directory,
  SR2Private, so that its entries can be listed. The table lies at the
  offset that IFD0's DNGPrivateData holds. Its values are read only
  when asked for. On 89 of the 245 public ARW-family files measured,
  this reads one more 64 KiB block.
- An entry of Exif 3.0's UTF-8 type (129) is kept, where it was left
  out as one of an unknown type. The metadata still reads text only
  from ASCII entries.

## [0.7.0] - 2026-11-03

The seventh release names the camera's maker and the name the body is
sold under, from raw-files' own table of names. Both stand beside the
Make and Model texts the file records. The camera's and the lens's
values are plain values, as the exposure's are. A D600 Medium JPEG's
autofocus points are placed in the window of the frame its picture
shows, and a D600's 3D-tracking area mode is named.

Five entries are marked **Breaking**, four under Changed and one under
Removed. They change some of 0.6.0's signatures. The camera's values
move from `Metadata` to a `Camera`, and `Metadata::lens` gives a
`&Lens`. The recorded texts are named `recorded_make` and
`recorded_model`, and `LensBuilder` is gone. The
[API docs of 0.7.0](https://docs.rs/raw-files/0.7.0/raw_files/) hold
the rules of each item named below.

### Added

- `Camera::make` gives the camera's maker as a `Make`: `Nikon`, `Sony`,
  `Hasselblad` or `Canon`. It is recognized from the whole Make text the
  file records, case included. `NIKON CORPORATION` and `NIKON` are
  Nikon, `SONY` Sony, `HASSELBLAD` Hasselblad and `Canon` Canon, and any
  other text gives `None`. `Make` has a `name` that `Display` writes,
  spelled as each maker writes its own name, and orders as its variants
  are listed. `make()` is read from the recorded Make each time it is
  asked for, with no setter of its own. So a program's tests set the
  recorded text with `with_recorded_make`.
- `Camera::model` gives the name the maker sells the body under, without
  the brand, spelled as the maker writes it. A file whose Model is
  `NIKON Z6_3` gives `Z6III`, and one whose Model is `ILCE-7M2` gives
  `α7 II`. It comes from raw-files' table of the bodies the notes on
  camera names list with their evidence. They are the Nikon D600, Z 9,
  Z 6, Z 30, Z f, Z fc, Z50II, Z5II and Z6III, and the Sony α1, α1 II,
  α6000, α6700, α7 II, α7 III, α7 IV, α7 V, α7R II, α7R III, α7S,
  α7S III, α9 and α9 III. Any other body gives `None`, never the
  recorded text, which `recorded_model` still gives. `model()` is read
  from the recorded make and Model each time it is asked for, with no
  setter of its own.

### Changed

- **Breaking:** `Metadata::lens` gives a `&Lens`, as
  `Metadata::exposure` gives a `&Exposure`, instead of an
  `Option<&Lens>`. A file that describes no lens gives a lens whose
  every value is `None`, as `Lens::default()` is. So 0.6.0's
  `metadata.lens().and_then(|lens| lens.model())` is
  `metadata.lens().recorded_model()` now. An `if let Some(lens)` on it
  fails to compile rather than change meaning. A program that asked
  whether the file describes a lens at all, with 0.6.0's
  `metadata.lens().is_some()`, compares with the empty lens:
  `*metadata.lens() != Lens::default()`.
- **Breaking:** `Metadata::make`, `Metadata::model` and
  `Metadata::serial_number` move to a `Camera`, which
  `Metadata::camera` gives as `Metadata::lens` gives a `Lens`. They are
  `Camera::recorded_make`, `Camera::recorded_model` and
  `Camera::serial_number` there. The first two give the text as the
  file records it, cleaned as before. So 0.6.0's `metadata.model()` is
  `metadata.camera().recorded_model()` now. A program's tests build a
  camera from `Camera::default()` through `with_recorded_make`,
  `with_recorded_model` and `with_serial_number`. They set it with
  `Metadata::with_camera`, in place of 0.6.0's `Metadata::with_make`,
  `Metadata::with_model` and `Metadata::with_serial_number`.
- **Breaking:** `FocusLayout::model` is `FocusLayout::recorded_model`.
  Its text is the Model as the file records it, such as `NIKON D600`,
  as `Camera::recorded_model` gives it.
- **Breaking:** `Lens::make` and `Lens::model` are
  `Lens::recorded_make` and `Lens::recorded_model`, with their setters
  `with_recorded_make` and `with_recorded_model`. They give the text as
  the file records it, as `Camera`'s do. They leave the plain words for
  the lens's name that a table of lenses is to give. 0.6.0's
  `lens.model()` is `lens.recorded_model()`.
- `Lens` is built as `Exposure` is, from the empty `Lens::default()`
  through `with_…` methods, each storing its value as given. They are
  `with_recorded_make`, `with_recorded_model`, `with_serial_number`,
  `with_min_focal_length`, `with_max_focal_length`,
  `with_f_number_at_min_focal_length` and
  `with_f_number_at_max_focal_length`.
- Nikon files name the DSLR area-mode code 9, under phase detection,
  through the viewfinder, as `FocusAreaMode::Tracking`. D600 frames
  shot in 3D-tracking write it. `FocusPoints::area_mode` gave `None`
  for such a file before.

### Removed

- **Breaking:** `LensBuilder` and `Lens::builder` are gone. A program's
  tests start from `Lens::default()` and write
  `.with_recorded_model(…)` for `.model(…)`, and so on for every value,
  with no `build()` call and no `?` after it.

### Fixed

- A Nikon D600 JPEG at the Medium size, 4512x3008, shot through the
  viewfinder ([#62](https://github.com/kuolas/raw-files/issues/62)),
  gives the autofocus points it names, where it gave none. They are
  placed from the D600's table. Its picture is not the whole 6016x4016
  frame scaled, but the 6010x4006 window centered in it. The table now
  holds that window for that size, as measured on RAW+JPEG pairs.
  The frame of `Metadata::focus` and `Metadata::focus_points` is that
  window, 6010x4006, every center moved back by its origin, (3, 5). So
  a point scaled from that frame onto the JPEG lands where the camera's
  picture shows it. The NEF of the same shot still gives the 6016x4016
  frame, and `Metadata::focus_layout` stays in it, as on a crop. A DX
  JPEG at a smaller size still gives none.
  An export of a D600 JPEG cut or scaled to exactly 4512x3008 is read
  as a Medium JPEG, its points placed in the window. On a cut, they are
  by far misplaced. An export scaled from a Medium JPEG to another size
  gives none. The exception is one about 750 pixels tall or less, small
  enough for the whole frame to pass the shape rule. Its point is
  placed in the whole frame, off by under a pixel.

## [0.6.0] - 2026-10-29

The sixth release places on the picture a DSLR's autofocus point, which
a shot through the viewfinder names only by number. It does so from
raw-files' own table of the body's points, the Nikon D600's first. The
focus API reaches its full shape: where a point came from, the points
the camera used with the area mode, and the layout of the body's
points.

The API is also made more consistent before more is built on it. Four
entries are marked **Breaking**, three under Changed and one under
Removed. They change some of 0.5.0's signatures: `Focus::new` takes the
origin, and `RawFile::preview_at_least` a `Size`. `Limits` names its
setters `with_…` and reads its caps back, and `Preview::width` and
`Preview::height` are gone. The
[API docs of 0.6.0](https://docs.rs/raw-files/0.6.0/raw_files/) hold
the rules of each item named below.

### Added

- `Focus::origin` says where a focus point came from, as a
  `FocusOrigin`. `Recorded` is a position the camera measured and
  wrote. `Table` is a point the camera named by number that raw-files
  placed from its table of the body's autofocus points. `Focus::new`
  takes it (see Changed) and `Focus::scaled_to` keeps it. `FocusOrigin`
  has a `name` that `Display` writes, "Recorded" and "Placed from a
  table", and orders as its variants are listed.
- `Metadata::focus_points` gives the autofocus points the camera used,
  as a `FocusPoints`. It holds every point in one frame, each a
  `FocusPoint` with its number when the file gives one, its center, its
  area when recorded and whether focus was confirmed when the file says.
  It says which of them is the primary, which `Metadata::focus` gives as
  a `Focus` and `FocusPoints::primary_index` by its place in the list.
  It gives the area mode, a `FocusAreaMode`: "Single point", "Dynamic
  area", "Auto area" or "Tracking", with a `name` that `Display` writes,
  ordered as its variants are listed. And it says where the points came
  from. `FocusPoints::scaled_to` places every point on a preview as
  `Focus::scaled_to` places one.
  A program's tests build a list with `FocusPoints::new`, `with_point`,
  `with_primary_index` and `with_area_mode`, each point with
  `FocusPoint::new` and its `with_number`, `with_size` and
  `with_in_focus`. A list of one point comes from a `Focus` with
  `From`. `Metadata::with_focus_points` sets a list, and
  `Metadata::with_focus` sets a list of that one point.
- NEF, ARW and JPEG files give the position the camera recorded as a
  list of one point, the primary, with no number. Nikon files give the
  area mode from AFInfo2's area-mode byte, on every version read. They
  give it for the codes that frames shot in a known mode confirmed,
  under the detection method the file records. The DSLR codes are 0
  single point, 5 dynamic area and 8 auto area, under phase detection,
  through the viewfinder. The mirrorless codes are 193 single point and
  197 auto area, under any detection method, and 207 tracking, under
  contrast or hybrid detection. Any other code, or a DSLR code in live
  view, gives `None`. Sony files give no area mode.
- `Metadata::focus_layout` gives the layout of the body's autofocus
  points, as a `FocusLayout`. A viewer can draw the grid from it, and
  mark on it the points the camera used. It holds the Model it is for,
  the frame its points are measured in, the body's full-size output, and
  every point. Each point is a `FocusLayoutPoint` with the camera's
  number for it, the center of its box and the box's size.
  `FocusLayout::point` finds a point by its number. `FocusLayout::new`
  builds one from a `static` array of points, which
  `FocusLayoutPoint::new` can fill since it is `const`. The docs of
  `FocusLayout::new` list what it refuses. A program's tests set it with
  `Metadata::with_focus_layout`. The readers give a layout only from a
  table of the body's points, and `None` for a body whose table is not
  in raw-files.
- NEF and JPEG files of the Nikon D600 give its layout of 39 autofocus
  points (`Metadata::focus_layout`), whatever the shot. That holds
  through the viewfinder, in live view, in manual focus or on a crop.
  The layout depends on the Model alone. So a file whose MakerNote is
  missing or cannot be read, such as an editor's re-save, gives it too.
  The table was measured by the project on frames shot with a D600 and
  read in Nikon's NX Studio, as the NEF notes record it.
- NEF and JPEG files of the Nikon D600 shot through the viewfinder give
  the autofocus point the camera names by number, placed from that
  table. `Metadata::focus` gives the point's center and its box of
  164x164 in the 6016x4016 frame. Its origin is `Table`, and nothing is
  said of whether focus was confirmed. `Metadata::focus_points` gives
  every point that the file's record of the points used names, one bit
  per point, and the primary. Each comes with its number, in number
  order, with the area mode: single point, dynamic area or auto area.
  On the DX crop the points are placed in the crop's window, 3936x2624,
  the size of the NEF's full-size preview and of its JPEG. A crop-mode
  JPEG at a smaller size gives none.
  A Medium JPEG, 4512x3008, gives none either. Its shape, exactly 3:2,
  is further from the 6016x4016 frame than the one pixel of rounding by
  which a JPEG's point must fit its picture. A Small JPEG, 3008x2008,
  half the frame, keeps its point. A shot through the viewfinder of any
  other body gives no focus point, since raw-files has no table of its
  points.

### Changed

- **Breaking:** `Focus::new` takes where the point came from, a
  `FocusOrigin`, as its third argument. So every focus value says where
  it came from when it is built. A program that built a focus with
  0.5.0's `Focus::new(frame, center)` writes
  `Focus::new(frame, center, FocusOrigin::Recorded)` for the same
  value.
- **Breaking:** `RawFile::preview_at_least` takes the box as a `Size`,
  as `Focus::scaled_to` takes a frame, so that a size has one form
  across the API. 0.5.0's `preview_at_least(1024, 768)` is
  `preview_at_least(Size { width: 1024, height: 768 })`.
- **Breaking:** the setters of `Limits` are named after the cap they
  set, as every `with_…` method is: `with_max_ifds`,
  `with_max_ifd_entries`, `with_max_ifd_depth` and
  `with_max_preview_bytes`. The names they had, `max_ifds`,
  `max_ifd_entries`, `max_ifd_depth` and `max_preview_bytes`, now read
  each cap back and take no argument. So 0.5.0's
  `Limits::default().max_ifds(64)`, which is
  `Limits::default().with_max_ifds(64)` now, fails to compile rather
  than set nothing.

### Removed

- **Breaking:** `Preview::width` and `Preview::height` are gone.
  `Preview::size` gives both, as `preview.size().width` and
  `preview.size().height`.

## [0.5.0] - 2026-10-26

The fifth release reads the camera's settings beside the exposure, for
a viewer's information panel. They come from the Exif tags and from the
plain tags of Nikon's and Sony's MakerNotes, in NEF, ARW and JPEG
files. It also reads the focal length in 35 mm terms and the software
that wrote the file.

Nothing in 0.4.0's API was removed or changed in signature. One entry
under Changed is marked **Breaking**: the Interoperability IFD now
counts towards the cap on directories. With a very low cap, a raw file
may now be refused, and a JPEG lose its Exif metadata. The
[API docs of 0.5.0](https://docs.rs/raw-files/0.5.0/raw_files/) hold
the rules of each item named below.

### Added

- `Metadata::camera_settings` gives a `CameraSettings`: how the camera
  was set for the shot, each value optional. It holds:
  - the white balance (`WhiteBalance`: its mode, its color temperature
    in kelvin and its fine-tuning, `FineTune`);
  - the exposure program and the metering mode;
  - the flash (`Flash`: whether it fired and its mode);
  - the shutter used, and Sony's electronic front curtain setting;
  - the image area, the raw image size and the color space.

  The enums `WhiteBalanceMode`, `ExposureProgram`, `MeteringMode`,
  `FlashMode`, `Shutter`, `ImageArea`, `RawImageSize` and `ColorSpace`
  name each setting's choices in words that fit any maker. Each has a
  `name` that `Display` writes, and orders as its variants are listed,
  so a program can use them as sorted keys. A program's tests build all
  of these through their `with_…` methods,
  `Metadata::with_camera_settings`, `FineTune::new` and `Flash::new`.
- NEF, ARW and JPEG files give the settings their Exif tags record.
  They are the exposure program, the metering mode, the flash, the
  white balance mode and the color space. The white balance mode comes
  from Exif's auto or manual, and the kind of light a manual one was
  set for. The color space is sRGB or, from the `R03` index of a DCF
  option file, Adobe RGB. A code Exif does not define, or its "not
  defined" and "unknown", gives `None`.
- NEF files and the JPEGs of Nikon cameras give the rest from Nikon's
  MakerNote:
  - the white balance setting, such as a preset or a chosen color
    temperature, in place of Exif's when the MakerNote names one;
  - the color temperature of the white balance the camera applied,
    which the D5, the D850, the D780 and the Z bodies record. Where that
    is missing or 0, it is the number of a chosen color temperature,
    such as `3230K`, the one temperature older bodies record;
  - the fine-tuning;
  - the shutter used;
  - the image area;
  - the raw image size, in a raw file only;
  - the color space, when Exif records none, as in every Nikon raw file
    checked.

  The single fine-tuning value of the D1 family, the D70, the D100 and
  the D200 gives `None`, since no file shows its axis.
- The files of the ARW family (ARW, SRF, SR2 and ARQ) and the JPEGs of
  Sony cameras give settings from Sony's MakerNote too:
  - the white balance setting, such as custom, or underwater, which Exif
    calls auto, in place of Exif's when the MakerNote names one;
  - the color temperature under a chosen color temperature or a custom
    white balance, the only settings under which the files checked
    record one;
  - the fine-tuning;
  - the electronic front curtain setting: the menu setting, never the
    shutter used.

  The shutter used, the image area and the raw image size are `None` on
  Sony files. Sony records the first two only in enciphered blocks, and
  gives an APS-C raw the raw size code of a medium one. The DSLR-A100,
  A200, A700 and A900, the SLT-A55V, the NEX-5, the DSC-F828 and the
  DSC-R1 record no fine-tuning. The white balance mode of the
  DSLR-A100, the DSC-F828 and the DSC-R1 is Exif's.
- Every fine-tuning is signed as its getters are named, whatever the
  maker. `FineTune::amber_blue` is positive towards blue and
  `FineTune::green_magenta` positive towards magenta. So a viewer can
  show or compare two files' shifts without knowing how each maker signs
  them. Every color temperature is read from 1 to 99,999 K: a recorded
  0, which means none was recorded, and a larger number give `None`.
- `Exposure::focal_length_in_35mm_film` and `Metadata::software` are
  read from NEF, ARW and JPEG files. The first is the focal length in
  35 mm terms the camera recorded, in whole millimeters, from Exif's
  FocalLengthIn35mmFilm. It is the focal length that would give the
  same field of view on a full-frame camera, so that pictures from
  bodies with different sensor sizes compare. A recorded 0, which a
  camera writes when it does not know it, gives `None`. The second is
  IFD0's Software, cleaned of padding as every text value. It is the
  camera's firmware version, such as `Ver.01.00` or `ILCE-7M4 v2.00`,
  or the name of a program that rewrote the file.
  `Exposure::with_focal_length_in_35mm_film` and
  `Metadata::with_software` build them for a program's tests.

### Changed

- **Breaking:** the Exif IFD's Interoperability IFD, read for the color
  space, counts as one more directory under `Limits::max_ifds`. A file
  that holds one, as most Sony-family sample files do, needs one
  directory more than under 0.4.0. So a cap set just high enough for it
  then fails a raw file with `LimitExceeded`. A JPEG still opens, as
  for any cap crossed in its Exif segment, but without what that
  segment holds: its Exif metadata, the MakerNote's settings and focus
  point, and the Exif thumbnail. The default cap, 256, is far above
  what any file checked needs.

## [0.4.0] - 2026-10-23

The fourth release opens JPEG files, as cameras and editors write them,
as a format of their own. They give their previews, their Exif metadata
and, from Nikon and Sony cameras, the focus point. `Format::is_raw`
tells raw formats from the others. The headerless MakerNote of the
Nikon D1 family is read.

Nothing in 0.3.0's API was removed or changed in signature. Two entries
under Changed are marked **Breaking**: a JPEG's extension is now known,
and opening a JPEG now succeeds. The
[API docs of 0.4.0](https://docs.rs/raw-files/0.4.0/raw_files/) hold
the rules of each item named below.

### Added

- JPEG files open, as `Format::Jpeg`: the pictures a camera writes
  beside its raw files or instead of them, and the JPEGs an editor
  exports. They are recognized by their content like every other
  format. A program gets from one what it gets from a raw file. Its
  previews are the file itself and the smaller images it holds. These
  are the Exif thumbnail, the images of its Multi-Picture Format (MPF)
  list and, from Nikon bodies such as the D70, one more small JPEG. Its
  metadata is what its Exif segment records, with the picture's size
  and no raw size.
  In a Nikon or a Sony camera's JPEG, the maker's MakerNote adds the
  focus point. In a Nikon camera's, it also adds what Exif lacks, such
  as the serial number, the ISO, the lens's range and the offset from
  UTC. On a Nikon crop mode (DX, 1:1, 16:9) the point is placed on the
  picture. A focus point is given only when its frame has the picture's
  shape. An export cut to another shape gives none, except one cut from
  a Nikon crop-mode JPEG, whose point may then be misplaced. A JPEG
  whose Exif segment is missing or cannot be read still opens, with the
  file itself and its MPF images as previews and its image size.
- NEF files of the Nikon D1 family give the ISO and the lens's range
  where their Exif tags lack them. Those of the D1H and the D1X give a
  preview, their only JPEG, where they gave none. A JPEG file whose
  Nikon MakerNote has the D1 family's layout is read the same way. On
  the early Coolpix JPEGs checked, from Wikimedia Commons, that gives
  nothing beyond the Exif values. The small JPEG some of them hold is
  not given, as it lies outside the MakerNote.
- `Format::is_raw` is true for NEF and ARW and false for JPEG. So a
  program can tell raw files from other pictures without matching every
  `Format`.
- Eight camera JPEGs from Wikimedia Commons check the JPEG reader: every
  preview, the metadata, and the focus point or that there is none.
  They are of five Sony bodies, two Nikon bodies and a Canon, under
  CC0, a public-domain dedication or CC BY.
  `scripts/fetch-samples.sh jpeg` downloads them.

### Changed

- **Breaking:** `Format::from_extension` returns `Some(Format::Jpeg)`
  for `jpg` and `jpeg` where it returned `None`, and `Format::all`
  lists `Format::Jpeg`. A program that took every file whose extension
  `from_extension` knows for a raw file now takes JPEGs too. It should
  ask `Format::is_raw` as well.
- **Breaking:** opening a JPEG file succeeds, as `Format::Jpeg`, where
  0.3.0 returned the `UnknownFormat` error. That holds for
  `RawFile::open`, `from_bytes` and `from_reader`, and the same on
  `Options`. A program that took every file that opens for a raw file,
  or that matched `format()` assuming NEF or ARW, should ask
  `Format::is_raw`.

## [0.3.0] - 2026-10-21

The third release reads where the camera focused, from Nikon NEF and
Sony ARW files, next to the previews and the metadata. Public
constructors let a program build every metadata value in its own tests.
No entry is marked **Breaking**. The
[API docs of 0.3.0](https://docs.rs/raw-files/0.3.0/raw_files/) hold
the rules of each item named below.

### Added

- `Metadata::focus` gives where the camera focused, as it recorded it,
  with the `Focus` and `Point` types. It holds the center of the focus
  area, the area's size when the file records one, the frame the two
  are measured in, and whether the camera confirmed focus.
  `Focus::new`, `with_size` and `with_in_focus` build one, and
  `Focus::scaled_to` places it on any preview.
  Nikon mirrorless bodies and Nikon DSLRs in live view give a focus
  point, read from the MakerNote's AFInfo2 block. On a crop mode it is
  placed inside the cropped picture. Sony bodies from the ILCE-7RM2 on
  give one too, read from the MakerNote's FocusLocation. They give the
  area's size from the ILCE-7SM3 on, and nothing about whether focus
  was confirmed.
  A file that records no position gives `None`. So does a Nikon DSLR
  shot through the viewfinder or a Sony body before the ILCE-7RM2,
  which record at most an autofocus point's number or a mask of points.
  So do a manual-focus shot, an AFInfo2 version raw-files does not
  know, and a Nikon crop whose point cannot be placed on the cropped
  picture.
- NEF previews and metadata are checked on ten more CC0 Nikon sample
  files, 33 NEF files in all. The focus point is checked on every one
  of the 33. The ten are of the D5 and the D5600 in live view, of the
  Z5II, Z50II and Z6III in their 1:1 and 16:9 crop modes, of a Z fc
  shot in manual focus and of a rotated Z 30 shot.
- Public constructors and builders let a program build the values
  raw-files reads in its own tests. The constructors are those of
  `DateTime` (`new`, `with_subseconds`, `with_offset_minutes`),
  `Rational` and `SignedRational` (`new`). The builders are a builder
  for `Lens` (`Lens::builder` and its `LensBuilder`), and `with_…`
  builders on `Metadata` and `Exposure`. The constructors give `None`
  for a date or time that does not exist, a fraction with a zero
  denominator or a lens with no value. The builders store what they are
  given.
- `Size` and `Point` print as text (`Display`), `8256x5504` and
  `(4905, 2461)`, as the Rust API Guidelines ask of such types.

## [0.2.0] - 2026-10-19

The second release reads the basic metadata of Nikon NEF and Sony ARW
files, next to the previews, when a file is opened. No entry is marked
**Breaking**. The
[API docs of 0.2.0](https://docs.rs/raw-files/0.2.0/raw_files/) hold
the rules of each item named below.

### Added

- `RawFile::metadata` gives the metadata of NEF and ARW files, with the
  `Metadata`, `Exposure`, `Lens`, `DateTime`, `Rational` and
  `SignedRational` types. It holds:
  - the camera make, model and serial number;
  - the capture time, with sub-seconds and offset from UTC when
    recorded;
  - the orientation;
  - the exposure: ISO, exposure time, f-number, focal length and
    exposure compensation, as stored;
  - the lens: make, model, serial number, and focal-length and aperture
    range;
  - the sizes of the raw data and of the visible image (on Nikon files,
    the crop area the camera records).

  On Nikon bodies whose standard tags lack them, the serial number, ISO
  and lens range come from the MakerNote. On those, the offset from UTC
  comes from the camera's time-zone setting.
- `DateTime::subsecond_digits` gives how many sub-second digits the file
  recorded. So a time without sub-seconds can be told from one whose
  recorded digits are zeros, without formatting them.
- NEF previews are checked on 20 more sample files of 18 Nikon bodies,
  from the D1H to the Z6III. The metadata is checked on all 23 NEF
  sample files.
- The metadata of the 36 Sony-family sample files is checked, as their
  previews were in 0.1.0.
- `Options` compares and hashes, `Limits` hashes, and `Format`,
  `PreviewFormat`, `ErrorKind`, `Orientation` and `Size` order. The
  Rust API Guidelines ask this of such types. The `Debug` form of
  `RawFile` shows the metadata.

## [0.1.0] - 2026-10-14

The first release reads the embedded JPEG previews of Nikon NEF and
Sony ARW files. The
[API docs of 0.1.0](https://docs.rs/raw-files/0.1.0/raw_files/) hold
the rules of each item named below.

### Added

- The project is licensed under the MIT license or the Apache License,
  version 2.0, at your option.
- The library builds with Rust 1.95 or later.
- `Error` has an `ErrorKind` category and the byte offset when known,
  with the `Result` alias.
- `Limits` holds the caps on what a file can make the library walk or
  allocate.
- `Orientation` is the Exif orientation a viewer must apply to a
  preview, and `Size` a width and height in pixels.
- `Preview` is one embedded preview: where its bytes are in the file,
  its size, format and orientation. `PreviewFormat` says how its bytes
  are encoded, with a media type and a file extension.
- `Format` names the RAW formats the library knows, with a name, the
  file extensions and a lookup by extension for file pickers.
- `RawFile` opens a RAW file by path, from bytes or from any reader
  that can seek, and detects its format from its content. It lists the
  file's previews smallest first, picks the largest or the smallest at
  least a given size, and reads one, exactly as stored. A file of no
  format the library knows is refused with the `UnknownFormat` error.
- `Options` opens files with custom `Limits`.
- The embedded JPEG previews of Nikon NEF files are read: those in IFD0
  and the SubIFDs, and the one the MakerNote points at. Each is sized
  from its own frame header, with IFD0's orientation.
- The embedded JPEG previews of Sony ARW files are read, and those of
  the SRF, SR2 and ARQ files of the same family. They are those in IFD0,
  IFD1 and, on bodies writing FileFormat 4.0 and later, the full-size
  one in IFD2. Each is sized from its own frame header, with IFD0's
  orientation.
- A DNG is of no known format, whatever camera maker its Make names.
  Converters and the phones that write DNG keep the maker's name.

[Unreleased]: https://github.com/kuolas/raw-files/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/kuolas/raw-files/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/kuolas/raw-files/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/kuolas/raw-files/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/kuolas/raw-files/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/kuolas/raw-files/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/kuolas/raw-files/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/kuolas/raw-files/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/kuolas/raw-files/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kuolas/raw-files/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kuolas/raw-files/releases/tag/v0.1.0
