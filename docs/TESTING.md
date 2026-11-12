# Testing

This page says how to run the tests that need more than `cargo test`.
They are the sample files, the ignored tests, the local corpus test,
the check for breaking changes and fuzzing. The page also says what CI
runs.
[CONTRIBUTING.md](../CONTRIBUTING.md#code-style-and-checks) has the six
commands to run before a pull request, and the rules a change follows.

## The sample files

Some tests read real camera files, the sample files, which are too
large to commit. [The sample files page](formats/sample-files.md) says
what they are for and how a sample comes in. They are listed in
[`tests/samples.tsv`](../tests/samples.tsv): RAW files from
raw.pixls.us and camera JPEGs from Wikimedia Commons, as its source
column says.

`scripts/fetch-samples.sh` downloads the pinned set, about 1.6 GB, and
checks each file's SHA-256. It saves them into `raw-samples/` at the
repository's root, a folder that git ignores and that `cargo clean`
leaves alone. `cargo test` then runs those tests too. When the files are
absent, those tests print a note and pass, so the suite runs offline.
`scripts/fetch-samples.sh nef` (or `arw`, or `jpeg`) fetches the files
of one format only.

Three environment variables change what the script and the tests do:

| Variable | What it does |
|---|---|
| `RAW_FILES_SAMPLES` | Names the directory the files are saved into and read from, in place of `raw-samples/`. |
| `RAW_FILES_LARGE` | Set to `1`, the script also fetches the large tier: one file, the 325 MiB ARQ. |
| `RAW_FILES_REQUIRE_SAMPLES` | Set to `1`, a missing sample file fails its test instead of skipping. A file of the large tier fails only when `RAW_FILES_LARGE=1` is set too. |

## The ignored tests

Four tests are ignored by default: `cargo test` reports them as ignored
and runs none of them.

| Test | What it does | How to run it |
|---|---|---|
| `every_file_of_the_local_corpus_opens_and_gives_a_whole_jpeg`, in `tests/local_corpus.rs` | Runs the reader over a folder of your own camera files | As [the local corpus test](#the-local-corpus-test) says |
| `write_fuzz_seeds`, in `src/container/tiff/robustness.rs` | Writes the seeds of the `tiff_walk` fuzz target again | `cargo test write_fuzz_seeds -- --ignored` |
| `write_fuzz_seeds`, in `src/container/jpeg/mod.rs` | Writes the seeds of the `jpeg_probe` fuzz target again | The same command |
| `write_fuzz_seeds`, in `tests/fuzz_seeds.rs` | Writes the seeds of the `open` and `jpeg_segments` fuzz targets again | The same command |

The three seed writers run after a fixture or a stub changes; the
command, from the repository root, runs all three.
[`fuzz/seeds/README.md`](../fuzz/seeds/README.md) says what the seeds
are.

## The local corpus test

One test runs the reader over a folder of your own camera files, RAW
and JPEG. It shows whether raw-files copes with cameras and settings
that the tests' own files do not cover. It only reads the folder.

The test is ignored by default. Run it in release mode, because it
reads every file:

```sh
RAW_FILES_LOCAL_CORPUS=/path/to/photos cargo test --release --test local_corpus -- --ignored --nocapture
```

It reads the folder and its subfolders, and follows no symbolic link.
Without `RAW_FILES_LOCAL_CORPUS` it checks nothing and passes. To see
what it prints before you point it at your own photos, run it on
`tests/fixtures`.

### What it prints

The test prints totals: first over the folder, then one section for
each format the folder holds. A total is tied to no file. The output
never holds:

- a path or a file name;
- a text as a file records it: a make, a model, a software or a lens's
  name;
- any of the texts on who made a picture, or the camera's owner, since
  they name people;
- a serial number, a capture time, an exposure or a lens's range;
- a coordinate, an altitude, a GPS time or a map datum, since they name
  places;
- where a focus point lies, how large its area is, or its number;
- an entry's value, or its tag, but where a directory is named by one,
  such as `MakerNote 0x0011`.

So the whole output can go into an issue. It does show, as totals,
which makers' cameras the folder holds, the sizes of their pictures and
the settings they were shot with. The lowest and the highest color
temperature are each one file's value. In a section of one file, every
total is that file's own.

### When it fails

The test stops at once when the folder cannot be read. Otherwise it
prints every total first, then fails when:

- something inside the folder cannot be listed;
- the folder holds no file of a format raw-files reads;
- a file does not open, or has no preview;
- a file's largest preview is not a whole JPEG;
- a value of a file's entries cannot be read;
- the metadata or the focus point of a file breaks one of the checks.

A failure says which check failed, and never names a file. The module
docs of [`tests/local_corpus.rs`](../tests/local_corpus.rs) say what
each check holds a file to, and what the test counts.

If it fails on your files, please open an issue with the output and the
camera's model.

## The check for breaking changes

CI compares the crate's public API with the newest version on
crates.io, with `cargo-semver-checks`. So a change that would break
callers of the last release is caught before it is merged. It catches
many breaking changes, such as a removed or renamed item, but not all.
A changed type or behavior is still for review to find. Adding to the
API needs no new version to pass.
[CONTRIBUTING.md](../CONTRIBUTING.md#code-style-and-checks) says when a
breaking change may be made.

To run it yourself, install it once, then run it in the repository
root:

```sh
cargo install cargo-semver-checks --locked
cargo semver-checks
```

The first run downloads the published version from crates.io. The
check is not one of the six commands, because it needs that install
and the network.

## Fuzzing

The readers take files from anyone, so they must answer any input with
a result: never a panic, and never unbounded memory. Fuzzing tests that
on inputs nobody would write by hand.

The fuzz targets live in [`fuzz/`](../fuzz/), a separate crate that is
not part of the published package. They need nightly Rust and
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz):

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
```

There are four targets:

| Target | What it reads | Its seeds |
|---|---|---|
| `tiff_walk` | The input as a TIFF file: the header, the walk, and every value of every IFD found | `fuzz/seeds/tiff_walk` |
| `jpeg_probe` | The search for a JPEG preview's frame header | `fuzz/seeds/jpeg_probe` |
| `jpeg_segments` | The input as a JPEG file: the walk over its segments, the TIFF structure of its Exif segment with every value in it, and the index of its MPF segment | `fuzz/seeds/jpeg_segments` |
| `open` | The input opened as a caller would: its metadata, its focus point placed on a preview, every preview, and every entry of its directories with its value | `fuzz/seeds/open`, `fuzz/seeds/tiff_walk` and `fuzz/seeds/jpeg_segments` |

The seeds are a few valid files, committed in
[`fuzz/seeds/`](../fuzz/seeds/README.md), which says what each folder
holds. Without them the fuzzer never builds a big-endian header, and
half of the reader goes unfuzzed. The `open` target takes three
folders, since a plain TIFF file and a JPEG file are also something it
must open.

To fuzz the TIFF reader for a minute, from the repository root:

```sh
mkdir -p fuzz/corpus/tiff_walk
cargo +nightly fuzz run tiff_walk fuzz/corpus/tiff_walk fuzz/seeds/tiff_walk -- -max_total_time=60 -timeout=25 -malloc_limit_mb=256
```

The first directory is where the fuzzer saves what it finds. It is not
committed, and libFuzzer refuses to start when it does not exist, hence
the `mkdir`. The directories after it hold the seeds it starts from.
The target `jpeg_probe` runs the same way, with `jpeg_probe` in place
of `tiff_walk` throughout. The other two:

```sh
mkdir -p fuzz/corpus/jpeg_segments
cargo +nightly fuzz run jpeg_segments fuzz/corpus/jpeg_segments fuzz/seeds/jpeg_segments -- -max_total_time=60 -timeout=25 -malloc_limit_mb=256
```

```sh
mkdir -p fuzz/corpus/open
cargo +nightly fuzz run open fuzz/corpus/open fuzz/seeds/open fuzz/seeds/tiff_walk fuzz/seeds/jpeg_segments -- -max_total_time=60 -timeout=25 -malloc_limit_mb=256
```

The allocation limit makes a runaway allocation a finding: the run
stops on any one allocation over 256 MiB. The run also stops once the
whole process passes 2 GiB, libFuzzer's own limit. A lower limit trips
on the sanitizer's own bookkeeping, not on the reader. The timeout makes
a hang a finding too. By default, libFuzzer waits 1200 seconds for one
input, longer than the CI job lasts. A parse takes milliseconds, so 25
seconds is a hang.

An input that fails is saved in `fuzz/artifacts/`, and a hang as a
`timeout-*` file there. A fix for it comes with a regression test that
reads that input, as for any other bug.

## What CI runs

CI runs three workflows, in
[`.github/workflows/`](../.github/workflows/). On every pull request,
`ci.yml` runs four jobs:

- **commits and hooks**: every commit message against the `commit-msg`
  hook, no merge commits in the pull request, `scripts/test-hooks.sh`,
  and every hook and script committed executable.
- **cargo**, on macOS, Linux and Windows: the six commands of
  [CONTRIBUTING.md](../CONTRIBUTING.md#code-style-and-checks), in the
  same order. On Linux, it fetches the sample files before the tests,
  but not the large tier. There it runs the tests with
  `RAW_FILES_REQUIRE_SAMPLES=1`: a wrong path or a misplaced cache
  cannot pass the sample tests without reading a file. On macOS and
  Windows the sample tests skip. No other build compiles the fuzz code, so on Linux the
  job also checks that both parts of it compile, on stable Rust:
  - the library's entry points, with
    `RUSTFLAGS="--cfg fuzzing" cargo clippy --workspace --all-targets -- -D warnings`;
  - the fuzz crate, with
    `RUSTFLAGS="--cfg fuzzing" cargo check --manifest-path fuzz/Cargo.toml`.
- **cargo (msrv)**: builds and tests the crate with Rust 1.95, the
  oldest version it supports.
- **cargo (semver)**: [the check for breaking
  changes](#the-check-for-breaking-changes).

`samples.yml` keeps the sample files in a cache that every pull
request can restore. So a pull request downloads nothing unless it
changes the list.

`fuzz.yml` runs the four fuzz targets for ten minutes each, twice a
week and on request, started by hand from the Actions tab. It runs the
commands above, with `-max_total_time=600`. It keeps the corpus each
run found in the Actions cache for the next run. It uploads any failing
input as a run artifact for thirty days. It is not a required
check, so a finding there is a bug to fix, not a blocked pull request.
