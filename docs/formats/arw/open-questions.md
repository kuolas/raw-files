# ARW: open questions

What the [ARW documentation](README.md) does not yet settle. Part 1 lists
decisions for the maintainers. Part 2 lists facts that sample files can
establish, collected from the pages' own open questions with duplicates
merged. Part 3 lists sources that were not read or could not be reached.
Part 4 lists questions that the sample checks have already settled, so
readers can see what moved from open to settled. Each item names the page it
affects; [test-data.md](test-data.md) describes the samples that would
answer Part 2.

## 1. Decisions for the maintainers

1. **Provenance of data a decoder must bring itself.** Raw decoding needs
   some data that the file does not carry
   ([raw-data.md](raw-data.md),
   [licences-and-references.md](licences-and-references.md)). Most of it
   is published in copyleft projects and in dcraw, whose header frees all
   its code outside the Foveon functions for all uses but whose licence is
   still described as informal. That is an input to the decision, not legal
   advice. Unlike NEF, most items reduce to a rule or a formula:
   - the ARW1 prefix code, published as an 18-entry table in dcraw, LibRaw
     and rawkit, reduces to a short rule, checked on files 2346 and 2271;
   - the inverse of the MakerNote cipher is computable from b³ mod 249
     (full metadata needs it, for what the enciphered tags hold);
   - the SR2 keystream is a formula with two constants.

   What remains:
   - per-body crops, black and white level fallbacks and CFA patterns for
     older bodies without crop tags, which dcraw and LibRaw trim by raw
     width and rawspeed lists per body;
   - colour matrices: every decoder uses Adobe-derived per-body matrices,
     and the meaning of Sony's own 0x7800 is unknown;
   - YCbCr to RGB coefficients for the M and S sizes, if raw-files ever
     converts them (LibRaw uses BT.601, a public standard);
   - **the ARW 6.0 delinearisation curve, the hard case**: 4096 entries,
     not in the file, construction unknown, and published only as tables
     extracted from existing decoders (LibRaw's embedded table, under its
     own licence, and an unlicensed write-up's);
   - tables of autofocus points, which turn the point number that the
     A-mount, NEX and early ILCE bodies record into a position; no open source of
     them is known
     ([makernote-and-metadata.md](makernote-and-metadata.md#96-focus-point)).

   The options are, as for NEF:
   - (a) treat a short table as a format fact, transcribed from two
     independent sources with provenance recorded;
   - (b) derive it from sample files;
   - (c) a clean-room process, in which the person who reads the copyleft
     source is not the person who writes the code.

   Per-body values can always take route (b). Graphite's rawkit does not
   change the picture: it matches dcraw element for element, so reading it
   is reading dcraw. A table that turned out to be a published formula
   would remove the problem for the ARW 6.0 curve. The decision needs to be
   recorded in an ADR before the first table enters raw-files.
2. **ARW 6.0.** The recommendation is to detect Compression 32766 and
   return a typed unsupported error before reading the strip, while still
   returning previews and metadata from these files
   ([raw-data.md](raw-data.md), [security.md](security.md)). Lifting the
   refusal needs two answers: where the curve comes from (decision 1), and
   the patent position. The stream's names suggest Sony's LLVC wavelet
   codec, disclosed as SMPTE RDD 34:2015, which directs IP enquiries to
   Sony; that link is an inference, and the JPEG XS patent pool may be
   relevant if the codec uses JPEG XS techniques
   ([licences-and-references.md](licences-and-references.md)). These are
   legal questions, not engineering ones, and the same posture as NEF
   HE/HE\*.
3. **Decryption and anti-circumvention.** Full metadata meets this
   question first, for the encrypted SRF MakerNote IFDs and the
   enciphered tags. Raw sensor data needs the SR2 keystream for the
   levels and white balance of older files and for Sony's white level on
   all of them; the key is stored in the file. Does decrypting it count
   as circumventing a technological measure under DMCA-like laws? The
   question is untested; open decoders have done it for about twenty
   years and no action against them was found
   ([licences-and-references.md](licences-and-references.md),
   [raw-data.md](raw-data.md)). Previews and the metadata raw-files reads
   need no decryption.
4. **Scope for SRF, ARQ and the A100.** The one preview walk covers all of
   them. For raw data
   ([raw-data.md](raw-data.md),
   [makernote-and-metadata.md](makernote-and-metadata.md)):
   - SRF serves two bodies from 2003–2004, encrypts the image, has a
     four-colour CFA according to the decoders, and its SRF2 to SRF6
     IFDs were not located;
   - ARQ is a desktop-software product of hundreds of megabytes (a 16-shot
     file is about 2 GB);
   - the A100 keeps its white balance in a Minolta MRW block, and where its
     levels are is unknown.

   Which of these raw-files decodes for raw sensor data, and whether the
   rest return a typed unsupported error, is to be decided. The pages
   suggest SRF and ARQ later, and the MRW block and SRF key chain behind
   "unsupported" until there is demand.
5. **Strictness and output where implementations disagree.** One
   behaviour should be picked and tested for each
   ([raw-data.md](raw-data.md),
   [makernote-and-metadata.md](makernote-and-metadata.md)):
   - **White level.** SR2SubIFD 0x787F says 15360 and the plain DNG tag
     0xC61D says 16383 in the same file; rawspeed and rawler use the first,
     libopenraw the second, LibRaw neither. Report both with provenance, or
     pick one?
   - **cRAW output scale.** dcraw writes the curve value shifted right by
     2 (12-bit scale); LibRaw, rawspeed and rawler write the curve value,
     rawspeed optionally and rawler always dithered. The pages recommend
     the undithered curve value, which can exceed 16383.
   - **`imax == imin` in a cRAW block.** rawspeed rejects the block; dcraw
     and LibRaw read a fifteenth delta past it. No sample block has it, so
     rejecting costs nothing on real files.
   - **cRAW widths not a multiple of 32.** dcraw and LibRaw leave the last
     columns undecoded, rawspeed rejects the file, rawler ignores the
     remainder. Every checked cRAW width is a multiple of 32.
   - **ARW1 range violations.** dcraw and LibRaw flag and continue,
     rawspeed throws, rawler wraps.
   - **M/S output form.** A three-channel YCbCr image flagged as
     white-balanced, or an RGB conversion (LibRaw and rawler convert, with
     different black levels)? The pages suggest YCbCr with any conversion
     labelled approximate.
   - **Black-level channel order** on non-RGGB bodies: fixed colour order
     (LibRaw) or CFA position (rawspeed, rawler). They differ only on the
     R1.
   - **Corrupt data.** Decode what can be decoded and count bad blocks and
     range violations, or fail the file?
6. **Size and framing for metadata.** Which "image dimensions" to
   report: the stored size, the visible size from DefaultCrop, the
   in-camera framing from SonyCrop, or 0x7038's usable area. The page
   recommends the stored size, DefaultCrop as the visible size and SonyCrop
   as an optional framing, with no per-body trim table
   ([makernote-and-metadata.md](makernote-and-metadata.md)).
7. **Test-data strategy.** The proposal is to download CC0 samples from
   raw.pixls.us in CI, pinned by SHA-256, and commit only small fixtures
   ([test-data.md](test-data.md)):
   - a minimal set of 25 files, about 690 MiB, with a preview-only subset
     of eleven files, about 220 MiB;
   - the one ARQ (325 MiB) in an opt-in large tier;
   - the server ignores range requests, so even a preview test fetches the
     whole file.

   Whether that download volume is acceptable in CI, or the set should be
   smaller, is to be decided.
8. **One preview finder for NEF and ARW?** The NEF algorithm already
   walks the whole IFD0 chain and rejects lossless SOF markers; the ARW
   finder adds a role filter that skips raw IFDs
   ([container-and-previews.md](container-and-previews.md),
   [../nef/container-and-previews.md](../nef/container-and-previews.md)).
   A shared finder must settle two remaining divergences:
   - **Candidates without a SOF.** NEF step 6 ranks them after every
     candidate with a SOF, by byte length; ARW step 8 drops them.
   - **The SOI test.** NEF requires `FF D8` and makes the third `FF`
     optional; ARW requires `FF D8 FF`.

   Whether the role filter is safe for NEF needs checking against NEF
   samples.
9. **How strict the SubIFD check should be.** The reader follows a SubIFD
   pointer only when its target parses as an IFD with every entry of a
   known type and inside the file, the rule of
   [container-and-previews.md](container-and-previews.md) §7.1 step 2 that
   rejects the A100's raw-data pointer. The same rule drops a real SubIFD
   with one entry of a vendor-private type, or one whose out-of-line value
   lies past the end of a truncated file, even when its preview is still
   readable; chain IFDs and the Exif IFD are kept with such entries. A
   looser rule (reject only when few or no entries can be read) would
   also reject the A100's pointer, whose entry types are garbage. The
   strict rule is the documented one and stays until the sample and
   corpus checks show it losing a preview.

## 2. Facts to establish from sample files

### Container and previews

- **ZV-1M2 without IFD2.** The only ZV-1M2 dump on raw.pixls.us
  (FileFormat `4 0 0 0`, firmware 2.01) shows no IFD2. Check that file's
  bytes for IFD1's next pointer and any large JPEG, and whether other
  1-inch compacts at 4.0 and later lack IFD2
  ([container-and-previews.md](container-and-previews.md)).
- **IFD2 on bodies not byte-checked.** ILCE-7RM5, ILCE-7CR, DSC-RX1RM3,
  ILCE-7CM2, ILME-FX2, ILME-FX3, ILME-FX30, ZV-E1 and ZV-E10M2: confirm
  SOF0, 8-bit, 4:2:2, no APP segment, and an SOF size equal to the tags
  ([container-and-previews.md](container-and-previews.md)).
- **ARW 6.0 bodies other than the ILCE-7M5.** Whether the ILCE-7RM6 has the
  same IFD2 layout ([container-and-previews.md](container-and-previews.md)).
- **Aspect ratio on 4.0 and later.** In a file shot at 16:9, 1:1 or 4:3,
  whether IFD2 carries the crop, as IFD0 does, or the full 3:2
  DefaultCropSize. Measure both SOF sizes
  ([container-and-previews.md](container-and-previews.md)).
- **4:3 IFD0 preview.** Which files' PreviewImageSize (MakerNote 0xB02C)
  says 1440×1080, and whether their IFD0 SOF agrees
  ([container-and-previews.md](container-and-previews.md)).
- **ILCE-7M5 RAW and HEIF before firmware 1.01.** Which embedded JPEG
  carried the noise that Sony's firmware note describes
  ([container-and-previews.md](container-and-previews.md)).
- **ARQ variants.** Whether ARQs from 16 shots (ILCE-7RM4 and later) and
  from 4.0-and-later frames keep the source frame's IFD0, IFD1 and IFD2 as
  file 3255 does, and their raw dimensions, Photometric and crop (A7R V,
  A7CR, A1 II) ([container-and-previews.md](container-and-previews.md),
  [raw-data.md](raw-data.md)).
- **Files edited by Sony's software.** In an ARW edited in Image Data
  Converter or Imaging Edge: whether IDCPreview JPEGs appear, whether IFD0
  or IFD2 is rewritten, whether the 0x7240 + 6 link survives, and whether
  the 0x7241 and version IFDs really count offsets from their own start. No
  checked file was edited (0x7241 is 0 everywhere)
  ([container-and-previews.md](container-and-previews.md),
  [makernote-and-metadata.md](makernote-and-metadata.md),
  [security.md](security.md)).
- **The 0x7240 + 6 link.** What the six zero bytes and the offset mean, and
  whether a writer must keep the link to IFD2 or the raw SubIFD
  ([container-and-previews.md](container-and-previews.md),
  [makernote-and-metadata.md](makernote-and-metadata.md)).
- **Third-party re-saves.** In a file re-saved by Photo Mechanic, ExifTool
  or Lightroom: whether IFD0 moves and whether IFD2 survives. Every checked
  file has IFD0 at offset 8
  ([container-and-previews.md](container-and-previews.md),
  [security.md](security.md)).
- **A100 firmware 1.00.** A file whose IFD1 offset is 0x10000 too small, to
  confirm ExifTool's repair; file 2346 is firmware 1.04
  ([container-and-previews.md](container-and-previews.md),
  [security.md](security.md)).
- **SRF details.** Whether the DSC-V3 has the F828's IFD0 and IFD1 layout,
  and whether the 672×492 RGB image reported in SRF6 exists in file 1351
  ([container-and-previews.md](container-and-previews.md)).
- **Hasselblad Stellar, HV and Lusso.** Make, MakerNote header (`VHAB` or
  none) and preview layout; raw.pixls.us has none of them
  ([container-and-previews.md](container-and-previews.md),
  [makernote-and-metadata.md](makernote-and-metadata.md)).
- **Byte order.** Any Sony raw file with an `MM` header. One would also
  settle the SR2SubIFDKey byte-order disagreement
  ([container-and-previews.md](container-and-previews.md),
  [makernote-and-metadata.md](makernote-and-metadata.md)).
- **Non-baseline previews.** Any Sony file whose embedded JPEG is
  progressive, not 4:2:2, or carries an APP segment
  ([container-and-previews.md](container-and-previews.md)).
- **Format identification.** Whether PRONOM's DROID tool identifies ARW 4.0
  and later files at all; a run on files 4466, 7361 and 8846 would show it
  ([licences-and-references.md](licences-and-references.md)).

### MakerNote and metadata

- **First body with OffsetTime\* and plain 0x7310/0x7313.** Both appear
  between ARW 2.3.1 (ILCE-7RM2) and 2.3.3 (ILCE-9). Read the bytes, not the
  exiv2 dumps, of the ARW 2.3.2 bodies (ILCE-6500, ILCE-7SM2, ILCA-99M2,
  DSC-RX100M5) and of one file per body up to `3 3 5 0` (for example
  DSC-RX10M3, ILCE-7M3, ILCE-9M2)
  ([makernote-and-metadata.md](makernote-and-metadata.md),
  [test-data.md](test-data.md)).
- **SubSecTime\* writers.** Which ARW 2.3.5 bodies write it (the dumps show
  some; the ILCE-7RM3 sample does not), and whether it depends on firmware
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **0x74C7 on older bodies.** Which bodies before ARW 2.3.3 write
  SonyCropTopLeft and SonyCropSize, and whether the ILCE-7RM3's (0, 8)
  against DefaultCropOrigin (8, 8) holds on other firmware or other 2.3.5
  bodies (ILCE-7RM4, ILCE-9M2)
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **APS-C mode marker.** Whether any tag other than the smaller raw size
  (and 0x202E = 3) marks Super 35 capture on ILCE bodies, for example in
  the enciphered blocks
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **0x9050 layouts of the ILCE-9M3, ILCE-6700 and ILCE-1M2.** The values at
  the documented offsets are not plausible shutter counts in files 7361,
  7364 and 6736, and file 7835's block is mostly `FF` (219 of 256 bytes,
  `FF` at the count offsets). It matters to full metadata, for the
  shutter count ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **Focus point.** What no sample shows yet
  ([makernote-and-metadata.md](makernote-and-metadata.md#96-focus-point),
  its section 13):
  - the meaning of FocusFrameSize's third value (257 or 0);
  - the point numbers in CameraInfo (0x0010) of the A-mount and NEX
    bodies and in AFPointSelected (0x201E) of the ILCE-7S, which no
    check decoded, and the A100's Minolta block;
  - whether the ILCA bodies write FocusLocation;
  - whether any plain tag, or the enciphered 0x940E block on ILCE
    bodies, records that focus was confirmed;
  - a rotated shot with a focus point, and a big-endian file;
  - a file with FocusLocation2 and no FocusLocation;
  - an autofocus shot whose FocusFrameSize is zero.

### Raw data

- **White level.** Which of 0x787F (15360) and 0xC61D (16383) matches
  saturation, from clipped highlights in uncompressed ILCE-9, ILCE-7M4 or
  ILCE-1 files ([raw-data.md](raw-data.md),
  [makernote-and-metadata.md](makernote-and-metadata.md)).
- **cRAW at full scale.** A clipped-highlight cRAW file from a 14-bit body
  (ILCE-9, ILCE-7M4 or ILCE-7RM3) to see whether code 2047 occurs and
  whether the output really exceeds 16383; no checked block went above code
  2020 ([raw-data.md](raw-data.md), [security.md](security.md)).
- **0x7010 knots.** Whether bodies other than the NEX-5 store other knots,
  and whether the knots change with ISO or the 12-bit modes
  ([raw-data.md](raw-data.md)).
- **12-bit fallback modes.** A silent-shutter or continuous-drive 12-bit
  file (ILCE-6300, ILCE-6500, ILCE-7M3), cRAW and uncompressed: confirm the
  14-bit scaling, what BitsPerSample and SonyRawFileType say, and whether
  the strip size, curve or levels change in a way the dispatch rule misses
  ([raw-data.md](raw-data.md), [security.md](security.md)).
- **ARW1 on other A-mount bodies.** Decode the DSLR-A230, A290, A350, A380
  and A390 files (CC0; the A300 and A330 are not) as ARW1 with H + 8 and
  check that all bytes are consumed; the 4600-wide bodies cannot be cRAW.
  Also what the A200's eight extra rows hold
  ([raw-data.md](raw-data.md), [security.md](security.md)).
- **SRF2 to SRF6.** Where SRF2 starts in file 1351 and with which key (the
  byte order of SRF2Key, and whether the keystream continues from SRF1);
  whether SRF3 to SRF5 are plain; then read RawDataOffset and
  RawDataLength from SRF2 and compare them with 862,144 and 16,531,200. This
  decides where SRF white balance lives and whether the chain can be walked
  without dcraw's absolute constants ([raw-data.md](raw-data.md),
  [makernote-and-metadata.md](makernote-and-metadata.md),
  [security.md](security.md)).
- **F828 CFA.** Decrypt and decode file 1351 and look for four distinct
  channel means in the RGBE (or Y R / B G) arrangement; the DSC-V3's
  pattern and data offset need a file that raw.pixls.us does not have
  ([raw-data.md](raw-data.md)).
- **R1 black-level order.** 0x7300's channel order against the R1's GRBG
  CFA, from a uniform patch in a DSC-R1 file ([raw-data.md](raw-data.md)).
- **A100 levels.** Where the A100's black and white levels are, if anywhere
  in the MRW block of file 2346 ([raw-data.md](raw-data.md)).
- **M/S YCbCr.** The luma and chroma scale (precision 16, the chroma
  midpoint), whether rawler's doubled black level is right, and M/S files
  from the ILCE-1, ILCE-1M2, ILCE-7RM5, ILCE-7CR, DSC-RX1RM3 and ILME-FX2
  (subsampling per body) ([raw-data.md](raw-data.md),
  [test-data.md](test-data.md)).
- **Lossless 12-bit.** Whether any lossless file uses 12-bit precision
  (ILCE-1 continuous or silent shutter) ([raw-data.md](raw-data.md)).
- **ARW 6.0 stream.** The meaning of the tile-stream header bytes 4 to 7
  and 12 to 15 on the ILCE-7M5; the ILCE-7RM6's tile layout (2×2?) and
  stream tag (`0000`?), from dnglab's RawDB files after a licence check;
  whether Compressed and Compressed (HQ) differ only in quantiser choice
  ([raw-data.md](raw-data.md), [test-data.md](test-data.md)).
- **ColorMatrix 0x7800.** What it maps; compare with Adobe's matrix for the
  ILCE-9 (file 1542) ([raw-data.md](raw-data.md)).
- **The ILCE-7 file.** Its bit depth and codec, since its catalogue label
  and file name disagree ([test-data.md](test-data.md)).

### Security

- **Enciphered block lengths.** The actual lengths of 0x2010 and 0x9050 on,
  for example, the ILCE-9, ILCE-1, ILCE-7M4 and ILCE-7M5, against the
  largest fixed offset LibRaw and ExifTool read. Only matters if those
  blocks are ever read ([security.md](security.md)).

Other questions that bear on security (edited and re-saved files, SRF,
cRAW highlights, ARW1 widths, 12-bit fallback, A100 firmware 1.00) are
listed under the pages above.

### Test data

- **Pixel output.** Whether decoded pixels match a reference decoder; no
  comparison was made on any checked file ([test-data.md](test-data.md)).
- **File 8845** (ILCE-7M5, full frame, lossless). Whether it has the same
  structure as the APS-C lossless file 8848, so it can join the minimal set
  ([test-data.md](test-data.md)).
- **The catalogue's licence field.** Whether it can change after upload;
  compare the `set=noncc0` view at intervals ([test-data.md](test-data.md)).
- **Stable ids and URLs.** Whether raw.pixls.us ids and download URLs stay
  put; compare the catalogue's URLs with `filelist.sha256` paths at
  intervals ([test-data.md](test-data.md)).
- **dnglab's RawDB files.** The licence of its ILCE-7RM6 and DSC-RX10M5
  files, the only public source for the ILCE-7RM6's ARW 6.0 layout
  ([test-data.md](test-data.md)).

## 3. Sources not consulted

- **Sony.** No public specification or decoding SDK was found. The Camera
  Remote SDK is for remote control, is distributed on application under a
  licence that forbids reverse engineering, and describes no file format;
  it was not used ([licences-and-references.md](licences-and-references.md)).
- **Not reachable.** The ExifTool forum was unavailable; claims from it
  rest on search extracts and are tagged [S]. dcraw's original site did not
  respond, so every page read dcraw 9.28 from a mirror not verified against
  the original ([security.md](security.md)).
  The per-file licences in dnglab's RawDB could not be read, because the site
  is a JavaScript application ([test-data.md](test-data.md)).
- **Read only through search extracts or summaries.** Sony's support
  article on 12-bit modes and its Pixel Shift Multi Shooting and RAW Driver
  pages; several forum and blog posts (Fred Miranda, XnView, Capture One,
  Jim Kasson, Lloyd Chambers, the ILCE-7M5 support reports); Google
  Patents' records for the cRAW patents. Claims from them are tagged [S].
- **Not opened.** ZDI's advisory pages and the OSS-Fuzz issue tracker
  ([security.md](security.md)); the LibRaw 0.18.2 release tarball, which
  may differ from its git tag ([security.md](security.md)); the status of
  EP 1841233, the European counterpart of the cRAW patent
  ([licences-and-references.md](licences-and-references.md)).
- **Read in part, deliberately.** arw6_decode has no licence, so only its
  README was read; LibRaw's ARW 6.0 decoder was read for its header,
  identifiers and the curve's shape only; SMPTE RDD 34 was read for pages 1
  to 8 ([licences-and-references.md](licences-and-references.md),
  [raw-data.md](raw-data.md)).
- **Paid standards.** ISO 12234-2 (TIFF/EP) and ISO/IEC 21122 (JPEG XS)
  were not read ([licences-and-references.md](licences-and-references.md)).
- **Samples not used.** The nine non-CC0 Sony files on raw.pixls.us,
  including the only DSLR-A850 file, were not downloaded, and bodies
  absent from the site (ILCE-7RM6, DSC-V3, the Hasselblad Stellar, HV and
  Lusso) were not examined ([test-data.md](test-data.md)).

## 4. Questions settled by the sample checks

Each of these was open, or answered differently by different sources,
before the sample checks. The file that settled it is named; its URL and
SHA-256 are in [test-data.md](test-data.md#files-checked).

- **Where the largest preview lives on ARW 4.0 and later**: a full-size
  JPEG in IFD2 of the IFD0 chain, in all 16 checked files at 4.0 or later
  (for example file 4466), not in a SubIFD; IFD0's ~1616×1080 before
  ([container-and-previews.md](container-and-previews.md)).
- **Raw tiles that look like JPEGs**: every lossless tile starts
  `FF D8 FF C3` (files 4466, 6929, 6930, 6931), and the tiles are stored
  column by column (file 4466)
  ([container-and-previews.md](container-and-previews.md),
  [raw-data.md](raw-data.md)).
- **FileFormat follows firmware, not body**: files 3751, 3251 and 6929
  write versions other than the published per-body tables, and file 4466
  (ILCE-1, firmware 1.00) writes ARW 4.0
  ([container-and-previews.md](container-and-previews.md)).
- **MakerNote headers**: the A100's ARW MakerNote has the `SONY DSC` header
  (file 2346), and the Hasselblad Lunar's is headerless with no "SONY"
  string in the file (file 4051)
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **0x9400 CameraOrientation in the 9400a layout**: byte 0x28 of the
  deciphered block holds 8 in the rotated SLT-A99V file, agreeing with
  IFD0 (file 3751), and 1 in the unrotated 9400a files 5811 and 4051. The
  offsets for 9400b (0x24) and 9400c (0x29) rest on ExifTool alone; IFD0
  Orientation remains the value to use
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **The order of 0x74C7**: (left, top), as LibRaw reads it, not (top, left)
  (files 3012, 3015) ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **Plain level copies and time-zone tags begin with ARW 2.3.3**, not ARW
  4.0: files 1541 and 1542 (ILCE-9) have both, files 1310 and 1311
  (ILCE-7RM2, ARW 2.3.1) neither
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **The SR2SubIFD key**: the bytes `11 22 33 44` on every checked ARW, from
  file 2271 to file 8846, with another key only on the R1 (file 3221)
  ([makernote-and-metadata.md](makernote-and-metadata.md)).
- **The A100 raw width**: 3881 columns, not 3880 (file 2346)
  ([raw-data.md](raw-data.md)).
- **ARW1 under Compression 32767**: written by the DSLR-A200, which
  explains the decoders' extra eight rows (file 2271)
  ([raw-data.md](raw-data.md)).
- **Level scaling**: 12-bit uncompressed data are stored at 14-bit scale
  (file 1311), while 12-bit packed data need the levels divided by 4 (file
  5858) ([raw-data.md](raw-data.md)).
- **SonyRawFileType does not mean what ExifTool says** (files 1311, 2271,
  4466), and BitsPerSample of cRAW can be 8, 12 or 14 (files 2521, 1310,
  1541) ([raw-data.md](raw-data.md)).
- **cRAW since ARW 2.0**: the DSLR-A700 writes it (file 2521), not only
  from ARW 2.1 ([raw-data.md](raw-data.md)).
- **SR2 and SRF data**: the R1's raw is plain big-endian 14-bit words (file
  3221), and the F828's decrypted data are 14-bit, not 12-bit (file 1351)
  ([raw-data.md](raw-data.md)).
- **The ILCE-7M5's two compressed modes**: both Compression 32766, told
  apart by MakerNote Quality, with one tile on full-frame files, not four
  (files 8846, 8847) ([raw-data.md](raw-data.md)).
- **ARQ layout**: four samples per pixel, Photometric 32892, R, G, G2, B,
  inside the source frame's unchanged container (files 3255, 3251)
  ([raw-data.md](raw-data.md),
  [container-and-previews.md](container-and-previews.md)).
