# Sony ARW raw sensor data

How Sony's ARW files, and their relatives SR2, SRF and ARQ, store the
sensor data: where the raw image is, the four TIFF Compression values and
how decoders tell the layouts apart, the ARW1 and cRAW lossy schemes, the
packed and uncompressed layouts, the lossless tiles and their YCbCr M/S
variant, the ARW 6.0 codec that raw-files will refuse, the encrypted SRF
data and the four-sample ARQ. It ends with the levels, the checks a decoder
of untrusted input needs and a recommendation for milestone 5.

Back to [the ARW overview](README.md).

## How to read this page

Sony publishes no specification for any of these formats. Everything below
comes from the source code of existing decoders, ExifTool's tag
documentation, community write-ups, Sony's camera help guides, and checks
on sample files.

Every claim carries a confidence tag:

- **[P]** primary: stated in a specification, tag documentation, a camera
  manual or the source code of an implementation.
- **[S]** secondary: a forum post, blog, issue report, review or
  search-result extract.
- **[I]** inferred, or derived by arithmetic from primary facts; a hypothesis
  to test on sample files.
- **[F]** measured on a named sample file. Files are cited by their
  raw.pixls.us id ("file 4466"); the list with URLs and SHA-256 hashes is
  source [40] under [Sources](#sources), and also in
  [test-data.md](test-data.md#files-checked).

Numbers in square brackets such as [3] refer to [Sources](#sources).

**Status.** This is desk research plus structural checks on 36 CC0 sample
files, done in October 2026. Where a sample contradicted the desk research,
the page follows the sample and says so. Several algorithms were *checked*
on samples with throwaway scripts written from the prose descriptions: the
cRAW block layout, ARW1 decoding, the lossless tile headers, the
SR2SubIFD keystream and the SRF key chain. They are not implemented in
raw-files, and the check scripts are not in the repository. **No pixel
output was compared with a reference decoder**, so a check means "the data
decode consistently with the description" (all bytes consumed, values in
range, neighbouring same-colour pixels close), not "bit-exact".

**Offsets, formulas and constants taken from implementation source code must
be checked against that source and against sample files before they are
hard-coded.**

**Licence hygiene.** The decoders read for this page are under dcraw's own
licence, LGPL-2.1 or CDDL-1.0 (LibRaw), LGPL-2.1-or-later (rawspeed) and
LGPL-2.1 (rawler, rawloader); ExifTool is under the Perl terms and Laurent
Clévy's notes under GPL-3.0. See
[licences-and-references.md](licences-and-references.md). None of their
code may be copied or transliterated into raw-files. This page restates
algorithms in prose and original pseudocode only. It gives the ARW1 prefix
code as a rule, not a table, and reproduces no tone curve or delinearisation
table.

## Where the raw data lives

- **The raw IFD.** An ARW is a little-endian TIFF file (every sample
  starts `49 49 2A 00` with IFD0 at offset 8) **[F]**; see
  [container-and-previews.md](container-and-previews.md). In SR2 and from ARW
  2.0 on, IFD0 has exactly one SubIFD (tag 0x014A), and that SubIFD is the raw
  IFD: NewSubfileType 0, PhotometricInterpretation 32803 (CFA) for Bayer data.
  [22] **[P]**, every ARW, SR2 and ARQ sample except the A100 **[F]**
- **Exceptions.**
  - **DSLR-A100 (ARW 1.0).** IFD0 tag 0x014A is not a SubIFD pointer but the
    absolute offset of the raw data; ExifTool calls it A100DataOffset. It
    becomes a real SubIFD only after Sony's Image Data Converter rewrites
    the file. [22] **[P]** In file 2346 it is 98304, and the bytes there do
    not parse as an IFD **[F]**.
  - **SRF (DSC-F828).** IFD0 itself describes the raw image (3360×2460,
    14-bit, StripByteCounts 16,531,200) but has **no StripOffsets**; the
    offset is inside the encrypted MakerNote (see [SRF](#srf-dsc-f828-dsc-v3)).
    File 1351 **[F]**
  - **Previews that look like raw IFDs.** From FileFormat 4.0 on, IFD2 of
    the IFD0 chain holds a full-size JPEG under Compression 7, Photometric 6,
    8-bit, 3 samples, NewSubfileType 1. **[F]** LibRaw skips exactly that
    profile when it picks the raw IFD. [3] **[P]** Raw tiles, conversely,
    begin with an SOI and an SOF3 marker, so a preview finder must reject
    them (see [container-and-previews.md](container-and-previews.md)).
- **How decoders find it.** dcraw and LibRaw take the IFD with the largest
  W·H·bps; rawspeed, rawler and rawloader take the first IFD with a
  StripOffsets tag. When no IFD has one, all three take the A100 path by
  model; otherwise rawspeed takes the SRF path only for a camera with an
  SRF hint, while rawler and rawloader try SRF for any other model.
  [1][3][12][18][19] **[P]**
- **Strips.** Uncompressed, cRAW, ARW1, 12-bit packed and ARW 6.0 data are
  one strip; rawspeed rejects more than one strip under Compression 32767.
  [12] **[P]** Every strip and tile in the samples lies inside its file,
  with 0 to 25,088 bytes after the end of the data. **[F]**
- **Tiles.** Lossless files (Compression 7) use 512×512 TIFF tiles. Their
  raw IFD *also* carries StripOffsets (equal to the first tile's offset)
  and StripByteCounts (equal to the sum of the TileByteCounts), which is why
  the "first IFD with StripOffsets" rule still finds it. Files 4466, 6929,
  6930, 6931, 6736, 7364, 8848 **[F]**
- **TileOffsets is not in ascending order.** The tiles are listed row by
  row, as TIFF requires, but stored in the file *column by column*: in the
  A1 file (17 tiles across) tile 0 is followed by tiles 17, 34, … and only
  then by tile 1. The tiles are contiguous, and each is padded after its
  EOI with 0 to 15 bytes of `FF`, inside its TileByteCount, so that every
  tile starts and ends on a 16-byte boundary (1 to 15 bytes in file 7364).
  Files 4466, 6929, 6930, 6931, 6736, 7364, 8848 **[F]** A
  decoder must use each tile's own offset and never assume that tile i+1
  follows tile i.
- **Tag order.** The raw SubIFD has its tags out of ascending order
  (0x011A, 0x7000, 0x0111, then 0x7316 and 0x74C7 on newer files; ARW 4.0
  adds 0x0102, 0x0142 and 0x7038 out of order). Files 1541, 1542, 3012,
  3251 **[F]** A parser that rejects unsorted IFDs rejects every ARW.

## Compression and the decode path

### TIFF Compression values

TIFF Compression (0x0103) of the raw IFD takes four values across ARW,
SR2 and ARQ files: [3][12][18][21] **[P]**

| Value | ExifTool name | Content | Bodies | Conf. |
|---|---|---|---|---|
| 1 | Uncompressed | one sample per 16-bit word; big-endian words in SR2; four samples per pixel in ARQ | R1 (SR2); A7R II (2015) and later; ARQ | **[P]**, **[F]** |
| 7 | JPEG | lossless JPEG (ITU-T T.81, SOF3) in 512×512 tiles; Bayer, or YCbCr for the M and S sizes | ILCE-1 (2021) and later | **[P]**, **[F]** |
| 32766 | Sony ARW Compressed 2 (the value is shared with a NeXT scheme) | ARW 6.0 wavelet codec | ILCE-7M5 (A7 V), ILCE-7RM6 (A7R VI) | **[P]**, **[F]** for the A7 V |
| 32767 | Sony ARW Compressed | three different layouts, told apart by size: cRAW, 12-bit packed, ARW1 | DSLR-A700 (2007) and later, up to the ILCE-1M2 | **[P]**, **[F]** |

LibRaw renames Compression 7 to 6 internally when it reaches the data
offset (StripOffsets or 0x0201) before BitsPerSample and finds a JPEG
header there; for Make exactly "SONY" with Photometric 32803, or 6 with 3
samples, it keeps the TIFF tile geometry, and otherwise it takes the
dimensions from the JPEG frame. 6 is not a value Sony writes for raw data.
[3] **[P]** Every lossless sample lists 0x0111 before 0x0102, so the
rename happens on them. Files 4466, 6929, 6930, 6736, 7364, 8848 **[F]**
The A100 carries no raw IFD at all and is recognised by model
and IFD width (3872). [1][3] **[P]**

### BitsPerSample is not reliable

BitsPerSample (0x0102) of the raw IFD often does not describe the data.
Measured on cRAW files, whose strips all hold exactly one byte per pixel:
**[F]**

| BitsPerSample | cRAW files |
|---|---|
| 8 | DSLR-A700 (file 2521) |
| 12 | NEX-5 (1457), SLT-A55 (964), DSC-RX100 (5811), SLT-A99 (3751), ILCE-7RM2 (1310), DSC-RX100M6 (3012, 3015), Hasselblad Lunar (4051) |
| 14 | ILCE-7S (1582), ILCE-9 (1541), ILCE-7SM3 (5418), ILCE-7M4 (6936), ILCE-9M3 (7361), ILCE-1M2 (7835) |

Elsewhere: 12 on the A900's 12-bit packed data (5858) and on the A200's
ARW1 stream (2271); 12 on the A7R II's 12-bit uncompressed file (1311),
whose values are nevertheless at 14-bit scale (see
[Uncompressed data](#uncompressed-data)); 14 on lossless Bayer and ARW 6.0
files; and 15,15,15 on the YCbCr M/S tiles, whose lossless JPEG frames
declare 16-bit precision. **[F]**

How decoders cope: [1][3][12][18][20] **[P]**

- dcraw and LibRaw ignore the tag for 32767 and test byte counts.
- rawspeed forces 8 when more than one IFD has a Make tag and any of them
  is exactly "SONY", without the trailing space of the A200, A700 and
  A900 samples; its comment blames a body that marks cRAW as 12.
- rawler forces 8 in the camera file of almost every Sony, except the A300,
  A700, A850, A900 and ZV-E1, which keeps its 12-bit packed path reachable.
- LibRaw repairs a "broken" BitsPerSample above 33 on a one-sample IFD from
  the bit length of the DNG WhiteLevel (0xC61D), or defaults to 14; its
  changelog mentions ILCE-1 files damaged by ExifTool. The link between the
  two is **[I]**.
- rawkit asserts 12 for cRAW, so an A700 file would fail the assertion.
  [20] **[P]**, consequence **[I]**

### SonyRawFileType and RAWFileType

Two tags name the mode, and neither should select the decoder.

- **SonyRawFileType (raw IFD 0x7000).** ExifTool lists 0 uncompressed
  14-bit, 1 uncompressed 12-bit, 2 compressed, 3 lossless, 4 lossless
  compressed RAW 2, 6 compressed RAW 2. [21] **[P]** The samples disagree
  with those meanings: the A7R II 12-bit uncompressed file says 0 (1311);
  every lossless file says 4, never 3, for L, M and S alike (4466, 6929,
  6930, 6931, 6736, 7364, 8848); the A200 says 3 for an ARW1 stream (2271);
  the A7 V's ARW 6.0 files say 6 (8846–8850). **[F]** The R1 already writes
  the tag (file 3221) **[F]**, as Clévy's notes say. [23] **[S]**
- **RAWFileType (MakerNote 0x2029).** 0 compressed, 1 uncompressed, 2
  lossless, 3 "Compressed RAW 2". [22] **[P]** It matches the samples where
  present (1310 0, 1311 and 1542 1, 4466, 6929, 8848 2, 8846 and 8847 3)
  but is absent before the A7R II (A99, RX100, ILCE-7S, Lunar). The A7 V
  writes 3 for both "Compressed" and "Compressed (HQ)"; MakerNote 0x202E
  Quality (4 and 5) tells them apart. **[F]**
- **No surveyed decoder dispatches on either tag.** LibRaw stores 0x7000
  and ignores it. [3][12][18] **[P]** Record both as cross-checks only.

### Choosing the decode path

Let W and H be the raw IFD's width and height and B the strip byte count
(for tiles, the sum of the tile byte counts). The layouts are recognised by
Compression, Photometric, SamplesPerPixel and size:

| Test | Layout | Samples |
|---|---|---|
| no raw IFD; Model DSLR-A100 | ARW1 from the IFD0 0x014A offset, 3881×2608 | 2346 |
| no StripOffsets in IFD0; SRF file | SRF, encrypted (see [SRF](#srf-dsc-f828-dsc-v3)) | 1351 |
| Compression 32766 | ARW 6.0: refuse | 8846–8850 |
| Compression 7, tiled, Photometric 32803, 1 sample | lossless Bayer tiles | 4466, 6929, 6736, 8848 |
| Compression 7, tiled, Photometric 6, 3 samples | lossless YCbCr M/S tiles | 6930, 6931, 7364 |
| Compression 1, 4 samples, B = 8·W·H, Photometric 32892 | ARQ | 3255 |
| Compression 1, B = 2·W·H | 16-bit words: little-endian, or big-endian on the R1 (SR2) | 1311, 1542, 3221, 3251, 6933 |
| Compression 32767, B = W·H | cRAW | 15 files |
| Compression 32767, 2B = 3·W·H | 12-bit packed | 5858 |
| Compression 32767, B = 2·W·H | 16-bit words (LibRaw only; no body known) | none |
| Compression 32767, otherwise | ARW1 with H + 8 rows | 2271 |

Measured bytes per pixel: 2.0000 for Compression 1 (8.0000 for the ARQ),
1.0000 for cRAW, 1.5000 for the A900, 1.0604 for the A200's ARW1 stream and
0.56 to 0.62 for ARW 6.0. **[F]**

### dcraw and LibRaw

dcraw 9.28, `apply_tiff`, where case 32767 falls through to the packed
case: [1] **[P]**

1. The A100 is set up at parse time (model plus width 3872): ARW1 from the
   0x014A offset.
2. 32767 and B = W·H: cRAW, with an internal depth of 12 bits and a maximum
   of 4095.
3. 32767 and 8B ≠ W·H·bps: ARW1, with H increased by 8.
4. 32767 otherwise: the packed loader with 24-bit little-endian refills and
   column pairs swapped, which is 12-bit little-endian packing.
5. Compression 0 or 1, Make SONY, bps < 14 and B = 2·W·H: bps set to 14,
   16-bit words.
6. F828 and V3, by model: the SRF loader at a fixed data offset.

LibRaw (master, October 2026) [3][4][5] **[P]**:

1. 32766 on a non-DNG Sony CFA IFD with 1 sample and bps 12 or 14: ARW 6.0
   (bps set to 14).
2. 32767: B = W·H gives cRAW (bps 14, the output is the full curve value);
   a Sony file with B = 2·W·H gives 16-bit words; 8B ≠ W·H·bps gives ARW1
   (H + 8); otherwise 12-bit packed, as dcraw.
3. Compression 1: a Sony file with B = 2·W·H gives 16-bit words; 4 samples
   and B = 8·W·H give ARQ.
4. Compression 7, renamed to 6 as described above (Make exactly "SONY"):
   YCbCr with bps ≥ 12 and 3 samples goes to the Sony YCC loader
   (three-channel output); otherwise the Sony lossless-JPEG tile loader. A
   7 that was not renamed reaches the Sony tile loader only through a
   second gate (ILCE-1, ILCE-7\* or ILCE-9\* with bps 14 and 1 sample);
   anything else goes to LibRaw's generic `lossless_jpeg_load_raw`.
5. F828 and V3, by model: the SRF loader.

### rawspeed, rawler and rawloader

rawspeed (develop, October 2026) [12] **[P]**:

1. No IFD with StripOffsets: the A100 (by model; 3881×2608, ARW1) or SRF
   (by a camera hint); otherwise an error.
2. Compression 1: 16-bit little-endian words, or big-endian with the R1's
   camera hint.
3. Compression 7: lossless tiles, Photometric 32803 only, so the YCbCr M/S
   files are rejected.
4. Anything but 32767: an error, so ARW 6.0 is unsupported.
5. 32767: exactly one strip; bps in {8, 12, 14}; bps forced to 8 when more than
   one IFD has a Make tag and any of them is exactly "SONY" (the A200, A700 and
   A900 samples write "SONY " with a trailing space, which does not match:
   files 2271, 2521, 5858 **[F]**); W, H > 0, H even, W ≤ 9600, H ≤ 6376;
   8B ≠ W·H·bps gives ARW1 (H + 8); else bps 8 gives cRAW and bps 12 gives
   12-bit packed (with the levels shifted down by 2); bps 14 is an error.

rawler (dnglab main, August 2026) follows rawspeed for 32767, with a per-camera
bps override instead of the Make count; Compression 1 is 16-bit little-endian
except the DSC-R1 (14-bit big-endian); Compression 7 decodes Bayer tiles and
converts YCbCr tiles to RGB. For Compression 32766 it looks the camera up in
mode "arw6", which no Sony camera file defines, so such files fail as
unsupported before decoding starts; its ARW 6.0 decompressor is an
`unimplemented!()` stub that is unreachable today, and `RawLoader::decode` also
catches panics. [18] **[P]** rawloader is the older rawler design: Compression
1 and 32767 only, no lossless, no ARW 6.0, no ARQ. [19] **[P]**

### rawkit

Graphite's rawkit (MIT OR Apache-2.0) chooses its path by model,
Compression and one byte-count test: the A100 by model goes to ARW1;
otherwise Compression 1 goes to 16-bit uncompressed, a strip of exactly
W·H bytes to cRAW (with an assertion that bps is 12), and anything else,
including the A200's ARW1 stream, to a `todo!()`. It has no lossless,
ARW 6.0, SR2, SRF or ARQ path, and it uses `unwrap`, `assert!` and
`todo!()` on file data (the A100 CFA is a `todo!()` too), so it panics on
unexpected input. [20] **[P]** Its ARW1 code
table, cRAW block loop and 0x7010 curve builder match dcraw element for
element **[P]**; that they were translated from dcraw is **[I]**. See
[licences-and-references.md](licences-and-references.md).

### Disagreements

- **Trusting BitsPerSample.** dcraw and LibRaw decide by byte count;
  rawspeed and rawler patch bps and then decide by bps; rawkit asserts it.
  The samples show that the size test is the only safe one. **[F]**
- **16-bit words under 32767.** Only LibRaw accepts B = 2·W·H with
  Compression 32767; rawspeed and rawler would send such a strip to ARW1. No
  sample or source names a body that writes it. [3][12][18] **[P]**
- **Order.** dcraw and LibRaw test sizes in a fixed order; with checked
  arithmetic the candidates W·H, 1.5·W·H and 2·W·H cannot coincide for
  W·H > 0, so order matters only for the "otherwise ARW1" fallback. **[I]**

Recommendation: decide by Compression first, then by exact byte count,
never by BitsPerSample, and treat "32767 with no size match" as ARW1 only
when the data and FileFormat are consistent with it (see
[ARW1](#arw1-dslr-a100-and-dslr-a200)). **[I]**

## ARW1 (DSLR-A100 and DSLR-A200)

### Layout and predictor

- **One bitstream**, read most-significant bit first from the data offset,
  with no byte stuffing. [1][13] **[P]**
- **Visiting order.** Column by column from the rightmost column to the
  leftmost; within a column, rows 0, 2, 4, … first, then rows 1, 3, 5, ….
  [1][13][18] **[P]**
- **Predictor.** One running sum for the whole image, never reset: each
  decoded difference is added to the sum and the sum is the sample. So a
  sample is predicted from the same-colour sample two rows above it, or, at
  the start of a column or of the odd-row pass, from the last sample
  visited. [1][13] **[P]**, interpretation **[I]**
- **Range.** The sum must stay within 0 to 4095. dcraw and LibRaw flag a
  data error and continue; rawspeed throws (and since 2018 also checks for
  negative sums); rawler does not check and wraps into a `u16`.
  [1][2][13][18] **[P]**

Original pseudocode:

```
sum = 0
for col from W-1 down to 0:
    for row in (0, 2, 4, … < H) then (1, 3, 5, … < H):
        len = read_category()
        sum = sum + read_difference(len)
        if sum < 0 or sum > 4095: count a range error
        out[row][col] = sum
```

### The category code

Each difference starts with a prefix code for its magnitude category `len`
(0 to 17). The code is not stored in the file, but it has a simple
structure, which is how rawspeed and rawler decode it without a table:
[13][18] **[P]**

- Read two bits. `11` means category 1, `10` means category 2.
- `01` is followed by one more bit: `1` means category 0, `0` means
  category 3.
- `00` is followed by a run of zero bits ended by a one bit; with z zeros
  in the run, the category is 4 + z. After 13 zeros the run stops without
  a terminating one bit (category 17).

That gives 18 codes of 2 to 15 bits whose code lengths fill the code space
exactly. [1] **[P]**, arithmetic **[I]** The rule is a restatement of the
code's structure; **ARW1 needs no copied table.**

### Difference decoding

After the category come `len` raw bits, read as an unsigned value v, and
then the JPEG sign convention: if `len > 0` and the top bit of v is 0, the
difference is v − (2^len − 1), otherwise it is v. A category of 0 means a
difference of 0 with no extra bits. [1][13] **[P]**

- Categories 13 to 17 cannot occur in valid data, since a difference
  between two 12-bit values needs at most 12 bits. **[I]** The A100 sample
  uses categories 0 to 12 only. File 2346 **[F]**
- dcraw's generic difference routine treats category 16 as −32768 with no
  extra bits, which then trips the range check. [1] **[P]** rawkit's sign
  step differs from dcraw's: it subtracts 2^len + 1, and it turns category
  0 into −2 instead of 0. [20] **[P]** Its effect on real data was not
  tested. **[I]**
- Handle category 0 without computing a shift by −1.

### Dimensions

- **A100.** The PRD segment of the MRW block that 0xC634 points at (the
  A100 has no SR2Private IFD) gives a sensor of 3880×2608 and an image of
  3872×2592. [22] **[P]**, file 2346 **[F]** dcraw and LibRaw widen 3880
  to 3881 columns and reduce the height by one row; rawspeed, rawloader and
  rawkit decode 3881×2608; rawler decodes 3880×2608. [1][4][12][18][19][20]
  **[P]**
- **3881 is right.** Decoding file 2346 from offset 98304 as 3881 columns
  by 2608 rows consumes 10,039,263 of 10,039,268 bytes with no range
  violation, and the mean same-colour differences between neighbours are
  small (about 18 vertically, 21 horizontally). With 3880 columns, 2485
  bytes are left unread. File 2346
  **[F]** Why the encoder writes one extra column is unexplained. **[I]**
- **Crops.** rawspeed then crops 1 column and 2 rows. rawler's camera
  file gives `crop_area` [0, 4, 0, 4], which its code reads as (left, top,
  right, bottom): 4 rows off the top and 4 off the bottom, no columns.
  [16][18] **[P]**
- **IDC-edited files.** dcraw and LibRaw have a second A100 branch (width
  not 3880): trim 4 rows and 4 columns, big-endian, and the packed loader
  with `load_flags` 2, which de-interlaces the rows: the data hold all even
  rows first, then all odd rows. It probably serves files rewritten by
  Sony's Image Data Converter, which turns 0x014A into a SubIFD. [1][4][22]
  **[P]**, link **[I]**

### ARW1 under Compression 32767

Every decoder treats a 32767 strip whose size does not match W·H·bps/8 as
ARW1 and adds 8 rows to H. [1][3][12][18] **[P]** No source named a body
that needs it, and Clévy lists ARW 1.0 for the A100 only [23] **[S]**, but
the DSLR-A200 (FileFormat 3.0.0.0) writes exactly this:

- Raw IFD 3880×2600, BitsPerSample 12, Compression 32767, strip of
  10,697,704 bytes (1.0604 bytes per pixel). File 2271 **[F]**
- Decoding it as ARW1 with W = 3880 and H = 2600 + 8 = 2608 consumes all
  but 5 bytes with no range violation; with H = 2600, 32,738 bytes are left
  over. File 2271 **[F]** This explains the "+8".
- What the 8 extra rows hold, and whether the visible image is the first
  2600 rows, was not checked. **[I]**
- Other 2008–2009 A-mount bodies are probably in the same family: the
  exiv2 metadata dumps published by raw.pixls.us give strip sizes of 0.88
  to 1.32 bytes per pixel for the A230, A290, A300, A330, A350, A380 and
  A390 under Compression 32767 [39] **[P]**, which fits variable-length
  ARW1 data rather than cRAW (exactly 1) or 12-bit packing (exactly 1.5).
  **[I]** The 4600-wide bodies (A290, A350, A380, A390) cannot be cRAW in
  any case, since 4600 is not a multiple of 32. **[I]**

## cRAW (Compression 32767, one byte per pixel)

Sony's lossy "compressed RAW", called ARW2 by decoders: 16 same-colour
pixels in 16 bytes, followed by a tone curve. It has been written since the
A700 (FileFormat 3.0.0.0, BitsPerSample 8, file 2521 **[F]**), although
Clévy's prose dates it to ARW 2.1 [23] **[S]**, and it is still the
"Compressed" mode of bodies up to the A1 II (file 7835 **[F]**). On the
A7 V, "Compressed" is the ARW 6.0 codec instead. [28] **[S]**, files
8846–8850 **[F]**

### Block format

[1][14][18][24][25] **[P]**/**[S]**

- **Rows.** A row of W pixels is exactly W bytes; rows are not padded, and
  each row decodes independently.
- **Spans.** Each 32-pixel span of a row is two 16-byte blocks. The first
  codes the 16 even columns of the span (s, s+2, …, s+30), the second the
  16 odd ones. In a Bayer row each block therefore holds a single colour.
- **Block fields.** Read the 16 bytes as one 128-bit little-endian integer
  and take fields from the least significant end:

  | Bits | Field |
  |---|---|
  | 0–10 | `max`, 11 bits |
  | 11–21 | `min`, 11 bits |
  | 22–25 | `imax`, index (0–15) of the pixel that equals `max` |
  | 26–29 | `imin`, index (0–15) of the pixel that equals `min` |
  | 30–127 | 14 deltas of 7 bits, for the other 14 pixels in order |

- **Step.** The shift `sh` is the smallest k in 0 to 3 for which
  `max − min < 128 << k`, and 4 if there is none; the delta step is
  therefore 1, 2, 4, 8 or 16. RawDigger describes the step as a power of
  two rounded from span/128 [24] **[S]**; the decoders all use the test
  above. [1][14] **[P]**
- **Value.** Each of the 14 pixels is `min + (delta << sh)`, clamped to
  2047. The result is an 11-bit code that indexes the tone curve.

### Pixel loop

Original pseudocode:

```
for row in 0 .. H:
    line = strip[row*W .. row*W + W]
    for s in 0, 32, 64, … while s + 32 <= W:
        for half in 0, 1:                       # even columns, then odd
            b   = u128 little-endian from the next 16 bytes of line
            mx  = bits 0..10 of b;  mn  = bits 11..21
            imx = bits 22..25;      imn = bits 26..29
            if imx == imn: block error           # see the checks below
            sh  = 0
            while sh < 4 and (mx - mn) >= (128 << sh): sh += 1
            next = 30
            for i in 0 .. 16:
                if i == imx:   code = mx
                elif i == imn: code = mn
                else:
                    code = min(mn + ((bits next..next+6 of b) << sh), 2047)
                    next += 7
                out[row][s + half + 2*i] = curve[2 * code]
```

`mx − mn` is computed in signed arithmetic, so a crafted block with
`max < min` gets `sh = 0`, as in dcraw. [1] **[P]**

### The 0x7010 tone curve

- **The tag.** SonyToneCurve (0x7010) in the raw IFD holds four int16u
  values. Each is shifted right by 2 and masked to 12 bits to give the
  knots k1 to k4; k0 = 0 and k5 = 4095 are implied. ExifTool notes that
  only the middle four points are stored. [1][21] **[P]**
- **The curve** has 4096 entries. Between knots k_i and k_(i+1), for i = 0
  to 4, it rises by 2^i per entry, starting from 0: [1][12][18] **[P]**

  ```
  k = [0, t1, t2, t3, t4, 4095]      # t_n = (tag value n >> 2) & 0xFFF
  curve[0] = 0
  for i in 0 .. 5:                   # slopes 1, 2, 4, 8, 16
      for j in k[i] + 1 ..= k[i+1]:
          curve[j] = curve[j-1] + (1 << i)
  ```

  Dietz describes the same map from the encoder side: a five-segment
  linear map from 14 bits to 11, whose step doubles in each segment. [25]
  **[S]**
- **Lookup at index 2·code.** The 11-bit code is doubled before the
  lookup, so only even entries are used and the largest index is 4094.
  [1][14][18] **[P]**
- **Measured knots.** 8000, 10400, 12900, 14100 on every cRAW file except the
  NEX-5, which stores 0, 9824, 11512, 16362 (file 1457), and on every pre-4.0
  ARW with other data (ARW1, 12-bit packed, uncompressed: files 2271, 5858,
  1311, 1542, 3251). The R1's SR2 has no 0x7010 (file 3221). 0, 0, 0, 0 on
  4.0-and-later uncompressed (6933), lossless and ARW 6.0 files, where the tag
  is not used. **[F]**
- **The curve exceeds 14 bits.** With the common knots (2000, 2600, 3225,
  3525 after the shift), the top code maps to
  curve[4094] = 2000 + 2·600 + 4·625 + 8·300 + 16·569 = 17204, above 16383.
  **[I]**, checked with a curve builder on the samples **[F]** With the NEX-5
  knots the top value is 16360. **[I]** rawspeed's white levels for
  cRAW-era bodies range from 16000 to 16628 (16300 is the most common
  value, 16596 the next); the entries of 16500 and above sit above 14-bit
  full scale too.
  [16] **[P]** No sample reached code 2047: decoded codes span 260 to 2020
  in the blocks checked. **[F]**
- **Worst case.** With all four knots 0 the top entry is 16 · 4095 =
  65520, which still fits in 16 bits. **[I]**
- **Non-monotonic knots.** No decoder checks that the knots increase; a
  decreasing knot empties a segment and leaves whatever the table held
  before (dcraw initialises it as the identity). [1][12] **[P]**,
  consequence **[I]**
- **Tag absent.** dcraw and LibRaw keep their identity curve; rawspeed and
  rawler fail, because the tag is required. There is no documented Sony
  default. [1][2][12][18] **[P]**

### Output scale and dithering

| Decoder | Value written |
|---|---|
| dcraw | `curve[2·code] >> 2`, a 12-bit scale with maximum 4095 |
| rawkit | `(curve[2·code] >> 2) << 2`: dcraw's value shifted back to 14-bit scale with the low two bits cleared, declared maximum 16383 |
| LibRaw | `curve[2·code]` |
| rawspeed | `curve[2·code]`, optionally dithered with a per-row seed from the row's first 24 bits |
| rawler | `curve[2·code]`, always dithered |

[1][2][14][18][20] **[P]** The black and white levels in the file are on
the scale of the undithered `curve[2·code]` (see
[Black level](#black-level)). **[I]**

### Checks on the samples

All 15 cRAW files were checked, over 500,000 blocks in total (a sample of
about 100 rows from each file). **[F]**

- No block has `max < min`, and no block has `imax == imin`.
- Same-colour jumps across block boundaries average 8 to 57 code units,
  which is what a correct field layout gives (measured on 13 of the 15
  files).
- 0.02 to 0.13 % of the reconstructed pixels exceed their block's `max`
  (for example 314 of 381,276 in file 2521; measured on 13 of the 15
  files). So the decoders' clamp is to 2047 only; clamping to the block's
  `max` would change real data.
- Every cRAW width is a multiple of 32: 2816, 4288, 4608, 4736, 4928,
  5504, 5664, 6048, 8000.

### Disagreements

- **`imax == imin`.** rawspeed rejects the block, since 14 deltas cannot
  cover 15 pixels. dcraw and LibRaw store `max` there and read a 15th delta
  at bit 128, past the block; on the row's last block that reads past
  dcraw's row buffer. [1][14] **[P]**, overrun **[I]** No sample block has
  it **[F]**, so rejecting it costs nothing on real files.
- **Widths not a multiple of 32.** dcraw and LibRaw loop while
  `col < W − 30`, leaving the last columns undecoded; rawspeed rejects the
  file; rawler ignores the remainder. [1][14][18] **[P]**
- **Output scale and dithering**, as in the table above.
- **Step rule.** Decoders use "smallest power with span < 128·2^k";
  RawDigger's description rounds. [1][14][24] **[P]**/**[S]**

### Patent

The cRAW block is the first embodiment of Sony patent US 8,023,748 B2
(application US 2007/0223824 A1, priority JP 2006-050170, 2006): 16
same-colour pixels, the maximum and minimum with their positions, and 14
differences quantised to 7 bits, 256 bits down to 128. Google Patents lists
it as "Expired – Fee Related"; a related patent, US 8,013,910 B2, is listed
the same way. [32] **[S]** A patent on an encoder may not read on a decoder,
and an expired patent cannot be enforced, so a cRAW decoder looks
unencumbered. **[I]** This identifies a question; it is not legal advice.

### Known artefacts

These are facts about the data, not decoder bugs: errors concentrate where
a high-contrast edge shares a 16-pixel block with a flat area, because the
large span forces a coarse step on all 14 deltas; RawDigger and Lloyd
Chambers showed posterization and 32-pixel line artefacts around star
trails in 2014. [24][25] **[S]** Dietz's KARWY tool models each pixel's
error range to repair them. [25] **[S]** A library that returns sensor
data should return the plain lookup.

## 12-bit packed

- **Layout.** ARW 2.0 and 2.1 bodies offered an uncompressed "RAW" mode in
  the same Compression 32767 container: B = 1.5·W·H, two pixels in three
  bytes, little-endian:
  `p0 = b0 | (b1 & 15) << 8`, `p1 = b1 >> 4 | b2 << 4`. [1][12][18] **[P]**
  (formula derived from the decoders' bit readers **[I]**)
- **Bodies.** A700, A850 and A900 per the decoders [12][18] **[P]**; the
  A900 sample is exactly 1.5000 bytes per pixel (6080×4048, file 5858).
  **[F]** The A850 sample on raw.pixls.us is not CC0 and was not
  downloaded; its published exiv2 dump also gives exactly 1.5. [39] **[P]**
- **Nibble order checked.** With the little-endian formula, same-colour
  neighbours in file 5858 differ by 16.7 on average and the values range
  from 134 to 1592; the big-endian alternative gives 167 and 0 to 4095.
  **[F]**
- **Levels.** The data are at 12-bit scale, so the 14-bit-scale black and
  white levels must be divided by 4: file 5858's black 0x7310 is 512, and
  its data minimum is 134, about 512/4. **[F]** rawspeed and rawler shift
  the levels right by 2 for this mode; LibRaw divides black by 4 when it
  exceeds 256 on packed Sony data. [5][12][18] **[P]**

## Uncompressed data

- **14-bit in 16-bit words.** Compression 1, B = 2·W·H, little-endian
  words, no row padding, values at most 16383. [1][2][12] **[P]** Checked
  on the ILCE-9, ILCE-7RM3 and ILCE-7M4 (APS-C) files: values ≤ 16383,
  all four values of the low two bits present. Files 1542, 3251, 6933
  **[F]** Offered since the A7R II (2015). [23] **[S]**
- **12-bit uncompressed is stored at 14-bit scale.** The A7R II's 12-bit
  uncompressed file (BitsPerSample 12, Compression 1, B = 2·W·H) has every
  value a multiple of 4 (the low two bits are zero over 200 rows), a maximum
  of 16376 and a minimum near 540. So its 0x7310 black of 512 applies
  unscaled. File 1311 **[F]** The decoders' source leaves this open; the
  sample agrees with dcraw's rule of setting bps to 14 for any Sony
  B = 2·W·H file. [1] **[P]** Do not divide the levels by 4 for
  Compression 1; divide only for 12-bit packed data and for ARW1.
- **SR2** is the big-endian variant; see [SR2](#sr2-dsc-r1).

## Lossless compressed (Compression 7)

### Bayer tiles

- **Raw IFD.** Photometric 32803, SamplesPerPixel 1, BitsPerSample 14,
  TileWidth and TileLength 512. [3][12][18] **[P]**, **[F]**
- **Each tile is a complete lossless JPEG** (SOI, SOF3, DHT, SOS, …, EOI,
  in that order in every tile of the seven lossless samples **[F]**)
  whose frame is **256 wide by 256 high with 4 components**, precision 14,
  predictor (Ss) 1, point transform 0 and no restart interval. Checked on
  the first and last tiles of files 4466, 6929, 6736 and 8848 **[F]**;
  rawspeed's tiled decoder accepts only predictor 1 [15] **[P]**.
- **Component mapping.** One JPEG sample position is one 2×2 CFA quad:
  component 0 goes to (2y, 2x), 1 to (2y, 2x+1), 2 to (2y+1, 2x) and 3 to
  (2y+1, 2x+1). LibRaw, rawspeed and rawler agree. [8][12][18] **[P]** With
  predictor 1 each component predicts from the same component one quad to
  the left, so from the same-colour pixel two columns left; T.81's usual
  rules for the first line and the first column presumably apply. **[I]**
- **The Huffman tables are in each tile** (DHT), so nothing has to be
  brought from outside. [8][12] **[P]**
- **Tile count.** ⌈W/512⌉ · ⌈H/512⌉ in every lossless sample: 204 (17×12,
  A1), 140 (A7 IV), 117 (A6700), 70 (A7 V APS-C). **[F]** rawspeed and
  rawler check it; LibRaw does not. [12][18] **[P]**
- **Storage order and padding**: column-major, with `FF` padding after
  each EOI; see [Where the raw data lives](#where-the-raw-data-lives).
  File 4466 **[F]**
- **First appearance.** The ILCE-1 wrote lossless Bayer files from
  firmware 1.00 (file 4466, Software "ILCE-1 v1.00") **[F]**; LibRaw added
  support in February 2021 [11] **[P]**. Sony's announcement that firmware
  1.3 "adds" lossless S, M and L [27] **[S]** therefore concerns at least
  the M and S sizes. **[I]**

### Real image size

ImageWidth and ImageLength are rounded up to whole tiles; the usable size
is in raw IFD tag 0x7038 SonyRawImageSize, and the visible area in
DefaultCropOrigin/Size. [21] **[P]**

| Body | Raw IFD | 0x7038 | DefaultCrop | File |
|---|---|---|---|---|
| ILCE-1 | 8704×6144 | 8664×5784 | (12, 12) 8640×5760 | 4466 **[F]** |
| ILCE-7M4 | 7168×5120 | 7032×4688 | (12, 8) 7008×4672 | 6929 **[F]** |

LibRaw's per-body table gives 8660×5784 for the A1 and 7028×4688 for the
A7 IV, a few columns narrower than 0x7038. [4] **[P]** rawspeed crops to
0x7038 after decoding, rawler uses it as the active area, and LibRaw
ignores it in favour of its table. [4][12][18] **[P]**

### M and S sizes: YCbCr tiles

- **What they are.** Sony's help guides offer "Lossless Comp (L)", "(M)"
  and "(S)" and say M and S reduce the image size. [27] **[P]** The data
  are not Bayer but YCbCr, already demosaiced and white-balanced:
  Photometric 6, SamplesPerPixel 3, lossless JPEG tiles. [9][18] **[P]**
- **Measured.** Tiles are 512×512 and each is one SOF3 frame of 512×512
  with 3 components and **precision 16**, although BitsPerSample says
  15,15,15. **[F]**

  | Size | YCbCrSubSampling | SOF3 sampling | Raw IFD | Tiles | File |
  |---|---|---|---|---|---|
  | M (ILCE-7M4) | 2, 2 (4:2:0) | 2×2, 1×1, 1×1 | 5120×3584 | 70 | 6930 |
  | S (ILCE-7M4) | 2, 1 (4:2:2) | 2×1, 1×1, 1×1 | 3584×2560 | 35 | 6931 |
  | S (ILCE-9M3) | 2, 1 (4:2:2) | 2×1, 1×1, 1×1 | 3072×2048 | 24 | 7364 |

- **Other tags.** The M/S raw IFDs carry a CFAPattern of 255, 255, 255,
  255, so a decoder that finds the raw IFD by the presence of CFAPattern
  must still check Photometric. They have no DNG WhiteLevel (0xC61D), but
  0x7310 (512) and the SR2SubIFD's 0x787F are present. **[F]** MakerNote
  0x202E Quality says 3 on the M file but also on APS-C Bayer files, so it
  does not identify YCbCr data. Files 6930, 6933, 6936, 7835 **[F]**
- **Decoders.**
  - rawspeed rejects them (Photometric must be 32803). [12] **[P]**
  - rawler converts them to linear RGB, doubles the black level for YCbCr
    data and treats white balance as applied; its maintainer had to adapt
    the 4:2:0 prediction order, which differs from the generic lossless
    JPEG decoder's. [18] **[P]**/**[S]**
  - LibRaw's Sony YCC loader credits dnglab; it marks the result as
    white-balanced, sets black 0 and an *estimated* maximum (17536, or
    18091 without RGB conversion), and converts with BT.601-style
    coefficients around a chroma midpoint of 16383. [5][9] **[P]**
  - Together these suggest the luma is on a scale above 14 bits, which
    the 16-bit precision supports. **[I]**
- **No ground truth.** Treat M/S like Nikon's sNEF
  ([../nef/raw-data.md](../nef/raw-data.md)): a three-channel, non-CFA,
  already white-balanced image, with the raw YCbCr samples available and
  any RGB conversion documented as an approximation. **[I]**

## ARW 6.0 (Compression 32766)

**raw-files will detect and refuse these files** with a typed unsupported
error for milestone 5, until two questions are decided: the provenance of
the 4096-entry curve that the file does not store, and the patent position.
This is the same posture as Nikon's HE/HE\* (see
[../nef/raw-data.md](../nef/raw-data.md)). Previews from ARW 6.0 files are
unaffected.

### What is known

- **Bodies and modes.** The A7 V (ILCE-7M5) and A7R VI (ILCE-7RM6) use it
  for "Compressed" and "Compressed (HQ)"; their lossless mode stays
  Compression 7. [10][17][18] **[P]**/**[S]** The A7 V's help guide offers
  no uncompressed mode. [27] **[P]** On the A7 V samples, both compressed
  modes have Compression 32766, SonyRawFileType 6 and RAWFileType 3, and
  differ only in MakerNote Quality (4 standard, 5 HQ); the HQ files are
  the smaller ones in both pairs. Files 8846–8850 **[F]**
- **Raw IFD.** One strip, BitsPerSample 14, 0.56 to 0.62 bytes per pixel;
  heights are multiples of 16 (4688, 3088). 0x7010 is 0, 0, 0, 0; 0x7310 is
  512; 0xC61D is 16383. Files 8846–8850 **[F]** A guess of 19501 (half of
  39002) for 0xC61D does not hold.
- **Strip header** (little-endian). A u32 tile count, then from byte 8 one
  24-byte record per tile: an offset (LibRaw reads a u32 with 4 bytes
  following; the reverse-engineering write-up reads a u64), then x, y,
  width and height. [10][30] **[P]**/**[S]** On the A7 V full-frame files
  the count is **1** and the record is (offset 512, x 0, y 0, 7040×4688);
  the APS-C files are also one tile. Files 8846, 8847, 8849 **[F]**
  Descriptions of four tiles on full-frame files (5024 + 4992 wide)
  [10][30] **[S]** do not fit the A7 V; they may describe the A7R VI, which
  reportedly uses 2×2 tiles [17] **[S]** and has no sample on raw.pixls.us.
- **Tile stream.** Starts at the record's offset (0x200 here) with a
  0x80-byte big-endian header: a four-character tag (`A000` on the A7 V;
  LibRaw also accepts `0000`), the coded width at +8 (7040, a multiple of
  16) and half the height at +0xA (2344 = 4688/2; 1544 on APS-C).
  [10] **[P]**, files 8846, 8849 **[F]** Other header bytes are not
  understood. **[I]**
- **Transform and coding**, as described by LibRaw's decoder and an
  independent write-up [10][30] **[P]**/**[S]**: each CFA quad is split
  into a green low-pass and a green high-pass band and two colour residuals
  against the greens; green low-pass and the residuals go through a
  three-level reversible 5/3 lifting wavelet; coefficients are entropy
  coded in groups with an adaptive Golomb–Rice-like width; each 16-row
  chunk carries 4-bit quantiser shifts, and quantisation is the only lossy
  step.
- **Output.** Signed 12-bit codes, offset by 2048 and clipped to 0 to 4095,
  go through a fixed **4096-entry delinearisation curve that is not in the
  file**. [10] **[P]** LibRaw embeds it as a table; the write-up ships one
  extracted by running files through an official decoder. [10][30]
  **[P]**/**[S]** The curve is the identity up to code 1427, then
  steepens, ending at 39002 (shape read from LibRaw's table, not copied)
  **[I]**; how it was constructed is unknown [30] **[S]**.
- **Levels.** LibRaw hard-codes black 1024, maximum 39002 and linear
  maximum 32800 [5] **[P]**; rawspeed's closed pull request used 1024 and
  32800 [17] **[S]**. The file's 512 and 16383 doubled are 1024 and 32766,
  close to those, which fits the write-up's remark that the stored levels
  are "halved". [30] **[S]**, files 8846–8850 **[F]**, reconciliation
  **[I]**

### Open-source status (October 2026)

| Project | ARW 6.0 status | Conf. |
|---|---|---|
| LibRaw | decoder on master since PR #824 (merged 2026-07-18), not in release 0.22.2; the file header says the code was generated with AI assistance [10][11] | **[P]** |
| rawspeed | PR #972 closed unmerged 2026-09-13; it claimed a byte-exact match with Adobe's output; Compression 32766 throws [12][17] | **[P]** |
| rawler (dnglab) | refused as unsupported (no camera file defines mode "arw6"); the decompressor behind it is an unreachable `unimplemented!()` stub; an AI-assisted port in PR #825 is open [18] | **[P]** |
| arw6_decode | independent Python decoder and format write-up, AI-assisted, **no licence file** [30] | **[P]** |
| Adobe | Camera Raw and Lightroom reportedly decode A7 V files from about April 2026 [35] | **[S]** |

### Patents: a legal question

LibRaw names the stream "llvc3", and the write-up calls the format "LLVC
v3". [10][30] **[P]** LLVC is the name of a Sony line-based wavelet video
codec published as SMPTE RDD 34:2015, which uses a 5/3 lifting filter,
16-line units and 128-bit words, and which directs intellectual-property
inquiries to Sony. [31] **[P]** That ARW 6.0 descends from LLVC is a strong
naming and structural match but is not confirmed **[I]**. If the codec also
uses JPEG XS techniques, the JPEG XS patent pool may be relevant, as it is
for Nikon HE. [33] **[S]**, relevance **[I]** No patent on ARW 6.0 itself
was identified. Whether raw-files may ever decode it is a legal question;
see [open-questions.md](open-questions.md). This is not legal advice.

## SR2 (DSC-R1)

- **Container.** FileFormat 1.0.0.0, SonyModelID 2. The raw IFD is the
  SubIFD of IFD0: Compression 1, BitsPerSample 14, 3984×2608, one strip of
  2·W·H bytes that ends at the end of the file. File 3221 **[F]**
- **The data are not encrypted.** They are 14-bit values in **big-endian**
  16-bit words inside a little-endian file: read big-endian, every value is
  below 16384 (480 to 16368); read little-endian, only a quarter are.
  File 3221 **[F]** dcraw and LibRaw switch byte order by raw width
  (3984), rawspeed by a camera hint and rawler by model. [1][4][12][18]
  **[P]** Decide it from FileFormat 1.0.0.0, and cross-check with the
  value range. **[I]**
- **What is encrypted** in SR2, as in every later ARW, is the SR2SubIFD
  holding the levels and white balance; see
  [The SR2SubIFD](#the-sr2subifd-where-the-levels-are).
- **Levels and CFA.** Black comes from SR2SubIFD tag 0x7300 (514, 511, 512,
  513); there is no 0x787F white level; the CFA is GRBG (CFAPattern 1, 0,
  2, 1). File 3221 **[F]** dcraw trims the width to 3925. [1] **[P]**

## SRF (DSC-F828, DSC-V3)

SRF is the 2003–2004 format of the DSC-F828 and DSC-V3. Unlike SR2, **the
image data are encrypted**, with the same keystream as the SR2SubIFD.
[1][22] **[P]**

- **Structure.** The MakerNote is a chain of IFDs, SRF0 to SRF6. SRF0 is
  plain; SRF1 is encrypted and holds tag 0 (SRF2Key) and tag 1 (DataKey);
  SRF2 holds the raw data offset and length (tags 4 and 5); SRF6 holds
  standard Exif. [22] **[P]**
- **The key chain**, checked on the F828 sample: [22] **[P]**, file 1351
  **[F]**
  1. Read the byte b at MakerNote start + 0x8DDC (164,580 + 0x8DDC =
     200,896 in the sample, where b = 109). ExifTool's comment on the
     constant asks "why?".
  2. The master key is the big-endian u32 at that position + 4b (201,332,
     value 0xABEF9C88).
  3. Decrypt SRF1 (at 164,600) with the master key: it becomes a two-entry
     IFD.
  4. DataKey is the little-endian u32 at byte 22 of the decrypted SRF1, the
     value field of its second entry (0x6ACE748A).
  5. Decrypt the image with DataKey as one continuous keystream from its
     first byte (dcraw decrypts row by row but seeds only once). The
     result is big-endian 16-bit words.
- **The image.** At 862,144, exactly the end of the MakerNote, 16,531,200
  bytes long (3360 · 2460 · 2), running to the end of the file. Decrypted,
  every value is below 2^14 (up to 16368); undecrypted, only a quarter
  are. File 1351 **[F]** So the data are 14-bit, as BitsPerSample, dcraw's
  check and rawspeed's white level say, not 12-bit as in Clévy's prose.
  [1][16][23] **[P]**/**[S]**
- **SRF2 to SRF6 were not located.** The bytes at SRF1's next pointer did
  not parse as an IFD, plain or decrypted with any key in the chain, so the
  offset and length above are dcraw's constants confirmed by arithmetic,
  not read from SRF2. File 1351 **[F]** dcraw, LibRaw, rawspeed and rawler
  all hard-code the offsets (the V3 data at 787,392 in dcraw); ExifTool
  walks the structure instead. [1][4][12][18][22] **[P]**
- **CFA.** dcraw and LibRaw describe the F828 as four-colour "RGBE"
  (E for emerald), R E / G B; rawspeed lists the fourth colour as yellow
  in the pattern Y R / B G, which is dcraw's shifted one column and fits
  rawspeed's crop at x = 5. [1][4][16] **[P]**, reconciliation **[I]** The
  file has no CFA tags, and the samples were not decoded far enough to
  confirm the pattern. **[F]** LibRaw does not apply the camera white
  balance by default, because it is given for RGB while the data are RGBE.
  [5][11] **[P]**
- **Geometry.** dcraw and LibRaw use a width of 3288 with a left margin of
  5; rawspeed crops x = 5 and 67 columns less, and caps SRF at 3360×2460.
  [1][12][16] **[P]**

## ARQ (pixel shift)

- **Origin.** ARQ files are made on a computer by Sony's desktop software
  (Imaging Edge, "Pixel Shift Multi Shooting") from 4 or 16 ARW frames,
  not by the camera; the source frames must be uncompressed or lossless L.
  4-shot since the A7R III, 16-shot from the A7R IV. [34] **[P]**, [23]
  **[S]**
- **Container.** The ARQ is the source ARW's container with the raw
  replaced: the previews, MakerNote and SR2Private keep the same offsets
  and lengths as the pixel-shift frame, and Software still names the
  camera. Files 3255 and 3251 **[F]**
- **Raw IFD.** Compression 1, **SamplesPerPixel 4**, BitsPerSample 14,
  **Photometric 32892**, 8000×5320, B = 340,480,000 = 8·W·H; values at most
  16383. File 3255 **[F]** There is no CFAPattern (only 0x828D and
  CFAPlaneColor 0xC616). **[F]**
- **Channel order** R, G, G2, B: the per-channel means above black are
  706, 1552, 1552 and 795; the two greens differ from each other by 26 on
  average, red and blue by 118. File 3255 **[F]** LibRaw's changelog says
  the same and its loader reorders the last two channels to its own R, G,
  B, G. [7][11] **[P]**
- **PixelShiftInfo** (MakerNote 0x202F) ends in 0, 4 on the composite and
  1, 4 on the source frame. Files 3255, 3251 **[F]**
- **16-shot files** double the width and height; LibRaw crops 32 columns
  from 4-shot files and 64 from 16-shot files (raw width above 12000).
  [5][7] **[P]** A 16-shot file is about 2 GB. [34] **[P]**
- **Decoders.** LibRaw reads ARQ; rawspeed, rawler and rawloader have no
  ARQ path. [7][12][18][19] **[P]**

## The SR2SubIFD: where the levels are

Black level, white level, white balance and the colour matrix of every SR2
and ARW 2.0-and-later file live in an encrypted IFD, the SR2SubIFD. A
decoder needs it even when the image data are plain. The structure is also
described in
[makernote-and-metadata.md](makernote-and-metadata.md); this section gives
what a raw decoder needs.

- **Path.** IFD0 tag 0xC634 (DNGPrivateData, written as 4 bytes) holds a
  little-endian offset to the SR2Private IFD. Its tags give the SR2SubIFD's
  offset (0x7200), length (0x7201) and key (0x7221, 4 bytes, read as a
  little-endian u32). [1][12][18][22] **[P]** On the A100, 0xC634 points to
  a Minolta MRW block instead. [22] **[P]**, file 2346 **[F]**
- **The key is a constant.** 0x7221 is the bytes `11 22 33 44` on every ARW
  sampled, from the A200 to the A7 V; only the R1 has another key. **[F]**
- **The keystream.** A 32-bit key seeds a lagged-Fibonacci XOR generator
  through a linear congruential step. Original pseudocode: [1][12][18][22]
  **[P]**

  ```
  seed(K):                                   # all arithmetic on u32
      for i in 0 .. 4:
          K = K * 48828125 + 1               # wrapping; 48828125 = 5^11
          x[i] = K
      x[3] = (x[3] << 1) | ((x[0] ^ x[2]) >> 31)
      for i in 4 .. 127:
          x[i] = ((x[i-4] ^ x[i-2]) << 1) | ((x[i-3] ^ x[i-1]) >> 31)
      for m >= 127:
          x[m] = x[m-127] ^ x[m-63]
  keystream word n (n = 0, 1, …) is x[127 + n]
  decrypt: XOR data word n, read as big-endian u32, with keystream word n;
           write it back big-endian; leave a tail of 1 to 3 bytes as it is
  ```

  The cipher is its own inverse. LibRaw computed the multiply in signed
  `int` until 0.20.0, which is undefined behaviour; in Rust it is
  `wrapping_mul` and `wrapping_add` on `u32`. [6] **[P]**
- **Checked.** The keystream decrypts the SR2SubIFD of all 34 SR2, ARW and
  ARQ samples that have one (every sample except the A100 and the SRF): the
  result parses as an IFD of 101 to 232 entries with valid types. The
  blocks are 11,932 to 62,112 bytes long, well under LibRaw's cap of
  10,240,000. **[F]**
- **Offsets inside are absolute.** The decrypted IFD's value offsets are
  file offsets, so rebase them by a checked subtraction of the SR2SubIFD's
  offset, and treat any offset outside the decrypted block as absent.
  rawspeed pads its buffer and forbids pointers into the padding; dcraw
  re-parses with a negative base. [1][12] **[P]**
- **Plain copies.** From ARW 2.3.3 (FileFormat 3.3.3.0, the ILCE-9 of
  2017) the raw IFD also carries 0x7310 BlackLevel and 0x7313
  WB_RGGBLevels in the clear, identical to the SR2SubIFD values; the A7R II
  and earlier files do not. Files 1541, 1542, 3012, 3015, 3251 and every
  later file **[F]** The SR2SubIFD is still written on the A7 V. **[F]**
  rawspeed and rawler read only the SR2SubIFD, and rawler fails when its
  white-balance tags are missing; LibRaw reads 0x7310/0x7313 in any IFD.
  [3][12][18] **[P]**
- **0x787F WhiteLevel exists only in the SR2SubIFD**, so a decoder that
  wants Sony's own white level still has to decrypt it.

## Levels, CFA, crop, white balance and colour

### Black level

| Source | Bodies | Values in the samples |
|---|---|---|
| SR2SubIFD 0x7310, int16u[4], R G G B | ARW 2.0 and later | 512 on most bodies; 800 on the 1-inch RX100 and RX100M6 (5811, 3012, 3015); 4 on the A200 (2271); 512 on ARW 6.0 (8846–8850) **[F]** |
| raw IFD 0x7310 (plain copy) | ARW 2.3.3 and later | identical to the SR2SubIFD **[F]** |
| SR2SubIFD 0x7300, int16u[4] | DSC-R1 | 514, 511, 512, 513 (3221) **[F]** |
| MRW block | DSLR-A100 | not established |

- **Scale.** The values are at 14-bit scale; ExifTool annotates 0x7310
  with "divide by 4". [22] **[P]** Divide by 4 for 12-bit packed data
  (A900: black 512, data minimum 134) and for ARW1 (A200: 4, about 1 at
  12-bit, and its data minimum is 0). Do not divide for 12-bit
  uncompressed data, which are at 14-bit scale (file 1311), or for cRAW,
  whose curve output is at 14-bit scale. Files 5858, 2271, 1311 **[F]**,
  cRAW **[I]**
- **ARW 6.0.** 512 in the file; LibRaw uses 1024. See
  [ARW 6.0](#arw-60-compression-32766).
- **Channel order: the decoders disagree.** LibRaw reads 0x7310 as fixed
  colours R, G1, G2, B and reads 0x7300 without reordering; rawspeed and
  rawler store the four values by CFA position. [3][6][12][18] **[P]** The
  readings agree on RGGB bodies, which is every sampled CFA body except
  the R1 **[F]** (the A100 is GRBG per dcraw, LibRaw and rawspeed, but
  RGGB in rawler's camera file; not measured), so on
  the samples they diverge only on the R1, where 0x7300's order is
  unknown. **[I]**
- **Fallbacks.** When the file gives nothing, dcraw uses 128 << (bps − 12)
  for raw widths above 3888 and 200 << (bps − 12) for 5504-wide DSC bodies;
  LibRaw uses 512 for cRAW and 128 << (bps − 12) otherwise. [1][4] **[P]**
  rawspeed lists a black of 0 for the 12-bit A200 to A390 and 495 for the
  F828. [16] **[P]** (quoted for cross-checking, not for transcription)
- **Masked pixels.** rawspeed declares a black area only for the RX1R (14
  columns at x = 6032); dcraw's per-width trims cut unusable columns, not
  calibrated black. [1][16] **[P]**

### White level

- **SR2SubIFD 0x787F** is 15360 in every sample that has it: one value on
  the A200, A700 and A900, three from the NEX-5 on; the R1 has none.
  **[F]** rawspeed and rawler use it as the white point (rawspeed takes
  the first value); LibRaw treats it only as a "linear maximum". [6][12][18]
  **[P]**
- **DNG WhiteLevel 0xC61D** in the raw IFD, from the ILCE-9 on: 16383
  (16380 on the RX100M6), and 16383 on ARW 6.0 too. Absent from the YCbCr
  M/S raw IFDs. **[F]** LibRaw uses it to repair a broken BitsPerSample.
  [3] **[P]**
- **The two disagree in the same file** (for example 15360 against 16383
  in files 1541 and 1542). **[F]** Which one matches real clipped
  highlights was not measured. A user reports practical white points of
  about 15360 to 16300. [29] **[S]**
- **cRAW can exceed both**: its curve reaches 17204 with the common knots
  (see [The 0x7010 tone curve](#the-0x7010-tone-curve)), and rawspeed lists
  16000 to 16628 for cRAW-era bodies, 15000 for the A850, 4095 for the
  12-bit A100 to A390, and 33192 at ISO 50 for the RX1R (unexplained).
  [16] **[P]**, quoted for cross-checking only
- **ARW 6.0 and M/S** have their own scales: LibRaw's 39002 and 32800 for
  ARW 6.0, and an estimated 17536 or 18091 for YCbCr. [5][9] **[P]**
- Report the file's values with their provenance and derive practical
  white levels from the project's own samples rather than copying a table.

### CFA pattern

- The raw IFD carries CFARepeatPatternDim (0x828D) and CFAPattern
  (0x828E). Measured: 1, 0, 2, 1 (GRBG) on the R1 (3221) and 0, 1, 1, 2
  (RGGB) on every other CFA sample. **[F]**
- **No CFA tags**: the A100 (no raw IFD), the SRF and the ARQ (four
  samples per pixel). The M/S YCbCr IFDs carry 255, 255, 255, 255. **[F]**
- rawspeed ignores the tag and uses its camera list: RGGB for every Sony
  except the A100 and R1 (GRBG) and the F828 (four colours). [12][16]
  **[P]** dcraw and LibRaw also give the A100 GRBG, while rawler's camera
  file says RGGB. [1][4][18] **[P]** The A100's pattern is therefore a
  disagreement between decoders, not measured.
- As for NEF, state the pattern relative to a named origin and re-phase it
  when a crop starts on an odd row or column. **[I]**

### Crop

- **Tags in the raw IFD** [21] **[P]**:
  - 0x74C7 SonyCropTopLeft and 0x74C8 SonyCropSize (int32u[2] each);
  - DefaultCropOrigin (0xC61F) and DefaultCropSize (0xC620), DNG tags that
    Sony writes;
  - 0x7038 SonyRawImageSize on lossless files.
- **0x74C7 is (left, top)**, as LibRaw reads it, not (top, left) as
  libopenraw does. The RX100M6 1:1 file gives (924, 12) for a centred
  3648×3648 crop of a 5504-wide raw: 924 = 12 + (5472 − 3648)/2. The 16:9
  file gives (12, 296). Files 3012, 3015 **[F]**
- **The two crop pairs can disagree.** They are equal on the A9, 7S III,
  A1, 7 IV, 6700, 9 III, 1 II and 7 V, but on the A7R III SonyCropTopLeft
  is (0, 8) while DefaultCropOrigin is (8, 8). Files 3251, 3255 **[F]**
  Prefer DefaultCrop for the visible area. **[I]**
- **Aspect-ratio crops live only in SonyCrop.** On the RX100M6 the raw and
  DefaultCrop stay 3:2 (5504×3672 and 5472×3648) while SonyCrop, Exif
  PixelX/YDimension and the preview carry the 1:1 or 16:9 crop. Files
  3012, 3015 **[F]**
- **APS-C crop mode** gives a smaller raw IFD with the same SonyModelID
  (A7 IV 4736×3132, A7S 2816×1872, A1 II 5664×3768). Files 6933, 6936,
  1582, 7835 **[F]** LibRaw also detects crop-mode files by raw width on
  older full-frame bodies and trims 32 columns. [5] **[P]**
- **Older bodies have no crop tags**; dcraw and LibRaw trim by raw width
  (for example 3984 to 3925 for the R1, −32 for 4288, −24 for 6048 with a
  further −6 on the RX1 and A99, −32 for 8000). dcraw has no case for
  9600; LibRaw trims 9600 by 36 on the ILCE-7RM5, ILCE-7CR and ILX-LR1
  and by 32 on other 9600-wide bodies (the ILCE-7RM4). LibRaw also has
  per-mode tables for newer bodies, whose fallbacks differ: the ILCE-7RM5
  and ILCE-9M3 groups trim 32 and set a "vendor crop suggested" warning,
  the ILCE-1 and ILCE-7M4 groups only set the warning, and the ILCE-6700
  group trims 32 without one. [1][4] **[P]**

### White balance

As-shot white balance is SR2SubIFD 0x7313 (R, G, G, B, greens normalised
to 1024, for example 2516, 1024, 1024, 1600 in file 1542) on ARW, 0x7303
(G, R, B, G, normalised to 256) on the R1, and the MRW `WBG` record on the
A100 (401, 256, 256, 574 in file 2346, little-endian). **[F]** rawspeed and
rawler prefer 0x7303 when both exist; dcraw guesses the 0x7313 order from
the greens. [1][12][18] **[P]** The details are in
[makernote-and-metadata.md](makernote-and-metadata.md). In M/S files the
data are already white-balanced. [5][9] **[P]**

### Colour matrix

The SR2SubIFD holds a 3×3 ColorMatrix (0x7800), which ExifTool names and
annotates "divide by 1024" [22] **[P]**; the samples store it as nine
signed 16-bit values (SSHORT, count 9; files 1542, 2271, 5811, 8846)
**[F]**. Its rows sum to about 1024: in file 1542, 1243 −205
−15 / −31 1209 −154 / 47 −47 1024. **[F]** LibRaw stores it, row-normalised,
and does not use it; every decoder converts colour with its own per-body,
Adobe-derived matrix instead. [1][6][16][18] **[P]** What space the Sony
matrix maps to is not documented. **[I]** raw-files returns sensor data, so
it can expose 0x7800 as metadata without interpreting it.

## 12-bit fallback modes

Sony documents conditions under which bodies that normally record 14 bits
record 12: continuous drive with compressed RAW on many bodies; bulb and
long-exposure noise reduction on the A7 and A7R; the silent shutter on the
A7 II, A7R II and A7S II class, compressed or uncompressed; and bulb and
long-exposure noise reduction on the A7R III, A7 III and A7R IV class,
whose uncompressed silent mode stays at 14 bits. [26] **[S]** (Sony's
support article, read through search extracts only)

- **cRAW**: the stream format is the same; only the precision behind the
  curve changes. **[I]**
- **Uncompressed**: the one 12-bit uncompressed sample (file 1311) stores
  12-bit values at 14-bit scale, so nothing in the layout or the levels
  changes. **[F]** Whether every body does the same is not established.
- **Tags**: BitsPerSample said 12 on file 1311 but SonyRawFileType said 0
  ("14-bit"). **[F]** So neither tag reliably flags a fallback; the low two
  bits of the data do.
- LibRaw has two heuristics for "14-bit metadata on 12-bit data": it
  divides black by 4 when it exceeds 256 on packed Sony data, and divides
  its linear maximum by 4 when that exceeds the maximum by up to four
  times. [5] **[P]**
- No 12-bit fallback sample (for example A6300 or A6500 silent shutter)
  was checked.

## Per-body quirks

- **DSLR-A100.** ARW1 from the IFD0 0x014A offset, 3881 columns, MRW
  metadata, a headered MakerNote (`SONY DSC`). [1][12][22] **[P]**, file
  2346 **[F]**
- **DSLR-A200** (and probably the A230 to A390). ARW1 under Compression
  32767 with H + 8; black 4 at 14-bit scale. File 2271 **[F]**
- **DSLR-A700.** cRAW with BitsPerSample 8. File 2521 **[F]**
- **DSLR-A850 and A900.** 12-bit packed uncompressed mode. [12][18]
  **[P]**, file 5858 **[F]**
- **NEX-5.** Different 0x7010 knots (0, 9824, 11512, 16362). File 1457
  **[F]**
- **DSC-R1 (SR2).** Big-endian 14-bit words; GRBG; black in 0x7300; no
  0x787F; width 3925 of 3984. [1] **[P]**, file 3221 **[F]**
- **DSC-F828 and DSC-V3 (SRF).** Encrypted image at hard-coded offsets;
  RGBE CFA on the F828. [1][4] **[P]**
- **1-inch RX bodies (RX100 series).** Black 800. Files 5811, 3012, 3015
  **[F]**
- **DSC-RX100M6.** Aspect-ratio modes keep a 3:2 raw; the crop is only in
  SonyCrop. Files 3012, 3015 **[F]**
- **RX1, RX1R and A99.** An extra 6-column trim in dcraw and LibRaw; the
  RX1R has a declared black area and an ISO 50 white level in rawspeed.
  [1][16] **[P]**
- **ILCE-7RM3.** SonyCropTopLeft and DefaultCropOrigin disagree. File 3251
  **[F]**
- **ILCE-1.** LibRaw has a workaround for files with a broken
  BitsPerSample. [3][11] **[P]**/**[I]**
- **Hasselblad Lunar** (and the Stellar, Stellar II, HV and Lusso, which are
  rebadged Sony bodies). Make is `HASSELBLAD`, the data are Sony's (cRAW
  with BitsPerSample 12 on the Lunar). File 4051 **[F]** LibRaw relabels a
  Hasselblad make with a Sony MakerNote as Sony [4] **[P]**; rawspeed's ARW
  decoder accepts only make "SONY", so it presumably misses them **[I]**.
- **"Star eater".** On several bodies and firmware versions the camera
  filters long exposures spatially before encoding (reported from firmware
  3.30 on the A7R II, with thresholds of 3.2 or 4 seconds). No decoder can
  undo it. [36] **[S]**

## Data a decoder must bring itself

What is not in the file, and where raw-files would have to get it. This is
an input to the licence decision in
[open-questions.md](open-questions.md), as the Nikon Huffman tables are for
NEF.

| Item | In the file? | Needed for | Notes |
|---|---|---|---|
| ARW1 prefix code | no | A100, A200 (and probably A230–A390) | Reduces to the rule in [The category code](#the-category-code); no table is needed. **[I]** |
| Keystream constants | no | SR2SubIFD (levels and white balance of every ARW), SRF image | A formula with two constants (5^11 and the lags 127 and 63); fully described above. **[I]** |
| SRF key position 0x8DDC | no | F828, V3 | One constant; the data offset and length should be in SRF2 but were not located. [22] **[P]**, **[F]** |
| MakerNote substitution cipher | no | metadata only (0x2010, 0x9050, 0x94xx), not raw data | The inverse is computable from the formula b³ mod 249; see [makernote-and-metadata.md](makernote-and-metadata.md). |
| cRAW tone curve | yes (0x7010) | cRAW | Five slopes from four knots; no default when the tag is absent. |
| Lossless JPEG Huffman tables | yes (DHT in each tile) | Compression 7 | Standard T.81. |
| ARW 6.0 delinearisation curve | **no** | Compression 32766 | 4096 entries, construction unknown; published only as tables extracted from Sony or Adobe output (LibRaw's embedded table, the unlicensed write-up's). **The hard case**; one reason ARW 6.0 is refused. [10][30] |
| ARW 6.0 levels | partly (0x7310, 0xC61D, apparently halved) | Compression 32766 | LibRaw hard-codes 1024 and 39002. [5] |
| Per-body crops | often (SonyCrop, DefaultCrop, 0x7038) | older bodies | Derive per-width trims from samples. |
| Per-body black and white levels | usually (SR2SubIFD, raw IFD) | all | Fallbacks and practical white points: derive from samples. |
| Colour matrices | Sony's 0x7800 exists, meaning unknown | rendering only | Every decoder uses Adobe-derived per-body matrices. |
| YCbCr→RGB coefficients, chroma midpoint | no | M/S, only if converting | LibRaw uses BT.601 and a midpoint of 16383. [9] |

Graphite's rawkit (MIT OR Apache-2.0) is the only permissively labelled
Rust code with an ARW1 table, a cRAW loop and a 0x7010 curve builder; they
match dcraw element for element **[P]**, and that they were translated
from it is **[I]**. It does not change the list above: the ARW1 code needs
no table, and the curve builder is a five-line rule.

## Decoder robustness checklist

Derived from the decoders' own guards, from the sample checks and from past
vulnerabilities; the history is in [security.md](security.md). The NEF
checklist ([../nef/raw-data.md](../nef/raw-data.md)) applies as well.

1. **Gate ARW 6.0 first.** Compression 32766 returns a typed unsupported
   error before any decoding starts, as rawler's camera-mode check does;
   do not rely on reaching a stub, and do not rely on catching panics as
   rawler's `RawLoader::decode` does. [18] **[P]**
2. **Dispatch on several agreeing facts.** Compression, Photometric,
   SamplesPerPixel, tiling and exact byte counts, each validated first;
   never BitsPerSample, SonyRawFileType or the model string alone. LibRaw's
   A100 special case keyed on model and width became CVE-2017-6887 (an
   unbounded IFD index). [37] **[P]**
3. **Checked size arithmetic.** Compute W·H, 1.5·W·H, 2·W·H and 8·W·H
   with checked operations; bound every allocation by the bytes present;
   reject zero dimensions. rawspeed's caps (9600×6376 for cRAW, 10240×7168
   for tiles) are policy, and Sony widths already reach 9728. [12] **[P]**
4. **Strips.** Offset plus count inside the file; report a truncated strip
   rather than silently clamping it as rawspeed does. [12] **[P]**
5. **ARW1.** Reject categories above 12; keep the running sum in 0 to 4095
   and count violations (rawspeed added the negative check in 2018); handle
   category 0 without a shift; define what happens when the bits run out.
   [37] **[P]**
6. **cRAW.** Require W ≥ 32 and W a multiple of 32 (or say what happens to
   a partial span); require exactly W·H bytes; compute `max − min` signed;
   reject or flag `imax == imin` and never read past the 16-byte block;
   clamp to 2047. rawspeed once read out of bounds on widths below 32
   (an unsigned `w − 30`), and LibRaw once overran its row buffer.
   [37] **[P]**
7. **Tone curve.** Mask each knot to 12 bits, build the table in `u32`,
   reject or flag decreasing knots, and index only with values below the
   table length.
8. **Lossless tiles.** Tile width and height non-zero and even; offsets
   and counts arrays of equal length; tile count equal to
   ⌈W/tw⌉·⌈H/th⌉, checked by division; `tileX · tw` in 64 bits; every tile
   inside the file and no two overlapping; **no assumption that offsets
   ascend** (they do not, file 4466 **[F]**); each frame's size, components,
   precision (≤ 16) and predictor checked against the tile; output clipped
   to the raster; a per-tile size cap; a bit reader that errors at the end
   of the tile instead of reading ahead. LibRaw overran the raster in 2024
   and its YCC reader over-read in 2026; rawler slices tile buffers
   unchecked. [18][37] **[P]**
9. **YCbCr M/S.** Subsampling 2×2 or 2×1 only; the SOF3 sampling factors
   consistent with YCbCrSubSampling; tile count as above; even widths.
   LibRaw caps tiles at 1024 and the compressed tile size at 1 GB. [9]
   **[P]**
10. **SR2SubIFD.** u32 offset, length and key; `offset + length` within
    the file (checked add); length at least 4 after rounding down to a
    multiple of 4; a cap; decrypt only bytes present; rebase offsets by a
    checked subtraction; ignore entries pointing outside the block; treat
    decrypted values as plausible-or-ignored. LibRaw decrypted short reads
    until 2024, and indexed its type-size table with a file value
    (OSV-2023-1295). [37] **[P]**
11. **Keystream.** `u32` wrapping arithmetic; ring index masked to 0–127.
12. **SRF.** Bounds-check the byte at MakerNote + 0x8DDC, the key word at
    +4b, SRF1 and the DataKey; next-IFD positions must not precede the
    current one; entry counts 1 to 1000. CVE-2020-35535 (LibRaw
    `parseSonySRF`) and an open rawler fuzzing report (a panic indexing
    offset 200,896 of a 44 KB file) show the class. [37] **[P]**
13. **ARQ.** Require 4 samples, Photometric 32892 or equivalent 4-colour
    metadata, and B = 8·W·H by checked arithmetic; 16-shot files exceed
    240 megapixels. LibRaw once decoded ARQ without checking it was
    four-colour. [37] **[P]**
14. **IFD roles.** Read NewSubfileType as `u32` and never narrow it
    (CVE-2026-88387); never let a SubIFD count or a special case advance an
    index into a fixed table (CVE-2017-6887); check Photometric and
    BitsPerSample before treating a Compression 7 IFD as raw, because
    IFD2 is a JPEG preview with Compression 7. [37] **[P]**, **[F]**
15. **Test and fuzz.** No lint enforces "never panic" (see
    [ADR008](../../architecture-decisions/adr008-lints-and-panics.md)), so
    truncated strips, crafted cRAW blocks, bad tile tables, a tiny SR2
    block and an SRF header belong in the tests and the fuzz corpus. Almost
    every Sony bug above was found by fuzzing. [37] **[P]**

## Recommendation for raw-files

For milestone 5: **[I]**

- **First:**
  - uncompressed 16-bit words (Compression 1, B = 2·W·H), including the
    12-bit-at-14-bit-scale case and the R1's big-endian SR2 data;
  - cRAW (32767, B = W·H) with the 0x7010 curve, no dither, returning
    `curve[2·code]` as LibRaw does;
  - lossless L (Compression 7, Bayer tiles), which needs a small T.81
    lossless decoder (predictor 1, up to 4 components) reusable for other
    formats;
  - SR2SubIFD decryption, because the black and white levels and the
    white balance of every ARW live there (the plain copies exist only from
    ARW 2.3.3, and 0x787F never has one).
- **Second:** lossless M/S as a three-channel YCbCr image, flagged as
  non-CFA, white-balanced and non-linear, with any RGB conversion labelled
  approximate; 12-bit packed (A700, A850, A900); ARW1 (A100, A200), now
  that its decoding and the A200's "+8" are checked on samples.
- **Later:** SRF (two bodies from 2003–2004, encrypted, RGBE, offsets not
  yet read from SRF2) and ARQ (simple layout, but a desktop-software
  product of hundreds of megabytes).
- **Refuse** ARW 6.0 (Compression 32766) with a typed unsupported error.
- **Dispatch** on Compression, then Photometric and SamplesPerPixel, then
  exact byte counts; never on BitsPerSample; record SonyRawFileType,
  RAWFileType and Quality as cross-checks.
- **Output:** samples at raw dimensions, with tile padding removed using
  0x7038 when present; the CFA pattern relative to a named origin; black
  and white levels from the file with their provenance (raw IFD, SR2SubIFD
  or fallback) and scale; DefaultCrop, SonyCrop and 0x7038 as separate
  metadata. Development to a linear image builds on this output
  ([ADR010](../../architecture-decisions/adr010-decoding-ends-at-a-linear-image.md)).
- **Corrupt data:** decode what can be decoded and count range violations
  and bad blocks, rather than silently returning different pixels.
- **Tests:** one sample per row of the dispatch table, plus truncated and
  corrupted variants; see [test-data.md](test-data.md).

## Open questions

These are also tracked in [open-questions.md](open-questions.md).

- **SRF2 to SRF6.** Where SRF2 starts and how it is keyed (the byte order
  of SRF2Key, whether the keystream starts at SRF1 or SRF2): re-test on
  file 1351, read RawDataOffset and RawDataLength from SRF2 and compare
  with 862,144 and 16,531,200.
- **F828 CFA.** Decrypt and decode file 1351 and check for four distinct
  channel means in the RGBE (or Y R / B G) arrangement; the DSC-V3's
  pattern and data offset (no sample on raw.pixls.us).
- **ARW1 on other bodies.** Decode the A230, A290, A300, A330, A350, A380
  and A390 samples as ARW1 (with H + 8) and check that all bytes are
  consumed; what the 8 extra rows of the A200 hold.
- **cRAW at full scale.** A clipped-highlight cRAW sample from a 14-bit
  body (for example ILCE-9 or ILCE-7M4) to see whether code 2047 occurs
  and whether the output really exceeds 16383; compare with 0x787F 15360
  and 0xC61D 16383.
- **White level.** On A9, A7 IV and A1 uncompressed samples with clipped
  areas, which of 0x787F (15360) and 0xC61D (16383) matches saturation.
- **0x7010 knots.** Whether bodies other than the NEX-5 store other knots,
  and whether the knots change with ISO or the 12-bit modes.
- **12-bit fallback.** A silent-shutter or continuous-drive 12-bit sample
  (A6300, A6500, A7 III) for both cRAW and uncompressed: confirm 14-bit
  scaling and what BitsPerSample and SonyRawFileType say.
- **R1 0x7300 channel order** against its GRBG CFA, from a uniform patch.
- **M/S YCbCr.** The luma and chroma scale (precision 16, chroma
  midpoint), whether rawler's doubled black is right, and M/S files from
  the A1, A1 II and A7R V (subsampling per body).
- **Lossless 12-bit.** Whether any lossless file uses 12-bit precision
  (A1 continuous or silent shutter).
- **ARW 6.0.** The A7R VI tile layout (2×2?) and its stream tag (`0000`?);
  the meaning of the tile-stream header bytes 4–7 (`00 00 00 02`) and 12–15
  (`3D 00 6E 00`) on the A7 V; whether Compressed and Compressed (HQ)
  differ only in quantiser choice.
- **ColorMatrix 0x7800.** What it maps; compare with Adobe's matrix for
  the ILCE-9.
- **ARQ variants.** A 16-shot ARQ and ARQs from the A7R V, A7CR and A1 II:
  dimensions, Photometric and crop.
- **A100 levels.** Where the A100's black and white levels are, if
  anywhere in the MRW block.
- **Implementation conflicts** to settle with samples: the cRAW output scale
  and the `imax == imin` policy (no sample has it), the A100 crop and CFA
  pattern (GRBG or RGGB), the black-level channel order, and the M/S RGB
  conversion.
- **ARW 6.0 and decryption** as legal questions (the curve's provenance,
  a possible LLVC or JPEG XS patent link, and decryption under
  anti-circumvention law, which every open decoder has done for about 20
  years with no known action [38] **[S]**).

## Sources

All retrieved October 2026. Implementation sources are read only; see
[licences-and-references.md](licences-and-references.md). Entries marked
"search extract" were read only through search-result snippets.

[1] dcraw 9.28 (rev 1.478), dcraw.c — https://www.dechifro.org/dcraw/dcraw.c ; the text was read from a mirror, https://raw.githubusercontent.com/ncruces/dcraw/master/dcraw.c, not verified against the original (`sony_decrypt`, `sony_load_raw`, `sony_arw_load_raw`, `sony_arw2_load_raw`, `parse_tiff_ifd` Sony cases, `apply_tiff` case 32767, `packed_load_raw`, `unpacked_load_raw`, `identify` Sony branches)
[2] LibRaw, decoders_dcraw.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/decoders_dcraw.cpp (`sony_load_raw`, `sony_arw_load_raw`, `sony_arw2_load_raw`)
[3] LibRaw, tiff.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/tiff.cpp (raw IFD selection, Sony lossless-JPEG probe, BitsPerSample repair, 0x7000, 0x7010, 0x7200–0x7221, 0x7310, 0x7313, 0x74C7, `apply_tiff` cases 1, 7, 32766, 32767)
[4] LibRaw, identify.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/identify.cpp (Sony black fallbacks, per-body size tables, F828, V3, A100); normalize_model.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/normalize_model.cpp (Hasselblad rebadges)
[5] LibRaw, open.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/utils/open.cpp (APS-C trims, ARQ crop, ARW 6.0 levels, packed-data black, YCC white balance, F828 white balance)
[6] LibRaw, sony.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/sony.cpp (`parseSonySR2`, `sony_decrypt`, 0x2029)
[7] LibRaw, decoders_libraw.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/decoders_libraw.cpp (`sony_arq_load_raw`)
[8] LibRaw, decoders_libraw_dcrdefs.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/decoders_libraw_dcrdefs.cpp (`sony_ljpeg_load_raw`)
[9] LibRaw, sonycc.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/sonycc.cpp (`sony_ycbcr_load_raw`)
[10] LibRaw, sony_arw6.cpp — https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/decoders/sony_arw6.cpp ; PR #824 (merged 2026-07-18) — https://github.com/LibRaw/LibRaw/pull/824
[11] LibRaw, Changelog — https://raw.githubusercontent.com/LibRaw/LibRaw/master/Changelog.txt
[12] rawspeed, ArwDecoder.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decoders/ArwDecoder.cpp
[13] rawspeed, SonyArw1Decompressor.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decompressors/SonyArw1Decompressor.cpp
[14] rawspeed, SonyArw2Decompressor.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decompressors/SonyArw2Decompressor.cpp
[15] rawspeed, LJpegDecoder.cpp — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/src/librawspeed/decompressors/LJpegDecoder.cpp
[16] rawspeed, cameras.xml (Sony entries) — https://raw.githubusercontent.com/darktable-org/rawspeed/develop/data/cameras.xml
[17] rawspeed PR #972 (ARW 6.0, closed unmerged 2026-09-13) — https://github.com/darktable-org/rawspeed/pull/972
[18] rawler (dnglab) — https://github.com/dnglab/dnglab/tree/main/rawler (`src/decoders/arw.rs`, `src/decompressors/arw6/mod.rs`, `src/decompressors/ljpeg/`, `data/cameras/sony/`); issue #263 (A7R V M/S) — https://github.com/dnglab/dnglab/issues/263 ; issue #681 (ARW 6.0) — https://github.com/dnglab/dnglab/issues/681 ; PR #825 — https://github.com/dnglab/dnglab/pull/825
[19] rawloader, arw.rs — https://raw.githubusercontent.com/pedrocr/rawloader/master/src/decoders/arw.rs
[20] rawkit (Graphite) — https://github.com/GraphiteEditor/Graphite/tree/master/libraries/rawkit (`src/lib.rs`, `src/decoder/arw1.rs`, `src/decoder/arw2.rs`, `src/decoder/uncompressed.rs`)
[21] ExifTool, Exif.pm (Compression values; tags 0x7000–0x74C8) — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Exif.pm
[22] ExifTool, Sony.pm (FileFormat, RAWFileType, SR2Private, SR2SubIFD, SRF, `Decrypt`, `ProcessSRF`) — https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Sony.pm ; tag pages — https://exiftool.org/TagNames/Sony.html
[23] Laurent Clévy, "Description of Sony RAW file formats" v0.55 (GPL-3.0) — https://github.com/lclevy/sony_raw
[24] RawDigger, "Detecting posterization in Sony cRAW/ARW2 files" (2014) — https://www.rawdigger.com/howtouse/sony-craw-arw2-posterization-detection ; Lloyd Chambers, star-trail artefacts (2014, search extract) — https://www.diglloyd.com/blog/2014/20140214_1-SonyA7-artifacts-star-trails.html
[25] Henry Dietz, "Sony ARW2 Compression: Artifacts and Credible Repair", IS&T Electronic Imaging 2016 — https://aggregate.org/DIT/EI2016/ei2016ARW.pdf ; https://doi.org/10.2352/ISSN.2470-1173.2016.2.VIPC-227
[26] Sony support article 00229990, "What shooting settings affect RAW image bit sizes (14 bits to 12bits)?" — https://www.sony.co.uk/electronics/support/articles/00229990 (search extracts only)
[27] Sony Help Guides, "RAW File Type": ILX-LR1 — https://helpguide.sony.net/ilc/2390/v1/en/contents/0404I_raw_file_type.html ; ILCE-1M2 — https://helpguide.sony.net/ilc/2440/v1/en/contents/201h_raw_file_type.html ; ILCE-7M5 — https://helpguide.sony.net/ilc/2540/v1/en/contents/251h_raw_file_type.html ; Sony ANZ, Alpha 1 and Alpha 7S III firmware — https://scene.sonyanz.com/articles-and-resources/sony-announces-new-firmware-for-alpha-1-and-alpha-7s-iii-cameras
[28] darktable issue #19917, "Sony ILCE 7m5" — https://github.com/darktable-org/darktable/issues/19917
[29] discuss.pixls.us, "raw black/white point" — https://discuss.pixls.us/t/raw-black-white-point/39426
[30] abbradar, arw6_decode README (no licence file) — https://github.com/abbradar/arw6_decode
[31] SMPTE RDD 34:2015, "LLVC – Low Latency Video Codec for Network Transfer" — https://pub.smpte.org/latest/rdd34/rdd34-2015.pdf
[32] Google Patents, US 2007/0223824 A1 (US 8,023,748 B2) — https://patents.google.com/patent/US20070223824A1/en ; US 8,013,910 B2 — https://patents.google.com/patent/US8013910B2/en
[33] JPEG XS patent pool — https://www.intopix.com/blogs/post/intoPIX-and-Fraunhofer-IIS-team-with-Vectis-to-offer-JPEG-XS-patent-pool-license ; https://jpegxspool.com/
[34] Sony Imaging Edge, "Pixel Shift Multi Shooting" — https://support.d-imaging.sony.co.jp/app/imagingedge/en/instruction/1_2_about_psm.php
[35] A7 V support reports: Adobe community — https://community.adobe.com/questions-563/feature-request-support-for-sony-a7-v-compressed-hq-raw-files-in-lightroom-camera-raw-1551146 ; DxO forum — https://forum.dxo.com/t/sony-a7v-compressed-and-hq-raw-not-supported/54584 (search extracts only)
[36] Jim Kasson, The Last Word, A7R II star eating — https://blog.kasson.com/the-last-word/sony-a7rii-fw-4-0-star-eating/ (search extracts only)
[37] Sony-path security history: NVD, CVE-2017-6887 — https://nvd.nist.gov/vuln/detail/CVE-2017-6887 ; CVE-2020-35535 — https://nvd.nist.gov/vuln/detail/CVE-2020-35535 ; CVE-2026-88387 — https://nvd.nist.gov/vuln/detail/CVE-2026-88387 ; LibRaw commits cc54bb6, 17294b5, 62f0423, 9c9c04b, 17f1683, 4e265f1 — https://github.com/LibRaw/LibRaw/commit/<sha> ; LibRaw issue #792 — https://github.com/LibRaw/LibRaw/issues/792 ; rawspeed commits a3a8b581, fa488def, b9f5b8b1, bbd6d363, 0953acf8 — https://github.com/darktable-org/rawspeed/commit/<sha> ; dnglab issues #569–#575 — https://github.com/dnglab/dnglab/issues/574 ; OSV-2023-1295 — https://github.com/google/oss-fuzz-vulns . Details in [security.md](security.md).
[38] Web searches for Sony legal action, DMCA notices or cease-and-desist
    letters against dcraw, LibRaw or other ARW decoders (October 2026): no hits
[39] raw.pixls.us, Sony catalogue and the exiv2 metadata dumps it publishes per file — https://raw.pixls.us/
[40] raw.pixls.us sample files (all CC0), checked October 2026. The same list
    is in [test-data.md](test-data.md#files-checked).

| File | Body | Mode (catalogue) | SHA-256 | URL |
|---|---|---|---|---|
| 964 | Sony SLT-A55 | 12bit 12bit compressed (3:2) | `f861454a49abe0345ff69887874d00617cdf0b96af96d605051e68deda424a08` | https://raw.pixls.us/getfile.php/964/nice/Sony%20-%20SLT-A55%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 1310 | Sony ILCE-7RM2 | 12bit 12bit compressed (3:2) | `a606ebf1dcc6704f03f2966326c60a87dd6bdb3704464b3897c10349a10d3a19` | https://raw.pixls.us/getfile.php/1310/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 1311 | Sony ILCE-7RM2 | 12bit 12bit uncompressed (3:2) | `71e0888396ef52c7e6a990b4e72ebb4d8413a1fc1c35bada449870539cef6e39` | https://raw.pixls.us/getfile.php/1311/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20uncompressed%20(3:2).ARW |
| 1351 | Sony DSC-F828 | 4:3 | `07739f94bcab0c7e198d418ac65172d0dce04bb84a7683ecffd0bfe4b32f6d79` | https://raw.pixls.us/getfile.php/1351/nice/Sony%20-%20DSC-F828%20-%204:3.SRF |
| 1457 | Sony NEX-5 | 12bit 12bit compressed (3:2) | `3b085569ac1fec37a1289ed5ea517497692a3ea14cea41953dd6e878abf230a3` | https://raw.pixls.us/getfile.php/1457/nice/Sony%20-%20NEX-5%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 1541 | Sony ILCE-9 | 14bit 14bit compressed (3:2) | `598bbc828c279528f504c51a10418521b17a90a25b198368f65e19cfda9e3711` | https://raw.pixls.us/getfile.php/1541/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20compressed%20(3:2).ARW |
| 1542 | Sony ILCE-9 | 14bit 14bit uncompressed (3:2) | `22c2158c9796386dba28d7cf040a1ca70f4ec2c927ec50771b78bfb523e21c29` | https://raw.pixls.us/getfile.php/1542/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW |
| 1582 | Sony ILCE-7S | 14bit 14bit compressed (3:2), APS-C crop | `a35ebb2fbec929daa5beb20d1ce5c15a8aac7b1a7a231455387f3df8a7442e07` | https://raw.pixls.us/getfile.php/1582/nice/Sony%20-%20ILCE-7S%20-%2014bit%2014bit%20compressed%20(3:2).ARW |
| 2271 | Sony DSLR-A200 | 12bit 12bit compressed (3:2) | `04eacd403d92e0c90da9d91b87615af8eda1afc963350d3ab774557c36763eef` | https://raw.pixls.us/getfile.php/2271/nice/Sony%20-%20DSLR-A200%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 2346 | Sony DSLR-A100 | compressed (3:2) | `4df7981ac4baec9648b4f106751ea4b09a6e27bc70a15f8ef09bec051fec362d` | https://raw.pixls.us/getfile.php/2346/nice/Sony%20-%20DSLR-A100%20-%20compressed%20(3:2).ARW |
| 2521 | Sony DSLR-A700 | 8bit 8bit compressed (3:2) | `3159e28892bf0771d01525cd4b6190f9c15bbb19fa2fab6d2515ced0594e8f40` | https://raw.pixls.us/getfile.php/2521/nice/Sony%20-%20DSLR-A700%20-%208bit%208bit%20compressed%20(3:2).ARW |
| 3012 | Sony DSC-RX100M6 | 12bit 12bit compressed (1:1) | `53c5abad853f17d2c46d9f96f8ff4e0af0ea095193fecf61222a8bf9d8beb00b` | https://raw.pixls.us/getfile.php/3012/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(1:1).arw |
| 3015 | Sony DSC-RX100M6 | 12bit 12bit compressed (16:9) | `fe9e4df604fefbcc0ed78fb4a1d9a58dbaa1f4011e3d119b3ee32fc583d43907` | https://raw.pixls.us/getfile.php/3015/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(16:9).arw |
| 3221 | Sony DSC-R1 | 14bit 14bit uncompressed (3:2) | `921c5f2513dd1671a9089e73fc778fc3573bc4cb70a3a555d0414661d0e62274` | https://raw.pixls.us/getfile.php/3221/nice/Sony%20-%20DSC-R1%20-%2014bit%2014bit%20uncompressed%20(3:2).SR2 |
| 3251 | Sony ILCE-7RM3 | 14bit 14bit uncompressed (3:2), pixel-shift frame | `eaedff10f11b7bdd7908db40d8e1f541bd29a89369b897f181b87c29d549f0a4` | https://raw.pixls.us/getfile.php/3251/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW |
| 3255 | Sony ILCE-7RM3 | 14bit 14bit uncompressed (3:2), ARQ | `67af1f9f4cff82bd182f3fc48e95d39ae7d8352595e3c72f3386c9dee114f27d` | https://raw.pixls.us/getfile.php/3255/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARQ |
| 3751 | Sony SLT-A99 | 12bit 12bit compressed (3:2) | `bc033b8b0df5b1ef603ead4b57f979b8a0a46e7a52d8b46ac6dfa22f86c243f8` | https://raw.pixls.us/getfile.php/3751/nice/Sony%20-%20SLT-A99%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 4051 | HASSELBLAD Lunar | 12bit (3:2) | `029b1283a3d1226e08fc1eb3b81874347f5c128e437f78f853436fd6ba15873b` | https://raw.pixls.us/getfile.php/4051/nice/HASSELBLAD%20-%20Lunar%20-%2012bit%20(3:2).ARW |
| 4466 | Sony ILCE-1 | 14bit (4:3), lossless | `4012536ee0f938d8c5c5e932bb7953a03134abbd9f3f4c664cb2526c1903d492` | https://raw.pixls.us/getfile.php/4466/nice/Sony%20-%20ILCE-1%20-%2014bit%20(4:3).ARW |
| 5418 | Sony ILCE-7SM3 | 14bit 14bit compressed (3:2) | `17e35d98d448b44164ecf414f980e9248b5829233e83a1adbee396b6c1a1a0c6` | https://raw.pixls.us/getfile.php/5418/nice/Sony%20-%20ILCE-7SM3%20-%2014bit%2014bit%20compressed%20(3:2).ARW |
| 5811 | Sony DSC-RX100 | 12bit 12bit compressed (3:2) | `579a485b5126a25cbd55cbd5dadfa7d09cf021c99cc7d4869f9e56e3f759390b` | https://raw.pixls.us/getfile.php/5811/nice/Sony%20-%20DSC-RX100%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 5858 | Sony DSLR-A900 | 12bit 12bit compressed (3:2) | `ac7c1532df77c321e8010aa1be60c1b9f245b1ac0db6f38fe617a73aae81af50` | https://raw.pixls.us/getfile.php/5858/nice/Sony%20-%20DSLR-A900%20-%2012bit%2012bit%20compressed%20(3:2).ARW |
| 6736 | Sony ILCE-6700 | 14bit (4:3), lossless | `dc3e2ed22f46fcef778553767e0703bc37cfb043f107afe2d7e6e4dbca34200a` | https://raw.pixls.us/getfile.php/6736/nice/Sony%20-%20ILCE-6700%20-%2014bit%20(4:3).ARW |
| 6929 | Sony ILCE-7M4 | 14bit (4:3), lossless L | `851b43c2116c4139104a5036f83ac3b6a148789b2142214dd7192c13972b25b6` | https://raw.pixls.us/getfile.php/6929/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(4:3).ARW |
| 6930 | Sony ILCE-7M4 | 4:3, lossless M | `d453005714327addd75bcb99c1c6223173dc92f2a59542d82e6761ef0e0e7571` | https://raw.pixls.us/getfile.php/6930/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW |
| 6931 | Sony ILCE-7M4 | 4:3, lossless S | `cbbd0930c7d8706dff84c68a2004454266e6fd0d8354f5f76a106b5d776e0223` | https://raw.pixls.us/getfile.php/6931/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW |
| 6933 | Sony ILCE-7M4 | 14bit 14bit uncompressed (3:2), APS-C | `bc0089c821a3711ebbeec3844cde5e5c247ac7cf9ca8ad9ae03a977b232216b2` | https://raw.pixls.us/getfile.php/6933/nice/Sony%20-%20ILCE-7M4%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW |
| 6936 | Sony ILCE-7M4 | 14bit (3:2), APS-C compressed | `639a6d4db881f1359e3ea7e1137b314e59417d24e6d4d862b7202111a2c823b2` | https://raw.pixls.us/getfile.php/6936/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(3:2).ARW |
| 7361 | Sony ILCE-9M3 | 14bit (3:2), compressed | `81b0b140f34356964fdb4adf5e202eb01d5e5e5a1b61b861954a183fc30d4bf0` | https://raw.pixls.us/getfile.php/7361/nice/Sony%20-%20ILCE-9M3%20-%2014bit%20(3:2).ARW |
| 7364 | Sony ILCE-9M3 | 3:2, lossless S | `3d9da593a027bdbf31589eb950b04be7ce3f3dc6105ca339c5b7702ffa7fefdd` | https://raw.pixls.us/getfile.php/7364/nice/Sony%20-%20ILCE-9M3%20-%203:2.ARW |
| 7835 | Sony ILCE-1M2 | 14bit Lossy (3:2), APS-C | `050c924e5f78afd8479aa442f17b8650e0a5ebfa5757e57368cb0dc865ca2417` | https://raw.pixls.us/getfile.php/7835/nice/Sony%20-%20ILCE-1M2%20-%2014bit%20Lossy%20(3:2).ARW |
| 8846 | Sony ILCE-7M5 | 14bit (3:2), FF compressed HQ | `4e3a99cbcf2a348decaf3ca68c5de22b7a05eaa590f4e9184735c1f9c45213a7` | https://raw.pixls.us/getfile.php/8846/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
| 8847 | Sony ILCE-7M5 | 14bit (3:2), FF compressed | `46d73475dfd8f96fd7e2f4d4a721a36cbbaba3d6ea95a4851809cd9d252b2a80` | https://raw.pixls.us/getfile.php/8847/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
| 8848 | Sony ILCE-7M5 | 14bit (4:3), APS-C lossless | `51ad709cba01a5f05e3c723b779f6b5fda8fa7b378494dbb40b4d3d3aa5494a4` | https://raw.pixls.us/getfile.php/8848/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(4:3).ARW |
| 8849 | Sony ILCE-7M5 | 14bit (3:2), APS-C compressed HQ | `c16cae99d26a2f8ad183d31a23823310a994aa058d55cfe56e4f82ba56865f57` | https://raw.pixls.us/getfile.php/8849/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
| 8850 | Sony ILCE-7M5 | 14bit (3:2), APS-C compressed | `8315af2a9889709f4c836dbaadc23d226847e5a0630fda212a9f896d5fe0569f` | https://raw.pixls.us/getfile.php/8850/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW |
