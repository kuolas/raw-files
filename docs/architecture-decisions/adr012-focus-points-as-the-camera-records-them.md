# ADR012: Focus points as the camera records them

- Status: Accepted
- Amends: [ADR011](adr011-metadata-from-standard-tags.md), in one point:
  the Sony MakerNote is read, for the focus point alone
- Date: 2026-10-19
- Issue: #25

## Context

A fast culler, a program that helps a photographer pick the best
pictures of a shoot, rates the sharpness of each picture from what the
file already holds, without decoding the raw data: it measures on the JPEG
preview embedded in the file, at the point where the camera focused. For
each file it needs that point, the size of the area around it when the
camera records one, whether the camera confirmed focus, and all of it in
coordinates that it can carry onto the preview it shows.

The point the camera records is where its autofocus aimed, not proof of
where the picture is sharpest: a body that focuses in front of or behind
the subject, a subject that moved, or a missed focus puts the sharpest
area elsewhere. A fast culler accepts that, for the speed of using what
the file already holds; slower ways of rating sharpness, which measure
the picture itself, can use the recorded point as a reference.

Cameras record where they focused in their MakerNote, the block of
manufacturer tags described in the format notes for
[NEF](../formats/nef/makernote-and-metadata.md) and
[ARW](../formats/arw/makernote-and-metadata.md). Some record a position in
pixels. Others record only the number of an autofocus point, and where
that point lies on the picture depends on each body's layout of points,
which no file carries.

