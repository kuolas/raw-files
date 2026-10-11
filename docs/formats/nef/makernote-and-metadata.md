# Nikon MakerNote and viewer metadata

This page describes the Nikon MakerNote found in NEF files: its three
layouts, the tags that matter, the encrypted blocks and how they are
decrypted, and where a viewer finds orientation, capture date, make, model
and image size. It serves milestone 3 (basic metadata) and, for the
decoding-related tags, milestone 4 (raw data).

See [the NEF overview](README.md). The TIFF container and the preview JPEGs
are in [container-and-previews.md](container-and-previews.md); decoding of
the sensor data is in [raw-data.md](raw-data.md).

Nikon publishes no specification for any of this. Everything below comes
from the Exif and TIFF standards, the tag documentation of ExifTool and
exiv2, and the source of existing decoders, read as references only (their
licences do not allow copying code; see [Licences](#licences)). Claims carry
a confidence tag: **[P]** primary (specification, tag documentation or
implementation source), **[S]** secondary (write-ups, forum posts, sample
listings), **[I]** inferred. Byte offsets and version numbers taken from
implementation sources were read through summaries; verify them against the
source, and against sample files, before hard-coding them.

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
| Headerless | none | MakerNote + 0 | the outer TIFF header | the outer file's (the D1 is big-endian) | E990, D1, D1H, D1X |
| Short header | `Nikon\0` `01 00` (8 bytes) | MakerNote + 8 | the outer TIFF header | exiv2 uses the outer order; ExifTool says little-endian | E700 to E950 |
| Embedded TIFF header | `Nikon\0`, version `02 10` or `02 00`, `00 00`, then a TIFF header (`II*\0` or `MM\0*` and a 4-byte offset) (18 bytes) | MakerNote + 10 + the inner IFD offset (normally 8, so + 18) | MakerNote + 10, the inner TIFF header | the inner header's, independent of the file's | D70, D100, D200 and later DSLRs, every Z body |

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
  embedded layout survives being moved as a block. [P][5]

## 2. Key tags

IDs are for the embedded-TIFF layout. Types are as documented by ExifTool;
read integers from the bytes at the declared count rather than trusting the
declared type (see [section 9](#9-robustness-pitfalls)). "M" is the roadmap
milestone that needs the tag. [P][1]

| ID | Name | Type | Meaning | M |
|---|---|---|---|---|
| 0x0001 | MakerNoteVersion | undef[4] | ASCII digits on some bodies, binary on others; do not rely on it | none |
| 0x000C | WB_RBLevels | rational[4] | as-shot white-balance multipliers; LibRaw reads them as R, B, then two greens (ExifTool leaves the last two unidentified) | 4 |
| 0x0011 | PreviewIFD | IFD | sub-IFD with a small JPEG preview; offsets relative to MakerNote + 10 | 1 |
| 0x001B | CropHiSpeed | u16[7] | crop mode and geometry; field order in [section 8.4](#84-image-dimensions) | 3, 4 |
| 0x001D | SerialNumber | string | decryption key input | 3, 4 |
| 0x0024 | WorldTime | undef | home time zone fallback; layout in [section 8.2](#82-capture-date-and-time-zone) | 3 |
| 0x0034 | ShutterMode | u16 | mechanical, electronic, EFCS | none |
| 0x0037 | MechanicalShutterCount | u32 | newer bodies; mechanical frames only | none |
| 0x003D | BlackLevel | u16[4] | per-channel black level, recorded at 14-bit scale | 4 |
| 0x003E | ImageSizeRAW | u16 | 1 large, 2 medium, 3 small | 4 |
| 0x0045 | CropArea | u16[4] | left, top, width, height of the output rectangle | 3, 4 |
| 0x0051 | (unnamed, Z 9 and later) | undef | NEFCompression as a little-endian u16 at byte 10 | 4 |
| 0x0091 | ShotInfo | undef | model-specific; encrypted for versions 02xx and 08xx | none |
| 0x0093 | NEFCompression | u16 | compression hint | 4 |
| 0x0096 | NEFLinearizationTable | undef | curve and predictor seeds; unencrypted | 4 |
| 0x008C | (older curve) | undef | fallback for 0x0096 | 4 |
| 0x0097 | ColorBalance | undef | white balance on older bodies; encrypted from version 0200 | 4 |
| 0x0098 | LensData | undef | lens description; encrypted from version 0201 | none |
| 0x00A7 | ShutterCount | u32 | decryption key input | 3, 4 |
| 0x0E01 | NikonCaptureData | undef | Capture NX edit data (optional) | none |
| 0x0E22 | NEFBitDepth | u16[4] | secondary bit-depth source | 4 |

Notes:

- 0x00A7 on newer bodies is the combined mechanical and electronic count;
  0x0037 is mechanical-only. The decryption key uses 0x00A7. [P][1]
- 0x0014 holds Coolpix colour data (2560-byte or 1280-byte variants, or an
  "NRW" block on P-series cameras) and is irrelevant to NEF DSLRs. [P][1]
- Capture NX data (0x0E01) is a sequence of records that can carry a white
  balance adjustment, a custom white balance and a flip; one implementation
  walks at most 1024 records. Ignore it unless a need arises. [P][7]
- The curve in 0x0096 (and its fallback 0x008C) is decoder input; see
  [raw-data.md](raw-data.md).

## 3. NEFCompression and bit depth

NEFCompression (0x0093; on Z 9 and later bodies the value is at byte 10 of
0x0051) takes these values. [P][1][3][9]

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
| 0x0097 ColorBalance | version 02xx (0100, 0102, 0103 are plain) | 4 for versions 0205 and 0219 to 0224 (0220 disputed: byte 4 or 284, see [open question 3](#11-open-questions)); 284 for the other 02xx versions |
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
from milestone 4.

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
MakerNote. IFD0's own ImageWidth and ImageLength describe the thumbnail
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
| Fallback zone | MakerNote 0x0024 WorldTime | the camera's home zone setting |

Also present: 0x9004 DateTimeDigitized and IFD0 0x0132 DateTime (the last
modification time). [P][2]

An unset camera clock yields a blank or space-filled string (spaces where the digits would be); return "none" for the date rather than an error. [I]

**OffsetTimeOriginal writers.** Only the Z 9 is confirmed to write the
OffsetTime tags (JPEG samples listing all three, Exif versions 0231 and
0232). Z 8, Z 6III, Z f, D6, D780 and Z 6II are unchecked, and older bodies
are reported to rely on the MakerNote zone instead. [S][15] Treat 0x9011 as
optional.

**WorldTime layout (0x0024).** The data is an array of bytes with this
layout, per ExifTool's definition: [P][3][1]

| Offset | Size | Field |
|---|---|---|
| 0 | i16 | time zone in minutes from UTC (60 means UTC+01:00) |
| 2 | u8 | daylight savings: 0 no, 1 yes |
| 3 | u8 | date display format: 0 Y/M/D, 1 M/D/Y, 2 D/M/Y |

The i16 follows the MakerNote's byte order [I]; Capture NX can flip it, and
a byte-swapped value then reads as an absurd offset (a bug report shows +256
hours for the swapped form of 60 minutes). [S][13] Because it is the home
zone and not necessarily the zone at capture, use it only as a fallback when
OffsetTimeOriginal is absent, and reject implausible values (beyond
roughly +-14 hours). [I]

### 8.3 Make and model

IFD0 0x010F and 0x0110. Typical values: Make "NIKON CORPORATION" and Model
"NIKON D90" on DSLRs, Make "NIKON" and a Model such as "COOLPIX P7000" on
compacts. [S][14]

In the one NEF dump inspected the strings were NUL-terminated with the count
including the NUL and no space padding (Make 18 bytes, Model 10 bytes for a
D40). [P][14] The Exif standard allows space-filled "unknown" values, and
ExifTool and LibRaw both strip trailing whitespace. So: cut at the first NUL,
then trim trailing ASCII whitespace. [P][2][7] No D850 or Z-series dump was
inspected. Compare model strings after trimming, because model checks
elsewhere ("D50" suffix, a "Z" at index 6) assume trimmed text. [I]

### 8.4 Image dimensions

The raw IFD's ImageWidth and ImageLength (0x0100, 0x0101) give the stored
sensor array, which includes masked and border pixels, so it is usually
larger than the image a viewer shows. NEF has no DNG-style DefaultCropOrigin,
DefaultCropSize or ActiveArea; those tags appear in DNG files only. [P][2][9]
Sources for the output size, in order: [P][7][9]

1. **CropArea (0x0045)**: four u16 as left, top, width, height in raw
   pixels. rawler uses it as the crop rectangle; LibRaw stores it as an
   inset crop. Whether every body writes it, especially before the D3, is
   unknown. [P][I]
2. A per-body table (as rawspeed's `cameras.xml` or rawler's camera data).
   Not suitable for a small library; raw-files should not carry one for
   milestone 3. [I]
3. Exif PixelXDimension and PixelYDimension (0xA002, 0xA003): these exist,
   but Nikon's values describe the in-camera JPEG size and may differ from
   the raw crop. [I]

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

## 9. Robustness pitfalls

The raw-files rule is that a malformed file returns an error and never panics.
The points specific to the MakerNote:

1. **Relocated MakerNotes.** Tools that move the MakerNote without fixing
   offsets leave the headerless and short layouts pointing at the wrong
   place. ExifTool detects this with a heuristic (value offsets compared
   with the expected position four bytes after the IFD) and repairs only on
   request. Bounds-check every offset and length against the MakerNote and
   the file, skip bad entries rather than abort the IFD, and do not repair
   in a first version. [P][S][4][19]
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
   Z 9 and later (compression in 0x0051; HE and HE* undecodable).
   [P][1][3][8][9]
9. **Nikon Scan files.** Scanner "NEFs" are TIFF RGB with a Nikon Scan IFD
   (tag 0x0E10); reject anything whose raw IFD is not CFA
   (PhotometricInterpretation 32803). [P][I][3]

## 10. Recommendation for raw-files

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
   key, the substitution tables have no licence-clean source yet
   ([section 11](#11-open-questions)), and white balance for every body of
   interest is available in plain form (0x000C). Defer 0x0097, 0x0098 and
   0x0091 to milestone 4, and only if older-body white balance is required.
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

1. **Licence-clean decryption tables.** The two 256-byte tables are only
   published in GPL, LGPL, CDDL or Artistic projects. raw-files needs either an
   independent derivation, a permissively licensed source, or a legal
   decision, before milestone 4 touches encrypted blocks.
2. **Non-numeric serial handling.** Which of the four conventions in
   [section 5.1](#51-non-numeric-serial-numbers) is correct for a real
   lettered serial. Checking a D50-family and a D200 sample would settle it.
3. **ColorBalance decrypt start for version 0220.** The sources read for
   this page list the 0219 to 0224 range as starting at byte 4 in one place
   and 0220 as starting at byte 284 in another. Verify against ExifTool's
   source.
4. **ColorBalance version 08xx (Z bodies).** Whether any Z body lacks
   0x000C, and whether the encrypted block holds anything useful.
5. **BlackLevel (0x003D) channel order.** LibRaw reads the four values in
   fixed colour order R, G1, G2, B regardless of the CFA layout. rawspeed and
   rawler read them by position after assuming an RGGB pattern. The two agree
   only on RGGB. In rawspeed's camera list every non-RGGB Nikon is a
   pre-2010 body, so there may be no practical difference, but which body
   first wrote the tag is unknown. [P][7][8][9][17]
6. **Bodies that write OffsetTimeOriginal.** Only the Z 9 is confirmed. [S]
7. **Make/Model padding on D850 and Z bodies.** Needs a hex dump.
8. **Presence of CropArea (0x0045).** Whether every body writes it, and the
   first body that did.
9. **Outer byte order of recent bodies** (D800, D850, Z 6, Z 7, Z 9, Z f):
   no primary evidence found. Readers must accept both `II` and `MM`.
   [P][11]
10. **Bodies that write NEFCompression 5 and 6.** Labels are known, bodies
    are not.

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
