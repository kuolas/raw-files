# ARW container and embedded previews

A Sony ARW is a little-endian TIFF file with Sony's own additions. Next to
the raw sensor data it carries two or three JPEG images: a preview of about
1616x1080 pixels in IFD0, a 160x120 thumbnail in IFD1 and, on files with
FileFormat 4.0 or later, a full-size JPEG in a third IFD of the IFD0 chain.
The older relatives SRF, SR2 and the A100's ARW 1.0, the computer-made ARQ
and the Hasselblad Lunar use the same container with variations. This page
describes where the JPEGs are, how implementations tell them apart, and the
algorithm raw-files should use to return the largest, for the previews of
the [roadmap](../../ROADMAP.md).

Back to [the ARW overview](README.md).

## How to read this page

Sony publishes no ARW specification. Everything below beyond plain TIFF and
Exif comes from other projects' documentation and source code, from reports
by people who inspected files, and from structural checks on sample files.

Every claim carries a confidence tag:

- **[P]** stated in a primary source: a specification, tag documentation,
  the source code of an implementation, or a published tool dump of a file.
- **[S]** secondary: a forum post, blog, issue report or search-result
  snippet.
- **[I]** inferred, or recalled without a source; a hypothesis to test.
- **[F]** measured on a named sample file. Files are cited as "file 4466",
  the id of the file in the public raw.pixls.us collection; the files cited
  here, with their URLs and SHA-256 hashes, are listed under
  [Sample files](#sample-files), and the full list of checked files is in
  [test-data.md](test-data.md#files-checked).

Numbers in square brackets such as [3] refer to [Sources](#sources).

**Status.** This page combines desk research with structural checks on 36
CC0 sample files (October 2026): every TIFF and MakerNote structure was
walked, and every embedded JPEG was parsed up to its start-of-frame marker.
The check scripts were throwaway and are not in the repository; nothing here
is implemented in raw-files yet, and no pixel output was compared with a
reference decoder. Where an algorithm below is said to be *checked*, it was
run by those scripts on the sample files, not by raw-files.

This page restates algorithms in prose. No code from other projects is
reproduced, and none should be copied into raw-files from GPL, LGPL or
similar sources.

## 1. The TIFF container

### 1.1 Header and byte order

Every Sony raw file starts with the standard 8-byte TIFF header: a
byte-order mark, the number 42, and the offset of IFD0.

| Evidence | Finding | Conf. |
|---|---|---|
| All 36 checked files (SRF, SR2, ARW 1.0 to 6.0, ARQ, Lunar) | start `49 49 2A 00` (`II`, 42) with IFD0 at offset 8 | **[F]** (all files) |
| exiv2 dumps of the 209 Sony-family files on raw.pixls.us | every Sony-body file that reports a byte order reports `II` | **[P]** [21] |
| Clévy's Sony notes | all four formats are TIFF "with Intel (little) endianess" | **[S]** [17] |
| PRONOM signatures for ARW 1.x, ARW 2.x and SR2 | all begin `49 49 2A 00` | **[P]** [19] |
| A big-endian (`MM`) Sony raw file | none found in any source or sample | **[I]** |

Consequence for raw-files: accept both `II` and `MM`, since it costs
nothing, but expect `II`. Follow the IFD0 offset wherever it points: one
published dump of an ILCE-9 file shows IFD0 rewritten at the end of the
file, after the raw data, by an editing program [18] **[P]**.

The Sony MakerNote has no byte-order mark of its own in any of its forms
(section 2). ExifTool detects the order from the entry count; LibRaw forces
little-endian for the `SONY` header form and otherwise uses the file's order
[3] [11] **[P]**. With every known file little-endian, the difference never
shows **[I]**.

A few structures are big-endian whatever the header says: the R1's raw
pixels (file 3221) **[F]**, the SRF's decrypted data and the keystream words
of the SR2 cipher, and the A100's Minolta CameraSettingsA100 block [6]
**[P]**. None of them matters for previews; see
[raw-data.md](raw-data.md) and
[makernote-and-metadata.md](makernote-and-metadata.md).

### 1.2 The format family and FileFormat versions

The format version is MakerNote tag 0xB000 **FileFormat**, four bytes,
which ExifTool maps to a name [1] **[P]**. There is no "ARW 3": the first
byte jumps from 3 (ARW 2.x) to 4 (ARW 4.0) [1] **[P]**.

| Format | FileFormat bytes | ExifTool name | Bodies named in ExifTool's comments [1] | Checked samples (firmware) |
|---|---|---|---|---|
| SRF | none (the MakerNote is a different structure) | – | DSC-F828, DSC-V3 [17] | file 1351, F828 **[F]** |
| SR2 | `1 0 0 0` | SR2 | DSC-R1 | file 3221 **[F]** |
| ARW 1.0 | `2 0 0 0` | ARW 1.0 | (DSLR-A100) | file 2346, A100 v1.04 **[F]** |
| ARW 2.0 | `3 0 0 0` | ARW 2.0 | – | files 2271, A200 v1.00; 2521, A700 v04 **[F]** |
| ARW 2.1 | `3 1 0 0` | ARW 2.1 | – | file 5858, A900 v2.00 **[F]** |
| ARW 2.2 | `3 2 0 0` | ARW 2.2 | NEX-5 | files 1457, NEX-5; 964, SLT-A55V **[F]** |
| ARW 2.3 | `3 3 0 0` | ARW 2.3 | SLT-A65, SLT-A77 | files 5811, RX100 v1.00; 4051, Lunar v1.02 **[F]** |
| ARW 2.3.1 | `3 3 1 0` | ARW 2.3.1 | DSC-RX1R, RX100M2, Hasselblad Stellar II | files 3751, SLT-A99V v1.02; 1582, ILCE-7S v3.20; 1310, ILCE-7RM2 v3.30 **[F]** |
| ARW 2.3.2 | `3 3 2 0` | ARW 2.3.2 | DSC-RX1RM2, ILCE-7SM2 | none checked |
| ARW 2.3.3 | `3 3 3 0` | ARW 2.3.3 | ILCE-9 | files 1541, ILCE-9 v1.00; 3012, RX100M6 v1.00 **[F]** |
| ARW 2.3.5 | `3 3 5 0` | ARW 2.3.5 | DSC-HX99 | file 3251, ILCE-7RM3 v3.00; ARQ file 3255 **[F]** |
| ARW 4.0 | `4 0 0 0` | ARW 4.0 | ILCE-7SM3 | files 5418, ILCE-7SM3 v2.01; 4466, ILCE-1 v1.00 **[F]** |
| ARW 4.0.1 | `4 0 1 0` | ARW 4.0.1 | ZV-E1 (added after ExifTool issue 195 [20]) | files 6929, ILCE-7M4 v2.00; 6736, ILCE-6700 v1.00 **[F]** |
| ARW 5.0 | `5 0 0 0` | ARW 5.0 | ILCE-9M3 | file 7361, ILCE-9M3 v1.00 **[F]** |
| ARW 5.0.1 | `5 0 1 0` | ARW 5.0.1 | ILCE-1 with firmware 2.0 | file 7835, ILCE-1M2 v2.00 **[F]** |
| ARW 6.0 | `6 0 0 0` | ARW 6.0 | ILCE-7M5 | file 8846, ILCE-7M5 v1.00 **[F]** |
| ARQ | that of the source frames | – | made on a computer, not in the camera [39] **[S]** | file 3255 (`3 3 5 0`) **[F]** |

ExifTool's own comment on the table asks what happens with compressed raws;
the mapping is empirical [1] **[P]**.

**The version follows the firmware, not the body.** Clévy's notes give one
version per body [17] **[S]**, but three checked bodies write a different
version, one body writes two versions across the exiv2 dumps, and ExifTool
ties one version to a firmware release:

- the SLT-A99V on firmware 1.02 writes `3 3 1 0` (ARW 2.3.1), where Clévy
  lists ARW 2.3 (file 3751) **[F]**;
- the ILCE-7RM3 on firmware 3.00 writes `3 3 5 0` (ARW 2.3.5), where Clévy
  lists ARW 2.3.3 (file 3251) **[F]**, and other ILCE-7RM3 dumps on
  raw.pixls.us show `3 3 3 0` [21] **[P]**;
- the ILCE-7M4 on firmware 2.00 writes `4 0 1 0` (ARW 4.0.1), where Clévy
  lists ARW 4.0 (file 6929) **[F]**;
- ExifTool attributes `5 0 1 0` to the ILCE-1 with firmware 2.0 [1]
  **[P]**, while an ILCE-1 on firmware 1.00 writes `4 0 0 0` (file 4466)
  **[F]**.

Consequence: never key preview location on FileFormat or on the model. The
container walk of section 7 finds every preview whatever the version; read
FileFormat only to report it.

**Identification by signature fails on new files.** PRONOM's signatures
require `II*\0`, the string "SONY" within the first 512 bytes, and the
MakerNote entry `00 B0 01 00 04 00 00 00` followed by `01` (SR2), `02`
(ARW 1.x) or `03` (ARW 2.x) [19] **[P]**. They cannot match ARW 4.0 to 6.0,
whose first FileFormat byte is 4, 5 or 6 (files 4466, 7361, 8846) **[F]**,
nor the Hasselblad Lunar, which contains no "SONY" string at all (file
4051) **[F]**. The 0xB000 entry itself lies at bytes 884 to 1,356 in files
with FileFormat up to `3 3 1 0` and at bytes 5,458 to 5,584 in files with
FileFormat `3 3 3 0` and later (for example 5,484 in file 1541) **[F]**.
ExifTool recognises ARW from IFD0 (Make "SONY", NewSubfileType 1,
Compression 6) in Exif.pm and switches to SR2 when it meets FileFormat
`1 0 0 0`; it recognises ARQ by extension only [2] [1] [7] **[P]**.
raw-files does not need to tell the formats apart to find a preview: one
walk serves all of them (section 7).

### 1.3 IFD map per generation

Every ARW, SR2 and ARQ file checked lists exactly one offset in IFD0's
SubIFDs tag (0x014A), and that SubIFD is the raw image; the IFD0 next-IFD
chain ends after IFD1 before FileFormat 4.0 and after IFD2 from 4.0 on
(all files) **[F]**. The exiv2 dumps of the whole raw.pixls.us Sony
collection agree: one SubIFD in every Sony-body file, and no JPEG in any
SubIFD [21] **[P]**.

| Generation | IFD0 | IFD1 (IFD0's next) | IFD2 (IFD1's next) | 0x014A target | MakerNote | 0xC634 target |
|---|---|---|---|---|---|---|
| SRF (F828) | describes the encrypted raw: 3360x2460, RGB, 14-bit, StripByteCounts but no StripOffsets; no NewSubfileType | the only JPEG, 640x480; no NewSubfileType | none | no 0x014A | SRF0 to SRF6 chain, encrypted from SRF1 | no 0xC634 |
| SR2 (R1) | NewSubfileType 1, JPEG 640x424 | NewSubfileType 1, JPEG 160x120 | none | raw, CFA, uncompressed 14-bit | headerless | SR2Private IFD |
| ARW 1.0 (A100) | NewSubfileType 1, JPEG 640x480, plus ImageWidth/Length 3872x2592 that describe the raw | NewSubfileType 1, JPEG 160x120 | none | **not an IFD**: the offset of the raw data | `SONY DSC \0\0\0` header, with a Minolta sub-MakerNote | a Minolta MRW block, not an IFD |
| ARW 2.0 to 2.3.5 | NewSubfileType 1, JPEG 1616x1080 | NewSubfileType 1, JPEG 160x120 | none | raw, CFA | headerless | SR2Private IFD |
| ARQ | as its source ARW | as its source ARW | none | raw, Photometric 32892, 4 samples | headerless | SR2Private IFD |
| Hasselblad Lunar | as ARW 2.3 | as ARW 2.3 | none | raw, CFA | headerless | SR2Private IFD |
| ARW 4.0 to 6.0 | NewSubfileType 1, JPEG 1616x1080 | NewSubfileType 1, JPEG 160x120 | **NewSubfileType 1, Compression 7, Photometric 6, JPEG at the image's full size**; next-IFD 0 | raw: CFA strips, CFA tiles, YCbCr tiles, or one Compression 32766 strip | headerless | SR2Private IFD |

Sources for the table: files 1351, 3221, 2346, 2271 to 3251, 3255, 4051 and
the 16 files from 5418 on **[F]**; the same shapes appear in the ExifTool
dumps of an F828, an R1, an A100 and an ILCE-9 [18] **[P]**.

The exceptions to "IFD0 and IFD1 hold JPEGs": in the SRF, IFD0 is the raw;
and among the exiv2 dumps, one FileFormat 4.0 file has no IFD2 (section
3.1).

#### ARW 2.x (FileFormat `3 0 0 0` to `3 3 5 0`)

File 1542 (ILCE-9, ARW 2.3.3) is typical **[F]**:

- **IFD0** at 8, 19 entries: NewSubfileType 1, Compression 6, Make, Model,
  Orientation, resolutions, Software, DateTime, SubIFDs (one offset),
  JPEGInterchangeFormat 0x0201 = 151,714 and its length 0x0202 = 393,347,
  YCbCrPositioning, an XMP packet (0x02BC), the Exif IFD pointer (0x8769),
  PrintIM (0xC4A5) and SR2Private (0xC634). **No ImageWidth, ImageLength,
  strips or tiles.**
- **IFD1** at 42,786, 14 entries: NewSubfileType 1, Compression 6, a repeat
  of Make, Model, Orientation and DateTime, and a 9,334-byte JPEG at 43,056.
  Its next-IFD pointer is 0.
- **Raw SubIFD** at 147,854, 35 entries: NewSubfileType 0, 6048x4024,
  14 bits, Compression 1, Photometric 32803, one strip at 548,864, and
  Sony's 0x7000-range tags.
- **Exif IFD**, holding the MakerNote (tag 0x927C) at 5,170: a headerless
  IFD.
- **SR2Private** at 53,912, nine entries: the offset, length and key of the
  encrypted SR2SubIFD (0x7200, 0x7201, 0x7221), and 0x7240 and 0x7241
  (section 5).

The XMP packet in IFD0 appears from FileFormat `3 3 3 0` on [21] **[P]**.
It is in all 21 checked files from `3 3 3 0` onwards (files 1541, 3012,
3251, 4466, 8846 and others) and in none of the 13 checked files with an
older FileFormat; no `3 3 2 0` file was checked. **[F]** That is why the
MakerNote moved from bytes 870 to 1,234 (files 3221, 2346) to bytes 5,144
to 5,294 (files 3012, 4466). **[F]**

#### ARW 4.0 and later (FileFormat `4 0 0 0` to `6 0 0 0`)

The same IFD0, IFD1 and raw SubIFD, plus **IFD2**, reached through IFD1's
next-IFD pointer, whose own next pointer is 0 (all 16 files) **[F]**.
IFD2 is described in section 3.1. File 4466 (ILCE-1, lossless) shows the
physical order, which is the same in shape in every 4.0 and later file
**[F]**:

| Offset | Content |
|---|---|
| 0 | TIFF header |
| 8 | IFD0 (22 entries) |
| 5,294 | MakerNote, a headerless IFD of 105 entries |
| 43,686 | IFD1 (16 entries) |
| 44,008 | IFD1 JPEG: 160x120, 13,144 bytes |
| 65,752 | SR2Private IFD (9 entries) |
| 65,866 | encrypted SR2SubIFD (35,398 bytes) |
| 138,128 | target of SR2Private tag 0x7240: six zero bytes |
| 138,134 | IFD2 (16 entries) |
| 138,442 | raw SubIFD (40 entries) |
| 143,522 | IFD0 JPEG: 1616x1080, 484,164 bytes |
| 630,784 | IFD2 JPEG: 8640x5760, 9,751,756 bytes |
| 10,383,360 | raw data: 204 lossless-JPEG tiles, running to the end of the file at 81,575,280 |

All the metadata and all three JPEGs come before the raw data in every
checked file, so a reader working from a `Read + Seek` source (as raw-files
does, [ADR009](../../architecture-decisions/adr009-public-api-shape.md))
can find and return the largest preview
without touching the raw. That ordering is an observation, not a rule; a
re-saved file can move IFD0 to the end [18] **[P]**.

#### SR2 (DSC-R1)

The ARW 2 shape (file 3221) **[F]**: IFD0 has NewSubfileType 1,
Compression 6 and a 145,173-byte JPEG of 640x424 at 1,358; IFD1 a
28,372-byte JPEG of 160x120 at 165,198; the SubIFD at 254,672 is the raw
(3984x2608, 14-bit, uncompressed, CFA). There is no Software tag. The Exif
IFD's PixelXDimension and PixelYDimension say 640x424, the preview's size,
not the raw's (file 3221) **[F]**. A published dump of another R1 file
shows the same shape with a 134,520-byte preview [18] **[P]**.

#### ARW 1.0 (DSLR-A100)

A hybrid of the Sony and Minolta designs (file 2346) **[F]**:

- IFD0 has NewSubfileType 1, Compression 6, a 42,850-byte JPEG at 25,683,
  and ImageWidth and ImageLength **3872x2592**, which describe the sensor;
  the JPEG they sit next to is 640x480.
- IFD0's 0x014A holds **98,304**, the offset of the raw data; the bytes
  there do not parse as an IFD. ExifTool calls this value A100DataOffset
  and decides by validating the target; after editing in Sony's Image Data
  Converter it becomes a real SubIFD pointer [1] [2] **[P]**. Another
  file's dump shows 65,536 [18] **[P]**, so the value is not a constant.
- 0xC634 points to a Minolta MRW block that begins `\0MRI`, not to an IFD;
  its lengths are little-endian (file 2346) **[F]**.
- IFD1 holds a 4,071-byte JPEG of 160x120 at 68,535.
- The MakerNote starts with the 12-byte `SONY DSC \0\0\0` header (section
  2), and its tag 0xB028 points to a Minolta-format IFD whose tags 0x0088
  and 0x0089 give the **same** offset and length as IFD0's JPEG.
- Make is `SONY` padded with 11 spaces to 16 bytes [21] **[P]** (file 2346)
  **[F]**.

#### SRF (DSC-F828, DSC-V3)

File 1351 (F828) **[F]**:

- IFD0 at 8 (18 entries) claims a 3360x2460, 3-sample, 14-bit RGB image
  (Photometric 2, Compression 1) with StripByteCounts 16,531,200 but **no
  StripOffsets**. The raw sits at a fixed offset and is encrypted (see
  [raw-data.md](raw-data.md)). Neither IFD carries NewSubfileType.
- IFD1 at 646 (6 entries: Compression, resolutions, ResolutionUnit,
  0x0201, 0x0202) holds the only JPEG: 640x480, 151,046 bytes, at offset
  740. ExifTool's comment says this "JPEG image in IFD1" is "actually a
  preview" [2] **[P]**.
- The MakerNote at 164,580 is the SRF0 to SRF6 chain of small IFDs, SRF1 on
  encrypted [1] **[P]** [17] **[S]**. A published ExifTool dump of an F828
  reports that SRF6 holds a 672x492 uncompressed 14-bit "RGB" image at 200,896
  [18] **[P]**; the sample checks could not locate SRF2 to SRF6 to confirm it.

#### ARQ (pixel-shift composite)

An ARQ is made on a computer by Sony's software from 4 or 16 pixel-shift
ARWs [39] **[S]**. File 3255 (ILCE-7RM3) keeps the container of its source
frame: IFD0's 1616x1080 JPEG and IFD1's 160x120 JPEG have exactly the same
offsets, lengths and bytes as in frame file 3251, and so do the MakerNote
and SR2Private offsets; Software still names the camera. Only the raw
SubIFD changed: 8000x5320, 4 samples per pixel, Photometric 32892,
340,480,000 bytes = 8 bytes per pixel. SR2Private tag 0x7241 is 0, so it
carries no Image Data Converter IFD **[F]**. MakerNote tag 0x202F
PixelShiftInfo ends `0 4` in the composite and `1 4` in the frame (files
3255, 3251) **[F]**, matching ExifTool's description of `0/4` as the
composed image [1] **[P]**.

#### Hasselblad Lunar

The Lunar is a rebadged NEX-7 [40] **[S]** and its file is an ordinary ARW
2.3 (file 4051) **[F]**: Make is `HASSELBLAD` (upper case, 11 bytes with
the NUL), Model `Lunar`, FileFormat `3 3 0 0`, SonyModelID 289 (the NEX-7's
value [1] **[P]**), a headerless MakerNote of 76 entries, IFD0 JPEG
1616x1080, IFD1 JPEG 160x120, and a 12-bit compressed raw SubIFD.
Its IFD0 carries Sony's DNGPrivateData pointer (BYTE, count 4) and no
DNGVersion, as every Sony ARW does **[F]**, while Hasselblad's own 3FR and
FFF files carry neither tag ([test data, section
3.2](test-data.md#32-contents)) **[P]**.

### 1.4 Tags that identify an IFD's role

| Tag | Name | What Sony files contain | Conf. |
|---|---|---|---|
| 0x00FE | NewSubfileType | 1 in IFD0, IFD1 and IFD2; 0 in the raw SubIFD; **absent** in both IFDs of the SRF | **[F]** (all files; 1351) |
| 0x0103 | Compression | 6 (old-style JPEG) in IFD0 and IFD1; **7 in IFD2, whose JPEG is baseline**; also 7 in lossless and YCbCr raw IFDs; 1, 32767 or 32766 in other raw IFDs; 1 in the SRF IFD0 | **[F]**; names [2] **[P]** |
| 0x0106 | PhotometricInterpretation | 32803 CFA raw; **32892** "Sequential Color Filter", the ARQ raw; **6 (YCbCr) both in IFD2 (8-bit) and in medium and small raws (BitsPerSample 15,15,15)**; 2 (RGB) in the SRF IFD0, which is raw | **[F]** (4466, 3255, 6930, 1351); 32892 [2] **[P]** |
| 0x0102 | BitsPerSample | absent in IFD0 and IFD1; 8,8,8 in IFD2; 8, 12 or 14 in CFA raws; 15,15,15 in YCbCr raws; 14,14,14 in the SRF IFD0 | **[F]** |
| 0x0100, 0x0101 | ImageWidth, ImageLength | absent in IFD0 and IFD1 except the A100's IFD0, where they describe the raw; in IFD2 equal to the JPEG's SOF size | **[F]** (2346; all 16 files from 4.0) |
| 0x0201, 0x0202 | JPEGInterchangeFormat, ...Length | offset and length of the JPEG in IFD0, IFD1 and IFD2 (and the SRF IFD1); never in a SubIFD | **[F]** |
| 0x0111, 0x0117 | StripOffsets, StripByteCounts | raw IFDs only; a lossless raw IFD has both one strip and the tiles, starting at the same offset; the SRF IFD0 has StripByteCounts without StripOffsets | **[F]** (4466, 1351) |
| 0x0142 to 0x0145 | TileWidth, TileLength, TileOffsets, TileByteCounts | lossless and YCbCr raw IFDs only, 512x512, from FileFormat 4.0 | **[F]**; [21] **[P]** |
| 0x0211 to 0x0214 | YCbCr tags | in IFD2, and also in YCbCr raw IFDs | **[F]** (4466, 6930) |
| 0x014A | SubIFDs | one offset in every ARW, SR2 and ARQ; on the A100 the raw data offset | **[F]** |
| 0xC634 | DNGPrivateData, used as SR2Private | written as BYTE[4] holding a little-endian offset, in every ARW, SR2 and ARQ; ExifTool writes it back the same way because Adobe Camera Raw 5.3 and Apple Preview misbehaved otherwise | **[F]**; [1] [2] **[P]** |
| MakerNote 0xB02C | PreviewImageSize | height then width, equal to the IFD0 JPEG's size (1080 1088 and 1080 1920 in the aspect-ratio files) | **[F]** (3012, 3015) |
| MakerNote 0xB02B | FullImageSize | height then width of the image; written from the A200 to the A7 V | **[F]** |

LibRaw's raw-IFD selection skips any Sony IFD with BitsPerSample 8,
Photometric 6, Compression 7, SamplesPerPixel 3 and NewSubfileType 1,
commented "Sony RGB preview" [10] **[P]**. That is exactly IFD2's profile
(file 4466 and the other 4.0 and later files) **[F]**.

Neither Compression nor NewSubfileType classifies an IFD on its own:
Compression 7 is used both by the largest preview and by raw tiles, and the
SRF's JPEG IFD has no NewSubfileType. Section 7 classifies by several facts
together.

### 1.5 Offset bases

| Pointer | Relative to | Conf. |
|---|---|---|
| IFD0, the next-IFD chain, 0x014A, the Exif IFD, 0x0201, strips and tiles in every IFD | the start of the file | **[F]** (all files) |
| Values inside the headerless MakerNote (ARW, SR2, ARQ, Lunar) | the outer TIFF header, which is the file start | **[F]**; [3] [16] **[P]** |
| Values inside the `SONY DSC` MakerNote (A100), including 0xB028 | the outer TIFF header | **[F]** (2346: 0xB028 = 1,612, and a Minolta IFD parses there) |
| Minolta sub-MakerNote 0x0088 and 0x0089 (A100) | the outer TIFF header: 0x0088 equals IFD0's 0x0201 | **[F]** (2346) |
| The SRF MakerNote chain | the file start | [17] **[S]**; [18] **[P]** |
| SR2Private (0xC634), and 0x7200 inside it | the file start | **[F]**; [1] **[P]** |
| SR2Private 0x7240 (IDC_IFD) | the file start, by ExifTool's default; but see section 5 for the 6-byte offset seen in every file | [1] **[P]**; **[F]** |
| SR2Private 0x7241 (IDC2_IFD) and each version IFD 0xD001 inside it, written by Sony's Image Data Converter | **the IFD's own start**: ExifTool switches the base to the start of the directory | [1] [5] **[P]**; no sample |
| MakerNote 0x2044 HiddenInfo (offset, size) | the TIFF header | [1] **[P]**; **[F]** (6929: (143,360, 53,248), data starting `55 26 11 05`; the offset varies across files from 139,264 (6933, 6936) to 147,456 (8846 to 8850), the size is 53,248 in all) |

ExifTool's base check (FixBase) expects the MakerNote's value data to
start 4 bytes after the IFD entries for DSLR-*, SLT-A33/A35/A55V and
NEX-3/5/C3/VG10E, and immediately after them for other Sony bodies; it uses
the difference to detect and repair a moved MakerNote in edited files [3]
**[P]**. The samples agree (section 2).

## 2. MakerNote layouts, as far as previews need them

No ARW, SR2 or ARQ preview is reachable only through the MakerNote. The
A100's Minolta sub-MakerNote points to the same bytes as IFD0, and
MakerNote tag 0x2001 (an inline preview) occurs in Sony JPEG files, not in
raw files (section 3). raw-files can therefore find every preview without
parsing the MakerNote **[I]**, and needs it only to report FileFormat or to
cross-check PreviewImageSize. The layouts, named by their bytes:

| Layout | Starts with | IFD starts at | Offsets relative to | Value data after the entries | Seen in | Conf. |
|---|---|---|---|---|---|---|
| Headerless | a bare IFD (entry count) | MakerNote start | outer TIFF header | 4 zero bytes, then values, on the A200, A700, A900, NEX-5, SLT-A55V and the R1; values immediately, with **no next-IFD pointer**, on the RX100, A99V, Lunar and every ILCE and RX from 2012 on | every ARW from the A200 on, the R1's SR2, ARQ, the Lunar | **[F]** (2271, 2521, 5858, 1457, 964, 3221; 5811, 3751, 4051, 4466 and later) |
| `SONY DSC \0\0\0` | 12-byte header | MakerNote start + 12 | outer TIFF header | 4 zero bytes | the A100's ARW; JPEGs from DSC compacts | **[F]** (2346); [3] [9] **[P]** |
| Other 12-byte headers: `SONY CAM \0\0\0`, `SONY MOBILE`, `\0\0SONY PIC\0`, `VHAB     \0` | 12-byte header | MakerNote start + 12 | outer TIFF header | – | JPEG files from other Sony devices, and Hasselblad rebadges according to ExifTool; no raw sample carries one | [3] **[P]**; **[F]** (none in 36 files) |
| SRF chain | a bare IFD (SRF0, one entry) | MakerNote start | file start | – | SRF | **[F]** (1351); [1] **[P]** |

ExifTool also maps `SONY PI\0` and `PREMI\0` (compacts' JPEGs) to its
Olympus tables, and a `SONY PIC\0` text block and a Sony Ericsson form to
other parsers [3] **[P]**; none appears in a raw file.

Detection, combining what the cited projects do: if the data starts with one of
the 12-byte headers, the IFD is at +12; otherwise it is a bare IFD at +0. Do
not decide by Make: the Lunar's Make is `HASSELBLAD` and its MakerNote is
headerless (file 4051) **[F]**. ExifTool falls back to the headerless form when
Make starts with "SONY", or when Make starts with upper-case "HASSELBLAD" and
Model starts with HV, Stellar, Lusso or Lunar [3] **[P]**. Exiv2 chooses its
Sony MakerNote parser by Make "SONY" [9] **[P]**, which is presumably why its
dump of the Lunar shows no FileFormat or SonyModelID although the bytes hold
them [21] **[P]**, file 4051 **[F]**, **[I]** for the cause. Never require a
next-IFD pointer after the entries.

## 3. Where a JPEG can live

ExifTool and Exiv2 give each location a name, which is useful for matching
reports to this page.

| ExifTool name | Exiv2 group | Location | Files | Size seen | Conf. |
|---|---|---|---|---|---|
| PreviewImage | `Exif.Image` | IFD0 0x0201/0x0202 | ARW, SR2, ARQ (not SRF) | 1616x1080 from the A200 on; A100 640x480; R1 640x424 | names [2] [8] **[P]**; sizes **[F]** |
| ThumbnailImage | `Exif.Thumbnail` | IFD1 0x0201/0x0202 | all, including SRF | 160x120; in the SRF 640x480, "actually a preview" [2] | names **[P]**; sizes **[F]** |
| JpgFromRaw | `Exif.Image2` | IFD2 0x0201/0x0202 | ARW with FileFormat 4.0 and later | the image's full size, up to 9504x6336 seen (section 4) | ExifTool names any IFD2 0x0201 JpgFromRaw [2] **[P]**; Exiv2 has a loader for `Exif.Image2` [8] **[P]**; sizes **[F]**, [21] **[P]** |
| JpgFromRaw | `Exif.SubImage1` | 0x0201 in a SubIFD | none | – | **[F]** (none in 36 files); [21] **[P]** (none in the dumps of 209 Sony-family files) |
| PreviewImage (Minolta) | `Exif.SonyMinolta` | A100 MakerNote 0xB028, Minolta tags 0x0088/0x0089 | A100 | the same bytes as IFD0's JPEG | [6] [8] **[P]**; **[F]** (2346) |
| PreviewImage (MakerNote) | not loaded by Exiv2 | MakerNote 0x2001: a 32-byte Sony header, then a JPEG whose first byte is 0x00 instead of 0xFF | Sony **JPEG** files | – | [1] **[P]**; absent from every checked ARW, SR2 and ARQ file **[F]** |
| IDCPreviewImage, IDCPreviewImage1 and up | – | IFDs that Sony's Image Data Converter writes into SR2Private (0x7240, 0x7241, version IFDs 0xD001) | edited files only | – | [5] **[P]**; none in the samples (0x7241 = 0 in every file) **[F]** |
| (none) | – | SRF6, a 672x492 uncompressed RGB image | SRF | not a JPEG | [18] **[P]** |

Notes:

1. **IFD0** holds the main preview of every ARW, SR2 and ARQ. It is the
   largest JPEG before FileFormat 4.0 and the middle one from 4.0 on.
2. **IFD1** holds the 160x120 thumbnail, except in the SRF, where it holds
   the only JPEG.
3. **IFD2** holds the full-size JPEG from FileFormat 4.0 on (section 3.1).
4. **SubIFDs** never hold a JPEG in Sony files: the single SubIFD is always
   the raw. A walker must still visit them, both because a rewritten file
   might differ and because the raw IFD has to be recognised and skipped.
5. **MPF.** No embedded JPEG in any checked file has an APP segment of any
   kind, so none carries MPF data, Exif or an Orientation of its own; every one
   starts `FF D8 FF DB` (all files) **[F]**. A comment in ExifTool's Sony.pm,
   written about tag 0x2001 in Sony JPEG files, notes that the corresponding
   bytes of ARW previews are `FF D8 FF DB`, a quantisation table straight after
   SOI [1] **[P]**; 0x2001 itself does not occur in ARW files. Sony JPEG files
   do use MPF for their preview [27] **[S]**.
6. **No hidden JPEGs.** A whole-file byte scan for DCT JPEG headers outside
   the referenced ones found none in any checked file **[F]**.

### 3.1 The full-size JPEG in IFD2 (FileFormat 4.0 and later)

Every one of the 16 checked files with FileFormat `4 0 0 0` or later
(ILCE-7SM3, ILCE-1, ILCE-7M4, ILCE-6700, ILCE-9M3, ILCE-1M2, ILCE-7M5) has
it, and none of the 20 earlier files (SRF, SR2, ARW 1.0 to 2.3.5, ARQ)
does **[F]**. In the exiv2 dumps, 78 of the 79 ARW files at those versions
have it [21] **[P]**.

- **Where.** The third IFD of the IFD0 chain, reached through IFD1's
  next-IFD pointer; its own next pointer is 0. **Not** a SubIFD.
- **Tags** (16 entries in every file): NewSubfileType 1, ImageWidth and
  ImageLength, BitsPerSample 8,8,8, Compression 7, Photometric 6,
  ImageDescription, Orientation, SamplesPerPixel 3, PlanarConfiguration 1,
  0x0201/0x0202, YCbCrCoefficients, YCbCrSubSampling 2,1,
  YCbCrPositioning and ReferenceBlackWhite (file 4466) **[F]**.
- **The JPEG.** Baseline (SOF0), 8-bit, 3 components, sampled 2x1, 1x1,
  1x1 (4:2:2), no APP segment, starting `FF D8 FF DB`; the declared length
  ends exactly at the end-of-image marker. Its SOF size equals the IFD's
  ImageWidth and ImageLength and the raw IFD's DefaultCropSize (all 16
  files) **[F]**; in the exiv2 dumps the tag size equals DefaultCropSize in
  all 78 files [21] **[P]**. Compression 7 notwithstanding, it is an
  ordinary baseline JPEG.
- **Size.** "Full size" means the size of the recorded image in the mode
  used, not the sensor's: 4608x3072 in the ILCE-7M4's APS-C mode and in its
  medium-size raw, 3504x2336 in its small raw (files 6933, 6936, 6930,
  6931) **[F]**. Lengths run from 1,102,572 bytes (file 7835) to 9,751,756
  bytes (file 4466) in the checked files **[F]**, and from 578,991 to
  12,313,510 bytes in the dumps [21] **[P]**.
- **The exception.** The only ZV-1M2 dump on raw.pixls.us (FileFormat
  `4 0 0 0`, firmware 2.01) shows no IFD2 [22] **[P]**. Its bytes were not
  checked. A preview finder that ranks all candidates handles it either way.
- **A second path to it.** SR2Private tag 0x7240 points exactly 6 bytes
  before IFD2, at six zero bytes, in every 4.0 and later file (section 5)
  **[F]**.

This settles a disagreement in earlier reports. One user extracted only a
1616x1080 preview from an ILCE-1 file with ExifTool's PreviewImage, while
another described a full-resolution JPEG extracted as JpgFromRaw [23]
**[S]**; Adobe acknowledged and fixed a Lightroom Classic bug in which it
used the smaller of two ILCE-1 previews [24] **[S]**. ExifTool names the
IFD0 JPEG PreviewImage and the IFD2 JPEG JpgFromRaw [2] **[P]**, so both
users were right about different IFDs; file 4466 holds both (1616x1080 and
8640x5760) **[F]**. The belief that Sony bodies do not embed a full-size
JPEG [25] [26] **[S]** holds only before FileFormat 4.0.

## 4. Preview sizes by body and generation

Sizes come from the SOF markers of the checked files **[F]**, and from the
tag sizes and byte lengths in the exiv2 dumps [21] **[P]**, which do not
show the IFD0 and IFD1 JPEGs' pixel sizes. raw-files should still measure
at run time (section 7).

| Generation, bodies | IFD0 | IFD1 | IFD2 | Evidence |
|---|---|---|---|---|
| SRF: F828 | – | 640x480, 151,046 bytes | – | file 1351 **[F]** |
| SR2: R1 | 640x424, 145,173 bytes | 160x120 | – | file 3221 **[F]** |
| ARW 1.0: A100 | 640x480, 42,850 bytes | 160x120 | – | file 2346 **[F]** |
| ARW 2.0 to 2.3.5 (3:2): A200, A700, A900, NEX-5, A55V, RX100, A99V, Lunar, ILCE-7S (APS-C crop), ILCE-7RM2, ILCE-9, ILCE-7RM3, ARQ | 1616x1080 | 160x120 | – | files 2271, 2521, 5858, 1457, 964, 5811, 3751, 4051, 1582, 1310, 1311, 1541, 1542, 3251, 3255 **[F]** |
| RX100M6 at 1:1 | 1088x1080 | 160x120 | – | file 3012 **[F]** |
| RX100M6 at 16:9 | 1920x1080 | 160x120 | – | file 3015 **[F]** |
| 4:3 aspect (bodies not identified) | 1440x1080, from the MakerNote's PreviewImageSize in two dumps | – | – | [21] **[P]** for the tag; **[I]** for the JPEG |
| ILCE-7SM3 (4.0) | 1616x1080 | 160x120 | 4240x2832 | file 5418 **[F]** |
| ILCE-1 (4.0) | 1616x1080 | 160x120 | 8640x5760 (9.75 MB lossless; 12.3 MB in a compressed-raw dump) | file 4466 **[F]**; [21] **[P]** |
| ILCE-7M4 (4.0.1) | 1616x1080 | 160x120 | 7008x4672 (large), 4608x3072 (APS-C, medium), 3504x2336 (small) | files 6929, 6933, 6936, 6930, 6931 **[F]** |
| ILCE-6700 (4.0.1) | 1616x1080 | 160x120 | 6192x4128 | file 6736 **[F]** |
| ILCE-9M3 (5.0) | 1616x1080 | 160x120 | 6000x4000; 3008x2000 (small) | files 7361, 7364 **[F]** |
| ILCE-1M2 (5.0.1) | 1616x1080 | 160x120 | 5616x3744 (APS-C) | file 7835 **[F]** |
| ILCE-7M5 (6.0) | 1616x1080 | 160x120 | 7008x4672; 4608x3072 (APS-C) | files 8846 to 8850 **[F]** |
| ILCE-7RM5, ILCE-7CR, DSC-RX1RM3 | – | – | 9504x6336, 6240x4160, 4752x3168 | [21] **[P]** |
| ILCE-7CM2, ILME-FX2 | – | – | 7008x4672, 4608x3072 (and 3504x2336 on the FX2) | [21] **[P]** |
| ILME-FX3, ZV-E1 | – | – | 4240x2832 | [21] **[P]** |
| ILME-FX30, ZV-E10M2 | – | – | 6192x4128 | [21] **[P]** |
| ILCE-9M3 and ILCE-1M2, other modes | – | – | 3936x2632 (ILCE-9M3); 8640x5760 and 4320x2880 (ILCE-1M2) | [21] **[P]** |
| ZV-1M2 (4.0) | 1,190,329 bytes | 9,105 bytes | **none** | [22] **[P]** |

Further observations:

- **1616x1080 everywhere at 3:2.** The MakerNote's PreviewImageSize says
  1616x1080 in 191 of the 205 dumps that have it; 11 say 1920x1080, 2 say
  1440x1080 and 1 says 1088x1080 [21] **[P]**. Earlier reports of
  1610x1080, 1660x1080 or 1616x1000 [28] [26] **[S]** are probably typos
  **[I]**; no checked file has them.
- **Byte lengths vary 30-fold for the same pixel size.** IFD0 JPEGs run
  from 85,740 bytes (ZV-E10M2) to 1,334,081 bytes (A350) in the dumps, and
  IFD1 thumbnails from 2,019 to 13,181 bytes [21] **[P]**. Byte length says
  nothing about pixel size.
- **Aspect ratio.** The raw and DefaultCropSize stay at 3:2 and the IFD0
  preview carries the in-camera crop (file 3015: raw 5504x3672,
  DefaultCropSize 5472x3648, preview 1920x1080) **[F]**, as users reported
  [30] **[S]**. A viewer that assumes 3:2 shows such a preview squashed
  [29] **[S]**. Whether IFD2 carries the crop on 4.0 and later bodies is
  open.
- **Every embedded JPEG is the same kind**: baseline, 8-bit, 4:2:2, three
  components, no APP segment (all files) **[F]**. Plan for others anyway
  (section 7).

## 5. Generational quirks

- **A100 (ARW 1.0).** See section 1.3: 0x014A is a raw pointer, 0xC634 points
  at an MRW block instead of an SR2Private IFD, the MakerNote has a header, the
  preview is referenced twice, and IFD0's ImageWidth and ImageLength
  (3872x2592) belong to the raw, not to the 640x480 JPEG (file 2346) **[F]**.
  ExifTool also repairs a **thumbnail offset 0x10000 too small** in some A100
  files: if the IFD1 data does not start `FF D8 FF`, the offset is below
  0x10000, and a JPEG starts 65,536 bytes later, it uses that and warns; its
  writer's comment names firmware 1.00 [2] [4] **[P]**. File 2346 is firmware
  1.04 and its thumbnail offset is correct **[F]**. ExifTool also notes that
  the A100's IFD0 preview length "may be wrong" and ignores the Minolta
  duplicate [2] **[P]**; in file 2346 the length is right and ends exactly at
  EOI **[F]**. No firmware-related offset bug is documented for any other body
  **[I]**.
- **SRF.** IFD0 describes the raw, not an image a reader can use, and has
  no StripOffsets; IFD1 is the preview (file 1351) **[F]**. raw-files must
  not treat IFD0 as an image, which the role filter of section 7 does by
  its bit depth and missing offsets.
- **SR2.** The ARW 2 shape with a small 640x424 preview (file 3221)
  **[F]**.
- **Lossless raws (ILCE-1 and later) look like JPEGs.** Every 512x512 tile
  of a lossless raw starts with `FF D8 FF C3`: a JPEG start-of-image
  followed by a lossless (SOF3) frame header (files 4466, 6929, 6930, 6931,
  6736, 7364, 8848) **[F]**, as ExifTool's, LibRaw's and rawspeed's
  handling suggests [10] [13] **[P]** [20] **[S]**. The raw IFD lists the
  tiles and also one strip starting at the first tile (file 4466:
  StripOffsets 10,383,360, StripByteCounts 71,191,920 = the sum of the
  tiles) **[F]**.
  A preview finder that accepts any IFD with SOI and a SOF will offer the
  raw as a preview.
- **Medium and small raws (ILCE-7M4, ILCE-9M3 and later)** are YCbCr
  lossless-JPEG tiles with Photometric 6, the same photometric as IFD2's
  preview; only BitsPerSample (15,15,15), the tile tags and NewSubfileType 0
  tell them apart (files 6930, 6931, 7364) **[F]**.
- **ARW 6.0 (ILCE-7M5).** A new raw compression, 32766, whose decoding
  raw-files will refuse for now (see [raw-data.md](raw-data.md)). The
  preview layout is unchanged: IFD0, IFD1 and IFD2 as in 4.0 (files 8846
  to 8850) **[F]**, so previews from ARW 6.0 files are supported. Sony's
  firmware 1.01 for the ILCE-7M5 fixes a case where "noise may appear in
  the preview image embedded in RAW files" shot as RAW and HEIF [38]
  **[S]**; which embedded JPEG was affected is not stated.
- **ARQ.** The source frame's container with a new raw (section 1.3); the
  same previews as the frame (file 3255) **[F]**.
- **Hasselblad Lunar.** A Sony ARW 2.3 under another Make (file 4051)
  **[F]**. ExifTool lists a `VHAB     \0` MakerNote header for Hasselblad
  models [3] **[P]**, but the Lunar's is headerless. The Stellar, HV and
  Lusso had no sample.
- **Bytes after the end-of-image marker.** The declared length usually ends
  exactly at EOI, but the NEX-5 and SLT-A55V IFD0 previews are followed by
  51 bytes of `FF` inside the declared length (files 1457, 964), and the
  A200, A700 and A900 IFD1 thumbnails by 15, 4 and 15 bytes of other data,
  their declared length running up to the next structure (files 2271,
  2521, 5858) **[F]**. Returning the declared bytes is harmless; JPEG
  decoders stop at EOI.
- **The 0x7240 link.** SR2Private tag 0x7240 (ExifTool: IDC_IFD) points 6
  bytes before IFD2 in every 4.0 and later file, and 6 bytes before the raw
  SubIFD in every ARW 2.3 to 2.3.5 file, the target being six zero bytes;
  on the A200 to SLT-A55V and the R1 it points at zeros or `FF` bytes
  (files 4466, 1541, 2271, 3221 and the rest) **[F]**. A forum user had
  reported the "6 bytes" pattern on about 20 models and called it an
  ExifTool bug [31] **[S]**. Its meaning is unknown; raw-files should not
  follow it.
- **Image Data Converter edits.** Sony's converter writes IDC IFDs into
  SR2Private (0x7240, and 0x7241 with a stack of version IFDs, each able to
  carry an IDCPreview JPEG); it can turn the A100's 0x014A into a real SubIFD;
  and a source comment in ExifTool's MinoltaRaw.pm says the IDC utility
  "corrupts MRWInfo" when writing ARW images [5] [1] [6] **[P]**. ExifTool's
  author says he has never seen the un-numbered IDCPreviewImage [5] **[P]**. No
  checked file was edited (0x7241 = 0 everywhere) **[F]**. Whether the
  converter also replaces the IFD0 or IFD2 JPEG is unknown.
- **Third-party re-saves.** A published ILCE-9 dump has IFD0 at about
  byte 49,595,930 of a 49,600,238-byte file, with an XMP packet naming
  Photo Mechanic [18] **[P]**; which program moved it is **[I]**. All 36
  checked files have IFD0 at 8 **[F]**. Exiv2 0.23 wrote NEX-6 ARWs that
  other software rejected, ExifTool then reporting a bad SR2Private entry
  [34] **[S]**. Users report that ARWs edited by ExifTool open everywhere
  except, reportedly, Sony's Imaging Edge, which may assume fixed offsets
  [31] **[S]**.
- **Tags out of order.** Raw SubIFDs list 0x011A, 0x7000, 0x0111, 0x7316
  and 0x74C7 out of ascending order, and from 4.0 also 0x0102, 0x0142 and
  0x7038 (files 1541, 1542, 3012, 3251, 4466) **[F]**, as ExifTool's
  warnings on the published ILCE-9 dump show [18] **[P]**. A parser must not
  require sorted tags.
- **IFD1 repeats IFD0's Make, Model, Orientation and DateTime** in every
  ARW and SR2 (all files) **[F]**; ExifTool gives IFD1 low priority for ARW
  [7] **[P]**, and users who edited ModifyDate found the IFD1 copy left
  behind [32] **[S]**.
- **Make padding.** `SONY` plus 11 spaces on the A100, `SONY ` with one
  space on the A200 to A900, `SONY` after that, `HASSELBLAD` on the Lunar
  (files 2346, 2271, 5858, 4051) **[F]**. Nothing in preview finding should
  depend on Make.

## 6. How other implementations choose the largest preview

| Project | Method for ARW | Conf. |
|---|---|---|
| ExifTool | extracts each location under its own name (PreviewImage, ThumbnailImage, JpgFromRaw, IDCPreviewImage, the MakerNote previews); does not choose a largest; validates image data, repairs the A100 thumbnail offset, and drops the A100 duplicate | [2] [4] [5] **[P]** |
| Exiv2 | collects `Exif.Image` (IFD0), `Exif.Thumbnail` (IFD1), `Exif.Image2` (IFD2), `Exif.SubImage1` to 4, `Exif.SonyMinolta` (A100) and strip-based loaders for groups with NewSubfileType 1; checks offset plus size against the file; makes no SOI check of its own, but reads dimensions by opening each candidate's bytes through `ImageFactory`, which identifies the format by its magic bytes (`FF D8` for JPEG) and so drops a candidate that no Exiv2 image class recognises; sorts by width times height; does not read MakerNote 0x2001, whose offset lies outside the MakerNote [35] | [8] **[P]**; [35] **[S]** |
| LibRaw | parses all IFDs in the chain and the SubIFDs; for an IFD with 0x0201 and no BitsPerSample it reads the JPEG header for width, height and bits; picks the IFD maximising width times height divided by (bits squared plus 1) among IFDs that are not the raw and not CFA or linear-raw photometric; checks offsets against the file size only when an option is set; keeps a deduplicated list of all thumbnails; its raw selection skips the "Sony RGB preview" profile, which is IFD2 | [10] **[P]**; that IFD2 wins its ranking is **[I]** |
| rawler (dnglab) | `preview_image` reads IFD0's 0x0201/0x0202 only, so on 4.0 and later files it returns the 1616x1080 preview, not IFD2's | [12] **[P]**; the consequence **[I]** |
| rawspeed | no preview extraction | [13] **[P]** |
| libopenraw | walks every non-raw IFD and its SubIFDs; treats NewSubfileType absent or 1 as a preview; for Compression 6 or 7 or 0x0201 data **uses ImageWidth and ImageLength when present** and parses the JPEG only when they are absent; disables SubIFD parsing for the A100 | [14] **[P]**; that it would report the A100's 640x480 JPEG as 3872x2592 is **[I]** |
| rawkit (Graphite) | IFD0's 0x0201/0x0202 only; checks `FF D8`; allocates the declared length before any bounds check | [15] **[P]** |
| metadata-extractor | no preview extraction; reads both Sony MakerNote forms with the outer TIFF header as base | [16] **[P]** |
| Lightroom Classic | used the smaller of two ILCE-1 previews until a fix in 2021 | [24] **[S]** |
| Viewers in general | macOS, XnView, IMatch and Lightroom's import show the embedded preview, so Sony files look soft at 1:1 | [25] [26] **[S]** |

## 7. Recommendation for raw-files

Rank by what is measured in the JPEG itself, never by slot, by tag
dimensions (absent in IFD0 and IFD1, wrong on the A100), by byte length or
by FileFormat. raw-files ranks previews this way: by the pixel area their
JPEG frame header gives, and by byte length only between two of the same
area. A procedure of this shape (candidates from 0x0201/0x0202 pairs and
single strips, lossless SOFs rejected, ranking by SOF area) was **checked**
by a throwaway script on the 36 sample files, and the role filter of step 3
was checked against the tags of every IFD in those files (section 7.2).

The steps restate the common approach in original pseudocode terms. They
follow the NEF recommendation
([../nef/container-and-previews.md](../nef/container-and-previews.md)),
which already walks the whole IFD0 next-IFD chain and rejects lossless SOF
markers. The one new rule is the role filter of step 3; the points
specific to ARW files are marked **(ARW)**. Steps 6 and 8 are slightly
stricter than the NEF steps (`FF D8 FF` required; candidates without a SOF
dropped rather than ranked last); a shared finder must settle that (see
[open-questions.md](open-questions.md)).

### 7.1 Algorithm

1. **Header.** Accept `II` or `MM` with magic 42. Follow the IFD0 offset
   wherever it points.
2. **Walk the whole IFD0 chain.** Visit IFD0 and follow every next-IFD
   pointer: IFD1, then IFD2 on 4.0 and later files, until a pointer of 0.
   **(ARW)** The largest JPEG on 4.0 and later files is in IFD2; a walker
   that stops after IFD1, or that only reads IFD0, misses it. Also visit
   every offset in each 0x014A array, but only after checking that the
   target parses as an IFD: an entry count of 1 to about 1000 (the cap the
   NEF page takes from LibRaw), every entry inside the file, and every
   entry type a valid TIFF type. That rejects the A100's raw-data pointer
   without a model check: read as an IFD, its target in file 2346 gives an
   entry count of 7520 whose entries all lie inside the file, so only the
   count cap and the type check (the first types read 58818, 21228, …)
   catch it **[F]**. Keep a set of visited offsets so loops end, limit
   SubIFD nesting, and cap the number of IFDs. Do not require sorted
   tags. The MakerNote and SR2Private need not be parsed for previews.
3. **Skip IFDs that are raw by role (ARW).** Skip an IFD if any of these
   holds:
   - Photometric 32803 (CFA) or 32892 (ARQ);
   - Photometric 6 (YCbCr) with any BitsPerSample value above 8 (medium
     and small raws carry 15,15,15);
   - BitsPerSample present with any value above 8 (this also catches the
     SRF's 14-bit IFD0);
   - it has tile tags (0x0142 to 0x0145);
   - NewSubfileType is present and 0.

   Do **not** skip on Compression 7: IFD2's JPEG has Compression 7 and
   Photometric 6, and only its 8-bit samples and its NewSubfileType 1 tell
   it from a raw. Do not require NewSubfileType 1 either: the SRF's JPEG
   IFD has none.
4. **Locate the bytes.** Take offset and length from 0x0201 and 0x0202 if
   both exist. Otherwise, if 0x0111 and 0x0117 describe a single strip and
   Compression is 6 or 7, use that; an IFD with StripByteCounts but no
   StripOffsets (the SRF IFD0) yields nothing. Offsets are relative to the
   file start (section 1.5). Deduplicate candidates by offset.
5. **Bounds-check.** Compute the end with checked addition; reject a zero
   length, a start beyond the file, or an end beyond the file. Use the same
   truncation policy as for NEF.
6. **Check SOI.** The first three bytes must be `FF D8 FF`. Exiv2 has no
   explicit check, though opening the bytes to read the dimensions drops
   data that is not recognisably a JPEG or another known format [8];
   raw-files should check explicitly. Optionally, for the A100 thumbnail only,
   apply ExifTool's repair (section 5); since the thumbnail is the smallest
   image, skipping it is also acceptable.
7. **Parse to the first SOF and reject lossless frames.** Walk marker
   segments with length checks and a bounded loop; stop at the first
   start-of-frame marker and read height and width from it; do not decode.
   Reject the lossless frames `C3`, `C7`, `CB` and `CF`: on ARW they are raw
   tiles or reduced-size raws, never previews. Accept the DCT frames `C0`,
   `C1`, `C2`, `C5`, `C6`, `C9`, `CA`, `CD` and `CE`; every embedded preview
   seen is `C0` (all files) **[F]**. `C4`, `C8` and `CC` are not frame
   markers. Handle markers without a length (`01`, `D0` to `D7`), and give
   up at SOS (`DA`) or EOI without a SOF. The NEF recommendation carries
   the same rejection. **(ARW)** On ARW it matters in practice: it is the
   second line of defence behind step 3.
8. **Rank.** Largest SOF area wins; break ties by the larger byte length.
   A candidate without a valid SOF is not a preview: drop it.
9. **Return the bytes unmodified**, as declared by the tags (they may
   extend a few bytes past EOI, section 5), and expose IFD0's Orientation
   separately (section 8).
10. **No candidate**: return an error, never a panic.

### 7.2 Expected results on the checked files

Applied as written, the procedure returns these (all **[F]**):

| Files | Format | Returned | Size |
|---|---|---|---|
| 1351 | SRF | IFD1 | 640x480 |
| 3221 | SR2 | IFD0 | 640x424 |
| 2346 | ARW 1.0 | IFD0 (the Minolta pointer is the same JPEG) | 640x480 |
| 2271, 2521, 5858, 1457, 964, 5811, 3751, 4051, 1582, 1310, 1311, 1541, 1542, 3251 | ARW 2.0 to 2.3.5, Lunar | IFD0 | 1616x1080 |
| 3012, 3015 | ARW 2.3.3 at 1:1 and 16:9 | IFD0 | 1088x1080, 1920x1080 |
| 3255 | ARQ | IFD0 | 1616x1080 |
| 5418, 4466, 6929 to 6936, 6736, 7361, 7364, 7835, 8846 to 8850 | ARW 4.0 to 6.0 | IFD2 | the image's full size (section 4) |

Every raw IFD in the checked files meets at least one skip rule of step 3
(CFA or ARQ photometric, or BitsPerSample above 8, or tiles), and no IFD
holding a preview meets any; the A100's raw is never walked, because its
0x014A target is not an IFD **[F]**. In the lossless and YCbCr files the
raw would also fail step 7, its strip starting with an SOF3 tile **[F]**.

### 7.3 Why this shape

- One walk serves every member of the family. The formats differ in which
  IFD holds the largest JPEG (IFD1 in the SRF, IFD0 up to ARW 2.3.5, IFD2
  from 4.0), and that is exactly what ranking by measured area absorbs.
- Classifying an IFD by role, from several facts together, is needed
  because the obvious single tests fail on real files: Compression 7 is
  both the largest preview and the raw, Photometric 6 is both a preview
  and a raw, NewSubfileType is missing in the SRF, and the A100's tag
  dimensions describe something else.
- Reading only up to the SOF keeps the cost low: the SOF sits near the
  start of every JPEG checked (no APP segments), and the 10 MB JPEG in IFD2
  need not be read to be ranked.
- Firmware decides the FileFormat version (section 1.2), so code keyed on
  versions or models would go stale with each firmware update; the walk
  does not.

## 8. Orientation

Return the JPEG bytes as they are and expose the orientation separately.

- IFD0 tag 0x0112 is the value to use; IFD1 repeats it, IFD2 also carries
  one, and the raw SubIFD carries one on the R1 and the A200 (files 3221,
  2271, 4466) **[F]**.
- The embedded JPEGs have no APP1 segment and so no orientation of their
  own (all files) **[F]**. In file 3751 (SLT-A99V) IFD0 and IFD1 say 8, a
  portrait shot that needs rotating for display, while the preview's SOF
  is 1616x1080, landscape **[F]**: the camera stores previews unrotated,
  and the consumer must apply the Orientation.
- The MakerNote has its own copies: CameraOrientation in the enciphered tag
  0x9400, Orientation2 in two settings blocks (where 2 means "rotate 180",
  not a mirror), and bit fields on some older bodies [1] **[P]**. ExifTool
  saw A55 files where Orientation2 disagreed with the other tags [1]
  **[P]**. CameraOrientation sits at byte 0x28 of the deciphered 0x9400 in
  the 9400a layout, 0x24 in 9400b and 0x29 in 9400c [1] **[P]**; in file
  3751 (9400a) it says 8, agreeing with IFD0 **[F]**. Use IFD0 all the
  same.
- Exiv2's tracker has a 2008 report of A700 previews shown unrotated in
  digiKam, closed without analysis [33] **[S]**.
- LibRaw takes the orientation from the lowest-numbered IFD with a
  non-zero value [10] **[P]**, which on ARW is IFD0.

So the library should read IFD0's 0x0112 (default 1 when absent or
invalid) and leave the rotation to the caller, as for NEF.

## 9. Pitfalls

- The largest JPEG is in IFD2 on FileFormat 4.0 and later; walking only
  IFD0, or IFD0 and IFD1, or the SubIFDs, returns a 1616x1080 preview from
  a 50-megapixel file (section 3.1) **[F]**.
- Raw tiles begin `FF D8 FF C3`: an SOI check alone accepts them (section
  5) **[F]**.
- Compression 7 and Photometric 6 occur both in the largest preview and in
  raw IFDs (section 1.4) **[F]**.
- Tag dimensions are absent in IFD0 and IFD1, and on the A100 describe the
  raw, not the JPEG (file 2346) **[F]**.
- The A100's 0x014A is a data offset, not an IFD, and its 0xC634 target is an
  MRW block, not SR2Private; treating either as an IFD reads garbage (file
  2346) **[F]**. LibRaw's A100 special case once overflowed a fixed IFD array
  (CVE-2017-6887) [36] **[P]**.
- The SRF's IFD0 claims an image with no StripOffsets (file 1351) **[F]**.
  LibRaw's SRF parser had an out-of-bounds read (CVE-2020-35535) [36]
  **[P]**, and rawler panics on a short file at the SRF's fixed key offset
  [37] **[S]**, an index still unchecked in its source [12] **[P]**.
- A thumbnail's declared length is not checked by every reader against the
  file (LibRaw CVE-2020-15503) [36] **[P]**; rawkit allocates the declared
  length before checking it [15] **[P]**. Bound every length by the file
  size before allocating.
- NewSubfileType must be read as an unsigned 32-bit value: LibRaw narrowed
  it to a signed integer (CVE-2026-88387, whose fix was reverted) [36]
  **[P]**.
- 0xC634 is BYTE[4], not LONG; a strict type check loses SR2Private
  (all files) **[F]**. Not needed for previews, but a generic TIFF reader
  should not reject the IFD over it.
- IFD0 need not be at offset 8 after a re-save [18] **[P]**.
- Tags are out of ascending order in raw SubIFDs (section 5) **[F]**.
- The MakerNote's header does not follow Make (the Lunar) (file 4051)
  **[F]**, and newer MakerNotes have no next-IFD pointer **[F]**.
- A declared JPEG length may include bytes after EOI (section 5) **[F]**.
- FileFormat varies with firmware; do not key on it (section 1.2) **[F]**.
- Closed-source viewers have had memory-safety bugs on ARW and SR2 files
  whose root causes are not public (for example IrfanView CVE-2024-11520)
  [36] **[P]**. See [security.md](security.md) for the full list and the
  checks raw-files adopts.
- Cycles and absurd counts: use a visited set and caps (section 7.1).

## 10. Disagreements

1. **Does the ILCE-1 embed a full-size JPEG?** Forum users disagreed [23]
   **[S]**; desk research had concluded that Sony bodies mostly do not.
   Settled by file 4466 and the 15 other 4.0 and later files: the
   full-size JPEG is in IFD2 **[F]**. The users were extracting different
   ExifTool tags (section 3.1).
2. **Where LibRaw's "Sony RGB preview" IFD lives.** Reading LibRaw's
   source suggested a JPEG in a SubIFD [10] **[I]**; the profile is IFD2's,
   in the IFD0 chain (file 4466) **[F]**.
3. **The A100's MakerNote header.** ExifTool's comments and Exiv2's
   example tie the `SONY DSC` header to JPEG files and describe ARW
   MakerNotes as headerless [3] [9] **[P]**, while LibRaw's comment labels
   its `SONY` branch "DSLR-A100" [10] **[P]**. Settled by file 2346: the
   A100's ARW MakerNote has the 12-byte header **[F]**.
4. **The Hasselblad MakerNote header.** ExifTool recognises a
   `VHAB     \0` header for Hasselblad models [3] **[P]**; the Lunar's
   MakerNote is headerless (file 4051) **[F]**. Whether any Hasselblad raw
   uses the header is open.
5. **FileFormat per body.** ExifTool's comments, Clévy's table and the
   samples disagree for the A99V, the ILCE-7RM3, the ILCE-7M4 and the
   ILCE-1; the version follows firmware (section 1.2) **[F]**.
6. **Clévy's "ARW 4.0" example** describes an ILCE-9 file whose FileFormat
   is `3 3 3 0`, ARW 2.3.3 [17] [18] **[P]**, and so shows no IFD2.
7. **The A100's preview pointer base.** One reading of LibRaw has it
   relative to a MakerNote base [10] **[P]**; in file 2346 the Minolta
   pointer equals IFD0's file-relative offset **[F]**. These agree if
   LibRaw's base is the outer TIFF header, which sits at file offset 0
   **[I]**.
8. **The A100's 0x014A value**: 65,536 in a published dump [18] **[P]**,
   98,304 in file 2346 **[F]**. Different files; the value varies.
9. **Tag dimensions versus the JPEG.** libopenraw trusts tag dimensions
   when present; Exiv2 and LibRaw measure the JPEG [14] [8] [10] **[P]**.
   On IFD2 the tags match the SOF; on the A100's IFD0 they do not (files
   4466, 2346) **[F]**.
10. **The exiv2 dumps and the bytes.** The dump of file 4051 shows no
    FileFormat or SonyModelID, but the bytes hold `3 3 0 0` and 289
    [21] **[P]**, file 4051 **[F]**; the cause is probably Exiv2's choice of
    MakerNote parser by Make **[I]**.
11. **rawler's comment** names tag 0x2002 as ExifTool's MakerNote preview;
    ExifTool defines it as 0x2001 (0x2002 is Rating) [12] [1] **[P]**.

## Open questions

Each needs sample files from the public raw.pixls.us collection, inspected
with a dump tool and byte-by-byte reading.

1. **ZV-1M2** (FileFormat `4 0 0 0`, firmware 2.01): the exiv2 dump shows
   no IFD2. Check the bytes of that file for IFD1's next-IFD pointer and
   any large JPEG, and whether other 1-inch compacts at 4.0 and later lack
   IFD2.
2. **IFD2 on the bodies not byte-checked**: ILCE-7RM5, ILCE-7CR and
   DSC-RX1RM3 (9504x6336 per the dumps), ILCE-7CM2, ILME-FX2, ILME-FX3,
   ILME-FX30, ZV-E1, ZV-E10M2. Confirm SOF0, 8-bit, 4:2:2, no APP segment,
   and that the SOF size equals the tags.
3. **Aspect ratio on 4.0 and later**: in a file shot at 16:9, 1:1 or 4:3,
   does IFD2 carry the crop (as IFD0 does) or the full 3:2 DefaultCropSize?
   Measure IFD0 and IFD2 SOF sizes.
4. **4:3 IFD0 preview**: which files' PreviewImageSize says 1440x1080, and
   does their IFD0 SOF agree?
5. **A100 firmware 1.00**: a file whose IFD1 offset is 0x10000 too small,
   to confirm ExifTool's repair.
6. **Image Data Converter or Imaging Edge edits**: does an edited ARW gain
   IDCPreview JPEGs, are IFD0 or IFD2 rewritten, is the 0x7240 + 6 link
   kept, and do the IDC2 and version IFDs really use their own start as
   base?
7. **Third-party re-saves** (Photo Mechanic, ExifTool, Lightroom metadata
   writes): does IFD0 move, and does IFD2 survive the rewrite?
8. **The 0x7240 + 6 link**: what the six bytes and the offset mean, and
   whether a writer must keep the link.
9. **ARQ from 16 shots** (ILCE-7RM4 and later) and ARQs made from 4.0 and
   later frames: do they keep the source frame's IFD0, IFD1 and IFD2
   unchanged, as file 3255 does for a 4-shot ILCE-7RM3 set?
10. **ARW 6.0 bodies other than the ILCE-7M5** (ILCE-7RM6 and others when
    samples appear): the same IFD2 layout?
11. **RAW and HEIF shooting** on the ILCE-7M5 before firmware 1.01: which
    embedded JPEG carried the noise Sony's note describes?
12. **Hasselblad Stellar, HV and Lusso**: Make, MakerNote header (`VHAB` or
    none) and preview layout.
13. **DSC-V3 SRF**: the same IFD0 and IFD1 layout as the F828?
14. **SRF6**: does the 672x492 RGB image exist in file 1351, and is it
    worth exposing as a non-JPEG preview?
15. **Byte order**: any Sony raw file with an `MM` header.
16. **Non-baseline previews**: any Sony file whose embedded JPEG is
    progressive, not 4:2:2, or carries an APP segment.

## Sources

All retrieved October 2026. Some pages were read through search-result
snippets only; those are marked. Source code was read at these revisions:
ExifTool 2200871d (27 May 2026), Exiv2 60d59d67 (1 October 2026), LibRaw
7bfffe2e (9 October 2026), dnglab ae01bcb2 (30 August 2026), rawspeed
c10cc500 (10 October 2026), libopenraw f18f7c81 (4 October 2026), Graphite
7477cd2f (10 October 2026), Clévy's sony_raw a15634ee (24 May 2026).

[1] ExifTool, Sony.pm (v3.87) — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Sony.pm ; tag page https://exiftool.org/TagNames/Sony.html
[2] ExifTool, Exif.pm (v4.65), including the 0x0201 naming conditions and the A100 thumbnail repair — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Exif.pm ; tag page https://exiftool.org/TagNames/EXIF.html
[3] ExifTool, MakerNotes.pm — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/MakerNotes.pm
[4] ExifTool, WriteExif.pl — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/WriteExif.pl
[5] ExifTool, SonyIDC.pm (v1.08) — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/SonyIDC.pm
[6] ExifTool, Minolta.pm and MinoltaRaw.pm — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Minolta.pm , https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/MinoltaRaw.pm
[7] ExifTool, ExifTool.pm — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool.pm
[8] Exiv2, preview.cpp — https://raw.githubusercontent.com/Exiv2/exiv2/main/src/preview.cpp
[9] Exiv2, makernote_int.cpp, sonymn_int.cpp, minoltamn_int.cpp, tiffimage_int.cpp — https://raw.githubusercontent.com/Exiv2/exiv2/main/src/makernote_int.cpp (and siblings); MakerNote notes https://exiv2.org/makernote.html
[10] LibRaw, tiff.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/tiff.cpp
[11] LibRaw, sony.cpp and makernotes.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/sony.cpp
[12] rawler (dnglab), arw.rs — https://raw.githubusercontent.com/dnglab/dnglab/main/rawler/src/decoders/arw.rs
[13] rawspeed, ArwDecoder.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decoders/ArwDecoder.cpp
[14] libopenraw, src/sony.rs, src/tiff/container.rs, src/tiff/dir.rs — https://gitlab.freedesktop.org/libopenraw/libopenraw/-/raw/master/src/sony.rs (and siblings)
[15] Graphite rawkit, lib.rs and tiff/file.rs — https://raw.githubusercontent.com/GraphiteEditor/Graphite/master/libraries/rawkit/src/lib.rs
[16] metadata-extractor, ExifTiffHandler.java — https://raw.githubusercontent.com/drewnoakes/metadata-extractor/main/Source/com/drew/metadata/exif/ExifTiffHandler.java
[17] Laurent Clévy, "Description of Sony RAW file formats (SRF, SR2, ARW and ARQ)", v0.55 (24 May 2026) — https://github.com/lclevy/sony_raw
[18] ExifTool `-v` dumps in the same repository: SONY_A9_DSC04458.txt (ExifTool 12.41), RAW_SONY_A100.txt, RAW_SONY_R1.txt, RAW_SONY_DSC-F828.SRF.txt (ExifTool 11.52) — https://github.com/lclevy/sony_raw
[19] PRONOM fmt/191 (ARW 1.x), fmt/1127 (ARW 2.x), fmt/1126 (SR2) — https://www.nationalarchives.gov.uk/PRONOM/fmt/191.xml , https://www.nationalarchives.gov.uk/PRONOM/fmt/1127.xml , https://www.nationalarchives.gov.uk/PRONOM/fmt/1126.xml
[20] ExifTool issue 195, "Sony ARW 4.0 tags" (2023) — https://github.com/exiftool/exiftool/issues/195
[21] raw.pixls.us catalogue and its exiv2 dumps of the 209 Sony-family files (233 dumps, counting 24 non-ARW Hasselblad files in the catalogue's Sony listing), one per file at `https://raw.pixls.us/getfile.php/<id>/exif/<name>.exif.txt` — https://raw.pixls.us/json/getrepository.php?set=all ; for example the ILCE-1 compressed file 4465: https://raw.pixls.us/getfile.php/4465/exif/A1_full_compressed.ARW.exif.txt
[22] raw.pixls.us exiv2 dump of ZV-1M2 file 8461 — https://raw.pixls.us/getfile.php/8461/exif/_DSC5498.ARW.exif.txt
[23] Lightroom Queen forum, "Sony A1 support" (2021) — https://lightroomqueen.com/community/threads/sony-a1-support.42388/
[24] Adobe Community, "Can I use the large embedded previews created by Sony A1" (2021) — https://community.adobe.com/t5/lightroom-classic-bugs/p-can-i-use-the-large-embedded-previews-created-by-sony-a1/idi-p/12389496
[25] Adobe Community, "Blurry Sony raw images on SD before import" (2023) — https://community.adobe.com/t5/lightroom-classic-discussions/blurry-sony-raw-images-on-sd-before-import/m-p/14188843
[26] Fred Miranda forum, A7R II embedded JPEGs (snippets only) — https://www.fredmiranda.com/forum/next/1502982 ; XnView forum, ARW thumbnail and preview (snippet only) — https://newsgroup.xnview.com/viewtopic.php?p=116403
[27] ExifTool forum, Sony preview and MPF threads (snippets only) — https://exiftool.org/forum/index.php?msg=42918 , https://exiftool.org/forum/index.php?msg=25005
[28] Exiv2 forum topic 3341, "Extract PreviewImage (not thumbnail)" (2019) — https://dev.exiv2.org/boards/3/topics/3341
[29] gThumb issue 159, "Aspect ratio - Sony ARW files" — https://gitlab.gnome.org/GNOME/gthumb/-/issues/159
[30] Capture One community, "16:9 aspect ratio" (snippet only) — https://support.captureone.com/hc/fr/community/posts/13768016885149-16-9-aspect-ratio
[31] ExifTool forum, Imaging Edge and edited ARWs, and the IDC offset report (snippets only) — https://exiftool.org/forum/index.php?msg=51532 , https://exiftool.org/forum/index.php?msg=60493
[32] ExifTool forum, ModifyDate in IFD1 of ARW (snippets only) — https://exiftool.org/forum/index.php?msg=48119 , https://exiftool.org/forum/index.php?msg=81168
[33] Exiv2 tracker issue 545, A700 ARW rotation (2008–2016) — https://dev.exiv2.org/issues/545
[34] Exiv2 forum topic 1347, NEX-6 ARW rewritten by Exiv2 0.23 (2012–2013) — https://dev.exiv2.org/boards/3/topics/1347
[35] Exiv2 tracker issue 1143, A77 JPEG preview (2015–2016) — https://dev.exiv2.org/issues/1143
[36] NVD records CVE-2017-6887, CVE-2020-35535, CVE-2020-15503, CVE-2026-88387, CVE-2024-11520 — https://nvd.nist.gov/vuln/detail/CVE-2017-6887 , https://nvd.nist.gov/vuln/detail/CVE-2020-35535 , https://nvd.nist.gov/vuln/detail/CVE-2020-15503 , https://nvd.nist.gov/vuln/detail/CVE-2026-88387 , https://nvd.nist.gov/vuln/detail/CVE-2024-11520
[37] dnglab issue 574, panic at the SRF key offset — https://github.com/dnglab/dnglab/issues/574
[38] Sony Alpha Rumors, ILCE-7M5 firmware 1.01 (12 February 2026) — https://www.sonyalpharumors.com/?p=143029
[39] Sony, Pixel Shift Multi Shooting support pages (snippets only) — https://support.d-imaging.sony.co.jp/support/ilc/psms/ilce1/en/index.html , https://support.d-imaging.sony.co.jp/support/ilc/psms/ilce7rm4/en
[40] Popular Photography, Hasselblad Lunar hands-on (snippet only) — https://www.popphoto.com/gear/2012/09/hands-hasselblad-lunar-interchangeable-lens-compact-seriously-beefed-nex-7

### Sample files

All CC0, from https://raw.pixls.us/ ; the SHA-256 was checked before any
parsing. Mode is the catalogue's label.

| id | Body | Mode | SHA-256 | URL |
|---|---|---|---|---|
| 964 | SLT-A55V | 12-bit compressed | `f861454a49abe0345ff69887874d00617cdf0b96af96d605051e68deda424a08` | https://raw.pixls.us/getfile.php/964/nice/Sony%20-%20SLT-A55%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 1310 | ILCE-7RM2 | 12-bit compressed | `a606ebf1dcc6704f03f2966326c60a87dd6bdb3704464b3897c10349a10d3a19` | https://raw.pixls.us/getfile.php/1310/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 1311 | ILCE-7RM2 | 12-bit uncompressed | `71e0888396ef52c7e6a990b4e72ebb4d8413a1fc1c35bada449870539cef6e39` | https://raw.pixls.us/getfile.php/1311/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20uncompressed%20(3:2).ARW |
| 1351 | DSC-F828 | SRF | `07739f94bcab0c7e198d418ac65172d0dce04bb84a7683ecffd0bfe4b32f6d79` | https://raw.pixls.us/getfile.php/1351/nice/Sony%20-%20DSC-F828%20-%204:3.SRF |
| 1457 | NEX-5 | 12-bit compressed | `3b085569ac1fec37a1289ed5ea517497692a3ea14cea41953dd6e878abf230a3` | https://raw.pixls.us/getfile.php/1457/nice/Sony%20-%20NEX-5%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 1541 | ILCE-9 | 14-bit compressed | `598bbc828c279528f504c51a10418521b17a90a25b198368f65e19cfda9e3711` | https://raw.pixls.us/getfile.php/1541/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20compressed%20(3:2).ARW |
| 1542 | ILCE-9 | 14-bit uncompressed | `22c2158c9796386dba28d7cf040a1ca70f4ec2c927ec50771b78bfb523e21c29` | https://raw.pixls.us/getfile.php/1542/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW |
| 1582 | ILCE-7S | 14-bit compressed, APS-C crop | `a35ebb2fbec929daa5beb20d1ce5c15a8aac7b1a7a231455387f3df8a7442e07` | https://raw.pixls.us/getfile.php/1582/nice/Sony%20-%20ILCE-7S%20-%2014bit%2014bit%20compressed%20(3:2).ARW |
| 2271 | DSLR-A200 | 12-bit compressed | `04eacd403d92e0c90da9d91b87615af8eda1afc963350d3ab774557c36763eef` | https://raw.pixls.us/getfile.php/2271/nice/Sony%20-%20DSLR-A200%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 2346 | DSLR-A100 | compressed | `4df7981ac4baec9648b4f106751ea4b09a6e27bc70a15f8ef09bec051fec362d` | https://raw.pixls.us/getfile.php/2346/nice/Sony%20-%20DSLR-A100%20-%20compressed%20(3:2).ARW |
| 2521 | DSLR-A700 | 8-bit compressed | `3159e28892bf0771d01525cd4b6190f9c15bbb19fa2fab6d2515ced0594e8f40` | https://raw.pixls.us/getfile.php/2521/nice/Sony%20-%20DSLR-A700%20-%208bit%208bit%20compressed%20(3:2).ARW |
| 3012 | DSC-RX100M6 | 12-bit compressed, 1:1 | `53c5abad853f17d2c46d9f96f8ff4e0af0ea095193fecf61222a8bf9d8beb00b` | https://raw.pixls.us/getfile.php/3012/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(1:1).arw |
| 3015 | DSC-RX100M6 | 12-bit compressed, 16:9 | `fe9e4df604fefbcc0ed78fb4a1d9a58dbaa1f4011e3d119b3ee32fc583d43907` | https://raw.pixls.us/getfile.php/3015/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(16:9).arw |
| 3221 | DSC-R1 | SR2, 14-bit uncompressed | `921c5f2513dd1671a9089e73fc778fc3573bc4cb70a3a555d0414661d0e62274` | https://raw.pixls.us/getfile.php/3221/nice/Sony%20-%20DSC-R1%20-%2014bit%2014bit%20uncompressed%20(3:2).SR2 |
| 3251 | ILCE-7RM3 | 14-bit uncompressed, pixel-shift frame | `eaedff10f11b7bdd7908db40d8e1f541bd29a89369b897f181b87c29d549f0a4` | https://raw.pixls.us/getfile.php/3251/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW |
| 3255 | ILCE-7RM3 | ARQ | `67af1f9f4cff82bd182f3fc48e95d39ae7d8352595e3c72f3386c9dee114f27d` | https://raw.pixls.us/getfile.php/3255/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARQ |
| 3751 | SLT-A99V | 12-bit compressed | `bc033b8b0df5b1ef603ead4b57f979b8a0a46e7a52d8b46ac6dfa22f86c243f8` | https://raw.pixls.us/getfile.php/3751/nice/Sony%20-%20SLT-A99%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 4051 | Hasselblad Lunar | 12-bit | `029b1283a3d1226e08fc1eb3b81874347f5c128e437f78f853436fd6ba15873b` | https://raw.pixls.us/getfile.php/4051/nice/HASSELBLAD%20-%20Lunar%20-%2012bit%20(3:2).ARW |
| 4466 | ILCE-1 | 14-bit lossless | `4012536ee0f938d8c5c5e932bb7953a03134abbd9f3f4c664cb2526c1903d492` | https://raw.pixls.us/getfile.php/4466/nice/Sony%20-%20ILCE-1%20-%2014bit%20(4:3).ARW |
| 5418 | ILCE-7SM3 | 14-bit compressed | `17e35d98d448b44164ecf414f980e9248b5829233e83a1adbee396b6c1a1a0c6` | https://raw.pixls.us/getfile.php/5418/nice/Sony%20-%20ILCE-7SM3%20-%2014bit%2014bit%20compressed%20(3:2).ARW |
| 5811 | DSC-RX100 | 12-bit compressed | `579a485b5126a25cbd55cbd5dadfa7d09cf021c99cc7d4869f9e56e3f759390b` | https://raw.pixls.us/getfile.php/5811/nice/Sony%20-%20DSC-RX100%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 5858 | DSLR-A900 | 12-bit compressed | `ac7c1532df77c321e8010aa1be60c1b9f245b1ac0db6f38fe617a73aae81af50` | https://raw.pixls.us/getfile.php/5858/nice/Sony%20-%20DSLR-A900%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 6736 | ILCE-6700 | 14-bit lossless | `dc3e2ed22f46fcef778553767e0703bc37cfb043f107afe2d7e6e4dbca34200a` | https://raw.pixls.us/getfile.php/6736/nice/Sony%20-%20ILCE-6700%20-%2014bit%20(4:3).ARW |
| 6929 | ILCE-7M4 | lossless, large | `851b43c2116c4139104a5036f83ac3b6a148789b2142214dd7192c13972b25b6` | https://raw.pixls.us/getfile.php/6929/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(4:3).ARW |
| 6930 | ILCE-7M4 | lossless, medium | `d453005714327addd75bcb99c1c6223173dc92f2a59542d82e6761ef0e0e7571` | https://raw.pixls.us/getfile.php/6930/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW |
| 6931 | ILCE-7M4 | lossless, small | `cbbd0930c7d8706dff84c68a2004454266e6fd0d8354f5f76a106b5d776e0223` | https://raw.pixls.us/getfile.php/6931/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW |
| 6933 | ILCE-7M4 | 14-bit uncompressed, APS-C | `bc0089c821a3711ebbeec3844cde5e5c247ac7cf9ca8ad9ae03a977b232216b2` | https://raw.pixls.us/getfile.php/6933/nice/Sony%20-%20ILCE-7M4%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW |
| 6936 | ILCE-7M4 | 14-bit compressed, APS-C | `639a6d4db881f1359e3ea7e1137b314e59417d24e6d4d862b7202111a2c823b2` | https://raw.pixls.us/getfile.php/6936/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(3:2).ARW |
| 7361 | ILCE-9M3 | 14-bit compressed | `81b0b140f34356964fdb4adf5e202eb01d5e5e5a1b61b861954a183fc30d4bf0` | https://raw.pixls.us/getfile.php/7361/nice/Sony%20-%20ILCE-9M3%20-%2014bit%20(3:2).ARW |
| 7364 | ILCE-9M3 | lossless, small | `3d9da593a027bdbf31589eb950b04be7ce3f3dc6105ca339c5b7702ffa7fefdd` | https://raw.pixls.us/getfile.php/7364/nice/Sony%20-%20ILCE-9M3%20-%203:2.ARW |
| 7835 | ILCE-1M2 | 14-bit lossy, APS-C | `050c924e5f78afd8479aa442f17b8650e0a5ebfa5757e57368cb0dc865ca2417` | https://raw.pixls.us/getfile.php/7835/nice/Sony%20-%20ILCE-1M2%20-%2014bit%20Lossy%20(3:2).ARW |
| 8846 | ILCE-7M5 | 14-bit compressed HQ | `4e3a99cbcf2a348decaf3ca68c5de22b7a05eaa590f4e9184735c1f9c45213a7` | https://raw.pixls.us/getfile.php/8846/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
| 8847 | ILCE-7M5 | 14-bit compressed | `46d73475dfd8f96fd7e2f4d4a721a36cbbaba3d6ea95a4851809cd9d252b2a80` | https://raw.pixls.us/getfile.php/8847/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
| 8848 | ILCE-7M5 | 14-bit lossless, APS-C | `51ad709cba01a5f05e3c723b779f6b5fda8fa7b378494dbb40b4d3d3aa5494a4` | https://raw.pixls.us/getfile.php/8848/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(4:3).ARW |
| 8849 | ILCE-7M5 | 14-bit compressed HQ, APS-C | `c16cae99d26a2f8ad183d31a23823310a994aa058d55cfe56e4f82ba56865f57` | https://raw.pixls.us/getfile.php/8849/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
| 8850 | ILCE-7M5 | 14-bit compressed, APS-C | `8315af2a9889709f4c836dbaadc23d226847e5a0630fda212a9f896d5fe0569f` | https://raw.pixls.us/getfile.php/8850/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
