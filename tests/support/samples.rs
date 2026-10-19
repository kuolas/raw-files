// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the integration tests find the real sample files of
//! `tests/samples.tsv`, and the check that one opens with exactly the
//! previews expected of it. They are too large to commit, so
//! `scripts/fetch-samples.sh` downloads them, and a test whose file is
//! absent skips with a note rather than failing: the suite runs offline.

use std::env;
use std::path::{Path, PathBuf};

use raw_files::{Format, Orientation, Preview, PreviewFormat, RawFile};

/// The path of the sample file `name`, or `None`, with a note on stderr,
/// when it has not been downloaded. The files are in `RAW_FILES_SAMPLES`
/// (relative to the current directory, which under `cargo test` is the
/// crate's root), or in `target/raw-samples` when it is unset, where
/// `scripts/fetch-samples.sh` puts them.
pub fn path(name: &str) -> Option<PathBuf> {
    let dir = match env::var_os("RAW_FILES_SAMPLES") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("target/raw-samples"),
    };
    let path = dir.join(name);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!(
            "skipped: sample file {} is missing; scripts/fetch-samples.sh downloads it",
            path.display()
        );
        None
    }
}

/// One JPEG a sample file holds, as the notes on its format record it and
/// the reader must report it: its size in pixels from its own frame header,
/// where its bytes are, and how many bytes the declared length runs past
/// the end-of-image marker (a preview can be padded, as some Sony files'
/// are: the thumbnails of the DSLR-A200, A700 and A900 and the 1616 by
/// 1080 previews of the SLT-A55 and NEX-5; see the ARW notes).
pub struct Jpeg {
    pub width: u32,
    pub height: u32,
    pub offset: u64,
    pub len: u64,
    pub after_eoi: u64,
}

/// A sample file of `tests/samples.tsv` and what opening it must give: its
/// previews, smallest first, none for a file in which the reader finds
/// none, and the orientation IFD0 gives them all.
pub struct Sample {
    pub file: &'static str,
    pub orientation: Orientation,
    pub previews: &'static [Jpeg],
}

/// The stored size of each preview, in order.
pub fn sizes(previews: &[Preview]) -> Vec<(u32, u32)> {
    previews.iter().map(|p| (p.width(), p.height())).collect()
}

/// Checks that `sample` opens as `format` with exactly its expected
/// previews, smallest first: each a JPEG at its offset and of its length,
/// with IFD0's orientation, that reads whole as stored, its padding after
/// the end-of-image marker included; and that the largest is the last, or
/// that there is none when the file has no preview. Does nothing when the
/// file has not been downloaded.
pub fn check(sample: &Sample, format: Format) {
    let Some(path) = path(sample.file) else {
        return;
    };
    let label = sample.file;
    let mut file = RawFile::open(&path).unwrap();
    assert_eq!(file.format(), format, "{label}");
    let expected_sizes: Vec<(u32, u32)> = sample
        .previews
        .iter()
        .map(|jpeg| (jpeg.width, jpeg.height))
        .collect();
    assert_eq!(sizes(file.previews()), expected_sizes, "{label}");

    let previews = file.previews().to_vec();
    for (preview, jpeg) in previews.iter().zip(sample.previews) {
        let size = (preview.width(), preview.height());
        assert_eq!(preview.format(), PreviewFormat::Jpeg, "{label} {size:?}");
        assert_eq!(
            preview.orientation(),
            sample.orientation,
            "{label} {size:?}"
        );
        assert_eq!(preview.byte_range().start, jpeg.offset, "{label} {size:?}");
        assert_eq!(preview.byte_len(), jpeg.len, "{label} {size:?}");

        // The declared length is returned as stored, so a preview padded
        // after its end-of-image marker comes back with the padding.
        let bytes = file.read_preview(preview).unwrap();
        assert_eq!(
            u64::try_from(bytes.len()).unwrap(),
            preview.byte_len(),
            "{label} {size:?}"
        );
        assert_eq!(bytes[..3], [0xFF, 0xD8, 0xFF], "{label} {size:?}");
        let eoi = usize::try_from(jpeg.len - jpeg.after_eoi - 2).unwrap();
        assert_eq!(
            bytes[eoi..eoi + 2],
            [0xFF, 0xD9],
            "{label} {size:?}: the end-of-image marker, followed by {} bytes",
            jpeg.after_eoi
        );
    }

    let place = |preview: &Preview| (preview.size(), preview.byte_range());
    assert_eq!(
        file.largest_preview().as_ref().map(place),
        previews.last().map(place),
        "{label}"
    );
}
