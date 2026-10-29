# NEF container and embedded previews

A Nikon NEF is a TIFF file with Nikon's own additions, and it carries several
JPEG images next to the raw sensor data: a full-size or near-full-size
preview, sometimes a second one of about 1620x1080 pixels, and a small
thumbnail. This page describes where those JPEGs are, how implementations
tell them apart, and the algorithm raw-files should use to return the largest.

Back to [the NEF overview](README.md).

## How to read this page

Nikon publishes no NEF specification. Everything below beyond plain TIFF and
Exif comes from other projects' documentation and source code, or from
reports by people who inspected files.

Every claim carries a confidence tag:

- **[P]** stated in a primary source: a specification, tag documentation or
  the source code of an implementation.
- **[S]** secondary: a forum post, blog, issue report or search-result
  snippet.
- **[I]** inferred, or recalled without a source; a hypothesis to test.
- **[F]** measured on a named sample file. Files are cited as "file 5146",
  the id of the file in the public raw.pixls.us collection; the files cited
  here are described under [Sample files](#sample-files), and their URLs and
  SHA-256 hashes are in [test-data.md](test-data.md#21-the-ci-set). A fact
  measured on my own files, which stay private, says so and cites them
  by counts only.

Numbers in square brackets such as [3] refer to [Sources](#sources).

**The Z 9 is the body checked against sample files throughout this
page.** The facts tagged **[F]** were measured on files 5146 to 5148, the
three CC0 Z 9 files (lossless, HE\* and HE), unless the tag names another
file, as sections 1.4 and 2 do for the D1 family and the early Coolpix
bodies, or my own files, as section 5 does for a habit of the D600 and
the D3200, a preview length that runs past the image; every other body
rests on the sources alone on this page. The previews of 32 more
files, of 25 other bodies, are checked by the tests, and
[test-data.md](test-data.md#21-the-ci-set) lists them; they are not yet
written up here. Where
a sample contradicts a source, this page follows the sample and records the
disagreement. Where a primary source's text was read only in part or in
summary, exact byte values should be confirmed on real NEFs (see
[Open questions](#open-questions)) before they are hard-coded.

This page restates algorithms in prose. No code from other projects is
reproduced, and none should be copied into raw-files from GPL, LGPL or similar
sources.

## 1. The TIFF container

### 1.1 Header and byte order

A NEF starts with the standard 8-byte TIFF header: a byte-order mark (`II`
little-endian or `MM` big-endian), the number 42, and the offset of IFD0.
[14] **[S]**

Which byte order a given generation uses is only partly established:

| Evidence | Finding | Conf. |
|---|---|---|
| Clévy's NEF notes (v0.31, 2009): header is usually `MM`, except the Coolpix E5700, which is `II` [15] | Parsed samples from the D1x, D100, D2x, D200, D3, D40, D40x, D60, D80, D90 and D700 are big-endian; only the E5700 is little-endian | **[P]** |
| PRONOM's signature record for Nikon RAW lists both `MM` and `II` signatures, including the D1 [24] | A reader has to accept both | **[P]** |
| A D40 NEF attachment on the Exiv2 tracker is `II`, but its Software tag names a Nikon editor and the MakerNote was rewritten [25] | A re-saved file, not a camera original | **[P]** that it is `II`; **[I]** that it was re-saved |
| Forum posts say files turned from `II` to `MM`, or the reverse, after editing in Nikon's own software [26] | Editing software may change the outer order | **[S]** |
| A snippet of an ExifTool forum thread says the D7100 and D5200 use Intel order for MakerNote and wrapper [23] | Probably about files from newer bodies; page not read in full | **[S]** |
| Files 5146 to 5148 (Z 9: lossless, HE\*, HE) | `II`, 42, IFD0 at offset 8; the MakerNote's embedded header is `II` too | **[F]** |
| D800, D850, Z 6, Z 7, Z f, camera-original D1 and D100, and Coolpix NRW | No evidence found | **[I]** |

The MakerNote has its own byte order, set by its embedded header, and it can
differ from the outer file's: ExifTool's comments say Nikon's editing
software can change one without the other [4]. Some structures inside the
MakerNote have a fixed order of their own, independent of both [5]. **[P]**

Consequence for raw-files: accept both orders in the outer header, and parse
the MakerNote in the order its own header declares. Do not reject a file
because the two differ. (A decoder that must sample raw data may reasonably
be stricter; one RAW decoder does reject mismatched files [10] **[P]**.
That is a decoding concern, not a preview concern.)

### 1.2 IFD map

A typical DSLR NEF has this shape. The table is based on one file described
in an issue report (a D5-like body) plus Clévy's notes. [16] [15] **[S]**

| IFD | Reached through | Typical content |
|---|---|---|
| IFD0 | TIFF header | Make, Model, Orientation, pointers; a tiny uncompressed RGB thumbnail (160x120 on D100-era bodies [15] **[P]**) |
| SubIFD, JPEG | tag 0x014A (SubIFDs) | Full-size or near-full-size JPEG |
| SubIFD, second JPEG | tag 0x014A | On roughly 2012 and later bodies, a JPEG of about 1620x1080 (section 4) |
| SubIFD, raw | tag 0x014A | The sensor data: CFA photometric, Compression 34713 or 1 |
| Exif IFD | tag 0x8769 | Capture date, and the MakerNote (tag 0x927C) |
| MakerNote PreviewIFD | MakerNote tag 0x0011 | A small JPEG (see section 3) |

SubIFD order is not fixed. The D5-like sample listed the reduced-resolution
JPEG as a second SubIFD after the large one; the 2012-and-later bodies
reportedly put the 1620x1080 JPEG after the raw SubIFD [27] **[S]**. Clévy
found, on his older samples, "usually" two SubIFDs: the JPEG first, the raw
second [15] **[P]**. The tag 0x014A value is an array of IFD offsets. Never
assume "SubIFD[0] is the big JPEG".

Clévy also reports that the D100 and D1x have a single SubIFD, holding only
the raw image, with no JPEG SubIFD [15] **[P]**.

The Z 9 has this shape, the same in files 5146 to 5148 (lossless, HE\* and
HE). Its IFD0 holds a JPEG rather than an uncompressed thumbnail, and it has
six SubIFDs, three of which hold no preview:

| IFD | Reached through | Content | Conf. |
|---|---|---|---|
| IFD0 | TIFF header | NewSubfileType 1, Compression 6, Orientation 1, Make `NIKON CORPORATION`, Model `NIKON Z 9`; a 160x120 JPEG through 0x0201/0x0202; six SubIFD offsets in 0x014A, and the Exif IFD | **[F]** |
| SubIFD0 | 0x014A, first offset | The full-size JPEG, 8256x5504: NewSubfileType 1, Compression 6, 0x0201/0x0202 | **[F]** |
| SubIFD1 | 0x014A, second offset | The raw data: NewSubfileType 0, Compression 34713, Photometric 32803, 8280x5520, 14 bits, one strip (see [raw-data.md](raw-data.md#where-the-raw-data-lives)) | **[F]** |
| SubIFD2 | 0x014A, third offset | The second JPEG, 1620x1080: NewSubfileType 1, Compression 6, 0x0201/0x0202 | **[F]** |
| SubIFD3 | 0x014A, fourth offset | An uncompressed 8-bit grey image, 384x256: NewSubfileType 1, Compression 1, Photometric 1 (BlackIsZero), one sample per pixel, one strip of 98,304 bytes; purpose unknown (section 5) | **[F]** |
| SubIFD4 | 0x014A, fifth offset | An empty image: NewSubfileType 1, Compression 34713, Photometric 1, width and height 0, BitsPerSample 12, strip offset and length 0 | **[F]** |
| SubIFD5 | 0x014A, sixth offset | An empty image, as SubIFD4 but with Compression 1 and BitsPerSample 8 | **[F]** |
| Exif IFD | tag 0x8769 | The MakerNote (tag 0x927C), in the embedded-TIFF-header layout, version 0211 (section 2) | **[F]** |
| MakerNote PreviewIFD | MakerNote tag 0x0011 | A 640x424 JPEG: Compression 6, 0x0201/0x0202 counting from the MakerNote's embedded header | **[F]** |

Every one of the four JPEGs is baseline (SOF0), 8-bit, with three
components, and is stored in one piece through 0x0201/0x0202 (section 4).
**[F]**

### 1.3 Tags that identify an IFD's role

| Tag | Name | Values worth knowing | Conf. |
|---|---|---|---|
| 0x00FE | NewSubfileType | 0 full resolution; 1 reduced resolution; 2 single page; 4 mask; 8 depth map; 16 enhanced; 0xFFFFFFFF invalid. The Nikon PreviewIFD table also lists 0x10001 ("alternate reduced") [1] | **[P]** [2] |
| 0x0103 | Compression | 1 uncompressed; 6 old-style JPEG; 7 JPEG; 34713 Nikon NEF compressed. Value 32769 is handled by dcraw as a packed layout with pad bytes [34] | **[P]** [2]; 32769 handling **[P]** [34]; which bodies write it **[I]** |
| 0x0106 | PhotometricInterpretation | 32803 colour filter array (raw); 2 RGB; 6 YCbCr | 32803 **[P]** [2]; 2 and 6 are standard TIFF **[I]** |
| 0x0111, 0x0117 | StripOffsets, StripByteCounts | A JPEG may be stored as a strip | **[P]** [3] |
| 0x0201, 0x0202 | JPEGInterchangeFormat, ...Length | Offset and length of a JPEG | **[P]** [2] |
| 0x0112 | Orientation | See section 8 | **[P]** |
| 0x828E | CFAPattern | Present on the raw IFD | **[P]** [9] |

Clévy's description of the typical SubIFDs: the JPEG one has SubfileType 1,
Compression 6 and tags 0x0201 and 0x0202; the raw one has SubfileType 0,
Compression 1 or 34713, Photometric 32803 and the CFA tags. [15] **[P]**

### 1.4 Offset bases

| Pointer | Relative to | Conf. |
|---|---|---|
| IFD0, next-IFD chain, SubIFD array, Exif IFD pointer, and 0x0201 / strip offsets inside SubIFDs | the start of the file | **[P]** [17]; files 5146 to 5148 **[F]** |
| Everything inside a MakerNote with an embedded TIFF header, including tag 0x0011 PreviewIFD and the 0x0201 value inside it | the embedded TIFF header, which is the MakerNote start plus 10 bytes | **[P]** [4] [6] [13]; files 5146 to 5148 **[F]** |
| Everything inside a headerless MakerNote or a `Nikon\0\1\0` one | the outer TIFF header | **[P]** [12]; the headerless layout on the D1H (file 5364) and both layouts in early Coolpix JPEGs, where the outer header is the Exif segment's **[F]** |

The metadata-extractor issue shows a user first assuming SubIFD offsets were
relative to the Exif block and then finding they were relative to the file.
[17] **[S]** Exiv2's loader for the Nikon preview "sometimes" adds a
MakerNote base offset, consistent with the table. [11] **[P]**

For the headerless layout, the PreviewIFD's offsets have been seen to
count from the outer header: the D1H's PreviewIFD and its JPEG are where
offsets counted from the file's own header put them (file 5364), and in
early Coolpix JPEGs the PreviewIFD is where offsets counted from the
Exif segment's TIFF header put it, inside the MakerNote, while its
320x240 JPEG lies 12,527 bytes past the MakerNote's end. **[F]** The
short-header layout has no PreviewIFD in the files that have it, JPEGs
of the Coolpix E700 to E950, whose other offsets count from the outer
header too. **[F]** The files are listed in the
[JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand),
and what they show is in
[makernote-and-metadata.md](makernote-and-metadata.md#in-a-jpeg-file).

## 2. MakerNote layouts, as far as previews need them

Nikon has three MakerNote layouts. Projects number them differently (one
project's "Type 1" is another's "Type 2"), so this page names them by their
bytes. [12] [31] **[P]**

| Layout | Starts with | IFD starts at | Offsets relative to | Bodies | Conf. |
|---|---|---|---|---|---|
| Headerless | no signature; a bare IFD | MakerNote start | outer TIFF header | D1 family, big-endian (files 201, 1785, 1786, 5364) **[F]**; in JPEGs, the Coolpix E990, E995 and E880, and the E885, E2500 and E5000 on their earlier firmware, little-endian **[F]** | **[P]** [12] [4]; **[F]** |
| Short header | `Nikon\0\1\0` (8 bytes) | MakerNote start + 8 | outer TIFF header | in JPEGs, the Coolpix E700, E800, E900 and E950, little-endian **[F]** | **[P]** [12]; **[F]** |
| Embedded TIFF header | `Nikon\0\2` + 3 more header bytes, then `II` or `MM`, 42, and an IFD offset (normally 8) | embedded header + its IFD offset, normally MakerNote start + 18 | embedded header = MakerNote start + 10 | E5400, D2H, D70, D100 and everything later; the Z 9 writes `Nikon\0`, `02 11` (version 0211), `00 00`, then an `II` header (files 5146 to 5148) **[F]** | **[P]** [12] [4] |

Detection, combining what the cited projects do: if the data starts with
`Nikon\0` and a valid byte-order mark plus magic 42 sit at offset 10, use the
embedded-header layout; if it starts with `Nikon\0` otherwise, use the short
header; if it has no signature, treat it as headerless. Test the
`Nikon\0\2` signature before looking at Make, because Nikon's editing
software can write Nikon-style MakerNotes into files from other cameras. [4]
[13] **[P]** The D200's MakerNote IFD reportedly lacks a next-IFD pointer, so
do not require one after the entries. [12] **[P]**

## 3. Where a JPEG can live

ExifTool gives each location a name, which is useful for matching reports to
this page. [3] [23] **[P]/[S]**

| ExifTool name | Location | Typical size | Conf. |
|---|---|---|---|
| JpgFromRaw | first full-size JPEG SubIFD (0x0201/0x0202, or a strip) | full sensor size on many bodies | **[P]** name; **[S]** sizes |
| OtherImage | a second JPEG SubIFD | about 1620x1080 on 2012+ bodies | **[S]** |
| PreviewImage | MakerNote PreviewIFD (tag 0x0011), own 0x0201/0x0202 | 640x424 or smaller; the smallest of the JPEGs. On the Z 9, 640x424 but not the smallest: IFD0's JPEG is smaller | **[S]**; Z 9 **[F]** |
| ThumbnailTIFF | IFD0 | 160x120 uncompressed RGB, not a JPEG; on the Z 9 a 160x120 JPEG (note 1) | **[S]** [15]; Z 9 **[F]** |

One forum snippet, for a file labelled only "D75" (the body is not
identified further), shows exactly this set: JpgFromRaw 6016x4016, OtherImage
1620x1080, PreviewImage 640x424, ThumbnailTIFF 160x120. [23] **[S]**

Notes on each location:

1. **IFD0.** On DSLR NEFs a thumbnail strip that is uncompressed RGB, never
   a JPEG; one issue's IFD0 strip pointed at zeroed data. [16] **[S]** On
   Coolpix NRW files IFD0 holds a JPEG. [14] **[S]** ExifTool switches its
   NEF/NRW file type on IFD0 compression 6, and that misfired on Z 9 files,
   which shows at least some Z 9 NEFs have a JPEG in IFD0 (section 5). [3]
   [21] **[P]/[S]** Files 5146 to 5148 confirm it: their IFD0 has
   Compression 6 and holds a 160x120 baseline JPEG through 0x0201/0x0202.
   **[F]**
2. **SubIFDs.** The large JPEGs. A JPEG here may use either the
   0x0201/0x0202 pair or a strip pair; ExifTool's naming covers both. [3]
   **[P]**
3. **PreviewIFD.** Its table holds NewSubfileType (0x00FE), Compression
   (0x0103), resolutions (0x011A, 0x011B), ResolutionUnit (0x0128),
   PreviewImageStart (0x0201), PreviewImageLength (0x0202) and
   YCbCrPositioning (0x0213). [1] **[P]** ExifTool marks the directory
   "non-essential and often corrupted". [23] **[S]** Offsets are
   MakerNote-relative (section 1.4). On the Z 9 it has Compression 6 and
   holds a 640x424 JPEG (files 5146 to 5148). **[F]**
4. **Exiv2's list.** Exiv2 checks the PreviewIFD (as `Exif.NikonPreview`) and
   about 40 generic loaders, and accepts TIFF-IFD previews only from sub-image
   groups with NewSubfileType 1. [11] **[P]**
5. **Multi-Picture Format (MPF).** No evidence that NEFs use it was found.
   **[I]**
6. **Re-saved files.** A D200 NEF rewritten by Nikon's editing software had
   a small new JPEG in the slot ExifTool expects to hold the large one,
   moving the large one to OtherImage. [23] **[S]** Slot position is never
   reliable.

### 3.1 The second ~1620x1080 JPEG

An open-source viewer's author compared 24 sample files from the public
raw.pixls.us collection. Bodies from about 2012 on (D800, Df, D750, D5, D500,
D850 and all Z bodies checked) have a full-size JPEG SubIFD first and a second
JPEG SubIFD of 1620x1080 (1632x1080 on the D800) after the raw SubIFD. Older
bodies (D3, D40, D70, D90, D7000) have a single, full-size JPEG SubIFD. The
author fully verified the Z 9, Z 8, Z 7II, Z 6II, Z 6, Z 5, Z f, Z fc, Z 50,
Z 30, D850 and D500; the Z 6III was not verified. [27] **[S]**

Files 5146 to 5148 confirm this for the Z 9: SubIFD0 holds the full-size
8256x5504 JPEG, SubIFD1 the raw data and SubIFD2 a 1620x1080 JPEG; three
more SubIFDs follow, none of them a JPEG (section 1.2). **[F]**

Consequences: a NEF can contain two comparable JPEGs in SubIFDs, and which
comes first varies. One report of a D5500 file lists a 1,737,279-byte JPEG at
0x0201/0x0202 in a SubIFD; that size is small for a full-size JPEG, which
hints it may be the 1620x1080 one, unverified. [17] **[S]/[I]**

## 4. Preview sizes by body

These are the sizes found in reports, except the Z 9's, which were measured
on files 5146 to 5148. The largest embedded JPEG is full sensor size where it
has been checked, but per-body tables are unreliable; raw-files should
measure at run time (section 7).

| Body | Findings | Conf. |
|---|---|---|
| D1, D1H, D1X, D100 | IFD0 thumbnail 160x120, uncompressed; D100 and D1x have no JPEG SubIFD; the MakerNote PreviewIFD holds a JPEG thumbnail whose size is not stated | **[P]** (Clévy) [15]; size unknown |
| D200 | PreviewIFD 570x375; JpgFromRaw 3872x2592 | **[S]** [19] |
| D800 | embedded 7360x4912 JPEG reported (forum report, via search; no URL kept); a listing shows JpgFromRaw about 4.8 MB and PreviewImage about 104 KB [23]; second JPEG 1632x1080 [27] | **[S]** |
| D810 | PreviewImage is the smallest of several JPEGs | **[S]** [23] |
| D7100 | a full-size 6000x4000 JPEG is embedded, though Nikon's Windows codec returns 1620x1080 | **[S]** [20] |
| D5 (probable) | SubIFD JPEGs of about 1.2 MB and 0.8 MB and a PreviewIFD JPEG of about 125 KB; dimensions not given; camera model inferred | **[S]** [16] |
| D5500 | 1,737,279-byte JPEG in a SubIFD, reduced resolution | **[S]** [17] |
| D850 | second JPEG 1620x1080 [27]; a full-size JPEG is also reported as embedded (uncited) | **[S]** (weak) |
| Z 9 | four JPEGs, the same in the lossless, HE\* and HE files: 160x120 in IFD0, 640x424 in the PreviewIFD, 1620x1080 in SubIFD2 and 8256x5504 in SubIFD0. All are baseline (SOF0), 8-bit, three components, Compression 6 through 0x0201/0x0202, with no APP segment, and each ends with EOI (`FF D9`) exactly at its declared length (files 5146 to 5148). This confirms the 1620x1080 second JPEG of [27] | **[F]** |
| Z 8 | an issue report mentions a matching-JPEG case of 8256x5504 | **[S]** [28] |
| Z f | an anecdotal forum post claims the preview is only 1616x1080; unconfirmed | **[S]** (anecdotal) [29] |
| D3/D300, D6, Z 6, Z 7, Z 8 full-size dimensions | would follow from sensor size (for example 8256x5504 for the Z 7, the size measured on the Z 9) but no source confirms the embedded dimensions | **[I]** |

The Z 9's four JPEGs are all baseline. **[F]** Whether any other body's
preview is progressive, or whether any body uses subsampling other than the
common ones, is not documented anywhere found. Plan to read the SOF marker of
any process, not just baseline (section 7).

## 5. Generational quirks

- **D1, D1H, D1X.** Uncompressed raw, say the sources; the D1X stores
  4024x1324 and each stored row covers two output rows (pixel aspect 0.5
  in dcraw and LibRaw), so double the height, or halve the width, to get
  square pixels and a roughly 3:2 image (see [raw-data.md](raw-data.md)).
  Their preview locations beyond Clévy's notes (section 4) are not
  sourced. [14] [8] **[S]/[P]** The files differ from the sources in two
  ways. Files 1785 and 1786 store 4028x1324, not 4024x1324; this page
  follows the files. And "uncompressed" holds for the D1H file and file
  1786 but not for the whole family: file 1785's raw is compressed
  (Compression 34713, NEFCompression 1, lossy type 1). **[F]** The D1X's
  MakerNote is headerless and big-endian, like the file (files 1785 and
  1786), as the D1's is (file 201, which is not in the CI set: its licence
  is CC BY-NC-SA 4.0). **[F]**
- **D100.** Its raw data may be uncompressed even though the TIFF
  Compression says compressed. [1] **[P]** Do not use Compression to classify
  the raw IFD; use the JPEG signature to recognise previews.
- **E2 and E3.** LibRaw adjusts its loading for models starting with "E"
  [8] **[P]**; no preview facts were found.
- **Coolpix NRW** (P6000, P7000 and others). The same TIFF family, with a
  JPEG-compressed thumbnail in IFD0 and uncompressed 12-bit raw data. [14]
  **[S]** MakerNote tag 0x0014 holds an "NRW " data block. [5] **[P]**
- **Z 9 IFD0 JPEG misclassification.** ExifTool decides NEF versus NRW by
  IFD0 compression 6; that classified Z 9 NEFs as NRW, and its author
  acknowledged that some Z files carry an IFD0 JPEG. [3] [21] **[P]/[S]**
  Files 5146 to 5148 confirm it: IFD0 has Compression 6 and holds a 160x120
  JPEG. **[F]** Do not decide the format from IFD0; walk the container
  uniformly.
- **sNEF ("Small").** The raw IFD holds 3 bytes per pixel of 12-bit YCbCr
  4:2:2 (Y0 Y1 Cb Cr in each 6 bytes) instead of a CFA. [9] [10] **[P]**
  ExifTool names NEFCompression value 8 "Small" [1] **[P]**; that sNEF files
  carry it is **[I]**, since rawspeed and rawler detect sNEF by byte count
  (3 bytes per pixel). See [raw-data.md](raw-data.md). The container is
  otherwise the same; previews are unaffected as far as is known.
- **HE and HE\* (Z 9 and later).** NEFCompression values 13 and 14 (Nikon
  MakerNote tag 0x0093) [1] **[P]**; on Z bodies a decoder reads the
  compression from MakerNote tag 0x51 [10] **[P]**, and tag 0x0093 is
  absent from files 5146 to 5148, the lossless one included **[F]**; the raw
  IFD still reports TIFF Compression 34713 on Z 9 samples [21] **[S]**, and
  does so on all three files, HE and HE\* included **[F]**, so the tag does
  not tell HE from lossless. The raw strip's first bytes do: `FF 10 FF 50`,
  the JPEG XS start-of-codestream and capabilities markers, on HE\* and HE
  (files 5147 and 5148), and no marker on lossless (file 5146). **[F]** The
  codec comes from intoPIX [32] **[S]**. HE changes little in the
  container: the IFD layout, the SubIFD order, the tags of every IFD of the
  tree and of the PreviewIFD, and the four preview sizes are the same in
  the lossless, HE\* and HE files; the raw strip's content and length
  differ; and the MakerNote IFD differs by one tag, 0x0096, present in the
  lossless file only (files 5146 to 5148). **[F]**
- **Z 9 SubIFDs that hold no preview.** After the two JPEG SubIFDs and the
  raw one, the Z 9 has three more (section 1.2). **[F]** SubIFD3 is an
  uncompressed 8-bit grey image of 384x256 (Photometric 1, one strip of
  exactly 384 × 256 = 98,304 bytes, which begins with zero bytes); SubIFD4
  and SubIFD5 are empty images whose width, height, strip offset and strip
  length are all 0. What any of the three is for is unknown. A finder that
  follows section 7.1 skips all three: the empty ones have no bytes, and the
  grey image is not a JPEG (Compression 1, and its first bytes are not an
  SOI).
- **D600 and D3200: a preview length that runs past the image.** On my
  own files, 1,374 from a D600 and 3,568 from a D3200, the largest
  JPEG lies in SubIFD0 and ends with `FF D9` exactly at its declared
  length, with no padding, on all but 2 and 7. On those nine the
  declared length is 2 more than a multiple of 512, and 513 bytes
  follow the end marker: not `FF` padding, not zeros, and not the bytes
  that follow the preview in the file. On every one of the nine the
  image itself, up to and including its end marker, is 1 byte longer
  than a multiple of 512. **[F]** It reads as a slip of the firmware
  when the image ends one byte into a 512-byte block: the count of
  blocks plus 2 written as the length, with the write buffer's leftovers
  after the image; a chance of about one in 512 per file, near the nine
  in 4,942 seen. **[I]** The JPEG is whole, and a decoder stops at the
  marker, so returning the declared bytes (section 7.1, step 7) is
  right. A check that expects the end marker at the declared end, or
  only padding after it, must allow for this shape on a NEF: a declared
  length 2 more than a multiple of 512, with up to 513 bytes of other
  data after the image's own end marker.
- **Capture NX re-saves.** Nikon's editing software can rewrite files: a
  changed byte order, a different slot for the large JPEG (section 3), and
  Nikon-style MakerNotes copied from another camera. [23] [4] **[S]/[P]**
- **Coolscan "NEF" (film scanner).** Community reports describe a TIFF with a
  16-bit (one post says 14-bit) RGB bitmap and instructions for Nikon's scan
  software; Nikon staff are reported to have said these are not RAW files.
  [30] **[S]** ExifTool defines MakerNote-table tag 0x0E10 **NikonScanIFD**
  (a sub-IFD, flagged "non-essential and often corrupted") with tags such as
  FilmType (0x02), MultiSample (0x40), BitDepth (0x41), MasterGain (0x50),
  DigitalICE (0x100) and ROCInfo (0x110). [5] [1] **[P]** Which IFD hosts
  it in a scan file is unconfirmed. **[I]** There is no CFA, so a CFA-based
  decoder can reject such a file on Photometric != 32803 **[I]**. For
  previews, expect no JPEG and return an error.
- **NikonNEFInfo, tag 0xC7D5.** ExifTool's EXIF table lists 0xC7D5
  NikonNEFInfo and links it to its "Nikon NEFInfo" table, which decodes
  DistortionInfo (0x0005) and VignetteInfo (0x0006). [2] [33] **[P]** It is
  reported in SubIFD1 of Z 6/Z 7 NEFs and some Coolpix NRW files. **[S]** It
  is not relevant to previews; which IFD hosts it across bodies remains open
  (see [Open questions](#open-questions)).

## 6. How other implementations choose the largest preview

| Project | Method | Conf. |
|---|---|---|
| Exiv2 | collects every candidate; for TIFF-IFD candidates reads width and height from tags (rejecting zero); for JPEG blobs, opens them and reads the dimensions from the JPEG; checks offset + size against the file and a non-zero size; **does not check the SOI marker**; sorts by pixel area ascending, last is largest | **[P]** [11] |
| rawler | among IFDs with NewSubfileType 1 and a JPEG length tag, picks the largest byte length; does not consult the PreviewIFD | **[P]** [10] |
| LibRaw | records thumbnails whose width and height may be zero; "largest" is judged by an area-based heuristic; checks offsets against the file size when a check is enabled; skips oversized entries; its documentation warns the largest thumbnail may have zero recorded size | **[P]** [7]; **[S]** docs |
| fiv (viewer) | picks the smallest by pixel count, skipping zero dimensions (the opposite goal) | **[S]** [18] |
| rawspeed / rawler (raw IFD) | the IFD with CFAPattern; rawler falls back to the first with NewSubfileType 0; LibRaw takes the largest by a size-times-depth measure | **[P]** [9] [10] [7] |

## 7. Recommendation for raw-files

Rank by what is measured in the JPEG itself, never by slot, tag dimensions
(often absent or zero) or tag-claimed type. raw-files implements these
steps in its preview finder, shared with ARW, except where it follows the
ARW steps instead: it skips every IFD that is raw by role, not only the
CFA and Compression 34713 IFDs of step 2, and takes a preview stored as
exactly one strip, not as several contiguous ones (step 2); a preview
must start `FF D8 FF` (step 4); and a candidate without a SOF is dropped
rather than ranked last (step 6)
([the ARW open questions](../arw/open-questions.md#5-questions-settled-by-a-decision-or-by-the-code)).
Its tests run it on files 5146 to 5148, which give the four JPEGs of
section 4 in size order. **[F]**
They also run it on the 32 files of 25 other bodies listed in
[test-data.md](test-data.md#21-the-ci-set), which give the previews listed
there. **[F]**

The steps below restate the common approach in original pseudocode terms.

### 7.1 Algorithm

1. **Collect candidate IFDs.** Start at IFD0. Add every IFD in its
   next-IFD chain, every offset in each 0x014A array, and from the Exif IFD
   the MakerNote (0x927C), then the PreviewIFD (0x0011) inside it. Keep a
   set of visited offsets so loops end, limit nesting to about 5 levels, and
   cap the total IFD count and each IFD's entry count (LibRaw caps SubIFDs
   at 1000 and MakerNote entries near 1000 [7] [6] **[P]**).
2. **Locate the bytes.** For each IFD take offset and length from
   0x0201 and 0x0202 if both exist. Otherwise, if 0x0111 and 0x0117 describe
   a single strip (or contiguous strips) and Compression is 6 or 7 and the
   photometric is not 32803, use that. Resolve the offset against the right
   base (section 1.4): the file start, or MakerNote start + 10 for the
   PreviewIFD. Skip IFDs that look like the raw (CFA photometric,
   Compression 34713).
3. **Bounds-check.** Compute the end with checked addition; reject a zero
   length, a start beyond the file, or an end beyond the file. If the end is
   only slightly beyond the file and the bytes begin with a valid SOI,
   decide a truncation policy once (reject, or clamp and flag); do not mix
   policies. **[I]**
4. **Check SOI.** The first two bytes must be `FF D8`. Optionally require
   `FF` at the third byte. Exiv2 skips this check [11]; raw-files should not,
   because an IFD0 strip pointing at zeros or non-JPEG data has been
   reported [16].
5. **Parse to the first SOF.** Walk marker segments with length checks and
   a bounded loop; stop at the first start-of-frame marker and read height
   and width from it. Do not decode. The SOF codes are `C0` to `C3`, `C5` to
   `C7`, `C9` to `CB` and `CD` to `CF`; the codes `C4` (define Huffman
   tables), `C8` (reserved JPG extension) and `CC` (arithmetic-coding
   conditioning) are **not** start-of-frame markers. Handle standalone
   markers without a length (`01`, `D0` to `D7`, `D8`, `D9`), and stop at
   SOS (`DA`) without finding a SOF. The listed set is standard JPEG, not
   NEF-specific. **[I]** A lossless-process SOF (`C3`, `C7`, `CB`, `CF`)
   marks raw data, not a preview: Sony ARW 4.0 and later store their raw
   tiles as lossless JPEGs that begin with `FF D8` (see
   [the ARW page](../arw/container-and-previews.md)). A preview finder shared
   between formats should reject them.
6. **Rank.** Largest pixel area (width times height from the SOF) wins; break
   ties by the larger byte length. If a candidate has no SOF, rank it after
   all candidates that have one, by byte length. Byte length alone is wrong
   when a low-resolution, high-quality JPEG outweighs a downsized one; the
   SOF area is cheap and reliable.
7. **Return the bytes unmodified**, as a slice of the file (or an owned copy,
   as the API settles). If no candidate survives, return an error (for
   example for a Coolscan file), never a panic.

### 7.2 Why this shape

- Ranking by measured area handles the slot-order variation, the 2012+
  second JPEG, and the re-saved files in one rule.
- The MakerNote PreviewIFD is a candidate like any other, since on the oldest
  bodies it may be the only JPEG; on newer bodies it will simply rank low
  (on the Z 9, third of four, above IFD0's 160x120 JPEG **[F]**).
- The next-IFD chain and the 0x014A array both have to be walked, because a
  file may have been rewritten by different software.

## 8. Orientation

Return the JPEG bytes as they are and expose the orientation separately.

- Nikon appears to write the image data in landscape orientation with one
  Orientation value in IFD0 meant for all images; extracted previews need the
  rotation applied by the consumer. [23] **[S]**
- SubIFDs may carry their own Orientation tag, but one NEF has 6 in IFD0 and
  1 in the SubIFDs, so SubIFD values should not override IFD0. [23] **[S]**
- ExifTool's notes and a user report say the embedded JPEGs carry no
  metadata of their own, at least on the D200. [19] **[S]** The Z 9's carry
  none either (next item); whether other later bodies' JPEGs have their own
  APP1 Exif is unverified. **[I]**
- Files 5146 to 5148 (Z 9) have Orientation 1 in IFD0, and none of their
  four JPEGs has an APP segment of any kind, so none carries an Orientation
  of its own. **[F]**
- LibRaw's maintainer says that for most cameras the extracted JPEG is
  already rotated or carries a correct Orientation; that is a general
  statement, not about Nikon. [22] **[S]**

So the library should read IFD0 tag 0x0112 (default 1 when absent or
invalid) and leave the decision to the caller.

## 9. Pitfalls

- Slot order differs between bodies and after re-saves; a "first JPEG
  SubIFD" rule fails. [23] [27] **[S]**
- IFD0's strip can point at non-JPEG data. [16] **[S]**
- Dimensions in tags are often missing or zero. [7] [11] **[P]**
- PreviewIFD offsets are MakerNote-relative; applying a file-start base
  returns garbage that may still pass a naive length check. [4] [11] **[P]**
- The PreviewIFD is documented as often corrupted, so validate with SOI.
  [23] **[S]**
- A tag-declared length is authoritative, but it can run past the image.
  On the Z 9 each of the four JPEGs ends with `FF D9` exactly at its
  declared length, with no padding and no truncation (files 5146 to
  5148) **[F]**; the D600 and the D3200 sometimes declare a length 513
  bytes past the end marker, with leftover bytes after it (section 5)
  **[F]**. Return the declared bytes: a decoder stops at the marker.
- Byte order differs between the outer file and the MakerNote; parse each in
  its own order. [4] **[P]** On the Z 9 both are `II` (files 5146 to 5148)
  **[F]**, so a file from it does not exercise this rule.
- A MakerNote header can be copied from another make; check
  the `Nikon\0\2` signature before Make. [4] **[P]**
- Do not classify NEF versus NRW by IFD0 compression. [3] [21] The Z 9's
  IFD0 has Compression 6 (files 5146 to 5148). **[F]**
- A second ~1620x1080 JPEG can be the one listed first in a SubIFD array.
  [17] **[S]/[I]**
- Cycles and absurd counts: use a visited set and caps (section 7.1).

## Open questions

Each needs sample files from the public raw.pixls.us collection, inspected
with a dump tool and byte-by-byte reading. The 32 files of 25 other bodies
in [test-data.md](test-data.md#21-the-ci-set) answer item 12 and part of
items 1 to 4 and 6 to 9; [open-questions.md](open-questions.md) records
what they show.

1. Preview dimensions, count and order for the D3/D300, D850, D5/D6, Z 6,
   Z 7, Z 8 and Z f, and whether the Z f really embeds only about
   1616x1080. Answered for the Z 9 (section 4, files 5146 to 5148).
2. Outer TIFF byte order per generation, Z bodies other than the Z 9 and
   camera-original D1 and D100 in particular. The Z 9 is `II`, MakerNote
   included (files 5146 to 5148).
3. Where the D1, D1H, D1X and E-series bodies keep their JPEG (the
   PreviewIFD, and its size). Answered for the D1H: its only JPEG, of
   570x375, is in the PreviewIFD of its headerless MakerNote (file 5364).
   Also for the D1X: the MakerNote's 570x375 is its only JPEG (files 1785
   and 1786). And for the D1 (file 201, not in the CI set): it has no
   PreviewIFD and no JPEG at all, only IFD0's 160x120 uncompressed strip,
   so a D1 file gives no preview.
4. Whether any body embeds a progressive or non-baseline JPEG, or unusual
   chroma subsampling. The Z 9's four JPEGs are baseline; their subsampling
   was not read.
5. Answered: HE and HE\* files have the same IFD layout, SubIFD order,
   tags in the tree's IFDs and the PreviewIFD, and preview slots as
   lossless ones; the MakerNote IFD differs by one tag, 0x0096, present in
   the lossless file only (section 5, files 5146 to 5148).
6. Whether the PreviewIFD offset base for headerless and short-header
   MakerNotes is the outer TIFF header, as inferred. Answered for the
   headerless layout: the D1H's PreviewIFD and its JPEG lie where offsets
   counted from the file's own header put them (file 5364), and so do
   the PreviewIFDs of early Coolpix JPEGs, counted from the Exif
   segment's TIFF header, though their JPEGs lie past the MakerNote's
   end (section 1.4). The short-header MakerNotes of early Coolpix JPEGs
   count their other offsets from that header but hold no PreviewIFD,
   so where theirs would count from cannot be seen; raw-files does not
   read that layout.
7. Whether any SubIFD JPEG uses several strips. None does on the Z 9: every
   JPEG is in one piece, through 0x0201/0x0202.
8. How common truncated JPEGs or padding after EOI are in real files. The
   Z 9 has neither: each JPEG ends with EOI exactly at its declared length.
   Of 4,942 files from my D600 and D3200, none is truncated and nine
   declare a length 513 bytes past EOI, with leftover bytes rather
   than padding after it (section 5).
9. Whether a SubIFD or PreviewIFD JPEG ever has its own Exif APP1 with an
   Orientation. Not on the Z 9, whose JPEGs have no APP segment at all.
10. Nikon tag 0xC7D5 (NikonNEFInfo): which IFD hosts it across bodies. Its
    contents are documented and it does not affect preview location.
11. Which IFD hosts NikonScanIFD (0x0E10) in a Coolscan file, and whether
    such files can ever contain a JPEG.
12. Whether the Z 6III follows the 1620x1080 second-JPEG pattern.
13. What the Z 9's 384x256 uncompressed grey SubIFD and its two empty
    SubIFDs are for (section 5), and whether other Z bodies have them.

## Sources

All retrieved October 2026. Some pages were read through search-result
snippets only; those are marked.

[1] ExifTool, Nikon tags — https://exiftool.org/TagNames/Nikon.html
[2] ExifTool, EXIF tags — https://exiftool.org/TagNames/EXIF.html
[3] ExifTool, Exif.pm source — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Exif.pm
[4] ExifTool, MakerNotes.pm source — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/MakerNotes.pm
[5] ExifTool, Nikon.pm source — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Nikon.pm
[6] LibRaw, nikon.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/nikon.cpp
[7] LibRaw, tiff.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/tiff.cpp
[8] LibRaw, identify.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/identify.cpp
[9] rawspeed, NefDecoder.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decoders/NefDecoder.cpp
[10] rawler (dnglab), nef.rs — https://raw.githubusercontent.com/dnglab/dnglab/main/rawler/src/decoders/nef.rs
[11] Exiv2, preview.cpp — https://raw.githubusercontent.com/Exiv2/exiv2/main/src/preview.cpp
[12] Exiv2, MakerNote notes — https://exiv2.org/makernote.html
[13] Exiv2, makernote_int.cpp — https://raw.githubusercontent.com/Exiv2/exiv2/main/src/makernote_int.cpp
[14] libopenraw, NEF format page — https://libopenraw.pages.freedesktop.org/formats/nef
[15] Laurent Clévy, NEF notes v0.31 (22 May 2009) — http://lclevy.free.fr/nef/
[16] metadata-extractor issue 692 — https://github.com/drewnoakes/metadata-extractor/issues/692
[17] metadata-extractor issue 262 — https://github.com/drewnoakes/metadata-extractor/issues/262
[18] fiv commit on NEF thumbnails (search snippet) — https://git.janouch.name/p/fiv/commit/2e8bbf0e43a49abcaa7ac55958b644be2f5e1af1
[19] Jeffrey Friedl, blog post on D200 embedded JPEGs — https://regex.info/blog/2006-12-08/303
[20] Photools community forum, D7100 codec thread — https://www.photools.com/community/index.php?topic=938.0
[21] pixls.us discussion of Z 9 images (also the Z 9 NRW misclassification) — https://discuss.pixls.us/t/if-you-want-to-play-with-z-9-images-in-darktable/28318
[22] LibRaw forum, thumbnail orientation — https://www.libraw.org/node/2483
[23] ExifTool forum threads, read as search snippets only — https://exiftool.org/forum/index.php?msg=81235 , https://exiftool.org/forum/index.php?msg=37571 , https://exiftool.org/forum/index.php?msg=76409 , https://exiftool.org/forum/index.php?msg=23787
[24] PRONOM fmt/202, Nikon RAW — https://pronom.nationalarchives.gov.uk/fmt/202.xml
[25] Exiv2 tracker, attachment 233 (D40 NEF) — https://dev.exiv2.org/attachments/233
[26] pixls.us, "NEFs has unknown format" — https://discuss.pixls.us/t/nefs-has-unknown-format/18318/41
[27] riffle pull request 564, per-body structure from 24 sample files — https://github.com/minodisk/riffle/pull/564
[28] lightcraft issue 211, Z 8 JPEG report — https://github.com/storytold/lightcraft/issues/211
[29] DPReview forum, Z f preview (search snippet only) — https://www.dpreview.com/forums/threads/nikon-zf-only-jpeg-preview-available.4788876/
[30] Capture One support, scanner NEF files — https://support.captureone.com/hc/fr/community/posts/360012272718-scanner-NEF-and-DNG
[31] ozhiker, Nikon MakerNote layouts — https://www.ozhiker.com/electronics/pjmt/jpeg_info/nikon_mn.html
[32] intoPIX, TicoRAW on the Z 9 — https://www.intopix.com/blogs/post/nab-22-intopix-show-tico-raw-enabling-8kp60-on-the-z9-and-tico-xs-under-1-gbps
[33] ExifTool, Nikon tags (NEFInfo table) — https://exiftool.org/TagNames/Nikon.html#NEFInfo
[34] dcraw 9.28, dcraw.c (`apply_tiff`, TIFF Compression 32769) — https://dechifro.org/dcraw/dcraw.c

### Sample files

Files 5146, 5147 and 5148: the three CC0 Nikon Z 9 files of raw.pixls.us,
one per compression mode (lossless, High Efficiency\* and High Efficiency).
Their SHA-256 hashes and URLs are in
[test-data.md](test-data.md#21-the-ci-set). They were checked on 13 October
2026 by a structural walk of the IFD0 chain, the SubIFDs, the Exif IFD and
the MakerNote's PreviewIFD, reading each JPEG's marker segments to its frame
header and its last two bytes.

The early Coolpix JPEGs from Wikimedia Commons that sections 1.4 and 2
cite were checked by hand on 23 October 2026, by a walk of their Exif
segment, MakerNote and PreviewIFD written for the purpose; they are not
samples, and the
[JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand)
lists them with their licences and SHA-256 hashes.
