# JPEG container and metadata

A JPEG file is one compressed picture wrapped in a series of segments that
say how it is coded and what it shows: when it was taken, by which camera,
and which smaller copies of it the file also holds. This page describes
those segments as far as raw-files reads them: the marker segments and the
frame header of the JPEG standard, the JFIF segment, the Exif segment with
the TIFF structure inside it and its thumbnail, and the Multi-Picture
Format segment that lists the larger previews. It ends with the way
raw-files reads a JPEG file and the traps on the way. Back to
[the JPEG overview](README.md).

## How to read this page

Claims carry a confidence tag:

- **[P]**: stated in a specification, cited by its section, table or
  figure, and page (the printed number).
- **[S]**: secondary: another program's documentation or output, a
  write-up, a search.
- **[I]**: inferred, or a recommendation. Treat it as a hypothesis.
- **[F]**: measured on a named sample file: one of the eight
  camera-original JPEGs from Wikimedia Commons, cited by body ("the
  ILCE-7M3 file"), which [test-data.md](test-data.md#2-the-sample-files)
  lists with its file name, licence and SHA-256.

Numbers in brackets point to the [Sources](#sources) at the end. The
specifications are the editions listed there: T.81 (1992), T.871 (2011),
Exif 3.1 (CIPA DC-008-2026) and the Multi-Picture Format of CIPA
DC-007-2026. Where a file departs from a specification, this page follows
the file and says so.

## Contents

- [1. Marker segments](#1-marker-segments)
- [2. The frame header](#2-the-frame-header)
- [3. JFIF: the APP0 segment](#3-jfif-the-app0-segment)
- [4. Exif: the APP1 segment](#4-exif-the-app1-segment)
- [5. Other application segments](#5-other-application-segments)
- [6. The thumbnail in IFD1](#6-the-thumbnail-in-ifd1)
- [7. The Multi-Picture Format](#7-the-multi-picture-format)
- [8. After the last image](#8-after-the-last-image)
- [9. Sizes and orientation](#9-sizes-and-orientation)
- [10. MakerNotes in a JPEG](#10-makernotes-in-a-jpeg)
- [11. The camera files, segment by segment](#11-the-camera-files-segment-by-segment)
- [12. Recommendation for raw-files](#12-recommendation-for-raw-files)
- [13. Pitfalls](#13-pitfalls)
- [Sources](#sources)

## 1. Marker segments

A JPEG stream is a sequence of markers. A marker is the byte `FF` followed
by a code that says what comes next. [P][1] (B.1.1.2, p. 31)

- **Fill bytes.** Any number of `FF` bytes may come before a marker; a
  reader skips them before deciding which marker it has. [P][1] (B.1.1.2,
  p. 31; E.1.2, p. 78; note to Table E.1, p. 82)
- **Markers that stand alone.** SOI (`FF D8`, start of image), EOI
  (`FF D9`, end of image), the eight restart markers RST0 to RST7 (`FF D0`
  to `FF D7`) and TEM (`FF 01`) have nothing after them. [P][1] (B.1.1.3
  and Table B.1, pp. 31 to 32)
- **Every other marker starts a segment**: after the marker comes a
  two-byte big-endian length, which counts itself and the segment's
  payload but not the two bytes of the marker. A length is therefore at
  least 2, and a segment holds at most 65,533 bytes after its length.
  [P][1] (B.1.1.4, p. 33; Table B.9, p. 44)
- **The order.** A file holds SOI, one frame and EOI. Before the frame
  header, and before each scan's header, may come any number of table and
  miscellaneous segments, in any order: quantisation tables (DQT,
  `FF DB`), Huffman tables (DHT, `FF C4`), arithmetic-coding conditions
  (DAC, `FF CC`), the restart interval (DRI, `FF DD`), comments (COM,
  `FF FE`) and application segments (APP0 to APP15, `FF E0` to `FF EF`).
  [P][1] (B.2.1 and Figure B.2, pp. 34 to 35; B.2.4 and Figure B.5,
  p. 39)
- **The scan.** A scan starts with its header (SOS, `FF DA`) and goes on
  with the entropy-coded data. Inside it, every `FF` byte the coder
  produced is followed by a stuffed `00`, so that it is not read as a
  marker, and restart markers may separate its parts. [P][1] (B.2.3,
  p. 37; B.1.1.5 note 2, p. 33; F.1.2.3, p. 91) A reader looking for
  segments stops at the first SOS: what follows is compressed data, not
  segments. [I]
- **Application segments** are left to applications: the standard gives
  their bytes no meaning, and a decoder reads them only so far as they do
  not interfere with decoding. [P][1] (B.2.4.6, p. 44; E.2.1, p. 83) The
  standard itself does not say a reader may skip them; Exif says a reader
  should skip the ones it does not know. [P][3] (4.5.4, p. 20; 4.7, p. 99)
- The other codes in Table B.1 are frame markers (section 2), DNL
  (`FF DC`), DHP (`FF DE`) and EXP (`FF DF`) for the hierarchical
  process, JPG (`FF C8`) and JPG0 to JPG13 (`FF F0` to `FF FD`) reserved
  for extensions, and RES (`FF 02` to `FF BF`) reserved. [P][1]
  (Table B.1, p. 32)

On the eight camera files the walk from SOI to SOS takes five to nine
segments, and the frame header lies 31,880 to 105,002 bytes into the file
(section 11). [F]

## 2. The frame header

The frame header gives the picture's size and how it is coded. After its
marker (SOFn) and its length come the sample precision (one byte), the
number of lines Y and the number of samples per line X (two bytes each,
big-endian), the number of components, and three bytes per component.
The length is therefore 8 plus three times the number of components.
[P][1] (B.2.2, Figure B.3, p. 35; Table B.2, p. 36)

- **The coding process is in the marker.** `FF C0` is baseline DCT;
  `C1`, `C2` and `C3` are extended, progressive and lossless sequential
  with Huffman coding; `C5` to `C7` their differential (hierarchical)
  forms; `C9` to `CB` and `CD` to `CF` the same processes with arithmetic
  coding. Table B.1 labels `C3` and `CB` "lossless (sequential)" and `C7`
  and `CF` "differential lossless (sequential)". [P][1] (Table B.1, p. 32)
- **A lossless frame holds sample values exactly**, which is how some raw
  files store their sensor data: Sony stores the raw data of its lossless
  ARW files as lossless JPEG tiles, each starting `FF D8 FF C3` (see the
  [ARW raw-data notes](../arw/raw-data.md#lossless-compressed-compression-7)).
  A lossless stream is not a picture to show. [I]
- **A height of 0** means that the number of lines is given later, by a
  DNL segment at the end of the first scan; a reader that stops at SOS
  cannot learn it. [P][1] (p. 36; B.2.5, p. 45)
- Every frame on the eight camera files, the main image's, the
  thumbnails' and the MPF images', is baseline (`FF C0`). [F]

## 3. JFIF: the APP0 segment

JFIF is a profile of the JPEG standard for exchanging files. A JFIF file
has an APP0 segment immediately after SOI, whose payload starts with the
five bytes `JFIF` and a zero, then a version (`01 02`), the units and the
horizontal and vertical pixel density, and the size of an optional
uncompressed thumbnail followed by its pixels. [P][2] (6.1, p. 2; 6.3,
p. 3; 10.1, p. 6) Extension segments (JFXX, also APP0) may follow it,
and other application segments come after those. [P][2] (6.4, 6.5, p. 3;
10.2, p. 7)

- **JFIF and Exif disagree about the first segment.** Each puts its own
  segment immediately after SOI, and neither mentions the other: T.871 says
  nothing of Exif, APP1 or TIFF, and Exif 3.1 says nothing of JFIF or APP0,
  anywhere in their text. [P][2][3] A file that follows both has its Exif
  segment second. [I]
- **None of the eight camera files has a JFIF segment**: each starts with
  its Exif segment. [F] Editors that re-encode a file often write one;
  in the search that chose the eight files, files whose first segment was
  JFIF were the re-encoded ones, and they were left out (see
  [test-data.md](test-data.md#3-how-the-files-were-chosen)). [S]
- raw-files does not read the JFIF segment: the frame header gives the
  size, and its thumbnail is uncompressed. [I]

## 4. Exif: the APP1 segment

Exif records the camera's metadata in an APP1 segment. Its payload starts
with the four bytes `Exif` and two zero bytes (Exif names the first
"NULL" and the second "padding"), and a TIFF header follows at once: a
byte order mark (`II` or `MM`), the number 42 in that order, and the
offset of the first IFD. [P][3] (4.7.2.3 and Figure 36, p. 105;
4.7.2.5, p. 106; Table 1, p. 20)

- **Offsets count from the TIFF header**, from the first byte of its
  byte order mark, not from the start of the file or of the segment.
  [P][3] (4.6.2, p. 28; 4.7.2.5, p. 106; 4.6.3.1 and 4.6.3.2, pp. 28 to
  29) The structure is the one a NEF or an ARW file is: IFD0 (Exif's 0th
  IFD), its next IFD, IFD1 (Exif's 1st IFD), and the Exif, GPS and
  Interoperability IFDs that IFD0 and the Exif IFD point to. [P][3]
  (4.5.2, p. 19; 4.6.3.1 and 4.6.3.2, pp. 28 to 29)
- **The segment comes immediately after SOI**, and APP2 segments may
  follow it. [P][3] (4.5.4, p. 20; Table 2, p. 22; 4.7.2.1, p. 105)
- **It holds at most 64 KB**, and the thumbnail must fit in it together
  with everything else. [P][3] (p. 21; 4.7.2, p. 105; 4.6.5.2.5, p. 40)
  The bound in bytes comes from the JPEG standard: a length of at most
  65,535, so at most 65,533 bytes after the length, 6 of them the
  identifier and at most 65,527 the TIFF structure. [P][1] (Table B.9,
  p. 44) The Exif standard does not repeat the number.
- **On the eight camera files** the Exif segment is the first segment,
  so its TIFF header lies at byte 12 of the file (two bytes of SOI, two
  of marker, two of length and six of identifier). Its length runs from
  27,646 (G7 X) to 65,534 (D5300, one byte short of the maximum). The
  A100's TIFF structure is big-endian (`MM`), the other seven
  little-endian. [F]
- **Everything the standard tags need lies inside the segment** on all
  eight: every IFD, every value raw-files reads, the thumbnail and the
  MakerNote. [F] One MakerNote entry points outside it (section 10).
- **One Exif segment per file**: no file has a second. [F] Exif describes
  one APP1 segment [P][3] (4.5.4, p. 20); what a reader does with a second
  is not said. [I]

## 5. Other application segments

- **XMP** is another APP1 segment, whose payload starts with
  `http://ns.adobe.com/xap/1.0/` and a zero byte. [F] (Its specification,
  Adobe's XMP Part 3, was not read for these notes. [I]) Three of the
  camera files write one, holding only a rating of 0: the ILCE-7M3
  (57,346 bytes, nearly all padding), the D5300 (2,046 bytes) and the
  G7 X (3,996 bytes). [F] raw-files does not read XMP (see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
- **APP2** segments of two kinds may follow the Exif segment: FlashPix
  extension data (`FPXR`) and the Multi-Picture Format (`MPF`), section 7.
  [P][3] (4.7.3, pp. 106 to 107; 4.7.4, p. 110) Exif also places JUMBF
  data in APP11 (4.7.5), and lets makers add segments of their own (4.5.5
  and 4.5.6, pp. 22 to 25). [P][3] Colour profiles (ICC) are also carried
  in APP2 segments; that specification was not read. [I]
- Exif asks that all quantisation tables be in one DQT segment and all
  Huffman tables in one DHT segment; the D70 file writes two DQT and four
  DHT segments. [P][3] (4.7.1.2 and 4.7.1.3, pp. 100 to 101) [F] Nothing
  raw-files does depends on it.

## 6. The thumbnail in IFD1

IFD1, the IFD that IFD0's next-IFD pointer leads to, holds the thumbnail.
[P][3] (4.5.2, p. 19; 4.5.8, pp. 25 to 26; 4.7.2.4, p. 106)

- **A JPEG thumbnail** has Compression 6. JPEGInterchangeFormat (0x0201)
  gives the offset of its SOI, from the TIFF header, and
  JPEGInterchangeFormatLength (0x0202) its length in bytes; both are
  required for a compressed thumbnail. It is one stream from SOI to EOI,
  with no application segments, comments or restart markers, stored inside
  the Exif segment. [P][3] (4.5.8, pp. 25 to 26; 4.6.5.1.4, p. 33;
  4.6.5.2.4 and 4.6.5.2.5, p. 40; Table 21, pp. 98 to 99; Figure 7, p. 21)
- **No size is set for it**: Exif 3.1 places no limit on its size, and it
  is optional. [P][3] (4.5.8, pp. 25 to 26) The 160×120 pixels often quoted
  is not in that edition.
- **On the camera files** every thumbnail is a JPEG of 160×120 pixels but
  the D70's, of 128×85, from 4,129 to 10,107 bytes, inside the segment
  and ending with EOI. [F]
- **The D70 file shares one IFD between two roles**: IFD0's next-IFD
  pointer and the Exif IFD's Interoperability pointer (0xA005) both lead
  to offset 29,110, an IFD whose seven entries are the thumbnail's.
  [F] (A reader that runs ExifTool on it is told that the IFD1 pointer
  refers to the Interoperability IFD. [S]) A reader that visits each IFD
  once and follows the Interoperability pointer first could take IFD1 for
  a loop and lose the thumbnail; raw-files does not follow the
  Interoperability pointer. [I]

## 7. The Multi-Picture Format

The Multi-Picture Format lets one file hold several images: the main
image, then larger thumbnails for screens and other images, each a whole
JPEG stream from SOI to EOI. An APP2 segment in the first image lists
them. [P][4] (5.1 and Figure 1, pp. 7 to 8)

### 7.1 The segment and its index

- **The segment.** Its payload starts with `MPF` and a zero byte, then an
  MP header in the shape of a TIFF header: a byte order mark, 42, and the
  offset of the first IFD, normally 8. There is one such segment per
  image, and it holds at most 64 KB. [P][4] (5.2.1 and Figure 4, pp. 10
  to 11; 5.2.2.1 and Figure 5, 5.2.2.2, p. 11; Figure 2, p. 9)
- **Every offset counts from the MP header**, from the first byte of its
  byte order mark. [P][4] (5.2, p. 10) On the camera files the MP header
  lies 8 bytes into the segment (marker, length, identifier). [F]
- **The MP Index IFD** holds MPFVersion (0xB000, four bytes, `0100`),
  NumberOfImages (0xB001, a LONG) and MPEntry (0xB002, 16 bytes per
  image), which are required, and ImageUIDList (0xB003) and TotalFrames
  (0xB004), which are not. Its tags are in ascending order, and it is in
  the first image only. [P][4] (Table 3, p. 12; 5.2.3, pp. 12 to 18;
  Table 6, p. 27; 6.2.1.2)
- **An MP Entry** is 16 bytes: the image's attribute (4 bytes), its size
  (4), its offset (4), and the entry numbers of two images that depend on
  it (2 and 2). The attribute holds, from the top bit down, three flags
  (dependent parent, dependent child, representative image), two
  reserved bits, the image data format in three bits (0 is JPEG), and the
  MP type in the low 24 bits. [P][4] (Figure 7, p. 13; Figure 8, p. 14;
  5.2.3.3.1, p. 14)
- **The size** is that of the image data "between the SOI and EOI
  markers"; whether the markers count is not said. [P][4] (5.2.3.3.2,
  p. 16) On the camera files the first image's size ends exactly after
  its EOI: the markers count. [F]
- **The offset** is that of the image's SOI, from the MP header, and is
  0 for the first image. [P][4] (5.2.3.3.3, p. 16)
- **The MP types** include 0x030000, the baseline primary image;
  0x010001 to 0x010005, large thumbnails of classes 1 to 5; 0x020001 to
  0x020003, the panorama, disparity and multi-angle images of a
  multi-frame file; 0x040000, an original kept for preservation;
  0x050000, a gain map; and 0x000000, undefined. [P][4] (Table 4, p. 15;
  Figure 9, p. 16)

### 7.2 Large thumbnails

A large thumbnail of class 1 has at most 640×480 pixels, of class 2
1920×1080, then 3840×2160, 7680×4320 and 15360×8640; one side equals the
class's maximum, the aspect ratio is the main image's, and a file holds at
most one per class. Large thumbnails carry no MPF segment of their own,
and their Exif segment is left out when they have no tag values of their
own. [P][4] (6.1.2.2 and Table 9, pp. 30 to 31; 6.1, p. 29; 6.1.2.1,
p. 30; clause 7, p. 36)

On the camera files [F]:

| File | MPF images after the first | Where they start |
|---|---|---|
| ILCE-7M3, ILCE-7RM2, DSC-RX100M4, ILCE-6000 | one of class 2, 1616×1080 | after zeros, at a multiple of 512 bytes |
| Canon G7 X | one of class 2, 1620×1080 | after 12 bytes that are not zeros, at a multiple of 512 bytes |
| Nikon D5300 | one of class 1, 640×424, and one of class 2, 1620×1080 | after zeros, at a multiple of 512 bytes |
| Nikon D70, Sony DSLR-A100 | no MPF segment | |

- Sony's 1616×1080 does not quite keep the 3:2 aspect ratio of the main
  image, which would give 1620×1080. [F]
- Each of these images starts with a small Exif segment of its own (160
  bytes on the Sony files, 510 on the G7 X, 1,022 on the D5300) and has
  no MPF segment; its frame header lies 648 to 1,160 bytes into it. [F]
- The first entry's attribute is 0xA0 in its top byte (a parent of
  dependent images, and the representative image) on the Sony and D5300
  files, and 0x80 (a parent, but not marked representative) on the G7 X;
  the thumbnails' is 0x40 (a dependent child). [F]
- On the Sony files the MPF segment ends where the next segment starts at
  a multiple of 512 bytes from the start of the file. [F]
- The specification does not say that the images follow the first one's
  end, or in which order: it shows them one after another, and leaves an
  extended MP file's order free. [P][4] (Figure 1, p. 8; 6.2, p. 32) On
  every camera file they follow the first image, in the order of their
  entries. [F]

## 8. After the last image

Exif sets nothing about bytes after EOI, and asks readers to carry on
past data they do not know after the main image's EOI. [P][3] (4.7,
p. 99; 4.7.1.8, p. 104; 4.5.7, p. 25)

- **Sony pads its files with zeros** after the last image's EOI, up to a
  multiple of 64 KiB: 28,938 bytes on the ILCE-7M3 file, 31,620 on the
  ILCE-7RM2 and 28,400 on the ILCE-6000. The DSC-RX100M4 and DSLR-A100
  files end at the EOI, as do the Nikon and Canon files. [F] A file whose
  size is a multiple of 64 KiB is therefore not a sign of truncation. [I]
- The file's own preview, as raw-files reports it, is the whole file, so
  its bytes may end with MPF images and zeros after the main image's EOI;
  a decoder stops at that EOI. [I] (see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md))

## 9. Sizes and orientation

- **PixelXDimension (0xA002) and PixelYDimension (0xA003)** give the
  width and height of the meaningful image, without padding; they are
  required in a compressed file. [P][3] (4.6.6.3.1 and 4.6.6.3.2, p. 48;
  Table 18, p. 95; 4.8.1, pp. 115 to 119) On all eight camera files they
  equal the main frame header's size. [F] raw-files takes the image
  size from the frame header, the picture itself, and does not read them (see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)).
- **Orientation (0x0112)** is a SHORT, 1 by default, whose values 1 to 8
  say where the stored image's first row and first column belong when it is
  shown. It is recommended in IFD0 and optional in IFD1. [P][3] (4.6.5.1.6,
  p. 33; Table 17, p. 96; Table 21, pp. 98 to 99) Every camera file has
  orientation 1 in IFD0. [F] Whether a camera stores its thumbnail and its
  MPF images turned when it records a portrait picture is not known from
  these files; see [open-questions.md](open-questions.md).
- **Nothing in a JPEG gives a raw size**: it holds no raw data. [I]

## 10. MakerNotes in a JPEG

The MakerNote (0x927C, UNDEFINED, in the Exif IFD) holds whatever the
maker records, in a layout the standard leaves to the maker. [P][3]
(4.6.6.4.1, p. 49; Table 18, p. 95) In a JPEG, the MakerNotes are those
of the same makers' raw files, read with the TIFF header inside the Exif
segment wherever the raw file counts from the start of the file:

- **Nikon** (the D5300 and D70 files): the layout with its own embedded
  TIFF header that NEF files use, whose offsets count from that inner
  header, so it reads the same in a JPEG. [F] The D70's PreviewIFD holds
  a 564×372 JPEG inside the MakerNote's declared length. [F] See the
  [NEF MakerNote notes](../nef/makernote-and-metadata.md#1-finding-the-makernote).
- **Sony** (all five Sony files): the 12-byte header `SONY DSC ` and three
  zero bytes, then an IFD whose offsets count from the TIFF header inside
  the Exif segment, as an ARW's count from the start of the file. [F] The
  ILCE-6000's MakerNote tag 0x2001 declares 549,518 bytes at an offset
  that lies inside its MPF image, outside the Exif segment. [F] See the
  [ARW MakerNote notes](../arw/makernote-and-metadata.md#1-finding-the-makernote).
- **Canon** (the G7 X file) and other makers: no reader. [I]

What each maker's MakerNote gives in a JPEG, the focus point among it, is
in those pages.

## 11. The camera files, segment by segment

Each segment as marker and length, in file order up to SOS. [F]

| File | Segments | Frame header at | Bytes |
|---|---|---|---|
| Sony ILCE-7M3 | APP1 Exif 46,742; APP1 XMP 57,346; APP2 MPF 352; DQT; DHT; SOF0 | 105,002 | 8,323,072 |
| Sony ILCE-7RM2 | APP1 Exif 48,580; APP2 MPF 566; DQT; DHT; SOF0 | 49,706 | 13,762,560 |
| Sony DSC-RX100M4 | APP1 Exif 44,749; APP2 MPF 301; DQT; DHT; SOF0 | 45,610 | 8,488,968 |
| Sony ILCE-6000 | APP1 Exif 44,334; APP2 MPF 204; DQT; DHT; SOF0 | 45,098 | 7,798,784 |
| Sony DSLR-A100 | APP1 Exif 53,681; DQT; DHT; SOF0 | 54,239 | 4,002,446 |
| Nikon D5300 | APP1 Exif 65,534; APP1 XMP 2,046; APP2 MPF 4,094; DQT; SOF0; DHT | 71,816 | 7,822,985 |
| Nikon D70 | APP1 Exif 33,778; DQT ×2; SOF0; DHT ×4 | 33,920 | 4,178,961 |
| Canon G7 X | APP1 Exif 27,646; APP1 XMP 3,996; APP2 MPF 96; DQT; SOF0; DHT | 31,880 | 8,667,055 |

On these files, reading the segments up to SOS, the Exif and MPF segments,
and each MPF image up to its frame header takes one to three blocks of
64 KiB. [F]

## 12. Recommendation for raw-files

What [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)
decided, as steps. [I]

1. **Recognise the container.** A file whose first bytes are `FF D8 FF`
   is a JPEG.
2. **Walk the segments** from SOI, one step per fill byte or marker:
   skip fill bytes; take a stand-alone marker as it is; for any other
   marker read the length. Keep the first frame header, the first APP1
   segment whose payload starts `Exif\0\0` and the first APP2 segment
   whose payload starts `MPF\0`. Stop at the first SOS. Stop as well,
   keeping what was found, at a length below 2, a segment that runs past
   the end of the file, an EOI, or the step cap.
3. **Accept the file** when the walk found a frame header with neither side
   zero that is not lossless (`C3`, `C7`, `CB`, `CF`); otherwise refuse it
   with the `UnknownFormat` error.
4. **Read the Exif segment as a bounded block.** The TIFF header lies 6
   bytes into the payload, and its position is the base of every offset.
   Every IFD, value, thumbnail and MakerNote must lie inside the segment;
   one outside is dropped as damage. A segment whose TIFF header or IFD0
   cannot be read gives none of the values the segment records, no
   thumbnail and no MakerNote preview; the file still opens, with its own
   preview, its MPF images and its image size, which the frame header
   gives, not the segment.
5. **The previews** are the file itself (its whole byte range, with the
   main frame header's size); IFD1's JPEG thumbnail; each MPF entry after
   the first, at the MP header's position plus its offset, inside the file;
   and on a Nikon file the MakerNote PreviewIFD's JPEG inside the
   MakerNote's declared length. Each must start with SOI and hold,
   within its own bytes, a frame header that has no zero side and is not
   lossless; each takes IFD0's orientation.
6. **The metadata** comes from the standard tags, then the MakerNote by
   IFD0's Make, as for a NEF or an ARW; the image size is the main frame
   header's, and there is no raw size.
7. **The focus point** is kept only when its frame has the JPEG's aspect
   ratio, to within one pixel of rounding in either size:
   `|fw × jh − fh × jw|` at most the longest of `fw`, `fh`, `jw` and
   `jh`, where `fw × fh` is the frame and `jw × jh` the JPEG, in stored
   orientation. Before that, a Nikon JPEG shot in a crop mode, whose
   MakerNote records a crop and holds no CropArea, has its point placed
   on the JPEG's own picture: a DX crop's scaled onto the JPEG's frame,
   a 1:1 or 16:9 crop's moved into the centred window of the recorded
   frame that has the JPEG's shape
   ([ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)).

## 13. Pitfalls

- **A length below 2** would make a reader step backwards or stay in
  place; it ends the walk, as a segment that runs past the end of the
  file does. [I]
- **`FF` bytes before a marker are legal**, in any number, but each is a
  step of the walk: a file with more of them than the step cap allows is
  refused. None of the eight camera files has one before its scan. [P][1]
  (B.1.1.2, p. 31) [F] [I]
- **A frame header after SOS** cannot be found by walking segments; one
  that gives a height of 0 has its height after the first scan. [P][1]
  (p. 36) [I]
- **Offsets in the Exif segment are not file offsets.** They count from
  the TIFF header, 12 bytes into a file whose Exif segment comes first,
  later behind a JFIF segment. MPF offsets count from the MP header, in
  another segment, and Sony's MakerNote offsets from the Exif segment's
  TIFF header. [P][3][4] [F]
- **A pointer that leaves the segment** (the ILCE-6000's tag 0x2001) is
  not a standard value; it is dropped, not followed. [F] [I]
- **An MPF image can start after bytes that are not zeros** (the G7 X)
  and after zeros elsewhere; follow its offset, never search for the next
  SOI. [F] [I]
- **The MP Entry's size of the first image** is not the file's size: MPF
  images and padding follow. [F]
- **The Software tag does not prove a file is the camera's**: a program
  that re-encodes the picture may keep the camera's Software value. The
  segment layout, the MakerNote and the native size do. [S] (see
  [test-data.md](test-data.md#3-how-the-files-were-chosen))
- **An export may keep the camera's MakerNote** with a focus frame that no
  longer matches its pixels; see
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md). [I]

## Sources

[1] ITU-T Recommendation T.81 (09/92) | ISO/IEC 10918-1:1993, "Digital
    compression and coding of continuous-tone still images: Requirements
    and guidelines". Read in the CCITT publication hosted by the W3C,
    https://www.w3.org/Graphics/JPEG/itu-t81.pdf; the ITU's page,
    https://www.itu.int/rec/T-REC-T.81-199209-I/en, offers it for a fee.
    **[P]**
[2] ITU-T Recommendation T.871 (05/2011) | ISO/IEC 10918-5:2012, "JPEG
    File Interchange Format (JFIF)",
    https://www.itu.int/rec/T-REC-T.871-201105-I/en (freely available).
    **[P]**
[3] CIPA DC-008-Translation-2026, "Exchangeable image file format for
    digital still cameras: Exif Version 3.1" (January 2026),
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-008-2026-E.
    **[P]**
[4] CIPA DC-007-Translation-2026, "Multi-Picture Format",
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-007-2026_E.
    **[P]**
[5] The eight camera-original JPEGs from Wikimedia Commons listed in
    [test-data.md](test-data.md#2-the-sample-files), walked byte by byte
    with a script of the maintainer's own in October 2026. **[F]**

All pages were retrieved in October 2026.
