# NEF: test data

Real NEF files are large and their licences vary, so the raw-files tests
combine small committed fixtures that we build ourselves with larger public
samples that CI downloads and checks against a hash. This page lists the
sources, the rules for what may be committed, a minimal set of camera bodies,
and how to build fixtures. Back to [the NEF overview](README.md).

Confidence tags: **[P]** primary (verified from the primary page or API),
**[S]** secondary, **[I]** inferred. Numbers in brackets point to the
[Sources](#sources) at the end, retrieved in October 2026.

## 1. Sources and their licence status

| Source | Licence status | Commit to the repository? |
|---|---|---|
| raw.pixls.us | The upload form asks contributors to release files under CC0. The site also offers a view of "non-CC0 samples", so the archive is mixed, and files inherited from rawsamples.ch were originally largely CC BY-NC-SA. The file listing carries no per-file licence. [1][2] **[P]** / **[S]** | Only a file whose licence you confirmed as CC0 through the site's licence view, and only if small. Normally download in CI instead (section 2). |
| rawsamples.ch (the earlier archive) | Mostly CC BY-NC-SA 2.5 CH per contributor; the site was damaged in 2016 and replaced by raw.pixls.us. [3] **[S]** | No. The non-commercial and share-alike terms do not fit `MIT OR Apache-2.0`. |
| Camera-review galleries (Imaging Resource, DPReview) | Proprietary, all rights reserved; terms not read. [I] | No, and do not redistribute. Manual experiments only. |
| ExifTool sample images | No licence stated. The site says they are JPEG-only, with the main image replaced "to avoid copyright problems". The repository's `t/images` likewise states none. [4] **[P]** | No. Treat as unlicensed; use locally if at all. |
| Fuzzing corpora from other projects | Seeds come from those projects' test data, so their licences are unknown. [5] **[S]** | No. Generate our own corpus from our own fixtures. |
| Files you made yourself | Yours to licence. | Yes, with provenance recorded (section 4). |

## 2. What is committed and what is downloaded

**Committed** (small, a few KB each, licence clear):

- hand-built fixtures: an IFD skeleton, a MakerNote and a tiny JPEG;
- fixtures stripped from a CC0 sample: pixel data removed, the structure
  kept;
- corrupted variants of those (section 5);
- minimal reproducers for bugs, each added with its fix.

**Downloaded in CI** (large, 13 to 55 MB per file on the site [1]):

- real samples from raw.pixls.us, fetched by URL and **pinned by SHA-256**.
  The site publishes a `filelist.sha256`; CI records the hash of each file
  it uses in the repository and fails if the download differs. [1] **[P]**
- Before adding a file to the list, check its CC0 status in the site's
  licence view and note that in the list entry.
- Tests that need these files are skipped, not failed, when they are absent,
  so contributors can run the suite offline.

The repository never holds a file whose licence is unclear, however small.

### 2.1 The CI set

The files CI downloads are listed in
[`tests/samples.tsv`](../../../tests/samples.tsv), the source of truth for
their names, hashes, licences and URLs; `scripts/fetch-samples.sh`
downloads them and checks each hash, and the tests in `tests/nef.rs` read
them. Today the set is the three Z 9 files of raw.pixls.us, one per
compression mode:

| Id | Body | Compression | SHA-256 | Licence |
|---|---|---|---|---|
| 5146 | Z 9 | Lossless | `99d0fe24a38f0f7623aeab21440d266a5ac09f9665cf5283f590337e84323c38` | CC0 |
| 5147 | Z 9 | HE* | `e113deb305d84153b26251ce259d1f839be1f36b048cbe83a4548cbda806c0f5` | CC0 |
| 5148 | Z 9 | HE | `be91bac25573adf7ec6320a542105d87b4fb9c956d7b620ec90d57ac2ae6c3f0` | CC0 |

The site's repository data, 2026-10-13, gives each file the CC0 mark and
this SHA-256, and the Z 9 is not among its non-CC0 models; the compression
mode is in the name the file was uploaded under, which the directory
listing shows. [1] **[P]** The site's download link names all three
files alike, "Nikon - Z 9 - 8bit 8bit compressed (3:2).NEF", so the list
gives each its own name. The "8bit" is the site's label, not the files':
all three hold 14-bit raw data (BitsPerSample 14 in the raw IFD), as the
upload names say. Keep the URLs as they are.

The container and preview facts tagged **[F]** on the other NEF pages were
measured on these three files on 2026-10-13, so this section is where the
**[F]** tag points.

## 3. What raw.pixls.us holds for Nikon

Directory listings, 2026-10-10. **[P]** The data lives under
`data/Nikon/` and `data/NIKON CORPORATION/` (the latter lists D200, D2X,
D2Xs, D5300, Z5_2, Z6_3, Z50_2, Z fc and COOLPIX P1100). [1]

- **D-series:** D1, D1H, D1X, D2H, D2Hs, D2Xs, D3, D3S, D3X, D4, D4S, D5,
  D6, D40, D40X, D50, D60, D70, D70s, D80, D90, D100, D200, D300, D300S,
  D500, D600, D610, D700, D750, D780, D800, D800E, D810, D850, D3000 to
  D3500, D5000 to D5600, D7000 to D7500, Df.
- **Z-series:** Z 5, Z 6, Z 6_2, Z 7, Z 7_2, Z 8, Z 9, Z 30, Z 50, Z f,
  Z fc.
- **Compacts (NEF or NRW, not checked per file):** A, A1000, B700, E5400,
  E5700, E8400, E8800, P330, P340, P950, P1000, P6000, P7000, P7100, P7700,
  P7800.
- **Nikon 1:** J1 to J5, S1, S2, V1 to V3, AW1.
- **Scanners:** folders for Coolscan IV ED, Coolscan V ED and LS-5000 exist;
  their contents were not opened, so confirm they hold NEF or TIFF before
  relying on them.
- **Z 9 has three files** that cover its compression modes: "14bit
  compressed (Lossless)" (55 MB), "Lossy High Efficiency" (24 MB) and "Lossy
  High Efficiency Star" (34 MB). The Z f folder has three files (30, 19 and
  13 MB) whose compression modes were not verified.

