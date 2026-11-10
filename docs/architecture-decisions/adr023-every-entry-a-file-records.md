# ADR023: Every entry a file records

- Status: Accepted
- Builds on: [ADR009](adr009-public-api-shape.md), the shape of the
  public API; [ADR011](adr011-metadata-from-standard-tags.md), metadata
  from the standard tags;
  [ADR016](adr016-full-metadata-before-raw-data.md), full metadata
  before raw data;
  [ADR021](adr021-what-the-file-records-and-what-raw-files-reads-into-it.md),
  what the file records and what raw-files reads into it
- Date: 2026-11-03
- Issue: #50

## Context

raw-files reads a file's metadata into typed values: a capture time, a
lens, an exposure, the camera's settings
([ADR011](adr011-metadata-from-standard-tags.md)). Each value is a
reading of one or a few tags, chosen because a viewer needs it. Full
metadata ([ADR016](adr016-full-metadata-before-raw-data.md)) will add
many more, group by group. Beneath every one of them lies the same
thing: the entries of the file's TIFF directories, each a tag number, a
type, a count and a value. The NEF, ARW and camera JPEG files raw-files
reads are all built from such directories: IFD0 and the IFDs chained
after it, the SubIFDs that hold the raw data and the larger previews,
the Exif, GPS and Interoperability IFDs, the maker's MakerNote, and in
a JPEG the Multi-Picture Format's index.

raw-files already parses most of these directories to find the
previews and the metadata, and then drops every entry it does not
read. A program that wants an entry raw-files gives no value for, such
as a tag full metadata has not reached yet, a maker's tag no group
will ever read, or the recorded bytes to compare with another reader's
output, needs a second TIFF reader beside raw-files.
[ADR021](adr021-what-the-file-records-and-what-raw-files-reads-into-it.md)
gives each value as the file records it beside what raw-files reads
into it; this is the same idea one level down: every entry as the file
records it, beneath the values read from them.

Before deciding, we measured what real files hold, on 2026-11-03, with
a walker of the project's own, written from the TIFF 6.0 and Exif
standards and the project's format notes, and with a counting program
over raw-files' own `open`. The files were 18,464 NEF, ARW and camera
JPEGs: the test fixtures, every NEF and ARW-family file of the
public-domain [raw.pixls.us](https://raw.pixls.us/) catalogue, and, as
counts only, my own: every NEF of a Z 9 and a sample of five other Nikon
bodies' NEFs, an ILCE-7M2 and the JPEGs of Nikon, Sony, Canon and Fujifilm
bodies. They showed:

- **The directories are few and shallow.** A NEF holds IFD0, one to six
  SubIFDs, the Exif IFD, the Nikon MakerNote and its preview IFD, and on
  most files a GPS IFD; an ARW holds IFD0, IFD1, sometimes IFD2, one
  SubIFD, the Exif and Interoperability IFDs, the Sony MakerNote, the
  SR2Private IFD and sometimes a GPS IFD; a camera JPEG holds the Exif
  segment's IFD0, IFD1, Exif, Interoperability and GPS IFDs and
  MakerNote, and the Multi-Picture Format's index. No SubIFD holds
  another, and no directory lies more than three links from IFD0.
- **Two of them raw-files does not walk.** The GPS IFD, on every one of
  my Z 9 and Nikon DSLR files and most public NEFs, and Sony's
  SR2Private IFD, which IFD0's DNGPrivateData entry points to on 741 of
  the 745 ARW-family files. Behind SR2Private lies the SR2SubIFD, which
  is encrypted as a whole (11,932 to 62,112 bytes on 741 files).
- **A file holds about two hundred entries.** A NEF holds 228 (the
  median; at most 238), an ARW 202 (at most 281), a camera JPEG 104 (at
  most 163). The
  largest directory has 116 entries (a Sony MakerNote), far under the
  1,000 that `Limits` allows; only damaged directories hold more.
- **Their values are about 100 KB per file.** About 100 KB on a NEF, 74
  to 81 KB on an ARW, up to 38 KB on a JPEG. The largest is the Exif
  IFD's MakerNote entry, whose content is the MakerNote's own entries:
  up to 993,826 bytes on a Nikon 1 NEF and 697,564 on an ARW. Apart
  from it, the largest value is 66,596 bytes (a Coolpix NRW's MakerNote
  tag 0x0014), then Nikon's ShotInfo at 65,472 bytes and an XMP packet
  at 34,816. No value in any file passes 1 MiB, the cap raw-files
  already applies to the values it reads.
- **Reading them costs little more than opening does.** `open` reads
  320 KB of a Z 9 NEF today (the median), 192 KB of an older NEF, 128 to
  256 KB of an ARW and 128 to 384 KB of a JPEG. Every value of a JPEG
  and of all but two Z 9 NEFs lies inside what `open` already reads.
  Most older NEFs need one more 64 KiB block, for their GPS IFD's
  table; an ARW needs SR2Private's table, inside what `open` reads on
  my files and one more block on 89 of the 245 public ones, and, to
  read every value, its 36,864-byte tag 0x7250 too, 26 to 65 KB more
  (26 to 31 KB on my files) on 672 of the 745 files.
- **Damage is rare and local.** 17 public files have a damaged entry,
  most of them a value past the end of its MakerNote's stated length
  (Sony DSLR-A200 to A900, COOLSCAN scanners), and 9 a directory that
  cannot be read: the DSLR-A100's, a SubIFD of junk in one file opened
  as JPEG, and the MakerNote's preview IFD that four Coolpix NRWs and
  the two COOLSCANs put wholly past that length; of my own files,
  2 JPEGs have such a directory, and none has a damaged entry. No file
  has a
  count or an offset that overflows, a type TIFF does not define (but in
  one damaged file opened as JPEG), or a value past the end of the
  file.
- **Some entries are enciphered or encrypted.** Nikon's ShotInfo,
  ColorBalance and LensData are encrypted on all but a few old files,
  depending on each record's version (ColorBalance is plain on versions
  0100 to 0103); about a dozen of Sony's MakerNote records are
  enciphered on every body that writes them. They are plain
  UNDEFINED entries to a TIFF reader.
