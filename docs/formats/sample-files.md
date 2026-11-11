# Sample files

Besides the small fixtures they build or commit, the tests read real
camera files, the sample files. This page says how a sample file comes
in and where its expected values are. It also says how those values
are made, and how anyone can check one on the file. The facts each file
shows stay in the format notes.

Where this page says *I*, it is me, Alvaro Kuolas, who maintains
raw-files.

## Contents

- [1. What the sample files are for](#1-what-the-sample-files-are-for)
- [2. How a sample comes in](#2-how-a-sample-comes-in)
- [3. The three tables](#3-the-three-tables)
- [4. Who makes the rows](#4-who-makes-the-rows)
- [5. How to check a row](#5-how-to-check-a-row)
- [6. If you are a contributor](#6-if-you-are-a-contributor)

## 1. What the sample files are for

The tests open each sample file and compare what raw-files reads with
tables of expected values. The files are too large to commit.
`scripts/fetch-samples.sh` downloads them and checks each one's SHA-256;
[CONTRIBUTING.md](../../CONTRIBUTING.md#code-style-and-checks) says how
to run it. A test whose file has not been downloaded says so and passes.

The list of the files is [`tests/samples.tsv`](../../tests/samples.tsv):
a header line, then one line per file, its columns separated by tabs.
It names 79 files: 71 RAW files from raw.pixls.us and 8 camera JPEGs
from Wikimedia Commons. Its columns:

| Column | What it holds |
|---|---|
| `source` | Where the file is published: `raw.pixls.us` or `commons` |
| `id` | The file's id there: a number on raw.pixls.us, the file's name on Wikimedia Commons |
| `format` | `nef`, `arw` or `jpeg` |
| `file` | The name the file is saved under, in `raw-samples/` |
| `tier` | `normal`, or `large` for a file fetched only with `RAW_FILES_LARGE=1` |
| `sha256` | The SHA-256 the download must have |
| `license` | The file's license, with its author where the license asks for credit |
| `url` | Where the file is downloaded from |

## 2. How a sample comes in

Anyone can propose a sample file, through an
[issue](https://github.com/kuolas/raw-files/issues/new/choose). Say in
it:

- the camera body, and what the file shows that no sample shows yet;
- where the file is published, with its URL;
- its license, as the place that publishes it states it.

Which licenses may enter is each format's rule, in section 1 of its
test-data page: [NEF](nef/test-data.md#1-sources-and-their-license-status),
[ARW](arw/test-data.md#1-sources-and-their-license-status) and
[JPEG](jpeg/test-data.md#1-sources-and-their-license-status).

I add the file's line to the list and its rows to the three tables.

## 3. The three tables

Each sample file has a row in each of three tables:

| Table | Type of a row | What a row holds |
|---|---|---|
| Previews | `Sample` | The orientation, and every preview, smallest first: its size, offset and length, and the bytes after its end marker |
| Metadata | `SampleMetadata` | Every value of `Metadata` the file must give, `None` where it must give none |
| Entries | `SampleEntries` | Every directory, in the order the reader finds it, with the number of its entries listed and left out |

The rows of each format are in its own test file:
[`tests/nef.rs`](../../tests/nef.rs),
[`tests/arw.rs`](../../tests/arw.rs) and
[`tests/jpeg.rs`](../../tests/jpeg.rs). In each, a test checks that the
list and each table name the same files, each in its tier. So a sample
without a row fails, even when no file has been downloaded. A `large`
file's previews and metadata have tables of their own, apart from the
`normal` files'. The one such file is an ARQ, in `tests/arw.rs`.

What each field of a row means is said once, in the docs of the types,
in [`tests/support/samples.rs`](../../tests/support/samples.rs). Those
docs also say how a row records what the repository must not hold. A
serial number is recorded by its shape. The texts of who took a picture
and the camera owner's name are recorded by which of them the file
gives. A location is recorded by which of its parts the file gives.

## 4. Who makes the rows

I generate the rows with scripts of my own, kept outside the
repository. They read the file's bytes by the rules the format notes
and the decision records give. Each part of a row is made one of three
ways:

- **Previews, metadata values, focus points and camera settings.**
  Read without raw-files, so that these values do not come from the
  code they test. The camera's maker and model names come from the
  [notes on camera names](camera-names.md).
- **Directories and their counts of entries.** Listed by raw-files, so
  a fault in its walk could reach the table. Each count was then
  checked against a walker of my own, written without raw-files from
  the TIFF 6.0 and Exif standards and the format notes.
- **Which texts and location parts a file gives.** Read from the
  entries raw-files lists. So these fields check the readers of the
  texts and the location, not the walk that lists the entries.

## 5. How to check a row

Each check below needs the downloaded file and a common tool, never
raw-files. The examples use `nikon-z9-lossless.nef`, and `xxd`, a hex
viewer.

### 5.1 A preview

A row gives each preview's offset and length in bytes, and its padding
(`after_eoi`), the bytes after its end marker.

- The two bytes at the offset are `FF D8`, the JPEG start marker.
- The two bytes before the padding, at the end of the length, are
  `FF D9`, the end marker.
- The preview's own bytes give its width and height. Copy them into a
  file of their own, and `file` or any image viewer shows the size.

The Z 9 file's smallest preview has offset 222,540, length 9,354 and no
padding:

```sh
xxd -s 222540 -l 2 nikon-z9-lossless.nef    # ffd8
xxd -s 231892 -l 2 nikon-z9-lossless.nef    # ffd9, at 222540 + 9354 - 2
tail -c +222541 nikon-z9-lossless.nef | head -c 9354 > preview.jpg
file preview.jpg                            # ... 160x120 ...
```

### 5.2 A directory's count of entries

A directory starts with its count of entries, two bytes in the file's
byte order. The file's first two bytes give that order: `II` is
little-endian and `MM` big-endian. Bytes 4 to 7 give IFD0's offset.

```sh
xxd -l 8 nikon-z9-lossless.nef         # 4949 2a00 0800 0000: II, IFD0 at 8
xxd -s 8 -l 2 nikon-z9-lossless.nef    # 1600: 22 entries
```

Each entry is 12 bytes: its tag, type, count, and value or offset. The
entry that leads to a directory gives its offset. In the Z 9 file,
IFD0's entry of tag 0x8769 gives the Exif IFD's offset, 0x8228. The two
bytes there are `29 00`, 41 entries.

In a JPEG file, offsets count from the TIFF header inside the Exif
segment, after `Exif` and two zero bytes. A MakerNote has its maker's
own layout, which the notes describe:
[NEF](nef/makernote-and-metadata.md#1-finding-the-makernote),
[ARW](arw/makernote-and-metadata.md#1-finding-the-makernote) and
[JPEG](jpeg/container-and-metadata.md#10-makernotes-in-a-jpeg).

### 5.3 A metadata value

Run another reader on the file to compare, such as exiv2 or ExifTool.
The clean-room rule allows this
([CONTRIBUTING.md](../../CONTRIBUTING.md#clean-room-and-third-party-code)).
exiv2's `-Pkv` prints each value as stored, so a fraction is not
reduced:

```sh
exiv2 -K Exif.Photo.ExposureTime -Pkv nikon-z9-lossless.nef    # 10/3200
```

Another reader may give a value that raw-files gives otherwise or not
at all. raw-files follows its own rules, which the format notes and the
decision records give. Say in the issue or pull request which reader
and version you ran.

## 6. If you are a contributor

A contributor
([CONTRIBUTING.md](../../CONTRIBUTING.md#becoming-a-contributor)) may
draft a sample's rows from what raw-files reads, and must say so in the
pull request. Before a row enters, I check it against an independent
read of the file.
