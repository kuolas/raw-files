# Format notes

What raw-files knows about the files it reads, and where each fact
comes from. The code is written from these pages
([ADR013](../architecture-decisions/adr013-clean-room-and-third-party-code.md)).

## The formats

- [nef/](nef/README.md): Nikon's NEF raw files.
- [arw/](arw/README.md): Sony's ARW raw files, and their relatives SRF,
  SR2 and ARQ.
- [jpeg/](jpeg/README.md): the JPEG files cameras write, with their
  Exif and MPF data.

Each folder's README lists its pages and says where to start.

## The pages all formats share

- [tag-names.md](tag-names.md): every name raw-files gives a TIFF tag,
  with the document that gives it, and the field types.
- [camera-names.md](camera-names.md): how a camera's Make and Model
  texts are read into its maker's and its model's names.
- [sample-files.md](sample-files.md): how a sample file comes in,
  where its expected values come from, and how to check one.

## How a fact's source is marked

A claim in the notes carries a tag that says where it comes from:

- **[P]**: stated in a primary source:
  - a specification, cited by section, table or figure and printed page;
  - a camera maker's own documentation;
  - a project's, publisher's or site's own pages, for what they say of
    it: its license, terms, releases, contents or security record; and
    the NVD.
- **[S]**: secondary:
  - another program's documentation, source code or output;
  - a person's write-up, or an archive's format registry;
  - a forum post, issue or review, or a page read only through a search
    extract.
- **[I]**: inferred, derived by arithmetic, recalled without a source,
  taken from a single unconfirmed report, or a recommendation. Treat it
  as a hypothesis to test on sample files.
- **[F]**: measured on a named sample file.

Where a sample contradicts a source, the pages follow the sample and
record the disagreement.

The READMEs of the three folders give the tags in these same words:
[NEF](nef/README.md#confidence-tags),
[ARW](arw/README.md#confidence-tags) and
[JPEG](jpeg/README.md#confidence-tags). Each also says how its sample
files are cited. A page beside the folders that uses the tags uses them
in this meaning. A page that gives the tags a meaning of its own says so
at its top.
