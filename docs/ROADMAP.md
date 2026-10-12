# Roadmap

The big picture: what is planned, and in what order. Bugs and smaller
requests live in the
[issue tracker](https://github.com/kuolas/raw-files/issues).

## Where we are

The repository, its license and its contribution process exist. The crate
is a library with no public API; it does nothing yet.

## Milestones

1. **NEF previews.** Find the largest JPEG preview embedded in a Nikon NEF
   file and return its bytes, without decoding the raw data.
2. **ARW previews.** The same for Sony ARW files.
3. **Basic metadata.** What a viewer needs next to the preview: orientation,
   capture date, camera make and model, and image dimensions.
4. **Raw sensor data.** Decode the sensor data of NEF and ARW files.

The order is a plan, not a promise; a change of direction gets an ADR.

## Formats after NEF and ARW

Support for other formats comes in this order:

1. **DNG**, Adobe's published raw format, also written by phones.
2. **Canon**: CR2 and CR3.
3. **Fujifilm**: RAF.

## Open questions

- **How far decoding goes.** Raw sensor data only, leaving demosaicing and
  colour to the caller, or a developed RGB image too.
- **Which formats after Fujifilm.** Pentax PEF, Olympus ORF, Panasonic
  RW2, …, and in what order.

Ideas and camera support requests are welcome as
[feature requests](https://github.com/kuolas/raw-files/issues/new/choose).
