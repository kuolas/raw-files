# ARW: research record

How the [ARW documentation](README.md) was put together, so a reader can judge
how far to trust it and a contributor can extend it the same way. The research
and the sample checks were done in October 2026, before any ARW code existed
in raw-files.

## Disclosure

Following [ADR006](../../architecture-decisions/adr006-ai-assisted-contributions.md),
this research, the sample checks and the first drafts of these pages were
produced with AI assistance (Claude). A human maintainer is responsible for
reviewing them before they are relied on. The disclosure covers the whole
folder.

## Question

The question was to gather everything publicly known about Sony's ARW format
and its relatives (SRF, SR2, ARQ) that raw-files needs for three of the
roadmap milestones:

- milestone 2: find and return the largest embedded JPEG preview;
- milestone 3: read basic metadata (orientation, capture date, camera,
  dimensions);
- milestone 5: decode the raw sensor data.

The research also had to establish three practical points:

- which existing references raw-files may learn from or reuse under its
  MIT OR Apache-2.0 licence, and which legal questions are open;
- what test data exists, and which of it may be committed or downloaded;
- what has gone wrong in other Sony parsers.

## Method

**Ground rules.**

- Every non-trivial claim carries a source URL and a confidence tag (see the
  [overview](README.md#confidence-tags)).
- Where sources disagree, the disagreement is recorded rather than resolved
  by guesswork; where a sample file settles it, the page names the file.
- Algorithms are restated in prose or original pseudocode.
- Copy no code and no wholesale data tables (such as the ARW1 code table,
  the MakerNote cipher's inverse table, or any tone or delinearisation
  curve); individual facts such as tag IDs, offsets, value meanings and
  per-body values are cited to their source.
- Quotations stay under fifteen words.
- No code from a copyleft project was adapted, for the pages or for the
  checks.
- Sample files come from raw.pixls.us only, and only files that its
  catalogue lists as CC0. Each file's SHA-256 was checked against the
  catalogue before any parsing.
- Structure and algorithm checks used throwaway scripts written from the
  pages' own prose. They are not committed. No external decoder or ExifTool
  was run on the files, so no measurement depends on another
  implementation.

**How the pages were checked.** The pages were first written from sources
compared against each other, with disagreements recorded in the page. The
claims that a file can show were then checked against 36 CC0 sample files
covering every member of the family from the SRF to ARW 6.0
([test-data.md](test-data.md#files-checked)), and each measured fact is
tagged [F] with its file. The sample checks overrode several desk findings,
for example:

- where the largest preview lives on ARW 4.0 and later;
- whether the A100's ARW MakerNote has a header;
- the element order of tag 0x74C7;
- the ARW version at which plain copies of the levels and the time-zone
  tags appear;
- which bodies write ARW1 streams, and the A100's raw width;
- how 12-bit data and their levels are scaled;
- what SonyRawFileType means.

Each such point is recorded as a disagreement on its page, and
[open-questions.md](open-questions.md) lists the questions the samples
settled.

## Sources consulted

The pages cite their sources individually. By kind:

- **Specifications and standards:**
  - TIFF 6.0 (an ITU mirror) and the ISO catalogue entry for TIFF/EP;
  - CIPA's Exif page;
  - Adobe's DNG specification page;
  - ITU-T T.81;
  - SMPTE RDD 34:2015 (LLVC);
  - the PRONOM, Library of Congress, freedesktop and IANA registries.
- **Sony:**
  - camera help guides and a support article on 12-bit modes;
  - Imaging Edge and Pixel Shift Multi Shooting pages;
  - the Camera Remote SDK page and its licence.
- **Tag references:**
  - ExifTool's Sony and EXIF tag pages and its `Sony.pm`, `Exif.pm`,
    `MakerNotes.pm`, `SonyIDC.pm`, `Minolta.pm`, `MinoltaRaw.pm`,
    `ExifTool.pm` and `WriteExif.pl` sources;
  - exiv2's MakerNote page and sources.
- **Implementations, read for understanding only:**
  - dcraw 9.28;
  - LibRaw (`tiff.cpp`, `sony.cpp`, `makernotes.cpp`, `identify.cpp`,
    `open.cpp` and its Sony decoders);
  - rawspeed (`ArwDecoder`, the Sony decompressors, `LJpegDecoder`,
    `cameras.xml`);
  - rawler (dnglab) and rawloader;
  - libopenraw and metadata-extractor;
  - Graphite's rawkit, compared with dcraw.
- **Community write-ups:**
  - Laurent Clévy's Sony notes (v0.55, 2026) and the ExifTool dumps
    published with them;
  - RawDigger's description of cRAW, and Henry Dietz's paper on its
    artefacts;
  - the README of arw6_decode;
  - discussions on pixls.us, the Adobe and Lightroom Queen forums, the
    ExifTool forum (through search extracts), and project issue trackers.
- **Registries and records:**
  - NVD records, Exiv2's GitHub advisories, OSS-Fuzz vulnerability records
    and the RustSec advisory database;
  - the LibRaw and rawspeed commit histories;
  - crates.io and GitHub metadata for licences;
  - Google Patents records for the cRAW patents.
- **Sample data:**
  - the raw.pixls.us catalogue, its hash list and the exiv2 dumps it
    publishes per file, used to choose files rather than as evidence;
  - the 36 CC0 files listed in [test-data.md](test-data.md#files-checked).

## Access problems

- **Down.** The ExifTool forum was unavailable. Claims from it come from
  search extracts and are tagged [S].
- **Search extracts only.** Several pages were read only through search
  extracts or summaries: Sony's support article on 12-bit modes and its
  Pixel Shift and RAW Driver pages, Google Patents' records, and some forum
  and blog posts. Claims that rest on them are tagged [S].
- **Mirror.** dcraw's original site did not respond, so every page
  read dcraw 9.28 from a mirror that was not verified against the original
  ([security.md](security.md)).
- **Not readable.** dnglab's RawDB is a JavaScript application whose
  per-file licences could not be read.
- **Not opened.** ZDI's advisory pages, the OSS-Fuzz issue tracker and the
  LibRaw 0.18.2 release tarball.
- **Not read, deliberately.** arw6_decode has no licence, so only its
  README was read. LibRaw's ARW 6.0 decoder was read for its header,
  identifiers and the curve's shape only.
- **Read in part.** SMPTE RDD 34 (pages 1 to 8). Paid standards (ISO
  12234-2, ISO/IEC 21122) were not read.
- **Incomplete dumps.** The exiv2 dumps on raw.pixls.us omit undecoded
  Sony tags and under-report OffsetTime\*; claims about those tags rest on
  the files' bytes instead.

## Cross-checks made

On the sample files, with the throwaway scripts described above:

- **SR2 keystream.** It decrypts the SR2SubIFD of all 34 files that have
  one (every file except the A100 and the SRF) into a valid IFD of 101 to
  232 entries. The key is `11 22 33 44` on every ARW and different on the
  R1. The ring-buffer and growing-sequence forms of the generator were
  shown equal by computation over random keys.
- **MakerNote cipher (b³ mod 249).** Deciphered shutter counts increase in
  capture order on four bodies (ILCE-7M4, ILCE-9, ILCE-7RM2, ILCE-7M5). The
  inverse exponent 55 was checked by computation for all 256 byte values.
- **cRAW block layout.** Over 500,000 blocks in 15 files: no block has its
  maximum below its minimum or its two indices equal, same-colour jumps across
  block boundaries are small, and every cRAW width is a multiple of 32. The
  tone-curve builder gives 17,204 at the top code with the common knots.
- **ARW1.** File 2346 decodes as 3881 columns by 2608 rows, consuming all
  but 5 of 10,039,268 bytes with no range violation; file 2271 decodes with
  eight extra rows, leaving 5 bytes.
- **Other layouts.** The nibble order of 12-bit packed data (file 5858),
  the big-endian words of the R1 (file 3221), the 14-bit scale of 12-bit
  uncompressed data (file 1311) and the ARQ's channel order (file 3255).
- **Lossless tile headers.** The first and last tiles of four lossless
  Bayer files are 256×256 frames with four components, precision 14 and
  predictor 1; tile counts match the tile grid; the M and S tiles are
  512×512 frames of precision 16.
- **SRF key chain.** In file 1351 the master key decrypts SRF1 to a
  two-entry IFD, and its data key decrypts the image to 14-bit values. The
  later SRF IFDs could not be located.
- **Previews.** A finder of the recommended shape returns the expected JPEG
  from all 36 files, and its role filter was checked against every IFD in
  them. A whole-file scan found no JPEG outside the referenced ones.

From the sources:

- **rawkit and dcraw.** rawkit's ARW1 table, cRAW loop and tone-curve
  builder were compared with dcraw 9.28 and match element for element.
- **Security records.** The fix commits for CVE-2017-6887, CVE-2020-35535
  and CVE-2026-88387 were checked against LibRaw's tags and history; the
  disagreements with NVD are recorded in [security.md](security.md).
- **PRONOM signatures.** They were compared with the FileFormat bytes and
  offsets of the files, and do not match ARW 4.0 and later or the Lunar.

## What this research did not do

- **No pixel comparison.** No decoded output was compared with a reference
  decoder. "Checked" means that the data decode consistently with the
  description (all bytes consumed, values in range, neighbouring
  same-colour pixels close), not that they are bit-exact.
- **No code in raw-files.** The check scripts were throwaway, so nothing
  here has been exercised by raw-files' own code or tests.
- **No fuzzing.** The robustness advice comes from other projects' history.
- **No legal review.** Notes on copyright, data tables, decryption and the
  cRAW and ARW 6.0 patents identify questions; they do not answer them.
- **Not every body.** Bodies absent from raw.pixls.us (ILCE-7RM6, DSC-V3,
  the Hasselblad Stellar, HV and Lusso), non-CC0 files, and edited or
  re-saved files were not examined; many other bodies are known only from
  the exiv2 dumps.

## Extending this documentation

Follow the same rules:

- cite every claim;
- tag its confidence;
- record disagreements;
- never copy copyleft code or tables.

When a sample file settles an open question or adds an [F] claim:

- use a file that the raw.pixls.us catalogue lists as CC0, and check its
  SHA-256 before parsing;
- name the file by its id, give its URL and SHA-256 in the page's Sources,
  and add it to [test-data.md](test-data.md#files-checked) if it is new;
- say what was measured, and how, in enough detail to repeat it;
- update the relevant page and change its tag to reflect the evidence;
- move the item from Part 2 of [open-questions.md](open-questions.md) to
  its Part 4, naming the file that settled it.
