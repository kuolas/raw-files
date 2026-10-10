# ADR008: No unsafe code, a few extra lints, no panics on malformed input

- Status: Accepted
- Date: 2026-10-10

## Context

raw-files parses untrusted files — RAW data and embedded JPEG and metadata from
any camera, or from anyone who sends a file — which is where memory-safety
bugs turn into security bugs. As a library it runs inside other people's
programs, so its failures become theirs. Contributors also need the same lint
rules locally and in CI.

## Decision

In `Cargo.toml`:

- **`unsafe_code = "forbid"`.** Our own code contains no `unsafe`, and no
  `#[allow]` can override that; dependencies may contain it. A real need for
  `unsafe` (for example a SIMD decoder) needs a new ADR.
- **`missing_docs = "warn"`.** Every public item is documented: for a
  library, the API documentation is part of what it ships.
- **Clippy `dbg_macro`, `todo`, `unimplemented` warn** — with CI's
  `-D warnings` they keep debugging leftovers and stubs off `master`.
  Clippy's default lint set otherwise applies; the stricter `pedantic` group
  is not enabled, because its casting lints are noisy in binary-parsing code.

And as a rule of the code:

- **Malformed input returns an error, never panics.** A truncated or corrupt
  file is an expected input for a RAW parser, not a bug in the caller, and a
  panic would take the caller's program down with it. No lint enforces this
  without drowning the code in noise (`clippy::indexing_slicing` would flag
  every bounds-checked slice); tests on truncated and corrupted input do, and
  fuzzing will once there is a parser to fuzz.

No `[profile.release]` is set: a library's profile settings do not apply to
the programs that depend on it.

## Consequences

The lint rules live in one place and apply identically in editors, local
runs and CI. Callers can rely on getting an `Err` for a bad file, which costs
the parser explicit bounds and overflow checks wherever it reads offsets and
lengths from the file.
