# Roadmap

The big picture: what raw-files aims for, what it reads so far, and
what comes next. Bugs and smaller requests live in the
[issue tracker](https://github.com/kuolas/raw-files/issues).

Where this roadmap says *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files, writing about my own cameras: they stay private.

## The goal

raw-files aims for a well-made API: ergonomic, idiomatic Rust that
follows the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and
that a person can read and understand. It strives for an API that is
complete, not for support of every camera body: cameras of different
makers record much the same things, so one API can serve them all
without every body being read first.

The work aims first at mirrorless cameras, such as Nikon Z, Sony
E-mount Alpha and Canon EOS R bodies. Bodies are added as the
opportunity comes, when I have one at hand or a contributor sends
files of it; no camera's support is promised, and none is scheduled.

## Capabilities

What raw-files gives is grouped in five capabilities: previews,
metadata, focus points, raw sensor data and the developed image.
Together they are the whole of what it means to give: decoding ends at
the developed image, and the creative edit stays with the application
([ADR010](architecture-decisions/adr010-decoding-ends-at-a-linear-image.md)).
A capability is a lasting area of work, not a step towards a finish:
each grows with every body and format it learns to read, so none is
ever complete for every body. What each gives so far:

- **Previews.** The JPEG previews a file holds, returned without
  decoding the raw data. A Nikon NEF file gives the JPEG previews in
  its IFD0, its SubIFDs and its MakerNote. A Sony ARW file, or an SRF,
  SR2 or ARQ file of the same family, gives the JPEG previews in its
  IFD0, IFD1 and, on newer bodies, IFD2. A JPEG file gives the file
  itself, the thumbnail of its Exif segment, the other images of its
  Multi-Picture Format (MPF) list and, from the Nikon bodies that
  write one, its MakerNote's small JPEG.
- **Metadata.** What a viewer or a file renamer needs next to the
  preview: capture time, camera make, model and serial number, lens,
  exposure, orientation, image sizes, and how the camera was set (white
  balance, exposure program, metering, flash, shutter, image area).
  Read from the standard TIFF and Exif tags and from the MakerNote, for
  the camera settings that only it records and, on Nikon files, for the
  fields that older bodies record only there. A JPEG file's metadata
  is read from its Exif segment as in a raw file, with no raw size,
  and a Nikon or Sony camera's JPEG also gives what its MakerNote adds,
  as a NEF or an ARW does. Not read: the shutter used and the image
  area of Sony bodies, which they record only in enciphered blocks, and
  their raw image size, because a tag gives an APS-C raw the code of a
  medium one.

  Open, and next: **full metadata**, every metadata value a file
  records, so that a program needs no second reader beside raw-files:
  more capture settings, how the camera processed its JPEG, the lens by
  name, the author and copyright, the location, the XMP and IPTC
  fields, what editing software, such as Nikon's, writes into the
  file, and what a maker records only in enciphered or encrypted
  blocks, which needs a decision of its own first. It comes before raw
  sensor data
  ([ADR016](architecture-decisions/adr016-full-metadata-before-raw-data.md)).
- **Focus points.** Where the camera focused, in coordinates that place
  it on a preview, to judge sharpness where the photographer focused.
  Read from the MakerNote, on Nikon mirrorless bodies, Nikon DSLRs in
  live view and Sony bodies from the ILCE-7RM2 on, with the points the
  camera used and, on Nikon files, the area mode; a Nikon or Sony
  camera's JPEG gives its focus point only when the point's frame has
  the picture's shape. A shot that records only the number of an
  autofocus point, such as a DSLR's through the viewfinder, needs a
  table of the body's points, measured by the project on that body
  ([ADR017](architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)).
  The Nikon D600 is the first body measured: its shots through the
  viewfinder give their points placed from that table, and its files
  the layout of its 39 points. Checked on the 56 frames, stripped of
  their pixels, on which I measured my own D600's points. A shot of
  another body that records only a number gives no point; a body's
  table joins when I have the body at hand, or when a contributor
  sends RAW + JPEG frames of it to measure.
- **Raw sensor data.** The sensor data of a raw file, decoded. Not
  started.
- **Developed image.** The sensor data developed to a linear image,
  plus the colour model that converts it to a standard colour space.
  Not started.

The public API is the file handle (`RawFile` and `Options`) with the
preview types (`Preview`, `PreviewFormat`, `Orientation`, `Size`), the
metadata types (`Metadata`, `Exposure`, `Lens`, `LensBuilder`, `Point`,
`DateTime`, `Rational` and `SignedRational`), the focus types (`Focus`,
`FocusOrigin`, `FocusPoints`, `FocusPoint`, `FocusAreaMode`,
`FocusLayout` and `FocusLayoutPoint`), the camera
settings types (`CameraSettings`, `WhiteBalance`, `FineTune`, `Flash`,
and the enums `WhiteBalanceMode`, `ExposureProgram`, `MeteringMode`,
`FlashMode`, `Shutter`, `ImageArea`, `RawImageSize` and `ColorSpace`),
`Format`, `Error`, `ErrorKind`, `Result` and `Limits`.

## Formats

raw-files reads three formats, and refuses any other file with the
`UnknownFormat` error:

- **NEF**, Nikon's raw format. Checked on test files laid out like a
  Nikon Z 9's, a Z6III's, a D300's and a D1H's, on two stripped files
  of my own Z 9, and on 35 of the CC0 files of raw.pixls.us: three of
  the Z 9 (lossless, HE and HE\*) and 32 of 25 other bodies from the
  D1H to the Z5II.
- **ARW**, Sony's raw format, with the SRF, SR2 and ARQ files of the
  same family. Checked on test files laid out like each generation of
  Sony's family, and on 36 of the CC0 files of raw.pixls.us. CI
  downloads these and the Nikon ones, all but the large ARQ, which is
  fetched on request.
- **JPEG**, as a camera writes it beside its raw file or instead of it
  ([ADR014](architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
  Checked on test files built in code, on three stripped JPEG files of
  my own Z 9 and on the eight camera JPEGs of Wikimedia Commons that CI
  downloads, of five Sony bodies, two Nikon bodies and a Canon.

Other formats come in this order:

1. **DNG**, Adobe's published raw format, also written by phones.
2. **Canon**: CR3, then CR2.
3. **Fujifilm**: RAF.

Here too the work aims first at mirrorless bodies, so Canon's CR3,
the format of its EOS R bodies, comes before CR2. DNG comes first: CR3
is the more complex format to read.
The order is a plan, not a promise; a change of direction gets an ADR.

## Releases

- **0.1.0**: previews of NEF and ARW files.
- **0.2.0**: their metadata, most of what a viewer or a file renamer
  needs.
- **0.3.0**: focus points, the position the camera records.
- **0.4.0**: plain JPEG files, as a third format, with their previews,
  metadata and focus point
  ([#23](https://github.com/kuolas/raw-files/issues/23)).
- **0.5.0**: metadata, the camera's settings, read from the plain tags
  ([#26](https://github.com/kuolas/raw-files/issues/26)).
- **0.6.0**: focus points, a DSLR's autofocus point, which a shot
  through the viewfinder names by number, placed on the picture from a
  table of the body's points, with the points the camera used and the
  body's layout beside it
  ([#57](https://github.com/kuolas/raw-files/issues/57)). The tables are
  the project's own, measured on frames shot with the body and read in
  the maker's software, the Nikon D600's first, by a procedure that a
  contributor who owns another body can repeat; a body without a table
  gives none, and the format notes say so per body
  ([ADR017](architecture-decisions/adr017-per-body-tables-measured-by-the-project.md),
  [ADR018](architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)).
- **Next**: full metadata, every value a file records, the ones that
  only enciphered or encrypted blocks hold among them
  ([#50](https://github.com/kuolas/raw-files/issues/50)).

## Open questions

- **Which formats after Fujifilm.** Pentax PEF, Olympus ORF, Panasonic
  RW2, …, and in what order.

Ideas and camera support requests are welcome as
[feature requests](https://github.com/kuolas/raw-files/issues/new/choose).
