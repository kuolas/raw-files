# Fuzz seeds

The files a fuzz target starts from. A fuzzer finds a valid file header only
by luck, and from an empty corpus it never builds a big-endian TIFF one, so
each target is given a few valid files to mutate. The seeds are committed;
the corpus the fuzzer writes is not.

- `tiff_walk/` seeds the `tiff_walk` target with the test fixtures of the
  TIFF reader, three layouts in both byte orders.

The files are generated from the fixtures in
`src/container/tiff/robustness.rs`, never edited by hand, and a test
(`the_fuzz_seeds_match_the_fixtures`) fails when one is missing or differs
from its fixture. After changing a fixture, regenerate them from the
repository root:

```sh
cargo test --lib write_fuzz_seeds -- --ignored
```

The fuzzer saves what it finds to `fuzz/corpus/`, which git ignores; see
[CONTRIBUTING.md](../../CONTRIBUTING.md#fuzzing) for how to run it.
