# Tag names

Every entry of a TIFF directory is a tag number, a type, a count and a
value. raw-files will give every entry it walks, as the file records it
([ADR023](../architecture-decisions/adr023-every-entry-a-file-records.md)),
and a name for an entry's tag where a published standard gives that tag
a name in that kind of directory. This page records every such name,
with the document and the place in it that gives the name, and the
field types the entries can have. The table of names in the code will
be written from this page and from nothing else
([ADR013](../architecture-decisions/adr013-clean-room-and-third-party-code.md)).

Where this page says *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files, writing about files from my own cameras: they stay
private, and are cited by counts only.

The page serves every format raw-files reads: NEF, ARW and camera JPEG
files hold the same kinds of directory, and a tag's name depends on the
kind of directory it is in, not on the format.

## Contents

- [1. How the names were gathered](#1-how-the-names-were-gathered)
- [2. Field types](#2-field-types)
- [3. The IFD chain and the SubIFDs](#3-the-ifd-chain-and-the-subifds)
- [4. The Exif IFD](#4-the-exif-ifd)
- [5. The GPS IFD](#5-the-gps-ifd)
- [6. The Interoperability IFD](#6-the-interoperability-ifd)
- [7. The Multi-Picture index](#7-the-multi-picture-index)
- [8. MakerNotes and private directories](#8-makernotes-and-private-directories)
- [9. Tags no document names](#9-tags-no-document-names)
- [Sources](#sources)

## 1. How the names were gathered

**The documents.** Six published documents, read on 9 November 2026
([Sources](#sources)): TIFF Revision 6.0 [1]; Exif 3.1 (CIPA
DC-008-2026) [2]; DCF 2.0, edition 2010 (CIPA DC-009-2010) [3], for the
Interoperability IFD; the Multi-Picture Format (CIPA DC-007-2026) [4];
the DNG specification 1.7.1.0 [5], for the DNG tags raw-files reads;
and XMP Specification Part 3 [6], for the XMP tag. Each name below is
the field name the document gives, spelled as it spells it, and each
row says where: a table or section, and the printed page number.

**How they were taken out.** The documents' text was extracted from
their PDFs, and the names read from their tag tables by a script of my
own: TIFF 6.0's Appendix A, "TIFF Tags Sorted by Number"; Exif's
tables 6, 8, 9, 14 and 16, with each tag's own section; DCF's section
4.4.5.3; the Multi-Picture Format's Table 3; and the DNG
specification's chapter 4, "DNG Tags". Every row matches a tag number
given both in decimal and in hexadecimal in the document. No other
reader's tag list was read or run to make this page.

**Where a document gives two names.** Only one tag: TIFF 6.0's
Appendix A writes 0x0202 as `JPEGInterchangeFormatLngth`, a spelling
its own Section 22 does not use (`JPEGInterchangeFormatLength`, p. 105),
and Exif writes it in full too, so the full name is taken. Exif's
section headings sometimes spell a field differently from its tables
(`SubsecTime`, "ISO Speed"); the tables' field names are taken.

**Which tags files hold.** Every tag seen in the standard directories
of NEF, ARW and camera JPEG files was counted on 9 November 2026 with a
walker of my own, written from these standards and the format notes:
on the test fixtures and every NEF and ARW-family file of
[raw.pixls.us](https://raw.pixls.us/) (383 NEF, 245 ARW-family files
and 14 camera JPEGs, "public" below), and on a sample of my own files,
at most 100 per folder (6,315 NEF, 297 ARW and 2,407 JPEG files from
Nikon, Sony, Canon and Fujifilm bodies, "mine" below). The "Seen in"
column says in which formats' files a tag was found; 194 different tags
were found, 140 of them named below, and the 54 others are listed in
[section 9](#9-tags-no-document-names).

## 2. Field types

An entry's type says how its value is stored. TIFF 6.0 defines twelve
(Section 2, "TIFF Structure", pp. 15 to 16) [1]:

| Code | Name | One value |
|---|---|---|
| 1 | BYTE | an 8-bit unsigned integer |
| 2 | ASCII | an 8-bit byte holding a 7-bit ASCII code; the last byte must be NUL |
| 3 | SHORT | a 16-bit unsigned integer |
| 4 | LONG | a 32-bit unsigned integer |
| 5 | RATIONAL | two LONGs, numerator then denominator |
| 6 | SBYTE | an 8-bit signed integer |
| 7 | UNDEFINED | an 8-bit byte that may hold anything, by the field's definition |
| 8 | SSHORT | a 16-bit signed integer |
| 9 | SLONG | a 32-bit signed integer |
| 10 | SRATIONAL | two SLONGs, numerator then denominator |
| 11 | FLOAT | a 4-byte IEEE floating-point number |
| 12 | DOUBLE | an 8-byte IEEE floating-point number |

Exif uses eight of them (1 to 5, 7, 9 and 10) and adds one of its own
(section 4.6.2, "IFD Structure", pp. 27 to 28) [2]:

| Code | Name | One value |
|---|---|---|
| 129 | UTF-8 | an 8-bit byte of a string in UTF-8; the last byte is NUL, no byte order mark is used, and the count includes the NUL |

- **UTF-8 came with Exif 3.0**, in 2023: its revision history says it
  "added Tag Type of UTF-8 as Exif specific tag type" and "enabled to
  select UTF-8 character string in existing ASCII-type tags" [2] (p. 3).
  It is "defined independently by this standard, rather than in
  TIFF6.0". Exif's tables give sixteen tags as "ASCII or UTF-8":
  ImageDescription, Make, Model, Software, Artist and Copyright in IFD0
  (Table 6, pp. 31 to 32), and CameraOwnerName, LensMake, LensModel,
  ImageTitle, Photographer, ImageEditor, CameraFirmware,
  RAWDevelopingSoftware, ImageEditingSoftware and
  MetadataEditingSoftware in the Exif IFD (Table 8, p. 45); and one as
  UTF-8 alone, DevelopmentTypeDescription (Table 9, p. 46). Every other
  text tag stays ASCII, the dates, ImageUniqueID, BodySerialNumber and
  LensSerialNumber among them.
- **The count of a UTF-8 value is its bytes**, NUL included, not its
  characters, since a character takes one to four bytes (section 4.6.2,
  p. 28).
- **No file writes it.** None of the 18,464 files walked for the raw
  view on 3 November 2026 has an entry of type 129.
- **Type 13, an IFD, is in neither document.** raw-files already
  accepts it for the SubIFDs tag, and the view will read it as a type
  of its own
  ([ADR023](../architecture-decisions/adr023-every-entry-a-file-records.md));
  but no document read defines it, so this page gives it no name.
  None of those 18,464 files has an entry of type 13.

## 3. The IFD chain and the SubIFDs

IFD0, the IFDs chained after it (IFD1, IFD2) and the SubIFDs that IFD0
points to share one set of names: TIFF 6.0's, with Exif's for the tags
it uses in IFD0 and IFD1 (Table 6, pp. 31 to 32) and for the two
pointers it adds there (section 4.6.3) [1][2], and the DNG
specification's for the four DNG tags raw-files reads [5]. A JPEG's
Exif segment holds the same IFD0 and IFD1. Exif's own tags have these
names only in the Exif IFD: a camera that writes one of them in IFD0,
as a NEF does DateTimeOriginal, gets no name there
([section 9](#9-tags-no-document-names)).

| Tag | Name | Document, where | Seen in |
|---|---|---|---|
| 0x00FE | NewSubfileType | TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x00FF | SubfileType | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0100 | ImageWidth | Exif 3.1, Table 6, p. 31; 4.6.5.1.1, p. 32; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0101 | ImageLength | Exif 3.1, Table 6, p. 31; 4.6.5.1.2, p. 32; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0102 | BitsPerSample | Exif 3.1, Table 6, p. 31; 4.6.5.1.3, p. 32; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0103 | Compression | Exif 3.1, Table 6, p. 31; 4.6.5.1.4, p. 33; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0106 | PhotometricInterpretation | Exif 3.1, Table 6, p. 31; 4.6.5.1.5, p. 33; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0107 | Threshholding | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0108 | CellWidth | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0109 | CellLength | TIFF 6.0, Appendix A, p. 117 | – |
| 0x010A | FillOrder | TIFF 6.0, Appendix A, p. 117 | – |
| 0x010D | DocumentName | TIFF 6.0, Appendix A, p. 117 | – |
| 0x010E | ImageDescription | Exif 3.1, Table 6, p. 32; 4.6.5.4.1, p. 41; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x010F | Make | Exif 3.1, Table 6, p. 32; 4.6.5.4.2, p. 42; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0110 | Model | Exif 3.1, Table 6, p. 32; 4.6.5.4.3, p. 42; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0111 | StripOffsets | Exif 3.1, Table 6, p. 31; 4.6.5.2.1, p. 39; TIFF 6.0, Appendix A, p. 117 | ARW, NEF |
| 0x0112 | Orientation | Exif 3.1, Table 6, p. 31; 4.6.5.1.6, p. 33; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0115 | SamplesPerPixel | Exif 3.1, Table 6, p. 31; 4.6.5.1.7, p. 37; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x0116 | RowsPerStrip | Exif 3.1, Table 6, p. 31; 4.6.5.2.2, p. 40; TIFF 6.0, Appendix A, p. 117 | ARW, NEF |
| 0x0117 | StripByteCounts | Exif 3.1, Table 6, p. 31; 4.6.5.2.3, p. 40; TIFF 6.0, Appendix A, p. 117 | ARW, NEF |
| 0x0118 | MinSampleValue | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0119 | MaxSampleValue | TIFF 6.0, Appendix A, p. 117 | – |
| 0x011A | XResolution | Exif 3.1, Table 6, p. 31; 4.6.5.1.8, p. 37; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x011B | YResolution | Exif 3.1, Table 6, p. 31; 4.6.5.1.9, p. 37; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x011C | PlanarConfiguration | Exif 3.1, Table 6, p. 31; 4.6.5.1.10, p. 38; TIFF 6.0, Appendix A, p. 117 | ARW, JPEG, NEF |
| 0x011D | PageName | TIFF 6.0, Appendix A, p. 117 | – |
| 0x011E | XPosition | TIFF 6.0, Appendix A, p. 117 | – |
| 0x011F | YPosition | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0120 | FreeOffsets | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0121 | FreeByteCounts | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0122 | GrayResponseUnit | TIFF 6.0, Appendix A, p. 117 | – |
| 0x0123 | GrayResponseCurve | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0124 | T4Options | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0125 | T6Options | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0128 | ResolutionUnit | Exif 3.1, Table 6, p. 31; 4.6.5.1.11, p. 38; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x0129 | PageNumber | TIFF 6.0, Appendix A, p. 118 | – |
| 0x012D | TransferFunction | Exif 3.1, Table 6, p. 31; 4.6.5.3.1, p. 40; TIFF 6.0, Appendix A, p. 118 | – |
| 0x0131 | Software | Exif 3.1, Table 6, p. 32; 4.6.5.4.4, p. 42; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x0132 | DateTime | Exif 3.1, Table 6, p. 32; 4.6.5.4.5, p. 43; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x013B | Artist | Exif 3.1, Table 6, p. 32; 4.6.5.4.6, p. 43; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x013C | HostComputer | TIFF 6.0, Appendix A, p. 118 | JPEG |
| 0x013D | Predictor | TIFF 6.0, Appendix A, p. 118 | – |
| 0x013E | WhitePoint | Exif 3.1, Table 6, p. 31; 4.6.5.3.2, p. 41; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG |
| 0x013F | PrimaryChromaticities | Exif 3.1, Table 6, p. 31; 4.6.5.3.3, p. 41; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG |
| 0x0140 | ColorMap | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0141 | HalftoneHints | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0142 | TileWidth | TIFF 6.0, Appendix A, p. 118 | ARW |
| 0x0143 | TileLength | TIFF 6.0, Appendix A, p. 118 | ARW |
| 0x0144 | TileOffsets | TIFF 6.0, Appendix A, p. 118 | ARW |
| 0x0145 | TileByteCounts | TIFF 6.0, Appendix A, p. 118 | ARW |
| 0x014C | InkSet | TIFF 6.0, Appendix A, p. 118 | – |
| 0x014D | InkNames | TIFF 6.0, Appendix A, p. 118 | – |
| 0x014E | NumberOfInks | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0150 | DotRange | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0151 | TargetPrinter | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0152 | ExtraSamples | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0153 | SampleFormat | TIFF 6.0, Appendix A, p. 118 | ARW |
| 0x0154 | SMinSampleValue | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0155 | SMaxSampleValue | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0156 | TransferRange | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0200 | JPEGProc | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0201 | JPEGInterchangeFormat | Exif 3.1, Table 6, p. 31; 4.6.5.2.4, p. 40; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x0202 | JPEGInterchangeFormatLength | Exif 3.1, Table 6, p. 31; 4.6.5.2.5, p. 40; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x0203 | JPEGRestartInterval | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0205 | JPEGLosslessPredictors | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0206 | JPEGPointTransforms | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0207 | JPEGQTables | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0208 | JPEGDCTables | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0209 | JPEGACTables | TIFF 6.0, Appendix A, p. 118 | – |
| 0x0211 | YCbCrCoefficients | Exif 3.1, Table 6, p. 31; 4.6.5.3.4, p. 41; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG |
| 0x0212 | YCbCrSubSampling | Exif 3.1, Table 6, p. 31; 4.6.5.1.12, p. 38; TIFF 6.0, Appendix A, p. 118 | ARW |
| 0x0213 | YCbCrPositioning | Exif 3.1, Table 6, p. 31; 4.6.5.1.13, p. 38; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x0214 | ReferenceBlackWhite | Exif 3.1, Table 6, p. 32; 4.6.5.3.5, p. 41; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x8298 | Copyright | Exif 3.1, Table 6, p. 32; 4.6.5.4.7, p. 43; TIFF 6.0, Appendix A, p. 118 | ARW, JPEG, NEF |
| 0x8769 | Exif IFD Pointer | Exif 3.1, 4.6.3.1.1, p. 28 | ARW, JPEG, NEF |
| 0x8825 | GPS Info IFD Pointer | Exif 3.1, 4.6.3.2.1, p. 29 | ARW, JPEG, NEF |
| 0xC612 | DNGVersion | DNG 1.7.1.0, Chapter 4, DNG Tags, p. 24 | – |
| 0xC61F | DefaultCropOrigin | DNG 1.7.1.0, Chapter 4, DNG Tags, p. 31 | ARW |
| 0xC620 | DefaultCropSize | DNG 1.7.1.0, Chapter 4, DNG Tags, p. 31 | ARW |
| 0xC634 | DNGPrivateData | DNG 1.7.1.0, Chapter 4, DNG Tags, p. 41 | ARW |

80 names.


## 4. The Exif IFD

The Exif IFD, which IFD0's tag 0x8769 points to, holds Exif's own tags:
tables 8 and 9 (pp. 44 to 47) and the pointer to the Interoperability
IFD (section 4.6.3.3.1) [2].

| Tag | Name | Document, where | Seen in |
|---|---|---|---|
| 0x829A | ExposureTime | Exif 3.1, Table 9, p. 45; 4.6.6.7.1, p. 55 | ARW, JPEG, NEF |
| 0x829D | FNumber | Exif 3.1, Table 9, p. 45; 4.6.6.7.2, p. 55 | ARW, JPEG, NEF |
| 0x8822 | ExposureProgram | Exif 3.1, Table 9, p. 45; 4.6.6.7.3, p. 55 | ARW, JPEG, NEF |
| 0x8824 | SpectralSensitivity | Exif 3.1, Table 9, p. 45; 4.6.6.7.4, p. 56 | – |
| 0x8827 | PhotographicSensitivity | Exif 3.1, Table 9, p. 45; 4.6.6.7.5, p. 56 | ARW, JPEG, NEF |
| 0x8828 | OECF | Exif 3.1, Table 9, p. 45; 4.6.6.7.6, p. 56 | – |
| 0x8830 | SensitivityType | Exif 3.1, Table 9, p. 45; 4.6.6.7.7, p. 57 | ARW, JPEG, NEF |
| 0x8831 | StandardOutputSensitivity | Exif 3.1, Table 9, p. 45; 4.6.6.7.8, p. 58 | JPEG |
| 0x8832 | RecommendedExposureIndex | Exif 3.1, Table 9, p. 45; 4.6.6.7.9, p. 58 | ARW, JPEG, NEF |
| 0x8833 | ISOSpeed | Exif 3.1, Table 9, p. 45; 4.6.6.7.10, p. 58 | – |
| 0x8834 | ISOSpeedLatitudeyyy | Exif 3.1, Table 9, p. 45; 4.6.6.7.11, p. 58 | – |
| 0x8835 | ISOSpeedLatitudezzz | Exif 3.1, Table 9, p. 45; 4.6.6.7.12, p. 59 | – |
| 0x9000 | ExifVersion | Exif 3.1, Table 8, p. 44; 4.6.6.1.1, p. 47 | ARW, JPEG, NEF |
| 0x9003 | DateTimeOriginal | Exif 3.1, Table 8, p. 44; 4.6.6.6.1, p. 52 | ARW, JPEG, NEF |
| 0x9004 | DateTimeDigitized | Exif 3.1, Table 8, p. 44; 4.6.6.6.2, p. 53 | ARW, JPEG, NEF |
| 0x9010 | OffsetTime | Exif 3.1, Table 8, p. 44; 4.6.6.6.3, p. 53 | ARW, JPEG, NEF |
| 0x9011 | OffsetTimeOriginal | Exif 3.1, Table 8, p. 44; 4.6.6.6.4, p. 53 | ARW, JPEG, NEF |
| 0x9012 | OffsetTimeDigitized | Exif 3.1, Table 8, p. 44; 4.6.6.6.5, p. 53 | ARW, JPEG, NEF |
| 0x9101 | ComponentsConfiguration | Exif 3.1, Table 8, p. 44; 4.6.6.3.3, p. 48 | ARW, JPEG |
| 0x9102 | CompressedBitsPerPixel | Exif 3.1, Table 8, p. 44; 4.6.6.3.4, p. 49 | ARW, JPEG |
| 0x9201 | ShutterSpeedValue | Exif 3.1, Table 9, p. 45; 4.6.6.7.13, p. 59 | JPEG |
| 0x9202 | ApertureValue | Exif 3.1, Table 9, p. 45; 4.6.6.7.14, p. 59 | JPEG |
| 0x9203 | BrightnessValue | Exif 3.1, Table 9, p. 45; 4.6.6.7.15, p. 59 | ARW, JPEG |
| 0x9204 | ExposureBiasValue | Exif 3.1, Table 9, p. 45; 4.6.6.7.16, p. 59 | ARW, JPEG, NEF |
| 0x9205 | MaxApertureValue | Exif 3.1, Table 9, p. 46; 4.6.6.7.17, p. 60 | ARW, JPEG, NEF |
| 0x9206 | SubjectDistance | Exif 3.1, Table 9, p. 46; 4.6.6.7.18, p. 60 | JPEG |
| 0x9207 | MeteringMode | Exif 3.1, Table 9, p. 46; 4.6.6.7.19, p. 60 | ARW, JPEG, NEF |
| 0x9208 | LightSource | Exif 3.1, Table 9, p. 46; 4.6.6.7.20, p. 60 | ARW, JPEG, NEF |
| 0x9209 | Flash | Exif 3.1, Table 9, p. 46; 4.6.6.7.21, p. 61 | ARW, JPEG, NEF |
| 0x920A | FocalLength | Exif 3.1, Table 9, p. 46; 4.6.6.7.23, p. 64 | ARW, JPEG, NEF |
| 0x9214 | SubjectArea | Exif 3.1, Table 9, p. 46; 4.6.6.7.22, p. 62 | ARW, JPEG |
| 0x927C | MakerNote | Exif 3.1, Table 8, p. 44; 4.6.6.4.1, p. 49 | ARW, JPEG, NEF |
| 0x9286 | UserComment | Exif 3.1, Table 8, p. 44; 4.6.6.4.2, p. 49 | ARW, JPEG, NEF |
| 0x9287 | LearningOptOutIn | Exif 3.1, Table 8, p. 44; 4.6.6.4.3, p. 49 | – |
| 0x9290 | SubSecTime | Exif 3.1, Table 8, p. 44; 4.6.6.6.6, p. 54 | ARW, JPEG, NEF |
| 0x9291 | SubSecTimeOriginal | Exif 3.1, Table 8, p. 44; 4.6.6.6.7, p. 54 | ARW, JPEG, NEF |
| 0x9292 | SubSecTimeDigitized | Exif 3.1, Table 8, p. 44; 4.6.6.6.8, p. 54 | ARW, JPEG, NEF |
| 0x9400 | Temperature | Exif 3.1, Table 8, p. 44; 4.6.6.8.2, p. 77 | JPEG |
| 0x9401 | Humidity | Exif 3.1, Table 8, p. 44; 4.6.6.8.3, p. 77 | – |
| 0x9402 | Pressure | Exif 3.1, Table 8, p. 44; 4.6.6.8.4, p. 78 | JPEG |
| 0x9403 | WaterDepth | Exif 3.1, Table 8, p. 44; 4.6.6.8.5, p. 78 | JPEG |
| 0x9404 | Acceleration | Exif 3.1, Table 8, p. 45; 4.6.6.8.6, p. 78 | JPEG |
| 0x9405 | CameraElevationAngle | Exif 3.1, Table 8, p. 45; 4.6.6.8.7, p. 78 | – |
| 0xA000 | FlashpixVersion | Exif 3.1, Table 8, p. 44; 4.6.6.1.2, p. 47 | ARW, JPEG |
| 0xA001 | ColorSpace | Exif 3.1, Table 8, p. 44; 4.6.6.2.1, p. 47 | ARW, JPEG, NEF |
| 0xA002 | PixelXDimension | Exif 3.1, Table 8, p. 44; 4.6.6.3.1, p. 48 | ARW, JPEG |
| 0xA003 | PixelYDimension | Exif 3.1, Table 8, p. 44; 4.6.6.3.2, p. 48 | ARW, JPEG |
| 0xA004 | RelatedSoundFile | Exif 3.1, Table 8, p. 44; 4.6.6.5.1, p. 51 | NEF |
| 0xA005 | Interoperability IFD Pointer | Exif 3.1, 4.6.3.3.1, p. 29 | ARW, JPEG |
| 0xA20B | FlashEnergy | Exif 3.1, Table 9, p. 46; 4.6.6.7.24, p. 64 | – |
| 0xA20C | SpatialFrequencyResponse | Exif 3.1, Table 9, p. 46; 4.6.6.7.25, p. 64 | – |
| 0xA20E | FocalPlaneXResolution | Exif 3.1, Table 9, p. 46; 4.6.6.7.26, p. 65 | JPEG |
| 0xA20F | FocalPlaneYResolution | Exif 3.1, Table 9, p. 46; 4.6.6.7.27, p. 66 | JPEG |
| 0xA210 | FocalPlaneResolutionUnit | Exif 3.1, Table 9, p. 46; 4.6.6.7.28, p. 66 | JPEG |
| 0xA214 | SubjectLocation | Exif 3.1, Table 9, p. 46; 4.6.6.7.29, p. 66 | – |
| 0xA215 | ExposureIndex | Exif 3.1, Table 9, p. 46; 4.6.6.7.30, p. 66 | – |
| 0xA217 | SensingMethod | Exif 3.1, Table 9, p. 46; 4.6.6.7.31, p. 66 | JPEG, NEF |
| 0xA300 | FileSource | Exif 3.1, Table 9, p. 46; 4.6.6.7.32, p. 67 | ARW, JPEG, NEF |
| 0xA301 | SceneType | Exif 3.1, Table 9, p. 46; 4.6.6.7.33, p. 67 | ARW, JPEG, NEF |
| 0xA302 | CFAPattern | Exif 3.1, Table 9, p. 46; 4.6.6.7.34, p. 67 | JPEG, NEF |
| 0xA401 | CustomRendered | Exif 3.1, Table 9, p. 46; 4.6.6.7.35, p. 69 | ARW, JPEG, NEF |
| 0xA402 | ExposureMode | Exif 3.1, Table 9, p. 46; 4.6.6.7.36, p. 69 | ARW, JPEG, NEF |
| 0xA403 | WhiteBalance | Exif 3.1, Table 9, p. 46; 4.6.6.7.37, p. 69 | ARW, JPEG, NEF |
| 0xA404 | DigitalZoomRatio | Exif 3.1, Table 9, p. 46; 4.6.6.7.38, p. 69 | ARW, JPEG, NEF |
| 0xA405 | FocalLengthIn35mmFilm | Exif 3.1, Table 9, p. 46; 4.6.6.7.39, p. 70 | ARW, JPEG, NEF |
| 0xA406 | SceneCaptureType | Exif 3.1, Table 9, p. 46; 4.6.6.7.40, p. 70 | ARW, JPEG, NEF |
| 0xA407 | GainControl | Exif 3.1, Table 9, p. 46; 4.6.6.7.41, p. 70 | ARW, JPEG, NEF |
| 0xA408 | Contrast | Exif 3.1, Table 9, p. 46; 4.6.6.7.42, p. 70 | ARW, JPEG, NEF |
| 0xA409 | Saturation | Exif 3.1, Table 9, p. 46; 4.6.6.7.43, p. 71 | ARW, JPEG, NEF |
| 0xA40A | Sharpness | Exif 3.1, Table 9, p. 46; 4.6.6.7.44, p. 71 | ARW, JPEG, NEF |
| 0xA40B | DeviceSettingDescription | Exif 3.1, Table 9, p. 46; 4.6.6.7.45, p. 71 | – |
| 0xA40C | SubjectDistanceRange | Exif 3.1, Table 9, p. 46; 4.6.6.7.46, p. 72 | JPEG, NEF |
| 0xA40D | DevelopmentType | Exif 3.1, Table 9, p. 46; 4.6.6.7.47, p. 72 | – |
| 0xA40E | DevelopmentTypeDescription | Exif 3.1, Table 9, p. 46; 4.6.6.7.48, p. 73 | – |
| 0xA40F | DistortionCorrection | Exif 3.1, Table 9, p. 46; 4.6.6.7.49, p. 74 | – |
| 0xA410 | ChromaticAberrationCorrection | Exif 3.1, Table 9, p. 46; 4.6.6.7.50, p. 74 | – |
| 0xA411 | ShadingCorrection | Exif 3.1, Table 9, p. 46; 4.6.6.7.51, p. 74 | – |
| 0xA412 | NoiseReduction | Exif 3.1, Table 9, p. 46; 4.6.6.7.52, p. 74 | – |
| 0xA420 | ImageUniqueID | Exif 3.1, Table 8, p. 45; 4.6.6.9.1, p. 79 | JPEG |
| 0xA430 | CameraOwnerName | Exif 3.1, Table 8, p. 45; 4.6.6.9.2, p. 79 | JPEG |
| 0xA431 | BodySerialNumber | Exif 3.1, Table 8, p. 45; 4.6.6.9.3, p. 79 | ARW, JPEG, NEF |
| 0xA432 | LensSpecification | Exif 3.1, Table 8, p. 45; 4.6.6.9.4, p. 79 | ARW, JPEG, NEF |
| 0xA433 | LensMake | Exif 3.1, Table 8, p. 45; 4.6.6.9.5, p. 80 | JPEG, NEF |
| 0xA434 | LensModel | Exif 3.1, Table 8, p. 45; 4.6.6.9.6, p. 80 | ARW, JPEG, NEF |
| 0xA435 | LensSerialNumber | Exif 3.1, Table 8, p. 45; 4.6.6.9.7, p. 80 | JPEG, NEF |
| 0xA436 | ImageTitle | Exif 3.1, Table 8, p. 45; 4.6.6.9.8, p. 80 | – |
| 0xA437 | Photographer | Exif 3.1, Table 8, p. 45; 4.6.6.9.9, p. 80 | – |
| 0xA438 | ImageEditor | Exif 3.1, Table 8, p. 45; 4.6.6.9.10, p. 80 | – |
| 0xA439 | CameraFirmware | Exif 3.1, Table 8, p. 45; 4.6.6.9.11, p. 81 | – |
| 0xA43A | RAWDevelopingSoftware | Exif 3.1, Table 8, p. 45; 4.6.6.9.12, p. 81 | – |
| 0xA43B | ImageEditingSoftware | Exif 3.1, Table 8, p. 45; 4.6.6.9.13, p. 81 | – |
| 0xA43C | MetadataEditingSoftware | Exif 3.1, Table 8, p. 45; 4.6.6.9.14, p. 82 | – |
| 0xA460 | CompositeImage | Exif 3.1, Table 9, p. 47; 4.6.6.7.53, p. 75 | ARW, JPEG, NEF |
| 0xA461 | SourceImageNumberOfCompositeImage | Exif 3.1, Table 9, p. 47; 4.6.6.7.54, p. 75 | – |
| 0xA462 | SourceExposureTimesOfCompositeImage | Exif 3.1, Table 9, p. 47; 4.6.6.7.55, p. 76 | – |
| 0xA500 | Gamma | Exif 3.1, Table 8, p. 44; 4.6.6.2.2, p. 48 | ARW, JPEG |

96 names.


## 5. The GPS IFD

The GPS IFD, which IFD0's tag 0x8825 points to: Table 14 (pp. 82 to 83) [2].
Its tags number from 0x0000, so their numbers repeat other
directories'; a GPS tag's name holds only in the GPS IFD.

| Tag | Name | Document, where | Seen in |
|---|---|---|---|
| 0x0000 | GPSVersionID | Exif 3.1, Table 14, p. 82; 4.6.7.1.1, p. 83 | ARW, JPEG, NEF |
| 0x0001 | GPSLatitudeRef | Exif 3.1, Table 14, p. 82; 4.6.7.1.2, p. 83 | ARW, JPEG, NEF |
| 0x0002 | GPSLatitude | Exif 3.1, Table 14, p. 82; 4.6.7.1.3, p. 84 | ARW, JPEG, NEF |
| 0x0003 | GPSLongitudeRef | Exif 3.1, Table 14, p. 82; 4.6.7.1.4, p. 84 | ARW, JPEG, NEF |
| 0x0004 | GPSLongitude | Exif 3.1, Table 14, p. 82; 4.6.7.1.5, p. 84 | ARW, JPEG, NEF |
| 0x0005 | GPSAltitudeRef | Exif 3.1, Table 14, p. 82; 4.6.7.1.6, p. 84 | ARW, JPEG, NEF |
| 0x0006 | GPSAltitude | Exif 3.1, Table 14, p. 82; 4.6.7.1.7, p. 85 | ARW, JPEG, NEF |
| 0x0007 | GPSTimeStamp | Exif 3.1, Table 14, p. 82; 4.6.7.1.8, p. 85 | ARW, JPEG, NEF |
| 0x0008 | GPSSatellites | Exif 3.1, Table 14, p. 82; 4.6.7.1.9, p. 86 | JPEG, NEF |
| 0x0009 | GPSStatus | Exif 3.1, Table 14, p. 82; 4.6.7.1.10, p. 86 | ARW, JPEG |
| 0x000A | GPSMeasureMode | Exif 3.1, Table 14, p. 82; 4.6.7.1.11, p. 86 | ARW |
| 0x000B | GPSDOP | Exif 3.1, Table 14, p. 82; 4.6.7.1.12, p. 86 | ARW |
| 0x000C | GPSSpeedRef | Exif 3.1, Table 14, p. 82; 4.6.7.1.13, p. 86 | ARW, JPEG |
| 0x000D | GPSSpeed | Exif 3.1, Table 14, p. 82; 4.6.7.1.14, p. 87 | ARW, JPEG |
| 0x000E | GPSTrackRef | Exif 3.1, Table 14, p. 83; 4.6.7.1.15, p. 87 | ARW |
| 0x000F | GPSTrack | Exif 3.1, Table 14, p. 83; 4.6.7.1.16, p. 87 | ARW |
| 0x0010 | GPSImgDirectionRef | Exif 3.1, Table 14, p. 83; 4.6.7.1.17, p. 87 | JPEG, NEF |
| 0x0011 | GPSImgDirection | Exif 3.1, Table 14, p. 83; 4.6.7.1.18, p. 88 | JPEG, NEF |
| 0x0012 | GPSMapDatum | Exif 3.1, Table 14, p. 83; 4.6.7.1.19, p. 88 | ARW, JPEG, NEF |
| 0x0013 | GPSDestLatitudeRef | Exif 3.1, Table 14, p. 83; 4.6.7.1.20, p. 88 | – |
| 0x0014 | GPSDestLatitude | Exif 3.1, Table 14, p. 83; 4.6.7.1.21, p. 88 | – |
| 0x0015 | GPSDestLongitudeRef | Exif 3.1, Table 14, p. 83; 4.6.7.1.22, p. 89 | – |
| 0x0016 | GPSDestLongitude | Exif 3.1, Table 14, p. 83; 4.6.7.1.23, p. 89 | – |
| 0x0017 | GPSDestBearingRef | Exif 3.1, Table 14, p. 83; 4.6.7.1.24, p. 89 | JPEG |
| 0x0018 | GPSDestBearing | Exif 3.1, Table 14, p. 83; 4.6.7.1.25, p. 89 | JPEG |
| 0x0019 | GPSDestDistanceRef | Exif 3.1, Table 14, p. 83; 4.6.7.1.26, p. 89 | – |
| 0x001A | GPSDestDistance | Exif 3.1, Table 14, p. 83; 4.6.7.1.27, p. 90 | – |
| 0x001B | GPSProcessingMethod | Exif 3.1, Table 14, p. 83; 4.6.7.1.28, p. 90 | – |
| 0x001C | GPSAreaInformation | Exif 3.1, Table 14, p. 83; 4.6.7.1.29, p. 91 | – |
| 0x001D | GPSDateStamp | Exif 3.1, Table 14, p. 83; 4.6.7.1.30, p. 91 | ARW, JPEG, NEF |
| 0x001E | GPSDifferential | Exif 3.1, Table 14, p. 83; 4.6.7.1.31, p. 91 | ARW, JPEG |
| 0x001F | GPSHPositioningError | Exif 3.1, Table 14, p. 83; 4.6.7.1.32, p. 91 | JPEG |

32 names.


## 6. The Interoperability IFD

The Interoperability IFD, which the Exif IFD's tag 0xA005 points to.
Exif names one tag (Table 16, p. 92) and leaves the others to each
interoperability rule [2]; DCF names four more (section 4.4.5.3,
pp. 19 to 20) [3], and repeats them for DCF option files and thumbnail
files. No NEF holds this IFD.

| Tag | Name | Document, where | Seen in |
|---|---|---|---|
| 0x0001 | InteroperabilityIndex | Exif 3.1, Table 16, p. 92; 4.6.8.1.1, p. 92; DCF 2.0, 4.4.5.3, p. 19 | ARW, JPEG |
| 0x0002 | InteroperabilityVersion | DCF 2.0, 4.4.5.3, p. 19 | ARW, JPEG |
| 0x1000 | RelatedImageFileFormat | DCF 2.0, 4.4.5.3, p. 19 | – |
| 0x1001 | RelatedImageWidth | DCF 2.0, 4.4.5.3, p. 20 | JPEG |
| 0x1002 | RelatedImageLength | DCF 2.0, 4.4.5.3, p. 20 | JPEG |

5 names.


## 7. The Multi-Picture index

The MP Index IFD of a JPEG's Multi-Picture Format segment
([JPEG notes](jpeg/container-and-metadata.md#71-the-segment-and-its-index)):
Table 3 (p. 12) [4]. The MP Attribute IFD, which the same document
defines for each image, is on no file walked, and raw-files does not
read it.

| Tag | Name | Document, where | Seen in |
|---|---|---|---|
| 0xB000 | MPFVersion | CIPA DC-007-2026, Table 3, p. 12 | JPEG |
| 0xB001 | NumberOfImages | CIPA DC-007-2026, Table 3, p. 12 | JPEG |
| 0xB002 | MPEntry | CIPA DC-007-2026, Table 3, p. 12 | JPEG |
| 0xB003 | ImageUIDList | CIPA DC-007-2026, Table 3, p. 12 | – |
| 0xB004 | TotalFrames | CIPA DC-007-2026, Table 3, p. 12 | – |

5 names.


## 8. MakerNotes and private directories

A maker's MakerNote, the IFDs inside it such as Nikon's preview IFD,
and Sony's SR2Private IFD carry no names on this page: no maker
publishes its tags, and a name only another reader's tables give is
not taken (ADR023). The format notes for
[NEF](nef/makernote-and-metadata.md#2-key-tags) and
[ARW](arw/makernote-and-metadata.md#2-key-tags) describe the MakerNote
tags they use, and those descriptions stay in the notes; raw-files will
give such entries by number only.

## 9. Tags no document names

These tags were found in the standard directories (section 1 gives the
files and the date) and are named by none of the documents read, so
raw-files will give them by number only. A tag a document names in one
kind of directory has no name in another. "Public" and "mine" are file
counts per format.

| Directory | Tag | Files, public / mine | Why it has no name |
|---|---|---|---|
| IFD chain | 0x000B | JPEG 0 / 5 | no document read names it |
| IFD chain | 0x014A | ARW 243 / 295; NEF 383 / 6314 | the DNG specification uses SubIFD trees "as described in the TIFF-EP specification" and names no tag; TIFF-EP was not read |
| IFD chain | 0x02BC | ARW 152 / 40; NEF 227 / 6186 | XMP part 3 (p. 10; Table 12, p. 19) and the DNG specification (p. 14) call it "the XMP metadata tag (700)", holding an XMP packet, and give it no field name |
| IFD chain | 0x0301 | JPEG 0 / 3 | no document read names it |
| IFD chain | 0x0303 | JPEG 0 / 3 | no document read names it |
| IFD chain | 0x4746 | JPEG 0 / 7 | no document read names it |
| IFD chain | 0x5110 | JPEG 0 / 3 | no document read names it |
| IFD chain | 0x5111 | JPEG 0 / 3 | no document read names it |
| IFD chain | 0x5112 | JPEG 0 / 3 | no document read names it |
| IFD chain | 0x7000 | ARW 241 / 277 | no document read names it |
| IFD chain | 0x7001 | ARW 241 / 277 | no document read names it |
| IFD chain | 0x7010 | ARW 239 / 277 | no document read names it |
| IFD chain | 0x7011 | ARW 239 / 277 | no document read names it |
| IFD chain | 0x7020 | ARW 228 / 274 | no document read names it |
| IFD chain | 0x7031 | ARW 197 / 265 | no document read names it |
| IFD chain | 0x7032 | ARW 197 / 265 | no document read names it |
| IFD chain | 0x7034 | ARW 197 / 265 | no document read names it |
| IFD chain | 0x7035 | ARW 197 / 265 | no document read names it |
| IFD chain | 0x7036 | ARW 197 / 265 | no document read names it |
| IFD chain | 0x7037 | ARW 197 / 265 | no document read names it |
| IFD chain | 0x7038 | ARW 45 / 13 | no document read names it |
| IFD chain | 0x7039 | ARW 22 / 6 | no document read names it |
| IFD chain | 0x703A | ARW 44 / 14 | no document read names it |
| IFD chain | 0x703B | ARW 44 / 14 | no document read names it |
| IFD chain | 0x703C | ARW 44 / 14 | no document read names it |
| IFD chain | 0x7310 | ARW 149 / 40 | no document read names it |
| IFD chain | 0x7313 | ARW 149 / 40 | no document read names it |
| IFD chain | 0x7316 | ARW 149 / 40 | no document read names it |
| IFD chain | 0x74C7 | ARW 149 / 40 | no document read names it |
| IFD chain | 0x74C8 | ARW 149 / 40 | no document read names it |
| IFD chain | 0x828D | ARW 241 / 277; NEF 381 / 6308 | no document read names it |
| IFD chain | 0x828E | ARW 239 / 277; NEF 381 / 6308 | no document read names it |
| IFD chain | 0x83BB | ARW 1 / 0 | XMP part 3 (p. 10; Table 12, p. 19) and the DNG specification (p. 14) call it "the IPTC metadata tag (33723)", and give it no field name |
| IFD chain | 0x8773 | NEF 2 / 0 | no document read names it |
| IFD chain | 0x9003 | JPEG 0 / 12; NEF 381 / 6308 | Exif names it in the Exif IFD only |
| IFD chain | 0x9216 | JPEG 0 / 5; NEF 383 / 6308 | no document read names it |
| IFD chain | 0x9217 | NEF 381 / 6308 | no document read names it |
| IFD chain | 0x9C9D | JPEG 0 / 2 | no document read names it |
| IFD chain | 0xA401 | JPEG 0 / 6 | Exif names it in the Exif IFD only |
| IFD chain | 0xA402 | JPEG 0 / 6 | Exif names it in the Exif IFD only |
| IFD chain | 0xA403 | JPEG 0 / 6 | Exif names it in the Exif IFD only |
| IFD chain | 0xA406 | JPEG 0 / 6 | Exif names it in the Exif IFD only |
| IFD chain | 0xA40A | JPEG 0 / 6 | Exif names it in the Exif IFD only |
| IFD chain | 0xA40B | JPEG 0 / 1 | Exif names it in the Exif IFD only |
| IFD chain | 0xA40C | JPEG 0 / 6 | Exif names it in the Exif IFD only |
| IFD chain | 0xC4A5 | ARW 229 / 276; JPEG 5 / 376 | no document read names it |
| IFD chain | 0xC616 | ARW 2 / 0 | the DNG specification names it CFAPlaneColor (p. 26), but names are taken from it only for the DNG tags raw-files reads |
| IFD chain | 0xC61D | ARW 127 / 34 | the DNG specification names it WhiteLevel (p. 29), but names are taken from it only for the DNG tags raw-files reads |
| IFD chain | 0xC7D5 | NEF 121 / 4418 | no document read names it |
| IFD chain | 0xC7D6 | NEF 121 / 4418 | no document read names it |
| IFD chain | 0xEA1C | JPEG 0 / 13 | no document read names it |
| Exif | 0x010D | JPEG 0 / 1 | TIFF 6.0 names it in an image's IFD only |
| Exif | 0xEA1C | JPEG 0 / 13 | no document read names it |
| Exif | 0xEA1D | JPEG 0 / 11 | no document read names it |

54 tags.


## Sources

All documents were downloaded and read on 9 November 2026.

[1] Adobe Systems, "TIFF, Revision 6.0", Final, June 3, 1992; the
    copy Adobe published. **[P]**
[2] CIPA DC-008-Translation-2026, "Exchangeable image file format for
    digital still cameras: Exif Version 3.1" (January 2026),
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-008-2026-E.
    **[P]**
[3] CIPA DC-009-Translation-2010, "Design rule for Camera File system:
    DCF Version 2.0 (Edition 2010)",
    https://www.cipa.jp/std/documents/download_e.html?DC-009-2010_E.
    **[P]**
[4] CIPA DC-007-Translation-2026, "Multi-Picture Format" (August 2026),
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-007-2026_E.
    **[P]**
[5] Adobe, "Digital Negative (DNG) Specification", version 1.7.1.0,
    September 2023,
    https://helpx.adobe.com/content/dam/help/en/photoshop/pdf/DNG_Spec_1_7_1_0.pdf.
    **[P]**
[6] Adobe, "XMP Specification Part 3: Storage in Files", January 2020,
    https://github.com/adobe/XMP-Toolkit-SDK/blob/main/docs/XMPSpecificationPart3.pdf.
    **[P]**
