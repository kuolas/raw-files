# ADR017: Per-body tables measured by the project

- Status: Accepted
- Builds on: [ADR013](adr013-clean-room-and-third-party-code.md), the
  clean room; [ADR012](adr012-focus-points-as-the-camera-records-them.md),
  focus points as the camera records them
- Date: 2026-10-26
- Issue: #57

## Context

Some facts about a camera are in no file it writes. Where each of a
DSLR's viewfinder autofocus points lies on the picture is one: a shot
through the viewfinder records the point's number and a mask of the
points used, and no position
([ADR012](adr012-focus-points-as-the-camera-records-them.md)). A
decoder's fixed tables, such as the symbol lists of a compression or
the substitution tables of a cipher, and the name of a lens that a file
records only as an identifier are others. Such facts can only come from
outside the file: from a table someone has built.

Other projects publish tables of autofocus-point layouts, some under
permissive licences that would allow copying with attribution.
[ADR013](adr013-clean-room-and-third-party-code.md) lets permissively
licensed code enter by a recorded decision, forbids copying or
translating anything from a copyleft source, and left where a per-body
table may come from undecided. A table taken from another project
carries that project's errors and its choices (what a position means,
which frame it is measured in), which the taker cannot check without
the body in hand. raw-files is new, and much of it is written with AI
assistance ([ADR006](adr006-ai-assisted-contributions.md)); a project
in that position that copies another project's tables before it has
evidence of its own to show stands on borrowed work, and the
community would be right to ask what it checked and what it gave
back. Cooperation between projects on such tables is possible, and
worth having, once each has something to offer the other; it is not
the same thing as copying under a licence.

