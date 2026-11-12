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
  with the standard that gives it, and the field types.
- [camera-names.md](camera-names.md): how a camera's Make and Model
  texts are read into its maker's and its model's names.
- [sample-files.md](sample-files.md): how a sample file comes in,
  where its expected values come from, and how to check one.

## How a fact's source is marked

A claim in the notes carries a tag that says where it comes from:

- **[P]**: a primary source, such as a specification.
- **[S]**: a secondary source, such as a write-up or a search.
- **[I]**: an inference, to be tested on sample files.
- **[F]**: measured on a named sample file.

Each folder's README says what counts as a primary source for its
format, and how its sample files are cited:
[NEF](nef/README.md#confidence-tags),
[ARW](arw/README.md#confidence-tags) and
[JPEG](jpeg/README.md#confidence-tags). A page beside the folders that
uses the tags uses them in the meaning above. A page that gives the tags
a meaning of its own says so at its top.
