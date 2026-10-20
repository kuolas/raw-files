# Nikon MakerNote and viewer metadata

This page describes the Nikon MakerNote found in NEF files: its three
layouts, the tags that matter, the encrypted blocks and how they are
decrypted, and where a viewer finds orientation, capture date, make, model
and image size, and where the camera recorded the point it focused on.
It serves milestone 3 (basic metadata), milestone 4 (focus points) and,
for the decoding-related tags, milestone 5 (raw data).

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
[test-data.md](test-data.md#23-files-checked-for-the-focus-point); where a file
contradicts a source, this page follows the file and records the
disagreement. Byte offsets and version numbers taken from implementation
sources were read through summaries; verify them against the source, and
against sample files, before hard-coding them.

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
| Headerless | none | MakerNote + 0 | the outer TIFF header | the outer file's (the D1 is big-endian) | E990, D1, D1H, D1X; the D1H's is big-endian (file 5364) **[F]** |
| Short header | `Nikon\0` `01 00` (8 bytes) | MakerNote + 8 | the outer TIFF header | exiv2 uses the outer order; ExifTool says little-endian | E700 to E950 |
| Embedded TIFF header | `Nikon\0`, version `02 00`, `02 10` or `02 11`, `00 00`, then a TIFF header (`II*\0` or `MM\0*` and a 4-byte offset) (18 bytes) | MakerNote + 10 + the inner IFD offset (normally 8, so + 18) | MakerNote + 10, the inner TIFF header | the inner header's, independent of the file's | D70, D100, D200 and later DSLRs, every Z body; every checked file but the D1H's: `02 00` on the D100 and the Coolpix E5700 and P7800, `02 10` from the D70 to the D7000 and on the Nikon 1 J1, `02 11` on the Coolpix A (2013) and on every body from the D810 (2014) on **[F]** |

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
| 0x0002 | ISO | u16[2] | `[0, ISO]` on most bodies before the D810; absent from the D810 on; four zero bytes of type UNDEFINED on the D700 (file 2255), `[0, 0]` on the E5700 (file 4456) **[F]** | 3 |
| 0x000C | WB_RBLevels | rational[4] | as-shot white-balance multipliers; LibRaw reads them as R, B, then two greens (ExifTool leaves the last two unidentified) | 5 |
| 0x0011 | PreviewIFD | IFD | sub-IFD with a small JPEG preview; offsets relative to MakerNote + 10 | 1 |
| 0x001B | CropHiSpeed | u16[7] | crop mode and geometry; field order in [section 8.4](#84-image-dimensions) | 3, 4, 5 |
| 0x001D | SerialNumber | string | the body's serial number; decryption key input. Seven digits on every checked DSLR and mirrorless body from the D300 on (eight on the Nikon 1 J1) and on the Coolpix A; seven spaces on the D200 (file 8079); absent before it and on the Coolpix E5700 and P7800 **[F]** | 3, 5 |
| 0x0021 | FaceDetect | u16 | the size of a frame and up to twelve face rectangles in it; on none of the 30 checked files **[F]** | none |
| 0x0024 | WorldTime | undef | the time zone and daylight saving setting of the camera's clock; layout in [section 8.2](#82-capture-date-and-time-zone) | 3 |
| 0x0034 | ShutterMode | u16 | mechanical, electronic, EFCS | none |
| 0x0037 | MechanicalShutterCount | u32 | newer bodies; mechanical frames only | none |
| 0x003D | BlackLevel | u16[4] | per-channel black level, recorded at 14-bit scale | 5 |
| 0x003E | ImageSizeRAW | u16 | 1 large, 2 medium, 3 small | 5 |
| 0x0045 | CropArea | u16[4] | left, top, width, height of the output rectangle | 3, 4, 5 |
| 0x0051 | (unnamed, Z bodies) | undef | NEFCompression as a u16 at byte 10; byte order disputed (see the note below). Present on the Z 6 (2018) too, and on the Z f, Z6III and Z 9 (files 3587, 6887, 7808 to 7810, 5146 to 5148); its value was not read **[F]** | 5 |
| 0x0084 | Lens | rational[4] | the lens's shortest and longest focal length and the widest aperture at each; on every checked body with interchangeable lenses, from the D1H on; equal to Exif LensSpecification wherever both exist **[F]** | 3 |
| 0x0088 | AFInfo | undef[4] | area mode, the number of the focus point and a mask of the points in focus; no position. The only focus data of the checked bodies without AFInfo2: the D1H, D70, D100, D200 and Coolpix E5700 and P7800, and written by no checked body that has AFInfo2 **[F]**; [section 8.6](#86-focus-point) | 4 |
| 0x0091 | ShotInfo | undef | model-specific; encrypted for versions 02xx and 08xx | none |
| 0x0093 | NEFCompression | u16 | compression hint; absent on the Z bodies checked, which write 0x0051 instead, and on the D1H and E5700 **[F]** | 5 |
| 0x0096 | NEFLinearizationTable | undef | curve and predictor seeds; unencrypted | 5 |
| 0x008C | (older curve) | undef | fallback for 0x0096 | 5 |
| 0x0097 | ColorBalance | undef | white balance on older bodies; encrypted from version 0200 | 5 |
| 0x0098 | LensData | undef | lens description; encrypted from version 0201 | none |
| 0x00A7 | ShutterCount | u32 | decryption key input | 3, 5 |
| 0x00B7 | AFInfo2 | undef | where the camera focused: a block that starts with a four-character version, plain on every version; layouts in [section 8.6](#86-focus-point) | 4 |
| 0x0E01 | NikonCaptureData | undef | Capture NX edit data (optional) | none |
| 0x0E22 | NEFBitDepth | u16[4] | secondary bit-depth source | 5 |

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

NEFCompression (0x0093; on Z bodies it is in 0x0051, since 0x0093 is
absent from every checked Z body, the Z 6 included **[F]**; the sources
put it at byte 10 of 0x0051 on the Z 9 and later) takes these values. [P][1][3][9]

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
from milestone 5.

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
the MakerNote records ([section 8.6](#86-focus-point)). **[F]** IFD0's own ImageWidth and
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
   inset crop. [P] The D850, D780, Z 6, Z f, Z6III and Z 9 write it; no
   older checked body does, the D810 (file 2036) included. **[F]** It is
   the visible image: on the Z 9 `[12, 8, 8256, 5504]` inside the
   8280x5520 raw, the size of the largest preview (files 5146 to 5148); on
   the D850's small raw (ImageSizeRAW 3) `[12, 4, 6192, 4128]` inside
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
| ISO | 0x8827 ISOSpeedRatings | 0x0002 ISO, `[0, ISO]` | Exif on every checked body except the D1H, D100, D70, D700 and E5700; the first three have it in the MakerNote |
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
[P][1]; every value was read on the 30 CC0 Nikon files of
[test-data.md](test-data.md#23-files-checked-for-the-focus-point), and a
field is tagged **[F]** where a sample holds a value for it.

**Where a Nikon file records focus data.** Every source found, whether
raw-files reads it or not:

| Source | What it holds | Checked files | raw-files |
|---|---|---|---|
| 0x00B7 AFInfo2, position fields | the size of a frame, the centre of the focus area and the area's size in that frame | mirrorless bodies, and DSLRs in live view **[F]** | read, at the versions of the table below |
| 0x00B7 AFInfo2, in-focus byte | whether focus was confirmed: FocusResult on 04xx, ContrastDetectAFInFocus on 0100 and 0101 | Z 9, Z f, Z6III, Z50II, Z5II, D7000, D5600, Coolpix A **[F]** | read; on 0100 the flag is byte 28, where ExifTool also lists a 20-byte AFPointsSelected that cannot fit in the 30-byte value |
| 0x00B7 AFInfo2, point fields | the primary point's number, a mask of the points used, and the layout of points (FocusPointSchema) | DSLRs through the viewfinder, the Nikon 1 J1, the Z5II in auto-area mode **[F]** | not read: a number needs a table of the body's points, which no file carries |
| 0x00B7 AFInfo2, AFPointsInFocus | a mask of the points in focus, version 0101 only (byte 48) [P] | the D850, one point **[F]** | not read: a mask, needs the same table |
| 0x0088 AFInfo | 4 bytes: area mode, point number, mask of the points in focus [P] | the bodies without AFInfo2: D1H, D70, D100, D200, E5700, P7800 **[F]** | not read: a number only |
| 0x0021 FaceDetect | a frame size and up to twelve face rectangles in it [P] | none of the 30 files **[F]** | not read: no sample |
| 0x0007 FocusMode | an ASCII string [P][1]: `MANUAL`, or an AF mode padded with spaces to six characters (`AF-S  `, `AF-C  `, `AF-A  `) **[F]** | `MANUAL` on the D300 and D850, both viewfinder shots with a zero frame (files 928, 1835); an AF mode on every other file with the embedded MakerNote, the maintainer's Z 9 shots included **[F]** | read: `MANUAL`, after trimming, gives no focus point |
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
| `0101` | 84 | 4 | 5 | 6 | none | 70, 72 | 74, 76 | 78, 80 | 82 | D5, D500, D850, D3400, D3500, D7500 [P][S]; sample: D850, a viewfinder shot **[F]** |
| `0200`, `0201` | 42 [F] | none | 5 | 6 | none | none | none | none | none | Nikon 1 bodies [P][S]; sample: J1, `0200` **[F]** |
| `0300`, `0301` | 60 | 4 | 5 | 6 | 7 | 42, 44 | 46, 48 | 50, 52 | none | `0300`: D780, Z 6, Z 7, Z 50; `0301`: D6, Z 5, Z 6II, Z 7II, Z 30, Z fc [S]; samples: D780, Z 6, both `0300` **[F]** |
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
offsets are checked on no file that fills them, and no sample has
version 0301.

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
  the same values. In the 8256x5504 frame they record:

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
  CropArea (ADR012). The values of the CC0 files are in
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
  D780, D850 and D750, 12 (DX uncropped) on the D5600 and 0 (off) on the
  D200, D300, D700, D7000, J1 and Coolpix A, except on the D810, whose
  mode 11 comes with a cropped width four pixels short:
  `[11, 7384, 4928, 7380, 4928, 0, 0]` (file 2036). **[F]**
- **In stored orientation**, like the sizes and previews raw-files
  reports. No CC0 Nikon sample is rotated, so none shows it.
- **A DSLR shot through the viewfinder records no position.** The frame,
  centre and area are zero on the D300, D700, D750, D810, D850 and D780,
  and only the point fields are filled in: primary point 3 on the D810,
  34 on the D780, with 30 bits of its mask set (files 928, 2255, 895,
  2036, 1835, 3827). ExifTool's 0100 and 0101 tables call the position
  fields valid for contrast-detect AF only [P][1]; the live-view files
  above, with a detection method of 1 (contrast) on the D7000, D5600 and
  Coolpix A and 2 (hybrid) on the Z bodies, are the ones that fill them.
  **[F]** Mapping a point number to pixels needs a table of the body's
  points, which no file carries (ADR012).
- **The coordinates flag (byte 7 of 03xx and 04xx)** is 1 on every
  sample with a position and 0 on the D780 viewfinder shot and the Z5II
  auto-area files. **[F]** ExifTool notes that on 04xx either the mask
  or the position is set, not both [P][1]; the Z5II files (area mode 197,
  auto-area) have a 28-bit mask, a zero frame and a FocusResult of 1
  (files 7735, 7736). **[F]** raw-files judges by the values, not by the
  flag.
- **The in-focus byte** is 1 on the Z 9, Z f, Z50II, D7000, D5600 and
  Coolpix A and 0 on all five Z6III files, which have a valid point
  beside it: the camera recorded a miss. **[F]** Version 03xx has no such
  byte.
- **Area modes** in the samples (byte 5): 193 (single point) on the Z 9
  and Z 6, 196 (wide area L) on the Z f, 192 (pinpoint) on the Z6III,
  207 (3D-tracking) on the Z50II, 197 (auto-area) on the Z5II; 1 on the
  D7000 and Coolpix A and 2 on the D5600, in live view. [P][1] **[F]**
  The area size follows the mode: 291x323 on the Z 9, 1859x1452 for the
  Z f's wide area, 984x816 on the D5600. **[F]** raw-files does not
  report the mode.
- **Every AFInfo2 and CropHiSpeed value lies inside the MakerNote's
  declared length** on every checked file. **[F]**

**What the samples cover.** Each kind of shot, the files that show it,
and what raw-files gives; a row without a file is an open question
([section 11](#11-open-questions)):

| Shot | What the file records | CC0 files | raw-files gives |
|---|---|---|---|
| Mirrorless, single point, pinpoint, wide area or 3D-tracking | a position | Z 6 3587, Z 9 5146 to 5148, Z f 6887, Z6III 7808 to 7810, 7812, 7813, Z50II 7767, 7770 | the point |
| Mirrorless on a DX crop | a position, scaled into the full frame | Z6III 7810; the maintainer's Z 9 shots, off-centre | the point, scaled onto the crop |
| Mirrorless on an aspect crop (1:1, 16:9) | a position in the full frame | Z6III 7808, 7809, 7812, 7813, Z50II 7767, 7770; the maintainer's Z 9 shots, off-centre | the point, moved into the window |
| A DX crop at 1:1 or 16:9 on an FX body | unknown | none | the point, scaled; unchecked |
| Mirrorless small raw | a position in the full frame | Z 6 3587 | the point, in the full frame |
| Mirrorless, auto-area with only a mask | a mask | Z5II 7735, 7736 | none: needs a table of points |
| DSLR in live view | a position | D7000 961, D5600 1415, Coolpix A 2238 | the point |
| DSLR in live view on a DX or 1.2x crop | unknown: the cropped or the full frame | none | the point, by the crop rule; unchecked |
| A `0101` body in live view | a position, at ExifTool's offsets | none | the point; unchecked |
| A `0301` body | a position, at the 03xx offsets | none | the point; unchecked |
| DSLR through the viewfinder | a point number and a mask | D300 928, D700 2255, D750 895, D810 2036, D850 1835, D780 3827 | none: needs a table of points; 928 and 1835 are manual-focus shots, which get none in any case |
| Nikon 1 | a point number and a mask | J1 2956 | none: needs a table of points |
| Bodies without AFInfo2 | AFInfo: a point number | D1H 5364, D70 2060, D100 2996, D200 8079, E5700 4456, P7800 1425 | none: needs a table of points |
| Manual focus | FocusMode `MANUAL`; a zero frame through the viewfinder | D300 928, D850 1835 | none |
| Manual focus on a Z body | FocusMode `MANUAL`; AFInfo2 unknown | none | none |
| A 1:1 or 16:9 crop on a small raw | unknown: CropArea counts stored pixels | none | none, since CropArea's kept side differs from the frame's |
| A DX crop on a small raw | unknown | none | the point, scaled onto CropArea in stored pixels; unchecked |
| A rotated (portrait) shot | expected: the landscape frame | none | the point, in stored orientation; unchecked |
| Face detection | FaceDetect | none | not read |

## 9. Robustness pitfalls

The raw-files rule is that a malformed file returns an error and never panics.
The points specific to the MakerNote:

1. **Relocated MakerNotes.** Tools that move the MakerNote without fixing
   offsets leave the headerless and short layouts pointing at the wrong
   place. ExifTool detects this with a heuristic (value offsets compared
   with the expected position four bytes after the IFD) and repairs only on
   request. Bounds-check every offset and length against the MakerNote and
   the file, skip bad entries rather than abort the IFD, and do not repair
   in a first version. [P][S][4][19] raw-files requires the PreviewIFD's
   JPEG, as declared, to lie inside the MakerNote too. The Coolpix P7800
   declares its MakerNote 2,816 bytes long, ending at byte 4546, and its
   640x480 MakerNote JPEG starts at byte 4546, just past it, so the reader
   does not read it and the file gives its 160x120 and 4000x3000 previews
   (file 1425); on the E5700 and the D100, the other `02 00` bodies, the
   declared length includes the JPEG (files 4456, 2996). **[F]**
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
   Z bodies (compression in 0x0051, at byte 10 on the Z 9 and later);
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
and only the embedded-TIFF layout is read, not the headerless and
short-header ones items 2 and 6 cover, so their bodies (the D1 family and
the earliest Coolpix) get the Exif fields only.

The focus point is decided in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md):
the MakerNote's AFInfo2 gives it where it records a position, at the
versions of [section 8.6](#86-focus-point), scaled onto CropArea on a
DX crop and moved into the window on an aspect crop; a FocusMode of
`MANUAL` gives none, and a point number gives none until raw-files has
a table of the body's points.

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
   0x0091 to milestone 5, and only if older-body white balance is required.
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
   a source it can use, or a recorded legal decision, before milestone 5
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
   D850, D780 and Z bodies write it, the D810 and older do not
   ([section 8.4](#84-image-dimensions)). Which bodies between the D810
   and the D850 (2014 to 2017) first wrote it is still open. **[F]**
8. **Outer byte order of recent bodies.** Answered: `II`, file and
   MakerNote alike, on the D750, D810, D850, D780, Z 6, Z f, Z6III and Z 9;
   `MM` on the DSLRs from the D1H to the D7000 and on the Coolpix A (2013);
   `II` on the Coolpix E5700 and P7800 and the Nikon 1 J1. The MakerNote's
   order equals the file's in every checked file. Readers must accept both
   `II` and `MM`. **[F]** [P][11]
9. **Bodies that write NEFCompression 5 and 6.** Labels are known, bodies
    are not.
10. **The 0101 position.** Its offsets (70 to 82) are ExifTool's; the only
    0101 sample, the D850's, is a viewfinder shot with a zero frame (file
    1835). A live-view file from a D5, D500, D850, D3400, D3500 or D7500
    would check them ([section 8.6](#86-focus-point)).
11. **Version 0301.** It is read at the 03xx offsets, and no sample has
    it. A file from a D6, Z 5, Z 6II, Z 7II, Z 30 or Z fc would check it.
12. **A DSLR in live view on a DX or 1.2x crop.** Whether AFInfo2's frame
    is then the cropped output or the full one. The crop rule needs a
    CropArea, which no checked DSLR before the D850 writes; a live-view
    crop file from a D750, D810 or D850 would show it.
13. **Manual focus on a Z body.** What it writes in AFInfo2. Its
    FocusMode, `MANUAL`, gives no focus point whatever AFInfo2 holds; the
    two manual-focus samples are DSLR viewfinder shots (files 928, 1835).
14. **A rotated shot.** No CC0 Nikon sample is rotated, so none shows
    that a portrait shot's frame stays in stored, landscape orientation.
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
17. **FaceDetect.** Which bodies write 0x0021; none of the 30 checked
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
