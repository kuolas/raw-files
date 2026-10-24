# ADR011: Metadata comes from the standard tags

- Status: Accepted; amended by
  [ADR012](adr012-focus-points-as-the-camera-records-them.md), which reads
  the Sony MakerNote for the focus point, and in place on 2026-10-24 to
  read the camera's settings (the sections "The camera settings" below)
- Date: 2026-10-15
- Issue: #13; the camera settings #26

## Context

After the previews, the next thing a culler needs from a RAW file is the
metadata shown beside them and used to rename files: when the picture was
taken, by which camera and lens, with what exposure, and how large the
image is. A file renamer builds names from the capture time and the
camera; an information panel shows the rest.

A camera records this metadata twice. The standard tags of TIFF and Exif
(IFD0, the Exif IFD, and the IFD that describes the raw data) are the same
in every format and documented in public standards. Each manufacturer also
writes a MakerNote, a block of its own tags whose layout differs between
brands and between generations, and parts of which are enciphered. The
format notes for [NEF](../formats/nef/makernote-and-metadata.md) and
[ARW](../formats/arw/makernote-and-metadata.md) recommended reading the
standard tags and keeping the MakerNote to what only it records.

We checked those recommendations against the public-domain (CC0) sample
files of [raw.pixls.us](https://raw.pixls.us/), 23 Nikon files from the
D1H to the Z6III and 36 Sony-family files from the DSC-F828 to the
ILCE-7M5. They showed:

- **Nikon bodies before 2018 record much of it only in the MakerNote.**
  The body serial number and the lens's focal and aperture range appear
  in the standard Exif tags only from the D780 and Z 6 (files 3827 and
  3587), and the lens model only on those and the Nikon 1 J1 (file 2956).
  Older bodies with interchangeable lenses keep the lens range, and from
  the D300 on the serial number, in the MakerNote. The D1H, D100, D70,
  D700 and Coolpix E5700 (files 5364, 2996, 2060, 2255 and 4456) write no
  Exif ISO either; the first three keep it in the MakerNote.
- **The Nikon visible image size is only in the MakerNote** (its
  CropArea tag), from the D850, D780 and Z bodies on. Nikon writes no
  standard crop tags, and no checked NEF has the Exif pixel
  dimensions.
- **Nikon's time-zone setting gives the offset from UTC.** The offset of
  the capture time from UTC is in the Exif tags only from the D850
  (file 1835). The MakerNote's WorldTime records the camera's time zone
  and whether daylight saving time is on; wherever both exist, the zone
  plus the daylight hour equals the Exif offset, with daylight saving on
  and off (files 1835, 3827, 3587, 6887, 7808 to 7810 and the Z 9 files
  5146 to 5148). It is the offset of the camera's clock, not a "home
  zone" as the NEF notes feared. The bodies from the D300 to the D810
  write WorldTime and no Exif offset, so the two cannot be compared
  there; we take the tag to mean the same on them.
- **The Sony MakerNote adds no lens range the standard tags lack.**
  Every lens range it holds is also in the Exif lens tags, and on the
  RX100 family it is the 35 mm equivalent (24 to 200 mm on the RX100M6,
  files 3012 and 3015) where Exif gives the real focal lengths (9 to
  72 mm). Only the ILCE-1 and ILCE-7M5 samples write a serial number in
  the Exif tags (files 4466 and 8846 to 8850); the other Sony samples
  write none there, and what the MakerNote holds instead is known only
  from other projects' documentation, which reports a serial in a plain
  tag on one ILCE-9 firmware and an internal serial in an enciphered
  block.
- Cameras write fractions without reducing them: an exposure time of
  `10/3200` s, an exposure bias of `0/6`. Text is sometimes padded with
  spaces (the DSLR-A100's Make is `SONY` and eleven spaces, file 2346),
  the longest text value is 65 bytes, and a manual lens on a Sony body
  is named `----` (file 1582).

### The camera settings

Added on 2026-10-24 (#26). Beside the exposure, a viewer's information
panel shows how the camera was set: the white balance, the exposure
program, the metering mode, the flash, the shutter, the image area,
the raw image size and the colour space, with the focal length in
35 mm terms and the software that wrote the file. We checked where
files record them on the 79 public sample files (35 Nikon raw files,
36 of the Sony family and eight camera JPEGs) and, as counts only, on
the maintainer's own files of six Nikon DSLRs and an ILCE-7M2; the
format notes for [NEF](../formats/nef/makernote-and-metadata.md),
[ARW](../formats/arw/makernote-and-metadata.md) and
[JPEG](../formats/jpeg/container-and-metadata.md) hold the details.
They showed:

- **Exif records half of them, on almost every file.** The exposure
  program, the metering mode, the flash and the 35 mm focal length are
  Exif tags on nearly every sample; the D1 family, the Coolpix E5700,
  the Sony F828 and R1 and a Canon JPEG lack one or more. Exif's white
  balance says only auto or manual, with the kind of light in a tag of
  its own. Its ColorSpace says only sRGB or "uncalibrated"; Adobe RGB
  is named by the InteropIndex `R03` of a DCF option file, which
  Nikon's own colour space tag confirms on all 615 of the maintainer's
  D3200 JPEGs that have it, and the A900's settings block too, as
  ExifTool decodes it. No Nikon raw file writes Exif's ColorSpace.
- **The rest is in plain MakerNote tags.** Nikon records the white
  balance setting as text, in words that cannot be guessed from one
  another (`AUTO1`, `SUNNY`, `COOL WHT FL`), the colour temperature that
  auto white balance chose (0 under the daylight setting, the only other
  one the files show it under), a fine-tuning, the shutter used (on the
  Nikon 1 J1, the D850, the D780 and the Z bodies), the image area
  as a crop code, the raw size (from the D810 on) and the colour space.
  Sony records the white balance setting, a colour temperature (0
  unless the white balance is custom), a fine-tuning that is 0 on every
  file, and its electronic front curtain setting.
- **Some are only in enciphered or encrypted blocks.** The shutter a
  Sony body used and whether it shot in APS-C mode are only in
  enciphered blocks, as ExifTool, run on the files as a reader to
  compare with, decodes them; the electronic front curtain setting
  contradicts that block on four of the 18 files where it shows whether
  the release was electronic. Sony's plain raw-quality
  tag gives the same code to an APS-C raw and a medium one. The
  firmware version in Nikon's encrypted ShotInfo, as ExifTool decodes
  it, differs from IFD0's Software on the D810.
- **The standard and MakerNote tags agree on the settings read here**,
  with one exception: a program that rewrites a JPEG can set ColorSpace
  to "uncalibrated" and leave the camera's MakerNote colour space
  behind, as on 10 of the maintainer's rewritten JPEGs. Two MakerNote
  tags left unread disagree: Nikon's flash mode (0x0087) says a flash
  fired on hundreds of the maintainer's files whose Exif says it did
  not, and a second Sony white balance tag (0xB054) says auto on two
  files set to shade.
- **Files hold codes no table names.** ExposureProgram is 0, "not
  defined", on more than half of the maintainer's D3200, D3300 and
  D7100 files and on recent Z bodies' samples; Nikon's raw size is 0
  on 506 of the maintainer's D7200 JPEGs, presumably shot without a
  raw. Exif defines exposure
  programs 1 to 8; ExifTool's documentation lists a 9, "bulb", which
  Exif does not define and no checked file holds.
- **Software holds the camera's firmware on every camera file that
  writes it** (the Sony F828 and R1 and a Canon JPEG write none), and
  the program's name on every file a program rewrote. Exif 3.0 added
  CameraFirmware for the camera's alone; no checked file writes it.

## Decision

We will read the metadata from the standard tags, and from a
manufacturer's MakerNote only where nothing standard records it.

- **Read when the file is opened.** `open` reads the metadata together
  with the file's structure, and `RawFile::metadata` returns it without
  reading again. The values it reads lie near the start of the file:
  the Exif and MakerNote values below 105 KB on every Nikon Z sample,
  the Exif values below 44 KB on every Sony one; the raw IFD, which gives the raw size, is one that
  `open` already reads to find the previews.
- **A damaged field costs only that field.** A field whose tag is
  missing, of an unexpected type, out of range or unreadable is `None`,
  and the file still opens; only an I/O error is passed up.
- **The standard tags come first.** IFD0, the Exif IFD and the raw IFD
  give every field. On NEF, the MakerNote (in the layout the preview
  reader already reads) gives the visible image size from CropArea, and,
  only when the standard tags give no value for it, the serial number,
  the ISO, the lens range and the offset from UTC. An Exif value always
  wins. The older Nikon MakerNote layouts (the D1 family and the earliest
  Coolpix bodies) are not read; their files get the Exif fields only. No
  Sony MakerNote is read. Nothing is deciphered.
- **The capture time is DateTimeOriginal**, with its own sub-seconds
  (SubSecTimeOriginal) and offset from UTC (OffsetTimeOriginal). When
  OffsetTimeOriginal gives no offset, because it is missing or cannot be
  read, a NEF's offset is its WorldTime zone, plus an hour when daylight
  saving is on, accepted only within ±14:00, since a value rewritten in
  the wrong byte order reads as hundreds of hours. The offset is reported
  the same way whichever tag gave it. The time is never taken from
  DateTimeDigitized or from IFD0's DateTime, which editors change. An
  unset clock (blanks, spaces or zeros) or an impossible date is `None`.
  raw-files has its own `DateTime` type.
- **Sizes as stored.** The raw size is that of the largest, by pixel
  count, of the IFDs holding raw data, the first of them on a tie: a file
  can hold a small full-resolution raw image beside the real one, as the
  Coolpix E5700 sample 4456 does. The image size is the default crop
  (DefaultCropSize, which Sony writes from ARW 2.3.3 on) or, on NEF,
  CropArea, when it lies inside the raw; otherwise it is the raw size, and
  then includes the border pixels. The Exif pixel dimensions, per-camera
  tables and Sony's aspect-ratio crop are not used.
- **Numbers as stored.** Exposure time, f-number, focal length, exposure
  bias and the lens range are fractions with the numerator and
  denominator the file recorded (`Rational`, `SignedRational`), so
  `10/3200` is not silently turned into `1/320`; callers reduce them for
  display. A zero means "unknown" except for the exposure bias, where
  `0/6` is a real zero. The ISO is Exif's PhotographicSensitivity; when
  it is absent, zero (unknown), or 65535, which Exif uses for "65535 or
  more", the RecommendedExposureIndex is used, or else the ISOSpeed, each
  only when above zero, and a 65535 that neither replaces stands. On NEF,
  when none of these gives an ISO, the MakerNote's ISO is the second of
  exactly two SHORT values, when above zero; most bodies write
  `[0, ISO]`.
- **Text as written, cleaned.** Cut at the first NUL, trim spaces at both
  ends; an empty result, a value longer than 255 bytes, or a lens named
  only with `-` characters is `None`. Bytes that are not UTF-8 become
  U+FFFD.

Rejected:

- **Reading the metadata on request** (`metadata(&mut self) ->
  Result<Metadata>`). Every caller would handle an error for data that is
  already in memory.
- **An error per field** (getters returning `Result<Option<T>>`), to tell
  "not recorded" from "damaged". No caller needs the difference, and
  every caller would pay for it.
- **The standard tags and CropArea only.** A decade of Nikon bodies would
  have no serial number and no lens range, and some no ISO, although the
  files record them in plain form.
- **The Sony MakerNote** for the lens range and the model ID. It adds no
  range the Exif tags lack, and gives the wrong one on the RX100 family.
- **Falling back on DateTimeDigitized or DateTime** for the capture time.
  Every sample has DateTimeOriginal, and IFD0's DateTime is the time of
  the last edit.
- **A separate label for an offset taken from WorldTime.** The files show
  that it is the same offset.
- **Reducing fractions when read, or floating-point values.** Both lose
  what the file recorded, and a renamer would have to turn 0.003125 back
  into `1/320`.
- **Trimming only trailing spaces, with no length limit**, as other
  readers do. A leading space is padding too, and the longest measured
  value has 65 bytes, so text past 255 bytes is taken for damage.
- **Deciphering Sony's enciphered block** for older serial numbers. Its
  tables have no settled provenance.

### The camera settings

Added on 2026-10-24 (#26). We will read the camera's settings by the
rule above: the standard tags first, a manufacturer's plain MakerNote
tag where no standard tag records the value, and nothing deciphered or
decrypted. This extends the decision above. The MakerNote is read for
these settings on Nikon and Sony files, beyond what "The standard tags
come first" lists, and maker-specific values (a crop mode, as the
image area; the shutter; the raw size) enter the API, which the
consequence on maker-specific values below no longer describes in full.

- **A damaged value costs only itself**, as above: a missing tag, a
  wrong type or count, a value outside the MakerNote, or a code no
  table names gives `None`. A code is named only where a standard or a
  measured file gives its meaning.
- **The exposure program, the metering mode and the flash come from Exif
  alone**, with Exif's codes: the programs 1 to 8, the metering modes 1
  to 6 and 255, "other"; 0, "not defined" or "unknown", gives none. The
  flash is whether it fired and its mode (compulsory firing, compulsory
  suppression or auto), without red-eye or return light. The
  MakerNote's versions are not read: they repeat Exif, and Nikon's
  flash mode contradicts Exif's "fired" on hundreds of files.
- **The white balance mode comes from the MakerNote first**, since it
  records the setting itself where Exif records only auto or manual,
  and from Exif's WhiteBalance and LightSource when the MakerNote gives
  none. Nikon's text is looked up in a table of the texts files are
  known to hold, whole, not by its first letters, and a text the table
  lacks gives none; Sony's is 0x0115, never 0xB054. The modes form one
  short list in neutral words (auto, daylight, cloudy, shade, tungsten,
  fluorescent, flash, underwater, custom, colour temperature), into
  which each maker's finer kinds fold.
- **The colour temperature** is the one the MakerNote records, when it
  is above zero; zero means not recorded. On Nikon (0x004F) it is the
  temperature auto white balance chose, on Sony (0xB021) the one a
  custom white balance set, so the mode beside it says which.
- **The fine-tuning** is Nikon's shift along two axes (0x003F, else
  0x000B), as stored, a recorded zero included. Sony's is none until a
  file shows what its sign and its axis mean.
- **The shutter used** is Nikon's 0x0034, whose auto codes give the
  shutter the camera chose; a movie code gives none. Sony's is none,
  since only an enciphered block records it. Sony's electronic front
  curtain setting (0x201A) is a value of its own, never taken for the
  shutter used.
- **The image area** is Nikon's crop code, the first value of
  CropHiSpeed, in words that fit any maker (full frame, APS-C, 1:1,
  16:9, 5:4, 1.3x and 2.7x crops, and the APS-C forms of 1:1 and 16:9).
  "Off" gives none, since it does not say which sensor the body has, and
  so do the 3:2 crop and the codes ExifTool names for movies, except
  two that DX bodies write on still files: the "DX movie 16:9 crop" is
  the APS-C 16:9 crop, and the "1.3x movie crop" the 1.3x crop. Sony's
  is none.
- **The raw image size** (large, medium or small) is Nikon's 0x003E on
  a raw file. A JPEG gives none, whatever its MakerNote says, since it
  holds no raw; a Sony file gives none, since its tag gives an APS-C
  raw the code of a medium one.
- **The colour space** is Exif's whenever Exif records ColorSpace: 1 is
  sRGB, 65535 with InteropIndex `R03` is Adobe RGB, and 65535 otherwise
  is none, so that a program's "uncalibrated" is never overruled by a
  MakerNote it left behind. Only when Exif records none, as on every
  Nikon raw file, does Nikon's 0x001E give sRGB or Adobe RGB.
- **The focal length in 35 mm terms** is Exif's; 0 is none.
- **The software** is IFD0's Software as written, cleaned like every
  text: the camera's firmware, or the program that last wrote the file.
  Exif's CameraFirmware is not read, since no checked file writes it.

Rejected for the camera settings:

- **Deciphering the Sony blocks that record the shutter used and APS-C
  mode, or decrypting Nikon's ShotInfo** for its firmware and image
  area. Their tables have no settled provenance, as for the serial
  numbers above; the image area that ExifTool, run as a reader to
  compare with, decodes from ShotInfo contradicts the crop code on two
  of the four Nikon files it decodes it on, and the APS-C flag it
  decodes is wrong on one Sony file.
- **A variant holding a code no table names** (an `Unknown(u16)`). A
  caller could do nothing with the number but print it, every match
  would have to handle it, and a code can be named later without
  breaking callers once a file shows what it means. The same holds for
  ExifTool's "bulb" program, which Exif does not define.
- **Sony's electronic front curtain setting as the shutter used.** It
  contradicts the enciphered record of the release, as ExifTool decodes
  it, on four of the 18
  files that have one, and on the other 14 that record does not say
  which shutter was used.
- **Mapping Nikon's white balance texts by their first letters.** A text
  such as `COOL WHT FL` shows that they cannot be predicted.
- **Letting the MakerNote's colour space overrule Exif's.** A rewritten
  JPEG can keep a stale one.

## Consequences

- Damaged metadata never costs the file or its previews, and a damaged
  field never costs the others.
- Opening a file reads a little more than before, close to what it
  already read, and nothing more is read later for the metadata.
- The serial number of most Sony bodies, which they do not write in the
  Exif tags, stays `None`; so do the ISO and lens range of the D1
  family, whose older MakerNote layout is not read.
- On the Nikon bodies from the D300 to the D810, the offset from UTC rests
  on WorldTime meaning there what it means on later bodies. If one of them
  recorded a home zone instead, pictures taken while travelling would get
  a wrong offset; no sample can show it, since those bodies write no Exif
  offset to compare with.
- On a body without a recorded crop, the image size includes the border
  pixels; a caller cannot tell such a size from a recorded one.
- The image size counts stored pixels, which on a Nikon small raw can
  differ from the size the camera's menu names: it is 6192×4128 on the
  D850's small raw (file 1835), whose menu size is 4128×2752.
- The image size is the default crop, not the aspect ratio chosen in the
  camera, which Sony records only in tags of its own.
- Maker-specific values (Sony's model ID, Nikon's shutter count or crop
  mode) are not in the API; they can be added later without breaking it.
- DNG files, which use the same standard tags and the same default crop,
  will use the same reader; a new format fills the same `Metadata`.
- `DateTime` converts to other crates' date types only through its
  getters until a caller needs a conversion feature.

### The camera settings

Added on 2026-10-24 (#26).

- A viewer gets most of the camera's settings from the NEF, ARW and JPEG
  files of the bodies checked. What only an enciphered block records
  stays `None` on every Sony file: the shutter used and APS-C mode;
  Sony's raw size and fine-tuning do too. Nikon DSLRs before the D850
  have no shutter, Nikon bodies before the D5 and D850 no colour
  temperature, and the NEF files of the D1 family, the D70, the D100
  and the Coolpix E5700 and P7800 no image area and no colour space.
- Nikon's table of white balance texts grows only from measured files:
  a text not in it yet gives way to Exif's white balance, which says
  only auto or the kind of light, and never to a guess from its
  letters.
- Codes that a later body or a later Exif adds give `None` until they
  are named, which adds to the types without breaking them.
- The software names the camera's firmware or the program that last
  wrote the file, and a caller cannot tell which without reading the
  text.