- **No file uses Exif's UTF-8 type.** Exif 3.0 added a type for text in
  UTF-8, code 129, which TIFF 6.0 does not define; Exif 3.1 (CIPA
  DC-008-2026, section 4.6.2) keeps it, NUL-terminated and without a
  byte order mark. No file of the 18,464 writes it, but a camera that
  follows Exif 3.0 may, in its text tags.

The format notes for [NEF](../formats/nef/makernote-and-metadata.md),
[ARW](../formats/arw/makernote-and-metadata.md) and
[JPEG](../formats/jpeg/container-and-metadata.md) list each format's
directories, entries and damage in full.

## Decision

We will give **every entry of every TIFF directory raw-files walks, as
the file records it**, through a type of its own reached from the
handle, with each value read only when it is asked for.

- **The entries are kept when the file is opened, their values read on
  request.** `open` keeps each entry it parses: its directory, tag,
  type, count and where its value lies. An entry's value is read only
  when a caller asks for it, through the handle, as a preview's bytes
  are ([ADR009](adr009-public-api-shape.md)), so opening a file reads
  only the GPS and SR2Private directories' tables more than before; an
  entry's value is checked against the file's length and its block,
  not read. The
  typed values do not read through this view: both are built from the
  same parsed directories.
- **A type of its own on the handle.** `RawFile::entries` gives the
  entries, in the file's order, directory by directory in the order
  the walk reaches them, with lookup by directory and tag; and
  `RawFile::read_value` reads one entry's value. `Metadata` stays the
  value type it is, built with setters. A caller cannot make an entry
  of its own, as it cannot make a `Preview`.
- **A damaged entry is left out and counted.** An entry of a type the
  view does not read (see the last point), whose length overflows, or whose value
  lies outside the file or outside the block its directory was read in
  is not listed; the view counts, per directory, how many it left out.
  Every listed entry is checked, so reading its value fails only on an
  I/O error, a value larger than the cap below, or a file that has
  shrunk since it was opened.
