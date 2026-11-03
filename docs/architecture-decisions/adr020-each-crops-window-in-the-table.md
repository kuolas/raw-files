# ADR020: Each crop's and JPEG size's window in the table

- Status: Accepted
- Amends: [ADR019](adr019-the-frame-of-a-placed-focus-point-on-a-crop.md),
  in one point: where the crop window's size comes from, which that
  record took from CropHiSpeed's origin alone, and so which files are
  placed on a crop
- Date: 2026-10-29
- Revision: 2, 2026-11-03
- Issue: #62

## Context

A point placed from a table on a crop is measured in the crop's window,
and its centre moves into that window by the window's origin
([ADR019](adr019-the-frame-of-a-placed-focus-point-on-a-crop.md)). That
record found the window from the file: the table's frame less twice
CropHiSpeed's origin on each side, 6016 − 2 × 1040 by 4016 − 2 × 696,
3936x2624, on a D600 DX shot.

CropHiSpeed counts the sensor's pixels, not the output's. On the D600
it records an original size of 6080x4028 and a DX crop of 3968x2636 at
(1040, 696), while the output is 6016x4016 and the DX picture
3936x2624. In the sensor's pixels the DX crop lies 16 pixels left of
the centre (1040 + 3968 / 2 = 3024, against 6080 / 2 = 3040), and in
the output's the DX picture lies centred, 1040 pixels from either edge.
Both hold because the camera trims the same left and top border from
the full frame and from the crop, within the frames' 2 pixels, but more
on the right of the full frame: 64 pixels across, where the crop loses
32. The record's DX
frames confirm the result, points 38 and 27 within 2 pixels of the
table's place less (1040, 696)
([autofocus-points.md](../formats/nef/autofocus-points.md#26-the-control-frames)),
but they cannot tell which of the two readings of the origin is the
rule: the D600 satisfies both exactly.

Taking the window from the origin makes one tag value decide both the
shift and the frame, and nothing checks it. My own D7200 files show
that a recorded origin need not be the centred one: its 1.3x stills
record `[1, 6016, 4016, 4840, 3208, 576, 0]`, a crop 808 rows shorter
than the frame at a top of 0
([makernote-and-metadata.md](../formats/nef/makernote-and-metadata.md#86-focus-point)).
Read by ADR019's rule, such a file would give a window as tall as the
frame. In a JPEG, the check that the JPEG's size is the window's would
refuse it; in a raw file nothing would, and a point would be placed in
the wrong frame without a sign. No table confirms such a crop, so no
file is read so, but nothing in the reader would stop it once a body's
record did.

The window's size is a fact the record's crop frames already measure:
on the D600, frames 49 to 51's full-size previews and JPEGs are
3936x2624
([autofocus-points.md](../formats/nef/autofocus-points.md#26-the-control-frames)).

A camera's JPEG can show a window of the frame without any crop.
Outside a crop, a point placed from the table is in the table's frame,
the body's full-size output, 6016x4016 on the D600
([ADR018](adr018-focus-points-placed-from-a-table-and-the-points-used.md)),
and in a JPEG the reader keeps the points only when their frame has
the JPEG's shape, to within one pixel of rounding
([ADR014](adr014-jpeg-files-as-a-format-of-their-own.md)), so that an
export cut to another shape gives none. The D600 writes its JPEG at
three sizes: Large, 6016x4016, the frame; Medium, 4512x3008; and Small,
3008x2008, half the frame. The Large and Small JPEGs pass the shape
rule. The Medium JPEG is exactly 3:2, where three quarters of the frame
would be 4512x3012: 6016 × 3008 and 4016 × 4512 differ by 24,064, four
times the longest side, so the rule refused it, and every Medium JPEG
shot through the viewfinder gave no point, though its AFInfo2 names the
same point as the NEF of the same shot.

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

We will keep, in a body's table, the window of each crop its record
confirms, beside the crop's setting, and the window of each JPEG size
whose picture its record finds is a window of the frame rather than the
whole of it, beside the JPEG's size as its frame header gives it. A
crop's window is the size of the crop's picture in the output's pixels,
as the record's crop frames measure it, the size of their full-size
previews and JPEGs; a JPEG size's window is the part of the frame its
picture shows, a whole number of the frame's pixels on each side, as
the record measures it on RAW+JPEG pairs at that size. Every window
lies centred in the table's frame, at an origin of half of the frame
less the window on each axis, which the record's frames confirm. The
table's tests refuse a window that cannot be centred to the pixel, the
frame less the window being odd, or that does not fit the frame, and a
JPEG size's window that has not its size's shape by ADR014's rule.

A point is placed on a crop only when the file's CropHiSpeed origin is
that centred origin, and is then measured in that window and moved by
that origin, as ADR019 decides. A file whose CropHiSpeed records
another origin, for a crop setting the table confirms, gives no placed
point. On every file that gives one, the window is still the table's
frame less twice CropHiSpeed's origin, so ADR019's rule holds as
written; this record makes it a check against a measured size instead
of the only source of it.

A point placed from the table in a JPEG that is no crop, of a size the
table lists, is measured in that size's window and moves back by its
centred origin, as a point placed on a crop is: the window is the
frame, every box is kept, a point whose centre or box the window leaves
out is dropped, and the primary dropped costs the focus point. ADR014's
shape rule then applies to the window unchanged. A JPEG of a size the
table does not list, and a raw file, take the table's frame. A
crop-mode JPEG is placed on its crop alone, whatever its size: a size's
window is measured uncropped.

On the D600, the table lists the DX crop, setting 2, with its window,
3936x2624 at (1040, 696), and the Medium JPEG, 4512x3008, with the
window 6010x4006 at (3, 5): the window centred to the pixel nearest the
measured one. A point placed in it and scaled onto the JPEG lands
within 0.08 of a Medium pixel across and 0.35 down of where the
measured window puts it, anywhere in the frame, well within the table's
own tolerance of 3 frame pixels.

Rejected:

- **The window from CropHiSpeed's origin alone, as ADR019 found it.** It holds
  on the D600, but one tag value decides the shift and the frame
  unchecked, and a raw file whose origin is not centred, as the
  D7200's 1.3x stills record, would give a point in a frame of the
  wrong size without a sign.
- **Half the frame less CropHiSpeed's cropped size.** That size counts
  the sensor's border: on the D600 it gives (1024, 690), 16 and 6 pixels off the
  frames' (1040, 696).
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

- A crop or a JPEG size whose picture is not centred in the output, or
  a JPEG size not scaled evenly: none is known. Its record would
  measure where the window lies, and a decision would say how the
  table keeps that.
- The crops that keep one side, 1:1 and 16:9, whose recorded positions
  are moved by CropArea or the JPEG's shape
  ([ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md)), not
  placed from a table.
- A crop-mode JPEG smaller than the crop's window, such as a DX JPEG
  at the Medium size: its picture is a part of the crop's window, which
  frames shot in that crop at that size would measure; until then it
  gives no point, as ADR018 and ADR019 leave it.
- A point the file records as a position, which is not placed from a
  table: its frame is the one the file records, under
  [ADR012](adr012-focus-points-as-the-camera-records-them.md), ADR014
  and ADR015.
- The layout of the body's points, which stays in the table's frame,
  on a crop and on a JPEG size's window alike.
- Which crops and sizes a body's record must shoot, and how a window is
  measured: the procedure of the NEF notes, under
  [ADR017](adr017-per-body-tables-measured-by-the-project.md).

## Consequences

- On the D600, every DX frame records (1040, 696), the centred origin
  of the 3936x2624 window, so nothing a DX file gives changes.
- A raw file whose CropHiSpeed origin differs from the window's, by a
  single pixel or by a crop placed elsewhere, gives no placed point
  instead of one in a frame no frame of the record checked.
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
  export, where the shape rule gave it none. An export cut from a
  larger JPEG to exactly that size cannot be told from one either, and
  its points are placed as on a Medium JPEG, off by as much as the cut
  moved the picture, hundreds of pixels, where the shape rule gave it
  none; a cut of the frame's own shape was misplaced so already.
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
- In ADR019, the reason given against keeping one more number per body,
  that the window follows from the origin the record confirms, no
  longer holds: the window is measured and the origin checked against
  it. A file whose origin is not the window's, which ADR019's rule
  would place, gives no point. The rest of ADR019 stands, its window
  among it on every file that gives a point.
