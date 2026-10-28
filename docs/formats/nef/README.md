# Nikon NEF

What raw-files needs to know about Nikon's NEF raw format, collected from
public sources. Nikon publishes no specification, so everything here comes from
the TIFF and Exif standards, community documentation and the source code of
existing raw decoders, each cited where it is used.

Where these notes say *I* or *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files, writing about my own cameras and shots: they stay
private, and are cited by counts or by frame numbers.

These pages support the [roadmap](../../ROADMAP.md): NEF previews first, then
basic metadata, then full metadata, then raw decoding.

## Status

This is desk research, done in October 2026. On 13 October 2026 the container
and preview notes were checked against the three CC0 Nikon Z 9 files that CI
downloads (files 5146 to 5148, listed in
[test-data.md](test-data.md#21-the-ci-set)); what they show is tagged
**[F]**. On 15 October 2026 the MakerNote's layout and the viewer
metadata were checked against those three files and 20 more CC0 files,
from the D1H to the Z6III, which CI downloads too (listed in
[test-data.md](test-data.md#21-the-ci-set)). On 19 October 2026 the
autofocus data was checked against those 23 files and seven more CC0
files, and on 20 October 2026 against three more, which CI downloads too
(listed in
[test-data.md](test-data.md#23-files-checked-for-the-focus-point)). On
21 October 2026 the MakerNote of two Nikon camera JPEGs from Wikimedia
Commons, a D5300's and a D70's, was checked as well
([makernote-and-metadata.md](makernote-and-metadata.md#in-a-jpeg-file),
listed in the [JPEG test data](../jpeg/test-data.md#2-the-sample-files)).
On 23 October 2026 two D1X files were added, which CI downloads too, to
check the headerless MakerNote on a second body (listed in
[test-data.md](test-data.md#21-the-ci-set)). The same day the headerless
and short-header MakerNotes were checked by hand on camera JPEGs of
early Coolpix bodies from Wikimedia Commons, which are not downloaded
([makernote-and-metadata.md](makernote-and-metadata.md#in-a-jpeg-file),
listed in the
[JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand)).
The tests check the previews of all 35, and test-data.md records them, but
the containers of the 32 files of 25 other bodies are not yet written up
in these pages. On 26 October 2026 the autofocus points of a D600 were
measured on 56 frames shot with my own D600, read in Nikon's NX Studio as a
reader to compare with ([autofocus-points.md](autofocus-points.md)); the
frames stay private and are cited by counts and frame numbers. The raw data
was not checked, beyond the raw IFD's tags and the first bytes of the Z 9's
strip. **Everything else has yet to be checked against sample files.** Byte
offsets and formulas taken from other projects' source code should be
checked against that source and against real files before they are
hard-coded. [open-questions.md](open-questions.md)
lists what is still unknown. [research-record.md](research-record.md)
describes how the research was done and what it could not reach.

## Pages

| Page | Read it for |
|---|---|
| [container-and-previews.md](container-and-previews.md) | The TIFF structure of a NEF, where the embedded JPEGs are, and how to pick the largest (milestone 1) |
| [makernote-and-metadata.md](makernote-and-metadata.md) | The Nikon MakerNote, its encrypted blocks, and orientation, date, camera and size (milestone 3) and the focus point (milestone 4) |
| [autofocus-points.md](autofocus-points.md) | The procedure by which a body's autofocus points are measured, and the record of each body measured, the D600 first (milestone 4) |
| [raw-data.md](raw-data.md) | How the sensor data is stored: Huffman compression, packed layouts, small raws, HE/HE\* (milestone 7) |
| [security.md](security.md) | Past vulnerabilities in NEF parsers and the checklist that follows from them |
| [licences-and-references.md](licences-and-references.md) | Standards, existing implementations and their licences, and what raw-files may reuse |
| [test-data.md](test-data.md) | Where sample files come from, which may be committed, and which bodies to cover |
| [open-questions.md](open-questions.md) | Decisions to take and facts still to establish |
| [research-record.md](research-record.md) | How this documentation was researched, and its limits |

## Confidence tags

Claims carry a tag:

- **[P]** — stated in a primary source: a specification, a tag reference such
  as ExifTool's, or an implementation's source code.
- **[S]** — secondary: a forum post, issue, blog or search extract.
- **[I]** — inferred, or from a single unconfirmed report. Treat it as a
  hypothesis to test on sample files.
- **[F]** — measured on a named sample file. Files are cited by their
  raw.pixls.us id ("file 5146"), and
  [test-data.md](test-data.md#2-what-is-committed-and-what-is-downloaded)
  lists them with their SHA-256.

Where a sample contradicts a source, the pages follow the sample and record
the disagreement.

## NEF at a glance

- **A NEF is a TIFF file.** It has IFD0, SubIFDs (tag 0x014A) and an Exif IFD
  whose MakerNote (0x927C) holds Nikon's own tags. The outer byte order is
  big-endian (`MM`) on the DSLRs documented up to about 2009 [P], and on
  the checked DSLRs from the D1H to the D7000 and on the Coolpix A [F].
  The D750 and every later checked body, the Z 9 among them, are
  little-endian (`II`), in the file and in the MakerNote alike, and so are
  the Coolpix E5700 and P7800 and the Nikon 1 J1 [F]. A reader must
  accept both orders.
- **Previews are JPEGs in SubIFDs and in the MakerNote.**
  - Bodies from about 2012 carry a full-size JPEG plus a second JPEG of about
    1620×1080 [S].
  - The Z 9 holds four JPEGs: 160×120, 640×424, 1620×1080 and 8256×5504
    [F].
  - The MakerNote's PreviewIFD (0x0011) holds a small JPEG. Its offsets count
    from the MakerNote's embedded TIFF header, not from the start of the file
    [P].
  - IFD0 holds a 160×120 thumbnail: uncompressed on the DSLRs documented
    [P], a JPEG on the Z 9 [F].
  - SubIFD order is not fixed, so choose by measured JPEG size.
- **Viewer metadata is in standard tags.** Orientation, capture date, make and
  model come from IFD0 and the Exif IFD; the stored sensor size comes from the
  raw SubIFD, since IFD0's own size is the thumbnail's [P]. The MakerNote is
  needed only for the crop area, black level, compression type and white
  balance.
- **Compressed raw data uses six fixed Huffman tables.**
  - The tables are built into decoders; the file does not store them.
  - Each pixel is predicted from the pixel of the same colour two columns to
    the left.
  - A linearisation curve in MakerNote 0x0096 maps the decoded values. Nikon's
    "lossy" modes are that curve [S].
- **HE and HE\*** (Z 9 and later) are a JPEG XS-based codec. JPEG XS is
  covered by a patent pool; whether it applies to HE is a legal question. No
  open decoder has been released. raw-files should detect them and refuse them
  [P/S].
- **No mature NEF implementation can be copied into raw-files.** They are GPL,
  LGPL, CDDL or under dcraw's own terms. Small permissively licensed crates
  exist but are immature. The mature ones can be read for understanding only
  (see [licences-and-references.md](licences-and-references.md)).

## Where to start, by milestone

- **Milestone 1, NEF previews:**
  - [container-and-previews.md](container-and-previews.md), especially its
    recommended selection algorithm.
  - [security.md](security.md) checklist items on offsets, IFD loops and
    allocation.
- **Milestone 3, basic metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the viewer
    metadata section.
- **Milestone 4, focus points:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the focus
    point section, with what the samples cover and what they do not.
- **Milestone 6, full metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the key tags and
    what each records, the camera settings, and the encrypted blocks with
    what they hold.
  - [open-questions.md](open-questions.md), for what must be decided before
    an encrypted block is read.
- **Milestone 7, raw decoding:**
  - [raw-data.md](raw-data.md).
  - [makernote-and-metadata.md](makernote-and-metadata.md), for black level,
    white balance and decryption.
  - [open-questions.md](open-questions.md), for the data-table decisions that
    must be taken first.

## Licence hygiene

raw-files is MIT OR Apache-2.0 ([ADR002](../../architecture-decisions/adr002-license-mit-or-apache-2.md)).
These pages restate algorithms in prose or in original pseudocode. They copy
no code and no wholesale data tables (such as Huffman symbol lists or
decryption tables); individual facts such as tag IDs, value meanings and
per-body values are cited to their source. Keep it that way when extending
them.
