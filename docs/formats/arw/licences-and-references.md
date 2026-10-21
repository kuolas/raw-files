# ARW: licences and references

Sony publishes no specification for ARW or its relatives SR2, SRF and ARQ,
and almost every mature implementation is under a copyleft licence. This
page records what we may learn from, what we may reuse, where the format is
documented, and which legal questions are still open. Back to
[the ARW overview](README.md).

Confidence tags: **[P]** primary (verified from the primary page, source
code or API), **[S]** secondary (forum, news report, search summary, or a
page read only through a summary), **[I]** inferred, **[F]** measured on a
named sample file. A sample is cited as "raw.pixls.us file" plus its id;
the URL and SHA-256 of each are in
[test-data.md](test-data.md#files-checked). Numbers in brackets point to
the [Sources](#sources) at the end. All pages were retrieved in October
2026.

Status: desk research plus structural checks on 36 CC0 sample files
(October 2026). The checks were made with throwaway scripts that are not in
the repository, and no pixel output was compared with a reference decoder.

**This is not legal advice.** It is a record of what we found and a proposal
for how to stay on the safe side; anything with legal weight needs a lawyer.

## 1. Sony: no public specification

- We found no public specification, white paper or decoding SDK for ARW,
  SR2, SRF or ARQ. [I] Sony's own converters are closed: Imaging Edge
  Desktop, which replaced Image Data Converter, and the "Sony RAW Driver", a
  Windows Vista and 7 codec last revised in 2020. [3] **[S]**
- Sony's developer offering is the Camera Remote SDK, for remote control,
  live view and fetching captured images. Version 2.02.00 (2026-06-10) added
  the ILCE-7RM6; 2.01.00 (2026-02-03) added the ILCE-7M5 and DSC-RX1RM3. It
  is distributed per region on application, under a click-through licence.
  [1] **[P]**
- That licence is limited, non-exclusive and non-transferable, forbids
  reverse engineering, decompiling or disassembling the licensed materials,
  and lets Sony terminate it. [2] **[P]** Nothing in the SDK describes the
  file format. Its terms are contractual, and raw-files neither uses nor
  needs it. [I]
- ARQ files are made on a computer by Sony's Imaging Edge software from
  Pixel Shift Multi Shooting frames, not by the camera. [3] **[S]** The one
  ARQ checked keeps its source frame's container, previews and MakerNote
  byte for byte, with the raw data replaced (raw.pixls.us files 3255 and
  3251). **[F]**
- We found no statement by Sony on opening the format. The OpenRAW campaign
  (2005 to 2006) asked camera makers to document their raw formats; none
  did, and the site went quiet in 2009. [37] **[S]**
- We found no report of Sony legal action (DMCA notice, cease and desist,
  lawsuit) against any ARW decoder. [38] **[S]**

Everything beyond TIFF and Exif is therefore reverse-engineered knowledge,
published by community projects (section 6) and read from sample files.

### 1.1 Registries

| Registry | Entry | Conf. |
|---|---|---|
| PRONOM fmt/191, "Sony ARW RAW Image File 1.x" | A little-endian TIFF header, "SONY" within 512 bytes, then the MakerNote FileFormat entry (tag 0xB000, BYTE, count 4) whose first value byte is 2; last updated 2017-11-28 [4] | P |
| PRONOM fmt/1127, "ARW 2.x" | The same signature with first byte 3 [4] | P |
| PRONOM fmt/1126, SR2 | The same pattern with first byte 1 [4] | P |
| SRF, ARQ | No PRONOM record; the Library of Congress also notes none for SRF [4][5] | P |
| Library of Congress, fdd000241 "Camera Raw Formats" (group description) | Lists `srf` and `arw`; says `image/x-sony-srf` is not registered with IANA; points only to fmt/191 for ARW; page dated "04/09/2024" [5] | P |
| freedesktop shared-mime-info | `image/x-sony-arw`, `image/x-sony-sr2`, `image/x-sony-srf`, matched by extension only, as subclasses of `image/tiff` [6] | P |
| IANA media types | No Sony registration [6] | P |
| Bio-Formats 8.5.0 | No Sony reader (it lists NEF and Minolta MRW) [7] | P |

The PRONOM signatures were written before ARW 4.0 and do not identify
newer files:

- ARW 4.0, 5.0 and 6.0 files write 4, 5 or 6 as the first FileFormat byte
  (raw.pixls.us files 5418, 4466, 6929, 7361, 7835, 8846), so they match
  none of the three records as written. **[F]**
- In those files the 0xB000 entry lies between bytes 5500 and 5584 (5458 to
  5488 in the ARW 2.3.3 and 2.3.5 files 3012, 1541, 3251). That is
  inside the `{0-6144}` window of the records' byte sequences but outside
  the `{0-2048}` that their own signature note states, so the records are
  inconsistent with themselves. [4] **[P]**, offsets **[F]**
- The Hasselblad Lunar ARW contains no "SONY" string at all; its Make is
  `HASSELBLAD` (raw.pixls.us file 4051). No record matches it. **[F]**

raw-files identifies ARW from its own reading of the file, not from these
signatures. [I]

## 2. Standards ARW builds on

| Standard | Relevance to ARW | State and availability | Conf. |
|---|---|---|---|
| TIFF 6.0 (Adobe, 1992) | The container: the IFD chain, the SubIFD, strips, tiles, the Compression tag | Unchanged since 1992; availability as on the [NEF page](../nef/licences-and-references.md) [8] | S |
| TIFF/EP, ISO 12234-2:2001 | CFAPattern and CFARepeatPatternDim in the raw SubIFD | Paid ISO document [9]. None of the 36 files checked declares TIFFEPStandardID (0x9216) or DNGVersion in IFD0 **[F]** | S, F |
| Exif, CIPA DC-008 | The Exif IFD, the MakerNote pointer, OffsetTime\*, BodySerialNumber | Current is **Exif 3.1** (DC-008-Translation-2026), published 2026-01-30; free from CIPA [10] | P |
| ITU-T T.81 (lossless process) | Lossless Compressed raw: Compression 7, tiled, each tile a SOF3 lossless JPEG; also the M and S YCbCr modes | Free from ITU [11]; patents on the 1992 lossless process have long expired [I] | P, I |
| DNG 1.7.1.0 (Adobe) | ARW is not DNG, but borrows DNGPrivateData (0xC634) to point at the SR2Private IFD, and writes DefaultCropOrigin, DefaultCropSize and WhiteLevel (0xC61D) in the raw SubIFD | Free PDF and SDK; patent licence as described on the NEF page [12] | P |
| SMPTE RDD 34:2015, "LLVC – Low Latency Video Codec for Network Transfer" | A possible ancestor of the ARW 6.0 codec (section 5) | Free PDF; a registered disclosure document, not a standard; Sony is the proponent, and IP enquiries go to Sony [13] | P (document), I (link) |
| JPEG XS, ISO/IEC 21122 | A possible technical relative of the ARW 6.0 codec | Paid ISO document; a patent pool exists [36] | I |

## 3. Implementations

"Dependency candidate" and "Read only, no copying" mean what they mean on
the [NEF page](../nef/licences-and-references.md): the first is compatible
with the raw-files `MIT OR Apache-2.0` (see
[ADR002](../../architecture-decisions/adr002-license-mit-or-apache-2.md))
and worth evaluating; the second means we may learn facts from it but copy
neither code nor tables. SPDX identifiers are as reported by each project's
repository, licence file, or crates.io; coverage is what we read in the
source.

| Project | Language | Licence (SPDX) | ARW coverage | Reuse status |
|---|---|---|---|---|
| dcraw 9.28 | C | Custom, no SPDX; the header frees all code outside the Foveon functions "for all uses" (see the NEF page) [14] | SRF, SR2 decryption, ARW1, cRAW, the 0x7010 tone curve; no lossless, no ARW 6.0 | Read only, no copying |
| LibRaw | C++ | LGPL-2.1-only OR CDDL-1.0 per the README; the GitHub API reports NOASSERTION [15] | SRF, SR2, ARW1, cRAW, lossless, M and S YCbCr, ARQ; ARW 6.0 on master only (section 3.4) | Read only, no copying |
| rawspeed | C++ | LGPL-2.1 per GitHub; source headers say 2.1 or any later version [16] | Compression 1, 7 (CFA only) and 32767; SRF and SR2 recognised; no 32766 | Read only, no copying |
| rawler / dnglab | Rust | LGPL-2.1 [17] | SRF, A100, Compression 1, 7 and 32767; 32766 refused as unsupported (the decompressor is an unreachable stub) | Read only, no copying |
| rawloader | Rust | LGPL-2.1 [18] | SRF, A100, Compression 1 and 32767; no lossless | Read only, no copying |
| imagepipe, quickraw | Rust | LGPL-3.0-only, LGPL-2.1 [19] | Pipelines over the above | Read only, no copying |
| libopenraw (Rust since 0.4) | Rust | LGPL-3.0-or-later per crates.io and the source headers; LGPL-3.0 per the GitHub mirror [20] | ARW detection, thumbnails, raw data | Read only, no copying |
| ExifTool 13.59 | Perl | Perl terms per the README; see section 3.1 [21] | The most complete Sony tag tables: FileFormat up to ARW 6.0, SR2Private, SRF, deciphered MakerNote blocks | Read only, no copying |
| exiv2 | C++ | GPL-2.0-or-later [22] | Sony MakerNote, previews | Read only, no copying |
| libexif | C | LGPL-2.1 [23] | No Sony MakerNote module | Not relevant |
| darktable, RawTherapee | C, C++ | GPL-3.0 [23] | Via rawspeed; dcraw-derived code | Read only, no copying |
| rexiv2, `exiftool-rs` 0.9.0 | Rust | GPL-3.0-or-later [19] | Bindings to exiv2; a reimplementation of ExifTool | Read only, no copying |
| metadata-extractor | Java | Apache-2.0 [24] | ARW; Sony MakerNote types 1 and 6; a 256-entry decipher table for 0x9050 and 0x94xx blocks (added 2022) | Permissive; read for facts; the table's provenance is not stated |
| Kim (Ashampoo) | Kotlin | Apache-2.0 [25] | ARW preview from IFD0 tags 0x0201 and 0x0202 only | Permissive, small |
| jpgfromraw (formerly `arwtojpg`) | Rust | MIT [26] | Walks the whole IFD0 chain, keeps the largest JPEG, bounds-checks offsets | Permissive; worth reading for previews |
| imagemeta | Go | MIT [27] | Sony MakerNote decoding; its README aims at ExifTool parity | Permissive label; tables probably follow ExifTool's [I] |
| exif-py (ExifRead) | Python | BSD-3-Clause [28] | A Sony MakerNote module | Permissive |
| rawpy | Python | MIT for the wrapper; bundles LibRaw [29] | Whatever LibRaw does | Not pure Rust; the bundle carries LibRaw's licence |
| rawkit (Graphite) | Rust | MIT OR Apache-2.0 [30] | Single-strip uncompressed, 12-bit cRAW, A100 ARW1 (stops at a `todo!()`); IFD0 preview on master only; no lossless, ARW 6.0, SR2, SRF or ARQ | Permissive label; see section 3.3 |
| arw6_decode (abbradar) | Python | None: no licence file, so all rights reserved [31] | A reference ARW 6.0 decoder with a prose description | Do not read or copy until it is licensed; only its README was read |
| image-rs `tiff` and `image`, `kamadak-exif`, `zune-jpeg`, `nom-exif`, `little_exif` | Rust | As on the NEF page [19] | Generic TIFF, Exif and JPEG; no Sony MakerNote; `image` does not decode ARW | Dependency candidates, as for NEF |

The small Rust crates listed on the NEF page (`rawlib`, `jpgfromraw-lib`,
`fpexif` and the copyleft ones) apply here unchanged. Two more were found:
`rawbit` 0.1.18 (MIT; Sony support not checked) and `gamut-exif` 1.0.0
(MIT OR Apache-2.0; an Exif 3.0 parser, Sony not checked). Both are unvetted.
[19] **[P]** (licences)

All licences above were read from the GitHub API, the crates.io API or the
project's licence file in October 2026. **[P]**

### 3.1 Where sources disagree about a licence

- **ExifTool.** The README and exiftool.org say ExifTool may be used under
  the same terms as Perl itself, the Artistic Licence or the GPL; that is
  `GPL-1.0-or-later OR Artistic-1.0-Perl`. The repository's `LICENSE` file,
  however, is the GPL-3.0 text, and the GitHub API reports GPL-3.0. [21]
  **[P]** The NEF page records the first reading, from the README, and so
  does this page: the README is the author's statement of terms. [I] For
  raw-files the difference does not matter, since either way ExifTool is
  read only.
- **libopenraw.** The NEF page gives "LGPL-3.0 per GitHub", from the GitHub
  mirror's metadata; crates.io and the source SPDX headers say
  LGPL-3.0-or-later. [20] **[P]**
- **rawspeed.** GitHub says LGPL-2.1; the source headers add "or (at your
  option) any later version". [16] **[P]**
- **LibRaw.** The README gives LGPL-2.1 or CDDL-1.0; the GitHub API reports
  NOASSERTION. [15] **[P]**
- **Graphite.** rawkit's crate metadata has said `MIT OR Apache-2.0` since
  its 0.1.0 release (2024-11-03), while the Graphite repository as a whole
  was Apache-2.0 until PR #4208 (2026-09-07) relicensed it under both. That
  PR notes that consent was not unanimous and that some small selections of
  code remain excluded; whether any are in rawkit was not checked. [30]
  **[P]** (PR text), **[I]** (effect on rawkit)

### 3.2 Why "read only" is the rule

The reasons on the [NEF page](../nef/licences-and-references.md) apply
unchanged: ADR002 excludes GPL, LGPL and MPL dependencies, translating
copyleft code into Rust does not escape its licence, and the selection and
arrangement of a tag table is expression even where each tag ID is a fact.

### 3.3 rawkit's provenance

rawkit is the only permissively labelled Rust code that decodes ARW raw
data, so where it came from matters. It lives in the Graphite repository
under `libraries/rawkit`; we read master at commit 7477cd2fab (2026-10-10).
Its one crates.io release is 0.1.0, published 2024-11-03, with about 1,250
downloads. It began as "Raw-rs" in June 2024 (PR #1757) and was renamed in
PR #2088. [30] **[P]**

We compared it with dcraw 9.28. [14][30] **[P]**

- The ARW1 decoder uses dcraw's function names, and its 18-entry constant
  table is the same as dcraw's, value for value. The table expansion, the
  column and row order and the overflow check follow dcraw too.
- The cRAW decoder follows dcraw's routine step for step: the row buffer
  and loop bound, the bit positions of the block fields, the clamp, the
  curve lookup and the correction that interleaves the two colours.
- The tone-curve builder for tag 0x7010 is dcraw's.
- A list of 22 camera makers appears in exactly the order of dcraw's list.
- No source header, README, Cargo metadata, commit message or PR
  description that we read mentions dcraw or LibRaw.
- It does not behave exactly like dcraw. In its ARW1 difference step a
  category of 0 yields −2, where dcraw returns 0 before any sign step; a
  negative difference subtracts 2^len + 1 rather than dcraw's 2^len − 1;
  and its cRAW path takes dcraw's 12-bit value and shifts it left by 2,
  declaring a maximum of 16383. [14][30] **[P]**

So rawkit matches dcraw element for element **[P]**; that it was translated
from dcraw is **[I]**. LibRaw carries the same inherited dcraw functions,
so LibRaw as the direct source cannot be excluded for the decoders, but the
maker list points to dcraw. [I] dcraw's header leaves all code outside the
Foveon functions free for all uses [14] **[P]**, so this is a question of
attribution and of what raw-files may learn from, not a finding of a
licence breach. [I]

For raw-files [I]:

- Reading rawkit's decoders is equivalent to reading dcraw, so rawkit is
  read only, no copying, like dcraw, until an ADR decides the dcraw
  question.
- As a dependency it is weak regardless of provenance. It panics on
  unexpected input (`unwrap`, `assert!`, `todo!()`), stops at a `todo!()`
  on the A100, and allocates its preview buffer from a length read from the
  file without a bound, which
  [ADR008](../../architecture-decisions/adr008-lints-and-panics.md)
  rules out. [30] **[P]**
- Its per-body colour matrices equal the Adobe-derived values in dcraw,
  LibRaw and rawspeed for the four bodies compared (ILCE-7M3, ILCE-6000,
  ZV-E1, ILCE-7RM5); its DSLR-A100 matrix differs from all three by one in
  three entries, source unknown. [30] **[P]** (values), **[I]** (source)
- Its tests download three files (ILCE-7M3, ILCE-7RM4, ILCE-6000) from
  Graphite's server and compare the output with LibRaw's, bit for bit,
  behind an opt-in feature. The licence of those files is not stated. [30]
  **[P]**

### 3.4 ARW 6.0: who decodes it

ARW 6.0 (TIFF Compression 32766) is written by the ILCE-7M5 (A7 V) for
both Compressed and Compressed (HQ), and by the ILCE-7RM6 (A7R VI). [15][16]
[17] **[P]** On the A7 V files checked, the MakerNote Quality tag (4 or 5)
separates the two modes, and Lossless Compressed stays Compression 7
(raw.pixls.us files 8846, 8847, 8849 and 8850; lossless file 8848).
**[F]**

- **LibRaw** decodes it on master only. The decoder arrived through PR
  #824, merged on 2026-07-18; the latest release, 0.22.2 (2026-07-16),
  does not have it. The new file credits its contributor and states that
  the code was generated with the help of a language model, and a
  maintainer commented on the maintenance risk of such contributions. The
  PR compared its output with Adobe DNG Converter 18.3.1. [15] **[P]**
  (merge, file), **[S]** (comparison)
- **rawspeed** PR #972, which describes itself as clean-room work from the
  decoder's observed input and output with substantial AI assistance, was
  closed unmerged on 2026-09-13. [16] **[P]**
- **dnglab** PR #825 is open. It ports arw6_decode, which has no licence,
  with AI assistance; rawler's main branch refuses Compression 32766 as
  unsupported (no camera file defines its "arw6" mode), and its ARW 6.0
  decompressor is an `unimplemented!()` stub that is unreachable today.
  [17][31] **[P]**
- Adobe Lightroom and Camera Raw reportedly read A7 V Compressed and
  Compressed (HQ) from about April 2026. [39] **[S]**

As of October 2026 no released open-source decoder handles ARW 6.0. **[P]**

## 4. Clean-room guidance for raw-files

raw-files decided the working rules of section 4.1 in
[ADR013](../../architecture-decisions/adr013-clean-room-and-third-party-code.md);
the rules for contributors are in
[CONTRIBUTING.md](../../../CONTRIBUTING.md#clean-room-and-third-party-code).
The tables of section 4.2 still wait for a decision of their own.

### 4.1 Facts and expression

The working rules in section 4.1 of the
[NEF page](../nef/licences-and-references.md) apply unchanged: learn facts
from several independent sources, record them in our own words with a
source and a tag, write code from this documentation rather than from
copyleft source, prefer a sample file to any single implementation, and
never paste code, comments or tables from a GPL or LGPL source.

For ARW the sample checks have already settled several disagreements
between implementations, which is the reason to prefer samples:

- the A100 raw width, 3881 columns (dcraw, LibRaw, rawspeed) rather than
  3880 (rawler): 3881 consumes the strip to within 5 bytes, 3880 leaves
  2,485 (raw.pixls.us file 2346) **[F]**;
- the element order of tag 0x74C7, which is (left, top) as LibRaw reads it,
  not (top, left) as libopenraw does (file 3012) **[F]**;
- the "ARW1 under Compression 32767" path that every decoder carries but no
  source tied to a body: the DSLR-A200 writes it (file 2271) **[F]**.

### 4.2 Tables that need a decision

These are the kinds of data an ARW decoder must bring itself, because the
file does not carry them. Unlike NEF, most of them reduce to a rule or a
formula, so no table needs to be copied.

| Data | In the file? | Where it is published | Position for raw-files |
|---|---|---|---|
| ARW1 prefix code (A100, and the ARW1 streams of the A200 class) | No | As an 18-entry table in dcraw, LibRaw and rawkit; rawspeed and rawler decode it with a rule instead | Reduces to a short rule, described in [raw-data.md](raw-data.md); checked on files 2346 and 2271 **[F]**. No table needed |
| Inverse of the MakerNote cipher (blocks 0x2010, 0x9050, 0x94xx) | No | Precomputed in LibRaw, ExifTool and metadata-extractor; computed from the formula in exiv2 and rawler | Computable from the formula b³ mod 249 (see [makernote-and-metadata.md](makernote-and-metadata.md)); checked on the shutter counts of 14 files: 6929, 6930, 6931, 6933, 6936, 1541, 1542, 1310, 1311 and 8846 to 8850 **[F]**. No table needed |
| SR2 and SRF keystream | The key is; the algorithm is not | dcraw, LibRaw, rawspeed, ExifTool | A formula with two constants, described in full in [makernote-and-metadata.md](makernote-and-metadata.md); checked on 34 files **[F]** |
| cRAW tone curve | Yes, four knots in tag 0x7010 | Built by rule in every decoder | No table |
| Lossless JPEG Huffman tables | Yes, a DHT segment in every tile | ITU-T T.81 | No table |
| Per-body crops, black and white levels, CFA pattern | Mostly (DefaultCrop, 0x7038, 0x7310, 0x787F, 0xC61D, CFAPattern); older bodies need fallbacks | rawspeed's `cameras.xml`, dcraw and LibRaw | Derive from samples (option (b) below) |
| Colour matrices | Sony's 0x7800 exists, meaning unconfirmed (its rows sum to about 1024 on file 1542 **[F]**) | Adobe-derived per-body matrices in dcraw, LibRaw, rawspeed and rawkit | Needed only for rendering, which the roadmap leaves open; defer [I] |
| YCbCr to RGB for the M and S modes | No | LibRaw uses ITU-R BT.601 | BT.601 is a public standard [I] |
| ARW 6.0 delinearisation curve, 4096 entries | **No** | LibRaw (an embedded table, LGPL or CDDL); arw6_decode (a table extracted from an official decoder's output, unlicensed) | The hard case; see section 4.3 |

The three options on the NEF page remain: (a) treat a short table as a
format fact and transcribe it from at least two sources, (b) derive it from
sample files, (c) strict clean-room with separate documenter and coder. For
ARW we suggest [I]:

- no table at all for the ARW1 code, the cipher inverse and the keystream,
  since each is a rule or a formula that the code computes;
- option (b) for per-body values, checked against the files in
  [test-data.md](test-data.md#files-checked);
- no colour matrices until the roadmap decides that raw-files renders
  colour;
- for the ARW 6.0 curve, no decision until section 4.3's questions are
  answered, and no table committed in the meantime.

The milestones that need any of this (raw sensor data, milestone 6) come
after ARW previews (milestone 2) and basic metadata (milestone 3). The
metadata that milestone 3 needs is stored in plain form in every file
checked, so milestones 2 and 3 need neither the keystream nor the cipher.
**[F]**

### 4.3 Out of scope for now: ARW 6.0

raw-files detects Compression 32766 and returns a typed unsupported error
for the raw data, as it does for Nikon HE and HE\* (see the
[NEF page](../nef/licences-and-references.md)). Previews and metadata from
these files are fine: every ARW 6.0 file checked has a full-size JPEG in
IFD2 (raw.pixls.us files 8846 to 8850). **[F]** Two questions have to be
answered before a decoder is written:

1. **Where the curve comes from.** The codec ends by mapping 12-bit codes
   through a fixed 4096-entry curve that is not stored in the file. It is
   published only as tables extracted from existing decoders, and its
   construction is unknown. [15][31] **[P]** Read without copying, LibRaw's
   table is the identity up to code 1427, then steepens and ends at 39002 [15]
   **[P]** (shape **[I]**); arw6_decode's README calls it a logarithmic curve
   of the Cineon or S-Log family with a linear toe [31] **[S]**. The files' own
   levels fit LibRaw's hard-coded ones at half scale: tag 0x7310 is 512 where
   LibRaw uses a black level of 1024, and 0xC61D is 16383 where LibRaw's linear
   maximum is 32800 (files 8846 to 8850). **[F]** (tags), **[I]** (relation) A
   curve that turned out to be a published formula would remove the table
   problem. [I]
2. **The patent position** (section 5).

## 5. Patents and encryption

**Not legal advice.** These notes identify questions; they answer none.

- **cRAW.** Sony's US 2007/0223824 A1, granted as **US 8,023,748 B2**
  (2011-09-20; inventors Fumiaki Kato and Ken Nakajima; priority JP
  2006-050170 of 2006-02-27), describes in its first embodiment the cRAW
  block: 16 same-colour pixels, the maximum and minimum with their
  positions, and the other 14 as 7-bit quantised differences, in 128 bits.
  Google Patents lists it as "Expired – Fee Related". [32] **[S]** A related
  Sony patent, US 8,013,910 B2 (priority 2007-05-24, same inventors), has
  the same status. [32] **[S]** The status of the European counterpart,
  EP 1841233 A1, was not checked. A patent on an encoder may not read on a
  decoder, and a lapsed patent cannot be enforced, so a cRAW decoder looks
  unencumbered. [I]
- **Lossless.** ITU-T T.81 dates from 1992; patents on its lossless
  process have long expired. [I]
- **ARW 6.0.** No patent was identified. LibRaw names the stream "llvc3"
  and arw6_decode calls the format "LLVC v3". [15][31] **[P]** LLVC is
  Sony's name for a line-based wavelet video codec disclosed as SMPTE RDD
  34:2015 (5/3 lifting, 16-line units, 128-bit words, variable-length
  coding), and RDD 34 directs intellectual-property enquiries to Sony. [13]
  **[P]** That ARW 6.0 descends from LLVC is **[I]**: the names and the
  structure match, but no Sony document confirms it. If the codec also uses
  JPEG XS techniques, the JPEG XS patent pool may be relevant, as it is for
  Nikon HE. [36] **[I]**
- **Encryption.** The SR2SubIFD and SRF data are XOR-ed with a keystream
  whose key is stored in the file, and some MakerNote blocks use a keyless
  byte substitution. [21] **[P]** On every ARW checked the key in tag 0x7221
  is the same four bytes, `11 22 33 44`. **[F]** Whether decrypting counts
  as circumventing a technological measure under DMCA-like laws is
  untested; the open decoders have done it for about twenty years and we
  found no action against any of them. [38] **[S]**

## 6. Community references

Read these for facts and cite them; do not copy their tables or prose.

| Reference | What it is good for | Note |
|---|---|---|
| ExifTool Sony tag pages [21] | Tag IDs and values: FileFormat up to ARW 6.0, SonyModelID, RAWFileType, Quality, SR2Private, SRF tags, deciphered blocks | Licensed as ExifTool; tag text is expression; the best single reference |
| Laurent Clévy, "Description of Sony RAW file formats (SRF, SR2, ARW and ARQ)", `lclevy/sony_raw`, v0.55 (2026-05-24) [33] | Format history, a body-to-version table up to the A7 V, TIFF trees for SRF, SR2, ARW 1.0 and ARW 4.0, ARQ notes, ExifTool dumps of four bodies | GPL-3.0 documentation: read for facts, copy no prose, tables or diagrams; "work in progress", with known errors (section 6.1) |
| RawDigger, "Detecting posterization in Sony cRAW/ARW2 files" (2014) [34] | The clearest prose description of the cRAW block | Commercial vendor (the LibRaw authors) |
| H. G. Dietz, "Sony ARW2 compression: artifacts and credible repair", IS&T Electronic Imaging 2016 [35] | An academic analysis of the cRAW mapping and its artefacts | The paper's repair tool, KARWY, states no licence yet |
| `sony_arw` README in the FLIF raw-tools hosted on MIT's stuff.mit.edu server [35] | Notes from re-encoding cRAW losslessly | Licence GPL-3.0-or-later (MIT here is the hosting university, not the licence); it speaks of 8-bit differences where the patent, dcraw and RawDigger say 7 bits, and 7-bit deltas fit the blocks measured on all 15 cRAW files **[F]** |
| SMPTE RDD 34:2015 [13] | Sony's LLVC wavelet codec | See section 5 |
| Lloyd Chambers (diglloyd) and Jim Kasson blogs [35] | Visible cRAW artefacts, 12-bit and 14-bit modes on various bodies | Opinionated blogs; facts secondary |
| exiv2 Sony tag page [22] | Sony tag names as exiv2 knows them | Licensed as exiv2 |
| fileformat.com ARW page [35] | Nothing: it calls ARW uncompressed and quotes 1 to 2 GB files | **Unreliable**; do not cite for facts |

### 6.1 Known errors in Clévy's notes

Clévy's notes are the usual community reference, and most of what they say
was confirmed on samples. These points were not [33]:

- They call ARW 1.0 "12 bits uncompressed". The A100 file holds an
  ARW1-coded (compressed) stream of 9.57 MiB for a 10 MP sensor, which the
  ARW1 rule decodes completely (raw.pixls.us file 2346). **[F]**
- Their "ARW 4.0" example is a dump of an ILCE-9 file whose FileFormat is
  `3 3 3 0`, which ExifTool names ARW 2.3.3, and whose Software is "ILCE-9
  v0.01". [33] **[P]** Real ARW 4.0 files differ from the tree shown: they
  add a third IFD holding a full-size JPEG (files 5418 and 4466). **[F]**
- They date cRAW to ARW 2.1 (2008), but the DSLR-A700, whose FileFormat is
  `3 0 0 0` (ARW 2.0), already writes cRAW (file 2521). **[F]**
- They describe SRF data as 12-bit, but the decrypted DSC-F828 data reaches
  16368, so it is 14-bit (file 1351). **[F]**
- Their body-to-version table does not match the files for two bodies: an
  SLT-A99V on firmware 1.02 writes `3 3 1 0` (ARW 2.3.1) where the table
  says ARW 2.3, and an ILCE-7RM3 on firmware 3.00 writes `3 3 5 0` (ARW
  2.3.5) where it says ARW 2.3.3 (files 3751 and 3251). **[F]** The version
  seems to follow firmware rather than body. [I]

## 7. What is still unknown

- Whether Sony holds any format documentation it would share, and on what
  terms.
- Whether rawkit came from dcraw or from LibRaw's inherited copy, and
  whether Graphite's relicensing left out any part of it.
- Whether metadata-extractor's decipher table and imagemeta's tag tables
  derive from ExifTool's; neither file says.
- How the ARW 6.0 curve is constructed. A formula would let raw-files
  compute it; until then the curve exists only as extracted tables.
- Whether ARW 6.0 is LLVC as disclosed in RDD 34, and whether Sony or the
  JPEG XS pool holds patents that would read on a decoder.
- The status of EP 1841233 A1, the European counterpart of the cRAW patent.
- Whether PRONOM's DROID tool identifies ARW 4.0 and later files at all; a
  run on raw.pixls.us files 4466, 7361 and 8846 would show it.
- The provenance policy for per-body values and colour matrices (section
  4.2), to be recorded in an ADR before milestone 6.

## Sources

[1] Sony, Camera Remote SDK page —
    https://support.d-imaging.sony.co.jp/app/sdk/en/index.html **[P]**
[2] Sony, SDK and Camera Remote Command licence agreements —
    https://support.d-imaging.sony.co.jp/app/sdk/licenseagreement_d/en.html ,
    https://support.d-imaging.sony.co.jp/app/cameraremotecommand/licenseagreement/en.html
    **[P]**
[3] Sony RAW Driver, Imaging Edge and Pixel Shift Multi Shooting pages —
    https://support.d-imaging.sony.co.jp/www/download/vista_raw/ ,
    https://support.d-imaging.sony.co.jp/app/imagingedge/en/instruction/1_2_about_psm.php ,
    https://support.d-imaging.sony.co.jp/support/ilc/psms/ilce7rm4/en/index.html
    **[S]** (search extracts)
[4] PRONOM records fmt/191, fmt/1127, fmt/1126 —
    https://pronom.nationalarchives.gov.uk/fmt/191.xml ,
    https://pronom.nationalarchives.gov.uk/fmt/1127.xml ,
    https://pronom.nationalarchives.gov.uk/fmt/1126.xml **[P]**
[5] Library of Congress, Camera Raw Formats (group description) —
    https://www.loc.gov/preservation/digital/formats/fdd/fdd000241.shtml
    **[P]**
[6] freedesktop shared-mime-info —
    https://gitlab.freedesktop.org/xdg/shared-mime-info/-/raw/master/data/freedesktop.org.xml.in ;
    IANA media types —
    https://www.iana.org/assignments/media-types/media-types.xhtml **[P]**
[7] Bio-Formats, supported formats —
    https://bio-formats.readthedocs.io/en/latest/supported-formats.html
    **[P]**
[8] Adobe, TIFF Revision 6.0 (1992), ITU mirror —
    https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf **[S]**
[9] ISO, ISO 12234-2:2001 listing — https://www.iso.org/standard/29377.html
    **[S]**
[10] CIPA, standards page (DC-008-Translation-2026, Exif 3.1) —
    https://www.cipa.jp/e/std/std-sec.html **[P]**
[11] ITU-T Recommendation T.81 — https://www.itu.int/rec/T-REC-T.81 **[I]**
[12] Adobe, DNG SDK download page —
    https://www.adobe.com/support/downloads/dng/dng_sdk.html **[P]** (as
    retrieved for the NEF page; not re-read for ARW)
[13] SMPTE RDD 34:2015, "LLVC – Low Latency Video Codec for Network
    Transfer" — https://pub.smpte.org/latest/rdd34/rdd34-2015.pdf **[P]**
    (pages 1 to 8 read)
[14] dcraw 9.28 (revision 1.478, 2018-06-01) —
    https://www.dechifro.org/dcraw/dcraw.c **[P]**; the text was read from
    a mirror, https://raw.githubusercontent.com/ncruces/dcraw/master/dcraw.c,
    not verified against the original
[15] LibRaw — README and Changelog
    https://github.com/LibRaw/LibRaw ; `src/decoders/sony_arw6.cpp` on
    master (header, identifiers and the curve's shape only); PR #824
    https://github.com/LibRaw/LibRaw/pull/824 ; releases
    https://api.github.com/repos/LibRaw/LibRaw/releases **[P]**
[16] rawspeed — `src/librawspeed/decoders/ArwDecoder.cpp` (develop,
    c10cc5004c, 2026-10-10); PR #972
    https://github.com/darktable-org/rawspeed/pull/972 **[P]**
[17] dnglab / rawler — `rawler/src/decoders/arw.rs` (main, ae01bcb2d0,
    2026-08-30); PR #825 https://github.com/dnglab/dnglab/pull/825 **[P]**
[18] rawloader — `src/decoders/arw.rs` (master, 8f2ace2d27) —
    https://github.com/pedrocr/rawloader **[P]**
[19] crates.io API, per-crate pages (imagepipe, quickraw, rexiv2,
    exiftool-rs, rawbit, gamut-exif and the crates named on the NEF page) —
    https://crates.io/api/v1/crates/<name> **[P]** (licences)
[20] libopenraw — https://gitlab.freedesktop.org/libopenraw/libopenraw ,
    `src/sony.rs` (main, 2026-10-05) **[P]**
[21] ExifTool — README
    https://raw.githubusercontent.com/exiftool/exiftool/master/README ;
    home page https://exiftool.org/ ; Sony tags
    https://exiftool.org/TagNames/Sony.html ; licence metadata
    https://api.github.com/repos/exiftool/exiftool/license **[P]**
[22] Exiv2 — LICENSE.txt
    https://raw.githubusercontent.com/Exiv2/exiv2/main/LICENSE.txt ;
    repository https://github.com/Exiv2/exiv2 **[P]**
[23] GitHub licence API for libexif, darktable and RawTherapee —
    https://api.github.com/repos/<owner>/<repo>/license **[P]**
[24] metadata-extractor — https://github.com/drewnoakes/metadata-extractor
    (main, 6f037943c0) **[P]**
[25] Ashampoo Kim — https://github.com/Ashampoo/kim **[P]**
[26] jpgfromraw — https://github.com/cdown/jpgfromraw **[P]**
[27] imagemeta — https://github.com/evanoberholster/imagemeta **[P]**
[28] exif-py — https://github.com/ianare/exif-py **[P]**
[29] rawpy — https://github.com/letmaik/rawpy **[P]**
[30] Graphite, rawkit —
    https://github.com/GraphiteEditor/Graphite/tree/master/libraries/rawkit
    (master, 7477cd2fab, 2026-10-10), its commit history and PRs #1757,
    #2088, #2659 and #4208 **[P]**
[31] abbradar, arw6_decode, README only —
    https://github.com/abbradar/arw6_decode **[P]** (no licence)
[32] Google Patents — US 8,023,748 B2
    https://patents.google.com/patent/US20070223824A1/en ; US 8,013,910 B2
    https://patents.google.com/patent/US8013910B2/en **[S]** (read through a
    summary)
[33] Laurent Clévy, "Description of Sony RAW file formats (SRF, SR2, ARW
    and ARQ)", v0.55 — https://github.com/lclevy/sony_raw **[S]** for the
    prose; **[P]** where the TIFF and ExifTool dumps it publishes are cited
[34] RawDigger, "Detecting posterization in Sony cRAW/ARW2 files" —
    https://www.rawdigger.com/howtouse/sony-craw-arw2-posterization-detection
    **[S]**
[35] Other community references — H. G. Dietz, IS&T EI 2016,
    https://doi.org/10.2352/ISSN.2470-1173.2016.2.VIPC-227 , KARWY
    https://aggregate.org/DIT/KARWY/ ; FLIF raw-tools on stuff.mit.edu
    https://stuff.mit.edu/afs/sipb/user/kenta/software/FLIF/raw-tools/sony_arw/README
    **[P]**; diglloyd
    https://diglloyd.com/blog/2019/20191026_2104-SonyA7R_IV-lossy-compression-artifacts.html ;
    Jim Kasson https://blog.kasson.com/?p=4876 ; fileformat.com
    https://docs.fileformat.com/image/arw/ **[S]** (except where marked)
[36] JPEG XS patent pool (Fraunhofer IIS, intoPIX, Vectis) —
    https://www.intopix.com/blogs/post/intoPIX-and-Fraunhofer-IIS-team-with-Vectis-to-offer-JPEG-XS-patent-pool-license
    **[S]**
[37] OpenRAW — https://en.wikipedia.org/wiki/OpenRAW **[S]**
[38] Web searches for Sony legal action, DMCA notices or cease-and-desist
    letters against dcraw, LibRaw or other ARW decoders, October 2026: no
    hits **[S]**
[39] A7 V support reports —
    https://lartdelaphoto.fr/sony-a7-v-raw-compresse-impact-workflow-lightroom/ ,
    https://forum.dxo.com/t/sony-a7v-compressed-and-hq-raw-not-supported/54584
    **[S]**
[40] raw.pixls.us sample files cited by id on this page; the URL and
    SHA-256 of each are listed in
    [test-data.md](test-data.md#files-checked) **[F]**
