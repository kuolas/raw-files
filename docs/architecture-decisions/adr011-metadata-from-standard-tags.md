# ADR011: Metadata comes from the standard tags

- Status: Accepted
- Date: 2026-10-15
- Revision: 3, 2026-11-09
- Issue: #13; the camera settings #26, revised in #51; authorship and
  location #97

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

Beside the exposure, a viewer's information
panel shows how the camera was set: the white balance, the exposure
program, the metering mode, the flash, the shutter, the image area,
the raw image size and the color space, with the focal length in
35 mm terms and the software that wrote the file. We checked where
files record them on the 79 public sample files (35 Nikon raw files,
36 of the Sony family and eight camera JPEGs), as counts only on my
own files of six Nikon DSLRs and an ILCE-7M2, and on frames I shot
with a Z 9, a D600 and the ILCE-7M2 under
each white balance setting and fine-tuning, to see what the tags hold
under each; the format notes for
[NEF](../formats/nef/makernote-and-metadata.md),
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
  Nikon's own color space tag confirms on all 615 of my
  D3200 JPEGs that have it, and the A900's settings block too, as
  ExifTool decodes it. No Nikon raw file writes Exif's ColorSpace.
- **The rest is in plain MakerNote tags.** Nikon records the white
  balance setting as text, in words that cannot be guessed from one
  another (`AUTO1`, `SUNNY`, `COOL WHT FL`), and a chosen color
  temperature as its number (`3230K`); the color temperature of the
  white balance it applied, under every setting on the Z 9 but only
  under auto on the D850 and the Z fc, which write 0 under daylight; a
  fine-tuning; the shutter used (on the Nikon 1 J1, the D850, the D780
  and the Z bodies); the image area as a crop code; the raw size (from
  the D810 on) and the color space. Sony records the white balance
  setting; a color temperature, the one chosen under its
  color-temperature setting and the custom white balance's own under
  custom, 0 under every other setting; a fine-tuning along both axes in
  one tag, 0x2014, while 0x0112, which ExifTool's documentation names a
  fine-tuning, holds the kind of fluorescent lamp; and its electronic
  front curtain setting.
- **The two makers sign the fine-tuning the opposite way.** A positive
  value shifts towards blue and towards green on Nikon, towards amber
  and towards magenta on Sony. Each maker's own software, run on the
  frames shot with each shift as a reader to compare with, names the
  shift each value records, and the camera's own pictures agree: they
  are warmer, cooler, greener or more magenta than one shot without a
  shift or with the opposite one.
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
  with two exceptions: Sony's underwater white balance, which its Exif
  WhiteBalance calls auto; and a program that rewrites a JPEG can set
  ColorSpace to "uncalibrated" and leave the camera's MakerNote color
  space behind, as on 10 of my rewritten JPEGs. Two
  MakerNote tags left unread disagree: Nikon's flash mode (0x0087) says
  a flash fired on hundreds of my files whose Exif says
  it did not, and a second Sony white balance tag (0xB054) says auto on
  two files set to shade.
- **Files hold codes no table names.** ExposureProgram is 0, "not
  defined", on more than half of my D3200, D3300 and
  D7100 files and on recent Z bodies' samples; Nikon's raw size is 0
  on 506 of my D7200 JPEGs, presumably shot without a
  raw. Exif defines exposure
  programs 1 to 8; ExifTool's documentation lists a 9, "bulb", which
  Exif does not define and no checked file holds.
- **Software holds the camera's firmware on every camera file that
  writes it** (the Sony F828 and R1 and a Canon JPEG write none), and
  the program's name on every file a program rewrote. Exif 3.0 added
  CameraFirmware for the camera's alone; no checked file writes it.

A viewer also shows who made a picture and where it was taken: the
artist, the copyright notice, a title, a description or a comment, and
the place on a map. Exif records them in tags of IFD0, the Exif IFD and
the GPS IFD (Exif 3.1, CIPA DC-008-2026: 4.6.5.4, 4.6.6.4.2, 4.6.6.9
and 4.6.7). We counted what files record in them on 9 November 2026,
with a counting program of my own over raw-files' own view of every
entry ([ADR023](adr023-every-entry-a-file-records.md)), written from
Exif 3.1; no other reader was run. It opened 57,786 files: 515 public
ones (the 507 NEF, ARW and JPEG files of raw.pixls.us and the eight
camera JPEGs from Wikimedia Commons) and, as counts only, 57,271 of my
own (a Z 9, six Nikon DSLRs, an ILCE-7M2, Canon and Fujifilm cameras,
and files a program rewrote). The format notes for
[NEF](../formats/nef/makernote-and-metadata.md),
[ARW](../formats/arw/makernote-and-metadata.md) and
[JPEG](../formats/jpeg/container-and-metadata.md) hold the details.
They showed:

