# Camera names

A camera writes its maker and its model as text, such as `NIKON
CORPORATION` and `NIKON Z6_3`, and its maker sells it under another
name, Nikon's Z6III. raw-files gives both: the text as the file records
it, and the maker's and the model's names, which it reads from that
text through a table of its own
([ADR021](../architecture-decisions/adr021-what-the-file-records-and-what-raw-files-reads-into-it.md)).
This page records every entry of that table and where each fact comes
from, as
[ADR022](../architecture-decisions/adr022-names-as-their-makers-write-them.md)
asks: for each body, a file that shows its text and a document of its
maker that gives its name. For five Nikon bodies, it also gives the
sensor's format and whether the body offers raw sizes, from the maker's
documents (§5). The code is written from this page.

Where this page says *I* or *my own* files, it is me, Alvaro Kuolas,
who maintains raw-files, writing about my own cameras and shots: they
stay private, and are cited by counts or by frame numbers.

The page serves every format raw-files reads: a body writes the same
Make and Model texts in its raw files and its JPEGs.

## 1. How the facts were gathered

**The texts.** Every Make and Model text below is one that raw-files
reads from the files named, cleaned as every text value is: cut at the
first NUL and trimmed of spaces at both ends. The tests check each of
them but the five cited from my own files alone: the ILCE-7M2 and
four Nikon DSLRs. The tables of the public samples
([NEF](nef/test-data.md), [ARW](arw/test-data.md),
[JPEG](jpeg/test-data.md#2-the-sample-files)) expect each file's Make
and Model, and the fixtures' tests their Model. ExifTool, run on the same files as a
reader to compare with on 30 October 2026, reported the same texts.
Numbers such as "file 5146" are raw.pixls.us ids; the camera JPEGs are
named as `tests/samples.tsv` names them. My own files are cited by
count. The ILCE-7M2 is the one body that no sample or fixture shows.
My own files of it were read on 2 November 2026 by raw-files and by
ExifTool as a reader. My own files of the D300S, D3200, D3300 and
D7100 were read on 20 November 2026 by raw-files alone.

**The names.** Every name was read from the maker's own published
documents. A product page or manual page on the maker's website was
fetched and read as text, with the name checked in the page's title.
In a manual in PDF, the name was checked on the page given. The names
of the D300S, D3200, D3300 and D7100 were read on 20 November 2026, the
others on 30 October 2026. Where a maker's documents spell a name
differently, the name its product pages give is the one taken, and the other spellings are
recorded beside it (ADR022). No name is taken from another reader's
table, a retailer, a review or an encyclopedia.

**The sensor formats and raw sizes.** Each fact in §5 was read on 20
November 2026 from the body's manual in PDF, as Nikon's download center
(downloadcenter.nikonimglib.com) gives it. The sensor format is the one
the manual's specifications name, Nikon's FX or DX. Whether the body
offers raw sizes was read from the manual's page on image size. That
page says whether the size chosen applies to NEF (RAW) images. No fact
is taken from a file, another reader's table or any other source.

**The characters.** Every space in a name is an ordinary space, U+0020,
as on the pages read. Sony's α is the Greek small letter alpha, U+03B1.

## 2. Makers

A maker is recognized from its whole Make text, case included; any
other text, such as `Sony` or `NIKON` followed by more words, names no
maker.

Which maker's reader opens a file is decided by a looser test: a Make
that starts with `NIKON` or `SONY`, in any case, once trimmed, or, in a
raw file that holds Sony's private pointer, `HASSELBLAD`. A file whose
Make is, say, `Nikon` is read as a Nikon file and has no maker here. None of the samples, the fixtures or my own files read for this
page writes such a text. The local corpus test counts, in each format's
section, the files whose recorded Make names no maker, so such a file
shows there; its text enters this table from that file, by the rule of
this page.

| Make text | Files | Name | Maker's document |
|---|---|---|---|
| `NIKON CORPORATION` | every Nikon raw sample but the two below, the D5300 and D70 camera JPEGs, the fixtures of my own D600 and Z 9, my own D300S, D3200, D3300 and D7100 files | Nikon | [nikon.com](https://www.nikon.com/), title "Nikon" |
| `NIKON` | files 4456 (Coolpix E5700) and 1425 (Coolpix P7800) | Nikon | as above |
| `SONY` | the Sony raw samples from the DSC-F828 to the ILCE-7M5, the five Sony camera JPEGs | Sony | [sony.net](https://www.sony.net/), title "Sony Corporation - Home" |
| `HASSELBLAD` | file 4051 (Lunar) | Hasselblad | [hasselblad.com](https://www.hasselblad.com/), title "Hasselblad" |
| `Canon` | `canon-powershot-g7x.jpg` | Canon | [global.canon](https://global.canon/en/), title "Canon Global" |

## 3. Nikon bodies

Nikon writes its older Z bodies with a space between the Z and the
rest, Z 9, Z 6, Z 30, Z f and Z fc, in their manuals and on its product
pages alike, and its newer ones without, Z6III, Z50II and Z5II. The
names follow Nikon body by body. The one other spelling found is
"Z9", in the FCC notice of the Z 9 Reference Guide (section "Notices").

| Model text | Files | Name | Maker's documents |
|---|---|---|---|
| `NIKON D600` | the frames of my own D600 in `tests/fixtures/nikon-d600/` | D600 | *D600 User's Manual*, 6MB15311-01, page 326 (specifications): "Nikon D600 Digital Camera" |
| `NIKON Z 9` | files 5146 to 5148; the fixtures of my own Z 9 | Z 9 | *Z 9 Reference Guide*, for firmware 5.20, page 951 (specifications): "Nikon Z 9 Digital Camera"; [online manual](https://onlinemanual.nikonimglib.com/z9/en/), title "Z 9 Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z_9/), title "Z 9 \| Mirrorless Cameras \| Nikon Consumer"; [Nikon USA](https://www.nikonusa.com/p/z-9/1669/overview), title "Nikon Z 9 Mirrorless Camera …" |
| `NIKON Z 6` | file 3587 | Z 6 | [online manual](https://onlinemanual.nikonimglib.com/z7_z6/en/), title "Z 7/Z 6 Online Manual"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z_6/), title "Z 6 \| Mirrorless Cameras …" |
| `NIKON Z 30` | file 5812 | Z 30 | [online manual](https://onlinemanual.nikonimglib.com/z30/en/), title "Z 30 Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z_30/), title "Z 30 \| Mirrorless Cameras …" |
| `NIKON Z f` | file 6887 | Z f | [online manual](https://onlinemanual.nikonimglib.com/zf/en/), title "Z f Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z_f/), title "Z f \| Mirrorless Cameras …"; [Nikon USA](https://www.nikonusa.com/p/z-f/1761/overview), title "Nikon Z f Mirrorless Camera …" |
| `NIKON Z fc` | file 8559 | Z fc | [online manual](https://onlinemanual.nikonimglib.com/zfc/en/), title "Z fc Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z_fc/), title "Z fc \| Mirrorless Cameras …" |
| `NIKON Z50_2` | files 7767 and 7770 | Z50II | [online manual](https://onlinemanual.nikonimglib.com/z50II/en/), title "Z50II Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z50_2/), title "Z50II \| Mirrorless Cameras …" |
| `NIKON Z5_2` | files 7735 and 7736 | Z5II | [online manual](https://onlinemanual.nikonimglib.com/z5II/en/), title "Z5II Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z5_2/), title "Z5II \| Mirrorless Cameras …" |
| `NIKON Z6_3` | files 7808 to 7810, 7812 and 7813 | Z6III | [online manual](https://onlinemanual.nikonimglib.com/z6III/en/), title "Z6III Reference Guide"; [product page](https://imaging.nikon.com/imaging/lineup/mirrorless/z6_3/), title "Z6III \| Mirrorless Cameras …" |

## 4. Sony bodies

Sony's photo gallery on sony.net, its marketing site for its cameras,
has a page for each body, whose title begins "Sony Corporation - Photo
Gallery |" and then the name, with a space before the numeral: α7 IV.
The Help Guide of each body, its manual on helpguide.sony.net, gives
the name on its first page without that space, α7IV, or, for the older
bodies, only as an α logo. The names follow the gallery; the Help
Guide's spelling is in the last column.

| Model text | Files | Name | Gallery page | Help Guide's first page |
|---|---|---|---|---|
| `ILCE-1` | file 4466 | α1 | [ILCE-1](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-1/) | [α1](https://helpguide.sony.net/ilc/2040/v1/en/index.html) |
| `ILCE-1M2` | file 7835 | α1 II | [ILCE-1M2](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-1M2/) | [α1II](https://helpguide.sony.net/ilc/2440/v1/en/index.html) |
| `ILCE-6000` | `sony-ilce-6000.jpg` | α6000 | [ILCE-6000](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-6000/) | [logo only](https://helpguide.sony.net/gbmig/45349331/v1/en/index.html) |
| `ILCE-6700` | file 6736 | α6700 | [ILCE-6700](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-6700/) | [α6700](https://helpguide.sony.net/ilc/2320/v1/en/index.html) |
| `ILCE-7M2` | 76 ARW files and 72 JPEGs of my own | α7 II | [ILCE-7M2](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7M2/) | [logo only](https://helpguide.sony.net/ilc/1450/v1/en/index.html) |
| `ILCE-7M3` | `sony-ilce-7m3.jpg` | α7 III | [ILCE-7M3](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7M3/) | [α7III](https://helpguide.sony.net/ilc/1720/v1/en/index.html) |
| `ILCE-7M4` | files 6929 to 6931, 6933 and 6936 | α7 IV | [ILCE-7M4](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7M4/) | [α7IV](https://helpguide.sony.net/ilc/2110/v1/en/index.html) |
| `ILCE-7M5` | files 8846 to 8850 | α7 V | [ILCE-7M5](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7M5/) | [α7V](https://helpguide.sony.net/ilc/2540/v1/en/index.html) |
| `ILCE-7RM2` | files 1310 and 1311, `sony-ilce-7rm2.jpg` | α7R II | [ILCE-7RM2](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7RM2/) | [logo only](https://helpguide.sony.net/ilc/1520/v1/en/index.html) |
| `ILCE-7RM3` | files 3251 and 3255 | α7R III | [ILCE-7RM3](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7RM3/) | [α7RIII](https://helpguide.sony.net/ilc/1710/v1/en/index.html) |
| `ILCE-7S` | file 1582 | α7S | [ILCE-7S](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7S/) | [logo only](https://helpguide.sony.net/ilc/1420/v1/en/index.html) |
| `ILCE-7SM3` | file 5418 | α7S III | [ILCE-7SM3](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-7SM3/) | [α7SIII](https://helpguide.sony.net/ilc/2010/v1/en/index.html) |
| `ILCE-9` | files 1541 and 1542 | α9 | [ILCE-9](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-9/) | [α9](https://helpguide.sony.net/ilc/1650/v1/en/index.html) |
| `ILCE-9M3` | files 7361 and 7364 | α9 III | [ILCE-9M3](https://www.sony.net/Products/di_photo-gallery/camera/ILCE-9M3/) | [α9III](https://helpguide.sony.net/ilc/2380/v1/en/index.html) |

## 5. Sensor format and raw sizes

Some Nikon files record neither an image area nor a raw size. raw-files
will then take both from the body's row below
([ADR011](../architecture-decisions/adr011-metadata-from-standard-tags.md)):

- Where Nikon's crop code is "off" or missing, the image area will be
  the body's sensor format.
- Where a raw file records no raw size and the body offers none, the
  raw size will be large.

Each row joins a body to the manual that states its two facts.
The other bodies of §3 and §4 have no row yet, so this table gives them
neither fact.

| Model text | Sensor format | Raw sizes | Maker's document |
|---|---|---|---|
| `NIKON D600` | FX | none | *D600 User's Manual*, 6MB15211-06, pages 326 (specifications) and 94 (image size) |
| `NIKON D300S` | DX | none | *D300S User's Manual*, 6MB05611-06, pages 385 (specifications) and 69 (image size) |
| `NIKON D3200` | DX | none | *D3200 Reference Manual*, 6MB22611-03, pages 196 (specifications) and 45 (image size) |
| `NIKON D3300` | DX | none | *D3300 Reference Manual*, 6MB19911-03, pages 342 (specifications) and 86 (image size) |
| `NIKON D7100` | DX | none | *D7100 User's Manual*, 6MB18111-04, pages 336 (specifications) and 67 (image size) |

The D300S, D3200, D3300 and D7100 have no row in §3 yet, so raw-files
gives them no name today. Each will join §3 when raw-files reads this
table, with my own files of it and the name its manual gives:

| Model text | Files | Name | Maker's documents |
|---|---|---|---|
| `NIKON D300S` | 647 NEF files and 1,033 JPEGs of my own | D300S | *D300S User's Manual*, 6MB05611-06, page 385 (specifications): "Nikon D300S Digital Camera"; [download center](https://downloadcenter.nikonimglib.com/en/products/12/D300S.html), title "Nikon \| Download center \| D300S" |
| `NIKON D3200` | 3,568 NEF files and 6,829 JPEGs of my own | D3200 | *D3200 Reference Manual*, 6MB22611-03, page 196 (specifications): "Nikon D3200 Digital Camera"; [download center](https://downloadcenter.nikonimglib.com/en/products/20/D3200.html), title "Nikon \| Download center \| D3200" |
| `NIKON D3300` | 659 JPEGs of my own | D3300 | *D3300 Reference Manual*, 6MB19911-03, page 342 (specifications): "Nikon D3300 Digital Camera"; [download center](https://downloadcenter.nikonimglib.com/en/products/21/D3300.html), title "Nikon \| Download center \| D3300" |
| `NIKON D7100` | 400 NEF files and 2,707 JPEGs of my own | D7100 | *D7100 User's Manual*, 6MB18111-04, page 336 (specifications): "Nikon D7100 Digital Camera"; [download center](https://downloadcenter.nikonimglib.com/en/products/27/D7100.html), title "Nikon \| Download center \| D7100" |

The D600's specifications name its format beside the image sensor. The
others name it beside the angle of view. "None" means the image size
chosen applies to JPEG images alone, or also to TIFF on the D300S. Each
manual says so: a NEF (RAW) image keeps one size, whatever the setting.
The D600's facts come from the edition Nikon's download center gives,
6MB15211-06; §3 cites the edition 6MB15311-01 for its name.

## 6. Bodies without a name

Every body not listed above has no name in raw-files: the program
reading its file gets the recorded text alone. The table holds:

- the Nikon Z bodies and Sony ILCE bodies the public samples and
  fixtures show;
- my own D600 and ILCE-7M2.

The other bodies the samples show wait for an entry of their own, as
do the four DSLRs of my own files that §5 names:

- Nikon's other DSLRs, Sony's DSLRs, and both makers' compacts;
- Sony's SLT-A55V and SLT-A99V;
- three more mirrorless bodies, Nikon's 1 J1, Sony's NEX-5 and
  Hasselblad's Lunar;
- Canon's PowerShot G7 X.
