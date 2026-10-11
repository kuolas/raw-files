# NEF: research record

How the [NEF documentation](README.md) was put together, so a reader can judge
how far to trust it and a contributor can extend it the same way. The research
was done on 10 October 2026, before any NEF code existed in raw-files.

## Disclosure

Following [ADR006](../../architecture-decisions/adr006-ai-assisted-contributions.md),
this research and the first drafts of these pages were produced with AI
assistance (Claude). A human maintainer is responsible for reviewing them
before they are relied on. The disclosure covers the whole folder.

## Question

The question was to gather everything publicly known about the Nikon NEF
format that raw-files needs for three roadmap milestones:

1. find and return the largest embedded JPEG preview;
2. read basic metadata (orientation, capture date, camera, dimensions);
3. decode the raw sensor data.

The research also had to establish two practical points:

- which existing references raw-files may learn from or reuse under its
  MIT OR Apache-2.0 licence;
- what test data exists, and what has gone wrong in other NEF parsers.

## Method

**Ground rules.**

- Every non-trivial claim carries a source URL and a confidence tag (see the
  [overview](README.md#confidence-tags)).
- Where sources disagree, the disagreement is recorded rather than resolved
  by guesswork.
- Algorithms are restated in prose or original pseudocode.
- Copy no code and no wholesale data tables (such as Huffman symbol lists
  or decryption tables); individual facts such as tag IDs, value meanings
  and per-body values are cited to their source.
- Quotations stay under fifteen words.
- No sample files were downloaded.

**Round 1: four topics, researched in parallel.**

1. The TIFF container and embedded previews.
2. The Nikon MakerNote, its encrypted blocks and viewer metadata.
3. Raw data encoding: Huffman compression, curves, packed layouts, small raws
   and HE/HE\*.
4. The ecosystem: official documentation, implementations and their licences,
   test data, and security history.

**Synthesis and cross-check.** The four sets of findings were merged, checked
against each other, and corrected where they conflicted (see
[Corrections](#corrections-made-along-the-way)). The open questions were
collected at this stage.

**Round 2: targeted gaps.** Two follow-up passes went after the open
questions that public sources could still answer:

- The first pass covered:
  - Laurent Clévy's NEF notes;
  - byte order by generation and preview sizes by body;
  - the WorldTime and CropHiSpeed layouts and OffsetTime support;
  - Make/Model padding, the white-balance source on Z bodies, and Coolscan
    files.
- The second pass covered:
  - dcraw details and the 0x0096 version bytes by generation;
  - NEFCompression 5 and 6, D850 and Z-series reduced-size raws, and the
    uncompressed layouts on Z bodies;
  - black-level channel order;
  - the 2026 LibRaw CVEs, the Exif 3.1 publication, and the HE detection and
    pull-request status.

**Writing.** The findings were rewritten as the topic pages in this folder.
They are addressed to contributors, with sources listed at the end of each
page.

**Review.** The drafted pages were reviewed against the collected evidence
for:

- technical errors;
- contradictions between pages;
- confidence tags stronger than the evidence;
- missing citations;
- conflicts with the raw-files ADRs.

Every finding was checked before it was fixed. The substantive corrections
are listed under
[Corrections made along the way](#corrections-made-along-the-way).

## Sources consulted

The pages cite their sources individually. By kind:

- **Specifications and standards:**
  - CIPA's Exif page;
  - Adobe's DNG specification page;
  - the ISO catalogue entry for TIFF/EP;
  - TIFF 6.0 mirrors;
  - Nikon's online camera manuals.
- **Tag references:**
  - ExifTool's Nikon and EXIF tag pages and its `Nikon.pm`, `Exif.pm` and
    `MakerNotes.pm` sources;
  - exiv2's MakerNote page and sources.
- **Implementations, read for understanding only:**
  - dcraw 9.28;
  - LibRaw (`tiff.cpp`, `nikon.cpp`, `identify.cpp`, `open.cpp` and its
    decoders);
  - rawspeed (`NefDecoder`, `NikonDecompressor`, `cameras.xml`);
  - rawler (dnglab);
  - libopenraw's format notes.
- **Community write-ups:**
  - Laurent Clévy's NEF notes (v0.31, 2009);
  - Leon Plickat's "Decoding Nikon NEF Files";
  - Bill Claff's NEF compression tables;
  - RawDigger's analysis of small raws;
  - discussions on pixls.us, the LibRaw forum and project issue trackers.
- **Registries:**
  - NVD records for every cited CVE;
  - crates.io and GitHub metadata for licences;
  - raw.pixls.us directory listings.

## Access problems

- **Read only by plain HTTP.** Laurent Clévy's site could be read only over
  plain HTTP, and was then read directly; the facts taken from it are
  tagged [P].
- **Down.** The ExifTool forum was down throughout. Claims from it come from
  search extracts and are tagged [S].
- **Refused.** DPReview (HTTP 403) and Photography Life (HTTP 402) refused
  access. The Internet Archive's Wayback Machine and archive.ph could not be
  used.
- **Not reachable.** Nikon's SDK portal and the oss-fuzz issue tracker could
  not be reached.
- **Not read line by line.** Some source files were read through automated
  page extraction. A fact marked [P] was stated in its source, but byte
  offsets should still be checked against that source before they are
  hard-coded.

## Cross-checks made

- **Huffman tables.** The counts per code length of the six NEF Huffman
  tables were compared with dcraw 9.28 and match.
- **MakerNote cipher.** ExifTool, exiv2 and LibRaw describe the same key
  derivation and keystream; dcraw 9.28 agrees.
- **0x0096 header.** The handling was confirmed in dcraw: the 2110-byte skip
  triggers on `0x49` *or* `0x58`, and the split row is read at offset 562.
  rawspeed and rawler were confirmed to add the `0x44 0x40` variant.
- **Primary-record checks.** These were confirmed against NVD records,
  CIPA's page and LibRaw's source:
  - CVE-2026-5342 and the other 2026 LibRaw CVEs;
  - the Exif 3.1 publication date;
  - LibRaw's HE detection and its model list.

## Corrections made along the way

- **Huffman tables.** One early finding said lossless NEF files carry their
  Huffman tables. They do not: the tables are fixed in every decoder, and only
  the curve and predictor seeds are in the file.
- **MakerNote layout names.** "Type 1/2/3" is numbered differently by
  different projects. The pages use descriptive names (headerless, short
  header, embedded TIFF header) instead.
- **Non-numeric serials.** The early findings described a single rule. In
  fact dcraw, LibRaw and ExifTool/exiv2 behave differently (see
  [open-questions.md](open-questions.md)).
- **LibRaw release dates.** 0.22.0 is dated 2025-12-25 in the master-branch
  changelog but 2026-01-13 on GitHub's release page. 0.22.1 was released on
  2026-04-06 and 0.22.2 on 2026-07-16.
- **Talos CVEs.** For the Talos-reported 2026 LibRaw CVEs, NVD and LibRaw's
  0.22.1 changelog disagree on which release fixed them. security.md treats
  0.22.2 as the conservative reference rather than claiming which release
  fixed them; NVD lists 0.22.0 as affected for CVE-2026-20889 and 0.22.1 for
  the other three.

Corrections from the review of the drafted pages:

- **`0x44 0x40` curve header.** This header is the lossy "2nd version"
  curve, not a lossless mode, and decoders narrow its domain by two bits.
- **D1X pixel aspect.** Each stored row covers two output rows. To get square
  pixels, double the height; the draft said to halve the rows.
- **Curve sizes.** Bill Claff's figures are encoding-table sizes, not the
  stored curve-point count `csize`. Clévy's directly read `csize` values (32
  or 34, 257, 567, 683) are now given separately.
- **Image dimensions.** These come from the raw SubIFD, not IFD0, whose size
  is the thumbnail's.
- **Preview orientation.** The preview JPEGs carry no orientation of their
  own; IFD0's Orientation tag wins.
- **NikonNEFInfo.** ExifTool decodes tag 0xC7D5 (NikonNEFInfo) into
  DistortionInfo and VignetteInfo.
- **Licence wording.** The wording was aligned with what the pages actually
  contain: no code or wholesale tables, but individual facts cited to their
  source. Per-body values quoted from rawspeed are marked as being for
  cross-checking only.

## What this research did not do

- **No sample files.** None were downloaded or inspected. Every per-body
  number is from documentation, and the first implementation step should
  check it on files (see [test-data.md](test-data.md)).
- **No code.** None was written, so no algorithm here has been exercised.
- **No legal review.** Notes on copyright, data tables and the HE patent pool
  identify questions; they do not answer them.

## Extending this documentation

Follow the same rules:

- cite every claim;
- tag its confidence;
- record disagreements;
- never copy copyleft code or tables.

When a sample file settles an open question:

- update the relevant page and change its tag to reflect the evidence;
- remove the item from [open-questions.md](open-questions.md);
- name the file that settled it.
