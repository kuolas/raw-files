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
- `open/` seeds the `open` target with the integration tests' fixtures of
  the RAW formats: files laid out like the NEF of a Nikon Z 9 and of a
  D300, and one like each generation of Sony's family (ARW 4.0 with a
  lossless and with a medium raw, ARW 2.x, ARQ, SR2, SRF and the A100), in
  both byte orders. The target still takes the `tiff_walk/` seeds as a
  second corpus directory, since a plain TIFF file is also something it
  must open.

The files are generated from the fixtures in
`src/container/tiff/robustness.rs`, the stubs in
`src/container/jpeg/mod.rs` and the integration tests' fixtures through
`tests/fuzz_seeds.rs`, never edited by hand, and a test in each
(`the_fuzz_seeds_match_the_fixtures`, `the_jpeg_fuzz_seeds_match_the_stubs`
and `the_open_fuzz_seeds_match_the_fixtures`) fails when a file is missing
or differs from what it is generated from.
After changing a fixture or a stub, regenerate them all from the repository
root:

```sh
cargo test write_fuzz_seeds -- --ignored
```

The fuzzer saves what it finds to `fuzz/corpus/`, which git ignores; see
[CONTRIBUTING.md](../../CONTRIBUTING.md#fuzzing) for how to run it.
