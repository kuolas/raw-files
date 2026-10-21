# ADR014: JPEG files as a format of their own

- Status: Accepted
- Builds on: [ADR009](adr009-public-api-shape.md), the shape of the
  public API; [ADR011](adr011-metadata-from-standard-tags.md), metadata
  from the standard tags; [ADR012](adr012-focus-points-as-the-camera-records-them.md),
  focus points as the camera records them
- Date: 2026-10-21
- Issue: #35

## Context

Many photographers have the camera write a JPEG beside each raw file, or
shoot JPEGs alone, and keep the JPEGs an editor exports beside the raw
files they came from. A culler shows all of these in one list. From a
JPEG it needs what it takes from a raw file: the capture time, the camera
and lens, the exposure and the orientation, small previews for its grid,
and the focus point to judge sharpness. It also needs to tell a raw file
from a JPEG, to pair the two files of one shot.

A file format is three things that vary independently. Its container
arranges the file's parts: NEF and ARW files are TIFF structures, a
JPEG is a series of marker segments, and Canon's CR3 files, on the
[roadmap](../ROADMAP.md), are neither. The codecs of the images inside
it are another matter: Nikon's HE and HE\* NEF files keep the TIFF
container and code their raw data with a codec based on JPEG XS (see
the [NEF notes](../formats/nef/raw-data.md#he-and-he)). Whether it holds
a camera's raw sensor data is a third: a JPEG holds none. None of the
three says what a caller can get from a file. An HE\* NEF gives its
previews and its metadata whether or not its raw data can be decoded,
as [ADR009](adr009-public-api-shape.md) has every capability succeed or
fail on its own; a JPEG, which is neither a TIFF structure nor a raw
file, records the same metadata and holds previews of its own. Until a
format is read, `open` refuses its files with the `UnknownFormat`
error, the one for a file whose format raw-files does not recognise.

What a JPEG file holds is set out in public standards:

- **The JPEG standard** (ITU-T T.81) describes a JPEG stream as a series
  of marker segments: the start of the image (SOI), tables and
  application data, the frame header, which gives the image's size and
  how it is coded, and then the scan, which holds the compressed image up
  to the end marker (EOI). A lossless frame keeps every sample value
  exactly, which is how some raw files store their raw data, not a
  picture to show.
- **JFIF** (ITU-T T.871) puts an APP0 segment right after SOI, and Exif
  puts its APP1 segment there. Neither standard mentions the other, so
  in a file that follows both, one of the two segments comes second.
- **Exif** (CIPA DC-008) puts the camera's metadata in an APP1 segment:
  the word `Exif` and two zero bytes, then a whole TIFF structure, the
  same structure a NEF or an ARW file is, whose offsets count from its
  own header inside the segment. Its IFD0 and Exif IFD hold the standard
  tags that [ADR011](adr011-metadata-from-standard-tags.md) reads; its
  IFD1 holds a small JPEG thumbnail; its MakerNote holds the
  manufacturer's own data, in a layout the standard leaves to the
  manufacturer. A segment's length is a two-byte number, so all of it
  fits in less than 64 KiB.
- **The Multi-Picture Format** (CIPA DC-007, MPF) lets a file hold more
  images after the end of the first: an APP2 segment lists them, each with
  its type, size and offset, counted from the list's own header. The
  types include large thumbnails for screens, with the main image's
  aspect ratio and at most 640×480 or 1920×1080 pixels, among other
  sizes.

We read eight camera-original JPEGs published on Wikimedia Commons under
open licences, one from each of eight bodies, checked byte by byte to be
as the camera wrote them:

| Body | File on Wikimedia Commons |
|---|---|
| Sony ILCE-7M3 | `Acanthodactylus_erythrurus_2025.jpg` |
| Sony ILCE-7RM2 | `Ulpu_Pajari_puhumassa_vastaanotettuaan_tiedonjulkistamisen_valtionpalkinnon_2.jpg` |
| Sony DSC-RX100M4 | `여의도한강공원샛강생태공원문화다리_09.jpg` |
| Sony ILCE-6000 | `140319_MS_Queen_Elizabeth_Kobe_Japan01o.JPG` |
| Sony DSLR-A100 | `"Krone"_flag_on_line_5_tram_(2564030031).jpg` |
| Nikon D5300 | `"Tours"_French_restaurant_in_Omotesando.JPG` |
| Nikon D70 | `"Stadswandeling".JPG` |
| Canon PowerShot G7 X | `DC-3_OH-LCH_at_EFJM_20160717.jpg` |

They showed:

- **Each is laid out as the Exif standard says.** The Exif APP1 segment
  comes right after SOI. IFD0 names the make, the model and the
  orientation; the Exif IFD holds the capture time and the exposure; IFD1
  holds a JPEG thumbnail of 160×120 pixels (128×85 on the D70). The
  thumbnail and the MakerNote lie inside the segment, and the size in the
  frame header equals the size the Exif tags record (PixelXDimension and
  PixelYDimension) on every file. No file has a second Exif segment.
- **Most hold larger previews in MPF.** The four newer Sony files and the
  G7 X list one image of 1616×1080 or 1620×1080 pixels, the D5300 two, of
  640×424 and 1620×1080, all stored after the end of the main image. The
  D70 and the A100 hold no MPF segment.
- **The MakerNotes are those of the raw files, with offsets counted from
  the segment's TIFF header.** The D5300 and D70 files hold the Nikon
  layout that NEF files use, which carries a TIFF header of its own as the
  base of its offsets. Every Sony file starts its MakerNote with the
  12-byte header `SONY DSC ` and three zero bytes, which among the ARW
  samples only the A100's has, and counts its offsets from the TIFF
  header inside the Exif segment, as an ARW counts them from the start of
  the file. Read that way, the ILCE-7M3 records its focus point off
  centre, at (1783, 1804) in a frame of 3936×2624, the size of its
  picture, and the ILCE-7RM2 at the exact centre (3976, 2652) of
  7952×5304.
- **The D70's MakerNote holds a preview too.** Its PreviewIFD holds a
  JPEG of 564×372 pixels, inside the length the MakerNote declares; the
  D70's NEF sample holds one of the same size (raw.pixls.us file 2060).
- **One value points outside the segment.** The ILCE-6000's MakerNote
  tag 0x2001 points at 549,518 bytes inside the MPF image after the main
  image. No reader in raw-files reads that tag.
- **Sony pads its files.** The ILCE-7M3, ILCE-7RM2 and ILCE-6000 files
  end with zeros after the last image's end marker, 28,400 to 31,620
  bytes of them, up to a multiple of 64 KiB; the RX100M4 and the A100
  end at the marker.

A JPEG holds no raw data, so nothing in it gives a raw size, and its
pixels are the picture.

An editor that exports a JPEG writes new pixels: it may crop them, scale
them, or turn them upright and record that no turn is needed. It may
also copy the camera's Exif segment, MakerNote included, into the export
unchanged, since it cannot rewrite a block whose layout it does not know.
Such a file's MakerNote still records the frame and focus point of the
camera's picture, which may no longer be the file's.

A program that takes any file whose extension `Format::from_extension`
knows for a raw file would take every `.jpg` for one, once JPEG is a
`Format`.

## Decision

We will accept a file format for what raw-files can read from its
files, its metadata and its previews, whatever its container, the
codecs inside it, and whether it holds raw data. On that ground we will
open a JPEG file as a format of its own, `Format::Jpeg`, through the
same handle and into the same types as a raw file, and report from it
what the file records about its own picture.

- **Detected by its content: the container first, then the format.**
  `open` recognises the container by the file's first bytes and builds
  its structure, and the formats that container can hold are then asked
  whether the structure is theirs. A file whose container raw-files does
  not know, or that no format claims, is refused with the `UnknownFormat`
  error. A new format adds a `Format` variant and a reader; a format in a
  new container, such as CR3, also adds that container's structure. A
  JPEG begins with `FF D8 FF`; its segments are walked from SOI to the
  start of the scan, for at most a fixed number of steps, and it opens as
  `Format::Jpeg` when they hold a frame header that gives a size, neither
  side zero, and is not lossless. A NEF's embedded JPEG, saved on its own,
  is a JPEG.
- **Whether a format holds raw data is said by the format.** `Format::Jpeg`
  is listed by `Format::all` and found by `Format::from_extension` (`jpg`,
  `jpeg`). `Format::is_raw` is true for a format whose files hold a
  camera's raw sensor data, as NEF and ARW do, and false for JPEG, so
  that a program tells raw formats from the others without matching
  every variant of an enum that will gain more. It says nothing of the
  container or the codecs. The handle keeps its name, `RawFile`; its
  documentation says that it opens a camera file raw-files reads, a raw
  file or a JPEG.
- **The first Exif segment holds the metadata.** It is the first APP1
  segment before the scan whose payload starts with `Exif` and two zero
  bytes, wherever it lies; a later one is skipped, as XMP, colour
  profiles and every other application segment are.
- **The Exif segment is a bounded block.** Its TIFF structure is read
  with the position of its header as the base of every offset, and every
  IFD, value, thumbnail and MakerNote must lie inside the segment. One
  that lies outside is left out as damage, as an entry that lies outside
  a MakerNote is.
- **A JPEG without usable Exif still opens.** When the file has no Exif
  segment, or the segment's TIFF header or IFD0 cannot be read, it opens
  with every metadata value absent and its previews shown as stored.
  Such a segment costs the metadata and the previews that it holds, the
  thumbnail and the MakerNote's preview, never the file: the file itself
  and the images of its MPF list, a segment of its own, remain previews.
- **The previews are the file and the smaller images it holds.** The
  file itself is a preview: its whole byte range, exactly as stored, the
  MPF images and any padding after the main image included, with the size
  of its frame header. A decoder stops at the end of the first image. The
  others are IFD1's JPEG thumbnail; each image the MPF list holds after
  the first, at its offset from the list's header; and, on a Nikon file,
  the JPEG of the MakerNote's PreviewIFD, under the rule that holds in a
  NEF: it must lie wholly inside the length the MakerNote declares. Each
  must lie inside the file, the thumbnail inside the Exif segment, and
  must start with SOI and hold, within its own bytes, a frame header that
  gives a size and is not lossless, as every preview raw-files reports.
  Each takes IFD0's orientation.
- **The image size is the picture's.** A JPEG has no raw size. The image
  size is the size in the file's frame header. The Exif pixel dimensions
  are not read, as [ADR011](adr011-metadata-from-standard-tags.md)
  decided for raw files, and a Nikon CropArea does not change the image
  size of a JPEG.
- **The metadata is read as in a raw file.** The reader of
  [ADR011](adr011-metadata-from-standard-tags.md) reads IFD0 and the Exif
  IFD. Then, by IFD0's Make, the Nikon or the Sony MakerNote reader reads
  what it reads in a NEF or an ARW, with the TIFF header inside the Exif
  segment in place of the start of the file wherever the maker counts
  from the outer TIFF header.
- **The focus point only where its frame fits the picture.** The rules of
  [ADR012](adr012-focus-points-as-the-camera-records-them.md) apply
  first, unchanged. Then the point is kept only when its frame has the
  JPEG's aspect ratio, to within one pixel of rounding in either of the
  two sizes: when the frame's width times the JPEG's height and the
  frame's height times the JPEG's width differ by at most the longest of
  the four sides, all in the stored orientation. A camera's own JPEG
  passes. An export cropped to another shape, or turned upright in its
  pixels, gives no focus point. A point that is kept is scaled onto any
  preview as in a raw file.
- **No new limits.** The segment walk stops after a fixed number of
  steps, a marker or a fill byte each, as the search for a preview's
  frame header does; the Exif segment is bounded by its two-byte
  length; the MPF list is an IFD under the existing limit on entries;
  and reading a preview, the file included, stays under the existing
  limit on preview bytes.

Rejected:

- **Refusing a JPEG without Exif with the `UnknownFormat` error, or a
  damaged Exif segment with an error of its own.** A culler would lose a
  picture it can still show, for the want of the data beside it.
- **The file and IFD1's thumbnail only.** A culler's grid and loupe would
  have nothing between a 160×120 thumbnail and the full image, which it
  would decode at full size to show a tile, although most camera JPEGs
  hold an image of 1616×1080 or 1620×1080 for that.
- **Leaving the MakerNote's preview out.** A D70 JPEG would give fewer
  previews than the raw file of the same body.
- **The size the MPF list gives the first image, as the extent of the
  file's own preview.** The whole file is what a viewer opens, decoders
  stop at the end of the first image anyway, and a file without MPF has
  no such size, while a damaged or edited list could give a wrong one.
- **A tolerance of the frame's longer side only.** It allows for a pixel
  of rounding in the JPEG's size, not in the frame's: a 640×427 frame,
  3:2 rounded, against a 6000×4000 JPEG differs by 2,000, more than 640,
  and a correct point would be dropped.
- **Two Exif segments taken as damage.** A program that adds a segment
  after the camera's would cost the file its metadata; skipping the later
  one keeps the camera's.
- **The Exif pixel dimensions as the image size, or the frame's size as
  the raw size too.** The pixel dimensions are a tag that an editor may
  leave unchanged, while the frame header is the picture; and a raw size
  would claim raw data that a JPEG does not hold.
- **The Exif structure bounded by the file only.** The standard puts all
  of it inside the segment, and a value outside, such as the ILCE-6000's
  pointer into its MPF image, is not one the standard tags describe.
- **The focus point whatever its frame**, which misplaces it on an export
  cropped to another shape; **no focus point in any JPEG**, which costs
  every camera JPEG its point; and **no focus point in any file an editor
  has touched**, judged by marks such as a JFIF segment or a raw file's
  CropArea. Such marks do not tell whether the pixels still have the
  camera's framing, and whether the frame fits the picture does.
- **Renaming `RawFile`**, for example to `CameraFile` with the old name
  kept as a deprecated alias. Callers would change code for a name and
  gain nothing they need.
- **Leaving `jpg` out of `Format::from_extension`.** It and `Format::all`
  would disagree, and a file picker built from them would not offer
  JPEGs.
- **A note in the changelog in place of `Format::is_raw`.** The next
  format that is not raw, such as HEIF, would set the same trap.
- **A public limit on the number of JPEG segments.** No caller needs to
  tune it, and a fixed bound keeps the walk short on any file.
- **Opening a JPEG outside the table of formats.** There would be two
  ways to open a file, and a third container would add a third.
- **Accepting only raw formats, or only formats in a TIFF structure.**
  Neither says what a caller can get from a file: the JPEG beside a raw
  file records the same metadata, and a raw format in another container,
  such as CR3, would be refused for its container alone.

## What this decision does not cover

Other containers that cameras and phones write, such as HEIF; the XMP
segment, which can record an editor's crop, rating or labels; the
MakerNotes of other makers; telling an exported JPEG from a camera's by
anything but the shape of its picture; decoding a JPEG's pixels; writing
or editing files; and pairing a JPEG with the raw file of the same shot.
Each is a decision of its own, which builds on this one.

## Consequences

- A culler lists JPEGs beside raw files through one API, gets the same
  `Metadata` from both, and tells them apart with `Format::is_raw`.
- `Format::from_extension("jpg")` returns `Some(Format::Jpeg)` where it
  returned `None`. A program that took any known extension for a raw file
  must switch to `Format::is_raw`; the changelog says so, since a check of
  the API's signatures cannot see a change of behaviour.
- Opening a JPEG reads its segments up to the scan, the Exif and MPF
  segments, and the start of each MPF image up to its frame header, which
  can lie after the image's own application segments: one to three
  blocks of 64 KiB on the Commons files. Nothing more is read for the
  metadata.
- A JPEG with more fill bytes before its scan than the walk's steps
  allow is refused, although the JPEG standard allows any number of
  them. None of the Commons files has a single one; the cap also bounds
  how much of the file `open` reads.
- The largest preview of a JPEG is the file itself, and reading it reads
  the whole file. Its bytes can run past the main image's end marker,
  through the MPF images and Sony's zeros, so a check that a preview ends
  with an end marker must allow what follows it.
- A JPEG larger than the limit on preview bytes, 128 MiB by default,
  opens, but its own preview cannot be read without raising the limit.
- An Exif segment whose TIFF header or IFD0 cannot be read costs the
  metadata, the thumbnail and the MakerNote's preview; the file and its
  MPF images remain. A single damaged value costs only itself.
- An export that keeps the shape of the camera's picture keeps its focus
  point, which scales with the picture. An export cropped to the same
  shape, or a square picture turned upright, cannot be told from the
  camera's own picture, and its point is in the wrong place.
- A Nikon JPEG shot in a crop mode gives a point when, after the crop
  rule of [ADR012](adr012-focus-points-as-the-camera-records-them.md),
  its frame fits the picture. What such a JPEG records is not checked on
  a public file.
- Whether a camera stores its thumbnail and MPF images already turned on
  a picture taken in portrait orientation, which would make IFD0's
  orientation wrong for them, is not checked on a public file: every
  Commons file has orientation 1.
- The Sony MakerNote gives a JPEG what it gives an ARW, the focus point
  alone; the A100's, whose tags are of an older layout, gives nothing.
  Canon and other makers' JPEGs give the standard metadata and previews,
  and no focus point.
- A JPEG has no raw data: the raw-data APIs, once they exist, report none
  for it.
- A format in a new container, such as CR3 or HEIF, adds that
  container's structure beside the TIFF and JPEG ones, its formats are
  asked of it in the same way, and it is accepted on the same ground:
  what raw-files can read from its files.
