# NEF: licences and references

Nikon publishes no open specification for NEF, and almost every mature
implementation is under a copyleft licence. This page records what we may
learn from, what we may reuse, and where the format is documented. Back to
[the NEF overview](README.md).

Confidence tags: **[P]** primary (verified from the primary page or API),
**[S]** secondary (forum, news report, search summary), **[I]** inferred.
Numbers in brackets point to the [Sources](#sources) at the end. All pages
were retrieved in October 2026.

**This is not legal advice.** It is a record of what we found and a proposal
for how to stay on the safe side; anything with legal weight needs a lawyer.

## 1. Nikon: no public specification

- NEF is Nikon's proprietary format. We found no public specification and no
  white paper. NX Studio, Nikon's own converter, is closed. [I]
- Bio-Formats says it holds a version 0.1 NEF specification from 2003 but
  cannot redistribute it, and rates NEF's openness as poor. [1] **[S]**
- In 2005 Nikon stated that NEF is proprietary and that an SDK is available
  to bona fide software companies on written application under a
  non-disclosure agreement. [2][3][4] **[S]** One report says that SDK
  returned converted JPEG or TIFF output rather than access to the raw data;
  that is unconfirmed.
- Today's Nikon developer portal offers a camera-control SDK. We could not
  load a page describing a RAW SDK, so its current availability and terms
  are **unverified**. Either way, an NDA-bound SDK cannot feed an open-source
  clean-room effort, and raw-files does not use one. [I]

Everything beyond TIFF and Exif is therefore reverse-engineered knowledge,
published by community projects (section 5) and read from sample files.

## 2. Standards NEF builds on

| Standard | State | Availability | Conf. |
|---|---|---|---|
| TIFF 6.0 (Adobe, 1992) | Unchanged since 1992 | Free PDF mirrors; copying the document commercially needs Adobe's permission, which does not restrict implementing the format [5] | S |
| TIFF/EP, ISO 12234-2:2001 | NEF's nominal base; edition 1, confirmed in 2026 per the ISO listing | Paid (the ISO store charges CHF 204); only the front matter is public [6] | S |
| Exif, CIPA DC-008 | 2.31 (2016), 2.32 (2019), 3.0 (2023, adds a UTF-8 type), DC-008-2024; current is **Exif 3.1** (DC-008-Translation-2026), published 2026-01-30 [7][8] | Free download from CIPA, which states no licence terms | P (3.1), S (earlier) |
| JPEG, ITU-T T.81 | Covers baseline and lossless processes | Free from ITU [9] | I |
| DNG 1.7.1.0 (Adobe, September 2023) | Not used by NEF itself | Free PDF and SDK [10] | P |

DNG carries a patent licence: royalty-free and worldwide for compliant
readers and writers, with a required notice in distributed implementations,
revocable if the licensee sues Adobe over DNG. [10] **[P]** The notice only
matters if raw-files ever reads or writes DNG (the roadmap lists DNG as a
possible later format); it does not apply to plain NEF. The page does not
state the licence of the DNG SDK source, so raw-files would not copy from it
without checking.

The standards give the container (IFD structure, entry types, SubIFDs, the
Exif IFD, the MakerNote tag). They say nothing about Nikon's MakerNote
contents or the raw data encoding.

## 3. Implementations

"Dependency candidate" means the licence is compatible with the raw-files
`MIT OR Apache-2.0` (see ADR002) and the project is worth evaluating.
"Read only, no copying" means we may study it to understand facts about the
format but copy neither code nor tables. SPDX identifiers are as reported by
each project's repository or by crates.io.

| Project | Language | Licence (SPDX) | NEF coverage | Reuse status |
|---|---|---|---|---|
| dcraw | C | Custom, no SPDX: restricted Foveon code requires source release or GPL-2.0-or-later; ambiguous for the whole file [11][12] | Decode, lossy and lossless; no HE | Read only, no copying |
| LibRaw | C++ | LGPL-2.1-only OR CDDL-1.0 [13] | Full decode, previews, metadata; HE/HE* not decoded | Read only, no copying |
| rawspeed | C++ | LGPL-2.1 [15] | NEF and NRW decoders (darktable's engine) | Read only, no copying |
| rawler / dnglab | Rust | LGPL-2.1 [16][17] | NEF and NRW; HE only in an open experimental PR | Read only, no copying |
| rawloader | Rust | LGPL-2.1 [17] | Bayer NEF and NRW decode | Read only, no copying |
| imagepipe, quickraw | Rust | LGPL-3.0-only, LGPL-2.1 [17] | Pipelines over the above | Read only, no copying |
| ExifTool | Perl | GPL-1.0-or-later OR Artistic-1.0-Perl [18] | Most complete Nikon tag tables | Read only, no copying |
| exiv2 | C++ | GPL-2.0-or-later (COPYING is the GPLv2 text; "or later" from source headers, not re-checked) [19] | Metadata, MakerNote, previews | Read only, no copying |
| libexif | C | LGPL-2.1 [20] | Exif and Nikon MakerNote, JPEG focus | Read only, no copying |
| libopenraw | C++ | LGPL-3.0 per GitHub [20] | NEF thumbnails and metadata | Read only, no copying |
| darktable, RawTherapee | C, C++ | GPL-3.0 | Via rawspeed, LibRaw or dcraw-derived code | Read only, no copying |
| rexiv2 | Rust | GPL-3.0-or-later (bindings to exiv2) [17] | Via exiv2 | Read only, no copying |
| `tiff` (image-rs) | Rust | MIT [21] | Generic TIFF; no MakerNote | Dependency candidate |
| `kamadak-exif` | Rust | BSD-2-Clause [21] | Generic Exif; no MakerNote decoding | Dependency candidate (keep the copyright notice) |
| `zune-jpeg` | Rust | MIT OR Apache-2.0 OR Zlib [21] | JPEG decoder, only if previews are ever decoded | Dependency candidate |
| `nom-exif` | Rust | MIT per GitHub; crates.io shows "non-standard" [21] | Exif and media metadata | Dependency candidate after reading its LICENSE file |
| `little_exif` | Rust | MIT OR Apache-2.0 (version entry; crate-level record empty) [21] | Exif read and write; NEF not listed | Dependency candidate, little relevance |
| rawkit (Graphite) | Rust | MIT OR Apache-2.0 [21] | Sony ARW only; no NEF | Permissive, but nothing to reuse for NEF |

All licences above were read from the GitHub API, the crates.io API or the
repository's licence file in October 2026. **[P]** except where noted. The
`image` crate is `MIT OR Apache-2.0` but does not decode NEF.

Licence identifiers are as reported by each project's GitHub licence
metadata or LICENSE file. Whether the GPL and LGPL entries are "-only" or
"-or-later" was not checked against source headers (rawspeed's source
headers are reported as LGPL-2.1-or-later).

### 3.1 Why "read only" is the rule

- ADR002 accepts only dependencies compatible with both raw-files
  licences; a GPL, LGPL or MPL dependency is excluded.
- Translating LGPL code into Rust is a derivative work, so it does not
  escape the licence.
- ExifTool's prose and the selection and arrangement of its tables are
  expression; individual tag IDs and value meanings are facts (see section
  4.1). The crate `revelo-exiftool-tables` (GPL-1.0-or-later OR
  Artistic-1.0-Perl) shows how that licence spreads to anything derived from
  the tables as a whole. [17] **[P]**

### 3.2 Small Rust crates that touch NEF

Crates.io has several small crates for NEF thumbnails or metadata. Their
licences are permissive but their maturity is low and unvetted. [17]

| Crate | Licence | Note |
|---|---|---|
| `rawlib` 0.7.1 | MIT | RAW thumbnail extractor; very few downloads |
| `jpgfromraw-lib` 0.7.2 | MIT | Embedded JPEG extraction |
| `fpexif` 0.0.3 | MIT OR Apache-2.0 | Pre-alpha, Exif for many formats |
| `nef-compactor` 0.3.0 | MIT | Parses NEF structure to recompress it |
| `rawshift-image` 0.1.1 | MPL-2.0 | File-level copyleft; read only |
| `raw_preview_rs` 0.1.2 | GPL-3.0 | Read only |
| `zenraw` | AGPL-3.0 or commercial | Wraps other decoders; read only |
| `rsraw` / `rsraw-sys` | LibRaw bindings | C FFI, not pure Rust; excluded |

Licence information is **[P]**; maturity is **[I]**.

### 3.3 Release facts worth knowing

- LibRaw 0.22.0: the project's Changelog dates it 2025-12-25, while the
  GitHub release is dated 2026-01-13. We do not know which is the intended
  date and cite both. [14] **[P]**
- LibRaw 0.22.1 was released on 2026-04-06 and 0.22.2 on 2026-07-16 (GitHub
  releases). [14] **[P]**
- HE and HE* (Nikon's High Efficiency modes) have no released open decoder.
  Two proposals are open and unmerged as of 2026-10-10: dnglab PR #835
  (experimental, tested on one body) and LibRaw PR #826. [28] **[P]**

## 4. Clean-room guidance for raw-files

### 4.1 Facts and expression

Copyright protects expression, not facts. Tag numbers, field offsets, the
meaning of a flag value and the order of fields in a structure are facts
about the format. Source code, its structure and comments, the wording and
layout of documentation, and a tag table's prose are expression. [I]

So the working rules are:

1. Learn the facts from as many independent sources as possible, and record
   each in our own words in `docs/formats/nef/`, with a source and a
   confidence tag.
2. Write the code from this documentation, not from the source of a copyleft
   project. Where practical, the person who reads third-party code is not
   the person who writes the raw-files code that implements the same thing.
3. Prefer to confirm a fact against a sample file over trusting any single
   implementation. Implementations disagree with each other in places
   (curve clamping, the "second version" lossy condition, the handling of
   non-numeric serial numbers), and a sample file settles which is right.
4. Never paste code, comments or tables from a GPL or LGPL source.

### 4.2 Tables that need a decision

Three kinds of data are needed by a decoder and are published in practice
only inside copyleft projects.

| Data | Where it is published | Why it is awkward |
|---|---|---|
| The symbol lists of the six Huffman tables used by compressed NEF (the counts per code length are given in [raw-data.md](raw-data.md)) | dcraw, LibRaw, rawspeed, rawler | Fixed in every decoder, not stored in the file; short numeric data repeated across projects |
| The two 256-byte substitution tables for MakerNote decryption | dcraw, LibRaw, ExifTool, exiv2 | Same: short, fixed, repeated; needed only for encrypted metadata |
| Per-body tables: CFA phase, crop trims, white levels, black levels | rawspeed's `cameras.xml`, dcraw and LibRaw `identify` | Large collections; much can be derived from sample files |

Whether a short numeric table that a file format requires is protected is
unsettled. Three options, to be decided by the maintainers:

- **(a) Treat as format facts.** Transcribe from at least two independent
  sources, check them against sample files, and record the provenance in the
  source file that holds the table.
- **(b) Derive empirically.** Reconstruct the table from sample files. This
  works for per-body values (white level, CFA phase, crop) and for the
  decryption tables, which can in principle be recovered from known
  plaintext. It does not work for the Huffman symbol lists, which are not in
  the file.
- **(c) Strict clean-room.** One person documents the table from a
  non-copyleft or neutral description; a different person writes the code.
  Heaviest, but cleanest.

We suggest (b) for per-body tables, which also keeps them small
and verifiable, and a decision between (a) and (c) before any Huffman or
decryption table is committed. Until that decision is recorded in an ADR,
we suggest committing neither table. The milestones that need them (raw sensor data,
encrypted metadata) come after previews and basic metadata, which need
neither. [I]

### 4.3 Out of scope: HE and HE*

HE and HE* are built on a licensed third-party codec (intoPIX TicoRAW, a
JPEG XS variant) and JPEG XS has a royalty-bearing patent pool. [28][29]
**[S]** Implementing a decoder may need a patent licence even though the
copyright position is clean. raw-files detects these files and returns a clear
unsupported error; see [security.md](security.md). Any future work on them
needs legal advice first.

## 5. Community references

These are the usual places the format is discussed. Read them for facts and
cite them; do not copy their tables or prose.

| Reference | What it is good for | Note |
|---|---|---|
| ExifTool Nikon tag pages [22] | Tag names, types, value meanings, sub-block layouts, compression value names | Licensed as ExifTool; tag text is expression |
| exiv2 MakerNote page [23] | The Nikon MakerNote layouts and offset bases | Licensed as exiv2 |
| Laurent Clévy, "Nikon NEF" notes (version 0.31, 2009) [24] | The container, the 0x0096 linearisation block, early bodies; the usual community reference | Old; reachable over plain HTTP only |
| Leon Plickat, "Decoding Nikon NEF Files" [25] | A readable walkthrough of the lossless scheme and the curve | Does not map version bytes to bodies |
| Bill Claff, Photons to Photos, NEF compression [26] | Per-body compression modes and curve sizes | Does not give version bytes |
| RawDigger, "Nikon small raw internals" [27] | How the YCbCr small-raw (sNEF) files are laid out | Describes D4S and D810 |

## 6. What is still unknown

- Current availability and terms of Nikon's RAW SDK.
- Whether an open specification of any part of the MakerNote exists
  outside community reverse engineering.
- The provenance policy for the Huffman symbol lists and the decryption
  tables (section 4.2).
- The legal position of HE and HE* beyond what is reported in forums and
  press releases.

## Sources

[1] Bio-Formats, "Nikon NEF" format page —
    https://bio-formats.readthedocs.io/en/latest/formats/nef.html **[S]**
[2] Imaging Resource, news on Nikon's NEF statement (2005) —
    https://www.imaging-resource.com/NEWS/1114203185.html **[S]**
[3] digitalkamera.de, Nikon's statement on NEF encryption (2005) —
    https://www.digitalkamera.de/Meldung/Nikon_nimmt_Stellung_zur_Teil-Verschlusselung_der_RAW_NEF-Daten/2675.aspx **[S]**
[4] DPReview, news on the NEF statement —
    https://www.dpreview.com/?p=318530 **[S]**
[5] Adobe, TIFF Revision 6.0 (1992), ITU mirror —
    https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf **[S]**
[6] ISO, ISO 12234-2:2001 listing — https://www.iso.org/standard/29377.html
    **[S]**
[7] CIPA, standards page (DC-008-Translation-2026, Exif 3.1) —
    https://www.cipa.jp/e/std/std-sec.html **[P]**
[8] IPTC, "Exif 3.0 released" —
    https://iptc.org/news/exif-3-0-released-featuring-utf-8-support/ **[S]**
[9] ITU-T Recommendation T.81 — https://www.itu.int/rec/T-REC-T.81 **[I]**
[10] Adobe, DNG SDK download page (specification 1.7.1.0, patent notice) —
    https://www.adobe.com/support/downloads/dng/dng_sdk.html **[P]**
[11] dcraw 9.28 source header — https://dechifro.org/dcraw/dcraw.c **[P]**
[12] Debian, dcraw 9.28 COPYING —
    https://sources.debian.org/src/dcraw/9.28-8/COPYING **[P]**
[13] LibRaw README —
    https://raw.githubusercontent.com/LibRaw/LibRaw/master/README.md **[P]**
[14] LibRaw Changelog and GitHub releases —
    https://raw.githubusercontent.com/LibRaw/LibRaw/0.22.2/Changelog.txt ,
    https://api.github.com/repos/LibRaw/LibRaw/releases **[P]**
[15] rawspeed repository — https://github.com/darktable-org/rawspeed **[P]**
[16] dnglab / rawler repository — https://github.com/dnglab/dnglab **[P]**
[17] crates.io API, per-crate pages (rawler, rawloader, imagepipe,
    quickraw, rexiv2, revelo-exiftool-tables and the crates in 3.2) —
    https://crates.io/api/v1/crates/<name> **[P]** (licences)
[18] ExifTool home page and repository metadata —
    https://exiftool.org/index.html ,
    https://api.github.com/repos/exiftool/exiftool **[P]**
[19] Exiv2 COPYING —
    https://raw.githubusercontent.com/Exiv2/exiv2/main/COPYING **[P]**
[20] GitHub API, libexif and libopenraw —
    https://api.github.com/repos/libexif/libexif ,
    https://api.github.com/repos/hfiguiere/libopenraw **[P]**
[21] crates.io and GitHub API for `tiff`, `kamadak-exif`, `zune-jpeg`,
    `nom-exif`, `little_exif`, `rawkit` —
    https://crates.io/api/v1/crates/tiff ,
    https://api.github.com/repos/mindeng/nom-exif/license **[P]**
[22] ExifTool, Nikon tags — https://exiftool.org/TagNames/Nikon.html **[P]**
[23] Exiv2, MakerNote notes — https://exiv2.org/makernote.html **[P]**
[24] Laurent Clévy, NEF notes — http://lclevy.free.fr/nef/ **[P]**
[25] Leon Plickat, "Decoding Nikon NEF Files" —
    https://leon_plickat.srht.site/nikon-nef.html **[S]**
[26] Bill Claff, NEF compression —
    https://www.photonstophotos.net/NikonInfo/NEF_Compression.htm **[S]**
[27] RawDigger, Nikon small raw internals —
    https://www.rawdigger.com/howtouse/nikon-small-raw-internals **[S]**
[28] HE/HE*: pixls.us discussion
    https://discuss.pixls.us/t/nikon-he-decoder/56955 ; dnglab PR #835
    https://github.com/dnglab/dnglab/pull/835 ; LibRaw PR #826
    https://github.com/LibRaw/LibRaw/pull/826 ; intoPIX TicoRAW and Z 9
    announcement
    https://www.intopix.com/blogs/post/TicoRAW-technology-added-with-High-Efficiency-RAW-recording-of-Nikon-Z-9-flagship-mirrorless-camera
    **[P]** (PR state), **[S]** (rest)
[29] intoPIX, Fraunhofer IIS and Vectis, JPEG XS patent pool —
    https://www.intopix.com/blogs/post/intoPIX-and-Fraunhofer-IIS-team-with-Vectis-to-offer-JPEG-XS-patent-pool-license
    **[S]**
