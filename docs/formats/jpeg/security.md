# JPEG: security notes

A JPEG file is untrusted input, and the parsers of its segments, of its
Exif data and of its thumbnails have a long record of memory-safety and
denial-of-service bugs. This page lists published vulnerabilities in
parsers of exactly the structures raw-files reads, grouped by bug class,
and a checklist that maps each class to the raw-files rules. Back to
[the JPEG overview](README.md).

Confidence tags: **[P]** primary (read in the NVD record or the project's
own advisory or changelog), **[S]** secondary (a bug report, a search
result), **[I]** inferred. Numbers in brackets point to the
[Sources](#sources) at the end, retrieved in October 2026. Only
vulnerability records, advisories, changelogs and bug reports were read;
no project's source code.

The raw-files baseline is [ADR008](../../architecture-decisions/adr008-lints-and-panics.md):
no `unsafe` code, and malformed input returns an error, never a panic.
Most bugs below are of the kind Rust prevents, buffer overflows; what Rust
does not prevent is a panic from an out-of-range index, an overflow that
panics in a debug build and wraps in a release build, an unbounded
allocation or a loop that never ends. Those are ours to avoid, and the
Rust records in section 1.6 show that they happen.

## 1. Published vulnerabilities

### 1.1 Segment lengths and the segment walk

| ID | Component | Class | What went wrong | Conf. |
|---|---|---|---|---|
| CVE-2004-0200 (MS04-028) | Microsoft GDI+, Windows XP and Server 2003, Office and Visual Studio 2002 to 2003 | Heap overflow | A comment segment whose length is below 2: subtracting 2 wrapped, and the huge result was a copy length. Published 2004-09-28. [1][2] | P |
| Openwall OW-002 (no CVE) | Netscape 3.0 to 4.73, Mozilla M15 | Heap overflow | The same bug in the comment handler: a length of 1 gave an empty buffer and a wrapped copy count. Advisory 2000-07-25. [3] | P |
| CVE-2018-6612 | jhead 3.00, Exif section | Integer underflow, over-read (CWE-191, CWE-125) | An underflow on the Exif section's length. Published 2018-02-04. [1] | P |
| CVE-2020-26208 | jhead before 3.04, section walk | Heap overflow (CWE-787) | While walking the JPEG sections. Published 2022-02-02. [1] | P |
| CVE-2020-0207 | libjpeg in Android 10, marker search | Out-of-bounds read (CWE-125) | The search for the next marker read past the end of the buffer. Published 2020-06-11. [1] | P |
| CVE-2025-65092 | ESP-IDF hardware JPEG driver, 5.5.1, 5.4.3, 5.3.4 | Out-of-bounds read and write (CWE-125, CWE-191) | A DQT segment whose declared length exceeds its fixed table was copied unchecked. Published 2025-11-21. [1][4] | P |
| CVE-2021-37622, CVE-2021-37623 | Exiv2 0.27.4 and earlier, JPEG segment walk | Endless loop (CWE-835) | The walk that prints a JPEG's structure looped forever on crafted input; the advisories do not say which field failed to advance. Published 2021-08-09. [1][5] | P |
| CVE-2025-55304 | Exiv2 0.28.5 and earlier, JPEG metadata reading | Quadratic time (CWE-407) | Reading the ICC profile of a crafted JPEG took quadratic time. Published 2025-08-29. [1][5] | P |
| CVE-2022-25851 | jpeg-js before 0.4.4 | Endless loop (CWE-835) | A crafted input made the decoder loop forever. Published 2022-06-10. [1] | P |
| CVE-2025-14177 | PHP `getimagesize()`, 8.1 to 8.5 before their fixes | Uninitialised memory returned (CWE-125) | Read in several chunks, an APPn segment's buffer was overwritten without moving forward, and old heap contents were returned. Published 2025-12-27. [1] | P |

### 1.2 The IFDs inside the Exif segment

| ID | Component | Class | What went wrong | Conf. |
|---|---|---|---|---|
| CVE-2019-9278 | libexif (Android 10) | Integer overflow, out-of-bounds write (CWE-190, CWE-787) | The bounds check added an entry's offset to its size; a large value wrapped the sum, and the check passed for data past the end. The maintainer's advice in the bug report: compare before adding. Published 2019-09-27. [1][6] | P, S |
| CVE-2006-4168, CVE-2007-2645 | libexif 0.6.9 to 0.6.15 | Integer overflow, heap overflow | An entry with a very large component count. Published 2007. [1] | P |
| CVE-2007-6351 | libexif 0.6.16 and earlier | Endless recursion | Crafted tags. Published 2007-12-20. [1] | P |
| CVE-2018-20030 | libexif 0.6.21 | CPU exhaustion (CWE-400) | Crafted Exif and Interoperability IFD pointers. Published 2019-02-20; the advisory with the details could not be opened. [1] | P |
| CVE-2020-12767 | libexif 0.6.21 | Division by zero (CWE-369) | Formatting an entry's value. Published 2020-05-09. [1] | P |
| CVE-2020-13112, CVE-2020-13114 | libexif before 0.6.22 | Over-reads (CWE-125); unbounded work (CWE-770) | MakerNote handling; Canon MakerNote sizes not limited. Published 2020-05-21. [1] | P |
| CVE-2026-40386 | libexif 0.6.25 and earlier | Integer underflow (CWE-191) | In the size check of Fuji and Olympus MakerNotes. Published 2026-04-12. [1] | P |
| CVE-2012-0259, CVE-2012-1610 | ImageMagick before 6.7.6-3 and 6.7.6-4 | Over-read, integer overflow (CWE-125, CWE-190) | A component count of 0 in a resolution tag, and once fixed, a very large one. Published 2012-06-05. [1] | P |
| CVE-2017-11338, CVE-2018-18915, CVE-2021-37621 | Exiv2 0.26 to 0.27.4 | Endless loops (CWE-835) | Walking the IFD structure. Published 2017 to 2021. [1][5] | P |
| CVE-2018-5772, CVE-2019-9143 | Exiv2 0.26, 0.27 | Unbounded recursion (CWE-674) | Through nested IFDs. Published 2018-01-18 and 2019-02-25. [1] | P |
| CVE-2022-24614 | metadata-extractor (Java) before 2.18.0 | Huge allocation (CWE-770) | A tiny crafted JPEG forced a huge allocation. Published 2022-02-24. [1] | P |
| CVE-2026-35590 | libvips 8.18.1 and earlier | Null dereference | An IFD index was not range-checked before use. NVD's description names a null dereference, while its record assigns CWE-122, a heap overflow. Published 2026-07-20. [1] | P |

### 1.3 Thumbnails and previews

| ID | Component | Class | What went wrong | Conf. |
|---|---|---|---|---|
| CVE-2007-6352, CVE-2020-0181 | libexif, thumbnail loading | Integer overflow (CWE-189, CWE-190) | On the thumbnail's offset and length. Published 2007-12-20 and 2020-06-11. [1] | P |
| CVE-2018-11531, CVE-2018-12264, CVE-2018-12265 | Exiv2 0.26, previews | Integer overflow, out-of-bounds read and write (CWE-787, CWE-125, CWE-190) | A preview's offset plus its size overflowed, including the Exif JPEG thumbnail's. Published 2018. [1] | P |
| CVE-2026-27596 | Exiv2 0.28.7 and earlier, previews | Integer underflow, over-read (CWE-125, CWE-191) | A read about 4 GB out of bounds. Published 2026-03-02. [1][5] | P |
| CVE-2020-15503 | LibRaw 0.19.5 and earlier | Unchecked size (CWE-20) | A thumbnail's length used unchecked as an allocation size. Published 2020-07-02. [1] | P |

### 1.4 The Multi-Picture Format

No CVE or advisory was found for any parser of the Multi-Picture Format
(searches of NVD for "Multi-Picture", "MPO", "UltraHDR" and the ExifTool
records, GitHub's advisory database for "MPO", and Pillow's changelog).
[1][7] The nearest:

| ID | Component | Class | What went wrong | Conf. |
|---|---|---|---|---|
| zenjpeg issue #223 (no advisory) | zenjpeg, a Rust crate, MPF reading | Unchecked addition | The MP Index IFD's offset and the MP Entries' offsets were added to positions without overflow checks: a panic in a debug build on a 32-bit target, a wrapped position in a release build. Fixed with checked arithmetic, October 2026. [8] | S |
| Pillow 7.0.0 and 10.3.0 changelog | Pillow, MPO reading | Robustness | An MPO frame without data now raises an error; frame sizes are taken from the markers, not from the declared sizes. [7] | P |
| CVE-2021-27921, CVE-2021-27923 | Pillow before 8.1.1 or 8.1.2, BLP and ICO | Unchecked declared size (CWE-20) | The declared size of an embedded image was trusted, allowing a huge allocation: the same trap as an MP Entry's size. Published 2021-03-03. [1] | P |

### 1.5 The frame header and allocation

| ID | Component | Class | What went wrong | Conf. |
|---|---|---|---|---|
| CVE-2019-19035 | jhead 3.03, frame header | Over-read (CWE-125) | A frame header shorter than its fields. Published 2019-11-17. [1] | P |
| CVE-2012-2806 | libjpeg-turbo 1.2.0, scan header | Heap overflow (CWE-787) | A large component count. Published 2012-08-13. [1] | P |
| CVE-2015-1872 | FFmpeg before 2.5.4, frame header | Out-of-bounds access (CWE-119) | The component count not validated. Published 2015-07-26. [1] | P |
| CVE-2017-11140 | GraphicsMagick 1.3.26 | Memory exhaustion (CWE-400) | Memory sized from the header before any data was read. Published 2017-07-10. [1] | P |
| CVE-2026-112403 | Pillow before 13.0.0 | Heap overflow (CWE-122) | A scanline written into a buffer sized for a smaller scale, with a lossless frame. Published 2026-10-19. [1] | P |

### 1.6 Rust parsers

| ID | Component | Class | What went wrong | Conf. |
|---|---|---|---|---|
| RUSTSEC-2021-0143, CVE-2021-21235 | kamadak-exif 0.5.2, fixed in 0.5.3 | Endless loop (CWE-835) | An empty buffer was not checked when reading from a container. [9][1] | P |
| little_exif issue #105 (no advisory) | little_exif 0.6.23 | Unchecked subtraction | Found by fuzzing: 2 subtracted from a segment's length without a check, so a length of 0 or 1 panics in a debug build and wraps in a release build. Open in October 2026. [10] | S |
| GHSA-2v8p-fqpx-2q3w | jxl-modular 0.11.2 and earlier | Integer overflow (CWE-190) | A subtraction that panics when overflow checks are on: a denial of service in a Rust image decoder. Published 2026-05-29. [11] | P |

## 2. What the searches did not find

- **No record of an endless loop on `FF` fill bytes or on a segment that
  never advances**, although several endless loops in segment walks are
  recorded (section 1.1) with no cause stated. [I]
- **No record of a frame header with a zero width or height** as the
  cause of a bug. [I]
- **No advisory for jpeg-decoder, zune-jpeg, rexif or little_exif** in
  the RustSec database, and none for the `image` crate's JPEG or Exif
  handling. nom-exif and img-parts were not looked up. [9]
- **No vulnerability in a Multi-Picture Format parser** (section 1.4).

Each of these is a bug class to test for all the same: an absence of
records says nothing about the code.

## 3. Checklist and how raw-files applies it

Each item names the bug class, the rule that prevents it in raw-files,
and the records above that motivated it. The items of the
[NEF checklist](../nef/security.md#3-checklist-and-how-raw-files-applies-it)
apply unchanged to the TIFF structure inside the Exif segment; these are
the ones a JPEG adds.

1. **A segment's length is checked before it is used.** A length below 2
   is malformed, since it cannot count itself: never subtract before
   checking. Such a length, or a segment that runs past the end of the
   file, ends the walk: the file opens if a frame header came before it,
   and is refused with the `UnknownFormat` error otherwise. The position
   of the next marker is computed with checked arithmetic.
   (CVE-2004-0200, OW-002, CVE-2018-6612, CVE-2025-65092, little_exif
   #105.)
2. **The segment walk ends on any input.** It takes one step per marker
   or fill byte, always moves forward, stops at the first SOS, and gives
   up after a fixed number of steps, a crate-private constant, as the
   search for a preview's frame header does. The cap also bounds what
   `open` reads. A file with more fill bytes before its scan than the cap
   allows is therefore refused, although the JPEG standard allows any
   number of them; none of the eight sample files has a single one.
   (CVE-2021-37622, CVE-2021-37623, CVE-2022-25851.)
3. **The Exif segment is a bounded block with its own base.** The TIFF
   header 6 bytes into the payload is the base of every offset. Every
   IFD, entry table, value, thumbnail and MakerNote must lie inside the
   segment, checked against the segment's end, never only against the
   file. An offset and a size are compared with the bytes that remain, or
   added with `checked_add`, never added in a type that wraps. A pointer
   that leaves the segment, such as the ILCE-6000's MakerNote tag 0x2001
   ([container-and-metadata.md](container-and-metadata.md#10-makernotes-in-a-jpeg)),
   is dropped, not followed. (CVE-2019-9278, CVE-2020-13112.)
4. **The IFD walk is bounded as in a raw file.** The set of visited IFDs,
   the limits on IFDs and entries (`Limits::max_ifds`,
   `Limits::max_ifd_entries`) and checked entry-size arithmetic apply
   inside the Exif segment as in a NEF, and the Interoperability IFD is
   not followed. (CVE-2007-6351, CVE-2018-20030, CVE-2017-11338,
   CVE-2018-5772, CVE-2006-4168.)
5. **Damage costs the data it touches, never the file.** A bad entry
   costs its value; a TIFF header or IFD0 that cannot be read costs the
   metadata, the thumbnail and the MakerNote's preview; the file still
   opens, with its own preview and its MPF images. A component count of 0
   is a value of no use, not a reason to read.
   (CVE-2012-0259, CVE-2012-1610, CVE-2020-12767.)
6. **The thumbnail is checked like any preview.** JPEGInterchangeFormat
   and JPEGInterchangeFormatLength must describe a range inside the Exif
   segment that starts with SOI and holds, within that range, a frame
   header with no zero side that is not lossless. Neither number sizes an
   allocation. (CVE-2007-6352, CVE-2020-0181, CVE-2018-12265,
   CVE-2020-15503.)
7. **The MPF index is an IFD inside its own segment.** The MP header must
   lie inside the APP2 segment, the MP Index IFD under
   `Limits::max_ifd_entries`, and the MPEntry value inside the segment,
   its length a multiple of 16 bytes. NumberOfImages is not trusted: the
   entries read are those the value holds. Each image's offset is added
   to the MP header's position with `checked_add`; the image must lie
   inside the file and start with SOI, and its frame header is searched
   for within it only. The declared size is a bound, never an allocation.
   The first entry, at offset 0, is the file itself. (zenjpeg #223,
   CVE-2021-27921.)
8. **No allocation is sized from a tag or a frame header.** The frame
   header's width and height are reported, never multiplied into a
   buffer; a segment's length is at most 65,535 by its type; reading a
   preview, the whole file among them, stays under
   `Limits::max_preview_bytes`. (CVE-2022-24614, CVE-2017-11140,
   CVE-2026-112403.)
9. **The frame header is read with its length checked.** It must hold
   its length, the sample precision, the height and the width, a length
   of at least 7, as the search for a preview's frame header already
   requires; a width or height of 0 gives no size; a lossless marker
   (`C3`, `C7`, `CB`, `CF`) is not a preview, and on the file's own frame
   the file is refused. The component count is not used.
   (CVE-2019-19035, CVE-2012-2806, CVE-2015-1872.)
10. **No panics from arithmetic or slicing** on anything derived from
    the file: `checked_*` arithmetic and `get(..)`, as everywhere else
    ([ADR008](../../architecture-decisions/adr008-lints-and-panics.md)).
    (little_exif #105, GHSA-2v8p-fqpx-2q3w, zenjpeg #223.)
11. **Fuzzing.** The `open` fuzz target is to take JPEG seeds built from
    the fixtures, and a target of its own, `jpeg_segments`, is to fuzz the
    segment walk, the Exif block and the MPF index; the robustness tests
    are to cut and corrupt every JPEG fixture. Every crash found becomes a
    committed reproducer with a regression test shown to fail without its
    fix. (little_exif #105 was found so.)

## 4. Reporting

Report a security problem in raw-files privately, as described in
[SECURITY.md](../../../SECURITY.md), not in a public issue.

## Sources

[1] NVD, the record of each CVE named, https://nvd.nist.gov/vuln/detail/
    followed by the ID; read through the NVD API,
    https://services.nvd.nist.gov/rest/json/cves/2.0 **[P]**
[2] Microsoft, Security Bulletin MS04-028,
    https://learn.microsoft.com/en-us/security-updates/securitybulletins/2004/ms04-028
    **[P]**
[3] Openwall, "JPEG COM Marker Processing Vulnerability" (OW-002),
    https://www.openwall.com/articles/JPEG-COM-Marker-Vulnerability **[P]**
[4] Espressif, advisory GHSA-vcw6-jc3p-4gj8,
    https://github.com/espressif/esp-idf/security/advisories/GHSA-vcw6-jc3p-4gj8
    **[P]**
[5] Exiv2, security advisories, https://github.com/Exiv2/exiv2/security/advisories
    **[P]**
[6] libexif, issue #26, https://github.com/libexif/libexif/issues/26 **[S]**
[7] Pillow, CHANGES.rst,
    https://raw.githubusercontent.com/python-pillow/Pillow/main/CHANGES.rst
    (first half read), and GitHub's advisory search for "MPO",
    https://github.com/advisories?query=MPO **[P]**
[8] zenjpeg, issue #223, https://github.com/imazen/zenjpeg/issues/223
    **[S]**
[9] RustSec, RUSTSEC-2021-0143,
    https://rustsec.org/advisories/RUSTSEC-2021-0143.html, and the package
    pages https://rustsec.org/packages/kamadak-exif.html and
    https://rustsec.org/packages/image.html **[P]**
[10] little_exif, issue #105, https://github.com/TechnikTobi/little_exif/issues/105
     **[S]**
[11] GitHub advisory GHSA-2v8p-fqpx-2q3w,
     https://github.com/advisories/GHSA-2v8p-fqpx-2q3w **[P]**
