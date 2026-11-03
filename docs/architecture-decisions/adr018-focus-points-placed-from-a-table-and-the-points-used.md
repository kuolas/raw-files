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
focus point wherever the file records a position, and none for a shot
that records only the number of an autofocus point, such as a Nikon
DSLR's through the viewfinder, until a table of the body's points
exists. [ADR017](adr017-per-body-tables-measured-by-the-project.md)
decides where such tables come from: the project measures them on
frames shot with the body. The first table is the Nikon D600's,
measured on 56 frames
([the record](../formats/nef/autofocus-points.md#2-nikon-d600)). What
those frames and my other files show, as counts:

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
  full-size output frame: 6016x4016 on the D600, the size of its
  full-size preview and of its JPEG, not the 6080x4028 of its stored
  raw data. Every point's box is 164x164 there. A point so placed is
  where the viewfinder's point lies on the picture, not a position the
  camera measured: the camera measured nothing.
- **On a DX crop the point shifts by the crop's origin.** Two frames in
  DX, frames 49 and 50 of the record, put their points, 38 and 27,
  within 2 pixels of the table's position less CropHiSpeed's origin,
  (1040, 696), which on the D600 is the centred window's. The raw
  file's CropHiSpeed counts a border that the picture does not have, 32
  pixels of width and 12 of height on the D600 (3968x2636 against the
  3936x2624 picture), as the raw size of every Nikon body without a
  CropArea does, where ADR012 accepts the same error.
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
different meanings. ADR012 first left out the area mode, the point
numbers and the masks, because no caller needed them then; it says
`Metadata::focus` holds the camera's record and nothing estimated from
the picture.

On a crop, a placed point needs the frame of the crop's picture, and
the D600's DX frames give it. On the three frames I shot with my own
D600 in the DX crop, frames 49 to 51 of its record
([autofocus-points.md](../formats/nef/autofocus-points.md#26-the-control-frames)),
committed stripped in
[`tests/fixtures/nikon-d600/`](../../tests/fixtures/nikon-d600/README.md),
the NEF's full-size preview is 3936x2624, and so is the JPEG of the same
frame. That is the table's frame less twice CropHiSpeed's origin, 6016 −
2 × 1040 by 4016 − 2 × 696: the crop's picture is the centred window of
the output frame at that origin, the window into which frames 49 and 50
confirmed the shift, and the raw file holds a picture of that size in
its own preview. The window's size is a fact the record's crop frames
measure, and no tag records it.

A focus point's frame has the framing of the previews (ADR012), so that
a caller places a point on a preview by scaling it from the frame. With
CropHiSpeed's cropped size as the frame, a point placed on the DX NEF's
own preview is scaled by 3936/3968 across and 2624/2636 down, and lands
up to about 25 pixels off at the far edge: point 27, at 3163 in the
window, lands at 3137. A raw file and its JPEG would give the same
point in two frames, and the uncropped file's frame would be the
output's while the cropped file's counted the stored raw data's border.

CropHiSpeed counts the sensor's pixels, not the output's. On the D600
it records an original size of 6080x4028 and a DX crop of 3968x2636 at
(1040, 696), while the output is 6016x4016 and the DX picture
3936x2624. In the sensor's pixels the DX crop lies 16 pixels left of
the centre (1040 + 3968 / 2 = 3024, against 6080 / 2 = 3040), and in
the output's the DX picture lies centred, 1040 pixels from either edge.
Both hold because the camera trims the same left and top border from
the full frame and from the crop, within the frames' 2 pixels, but more
on the right of the full frame: 64 pixels across, where the crop loses
32. The record's DX frames confirm the shift, but they cannot tell
which of the two readings of the origin is the rule: the D600
satisfies both exactly.

Taking the window from the origin alone makes one tag value decide both
the shift and the frame, and nothing checks it. My own D7200 files show
that a recorded origin need not be the centred one: its 1.3x stills
record `[1, 6016, 4016, 4840, 3208, 576, 0]`, a crop 808 rows shorter
than the frame at a top of 0
([makernote-and-metadata.md](../formats/nef/makernote-and-metadata.md#86-focus-point)).
Read so, such a file would give a window as tall as the frame. In a
JPEG, the check that the JPEG's size is the window's would refuse it;
in a raw file nothing would, and a point would be placed in the wrong
frame without a sign. No table confirms such a crop, so no file would
be read so, but nothing in such a reader would stop it once a body's
record did.

A camera's JPEG can show a window of the frame without any crop.
Outside a crop, a point placed from the table is in the table's frame,
the body's full-size output, 6016x4016 on the D600, and in a JPEG the
reader keeps the points only when their frame has the JPEG's shape, to
within one pixel of rounding
([ADR014](adr014-jpeg-files-as-a-format-of-their-own.md)), so that an
export cut to another shape gives none. The D600 writes its JPEG at
three sizes: Large, 6016x4016, the frame; Medium, 4512x3008; and Small,
3008x2008, half the frame. The Large and Small JPEGs pass the shape
rule. The Medium JPEG is exactly 3:2, where three quarters of the frame
would be 4512x3012: 6016 × 3008 and 4016 × 4512 differ by 24,064, four
times the longest side, so the rule refuses the whole frame for it, and
in that frame every Medium JPEG shot through the viewfinder gave no
point, though its AFInfo2 names the same point as the NEF of the same
shot.

The JPEG's size alone does not say what part of the frame its picture
shows: the whole frame scaled more down than across, or a part of the
frame scaled evenly. I shot six RAW+JPEG pairs with my own D600, two at
each size, and fitted each JPEG onto the full-size preview of its own
NEF, which has the framing of the Large JPEG, with a scale and an
offset on each axis
([autofocus-points.md](../formats/nef/autofocus-points.md#29-the-jpeg-sizes)).
The Large JPEGs are the preview, and the Small JPEGs the whole frame at
exactly half its size. The Medium JPEGs show a window of the frame,
6010.2x4006.9 frame pixels, 3:2, centred to within 0.06 pixels, scaled
evenly by 0.75072; the two frames agree to 0.05 pixels. So a Medium
JPEG is to the frame what a full-size DX JPEG is: a centred window of
it, of a size that the record measures and no tag records.

## Decision

We will place a focus point that the camera names by number from the
table of the body's points, say where every point came from, and
report the points the camera used, with the primary, the area mode and,
for a body with a table, the body's layout. On a crop, and in a JPEG of
a size whose picture is a window of the frame, a placed point is
measured in that window, which the body's table keeps.

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
- **The table keeps each window its record measures**: the window of
  each crop its record confirms, beside the crop's setting, and the
  window of each JPEG size whose picture its record finds is a window
  of the frame rather than the whole of it, beside the JPEG's size as
  its frame header gives it. A crop's window is the size of the crop's
  picture in the output's pixels, as the record's crop frames measure
  it, the size of their full-size previews and JPEGs; a JPEG size's
  window is the part of the frame its picture shows, a whole number of
  the frame's pixels on each side, as the record measures it on
  RAW+JPEG pairs at that size. Every window lies centred in the table's
  frame, at an origin of half of the frame less the window on each
  axis, which the record's frames confirm. The table's tests refuse a
  window that cannot be centred to the pixel, the frame less the window
  being odd, or that does not fit the frame, and a JPEG size's window
  that has not its size's shape by ADR014's rule.
- **On a crop the point moves into the crop's window.** The window is a
  fact of the body, as the shift is: a point is placed on a crop only on
  a body whose record confirms the shift with the crop frames of the
  procedure, as the D600's does, and whose previews and JPEGs show the
  window's size; a table whose record does not gives no placed point on
  a crop. On a crop it confirms, a point is placed only when the file's
  CropHiSpeed origin is that window's centred origin, and is then
  measured in that window, in a raw file as in a JPEG: its centre moves
  by that origin and its box is kept. A file whose CropHiSpeed records another origin,
  for a crop setting the table confirms, gives no placed point. On
  every file that gives one, the window is still the table's frame less
  twice CropHiSpeed's origin on each side, 3936x2624 on a D600 DX shot,
  so the origin is a check against a measured size instead of the only
  source of it.
- **A JPEG of a size the table lists**, when it is no crop, has a point
  placed from the table measured in that size's window, moved back by
  its centred origin, as a point placed on a crop is: the window is the
  frame, and every box is kept. A JPEG of a size the table does not
  list, and a raw file, take the table's frame. A crop-mode JPEG is
  placed on its crop alone, whatever its size: a size's window is
  measured uncropped.
- **A point the window leaves out is dropped**: a point whose centre
  leaves the window, or whose box crosses the window's edge, as every
  point's box must fit inside its frame; and the primary dropped costs
  the focus point.
- **In a JPEG a placed point takes the path of a recorded one**, with
  the table's frame standing in for the recorded frame, except on a
  crop and in a JPEG of a size the table lists, where the frame is that
  window: a placed point is in full-size output pixels, so it moves by
  that window's origin, as above, where a recorded DX point, in
  fractions of the picture, is scaled by
  [ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md). A JPEG gives
  a placed point on a crop only when its own size equals that window,
  which on the D600 is the full image size; a crop-mode JPEG at a
  smaller size gives none, counted. The aspect-ratio rule of
  [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md) then applies
  to the placed frame, or the window, unchanged.
- **On the D600**, the table lists the DX crop, setting 2, with its
  window, 3936x2624 at (1040, 696), and the Medium JPEG, 4512x3008,
  with the window 6010x4006 at (3, 5): the window centred to the pixel
  nearest the measured one. A point placed in it and scaled onto the
  JPEG lands within 0.08 of a Medium pixel across and 0.35 down of
  where the measured window puts it, anywhere in the frame, well within
  the table's own tolerance of 3 frame pixels.
- **The points the camera used are reported**, through
  `Metadata::focus_points`: every point, each with its number when the
  file gives one, its centre, its box and whether focus was confirmed
  when the file says; which of them is the primary, the one
  `Metadata::focus` reports; and the area mode. For a placed point the
  list is one point per set bit of the mask whose number the table has,
  in number order, or the primary alone when the mask is empty; a set
  bit the table lacks is skipped and counted, never a reason to give
  none. For a recorded position the list is that one point. On a crop,
  and in a JPEG size's window, every point of the list shifts as the
  primary does.
- **The area mode is reported in neutral words**, as `FocusAreaMode`:
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
- **CropHiSpeed's cropped size as the frame on a crop in a raw file**,
  3968x2636 on the D600, with the JPEG's own size in a JPEG, as this
  record first had it, taking the 32 and 12 pixels that the cropped
  size counts beyond the picture as an error that ADR012 already
  accepts for the bodies without a CropArea, since no raw file was
  known to give the picture's size. It counts the border of the stored
  raw data, which the picture and its previews do not have, so a point
  scaled onto the NEF's own preview is off by up to 25 pixels, and a
  raw file and its JPEG give the same point in two frames.
- **CropHiSpeed's cropped size less a border measured per body.** One
  more number for each body's record to measure and keep. It was first
  rejected because the window follows from the origin the record
  already confirms, a reason that no longer holds now that the window is
  measured and the origin checked against it; the measured window is
  kept instead.
- **The window from CropHiSpeed's origin alone**, the table's frame
  less twice the origin, as this record next had it. It holds on the
  D600, but one tag value decides the shift and the frame unchecked,
  and a raw file whose origin is not centred, as the D7200's 1.3x
  stills record, would give a point in a frame of the wrong size
  without a sign.
- **Half the frame less CropHiSpeed's cropped size.** That size counts
  the sensor's border: on the D600 it gives (1024, 690), 16 and 6
  pixels off the frames' (1040, 696).
- **The window from the size of the file's own full-size preview.** It
  needs no table change, but the MakerNote's reader does not see the
  previews, a body may write no full-size preview, and a JPEG has its
  own check already.
- **The Medium JPEG's window as 6012x4008 at (2, 4).** It is exactly
  3:2, the JPEG's own shape, but 1.8 pixels wider than measured and 1.1
  taller, and places a point up to 0.71 of a Medium pixel off across
  and 0.45 down, about nine times the nearest window's error across and
  1.3 times down.
- **A window of fractional pixels**, 6010.2x4006.9 at (2.90, 4.54), the
  measurement as it is. It needs a size and an origin that are not
  whole numbers in the table, and in the frame that a focus point and
  a list of points give a caller, which the public API measures in
  whole pixels, for a gain far below the table's tolerance.
- **A rule for every JPEG of exactly 3:2**, mapping it to the centred
  3:2 window of the frame, as a crop-mode JPEG's window is found from
  the JPEG's shape
  ([ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md)). It needs
  no measurement, but on the D600 that window is the frame's whole
  width, 6016, where the picture shows 6010.2, so a point lands up to
  about 2 Medium pixels off; and it would give a point to any export
  cut to 3:2 from a camera's JPEG, which ADR014 refuses.
- **Leaving the Medium JPEG without a point**, while the NEF of the
  same shot gives one.

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
- A crop-mode JPEG smaller than the crop's window, such as a DX JPEG
  at the Medium size. The shift is in full-size pixels, the D600's
  frames were shot at full size only, and its picture is a part of the
  crop's window, which frames shot in that crop at that size would
  measure; such a JPEG gives no placed point until frames at those
  sizes are measured, by this record's rule or another.
- A crop or a JPEG size whose picture is not centred in the output, or
  a JPEG size not scaled evenly: none is known. Its record would
  measure where the window lies, and a decision would say how the
  table keeps that.
- The crops that keep one side, 1:1 and 16:9, whose recorded positions
  are moved by CropArea or the JPEG's shape
  ([ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md)), not
  placed from a table.
- A point the file records as a position, which is not placed from a
  table: its frame is the one the file records, under
  [ADR012](adr012-focus-points-as-the-camera-records-them.md), ADR014
  and ADR015.
- The layout of the body's points, which stays in the table's frame,
  on a crop and on a JPEG size's window alike.
- Which bodies have tables, which crops and sizes a body's record must
  shoot, and how a table and a window are measured: the procedure of
  the NEF notes, under
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
- A placed point carries the table's tolerance, a few pixels, which the
  notes state. A viewer's box is far larger.
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
- A D600 DX NEF and its JPEG give the same placed point in the same
  frame, 3936x2624, the size of the NEF's full-size preview, so a
  point scaled onto that preview lands where the table puts it.
- On the D600, every DX frame records (1040, 696), the centred origin
  of the 3936x2624 window, so the check against the window costs no DX
  file its point.
- A raw file whose CropHiSpeed origin differs from the window's, by a
  single pixel or by a crop placed elsewhere, gives no placed point
  instead of one in a frame no frame of the record checked.
- A crop-mode JPEG at a smaller size is told from a full-size one by
  its size against the window, with no border to know.
- On a crop, as on a full frame, a placed point's frame is the
  output's, not the raw file's image size, which on a body without a
  CropArea counts the stored raw data's border; `Focus`'s
  documentation already tells a caller not to assume the two are
  equal.
- A JPEG of a body with a table gives the point its raw file gives,
  placed on the JPEG's own picture, at full image size, at a smaller
  size whose picture is the whole frame scaled, and at a size whose
  window the table keeps; an export cut to another shape gives none,
  as ADR014 says.
- A D600 Medium JPEG shot through the viewfinder gives its points,
  measured in the 6010x4006 window, every centre the table's less
  (3, 5); a caller that scales a point from that frame onto the JPEG
  finds it where the picture shows the point. A NEF and its Medium JPEG
  give the same point in two frames, the table's and the window, where
  a DX NEF and its JPEG give it in one; a caller compares points across
  files in their own frames.
- The JPEG reader's shape rule is unchanged: a window passes it, and an
  export cut to another shape still gives none.
- An export scaled from a Large JPEG to exactly the Medium size cannot
  be told from a Medium JPEG, and its points are measured in the
  window: up to about 2.3 pixels off across and 3.8 down on the
  export, where the shape rule alone would give it none. An export cut
  from a larger JPEG to exactly that size cannot be told from one
  either, and its points are placed as on a Medium JPEG, off by as much
  as the cut moved the picture, hundreds of pixels, where the shape
  rule alone would give it none; a cut of the frame's own shape is
  misplaced so already.
- An export scaled from a Medium JPEG to another size takes the whole
  frame, since the window is kept for the Medium size alone. An export
  taller than about 750 pixels has not the whole frame's shape and
  gives no point; at that height or less, the rule's rounding of one
  pixel takes the two shapes for one, and its point is placed in the
  whole frame, off by under a pixel at that size.
- Each confirmed crop costs a body's record one more measured size,
  the size of the crop frames' previews or JPEGs, which the procedure
  already shoots; each smaller JPEG size costs RAW+JPEG pairs at that
  size and the fit of each JPEG onto its raw file's full-size preview,
  and a size the record finds to be the whole frame scaled needs no
  entry.