- **Every text is plain ASCII.** Artist, Copyright and
  ImageDescription are of type ASCII on every file, with no byte above
  0x7F. No file uses the UTF-8 type that Exif 3.0 added and that Exif
  3.1 allows beside ASCII for sixteen texts, among them Make, Model,
  Software and the lens's make and model, but not the serial numbers.
  Cleaned, the longest texts are an Artist of 36 bytes, a Copyright of
  34 and a description of 176, and the longest comment holds 176 bytes
  after its code; a program wrote both, the description with control
  characters.
- **Cameras write fields of a fixed length, and blanks.** Nikon writes
  Artist in 37 bytes and Copyright in 55, a NUL first or all spaces
  when they are not set; Sony an ImageDescription of 32 spaces on all
  but its older bodies' files, with copies in IFD1 and IFD2; a Canon DSLR an Artist, a
  Copyright and a CameraOwnerName of one NUL; Fujifilm compacts a
  Copyright of four spaces.
- **Older Sony bodies describe every picture as `SONY DSC`**, on 24
  files (23 public ARWs and a camera JPEG); newer ones write the
  spaces.
- **A comment is ASCII or blank.** UserComment starts with 8 bytes
  naming its characters: ASCII, JIS, Unicode (UTF-8) or undefined
  (4.6.4 and Table 4). There is text under the ASCII code only, on
  11,319 files, 11,277 of them Nikon's; Nikon cameras write the code
  and 36 bytes of NULs, spaces or text. The undefined code is followed by
  NULs and spaces alone, and no file uses JIS or Unicode.
- **No file holds text in CameraOwnerName, Photographer, ImageEditor
  or ImageTitle.** CameraOwnerName is blank on the 138 files that have
  it, all Canon's; the other three, added in Exif 3.0, are in no file.
- **A GPS IFD is seldom a position.** 46,250 files hold a GPS IFD and
  548 a position. Nearly every Nikon file, and the Canon DSLR's, holds
  one of a single entry, its version; a Sony body without a fix writes
  four (the version, the status `V`, the datum and the differential
  correction); three Coolpix files write every position tag with blank
  references and zero denominators.
- **Each maker writes a position its own way.** Nikon writes degrees
  and minutes with four decimals (`d/1, m/10000, 0/1`), Sony degrees,
  minutes and seconds with three (`d/1, m/1, s/1000`), a phone whole
  seconds; the GPS time has hundredths of a second on Nikon and
  thousandths on Sony. Every one of the 531 files with an altitude
  records the reference 0, under GPS version 2.3, though
  Exif 3.0 changed what 0 means, from sea level to the ellipsoid, and
  added 2 and 3 for sea level (4.6.7.1.6). The datum is `WGS-84` on
  Sony's files with a position and blank on all but 3 of Nikon's; one public file records the image
  direction, from magnetic north.
- **Nothing here costs a read.** Every text and GPS value measured lies
  inside the blocks that opening the file already reads.

## Decision

We will read the metadata, the camera's settings among it, from the
standard tags, and from a manufacturer's plain MakerNote tags only where
nothing standard records a value; nothing is deciphered or decrypted.

- **Read when the file is opened.** `open` reads the metadata together
  with the file's structure, and `RawFile::metadata` returns it without
  reading again. The values it reads lie near the start of the file:
  the Exif and MakerNote values below 105 KB on every Nikon Z sample,
  the Exif values below 44 KB on every Sony one; the raw IFD, which gives the raw size, is one that
  `open` already reads to find the previews. The authorship texts and
  the GPS IFD's values lie inside the blocks `open` reads on every file
  counted.
