# NEF: security notes

A NEF file is untrusted input, and camera RAW parsers have a long record of
memory-safety and denial-of-service bugs. This page lists the known
vulnerabilities in NEF and Nikon handling, the adjacent ones grouped by bug
class, and a checklist that maps each class to the raw-files rules. Back to
[the NEF overview](README.md).

Confidence tags: **[P]** primary (read in the NVD record, the project's
changelog or source), **[S]** secondary (distribution trackers, summaries),
**[I]** inferred. Numbers in brackets point to the [Sources](#sources) at the
end, retrieved in October 2026.

The raw-files baseline is [ADR008](../../architecture-decisions/adr008-lints-and-panics.md):
no `unsafe` code, and malformed input returns an error, never a panic. The
bugs below are mostly the kind Rust prevents (buffer overflows); what Rust
does not prevent is a panic from an out-of-range index, an overflow in debug
builds, a divide by zero, an unbounded allocation or an endless loop. Those
are our bugs to avoid.

## 1. NEF and Nikon-specific vulnerabilities

| ID | Component | Class | Notes | Conf. |
|---|---|---|---|---|
| CVE-2026-5342 | LibRaw `nikon_load_padded_packed_raw` (TIFF/NEF), up to 0.22.0 | Out-of-bounds read (CWE-119, 125) from attacker-controlled `load_flags` and `raw_width` | Published 2026-04-02; fixed in 0.22.1 (commit b8397cd, issue #795); public proof of concept; CVSS 5.3 (v3.1, as scored by VulDB). This is the padded 12-bit loader. [1][2] | P |
| CVE-2024-8025 | Nikon NEF Codec 1.31.2 for Windows, thumbnail provider parsing NRW | Heap overflow (CWE-787): a copy length is not validated | Published 2024-11-22; NVD 7.8; ZDI-CAN-19873. Directly relevant: it is a preview and thumbnail path. [3] | P |
| CVE-2020-15503 | LibRaw thumbnail handling | Missing range check on the thumbnail size | Directly relevant to preview extraction. [4] | P |
| CVE-2008-2696 | Exiv2 0.16, Nikon lens information | Divide by zero | Nikon MakerNote. [6] | P |
| CVE-2020-18771 | Exiv2 0.27.99 `Nikon1MakerNote::print0x0088` | Global buffer over-read | Nikon MakerNote. [6] | P |

Two further LibRaw identifiers (CVE-2018-5811 and CVE-2018-5812) appeared in
a Nikon-related listing but were not opened, so we do not cite them.

### 1.1 Other LibRaw issues in 2026

LibRaw reported a group of problems in April 2026, six of them with CVEs. Only
the first is Nikon-specific; the others are listed because they sit in code
shared with NEF decoding (lossless JPEG, the JPEG header parser) or show how
the same bug classes recur. [1][2][7]

| CVE | Function | Class | Published | Fix status |
|---|---|---|---|---|
| CVE-2026-5342 | `nikon_load_padded_packed_raw` | Out-of-bounds read | 2026-04-02 | Fixed in 0.22.1 (b8397cd) |
| CVE-2026-5318 | `HuffTable::initval` (JPEG DHT parser, `losslessjpeg.cpp`) | Out-of-bounds write (CWE-119, 787), triggered by the `bits[]` counts | 2026-04-02 | Fixed in 0.22.1 (a6734e8, issue #794) |
| CVE-2026-20884 | `deflate_dng_load_raw` (TALOS-2026-2364) | Integer overflow leading to heap overflow (CWE-190) | 2026-04-07 | See note |
| CVE-2026-21413 | `lossless_jpeg_load_raw` (TALOS-2026-2331) | Heap overflow | 2026-04-07 | See note |
| CVE-2026-20889 | `x3f_thumb_loader` (TALOS-2026-2358) | Heap overflow (CWE-190, 120) | 2026-04-07 | See note |
| CVE-2026-24450 | `uncompressed_fp_dng_load_raw` (TALOS-2026-2363) | Integer overflow leading to heap overflow (CWE-190) | 2026-04-07 | See note |

**Note on the four Talos-reported entries.** LibRaw's 0.22.1 changelog
(released 2026-04-06) lists the matching Talos identifiers as fixed. The
National Vulnerability Database nonetheless lists 0.22.1 as an affected
version for CVE-2026-20884, CVE-2026-21413 and CVE-2026-24450, and 0.22.0 for
CVE-2026-20889, and does not state a fixed version. [2][7] **[P]** The two
sources disagree. Our wording is therefore: fixed upstream in 0.22.1 per
LibRaw's changelog; the NVD record disagrees; LibRaw 0.22.2 (2026-07-16,
bug fixes for input validation, recursion limits and memory zeroing) is the
conservative reference. We do not claim these CVEs are fixed in any
particular release.

LibRaw 0.22.0 is dated 2025-12-25 in LibRaw's Changelog and 2026-01-13 on its
GitHub release page. The 0.22.0 tag contains commits from as late as
2026-01-09, so the Changelog date precedes the code it describes; this page
uses 2026-01-13. [7]

## 2. Adjacent vulnerabilities by bug class

These are not NEF-specific but are the same shape as bugs a NEF parser can
have. Identifiers are from the NVD keyword listings for each project and,
where a component or class is named, from the NVD records. [5][6][8][13]
**[P]** unless marked.

### 2.1 Offsets, lengths and bounds

| Project | IDs | Component |
|---|---|---|
| LibRaw | CVE-2017-6886, CVE-2017-6887 | `parse_tiff_ifd` |
| LibRaw | CVE-2018-5809, CVE-2020-15365 | `parse_exif` (the latter an out-of-bounds write) |
| LibRaw | CVE-2020-24890 | NULL dereference |
| LibRaw | CVE-2018-20337 | Stack buffer overflow in `parse_makernote` (0.19.1); the crashing line is in its Fujifilm branch, not a Nikon path **[S]** |
| dcraw | CVE-2018-19566 | Heap over-read in `parse_tiff_ifd` |
| libexif | CVE-2020-13112 | MakerNote over-reads |
| libexif | CVE-2020-13114 | Canon MakerNote, unrestricted size |
| libexif | CVE-2017-7544 | Save routine |
| Exiv2 | CVE-2025-26623 | Heap overflow in the metadata write path (0.28.0 to 0.28.4) **[S]** |
| Exiv2 | CVE-2017-9953 | `printIFDStructure`, invalid free |
| Exiv2 | CVE-2017-11336 | `printIFDStructure`, heap over-read |
| Exiv2 | CVE-2017-11339 | `printIFDStructure`, heap overflow |
| Exiv2 | CVE-2017-12955 | `printIFDStructure`, out-of-bounds write |
| Exiv2 | CVE-2017-14863 | `printIFDStructure`, NULL dereference |

### 2.2 IFD loops, recursion and runaway work

| Project | IDs | Component |
|---|---|---|
| libexif | CVE-2007-6351 | Infinite recursion |
| libexif | CVE-2018-20030 | CPU exhaustion through IFD loops |
| Exiv2 | CVE-2017-11338 | `printIFDStructure`, infinite loop (its siblings in the same function are in section 2.1) |
| Exiv2 | CVE-2017-14861 | Stack consumption through recursion |
| Exiv2 | CVE-2017-11683 | TiffReader assertion |

### 2.3 Integer overflow and oversized allocation

| Project | IDs | Component |
|---|---|---|
| dcraw | CVE-2015-3885 | Integer overflow in `ljpeg_start` |
| dcraw | CVE-2021-3624 | Integer overflow in `foveon_load_camf` |
| dcraw-derived code | CVE-2026-24808, CVE-2026-4731 | Integer overflows in RawTherapee and ART |
| LibRaw | CVE-2018-5815, CVE-2018-5816 | Integer overflows (from a summary **[S]**) |
| LibRaw | CVE-2020-35534 | Excessive allocation (CWE-770) |
| RawSpeed (librawspeed) 3.1 | CVE-2018-25017 | Heap overflow in `TableLookUp::setTable`; relevant because `TableLookUp` is rawspeed's NEF curve lookup |
| libexif | CVE-2007-2645, CVE-2007-6352 | Entry and thumbnail loading |
| libexif | CVE-2016-6328 | MakerNote entry data |
| libexif | CVE-2019-9278 | Integer overflow (Android) |
| Exiv2 | CVE-2007-6353 | `exif.cpp` |
| Exiv2 | CVE-2018-4868 | Excessive allocation |
| Exiv2 | CVE-2017-1000127 | TIFF parser heap overflow |

### 2.4 Huffman and entropy decoding

| Project | IDs | Component |
|---|---|---|
| LibRaw | CVE-2026-5318 | `HuffTable::initval`, out-of-bounds write |
| LibRaw-demosaic-pack-GPL2 (a separate GPL add-on), before 0.18.2 | CVE-2017-6890 | `foveon_load_camf()`, Foveon code, not NEF |
| LibRaw (Sigma X3F code) | CVE-2020-35531 | `get_huffman_diff()` in `x3f_utils_patched.cpp`, out-of-bounds read; X3F, not NEF |

The two Foveon/X3F entries are adjacent examples only: they are not NEF bugs.

### 2.5 Division and modulo by zero

| Project | IDs | Component |
|---|---|---|
| Exiv2 | CVE-2008-2696 | Nikon lens information |
| dcraw | CVE-2018-19567 | Floating-point exception in `parse_tiff_ifd` |
| libexif | CVE-2012-2837 | Olympus MakerNote |

### 2.6 Scripting and process-spawning

ExifTool's CVEs (CVE-2021-22204, CVE-2022-23935, CVE-2026-3102,
CVE-2026-7580) are command-injection and evaluation bugs, not memory
safety. [9] **[P]** They are not relevant to a Rust parser except as a
reminder never to evaluate file contents or build shell commands from them.
Programs that hand NEF files to an external dcraw or similar binary carry
that risk too; raw-files does neither.

### 2.7 Not verified

Windows camera-codec CVEs (for example CVE-2020-16967, CVE-2020-16968 and
CVE-2020-0997) were found, but their snippets did not tie them to NEF, so
they are not included. No Apple ImageIO NEF CVE was found, but our search
there and for Android was shallow. rawler and rawloader have no advisories
that we found, and rawler's README warns against using it on untrusted input
where panics are unacceptable, which is the gap raw-files aims to close. [10]
**[S]**

## 3. Checklist and how raw-files applies it

Each item names the bug class, the raw-files rule that prevents it, and the
CVEs that motivated it. Items marked "when relevant" apply once the milestone
that introduces the feature lands (see [the roadmap](../../ROADMAP.md)).

1. **Bound every offset and length from the file.** Check
   `offset + len <= available` with `checked_add`, against both the file and
   the enclosing sub-slice. Use `get(..)` and checked slicing, never plain
   indexing, on data derived from the file. A MakerNote is a bounded
   sub-slice with the base its layout sets: its inner TIFF header in the
   embedded layout, never the outer base, and the outer TIFF header in
   the headerless layout, whose offsets count from there.
   Applies to IFD offsets, entry value offsets, strip offsets and byte
   counts, SubIFD pointers, and JPEG preview offsets and lengths.
   (CVE-2020-15503, CVE-2024-8025, CVE-2017-7544, CVE-2018-20337,
   CVE-2020-13112.)
2. **Track visited offsets and cap depth.** Keep a set of visited IFD
   offsets, a depth cap (about five) and caps on the total IFDs and entries
   (LibRaw caps MakerNote entries at 1000). This covers SubIFDs, the Exif
   IFD, the MakerNote IFD, the Interoperability IFD and next-IFD chains.
   Parsing must terminate on any input. (CVE-2017-11338, CVE-2018-20030,
   CVE-2007-6351.)
3. **Check entry size arithmetic.** Compute `count * type_size` with
   `checked_mul`, cap `count`, and honour the rule that values of four bytes
   or fewer are stored inline. (CVE-2007-2645, CVE-2016-6328.)
4. **Skip bad entries; do not fail the file.** A single malformed entry is
   skipped, so a damaged or editor-relocated MakerNote costs metadata, not
   the preview.
5. **Bound allocations by the bytes present.** Never size a buffer from
   width times height alone, and never call `Vec::with_capacity` with a value
   from the file. Compute width times height times bytes per sample with
   checked arithmetic, cap by a configurable maximum pixel count, and for
   uncompressed data also by the bytes actually in the strip.
   (CVE-2015-3885, CVE-2020-35534, CVE-2018-4868.)
6. **Make Huffman decoding total (when relevant).** The six Nikon tables are
   fixed in the decoder, not read from the file, so there is no table-parsing
   attack surface of the DHT kind; the risk is in decoding. Guard the length
   `len` and shift `shl` fields of every decoded symbol (reject `shl > len`,
   `len > 15`, and handle `len == 0`), bounds-check the predictor value
   before indexing the curve, bounds-check the output row, and treat a
   bitstream that ends early as an error or a flagged zero-fill, never a
   panic. A hard-coded table still needs a test proving it is a complete,
   valid prefix code. (Adjacent examples, none in NEF: CVE-2026-5318,
   and the Foveon/X3F bugs CVE-2017-6890 and CVE-2020-35531.)
7. **Validate strides against the strip size (when relevant).** For packed
   and padded layouts compute the stride from validated tags, require
   `rows * stride <= strip bytes` per strip, and honour each strip's own
   offset. Derive the layout from several agreeing facts rather than one
   tag. (CVE-2026-5342, in the padded 12-bit loader, was this bug.)
8. **Bounds-check the 0x0096 block field by field (when relevant).** Header
   bytes, the 2110-byte skip, the predictor seeds, the curve size, the curve
   points, and the split row at offset 562 each need their own check. Reject
   a curve whose anchors do not tile the range, and ignore a split row at or
   past the image height.
9. **Reject zero divisors.** Rational denominators, strip heights, tile
   sizes, crop sizes and the curve step are all checked before dividing.
   (CVE-2008-2696, CVE-2018-19567.)
10. **Fail soft on decryption (when relevant).** If the serial number or
    shutter count is missing or unusable, skip decrypting and carry on. Check
    the decrypted block's version and length against minimums before reading
    fields from it, and treat decrypted values as plausible-or-ignored, since
    a wrong key produces garbage without any error. Index into the
    substitution tables with values that cannot go out of range.
11. **Gate HE and HE\* before any decoding (when relevant).** Detect the
    High Efficiency formats by the first bytes of the raw strip (the two
    JPEG XS markers `FF 10 FF 50`) and by the compression value, and return
    a typed unsupported error. Never fall through to the Huffman path.
    LibRaw performs this check only on a handful of models [11] **[P]**, and
    decodes noise on other bodies [12] **[S]**. raw-files runs the check on every file. See
    [licences-and-references.md](licences-and-references.md) for why HE is
    out of scope.
12. **Treat the embedded JPEG as opaque.** For previews, check that the
    declared range holds a start-of-image marker and walk markers with length
    checks to the first frame header, inside the range only. Do not trust
    the preview's own dimension tags (often absent or zero). Return the bytes
    unmodified; decoding them is out of scope until a decoder is chosen.
13. **No panics from arithmetic.** Release builds wrap on overflow while
    debug builds panic, so a bug can hide until a test. Use `checked_*` and
    `saturating_*` on everything derived from file data, and use `wrapping_*`
    only where the format specifies wraparound (the predictor).
14. **Lints and fuzzing.** ADR008 deliberately does not enable
    `clippy::indexing_slicing`, because it flags every bounds-checked slice.
    Enabling `clippy::arithmetic_side_effects` would be a lint-policy change
    needing an ADR that amends ADR008, which keeps lints in `Cargo.toml` and
    rejected `indexing_slicing` for noise. Until then the no-panic rule
    rests on the following: tests on truncated and corrupted input (see
    [test-data.md](test-data.md)), and `cargo-fuzz` targets for the container
    parse, the MakerNote parse, preview extraction and, later, the decoder.
    Seed them from our tiny fixtures, and keep every crash reproducer as a
    committed small file with a regression test shown to fail without the
    fix.

## 4. Reporting

Report a security problem in raw-files privately, as described in
[SECURITY.md](../../../SECURITY.md), not in a public issue.

## Sources

[1] NVD, CVE-2026-5342 —
    https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=CVE-2026-5342 ;
    also https://cveawg.mitre.org/api/cve/CVE-2026-5342 **[P]**
[2] NVD records for CVE-2026-5318, CVE-2026-20884, CVE-2026-21413,
    CVE-2026-20889 and CVE-2026-24450 —
    https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=<ID> **[P]**
[3] NVD, CVE-2024-8025 —
    https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=CVE-2024-8025
    **[P]**
[4] NVD keyword listing "libraw" (CVE-2020-15503 and the LibRaw entries in
    section 2) —
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=libraw
    **[P]**
[5] NVD keyword listings "dcraw", "libraw" and "libexif" (CVE-2018-20337,
    section 2) —
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=dcraw ,
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=libexif
    **[P]**
[6] NVD keyword listings "exiv2" and "exiv2 nikon" —
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=exiv2 ,
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=exiv2%20nikon
    **[P]** (first 34 of 125 Exiv2 entries read)
[7] LibRaw Changelog at tag 0.22.2 and GitHub releases —
    https://raw.githubusercontent.com/LibRaw/LibRaw/0.22.2/Changelog.txt ,
    https://api.github.com/repos/LibRaw/LibRaw/releases ; distribution
    trackers https://security.archlinux.org/package/libraw ,
    https://tracker.debian.org/pkg/src:libraw **[P]** (changelog, releases),
    **[S]** (trackers)
[8] NVD keyword listing "libexif" —
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=libexif
    **[P]**
[9] NVD keyword listing "exiftool" —
    https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=exiftool
    **[P]**
[10] dnglab README —
    https://raw.githubusercontent.com/dnglab/dnglab/main/README.md **[P]**;
    Windows, Apple and Android searches **[S]**
[11] LibRaw source, TIFF tag handling (HE detection) —
    https://raw.githubusercontent.com/LibRaw/LibRaw/master/src/metadata/tiff.cpp
    **[P]**
[12] rawmakase PR #412 (LibRaw output on an ungated Z5 II) —
    https://github.com/pch/rawmakase/pull/412 **[S]**
[13] NVD records for CVE-2017-9953, CVE-2017-11336, CVE-2017-11338,
    CVE-2017-11339, CVE-2017-12955, CVE-2017-14863, CVE-2017-6890,
    CVE-2018-25017 and CVE-2020-35531 —
    https://nvd.nist.gov/vuln/detail/<ID> **[P]**
