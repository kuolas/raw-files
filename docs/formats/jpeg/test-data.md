# JPEG: test data

The JPEG tests combine small fixtures built in code, small fixtures
stripped from the maintainer's own camera JPEGs, and camera-original
JPEGs published on Wikimedia Commons, which `scripts/fetch-samples.sh`
downloads and checks against a hash, in CI as on a contributor's
machine. This page lists the sources, the eight Commons files, how they
were chosen and how they are downloaded, and the fixtures. Back to
[the JPEG overview](README.md).

Confidence tags: **[P]** primary (read on the file's page or in the
specification), **[S]** secondary, **[I]** inferred, **[F]** measured on
the file. Numbers in brackets point to the [Sources](#sources) at the end,
retrieved in October 2026.

## 1. Sources and their licence status

| Source | Licence status | Commit to the repository? |
|---|---|---|
| Wikimedia Commons | A licence per file, stated on its page: CC0, public-domain dedications, CC BY, CC BY-SA and others. Commons keeps the uploaded file as it was uploaded; only its scaled copies are re-encoded. [1] **[P]** / **[S]** | No: the files are several megabytes each. CC0, public-domain and CC BY files are downloaded, pinned by SHA-256, with the author and licence recorded for CC BY (section 2). CC BY-SA files are not used. |
| raw.pixls.us | CC0, but raw files only; it holds no camera JPEGs. [2] **[S]** | Not a source of JPEG files. |
| Camera makers' sample galleries | All rights reserved or non-commercial terms, as usually stated; the terms of each were not read. [I] | No. |
| Stock-photo sites (Unsplash, Pexels, Pixabay) | Licences of their own, not Creative Commons, and their downloads are re-encoded. [S] | No. |
| Files the maintainer made | The maintainer's to licence. | Yes, as stripped fixtures (section 4.2), with provenance in [`tests/fixtures/README.md`](../../../tests/fixtures/README.md). |
| Fixtures built in code | The project's own, MIT OR Apache-2.0. | Built by the tests; nothing to commit. |

The repository never holds a file whose licence is unclear, however small.

## 2. The sample files

Eight camera-original JPEGs, one per body, chosen to cover the Sony and
Nikon MakerNotes, a Multi-Picture Format segment with one image and with
two, a file with none, Sony's padding after the last image, and a maker
whose MakerNote raw-files does not read. Each was downloaded from its
original upload and walked byte by byte (section 3). Licences and
authors are as shown on each file's page, read when the files were
chosen, on 21 October 2026, and checked again on each file page on 22
October 2026, when they joined the downloaded set. [P]

| Body (firmware) | File on Wikimedia Commons | Licence, author | Bytes | SHA-256 |
|---|---|---|---|---|
| Sony ILCE-7M3 (v4.01) | [`Acanthodactylus_erythrurus_2025.jpg`](https://commons.wikimedia.org/wiki/File:Acanthodactylus_erythrurus_2025.jpg) | CC0, Bernatburriel | 8,323,072 | `9c0f5a9f3c5dc3070ab6a6eb45d535a943e94a4842fc5632cccc31324077531c` |
| Sony ILCE-7RM2 (v4.00) | [`Ulpu_Pajari_puhumassa_vastaanotettuaan_tiedonjulkistamisen_valtionpalkinnon_2.jpg`](https://commons.wikimedia.org/wiki/File:Ulpu_Pajari_puhumassa_vastaanotettuaan_tiedonjulkistamisen_valtionpalkinnon_2.jpg) | CC BY 4.0, Heikki Kastemaa | 13,762,560 | `0364ad38f5822045d3aaa289eb4ce60fbd0ef06ae5b2f523a4d55be5b5a6b7af` |
| Sony DSC-RX100M4 (v2.00) | [`여의도한강공원샛강생태공원문화다리_09.jpg`](https://commons.wikimedia.org/wiki/File:%EC%97%AC%EC%9D%98%EB%8F%84%ED%95%9C%EA%B0%95%EA%B3%B5%EC%9B%90%EC%83%9B%EA%B0%95%EC%83%9D%ED%83%9C%EA%B3%B5%EC%9B%90%EB%AC%B8%ED%99%94%EB%8B%A4%EB%A6%AC_09.jpg) | CC BY 4.0, Sunghyun | 8,488,968 | `67ff681c9942adcfb1b1a41883d42f31eb3fc5ee28b2a73121b9f6f118430efa` |
| Sony ILCE-6000 (v1.00) | [`140319_MS_Queen_Elizabeth_Kobe_Japan01o.JPG`](https://commons.wikimedia.org/wiki/File:140319_MS_Queen_Elizabeth_Kobe_Japan01o.JPG) | CC BY 2.5, 663highland | 7,798,784 | `532e7e5023ae42de29b28149f35ad8d76614fdd895d92d417352d2c837bda395` |
| Sony DSLR-A100 (v1.04) | [`"Krone"_flag_on_line_5_tram_(2564030031).jpg`](https://commons.wikimedia.org/wiki/File:%22Krone%22_flag_on_line_5_tram_%282564030031%29.jpg) | CC BY 2.0, Florian Prischl | 4,002,446 | `d77a7e6e9394a61326b6b51a8e9a08c1a3c4b3ddb4a9d22be54d4c0f8514ea98` |
| Nikon D5300 (Ver.1.02) | [`"Tours"_French_restaurant_in_Omotesando.JPG`](https://commons.wikimedia.org/wiki/File:%22Tours%22_French_restaurant_in_Omotesando.JPG) | CC0, Syced | 7,822,985 | `29e4f45b01092c2067bb09597bc46117b3cf7dbae1e035921878bae206a45223` |
| Nikon D70 (Ver.1.02) | [`"Stadswandeling".JPG`](https://commons.wikimedia.org/wiki/File:%22Stadswandeling%22.JPG) | public-domain dedication, Charles Henri | 4,178,961 | `5abe888aed3bc110361e38a78c43c8fb91b7e2808d6e53d37468e8e9704d6116` |
| Canon PowerShot G7 X | [`DC-3_OH-LCH_at_EFJM_20160717.jpg`](https://commons.wikimedia.org/wiki/File:DC-3_OH-LCH_at_EFJM_20160717.jpg) | CC BY 2.0, Timo Kuusela | 8,667,055 | `ce0b9b6b851d38ded8f32cf8d7a64d6424c5bd2a688c89c85fcdecb9e86dcd80` |

What each covers [F]:

| File | Covers |
|---|---|
| ILCE-7M3 | Sony MakerNote with the `SONY DSC` header; FocusLocation off centre, (1783, 1804) in a 3936×2624 frame, the size of its picture (the camera's M size); a camera XMP segment; MPF 1616×1080; 28,938 bytes of zeros after the last image |
| ILCE-7RM2 | FocusLocation at the exact centre, (3976, 2652) of 7952×5304; zeros after the last image |
| DSC-RX100M4 | FocusMode 0, manual focus, with a location at the centre: no focus point; no padding |
| ILCE-6000 | No FocusLocation: no focus point; a MakerNote entry (tag 0x2001) that points outside the Exif segment, into the MPF image |
| DSLR-A100 | The `SONY DSC` header before an IFD of 28 entries in an older layout; big-endian; no MPF |
| D5300 | Nikon MakerNote with its embedded TIFF header; AFInfo2 version 0100 with a zero frame, shot through the viewfinder: no focus point; the Exif segment one byte short of its maximum length; MPF with two images, 640×424 and 1620×1080 |
| D70 | Nikon MakerNote version 0210 with a PreviewIFD JPEG of 564×372; a 128×85 thumbnail; IFD1 shared with the Interoperability IFD; two DQT and four DHT segments; no MPF |
| G7 X | A MakerNote no reader reads; MPF 1620×1080 after 12 bytes that are not zeros |

What the file pages say of the licences, as checked on 22 October 2026
[P]:

- **ILCE-6000**: the page offers three licences to choose from, the GNU
  Free Documentation License 1.2 or later, CC BY-SA 3.0 and CC BY 2.5.
  raw-files uses the file under CC BY 2.5, with attribution to
  663highland.
- **DSC-RX100M4**: CC BY 4.0, credited to Sunghyun, with the author's own
  site given as the source. Commons marks the file as not yet reviewed
  by a licence reviewer, and its Exif Artist and Copyright fields name
  someone other than the person credited. It is kept, since it is only
  downloaded and never committed, with this status recorded here; it
  will be dropped if a review rejects its licence.
- **ILCE-7RM2**: the page says the camera was lent to the author, who is
  credited; the permission is confirmed through Wikimedia's volunteer
  response team.
- **G7 X** and **DSLR-A100**: both came from Flickr, and the review of
  each Flickr licence on Commons confirms CC BY 2.0.
- **D70**: released into the public domain by its copyright holder,
  Charles Henri, a weaker statement than CC0.
- **ILCE-7M3** and **D5300**: CC0, by Bernatburriel and by Syced.

### 2.1 How they are downloaded and checked

[`tests/samples.tsv`](../../../tests/samples.tsv), the list of every
downloaded sample, names each of the eight with the source `commons`,
its Commons file name as its id, the format `jpeg`, the name it is saved
under, its SHA-256, its licence, with the author for CC BY, and the URL
of its original upload. `scripts/fetch-samples.sh jpeg` downloads them,
about 60 MiB, into `raw-samples/`, or the directory `RAW_FILES_SAMPLES`
names, with the other samples; CI does the same. It sends Commons a
User-Agent that names the script and the project, as Wikimedia asks of
a script, and waits a moment between downloads, since quick successive
downloads failed (section 3); a download whose SHA-256 differs is
deleted, and the script fails. [F] The files are saved as:

| Body | Saved as |
|---|---|
| Sony ILCE-7M3 | `sony-ilce-7m3.jpg` |
| Sony ILCE-7RM2 | `sony-ilce-7rm2.jpg` |
| Sony DSC-RX100M4 | `sony-dsc-rx100m4.jpg` |
| Sony ILCE-6000 | `sony-ilce-6000.jpg` |
| Sony DSLR-A100 | `sony-dslr-a100.jpg` |
| Nikon D5300 | `nikon-d5300.jpg` |
| Nikon D70 | `nikon-d70.jpg` |
| Canon PowerShot G7 X | `canon-powershot-g7x.jpg` |

`tests/jpeg.rs` opens each and checks every preview the reader gives,
its size, place, format and orientation, read whole, the file's own
preview through the zeros after its last image; every value of its
metadata, serial numbers by their shape alone; and its focus point, or
that it has none. The expected values are tables generated from a walk
of each file's bytes, by the reader's rules, not typed in. A test whose
file has not been downloaded says so and passes, so the suite runs
offline. [F]

## 3. How the files were chosen

Commons was searched for JPEGs written by Sony bodies first, then by
other makers, with a CC0, public-domain or CC BY licence. [1] Each
candidate was downloaded from its original upload (`upload.wikimedia.org`,
with a descriptive User-Agent, which Wikimedia asks for, and pausing
between downloads, since quick successive downloads failed without an
error). Its SHA-1 was compared with the one Commons reports, so the file
is the upload byte for byte. [P][1][F]

A file was kept only when its bytes showed it as the camera wrote it [F]:

- the Exif segment is the first segment after SOI, with no JFIF segment;
- the Software tag names the camera's firmware, and the XMP segment, if
  any, names no editing program;
- the thumbnail and the MakerNote lie inside the Exif segment, and the
  MakerNote parses in its maker's layout;
- the frame header's size equals the Exif pixel dimensions and a size the
  body records natively.

The Software tag alone proves nothing: several rejected files kept the
camera's Software value but had been re-encoded, with a JFIF segment
first, no MakerNote, or an editor's XMP. [S] Commons' own page leaves the
MakerNote out of its metadata table, so a page that shows none says
nothing about the file. [P][1]

While choosing them, ExifTool was run on the first 300 KB of each
candidate as an independent reader, to shortlist files. What it showed of
the files left out (a JFIF segment first, an editor's XMP, a missing
MakerNote, a size that is not native, flagged MakerNote offsets) and its
warning about the D70 file's IFD1 are recorded on these pages tagged
**[S]**. The facts about the eight files tagged **[F]** come from walking
their bytes with the maintainer's own script. None of ExifTool's code or
printed text is copied into the repository.

Left out: three camera-original files under CC BY-SA (an ILCE-7RM3, a
DSC-RX100M5A and an SLT-A99V), since raw-files uses no share-alike
data; files whose pixel size is not a native one (cropped or resized);
and a CC0 SLT-A57 file whose MakerNote offsets ExifTool flags as possibly
moved by an editor. [P][S]

Not found with a usable licence: camera-original JPEGs from Fujifilm,
Panasonic and phones; the categories searched held none, and the search
was not exhaustive. [S]

## 4. Fixtures

The JPEG reader's tests use three kinds of fixture: files built in code,
three files stripped from the maintainer's own camera JPEGs, and damaged
copies of both. This section says what each kind is for; the tests' own
documentation says what each file holds.

### 4.1 Built in code

The tests build JPEG files in code (`tests/support/jpeg_builder.rs` and
`tests/support/jpeg_files.rs`): an Exif segment around the bytes of the
TIFF builder that the NEF and ARW tests use, with the Nikon and Sony
MakerNote builders inside, a JFIF segment, an XMP segment, an MPF
segment with its images appended, and padding after the last image. They
cover what no sample shows: a file without Exif, a damaged TIFF header,
a value or a thumbnail outside the Exif segment, a second Exif segment,
an MPF image past the end of the file, a lossless frame, a zero side in a
frame header, fill bytes, a segment length below 2, an Exif segment
behind a JFIF segment, and an export whose MakerNote frame does not fit
its picture. Some of these are tests of the segment walk alone, in
`src/container/jpeg/mod.rs`, and the rest open whole files, in
`tests/jpeg.rs`.

### 4.2 Stripped from the maintainer's own camera JPEGs

Three JPEGs from the maintainer's own Nikon Z 9, taken on 2026-10-22,
are committed in `tests/fixtures/`: one in FX, the camera's full output,
at full size, 8256×5504; one in DX crop mode at full size, 5392×3592; and
one in 16:9 crop mode at the middle size, 6192×3480, taken in portrait
orientation and stored with Orientation 8. A script stripped them. The
Exif segment is kept as the camera wrote it, with the serial numbers,
the shutter counts, the time the camera was switched on, the MakerNote's
encrypted blocks, Artist, Copyright, UserComment and the GPS values
zeroed in place, so that no offset moves;
the XMP segment is dropped; and every image (the main image, the
thumbnail and the two MPF images) is replaced by a stub JPEG of the same
frame size. Each file is 69,998 bytes, 65,536 of them the Exif segment.
They are published under MIT OR Apache-2.0 like the other fixtures, with
their provenance and SHA-256 in
[`tests/fixtures/README.md`](../../../tests/fixtures/README.md). [F]

### 4.3 Corrupted variants

As for NEF and ARW, the robustness tests (`tests/robustness.rs`) cut
every JPEG fixture short at every length and overwrite its bytes one at a
time, and expect an error or a result, never a panic. On the three
stripped files the overwriting skips three spans of zeros that no
reader parses: the MakerNote's zeroed ShotInfo, the padding after the
thumbnail's stub, and the camera's zeros after the thumbnail's declared
length, up to the end of the Exif segment.
The fuzz targets `jpeg_segments` and `open` start from copies of the
fixtures, in `fuzz/seeds/jpeg_segments/` (see
[security.md](security.md)).

## 5. What the samples do not cover

- **Exported files.** No exported JPEG is in the set: an export's
  pixels and its kept MakerNote are the case the focus rule of
  [ADR014](../../architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md)
  guards against, which the built fixtures cover. [I]
- **A portrait picture**: every file has orientation 1. [F]
- **A camera crop mode**, a Nikon JPEG with a CropArea, and a Nikon JPEG
  with a focus position: the D5300 shot through the viewfinder, and the
  D70 writes no AFInfo2. [F] The maintainer's own Z 9 shots show the
  crop modes
  ([ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)).
- **Fujifilm, Panasonic, Canon focus data and phones.** [S]

See [open-questions.md](open-questions.md) for what these gaps leave
open.

## Sources

[1] Wikimedia Commons, the file pages linked in section 2, and
    "Commons:Reusing content outside Wikimedia",
    https://commons.wikimedia.org/wiki/Commons:Reusing_content_outside_Wikimedia
    **[P]**
[2] raw.pixls.us, https://raw.pixls.us/ **[S]**