We read what the public-domain (CC0) sample files of
[raw.pixls.us](https://raw.pixls.us/) record: 30 Nikon files from the D1H
to the Z5II and 36 Sony-family files from the DSC-F828 to the ILCE-7M5.
To these we added 13 shots made for this check with the maintainer's
Z 9, in its FX, DX, 1:1 and 16:9 modes, with the focus point placed in
the top-left corner of the picture.
Where the fields lie inside each block comes from ExifTool's published
tag tables, confirmed on the files whose fields hold values. The files
showed:

- **Nikon records a position in pixels in its AFInfo2 tag** (MakerNote
  tag 0x00B7), a plain value, not encrypted, that starts with a
  four-character version. Each version has its own layout, and at its
  offsets lie the size of a frame, the centre of the focus area and the
  area's width and height, in the MakerNote's byte order, and, on most
  versions, a byte saying whether focus was confirmed. The samples hold
  versions 0100, 0101, 0200, 0300, 0400, 0401 and 0402; the metadata
  listings of the raw.pixls.us catalogue add 0301 (D6, Z 5, Z 6II, Z 7II,
  Z 30 and Z fc) and, besides the Nikon 1's 0200 and 0201, no other.
  Mirrorless bodies fill in the position (Z 6, Z 9, Z f, Z6III and
  Z50II: files 3587, 5146 to 5148, 6887, 7808 to 7810, 7812, 7813, 7767
  and 7770), and so do DSLRs in live view (D7000, D5600 and
  Coolpix A: files 961, 1415 and 2238).
- **The position is the centre of the area.** On the Coolpix A it is
  exactly half the frame: (2464, 1632) in 4928×3264 (file 2238).
- **The frame is the camera's full-size output, not the stored raw.** On
  the Z 9 it is 8256×5504, the size of the visible image and of the
  largest preview, inside an 8280×5520 raw (files 5146 to 5148). On the
  Z 6's small raw it is still 6048×4024, the body's full-size output,
  while the stored image is 4548×3030 (file 3587): the point scales with
  the picture.
- **On a crop mode the frame stays the full output.** A Z6III 1:1 file
  stores 4048×4040 pixels and shows 4032×4032 of them, yet its AFInfo2
  frame is 6048×4032 (file 7808); its 16:9 and DX files and the Z50II's
  1:1 and 16:9 files do the same (7809, 7810, 7767, 7770). The tag
  CropHiSpeed (0x001B) tells these files apart: its cropped size differs
  from its original size on them, and equals it on every sample that is
  no crop, except the D810's, whose cropped width is four pixels short
  (file 2036).
- **A DX crop is scaled into the frame; an aspect crop is a window in
  it.** On every CC0 crop file the point lies within 12 pixels of the
  centre, which cannot tell the two apart; the Z 9 shots can. With the
  point in the top-left corner, the FX shot records the centre (502, 424)
  and an area of 291×323 in 8256×5504. The DX shot records (636, 572)
  and 420×468: shifted by the DX window's origin it would lie outside
  the picture, while scaled onto the 5392×3592 crop it is (415, 373) and
  274×305, near the corner and about the FX area. The 1:1 shot records
  (1797, 424) and the 16:9 shot (502, 715), both with the FX area of
  291×323: shifted by the centred window's origin they are (421, 424) in
  5504×5504 and (502, 283) in 8256×4640, as near the corner as in FX,
  while scaled they would move to 22 % and 13 % of the way in and their
  single point would shrink. So a crop that keeps the aspect of the full
  frame (DX) records fractions of the cropped picture, and a crop that
  keeps one side (1:1, 16:9) records positions in the full frame; the
  Z50II's 1:1 and 16:9 areas (278×276, 280×280) fit the window too.
- **A DSLR shot through the viewfinder records no position.** On the
  D300, D700, D750, D810, D850 and D780 the frame and area are zero and
  only a point number and a mask of the points used are filled in
  (files 928, 2255, 895, 2036, 1835 and 3827). The Nikon 1 J1's version
  0200 holds a point number only (file 2956), and the Z5II in auto-area
  mode a mask only (files 7735 and 7736). The D1H, D70, D100, D200 and
  the Coolpix E5700 and P7800 write no AFInfo2 at all.
- **Nikon records manual focus in plain text.** Its FocusMode tag
  (0x0007) reads `MANUAL` on the D300 and D850 samples, both shot through
  the viewfinder with a zero frame (files 928 and 1835), and `AF-S`,
  `AF-C` or `AF-A` on every other sample but the D1H's, whose older
  MakerNote is not read.
- **Sony records a position in its FocusLocation tag** (0x2027), four
  numbers in a plain tag of the MakerNote, outside its enciphered blocks:
  the frame's width and height and the centre's x and y. Every sample
  from the ILCE-7RM2 (2015) on has it; the ILCE-7S, the NEX-5, the
  RX100, the A-mount bodies, the Hasselblad Lunar and the older formats
  do not. Bodies from the ILCE-7SM3 (2020) on add the area's width and
  height in FocusFrameSize (0x2037), and the ILCE-9M3, ILCE-1M2 and
  ILCE-7M5 repeat the location in FocusLocation2 (0x204A), with the same
  values.
- **The position is the centre there too.** ExifTool describes the
  focus area as centred on FocusLocation; several autofocus shots with a
  recorded area sit at exactly half the frame, such as the ILCE-1's
  (4320, 2880) in 8640×5760 with a 1026×912 area and the ILCE-6700's
  (files 4466 and 6736); and the ILCE-7RM3's location agrees with the
  selected spot, which another tag records on a coarse grid (file 3251).
- **Sony's frame is the picture as framed in the camera**, which on an
  aspect-ratio setting is not the visible image raw-files reports as the
  image size: the RX100M6's 1:1 file has a 3648×3648 frame and a
  1088×1080 preview, while its default crop is 5472×3648 (file 3012). On
  the ILCE-7M4's medium and small raws the frame is the full-size
  7008×4672, larger than the stored image (files 6930 and 6931).
- **A Sony shot in manual focus still records a location**: the exact
  centre of the frame, with FocusMode (0x201B) 0, on the ILCE-7RM2 and
  the ILCE-9M3 (files 1310, 1311, 7361 and 7364).
- **Whether focus was confirmed** is recorded by Nikon's versions 0100,
  0101 and 04xx; the Z6III samples record a miss beside a valid point.
  No plain Sony tag records it; ExifTool decodes an autofocus status
  from an enciphered block on the SLT bodies only.
- On every sample, each of these values lies inside the length the
  MakerNote declares.

## Decision

We will report a focus point wherever the file records one, and none
only where the file records none or what it records cannot be used. A
position in pixels is reported as the camera recorded it. The number of
an autofocus point is reported by mapping it to pixels through a table
of that body's points. No file carries such a table, so the mapping
waits for a decision on where such tables may come from, and a body
without a table gives none.

- **Only what the file records.** `Metadata::focus` holds the camera's
  record and nothing estimated from the picture. A position found by
  analysing the pixels, by any method, is a different value and gets a
  name of its own, so that a caller always knows which one it has.
- **What it is.** `Metadata::focus` returns a `Focus`: the centre of the
  focus area as a `Point`, the area's size when the file records one,
  the frame the two are measured in, and whether the camera confirmed
  focus, when the file says. Coordinates count pixels from the left and
  top of the frame, in the orientation of the stored image, like every
  size and preview raw-files reports.
- **The frame has the framing of the previews.** It is the picture as the
  camera framed it, at whatever scale the file records. To place the
  point on a preview, a caller multiplies x and the width by the
  preview's width over the frame's, and y and the height by the heights;
  `Focus::scaled_to` does it. The frame equals the image size except
  where the camera's framing differs from the default crop (Sony's
  aspect ratios), where the stored raw is a smaller copy of the output
  (small raws), or where the file records no visible size, so that the
  image size includes the border pixels (the Nikon bodies without a
  CropArea, such as the D7000 and the D5600: files 961 and 1415).
