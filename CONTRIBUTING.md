# Contributing to raw-files

Thanks for your interest! raw-files is a camera RAW library written in pure
Rust. It is at a very early stage, so ideas, bug reports and sample files are
the help it needs most. Changes to the code and docs come from contributors;
see [Becoming a contributor](#becoming-a-contributor).

This page is the one place the project's rules live. How to run the tests
beyond the six commands is in [docs/TESTING.md](docs/TESTING.md). If
something here is unclear, that is a bug in this page, and an issue about it
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
- `enhancement` — a new feature or a change in behavior. The *Feature
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

After an issue is opened:

- `question` — work waits on more information, usually from whoever
  opened the issue. Despite its name, it does not mark an issue that asks
  a question.
- `help wanted` — the maintainer would welcome a contributor taking it on.
- `good first issue` — a small task that needs little knowledge of the
  code, a good first one for a new contributor.
- `duplicate` — another issue already covers it; it is closed with a link
  to that one.
- `invalid` — not a problem in raw-files; it is closed with the reason.
- `wontfix` — decided against; it is closed, and the issue says why.

## Security issues

Please **don't** report security problems in a public issue. See
[SECURITY.md](SECURITY.md) for how to report them privately.

## Becoming a contributor

Pull requests come from contributors. While the project is young, a pull
request from someone who is not one yet is closed with a link to this section,
however good the change. Sample files and test shots are welcome from anyone,
through an issue. The [sample files page](docs/formats/sample-files.md) says
how to propose a sample file.

You become a contributor by taking part first:

1. **Start in [Discussions](https://github.com/kuolas/raw-files/discussions).**
   Say who you are and what brings you to the project.
2. **Use the library** on your own files, from your own cameras.
3. **Report what you find**: bug reports and feature requests that come from
   that use, through the issue forms.

A contributor also has a public GitHub profile with a visible record of
activity, such as projects, issues and reviews, so the maintainer can see how
they work.

After you have taken part this way for a while, the maintainer invites you to
work on an issue. Agree on the approach in that issue before you start.

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

   - Keep refactoring and behavior changes in separate commits.
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

## Writing

These rules hold for everything the project writes: the docs, the
comments, the CHANGELOG, commit messages, pull requests and issues.

- **One claim per sentence**, of about 25 words at most. An exception
  gets a sentence of its own.
- **A list or a table** where a sentence would count things off.
- **One home for each rule.** State it once, and link to it from the
  other places that need it. Three kinds of fact have a set home:
  - what a camera body or a file records: the format notes, in
    [docs/formats/](docs/formats/README.md);
  - the committed test files: [tests/fixtures/README.md](tests/fixtures/README.md);
  - the public types: the [API documentation](https://docs.rs/raw-files).
- **The API's own names.** Write `RawFile::entries`, not "the view of
  every entry". A term of the project's own is defined where it is
  first used.

**Spelling.** Everything is American, prose and every name in code
alike: color, center, meter, recognize. "License" is the noun and the
verb. The standard is written "Exif". A name keeps the
spelling its owner gives it, such as ExifTool's `EXIF` group or a term
a standard defines. A quoted text keeps its own.

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
  needed — the diff already shows *how*. It is written as
  [Writing](#writing) says. If the commit fixes a bug introduced by an
  earlier commit, name it in the body as `64aee44 ("tiff: Read IFDs
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

Git's own subjects (`Revert "…"`, `Reapply "…"`, `fixup! …`, `squash! …`,
`amend! …`, and the merge subjects git and GitHub write, such as
`Merge branch '…'`) are accepted as git writes them; for a revert, add the
reason to the body. Only a subject that starts the way git writes it is
exempt: `Revert stuff.` is checked like any other subject. The rule on
authorship holds for these subjects too.

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

Some tests read real sample files, which are too large to commit. When
the files are absent, those tests print a note and pass, so the suite
runs offline. `scripts/fetch-samples.sh` downloads them.

Four tests are ignored by default. One runs the reader over a folder of
your own camera files. The other three write the fuzz seeds again
after a fixture changes.

[docs/TESTING.md](docs/TESTING.md) says how to fetch the sample files
and how to run the ignored tests. It also says how to fuzz the readers,
and what CI runs.

CI is the real gate: a pull request is merged only when all of its
checks pass.

A breaking change is allowed only once the version in `Cargo.toml`
permits it. Before 1.0, that is a raised minor number, as the
versioning promise in `CHANGELOG.md` says. So the pull request that
makes the first breaking change of a release raises the version, not
the commit that releases it. CI's
[check for breaking changes](docs/TESTING.md#the-check-for-breaking-changes)
catches many breaking changes, but not all. That a patch release adds
nothing to the API is for review to check.

The oldest supported Rust (`rust-version` in `Cargo.toml`) is a stable
release at least six months old. It is raised only when the code needs
something newer, in a minor release, with a line in the changelog; raising
it is not a breaking change.

A few conventions:

- **Bug fixes come with a regression test** — one you have seen fail without
  the fix and pass with it. For a file that failed to parse, the test feeds
  the parser the smallest input that shows the problem.
- **A user-visible change adds an entry to [CHANGELOG.md](CHANGELOG.md)**
  under `[Unreleased]`, in the same pull request. For a library,
  user-visible means anything a caller can see: the public API, its
  behavior, a newly supported camera, the minimum Rust version. Internal
  refactors and docs changes don't need one.
- **An entry opens with one sentence that says what changed**, of 25
  words at most, and names the item. The detail follows in short
  sentences or a list. A rule that the item's docs hold is left to
  them: a release's opening links to the API docs of that version.
- **An entry that can break a caller opens with `**Breaking:**`**, and
  comes first in its section. A change is breaking when a caller must
  change their code, or their limits, to keep what the last release
  gave them. The release's opening says how many entries are marked.
- **Comments describe the code as it is**, not its history ("added in v0.2"
  belongs in git). A comment marking a known shortcut cites its issue —
  `// See #123` — rather than a bare `TODO`.
- **An item only the tests use is marked `#[cfg(test)]`**, or
  `#[cfg(any(test, fuzzing))]` when the fuzz entry points call it too.
  The library's normal build then leaves it out, so any other caller
  fails that build until the mark is removed.
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
- **Every setter is a `with_…` method.** A value a caller reads,
  such as `Metadata`, `Exposure` or `Lens`, starts from `default()`, and
  each of its values is set by a `with_…` method named after the getter
  that reads it back: `with_iso` sets what `iso()` returns. A value that
  raw-files computes from others when it is read has no `with_…` method:
  its docs name the method that sets the value it is read from
  ([ADR021](docs/architecture-decisions/adr021-what-the-file-records-and-what-raw-files-reads-into-it.md)).
  A value whose parts mean nothing alone, such as `FocusPoints`, which
  needs a frame and an origin, or `Flash`, which needs whether it fired,
  has no `default()`: its constructor takes those parts, its `with_…`
  methods set the rest, and where a file may not record it, the getter
  that gives it returns an `Option`, as `Metadata::focus_points` and
  `CameraSettings::flash` do. `Options`, which opens files, starts from
  `new()`. Its setters follow the same rule: `with_limits` sets what
  `limits()` returns.
- **A function that can refuse its arguments returns
  `Result<_, ValueError>`**, and its `# Errors` section lists the
  variants in the order it checks them
  ([ADR009](docs/architecture-decisions/adr009-public-api-shape.md)).
- **A malformed file gets a result, never a panic**, and never
  unbounded memory
  ([ADR008](docs/architecture-decisions/adr008-lints-and-panics.md),
  [ADR009](docs/architecture-decisions/adr009-public-api-shape.md)).
  Fuzzing tests that, as [docs/TESTING.md](docs/TESTING.md#fuzzing)
  says.

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

If you change `commit-msg`, run `scripts/test-hooks.sh`; CI runs it too. If
you change a hook or a script, keep it executable in git:
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
  fact is fine; what you learned goes through the notes and a sample file
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
another project on its license alone
([ADR017](docs/architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)).
A body without a table gives no point for such a shot.

You need no code and no measuring to add a body you own: shoot the
frames the NEF notes' procedure lists, RAW + JPEG
([autofocus-points.md](docs/formats/nef/autofocus-points.md#1-the-procedure)),
and open an issue naming the body. The maintainer will say where to
send them, measure them, write the record and the table, and credit
you in both. With your agreement, the frames are committed stripped of
their pixels and personal fields, under the repository's license.

A contributor ([Becoming a contributor](#becoming-a-contributor)) can
also measure a body themselves, by the whole procedure
([autofocus-points.md](docs/formats/nef/autofocus-points.md#1-the-procedure)).
It says how to shoot, read and measure the frames, and what the
record, the table and the fixtures hold. In the pull request, say
which software and version you ran, and that the frames are your own.
They are licensed as the repository's fixtures are
([tests/fixtures/README.md](tests/fixtures/README.md)). The maintainer
checks the table against your frames before it enters, so keep them,
and expect to be asked for them.

## License of contributions

raw-files is licensed under the [MIT license](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option. Unless you
explicitly state otherwise, any contribution you intentionally submit for
inclusion in raw-files, as defined in the Apache-2.0 license, is licensed under
the same terms, without any additional terms or conditions. There is no
contributor license agreement (CLA), and you keep the copyright to your work.
