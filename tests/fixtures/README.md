# Test fixtures

Five real files from a Nikon Z 9, two NEF files and three JPEG files,
small enough to commit, that the tests read as they are. Every other
fixture is built in code; these five are the only files here that a
camera wrote.

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

The maintainer took the two NEF pictures with their own Z 9 on
2026-10-19, one in DX crop mode and one in 1:1, with a single autofocus
point placed in the top-left corner of the picture. They were taken to
tell apart the two ways a crop mode records its focus point: as a
fraction of the cropped picture, which the reader scales, or as a
position in the full frame, which the reader shifts into the cropped
window. A point near the centre lands in the same place either way, and
every public sample of a crop mode has its point near the centre, so no
public sample can tell the two apart.

The maintainer took the three JPEG pictures with the same camera on
2026-10-22, to check what a camera JPEG records in each crop mode and
image size
([ADR015](../../docs/architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)):
one in FX, the camera's full output, at full size; one in DX crop mode at
full size; and one in 16:9 crop mode at the middle size, taken in
portrait orientation, so that the picture is stored on its side with
Orientation 8. None of the public JPEG samples the tests use is shot in
a crop mode or in portrait orientation.

## What was kept, replaced and removed

### The NEF files

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
  GPS IFD; and every other MakerNote value stored outside its entry.

Once they were stripped, the maintainer checked that no byte sequence of
the removed serial numbers or shutter count remains in either file, that
neither holds a run of six or more digits, and that the focus fields,
CropHiSpeed, CropArea and the raw size are those of the camera's file.

### The JPEG files

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

Once they were stripped, the maintainer checked that no byte sequence of
the zeroed serial numbers or shutter count remains in any of the three
files, as text or as a binary number in either byte order, nor the bytes
of the time the camera was switched on; that every
GPS value is zero and no XMP is left; that the Exif segment differs from
the camera's only inside the zeroed values and the thumbnail; that no run
of six or more digits remains outside the capture times but the
four-digit version fields that MakerNote blocks start with; and that
every image keeps its frame size, and the focus fields, CropHiSpeed,
FocusMode, the orientation, the exposure, the capture times and the lens
are those of the camera's file.

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

The tests also cut all five files at every length and overwrite each of
their bytes, but, in the JPEG files, those of three spans of zeros that
no reader parses, the zeroed ShotInfo, the padding after the thumbnail's
stub and the camera's zeros after the thumbnail's declared length, up to
the end of the Exif segment; and the fuzzer starts from copies of them.

## Licence and changes

The maintainer, who took the photographs, publishes these files under
the repository's licence, MIT OR Apache-2.0, at your option.

The NEF files were made on 2026-10-20, the JPEG files on 2026-10-22.
Only the maintainer can regenerate them, from the camera's original
files, which are not public; that is why this page records how they were
made and what was removed. A change to any file changes its SHA-256
above, which must then be updated in the same change.
