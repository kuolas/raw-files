# CLAUDE.md

raw-files is a camera RAW library in pure Rust (edition 2024, one library
crate, no unsafe code, MIT OR Apache-2.0). It is at an early stage; see
[docs/ROADMAP.md](docs/ROADMAP.md) for what is planned.

[CONTRIBUTING.md](CONTRIBUTING.md) is the rulebook — workflow, commit
messages, checks — and applies to assistants exactly as to humans.
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) explains the structure, and
[docs/architecture-decisions/](docs/architecture-decisions/README.md) why.

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
RUSTDOCFLAGS="--cfg test -D warnings" cargo doc --workspace --no-deps --document-private-items
```

## Reminders

- Commit messages follow CONTRIBUTING.md §Commit messages; run
  `scripts/install-hooks.sh` once so the `commit-msg` hook checks them.
- Keep history linear: rebase, never merge; every commit builds and passes
  tests (`scripts/check-series.sh`).
- AI-assisted work follows CONTRIBUTING.md §AI-assisted contributions: when a
  significant part of a commit is taken unchanged from AI output, add
  `Assisted-by: Claude` and tick the PR template's AI box. Never name a tool
  in `Co-Authored-By:`.
- A malformed file returns an error, never a panic: check every offset and
  length read from a file before using it (ADR008).
- Clean room (CONTRIBUTING.md §Clean room and third-party code, ADR013):
  facts from documentation, sample files and other readers' output go
  into the notes, and code is written from the notes; never copy or
  translate code, comments or tables from another project's source,
  whatever its license, and never disassemble a program or firmware to
  learn a format; permissive code only by a recorded decision with
  attribution; ExifTool and exiv2 run only to compare, never in the suite
  or CI; say in every commit, PR and doc how a third-party source was
  used.
- New source files start with `// SPDX-License-Identifier: MIT OR Apache-2.0`.