- **A damaged field costs only that field.** A field whose tag is
  missing, of an unexpected type, out of range or unreadable is `None`,
  and the file still opens; only an I/O error is passed up. So does a
  camera setting: a missing tag, a wrong type or count, a value outside
  the MakerNote, or a code no table names gives `None`. A code is named
  only where a standard or a measured file gives its meaning.
- **The standard tags come first.** IFD0, the Exif IFD and the raw IFD
  give every field. On NEF, the MakerNote (in the layout the preview
  reader already reads) gives the visible image size from CropArea, and,
  only when the standard tags give no value for it, the serial number,
  the ISO, the lens range and the offset from UTC. An Exif value always
  wins. The older Nikon MakerNote layouts (the D1 family and the earliest
  Coolpix bodies) are not read; their files get the Exif fields only. The
  Sony MakerNote gives none of these fields: it is read for the focus
  point, as
  [ADR012](adr012-focus-points-as-the-camera-records-them.md) decides.
  On Nikon and Sony files alike, the MakerNote is read too for the camera
  settings below that no standard tag records. Nothing is deciphered.
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
- **A text of either type where Exif allows both.** The twelve texts
  raw-files reads that Exif 3.1 gives as "ASCII or UTF-8" are read from
  a value of either type: Make, Model, Software, LensMake, LensModel, ImageDescription,
  Artist, Copyright, CameraOwnerName, ImageTitle, Photographer and
  ImageEditor. Every other text, such as the serial numbers and the
  capture time, which Exif gives as ASCII alone, and every MakerNote
  text, is read only from type ASCII.
- **A description and a comment are free text.** They have no 255-byte
  limit, only the cap on the size of any value read (`Limits`), and
  keep their line breaks; every other text keeps the limit.
- **A camera's placeholder is no description.** A text that a measured
  file shows a camera writes in ImageDescription by itself is `None`,
  as a lens named only with dashes is. The list holds `SONY DSC`; a
  text is matched as the whole cleaned text, whatever the Make, and the
  list grows only from measured files.
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
  lacks gives none; a chosen color temperature, a number followed by
  `K`, is the one text read by its form, since no table could list
  every number; Sony's is 0x0115, never 0xB054. The modes form one
  short list in neutral words (auto, daylight, cloudy, shade, tungsten,
  fluorescent, flash, underwater, custom, color temperature), into
  which each maker's finer kinds fold.
- **The color temperature** is the one the MakerNote records, from 1
  to 99,999 K: zero means not recorded, and a larger number is no
  temperature a camera is set to. On Nikon (0x004F) it is the color
  temperature of the white balance the camera applied, under whichever
  setting the body records it. The Nikon bodies that write no 0x004F
  record a temperature only when the photographer chose one, as the
  number of the white balance text (`3230K`), so that number is the
  color temperature when 0x004F gives none; on the Z 9, which writes
  both, they agree. On Sony (0xB021) it is the one the photographer
  chose under the color-temperature setting, and the custom white
  balance's own under custom; the mode beside it says which.
- **The fine-tuning** is the shift along two axes, amber to blue and
  green to magenta, in the camera's own steps, a recorded zero
  included: Nikon's 0x003F, else 0x000B when it holds both axes, and
  Sony's 0x2014. Each value is signed as its axis is named, positive
  towards blue and towards magenta, so the reader turns Nikon's
  green-magenta value and Sony's amber-blue value around. The single
  value the oldest Nikon bodies write in 0x000B gives none, since no
  file shows its axis or its sign.
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
- **The color space** is Exif's whenever Exif records ColorSpace: 1 is
  sRGB, 65535 with InteropIndex `R03` is Adobe RGB, and 65535 otherwise
  is none, so that a program's "uncalibrated" is never overruled by a
  MakerNote it left behind. Only when Exif records none, as on every
  Nikon raw file, does Nikon's 0x001E give sRGB or Adobe RGB.
- **The focal length in 35 mm terms** is Exif's; 0 is none.
- **The software** is IFD0's Software as written, cleaned like every
  text: the camera's firmware, or the program that last wrote the file.
  Exif's CameraFirmware is not read, since no checked file writes it.
