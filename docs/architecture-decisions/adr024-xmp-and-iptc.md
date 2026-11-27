# ADR024: XMP and IPTC

- Status: Accepted
- Builds on: [ADR011](adr011-metadata-from-standard-tags.md), metadata
  from the standard tags;
  [ADR016](adr016-full-metadata-before-raw-data.md), full metadata
  before raw data; [ADR014](adr014-jpeg-files-as-a-format-of-their-own.md),
  JPEG files as a format of their own;
  [ADR023](adr023-every-entry-a-file-records.md), every entry a file
  records
- Date: 2026-11-27
- Issue: #237

## Context

A photographer, or the program they use, adds what the camera cannot
know. That is a rating, a color label, keywords, a title, a
description, who made the picture and on what terms. A culler marks
keepers and rejects. A catalog sorts and searches by all of these.

Two standards hold these values:

- **XMP**, the Extensible Metadata Platform, is XML. Its data model is
  XMP Part 1, which is ISO 16684-1. A packet is a set of properties,
  each named by a namespace URI and a local name. A value is simple, a
  structure of named fields, or an array, and can carry qualifiers such
  as its language.
- **IIM**, the IPTC's Information Interchange Model, is older and
  binary. Its block is a list of datasets, each a record number, a
  dataset number and bytes.

The IPTC Photo Metadata Standard 2025.1 defines 25 Core properties.
Each is an XMP property, and most also name the IIM dataset that
holds the same value (§7). A picture can hold both forms, written by
different programs.

The files raw-files reads hold them in a few places:

- **In a NEF or ARW file**, IFD0's tag 0x02BC (700) holds an XMP packet,
  of type UNDEFINED or BYTE. Tag 0x83BB (33723) holds an IIM block. Tag
  0x8649 holds Photoshop's image resources, which can repeat it (XMP
  Part 3, Tables 12 and 13, p. 19).
- **In a JPEG file**, an APP1 segment whose payload starts with the XMP
  namespace URI and a NUL holds the packet. A packet over 65,502 bytes
  continues as Extended XMP in further APP1 segments. An APP13 segment
  that starts with `Photoshop 3.0` holds Photoshop's image resources,
  and resource 1028 is the IIM block (XMP Part 3, Table 5, p. 13;
  §1.1.3.1, p. 14; Table 36, p. 68).
- **Beside the file**, a sidecar is an XMP document of its own. XMP
  Part 3 names it after the file, with the extension `.xmp` in place of
  the file's own (p. 7). darktable names it `<name>.<ext>.xmp`, the
  file's whole name and `.xmp`.

