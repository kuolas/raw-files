# ADR002: License: MIT OR Apache-2.0

- Status: Accepted
- Date: 2026-10-10

## Context

raw-files is an open-source library. The license decides which programs can use
it, which dependencies it can take, and how contributions come in. The
options considered were a copyleft license (the GPL, the LGPL or the MPL-2.0),
MIT alone, Apache-2.0 alone, and both permissive licenses at the user's
choice — the convention of Rust itself and of most crates.

## Decision

We will license raw-files under the MIT license or the Apache License, version
2.0, at the user's option (`MIT OR Apache-2.0`). Contributions come in under
the same terms (inbound = outbound, as Apache-2.0 section 5 states), with no
CLA and no `Signed-off-by:` requirement.

- **Over a copyleft license:** a RAW library is most useful when any program
  can use it. The GPL would limit it to GPL programs. The LGPL, with Rust's
  static linking, asks every closed-source program to ship its object files
  so that users can relink it, which in practice keeps those programs away.
  Copyleft would keep modified versions free; we accept closed forks as the
  price of wide use.
- **Over MIT alone:** Apache-2.0 adds an explicit patent license from every
  contributor.
- **Over Apache-2.0 alone:** Apache-2.0 is not compatible with GPLv2-only
  code; MIT is, so GPLv2 programs can use raw-files too.
- **The same pair as the ecosystem:** a program that already depends on
  `MIT OR Apache-2.0` crates needs no new license review to add raw-files.

## Consequences

The repository carries both texts, `LICENSE-MIT` and `LICENSE-APACHE`. Each
source file carries `// SPDX-License-Identifier: MIT OR Apache-2.0`, and
`Cargo.toml` declares the same expression.

Every dependency must be compatible with both licenses: MIT, BSD, Apache-2.0,
Zlib and similar permissive licenses are. A copyleft dependency (GPL, LGPL,
MPL) is not accepted, because it would impose terms on programs that use
raw-files that its own license does not.