- **Authorship is a value of its own**, of seven texts: the artist
  (IFD0's Artist), the copyright notice (IFD0's Copyright), the
  photographer (the Exif IFD's Photographer), the editor (ImageEditor),
  the title (ImageTitle), the description (IFD0's ImageDescription)
  and the comment (the Exif IFD's UserComment). IFD0's texts are read
  from IFD0 alone, not from the copies Sony writes in IFD1 and IFD2,
  and nothing is read from a MakerNote.
- **A comment is read under the ASCII code alone.** The text after
  UserComment's 8-byte code `ASCII` is read and cleaned as every text.
  Under the JIS, Unicode or undefined code, or a code Exif does not
  list, and when the value is no longer than its code, the comment is
  `None`.
- **The camera's owner is the camera's.** CameraOwnerName, which names
  whoever owns the body, whoever takes the picture, and which Exif says
  shall in principle not be rewritten (4.6.6.9.2), is read with the
  camera, beside its serial number.
- **A location needs a sound position.** A file has a location when
  its GPS IFD records GPSLatitudeRef `N` or `S`, GPSLongitudeRef `E` or
  `W`, and GPSLatitude and GPSLongitude as three RATIONAL values each,
  with no zero denominator, minutes and seconds below 60, and no more
  than 90° of latitude or 180° of longitude. Without one, none of the
  location's parts is given, whatever else the GPS IFD records.
- **Coordinates as recorded.** A coordinate keeps the three fractions
  the file records, its degrees, minutes and seconds, and its
  hemisphere; decimal degrees, negative to the south and west, are
  computed when asked for.
- **The location's other parts** are each read from both their tags,
  and a part whose tags are missing or damaged is `None` alone. The
  altitude is GPSAltitude in meters as recorded and whether it lies
  above (GPSAltitudeRef 0 or 2) or below (1 or 3) its reference,
  without naming the surface it is measured from. The time is
  GPSDateStamp and GPSTimeStamp together, one time in UTC; it needs
  whole hours and minutes, seconds whose denominator is a power of ten,
  so that the fraction of a second the file records is kept, and a date
  and time that exist, as the capture time does. The image direction
  is GPSImgDirection, below 360°, counted from the north that
  GPSImgDirectionRef names, `T` true or `M` magnetic, with no default
  when the reference is missing. The map datum is GPSMapDatum, cleaned
  as every text. GPSVersionID, GPSStatus and the other GPS tags are not
  read.
- **As recorded, nothing redacted.** The artist, the copyright notice,
  the owner, the description, the comment and the location name people
  and places. raw-files gives them as the file records them, with no
  option to hide them, as the view of every entry does
  ([ADR023](adr023-every-entry-a-file-records.md)). Tests on public
  sample files check them by presence and shape only, and tests on my
  own files print counts only.

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
- **Reading no Sony MakerNote at all**, as this record first had it. The
  focus point ([ADR012](adr012-focus-points-as-the-camera-records-them.md))
  and the camera settings that only the MakerNote records need it.
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
  value has 65 bytes, so text past 255 bytes is taken for damage,
  except in a description and a comment, which are free text.
- **Deciphering Sony's enciphered block** for older serial numbers. Its
  tables have no settled provenance.
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
  grouped, and a split would put the answers to one question, who made
  the picture and what they wrote about it, in two places.
- **The names `Gps` and `Position`.** Exif says its GPS tags also hold
  positions from other satellite systems and from mobile networks
  (4.6.7), and the value holds more than a position: an altitude, a
  time, a direction and a datum.
- **Decimal degrees only, or no decimal degrees.** Degrees alone lose
  what the file records, and a floating-point value would keep the
  metadata from being compared and hashed; fractions alone make every
  program that shows a map compute the same degrees.
- **A typed value for every GPS tag, or also for those cameras
  write**: the satellites, the status, the measure mode. Most of the
  tags are written by no measured camera file, the rest describe the
  receiver rather than the picture, and the view of every entry gives
  them all.
- **Reading the Unicode code as UTF-8, and text under the undefined
  code.** Exif defines the Unicode code as UTF-8 (Table 4), but no
  measured file shows what writers put there, and it leaves the
  undefined code to the reader's locale (Table 5), so either reading
  would be a guess. The view of every entry gives the bytes.
- **A placeholder given as recorded.** A viewer would show `SONY DSC`
  as the description of every picture those bodies took.
- **The 255-byte limit for every text, or a fixed 64 KiB for the free
  texts.** A program's description is ordinary text at any length, not
  damage, and a second number beside the cap on any value would make a
  value readable one way and not the other.
- **The UTF-8 type for the new texts only, or for none.** Make and
  Model would be read by a different rule from Artist in the same IFD,
  and a file that follows Exif 3.0 with no UTF-8 at all would lose its
  texts as damaged.
- **A redaction option.** raw-files gives what the file records; a
  program that hides a name or a place decides what to hide, as with
  the view of every entry.
- **The surface of the altitude named, or signed meters as a
  floating-point value.** Exif 3.0 changed the meaning of 0 and 1, and
  every measured file records 0 under an older GPS version, so a
  surface would be guessed from a version number; a floating-point
  value loses what the file records.
- **The photographer, the editor, the title and the owner left until a
  file records one.** The standard defines each as plainly as the
  artist, so a file could add nothing their reading needs; their
  readers are checked on built files, since no measured file records
  one.
- **The owner with the authorship.** The owner is a fact about the
  body, set once in the camera and the same on every picture it takes,
  whoever takes it, as the serial number is; the photographer, the
  editor and the title are facts about the picture.

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
  recorded a home zone instead, pictures taken while traveling would get
  a wrong offset; no sample can show it, since those bodies write no Exif
  offset to compare with.
- On a body without a recorded crop, the image size includes the border
  pixels; a caller cannot tell such a size from a recorded one.
- The image size counts stored pixels, which on a Nikon small raw can
  differ from the size the camera's menu names: it is 6192×4128 on the
  D850's small raw (file 1835), whose menu size is 4128×2752.
- The image size is the default crop, not the aspect ratio chosen in the
  camera, which Sony records only in tags of its own.
- Some maker-specific values enter the API, in the camera settings: a
  crop mode, as the image area; the shutter; the raw size. Others (Sony's
  model ID, Nikon's shutter count) are not in the API; they can be added
  later without breaking it.
- DNG files, which use the same standard tags and the same default crop,
  will use the same reader; a new format fills the same `Metadata`.
- `DateTime` converts to other crates' date types only through its
  getters until a caller needs a conversion feature.
- A viewer gets most of the camera's settings from the NEF, ARW and JPEG
  files of the bodies checked. What only an enciphered block records
  stays `None` on every Sony file: the shutter used and APS-C mode;
  Sony's raw size does too. Nikon DSLRs before the D850 have no shutter,
  and the NEF files of the D1 family, the D70, the D100 and the Coolpix
  E5700 and P7800 no image area and no color space. Nikon bodies before
  the D5 and D850 have a color temperature only when the photographer
  chose one, which their white balance text records as its number. The
  D1 family, the D70, the D100 and the D200 have no fine-tuning, and
  neither have the Sony bodies that write no 0x2014: the A100, A200,
  A700, A900, A55, NEX-5, F828 and R1. A Sony file has a color
  temperature only under a chosen color temperature or a custom white
  balance, the only settings under which the files checked record one.
- Nikon's table of white balance texts grows only from measured files:
  a text not in it yet gives way to Exif's white balance, which says
  only auto or the kind of light, and never to a guess from its
  letters.
- Codes that a later body or a later Exif adds give `None` until they
  are named, which adds to the types without breaking them.
- The software names the camera's firmware or the program that last
  wrote the file, and a caller cannot tell which without reading the
  text.
- A comment under the JIS, Unicode or undefined code is `None`; no
  measured file has text under one. The view of every entry gives its
  bytes.
- The GPS tags not read stay in the view of every entry: the
  satellites, on every Nikon file with a position; the status and the
  measure mode, on Sony files; the speed, the track and the precision,
  on 2 public ARWs; the differential correction; and the destination,
  processing-method, area and positioning-error tags, on no measured
  camera file.
- Which surface an altitude is measured from is not given.
- A GPS IFD without a sound position gives no time, altitude,
  direction or datum, even where it records them, as a Sony file
  without a fix records its datum.
- A text that only IFD1, IFD2 or a MakerNote holds gives none; no
  measured file holds one there and not in IFD0.
- The photographer, the editor, the title and the owner are read by
  the standard's text alone, and checked on built files only, until a
  file records one.
- A placeholder that another body writes is given as a description
  until a measured file shows it and the list gains it.
- A program that shows these values shows people's names and the
  places they were; raw-files leaves what to show to it.