- **Values as recorded, text cleaned on request.** A value has one
  variant per type, as the file records it: an array of the type's
  numbers; a RATIONAL or SRATIONAL as numerator and denominator pairs,
  zero denominators kept; FLOAT and DOUBLE as floating-point numbers;
  ASCII and UTF-8 as their bytes. A caller can ask for the bytes of a
  BYTE, UNDEFINED, ASCII or UTF-8 value, and for its text cleaned as
  [ADR011](adr011-metadata-from-standard-tags.md) cleans every text
  value (cut at the first NUL, trimmed of spaces, bytes that are not
  UTF-8 replaced), without that record's 255-byte limit.
- **One cap bounds one value.** A new `Limits` cap, a number of bytes
  as the preview cap is, bounds the value one read may allocate, at
  1 MiB by default, the cap the typed values already use; a value over
  it fails only its own read, with `LimitExceeded`.
- **The reach is every TIFF directory raw-files reads or can read
  cheaply.** IFD0 and the IFDs chained after it, the SubIFDs, the Exif,
  GPS and Interoperability IFDs; the Nikon and Sony MakerNotes in the
  layouts raw-files reads, Nikon's preview IFD (MakerNote tag 0x0011)
  and Sony's SR2Private IFD; in a JPEG, the Exif segment's directories
  the same way, and the Multi-Picture Format's index IFD. The GPS IFD is
  walked after IFD0's chain, with the Interoperability IFD, so that an
  IFD a file points to twice keeps its place in the chain. A binary
  record inside a MakerNote is one UNDEFINED or BYTE entry whose bytes
  the view gives.
- **Directories are named by what led the walk to them.** IFD0 and the
  IFDs after it by their place in the chain, SubIFDs numbered in the
  order the walk finds them, and Exif, GPS, Interoperability,
  MakerNote, a MakerNote's own IFD by the tag that points to it,
  SR2Private and the Multi-Picture index each by its kind. The
  MakerNote is the camera maker's, whom the file's camera names.
- **Names come from the standards only.** An entry has a name where a
  published standard gives its tag one in that kind of directory: TIFF
  6.0, Exif, DCF for the Interoperability IFD, CIPA DC-007 for the
  Multi-Picture index, and the DNG specification for the DNG tags
  raw-files reads. The names, each with its document and section, are
  recorded in [the tag names](../formats/tag-names.md) before any code
  holds them ([ADR013](adr013-clean-room-and-third-party-code.md)). A
  MakerNote tag, and any tag no such document names, has none.
- **As recorded, nothing redacted.** The view gives serial numbers, GPS
  entries and the owner's, artist's and copyright texts as the file
  records them, with no option to hide them, as full metadata gives
  every value ([ADR016](adr016-full-metadata-before-raw-data.md)).
  Tests on my own files print counts only, never a value.
- **Enciphered and encrypted records are bytes, not marked.** Such a
  record is a plain UNDEFINED or BYTE entry whose bytes, as recorded,
  the view gives; the entry does not say it is enciphered. Whether a
  record is enciphered depends on its maker and, for Nikon, on a
  version written inside it, so marking it is already reading it: that
  belongs with the decision on reading enciphered blocks that
  [ADR016](adr016-full-metadata-before-raw-data.md) calls for. The
  format notes say which records are enciphered, by maker and record
  version.
- **Exif's UTF-8 type is read.** The view reads TIFF 6.0's twelve
  types; the IFD type, 13, which no document read defines but raw-files
  already accepts for the SubIFDs tag; and Exif's UTF-8, 129. An entry
  of type 129 is listed as UTF-8, and its bytes and cleaned text are
  given as an ASCII value's are.

Rejected:

- **Every value read when the file is opened.** It would keep about
  100 KB per NEF and 75 KB per ARW in memory for values few callers
  read, and make most ARWs read 26 to 65 KB more when they are opened,
  for SR2Private's tag 0x7250.
- **The directories walked again on request.** Opening a file already
  parses them; a second walk would read them twice and keep two
  walkers in step.
- **The view on `Metadata`.** `Metadata` is a value a caller can build
  and compare ([ADR011](adr011-metadata-from-standard-tags.md)); entries
  read from the file on request belong with the handle, as previews do.