The camera itself can supply the facts. One session with a body
measures its layout: a frame per point in single-point mode, read in
the maker's own software with its focus-point display on, gives every
point's position; control frames that the file can check, repeats, a
portrait frame, frames in a crop mode, auto-area frames and, where the
body has live view, frames whose area the file records, calibrate the
reading and show how the point behaves. A session of 56 frames with a
Nikon D600, measured this way, gave its 39 points to about 2 pixels,
the bit order of its mask, how a point shifts into a DX crop and what
its manual-focus and live-view shots write
([the record](../formats/nef/autofocus-points.md#2-nikon-d600)). The
reading's error is far smaller than the box a viewer draws.

Each body needs its own session, with the body in hand. A table for a
body the project does not own can come only from someone who owns it:
from their frames, which the project can measure, or from their own
measurement. Shooting the frames asks for no code and no measuring,
so it is open to anyone who owns the body.

## Decision

We will build every per-body table of raw-files from the project's own
measurements, by a procedure the format notes record, and take none
from another project except by a decision made together with that
project.

- **A table enters only when measured by the procedure.** The
  procedure is written in the format notes beside the records it
  produces ([the procedure](../formats/nef/autofocus-points.md#1-the-procedure)):
  frames shot on the body, one per value (for autofocus points, one per
  point in single-point mode); control frames that record the quantity
  in the file where the body can, so that the file calibrates the
  reading; the maker's own software run as a reader to compare with,
  under [ADR013](adr013-clean-room-and-third-party-code.md)'s terms,
  never taken apart; every reading recorded in the notes with its
  method and tolerance, as read, beside the value the table stores; and
  one frame per rule the table serves committed as a fixture, stripped
  of its pixels and personal fields as the existing fixtures are, with
  its expected values in the tests.
- **A table is Rust source, with its provenance in the file**: who
  measured it, when, with which software and version, on which frames,
  and which fixture checks it. Tables are registered explicitly, and
  tests check that every table file is registered, that each table is
  consistent with itself and with the body it names, and that each
  fixture gives the table's values.
- **A table stores the readings, rounded, not a grid fitted to them.**
  The readings are the evidence; a grid is a guess at what the maker's
  software draws.
- **No table is taken from another project on its licence alone.** A
  table from another project enters only by a decision of its own,
  made in cooperation with that project: with its agreement, its
  licence terms and attribution kept as
  [ADR013](adr013-clean-room-and-third-party-code.md) asks, and a way
  for raw-files to check the values it did not measure. Until such a
  decision exists, no table is transcribed from another project,
  whatever its licence, and none is derived from reading another
  program's code. A name, a numbering or an order that published
  documentation gives may be read as documentation and confirmed on
  the frames, as any fact under ADR013; the values come from the
  frames alone.
- **A contribution is the frames, or the frames and the table.** A
  contributor who owns a body may shoot the frames the procedure asks
  for and send them to the project, which measures them, writes the
  record and the table, and credits the photographer in both; or may
  do the whole procedure and submit the record, the table and the
  fixtures, saying which software and version was run. A table
  measured outside the project enters only after the project has
  checked it against the frames. In both cases the record names who
  shot the frames and who measured them; the frames stay private,
  except those committed as fixtures, which are the photographer's own
  and are published under the repository's licence with their
  agreement.
- **A body without a table gives none**, and the format notes say so
  per body, with the reason counted where the project's test over a
  corpus can count it.

Rejected:

- **Transcribing permissively licensed tables on the licence alone,
  with attribution.** The project's own evidence is the only check it
  can offer on a table, and a borrowed number cannot be re-measured by
  the project; a reader could not tell which values the project had
  checked; and a new project that starts by copying gives the
  community reason to doubt the rest.
- **Forbidding tables from other projects for good.** Cooperation
  between projects that measure the same cameras is worth having; the
  decision that allows a table in is made with the project it comes
  from, not against it.
- **Fitting a grid to the readings.** A grid hides the readings it was
  fitted to, and the maker's software may not draw one.
- **A text format and a parser for tables.** One more format to
  specify, parse, check and fuzz, for a handful of files; Rust source
  is checked by the compiler and read by the same tools as the rest.
- **A build script that discovers table files.** Generated code and a
  build dependency, for a registration of two lines.

## What this decision does not cover

- Per-body facts that the files themselves hold, such as a sensor's
  colour filter phase, a crop's trim or a white level, which are read
  from the files and recorded in the format notes as any other fact,
  not measured outside them.
- A decoder's fixed tables, such as the symbol lists of a compression
  and the substitution tables of a cipher, which no session with a
  camera can measure; they need a decision of their own, which
  [ADR013](adr013-clean-room-and-third-party-code.md) also calls for.
- Tables of lens names, which
  [ADR016](adr016-full-metadata-before-raw-data.md) names among the
  metadata still to read and leaves to their own decision.
- The terms of a table taken from another project in cooperation with
  it: the decision that allows one records that project's agreement,
  the licence, the attribution and how raw-files checks the values it
  did not measure. This record names the condition and does not make
  that decision.
- How a table is used once it exists: which conditions a file must
  meet for a point to be placed from it, and what the placed point
  says of itself, are decided where the mapping is.

## Consequences

- Coverage grows one body at a time, by whoever owns the body. The
  procedure is the contribution: a session, a record, a table, its
  fixtures.
- A body no one with the procedure owns stays without a table, whatever
  its files show; files alone cannot give a layout, since no frame
  among them records a known point.
- Nothing of another project enters with a measured table, so no
  third-party notices file is created by one; a table taken by a
  decision made in cooperation with its project would create it, as
  ADR013 says.
- Every table's error is stated, as a tolerance in the record; a reader
  of a placed point knows how far it may be from where the maker's
  software draws it, and a viewer's box is far larger than that.
- The maker's software is the reference for where a point lies, read
  from its display; a body whose maker's software draws no focus
  points needs another reference, chosen in its record.
- A contributor needs the body and an hour for the frames; measuring
  them needs the maker's software and a day, which the project spends
  for anyone who sends frames. More than copying a table, and the
  reason each table can be trusted.
- Every table that enters was measured or checked by the project on
  the frames, so the project answers for each; a contributor's
  measurement is checked, not taken on trust.
- The records in the notes grow by a section per body, and the tests
  by a fixture per rule each table serves.
