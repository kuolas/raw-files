# ADR013: Clean-room implementation and third-party code

- Status: Accepted
- Builds on: [ADR002](adr002-license-mit-or-apache-2.md), the license;
  [ADR006](adr006-ai-assisted-contributions.md), who is accountable for
  a contribution; [ADR007](adr007-pure-rust-cross-platform.md), pure Rust
- Date: 2026-10-20

This record states how the project reads the licenses it meets. It is not
legal advice.

## Context

Camera RAW formats are not published by their makers. What is known of
them lives in two kinds of place: documentation written by people who
studied the files, such as ExifTool's tag tables, exiv2's MakerNote pages
and the community notes on NEF; and the source code of the programs that
read them, such as dcraw, LibRaw, rawspeed, rawler, ExifTool and exiv2.
Most of that source code is under a copyleft license, the GPL or the LGPL,
some under two licenses at the reader's choice, some under terms of its
own, and some under none. The format notes in [docs/formats](../formats/)
record what raw-files has learnt from these sources, each fact with its
source and how well it is confirmed, and their license pages
([NEF](../formats/nef/licences-and-references.md),
[ARW](../formats/arw/licences-and-references.md)) set out how each source
may be used, with the caution that they are a project's reading of the
licenses, not legal advice.

raw-files is licensed MIT OR Apache-2.0
([ADR002](adr002-license-mit-or-apache-2.md)). As the format notes read
the licenses, a program under those terms cannot take code from a GPL or
LGPL project: copying it, and also translating it into another language,
makes a derivative work, which may be distributed only under the copyleft
terms. Code under a permissive license can be taken, with the notice and
attribution that most such licenses require.

As the notes also read it, copyright protects expression, not facts. The
number of a tag, the offset of a field, the meaning of a flag value and
the order of fields in a structure are facts about a format, which anyone
may state. Source code, its comments and structure, the prose of
documentation and the selection and arrangement of a large table are
expression. A project can therefore learn the facts of a format from any
source it is free to read and write its own code from them, as long as it
writes the code itself. A source that binds its reader, such as a maker's
SDK under a non-disclosure agreement, is not free to read for this
purpose, and the notes say so where they meet one.

A clean-room implementation is one built that way: from facts recorded in
the project's own words, by people who write the code from that record
rather than from another project's code.

Programs such as ExifTool and exiv2 can also be run on the same files to
show what they read, and sample archives publish such output for the
files they host. That output states, in the program's own words, what the
program found in a file. Running a program, or reading what it printed,
is not copying it. A camera's firmware and a maker's own software hold
the formats too, but only as compiled code, and some makers' terms forbid
taking them apart.

The data tables a decoder needs, such as the Huffman symbol lists of NEF's
compression and the substitution tables of its encrypted blocks, are
published only in those decoders' source and are neither plainly facts nor
plainly expression. The format notes call for a decision of their own
before any such table is committed.

## Decision

We will build raw-files as a clean-room implementation.

- **Facts come from documentation, from sample files and from what other
  programs report, and are recorded before they are coded.** A fact about
  a format is learnt from a published description, from reading files,
  from another program's output, or from reading another project's source
  code, and written into the format notes in our own words, with its
  source and a confidence tag. Where a fact can be checked on a sample
  file, it is, and the file is named. The code is written from the notes.
  A fact learnt from documentation may be coded before a file confirms it,
  and the notes say so; a fact learnt only from another project's source
  code is coded once a file confirms it.
- **No code, comment or table is copied or translated from another
  project's source code**, whatever its license: copyleft, dual, custom or
  none. Such source is read only to learn a fact or to settle a
  disagreement between sources, and the fact then goes through the notes
  and a sample file like any other. Where practical, the person who reads
  such source is not the person who writes the raw-files code for the same
  part; the project is small, so this separation is a practice, not a
  requirement. An assistant that has read such source in a session counts
  as a reader of it, and writes from the notes like anyone else. The
  contributor who submits the work is accountable for it, as
  [ADR006](adr006-ai-assisted-contributions.md) says.
- **No program is taken apart.** Camera firmware, a maker's software and
  any other program are never disassembled, decompiled or otherwise read
  as compiled code to learn a format. Observing a program from outside,
  by giving it inputs and reading its outputs, is allowed, and the method
  is recorded in the notes so that a reader can repeat it.
- **Permissively licensed code enters only by a recorded decision, with
  attribution.** Code under the MIT, BSD, Apache-2.0 or a like license may
  be copied into the repository when a decision recorded in an ADR, or in
  an issue that the notices entry links, says so, with its license notice
  and author kept in a third-party notices file at the repository root
  and a comment at the code saying where it came from. Nothing is copied
  without that, however small. A Cargo dependency is not code copied in;
  it stays under its own license, and which licenses a dependency may
  have is [ADR002](adr002-license-mit-or-apache-2.md)'s decision.
- **Third-party programs are run, and their output read, only as a source
  to compare with.** ExifTool, exiv2 and like tools may be run on files,
  and their published output for a named file may be read, to see what
  another reader found in that file. What they report is a source like
  documentation, recorded under the notes' tags and confirmed on the
  file's bytes where the file is available; the words they print are not
  copied. Nothing of theirs ships with raw-files and nothing in the build,
  the tests or CI depends on them: the crate has no dependencies, and any
  it takes follow [ADR002](adr002-license-mit-or-apache-2.md) and
  [ADR007](adr007-pure-rust-cross-platform.md). A comparison is a step a
  person takes and records, not a check the suite runs.
- **Prose says how a source was used.** A commit message, a pull request,
  a doc page or a comment that names another project as a source says
  whether its documentation was read, its source code read to settle a
  fact, a file confirmed the fact, or the program was run as a reader to
  compare with.

Rejected:

- **Translating LGPL code into Rust.** A translation is a derivative work;
  the LGPL permits linking to the library as it is, not copying its code
  into another work.
- **Copying permissive code into the repository without a record.** A
  reader could not tell our code from the author's, and the license's
  notice requirement would go unmet.
- **Running ExifTool or exiv2 in the test suite or CI as a cross-check.**
  Their licenses would allow it, but every contributor and every CI runner
  would need the tool installed, and the tables of expected values would
  rest on another program's output rather than on the files. The samples
  are the evidence; a comparison with another reader is extra assurance,
  recorded where it was made.

## What this decision does not cover

Whether and how the fixed data tables a decoder needs may enter the crate,
where per-body tables such as autofocus-point layouts may come from, and
whether dcraw's own terms, which the notes read as freeing most of its
code, would allow more than reading, are decisions of their own. They
build on this one and do not change it.

## Consequences

- A fact takes longer to establish: a source, our own record of it, and a
  sample file where one exists. The format notes are the audit trail, and
  a reviewer can ask of any offset in the readers where it is recorded and
  which file confirmed it.
- raw-files may lag other readers on a body or a field for which no sample
  file is available, since a fact read in another project's source alone
  is not coded until a file confirms it.
- The repository holds no third-party code, so there is no notices file.
  The first permissively licensed code that enters creates it, together
  with a check that every such file has its entry, as the sample list has
  a test that keeps it and the tables in step.
- Running a third-party program against the samples, as was done when
  checking the focus points of the Nikon samples, stays a recorded step in
  a commit message or the notes; it adds assurance without adding a
  dependency.
- Every mention of another project as a source says how it was used, so
  that a reader of the public repository is not left to guess whether code
  was copied.
- The pull request template asks how any third-party source was used, so
  that the question is put to every contributor.
