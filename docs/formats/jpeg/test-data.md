# JPEG: test data

The JPEG tests combine small fixtures built in code, small fixtures
stripped from my own camera JPEGs, and camera-original JPEGs published
on Wikimedia Commons, which `scripts/fetch-samples.sh` downloads and
checks against a hash, in CI as on a contributor's machine. This page
lists the sources, the eight Commons files, how they were chosen and
how they are downloaded, the fixtures, and the early Coolpix files
that were checked by hand without joining the samples. Back to
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
| Files I made | Mine to licence. | Yes, as stripped fixtures (section 4.2), with provenance in [`tests/fixtures/README.md`](../../../tests/fixtures/README.md). |
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
their bytes with my own script. None of ExifTool's code or printed
text is copied into the repository.

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
four files stripped from my own camera JPEGs, and damaged copies of
both. This section says what each kind is for; the tests' own
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

### 4.2 Stripped from my own camera JPEGs

Three JPEGs from my own Nikon Z 9, taken on 2026-10-22, are committed
in `tests/fixtures/`: one in FX, the camera's full output, at full
size, 8256×5504; one in DX crop mode at full size, 5392×3592; and one
in 16:9 crop mode at the middle size, 6192×3480, taken in portrait
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

The JPEG of frame 4 of the session on which my own D600's autofocus
points were measured, shot on 2026-10-26 through the viewfinder with
the single point 38, is committed beside its NEF in
`tests/fixtures/nikon-d600/`, stripped the same way: the camera's full
output, 6016×4016, with Orientation 1, whose MakerNote's preview,
570×375, and one MPF image, 1632×1080, become stubs too. It is 69,876
bytes, 65,536 of them the Exif segment, which the D600 writes in
big-endian byte order. Its AFInfo2 names the point by number, which the
reader places from the D600's table as on the NEF
([autofocus-points.md](../nef/autofocus-points.md#2-nikon-d600)); its
provenance and SHA-256 are in
[the session's manifest](../../../tests/fixtures/nikon-d600/README.md).
[F]

### 4.3 Corrupted variants

As for NEF and ARW, the robustness tests (`tests/robustness.rs`) cut
every JPEG fixture short at every length and overwrite its bytes one at a
time, and expect an error or a result, never a panic. On the four
stripped files the overwriting skips three spans of zeros that no
reader parses: the MakerNote's zeroed ShotInfo, the padding after the
thumbnail's stub, and the camera's zeros after the thumbnail's declared
length, up to the end of the Exif segment; and, on the D600's, a
fourth, the padding after the MakerNote preview's stub.
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
  D70 writes no AFInfo2. [F] My own Z 9 shots show the crop modes
  ([ADR015](../../architecture-decisions/adr015-the-focus-point-of-a-crop-mode-jpeg.md)).
- **Fujifilm, Panasonic, Canon focus data and phones.** [S]
- **Some camera settings.** All eight files are sRGB and none had its
  flash fire; my Z 9 fixtures add a Nikon JPEG at a smaller image size
  ([container-and-metadata.md](container-and-metadata.md#10-makernotes-in-a-jpeg)). [F]

See [open-questions.md](open-questions.md) for what these gaps leave
open.

## 6. Early Coolpix files checked by hand

The Nikon MakerNote has two older layouts besides the one every sample
holds: the headerless one, a bare IFD, and the one with the short header
`Nikon\0\1\0`
([NEF MakerNote notes](../nef/makernote-and-metadata.md#1-finding-the-makernote)).
Early Coolpix bodies wrote those layouts into their JPEGs. To see how,
camera-original JPEGs of eleven early Coolpix bodies on Wikimedia
Commons, the E700, E800, E900, E950, E990, E995, E880, E885, E2500,
E4500 and E5000, were read byte by byte on 23 October 2026, with a
script written for the purpose that uses no other project's code. They
are evidence checked by hand, not samples: none is listed in
`tests/samples.tsv`, nothing downloads them, and no test reads them.

58 files were read, found in the Commons category of each body, each
downloaded from its original upload and compared with the SHA-1 that
Commons reports, as in section 3. [P] A file counts as the camera's when
its segments up to SOS are the ones these bodies write, the Exif segment,
one DQT segment, one DHT segment and the frame header, with nothing
else; Make is `NIKON` and Model names the body; Software names a
firmware version; the frame header's size equals the Exif pixel size and
is a size the body records; and the MakerNote lies inside the Exif
segment. [F] 47 files pass. The other 11 show an editor's hand, a
segment the camera does not write, such as an ICC profile or a JFIF,
Photoshop or XMP segment, or tables laid out otherwise, and are not
evidence for these pages, except one, cited below for what an editor
did to it.

The files the notes cite are listed here. Licences and authors are as
Commons' file information gave them, read through its API on 23 October
2026. [P]

| Body (firmware) | File on Wikimedia Commons | Licence, author | MakerNote | SHA-256 |
|---|---|---|---|---|
| E700 (v982p-71) | [`Pokfulam_Service_Reservoir_1.jpg`](https://commons.wikimedia.org/wiki/File:Pokfulam_Service_Reservoir_1.jpg) | CC BY-SA 3.0, Minghong | short header | `924ff0608031c2e7176fc095eaa2eda3f647a54177a30512d8e3d8e7cc26f737` |
| E800 (v984-75) | [`MIKI-eki.jpg`](https://commons.wikimedia.org/wiki/File:MIKI-eki.jpg) | public domain, released by its author, Deer in alcove | short header | `3a9c59bc9b0a8bed98302661a751d4001e6631fbde3055b93077862c78878882` |
| E800 (v984p-76) | [`Arlbergbahn_Kronburgabschnitt.jpg`](https://commons.wikimedia.org/wiki/File:Arlbergbahn_Kronburgabschnitt.jpg) | CC BY 3.0 AT, Siegele Roland | short header | `1f993b4969d02668d3a07f4787be7b98f9a355961eab289593565f4082f4cf04` |
| E900 (v88-74) | [`Yamaha_FJ1100.jpg`](https://commons.wikimedia.org/wiki/File:Yamaha_FJ1100.jpg) | public domain, released by its author | short header | `fa11fb20793ccd8eb16c8abc5b1543172afd00048fa7c2883a5061c13e195b47` |
| E900 (v88-74) | [`Petro_Star_refinery.jpg`](https://commons.wikimedia.org/wiki/File:Petro_Star_refinery.jpg) | public domain, a work of the US Department of Energy | short header | `d9a248163d093cb34d04745ad76e3d81d3198fe57a24a35e108fa5a9cadb8189` |
| E950 (v981-79) | [`Starr_020628-0045_Prosopis_pallida.jpg`](https://commons.wikimedia.org/wiki/File:Starr_020628-0045_Prosopis_pallida.jpg) | CC BY 3.0, Forest & Kim Starr | short header | `326864e25fc315b930e0ab17008420e6421f4520d094f26ae215d74376f6f604` |
| E950 (v981-79) | [`Bat_Week_2019_in_Utah_(48918908978).jpg`](https://commons.wikimedia.org/wiki/File:Bat_Week_2019_in_Utah_(48918908978).jpg) | public domain, a work of the US Bureau of Land Management | short header | `b13811c5b1711b4d1ad78ee1f759c12621db95dd05c896be638612f20913e7c1` |
| E990 (E990v1.1) | [`2006T629_finger_ring.jpg`](https://commons.wikimedia.org/wiki/File:2006T629_finger_ring.jpg) | CC BY 2.0, Portable Antiquities Scheme | headerless | `400441b2539e1cc1d48389148521caee4d862d5cd6d9d18d31abd5158fc80de1` |
| E990 (E990v1.1) | [`A_Fekete_Tisza_forrásvidéke_-_panoramio.jpg`](https://commons.wikimedia.org/wiki/File:A_Fekete_Tisza_forr%C3%A1svid%C3%A9ke_-_panoramio.jpg) | CC BY 3.0, Gyula Péter | headerless | `3d9b32b7e2a0b9491240389716f01f1595c949e3055978098b35d7d9d8a7d2b0` |
| E880 (E880v1.0) | [`Canoe_launch.jpg`](https://commons.wikimedia.org/wiki/File:Canoe_launch.jpg) | CC BY 2.0, rampartgeneral | headerless | `620b6f373e980f147be80d768faf9842ffb828b7fd28a15213a1c015c27ce607` |
| E880 (E880v1.1) | [`Εύηνος_-_panoramio.jpg`](https://commons.wikimedia.org/wiki/File:%CE%95%CF%8D%CE%B7%CE%BD%CE%BF%CF%82_-_panoramio.jpg) | CC BY 3.0, koupakia | headerless | `58212ff0595bc744b870b19e24e5eef719fc412ae24989823ce563c3603d0910` |
| E880 (E880v1.1) | [`ΜΑΛΑΚΑΣΑ_-_panoramio.jpg`](https://commons.wikimedia.org/wiki/File:%CE%9C%CE%91%CE%9B%CE%91%CE%9A%CE%91%CE%A3%CE%91_-_panoramio.jpg) | CC BY 3.0, koupakia | headerless | `fc3116db8e48966c90daf78a84b74cea5e77603975f972c92eacae7b73ffb88f` |
| E885 (E885v1.0) | [`Villa_borghese_-_George_Byron_1959_copia_da_Thorvaldsen.JPG`](https://commons.wikimedia.org/wiki/File:Villa_borghese_-_George_Byron_1959_copia_da_Thorvaldsen.JPG) | CC BY-SA 3.0, Lalupa | headerless, PreviewIFD | `19ee3a6542d127ce1b0ed8cef4aa1c22095f7d2aa107a99495b58afb2de7088d` |
| E885 (E885v1.1) | [`Ion_Oblemenco-Wappen.JPG`](https://commons.wikimedia.org/wiki/File:Ion_Oblemenco-Wappen.JPG) | CC BY-SA 3.0, Christopher Voitus | headerless, PreviewIFD | `025020bc857134fcf3a0c9e1fdc40dcd4f0c412994a3958f2ae5860b670fd75d` |
| E995 (E995v1.6) | [`2013-08-18_Cantharellus_minor_Peck_358951.jpg`](https://commons.wikimedia.org/wiki/File:2013-08-18_Cantharellus_minor_Peck_358951.jpg) | CC BY-SA 3.0, Jon (watchcat) at Mushroom Observer | headerless, PreviewIFD | `5530c3764b88dc94fab7f430dba742400ca6454b393e2ee6f6dc083a620dfae1` |
| E995 (E995v1.7) | [`9817_Narthecium_ossifragum.JPG`](https://commons.wikimedia.org/wiki/File:9817_Narthecium_ossifragum.JPG) | CC BY-SA 3.0, Deutne | headerless, PreviewIFD | `2178d0ac285788ea62795c43aac06801de6359427af65b6e056c013684597893` |
| E2500 (E2500v1.0) | [`Adastra_Park_-_S_Field_1.jpg`](https://commons.wikimedia.org/wiki/File:Adastra_Park_-_S_Field_1.jpg) | public domain, released by its author, Hassocks5489 | headerless, PreviewIFD | `c8051012b0c82e43c85783e98f6f7949a4a31b087bd5a2f23df590037782d3e3` |
| E2500 (E2500v1.0) | [`150019_and_158764_at_Worcester_Shrub_Hill.jpg`](https://commons.wikimedia.org/wiki/File:150019_and_158764_at_Worcester_Shrub_Hill.jpg) | CC BY 2.0, Matt Davis | headerless, PreviewIFD | `fa07eaf301b114d2eac5c97723ace12d92ad2d76728302e04eab1edcbc9f1afc` |
| E5000 (E5000v1.6) | [`03_Perunika_Glacier.JPG`](https://commons.wikimedia.org/wiki/File:03_Perunika_Glacier.JPG) | CC BY-SA 4.0, Àlex Simón i Casanovas | headerless, PreviewIFD | `68ef92fda462c48427438cef6febfd6478f897d89fa43835d1bd596354584e83` |
| E885 (E885v1.2) | [`3_silver_volcano(firework).JPG`](https://commons.wikimedia.org/wiki/File:3_silver_volcano(firework).JPG) | CC BY-SA 3.0, Christian Haselgrübler | embedded TIFF header | `9741a5bf184bb3c645dec0bd763bb6c7da2cc5be737ab0cde435b67c060d9a9f` |
| E2500 (E2500v1.1) | [`Acer_Travelmate_user.jpg`](https://commons.wikimedia.org/wiki/File:Acer_Travelmate_user.jpg) | public domain, released by its author, Love Krittaya | embedded TIFF header | `0be183e302b2ae77e09df7b2b923fbad3f111664fde6334c5a993a72714b64ff` |
| E5000 (E5000v1.7) | [`20030917_09_CTA_Blue_Line_UIC_Halsted_Station.jpg`](https://commons.wikimedia.org/wiki/File:20030917_09_CTA_Blue_Line_UIC_Halsted_Station.jpg) | CC BY 2.0, David Wilson | embedded TIFF header | `65ba15f0908429c209acdcc47b6d52b281c2504d2da6396d947c85386406c5f1` |
| E4500 (E4500v1.1) | [`Achnagavin_Bridge,_River_Aline_-_geograph.org.uk_-_1771428.jpg`](https://commons.wikimedia.org/wiki/File:Achnagavin_Bridge,_River_Aline_-_geograph.org.uk_-_1771428.jpg) | CC BY-SA 2.0, Peter Bond | embedded TIFF header | `e02424a26674d02dd28c96e88b1b3fd77b3f8a99b5bfc72485ab3626dc489ca5` |
| E885 (E885v1.1), turned by an editor | [`Priere-mosquee-saint-denis.JPG`](https://commons.wikimedia.org/wiki/File:Priere-mosquee-saint-denis.JPG) | public domain, released by its author, Thierry Caro | headerless, moved | `76d7f4d9f90d26a73c2a192aaf9c064eae269abfa612846091148db49825ee20` |

Where every camera-original file read of a body or a firmware version
is under CC BY-SA, as for the E700, E885, E995 and E4500 and the
E5000's v1.6, a file under CC BY-SA is cited; why such a file may be
cited is in
[licences-and-references.md](licences-and-references.md#3-the-sample-files-licences).
The E950 file `Bat_Week_2019_in_Utah_(48918908978).jpg` records
orientation 6 where the other E950 files record 1, which a viewer can
change in place without moving any byte; its segments and its MakerNote
are those of the other E950 files.

What the 47 camera files show [F]:

- **Byte order.** Every one is little-endian, `II`, in its Exif segment
  and in its MakerNote alike.
- **The short header** on all 18 files of the E700, E800, E900 and E950
  (5, 4, 4 and 5 files): the MakerNote starts `Nikon\0`, `01 00`, and an
  IFD follows those 8 bytes, of 11 entries, 5 on the E900, with a
  next-IFD pointer of 0 and no PreviewIFD (0x0011). Read big-endian, the
  count would be 2,816, or 1,280, a table longer than the MakerNote. The
  values stored out of line, five, two on the E900, all lie inside the
  MakerNote when their offsets count from the Exif segment's TIFF
  header, and none does when they count from the MakerNote's start or
  from the end of its 8 bytes.
- **The headerless layout** on all 18 files of the E990 (firmware v1.0
  and v1.1), E995 (v1.6 and v1.7), E880 (v1.0 and v1.1), E885 (v1.0 and
  v1.1), E2500 (v1.0) and E5000 (v1.6) (5, 3, 5, 2, 2 and 1 files): the
  MakerNote is a bare IFD from its first byte, of 16 entries on the
  E990, 17 on the E880 and 21 on the others, with a next-IFD pointer of
  0. Read big-endian, the count would be 4,096 or more. The 13 to 16
  values stored out of line all lie inside the MakerNote when their
  offsets count from the Exif segment's TIFF header, and none does when
  they count from the MakerNote's start.
- **The embedded TIFF header** on later firmware of three of those
  bodies, the E885 (v1.2), the E2500 (v1.1) and the E5000 (v1.7), and on
  every E4500 file (v1.1 to v1.3): `Nikon\0`, `02 00`, `00 00` and an
  inner `II` header. The layout follows the firmware, not the body.
- **The PreviewIFD outside the MakerNote.** On the headerless files of
  the E885, E995, E2500 and E5000, tag 0x0011 points at an IFD of 7
  entries at 1,764, inside the MakerNote, which is 954 bytes long from
  916; that IFD's JPEG, 320×240, starts at 14,397, 12,527 bytes past the
  MakerNote's end, and ends inside the Exif segment. All three offsets
  count from the Exif segment's TIFF header. The E990 and E880 files
  have no PreviewIFD.
- **The ISO tag** (0x0002) of every headerless MakerNote holds the two
  SHORTs `0 0`, so it records no ISO; the Exif segment of each of those
  files records one.

The file an editor turned, `Priere-mosquee-saint-denis.JPG`, shows why
offsets counted from outside the MakerNote are fragile: its headerless
MakerNote lies 342 bytes nearer the Exif segment's TIFF header than in
the camera's E885 files, at 574 rather than 916, and its offsets were
not moved with it. Every value stored out of line lies 342 bytes before
the place its entry points at, and the PreviewIFD pointer reaches no
IFD. [F]

What raw-files gives on them, opening each with the library [F]: every
one of the 58 files opens; each camera file gives two previews, the
160×120 thumbnail of IFD1 and the file itself, and the ISO of its Exif
segment where it records one other than 0. The headerless MakerNote is
read and adds nothing: its ISO tag records none, and its PreviewIFD's
JPEG lies outside the MakerNote, so the rule that a MakerNote's preview
lie inside the MakerNote refuses it
([open-questions.md](open-questions.md#1-decisions-for-the-maintainers),
item 3). The short-header MakerNote is not read.

ExifTool 13.55 was run on files of each layout as an independent reader,
to compare: it read the same layouts, numbers of entries and byte order.
[S] The facts above come from the files' bytes; none of ExifTool's code
or printed text is copied into the repository.

## Sources

[1] Wikimedia Commons, the file pages linked in section 2, and
    "Commons:Reusing content outside Wikimedia",
    https://commons.wikimedia.org/wiki/Commons:Reusing_content_outside_Wikimedia
    **[P]**
[2] raw.pixls.us, https://raw.pixls.us/ **[S]**
