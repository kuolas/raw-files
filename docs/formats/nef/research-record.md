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
format that raw-files needs for three of the roadmap milestones:

- milestone 1: find and return the largest embedded JPEG preview;
- milestone 3: read basic metadata (orientation, capture date, camera,
  dimensions);
- milestone 5: decode the raw sensor data.

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

**How the pages were checked.** The pages were written from sources compared
against each other; where they disagree, the disagreement is recorded in the
page rather than settled by guesswork. The pages were reviewed against those
sources before publication, and a later review against primary sources
corrected several points; the corrections are in the git history of this
folder.

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
  rawspeed and rawler were confirmed to add the `0x44 0x40` variant. The
  order of the four predictor seeds is not a cross-check: dcraw and LibRaw
  disagree with rawspeed and rawler (see
  [raw-data.md](raw-data.md#disagreements-1)).
- **Primary-record checks.** These were confirmed against NVD records,
  CIPA's page and LibRaw's source:
  - CVE-2026-5342 and the other 2026 LibRaw CVEs;
  - the Exif 3.1 publication date;
  - LibRaw's HE detection and its model list.

## What this research did not do

- **No sample files.** None were downloaded or inspected. Every per-body
  number is from documentation, and the first implementation step should
  check it on files (see [test-data.md](test-data.md)).
- **No code.** None was written, so no algorithm here has been exercised.
- **No legal review.** Notes on copyright, data tables and the HE patent pool
  identify questions; they do not answer them.

## Later check against sample files

On 13 October 2026 the container and preview notes were checked against
the three CC0 Nikon Z 9 files that CI downloads (files 5146 to 5148, listed
in [test-data.md](test-data.md#21-the-ci-set)), by a structural walk of
their IFDs and embedded JPEGs. What they show is tagged **[F]** in
[container-and-previews.md](container-and-previews.md) and
[raw-data.md](raw-data.md), and the questions they answer are marked in
[open-questions.md](open-questions.md). Metadata and raw decoding were not
checked.

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
