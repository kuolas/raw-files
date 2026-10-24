# Sony MakerNote and viewer metadata

This page describes the Sony MakerNote found in ARW files (and in the older
SR2 and SRF files and the DSLR-A100's ARW 1.0): where it is and how its
variants are told apart, the tags that matter, the two unrelated ciphers Sony
uses, the SR2Private block that carries the decoding parameters, and where a
viewer finds orientation, capture date, make, model, image size, lens and
the point the camera focused on. It serves milestone 3 (basic metadata),
milestone 4 (focus points) and, for the decoding-related tags, milestone 6
(raw data).

See [the ARW overview](README.md). The TIFF container and the preview JPEGs
(milestone 2) are in [container-and-previews.md](container-and-previews.md);
decoding of the sensor data is in [raw-data.md](raw-data.md); parser
hardening is in [security.md](security.md).

Sony publishes no specification for any of this. Everything below comes from
the Exif and TIFF standards, the tag documentation of ExifTool and exiv2, and
the source of existing decoders, read as references only (their licences do
not allow copying code; see [Licences](#licences)), plus structural checks
on 36 CC0 sample files from raw.pixls.us made in October 2026. The two
ciphers and the SRF key chain described below were checked on those samples
with throwaway scripts written from the prose on this page; they are not
implemented in raw-files, and the scripts are not in the repository. No
pixel output was compared with a reference decoder. The camera settings
(section 9.7) were also compared with ExifTool's output, ExifTool being
run on the files as an independent reader; none of its code is in
raw-files.

Claims carry a confidence tag: **[P]** primary (specification, tag
documentation or implementation source), **[S]** secondary (write-ups, forum
posts, sample listings), **[I]** inferred, and **[F]** measured on a named
sample file. Samples are cited as "file 2346", the raw.pixls.us file id; the
files cited here are listed under [Sources](#sources) with URL and SHA-256,
and all checked files are in [test-data.md](test-data.md#files-checked).
Where a sample contradicts a source, the sample wins and the disagreement is
recorded.

Sony's camera JPEGs carry the same MakerNote, behind a header. What five of
them show, the ILCE-7M3, ILCE-7RM2, DSC-RX100M4, ILCE-6000 and DSLR-A100
JPEGs from Wikimedia Commons listed in the
[JPEG test data](../jpeg/test-data.md#2-the-sample-files) and cited as "the
ILCE-7M3 JPEG", is in [section 1](#in-a-jpeg-file) and
[section 9.6](#96-focus-point); how raw-files reads a JPEG is decided in
[ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)
and described in the [JPEG notes](../jpeg/README.md).

Versions are named as ExifTool names them ("ARW 2.3.3"), with the four bytes
of MakerNote tag 0xB000 where it matters ("3 3 3 0").

## Contents

- [1. Finding the MakerNote](#1-finding-the-makernote)
- [2. Key tags](#2-key-tags)
- [3. Two ciphers](#3-two-ciphers)
- [4. The MakerNote substitution cipher](#4-the-makernote-substitution-cipher)
- [5. SR2Private and the SR2SubIFD](#5-sr2private-and-the-sr2subifd)
- [6. The SR2 keystream](#6-the-sr2-keystream)
- [7. Raw-relevant tags](#7-raw-relevant-tags)
- [8. White balance](#8-white-balance)
- [9. Viewer metadata](#9-viewer-metadata)
- [10. A100, SR2 and SRF](#10-a100-sr2-and-srf)
- [11. Robustness pitfalls](#11-robustness-pitfalls)
- [12. Recommendation for raw-files](#12-recommendation-for-raw-files)
- [13. Open questions](#13-open-questions)
- [Sources](#sources)

## Licences

Every mature Sony implementation is GPL, LGPL, CDDL or Artistic licensed
(ExifTool, exiv2, LibRaw, rawspeed, rawler, libopenraw), which is
incompatible with the raw-files MIT OR Apache-2.0 licence, or is dcraw, whose
position is informal (see
[licences-and-references.md](licences-and-references.md)). This page
therefore restates algorithms in prose; do not paste code or tables from
those projects into raw-files. [P]

Neither cipher needs a copied table. The substitution cipher's inverse is
computed from its formula ([section 4](#4-the-makernote-substitution-cipher)),
and the SR2 keystream is a short generator described in full in
[section 6](#6-the-sr2-keystream). The per-body ModelID list and similar
tables are not reproduced here; cite them by location. [P][I]

## 1. Finding the MakerNote

The MakerNote is Exif tag 0x927C (type UNDEFINED) in the Exif IFD. [P][4]
Sony has used many shapes, mostly in JPEGs from compacts. Projects name them
differently (ExifTool: MakerNoteSony to MakerNoteSony5 plus
MakerNoteSonySRF; exiv2: Sony1 and Sony2), so this page names them by shape
and raw-files should detect them by bytes. [P][2][3][6][7]

| Layout | Header | IFD starts at | Offsets relative to | Byte order | Seen in |
|---|---|---|---|---|---|
| Headerless | none | MakerNote + 0 | the outer TIFF header | the outer file's | every ARW, SR2 and ARQ sample except the A100, including the DSC-R1's SR2 (file 3221) and the Hasselblad Lunar (file 4051) [F] |
| `SONY DSC` header | `SONY DSC ` then three NUL bytes (12 bytes) | MakerNote + 12 | the outer TIFF header | ExifTool detects it; LibRaw and dcraw force little-endian; exiv2 uses the file's | the DSLR-A100's ARW 1.0 (file 2346) [F]; every Sony camera JPEG checked, from the DSLR-A100's to the ILCE-7M3's ([in a JPEG file](#in-a-jpeg-file)) [F]; JPEGs from DSC compacts [P] |
| Other 12-byte headers | `SONY CAM `, `SONY MOBILE`, `\0\0SONY PIC\0`, `VHAB` + five spaces + NUL | MakerNote + 12 | the outer TIFF header | as above | listed in ExifTool's MakerNote types page [27]; seen in no raw sample [P][F] |
| Olympus-format | `SONY PI\0` (IFD at + 12) or `PREMI\0` (+ 8), with Olympus tag tables | as stated | the outer TIFF header | detected | JPEGs of early compacts (DSC-S650 and others) [P][27] |
| Text block | `SONY PIC\0` | not an IFD | n/a | n/a | JPEGs of some compacts [P][27] |
| SRF chain | none | MakerNote + 0, seven chained IFDs SRF0 to SRF6 | the outer file | the file's | DSC-F828 (file 1351) and DSC-V3 SRF files [P][F] |

Sources: [2][3][6][7][9][11]; the header bytes are also on ExifTool's
MakerNote types page [27], documentation, which is what raw-files codes the
headers seen in no sample from. ExifTool also knows a `SEMC MS\0` header from
Sony Ericsson phones; Sony phones write DNG, not ARW, so it does not concern
raw-files. [P][3]

Two claims found in the sources did not survive the samples:

- One reading of ExifTool's comments is that every ARW MakerNote is
  headerless. The A100 is the exception: file 2346 starts `SONY DSC ` and
  three NUL bytes, with a 28-entry IFD at + 12. [F] LibRaw's source labels
  its `SONY` branch "DSLR-A100", which fits. [P][11]
- The Hasselblad rebadges were said to use the `VHAB` header. The Lunar
  (file 4051, a NEX-7 underneath) is headerless; its MakerNote starts
  `4C 00`, a little-endian count of 76 entries. [F] Whether the Stellar, HV
  and Lusso use `VHAB` is not known; raw.pixls.us has none of them.

### Detection by bytes

1. If the MakerNote starts with `SONY DSC \0\0\0`, `SONY CAM \0\0\0`,
   `SONY MOBILE`, `\0\0SONY PIC\0` or `VHAB     \0`, the IFD is at + 12.
2. If it starts with `SONY PI\0`, `PREMI\0` or `SONY PIC\0`, it is an
   Olympus-format or text MakerNote from a compact's JPEG; ignore it.
3. Otherwise, if Make (trimmed, compared without regard to case) is `SONY`,
   or is `HASSELBLAD` with a Sony-built Model (Lunar, Stellar, HV, Lusso),
   the IFD is at + 0.
4. In every case, check that the first u16 is a plausible entry count (1 to
   about 1000) and that the entries parse inside the file; otherwise treat
   the MakerNote as absent.
5. An IFD at + 0 with a single entry (tag 0x0003) followed by a next pointer
   is SRF0, the start of an SRF chain ([section 10](#10-a100-sr2-and-srf)).
   [I]

Test the signatures before Make. The projects disagree here: ExifTool tests
all the byte patterns above, exiv2 recognises only `SONY DSC \0\0\0` and
parses everything else from a `SONY` Make as headerless, and LibRaw and
dcraw accept any `SONY` prefix, skip 12 bytes and force little-endian. A
`SONY PI\0` or `SONY PIC\0` MakerNote is misread by the last approach, and a
`VHAB` one takes LibRaw's + 0 path and fails its entry-count check. [P][3][7]
[11][14][I] On the samples only the headerless, `SONY DSC` and SRF layouts
occur, and the projects agree on them. [F]

The Make test must accept `HASSELBLAD`: the Lunar contains no `SONY` string
anywhere, so a Sony check keyed on Make alone rejects a Sony-built file.
[F] (file 4051)

### Layout details

- **Offsets are absolute.** In both the headerless and the `SONY DSC`
  layouts, value offsets count from the outer TIFF header, which is at file
  offset 0 in every sample. In file 2346 the A100's MinoltaMakerNote offset
  (0xB028) is 1612, and a Minolta IFD parses at file offset 1612. [P][3][F]
  Anything that moves the MakerNote without fixing its offsets breaks them.
- **Byte order.** All 36 samples are little-endian (`II`), and the MakerNote
  uses the file's order. [F] The DSLR-A100 JPEG is big-endian, and its
  MakerNote follows the file's order too: its 28 entries read only in
  big-endian order. [F] Inner blocks with a fixed order are noted where
  they occur.
- **No next-IFD pointer.** The four bytes after the IFD entries are a zero
  pointer on older bodies and the start of the value data on newer ones.
  Measured: four zero bytes on the A100, A200, A700, A900, NEX-5, SLT-A55V
  and the R1's SR2; value data immediately on the RX100, SLT-A99V, Lunar and
  every ILCE and RX body from 2012 on. [F] This matches ExifTool's
  relocation heuristic, which expects a 4-byte gap for DSLR-, SLT-A33/A35/
  A55V and NEX-3/5/C3/VG10E bodies and none for the rest. [P][3] Never
  require or follow a next pointer in a Sony MakerNote.
- **Where it sits.** The MakerNote starts at byte 870 to 1234 in files with
  FileFormat up to `3 3 1 0` (files 3221, 2346) and at byte 5144 to 5294
  from `3 3 3 0` on (files 3012, 4466), because IFD0 grew by an XMP
  packet. [F] No `3 3 2 0` file was checked, so where the move happened
  between the two is open. Follow the offsets; never assume a position.
- **Tag order.** Sony's raw SubIFD has tags out of ascending order (on the
  ILCE-9: 0x011A, 0x7000, 0x0111, 0x7316 and 0x74C7; files 1541, 1542), and
  ExifTool allows out-of-order tags in Sony IFDs. Do not require sorted
  tags in any Sony IFD. [F][P][2]

### In a JPEG file

In a camera JPEG the MakerNote is an entry of the Exif IFD inside the Exif
segment, an APP1 segment that holds a TIFF structure of its own; that
structure's header, 12 bytes into the file when the segment comes first,
is the base of its offsets
([JPEG notes](../jpeg/container-and-metadata.md#4-exif-the-app1-segment)).
[P] In a JPEG, what this page calls the outer TIFF header is that header.

- **Every Sony camera JPEG checked has the `SONY DSC` header**: `SONY DSC `
  and three zero bytes, then the IFD at + 12, on the ILCE-7M3 (102
  entries), ILCE-7RM2 (97), DSC-RX100M4 (93), ILCE-6000 (94) and DSLR-A100
  (28) JPEGs, while every ARW sample but the A100's is headerless. [F]
- **Its offsets count from the Exif segment's TIFF header**, as an ARW's
  count from the start of the file: read so, FocusLocation parses on the
  ILCE-7M3, ILCE-7RM2 and DSC-RX100M4 JPEGs, and every focus tag's value
  lies inside the length the MakerNote declares. [F]
- **Its byte order is the file's**: little-endian on four files,
  big-endian on the DSLR-A100's. A reader that forces little-endian, as
  the table above records of LibRaw and dcraw, would misread the A100
  JPEG's MakerNote. [F][I]
- **One entry points outside the Exif segment.** The ILCE-6000 JPEG's tag
  0x2001, PreviewImage, which ExifTool's tables describe as an HD-size
  preview in JPEG images [P][1], declares 549,518 bytes at an offset that
  lies inside the 1616 × 1080 MPF image after the main image. raw-files
  drops such an entry when it reads the MakerNote, as it drops any entry
  whose value lies outside the MakerNote, and no reader in raw-files reads
  that tag. [F]

## 2. Key tags

IDs are MakerNote tags unless stated. Types are as documented by ExifTool;
read integers from the bytes at the declared count rather than trusting the
declared type (see [section 11](#11-robustness-pitfalls)). "Enc." says
whether the block is enciphered with the substitution cipher of
[section 4](#4-the-makernote-substitution-cipher). "M" is the roadmap
milestone that needs the tag; "none" means nice-to-have. [P][1][2]

| ID | Name | Enc. | Meaning | M |
|---|---|---|---|---|
| 0x0010 | CameraInfo | no | A-mount and NEX AF and lens data, the AF point among them by number; layout chosen by byte count | 4 (point numbers) |
| 0x0112 | WhiteBalanceFineTune | no | u32; the white balance fine-tuning, 0 on every sample; [section 9.7](#97-camera-settings) [F] | 3 |
| 0x0114 | CameraSettings | no | layout by count; aspect ratio, drive mode, a Sony-coded orientation | none |
| 0x0115 | WhiteBalance | no | u32; the white balance setting: 0 auto, 16 daylight, 48 shade, 112 custom and others; not on the F828, R1 and A100 [F] | 3 |
| 0x2001 | PreviewImage | no | a JPEG-file feature; absent from every ARW, SR2 and ARQ sample [F] | none |
| 0x2010 | Tag2010 | yes | settings, a date string, aspect ratio; not written from the ILCE-7SM3 on [F] | none |
| 0x201A | ElectronicFrontCurtainShutter | no | u32; the electronic front curtain setting, 0 off, 1 on; not the shutter used [F] | 3 |
| 0x201B | FocusMode | no | 0 manual, 2 AF-S, 3 AF-C, 4 AF-A, 6 DMF, 7 AF-D; [section 9.6](#96-focus-point) | 4 |
| 0x201D | FlexibleSpotPosition | no | the spot chosen in Flexible Spot mode, on a 640 × 480 grid | none |
| 0x201E | AFPointSelected | no | the selected point or zone, by number | 4 (point numbers) |
| 0x2020 | AFPointsUsed | no | a mask of the points used | 4 (point numbers) |
| 0x2022 | FocalPlaneAFPointsUsed | no | a mask of the on-sensor focus points used | 4 (point numbers) |
| 0x2027 | FocusLocation | no | u16[4]: frame width and height, centre x and y of the focus area | 4 |
| 0x2029 | RAWFileType | no | 0 compressed, 1 uncompressed, 2 lossless, 3 Compressed RAW 2 | 6 (hint) |
| 0x202E | Quality | no | u16[2]; first value 1 RAW, 2 S-size, 3 M-size, 4 compressed, 5 compressed HQ; APS-C files say 3 too, and the ILCE-7M5 gives the compression whatever the size [F] | 3, 6 (hint) |
| 0x202F | PixelShiftInfo | no | group ID and shot number; marks ARQ composites | none |
| 0x2037 | FocusFrameSize | no | 6 bytes: the focus area's width and height and a third value | 4 |
| 0x2044 | HiddenInfo | no | offset and size of an unknown block | none |
| 0x204A | FocusLocation2 | no | u16[4], as FocusLocation | 4 |
| 0x3000 | ShotInfo | no | includes a plain date string at byte 6 | none |
| 0x900B | Tag900b | yes | face detection | none |
| 0x9050 | Tag9050 | yes | shutter count, LensType2, internal serial, the shutter used and the APS-C mode flag; [section 9.7](#97-camera-settings) | none |
| 0x9400 | Tag9400 | yes | sequence numbers, CameraOrientation, the shutter type on the RX100 bodies | none |
| 0x9401 to 0x9406, 0x940C, 0x940E, 0x9416 | Tag94xx | yes | ISO, lens, focus and AF data, and in 0x9416 the APS-C mode flag; 0x9416 replaces 0x9405 from the ILCE-7SM3 on [F] | none |
| 0xB000 | FileFormat | no | u8[4] format version | 3, 6 |
| 0xB001 | SonyModelID | no | u16 body identifier | 3, 6 |
| 0xB021 | ColorTemperature | no | u32; kelvin, 0 unless the white balance is custom on the samples [F] | 3 |
| 0xB027 | LensType | no | A-mount lens ID; 65535 for E-mount lenses | 3 (lens) |
| 0xB028 | MinoltaMakerNote | no | A100 only: offset of a Minolta-format IFD; 0 means absent | none |
| 0xB02A | LensSpec | no | 8 bytes: flags, BCD focal and aperture range, flags | 3 (lens) |
| 0xB02B | FullImageSize | no | u32[2], **height first**; the camera's output size | 3 (cross-check) |
| 0xB02C | PreviewImageSize | no | u32[2], height first; the IFD0 preview's size | 2 (cross-check) |
| 0xB054 | WhiteBalance | no | u16; a second white balance code, "auto" on two samples set to shade; not the setting [F] | none |

Notes:

- **Essential for milestone 3:** 0xB000 and 0xB001, plus 0xB027 and 0xB02A
  if the API reports a lens. Everything else is optional, and nothing in the
  table needs deciphering for milestone 3. [P][F]
- **0x2029 and 0x202E** agree with their ExifTool meanings on the samples:
  0x2029 is 0 on a compressed ILCE-7RM2 file (1310), 1 on uncompressed
  files (1311, 1542), 2 on lossless files (4466, 6929, 8848) and 3 on both
  ARW 6.0 compressed modes (8846, 8847); it is absent before the ILCE-7RM2.
  0x202E reads [1, 2] on a lossless large file (6929), [3, 2] medium (6930),
  [2, 2] small (6931), [4, 0] compressed and [5, 0] compressed HQ (8847,
  8846). But APS-C Bayer files also say 3 (6933, 6936, 7835), so 3 does
  not mean a YCbCr medium raw. Neither tag is enough to choose a decoder.
  [P][1][F]
- **0x2044** appears from ARW 4.0.1 on; its block (53,248 bytes, at an
  absolute offset) begins `55 26 11 05 00`. Its content is unknown. [P][F]
- **0x202F** reads `… 0 4` in the ARQ composite (file 3255) and `… 1 4` in
  the source frame (file 3251). [F]
- **Serial numbers.** Exif BodySerialNumber (0xA431) is present in the
  ILCE-1 and ILCE-7M5 samples only (files 4466, 8846 to 8850); MakerNote
  0x2031 exists only on ILCE-9 firmware 2.00. [P][1][F] raw-files reports
  the Exif one and reads no MakerNote serial, so the other bodies have none
  ([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)).
- **Tag order trap.** 0xB001 comes after 0x2010, 0x9050 and the 0x94xx tags
  in IFD order, so anything that chooses a block layout by ModelID must parse
  the whole IFD first. LibRaw buffers those blocks until it reaches 0xB001.
  [P][9]

### FileFormat (0xB000)

Four bytes. ExifTool's names, and the versions seen in the samples: [P][1][F]

| Bytes | Name | Seen in |
|---|---|---|
| 0 0 0 2 | JPEG | (JPEG files) |
| 1 0 0 0 | SR2 | DSC-R1 (file 3221) |
| 2 0 0 0 | ARW 1.0 | DSLR-A100 (2346) |
| 3 0 0 0 | ARW 2.0 | DSLR-A200 (2271), DSLR-A700 (2521) |
| 3 1 0 0 | ARW 2.1 | DSLR-A900 (5858) |
| 3 2 0 0 | ARW 2.2 | NEX-5 (1457), SLT-A55V (964) |
| 3 3 0 0 | ARW 2.3 | DSC-RX100 (5811), Hasselblad Lunar (4051) |
| 3 3 1 0 | ARW 2.3.1 | SLT-A99V firmware 1.02 (3751), ILCE-7S (1582), ILCE-7RM2 (1310, 1311) |
| 3 3 2 0 | ARW 2.3.2 | no sample checked |
| 3 3 3 0 | ARW 2.3.3 | ILCE-9 (1541, 1542), DSC-RX100M6 (3012, 3015) |
| 3 3 5 0 | ARW 2.3.5 | ILCE-7RM3 firmware 3.00 (3251, ARQ 3255) |
| 4 0 0 0 | ARW 4.0 | ILCE-7SM3 (5418), ILCE-1 (4466) |
| 4 0 1 0 | ARW 4.0.1 | ILCE-7M4 firmware 2.00 (6929 to 6936), ILCE-6700 (6736) |
| 5 0 0 0 | ARW 5.0 | ILCE-9M3 (7361, 7364) |
| 5 0 1 0 | ARW 5.0.1 | ILCE-1M2 (7835) |
| 6 0 0 0 | ARW 6.0 | ILCE-7M5 (8846 to 8850) |

- There is no ARW 3.x. exiv2 labels a few values differently ("SR2 1.0",
  "ARW 5.0.0"); this is cosmetic. LibRaw folds the bytes into a decimal
  number (3310 for 3 3 1 0). [P][1][8][9]
- The version follows the firmware, not only the body: the SLT-A99V with
  firmware 1.02 writes 2.3.1 where Clévy's body table says 2.3, the
  ILCE-7RM3 with firmware 3.00 writes 2.3.5 where Clévy says 2.3.3 and
  ExifTool names only the HX99, and the ILCE-7M4 with firmware 2.00 writes
  4.0.1 where ExifTool names only the ZV-E1. [P][1] [S][25] [F] Use the version
  for gross structure (for example, ARW 4.0 and later carry a full-size JPEG
  in IFD2), never to identify a body.
- An exiv2 dump sweep over raw.pixls.us reported no FileFormat or ModelID
  for the Lunar; the bytes of file 4051 have both (3 3 0 0 and 289). [S][24]
  [F]
- The SRF sample (file 1351) has no FileFormat tag. ExifTool sets the file
  type to SR2 when FileFormat is 1 0 0 0 and otherwise assumes ARW. [P][1][F]

### SonyModelID (0xB001)

A u16 that identifies the body more reliably than the Model string. IDs run
from 2 (DSC-R1) and 256 (DSLR-A100) upwards; ExifTool's table [1] (search
for `SonyModelID`) and exiv2's go up to 410 (ILCE-7RM6). Do not copy those
tables. [P][1][8]

Values read from the samples: [F]

- DSC-R1 2, DSLR-A100 256, DSLR-A900 257, DSLR-A700 258, DSLR-A200 259,
  NEX-5 278, SLT-A55V 281, Hasselblad Lunar 289 (the NEX-7's ID), SLT-A99V
  294, DSC-RX100 297, ILCE-7S 318, ILCE-7RM2 347, ILCE-9 358, ILCE-7RM3 362,
  DSC-RX100M6 366, ILCE-7SM3 383, ILCE-1 384, ILCE-7M4 388, ILCE-9M3 392,
  ILCE-6700 394, ILCE-1M2 400, ILCE-7M5 407.

Notes:

- On the DSLR-A900 and A850, APS-C mode changes the ModelID (262 and 270,
  "APS-C mode" in ExifTool). [P][1] On newer bodies it does not: the
  ILCE-7M4 APS-C files (6933, 6936) say 388 like the full-frame ones, and
  the ILCE-7S APS-C file (1582) says 318. [F]
- Some IDs cover two Models (263 is "DSLR-A380/A390"). [P][1]
- Hasselblad rebadges share Sony's IDs (Lunar = NEX-7 289, HV = A99 294,
  Stellar = RX100 297). [P][1][13][F]

## 3. Two ciphers

Sony obscures some data with two unrelated schemes. Neither has a secret:
the first has no key, and the second uses a key stored next to the data.
This page calls the first "enciphered" (ExifTool's word) and the second
"encrypted". [P][2][14]

| Scheme | Applies to | Key | Needed for |
|---|---|---|---|
| Byte substitution, b³ mod 249 | MakerNote 0x2010, 0x900B, 0x9050 and the 0x94xx tags | none | nothing raw-files plans: shutter count, aspect ratio, LensType2, MakerNote orientation |
| XOR keystream | the SR2SubIFD (every ARW 2.x to 6.0 and SR2), SRF's MakerNote IFDs and SRF pixel data | 32 bits from the file | milestone 6 white balance, black and white level on files before ARW 2.3.3; the SR2SubIFD's own white level on every file |

Both were checked on the samples:

- The substitution cipher yields shutter counts in 0x9050 that increase in
  capture order on four bodies
  ([section 4](#4-the-makernote-substitution-cipher)). [F]
- The keystream decrypts the SR2SubIFD of every sample that has one (the
  R1's SR2 and all 33 ARW and ARQ files from the A200 to the ILCE-7M5) into
  a valid IFD of 101 to 232 entries, with every entry type valid. [F]

## 4. The MakerNote substitution cipher

**The rule.** Each byte is enciphered on its own, independently of its
position. A byte b from 0 to 248 becomes b³ mod 249; a byte from 249 to 255
is left unchanged. There is no key and no state. [P][2][8][9][16]

**Why it is invertible.** 249 = 3 × 83. Cubing permutes the residues modulo
3 (by Fermat, x³ ≡ x mod 3) and modulo 83 (since 3 does not divide 82), so
by the Chinese remainder theorem it permutes 0 to 248. With 249 to 255
passed through, the map is a bijection on all 256 byte values. [P][I]

**Fixed points.** b³ ≡ b mod 249 holds when b is 0, 1 or 82 modulo 83,
which gives 0, 1, 82, 83, 84, 165, 166, 167 and 248, plus the seven
pass-through values 249 to 255: sixteen in all. This matches ExifTool's
comment. [P][2][I] Fixed points mean that a deciphered block can still show
recognisable bytes; do not take that as a sign that it is plain.

**Inverting it by computation.** Either way below needs no copied table:

- Build a 256-entry table once: for every b from 0 to 248 set
  `inverse[b³ mod 249] = b`, and set `inverse[b] = b` for b from 249 to 255.
  A `const fn` can do this at compile time; rawler does exactly that and
  exiv2 builds the table at run time. [P][8][16]
- Or raise to the inverse exponent: for c from 0 to 248, the plain byte is
  c^55 mod 249. 3 × 55 = 165 = 2 × 82 + 1, and 249 is square-free, so
  (b³)^55 ≡ b for every b. [I] (checked by computation for all 256 values)

Spot values for tests: plain 2 and 3 encipher to 8 and 27; enciphered 231,
234, 205, 138 and 112 decipher to 9, 12, 13, 15 and 16, the values exiv2's
0x9404 comment lists. [P][8][I]

**Applying it.** Decipher the whole block, then read fields from the
plaintext. Multi-byte fields in 0x9050 and the 0x94xx blocks are
little-endian in ExifTool, exiv2 and LibRaw. [P][2][8][9]

**Which blocks.** ExifTool deciphers 0x2010, 0x900B, 0x9050 and every 0x94xx
tag; its comment says encryption is certain for 0x2010, 0x9050, 0x9400,
0x9402 and 0x9403 and assumed for the rest. exiv2 deciphers 0x2010, 0x9400,
0x9402, 0x9403 and 0x9404 (not 0x9050). LibRaw deciphers fields from
0x2010, 0x9050, 0x9400 to 0x9406, 0x940C, 0x940E and 0x9416. [P][2][8][9]

### Version detection and project disagreements

Each enciphered block has several layouts (ExifTool's Tag2010a to Tag2010i,
Tag9050a to Tag9050d, Tag9400a to Tag9400c, and so on). The projects pick
the layout differently, and a wrong pick yields silently wrong values,
never an error. [P][1][2][9]

| Block | ExifTool | exiv2 | LibRaw |
|---|---|---|---|
| 0x2010 | Model-name patterns | one layout, by Model list | ModelID |
| 0x9050 | Model-name patterns; for ILCE-1M2, 7M5 and 7RM6 the 9050d layout only if the block starts with five zero bytes | not decoded | ModelID; pre-production Models force 9050a |
| 0x9400 | the **enciphered** first byte: 0x07, 0x09, 0x0A → 9400a; 0x0C → 9400b; the discrete set 0x23, 0x24, 0x26, 0x28, 0x31, 0x32, 0x33, 0x41, 0x43 → 9400c | 9400c only, for the same set plus 0x34 (DSC-RX10M5); no 9400a or 9400b | 0x23, 0x24, 0x26, 0x28, 0x31, 0x32, 0x33 → 9400c; 0x0C → 9400b; 0x0A only → 9400a; 0x07, 0x09, 0x41 and 0x43 not handled |
| 0x9404, 0x9405, 0x9406 | enciphered first byte (and the third or fourth byte) | 0x9404 the same way | 0x9404 and 0x9405 the same way |

Sources: [2][7][8][9]. The 0x9400 "version" is an enciphered value: deciphered,
its bytes do not increase. ExifTool's comment gives 58 as the plain value
of enciphered 0xD0; the formula gives 88 (58³ mod 249 is 145), so the
comment looks like a typo. [P][2][I]

Measured enciphered first bytes of 0x9400: [F]

| First byte | Bodies (files) |
|---|---|
| 0x0A | DSC-RX100 (5811), SLT-A99V (3751), Lunar (4051) |
| 0x24 | ILCE-7S (1582), ILCE-7RM2 (1310) |
| 0x26 | ILCE-9 (1541), ILCE-7RM3 (3251), DSC-RX100M6 (3012) |
| 0x31 | ILCE-7SM3 (5418), ILCE-1 (4466), ILCE-7M4 (6929) |
| 0x33 | ILCE-6700 (6736), ILCE-9M3 (7361), ILCE-1M2 (7835) |
| 0x41 | ILCE-7M5 (8846) |

0x41 occurs on a real body, so LibRaw's list, which stops at 0x33, misses
the ILCE-7M5. [F] For the newest bodies the projects also disagree on the
0x9050 layout: [P][2][9]

- ExifTool has none for the ILCE-9M3, while LibRaw uses 9050d.
- The ILX-LR1 is 9050a in ExifTool (its Model matches none of the
  exclusions) but 9050c in LibRaw.
- ExifTool picks 9050d for the ILCE-1M2, 7M5 and 7RM6 only when the
  enciphered block starts with five zero bytes (and for the ILCE-6700,
  7CM2, 7CR, ILME-FX2, ZV-E1 and ZV-E10M2 by Model alone); LibRaw uses
  9050d for the ILCE-1M2 and 7M5 always. LibRaw has no ModelID for the
  ILCE-7RM6, so it falls back to 9050a there.

### Shutter count (nice-to-have)

0x9050 holds the shutter count as a little-endian integer masked to 24 bits
by ExifTool: at byte 0x32 in 9050a, 0x3A in 9050b and 9050c, and 0x0A in
9050d, where it counts mechanical actuations only. [P][2] These offsets
rest on ExifTool alone: LibRaw does not read them, and takes a different
counter (its ImageCount3) from per-body offsets between 0x19F and 0x1CD.
[P][9] Checked: [F]

- ILCE-7M4 (6929, 6930, 6931, 6933, 6936), at 0x3A: 16904, 16905, 16906,
  16908, 16911;
- ILCE-9 (1541, 1542), at 0x3A: 942, 945;
- ILCE-7RM2 (1310, 1311), at 0x3A: 9482, 9486;
- ILCE-7M5 (8846 to 8850), at 0x0A: 662, 665, 669, 671, 673. These blocks
  start with at least eight zero bytes, as the 9050d rule expects.

The ILCE-1M2 file 7835 has a 0x9050 block that is mostly `FF` (219 of 256
bytes, `FF` at the count offsets), probably because it was shot with the
electronic shutter, so it holds no count; it does not start with five zero
bytes, so ExifTool decodes no layout for it. On
the ILCE-9M3 (7361, 7364) and ILCE-6700 (6736) the deciphered values at
0x0A, 0x32 and 0x3A are not plausible counts (34, 4096, 65536; 2, 649). [F]

### Other traps

- **Double encipherment.** ExifTool 9.04 to 9.10 enciphered these blocks
  twice when writing. ExifTool recognises the doubly enciphered 0x9400 first
  bytes 0x5E, 0xE7 and 0x04 (0x07, 0x09 and 0x0A enciphered again) and
  deciphers twice. Files edited by those versions exist. [P][2]
- **Firmware-dependent offsets.** LibRaw moves an offset inside 0x9050 by
  firmware version (parsed from the Software tag) on the ILCE-7, 7R, 6000,
  7S and 7M2; ExifTool picks ISO offsets in 0x9401 by version byte and, for
  the ILCE-7M4 and ILCE-1, by firmware. Treat every offset inside these
  blocks as fragile. [P][2][9]
- **0x2010 is gone** from the ILCE-7SM3 on (file 5418 and every later
  sample), and 0x9416 replaces 0x9405 from the same body. [P][2][F]

## 5. SR2Private and the SR2SubIFD

The decoding parameters of most Sony raws (white balance, black and white
level, a colour matrix) live in an encrypted IFD reached from IFD0.
[P][2][10][15][16]

**Path.**

1. IFD0 tag 0xC634 (DNGPrivateData, which Sony reuses) holds a 4-byte
   absolute file offset. Every ARW, SR2 and ARQ sample writes it as type
   BYTE, count 4. [F] Read the four bytes as a u32 in the file's order.
   ExifTool reads it as int32u whatever the declared type, and writes it
   back as int8u because Adobe Camera Raw 5.3 and Apple Preview 10.5.8
   misbehaved otherwise. [P][4] A parser that insists on type LONG loses
   SR2Private.
2. At that offset is the **SR2Private** IFD, an ordinary TIFF IFD with
   absolute offsets. On the A100 there is no SR2Private IFD: 0xC634 points
   straight at a Minolta MRW block ([section 10](#10-a100-sr2-and-srf)).
   [P][2][F]
3. SR2Private tags: [P][2]
   - 0x7200 SR2SubIFDOffset: absolute offset of the encrypted block.
     ExifTool's comment says it takes only the first value if there are
     several, but its code then parses each further offset as SR2SubIFD2,
     SR2SubIFD3, … inside the same decrypted buffer. Every sample has one
     value. [F]
   - 0x7201 SR2SubIFDLength: its length in bytes;
   - 0x7202: not named by ExifTool; one SHORT, 0 in every ARW sample, absent
     from the R1 [F];
   - 0x7220: not named by ExifTool; one SHORT, 1 in every sample, the R1
     included [F];
   - 0x7221 SR2SubIFDKey: 4 bytes, read as a u32;
   - 0x7240 IDC_IFD and 0x7241 IDC2_IFD: Sony Image Data Converter edits;
     0x7241 is zero in every sample (one LONG, or ten LONGs on the NEX-5,
     SLT-A55 and A900, files 1457, 964, 5858) [F];
   - 0x7242: not named by ExifTool; ten SHORTs of 0, only on those three
     files [F];
   - 0x7250 MRWInfo in ExifTool, decoded as a Minolta MRW block only when
     its first four bytes are not all zero. It is in the SR2Private IFD of
     every ARW sample from the A200 to the ILCE-7M5 (32 files). On the A200,
     A700 and A900 (2271, 2521, 5858) it is 5120 SHORTs holding an MRW block
     that starts `00 4D 52 49` (`\0MRI`); on the NEX-5 and SLT-A55 (1457,
     964) the same 5120 SHORTs are all zero; from the RX100 and A99 (5811,
     3751) on it is 18,432 SHORTs of data of unknown meaning (in file 1542,
     at 110,984, starting `5C C6 B4 C1`). [F] The A100 has no SR2Private
     IFD at all ([section 10](#10-a100-sr2-and-srf));
   - 0x7251: not named by ExifTool; one SHORT, 0 in every sample except the
     Lunar (file 4051), where it is 65535 [F].
4. Read SR2SubIFDLength bytes at SR2SubIFDOffset, decrypt them with the key
   ([section 6](#6-the-sr2-keystream)), and parse the result as a TIFF IFD
   whose first entry count is at the start of the buffer and **whose value
   offsets are still absolute file offsets**. Rebase each by a checked
   subtraction of SR2SubIFDOffset, and skip entries that point outside the
   block. rawspeed pads its buffer with `offset` bytes in front; dcraw
   re-parses the plaintext with a negative base; ExifTool sets the data
   position to the offset. [P][2][14][15]

**Measured facts.** [F]

- The key 0x7221 is type UNDEFINED, count 4. Its bytes are `11 22 33 44`
  (0x44332211 read little-endian) on **every ARW** sampled, from the A200
  (2008) to the ILCE-7M5 (2025). Only the R1's SR2 uses another key, bytes
  `63 BE 01 3E` (file 3221).
- SR2SubIFD lengths are 11,932 to 62,112 bytes, well under LibRaw's cap of
  10,240,000.
- The SR2SubIFD has no next-IFD pointer on ARW 2.3.x and later: the four
  bytes after the entries are value data (they read as 33,554,944 or
  52,429,600). On older files they are 0.
- The SR2SubIFD is still written by the newest bodies (ILCE-7SM3, ILCE-1,
  ILCE-7M4, ILCE-6700, ILCE-9M3, ILCE-1M2, ILCE-7M5).
- 0x7240 + 6 is a second route to an IFD: on ARW 4.0 and later, six zero
  bytes at the 0x7240 offset are followed by the IFD2 that holds the
  full-size JPEG; on ARW 2.3.x files 0x7240 + 6 is the raw SubIFD; on older
  bodies the offset points at zeros or `FF` bytes. A forum report of the
  same 6-byte gap called it an ExifTool bug. [F][S][21] Do not rely on it.

**Disagreement: the key's byte order.** rawspeed and rawler read 0x7221 as
little-endian whatever the file; ExifTool and dcraw use the file's order.
They differ only for a big-endian file, and every Sony sample is
little-endian. [P][2][14][15][16][F]

**What the decrypted SR2SubIFD contains.** Names from ExifTool [2]; uses
from [9][10][14][15][16]:

| Tag | Name | Count | Notes |
|---|---|---|---|
| 0x7300 | BlackLevel | 4 | R1 only; 514 511 512 513 in file 3221 [F] |
| 0x7302 | WB_GRBGLevelsAuto | 4 | R1; G, R, B, G |
| 0x7303 | WB_GRBGLevels | 4 | R1 as-shot; G, R, B, G; 256 500 385 256 in file 3221 [F] |
| 0x7310 | BlackLevel | 4 | R, G, G, B, at 14-bit scale |
| 0x7312 | WB_RGGBLevelsAuto | 4 | |
| 0x7313 | WB_RGGBLevels | 4 | as shot; R, G, G, B |
| 0x7480 to 0x7486 | WB presets | 4 | R1 (0x7480 is 519 256 371 in file 3221 [F]) |
| 0x74A0 to 0x74A3 | lens apertures and focal lengths | | R1 |
| 0x74C0 | SR2DataIFD | | sub-IFDs, each repeating 0x7313 |
| 0x7800 | ColorMatrix | 9 | "divide by 1024"; rows sum to about 1024 (file 1542) [F] |
| 0x7820 to 0x782D | WB presets | 3 | daylight, cloudy, tungsten and others |
| 0x787F | WhiteLevel | 3 or 1 | 15360 in every sample that has it; count 1 on the A200, A700 and A900, 3 from the NEX-5 on; absent on the R1 [F] |
| 0x797D, 0x7980, 0x7982 | lens correction parameters | | used by no surveyed decoder |

### Plain copies from ARW 2.3.3 on

From ARW 2.3.3 (the ILCE-9, 2017) on, the raw SubIFD also carries 0x7310
BlackLevel and 0x7313 WB_RGGBLevels **in the clear**, with exactly the same
values as the decrypted SR2SubIFD. Seen in the ILCE-9 (1541, 1542), the
DSC-RX100M6 (3012, 3015), the ILCE-7RM3 (3251) and every later sample;
absent on the ILCE-7RM2 (1310, 1311, ARW 2.3.1) and everything older. [F]
The first public report placed the start at ARW 4.0 with an ILCE-7M4 file
(ExifTool issue #195, 2023) [S][18]; the samples move it back to 2.3.3.
Whether 2.3.2 bodies (ILCE-6500, ILCE-7SM2, ILCA-99M2, DSC-RX100M5) write
them is open.

The plain copies do not include 0x787F WhiteLevel; newer raw SubIFDs carry
the DNG WhiteLevel 0xC61D instead, with a different value
([section 7](#7-raw-relevant-tags)). [F]

Decoders differ here: LibRaw handles 0x7310 and 0x7313 in any IFD, and
libopenraw reads them only from the raw SubIFD; rawspeed and rawler read
only the SR2SubIFD, and rawler fails to decode when SR2Private or its white
balance tags are missing. [P][10][15][16][17]

exiv2 does not decode SR2Private or the SR2SubIFD. [P][7][8] A DNG converted
from an ARW can carry the original SR2Private inside Adobe's
DNGPrivateData; that is out of scope for raw-files. [P][10]

## 6. The SR2 keystream

The same generator encrypts the SR2SubIFD, SRF's MakerNote IFDs and SRF
pixel data. dcraw and LibRaw call it `sony_decrypt`. ExifTool, LibRaw,
dcraw, rawspeed and rawler implement the same algorithm. [P][2][9][14][15]
[16]

Inputs: a 32-bit key K and a buffer. Only whole 4-byte words are processed;
trailing bytes are left unchanged (rawspeed rounds the length down to a
multiple of 4; ExifTool processes the integer part of length / 4). [P][2]
[15]

All arithmetic is on unsigned 32-bit values, wrapping on overflow.

1. **Seed** with a linear congruential step, four times:
   K = K × 48828125 + 1 (mod 2³²), recording the results as w0, w1, w2, w3.
   48828125 is 5¹¹ (0x02E90EDD); ExifTool computes the same product in
   16-bit halves. [P][2]
2. **Adjust w3:** w3 = (w3 shifted left by 1) OR (the top bit of
   (w0 XOR w2)).
3. **Fill** w4 to w126: w_i = ((w_(i−4) XOR w_(i−2)) shifted left by 1) OR
   ((w_(i−3) XOR w_(i−1)) shifted right by 31).
4. **Run** the lagged XOR recurrence for n = 127, 128, …:
   w_n = w_(n−127) XOR w_(n−63). The k-th keystream word (k = 0, 1, …) is
   w_(127+k).
5. **Apply:** read the k-th data word as a **big-endian** u32, XOR it with
   the k-th keystream word, and write it back big-endian. dcraw and LibRaw
   byte-swap the pad and XOR native words, rawler XORs little-endian words
   with a byte-swapped pad, and ExifTool unpacks big-endian words; all of
   these are the same operation.
6. The cipher is its own inverse.

dcraw, LibRaw and rawspeed keep a 128-word ring buffer instead of the
growing sequence in step 4; the two forms produce the same keystream
(checked by computation over random keys). [P][I] dcraw and LibRaw keep the
state between calls so that SRF pixel data can be decrypted row by row as
one continuous stream; nothing else needs that. [P][9][14]

**Checked on samples.** With key 0x44332211 the SR2SubIFD of every ARW
from the A200 (2271) to the ILCE-7M5 (8846 to 8850) decrypts to a valid
IFD; with the R1's own key, file 3221 does too. The decrypted values are
plausible: black 512 on most bodies, 800 on the 1-inch RX100 and RX100M6,
4 on the A200; white balance normalised to a green of 1024; 0x787F 15360.
[F]

**Arithmetic hazard.** LibRaw computed the seed step in signed `int`
(undefined behaviour on overflow) until 0.20.0. In Rust, use
`u32::wrapping_mul` and `wrapping_add`, as rawler does. [P][16] See
[security.md](security.md).

## 7. Raw-relevant tags

These live in the raw SubIFD (IFD0 tag 0x014A) unless noted. Most are
decoder input for milestone 6 and are documented further in
[raw-data.md](raw-data.md); the crop tags also serve milestone 3.

| Tag | Name | Type | Meaning and measured values | M |
|---|---|---|---|---|
| 0x7000 | SonyRawFileType | u16 | ExifTool: 0 uncompressed 14-bit, 1 uncompressed 12-bit, 2 compressed, 3 lossless, 4 lossless 2, 6 compressed 2. **The samples do not match** (below) | none |
| 0x7010 | SonyToneCurve | u16[4] | four knots of the cRAW decompression curve | 6 |
| 0x7031 to 0x7037 | lens corrections | i16 | vignetting, chromatic aberration, distortion: mode and parameters | none |
| 0x7038 | SonyRawImageSize | u32[2] | width, height of the usable area of a tile-padded raw | 3, 6 |
| 0x7310 | BlackLevel | u16[4] | plain copy, ARW 2.3.3 and later | 6 |
| 0x7313 | WB_RGGBLevels | i16[4] | plain copy, ARW 2.3.3 and later | 6 |
| 0x74C7 | SonyCropTopLeft | u32[2] | **(left, top)** | 3, 6 |
| 0x74C8 | SonyCropSize | u32[2] | width, height | 3, 6 |
| 0xC61D | WhiteLevel (DNG tag) | | ARW 2.3.3 and later | 6 |
| 0xC61F, 0xC620 | DefaultCropOrigin, DefaultCropSize (DNG tags) | | ARW 2.3.3 and later | 3, 6 |

Sources: [2][4][10][15][16][17][18]; measurements [F] as cited below. The
DNG BlackLevel tag (0xC61A) appears in no Sony file of the raw.pixls.us
sweep. [S][24]

**0x7000 does not mean what ExifTool says.** The ILCE-7RM2 12-bit
uncompressed file (1311) says 0 ("uncompressed 14-bit"); every lossless
file (4466, 6929, 6930, 6931, 6736, 7364, 8848; large, medium and small)
says 4, never 3; the A200 file (2271), whose data is an ARW1-coded stream,
says 3; the ILCE-7M5 compressed files (8846 to 8850 except the lossless
8848) say 6. [F] ExifTool's own note limits the tag to ARW 2.0 and later.
[P][4] Never choose a decoder from it; [raw-data.md](raw-data.md) gives the
rules that work.

**0x7010 SonyToneCurve.** Every decoder builds a 0 to 16383 curve from it;
see [raw-data.md](raw-data.md). Measured: 8000, 10400, 12900, 14100 in every
cRAW file except the NEX-5 (0, 9824, 11512, 16362; file 1457) and in every
other ARW before 4.0, uncompressed ones included; the R1's SR2 has no
0x7010 (file 3221); 0, 0, 0, 0 on ARW 4.0 and later
uncompressed (6933), lossless and ARW 6.0 files. [F] The curve is used only
for cRAW; the values in other files are not decoder input.

**0x7038 SonyRawImageSize** appears from ARW 4.0 (absent in the ILCE-9
files). Lossless raws are padded to whole 512-pixel tiles, and 0x7038 gives
the usable size, which is still larger than the default crop: [F]

| File | Stored size | 0x7038 | DefaultCrop origin, size |
|---|---|---|---|
| 6929 (ILCE-7M4, lossless) | 7168 × 5120 | 7032 × 4688 | (12, 8), 7008 × 4672 |
| 4466 (ILCE-1, lossless) | 8704 × 6144 | 8664 × 5784 | (12, 12), 8640 × 5760 |

LibRaw's per-body table gives 7028 and 8660 for those two widths. rawspeed
crops lossless images to 0x7038, while rawler takes it as the active area and
the crop from DefaultCrop. [P][12][15][16][F]

**0x74C7 is (left, top), not (top, left).** LibRaw reads (left, top);
libopenraw reads (top, left), following ExifTool's name. The samples settle
it: in the 1:1 RX100M6 file (3012) SonyCropTopLeft is [924, 12] with a
3648 × 3648 crop of a 5504-wide raw, and 924 = 12 + (5472 − 3648) / 2 is the
centred horizontal offset; the 16:9 file (3015) gives [12, 296], and
296 = 12 + (3648 − 3080) / 2 is the centred vertical offset. [P][10][17][F]

**SonyCrop and DefaultCrop usually agree, but not always.** They are equal on
the ILCE-9, 7SM3, 1, 7M4, 6700, 9M3, 1M2 and 7M5 samples. On the ILCE-7RM3
(3251, and the ARQ 3255) 0x74C7 is (0, 8) while DefaultCropOrigin is (8, 8).
On the RX100M6, SonyCrop holds the aspect-ratio crop and DefaultCrop the
3:2 sensor area. [F] Prefer DefaultCrop for the visible area
([section 9.4](#94-image-dimensions)).

**White level.** Two values coexist: [F]

- 0x787F in the encrypted SR2SubIFD is 15360 in every sample that has it;
- the plain DNG WhiteLevel 0xC61D, present from ARW 2.3.3, is 16383 (16380
  on the RX100M6, files 3012 and 3015), including the four ARW 6.0
  compressed files. It is absent in the YCbCr medium and small raw IFDs
  (6930, 6931, 7364).

A forum post calls 16384 a theoretical maximum, with practical white points
of about 15360 to 16300. [S][22]

rawspeed and rawler use 0x787F; LibRaw treats it as a "linear max" and keeps
its own maximum; libopenraw reads 0xC61D. Which is right is a decoding
question for [raw-data.md](raw-data.md). [P][10][15][16][17]

**Black level.** 0x7310 is 512 on most bodies, 800 on the 1-inch DSC-RX100
and RX100M6 (5811, 3012), and 4 on the A200 (2271). The R1 uses 0x7300
instead (514, 511, 512, 513). [F]

**14-bit scale, and the 12-bit exception.** Black and white levels are
written at 14-bit scale. ExifTool notes "divide by 4" for 0x7310 and
0x787F, and rawspeed and rawler shift them right by 2 for 12-bit data.
[P][2][15][16] The samples show that this holds only for some 12-bit
storage: [F]

- 12-bit **packed** data (DSLR-A900, file 5858): 0x7310 says 512 and the
  data minimum is 134, about 512 / 4, so divide by 4. rawspeed does the
  same for ARW1 (the A100 and A200). [P][15]
- 12-bit **uncompressed** data (ILCE-7RM2, file 1311, Compression 1) is
  stored at 14-bit scale: the low two bits are always 0, the maximum is
  16376, and the minimum is about 540. Use 0x7310 unscaled there.

So the bit depth of the stored values, not BitsPerSample alone, decides the
scaling; [raw-data.md](raw-data.md) has the full rule.

**Channel orders.** [P][9][10][14][15][F]

- 0x7310, 0x7313 and 0x7312 are R, G, G, B; 0x7302 and 0x7303 are G, R, B,
  G. Every ARW sample has 0x7313 = [R, 1024, 1024, B] (for example 2516,
  1024, 1024, 1600 in file 1542).
- The CFA pattern is RGGB (CFAPattern 0 1 1 2) on every CFA sample except
  the R1, which is GRBG (1 0 2 1). The ARQ has no CFAPattern.
- For 0x7300 LibRaw does not reorder; rawspeed and rawler place values by
  CFA position. The two agree on RGGB and diverge only on the R1. [P][I]

## 8. White balance

As-shot multipliers by generation: [P][F]

| Generation | Source | Encrypted | Measured |
|---|---|---|---|
| SRF (DSC-F828, DSC-V3) | SRF2 IFD, tags 0x00D0 to 0x00D2 (R, G, B as u32) | yes | not located (section 10) |
| SR2 (DSC-R1) | SR2SubIFD 0x7303, G R B G | yes | 256 500 385 256 (file 3221) |
| ARW 1.0 (DSLR-A100) | the `WBG` record of the MRW block reached through 0xC634: skip 4 bytes, then four u16 | no | 401, 256, 256, 574 (file 2346) |
| ARW 2.0 to 2.3.1 | SR2SubIFD 0x7313, R G G B | yes | greens 1024 in every sample |
| ARW 2.3.3 and later | plain 0x7313 in the raw SubIFD, identical to the SR2SubIFD copy | no | 2516 1024 1024 1600 (file 1542) |

Sources: [2][9][10][14][15][16][17] and the files cited.

What the decoders do, and where they disagree: [P][9][10][14][15][16][17]

- rawspeed: 0x7303 if present, else 0x7313.
- rawler: 0x7303 if present, else 0x7313; normalises by the first green and
  averages the greens.
- libopenraw: the plain 0x7313 only, so it has no white balance before ARW
  2.3.3.
- LibRaw: whichever of 0x7303 and 0x7313 it meets last, in any IFD.
- dcraw: for 0x7313, assumes G R B G order unless both middle values are
  1024, in which case R G G B. LibRaw dropped this heuristic. Every ARW
  sample has middle values of 1024, so the projects agree on all of them;
  the heuristic would matter only for a file whose greens are not 1024.
- For the A100 `WBG` record, rawspeed takes values 0, 1 and 3 as R, G and B;
  rawler reads four u16 at + 12 into the record.

The MakerNote also has 0xB021 ColorTemperature and a white balance mode and
fine-tune; these are settings, not multipliers. [P][2] They are described
in [section 9.7](#97-camera-settings); the multipliers are not needed for
milestone 3.

## 9. Viewer metadata

Everything a viewer needs next to the preview comes from IFD0, the Exif IFD
and the raw SubIFD, plus two plain MakerNote tags for identification and
lens. **No decryption or deciphering is needed** on any sample: Make, Model,
Orientation, dates, 0xB000, 0xB001 and the crop tags are plain in every
file. [F] The focus point is a plain MakerNote tag too
([section 9.6](#96-focus-point)), and so are most of the camera
settings; the shutter used and APS-C mode are only in enciphered blocks
([section 9.7](#97-camera-settings)). IFD0 has no ImageWidth or ImageLength except on the A100 (where
they do not describe IFD0's JPEG) and in SRF files (where IFD0 describes the
raw). [S][24][F]

### 9.1 Orientation

IFD0 tag 0x0112, a u16 with the Exif values 1 to 8. Treat 0 and values
above 8 as 1. [P][4] IFD1 repeats it, and the R1's raw SubIFD has its own
copy; IFD0 wins. [F] The embedded JPEGs have no APP segments at all, so
they carry no orientation of their own (see
[container-and-previews.md](container-and-previews.md)). [F]

The one rotated sample, an SLT-A99V (file 3751), has 8 in IFD0 and IFD1.
[F] The raw.pixls.us sweep shows rotated files from the DSLR-A450,
SLT-A58, DSC-HX95 and ILCE-6400A too. [S][24]

The MakerNote has its own orientation fields. Ignore all of them: [P][1][2]

- CameraOrientation in the deciphered 0x9400 (values 1, 3, 6 and 8, which
  happen to match Exif), at byte 0x28 in the 9400a layout, 0x24 in 9400b
  and 0x29 in 9400c. [P][2] The rotated A99V sample is 9400a (enciphered
  first byte 0x0A) and reads 8 at 0x28, agreeing with IFD0; the other
  9400a files (5811, 4051) read 1 there, as their IFD0 does. [F]
- Orientation2 in MoreSettings, at byte 0x28 on the A450, A500 and A550
  and 0x2E on the other bodies with that block, in **Sony's codes, where 2
  means rotate 180°**, not Exif's horizontal mirror. ExifTool notes A55
  images where it disagrees with the other orientation tags. [P][2]
- Rotation in older CameraSettings (0, 1, 2, with 1 and 2 swapped in
  ExifTool's notes), and a CameraOrientation bit field in ExtraInfo3 (the
  A450 to A580, SLT-A33, A35 and A55, and NEX-3 and NEX-5 class). The
  A230 to A390 have ExtraInfo2, which has no orientation field. [P][2]

No Sony tag overrides IFD0. [P][I]

### 9.2 Capture date and time zone

| Item | Source | Notes |
|---|---|---|
| Local capture time | Exif 0x9003 DateTimeOriginal | `YYYY:MM:DD HH:MM:SS`, present in every Sony file of the sweep [S][24] |
| Sub-seconds | Exif 0x9291 SubSecTimeOriginal | from some ARW 2.3.5 bodies on; every ARW 4.0 and later sample |
| Offset from UTC | Exif 0x9011 OffsetTimeOriginal, form `+HH:MM` | from ARW 2.3.3 on in the samples |
| Last modification | IFD0 0x0132 DateTime | repeated in IFD1 |

Sources: [4][F] and as cited.

**OffsetTime\*.** OffsetTime, OffsetTimeOriginal and OffsetTimeDigitized are
present, all three together, in the ILCE-9 (1541, 1542; firmware 1.00,
June 2017; +02:00), the RX100M6 (3012, 3015; +01:00), the ILCE-7RM3 (3251;
−04:00), the ILCE-7SM3 (5418; +08:00), the ILCE-1 (4466; +00:00) and every
later sample. They are absent from the ILCE-7RM2 (firmware 3.30, a file
dated April 2017) and the ILCE-7S (firmware 3.20, July 2017) and every older
sample. [F] So the tags arrive with the ARW 2.3.3 generation, not at a
date: an older body updated later in 2017 still lacks them. An exiv2-dump
sweep of raw.pixls.us reported them as absent on ARW 2.3.3, most ARW 2.3.5
files, the ILCE-1 and the ILCE-7SM3; the bytes of the files show that the
dumps omit them, so that sweep cannot be used for this question.
[S][24][F] Earlier public reports showed them on an ILCE-7RM3 and an
RX10 IV in 2017. [S][20][21] Treat 0x9011 as optional.

**SubSecTime\*.** Absent on the ILCE-9, RX100M6 and ILCE-7RM3 samples and
present from the ILCE-7SM3 (5418) on. [F] The sweep shows all three
SubSecTime tags on some ARW 2.3.5 bodies (ILCE-9M2, ZV-1, ILCE-7C, ZV-E10)
and on every ARW 4.0 and later file. [S][24]

**No MakerNote time zone.** ExifTool decodes no Sony time-zone or daylight
saving field; there is no equivalent of Nikon's WorldTime
(see [the NEF page](../nef/makernote-and-metadata.md)). [P][2] The MakerNote
has copies of the local time (a plain string in 0x3000 ShotInfo, enciphered
ones in 0x2010 and 0x9050); they add nothing. [P][2] An ILCE-7M3 owner
reports DateTimeOriginal one hour off for half the year with in-camera
daylight saving on; the cause is not established. [S][21]

Read DateTime from IFD0 only: editors that change it can leave the IFD1
copy behind. [S][21][F] An unset camera clock yields a blank or
space-filled string; return "none" for the date rather than an error. [I]

### 9.3 Make, model and ModelID

IFD0 tags 0x010F and 0x0110. Make bytes as measured [F]; Model padding as
listed in the raw.pixls.us sweep [S][24]:

| Body | Make bytes | Model |
|---|---|---|
| DSLR-A100 (2346) | `SONY` + 11 spaces + NUL (16 bytes) | unpadded |
| DSLR-A200, A700, A900 (2271, 2521, 5858) | `SONY ` + NUL (one trailing space) | unpadded |
| every later Sony sample | `SONY` + NUL | unpadded |
| Hasselblad Lunar (4051) | `HASSELBLAD` + NUL (11 bytes, upper case) | `Lunar` + NUL |

The sweep lists the same single trailing space for the A230, A290, A300,
A330, A350, A380, A390 and A850 [S][24]. The A100 also has an
ImageDescription of `SONY DSC` padded with spaces. [S][24]

So: cut at the first NUL, trim trailing ASCII whitespace, and compare Make
without regard to case. ExifTool does the same: it reads the string up to
the first NUL and its Make and Model conversions trim trailing whitespace;
its rule for the headerless Sony MakerNote matches an upper-case
`^HASSELBLAD` (the Lunar's Make) together with the HV, Stellar, Lusso and
Lunar models. LibRaw strips trailing spaces. [P][3][4][12][F]

**Model strings.** Product codes such as `DSLR-A700`, `SLT-A99V`, `NEX-5`,
`ILCE-7M4`, `DSC-RX100M6`, `ILME-FX3`, `ZV-E10`, not marketing names. The
prefix gives the class: DSLR-, SLT- and ILCA- are A-mount; NEX-, ILCE- and
ILME- are E-mount; DSC- and ZV-1 have fixed lenses. [P][2][9]
Pre-production firmware writes placeholder Models (`MODEL-NAME`,
`DSLR-A000…`, `SLT-A00…`, `NEX-00…`). [P][9]

**Hasselblad rebadges.** The Lunar (NEX-7), Stellar and Stellar II (RX100,
RX100 II), HV (A99) and Lusso (A7R) keep Make Hasselblad and their own Model.
LibRaw rewrites Make and Model to the Sony names by ModelID; ExifTool and
exiv2 keep the file's strings. [P][2][13] raw-files should report the
strings as stored and expose ModelID separately, so that callers can
recognise a rebadge or an A850/A900 APS-C file. [I]

### 9.4 Image dimensions

The raw SubIFD's ImageWidth and ImageLength give the stored array, which
includes masked or unused columns and, on lossless files, padding to whole
512-pixel tiles. IFD0's own size, where present, is not the sensor's.
[P][F] Sources for the visible size, in order of preference:

1. **DefaultCropOrigin and DefaultCropSize** (0xC61F, 0xC620) in the raw
   SubIFD, written from ARW 2.3.3 on (for example (12, 12) and 6000 × 4000
   on the ILCE-9, file 1542; (32, 20) and 9504 × 6336 in a 9728 × 6656
   lossless ILCE-7CR file). [F][S][19][24] On medium and small raws they are
   smaller than the stored image (6930: 5120 × 3584 stored, 4608 × 3072
   crop; 6931: 3584 × 2560 stored, 3504 × 2336 crop; the ILCE-9M3's small
   raw, 7364: 3072 × 2048 stored, 3008 × 2000 crop). [F] APS-C files have
   the same 4608 × 3072 crop inside differently sized stored images:
   4736 × 3132 on the ILCE-7M4 (6933, 6936), 4640 × 3088 on the ILCE-7M5
   (8849, 8850) and 5120 × 3584 on its lossless APS-C file (8848). [F]
2. **SonyCropTopLeft and SonyCropSize** (0x74C7, 0x74C8), as (left, top) and
   (width, height). They hold the aspect-ratio crop where one is set, and
   can disagree with DefaultCrop by a few pixels (ILCE-7RM3). [F]
3. **SonyRawImageSize** (0x7038), from ARW 4.0: the usable area of a
   tile-padded raw, still larger than the default crop. [F]
4. **A per-body trim.** dcraw and LibRaw trim by raw width (for example 24
   columns off 6048-wide bodies). Do not carry such a table for milestone 3.
   [P][12][14]

Other size tags, useful as cross-checks: [F]

- **MakerNote 0xB02B FullImageSize**, height first, is present on every ARW
  from the A200 to the ILCE-7M5 (not the A100 or R1); file 1542 gives
  [4000, 6000]. It includes the aspect-ratio crop. 0xB02C PreviewImageSize,
  also height first, equals the IFD0 JPEG's size (3012: [1080, 1088]; 3015:
  [1080, 1920]).
- **Exif PixelXDimension and PixelYDimension** give the raw IFD size on the
  A100 (where they repeat IFD0's claim), A200, A700, A900 and A55; the
  visible size including the aspect crop (equal to FullImageSize) on the
  NEX-5, RX100, A99 and every ILCE; and the preview's 640 × 424 on the R1.
  They are not reliable across generations.

**Aspect-ratio settings** do not change the raw. The 1:1 and 16:9 RX100M6
files (3012, 3015) both store 5504 × 3672 with DefaultCrop 5472 × 3648, the
same as 3:2. The aspect shows only in SonyCrop (3648 × 3648 and
5472 × 3080), FullImageSize, Exif PixelX/YDimension and the IFD0 preview
(1088 × 1080 and 1920 × 1080). [F] The sweep finds the same on the HX99,
HX95, RX0M2, ILCE-7C, ILCE-7RM3A and ZV-E10. [S][24] So a viewer that
wants the image as framed in the camera should apply SonyCrop inside
DefaultCrop when the two differ by more than a border. [I] The focus
point's frame, FocusLocation's, has that framing on these files, and is
the full-size output on the ILCE-7M4's medium and small raws
([section 9.6](#96-focus-point)). [F]

**APS-C (Super 35) mode** on full-frame bodies writes a smaller raw with the
ModelID unchanged: ILCE-7M4 4736 × 3132 (6933, 6936), ILCE-7S 2816 × 1872
(1582), ILCE-1M2 5664 × 3768 (7835). [F] The A850 and A900 instead change
the ModelID ([section 2](#sonymodelid-0xb001)). [P][1] 0x202E Quality is 3
on these APS-C Bayer files, the same value as a medium-size raw. [F] Nothing
in the samples marks APS-C mode except the smaller size; LibRaw recognises
it by raw width. [P][12][F]

### 9.5 Lens

- **Exif LensModel (0xA434)** is present from the NEX-5 and SLT-A55 (2010)
  on for interchangeable-lens bodies, and reads `----` for a manual lens
  (file 1582). It is absent on the A100 to A900, the R1, and the fixed-lens
  RX100 and RX100M6. [F]
- **MakerNote 0xB027 LensType** is the A-mount lens ID; it is 65535 for
  E-mount lenses. LibRaw reads 0xEFxx values as Canon EF adapters and
  0x49xx to 0x59xx as the Sigma MC-11. [P][1][9]
- **MakerNote 0xB02A LensSpec** holds a flags byte, the focal and aperture
  range in BCD, and a second flags byte. [P][1][9]
- E-mount lens IDs (LensType2) live only in enciphered blocks: 0x9050,
  0x940C and 0x9416. rawler tries 0x9416, then 0x9050, then 0x940C. [P][16]
- ExifTool's composite LensID combines LensType, LensType2 and LensSpec. Its
  lens table is not something to copy. [P][4]

**The samples overturn LensSpec as a fallback.** [F] Wherever 0xB02A is
not all zeros, Exif LensSpecification (0xA432) describes the same lens;
every body from the NEX-5 and SLT-A55 on writes it, the fixed-lens RX100
and RX100M6 included, except with the ILCE-7S's manual lens (1582).
LensSpec is all zeros on the A200, A700 and A900 (files 2271, 2521, 5858),
which write no Exif range, on the A-mount lenses of the A55 and A99 (964,
3751), whose Exif range is there, and with that manual lens. And on the
RX100 family LensSpec holds the **35 mm-equivalent** focal
lengths, 28 to 100 mm on the RX100 (5811) and 24 to 200 mm on the RX100M6
(3012, 3015), where Exif gives the real ones (10.4 to 37.1 mm and 9 to
72 mm). The A100 (2346) has no 0xB02A. So for milestone 3, report the
lens from the Exif tags only: LensModel when present and not `----`, and
the LensSpecification range. FNumber and FocalLength are `0/10` with the
manual lens (1582), which means unknown. [F] ([ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md))

### 9.6 Focus point

Where the camera focused, which milestone 4 reports so that a viewer can
judge sharpness at that point. raw-files decided what it reports in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md);
this section is the record behind it. Tag meanings are ExifTool's [P][1];
every value was read on the 36 files of
[test-data.md](test-data.md#files-checked) on 2026-10-19 **[F]**, and the
expected focus point of each file is in
[test-data.md](test-data.md#22-the-focus-point-of-each-file).

**Where a Sony file records focus data.** Every source found, whether
raw-files reads it or not; all are tags of the MakerNote IFD:

| Source | What it holds | Checked files | raw-files |
|---|---|---|---|
| 0x2027 FocusLocation | four SHORTs: the frame's width and height, the centre's x and y | every sample from the ILCE-7RM2 (2015) on: ILCE-7RM2, ILCE-9, ILCE-7RM3 (ARW and ARQ), DSC-RX100M6, ILCE-1, ILCE-7SM3, ILCE-6700, ILCE-7M4, ILCE-9M3, ILCE-1M2, ILCE-7M5 **[F]** | read |
| 0x204A FocusLocation2 | the same four values | the ILCE-9M3, ILCE-1M2 and ILCE-7M5, equal to FocusLocation on every one **[F]** | read when FocusLocation gives nothing, absent or of another shape, which no sample is |
| 0x2037 FocusFrameSize | six bytes: the focus area's width and height as two u16, and a third u16 of 257 or 0 **[F]**; "width and height of FocusFrame, centered on FocusLocation" [P][1] | from the ILCE-7SM3 (2020) on: ILCE-7SM3, ILCE-1, ILCE-6700, ILCE-7M4, ILCE-9M3, ILCE-1M2, ILCE-7M5 **[F]** | read |
| 0x201B FocusMode | 0 manual, 2 AF-S, 3 AF-C, 4 AF-A, 6 DMF, 7 AF-D [P][1] | the SLT-A99V, the ILCE-7S and every body with FocusLocation **[F]** | read: 0 gives no focus point |
| 0x201D FlexibleSpotPosition | the spot chosen in Flexible Spot mode, on a 640 × 480 grid; valid in that mode only [P][1] | the same bodies as FocusMode **[F]** | not read: the spot chosen, not where focus landed, which FocusLocation records on every body that has both |
| 0x201E AFPointSelected, 0x2020 AFPointsUsed | the selected point or zone by number, a mask of the points used [P][1] | the same bodies; the SLT-A99V's mask has one bit set, every other is zero **[F]** | not read: a number needs a table of the body's points |
| 0x2022 FocalPlaneAFPointsUsed | a mask of the on-sensor focus points used [P][1] | the SLT-A99V, ILCE-7S and ILCE-7RM2, all zero **[F]** | not read: a mask, needs the same table |
| 0x0010 CameraInfo | on the A-mount DSLRs and the SLT and NEX bodies, the AF point by number and a status per point, among lens data [P][1] | the SLT-A55V, NEX-5, DSLR-A200, A700 and A900 **[F]**; its values were not decoded | not read: a number only |
| 0x940E AFInfo, enciphered | the AF type, the point in focus and the AF status; ExifTool decodes it for the SLT bodies only [P][1] | the ILCE-7S, SLT-A99V, Hasselblad Lunar and every ILCE sample from the ILCE-7RM2 on; not the RX100M6 **[F]** | not read: enciphered ([section 4](#4-the-makernote-substitution-cipher)) |
| The A100's Minolta block (0xB028) | not looked at | the DSLR-A100 | not read |

What the fields hold:

- **FocusLocation is plain**, a tag of the MakerNote IFD outside every
  enciphered block, read in the file's byte order; every sample is
  little-endian. **[F]**
- **The position is the centre of the area.** ExifTool's note on
  FocusFrameSize says the area is centred on FocusLocation. [P][1] Several
  autofocus shots with a recorded area sit at exactly half the frame,
  among them the ILCE-1's `[8640, 5760, 4320, 2880]` with a 1026 × 912
  area in AF-S (file 4466), and the ILCE-6700's `[6192, 4128, 3096, 2064]`
  with 135 × 138 (file 6736). On the ILCE-7RM3 the location agrees with
  FlexibleSpotPosition (320, 173) on its 640 × 480 grid: 320 / 640 = 0.500
  = 3976 / 7952 and 173 / 480 = 0.360 = 1911 / 5304 (file 3251). **[F]**
  The ILCE-7RM2's exact half (file 1310) proves nothing here, since it is
  a manual-focus shot, which always records the centre.
- **The frame is the picture as the camera framed it**, in stored
  orientation and at the scale the file records. It is the DefaultCrop's
  6000 × 4000 on the ILCE-9 (files 1541, 1542) and 4608 × 3072 in APS-C
  mode on the ILCE-7M4 (6933, 6936). On the RX100M6's 1:1 file it is
  SonyCrop's 3648 × 3648, not the 5472 × 3648 DefaultCrop that raw-files
  reports as the image size, and the IFD0 preview is 1088 × 1080 (file
  3012); on its 16:9 file SonyCrop's 5472 × 3080 (3015). On the ILCE-7M4's
  medium and small raws it is the full-size 7008 × 4672, while their
  default crops are 4608 × 3072 and 3504 × 2336 (6930, 6931); on the
  ILCE-9M3's small raw it is 3008 × 2000, its default crop (7364).
  **[F]** So the frame is not always the output size, but it always has
  the framing of the previews ([section 9.4](#94-image-dimensions)).
- **A manual-focus shot still records a location**: the frame's exact
  centre, with FocusMode 0, on the ILCE-7RM2 (files 1310, 1311, with
  FlexibleSpotPosition (0, 0)) and the ILCE-9M3 (7361, 7364, with
  FocusFrameSize `0, 0, 0`). The ILCE-7S file is a manual-focus shot
  without FocusLocation (1582). **[F]** Nothing else in the plain tags
  says that no autofocus took place.
- **FocusFrameSize's third value** is 257 on the ILCE-7SM3, ILCE-6700,
  ILCE-7M4 and ILCE-7M5, and 0 on the ILCE-1 and ILCE-1M2 (files 4466,
  7835) and on the manual-focus ILCE-9M3 files. **[F]** Its meaning is
  unknown. Area sizes follow the body and mode: 92 × 94 on the
  ILCE-7SM3, 350 × 351 on the ILCE-7M4, 1026 × 912 on the ILCE-1,
  1386 × 998 on the ILCE-1M2. **[F]**
- **No plain tag says whether focus was confirmed.** ExifTool finds an
  AF status only in the enciphered 0x940E block, and only on SLT bodies.
  [P][1]
- **Every focus tag's value lies inside the length the MakerNote
  declares**, on every sample. Only the DSLR-A200, A700 and A900 store
  anything past it: five entries each, 0x2003, whose 256 bytes straddle
  the end, 0xB020, LensSpec (0xB02A), FullImageSize (0xB02B) and
  PreviewImageSize (0xB02C), read on the bytes of files 2271, 2521 and
  5858 on 2026-10-21. None is a focus tag, and raw-files drops them
  unread. **[F]**
- **In a camera JPEG** the frame is the JPEG's own picture. The ILCE-7M3
  JPEG, at the camera's M size, records `[3936, 2624, 1783, 1804]` in
  AF-C (FocusMode 3): an off-centre point in a frame of the JPEG's
  3936 × 2624. The ILCE-7RM2 JPEG records `[7952, 5304, 3976, 2652]` in
  AF-S, the exact centre; the DSC-RX100M4 JPEG the exact centre,
  `[5472, 3648, 2736, 1824]`, with FocusMode 0, manual focus, which gives
  no point; the ILCE-6000 JPEG has FocusMode 4 and no FocusLocation, like
  the bodies before the ILCE-7RM2; the DSLR-A100 JPEG has neither.
  **[F]** The RX100M4 adds a body to those known to write FocusLocation.
  Unlike the ILCE-7M4's medium raw, whose frame stays full size, the
  ILCE-7M3's medium JPEG records the frame at its own size; whether the
  ILCE-7M4 does the same in a JPEG is unchecked. **[F]** A JPEG keeps a
  point only when its frame has the JPEG's aspect ratio
  ([ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).

**What the samples cover.** Each kind of shot, the files that show it,
and what raw-files gives; a row without a file is an open question
([section 13](#13-open-questions)):

| Shot | What the file records | CC0 files | raw-files gives |
|---|---|---|---|
| E-mount or RX body from the ILCE-7RM2 on, autofocus | FocusLocation; the area's size from the ILCE-7SM3 on | 1541, 1542, 3251, 4466, 5418, 6736, 6929, 8846, 8847 | the point |
| Aspect ratio 1:1 or 16:9 | FocusLocation in SonyCrop's framing | RX100M6 3012, 3015 | the point; the frame differs from the image size |
| APS-C (Super 35) mode | FocusLocation in the APS-C framing | ILCE-7M4 6933, 6936, ILCE-7M5 8848 to 8850, ILCE-1M2 7835 | the point |
| Medium or small raw | FocusLocation in a full-size frame | ILCE-7M4 6930, 6931 | the point; the frame is larger than the image size |
| Pixel-shift composite (ARQ) | the FocusLocation of its source frame | ILCE-7RM3 3255 | the point |
| Manual focus | FocusMode 0, with the frame's centre where FocusLocation exists | ILCE-7RM2 1310, 1311, ILCE-9M3 7361, 7364, ILCE-7S 1582 (no FocusLocation) | none |
| SLT body | a mask of A-mount points, CameraInfo | SLT-A99V 3751, SLT-A55V 964 | none: needs a table of points |
| A-mount DSLR | CameraInfo, or the Minolta block on the A100 | DSLR-A100 2346, A200 2271, A700 2521, A900 5858 | none: needs a table of points |
| NEX or ILCE body before the ILCE-7RM2 | a point number: CameraInfo on the NEX bodies, AFPointSelected on the ILCE-7S | NEX-5 1457; ILCE-7S 1582, a manual-focus shot, which gets none in any case | none: needs a table of points |
| Hasselblad Lunar | only the enciphered block | 4051 | none |
| The RX100 (2012) | no focus tag | DSC-RX100 5811 | none |
| SR2 and SRF files | no focus tag | DSC-R1 3221, DSC-F828 1351 | none |
| An ILCA body (A-mount, from 2014) | unknown | none | unknown |
| FocusLocation2 without FocusLocation | FocusLocation2 | none | the point; unchecked |
| A rotated shot with a focus point | expected: the frame in stored orientation | none (the rotated SLT-A99V file records no FocusLocation) | the point; unchecked |
| A big-endian file | unknown: FocusFrameSize's byte order | none | read in the file's order; unchecked |
| Camera JPEG, autofocus | FocusLocation in the JPEG's framing, at its own size | the ILCE-7M3 (CC0) and ILCE-7RM2 (CC BY) JPEGs from Wikimedia Commons | the point, when its frame has the JPEG's aspect ratio |
| Camera JPEG, manual focus | the frame's centre, FocusMode 0 | the DSC-RX100M4 JPEG (CC BY) | none |
| Camera JPEG of a body without FocusLocation | no focus tag raw-files reads | the ILCE-6000 and DSLR-A100 JPEGs (CC BY) | none |

### 9.7 Camera settings

How the photographer set the camera for the shot, which a viewer shows
in its information panel beside the exposure: the white balance, the
exposure program, the metering mode, the flash, the shutter, the image
area, the raw image size and the colour space, and with them the focal
length in 35 mm terms and the software that wrote the file. The Exif
tags record most of these on every Sony body. The MakerNote adds the
white balance setting itself, its colour temperature and fine-tuning,
and the electronic front curtain setting in plain tags; the shutter the
camera used and whether it shot in APS-C mode are only in enciphered
blocks ([section 4](#4-the-makernote-substitution-cipher)).

Every value below was read on the 36 files of
[test-data.md](test-data.md#files-checked) and on the five Sony JPEGs of
the [JPEG test data](../jpeg/test-data.md#2-the-sample-files) on
2026-10-23, with a bounds-checked script written from these pages
**[F]**; [test-data.md](test-data.md#23-the-camera-settings-of-each-file)
lists which file records which value. The meaning of each code is
ExifTool's, read in its published tag documentation [P][1][28][29].
ExifTool was also run on the same files as an independent reader to
compare with. It reads the same value as the script for every tag both
read, with three exceptions: it prints FullImageSize (0xB02B) width
first, where the file stores the height first
([section 2](#2-key-tags)); it prints nothing for an ExposureMode
(0xB041) of 65535 (the A100's); and on the A100 it finds the white
balance and its fine-tuning in another IFD (below).
Where this section reports a value that ExifTool decodes from a block
the script does not read, enciphered or not, it is tagged **[S]**; none
of ExifTool's code is in raw-files.

**Where a Sony file records each setting.**

| Setting | Standard tag | MakerNote tag | Checked files |
|---|---|---|---|
| Exposure program | Exif 0x8822 ExposureProgram | 0xB041 ExposureMode, not needed | Exif on every file |
| Metering mode | Exif 0x9207 MeteringMode | 0x202C MeteringMode2, not needed | Exif on every file |
| Flash | Exif 0x9209 Flash | 0x2017 FlashAction | Exif on every file; 0x2017 on 26 files, of the SLT-A99V, the ILCE-7S and every later body |
| White balance | Exif 0xA403 WhiteBalance, auto or manual, and 0x9208 LightSource | 0x0115 WhiteBalance | Exif on every file; 0x0115 on every file but the F828's, the R1's and the A100's |
| Colour temperature | none | 0xB021 ColorTemperature | every file but the F828's and the R1's; 0 unless the white balance is custom |
| White balance fine-tuning | none | 0x0112 WhiteBalanceFineTune | every file but the F828's, the R1's and the A100's; 0 on all |
| Shutter used | none | enciphered only: 0x9050 and 0x9400 | none in plain form |
| Electronic front curtain setting | none | 0x201A ElectronicFrontCurtainShutter | 26 files, of the SLT-A99V, the ILCE-7S and every later body |
| Image area (APS-C mode) | none | enciphered only: 0x9050 and 0x9416 | none in plain form |
| Raw image size | none | 0x202E Quality, its first value | 20 files, of the ILCE-7RM3, the RX100M6 and every later body; APS-C and medium cannot be told apart |
| Colour space | Exif 0xA001 ColorSpace and the Interoperability IFD's InteropIndex | none plain on the ARW bodies | Exif on every file |
| Focal length in 35 mm terms | Exif 0xA405 FocalLengthIn35mmFilm | none | every file but the F828's and the R1's |
| Software | IFD0 0x0131 Software | none | every file but the F828's and the R1's |

**Exposure program and metering mode.** Both are Exif tags on every
checked file **[F]**, with Exif's codes [P][28]. ExposureProgram is 1
(manual) on ten files, 2 (normal program) on five, 3 (aperture
priority) on 20, 6 (action) on the A200 (file 2271), and 4 (shutter
priority) on the A100 JPEG; it is never 0. MeteringMode is 5
(multi-segment) on 29 files and the five JPEGs, 2 (centre-weighted average) on the NEX-5,
A100 and A700 (files 1457, 2346, 2521) and 3 (spot) on the RX100M6,
A99V and A900 (files 3012, 3015, 3751, 5858) **[F]**. The MakerNote's
ExposureMode (0xB041) and MeteringMode2 (0x202C) agree with them on
every file that has them: ExposureMode 15 "manual" with Exif 1, 7
"aperture-priority AE" with 3, 0 "program AE" or 6 "auto" with 2, and
65535 on the A100; MeteringMode2 256 "multi-segment" with Exif 5, 770
"spot (large)" with 3 [P][1] **[F]**. They add nothing Exif lacks.

**Flash.** Exif's Flash (its bits are described in the
[NEF notes](../nef/makernote-and-metadata.md#87-camera-settings)) is 16
on every checked Sony file: the flash did not fire, and was set not to
**[F]**. The MakerNote's FlashAction (0x2017, 0 "did not fire" [P][1])
is 0 on all 26 files and four JPEGs that have it **[F]**. No sample
shows a fired flash.

**White balance.** The photographer chooses auto white balance, a kind
of light, a colour temperature, or a custom value measured from a grey
card, and may shift the result (the fine-tuning below). Exif records
only whether it was auto (0xA403) and, in a tag of its own, the kind of
light (0x9208 LightSource) [P][28]. The MakerNote's 0x0115 WhiteBalance,
one LONG **[F]**, records the setting, with ExifTool's codes [P][1]: 0
auto, 1 colour temperature or colour filter, 16 daylight, 32 cloudy, 48
shade, 64 tungsten, 80 flash, 96 fluorescent, 112 custom, 128
underwater. The checked files hold 0 on 30 files and the ILCE-6000,
ILCE-7M3 and ILCE-7RM2 JPEGs, 48 on the ILCE-7S (file 1582) and the
RX100M4 JPEG, and 112 on the ILCE-9M3 (files 7361, 7364) **[F]**. Exif
agrees on every file: 0xA403 is 0 with 0x0115 0, and 1 with 48
(LightSource 11, "shade") and with 112 (LightSource 255, "other").

0xB054, also called WhiteBalance by ExifTool, one SHORT with codes for
auto, custom, daylight, cloudy, the kinds of fluorescent and
incandescent light, flash and underwater, and none for shade [P][1],
is on 28 files and four JPEGs: every body but the A-mount DSLRs, the
NEX-5, the SLT-A55V, the R1 and the F828.
It is 4 ("custom") on the ILCE-9M3 files, and 0 ("auto") on every other
one, the ILCE-7S's and the RX100M4 JPEG's included, whose setting
0x0115 and Exif both give as shade **[F]**. So 0xB054 does not record
the setting on those two files. The A100 writes 0x0115 and 0x0112 not
in its Sony MakerNote but in the Minolta-format IFD that 0xB028 points
to ([section 10](#dslr-a100-arw-10)), where ExifTool reads them as 0
[S]. The F828 and the R1 have no white balance tag in their MakerNote;
Exif gives auto on both **[F]**.

**Colour temperature.** 0xB021 ColorTemperature, one LONG, in kelvin
[P][1], is on every file but the F828's and the R1's. It is 5500 on the
ILCE-9M3's two files, whose setting is custom, and 0 on all the others,
under auto and under shade **[F]**: 0 means it was not recorded. What it
holds under a colour-temperature setting is on no checked file.

**White balance fine-tuning.** 0x0112 WhiteBalanceFineTune is one LONG
**[F]**, which ExifTool documents as unsigned [P][1], on every file but
the F828's, the R1's and the A100's, and 0 on all of them **[F]**. One
value can hold only one axis; which one, which sign means which colour,
and how a shift towards the negative side is written are shown by no
file ([section 13](#13-open-questions)).

**Shutter.** Which shutter took the picture: the mechanical one, the
electronic front curtain (an electronic start of the exposure and a
mechanical end), or a fully electronic, silent one. No plain tag
records it. 0x201A ElectronicFrontCurtainShutter, one LONG, 0 "off" and
1 "on" [P][1], is on 26 files, of the SLT-A99V, the ILCE-7S and every
later body, and on four JPEGs
**[F]**:

| 0x201A | Files |
|---|---|
| 1 | the ILCE-7RM2, ILCE-9, ILCE-7RM3 (ARW and ARQ), SLT-A99V, ILCE-6700, ILCE-7M4 and ILCE-7M5 (files 1310, 1311, 1541, 1542, 3251, 3255, 3751, 6736, 6929 to 6931, 6933, 6936, 8846 to 8850) and the ILCE-6000, ILCE-7M3 and ILCE-7RM2 JPEGs |
| 0 | the ILCE-7S, RX100M6, ILCE-1, ILCE-7SM3, ILCE-9M3 and ILCE-1M2 (files 1582, 3012, 3015, 4466, 5418, 7361, 7364, 7835) and the RX100M4 JPEG |

It is the menu setting, not the shutter used. ExifTool decodes from the
enciphered 0x9050 block (its Tag9050a, b and c tables, "Shutter":
three numbers, `0 0 0` for a silent, electronic release) whether the
release was electronic, on 18 files **[S]**. On four of them 0x201A
contradicts it: the ILCE-1 and ILCE-7SM3 (files 4466, 5418) have the
setting off and released electronically, and the ILCE-7RM3's
pixel-shift ARW and its ARQ (files 3251, 3255) have it on and released
electronically. The other 14 have non-zero numbers, a release with the
mechanical shutter or the front curtain, which the numbers do not tell
apart [I], so on them the shutter used is not known either. On the RX100M6 and the RX100M4 JPEG ExifTool decodes a
ShutterType of 23, "mechanical", from the enciphered 0x9400 block (its
Tag9400c table) **[S]**. On the A-mount DSLRs, the NEX-5, the SLT-A55V,
the RX100, the Lunar, the R1 and the F828, nothing records the shutter
or its setting in plain form.

**Image area (APS-C mode).** A full-frame body can record only the
centre of its sensor, the APS-C or Super 35 mm area. Nothing plain marks
it except the smaller raw size ([section 9.4](#94-image-dimensions)).
ExifTool decodes an APS-CSizeCapture flag from the enciphered 0x9050
block (Tag9050a and b) and the enciphered 0x9416 block **[S]**: "on" on
the ILCE-7M4's, ILCE-1M2's and ILCE-7M5's APS-C files (6933, 6936, 7835,
8848 to 8850) and the ILCE-7M3 JPEG, whose 3936 × 2624 picture is that
body's APS-C size and its M size alike; "off" on the other files it
decodes, but also on the ILCE-7S's APS-C file (1582), whose 2816 × 1872
raw is its APS-C crop. So the enciphered flag, as ExifTool reads it,
fails on one of the seven APS-C files.

**Raw image size.** 0x202E Quality, two SHORTs, gives the kind of raw
in its first value (1 RAW, 2 S-size RAW, 3 M-size RAW, 4 compressed RAW,
5 compressed HQ RAW) and the JPEG quality written beside it in its
second [P][1]. It is on 20 files, of the ILCE-7RM3, the RX100M6 and
every later body, and on the
ILCE-7M3 JPEG **[F]**:

| 0x202E | Files |
|---|---|
| 1, 0 | the ILCE-7RM3 (ARW and ARQ), ILCE-1, ILCE-6700, ILCE-9M3 and the ILCE-7M5's lossless APS-C file (3251, 3255, 4466, 6736, 7361, 8848) |
| 1, 2 or 1, 3 | the RX100M6, the ILCE-7M4's large raw and the ILCE-7SM3 (3012, 3015, 6929, 5418) |
| 2, 0 or 2, 2 | the small raws of the ILCE-9M3 and ILCE-7M4 (7364, 6931) |
| 3, 2 | the ILCE-7M4's medium raw (6930), and the APS-C files of the ILCE-7M4 and ILCE-1M2 (6933, 6936, 7835) |
| 4, 0 and 5, 0 | the ILCE-7M5's compressed and compressed HQ files, full frame and APS-C alike (8847, 8850 and 8846, 8849) |
| 0, 3 | the ILCE-7M3 JPEG, written without a raw |

So 3 means a medium raw on one body and an APS-C raw on two, and on the
ILCE-7M5 the first value gives the compression whatever the size: the
tag does not tell large, medium and small apart in general **[F]**. The
older 0x0102 Quality (0 RAW, 6 RAW + JPEG, 7 compressed RAW [P][1])
gives no size at all.

**Colour space.** The colour space the camera was set to for its JPEG
output. Exif ColorSpace (0xA001) is 1 (sRGB) on 33 files and the five
JPEGs, and 65535 ("uncalibrated") on the A100, R1 and A900 (files 2346,
3221, 5858) **[F]** [P][28]. The Interoperability IFD's InteropIndex
(0x0001) then names the colour space under the DCF rule, `R98` for
sRGB and `R03` for Adobe RGB [P][28]: it is `R98` on every file with
ColorSpace 1 but the F828's, which has none, `R03` on the R1 and the
A900, and empty on the A100 **[F]**. So the A100's colour space is not
named in the standard tags, although its Minolta block, as ExifTool
reads it, says Adobe RGB **[S]**; on the A900 the plain CameraSettings
block (0x0114) says Adobe RGB too, agreeing with `R03` **[S]**.

**Focal length in 35 mm terms.** Exif 0xA405 FocalLengthIn35mmFilm
[P][28] is on every file but the F828's and the R1's, and is 0, meaning
unknown, on the ILCE-7S with its manual lens (file 1582) **[F]**.

**Software and firmware.** IFD0's Software is on every file but the
F828's and the R1's, and holds the body's own name and its firmware
version **[F]**: `ILCE-7M4 v2.00`, `SLT-A55V v1.10`, `DSLR-A700 v04`,
`NEX-5 Ver.05`, and `Lunar v1.02` on the Hasselblad Lunar. No checked
file carries an editor's name. The MakerNote holds no firmware version
in a plain tag.

**MakerNote values past the declared length.** On the A200, A700 and
A900 (files 2271, 2521, 5858) the values of five MakerNote tags run past
the MakerNote's declared length: 0x2003 straddles its end, and 0xB020,
0xB02A, 0xB02B and 0xB02C lie wholly past it, ending 18 to 62 bytes
after it **[F]**. A reader that reads only values inside the MakerNote
drops them. None of them is a setting: every settings tag of those
files lies inside **[F]**.

**On the maintainer's own files.** 3,161 ARW files and 1,972 camera
JPEGs of the maintainer's own ILCE-7M2, read the same way on 2026-10-24
and given as counts only
([test-data.md](test-data.md#23-the-camera-settings-of-each-file)),
agree with the samples **[F]**: auto white balance on every one, with
0x0115, 0xB054, 0xB021 and 0x0112 all 0; 0x201A 1 on every one; no
0x202C and no 0x202E; Software `ILCE-7M2 v4.01`; on every ARW file
ColorSpace 1 with `R98`. The focal length in 35 mm terms is 0 on 1,627 of the ARW files
and 1,625 of the JPEGs, presumably shots with manual lenses, as on the
ILCE-7S sample [I]. Every MakerNote value lies inside the MakerNote.

## 10. A100, SR2 and SRF

### DSLR-A100 (ARW 1.0)

- The MakerNote has the `SONY DSC` header, 28 entries at + 12, and a 4-byte
  gap after them (file 2346). It includes 0xB023 SceneMode, 0xB024
  ZoneMatching, 0xB025 DynamicRangeOptimizer and 0xB026
  ImageStabilization, which ExifTool first documented from the A100 but
  does not restrict to it [P][1]; they are present in every later sample
  checked, from the A200 to the ILCE-7M5 [F].
- MakerNote 0xB028 points at a Minolta-format IFD (absolute offset 1612 in
  file 2346; 0 means absent). Its tags 0x0088 and 0x0089 point at the same
  JPEG as IFD0's 0x0201 and 0x0202 (25683 and 42850 in file 2346). The
  Minolta CameraSettingsA100 (0x0114) block is big-endian whatever the file.
  [P][5][F]
- IFD0 0xC634 does not lead to a TIFF IFD: in file 2346 it points at offset
  72606, where a Minolta MRW block starts `00 4D 52 49` (`\0MRI`). The block
  is 144 bytes and **little-endian**, unlike Minolta's own MRW files
  (ExifTool's MinoltaRaw.pm takes the byte order from the fourth byte, `I`
  here, `M` in MRW files); its PRD record gives a sensor of 3880 × 2608 and
  an image of 3872 × 2592, and its `WBG` record (after 4 bytes) 401, 256,
  256, 574. [P][2][26][F] dcraw and
  LibRaw try the Minolta parser on that offset first. [P][10][14]
- IFD0 0x014A is the raw data offset, not a SubIFD (98304 in file 2346),
  except after editing with Sony's Image Data Converter. ExifTool checks
  whether a valid IFD is there. [P][2][F] See
  [container-and-previews.md](container-and-previews.md).
- Known bad offsets: some firmware 1.00 thumbnails are 0x10000 too low (file
  2346 is firmware 1.04 and is correct); the IFD0 preview length "may be
  wrong" in ExifTool's words. [P][2][F]

### DSC-R1 (SR2)

- FileFormat 1 0 0 0, SonyModelID 2, a headerless MakerNote with a 4-byte
  gap; no Software tag. [F][S][24]
- The SR2SubIFD uses R1-only tags (0x7300, 0x7302, 0x7303, 0x7480 to 0x7486,
  0x74A0 to 0x74A3), its own key, and has no 0x787F WhiteLevel. [P][2][F]
- The raw strip itself is not encrypted; it is stored as big-endian 16-bit
  words in a little-endian file (see [raw-data.md](raw-data.md)). [P][F]

### DSC-F828 and DSC-V3 (SRF)

SRF needs no decryption for milestone 3: IFD0 has Make, Model and an Exif
IFD with the dates, and IFD1 holds the preview JPEG. [S][24] The key
chain matters only for white balance and the raw data (milestone 6, low
priority). At the level metadata needs:

- The MakerNote is a chain of seven IFDs, SRF0 to SRF6. SRF0 is plain, with
  one entry (tag 0x0003) and a next pointer to SRF1. [P][2][F]
- **Master key.** Read the byte b at MakerNote start + 0x8DDC; the master
  key is the big-endian u32 at MakerNote start + 0x8DDC + 4b. ExifTool's
  comment on the constant is "why?". dcraw, LibRaw and rawspeed use the
  absolute file offset 200896 instead, which equals MakerNote + 0x8DDC only
  when the MakerNote is at 164580 as on the F828; ExifTool's form survives a
  moved MakerNote. [P][2][9][14][15] In file 1351 the MakerNote is at
  164580, b is 109 and the key at 201332 is 0xABEF9C88. [F]
- **SRF1** decrypts with that key (only up to the key's position is needed)
  to a two-entry IFD: tag 0 SRF2Key and tag 1 DataKey (0x52D6BDBC and
  0x6ACE748A in file 1351), with a next pointer to SRF2. [P][2][F]
- **DataKey** is the u32 at byte 22 of the decrypted SRF1, read
  little-endian: the value field of its second entry. dcraw and rawspeed
  decrypt 40 bytes at absolute offset 164600 and take bytes 22 to 25; LibRaw
  reads tag 1 but uses the dcraw path. It decrypts the raw data at 862144
  (F828) or 787392 (V3). [P][9][14][15][F]
- **SRF2 to SRF6** hold the raw data offset and length, lens data and
  white balance (SRF2 to SRF5) and standard Exif (SRF6). ExifTool says SRF2
  to SRF5 are encrypted with SRF2Key from SRF2 to the end of the MakerNote;
  the write-ups read here disagree on whether SRF3 to SRF5 are plain. In
  file 1351 the bytes at SRF1's next pointer (164640) did not parse as an
  IFD, plain or decrypted with any of the three keys, fresh or continued;
  this is open. [P][2][14][I][F]
- LibRaw's SRF parser had an out-of-bounds read (CVE-2020-35535), and rawler
  still indexes the fixed offset 200896 without a check. [S][23][P][16] See
  [security.md](security.md).

## 11. Robustness pitfalls

The raw-files rule is that a malformed file returns an error and never
panics (see
[ADR008](../../architecture-decisions/adr008-lints-and-panics.md)). The
points specific to Sony metadata:

1. **Offsets are absolute and unchecked by the camera's own tools.**
   Bounds-check every offset and length in the MakerNote, SR2Private, the
   SR2SubIFD and the SRF chain against the file; skip bad entries rather
   than abort the IFD. ExifTool repairs moved MakerNotes with a heuristic
   ([section 1](#1-finding-the-makernote)); do not repair in a first
   version. [P][3]
2. **Rebasing the SR2SubIFD.** Its value offsets are absolute; rebase by a
   checked subtraction of SR2SubIFDOffset and skip any entry that points
   before or past the block. Reject a length below 4 after rounding down,
   and cap it (LibRaw: 10,240,000 bytes; the samples use at most 62,112).
   Decrypt only bytes actually present: LibRaw decrypted a short read until
   2024. [P][10][F]
3. **Declared types lie.** 0xC634 is BYTE[4] holding a u32; 0x7221 is
   UNDEFINED[4] holding a u32; LibRaw repairs BitsPerSample values above
   33 from the white level. Decode by count and bytes. [P][4][10][F]
4. **No next pointers.** The MakerNote and the SR2SubIFD have none on newer
   bodies; the four bytes there are value data. [F]
5. **Unsorted tags.** Do not require ascending order in any Sony IFD. [F]
6. **Zero means absent.** 0xB028 = 0, 0x7240 or 0x7241 = 0, and a 0x2001
   with offset and size 0 (ILCE-5100, 7M2, 7RM2, 7SM2 JPEGs) are "not
   present". [P][2]
7. **Version-dependent layouts.** For enciphered blocks, pick the layout only
   from data that can be checked, require every fixed offset to be inside the
   block, and treat a short or mostly-`FF` block (file 7835: 219 of 256 bytes
   `FF`, including the count offsets) as absent. A wrong layout gives silently
   wrong values. Assemble multi-byte values without shifting by the full bit
   width: LibRaw 0.18.2 fixed shifts by 32 and 40 bits when building a serial
   number from 0x9050. [P][9][F]
8. **Garbage from a wrong key.** Decryption never fails; it produces noise.
   Check plausibility (white balance greens near 1024 on ARW, black below
   white, an entry count and types that make sense) and treat failure as
   "value absent". [I][F]
9. **Edited files.** Sony's Image Data Converter adds IDC IFDs and, on the
   A100, turns 0x014A into a SubIFD; ExifTool 9.04 to 9.10 double-enciphered
   blocks; and a source comment in ExifTool's MinoltaRaw.pm says Sony's IDC
   utility "corrupts MRWInfo" when writing ARW images. [P][2][26]
10. **Caps.** LibRaw caps MakerNote blocks below 256,000 bytes, IFDs below
    1000 entries and SRF lengths below 0xFFFFF. Apply similar caps and a
    visited-offset set for chained IFDs (the SRF chain, the IFD0 chain).
    [P][9][10]
11. **Pre-production and rebadged bodies.** Placeholder Models and Hasselblad
    Makes must not break detection ([section 9.3](#93-make-model-and-modelid)).
    [P][9][F]

[security.md](security.md) has the full checklist and the history behind it.

## 12. Recommendation for raw-files

**Milestone 3 needs no decryption and no deciphering.**

raw-files decided this in [ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md), which
differs from the list below in three points. It reads no Sony MakerNote,
so the lens comes from the Exif tags alone (items 2 and 5): LensSpec adds
no range the Exif tags lack and is the 35 mm equivalent on the RX100
family ([section 9.5](#95-lens)), and ModelID waits for a caller who
needs it. And the image size is the DefaultCrop when it lies inside the
stored size, else the stored size, without SonyCrop or 0x7038 (item 3).
Text is also trimmed of spaces at both ends, not only trailing ones as
in [section 9.3](#93-make-model-and-modelid), and a value over 255 bytes
is taken for damage.

The focus point is decided in
[ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md),
under which the MakerNote is read after all, for the focus point
alone. Its IFD is found by bytes as in
[section 1](#1-finding-the-makernote), only the values inside its
declared length are read, and the tags read are FocusLocation,
FocusLocation2 when FocusLocation gives nothing (absent, or not four
SHORTs), FocusFrameSize and
FocusMode ([section 9.6](#96-focus-point)). Nothing is deciphered, and
0xB000, 0xB001 and the lens tags stay unread.

1. **Standard tags:** IFD0 Make, Model and Orientation; Exif
   DateTimeOriginal, SubSecTimeOriginal and OffsetTimeOriginal (the last two
   optional); the raw SubIFD's stored size and its DefaultCrop, SonyCrop and
   0x7038 tags. Every one is plain in every sample. [F]
2. **MakerNote:** detect the layout by bytes
   ([section 1](#1-finding-the-makernote)), then read only 0xB000
   FileFormat, 0xB001 SonyModelID and, for the lens, 0xB027 and 0xB02A.
   Return "absent" rather than an error for a MakerNote that does not parse.
   Expose ModelID alongside Make and Model. [I]
3. **Size policy:** report the stored size, and the visible size from
   DefaultCrop when present and inside the stored size; offer the SonyCrop
   rectangle as the in-camera framing when it lies inside DefaultCrop and
   differs from it; fall back to 0x7038, then to the stored size, saying so.
   Do not use Exif PixelX/YDimension. Do not ship a per-body trim table. [I]
4. **Time policy:** prefer OffsetTimeOriginal when it parses; otherwise
   report local time with no zone. There is no MakerNote fallback. [I]
5. **Lens:** Exif LensModel when present; LensSpec otherwise. [I]
6. **Tests:** cover the headerless and `SONY DSC` layouts (file 2346), a
   Hasselblad Make (4051), a padded Make (2346, 2271), a rotated file (3751),
   a file with and without OffsetTime (1542, 1311), aspect crops (3012,
   3015), APS-C (6933), a lossless tile-padded raw (6929) and the SRF (1351).
   [I]

For the camera settings of [section 9.7](#97-camera-settings),
[ADR011](../../architecture-decisions/adr011-metadata-from-standard-tags.md)
decides, in its sections on them, what this list recommends: read the
Exif tags first and the plain MakerNote tags where Exif has no value,
and decipher nothing [I]:

- the exposure program, the metering mode, the flash and the focal
  length in 35 mm terms from Exif alone: the MakerNote only repeats the
  first three, and has no 35 mm focal length for the F828 and R1, which
  lack it in Exif;
- the white balance from 0x0115, which is finer than Exif's auto or
  manual, else from Exif's WhiteBalance and LightSource; never from
  0xB054, which says auto on two files set to shade;
- the colour temperature from 0xB021 when it is above zero;
- the electronic front curtain setting from 0x201A, as a setting and
  never as the shutter used, which it contradicts on four of the 18
  files where an enciphered block shows whether the release was
  electronic;
- the colour space from Exif's ColorSpace, with an InteropIndex of
  `R03` naming Adobe RGB when ColorSpace is 65535;
- the software from IFD0.

The fine-tuning stays unread until a file shows what 0x0112's sign and
axis mean; the raw image size, because 0x202E cannot tell an APS-C raw
from a medium one; and the shutter used and APS-C mode, because only
enciphered blocks hold them. The A100's Minolta-format IFD stays unread
too, so its white balance comes from Exif alone.

**Milestone 6 needs the SR2 keystream for older files.**

1. Implement the keystream of [section 6](#6-the-sr2-keystream); it is about
   twenty lines and needs no table. Read white balance and black level from
   the plain raw-SubIFD copies when present (ARW 2.3.3 and later) and
   otherwise from the decrypted SR2SubIFD; the white level 0x787F exists
   only in the SR2SubIFD. [F][I]
2. Read 0x7010 SonyToneCurve (plain) for cRAW. [P]
3. Do not implement the substitution cipher; nothing planned needs it. [I]
4. Treat the A100's MRW block and the SRF key chain as optional, behind
   typed "unsupported" errors until there is demand. [I]

## 13. Open questions

1. **Which white level is right.** 0x787F (15360) and 0xC61D (16383) differ
   in every file from the ILCE-9 on. A sample with clipped highlights
   (any ILCE-7M4 or ILCE-9 file) would show where the data saturates.
2. **First body with OffsetTime\* and plain 0x7310/0x7313.** Both appear
   between ARW 2.3.1 (ILCE-7RM2) and 2.3.3 (ILCE-9). Check the ARW 2.3.2
   bodies: ILCE-6500, ILCE-7SM2, ILCA-99M2, DSC-RX100M5.
3. **0x74C7 on older bodies, and the ILCE-7RM3 offset.** Which bodies before
   ARW 2.3.3 write SonyCropTopLeft/Size, and whether the ILCE-7RM3's (0, 8)
   against DefaultCropOrigin (8, 8) holds on other firmware or other 2.3.5
   bodies (ILCE-7RM4, ILCE-9M2).
4. **SRF2 to SRF6.** Where SRF2 starts in file 1351 and with which key
   (byte order of SRF2Key; whether the keystream continues from SRF1);
   whether SRF3 to SRF5 are plain. This decides where SRF white balance
   lives.
5. **Hasselblad Stellar, HV and Lusso.** Whether their MakerNotes are
   headerless like the Lunar's or use `VHAB`. raw.pixls.us has none; another
   source of CC0 files is needed.
6. **0x9050 layouts of the ILCE-9M3, ILCE-6700 and ILCE-1M2.** The values at
   the documented offsets are not plausible counts in files 7361, 7364 and
   6736. Nice-to-have only.
7. **The 0x7240 + 6 link.** What Sony's software writes at 0x7240 in edited
   files, and whether a writer must keep the link to IFD2 or the raw
   SubIFD. No IDC-edited sample was available.
8. **APS-C mode marker.** Whether any tag other than the smaller raw size
   (and 0x202E = 3) marks Super 35 capture on ILCE bodies, for example in
   the enciphered blocks.
9. **SubSecTime\* writers.** Which ARW 2.3.5 bodies write it (the sweep
    shows some, the ILCE-7RM3 sample does not), and whether it depends on
    firmware.
10. **Big-endian Sony files.** No `MM` Sony raw is known; if one exists, it
    would decide the SR2SubIFDKey byte-order disagreement
    ([section 5](#5-sr2private-and-the-sr2subifd)), and the byte order of
    FocusFrameSize's values, which raw-files reads in the file's order
    ([section 9.6](#96-focus-point)).
11. **FocusFrameSize's third value.** It is 257 on most bodies and 0 on
    the ILCE-1 and ILCE-1M2; its meaning is unknown.
12. **The point numbers of the A-mount, NEX and early ILCE bodies.**
    CameraInfo (0x0010) holds them by ExifTool's tables on the A-mount
    and NEX bodies, and AFPointSelected (0x201E) on the ILCE-7S; the
    values in the SLT-A55V, NEX-5, A200, A700 and A900 files were not
    decoded, the ILCE-7S file's 0 is a manual-focus shot's, whose meaning
    (perhaps "auto") was not checked, and the A100's Minolta block was
    not looked at. Turning them into positions needs a
    table of each body's points
    ([ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md)).
13. **ILCA bodies.** Whether the ILCA A-mount bodies (the ILCA-77M2,
    ILCA-68 and ILCA-99M2, from 2014 on) write FocusLocation;
    raw.pixls.us's metadata listings name no 0x20xx tag at all.
14. **Whether focus was confirmed.** No plain tag records it; ExifTool
    decodes an AF status from the enciphered 0x940E block on SLT bodies
    only. Whether ILCE bodies record it there, and whether raw-files
    should decipher the block for it, is open.
15. **A rotated shot with a focus point**, which would show that
    FocusLocation stays in stored orientation; the one rotated sample,
    the SLT-A99V's, records none.
16. **FocusLocation2 alone.** Every sample with FocusLocation2 also has
    FocusLocation, with equal values, so the fallback is unchecked.
17. **A zero FocusFrameSize on an autofocus shot.** raw-files keeps the
    point without a size; the only samples with `0, 0, 0` are
    manual-focus shots (files 7361, 7364), which give no point anyway.
18. **The fine-tuning's axis and sign.** 0x0112 is 0 on every sample;
    which axis it records, which sign is which colour, and how a
    negative shift is stored in a tag ExifTool documents as unsigned
    would need a shot with a known shift each way.
19. **The colour temperature under a set temperature.** 0xB021 is 0
    under auto and shade and 5500 under a custom white balance; whether
    it holds the chosen value when the photographer sets one in kelvin
    is on no sample.
20. **What 0xB054 records.** It gives auto where 0x0115 and Exif give
    shade (files 1582 and the RX100M4 JPEG), and ExifTool's list for it
    has no shade; whether it is a second setting, a fallback or
    something else is open.
21. **A plain record of the shutter used or of APS-C mode.** Both are
    only in enciphered blocks on the samples ([section 9.7](#97-camera-settings)),
    and ExifTool's reading of the APS-C flag is wrong on the ILCE-7S
    (file 1582). Whether a newer body records either in plain form is
    open.
22. **The raw size on the ILCE-7M5.** Its 0x202E gives the compression,
    not the size, on every sample, all of them large or APS-C;
    whether a medium or small raw of that body says so anywhere plain
    is open.

## Sources

Retrieved October 2026.

- [1] ExifTool, Sony tags — https://exiftool.org/TagNames/Sony.html
- [2] ExifTool, `Sony.pm` (read as a reference only; GPL/Artistic) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/Sony.pm
- [3] ExifTool, `MakerNotes.pm` (read only) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/MakerNotes.pm
- [4] ExifTool, `Exif.pm` (read only) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/Exif.pm
- [5] ExifTool, `Minolta.pm` (read only) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/Minolta.pm
- [6] exiv2, MakerNote formats — https://exiv2.org/makernote.html
- [7] exiv2, `makernote_int.cpp` and `tiffimage_int.cpp` (read only) —
  https://github.com/Exiv2/exiv2/blob/main/src/makernote_int.cpp
- [8] exiv2, `sonymn_int.cpp` (read only) —
  https://github.com/Exiv2/exiv2/blob/main/src/sonymn_int.cpp
- [9] LibRaw, Sony metadata parser `sony.cpp` (read only) —
  https://github.com/LibRaw/LibRaw/blob/master/src/metadata/sony.cpp
- [10] LibRaw, `tiff.cpp` (read only) —
  https://github.com/LibRaw/LibRaw/blob/master/src/metadata/tiff.cpp
- [11] LibRaw, `makernotes.cpp` (read only) —
  https://github.com/LibRaw/LibRaw/blob/master/src/metadata/makernotes.cpp
- [12] LibRaw, `identify.cpp` (read only) —
  https://github.com/LibRaw/LibRaw/blob/master/src/metadata/identify.cpp
- [13] LibRaw, `normalize_model.cpp` (read only) —
  https://github.com/LibRaw/LibRaw/blob/master/src/metadata/normalize_model.cpp
- [14] dcraw (read only) — https://www.dechifro.org/dcraw/dcraw.c ; the
  text was read from a mirror, https://raw.githubusercontent.com/ncruces/dcraw/master/dcraw.c,
  not verified against the original
- [15] rawspeed, `ArwDecoder.cpp` (read only) —
  https://github.com/darktable-org/rawspeed/blob/develop/src/librawspeed/decoders/ArwDecoder.cpp
- [16] rawler (dnglab), ARW decoder (read only) —
  https://github.com/dnglab/dnglab/blob/main/rawler/src/decoders/arw.rs
- [17] libopenraw, `sony.rs` (read only; LGPL-3.0-or-later) —
  https://gitlab.freedesktop.org/libopenraw/libopenraw/-/blob/master/src/sony.rs
- [18] ExifTool issue #195, "Sony ARW 4.0 tags" (2023) —
  https://github.com/exiftool/exiftool/issues/195
- [19] pixls.us forum, "Default crop size" —
  https://discuss.pixls.us/t/default-crop-size/43488
- [20] zenn.dev, ILCE-7RM3 Exif listing with OffsetTime —
  https://zenn.dev/lovegraph/articles/559985737db6ea
- [21] ExifTool forum threads, read through search extracts only (the forum
  was unavailable): ILCE-7M3 daylight saving
  (https://exiftool.org/forum/index.php?msg=81508); ModifyDate in IFD1 and
  RX10 IV OffsetTimeOriginal (https://exiftool.org/forum/index.php?msg=48119,
  https://exiftool.org/forum/index.php?msg=81168); Sony software and edited
  ARWs, and the 0x7240 offset report
  (https://exiftool.org/forum/index.php?msg=51532,
  https://exiftool.org/forum/index.php?msg=60493)
- [22] pixls.us forum, "raw black/white point" —
  https://discuss.pixls.us/t/raw-black-white-point/39426
- [23] Debian security tracker, CVE-2020-35535 —
  https://security-tracker.debian.org/tracker/CVE-2020-35535
- [24] raw.pixls.us, per-file exiv2 metadata dumps of the 209 Sony-family
  files (233 dumps, counting 24 non-ARW Hasselblad files in the catalogue's
  Sony listing; a sample listing that omits OffsetTime\* in older dumps
  and prints no undecoded tags) — https://raw.pixls.us/ (each file's
  `getfile.php/<id>/exif/…` link)
- [25] Laurent Clévy, `sony_raw` notes (read only; GPL-3.0) —
  https://github.com/lclevy/sony_raw
- [26] ExifTool, `MinoltaRaw.pm` (read only) —
  https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/MinoltaRaw.pm

- [27] ExifTool, MakerNote types — https://exiftool.org/makernote_types.html
- [28] ExifTool, EXIF tags — https://exiftool.org/TagNames/EXIF.html
- [29] ExifTool, Minolta tags — https://exiftool.org/TagNames/Minolta.html

### Sample files

All CC0, from raw.pixls.us; SHA-256 checked before use. The full list of
checked files is in [test-data.md](test-data.md#files-checked).

- file 964: Sony SLT-A55, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/964/nice/Sony%20-%20SLT-A55%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `f861454a49abe0345ff69887874d00617cdf0b96af96d605051e68deda424a08`
- file 1310: Sony ILCE-7RM2, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/1310/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `a606ebf1dcc6704f03f2966326c60a87dd6bdb3704464b3897c10349a10d3a19`
- file 1311: Sony ILCE-7RM2, 12bit 12bit uncompressed (3:2) —
  https://raw.pixls.us/getfile.php/1311/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20uncompressed%20(3:2).ARW
  SHA-256 `71e0888396ef52c7e6a990b4e72ebb4d8413a1fc1c35bada449870539cef6e39`
- file 1351: Sony DSC-F828, 4:3 —
  https://raw.pixls.us/getfile.php/1351/nice/Sony%20-%20DSC-F828%20-%204:3.SRF
  SHA-256 `07739f94bcab0c7e198d418ac65172d0dce04bb84a7683ecffd0bfe4b32f6d79`
- file 1457: Sony NEX-5, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/1457/nice/Sony%20-%20NEX-5%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `3b085569ac1fec37a1289ed5ea517497692a3ea14cea41953dd6e878abf230a3`
- file 1541: Sony ILCE-9, 14bit 14bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/1541/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20compressed%20(3:2).ARW
  SHA-256 `598bbc828c279528f504c51a10418521b17a90a25b198368f65e19cfda9e3711`
- file 1542: Sony ILCE-9, 14bit 14bit uncompressed (3:2) —
  https://raw.pixls.us/getfile.php/1542/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW
  SHA-256 `22c2158c9796386dba28d7cf040a1ca70f4ec2c927ec50771b78bfb523e21c29`
- file 1582: Sony ILCE-7S, 14bit 14bit compressed (3:2), APS-C crop —
  https://raw.pixls.us/getfile.php/1582/nice/Sony%20-%20ILCE-7S%20-%2014bit%2014bit%20compressed%20(3:2).ARW
  SHA-256 `a35ebb2fbec929daa5beb20d1ce5c15a8aac7b1a7a231455387f3df8a7442e07`
- file 2271: Sony DSLR-A200, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/2271/nice/Sony%20-%20DSLR-A200%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `04eacd403d92e0c90da9d91b87615af8eda1afc963350d3ab774557c36763eef`
- file 2346: Sony DSLR-A100, compressed (3:2) —
  https://raw.pixls.us/getfile.php/2346/nice/Sony%20-%20DSLR-A100%20-%20compressed%20(3:2).ARW
  SHA-256 `4df7981ac4baec9648b4f106751ea4b09a6e27bc70a15f8ef09bec051fec362d`
- file 2521: Sony DSLR-A700, 8bit 8bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/2521/nice/Sony%20-%20DSLR-A700%20-%208bit%208bit%20compressed%20(3:2).ARW
  SHA-256 `3159e28892bf0771d01525cd4b6190f9c15bbb19fa2fab6d2515ced0594e8f40`
- file 3012: Sony DSC-RX100M6, 12bit 12bit compressed (1:1) —
  https://raw.pixls.us/getfile.php/3012/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(1:1).arw
  SHA-256 `53c5abad853f17d2c46d9f96f8ff4e0af0ea095193fecf61222a8bf9d8beb00b`
- file 3015: Sony DSC-RX100M6, 12bit 12bit compressed (16:9) —
  https://raw.pixls.us/getfile.php/3015/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(16:9).arw
  SHA-256 `fe9e4df604fefbcc0ed78fb4a1d9a58dbaa1f4011e3d119b3ee32fc583d43907`
- file 3221: Sony DSC-R1, 14bit 14bit uncompressed (3:2) —
  https://raw.pixls.us/getfile.php/3221/nice/Sony%20-%20DSC-R1%20-%2014bit%2014bit%20uncompressed%20(3:2).SR2
  SHA-256 `921c5f2513dd1671a9089e73fc778fc3573bc4cb70a3a555d0414661d0e62274`
- file 3251: Sony ILCE-7RM3, 14bit 14bit uncompressed (3:2), pixel-shift
  frame —
  https://raw.pixls.us/getfile.php/3251/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW
  SHA-256 `eaedff10f11b7bdd7908db40d8e1f541bd29a89369b897f181b87c29d549f0a4`
- file 3255: Sony ILCE-7RM3, 14bit 14bit uncompressed (3:2), ARQ —
  https://raw.pixls.us/getfile.php/3255/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARQ
  SHA-256 `67af1f9f4cff82bd182f3fc48e95d39ae7d8352595e3c72f3386c9dee114f27d`
- file 3751: Sony SLT-A99, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/3751/nice/Sony%20-%20SLT-A99%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `bc033b8b0df5b1ef603ead4b57f979b8a0a46e7a52d8b46ac6dfa22f86c243f8`
- file 4051: HASSELBLAD Lunar, 12bit (3:2) —
  https://raw.pixls.us/getfile.php/4051/nice/HASSELBLAD%20-%20Lunar%20-%2012bit%20(3:2).ARW
  SHA-256 `029b1283a3d1226e08fc1eb3b81874347f5c128e437f78f853436fd6ba15873b`
- file 4466: Sony ILCE-1, 14bit (4:3), lossless —
  https://raw.pixls.us/getfile.php/4466/nice/Sony%20-%20ILCE-1%20-%2014bit%20(4:3).ARW
  SHA-256 `4012536ee0f938d8c5c5e932bb7953a03134abbd9f3f4c664cb2526c1903d492`
- file 5418: Sony ILCE-7SM3, 14bit 14bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/5418/nice/Sony%20-%20ILCE-7SM3%20-%2014bit%2014bit%20compressed%20(3:2).ARW
  SHA-256 `17e35d98d448b44164ecf414f980e9248b5829233e83a1adbee396b6c1a1a0c6`
- file 5811: Sony DSC-RX100, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/5811/nice/Sony%20-%20DSC-RX100%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `579a485b5126a25cbd55cbd5dadfa7d09cf021c99cc7d4869f9e56e3f759390b`
- file 5858: Sony DSLR-A900, 12bit 12bit compressed (3:2) —
  https://raw.pixls.us/getfile.php/5858/nice/Sony%20-%20DSLR-A900%20-%2012bit%2012bit%20compressed%20(3:2).ARW
  SHA-256 `ac7c1532df77c321e8010aa1be60c1b9f245b1ac0db6f38fe617a73aae81af50`
- file 6736: Sony ILCE-6700, 14bit (4:3), lossless —
  https://raw.pixls.us/getfile.php/6736/nice/Sony%20-%20ILCE-6700%20-%2014bit%20(4:3).ARW
  SHA-256 `dc3e2ed22f46fcef778553767e0703bc37cfb043f107afe2d7e6e4dbca34200a`
- file 6929: Sony ILCE-7M4, 14bit (4:3), lossless L —
  https://raw.pixls.us/getfile.php/6929/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(4:3).ARW
  SHA-256 `851b43c2116c4139104a5036f83ac3b6a148789b2142214dd7192c13972b25b6`
- file 6930: Sony ILCE-7M4, 4:3, lossless M —
  https://raw.pixls.us/getfile.php/6930/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW
  SHA-256 `d453005714327addd75bcb99c1c6223173dc92f2a59542d82e6761ef0e0e7571`
- file 6931: Sony ILCE-7M4, 4:3, lossless S —
  https://raw.pixls.us/getfile.php/6931/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW
  SHA-256 `cbbd0930c7d8706dff84c68a2004454266e6fd0d8354f5f76a106b5d776e0223`
- file 6933: Sony ILCE-7M4, 14bit 14bit uncompressed (3:2), APS-C —
  https://raw.pixls.us/getfile.php/6933/nice/Sony%20-%20ILCE-7M4%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW
  SHA-256 `bc0089c821a3711ebbeec3844cde5e5c247ac7cf9ca8ad9ae03a977b232216b2`
- file 6936: Sony ILCE-7M4, 14bit (3:2), APS-C compressed —
  https://raw.pixls.us/getfile.php/6936/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(3:2).ARW
  SHA-256 `639a6d4db881f1359e3ea7e1137b314e59417d24e6d4d862b7202111a2c823b2`
- file 7361: Sony ILCE-9M3, 14bit (3:2), compressed —
  https://raw.pixls.us/getfile.php/7361/nice/Sony%20-%20ILCE-9M3%20-%2014bit%20(3:2).ARW
  SHA-256 `81b0b140f34356964fdb4adf5e202eb01d5e5e5a1b61b861954a183fc30d4bf0`
- file 7364: Sony ILCE-9M3, 3:2, lossless S —
  https://raw.pixls.us/getfile.php/7364/nice/Sony%20-%20ILCE-9M3%20-%203:2.ARW
  SHA-256 `3d9da593a027bdbf31589eb950b04be7ce3f3dc6105ca339c5b7702ffa7fefdd`
- file 7835: Sony ILCE-1M2, 14bit Lossy (3:2), APS-C —
  https://raw.pixls.us/getfile.php/7835/nice/Sony%20-%20ILCE-1M2%20-%2014bit%20Lossy%20(3:2).ARW
  SHA-256 `050c924e5f78afd8479aa442f17b8650e0a5ebfa5757e57368cb0dc865ca2417`
- file 8846: Sony ILCE-7M5, 14bit (3:2), FF compressed HQ —
  https://raw.pixls.us/getfile.php/8846/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
  SHA-256 `4e3a99cbcf2a348decaf3ca68c5de22b7a05eaa590f4e9184735c1f9c45213a7`
- file 8847: Sony ILCE-7M5, 14bit (3:2), FF compressed —
  https://raw.pixls.us/getfile.php/8847/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
  SHA-256 `46d73475dfd8f96fd7e2f4d4a721a36cbbaba3d6ea95a4851809cd9d252b2a80`
- file 8848: Sony ILCE-7M5, 14bit (4:3), APS-C lossless —
  https://raw.pixls.us/getfile.php/8848/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(4:3).ARW
  SHA-256 `51ad709cba01a5f05e3c723b779f6b5fda8fa7b378494dbb40b4d3d3aa5494a4`
- file 8849: Sony ILCE-7M5, 14bit (3:2), APS-C compressed HQ —
  https://raw.pixls.us/getfile.php/8849/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
  SHA-256 `c16cae99d26a2f8ad183d31a23823310a994aa058d55cfe56e4f82ba56865f57`
- file 8850: Sony ILCE-7M5, 14bit (3:2), APS-C compressed —
  https://raw.pixls.us/getfile.php/8850/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
  SHA-256 `8315af2a9889709f4c836dbaadc23d226847e5a0630fda212a9f896d5fe0569f`
