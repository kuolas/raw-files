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
  - `Bug: #12` refers to an issue without closing it.

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

Before you open a pull request, run these six commands — the same ones CI
runs, in the same order:

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

CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs them on every
pull request on macOS, Linux and Windows, builds and tests the crate with
Rust 1.95, the oldest version it supports, and checks the commit messages and
that there are no merge commits in the PR. CI is the real gate: a pull request
is merged only when all of its checks pass.

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
cargo +nightly fuzz run tiff_walk fuzz/corpus/tiff_walk fuzz/seeds/tiff_walk -- -max_total_time=60 -malloc_limit_mb=256
```

The first directory is where the fuzzer saves what it finds (it is not
committed, and libFuzzer refuses to start when it does not exist, hence the
`mkdir`); the second holds the seeds it starts from, a few valid files
committed in [`fuzz/seeds/`](fuzz/seeds/README.md). Without them the fuzzer
never builds a big-endian header, and half of the reader goes unfuzzed.

The allocation limit makes a runaway allocation a finding: the run stops
on any one allocation over 256 MiB. (libFuzzer also stops once the whole
process passes 2 GiB; a lower limit trips on the sanitizer's own
bookkeeping, not on the reader.)
An input that fails is saved in `fuzz/artifacts/`; a fix for it comes with
a regression test that reads that input, as for any other bug.

No other build compiles the fuzz code, so CI checks that both parts of it
compile, on stable Rust: the library's entry points, with
`RUSTFLAGS="--cfg fuzzing" cargo clippy --workspace --all-targets -- -D warnings`,
and the fuzz crate, with
`RUSTFLAGS="--cfg fuzzing" cargo check --manifest-path fuzz/Cargo.toml`.
It does not run the targets yet; it will, on a schedule.

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

## License of contributions

raw-files is licensed under the [MIT license](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option. Unless you
explicitly state otherwise, any contribution you intentionally submit for
inclusion in raw-files, as defined in the Apache-2.0 license, is licensed under
the same terms, without any additional terms or conditions. There is no
contributor license agreement (CLA), and you keep the copyright to your work.