- **Nikon: AFInfo2, read only at a version we know.** The layout is
  chosen by the exact version, among 0100, 0101, 0300, 0301, 0400, 0401
  and 0402. Any other version gives no focus point until its layout is
  checked on a file and added. The frame is AFInfo2's as recorded,
  except on a crop: when CropHiSpeed's cropped size differs from its
  original size and the file has a CropArea, the frame becomes
  CropArea's width and height. If both sides of the crop are smaller
  (DX), the centre and the area are scaled onto it on each axis, rounded
  to the nearest pixel. If one side is kept (1:1, 16:9), the crop is a
  centred window: the centre moves back on the cut axis by half the
  difference between the recorded frame and CropArea, and the area is
  kept. That holds only when CropArea's kept side equals the recorded
  frame's, as on every crop sample with a position; otherwise, as on a
  small raw, whose CropArea counts stored pixels, the point cannot be
  placed and the file gives none.
- **Sony: the MakerNote is read, for the focus point alone.** raw-files
  finds the Sony MakerNote's directory, with no header or after one of
  Sony's 12-byte headers, and reads FocusLocation, FocusLocation2 when
  FocusLocation is absent, FocusFrameSize and FocusMode. Nothing else in
  it is read and nothing is deciphered. This changes one point of
  [ADR011](adr011-metadata-from-standard-tags.md), which read no Sony
  MakerNote.
- **Whether focus was confirmed** is Nikon's FocusResult on the 04xx
  versions and ContrastDetectAFInFocus on 0100 and 0101: 1 is yes, 0 is
  no, any other value unknown. Version 03xx and Sony files do not say.
- **Point numbers need a table of the body's points.** A DSLR shot
  through the viewfinder, the Nikon bodies that write only the older
  AFInfo tag (0x0088), the Nikon 1 bodies, a Z body in auto-area mode
  that records only a mask, and the Sony bodies before the ILCE-7RM2
  whose plain tags hold the point by number (according to ExifTool's
  tables, CameraInfo on the A-mount and NEX bodies and AFPointSelected on
  the ILCE-7S; the SLT-A99V also writes a mask of its points, file 3751)
  record the number of an autofocus point, or a mask of the points used,
  and no position. Until raw-files has a table for a body, its files give
  none.
- **No focus point** when the file records none: the Sony files with
  neither a FocusLocation nor a point number in a plain tag (among the
  samples, the RX100, the Hasselblad Lunar, whose focus data lies only in
  an enciphered block, and the SR2 and SRF files), and a Sony shot in
  manual focus, whose location is the frame's centre with no focusing
  behind it. A Nikon shot whose FocusMode, trimmed of its padding, is
  `MANUAL` gives none for the same reason, whatever AFInfo2 holds. A side
  of the frame or of the area that is zero is how a Nikon file says it
  records no position, and gives none too. On Sony a zero side in
  FocusFrameSize means only that no size is recorded: the point is kept,
  without a size.
- **None either when what the file records cannot be used**, on every
  format: a value shorter than its layout, a frame with a zero side, a
  centre outside the frame, or an area that does not fit inside the
  frame around its centre.
- **Read when the file is opened, inside the MakerNote.** A value is
  read only when it lies inside the MakerNote's declared length; Sony's
  offsets count from the start of the file, Nikon's from the MakerNote's
  own header. A damaged value costs only the focus point, never the file
  or the other metadata.

Rejected:

- **Mapping point numbers without a decision on the tables.** Open data
  for the layouts exists, but taking a per-body table into raw-files
  needs its own decision on where such tables may come from.
- **Coordinates in the frame of the image size.** On a Sony aspect-ratio
  file the previews show the camera's framing, not the default crop, and
  a caller could not place the point on them.
- **Nikon's values as stored on a crop.** A Z6III 1:1 file would report a
  6048×4032 frame for a 4032×4032 picture, and a caller scaling the point
  onto the square preview would misplace it.
- **One rule for every crop.** Scaling every crop moves the point of a
  1:1 or 16:9 shot away from where the camera focused and shrinks its
  area; shifting every crop puts the point of a DX shot outside the
  picture. The Z 9 shots show both.
