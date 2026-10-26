# NEF: open questions

What the [NEF documentation](README.md) does not yet settle. Part 1 lists
decisions for the maintainers. Part 2 lists facts that sample files can
establish. Part 3 lists sources that were never read. Part 4 lists questions
that sample files have already settled. Each item names the page it affects,
and test-data.md describes the sample set that would answer Part 2.

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
   - per-body tables of autofocus points, which turn the point number of a
     DSLR shot through the viewfinder, a Nikon 1 shot or a Z body's
     auto-area mask into a position
     ([makernote-and-metadata.md](makernote-and-metadata.md#86-focus-point)).

   The options are:
   - (a) treat them as format facts, transcribed from two independent sources
     with provenance recorded;
   - (b) derive them from sample files;
   - (c) a clean-room process, in which the person who reads the copyleft
     source is not the person who writes the code.

   Per-body tables can always take route (b), though route (b) can check
   a table of autofocus points only where a sample's point is known. The
   decision needs to be recorded in an ADR before the first table enters
   raw-files, which may be a table of autofocus points.
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
  the D3, D6 and the Z bodies not yet checked, such as the Z 7 and Z 8.
  The Z 9 is answered: four JPEGs of 160×120, 640×424, 1620×1080 and
  8256×5504 (files 5146 to 5148). So are the 25 bodies of the other 32 files
  of the CI set, the D300, D700, D850, D5, Z 6 and Z f among them; the Z f's
  second JPEG is 1620×1080, not the 1616×1080 reported (file 6887). Each
  file's JPEGs are listed in
  [test-data.md](test-data.md#21-the-ci-set)
  ([container-and-previews.md](container-and-previews.md)).
- **JPEG encoding.** Whether any body embeds a progressive JPEG, or one with
  unusual chroma subsampling. The Z 9's four are baseline (files 5146 to
  5148), and so is every JPEG of the other 32 files of the CI set; their
  subsampling was not read.
- **Early bodies.** The D1H is answered: its only JPEG is a 570×375
  preview (24,567 bytes at offset 3688) in the PreviewIFD of its
  headerless MakerNote, which raw-files reads, so the file opens with that
  preview, and with the ISO and the lens range the MakerNote records
  (file 5364) **[F]**. So is the D1X: its MakerNote preview, its only
  JPEG, is 570×375, 27,540 bytes at offset 4,878 on file 1785 and 26,984
  bytes at 3,688 on file 1786, and both files were read **[F]**. The D1
  (file 201, not in the CI set) has no PreviewIFD and no JPEG at all: its
  only image besides the raw is the 160×120 uncompressed strip of IFD0,
  so it opens with no preview **[F]**. The D100's MakerNote preview, its
  only JPEG, is 570×375 (file 2996).
- **Coolscan NEF.** The exact structure of a scanner NEF, and which IFD holds
  NikonScanIFD (0x0E10).
- **NikonNEFInfo.** Which IFD hosts tag 0xC7D5 (NikonNEFInfo) across bodies.
  Its contents are documented and do not affect previews
  ([container-and-previews.md](container-and-previews.md)).
- **PreviewIFD offset base.** Whether the PreviewIFD offsets of the
  short-header MakerNote count from the outer TIFF header, as inferred
  ([container-and-previews.md](container-and-previews.md)). The
  short-header MakerNotes of the early Coolpix JPEGs checked hold no
  PreviewIFD, though their other offsets count from that header
  **[F]**, so no file shows it. The headerless one's do: the D1H's
  PreviewIFD and its JPEG are where offsets counted from the file's own
  TIFF header put them (file 5364), and the early Coolpix JPEGs'
  PreviewIFDs are where offsets counted from the Exif segment's TIFF
  header put them **[F]**.
- **The short-header layout.** raw-files does not read the MakerNote that
  starts `Nikon\0\1\0` yet. raw.pixls.us has no file of the Coolpix E700
  to E950 that wrote it, as those bodies wrote no raw files, but their
  camera JPEGs on Wikimedia Commons have it: an IFD 8 bytes in, whose
  offsets count from the outer TIFF header, the Exif segment's
  ([makernote-and-metadata.md](makernote-and-metadata.md#in-a-jpeg-file),
  files listed in the
  [JPEG test data](../jpeg/test-data.md#6-early-coolpix-files-checked-by-hand))
  **[F]**. What stays open is the byte order of its numbers, on which
  the sources disagree, ExifTool's documentation saying little-endian
  and exiv2 reading the outer order: every one of those files is
  little-endian inside the MakerNote and out, so they agree with both.
  Only a file whose outer order is big-endian could settle it, and no
  camera-original one is known.
- **Multi-strip previews.** Whether any SubIFD JPEG is stored in several
  strips. On the Z 9 every JPEG is in one piece, through 0x0201/0x0202 (files
  5146 to 5148), and so is every JPEG of the other 32 files of the CI set;
  other bodies are open
  ([container-and-previews.md](container-and-previews.md)).
- **Truncation and padding.** How common truncated JPEGs or padding after the
  end-of-image marker are in real files. The Z 9 has neither: each JPEG ends
  with the end-of-image marker exactly at its declared length (files 5146 to
  5148), as does every JPEG of the other 32 files of the CI set. Of 4,942
  files from the maintainer's D600 and D3200, none is truncated and nine
  declare a length 513 bytes past the end-of-image marker, with leftover
  bytes rather than padding after it; other bodies are open
  ([container-and-previews.md](container-and-previews.md)).
- **Preview Exif.** Whether a SubIFD or PreviewIFD JPEG ever carries its own
  Exif APP1 with an Orientation. Not on the Z 9, whose JPEGs have no APP
  segment at all (files 5146 to 5148), nor on the other 32 files of the CI
  set, whose JPEGs have none before their frame header; other bodies are
  open
  ([container-and-previews.md](container-and-previews.md)).
- **Z 9 SubIFDs without a preview.** What the Z 9's 384×256 uncompressed
  8-bit grey SubIFD and its two empty SubIFDs (width, height and strip
  length 0) are for (files 5146 to 5148). A preview finder skips all three
  ([container-and-previews.md](container-and-previews.md)).

### MakerNote and metadata

- **Z-series white balance.** Whether 0x000C WB_RBLevels is present and
  correct on every Z body. Every decoder relies on it there, and no failure
  has been reported.
- **Crop area.** Which body first wrote 0x0045 CropArea: the D5 (2016)
  does (file 2218) and the D810 (2014) does not, and no body in between
  was checked.
  And how the raw IFD size maps to the visible size on the bodies without
  it, which today report the raw size with its border pixels
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **Focus point.** What no sample shows yet
  ([makernote-and-metadata.md](makernote-and-metadata.md#86-focus-point),
  its section 11):
  - a DSLR in live view on a DX or 1.2x crop: the cropped or the full
    frame;
  - CropHiSpeed's sizes on uncropped files, after the D810's four-pixel
    difference under mode 11 (file 2036);
  - which bodies write FaceDetect (0x0021);
  - crops that cut both sides and change the aspect, such as a DX crop
    at 1:1 on an FX body: scaled like DX or a window like 1:1;
  - where an aspect crop's window lies: centred, as raw-files assumes,
    or at CropHiSpeed's offsets, up to eight pixels away;
  - whether a Z body records a crop mode on a small raw, and how.

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
- **NEFCompression on Z bodies.** Tag 0x0093 is absent from the Z 9 files,
  the lossless one included (files 5146 to 5148), as the claim that Z bodies
  keep the value in tag 0x0051 predicts; but the Z fc and the Z 30 write
  0x0093, value 3, and no 0x0051 (files 8559, 5812), so reading the
  compression on Z bodies must try both. Byte 10 of 0x0051, read as a u16
  in the MakerNote's byte order, reads 3, 14 and 13 on files 5146 to 5148,
  their lossless, HE\* and HE files
  ([makernote-and-metadata.md](makernote-and-metadata.md#2-key-tags));
  since all three MakerNotes are little-endian, those files do not tell
  which byte order the sources mean ([raw-data.md](raw-data.md)).
- **Lossless debut.** Whether lossless compression first appeared on the
  D3/D300.
- **Curve sizes.** How Claff's encoding-table sizes relate to the stored
  `csize` on bodies after 2009 ([raw-data.md](raw-data.md)).
- **D850 RAW M/S.** Confirmation at the byte level that these are a smaller
  Bayer CFA with 12-bit lossless compression, and which Huffman table they use.
  Also whether the Z 9 and the original Z 6/Z 7 write M/S NEF.
- **BlackLevel.** Which body first wrote 0x003D BlackLevel, and its channel
  order on any non-RGGB body that carries it.
- **Z 7 header.** The byte order of a Z 7 TIFF header; the Z 6's is `II`
  (file 3587). Their 12-bit packing is little-endian either way.
- **sNEF.** A reference conversion for sNEF (D810, D4S), where implementations
  disagree.

## 3. Sources not consulted

- **Nikon SDK.** Nikon's SDK portal, which was not reachable. Any SDK is
  under NDA, so it could not inform an open implementation anyway.
- **Not reachable.** The ExifTool forum, which was down, and the oss-fuzz
  issue tracker, which is not publicly indexed.
- **Blocked.** DPReview and Photography Life articles. Claims that rest on
  them are tagged [S] from search extracts.

## 4. Questions settled by sample files

Each of these was open before the sample files were checked. The files
that settled it are named; their SHA-256 hashes are in
[test-data.md](test-data.md#2-what-is-committed-and-what-is-downloaded).
These were settled by the three CC0 Z 9 files of the CI set:

- **HE / HE\* layout**: HE and HE\* files do not differ from lossless ones
  in IFD or preview layout. The IFD layout, the SubIFD order, the tags of
  every IFD of the tree and of the PreviewIFD, and the four preview sizes
  are the same in the lossless, HE\* and HE files; the raw strip's content
  and length differ; and the MakerNote IFD differs by one tag, 0x0096,
  present in the lossless file only (files 5146 to 5148)
  ([container-and-previews.md](container-and-previews.md)).

These were settled by the other 20 NEF files of the CI set, of 18 bodies
from the D1H to the Z6III, listed in
[test-data.md](test-data.md#21-the-ci-set) with the previews of each:

- **Outer byte order**: big-endian (`MM`) on the D1H, D100, D70, D200,
  D300, D700, D7000 and Coolpix A (files 5364, 2996, 2060, 8079, 928,
  2255, 961, 2238); little-endian (`II`) on the D750, D810, D850, D780,
  Z 6, Z f and Z6III, as on the Z 9, and on the E5700, the Nikon 1 J1 and
  the Coolpix P7800, whose file is an NRW (files 895, 2036, 1835, 3827,
  3587, 6887, 7808 to 7810, 4456, 2956, 1425). Every embedded MakerNote
  header has its file's byte order
  ([container-and-previews.md](container-and-previews.md)).
- **Z 6III previews**: the Z6III follows the second-JPEG pattern,
  1620×1080 at 3:2, and at its other aspect ratios writes a JPEG 1080
  pixels high of the same shape, 1920×1080 at 16:9 and 1080×1080 at 1:1
  (files 7810, 7809, 7808).

The following were settled by the 23 files checked for metadata, listed in
[test-data.md](test-data.md#21-the-ci-set) with their SHA-256 hashes; the
details are in
[makernote-and-metadata.md](makernote-and-metadata.md).

- **Time-zone tags**: the D850 and every later checked body write
  OffsetTimeOriginal (0x9011) with OffsetTime and OffsetTimeDigitized,
  and the MakerNote's WorldTime agrees with it in all of them (files
  1835, 3827, 3587, 6887, 7808 to 7810, 5146 to 5148). Older bodies write
  no Exif offset; from the D300 on they have WorldTime, which is taken to
  mean the same but cannot be compared there.
- **Make and Model strings**: NUL-terminated and unpadded on every checked
  body except the D100, whose Model has one trailing space (file 2996).
- **Crop area on recent bodies**: the D850, D780 and Z bodies write 0x0045
  CropArea, and it is the visible image inside the stored raw (files 1835,
  3827, 3587, 6887, 7808 to 7810, 5146 to 5148).
