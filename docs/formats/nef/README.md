# Nikon NEF

What raw-files needs to know about Nikon's NEF raw format, collected from
public sources. Nikon publishes no specification, so everything here comes from
the TIFF and Exif standards, community documentation and the source code of
existing raw decoders, each cited where it is used.

Where these notes say *I* or *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files, writing about my own cameras and shots: they stay
private, and are cited by counts or by frame numbers.

These pages support the [roadmap](../../ROADMAP.md)'s capabilities for NEF
files: previews; metadata, both what raw-files reads from the plain
tags and full metadata, which is still open; focus points; and raw
sensor data.

## Status

These pages began as desk research, as the
[research record](research-record.md) says. The table shows what has
since been checked against sample files, and where. What a file shows
is tagged **[F]**.

| Topic | Checked against sample files | Where |
|---|---|---|
| The container and the previews | The Z 9's three files, and a few more the page names; the tests also check the previews of the CI set's other files, whose containers the page does not describe | [container-and-previews.md](container-and-previews.md); the files: [test-data.md, section 2.1](test-data.md#21-the-ci-set) |
| The MakerNote's layouts | The CI set, with two bodies of the headerless layout; by hand, camera JPEGs of early Coolpix bodies, which are not downloaded | [makernote-and-metadata.md, section 1](makernote-and-metadata.md#1-finding-the-makernote); the Coolpix files: [JPEG test data, section 6](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand) |
| The MakerNote in a camera JPEG | A D5300's and a D70's JPEG | [makernote-and-metadata.md, "In a JPEG file"](makernote-and-metadata.md#in-a-jpeg-file); the files: [JPEG test data, section 2](../jpeg/test-data.md#2-the-sample-files) |
| The orientation, capture time, camera, sizes, serial number, ISO and lens | The CI set | [makernote-and-metadata.md, sections 8.1 to 8.5](makernote-and-metadata.md#8-viewer-metadata); the files: [test-data.md, section 2.2](test-data.md#22-files-checked-for-metadata) |
| The focus point | The CI set, and shots of my own Z 9 and D600 | [makernote-and-metadata.md, section 8.6](makernote-and-metadata.md#86-focus-point); the files: [test-data.md, section 2.3](test-data.md#23-files-checked-for-the-focus-point) |
| A body's autofocus points | The D600's, on frames I shot with my own D600 | [autofocus-points.md, section 2](autofocus-points.md#2-nikon-d600) |
| The camera settings | The CI set, two Nikon camera JPEGs and files of my own | [makernote-and-metadata.md, section 8.7](makernote-and-metadata.md#87-camera-settings); the files: [test-data.md, section 2.4](test-data.md#24-files-checked-for-the-camera-settings) |
| Authorship and location | Counted on the NEF files of raw.pixls.us and on my own | [makernote-and-metadata.md, sections 8.8 and 8.9](makernote-and-metadata.md#88-authorship), which give the counts |
| The directories a file holds | Counted on the same kinds of file | [makernote-and-metadata.md, section 12](makernote-and-metadata.md#12-the-directories-a-file-holds) |
| The raw data | Only the raw IFD's tags, the compression tags and the first bytes of the Z 9's strip; no strip was decoded | [raw-data.md](raw-data.md#how-to-read-this-page) |
| The encrypted blocks and their decryption, the white balance multipliers, LensData and ShotInfo | Not checked: from the sources alone | [makernote-and-metadata.md, sections 4 to 7](makernote-and-metadata.md#4-encrypted-blocks) |

A topic the table does not list rests on the sources alone. Byte offsets
and formulas taken from other projects' source code should be checked
against that source and against real files before they are hard-coded.
[open-questions.md](open-questions.md) lists what is still unknown.
[research-record.md](research-record.md) describes how the research was
done and what it could not reach.

## Pages

| Page | Read it for |
|---|---|
| [container-and-previews.md](container-and-previews.md) | The TIFF structure of a NEF, where the embedded JPEGs are, and how to pick the largest (previews) |
| [makernote-and-metadata.md](makernote-and-metadata.md) | The Nikon MakerNote, its encrypted blocks, and orientation, date, camera and size (metadata) and the focus point (focus points) |
| [autofocus-points.md](autofocus-points.md) | The procedure by which a body's autofocus points are measured, and the record of each body measured, the D600 first (focus points) |
| [raw-data.md](raw-data.md) | How the sensor data is stored: Huffman compression, packed layouts, small raws, HE/HE\* (raw sensor data) |
| [security.md](security.md) | Past vulnerabilities in NEF parsers and the checklist that follows from them |
| [licenses-and-references.md](licenses-and-references.md) | Standards, existing implementations and their licenses, and what raw-files may reuse |
| [test-data.md](test-data.md) | Where sample files come from, which may be committed, and which bodies to cover |
| [open-questions.md](open-questions.md) | Decisions to take, facts still to establish, and what the samples, a decision or the code settled |
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
  raw SubIFD, since IFD0's own size is the thumbnail's [P]. What the key
  tags of the MakerNote are used for is in
  [makernote-and-metadata.md, section 2](makernote-and-metadata.md#2-key-tags).
- **Compressed raw data uses six fixed Huffman tables.**
  - The tables are built into decoders; the file does not store them.
  - Each pixel is predicted from the pixel of the same color two columns to
    the left.
  - A linearization curve in MakerNote 0x0096 maps the decoded values. Nikon's
    "lossy" modes are that curve [S].
- **HE and HE\*** (Z 9 and later) are a JPEG XS-based codec. JPEG XS is
  covered by a patent pool; whether it applies to HE is a legal question. No
  open decoder has been released. raw-files should detect them and refuse them
  [P/S].
- **No mature NEF implementation can be copied into raw-files.** They are GPL,
  LGPL, CDDL or under dcraw's own terms. Small permissively licensed crates
  exist but are immature. The mature ones can be read for understanding only
  (see [licenses-and-references.md](licenses-and-references.md)).

## Where to start, by capability

- **NEF previews:**
  - [container-and-previews.md](container-and-previews.md), especially its
    recommended selection algorithm.
  - [security.md](security.md) checklist items on offsets, IFD loops and
    allocation.
- **Metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the viewer
    metadata section.
- **Full metadata:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the key tags and
    what each records, the camera settings, and the encrypted blocks with
    what they hold.
  - [open-questions.md](open-questions.md), for what must be decided before
    an encrypted block is read.
- **Focus points:**
  - [makernote-and-metadata.md](makernote-and-metadata.md), the focus
    point section, with what the samples cover and what they do not.
- **Raw sensor data:**
  - [raw-data.md](raw-data.md).
  - [makernote-and-metadata.md](makernote-and-metadata.md), for black level,
    white balance and decryption.
  - [open-questions.md](open-questions.md), for the data-table decisions that
    must be taken first.

## License hygiene

raw-files is MIT OR Apache-2.0 ([ADR002](../../architecture-decisions/adr002-license-mit-or-apache-2.md)).
These pages restate algorithms in prose or in original pseudocode. They copy
no code and no wholesale data tables (such as Huffman symbol lists or
decryption tables); individual facts such as tag IDs, value meanings and
per-body values are cited to their source. Keep it that way when extending
them.
