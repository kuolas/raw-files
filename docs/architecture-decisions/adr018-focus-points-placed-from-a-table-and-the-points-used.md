# ADR018: Focus points placed from a table, and the points used

- Status: Accepted; amended by [ADR019](adr019-the-frame-of-a-placed-focus-point-on-a-crop.md),
  which measures a placed point on a crop in the crop window, the
  table's frame less twice CropHiSpeed's origin, in a raw file as in a
  JPEG, where this record took CropHiSpeed's cropped size in a raw file
- Amends: [ADR012](adr012-focus-points-as-the-camera-records-them.md),
  in two points: a point the camera names by number is placed from a
  table of the body's points, which that record left waiting for a
  decision on the tables; and the points the camera used, with the
  area mode, are reported, which that record rejected
- Builds on: [ADR017](adr017-per-body-tables-measured-by-the-project.md),
  per-body tables measured by the project;
  [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md) and
  [ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md), the focus
  point of a JPEG
- Date: 2026-10-26
- Issue: #57

## Context

[ADR012](adr012-focus-points-as-the-camera-records-them.md) reports a
focus point wherever the file records a position, and none for a shot
that records only the number of an autofocus point, such as a Nikon
DSLR's through the viewfinder, until a table of the body's points
exists. [ADR017](adr017-per-body-tables-measured-by-the-project.md)
decides where such tables come from: the project measures them on
frames shot with the body. The first table is the Nikon D600's,
measured on 56 frames
([the record](../formats/nef/autofocus-points.md#2-nikon-d600)). What
those frames and the maintainer's other files show, as counts:

- **A viewfinder shot names its points fully.** Through the viewfinder
  the D600 writes a layout code, the primary point's number and a mask
  of the points used, and a zero frame and area; the mask's bit is the
  point's number less one. A single-point shot sets one bit, the
  primary's, or none; a dynamic-area shot sets one bit too, the point
  chosen, or none; an
  auto-area shot sets between 2 and 16, the primary among them. On 59
  of the D600's 1,374 files the mask is empty beside a number. A
  manual-focus shot writes layout 0, number 0 and an empty mask.
- **The table places a point to a few pixels.** The 39 centres, read
  in the maker's software, are good to about 2 pixels, in the body's
  full-size output frame; every point's box is 164x164 there. A point
  so placed is where the viewfinder's point lies on the picture, not a
  position the camera measured: the camera measured nothing.
- **On a DX crop the point shifts by the crop's origin.** Two frames in
  DX put their points within 2 pixels of the table's position less
  CropHiSpeed's origin, which on the D600 is the centred window's. The
  raw file's CropHiSpeed counts a border that the picture does not
  have, 32 pixels of width and 12 of height on the D600 (3968x2636
  against the 3936x2624 picture), as the raw size of every Nikon body
  without a CropArea does, where ADR012 accepts the same error.
- **A camera JPEG carries the same record as its raw file**: the same
  AFInfo2 and the same CropHiSpeed on all 56 frames, so a JPEG's point
  can be placed by the same table.
- **The in-focus byte of version 0100 says nothing of a viewfinder
  shot.** It is the contrast-detect result: it reads 0 on every
  viewfinder frame, in focus or not, and 1 on the four live-view
  frames, where contrast detection ran.
- **One Model can carry two layouts.** ExifTool's published tables,
  read as documentation, list two layouts for the D780, 51 points
  through the viewfinder and 81 in live view, each with its own layout
  code; the public sample names point 34 in the 51-point layout (file
  3827). A file's layout code tells which grid its number belongs to.
- **The same fields serve a recorded position.** Mirrorless bodies and
  DSLRs in live view write the area mode and, on some versions, a mask
  beside the position they record (files 7735 and 7736 of the public
  samples record a mask alone).

A viewer that draws where the camera focused wants more than the one
point ADR012 gives: on a dynamic-area or auto-area shot, every point
the camera used, with the primary marked; the area mode, to label
them; and, for a body with a table, the whole layout, to draw the grid
the photographer saw. A culler that measures sharpness at the point
must know whether the point is a position the camera measured or a
place looked up in a table, since the two carry different errors and
different meanings. ADR012 rejected exposing the area mode, the point
numbers and the masks because no caller needed them then, and said
`Metadata::focus` holds the camera's record and nothing estimated from
the picture.

## Decision

We will place a focus point that the camera names by number from the
table of the body's points, say where every point came from, and
report the points the camera used, with the primary, the area mode and,
for a body with a table, the body's layout.

- **A named point is placed from the table.** When a file's record has
  a zero frame or area and names a point, the point's centre and box
  are the table's, in the table's frame, the body's full-size output.
  The point says it came from a table: `Focus::origin` is `Table`,
  against `Recorded` for a position the camera measured. Whether focus
  was confirmed is unsaid for a placed point, since version 0100's
  in-focus byte does not speak of it.
- **Only under four conditions**, each counted when it fails: the
  file's FocusMode is not `MANUAL`; the Model has a table; the file's
  layout code equals the table's; and the number is above 0 and within
  the table. Any other file gives no point, as under ADR012.
- **On a crop the point shifts by CropHiSpeed's origin** into the
  crop's picture, with its box kept, on a body whose record confirms
  that shift with the procedure's crop frames, as the D600's does; a
  table whose record does not gives no placed point on a crop. The
  frame becomes the crop's picture: CropHiSpeed's cropped size in a raw
  file, and the JPEG's own size in a JPEG at full image size, the size
  the D600's frames check. A centre that leaves the frame costs that
  point.
- **In a JPEG a placed point takes the path of a recorded one**, with
  the table's frame standing in for the recorded frame, except on a
  crop: a placed point is in full-size output pixels, so it shifts as
  above, where a recorded DX point, in fractions of the picture, is
  scaled by [ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md).
  The aspect-ratio rule of
  [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md) then applies
  to the placed frame unchanged.
- **The points the camera used are reported**, through
  `Metadata::focus_points`: every point, each with its number when the
  file gives one, its centre, its box and whether focus was confirmed
  when the file says; which of them is the primary, the one
  `Metadata::focus` reports; and the area mode. For a placed point the
  list is one point per set bit of the mask whose number the table has,
  in number order, or the primary alone when the mask is empty; a set
  bit the table lacks is skipped and counted, never a reason to give
  none. For a recorded position the list is that one point. On a crop
  every point of the list shifts as the primary does.
- **The area mode is reported in neutral words**, as `AfAreaMode`:
  single point, dynamic area, group area, auto area, tracking, wide
  area, pinpoint, named only for a code a checked file shows, as the
  camera settings are
  ([ADR011](adr011-metadata-from-standard-tags.md)); any other code
  gives no mode.
- **A body with a table gives its layout**, through
  `Metadata::focus_layout`: every point's number, centre and box in the
  table's frame, whatever the shot, so that a viewer can draw the grid;
  a body without a table gives none.
- **`Metadata::focus` keeps its shape and meaning** for every file that
  had a point: a recorded position is reported as before, with origin
  `Recorded`, and `Focus` stays `Copy`. A placed point is still the
  camera's record, read through the table, and nothing estimated from
  the picture: a position found by analysing the pixels is a different
  value with a name of its own, as ADR012 says.

Rejected:

- **A placed point presented as the camera's record, with no origin.**
  A culler could not tell a measured position from a table's estimate,
  and the two differ in error and in meaning.
- **The area mode as the maker's code.** A number that means one thing
  on Nikon and another on Sony, in a type every format shares.
- **The list of points inside `Focus`.** `Focus` would lose `Copy`, and
  every caller that wants only the primary would carry the list.
- **Mapping on the Model alone, without the layout code.** The D780's
  two layouts under one Model would misplace every number of the other
  layout, and
  a body whose later firmware changed its layout would be misread.
- **Clamping a number beyond the table to the centre, or to the last
  point.** A wrong point placed with confidence; none, counted, tells
  the truth.
- **Reporting whether a placed point was in focus from version 0100's
  byte.** It is the contrast-detect result, 0 on every viewfinder
  frame, and would say "not in focus" of every sharp picture.

## What this decision does not cover

- A chosen spot that the camera records without a position, such as
  Sony's flexible spot on a coarse grid: a decision of its own, with
  frames shot on such a body, which adds an origin of its own.
- The shots of a mirrorless body that record a mask alone (the Z
  bodies' auto-area mode): they need a table of that body's points by
  [ADR017](adr017-per-body-tables-measured-by-the-project.md), and are
  then placed by this record.
- The point fields of AFInfo2 versions 0101 and 03xx, documented but
  checked for presence only, and the mask of points in focus that
  version 0101 writes: read once a table for a body that writes those
  versions exists.
- A crop-mode JPEG at a smaller image size than full. The shift is in
  full-size pixels and the D600's frames were shot at full size only;
  such a JPEG gives no placed point until frames at those sizes are
  measured, by this record's rule or another.
- Which bodies have tables, and how one is measured:
  [ADR017](adr017-per-body-tables-measured-by-the-project.md).
- How sharp the picture is at a point, and what a culler does with a
  point that is an estimate: as under ADR012, separate decisions that
  build on this one.

## Consequences

- A Nikon DSLR shot through the viewfinder gives a focus point once its
  body has a table; the D600's 1,374 files give one on 1,339, and none
  on the 35 that name no point, 34 of them through the viewfinder.
  Every other body's files give what they gave, and the format notes
  name each body's reason.
- A placed point carries the table's tolerance, a few pixels, and on a
  raw DX file the border that CropHiSpeed counts, 32 pixels of width on
  the D600, which a caller scaling by that frame carries to the far
  edge; the notes state both. A viewer's box is far larger than either.
- A caller that wants only the one point reads `Metadata::focus` as
  before; one that wants the origin, the list, the mode or the layout
  reads the new getters. Nothing a caller had changes shape.
- The list of a recorded position has one point, so a caller can treat
  every file alike.
- A set bit the table lacks, or a number beyond it, is counted in the
  project's test over a corpus, so that a table that does not match
  its body's files shows as a count, not as a wrong point.
- The area mode names only codes a checked file shows; a new body's
  code gives no mode until a file shows what it means.
- A JPEG of a body with a table gives the point its raw file gives,
  placed on the JPEG's own picture, at full image size; an export cut
  to another shape gives none, as ADR014 says.
