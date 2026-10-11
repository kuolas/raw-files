# NEF: open questions

What the [NEF documentation](README.md) does not yet settle. Part 1 lists
decisions for the maintainers. Part 2 lists facts that sample files can
establish. Part 3 lists sources that were never read. Each item names the page
it affects, and test-data.md describes the sample set that would answer Part 2.

## 1. Decisions for the maintainers

1. **Provenance of format data tables.** Raw decoding needs data that is
   published in copyleft projects and in dcraw, whose header states that
   all its code outside a single restricted (Foveon) block is free for all
   uses; dcraw's licence is still described as ambiguous for the whole file
   ([licences-and-references.md](licences-and-references.md)). That is an
   input to the decision, not legal advice. The data:
   - the symbol lists of the six NEF Huffman tables
     ([raw-data.md](raw-data.md)). Their structure, the counts per code
     length, is already documented.
   - the two 256-byte substitution tables used by the MakerNote cipher
     ([makernote-and-metadata.md](makernote-and-metadata.md)).
   - per-body tables: CFA phase, crop trims and white levels.

   The options are:
   - (a) treat them as format facts, transcribed from two independent sources
     with provenance recorded;
   - (b) derive them from sample files;
   - (c) a clean-room process, in which the person who reads the copyleft
     source is not the person who writes the code.

   Per-body tables can always take route (b). The decision needs to be
   recorded in an ADR before milestone 4.
2. **HE / HE\*.** The recommendation is to detect these files and return an
   "unsupported" error ([raw-data.md](raw-data.md)). Implementing a decoder
   would touch the JPEG XS patent pool. That is a legal question, not an
   engineering one.
3. **Byte-order mismatch.** Nikon software can leave a NEF whose MakerNote
   byte order differs from the file's. The question is whether to tolerate
   that for previews and metadata (recommended) and to reject it for raw
   decoding. rawler rejects it.
4. **Serial-number parsing for decryption.** Implementations differ when the
   serial contains non-digits:
   - dcraw folds each non-digit in as `c % 10`.
   - LibRaw stops at the first non-digit and uses 34 for D50-family bodies, 96
     otherwise.
   - ExifTool and exiv2 also use 0x22 and 0x60.

   One behaviour should be picked and tested
   ([makernote-and-metadata.md](makernote-and-metadata.md)).
5. **Test data strategy.** The proposal is to download large samples in CI,
   pinned by SHA-256, and commit only tiny fixtures ([test-data.md](test-data.md)).
6. **Strictness on corrupt data.** Implementations differ in three places
   ([raw-data.md](raw-data.md)):
   - how they clamp the curve index;
   - whether they reject a curve whose anchor points do not tile the range;
   - whether a truncated bitstream is an error or zero-filled.

## 2. Facts to establish from sample files

### Container and previews

- **Largest preview per body.** The dimensions and count of embedded JPEGs on
  the D3/D300/D700, D850, D5/D6 and Z bodies. The Z f's reported 1616×1080 in
  particular needs checking.
- **JPEG encoding.** Whether any body embeds a progressive JPEG, or one with
  unusual chroma subsampling.
- **Outer byte order.** For the D800 and later bodies, Z bodies and Coolpix
  NRW. Pre-2010 DSLRs are `MM` and the E5700 is `II`.
- **Early bodies.** Where the D1 and D1H keep their previews, and how large the
  MakerNote preview is on the D1X and D100.
- **HE / HE\* layout.** Whether HE/HE\* files differ in IFD or preview layout.
- **Coolscan NEF.** The exact structure of a scanner NEF, and which IFD holds
  NikonScanIFD (0x0E10).
- **NikonNEFInfo.** Which IFD hosts tag 0xC7D5 (NikonNEFInfo) across bodies.
  Its contents are documented and do not affect previews
  ([container-and-previews.md](container-and-previews.md)).
- **PreviewIFD offset base.** Whether the PreviewIFD offsets of the
  headerless and short-header MakerNotes count from the outer TIFF header, as
  inferred ([container-and-previews.md](container-and-previews.md)).
- **Multi-strip previews.** Whether any SubIFD JPEG is stored in several
  strips ([container-and-previews.md](container-and-previews.md)).
- **Truncation and padding.** How common truncated JPEGs or padding after the
  end-of-image marker are in real files
  ([container-and-previews.md](container-and-previews.md)).
- **Preview Exif.** Whether a SubIFD or PreviewIFD JPEG ever carries its own
  Exif APP1 with an Orientation
  ([container-and-previews.md](container-and-previews.md)).
- **Z 6III previews.** Whether the Z 6III follows the 1620×1080 second-JPEG
  pattern ([container-and-previews.md](container-and-previews.md)).

### MakerNote and metadata

- **Time-zone tags.** Which bodies write OffsetTimeOriginal (0x9011). Only the
  Z 9 is evidenced.
- **Make and Model strings.** Their padding on recent bodies:
  NUL-terminated is confirmed on a D40, and a trailing space on the D100
  (rawspeed compares Model against "NIKON D100 "), but trailing spaces on
  recent bodies are unconfirmed.
- **Z-series white balance.** Whether 0x000C WB_RBLevels is present and
  correct on every Z body. Every decoder relies on it there, and no failure
  has been reported.
- **Crop area.** Whether 0x0045 CropArea is always present, and how raw IFD
  size maps to output size on each body.

### Raw data

- **0x0096 version bytes.** The version bytes written by lossy type 1 bodies:
  D1H, D1X, D70 and D2H are unconfirmed. Separately, which bodies write 0x49 or
  0x58, and what the 2110 bytes skipped for them contain.
- **Predictor seed order.** dcraw and LibRaw read the four seeds of 0x0096
  row-major, rawspeed and rawler column-major. Read the four seeds from
  samples; if they are always equal the question is moot
  ([raw-data.md](raw-data.md)).
- **NEFCompression 5 and 6.** Which bodies write them.
- **NEFCompression 9.** Whether Z bodies write 9 for 12-bit uncompressed
  files, or whether it comes only from Coolpix.
- **Lossless debut.** Whether lossless compression first appeared on the
  D3/D300.
- **Curve sizes.** How Claff's encoding-table sizes relate to the stored
  `csize` on bodies after 2009 ([raw-data.md](raw-data.md)).
- **D850 RAW M/S.** Confirmation at the byte level that these are a smaller
  Bayer CFA with 12-bit lossless compression, and which Huffman table they use.
  Also whether the Z 9 and the original Z 6/Z 7 write M/S NEF.
- **BlackLevel.** Which body first wrote 0x003D BlackLevel, and its channel
  order on any non-RGGB body that carries it.
- **Z 6 and Z 7 header.** The byte order of a Z 6/Z 7 TIFF header. Their
  12-bit packing is little-endian either way.
- **sNEF.** A reference conversion for sNEF (D810, D4S), where implementations
  disagree.

## 3. Sources not consulted

- **Nikon SDK.** Nikon's SDK portal, which was not reachable. Any SDK is
  under NDA, so it could not inform an open implementation anyway.
- **Not reachable.** The ExifTool forum, which was down, and the oss-fuzz
  issue tracker, which is not publicly indexed.
- **Blocked.** DPReview and Photography Life articles. Claims that rest on
  them are tagged [S] from search extracts.
