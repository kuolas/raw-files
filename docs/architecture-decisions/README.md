# Architecture Decision Records

The substantial decisions that shape raw-files live here, one short record
per problem, kept current: the context, what was decided, and what follows
from it. They answer
"why is it like this?" for anyone reading the code later. The records
speak for the project; where one cites my own shots, it is me, Alvaro
Kuolas, who maintains raw-files.

| ADR | Title | Status |
|---|---|---|
| [ADR001](adr001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [ADR002](adr002-license-mit-or-apache-2.md) | License: MIT OR Apache-2.0 | Accepted |
| [ADR003](adr003-linear-history.md) | Linear history with rebase-merge | Accepted |
| [ADR004](adr004-gnome-commit-messages.md) | GNOME-style commit messages | Accepted |
| [ADR005](adr005-github-issues-tracker.md) | GitHub Issues as the only tracker | Accepted |
| [ADR006](adr006-ai-assisted-contributions.md) | AI-assisted contributions | Accepted |
| [ADR007](adr007-pure-rust-cross-platform.md) | Pure Rust, on macOS, Linux and Windows | Accepted |
| [ADR008](adr008-lints-and-panics.md) | No unsafe code, a few extra lints, no panics on malformed input | Accepted |
| [ADR009](adr009-public-api-shape.md) | The shape of the public API | Accepted |
| [ADR010](adr010-decoding-ends-at-a-linear-image.md) | Decoding ends at a linear image | Accepted |
| [ADR011](adr011-metadata-from-standard-tags.md) | Metadata comes from the standard tags | Accepted |
| [ADR012](adr012-focus-points-as-the-camera-records-them.md) | Focus points as the camera records them | Accepted |
| [ADR013](adr013-clean-room-and-third-party-code.md) | Clean-room implementation and third-party code | Accepted |
| [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md) | JPEG files as a format of their own | Accepted |
| [ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md) | The focus point of a crop-mode JPEG | Accepted |
| [ADR016](adr016-full-metadata-before-raw-data.md) | Full metadata before raw data | Accepted |
| [ADR017](adr017-per-body-tables-measured-by-the-project.md) | Per-body tables measured by the project | Accepted |
| [ADR018](adr018-focus-points-placed-from-a-table-and-the-points-used.md) | Focus points placed from a table, and the points used | Accepted |
| [ADR019](adr019-the-frame-of-a-placed-focus-point-on-a-crop.md) | The frame of a placed focus point on a crop | Folded into [ADR018](adr018-focus-points-placed-from-a-table-and-the-points-used.md) |
| [ADR020](adr020-each-crops-window-in-the-table.md) | Each crop's and JPEG size's window in the table | Folded into [ADR018](adr018-focus-points-placed-from-a-table-and-the-points-used.md) |
| [ADR021](adr021-what-the-file-records-and-what-raw-files-reads-into-it.md) | What the file records and what raw-files reads into it | Accepted |
| [ADR022](adr022-names-as-their-makers-write-them.md) | Names as their makers write them | Accepted |
| [ADR023](adr023-every-entry-a-file-records.md) | Every entry a file records | Accepted |

## When to write one

Write an ADR when a decision would surprise someone reading only the code, is
hard to reverse, or settles a debate that would otherwise come back. Small,
local choices belong in a code comment or the commit message instead.

## Creating an ADR

1. Copy [`adr000-template.md`](adr000-template.md) to
   `adr000-my-decision.md` (a short, descriptive name; leave the number at 000).
2. Fill it in and open a pull request. Discussion happens on the PR.
3. Before merging, give it the next free number, rename the file to match,
   and add it to the table above.

## Changing a decision

There is one record per problem, and it states the current answer. When
the answer to a problem changes, whether the decision grows, is corrected
or is reversed, its record changes with it, so that a reader finds the
answer in one place rather than piecing it together from a record and the
records that amend it: while a decision holds, in substance or in spirit,
a new record would only make it harder to read. The text a record had
before stays in git history and in each release's tag. ADR001 is the
decision and its reasons.

- **Same problem, small change** (a renamed term, a figure measured again,
  a moved file, a slip): edit the record in place, with no new revision.
- **Same problem, a decision grown or a new solution** (the decision
  extended to more values or cases, or another answer in its place):
  rewrite the record in place and raise its revision. The header keeps
  `- Date:`, when the problem was first decided, and gains a line under
  it, `- Revision: N, YYYY-MM-DD`, with N at 2 the first time and the date
  the change was decided; the `- Issue:` line names the issue or pull
  request where it was decided. A solution left behind, and an alternative
  weighed for the change, become Rejected bullets saying why.
- **Two records, one problem:** fold one into the other, keeping every
  reason. The folded record's file stays at its old path as a stub, its
  title and one sentence linking the record it went into, since the
  documentation of a published release links to the records on the
  default branch; its number is not reused, and the table above says
  where it went.
- **New problem:** a new record, numbered as in "Creating an ADR". When its
  answer rests on another record's decision, it says "Builds on
  [ADRxxx](…)" next to its status, and both stay Accepted.
- **Problem gone:** the record's status becomes Deprecated, with one line
  saying why. A number is never reused.

A record keeps its file name when its title changes, since every link
names the file.

Every change to a record greps the tree for its number (`git grep -n
ADRnnn`) and rechecks each sentence that cites it, in the same pull
request: a citation says what the record decided, so a new answer can
leave a citing sentence wrong.
