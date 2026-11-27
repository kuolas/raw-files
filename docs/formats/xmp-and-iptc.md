# XMP and IPTC

A photographer, or a program, adds what the camera cannot know: a
rating, a label, keywords, a title, a description, who made the
picture.
Two standards hold these values. XMP is XML, inside the file or in a
sidecar file beside it. IIM, the IPTC's Information Interchange Model,
is an older binary block. This page records where NEF, ARW and camera
JPEG files hold each, and what they hold. raw-files will read both as
[ADR024](../architecture-decisions/adr024-xmp-and-iptc.md) decides.
Its readers will be written from this page and from the standards it
cites
([ADR013](../architecture-decisions/adr013-clean-room-and-third-party-code.md)).

Where this page says *my own* files, it is me, Alvaro Kuolas, who
maintains raw-files, writing about my own cameras' files. They stay
private, and are cited by counts only.

The page serves every format raw-files reads. A packet is the same XML
in a NEF, an ARW and a JPEG file, and an IIM block the same datasets.
Only the place that holds them differs. Each format's notes give that
place and point here.

The tags [P], [S], [I] and [F] mean what the
[format notes](README.md#how-a-facts-source-is-marked) say. A property
or a dataset is marked, as
[ADR011](../architecture-decisions/adr011-metadata-from-standard-tags.md)
marks a code:

- **observed**: files hold it, with the count, the public files first;
  "in my files only" where no public file holds it;
- **documented only**: a standard defines it, and no measured file
  holds it.

## Contents

- [1. How the facts were gathered](#1-how-the-facts-were-gathered)
- [2. Where the packet and the block lie](#2-where-the-packet-and-the-block-lie)
- [3. What cameras write](#3-what-cameras-write)
- [4. What editors and phones write](#4-what-editors-and-phones-write)
- [5. Sidecars](#5-sidecars)
- [6. The XML the packets use](#6-the-xml-the-packets-use)
- [7. IPTC Core in the files](#7-iptc-core-in-the-files)
- [8. The IIM block](#8-the-iim-block)
- [9. A segment that repeats](#9-a-segment-that-repeats)
- [10. Text that Exif and XMP both record](#10-text-that-exif-and-xmp-both-record)
- [11. The date created](#11-the-date-created)
- [12. How raw-files will read them](#12-how-raw-files-will-read-them)
- [13. Named gaps](#13-named-gaps)
- [Sources](#sources)

## 1. How the facts were gathered

**The standards.** Six documents, read on 27 November 2026
([Sources](#sources)):

- XMP Specification Part 1, the data model, which is ISO 16684-1 [1];
- XMP Part 2, the additional properties [2];
- XMP Part 3, storage in files [3];
- the IPTC Photo Metadata Standard 2025.1 [4];
- the IPTC's Information Interchange Model 4.2 [5];
- the Metadata Working Group's *Guidelines for Handling Image
  Metadata* 2.0 [6], on a value that Exif, XMP and IIM all record.

Each fact from a document cites its section or table and printed page.
ExifTool's XMP and IPTC tag pages were read only to compare with. No
fact on this page rests on them, and no other reader's source was
read.

**The files.** Counted on 27 November 2026 with a program of my own,
over raw-files' own entries
([ADR023](../architecture-decisions/adr023-every-entry-a-file-records.md))
on a raw file. For what the entries do not give, it walks a JPEG's
segments, IIM's datasets and Photoshop's image resources itself. It
parsed the packets with an XML library, which decides nothing for
raw-files. It printed counts, property names and the names of
software, never a value that can name a person or a place. **[F]**

| Set | NEF and NRW | ARW family | JPEG |
|---|---|---|---|
| Public: every file of the [raw.pixls.us](https://raw.pixls.us/) clone raw-files opens | 289 | 209 | 9 |
| Fixtures: the test fixtures and samples | 95 | 36 | 18 |
| Mine: my photography disks and cards | 30,422 | 3,217 | 132,700 |

The fixtures are copies, whole or stripped, of public files and of
mine, so their counts are given apart. A copy of my files kept on a
backup disk is in no count. Where a count adds sets, it says so.

## 2. Where the packet and the block lie

| Where | Public | Fixtures | Mine |
|---|---|---|---|
| NEF, IFD0 tag 0x02BC (XMP) | 204 | 23 | 29,775 |
| ARW, IFD0 tag 0x02BC (XMP) | 130 | 22 | 0 |
| ARW, IFD0 tag 0x83BB (IIM) | 1 | 0 | 0 |
| NEF or ARW, IFD0 tag 0x8649 (Photoshop's resources) | 0 | 0 | 0 |
| JPEG, APP1 XMP segment | 0 | 3 | 25,377 |
| JPEG, APP1 Extended XMP segment | 0 | 0 | 0 |
| JPEG, APP13 `Photoshop 3.0` segment | 0 | 0 | 907 |
| A preview's own XMP or APP13 segment, inside a raw file | 0 | 0 | 0 |
| A sidecar beside the file | 0 | 0 | 10,256 |

- **In a NEF or ARW file**, the packet is the value of IFD0's tag 0x02BC
  (700). XMP Part 3 calls it "the XMP metadata tag", of type UNDEFINED
  or BYTE, and asks for UTF-8 [P][3] (Table 13, p. 19). Every packet
  measured is of type BYTE. [F]
- **The IIM block** is the value of IFD0's tag 0x83BB (33723). Tag
  0x8649 (34377) holds Photoshop's image resources, which can repeat
  it [P][3] (Table 12, p. 19). The one public file with 0x83BB writes it
  as LONG, 8 values. [F]
- **In a JPEG file**, the packet is an APP1 segment whose payload starts
  with `http://ns.adobe.com/xap/1.0/` and a NUL. A packet over 65,502
  bytes continues as Extended XMP in further APP1 segments [P][3]
  (Tables 5 and 6, pp. 13 to 14; §1.1.3.1, p. 14). Part 3 advises the
  Exif segment first, then the XMP segment, then the Photoshop segment
  (p. 14).
- **The IIM block in a JPEG** is resource 1028 among Photoshop's image
  resources. They lie in an APP13 segment that starts with
  `Photoshop 3.0` and a NUL [P][3] (Table 5, p. 13; Table 36, p. 68).
  Each resource is `8BIM`, a two-byte number, a name as a Pascal string
  and a four-byte length, then its data [P][3] (§3.1.3, Table 33,
  p. 66). The numbers are big-endian. The name and the data are each padded with a NUL to
  an even length, counted from the start of the resource section
  (p. 67). Part 3 says a resource
  section over 64 KB is split over several segments (p. 14).
- **No packet lies anywhere else.** A scan of every byte of every public
  and fixture raw file found a packet's opening only where 0x02BC
  points. No raw file's preview holds a packet of its own, and no JPEG
  holds Extended XMP. [F]
- **Every packet and block lies inside what `open` reads**, in every
  set. That is because `open` follows IFD0, not because packets lie
  near the start. Two edited public ARW files, the clone's 348 (a NEX-3)
  and 362 (a DSLR-A550), hold IFD0 in their last kilobyte, and the
  packet after it. [F]
- **Sizes.** The largest NEF packet is 34,816 bytes (a Z f), and every
  Z 9 packet is 32,768. The 128 camera packets of ARW files are 4,096
  bytes each, and the two edited ones 740 and 344. The largest JPEG
  segment is 57,315 bytes (an ILCE-7M3 fixture), and 33,759 among my
  files. [F]
- **Every packet is UTF-8.** No packet is UTF-16 or holds bytes that are
  not UTF-8. [F]

## 3. What cameras write

**Nikon.** Every packet on a public NEF, per body, the clone's files
each counted once; my own files per body after them. **[F]**

| What the packet holds | Public bodies (files) | My files |
|---|---|---|
| no property: an empty `rdf:RDF` in `x:xmpmeta` | D3200, D5200, D600, D610, D7100, D800, D800E, Df (29) | D3200 3,568, D600 1,812, D7100 400 NEF |
| `xmp:Rating` 0 alone | D3300, D5300 (3); the Nikon 1 bodies, under the prefix `xap` (15) | D3300 659 JPEG |
| `xmp:CreatorTool` (the body and its firmware) and `xmp:CreateDate` | D4, D4S, D7200, D750, D810 (28) | D7200 7,116 NEF |
| the same and `xmp:Rating` 0 | D3400, D3500, D5, D500, D5500, D5600, D7500, D850 (32) | |
| the same and Adobe Camera Raw's defaults (`crd:`) | D6, D780 and every Z body (97) | Z 9 16,879 NEF, 27 JPEG |

- The `crd:` properties are those of Adobe Camera Raw's defaults
  namespace: the exposure, the contrast, the highlights and shadows,
  sharpening, noise reduction, saturation and hues. Their number grows
  with the body: [F]
  - 16 on the D6, the D780, and the Z 5, Z 50, Z 6, Z 6II, Z 7, Z 7II
    and Z fc;
  - 18 on the public Z 9 files;
  - 25 on the Z 30;
  - 27 on the Z 8, Z f, Z6III, Z50II and Z5II, and on every Z 9 file of
    mine.
- My D300S writes no packet, and neither do the older public bodies
  and the Coolpix bodies: 85 of the 289 public files have none. [F]
- Nikon's packet holds no person's text. [F]
- 328 JPEG packets of mine name the Z 9 as their creator tool
  (section 4). The 27 from my cards hold the `crd:` properties as the
  NEFs do. The other 301 hold none, and where they came from was not
  traced. [F]

**Sony.** Every ARW body from FileFormat 3.3.3.0 on writes a packet
of `xmp:Rating` 0 ([ARW notes](arw/container-and-previews.md)). 128
public files hold it, 27 of them with `xmpMM:DocumentID` and
`InstanceID` too. My ILCE-7M2 writes no packet. **[F]**

**The camera JPEGs.** The three camera JPEG fixtures that write a packet
hold `xmp:Rating` 0 alone. My D3300's JPEGs hold the same, and my
D7200's name the body in `xmp:CreatorTool`, as its NEFs do. [F]

Every rating a camera writes is 0. [F]

## 4. What editors and phones write

**Who wrote the JPEG packets.** Of my JPEG packets that name the tool
that wrote them (`xmp:CreatorTool`), 6,152 in all: **[F]**

| Tool | Packets |
|---|---|
| a Nikon body: the D7200 (4,853), the Z 9 (328) | 5,181 |
| Microsoft's Windows Photo Viewer | 532 |
| Adobe's Lightroom Classic (342) and Photoshop (5) | 347 |
| GIMP | 73 |
| Apple's Photos | 17 |
| Capture One | 1 |
| a phone | 1 |

**The namespaces editors and phones add**, on my JPEG packets, beside
XMP's own (`xmp:`, `dc:`, `xmpMM:`): **[F]**

| Namespace | Packets |
|---|---|
| Google's camera namespace, on phones | 2,455 |
| Xiaomi's camera namespace, on phones | 2,161 |
| Sony Ericsson's `cell:` namespace, on phones | 1,386 |
| Photoshop's (`photoshop:`) | 434 |
| Adobe's auxiliary Exif properties (`aux:`) | 415 |
| Adobe Camera Raw's settings (`crs:`) | 348 |
| Microsoft's photo namespace, under two URIs, with and without the final slash | 142 |
| the Metadata Working Group's regions | 93 |
| GIMP's | 73 |
| PLUS | 70 |
| the IPTC Extension | 70 |
| IPTC Core | 0 |

The 70 PLUS and IPTC Extension packets hold only empty lists. One
public ARW, the edited 362, holds an IPTC Core structure: the creator's
contact information, with a city. [F]

**The descriptive properties**, on my JPEG packets: **[F]**

| Property | Packets |
|---|---|
| `xmp:Rating` | 6,382: 0 on 6,368, 1 on 8, 2 and 3 on one each, 5 on 4 |
| `photoshop:DateCreated` | 430 |
| `dc:description` | 220 |
| `dc:creator`, one item each | 143 |
| `dc:rights` | 142 |
| `dc:subject` | 59, of which 58 are empty |
| `photoshop:LegacyIPTCDigest` | 67 |
| `MicrosoftPhoto:Rating` | 5 |
| `xmp:Label` | 1, a text of 7 letters |
| `dc:title` | 1 |
| `xmpRights:Marked`, `xmpRights:WebStatement` | 1 each |

- No rating of -1, "rejected", and no fraction occurs in any set. [F]
- The 14 JPEG packets with a rating of 1 to 5 were not tied to a
  writer. [F]
- XMP Part 1 gives `xmp:Rating` as -1 or a number from 0 to 5, where -1
  is "rejected" and 0 "unrated". A missing rating "should be assumed"
  to be 0 [P][1] (§8.4, p. 27). The Metadata Working Group allows
  fractions, and asks a reader to round a value out of range to -1 or 5
  [P][6] (§5.5, p. 41).

## 5. Sidecars

- **Names.** XMP Part 3 names a sidecar after the file, with `.xmp` in
  place of its extension [P][3] (p. 7). All 10,256 sidecars on my disks
  are named `<name>.<ext>.xmp` instead, the file's whole name and
  `.xmp`. 5,433 lie beside a NEF, 4,819 beside a JPEG and 4 beside a
  PNG.
  None is named as Part 3 says. [F]
- **Who wrote them.** Every one names Exiv2's toolkit
  (`XMP Core 4.4.0-Exiv2`) and holds darktable's namespace: they are
  darktable's. [F]
- **Their form.** Each is a whole XML document, with an XML declaration
  that names UTF-8 and no packet wrapper, as Part 3 asks [P][3] (p. 7).
  The largest is 8,088 bytes. [F]
- **What they hold.** Each holds `xmp:Rating` (1 on every one), a
  `dc:subject` of 3 items (4 or 5 on one), and darktable's history.
  9,854 hold `dc:creator` and `dc:rights`, and 6,221 `dc:description`.
  The file's own packet holds no rating, keyword or text on any of
  them. [F]
- No public file has a sidecar. [F]

## 6. The XML the packets use

Every packet that parses holds one `rdf:RDF` element under
`x:xmpmeta`. Of my packets and sidecars: **[F]**

| Feature | Packets |
|---|---|
| a DOCTYPE, a CDATA section, a character reference, `&amp;` | none, in any set |
| `&lt;` and `&gt;` (Xiaomi's camera namespace) | 2,161 |
| `&quot;` | 548 |
| a comment | 1 |
| `rdf:parseType="Resource"` | 153 |
| properties as attributes of `rdf:Description` | every camera packet and sidecar |
| several `rdf:Description` elements | 17,162 |
| `xml:lang` | 10,209 |
| attributes in single quotes | 9,706 |
| a packet wrapper whose end says `w`, `r` or that has none | 52,001, 60, 13,347 |
| an XML declaration, naming UTF-8 | 10,256, the sidecars |

- **110 packets do not parse.** All are from a Sony Ericsson phone, and
  each uses the prefix `cell:` without declaring it, which Namespaces in
  XML does not allow. Every other packet of every set is well-formed.
  [F]
- **Names are compared by namespace URI**, never by prefix [P][1] (§6.2,
  p. 5). The Nikon 1 bodies, and 4 packets of mine, bind the XMP
  namespace to the prefix `xap`; the rest bind it to `xmp`. [F]
- **Depth.** The public and camera packets are at most 7 elements deep.
  Editors' go deeper, to 22 elements on 56 of my JPEG packets. [F]
- **Values per packet**, counting every property, field, item and
  qualifier: 357 at most in a JPEG packet of mine. A sidecar holds 173
  at most, and a camera's packet 30. Of my 65,298 packets and sidecars that parse,
  37,583 hold 10 or fewer, 27,344 from 11 to 100 and 371 from 101 to
  500. [F]
- **A repeated property.** XMP Part 1 allows a property's name once in a
  packet [P][1] (§6.1, p. 4). 15 JPEG packets in my D3200's folder
  repeat ten `exif:` properties, 88 repeats in all, 86 with the same
  value. The tool that wrote them names itself nowhere. Phones' packets hold 12
  more repeats. No repeated property is one IPTC Core or the rating
  uses. [F]

## 7. IPTC Core in the files

The IPTC Photo Metadata Standard 2025.1 defines 25 Core properties,
each an XMP property [P][4] (§7). Most also name the IIM dataset that
holds the same value. Files hold 9 of them. Counts are of XMP packets
and of IIM blocks in my files, but where a public ARW is named.
**[F]**

| IPTC name | XMP property | IIM dataset | Files |
|---|---|---|---|
| Alt Text (Accessibility) | `Iptc4xmpCore:AltTextAccessibility` | none | documented only |
| City | `photoshop:City` | 2:90 | documented only |
| Copyright Notice | `dc:rights` | 2:116 | observed: XMP 142 + 9,854 sidecars; IIM 134 |
| Country | `photoshop:Country` | 2:101 | documented only |
| Country Code | `Iptc4xmpCore:CountryCode` | 2:100 | documented only |
| Creator | `dc:creator` | 2:80 | observed: XMP 143 + 9,854 sidecars; IIM 154 |
| Creator's Contact Info | `Iptc4xmpCore:CreatorContactInfo` | none | observed: the public ARW 362 |
| Creator's jobtitle | `photoshop:AuthorsPosition` | 2:85 | documented only |
| Credit Line | `photoshop:Credit` | 2:110 | documented only |
| Date Created | `photoshop:DateCreated` | 2:55 with 2:60 | observed: XMP 430; IIM 414 |
| Description | `dc:description` | 2:120 | observed: XMP 220 + 6,221 sidecars; IIM 213 |
| Description Writer | `photoshop:CaptionWriter` | 2:122 | documented only |
| Extended Description (Accessibility) | `Iptc4xmpCore:ExtDescrAccessibility` | none | documented only |
| Headline | `photoshop:Headline` | 2:105 | documented only |
| Instructions | `photoshop:Instructions` | 2:40 | observed: IIM 375 |
| Intellectual Genre | `Iptc4xmpCore:IntellectualGenre` | 2:04 | documented only |
| Job Id | `photoshop:TransmissionReference` | 2:103 | observed: IIM 276 |
| Keywords | `dc:subject` | 2:25 | observed, not empty: XMP the public ARW 362, 1 JPEG packet + 10,256 sidecars; IIM the public ARW 362, 17 |
| Province or State | `photoshop:State` | 2:95 | documented only |
| Rights Usage Terms | `xmpRights:UsageTerms` | none | documented only |
| Scene Code | `Iptc4xmpCore:Scene` | none | documented only |
| Source | `photoshop:Source` | 2:115 | documented only |
| Subject Code | `Iptc4xmpCore:SubjectCode` | 2:12 | documented only |
| Sublocation | `Iptc4xmpCore:Location` | 2:92 | documented only |
| Title | `dc:title` | 2:05 | observed: XMP 1 |

- The Creator's Contact Info is a structure of eight parts: the
  address, city, region, postal code, country, phone, email and URL
  [P][4] (§8.1). The one measured holds a city. [F]
- Six are language alternatives, an Alt array whose items carry
  `xml:lang` [P][4] (§7). They are the title, the description, the
  copyright notice, the rights usage terms and the two accessibility
  texts. Every one
  measured holds one item. [F]
- An `x-default` item, when there is one, comes first [P][1]
  (§8.2.2.4, pp. 22 to 23).
- The categories, `photoshop:Category` and `SupplementalCategories`
  (IIM 2:15 and 2:20), are not among the Core properties. IIM 4.1
  marked them deprecated, and IPTC Core did not take them up [P][4]
  (§9.1). No measured file holds one. [F]
- The IPTC Extension and PLUS add 41 more properties [P][4] (§11). No
  measured file holds a value of one: their 70 lists are empty
  (section 4). [F]

## 8. The IIM block

| Where | Public | Mine |
|---|---|---|
| ARW, IFD0 tag 0x83BB | 1 (the edited 362) | 0 |
| JPEG, resource 1028 of the APP13 segments | 0 | 895 files, 906 resources |
| JPEG, Photoshop's MD5 digest of the block, resource 1061 | 0 | 430 resources |

**The datasets.** A dataset is the byte 0x1C, a record number, a dataset
number, a two-byte big-endian length and the data. A length over
32,767 takes IIM's extended form [P][5] (§1.4, p. 13). The datasets
in my files' blocks, the first block of each file (section 9): **[F]**

| Dataset | Blocks |
|---|---|
| 1:90 Coded Character Set | 422 |
| 2:00 Record Version | 391 |
| 2:55 Date Created, 2:60 Time Created | 414 each |
| 2:62 Digital Creation Date, 2:63 Digital Creation Time | 412 each |
| 2:40 Special Instructions | 375 |
| 2:103 Original Transmission Reference | 276 |
| 2:120 Caption/Abstract | 213 |
| 2:80 By-line | 154 |
| 2:116 Copyright Notice | 134 |
| 2:25 Keywords | 17 |
| 2:118 Contact | 1 |
| 2:117, which IIM 4.2 does not define | 1 |

- **The character set.** 1:90 declares UTF-8 with the escape sequence
  `1B 25 47` on 422 of my blocks and on the public one; the rest
  declare nothing [F]. Without 1:90, IIM's default is ISO 646 or
  ISO 4873, which are 7-bit and 8-bit sets [P][5] (1:90, p. 20).
- **The record version** (2:00) is 4 on 342 blocks, 2 on 5 and 0 on 2.
  One block writes 41464, 41 write a single byte, and 504 have none. The public
  block's is 2. IIM makes 2:00 mandatory [P][5] (p. 24). [F]
- **The ends.** 97 of my blocks and the public one end in NUL padding;
  none holds a dataset past its end. [F]
- **No section is split.** The largest Photoshop segment measured is
  under 32 KB, far under the 64 KB at which Part 3 splits a resource
  section. [F]
- **Who wrote them.** Where the writer was checked, it is an editor:
  - the 8 blocks in my Z 9's JPEGs: Lightroom Classic 15.3;
  - the 211 in my ILCE-7M2's: Lightroom Classic 10.1.1 (144), GIMP (66)
    and Capture One (1).

  432 blocks are in JPEGs with no camera Make. The
  rest were not tied to a writer, and none of those bodies is
  documented to write IIM. [F]
- **The digest.** The Metadata Working Group keeps an MD5 digest of the
  whole IIM block in resource 1061. A program that writes both XMP and
  IIM stores it. A reader that finds it changed knows that a program
  unaware of XMP changed the block [P][6] (§4.2.3.2, p. 26).

## 9. A segment that repeats

- **Two Photoshop segments.** 11 of my JPEGs hold two `Photoshop 3.0`
  segments. Each is a whole resource list of its own, 68 to 18,062
  bytes, with its own IIM block. A program appended a segment and left the
  old one. The second block is the fuller in all 11. Where both hold a
  dataset, the values are equal, but for the record version in one.
  1:90 and 2:00 are in the second block only in 9 files, and further
  datasets in 3. [F]
- **Two XMP segments.** 6 of my JPEGs hold two. In five, the two packets
  are 1,911 and 1,907 bytes, of one toolkit and one property set, and
  differ in one property's value. In one, the first is 19,254 bytes,
  with the rating, the creator and an editor's settings. The second is
  658 bytes, with seven camera and lens properties. [F]
- All 17 files are in one folder of my edited pictures. [F]
- No standard says which copy is right. XMP Part 3 speaks of "the" XMP
  segment. The Metadata Working Group reads XMP from the XMP APP1
  segment and IIM from resource 1028. "All other occurrences SHOULD be
  ignored" [P][6] (§4.2.3.4, p. 30).

## 10. Text that Exif and XMP both record

| Value | Exif tag | XMP property | Files where both record it |
|---|---|---|---|
| the creator | Artist | `dc:creator` | 9,997: 143 JPEG packets, 9,854 sidecars |
| the copyright notice | Copyright | `dc:rights` | 9,996: 142 JPEG packets, 9,854 sidecars |
| the description | ImageDescription | `dc:description` | 8 JPEG packets |
| the title | ImageTitle | `dc:title` | none: the title is in XMP only |

On every file where both record a value, the two texts are equal: no
file has them differ. **[F]**

The Metadata Working Group maps these values between Exif and XMP, and
asks a reader to prefer Exif's. Between IIM and XMP it prefers XMP,
unless the digest shows the IIM block changed [P][6] (§4.2.3.1 and
§4.2.3.2, pp. 23 to 28).

## 11. The date created

- **XMP's** `photoshop:DateCreated` is an XMP date [P][1] (§8.2.1.2,
  p. 21), which can stop at the year, the month, the day, the minute,
  the second or a fraction of it, with or without an offset.
- **IIM's** is 2:55, `CCYYMMDD`, with 2:60, `HHMMSS±HHMM` [P][5] (pp. 34
  to 35). A month or a day that cannot be determined is written `00`
  (p. 34).
- **What files hold.** The forms, with their digits masked, in my
  files: **[F]**

  | Where | Form | Values |
  |---|---|---|
  | XMP | `YYYY-MM-DDThh:mm:ss` | 228 |
  | XMP | the same, with hundredths of a second | 193 |
  | XMP | the same, with nine digits of a second | 1 |
  | XMP | the same, with hundredths and an offset, `-hh:mm` | 8 |
  | IIM 2:55 | `CCYYMMDD` | 344 |
  | IIM 2:55 | `CCYY-M-DD`, with a one-digit month | 70 |
  | IIM 2:60 | `HHMMSS`, with no offset | 335 |
  | IIM 2:60 | `HHMMSS±HHMM` | 9 |
  | IIM 2:60 | `HH:MM:SS-HH:MM` | 70 |

- No date is truncated: every XMP date and every IIM block has a time
  to the second. [F]
- The 70 IIM blocks of the second and the last form write the date and
  the time in a form IIM does not define. The 335 times with no offset
  are short of IIM's form too. [F]

## 12. How raw-files will read them

[ADR024](../architecture-decisions/adr024-xmp-and-iptc.md) decides how;
in short, on these files:

- **The packet** will be read when the file is opened. It is IFD0's
  0x02BC in a NEF or ARW file, and the first APP1 XMP segment in a JPEG
  file. `open`
  will find it by following IFD0, as it finds the other entries, so a
  packet lies inside what it reads on every measured file.
- **The IIM block** will be read from IFD0's 0x83BB, else from resource
  1028 of 0x8649. In a JPEG file, it will be the first resource 1028 of
  the `Photoshop 3.0` segments, joined in file order.
- **A sidecar** will be read only when a caller hands it in, by either
  name.
- **Every XMP property** will be given in XMP Part 1's data model. The
  rating, the label and IPTC Core will be typed values, from XMP and
  from IIM each apart.
- **The first copy will stand**, for a segment, a resource, a property
  and a dataset.
- **The 110 phone packets** will cost only themselves: the file opens,
  with no packet.

## 13. Named gaps

What raw-files will not read, each with its count and what it waits
for:

- **Extended XMP**: on no measured file. It waits for a file that holds
  it.
- **A truncated date created**: on no measured file (section 11). It
  waits for a file.
- **The categories** (section 7): on no measured file. IPTC Core does
  not hold them, and the property list gives them.
- **The IPTC Extension's and PLUS's values**: on no measured file but
  as empty lists. The property list gives them.
- **An IIM date and time in a form IIM does not define**: 70 blocks
  (section 11). Reading them waits for a decision of its own.
- **A later copy of a repeated segment**: 11 JPEGs with two IIM blocks
  and 6 with two XMP segments (section 9).
- **A sidecar in UTF-16 or UTF-32**: every measured sidecar is UTF-8.
- **A JPEG's damaged packet**: a JPEG's segments are not entries, so a
  packet that fails to parse is not reachable at all. A NEF's or ARW's
  stays reachable as the bytes of its 0x02BC entry.
- **Writing XMP or IIM**: raw-files reads files.

## Sources

Documents 1 to 6 were downloaded and read on 27 November 2026.

[1] Adobe Systems, "XMP Specification Part 1: Data Model,
    Serialization, and Core Properties", April 2012, the text of ISO
    16684-1:2011,
    https://github.com/adobe/XMP-Toolkit-SDK/blob/main/docs/XMPSpecificationPart1.pdf.
    **[P]**
[2] Adobe, "XMP Specification Part 2: Additional Properties", 2022,
    https://github.com/adobe/XMP-Toolkit-SDK/blob/main/docs/XMPSpecificationPart2.pdf.
    **[P]**
[3] Adobe, "XMP Specification Part 3: Storage in Files", January 2020,
    https://github.com/adobe/XMP-Toolkit-SDK/blob/main/docs/XMPSpecificationPart3.pdf.
    **[P]**
[4] IPTC, "IPTC Photo Metadata Standard 2025.1", revision 1,
    2025-11-26,
    https://iptc.org/std/photometadata/specification/IPTC-PhotoMetadata.
    **[P]**
[5] IPTC and NAA, "Information Interchange Model", version 4, revision
    2, July 2014,
    https://www.iptc.org/std/IIM/4.2/specification/IIMV4.2.pdf.
    **[P]**
[6] Metadata Working Group, "Guidelines for Handling Image Metadata",
    version 2.0, November 2010. The group's site is gone; the copy read
    is the Internet Archive's capture of
    http://www.metadataworkinggroup.org/pdf/mwg_guidance.pdf. **[P]**
