# Nikon MakerNote and viewer metadata

This page describes the Nikon MakerNote found in NEF files: its three
layouts, the tags that matter, the encrypted blocks and how they are
decrypted, and where a viewer finds orientation, capture date, make, model
and image size, where the camera recorded the point it focused on,
how the camera was set for the shot, and who made the picture and where
it was taken.
It serves three of the [roadmap](../../ROADMAP.md)'s capabilities:
metadata, both what raw-files reads from the plain tags and full
metadata; focus points; and, for the decoding-related tags, raw sensor
data.

See [the NEF overview](README.md). The TIFF container and the preview JPEGs
are in [container-and-previews.md](container-and-previews.md); decoding of
the sensor data is in [raw-data.md](raw-data.md).

Nikon publishes no specification for any of this. Everything below comes
from the Exif and TIFF standards, the tag documentation of ExifTool and
exiv2, and the source of existing decoders, read as references only (their
licenses do not allow copying code; see [Licenses](#licenses)). Claims carry
the confidence tags of the NEF notes ([README](README.md#confidence-tags)),
and a sample file is cited by its raw.pixls.us id ("file 928"). A fact measured on my own
files, which stay private, says so and cites them by counts only, or by
the frame numbers of a session. The MakerNote layout and the viewer
metadata (sections 1, 2 and 8) were checked in October 2026 on 23 CC0
files from the D1H to the Z6III, listed in
[test-data.md](test-data.md#21-the-ci-set), and the focus point (section
8.6) on those and seven more CC0 files, listed in
[test-data.md](test-data.md#23-files-checked-for-the-focus-point), and on
frames shot with my own D600, recorded in
[autofocus-points.md](autofocus-points.md), and the
camera settings (section 8.7) on the 35 files of the CI set and two
Nikon camera JPEGs, listed in
[test-data.md](test-data.md#24-files-checked-for-the-camera-settings),
where ExifTool was also run on the files as an independent reader to
compare with; none of its code is in raw-files. Where a file
contradicts a source, this page follows the file and records the
disagreement. Byte offsets and version numbers taken from implementation
sources were read through summaries; verify them against the source, and
against sample files, before hard-coding them.

Nikon's camera JPEGs carry the same MakerNote, in their Exif segment. What
the two Nikon JPEGs of the [JPEG test data](../jpeg/test-data.md#2-the-sample-files)
show, a D5300's and a D70's from Wikimedia Commons, cited as "the D5300
JPEG", is in [section 1](#in-a-jpeg-file) and
[section 8.6](#86-focus-point). The two older layouts were checked by
hand on JPEGs of early Coolpix bodies from Wikimedia Commons, cited by
their Commons file name, listed in the
[JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand)
and described in [section 1](#in-a-jpeg-file) too. How raw-files reads
a JPEG is decided in
[ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)
and described in the [JPEG notes](../jpeg/README.md).

## Contents

- [1. Finding the MakerNote](#1-finding-the-makernote)
- [2. Key tags](#2-key-tags)
- [3. NEFCompression and bit depth](#3-nefcompression-and-bit-depth)
- [4. Encrypted blocks](#4-encrypted-blocks)
- [5. The decryption algorithm](#5-the-decryption-algorithm)
- [6. White balance](#6-white-balance)
- [7. LensData and ShotInfo](#7-lensdata-and-shotinfo)
- [8. Viewer metadata](#8-viewer-metadata)
- [9. Robustness pitfalls](#9-robustness-pitfalls)
- [10. Recommendation for raw-files](#10-recommendation-for-raw-files)
- [11. Open questions](#11-open-questions)
- [12. The directories a file holds](#12-the-directories-a-file-holds)
- [Sources](#sources)

## Licenses

Every mature NEF implementation is GPL, LGPL, CDDL or Artistic licensed
(licenses incompatible with the raw-files MIT OR Apache-2.0 license, or
ambiguous in dcraw's case). This page therefore restates algorithms in prose
and cites the two decryption tables by location only; do not paste code or
tables from those projects into raw-files. [P]

## 1. Finding the MakerNote

The MakerNote is Exif tag 0x927C (type UNDEFINED) in the Exif IFD. [S][5]
Nikon has used three layouts. Projects number them differently (exiv2 calls
them Nikon1, Nikon2, Nikon3; one older write-up swaps the first two; ExifTool
uses yet other names), so this page names them by shape and raw-files should
detect them by bytes, never by a "type" number. [S][5][6][12]

| Layout | Header | IFD starts at | Offsets relative to | Byte order | Bodies |
|---|---|---|---|---|---|
| Headerless | none | MakerNote + 0 | the outer TIFF header | the outer file's (the D1 is big-endian) | confirmed on the D1H and the D1X (files 5364, 1785 and 1786) and on the D1 (file 201, not in the CI set), big-endian, file and MakerNote alike **[F]**; in camera JPEGs of the Coolpix E990, E995 and E880, and of the E885, E2500 and E5000 on their earlier firmware, little-endian, Exif segment and MakerNote alike ([In a JPEG file](#in-a-jpeg-file)) **[F]**; the E-series Coolpix files of raw.pixls.us, the E5400, E5700, E8400 and E8800 (files 212, 4456, 2082 and 2769), write the embedded-TIFF layout instead **[F]** |
| Short header | `Nikon\0` `01 00` (8 bytes) | MakerNote + 8 | the outer TIFF header | ExifTool's documentation says little-endian, and exiv2 uses the outer order; every file checked is little-endian, outside the MakerNote and inside, the one case where both agree **[F]** | in camera JPEGs of the Coolpix E700, E800, E900 and E950 ([In a JPEG file](#in-a-jpeg-file)) **[F]** |
| Embedded TIFF header | `Nikon\0`, version `02 00`, `02 10` or `02 11`, `00 00`, then a TIFF header (`II*\0` or `MM\0*` and a 4-byte offset) (18 bytes) | MakerNote + 10 + the inner IFD offset (normally 8, so + 18) | MakerNote + 10, the inner TIFF header | the inner header's, independent of the file's | D70, D100, D200 and later DSLRs, every Z body; every checked file but the D1H's and the D1X's: `02 00` on the D100 and the Coolpix E5700 and P7800, `02 10` from the D70 to the D7000 and on the Nikon 1 J1, `02 11` on the Coolpix A (2013) and on every body from the D810 (2014) on **[F]** |

Sources: [5][6][4][7]. The D1 byte-order remark is from ExifTool. [S]

NEFs from DSLRs (D100 era onward) and all mirrorless bodies use the embedded
TIFF header layout. The other two layouts matter only for the earliest
bodies; a RAW library must not crash on them but can treat them as
low-priority. [I]

### Detection by bytes

1. If the MakerNote starts with `Nikon\0`:
   - if it is at least 18 bytes long, byte 6 is 2, byte 10 is a valid byte
     order mark (`II` or `MM`) and the next u16 (read in that order) is 42,
     it is the embedded-TIFF layout;
   - otherwise it is the short-header layout, IFD at + 8.

   Testing byte 6 rather than the whole version accepts the `02 11` that
   recent bodies write beside `02 10` and `02 00` (files 5146 to 5148 and
   the bodies listed in the table above). **[F]**
2. Otherwise, if Make starts with "NIKON" (any case), it is the headerless
   layout, IFD at + 0.
3. Otherwise the MakerNote is not Nikon's; ignore it.

exiv2, LibRaw and ExifTool find the embedded layout in the same real
files. [S][6][7][4] Testing all of the conditions above is the robust
choice.

Check the signature before Make. Capture NX can write Nikon-format
MakerNotes into files from other brands, so the Make test must not gate the
embedded layout. [S][4]

raw-files reaches the MakerNote reader only under a Make that starts with
`NIKON`: the NEF claim and the JPEG reader test IFD0's Make first, so the
Make gates every layout, the embedded one included
([section 10](#10-recommendation-for-raw-files)). Under that Make:

- a MakerNote that starts with the 8 bytes `Nikon\0\1\0` is read as the
  short-header layout, and only when the TIFF structure that holds it
  is little-endian ([In a JPEG file](#in-a-jpeg-file));
- any other MakerNote that starts with `Nikon\0` is read as the
  embedded-TIFF layout or not at all, never retried as headerless;
- any other MakerNote is read as headerless, an IFD at + 0, whose first
  two bytes are its count of entries.

### Layout details

- The inner IFD offset is relative to the inner TIFF header. A gap between
  the 8-byte inner header and the IFD is legal. A file with one that
  exiv2 rewrites comes out without the gap. [S][6]
- The inner byte order can differ from the outer file's, for example a
  big-endian NEF carrying a little-endian MakerNote. Always use the inner
  header's order for the IFD itself. [S][3]
- Some sub-blocks have a fixed byte order regardless of the MakerNote's: [S][3]
  - 0x0014 (ColorBalanceA, Coolpix) and 0x0025 (ISOInfo) are big-endian;
  - LensData versions 08xx and ShotInfo on newer bodies (D780, D7500 and
    later, and the Z bodies) are little-endian, while older D-series
    ShotInfo is big-endian;
  - Capture NX can change the MakerNote's byte order. ColorBalance 0x0097
    and WorldTime then keep their original order, and ShotInfo stays
    big-endian.
- The D200's MakerNote IFD has no next-IFD pointer; do not require one. [S][5]
- Offsets in the headerless and short-header layouts are relative to the
  outer TIFF header, so anything that moves the MakerNote breaks them. The
  embedded layout survives being moved as a block. [S][5] Early Coolpix
  JPEGs confirm the base for both layouts, and an E885 JPEG that an
  editor turned upright shows the breakage: its headerless MakerNote
  lies 342 bytes nearer the Exif segment's TIFF header than in the
  camera's own E885 files, its offsets unchanged, so every value stored
  out of line lies 342 bytes before the place its entry points at, and
  the PreviewIFD pointer reaches no IFD (`Priere-mosquee-saint-denis.JPG`
  on Wikimedia Commons, listed in the
  [JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand)).
  **[F]**
- The D1H's headerless MakerNote is bytes 898 to 28,256 of the file
  (27,358 bytes): a big-endian IFD of 27 entries at its first byte, whose
  table ends at 1224, with a next-IFD pointer of 0. Its PreviewIFD
  pointer (0x0011, a LONG held in its entry) is 3582, where the
  PreviewIFD's 7 entries are; the PreviewIFD's JPEG, 570x375, is bytes
  3688 to 28,255, one byte short of the block's end. Every one of these
  offsets counts from the start of the file, and all of them lie inside
  the MakerNote's declared length (file 5364). **[F]**

### In a JPEG file

In a camera JPEG the MakerNote is an entry of the Exif IFD inside the
Exif segment, an APP1 segment that holds a TIFF structure of its own;
that structure's header, 12 bytes into the file when the segment comes
first, is the base of its offsets
([JPEG notes](../jpeg/container-and-metadata.md#4-exif-the-app1-segment)).
**[P]** In a JPEG, what this page calls the outer TIFF header is that
header.

- **The embedded-TIFF layout reads unchanged**, since its offsets count
  from its own inner header. The D5300 JPEG's MakerNote starts `Nikon\0`,
  `02 11`, `00 00` and an inner `II` header, with 58 entries; the D70
  JPEG's starts `Nikon\0`, `02 10`, `00 00` and an inner `MM` header,
  with 42 entries, inside a file whose own TIFF structure is
  little-endian, while the D70's NEF is big-endian (file 2060). **[F]**
  The inner byte order is independent of the file's in a JPEG too.
- **Every value read lies inside the Exif segment and the MakerNote's
  declared length** on the D5300 and D70 files: the ISO (`[0, 400]` and
  `[0, 200]`), the lens range, and on the D5300 AFInfo2 and CropHiSpeed.
  **[F]**
- **The D70 JPEG's PreviewIFD holds a JPEG of 564x372**, inside the
  declared length, as the PreviewIFD of the D70's NEF does (file 2060);
  the D5300 JPEG has no PreviewIFD. raw-files offers the D70's as a
  preview of the JPEG, under the same in-block rule as in a NEF
  ([ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
  **[F]**

The two older layouts appear in the JPEGs of early Coolpix bodies.
Camera-original JPEGs of eleven of them on Wikimedia Commons were read
byte by byte, as evidence checked by hand rather than as samples; the
[JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand)
lists them with their licenses. Every one is little-endian, Exif segment
and MakerNote alike. **[F]**

- **The headerless layout**, in all 18 files of the E990 (firmware v1.0
  and v1.1), E995 (v1.6 and v1.7), E880 (v1.0 and v1.1), E885 (v1.0 and
  v1.1), E2500 (v1.0) and E5000 (v1.6): the MakerNote's first two bytes
  are its count of entries, 16 on the E990, 17 on the E880 and 21 on
  the others, which read big-endian, 4,096 or more, would make a table
  longer than the MakerNote. Its 13 to 16 values stored out of line all
  lie inside the MakerNote when their offsets count from the Exif
  segment's TIFF header, and none does when they count from the
  MakerNote's start (`2006T629_finger_ring.jpg`, an E990's;
  `Adastra_Park_-_S_Field_1.jpg`, an E2500's). **[F]** Later firmware
  of the E885 (v1.2), the E2500 (v1.1) and the E5000 (v1.7), and every
  E4500 file (v1.1 to v1.3), write the embedded-TIFF layout instead.
  **[F]** raw-files reads the headerless layout in a JPEG as in a NEF,
  at the Exif segment's TIFF header and in that structure's byte order.
  On these files it gives nothing the Exif segment lacks: the ISO tag
  (0x0002) holds `[0, 0]`, which records no ISO, and the Exif segment
  records one. **[F]**
- **The headerless PreviewIFD's JPEG lies outside the MakerNote** on the
  E885, E995, E2500 and E5000 files; the E990 and E880 have no
  PreviewIFD. Tag 0x0011 points at an IFD of 7 entries at 1,764, inside
  the MakerNote, which is 954 bytes long from 916; that IFD's JPEG,
  320x240, starts at 14,397, 12,527 bytes past the MakerNote's end, and
  ends inside the Exif segment, all offsets counted from the Exif
  segment's TIFF header (`Adastra_Park_-_S_Field_1.jpg` and
  `150019_and_158764_at_Worcester_Shrub_Hill.jpg`, both an E2500's).
  **[F]** raw-files does not give it, under the in-block rule
  ([section 9](#9-robustness-pitfalls), item 1).
- **The short-header layout**, in all 18 files of the E700, E800, E900
  and E950: the MakerNote starts `Nikon\0`, `01 00`, and an IFD follows
  those 8 bytes, of 11 entries, 5 on the E900, with a next-IFD pointer
  of 0 and no PreviewIFD. Read big-endian, the count would be 2,816, or
  1,280, a table longer than the MakerNote. Its values stored out of
  line, five, two on the E900, all lie inside the MakerNote when their
  offsets count from the Exif segment's TIFF header, and none does when
  they count from the MakerNote's start or from the end of its 8 bytes
  (`MIKI-eki.jpg`, an E800's; `Yamaha_FJ1100.jpg` and
  `Petro_Star_refinery.jpg`, both an E900's;
  `Starr_020628-0045_Prosopis_pallida.jpg`, an E950's). **[F]**
  raw-files reads the IFD 8 bytes in as it reads the headerless
  layout's. It lists the entries in the raw view and gives no value
  from them (below).
- **The short header is read only in a little-endian structure.**
  ExifTool's documentation says the layout's numbers are always
  little-endian, and exiv2 reads them in the byte order of the TIFF
  structure that holds the MakerNote (table above). [S] All 18 files
  are little-endian outside the MakerNote and inside, the one case
  where both agree. **[F]** So raw-files reads the layout only in a
  little-endian structure, and leaves it unread in a big-endian one
  rather than guess. A big-endian file would settle that case
  ([open questions](open-questions.md#container-and-previews)).

**The short header's entries.** The E700, E800 and E950 write 11, and
the E900 5. The table gives each tag's type and count, what the 18
files hold, and its name in ExifTool's published tag table for the
layout, which lists names and no values. [F] [S][1]

| Tag | Type, count | Bodies | What the 18 files hold **[F]** | ExifTool's name [S][1] |
|---|---|---|---|---|
| 0x0002 | ASCII, 6 | all four | `08.00`, or `09.41` on the E900 | none |
| 0x0003 | SHORT, 1 | all four | 11 or 12, and 4 to 6 on the E900 (below) | Quality |
| 0x0004 | SHORT, 1 | all four | 1 | ColorMode |
| 0x0005 | SHORT, 1 | all four | 0 | ImageAdjustment |
| 0x0006 | SHORT, 1 | not the E900 | 0, or 5 on one E700 file | CCDSensitivity |
| 0x0007 | SHORT, 1 | not the E900 | 0, or 5 on two E700 files | WhiteBalance |
| 0x0008 | RATIONAL, 1 | not the E900 | 0/0, or 1/0 on one E950 file | Focus |
| 0x0009 | ASCII, 20 | not the E900 | empty | none |
| 0x000A | RATIONAL, 1 | not the E900 | 0/100 | DigitalZoom |
| 0x000B | SHORT, 1 | not the E900 | 0 | Converter |
| 0x0F00 | LONG, 30; UNDEFINED, 122 on the E800; LONG, 25 on the E900 | all four | values neither reader names | none |

exiv2's documentation lists the same tags with the same types, under
names of its own, and calls 0x0002, 0x0009 and 0x0F00 unknown. [S][24]
ExifTool 13.50 and exiv2 0.28.9, run on all 18 files as readers to
compare, list the same entries; ExifTool reports a little-endian
directory of 11 entries, or of 5. [S]

**raw-files reads no value from them.** On these files, no tag's
meaning is confirmed where it would add a value the Exif segment
lacks. **[F]**

- 0x0002 holds a text. The later layouts' ISO rule, two SHORTs under
  that tag, never applies to it.
- 0x0006 is the ISO setting by its name. It holds 0 beside an Exif ISO
  of 100 on all four E800 files, beside 80 on three E950 files, and
  beside no Exif ISO on four E700 files. So 0 stands for no one ISO.
- exiv2, run as a reader, prints that 0 as ISO 80, which the E800 files
  contradict. [S] The 5 of 0x0006 sits beside an Exif ISO of 100 on its
  one file (`Pokfulam_Service_Reservoir_1.jpg`), where Exif already
  gives it.
- 0x0007 holds 5 on two E700 files, each beside the Exif LightSource
  20, the standard light D55, and 0 elsewhere. exiv2 prints 5 as cloudy
  and 0 as auto. [S] A standard light names no setting, so the files
  confirm neither name.
- 0x0003 follows the JPEG's compression. At 1600×1200 it is 11 at 2
  compressed bits per pixel and 12 at 4; on the E900, at 1280×960, it
  is 4, 5 and 6 at 1, 2 and 4. `Metadata` holds no JPEG quality.

The other tags hold one value on every file, or values neither reader
names. The files are listed in the JPEG test data, three as samples in
[section 2](../jpeg/test-data.md#2-the-sample-files) and the others in
[section 6](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand).

## 2. Key tags

IDs are for the embedded-TIFF layout. Types are as documented by ExifTool;
read integers from the bytes at the declared count rather than trusting the
declared type (see [section 9](#9-robustness-pitfalls)). "Used for" names
the [roadmap](../../ROADMAP.md)'s capabilities that use the tag:
"metadata" is what a viewer or a file renamer needs next to the
preview, read from the plain tags, and "full metadata" the rest of
what the file records, the open part of metadata; "raw data" is raw
sensor data. [S][1]

| ID | Name | Type | Meaning | Used for |
|---|---|---|---|---|
| 0x0001 | MakerNoteVersion | undef[4] | ASCII digits on some bodies, binary on others; do not rely on it | full metadata |
| 0x0002 | ISO | u16[2] | `[0, ISO]` on most bodies before the D810; absent from the D810 on; four zero bytes of type UNDEFINED on the D700 (file 2255), `[0, 0]` on the E5700 (file 4456) **[F]** | metadata |
| 0x0004 | Quality | string | the image quality as text: `RAW` or `NRW` in a raw file, `FINE`, `NORMAL` or `BASIC` in a camera JPEG; on every Nikon file measured; values in [section 8.10](#810-more-capture-settings) **[F]** | full metadata |
| 0x0005 | WhiteBalance | string | the white balance setting, as upper-case text padded with spaces; on every checked body; values in [section 8.7](#87-camera-settings) **[F]** | metadata, full metadata |
| 0x0007 | FocusMode | string | the focus mode as text, padded with spaces: `AF-S`, `AF-C`, `AF-A`, `AF-F` or `MANUAL`; on every Nikon file measured; [section 8.10](#810-more-capture-settings) **[F]** | focus points, full metadata |
| 0x0008 | FlashSetting | string | the flash's sync as text, such as `NORMAL` or `REAR SLOW`, or blank; [section 8.10](#810-more-capture-settings) **[F]** | full metadata |
| 0x0009 | FlashType | string | the flash's source and control as text, such as `Built-in,TTL` or `Optional,M`, or blank; [section 8.10](#810-more-capture-settings) **[F]** | full metadata |
| 0x000B | WhiteBalanceFineTune | s16[1 or 2] | the white balance fine-tuning: one value from the D1H to the D200, two from the D300 on; not written by the Z 9, Z f, Z5II, Z50II and Z6III, which write 0x003F alone **[F]** | metadata |
| 0x000C | WB_RBLevels | rational[4] | as-shot white-balance multipliers; LibRaw reads them as R, B, then two greens (ExifTool leaves the last two unidentified) | full metadata, raw data |
| 0x000F | ISOSelection | string | `AUTO` or `MANUAL`, on the Coolpix bodies that write no ShootingMode, all but the Coolpix A **[F]** | full metadata |
| 0x0011 | PreviewIFD | u32 | the offset of a sub-IFD with a small JPEG preview; offsets relative to MakerNote + 10. One LONG in every checked file that has it, as are the sub-IFD's 0x0201 and 0x0202; raw-files reads them in no other shape **[F]** | previews |
| 0x0012 | FlashExposureComp | undef[4] | the built-in flash's compensation: a signed count, then 1, 6 and 0; absent on bodies without a built-in flash, the Z 9 among them **[F]** | full metadata |
| 0x0017 | ExternalFlashExposureComp | undef[4] | an external flash unit's compensation, in the same four bytes **[F]** | full metadata |
| 0x001B | CropHiSpeed | u16[7] | crop mode and geometry; field order in [section 8.4](#84-image-dimensions) | metadata, focus points, raw data |
| 0x001C | ExposureTuning | undef[3] | the exposure fine-tuning: a signed count, then 1 and 6 **[F]** | full metadata |
| 0x001D | SerialNumber | string | the body's serial number; decryption key input. Seven digits on every checked DSLR and mirrorless body from the D300 on (eight on the Nikon 1 J1) and on the Coolpix A; seven spaces on the D200 (file 8079); absent before it and on the Coolpix E5700 and P7800 **[F]** | metadata, full metadata, raw data |
| 0x001E | ColorSpace | u16 | the color space setting: 1 sRGB, 2 Adobe RGB, 4 BT.2100; from the D200 on, but not on the Coolpix P7800, while no checked NEF has the Exif ColorSpace **[F]** | metadata |
| 0x001F | VRInfo | undef[8 or 9] | vibration reduction: a four-digit version, 0100, 0101 or 0200, then whether it was on and its mode **[F]** | full metadata |
| 0x0021 | FaceDetect | u16 | the size of a frame and up to twelve face rectangles in it; on none of the 30 checked files **[F]** | full metadata |
| 0x0022 | ActiveD-Lighting | u16 | Active D-Lighting: 0 off, 1 low, 3 normal, 5 high, 7 and 8 to 11 the extra high steps, 65535 auto; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0023 | PictureControlData | undef[58, 68 or 108] | the Picture Control: a four-digit version, the name, the base and the values, in three layouts by version; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0024 | WorldTime | undef | the time zone and daylight saving setting of the camera's clock; layout in [section 8.2](#82-capture-date-and-time-zone) | metadata, full metadata |
| 0x002A | VignetteControl | u16 | vignette control: 0 off, 1 low, 3 normal, 5 high; not written by the D300S, D3200, D3300 and D7100; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x002B | DistortInfo | undef[16] | a record of version `0100`, whose byte 4 is auto distortion control: 0 off, 1 on, 2 on for underwater; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0034 | ShutterMode | u16 | the shutter used: mechanical, electronic front curtain or electronic, and whether the camera chose it in its auto mode; on the Nikon 1 J1, D850, D780 and every Z body; codes in [section 8.7](#87-camera-settings) **[F]** | metadata |
| 0x0035 | HDRInfo | undef[6 or 8] | HDR: a four-digit version, 0100, 0101 or 0200, then on or off and its parts; in JPEGs, and in the raw files of Coolpix and Nikon 1 bodies; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0037 | MechanicalShutterCount | u32 | newer bodies; mechanical frames only | full metadata |
| 0x003D | BlackLevel | u16[4] | per-channel black level, recorded at 14-bit scale | raw data |
| 0x003E | ImageSizeRAW | u16 | the raw size setting: 1 large, 2 medium, 3 small; from the D750, D810 and D5600 on **[F]** | metadata, raw data |
| 0x003F | WhiteBalanceFineTune | srational[2] | the white balance fine-tuning as two fractions; from the D810 on **[F]** | metadata |
| 0x0044 | JPGCompression | u16 | the JPEG compression: 1 size priority, 3 optimal quality, in my Z 9's JPEGs; 0 in every NEF that has it; [section 8.10](#810-more-capture-settings) **[F]** | full metadata |
| 0x0045 | CropArea | u16[4] | left, top, width, height of the output rectangle | metadata, focus points, raw data |
| 0x004F | ColorTemperatureAuto | u16 | the color temperature, in kelvin, of the white balance the camera applied: the Z 9 records it under every setting, the D850 and the Z fc write 0 under daylight; on the D5, D850, D780 and every Z body **[F]** | metadata |
| 0x0051 | (unnamed, Z bodies) | undef | NEFCompression as a u16 at byte 10; byte order disputed (see the note below). Present on the Z 6 (2018) too, and on the Z f, Z6III, Z 9, Z5II and Z50II (files 3587, 6887, 7808 to 7810, 7812, 7813, 5146 to 5148, 7735, 7736, 7767, 7770); absent on the Z fc (2021) and Z 30 (2022), which write 0x0093 instead (files 8559, 5812). Byte 10, read as a u16 in the MakerNote's byte order, little-endian on the three Z 9 files, is 3 (lossless) on the Z 9's lossless file, 14 (HE\*) on its HE\* file and 13 (HE) on its HE file (files 5146 to 5148) **[F]** | full metadata, raw data |
| 0x0084 | Lens | rational[4] | the lens's shortest and longest focal length and the widest aperture at each; on every checked body with interchangeable lenses, from the D1H on; equal to Exif LensSpecification wherever both exist **[F]** | metadata |
| 0x0087 | FlashMode | u8 | whether and how the flash fired; not read, since it contradicts Exif's "fired" on hundreds of files ([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)); [section 8.7](#87-camera-settings) **[F]** | full metadata |
| 0x0088 | AFInfo | undef[4] | area mode, the number of the focus point and a mask of the points in focus; no position. The only focus data of the checked bodies without AFInfo2: the D1H, D70, D100, D200 and Coolpix E5700 and P7800, and written by no checked body that has AFInfo2 **[F]**; its first byte, the area mode, will be read; [section 8.6](#86-focus-point) | focus points, full metadata |
| 0x0089 | ShootingMode | u16 (u8 on the D1 family and the D100) | bits for the release mode, bracketing and auto ISO; [section 8.10](#810-more-capture-settings) **[F]** | full metadata |
| 0x008F | SceneMode | string | the scene mode as text, on the Coolpix bodies; blank outside a scene or auto mode; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0091 | ShotInfo | undef | model-specific; encrypted for versions 02xx and 08xx | full metadata |
| 0x0093 | NEFCompression | u16 | compression hint; absent on the Z bodies checked but the Z fc and Z 30, which write 0x0051 instead, and on the D1H and E5700; present on the Z fc and Z 30, with the value 3 (lossless), where 0x0051 is absent (files 8559, 5812) **[F]** | full metadata, raw data |
| 0x0095 | NoiseReduction | string | `FPNR` where the long exposure noise reduction ran on the picture, else `OFF`; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0096 | NEFLinearizationTable | undef | curve and predictor seeds; unencrypted | raw data |
| 0x008C | (older curve) | undef | the curve on a file without 0x0096 | raw data |
| 0x0097 | ColorBalance | undef | white balance on older bodies; encrypted from version 0200 | full metadata, raw data |
| 0x0098 | LensData | undef | lens description; encrypted from version 0201 | full metadata |
| 0x00A7 | ShutterCount | u32 | decryption key input | full metadata, raw data |
| 0x00AB | VariProgram | string | the scene mode as text, such as `AUTO(FLASH OFF)`, on the DSLR and Z bodies; blank outside a scene or auto mode; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x00B1 | HighISONoiseReduction | u16 | high ISO noise reduction: 0 off, 1 minimal, 2 low, 3 medium low, 4 normal, 5 medium high, 6 high; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x00B7 | AFInfo2 | undef | where the camera focused and the AF-area mode: a block that starts with a four-character version, plain on every version; layouts in [section 8.6](#86-focus-point) | focus points, full metadata |
| 0x00B9 | AFTune | undef[4] | AF fine-tune: on or off, the saved lens's index, the value and the value at a zoom's long end; [section 8.10](#810-more-capture-settings) **[F]** | full metadata |
| 0x00BD | PictureControlData | undef[58] | the Picture Control of the Coolpix bodies, in the first of its layouts; [section 8.11](#811-picture-style-and-processing) **[F]** | full metadata |
| 0x0E01 | NikonCaptureData | undef | Capture NX edit data (optional) | full metadata |
| 0x0E22 | NEFBitDepth | u16[4] | secondary bit-depth source | full metadata, raw data |

Notes:

- 0x00A7 on newer bodies is the combined mechanical and electronic count;
  0x0037 is mechanical-only. The decryption key uses 0x00A7. [S][1]
- LibRaw gives byte 10 of 0x0051 as a u16 in the MakerNote's byte order,
  rawler as a little-endian u16. [S][7][9] The two agree on a
  little-endian MakerNote, as on the three Z 9 files. [I]
- 0x0014 holds Coolpix color data (2560-byte or 1280-byte variants, or an
  "NRW" block on P-series cameras) and is irrelevant to NEF DSLRs. [S][1]
- Capture NX data (0x0E01) is a sequence of records that can carry a white
  balance adjustment, a custom white balance and a flip. Only full
  metadata has a use for it. [S][7]
- The curves in 0x0096 and 0x008C are decoder input; see
  [raw-data.md](raw-data.md).

## 3. NEFCompression and bit depth

NEFCompression (0x0093; on most Z bodies it is in 0x0051, since 0x0093
is absent from every checked Z body, the Z 6 included, but the Z fc and
Z 30, which write 0x0093 and no 0x0051 (files 8559, 5812) **[F]**; the
sources put it at byte 10 of 0x0051 on the Z 9 and later) takes these values. [S][1][3][9]

| Value | Meaning |
|---|---|
| 1 | Lossy, type 1 |
| 2 | Uncompressed |
| 3 | Lossless |
| 4 | Lossy, type 2 |
| 5 | Striped packed 12 bits |
| 6 | Uncompressed, reduced to 12 bits |
| 7 | Unpacked 12 bits |
| 8 | Small |
| 9 | Packed 12 bits |
| 10 | Packed 14 bits |
| 13 | High Efficiency (HE) |
| 14 | High Efficiency* (HE*) |

Which bodies write values 5 and 6 is not established. [I] Decoding each of
these is covered in [raw-data.md](raw-data.md). Three things matter for
metadata code:

- The value is a hint, not an authority. The TIFF Compression tag of the raw
  IFD (34713 for Nikon's compressed data, 1 for uncompressed) can disagree
  with it, and some firmware labels uncompressed data as compressed.
  Uncompressed data shows in its size: its strip byte count matches the
  size that the dimensions and bit depth give, row padding included
  ([raw-data.md](raw-data.md)). [S][8][9][1]
- Values 13 and 14 (HE, HE*) are not decodable with the open implementations
  surveyed; report a typed "unsupported compression" error and never attempt
  the lossless decoder on them. [S][9]
- LibRaw counts a file with value 7 or 9 as one from a fixed-lens
  (Coolpix) body. [S][7]

### Bit depth sources

In order of authority: [S]

1. BitsPerSample (0x0102) of the raw IFD, the depth rawspeed and rawler
   give; the values are 12 or 14, with 16 on some uncompressed files. The
   sNEF variant stores 3 bytes per pixel of 12-bit YCbCr 4:2:2 (see
   [raw-data.md](raw-data.md)), so this tag does not describe a CFA depth
   there. [8][9]
2. The curve header in 0x0096: a version header of 0x44 0x40 marks the
   lossy "2nd version" curve (D780, Z 6, Z 7 and others). rawspeed and
   rawler give its curve a domain two bits narrower than BitsPerSample:
   12 bits in a file labeled 14, 10 in one labeled 12. [8][9] dcraw and
   LibRaw read it otherwise ([raw-data.md](raw-data.md)). A 0x46 header
   is lossless. [8][9]
3. NEFBitDepth (0x0E22): 12 0 0 0, 14 0 0 0, 8 8 8 0 or 16 16 16 0.
   [7][1]
4. The NEFCompression value (5, 6, 7 and 9 are 12-bit variants, 10 is packed
   14-bit). [1]

## 4. Encrypted blocks

Three MakerNote blocks are encrypted. The four-byte ASCII version at the
start of each is never encrypted, and the version selects both whether
decryption applies and where it starts. [S][1][3][5][7]

| Tag | Encrypted when | Decrypt from byte |
|---|---|---|
| 0x0097 ColorBalance | version 02xx (0100, 0102, 0103 are plain) | 4 for versions 0205, 0219 and 0221 to 0224; 284 for the other 02xx versions, including 0220 (ExifTool's ColorBalanceUnknown1, whose white balance it does not decode) |
| 0x0098 LensData | version 0201 and later (0201 to 0204, 04xx, 08xx) | 4 |
| 0x0091 ShotInfo | versions 02xx and 08xx | 4 |

0x00A8 FlashInfo and 0x00B7 AFInfo2 are not encrypted. [S][5] Encrypted
ColorBalance first appeared on the D2X, D2Hs and D50 in 2005. [S][18] The
0x0097 variants of the Nikon 1 (04xx) and Coolpix (05xx, 06xx) are
encrypted but not decoded by the surveyed projects. [S][3][7]

Z-series ColorBalance (version 0800) is encrypted and no surveyed project
maps its white balance offsets. [S][3][7] This does not matter in practice;
see [section 6](#6-white-balance).

## 5. The decryption algorithm

The cipher is a small stream cipher over the block's bytes. It is symmetric
(the same routine encrypts and decrypts) and three independent
implementations describe it the same way. [S][3][6][7]

Inputs, all from the same file's MakerNote:

- the serial number (tag 0x001D) as a decimal integer;
- the shutter count (tag 0x00A7) as four bytes;
- two fixed 256-byte substitution tables, T0 and T1, listed below.

The keystream:

1. The serial number gives one byte, s: the low eight bits of the
   integer. Non-numeric serials are in
   [section 5.1](#51-non-numeric-serial-numbers).
2. The shutter count gives one byte, k: its four bytes XOR-ed together.
3. Three bytes make the state: a = T0[s], which stays constant; x,
   starting at T1[k]; and a counter c, starting at 0x60.
4. For the i-th byte from the start of the encrypted range (i = 1, 2, …),
   x_i = (x_(i−1) + a · c_(i−1)) mod 256 and c_i = (c_(i−1) + 1) mod 256.
   The plain byte is the encrypted byte XOR x_i. [S][3][10]
5. After n bytes, the state is x_n = (x_0 + a · (n · c_0 + n · (n − 1) / 2))
   mod 256 and c_n = (c_0 + n) mod 256, with x_0 = T1[k] and c_0 = 0x60.
   The byte at offset n of the range is then decrypted with x_(n+1), so a
   block of which only a later part is needed can start there. [S][3]

Where the tables live (cite only; do not copy them): ExifTool's
`Nikon.pm` (the array named `@xlat`) [3], exiv2's `nikonmn_int.cpp` and
`makernote_int.cpp` [6], LibRaw's `nikon.cpp` [7], and dcraw's `dcraw.c`
[10]. All of these are under licenses incompatible with that of raw-files, or
ambiguous in dcraw's case; see [Open questions](#11-open-questions).

Consequences for the parser:

- The shutter count (0x00A7) sits after the encrypted blocks in IFD order.
  Parse the whole MakerNote IFD first, then decrypt. [S][7]
- A wrong key produces garbage silently, never an error. Check decrypted
  values for plausibility (for example white balance multipliers in a sane
  range) and treat failure as "value absent". [I]
- If the serial or shutter count is missing, skip decryption and carry on.
  [I]
- rawspeed gives no white balance from a 0x0097 block that is too short.
  It needs at least 80 bytes at version 0100, 26 at 0103, 564 at 0204
  and 284 at 0205. [S][8]
- dcraw decrypts a 324-byte part of ColorBalance, at versions 0200 to
  0216 only. [S][10] LibRaw also decrypts LensData above version 0200, and
  ShotInfo from version 0208. [S][7]

### 5.1 Non-numeric serial numbers

Most serial numbers are digits only. On some bodies (the D50 family and a
few others) tag 0x001D is not numeric, and the surveyed implementations
disagree on what to do. [S]

| Project | Behavior |
|---|---|
| ExifTool and exiv2 | key byte 0x22 (34) if the model is a D50-family body, otherwise 0x60 (96) [3][6] |
| LibRaw | key byte 34 if the model is "D50" (or ends in D50 after a non-alphanumeric character), otherwise 96 [7] |
| dcraw | key byte: the low byte of the serial taken as a number, each letter counting as the digit its character code gives modulo 10 [10] |

ExifTool and exiv2 use the same constants as LibRaw, but dcraw can derive a
different key for a serial that contains letters. All four agree for
all-digit serials. Which behavior is correct for a real body with a
lettered serial is not settled. [S][I] The D200 and D40X are said to
need 0x60. [S][3]

## 6. White balance

Sources, in the order raw-files should try them: [S][7][8][9][16]

1. **MakerNote 0x000C** (WB_RBLevels), unencrypted: four rationals. The
   reading "R, B, then two greens" for the last two values is LibRaw's.
   ExifTool calls them unidentified, and rawler gives green a multiplier
   of 1. On a file with both tags, rawspeed and rawler give 0x000C's white
   balance; they give 0x0097's only on a file without 0x000C. Which of the
   two LibRaw gives on a file with both is not established. If a green
   value is zero or missing, treat green as 1.0 and normalize to it. The
   tag is present on most NEFs, including the Z bodies. [S][3][7][8][9]
2. **MakerNote 0x0097** (ColorBalance), used when 0x000C is absent, with a
   per-version layout. Decryption is required from version 0200 on.
3. A Coolpix-only 0x0014 block as a last resort, not relevant for DSLRs.

The 0x0097 versions and where the four white-balance values sit:

| Version | Bodies | Encrypted | Where the values are | Channel order |
|---|---|---|---|---|
| 0100 | D100 | no | four u16 at byte 72 | R, B, G, G |
| 0102 | D2H | no | four u16 at byte 10 | R, G, G, B |
| 0103 | D70, D70s | no | four u16 at byte 20 | R, G, B, G |
| 0204, 0206 | D2X and similar | yes, from 284 | decrypted data, big-endian u16 at offset 6 | R, G, B |
| 0205 | D50 | yes, from 4 | decrypted data, offset 14 | R, G, G, B |
| 0209, 0212, 0214 | | yes | start 10 | G-first variants |
| 0211 | | yes | start 16 | G-first variants |
| 0215 to 0217 | | yes | start 4 | G-first variants |
| 0219, 0221 to 0224 | | yes, from 4 | ExifTool puts the values at 0x7C after decryption | |
| 0220 | | yes, from 284 | not decoded by ExifTool (ColorBalanceUnknown1) | |
| 08xx | Z bodies | yes | not mapped by any surveyed project | |

Sources: [3][7][8][9]. ExifTool's offsets count from the start of the
decrypted part, and LibRaw's from byte 284 of the block, where its
324-byte part starts. So offsets in different rows are not directly
comparable. [S] ExifTool names the channel orders RBGG
(0100), RGGB (0102, 0205 and others), RGBG (0103), and GRBG for the
0209/0211/0212/0214 to 0217 group. Normalize to a fixed order and divide by
the first green. [S][3]

### The Z-series answer

Z bodies do not need the encrypted 0800 ColorBalance. A Z 6 forum report
describes a normal 0x000C with usable values [S][16]. In the decoders,
rawspeed, rawler and LibRaw all give a Z body's white balance from
0x000C. None of them decodes the 0800 ColorBalance; LibRaw decodes
versions 0200 to 0217 only [S][7][8][9]. No report of an absent or wrong
0x000C on a Z body was found, but this is an absence of evidence rather
than a proof. [I]

The metadata raw-files reads from the plain tags gives the white balance
setting ([section 8.7](#87-camera-settings)), not these multipliers;
full metadata reads these multipliers, and raw decoding uses them.

## 7. LensData and ShotInfo

Neither is needed for previews, for the metadata raw-files reads from
the plain tags or for decoding: full metadata reads what they hold once
a decision on the tables allows it. They are documented here so that
nobody decrypts them by accident.

**LensData (0x0098).** A version-prefixed block (0100 is 9 bytes, 0101 and
0201 to 0203 are 15, 0204 is 16, 0400 to 0403 are 459 to 879, 0800 and 0801
are 58, 0802 is 108). It is encrypted from version 0201. Decrypted, older
versions hold a lens ID, F-stops, min and max focal length and min and max
aperture; focal lengths are 5 * 2^(b/24) mm and apertures 2^(b/24) for a
stored byte b. Z bodies use little-endian u16 fields instead. [S][1][7]

**ShotInfo (0x0091).** Model-specific tables selected by version (and
sometimes by block size). It holds roll, pitch and yaw as fixed-point
numbers, at offsets that depend on the version. This is the only
orientation-like data in the MakerNote, and it is angle data, not the
Exif Orientation value. ExifTool leaves a ShotInfo above 64 KB, seen on
the Z 6III, out of an Exif segment it writes. That is a size limit, not
a sign of a corrupt block: the block can be that large, so cap
allocations. [S][1][3][7]

## 8. Viewer metadata

Everything a viewer needs next to the preview comes from standard IFD0, its
SubIFDs, the Exif IFD and, for the location, the GPS IFD
([section 8.9](#89-location)), except the crop rectangle, which comes from the
MakerNote, and the serial number, ISO, lens range and offset from UTC
that older bodies record only there (sections
[8.2](#82-capture-date-and-time-zone) and
[8.5](#85-serial-number-iso-and-lens)), and the focus point, which only
the MakerNote records ([section 8.6](#86-focus-point)), and the camera
settings that Exif does not record ([section 8.7](#87-camera-settings)).
**[F]** IFD0's own ImageWidth and
ImageLength describe the thumbnail
(160x120 on DSLRs), not the sensor; the stored sensor size is in the raw
SubIFD reached through tag 0x014A.
Decryption is never needed for these fields. [S][1][2] On every Nikon Z
sample, the Exif and MakerNote values raw-files reads lie in the first
105 KB of the file. **[F]**

The picture style and the processing settings are in
[section 8.11](#811-picture-style-and-processing). Exif records six of
their values, and the MakerNote the rest.

### 8.1 Orientation

IFD0 tag 0x0112, Orientation, a u16 with values 1 to 8, each defined by
where the image's first row and first column lie. [P][23] (4.6.5.1.6,
p. 33) As turns: 1 normal, 2 mirror horizontal, 3 rotate 180, 4 mirror
vertical, 5 transpose, 6 rotate 90 clockwise, 7 transverse, 8 rotate 270
clockwise. There is no plain orientation tag in the MakerNote. Treat 0
and values above 8 as 1. [S][2]

The preview JPEGs carry no Exif of their own. [S] Preview SubIFDs may carry
an Orientation tag that disagrees with IFD0 (seen: IFD0 = 6, SubIFD = 1);
IFD0 0x0112 wins. [S] See
[container-and-previews.md](container-and-previews.md). Ignore ShotInfo angles
([section 7](#7-lensdata-and-shotinfo)) and the Capture NX flip in 0x0E01.

### 8.2 Capture date and time zone

| Item | Source | Notes |
|---|---|---|
| Local capture time | Exif IFD 0x9003 DateTimeOriginal | `YYYY:MM:DD HH:MM:SS`, 20 bytes with NUL |
| Sub-seconds | Exif IFD 0x9291 SubSecTimeOriginal | decimal fraction digits |
| Offset from UTC | Exif IFD 0x9011 OffsetTimeOriginal, form `+HH:MM` | optional |
| Fallback zone | MakerNote 0x0024 WorldTime | the time zone and daylight saving setting of the camera's clock; agrees with OffsetTimeOriginal wherever both exist **[F]** |
| Digitized time | Exif IFD 0x9004 DateTimeDigitized, with 0x9292 SubSecTimeDigitized and 0x9012 OffsetTimeDigitized | read as the digitized time, by the capture time's rules, WorldTime included; equal to the capture time on every public file that records both **[F]** |

The six Exif tags and their forms: [P][23] (4.6.6.6.1 and 4.6.6.6.2,
pp. 52 to 53; 4.6.6.6.4 and 4.6.6.6.5, pp. 53 to 54; 4.6.6.6.7 and
4.6.6.6.8, p. 54)

Also present: IFD0 0x0132 DateTime, the last modification time [P][23]
(4.6.5.4.5, p. 43). The census of
[tag-names.md](../tag-names.md#3-the-ifd-chain-and-the-subifds) finds
it and DateTimeDigitized in NEF files. **[F]** Where the digitized time
differs from the capture time, on JPEGs a Nikon camera wrote later, see
the [JPEG notes, section 4](../jpeg/container-and-metadata.md#4-exif-the-app1-segment).

An unset camera clock yields a blank or space-filled string (spaces where the digits would be); return "none" for the date rather than an error. [I]

**OffsetTimeOriginal writers.** The D850, D780, Z 6, Z f, Z6III and Z 9
write all three OffsetTime tags (files 1835, 3827, 3587, 6887, 7808 to 7810
and 5146 to 5148); no older checked body writes any, the D810 (file 2036)
included. **[F]** A Z 9 JPEG listing showed the same. [S][15] Treat 0x9011
as optional. SubSecTimeOriginal is in every checked file except the
Coolpix E5700's and P7800's (files 4456 and 1425). **[F]**

**WorldTime layout (0x0024).** The data is an array of bytes with this
layout, per ExifTool's definition: [S][3][1]

| Offset | Size | Field |
|---|---|---|
| 0 | i16 | time zone in minutes from UTC (60 means UTC+01:00) |
| 2 | u8 | daylight savings: 0 no, 1 yes |
| 3 | u8 | date display format: 0 Y/M/D, 1 M/D/Y, 2 D/M/Y |

The i16 follows the MakerNote's byte order [I]; Capture NX can flip it, and
a byte-swapped value then reads as an absurd offset (a bug report shows +256
hours for the swapped form of 60 minutes). [S][13] Use it only when
OffsetTimeOriginal is absent, and reject implausible values (beyond +-14
hours). [I]

**WorldTime is the offset of the camera's clock.** Sources describe it as
the "home" zone, which need not be the zone at capture. The samples show
otherwise: in every file that has both, the zone plus 60 minutes when the
daylight saving byte is 1 equals OffsetTimeOriginal, with daylight saving
off (files 1835, 3827, 3587, 5146 to 5148) and on (files 6887, 7808 to
7810, at -04:00 from a zone of -300). **[F]** WorldTime is present on
every checked DSLR and mirrorless body from the D300 on and on the Coolpix
A, so also on the D300, D700, D7000, D750, D810 and Nikon 1 J1, which write
no OffsetTimeOriginal; it is absent on the D1H, D100, D70, D200, E5700 and
P7800. **[F]** On the D300 to the D810, which write no Exif offset, the
agreement cannot be checked; taking WorldTime to mean the same there gives
the offset of a decade of bodies. raw-files does so, and reports it as the
offset, not as a separate home zone
([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)).

### 8.3 Make and model

IFD0 0x010F Make and 0x0110 Model. [P][23] (4.6.5.4.2 and 4.6.5.4.3,
p. 42) Typical values: Make "NIKON CORPORATION" and Model
"NIKON D90" on DSLRs, Make "NIKON" and a Model such as "COOLPIX P7000" on
compacts. [S][14]

In the one NEF dump inspected the strings were NUL-terminated with the
count including the NUL and no space padding (Make 18 bytes, Model 10
bytes for a D40). [S][14] The D100 does space-pad Model, with one
trailing space. [S][8] Exif
treats a Make or Model left blank as unknown. [P][23] (4.6.5.4.2 and
4.6.5.4.3, p. 42) ExifTool and LibRaw both report Make and Model without
trailing whitespace.
So: cut at the first NUL, then trim trailing ASCII whitespace. [S][2][7]
Compare model strings after trimming, because a comparison such as the
"D50" suffix of section 5.1 assumes trimmed text. [I]

The samples agree: every checked file has NUL-terminated Make and Model
with no padding, the Z 9's 18 and 10 bytes long like the D40's, except the
D100, whose Model is `NIKON D100 ` with one trailing space (file 2996).
Make is `NIKON` on the Coolpix E5700 and P7800 and `NIKON CORPORATION`
on every other body, the Coolpix A included. **[F]**

### 8.4 Image dimensions

ImageWidth and ImageLength are tags 0x0100 and 0x0101. [P][23]
(4.6.5.1.1 and 4.6.5.1.2, p. 32) In the raw IFD they give the stored
sensor array, which includes masked and border pixels, so it is usually
larger than the image a viewer shows. NEF has no DNG-style DefaultCropOrigin,
DefaultCropSize or ActiveArea; those tags appear in DNG files only. [S][2][9]
Sources for the output size, in order: [S][7][9]

1. **CropArea (0x0045)**: four u16 as left, top, width, height in raw
   pixels. rawler and LibRaw both give it as the image's crop
   rectangle. [S] The D5, D850, D780, Z 6, Z f, Z fc, Z 30, Z6III and
   Z 9 write it; no checked body older than the D5 (2016) does, the D810
   (file 2036) included, and the D5600, of the same year, does not either
   (file 1415). **[F]** It is the visible image: on the Z 9
   `[12, 8, 8256, 5504]` inside the 8280x5520 raw, the size of the largest
   preview (files 5146 to 5148); on the D5 `[8, 8, 5568, 3712]` inside
   5584x3728, the size of its largest preview too (file 2218); on the
   D850's small raw (ImageSizeRAW 3) `[12, 4, 6192, 4128]` inside
   6216x4136 (file 1835), so CropArea counts stored pixels, not the
   4128x2752 the menu names for that size (see ImageSizeRAW below); the
   Z 6's small raw likewise `[10, 6, 4528, 3018]` inside 4548x3030
   (file 3587). Z bodies change the stored raw
   itself for an aspect ratio, and CropArea stays inside it: the Z6III's
   1:1 file stores 4048x4040 with CropArea `[8, 4, 4032, 4032]` (file
   7808). **[F]**
2. A per-body table (as rawspeed's `cameras.xml` or rawler's camera data).
   Not suitable for a small library; raw-files should not carry one for
   its metadata. [I]
3. Exif PixelXDimension and PixelYDimension (0xA002, 0xA003) [P][23]
   (4.6.6.3.1 and 4.6.6.3.2, p. 48): these exist, but Nikon's values
   describe the in-camera JPEG size and may differ from the raw crop.
   [I] No checked NEF has them. **[F]**

**CropHiSpeed (0x001B)** is seven u16 values, in this order [S][3]; the
crop files of [section 8.6](#86-focus-point) bear it out **[F]**:

| Index | Field |
|---|---|
| 0 | crop mode |
| 1 | original (sensor) width |
| 2 | original (sensor) height |
| 3 | cropped width |
| 4 | cropped height |
| 5 | crop X offset in pixels |
| 6 | crop Y offset in pixels |

Mode codes: 0 off, 1 1.3x, 2 DX, 3 5:4, 4 3:2, 6 16:9, 8 2.7x, 9 DX movie
16:9, 10 1.3x movie, 11 FX uncropped, 12 DX uncropped, 13 2.8x movie, 14
1.4x movie, 15 1.5x movie, 17 FX 1:1, 18 DX 1:1. [S][1] LibRaw reports
modes 0, 1, 2 and 4 as 3:2, 3 as 5:4, 6 as 16:9 and 17 as 1:1. [S][7] On
"uncropped" bodies the crop modes change the raw array itself. [S][1]

ImageSizeRAW (0x003E) says whether a small-raw file is large, medium or
small. The D850 stores M and S files at sizes that differ from the menu
sizes (reported as 7104x4728 and 6216x4136 where the menu says 6192x4128 and
4128x2752); converters rescale. [S][20] For the preview, the size comes from the
SOF marker of the chosen JPEG; see
[container-and-previews.md](container-and-previews.md).

The D100 is special: rawler gives its uncompressed files a raw width of
3040 pixels. [S][9]

### 8.5 Serial number, ISO and lens

Nikon added these to the standard Exif tags late; older bodies record
them only in the MakerNote, in plain form. **[F]**

| Item | Exif tag | MakerNote tag | Measured |
|---|---|---|---|
| Body serial number | 0xA431 BodySerialNumber | 0x001D SerialNumber | Exif from the D780 and Z 6 on (files 3827, 3587); MakerNote from the D300 on, blank on the D200 |
| ISO | 0x8827 PhotographicSensitivity, called ISOSpeedRatings up to Exif 2.21 | 0x0002 ISO, `[0, ISO]` | Exif on every checked body except the D1H, D1X, D100, D70, D700 and E5700; the first four have it in the MakerNote |
| Lens model | 0xA434 LensModel | none in plain text | the D780, Z 6 and later, and the Nikon 1 J1 (file 2956) |
| Lens range | 0xA432 LensSpecification | 0x0084 Lens | Exif from the D780 and Z 6 on; MakerNote on every body with interchangeable lenses, equal to Exif where both exist |
| Lens make, serial | 0xA433 LensMake, 0xA435 LensSerialNumber | none | the D780 and Z bodies; LensMake is empty for the D780's F-mount lens and the Z f's third-party lens (files 3827, 6887) |

The Exif tags: [P][23] (4.6.6.7.5, p. 56; 4.6.6.9.3 to 4.6.6.9.7,
pp. 79 to 80)

ISO is two SHORT values in 0x0002, of which the second is the ISO; the
D700 writes four zero bytes of type UNDEFINED and the E5700 `[0, 0]`,
which mean no ISO is recorded. The Z 9's Exif ISO is 0x8827 with
SensitivityType 2 and an equal RecommendedExposureIndex (0x8832). The Z 9
writes exposure values unreduced (ExposureTime `10/3200`, FNumber
`900/100`, FocalLength `180/10`, files 5146 to 5148) and every checked
body writes ExposureBiasValue as a signed rational (`0/6`; `-6/6` on the
D100). **[F]**

### 8.6 Focus point

Where the camera focused, which raw-files reports as its focus points so
that a viewer can judge sharpness at that point. raw-files decided what
it reports in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md);
this section is the record behind it. The field offsets are ExifTool's
[S][1]; every value was read on the 35 CC0 Nikon files of
[test-data.md](test-data.md#23-files-checked-for-the-focus-point), and a
field is tagged **[F]** where a sample holds a value for it.

**Where a Nikon file records focus data.** Every source found, whether
raw-files reads it or not:

| Source | What it holds | Checked files | raw-files |
|---|---|---|---|
| 0x00B7 AFInfo2, position fields | the size of a frame, the center of the focus area and the area's size in that frame | mirrorless bodies, and DSLRs in live view **[F]** | read, at the versions of the table below |
| 0x00B7 AFInfo2, in-focus byte | whether focus was confirmed: FocusResult on 04xx, ContrastDetectAFInFocus on 0100 and 0101 | Z 9, Z f, Z6III, Z50II, Z5II, D7000, D5600, D5, Coolpix A **[F]** | read; on 0100 the flag is byte 28, where ExifTool also lists a 20-byte AFPointsSelected that cannot fit in the 30-byte value |
| 0x00B7 AFInfo2, point fields | the primary point's number, a mask of the points used, and the layout of points (FocusPointSchema) | DSLRs through the viewfinder, the Nikon 1 J1, the Z5II in auto-area mode **[F]** | read on `0100` alone, in a record with no position: the number is placed from a table of the body's points, measured by the project on frames shot with that body ([autofocus-points.md](autofocus-points.md), [ADR017](../../architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)), with the points the mask names ([ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)); the D600's is the only table, and every other body gives none; the other versions' point fields are not read |
| 0x00B7 AFInfo2, AFPointsInFocus | a mask of the points in focus, version 0101 only (byte 48) [S] | the D850, one point **[F]** | not read: a mask of version 0101, which needs a table of a body that writes that version |
| 0x0088 AFInfo | 4 bytes: area mode, point number, mask of the points in focus [S] | the bodies without AFInfo2: D1H, D1X, D70, D100, D200, E5700, P7800 **[F]** | the point not read: a number only; the area mode, its first byte, read as a camera setting (below) |
| 0x0021 FaceDetect | a frame size and up to twelve face rectangles in it [S] | none of the 35 files **[F]** | not read: no sample |
| 0x0007 FocusMode | an ASCII string [S][1]: `MANUAL`, or an AF mode padded with spaces to six characters (`AF-S  `, `AF-C  `, `AF-A  `) **[F]** | `MANUAL` on the D300 and D850, both viewfinder shots with a zero frame (files 928, 1835), and on the Z fc, which records a position under it (file 8559); an AF mode on every other file with the embedded MakerNote, my Z 9 shots included **[F]** | read as the focus mode, a camera setting ([section 8.10](#810-more-capture-settings)); `MANUAL`, after trimming, gives no focus point |
| 0x00B9 AFTune | AF fine-tune, a setting, not a position [S] | 225 public files of 49 bodies and 37,339 of mine ([section 8.10](#810-more-capture-settings)) **[F]** | read as a camera setting |
| 0x0085 ManualFocusDistance, the area modes in the MenuSettings blocks | settings, not a position [S] | not looked for | not read |

**AFInfo2 layouts.** The value is UNDEFINED and starts with a version in
four ASCII digits; it is not encrypted on any version
([section 4](#4-encrypted-blocks)). Its multi-byte fields are in the
MakerNote's byte order: the Coolpix A, big-endian, reads a 4928x3264
frame, and the Z 9, little-endian, 8256x5504; the other order gives
nonsense (16403x49164, 16416x32789) (files 2238, 5146). **[F]** Byte
offsets from the start of the value:

| Version | Length | Detection | Area mode | Point schema | Coordinates flag | Frame W, H | Center X, Y | Area W, H | In focus | Bodies |
|---|---|---|---|---|---|---|---|---|---|---|
| `0100` | 30 | 4 | 5 | 6 | none | 16, 18 | 20, 22 | 24, 26 | 28 | DSLRs from the D3 and D300 (2007) and the D90 (2008) up to the D4S, D750, D810, D7200, D5500, D5600 and Df, and the Coolpix A [S]; samples: D300, D700, D750, D810, D7000, D5600, Coolpix A **[F]** |
| `0101` | 84 | 4 | 5 | 6 | none | 70, 72 | 74, 76 | 78, 80 | 82 | D5, D500, D850, D3400, D3500, D7500 [S]; samples: D850, a viewfinder shot, and D5, in live view **[F]** |
| `0200`, `0201` | 42 (`0200`), 52 (`0201`) [F] | none | 5 | 6 | none | none | none | none | none | Nikon 1 bodies [S]; samples: `0200` on the J1, J2, J3, S1, V1, V2 and AW1, `0201` on the J4, J5, S2 and V3 **[F]** |
| `0300`, `0301` | 60 | 4 | 5 | 6 | 7 | 42, 44 | 46, 48 | 50, 52 | none | `0300`: D780, Z 6, Z 7, Z 50; `0301`: D6, Z 5, Z 6II, Z 7II, Z 30, Z fc [S]; samples: D780, Z 6, both `0300`, and Z 30, Z fc, both `0301` **[F]** |
| `0400`, `0401`, `0402` | 80 | 4 | 5 | none | 7 | 62, 64 | 66, 68 | 70, 72 | 74 | `0400`: Z 8, Z 9; `0401`: Z f, Z6III; `0402`: Z50II, Z5II [S]; samples: Z 9, Z f, Z6III, Z50II, Z5II **[F]** |

ExifTool groups the tables by the first two digits (0100; 0101 and later
01xx; 02xx; 03xx; 04xx). [S][1] The versions by body come from the exiv2
metadata listings of all 291 Nikon files of the raw.pixls.us catalog
(October 2026), which show these seven versions besides the Nikon 1's
and no other; the bodies before the D3 and every Coolpix but the A have
no AFInfo2 there. [S][22] Those listings decode only the 0100 offsets,
and in one byte order whatever the file's, so they settle which version a
body writes and nothing else. raw-files reads the position at the exact
versions 0100, 0101, 0300, 0301, 0400, 0401 and 0402 (ADR012). It
reads the area mode at those and at 0200 and 0201, the Nikon 1
versions, which record no position
([ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md)).
The 0101
offsets are checked on the D5's live-view file, the only one that fills
them (file 2218), and version 0301, at the 03xx offsets, on the Z 30's
and the Z fc's (files 5812, 8559).

**The point fields.** Where each version keeps the layout code
(FocusPointSchema), the primary point's number, the mask of the points
used and, on one version, a mask of the points in focus, read in
ExifTool's published tables [S][1] and checked on the files named. The
offsets count from the start of the value; a mask's length is the one
ExifTool lists for the layout or the body:

| Version | Layout code | Primary point | Mask of points used | Mask of points in focus | Checked on |
|---|---|---|---|---|---|
| `0100` | 6 | 7 | 8, up to 7 bytes; the D600's 39 bits lie in the first five | none | the frames of my D600 session ([autofocus-points.md](autofocus-points.md#21-the-session)) and 1,374 files of my D600 (layout 3, the 39-point), and my 3,568 D3200 files (layout 2, the 11-point, numbers 1 to 11) and 647 D300s, 400 D7100 and 6,277 D7200 files (layout 1, the 51-point) **[F]**; the D810 (layout 1, point 3, one bit), the D750 (point 1, one bit), the D700 (point 1, no bit) and the D300 (layout 0, point 0, manual focus) (files 2036, 895, 2255, 928) **[F]** |
| `0101` | 6 | 68 | 8, up to 20 bytes by the layout | 48, up to 20 bytes | the D850: layout 7 (the 153-point), point 1, no bit, manual focus (file 1835) **[F]**; the D5 in live view: layout 0, point 0 (file 2218) **[F]** |
| `0200` | 6 | 7 | 8, 29 bytes | none | the Nikon 1 J1: layout 4, point 1, one bit (file 2956) **[F]** |
| `0300`, `0301` | 6 | 56 | 10, 7, 11 or 14 bytes by the layout | none | the D780: layout 1, point 34, 30 bits (file 3827) **[F]** |
| `0400` to `0402` | none | none | 10, 51, 38 or 29 bytes by the body | none | the Z5II in auto-area mode: 28 bits (files 7735, 7736) **[F]** |

The layout codes seen, with ExifTool's names for them [S][1]: 0, no
layout, on every live-view and manual-focus file; 1, the 51-point
layout; 2, the 11-point; 3, the 39-point; 4, the Nikon 1's; 7, the
153-point. **[F]** ExifTool's published tables, read as documentation,
list two layouts for the D780, 51 points through the viewfinder and 81
points in live view, each with its own layout code [S][1]. So a file's
layout code tells which grid its number belongs to. [I] The mask's bit
order is bit number − 1, counting
from the least significant bit of the first byte, on the D600's
39-point layout, measured on 45 frames that each set exactly one bit
([autofocus-points.md](autofocus-points.md#23-the-masks-bit-order))
**[F]**; ExifTool documents that order for the 11-point layout [S][1];
the other layouts are unmeasured.

What the fields hold:

- **The center of the area, not its corner.** ExifTool's 0100 and 0101
  tables say so [S][1]; on the Coolpix A it is exactly half the frame,
  (2464, 1632) in 4928x3264 (file 2238), and on the Z f (3023, 2016) in
  6048x4032 (file 6887). **[F]**
- **The frame is the camera's full-size output, not the stored raw.** On
  the Z 9 it is 8256x5504, the size of CropArea and of the largest
  preview, inside the 8280x5520 raw (files 5146 to 5148); on the D7000
  and the Coolpix A 4928x3264, the largest preview, inside 4992x3280
  (files 961, 2238); on the Z f 6048x4032, CropArea's size (file 6887).
  On the Z 6's small raw it is still 6048x4024, the body's full-size
  output, while the stored image is 4548x3030 with CropArea 4528x3018
  (file 3587): the point scales with the picture. On the D5600 it is
  6000x4000, in a 6016x4016 raw that has no CropArea (file 1415).
  **[F]**
- **On a crop mode the frame stays the full output.** The Z6III's 1:1,
  16:9 and DX files record a 6048x4032 frame while they store 4048x4040,
  6064x3416 and 4000x2672 pixels and show CropArea's 4032x4032, 6048x3400
  and 3984x2656 (files 7808, 7809, 7810; 7812 and 7813 repeat 7808 and
  7809); the Z50II's 1:1 and 16:9 files record its 5568x3712 frame with
  CropArea 3712x3712 and 5568x3128 (files 7767, 7770). **[F]** On every
  one of them the point lies within 12 pixels of the frame's center, so
  these files cannot tell how the values relate to the crop.
- **A DX crop records fractions of the cropped picture; an aspect crop
  records positions in the full frame.** Thirteen shots made for this
  check with my Z 9 tell the two apart: two frames in each of FX, DX,
  1:1 and 16:9 with a single point placed in the top-left corner of
  the picture, and five earlier frames of the same modes with the same
  values. One DX and one 1:1 frame are committed, stripped of their
  pixels and personal fields, as fixtures
  ([`tests/fixtures/`](../../../tests/fixtures/README.md)). In the
  8256x5504 frame they record:

  | Mode | CropHiSpeed | CropArea | Center, area recorded | Scaled onto CropArea | Shifted by the centered window |
  |---|---|---|---|---|---|
  | FX | `[11, 8280, 5520, 8280, 5520, 0, 0]` | 8256x5504 | (502, 424), 291x323 | the same | the same |
  | DX | `[2, 8280, 5520, 5408, 3608, 1436, 956]` | 5392x3592 | (636, 572), 420x468 | **(415, 373), 274x305** | (-796, -384): outside the picture |
  | 1:1 | `[17, 8280, 5520, 5520, 5520, 1380, 0]` | 5504x5504 | (1797, 424), 291x323 | (1198, 424), 194x323 | **(421, 424), 291x323** |
  | 16:9 | `[6, 8280, 5520, 8280, 4656, 0, 432]` | 8256x4640 | (502, 715), 291x323 | (502, 603), 291x272 | **(502, 283), 291x323** |

  **[F]** In bold, the reading that puts the point as near the corner as
  in FX (6 to 10 % of the way in) and keeps a single point's area close to
  FX's 291x323. A likely reason, not checked: the DX image fills the
  viewfinder, so its point is recorded in the full frame's units, while
  the 1:1 and 16:9 images are shown as windows of the full frame, so their
  point is recorded where it lies in that frame. [I] The Z50II's 1:1 and
  16:9 files fit the window too: a DX body whose areas in both, 278x276
  and 280x280, keep the shape of a single point (files 7767, 7770).
  **[F]** So raw-files scales a crop whose two sides are both smaller than
  the original (DX) and shifts the cut axis of a crop that keeps one side
  (1:1, 16:9), by half the difference between the recorded frame and
  CropArea, keeping the area, when CropArea's kept side equals the
  frame's. A 1:1 or 16:9 crop on a small raw, whose CropArea counts
  stored pixels, gives no point, and a DX crop on one is scaled onto that
  CropArea (ADR012). A crop whose CropArea is present but cannot be used,
  or cannot be checked against a raw size, gives no point; a crop with no
  CropArea at all, such as the D810's, is read as recorded. The values of
  the CC0 files are in
  [test-data.md](test-data.md#23-files-checked-for-the-focus-point).
- **CropHiSpeed (0x001B) tells a cropped file apart**
  ([section 8.4](#84-image-dimensions)). On the crop files above its
  cropped size, values 3 and 4, differs from its original size, values 1
  and 2: `[17, 6064, 4040, 4048, 4040, 1008, 0]` (1:1),
  `[6, 6064, 4040, 6064, 3416, 0, 312]` (16:9),
  `[2, 6064, 4040, 4000, 2672, 1032, 684]` (DX) on the Z6III, and
  `[18, 5600, 3728, 3728, 3728, 936, 0]` (1:1) and
  `[9, 5600, 3728, 5600, 3144, 0, 292]` (16:9) on the Z50II; the Z5II's
  crop files read like the Z6III's (7735, 7736). On the other files the
  two sizes are equal, with mode 11 (FX uncropped) on the Z 9, Z 6, Z f,
  D780, D850, D750 and D5, 12 (DX uncropped) on the D5600, Z 30 and Z fc
  and 0 (off) on the D200, D300, D700, D7000, J1 and Coolpix A, except on
  the D810, whose mode 11 comes with a cropped width four pixels short:
  `[11, 7384, 4928, 7380, 4928, 0, 0]` (file 2036). **[F]**
- **In stored orientation**, like the sizes and previews raw-files
  reports. The Z 30's shot, rotated (Orientation 8, 270 degrees clockwise
  to view), records the landscape frame 5568x3712, and the position in
  it, center (3776, 1741) and area 552x552 (file 5812). **[F]**
- **A DSLR shot through the viewfinder records no position.** The frame,
  center and area are zero on the D300, D700, D750, D810, D850 and D780,
  and only the point fields are filled in: primary point 3 on the D810,
  34 on the D780, with 30 bits of its mask set (files 928, 2255, 895,
  2036, 1835, 3827). ExifTool's 0100 and 0101 tables call the position
  fields valid for contrast-detect AF only [S][1]; the live-view files
  above, with a detection method of 1 (contrast) on the D7000, D5600, D5
  and Coolpix A and 2 (hybrid) on the Z bodies, are the ones that fill
  them; my Z 9 writes 1 in pinpoint AF. **[F]**
  Mapping a point number to pixels needs a table of the body's
  points, which no file carries (ADR012). raw-files measures such
  tables itself, one body at a time, on frames shot with the body and
  read in the maker's software. The D600's is measured: the record,
  with the layout, the bit order, the box of 164x164 and the 39 centers
  in the 6016x4016 output frame, is in
  [autofocus-points.md](autofocus-points.md#2-nikon-d600). On a DX
  frame the point placed from the table shifts by CropHiSpeed's
  origin, (1040, 696) on the D600, within 2 pixels of where NX Studio
  draws it. **[F]** How a named point is placed from the table and
  under which conditions, its frame on a crop, the crop's window,
  3936x2624 on a D600 DX frame, whose size the table keeps and checks
  against CropHiSpeed's origin, and how the points used are reported
  are decided in
  [ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md).
  raw-files places the D600's point so, from version `0100`'s layout
  code, primary point and mask: in the 6016x4016 frame, or in that
  window on the DX crop, the one crop the record confirms.
- **A DSLR in live view on a crop records the cropped frame.** One of
  my D7200 files, shot in live view in the 1.3x movie crop,
  CropHiSpeed `[10, 6016, 4016, 4840, 2712, 576, 248]`, records an
  AFInfo2 frame of 4800x2696, 40 and 16 pixels less than the cropped
  size CropHiSpeed records, while its 155 other live-view files,
  uncropped, record 6000x4000 in a 6016x4016 raw; 168 of its 6,277
  files are 1.3x stills, `[1, 6016, 4016, 4840, 3208, 576, 0]`, none
  of them in live view. **[F]** The frame is the cropped picture, so
  the point is placed as recorded. Whether a body with a CropArea does
  the same is open ([section 11](#11-open-questions)).
- **The coordinates flag (byte 7 of 03xx and 04xx)** is 1 on every
  sample with a position and 0 on the D780 viewfinder shot and the Z5II
  auto-area files. **[F]** ExifTool notes that on 04xx either the mask
  or the position is set, not both [S][1]; the Z5II files (area mode 197,
  auto-area) have a 28-bit mask, a zero frame and a FocusResult of 1
  (files 7735, 7736). **[F]** raw-files judges by the values, not by the
  flag.
- **The in-focus byte** is 1 on the Z 9, Z f, Z50II, D7000, D5600, D5
  and Coolpix A and 0 on all five Z6III files, which have a valid point
  beside it: the camera recorded a miss. **[F]** Version 03xx has no such
  byte.
- **Area modes** in the samples (byte 5): 193 (single point) on the Z 9
  and Z 6, 196 (wide area L) on the Z f, 192 (pinpoint) on the Z6III,
  207 (3D-tracking) on the Z50II, 197 (auto-area) on the Z5II, 199 on
  the Z 30, which records a position under it, and 193 on the Z fc too,
  in manual focus; 1 on the D7000 and Coolpix A and 2 on the D5600 and
  D5, in live view. [S][1] **[F]**
  The area size follows the mode: 291x323 on the Z 9, 1859x1452 for the
  Z f's wide area, 984x816 on the D5600. **[F]**
  ExifTool's published documentation gives byte 5 two lists, told apart
  by the detection method at byte 4: under phase detection, 0 single
  area, 1 to 18 the other viewfinder modes, dynamic-area, group-area,
  wide single-area, auto-area and 3D-tracking among them (8 auto-area,
  9 3D-tracking), 128 to 131, and the mirrorless codes 192 to 197 and
  199; under contrast detection, 0 plain contrast-detect, 1 to 4 its
  area modes, 128 to 131, and the mirrorless codes 192 to 208,
  207 3D-tracking among them, which the phase-detect list lacks;
  version 04xx has the mirrorless list alone [S][1]. My D600 frames
  agree: 0, 5, 8 and 9 came with detection 0 through the viewfinder, and
  the live-view frames wrote 4 with detection 1
  ([autofocus-points.md](autofocus-points.md#22-what-the-body-writes))
  **[F]**. Frames shot in a known mode confirm these codes, under the
  detection method the file records: under phase detection, 0 single
  point, 5 dynamic area and 8 auto-area on my D600 session, and 9
  3D-tracking on eight D600 frames I shot in that mode outside it
  **[F]**; 192 pinpoint, 193 single point and 197 auto-area, on which
  the two lists agree (192 on seven Z 9 frames I shot in pinpoint AF on 2026-11-19, four in
  FX and three in DX, each NEF and the JPEG of four of them writing it
  under detection 1, contrast detection, one of them committed as
  fixtures; 193 on the two Z 9 frames shot with a single point and
  committed as fixtures
  ([`tests/fixtures/`](../../../tests/fixtures/README.md)), 197 on my
  Z 9 files, 99 of which carry it and were shot in auto-area) **[F]**;
  and 207 3D-tracking under contrast or hybrid detection, the lists
  that carry it (on my Z 9 files, 8,877 of which carry it and were
  shot in 3D-tracking, under hybrid detection), counts only **[F]**.
  raw-files names the mode by the table that ends this section, which
  adds the codes the documentation names and the files agree with
  ([ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)).
  ExifTool's published documentation, read as documentation, lists 192
  as pinpoint in both lists [S][1], and my frames confirm it. The
  Z6III's five files write it under detection 2, in a mode not
  recorded, and take the name as every file that writes the code under
  contrast or hybrid detection does.
  The mode is a camera setting, given whether or not the file gives a
  point.
- **Every AFInfo2 and CropHiSpeed value lies inside the MakerNote's
  declared length** on every checked file. **[F]**
- **In a camera JPEG**, the D5300's AFInfo2 is version 0100 with a zero
  frame and area, a shot through the viewfinder, and the D70 JPEG writes
  only AFInfo (0x0088): neither gives a point. **[F]** No public Nikon
  JPEG in the set records a position. The JPEGs of my D600 session
  carry the same AFInfo2 and CropHiSpeed as the NEF of the same frame
  ([autofocus-points.md](autofocus-points.md#22-what-the-body-writes)),
  and the reader gives them the same points as the NEF files: 54 with
  a point, 50 of them placed from the table, the 3 DX JPEGs' in the
  crop's window
  ([autofocus-points.md](autofocus-points.md#28-what-the-reader-gives-on-my-own-files)).
  **[F]** So a D600 JPEG gives the point its NEF gives, placed from the
  table; on a DX frame only when the JPEG's size is the crop's window,
  3936x2624, its full image size, since a smaller DX JPEG is unmeasured
  (ADR018). A JPEG keeps a point only when its
  frame has the JPEG's aspect ratio, so that an export cropped to another
  shape, which may keep the camera's MakerNote, gives none
  ([ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)),
  except an export cut from a crop-mode JPEG, whose point is placed on
  the export's own pixels (below), and an export of a D600 JPEG cut or
  scaled to exactly 4512x3008, which is read as a Medium JPEG (next
  sentences), its point by far misplaced on a cut. My D600's Medium JPEG, 4512x3008,
  exactly 3:2, would fail that rule against the table's 6016x4016
  frame, but its picture is not the whole frame: it shows a window
  centered in it, a few pixels smaller, which the table keeps for that
  size, and its points are placed in that window, which has its shape
  ([autofocus-points.md](autofocus-points.md#29-the-jpeg-sizes),
  [ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)).
  The Small JPEG, 3008x2008, is the whole frame at half its size, and
  takes the table's frame. **[F]**
- **In a camera JPEG shot in a crop mode**, my Z 9 records the raw
  file's AFInfo2 and CropHiSpeed, with the full 8256x5504 frame, at
  every image size, and no CropArea; its full-size JPEG is the raw
  file's CropArea (5392x3592, 5504x5504, 8256x4640), and its DX JPEGs at
  the smaller sizes are exactly 3:2. **[F]** raw-files places such a
  point on the JPEG's picture, as on CropArea in a raw file: a DX crop's
  scaled onto the JPEG's frame, a 1:1 or 16:9 crop's moved into the
  centered window of the frame that has the JPEG's shape
  ([ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)).
  In a JPEG, a CropHiSpeed whose sizes differ on a file that is no crop,
  as the D810's cropped width four pixels short does, reads as a crop of
  the width: a JPEG of the frame's shape then gives its point unchanged,
  since the window is the frame; an export cut narrower keeps a point
  placed on its pixels, which may be misplaced, as ADR015 says of a
  crop-mode JPEG cut again, and one cut wider makes a window longer than
  the frame and gives none. No public D810 JPEG is in the set.

**What the samples cover.** Each kind of shot, the files that show it,
and what raw-files gives; a row without a file is an open question
([section 11](#11-open-questions)):

| Shot | What the file records | CC0 files | raw-files gives |
|---|---|---|---|
| Mirrorless, single point, pinpoint, wide area or 3D-tracking | a position | Z 6 3587, Z 9 5146 to 5148, Z f 6887, Z6III 7808 to 7810, 7812, 7813, Z50II 7767, 7770; my Z 9 shots, one pinpoint frame committed stripped | the point |
| Mirrorless on a DX crop | a position, scaled into the full frame | Z6III 7810; my Z 9 shots, off-center, one DX frame committed stripped | the point, scaled onto the crop |
| Mirrorless on an aspect crop (1:1, 16:9) | a position in the full frame | Z6III 7808, 7809, 7812, 7813, Z50II 7767, 7770; my Z 9 shots, off-center, one 1:1 frame committed stripped | the point, moved into the window |
| A DX crop at 1:1 or 16:9 on an FX body | unknown | none | the point, scaled; unchecked |
| Mirrorless small raw | a position in the full frame | Z 6 3587 | the point, in the full frame |
| Mirrorless, auto-area with only a mask | a mask | Z5II 7735, 7736 | none: needs a table of points |
| DSLR in live view | a position | D7000 961, D5600 1415, D5 2218, Coolpix A 2238 | the point |
| DSLR in live view on a crop | the cropped frame | none public; one of my D7200 files, in the 1.3x movie crop | the point as recorded, in the cropped frame; unchecked on a body with a CropArea |
| A `0101` body in live view | a position, at ExifTool's offsets | D5 2218 | the point; checked |
| A `0301` body | a position, at the 03xx offsets | Z 30 5812, Z fc 8559 | the point, none on 8559, a manual-focus shot; checked |
| DSLR through the viewfinder, a body with a table | a point number and a mask | none among the public samples; my D600 frames, all committed stripped, and my D600 files ([autofocus-points.md](autofocus-points.md#2-nikon-d600)) | the point placed from the table, with the points the mask names, as decided in [ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md), in the crop's window on a DX frame ([ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)); checked on those frames |
| DSLR through the viewfinder, any other body | a point number and a mask | D300 928, D700 2255, D750 895, D810 2036, D850 1835, D780 3827 | none: no table for the body; 928 and 1835 are manual-focus shots, which get none in any case |
| Nikon 1 | a point number and a mask | J1 2956 | none: needs a table of points |
| Bodies without AFInfo2 | AFInfo: a point number | D1H 5364, D70 2060, D100 2996, D200 8079, E5700 4456, P7800 1425 | none: needs a table of points |
| Manual focus | FocusMode `MANUAL`; a zero frame through the viewfinder | D300 928, D850 1835 | none |
| Manual focus on a Z body | FocusMode `MANUAL`; a position in AFInfo2 | Z fc 8559 | none; checked |
| A 1:1 or 16:9 crop on a small raw | unknown: CropArea counts stored pixels | none | none, since CropArea's kept side differs from the frame's |
| A DX crop on a small raw | unknown | none | the point, scaled onto CropArea in stored pixels; unchecked |
| A rotated (portrait) shot | a position in the landscape frame | Z 30 5812 | the point, in stored orientation; checked |
| Face detection | FaceDetect | none | not read |
| Camera JPEG through the viewfinder | a zero frame (`0100`) with a point number and a mask, as in the NEF of the same frame | the D5300 JPEG from Wikimedia Commons; my D600 JPEGs | none for the D5300, which has no table; the D600's point placed from the table, as in its NEF, on a DX frame only at the size of the crop's window, its full image size, and on a Medium JPEG, 4512x3008, in the window of the frame its picture shows ([autofocus-points.md](autofocus-points.md#29-the-jpeg-sizes)); checked on the committed JPEG of frame 4 and on a committed Medium and Small JPEG |
| Camera JPEG with a position | a position | none public | the point, when its frame has the JPEG's aspect ratio; unchecked on a public file |
| Camera JPEG in a crop mode (DX, 1:1, 16:9), at any image size | a position in the full frame, no CropArea | none public; my Z 9 shots | the point, placed on the JPEG's picture; unchecked on a public file |
| Exported JPEG that keeps the camera's MakerNote | the camera's frame and position | none public | the point only when the frame has the export's aspect ratio |

**The AF-area mode, a camera setting.** raw-files gives the AF-area
mode as a camera setting, whether or not the file gives a focus point,
and under manual focus too
([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)).
It comes from AFInfo2's bytes 4 and 5, at every version of the layouts
table above. On the bodies without AFInfo2, it comes from the first
byte of AFInfo (0x0088). The codes were counted on 2026-11-20
with a counting program over raw-files' own view of the entries. The
files are the 289 public NEF and NRW files and 48,395 of my own, as
counts only. Mine are 31,991 NEF files and 16,404 JPEGs with a Nikon
MakerNote, of seven bodies. **[F]**

Each code carries one of three marks, by the rule of
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md),
with its count of files:

- **confirmed**: a frame shot in a known setting shows it;
- **observed**: ExifTool's published tag documentation names it
  [S][1], and measured files hold it; "in my files only" where no
  public file does;
- **documented only**: the documentation names it, and no measured
  file holds it.

The documentation gives its codes in lists, each the codes of one tag
at one place: here a record's version and the detection method. Once a
measured file holds a code of a list where the documentation places
it, and no other value of that file contradicts it, every code of the
list is named, those no file holds among them. A list no file holds
any part of stays unread. A code the documentation does not name stays
unnamed. So does a code it names two ways the files cannot tell apart,
unless both fold into one variant, as code 199 does below. An unnamed
code gives `None`.

The documentation gives versions 0100, 0101 and 03xx two lists for
byte 5. Detection 0, phase detection, takes the first; detection 1,
contrast detection, the second. It does not say which list detection
2, hybrid, takes. So under hybrid detection on those versions:

- a code both lists give, with one meaning or with two that fold into
  one variant, is named: 128 to 130, 192 to 197 and 199;
- a mirrorless code that only the second list gives is named by it:
  198 and 200 to 208. Of these, version 0400's list holds 204 to 208,
  which my Z 9's frames write under hybrid detection; 198 and 200 to
  203 are named as the rest of the second list's mirrorless codes, and
  no file shows them;
- the DSLR codes 0 to 18 stay unnamed: 0 to 4, which the two lists
  name differently (0 is a single area in one and plain contrast
  detection in the other), and 5 to 18, viewfinder modes of the first
  list alone, which no file shows under hybrid detection.

Versions 02xx and 04xx have one list each, whatever byte 4 holds.
[S][1] Every list is read: files hold part of each (the table below).

Each variant folds in Nikon's finer kinds: the dynamic-area sizes are
`Dynamic`, and the wide-area and group-area modes are `Zone`. Two
names need a reading of their own [I]. In "Dynamic Area (closest
subject)" the camera picks the point with the closest subject, so it
is `Auto`; in "Group Dynamic" the photographer places a group of points
and the camera chooses within it, so it is `Zone`. "Wide" in "Single
Area (wide)" and "Dynamic Area (wide)" names the size of the focus
frame, so they are `Single` and `Dynamic`. Face priority picks faces
for the photographer, a mode none of the neutral words fits, and
plain contrast detection (0) names no area: both give `None`.

**The codes files hold.** The counts were taken on 2026-11-20. The
detection column gives the methods the files record the code under;
"none" means the version records no method:

| Versions | Detection | Code | The documentation's name [S][1] | `FocusAreaMode` | Mark | Files, public / mine |
|---|---|---|---|---|---|---|
| `0100`, `0101`, `0301` | 0 | 0 | Single Area | `Single` | confirmed: my D600's frames 1 to 41 and 47 to 50 | 90 / 13,980 |
| `0100`, `0101` | 0 | 1 | Dynamic Area | `Dynamic` | observed | 2 / 62 |
| `0100`, `0101` | 0 | 4 | Dynamic Area (9 points) | `Dynamic` | observed | 6 / 413 |
| `0100` | 0 | 5 | Dynamic Area (21 points) | `Dynamic` | confirmed: my D600's frames 51 and 52 | 4 / 3,999 |
| `0100` | 0 | 6 | Dynamic Area (51 points) | `Dynamic` | observed | 5 / 0 |
| `0100` | 0 | 7 | Dynamic Area (51 points, 3D-tracking) | `Tracking` | observed | 6 / 0 |
| `0100`, `0101`, `0300` | 0 | 8 | Auto-area | `Auto` | confirmed: my D600's frames 42 to 44 | 24 / 11,579 |
| `0100` | 0 | 9 | Dynamic Area (3D-tracking) | `Tracking` | confirmed: my D600's frames 7852 to 7859 | 4 / 720 |
| `0100` | 1 | 0 | Contrast-detect | none: it names no area | observed | 1 / 1 |
| `0100` | 1 | 1 | Contrast-detect (normal area) | `Single` | observed | 5 / 11 |
| `0100`, `0101` | 1 | 2 | Contrast-detect (wide area) | `Zone` | observed | 3 / 402 |
| `0100` | 1 | 3 | Contrast-detect (face priority) | none: no neutral word fits | observed, in my files only | 0 / 19 |
| `0100` | 1 | 4 | Contrast-detect (subject tracking) | `Tracking` | observed, in my files only; my D600's live-view frames 53 to 56 hold it, their mode not recorded | 0 / 12 |
| `0200`, `0201` | none | 128 | Single | `Single` | observed | 7 / 0 |
| `0200` | none | 129 | Auto (41 points) | `Auto` | observed | 5 / 0 |
| `0201` | none | 130 | Subject Tracking (41 points) | `Tracking` | observed | 1 / 0 |
| `0201` | none | 0 | none: not in the 02xx list | none | none | 2 / 0 |
| `0400`, `0401` | 1, 2 | 192 | Pinpoint | `Pinpoint` | confirmed: my Z 9's frames 2484, 2485, 2509 and 2510 | 12 / 103 |
| `0300`, `0301`, `0400` | 2 | 193 | Single | `Single` | confirmed: my Z 9's frame 2486 | 37 / 34 |
| `0301`, `0400` | 1, 2 | 195 | Wide (S) | `Zone` | confirmed: my Z 9's frames 2493, 2494, 2497 and 2498 | 4 / 14 |
| `0400`, `0401` | 2 | 196 | Wide (L) | `Zone` | confirmed: my Z 9's frames 2495, 2496, 2499, 2500 and 2507 | 3 / 12 |
| `0301`, `0400`, `0402` | 1, 2 | 197 | Auto | `Auto` | confirmed: my Z 9's frame 2506 | 18 / 605 |
| `0301` | 2 | 199 | Auto in the first list, Auto (Animal) in the second | `Auto` | observed: both meanings fold into `Auto` | 2 / 0 |
| `0400` | 2 | 204, 205, 206 | Dynamic Area (S), (M), (L) | `Dynamic` | confirmed: my Z 9's frames 2487 to 2492 | 0 / 15 |
| `0400`, `0402` | 1, 2 | 207 | 3D-tracking | `Tracking` | confirmed: my Z 9's frame 2505 | 9 / 16,328 |
| `0400` | 1, 2 | 208 | Wide (C1/C2) | `Zone` | confirmed: my Z 9's frames 2501 to 2504 and 2508 | 0 / 86 |
| AFInfo | none | 0 | Single Area | `Single` | observed | 22 / 0 |
| AFInfo | none | 1 | Dynamic Area | `Dynamic` | observed | 15 / 0 |
| AFInfo | none | 2 | Dynamic Area (closest subject) | `Auto` | observed: the D70 JPEG of the [JPEG test data](../jpeg/test-data.md), counted on 2026-11-22 | 1 / 0 |

**[F]** The frames are listed in
[test-data.md](test-data.md#24-files-checked-for-the-camera-settings).
Every frame agrees with the documentation's name for its code. Under
manual focus the mode is still recorded: 0 on my D600's frames 45 and
46, and 193 on the Z fc (file 8559). Four Coolpix files write AFInfo's
1, and no measured value of theirs contradicts it. **[F]** The public
counts of AFInfo are the NEF and NRW files' and, for 2, the D70 JPEG's;
the other public JPEGs with AFInfo hold 0 or 1. **[F]**

**The codes no file holds.** Each is documented only, named with its
list, and checked on files built in the tests alone:

| List | Code | The documentation's name [S][1] | `FocusAreaMode` |
|---|---|---|---|
| `0100` to `0301`, phase detection | 2 | Dynamic Area (closest subject) | `Auto` |
| | 3 | Group Dynamic | `Zone` |
| | 10 | Single Area (wide) | `Single` |
| | 11 | Dynamic Area (wide) | `Dynamic` |
| | 12 | Dynamic Area (wide, 3D-tracking) | `Tracking` |
| | 13, 16, 17 | Group Area; (HL); (VL) | `Zone` |
| | 14, 15, 18 | Dynamic Area (25, 72 and 49 points) | `Dynamic` |
| | 128 | Single | `Single` |
| | 129 | Auto (41 points) | `Auto` |
| | 130 | Subject Tracking (41 points) | `Tracking` |
| | 131 | Face Priority (41 points) | none: no neutral word fits |
| | 192 | Pinpoint | `Pinpoint` |
| | 193 | Single | `Single` |
| | 194 | Dynamic | `Dynamic` |
| | 195, 196 | Wide (S), Wide (L) | `Zone` |
| | 197, 199 | Auto | `Auto` |
| `0100` to `0301`, contrast detection | 128 to 131, 192 to 197 | as under phase detection | as under phase detection |
| | 198 | Auto (People) | `Auto` |
| | 199 | Auto (Animal) | `Auto` |
| | 200 | Normal-area AF | `Single` |
| | 201 | Wide-area AF | `Zone` |
| | 202 | Face-priority AF | none: no neutral word fits |
| | 203 | Subject-tracking AF | `Tracking` |
| | 204, 205, 206 | Dynamic Area (S), (M), (L) | `Dynamic` |
| | 207 | 3D-tracking | `Tracking` |
| | 208 | Wide-Area (C1/C2) | `Zone` |
| `0100` to `0301`, hybrid detection | 128 to 130, 192, 194, 196, 198, 200 to 208 | by the list that gives the code, as above | as above |
| `0200`, `0201` | 131 | Face Priority (41 points) | none: no neutral word fits |
| AFInfo | 3 | Group Dynamic | `Zone` |
| | 4 | Single Area (wide) | `Single` |
| | 5 | Dynamic Area (wide) | `Dynamic` |

**[F]** Version 04xx's list, 192, 193, 195 to 197 and 204 to 208, is
whole on the files: each code is in the first table, and each is named
under any detection method, since that version has one list. The
mapped codes follow ADR011's neutral words.

### 8.7 Camera settings

How the photographer set the camera for the shot, which a viewer shows
in its information panel beside the exposure: the white balance, the
exposure program, the metering mode, the flash, the shutter, the image
area, the raw image size and the color space, and with them the focal
length in 35 mm terms and the software that wrote the file. The Exif
tags record some of these on almost every body. Nikon records the rest
only in its MakerNote, a few of them only in blocks that are encrypted
or that have a record format of their own. The further settings, from
the focus mode to the JPEG quality, are in
[section 8.10](#810-more-capture-settings).

Every value below was read on the 35 NEF and NRW files of
[test-data.md](test-data.md#21-the-ci-set) and on the D70 and D5300
JPEGs of the [JPEG test data](../jpeg/test-data.md#2-the-sample-files)
on 2026-10-23, with a bounds-checked script written from these pages
**[F]**; [test-data.md](test-data.md#24-files-checked-for-the-camera-settings)
lists which file records which value. The meaning of each MakerNote
code is ExifTool's, read in its published tag documentation [S][1];
the meaning of each Exif code is the standard's [P][23]. ExifTool
was also run on the same files as an independent reader to compare
with: it reads the same value as the script for every tag both read,
and prints the four bytes of MakerNoteVersion (0x0001) on the E5700
and the P7800 (files 4456, 1425) as `0200` where the script reads them
as the bytes 0, 2, 0, 0. Where this section reports a value that
ExifTool decodes from a block the script does not read, it is tagged
**[S]**.

**Where a Nikon file records each setting.**

| Setting | Standard tag | MakerNote tag | Checked files |
|---|---|---|---|
| Exposure program | Exif 0x8822 ExposureProgram | none needed | Exif on every file |
| Metering mode | Exif 0x9207 MeteringMode | none needed | Exif on every file |
| Flash | Exif 0x9209 Flash | 0x0087 FlashMode | Exif on every file but the D1H's and the D1X's (files 5364, 1785, 1786); 0x0087 on every file but the E5700's (file 4456) |
| White balance | Exif 0xA403 WhiteBalance, auto or manual, and 0x9208 LightSource | 0x0005 WhiteBalance, as text | 0x0005 on every file; Exif on every file but the D1H's, the D1X's and the E5700's |
| Color temperature | none | 0x004F ColorTemperatureAuto; under a chosen color temperature, 0x0005's number too | 0x004F on the D5, D850, D780 and every Z body (19 files); a chosen temperature in 0x0005 on no file here, only on the frames shot for these values (below) |
| White balance fine-tuning | none | 0x003F, else 0x000B, both WhiteBalanceFineTune | 0x000B on 21 NEF files and both JPEGs, 0x003F on 22 files from the D810 on; neither on the E5700 |
| Shutter | none | 0x0034 ShutterMode | the Nikon 1 J1, D850, D780 and every Z body (19 files) |
| Image area | none | 0x001B CropHiSpeed, its first value | the D200 and every later body but the Coolpix P7800 (28 NEF files and the D5300 JPEG) |
| Raw image size | none | 0x003E ImageSizeRAW | the D750, D810, D5600 and every later body (22 files) |
| Color space | Exif 0xA001 ColorSpace | 0x001E ColorSpace | Exif on no NEF file and on both JPEGs; 0x001E on the D200 and every later body but the Coolpix P7800 (28 NEF files and the D5300 JPEG) |
| Focal length in 35 mm terms | Exif 0xA405 FocalLengthIn35mmFilm | none | Exif on every file but the D1H's, the D1X's and the E5700's |
| Software | IFD0 0x0131 Software | ShotInfo's FirmwareVersion, encrypted | IFD0 on every file |

The standard tags: [P][23] (4.6.5.4.4, p. 42; 4.6.6.2.1, p. 47;
4.6.6.7.3, p. 55; 4.6.6.7.19 to 4.6.6.7.21, pp. 60 to 61; 4.6.6.7.37,
p. 69; 4.6.6.7.39, p. 70)

**Exposure program and metering mode.** Both are Exif tags on every
checked file, the D1H's included **[F]**, with Exif's codes. [P][23]
(4.6.6.7.3, p. 55; 4.6.6.7.19, p. 60) ExposureProgram is 1 (manual) on
11 files, 2 (normal program) on five and the D70 JPEG, 3 (aperture
priority) on 11, 4 (shutter priority) on the D700 (file 2255), and 0,
"Not defined", on the seven Z5II and Z6III files (7735, 7736, 7808 to
7810, 7812, 7813) and the D5300 JPEG: a recent body writes 0 too.
MeteringMode is 5 ("Pattern", raw-files' `MeteringMode::MultiSegment`)
on 24 NEF files and both JPEGs, 2 (center-weighted average) on nine
files (928, 1786, 2238, 5146 to 5148, 7767, 7770, 8079), 3 (spot) on the
D100 (file 2996) and 255 ("other") on the D850 (file 1835) **[F]**,
perhaps for its highlight-weighted metering, which Exif has no code for
[I].

**Flash.** Exif's Flash is a set of bits: bit 0 says whether the flash
fired, bits 1 and 2 whether its return light was detected, bits 3 and 4
its mode (1 compulsory firing, 2 compulsory suppression, 3 auto), bit 5
that the camera has no flash function, bit 6 red-eye reduction. [P][23]
(4.6.6.7.21 and Figure 19, pp. 61 to 62)
The checked files hold 0 (no flash, no mode; 23 NEF files and the D70
JPEG), 16 (not fired, mode 2; files 961, 1415, 1425, 4456, 7767, 7770
and the D5300 JPEG), 15 (fired in mode 1, return light detected; the
D750 and D810, files 895 and 2036) and 7 (fired, return light detected,
no mode; the D100, file 2996) **[F]**. The MakerNote's 0x0087 FlashMode
(0 did not fire, 7 fired with an external flash, 9 fired in TTL mode,
among others [S][1]) agrees wherever both exist: 7 on files 895 and
2036, 9 on file 2996, 0 elsewhere **[F]**. On the D1H and D1X files it
is the only record of the flash, and it says 0 even on the D1X file
whose white balance is set to `FLASH` (file 1786) **[F]**.

**White balance.** The photographer chooses auto white balance, a kind
of light (daylight, shade, incandescent and so on), a color
temperature, or a preset measured from a gray card, and may shift the
result (the fine-tuning below). Exif records only whether it was auto
(0xA403: 0 auto, 1 manual) and, in a tag of its own, the kind of light
(0x9208 LightSource). [P][23] (4.6.6.7.37, p. 69; 4.6.6.7.20, p. 60)
Nikon's 0x0005 records the setting itself,
as 13 bytes of ASCII: an upper-case word padded with spaces to 12
characters, then a NUL **[F]**. ExifTool documents it as a string and
lists no values [S][1]. The checked files hold:

| 0x0005 | Files |
|---|---|
| `AUTO` | the D300, D5600, D1X, D70, D700, Nikon 1 J1, E5700 and D200 (files 928, 1415, 1785, 2060, 2255, 2956, 4456, 8079) and both JPEGs |
| `AUTO0` | the D5 and the Z 9 (files 2218, 5146 to 5148) |
| `AUTO1` | the D750, D7000, P7800, D810, Coolpix A, Z 6, D780, Z 30, Z f, Z5II, Z50II and Z6III (18 files) |
| `SUNNY` | the D850, D1H and Z fc (files 1835, 5364, 8559) |
| `FLASH` | the D1X (file 1786) |
| `SHADE` | the D100 (file 2996) |

The Exif tags agree with it on every file that has both: 0xA403 is 0
and LightSource 0 ("unknown") under every `AUTO` setting, and 0xA403 is
1 under `SUNNY` (LightSource 9, "Fine weather") and `SHADE`
(LightSource 11, "Shade") **[F]**; the quoted names are Exif's.
[P][23] (4.6.6.7.20, pp. 60 to 61) The D1H, D1X and E5700 write no Exif
white balance, so 0x0005 is their only record of it. The texts of the
other settings are on my own files and on frames shot for these values
(below): a chosen color temperature is written as its number, such as
`3230K`.

**Color temperature.** 0x004F ColorTemperatureAuto is one SHORT, in
kelvin [S][1], on the D5, D850, D780 and every Z body (19 files). Under
an `AUTO` setting it holds the temperature auto white balance chose:
3050 to 7000 on 17 files. Under `SUNNY` it is 0 (the D850 and the Z fc,
files 1835 and 8559) **[F]**: 0 means it was not recorded. My Z 9
records it under every setting, a chosen color temperature and a
preset included (below). No other plain tag, Exif or MakerNote,
records the temperature the camera applied on the older bodies:
ExifTool reports none there **[S]**. A chosen color temperature is
the exception, since 0x0005 records it as its number: the D600 writes
`3230K` there with no 0x004F beside it (below).

**White balance fine-tuning.** A shift of the white balance along two
axes, amber to blue and green to magenta. Nikon writes it in two tags,
both called WhiteBalanceFineTune [S][1]:

- 0x000B, signed SHORTs: one value on the D1H, D1X, D70, D100 and D200
  (files 5364, 1785, 1786, 2060, 2996, 8079) and the D70 JPEG; two on
  15 files from the D300 to the Z fc, the Coolpix P7800 and A and the
  Nikon 1 J1 included, and on the D5300 JPEG **[F]**.
- 0x003F, two SRATIONALs, on the 22 files from the D810 on. The Z 9,
  Z f, Z5II, Z50II and Z6III write it alone, without 0x000B **[F]**.

Every checked value is zero: `0` or `0, 0` in 0x000B, `0/4, 0/4` in
0x003F **[F]**. Frames shot with a known shift show the axes (below):
the first value is amber to blue, the second green to magenta, and a
positive value is a shift towards blue and towards green **[F]**. That
holds for the two values of 0x003F and of 0x000B; which axis and sign
the single value of the oldest bodies has, no file shows. Both count
the steps of the camera's fine-tuning menu: Nikon's NX Studio shows a
0x003F of `-12/4, 8/4` as "A3.00, G2.00", so 0x003F counts them in
quarters, and 0x000B in whole steps; on my own D7200 files 0x003F
holds 0x000B's value in quarters (below). How large a menu step is, no
documentation checked says. The E5700 writes neither tag **[F]**.

**Shutter.** Which shutter took the picture: the mechanical one, the
electronic front curtain (an electronic start of the exposure and a
mechanical end), or a fully electronic one. 0x0034 ShutterMode is one
SHORT, with ExifTool's codes [S][1]: 0 mechanical, 16 electronic, 48
electronic front curtain, 64 electronic (movie), 80 auto (mechanical),
81 auto (electronic front curtain), 96 electronic (high speed). An auto
code records both the setting, auto, and the shutter the camera chose.
The checked files hold **[F]**:

| 0x0034 | Files |
|---|---|
| 0 | the D850, D780 and Z f (files 1835, 3827, 6887) |
| 16 | the Nikon 1 J1 and the Z 9 (files 2956, 5146 to 5148) |
| 80 | the Z5II (files 7735, 7736) |
| 81 | the Z 6, Z 30, Z50II, Z6III and Z fc (files 3587, 5812, 7767, 7770, 7808 to 7810, 7812, 7813, 8559) |

No checked DSLR older than the D850 (2017) writes it, the D5 (2016)
included, and no block that ExifTool decodes records the shutter on
those bodies **[S]**. On the Z 6, Z 30 and Z fc, 0x004E NikonSettings,
a block with a record format of its own, holds a ShutterType that
ExifTool reports as "Auto" [S]: the setting alone, which 0x0034
also gives.

**Image area.** The part of the sensor the picture uses: the whole
sensor, a DX crop of an FX sensor (the APS-C size), or an aspect crop
such as 1:1 or 16:9. The first value of 0x001B CropHiSpeed records it,
with the codes of [section 8.4](#84-image-dimensions) [S][1]. The
checked files hold **[F]**:

| CropHiSpeed | Files |
|---|---|
| 0 "off" | the D200, D300, D700, D7000, Coolpix A and Nikon 1 J1 (files 8079, 928, 2255, 961, 2238, 2956) and the D5300 JPEG |
| 11 "FX uncropped" | the D750, D850, D810, D5, Z 6, D780, Z 9 and Z f (files 895, 1835, 2036, 2218, 3587, 3827, 5146 to 5148, 6887) |
| 12 "DX uncropped" | the D5600, Z 30 and Z fc (files 1415, 5812, 8559) |
| 2 "DX crop" | the Z6III's DX file (7810) |
| 17 "FX 1:1 crop" | the Z5II's and Z6III's 1:1 files (7735, 7808, 7812) |
| 6 "16:9 crop" | the Z5II's and Z6III's 16:9 files (7736, 7809, 7813) |
| 18 "DX 1:1 crop" | the Z50II's 1:1 file (7767) |
| 9 "DX movie 16:9 crop" | the Z50II's 16:9 file (7770), a still |

So 0 does not say which sensor the body has: the D700 (FX) and the
D300 (DX) both write it. The five bodies of raw-files' table of bodies
([camera-names.md](../camera-names.md), section 5), the D600, D300S,
D3200, D3300 and D7100, write 0 on every shot that uses the whole
sensor: on 16 public files (916 to 919, 3049, 3052 to 3056, 743, 1734,
1855 to 1858) and 17,215 of mine, CropHiSpeed's cropped size equals its
original, and every raw file stores the whole sensor **[F]**. Where
they crop, they write the crop's code: 2 on my D600's DX frames, 9 on
two files of my D7100. raw-files gives a file of 0 the body's sensor
format from the table. And code 9, which ExifTool names for movies,
is a DX body's 16:9 still too. The bodies before the D200 and the
Coolpix E5700 and P7800 write no CropHiSpeed (seven NEF files and the
D70 JPEG). ShotInfo holds an ImageArea in the part ExifTool calls
IntervalInfo, encrypted, which ExifTool decodes on the Z 6, Z 30, Z f
and Z fc **[S]**: "FX" on the Z 30 and Z f, "DX" on the Z 6 and Z fc,
so it contradicts CropHiSpeed on the Z 6 (FX uncropped, file 3587) and
on the Z 30, a DX body (file 5812).

**Raw image size.** Whether the camera was set to its large, medium or
small raw size. 0x003E ImageSizeRAW is one SHORT [F] with ExifTool's
codes 1 large, 2 medium, 3 small [S][1]. It is 3 on the three small-raw
samples, the D850, D810 and Z 6 (files 1835, 2036, 3587), and 1 on the
19 other files that have it, from the D810 on **[F]**. No checked file
is a medium raw, and the Z 9 makes none: on firmware 5.30 its menu
offers the large, medium and small sizes for the JPEG only, when it
shoots a raw file and a JPEG together. The five bodies of raw-files'
table of bodies ([camera-names.md](../camera-names.md), section 5),
which offer no raw sizes, write no ImageSizeRAW: not on their 16
public raw files, nor on 5,989 of mine **[F]**. raw-files gives their
raw files the large size from the table. The size in pixels is
CropArea's ([section 8.4](#84-image-dimensions)).

**Color space.** The color space the camera was set to for its JPEG
output, sRGB or Adobe RGB. No checked NEF file has the Exif ColorSpace
(0xA001) or an InteropIndex; both JPEGs have ColorSpace 1 **[F]**, which
Exif defines as sRGB. [P][23] (4.6.6.2.1, p. 47) 0x001E ColorSpace is
one SHORT with ExifTool's codes 1 sRGB, 2 Adobe RGB, 4 BT.2100 [S][1]: 2
on the Coolpix A, D700, Nikon 1 J1, Z 30 and D200 (files 2238, 2255,
2956, 5812, 8079), 1 on the other 23 NEF files that have it and on the
D5300 JPEG **[F]**. The NEF files of the bodies before the D200 and of
the Coolpix E5700 and P7800 record a color space nowhere in plain form
(seven files).

**Focal length in 35 mm terms.** Exif 0xA405 FocalLengthIn35mmFilm is
one SHORT in millimeters. [P][23] (4.6.6.7.39, p. 70) It is on every
file but the D1H's, the D1X's and the E5700's, and is never 0 on a
Nikon file **[F]**.

**Software and firmware.** IFD0's Software (0x0131) is on every
checked file, and on all of them it is the body's firmware version
**[F]**: `Ver.` and a number, with a trailing space on the DSLRs from
the D1H to the D850 and on the Coolpix A (`Ver.1.10 `), without one on
the Nikon 1 J1 (`Ver.1.40`), with two digits before the point on the
D780 and the Z bodies (`Ver.01.00`, `Ver.02.00` on the Z 6, `Ver.01.70`
on the Z fc); the Coolpix E5700 and P7800 write `E5700v1.1` and
`COOLPIX P7800  V1.0`, with two spaces. No checked file carries an
editor's name; on my own JPEGs that a program rewrote, it names that
program (below). ShotInfo (0x0091) has a FirmwareVersion of its own: on
none of the 36 files that have ShotInfo does it hold readable text
where ExifTool documents it, and ExifTool decodes it from the encrypted
block ([section 7](#7-lensdata-and-shotinfo)) [S][1]. Its decoded value
**[S]** carries Software's number, on some bodies with a letter or a revision
added (`1.10B` on the D300, `01.00.k0` on the Z 9), on every body where
ExifTool reads it, except the D810: `Ver.1.12` in Software, `1.11d` in ShotInfo
(file 2036). No file can say which of the two is right; Software is the
plain one.

0x0051 and 0x0056 begin with ASCII digits on the Z bodies that write
them: 0x0051 with eight (`01000400` on the Z 6, `01010500` on the Z 9,
`01020501` on the Z f, Z5II, Z50II and Z6III), 0x0056 with four (`0100`
on the same bodies but the Z 6) **[F]**. ExifTool calls them
FirmwareVersion51 and FirmwareVersion56 [S][1]. They are not the body's
firmware: beside them the Z 6's Software is `Ver.02.00` and the Z 9's
`Ver.01.00`, and four bodies share `01020501` **[F]**. What they number
is not known; byte 10 of 0x0051 is the compression
([section 2](#2-key-tags)).

**On my own files.** My own raw files and camera JPEGs of six Nikon
DSLRs, the D300s, D3200, D3300, D600, D7100 and D7200, were read the
same way on 2026-10-24 and are given here as counts only
([test-data.md](test-data.md#24-files-checked-for-the-camera-settings)).
They add what the public samples lack **[F]**:

- **More white balance texts:** `CLOUDY` (D300s, D7200),
  `INCANDESCENT` (D7200), `PRESET` (D3200) and `COOL WHT FL` (D300s),
  beside `AUTO`, `AUTO1`, `SUNNY` and `SHADE`. Each text comes with one
  pair of Exif values: `SUNNY` with WhiteBalance 1 and LightSource 9,
  `CLOUDY` 1 and 10, `SHADE` 1 and 11, `INCANDESCENT` 1 and 3,
  `COOL WHT FL` 1 and 14 ("Cool white fluorescent", in Exif's words
  [P][23] (4.6.6.7.20, p. 61)), `PRESET` 1 and 0,
  and `AUTO` and `AUTO1` with 0 and 0 (13 D3200 files with `AUTO` have
  no Exif white balance). A text like `COOL WHT FL` could not have been
  guessed from the others.
- **Fine-tuning that is not zero:** 0x000B is `1, 0` on two D300s files,
  and `-6, 0` on 42 D7200 files and `-3, 0` on two. On every D7200 file
  with a Nikon MakerNote, 11,094 with one rewritten JPEG among them,
  0x003F holds 0x000B's two values as fractions of four: `-24/4, 0/4`
  beside `-6, 0`. A negative first value is a shift towards amber
  (below).
- **The color space:** none of these NEF files has an Exif ColorSpace
  either. 615 D3200 JPEGs have ColorSpace 65535 with InteropIndex `R03`,
  and every one has 0x001E 2, Adobe RGB; 5,179 have ColorSpace 1 with
  `R98` and 660 ColorSpace 1 with no Interoperability IFD, all with
  0x001E 1. Of 425 JPEGs that a program rewrote, 32 have ColorSpace
  65535 with no InteropIndex, and 10 of those still hold the camera's
  0x001E of 1, which no longer describes the picture.
- **Codes and gaps:** no file of the six bodies has 0x004F or 0x0034.
  0x003E is only on the D7200: 1 on its 6,277 NEF files and on 4,310
  JPEGs, and 0, which ExifTool's table does not list, on 506 JPEGs,
  presumably shots taken without a raw [I]. CropHiSpeed is 1 (1.3x
  crop) on 168 D7200 NEF files, 10 (1.3x movie crop) on three of its
  NEF files and four JPEGs, 9 on one D7100 NEF file and one JPEG, and 0
  on every D300s, D3200, D3300 and D600 file. ExposureProgram is 0 on
  5,691 of 10,036 D3200 files, 632 of 659 D3300 JPEGs, 2,680 of 3,106
  D7100 files, 1,930 of 11,093 D7200 files and 12 of 1,374 D600 files,
  presumably their scene and auto modes [I]. 0x0087 FlashMode is 8,
  which ExifTool calls "fired, commander mode", on 538 files whose Exif
  Flash did not fire; 6, which its table does not list, on 112 such
  files; and 3, "not ready", on 11 D7200 files whose flash fired.
- **Software:** on the 425 JPEGs that a program rewrote (415 of the
  D3200, eight of the D7100, two of the D7200), Software names the
  program: Windows Photo Viewer (349), GIMP (49), Luminance HDR (20),
  Shotwell (5), ViewNX 2 (1) and darktable (1).

**Shots made for these values.** To see what each tag holds under each
setting, I shot frames of one scene on 2026-10-25, one or two per
setting, and read them the same way
([test-data.md](test-data.md#24-files-checked-for-the-camera-settings)):
19 NEF files from a Z 9 on firmware `Ver.05.30`, and 19 NEF files from
the D600, 14 of them with a JPEG of the same shot **[F]**.

- **White balance texts:** the Z 9 writes `AUTO0`, `AUTO1`, `AUTO2` and
  `NATURAL AUTO` under its four auto settings; `SUNNY`, `CLOUDY`,
  `SHADE`, `INCANDESCENT`, `COOL WHT FL` (the one fluorescent lamp
  tried) and `FLASH` under the kinds of light; `PRESET1` and `PRESET2`
  under two of its preset slots. The D600 writes `AUTO1` and, under its
  four preset slots, `PRESET1` to `PRESET4`, which NX Studio names
  "Preset manual d-1" to "d-4". The Exif tags agree as on
  the other files: WhiteBalance 0 under auto; 1 with LightSource 9, 10,
  11, 3, 14 and 4 under the six kinds of light, and with LightSource 0
  under a preset.
- **A chosen color temperature:** both bodies store it as its number,
  the text `3230K`, with Exif WhiteBalance 1 and LightSource 0, and the
  Z 9's 0x004F holds 3230. The number is the setting, so no table of
  texts can list them all.
- **Color temperature:** the Z 9's 0x004F holds a temperature under
  every setting: 5210 under `SUNNY`, 5990 under `CLOUDY`, 8000 under
  `SHADE`, 3000 under `INCANDESCENT`, 4200 under `COOL WHT FL`, 5410
  under `FLASH`, 5200 under both presets, 3230 under `3230K`, and 4550
  to 4900 under auto. Under auto it moves with the fine-tuning: 4730
  without a shift, 4900 shifted towards amber, 4550 towards blue. The
  D850 and the Z fc write 0 under `SUNNY` (above); the D600 writes no
  0x004F.
- **Fine-tuning:** Nikon's NX Studio, run on the frames as a reader to
  compare with, names each shift. On the Z 9, under `AUTO1`, 0x003F
  holds `-12/4, 8/4` on the frame it shows as "A3.00, G2.00", with
  0x004F 4900 beside the "4900K" it shows; `8/4, -8/4` is on a second
  frame. On the D600, 0x000B holds `3, 0` on the frame it shows as
  "Auto1, B3, 0", `-3, 0` on "Auto1, A3, 0", `0, 3` on "Preset manual
  d-1, 0, G3" and `-3, 0` on "Preset manual d-4, A3, 0". So on both
  bodies the first value runs from amber (negative) to blue (positive)
  and the second from magenta (negative) to green (positive). The
  camera's own pictures agree: the Z 9's frame at `-12/4, 8/4` is
  warmer and greener than the frame shot without a shift, and its
  frame at `8/4, -8/4` cooler and more magenta; the D600's JPEG at
  `3, 0` is cooler than the one at `-3, 0`, and its preset frames at
  `0, 3` and `0, -3` greener and more magenta. The pictures compared
  are the D600's JPEGs and the full-size preview the Z 9 writes into
  each NEF file.
- **Color space:** set to Adobe RGB, both bodies write 0x001E 2, and the
  D600's JPEG has Exif ColorSpace 65535 with InteropIndex `R03`.
- **Image area:** the D600 set to a DX crop writes CropHiSpeed 2, as the
  Z6III's DX file does.

Every value of these frames lies inside the MakerNote **[F]**.

### 8.8 Authorship

Who made the picture and what was written about it. Exif records the
artist in IFD0's 0x013B Artist, the copyright notice in 0x8298
Copyright and a description in 0x010E ImageDescription; a comment in
the Exif IFD's 0x9286 UserComment; and, since Exif 3.0, the
photographer, the editor and the title in 0xA437 Photographer, 0xA438
ImageEditor and 0xA436 ImageTitle, beside the camera's owner in 0xA430
CameraOwnerName. Each is of type ASCII or UTF-8, the UTF-8 type being
the one Exif 3.0 added, except UserComment, which is UNDEFINED and
starts with 8 bytes naming its characters: `ASCII` and three NULs,
`JIS`, `UNICODE` (UTF-8), or eight NULs for undefined text, which Exif
leaves to the reader's locale. Exif says CameraOwnerName and
Photographer shall in principle not be rewritten. [P][23] (4.6.4 and
Tables 4 and 5, pp. 29 to 30; 4.6.5.4.1, 4.6.5.4.6 and 4.6.5.4.7, pp.
41 to 43; 4.6.6.4.2, p. 49; 4.6.6.9.2 and 4.6.6.9.8 to 4.6.6.9.10, pp.
79 to 81)

The numbers below and in [section 8.9](#89-location) were taken on 9
November 2026 with a counting program of my own over raw-files' own
view of every entry ([section 12](#12-the-directories-a-file-holds)),
written from Exif 3.1; no other reader was run. They cover the 289 NEF
and NRW files of raw.pixls.us and, as counts only, 29,447 of my own:
14,335 NEFs of my Z 9, 14,338 of five Nikon DSLRs and 774 of a folder
of files a program exported. **[F]** In sections 8.8 and 8.9, **[F]**
marks a count over these files rather than a fact of one named file.

- **Every text is plain ASCII**: of type ASCII, NUL-terminated, with
  no byte above 0x7F and nothing after the NUL. No NEF uses the UTF-8
  type. **[F]**
- **Artist and Copyright are fixed fields.** Artist is 37 bytes on
  every file that has it; Copyright is 55 bytes on all but 8 public
  files, which write 33 or 34 bytes of text (4) or 8 of spaces (4).
  Unset, a field starts with a NUL or is all spaces; set, its text is
  padded with spaces on some bodies. On the public files Artist is on
  223 (text on 102, a NUL first on 102, spaces on 19) and Copyright on
  231 (text on 90, a NUL first on 114, spaces on 27), the text up to 36
  bytes of Artist and 34 of Copyright once trimmed; on mine both are on
  25,879 files, with text on 23,091. **[F]**
- **Nikon cameras write no description.** ImageDescription is on 13
  public files, all spaces (11, 32 or 33 bytes), and on none of mine.
  **[F]**
- **UserComment is 44 bytes, the code `ASCII` and 36 bytes of NULs,
  spaces or text**, on all but a few files: on 257 of the 268 public
  files that have it, the rest 48, 126 or 128 bytes, and on every one
  of mine. Its text is on 21 public files and 8,083 of mine, ending at
  a NUL or padded with spaces. 11 public files, 7 of them with the Make
  `NIKON`, have the undefined code followed by NULs or spaces. No NEF
  has the `JIS` or `UNICODE` code, or text under the undefined code.
  **[F]**
- **No NEF has CameraOwnerName, Photographer, ImageEditor or
  ImageTitle.** **[F]**

raw-files reads these texts into a value of their own,
`Metadata::authorship`, and the owner with the camera,
`Camera::owner_name`, as
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
decides: IFD0's texts from IFD0 alone, each text cleaned as every text
is, from either type where Exif allows both; the description and the
comment without the 255-byte limit; the comment only under the `ASCII`
code. A comment under another code is `None`, and the view of every
entry gives its bytes. The photographer, the editor, the title and the
owner are checked on built files only, since no file here records
one; the tests of the public samples check which texts each file
gives, never what they say.

### 8.9 Location

Exif records where the picture was taken in the GPS IFD, which IFD0's
0x8825 points to: GPSVersionID (0x0000, four BYTEs, 2.4.0.0 in Exif
3.1); the latitude and longitude, each a reference letter (`N` or `S`,
`E` or `W`, ASCII, count 2) and three RATIONALs, degrees, minutes and
seconds (0x0001 to 0x0004); the altitude, GPSAltitudeRef (0x0005,
BYTE) and GPSAltitude (0x0006, a RATIONAL in meters); the time,
GPSTimeStamp (0x0007, three RATIONALs, in UTC) with GPSDateStamp
(0x001D, `YYYY:MM:DD`, ASCII, count 11); the image direction,
GPSImgDirectionRef (0x0010, `T` true or `M` magnetic north) and
GPSImgDirection (0x0011, 0.00 to 359.99 degrees); the map datum,
GPSMapDatum (0x0012, ASCII); and 20 more tags, from the satellites
used to the horizontal positioning error. Exif names both the form
`dd/1, mm/1, ss/1` and degrees with minutes to a fraction,
`dd/1, mmmm/100, 0/1`. GPSAltitudeRef 0 and 1 mean at or above and
below the ellipsoid, and 2 and 3 above and below sea level; until
Exif 2.32, 0 and 1 meant sea level. The tags also hold positions from
other satellite systems and from mobile networks. [P][23] (Table 14,
pp. 82 to 83; 4.6.7 and 4.6.7.1.1 to 4.6.7.1.19, pp. 82 to 88;
4.6.7.1.30, p. 91)

- **A GPS IFD is no location.** It is on every NEF of mine and on 263
  of the 289 public files, and on 254 of those and on all of mine but
  262 it holds a single entry, its version: 2.3.0.0, or 2.2.0.0 on
  older bodies (43 of the public files and my D300S's 647), or 2.0.0.0
  on the D1H and D1X (3 public files). **[F]**
- **A position is on 6 public files and 262 of mine.** The public ones
  are 3 Z 9 files, 2 D5600 files and a Nikon 1 AW1 file; mine are all
  Z 9 files. Every one writes the latitude and longitude as degrees
  and minutes with four decimals (`d/1, m/10000, 0/1`), the GPS time
  with seconds over 100, the date, the satellites as two digits and
  the datum in 17 bytes: blank on the Z 9's, `WGS-84` on the D5600's
  and the AW1's. All but one write the altitude, with
  the reference 0; one public file writes the image direction, from
  magnetic north. **[F]**
- **Three Coolpix NRWs write the tags with nothing in them.** The
  P6000 (version 2.2.0.0), P7700 and P7800 (2.3.0.0) write every
  position tag, the altitude, the time and the image direction with
  blank references and zero denominators, and a blank datum of 10
  bytes. They record no position. **[F]**
- **Every GPS entry is of the type and count Exif gives**, every GPS
  IFD sorted by tag with no tag twice, and no entry left out. **[F]**

raw-files gives a location, `Metadata::location`, only where both
references and both coordinates are sound, as
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
decides, with the altitude, the time, the image direction and the
datum beside it; a GPS IFD of only a version, and the Coolpix files'
blanks, give none. The satellites, which every NEF with a position
records, stay in the view of every entry, as do the tags no NEF here
writes. The tests of the public samples check which parts of a
location each file gives, never where it lies: the three Z 9 samples
give one with an altitude and a time, and no other sample gives one.

Every text and GPS value counted lies inside the blocks raw-files'
`open` already reads. **[F]**

### 8.10 More capture settings

More of how the camera was set, as camera settings
([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)).
raw-files reads:

- the focus mode, AF fine-tune, stabilization and auto ISO;
- the exposure fine-tuning, the release mode and bracketing, and
  daylight saving time;
- the flash's source, control, sync and compensation;
- the raw compression and bit depth, and the JPEG quality;
- the white balance's preset slot and fluorescent lamp.

The AF-area mode is in [section 8.6](#86-focus-point).

The values were counted on the files and on the day of section 8.6's
AF-area table, by the same program **[F]**. Counts read "public / mine":
the 289 public NEF and NRW files, then my 48,395. The marks
**confirmed**, **observed** and **documented only** are those of
[section 8.6](#86-focus-point), and so is the rule that names every code
of a documented list once files hold part of it. The meaning of each MakerNote code comes from
ExifTool's published tag documentation [S][1], read as documentation,
and no reader's source was read. ExifTool and exiv2 were not run for
this section. Where a camera writes its own text, its meaning is the
maker's. The D600's manual [25] and the Z 9's Reference Guide [26] name
the settings [P]. Every value read lies inside what `open` already
reads. **[F]**

**Focus mode.** 0x0007 FocusMode is ASCII, seven bytes with the NUL,
padded with spaces. raw-files trims it and reads it as a camera
setting:

| Text | Setting [P][25][26] | `FocusMode` | Mark | Files, public / mine |
|---|---|---|---|---|
| `AF-S` | single-servo AF | `Single` | confirmed: my Z 9's frames 2484 to 2486, 2497 to 2504 and 2506 to 2510 | 104 / 4,554 |
| `AF-C` | continuous-servo AF | `Continuous` | confirmed: my Z 9's frames 2487 to 2496 and 2505, my D600's frames 7856 and 7858 | 65 / 27,526 |
| `AF-A` | auto-servo AF, the camera choosing between the two | `Automatic` | observed | 54 / 15,228 |
| `AF-F` | full-time AF, which the D600 offers in live view | `FullTime` | observed: 3 Coolpix files | 3 / 0 |
| `MANUAL` | manual focus | `Manual` | confirmed: my D600's frames 45 and 46 | 61 / 1,087 |

**[F]** ExifTool documents the tag as a string and lists no values
[S][1]. Nikon writes no direct manual focus mode, so `DirectManual`
serves other makers, Sony's DMF among them.

**AF fine-tune.** 0x00B9 AFTune is four UNDEFINED bytes [S][1]:

- whether fine-tuning is on;
- the index of the saved lens, 255 for none;
- the value, signed;
- the value at a zoom's long end, signed.

The D600's menu tunes up to 12 lenses, from -20 to +20 [P][25] (p. 259).
raw-files reads whether it is on, with its value, and under code 3 the
value at the long end too:

| Byte 0 | The documentation's name [S][1] | Read as | Mark | Files, public / mine |
|---|---|---|---|---|
| 0 | Off | off | observed | 189 / 37,260 |
| 1 | On (1) | on, with byte 2's value | observed | 15 / 3 |
| 2 | On (2) | on, with byte 2's value | observed | 21 / 76 |
| 3 | On (Zoom) | on, with byte 2's value and byte 3's at the long end | documented only | 0 / 0 |

**[F]** The values seen run from -20 to +20 on my D600, and -20 and
-2 on public D7000 and D300 files. Byte 3 is 0 on every file. The
documentation calls it valid only under code 3, which no file holds,
so under 1 and 2 it is not read. Code 3 is named with its list, as
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
decides, and is checked on built files alone. What tells 1 from 2 is
not documented: both read as on, and the difference stays open.
**[F]**

**Stabilization.** 0x001F VRInfo starts with a four-digit version. Byte
4 says whether vibration reduction was on, and byte 6 its mode [S][1].
The versions agree with the bodies the documentation places them on:

| Version | Length | Bodies | Files, public / mine |
|---|---|---|---|
| `0100` | 8 | the DSLRs from the D2Hs and D2X on, but the D6 and D780; the Coolpix A | 147 / 31,198 |
| `0101` | 8 | the Nikon 1 bodies | 15 / 0 |
| `0200` | 9 | the Z bodies, the D6 and the D780 | 97 / 17,197 |

**[F]** Mine are my six DSLRs' files at 0100 and my Z 9's at 0200. No
frame was shot in a known stabilization setting, so no code here is
confirmed.

Byte 6 has two lists in the documentation: one for the Z bodies "thru
July 2025", and one for the others. Each is read whole, since files hold
part of each: the Z bodies' list at version 0200, the others' at 0100
and 0101.

| Byte | Code | The documentation's name [S][1] | Read as | Mark | Files, public / mine |
|---|---|---|---|---|---|
| 4 | 0 | n/a | `None` | observed | 4 / 0 |
| 4 | 1 | On | on | observed | 113 / 41,910 |
| 4 | 2 | Off | off | observed | 142 / 6,485 |
| 6, `0100`, `0101` | 0 | Normal | `Normal` | observed, on `0100` | 36 / 24,899 |
| 6, `0100`, `0101` | 1 | On (1) | no mode: it names none | documented only | 0 / 0 |
| 6, `0100`, `0101` | 2 | Active | `Active` | observed, on `0101` | 6 / 0 |
| 6, `0100`, `0101` | 3 | Sport | `Sport` | documented only | 0 / 0 |
| 6, `0200` | 0 | Off (Z bodies) | no mode: byte 4 says whether it was on | documented only | 0 / 0 |
| 6, `0200` | 1 | Normal (Z bodies) | `Normal` | observed | 71 / 2,345 |
| 6, `0200` | 3 | Sport (Z bodies) | `Sport` | observed, in my files only | 0 / 14,666 |

**[F]** The counts of byte 6 count the files where byte 4 says on.
raw-files reads the mode only where byte 4 says on, so the two
lists' 0 never meet on a file. A record of another version, or of
another length than its version's, gives no stabilization, and so does
byte 4's 0. On version 0100 byte 6 is 0 on every
file, on and off alike. Byte 5, which the documentation does not name,
repeats byte 4 on 139 public 0100 files and on all of mine. Still open,
since no frame was shot to settle them:

- whether 0100's byte 6 ever holds another mode, such as active on a
  D600 with a lens that offers it;
- whether the Z bodies' 1 and 3 are normal and sport as documented;
- which version and list a Z body later than July 2025 writes: no
  measured file shows one.

Byte 8 of 0200, the kind of stabilization, is not read: it is 2
without a VR lens and 3 with one on my files, and what it records is
open ([section 12.5](#125-what-the-view-leaves-out)).

**Auto ISO.** Bit 5 of 0x0089 ShootingMode is auto ISO [S][1]. It is set
on 71 of the 271 public files that have the tag and on 30,403 of my
48,395. **[F]** The documentation gives the D70's bit 5 another meaning,
and the one public D70 file sets it (value 33). So the D70 gives no
auto ISO. The Coolpix bodies but the Coolpix A write no ShootingMode.
Their 0x000F ISOSelection says `AUTO` or `MANUAL` instead, on 16 public
files (9 and 7). **[F]** raw-files reads the bit, else that text:
`AUTO` on, `MANUAL` off, and any other text none. A file without a
Model takes the bit as every body but the D70 has it. Both are
observed; no frame was shot with auto ISO set either way. The D1
bodies, the D100 and the D2H write ShootingMode as one BYTE, and give
none of its values. **[F]**

**Exposure fine-tuning.** The photographer's fine-tuning of the camera's
metering, from -1 to +1 EV in sixths on the D600 [P][25] (p. 226).
0x001C ExposureTuning is three UNDEFINED bytes: a signed count, then 1
and 6. ExifTool's documentation gives the tag's name and its three
bytes alone [S][1]. raw-files reads the count times the second byte
over the third, in EV: `fd 01 06` is -3/6. Of 242 public files, 238
hold `00 01 06` and four D750 files `fd 01 06`, -3/6 EV. All of mine
hold `00 01 06`. **[F]** The reading rests on the D600's manual, whose
steps of 1/6 EV the bytes 1 and 6 match, and on every file agreeing
with it, -3 lying inside the menu's ±6 steps; no frame shows it. A
divisor of zero gives none, and so does a second byte of zero, which
would read any count as zero; no file holds either. 36 of my JPEGs store the tag with four
bytes, 35 of them made by a program from a raw file. They give none, by
the rule on a value's count. **[F]**

**Release mode and bracketing.** The bits of 0x0089 ShootingMode [S][1],
with the files that set each:

| Bit | The documentation's name [S][1] | Read as | Mark | Files, public / mine |
|---|---|---|---|---|
| 0 | Continuous | `ReleaseMode::Continuous` | observed | 71 / 29,902 |
| 1 | Delay | not read: the exposure delay mode [P][25] (p. 231), beside any release mode | observed | 30 / 8 |
| 2 | PC Control | not read: a release from a computer, beside any release mode | observed | 12 / 0 |
| 3 | Self-timer | `ReleaseMode::SelfTimer` | observed | 10 / 0 |
| 4 | Exposure Bracketing | `Bracketing::Exposure` | observed | 3 / 18 |
| 6 | White-Balance Bracketing | `Bracketing::WhiteBalance` | documented only | 0 / 0 |
| 7 | IR Control | `ReleaseMode::Remote` | observed | 2 / 1,109 |
| 8 | D-Lighting Bracketing | `Bracketing::DynamicRange` | documented only | 0 / 0 |
| 9 | none | not read | none | 2 / 277 |
| 11 | Pre-capture | no release mode: a burst ([section 12.5](#125-what-the-view-leaves-out)) | documented only | 0 / 0 |

**[F]** None is confirmed. Bits 6 and 8 are on no file, and are
named with the rest of the list, as section 8.6 says. With none of bits
4, 6 and 8 set, bracketing is `Off`. No bit names a single frame: the
documentation names no value 0, and Nikon's single frame, quiet release
and mirror up all set none of bits 0, 3 and 7. raw-files reads that as
`ReleaseMode::Single`, the three folded into one, as Sony's 0 is read.
Two of bits 0, 3 and 7 together, or two of bits 4, 6 and 8, name two
kinds and give none; no file holds either, among the values seen: 0, 1,
2, 4, 8, 16, 17, 24, 32, 33, 49, 128, 160, 512, 520, 528 and 544. Bit 11
set gives no release mode, since a burst would otherwise read as a
single frame. Bit 9 is undocumented. One
D600 frame of mine, outside the counted sets, sets it too, its setting
not recorded: what it records is open. Exif's ExposureMode 2, auto bracket [P][23] (4.6.6.7.36,
p. 69), gives `Bracketing::Exposure` where no MakerNote gives a
bracketing value, not even `Off`. No Nikon
file holds 2: 177 and 102 public files hold 0 and 1, and mine 0 and 1
too. **[F]**

**Daylight saving time.** WorldTime's third byte
([section 8.2](#82-capture-date-and-time-zone)): 0 no, 1 yes [S][1]. It is 1 on
142 of the 251 public files with WorldTime, and on 8,913 of my 48,395,
all D3200 and Z 9 files. **[F]** Section 8.2 found the offset equal to
the zone plus 60 minutes where it is 1, so the files agree. raw-files
reads it as a camera setting, observed; any other value gives none.

**The flash's new parts.** Exif's Flash bits stay the flash's first
source ([section 8.7](#87-camera-settings)). Plain MakerNote texts and
counts add the rest; FlashInfo (0x00A8) will not be read
([section 12.5](#125-what-the-view-leaves-out)). No frame was shot with a known
flash setting, so each text is documented by the maker's manual, none
confirmed.

0x0009 FlashType gives the source before the comma and the control after
it. raw-files reads both, looking each text up whole in this table of the
texts files hold:

| Text | Setting [P][25] (pp. 143 to 146) | Source, control | Files, public / mine |
|---|---|---|---|
| blank | no flash | none | 254 / 44,426 |
| `Built-in,TTL` | built-in flash, i-TTL | `BuiltIn`, `Ttl` | 0 / 621 |
| `Built-in,TTL-BL` | built-in flash, i-TTL balanced fill-flash | `BuiltIn`, `Ttl` | 0 / 1,770 |
| `Optional,TTL` | a flash unit, i-TTL | `External`, `Ttl` | 4 / 1,249 |
| `Optional,TTL-BL` | a flash unit, balanced fill-flash | `External`, `Ttl` | 5 / 0 |
| `Optional,AA` | a flash unit, auto aperture | `External`, `AutoAperture` | 7 / 0 |
| `Optional,M` | a flash unit, manual | `External`, `Manual` | 0 / 211 |
| `Optional,RPT&Comdr.` | a flash unit, repeating flash, commander mode | `External`, `Repeating` | 0 / 80 |
| `Optional,TTL&Comdr.` | a flash unit, i-TTL, commander mode | `External`, `Ttl` | 0 / 2 |
| `NEW_TTL` | none known | none | 1 / 0 |

**[F]** The D100's `NEW_TTL` names neither a source nor a control, so it
stays unnamed. The commander part is not read
([section 12.5](#125-what-the-view-leaves-out)). A text the table lacks
gives neither part. No file holds a FlashType text where Exif's Flash
says the flash did not fire; a file without Exif's Flash gives no
flash, whatever FlashType holds.

0x0008 FlashSetting gives the sync. raw-files reads it, looking each text
up whole in this table:

| Text | Setting [P][25] (p. 144) | `FlashSync` | Files, public / mine |
|---|---|---|---|
| blank | none recorded | none | 96 / 27,120 |
| `NORMAL` | front-curtain sync | `FrontCurtain` | 166 / 21,171 |
| `SLOW` | slow sync | `Slow` | 2 / 57 |
| `REAR` | rear-curtain sync | `RearCurtain` | 7 / 1 |
| `REAR SLOW` | rear curtain with slow sync | `SlowRearCurtain` | 15 / 10 |
| `RED-EYE` | red-eye reduction | `FrontCurtain`, red-eye reduction on | 1 / 0 |

**[F]** My Z 9 writes `NORMAL` on every NEF file, though it has no
built-in flash: the text is the setting, given whether or not the flash
fired. Under Nikon's red-eye reduction a lamp lights before the flash
fires, and the flash fires as the shutter opens unless rear-curtain sync
is set [P][25] (p. 145), so `RED-EYE` gives front-curtain sync and turns
red-eye reduction on. Exif's bit 6, which no Nikon file sets,
reads "no red-eye reduction mode or unknown" when clear, so the two
never contradict. The one `RED-EYE` file, a Nikon 1 AW1, has an Exif
Flash of 0, a flash that did not fire. **[F]**

0x0012 FlashExposureComp and 0x0017 ExternalFlashExposureComp are four
UNDEFINED bytes: a signed count, then 1, 6 and 0. ExifTool's
documentation gives their names and four bytes, and says 0x0012 may be
set even when the flash does not fire [S][1]. raw-files reads 0x0012 as
the count times the second byte over the third, in EV, as it reads the
exposure fine-tuning, so that `fa 01 06 00` is -6/6 EV:

| Tag | Files, public / mine | Not zero, public / mine | Counts seen |
|---|---|---|---|
| 0x0012 | 125 / 31,179 | 13 / 3,036 | -18 to +6 |
| 0x0017 | 266 / 48,359 | 5 / 8 | -4 to +18 |

**[F]** Every count of 0x0012 is a multiple of two, so a third of a
stop, from -3 EV to +1 EV. That is the range and step the D600's manual
gives [P][25] (p. 148), so the reading in sixths rests on the manual and
the files, as the exposure fine-tuning's does. The Z 50 and Z50II write
0x0012 as `00 01 00 00` (11 public files), a zero divisor, which gives
no compensation; so do a zero factor and a fourth byte other than zero,
which no file holds. A recorded zero, `00 01 06 00`, is a compensation of zero.
**[F]**

0x0012 is the camera's compensation for whichever flash fires, not the
built-in flash's alone. Measured on 2026-11-23 over the runs of
2026-11-20: on my files whose flash unit fired (FlashType
`Optional,…`), 0x0012 is not zero on 224 while 0x0017 is zero
there; 0x0017 is not zero on 8 of my files and 5 public ones, and both
are not zero on 3, such as the public D800E's, +1 EV in 0x0012 and +3 EV
in 0x0017. So 0x0017 holds a flash unit's own setting beside the
camera's, and is not read. 0x0012 is absent on my Z 9, which has no
built-in flash, and on the other bodies without one, which give no
compensation; whether the Z 9's 0x0017 or only FlashInfo holds a unit's
compensation is open. **[F]**

**Raw compression and bit depth.** The raw IFD's Compression says
whether the data is compressed: 1 uncompressed [P], 34713 Nikon's
compressed form ([raw-data.md](raw-data.md)). On 34713, Nikon's
NEFCompression refines it
([section 3](#3-nefcompression-and-bit-depth)): 0x0093, or byte 10 of 0x0051 on
the Z bodies that write no 0x0093. Byte 10 of 0x0051 is read as a
16-bit number in the MakerNote's byte order: the two readings section 2
cites, the MakerNote's order and little-endian, agree on every Z file
measured, whose MakerNotes are little-endian. The documentation gives
one list of NEFCompression's codes, and files hold part of it in each
tag, so both read it whole. raw-files reads them so:

| Code | The documentation's name [S][1] | `RawCompression` | Mark | 0x0093, public / mine | 0x0051, public / mine |
|---|---|---|---|---|---|
| 1 | Lossy (type 1) | `Lossy` | observed, in 0x0093 | 13 / 0 | 0 / 0 |
| 2 | Uncompressed | `Uncompressed` | observed, in 0x0093 | 1 / 0 | 0 / 0 |
| 3 | Lossless | `Lossless` | observed | 65 / 11,544 | 23 / 1,080 |
| 4 | Lossy (type 2) | `Lossy` | observed | 87 / 3,568 | 4 / 0 |
| 5 | Striped packed 12 bits | `Uncompressed` | documented only | 0 / 0 | 0 / 0 |
| 6 | Uncompressed (reduced to 12 bit) | `Uncompressed` | observed | 8 / 0 | 2 / 0 |
| 7 | Unpacked 12 bits | `Uncompressed` | observed, in 0x0093 | 3 / 0 | 0 / 0 |
| 8 | Small | none: a size, not a compression | observed, in 0x0093 | 2 / 0 | 0 / 0 |
| 9 | Packed 12 bits | `Uncompressed` | observed, in 0x0093 | 1 / 0 | 0 / 0 |
| 10 | Packed 14 bits | `Uncompressed` | observed | 4 / 0 | 2 / 0 |
| 13 | High Efficiency | `HighEfficiency` | observed, in 0x0051 | 0 / 0 | 14 / 27 |
| 14 | High Efficiency\* | `HighEfficiencyStar` | observed, in 0x0051 | 0 / 0 | 14 / 15,772 |

The raw IFD's Compression 1, uncompressed by TIFF 6.0 [P], is on 43
public camera files and none of mine, and gives `Uncompressed`. The
files of Nikon's two Coolscan film scanners in the set hold it too.

**[F]** The Z 9 samples 5146 to 5148, which the
[test data](test-data.md) lists as lossless, HE\* and HE, hold codes 3,
14 and 13. Their catalog entries state no compression, so they confirm
no code. Codes 5, 7, 9 and 10 are packings of uncompressed data, by
their names and, for 7, 9 and 10, by the data
([raw-data.md](raw-data.md)), so they fold into `Uncompressed`. The
one public file whose Compression is a LONG, a Coolpix A1000's, gives
none, by the rule on a value's type. **[F]** The raw bit depth is
the raw IFD's BitsPerSample where it holds one value. It is 12 on 153
public files and 3,568 of mine, and 14 on 134 and 28,423. **[F]** It is
the depth as recorded, which section 3 says a lossy curve can exceed by
two bits.

**JPEG quality.** 0x0004 Quality in a camera JPEG names the JPEG's
quality step. raw-files reads it in a JPEG only:

| Text | Setting [P][25] (p. 93) [26] (p. 103) | `JpegQuality` | Files, mine |
|---|---|---|---|
| `FINE` | JPEG fine | `Fine` | 14,894 |
| `NORMAL` | JPEG normal | `Normal` | 764 |
| `BASIC` | JPEG basic | `Basic` | 41 |
| `RAW` | a JPEG a program made from a raw file | none | 705 |

**[F]** No public Nikon JPEG is in the set. No frame was shot in a known
quality, so each text is documented by the manual. The 705 JPEGs that
say `RAW` came from a program, not the camera: 670 of them carry the raw
file's NEFCompression too. A raw file's Quality, `RAW`, `NRW` or
`RAW2.7M`, gives no JPEG quality. **[F]**

0x0044 JPGCompression is one SHORT: 1 size priority, 3 optimal quality
[S][1]. The D600's menu names the two [P][25] (p. 94), and the Z 9 marks
optimal quality with a star [P][26] (p. 103). raw-files reads 1 and 3 in
a JPEG as `JpegCompression::SizePriority` and `OptimalQuality`, the
whole list, both observed: on my files and on the public JPEG fixtures
that the
[JPEG notes](../jpeg/container-and-metadata.md#10-makernotes-in-a-jpeg)
count, though on no public NEF or NRW file:

| Code | Files, public / mine |
|---|---|
| 0, in every NEF that has the tag and in 17 Z 9 JPEGs a program made | 122 / 16,896 |
| 1, on my Z 9's JPEGs with `BASIC` | 0 / 27 |
| 3, on my Z 9's JPEGs with `FINE` | 0 / 274 |

**[F]** 0 is undocumented and gives none. Whether the Z 9's 3 and 1
mark the star or follow the step is open: no frame set both apart.

**The white balance preset slot and fluorescent lamp.** Two parts of the
white balance ([section 8.7](#87-camera-settings)). The preset slot is
the number after 0x0005's `PRESET`, read by its form, as a chosen color
temperature is:

| Text | Slot | Mark | Files, public / mine |
|---|---|---|---|
| `PRESET1` to `PRESET4` | 1 to 4 | confirmed: my D600's frames of 2026-10-25, each slot; my Z 9's frames 9967 to 9978, `PRESET1` and `PRESET2` | 0 / 4 |
| `PRESET0` | 0 | the D3X's text, its number read by its form, as a chosen color temperature's is; no frame | 1 / 0 |
| `PRESET` | none: no number | none | 2 / 32 |

**[F]** The count of mine is the Z 9's two frames, each counted twice;
the D600's frames lie outside the counted sets. The fluorescent lamp
comes from Exif's LightSource 12 to 16, as the standard names them
[P][23] (4.6.6.7.20, pp. 60 to 61): daylight, day white, cool white,
white and warm white. It is read under a manual white balance,
WhiteBalance 1, as the mode is, and a text that names another mode
drops it. Only 14, cool white, is on Nikon files. It comes
with `COOL WHT FL` on 15 of my D300S JPEGs and on my Z 9's frame of that
setting in the series above. **[F]** Nikon's text agrees with the
standard's code wherever both exist.

### 8.11 Picture style and processing

How the camera made its JPEG from the shot, which a maker's own
software shows beside the capture settings. raw-files reads, as camera
settings
([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)):

- the picture style, Nikon's Picture Control, with its values;
- Exif's six processing values;
- Active D-Lighting and HDR.

It will read:

- the high ISO and long exposure noise reductions;
- vignette control and auto distortion control;
- the scene mode.

Each setting keeps Nikon's own name and kinds. Where Sony has a
setting for the same thing, one value of the camera settings holds
either maker's
([ARW notes](../arw/makernote-and-metadata.md#911-picture-style-and-processing)).

**How it was measured.** On 2026-11-24 a counting program written from
these pages read each tag through raw-files' own raw view
(`CameraFile::entries` and `CameraFile::read_value`) **[F]**. It read:

- the 289 public NEF and NRW files of raw.pixls.us;
- as counts only, 48,632 files of my seven Nikon bodies: 31,991 NEF
  files and 16,641 JPEGs. 48,428 of them hold the MakerNote tags below.

Counts read "public / mine". The marks **confirmed**, **observed** and
**documented only** are those of [section 8.6](#86-focus-point), and so
is the rule that names every code of a documented list once files hold
part of it. The tag numbers, the offsets inside each record and the
meaning of each code come from ExifTool's published tag documentation
[S][1], read as documentation. No reader's source was read, and
ExifTool and exiv2 were not run. Each setting's name is Nikon's own,
from its manuals [P][25][26][27][28]. No frame was shot in a known
setting for this section, so no code is confirmed. I read one frame in
Nikon's NX Studio, run as a reader to compare with, and the text says
where it shows a value (below).

Every value read lies inside what `open` already reads, on every file
but one. The COOLPIX P6000's file (651) has a MakerNote of 70,052
bytes. Its last 206 bytes hold the scene text, the long exposure text
and the Picture Control. Reading them will cost that file one more
read. **[F]**

**The Picture Control.** Nikon's picture style is a preset of
sharpening, contrast, brightness, saturation and hue. The photographer
picks one and may adjust it [P][25] (pp. 129 to 133). 0x0023
PictureControlData records it, as UNDEFINED bytes that start with a
four-digit version. The Coolpix bodies write the same record under
0x00BD [S][1]. No measured file holds both tags. raw-files reads
0x0023, else 0x00BD.

The documentation gives three layouts, and does not say which version
takes which [S][1]. The files do: each version has one length, and
each length fits one layout. **[F]**

| Version | Length | Layout | Bodies | Files, public / mine |
|---|---|---|---|---|
| `0100` | 58 | first | 36 public bodies: the DSLRs from the D3, D300 and D90 to the D4S, D5300 and Df, the Nikon 1 bodies and the Coolpix A; my D300S, D3200, D3300, D600 and D7100 | 106 / 18,403 |
| `0100`, under 0x00BD | 58 | first | 12 Coolpix bodies, from the P6000 to the P1100 | 12 / 0 |
| `0200` | 68 | second | 11 public bodies: the D810, D750, D5500, D7200, D5, D500, D3400, D5600, D7500, D850 and D3500; my D7200 | 47 / 12,814 |
| `0300` | 108 | third | the D6, D780, Z 5, Z 6, Z 6II, Z 7, Z 7II, Z 30, Z 50 and Z fc | 56 / 0 |
| `0301` | 108 | third | the Z 9, on firmware 1.00 | 3 / 0 |
| `0302` | 108 | third | the Z 8 | 3 / 0 |
| `0310` | 108 | third | the Z f, Z5II, Z50II and Z6III; my Z 9 | 35 / 17,211 |

raw-files reads the record at these six versions, each at its one
length. A record of another version, or of another length than its
version's, gives no picture style: no file shows which layout it
takes.

The three layouts, as offsets from the record's first byte. The fields'
names and places are the documentation's [S][1], but for the rows the
text below explains. **[F]**

| Field | First, 58 bytes | Second, 68 bytes | Third, 108 bytes |
|---|---|---|---|
| Version, four ASCII digits | 0 | 0 | 0, and again at 4 |
| Name, 20 bytes | 4 | 4 | 8 |
| Base, 20 bytes | 24 | 24 | 28 |
| The control's number, not documented | 44 to 47 | 44 to 47 | 48 to 51 |
| Adjustment kind | 48 | 48 | 52 |
| Quick adjust | 49 | 49, divisor at 50 | none |
| Quick sharp | none | none | 53, divisor at 54 |
| Not documented | none | none | 55, divisor at 56 |
| Sharpening | 50 | 51 | 57 |
| Mid-range sharpening | none | none | 59 |
| Clarity | none | 53 | 61 |
| Contrast | 51 | 55 | 63 |
| Brightness | 52 | 57 | 65 |
| Saturation | 53 | 59 | 67 |
| Hue | 54 | 61 | 69 |
| Filter effect | 55 | 63 | 71 |
| Toning | 56 | 64 | 72 |
| Toning saturation | 57 | 65 | 73 |
| Not documented | none | 67 | 75 to 107 |

In the second and third layouts the byte after a value is its divisor.
The documentation does not name these bytes. **[F]**

**Two fields lie two bytes before their documented places** in the
third layout. The documentation puts the adjustment kind at byte 54
and quick adjust at byte 55 [S][1]. The files contradict it:

- Bytes 54 and 55 hold `01 FF` on every file, 97 public and 17,211 of
  mine, whatever the control. **[F]**
- From the second layout to the third, the name, the base and the
  control's number each move four bytes. So do these two, to bytes 52
  and 53. The values after them move further, since the third layout
  adds fields before and among them. **[F]**
- Byte 52 holds 0, 1 or 2, the documented list of the adjustment kind.
  Byte 53 holds 0x80, 0x81, 0x82 or 0xFF, with a divisor of 1 at byte
  54, as quick adjust has in the second layout. **[F]**
- Byte 53 moves the three sharpening values with it. My Z 9's `VIVID`
  holds sharpening +4, mid-range sharpening +2 and clarity +1 where
  byte 53 is 0x80 (3 files). It holds +5, +3.5 and +2 where byte 53 is
  0x81 (4,197 files). Its `LANDSCAPE` holds +6, +4.5 and +3 at 0x82
  (one file). **[F]** The Z 9's Quick sharp does that: it adjusts
  sharpening, mid-range sharpening and clarity together [P][26]
  (p. 181).
- Byte 52 is 0 wherever byte 53 is 0x80 or 0xFF, and 1 or 2 wherever
  it is 0x81 or 0x82. **[F]**
- NX Studio shows the same. I opened the fixture `nikon-z9-fx.jpg`, a
  JPEG of my Z 9, in Nikon's NX Studio. Its bytes 52 to 54 are
  `02 81 01`, and its bytes 57, 59 and 61 are `94 8E 88`. NX Studio
  shows the Picture Control VIVID with Quick sharp +1, Sharpening +5,
  Mid-range sharpening +3.5 and Clarity +2. It shows 0 for contrast,
  brightness, saturation and hue. **[F]**

So raw-files reads the adjustment kind at byte 52 and Quick sharp at
byte 53. NX Studio shows Quick sharp there on that one file, and
with it the quarter steps of sharpening, mid-range sharpening and
clarity. It does not show the adjustment kind. Byte 52 rests on the
files, and on its place before quick adjust in all three layouts [I].

**Quick adjust and Quick sharp are two settings.** The D600's and the
D7200's Quick adjust mutes or heightens the whole Picture Control
[P][25] (p. 132) [28] (p. 133). The Z 9's Quick sharp balances the
three sharpening values. Its guide gives the older description to
another setting, the effect level of the Creative Picture Controls
[P][26] (p. 181). The documentation names the field quick adjust in
all three layouts [S][1]. raw-files gives each under Nikon's word:
quick adjust from the first and second layouts, Quick sharp from the
third. Bytes 55 and 56 of the third layout hold `FF 0A` on every file.
They may hold the effect level [I], and are not read.

**Reading a value.** A value is one byte in all three layouts.
**[F]**

- **A count of steps** is the byte less 0x80, over its divisor. The
  first layout records no divisor, and counts whole steps. In the
  second and third the divisor is 4, so a step can be a quarter. The
  divisor of quick adjust and of Quick sharp is 1. The Z 9 changes a
  value by 0.25 with its sub-command dial [P][26] (p. 180).
- **0x00 is "auto".** It stands only in sharpening, contrast and
  saturation, and in clarity in the second layout. Those are the
  settings that Nikon's manuals offer an "A" for [P][25] (p. 132) [28]
  (p. 133). No 108-byte record holds it. There raw-files reads it as
  auto where the Z 9 offers an "A": Quick sharp, contrast and
  saturation, and not sharpening or clarity [P][26] (pp. 181 to 183).
- **0xFF is "not available"**, and gives `None`. It stands:
  - in quick adjust on every file under `NEUTRAL`, `FLAT`, `MONOCHROME`
    and a typed name, where the manuals say it is not available
    [P][25] (p. 132) [28] (p. 133);
  - in quick adjust on 11,327 files under the other controls, where
    11,778 hold 0;
  - in Quick sharp under a typed name (6 public files);
  - in the filter effect and the toning on every file but my one
    `MONOCHROME` file, and in the toning's saturation on every file;
  - in every value of the `AUTO` control, in the second and third
    layouts;
  - in brightness and hue on the 12 Coolpix records, and in
    sharpening, contrast and saturation on three of them.

| Field | Values the files hold | Read as | Mark |
|---|---|---|---|
| Adjustment kind | 0 on 16,147, 12,067 and 13,109 files of the three layouts; 2 on 2,374, 794 and 4,184; 1 on 15, all JPEGs of my Z 9 | the documentation's 0 default settings, 1 quick adjust, 2 full control [S][1] | observed; 1 in my files only |
| Quick adjust | 0, or not available | steps | observed, for 0 alone |
| Quick sharp | 0, +1 and +2, or not available | steps, or auto | observed; NX Studio shows the +1 of one file |
| Sharpening | 1 to 8 in the first layout, 0 to 7 in the second, 1 to 9 in the third; auto in the first two | steps, or auto in the first two layouts | observed; NX Studio shows the +5 of one file of the third layout |
| Mid-range sharpening | 0 to 4.5 | steps | observed; NX Studio shows the +3.5 of one file |
| Clarity | 0 and 1 in the second layout, 0 to 3 in the third; auto in the second | steps, or auto in the second layout | observed; NX Studio shows the +2 of one file of the third layout |
| Contrast | -1 to 3; auto in the first two layouts | steps, or auto | observed |
| Brightness | -1 to 1 | steps | observed |
| Saturation | 0 to 3; auto in the first two layouts | steps, or auto | observed |
| Hue | 0 on every file | steps | observed, for 0 alone |
| Filter effect | 0x80 on my one `MONOCHROME` file; else not available | 0x80 off, 0x81 yellow, 0x82 orange, 0x83 red, 0x84 green [S][1] | 0x80 observed, in my files only; the rest documented only |
| Toning | 0x80 on the same file; else not available | 0x80 B&W, 0x81 sepia, 0x82 cyanotype, 0x83 to 0x89 red, yellow, green, blue green, blue, purple blue and red purple [S][1] | 0x80 observed, in my files only; the rest documented only |
| Toning saturation | not available on every file | steps | documented only |

**[F]** The filter effects and the tonings are the D600 manual's own
lists [P][25] (p. 132).

**The name and the base.** Each is 20 bytes of ASCII, ended by a NUL.
The base is the preset a control was made from. raw-files gives both
as recorded, cleaned as every text, and keeps no list of names.

| Name | Files, public / mine |
|---|---|
| `STANDARD` | 151 / 20,381 |
| `AUTO` | 67 / 13,009 |
| `NEUTRAL` | 7 / 8,030 |
| `VIVID` | 3 / 5,576 |
| `PORTRAIT` | 8 / 1,116 |
| `LANDSCAPE` | 8 / 86 |
| `FLAT` | 10 / 0 |
| `MONOCHROME` | 0 / 1 |
| a name its owner typed | 8 / 229 |

**[F]** The counts take both tags. The typed names are `STANDARD-02`
on a D5300 and `VIVID-KR` on a Z 6II, with the bases `STANDARD` and
`VIVID`. Mine are 229 files of one name, with the base `VIVID`. On
every other file the name equals the base.

**What the record holds beside these.** None of it is read:

- **The control's number.** Of the four bytes before the adjustment
  kind, two hold a number for each preset, in the MakerNote's byte
  order. It is 0x0001 under `STANDARD`, 0x00C3 under `VIVID` and 0x02CF
  under `AUTO`. A third byte is 1 under a typed name. The documentation
  does not name them. **[F]**
- **Bytes 75 to 78** of versions 0302 and 0310: two values with a
  divisor of 4, both 0xFF on every file. **[F]**
- **Bytes 90 to 99** of the third layout: five values with a divisor
  of 1. They hold a step under `AUTO` and 0xFF under every other
  control, where the documented values hold the reverse. They may be the `AUTO`
  control's own adjustments, which the Z 9 sets from A-2 to A+2 [P][26]
  (p. 183) [I].

**Active D-Lighting.** It keeps detail in highlights and shadows, by
adjusting the exposure before the shot [P][25] (pp. 137 to 138).
0x0022 ActiveD-Lighting is one SHORT [S][1], on 263 public files of 76
bodies and on all 48,428 of mine. raw-files reads it as the
`ActiveDLighting` of a `DynamicRangeOptimization`:

| Code | The documentation's name [S][1] | Mark | Files, public / mine |
|---|---|---|---|
| 0 | Off | observed | 161 / 8,683 |
| 1 | Low | observed | 2 / 0 |
| 3 | Normal | observed | 18 / 0 |
| 5 | High | observed | 7 / 269 |
| 7 | Extra High | documented only | 0 / 0 |
| 8 to 11 | Extra High 1 to Extra High 4 | documented only | 0 / 0 |
| 65535 | Auto | observed | 75 / 39,476 |

**[F]** The D600 offers auto, extra high, high, normal, low and off
[P][25] (p. 137). No manual read for this section names the four
numbered steps. The list is read whole, since files hold part of it.

**HDR.** The camera combines two exposures into one JPEG, and cannot
record a NEF under it [P][25] (pp. 139 to 141). 0x0035 HDRInfo is a
record that starts with a four-digit version [S][1]. raw-files reads
it as the `Hdr` of a `HighDynamicRange`, on or off with its parts:

| Version | Length | Byte 4 | The parts | Files, public / mine |
|---|---|---|---|---|
| `0100` | 8 | 0 off, 1 on, 48 auto | byte 5 the exposure differential: 0 auto, 1 to 3 that many EV; byte 6 the smoothing: 0 off, 1 normal, 2 low, 3 high, 48 auto | 1 / 14 |
| `0101` | 8 | the same | the same | 9 / 0 |
| `0200` | 6 | 0 off, 1 on | byte 5 the strength: 0 not available, 1 normal, 2 low, 3 high, 4 high+, 5 auto | 0 / 7,827 |

**[F]** The codes' meanings are the documentation's [S][1].

- **Where it is.** The public files are a Nikon 1 J2's raw file at 0100
  and nine Coolpix NRW files at 0101. Mine are JPEGs: 14 of my D600 at
  0100, and 7,827 of my D7100, D7200 and Z 9 at 0200. No NEF of mine
  holds the record.
- **On or off.** HDR is on on four files, all JPEGs of my D7100, whose
  bytes 4 and 5 are `01 05`: on, with the strength auto. It is off on
  every other file. So 1 is observed in my files only, and 0 on both
  sets.
- **The parts are Nikon's words.** The D600 sets an exposure
  differential and a smoothing [P][25] (pp. 140 to 141). The D7100
  sets an HDR strength of auto, extra high, high, normal or low
  [P][27] (p. 116). The documentation's "high+" is that manual's extra
  high.
- **The parts are read only where HDR is on**, as the mode of
  vibration reduction is. With HDR off, my D600's files hold 0 for
  both parts, which reads as a smoothing of off, a step its menu does
  not offer.
- **Byte 4's 48, "auto"**, names neither on nor off. No file holds it,
  and it gives no value: `Hdr` is on or off, so the code stays a named
  gap, though the rest of its list is read.
- **Byte 7** of the 8-byte versions, a second level in the
  documentation, is not read. It is 0xFF on my D600's files and 0
  on the public ones.
- **Any other record gives none.** One public file, a COOLPIX P7100's,
  holds eight bytes with no version.

**High ISO noise reduction.** 0x00B1 HighISONoiseReduction is one
SHORT [S][1], on 270 public files of 80 bodies and on all 48,428 of
mine. raw-files will read it as a `HighIsoNoiseReduction`, one list of
steps for both makers:

| Code | The documentation's name [S][1] | Mark | Files, public / mine |
|---|---|---|---|
| 0 | Off | observed | 97 / 14,164 |
| 1 | Minimal | observed | 2 / 1,927 |
| 2 | Low | observed | 22 / 0 |
| 3 | Medium Low | documented only | 0 / 0 |
| 4 | Normal | observed | 149 / 31,629 |
| 5 | Medium High | documented only | 0 / 0 |
| 6 | High | observed, in my files only | 0 / 708 |

**[F]** The D600's menu offers high, normal, low and off. Under off
the camera still reduces noise at high sensitivities, by less than
under low [P][25] (p. 218). Code 1 is on my D3200, D600 and D7100,
beside code 0 on the same bodies. What sets the two apart is open.

**Long exposure noise reduction.** With the setting on, the camera
takes a dark frame after an exposure slower than 1 s and subtracts it
[P][25] (p. 218). 0x0095 NoiseReduction is ASCII, five bytes with the
NUL [S][1]. It is on 283 public files of 92 bodies and on all 48,428
of mine. The documentation lists no values. The files hold two texts,
counted here by the exposure time, Exif's ExposureTime:

| Text | Under 1 s | Exactly 1 s | Over 1 s |
|---|---|---|---|
| `FPNR`, mine | 0 | 2 | 264 |
| `OFF`, mine | 48,079 | 12 | 71 |
| `FPNR`, public | 0 | 0 | 1 |
| `OFF`, public | 269 | 11 | 2 |

**[F]** Mine are the D3200's 250 `FPNR` files and the D7200's 16. The
public one is a COOLPIX P7000's, at 4.81 s.

- **The text says whether the reduction ran**, not how the camera was
  set. On my D3200 every exposure over 1 s says `FPNR` (248 files), and
  every shorter one says `OFF` (10,139). A setting does not change
  with each exposure.
- **So `FPNR` will give on, and `OFF` will give `None`.** `FPNR` shows
  that the setting was on. `OFF` shows nothing below 1 s. Above it,
  `OFF` can also stand with the setting on: a camera switched off
  while it processes saves the picture without the reduction [P][25]
  (p. 218).
- **Exif's CustomRendered follows it on two bodies.** On my D3200 and
  D7200 it is 1 on exactly the `FPNR` files. On my Z 9 it is 1 on
  every file, and on the public P7000's file it is 0.
- Both texts are observed. Any other text will give `None`.

**Vignette control.** It reduces the drop in brightness at a picture's
edges [P][25] (p. 218). 0x002A VignetteControl is one SHORT [S][1], on
199 public files of 38 bodies and on 32,584 of mine, of the D600, D7200
and Z 9. raw-files will read it as the `VignetteControl` of a
`VignettingCorrection`:

| Code | The documentation's name [S][1] | Mark | Files, public / mine |
|---|---|---|---|
| 0 | Off | observed | 45 / 8,441 |
| 1 | Low | observed | 6 / 0 |
| 3 | Normal | observed | 148 / 24,143 |
| 5 | High | documented only | 0 / 0 |

**[F]** The four steps are the D600's menu [P][25] (p. 218). My D300S,
D3200, D3300 and D7100 write no 0x002A, and will give none.

**Auto distortion control.** It reduces barrel and pin-cushion
distortion [P][25] (p. 217). 0x002B DistortInfo is a record of 16
bytes at version `0100`, whose byte 4 is the setting [S][1]. It is on
219 public files and 46,747 of mine. raw-files will read it as the
`AutoDistortionControl` of a `DistortionCorrection`:

| Byte 4 | The documentation's name [S][1] | Mark | Files, public / mine |
|---|---|---|---|
| 0 | Off | observed | 103 / 19,308 |
| 1 | On | observed | 116 / 27,439 |
| 2 | On (underwater) | documented only | 0 / 0 |

**[F]** Bytes 5 to 7 vary with the lens and are not documented. A
record of another shape will give none. Two files hold one: a public
D3000's holds one SHORT, and one JPEG of mine a text that an editor
left. My D300S writes no 0x002B.

**The scene mode.** In a scene mode the camera sets itself for a kind
of picture, such as a portrait or a sport [P][25] (p. 40). The DSLR and
Z bodies record the mode as text in 0x00AB, which the documentation
calls VariProgram. The Coolpix bodies record it in 0x008F SceneMode
[S][1]. Each is ASCII, 16 bytes with the NUL, padded with spaces. No
measured file holds both. raw-files will read 0x00AB, else 0x008F, as
the scene mode of a `Scene`: the text as recorded, cleaned as every
text.

| Text | Files, public / mine |
|---|---|
| blank | 164 / 35,428 |
| `AUTO(FLASH OFF)` | 1 / 5,811 |
| `AUTO` | 26 / 5,146 |
| `CLOSE UP` | 0 / 107 |
| `SCENE AUTO` | 1 / 82 |
| `LANDSCAPE` | 1 / 77 |
| `SPORT` | 0 / 67 |
| `PORTRAIT` | 0 / 16 |
| `CHILD` | 4 / 0 |
| 13 more texts, of my D3300's scenes and effects | 0 / 14 |

**[F]** The counts take both tags: 197 public files and 46,748 of
mine.

- **A blank text will give `None`.** It is blank under the program,
  priority and manual modes.
- **The text is set exactly where Exif names no program.** On my
  files, ExposureProgram is 0 on every file whose text is set, and 1
  to 4 on every file whose text is blank.
- **The auto modes will be given too.** The D600's mode dial has an
  auto mode and an auto mode with the flash off beside its scene
  position [P][25] (pp. 35 and 40). The files hold them as `AUTO` and
  `AUTO(FLASH OFF)`.
- **No list of texts enters raw-files.** A camera's effects are
  recorded here too, such as `TOY CAMERA` on my D3300, and a later
  body writes texts no file here holds.
- My D300S writes neither tag, and will give no scene.

**Exif's six values.** Each is one SHORT in the Exif IFD, with the
standard's codes [P][23] (4.6.6.7.35, p. 69; 4.6.6.7.40 to 4.6.6.7.44,
pp. 70 to 71). The
[JPEG notes](../jpeg/container-and-metadata.md#4-exif-the-app1-segment)
give the codes. raw-files reads them from Exif alone, in a raw file as
in a JPEG. They are on 279 public files and on 48,615 of mine;
CustomRendered is on 48,561 of mine.

| Tag | Values, public | Values, mine |
|---|---|---|
| 0xA408 Contrast | 0 on 274, 1 on 3, 2 on 2 | 0 on 48,613, 2 on 2 |
| 0xA409 Saturation | 0 on 272, 2 on 7 | 0 on 48,611, 2 on 4 |
| 0xA40A Sharpness | 0 on 245, 2 on 20, 1 on 14 | 0 on 41,157, 2 on 7,458 |
| 0xA406 SceneCaptureType | 0 on 263, 1 on 8, 2 on 8 | 0 on 47,671, 2 on 858, 1 on 86 |
| 0xA401 CustomRendered | 0 on 182, 1 on 97 | 0 on 31,075, 1 on 17,486 |
| 0xA407 GainControl | 0 on 237, 1 on 28, 2 on 14 | 0 on 31,521, 1 on 10,197, 2 on 6,897 |

**[F]**

- **SceneCaptureType follows the Picture Control on the public
  files.** It is 1, landscape, on the eight files whose control is
  `LANDSCAPE`, and 2, portrait, on the eight whose control is
  `PORTRAIT`. Each was shot with a program set.
- **CustomRendered marks different things by body.** It is 1 on every
  file of my Z 9, and on my D3200 and D7200 only where the long
  exposure text is `FPNR`.
- No Nikon file holds a code that the standard reserves.

**A raw file and its JPEG agree.** 1,887 shots of mine have one NEF
and one camera JPEG. They are 933 of my D7200, 633 of my D3200, 315 of
my Z 9 and 6 of my D600. A shot is a file name and a capture time that both
files share. Every value of this section is the same in the two files,
with two exceptions **[F]**:

- the HDR record, which only the JPEG holds, on the D600, D7200 and
  Z 9;
- 15 shots of my Z 9 whose JPEG a program wrote [I]. Six of those
  JPEGs hold none of the MakerNote tags. Nine hold a scene tag of two
  bytes, and five of the nine another Picture Control than the NEF.

## 9. Robustness pitfalls

The raw-files rule is that a malformed file returns an error and never panics.
The points specific to the MakerNote:

1. **Relocated MakerNotes.** Tools that move the MakerNote without fixing
   offsets leave the headerless and short layouts pointing at the wrong
   place. ExifTool can recognize such a MakerNote, and repairs its offsets
   only on request. Bounds-check every offset and length against the
   MakerNote and the file, skip bad entries rather than abort the IFD,
   and do not repair in a first version. [S][4][19] An E885 JPEG that an
   editor turned upright is such a file: its headerless MakerNote moved
   342 bytes, and its offsets did not ([section 1](#layout-details)).
   **[F]** raw-files requires the PreviewIFD's JPEG, as declared, to lie
   inside the MakerNote too. The Coolpix P7800 declares its MakerNote
   2,816 bytes long, ending at byte 4546, and its 640x480 MakerNote JPEG
   starts at byte 4546, just past it, so the reader does not read it and
   the file gives its 160x120 and 4000x3000 previews (file 1425); on the
   E5700 and the D100, the other `02 00` bodies, the declared length
   includes the JPEG (files 4456, 2996). **[F]** Early Coolpix JPEGs with
   a headerless MakerNote go further: the E885, E995, E2500 and E5000, on
   their earlier firmware, put their PreviewIFD's 320x240 JPEG 12,527
   bytes past the MakerNote's end, inside the Exif segment, so the reader
   does not read it either ([section 1](#in-a-jpeg-file)). **[F]** The rule
   was weighed against the ten public NEF and NRW files whose preview
   lies outside the MakerNote, and kept
   ([container-and-previews.md](container-and-previews.md#21-where-the-makernotes-preview-lies)).
2. **Truncated MakerNotes.** DNG-converted NEFs can cut the MakerNote short
   (reported as "bad offset" errors on later tags). [S][21]
3. **Capture NX.** Foreign brands, changed byte order and fixed-order
   sub-blocks ([section 1](#1-finding-the-makernote)). [S][3][4]
4. **Inner versus outer byte order.** Always take the inner header's order.
   rawler gives an error for a file whose MakerNote byte order differs
   from the TIFF's, yet such files exist ([section 1](#layout-details)).
   Tolerate them for metadata, reject them for decoding. [S][I]
5. **Type and count mismatches.** Tag 0x0001 is a string on some bodies and
   binary on others, 0x0002 writes four zero bytes for "Lo" ISO, and the
   NEFBitDepth and ImageBoundary types vary by model. Read from the raw
   bytes at the stated count; cap counts. [S][3]
6. **Loops and sizes.** Cap the MakerNote at 1000 entries; no checked
   MakerNote has more than 84 ([section 12.1](#121-the-directories)).
   **[F]** Skip an entry whose declared length is implausible for the
   file. Keep a visited-offset set for PreviewIFD and SubIFD recursion.
   Cap each allocation at the bytes actually remaining. LibRaw likewise
   reads at most 1000 entries of a MakerNote and none of implausible
   length. [S][7]
7. **Version-gated decryption.** Decrypt only when the version prefix, the
   minimum length and both key inputs are present; otherwise skip and
   continue. Never unwrap. [I]
8. **Odd bodies.** D1 (big-endian headerless MakerNote); D100 (an
   uncompressed layout of its own, NEFCompression 2 with a conflicting
   TIFF tag); D200
   (no next-IFD pointer); Coolpix NRW files (no 0x0096); D3 and D810
   (14-bit versus 16-bit and mislabeled uncompressed data); D780, Z 6 and
   Z 7 (curve header 0x44 0x40, bit depth two bits lower than labeled);
   Z bodies (compression in 0x0051, at byte 10 on the Z 9 and later,
   but in 0x0093 on the Z fc and Z 30, which write no 0x0051);
   Z 9 and later (HE and HE* undecodable).
   [S][1][3][8][9]
9. **Nikon Scan files.** Scanner "NEFs" are TIFF RGB with a Nikon Scan IFD
   (tag 0x0E10); reject anything whose raw IFD is not CFA
   (PhotometricInterpretation 32803). [S][I][3]

## 10. Recommendation for raw-files

raw-files decided this in
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md),
which follows the recommendation below with these changes. From the
samples: the MakerNote also gives the serial number, the ISO and the lens
range when the Exif tags lack them
([section 8.5](#85-serial-number-iso-and-lens)), and WorldTime gives the
offset from UTC as a plain offset, not a labeled home zone
([section 8.2](#82-capture-date-and-time-zone)). And by choice: text is
trimmed of spaces at both ends, not only trailing ones as in
[section 8.3](#83-make-and-model), and a value over 255 bytes is taken
for damage; and CropArea is used without the 0x001B cross-check of
item 2. raw-files reads each of the three layouts that items 2 and 6
cover. The D1 family gets its MakerNote preview, its ISO and its lens
range from the headerless MakerNote (file 5364). The earliest Coolpix
bodies get their short-header MakerNote's entries in the raw view, and
the Exif fields only. The short header is read only in a little-endian
structure ([section 1](#in-a-jpeg-file)). Every Nikon MakerNote is read only
under a Make that starts with `NIKON`, whatever its first bytes: the
format's reader decides by IFD0's Make before the MakerNote is looked
at, unlike the "Check the signature before Make" of section 1, so a
Nikon-format MakerNote that Capture NX wrote into another brand's file
is not read.

The focus point is decided in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md):
the MakerNote's AFInfo2 gives it where it records a position, at the
versions of [section 8.6](#86-focus-point), scaled onto CropArea on a
DX crop and moved into the window on an aspect crop; a FocusMode of
`MANUAL` gives none, and a point number gives none until raw-files has
a table of the body's points. The AF-area mode is a camera setting,
given with or without a point, from AFInfo2 at every version of section
8.6 and from AFInfo's first byte, by the table that ends that section.

For the camera settings of [section 8.7](#87-camera-settings),
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
decides what this list recommends: read the Exif tags first and the
plain MakerNote tags where Exif has no value, and decrypt nothing [I]:

- the exposure program, the metering mode and the focal length in
  35 mm terms from Exif alone. Every body that lacks them in Exif lacks
  them in plain MakerNote tags too;
- the flash from Exif's Flash bits, with red-eye reduction and the
  return light. Three plain MakerNote tags add the rest: the source and
  control from 0x0009's text, the sync from 0x0008's, the compensation
  from 0x0012; 0x0017, a flash unit's own setting, is not read. A file
  without Exif's Flash gives no flash.
  So the D1 family gives none, whose 0x0087 says "did not fire" even on
  a shot whose white balance is set to flash. Neither 0x0087 nor
  FlashInfo is read
  ([section 8.10](#810-more-capture-settings));
- the white balance from 0x0005, which every body writes and which is
  finer than Exif's auto or manual, by a table of the texts files are
  known to hold, since a text like `COOL WHT FL` cannot be guessed,
  else from Exif's WhiteBalance and LightSource;
- the color temperature from 0x004F when it is above zero, else, under
  a chosen color temperature, from the number of 0x0005's text, such
  as `3230K`, which is all the bodies without 0x004F record of a
  temperature; either only up to 99,999 K, the five digits of that
  text;
- the fine-tuning from 0x003F, else from 0x000B;
- the shutter from 0x0034;
- the image area from CropHiSpeed's first value. 0 does not say which
  sensor the body has. Where the code is 0 or the tag is missing, the
  body's sensor format comes from raw-files' table of bodies
  ([camera-names.md](../camera-names.md)). A body the table lacks gives
  none, and so does a tag that breaks its rule or a MakerNote with an
  entry too damaged to read, which may be the tag. 4 (3:2) and the movie
  codes give none, but 9 and 10, which DX bodies write on stills. 9 is
  the APS-C 16:9 crop, and 10 the 1.3x crop;
- the raw image size from 0x003E, else large where the table says the
  body offers no raw sizes;
- the color space from Exif whenever it records ColorSpace, 65535
  without `R03` included, and from 0x001E only when it records none,
  since a program that rewrites a JPEG can leave the camera's 0x001E
  behind;
- the software from IFD0;
- the further settings of [section 8.10](#810-more-capture-settings):
  - the focus mode from 0x0007, and AF fine-tune from 0x00B9;
  - stabilization from 0x001F, at its three versions;
  - auto ISO from 0x0089, else from 0x000F;
  - the exposure fine-tuning from 0x001C;
  - the release mode and bracketing from 0x0089, and Exif's
    ExposureMode;
  - daylight saving time from WorldTime;
- the raw compression from the raw IFD's Compression, refined by
  0x0093 or byte 10 of 0x0051, and the raw bit depth from its
  BitsPerSample;
- the JPEG quality from 0x0004 in a JPEG, a `RAW` there giving none,
  and the size priority or optimal quality from 0x0044;
- the white balance's preset slot from the number after `PRESET`, and
  its fluorescent lamp from Exif's LightSource 12 to 16;
- contrast, saturation, sharpness, the scene capture type, custom
  rendering and gain control from Exif alone;
- the picture style, the Picture Control, from 0x0023, else 0x00BD,
  at its six versions. In the 108-byte record the adjustment kind and
  Quick sharp lie at bytes 52 and 53;
- Active D-Lighting from 0x0022;
- HDR from 0x0035 at its three versions, its parts only where it is
  on;
- the other processing settings of
  [section 8.11](#811-picture-style-and-processing), which raw-files
  will read:
  - the high ISO noise reduction from 0x00B1;
  - the long exposure noise reduction from 0x0095, where `FPNR` will
    give on and `OFF` none;
  - vignette control from 0x002A, and auto distortion control from
    byte 4 of 0x002B;
  - the scene mode from 0x00AB, else 0x008F, where a blank text will
    give none.

ShotInfo's firmware version and image area, NikonSettings, and the
digits that 0x0051 and 0x0056 start with stay unread: the first two are
encrypted, and the image area contradicts CropHiSpeed on two of the four
files ExifTool decodes it on; NikonSettings repeats the setting that
0x0034 records; and 0x0051 and 0x0056 do not hold the body's firmware.

For the metadata a viewer needs, the research recommended keeping the
MakerNote out of the critical path:

1. **Fields from standard tags only:** orientation, capture date, sub-seconds,
   offset from UTC, make and model, and the stored sensor size all come from
   IFD0, its SubIFDs and the Exif IFD (IFD0's ImageWidth is the thumbnail;
   the sensor size is in the raw SubIFD). This works for every NEF regardless of how the
   MakerNote is laid out. [S]
2. **Parse the MakerNote only for two things:** the crop rectangle (0x0045,
   with 0x001B as a cross-check) and, optionally, WorldTime (0x0024) as a
   time zone fallback. Implement the byte-pattern detection in
   [section 1](#1-finding-the-makernote), bounds-check every read, and
   return "absent" rather than an error for a MakerNote that does not parse.
   [I]
3. **Do not implement decryption for the plain metadata.** The fields
   above need no key, the substitution tables have no settled provenance
   yet ([section 11](#11-open-questions)), and white balance for every
   body of interest is available in plain form (0x000C). Defer 0x0097,
   0x0098 and 0x0091 to full metadata, which reads what they hold,
   0x0097's white balance multipliers among it, once a decision on the
   tables allows it; raw decoding uses those multipliers, where older
   bodies record them only there. [I]
4. **Output size policy:** report the CropArea rectangle if present and
   plausible (inside the raw IFD dimensions); otherwise report the stored
   raw IFD dimensions and say in the API documentation that they may include
   border pixels. Do not guess from Exif PixelXDimension. [I]
5. **Time policy:** prefer 0x9011 when it parses; otherwise offer the
   WorldTime zone clearly labeled as a home-zone fallback, or none. [I]
6. **Tests:** cover all three layouts, an inner byte order different from
   the outer one, a truncated MakerNote, an unset clock, a 0x0045 outside the
   raw dimensions, and NUL-terminated and space-padded Make/Model strings.
   Use files from raw.pixls.us (check each file's license) for the main
   cases. [I]

## 11. Open questions

1. **Provenance of the decryption tables.** The two 256-byte tables are
   published in GPL, LGPL, CDDL and Artistic projects and in dcraw, whose
   header states that all of its code outside a single restricted (Foveon)
   block is free for all uses; dcraw's license is nevertheless described as
   ambiguous for the whole file elsewhere on this page. That is an input to
   the decision, not legal advice. raw-files needs an independent derivation,
   a source it can use, or a recorded legal decision, before any
   capability touches encrypted blocks, full metadata the first. [10]
2. **Non-numeric serial handling.** Which of the four conventions in
   [section 5.1](#51-non-numeric-serial-numbers) is correct for a real
   lettered serial. Checking a D50-family and a D200 sample would settle it.
3. **ColorBalance version 08xx (Z bodies).** Whether any Z body lacks
   0x000C, and whether the encrypted block holds anything useful.
4. **BlackLevel (0x003D) channel order.** Whether the four values are in
   the fixed color order R, G1, G2, B or in CFA position order is not
   established. LibRaw gives them in the first order, rawspeed and rawler
   in the second. The two give the same black levels only on an RGGB
   file. In rawspeed's camera list every non-RGGB Nikon is a
   pre-2010 body, so there may be no practical difference, but which body
   first wrote the tag is unknown. [S][7][8][9][17]
5. **Bodies that write OffsetTimeOriginal.** Answered: the D850 and every
   later checked body. WorldTime agrees with it wherever both exist, and
   is taken to give the offset on the older bodies from the D300 on,
   where it cannot be compared ([section 8.2](#82-capture-date-and-time-zone)).
   **[F]**
6. **Make/Model padding on recent bodies.** Answered: none of the checked
   bodies pads either, except the D100's Model
   ([section 8.3](#83-make-and-model)). **[F]**
7. **Presence of CropArea (0x0045).** Answered for the checked bodies: the
   D5 (2016, file 2218), D850, D780 and Z bodies write it; the D810 and
   older do not, nor does the D5600 ([section 8.4](#84-image-dimensions)).
   Which body between the D810 and the D5 (2014 to 2016) first wrote it
   is still open. **[F]**
8. **Outer byte order of recent bodies.** Answered: `II`, file and
   MakerNote alike, on the D750, D810, D850, D780, Z 6, Z f, Z6III and Z 9,
   and on the D5600, Z5II, Z50II, D5, Z 30 and Z fc and the Z6III's HE\*
   files (files 1415, 7735, 7736, 7767, 7770, 2218, 5812, 8559, 7812,
   7813);
   `MM` on the DSLRs from the D1H to the D7000 and on the Coolpix A (2013);
   `II` on the Coolpix E5700 and P7800 and the Nikon 1 J1. The MakerNote's
   order equals the file's in every checked file. Readers must accept both
   `II` and `MM`. **[F]** [S][11]
9. **Bodies that write NEFCompression 5 and 6.** Labels are known, bodies
    are not.
10. **The 0101 position.** Answered: ExifTool's offsets (70 to 82) hold
    the frame, the center, the area and the in-focus byte on the D5's
    live-view file: 5568x3712, (3077, 2427), 588x500, in focus (file 2218)
    ([section 8.6](#86-focus-point)). **[F]**
11. **Version 0301.** Answered: it has the 03xx layout, 60 bytes with the
    position at the 0300 offsets, on the Z 30 and the Z fc (files 5812,
    8559). **[F]**
12. **A DSLR in live view on a crop.** Answered for the D7200: AFInfo2's
    frame is the cropped output, 4800x2696 in its 1.3x movie crop, on
    one of my files ([section 8.6](#86-focus-point)). **[F]** Whether a DX
    or 1.2x crop on a D5, D750, D810 or D850 does the same, and whether a
    CropArea then counts the crop, is open; no public file shows one.
13. **Manual focus on a Z body.** Answered: the Z fc writes a position
    in AFInfo2 under FocusMode `MANUAL`, center (2520, 1856) and area
    288x319 in its 5568x3712 frame (file 8559). raw-files gives no focus
    point for it, as for the two DSLR viewfinder shots in manual focus
    (files 928, 1835). **[F]**
14. **A rotated shot.** Answered: the Z 30's portrait shot (Orientation
    8) records its frame in stored, landscape orientation, 5568x3712
    (file 5812). **[F]**
15. **CropHiSpeed's sizes on uncropped files.** The D810 writes mode 11,
    FX uncropped, with a cropped width four pixels short of the original
    (file 2036). Whether other bodies do the same, and so whether a crop
    is better told by the mode code, or by a difference larger than a
    border, than by unequal sizes.
16. **Tables of autofocus points.** Turning a point number or mask into a
    position needs a table of each body's points. The D600's is
    measured on frames shot with my own body, with the bit order of
    its mask ([autofocus-points.md](autofocus-points.md)) **[F]**;
    every other body's waits for a session on that body by the same
    procedure. ExifTool's published tables give the order of the
    points by number and, for the 11-point layout only, the bit order of
    the mask. [S][1]
17. **FaceDetect.** Which bodies write 0x0021; none of the 35 checked
    files does.
18. **Crops that cut both sides and change the aspect**, such as a DX
    crop at 1:1 or 16:9 on an FX body: whether they are scaled like DX or
    windows like 1:1. The rule scales them; no sample shows one.
19. **Where an aspect crop's window lies.** raw-files centers it, at
    half the difference between the recorded frame and CropArea.
    CropHiSpeed's own offsets, in stored pixels, differ from that by up to
    eight pixels (936 against 928 on the Z50II's 1:1 file, 312 against
    316 on the Z6III's 16:9 file); an off-center shot near an edge of the
    window, on one of those bodies, would show which is right. On the
    D600, a body without a CropArea, the point placed from the table on
    a DX frame shifts by CropHiSpeed's origin, (1040, 696), which is the
    centered window's origin to the pixel
    ([autofocus-points.md](autofocus-points.md#26-the-control-frames)).
    **[F]**
20. **A crop on a small raw.** Whether a Z body records a crop mode on a
    small raw, and how. The rule gives a 1:1 or 16:9 crop on one no focus
    point, since its CropArea counts stored pixels while AFInfo2's frame
    counts full-size ones, and scales a DX crop onto that CropArea.
21. **The other white balance texts.** Mostly answered: the public
    files show 0x0005 as `AUTO`, `AUTO0`, `AUTO1`, `SUNNY`, `FLASH` and
    `SHADE`, my own files add `CLOUDY`, `INCANDESCENT`, `PRESET` and
    `COOL WHT FL`, and frames shot on a Z 9 and a D600 add `AUTO2`,
    `NATURAL AUTO`, `PRESET1` to `PRESET4` and a chosen color
    temperature as its number, such as `3230K`
    ([section 8.7](#87-camera-settings)). **[F]** Still on no checked
    file: the other fluorescent lamps, the Z 9's preset slots 5 and 6,
    which `PRESET` and its number reads by its form
    ([section 8.10](#810-more-capture-settings)), and whatever the bodies
    not checked write.
22. **The color temperature under a manual setting.** Answered for the
    Z 9: it holds a temperature under every setting, 3230 under a chosen
    3230 K and 5200 under a preset; the D850 and the Z fc write 0 under
    `SUNNY` ([section 8.7](#87-camera-settings)). **[F]**
23. **The axes of the fine-tuning.** Answered for the two values of
    0x003F and 0x000B: the first is amber (negative) to blue (positive),
    the second magenta (negative) to green (positive), as Nikon's NX
    Studio names the shifts on frames shot on a Z 9 and a D600 with a
    shift along each axis, and as their pictures show
    ([section 8.7](#87-camera-settings)). **[F]** Both count the
    menu's steps, 0x003F in quarters. Still open: the axis and sign of
    the single value the D1 family, the D70, the D100 and the D200
    write, and how large a menu step is.
24. **The shutter of the older DSLRs.** No DSLR older than the D850
    writes 0x0034, nor any of my six DSLRs, and none records the
    shutter in a block ExifTool decodes. Whether a DSLR in live view
    records it anywhere is open; so is which bodies write the codes
    48, 64 and 96, which no checked file holds.
25. **CropHiSpeed's other codes.** 3 (5:4), 4 (3:2), 8 (2.7x) and the
    movie codes other than 9 and 10 are on no checked file; 1 (1.3x) and 10
    are only on my D7200 files, where 10, which ExifTool names "1.3x
    movie crop", is on still NEF files and JPEGs. 9, which ExifTool names
    for movies, is a 16:9 still on the Z50II (file 7770) and on one of
    my D7100 NEF files and one JPEG, so on a DX body it marks a 16:9
    still too.
26. **A medium raw.** 0x003E is 1 or 3 on the checked files; no medium
    raw was checked. The Z 9 makes none: its medium and small sizes are
    for the JPEG only.
27. **The D810's firmware.** Its Software says 1.12 and its encrypted
    ShotInfo, as ExifTool decodes it, 1.11d (file 2036). Which one a
    firmware update changes is open.
28. **What 0x0051 and 0x0056 number.** Their leading digits are not the
    body's firmware ([section 8.7](#87-camera-settings)).
29. **ExposureProgram 0.** The Z5II and Z6III files, the D5300 JPEG
    and many of my own D3200, D3300, D7100, D7200 and D600 files
    record "not defined"; whether another tag records the program or
    scene mode those shots used is open.
30. **The mask's bit order on the other layouts.** Measured on the
    39-point layout only, bit number − 1 from the least significant bit
    of the first byte, and documented by ExifTool for the 11-point
    [S][1]; the 51-point, the 153-point and the Nikon 1 layouts, and the
    Z bodies' masks, are unmeasured. Frames with known points on such a
    body would show it.
31. **The point fields of the 0101 and 03xx versions.** Their offsets
    are ExifTool's [S][1], checked for presence only: the D850 names
    point 1 at byte 68 and the D780 point 34 at byte 56 (files 1835,
    3827). What a 0101 body's mask of points in focus (byte 48) holds
    beside a valid point is on no checked file.

## 12. The directories a file holds

raw-files gives every entry of every TIFF directory it walks, as the
file records it
([ADR023](../../architecture-decisions/adr023-every-entry-a-file-records.md)):
IFD0, the SubIFDs, the Exif and GPS IFDs, and the MakerNote IFD and its
preview IFD, which the view calls `MakerNote` and `MakerNote 0x0011`
after the MakerNote tag that points to it. This section lists what NEF
files hold: the directories, how many entries each has and of which
types, the largest values, the entries a reader has to leave out, and
the encrypted records. The tag names the view gives are in
[the tag names](../tag-names.md).

The numbers were taken on 3 November 2026 with a walker of my own,
written from the TIFF 6.0 and Exif standards and these notes, and with
a counting program over raw-files' own `open`; no other reader was run.
They cover 11,759 NEF and NRW files: the 94 of the test fixtures and
`raw-samples/`, every one of the 289 NEF and NRW files of raw.pixls.us,
and, as counts only, 11,376 of my own: every NEF of my Z 9 (8,976) and
up to 500 each of my D300s, D3200, D600, D7100 and D7200. **[F]** In this
section, **[F]** marks a count over these files rather than a fact
of one named file.

### 12.1 The directories

| Directory | Files | Entries, min / median / max | Types seen |
|---|---|---|---|
| IFD0 | 11,759 | 18 / 22 / 28 | BYTE, ASCII, SHORT, LONG, RATIONAL, UNDEFINED |
| SubIFDs | 11,759, one to six each | 8 / 14 / 19 | BYTE, SHORT, LONG, RATIONAL, UNDEFINED |
| Exif | 11,759 | 2 / 41 / 41 | ASCII, SHORT, LONG, RATIONAL, UNDEFINED, SRATIONAL |
| MakerNote | 11,759 | 5 / 80 / 84 | BYTE, ASCII, SHORT, LONG, RATIONAL, UNDEFINED, SSHORT, SRATIONAL |
| MakerNote preview IFD (0x0011) | 11,755 | 5 / 7 / 7 | SHORT, LONG, RATIONAL |
| GPS | 11,670 | 1 / 1 / 13 | BYTE, ASCII, RATIONAL |

- **No NEF has an IFD1 or an Interoperability IFD**, and no SubIFD
  holds another: every SubIFD is one link from IFD0, and no directory
  lies more than three links from it (IFD0, Exif, MakerNote, preview
  IFD). No SubIFD, Exif or MakerNote next-IFD pointer leads anywhere.
  **[F]**
- **The MakerNote** is in the embedded-TIFF layout on 11,752 files and
  headerless on 7, all of them D1-family files; no NEF has the short-header layout
  ([section 1](#1-finding-the-makernote)). raw-files lists it in both
  layouts. **[F]**
- **The preview IFD** is on 11,755 files, but raw-files lists it on
  11,749: on 4 Coolpix NRW files (P6000, P7000, P7100, B700) and the 2
  COOLSCAN files, the whole IFD lies past the length the MakerNote
  declares. raw-files reads the MakerNote's IFDs and its JPEG only
  inside that length ([section 9](#9-robustness-pitfalls), item 1), so
  it gives no small preview there, and the view does not list the IFD
  ([12.3](#123-entries-a-reader-leaves-out)). **[F]**
- **The GPS IFD is on every one of my files**, on 263 of the 289
  raw.pixls.us files and on 31 of the 94 fixtures, most often with a
  single entry. raw-files walks it after IFD0's chain. **[F]**
- **No entry has the types SBYTE, SLONG, FLOAT, DOUBLE, IFD (13) or
  Exif's UTF-8 (129).** **[F]**
- **A file holds 228 entries** (the median; at most 238) **and about
  100 KB of values** (100,440 bytes, the median; at most 104,718), not
  counting the MakerNote entry itself, whose content is its own
  entries. **[F]**

### 12.2 The largest values

| Directory | Tag | Type | Largest | Median | Files with 256 bytes or more, public / mine |
|---|---|---|---|---|---|
| Exif | 0x927C MakerNote | UNDEFINED | 993,826 | 119,480 | 383 / 11,376 |
| MakerNote | 0x0014 | UNDEFINED | 66,596 | 452 | 13 / 0 |
| MakerNote | 0x0091 ShotInfo | UNDEFINED | 65,472 | 63,397 | 306 / 11,376 |
| IFD0 | 0x02BC (XMP) | BYTE | 34,816 | 32,768 | 227 / 10,876 |
| MakerNote | 0x00BC | UNDEFINED | 14,430 | 7,700 | 119 / 1,900 |
| MakerNote | 0x0100 | UNDEFINED | 10,493 | 10,477 | 5 / 0 |
| MakerNote | 0x0010 | UNDEFINED | 8,918 | 3,342 | 11 / 0 |
| IFD0 | 0x8773 (ICC profile) | UNDEFINED | 8,684 | 8,684 | 2 / 0 |
| MakerNote | 0x008C | UNDEFINED | 8,256 | 578 | 301 / 11,376 |
| SubIFD | 0x0111 StripOffsets | LONG (1,231 values) | 4,924 | 1,148 | 29 / 0 |

The ten largest values, sizes in bytes; StripByteCounts (0x0117) ties
with StripOffsets. **[F]**

- **The MakerNote entry is the largest value of all**, up to 993,826
  bytes on the Nikon 1 bodies, whose preview IFD's JPEG lies inside it;
  the view gives its content as its own entries, and its bytes too.
- **Apart from the MakerNote entry, only one value passes 64 KiB**, the
  66,596 bytes of tag 0x0014 on a Coolpix NRW. **None passes 1 MiB**,
  the cap on one value, though the MakerNote entry's 993,826 bytes come
  within 6% of it
  ([ADR023](../../architecture-decisions/adr023-every-entry-a-file-records.md)).
  ShotInfo is 63,397 bytes on every Z 9 file, and the XMP packet 32,768.
- **The largest arrays** are IFD0's XMP (34,816 BYTEs) and a SubIFD's
  StripOffsets (1,231 LONGs). No RATIONAL entry has more than 6 values,
  no ASCII entry more than 65 bytes.

### 12.3 Entries a reader leaves out

The view leaves out a damaged entry and counts it in its directory
(ADR023). A directory that does not lie where its reader allows is not
listed; the view gives it as left out instead, with the link that leads
to it and the cause (`Entries::skipped_directories`). On the NEF files:
**[F]**

| Directory | What | Tags | Files, public / mine |
|---|---|---|---|
| MakerNote | a value past the MakerNote's declared length, left out and counted | 0x0E01 | 2 / 0 (COOLSCAN files) |
| MakerNote preview IFD | the whole IFD past the MakerNote's declared length: given as left out, held by the MakerNote's 0x0011, malformed | whole directory, whose 0x011A and 0x011B values lie further out | 6 / 0 (Coolpix NRW and COOLSCAN files) |
| MakerNote preview IFD | a PreviewIFD tag that is not one LONG: given as left out, with no position, malformed | 0x0011 | 0 / 0 |

- **None of my 11,376 files has a damaged entry or directory**, nor any
  of the fixtures. Measured again on 2026-11-16, none of my 29,483 NEFs
  gives a directory left out; 11,743 of the 11,759 files have no irregularity of
  any kind. **[F]**
- **Irregular but readable, and kept:** a value at an odd offset, which
  TIFF asks to be even (in the MakerNotes of 8 public files, IFD0 of 7,
  the Exif IFD of 1 and a SubIFD of 1); tags out of ascending order in the
  MakerNote of 2 public files. **[F]**
- **No file has** a count or an offset that overflows, a value past the
  end of the file, a type TIFF does not define, a loop, or a directory
  of more than 1,000 entries. **[F]**

### 12.4 The encrypted records

ShotInfo, ColorBalance and LensData are plain UNDEFINED entries to a
TIFF reader; whether each is encrypted depends on the four-digit
version at its start ([section 4](#4-encrypted-blocks)). The view gives
their bytes as recorded and does not mark them
([ADR023](../../architecture-decisions/adr023-every-entry-a-file-records.md)).
By the rule of section 4, on these files: **[F]**

| Record | Plain versions | Encrypted versions | Files with no version |
|---|---|---|---|
| 0x0091 ShotInfo | 0100, 0102, 0103, 0400, 0600: 31 public files | 02xx (0204 to 0246) and 08xx (0800 to 0811): 272 public files and all 11,376 of mine (0805 on every Z 9 file) | 13 public |
| 0x0097 ColorBalance | 0100, 0102, 0103: 7 public files | 0204 to 0228, 0400 to 0405, 0500, 0502, 0601, 0800 to 0803: 295 public files and all of mine (0803 on every Z 9 file) | none |
| 0x0098 LensData | 0100, 0101: 7 public files | 0201 to 0204, 0401 to 0403, 0800 to 0802: 288 public files and all of mine (0802 on every Z 9 file) | none |

### 12.5 What the view leaves out

- **The content of the encrypted records**: their bytes only, until a
  decision allows decrypting them (ADR016).
- **The XMP packet** (IFD0 0x02BC, on 227 public files and 10,876 of
  mine) and **the ICC profile** (0x8773, 2 public files): their bytes
  only; XMP is parsed by full metadata's group for XMP and IPTC. No NEF
  has an IPTC (0x83BB) or Photoshop (0x8649) entry. **[F]**
- **Capture NX's edit records** (0x0E01): their bytes only.
- **Scanner NEFs** of the COOLSCAN, which open as NEF; their directories
  are walked as any NEF's, and their entry and preview IFD past the
  MakerNote's end are left out as above.
- **Capture settings the typed values will not read.** Their entries
  stay in the view with their bytes. Each is a named gap
  ([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)),
  counted on the files of [section 8.10](#810-more-capture-settings),
  public / mine **[F]**:

| Value | Where | Files, public / mine | Why it waits |
|---|---|---|---|
| The flash record's fields, the unit's model among them | 0x00A8 FlashInfo, in 12 versions: 0100 to 0108, 0200, 0300, 0301 | 278 / 48,359 | its layout changes with each version, each needing frames of its own; the model is named only by another reader's table of firmware numbers |
| Commander mode | `&Comdr.` in 0x0009's text; 0x0087's code 8 | 0 / 82; 0 / 538 | the commander settings lie in FlashInfo, and 0x0087 is not read |
| A flash unit's own compensation | 0x0017 ExternalFlashExposureComp | 266 / 48,359, not zero on 5 / 8 | one getter gives the camera's compensation, 0x0012; a second for the unit's waits for a caller who needs it |
| Which saved lens AF fine-tune belongs to | byte 1 of 0x00B9, when not 255 | 8 / 56 | an index into the camera's list of saved lenses, which no file records [I] |
| The kind of stabilization, in the body or the lens too | byte 8 of 0x001F at 0200 | 97 / 2,571, my Z 9 card folders, the only files of mine on which byte 8 was counted | 0 and 1, on 33 public files, are undocumented; 1 is on the D780 and the Z bodies without in-body stabilization |
| Program shift | 0x000D | 271 / 48,395, not zero on 0 / 2 | reading every plain setting found would double the work |
| Exposure difference | 0x000E | 271 / 48,395, not zero on 81 / 5,435 | the same |
| ISO expansion | in 0x0025 ISOInfo | 260 / 48,395 | the same |
| ShootingMode's other bits | 0x0089 bits 1 (delay), 2 (PC control), 9 and 11 (pre-capture) | bit 1 on 30 / 8, bit 2 on 12 / 0, bit 9 on 2 / 277, bit 11 on none | bit 9 is undocumented, seen also on one D600 frame of mine; bits 1 and 2 come beside any release mode; bit 11, a burst, is not read, and gives the file no release mode ([section 8.10](#810-more-capture-settings)) |
| Sensitivity type | Exif 0x8830 SensitivityType | 218 / 46,808 | the same as program shift |

- **Processing values the typed values will not read.** Each stays in
  the view with its bytes, and is a named gap
  ([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)),
  counted on the files of
  [section 8.11](#811-picture-style-and-processing), public / mine
  **[F]**:

| Value | Where | Files, public / mine | Why it waits |
|---|---|---|---|
| Diffraction compensation | the menu settings of the encrypted ShotInfo, where the documentation places it [S][1]; no plain tag | none | it waits with the encrypted records (ADR016) |
| A Picture Control's number, and its bytes beside the documented fields | 0x0023 and 0x00BD: the bytes before the adjustment kind, and bytes 55, 56 and 75 to 107 of the 108-byte record | 262 / 48,428 | not documented; section 8.11 says what the files show |
| A Picture Control of another version | 0x0023 | 0 / 0 | no file shows which layout it takes |
| HDR's second level, and its "auto" | byte 7 of 0x0035 at 0100 and 0101; byte 4's code 48 | 10 / 14; 0 / 0 | the documentation does not say what the second level adds; 48 names neither on nor off |
| Multiple exposure | 0x00B0 MultiExposure, at versions 0100 to 0103 | 260 / 48,428, on on 0 / 5 | reading every plain value found would add about ten values, most of them off on every measured file |
| Retouch history | 0x009E RetouchHistory, ten SHORTs | 270 / 48,428, a retouch on 0 / 510 | the same; it describes a copy the camera made, not the shot |
| Retouch record | 0x00BB RetouchInfo | 214 / 48,428 | the same |
| The texts of the bodies before the Picture Control | 0x0006, 0x0080, 0x0081, 0x008D, 0x0092, 0x0094, 0x00A9, 0x00AA, 0x00B3 | 0x0006 on 31 / 0; the others on 3 to 28 / 0 | the same; texts with no documented list, on the DSLRs before the D300 and on Coolpix bodies |
| The lens correction parameters | 0xC7D5 NEFInfo, in a SubIFD, with distortion and vignette parameters [S][1] | 102 / 16,879 | a raw developer needs them; they wait with raw sensor data |

## Sources

Retrieved October 2026.

- [1] ExifTool, Nikon tags — https://exiftool.org/TagNames/Nikon.html
- [2] ExifTool, EXIF tags — https://exiftool.org/TagNames/EXIF.html
- [3] ExifTool, `Nikon.pm` (read as a reference only; GPL/Artistic) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/Nikon.pm
- [4] ExifTool, `MakerNotes.pm` (read only) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/MakerNotes.pm
- [5] exiv2, MakerNote notes — https://exiv2.org/makernote.html
- [6] exiv2, `makernote_int.cpp` and `nikonmn_int.cpp` (read only) —
  https://github.com/Exiv2/exiv2/blob/main/src/makernote_int.cpp
- [7] LibRaw, Nikon metadata parser `nikon.cpp` (read only) —
  https://github.com/LibRaw/LibRaw/blob/master/src/metadata/nikon.cpp
- [8] rawspeed, `NefDecoder.cpp` (read only) —
  https://github.com/darktable-org/rawspeed/blob/develop/src/librawspeed/decoders/NefDecoder.cpp
- [9] rawler (dnglab), NEF decoder (read only) —
  https://github.com/dnglab/dnglab/blob/main/rawler/src/decoders/nef.rs
- [10] dcraw (read only) — https://dechifro.org/dcraw/dcraw.c
- [11] Laurent Clévy, NEF file format notes (2009) — http://lclevy.free.fr/nef/
- [12] ozhiker, Nikon MakerNote tags —
  https://www.ozhiker.com/electronics/pjmt/jpeg_info/nikon_mn.html
- [13] exiv2 bug tracker, WorldTime time zone thread —
  https://dev.exiv2.org/boards/3/topics/1691
- [14] exiv2 bug tracker, D40 NEF tag dump —
  https://dev.exiv2.org/attachments/233
- [15] Imaging Resource, Nikon Z 9 sample EXIF listing —
  https://imaging-resource.com/PRODS/nikon-z9/EXIF/Y-JG-NIKON-Z9-0375.HTM
- [16] pixls.us forum, Z 6 white balance thread —
  https://discuss.pixls.us/t/darktable-nikon-z6-white-balance-error/10919
- [17] LibRaw, internal definitions (channel-order macro) —
  https://github.com/LibRaw/LibRaw (`internal/defines.h`)
- [18] DPReview, Nikon WB encryption article —
  https://www.dpreview.com/articles/8306390752/nikon-encryptnef
- [19] ExifTool forum, "Possibly incorrect maker notes offsets" —
  https://exiftool.org/forum/index.php?msg=6994
- [20] DxO forum, "Nikon D850 RAW M size resolution is not recognized
  correctly" —
  https://forum.dxo.com/t/nikon-d850-raw-m-size-resolution-is-not-recognized-correctly/24178
- [21] ExifTool forum, truncated MakerNote report (search extract) —
  https://exiftool.org/forum/index.php?msg=28917
- [22] raw.pixls.us, the catalog's exiv2 metadata listing of each file
  (read October 2026) — https://raw.pixls.us/
- [23] CIPA DC-008-Translation-2026, "Exchangeable image file format for
  digital still cameras: Exif Version 3.1" (January 2026), read
  November 2026 —
  https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-008-2026-E
- [24] exiv2, Nikon MakerNote tag reference, read November 2026 —
  https://exiv2.org/tags-nikon.html
- [25] Nikon, *D600 User's Manual*, 6MB15211-06, the edition Nikon's
  download center gives, read as documentation November 2026; pages
  as printed
- [26] Nikon, *Z 9 Reference Guide*, for firmware 5.20, read as
  documentation November 2026 — https://onlinemanual.nikonimglib.com/z9/en/
- [27] Nikon, *D7100 User's Manual*, 6MB18111-04, the edition Nikon's
  download center gives, read as documentation November 2026; pages
  as printed
- [28] Nikon, *D7200 User's Manual*, 6MB27011-03, read as
  documentation November 2026; pages as printed
