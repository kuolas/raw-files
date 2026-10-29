# ADR020: Each crop's window in the table

- Status: Accepted
- Amends: [ADR019](adr019-the-frame-of-a-placed-focus-point-on-a-crop.md),
  in one point: where the crop window's size comes from, which that
  record took from CropHiSpeed's origin alone, and so which files are
  placed on a crop
- Date: 2026-10-29

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

## Decision

We will keep, in a body's table, the window of each crop its record
confirms, beside the crop's setting: the size of the crop's picture in
the output's pixels, as the record's crop frames measure it, the size
of their full-size previews and JPEGs. The window lies centred in the
table's frame, at an origin of half of the frame less the window on
each axis, which the crop frames confirm; a window that cannot be
centred to the pixel, the frame less the window being odd, or that does
not fit the frame, is refused by the table's tests.

A point is placed on a crop only when the file's CropHiSpeed origin is
that centred origin, and is then measured in that window and moved by
that origin, as ADR019 decides. A file whose CropHiSpeed records
another origin, for a crop setting the table confirms, gives no placed
point. On every file that gives one, the window is still the table's
frame less twice CropHiSpeed's origin, so ADR019's rule holds as
written; this record makes it a check against a measured size instead
of the only source of it.

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

## What this decision does not cover

- A crop whose picture is not centred in the output: none is known.
  Its record would measure where the window lies, and a decision would
  say how the table keeps that.
- The crops that keep one side, 1:1 and 16:9, whose recorded positions
  are moved by CropArea or the JPEG's shape
  ([ADR015](adr015-the-focus-point-of-a-crop-mode-jpeg.md)), not
  placed from a table.
- Which crops a body's record must shoot, and how a window is measured:
  the procedure of the NEF notes, under
  [ADR017](adr017-per-body-tables-measured-by-the-project.md).

## Consequences

- On the D600, nothing a file gives changes: every DX frame records
  (1040, 696), the centred origin of the 3936x2624 window.
- A raw file whose CropHiSpeed origin differs from the window's, by a
  single pixel or by a crop placed elsewhere, gives no placed point
  instead of one in a frame no frame of the record checked.
- Each confirmed crop costs a body's record one more measured size, the
  size of the crop frames' previews or JPEGs, which the procedure
  already shoots.
- In ADR019, the reason given against keeping one more number per body,
  that the window follows from the origin the record confirms, no
  longer holds: the window is measured and the origin checked against
  it. A file whose origin is not the window's, which ADR019's rule
  would place, gives no point. The rest of ADR019 stands, its window
  among it on every file that gives a point.
