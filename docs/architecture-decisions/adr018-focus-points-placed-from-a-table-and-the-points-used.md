# ADR018: Focus points placed from a table, and the points used

- Status: Accepted
- Builds on: [ADR012](adr012-focus-points-as-the-camera-records-them.md),
  focus points as the camera records them;
  [ADR017](adr017-per-body-tables-measured-by-the-project.md), per-body
  tables measured by the project;
  [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md) and
  [ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md), the focus
  point of a JPEG
- Date: 2026-10-26
- Revision: 2, 2026-11-03
- Issue: #57; each crop's and JPEG size's window #62

## Context

[ADR012](adr012-focus-points-as-the-camera-records-them.md) reports a
focus point wherever the file records a position. A shot that records
only the number of an autofocus point, such as a Nikon DSLR's through
the viewfinder, gets none until a table of the body's points exists.
[ADR017](adr017-per-body-tables-measured-by-the-project.md) decides
that the project measures such tables on frames shot with the body. The
first table is the Nikon D600's, measured on frames I shot with my own
D600 ([the record](../formats/nef/autofocus-points.md#2-nikon-d600)).

A viewer that draws where the camera focused wants more than one point.
On a dynamic-area or auto-area shot, it wants every point the camera
used, with the primary marked, and the area mode to label them. For a
body with a table, it wants the whole layout, to draw the grid the
photographer saw. A culler that measures sharpness at the point must
know whether the camera measured the point or a table placed it. The
two carry different errors and different meanings. ADR012 first left
out the area mode, the point numbers and the masks, because no caller
needed them then.

The D600's frames show, as the record details:

- Through the viewfinder the body writes a layout code, the primary
  point's number and a mask of the points used, beside a zero frame and
  area. A camera JPEG carries the same record as its raw file
  ([§2.2](../formats/nef/autofocus-points.md#22-what-the-body-writes)).
- The table places a point to about 2 pixels, in the body's full-size
  output frame, 6016x4016. Such a point is where the viewfinder's point
  lies on the picture; the camera measured nothing
  ([§2.5](../formats/nef/autofocus-points.md#25-how-the-table-was-read-and-its-tolerance)).
- On a DX crop, the point shifts by CropHiSpeed's origin into a window
  of the frame, 3936x2624, the size of the crop's previews and JPEG. No
  tag records that size: CropHiSpeed counts the sensor's pixels, with a
  border the picture does not have. And the frames cannot tell whether
  the window follows from the origin or is centered in the output: on
  the D600 both hold
  ([§2.6](../formats/nef/autofocus-points.md#26-the-control-frames)).
- The Medium JPEG, 4512x3008, shows a centered window of the frame, a
  few pixels smaller, scaled evenly. No tag records it either.
  In the whole frame, the JPEG reader's shape rule
  ([ADR014](adr014-jpeg-files-as-a-format-of-their-own.md)) refuses it,
  so a Medium JPEG gave no point
  ([§2.9](../formats/nef/autofocus-points.md#29-the-jpeg-sizes)).

So on a crop, and in some JPEG sizes, a placed point needs a window
that only the body's record measures.

## Decision

We will place a focus point that the camera names by number from the
table of the body's points, and say where every point came from. We
will report the points the camera used, with the primary, the area mode
and, for a body with a table, the body's layout. On a crop, and in a JPEG of
a size whose picture is a window of the frame, a placed point is
measured in that window, which the body's table keeps.

- **A named point is placed from the table.** When a file's record has
  a zero frame or area and names a point, the point's center and box
  are the table's, in the table's frame, the body's full-size output.
  `Focus::origin` says where it came from: `Table`, against `Recorded`
  for a position the camera measured. Whether focus was confirmed is
  unsaid for a placed point, since version 0100's in-focus byte does
  not speak of it.
- **Only under four conditions**, each counted when it fails: the
  file's FocusMode is not `MANUAL`; the Model has a table; the file's
  layout code equals the table's; and the number is above 0 and within
  the table. Any other file gives no point, as under ADR012.
- **The table keeps each window its record measures**: each confirmed
  crop's window, beside the crop's setting, and the window of each JPEG
  size whose picture is a part of the frame, beside that size as the
  frame header gives it.
  - A crop's window is the size of the crop's picture in output pixels,
    as the crop frames' full-size previews and JPEGs measure it.
  - A JPEG size's window is the part of the frame its picture shows, a
    whole number of frame pixels on each side, as RAW+JPEG pairs at
    that size measure it.
  - Every window lies centered in the frame, at half the frame less the
    window on each axis, as the record's frames confirm. The table's
    tests refuse a window that cannot be centered to the pixel or does
    not fit the frame. They refuse a JPEG size's window that has not
    that size's shape by ADR014's rule.
- **On a crop the point moves into the crop's window.** The window is a
  fact of the body, as the shift is. So a point is placed on a crop
  only on a body whose record confirms the shift with the procedure's
  crop frames, and whose previews and JPEGs show the window's size.
  Even then, the file's CropHiSpeed origin must be that window's
  centered origin. The point's center then moves by that origin and its
  box is kept, in a raw file as in a JPEG. A crop the table does not
  confirm, or another origin, gives no placed point. So the origin is
  checked against a measured size instead of being its only source.
- **A JPEG of a size the table lists**, when it is no crop, has its
  points measured in that size's window, moved by its centered origin
  as on a crop. A JPEG of a size the table does not list, and a raw
  file, take the table's frame. A crop-mode JPEG is placed on its crop
  alone, whatever its size, since a size's window is measured
  uncropped.
- **A point the window leaves out is dropped**: one whose center leaves
  the window, or whose box crosses the window's edge, as every point's
  box must fit inside its frame. The primary dropped costs the focus
  point.
- **In a JPEG a placed point takes the path of a recorded one.** The
  table's frame stands in for the recorded frame, or the window on a
  crop and in a listed size. A placed point is in output pixels, so it
  moves by the window's origin; a recorded DX point, in fractions of
  the picture, is scaled by
  [ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md). On a crop, a
  JPEG gives a placed point only when its own size equals the window,
  so a crop-mode JPEG at a smaller size gives none, counted. ADR014's
  aspect-ratio rule then applies to the placed frame, or the window,
  unchanged.
- **The D600's table** keeps the DX crop's window and the Medium
  JPEG's, each centered to the pixel nearest the measured one
  ([§2.6](../formats/nef/autofocus-points.md#26-the-control-frames),
  [§2.9](../formats/nef/autofocus-points.md#29-the-jpeg-sizes)).
- **The points the camera used are reported**, through
  `Metadata::focus_points`: each with its number when the file gives
  one, its center, its box, and whether focus was confirmed when the
  file says. The list marks the primary, the one `Metadata::focus`
  reports, and gives the area mode.
  - For a placed point, the list holds one point per set bit of the
    mask whose number the table has, in number order, or the primary
    alone when the mask is empty. A set bit the table lacks is skipped
    and counted, never a reason to give none.
  - For a recorded position, the list is that one point.
  - On a crop, and in a JPEG size's window, every point of the list
    shifts as the primary does.
- **The area mode is reported in neutral words**, as `FocusAreaMode`:
  single point, dynamic area, group area, auto area, tracking, wide
  area, pinpoint. A code is named only once a checked file shows it, as
  the camera settings' codes are
  ([ADR011](adr011-metadata-from-standard-tags.md)). Any other code
  gives no mode.
- **A body with a table gives its layout**, through
  `Metadata::focus_layout`: every point's number, center and box in the
  table's frame, whatever the shot, so that a viewer can draw the grid.
  A body without a table gives none.
- **`Metadata::focus` keeps its shape and meaning** for every file that
  had a point. A recorded position is reported as before, with origin
  `Recorded`, and `Focus` stays `Copy`. A placed point is still the
  camera's record, read through the table, and nothing estimated from
  the picture. A position found by analyzing the pixels is a different
  value with a name of its own, as ADR012 says.

Rejected:

- **A placed point presented as the camera's record, with no origin.**
  A culler could not tell a measured position from a table's estimate,
  and the two differ in error and in meaning.
- **The area mode as the maker's code.** A number that means one thing
  on Nikon and another on Sony, in a type every format shares.
- **The list of points inside `Focus`.** `Focus` would lose `Copy`, and
  every caller that wants only the primary would carry the list.
- **Mapping on the Model alone, without the layout code.** One Model
  can carry two layouts, as the D780 does through the viewfinder and in
  live view, and every number of the other layout would be misplaced. A
  body whose later firmware changed its layout would be misread too.
- **Clamping a number beyond the table to the center, or to the last
  point.** That is a wrong point placed with confidence; none, counted,
  tells the truth.
- **Reporting whether a placed point was in focus from version 0100's
  byte.** It is the contrast-detect result, 0 on every viewfinder
  frame, and would say "not in focus" of every sharp picture.
- **CropHiSpeed's cropped size as the frame on a crop**, with the
  JPEG's own size in a JPEG, as this record first had it. The border
  that size counts beyond the picture was taken for an error ADR012
  already accepts on bodies without a CropArea, since no raw file was
  known to give the picture's size. But the border is the stored raw
  data's, which the picture and its previews do not have. A point
  scaled onto the NEF's own preview lands up to about 25 pixels off, and
  a raw file and its JPEG give the same point in two frames. The
  uncropped file's frame would be the output's, while the cropped
  file's counted the stored raw data's border.
- **CropHiSpeed's cropped size less a border measured per body.** One
  more number for each body's record to measure and keep. It was first
  rejected because the window follows from the origin the record
  confirms. That reason fell once the window was measured and the
  origin checked against it; the measured window is kept instead.
- **The window from CropHiSpeed's origin alone**, the frame less twice
  the origin, as this record next had it. It holds on the D600, but one
  tag value would decide the shift and the frame unchecked. A raw file
  whose origin is not centered, as the D7200's 1.3x stills record,
  would give a point in a frame of the wrong size without a sign.
- **Half the frame less CropHiSpeed's cropped size.** That size counts
  the sensor's border, so on the D600 the origin misses the frames' by
  16 and 6 pixels.
- **The window from the size of the file's own full-size preview.** It
  needs no table change. But the MakerNote's reader does not see the
  previews, a body may write no full-size preview, and a JPEG has its
  own check already.
- **The Medium JPEG's window as 6012x4008 at (2, 4).** It is exactly
  3:2, the JPEG's own shape, but wider and taller than measured. It
  places a point about nine times further off across than the nearest
  whole window, and 1.3 times down.
- **A window of fractional pixels**, the measurement as it is. The
  table, and the frame of a focus point and of a list of points, would
  need sizes and origins that are not whole numbers. The public API
  measures in whole pixels, and the gain is far below the table's
  tolerance.
- **A rule for every JPEG of exactly 3:2**, mapping it to the centered
  3:2 window of the frame, as a crop-mode JPEG's window is found from
  its shape ([ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md)).
  It needs no measurement. But on the D600 that window is the frame's
  whole width, wider than the picture, so a point lands up to about 2
  Medium pixels off. And it would give a point to any export cut to 3:2
  from a camera's JPEG, which ADR014 refuses.
- **Leaving the Medium JPEG without a point**, while the NEF of the
  same shot gives one.

## What this decision does not cover

- A chosen spot that the camera records without a position, such as
  Sony's flexible spot on a coarse grid. It needs a decision of its
  own, with frames shot on such a body, and adds an origin of its own.
- The shots of a mirrorless body that record a mask alone, such as the
  Z bodies' auto-area mode. They need a table of that body's points by
  [ADR017](adr017-per-body-tables-measured-by-the-project.md), and are
  then placed by this record.
- The point fields of AFInfo2 versions 0101 and 03xx, documented but
  checked for presence only, and the mask of points in focus that
  version 0101 writes. They are read once a table exists for a body
  that writes those versions.
- A crop-mode JPEG smaller than the crop's window, such as a DX JPEG at
  the Medium size. Its picture is a part of the crop's window, and the
  D600's crop frames were shot at full size only. Such a JPEG gives no
  placed point until frames in that crop at that size are measured, by
  this record's rule or another.
- A crop or a JPEG size whose picture is not centered in the output, or
  a JPEG size not scaled evenly: none is known. Its record would
  measure where the window lies, and a decision would say how the table
  keeps that.
- The crops that keep one side, 1:1 and 16:9. Their recorded positions
  are moved by CropArea or the JPEG's shape
  ([ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md)), not placed
  from a table.
- A point the file records as a position, which is not placed from a
  table. Its frame is the one the file records, under
  [ADR012](adr012-focus-points-as-the-camera-records-them.md), ADR014
  and ADR015.
- The layout of the body's points, which stays in the table's frame, on
  a crop and on a JPEG size's window alike.
- Which bodies have tables, which crops and sizes a body's record must
  shoot, and how a table and a window are measured: the procedure of
  the NEF notes, under
  [ADR017](adr017-per-body-tables-measured-by-the-project.md).
- How sharp the picture is at a point, and what a culler does with a
  point that is an estimate. As under ADR012, these are separate
  decisions that build on this one.

## Consequences

- A Nikon DSLR shot through the viewfinder gives a focus point once its
  body has a table. Every other body's files give what they gave, and
  the format notes name each body's reason
  ([autofocus-points.md §3](../formats/nef/autofocus-points.md#3-bodies-without-a-table)).
- A placed point carries the table's tolerance, a few pixels, which the
  notes state. A viewer's box is far larger.
- A caller that wants only the one point reads `Metadata::focus` as
  before. One that wants the origin, the list, the mode or the layout
  reads the new getters. Nothing a caller had changes shape.
- The list of a recorded position has one point, so a caller can treat
  every file alike.
- A set bit the table lacks, or a number beyond it, is counted in the
  project's test over a corpus. So a table that does not match its
  body's files shows as a count, not as a wrong point.
- A D600 DX NEF and its JPEG give the same placed point in the same
  frame, the size of the NEF's full-size preview. A point scaled onto
  that preview lands where the table puts it.
- Every D600 DX frame records the window's centered origin, so the
  check against the window costs no DX file its point.
- A raw file whose CropHiSpeed origin differs from the window's, by a
  single pixel or by a crop placed elsewhere, gives no placed point. It
  does not get one in a frame no frame of the record checked.
- A crop-mode JPEG at a smaller size is told from a full-size one by
  its size against the window, with no border to know.
- On a crop, as on a full frame, a placed point's frame is the
  output's, not the raw file's image size. On a body without a
  CropArea, that image size counts the stored raw data's border.
  `Focus`'s documentation already tells a caller not to assume the two
  are equal.
- A JPEG of a body with a table gives the point its raw file gives,
  placed on the JPEG's own picture. It does so at full image size, at a
  smaller size whose picture is the whole frame scaled, and at a size
  whose window the table keeps. An export cut to another shape gives none, as
  ADR014 says.
- A D600 Medium JPEG shot through the viewfinder gives its points in
  the window, where the picture shows them. A NEF and its Medium JPEG
  give the same point in two frames, the table's and the window, where
  a DX NEF and its JPEG give it in one. A caller compares points across
  files in their own frames.
- The JPEG reader's shape rule is unchanged: a window passes it, and an
  export cut to another shape still gives none.
- An export scaled or cut to exactly a size the table lists cannot be
  told from a camera JPEG of that size. Its points are placed in the
  window, where the shape rule alone would give it none. A scaled
  export's points land a few pixels off, a cut one's by as much as the
  cut moved the picture
  ([§2.9](../formats/nef/autofocus-points.md#29-the-jpeg-sizes)). A
  cut of the frame's own shape is misplaced so already.
- An export scaled from such a JPEG to another size takes the whole
  frame, since a window is kept for its own size alone. A tall one has
  not the whole frame's shape and gives no point. A small one passes
  the rule's one pixel of rounding, and its point lands under a pixel
  off in the whole frame. The notes give the height between the two
  ([§2.9](../formats/nef/autofocus-points.md#29-the-jpeg-sizes)).
- Each confirmed crop costs a body's record one more measured size,
  that of the crop frames' previews or JPEGs, which the procedure
  already shoots. Each smaller JPEG size costs RAW+JPEG pairs at that
  size, and the fit of each JPEG onto its raw file's full-size preview.
  A size whose picture is the whole frame scaled needs no entry.
