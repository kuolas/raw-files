# ADR016: Full metadata before raw data

- Status: Accepted
- Builds on: [ADR010](adr010-decoding-ends-at-a-linear-image.md),
  decoding ends at a linear image;
  [ADR011](adr011-metadata-from-standard-tags.md), metadata from the
  standard tags; [ADR013](adr013-clean-room-and-third-party-code.md),
  the clean room
- Date: 2026-10-25
- Issue: #50

## Context

raw-files reads the metadata a viewer or a file renamer needs beside a
preview: the capture time, the camera and lens, the exposure, the sizes,
the focus point, and the main camera settings, such as the white balance
and the exposure program
([ADR011](adr011-metadata-from-standard-tags.md)). A camera maker's own
software shows much more. On a D600 frame, Nikon's NX Studio, run on the
file as a reader to compare with, lists about 45 metadata fields beyond
the file's name and size, of which raw-files gives about 18, and an XMP
and IPTC tab of about 35 more, of which it gives none. Sony's Imaging
Edge Desktop shows the same kinds of fields for an ARW file. A viewer's
information panel, or a catalogue that sorts and searches by these
fields, has to read the rest with a second library beside raw-files.

The rest falls into seven groups:

1. more capture settings, such as the focus mode, the AF-area mode,
   vibration reduction, the flash unit and its compensation, and the
   image quality;
2. how the camera processed its JPEG: Nikon's Picture Control, Sony's
   Creative Style, the dynamic range and noise reduction settings, and
   the scene modes many consumer bodies record only in their MakerNote;
3. the lens by name, which some bodies record only as an identifier;
4. authorship: the artist, the copyright and the comments;
5. the location, from the GPS tags;
6. XMP and IPTC: the rating, label, keywords, title and description a
   photographer or a program adds, inside the file or in a sidecar file
   beside it;
7. what a maker records only in enciphered or encrypted blocks, such as
   the shutter a Sony body used, whether it shot in APS-C mode, and the
   lens data in Nikon's encrypted records.

Groups 1 to 6 sit in places raw-files already reads, or in standard
blocks beside them: the TIFF and Exif tags, the plain MakerNote tags
and records, the GPS directory, and the XMP and IPTC blocks. Group 7
does not: [ADR011](adr011-metadata-from-standard-tags.md) reads nothing
deciphered or decrypted, and
[ADR013](adr013-clean-room-and-third-party-code.md) lets a fixed table,
such as a cipher's, enter the code only once where it may come from is
decided.

The [roadmap](../ROADMAP.md) had raw sensor data next after the focus
points, and then the developed image that
[ADR010](adr010-decoding-ends-at-a-linear-image.md) ends at. A program
that only shows, sorts and searches pictures needs none of that, and
needs all of the metadata.

## Decision

We will read **every metadata value a file of a format raw-files reads
records**, plain, enciphered or encrypted, as a roadmap milestone of its
own, "Full metadata", after the focus points and before raw sensor data.
NEF and ARW files, and the camera JPEGs that carry the same blocks, are
its first files.

- **Every value follows the rules of the camera settings**
  ([ADR011](adr011-metadata-from-standard-tags.md)): the standard tags
  first, then the maker's MakerNote; each value from a tag of an exact
  type and count; a damaged value costs only itself; a code is named only
  once a standard or a measured file shows what it means; every fact is
  recorded in the format notes, from the bytes of a file, before any code
  ([ADR013](adr013-clean-room-and-third-party-code.md)).
- **The enciphered and encrypted blocks are part of the goal.** What
  only they record is read once a decision of its own, which amends
  [ADR011](adr011-metadata-from-standard-tags.md), allows it. That
  decision first settles where each cipher's or key's table comes from,
  under [ADR013](adr013-clean-room-and-third-party-code.md), and how a
  value read through it is checked against the files. Until then such a
  value is `None`, and the format notes say where it is kept and which
  files lack it.
- **Each group is planned on its own**, with an issue of its own and the
  questions it raises settled there.

Rejected:

- **Raw sensor data first**, as the roadmap had it. A program that
  shows, sorts and searches pictures would keep a second reader for the
  metadata until both raw milestones were done, and the metadata is read
  by the readers raw-files already has.
- **The plain values only, leaving the enciphered and encrypted blocks
  out for good.** A Sony body's shutter and its APS-C mode, and the lens
  of many Nikon files, would never be read, and a program would keep a
  second reader for them.
- **Leaving the rest to other libraries.** That is the second reader the
  milestone is meant to remove.

## What this decision does not cover

- How each group enters the API: whether presets that only one maker has
  keep that maker's names or get neutral ones, whether how the camera
  processed its JPEG sits beside the camera settings or in a record of
  its own, and whether opening a file also reads an XMP sidecar beside
  it or the caller hands it in. Each group's issue settles its own.
- How an enciphered or encrypted block is read: that is the decision the
  second point above calls for.
- The order of the groups within the milestone, and the order of the
  formats after NEF and ARW, which the roadmap keeps.
- Anything about raw sensor data and the developed image, which
  [ADR010](adr010-decoding-ends-at-a-linear-image.md) decides.

## Consequences

- Raw sensor data and the developed image come after this milestone.
  The roadmap gains "Full metadata" between the focus points and raw
  sensor data, and its later milestones move down by one.
- A program can drop its second metadata reader group by group, as each
  lands.
- The API grows by many values, each optional, which can be added
  without breaking callers.
- The values only enciphered or encrypted blocks record stay `None`
  until the decision on reading them allows them.
- Some values name a person or a place: the artist, the copyright, the
  comments and the GPS position. The API gives them as recorded. The
  local corpus test prints counts only, never such a value.
- Each group's reads are measured against what opening a file costs
  ([ADR011](adr011-metadata-from-standard-tags.md) reads the metadata
  when the file is opened, near its start); a block that lies far into
  the file, as an XMP packet may, is a question for that group's issue.