- **Always taking CropArea as the frame.** The frame would equal the
  image size on every Nikon file with a CropArea, but values the file
  records would change where no crop makes it necessary: the Z 6 small
  raw keeps its 6048×4024.
- **A rectangle (left, top, width, height)** in place of a centre and a
  size. It converts the stored centre, and holds nothing when the size is
  absent.
- **Fractions of the frame.** Floating-point values lose `Eq` and `Hash`
  and the values the file recorded.
- **Choosing the AFInfo2 layout by the version's first two digits.** It
  would also read versions that no file has shown, such as a future
  0403, at offsets that may have moved. The versions read are those
  found in files, each at ExifTool's published layout, even where no
  sample fills it in.
- **Reporting the point of a manual-focus shot**, with a focus mode
  beside it: a point the camera did not focus on, which a caller would
  measure for nothing, and a maker's value in a type shared by every
  format.
- **Exposing the area mode, point numbers or masks.** No caller needs
  them; they can be added without breaking the API.

## What this decision does not cover

How sharp a picture is, where its sharpest area lies, whether the camera
focused in front of or behind the subject, and what a culler does with a
file that records no focus point (for example, take the point of the
next frame of a burst) are separate decisions. They build on this one
and do not change it.

## Consequences

- Nikon mirrorless bodies, Nikon DSLRs in live view and Sony bodies from
  the ILCE-7RM2 on give a focus point. A DSLR shot through the viewfinder,
  the older Nikon bodies, the Nikon 1 bodies and the Sony bodies before
  the ILCE-7RM2 that record a point number give none until raw-files has
  a table of their points; the Sony files with no focus data in a plain
  tag and a manual-focus shot give none at all.
- A point number can be mapped only for a body whose layout of points
  is known. No open source of the layouts of Sony's A-mount, NEX and
  early ILCE bodies, or of the Nikon bodies older than the D3 and the
  D300, is known to us, so those may stay without a focus point.
- A caller scales the point onto the preview it shows, and must not
  assume that the frame equals the image size.
- The coordinates are those of the stored image: a viewer that rotates
  the preview for display rotates the point with it.
- A body that brings a new AFInfo2 version gives no focus point until
  the version is added.
- The 0101 layout and the 0301 version rest on ExifTool's tables: no CC0
  sample fills in the 0101 position, and none has version 0301.
- The crop rule relies on CropHiSpeed's two sizes differing only on a
  crop. The D810's four-pixel difference on an uncropped file is
  harmless, since the D810 writes no CropArea. A body that wrote both
  would have its point moved by half the gap between AFInfo2's frame and
  CropArea: nothing where the two are equal, as on every full-size
  uncropped file with a position and a CropArea (files 5146 to 5148 and
  6887), and no point at all on a small raw, whose CropArea is smaller
  than the frame on the side the crop keeps.
- A 1:1 or 16:9 crop on a small raw gives no focus point, since its
  CropArea's kept side differs from the frame's. A DX crop on a small raw
  is scaled onto CropArea, in stored pixels. No sample shows either.
- Which rule applies to a crop that cuts both sides and changes the
  aspect, such as a DX crop at 1:1 on an FX body, is unchecked: the rule
  scales it, and no sample shows one.
- Whether a Nikon DSLR in live view records a cropped or a full frame on
  a DX or 1.2× crop, and what a Nikon Z body in manual focus writes in
  AFInfo2, are unchecked; no sample shows either. The Z body's FocusMode
  gives no point either way.
- That a zero side in Sony's FocusFrameSize keeps the point rests on
  manual-focus files only (files 7361 and 7364), which give no point
  anyway.
- Sony files say in no plain tag whether focus was confirmed. ExifTool
  finds it in an enciphered block on the SLT bodies only, and reading
  that block would need its own decision.
- A file without a focus point has a known reason. The format notes list
  every place each maker records focus data, read or not, and which
  bodies and focus modes the samples cover, so that the gaps still to
  close are named.
- Opening a NEF reads nothing more, since AFInfo2 lies among the
  MakerNote's tags. Opening a Sony file also reads its MakerNote's
  directory, about a hundred entries near the start of the file.
- Mapping point numbers fills the same `Focus` for the files that record
  only a number; no signature changes.
- Ways of rating sharpness that measure the picture can use the recorded
  point rather than replace it: comparing it with the sharpest area they
  find shows a body that focuses in front of or behind the subject.
- A culler that takes the point of another frame of a burst needs to
  know which files form a burst, which raw-files does not report; that
  would be a metadata field of its own, added beside `Focus`.
