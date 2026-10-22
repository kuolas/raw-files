# ADR015: The focus point of a crop-mode JPEG

- Status: Accepted
- Builds on: [ADR012](adr012-focus-points-as-the-camera-records-them.md),
  focus points as the camera records them;
  [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md), JPEG files as
  a format of their own
- Date: 2026-10-22
- Issue: #35

## Context

A Nikon body can record a picture smaller than its sensor's full
output, in a crop mode: DX, which cuts both sides to the area of a
smaller sensor, or an aspect ratio, 1:1 or 16:9, which keeps one side
and cuts the other. Its MakerNote then records the focus point in the
frame of the full output, not of the picture.
[ADR012](adr012-focus-points-as-the-camera-records-them.md) decided how
a raw file's point is placed on the picture: the tag CropHiSpeed tells
a crop apart, and CropArea, which records the visible rectangle of the
stored raw data, gives the picture's size. A DX crop's point is scaled
onto CropArea, since a DX crop records fractions of its picture; an
aspect crop's point moves back by the origin of the centred window that
CropArea is in the full frame, since it records positions in that frame.
Without a CropArea, a crop is read as recorded.

[ADR014](adr014-jpeg-files-as-a-format-of-their-own.md) keeps a JPEG's
focus point, after those rules, only when its frame has the JPEG's
aspect ratio to within a pixel of rounding, so that an export cropped to
another shape gives none. It left what a camera writes in a crop-mode
JPEG unchecked.

No public sample shows one. Shots made with the maintainer's Z 9, as a
JPEG and a raw file of the same picture, in each crop mode and at each
of the camera's three image sizes, show what such a JPEG records:

- **The frame stays the full output at every crop and size.** AFInfo2's
  frame is 8256×5504 in every JPEG, with the same centre and area as in
  the raw file of the same shot; it is not scaled to the smaller sizes.
- **The JPEG holds no CropArea.** Each raw file holds one, and its
  CropHiSpeed is the JPEG's, at every size.
- **The full-size JPEG is the raw file's CropArea**: 5392×3592 on DX,
  5504×5504 on 1:1, 8256×4640 on 16:9. The smaller sizes keep the
  crop's shape, but not always that rectangle's: the DX JPEGs at the
  middle and small sizes are exactly 3:2 (4032×2688 and 2688×1792), as
  the 5392×3592 rectangle is not; the 1:1 and 16:9 JPEGs keep their
  shape at every size (4128×4128 and 2752×2752; 6192×3480 and
  4128×2320).
- **CropArea is not CropHiSpeed's size scaled to the frame.** It is the
  cropped size CropHiSpeed records, in the sensor's pixels, less a
  border of 16 pixels on each side the crop cuts.
- **A picture taken in portrait orientation** is stored as on its side,
  with Orientation 8, as in the raw file: the JPEG's frame header and
  AFInfo2's frame are both landscape, square on 1:1.

So ADR014 alone keeps the point of an uncropped JPEG and of a DX JPEG at
the two smaller sizes, whose shape is the full frame's, and drops the
point of every 1:1 and 16:9 JPEG and of the DX JPEG at full size, whose
5392×3592 differs from 3:2 by more than a pixel of rounding. Nothing is
wrong with those points but where they are recorded: the raw file of
the same shot gives each of them, placed by ADR012's rules.

## Decision

We will place the focus point of a Nikon JPEG shot in a crop mode on the
JPEG's own picture, as ADR012 places a raw file's point on CropArea,
when its MakerNote records a crop and holds no CropArea. The JPEG's
frame header gives the picture, as it gives the image size.

- **A crop that cuts both sides (DX)** records fractions of its picture:
  the centre and the area are scaled onto the JPEG's frame, as onto
  CropArea in a raw file, rounded to the nearest pixel; the frame becomes
  the JPEG's size.
- **A crop that keeps one side (1:1, 16:9)** records positions in the
  full frame: its picture is the centred window of the recorded frame
  that has the frame's length on the side the crop keeps and the JPEG's
  shape, its other side rounded to the nearest pixel. The centre moves
  back by the window's origin, half the difference between the frame and
  the window on the cut side, rounded down, and the area is kept, as in a
  raw file; the frame becomes the window. A window that would be longer
  than the frame on the cut side gives no point.
- **Then the aspect ratio rule of ADR014 applies, unchanged**, to the
  placed frame.

What tells a crop apart, and which side it keeps, is CropHiSpeed, as in
a raw file. A JPEG whose MakerNote holds a CropArea, as a MakerNote
copied from a raw file does, is read by ADR012's rule: a JPEG has no raw
size to check CropArea against, so a crop gives no point, and a file that
is no crop gives its point as recorded, for the aspect ratio rule to
judge. Every other rule of ADR012 and ADR014 stands.

Rejected:

- **ADR014's rule alone.** It drops the point of every aspect-ratio crop
  and of a DX crop at full size, which the camera recorded and the raw
  file of the same shot gives.
- **CropHiSpeed's size in place of CropArea's.** It counts the border
  that CropArea leaves out, so the windows it gives are a few pixels
  taller than the picture, and the aspect ratio rule then drops the
  16:9 and DX crops.
- **A rule by the image size the MakerNote records** (full, middle,
  small). The size does not tell a crop apart, and placing a crop only
  at full size, where the JPEG equals the raw file's CropArea, drops the
  points of every smaller crop JPEG.
- **A second check of the JPEG's shape against CropHiSpeed's cropped
  shape**, within a tolerance. It would refuse a crop-mode JPEG that an
  editor later cut to another shape, but no file shows one, and it would
  add a tolerance of its own beside the aspect ratio rule's.

## What this decision does not cover

What a Nikon DSLR in live view writes in a crop-mode JPEG, or a DX crop
taken at 1:1 or 16:9 on a body with a larger sensor; the crop modes of
other makers, whose recorded frame, where one is recorded, has been the
picture's own (Sony's, as ADR012 found); and telling a crop-mode JPEG
that an editor cut again from the camera's own. Each would build on this
decision.

## Consequences

- A crop-mode JPEG from a Nikon Z body gives the point its raw file
  gives, at every image size, scaled onto the JPEG: on the shots above,
  within a pixel of the raw file's point scaled the same way.
- On a DX crop, the frame of the point is the JPEG's size; on a 1:1 or
  16:9 crop, it is the window in the recorded frame's pixels, which has
  the JPEG's shape. A caller scales the point onto any preview in the
  same way either way.
- The aspect ratio rule passes on every crop placed this way, since the
  window is built to the JPEG's shape. An editor's export of a crop-mode
  JPEG that keeps the camera's MakerNote and adds no CropArea keeps a
  point placed on the export's pixels, which is misplaced if the editor
  cut them to another shape. No file in the set shows one.
- An export that carries a raw file's CropArea gives no point on a crop,
  whatever its shape.
- What a crop-mode JPEG records is not checked on a public file.
