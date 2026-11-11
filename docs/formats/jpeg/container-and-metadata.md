# JPEG container and metadata

A JPEG file is one compressed picture wrapped in a series of segments that
say how it is coded and what it shows: when it was taken, by which camera,
and which smaller copies of it the file also holds. This page describes
those segments as far as raw-files reads them: the marker segments and the
frame header of the JPEG standard, the JFIF segment, the Exif segment with
the TIFF structure inside it and its thumbnail, and the Multi-Picture
Format segment that lists the larger previews. It ends with the way
raw-files reads a JPEG file, the traps on the way, and what the
directories of camera JPEGs hold. Back to
[the JPEG overview](README.md).

## How to read this page

Claims carry a confidence tag:

- **[P]**: stated in a specification, cited by its section, table or
  figure, and page (the printed number).
- **[S]**: secondary: another program's documentation or output, a
  write-up, a search.
- **[I]**: inferred, or a recommendation. Treat it as a hypothesis.
- **[F]**: measured on a named sample file: one of the eight
  camera-original JPEGs from Wikimedia Commons, cited by body ("the
  ILCE-7M3 file"), which [test-data.md](test-data.md#2-the-sample-files)
  lists with its file name, license and SHA-256; or on the early
  Coolpix files checked by hand, which
  [test-data.md](test-data.md#6-early-coolpix-files-checked-by-hand)
  lists the same way.

Numbers in brackets point to the [Sources](#sources) at the end. The
specifications are the editions listed there: T.81 (1992), T.871 (2011),
Exif 3.1 (CIPA DC-008-2026) and the Multi-Picture Format of CIPA
DC-007-2026. Where a file departs from a specification, this page follows
the file and says so.

## Contents

- [1. Marker segments](#1-marker-segments)
- [2. The frame header](#2-the-frame-header)
- [3. JFIF: the APP0 segment](#3-jfif-the-app0-segment)
- [4. Exif: the APP1 segment](#4-exif-the-app1-segment)
- [5. Other application segments](#5-other-application-segments)
- [6. The thumbnail in IFD1](#6-the-thumbnail-in-ifd1)
- [7. The Multi-Picture Format](#7-the-multi-picture-format)
- [8. After the last image](#8-after-the-last-image)
- [9. Sizes and orientation](#9-sizes-and-orientation)
- [10. MakerNotes in a JPEG](#10-makernotes-in-a-jpeg)
- [11. The camera files, segment by segment](#11-the-camera-files-segment-by-segment)
- [12. Recommendation for raw-files](#12-recommendation-for-raw-files)
- [13. Pitfalls](#13-pitfalls)
- [14. The directories a file holds](#14-the-directories-a-file-holds)
- [Sources](#sources)

## 1. Marker segments

A JPEG stream is a sequence of markers. A marker is the byte `FF` followed
by a code that says what comes next. [P][1] (B.1.1.2, p. 31)

- **Fill bytes.** Any number of `FF` bytes may come before a marker; a
  reader skips them before deciding which marker it has. [P][1] (B.1.1.2,
  p. 31; E.1.2, p. 78; note to Table E.1, p. 82)
- **Markers that stand alone.** SOI (`FF D8`, start of image), EOI
  (`FF D9`, end of image), the eight restart markers RST0 to RST7 (`FF D0`
  to `FF D7`) and TEM (`FF 01`) have nothing after them. [P][1] (B.1.1.3
  and Table B.1, pp. 31 to 32)
- **Every other marker starts a segment**: after the marker comes a
  two-byte big-endian length, which counts itself and the segment's
  payload but not the two bytes of the marker. A length is therefore at
  least 2, and a segment holds at most 65,533 bytes after its length.
  [P][1] (B.1.1.4, p. 33; Table B.9, p. 44)
- **The order.** A file holds SOI, one frame and EOI. Before the frame
  header, and before each scan's header, may come any number of table and
  miscellaneous segments, in any order: quantization tables (DQT,
  `FF DB`), Huffman tables (DHT, `FF C4`), arithmetic-coding conditions
  (DAC, `FF CC`), the restart interval (DRI, `FF DD`), comments (COM,
  `FF FE`) and application segments (APP0 to APP15, `FF E0` to `FF EF`).
  [P][1] (B.2.1 and Figure B.2, pp. 34 to 35; B.2.4 and Figure B.5,
  p. 39)
- **The scan.** A scan starts with its header (SOS, `FF DA`) and goes on
  with the entropy-coded data. Inside it, every `FF` byte the coder
  produced is followed by a stuffed `00`, so that it is not read as a
  marker, and restart markers may separate its parts. [P][1] (B.2.3,
  p. 37; B.1.1.5 note 2, p. 33; F.1.2.3, p. 91) A reader looking for
  segments stops at the first SOS: what follows is compressed data, not
  segments. [I]
- **Application segments** are left to applications: the standard gives
  their bytes no meaning, and a decoder reads them only so far as they do
  not interfere with decoding. [P][1] (B.2.4.6, p. 44; E.2.1, p. 83) The
  standard itself does not say a reader may skip them; Exif says a reader
  should skip the ones it does not know. [P][3] (4.5.4, p. 20; 4.7, p. 99)
- The other codes in Table B.1 are frame markers (section 2), DNL
  (`FF DC`), DHP (`FF DE`) and EXP (`FF DF`) for the hierarchical
  process, JPG (`FF C8`) and JPG0 to JPG13 (`FF F0` to `FF FD`) reserved
  for extensions, and RES (`FF 02` to `FF BF`) reserved. [P][1]
  (Table B.1, p. 32)

On the eight camera files the walk from SOI to SOS takes five to nine
segments, and the frame header lies 31,880 to 105,002 bytes into the file
(section 11). [F]

## 2. The frame header

The frame header gives the picture's size and how it is coded. After its
marker (SOFn) and its length come the sample precision (one byte), the
number of lines Y and the number of samples per line X (two bytes each,
big-endian), the number of components, and three bytes per component.
The length is therefore 8 plus three times the number of components.
[P][1] (B.2.2, Figure B.3, p. 35; Table B.2, p. 36)

- **The coding process is in the marker.** `FF C0` is baseline DCT;
  `C1`, `C2` and `C3` are extended, progressive and lossless sequential
  with Huffman coding; `C5` to `C7` their differential (hierarchical)
  forms; `C9` to `CB` and `CD` to `CF` the same processes with arithmetic
  coding. Table B.1 labels `C3` and `CB` "lossless (sequential)" and `C7`
  and `CF` "differential lossless (sequential)". [P][1] (Table B.1, p. 32)
- **A lossless frame holds sample values exactly**, which is how some raw
  files store their sensor data: Sony stores the raw data of its lossless
  ARW files as lossless JPEG tiles, each starting `FF D8 FF C3` (see the
  [ARW raw-data notes](../arw/raw-data.md#lossless-compressed-compression-7)).
  A lossless stream is not a picture to show. [I]
- **A height of 0** means that the number of lines is given later, by a
  DNL segment at the end of the first scan; a reader that stops at SOS
  cannot learn it. [P][1] (p. 36; B.2.5, p. 45)
- Every frame on the eight camera files, the main image's, the
  thumbnails' and the MPF images', is baseline (`FF C0`). [F]

## 3. JFIF: the APP0 segment

JFIF is a profile of the JPEG standard for exchanging files. A JFIF file
has an APP0 segment immediately after SOI, whose payload starts with the
five bytes `JFIF` and a zero, then a version (`01 02`), the units and the
horizontal and vertical pixel density, and the size of an optional
uncompressed thumbnail followed by its pixels. [P][2] (6.1, p. 2; 6.3,
p. 3; 10.1, p. 6) Extension segments (JFXX, also APP0) may follow it,
and other application segments come after those. [P][2] (6.4, 6.5, p. 3;
10.2, p. 7)

- **JFIF and Exif disagree about the first segment.** Each puts its own
  segment immediately after SOI, and neither mentions the other: T.871 says
  nothing of Exif, APP1 or TIFF, and Exif 3.1 says nothing of JFIF or APP0,
  anywhere in their text. [P][2][3] A file that follows both has its Exif
  segment second. [I]
- **None of the eight camera files has a JFIF segment**: each starts with
  its Exif segment. [F] Editors that re-encode a file often write one;
  in the search that chose the eight files, files whose first segment was
  JFIF were the re-encoded ones, and they were left out (see
  [test-data.md](test-data.md#3-how-the-files-were-chosen)). [S]
- raw-files does not read the JFIF segment: the frame header gives the
  size, and its thumbnail is uncompressed. [I]

## 4. Exif: the APP1 segment

Exif records the camera's metadata in an APP1 segment. Its payload starts
with the four bytes `Exif` and two zero bytes (Exif names the first
"NULL" and the second "padding"), and a TIFF header follows at once: a
byte order mark (`II` or `MM`), the number 42 in that order, and the
offset of the first IFD. [P][3] (4.7.2.3 and Figure 36, p. 105;
4.7.2.5, p. 106; Table 1, p. 20)

- **Offsets count from the TIFF header**, from the first byte of its
  byte order mark, not from the start of the file or of the segment.
  [P][3] (4.6.2, p. 28; 4.7.2.5, p. 106; 4.6.3.1 and 4.6.3.2, pp. 28 to
  29) The structure is the one a NEF or an ARW file is: IFD0 (Exif's 0th
  IFD), its next IFD, IFD1 (Exif's 1st IFD), and the Exif, GPS and
  Interoperability IFDs that IFD0 and the Exif IFD point to. [P][3]
  (4.5.2, p. 19; 4.6.3.1 and 4.6.3.2, pp. 28 to 29)
- **The segment comes immediately after SOI**, and APP2 segments may
  follow it. [P][3] (4.5.4, p. 20; Table 2, p. 22; 4.7.2.1, p. 105)
- **It holds at most 64 KB**, and the thumbnail must fit in it together
  with everything else. [P][3] (p. 21; 4.7.2, p. 105; 4.6.5.2.5, p. 40)
  The bound in bytes comes from the JPEG standard: a length of at most
  65,535, so at most 65,533 bytes after the length, 6 of them the
  identifier and at most 65,527 the TIFF structure. [P][1] (Table B.9,
  p. 44) The Exif standard does not repeat the number.
- **On the eight camera files** the Exif segment is the first segment,
  so its TIFF header lies at byte 12 of the file (two bytes of SOI, two
  of marker, two of length and six of identifier). Its length runs from
  27,646 (G7 X) to 65,534 (D5300, one byte short of the maximum). The
  A100's TIFF structure is big-endian (`MM`), the other seven
  little-endian. [F]
- **Everything the standard tags need lies inside the segment** on all
  eight: every IFD, every value raw-files reads, the thumbnail and the
  MakerNote. [F] One MakerNote entry points outside it (section 10).
- **One Exif segment per file**: no file has a second. [F] Exif describes
  one APP1 segment [P][3] (4.5.4, p. 20); what a reader does with a second
  is not said. [I]

**The camera's settings.** How the camera was set for the shot is
recorded in tags of the Exif IFD, and the program that wrote the file
in IFD0's Software [P][3]:

| Setting | Tag | Values | Exif 3.1 |
|---|---|---|---|
| Exposure program | 0x8822 ExposureProgram, SHORT | 0 not defined, 1 manual, 2 normal program, 3 aperture priority, 4 shutter priority, 5 creative, 6 action, 7 portrait, 8 landscape; others reserved | 4.6.6.7.3, p. 55 |
| Metering mode | 0x9207 MeteringMode, SHORT | 0 unknown, 1 average, 2 center-weighted average, 3 spot, 4 multi-spot, 5 pattern, 6 partial, 255 other | 4.6.6.7.19, p. 60 |
| Kind of light | 0x9208 LightSource, SHORT | 0 unknown, 1 daylight, 2 fluorescent, 3 tungsten, 4 flash, 9 fine weather, 10 cloudy weather, 11 shade, 12 to 16 five kinds of fluorescent lamp, 17 to 23 standard lights A, B, C, D55, D65, D75, D50, 24 ISO studio tungsten, 25 to 29 five kinds of light source and 30 to 34 five kinds of LED by color temperature, 255 other | 4.6.6.7.20, pp. 60 to 61 |
| Flash | 0x9209 Flash, SHORT, bits | bit 0 fired; bits 1 and 2 whether return light was detected; bits 3 and 4 the mode: unknown, compulsory firing, compulsory suppression, auto; bit 5 no flash function; bit 6 red-eye reduction | 4.6.6.7.21 and Figure 19, pp. 61 to 62 |
| White balance | 0xA403 WhiteBalance, SHORT | 0 auto, 1 manual | 4.6.6.7.37, p. 69 |
| Focal length in 35 mm terms | 0xA405 FocalLengthIn35mmFilm, SHORT, in millimeters | 0 unknown | 4.6.6.7.39, p. 70 |
| Color space | 0xA001 ColorSpace, SHORT | 1 sRGB, 0xFFFF uncalibrated: any color space but sRGB | 4.6.6.2.1, p. 47 |
| Software | IFD0 0x0131 Software, ASCII or UTF-8 | the name and version of the software or firmware that generated the image | 4.6.5.4.4, p. 42 |

- **ExifTool's documentation lists an exposure program 9**, "bulb",
  which Exif does not define [S][6]. No file checked for the camera
  settings holds it. [F]
- **A color space other than sRGB is named elsewhere.** ColorSpace says
  only sRGB or "uncalibrated". The Interoperability IFD, which the Exif
  IFD points to with tag 0xA005 (4.6.3.3.1, p. 29), holds
  InteroperabilityIndex (0x0001, ASCII), which names the rule the file
  follows: `R98` a DCF basic file, `THM` a DCF thumbnail, `R03` a DCF
  option file (4.6.8.1.1, p. 92) [P][3]. A DCF option file is in Adobe
  RGB, in ExifTool's documentation of the tag [S][6], and the files
  agree: all 615 of my own D3200 JPEGs with ColorSpace 65535 and `R03`
  record Adobe RGB in Nikon's 0x001E
  ([NEF notes](../nef/makernote-and-metadata.md#87-camera-settings))
  **[F]**, and so does the A900 raw file's settings block, as ExifTool
  decodes it
  ([ARW notes](../arw/makernote-and-metadata.md#97-camera-settings))
  **[S]**.
- **Exif 3.0 added the camera's firmware** in a tag of its own,
  CameraFirmware (0xA439), which "in principle, shall not be
  overwritten", beside tags for the RAW developer, the image editor and
  the metadata editor; Software should then repeat one of them
  (4.6.6.9.11 to 4.6.6.9.14, pp. 81 to 82) [P][3]. None of the eight
  camera files writes these tags, nor any raw sample or the stripped
  Z 9 JPEGs of the fixtures; every one of them that declares an Exif
  version declares one from 2.21 to 2.32, except the D70 file, which
  writes `1220` **[F]**.
- **On the eight camera files** [F]: MeteringMode, Flash, WhiteBalance
  and ColorSpace are on all eight, ColorSpace 1 (sRGB) on every one.
  The G7 X writes no ExposureProgram, LightSource,
  FocalLengthIn35mmFilm or Software; the other seven write all four.
  ExposureProgram is 0, "not defined", on the D5300 file. The flash
  fired on none (Flash 16 or 0). White balance is manual on the
  DSC-RX100M4 file only, with LightSource 11 (shade). InteropIndex is
  `R98` on seven files. On the D70 file the Interoperability pointer
  leads to IFD1 (section 6), which holds no InteropIndex.
- **On my own JPEGs that a program rewrote** (425, from three Nikon DSLRs),
  Software names that program, not the camera: Windows Photo Viewer, GIMP,
  Luminance HDR, Shotwell, ViewNX 2 and darktable. 32 of them have
  ColorSpace 65535 with no InteropIndex, and 10 of those still hold the
  camera's sRGB in the MakerNote, which no longer describes the picture
  ([NEF notes](../nef/makernote-and-metadata.md#87-camera-settings)).
  [F]

**Who made the picture.** IFD0 records the artist in 0x013B Artist,
the copyright notice in 0x8298 Copyright and a description in 0x010E
ImageDescription; the Exif IFD a comment in 0x9286 UserComment, the
camera's owner in 0xA430 CameraOwnerName and, since Exif 3.0, the
title, the photographer and the editor in 0xA436 ImageTitle, 0xA437
Photographer and 0xA438 ImageEditor. Each is of type ASCII or UTF-8,
its count including the NUL that ends it, except UserComment: it is
UNDEFINED, needs no NUL, and starts with 8 bytes naming its
characters, `ASCII` and three NULs, `JIS` (ISO-2022-JP), `UNICODE`
(UTF-8), or eight NULs for undefined text, whose characters Exif
leaves to the reader's locale. A tag kept free for a comment should
hold the code `ASCII` and spaces. [P][3] (4.6.4 and Tables 4 and 5,
pp. 29 to 31; 4.6.5.4.1, 4.6.5.4.6 and 4.6.5.4.7, pp. 41 to 43;
4.6.6.4.2, p. 49; 4.6.6.9.2 and 4.6.6.9.8 to 4.6.6.9.10, pp. 79 to 81)
The [NEF](../nef/makernote-and-metadata.md#88-authorship) and
[ARW](../arw/makernote-and-metadata.md#98-authorship) notes say what
Nikon's and Sony's raw files hold in these tags.

- **On the eight camera files** [F]: the D5300 and D70 files have no
  Artist and no Copyright, and a UserComment of 44 bytes, the code
  `ASCII` and spaces. Four of the five Sony files have an
  ImageDescription of 32 spaces, with a copy in IFD1, and the A100
  file one of `SONY DSC` padded with spaces to 30 bytes; all five have
  a UserComment of 64 bytes, the undefined code and NULs, and the
  RX100M4 file has an Artist and a Copyright, with copies in IFD1.
  The G7 X file has an ImageDescription of 32 spaces, an Artist, a
  Copyright and a CameraOwnerName of one NUL each, and a UserComment
  of 264 bytes, the undefined code and NULs.
- **On more files** **[F]**, where **[F]** marks a count over the
  files named here rather than a fact of one named file: counted on 9
  November 2026 with a counting program of my own over raw-files' own
  view of every entry ([section 14](#14-the-directories-a-file-holds)),
  written from Exif 3.1, with no other reader run, on the 9 JPEGs of
  raw.pixls.us and,
  as counts only, 24,607 of my own: 274 from my Z 9, 16,792 from
  folders of six Nikon DSLRs' pictures, 1,974 from my ILCE-7M2, 5,345
  from Canon and Fujifilm cameras, and 222 a program exported or
  rewrote.
  - Every text is plain ASCII: of type ASCII, NUL-terminated, with no
    byte above 0x7F and nothing after the NUL. No JPEG uses the UTF-8
    type.
  - Nikon's JPEGs carry the fixed fields of its raw files: Artist in 37
    bytes and Copyright in 55, a NUL first or all spaces when unset,
    on 8,850 of my Z 9's and DSLRs' JPEGs (all but one in those
    lengths), with text on 4,744; and a UserComment of 44 bytes,
    `ASCII` and 36 bytes of NULs, spaces or text, text on 3,022 of the
    DSLRs' JPEGs. Nikon's camera JPEGs have no ImageDescription. Of the
    Nikon JPEGs a program exported or rewrote, 151 hold comment text.
  - Sony's JPEGs, like its raw files, have an ImageDescription of 32
    spaces in IFD0 and IFD1, and a UserComment of 64 bytes, the
    undefined code and NULs: on all 1,974 of my ILCE-7M2's.
  - A Canon DSLR writes an Artist, a Copyright and a CameraOwnerName
    of one NUL (137 files), and Canon cameras a UserComment of 264
    bytes, the undefined code and NULs (805 files). Fujifilm compacts
    write a Copyright of four spaces (4,534 files). Of the 9
    raw.pixls.us files, the phone's below has a Copyright and an
    ImageDescription of one NUL, and none has a UserComment.
  - A program's texts can be long: 42 JPEGs of mine that a program
    rewrote, with no Make, hold an ImageDescription of 153 to 177
    bytes, with control characters, and a UserComment (code `ASCII`)
    of 160 to 184; 8 more that a program rewrote have a description of
    text.
  - No JPEG has text in CameraOwnerName, Photographer, ImageEditor or
    ImageTitle, or a UserComment under the `JIS` or `UNICODE` code, or
    text under the undefined one.
- raw-files reads these texts as in a raw file, as
  [ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
  decides: IFD0's from IFD0 alone, not from IFD1; the comment only
  under the `ASCII` code; `SONY DSC` as no description, as the tests
  check on the A100 file. A comment segment (COM, section 1) and XMP
  (section 5) are not read for them.

**Where it was taken.** The GPS IFD, which IFD0's 0x8825 points to,
records the position, the altitude, the GPS time in UTC, the image
direction and the map datum, beside tags about the receiver; the
[NEF notes](../nef/makernote-and-metadata.md#89-location) describe
them, with Exif 3.1's sections. [P][3] (Table 14, pp. 82 to 83;
4.6.7, pp. 82 to 91)

- **On the eight camera files** [F]: the D5300 and G7 X files have a
  GPS IFD of a single entry, its version (2.3.0.0); the ILCE-7M3 file
  one of four, the version, the status `V`, measurement interrupted,
  the datum `WGS-84` and the differential correction 0. None records a
  position.
- **On more files** **[F]**, the same sets as above: a phone's file
  among the raw.pixls.us files (a Samsung SM-G973U; the catalog lists
  it as a DNG, but its bytes are a JPEG's), of GPS version 2.2.0.0,
  records a position in whole seconds (`d/1, m/1, s/1`) and
  GPSAltitudeRef without GPSAltitude, and no time or datum. My Z 9's
  JPEGs record a position on 262 of 274, as its NEFs do; my DSLRs'
  JPEGs (16,060), the Canon DSLR's (137) and 35 rewritten Nikon files
  have a GPS IFD of only its version; my ILCE-7M2's JPEGs have no GPS
  IFD. Every GPS entry is of the type and count Exif gives.
- raw-files gives a location, `Metadata::location`, only where both
  references and both coordinates are sound, as
  [ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
  decides; a GPS IFD of only a version, or without a fix, gives none,
  and a reference without its altitude gives no altitude. The tests
  check that none of the eight camera files gives a location.

Every text and GPS value counted lies inside the blocks raw-files'
`open` already reads. [F]

## 5. Other application segments

- **XMP** is another APP1 segment, whose payload starts with
  `http://ns.adobe.com/xap/1.0/` and a zero byte. [F] (Its specification,
  Adobe's XMP Part 3, was not read for these notes. [I]) Three of the
  camera files write one, holding only a rating of 0: the ILCE-7M3
  (57,346 bytes, nearly all padding), the D5300 (2,046 bytes) and the
  G7 X (3,996 bytes). [F] raw-files does not read XMP (see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
- **APP2** segments of two kinds may follow the Exif segment: FlashPix
  extension data (`FPXR`) and the Multi-Picture Format (`MPF`), section 7.
  [P][3] (4.7.3, pp. 106 to 107; 4.7.4, p. 110) Exif also places JUMBF
  data in APP11 (4.7.5), and lets makers add segments of their own (4.5.5
  and 4.5.6, pp. 22 to 25). [P][3] Color profiles (ICC) are also carried
  in APP2 segments; that specification was not read. [I]
- Exif asks that all quantization tables be in one DQT segment and all
  Huffman tables in one DHT segment; the D70 file writes two DQT and four
  DHT segments. [P][3] (4.7.1.2 and 4.7.1.3, pp. 100 to 101) [F] Nothing
  raw-files does depends on it.

## 6. The thumbnail in IFD1

IFD1, the IFD that IFD0's next-IFD pointer leads to, holds the thumbnail.
[P][3] (4.5.2, p. 19; 4.5.8, pp. 25 to 26; 4.7.2.4, p. 106)

- **A JPEG thumbnail** has Compression 6. JPEGInterchangeFormat (0x0201)
  gives the offset of its SOI, from the TIFF header, and
  JPEGInterchangeFormatLength (0x0202) its length in bytes; both are
  required for a compressed thumbnail. It is one stream from SOI to EOI,
  with no application segments, comments or restart markers, stored inside
  the Exif segment. [P][3] (4.5.8, pp. 25 to 26; 4.6.5.1.4, p. 33;
  4.6.5.2.4 and 4.6.5.2.5, p. 40; Table 21, pp. 98 to 99; Figure 7, p. 21)
- **No size is set for it**: Exif 3.1 places no limit on its size, and it
  is optional. [P][3] (4.5.8, pp. 25 to 26) The 160×120 pixels often quoted
  is not in that edition.
- **On the camera files** every thumbnail is a JPEG of 160×120 pixels but
  the D70's, of 128×85, from 4,129 to 10,107 bytes, inside the segment
  and ending with EOI. [F]
- **The D70 file shares one IFD between two roles**: IFD0's next-IFD
  pointer and the Exif IFD's Interoperability pointer (0xA005) both lead
  to offset 29,110, an IFD whose seven entries are the thumbnail's.
  [F] (A reader that runs ExifTool on it is told that the IFD1 pointer
  refers to the Interoperability IFD. [S]) A reader that visits each IFD
  once and follows the Interoperability pointer first could take IFD1 for
  a loop and lose the thumbnail. [I] raw-files follows the
  Interoperability pointer only after IFD0's whole chain, so it reads the
  shared IFD as IFD1 and keeps the thumbnail; it still takes that IFD as
  the Interoperability IFD, which holds no InteropIndex. [F]

## 7. The Multi-Picture Format

The Multi-Picture Format lets one file hold several images: the main
image, then larger thumbnails for screens and other images, each a whole
JPEG stream from SOI to EOI. An APP2 segment in the first image lists
them. [P][4] (5.1 and Figure 1, pp. 7 to 8)

### 7.1 The segment and its index

- **The segment.** Its payload starts with `MPF` and a zero byte, then an
  MP header in the shape of a TIFF header: a byte order mark, 42, and the
  offset of the first IFD, normally 8. There is one such segment per
  image, and it holds at most 64 KB. [P][4] (5.2.1 and Figure 4, pp. 10
  to 11; 5.2.2.1 and Figure 5, 5.2.2.2, p. 11; Figure 2, p. 9)
- **Every offset counts from the MP header**, from the first byte of its
  byte order mark. [P][4] (5.2, p. 10) On the camera files the MP header
  lies 8 bytes into the segment (marker, length, identifier). [F]
- **The MP Index IFD** holds MPFVersion (0xB000, four bytes, `0100`),
  NumberOfImages (0xB001, a LONG) and MPEntry (0xB002, 16 bytes per
  image), which are required, and ImageUIDList (0xB003) and TotalFrames
  (0xB004), which are not. Its tags are in ascending order, and it is in
  the first image only. [P][4] (Table 3, p. 12; 5.2.3, pp. 12 to 18;
  Table 6, p. 27; 6.2.1.2)
- **An MP Entry** is 16 bytes: the image's attribute (4 bytes), its size
  (4), its offset (4), and the entry numbers of two images that depend on
  it (2 and 2). The attribute holds, from the top bit down, three flags
  (dependent parent, dependent child, representative image), two
  reserved bits, the image data format in three bits (0 is JPEG), and the
  MP type in the low 24 bits. [P][4] (Figure 7, p. 13; Figure 8, p. 14;
  5.2.3.3.1, p. 14)
- **The size** is that of the image data "between the SOI and EOI
  markers"; whether the markers count is not said. [P][4] (5.2.3.3.2,
  p. 16) On the camera files the first image's size ends exactly after
  its EOI: the markers count. [F]
- **The offset** is that of the image's SOI, from the MP header, and is
  0 for the first image. [P][4] (5.2.3.3.3, p. 16)
- **The MP types** include 0x030000, the baseline primary image;
  0x010001 to 0x010005, large thumbnails of classes 1 to 5; 0x020001 to
  0x020003, the panorama, disparity and multi-angle images of a
  multi-frame file; 0x040000, an original kept for preservation;
  0x050000, a gain map; and 0x000000, undefined. [P][4] (Table 4, p. 15;
  Figure 9, p. 16)

### 7.2 Large thumbnails

A large thumbnail of class 1 has at most 640×480 pixels, of class 2
1920×1080, then 3840×2160, 7680×4320 and 15360×8640; one side equals the
class's maximum, the aspect ratio is the main image's, and a file holds at
most one per class. Large thumbnails carry no MPF segment of their own,
and their Exif segment is left out when they have no tag values of their
own. [P][4] (6.1.2.2 and Table 9, pp. 30 to 31; 6.1, p. 29; 6.1.2.1,
p. 30; clause 7, p. 36)

On the camera files [F]:

| File | MPF images after the first | Where they start |
|---|---|---|
| ILCE-7M3, ILCE-7RM2, DSC-RX100M4, ILCE-6000 | one of class 2, 1616×1080 | after zeros, at a multiple of 512 bytes |
| Canon G7 X | one of class 2, 1620×1080 | after 12 bytes that are not zeros, at a multiple of 512 bytes |
| Nikon D5300 | one of class 1, 640×424, and one of class 2, 1620×1080 | after zeros, at a multiple of 512 bytes |
| Nikon D70, Sony DSLR-A100 | no MPF segment | |

- Sony's 1616×1080 does not quite keep the 3:2 aspect ratio of the main
  image, which would give 1620×1080. [F]
- Each of these images starts with a small Exif segment of its own (160
  bytes on the Sony files, 510 on the G7 X, 1,022 on the D5300) and has
  no MPF segment; its frame header lies 648 to 1,160 bytes into it. [F]
- The first entry's attribute is 0xA0 in its top byte (a parent of
  dependent images, and the representative image) on the Sony and D5300
  files, and 0x80 (a parent, but not marked representative) on the G7 X;
  the thumbnails' is 0x40 (a dependent child). [F]
- On the Sony files the MPF segment ends where the next segment starts at
  a multiple of 512 bytes from the start of the file. [F]
- The specification does not say that the images follow the first one's
  end, or in which order: it shows them one after another, and leaves an
  extended MP file's order free. [P][4] (Figure 1, p. 8; 6.2, p. 32) On
  every camera file they follow the first image, in the order of their
  entries. [F]

## 8. After the last image

Exif sets nothing about bytes after EOI, and asks readers to carry on
past data they do not know after the main image's EOI. [P][3] (4.7,
p. 99; 4.7.1.8, p. 104; 4.5.7, p. 25)

- **Sony pads its files with zeros** after the last image's EOI, up to a
  multiple of 64 KiB: 28,938 bytes on the ILCE-7M3 file, 31,620 on the
  ILCE-7RM2 and 28,400 on the ILCE-6000. The DSC-RX100M4 and DSLR-A100
  files end at the EOI, as do the Nikon and Canon files. [F] A file whose
  size is a multiple of 64 KiB is therefore not a sign of truncation. [I]
- The file's own preview, as raw-files reports it, is the whole file, so
  its bytes may end with MPF images and zeros after the main image's EOI;
  a decoder stops at that EOI. [I] (see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md))

## 9. Sizes and orientation

- **PixelXDimension (0xA002) and PixelYDimension (0xA003)** give the
  width and height of the meaningful image, without padding; they are
  required in a compressed file. [P][3] (4.6.6.3.1 and 4.6.6.3.2, p. 48;
  Table 18, p. 95; 4.8.1, pp. 115 to 119) On all eight camera files they
  equal the main frame header's size. [F] raw-files takes the image
  size from the frame header, the picture itself, and does not read them (see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
- **Orientation (0x0112)** is a SHORT, 1 by default, whose values 1 to 8
  say where the stored image's first row and first column belong when it is
  shown. It is recommended in IFD0 and optional in IFD1. [P][3] (4.6.5.1.6,
  p. 33; Table 17, p. 96; Table 21, pp. 98 to 99) Every camera file has
  orientation 1 in IFD0. [F] Whether a camera stores its thumbnail and its
  MPF images turned when it records a portrait picture is not known from
  these files; see [open-questions.md](open-questions.md).
- **Nothing in a JPEG gives a raw size**: it holds no raw data. [I]

## 10. MakerNotes in a JPEG

The MakerNote (0x927C, UNDEFINED, in the Exif IFD) holds whatever the
maker records, in a layout the standard leaves to the maker. [P][3]
(4.6.6.4.1, p. 49; Table 18, p. 95) In a JPEG, the MakerNotes are those
of the same makers' raw files, read with the TIFF header inside the Exif
segment wherever the raw file counts from the start of the file:

- **Nikon** (the D5300 and D70 files): the layout with its own embedded
  TIFF header that NEF files use, whose offsets count from that inner
  header, so it reads the same in a JPEG. [F] The D70's PreviewIFD holds
  a 564×372 JPEG inside the MakerNote's declared length. [F]
- **Nikon, early Coolpix bodies** (files checked by hand, listed in
  [test-data.md](test-data.md#6-early-coolpix-files-checked-by-hand)):
  the two older layouts, little-endian like the Exif segment. The E990,
  E995, E880, and the E885, E2500 and E5000 on their earlier firmware,
  write the headerless layout of the D1 family, an IFD at the
  MakerNote's first byte whose offsets count from the TIFF header inside
  the Exif segment, as in a NEF they count from the start of the file
  (file 5364 of raw.pixls.us). [F] raw-files reads it so, in that
  structure's byte order. Its PreviewIFD, on the E885, E995, E2500 and
  E5000, holds a 320×240 JPEG inside the Exif segment but 12,527 bytes
  past the MakerNote's end; raw-files does not give it, since a
  MakerNote's preview must lie inside the MakerNote. [F] The E700, E800,
  E900 and E950 write the short header `Nikon\0\1\0`, then an IFD whose
  offsets count from the same TIFF header; raw-files does not read that
  layout. [F] See the
  [NEF MakerNote notes](../nef/makernote-and-metadata.md#in-a-jpeg-file).
- **Sony** (all five Sony files): the 12-byte header `SONY DSC ` and three
  zero bytes, then an IFD whose offsets count from the TIFF header inside
  the Exif segment, as an ARW's count from the start of the file. [F] The
  ILCE-6000's MakerNote tag 0x2001 declares 549,518 bytes at an offset
  that lies inside its MPF image, outside the Exif segment. [F] See the
  [ARW MakerNote notes](../arw/makernote-and-metadata.md#1-finding-the-makernote).
- **Canon** (the G7 X file) and other makers: no reader. [I]

What each maker's MakerNote gives in a JPEG, the focus point and the
camera settings among it, is in those pages.

**The camera settings come through the same paths** as in a raw file:
the Exif tags of section 4 from the Exif segment, and the MakerNote's
settings tags from its IFD, in the layouts above. On the D5300, D70 and
five Sony files they are the tags the
[NEF notes](../nef/makernote-and-metadata.md#87-camera-settings) and the
[ARW notes](../arw/makernote-and-metadata.md#97-camera-settings) record,
with the values listed there. [F] The three stripped Z 9 JPEGs of the
[fixtures](../../../tests/fixtures/README.md), shot with my own Z 9 on
firmware `Ver.05.30` and made public as fixtures, keep the camera's
MakerNote whole, all 75 entries, beside the two stripped NEF files of
the same body and firmware and the Z 9 NEF samples of raw.pixls.us
(firmware `Ver.01.00`, 73 entries on the lossless file, 72 on the HE
and HE\* ones). [F]

- Every settings tag of the NEF notes is in the JPEG, with the NEF's
  types: WhiteBalance (`AUTO0`), WhiteBalanceFineTune (0x003F),
  ColorTemperatureAuto, ShutterMode (16), CropHiSpeed (11, 2 and 6 on
  the FX, DX and 16:9 files), ImageSizeRAW, ColorSpace (1) and FlashMode.
  [F]
- The JPEG lacks what only the raw file needs, or what the camera
  writes only beside raw data: the PreviewIFD (0x0011), CropArea
  (0x0045, as [ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)
  records), 0x0046 and 0x00A4, which the NEF fixtures keep, and against
  the raw.pixls.us NEF files also the decoding tags (0x003D, 0x008C,
  and 0x0096 on the lossless file) and 0x0051, 0x0058. [F] The JPEG has eleven tags those older
  NEF files lack (0x0016, 0x0035, 0x0057, 0x0059 to 0x005D, 0x005F,
  0x0062, 0x00A2); whether they come with the format or with the newer
  firmware, no public file can say. [I]
- **ImageSizeRAW (0x003E) is 1 on the two full-size JPEGs and 2 on the
  middle-size one** [F]: in a JPEG it follows the JPEG's own size. My
  49 Z 9 shots of 2026-10-22 taken as a NEF and a JPEG together, at
  the JPEG's large, medium and small sizes in each crop mode, show it:
  the JPEG's 0x003E is 1, 2 or 3 as its size is large, medium or
  small, while the NEF beside it holds 1 every time, the Z 9 making no
  medium or small raw. [F] On my own D7200 JPEGs it is 1 on 4,310 and
  0 on 506, presumably those taken without a raw. [F][I] On neither
  body does a JPEG's 0x003E give a raw size. Quality (0x0004) is `FINE` or
  `BASIC`, the JPEG's compression, where the NEF's says `RAW`. [F]

## 11. The camera files, segment by segment

Each segment as marker and length, in file order up to SOS. [F]

| File | Segments | Frame header at | Bytes |
|---|---|---|---|
| Sony ILCE-7M3 | APP1 Exif 46,742; APP1 XMP 57,346; APP2 MPF 352; DQT; DHT; SOF0 | 105,002 | 8,323,072 |
| Sony ILCE-7RM2 | APP1 Exif 48,580; APP2 MPF 566; DQT; DHT; SOF0 | 49,706 | 13,762,560 |
| Sony DSC-RX100M4 | APP1 Exif 44,749; APP2 MPF 301; DQT; DHT; SOF0 | 45,610 | 8,488,968 |
| Sony ILCE-6000 | APP1 Exif 44,334; APP2 MPF 204; DQT; DHT; SOF0 | 45,098 | 7,798,784 |
| Sony DSLR-A100 | APP1 Exif 53,681; DQT; DHT; SOF0 | 54,239 | 4,002,446 |
| Nikon D5300 | APP1 Exif 65,534; APP1 XMP 2,046; APP2 MPF 4,094; DQT; SOF0; DHT | 71,816 | 7,822,985 |
| Nikon D70 | APP1 Exif 33,778; DQT ×2; SOF0; DHT ×4 | 33,920 | 4,178,961 |
| Canon G7 X | APP1 Exif 27,646; APP1 XMP 3,996; APP2 MPF 96; DQT; SOF0; DHT | 31,880 | 8,667,055 |

On these files, reading the segments up to SOS, the Exif and MPF segments,
and each MPF image up to its frame header takes one to three blocks of
64 KiB. [F]

## 12. Recommendation for raw-files

What [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)
decided, as steps. [I]

1. **Recognize the container.** A file whose first bytes are `FF D8 FF`
   is a JPEG.
2. **Walk the segments** from SOI, one step per fill byte or marker:
   skip fill bytes; take a stand-alone marker as it is; for any other
   marker read the length. Keep the first frame header, the first APP1
   segment whose payload starts `Exif\0\0` and the first APP2 segment
   whose payload starts `MPF\0`. Stop at the first SOS. Stop as well,
   keeping what was found, at a length below 2, a segment that runs past
   the end of the file, an EOI, or the step cap.
3. **Accept the file** when the walk found a frame header with neither side
   zero that is not lossless (`C3`, `C7`, `CB`, `CF`); otherwise refuse it
   with the `UnknownFormat` error.
4. **Read the Exif segment as a bounded block.** The TIFF header lies 6
   bytes into the payload, and its position is the base of every offset.
   Every IFD, value, thumbnail and MakerNote must lie inside the segment;
   one outside is dropped as damage. A segment whose TIFF header or IFD0
   cannot be read gives none of the values the segment records, no
   thumbnail and no MakerNote preview; the file still opens, with its own
   preview, its MPF images and its image size, which the frame header
   gives, not the segment.
5. **The previews** are the file itself (its whole byte range, with the
   main frame header's size); IFD1's JPEG thumbnail; each MPF entry after
   the first, at the MP header's position plus its offset, inside the file;
   and on a Nikon file the MakerNote PreviewIFD's JPEG inside the
   MakerNote's declared length. Each must start with SOI and hold,
   within its own bytes, a frame header that has no zero side and is not
   lossless; each takes IFD0's orientation.
6. **The metadata** comes from the standard tags, then the MakerNote by
   IFD0's Make, as for a NEF or an ARW; the image size is the main frame
   header's, and there is no raw size. The camera settings come the same
   way, but a JPEG gives no raw image size, whatever its MakerNote's
   ImageSizeRAW says (section 10).
7. **The focus point** is kept only when its frame has the JPEG's aspect
   ratio, to within one pixel of rounding in either size:
   `|fw × jh − fh × jw|` at most the longest of `fw`, `fh`, `jw` and
   `jh`, where `fw × fh` is the frame and `jw × jh` the JPEG, in stored
   orientation. Before that, a Nikon JPEG shot in a crop mode, whose
   MakerNote records a crop and holds no CropArea, has its point placed
   on the JPEG's own picture: a DX crop's scaled onto the JPEG's frame,
   a 1:1 or 16:9 crop's moved into the centered window of the recorded
   frame that has the JPEG's shape
   ([ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)).

## 13. Pitfalls

- **A length below 2** would make a reader step backwards or stay in
  place; it ends the walk, as a segment that runs past the end of the
  file does. [I]
- **`FF` bytes before a marker are legal**, in any number, but each is a
  step of the walk: a file with more of them than the step cap allows is
  refused. None of the eight camera files has one before its scan. [P][1]
  (B.1.1.2, p. 31) [F] [I]
- **A frame header after SOS** cannot be found by walking segments; one
  that gives a height of 0 has its height after the first scan. [P][1]
  (p. 36) [I]
- **Offsets in the Exif segment are not file offsets.** They count from
  the TIFF header, 12 bytes into a file whose Exif segment comes first,
  later behind a JFIF segment. MPF offsets count from the MP header, in
  another segment, and Sony's MakerNote offsets from the Exif segment's
  TIFF header. [P][3][4] [F]
- **A pointer that leaves the segment** (the ILCE-6000's tag 0x2001) is
  not a standard value; it is dropped, not followed. [F] [I]
- **An MPF image can start after bytes that are not zeros** (the G7 X)
  and after zeros elsewhere; follow its offset, never search for the next
  SOI. [F] [I]
- **The MP Entry's size of the first image** is not the file's size: MPF
  images and padding follow. [F]
- **The Software tag does not prove a file is the camera's**: a program
  that re-encodes the picture may keep the camera's Software value. The
  segment layout, the MakerNote and the native size do. [S] (see
  [test-data.md](test-data.md#3-how-the-files-were-chosen))
- **An export may keep the camera's MakerNote** with a focus frame that no
  longer matches its pixels; see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md). [I]

## 14. The directories a file holds

raw-files gives every entry of every TIFF directory it walks, as the
file records it
([ADR023](../../architecture-decisions/adr023-every-entry-a-file-records.md)):
in a JPEG, the directories of the Exif segment (section 4) and the
Multi-Picture Format's index IFD (section 7); and a Nikon or Sony
MakerNote's, in the layouts raw-files reads (section 10), listed after
the segment's directories and before the index: the MakerNote IFD as
`MakerNote` and Nikon's preview IFD as `MakerNote 0x0011`. This section
lists what camera JPEGs hold: the directories, how many entries each has
and of which types, the largest values, the entries a reader has to
leave out, and the encrypted and enciphered records. The tag names the
view gives are in [the tag names](../tag-names.md).

The numbers were taken on 3 November 2026 with a walker of my own,
written from the TIFF 6.0 and Exif standards and these notes, and with
a counting program over raw-files' own `open`; no other reader was run.
They cover 5,960 files: the 14 JPEGs of the test fixtures and
`raw-samples/`; 9 files of raw.pixls.us that raw-files opens as JPEG,
7 `.raw` and 2 `.dng` files, none of them a camera's own JPEG, since
raw.pixls.us collects raw files; and, as counts only, 5,937 JPEGs of my
own, from Nikon, Sony, Canon and Fujifilm bodies (99 of them my Z 9's).
"Public" below is the first two groups. **[F]** In this
section, **[F]** marks a count over these files rather than a fact
of one named file.

### 14.1 The directories

| Directory | Files | Entries, min / median / max | Types seen |
|---|---|---|---|
| IFD0 | 5,860 | 3 / 12 / 50 | BYTE, ASCII, SHORT, LONG, RATIONAL, UNDEFINED, SRATIONAL, DOUBLE |
| Exif | 5,860 | 1 / 37 / 49 | ASCII, SHORT, LONG, RATIONAL, UNDEFINED, SLONG, SRATIONAL |
| IFD1 | 5,800 | 6 / 7 / 15 | ASCII, SHORT, LONG, RATIONAL |
| Interoperability | 5,766 | 1 / 2 / 4 | ASCII, SHORT, UNDEFINED |
| GPS | 2,640 | 1 / 1 / 11 | BYTE, ASCII, SHORT, RATIONAL |
| Nikon MakerNote | 2,501 | 42 / 57 / 75 | BYTE, ASCII, SHORT, LONG, RATIONAL, UNDEFINED, SSHORT, SRATIONAL |
| Nikon MakerNote preview IFD (0x0011) | 1,343 | 7 / 7 / 7 | SHORT, LONG, RATIONAL |
| Sony MakerNote | 505 | 28 / 95 / 102 | BYTE, ASCII, SHORT, LONG, RATIONAL, UNDEFINED, SSHORT, SLONG, SRATIONAL |
| MP Index | 2,393 | 3 / 3 / 3 | LONG, UNDEFINED |

- **The Exif segment** is in 5,860 files; the 100 others have none.
  No camera JPEG has a SubIFD; one raw.pixls.us file opened as JPEG has
  a "SubIFD" of 540 entries of types TIFF does not define (14.3).
  **[F]**
- **The MakerNote** is walked on 2,501 Nikon files (in the
  embedded-TIFF layout) and 505 Sony files (behind the `SONY DSC`
  header), and raw-files lists each of them; 2,837 files of other
  makers, Canon's and Fujifilm's among them, have a MakerNote no notes
  describe, and 117 have none. The Sony MakerNote's next-IFD slot holds
  its first value on 504 files, and is not followed. **[F]**
- **The MP Index** is in the 2,393 files with a Multi-Picture Format
  segment; no file has an MP Attribute IFD. **[F]**
- **No entry has the types SBYTE, FLOAT, IFD (13) or Exif's UTF-8
  (129).** **[F]**
- **A file holds 104 entries** (the median; at most 163, and 615 in
  the damaged raw.pixls.us file) **and about 8 KB of values** (8,046
  bytes, the median; at most 38,337), not counting the MakerNote entry
  itself. **[F]**

### 14.2 The largest values

| Directory | Tag | Type | Largest | Median | Files with 256 bytes or more, public / mine |
|---|---|---|---|---|---|
| Exif | 0x927C MakerNote | UNDEFINED | 44,000 | 14,360 | 21 / 5,822 |
| Nikon MakerNote | 0x0091 ShotInfo | UNDEFINED | 35,785 | 11,168 | 8 / 2,440 |
| Sony MakerNote | 0x940E | UNDEFINED | 12,288 | 12,288 | 3 / 500 |
| IFD0 | 0x0111 StripOffsets | LONG (1,908 values) | 7,632 | 7,632 | 1 / 0 |
| Sony MakerNote | 0x9411 | UNDEFINED | 6,656 | 6,656 | 1 / 0 |
| Sony MakerNote | 0x2010 | UNDEFINED | 6,604 | 6,556 | 4 / 500 |
| Sony MakerNote | 0x9401 | UNDEFINED | 4,096 | 4,096 | 4 / 500 |
| Sony MakerNote | 0x9410 | UNDEFINED | 3,656 | 3,656 | 1 / 0 |
| Sony MakerNote | 0x9405 | UNDEFINED | 2,300 | 2,048 | 4 / 500 |
| IFD0, Exif | 0xEA1C | UNDEFINED | 2,060 | 2,060 | 0 / 78 |

The ten largest values, sizes in bytes. **[F]**

- **No value inside the segment passes 64 KiB**, since the Exif
  segment itself holds at most 65,533 bytes (section 4), far under the 1 MiB cap on one value
  ([ADR023](../../architecture-decisions/adr023-every-entry-a-file-records.md)).
  The one larger value a camera JPEG declares, a Sony MakerNote's
  0x2001, lies outside the segment (14.3).

### 14.3 Entries a reader leaves out

The view leaves out a damaged entry and counts it in its directory,
and a directory that cannot be read is not listed (ADR023). On the
JPEG files: **[F]**

| Directory | What | Files, public / mine |
|---|---|---|
| Sony MakerNote | 0x2001, declared 549,518 bytes, lies outside the Exif segment | 1 / 0 (the ILCE-6000 file, section 10) |
| Interoperability | the pointer to it leaves the Exif segment | 0 / 1 |
| Nikon MakerNote preview IFD | more than 1,000 entries | 0 / 1 |
| SubIFD | 540 entries of tags 0x2000 and 0xA000, of types TIFF does not define | 1 / 0 (a raw.pixls.us file) |

- **The D70 file's Interoperability pointer leads to IFD1** (section
  6), which the walk has already listed in IFD0's chain; the view keeps
  IFD1 there and lists no Interoperability IFD for that file. **[F]**
- **Irregular but readable, and kept:** tags out of ascending order in
  every Sony MakerNote, in the Exif IFD of 10 of my files and in IFD1
  of 1; a tag written twice in IFD1 of 1 of my files (5 tags); a value
  at an odd offset in IFD0, the Exif IFD or a Nikon MakerNote of at
  most 6 files. 5,439 of the 5,960 files have no
  irregularity of any kind. **[F]**
- **No file has** a count or an offset that overflows or a value past
  the end of the file. **[F]**

### 14.4 The encrypted and enciphered records

As in the raw files, these records are plain UNDEFINED entries to a
TIFF reader; the view gives their bytes as recorded and does not mark
them
([ADR023](../../architecture-decisions/adr023-every-entry-a-file-records.md)).
**[F]**

- **Nikon's ShotInfo, ColorBalance and LensData**, encrypted by their
  version as the
  [NEF notes](../nef/makernote-and-metadata.md#4-encrypted-blocks)
  describe: plain on the D70 file (ShotInfo and ColorBalance 0103,
  LensData 0101); encrypted on the D5300 file (ShotInfo 0229,
  ColorBalance 0219, LensData 0204) and on all 2,493 of my Nikon JPEGs
  (ShotInfo 0216 to 0236 and, on the Z 9, 0805; ColorBalance 0212 to
  0224 and 0803; LensData 0204 and 0802). The records of the six
  stripped D600 and Z 9 fixtures carry no version.
- **Sony's enciphered records**, by tag as the
  [ARW notes](../arw/makernote-and-metadata.md#4-the-makernote-substitution-cipher)
  describe: 0x2010, 0x9050, 0x9400 to 0x9406, 0x940C and 0x940E, each
  on 503 or 504 of the 505 Sony JPEGs.

### 14.5 What the view leaves out

- **Segments that are not TIFF directories** (section 5): XMP (3
  public files, 2,122 of mine), FlashPix extensions (2,139 of mine),
  ICC profiles (16), Photoshop segments holding IPTC resources (10),
  JFIF (2 public, 142 of mine), Adobe (4) and comments (1). XMP and
  IPTC are for full metadata's group for them. **[F]**
- **Other makers' MakerNotes**, on 2,837 files: the view gives their
  bytes through the Exif IFD's 0x927C entry, and walks none of them
  until notes describe them.
- **Nikon's short-header layout** of the earliest Coolpix JPEGs
  ([NEF notes](../nef/makernote-and-metadata.md#in-a-jpeg-file)),
  which raw-files does not read; none is among the files counted here.
- **The content of the encrypted and enciphered records**: their bytes
  only, until a decision allows reading them (ADR016).

## Sources

[1] ITU-T Recommendation T.81 (09/92) | ISO/IEC 10918-1:1993, "Digital
    compression and coding of continuous-tone still images: Requirements
    and guidelines". Read in the CCITT publication hosted by the W3C,
    https://www.w3.org/Graphics/JPEG/itu-t81.pdf; the ITU's page,
    https://www.itu.int/rec/T-REC-T.81-199209-I/en, offers it for a fee.
    **[P]**
[2] ITU-T Recommendation T.871 (05/2011) | ISO/IEC 10918-5:2012, "JPEG
    File Interchange Format (JFIF)",
    https://www.itu.int/rec/T-REC-T.871-201105-I/en (freely available).
    **[P]**
[3] CIPA DC-008-Translation-2026, "Exchangeable image file format for
    digital still cameras: Exif Version 3.1" (January 2026),
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-008-2026-E.
    **[P]**
[4] CIPA DC-007-Translation-2026, "Multi-Picture Format",
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-007-2026_E.
    **[P]**
[5] The eight camera-original JPEGs from Wikimedia Commons listed in
    [test-data.md](test-data.md#2-the-sample-files), walked byte by byte
    with a script of my own in October 2026. **[F]**
[6] ExifTool, EXIF tags, https://exiftool.org/TagNames/EXIF.html, read
    as documentation. **[S]**

All pages were retrieved in October 2026.
