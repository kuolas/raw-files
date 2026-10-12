# ADR010: Decoding ends at a linear image

- Status: Accepted
- Date: 2026-10-11

## Context

After previews and metadata, the roadmap decodes the raw sensor data. What
comes after that decides how large raw-files becomes and whom it serves.

Turning sensor data into a picture has two halves. The first is fixed by
the file and the camera: unpacking the samples, black and white levels,
white balance as shot, demosaicing, highlights, the crop, and the colour
model that maps the camera's colours to a standard space. Every program
that shows a raw image needs this half, and it is where the format
knowledge lives. The second half is the creative edit: tone curves, colour
mixing, masks, sharpening, and the history of edits that has to render the
same years later. Editors differ most in this half. A scene-referred
editor and a display-referred one want different operations, in a different
order, with different saved formats.

The options were to stop at the raw data, to stop after the first half, or
to include an editing engine that applications drive.

## Decision

We will make raw-files **decode and develop up to a scene-referred linear
image plus its colour model**, and leave the creative edit to the
application.

- In scope: raw samples and their layout (CFA pattern, levels, crops,
  white balance as shot); development to a linear floating-point RGB image
  in the camera's colour space, with a choice of demosaicing and highlight
  handling, a crop and a scale (full, half, binned); and the colour model,
  which converts camera colour to CIE XYZ at any white point and between
  white points and colour temperature.
- Development balances the channels with a fixed reference white before
  demosaicing and highlight handling, and the image comes out with that
  balance; the colour model accounts for it. Changing the white balance is
  then a matrix on the linear image, without developing again. A caller
  may develop with another white for the last bit of quality at extreme
  settings.
- Out of scope: tone curves, colour mixing, masks, sharpening, edit history
  and process versions; rendering for a display or an output colour space;
  lens correction (raw-files reports the lens).
- Each development algorithm is identified by name and version. Once
  released, an algorithm's output for a given file does not change; an
  improvement is a new variant or version.

Rejected:

- **Raw data only.** Every application would reimplement demosaicing and
  colour, the hard part they all share and the part closest to the formats.
- **An editing engine in raw-files.** It would impose one pipeline on
  editors that want different ones, and make raw-files carry their saved
  edits' compatibility.

## Consequences

- The roadmap's last milestone is a developed linear image and its colour
  model; there is no editing-pipeline milestone.
- Editors build their pipeline on the linear image, and can keep old edits
  rendering the same because the algorithms underneath do not change.
- An editor's white-balance control stays interactive: a new white is a new
  matrix from the colour model, not a new development.
- A culler can use the half-scale and binned images, which skip
  interpolation, to judge sharpness from the raw data.
- Requests for creative operations are out of scope and are declined with a
  pointer to this record.
- raw-files carries every released algorithm version for as long as it
  supports them, so new algorithm versions are added with care.
