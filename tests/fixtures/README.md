# Test fixtures

Real files that a camera wrote, small enough to commit, that the tests
read as they are: five from a Nikon Z 9, two NEF files and three JPEG
files, listed below, and the 56 NEF files of a session shot with a
Nikon D600, in [`nikon-d600/`](nikon-d600/README.md), which lists them.
Every other fixture is built in code.

| File | Crop mode | Frame | SHA-256 |
|---|---|---|---|
| `nikon-z9-dx.nef` | DX | 9908 | `2f8db98a8e19788f9b3427ec1530591833173962ceceaa7e693ab30579185157` |
| `nikon-z9-1x1.nef` | 1:1 | 9909 | `2ddfe140ca6f8cfbdd3aec355342a2a72fd9da13f847c5f532123c5e397bb263` |

| File | Crop mode | Image size | Orientation | Frame | SHA-256 |
|---|---|---|---|---|---|
| `nikon-z9-fx.jpg` | FX (no crop) | 8256×5504, full | 1 | 9915 | `87cfa8ee041fbc823f074b94dd0026b06dfd3cf1bed66492b1812afdd5c2c10b` |
| `nikon-z9-dx.jpg` | DX | 5392×3592, full | 1 | 9936 | `2d4e0736c7cd58963b250f1de47caf1161de42f7b4b2ae34ef6411ab178a4ac1` |
| `nikon-z9-16x9.jpg` | 16:9 | 6192×3480, middle | 8, portrait | 9954 | `74106bc62d37eac2b89e3b5f2c2d07639f54d21c25feee4dc577471c81a7c017` |

## Where they come from

Where this page says *I*, it is me, Alvaro Kuolas, who maintains raw-files,
writing about my own cameras.

I took the two NEF pictures with my own Z 9 on 2026-10-19, one in DX crop
mode and one in 1:1, with a single autofocus point placed in the top-left
corner of the picture. They were taken to tell apart the two ways a crop
mode records its focus point: as a fraction of the cropped picture, which
the reader scales, or as a position in the full frame, which the reader
shifts into the cropped window. A point near the centre lands in the same
place either way, and every public sample of a crop mode has its point near
the centre, so no public sample can tell the two apart.

I took the three JPEG pictures with the same camera on 2026-10-22, to
check what a camera JPEG records in each crop mode and image size
([ADR015](../../docs/architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)):
one in FX, the camera's full output, at full size; one in DX crop mode at
full size; and one in 16:9 crop mode at the middle size, taken in
portrait orientation, so that the picture is stored on its side with
Orientation 8. None of the public JPEG samples the tests use is shot in
a crop mode or in portrait orientation.

## What was kept, replaced and removed

### The Z 9's NEF files

Each file is the camera's file with its pixels and personal fields
removed, and its structure and the values the reader reads kept as the
camera wrote them.

- **Kept:** the TIFF header and IFD0 (Make, Model, Orientation, Software,
  DateTime, the pointers to the SubIFDs and the Exif IFD, and the small
  tags); the six SubIFDs with their tags; from the Exif IFD, the
  exposure, the capture times with their fractions of a second and
  offsets from UTC, and the lens's model and specification; the
  MakerNote, in its layout with an embedded TIFF header, with 32 of its
  80 entries: every entry whose value fits in the entry itself, and
  FocusMode, CropHiSpeed, CropArea, Lens, AFInfo2 and the PreviewIFD; and
  the raw SubIFD's block of compression parameters, NikonNEFInfo
  (0xC7D5).
