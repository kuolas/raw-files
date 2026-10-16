# ADR011: Metadata comes from the standard tags

- Status: Accepted
- Date: 2026-10-15
- Issue: #13

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
  (SubSecTimeOriginal) and offset from UTC (OffsetTimeOriginal). Without
  OffsetTimeOriginal, a NEF's offset is its WorldTime zone, plus an hour
  when daylight saving is on, accepted only within ±14:00, since a value
  rewritten in the wrong byte order reads as hundreds of hours. The
  offset is reported the same way whichever tag gave it. The time is
  never taken from DateTimeDigitized or from IFD0's DateTime, which
  editors change. An unset clock (blanks, spaces or zeros) or an
  impossible date is `None`. raw-files has its own `DateTime` type.
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
