# Sony ARW

What raw-files needs to know about Sony's ARW raw format and its relatives
SRF, SR2 and ARQ, collected from public sources and checked where possible
against public sample files. Sony publishes no specification, so everything
here comes from the TIFF and Exif standards, community documentation, the
source code of existing raw decoders and the bytes of sample files, each
cited where it is used.

Where these notes say *I* or *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files, writing about my own cameras and shots: they stay
private, and are cited by counts or by frame numbers.

These pages support four of the capabilities of the
[roadmap](../../ROADMAP.md) for ARW files: previews, metadata, focus
points and raw sensor data. Full metadata, every metadata value a file
records, is the open part of metadata.

## Status

These pages are desk research, checked against 36 CC0 sample files from
raw.pixls.us from the start, as the [research record](research-record.md)
says. The table shows what has been checked against sample files, and
where. What a file shows is tagged **[F]**.

| Topic | Checked against sample files | Where |
|---|---|---|
| The container and the previews | The 36 files of the CI set; the tests check every file's previews | [container-and-previews.md](container-and-previews.md); the files: [test-data.md, section 2.1](test-data.md#21-the-ci-set) |
| The MakerNote's layouts | The CI set, and five Sony camera JPEGs | [makernote-and-metadata.md, section 1](makernote-and-metadata.md#1-finding-the-makernote); the JPEGs: [JPEG test data, section 2](../jpeg/test-data.md#2-the-sample-files) |
| The MakerNote in a camera JPEG | The five Sony camera JPEGs | [makernote-and-metadata.md, "In a JPEG file"](makernote-and-metadata.md#in-a-jpeg-file); the files: [JPEG test data, section 2](../jpeg/test-data.md#2-the-sample-files) |
| The orientation, capture time, camera, sizes and lens | The CI set | [makernote-and-metadata.md, sections 9.1 to 9.5](makernote-and-metadata.md#9-viewer-metadata); the files: [test-data.md, section 2.1](test-data.md#21-the-ci-set) |
| The focus point | The CI set; the tests check every file's | [makernote-and-metadata.md, section 9.6](makernote-and-metadata.md#96-focus-point); the files: [test-data.md, section 2.2](test-data.md#22-the-focus-point-of-each-file) |
| The camera settings | The CI set and the five Sony camera JPEGs | [makernote-and-metadata.md, section 9.7](makernote-and-metadata.md#97-camera-settings); the files: [test-data.md, section 2.3](test-data.md#23-the-camera-settings-of-each-file) |
| Authorship and location | Counted on the ARW files of raw.pixls.us and on my own | [makernote-and-metadata.md, sections 9.8 and 9.9](makernote-and-metadata.md#98-authorship), which give the counts |
| The directories a file holds | Counted on the same kinds of file | [makernote-and-metadata.md, section 14](makernote-and-metadata.md#14-the-directories-a-file-holds) |
| The two ciphers and the SR2 keystream | Files of the CI set, with scripts kept outside the repository: the keystream on the 34 files that have an SR2SubIFD, the MakerNote cipher on four bodies | [makernote-and-metadata.md, sections 3 to 6](makernote-and-metadata.md#3-two-ciphers); the checks: [research-record.md, "Cross-checks made"](research-record.md#cross-checks-made) |
| The raw data | Files of the CI set, with scripts kept outside the repository: the cRAW blocks of 15 files, ARW1 on two, the other layouts on one each, the tile headers of four lossless Bayer files and of the M and S files, and the SRF key chain of one | [raw-data.md](raw-data.md); the checks: [research-record.md, "Cross-checks made"](research-record.md#cross-checks-made) |

A topic the table does not list is checked only where its page tags a
fact **[F]**. The deciphering and the decoding of the last two rows are
not in raw-files. The previews, the metadata and the focus point it reads
need none of them, and it walks SR2Private as a plain directory. On
those rows,
**no pixel output was compared with a reference decoder**. "Checked"
there means that the data decode consistently with the description,
not that they are bit-exact.

Byte offsets and formulas taken from other projects' source code should
still be checked against that source and against real files before they are
hard-coded. [open-questions.md](open-questions.md) lists what is still
unknown. [research-record.md](research-record.md) describes how the research
was done and what it could not reach. The files checked are listed in
[test-data.md](test-data.md#files-checked).

## Pages

| Page | Read it for |
|---|---|
| [container-and-previews.md](container-and-previews.md) | The TIFF structure of each member of the family, where the embedded JPEGs are, and how to pick the largest (previews) |
| [makernote-and-metadata.md](makernote-and-metadata.md) | The Sony MakerNote, its two ciphers, SR2Private, and orientation, date, camera, size and lens (metadata) and the focus point (focus points) |
| [raw-data.md](raw-data.md) | How the sensor data is stored: uncompressed, cRAW, 12-bit packed, ARW1, lossless tiles, M/S YCbCr, SR2, SRF, ARQ and the refused ARW 6.0 (raw sensor data) |
| [security.md](security.md) | Past vulnerabilities in Sony parsers and the Sony additions to the NEF checklist |
| [licenses-and-references.md](licenses-and-references.md) | Standards, existing implementations and their licenses, what raw-files may reuse, and the legal questions |
| [test-data.md](test-data.md) | Where sample files come from, which may be committed, a minimal body set, and the files checked |
| [open-questions.md](open-questions.md) | Decisions to take, facts still to establish, and what the samples, a decision or the code settled |
| [research-record.md](research-record.md) | How this documentation was researched and checked, and its limits |

## Confidence tags

Claims carry a tag:

- **[P]**: stated in a primary source:
  - a specification, cited by section, table or figure and printed page;
  - a camera maker's own documentation;
  - a project's, publisher's or site's own pages, for what they say of
    it: its license, terms, releases, contents or security record; and
    the NVD.
- **[S]**: secondary:
  - another program's documentation, source code or output;
  - a person's write-up, or an archive's format registry;
  - a forum post, issue or review, or a page read only through a search
    extract.
- **[I]**: inferred, derived by arithmetic, recalled without a source,
  taken from a single unconfirmed report, or a recommendation. Treat it
  as a hypothesis to test on sample files.
- **[F]**: measured on a named sample file.

Where a sample contradicts a source, the pages follow the sample and
record the disagreement.

Files are cited by their raw.pixls.us id ("file 4466"). Each page's
Sources give the URL and SHA-256 of the files it cites, and
[test-data.md](test-data.md#files-checked) lists all of them.

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
  the crop tags come from IFD0, the Exif IFD and the raw SubIFD. The tags
  raw-files reads from the MakerNote are in
  [makernote-and-metadata.md, section 9](makernote-and-metadata.md#9-viewer-metadata).
  The previews and the metadata raw-files reads need **no decryption and
  no deciphering** on any checked file [F].
- **The focus point is plain too.** FocusLocation (0x2027) gives the
  frame and the center of the focus area on every checked body from the
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
    A850, A900) and an ARW1 stream (the A200) [S][F];
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
  The MakerNote's separate byte-substitution cipher (b³ mod 249) hides
  values that only full metadata needs, such as the shutter count.
- **No mature ARW implementation can be copied into raw-files.** dcraw,
  LibRaw, rawspeed, rawler, libopenraw, ExifTool and exiv2 are read for
  understanding only. Graphite's rawkit is labeled MIT OR Apache-2.0, but
  its ARW1 table, cRAW decoding and 0x7010 tone curve match dcraw element
  for element [S]. That they were translated is [I]. It also panics on
  unexpected input ([licenses-and-references.md](licenses-and-references.md)).
- **Few tables need a decision.** Unlike NEF, the ARW1 prefix code, the
  cipher inverse and the keystream reduce to rules or formulas, and per-body
  crops and levels can be derived from samples. The ARW 6.0 curve, published
  only as extracted tables, is the hard case
  ([open-questions.md](open-questions.md)).

## Where to start, by capability

- **Previews:**
  - [container-and-previews.md](container-and-previews.md), especially its
    recommended algorithm and the expected results on the checked files.
  - [security.md](security.md), the preview items and the NEF checklist
    items they build on (offsets, IFD loops, allocation).
  - [test-data.md](test-data.md), for the smaller set of files that covers
    preview tests.
- **Metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the viewer
    metadata section and its recommendation.
- **Full metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the key tags and
    what each records, the camera settings, and the enciphered blocks with
    what they hold.
  - [open-questions.md](open-questions.md), for what must be decided before
    an enciphered block is read.
- **Focus points:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the focus
    point section, with what the samples cover and what they do not.
- **Raw sensor data:**
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
  which only a whole-chain walk reaches. Two smaller differences remained,
  and raw-files' one finder takes the ARW rule in both (see
  [open-questions.md](open-questions.md#5-questions-settled-by-a-decision-or-by-the-code)).
  Orientation is handled the same way in both: return the JPEG unchanged
  and report IFD0's tag separately.
- **Security.** The NEF checklist items 1 to 14 apply to ARW unchanged;
  [security.md](security.md) adds Sony-specific items numbered A1 onwards.
- **Licenses and test data.** The NEF rules on reading copyleft sources,
  the options for data tables, and downloading samples in CI pinned by
  SHA-256 apply here too.

Two things differ: ARW needs no built-in Huffman tables (lossless tiles
carry their own) and no MakerNote cipher for anything but full metadata, and
the ARW pages were checked against sample files from the start, while the
NEF pages were desk research first, and their
[Status](../nef/README.md#status) says what has been checked since.

## License hygiene

raw-files is MIT OR Apache-2.0 ([ADR002](../../architecture-decisions/adr002-license-mit-or-apache-2.md)).
These pages restate algorithms in their own words, as prose, numbered
steps, tables and formulas, never as another program's code or
structure. They copy no code and no wholesale data tables: the ARW1
prefix code is given as a rule, the MakerNote cipher's inverse and the
SR2 keystream as formulas, and no tone curve, delinearization curve or
per-body table is reproduced.
Quotations stay under fifteen words; individual facts such as tag IDs,
offsets, value meanings and per-body values are cited to their source. Keep
it that way when extending them.
