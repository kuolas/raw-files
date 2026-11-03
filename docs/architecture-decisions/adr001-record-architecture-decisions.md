# ADR001: Record architecture decisions

- Status: Accepted
- Date: 2026-10-10
- Revision: 3, 2026-11-03
- Issue: #80; the stub of a folded record #81

## Context

Decisions about licensing, workflow and design are made once and then live on
in the code and the process. Without a record, the reasons are lost, newcomers
cannot tell a deliberate choice from an accident, and settled debates come
back.

The records first kept the rule most projects give them: a record is never
rewritten, and a changed decision gets a new number that supersedes the old
record or amends part of it. raw-files is at an early stage, and its
decisions grow as the files it reads teach more. Under that rule a decision
whose substance held still gained a new record each time it grew: the frame
of a point placed from a table was decided by ADR018, amended by ADR019,
which ADR020 amended in turn, so a reader had to read three records, in
order, to learn one answer; and ADR011 gained sections that a reader had to
weigh against the text above them.

## Decision

We will keep Architecture Decision Records in `docs/architecture-decisions/`,
one Markdown file per problem, numbered `adrNNN-short-name.md`, in the
Context / Decision / Consequences format, each stating the current answer to
its problem. The folder's [README](README.md) indexes them and explains how
to add and change one. A change to the answer to a problem, whether the
decision grows, is corrected or is reversed, is written into that problem's
record, never into a new one: while a decision holds, in substance or in
spirit, a new record would only make it harder to read.

- **Same problem, small change** (a renamed term, a figure measured again,
  a moved file, a slip): the record is edited in place, with no new
  revision.
- **Same problem, a decision grown or a new solution** (the decision
  extended to more values or cases, or another answer in its place): the
  record is rewritten in place and its revision raised. The header keeps
  `Date:`, when the problem was first decided, and gains
  `Revision: N, YYYY-MM-DD`, dated when the change was decided, with its
  `Issue:` line naming the issue or pull request where it was decided; a
  solution left behind, and an alternative weighed for the change, become
  Rejected bullets saying why. The old text stays in git history and in
  each release's tag.
- **Two records, one problem:** one is folded into the other, every reason
  kept. Its file is left at its old path as a stub, its title and one
  sentence linking the record it went into, since the documentation of a
  published release links to the records on the default branch; its
  number is not reused.
- **New problem:** a new record with a new number, which says "Builds on"
  a record whose decision its answer rests on. **Problem gone:** the
  record's status becomes Deprecated, with one line saying why. A number
  is never reused.

Every change to a record greps the tree for its number and rechecks each
sentence that cites it, in the same pull request: a citation states what the
record decided, so a new answer can leave a citing sentence wrong.

Rejected:

- **A new record for each changed decision**, superseding or amending the
  old one: the folder fills with answers no longer current, a reader must
  piece an amended record together with its amendments, and a citation of
  a superseded number leads to an old answer. What that rule keeps, the
  text each record had when it was decided, git history and the release
  tags keep too.
- **Additions written into the old record in dated sections**, the old text
  kept as it was: the record stays one file, but its reader still weighs
  each old sentence against the sections that say it no longer holds.
- **A revision as a new file beside the old one:** every link names a file,
  so each revision would move them all, and two files per number bring the
  old answers back.
- **Deleting a folded record's file**, as this record first had it: the
  documentation of a published release links to the records on the
  default branch, so its links would break.

## Consequences

Anyone can find why something is the way it is, and propose a change by
editing its record in a pull request, or by writing a new one for a new
problem. Records cost a few minutes each and only help if people keep
writing them; a decision taken without one leaves a gap. Docs and commit
messages cite a record by saying what it decided, with the number in
parentheses, which is what lets a revision find the sentences it may have
made wrong.

A record shows only the current answer; what it said before, and when, is
read from its file's history (`git log -p`) or from the tag of a release.
The documentation of a published release links to the records on the
default branch, so it shows their current answer, which a record's
revision line dates.
