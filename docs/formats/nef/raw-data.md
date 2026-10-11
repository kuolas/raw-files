# NEF raw sensor data

How a Nikon NEF stores its sensor data: where the strip is, the compressed
(Huffman plus curve) scheme used by most bodies, the packed and unpacked
layouts, the small and reduced-size raws, and the HE/HE\* codec that raw-files
cannot decode. It ends with the checks a decoder of untrusted input needs and
a recommendation for milestone 4.

Back to [the NEF overview](README.md).

## How to read this page

Nikon publishes no NEF specification. Everything below comes from the
source code of existing decoders, from ExifTool's tag documentation, from
community write-ups, and from Nikon's camera manuals.

Every claim carries a confidence tag:

- **[P]** primary: stated in a specification, tag documentation, a camera
  manual or the source code of an implementation.
- **[S]** secondary: a forum post, blog, issue report, review or
  search-result extract.
- **[I]** inferred, or derived by arithmetic from primary facts; a hypothesis
  to test on sample files.

Numbers in square brackets such as [3] refer to [Sources](#sources).

**Offsets, formulas and constants taken from implementation source code must
be checked against that source and against sample files before they are
hard-coded.** Nothing on this page has yet been checked against real NEFs.

**Licence hygiene.** The decoders read for this page are under dcraw's own
licence, LGPL-2.1 or CDDL-1.0 (LibRaw), and LGPL-2.1 (rawspeed, rawler,
rawloader) [43]; see [licences-and-references.md](licences-and-references.md).
None of their code may be copied or transliterated into raw-files. This page
restates algorithms in prose and in original pseudocode only. It gives the six
Huffman tables as counts per code length, never their symbol lists, and
reproduces no curve data. A decoder does need the symbol lists; where raw-files
takes them from is an open decision (see
[open-questions.md](open-questions.md)).

## Where the raw data lives

- **The raw IFD.** A NEF is a TIFF file
  ([container-and-previews.md](container-and-previews.md)). The raw image is
  in the IFD whose PhotometricInterpretation is 32803 (CFA) or that carries
  a CFAPattern tag; it is usually a SubIFD of IFD0. rawspeed and rawler both
  locate it as the IFD with a CFAPattern tag, and rawler falls back to the
  full-resolution subfile. [11][14] **[P]**
- **The strip.** StripOffsets and StripByteCounts of that IFD give the data.
  Most bodies write one strip; rawler notes that the D500 writes several and
  sums their byte counts. [14] **[P]**
- **TIFF Compression (0x0103)** of the raw IFD takes three values: [1][14][17]
  - **34713**, Nikon compressed: the Huffman scheme below, although some
    packed layouts carry this value too (see
    [Choosing the decode path](#choosing-the-decode-path)). **[P]**
  - **1**, uncompressed. **[P]**
  - **32769**, a packed layout with one pad byte after every ten pixels:
    dcraw handles the value that way, generically and without naming a
    camera. [1] **[P]** Which bodies write it is not established; it is
    probably old ones. **[I]**
- **The compression parameters** are in MakerNote tag **0x0096**
  (NEFLinearizationTable in ExifTool) [15], with **0x008C** as a fallback.
  dcraw and LibRaw record the position of whichever of the two tags they
  parse last; rawspeed and rawler prefer 0x0096 and fall back to 0x008C.
  [1][6][11][14] **[P]** Offsets inside the MakerNote are relative to its
  embedded TIFF header, not to the file
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **Byte order.** The 16-bit values in 0x0096 are in the MakerNote's byte
  order, which can differ from the file's. [2][10] **[P]** Older NEFs are
  big-endian; Nikon began writing little-endian MakerNotes with the D7100
  and D5200 (2013), and files re-saved by Nikon's software can keep some
  big-endian structures. [30] **[S]** rawler rejects a file whose TIFF and
  MakerNote byte orders differ [14] **[P]**; LibRaw has a fix for NEFs
  damaged by Nikon Transfer [9] **[P]**.

## NEFCompression

MakerNote tag 0x0093 (NEFCompression) names the encoding. On the Z 9 and
later bodies 0x0093 is absent and the same value is an int16u at byte 10 of
tag 0x0051; ExifTool, LibRaw and rawler all read it there. [6][14][15][16]
**[P]** The sources disagree on the byte order: LibRaw reads the field in the
MakerNote's byte order, and only rawler forces little-endian. **[P]**

| Value | Name (ExifTool) | Layout | Bodies seen | Conf. |
|---|---|---|---|---|
| 1 | Lossy (type 1) | Huffman, full stored curve | Older DSLRs: D100, D200, D2X, D40, D80, D60 at 12-bit [22] | **[P]** |
| 2 | Uncompressed | packed or unpacked; ExifTool notes the D100 writes this even when the TIFF tag says compressed | D100 and others [16] | **[P]** |
| 3 | Lossless | Huffman, lossless tables, identity curve | D3/D300 generation onward (Clévy: D300, D700, D3X), all Z bodies [22] | **[P]** for the Clévy bodies, debut **[I]** |
| 4 | Lossy (type 2) | Huffman, sparse curve, optional split | D300, D90, D5000 per Clévy; D3, D700 and later bodies [22] | **[P]** for the Clévy bodies, **[S]** otherwise |
| 5 | Striped packed 12 bits | several strips of 12-bit packed data | not established | **[P]** label, **[I]** layout |
| 6 | Uncompressed (reduced to 12 bit) | not established | not established | **[P]** label |
| 7 | Unpacked 12 bits (padded to 16) | 12 bits in 16-bit words | Coolpix P330/P340/P7700/P7800/A1000 | **[P]** label, **[I]** bodies |
| 8 | Small | sNEF YCbCr | D4S, D810 "RAW S" | **[I]** |
| 9 | Packed 12 bits (2 pixels in 3 bytes) | 12-bit packed, little-endian | Coolpix; possibly Z 12-bit uncompressed | **[P]** label, **[I]** bodies |
| 10 | Packed 14 bits (4 pixels in 7 bytes) | 14-bit packed, rows padded | D6 uncompressed [16] **[P]**; Z bodies **[I]** | **[P]**/**[I]** |
| 13 | High Efficiency | JPEG XS based | Z 9 (2022 firmware), Z 8, Z f, Z 6III, Z5 II | **[P]**/**[S]** |
| 14 | High Efficiency\* | JPEG XS based | same | **[P]**/**[S]** |

Values 0, 11 and 12 are not defined. [15] **[P]** ExifTool credits values 5
to 9 to Iliah Borg and names no camera for 5 or 6. [16] **[P]** LibRaw treats
values 7 and 9 as a sign of a fixed-lens body, which suggests they come from
Coolpix compacts [6] **[P]**; whether Z bodies write 9 for their 12-bit
uncompressed files is not established **[I]**. The ExifTool names are also
listed in
[makernote-and-metadata.md](makernote-and-metadata.md#3-nefcompression-and-bit-depth).

**No surveyed decoder dispatches on NEFCompression**, apart from rawler's
refusal of the HE values 13 and 14 (see
[Choosing the decode path](#choosing-the-decode-path)). They select the path
from TIFF Compression, the strip byte count, the 0x0096 header and model
hints; rawler's source says compression flags in some raws are unreliable
because of firmware bugs. [4][11][14] **[P]** Use the value as a cross-check.

## Choosing the decode path

Let W and H be the raw IFD's width and height and B the strip byte count.
The candidate layouts are recognised by size:

| Test | Layout |
|---|---|
| strip starts with `FF 10 FF 50` | HE/HE\* (JPEG XS); see [HE and HE\*](#he-and-he) |
| B = ⌈W/10⌉ · 16 · H | 12-bit packed with a pad byte after every 10 pixels (D100) |
| 2B = 3 · W · H | 12-bit packed, 1.5 bytes per pixel |
| B = 3 · W · H | sNEF, 3 bytes per pixel |
| B = 2 · W · H | one sample per 16-bit word |
| B = ceil16(⌈14W/8⌉) · H | 14-bit packed, rows padded to 16 bytes (D6, Z) |
| several strips, each full strip with 2 · B_strip = 3 · rows · W | striped 12-bit packed |
| otherwise, with Compression 34713 | Huffman |

where ceil16(x) rounds x up to a multiple of 16.

### dcraw and LibRaw

`apply_tiff`, case 34713, tests in this order: [1][4] **[P]**

1. B = ⌈W/10⌉ · 16 · H: the D100 case, compressed-tagged but packed.
2. 2B = 3 · W · H: 12-bit packed. For model names starting with "N", and in
   LibRaw also for the D810 and D4S, special load flags select the
   little-endian nibble order. [1][5] **[P]**/**[S]** The "N" test runs on
   the Model string before `identify()` strips the make, so it matches
   every "NIKON …" body (all DSLRs, the Z bodies and Nikon 1) and excludes
   only "COOLPIX …" and "E…" models. LibRaw's gate in the same function
   likewise compares against the unstripped "NIKON Z 9". [1][4] **[P]** Files
   with TIFF Compression 1 take the big-endian path. [1] **[P]**
3. B = 3 · W · H: sNEF.
4. B = 2 · W · H: 16-bit unpacked, read as big-endian.
5. LibRaw only:
   - the striped 12-bit test, strip by strip: every strip except the last
     must satisfy 2 · B_strip = 3 · rows_per_strip · W; [4] **[P]**
   - rows padded to 16 bytes (the padded 12-bit loader); [3] **[P]**
   - for four Z models only, the JPEG XS sniff (see
     [HE and HE\*](#he-and-he)); the sniff is also missed whenever the raw
     IFD has more than one strip, because the striped branch, or the
     fallback to the Huffman loader, comes before it. [4] **[P]**
6. Otherwise: the Huffman decoder.

LibRaw's `open_datastream` then re-routes some files: a Z body or the D6
whose data size equals the 14-bit packed size goes to the 14-bit packed
loader, and a size of three times that goes to a three-colour "NEFX"
pixel-shift merge. [5] **[S]**

### rawspeed and rawler

rawspeed's `NefDecoder` tests: [11] **[S]**

1. The D100 special case (see [Per-body quirks](#per-body-quirks)).
2. Compression 1, a "force uncompressed" camera hint, or a strip whose size
   matches W · H · bps / 8 with a surplus that divides evenly by H and gives
   less than 16 bytes of padding per row.
3. B divisible by 3 with B / 3 = W · H: sNEF.
4. Otherwise it requires Compression 34713, a single strip and a valid byte
   range, and throws on anything else.

rawler mirrors rawspeed, but sums multi-strip byte counts, rejects files
whose TIFF and MakerNote byte orders differ, and returns an explicit error
for HE/HE\*. [14] **[P]**

### Disagreements

- **Order.** dcraw and LibRaw test the D100 pad layout first and the
  uncompressed sizes after it; rawspeed tries "uncompressed" first. The
  formulas can coincide for some W and H, so the order changes the result
  on such files. **[I]**
- **HE gating.** LibRaw only sniffs for JPEG XS on four models, and only
  for a single-strip raw IFD; on other HE bodies, or with several strips,
  it runs the Huffman decoder. [4][31] **[P]**/**[S]**
- **Strips.** rawler treats several strips as one block; LibRaw seeks to
  each strip's offset. [3][14] **[P]**

Recommendation: compute every candidate with checked arithmetic. If more
than one matches, prefer the one consistent with NEFCompression and record
the ambiguity. Run the HE sniff on every file, before anything else. **[I]**

## The Huffman scheme

### Overview

Compressed NEF is a per-pixel DPCM over the CFA: each sample is predicted
from an earlier sample of the same colour, the difference is coded with a
fixed Huffman alphabet of JPEG-style magnitude categories, and the decoded
value goes through a one-dimensional lookup, the curve. [1][2][10] **[P]**

"Lossy" Nikon compression *is* the curve: the camera first quantises the
ADC values to fewer levels, then codes the quantised indices losslessly.
"Lossless" uses an identity curve. [19] **[S]**

**The Huffman tables are not stored in the file.** Every decoder hard-codes
the same six tables; only the predictor seeds and the curve come from
0x0096. [1][2][10] **[P]** Some community descriptions say otherwise; they
are wrong on this point.

### The six tables

Codes are canonical, assigned in JPEG DHT order from the number of codes of
each length. [10] **[P]** The counts below are for code lengths 1 to 16.
They were checked against dcraw 9.28 and match in LibRaw and rawspeed; they
are structural facts about the code, and the symbol lists are deliberately
omitted. [1][2][10] **[P]**

| Table | Use | Codes of length 1–16 | Codes | Longest |
|---|---|---|---|---|
| 0 | 12-bit lossy | 0,1,5,1,1,1,1,1,1,2,0,0,0,0,0,0 | 14 | 10 |
| 1 | 12-bit lossy, after split | 0,1,5,1,1,1,1,1,1,2,0,0,0,0,0,0 | 14 | 10 |
| 2 | 12-bit lossless | 0,1,4,2,3,1,2,0,0,0,0,0,0,0,0,0 | 13 | 7 |
| 3 | 14-bit lossy | 0,1,4,3,1,1,1,1,1,2,0,0,0,0,0,0 | 15 | 10 |
| 4 | 14-bit lossy, after split | 0,1,5,1,1,1,1,1,1,1,2,0,0,0,0,0 | 15 | 11 |
| 5 | 14-bit lossless | 0,1,4,2,2,3,1,2,0,0,0,0,0,0,0,0 | 15 | 8 |

The last two columns follow from the counts. **[I]** No code is longer than
11 bits. In one source the table 0 symbol list has one entry fewer than its
14 codes, the last symbol being a zero byte from array padding; whoever
transcribes the lists should check that case. **[I]**

The symbol lists are not given here (see
[open-questions.md](open-questions.md)). Tables 0 and 1 share their counts
but not their symbols.

### Table selection

Start from table 0. Add 2 when the 0x0096 `ver0` byte is 0x46 (lossless).
Add 3 when the raw IFD's BitsPerSample is 14. At the split row (see
[Split files](#split-files)) add 1. The split is only read on the lossy
0x44 path, so the lossless tables never get the extra 1; rawspeed asserts
this. [1][2][10] **[P]**

### Symbol byte and difference decoding

Each symbol is one byte: [2] **[P]**

- the low nibble, `len`, is the magnitude category: the number of
  significant bits of the difference;
- the high nibble, `shl`, is a quantisation shift. It is non-zero only in
  the after-split tables.

rawspeed once rejected after-split symbols because their byte value
exceeds 16, which broke D5500 and D500 14-bit lossy files. [20] **[S]**

Decoding one difference, in original pseudocode:

```
s   = next_symbol(table)          # canonical code, read MSB-first
len = s & 0x0F
shl = s >> 4
if len == 0: return 0
if shl > len: error "corrupt symbol"
v = read_bits(len - shl)          # unsigned; zero bits read gives 0
m = ((2*v + 1) << shl) >> 1       # centre of a 2^shl wide bucket
if bit (len - 1) of m is 0:       # negative, JPEG convention
    m = m - (2^len - (1 if shl == 0 else 0))
return m
```

With `shl = 0` the magnitude is just `v` and the sign rule is JPEG's.
[2] **[P]**

- **dcraw has undefined behaviour here.** It does not test `len > 0` before
  shifting by `len - 1`; LibRaw added the guard. A Rust port must guard it.
  [1][2] **[P]**
- **There is no category 16.** JPEG lossless gives category 16 a special
  meaning (−32768, no extra bits). That rule lives in the generic lossless
  JPEG decoders; rawspeed's Nikon path maps it too, but `len` is a four-bit
  field and no Nikon table uses 16. [2][10] **[P]** A decoder that widens
  symbols should reject categories above 15.
- With the fixed tables `shl > len` cannot occur **[I]**; guard it anyway.

### Bitstream

The bitstream starts at the strip offset, is read MSB-first, has no JPEG
`FF 00` byte stuffing, and runs continuously across rows with no per-row
byte alignment. [1][10][14] **[I]** (inferred from the bit readers each
decoder uses). dcraw's bit reader returns zeros past the end of the data
**[I]**; raw-files needs its own documented policy (see
[Decoder robustness checklist](#decoder-robustness-checklist)).

## The 0x0096 block

### Layout

All 16-bit values are in the MakerNote's byte order. [2][10][14] **[P]**

| Offset | Size | Field |
|---|---|---|
| 0 | u8 | `ver0` |
| 1 | u8 | `ver1` |
| 2 | 2110 bytes | **skipped** when `ver0 == 0x49` **or** `ver1 == 0x58`; the fields below then start 2110 bytes later |
| 2 | 4 × u16 | predictor seeds; the order is disputed, see [Disagreements](#disagreements-1) |
| 10 | u16 | `csize`, number of curve values |
| 12 | `csize` × u16 | curve values (only some branches read them) |
| 562 | u16 | `split` row, read only on the sparse lossy path; offset from the start of the block, **not** moved by the 2110-byte skip |

Notes: [1][2][10][14][18]

- **The skip uses OR.** dcraw, rawspeed and rawler all test `ver0 == 0x49`
  or `ver1 == 0x58`. The combination looks odd but is universal; keep it.
  **[P]** No source names a body that writes 0x49 or 0x58 or says what the
  2110 bytes hold; one write-up says its author does not know why. [18]
  **[P]**/**[S]**
- **dcraw's order.** It reads the version bytes, applies the skip, reads
  the four seeds, sets `max = (1 << bps) & 0x7fff`, reads `csize`, and
  computes `step = max / (csize - 1)` only when `csize > 1`. [1] **[P]**
- **The split read.** dcraw reads `split` only when `ver0 == 0x44`,
  `ver1 == 0x20` and `step > 0`. [1] **[P]** rawspeed and rawler also read it
  for `ver1 == 0x40`. [10][14] **[P]** So does LibRaw's `nikon_load_raw`: it
  reads offset 562 for `ver0 == 0x44` with `ver1` of 0x20 or 0x40, without
  dcraw's `step > 0` condition, and divides `max` by 4 for 0x40 regardless
  of `step > 3`. [2] **[P]** The last point conflicts with the `step > 3`
  rule recorded under [The curve](#the-curve); both are observed, and the
  case needs a sample check. **[I]** A sweep of raw.pixls.us samples
  reports a non-zero value at offset 562 on a Z 6II file. [41] **[S]**
- LibRaw reads the curve in `open_datastream`, before unpacking, and the
  Huffman decoder re-reads the header for the seeds and the split. [2][5]
  **[P]**/**[S]**

### Disagreements

- **Order of the four predictor seeds.** dcraw and LibRaw read the four
  shorts into `vpred[row & 1][col]`, so the file order is even row column 0,
  even row column 1, odd row column 0, odd row column 1. rawspeed reads them
  as `pUp[0][0]`, `pUp[1][0]`, `pUp[0][1]`, `pUp[1][1]`, which is
  column-major, and rawler does the same. [1][2][10][14] **[P]** The
  pseudocode under [Predictor loop](#predictor-loop) uses dcraw's reading.
  If the four seeds are always equal the question is moot; reading them from
  sample files settles it. **[I]**

### Version bytes by generation

| `ver0 ver1` | Meaning | Bodies reported | Conf. |
|---|---|---|---|
| 0x44 0x10 | lossy type 1 (NEFCompression 1), full stored curve | D100, D200, D2X, D40, D80, D60, 12-bit [22][41] | **[P]** [22] |
| 0x44 0x20 | lossy type 2 (NEFCompression 4), sparse curve, optional split | D300, D90, D5000 (Clévy) [22] | **[P]** [22] |
| 0x46 0x30 | lossless (NEFCompression 3), identity curve | D300, D700, D3X (Clévy) [22] | **[P]** [22] |
| 0x44 0x40 | lossy "2nd version"; rawspeed and rawler lower bits per sample by 2 (14 to 12, 12 to 10) | D780, D6, Z 5, Z 6, Z 7, Z 7II [14][19][41] | **[S]** (rawler names D780 and Z 7 **[P]**) |
| 0x49 any, or any 0x58 | 2110 bytes skipped before the seeds | none known | **[P]** for the code |

- For 0x44 0x10 the full stored curve matches dcraw's logic: `ver1 = 0x10`
  misses the sparse branch, so the curve is read directly. It also matches
  the `csize` values Clévy read (683 for lossy type 1, 567 for the D100) and
  Claff's per-body sizes (683 for the D200). [1][19][22] **[I]**
- The D1H, D1X, D70 and D2H are not covered by any source. **[I]**
- Read as ASCII, 0x44, 0x46, 0x49 and 0x58 are D, F, I and X. That may be
  coincidence. **[I]**

## The curve

Let `bps` be the raw IFD's BitsPerSample (12 or 14), `max = 1 << bps` and,
when `csize > 1`, `step = max / (csize - 1)` with integer division. The
curve starts as the identity over 0x10000 entries. [1][2] **[P]**

| Branch | Condition | Curve |
|---|---|---|
| A, sparse | `ver0 == 0x44`, `ver1 == 0x20`, `step > 0` | `csize` anchors placed every `step` entries; entries between anchors linearly interpolated |
| B, "2nd version" | `ver0 == 0x44`, `ver1 == 0x40` (condition differs, below) | as A over a domain of `max / 4` |
| C, direct | otherwise, `ver0 != 0x46` and `csize <= 0x4001` | `csize` values read as the curve; `max = csize` |
| D, lossless | `ver0 == 0x46` | identity |

Then trim a flat tail: while `max > 2` and the last two entries below `max`
are equal, decrease `max` by one. [2] **[P]** The interpolation in A and B
weights the two surrounding anchors by their distance and divides by `step`.
[2] **[P]**

**Stored `csize` values read directly** (Clévy, from parsed 0x0096 blocks)
[22] **[P]**:

| `csize` | Where |
|---|---|
| 32 or 34 | lossless |
| 257 | lossy type 2 (sparse anchors) |
| 567 | D100 |
| 683 | lossy type 1 (curve stored in full) |

For 257 anchors, `step` is `max / 256`: 16 at 12 bits and 64 at 14 bits. Both
divide `max` exactly, since 257 − 1 = 256 divides 4096 and 16384, so the
anchors tile `max`. **[I]**

**Encoding-table sizes** on Claff's site are 567, 683, 689, 769, 1025, 2753,
3073 and 4097, and the first quantised level (where values start to be
skipped) lies between 107 and 1601, depending on the body. [19] **[S]** How
these sizes relate to the stored `csize` is unconfirmed: 567 and 683 coincide
with Clévy's values, but the larger ones are not seen in his data. **[I]**

| Encoding-table size | Bodies (Claff) [19] **[S]** |
|---|---|
| 567 | D1H, D100, D1X (lossy type 1) |
| 683 | D70, D50, D40, D60, D80, D3000, D2H, D2X, D200 |
| 689, 2753 | D3, D300, D700 generation, lossy type 2 |
| 769, 3073 | D3S, D3X, D4, D800, D850, Df, D600, D7000, D7100, D500 and others |
| 1025, 4097 | D7200, D750, D810, D5500, D5600; "2nd version" on D780, Z 6, Z 7 |

### Disagreements

- **When branch B applies.** dcraw has no branch B: a 0x44 0x40 file falls
  into branch C there. [1] **[P]**, consequence **[I]**. LibRaw takes branch B
  when `step > 3` and then divides `step` and `max` by 4 [2] **[P]**; its
  `nikon_load_raw` is also read as dividing `max` by 4 for 0x40 without the
  `step > 3` condition (see [The 0x0096 block](#the-0x0096-block)), a
  conflict to settle with samples. **[I]**
  rawspeed and rawler instead reduce bits per sample by 2 before computing
  `max` (rawspeed calls it a Z 7 12/14-bit hack; rawler names the D780 and
  Z 7) [10][14] **[P]**. LibRaw and rawspeed agree when `csize − 1` divides
  `max / 4`. **[I]**
- **Anchor coverage.** rawspeed requires `(csize − 1) · step == max` and
  rejects the file otherwise; LibRaw interpolates past the anchors without
  checking. [2][10] **[P]** The last interpolated segment reads the last
  anchor, which is in bounds only if `(csize − 1) · step ≥ max − 1`.
  **[I]**
- **Branch C bounds.** rawspeed rejects `csize == 0` and `csize > 0x4001`.
  [10] **[P]** Neither dcraw nor LibRaw checks that the block actually holds
  `2 · csize` bytes **[I]**; raw-files must.

Recommendation: follow rawspeed's strictness (anchors must tile `max`; the
`csize` of 257 that Clévy read satisfies it at both bit depths) and take
branch B on the version bytes, as rawspeed and rawler do. **[I]**

## Predictor loop

Original pseudocode for the pixel loop, after the 0x0096 block has been
parsed into seeds, table, curve, `max` and `split`: [1][2][10] **[P]**

```
lo = 0                                    # range-check offset
for row in 0 .. H:
    if split > 0 and row == split:
        table = table + 1; lo = 16; max = max + 32
    p = row mod 2
    for col in 0 .. W:                    # every column, cropped or not
        d = next_difference(table)
        if col < 2:
            vpred[p][col] = vpred[p][col] + d      # wrapping 16-bit
            hpred[col]    = vpred[p][col]
        else:
            hpred[col mod 2] = hpred[col mod 2] + d  # wrapping 16-bit
        v = hpred[col mod 2]
        if (v + lo) as unsigned 16-bit >= max:
            count a range error           # dcraw and LibRaw flag and go on
        out[row][col] = curve[clamp(v as signed 16-bit, 0, limit)]
```

- **What predicts what.** Each sample is predicted from the sample two
  columns to its left, which has the same CFA colour. The first two columns
  of a row continue from the first two columns of the last row of the same
  parity, two rows up. It is not a general "row above" predictor. [2]
  **[P]**
- **Arithmetic.** dcraw uses unsigned 16-bit values that wrap. In Rust use
  `wrapping_add`, or a wider type with explicit range checks; plain `+` on
  `u16` panics in debug builds. **[I]**
- **The clamp limit differs.** LibRaw clamps the curve index to 0x3fff for
  both bit depths, rawspeed to 15 bits, rawler to the effective bit depth.
  [2][10][14] **[P]** All stay in bounds, but they give different pixels on
  corrupt data. Clamp to `min(curve length, 1 << bps) − 1` and count the
  violations. **[I]**
- **Dithering.** rawspeed and rawler can dither the curve output: each
  entry becomes a base and a delta, and a per-row seeded linear
  congruential generator adds a fraction of the delta; rawler seeds it from
  the first 24 bits of the stream. LibRaw and dcraw do not dither. [12][14]
  **[P]** A library that returns sensor data should return the plain lookup.
- **Size limits.** rawspeed requires an even width, W ≤ 8288, H ≤ 5520 and
  bps of 12 or 14. [10] **[P]** These caps are rawspeed policy (the D850's
  8288 × 5520 sits exactly on them); raw-files should bound sizes by the bytes
  present instead. **[I]**

## Split files

Some lossy type 2 files switch to the after-split tables part-way down the
image: [2][20]

- The row is the u16 at offset 562 of 0x0096, and it is signalled nowhere
  else, not in Exif. **[S]**
- After the split, symbols carry `shl > 0`, so differences are coarser, and
  the range check moves (`lo = 16`, `max + 32`). [2] **[P]**
- Reported bodies: the Nikon 1 J5 and D3400 at 12 bits, the D5500 and D500
  at 14 bits, in high-ISO lossy files. [20] **[S]** The trigger is unknown;
  a slow card was suspected and then contradicted. [20] **[S]**
- rawspeed treats `split ≥ H` as no split. [10] **[P]** Switch tables at
  most once.

## Uncompressed and packed layouts

Bit formulas below use bytes `b0, b1, …` in file order. Two 12-bit nibble
orders exist; never assume one without evidence. [1][3][11][14]

| Layout | Pixel formula | Row stride | Bodies | Conf. |
|---|---|---|---|---|
| 12-bit packed, big-endian | `p0 = b0<<4 \| b1>>4`, `p1 = (b1&15)<<8 \| b2` | 1.5 W | old DSLR uncompressed (D1, D1X, D2X, D200 …) | **[S]**/**[P]** |
| 12-bit packed, little-endian | `p0 = b0 \| (b1&15)<<8`, `p1 = b1>>4 \| b2<<4` | 1.5 W, rounded up to 16 | Z 6, Z 7 12-bit uncompressed; other little-endian files | **[P]** |
| 12-bit with pad byte | big-endian as above; a zero pad byte after every 15 data bytes (10 pixels) | 16 per 10 pixels | D100 **[S]**; TIFF Compression 32769 (generic in dcraw) | **[P]** for the layout |
| 14-bit in 16-bit words | one u16 per pixel in the TIFF byte order | 2 W | D3, D300, D800, D810 uncompressed | **[S]**/**[P]** |
| 14-bit packed | 7 bytes as a 56-bit little-endian integer G; pixel n = `(G >> 14n) & 0x3FFF`, n = 0..3 | ceil16(⌈14W/8⌉) | D6; Z uncompressed (NEFCompression 10) | **[P]** |
| 12-bit in 32-bit words | 32-bit little-endian words, 12-bit fields taken from the most significant end | from W | Coolpix NRW: P6000, P7000, P7100, B700, P950, P1000 | **[P]** |
| 12-bit in 16-bit words | one u16 per pixel, 12 significant bits | 2 W | Coolpix P330, P340, P7700, P7800, A1000 | **[P]** |
| interlaced fields | 12-bit big-endian packed; all even rows, then all odd rows | 1.5 W | Coolpix E5000, E5400, E5700, E8400, E8800 | **[P]** |
| sNEF | see [Small NEF](#small-nef-snef) | 3 W | D4S, D810 RAW S | **[P]** |

Notes:

- **Big-endian 12-bit.** rawspeed chooses MSB order when the TIFF header is
  big-endian, unless a camera hint overrides it; rawler has a matching
  big-endian reader. [11][14] **[S]**/**[P]**
- **Little-endian 12-bit.** LibRaw's padded loader computes the stride as
  `ceil16(W · 3 / 2)` and rejects row sizes below 2000 or above 64000 bytes.
  [3] **[P]**/**[S]** dcraw's unpadded branch for "N" models (which
  covers every "NIKON …" body, see [dcraw and LibRaw](#dcraw-and-libraw))
  reads 24-bit little-endian groups and swaps column pairs, which gives the
  same nibble order. [1] **[P]**, equivalence **[I]** RawTherapee added the Z 7 12-bit
  case in 2018. [28] **[S]** rawler takes this path when the TIFF is
  little-endian, with per-row padding below 16 bytes. [14] **[P]** This
  loader is where LibRaw's out-of-bounds read CVE-2026-5342 was, fixed in
  0.22.1. [32] **[P]**
- **Pad byte.** dcraw checks that each pad byte is zero and reports an error
  otherwise. [1] **[P]**
- **14-bit in 16-bit words.** rawspeed recognises it as 14 bps with row
  bytes equal to 2W. [11] **[S]** A Z 7II uncompressed file is about 84 MB
  against 97 MB for a D850, because the D850 does not pack. [29] **[S]**
- **14-bit packed stride.** LibRaw computes the line size with an integer
  division before rounding up, which can under-count for widths not
  divisible by 4. [3] **[S]** RawTherapee first used fixed per-body paddings
  (8 bytes for the Z 7, 12 for the Z 6); they were wrong, and the fix
  derives the stride from the width. [28] **[S]** Use `⌈14W/8⌉` rounded up
  to 16, and cross-check it against B / H. **[I]**
- **Z 9 and later.** rawspeed lists only 14-bit compressed modes for the
  Z 9, Z 8, Z f and Z 6III; they have no uncompressed layout. [13] **[P]**
- **The Z 6/Z 7 TIFF byte order** is not confirmed; the 12-bit packing is
  little-endian whatever the header says. [3][1] **[I]**
- **P7800.** rawler notes the file is little-endian while the data is
  big-endian. [14] **[P]**

### Striped 12-bit packed

NEFCompression 5 is probably this layout **[I]**: several strips
(RowsPerStrip < H), each full strip holding exactly `rows · W · 3 / 2`
bytes. LibRaw selects it purely by byte count, reads 32-bit little-endian
chunks MSB-first, discards per-row padding bits and seeks to each strip's
offset. [3][4] **[P]** rawler sums the strips into one block and says this
is safe for every known model. [14] **[P]** Strips need not be contiguous
in a hostile file, so honour each offset. **[I]**

### dcraw's packed-loader flags

dcraw describes its packed variants with flag bits rather than a format:
skip a byte every 10 pixels; rows interleaved by field; refill units of 16,
24 or 32 bits assembled little-endian; column-index swapping; and per-row
padding bits. Nikon uses the pad-byte flag (D100), the field flags (Coolpix
E), 32-bit words (Coolpix P and B), and 24-bit groups with column swapping
(12-bit little-endian files). [1] **[P]** These flags are a description of
dcraw, not of the format.

## Per-body quirks

- **D100.** Data tagged as compressed (TIFF 34713 or NEFCompression 2) is
  often 12-bit packed with pad bytes. rawspeed reads every 16th byte from
  index 15 over the first 256 bytes; pad bytes are always zero, so any
  non-zero value means the data really is compressed. [11] **[S]** The
  packed path takes the first StripOffsets value of the second IFD that has
  StripOffsets, and a fixed 3040 × 2024 size, because the stored width is
  unreliable [11] **[S]**; rawler forces a width of 3040 for every D100
  [14] **[P]**; dcraw widens D100 files on the packed (pad-byte) path by 3
  columns [1] **[P]**. libopenraw reports
  that most D100 files are packed because compression froze the camera.
  [17] **[S]**
- **D1.** Uncompressed. dcraw scales the red and blue white balance
  multipliers (red by 256/527, blue by 256/317); rawspeed carries a
  matching hint, and LibRaw moves them to its analogue balance instead.
  [1][7][13] **[P]**
- **D1X.** 4024 × 1324 stored, with a pixel aspect ratio of 0.5 in dcraw and
  LibRaw: each stored row covers two output rows. Double the height (or
  halve the width) to get square pixels and a roughly 3:2 image. dcraw trims
  the width by 4. [1][13] **[P]**
- **Coolpix E-series diagnostic raws.** Headerless dumps from the E900 to
  the E5700 era, identified by file size alone, some with four-colour CMYG
  filters, and told apart from other brands' files by byte histograms.
  [1][7] **[P]** They are not NEF; out of scope.
- **Coolpix E5000, E5400, E5700, E8400, E8800 NEF.** Interlaced fields (see
  the table above); the E5000 and E5700 use a CMYG CFA. [1][13] **[P]**
- **Coolpix NRW** (P6000, P7000, P7100, B700, P950, P1000). 12-bit in 32-bit
  words. dcraw and LibRaw set black to 255 on P7000-class bodies at
  ISO 400 and above. [7][13] **[P]**
- **Coolpix P330, P340, P7700, P7800, A1000.** 12 bits in 16-bit words;
  LibRaw sets the maximum to 65504 for the P7700 and P7800. [7][13]
  **[P]** The P7800 has a little-endian file with big-endian data. [14]
  **[P]**
- **Nikon 1.** Standard Huffman NEF; LibRaw's dcraw-derived body rules
  trim 2 rows. [7] **[P]**

## Small NEF (sNEF)

The D4S and D810 "RAW S" files store YCbCr, not CFA data. [21] **[S]**

- **Detection.** B = 3 · W · H: three bytes per pixel. [1][11] **[P]**
  ExifTool names NEFCompression value 8 "Small" [16] **[P]**; that sNEF
  files carry it is **[I]**.
- **Packing.** Every 6 bytes, read as a 48-bit little-endian integer, hold
  four 12-bit samples: sample k is bits 12k to 12k + 11, in the order Y0,
  Y1, Cb, Cr. [2] **[P]**, [21] **[S]**
- **Format.** 4:2:2: one chroma pair per two horizontal pixels; chroma is
  offset by 2048; luma uses about 11 bits (maximum roughly 2392 to 2549,
  depending on white balance). D810 FX sNEF is 2464 × 1640. [21] **[S]**
- **Not scene-linear.** The camera has already applied white balance and a
  gamma-like tone curve. [21] **[S]**

### Conversion: the implementations disagree

| Implementation | Chroma for the odd pixel | YCbCr to RGB | Linearisation | White balance |
|---|---|---|---|---|
| dcraw, LibRaw (older loader) | the same pair as the even pixel | coefficients close to BT.601 (1.3707 for Cr to R) | sRGB-like gamma | divides by the camera multipliers to undo it [2] **[P]** |
| rawspeed | average of this pair and the next | as dcraw | the same gamma, scaled | multiplies R and B by the inverse of 0x000C; rejects implausible values [11] **[S]** |
| rawler | one pair per two pixels | not read in detail | gamma-like curve | inverse white balance [14] **[S]** |
| LibRaw (newer loader, now selected) | averaged with the next pair; can be disabled | BT.601 (1.402 for Cr to R); luma normalised by 2549; chroma forced neutral near the top of the range | an empirical curve | marked as already applied [3][5] **[S]** |
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
  - Neither rawspeed nor LibRaw has D850-specific small-raw code; both gate
    the sNEF path on B = 3 · W · H, so these files go through the Huffman
    decoder. [4][13] **[P]**, consequence **[I]**
  - Not confirmed at the byte level, nor which Huffman table they use
    (presumably the 12-bit lossless one **[I]**).
- **Z 7II and Z 6II** offer M and S NEF, recorded at 12 bits with lossless
  compression. [26] **[P]**
- **Z 8, Z 6III and Z f** record NEF at Large size only. [27] **[P]** The
  Z 9 was not checked; its specification lists M and S only for JPEG and
  HEIF [44] **[S]**, so it is probably Large only **[I]**. A user reports
  shooting M-size NEF on an original Z 7. [24] **[S]**
- **Detecting the size.** ExifTool documents MakerNote 0x003E ImageSizeRAW:
  1 Large, 2 Medium, 3 Small. [15] **[P]**

## HE and HE\*

- **What it is.** Nikon announced intoPIX TicoRAW for the Z 9 in December
  2021, delivered by firmware in 2022. intoPIX calls it patented and gives
  no algorithm detail. [33] **[P]** It is a modified JPEG XS (ISO/IEC 21122)
  with a Bayer decorrelation step. [34][35] **[S]** The JPEG XS second
  edition (2022) added Bayer support, and an amendment defines a CMainBayer
  profile. [36] **[S]**
- **Bodies.** Z 9 (from firmware 2.00), Z 8, Z f, Z 6III, and reportedly the
  Z5 II. [4][31] **[P]**/**[S]** NEFCompression is 13 (HE) or 14 (HE\*), in
  tag 0x0051. [16] **[P]**
- **Sniff.** The strip starts with the JPEG XS SOC and CAP markers,
  `FF 10 FF 50`. [4] **[P]**
- **Reverse-engineered details**, from two unmerged pull requests [34][35]
  **[S]**: a 5/3 wavelet; four components from a Star-Tetrix-style
  decorrelation of the Bayer planes, which the header does not signal; the
  long packet header always; extra sign bits past a short final group;
  chroma scaling with square-root companding; output is 14-bit RGGB. HE and
  HE\* reportedly differ in precinct parameter ranges. Some constants were
  fitted empirically from one scene.

### The model-gating gap in LibRaw

LibRaw's `apply_tiff` sniffs for JPEG XS only when the model matches one of
four names: prefix matches for "NIKON Z 9" and "NIKON Z 8", exact matches
for "NIKON Z f" and "NIKON Z6_3". On a match it calls an HE loader that is a
stub and always throws "unsupported format"; otherwise it calls the Huffman
decoder. [4][8] **[P]** The Z5_2, Z50_2 and ZR are not in the gate. [4]
**[P]** On an ungated HE body such as the Z5 II the result is noise and a
"data corrupted" warning, not an error. [31] **[S]**

LibRaw also misses the sniff on a gated model whenever the raw IFD has more
than one strip: the striped branch or the fallback to the Huffman loader
comes first. [4] **[P]**

The lesson: sniff `FF 10 FF 50` at the strip start of every NEF, whatever
the model and the strip count, before choosing any other path.

### Open-source status (October 2026)

| Project | HE/HE\* status | Conf. |
|---|---|---|
| LibRaw | 0.21 reads the NEFCompression tag for HE; 0.22 (Changelog dated 2025-12-25) supports recent Z bodies with "standard compression only". The maintainers say their own decoder will come in a snapshot in autumn 2026. [9][35] | **[P]**/**[S]** |
| LibRaw PR #826 | open, not merged; routes Z 9/Z 8/Z f/Z 6III strips through the existing sniff to a new loader [35] | **[P]** |
| dnglab (rawler) | returns an error [14] | **[P]** |
| dnglab PR #835 | experimental decoder, opened 2026-08-30, still open; tested on one body (Z 6III); describes itself as largely AI-written; compared against a C reference decoder; the maintainer wants output to match LibRaw's coming decoder [34] | **[P]**/**[S]** |
| rawspeed, darktable | not supported [38] | **[S]** |
| RawTherapee | not supported; Adobe DNG Converter suggested as a workaround [39] | **[S]** |
| Adobe | Camera Raw and DNG Converter decode HE, but not on some ARM platforms, which fits a licensed binary SDK [40] | **[S]** |

### Patents: a legal question

JPEG XS has a royalty-bearing patent pool (Fraunhofer IIS and intoPIX,
administered by Vectis, announced 1 March 2022); rates are not public.
[37] **[S]** intoPIX describes TicoRAW as patented. [33] **[P]** An MIT or
Apache-2.0 crate with a JPEG XS decoder might still need a patent licence
in some jurisdictions; copyright is a separate matter. **[I]** Whether
raw-files may ever decode HE is a legal question, not an engineering one; see
[open-questions.md](open-questions.md) and
[licences-and-references.md](licences-and-references.md).

## Levels, CFA, crop and white balance

### Black level

- **MakerNote 0x003D BlackLevel**, int16u[4], one value per CFA channel, in
  14-bit units: shift right by `14 − bps` for 12-bit files. [1][11][15]
  **[P]** dcraw applies the shift while parsing, which makes the result
  depend on tag order; LibRaw defers it and divides by 4 for 12-bit
  non-Coolpix files, storing a common minimum plus per-channel residuals.
  [1][5][6] **[P]**/**[S]**
- **Channel order: the implementations disagree.** LibRaw reads the four
  values as fixed colours R, G1, G2, B, whatever the CFA layout. [6] **[P]**
  rawspeed stores them by position after setting an RGGB pattern [11]
  **[P]**, and rawler also stores them by position [14] **[P]**. The two
  readings agree on RGGB bodies. **[I]** Every non-RGGB Nikon in rawspeed's
  camera list is a pre-2010 body, so the difference matters only if one of
  them writes 0x003D; which body first wrote it is not known. [13] **[P]**,
  consequence **[I]** Until samples settle it, take LibRaw's reading as the
  reference.
- **Bodies without 0x003D.** (Per-body values on this page are quoted from
  rawspeed's `cameras.xml` for cross-checking against samples, not for
  transcription into code.) rawspeed lists a black level of 0 for the D1
  to D3, D300, D700 and D7000 (the camera clipped or subtracted it), 400
  (14-bit) and 100 (12-bit) for the D850, and 1008 and 251–252 for Z bodies,
  which also write 0x003D. [13] **[P]**
- **Masked pixels.** NEFs are not known to carry usable optical-black
  margins; the per-body width trims in dcraw cut unusable columns, not
  calibrated black. **[I]**
- **Coolpix.** See [Per-body quirks](#per-body-quirks) for the P7000-class
  black of 255.

### White level

- **Nominal**: `(1 << bps) − 1`, so 4095 or 16383.
- **Practical saturation is lower and per body.** rawspeed lists, at 14 and
  12 bits: D3 and D700 15892 / 3972; D300 15236 / 3808; D850 15520 / 3880;
  Z 6 and Z 7 15520 / 3880; Z 8, Z 9, Z f and Z 6III 15892. [13] **[P]**
  These are quoted from rawspeed's `cameras.xml` for cross-checking against
  samples, not for transcription into code.
- LibRaw uses its maximum divided by 1.07 as a linear limit when it knows
  nothing better. [5] **[S]**
- In lossy files the last value of the trimmed curve bounds the output.
  **[I]**
- Derive per-body white levels from the project's own samples rather than
  copying a table (see [open-questions.md](open-questions.md)).

### CFA pattern

- **Sources.** The raw IFD carries TIFF/EP CFARepeatPatternDim (0x828D)
  and CFAPattern (0x828E). dcraw and LibRaw read them; rawspeed ignores the
  tag and uses its camera list; rawler uses the tag only to find the IFD.
  [1][11][14] **[P]**
- **Patterns by body** (rawspeed's camera list) [13] **[P]**:

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
  visible origin and agrees with rawspeed's GRBG. [1][13] **[P]**,
  reconciliation **[I]** Any CFA statement must say whether it is relative
  to the raw origin or to the cropped one, and the pattern must be
  re-phased when a crop starts on an odd row or column.
- Read 0x828E when present, check it against the colour count, and allow
  per-body overrides. **[I]**

### Crop

- **MakerNote 0x0045 CropArea**: int16u[4] = left, top, width, height.
  [15] **[P]** rawler uses it as the crop; LibRaw stores it as an inset
  crop. [14] **[P]**, [6] **[S]**
- **MakerNote 0x001B CropHiSpeed**: int16u[7], the crop mode (off, 1.3x,
  DX, FX uncropped and so on) plus sizes and offsets; the field order is in
  [makernote-and-metadata.md](makernote-and-metadata.md#84-image-dimensions).
  [15] **[P]**
- **Per-body trims.** dcraw and LibRaw trim the width per body (for
  example D3 and D700 by 4 with a left margin of 2, D600 and D800 by 46, D4
  and Df by 52) [1][7] **[P]**; rawspeed has similar crops, and none for
  most Z bodies (Z 9: 12 columns) [13] **[P]**.
- NEF has no equivalent of DNG's DefaultCrop: crop comes from 0x0045 or a
  per-body table.

### White balance

White balance comes from MakerNote 0x000C, then the encrypted 0x0097, then
the Coolpix 0x0014 block; see
[makernote-and-metadata.md](makernote-and-metadata.md#6-white-balance). In
sNEF it is already applied to the data. [21] **[S]** The D1 needs the
scaling described under [Per-body quirks](#per-body-quirks).

## Decoder robustness checklist

Derived from the decoders' own guards and from past vulnerabilities; the
full history is in [security.md](security.md).

1. **Gate HE first.** Sniff `FF 10 FF 50` at every strip start and return a
   typed unsupported error. Never fall through to the Huffman decoder.
2. **Checked size arithmetic.** Compute W · H · bps, ⌈W/10⌉ · 16 · H,
   ceil16(⌈14W/8⌉) · H and the other candidates with checked operations.
   Reject zero or absurd dimensions before allocating, and bound every
   allocation by the bytes present, never by W · H alone.
3. **Strips.** Honour each strip's own offset; check that every strip lies
   inside the file and that `rows · stride ≤ strip bytes`. The padded
   12-bit loader in LibRaw had an out-of-bounds read here (CVE-2026-5342).
   [32] **[P]**
4. **The 0x0096 block, field by field.** Bounds-check the two version
   bytes, the 2110-byte skip, the eight seed bytes, `csize`, the
   `2 · csize` curve bytes and the two bytes at offset 562; a short block is
   common.
5. **The curve.** Reject `csize == 0`; require `csize ≤ 0x4001` in branch C;
   require the anchors to tile `max` in branches A and B; reject a zero
   `step`.
6. **Huffman.** Build the six fixed tables once. Treat `shl > len` as
   corrupt and handle `len == 0` without shifting. Two Huffman-style
   bugs in the LibRaw family, both in Foveon/X3F code and not NEF, show the
   class: CVE-2017-6890 (`foveon_load_camf()` in the separate
   LibRaw-demosaic-pack-GPL2 add-on, before 0.18.2) and CVE-2020-35531
   (`get_huffman_diff()` in `x3f_utils_patched.cpp`, Sigma X3F). [42] **[S]**
7. **End of data.** A bitstream that ends mid-image is either an error or
   zero-filled with a flag; pick one and document it. Never panic.
8. **Predictor and lookup.** Wrapping or checked arithmetic; count range
   violations instead of failing; clamp the curve index to the curve's
   length.
9. **Split.** Ignore `split ≥ H`; switch tables at most once.
10. **sNEF.** Require an even W; define the edge pair.
11. **Byte order.** Read the TIFF structures and the MakerNote each with
    its own byte order.
12. **Test and fuzz.** No lint enforces "never panic" (see
    [ADR008](../../architecture-decisions/adr008-lints-and-panics.md)), so
    truncated and corrupted strips in the tests, and fuzzing of the decoder
    from tiny hand-built strips, carry that guarantee.

## Recommendation for raw-files

For milestone 4: **[I]**

- **Scope.** Decode the Huffman path (lossless, lossy type 1 and type 2,
  including split files), the uncompressed and packed layouts of DSLRs and
  Z bodies, and the Coolpix NRW and 16-bit layouts. Return sNEF as YCbCr
  with a documented flag. Refuse HE/HE\* with a typed error. Leave Coolpix
  diagnostic raws out.
- **Output.** The undemosaiced samples at raw dimensions, plus the crop,
  the CFA pattern stated relative to a named origin, and black and white
  levels. No dithering. This fits the roadmap's open question on how far
  decoding goes ([ROADMAP.md](../../ROADMAP.md#open-questions)).
- **Dispatch.** HE sniff first; then compute every size candidate; resolve
  ties with NEFCompression; use the Huffman path only for Compression 34713
  when no size candidate matches.
- **One parse of 0x0096** into a validated value holding the table index,
  seeds, curve, `max` and `split`, rejected as a whole when any field is
  out of range.
- **Branch B on the version bytes** (0x44 0x40), as rawspeed and rawler do,
  with rawspeed's anchor check.
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
- **D850 M/S**: byte-level confirmation of a CFA layout and which Huffman
  table they use; whether the Z 9 and the original Z 6/Z 7 write M/S NEF.
- **0x003D**: the first body to write it, and its channel order on any
  non-RGGB body.
- **Curve sizes**: how Claff's encoding-table sizes relate to the stored
  `csize` on bodies after 2009.
- **Lossless debut**: whether lossless compression first appeared on the
  D3/D300.
- **Implementation conflicts** to settle with samples: the curve-index
  clamp, the branch B condition, the end-of-data policy, and the sNEF
  conversion.
- **Huffman symbol lists**: their provenance (a project decision).
- **HE/HE\***: whether any work on it is ever in scope (a legal question).

## Sources

All retrieved October 2026. Implementation sources are read only; see
[licences-and-references.md](licences-and-references.md). Entries marked
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
[43] Licences: LibRaw README — https://raw.githubusercontent.com/LibRaw/LibRaw/master/README.md ; dcraw COPYING (Debian) — https://sources.debian.org/src/dcraw/9.28-8/COPYING ; dnglab — https://github.com/dnglab/dnglab ; rawspeed — https://github.com/darktable-org/rawspeed ; rawloader — https://github.com/pedrocr/rawloader
[44] Nikon, Z 9 online manual, specifications — https://onlinemanual.nikonimglib.com/z9/en/specifications_348.html
