# JPEG: open questions

What the [JPEG documentation](README.md) does not yet settle. Part 1 lists
decisions for the maintainers. Part 2 lists facts that sample files can
establish. Part 3 lists sources that were not read. Part 4 lists questions
that the sample files have already settled. Each item names the page it
affects; [test-data.md](test-data.md) describes the files.

## 1. Decisions for the maintainers

1. **Telling an export from a camera's JPEG.** An exported JPEG may keep
   the camera's MakerNote while its pixels were cropped, scaled or turned.
   [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)
   keeps a focus point only when its frame has the picture's aspect
   ratio, which an export cropped to the same shape passes with its point
   in the wrong place. Whether other evidence (an editor's XMP, a JFIF
   segment, the Software tag) should refuse more points is left out of
   that decision; such marks did not tell whether the pixels kept the
   camera's framing. Affects
   [container-and-metadata.md](container-and-metadata.md#13-pitfalls).
2. **XMP.** An XMP segment can record an editor's crop, rating or labels,
   and a camera's rating. raw-files does not read it; reading it would
   need a decision of its own. Affects
   [container-and-metadata.md](container-and-metadata.md#5-other-application-segments).
3. **A MakerNote preview outside the declared MakerNote.** The NEF
   reader refuses a PreviewIFD JPEG that does not lie wholly inside the
   MakerNote's declared length, which costs one Coolpix NRW file its
   preview (#18). The JPEG reader is to apply the same rule. If the rule
   is relaxed for that file (#18), the question is whether a JPEG should
   follow, since an editor that
   rewrites the Exif segment can leave such a pointer behind. Affects
   [container-and-metadata.md](container-and-metadata.md#10-makernotes-in-a-jpeg).

## 2. Facts to establish from sample files

### Previews and sizes

- **The orientation of the small images.** Every preview takes IFD0's
  orientation. Whether a camera stores its thumbnail and its MPF images
  turned, or already upright, when it records a portrait picture is not
  shown by any file in the set: all have orientation 1. A camera JPEG of
  a portrait picture, from each maker, would settle it. [I]
- **Sony's padding** after the last image: three of the five Sony files
  pad to a multiple of 64 KiB with zeros, the RX100M4 and the A100 do
  not. Which bodies pad, and whether any pads with something other than
  zeros, needs more files. [F] [I]
- **The 12 bytes before the G7 X's MPF image** are not zeros; what they
  are is unknown, and nothing depends on it. [F]
- **Sony's 1616×1080 large thumbnail** does not quite keep the main
  image's 3:2 aspect ratio, as the Multi-Picture Format asks of a large
  thumbnail; whether other Sony bodies and image sizes do the same is
  unchecked. [F] [P]
- **The MP Entry flags.** The G7 X does not mark its first image as the
  representative image; whether any file marks another image so, and
  whether a reader should care, is open. [F] [I]
- **Two Exif segments.** No file in the set has a second Exif segment;
  the reader takes the first. A real file with two would show which one
  holds the camera's data. [F] [I]

### MakerNotes and the focus point

- **A Nikon JPEG with a focus position.** The D5300 file was shot
  through the viewfinder (a zero frame) and the D70 writes no AFInfo2,
  so no public Nikon JPEG in the set shows a position. A camera-original
  JPEG from a Z body, or from a DSLR in live view, under a usable
  licence, would. [F]
- **A Nikon JPEG shot in a crop mode** (DX, 1:1, 16:9) and at a smaller
  image size: what its AFInfo2 frame, CropHiSpeed and CropArea record,
  and whether the frame then fits the JPEG's picture. [I]
- **The aspect rule's tolerance** of one pixel of rounding has been
  checked only on files whose frame equals the picture's size. A camera
  JPEG at a smaller size, whose frame may stay the full-size one, would
  test it. [I]
- **The Sony PreviewImage pointer.** The ILCE-6000's MakerNote tag 0x2001
  points into its MPF image, outside the Exif segment, and is dropped.
  Whether other Sony bodies write it, and where it points, is unchecked;
  no reader needs it. [F]
- **Other makers.** No camera-original Fujifilm, Panasonic or phone JPEG
  with a usable licence was found; Canon's MakerNote is not read. [S]

## 3. Sources not consulted

- Adobe's XMP specification, for the XMP segment's identifier and
  packet; the identifier on these pages is as found in the files.
- The ICC profile specification, for the APP2 segments that carry colour
  profiles.
- The earlier editions of Exif (2.32, 3.0) and of the Multi-Picture
  Format (2009, 2021), and any amendment to T.81 after 1992. The pages
  cite the editions listed in
  [licences-and-references.md](licences-and-references.md).
- ISO/IEC 10918-1 itself, which is technically the same text as T.81 and
  was not bought.

## 4. Questions settled by the sample files

- **Sony's MakerNote in a JPEG.** It has the `SONY DSC` header, unlike
  the ARW files of the same bodies, and its offsets count from the TIFF
  header inside the Exif segment; read so, FocusLocation parses on every
  file that has it (the ILCE-7M3, ILCE-7RM2 and DSC-RX100M4 files). [F]
- **Nikon's MakerNote in a JPEG** is the embedded-TIFF layout of the NEF
  files, so its offsets need no change. [F]
- **A camera JPEG can hold a MakerNote preview**: the D70's PreviewIFD
  holds a 564×372 JPEG inside the declared MakerNote. [F]
- **The D70's IFD1 is also its Interoperability IFD**: both pointers lead
  to one IFD that holds the thumbnail's tags. It is not a loop for
  raw-files' walk, which follows no Interoperability pointer. [F]
- **The MP Entry's size** counts the SOI and EOI markers, on every file
  with MPF. [F]
- **The MPF images' frame headers** lie 648 to 1,160 bytes into each
  image, behind a small Exif segment of its own. [F]
- **The Software tag** does not prove a file is the camera's: files
  re-encoded by an editor kept it (see
  [test-data.md](test-data.md#3-how-the-files-were-chosen)). [S]
- **A file whose size is a multiple of 64 KiB** need not be truncated:
  Sony pads to that size. [F]