- **Replaced:** every JPEG (IFD0's, SubIFD0's, SubIFD2's and the
  MakerNote's preview) by a stub JPEG of 122 bytes with no image in it,
  whose frame header keeps the original's size, the same stub the
  fixtures built in code use; the raw data, the pixels, by 64 bytes
  starting `FF 10 FF 50`; and the small grey image of SubIFD3 by 64 zero
  bytes.
- **Removed:** the serial numbers of the camera and of the lens
  (BodySerialNumber, LensSerialNumber and the MakerNote's SerialNumber);
  the shutter counts (the MakerNote's ShutterCount and
  MechanicalShutterCount); the MakerNote's encrypted blocks (ShotInfo,
  ColorBalance and LensData); the linearization table and the blocks of
  menu settings; the XMP packet; Artist, Copyright and UserComment; the
  GPS IFD; and every other MakerNote value stored outside its entry, the
  white balance setting's text (0x0005) and the white balance
  fine-tuning (0x003F) among them.

Once they were stripped, I checked that no byte sequence of the
removed serial numbers or shutter count remains in either file, that
neither holds a run of six or more digits, and that the focus fields,
CropHiSpeed, CropArea and the raw size are those of the camera's file.

### The Z 9's JPEG files

Each file keeps the camera's Exif segment, all 65,536 bytes of it, with
every IFD, value and offset where the camera put them, so that the reader
walks the camera's own layout; its images are replaced and its personal
fields zeroed in place. Each is 69,998 bytes.

- **Kept:** the start of the image (SOI); the Exif segment: IFD0 (Make,
  Model, Orientation, Software, DateTime, the resolution tags and the
  pointers to the Exif and GPS IFDs), the Exif IFD (the exposure, the
  capture times with their fractions of a second and offsets from UTC,
  and the lens's model and specification), the GPS IFD's one entry and
  the Interoperability IFD, IFD1, and the MakerNote, in its layout with
  an embedded TIFF header, with all 75 of its entries, FocusMode,
  CropHiSpeed, Lens and AFInfo2 among them; and the MPF segment, whose
  index lists the main image and two smaller ones.
- **Replaced:** every image by the stub JPEG of 122 bytes with no image
  in it, whose frame header keeps the original's size, the same stub the
  fixtures built in code use. The main image's tables, frame header and
  scan give way to the stub's, after the kept segments. IFD1's thumbnail,
  160 by 120, is written where it was, and the rest of the length IFD1
  declares for it is zeros, so that nothing in the Exif segment moves.
  The two MPF images, 640 by 424 and 1620 by 1080 (640 by 360 and 1920
  by 1080 on the 16:9 shot), each of which held a small Exif segment of
  its own in the camera's file, become stubs too, written after the main
  image, and the MPF index gives each stub's offset and size.
- **Zeroed in place**, each value's bytes set to zero with its tag, type,
  count and offset kept: the serial numbers of the camera and of the
  lens (BodySerialNumber, LensSerialNumber and the MakerNote's
  SerialNumber); the shutter counts (the MakerNote's ShutterCount and
  MechanicalShutterCount); the time the camera was switched on (the
  MakerNote's PowerUpTime); the MakerNote's encrypted blocks (ShotInfo,
  ColorBalance and LensData); Artist, Copyright and UserComment; and the
  GPS IFD's one value, its version.
- **Dropped:** the XMP segment, and the zeros the camera wrote before
  each MPF image.

Once they were stripped, I checked that no byte sequence of the zeroed
serial numbers or shutter count remains in any of the three files, as
text or as a binary number in either byte order, nor the bytes of the
time the camera was switched on; that every GPS value is zero and no XMP
is left; that the Exif segment differs from the camera's only inside the
zeroed values and the thumbnail; that no run of six or more digits
remains outside the capture times but the four-digit version fields that
MakerNote blocks start with; and that every image keeps its frame size,
and the focus fields, CropHiSpeed, FocusMode, the orientation, the
exposure, the capture times and the lens are those of the camera's file.

## What they test

The crop rule of the focus point, on real camera bytes, beside the
previews and the metadata of real Z 9 files. In the full 8256 by 5504
frame, for the NEF files:

- `nikon-z9-dx.nef`, the DX crop, which cuts both sides: AFInfo2 records
  the centre (636, 572) and an area of 420 by 468, and the reader scales
  them onto CropArea's 5392 by 3592, giving the centre (415, 373) and an
  area of 274 by 305.
- `nikon-z9-1x1.nef`, the 1:1 crop, which keeps the height: AFInfo2
  records the centre (1797, 424) and an area of 291 by 323, and the
  reader moves the centre into CropArea's 5504 by 5504 window, 1376
  pixels from the frame's left edge, giving the centre (421, 424) and the
  same area.

For the JPEG files, which hold no CropArea, the reader places the point
on the JPEG's own picture
([ADR015](../../docs/architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)):

- `nikon-z9-fx.jpg`, no crop: AFInfo2 records the centre (4225, 2614)
  and an area of 406 by 406 in a frame of the JPEG's own size, and the
  reader gives them as recorded.
- `nikon-z9-dx.jpg`, the DX crop: AFInfo2 records the centre (1024, 718)
  and an area of 226 by 258, and the reader scales them onto the JPEG's
  5392 by 3592, giving the centre (669, 469) and an area of 148 by 168.
- `nikon-z9-16x9.jpg`, the 16:9 crop at the middle size, which keeps the
  width: AFInfo2 records the centre (878, 4504) and an area of 162 by
  178, and the reader moves the centre into the window of the JPEG's
  shape, 8256 by 4640, 432 pixels below the frame's top edge, giving the
  centre (878, 4072) and the same area. Every preview takes the file's
  Orientation 8.

Each point is the one the reader gives on the raw file of the same shot,
placed on the same frame. The tests also check each JPEG's four
previews, where the camera's layout puts them, and every value of its
metadata.

The tests check every camera setting each file gives. From the Exif
IFD, the same on all five shots: aperture priority, multi-segment
(Nikon's matrix) metering, a flash that did not fire, with no mode
recorded, and auto white balance, which the JPEG files' MakerNote
confirms with `AUTO0`. From the MakerNote: the colour temperature auto
white balance chose, which differs from shot to shot; the electronic
shutter; and the image area of each crop mode, from CropHiSpeed. The NEF
files also give the large raw size, and the JPEG files a recorded zero
fine-tuning, `0/4, 0/4`; a JPEG gives no raw size, and the NEF files no
fine-tuning, since the stripping removed it. The JPEG files record the
colour space sRGB in their Exif IFD, which alone decides; the NEF files,
like every Nikon raw file checked, record no ColorSpace there, so their
MakerNote's sRGB is the one given.

The tests also cut all five files at every length and overwrite each of
their bytes, but, in the JPEG files, those of three spans of zeros that
no reader parses, the zeroed ShotInfo, the padding after the thumbnail's
stub and the camera's zeros after the thumbnail's declared length, up to
the end of the Exif segment; and the fuzzer starts from copies of them.

## The D600 session

I shot the 56 frames of [`nikon-d600/`](nikon-d600/README.md) with my
own D600 on 2026-10-26, RAW + JPEG, to measure where the body's
autofocus points lie on the picture, as the NEF notes record
([autofocus-points.md](../../docs/formats/nef/autofocus-points.md#2-nikon-d600)).
The whole session is committed, so that every value the record reads
from the files' bytes can be checked on them; the NEF files only, not
the JPEGs. The folder's README lists each frame with what it shows and
its SHA-256.

Each file is stripped by the rules of the Z 9's NEF files, adapted to
the D600's layout, and all 56 come out alike, 2,452 bytes each.

- **Kept:** the TIFF header, in the camera's big-endian byte order, and
  IFD0 (Make, Model, Orientation, Software, DateTime, DateTimeOriginal,
  the pointers to the SubIFDs and the Exif IFD, and the small tags); the
  three SubIFDs with their tags; the Exif IFD but for UserComment: the
  exposure, the capture times with their fractions of a second, and the
  other shooting settings it records; and the MakerNote, in its layout with an embedded
  TIFF header, with 31 of its 60 entries: every entry whose value fits
  in the entry itself but ShutterCount, and FocusMode, CropHiSpeed, Lens,
  AFInfo2 and the PreviewIFD, whose entries are kept but its resolution,
  stored outside its entries. The D600 writes no CropArea, no lens tags
  in the Exif IFD and no Interoperability IFD.
- **Replaced:** IFD0's thumbnail, an uncompressed RGB image of 160 by
  120, and the raw data, the pixels, each by 64 zero bytes; and every
  JPEG (SubIFD0's full-size one, 6016 by 4016, or 3936 by 2624 in DX,
  SubIFD2's of 1632 by 1080 and the MakerNote's preview of 570 by 375)
  by the stub JPEG of 122 bytes with no image in it, whose frame header
  keeps the original's size.
- **Removed:** the camera's serial number (the MakerNote's
  SerialNumber); the shutter count (the MakerNote's ShutterCount); the
  time the camera was switched on (the MakerNote's PowerUpTime); the
  MakerNote's encrypted blocks (ShotInfo, ColorBalance and LensData);
  the linearization table and the blocks of settings; the XMP packet;
  Artist, Copyright and UserComment; the GPS IFD; and every other
  MakerNote value stored outside its entry, the white balance setting's
  text (0x0005) among them.

Once they were stripped, I checked on each of the 56 files that no byte
sequence of the removed serial number, shutter count or switch-on time
remains, as text or as a binary number in either byte order, nor the
text of Artist, Copyright or UserComment; that it holds no GPS IFD, no
XMP, no owner's name and no serial number tag; that no run of six or
more digits remains; and that every value kept is the camera's, byte
for byte, AFInfo2, FocusMode, CropHiSpeed and the orientation among
them.

The tests open every frame and check what the reader gives it: the
model, the orientation, the image area that CropHiSpeed names, the
layout of the D600's 39 points, which the D600's table in the code
gives every frame by its Model, whatever the shot; on the frames shot
through the viewfinder in autofocus, which name their points by number,
those points placed from the table, in the crop's window on the DX
frames; no focus point on the manual-focus frames, which name none; and
on the four live-view frames the point AFInfo2 records. On frame 4, shot
through the viewfinder with point 38, they check every value too: its three previews, each a stub;
the capture time, with the offset from UTC that WorldTime gives, since
the D600 writes no OffsetTimeOriginal; the exposure; the lens's range,
which only the MakerNote records; the sizes; the camera settings, from
the Exif IFD aperture priority, multi-segment metering, a flash set off
that did not fire and auto white balance, and from the MakerNote a
recorded zero fine-tuning (0x000B), whose two values fit in their entry
and so were kept, and sRGB; and point 38 at (1812, 1705) with its box of 164 by 164,
placed from the table as its focus point and in the layout. It gives
no colour
temperature, shutter, image area or raw size: the D600 writes no
ColorTemperatureAuto, ShutterMode or ImageSizeRAW, the stripping
removed the white balance text, and CropHiSpeed's crop setting, 0,
names no image area. The tests that cut files
at every length and overwrite their bytes take frame 4, which stands
for the others' layout, and the fuzzer starts from copies of frames 4,
42 and 49.

## Licence and changes

I, who took the photographs, publish these files under the
repository's licence, MIT OR Apache-2.0, at your option.

The Z 9's NEF files were made on 2026-10-20, its JPEG files on
2026-10-22, and the D600's NEF files on 2026-10-27. Only I can
regenerate them, from the camera's original files, which are not
public; that is why this page records how they were made and what was
removed. A change to any file changes its SHA-256, above or in
[`nikon-d600/README.md`](nikon-d600/README.md), which must then be
updated in the same change.
