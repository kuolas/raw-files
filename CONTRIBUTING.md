# Contributing to raw-files

Thanks for your interest! raw-files is a camera RAW library written in pure
Rust. It is at a very early stage, so ideas, bug reports, sample files and
small fixes are all welcome.

This page is the one place the project's rules live. It is short on purpose;
if something here is unclear, that is a bug in this page, and an issue about it
is a fine first contribution.

Everyone taking part is expected to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Getting started

You need:

- a stable Rust toolchain, **Rust 1.95 or later**, with the `rustfmt` and
  `clippy` components (`rustup component add rustfmt clippy`);
- `git`. The GitHub CLI (`gh`) is handy but optional.

Then:

```sh
git clone https://github.com/kuolas/raw-files.git
cd raw-files
cargo build
cargo test
```

To run a single test, or every test whose name contains a word:
`cargo test <name>`.

## Reporting bugs and requesting features

The tracker is [GitHub Issues](https://github.com/kuolas/raw-files/issues).
It is the project's one list of what is broken, deferred or wanted.

- **A bug** — use the *Bug report* form: what happened, how to reproduce it,
  and what you expected instead. Most bugs in a RAW library are about one
  camera's files, so name the camera model, and link a sample file if you can
  share one.
- **A feature** — use the *Feature request* form, and start from the problem
  rather than the solution. A request for a camera or format is input to the
  order in [docs/ROADMAP.md](docs/ROADMAP.md) rather than a place in a queue.
- **Anything else** (a docs fix, a question) — a blank issue is fine.

For anything bigger than a small fix, please open or find an issue **before**
you start, so we can agree on the approach before you spend time on it.

### Labels

The maintainer labels each new issue, so there is nothing for you to pick
when you open one. They sort the tracker by what an issue is, where it
belongs, what kind of work it needs and whether it can start; an issue
often has several.

What it is:

- `bug` — something doesn't work, or gives a wrong result. The *Bug report*
  form adds it.
- `enhancement` — a new feature or a change in behaviour. The *Feature
  request* form adds it.

Where it belongs, by the format or part of the library it touches:

- `nef` — Nikon NEF and NRW files.
- `arw` — Sony ARW files.
- `jpeg` — camera JPEG files and the JPEG container, with its Exif and MPF
  segments.
- `tiff` — the TIFF container the RAW formats are built on.
- `makernote` — the MakerNote blocks, where each camera maker records data
  of its own.
- `metadata` — capture data and camera settings, read through the
  `Metadata` API.
- `focus` — focus points, and the layouts of a camera's autofocus points.
- `preview` — the previews embedded in a file, and choosing among them.
- `ci` — the CI workflows, the git hooks and the scripts in `scripts/`.
- `docs` — documentation: `docs/`, the READMEs and the API docs.

What kind of work it is:

- `security` — hardening the readers against hostile files, or a
  vulnerability once it has been disclosed. Report a vulnerability
  privately first, as [Security issues](#security-issues) says.
- `performance` — speed, memory use or allocations.
- `testing` — tests, fixtures, sample files and fuzzing.
- `accessibility` — a barrier for people with disabilities, for example in
  the documentation.
- `camera-support` — a camera body or file variant the readers do not
  handle yet.

Whether it can start:

- `needs-samples` — waits on sample files or test shots from a camera. If
  you own that camera, this is where you can help most: the issue says
  which files or shots it needs.
- `needs-decision` — waits on a decision about how the library should
  behave. Comments on the issue help make it.
- `breaking-change` — changes the public API, so it waits for a release
  that breaks the API anyway.

Where to start:

- `good first issue` — small and well defined, a good way into the
  project.

After an issue is opened:

- `question` — work waits on more information, usually from whoever
  opened the issue. Despite its name, it does not mark an issue that asks
  a question.
- `help wanted` — the maintainer would welcome someone taking it on,
  though it may be bigger than a good first issue.
- `duplicate` — another issue already covers it; it is closed with a link
  to that one.
- `invalid` — not a problem in raw-files; it is closed with the reason.
- `wontfix` — decided against; it is closed, and the issue says why.

## Security issues

Please **don't** report security problems in a public issue. See
[SECURITY.md](SECURITY.md) for how to report them privately.

## Making a change

Every change reaches `master` through a pull request — including the
maintainer's own. The flow:

1. **Fork** the repository (or, if you have write access, work in it
   directly) and create a branch from an up-to-date `master`:

   ```sh
   git fetch origin
   git switch -c wip/short-description origin/master
   ```

   Branch names follow GNOME's convention: `wip/short-description` or
   `wip/yournick/short-description`, lower-case, using digits and `. _ -`.
   The `wip/` prefix says the branch's history may still be rewritten — which
   it will be, when you rebase it. If you work from a fork, branch from
   `upstream/master` instead (`git remote add upstream
   https://github.com/kuolas/raw-files.git` once).

2. **Commit in small, atomic steps.** One logical change per commit, and
   **every commit must build and pass the tests on its own** — not just the
   last one. This is what keeps `git bisect` useful, and it is the one rule
   that cannot be fixed after the fact. A few habits that make it easy:

   - Keep refactoring and behaviour changes in separate commits.
   - Put tests and docs in the same commit as the change that needs them.
   - Fix a mistake from an earlier commit with `git commit --fixup=<sha>` and
     fold it in with `git rebase -i --autosquash origin/master`, rather than
     adding a "fix the previous commit" commit.

3. **Run the checks** (see [Code style and checks](#code-style-and-checks)).

4. **Rebase onto the latest `master`** — never merge `master` into your
   branch:

   ```sh
   git fetch origin
   git rebase origin/master
   git push --force-with-lease   # only your own branch, never master
   ```

5. **Open a pull request.** Describe what changed and why, and link the issue
   it answers. The template has a short checklist. Give the PR a title in the
   same shape as a commit subject (`nef: Read the preview from the SubIFDs`).

### Linear history: why rebase, not merge

`master` has a **linear history**: no merge commits and no squashing. Pull
requests are landed with GitHub's *Rebase and merge*, so every commit you
wrote becomes a commit on `master`. That is why each one has to build and pass
on its own, and why we ask you to rebase rather than merge. The payoff is a
history that reads as a sequence of small, reviewable steps, reverts that touch
exactly one idea, and `git bisect` runs that stop on the commit that actually
caused a problem.

One consequence worth knowing: GitHub's rebase-merge always writes new commits
with new hashes. When you refer to a commit, use its hash **on `master`**, not
the one from your branch.

### Landing a pull request (maintainers)

Before merging, with the branch rebased onto the current `origin/master`:

```sh
scripts/check-series.sh              # every commit builds and passes tests
gh pr merge --rebase --delete-branch
```

`scripts/check-series.sh` builds and tests each commit of the branch in a
throwaway worktree, so your checkout is untouched. It refuses a branch that is
not on top of the base or that contains a merge commit. `--quick` builds
without testing; commits that touch only docs are skipped unless you pass
`--all`. For a fork, pass the base explicitly: `scripts/check-series.sh
upstream/master`. If `master` moves before the merge, rebase and run it again.

## Commit messages

We follow the
[GNOME commit message guidelines](https://handbook.gnome.org/development/commit-messages.html):

```
nef: Find the full-size preview in the SubIFDs

IFD0 only holds a small thumbnail; the full-size JPEG preview sits in
a SubIFD. Walk the SubIFDs and return the largest JPEG, which is what
a viewer means by "the preview".

Closes: #12
```

- **Subject line:** `component: Summary`.
  - The component tag is optional and a single word naming the part of the
    project the commit touches (`tiff`, `nef`, `arw`, `docs`, `build`,
    `ci`).
  - The summary is in the imperative ("Add", not "Added"), **starts with a
    capital letter** (or with an identifier written as it appears in code,
    ``tiff: `read_ifd()` rejects offsets past the end``), has **no trailing
    period**, and stays within about **72 characters**.
- **A blank second line**, always.
- **Body:** plain prose wrapped at 75 columns, explaining *why* the change is
  needed — the diff already shows *how*. If the commit fixes a bug introduced
  by an earlier commit, name it in the body as `64aee44 ("tiff: Read IFDs
  lazily")`, using the hash on `master`.
- **Issue references** go in the last paragraph, one per line:
  - `Closes: #12` or `Fixes: #12` closes the issue when the commit reaches
    `master` — put it on the commit that actually completes the work;
  - `Issue: #12` refers to an issue without closing it, as each commit of a
    series toward one issue does.

  The value is `#12`, `owner/repo#12` or a full URL.
- **Commits are authored by their human author.** Disclose significant AI help
  with an `Assisted-by:` trailer (see [AI-assisted contributions](#ai-assisted-contributions));
  don't name a tool in `Co-Authored-By:` or add a "Generated with …" footer.
  `Signed-off-by:` is not needed.

More examples:

```
arw: Reject a preview that runs past the end of the file
build: Pin the Rust toolchain for CI
Add a README
```

Git's own subjects (`Revert "…"`, `fixup! …`, `squash! …`) are accepted as
they are; for a revert, add the reason to the body.

## AI-assisted contributions

You may use AI tools. We follow the
[Fedora AI-Assisted Contributions Policy](https://docs.fedoraproject.org/en-US/council/policy/ai-contribution-policy/);
the full text as it applies here is in
[ADR006](docs/architecture-decisions/adr006-ai-assisted-contributions.md). In
short:

- **You are accountable.** You are always the author, and you vouch for the
  quality, license compliance (MIT OR Apache-2.0) and usefulness of
  everything you submit, however it was produced.
- **Disclose.** You must disclose AI use when a significant part of the
  contribution is taken from a tool without changes, and should disclose
  other uses where it might be useful. In commits, use an `Assisted-by:`
  trailer; in issues and comments, a short note:

  ```
  nef: Find the full-size preview in the SubIFDs

  …

  Assisted-by: Claude
  ```

  Help with grammar, spelling or clarity needs no disclosure.
- **A human decides.** AI may assist reviewers, but never as the sole or
  final judge of a contribution, nor to evaluate a person's standing in the
  community.
- **Large-scale initiatives** — bots or agents opening pull requests, mass
  changes — need an issue and the maintainers' agreement first.

Concerns about a possible violation go privately to the maintainer (address
in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)), not to a public issue.

## Code style and checks

Before you open a pull request, run these six commands — the ones CI's build
job runs on every platform, in the same order:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
RUSTDOCFLAGS="--cfg test -D warnings" cargo doc --workspace --no-deps --document-private-items
```

The last one documents the private items too, with the test configuration
on, so that the modules compiled only for tests get their doc comments and
links checked as well; the normal doc build never sees them.

Some tests read real sample files, which are too large to commit. When the
files are absent, those tests print a note and pass, so the suite runs
offline. `scripts/fetch-samples.sh` downloads the pinned set listed in
[`tests/samples.tsv`](tests/samples.tsv), about 1.6 GB: RAW files from
raw.pixls.us and camera JPEGs from Wikimedia Commons, as the list's
source column says. It saves them into `raw-samples/` at the
repository's root, a folder that git ignores and that `cargo clean`
leaves alone, or into the directory named by `RAW_FILES_SAMPLES`, and
checks each file's SHA-256; `cargo test` then runs those tests too.
`scripts/fetch-samples.sh nef` (or `arw`, or `jpeg`) fetches the files of
one format only, and one file, the 325 MiB ARQ, is fetched only with
`RAW_FILES_LARGE=1`. CI does this on Linux, from a cache that a workflow
on `master` keeps filled, so a pull request downloads nothing unless it
changes the list.

One test is ignored by default: it runs the reader over a folder of your own
camera files, RAW and JPEG, and prints counts only, never a path, the value
of a make, model, software, serial number, time, exposure or lens, or where
the camera focused, so its output can go into an issue. It counts each file
under the format its extension names, in one section per format, and skips
the AppleDouble files, whose name starts with `._`, which macOS writes
beside each file on a disk such as an exFAT one, saying how many it
skipped. It checks that every file opens, that every one has a preview,
that the largest preview is a whole JPEG, that the metadata gives the
preview's orientation, and that every focus point, and every point of a
list of the points the camera used, lies inside its frame, area included,
both as read and once placed on the largest preview; and that the focus
point, the list and the layout of the body's points agree: the focus point
is the list's primary, a recorded position is a list of one point, and
points placed from a table are points of the layout for the file's Model.
It counts how many files have each metadata field and a focus point, how
many previews of each size they hold, the focus points by origin, recorded
or placed from a table, the lists by number of points and by area mode, and
the files with a layout, with and without a focus point; and, for each
camera setting, how many files have each kind of it,
such as aperture priority, auto white balance or a white balance
fine-tuning of `(-24/4, 0/4)`, for the colour temperature its range, and
how many flashes fired: totals over the files of a format, never tied to
one file. A preview may end in padding after the image's end marker, as
some Sony cameras write it: `FF` bytes in any file, and zeros in a JPEG
file, whose largest preview is the whole file and must hold its own image's
end marker, not only its thumbnail's. Other data there fails a RAW file's
preview, and is only counted in a JPEG file; except that a NEF file's
preview may end before its declared length in the shape the D600 and
the D3200 write about once in 500 files, a length 2 more than a
multiple of 512 with up to 513 bytes of other data after the image's
own end marker, which is counted as a preview that ends early. It fails
on a folder it cannot
read, that holds no file of a format raw-files reads, or that holds an entry
it cannot read. Run it with `--release`, because it reads every file:

```sh
RAW_FILES_LOCAL_CORPUS=/path/to/photos cargo test --release --test local_corpus -- --ignored --nocapture
```

CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs them on every
pull request on macOS, Linux and Windows, builds and tests the crate with
Rust 1.95, the oldest version it supports, and checks the commit messages and
that there are no merge commits in the PR. CI is the real gate: a pull request
is merged only when all of its checks pass.

CI also compares the crate's public API with the newest version on crates.io,
with `cargo-semver-checks`, so that a change that would break callers of the
last release is caught before it is merged. It catches many breaking changes,
such as a removed or renamed item, but not all: a changed type or behaviour
is still for review to find. A breaking change is allowed only once the
version in `Cargo.toml` has been raised to one that permits it: before 1.0,
the minor number, as the versioning promise in `CHANGELOG.md` says. So the
pull request that makes the first breaking change of a release raises the
version, not the commit that releases it. Adding to the API needs no bump to
pass, and that a patch release adds nothing to the API is for review to
check. To run it yourself, use `cargo install cargo-semver-checks --locked`,
then `cargo semver-checks` in the repository root (it downloads the published
version from crates.io the first time). It is not one of the six commands
above because it needs that install and the network.

The oldest supported Rust (`rust-version` in `Cargo.toml`) is a stable
release at least six months old. It is raised only when the code needs
something newer, in a minor release, with a line in the changelog; raising
it is not a breaking change.

A few conventions:

- **Bug fixes come with a regression test** — one you have seen fail without
  the fix and pass with it. For a file that failed to parse, the test feeds
  the parser the smallest input that shows the problem.
- **A user-visible change adds a line to [CHANGELOG.md](CHANGELOG.md)** under
  `[Unreleased]`, in the same pull request. For a library, user-visible means
  anything a caller can see: the public API, its behaviour, a newly supported
  camera, the minimum Rust version. Internal refactors and docs changes don't
  need one.
- **Comments describe the code as it is**, not its history ("added in v0.2"
  belongs in git). A comment marking a known shortcut cites its issue —
  `// See #123` — rather than a bare `TODO`.
- **New source files start with the license header:**
  `// SPDX-License-Identifier: MIT OR Apache-2.0`.
- **Decisions that would surprise someone reading only the code** get a short
  Architecture Decision Record in
  [docs/architecture-decisions/](docs/architecture-decisions/README.md).
- **The public API follows the
  [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html).**
  Before a release, go through its checklist for every public item added
  since the last one: the common traits, a rustdoc example on each type,
  an `# Errors` section on each fallible function, the Cargo metadata.
  Where the project departs from a guideline on purpose, say so where a
  reader would look: `DateTime`, `Rational` and `SignedRational` have no
  `Ord`, and their docs say why; there is no serde feature, because the
  crate has no dependencies, and the crate docs say so. The items about
  macros, collections, iterators, smart pointers and destructors have
  nothing to apply to yet.
- **Values are changed with `with_…` methods.** A value a caller reads,
  such as `Metadata`, `Exposure` or `Lens`, starts from `default()`, and
  each of its values is set by a `with_…` method named after the getter
  that reads it back: `with_model` sets what `model()` returns. A value
  whose parts mean nothing alone, such as `FocusPoints`, which needs a
  frame and an origin, or `Flash`, which needs whether it fired, has no
  `default()`: its constructor takes those parts, its `with_…` methods
  set the rest, and where a file may not record it, the getter that
  gives it returns an `Option`, as `Metadata::focus_points` and
  `CameraSettings::flash` do. A builder whose last call does the work,
  such as `Options`, which ends in `open`, `from_bytes` or `from_reader`,
  names its setters without the prefix, as `std::fs::OpenOptions` does.

## Fuzzing

The parsers read files from anyone, so they must answer any input with a
result: never a panic, and never unbounded memory. Fuzzing tests that on
inputs nobody would write by hand. The fuzz targets live in
[`fuzz/`](fuzz/), a separate crate that is not part of the published
package, and need nightly Rust and
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz):

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
```

To fuzz the TIFF reader for a minute, from the repository root:

```sh
mkdir -p fuzz/corpus/tiff_walk
cargo +nightly fuzz run tiff_walk fuzz/corpus/tiff_walk fuzz/seeds/tiff_walk -- -max_total_time=60 -timeout=25 -malloc_limit_mb=256
```

The first directory is where the fuzzer saves what it finds (it is not
committed, and libFuzzer refuses to start when it does not exist, hence the
`mkdir`); the second holds the seeds it starts from, a few valid files
committed in [`fuzz/seeds/`](fuzz/seeds/README.md). Without them the fuzzer
never builds a big-endian header, and half of the reader goes unfuzzed.
The target `jpeg_probe`, which fuzzes the search for a JPEG preview's
frame header, runs the same way, with `jpeg_probe` in place of `tiff_walk`
throughout. So does the target `jpeg_segments`, which reads the input as
a JPEG file: the walk over its segments, the TIFF structure of its Exif
segment with every value in it, and the index of its MPF segment, which
lists the images after the first. Its seeds are JPEG files built as the
JPEG reader's tests build them, and the stripped camera JPEGs committed
in `tests/fixtures/`:

```sh
mkdir -p fuzz/corpus/jpeg_segments
cargo +nightly fuzz run jpeg_segments fuzz/corpus/jpeg_segments fuzz/seeds/jpeg_segments -- -max_total_time=60 -timeout=25 -malloc_limit_mb=256
```

The target `open`, which opens the input as a caller would and reads its
metadata, its focus point placed on a preview, and every preview it finds,
has seeds of its own, fixtures of the Nikon and Sony layouts, and takes
the TIFF ones and the JPEG ones too, since a plain TIFF file and a JPEG
file are also something it must open; libFuzzer takes several corpus
directories:

```sh
mkdir -p fuzz/corpus/open
cargo +nightly fuzz run open fuzz/corpus/open fuzz/seeds/open fuzz/seeds/tiff_walk fuzz/seeds/jpeg_segments -- -max_total_time=60 -timeout=25 -malloc_limit_mb=256
```

The allocation limit makes a runaway allocation a finding: the run stops
on any one allocation over 256 MiB. (libFuzzer also stops once the whole
process passes 2 GiB; a lower limit trips on the sanitizer's own
bookkeeping, not on the reader.) The timeout makes a hang a finding too:
libFuzzer waits 1200 seconds for one input by default, longer than the CI
job lasts, and a parse takes milliseconds, so 25 seconds is a hang. It is
reported as a `timeout-*` file in `fuzz/artifacts/`.
An input that fails is saved in `fuzz/artifacts/`; a fix for it comes with
a regression test that reads that input, as for any other bug.

No other build compiles the fuzz code, so CI checks that both parts of it
compile, on stable Rust: the library's entry points, with
`RUSTFLAGS="--cfg fuzzing" cargo clippy --workspace --all-targets -- -D warnings`,
and the fuzz crate, with
`RUSTFLAGS="--cfg fuzzing" cargo check --manifest-path fuzz/Cargo.toml`.
CI runs the four targets for ten minutes each twice a week and on request
(`.github/workflows/fuzz.yml`, started by hand from the Actions tab), with
the same commands as above but `-max_total_time=600`, ten minutes per
target, and keeps the corpus each run found in the Actions cache for the
next run. It uploads any failing input as a run artifact for thirty days.
It is not a required check, so a finding there is a bug to fix, not a
blocked pull request.

## Local git hooks (optional)

The repository ships git hooks that catch the common mistakes before they
reach CI. They are optional — CI checks the same things — but they save a
round trip:

```sh
scripts/install-hooks.sh          # enable them for this checkout
scripts/install-hooks.sh --check  # report whether they are enabled
```

The script points `core.hooksPath` at [`.githooks/`](.githooks/) and sets
`pull.ff only`, so a `git pull` that would create a merge commit stops with an
error instead.

- `commit-msg` checks the commit message against the active grammar.
- `pre-merge-commit` and `pre-commit` refuse merge commits and squash merges
  on any branch — rebase instead.
- `pre-push` refuses a direct push to `master` or a push containing a merge
  commit, then runs `cargo fmt --check` and `cargo clippy`.

To skip them once, set `SKIP_HOOKS=1` (for example `SKIP_HOOKS=1 git push`).

If you change a hook or a script, keep it executable in git:
`git update-index --chmod=+x <file>`. CI checks this.

## Clean room and third-party code

raw-files is a clean-room implementation: it is written from what the
format notes in [docs/formats](docs/formats/) record, never from another
project's source code
([ADR013](docs/architecture-decisions/adr013-clean-room-and-third-party-code.md)).
In short:

- **Learn facts, write your own code.** A tag number, a field offset or a
  flag's meaning is a fact: take it from published documentation, from a
  sample file, or from what another program reports about a file, write
  it into the notes with its source, confirm it on a file where you can,
  and code from the notes.
- **Never copy or translate code, comments or tables from another
  project's source**, whatever its license (dcraw, LibRaw, rawspeed,
  rawler, ExifTool, exiv2 and the like). Reading their source to settle a
  fact is fine; what you learnt goes through the notes and a sample file
  like any other fact, and is coded once a file confirms it. Where you
  can, let someone else write the code for a part whose source you have
  read. Never take a program apart: no firmware or software is
  disassembled or decompiled to learn a format; observing what a program
  does with an input is fine, recorded so that others can repeat it.
- **Permissively licensed code (MIT, BSD, Apache-2.0) enters only by a
  recorded decision**, with its license notice and author kept in a
  notices file and a comment at the code. Nothing is copied without
  attribution.
- **Run other readers only to compare.** ExifTool or exiv2 may be run on a
  file, or their published output read, to see what another reader found;
  record it as a source and confirm it on the file's bytes where you can.
  Nothing of theirs is a dependency, and neither the test suite nor CI
  runs them: the samples stay the evidence.
- **Say how you used a source.** In a commit, a pull request or a doc,
  name whether you read a project's documentation or its source, which
  file confirmed the fact, or that you ran a program to compare with.

## Adding a camera's focus points

A DSLR shot through the viewfinder names its autofocus point by number,
and where that point lies on the picture is in no file: it takes a
table of the body's points. Such a table enters raw-files only when
measured by the project or by a contributor on that body, on frames
shot with it and read in the maker's own software; none is taken from
another project on its licence alone
([ADR017](docs/architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)).
A body without a table gives no point for such a shot.

You need no code and no measuring to add a body you own: shoot the
frames the NEF notes' procedure lists, RAW + JPEG
([autofocus-points.md](docs/formats/nef/autofocus-points.md#1-the-procedure)),
and open an issue naming the body. The maintainer will say where to
send them, measure them, write the record and the table, and credit
you in both. With your agreement, the frames are committed stripped of
their pixels and personal fields, under the repository's licence.

To measure a body yourself, follow the whole procedure
([autofocus-points.md](docs/formats/nef/autofocus-points.md#1-the-procedure)):
shoot one frame per point in single-point mode and the control frames
the page lists, read every frame in the maker's software with its
focus-point display on, measure the screenshots by pixel, and check
the controls. Then write the body's record on that page, as the D600's
is written, with the readings and their tolerance; the table in the
code, with its provenance; and the frames, stripped of their pixels
and personal fields, with their expected values in the tests. In the
pull request say which software and version you ran, and that the
frames are your own: they are licensed as the repository's fixtures
are ([tests/fixtures/README.md](tests/fixtures/README.md)).
The maintainer checks the table against your frames before it enters,
so keep them, and expect to be asked for them.

## License of contributions

raw-files is licensed under the [MIT license](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option. Unless you
explicitly state otherwise, any contribution you intentionally submit for
inclusion in raw-files, as defined in the Apache-2.0 license, is licensed under
the same terms, without any additional terms or conditions. There is no
contributor license agreement (CLA), and you keep the copyright to your work.