- **The entries flat on `RawFile`**, a method per lookup. Iteration,
  lookup and the counts of entries left out are one thing, which a type
  holds better than a handful of methods.
- **Damaged entries listed and marked.** Every caller would have to
  check each entry before reading it, for entries on 22 files out of
  18,464. A listing of the entries left out can still be added.
- **Text cleaned only, or bytes only.** Cleaned text loses what the
  file records, which a program comparing with another reader needs;
  bytes alone make every caller clean text itself.
- **A smaller cap, 256 KiB or 128 KiB.** Both are above every value
  measured but the MakerNote entry, which reaches 993,826 bytes on a
  Nikon 1 NEF and 697,564 on an ARW and whose bytes a caller could then
  not read; and a second number beside the typed values' 1 MiB would
  make a value readable one way and not the other.
- **The reach without SR2Private**, or with a JPEG's XMP and IPTC
  segments as bytes. SR2Private holds the plain entries that lead to
  the SR2SubIFD; the XMP and IPTC segments are not TIFF entries, and
  full metadata gives them a group of their own.
- **SubIFDs named by their parent and slot**, or a MakerNote variant
  per maker. No measured SubIFD holds another, so a number in walk
  order says as much; the camera already names the maker.
- **No names yet.** A program would keep its own table of the standard
  names beside raw-files, the second reader this view is meant to
  remove.
- **A redaction option.** raw-files gives what the file records; a
  program that hides a serial number or a position decides what to
  hide.
- **Enciphered records marked.** See above: the mark needs the record's
  version read, which is reading the record.
- **The UTF-8 type left until a file writes it.** A file that follows
  Exif 3.0 would lose its text entries as damaged, and the type costs
  one variant.

## What this decision does not cover

- Deciphering or decrypting a record, and so marking one as enciphered:
  that waits for the decision on reading enciphered and encrypted
  blocks, with which [ADR011](adr011-metadata-from-standard-tags.md) is
  revised.
- Parsing XMP and IPTC, and the JPEG segments that are not TIFF
  directories (XMP, IPTC, ICC profiles, FlashPix extensions): full
  metadata's group for XMP and IPTC.
- Other makers' MakerNotes, such as Canon's and Fujifilm's: the Exif
  IFD's MakerNote entry gives their bytes, and nothing walks them until
  the format notes describe them.
- Names for MakerNote tags. A maker publishes none, and a name only
  another reader's tables give is not taken.
- Nikon's short-header MakerNote layout, of the earliest Coolpix
  bodies, which raw-files does not read.
- Sony's SR2SubIFD, which is encrypted as a whole, and the MakerNote
  chain of SRF files, which needs a key read from an enciphered record.
- Which typed values accept a UTF-8 entry beside an ASCII one:
  [ADR011](adr011-metadata-from-standard-tags.md) decides it, text by
  text, as Exif 3.1 gives each.

## Consequences

- A program reads any entry raw-files walks without a second TIFF
  reader, and compares its bytes with another reader's output.
- Opening a file reads a little more: the GPS IFD's table, one more
  64 KiB block on most older NEFs, and SR2Private's table, one more
  block on some ARWs.
- The handle keeps every entry for as long as it lives: about two
  hundred on a camera file. For a hostile one, the default `Limits`
  allow 1,000 entries in each directory, and 256 directories in the
  tree walked from IFD0; the few read beside that tree, such as the
  MakerNote, its own IFDs, SR2Private and the Multi-Picture index, add
  at most 1,000 entries each.
- A value is read from the file each time it is asked for; a caller
  that reads one often keeps its own copy.
- A damaged entry is invisible but for its directory's count, so a
  caller cannot tell why an entry is missing.
- An enciphered record looks like any other binary entry; a caller
  reads which ones are enciphered in the format notes, not in the API.
- The names are those of the standards, which differ from other
  readers' names for the same tags in places, and most MakerNote
  entries have none.
- Every new walk raw-files adds, such as another maker's MakerNote,
  adds its entries to the view, and the directory type grows by a
  variant, which does not break callers.
