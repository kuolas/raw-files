# Test fixtures

Two real NEF files from a Nikon Z 9, small enough to commit, that the
tests read as they are. Every other fixture is built in code; these two
are the only files here that a camera wrote.

| File | Crop mode | Frame | SHA-256 |
|---|---|---|---|
| `nikon-z9-dx.nef` | DX | 9908 | `2f8db98a8e19788f9b3427ec1530591833173962ceceaa7e693ab30579185157` |
| `nikon-z9-1x1.nef` | 1:1 | 9909 | `2ddfe140ca6f8cfbdd3aec355342a2a72fd9da13f847c5f532123c5e397bb263` |

## Where they come from

The maintainer took both pictures with their own Z 9 on 2026-10-19, one
in DX crop mode and one in 1:1, with a single autofocus point placed in
the top-left corner of the picture. They were taken to tell apart the two
ways a crop mode records its focus point: as a fraction of the cropped
picture, which the reader scales, or as a position in the full frame,
which the reader shifts into the cropped window. A point near the centre
lands in the same place either way, and every public sample of a crop
mode has its point near the centre, so no public sample can tell the two
apart.

## What was kept, replaced and removed

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

## What they test

The crop rule of the focus point, on real camera bytes, beside the
previews and the metadata of a real Z 9 file. In the full 8256 by 5504
frame:

- `nikon-z9-dx.nef`, the DX crop, which cuts both sides: AFInfo2 records
  the centre (636, 572) and an area of 420 by 468, and the reader scales
  them onto CropArea's 5392 by 3592, giving the centre (415, 373) and an
  area of 274 by 305.
- `nikon-z9-1x1.nef`, the 1:1 crop, which keeps the height: AFInfo2
  records the centre (1797, 424) and an area of 291 by 323, and the
  reader moves the centre into CropArea's 5504 by 5504 window, 1376
  pixels from the frame's left edge, giving the centre (421, 424) and the
  same area.

The tests also cut both files at every length and overwrite each of
their bytes, and the fuzzer starts from copies of them.

## Licence and changes

The maintainer, who took the photographs, publishes these files under
the repository's licence, MIT OR Apache-2.0, at your option.

The files were made on 2026-10-20. Only the maintainer can regenerate
them, from the camera's original files, which are not public; that is
why this page records how they were made and what was removed. A change
to either file changes its SHA-256 above, which must then be updated in
the same change.
