# ARW: test data

Real ARW files are large and their licences vary, so the raw-files tests
combine small committed fixtures that we build ourselves with larger public
samples that CI downloads and checks against a hash. This page lists the
sources, the rules for what may be committed, what the public archive holds
for Sony, a minimal set of files, how to build fixtures, and the files the
research was checked against. Back to [the ARW overview](README.md).

Confidence tags: **[P]** primary (verified from the primary page or API),
**[S]** secondary, **[I]** inferred, **[F]** measured on a named sample
file. A sample is cited as "raw.pixls.us file" plus its id;
[Files checked](#files-checked) gives the URL and SHA-256 of each.
Numbers in brackets point to the [Sources](#sources) at the end, retrieved
in October 2026.

Status: desk research plus structural checks on 36 CC0 sample files
(October 2026), listed in [Files checked](#files-checked). The checks were
made with throwaway scripts that are not in the repository, and no pixel
output was compared with a reference decoder.

## 1. Sources and their licence status

| Source | Licence status | Commit to the repository? |
|---|---|---|
| raw.pixls.us | The upload form asks contributors to release files under CC0. Unlike the directory listing, the site's JSON catalogue gives a licence for each file: 200 of the 209 Sony-family files are CC0, and nine are CC BY-NC-SA imports from rawsamples.ch (section 3.2). [1] **[P]** | Only a file the catalogue lists as CC0, and only if small. Normally download in CI instead (section 2). |
| rawsamples.ch (the earlier archive) | Mostly CC BY-NC-SA 2.5 CH; replaced by raw.pixls.us in 2017. [4][5] **[S]** | No. The non-commercial and share-alike terms do not fit `MIT OR Apache-2.0`. |
| dnglab RawDB (rawdb.dnglab.org) | Its contribution guide says sample sets must be CC0. The site is a JavaScript application whose per-file licences we could not read; the dnglab repository holds only analysis results. It has bodies that raw.pixls.us lacks: ILCE-7RM6 (four files) and DSC-RX10M5 (two). [6] **[P]** | No. Download only, after confirming the licence of the specific file. |
| ExifTool sample images | The Sony archive (761 models, 8.5 MB, 2025-12-03) holds JPEG files only, with the main image replaced and previews removed. No licence is stated. [7] **[P]** | No. Treat as unlicensed. |
| Test files of other projects: exiv2's `.exv` metadata extracts, exif-py's A7 III ARW, metadata-extractor's Sony APP1 segments, the three files rawkit's tests fetch from Graphite's server | The projects are GPL-2.0-or-later, BSD-3-Clause, Apache-2.0 and MIT OR Apache-2.0; the licence of each file is not stated. [8] **[P]** | No. |
| Fuzzing corpora | OSS-Fuzz's LibRaw seeds cover CR2, NEF and RAF, **not ARW**; exiv2's corpus is built from exiv2's own test data. [9] **[P]** | No. Generate our own corpus from our own fixtures. |
| Review galleries (Imaging Resource, DPReview, PetaPixel, photographyblog), vendor samples (DxO), files linked from issue threads | Proprietary or unknown. Imaging Resource allows local, private, non-commercial use only. [10] **[S]** | No, and do not redistribute. Manual experiments only. |
| Files you made yourself | Yours to licence. | Yes, with provenance recorded (section 5). |

## 2. What is committed and what is downloaded

The rules are those of the [NEF test-data page](../nef/test-data.md); this
section only adds what is specific to Sony.

**Committed** (small, a few KB each, licence clear):

- hand-built fixtures (section 5.1);
- fixtures stripped from a CC0 sample, with the pixel data removed and the
  structure kept;
- corrupted variants of those (section 5.2);
- minimal reproducers for bugs, each added with its fix.

**Downloaded, in CI or on request** (large: 5.9 MiB to about 129 MiB per
ARW, and 325 MiB for the one ARQ [1]):

- real samples from raw.pixls.us, fetched by URL and **pinned by SHA-256**.
  The site publishes `filelist.sha256` and repeats each file's hash in the
  catalogue; the repository records the hash of each file it uses, and CI
  fails if a download differs. [1][3] **[P]**
- Before adding a file to the list, check its licence in the catalogue
  (the `set=noncc0` view lists the exceptions) and note it in the list
  entry. [1] **[P]**
- Tests that need these files are skipped, not failed, when they are
  absent, so contributors can run the suite offline.
- The ARQ is in an opt-in "large" tier, fetched only with
  `RAW_FILES_LARGE=1`.

Practical points for the download step, as observed in October 2026
**[P]**:

- The server ignores HTTP range requests, so even a preview-only test
  fetches the whole file.
- File URLs contain spaces, parentheses and colons; percent-encode them.
- The folders `data/Sony/` and `data/SONY/` coincide on a case-insensitive
  file system. Key a local cache on the file id or the hash, not on the
  path.

The repository never holds a file whose licence is unclear, however small.

### 2.1 The CI set

The files CI downloads are listed in
[`tests/samples.tsv`](../../../tests/samples.tsv), the source of truth for
their names, hashes, licences and URLs; `scripts/fetch-samples.sh`
downloads them and checks each hash, and the tests in `tests/arw.rs` read
them. The set is every file of [Files checked](#files-checked): 35 in the
normal tier, and the ARQ (file 3255) in the large tier, which the script
fetches only with `RAW_FILES_LARGE=1`.

For each file, `tests/arw.rs` checks the previews against
[section 7.2](container-and-previews.md#72-expected-results-on-the-checked-files)
and [section 4](container-and-previews.md#4-preview-sizes-by-body-and-generation)
of the container page: their sizes, offsets and lengths, the padding after
each end-of-image marker, and IFD0's orientation. Those expected results
are now checked by the test suite as well as by the throwaway scripts.

The tests also check every metadata value the reader gives for each file
against a table generated from a walk of the file's IFDs and Exif IFD, by
the reader's rules, with serial numbers recorded by their shape alone; that
table is the record of the values, so this page does not repeat it.

## 3. What raw.pixls.us holds for Sony

### 3.1 Access

- Files live under `https://raw.pixls.us/data/<Make>/<Model>/`; each also
  has a download URL by id, `https://raw.pixls.us/getfile.php/<id>/nice/`
  followed by a readable name ([Files checked](#files-checked) gives full
  examples). [1][2]
  **[P]**
- `https://raw.pixls.us/data/filelist.sha256` lists `<sha256> *<path>` for
  every file on the site (2016 lines). [3] **[P]**
- The JSON catalogue, `https://raw.pixls.us/json/getrepository.php?set=all`,
  gives per file the make, model, mode label, remark, licence, date, the
  download link with its SHA-256 and size, and a link to a metadata dump;
  `set=noncc0` lists the non-CC0 files, and
  `https://raw.pixls.us/json/getrepository-missing.php?set=missing` the
  bodies without samples. [1] **[P]**
- The whole archive can be cloned with Git LFS
  (`https://raw.pixls.us/data.lfs.git`). [1] **[P]**

Three traps in the catalogue:

- **The column that looks like a size in MB is megapixels.** It equals the
  raw IFD's width times height (42.56 for the 8000×5320 of file 1310). The
  file size is the MiB figure in the link text, which matched every one of
  the 36 downloads to the byte. Every strip and tile lies inside its file;
  apparent "strips larger than the file" come from comparing with the
  megapixel column. **[F]**
- **Mode labels are not a statement of the codec.** They follow fields
  such as BitsPerSample: "8bit" is the A700's BitsPerSample (file 2521),
  and "(4:3)" marks lossless files whose tile-padded raw IFD is close to
  4:3 although the image is 3:2 (files 4466, 6929, 6736, 8848). **[F]**
  Some aspect-ratio labels are wrong, and the ILCE-7 file is labelled
  "14bit compressed" under a file name that says "uncompressed". [1]
  **[P]**
- **The per-file metadata dumps are exiv2 output, not ExifTool's**, at
  `getfile.php/<id>/exif/<original name>.exif.txt`, and no hash is
  published for them. They omit what exiv2 does not decode (the raw IFD's
  Sony tags such as 0x7000, 0x7010, 0x7310 and 0x7313, the SR2Private
  contents, the MakerNote header bytes), they give no pixel size for the
  IFD0 and IFD1 JPEGs, and they under-report OffsetTime\*: files 1541,
  1542, 3012, 3015, 3251, 4466 and 5418 carry all three OffsetTime tags
  that their dumps do not show. **[F]** Use the dumps to choose files, not
  as evidence.

### 3.2 Contents

From the catalogue and directory listings in October 2026 [1][2] **[P]**:

- **209 Sony-family files from 109 bodies**, about 7.8 GiB, of which
  7.6 GiB is CC0: 100 body folders under `data/Sony/`, 8 under
  `data/SONY/` (DSC-RX1RM3, DSC-RX100M7A, ILCE-1M2, ILCE-7M5, ILCE-6400A,
  ILME-FX2, ZV-1M2, ZV-E10M2) and `data/HASSELBLAD/Lunar/`.
- **Formats:** 1 SRF (DSC-F828), 1 SR2 (DSC-R1), 1 ARQ (ILCE-7RM3) and
  206 ARW, the Hasselblad Lunar included. The extension is `.ARW` on most
  files and `.arw` on nine.
- The catalogue's Sony listing also carries 24 Hasselblad 3FR, FFF and DNG
  files. They are not Sony formats, but their metadata dumps were read to
  check the reader's rule for a Hasselblad Make: none of the 11 3FR and 11
  FFF files carries DNGVersion or DNGPrivateData, and the two DNG files,
  from the L1D-20c and L2D-20c drone camera modules, carry both. The
  Lunar's dump shows its DNGPrivateData, so the dumps show the tag when a
  file has it. **[P]**
- **The nine non-CC0 Sony files**, all CC BY-NC-SA imports from
  rawsamples.ch, with their ids: DSLR-A300 (339), DSC-RX10M2 (340), NEX-3
  (348), DSLR-A580 (350), DSLR-A330 (355), DSLR-A850 (356), NEX-5R (359),
  NEX-7 (360) and DSLR-A550 (362). Never commit them and never use them in
  CI.
- **Bodies under `data/Sony/`:** DSC-F828, DSC-HX95, DSC-HX99, DSC-R1,
  DSC-RX0, DSC-RX0M2, DSC-RX1, DSC-RX1R, DSC-RX1RM2, DSC-RX10 to RX10M4,
  DSC-RX100 to RX100M7 (with M5A); DSLR-A100, A200, A230, A290, A300, A330,
  A350, A380, A390, A450, A500, A550, A560, A580, A700, A850, A900;
  ILCA-68, ILCA-77M2, ILCA-99M2; ILCE-1, ILCE-7, 7C, 7CM2, 7CR, 7M2, 7M3,
  7M4, 7R, 7RM2, 7RM3, 7RM3A, 7RM4, 7RM4A, 7RM5, 7S, 7SM2, 7SM3; ILCE-9,
  9M2, 9M3; ILCE-3000, 3500, 5000, 5100, 6000, 6001, 6100, 6300, 6400,
  6500, 6600, 6700; ILCE-QX1; ILME-FX3, ILME-FX30; NEX-3, 3N, 5, 5N, 5R,
  5T, 6, 7, C3, F3; SLT-A33, A35, A37, A55, A57, A58, A65, A77, A99;
  UMC-R10C; ZV-1, ZV-E1, ZV-E10.
- **Bodies with several modes:**
  - nine files each, covering uncompressed, compressed and lossless L, M
    and S in full frame and APS-C: ILCE-1M2, ILCE-7M4, ILCE-7RM5,
    ILCE-7CR, ILCE-9M3;
  - ILCE-7M5 (A7 V): six files, ids 8845 to 8850, Compressed, Compressed
    (HQ) and Lossless in full frame and APS-C, all ARW 6.0;
  - ILCE-7RM3: nine files, 12- and 14-bit compressed and uncompressed,
    four pixel-shift frames and the only ARQ;
  - ILCE-7RM2: seven files, 12- and 14-bit compressed and uncompressed;
  - DSC-RX1RM3 and ILME-FX2: five files each, with lossless L, M and S;
  - DSC-RX100M6: four aspect ratios (1:1, 3:2, 4:3, 16:9);
  - ILCE-1: compressed, lossless and uncompressed.
- **Missing** (the site's own list): **ILCE-7RM6** (A7R VI, the second ARW
  6.0 body), **DSC-V3** (the second SRF body), DSC-RX10M5, NEX-VG20,
  NEX-VG30, NEX-VG900, ILCE-6100A, ILX-LR1, ZV-1A and ILME-FX3A. The
  Sony-based Hasselblad **Stellar** and **HV** have no sample either.
- The missing list also names SLT-A55V and SLT-A99V, but the files
  catalogued as SLT-A55 and SLT-A99 carry the Model strings `SLT-A55V` and
  `SLT-A99V` (files 964 and 3751), so those two are naming artefacts.
  **[F]**

## 4. Recommended minimal body set

Every file below is CC0 in the catalogue and was checked (see
[Files checked](#files-checked)).
"FileFormat" is the MakerNote tag 0xB000 as read from the file; ExifTool
names `3 3 1 0` ARW 2.3.1, and so on. "Why" says what the file exercises.

| FileFormat (ARW) | Body | Files | Why |
|---|---|---|---|
| none (SRF) | DSC-F828 | 1351 | IFD0 is the raw; the only JPEG is in IFD1; the SRF key chain; 14-bit data |
| `1 0 0 0` (SR2) | DSC-R1 | 3221 | SR2 keystream with a key other than the usual one; big-endian 14-bit raw in a little-endian file; GRBG pattern |
| `2 0 0 0` (1.0) | DSLR-A100 | 2346 | `SONY DSC` MakerNote header; 0x014A as a data pointer; Minolta block; ARW1 stream; 640×480 preview |
| `3 0 0 0` (2.0) | DSLR-A200, DSLR-A700 | 2271, 2521 | ARW1 coding under Compression 32767; 8-bit cRAW; thumbnail length that runs past EOI |
| `3 1 0 0` (2.1) | DSLR-A900 | 5858 | Packed 12-bit (1.5 bytes per pixel) |
| `3 2 0 0` (2.2) | NEX-5 | 1457 | MakerNote values four bytes after the IFD; tone-curve knots unlike all others; 51 bytes of padding after the preview's EOI |
| `3 3 0 0` (2.3) | DSC-RX100, Hasselblad Lunar | 5811, 4051 | Black level 800 (1-inch sensor); non-Sony Make with a headerless MakerNote |
| `3 3 1 0` (2.3.1) | SLT-A99, ILCE-7S, ILCE-7RM2 | 3751, 1582, 1311 | The only rotated sample (Orientation 8); the smallest file (APS-C crop, 14-bit cRAW); 12-bit uncompressed stored at 14-bit scale |
| `3 3 3 0` (2.3.3) | ILCE-9, DSC-RX100M6 | 1542, 3015 | 14-bit uncompressed; first plain 0x7310 and 0x7313; a 16:9 crop with a 1920×1080 preview |
| `3 3 5 0` (2.3.5) | ILCE-7RM3 | 3251 (and ARQ 3255, large tier) | Pixel-shift frame; 0x74C7 and DefaultCropOrigin disagree; the ARQ layout |
| `4 0 0 0` (4.0) | ILCE-7SM3, ILCE-1 | 5418, 4466 | The smallest FileFormat 4.0 file with an IFD2 full-size JPEG (the smallest checked file with an IFD2 is 8849, 11.5 MB); lossless L tiles |
| `4 0 1 0` (4.0.1) | ILCE-7M4 | 6930, 6931, 6936 | Lossless M (YCbCr 4:2:0) and S (4:2:2); 14-bit cRAW in APS-C |
| `5 0 0 0` (5.0) | ILCE-9M3 | 7361 | ARW 5.0 cRAW |
| `5 0 1 0` (5.0.1) | ILCE-1M2 | 7835 | ARW 5.0.1; an enciphered 0x9050 block that is mostly `FF` (219 of 256 bytes, `FF` at the count offsets) |
| `6 0 0 0` (6.0) | ILCE-7M5 | 8847, 8849, 8848 | Compression 32766 in Compressed and Compressed (HQ), to detect and refuse; lossless tiles in an ARW 6.0 file |

These 25 files total about 690 MiB; the ARQ adds 325 MiB. **[F]**

For **preview-only tests** (milestone 2), eleven files are enough, about
220 MiB: 1351, 3221, 2346, 2521, 1457, 3015, 1582, 5418, 4466, 7364 and
8850. They cover the SRF and SR2 layouts, the A100, padding after EOI, a
non-standard preview size, IFD2 on ARW 4.0, raw tiles that begin like a
JPEG (4466), a YCbCr raw IFD that a preview finder must skip (7364) and an
ARW 6.0 file. **[F]**

Not on raw.pixls.us: an ILCE-7RM6 file (in dnglab's RawDB; check the
file's licence first) for the tiled ARW 6.0 layout, and a DSC-V3 SRF.

## 5. Building fixtures

### 5.1 Tiny valid fixtures

Write the fixture generator as a test helper or small script in the
repository, not a binary blob nobody can regenerate. Where a binary fixture
is committed, a text file next to it records how it was made, the source
sample's id, URL, SHA-256 and licence as checked (for a derived fixture),
the date, and what it exercises, as on the
[NEF page](../nef/test-data.md). The ARW-specific pieces:

1. **The IFD chain, per generation.** Every checked file is little-endian
   with IFD0 at offset 8. **[F]**
   - Before ARW 4.0: IFD0 with NewSubfileType 1, Compression 6, the
     preview in 0x0201 and 0x0202, one SubIFD offset, the Exif IFD and
     0xC634 (BYTE, count 4); IFD1 with a thumbnail in 0x0201 and 0x0202;
     IFD1's next pointer 0.
   - From ARW 4.0: the same, plus an IFD2 reached from IFD1's next pointer,
     with NewSubfileType 1, Compression 7, Photometric 6, BitsPerSample
     8,8,8, SamplesPerPixel 3, YCbCrSubSampling 2,1 and a baseline JPEG in
     0x0201 and 0x0202; its next pointer 0.
   - The raw SubIFD: NewSubfileType 0, Photometric 32803, a CFAPattern,
     and a strip of a few dummy bytes. Keep the Sony tags out of ascending
     order as real files do (0x011A, 0x7000, 0x0111 …) so a reader that
     requires sorted entries fails. **[F]**
   - An SRF shape (IFD0 is the raw and has no StripOffsets; the only JPEG
     is in IFD1) and an A100 shape (MakerNote with the `SONY DSC \0\0\0`
     header, 0x014A holding a data offset rather than an IFD, MakerNote
     tag 0xB028 pointing at a Minolta block that references the IFD0
     preview a second time). **[F]**
2. **Preview JPEGs.** Small baseline JPEGs that start `FF D8 FF DB`, with
   no APP segment, as every embedded Sony JPEG checked does. **[F]** Put
   the largest in a different IFD in different fixtures, so tests prove
   the choice is by measured size. Give one a declared length that runs a
   few bytes past EOI, as the A-mount thumbnails and the NEX-5 and A55
   previews do. **[F]**
3. **Lossless tiles.** A tiled raw IFD (Compression 7, 512×512 tiles,
   TileOffsets and TileByteCounts, plus StripOffsets and StripByteCounts
   that cover the same bytes as real files have) whose tiles begin
   `FF D8 FF C3`. A preview finder must not take them. Each tile is a SOF3
   lossless JPEG; real L tiles are 256×256 with four components, precision
   14 and predictor 1, and real M and S tiles are 512×512 YCbCr with
   precision 16 while BitsPerSample says 15. **[F]** Store the tiles column
   by column while TileOffsets lists them row by row, as file 4466 does, so
   a decoder cannot assume ascending offsets. **[F]** A lossless JPEG
   encoder with predictor 1 is short enough to write in the test helper
   from ITU-T T.81.
4. **SR2Private with the keystream.** IFD0's 0xC634 points at the
   SR2Private IFD (0x7200 offset, 0x7201 length, 0x7221 key), which locates
   the encrypted SR2SubIFD holding 0x7310, 0x7313 and 0x787F. Build the
   SR2SubIFD in plain form, then encrypt it with the keystream described in
   [makernote-and-metadata.md](makernote-and-metadata.md). Offsets inside
   it are file offsets, and it has no next-IFD pointer. **[F]**
   - Use the key bytes `11 22 33 44`, which every ARW checked carries, in
     one fixture and a different key in another (the DSC-R1 file 3221 uses
     one), so the code cannot hard-code the key. **[F]**
   - The cipher is its own inverse, so a generator and a reader that share
     one routine would agree even if both were wrong. Add a known-answer
     test: a short run of encrypted bytes, the key and the plain bytes
     taken from a CC0 sample, with its id and hash in the provenance note.
5. **MakerNote variants.** Headerless, with the value data starting right
   after the entries (most bodies) and four bytes after them (A-mount
   DSLRs, early NEX and SLT, the DSC-R1); FileFormat values for each
   generation; Make `HASSELBLAD` with SonyModelID 289 (the Lunar). **[F]**
   An enciphered 0x9050 block with a known shutter count can be made with
   the forward map, b³ mod 249, so the generator needs no table.
6. **cRAW blocks.** A strip of a few 32-pixel groups, each two 128-bit
   blocks, built by an encoder in the test helper (11-bit maximum and
   minimum, 4-bit positions, fourteen 7-bit differences; see
   [raw-data.md](raw-data.md)), plus the four 0x7010 knots. Cover the
   maximum and minimum at the first and last positions, every shift from 0
   to 4, and values near 2047, with expected outputs worked out by hand.
   The width is a multiple of 32, as in every cRAW file checked. **[F]**
7. **ARW 6.0 detection.** A raw IFD with Compression 32766 and a short
   strip that starts like a real one (a tile count of 1, a tile record
   pointing at offset 512, a stream beginning `A000`), plus an IFD2
   preview. The test expects the preview and a typed unsupported error for
   the raw data, never a decode. **[F]** (layout)
8. **ARQ.** A raw IFD with SamplesPerPixel 4 and Photometric 32892, which
   the preview finder must skip by role, and MakerNote 0x202F PixelShiftInfo.
   **[F]**
9. **Stripped real files.** Offsets are absolute everywhere in an ARW,
   inside the MakerNote and the SR2SubIFD too, so stripping pixel data
   means rewriting offsets: the stripping helper is in effect a small TIFF
   rewriter. Getting those offsets right is what such a fixture tests.
   **[F]** (absolute offsets)

### 5.2 Corrupted variants

Everything in section 5.2 of the [NEF page](../nef/test-data.md) applies.
ARW adds the cases below; [security.md](security.md) gives the history
behind each. Derive them from valid fixtures so each breaks one rule, and,
as CONTRIBUTING.md asks of regression tests, see each test fail before
relying on it. Every one must produce an error, never a panic
([ADR008](../../architecture-decisions/adr008-lints-and-panics.md)).

- An IFD0 chain that loops (IFD2 back to IFD0), or that is longer than any
  real file's (real chains end after IFD1 or IFD2). **[F]**
- 0x0201 and 0x0202 pointing at a raw tile that starts `FF D8 FF C3`; an
  IFD that claims Compression 7, Photometric 6 and 8 bits but holds SOF3
  data.
- 0xC634 with a type other than BYTE or a count other than 4; SR2Private
  offset plus length past the end of the file; a length of 0, not a
  multiple of 4, or huge; a decrypted IFD whose offsets point before the
  block or past its end; an entry count of 0 or above 1000; a wrong key,
  which yields garbage that must be ignored.
- A truncated enciphered 0x9050 or 0x94xx block, shorter than the fixed
  offsets read from it; a `SONY DSC` header with nothing after it.
- The A100's 0x014A data pointer past the end of the file; Minolta block
  lengths that run past the MakerNote.
- An SRF whose key position (MakerNote start plus 0x8DDC) is past the end
  of the file, or whose chain of SRF IFDs points backwards or to itself.
- cRAW with a width under 32 or not a multiple of 32, a strip shorter than
  width times height, a block whose maximum and minimum share a position,
  or whose maximum is below its minimum.
- Lossless tiles: a tile count that does not match the image size divided
  by the tile size, a tile size of 0, overlapping tile ranges, a tile
  shorter than its JPEG header, a malformed DHT segment.
- An ARQ strip shorter than 8 × width × height bytes; SamplesPerPixel 4
  without Photometric 32892.
- An ARW 6.0 strip with a huge tile count or a tile record outside the
  strip (the error must be the same typed refusal or a parse error, never a
  read out of bounds).

## 6. How samples answer the open questions

The list of unresolved facts is in [open-questions.md](open-questions.md).
The first rows were settled by the files checked; the rest name the files
that would settle them, mostly with a dump of the tags and a read of a few
bytes. **[I]** unless marked.

| Question | Sample files | State |
|---|---|---|
| Where the largest preview lives, per generation | All 36 | Settled: IFD0 (about 1616×1080) before ARW 4.0, a full-size JPEG in IFD2 from ARW 4.0 on (16 files) **[F]** |
| Whether the A7 V's Compressed and Compressed (HQ) differ in codec, and whether the catalogue swapped their labels | 8846 to 8850 | Settled: both Compression 32766; Quality 4 or 5 tells them apart; RAWFileType is 3 on both; labels not swapped, HQ files are smaller **[F]** |
| Whether ARW 1.0 is compressed, and the A100's byte order | 2346 | Settled: an ARW1 stream in a little-endian file **[F]** |
| How 12-bit uncompressed data is scaled | 1311 | Settled: stored at 14-bit scale **[F]** |
| The ARQ layout | 3255 | Settled: four samples per pixel, Photometric 32892, R-G-G2-B **[F]** |
| Whether any ARW declares TIFF/EP or DNG conformance | All 36 | Settled: none does **[F]** |
| Where the SRF2 to SRF6 IFDs are and which key decrypts them | 1351 | Open: the IFD at SRF1's next pointer did not decrypt with any key tried |
| The DSC-F828's colour filter (RGBE) | 1351 | Open: decode the decrypted image and compare the four channel means |
| The ARW 6.0 header fields beyond width and half-height; the four-tile layout | 8846 to 8850; ILCE-7RM6 files from dnglab's RawDB after a licence check | Open: the A7 V full-frame files have one tile, so the four-tile layout is probably the A7R VI's |
| CameraOrientation in deciphered 0x9400 | 3751 | Settled for the 9400a layout: byte 0x28 says 8, as IFD0 does **[F]**; the 9400b and 9400c offsets are unchecked on a rotated file |
| The 0x9050 layout on ILCE-9M3, ILCE-6700 and ILCE-1M2 | 7361, 7364, 6736, 7835 and the other files of those bodies | Open: the deciphered values at the known offsets are not shutter counts |
| Which other A-mount DSLRs write ARW1 streams under Compression 32767 | DSLR-A230, A290, A350, A380, A390 (CC0); A300 and A330 are not CC0 | Open: the A200 does (2271) **[F]** |
| 12-bit fallback in silent or continuous shooting | ILCE-6500 silent 12-bit and 14-bit files; the ILCE-6300 pair | Open: not downloaded |
| The ILCE-7's bit depth and codec | The ILCE-7 file | Open: its label and file name disagree |
| Lossless M and S on other bodies | M and S files of ILCE-1M2, ILCE-7RM5, ILCE-7CR, DSC-RX1RM3, ILME-FX2 | Open: checked on the ILCE-7M4 and ILCE-9M3 only |
| The A7 V full-frame lossless file | 8845 | Open: not downloaded |
| Which bodies and firmware write OffsetTime\* | One file per body at FileFormat `3 3 2 0` to `3 3 5 0` (for example DSC-RX10M3, ILCE-6500, ILCE-7M3, ILCE-9M2), read from the bytes because the dumps under-report it | Open: absent on ILCE-7S firmware 3.20 (1582), present on ILCE-9 firmware 1.00 (1541) **[F]** |
| How files edited by Sony's software differ (tag 0x7240) | None on the site; needs a CC0 file edited in Imaging Edge | Open |
| The A100 firmware 1.00 thumbnail offset | An A100 file on firmware 1.00 | Open: 2346 is firmware 1.04 |
| Whether cRAW values exceed 16383 after the curve | A cRAW file of a clipped highlight | Open: no file checked reaches code 2047 |
| Whether decoded pixels match a reference decoder | Any of the files checked | Open: no pixel output was compared |

Findings from these checks should be written back into the ARW pages with
the file id, its hash and what was measured.

## 7. Open questions about test data

- **Whether the catalogue's licence field can be relied on.** It is the
  only per-file licence statement; check whether it can change after
  upload by comparing `set=noncc0` at intervals.
- **Whether raw.pixls.us ids and download URLs are stable.** CI pins by
  hash, but a moved file would turn into a skipped test; compare the
  catalogue's URLs with `filelist.sha256` paths at intervals.
- **The licence of dnglab's ILCE-7RM6 and DSC-RX10M5 files**, which only
  the RawDB application shows; they are the only public source for the
  tiled ARW 6.0 layout.
- **Whether file 8845** (A7 V, full frame, lossless) has the same
  structure as the APS-C lossless file 8848, so it can join the minimal
  set.

## Files checked

These 36 files were downloaded from raw.pixls.us in October 2026 and used
for the sample checks behind the [F] tags in the ARW pages. All are CC0 in
the catalogue. Each file's SHA-256 matched the catalogue before any
parsing, and its size matched the catalogue's MiB figure. All are
little-endian TIFF with IFD0 at offset 8. **[F]**

The checks used throwaway scripts written from the prose of these pages: a
bounds-checked TIFF walker, a JPEG marker walker, the SR2 keystream, the
MakerNote cipher, a cRAW block decoder and tone-curve builder, an ARW1
decoder, a lossless tile header reader and the SRF key chain. They are not
in the repository, and no code was taken from another project. No external
decoder or ExifTool was run on the files, so no result depends on another
implementation, and no pixel output was compared with a reference
decoder. The ARQ (3255) was deleted after measurement.

"Mode" is the catalogue's label, with its remark where that adds the crop
or size; "Bytes" is the file size.

| Id | Body | Mode | Bytes | Used to check |
|---|---|---|---|---|
| 964 | SLT-A55 | 12-bit compressed | 16,973,824 | Model string `SLT-A55V`; MakerNote values four bytes after the IFD; 51 bytes after the preview's EOI |
| 1310 | ILCE-7RM2 | 12-bit compressed | 43,286,528 | 12-bit cRAW; RAWFileType 0; cipher shutter count; catalogue megapixel column (42.56) |
| 1311 | ILCE-7RM2 | 12-bit uncompressed | 85,852,160 | 12-bit data at 14-bit scale; SonyRawFileType contradicts ExifTool's meaning; no plain 0x7310 |
| 1351 | DSC-F828 | SRF, 4:3 | 17,393,344 | SRF layout; key chain and DataKey; decrypted image is 14-bit; IFD1 640×480 JPEG |
| 1457 | NEX-5 | 12-bit compressed | 14,811,136 | ARW 2.2; unusual 0x7010 knots; padding after the preview's EOI |
| 1541 | ILCE-9 | 14-bit compressed | 24,890,112 | First plain 0x7310 and 0x7313; OffsetTime\*; 14-bit cRAW; cipher shutter count |
| 1542 | ILCE-9 | 14-bit uncompressed | 49,223,168 | 14-bit little-endian uncompressed; crop pairs; ColorMatrix 0x7800; FullImageSize |
| 1582 | ILCE-7S | 14-bit compressed, APS-C crop | 6,160,384 | Smallest file; 14-bit cRAW; APS-C keeps the SonyModelID; manual-lens LensModel |
| 2271 | DSLR-A200 | 12-bit compressed | 11,418,600 | ARW1 stream under Compression 32767 with 8 extra rows; Make with a trailing space; black level 4 |
| 2346 | DSLR-A100 | compressed | 10,137,572 | `SONY DSC` MakerNote header; 0x014A data pointer; little-endian Minolta block; ARW1 decode at 3881 columns |
| 2521 | DSLR-A700 | 8-bit compressed | 12,967,424 | cRAW in ARW 2.0 with BitsPerSample 8; thumbnail length is a slot size; 0x787F with count 1 |
| 3012 | DSC-RX100M6 | 12-bit compressed, 1:1 | 20,673,536 | 0x74C7 order (left, top); 1088×1080 preview |
| 3015 | DSC-RX100M6 | 12-bit compressed, 16:9 | 20,833,280 | 1920×1080 preview; aspect crop only in the Sony crop tags, Exif size and preview |
| 3221 | DSC-R1 | SR2, 14-bit uncompressed | 21,035,488 | SR2 keystream with its own key; big-endian raw words; GRBG; 0x7300 and 0x7303 |
| 3251 | ILCE-7RM3 | 14-bit uncompressed, pixel-shift frame | 85,513,216 | FileFormat `3 3 5 0` on firmware 3.00; 0x74C7 differs from DefaultCropOrigin; 0x7240 plus 6 points at the raw SubIFD |
| 3255 | ILCE-7RM3 | ARQ | 340,873,216 | Four samples per pixel, Photometric 32892, R-G-G2-B order; container identical to file 3251; PixelShiftInfo |
| 3751 | SLT-A99 | 12-bit compressed | 25,395,200 | Model string `SLT-A99V`; FileFormat `3 3 1 0` on firmware 1.02; the only Orientation 8 |
| 4051 | Hasselblad Lunar | 12-bit | 24,870,912 | Make `HASSELBLAD`; headerless MakerNote; SonyModelID 289; no "SONY" string |
| 4466 | ILCE-1 | 14-bit, lossless | 81,575,280 | IFD2 JPEG 8640×5760; SOF3 tiles stored column by column; 0x7038; RAWFileType 2; BodySerialNumber |
| 5418 | ILCE-7SM3 | 14-bit compressed | 15,468,544 | First FileFormat `4 0 0 0`; IFD2 JPEG 4240×2832; no 0x2010; SubSecTime\* |
| 5811 | DSC-RX100 | 12-bit compressed | 21,155,328 | FileFormat `3 3 0 0`; black level 800; MakerNote values right after the IFD |
| 5858 | DSLR-A900 | 12-bit compressed | 37,933,568 | Packed 12-bit, 1.5 bytes per pixel, little-endian nibble order; 0x7310 must be divided by 4 for this data |
| 6736 | ILCE-6700 | 14-bit, lossless | 43,720,704 | IFD2 JPEG 6192×4128; 117 tiles; 0x9050 layout unclear |
| 6929 | ILCE-7M4 | 14-bit, lossless L | 49,184,768 | IFD2 JPEG 7008×4672; 140 tiles; 0x7038 and DefaultCrop; Quality; HiddenInfo; cipher shutter count |
| 6930 | ILCE-7M4 | lossless M | 32,804,864 | YCbCr 4:2:0 raw; SOF3 precision 16 against BitsPerSample 15 |
| 6931 | ILCE-7M4 | lossless S | 22,933,504 | YCbCr 4:2:2 raw; IFD2 JPEG 3504×2336 |
| 6933 | ILCE-7M4 | 14-bit uncompressed, APS-C | 34,177,024 | 0x7010 all zero on ARW 4.0 uncompressed; APS-C keeps the SonyModelID |
| 6936 | ILCE-7M4 | 14-bit compressed, APS-C | 19,636,224 | 14-bit cRAW on ARW 4.0; Quality 3 on a Bayer APS-C file |
| 7361 | ILCE-9M3 | 14-bit compressed | 29,077,504 | FileFormat `5 0 0 0`; IFD2 JPEG 6000×4000; 0x9050 layout unclear |
| 7364 | ILCE-9M3 | lossless S | 16,367,616 | YCbCr 4:2:2 raw, 24 tiles; IFD2 JPEG 3008×2000 |
| 7835 | ILCE-1M2 | 14-bit lossy, APS-C | 22,757,376 | FileFormat `5 0 1 0`; IFD2 JPEG 5616×3744; 0x9050 block mostly `FF` (219 of 256 bytes) |
| 8846 | ILCE-7M5 | 14-bit, full frame, compressed HQ | 23,232,512 | Compression 32766; one 7040×4688 tile; stream tag `A000`; Quality 5; 0xC61D 16383 |
| 8847 | ILCE-7M5 | 14-bit, full frame, compressed | 23,314,432 | Quality 4; RAWFileType 3 as on the HQ file |
| 8848 | ILCE-7M5 | 14-bit, APS-C, lossless | 22,233,088 | Lossless Compression 7 tiles in an ARW 6.0 file; 70 tiles |
| 8849 | ILCE-7M5 | 14-bit, APS-C, compressed HQ | 11,481,088 | ARW 6.0 header for 4640×3088; HQ file smaller than the standard one |
| 8850 | ILCE-7M5 | 14-bit, APS-C, compressed | 12,103,680 | Quality 4; 0x9050 shutter count at a different offset |

The full hashes and download URLs:

- 964 `f861454a49abe0345ff69887874d00617cdf0b96af96d605051e68deda424a08`
  https://raw.pixls.us/getfile.php/964/nice/Sony%20-%20SLT-A55%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 1310 `a606ebf1dcc6704f03f2966326c60a87dd6bdb3704464b3897c10349a10d3a19`
  https://raw.pixls.us/getfile.php/1310/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 1311 `71e0888396ef52c7e6a990b4e72ebb4d8413a1fc1c35bada449870539cef6e39`
  https://raw.pixls.us/getfile.php/1311/nice/Sony%20-%20ILCE-7RM2%20-%2012bit%2012bit%20uncompressed%20(3:2).ARW
- 1351 `07739f94bcab0c7e198d418ac65172d0dce04bb84a7683ecffd0bfe4b32f6d79`
  https://raw.pixls.us/getfile.php/1351/nice/Sony%20-%20DSC-F828%20-%204:3.SRF
- 1457 `3b085569ac1fec37a1289ed5ea517497692a3ea14cea41953dd6e878abf230a3`
  https://raw.pixls.us/getfile.php/1457/nice/Sony%20-%20NEX-5%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 1541 `598bbc828c279528f504c51a10418521b17a90a25b198368f65e19cfda9e3711`
  https://raw.pixls.us/getfile.php/1541/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20compressed%20(3:2).ARW
- 1542 `22c2158c9796386dba28d7cf040a1ca70f4ec2c927ec50771b78bfb523e21c29`
  https://raw.pixls.us/getfile.php/1542/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW
- 1582 `a35ebb2fbec929daa5beb20d1ce5c15a8aac7b1a7a231455387f3df8a7442e07`
  https://raw.pixls.us/getfile.php/1582/nice/Sony%20-%20ILCE-7S%20-%2014bit%2014bit%20compressed%20(3:2).ARW
- 2271 `04eacd403d92e0c90da9d91b87615af8eda1afc963350d3ab774557c36763eef`
  https://raw.pixls.us/getfile.php/2271/nice/Sony%20-%20DSLR-A200%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 2346 `4df7981ac4baec9648b4f106751ea4b09a6e27bc70a15f8ef09bec051fec362d`
  https://raw.pixls.us/getfile.php/2346/nice/Sony%20-%20DSLR-A100%20-%20compressed%20(3:2).ARW
- 2521 `3159e28892bf0771d01525cd4b6190f9c15bbb19fa2fab6d2515ced0594e8f40`
  https://raw.pixls.us/getfile.php/2521/nice/Sony%20-%20DSLR-A700%20-%208bit%208bit%20compressed%20(3:2).ARW
- 3012 `53c5abad853f17d2c46d9f96f8ff4e0af0ea095193fecf61222a8bf9d8beb00b`
  https://raw.pixls.us/getfile.php/3012/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(1:1).arw
- 3015 `fe9e4df604fefbcc0ed78fb4a1d9a58dbaa1f4011e3d119b3ee32fc583d43907`
  https://raw.pixls.us/getfile.php/3015/nice/Sony%20-%20DSC-RX100M6%20-%2012bit%2012bit%20compressed%20(16:9).arw
- 3221 `921c5f2513dd1671a9089e73fc778fc3573bc4cb70a3a555d0414661d0e62274`
  https://raw.pixls.us/getfile.php/3221/nice/Sony%20-%20DSC-R1%20-%2014bit%2014bit%20uncompressed%20(3:2).SR2
- 3251 `eaedff10f11b7bdd7908db40d8e1f541bd29a89369b897f181b87c29d549f0a4`
  https://raw.pixls.us/getfile.php/3251/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW
- 3255 `67af1f9f4cff82bd182f3fc48e95d39ae7d8352595e3c72f3386c9dee114f27d`
  https://raw.pixls.us/getfile.php/3255/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARQ
- 3751 `bc033b8b0df5b1ef603ead4b57f979b8a0a46e7a52d8b46ac6dfa22f86c243f8`
  https://raw.pixls.us/getfile.php/3751/nice/Sony%20-%20SLT-A99%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 4051 `029b1283a3d1226e08fc1eb3b81874347f5c128e437f78f853436fd6ba15873b`
  https://raw.pixls.us/getfile.php/4051/nice/HASSELBLAD%20-%20Lunar%20-%2012bit%20(3:2).ARW
- 4466 `4012536ee0f938d8c5c5e932bb7953a03134abbd9f3f4c664cb2526c1903d492`
  https://raw.pixls.us/getfile.php/4466/nice/Sony%20-%20ILCE-1%20-%2014bit%20(4:3).ARW
- 5418 `17e35d98d448b44164ecf414f980e9248b5829233e83a1adbee396b6c1a1a0c6`
  https://raw.pixls.us/getfile.php/5418/nice/Sony%20-%20ILCE-7SM3%20-%2014bit%2014bit%20compressed%20(3:2).ARW
- 5811 `579a485b5126a25cbd55cbd5dadfa7d09cf021c99cc7d4869f9e56e3f759390b`
  https://raw.pixls.us/getfile.php/5811/nice/Sony%20-%20DSC-RX100%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 5858 `ac7c1532df77c321e8010aa1be60c1b9f245b1ac0db6f38fe617a73aae81af50`
  https://raw.pixls.us/getfile.php/5858/nice/Sony%20-%20DSLR-A900%20-%2012bit%2012bit%20compressed%20(3:2).ARW
- 6736 `dc3e2ed22f46fcef778553767e0703bc37cfb043f107afe2d7e6e4dbca34200a`
  https://raw.pixls.us/getfile.php/6736/nice/Sony%20-%20ILCE-6700%20-%2014bit%20(4:3).ARW
- 6929 `851b43c2116c4139104a5036f83ac3b6a148789b2142214dd7192c13972b25b6`
  https://raw.pixls.us/getfile.php/6929/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(4:3).ARW
- 6930 `d453005714327addd75bcb99c1c6223173dc92f2a59542d82e6761ef0e0e7571`
  https://raw.pixls.us/getfile.php/6930/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW
- 6931 `cbbd0930c7d8706dff84c68a2004454266e6fd0d8354f5f76a106b5d776e0223`
  https://raw.pixls.us/getfile.php/6931/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW
- 6933 `bc0089c821a3711ebbeec3844cde5e5c247ac7cf9ca8ad9ae03a977b232216b2`
  https://raw.pixls.us/getfile.php/6933/nice/Sony%20-%20ILCE-7M4%20-%2014bit%2014bit%20uncompressed%20(3:2).ARW
- 6936 `639a6d4db881f1359e3ea7e1137b314e59417d24e6d4d862b7202111a2c823b2`
  https://raw.pixls.us/getfile.php/6936/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(3:2).ARW
- 7361 `81b0b140f34356964fdb4adf5e202eb01d5e5e5a1b61b861954a183fc30d4bf0`
  https://raw.pixls.us/getfile.php/7361/nice/Sony%20-%20ILCE-9M3%20-%2014bit%20(3:2).ARW
- 7364 `3d9da593a027bdbf31589eb950b04be7ce3f3dc6105ca339c5b7702ffa7fefdd`
  https://raw.pixls.us/getfile.php/7364/nice/Sony%20-%20ILCE-9M3%20-%203:2.ARW
- 7835 `050c924e5f78afd8479aa442f17b8650e0a5ebfa5757e57368cb0dc865ca2417`
  https://raw.pixls.us/getfile.php/7835/nice/Sony%20-%20ILCE-1M2%20-%2014bit%20Lossy%20(3:2).ARW
- 8846 `4e3a99cbcf2a348decaf3ca68c5de22b7a05eaa590f4e9184735c1f9c45213a7`
  https://raw.pixls.us/getfile.php/8846/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
- 8847 `46d73475dfd8f96fd7e2f4d4a721a36cbbaba3d6ea95a4851809cd9d252b2a80`
  https://raw.pixls.us/getfile.php/8847/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
- 8848 `51ad709cba01a5f05e3c723b779f6b5fda8fa7b378494dbb40b4d3d3aa5494a4`
  https://raw.pixls.us/getfile.php/8848/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(4:3).ARW
- 8849 `c16cae99d26a2f8ad183d31a23823310a994aa058d55cfe56e4f82ba56865f57`
  https://raw.pixls.us/getfile.php/8849/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW
- 8850 `8315af2a9889709f4c836dbaadc23d226847e5a0630fda212a9f896d5fe0569f`
  https://raw.pixls.us/getfile.php/8850/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW

## Sources

[1] raw.pixls.us home page, upload terms and JSON catalogue —
    https://raw.pixls.us/ ,
    https://raw.pixls.us/json/getrepository.php?set=all ,
    https://raw.pixls.us/json/getrepository.php?set=noncc0 ,
    https://raw.pixls.us/json/getrepository-missing.php?set=missing **[P]**
[2] raw.pixls.us directory listings — https://raw.pixls.us/data/Sony/ ,
    https://raw.pixls.us/data/SONY/ , https://raw.pixls.us/data/HASSELBLAD/
    **[P]**
[3] raw.pixls.us hash list — https://raw.pixls.us/data/filelist.sha256
    **[P]**
[4] pixls.us, "New year, new raw samples website" (2017) —
    https://pixls.us/blog/2017/01/new-year-new-raw-samples-website **[S]**
[5] darktable, "rawsamples.ch replacement" (2017) —
    https://www.darktable.org/2017/01/rawsamples-ch-replacement/ **[S]**
[6] dnglab RawDB — https://rawdb.dnglab.org ; `CONTRIBUTE_SAMPLES.md` and
    `rawler/data/testdata/rawdb/` in https://github.com/dnglab/dnglab
    **[P]**
[7] ExifTool, sample images — https://exiftool.org/sample_images.html
    **[P]**
[8] Test data of other projects — https://github.com/Exiv2/exiv2 (`test/data`),
    https://github.com/ianare/exif-py (`tests/resources/raw`),
    https://github.com/drewnoakes/metadata-extractor (`Tests/Data`),
    https://github.com/GraphiteEditor/Graphite/tree/master/libraries/rawkit
    (`tests/tests.rs`) **[P]**
[9] OSS-Fuzz, LibRaw and exiv2 projects —
    https://github.com/google/oss-fuzz/tree/master/projects/libraw ,
    https://github.com/google/oss-fuzz/tree/master/projects/exiv2 **[P]**
[10] Imaging Resource usage policy — https://imaging-resource.com/USAGE.HTM ;
    DxO sample raws —
    https://support.dxo.com/hc/en-us/articles/7618135612957 **[S]**
[11] The 36 sample files listed under [Files checked](#files-checked), by
    id, SHA-256 and URL
    **[F]**
