# ADR007: Pure Rust, on macOS, Linux and Windows

- Status: Accepted
- Date: 2026-10-10

## Context

raw-files reads camera RAW files for programs that run on all three desktop
systems. How it is built decides what a program that depends on it has to
install, compile and ship, and whether the code that parses untrusted files
is memory-safe. The options considered were bindings to LibRaw (C++), building
on an existing pure-Rust RAW decoder, and a pure-Rust implementation of our
own.

## Decision

We will write raw-files in **pure Rust**: no C or C++ code, no `-sys` crate
linking a native library, and no build script that compiles native code.
**macOS, Linux and Windows** are equal targets.

- **Pure Rust:** adding the dependency is the whole installation, on every
  platform Rust targets. There is no system library to find, build or ship,
  cross-compiling works, and the code that parses untrusted files is written
  in a memory-safe language.
- **Rejected LibRaw bindings:** the widest camera support, but they need a C++
  toolchain on every build machine and a native library in every package, and
  they put a large C++ parser of untrusted input inside every program that
  uses them — the kind of code where memory-safety bugs become security bugs.
- **Rejected building on an existing pure-Rust decoder:** those crates aim at
  full decoding, and bring that weight and their API to a program that only
  wants the preview. Starting from the preview keeps the common case small,
  and lets the API grow from what callers actually use.

## Consequences

- CI builds and tests on all three systems on every pull request.
- Camera support is built format by format. Knowledge of a format comes from
  public documentation and sample files; code taken from another project
  needs a license compatible with MIT OR Apache-2.0
  ([ADR002](adr002-license-mit-or-apache-2.md)).
- Every dependency must be pure Rust too; one that links a native library
  needs a new ADR.