## 4. Recommended minimal body set

Verify the licence flag for every file before adding it. "Why" says what the
body exercises.

| Generation | Body | Why |
|---|---|---|
| Early | D1 or D1X, D100 | 12-bit uncompressed and odd layouts (D100 pad bytes); headerless MakerNote (D1/D1X); the D100 uses the embedded-TIFF MakerNote |
| Mid | D70, D200 | Embedded-TIFF MakerNote; lossy type 1 with curve; the D200 MakerNote has no next-IFD pointer |
| Pro CMOS | D300, D3 | 14-bit lossless; newer MakerNote versions |
| 2012 to 2016 | D800 or D810, D850 | Several SubIFDs and previews; D810 has the YCbCr small-raw variant; D850 has M and S sizes |
| Flagship | D5, D6 | Newer MakerNote revisions; D6 has the 14-bit uncompressed packed layout |
| Mirrorless | Z 6, Z 7 | Z-mount MakerNote; "second version" lossy curve; 12-bit uncompressed |
| High efficiency | Z 9 (all three files), optionally Z 8 | The only way to test that HE and HE* are detected and refused |
| Recent | Z f | Newest body in the set |
| Compact | Coolpix A or P7000 (NRW), E5700 or E8800 (NEF) | The NRW variant, and 2003-era E-series layouts (even rows first, CMYG on some) |
| Scanner | Coolscan V ED | A TIFF-like "NEF" with no CFA; contents unverified |

The short-header MakerNote (`Nikon\0\1\0`) only occurs on Coolpix E700 to
E950, which have no NEF raw, so it is covered only by a hand-built fixture.

Plus corrupted variants (section 5) of the smallest of these.

## 5. Building fixtures

### 5.1 Tiny valid fixtures

A fixture is a few hundred bytes to a few KB containing only what a test
needs:

1. **IFD skeleton.** A TIFF header (`II` or `MM`, magic 42), IFD0 with Make,
   Model and Orientation, a SubIFD pointer, an Exif IFD pointer, and a
   raw-role IFD with a CFA pattern. Strip offsets point at a few dummy bytes
   rather than real sensor data.
2. **MakerNote.** The `Nikon\0` header and an inner TIFF header for the
   embedded layout, then an IFD with the tags under test (PreviewIFD,
   serial number, shutter count, compression). Offsets inside it are
   relative to the MakerNote's inner header, which is the easiest thing to
   get wrong, so cover it explicitly.
3. **Preview JPEG.** A valid JPEG of a few dozen bytes to a few hundred
   (a small solid-colour image). Make fixtures with several JPEGs of
   different sizes and put the largest in a different slot each time, so
   tests prove the choice is by measured size and not by position.
4. **Stripped real file.** From a CC0 sample, copy the headers and IFDs and
   replace the strip bytes with a short run. This keeps real-world
   structure that hand-building misses (padding, gaps, odd byte order).

