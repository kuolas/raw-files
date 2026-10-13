# Changelog

All notable changes to raw-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Nothing has been released yet, so there is no versioning promise; that will be
decided with the first release.

## [Unreleased]

### Added

- The project is licensed under the MIT license or the Apache License,
  version 2.0, at your option.
- Builds with Rust 1.95 or later.
- `Error`, with an `ErrorKind` category and the byte offset when known,
  and the `Result` alias.
- `Limits`, the caps on what a file can make the library walk or allocate.
- `Orientation`, the EXIF orientation a viewer must apply to a preview, and
  `Size`, a width and height in pixels.
- `Preview`, one embedded preview: where its bytes are in the file, its
  size, format and orientation; and `PreviewFormat`, how its bytes are
  encoded, with a media type and a file extension.
