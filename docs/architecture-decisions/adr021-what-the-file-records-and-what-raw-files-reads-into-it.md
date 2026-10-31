# ADR021: What the file records and what raw-files reads into it

- Status: Accepted
- Builds on: [ADR009](adr009-public-api-shape.md), the shape of the
  public API; [ADR011](adr011-metadata-from-standard-tags.md), metadata
  from the standard tags
- Date: 2026-10-30
- Issue: #49

## Context

A camera names itself in two text tags: its maker in Make and its model
in Model. raw-files gives both as the file records them, cleaned as
every text value is
([ADR011](adr011-metadata-from-standard-tags.md)): cut at the first NUL
character and trimmed of spaces at both ends. That text is not what a
person reads. A Nikon Z6III writes `NIKON CORPORATION` and
`NIKON Z6_3`; a Sony α7 II writes `SONY` and `ILCE-7M2`. A program that
shows the camera to a person keeps a table of its own to get from one
to the other, as the culler
[photography-workbench](https://github.com/kuolas/photography-workbench)
does for the makers' names, and #49 asked raw-files to give the names
itself.

The 79 public sample files show five Make texts: `NIKON CORPORATION`,
`NIKON`, `SONY`, `HASSELBLAD` and `Canon`. Their Model texts carry the
maker's name unevenly: `NIKON D850` and `NIKON Z 9`, but `COOLPIX P7800`
under the Make `NIKON`; `Canon PowerShot G7 X`; none at all on Sony's
bodies, such as `ILCE-7M4`, or on Hasselblad's `Lunar`. A name cannot be
had from such text by rule. Writing the maker's word in another case
gives `Nikon Z6_3`, and removing it gives `Z6_3`, where Nikon sells the
body as the Z6III; no rule turns `ILCE-7M2` into α7 II. And a maker's
own spelling is not always in lower case: Nikon writes its lenses'
brand NIKKOR, in capitals.

So a name is something raw-files reads into the recorded text, from a
table of its own, and both have a use: the name for a person, the text
for a program that renames files, compares with another reader or needs
plain ASCII. The same holds for many of the values full metadata will
add ([ADR016](adr016-full-metadata-before-raw-data.md)), such as a lens
that a file records only as an identifier, which a table of lenses
names. The getters need one rule for naming the two, and for which of
them raw-files stores, before those values are built.

The options were:

- the text written in each maker's own case, as #49 first asked;
- the getters `make` and `model` kept for the text as recorded, with
  `maker` and `marketing_name` beside them for the names;
- one getter giving a pair, the text with the name read into it;
- a display adaptor, as `std::path::Path::display` is, that formats the
  text as a name;
- the name under the plain word, and the text under a word that says it
  is the file's.

## Decision

We will give **the value a person reads under the plain word, and the
text as the file records it under `recorded_` and the same word**: a
camera's `make` is its maker, which raw-files reads from the text that
`recorded_make` gives, and its `model` is the name the maker sells it
under, which raw-files reads from the text that `recorded_model` gives.

- **The recorded text is the file's, cleaned and nothing more**, as
  ADR011 reads every text value. raw-files reads nothing into it, so a
  program gets the text another reader of the same file shows.
- **A value with nothing read into it keeps the plain word**, such as a
  serial number or the software that wrote the file: there is no other
  value for the plain word to name. A lens's make and model are named
  `recorded_` as a camera's are, so that the two read alike and the
  plain word stays free for the name a table of lenses gives.
- **A value read from others and from raw-files' own tables alone is
  computed when it is read**, with no field and no `with_…` method of
  its own. Its docs name the method that sets the text it is read from.
  A program's test that sets the recorded text gets the value a file
  with that text gives, and the two can never disagree. A value that
  needs more of the file than the type holds is read when the file is
  opened and stored, with a `with_…` method of its own, as the layout
  of a body's autofocus points is: a body whose files name one of
  several layouts by a code recorded elsewhere needs that code.
- **A value raw-files cannot read is `None`**, never the recorded text
  in its place. A program that shows the text where no name is known
  says so in one call, such as `model().or(recorded_model())`, and can
  tell the two apart.

Rejected:

- **The text in each maker's own case**, as #49 first asked: `NIKON`
  written as `Nikon`, the rest left as recorded. It gives `Nikon Z6_3`
  for a Z6III and `ILCE-7M2` for an α7 II, and a maker's own spelling
  is not always in lower case.
- **`make` and `model` kept for the text, with `maker` and
  `marketing_name` beside them.** `make` and `maker` are too alike to
  tell which is which, and the plain word would go to the text, which
  fewer programs want.
- **A pair, the text with the name read into it.** Every getter would
  return a wrapper that each caller opens to reach either value, and
  values with nothing read into them would either take the wrapper too
  or have a different shape.
- **A display adaptor.** It formats a value but gives none to compare
  or match on: a program could not ask whether a camera is a Nikon
  without comparing text.
- **The recorded text in place of a name raw-files cannot read.** A
  program could not tell a name from a code: `NIKON Z6_3` would look
  like a name.

## What this decision does not cover

- Where each name comes from, and what evidence a table of names needs:
  a decision of its own.
- Every entry a file records, with those raw-files does not name,
  beside the typed values: whether raw-files gives such a view is a
  question for full metadata (#50).
- Which values are read into something of their own: each group of full
  metadata decides its own, by this rule.

## Consequences

- A program reads a camera's maker and name with `make` and `model`,
  and the text as recorded with `recorded_make` and `recorded_model`;
  the getters that gave the text under the plain words before change
  names, which breaks the public API.
- A program's tests set only the recorded text, and get the values
  raw-files reads from it. A test cannot set a name for a body raw-files
  has no name for; it picks a body that has one.
- A name for a body raw-files does not list is `None`, and a program
  shows what it chooses in its place, the recorded text or nothing.
- Each group of full metadata names its values by this rule, so a
  program meets the same two words for every value that has both.