Write the fixture generator as a test helper or small script in the
repository, not a binary blob nobody can regenerate. Where a binary fixture
is committed, a text file next to it records the provenance:

- how it was made (generated, or derived from which sample);
- for a derived fixture, the sample's name, source URL, SHA-256 and licence
  as checked;
- the date, and what the fixture is meant to exercise.

### 5.2 Corrupted variants

Derive these from valid fixtures so each bites one rule:

- truncated at every structurally interesting point (inside the header,
  inside an entry, in the middle of a preview, in the middle of the
  MakerNote);
- an IFD whose next-IFD pointer points to itself or to an earlier IFD;
- a SubIFD list that points back at its parent;
- offsets and lengths past the end of the file, and sums that overflow
  `u32` or `usize`;
- an entry whose count times type size overflows, or is huge;
- a MakerNote offset that points outside the MakerNote;
- a JPEG preview with a length larger than the file, or without `FF D8`;
- width, height or bit depth that overflow when multiplied, and strip sizes
  that disagree with the dimensions;
- zero divisors: a rational with denominator 0, strip height 0, tile size 0;
- a strip beginning with the HE marker bytes;
- a damaged or short 0x0096 block.

As CONTRIBUTING.md asks of regression tests, see each corrupted-input test
fail before relying on it.

## 6. How samples answer the open questions

The list of unresolved facts is in [open-questions.md](open-questions.md).
The set above answers many of them, mostly with `exiftool -v3`, `exiftool
-b` on a tag, or a hex dump of the first bytes. **[I]**

| Open question | Sample files that answer it |
|---|---|
| Preview dimensions and count per body; unusual JPEG types | D3 or D300, D850, D5, D6, Z 6, Z 7, Z 8, Z 9, Z f: list each SubIFD and the PreviewIFD, read each JPEG's frame header |
| Outer TIFF byte order per generation | The first eight bytes of one file per body; settles the Z bodies in particular |
| Where previews live on the earliest bodies | D1, D1X, D1H and the E-series |
| Whether HE and HE* change IFD or preview layout | The three Z 9 files side by side. Answered: they do not (section 2.1, files 5146 to 5148) |
| Whether tag 0x000C white balance is present and correct on Z bodies | Z 6, Z 7, Z 8, Z 9, Z f |
| Whether CropArea (0x0045) is always present, and raw-to-output size mapping | One file per body, compared with the Exif image size |
| Version bytes written by lossy type 1 bodies; what the 2110-byte skip is | `exiftool -b -NEFLinearizationTable` on D1H, D1X, D100, D70, D200, D2X; read the first two bytes |
| Order of the four 0x0096 predictor seeds (row-major or column-major) | Read the four seeds from 0x0096 on several bodies; if always equal the question is moot |
| Which bodies write NEFCompression 5 and 6 | `exiftool -NEFCompression` across the whole Nikon set |
| Black-level channel order on non-RGGB bodies | A non-RGGB body that carries tag 0x003D, such as D3100 or D90 if it does |
| D850 M and S internals | D850 files at each size: CFA pattern, photometric interpretation, strip size |
| Whether any Z body writes M or S NEF | Z 7 and Z 6 samples at M and S size, if the site has them |
| Make and Model padding (NUL versus spaces) | The raw entry bytes of a few files, recent bodies in particular (the D100 is known to space-pad Model) |

Findings from these checks should be written back into these NEF pages with
the file name, hash and the exact command used.

## Sources

[1] raw.pixls.us home page, upload terms and directory listings —
    https://raw.pixls.us/ , https://raw.pixls.us/data/Nikon/ ,
    https://raw.pixls.us/data/NIKON%20CORPORATION/ ,
    https://raw.pixls.us/data/Nikon/Z%209/ ,
    https://raw.pixls.us/data/Nikon/Z%20f/ **[P]**
[2] pixls.us, "New year, new raw samples website" (2017) —
    https://pixls.us/blog/2017/01/new-year-new-raw-samples-website **[S]**
[3] darktable, "rawsamples.ch replacement" (2017) —
    https://www.darktable.org/2017/01/rawsamples-ch-replacement/ **[S]**
[4] ExifTool, sample images — https://exiftool.org/sample_images.html **[P]**
[5] OSS-Fuzz, LibRaw project and enrolment overview —
    https://github.com/google/oss-fuzz/tree/master/projects/libraw ,
    https://github.blog/open-source/how-to-squash-bugs-by-enrolling-in-oss-fuzz/
    **[S]**
