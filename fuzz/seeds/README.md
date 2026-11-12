# Fuzz seeds

The files a fuzz target starts from. A fuzzer finds a valid file header only
by luck, and from an empty corpus it never builds a big-endian TIFF one, so
each target is given a few valid files to mutate. The seeds are committed;
the corpus the fuzzer writes is not. Where this page says *my own*, it is
me, Alvaro Kuolas, who maintains raw-files, as
[`tests/fixtures/README.md`](../../tests/fixtures/README.md) says.

- `tiff_walk/` seeds the `tiff_walk` target with the test fixtures of the
  TIFF reader, three layouts in both byte orders.
- `jpeg_probe/` seeds the `jpeg_probe` target with stub JPEG streams: a
  baseline, a progressive and a lossless frame header, application segments
  before the frame header, and a frame header straight after SOI, as in a
  Sony raw tile; and one input that is not a stream but two bytes choosing
  a range inside the input, then a stream, since the target also probes the
  range those bytes name.
- `jpeg_segments/` seeds the `jpeg_segments` target with the JPEG files of
  the JPEG reader's integration tests, built in code by
  [`tests/support/jpeg_files.rs`](../../tests/support/jpeg_files.rs):
  plain and Exif layouts, a camera's layout, and Nikon and Sony cameras'
  JPEGs whose Exif segment holds a MakerNote, all but the RX100 IV's and
  the ILCE-6000's. Each file with an Exif segment comes in both byte
  orders. Beside them are the four real JPEGs committed in
  [`tests/fixtures/`](../../tests/fixtures/README.md), copied as they
  are: my own Z 9's three, and frame 4 of the session on which my own
  D600's autofocus points were measured, named
  `nikon-d600-frame-04.jpg`.
- `open/` seeds the `open` target with the integration tests' fixtures of
  the RAW formats, built in code by
  [`tests/support/nef.rs`](../../tests/support/nef.rs) and
  [`tests/support/arw.rs`](../../tests/support/arw.rs): the NEF layouts
  of four Nikon bodies, and files of each generation of Sony's family,
  in both byte orders. Beside them are five real files committed in
  [`tests/fixtures/`](../../tests/fixtures/README.md), copied as they
  are: my own Z 9's two NEF files, and frames 4, 42 and 49 of the D600
  session, named `nikon-d600-frame-04.nef` and so on. The target also
  takes the `tiff_walk/` and `jpeg_segments/` seeds as further corpus
  directories, since a plain TIFF file and a JPEG file are also
  something it must open.

The files are generated from the fixtures in
`src/container/tiff/robustness.rs`, the stubs in
`src/container/jpeg/mod.rs` and the integration tests' fixtures, built in
code or committed in `tests/fixtures/`, through `tests/fuzz_seeds.rs`,
never edited by hand, and a test for each folder
(`the_fuzz_seeds_match_the_fixtures`, `the_jpeg_fuzz_seeds_match_the_stubs`,
`the_jpeg_segments_fuzz_seeds_match_the_fixtures` and
`the_open_fuzz_seeds_match_the_fixtures`) fails when a file is missing or
differs from what it is generated from.
After changing a fixture or a stub, regenerate them all from the repository
root:

```sh
cargo test write_fuzz_seeds -- --ignored
```

The fuzzer saves what it finds to `fuzz/corpus/`, which git ignores; see
[docs/TESTING.md](../../docs/TESTING.md#fuzzing) for how to run it.
