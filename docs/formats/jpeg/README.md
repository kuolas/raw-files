# JPEG files

What raw-files needs to know about JPEG files, as cameras write them
beside their raw files and as editors export them. Unlike a raw format, a
JPEG file is described by published standards: the JPEG standard (ITU-T
T.81), JFIF (ITU-T T.871), Exif (CIPA DC-008) and the Multi-Picture Format
(CIPA DC-007). These pages cite them by section, and check them against
camera-original JPEG files.

These pages support the [roadmap](../../ROADMAP.md)'s milestone for plain
JPEG files: a JPEG opens as a format of its own, with its metadata, its
previews and, when the camera recorded one that fits its picture, its focus
point. Why raw-files reads JPEG files, and how, is decided in
[ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md),
and how the focus point of a Nikon JPEG shot in a crop mode is placed on
its picture, in
[ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md).

## Status

These pages were written in October 2026 from the four specifications, in
the editions listed in [licences-and-references.md](licences-and-references.md),
and checked against eight camera-original JPEGs from Wikimedia Commons, of
Sony, Nikon and Canon bodies, listed in
[test-data.md](test-data.md#2-the-sample-files); what they show is tagged
**[F]**. The tests download them and check every preview, metadata value
and focus point of each. [open-questions.md](open-questions.md) lists what is still
unknown, among it the orientation of the small images on a portrait
picture on a public file. What a Nikon JPEG records in a crop mode is
known from the maintainer's own Z 9 shots, not from a public file.

## Pages

| Page | Read it for |
|---|---|
| [container-and-metadata.md](container-and-metadata.md) | The marker segments, the frame header, the JFIF, Exif and Multi-Picture Format segments, the thumbnail, what lies after the last image, and how raw-files reads a JPEG |
| [security.md](security.md) | Past vulnerabilities in JPEG and Exif parsers and the checklist that follows from them |
| [licences-and-references.md](licences-and-references.md) | Where the standards are published and on what terms, the sample files' licences, and the clean-room rule |
| [test-data.md](test-data.md) | The sample files, how they were chosen, the fixtures, and what the samples do not cover |
| [open-questions.md](open-questions.md) | Decisions to take and facts still to establish |

The MakerNotes that Nikon and Sony write into their JPEGs are described
with their raw files: the [NEF](../nef/makernote-and-metadata.md) and
[ARW](../arw/makernote-and-metadata.md) MakerNote pages.

## Confidence tags

Claims carry a tag:

- **[P]**: stated in a primary source: a specification, cited by section,
  table or figure and printed page, or a publisher's page.
- **[S]**: secondary: another program's documentation or output, a
  write-up, a search.
- **[I]**: inferred, or a recommendation. Treat it as a hypothesis to test
  on sample files.
- **[F]**: measured on a named sample file. Files are cited by body ("the
  ILCE-7M3 file"), and [test-data.md](test-data.md#2-the-sample-files)
  lists each with its Commons name, licence and SHA-256.

Where a sample contradicts a specification, the pages follow the sample
and record the disagreement.

## JPEG at a glance

- **A JPEG file is a series of marker segments**: SOI, tables and
  application segments, the frame header that gives the picture's size,
  then the scan with the compressed picture, up to EOI. A reader looking
  for segments stops at the scan. [P]
- **The camera's metadata is a TIFF structure inside an APP1 segment**,
  the Exif segment: the same IFDs and tags as in a NEF or an ARW, with
  offsets counted from the TIFF header inside the segment, and all of it
  within 64 KiB. On the camera files it is the first segment, so its TIFF
  header lies at byte 12. [P][F]
- **The previews are the file itself and the smaller images it holds**:
  a thumbnail in IFD1 (160×120 on most camera files), the large
  thumbnails that the Multi-Picture Format lists after the main image
  (1616×1080 or 1620×1080, and 640×424 on the D5300), and on older Nikon
  bodies a MakerNote preview. [P][F]
- **There is no raw size**: the image size is the frame header's. [I]
- **The MakerNotes are the raw files' ones.** Nikon's layout carries its
  own TIFF header; Sony's JPEGs start their MakerNote with a `SONY DSC`
  header and count its offsets from the Exif segment's TIFF header. [F]
- **Sony pads its files with zeros** after the last image, up to a
  multiple of 64 KiB, on three of the five Sony files. [F]
- **An exported JPEG may keep the camera's MakerNote** after its pixels
  were cropped or turned, so a focus point is kept only when its frame
  has the picture's aspect ratio
  ([ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
  [I]

## Where to start

- **Opening a JPEG and listing its previews**:
  [container-and-metadata.md](container-and-metadata.md), sections 1, 2,
  6, 7 and 12, and the [security.md](security.md) checklist.
- **Its metadata and focus point**:
  [container-and-metadata.md](container-and-metadata.md) sections 4, 9
  and 10, then the NEF and ARW MakerNote pages.
- **Tests**: [test-data.md](test-data.md).

## Licence hygiene

raw-files is MIT OR Apache-2.0 ([ADR002](../../architecture-decisions/adr002-license-mit-or-apache-2.md))
and a clean-room implementation ([ADR013](../../architecture-decisions/adr013-clean-room-and-third-party-code.md)).
These pages restate the standards in their own words, citing section and
page, and quote at most a few words of them; they copy no code and no
other project's tables. Keep it that way when extending them.
