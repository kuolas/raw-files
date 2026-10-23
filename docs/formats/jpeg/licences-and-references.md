# JPEG: licences and references

Unlike a raw format, a JPEG file is described by published standards:
the JPEG standard itself, JFIF, Exif and the Multi-Picture Format. This
page records where each is published and on what terms, how the sample
files' licences are honoured, and how these notes keep to the clean-room
rule. Back to [the JPEG overview](README.md).

Confidence tags: **[P]** primary (read on the publisher's page or in the
document), **[S]** secondary, **[I]** inferred. Numbers in brackets point
to the [Sources](#sources) at the end. All pages were retrieved in
October 2026.

**This is not legal advice.** It is a record of what we found.

## 1. The standards

| Standard | Edition read | Availability | Conf. |
|---|---|---|---|
| JPEG: ITU-T T.81, the same text as ISO/IEC 10918-1 | T.81 (09/92), ISO/IEC 10918-1:1993 | The ITU offers it only for a fee; its page says it "is only available through payment". The W3C hosts the CCITT publication of 1992, which is the copy these notes read. ISO sells 10918-1. [1][2] | P |
| JFIF: ITU-T T.871, the same text as ISO/IEC 10918-5 | T.871 (05/2011), ISO/IEC 10918-5:2012 | Free from the ITU, listed among its freely available items. [3] | P |
| Exif: CIPA DC-008, technically equivalent to JEITA CP-3451H | Exif 3.1, CIPA DC-008-Translation-2026 (January 2026) | Free from CIPA, after accepting a disclaimer of CIPA's liability in a download form; it states no licence terms for implementing it. CIPA lists DC-008-2026 and DC-008-2024 (Exif 3.0). [4] | P |
| Multi-Picture Format: CIPA DC-007 | CIPA DC-007-Translation-2026 (its revision history lists 2009, 2021 and 2025) | Free from CIPA, through the same download form. [5] | P |

These standards describe the container and the metadata. raw-files
decodes no JPEG image data, so the coding processes of T.81 and any
patents on them do not concern it. [I] What a maker puts in its
MakerNote is not in any standard; the NEF and ARW notes cover it.

## 2. Other projects

No other project's source code was read for these pages: the facts come
from the standards, the sample files, the vulnerability records and
advisories that [security.md](security.md) cites, and the ExifTool output
described below. [I] Under the clean-room
rule ([ADR013](../../architecture-decisions/adr013-clean-room-and-third-party-code.md)),
no code, comment or table is taken from a JPEG or Exif library, whatever
its licence.

ExifTool was run on the first 300 KB of each candidate sample, as an
independent reader, while the samples were chosen
([test-data.md](test-data.md#3-how-the-files-were-chosen)). Its output
shortlisted the files, and what it showed of the files left out, and its
warning about the D70 file's IFD1, which that file's bytes then
explained, are recorded tagged **[S]**. It was also run as a reader on
some of the early Coolpix files checked by hand
([test-data.md](test-data.md#6-early-coolpix-files-checked-by-hand)),
to compare its reading of their MakerNotes with what their bytes show.
Every fact tagged **[F]** comes from the files' bytes, read with a
script written for the purpose. None of ExifTool's code or printed text
is copied into the repository. [I]

## 3. The sample files' licences

The eight Commons files ([test-data.md](test-data.md#2-the-sample-files))
are under CC0, a public-domain dedication, CC BY 2.0, CC BY 2.5 or CC BY
4.0. [P][6]

- **CC0** asks nothing of a user. [S][7] The D70 file's page declares it
  in the public domain by its author, a dedication that Commons records
  but that is weaker than CC0. [S][6]
- **CC BY** asks that a copy or adaptation credit the author, link the
  licence and say whether it was changed. [S][7] raw-files does not
  redistribute the files: `scripts/fetch-samples.sh` downloads them from
  Commons, in CI as elsewhere, pinned by their SHA-256. test-data.md
  records each file's author and licence all the same, as the list of
  samples, `tests/samples.tsv`, does, so that the credit travels with any
  reference to the file. [I] The ILCE-6000 file's page offers it under
  three licences, and raw-files takes CC BY 2.5 of them. [P][6]
- **CC BY-SA** files are not used, since share-alike terms would follow
  any adaptation of the data. [I]
- **The early Coolpix files checked by hand**
  ([test-data.md](test-data.md#6-early-coolpix-files-checked-by-hand))
  are not samples: nothing downloads them, and the repository holds
  nothing of theirs but their names, licences and hashes and facts about
  their structure, such as where a MakerNote lies and what its offsets
  count from. Facts are not the work a licence covers, so some of them
  are cited although they are under CC BY-SA, where no file of that body
  under another licence was found; each is credited all the same. [I]
- Commons keeps each upload byte for byte, and a later upload can replace
  the file behind the same name; the SHA-256 pin makes the download
  script notice, deleting the file it downloaded and failing, in CI as
  elsewhere. [S][6]

## 4. Clean-room guidance for raw-files

- **The standards are facts and expression.** Marker codes, field
  layouts, tag numbers and type codes are facts about the format, which
  these pages state in their own words, citing the section, table and
  page. The standards' prose is expression: quote at most a few words,
  where the exact wording matters. [I]
- **A sample file confirms a fact where it can**, and the pages say which
  file showed it. Where a file departs from the standard, as Sony's large
  thumbnail does, the pages follow the file. [I]
- **Another reader's output is a comparison, not a source**: it may be
  recorded where it was run, and confirmed on the file's bytes. [I] (see
  [ADR013](../../architecture-decisions/adr013-clean-room-and-third-party-code.md))

## Sources

[1] ITU, T.81 (09/92), https://www.itu.int/rec/T-REC-T.81-199209-I/en
    **[P]**
[2] W3C, CCITT T.81 (09/92) publication,
    https://www.w3.org/Graphics/JPEG/itu-t81.pdf **[P]**
[3] ITU, T.871 (05/2011), https://www.itu.int/rec/T-REC-T.871-201105-I/en
    **[P]**
[4] CIPA, download page for DC-008-Translation-2026,
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-008-2026-E
    **[P]**
[5] CIPA, download page for DC-007-Translation-2026,
    https://www.cipa.jp/std/documents/download_e.html?CIPA_DC-007-2026_E
    **[P]**
[6] Wikimedia Commons, the file pages linked in
    [test-data.md](test-data.md#2-the-sample-files), and
    "Commons:Reusing content outside Wikimedia",
    https://commons.wikimedia.org/wiki/Commons:Reusing_content_outside_Wikimedia
    **[P]** / **[S]**
[7] Creative Commons, the licence deeds of CC0 1.0, CC BY 2.0, 2.5 and
    4.0, https://creativecommons.org/publicdomain/zero/1.0/ ,
    https://creativecommons.org/licenses/by/2.0/ ,
    https://creativecommons.org/licenses/by/2.5/ ,
    https://creativecommons.org/licenses/by/4.0/ **[P]**
