# Roadmap

The big picture: what raw-files aims for, what it reads so far, and
what comes next. Bugs and smaller requests live in the
[issue tracker](https://github.com/kuolas/raw-files/issues).

Where this roadmap says *I* or *my own*, it is me, Alvaro Kuolas, who
maintains raw-files, writing about my own cameras. Their files stay
private.

## The goal

raw-files aims for a well-made API: ergonomic, idiomatic Rust that
follows the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and
that a person can read and understand. It strives for an API that is
complete, not for support of every camera body. Cameras of different
makers record much the same things, so one API can serve them all
without every body being read first.

The work aims first at mirrorless cameras, such as Nikon Z, Sony
E-mount Alpha and Canon EOS R bodies. Bodies are added as the
opportunity comes: when I have one at hand, or when a contributor sends
files of it. No camera's support is promised, and none is scheduled.

## Capabilities

What raw-files gives is grouped in five capabilities: previews,
metadata, focus points, raw sensor data and the developed image.
Together they are the whole of what it means to give. Decoding ends at
the developed image, and the creative edit stays with the application
([ADR010](architecture-decisions/adr010-decoding-ends-at-a-linear-image.md)).

A capability is a lasting area of work, not a step towards a finish.
Each grows with every body and format it learns to read, so none is
ever complete for every body.

Each capability raw-files gives starts from `CameraFile`, the handle
on an open file, and names the methods that give it. The
[API documentation](https://docs.rs/raw-files) gives each type's rules,
and the [format notes](formats/README.md) what each body records.

### Previews

The JPEG previews a file holds, returned without decoding the raw data.
`CameraFile::previews` lists them, and `CameraFile::read_preview` reads
one. Each format's notes say where its previews lie:
[NEF](formats/nef/container-and-previews.md),
[ARW](formats/arw/container-and-previews.md) and
[JPEG](formats/jpeg/container-and-metadata.md).

### Metadata

What a viewer or a file renamer needs next to the preview, as
`CameraFile::metadata` gives it. The README's
[What it reads](../README.md#what-it-reads) lists every value, and
`CameraSettings` says which settings a maker's files leave out.

- **Where it comes from.** The standard TIFF and Exif tags, and the
  MakerNote for what only the MakerNote records. A camera JPEG's comes
  from its Exif segment, and from its MakerNote on Nikon and Sony
  cameras.
- **What it is checked on.** The files each format's notes list, such
  as the
  [NEF metadata section](formats/nef/makernote-and-metadata.md#8-viewer-metadata).

Under way, and next: **full metadata**, every metadata value a file
records. A program then needs no second reader beside raw-files. It
comes before raw sensor data
([ADR016](architecture-decisions/adr016-full-metadata-before-raw-data.md)).

raw-files gives these steps of it:

1. **Every entry a file records**, from `CameraFile::entries`. That is
   each entry of the TIFF directories raw-files walks, as the file
   records it
   ([ADR023](architecture-decisions/adr023-every-entry-a-file-records.md)).
   A `Directory` names each directory walked. An entry's tag comes with
   the name a published specification gives it, where one does
   ([the tag names](formats/tag-names.md)).
2. **Who made a picture**, from `Metadata::authorship` and
   `Camera::owner_name`: the artist, the copyright notice and the other
   texts `Authorship` lists, and the camera owner's name.
3. **Where it was taken**, from `Metadata::location`, with the parts
   `Location` lists.
4. **More of how the camera was set**, from `Metadata::camera_settings`,
   with the settings `CameraSettings` lists, and
   `Metadata::digitized_time`.

Steps 2 and 3 read the standard tags alone
([ADR011](architecture-decisions/adr011-metadata-from-standard-tags.md),
[#97](https://github.com/kuolas/raw-files/issues/97)). They are
checked on the files the format notes count. A text no measured file
records is checked on files built in the tests
([NEF notes, section 8.8](formats/nef/makernote-and-metadata.md#88-authorship)).

Step 4 adds these values
([ADR011](architecture-decisions/adr011-metadata-from-standard-tags.md),
[#214](https://github.com/kuolas/raw-files/issues/214)):

- on `CameraSettings`: the focus mode, the AF-area mode, AF fine-tune,
  the stabilization, auto ISO, the exposure fine-tuning, the release
  mode, bracketing, daylight saving time, the raw bit depth and
  compression, and the JPEG quality and compression. The AF-area mode
  is given whether or not a file gives a focus point;
- on `Flash`: its red-eye reduction, return light, source, control,
  sync and compensation;
- on `WhiteBalance`: its preset slot and fluorescent lamp;
- on `Metadata`: the digitized time, beside the capture time.

A Nikon body that records no image area or raw size gets them from a
table of bodies, for the bodies the table holds. Each fact in it comes
from the maker's published specification
([ADR022](architecture-decisions/adr022-names-as-their-makers-write-them.md),
[the table](formats/camera-names.md)). The format notes say what each
file records and where each code's meaning comes from
([NEF](formats/nef/makernote-and-metadata.md#810-more-capture-settings),
[ARW](formats/arw/makernote-and-metadata.md#910-more-capture-settings)).
A list of codes that published documentation gives is read whole once
the files hold part of it, and the notes mark each code as confirmed by
a frame, observed on files, or documented only.

**How the camera made its JPEG** is read too, as values of
`CameraSettings`
([ADR011](architecture-decisions/adr011-metadata-from-standard-tags.md),
[#230](https://github.com/kuolas/raw-files/issues/230)).

Exif's contrast, saturation, sharpness, scene capture type, custom
rendering and gain control are read for any maker, from Exif alone. The
picture style is read from each maker's MakerNote: a `PictureStyle`
holds Nikon's Picture Control, or Sony's Creative Style or Creative
Look, with its values. So are five more values, each holding either
maker's setting. A `DynamicRangeOptimization` holds Nikon's Active
D-Lighting or Sony's D-Range Optimizer. A `HighDynamicRange` holds
Nikon's HDR or Sony's Auto HDR. The noise reductions are read as well:
the high ISO one, in one list of both makers' steps, the long exposure
one, and Sony's multi-frame one. A `VignettingCorrection` holds Nikon's
vignette control or Sony's shading compensation, and a
`DistortionCorrection` Nikon's auto distortion control or Sony's
distortion compensation; Sony's chromatic aberration compensation is
read too. A `Scene` holds Nikon's scene mode, as the camera writes it,
or Sony's Scene Selection.

Each of these settings keeps its maker's own name and kinds, as the
picture style does. Where two makers each have a setting for one thing,
one value holds either maker's, in the picture style's shape. The
format notes say what each file records
([NEF](formats/nef/makernote-and-metadata.md#811-picture-style-and-processing),
[ARW](formats/arw/makernote-and-metadata.md#911-picture-style-and-processing),
[JPEG](formats/jpeg/container-and-metadata.md#4-exif-the-app1-segment)).

**What a photographer or a program adds** comes next
([ADR024](architecture-decisions/adr024-xmp-and-iptc.md),
[#237](https://github.com/kuolas/raw-files/issues/237)). That is the
rating, the label, and the IPTC fields, such as the title, the keywords
and the creator.

- raw-files will read them from the XMP packet a file embeds and from
  its older IPTC block, each given apart.
- A caller will hand in an XMP sidecar, by its path, its bytes or a
  reader; opening a file will still read that file alone.
- Every XMP property will be given too, by its namespace and name.

The [XMP and IPTC notes](formats/xmp-and-iptc.md) say what files hold.

These groups of values follow, each read from the same directories:

- the lens by name;
- what editing software, such as Nikon's, writes into the file;
- what a maker records only in enciphered or encrypted blocks. Reading
  these needs a decision of its own first, and they are given only as
  their bytes until then.

### Focus points

Where the camera focused, in coordinates that place it on a preview, to
judge sharpness where the photographer focused. `Metadata::focus` gives
it, and its docs say which shots give none.

- **Read from the MakerNote** of Nikon mirrorless bodies, Nikon DSLRs
  in live view and Sony bodies from the ILCE-7RM2 on. With the point
  come the points the camera used.
- **From a camera JPEG**, of a Nikon or Sony body, only when the
  point's frame has the picture's shape.
- **From a table of the body's points**, for a shot that records only
  a point's number, such as a DSLR's through the viewfinder. The project
  measures each table on the body itself
  ([ADR017](architecture-decisions/adr017-per-body-tables-measured-by-the-project.md),
  [ADR018](architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)).

The Nikon D600 is the first body measured. Its shots through the
viewfinder give their points placed from its table, and its files the
layout of its 39 points, also placed on each picture, a crop's
included. The table is checked on frames of my own D600,
stripped of their pixels
([the session](formats/nef/autofocus-points.md#21-the-session)).

A shot of another body that records only a number gives no point. A
body's table joins when I have the body at hand, or when a contributor
sends RAW + JPEG frames of it to measure.

### Raw sensor data

The sensor data of a raw file, decoded. Not started.

### Developed image

The sensor data developed to a linear image, plus the color model that
converts it to a standard color space. Not started.

## Formats

raw-files reads three formats, and refuses any other file with the
`UnknownFormat` error:

- **NEF**, Nikon's raw format, with the NRW files of Coolpix cameras
  ([format notes](formats/nef/README.md)).
- **ARW**, Sony's raw format, with the SRF, SR2 and ARQ files of the
  same family and the ARW files of Hasselblad's Sony-built bodies
  ([format notes](formats/arw/README.md)).
- **JPEG**, as a camera writes it beside its raw file or instead of it
  ([ADR014](architecture-decisions/adr014-jpeg-files-as-a-format-of-their-own.md),
  [format notes](formats/jpeg/README.md)).

The README's [Formats](../README.md#formats) lists the files each format
is checked on, and which of them CI downloads.

Other formats come in this order:

1. **DNG**, Adobe's published raw format, also written by phones.
2. **Canon**: CR3, then CR2.
3. **Fujifilm**: RAF.

Here too the work aims first at mirrorless bodies. So Canon's CR3, the
format of its EOS R bodies, comes before CR2. DNG comes first: CR3 is
the more complex format to read. The order is a plan, not a promise. A
change of direction is recorded in the architecture decisions.

## Releases

One line per release. The [CHANGELOG](../CHANGELOG.md) has each
release's changes.

- **0.1.0**: previews of NEF and ARW files.
- **0.2.0**: their metadata, most of what a viewer or a file renamer
  needs.
- **0.3.0**: focus points, the position the camera records.
- **0.4.0**: plain JPEG files, as a third format
  ([#23](https://github.com/kuolas/raw-files/issues/23)).
- **0.5.0**: the camera's settings
  ([#26](https://github.com/kuolas/raw-files/issues/26)).
- **0.6.0**: a DSLR's autofocus point, placed from a table of the
  body's points ([#57](https://github.com/kuolas/raw-files/issues/57)).
- **0.7.0**: the body's maker and sales name, plain camera and lens
  values, and two D600 focus changes
  ([#49](https://github.com/kuolas/raw-files/issues/49),
  [#33](https://github.com/kuolas/raw-files/issues/33),
  [#62](https://github.com/kuolas/raw-files/issues/62),
  [#63](https://github.com/kuolas/raw-files/issues/63)).
- **0.8.0**: every entry a file records, the first step of full
  metadata ([#50](https://github.com/kuolas/raw-files/issues/50)).
- **0.9.0**: who made a picture and where it was taken, the second
  step of full metadata
  ([#97](https://github.com/kuolas/raw-files/issues/97)).
- **0.10.0**: the API made consistent before full metadata is built on
  it ([#102](https://github.com/kuolas/raw-files/issues/102),
  [#104](https://github.com/kuolas/raw-files/issues/104),
  [#105](https://github.com/kuolas/raw-files/issues/105),
  [#107](https://github.com/kuolas/raw-files/issues/107),
  [#150](https://github.com/kuolas/raw-files/issues/150)).
- **On master, not released**: what the CHANGELOG lists under
  [Unreleased](../CHANGELOG.md#unreleased), among it the handle named
  `CameraFile`, the layout of a body's points placed on the picture,
  more capture settings
  ([#214](https://github.com/kuolas/raw-files/issues/214)), and how the
  camera made its JPEG
  ([#230](https://github.com/kuolas/raw-files/issues/230)).
- **Next**: more of full metadata, group by group
  ([#50](https://github.com/kuolas/raw-files/issues/50)), beginning with
  the XMP and IPTC fields
  ([#237](https://github.com/kuolas/raw-files/issues/237)). The values
  only enciphered or encrypted blocks hold come last.

## Open questions

- **Which formats after Fujifilm.** Pentax PEF, Olympus ORF, Panasonic
  RW2, …, and in what order.

Ideas and camera support requests are welcome as
[feature requests](https://github.com/kuolas/raw-files/issues/new/choose).
