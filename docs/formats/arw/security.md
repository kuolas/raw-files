# ARW: security notes

A Sony raw file is untrusted input. This page covers the Sony layer only:
the known vulnerabilities in ARW, SR2, SRF and Sony MakerNote handling, the
Sony hardening that other projects shipped without a CVE, the fuzzing
reports against a Rust decoder, and a checklist that maps each Sony
structure to the raw-files rules. Back to [the ARW overview](README.md).

The generic layer is on the NEF page and is not repeated here: TIFF, Exif
and LibRaw bug classes, grouped by class, are in
[NEF security, section 2](../nef/security.md), and its checklist items 1 to
14 apply to ARW unchanged. Section 4 below lists only what has appeared in
the generic layer since that page was written.

Confidence tags: **[P]** primary (read in the NVD record, the project's
changelog, commit or source), **[S]** secondary (distribution trackers,
summaries), **[I]** inferred, **[F]** measured on a named sample file.
Samples are cited as "raw.pixls.us file &lt;id&gt;"; the files cited here
are listed with their URLs and SHA-256 in [Sources](#sources), and every
file checked is in [test-data.md](test-data.md#files-checked). Numbers in
brackets point to the Sources, retrieved in October 2026. The sample checks
were structural (offsets, lengths, headers, block statistics) and used
throwaway scripts that are not in the repository; nothing here was fuzzed.

The raw-files baseline is
[ADR008](../../architecture-decisions/adr008-lints-and-panics.md): no
`unsafe` code, and malformed input returns an error, never a panic. Section 3
shows why the second half matters as much as the first: a Rust ARW decoder
without that rule panics on fuzzed files.

## 1. Sony and ARW vulnerabilities with CVEs

### 1.1 Open-source parsers

Only three CVEs name a Sony code path in an open-source parser, and none of
them is in an ARW pixel decoder. [1][2][3] **[P]**

| ID | Component | Class | Notes | Conf. |
|---|---|---|---|---|
| CVE-2017-6887 | LibRaw `parse_tiff_ifd()`, SubIFDs tag (0x014A) special case for model `DSLR-A100` | Memory corruption (CWE-119): the special case selects the A100 loader and advances the IFD index with no bound, so repeated 0x0100/0x014A sequences write past the fixed IFD array | Published 2017-05-16. NVD's description names a crafted KDC file with the model set to `DSLR-A100`; the code path is the A100 ARW one. Fix commit d7c3d2c (2017-03-04, "Secunia SA75000"). Fixed version disputed, see 1.4. The NEF page lists it under generic `parse_tiff_ifd` bugs. [1][8][10] | P |
| CVE-2020-35535 | LibRaw `parseSonySRF()` (`src/metadata/sony.cpp`) | Out-of-bounds read (CWE-125) | Published 2022-09-01; reported 2020-05-10 as issue #283 with a proof of concept through ImageMagick. Fix c243f45 (2020-05-10), which also bounds-checks `parseSonySR2`. SRF data is reached from the DSC-F828/V3 MakerNote. Affected versions disputed, see 1.4. [2][8][9] | P |
| CVE-2021-34335 (GHSA-pvjp-m4f6-q984) | Exiv2 `resolveLens0xffff()` in `minoltamn_int.cpp`, called when printing a Sony lens type 0xffff | Divide by zero (CWE-369) by `Exif.Photo.FocalLength` | Published 2021-08-09; affected up to 0.27.4, fixed in 0.27.5. Commit 11a26fb0, which requires `FocalLength > 0`, is from PR #1753, the port of the fix to the main branch, and is not in 0.27.5. Reachable only when the model string is `ILCE-6000` and FocalLength is 0, and only when printing. A later commit, dc2c77ce (PR #1786), made a token lookup in the same file checked. [3][13] | P |

### 1.2 Closed-source viewers

Five CVEs from the Zero Day Initiative name ARW or SR2 parsing in
closed-source viewers. No root-cause details are public, so which Sony
structure each one mis-parsed is unknown. [4][5] **[P]**

| ID | Product | Class | Format | Published |
|---|---|---|---|---|
| CVE-2020-17423 (ZDI-20-1334) | Foxit Studio Photo 3.6.6.922 | Heap overflow: a length is not validated before a copy (CWE-122, 787) | ARW | 2021-02-09 |
| CVE-2020-17434 (ZDI-20-1345) | Foxit Studio Photo 3.6.6.922 | Out-of-bounds read (CWE-125) | ARW | 2021-02-09 |
| CVE-2020-27855 (ZDI-20-1348) | Foxit Studio Photo 3.6.6.922 | Out-of-bounds read (CWE-125) | SR2 | 2021-02-09 |
| CVE-2021-31433 (ZDI-21-476) | Foxit Studio Photo up to 3.6.6.933 | Out-of-bounds write (CWE-787) | ARW | 2021-04-29 |
| CVE-2024-11520 (ZDI-24-1580) | IrfanView 4.67 | Out-of-bounds write (CWE-787) | ARW | 2024-11-22 |

Viewers like these usually display the embedded preview, so the preview
path is a plausible target, but nothing published says so. **[I]** The SR2
entry may be the SR2Private path (checklist item A5); that too is **[I]**.

### 1.3 What the searches did not find

- **Exiv2 has one Sony-reachable CVE, not several.** All 37 Exiv2 GitHub
  advisories and all 125 NVD Exiv2 entries were read; the only CVE that
  reaches Sony code is CVE-2021-34335. The Sony print functions had other
  bugs, fixed without a CVE (section 2.3). [13][14] **[P]**
- NVD keyword searches for `sony`, `arw`, `sr2`, `srf`, `arq`, `sony arw`,
  `sony makernote`, `exiv2 sony`, `libraw sony`, `imaging edge` and similar
  returned nothing else relevant. The other `sony` hits are devices (TVs,
  camera web interfaces, a PSP TIFF viewer) and libgphoto2's PTP code
  (CVE-2026-40336, CVE-2026-40338, CVE-2026-40339), which parses USB
  traffic, not files. [6] **[P]**
- Apple's NVD "RAW image" entries name Canon, DNG, NEF and PEF, never Sony.
  The Windows Camera Codec Pack and Raw Image Extension CVEs name no format.
  Android shows only ExifInterface CVE-2020-0020, which is not Sony-specific;
  Google's preview extractor piex has no NVD entries. No ARW-specific Adobe
  advisory was found. ExifTool's CVEs are all command or code injection, none
  in its Sony module. [6][20] **[P]**; the Apple and Android searches were
  keyword-only.
- The NVD keyword search matches whole words, so a CVE that says "Sony RAW"
  without "ARW", or names the format only in a vendor advisory, can be
  missed. ZDI advisory pages and the OSS-Fuzz tracker were not opened.

### 1.4 Disagreements

**CVE-2017-6887, fixed version.** NVD lists LibRaw up to 0.18.1 as affected,
which implies a fix in 0.18.2, and LibRaw's changelog dates 0.18.2 to
2017-03-08 with "Fixed several errors (Secunia advisory SA75000)". But the
git tag 0.18.2 (commit 4d2aec2, 2017-03-04) does not contain the A100 bound
check: the fix commit d7c3d2c is not an ancestor of that tag, and the check
first appears in tag 0.18.3 (2017-09-09). The release tarball may differ
from the tag; we did not download it. Conservative reference: 0.18.3.
[1][8][10] **[P]**

**CVE-2020-35535, affected versions.** NVD lists 0.20.0, 0.20.0-rc2,
0.20.1, 0.20.2 and 0.21.0-beta1 as affected and gives no fixed version. The
fix commit c243f45, which NVD itself references, is contained in tag
0.20-RC2 and every later tag; the 0.20.0 source has the added SR2 and SRF
checks; and LibRaw's 0.20 notes say "fixed possible overrun in Sony SRF and
SR2 metadata parsers". Both cannot be right. Conservative reading: fixed in
0.20.0 per the source; the NVD record disagrees. [2][8][10] **[P]**

**CVE-2026-88387, fix.** This LibRaw NewSubfileType (0x00FE) bug is listed
with the generic additions in section 4, but it matters for ARW because a
parser picks the raw and preview IFDs by NewSubfileType. NVD (2026-09-24,
status Deferred) names LibRaw 0.22.0 and references commit b41cbbd, issue
#844 and PR #853. PR #853 was closed unmerged. Commit b41cbbd only skipped
FLOAT and DOUBLE types, and was reverted on 2026-09-09 (dde798d, "No need
for it"). LibRaw master at 7bfffe2 (2026-10-09) still converts the value
with `int(getreal(type))`, and for type LONG, which the proof of concept
uses, `getreal` returns the unsigned 32-bit value. [7][8][9] **[P]** We infer
that the LONG case is unfixed in 0.22.1, 0.22.2 and master. **[I]**

## 2. Sony hardening without a CVE

Most Sony hardening in LibRaw and rawspeed came from fuzzing (OSS-Fuzz,
AFL) and was fixed without a CVE. These fixes show where the Sony bugs
actually are.

### 2.1 LibRaw

LibRaw history read at master 7bfffe2 (2026-10-09); the first release is
the first tag that contains the commit. [8][9][10][11] **[P]**

| Date | Commit | Function | Class | First release |
|---|---|---|---|---|
| 2012-03-15 | cc54bb6 | `sony_arw2_load_raw` | Over-read: the cRAW row buffer was sized to the raw width, but 16-byte block reads run up to 4 bytes past it | 0.14.6 |
| 2017-03 | in the 0.18.2 diff | `process_Sony_0x9050` | Undefined shifts by 32 and 40 bits when assembling an internal serial number on 32-bit builds | 0.18.2 |
| 2017-12-15 | ba673ca | `process_Sony_0x0116`, `0x9400`, `0x9402`, `0x9403`, `0x9406`, `0x940c` | Over-read: fixed-index reads into enciphered MakerNote blocks without a length | 0.19.0 |
| 2018-01-29 | 8d44074 | SR2 block inside DNG | Over-read: a value offset not checked against SR2SubIFDLength | 0.19.0 |
| 2018-10-02 | 02e24bf | `parseSonySRF` | Over-read: a key field at a fixed offset read without a length check | 0.20-RC2 |
| 2019-08-05 to 2019-11-12 | 1375b07, af705b8, 5d0707d, 3e2cc80, 74c2689, b1c2fcd, f9ff88f, 2099f64 | `parseSonySRF`, `sony_decrypt` | Over-reads; negative offsets from absolute positions minus the current position; a decrypt length smaller than the offset; entry counts of 0 or above 1000; a `sony_decrypt` over-read on 32-bit builds (OSS-Fuzz 13897; issues #231, #244, #246, #251) | 0.20-RC2 |
| 2020-05-14 | 72e05585 | `sony_decrypt` | Signed integer overflow (undefined behaviour) in the keystream seeding, moved to unsigned 64-bit (OSS-Fuzz 7432 and 9465; the maintainer called it "Not a bug" and "GIGO" but changed it) | 0.20-RC2 |
| 2020-06-29 | 35d7083 | SRF and SR2 white-balance lists | Static index tables replaced by bounds-checked ones | 0.20-RC2 |
| 2023-07-31 | 17294b5 | `sony_arq_load_raw` | ARQ decoded without checking the metadata: the fix rejects a file that has a CFA pattern or fewer than three colours | 0.21.2 and 0.22.0 |
| 2023-12-14 | 62f0423, a6f212a | `libraw_tagtype_dataunit_bytes`, via `parseSonySR2` | Off-by-one indexing a 19-entry TIFF type-size table with a type from the file (OSS-Fuzz 65027, OSV-2023-1295) | 0.21.2 |
| 2024-05-04 | c2e8a90 | `process_Sony_0x2010` | Over-read: fixed offsets (aspect ratio, metering and others) read past the block length (OSS-Fuzz 68535, issue #646) | 0.22.0 |
| 2024-05-25 | 54c6af9 | `parseSonySR2` | Overrun of a 64-row white-balance table from tags 0x7480 to 0x7486 and 0x7820 to 0x782D | 0.22.0 |
| 2024-07-06 | 9c9c04b | SR2Private reading in `parse_tiff_ifd` | A short read was decrypted and parsed anyway | 0.22.0 |
| 2024-07-07 | 17f1683 | `sony_ljpeg_load_raw` | Out-of-bounds write: lossless tile output written past the raw height and width | 0.21.3 and 0.22.0 |
| 2026-03-10 | 4e265f1 | `sony_ycbcr_load_raw` (new in 0.22.0) | Tile size unchecked; the bit reader's fast path read 1 to 3 bytes past the buffer (issue #792, CWE-125) | 0.22.1 |

A missing NULL check after an allocation in the Sony AFInfo copy was
reported in April 2026 (#810) and rejected, because LibRaw's allocator
throws on failure. LibRaw still loops over cRAW blocks with a signed
`raw_width - 30` and does not check that a block's maximum and minimum
indices differ. [8][9] **[P]**

**New attack surface.** An ARW 6.0 decoder of about 1,600 lines was merged
into LibRaw master on 2026-07-18 (PR #824) and is in no release yet. [9]
**[P]** raw-files refuses ARW 6.0 (checklist item A14).

### 2.2 rawspeed

rawspeed read at develop c10cc500 (2026-10-10). We did not map these
commits to releases. [12] **[P]**

| Date | Commit | Function | Class |
|---|---|---|---|
| 2016-03-06 | a3a8b581 | ARW2 (cRAW) decompressor | Out-of-bounds read on images narrower than 32 pixels: the loop bound `w - 30` wrapped in unsigned arithmetic |
| 2017-08/09 | 376e0d1b, 46eae2f7, 35fe49b5 | ARW and SRF decoders | Image dimensions capped; an empty strip rejected before slicing |
| 2017-11-14 | 5ff5484d | ARW decoder | Bits per sample restricted to 8, 12 and 14 |
| 2017-11-15/16 | 582bf5d0, 7e51ed90, a74c1a57 | ARW1 and ARW2 decompressors | Fuzzers added; ARW2 now takes exactly W·H input bytes |
| 2018-01-03 | fa488def | ARW1 decompressor | The running sum is checked for negative values as well as values above 4095 |
| 2018-01-11 | dbe7591e | `TableLookUp::setTable` | Tables of 65,536 entries or more rejected. This is CVE-2018-25017 (OSS-Fuzz 5256, OSV-2018-227, found through a DNG); the same lookup carries the ARW 0x7010 tone curve [11] |
| 2019 and 2021-09-18 | b9f5b8b1 and earlier | ARW2 decompressor | Width must be a positive multiple of 32 (and at most 9600, height at most 6376); a block whose maximum and minimum indices are equal is an error |
| 2023-02-16 | bbd6d363 | SR2 white-balance decryption | A length that rounds down to 0 is refused; the decrypted IFD may not point into the padding used to make absolute offsets resolve |
| 2023-12-23 | 0953acf8, 8d37641f | Sony lossless-JPEG tiles | Tile width doubled in 64-bit arithmetic to avoid overflow; tile byte ranges must not overlap |

rawspeed's current lossless path also requires an even image width and
height, non-zero tile width and height, an even tile height, as many tile
offsets as byte counts, and a tile count equal to the tile grid, checked by
division so the product cannot overflow. Its A100 white-balance reader walks
the Minolta-style blocks with a bounded sub-stream and rejects zero-length
entries, which would otherwise loop without advancing. [12] **[P]**

### 2.3 Exiv2 and dcraw

- **Exiv2.** OSS-Fuzz 52355 (issue #2385, 2022-10-22): Sony's
  `printColorCompensationFilter` negated `INT_MIN`; fixed in 3897f17b
  (v0.28.0), and the maintainer judged it not a security vulnerability.
  Commit 1119a68a (2023-06-25) added NULL checks to 17 Sony print functions
  that dereferenced their metadata argument; present in v0.28.0, fixed in
  v0.28.1, no CVE. Issue #2380, "Crash when loading Sony ARW", was a generic
  I/O bug. [13] **[P]**
- **dcraw 9.28** (read from a mirror; the original site did not respond):
  the SR2Private length is not capped and the read is not checked before
  decrypting; the decrypted block is written to a temporary file and parsed
  again with a negative base; nested parsing is bounded only by a global IFD
  limit; the 0x7010 curve points are masked to 12 bits; the cRAW row buffer
  is one byte longer than the row. [15] **[P]** A block with equal maximum
  and minimum indices makes dcraw read a fifteenth delta past the block, one
  byte beyond its buffer on a row's last block. **[I]**

## 3. Rust is not immune: rawler's fuzzing reports

RustSec has no advisories for rawler, rawloader, the `tiff` crate or any
raw crate (advisory-db at 7eebec6, 2026-10-09). [18] **[P]** But rawler
(the decoder in dnglab, LGPL-2.1) has open, unadvised fuzzing reports from
July 2025 on TIFF-family input, three of them ARW (#572, #574, #575); the
others came from RW2, DNG, generic TIFF and, for #797, a CR2 seed.
[16] **[P]**

| Issue | Input | Report | State |
|---|---|---|---|
| #574 | ARW | Panic indexing byte 200,896 (dcraw's absolute SRF key offset) of a 44,210-byte file in the SRF path | Open; the index is still unchecked on main at ae01bcb2 (2026-08-30), where the same code also multiplies width × height × 2 unchecked |
| #575 | ARW | `unwrap` on a sub-view past the end of the file | Closed |
| #571 | RW2 | Slice-range panic in the bit reader | Open |
| #569 | TIFF | Index panic in the TIFF IFD code | Open |
| #570 | TIFF | Out of memory | Open |
| #572 | ARW | Hang | Open |
| #573 | DNG | Assertion failure | Open |
| #582 | DNG | Subtraction overflow | Open |
| #797 | CR2 seed | An IFD `count` drove a 6.6 GB allocation from an input of at most 2 MB (2026-07) | Closed |

rawler's ARW 6.0 decompressor is an `unimplemented!()` stub, but it is
unreachable today: rawler refuses Compression 32766 as unsupported before
decoding (no camera file defines its "arw6" mode), and `RawLoader::decode`
catches panics in any case. An earlier report (#514) showed ILCE-7CR M/S-size
lossless files panicking it. rawloader issue #54 (2026-04) reports three more
fuzzing panics from unchecked indexing, in RAF, X3F and TIFF code. [16][17]
**[P]**

Every one of these is a panic, an abort or runaway work, not memory
corruption: exactly the class Rust's safety does not cover and ADR008's
"error, never a panic" rule does. The same reports are a ready-made list of
inputs for raw-files' own fuzz targets (item A17).

## 4. Additions to the generic layer, 2025–2026

New since [the NEF page](../nef/security.md); not Sony-specific. [7][14][19]
**[P]**

| ID | Project and component | Class | Note |
|---|---|---|---|
| CVE-2025-43961 | LibRaw `tiff.cpp`, Fujifilm 0xf00c | Out-of-bounds read | Before 0.21.4 |
| CVE-2025-43962, -43963, -43964 | LibRaw `phase_one_correct`, tags 0x412 and 0x41f | Out-of-bounds read; missing minimums | Before 0.21.4; LibRaw's 0.21.5 changelog says the CVE numbers arrived after the 0.21.4 release [10] |
| CVE-2026-20911 | LibRaw `HuffTable::initval` (TALOS-2026-2330) | Heap overflow (CWE-131) | New as an ID, not in kind: it is the bug of CVE-2026-5318 on the NEF page, and both are fixed by the same commit, a6734e8 ("Fix for TALOS-2026-2330"). NVD contradicts itself: this record lists 0.22.1 as affected, while CVE-2026-5318's gives 0.22.1 as the fix. LibRaw's 0.22.1 changelog lists the Talos ID as fixed. |
| CVE-2026-24660 | LibRaw `x3f_load_huffman` (TALOS-2026-2359) | Heap overflow | Fixed per changelog in 0.22.1 |
| CVE-2026-88387 | LibRaw `parse_tiff_ifd`, NewSubfileType | Out-of-range float-to-integer conversion (CWE-681) | Matters for ARW IFD roles; see 1.4 |
| CVE-2026-32775, -40385, -40386 | libexif MakerNote handling, up to 0.6.25 | Size underflow (CWE-191); 32-bit overflow in the Nikon MakerNote; underflow in a size check (Fujifilm, Olympus) | |
| CVE-2025-9900 | libtiff | Write-what-where through a large image height | Fixed in 4.7.1 |
| CVE-2026-4775, -12912, -18495 | libtiff YCbCr tiles, PixarLog, tiff2pdf | Integer and heap overflows | |
| CVE-2025-64783 to CVE-2026-48389 (18 IDs) | Adobe DNG SDK up to 1.7.1 | Integer overflow, out-of-bounds read and write, heap and stack overflow, NULL dereference | December 2025 to August 2026 |
| CVE-2026-69649 | Windows Raw Image Extension before 2.5.24.0 | Heap overflow | Published 2026-09-08; no format named |
| CVE-2025-54080, CVE-2025-55304, CVE-2026-25884, -27596, -27631 | Exiv2 (EPS write, ICC, CRW, preview loader) | Out-of-bounds read, denial of service, uncaught exception | Two preview-loader bugs, the same class as preview extraction: CVE-2026-27631 (a huge vector) came from a new preview fuzz target, and CVE-2026-27596 (a 28-byte header subtracted without a minimum) was reported from outside |
| CVE-2026-52834 (RUSTSEC-2026-0151) | jxl-oxide (`jxl-grid`), Rust | 32-bit length overflow leading to an undersized buffer | A Rust image-decoder CVE: element counts need checked arithmetic in Rust too |

Exiv2's advisories for 0.28.9 (2026-08-30) also list CVE-2026-49275,
CVE-2026-68546 and CVE-2026-68547, which had no NVD or CVE.org record when
checked. [14] **[P]**

## 5. Checklist: the Sony layer

The NEF checklist items 1 to 14 apply to ARW unchanged and are not repeated.
The items below add what Sony files need. **They are numbered A1 onwards**
so they cannot be confused with the NEF items, and they refer to them as
"NEF item n". Each names the structure, the rule and the evidence; "(when
relevant)" means the item applies once the milestone that introduces the
feature lands (see [the roadmap](../../ROADMAP.md)). Structures are
described in [container-and-previews.md](container-and-previews.md),
[makernote-and-metadata.md](makernote-and-metadata.md) and
[raw-data.md](raw-data.md).

### Milestone 2: previews

A1. **Walk the whole IFD0 chain, with a visited set.** From FileFormat 4.0
    on, the chain is IFD0 → IFD1 → IFD2 and the full-size JPEG is in IFD2;
    before 4.0 the chain ends after IFD1 (IFD2 in all 16 sampled files of
    FileFormat 4.0 or later, raw.pixls.us file 4466 among them, and in none
    of the earlier ones). **[F]** The same IFD can be reached
    twice: IFD2 also sits 6 bytes after the SR2Private tag 0x7240 offset,
    and on ARW 2.3.x files that position is the raw SubIFD. **[F]** So the
    visited set (NEF item 2) is needed for de-duplication as well as for
    loops; cap the chain length and the total IFDs. Do not assume IFD0 is
    at offset 8 (all samples have it there, but re-saving tools may move
    it). **[S]**

A2. **The preview finder must reject raw data that looks like a JPEG.**
    Every lossless raw tile starts `FF D8 FF C3` (raw.pixls.us files 4466,
    6929, 6930, 6931). **[F]** The lossless raw IFD also carries a strip
    whose offset is the first tile and whose length is the sum of the
    tiles (file 4466), so a rule that accepts any strip of Compression 7
    picks raw data. **[F]** Rules: skip IFDs that are raw by role
    (Photometric 32803 CFA, 32892 ARQ, YCbCr 6 with BitsPerSample above 8,
    any IFD with tile tags); walk markers inside the declared range only
    (NEF item 12) and reject the lossless frame markers C3, C7, CB and CF;
    rank by the SOF size, never by tag dimensions (the A100's IFD0 claims
    3872×2592 for a 640×480 JPEG, file 2346 **[F]**). The IFD2 preview has
    Compression 7 although it is a baseline JPEG, so Compression alone
    does not separate preview from raw. **[F]** Declared lengths can run
    past the EOI marker (4 to 51 bytes of padding on older bodies) **[F]**;
    returning the declared bytes is harmless. Evidence: CVE-2020-15503,
    CVE-2026-27596, CVE-2026-27631; the viewer CVEs in 1.2 for context.

A3. **The A100's 0x014A is not a SubIFD, and its SR2Private is not an
    IFD.** In camera-original A100 files, IFD0 tag 0x014A holds the raw
    data offset (98304 in file 2346, where the bytes do not parse as an
    IFD) **[F]**; only Sony's converter turns it into a real SubIFD. **[P]**
    Follow 0x014A only after checking that the target looks like an IFD
    (sane entry count, all entries inside the file), never by model name,
    and never let a SubIFD special case advance an index into a fixed-size
    table. The A100's 0xC634 points at a little-endian Minolta MRW block
    starting `\0MRI` (file 2346, offset 72606) **[F]**: parse it by its own
    segment lengths inside the file and reject zero-length segments, which
    would loop without advancing. The MakerNote's Minolta pointer 0xB028
    and its preview tags 0x0088/0x0089 are file-relative in file 2346 and
    point at the same JPEG as IFD0 **[F]**; de-duplicate candidates by
    offset. Evidence: CVE-2017-6887; rawspeed's A100 block reader.

A4. **Read NewSubfileType as `u32` and never narrow it.** Measured values
    are 1 on IFD0, IFD1 and IFD2 and 0 on the raw SubIFD; SRF files omit
    it. **[F]** Treat any other value as "role unknown" rather than an
    index. Evidence: CVE-2026-88387 (section 1.4).

### Milestone 3: metadata

A5. **No next-IFD pointer in Sony MakerNotes and the SR2SubIFD.** In modern
    files the four bytes after the entries are value data (values such as
    33,554,944 or 52,429,600 in the SR2SubIFD) **[F]**, so following them as
    a next-IFD offset walks into garbage. Do not follow a next pointer from
    these IFDs, and do not require sorted tags. The A100 MakerNote has the
    12-byte `SONY DSC` header (file 2346); every other sampled ARW, SR2 and
    ARQ MakerNote is headerless, including the Hasselblad Lunar (file
    4051). **[F]** Milestone 3 needs no decryption: make, model,
    orientation, dates and the crop tags are plain in every file. **[F]**

### Milestone 5: raw data

A6. **SR2Private as a bounded sub-slice (when relevant).** Black level,
    white level and white balance of every ARW 2.x and later file are in the
    encrypted SR2SubIFD (newer files also carry plain copies). Read 0xC634
    as four bytes holding a `u32` (it is BYTE[4] in every file **[F]**; a
    check that demands type LONG loses it). Read 0x7200 offset, 0x7201
    length and 0x7221 key (count must be 4). Require `offset + length <=
    file length` with `checked_add`, and a configurable cap: LibRaw uses
    10,240,000 bytes **[P]**; the measured lengths are 11,932 to 62,112
    bytes (all files in [test-data.md](test-data.md#files-checked)) **[F]**,
    so a cap of 1 MiB leaves a wide margin **[I]**. Allocate only after
    these checks and decrypt only bytes actually present. The decrypted IFD
    uses absolute file offsets: rebase each value offset with a
    `checked_sub` of the block offset, and skip any entry whose data falls
    outside the block. Skip entries of unknown TIFF type instead of looking
    the type up in a table. Treat decrypted values as plausible or ignored
    (NEF item 10): a wrong key gives garbage, not an error. Evidence:
    OSV-2023-1295; LibRaw 8d44074, 54c6af9, 9c9c04b; rawspeed bbd6d363;
    CVE-2020-27855 for context.

A7. **Keystream length and arithmetic (when relevant).** Round the length
    down to a multiple of four and refuse a result of zero. Seed with
    `wrapping_mul` and `wrapping_add` on `u32` (in Rust an overflow panics
    in debug builds; LibRaw's signed version was undefined behaviour until
    0.20.0); keep the generator state in a fixed 128-word ring indexed
    modulo 128, and generate one word per data word as it goes, never a
    keystream buffer sized from the file. The key is the constant bytes
    `11 22 33 44` in every sampled ARW and different in the DSC-R1 SR2
    (file 3221) **[F]**; do not depend on either. Evidence: OSS-Fuzz 7432
    and 9465; rawspeed bbd6d363.

A8. **Dispatch by agreeing facts, never by one tag or a model name (when
    relevant).** Choose the scheme from Compression, Photometric,
    SamplesPerPixel and the strip size compared with W·H (cRAW), 1.5·W·H
    (12-bit packed), 2·W·H (16-bit) and 8·W·H (ARQ), each product computed
    with checked arithmetic. Do not trust BitsPerSample (8, 12 and 14 all
    occur with exactly W·H cRAW bytes) or SonyRawFileType (an A7R II
    12-bit uncompressed file says 0, every lossless file says 4, an A200
    ARW1 stream says 3). **[F]** Anything that matches no scheme returns a
    typed unsupported error. Evidence: CVE-2017-6887 (keyed on model and
    width), CVE-2021-34335 (keyed on a model string, then divided by a tag).

A9. **cRAW geometry (when relevant).** Require a width that is positive and
    a multiple of 32, and a strip of exactly W·H bytes. Every measured cRAW
    width is a multiple of 32 **[F]**, so rejecting others costs nothing on
    real files. Never compute `width - 30` in unsigned arithmetic. Read
    each 16-byte block as a whole and nothing beyond it. Reject a block
    whose maximum and minimum indices are equal (14 deltas cannot fill 15
    pixels) and flag one whose maximum is below its minimum: neither
    occurred in more than 500,000 blocks over 15 files. **[F]** Keep the
    shift at most 4 and clamp each value to 2047 before the curve lookup.
    Evidence: rawspeed a3a8b581, a74c1a57, b9f5b8b1; LibRaw cc54bb6.

A10. **Tone-curve bounds and values above 14 bits (when relevant).** Require
     exactly four values in 0x7010; shift each right by 2 and mask to 12
     bits; build the 4096-entry table in `u32` or prove each step fits
     `u16` (the worst case, all knots zero, ends at 65,520). The lookup
     index is twice an 11-bit value, so at most 4094: index with that
     proven bound, or with `get`. Decreasing knots skip a segment; handle
     that without underflow. The output exceeds 14 bits: the knots 8000,
     10400, 12900 and 14100 of raw.pixls.us file 1541 give 17,204 at the
     top code. **[F]** for the knots, **[I]** for the arithmetic. So do not
     store the result in a 14-bit type or assume a white level of 16383.
     The NEX-5's first knot is 0 (file 1457) **[F]**, so a zero knot is
     legitimate, not a sign of corruption. Evidence: CVE-2018-25017 (the
     rawspeed lookup that carries this curve).

A11. **ARW1 range (when relevant).** The A100, and at least the DSLR-A200,
     store a column-major running-sum stream (files 2346 and 2271 decode
     with no range violation). **[F]** Keep the running sum in 0 to 4095 at
     every step and treat a violation as an error; cap the code category
     at 12; write only rows below the image height, because the stream
     covers more rows than the image. Evidence: rawspeed fa488def.

A12. **Lossless tiles: count, geometry and order (when relevant).** Require
     as many TileOffsets as TileByteCounts and a tile count equal to
     ⌈W/tw⌉ × ⌈H/th⌉, checked by division. Measured tiles are 512×512, and
     each is one lossless JPEG frame of 256×256 samples with 4 components,
     precision 14, predictor 1 (files 4466, 6929, 6736, 8848). **[F]**
     Check the frame's size × components against the tile area before
     decoding. TileOffsets are in row-major tile order, but the tiles are
     stored column by column (in file 4466, tile 0 is followed by tile 17)
     and padded with 0 to 15 bytes of `FF` after EOI to a 16-byte boundary.
     **[F]** So never
     assume tile i + 1 follows tile i, never derive a tile's length from
     the next offset, and stop at EOI inside the tile's own range. Check
     every range is inside the file and that ranges do not overlap. The
     image size is rounded up to whole tiles (8704×6144 against a usable
     area of 8664×5784 in file 4466) **[F]**: clip output to the raster.
     Compute tile positions in `u64`, and make the bit reader return an
     error at the end of the tile instead of reading ahead. The Huffman
     tables come from each tile, so the DHT parsing class applies.
     Evidence: rawspeed 0953acf8, 8d37641f; LibRaw 17f1683, 4e265f1,
     issue #792; CVE-2026-5318, CVE-2026-20911.

A13. **M/S-size YCbCr precision, and ARQ sample counts (when relevant).**
     M and S lossless files are YCbCr, not CFA: BitsPerSample says 15 but
     each tile's frame header says precision 16, with 3 components in
     512×512 frames, subsampled 4:2:0 (M) or 4:2:2 (S) (files 6930 and
     6931). **[F]** Take the precision from the frame header, accept up to
     16, and check the subsampling in the frame against YCbCrSubSampling.
     These IFDs have no WhiteLevel tag and a CFAPattern of `255 255 255 255`
     **[F]**: handle the absence, and never use those pattern bytes as an
     index into a colour table. Cap the tile count and the compressed and
     decoded size of each tile (LibRaw 4e265f1 caps them at 1 GB and 2 GB).
     ARQ is Photometric 32892 with four samples per pixel and B = 8·W·H:
     340,480,000 bytes for 8000×5320 in file 3255, with every value at most
     16383. **[F]** Require
     SamplesPerPixel 4 and the exact size with checked arithmetic, and
     bound the allocation by the bytes present and a configurable pixel
     cap: a 16-shot ARQ doubles both dimensions, beyond 240 megapixels.
     Evidence: LibRaw 17294b5.

A14. **ARW 6.0: detect and refuse before any decoding (when relevant).**
     Compression 32766 (ILCE-7M5 and ILCE-7RM6) returns a typed unsupported
     error before reading a byte of the strip, including its tile header.
     Decide on Compression alone, not on FileFormat: an ARW 6.0 file can be
     lossless Compression 7, which is fine to decode (file 8848), while
     files 8846 and 8847 use 32766. **[F]** Previews and metadata from ARW
     6.0 files are unaffected. This is the same posture as NEF HE (NEF item
     11); why ARW 6.0 is out of scope is in
     [licences-and-references.md](licences-and-references.md). If the
     refusal is ever lifted, LibRaw's new decoder (section 2.1) is the only
     public implementation, so expect new bugs: bound the tile count and
     rectangles, the per-region sizes and the quantiser shifts.

A15. **SRF key chain: never hard-code 200896 without bounds (when
     relevant).** In the DSC-F828 sample (file 1351) the MakerNote starts at
     164,580, the key byte is at 200,896 (MakerNote + 0x8DDC), the first
     encrypted IFD at 164,600, the data key at its decrypted byte 22, and
     the encrypted image at 862,144 for 16,531,200 bytes, running exactly
     to the end of the file. **[F]** dcraw, LibRaw, rawspeed and rawler use
     these as absolute constants **[P]**; rawler panics on a 44,210-byte
     file (section 3). Derive every position from the MakerNote and check
     each one against the file: the key byte b, the key word 4·b bytes
     after it, each IFD and its entries, and the data range
     (`width × height × 2` with checked arithmetic, compared with the
     bytes present). Next-block positions are absolute and must not precede
     the current block; entry counts are 1 to 1000. The later SRF IFDs
     could not be located in the sample **[F]**, so a decoder that needs
     them must fail soft. Evidence: CVE-2020-35535; LibRaw 02e24bf and the
     2019 series; rawler #574.

### Not planned

A16. **Enciphered MakerNote blocks (0x2010, 0x9050, 0x94xx).** No milestone
     needs them: they hold extras such as shutter count and aspect ratio.
     If they are ever read: pick the layout only from checked data, require
     every fixed offset to be inside the block before reading, and treat a
     short or unknown block as absent. Layout selection is fragile: the A7
     V writes 0x9400 version byte 0x41, which LibRaw does not know; the
     0x9050 block of file 7835 (A1 II, APS-C) is mostly `FF` (219 of 256
     bytes, `FF` at the count offsets); and the 0x9050
     values of the ILCE-9M3 and ILCE-6700 samples are not shutter counts at
     the documented offsets. **[F]** Assemble multi-byte values with
     `from_le_bytes`, never with shifts of the type's width or more. The
     substitution itself (b³ mod 249 on bytes 0 to 248) maps a byte to a
     byte and cannot go out of range. Evidence: LibRaw ba673ca, c2e8a90,
     the 0.18.2 shift fix.

### All milestones

A17. **Fuzz the Sony paths explicitly.** Extend the NEF item 14 targets
     with seeds for: an IFD0 chain with IFD2 and a 0x7240 link back into it,
     an A100-style 0x014A data pointer and MRW block, an SR2Private block,
     a lossless tile table out of order, a cRAW strip, an ARQ header and an
     SRF MakerNote shorter than 200,896 bytes. Nearly every Sony bug in
     sections 2 and 3 was found by a fuzzer, and the rawler reports are
     ready-made regression inputs. Keep each crash reproducer as a small
     committed file with a test shown to fail without the fix.

## 6. Reporting

Report a security problem in raw-files privately, as described in
[SECURITY.md](../../../SECURITY.md), not in a public issue.

## 7. Open questions

1. **IDC-edited files.** No sample has a Sony converter edit: 0x7241 is 0
   everywhere. Which offsets in the 0x7240/0x7241 IFDs and their version
   IFDs are relative to the IFD's own start, and does the 0x7240 + 6 link to
   IFD2 survive an edit? Needs an ARW edited in Image Data Converter or
   Imaging Edge.
2. **Re-saved files.** Every sample has IFD0 at offset 8. A file re-saved by
   a third-party tool (IFD0 reportedly moved to the end) would test chain
   walking and the visited set on a real layout.
3. **SRF later IFDs.** Where SRF2 to SRF6 start in file 1351, and with which
   key; this decides whether the SRF chain can be walked without dcraw's
   absolute constants. A DSC-V3 file (data offset 787,392 in dcraw) would
   test the same rule on a second body.
4. **Enciphered block lengths.** Actual lengths of 0x2010 and 0x9050 on, for
   example, the ILCE-9, ILCE-1, ILCE-7M4 and ILCE-7M5 against the largest
   fixed offset LibRaw and ExifTool read (for example 0x188C in the last
   0x2010 layout). Only matters if A16 is ever implemented.
5. **cRAW highlights.** No sample reaches code 2047. A clipped highlight on
   a cRAW body (an ILCE-9 or ILCE-7RM3 file) would show whether outputs
   above 16383 occur in practice (A10).
6. **ARW1 and non-multiple-of-32 widths.** Raw width 4600 (A290, A350,
   A380, A390) is not a multiple of 32. Do those bodies write ARW1, cRAW or
   something else? If cRAW, item A9's width rule needs a defined partial-block
   behaviour.
7. **12-bit fallback modes.** Silent shutter, bulb and long-exposure noise
   reduction drop some bodies to 12 bits. Do they change the strip size,
   the 0x7010 curve or the levels in a way the dispatch rule (A8) misses?
8. **A100 firmware 1.00.** The reported thumbnail offset 0x10000 too small
   needs a firmware 1.00 file; any repair that reads at offset + 0x10000
   must be bounds-checked like any other offset.

## Sources

All retrieved in October 2026.

[1] NVD, CVE-2017-6887 —
    https://nvd.nist.gov/vuln/detail/CVE-2017-6887 ;
    https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=CVE-2017-6887
    **[P]**
[2] NVD, CVE-2020-35535 —
    https://nvd.nist.gov/vuln/detail/CVE-2020-35535 ;
    https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=CVE-2020-35535
    **[P]**
[3] NVD, CVE-2021-34335 —
    https://nvd.nist.gov/vuln/detail/CVE-2021-34335 ; Exiv2 advisory
    https://github.com/Exiv2/exiv2/security/advisories/GHSA-pvjp-m4f6-q984 ;
    `src/minoltamn_int.cpp` at v0.27.4 —
    https://raw.githubusercontent.com/Exiv2/exiv2/v0.27.4/src/minoltamn_int.cpp
    **[P]**
[4] NVD, Foxit Studio Photo —
    https://nvd.nist.gov/vuln/detail/CVE-2020-17423 ,
    https://nvd.nist.gov/vuln/detail/CVE-2020-17434 ,
    https://nvd.nist.gov/vuln/detail/CVE-2020-27855 ,
    https://nvd.nist.gov/vuln/detail/CVE-2021-31433 ; keyword listing
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=foxit%20studio%20photo
    **[P]**
[5] NVD, CVE-2024-11520 (IrfanView) —
    https://nvd.nist.gov/vuln/detail/CVE-2024-11520 **[P]**
[6] NVD keyword listings `sony`, `arw`, `sr2`, `srf`, `arq`, `sony arw`,
    `sonymn`, `sony makernote`, `exiv2 sony`, `libraw sony`,
    `imaging edge`, `playmemories`, `exiftool` —
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=<term>
    **[P]**
[7] NVD, CVE-2026-88387 —
    https://nvd.nist.gov/vuln/detail/CVE-2026-88387 **[P]**
[8] LibRaw git history at master 7bfffe2 (2026-10-09) and tags 0.18.1 to
    0.22.2 — https://github.com/LibRaw/LibRaw/commit/<sha> for cc54bb6,
    d7c3d2c, 4d2aec2, ba673ca, 8d44074, 02e24bf, 1375b07, af705b8, 5d0707d,
    3e2cc80, 74c2689, b1c2fcd, f9ff88f, 2099f64, 72e05585, c243f45, 35d7083,
    17294b5,
    62f0423, a6f212a, c2e8a90, 54c6af9, 9c9c04b, 17f1683, 4e265f1, b41cbbd,
    dde798d; source files `src/metadata/sony.cpp`, `src/metadata/tiff.cpp`,
    `src/decoders/decoders_dcraw.cpp`, `src/decoders/sonycc.cpp` **[P]**
[9] LibRaw issues and pull requests #124, #218, #231, #244, #246, #251,
    #283, #646, #792, #810, #824, #844, #853 —
    https://github.com/LibRaw/LibRaw/issues/<n> **[P]**
[10] LibRaw Changelog at tags 0.18.3, 0.21.5 and 0.22.2 —
    https://raw.githubusercontent.com/LibRaw/LibRaw/0.22.2/Changelog.txt ,
    https://raw.githubusercontent.com/LibRaw/LibRaw/0.21.5/Changelog.txt
    **[P]**
[11] OSS-Fuzz vulnerability records OSV-2023-1295 (LibRaw) and
    OSV-2018-227 (rawspeed) — https://github.com/google/oss-fuzz-vulns ;
    https://osv.dev/vulnerability/OSV-2023-1295 **[P]**
[12] rawspeed at develop c10cc500 (2026-10-10) —
    https://github.com/darktable-org/rawspeed/commit/<sha> for a3a8b581,
    376e0d1b, 46eae2f7, 35fe49b5, 5ff5484d, 582bf5d0, 7e51ed90, a74c1a57,
    fa488def, dbe7591e, b9f5b8b1, bbd6d363, 0953acf8, 8d37641f; source
    `src/librawspeed/decoders/ArwDecoder.cpp` and the Sony decompressors
    **[P]**
[13] Exiv2 issues #2380 and #2385, pull requests #1753 and #1786, commits
    3897f17b, 1119a68a, 11a26fb0, dc2c77ce —
    https://github.com/Exiv2/exiv2/issues/2385 ,
    https://github.com/Exiv2/exiv2/commit/1119a68a **[P]**
[14] Exiv2 security advisories (37) —
    https://github.com/Exiv2/exiv2/security/advisories ; NVD keyword listing
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=exiv2
    **[P]**
[15] dcraw 9.28, read from a mirror because the original site did not
    respond — https://raw.githubusercontent.com/ncruces/dcraw/master/dcraw.c
    (original https://www.dechifro.org/dcraw/dcraw.c) **[P]** (mirror not
    verified against the original)
[16] dnglab (rawler) issues #514, #569 to #575, #582, #797 —
    https://github.com/dnglab/dnglab/issues/<n> ; `rawler/src/decoders/arw.rs`
    at main ae01bcb2 (2026-08-30) —
    https://raw.githubusercontent.com/dnglab/dnglab/main/rawler/src/decoders/arw.rs
    **[P]**
[17] rawloader issue #54 — https://github.com/pedrocr/rawloader/issues/54
    **[P]**
[18] RustSec advisory database at 7eebec6 (2026-10-09) —
    https://github.com/rustsec/advisory-db **[P]**
[19] NVD records for the section 4 entries —
    https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=<ID> ; keyword
    listings `libraw`, `libexif`, `libtiff`, `dng sdk`, `Image RAW` **[P]**
[20] NVD keyword listings `Image RAW`, `camera codec`, `camera raw`,
    `ExifInterface`, `piex`, `Adobe Camera Raw` **[P]** (Apple and Android
    searches keyword-only)
[21] ExifTool Sony tag names — https://exiftool.org/TagNames/Sony.html
    **[P]**
[22] raw.pixls.us sample files cited on this page, all CC0; the full list is
    in [test-data.md](test-data.md#files-checked). **[F]**
    - file 1351, Sony DSC-F828 (SRF) —
      https://raw.pixls.us/getfile.php/1351/nice/Sony%20-%20DSC-F828%20-%204:3.SRF ;
      SHA-256 `07739f94bcab0c7e198d418ac65172d0dce04bb84a7683ecffd0bfe4b32f6d79`
    - file 1457, Sony NEX-5 —
      https://raw.pixls.us/getfile.php/1457/nice/Sony%20-%20NEX-5%20-%2012bit%2012bit%20compressed%20(3:2).ARW ;
      SHA-256 `3b085569ac1fec37a1289ed5ea517497692a3ea14cea41953dd6e878abf230a3`
    - file 1541, Sony ILCE-9 —
      https://raw.pixls.us/getfile.php/1541/nice/Sony%20-%20ILCE-9%20-%2014bit%2014bit%20compressed%20(3:2).ARW ;
      SHA-256 `598bbc828c279528f504c51a10418521b17a90a25b198368f65e19cfda9e3711`
    - file 2271, Sony DSLR-A200 —
      https://raw.pixls.us/getfile.php/2271/nice/Sony%20-%20DSLR-A200%20-%2012bit%2012bit%20compressed%20(3:2).ARW ;
      SHA-256 `04eacd403d92e0c90da9d91b87615af8eda1afc963350d3ab774557c36763eef`
    - file 2346, Sony DSLR-A100 —
      https://raw.pixls.us/getfile.php/2346/nice/Sony%20-%20DSLR-A100%20-%20compressed%20(3:2).ARW ;
      SHA-256 `4df7981ac4baec9648b4f106751ea4b09a6e27bc70a15f8ef09bec051fec362d`
    - file 3221, Sony DSC-R1 (SR2) —
      https://raw.pixls.us/getfile.php/3221/nice/Sony%20-%20DSC-R1%20-%2014bit%2014bit%20uncompressed%20(3:2).SR2 ;
      SHA-256 `921c5f2513dd1671a9089e73fc778fc3573bc4cb70a3a555d0414661d0e62274`
    - file 3255, Sony ILCE-7RM3 (ARQ) —
      https://raw.pixls.us/getfile.php/3255/nice/Sony%20-%20ILCE-7RM3%20-%2014bit%2014bit%20uncompressed%20(3:2).ARQ ;
      SHA-256 `67af1f9f4cff82bd182f3fc48e95d39ae7d8352595e3c72f3386c9dee114f27d`
    - file 4051, Hasselblad Lunar —
      https://raw.pixls.us/getfile.php/4051/nice/HASSELBLAD%20-%20Lunar%20-%2012bit%20(3:2).ARW ;
      SHA-256 `029b1283a3d1226e08fc1eb3b81874347f5c128e437f78f853436fd6ba15873b`
    - file 4466, Sony ILCE-1 (lossless) —
      https://raw.pixls.us/getfile.php/4466/nice/Sony%20-%20ILCE-1%20-%2014bit%20(4:3).ARW ;
      SHA-256 `4012536ee0f938d8c5c5e932bb7953a03134abbd9f3f4c664cb2526c1903d492`
    - file 6736, Sony ILCE-6700 (lossless) —
      https://raw.pixls.us/getfile.php/6736/nice/Sony%20-%20ILCE-6700%20-%2014bit%20(4:3).ARW ;
      SHA-256 `dc3e2ed22f46fcef778553767e0703bc37cfb043f107afe2d7e6e4dbca34200a`
    - file 6929, Sony ILCE-7M4 (lossless L) —
      https://raw.pixls.us/getfile.php/6929/nice/Sony%20-%20ILCE-7M4%20-%2014bit%20(4:3).ARW ;
      SHA-256 `851b43c2116c4139104a5036f83ac3b6a148789b2142214dd7192c13972b25b6`
    - file 6930, Sony ILCE-7M4 (lossless M) —
      https://raw.pixls.us/getfile.php/6930/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW ;
      SHA-256 `d453005714327addd75bcb99c1c6223173dc92f2a59542d82e6761ef0e0e7571`
    - file 6931, Sony ILCE-7M4 (lossless S) —
      https://raw.pixls.us/getfile.php/6931/nice/Sony%20-%20ILCE-7M4%20-%204:3.ARW ;
      SHA-256 `cbbd0930c7d8706dff84c68a2004454266e6fd0d8354f5f76a106b5d776e0223`
    - file 7835, Sony ILCE-1M2 (APS-C) —
      https://raw.pixls.us/getfile.php/7835/nice/Sony%20-%20ILCE-1M2%20-%2014bit%20Lossy%20(3:2).ARW ;
      SHA-256 `050c924e5f78afd8479aa442f17b8650e0a5ebfa5757e57368cb0dc865ca2417`
    - file 8846, Sony ILCE-7M5 (compressed HQ, ARW 6.0) —
      https://raw.pixls.us/getfile.php/8846/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW ;
      SHA-256 `4e3a99cbcf2a348decaf3ca68c5de22b7a05eaa590f4e9184735c1f9c45213a7`
    - file 8847, Sony ILCE-7M5 (compressed, ARW 6.0) —
      https://raw.pixls.us/getfile.php/8847/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(3:2).ARW ;
      SHA-256 `46d73475dfd8f96fd7e2f4d4a721a36cbbaba3d6ea95a4851809cd9d252b2a80`
    - file 8848, Sony ILCE-7M5 (APS-C lossless) —
      https://raw.pixls.us/getfile.php/8848/nice/Sony%20-%20ILCE-7M5%20-%2014bit%20(4:3).ARW ;
      SHA-256 `51ad709cba01a5f05e3c723b779f6b5fda8fa7b378494dbb40b4d3d3aa5494a4`
