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
- The `open` target has no directory of its own: it starts from
  `tiff_walk/`, since every file it can open is a TIFF file. Fixtures of
  the RAW formats will join it as their readers arrive.

The files are generated from the fixtures in
`src/container/tiff/robustness.rs` and the stubs in
`src/container/jpeg/mod.rs`, never edited by hand, and a test in each
(`the_fuzz_seeds_match_the_fixtures`, `the_jpeg_fuzz_seeds_match_the_stubs`)
fails when a file is missing or differs from what it is generated from.
After changing a fixture or a stub, regenerate them all from the repository
root:

```sh
cargo test --lib write_fuzz_seeds -- --ignored
```

The fuzzer saves what it finds to `fuzz/corpus/`, which git ignores; see
[CONTRIBUTING.md](../../CONTRIBUTING.md#fuzzing) for how to run it.
