# NEF raw sensor data

How a Nikon NEF stores its sensor data: where the strip is, the compressed
(Huffman plus curve) scheme used by most bodies, the packed and unpacked
layouts, the small and reduced-size raws, and the HE/HE\* codec that raw-files
cannot decode. It ends with the checks a decoder of untrusted input needs and
a recommendation for decoding the raw sensor data.

Back to [the NEF overview](README.md).

## How to read this page

Nikon publishes no NEF specification. Everything below comes from the
source code of existing decoders, from ExifTool's tag documentation, from
community write-ups, and from Nikon's camera manuals.

Claims carry the confidence tags of the NEF notes
([README](README.md#confidence-tags)). Files are cited as "file 5146",
the id of the file in the public raw.pixls.us collection. The files cited
here are described under [Sample files](#sample-files), and their URLs
and SHA-256 hashes are in [test-data.md](test-data.md#21-the-ci-set).

Numbers in square brackets such as [3] refer to [Sources](#sources).

**Offsets, formulas and constants taken from implementation source code must
be checked against that source and against sample files before they are
hard-coded.** Only the facts tagged **[F]** have been checked against real
NEFs: the Z 9's raw IFD, the first bytes of its strip and the absence of
tag 0x0093, on files 5146 to 5148, and the presence of 0x0093 and the
absence of 0x0051 on the Z fc and the Z 30, on files 8559 and 5812, and
byte 10 of 0x0051 on the three Z 9 files. No strip was decoded, and no
other MakerNote value was read from them.

**License hygiene.** The decoders read for this page are under dcraw's own
license, LGPL-2.1 or CDDL-1.0 (LibRaw), and LGPL-2.1 (rawspeed, rawler,
rawloader) [43]; see [licenses-and-references.md](licenses-and-references.md).
None of their code may be copied or transliterated into raw-files. This page
states what the files hold and what each value means, in prose, tables and
formulas. It gives no pseudocode and does not describe how a program is
written; a program's behavior appears only where readers are compared. It
gives the six Huffman tables as counts per code length, never their symbol
lists, and reproduces no curve data. A decoder does need the symbol
lists; where raw-files takes them from is an open decision (see
[open-questions.md](open-questions.md)).

## Where the raw data lives

- **The raw IFD.** A NEF is a TIFF file
  ([container-and-previews.md](container-and-previews.md)). The raw image is
  in the IFD whose PhotometricInterpretation is 32803 (CFA) or that carries
  a CFAPattern tag; it is usually a SubIFD of IFD0. [11][14] **[S]** rawspeed
  and rawler look for the CFAPattern tag, and rawler also accepts the
  full-resolution subfile when none carries it. [11][14] **[S]** On the Z 9
  it is SubIFD1, the second of six SubIFDs: NewSubfileType 0,
  Compression 34713, PhotometricInterpretation 32803, 8280 × 5520,
  BitsPerSample 14, SamplesPerPixel 1 (files 5146 to 5148). **[F]**
- **The strip.** StripOffsets and StripByteCounts of that IFD give the data.
  Most bodies write one strip; rawler notes that the D500 writes several.
  [14] **[S]** The Z 9 writes one strip, of
  49,679,511 bytes for lossless (file 5146), 28,565,504 for HE\* (file 5147)
  and 17,139,200 for HE (file 5148). **[F]**
- **TIFF Compression (0x0103)** of the raw IFD takes three values: [1][14][17]
  - **34713**, Nikon compressed: the Huffman scheme below, although some
    packed layouts carry this value too (see
    [Telling the layouts apart](#telling-the-layouts-apart)). **[S]** The
    Z 9 writes it for lossless, HE and HE\* alike (files 5146 to 5148), so
    the value does not tell HE from lossless. **[F]**
  - **1**, uncompressed. **[S]**
  - **32769**, a packed layout with one pad byte after every ten pixels:
    dcraw handles the value that way, generically and without naming a
    camera. [1] **[S]** Which bodies write it is not established; it is
    probably old ones. **[I]**
- **The compression parameters** are in MakerNote tag **0x0096**
  (NEFLinearizationTable in ExifTool) [15], with **0x008C** as a fallback.
  When a file has both, dcraw and LibRaw use the one that comes later in
  the MakerNote, and rawspeed and rawler use 0x0096. [1][6][11][14]
  **[S]** Offsets inside the MakerNote are relative to its embedded TIFF
  header, not to the file
  ([makernote-and-metadata.md](makernote-and-metadata.md)). On the Z 9,
  0x0096 is present in the lossless file 5146 only; the HE and HE\* files
  5147 and 5148 carry 0x008C and not 0x0096, and all three carry 0x0051,
  whose byte 10 reads 3, 14 and 13 on them, lossless, HE\* and HE
  ([makernote-and-metadata.md](makernote-and-metadata.md#2-key-tags)).
  **[F]**
- **Byte order.** The 16-bit values in 0x0096 are in the MakerNote's byte
  order, which can differ from the file's. [2][10] **[S]** Older NEFs are
  big-endian; Nikon began writing little-endian MakerNotes with the D7100
  and D5200 (2013), and files re-saved by Nikon's software can keep some
  big-endian structures. [30] **[S]** The Z 9 is little-endian (`II`) in
  the file and in the MakerNote (files 5146 to 5148). **[F]** rawler
  rejects a file whose TIFF and MakerNote byte orders differ [14] **[S]**;
  LibRaw has a fix for NEFs damaged by Nikon Transfer [9] **[P]**.

## NEFCompression

MakerNote tag 0x0093 (NEFCompression) names the encoding. On the Z 9 and
later bodies 0x0093 is absent and the same value is an int16u at byte 10 of
tag 0x0051; ExifTool, LibRaw and rawler all read it there. [6][14][15][16]
**[S]** On the Z 9, 0x0093 is absent from files 5146 to 5148, the lossless
one included, as it is on most Z bodies checked; but the Z fc and the
Z 30 write it, with the value 3, and no 0x0051 (files 8559 and 5812), so
the compression of a Z body has to be looked for in both tags **[F]**.
Byte 10 of 0x0051 reads 3 on the lossless file 5146, 14 on the HE\* file
5147 and 13 on the HE file 5148, as a u16 in the MakerNote's order
**[F]**. The sources disagree on the byte order: LibRaw reads the field in
the MakerNote's byte order, and only rawler forces little-endian; the
three files are little-endian, so they do not tell the two apart. **[S]**

| Value | Name (ExifTool) | Layout | Bodies seen | Conf. |
|---|---|---|---|---|
| 1 | Lossy (type 1) | Huffman, full stored curve | Older DSLRs: D100, D200, D2X, D40, D80, D60 at 12-bit [22] | **[S]** |
| 2 | Uncompressed | packed or unpacked; ExifTool notes the D100 writes this even when the TIFF tag says compressed | D100 and others [16] | **[S]** |
| 3 | Lossless | Huffman, lossless tables, identity curve | D3/D300 generation onward (Clévy: D300, D700, D3X), all Z bodies [22] | **[S]** for the Clévy bodies, debut **[I]** |
| 4 | Lossy (type 2) | Huffman, sparse curve, optional split | D300, D90, D5000 per Clévy; D3, D700 and later bodies [22] | **[S]** |
| 5 | Striped packed 12 bits | several strips of 12-bit packed data | not established | **[S]** label, **[I]** layout |
| 6 | Uncompressed (reduced to 12 bit) | not established | not established | **[S]** label |
| 7 | Unpacked 12 bits (padded to 16) | 12 bits in 16-bit words | Coolpix P330/P340/P7700/P7800/A1000 | **[S]** label, **[I]** bodies |
| 8 | Small | sNEF YCbCr | D4S, D810 "RAW S" | **[I]** |
| 9 | Packed 12 bits (2 pixels in 3 bytes) | 12-bit packed, little-endian | Coolpix; possibly Z 12-bit uncompressed | **[S]** label, **[I]** bodies |
| 10 | Packed 14 bits (4 pixels in 7 bytes) | 14-bit packed, rows padded | D6 uncompressed [16] **[S]**; Z bodies **[I]** | **[S]**/**[I]** |
| 13 | High Efficiency | JPEG XS based | Z 9 (2022 firmware), Z 8, Z f, Z 6III, Z5 II | **[S]**; Z 9 strip **[F]** (below) |
| 14 | High Efficiency\* | JPEG XS based | same | **[S]**; Z 9 strip **[F]** (below) |

For the Z 9, files 5148 (HE) and 5147 (HE\*) confirm rows 13 and 14 only
in that their raw strips start with the JPEG XS markers `FF 10 FF 50`
**[F]**; the NEFCompression value itself was not read.

Values 0, 11 and 12 are not defined. [15] **[S]** ExifTool credits values 5
to 9 to Iliah Borg and names no camera for 5 or 6. [16] **[S]** LibRaw treats
values 7 and 9 as a sign of a fixed-lens body, which suggests they come from
Coolpix compacts [6] **[S]**; whether Z bodies write 9 for their 12-bit
uncompressed files is not established **[I]**. The ExifTool names are also
listed in
[makernote-and-metadata.md](makernote-and-metadata.md#3-nefcompression-and-bit-depth).

**No surveyed reader chooses the layout by NEFCompression**, except that
rawler refuses the HE values 13 and 14. They go by TIFF Compression, the
strip's size and the 0x0096 header (see
[Telling the layouts apart](#telling-the-layouts-apart)). [4][11][14]
**[S]** rawler's source says that firmware bugs make the compression flags
of some files unreliable. [14] **[S]** Use the value as a cross-check.

## Telling the layouts apart

Let W and H be the raw IFD's width and height, and B the strip's byte
count. No tag names the layout: TIFF Compression 34713 covers the
Huffman data and some packed layouts, and NEFCompression is only a
cross-check (see [NEFCompression](#nefcompression)). The strip's first
bytes and its size tell the layouts apart: [1][3][4][5][11] **[S]**

| Strip | Layout |
|---|---|
| starts with `FF 10 FF 50` | HE/HE\* (JPEG XS); see [HE and HE\*](#he-and-he) |
| B = ⌈W/10⌉ · 16 · H | 12-bit packed with a pad byte after every 10 pixels (D100) |
| 2B = 3 · W · H | 12-bit packed, 1.5 bytes per pixel |
| B = ceil16(⌈3W/2⌉) · H, where ⌈3W/2⌉ is not a multiple of 16 | 12-bit packed, rows padded to 16 bytes |
| B = 3 · W · H | sNEF, 3 bytes per pixel |
| B = 2 · W · H | one sample per 16-bit word |
| B = ceil16(⌈14W/8⌉) · H | 14-bit packed, rows padded to 16 bytes (D6, Z) |
| B = 3 · ceil16(⌈14W/8⌉) · H | three 14-bit packed frames of a pixel-shift merge (Z bodies and the D6) |
| several strips, each full strip with 2 · B_strip = 3 · rows · W | striped 12-bit packed |
| none of these sizes, Compression 34713 | Huffman |

where ceil16(x) rounds x up to a multiple of 16.

- **The D100's padded layout is tagged as compressed**: TIFF Compression
  34713 over packed data. [1][11] **[S]** See
  [Per-body quirks](#per-body-quirks).
- **Uncompressed rows may carry padding** of up to 15 bytes, so that a
  row's bytes exceed W · BitsPerSample / 8 by less than 16. [11] **[S]**
- **The nibble order of 12-bit packed data** depends on the body: under
  Compression 34713 it is little-endian on bodies whose Model starts
  "NIKON" (DSLRs, Z bodies and Nikon 1), and big-endian on "COOLPIX" and
  "E" models; under Compression 1 it is big-endian. [1] **[S]** See
  [Uncompressed and packed layouts](#uncompressed-and-packed-layouts).
- **One sample per 16-bit word**, under Compression 34713, is read
  big-endian by dcraw. [1] **[S]**
- **The pixel-shift row** is LibRaw's reading alone: it reads a strip
  three times the 14-bit packed size as three frames of a pixel-shift
  merge. [5] **[S]**

The first row is confirmed on the Z 9: the strips of files 5147 (HE\*)
and 5148 (HE) start `FF 10 FF 50`, and the strip of the lossless file
5146 starts `A2 E6 43`, with no marker. All three carry TIFF Compression
34713, so only the strip's first bytes tell them apart. **[F]** The other
rows and the notes above come from other programs' source alone, and
wait for a sample file before they are coded (see
[open-questions.md](open-questions.md#2-facts-to-establish-from-sample-files)).

### Disagreements

- **Sizes that fit two layouts.** The formulas can coincide for some W
  and H. **[I]** On such a file dcraw and LibRaw read the D100 pad
  layout, and rawspeed reads uncompressed data. [1][4][11] **[S]**
- **HE files.** LibRaw looks for the JPEG XS markers only on four models
  and only in a raw IFD of one strip, and reads other HE files as Huffman
  data. [4][31] **[S]** See [HE and HE\*](#he-and-he).
- **Several strips.** rawler reads them as one block, LibRaw reads each
  from its own offset, and rawspeed accepts only one for Huffman data.
  [3][11][14] **[S]**

Recommendation: compute every candidate with checked arithmetic. If more
than one matches, prefer the one consistent with NEFCompression and record
the ambiguity. Look for the HE markers on every file, before anything
else. **[I]**

## The Huffman scheme

### Overview

Compressed NEF is a per-pixel DPCM over the CFA: each sample is predicted
from an earlier sample of the same color, the difference is coded with a
fixed Huffman alphabet of JPEG-style magnitude categories, and the decoded
value goes through a one-dimensional lookup, the curve. [1][2][10] **[S]**

"Lossy" Nikon compression *is* the curve: the camera first quantizes the
ADC values to fewer levels, then codes the quantized indices losslessly.
"Lossless" uses an identity curve. [19] **[S]**

**The Huffman tables are not stored in the file.** Every decoder hard-codes
the same six tables; only the predictor seeds and the curve come from
0x0096. [1][2][10] **[S]** Some community descriptions say otherwise; they
are wrong on this point.

### The six tables

Codes are canonical, assigned in JPEG DHT order from the number of codes of
each length. [10] **[S]** The counts below are for code lengths 1 to 16.
They were checked against dcraw 9.28 and match in LibRaw and rawspeed; they
are structural facts about the code, and the symbol lists are deliberately
omitted. [1][2][10] **[S]**

| Table | Use | Codes of length 1–16 | Codes | Longest |
|---|---|---|---|---|
| 0 | 12-bit lossy | 0,1,5,1,1,1,1,1,1,2,0,0,0,0,0,0 | 14 | 10 |
| 1 | 12-bit lossy, after split | 0,1,5,1,1,1,1,1,1,2,0,0,0,0,0,0 | 14 | 10 |
| 2 | 12-bit lossless | 0,1,4,2,3,1,2,0,0,0,0,0,0,0,0,0 | 13 | 7 |
| 3 | 14-bit lossy | 0,1,4,3,1,1,1,1,1,2,0,0,0,0,0,0 | 15 | 10 |
| 4 | 14-bit lossy, after split | 0,1,5,1,1,1,1,1,1,1,2,0,0,0,0,0 | 15 | 11 |
| 5 | 14-bit lossless | 0,1,4,2,2,3,1,2,0,0,0,0,0,0,0,0 | 15 | 8 |

The last two columns follow from the counts. **[I]** No code is longer than
11 bits. Table 0 has 14 codes and so 14 symbols; whoever transcribes the
lists should check that none is missing, since one published list gives
table 0 one symbol fewer. **[I]**

The symbol lists are not given here (see
[open-questions.md](open-questions.md)). Tables 0 and 1 share their counts
but not their symbols.

### Table selection

The table follows from the version bytes of 0x0096 and the raw IFD's
BitsPerSample, as the "Use" column says: tables 0 and 3 for lossy files
at 12 and 14 bits, tables 2 and 5 for lossless ones (first version byte
0x46). A lossy file whose split row is set (see [Split files](#split-files))
changes to table 1 or 4 from that row on. Only lossy files of first
version byte 0x44 have a split row, so the lossless tables have no
after-split partner. [1][2][10] **[S]**

### The difference after each code

Each code stands for a one-byte symbol. Its low four bits are the
category c, the number of significant bits of the difference; its high
four bits are a shift q, which is non-zero only in the after-split
tables. [2] **[S]** After-split symbols therefore have byte values above
16; rawspeed once rejected them, which broke D5500 and D500 14-bit lossy
files. [20] **[S]**

- **Category 0** is a difference of 0, with no bits after the code.
- **Otherwise c − q bits follow** the code, read as an unsigned number v
  (none when q = c, so that v = 0). The magnitude m is v when q = 0, and
  v · 2^q + 2^(q−1) when q > 0: the middle of a bucket 2^q values wide.
  [2] **[S]**
- **The sign** follows the JPEG convention. When m < 2^(c−1) the
  difference is negative: m − (2^c − 1) when q = 0, and m − 2^c when
  q > 0. Otherwise it is m. [2] **[S]**
- **A shift above the category** cannot occur with the fixed tables
  **[I]**; a decoder treats it as corrupt data.
- **There is no category 16.** JPEG lossless gives category 16 a special
  meaning (−32768, no extra bits), but the category is a four-bit field
  and no Nikon table uses 16. [2][10] **[S]** A decoder that widens
  symbols should reject categories above 15.

These facts come from other programs' source alone and wait for a sample
file before they are coded (see
[open-questions.md](open-questions.md#2-facts-to-establish-from-sample-files)).

### Bitstream

The bitstream starts at the strip offset, is read MSB-first, has no JPEG
`FF 00` byte stuffing, and runs continuously across rows with no per-row
byte alignment. [1][10][14] **[I]** (inferred from how the decoders read
it). The data carry no end marker; past the end dcraw decodes zero bits
**[I]**, and raw-files needs its own documented policy (see
[Decoder robustness checklist](#decoder-robustness-checklist)).

## The 0x0096 block

### Layout

All 16-bit values are in the MakerNote's byte order. [2][10][14] **[S]**

| Offset | Size | Field |
|---|---|---|
| 0 | u8 | first version byte |
| 1 | u8 | second version byte |
| 2 | 2110 bytes | present only when the first version byte is 0x49 **or** the second is 0x58; the fields below then start 2110 bytes later |
| 2 | 4 × u16 | predictor seeds; the order is disputed, see [Disagreements](#disagreements-1) |
| 10 | u16 | N, the number of curve values |
| 12 | N × u16 | curve values; a lossless file's curve is the identity whatever they hold (see [The curve](#the-curve)) |
| 562 | u16 | the split row, on lossy files only (below); offset from the start of the block, **not** moved by the 2110 bytes |

Notes: [1][2][10][14][18]

- **Either version byte brings the 2110 bytes.** dcraw, rawspeed and
  rawler agree that a first byte of 0x49 or a second byte of 0x58 is
  enough. The combination looks odd but is universal; keep it. **[S]** No
  source names a body that writes 0x49 or 0x58 or says what the 2110
  bytes hold; one write-up says its author does not know why. [18]
  **[S]**
- **Which files have a split row.** Files of version bytes 0x44 0x20
  have one in every reader, dcraw adding the condition that the anchor
  spacing of [The curve](#the-curve) is above 0. [1] **[S]** rawspeed,
  rawler and LibRaw also read one for 0x44 0x40. [2][10][14] **[S]** A
  sweep of raw.pixls.us samples reports a non-zero value at offset 562 on
  a Z 6II file. [41] **[S]**

### Disagreements

- **Order of the four predictor seeds.** dcraw and LibRaw read them row
  by row: even row column 0, even row column 1, odd row column 0, odd row
  column 1. rawspeed and rawler read them column by column: even row
  column 0, odd row column 0, even row column 1, odd row column 1.
  [1][2][10][14] **[S]** If the four seeds are always equal the question
  is moot; reading them from sample files settles it. **[I]**

### Version bytes by generation

| Version bytes | Meaning | Bodies reported | Conf. |
|---|---|---|---|
| 0x44 0x10 | lossy type 1 (NEFCompression 1), full stored curve | D100, D200, D2X, D40, D80, D60, 12-bit [22][41] | **[S]** [22] |
| 0x44 0x20 | lossy type 2 (NEFCompression 4), sparse curve, optional split | D300, D90, D5000 (Clévy) [22] | **[S]** [22] |
| 0x46 0x30 | lossless (NEFCompression 3), identity curve | D300, D700, D3X (Clévy) [22] | **[S]** [22] |
| 0x44 0x40 | lossy "2nd version"; rawspeed and rawler take 2 bits less per sample for its curve (14 to 12, 12 to 10) | D780, D6, Z 5, Z 6, Z 7, Z 7II [14][19][41] | **[S]** (rawler names D780 and Z 7 **[S]**) |
| 0x49 any, or any 0x58 | 2110 bytes before the seeds | none known | **[S]** |

- For 0x44 0x10 a full stored curve agrees with dcraw, which reads a
  sparse curve only for 0x44 0x20. [1] **[S]** It also agrees with the N
  values Clévy read (683 for lossy type 1, 567 for the D100) and with
  Claff's per-body sizes (683 for the D200). [1][19][22] **[I]**
- The D1H, D1X, D70 and D2H are not covered by any source. **[I]**
- Read as ASCII, 0x44, 0x46, 0x49 and 0x58 are D, F, I and X. That may be
  coincidence. **[I]**

## The curve

The curve maps each decoded value to the output value. [1][2] **[S]** Let
b be the raw IFD's BitsPerSample (12 or 14) and L = 2^b. When N > 1, the
anchor spacing is s = ⌊L / (N − 1)⌋. The version bytes say what the curve
is: [1][2] **[S]**

| Version bytes | Curve |
|---|---|
| 0x44 0x20 (lossy type 2) | sparse: the N stored values are anchors at the values 0, s, 2s, …, and a value between two anchors takes the value on the straight line between them |
| 0x44 0x40 ("2nd version") | sparse over L / 4 values in LibRaw; the readers disagree (below) |
| 0x44 0x10 and other lossy files | stored in full: the N values are the curve for the values 0 to N − 1 |
| first byte 0x46 (lossless) | identity |

- **Between two anchors** A_k at ks and A_(k+1) at (k + 1)s, a value x
  maps to (A_k · ((k + 1)s − x) + A_(k+1) · (x − ks)) / s, in integer
  division. [2] **[S]**
- **A full curve** covers the values below N, and a larger value is out
  of range. dcraw and LibRaw read a full curve only when N ≤ 0x4001.
  [1][2] **[S]**
- **No spacing.** When N − 1 > L the spacing is 0 and no sparse curve
  fits; dcraw then reads the values as a full curve. [1] **[S]**
- **A flat tail.** A curve can end in a run of equal values; LibRaw's
  range limit leaves that run out. [2] **[S]**

These facts come from other programs' source alone and wait for a sample
file before they are coded (see
[open-questions.md](open-questions.md#2-facts-to-establish-from-sample-files)).

**Stored N values read directly** (Clévy, from parsed 0x0096 blocks)
[22] **[S]**:

| N | Where |
|---|---|
| 32 or 34 | lossless |
| 257 | lossy type 2 (sparse anchors) |
| 567 | D100 |
| 683 | lossy type 1 (curve stored in full) |

For 257 anchors, s is L / 256: 16 at 12 bits and 64 at 14 bits. Both
divide L exactly, since 257 − 1 = 256 divides 4096 and 16384, so the
anchors tile the range. **[I]**

**Encoding-table sizes** on Claff's site are 567, 683, 689, 769, 1025, 2753,
3073 and 4097, and the first quantized level (where values start to be
skipped) lies between 107 and 1601, depending on the body. [19] **[S]** How
these sizes relate to the stored N is unconfirmed: 567 and 683 coincide
with Clévy's values, but the larger ones are not seen in his data. **[I]**

| Encoding-table size | Bodies (Claff) [19] **[S]** |
|---|---|
| 567 | D1H, D100, D1X (lossy type 1) |
| 683 | D70, D50, D40, D60, D80, D3000, D2H, D2X, D200 |
| 689, 2753 | D3, D300, D700 generation, lossy type 2 |
| 769, 3073 | D3S, D3X, D4, D800, D850, Df, D600, D7000, D7100, D500 and others |
| 1025, 4097 | D7200, D750, D810, D5500, D5600; "2nd version" on D780, Z 6, Z 7 |

### Disagreements

- **The 0x44 0x40 curve.** dcraw reads it as a full curve. [1] **[S]**,
  consequence **[I]** LibRaw reads it as sparse over L / 4 when s > 3,
  with s and L divided by 4. [2] **[S]** Yet LibRaw also limits the decoded
  values to L / 4 on every 0x44 0x40 file, whatever s, a conflict to
  settle with samples. [2] **[S]**, conflict **[I]** rawspeed and rawler
  instead read it with L = 2^(b−2); rawspeed names the Z 7, rawler the
  D780 and Z 7. [10][14] **[S]** LibRaw and rawspeed agree when N − 1
  divides L / 4. **[I]**
- **Anchor coverage.** rawspeed requires (N − 1) · s = L and rejects the
  file otherwise; LibRaw interpolates past the anchors without checking.
  [2][10] **[S]** The last segment needs the last anchor, which is in
  bounds only if (N − 1) · s ≥ L − 1. **[I]**
- **Full-curve bounds.** rawspeed rejects N = 0 and N > 0x4001. [10]
  **[S]** Neither dcraw nor LibRaw checks that the block actually holds
  2N bytes of curve **[I]**; raw-files must.

Recommendation: follow rawspeed's strictness (the anchors must tile L;
the N of 257 that Clévy read satisfies it at both bit depths) and read
0x44 0x40 as a sparse curve from the version bytes, as rawspeed and
rawler do. **[I]**

## Prediction

Each sample is the sum of its prediction and the decoded difference, and
the sum wraps modulo 2^16. [1][2] **[S]**

- **Columns 2 onward** are predicted from the sample two columns to the
  left in the same row, which has the same CFA color. [2] **[S]**
- **Columns 0 and 1** are predicted from the same column two rows up, the
  last row of the same parity. In the first two rows they start from the
  seeds: the seed of the row's parity and the column. [2] **[S]** It is not
  a general "row above" predictor.
- **Every column is coded**, including those a crop removes later.
  [1][2][10] **[S]**
- **The output** is the curve's value for the sample, a negative sample
  taken as 0 and a large one limited to the curve's range (the readers
  disagree on the limit, below). [2] **[S]**
- **The valid range.** Up to the split row a sample lies in 0 to L − 1;
  from the split row on, in −16 to L + 15 (see
  [Split files](#split-files)). With a full curve, N takes the place of
  L. [1][2] **[S]** dcraw and LibRaw count a sample outside that range as
  a data error and go on. [1][2] **[S]**

These facts come from other programs' source alone and wait for a sample
file before they are coded (see
[open-questions.md](open-questions.md#2-facts-to-establish-from-sample-files)).

- **Arithmetic.** In Rust use `wrapping_add`, or a wider type with
  explicit range checks; plain `+` on `u16` panics in debug builds. **[I]**
- **The limit on the curve's index differs.** LibRaw limits it to 0x3fff
  at both bit depths, rawspeed to 15 bits, rawler to the effective bit
  depth. [2][10][14] **[S]** All stay in bounds, but they give different
  pixels on corrupt data. Limit it to min(curve length, L) − 1 and count
  the violations. **[I]**
- **Dithering.** rawspeed and rawler can add a pseudo-random fraction of
  the step to the next curve value; LibRaw and dcraw do not. [12][14]
  **[S]** A library that returns sensor data should return the plain
  lookup.
- **Size limits.** rawspeed requires an even width, W ≤ 8288, H ≤ 5520
  and 12 or 14 bits. [10] **[S]** These caps are rawspeed policy (the
  D850's 8288 × 5520 sits exactly on them); raw-files should bound sizes
  by the bytes present instead. **[I]**

## Split files

Some lossy type 2 files switch to the after-split tables part-way down the
image: [2][20]

- The row is the u16 at offset 562 of 0x0096, and it is signaled nowhere
  else, not in Exif. **[S]**
- After the split, symbols carry a non-zero shift, so differences are
  coarser, and the range check moves (see [Prediction](#prediction)). [2]
  **[S]**
- Reported bodies: the Nikon 1 J5 and D3400 at 12 bits, the D5500 and D500
  at 14 bits, in high-ISO lossy files. [20] **[S]** The trigger is unknown;
  a slow card was suspected and then contradicted. [20] **[S]**
- rawspeed treats a split row of H or more as no split. [10] **[S]**
  Switch tables at most once.

## Uncompressed and packed layouts

Bit formulas below use bytes `b0, b1, …` in file order. Two 12-bit nibble
orders exist; never assume one without evidence. [1][3][11][14]

| Layout | Pixel formula | Row stride | Bodies | Conf. |
|---|---|---|---|---|
| 12-bit packed, big-endian | `p0 = b0<<4 \| b1>>4`, `p1 = (b1&15)<<8 \| b2` | 1.5 W | old DSLR uncompressed (D1, D1X, D2X, D200 …) | **[S]** |
| 12-bit packed, little-endian | `p0 = b0 \| (b1&15)<<8`, `p1 = b1>>4 \| b2<<4` | 1.5 W, rounded up to 16 | Z 6, Z 7 12-bit uncompressed; other little-endian files | **[S]** |
| 12-bit with pad byte | big-endian as above; a zero pad byte after every 15 data bytes (10 pixels) | 16 per 10 pixels | D100 **[S]**; TIFF Compression 32769 (generic in dcraw) | **[S]** for the layout |
| 14-bit in 16-bit words | one u16 per pixel in the TIFF byte order | 2 W | D3, D300, D800, D810 uncompressed | **[S]** |
| 14-bit packed | 7 bytes as a 56-bit little-endian integer G; pixel n = `(G >> 14n) & 0x3FFF`, n = 0..3 | ceil16(⌈14W/8⌉) | D6; Z uncompressed (NEFCompression 10) | **[S]** |
| 12-bit in 32-bit words | 32-bit little-endian words, 12-bit fields taken from the most significant end | from W | Coolpix NRW: P6000, P7000, P7100, B700, P950, P1000 | **[S]** |
| 12-bit in 16-bit words | one u16 per pixel, 12 significant bits | 2 W | Coolpix P330, P340, P7700, P7800, A1000 | **[S]** |
| interlaced fields | 12-bit big-endian packed; all even rows, then all odd rows | 1.5 W | Coolpix E5000, E5400, E5700, E8400, E8800 | **[S]** |
| sNEF | see [Small NEF](#small-nef-snef) | 3 W | D4S, D810 RAW S | **[S]** |

Notes:

- **Big-endian 12-bit.** rawspeed and rawler read 12-bit packed data as
  big-endian in a big-endian file, rawspeed with exceptions per camera.
  [11][14] **[S]**
- **Little-endian 12-bit.** Rows are padded to a multiple of 16 bytes,
  ceil16(⌈3W/2⌉); LibRaw refuses rows under 2000 or over 64000 bytes. [3]
  **[S]** dcraw reads the same nibble order from unpadded rows of "NIKON
  …" bodies (see [Telling the layouts apart](#telling-the-layouts-apart)).
  [1] **[S]**, equivalence **[I]** RawTherapee added the Z 7 12-bit case
  in 2018. [28] **[S]** rawler reads this layout in a little-endian file
  with under 16 bytes of padding per row. [14] **[S]** LibRaw's
  out-of-bounds read CVE-2026-5342 was in its reading of this layout,
  fixed in 0.22.1. [32] **[P]**
- **Pad byte.** Each pad byte is zero; dcraw reports an error otherwise.
  [1] **[S]**
- **14-bit in 16-bit words.** A row is 2W bytes at 14 bits per sample.
  [11] **[S]** A Z 7II uncompressed file is about 84 MB against 97 MB for
  a D850, because the D850 does not pack. [29] **[S]**
- **14-bit packed stride.** LibRaw's row length can fall short of
  ceil16(⌈14W/8⌉) when W is not a multiple of 4. [3] **[S]** RawTherapee
  first used fixed per-body paddings
  (8 bytes for the Z 7, 12 for the Z 6); they were wrong, and the fix
  derives the stride from the width. [28] **[S]** Use `⌈14W/8⌉` rounded up
  to 16, and cross-check it against B / H. **[I]**
- **Z 9 and later.** rawspeed lists only 14-bit compressed modes for the
  Z 9, Z 8, Z f and Z 6III; they have no uncompressed layout. [13] **[S]**
- **The Z 6/Z 7 TIFF byte order** is not confirmed; the 12-bit packing is
  little-endian whatever the header says. [3][1] **[I]**
- **P7800.** rawler notes the file is little-endian while the data is
  big-endian. [14] **[S]**

### Striped 12-bit packed

NEFCompression 5 is probably this layout **[I]**: several strips
(RowsPerStrip < H), each full strip holding exactly `rows · W · 3 / 2`
bytes. The data are 32-bit little-endian words read from their most
significant bit, and each row ends in padding bits. [3][4] **[S]** LibRaw
tells the layout by its size alone and reads each strip from its own
offset; rawler reads the strips as one block and says this is safe for
every known model. [3][4][14] **[S]** Strips need not be contiguous in a
hostile file, so honor each offset. **[I]**

## Per-body quirks

- **D100.** Data tagged as compressed (TIFF 34713 or NEFCompression 2) is
  often 12-bit packed with pad bytes. Every sixteenth byte of the padded
  layout (offsets 15, 31, 47, …) is a zero pad byte, so a non-zero byte at
  one of them shows Huffman data. [11] **[S]** The stored width is
  unreliable: rawspeed reads the packed data from the second IFD that has
  StripOffsets, at 3040 × 2024 [11] **[S]**; rawler takes a width of 3040
  for every D100 [14] **[S]**; dcraw widens the packed files by 3 columns
  [1] **[S]**. libopenraw reports
  that most D100 files are packed because compression froze the camera.
  [17] **[S]**
- **D1.** Uncompressed. dcraw scales the red and blue white balance
  multipliers (red by 256/527, blue by 256/317); rawspeed carries a
  matching setting, and LibRaw moves them to its analog balance instead.
  [1][7][13] **[S]**
- **D1X.** 4024 × 1324 stored, with a pixel aspect ratio of 0.5 in dcraw and
  LibRaw: each stored row covers two output rows. Double the height (or
  halve the width) to get square pixels and a roughly 3:2 image. dcraw trims
  the width by 4. [1][13] **[S]**
- **Coolpix E-series diagnostic raws.** Headerless dumps from the E900 to
  the E5700 era, identified by file size alone, some with four-color CMYG
  filters, and told apart from other brands' files by byte histograms.
  [1][7] **[S]** They are not NEF; out of scope.
- **Coolpix E5000, E5400, E5700, E8400, E8800 NEF.** Interlaced fields (see
  the table above); the E5000 and E5700 use a CMYG CFA. [1][13] **[S]**
- **Coolpix NRW** (P6000, P7000, P7100, B700, P950, P1000). 12-bit in 32-bit
  words. dcraw and LibRaw set black to 255 on P7000-class bodies at
  ISO 400 and above. [7][13] **[S]**
- **Coolpix P330, P340, P7700, P7800, A1000.** 12 bits in 16-bit words;
  LibRaw sets the maximum to 65504 for the P7700 and P7800. [7][13]
  **[S]** The P7800 has a little-endian file with big-endian data. [14]
  **[S]**
- **Nikon 1.** Standard Huffman NEF; LibRaw trims 2 rows. [7] **[S]**

## Small NEF (sNEF)

The D4S and D810 "RAW S" files store YCbCr, not CFA data. [21] **[S]**

- **Detection.** B = 3 · W · H: three bytes per pixel. [1][11] **[S]**
  ExifTool names NEFCompression value 8 "Small" [16] **[S]**; that sNEF
  files carry it is **[I]**.
- **Packing.** Every 6 bytes, read as a 48-bit little-endian integer, hold
  four 12-bit samples: sample k is bits 12k to 12k + 11, in the order Y0,
  Y1, Cb, Cr. [2] **[S]**, [21] **[S]**
- **Format.** 4:2:2: one chroma pair per two horizontal pixels; chroma is
  offset by 2048; luma uses about 11 bits (maximum roughly 2392 to 2549,
  depending on white balance). D810 FX sNEF is 2464 × 1640. [21] **[S]**
- **Not scene-linear.** The camera has already applied white balance and a
  gamma-like tone curve. [21] **[S]**

### Conversion: the implementations disagree

| Implementation | Chroma for the odd pixel | YCbCr to RGB | Linearization | White balance |
|---|---|---|---|---|
| dcraw | the same pair as the even pixel | coefficients close to BT.601 (1.3707 for Cr to R) | sRGB-like gamma | divides by the camera multipliers to undo it [2] **[S]** |
| rawspeed | average of this pair and the next | as dcraw | the same gamma, scaled | multiplies R and B by the inverse of 0x000C; rejects implausible values [11] **[S]** |
| rawler | one pair per two pixels | not read in detail | gamma-like curve | inverse white balance [14] **[S]** |
| LibRaw | averaged with the next pair; can be disabled | BT.601 (1.402 for Cr to R); luma normalized by 2549; chroma forced neutral near the top of the range | an empirical curve | marked as already applied [3][5] **[S]** |
| RawDigger analysis | shared per pair | sYCC (BT.601), 2048 midpoint | about gamma 2; suggests deriving a curve from paired captures | applied in-camera in the linear domain [21] **[S]** |

No implementation has ground truth. Expose sNEF as a three-channel,
non-CFA, already white-balanced, non-linear image, with the raw YCbCr
samples available and any RGB conversion documented as an approximation.
W must be even (rawspeed also requires W ≥ 6), and the last pair needs an
edge rule because averaging looks one pair ahead. [11] **[S]**, policy
**[I]**

## Reduced-size raws on later bodies

- **D850 RAW M and S.** Nikon's manual says medium and small NEFs are always
  recorded as 12-bit lossless compressed, whatever the compression and
  bit-depth settings; the menu sizes are 8256 × 5504 (L), 6192 × 4128 (M)
  and 4128 × 2752 (S). [23] **[P]** ExifTool reports stored sizes of
  8288 × 5520, 7104 × 4728 and 6216 × 4136; converters then scale to the
  menu size. [24] **[S]**
  - They are most likely a smaller Bayer CFA, not YCbCr: Iliah Borg
    (LibRaw, RawDigger) said so in 2017, and a review using RawDigger
    reports the camera resampling the raw data to the stored sizes. [25]
    **[S]**
  - Neither rawspeed nor LibRaw treats the D850's small raws apart: both
    read sNEF only when B = 3 · W · H, so they read these files as Huffman
    data. [4][13] **[S]**, consequence **[I]**
  - Not confirmed at the byte level, nor which Huffman table they use
    (presumably the 12-bit lossless one **[I]**).
- **Z 7II and Z 6II** offer M and S NEF, recorded at 12 bits with lossless
  compression. [26] **[P]**
- **Z 8, Z 6III and Z f** record NEF at Large size only. [27] **[P]** The
  Z 9 was not checked; its specification lists M and S only for JPEG and
  HEIF [44] **[S]**, so it is probably Large only **[I]**. A user reports
  shooting M-size NEF on an original Z 7. [24] **[S]**
- **Detecting the size.** ExifTool documents MakerNote 0x003E ImageSizeRAW:
  1 Large, 2 Medium, 3 Small. [15] **[S]**

## HE and HE\*

- **What it is.** Nikon announced intoPIX TicoRAW for the Z 9 in December
  2021, delivered by firmware in 2022. intoPIX calls it patented and gives
  no algorithm detail. [33] **[P]** It is a modified JPEG XS (ISO/IEC 21122)
  with a Bayer decorrelation step. [34][35] **[S]** The JPEG XS second
  edition (2022) added Bayer support, and an amendment defines a CMainBayer
  profile. [36] **[S]**
- **Bodies.** Z 9 (from firmware 2.00), Z 8, Z f, Z 6III, and reportedly the
  Z5 II. [4][31] **[S]** NEFCompression is 13 (HE) or 14 (HE\*), in
  tag 0x0051. [16] **[S]**
- **Sniff.** The strip starts with the JPEG XS SOC and CAP markers,
  `FF 10 FF 50`. [4] **[S]** Confirmed on the Z 9: the HE\* and HE strips
  of files 5147 and 5148 start with them, and the lossless strip of file
  5146 does not. **[F]**
- **Reverse-engineered details**, from two unmerged pull requests [34][35]
  **[S]**: a 5/3 wavelet; four components from a Star-Tetrix-style
  decorrelation of the Bayer planes, which the header does not signal; the
  long packet header always; extra sign bits past a short final group;
  chroma scaling with square-root companding; output is 14-bit RGGB. HE and
  HE\* reportedly differ in precinct parameter ranges. Some constants were
  fitted empirically from one scene.

### LibRaw's model list

LibRaw recognizes HE strips only on the Z 9, Z 8, Z f and Z 6III, and
only in a raw IFD of one strip; it refuses those files as unsupported.
[4][8] **[S]** It does not recognize them on the Z5 II, the Z50 II or the
ZR. [4] **[S]** On such a body LibRaw returns noise and a "data corrupted"
warning, not an error. [31] **[S]**

The lesson: look for `FF 10 FF 50` at the strip start of every NEF,
whatever the model and the strip count, before choosing any other path.
**[I]**

### Open-source status (October 2026)

| Project | HE/HE\* status | Conf. |
|---|---|---|
| LibRaw | 0.21 reads the NEFCompression tag for HE; 0.22 (Changelog dated 2025-12-25) supports recent Z bodies with "standard compression only". The maintainers say their own decoder will come in a snapshot in autumn 2026. [9][35] | **[P]**/**[S]** |
| LibRaw PR #826 | open, not merged; adds a decoder for the HE strips of the Z 9, Z 8, Z f and Z 6III [35] | **[P]** |
| dnglab (rawler) | returns an error [14] | **[S]** |
| dnglab PR #835 | experimental decoder, opened 2026-08-30, still open; tested on one body (Z 6III); describes itself as largely AI-written; compared against a C reference decoder; the maintainer wants output to match LibRaw's coming decoder [34] | **[P]** |
| rawspeed, darktable | not supported [38] | **[S]** |
| RawTherapee | not supported; Adobe DNG Converter suggested as a workaround [39] | **[S]** |
| Adobe | Camera Raw and DNG Converter decode HE, but not on some ARM platforms, which fits a licensed binary SDK [40] | **[S]** |

### Patents: a legal question

JPEG XS has a royalty-bearing patent pool (Fraunhofer IIS and intoPIX,
administered by Vectis, announced 1 March 2022); rates are not public.
[37] **[S]** intoPIX describes TicoRAW as patented. [33] **[P]** An MIT or
Apache-2.0 crate with a JPEG XS decoder might still need a patent license
in some jurisdictions; copyright is a separate matter. **[I]** Whether
raw-files may ever decode HE is a legal question, not an engineering one; see
[open-questions.md](open-questions.md) and
[licenses-and-references.md](licenses-and-references.md).

## Levels, CFA, crop and white balance

### Black level

- **MakerNote 0x003D BlackLevel**, int16u[4], one value per CFA channel, in
  14-bit units: shift right by 14 − b, b being BitsPerSample, for 12-bit
  files. [1][11][15] **[S]** LibRaw divides by 4 for 12-bit files of
  bodies other than Coolpix. [5][6] **[S]**
- **Channel order: the implementations disagree.** LibRaw reads the four
  values as fixed colors R, G1, G2, B, whatever the CFA layout. [6] **[S]**
  rawspeed stores them by position after setting an RGGB pattern [11]
  **[S]**, and rawler also stores them by position [14] **[S]**. The two
  readings agree on RGGB bodies. **[I]** Every non-RGGB Nikon in rawspeed's
  camera list is a pre-2010 body, so the difference matters only if one of
  them writes 0x003D; which body first wrote it is not known. [13] **[S]**,
  consequence **[I]** Until samples settle it, take LibRaw's reading as the
  reference.
- **Bodies without 0x003D.** (Per-body values on this page are quoted from
  rawspeed's `cameras.xml` for cross-checking against samples, not for
  transcription into code.) rawspeed lists a black level of 0 for the D1
  to D3, D300, D700 and D7000 (the camera clipped or subtracted it), 400
  (14-bit) and 100 (12-bit) for the D850, and 1008 and 251–252 for Z bodies,
  which also write 0x003D. [13] **[S]**
- **Masked pixels.** NEFs are not known to carry usable optical-black
  margins; the per-body width trims in dcraw cut unusable columns, not
  calibrated black. **[I]**
- **Coolpix.** See [Per-body quirks](#per-body-quirks) for the P7000-class
  black of 255.

### White level

- **Nominal**: 2^b − 1, so 4095 or 16383.
- **Practical saturation is lower and per body.** rawspeed lists, at 14 and
  12 bits: D3 and D700 15892 / 3972; D300 15236 / 3808; D850 15520 / 3880;
  Z 6 and Z 7 15520 / 3880; Z 8, Z 9, Z f and Z 6III 15892. [13] **[S]**
  These are quoted from rawspeed's `cameras.xml` for cross-checking against
  samples, not for transcription into code.
- LibRaw uses its maximum divided by 1.07 as a linear limit when it knows
  nothing better. [5] **[S]**
- In lossy files the curve's last value bounds the output.
  **[I]**
- Derive per-body white levels from the project's own samples rather than
  copying a table (see [open-questions.md](open-questions.md)).

### CFA pattern

- **Sources.** The raw IFD carries TIFF/EP CFARepeatPatternDim (0x828D)
  and CFAPattern (0x828E). dcraw and LibRaw read them; rawspeed ignores the
  tag and uses its camera list; rawler uses the tag only to find the IFD.
  [1][11][14] **[S]**
- **Patterns by body** (rawspeed's camera list) [13] **[S]**:

  | Pattern | Bodies |
  |---|---|
  | BGGR | D1, D1H, D1X, D40, D50, D70, D70s, Coolpix E5400, E8400, E8800 |
  | GRBG | D100, D200 |
  | GBRG | D2H, D2Hs, D40X, D60, D80, D90, D3000, D3100, D5000 |
  | RGGB | most others, including the D2X and every Z body |
  | CMYG | Coolpix E5000, E5700 |

  These values are quoted from rawspeed's `cameras.xml` for cross-checking
  against samples, not for transcription into code.

- **The pattern depends on the origin.** dcraw sets an RGGB pattern for the
  D200 together with a left margin of one column, which is GRBG at the
  visible origin and agrees with rawspeed's GRBG. [1][13] **[S]**,
  reconciliation **[I]** Any CFA statement must say whether it is relative
  to the raw origin or to the cropped one, and the pattern must be
  re-phased when a crop starts on an odd row or column.
- Read 0x828E when present, check it against the color count, and allow
  per-body overrides. **[I]**

### Crop

- **MakerNote 0x0045 CropArea**: int16u[4] = left, top, width, height.
  [15] **[S]** rawler uses it as the crop; LibRaw stores it as an inset
  crop. [14] **[S]**, [6] **[S]**
- **MakerNote 0x001B CropHiSpeed**: int16u[7], the crop mode (off, 1.3x,
  DX, FX uncropped and so on) plus sizes and offsets; the field order is in
  [makernote-and-metadata.md](makernote-and-metadata.md#84-image-dimensions).
  [15] **[S]**
- **Per-body trims.** dcraw and LibRaw trim the width per body (for
  example D3 and D700 by 4 with a left margin of 2, D600 and D800 by 46, D4
  and Df by 52) [1][7] **[S]**; rawspeed has similar crops, and none for
  most Z bodies (Z 9: 12 columns) [13] **[S]**.
- NEF has no equivalent of DNG's DefaultCrop: crop comes from 0x0045 or a
  per-body table.

### White balance

White balance is in MakerNote 0x000C, in the encrypted 0x0097 and, on
Coolpix bodies, in the 0x0014 block; see
[makernote-and-metadata.md](makernote-and-metadata.md#6-white-balance). In
sNEF it is already applied to the data. [21] **[S]** The D1 needs the
scaling described under [Per-body quirks](#per-body-quirks).

## Decoder robustness checklist

Derived from the decoders' own guards and from past vulnerabilities; the
full history is in [security.md](security.md).

1. **Gate HE first.** Sniff `FF 10 FF 50` at every strip start and return a
   typed unsupported error. Never fall through to the Huffman decoder.
2. **Checked size arithmetic.** Compute W · H · b, ⌈W/10⌉ · 16 · H,
   ceil16(⌈14W/8⌉) · H and the other candidates with checked operations.
   Reject zero or absurd dimensions before allocating, and bound every
   allocation by the bytes present, never by W · H alone.
3. **Strips.** Honor each strip's own offset; check that every strip lies
   inside the file and that `rows · stride ≤ strip bytes`. LibRaw's
   reading of padded 12-bit rows had an out-of-bounds read here
   (CVE-2026-5342).
   [32] **[P]**
4. **The 0x0096 block, field by field.** Bounds-check the two version
   bytes, the 2110 bytes, the eight seed bytes, N, the 2N curve bytes
   and the two bytes at offset 562; a short block is common.
5. **The curve.** Reject N = 0; require N ≤ 0x4001 for a full curve;
   require the anchors to tile L for a sparse one; reject a zero spacing.
6. **Huffman.** Build the six fixed tables once. Treat a shift above the
   category as corrupt, and handle category 0 without shifting. Two
   Huffman-style
   bugs in the LibRaw family, both in Foveon/X3F code and not NEF, show the
   class: CVE-2017-6890 (`foveon_load_camf()` in the separate
   LibRaw-demosaic-pack-GPL2 add-on, before 0.18.2) and CVE-2020-35531
   (`get_huffman_diff()` in `x3f_utils_patched.cpp`, Sigma X3F). [42] **[S]**
7. **End of data.** A bitstream that ends mid-image is either an error or
   zero-filled with a flag; pick one and document it. Never panic.
8. **Predictor and lookup.** Wrapping or checked arithmetic; count range
   violations instead of failing; clamp the curve index to the curve's
   length.
9. **Split.** Ignore a split row of H or more; switch tables at most once.
10. **sNEF.** Require an even W; define the edge pair.
11. **Byte order.** Read the TIFF structures and the MakerNote each with
    its own byte order.
12. **Test and fuzz.** No lint enforces "never panic" (see
    [ADR008](../../architecture-decisions/adr008-lints-and-panics.md)), so
    truncated and corrupted strips in the tests, and fuzzing of the decoder
    from tiny hand-built strips, carry that guarantee.

## Recommendation for raw-files

For the raw sensor data capability: **[I]**

- **Scope.** Decode the Huffman path (lossless, lossy type 1 and type 2,
  including split files), the uncompressed and packed layouts of DSLRs and
  Z bodies, and the Coolpix NRW and 16-bit layouts. Return sNEF as YCbCr
  with a documented flag. Refuse HE/HE\* with a typed error. Leave Coolpix
  diagnostic raws out.
- **Output.** The undemosaiced samples at raw dimensions, plus the crop,
  the CFA pattern stated relative to a named origin, and black and white
  levels. No dithering. Development to a linear image builds on this
  output ([ADR010](../../architecture-decisions/adr010-decoding-ends-at-a-linear-image.md)).
- **Dispatch.** Look for the HE markers first; then compute every size
  candidate; resolve ties with NEFCompression; use the Huffman path only
  for Compression 34713 when no size candidate matches.
- **One parse of 0x0096** into a validated value holding the table, the
  seeds, the curve, its range and the split row, rejected as a whole when
  any field is out of range.
- **A sparse curve for 0x44 0x40**, from the version bytes, as rawspeed
  and rawler read it, with rawspeed's anchor check.
- **Corrupt data.** Decode what can be decoded and report a count of range
  violations, as LibRaw does with its data-error flag, rather than
  returning different pixels silently.
- **Data tables.** Do not commit the Huffman symbol lists until the
  provenance decision is recorded; derive black and white levels, CFA
  phases and crops from sample files.
- **Byte order.** Tolerate a TIFF/MakerNote mismatch rather than rejecting
  the file as rawler does, unless the project decides otherwise.
- **Tests.** One sample per layout row above, plus truncated and corrupted
  variants; see [test-data.md](test-data.md).

## Open questions

These are also tracked in [open-questions.md](open-questions.md).

- **0x0096 version bytes** for the D1H, D1X, D70 and D2H; which bodies
  write 0x49 or 0x58, and what the 2110 skipped bytes contain.
- **Predictor seed order**: read the four seeds from samples; dcraw and
  LibRaw (row-major) disagree with rawspeed and rawler (column-major), and
  the question is moot if the seeds are always equal.
- **NEFCompression 5 and 6**: which bodies write them, and whether 5 is the
  striped layout LibRaw decodes.
- **NEFCompression 9 on Z bodies**: whether their 12-bit uncompressed files
  carry 9, and the byte order of a Z 6/Z 7 TIFF header.
- **0x0051 on Z bodies**: 0x0093 is absent from the Z 9 files, the lossless
  one included, and from most Z bodies checked, while the Z fc and the Z 30
  write 0x0093 and no 0x0051 (files 8559 and 5812), so the compression of
  a Z body has to be looked for in both tags; byte 10 of 0x0051 reads 3,
  14 and 13 on files 5146 to 5148, but their byte order, little-endian on
  all three, cannot settle how the field is to be read on a big-endian
  MakerNote.
- **D850 M/S**: byte-level confirmation of a CFA layout and which Huffman
  table they use; whether the Z 9 and the original Z 6/Z 7 write M/S NEF.
- **0x003D**: the first body to write it, and its channel order on any
  non-RGGB body.
- **Curve sizes**: how Claff's encoding-table sizes relate to the stored
  N on bodies after 2009.
- **Lossless debut**: whether lossless compression first appeared on the
  D3/D300.
- **Implementation conflicts** to settle with samples: the curve-index
  limit, the 0x44 0x40 curve, the end-of-data policy, and the sNEF
  conversion.
- **Huffman symbol lists**: their provenance (a project decision).
- **HE/HE\***: whether any work on it is ever in scope (a legal question).

## Sources

All retrieved October 2026. Implementation sources are read only; see
[licenses-and-references.md](licenses-and-references.md). Entries marked
"search extract" were read only through search-result snippets.

[1] dcraw 9.28 (rev 1.478), dcraw.c — https://dechifro.org/dcraw/dcraw.c (`parse_makernote`, `nikon_load_raw`, `apply_tiff` case 34713, `packed_load_raw`, `identify`)
[2] LibRaw, decoders_dcraw.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/decoders_dcraw.cpp (`nikon_read_curve`, `nikon_load_raw`, `nikon_yuv_load_raw`)
[3] LibRaw, decoders_libraw.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/decoders_libraw.cpp (`nikon_load_padded_packed_raw`, `nikon_load_striped_packed_raw`, `nikon_14bit_load_raw`, `nikon_load_sraw`)
[4] LibRaw, tiff.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/tiff.cpp (`apply_tiff` case 34713, JPEG XS sniff)
[5] LibRaw, open.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/utils/open.cpp (`open_datastream`)
[6] LibRaw, nikon.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/nikon.cpp (`parseNikonMakernote`: 0x0051, 0x0093, 0x0096/0x008C, 0x003D, 0x0045)
[7] LibRaw, identify.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/identify.cpp (Nikon body rules, Coolpix flags)
[8] LibRaw, decoders_libraw_dcrdefs.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/decoders_libraw_dcrdefs.cpp (`nikon_he_load_raw`)
[9] LibRaw, Changelog — https://raw.githubusercontent.com/LibRaw/LibRaw/master/Changelog.txt ; tag 0.22.2: https://raw.githubusercontent.com/LibRaw/LibRaw/0.22.2/Changelog.txt
[10] rawspeed, NikonDecompressor.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decompressors/NikonDecompressor.cpp
[11] rawspeed, NefDecoder.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decoders/NefDecoder.cpp (`decodeRawInternal`, `D100IsCompressed`, `NEFIsUncompressed`, `NEFIsUncompressedRGB`, `DecodeNikonSNef`, `decodeMetaDataInternal`)
[12] rawspeed, RawImageDataU16.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/common/RawImageDataU16.cpp (dithered lookup)
[13] rawspeed, cameras.xml — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/data/cameras.xml (Nikon modes, hints, CFA, crops, black and white levels)
[14] rawler (dnglab), nef.rs — https://raw.githubusercontent.com/dnglab/dnglab/main/rawler/src/decoders/nef.rs
[15] ExifTool, Nikon tags — https://exiftool.org/TagNames/Nikon.html
[16] ExifTool, Nikon.pm — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Nikon.pm (`%nefCompression`)
[17] libopenraw, NEF format page — https://libopenraw.pages.freedesktop.org/formats/nef
[18] Leon Plickat, "Decoding Nikon NEF Files" — https://leon_plickat.srht.site/nikon-nef.html
[19] Bill Claff, "NEF Compression" — https://www.photonstophotos.net/NikonInfo/NEF_Compression.htm
[20] discuss.pixls.us, "Nikon: a specific raw sample wanted" — https://discuss.pixls.us/t/nikon-a-specific-raw-sample-wanted/5483
[21] RawDigger, "Nikon small raw internals" — https://www.rawdigger.com/howtouse/nikon-small-raw-internals ; summary: https://nikonrumors.com/2014/05/21/what-exactly-is-the-new-nikon-sraw-file-format.aspx/amp/
[22] Laurent Clévy, NEF notes v0.31 (2009) — http://lclevy.free.fr/nef/
[23] Nikon, D850 online manual, recording options — https://onlinemanual.nikonimglib.com/d850/en/07_recording_options_03.html
[24] DxO forum, "Nikon D850 RAW M size resolution is not recognized correctly" — https://forum.dxo.com/t/nikon-d850-raw-m-size-resolution-is-not-recognized-correctly/24178
[25] DPReview forum, "D850 M sRAW" — https://www.dpreview.com/forums/threads/d850-m-sraw.4204326/ ; Photography Life, Nikon D850 review p. 4 — https://photographylife.com/reviews/nikon-d850/4 (both search extracts only)
[26] Nikon, Z 7II/Z 6II online manual, technical notes — https://onlinemanual.nikonimglib.com/z7II_z6II/en/15_technical_notes_08.html
[27] Nikon online manuals, image size: Z 8 — https://onlinemanual.nikonimglib.com/z8/en/image_area_quality_and_size_30.html ; Z 6III — https://onlinemanual.nikonimglib.com/z6III/en/image_area_quality_and_size_34.html ; Z f — https://onlinemanual.nikonimglib.com/zf/en/image_area_quality_and_size_31.html
[28] RawTherapee issue #4801, "Support for Nikon Z7 uncompressed NEF files" (closed) — https://github.com/RawTherapee/RawTherapee/issues/4801
[29] RawDigger forum, Z 7II and D850 uncompressed packing — https://www.rawdigger.com/node/2454
[30] ExifTool forum, "Nikon now using little endian for maker note" — https://exiftool.org/forum/index.php?msg=23787
[31] rawmakase PR #412 (HE refused; LibRaw output on an ungated Z5 II) — https://github.com/pch/rawmakase/pull/412
[32] NVD, CVE-2026-5342 — https://nvd.nist.gov/vuln/detail/CVE-2026-5342 ; Debian tracker: https://security-tracker.debian.org/tracker/CVE-2026-5342
[33] intoPIX, TicoRAW in the Nikon Z 9 (press release, 2021-12-07) — https://www.intopix.com/blogs/post/TicoRAW-technology-added-with-High-Efficiency-RAW-recording-of-Nikon-Z-9-flagship-mirrorless-camera
[34] dnglab PR #835, "Add (experimental) Nikon HE compression Support" — https://github.com/dnglab/dnglab/pull/835
[35] LibRaw PR #826, "Add Nikon Z9 High-Efficiency (HE) RAW format support" — https://github.com/LibRaw/LibRaw/pull/826 ; LibRaw forum: https://libraw.org/node/2766
[36] JPEG XS, Bayer support — https://en.wikipedia.org/wiki/JPEG_XS ; Fraunhofer: https://www.digitalmedia.fraunhofer.de/en/mediainformation/trendbrochures/trendbrochure-2022/jpeg-xs-new-features.html ; CMainBayer amendment: https://knowledge.bsigroup.com/products/bs-iso-iec-21122-2-2024-amd-2-information-technology-jpeg-xs-low-latency-lightweight-image-coding-system-part-2-profiles-and-buffer-models-amendment-2-cmainbayer-profile
[37] JPEG XS patent pool — https://www.intopix.com/blogs/post/intoPIX-and-Fraunhofer-IIS-team-with-Vectis-to-offer-JPEG-XS-patent-pool-license ; https://jpegxspool.com/
[38] darktable, camera support — https://www.darktable.org/resources/camera-support/
[39] discuss.pixls.us, "Nikon Z series high efficiency support" — https://discuss.pixls.us/t/nikon-z-series-high-efficiency-support/50427
[40] HE decoding in Adobe software: bcgforums — https://bcgforums.com/threads/converting-nef-to-dng.29623/page-2 ; Capture One community — https://support.captureone.com/hc/es/community/posts/4415914839441-Nikon-Z9-RAW-NEF-High-Efficiency?page=4
[41] Third-party sample sweeps: storytold/photocraft #1310 (2026) — https://github.com/storytold/photocraft ; radialmonster/lightcraft #16 (2026-10-08) — https://github.com/radialmonster/lightcraft (search extracts only)
[42] NVD, Foveon/X3F Huffman-style vulnerabilities in the LibRaw family — https://nvd.nist.gov/vuln/detail/CVE-2017-6890 ; https://nvd.nist.gov/vuln/detail/CVE-2020-35531
[43] Licenses: LibRaw README — https://raw.githubusercontent.com/LibRaw/LibRaw/master/README.md ; dcraw COPYING (Debian) — https://sources.debian.org/src/dcraw/9.28-8/COPYING ; dnglab — https://github.com/dnglab/dnglab ; rawspeed — https://github.com/darktable-org/rawspeed ; rawloader — https://github.com/pedrocr/rawloader
[44] Nikon, Z 9 online manual, specifications — https://onlinemanual.nikonimglib.com/z9/en/specifications_348.html

### Sample files

Files 5146, 5147 and 5148: the three CC0 Nikon Z 9 files of raw.pixls.us,
one per compression mode (lossless, High Efficiency\* and High Efficiency).
Their SHA-256 hashes and URLs are in
[test-data.md](test-data.md#21-the-ci-set). They were checked on 13 October
2026 by a structural walk of the IFD0 chain, the SubIFDs, the Exif IFD and
the MakerNote's PreviewIFD, reading each JPEG's marker segments to its frame
header and its last two bytes.
