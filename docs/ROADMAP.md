# Roadmap

The big picture: what is planned, and in what order. Bugs and smaller
requests live in the
[issue tracker](https://github.com/kuolas/raw-files/issues).

## Where we are

The repository, its license and its contribution process exist, and
five releases are out. Where this roadmap says *my own* files, it is me,
Alvaro Kuolas, who maintains raw-files, writing about my own cameras:
they stay private. What raw-files reads, milestone by milestone:

- **Previews (milestones 1 and 2): done.** A Nikon NEF file opens with
  the JPEG previews in its IFD0, its SubIFDs and its MakerNote. A Sony
  ARW file, or an SRF, SR2 or ARQ file of the same family, opens with
  the JPEG previews in its IFD0, IFD1 and, on newer bodies, IFD2.
  Checked on test files laid out like a Nikon Z 9's, a Z6III's, a
  D300's and a D1H's and like each generation of Sony's family, on two
  stripped files of my own Z 9, and on the CC0 files of raw.pixls.us:
  35 Nikon files, three of the Z 9 (lossless, HE and HE\*) and 32 of
  25 other bodies from the D1H to the Z5II, and 36 Sony-family files,
  all of which CI downloads but the large ARQ, fetched on request.
- **Basic metadata (milestone 3): done, as far as plain tags record
  it.** Both formats open with their metadata, read from the standard
  TIFF and Exif tags and from the MakerNote, for the camera settings
  that only it records and, on Nikon files, for the fields that older
  bodies record only there, checked on the same files. Not read: the
  shutter used and the image area of Sony bodies, which they record
  only in enciphered blocks and which wait for milestone 6, and their
  raw image size, because a tag gives an APS-C raw the code of a
  medium one.
- **Focus points (milestone 4): the first step done, the second under
  way.** Both formats give where the camera focused, read from the
  MakerNote, on Nikon mirrorless bodies, Nikon DSLRs in live view and
  Sony bodies from the ILCE-7RM2 on. A shot that records only an
  autofocus point's number gives none, except on the Nikon D600, the
  first body whose table of points the project has measured: its shots
  through the viewfinder give their points placed from that table.
  Further bodies join as their tables are measured.
- **Plain JPEG files (milestone 5): done.** A JPEG file, as a camera
  writes it beside its raw file or instead of it, opens with the file
  itself, the thumbnail of its Exif segment, the other images of its
  Multi-Picture Format (MPF) list and, from the Nikon bodies that write
  one, its MakerNote's small JPEG as previews, and with the metadata of
  its Exif segment, read as in a raw file, with no raw size. A Nikon or
  Sony camera's JPEG also gives what its MakerNote adds, as a NEF or an
  ARW does, its focus point only when the point's frame has the
  picture's shape. Checked on test files built in code, on three
  stripped JPEG files of my own Z 9 and on the eight camera JPEGs of
  Wikimedia Commons that CI downloads, of five Sony bodies, two Nikon
  bodies and a Canon.
- **Full metadata (milestone 6), raw sensor data (milestone 7) and the
  developed image (milestone 8): not
  started.**

Any other file is refused with the `UnknownFormat` error.

The public API is the file handle (`RawFile` and `Options`) with the
preview types (`Preview`, `PreviewFormat`, `Orientation`, `Size`), the
metadata types (`Metadata`, `Exposure`, `Lens`, `LensBuilder`, `Focus`,
`Point`, `DateTime`, `Rational` and `SignedRational`), the camera
settings types (`CameraSettings`, `WhiteBalance`, `FineTune`, `Flash`,
and the enums `WhiteBalanceMode`, `ExposureProgram`, `MeteringMode`,
`FlashMode`, `Shutter`, `ImageArea`, `RawImageSize` and `ColorSpace`),
`Format`, `Error`, `ErrorKind`, `Result` and `Limits`.

### Releases

- **0.1.0**: milestones 1 and 2, the embedded previews of NEF and ARW
  files.
- **0.2.0**: most of milestone 3, their basic metadata.
- **0.3.0**: the first step of milestone 4, the focus point as the
  camera records it.
- **0.4.0**: milestone 5, plain JPEG files
  ([#23](https://github.com/kuolas/raw-files/issues/23)).
- **0.5.0**: milestone 3 completed, as far as plain tags record it, with
  the camera's settings
  ([#26](https://github.com/kuolas/raw-files/issues/26)).
- **Next**: the second step of milestone 4, a DSLR's autofocus point,
  which a shot through the viewfinder names by number, placed on the
  picture from a table of the body's points, with the points the camera
  used and the body's layout beside it
  ([#57](https://github.com/kuolas/raw-files/issues/57)). The tables are
  the project's own, measured on frames shot with the body and read in
  the maker's software, the Nikon D600's first, by a procedure that a
  contributor who owns another body can repeat; a body without a table
  gives none, and the format notes say so per body
  ([ADR017](architecture-decisions/adr017-per-body-tables-measured-by-the-project.md),
  [ADR018](architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)).
- **Then**: milestone 6, every value a file records, the ones that only
  enciphered or encrypted blocks hold among them
  ([#50](https://github.com/kuolas/raw-files/issues/50)).

## Milestones

1. **NEF previews.** Find the largest JPEG preview embedded in a Nikon NEF
   file and return its bytes, without decoding the raw data.
2. **ARW previews.** The same for Sony ARW files.
3. **Basic metadata.** What a viewer or a file renamer needs next to the
   preview: capture time, camera make, model and serial number, lens,
   exposure, orientation, image sizes, and how the camera was set (white
   balance, exposure program, metering, flash, shutter, image area).
4. **Focus points.** Where the camera focused, in coordinates that place
   it on a preview, to judge sharpness where the photographer focused.
   First the position the camera records; then the shots that record only
   the number of an autofocus point, such as a DSLR's through the
   viewfinder, mapped to pixels through a table of the body's points,
   measured by the project on that body
   ([ADR017](architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)).
5. **Plain JPEG files.** Open the JPEG files a camera writes beside its
   raw files, for their Exif metadata and their previews.
6. **Full metadata.** Every metadata value a file records, so that a
   program needs no second reader beside raw-files: more capture
   settings, how the camera processed its JPEG, the lens by name, the
   author and copyright, the location, the XMP and IPTC fields, and what
   a maker records only in enciphered or encrypted blocks, which needs a
   decision of its own first
   ([ADR016](architecture-decisions/adr016-full-metadata-before-raw-data.md)).
7. **Raw sensor data.** Decode the sensor data of NEF and ARW files.
8. **Developed image.** Develop the sensor data to a linear image, plus the
   colour model that converts it to a standard colour space; the creative
   edit stays with the application
   ([ADR010](architecture-decisions/adr010-decoding-ends-at-a-linear-image.md)).

The order is a plan, not a promise; a change of direction gets an ADR.

## Formats after NEF and ARW

Support for other formats comes in this order:

1. **DNG**, Adobe's published raw format, also written by phones.
2. **Canon**: CR2 and CR3.
3. **Fujifilm**: RAF.

## Open questions

- **Which formats after Fujifilm.** Pentax PEF, Olympus ORF, Panasonic
  RW2, …, and in what order.

Ideas and camera support requests are welcome as
[feature requests](https://github.com/kuolas/raw-files/issues/new/choose).
