# ADR022: Names as their makers write them

- Status: Accepted
- Builds on: [ADR013](adr013-clean-room-and-third-party-code.md), the
  clean room; [ADR017](adr017-per-body-tables-measured-by-the-project.md),
  per-body tables measured by the project;
  [ADR021](adr021-what-the-file-records-and-what-raw-files-reads-into-it.md),
  what the file records and what raw-files reads into it
- Date: 2026-11-02
- Issue: #49

## Context

raw-files gives a camera's maker and the name it is sold under beside
the text the file records, reading the one from the other through
tables of its own
([ADR021](adr021-what-the-file-records-and-what-raw-files-reads-into-it.md)).
No file holds the name: a Sony α7 II records `ILCE-7M2`, and a Nikon
Z6III records `NIKON Z6_3`. Each entry of such a table joins two facts:
the text a body writes, and the name its maker sells it under.

The text is in the files, and the project reads it from the public
samples, the fixtures and my own files, as it reads every other value.
The name has to come from outside the files. It can be had from the
maker's own documents, its manuals and its product pages; from the
tables other readers keep, such as ExifTool's list of Sony's model
numbers; or from retailers, reviews and encyclopedias.

The makers' own documents do not always agree with each other. Sony's
photo gallery writes α7 IV and α9 III, and the Help Guide of the same
bodies α7IV and α9III; the Help Guides of some older bodies give the
name only as a logo. Nikon writes its older Z bodies with a space,
Z 9, Z 6, Z f, in their manuals and on its product pages alike, and
its newer ones without, Z6III, Z50II; a regulatory notice in the Z 9's
manual has Z9. A rule that wrote every Nikon name alike would give
names Nikon does not use for some of its bodies.

[ADR017](adr017-per-body-tables-measured-by-the-project.md) builds every
per-body table from the project's own measurements, and takes none from
another project. A table of names is a per-body table, but nothing in it
is measured: its entries are read from files and from documents. ADR017
already lets a name that published documentation gives be read as
documentation, and this record builds on it for tables whose values are
names, without changing what it decides for tables whose values are
measured.

## Decision

We will take each **name as its maker writes it in its own published
documents**, and enter a body in the table only with **a file that
shows its text and a document of its maker that gives its name**, both
recorded in the format notes before the code, on a page of their own
([camera-names.md](../formats/camera-names.md)).

- **The file.** The Make and Model texts an entry is looked up by come
  from a file the project has read: a public sample, a fixture, or my
  own files, cited by count. The notes name the file.
- **The document.** The name comes from the maker's own published
  documents: a product page, a manual or a guide. The notes give its
  title, its edition or address, the page or place where the name
  stands, and the date it was read. Where the maker's documents differ,
  the name its marketing gives, on its product pages, is taken, and the
  notes record the other spellings beside it.
- **The spelling as written.** The name keeps the maker's case, spaces
  and letters: NIKKOR in capitals, Z 9 with its space and Z6III without
  one, α7 II with the Greek letter α. Nothing is recased or made
  uniform across a maker's line. The brand is not part of the name: the
  camera's text to show joins the two
  ([ADR021](adr021-what-the-file-records-and-what-raw-files-reads-into-it.md)).
- **The maker by its whole text.** A maker is recognized from a Make
  text a file has shown, matched whole and with its case, as the white
  balance texts are
  ([ADR011](adr011-metadata-from-standard-tags.md)); its name, such as
  Nikon or Sony, comes from the maker's own documents, as a body's
  does.
- **The body by its maker and its text.** A name is looked up by the
  maker and the recorded Model text together, exactly, so that a Model
  text two makers might both write cannot give the other's name.
- **No other project's table.** No name is taken from another reader's
  table, whatever its license, nor from a retailer, a review or an
  encyclopedia. Other readers may be run on a file to see its Model
  text, as [ADR013](adr013-clean-room-and-third-party-code.md) allows,
  never to take a name.
- **A body not listed has no name**, and a body whose maker's document
  cannot be found stays out of the table; the notes name it and why.

Rejected:

- **Names from other readers' tables**, such as ExifTool's list of
  Sony's model numbers. Their selection and arrangement are another
  project's work, and a name taken from them could not be checked
  against the maker by a reader of the notes.
- **Names from retailers, reviews or encyclopedias.** They are not the
  maker's, and differ from one another.
- **One spelling for a maker's whole line**, such as every Nikon Z name
  without a space. The line would read evenly, but some of its names
  would be ones the maker does not use.
- **The maker's manual over its product pages** where they differ. A
  manual's cover may give the name only as a logo, and Sony's product
  pages write the names as its marketing does.

## What this decision does not cover

- Lens names, which a table of lenses is to give under the decisions of
  full metadata ([ADR016](adr016-full-metadata-before-raw-data.md)).
- Who built a body sold under another maker's name, such as
  Hasselblad's Lunar: a fact for full metadata, from the files.
- Names that differ by region for one Model text; a body that writes a
  different Model text in each region gets an entry for each text.
- The tables of measured values that
  [ADR017](adr017-per-body-tables-measured-by-the-project.md) decides.
- Which maker's reader opens a file, or reads a camera JPEG's
  MakerNote. The formats decide it by a looser test of the Make, one
  that starts with a maker's name in any case, so a file whose Make is
  `Nikon` is read as a Nikon file and has no maker here; such a text
  becomes a maker's only when a file shows it, under the rule above.

## Consequences

- The table grows one body at a time, each with a file and a maker's
  document; the notes are the audit trail, and a reader can open the
  document an entry cites.
- A body with no file among the samples, the fixtures and my own files
  has no name, however well known it is.
- Some names are not ASCII, such as Sony's α; a program that needs
  ASCII uses the recorded text.
- A maker that renames a body, or writes its name differently later,
  makes an entry out of date; the date each document was read shows
  when, and a correction changes the entry and its notes.