We measured what files hold on 2026-11-27, with a counting program
written for it. It read raw-files' entries, and walked each JPEG's segments and
each IIM block itself. It parsed the packets with an XML library that
decides nothing for raw-files. The files were 507 public NEF, ARW and JPEG
files of [raw.pixls.us](https://raw.pixls.us/), the 149 test fixtures,
and, as counts only, 166,339 of my own. They showed:

- **Every packet lies where the standard puts it.** 204 public NEF and
  130 public ARW files hold one in IFD0, and so do 29,775 of my NEFs.
  25,377 of my JPEGs hold an APP1 packet. A scan of every byte of the
  public raw files found no packet anywhere else.
- **Every packet lies inside what opening a file reads.** `open`
  follows IFD0, and two edited public ARW files hold IFD0 itself, and
  the packet after it, in the file's last kilobyte. No file holds
  Extended XMP, and no preview inside a raw file holds a packet of its
  own.
- **Cameras write little.** Nikon bodies write a rating of 0, the body
  and firmware, the creation time and, on the Z bodies, Adobe Camera
  Raw's default settings. Sony bodies write a rating of 0. Some older
  bodies write an empty packet or none.
- **Editors write the rest.** Lightroom, GIMP, Microsoft's photo
  viewer, phones' camera apps and darktable write ratings, creators,
  copyright notices, descriptions and namespaces of their own. All
  10,256 sidecars on my disks are darktable's, named
  `<name>.<ext>.xmp`; none is named the way XMP Part 3 says.
- **The XML is plain.** No packet uses a DOCTYPE, a CDATA section or a
  character reference. 110 packets of Sony Ericsson phones use a prefix
  they never declare, which Namespaces in XML does not allow; every
  other packet is well-formed. The Nikon 1 bodies bind the XMP
  namespace to the prefix `xap`, the rest to `xmp`.
- **IIM is an editor's.** 895 of my JPEGs and one public ARW hold an
  IIM block. Where the writer was checked, it is an editor: Lightroom,
  GIMP or Capture One. 422 blocks declare UTF-8; the rest declare
  nothing.
- **A segment can repeat.** 11 of my JPEGs hold two Photoshop segments,
  each with an IIM block of its own, and 6 hold two XMP segments.
- **Exif and XMP agree.** Wherever both record the creator, the
  copyright notice or the description, the two texts are equal.
- **The values are simple.** Every rating is 0, 1, 2, 3 or 5: none is
  -1, a fraction or out of range. Every date created has a time to the
  second. A packet holds 357 values at most, nested 22 elements deep
  at most.

[The XMP and IPTC notes](../formats/xmp-and-iptc.md) give every count.

XMP Part 1 leaves out how to choose between two packets about one
file (§6.1, p. 4). The Metadata Working Group's guidelines give a
reader a rule for a value that Exif, XMP and IIM all record (§4.2.3,
pp. 23 to 30):

- Exif over XMP, for the date, the description, the copyright notice
  and the creator;
- XMP over IIM, unless IIM's MD5 digest, Photoshop's resource 1061,
  shows that a program unaware of XMP changed the IIM block.

raw-files has no XML reader, and no dependency. One it took would have
to be pure Rust ([ADR007](adr007-pure-rust-cross-platform.md)) under a
permissive license ([ADR002](adr002-license-mit-or-apache-2.md)).

## Decision

We will read **the XMP packet and the IIM block a file embeds, and an
XMP sidecar a caller hands in**. An XML reader of raw-files' own
parses the XML. Every XMP property is given in the standard's data
model, and the rating, the label and IPTC Core are typed values read
from it.

- **An XML reader of raw-files' own.** It is written from XML 1.0,
  Namespaces in XML 1.0 and XMP Part 1 §7, for the part of XML that XMP
  uses. It reads:
  - UTF-8 only, a leading byte order mark skipped;
  - elements, attributes and text, with namespaces resolved by URI;
  - the five predefined entities, character references and CDATA.

  Comments and processing instructions are skipped, the packet wrapper
  among them. The XML declaration is checked. Any of these is damage
  that costs the packet:
  - bytes that are not UTF-8, or UTF-16 and UTF-32;
  - an XML declaration that names another encoding or version 1.1;
  - a DOCTYPE, or an entity that is not predefined;
  - an undeclared prefix, or a name with two colons;
  - a duplicate attribute, or `<` in an attribute value;
  - a character XML forbids;
  - a mismatched or missing end tag, or anything outside the root.

  The reader is iterative, never recursive, and its depth is bounded
  by `Limits`. The fuzz target covers it.
- **A sidecar is handed in; raw-files never looks for one.** A caller
  names it, by its path (`Xmp::open`), its bytes (`Xmp::from_bytes`) or
  a reader (`Xmp::from_reader`). `Options` gives the same three under
  other limits. The read is bounded by `Limits::max_value_bytes`.
  Opening a camera file reads that file alone. raw-files chooses
  nothing between two sidecars, or between a sidecar and the embedded
  packet. The documentation shows a lookup by each of the two names,
  XMP Part 3's and darktable's.
- **The embedded packet is read when the file is opened.** It is IFD0's
  0x02BC, of type BYTE or UNDEFINED, in a NEF or ARW file, and the
  first APP1 XMP segment in a JPEG file. It is given as
  `Metadata::xmp`, wherever in the file it lies. A packet past what
  `open` already read costs one more read, bounded by
  `Limits::max_value_bytes`.
- **Every property, in XMP Part 1's data model** (§6). A property is
  named by its namespace URI and local name, never by its prefix. A
  value is simple text, a URI, a structure of named fields or an array,
  unordered, ordered or alternative (Bag, Seq or Alt). Every qualifier
  is kept, `xml:lang` among them. A caller looks a property up by
  namespace URI and name, or iterates over all. raw-files keeps no
  table of property names: the model is the standard's, and the names
  are each file's.
- **The typed values are the rating, the label and IPTC Core.** They
  are `xmp:Rating` and `xmp:Label` (XMP Part 1 §8.4), and the 25 Core
  properties of the IPTC Photo Metadata Standard 2025.1, under IPTC's
  own names. IPTC Core is read whole, as
  [ADR011](adr011-metadata-from-standard-tags.md) reads a documented
  list that the files hold part of. The IPTC Extension, the PLUS
  structures and any other namespace are left to the property list.
- **Each source is given apart, as recorded.** `Metadata::authorship`
  keeps Exif's texts unchanged. The XMP typed values give XMP's own:
  the creators as a list, the texts with their languages. Nothing is
  merged or converted. The documentation states the Metadata Working
  Group's rule, for a caller that wants one value.
- **IIM is read into the same IPTC Core type, apart from XMP.** In a
  NEF or ARW file, the block is IFD0's 0x83BB, else resource 1028 of
  0x8649. In a JPEG file, it is resource 1028 of the APP13 segments. It is
  given as `Metadata::iim`, with every dataset (record, number and
  bytes) and the IPTC Core values it holds. A value IIM has no dataset
  for is `None` there. A text is UTF-8 where dataset 1:90 declares it,
  and otherwise its bytes that are not UTF-8 become U+FFFD, as
  ADR011 cleans every text. The MD5 digest is not checked.
- **The rating is `Rating`**, an enum of three cases:
  - -1 is `Rejected`;
  - 0 is `Unrated`;
  - a value above 0 and at most 5 is `Stars`, with the number as
    recorded, so that `2.5` is 25/10.

  Any other value, or one that is not a decimal number, is `None`, not
  clamped as the Metadata Working Group advises (§5.5, p. 41). A
  missing rating is `None`, though XMP Part 1 says to assume 0; the
  documentation says so. A number kept as stored has no order
  ([ADR009](adr009-public-api-shape.md)), so `Rating` has none, and a
  caller sorts by `Rating::to_f64`. `Rating::stars` refuses a number
  out of range.
- **The date created is raw-files' `DateTime`**, when the value has a
  date and a time to the second. It is XMP's `photoshop:DateCreated`,
  or IIM's dataset 2:55 with 2:60. XMP's fraction of a second becomes
  its subseconds, and an offset its offset; a time with no offset gives
  a `DateTime` with none. A truncated date is `None`: a year alone, a
  year and month, a date alone, or a time to the minute.
- **Three enums are closed by their standards**: the value forms
  (`XmpForm`), the kinds of array (`XmpArrayKind`) and `Rating`. They
  carry no `#[non_exhaustive]`, as `Orientation` carries none. A
  caller's match covers every case without a wildcard arm.
- **A constant for each namespace raw-files reads**, in the module
  `raw_files::namespaces`: XMP, Dublin Core, XMP Rights, Photoshop,
  IPTC Core, and XML and RDF for qualifiers. The rule is the `tags`
  module's: a constant exists where raw-files reads the thing itself.
- **The first copy stands**, for every kind of repeated thing:
  - the first APP1 XMP segment, as the first Exif segment
    ([ADR014](adr014-jpeg-files-as-a-format-of-their-own.md));
  - the first resource 1028 in the `Photoshop 3.0` segments, once they
    are joined in file order: XMP Part 3 splits a long resource section
    (p. 14);
  - the first of a repeated property;
  - the first of a repeated dataset, when it is looked up alone.
- **Damage has two reaches.** XML that is not well-formed costs the
  packet: `Xmp::from_bytes` fails with `ErrorKind::Malformed`, and
  `Metadata::xmp` is `None` while the rest of the file stands. A
  property whose RDF breaks XMP Part 1 §7 costs that property alone.
  An IIM dataset past the block's end costs the block.
- **Every cap is in `Limits`.** The XML's depth, at 64 by default; the
  values one packet gives, at 10,000; the datasets one IIM block gives,
  at 1,000. A packet's or a sidecar's size is bounded by the cap on any
  value. Under `Limits::unlimited()`, a packet nested deep enough can
  overflow the stack when its value is dropped or compared. The
  documentation says so.
- **The values are shaped as `Entries` is**, and built as every record
  is. `Xmp` and `Iim` iterate in the file's order, give their length,
  and look up by two keys. Each record is built with `with_…` methods,
  for a caller's tests. The IPTC Core values are rebuilt when a
  property changes, so they cannot disagree with it.

Rejected:

- **roxmltree, as a private dependency.** It is MIT OR Apache-2.0 and
  forbids unsafe code itself. But memchr lies beneath it, with unsafe
  code for its SIMD searches, and XMP needs a small part of XML.
- **quick-xml.** It is MIT alone, streams, and has memchr beneath it
  too.
- **Sidecars read when the file is opened**, always or when `Options`
  asks. `open` would read files the caller did not name, and raw-files
  would choose between two names and two packets.
- **A helper that names a sidecar's candidate paths.** It can be added
  later without a break, if callers ask for it.
- **The packet read only when asked for**, through `CameraFile`. Every
  packet measured lies where opening a file reads, and the typed values
  belong in `Metadata`.
- **A packet read only inside what `open` read**, another one a named
  gap. Two edited files hold IFD0 at their end, so the packet's place
  follows IFD0's, and one more read is cheap.
- **The typed values only**, or every property without them. Without
  the properties, a value raw-files does not type would need a second
  reader; without the typed values, every caller would type the rating.
- **Only the fields the full metadata issue lists** (#50), or IPTC Core
  with the Extension's locations and persons shown. The first leaves
  part of one published list out. No measured file holds a value of the
  second.
- **The sources reconciled into one getter**, by the Metadata Working
  Group's rule, or both reconciled and apart. A reconciled getter hides
  where a value came from, and the rule needs the digest checked. A
  caller already reconciles a sidecar with the file.
- **IIM not read, or read as datasets only.** Instructions and the job
  ID occur only there, on hundreds of my files. Datasets alone would
  make every caller map them to IPTC's names.
- **A rating in whole stars**, a fraction then `None` or rounded. The
  standard allows a fraction, and the Metadata Working Group expects
  them.
- **A rating as a number with `is_rejected` and `is_unrated`.** Special
  values in a number are what an enum names
  ([ADR009](adr009-public-api-shape.md)).
- **A type for a partial date**, or the date as recorded text. No file
  holds a truncated date, and a partial date can be added later without
  a break.
- **The decisions folded into ADR011, ADR016 and ADR023.** Reading XML,
  sidecars and a second standard's blocks are a problem of their own.
- **`#[non_exhaustive]` on the three enums**, as on raw-files' other
  enums. A standard closes each; a wildcard arm would only hide a
  missed case.
- **A constant for every standard namespace**, about twenty, or none.
  The first is a table of names raw-files does not read. With none, a
  caller writes out the URIs raw-files itself looks for.
- **A sidecar by its bytes and a reader only**, with no `Xmp::open`.
  `CameraFile` has all three, and a path is what a caller holds.
- **The last copy of a repeated segment**, or the first XMP segment and
  the last IIM block. No standard says which copy is right, and the
  Metadata Working Group says the other occurrences should be ignored
  (§4.2.3.4, p. 30). One rule for every kind is easier to explain.

## What this decision does not cover

- Writing XMP or IIM, into a file or a sidecar. raw-files reads files.
- Extended XMP, which no measured file holds. A JPEG's standard packet
  is read alone until a file shows Extended XMP.
- Typed values for other namespaces: Adobe Camera Raw's settings, the
  makers' and phones' own, the IPTC Extension and PLUS. The property
  list gives them all.
- XMP in formats raw-files does not open, such as PNG, TIFF, PDF and
  video.
- A sidecar in UTF-16 or UTF-32, which no measured sidecar is.
- An IIM date or time in a form IIM does not define, such as
  `CCYY-M-DD`, which 70 measured blocks write.

## Consequences

- raw-files keeps no dependency. Its XML reader is its own code to
  keep, and the fuzz target runs over it.
- A caller that wants one value reconciles Exif, XMP and IIM itself.
  The documentation gives the Metadata Working Group's rule.
- A file that holds Extended XMP gives its standard packet alone.
- A truncated date created is `None` until a file shows one; then it
  can be read without a break.
- The IPTC Extension's values, the categories and every other
  namespace stay in the property list, with no typed value.
- A new namespace needs no release: the property list gives it.
- A culler's reject mark is `Rating::Rejected`, the standard's -1.
- A missing rating is `None`, never the 0 XMP Part 1 says to assume.
- Opening a file costs one more read only for a packet that lies past
  what `open` reads; no measured file has one.
- A later copy of a repeated segment, resource or property is not
  given. On the 17 measured JPEGs that repeat one, a caller sees the
  first.
- The packet's and the block's bytes in a NEF or ARW file stay
  reachable as entries ([ADR023](adr023-every-entry-a-file-records.md)).
  A JPEG's segments are not entries, so its damaged packet is not
  reachable at all.
- An IIM text in a character set other than UTF-8 shows U+FFFD in place
  of each byte that is not ASCII.
- Some values name people and places. raw-files gives them as recorded,
  as [ADR011](adr011-metadata-from-standard-tags.md) gives the
  authorship texts.
