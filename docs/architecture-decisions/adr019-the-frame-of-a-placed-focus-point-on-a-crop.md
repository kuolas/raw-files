# ADR019: The frame of a placed focus point on a crop

- Status: Accepted
- Amends: [ADR018](adr018-focus-points-placed-from-a-table-and-the-points-used.md),
  in one point: the frame that a point placed from a table takes on a
  crop in a raw file, which that record set at CropHiSpeed's cropped
  size
- Date: 2026-10-27
- Issue: #57

## Context

[ADR018](adr018-focus-points-placed-from-a-table-and-the-points-used.md)
places a point that the camera names by number from the table of the
body's points, in the table's frame, the body's full-size output:
6016x4016 on the D600, the size of its full-size preview and of its
JPEG, not the 6080x4028 of its stored raw data. On a crop, the point
shifts by CropHiSpeed's origin into the crop's picture, and ADR018 made
the frame of that picture CropHiSpeed's cropped size in a raw file,
3968x2636 on the D600, and the JPEG's own size in a JPEG. It took the
32 and 12 pixels that the cropped size counts beyond the picture as an
error that [ADR012](adr012-focus-points-as-the-camera-records-them.md)
already accepts for the bodies without a CropArea, since no raw file
was known to give the picture's size.

The D600's DX frames give it. On the three frames I shot with my own
D600 in the DX crop, frames 49 to 51 of its record
([autofocus-points.md](../formats/nef/autofocus-points.md#26-the-control-frames)),
committed stripped in
[`tests/fixtures/nikon-d600/`](../../tests/fixtures/nikon-d600/README.md),
the NEF's full-size preview is 3936x2624, and so is the JPEG of the same
frame. That is the table's frame less twice CropHiSpeed's origin, 6016 −
2 × 1040 by 4016 − 2 × 696: the crop's picture is the centred window of
the output frame at that origin, the window into which frames 49 and 50
confirmed the shift, and the raw file holds a picture of that size in
its own preview.

A focus point's frame has the framing of the previews (ADR012), so that
a caller places a point on a preview by scaling it from the frame. With
CropHiSpeed's cropped size as the frame, a point placed on the DX NEF's
own preview is scaled by 3936/3968 across and 2624/2636 down, and lands
up to about 25 pixels off at the far edge: point 27, at 3163 in the
window, lands at 3137. A raw file and its JPEG would give the same
point in two frames, and the uncropped file's frame would be the
output's while the cropped file's counted the stored raw data's border.

## Decision

We will measure a point placed from a table on a crop in the crop
window: the table's frame less twice CropHiSpeed's origin on each side,
3936x2624 on a D600 DX shot, in a raw file as in a JPEG. The centre
moves by CropHiSpeed's origin and the box is kept, as ADR018 decides.
A point whose centre leaves the window, or whose box crosses the
window's edge, is dropped, as every point's box must fit inside its
frame; and the primary dropped costs the focus point.

A JPEG gives a placed point on a crop only when its own size equals
that window, which on the D600 is the full image size; a crop-mode JPEG
at a smaller size gives none, counted, as ADR018 leaves it.

The window is a fact of the body, as the shift is: it holds on a body
whose record confirms the shift with the crop frames of the procedure,
whose previews and JPEGs show the window's size, and a table whose
record does not gives no placed point on a crop.

Rejected:

- **CropHiSpeed's cropped size, as ADR018 had it.** It counts the
  border of the stored raw data, which the picture and its previews do
  not have, so a point scaled onto the NEF's own preview is off by up
  to 25 pixels, and a raw file and its JPEG give the same point in two
  frames.
- **CropHiSpeed's cropped size less a border measured per body.** One
  more number for each body's record to measure and keep, where the
  window follows from the origin the record already confirms.

## Consequences

- A D600 DX NEF and its JPEG give the same placed point in the same
  frame, 3936x2624, the size of the NEF's full-size preview, so a
  point scaled onto that preview lands where the table puts it.
- In ADR018, the frame "CropHiSpeed's cropped size in a raw file", and
  what its context and consequences say of the 32 pixels of border a
  caller carries to the far edge, no longer describe the decision, nor
  does "A centre that leaves the frame costs that point", since a box
  that crosses the window's edge costs its point too; the rest of
  ADR018 stands.
- A crop-mode JPEG at a smaller size is told from a full-size one by
  its size against the window, with no border to know.
- On a crop, as on a full frame, a placed point's frame is the
  output's, not the raw file's image size, which on a body without a
  CropArea counts the stored raw data's border; `Focus`'s
  documentation already tells a caller not to assume the two are
  equal.
