# Sony ARW

What raw-files needs to know about Sony's ARW raw format and its relatives
SRF, SR2 and ARQ, collected from public sources and checked where possible
against public sample files. Sony publishes no specification, so everything
here comes from the TIFF and Exif standards, community documentation, the
source code of existing raw decoders and the bytes of sample files, each
cited where it is used.

These pages support the [roadmap](../../ROADMAP.md): ARW previews
(milestone 2), then basic metadata (milestone 3), then focus points
(milestone 4), then raw decoding (milestone 6).

## Status

This is desk research plus structural checks on 36 CC0 sample files from
raw.pixls.us, done in October 2026. The checks walked every TIFF and
MakerNote structure, parsed every embedded JPEG up to its start-of-frame
marker, and ran the algorithms that the pages describe as *checked* (the SR2
keystream, the MakerNote cipher, the cRAW block layout, ARW1 decoding, the
lossless tile headers and the SRF key chain). They used throwaway scripts
written from the pages' prose, which are not in the repository; none of this
is implemented in raw-files yet. On 19 October 2026 the autofocus data of
the same 36 files was checked too; raw-files reads the focus point from
it, and `tests/arw.rs` checks every file's. On 21 October 2026 the
MakerNote of five Sony camera JPEGs from Wikimedia Commons was checked
as well ([makernote-and-metadata.md](makernote-and-metadata.md#in-a-jpeg-file),
listed in the [JPEG test data](../jpeg/test-data.md#2-the-sample-files)).
**No pixel output was compared with a reference decoder**, so "checked"
means that the data decode consistently with the description, not that
they are bit-exact.

Byte offsets and formulas taken from other projects' source code should
still be checked against that source and against real files before they are
hard-coded. [open-questions.md](open-questions.md) lists what is still
unknown. [research-record.md](research-record.md) describes how the research
was done and what it could not reach. The files checked are listed in
[test-data.md](test-data.md#files-checked).

## Pages

| Page | Read it for |
|---|---|
| [container-and-previews.md](container-and-previews.md) | The TIFF structure of each member of the family, where the embedded JPEGs are, and how to pick the largest (milestone 2) |
| [makernote-and-metadata.md](makernote-and-metadata.md) | The Sony MakerNote, its two ciphers, SR2Private, and orientation, date, camera, size and lens (milestone 3) and the focus point (milestone 4) |
| [raw-data.md](raw-data.md) | How the sensor data is stored: uncompressed, cRAW, 12-bit packed, ARW1, lossless tiles, M/S YCbCr, SR2, SRF, ARQ and the refused ARW 6.0 (milestone 6) |
| [security.md](security.md) | Past vulnerabilities in Sony parsers and the Sony additions to the NEF checklist |
| [licences-and-references.md](licences-and-references.md) | Standards, existing implementations and their licences, what raw-files may reuse, and the legal questions |
| [test-data.md](test-data.md) | Where sample files come from, which may be committed, a minimal body set, and the files checked |
| [open-questions.md](open-questions.md) | Decisions to take, facts still to establish, and what the samples settled |
| [research-record.md](research-record.md) | How this documentation was researched and checked, and its limits |

## Confidence tags

Claims carry a tag:

- **[P]** — stated in a primary source: a specification, a tag reference such
  as ExifTool's, an implementation's source code, or a published tool dump of
  a file.
- **[S]** — secondary: a forum post, issue, blog or search extract.
- **[I]** — inferred, or from a single unconfirmed report. Treat it as a
  hypothesis to test on sample files.
- **[F]** — measured on a named sample file. Files are cited by their
  raw.pixls.us id ("file 4466"); each page's Sources give the URL and
  SHA-256 of the files it cites, and
  [test-data.md](test-data.md#files-checked) lists all of them.

Where a sample contradicts a source, the pages follow the sample and record
the disagreement.

## ARW at a glance

- **An ARW is a little-endian TIFF file.** Every checked file, from the 2003
  SRF to the 2025 ILCE-7M5, starts `II`, 42, with IFD0 at offset 8 [F]; no
  big-endian Sony raw file is known [I], but a reader should accept both
  orders. IFD0 has one SubIFD (0x014A), the raw image; on the DSLR-A100 that
  tag holds the raw data offset instead [F]. The MakerNote has no byte-order
  mark of its own, and its offsets count from the start of the file [F]
  ([container-and-previews.md](container-and-previews.md)).
- **One family, many versions.** SRF (DSC-F828, DSC-V3), SR2 (DSC-R1), ARW
  1.0 (DSLR-A100), ARW 2.0 to 2.3.5, and ARW 4.0 to 6.0; there is no ARW 3.
  ARQ is a pixel-shift composite made on a computer, and the Hasselblad
  Lunar is a Sony ARW under another Make. MakerNote tag 0xB000 FileFormat
  names the version, but it follows the firmware, not the body: three
  checked bodies write a different version from the published tables [F]. Do
  not key code on it.
- **Previews.** Before FileFormat 4.0 the largest JPEG is IFD0's, about
  1616×1080 from the A200 on (640×480 on the A100, 640×424 on the R1; the
  SRF's only JPEG, 640×480, is in IFD1), with a 160×120 thumbnail in IFD1.
  From 4.0 on, a full-size JPEG sits in **IFD2**, the third IFD of the IFD0
  chain, under Compression 7 and Photometric 6 with 8-bit samples: it is in
  all 16 checked files at 4.0 or later and in none of the 20 earlier ones
  [F].
  - **The lossless trap.** Lossless raw tiles begin `FF D8 FF C3`, an SOI
    and a lossless SOF3 header, and the M/S raws share IFD2's Photometric 6.
    A finder that accepts any IFD with an SOI and a SOF offers the raw as a
    preview [F].
  - The recommended finder walks the whole IFD0 chain, skips IFDs that are
    raw by role, rejects lossless frame markers (C3, C7, CB, CF) and ranks
    by SOF area; a procedure of that shape was checked on all 36 files [F].
  - Embedded JPEGs carry no APP segment and are stored unrotated; IFD0's
    Orientation applies [F].
- **Viewer metadata is plain.** Orientation, capture date, make, model and
  the crop tags come from IFD0, the Exif IFD and the raw SubIFD, plus the
  plain MakerNote tags 0xB000 and 0xB001 (and 0xB027 and 0xB02A for the
  lens). Milestones 2 and 3 need **no decryption and no deciphering** on any
  checked file [F]
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **The focus point is plain too.** FocusLocation (0x2027) gives the
  frame and the centre of the focus area on every checked body from the
  ILCE-7RM2 (2015) on, and FocusFrameSize (0x2037) the area's size from
  the ILCE-7SM3 (2020) on; the frame has the framing of the previews,
  which on an aspect-ratio file is not the default crop [F]. Older bodies
  record at most a point number, and no plain tag says whether focus
  was confirmed
  ([makernote-and-metadata.md](makernote-and-metadata.md#96-focus-point)).
- **Raw encodings** ([raw-data.md](raw-data.md)):
  - Compression 1, uncompressed 16-bit words (big-endian on the R1; four
    samples per pixel in ARQ);
  - Compression 32767, three layouts told apart by byte count: cRAW (one
    byte per pixel, 16-pixel blocks and a tone curve), 12-bit packed (A700,
    A850, A900) and an ARW1 stream (the A200) [P][F];
  - ARW1 on the A100, which has no raw IFD: the stream starts at the offset
    in IFD0's 0x014A [F];
  - Compression 7, lossless JPEG in 512×512 tiles: Bayer for the L size,
    already demosaiced YCbCr for M and S [F];
  - Compression 32766, **ARW 6.0** (ILCE-7M5, ILCE-7RM6): raw-files will
    detect and refuse it, because its 4096-entry curve is not in the file
    and its patent position is open. Previews from ARW 6.0 files are fine.

  BitsPerSample and SonyRawFileType do not reliably describe the data [F];
  dispatch on Compression, Photometric, samples per pixel and exact byte
  counts.
- **Levels need the SR2 keystream on older files.** Black level, white
  balance and Sony's white level are in the SR2SubIFD, reached through IFD0
  tag 0xC634 and encrypted with a keystream whose key is stored in the file
  (`11 22 33 44` on every checked ARW) [F]. Plain copies of black level and
  white balance exist from ARW 2.3.3 on; white level 0x787F exists only in
  the encrypted block [F]. The keystream is a short formula with no table.
  The MakerNote's separate byte-substitution cipher (b³ mod 249) hides only
  extras such as the shutter count, which no milestone needs.
- **No mature ARW implementation can be copied into raw-files.** dcraw,
  LibRaw, rawspeed, rawler, libopenraw, ExifTool and exiv2 are read for
  understanding only. Graphite's rawkit is labelled MIT OR Apache-2.0, but
  its ARW1 table, cRAW loop and tone-curve builder match dcraw element for
  element [P] (that they were translated is [I]), and it panics on
  unexpected input ([licences-and-references.md](licences-and-references.md)).
- **Few tables need a decision.** Unlike NEF, the ARW1 prefix code, the
  cipher inverse and the keystream reduce to rules or formulas, and per-body
  crops and levels can be derived from samples. The ARW 6.0 curve, published
  only as extracted tables, is the hard case
  ([open-questions.md](open-questions.md)).

## Where to start, by milestone

- **Milestone 2, ARW previews:**
  - [container-and-previews.md](container-and-previews.md), especially its
    recommended algorithm and the expected results on the checked files.
  - [security.md](security.md), the milestone 2 items and the NEF checklist
    items they build on (offsets, IFD loops, allocation).
  - [test-data.md](test-data.md), for the smaller set of files that covers
    preview tests.
- **Milestone 3, basic metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the viewer
    metadata section and its recommendation.
- **Milestone 4, focus points:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the focus
    point section, with what the samples cover and what they do not.
- **Milestone 6, raw decoding:**
  - [raw-data.md](raw-data.md).
  - [makernote-and-metadata.md](makernote-and-metadata.md), for SR2Private,
    the keystream, white balance and the levels.
  - [open-questions.md](open-questions.md), for the decisions on data
    provenance and ARW 6.0 that must be taken first.

## Relation to the NEF pages

These pages follow the structure of the [NEF pages](../nef/README.md) and do
not repeat what the two formats share:

- **TIFF walking.** Both are TIFF files with SubIFDs and a MakerNote in the
  Exif IFD; the same bounds checks, visited-offset set and caps apply.
- **The preview algorithm.** The NEF recommendation already walks the
  whole IFD0 next-IFD chain and rejects lossless SOF markers, so the ARW
  one adds a single new rule: skip IFDs that are raw by role. It also
  states explicitly that the largest JPEG on ARW 4.0 and later is in IFD2,
  which only a whole-chain walk reaches. Two smaller differences remain
  (see [open-questions.md](open-questions.md)).
  Orientation is handled the same way in both: return the JPEG unchanged
  and report IFD0's tag separately.
- **Security.** The NEF checklist items 1 to 14 apply to ARW unchanged;
  [security.md](security.md) adds Sony-specific items numbered A1 onwards.
- **Licences and test data.** The NEF rules on reading copyleft sources,
  the options for data tables, and downloading samples in CI pinned by
  SHA-256 apply here too.

Two things differ: ARW needs no built-in Huffman tables (lossless tiles
carry their own) and no MakerNote cipher for anything a milestone needs, and
the ARW pages were checked against sample files from the start, while the
NEF pages were desk research first and have so far been checked on the Z 9
only.

## Licence hygiene

raw-files is MIT OR Apache-2.0 ([ADR002](../../architecture-decisions/adr002-license-mit-or-apache-2.md)).
These pages restate algorithms in prose or in original pseudocode. They copy
no code and no wholesale data tables: the ARW1 prefix code is given as a
rule, the MakerNote cipher's inverse and the SR2 keystream as formulas, and
no tone curve, delinearisation curve or per-body table is reproduced.
Quotations stay under fifteen words; individual facts such as tag IDs,
offsets, value meanings and per-body values are cited to their source. Keep
it that way when extending them.
