# Fuzz seeds

The files a fuzz target starts from. A fuzzer finds a valid file header only
by luck, and from an empty corpus it never builds a big-endian TIFF one, so
each target is given a few valid files to mutate. The seeds are committed;
the corpus the fuzzer writes is not.

- `tiff_walk/` seeds the `tiff_walk` target with the test fixtures of the
  TIFF reader, three layouts in both byte orders.
- `jpeg_probe/` seeds the `jpeg_probe` target with stub JPEG streams: a
  baseline, a progressive and a lossless frame header, application segments
  before the frame header, and a frame header straight after SOI, as in a
  Sony raw tile; and one input that is not a stream but two bytes choosing
  a range inside the input, then a stream, since the target also probes the
  range those bytes name.
- `jpeg_segments/` seeds the `jpeg_segments` target with the JPEG files of
  the JPEG reader's integration tests: one with no Exif segment; one whose
  Exif segment holds IFD0 and IFD1's thumbnail; a camera's, whose Exif
  segment holds every value the metadata reads, IFD0's and the Exif IFD's,
  and whose MPF index lists two images after the main one; a camera's
  with a JFIF segment first, whose Exif segment holds only IFD0's
  orientation and IFD1's thumbnail, beside the same MPF index; and one with
  zeros after its end. Each file with an Exif segment comes in both byte
  orders, which the TIFF structure inside that segment and the camera's
  MPF index share.
- `open/` seeds the `open` target with the integration tests' fixtures of
  the RAW formats: files laid out like the NEF of a Nikon Z 9, of a Z6III
  on a 1:1 crop and of a D300, and one like each generation of Sony's
  family (ARW 4.0 with a lossless and with a medium raw, ARW 2.x, ARQ, SR2,
  SRF and the A100), in both byte orders; and, beside these fixtures built
  in code, the two real files committed in
  [`tests/fixtures/`](../../tests/fixtures/README.md), shots of the
  maintainer's own Z 9 in DX and 1:1 crop modes with their pixels and
  personal fields removed, copied as they are. The target also takes the
  `tiff_walk/` and `jpeg_segments/` seeds as further corpus directories,
  since a plain TIFF file and a JPEG file are also something it must open.

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
[CONTRIBUTING.md](../../CONTRIBUTING.md#fuzzing) for how to run it.
