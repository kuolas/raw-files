# Nikon autofocus points, measured per body

A Nikon DSLR shot through the viewfinder records which autofocus point
the camera used, by its number, and no position
([makernote-and-metadata.md](makernote-and-metadata.md#86-focus-point)).
Where that point lies on the picture depends on the body's layout of
points, which no file carries. This page holds what raw-files knows of
those layouts: the procedure by which a body's points are measured, and
the record of each body measured so far, one section per body. The
tables in the code are to be written from these records.

Every table is measured by the project, or by a contributor, on frames
shot with the body itself and read in the maker's own software, run as
a reader to compare with; no table is transcribed from another
project on its licence alone, and one from another project would
enter only by a decision made together with that project: that is the
decision of
[ADR017](../../architecture-decisions/adr017-per-body-tables-measured-by-the-project.md),
built on the clean room of
[ADR013](../../architecture-decisions/adr013-clean-room-and-third-party-code.md).
The frames are the photographer's own and stay private; a record cites
them by their number in the session and gives counts and values, never
a personal field. Claims carry the confidence tags of the NEF notes
([README.md](README.md#confidence-tags)): **[F]** measured on the
frames named, **[P]** stated in published documentation, **[I]**
inferred.

## Contents

- [1. The procedure](#1-the-procedure)
- [2. Nikon D600](#2-nikon-d600)
- [3. Bodies without a table](#3-bodies-without-a-table)

## 1. The procedure

A body's table is measured in one session, with the body, a lens and a
static subject. The frames are shot RAW + JPEG so that the JPEG path can
be checked on the same shots. There are two ways to contribute a body:
shoot the frames (steps 1 and 2) and send them to the project, which
measures them, writes the record and the table, and credits the
photographer in both; or do the whole procedure and submit the record,
the table and the fixtures, which the project then checks against the
frames before the table enters. Shooting the frames needs no code and
no measuring, only the body and an hour; it is the contribution that
matters, since no table exists without it. The contributing guide says
how to get in touch
([CONTRIBUTING.md](../../../CONTRIBUTING.md#adding-a-cameras-focus-points)).

1. **Shoot one frame per selectable point in single-point autofocus**,
   the point moved through every position the body offers, in the
   body's full frame (FX on an FX body) and in landscape orientation.
   Any static subject with enough detail to focus on will do; the
   subject is not measured, the point is.
2. **Shoot the control frames**, which the file itself can check:
   - two repeats of points already shot, to show that a reading repeats;
   - a frame or two in portrait orientation, to show that the point is
     drawn turned with the picture and the file's bytes do not change;
   - two frames in a crop mode (DX on an FX body) with points near
     opposite corners, to measure how a point shifts into the crop;
   - two or three frames in auto-area mode, which record several points
     at once, to check the mask of points used against the display;
   - a frame or two in a dynamic-area mode;
   - a frame in manual focus, to see what the body writes then;
   - where the body has live view, four live-view frames with the focus
     area moved to the four limits of its travel: a live-view frame
     records its area in the file, so these calibrate the reading of
     every other frame.
3. **Open each frame in the maker's own software** (Nikon's NX Studio)
   with its focus-point display on, the picture fitted to the window
   with all four edges in view, the window the same size throughout, and
   save one screenshot per frame.
4. **Measure each screenshot**: the picture's bounds in the window and
   the drawn box's outer bounds, in screen pixels, converted to the
   frame's pixels by the ratio of the picture's size to its size on
   screen. A script does it from the screenshots' pixels; the method,
   the scale and the tolerance go into the record.
5. **Check the controls**: the live-view frames against the area their
   file records, the repeats against their originals, the portrait
   frames against the landscape readings turned, the crop frames against
   the crop tag's origin, the auto-area frames against their masks.
6. **Read the files' bytes**: the point number, the layout code, the
   mask and the focus mode of every frame, with a script written from
   the notes. The mask's bit order is read off the single-point frames,
   where exactly one bit is set.
7. **Write the record** in this page, as section 2 is written: the
   session, what the body writes, the bit order, the table with its
   readings, the calibration, the control frames, and the frames chosen
   as fixtures. Write the table file in the code with its provenance,
   register it, strip one single-point frame with its JPEG, one crop
   frame and one auto-area frame as fixtures
   ([`tests/fixtures/`](../../../tests/fixtures/README.md)), add their
   expected values, and run the tests. In the pull request say which
   software and version was run, and cite the frames as your own. A
   record names who shot the frames and who measured them; the frames
   stay private except the fixtures, which are published under the
   repository's licence with the photographer's agreement.

What the record must give a reader: enough to repeat the measurement
and to judge its error. A viewer that draws the point draws a box far
larger than the tolerance of a careful reading, so a reading to a few
pixels is enough; what matters is that the error is known.

## 2. Nikon D600

Measured on 2026-10-26 on 56 frames shot with the maintainer's own
D600, RAW + JPEG Fine Large, a 35 mm f/1.8 lens, one static scene, and
read in Nikon NX Studio 1.10.0, run as a reader to compare with.
The frames are numbered 1 to 56 in the session's order. Every value
below was read from the files' bytes with a script written from the
notes and, for the box, from the NX Studio screenshots measured as
section 1 says. **[F]**

### 2.1 The session

| Frames | Shot | What they show |
|---|---|---|
| 1 to 39 | single point, one frame per point, landscape, FX | the table |
| 40, 41 | repeats of points 1 and 13 | the reading repeats |
| 42 to 44 | auto-area | the mask of points used |
| 45, 46 | manual focus | what a manual-focus shot writes |
| 47, 48 | portrait orientation, points 13 and 38 | the point turns with the picture |
| 49, 50 | DX crop, points 38 and 27 | the shift into the crop |
| 51, 52 | dynamic-area 21-point, AF-C, 51 in DX | a dynamic-area shot |
| 53 to 56 | live view, the area at the four limits of its travel | the calibration |

Of the 56, 45 are single-point frames with a point number: the 39, the
two repeats, the two portrait frames and the two DX frames. **[F]**

### 2.2 What the body writes

Through the viewfinder, focusing with autofocus, the D600 writes
AFInfo2 version `0100`, 30
bytes: detection method 0 at byte 4, the area mode at byte 5, the
layout code (FocusPointSchema) 3, the 39-point layout, at byte 6, the
primary point's number at byte 7, a mask of the points used in the
seven bytes 8 to 14, a zero frame and area in bytes 16 to 27, and 0 in
the in-focus byte 28. FocusMode is `AF-S  ` or `AF-C  `; what a
manual-focus or live-view shot writes instead is in section 2.6.
CropHiSpeed is
`[0, 6080, 4028, 6080, 4028, 0, 0]` with the crop off and
`[2, 6080, 4028, 3968, 2636, 1040, 696]` in DX. The raw image is
6080x4028 (DX: 3968x2636), with no CropArea; the JPEG is 6016x4016
(DX: 3936x2624). On all 56 frames the JPEG carries the same AFInfo2 and
the same CropHiSpeed as the NEF of the same frame. **[F]**

The area modes seen, with their counts: 0 (single point) on 45 frames
with the 39-point layout and on the 2 manual-focus frames with layout
code 0; 5 (dynamic area, 21 points) on 2; 8 (auto-area) on 3; 4 on the
4 live-view frames, with layout code 0. **[F]** The names are ExifTool's
for these codes, read in its published tag documentation [P][1], and
match the modes the frames were shot in. **[F]**

### 2.3 The mask's bit order

On all 45 single-point frames with a point number exactly one bit of
the seven mask bytes is set: bit number − 1, counting from the least
significant bit of the first byte, so point 1 sets bit 0 of byte 8,
point 13 sets bit 4 of byte 9 and point 38 sets bit 5 of byte 12.
**[F]** ExifTool documents a bit order for the 11-point layout only
[P][1]; the 39-point order is measured here.

### 2.4 The table

The frame is 6016x4016, the body's full-size output and the frame its
live-view shots record. Every point's box reads 164x164 (readings 162.8
to 165.7). The centres are read from NX Studio's box and rounded to the
pixel; the tolerance is ±3 pixels (section 2.5). The name is the
viewfinder's row letter, A at the top to E at the bottom, and column
number, 1 at the left to 11 at the right; rows A and E have points over
the three inner columns only and number them 1 to 3. That is the order
in which ExifTool's published table names the numbers, read as
documentation [P][1], and the frames confirm it: 1 is the centre, 13
the top of the inner columns' left one, 38 the top of the outer columns'
left one. **[F]** The mask bit is number − 1 (section 2.3).

| Number | Name | Bit | Centre | Centre as read | Box as read |
|---|---|---|---|---|---|
| 1 | C6 | 0 | (3008, 2007) | (3008.0, 2006.6) | 162.9 x 162.8 |
| 2 | B6 | 1 | (3008, 1755) | (3008.0, 1755.4) | 162.9 x 165.6 |
| 3 | A2 | 2 | (3008, 1503) | (3008.0, 1502.8) | 162.9 x 165.6 |
| 4 | D6 | 3 | (3008, 2258) | (3008.0, 2257.8) | 162.9 x 165.6 |
| 5 | E2 | 4 | (3008, 2510) | (3008.0, 2510.4) | 162.9 x 165.6 |
| 6 | C7 | 5 | (3247, 2007) | (3246.7, 2006.6) | 162.9 x 162.8 |
| 7 | B7 | 6 | (3247, 1755) | (3246.7, 1755.4) | 162.9 x 165.6 |
| 8 | A3 | 7 | (3247, 1503) | (3246.7, 1502.8) | 162.9 x 165.6 |
| 9 | D7 | 8 | (3247, 2258) | (3246.7, 2257.8) | 162.9 x 165.6 |
| 10 | E3 | 9 | (3247, 2510) | (3246.7, 2510.4) | 162.9 x 165.6 |
| 11 | C5 | 10 | (2768, 2007) | (2767.9, 2006.6) | 165.7 x 162.8 |
| 12 | B5 | 11 | (2768, 1755) | (2767.9, 1755.4) | 165.7 x 165.6 |
| 13 | A1 | 12 | (2768, 1503) | (2767.9, 1502.8) | 165.7 x 165.6 |
| 14 | D5 | 13 | (2768, 2258) | (2767.9, 2257.8) | 165.7 x 165.6 |
| 15 | E1 | 14 | (2768, 2510) | (2767.9, 2510.4) | 165.7 x 165.6 |
| 16 | C8 | 15 | (3486, 2007) | (3485.5, 2006.6) | 162.9 x 162.8 |
| 17 | B8 | 16 | (3486, 1705) | (3485.5, 1704.9) | 162.9 x 165.6 |
| 18 | D8 | 17 | (3486, 2308) | (3485.5, 2308.3) | 162.9 x 165.6 |
| 19 | C9 | 18 | (3724, 2007) | (3724.2, 2006.6) | 162.9 x 162.8 |
| 20 | B9 | 19 | (3724, 1705) | (3724.2, 1704.9) | 162.9 x 165.6 |
| 21 | D9 | 20 | (3724, 2308) | (3724.2, 2308.3) | 162.9 x 165.6 |
| 22 | C10 | 21 | (3963, 2007) | (3962.9, 2006.6) | 162.9 x 162.8 |
| 23 | B10 | 22 | (3963, 1705) | (3962.9, 1704.9) | 162.9 x 165.6 |
| 24 | D10 | 23 | (3963, 2308) | (3962.9, 2308.3) | 162.9 x 165.6 |
| 25 | C11 | 24 | (4203, 2007) | (4203.1, 2006.6) | 165.7 x 162.8 |
| 26 | B11 | 25 | (4203, 1705) | (4203.1, 1704.9) | 165.7 x 165.6 |
| 27 | D11 | 26 | (4203, 2308) | (4203.1, 2308.3) | 165.7 x 165.6 |
| 28 | C4 | 27 | (2529, 2007) | (2529.1, 2006.6) | 165.7 x 162.8 |
| 29 | B4 | 28 | (2529, 1705) | (2529.1, 1704.9) | 165.7 x 165.6 |
| 30 | D4 | 29 | (2529, 2308) | (2529.1, 2308.3) | 165.7 x 165.6 |
| 31 | C3 | 30 | (2289, 2007) | (2289.0, 2006.6) | 162.9 x 162.8 |
| 32 | B3 | 31 | (2289, 1705) | (2289.0, 1704.9) | 162.9 x 165.6 |
| 33 | D3 | 32 | (2289, 2308) | (2289.0, 2308.3) | 162.9 x 165.6 |
| 34 | C2 | 33 | (2050, 2007) | (2050.3, 2006.6) | 162.9 x 162.8 |
| 35 | B2 | 34 | (2050, 1705) | (2050.3, 1704.9) | 162.9 x 165.6 |
| 36 | D2 | 35 | (2050, 2308) | (2050.3, 2308.3) | 162.9 x 165.6 |
| 37 | C1 | 36 | (1812, 2007) | (1811.5, 2006.6) | 162.9 x 162.8 |
| 38 | B1 | 37 | (1812, 1705) | (1811.5, 1704.9) | 162.9 x 165.6 |
| 39 | D1 | 38 | (1812, 2308) | (1811.5, 2308.3) | 162.9 x 165.6 |

**[F]** The eleven columns lie at 1812, 2050, 2289, 2529, 2768, 3008,
3247, 3486, 3724, 3963 and 4203, about 239 pixels apart. The three
inner columns (5, 6 and 7) have five rows, at 1503, 1755, 2007, 2258
and 2510, about 252 apart; the eight outer columns have three, at 1705,
2007 and 2308, about 302 apart. The centre point, 1, lies at the frame's
centre (3008, 2008) within 2 pixels. The table stores the readings
rounded to the pixel, not a grid fitted to them: the readings are the
evidence, and a grid would be a guess at what the maker's software
draws.

### 2.5 How the table was read, and its tolerance

NX Studio fitted each picture to its window: at 36 % of its size for
the landscape FX frames, so that one screen pixel is about 2.8 frame
pixels and the box is 58 screen pixels wide; at 24 % for the portrait
frames (4.2 frame pixels, a box of 38); and at 54 % for the DX frames
(1.8 frame pixels, a box of 89). The screenshots were measured by pixel, not by eye: a script found
the picture's bounds in the window, taking the median of many rows
because the viewer's information overlay at the top-left fools a search
of a single row, and the pure-red box's outer bounds. Converted to the
frame's pixels and rounded, a reading is good to about 2 pixels, and
the table's tolerance is set at ±3.

**The live-view frames calibrate the reading.** Frames 53 to 56 record
their focus area in the file, in the 6016x4016 frame: an area of
540x540 centred at (343, 3673), (343, 343), (5671, 343) and
(5671, 3673), the four limits of the area's travel. The screenshots
read (342.6, 3670.8), (342.6, 342.4), (5671.9, 342.4) and
(5670.5, 3670.8), with a box of 539x539: NX Studio draws the box with
its outer edge at the recorded area's bounds, so the outer edge is what
the table records, and the reading is right to about 2 pixels. **[F]**

**The repeats read the same.** Frames 40 and 41, points 1 and 13 shot
again, read identically to their originals. **[F]**

### 2.6 The control frames

- **Portrait (frames 47 and 48, Orientation 8).** The files' bytes are
  the same as in landscape, points 13 and 38; NX Studio draws the point
  turned with the picture, at (1501, 3246) and (1703, 4202) in the
  turned 4016x6016 view, which is the landscape table's (2768, 1503)
  and (1812, 1705) rotated, within 2 pixels. The table stays in stored
  orientation, as every size and preview raw-files reports, and a
  viewer turns the point with the picture. **[F]**
- **DX crop (frames 49 and 50).** Points 38 and 27 land on the
  3936x2624 DX picture at (770, 1009) and (3162, 1613): their
  positions in the table less (1042, 696) and (1041, 695), that is,
  within 2 pixels of CropHiSpeed's origin (1040, 696), which is
  ((6016 − 3936) / 2, (4016 − 2624) / 2): the centred window of the
  output frame, in output pixels, with no correction for the border:
  CropHiSpeed's cropped size, 3968x2636, counts 32 and 12 pixels that
  the 3936x2624 picture does not have, as the raw image's 6080x4028
  counts 64 and 12 that the 6016x4016 output does not.
  So a point placed from the table on a DX frame shifts by
  CropHiSpeed's origin, the rule of
  [ADR018](../../architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md).
  **[F]** Whether the shift is CropHiSpeed's
  origin on other bodies is a fact per body, which the two crop frames
  of the procedure check; the D7200's 1.3x files record other offsets
  ([makernote-and-metadata.md](makernote-and-metadata.md#86-focus-point)).
- **Auto-area (frames 42 to 44).** Area mode 8; primary points 22, 22
  and 4; masks of 12, 13 and 9 bits. NX Studio draws one box per bit,
  and the union of the boxes measured spans the columns and rows the
  bits name. **[F]**
- **Dynamic area, 21 points (frames 51 and 52, AF-C).** Area mode 5,
  primary point 1, one bit set: the file records the point chosen, not
  the points the mode may use. **[F]**
- **Manual focus (frames 45 and 46).** FocusMode `MANUAL`, layout code
  0, area mode 0, point number 0, an empty mask and a zero frame: a
  manual-focus shot names no point at all. **[F]**
- **Live view (frames 53 to 56).** Detection method 1, area mode 4,
  layout code 0, point number 0, and a recorded position, which the
  existing rules for a recorded position give
  ([ADR012](../../architecture-decisions/adr012-focus-points-as-the-camera-records-them.md)).
  **[F]**

### 2.7 The frames chosen as fixtures

Four files from three frames serve as test fixtures once stripped of
their pixels and
personal fields, as the Z 9 fixtures were
([`tests/fixtures/`](../../../tests/fixtures/README.md)); their names and
hashes are recorded there when they are committed:

- frame 4, point 38 (B1, the top-left of the outer columns), as a NEF
  and as its JPEG: a single point far from the centre, where a wrong
  table, a wrong bit order or a wrong frame shows, on both paths;
- frame 49, point 38 in DX: the shift into the crop;
- frame 42, auto-area: the mask of points used, 12 bits, with the
  primary among them.

## 3. Bodies without a table

Every other body is to give no point for a shot that names one, with
the reason counted. What the maintainer's own files of other Nikon bodies
record, read with the same script, as counts only **[F]**:

| Body | Files | AFInfo2, layout | With a point number | Without | Live view |
|---|---|---|---|---|---|
| D300s | 647 | `0100`, 51-point (code 1) | 647, 600 of them point 1 | 0 | 0 |
| D7100 | 400 | `0100`, 51-point | 392 | 8 | 5, one of them a 16:9 crop |
| D7200 | 6,277 | `0100`, 51-point | 5,950 | 327 | 156, one of them a 1.3x movie crop |
| D3200 | 3,568 | `0100`, 11-point (code 2), numbers 1 to 11 | 3,317 | 251 | 28 |
| Z 9 | 8,976 | `0400`, no number | 0 | 8,976; 62 of the 168 with a zero frame carry a mask only (20 auto-area, 42 3D-tracking) | 8,808 with a position |

On the D300s, D7100, D7200 and D3200 files, every single-point and
dynamic-area file with a number has one mask bit set, and an auto-area
file 2 to 51; so the bit order
of the 51-point and 11-point layouts can be read the same way once
frames with known points exist. The D600's own files, 1,374 of them,
have a point number on 1,339 and none on 35 (34 through the viewfinder,
1 in live view); 94 of them have an empty mask, the 35 and 59 with a
number (23 single-point, 36 dynamic-area), so the mask is not always
written beside the number; its 13 auto-area files, and the session's
three, set between 2 and 16 bits. **[F]**

A table for the 51-point bodies, the 11-point bodies or the Z 9's
mask-only shots needs a session on that body by the procedure of
section 1; the files above cannot give it, since no frame among them
records a known point. The public samples of raw.pixls.us show which
bodies name a point through the viewfinder
([test-data.md](test-data.md#23-files-checked-for-the-focus-point)).

## Sources

- [1] ExifTool, Nikon tags, the AFInfo2 tables and the point lists,
  read as documentation — https://exiftool.org/TagNames/Nikon.html
