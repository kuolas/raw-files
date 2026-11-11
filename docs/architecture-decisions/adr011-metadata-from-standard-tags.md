# ADR011: Metadata comes from the standard tags

- Status: Accepted
- Date: 2026-10-15
- Revision: 3, 2026-11-09
- Issue: #13; the camera settings #26, revised in #51; authorship and
  location #97

## Context

A culler shows a file's metadata beside its previews and renames files
by it: when the picture was taken, by which camera and lens, with what
exposure, and how large the image is. A viewer's information panel also
shows how the camera was set, who made the picture and where it was
taken.

A camera records this metadata twice. The standard tags of TIFF and
Exif are the same in every format and documented in public standards.
Each maker also writes a MakerNote, a block of its own tags. Its layout
differs between brands and between generations, and parts of it are
enciphered.

We checked where files record each value, on the public samples of
[raw.pixls.us](https://raw.pixls.us/), on camera JPEGs from Wikimedia
Commons, and on my own files and frames.
The format notes hold the measurements: the
[NEF notes](../formats/nef/makernote-and-metadata.md#8-viewer-metadata),
the [ARW notes](../formats/arw/makernote-and-metadata.md#9-viewer-metadata)
and the [JPEG notes](../formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment).
In short:

- Nikon bodies before the D780 and the Z 6 record the serial number,
  the lens range and on some bodies the ISO only in the MakerNote, if
  at all. Its CropArea is the only record of the visible size, on the
  bodies that write one, and its WorldTime gives the offset from UTC
  where Exif has none.
- Sony's MakerNote adds no lens range that the Exif tags lack. On the
  RX100 family it gives the 35 mm equivalent instead of the real one.
- Exif records the exposure program, the metering mode, the flash and
  the focal length in 35 mm terms. For the white balance it records
  only auto or manual, and for the color space only sRGB or
  "uncalibrated".
- Plain MakerNote tags record the white balance setting, its color
  temperature and its fine-tuning, the shutter and the image area. Sony
  records the shutter used and APS-C mode only in enciphered blocks, as
  ExifTool, run on the files as a reader to compare with, decodes them.
- The authorship texts are plain ASCII on every file counted, and
  cameras fill unset ones with NULs or spaces. A GPS IFD seldom holds a
  position.
- On the Nikon Z and Sony samples measured, the values read here lie
  near the start of the file. Every text and GPS value counted lies
  inside the blocks that opening a file already reads.

## Decision

We will read the metadata, the camera's settings among it, from the
standard tags. A maker's plain MakerNote tags are read only where
nothing standard records a value. Nothing is deciphered or decrypted.

- **Read when the file is opened.** `open` reads the metadata together
  with the file's structure, and `RawFile::metadata` returns it without
  reading again. On the Nikon Z and Sony samples measured, its values
  lie near the start of the file. The raw IFD, which gives the raw
  size, is one `open` already reads to find the previews.
- **A damaged field costs only that field.** A missing tag, a wrong
  type or count, a value out of range or unreadable, a MakerNote value
  outside the MakerNote, or a code no table names gives `None`, and the
  file still opens. Only an I/O error is
  passed up. A code is named only where a standard or a measured file
  gives its meaning.
- **The standard tags come first.** IFD0, the Exif IFD and the raw IFD
  give every field, and an Exif value always wins. On NEF, the
  MakerNote gives the visible image size from CropArea. Only where the
  standard tags have no value, it gives the serial number, the ISO, the
  lens range and the offset from UTC
  ([NEF notes §10](../formats/nef/makernote-and-metadata.md#10-recommendation-for-raw-files)).
  The short-header MakerNote of the earliest Coolpix bodies is not
  read, so their files get the Exif fields only. Sony's MakerNote gives
  none of these fields. On both, the MakerNote is read for the focus
  point ([ADR012](adr012-focus-points-as-the-camera-records-them.md))
  and for the camera settings that no standard tag records.
- **The capture time is DateTimeOriginal**, with SubSecTimeOriginal and
  OffsetTimeOriginal. When OffsetTimeOriginal gives no offset, a NEF's
  offset is its WorldTime zone, plus an hour under daylight saving
  time. It is accepted only within ±14:00, since a value rewritten in
  the wrong byte order reads as hundreds of hours. The offset is
  reported the same way whichever tag gave it. DateTimeDigitized and
  IFD0's DateTime, which editors change, are never used. An unset clock
  or an impossible date is `None`. raw-files has its own `DateTime`
  type.
- **Sizes as stored.** The raw size is that of the largest IFD holding
  raw data, by pixel count, the first on a tie. A file can hold a small
  full-resolution raw image beside the real one, as the Coolpix E5700
  sample does. The image size is the default crop (DefaultCropSize) or,
  on NEF, CropArea, when it lies inside the raw. Otherwise it is the
  raw size, border pixels included. The Exif pixel dimensions,
  per-camera tables and Sony's aspect-ratio crop are not used.
- **Numbers as stored.** The exposure time, the f-number, the focal
  length, the exposure bias and the lens range keep the numerator and
  denominator the file recorded (`Rational`, `SignedRational`).
  `10/3200` is not turned into `1/320`; callers reduce for display. A
  zero means unknown, except in the exposure bias, where `0/6` is a
  real zero.
- **The ISO is Exif's PhotographicSensitivity.** When it is absent, 0,
  or 65535 (Exif's "65535 or more"), RecommendedExposureIndex is used,
  else ISOSpeed, each only when above 0. A 65535 that neither replaces
  stands. On NEF, when none of these gives an ISO, the MakerNote's is
  used: the second of exactly two SHORT values, when above 0
  ([NEF notes §8.5](../formats/nef/makernote-and-metadata.md#85-serial-number-iso-and-lens)).
- **Text as written, cleaned.** A text is cut at the first NUL and
  trimmed of spaces at both ends. An empty result, a value longer than
  255 bytes, or a lens named only with `-` characters is `None`. Bytes
  that are not UTF-8 become U+FFFD.
- **A text of either type where Exif allows both.** A text that Exif
  3.1 gives as "ASCII or UTF-8" is read from a value of either type
  ([tag-names.md §2](../formats/tag-names.md#2-field-types) lists
  them). Every other text, and every MakerNote text, is read only
  from type ASCII.
- **A description and a comment are free text.** They have no 255-byte
  limit, only the cap on any value read (`Limits`), and they keep their
  line breaks.
- **A camera's placeholder is no description.** A text that measured
  files show a camera writing in ImageDescription by itself is `None`,
  as a lens named only with dashes is. The list holds `SONY DSC`. It
  matches the whole cleaned text, whatever the Make, and grows only
  from measured files.
- **The exposure program, the metering mode and the flash come from
  Exif alone**, with Exif's codes. 0, "not defined" or "unknown", gives
  none. The flash is whether it fired and its mode (compulsory firing,
  compulsory suppression or auto), without red-eye or return light. The
  MakerNote's versions are not read: they repeat Exif, and Nikon's
  flash mode contradicts Exif's "fired" on hundreds of files.
- **The white balance mode comes from the MakerNote first**, since it
  records the setting itself, else from Exif's WhiteBalance and
  LightSource.
  - Nikon's text is looked up whole in a table of the texts files are
    known to hold, and a text the table lacks gives none. A chosen
    color temperature, a number followed by `K`, is the one text read
    by its form, since no table could list every number.
  - Sony's is 0x0115, never 0xB054, which says auto on files set to
    shade.
  - The modes form one short list in neutral words (auto, daylight,
    cloudy, shade, tungsten, fluorescent, flash, underwater, custom,
    color temperature). Each maker's finer kinds fold into it.
- **The color temperature** is the one the MakerNote records, from 1
  to 99,999 K. Zero means not recorded, and a larger number is no
  temperature a camera is set to. Nikon's is the temperature of the
  white balance the camera applied. When that tag gives none, the
  number of a chosen color temperature's text stands in. Sony's is the
  one chosen under the color-temperature setting, or the custom white
  balance's own; the mode beside it says which.
- **The fine-tuning** is the shift along two axes, amber to blue and
  green to magenta, in the camera's own steps, a recorded zero
  included. Each value is signed as its axis is named, positive towards
  blue and towards magenta. So the reader turns Nikon's green-magenta
  value and Sony's amber-blue value around. The single value the oldest
  Nikon bodies write gives none, since no file shows its axis or sign.
- **The shutter used** is Nikon's ShutterMode, whose auto codes give
  the shutter the camera chose; a movie code gives none. Sony's is
  none, since only an enciphered block records it. Sony's electronic
  front curtain setting is a value of its own, never taken for the
  shutter used.
- **The image area** is Nikon's crop code, CropHiSpeed's first value,
  in words that fit any maker: full frame, APS-C, 1:1, 16:9, 5:4, 1.3x
  and 2.7x crops, and the APS-C forms of 1:1 and 16:9. "Off" gives
  none, since it does not say which sensor the body has. So do the 3:2
  crop and the movie codes, except two that DX bodies write on stills
  ([NEF notes §10](../formats/nef/makernote-and-metadata.md#10-recommendation-for-raw-files)).
  Sony's is none.
- **The raw image size** (large, medium or small) is Nikon's
  ImageSizeRAW on a raw file. A JPEG gives none, whatever its MakerNote
  says, since it holds no raw. A Sony file gives none, since its tag
  gives an APS-C raw the code of a medium one.
- **The color space** is Exif's whenever Exif records ColorSpace. 1 is
  sRGB, 65535 with InteropIndex `R03` is Adobe RGB, and any other 65535
  is none. So a program's "uncalibrated" is never overruled by a
  MakerNote it left behind. Only when Exif records none, as on every
  Nikon raw file, does Nikon's MakerNote give sRGB or Adobe RGB.
- **The focal length in 35 mm terms** is Exif's; 0 is none.
- **The software** is IFD0's Software, cleaned like every text: the
  camera's firmware, or the program that last wrote the file. Exif's
  CameraFirmware is not read, since no checked file writes it.
- **Authorship is a value of its own**, of seven texts: the artist
  (IFD0's Artist), the copyright notice (Copyright), the description
  (ImageDescription), and from the Exif IFD the photographer
  (Photographer), the editor (ImageEditor), the title (ImageTitle) and
  the comment (UserComment). IFD0's texts are read from IFD0 alone, not
  from the copies Sony writes in IFD1 and IFD2. Nothing is read from a
  MakerNote.
- **A comment is read under the ASCII code alone.** The text after
  UserComment's 8-byte code `ASCII` is cleaned as every text. Under any
  other code, or when the value is no longer than its code, the comment
  is `None`.
- **The camera's owner is the camera's.** CameraOwnerName names whoever
  owns the body, whoever takes the picture, and Exif says it shall in
  principle not be rewritten (4.6.6.9.2). It is read with the camera,
  beside its serial number.
- **A location needs a sound position.** The GPS IFD must record
  GPSLatitudeRef `N` or `S`, GPSLongitudeRef `E` or `W`, and
  GPSLatitude and GPSLongitude as three RATIONAL values each. No
  denominator may be zero, minutes and seconds must be below 60, and
  the latitude may not pass 90° or the longitude 180°. Without such a
  position, no part of the location is given, whatever else the GPS IFD
  records.
- **Coordinates as recorded.** A coordinate keeps the three fractions
  the file records, its degrees, minutes and seconds, and its
  hemisphere. Decimal degrees, negative to the south and west, are
  computed when asked for.
- **The location's other parts** are each read from both their tags,
  and a part whose tags are missing or damaged is `None` alone.
  - The altitude is GPSAltitude in meters as recorded, above
    (GPSAltitudeRef 0 or 2) or below (1 or 3) its reference, without
    naming the surface it is measured from.
  - The time is GPSDateStamp and GPSTimeStamp, one time in UTC. It
    needs whole hours and minutes, and a date and time that exist. Its
    seconds' denominator must be a power of ten, so that the fraction
    of a second the file records is kept.
  - The image direction is GPSImgDirection, below 360°, from the north
    that GPSImgDirectionRef names, `T` true or `M` magnetic. There is
    no default when the reference is missing.
  - The map datum is GPSMapDatum, cleaned as every text.
  - GPSVersionID, GPSStatus and the other GPS tags are not read.
- **As recorded, nothing redacted.** The authorship texts, the owner
  and the location name people and places. raw-files gives them as the
  file records them, with no option to hide them, as `RawFile::entries`
  does ([ADR023](adr023-every-entry-a-file-records.md)). Tests on
  public sample files check them by presence and shape only. Tests on
  my own files print counts only.

Rejected:

- **Reading the metadata on request** (`metadata(&mut self) ->
  Result<Metadata>`). Every caller would handle an error for data that
  is already in memory.
- **An error per field** (getters returning `Result<Option<T>>`), to
  tell "not recorded" from "damaged". No caller needs the difference,
  and every caller would pay for it.
- **The standard tags and CropArea only.** A decade of Nikon bodies
  would have no serial number and no lens range, and some no ISO,
  though their files record them in plain form.
- **The Sony MakerNote for the lens range and the model ID.** It adds
  no range the Exif tags lack, and gives the wrong one on the RX100
  family.
- **Reading no Sony MakerNote at all**, as this record first had it.
  The focus point and the camera settings that only the MakerNote
  records need it.
- **Falling back on DateTimeDigitized or DateTime** for the capture
  time. Every sample has DateTimeOriginal, and IFD0's DateTime is the
  time of the last edit.
- **A separate label for an offset taken from WorldTime.** The files
  show that it is the same offset.
- **Reducing fractions when read, or floating-point values.** Both lose
  what the file recorded. A renamer would have to turn 0.003125 back
  into `1/320`.
- **Trimming only trailing spaces, with no length limit**, as other
  readers do. A leading space is padding too. The longest measured
  value has 65 bytes, so a text past 255 bytes is taken for damage,
  except a description or a comment, which are free text.
- **Deciphering Sony's enciphered block** for older serial numbers. Its
  tables have no settled provenance.
- **Deciphering the Sony blocks that record the shutter used and APS-C
  mode, or decrypting Nikon's ShotInfo** for its firmware and image
  area. Their tables have no settled provenance. And what ExifTool, run
  as a reader to compare with, decodes from them is wrong on some
  files: ShotInfo's image area contradicts the crop code on two of four
  Nikon files, and the APS-C flag is wrong on one Sony file.
- **A variant holding a code no table names** (an `Unknown(u16)`). A
  caller could do nothing with the number but print it, and every match
  would have to handle it. A code can be named later, without breaking
  callers, once a file shows what it means. The same holds for
  ExifTool's "bulb" program, which Exif does not define.
- **Sony's electronic front curtain setting as the shutter used.** It
  contradicts the enciphered record of the release, as ExifTool decodes
  it, on four of the 18 files that have one. On the other 14 that
  record does not say which shutter was used.
- **Mapping Nikon's white balance texts by their first letters.** A
  text such as `COOL WHT FL` shows that they cannot be predicted.
- **The fine-tuning signed as each maker signs it.** A viewer showing
  or comparing two files' shifts would need to know, for each maker,
  which way a positive value points.
- **Sony's 0x0112 as the fine-tuning**, the tag ExifTool's
  documentation names so. It records the kind of fluorescent lamp, and
  stays 0 when the photographer shifts the white balance.
- **Letting the MakerNote's color space overrule Exif's.** A rewritten
  JPEG can keep a stale one.
- **The authorship texts flat on the metadata, or split**, the artist
  and the copyright in a value and the description and the comment
  flat. Seven more getters would crowd a type whose other values are
  grouped. A split would put the answers to one question, who made the
  picture and what they wrote about it, in two places.
- **The names `Gps` and `Position`.** Exif says its GPS tags also hold
  positions from other satellite systems and from mobile networks
  (4.6.7). And the value holds more than a position: an altitude, a
  time, a direction and a datum.
- **Decimal degrees only, or no decimal degrees.** Degrees alone lose
  what the file records, and a floating-point value would keep the
  metadata from being compared and hashed. Fractions alone make every
  program that shows a map compute the same degrees.
- **A typed value for every GPS tag, or also for those cameras write**:
  the satellites, the status, the measure mode. No measured camera file
  writes most of the tags, and the rest describe the receiver rather
  than the picture. `RawFile::entries` gives them all.
- **Reading the Unicode code as UTF-8, and text under the undefined
  code.** Exif defines the Unicode code as UTF-8 (Table 4), but no
  measured file shows what writers put there. Exif leaves the undefined
  code to the reader's locale (Table 5). Either reading would be a
  guess, and `RawFile::entries` gives the bytes.
- **A placeholder given as recorded.** A viewer would show `SONY DSC`
  as the description of every picture those bodies took.
- **The 255-byte limit for every text, or a fixed 64 KiB for the free
  texts.** A program's description is ordinary text at any length, not
  damage. A second number beside the cap on any value would make a
  value readable one way and not the other.
- **The UTF-8 type for the new texts only, or for none.** Make and
  Model would be read by a different rule from Artist in the same IFD.
  With UTF-8 for none, a file that follows Exif 3.0 would lose its
  UTF-8 texts as damaged.
- **A redaction option.** raw-files gives what the file records. A
  program that hides a name or a place decides what to hide, as with
  `RawFile::entries`.
- **The surface of the altitude named, or signed meters as a
  floating-point value.** Exif 3.0 changed the meaning of 0 and 1, and
  every measured file records 0 under an older GPS version. So a
  surface would be guessed from a version number. A floating-point
  value loses what the file records.
- **The photographer, the editor, the title and the owner left until a
  file records one.** The standard defines each as plainly as the
  artist, so a file could add nothing their reading needs. Their
  readers are checked on built files, since no measured file records
  one.
- **The owner with the authorship.** The owner is a fact about the
  body, set once in the camera and the same on every picture it takes,
  whoever takes it, as the serial number is. The photographer, the
  editor and the title are facts about the picture.

## Consequences

- Damaged metadata never costs the file or its previews, and a damaged
  field never costs the others.
- Opening a file reads a little more than its structure alone, close
  to what that already reads, and nothing more is read later for the
  metadata.
- Most Sony bodies write no serial number in the Exif tags, so theirs
  stays `None`.
- On the Nikon bodies from the D300 to the D810, the offset from UTC
  rests on WorldTime meaning there what it means on later bodies. If
  one of them recorded a home zone instead, pictures taken while
  traveling would get a wrong offset. No sample can show it, since
  those bodies write no Exif offset to compare with.
- On a body without a recorded crop, the image size includes the
  border pixels. A caller cannot tell such a size from a recorded one.
- The image size counts stored pixels. On a Nikon small raw, that can
  differ from the size the camera's menu names
  ([NEF notes §8.4](../formats/nef/makernote-and-metadata.md#84-image-dimensions)).
- The image size is the default crop, not the aspect ratio chosen in
  the camera, which Sony records only in tags of its own.
- Some maker-specific values enter the API, in the camera settings: a
  crop mode as the image area, the shutter, and the raw size. Others,
  such as Sony's model ID and Nikon's shutter count, are not in the
  API, and can be added later without breaking it.
- DNG files, which use the same standard tags and the same default
  crop, will use the same reader. A new format fills the same
  `Metadata`.
- `DateTime` converts to other crates' date types only through its
  getters, until a caller needs a conversion feature.
- A viewer gets most of the camera's settings from the NEF, ARW and
  JPEG files of the bodies checked. What only an enciphered block
  records stays `None`, and so does what an older body does not record.
  The notes list both by body
  ([NEF notes §8.7](../formats/nef/makernote-and-metadata.md#87-camera-settings),
  [ARW notes §9.7](../formats/arw/makernote-and-metadata.md#97-camera-settings)).
- Nikon's table of white balance texts grows only from measured files.
  A text not in it yet gives way to Exif's white balance, which says
  only auto or the kind of light, and never to a guess from its
  letters.
- Codes that a later body or a later Exif adds give `None` until they
  are named, which adds to the types without breaking them.
- The software names the camera's firmware or the program that last
  wrote the file. A caller cannot tell which without reading the text.
- A comment under the JIS, Unicode or undefined code is `None`, and
  `RawFile::entries` gives its bytes. No measured file has text under
  one.
- The GPS tags not read, such as the satellites, the status and the
  measure mode, stay in `RawFile::entries`
  ([NEF notes §8.9](../formats/nef/makernote-and-metadata.md#89-location),
  [ARW notes §9.9](../formats/arw/makernote-and-metadata.md#99-location)).
- Which surface an altitude is measured from is not given. A GPS IFD
  without a sound position gives no time, altitude, direction or datum,
  even where it records them, as a Sony file without a fix records its
  datum.
- A text that only IFD1, IFD2 or a MakerNote holds gives none. No
  measured file holds one there and not in IFD0.
- The photographer, the editor, the title and the owner are read by
  the standard's text alone, and checked on built files only, until a
  file records one.
- A placeholder that another body writes is given as a description
  until a measured file shows it and the list gains it.
- A program that shows these values shows people's names and the
  places they were. raw-files leaves what to show to it.
