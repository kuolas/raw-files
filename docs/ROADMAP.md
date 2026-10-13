# Roadmap

The big picture: what is planned, and in what order. Bugs and smaller
requests live in the
[issue tracker](https://github.com/kuolas/raw-files/issues).

## Where we are

The repository, its license and its contribution process exist. The public
API is the file handle (`RawFile` and `Options`) with the preview types
(`Preview`, `PreviewFormat`, `Orientation`, `Size`), `Format`, `Error` and
`Limits`. A Nikon NEF file opens with the JPEG previews in its IFD0 and
SubIFDs, checked on a test file laid out like a Nikon Z 9's; the preview
in the MakerNote, and tests on real sample files, are still to come. Any
other file that can be read opens as `UnknownFormat`.

The first release, version 0.1.0, is milestones 1 and 2: the embedded
previews of NEF and ARW files.

## Milestones

1. **NEF previews.** Find the largest JPEG preview embedded in a Nikon NEF
   file and return its bytes, without decoding the raw data.
2. **ARW previews.** The same for Sony ARW files.
3. **Basic metadata.** What a viewer or a file renamer needs next to the
   preview: capture time, camera make, model and serial number, lens,
   exposure, orientation, and image sizes.
4. **Focus points.** The autofocus points the camera used, in the
   coordinates of the image, to judge sharpness where the photographer
   focused.
5. **Raw sensor data.** Decode the sensor data of NEF and ARW files.
6. **Developed image.** Develop the sensor data to a linear image, plus the
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
