# Nikon MakerNote and viewer metadata

This page describes the Nikon MakerNote found in NEF files: its three
layouts, the tags that matter, the encrypted blocks and how they are
decrypted, and where a viewer finds orientation, capture date, make, model
and image size, where the camera recorded the point it focused on, and
how the camera was set for the shot.
It serves milestone 3 (basic metadata), milestone 4 (focus points) and,
for the decoding-related tags, milestone 6 (raw data).

See [the NEF overview](README.md). The TIFF container and the preview JPEGs
are in [container-and-previews.md](container-and-previews.md); decoding of
the sensor data is in [raw-data.md](raw-data.md).

Nikon publishes no specification for any of this. Everything below comes
from the Exif and TIFF standards, the tag documentation of ExifTool and
exiv2, and the source of existing decoders, read as references only (their
licences do not allow copying code; see [Licences](#licences)). Claims carry
a confidence tag: **[P]** primary (specification, tag documentation or
implementation source), **[S]** secondary (write-ups, forum posts, sample
listings), **[I]** inferred, and **[F]** measured on a named sample file,
cited by its raw.pixls.us id ("file 928"). The MakerNote layout and the
viewer metadata (sections 1, 2 and 8) were checked in October 2026 on 23
CC0 files from the D1H to the Z6III, listed in
[test-data.md](test-data.md#21-the-ci-set), and the focus point
(section 8.6) on those and seven more CC0 files, listed in
[test-data.md](test-data.md#23-files-checked-for-the-focus-point), and the
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
- [Sources](#sources)

## Licences

Every mature NEF implementation is GPL, LGPL, CDDL or Artistic licensed
(licences incompatible with the raw-files MIT OR Apache-2.0 licence, or
ambiguous in dcraw's case). This page therefore restates algorithms in prose
and cites the two decryption tables by location only; do not paste code or
tables from those projects into raw-files. [P]

## 1. Finding the MakerNote

The MakerNote is Exif tag 0x927C (type UNDEFINED) in the Exif IFD. [P][5]
Nikon has used three layouts. Projects number them differently (exiv2 calls
them Nikon1, Nikon2, Nikon3; one older write-up swaps the first two; ExifTool
uses yet other names), so this page names them by shape and raw-files should
detect them by bytes, never by a "type" number. [P][5][6][12]

| Layout | Header | IFD starts at | Offsets relative to | Byte order | Bodies |
|---|---|---|---|---|---|
| Headerless | none | MakerNote + 0 | the outer TIFF header | the outer file's (the D1 is big-endian) | confirmed on the D1H and the D1X (files 5364, 1785 and 1786) and on the D1 (file 201, not in the CI set), big-endian, file and MakerNote alike **[F]**; in camera JPEGs of the Coolpix E990, E995 and E880, and of the E885, E2500 and E5000 on their earlier firmware, little-endian, Exif segment and MakerNote alike ([In a JPEG file](#in-a-jpeg-file)) **[F]**; the E-series Coolpix files of raw.pixls.us, the E5400, E5700, E8400 and E8800 (files 212, 4456, 2082 and 2769), write the embedded-TIFF layout instead **[F]** |
| Short header | `Nikon\0` `01 00` (8 bytes) | MakerNote + 8 | the outer TIFF header | exiv2 uses the outer order; ExifTool says little-endian; the files are little-endian, and so is their outer order, so they do not tell which is right **[F]** | in camera JPEGs of the Coolpix E700, E800, E900 and E950 ([In a JPEG file](#in-a-jpeg-file)) **[F]** |
| Embedded TIFF header | `Nikon\0`, version `02 00`, `02 10` or `02 11`, `00 00`, then a TIFF header (`II*\0` or `MM\0*` and a 4-byte offset) (18 bytes) | MakerNote + 10 + the inner IFD offset (normally 8, so + 18) | MakerNote + 10, the inner TIFF header | the inner header's, independent of the file's | D70, D100, D200 and later DSLRs, every Z body; every checked file but the D1H's and the D1X's: `02 00` on the D100 and the Coolpix E5700 and P7800, `02 10` from the D70 to the D7000 and on the Nikon 1 J1, `02 11` on the Coolpix A (2013) and on every body from the D810 (2014) on **[F]** |

Sources: [5][6][4][7]. The D1 byte-order remark is from ExifTool. [P]

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

The three implementations differ on how they recognise the embedded layout:
exiv2 tests for the TIFF magic at offset 10, LibRaw tests the version byte at
offset 6, ExifTool tests the `Nikon\0\2` prefix. They agree on real files;
testing all of the conditions above is the robust choice. [P][6][7][4]

Check the signature before Make. Capture NX can write Nikon-format
MakerNotes into files from other brands, so the Make test must not gate the
embedded layout. [P][4]

raw-files reaches the MakerNote reader only under a Make that starts with
`NIKON`: the NEF claim and the JPEG reader test IFD0's Make first, so the
Make gates every layout, the embedded one included
([section 10](#10-recommendation-for-raw-files)). Under that Make, a
MakerNote that starts with `Nikon\0` is read as the embedded-TIFF layout
or not at all, never retried as headerless, and the short-header layout
is not read; any other MakerNote is read as headerless, an IFD at + 0,
whose first two bytes are its count of entries.

### Layout details

- The inner IFD offset is relative to the inner TIFF header. A gap between
  the 8-byte inner header and the IFD is legal; exiv2 drops it when
  rewriting, which shows that such files exist. [P][6]
- The inner byte order can differ from the outer file's, for example a
  big-endian NEF carrying a little-endian MakerNote. Always use the inner
  header's order for the IFD itself. [P][3]
- Some sub-blocks have a fixed byte order regardless of the MakerNote's: [P][3]
  - 0x0014 (ColorBalanceA, Coolpix) and 0x0025 (ISOInfo) are big-endian;
  - LensData versions 08xx and ShotInfo on newer bodies (D780, D7500 and
    later, and the Z bodies) are little-endian, while older D-series
    ShotInfo is big-endian;
  - Capture NX can change the MakerNote's byte order, after which ExifTool
    byte-swaps ColorBalance 0x0097 and WorldTime to compensate; it also
    notes ShotInfo staying big-endian regardless.
- The D200's MakerNote IFD has no next-IFD pointer; do not require one. [P][5]
- Offsets in the headerless and short-header layouts are relative to the
  outer TIFF header, so anything that moves the MakerNote breaks them. The
  embedded layout survives being moved as a block. [P][5] Early Coolpix
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
lists them with their licences. Every one is little-endian, Exif segment
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
  raw-files does not read it
  ([section 10](#10-recommendation-for-raw-files)).
- **The short header's byte order stays open.** ExifTool's
  documentation says its numbers are little-endian, and exiv2 reads
  them in the outer order (table above); since every one of these files
  is little-endian outside the MakerNote too, they agree with both.
  [P] **[F]**

## 2. Key tags

IDs are for the embedded-TIFF layout. Types are as documented by ExifTool;
read integers from the bytes at the declared count rather than trusting the
declared type (see [section 9](#9-robustness-pitfalls)). "M" is the roadmap
milestone that needs the tag. [P][1]

| ID | Name | Type | Meaning | M |
|---|---|---|---|---|
| 0x0001 | MakerNoteVersion | undef[4] | ASCII digits on some bodies, binary on others; do not rely on it | none |
| 0x0002 | ISO | u16[2] | `[0, ISO]` on most bodies before the D810; absent from the D810 on; four zero bytes of type UNDEFINED on the D700 (file 2255), `[0, 0]` on the E5700 (file 4456) **[F]** | 3 |
| 0x0005 | WhiteBalance | string | the white balance setting, as upper-case text padded with spaces; on every checked body; values in [section 8.7](#87-camera-settings) **[F]** | 3 |
| 0x000B | WhiteBalanceFineTune | s16[1 or 2] | the white balance fine-tuning: one value from the D1H to the D200, two from the D300 on; not written by the Z 9, Z f, Z5II, Z50II and Z6III, which write 0x003F alone **[F]** | 3 |
| 0x000C | WB_RBLevels | rational[4] | as-shot white-balance multipliers; LibRaw reads them as R, B, then two greens (ExifTool leaves the last two unidentified) | 6 |
| 0x0011 | PreviewIFD | IFD | sub-IFD with a small JPEG preview; offsets relative to MakerNote + 10 | 1 |
| 0x001B | CropHiSpeed | u16[7] | crop mode and geometry; field order in [section 8.4](#84-image-dimensions) | 3, 4, 6 |
| 0x001D | SerialNumber | string | the body's serial number; decryption key input. Seven digits on every checked DSLR and mirrorless body from the D300 on (eight on the Nikon 1 J1) and on the Coolpix A; seven spaces on the D200 (file 8079); absent before it and on the Coolpix E5700 and P7800 **[F]** | 3, 6 |
| 0x001E | ColorSpace | u16 | the colour space setting: 1 sRGB, 2 Adobe RGB, 4 BT.2100; from the D200 on, but not on the Coolpix P7800, while no checked NEF has the Exif ColorSpace **[F]** | 3 |
| 0x0021 | FaceDetect | u16 | the size of a frame and up to twelve face rectangles in it; on none of the 30 checked files **[F]** | none |
| 0x0024 | WorldTime | undef | the time zone and daylight saving setting of the camera's clock; layout in [section 8.2](#82-capture-date-and-time-zone) | 3 |
| 0x0034 | ShutterMode | u16 | the shutter used: mechanical, electronic front curtain or electronic, and whether the camera chose it in its auto mode; on the Nikon 1 J1, D850, D780 and every Z body; codes in [section 8.7](#87-camera-settings) **[F]** | 3 |
| 0x0037 | MechanicalShutterCount | u32 | newer bodies; mechanical frames only | none |
| 0x003D | BlackLevel | u16[4] | per-channel black level, recorded at 14-bit scale | 6 |
| 0x003E | ImageSizeRAW | u16 | the raw size setting: 1 large, 2 medium, 3 small; from the D750, D810 and D5600 on **[F]** | 3, 6 |
| 0x003F | WhiteBalanceFineTune | srational[2] | the white balance fine-tuning as two fractions; from the D810 on **[F]** | 3 |
| 0x0045 | CropArea | u16[4] | left, top, width, height of the output rectangle | 3, 4, 6 |
| 0x004F | ColorTemperatureAuto | u16 | the colour temperature, in kelvin, that auto white balance chose; 0 under another setting; on the D5, D850, D780 and every Z body **[F]** | 3 |
| 0x0051 | (unnamed, Z bodies) | undef | NEFCompression as a u16 at byte 10; byte order disputed (see the note below). Present on the Z 6 (2018) too, and on the Z f, Z6III, Z 9, Z5II and Z50II (files 3587, 6887, 7808 to 7810, 7812, 7813, 5146 to 5148, 7735, 7736, 7767, 7770); absent on the Z fc (2021) and Z 30 (2022), which write 0x0093 instead (files 8559, 5812). Byte 10, read as a u16 in the MakerNote's byte order, little-endian on the three Z 9 files, is 3 (lossless) on the Z 9's lossless file, 14 (HE\*) on its HE\* file and 13 (HE) on its HE file (files 5146 to 5148) **[F]** | 6 |
| 0x0084 | Lens | rational[4] | the lens's shortest and longest focal length and the widest aperture at each; on every checked body with interchangeable lenses, from the D1H on; equal to Exif LensSpecification wherever both exist **[F]** | 3 |
| 0x0088 | AFInfo | undef[4] | area mode, the number of the focus point and a mask of the points in focus; no position. The only focus data of the checked bodies without AFInfo2: the D1H, D70, D100, D200 and Coolpix E5700 and P7800, and written by no checked body that has AFInfo2 **[F]**; [section 8.6](#86-focus-point) | 4 |
| 0x0091 | ShotInfo | undef | model-specific; encrypted for versions 02xx and 08xx | none |
| 0x0093 | NEFCompression | u16 | compression hint; absent on the Z bodies checked but the Z fc and Z 30, which write 0x0051 instead, and on the D1H and E5700; present on the Z fc and Z 30, with the value 3 (lossless), where 0x0051 is absent (files 8559, 5812) **[F]** | 6 |
| 0x0096 | NEFLinearizationTable | undef | curve and predictor seeds; unencrypted | 6 |
| 0x008C | (older curve) | undef | fallback for 0x0096 | 6 |
| 0x0097 | ColorBalance | undef | white balance on older bodies; encrypted from version 0200 | 6 |
| 0x0098 | LensData | undef | lens description; encrypted from version 0201 | none |
| 0x00A7 | ShutterCount | u32 | decryption key input | 3, 6 |
| 0x00B7 | AFInfo2 | undef | where the camera focused: a block that starts with a four-character version, plain on every version; layouts in [section 8.6](#86-focus-point) | 4 |
| 0x0E01 | NikonCaptureData | undef | Capture NX edit data (optional) | none |
| 0x0E22 | NEFBitDepth | u16[4] | secondary bit-depth source | 6 |

Notes:

- 0x00A7 on newer bodies is the combined mechanical and electronic count;
  0x0037 is mechanical-only. The decryption key uses 0x00A7. [P][1]
- Byte 10 of 0x0051 is read in the MakerNote's byte order by LibRaw, while
  only rawler forces little-endian; the sources disagree. [P][7][9]
- 0x0014 holds Coolpix colour data (2560-byte or 1280-byte variants, or an
  "NRW" block on P-series cameras) and is irrelevant to NEF DSLRs. [P][1]
- Capture NX data (0x0E01) is a sequence of records that can carry a white
  balance adjustment, a custom white balance and a flip; one implementation
  walks at most 1024 records. Ignore it unless a need arises. [P][7]
- The curve in 0x0096 (and its fallback 0x008C) is decoder input; see
  [raw-data.md](raw-data.md).

## 3. NEFCompression and bit depth

NEFCompression (0x0093; on most Z bodies it is in 0x0051, since 0x0093
is absent from every checked Z body, the Z 6 included, but the Z fc and
Z 30, which write 0x0093 and no 0x0051 (files 8559, 5812) **[F]**; the
sources put it at byte 10 of 0x0051 on the Z 9 and later) takes these values. [P][1][3][9]

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
  Decoders therefore also compare the strip byte count with the expected
  uncompressed size. [P][8][9][1]
- Values 13 and 14 (HE, HE*) are not decodable with the open implementations
  surveyed; report a typed "unsupported compression" error and never attempt
  the lossless decoder on them. [P][9]
- Fixed-lens (Coolpix) detection in LibRaw uses values 7 and 9 as a
  heuristic. [P][7]

### Bit depth sources

In order of authority: [P]

1. BitsPerSample (0x0102) of the raw IFD, used by rawspeed and rawler; the
   values are 12 or 14, with 16 on some uncompressed paths. The sNEF variant
   stores 3 bytes per pixel of 12-bit YCbCr 4:2:2 (see
   [raw-data.md](raw-data.md)), so this tag does not describe a CFA depth
   there. [8][9]
2. The curve header in 0x0096: a version header of 0x44 0x40 marks the
   lossy "2nd version" curve (D780, Z 6, Z 7 and others). rawspeed and
   rawler treat it as a curve over a domain two bits narrower and lower the
   bits per sample by 2 (14 to 12, or 12 to 10). A 0x46 header is lossless.
   [8][9]
3. NEFBitDepth (0x0E22): 12 0 0 0, 14 0 0 0, 8 8 8 0 or 16 16 16 0; LibRaw
   stores it without acting on it. [7][1]
4. The NEFCompression value (5, 6, 7 and 9 are 12-bit variants, 10 is packed
   14-bit). [1]

## 4. Encrypted blocks

Three MakerNote blocks are encrypted. The four-byte ASCII version at the
start of each is never encrypted, and the version selects both whether
decryption applies and where it starts. [P][1][3][5][7]

| Tag | Encrypted when | Decrypt from byte |
|---|---|---|
| 0x0097 ColorBalance | version 02xx (0100, 0102, 0103 are plain) | 4 for versions 0205, 0219 and 0221 to 0224; 284 for the other 02xx versions, including 0220 (ExifTool's ColorBalanceUnknown1, whose white balance it does not decode) |
| 0x0098 LensData | version 0201 and later (0201 to 0204, 04xx, 08xx) | 4 |
| 0x0091 ShotInfo | versions 02xx and 08xx | 4 |

0x00A8 FlashInfo and 0x00B7 AFInfo2 are not encrypted. [P][5] Encrypted
ColorBalance first appeared on the D2X, D2Hs and D50 in 2005. [S][18] The
0x0097 variants of the Nikon 1 (04xx) and Coolpix (05xx, 06xx) are
encrypted but not decoded by the surveyed projects. [P][3][7]

Z-series ColorBalance (version 0800) is encrypted and no surveyed project
maps its white balance offsets. [P][3][7] This does not matter in practice;
see [section 6](#6-white-balance).

## 5. The decryption algorithm

The cipher is a small stream cipher over the block's bytes. It is symmetric
(the same routine encrypts and decrypts) and three independent
implementations describe it the same way. [P][3][6][7]

Inputs, all from the same file's MakerNote:

- the serial number (tag 0x001D) as a decimal integer;
- the shutter count (tag 0x00A7) as four bytes;
- two fixed 256-byte substitution tables, T0 and T1, listed below.

Procedure:

1. Reduce the serial number to a single byte, `s` (the low eight bits of
   the integer). Handling of non-numeric serials is in
   [section 5.1](#51-non-numeric-serial-numbers).
2. Reduce the shutter count to a byte, `k`, by XOR-ing its four bytes
   together.
3. Initial state: `ci = T0[s]`, `cj = T1[k]`, `ck = 0x60`. `ci` stays
   constant; `cj` and `ck` evolve.
4. For every byte from the decryption start offset: `cj = (cj + ci * ck)
   mod 256`, then `ck = (ck + 1) mod 256`, then the output byte is the input
   byte XOR `cj`. [P][3][10]
5. To start `n` bytes into the stream without replaying the first `n`, set
   `cj = (cj0 + ci * (n * ck0 + n * (n - 1) / 2)) mod 256` and `ck = (ck0 +
   n) mod 256`, where `cj0` and `ck0` are the initial values from step 3.
   This is useful for blocks where only a later range is needed. [P][3]

Where the tables live (cite only; do not copy them): ExifTool's
`Nikon.pm` (the array named `@xlat`) [3], exiv2's `nikonmn_int.cpp` and
`makernote_int.cpp` [6], LibRaw's `nikon.cpp` [7], and dcraw's `dcraw.c`
[10]. All of these are under licences incompatible with that of raw-files, or
ambiguous in dcraw's case; see [Open questions](#11-open-questions).

Consequences for the parser:

- The shutter count (0x00A7) sits after the encrypted blocks in IFD order.
  Parse the whole MakerNote IFD first, then decrypt. LibRaw works around
  this by deferring decryption until it reaches 0x00A7. [P][7]
- A wrong key produces garbage silently, never an error. Check decrypted
  values for plausibility (for example white balance multipliers in a sane
  range) and treat failure as "value absent". [I]
- If the serial or shutter count is missing, skip decryption and carry on.
  [I]
- Minimum lengths used by rawspeed as bounds checks for 0x0097 white balance
  reads: version 0100 needs 80 bytes, 0103 needs 26, 0204 needs 564, 0205
  needs 284. [P][8]
- dcraw decrypts only a 324-byte ColorBalance buffer for versions 0200 to
  0216 (it checks the version number against that range); LibRaw applies the
  same keystream to ColorBalance, LensData (versions above 0200) and ShotInfo
  (from byte 4, version 0208 and later). [P][10][7]

### 5.1 Non-numeric serial numbers

Most serial numbers are digits only. On some bodies (the D50 family and a
few others) tag 0x001D is not numeric, and the surveyed implementations
disagree on what to do. [P]

| Project | Behaviour |
|---|---|
| ExifTool and exiv2 | key byte 0x22 (34) if the model is a D50-family body, otherwise 0x60 (96) [3][6] |
| LibRaw | stops at the first non-digit; uses 34 if the model is "D50" (or ends in D50 after a non-alphanumeric character), otherwise 96; this value replaces the low byte of the serial number as the index into T0 [7] |
| dcraw | never stops: each non-digit contributes `c mod 10` to the decimal value, so the key is the low byte of that folded number [10] |

ExifTool and exiv2 use the same constants as LibRaw, but dcraw can derive a
different key for a serial that contains letters. All four agree for
all-digit serials. Which behaviour is correct for a real body with a
lettered serial (the ExifTool comments attribute the 0x60 case to the D200
and D40X) is not settled. [P][I]

## 6. White balance

Sources, in the order raw-files should try them: [P][7][8][9][16]

1. **MakerNote 0x000C** (WB_RBLevels), unencrypted: four rationals. The
   reading "R, B, then two greens" for the last two values is LibRaw's;
   ExifTool calls them unidentified, and rawler uses G = 1.0. rawspeed and
   rawler use 0x000C first and fall back to 0x0097 only when 0x000C is
   absent. LibRaw reads both tags and a later tag may override an earlier
   one, so precedence there is implementation-defined. If a green value is
   zero or missing, treat green as 1.0 and normalise to it. The tag is
   present on most NEFs, including the Z bodies. [P][3][7][8][9]
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

Sources: [3][7][8][9]. ExifTool and LibRaw count the offset from different
bases (ExifTool from the start of the decrypted segment, LibRaw from inside
a 324-byte block after skipping 284 bytes), so offsets in different rows
are not directly comparable. [P] ExifTool names the channel orders RBGG
(0100), RGGB (0102, 0205 and others), RGBG (0103), and GRBG for the
0209/0211/0212/0214 to 0217 group. Normalise to a fixed order and divide by
the first green. [P][3]

### The Z-series answer

Z bodies do not need the encrypted 0800 ColorBalance. A Z 6 forum report
describes a normal 0x000C with usable values [S][16]. In the decoders,
rawspeed and rawler contain no Z-specific white balance code (they read
0x000C), and LibRaw's 0x0097 branch covers only versions 0200 to 0217, so Z
bodies depend on 0x000C there too [P][7][8][9]. No report of an absent or wrong 0x000C on a Z body was found,
but this is an absence of evidence rather than a proof. [I]

For milestone 3 white balance is not needed at all; this section matters
from milestone 6.

## 7. LensData and ShotInfo

Neither is needed for previews, basic metadata or decoding. They are
documented here so that nobody decrypts them by accident or unnecessarily.

**LensData (0x0098).** A version-prefixed block (0100 is 9 bytes, 0101 and
0201 to 0203 are 15, 0204 is 16, 0400 to 0403 are 459 to 879, 0800 and 0801
are 58, 0802 is 108). It is encrypted from version 0201. Decrypted, older
versions hold a lens ID, F-stops, min and max focal length and min and max
aperture; focal lengths are 5 * 2^(b/24) mm and apertures 2^(b/24) for a
stored byte b. Z bodies use little-endian u16 fields instead. [P][1][7]

**ShotInfo (0x0091).** Model-specific tables selected by version (and
sometimes by block size). It holds roll, pitch and yaw as fixed-point
numbers, which LibRaw turns into a rotation flag at version-specific
offsets; this is the only orientation-like data in the MakerNote and it is
angle data, not the Exif Orientation value. ExifTool drops an oversized
ShotInfo (above 64 KB, seen on the Z 6III) when copying it into an Exif
segment; that is a size limit, not a judgement that the block is corrupt.
The lesson is that the block can be that large, so cap allocations.
[P][1][3][7]

## 8. Viewer metadata

Everything a viewer needs next to the preview comes from standard IFD0, its
SubIFDs and the Exif IFD, except the crop rectangle, which comes from the
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
Decryption is never needed for these fields. [P][1][2]

### 8.1 Orientation

IFD0 tag 0x0112, a u16 with values 1 to 8 (1 normal, 2 mirror horizontal, 3
rotate 180, 4 mirror vertical, 5 transpose, 6 rotate 90 clockwise, 7
transverse, 8 rotate 270 clockwise). There is no plain orientation tag in the
MakerNote. Treat 0 and values above 8 as 1. [P][2]

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

Also present: 0x9004 DateTimeDigitized and IFD0 0x0132 DateTime (the last
modification time). [P][2]

An unset camera clock yields a blank or space-filled string (spaces where the digits would be); return "none" for the date rather than an error. [I]

**OffsetTimeOriginal writers.** The D850, D780, Z 6, Z f, Z6III and Z 9
write all three OffsetTime tags (files 1835, 3827, 3587, 6887, 7808 to 7810
and 5146 to 5148); no older checked body writes any, the D810 (file 2036)
included. **[F]** A Z 9 JPEG listing showed the same. [S][15] Treat 0x9011
as optional. SubSecTimeOriginal is in every checked file except the
Coolpix E5700's and P7800's (files 4456 and 1425). **[F]**

**WorldTime layout (0x0024).** The data is an array of bytes with this
layout, per ExifTool's definition: [P][3][1]

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

IFD0 0x010F and 0x0110. Typical values: Make "NIKON CORPORATION" and Model
"NIKON D90" on DSLRs, Make "NIKON" and a Model such as "COOLPIX P7000" on
compacts. [S][14]

In the one NEF dump inspected the strings were NUL-terminated with the count
including the NUL and no space padding (Make 18 bytes, Model 10 bytes for a
D40). [P][14] The D100 does space-pad Model: rawspeed compares it against
"NIKON D100 " with a trailing space. [P][8] The Exif standard allows
space-filled "unknown" values, and ExifTool and LibRaw both strip trailing
whitespace. So: cut at the first NUL, then trim trailing ASCII whitespace.
[P][2][7] Compare model strings after trimming, because model checks
elsewhere ("D50" suffix, a "Z" at index 6) assume trimmed text. [I]

The samples agree: every checked file has NUL-terminated Make and Model
with no padding, the Z 9's 18 and 10 bytes long like the D40's, except the
D100, whose Model is `NIKON D100 ` with one trailing space (file 2996).
Make is `NIKON` on the Coolpix E5700 and P7800 and `NIKON CORPORATION`
on every other body, the Coolpix A included. **[F]**

### 8.4 Image dimensions

The raw IFD's ImageWidth and ImageLength (0x0100, 0x0101) give the stored
sensor array, which includes masked and border pixels, so it is usually
larger than the image a viewer shows. NEF has no DNG-style DefaultCropOrigin,
DefaultCropSize or ActiveArea; those tags appear in DNG files only. [P][2][9]
Sources for the output size, in order: [P][7][9]

1. **CropArea (0x0045)**: four u16 as left, top, width, height in raw
   pixels. rawler uses it as the crop rectangle; LibRaw stores it as an
   inset crop. [P] The D5, D850, D780, Z 6, Z f, Z fc, Z 30, Z6III and
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
   milestone 3. [I]
3. Exif PixelXDimension and PixelYDimension (0xA002, 0xA003): these exist,
   but Nikon's values describe the in-camera JPEG size and may differ from
   the raw crop. [I] No checked NEF has them. **[F]**

**CropHiSpeed (0x001B)** is seven u16 values. ExifTool's print conversion
fixes the order as: [P][3]

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
1.4x movie, 15 1.5x movie, 17 FX 1:1, 18 DX 1:1. [P][1] LibRaw maps the
modes to aspect ratios (0, 1, 2 and 4 as 3:2; 3 as 5:4; 6 as 16:9; 17 as
1:1). [P][7] On "uncropped" bodies the crop modes change the raw array
itself. [P][1]

ImageSizeRAW (0x003E) says whether a small-raw file is large, medium or
small. The D850 stores M and S files at sizes that differ from the menu
sizes (reported as 7104x4728 and 6216x4136 where the menu says 6192x4128 and
4128x2752); converters rescale. [S][20] For the preview, the size comes from the
SOF marker of the chosen JPEG; see
[container-and-previews.md](container-and-previews.md).

The D100 is special: rawler forces a raw width of 3040 on its uncompressed
path. [P][9]

### 8.5 Serial number, ISO and lens

Nikon added these to the standard Exif tags late; older bodies record
them only in the MakerNote, in plain form. **[F]**

| Item | Exif tag | MakerNote tag | Measured |
|---|---|---|---|
| Body serial number | 0xA431 BodySerialNumber | 0x001D SerialNumber | Exif from the D780 and Z 6 on (files 3827, 3587); MakerNote from the D300 on, blank on the D200 |
| ISO | 0x8827 ISOSpeedRatings | 0x0002 ISO, `[0, ISO]` | Exif on every checked body except the D1H, D1X, D100, D70, D700 and E5700; the first four have it in the MakerNote |
| Lens model | 0xA434 LensModel | none in plain text | the D780, Z 6 and later, and the Nikon 1 J1 (file 2956) |
| Lens range | 0xA432 LensSpecification | 0x0084 Lens | Exif from the D780 and Z 6 on; MakerNote on every body with interchangeable lenses, equal to Exif where both exist |
| Lens make, serial | 0xA433 LensMake, 0xA435 LensSerialNumber | none | the D780 and Z bodies; LensMake is empty for the D780's F-mount lens and the Z f's third-party lens (files 3827, 6887) |

ISO is two SHORT values in 0x0002, of which the second is the ISO; the
D700 writes four zero bytes of type UNDEFINED and the E5700 `[0, 0]`,
which mean no ISO is recorded. The Z 9's Exif ISO is 0x8827 with
SensitivityType 2 and an equal RecommendedExposureIndex (0x8832). The Z 9
writes exposure values unreduced (ExposureTime `10/3200`, FNumber
`900/100`, FocalLength `180/10`, files 5146 to 5148) and every checked
body writes ExposureBiasValue as a signed rational (`0/6`; `-6/6` on the
D100). **[F]**

### 8.6 Focus point

Where the camera focused, which milestone 4 reports so that a viewer can
judge sharpness at that point. raw-files decided what it reports in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md);
this section is the record behind it. The field offsets are ExifTool's
[P][1]; every value was read on the 35 CC0 Nikon files of
[test-data.md](test-data.md#23-files-checked-for-the-focus-point), and a
field is tagged **[F]** where a sample holds a value for it.

**Where a Nikon file records focus data.** Every source found, whether
raw-files reads it or not:

| Source | What it holds | Checked files | raw-files |
|---|---|---|---|
| 0x00B7 AFInfo2, position fields | the size of a frame, the centre of the focus area and the area's size in that frame | mirrorless bodies, and DSLRs in live view **[F]** | read, at the versions of the table below |
| 0x00B7 AFInfo2, in-focus byte | whether focus was confirmed: FocusResult on 04xx, ContrastDetectAFInFocus on 0100 and 0101 | Z 9, Z f, Z6III, Z50II, Z5II, D7000, D5600, D5, Coolpix A **[F]** | read; on 0100 the flag is byte 28, where ExifTool also lists a 20-byte AFPointsSelected that cannot fit in the 30-byte value |
| 0x00B7 AFInfo2, point fields | the primary point's number, a mask of the points used, and the layout of points (FocusPointSchema) | DSLRs through the viewfinder, the Nikon 1 J1, the Z5II in auto-area mode **[F]** | not read: a number needs a table of the body's points, which no file carries |
| 0x00B7 AFInfo2, AFPointsInFocus | a mask of the points in focus, version 0101 only (byte 48) [P] | the D850, one point **[F]** | not read: a mask, needs the same table |
| 0x0088 AFInfo | 4 bytes: area mode, point number, mask of the points in focus [P] | the bodies without AFInfo2: D1H, D1X, D70, D100, D200, E5700, P7800 **[F]** | not read: a number only |
| 0x0021 FaceDetect | a frame size and up to twelve face rectangles in it [P] | none of the 35 files **[F]** | not read: no sample |
| 0x0007 FocusMode | an ASCII string [P][1]: `MANUAL`, or an AF mode padded with spaces to six characters (`AF-S  `, `AF-C  `, `AF-A  `) **[F]** | `MANUAL` on the D300 and D850, both viewfinder shots with a zero frame (files 928, 1835), and on the Z fc, which records a position under it (file 8559); an AF mode on every other file with the embedded MakerNote, the maintainer's Z 9 shots included **[F]** | read: `MANUAL`, after trimming, gives no focus point |
| 0x0085 ManualFocusDistance, 0x00B9 AFTune, the area modes in the MenuSettings blocks | settings, not a position [P] | not looked for | not read |

**AFInfo2 layouts.** The value is UNDEFINED and starts with a version in
four ASCII digits; it is not encrypted on any version
([section 4](#4-encrypted-blocks)). Its multi-byte fields are in the
MakerNote's byte order: the Coolpix A, big-endian, reads a 4928x3264
frame, and the Z 9, little-endian, 8256x5504; the other order gives
nonsense (16403x49164, 16416x32789) (files 2238, 5146). **[F]** Byte
offsets from the start of the value:

| Version | Length | Detection | Area mode | Point schema | Coordinates flag | Frame W, H | Centre X, Y | Area W, H | In focus | Bodies |
|---|---|---|---|---|---|---|---|---|---|---|
| `0100` | 30 | 4 | 5 | 6 | none | 16, 18 | 20, 22 | 24, 26 | 28 | DSLRs from the D3 and D300 (2007) and the D90 (2008) up to the D4S, D750, D810, D7200, D5500, D5600 and Df, and the Coolpix A [S]; samples: D300, D700, D750, D810, D7000, D5600, Coolpix A **[F]** |
| `0101` | 84 | 4 | 5 | 6 | none | 70, 72 | 74, 76 | 78, 80 | 82 | D5, D500, D850, D3400, D3500, D7500 [P][S]; samples: D850, a viewfinder shot, and D5, in live view **[F]** |
| `0200`, `0201` | 42 [F] | none | 5 | 6 | none | none | none | none | none | Nikon 1 bodies [P][S]; sample: J1, `0200` **[F]** |
| `0300`, `0301` | 60 | 4 | 5 | 6 | 7 | 42, 44 | 46, 48 | 50, 52 | none | `0300`: D780, Z 6, Z 7, Z 50; `0301`: D6, Z 5, Z 6II, Z 7II, Z 30, Z fc [S]; samples: D780, Z 6, both `0300`, and Z 30, Z fc, both `0301` **[F]** |
| `0400`, `0401`, `0402` | 80 | 4 | 5 | none | 7 | 62, 64 | 66, 68 | 70, 72 | 74 | `0400`: Z 8, Z 9; `0401`: Z f, Z6III; `0402`: Z50II, Z5II [S]; samples: Z 9, Z f, Z6III, Z50II, Z5II **[F]** |

ExifTool groups the tables by the first two digits (0100; 0101 and later
01xx; 02xx; 03xx; 04xx). [P][1] The versions by body come from the exiv2
metadata listings of all 291 Nikon files of the raw.pixls.us catalogue
(October 2026), which show these seven versions besides the Nikon 1's
and no other; the bodies before the D3 and every Coolpix but the A have
no AFInfo2 there. [S][22] Those listings decode only the 0100 offsets,
and in one byte order whatever the file's, so they settle which version a
body writes and nothing else. raw-files reads the position at the exact
versions 0100, 0101, 0300, 0301, 0400, 0401 and 0402 (ADR012). The 0101
offsets are checked on the D5's live-view file, the only one that fills
them (file 2218), and version 0301, at the 03xx offsets, on the Z 30's
and the Z fc's (files 5812, 8559).

What the fields hold:

- **The centre of the area, not its corner.** ExifTool's 0100 and 0101
  tables say so [P][1]; on the Coolpix A it is exactly half the frame,
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
  one of them the point lies within 12 pixels of the frame's centre, so
  these files cannot tell how the values relate to the crop.
- **A DX crop records fractions of the cropped picture; an aspect crop
  records positions in the full frame.** Thirteen shots made for this
  check with the maintainer's Z 9 tell the two apart: two frames in each
  of FX, DX, 1:1 and 16:9 with a single point placed in the top-left
  corner of the picture, and five earlier frames of the same modes with
  the same values. One DX and one 1:1 frame are committed, stripped of
  their pixels and personal fields, as fixtures
  ([`tests/fixtures/`](../../../tests/fixtures/README.md)). In the
  8256x5504 frame they record:

  | Mode | CropHiSpeed | CropArea | Centre, area recorded | Scaled onto CropArea | Shifted by the centred window |
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
  it, centre (3776, 1741) and area 552x552 (file 5812). **[F]**
- **A DSLR shot through the viewfinder records no position.** The frame,
  centre and area are zero on the D300, D700, D750, D810, D850 and D780,
  and only the point fields are filled in: primary point 3 on the D810,
  34 on the D780, with 30 bits of its mask set (files 928, 2255, 895,
  2036, 1835, 3827). ExifTool's 0100 and 0101 tables call the position
  fields valid for contrast-detect AF only [P][1]; the live-view files
  above, with a detection method of 1 (contrast) on the D7000, D5600, D5
  and Coolpix A and 2 (hybrid) on the Z bodies, are the ones that fill them.
  **[F]** Mapping a point number to pixels needs a table of the body's
  points, which no file carries (ADR012).
- **The coordinates flag (byte 7 of 03xx and 04xx)** is 1 on every
  sample with a position and 0 on the D780 viewfinder shot and the Z5II
  auto-area files. **[F]** ExifTool notes that on 04xx either the mask
  or the position is set, not both [P][1]; the Z5II files (area mode 197,
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
  D5, in live view. [P][1] **[F]**
  The area size follows the mode: 291x323 on the Z 9, 1859x1452 for the
  Z f's wide area, 984x816 on the D5600. **[F]** raw-files does not
  report the mode.
- **Every AFInfo2 and CropHiSpeed value lies inside the MakerNote's
  declared length** on every checked file. **[F]**
- **In a camera JPEG**, the D5300's AFInfo2 is version 0100 with a zero
  frame and area, a shot through the viewfinder, and the D70 JPEG writes
  only AFInfo (0x0088): neither gives a point. **[F]** No public Nikon
  JPEG in the set records a position. A JPEG keeps a point only when its
  frame has the JPEG's aspect ratio, so that an export cropped to another
  shape, which may keep the camera's MakerNote, gives none
  ([ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)),
  except an export cut from a crop-mode JPEG, whose point is placed on
  the export's own pixels (below).
- **In a camera JPEG shot in a crop mode**, the maintainer's Z 9 records
  the raw file's AFInfo2 and CropHiSpeed, with the full 8256x5504 frame,
  at every image size, and no CropArea; its full-size JPEG is the raw
  file's CropArea (5392x3592, 5504x5504, 8256x4640), and its DX JPEGs at
  the smaller sizes are exactly 3:2. **[F]** raw-files places such a
  point on the JPEG's picture, as on CropArea in a raw file: a DX crop's
  scaled onto the JPEG's frame, a 1:1 or 16:9 crop's moved into the
  centred window of the frame that has the JPEG's shape
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
| Mirrorless, single point, pinpoint, wide area or 3D-tracking | a position | Z 6 3587, Z 9 5146 to 5148, Z f 6887, Z6III 7808 to 7810, 7812, 7813, Z50II 7767, 7770 | the point |
| Mirrorless on a DX crop | a position, scaled into the full frame | Z6III 7810; the maintainer's Z 9 shots, off-centre, one DX frame committed stripped | the point, scaled onto the crop |
| Mirrorless on an aspect crop (1:1, 16:9) | a position in the full frame | Z6III 7808, 7809, 7812, 7813, Z50II 7767, 7770; the maintainer's Z 9 shots, off-centre, one 1:1 frame committed stripped | the point, moved into the window |
| A DX crop at 1:1 or 16:9 on an FX body | unknown | none | the point, scaled; unchecked |
| Mirrorless small raw | a position in the full frame | Z 6 3587 | the point, in the full frame |
| Mirrorless, auto-area with only a mask | a mask | Z5II 7735, 7736 | none: needs a table of points |
| DSLR in live view | a position | D7000 961, D5600 1415, D5 2218, Coolpix A 2238 | the point |
| DSLR in live view on a DX or 1.2x crop | unknown: the cropped or the full frame | none | the point, by the crop rule; unchecked |
| A `0101` body in live view | a position, at ExifTool's offsets | D5 2218 | the point; checked |
| A `0301` body | a position, at the 03xx offsets | Z 30 5812, Z fc 8559 | the point, none on 8559, a manual-focus shot; checked |
| DSLR through the viewfinder | a point number and a mask | D300 928, D700 2255, D750 895, D810 2036, D850 1835, D780 3827 | none: needs a table of points; 928 and 1835 are manual-focus shots, which get none in any case |
| Nikon 1 | a point number and a mask | J1 2956 | none: needs a table of points |
| Bodies without AFInfo2 | AFInfo: a point number | D1H 5364, D70 2060, D100 2996, D200 8079, E5700 4456, P7800 1425 | none: needs a table of points |
| Manual focus | FocusMode `MANUAL`; a zero frame through the viewfinder | D300 928, D850 1835 | none |
| Manual focus on a Z body | FocusMode `MANUAL`; a position in AFInfo2 | Z fc 8559 | none; checked |
| A 1:1 or 16:9 crop on a small raw | unknown: CropArea counts stored pixels | none | none, since CropArea's kept side differs from the frame's |
| A DX crop on a small raw | unknown | none | the point, scaled onto CropArea in stored pixels; unchecked |
| A rotated (portrait) shot | a position in the landscape frame | Z 30 5812 | the point, in stored orientation; checked |
| Face detection | FaceDetect | none | not read |
| Camera JPEG through the viewfinder | a zero frame (`0100`) | the D5300 JPEG from Wikimedia Commons | none |
| Camera JPEG with a position | a position | none public | the point, when its frame has the JPEG's aspect ratio; unchecked on a public file |
| Camera JPEG in a crop mode (DX, 1:1, 16:9), at any image size | a position in the full frame, no CropArea | none public; the maintainer's Z 9 shots | the point, placed on the JPEG's picture; unchecked on a public file |
| Exported JPEG that keeps the camera's MakerNote | the camera's frame and position | none public | the point only when the frame has the export's aspect ratio |

### 8.7 Camera settings

How the photographer set the camera for the shot, which a viewer shows
in its information panel beside the exposure: the white balance, the
exposure program, the metering mode, the flash, the shutter, the image
area, the raw image size and the colour space, and with them the focal
length in 35 mm terms and the software that wrote the file. The Exif
tags record some of these on almost every body. Nikon records the rest
only in its MakerNote, a few of them only in blocks that are encrypted
or that have a record format of their own.

Every value below was read on the 35 NEF and NRW files of
[test-data.md](test-data.md#21-the-ci-set) and on the D70 and D5300
JPEGs of the [JPEG test data](../jpeg/test-data.md#2-the-sample-files)
on 2026-10-23, with a bounds-checked script written from these pages
**[F]**; [test-data.md](test-data.md#24-files-checked-for-the-camera-settings)
lists which file records which value. The meaning of each code is
ExifTool's, read in its published tag documentation [P][1][2]. ExifTool
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
| Colour temperature | none | 0x004F ColorTemperatureAuto | the D5, D850, D780 and every Z body (19 files) |
| White balance fine-tuning | none | 0x003F, else 0x000B, both WhiteBalanceFineTune | 0x000B on 21 NEF files and both JPEGs, 0x003F on 22 files from the D810 on; neither on the E5700 |
| Shutter | none | 0x0034 ShutterMode | the Nikon 1 J1, D850, D780 and every Z body (19 files) |
| Image area | none | 0x001B CropHiSpeed, its first value | the D200 and every later body but the Coolpix P7800 (28 NEF files and the D5300 JPEG) |
| Raw image size | none | 0x003E ImageSizeRAW | the D750, D810, D5600 and every later body (22 files) |
| Colour space | Exif 0xA001 ColorSpace | 0x001E ColorSpace | Exif on no NEF file and on both JPEGs; 0x001E on the D200 and every later body but the Coolpix P7800 (28 NEF files and the D5300 JPEG) |
| Focal length in 35 mm terms | Exif 0xA405 FocalLengthIn35mmFilm | none | Exif on every file but the D1H's, the D1X's and the E5700's |
| Software | IFD0 0x0131 Software | ShotInfo's FirmwareVersion, encrypted | IFD0 on every file |

**Exposure program and metering mode.** Both are Exif tags on every
checked file, the D1H's included **[F]**, with Exif's codes [P][2].
ExposureProgram is 1 (manual) on 11 files, 2 (normal program) on five
and the D70 JPEG, 3 (aperture priority) on 11, 4 (shutter priority) on
the D700 (file 2255), and 0, "not defined", on the seven Z5II and Z6III
files (7735, 7736, 7808 to 7810, 7812, 7813) and the D5300 JPEG: a
recent body writes 0 too. MeteringMode is 5 (multi-segment) on 24 NEF
files and both JPEGs, 2 (centre-weighted average) on nine files (928,
1786, 2238, 5146 to 5148, 7767, 7770, 8079), 3 (spot) on the D100
(file 2996) and 255 ("other") on the D850 (file 1835) **[F]**, perhaps
for its highlight-weighted metering, which Exif has no code for [I].

**Flash.** Exif's Flash is a set of bits: bit 0 says whether the flash
fired, bits 1 and 2 whether its return light was detected, bits 3 and 4
its mode (1 compulsory firing, 2 compulsory suppression, 3 auto), bit 5
that the camera has no flash function, bit 6 red-eye reduction [P][2].
The checked files hold 0 (no flash, no mode; 23 NEF files and the D70
JPEG), 16 (not fired, mode 2; files 961, 1415, 1425, 4456, 7767, 7770
and the D5300 JPEG), 15 (fired in mode 1, return light detected; the
D750 and D810, files 895 and 2036) and 7 (fired, return light detected,
no mode; the D100, file 2996) **[F]**. The MakerNote's 0x0087 FlashMode
(0 did not fire, 7 fired with an external flash, 9 fired in TTL mode,
among others [P][1]) agrees wherever both exist: 7 on files 895 and
2036, 9 on file 2996, 0 elsewhere **[F]**. On the D1H and D1X files it
is the only record of the flash, and it says 0 even on the D1X file
whose white balance is set to `FLASH` (file 1786) **[F]**.

**White balance.** The photographer chooses auto white balance, a kind
of light (daylight, shade, incandescent and so on), a colour
temperature, or a preset measured from a grey card, and may shift the
result (the fine-tuning below). Exif records only whether it was auto
(0xA403: 0 auto, 1 manual) and, in a tag of its own, the kind of light
(0x9208 LightSource) [P][2]. Nikon's 0x0005 records the setting itself,
as 13 bytes of ASCII: an upper-case word padded with spaces to 12
characters, then a NUL **[F]**. ExifTool documents it as a string and
lists no values [P][1]. The checked files hold:

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
1 under `SUNNY` (LightSource 9, "fine weather") and `SHADE`
(LightSource 11, "shade") **[F]**. The D1H, D1X and E5700 write no Exif
white balance, so 0x0005 is their only record of it. The texts for an
incandescent, fluorescent, colour-temperature or preset setting, and
for the later bodies' third auto setting, are on no checked file
([section 11](#11-open-questions)).

**Colour temperature.** 0x004F ColorTemperatureAuto is one SHORT, in
kelvin [P][1], on the D5, D850, D780 and every Z body (19 files). Under
an `AUTO` setting it holds the temperature auto white balance chose:
3050 to 7000 on 17 files. Under `SUNNY` it is 0 (the D850 and the Z fc,
files 1835 and 8559) **[F]**: 0 means it was not recorded. What it
holds under a colour-temperature or preset setting is on no checked
file. No other plain tag, Exif or MakerNote, records a colour
temperature on the older bodies: ExifTool reports none there
**[S]**.

**White balance fine-tuning.** A shift of the white balance along two
axes, amber to blue and green to magenta. Nikon writes it in two tags,
both called WhiteBalanceFineTune [P][1]:

- 0x000B, signed SHORTs: one value on the D1H, D1X, D70, D100 and D200
  (files 5364, 1785, 1786, 2060, 2996, 8079) and the D70 JPEG; two on
  15 files from the D300 to the Z fc, the Coolpix P7800 and A and the
  Nikon 1 J1 included, and on the D5300 JPEG **[F]**.
- 0x003F, two SRATIONALs, on the 22 files from the D810 on. The Z 9,
  Z f, Z5II, Z50II and Z6III write it alone, without 0x000B **[F]**.

Every checked value is zero: `0` or `0, 0` in 0x000B, `0/4, 0/4` in
0x003F **[F]**. Which axis comes first, which sign means which colour,
and the size of a step are documented nowhere and shown by no file
([section 11](#11-open-questions)); the denominator 4 suggests quarter
steps [I], and on the maintainer's own D7200 files 0x003F holds 0x000B's
value in quarters (below). The E5700 writes neither tag **[F]**.

**Shutter.** Which shutter took the picture: the mechanical one, the
electronic front curtain (an electronic start of the exposure and a
mechanical end), or a fully electronic one. 0x0034 ShutterMode is one
SHORT, with ExifTool's codes [P][1]: 0 mechanical, 16 electronic, 48
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
ExifTool reports as "Auto" [P][S]: the setting alone, which 0x0034
also gives.

**Image area.** The part of the sensor the picture uses: the whole
sensor, a DX crop of an FX sensor (the APS-C size), or an aspect crop
such as 1:1 or 16:9. The first value of 0x001B CropHiSpeed records it,
with the codes of [section 8.4](#84-image-dimensions) [P][1]. The
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
D300 (DX) both write it. And code 9, which ExifTool names for movies,
is a DX body's 16:9 still too. The bodies before the D200 and the
Coolpix E5700 and P7800 write no CropHiSpeed (seven NEF files and the
D70 JPEG). ShotInfo holds an ImageArea in the part ExifTool calls
IntervalInfo, encrypted, which ExifTool decodes on the Z 6, Z 30, Z f
and Z fc **[S]**: "FX" on the Z 30 and Z f, "DX" on the Z 6 and Z fc,
so it contradicts CropHiSpeed on the Z 6 (FX uncropped, file 3587) and
on the Z 30, a DX body (file 5812).

**Raw image size.** Whether the camera was set to its large, medium or
small raw size. 0x003E ImageSizeRAW is one SHORT [F] with ExifTool's
codes 1 large, 2 medium, 3 small [P][1]. It is 3 on the three small-raw
samples, the D850, D810 and Z 6 (files 1835, 2036, 3587), and 1 on the
19 other files that have it, from the D810 on **[F]**. No checked file
is a medium raw. The size in pixels is CropArea's
([section 8.4](#84-image-dimensions)).

**Colour space.** The colour space the camera was set to for its JPEG
output, sRGB or Adobe RGB. No checked NEF file has the Exif ColorSpace
(0xA001) or an InteropIndex; both JPEGs have ColorSpace 1, sRGB
**[F]**. 0x001E ColorSpace is one SHORT with ExifTool's codes 1 sRGB,
2 Adobe RGB, 4 BT.2100 [P][1]: 2 on the Coolpix A, D700, Nikon 1 J1,
Z 30 and D200 (files 2238, 2255, 2956, 5812, 8079), 1 on the other 23
NEF files that have it and on the D5300 JPEG **[F]**. The NEF files
of the bodies before the D200 and of the Coolpix E5700 and P7800
record a colour space nowhere in plain form (seven files).

**Focal length in 35 mm terms.** Exif 0xA405 FocalLengthIn35mmFilm
[P][2], one SHORT in millimetres, is on every file but the D1H's, the
D1X's and the E5700's, and is never 0 on a Nikon file **[F]**.

**Software and firmware.** IFD0's Software (0x0131) is on every
checked file, and on all of them it is the body's firmware version
**[F]**: `Ver.` and a number, with a trailing space on the DSLRs from
the D1H to the D850 and on the Coolpix A (`Ver.1.10 `), without one on
the Nikon 1 J1 (`Ver.1.40`), with two digits before the point on the
D780 and the Z bodies (`Ver.01.00`, `Ver.02.00` on the Z 6, `Ver.01.70`
on the Z fc); the Coolpix E5700 and P7800 write `E5700v1.1` and
`COOLPIX P7800  V1.0`, with two spaces. No checked file carries an
editor's name; on the maintainer's own JPEGs that a program rewrote, it
names that program (below). ShotInfo (0x0091) has a FirmwareVersion of its own: on
none of the 36 files that have ShotInfo does it hold readable text
where ExifTool documents it, and ExifTool decodes it from the encrypted
block ([section 7](#7-lensdata-and-shotinfo)) [P][1]. Its decoded value
**[S]** carries Software's number, on some bodies with a letter or a revision
added (`1.10B` on the D300, `01.00.k0` on the Z 9), on every body where
ExifTool reads it, except the D810: `Ver.1.12` in Software, `1.11d` in ShotInfo
(file 2036). No file can say which of the two is right; Software is the
plain one.

0x0051 and 0x0056 begin with ASCII digits on the Z bodies that write
them: 0x0051 with eight (`01000400` on the Z 6, `01010500` on the Z 9,
`01020501` on the Z f, Z5II, Z50II and Z6III), 0x0056 with four (`0100`
on the same bodies but the Z 6) **[F]**. ExifTool calls them
FirmwareVersion51 and FirmwareVersion56 [P][1]. They are not the body's
firmware: beside them the Z 6's Software is `Ver.02.00` and the Z 9's
`Ver.01.00`, and four bodies share `01020501` **[F]**. What they number
is not known; byte 10 of 0x0051 is the compression
([section 2](#2-key-tags)).

**On the maintainer's own files.** The maintainer's own raw files and
camera JPEGs of six Nikon DSLRs, the D300s, D3200, D3300, D600, D7100
and D7200, were read the same way on 2026-10-24 and are given here as
counts only
([test-data.md](test-data.md#24-files-checked-for-the-camera-settings)).
They add what the public samples lack **[F]**:

- **More white balance texts:** `CLOUDY` (D300s, D7200),
  `INCANDESCENT` (D7200), `PRESET` (D3200) and `COOL WHT FL` (D300s),
  beside `AUTO`, `AUTO1`, `SUNNY` and `SHADE`. Each text comes with one
  pair of Exif values: `SUNNY` with WhiteBalance 1 and LightSource 9,
  `CLOUDY` 1 and 10, `SHADE` 1 and 11, `INCANDESCENT` 1 and 3,
  `COOL WHT FL` 1 and 14 (cool white fluorescent), `PRESET` 1 and 0,
  and `AUTO` and `AUTO1` with 0 and 0 (13 D3200 files with `AUTO` have
  no Exif white balance). A text like `COOL WHT FL` could not have been
  guessed from the others.
- **Fine-tuning that is not zero:** 0x000B is `1, 0` on two D300s files,
  and `-6, 0` on 42 D7200 files and `-3, 0` on two. On every D7200 file
  with a Nikon MakerNote, 11,094 with one rewritten JPEG among them,
  0x003F holds 0x000B's two values as fractions of four: `-24/4, 0/4` beside `-6, 0`. Which colour a negative value
  means is still shown by no file.
- **The colour space:** none of these NEF files has an Exif ColorSpace
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

## 9. Robustness pitfalls

The raw-files rule is that a malformed file returns an error and never panics.
The points specific to the MakerNote:

1. **Relocated MakerNotes.** Tools that move the MakerNote without fixing
   offsets leave the headerless and short layouts pointing at the wrong
   place. ExifTool detects this with a heuristic (value offsets compared
   with the expected position four bytes after the IFD) and repairs only on
   request. Bounds-check every offset and length against the MakerNote and
   the file, skip bad entries rather than abort the IFD, and do not repair
   in a first version. [P][S][4][19] An E885 JPEG that an editor turned
   upright is such a file: its headerless MakerNote moved 342 bytes, and
   its offsets did not ([section 1](#layout-details)). **[F]** raw-files
   requires the PreviewIFD's JPEG, as declared, to lie inside the
   MakerNote too. The Coolpix P7800 declares its MakerNote 2,816 bytes
   long, ending at byte 4546, and its 640x480 MakerNote JPEG starts at
   byte 4546, just past it, so the reader does not read it and the file
   gives its 160x120 and 4000x3000 previews (file 1425); on the E5700 and
   the D100, the other `02 00` bodies, the declared length includes the
   JPEG (files 4456, 2996). **[F]** Early Coolpix JPEGs with a headerless
   MakerNote go further: the E885, E995, E2500 and E5000, on their
   earlier firmware, put their PreviewIFD's 320x240 JPEG 12,527 bytes
   past the MakerNote's end, inside the Exif segment, so the reader does
   not read it either ([section 1](#in-a-jpeg-file)). **[F]** Whether the
   rule should let such a JPEG through is still to be decided: the issue
   that asks for reading a MakerNote preview just past the MakerNote, the
   P7800's, is open (#18).
2. **Truncated MakerNotes.** DNG-converted NEFs can cut the MakerNote short
   (reported as "bad offset" errors on later tags). [S][21]
3. **Capture NX.** Foreign brands, changed byte order and fixed-order
   sub-blocks ([section 1](#1-finding-the-makernote)). [P][3][4]
4. **Inner versus outer byte order.** Always take the inner header's order.
   rawler treats a TIFF versus MakerNote mismatch as an error, yet it
   occurs in real files: tolerate it for metadata, reject it for decoding.
   [P][I]
5. **Type and count mismatches.** Tag 0x0001 is a string on some bodies and
   binary on others, 0x0002 writes four zero bytes for "Lo" ISO, and the
   NEFBitDepth and ImageBoundary types vary by model. Read from the raw
   bytes at the stated count; cap counts. [P][3]
6. **Loops and sizes.** LibRaw caps the MakerNote at 1000 entries and
   ignores entries whose declared length is implausible for the file;
   apply those caps plus a visited-offset set for PreviewIFD and SubIFD
   recursion. Cap each allocation at the bytes actually remaining. [P][7]
7. **Version-gated decryption.** Decrypt only when the version prefix, the
   minimum length and both key inputs are present; otherwise skip and
   continue. Never unwrap. [I]
8. **Odd bodies.** D1 (big-endian headerless MakerNote); D100 (own
   uncompressed path, NEFCompression 2 with a conflicting TIFF tag); D200
   (no next-IFD pointer); Coolpix NRW files (no 0x0096); D3 and D810
   (14-bit versus 16-bit and mislabelled uncompressed data); D780, Z 6 and
   Z 7 (curve header 0x44 0x40, bit depth two bits lower than labelled);
   Z bodies (compression in 0x0051, at byte 10 on the Z 9 and later,
   but in 0x0093 on the Z fc and Z 30, which write no 0x0051);
   Z 9 and later (HE and HE* undecodable).
   [P][1][3][8][9]
9. **Nikon Scan files.** Scanner "NEFs" are TIFF RGB with a Nikon Scan IFD
   (tag 0x0E10); reject anything whose raw IFD is not CFA
   (PhotometricInterpretation 32803). [P][I][3]

## 10. Recommendation for raw-files

raw-files decided this in
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md),
which follows the recommendation below with these changes. From the
samples: the MakerNote also gives the serial number, the ISO and the lens
range when the Exif tags lack them
([section 8.5](#85-serial-number-iso-and-lens)), and WorldTime gives the
offset from UTC as a plain offset, not a labelled home zone
([section 8.2](#82-capture-date-and-time-zone)). And by choice: text is
trimmed of spaces at both ends, not only trailing ones as in
[section 8.3](#83-make-and-model), and a value over 255 bytes is taken
for damage; CropArea is used without the 0x001B cross-check of item 2;
and of the layouts items 2 and 6 cover, the embedded-TIFF and the
headerless ones are read, not the short-header one, whose byte order
the sources disagree on and the public files that have it, all
little-endian, cannot settle ([section 1](#in-a-jpeg-file)): the D1
family gets its MakerNote preview, its ISO and its lens range from the
headerless MakerNote (file 5364), and the earliest Coolpix the Exif
fields only. Every Nikon MakerNote is read only under a Make that starts
with `NIKON`, whatever its first bytes: the format's reader decides by
IFD0's Make before the MakerNote is looked at, unlike the "Check the
signature before Make" of section 1, so a Nikon-format MakerNote that
Capture NX wrote into another brand's file is not read.

The focus point is decided in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md):
the MakerNote's AFInfo2 gives it where it records a position, at the
versions of [section 8.6](#86-focus-point), scaled onto CropArea on a
DX crop and moved into the window on an aspect crop; a FocusMode of
`MANUAL` gives none, and a point number gives none until raw-files has
a table of the body's points.

For the camera settings of [section 8.7](#87-camera-settings), read the
Exif tags first and the plain MakerNote tags where Exif has no value,
and decrypt nothing [I]:

- the exposure program, the metering mode, the flash and the focal
  length in 35 mm terms from Exif alone. Every body that lacks them in
  Exif lacks them in plain MakerNote tags too, except the D1 family's
  flash, whose 0x0087 says "did not fire" even on a shot whose white
  balance is set to flash;
- the white balance from 0x0005, which every body writes and which is
  finer than Exif's auto or manual, by a table of the texts files are
  known to hold, since a text like `COOL WHT FL` cannot be guessed,
  else from Exif's WhiteBalance and LightSource;
- the colour temperature from 0x004F when it is above zero;
- the fine-tuning from 0x003F, else from 0x000B;
- the shutter from 0x0034;
- the image area from CropHiSpeed's first value, with 0 giving none,
  since it does not say which sensor the body has, and 4 (3:2) and the
  movie codes too, except 9 and 10, which DX bodies write on stills: 9
  for the APS-C 16:9 crop, 10 for the 1.3x crop;
- the raw image size from 0x003E;
- the colour space from Exif whenever it records ColorSpace, 65535
  without `R03` included, and from 0x001E only when it records none,
  since a program that rewrites a JPEG can leave the camera's 0x001E
  behind;
- the software from IFD0.

ShotInfo's firmware version and image area, NikonSettings, and the
digits that 0x0051 and 0x0056 start with stay unread: the first two are
encrypted, and the image area contradicts CropHiSpeed on two of the four
files ExifTool decodes it on; NikonSettings repeats the setting that
0x0034 records; and 0x0051 and 0x0056 do not hold the body's firmware.

For milestone 3, keep the MakerNote out of the critical path.

1. **Fields from standard tags only:** orientation, capture date, sub-seconds,
   offset from UTC, make and model, and the stored sensor size all come from
   IFD0, its SubIFDs and the Exif IFD (IFD0's ImageWidth is the thumbnail;
   the sensor size is in the raw SubIFD). This works for every NEF regardless of how the
   MakerNote is laid out. [P]
2. **Parse the MakerNote only for two things:** the crop rectangle (0x0045,
   with 0x001B as a cross-check) and, optionally, WorldTime (0x0024) as a
   time zone fallback. Implement the byte-pattern detection in
   [section 1](#1-finding-the-makernote), bounds-check every read, and
   return "absent" rather than an error for a MakerNote that does not parse.
   [I]
3. **Do not implement decryption in milestone 3.** The fields above need no
   key, the substitution tables have no settled provenance yet
   ([section 11](#11-open-questions)), and white balance for every body of
   interest is available in plain form (0x000C). Defer 0x0097, 0x0098 and
   0x0091 to milestone 6, and only if older-body white balance is required.
   [I]
4. **Output size policy:** report the CropArea rectangle if present and
   plausible (inside the raw IFD dimensions); otherwise report the stored
   raw IFD dimensions and say in the API documentation that they may include
   border pixels. Do not guess from Exif PixelXDimension. [I]
5. **Time policy:** prefer 0x9011 when it parses; otherwise offer the
   WorldTime zone clearly labelled as a home-zone fallback, or none. [I]
6. **Tests:** cover all three layouts, an inner byte order different from
   the outer one, a truncated MakerNote, an unset clock, a 0x0045 outside the
   raw dimensions, and NUL-terminated and space-padded Make/Model strings.
   Use files from raw.pixls.us (check each file's licence) for the main
   cases. [I]

## 11. Open questions

1. **Provenance of the decryption tables.** The two 256-byte tables are
   published in GPL, LGPL, CDDL and Artistic projects and in dcraw, whose
   header states that all of its code outside a single restricted (Foveon)
   block is free for all uses; dcraw's licence is nevertheless described as
   ambiguous for the whole file elsewhere on this page. That is an input to
   the decision, not legal advice. raw-files needs an independent derivation,
   a source it can use, or a recorded legal decision, before milestone 6
   touches encrypted blocks. [10]
2. **Non-numeric serial handling.** Which of the four conventions in
   [section 5.1](#51-non-numeric-serial-numbers) is correct for a real
   lettered serial. Checking a D50-family and a D200 sample would settle it.
3. **ColorBalance version 08xx (Z bodies).** Whether any Z body lacks
   0x000C, and whether the encrypted block holds anything useful.
4. **BlackLevel (0x003D) channel order.** LibRaw reads the four values in
   fixed colour order R, G1, G2, B regardless of the CFA layout. rawspeed and
   rawler read them by position after assuming an RGGB pattern. The two agree
   only on RGGB. In rawspeed's camera list every non-RGGB Nikon is a
   pre-2010 body, so there may be no practical difference, but which body
   first wrote the tag is unknown. [P][7][8][9][17]
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
   `II` and `MM`. **[F]** [P][11]
9. **Bodies that write NEFCompression 5 and 6.** Labels are known, bodies
    are not.
10. **The 0101 position.** Answered: ExifTool's offsets (70 to 82) hold
    the frame, the centre, the area and the in-focus byte on the D5's
    live-view file: 5568x3712, (3077, 2427), 588x500, in focus (file 2218)
    ([section 8.6](#86-focus-point)). **[F]**
11. **Version 0301.** Answered: it has the 03xx layout, 60 bytes with the
    position at the 0300 offsets, on the Z 30 and the Z fc (files 5812,
    8559). **[F]**
12. **A DSLR in live view on a DX or 1.2x crop.** Whether AFInfo2's frame
    is then the cropped output or the full one. The crop rule needs a
    CropArea, which no checked DSLR before the D5 writes; a live-view
    crop file from a D5, D750, D810 or D850 would show it.
13. **Manual focus on a Z body.** Answered: the Z fc writes a position
    in AFInfo2 under FocusMode `MANUAL`, centre (2520, 1856) and area
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
    position needs a table of each body's points, and a decision on where
    such tables may come from
    ([ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md)).
    ExifTool's tables give the order of the points by number and, for
    the 11-point layout only, the bit order of the mask. [P][1]
17. **FaceDetect.** Which bodies write 0x0021; none of the 35 checked
    files does.
18. **Crops that cut both sides and change the aspect**, such as a DX
    crop at 1:1 or 16:9 on an FX body: whether they are scaled like DX or
    windows like 1:1. The rule scales them; no sample shows one.
19. **Where an aspect crop's window lies.** raw-files centres it, at
    half the difference between the recorded frame and CropArea.
    CropHiSpeed's own offsets, in stored pixels, differ from that by up to
    eight pixels (936 against 928 on the Z50II's 1:1 file, 312 against
    316 on the Z6III's 16:9 file); an off-centre shot near an edge of the
    window, on one of those bodies, would show which is right.
20. **A crop on a small raw.** Whether a Z body records a crop mode on a
    small raw, and how. The rule gives a 1:1 or 16:9 crop on one no focus
    point, since its CropArea counts stored pixels while AFInfo2's frame
    counts full-size ones, and scales a DX crop onto that CropArea.
21. **The other white balance texts.** The public files show 0x0005 as
    `AUTO`, `AUTO0`, `AUTO1`, `SUNNY`, `FLASH` and `SHADE`, and the
    maintainer's own files add `CLOUDY`, `INCANDESCENT`, `PRESET` and
    `COOL WHT FL` ([section 8.7](#87-camera-settings)). The texts for
    the other fluorescent lamps, a colour temperature, underwater and
    the later bodies' third auto setting, on the Z bodies above all,
    need a shot with each.
22. **The colour temperature under a manual setting.** 0x004F is 0
    under `SUNNY`; whether it holds the chosen temperature when the
    photographer sets one in kelvin is on no checked file.
23. **The axes of the fine-tuning.** Which value of 0x000B and 0x003F is
    amber to blue and which green to magenta, which sign is which
    colour, and the size of a step: every public value is zero, and the
    maintainer's non-zero ones (`-6, 0` on a D7200) come with no record
    of the shift that was set. Shots with a known shift along each axis
    would show them.
24. **The shutter of the older DSLRs.** No DSLR older than the D850
    writes 0x0034, nor any of the maintainer's six DSLRs, and none
    records the shutter in a block ExifTool decodes. Whether a DSLR in live view records it anywhere is open;
    so is which
    bodies write the codes 48, 64 and 96, which no checked file holds.
25. **CropHiSpeed's other codes.** 3 (5:4), 4 (3:2), 8 (2.7x) and the
    movie codes other than 9 and 10 are on no checked file; 1 (1.3x) and 10
    are only on the maintainer's D7200 files, where 10, which ExifTool
    names "1.3x movie crop", is on still NEF files and JPEGs. 9, which ExifTool names
    for movies, is a 16:9 still on the Z50II (file 7770) and on one of
    the maintainer's D7100 NEF files and one JPEG, so on a DX body it
    marks a 16:9 still too.
26. **A medium raw.** 0x003E is 1 or 3 on the checked files; no medium
    raw was checked.
27. **The D810's firmware.** Its Software says 1.12 and its encrypted
    ShotInfo, as ExifTool decodes it, 1.11d (file 2036). Which one a
    firmware update changes is open.
28. **What 0x0051 and 0x0056 number.** Their leading digits are not the
    body's firmware ([section 8.7](#87-camera-settings)).
29. **ExposureProgram 0.** The Z5II and Z6III files, the D5300 JPEG
    and many of the maintainer's own D3200, D3300, D7100, D7200 and D600
    files record "not defined"; whether another tag records the program
    or scene mode those shots used is open.

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
- [22] raw.pixls.us, the catalogue's exiv2 metadata listing of each file
  (read October 2026) — https://raw.pixls.us/
