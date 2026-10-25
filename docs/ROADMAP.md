# Roadmap

The big picture: what is planned, and in what order. Bugs and smaller
requests live in the
[issue tracker](https://github.com/kuolas/raw-files/issues).

## Where we are

The repository, its license and its contribution process exist. The public
API is the file handle (`RawFile` and `Options`) with the preview types
(`Preview`, `PreviewFormat`, `Orientation`, `Size`), the metadata types
(`Metadata`, `Exposure`, `Lens`, `LensBuilder`, `Focus`, `Point`,
`DateTime`, `Rational` and `SignedRational`), `Format`, `Error`,
`ErrorKind`, `Result` and `Limits`. A Nikon NEF file opens with the JPEG
previews in its IFD0, its SubIFDs and its MakerNote, checked on test files
laid out like a Nikon Z 9's, a Z6III's, a D300's and a D1H's, on two
stripped files of the maintainer's own Z 9, and on the 35 CC0 Nikon files of
raw.pixls.us that CI downloads: three of the Z 9 (lossless, HE and HE*),
and 32 of 25 other bodies, from the D1H to the Z5II. A Sony ARW file, or
an SRF, SR2 or ARQ file of the same family, opens with the JPEG previews in
its IFD0, IFD1 and, on newer bodies, IFD2, checked on test files laid out
like each generation of the family and on 36 CC0 Sony-family files of the
same site, 35 of which CI downloads. Both open with their metadata too,
read from the standard TIFF and Exif tags and, on Nikon files, from the
MakerNote fields that older bodies record only there, checked on the same
files. Both give where the camera focused, read from the MakerNote, on
Nikon mirrorless bodies, Nikon DSLRs in live view and Sony bodies from the
ILCE-7RM2 on, while a shot that records only an autofocus point's number
gives none until the second step of milestone 4. A JPEG file, as a camera
writes it beside its raw file or instead of it, opens with the file itself,
the thumbnail of its Exif segment, the other images of its Multi-Picture
Format (MPF) list and, from the Nikon bodies that write one, its
MakerNote's small JPEG as previews, and with the metadata of its Exif
segment, read as in a raw file, with no raw size. A Nikon camera's JPEG
also gives what its MakerNote adds, as a NEF does, and a Nikon or Sony
camera's JPEG gives the focus point, but only when its frame has the
picture's shape. JPEG files are checked on test files built in code, on
three stripped JPEG files of the maintainer's own Z 9 and on the eight
camera JPEGs of Wikimedia Commons that CI downloads, of five Sony bodies,
two Nikon bodies and a Canon. Milestones 1, 2 and 5 all work;
milestone 3 works except for the camera's settings that only Sony's
MakerNote records, and milestone 4 works for the bodies that record a
position. Any other file is refused with the `UnknownFormat` error.

The first release, version 0.1.0, is milestones 1 and 2: the embedded
previews of NEF and ARW files. The second, 0.2.0, is most of milestone 3:
their basic metadata. The third, 0.3.0, is the first step of milestone 4: the
focus point as the camera records it. The fourth, 0.4.0, is milestone 5,
plain JPEG files ([#23](https://github.com/kuolas/raw-files/issues/23)).
The next release completes milestone 3 with the camera's settings
([#26](https://github.com/kuolas/raw-files/issues/26)), and the one
after it is the second step of milestone 4: the mapping of a DSLR's
autofocus point number to pixels.

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
   viewfinder, mapped to pixels through a table of the body's points.
5. **Plain JPEG files.** Open the JPEG files a camera writes beside its
   raw files, for their Exif metadata and their previews.
6. **Raw sensor data.** Decode the sensor data of NEF and ARW files.
7. **Developed image.** Develop the sensor data to a linear image, plus the
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
