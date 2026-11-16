// SPDX-License-Identifier: MIT OR Apache-2.0

//! Resource limits: caps on what a file can make raw-files walk or allocate.

/// Caps on what a file can make raw-files allocate or walk, so that an
/// untrusted file cannot exhaust memory or time.
///
/// The defaults are safe for untrusted input and sit above anything a real
/// camera file needs. They may be tuned in minor releases, so do not rely on
/// their exact values. A cap the reader cannot go on without fails with
/// [`ErrorKind::LimitExceeded`](crate::ErrorKind::LimitExceeded); past the
/// first directory, a directory that crosses a cap is left out as a damaged
/// one is. In a JPEG file, a cap crossed inside its Exif or MPF segment
/// costs at most what that segment holds, not the file, which still opens.
/// A walk over a JPEG stream stops at its cap as at a damaged segment
/// ([`max_jpeg_markers`](Self::max_jpeg_markers)).
///
/// Build it from [`Limits::default()`], or [`Limits::unlimited()`], and
/// change a cap with its `with_…` method, such as
/// [`with_max_preview_bytes`](Self::with_max_preview_bytes); the method
/// named after the cap, such as
/// [`max_preview_bytes`](Self::max_preview_bytes), reads it back.
///
/// # Examples
///
/// ```
/// use raw_files::Limits;
///
/// let limits = Limits::default().with_max_preview_bytes(32 * 1024 * 1024);
/// assert_eq!(limits.max_preview_bytes(), 32 * 1024 * 1024);
/// assert_eq!(limits.max_ifds(), Limits::default().max_ifds());
///
/// // A JPEG stream may hold any number of fill bytes before a marker.
/// let limits = Limits::default().with_max_jpeg_markers(65_536);
/// assert_eq!(limits.max_jpeg_markers(), 65_536);
/// assert_eq!(Limits::unlimited().max_jpeg_markers(), u32::MAX);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Limits {
    max_ifds: u32,
    max_ifd_entries: u32,
    max_ifd_depth: u32,
    max_preview_bytes: u64,
    max_value_bytes: u64,
    max_jpeg_markers: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_ifds: 256,
            max_ifd_entries: 1_000,
            max_ifd_depth: 5,
            max_preview_bytes: 128 * 1024 * 1024,
            max_value_bytes: 1024 * 1024,
            max_jpeg_markers: 1_024,
        }
    }
}

impl Limits {
    /// Every limit at the largest value its type holds.
    ///
    /// For trusted input and tests, never for files from anyone else.
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            max_ifds: u32::MAX,
            max_ifd_entries: u32::MAX,
            max_ifd_depth: u32::MAX,
            max_preview_bytes: u64::MAX,
            max_value_bytes: u64::MAX,
            max_jpeg_markers: u32::MAX,
        }
    }

    /// Sets how many IFDs are visited per file, in place of the cap set
    /// before; [`max_ifds`](Self::max_ifds) reads it. Loops are also
    /// stopped by a visited set.
    #[must_use]
    pub fn with_max_ifds(mut self, n: u32) -> Self {
        self.max_ifds = n;
        self
    }

    /// Sets how many entries one IFD may hold, in place of the cap set
    /// before; [`max_ifd_entries`](Self::max_ifd_entries) reads it.
    #[must_use]
    pub fn with_max_ifd_entries(mut self, n: u32) -> Self {
        self.max_ifd_entries = n;
        self
    }

    /// Sets how deeply SubIFDs may nest, in place of the cap set before;
    /// [`max_ifd_depth`](Self::max_ifd_depth) reads it.
    #[must_use]
    pub fn with_max_ifd_depth(mut self, n: u32) -> Self {
        self.max_ifd_depth = n;
        self
    }

    /// Sets how many bytes one preview may take, which is the most that
    /// `read_preview` will allocate, in place of the cap set before;
    /// [`max_preview_bytes`](Self::max_preview_bytes) reads it.
    #[must_use]
    pub fn with_max_preview_bytes(mut self, n: u64) -> Self {
        self.max_preview_bytes = n;
        self
    }

    /// Sets how many bytes the value of one entry may take, in place of the
    /// cap set before; [`max_value_bytes`](Self::max_value_bytes) reads it
    /// and says what a lowered cap costs.
    #[must_use]
    pub fn with_max_value_bytes(mut self, n: u64) -> Self {
        self.max_value_bytes = n;
        self
    }

    /// Sets how many markers one walk over a JPEG stream reads, in place of
    /// the cap set before; [`max_jpeg_markers`](Self::max_jpeg_markers)
    /// reads it and says what counts.
    #[must_use]
    pub fn with_max_jpeg_markers(mut self, n: u32) -> Self {
        self.max_jpeg_markers = n;
        self
    }

    /// How many IFDs, the directories of a TIFF-based file, are visited
    /// per file at most.
    pub fn max_ifds(self) -> u32 {
        self.max_ifds
    }

    /// How many entries one IFD may hold at most.
    pub fn max_ifd_entries(self) -> u32 {
        self.max_ifd_entries
    }

    /// How deeply SubIFDs may nest at most.
    pub fn max_ifd_depth(self) -> u32 {
        self.max_ifd_depth
    }

    /// How many bytes one preview may take at most, which is the most that
    /// `read_preview` will allocate.
    pub fn max_preview_bytes(self) -> u64 {
        self.max_preview_bytes
    }

    /// How many bytes the value of one entry may take at most.
    ///
    /// The cap holds for every value read whole from the file: those
    /// raw-files reads for the metadata and the previews when it opens a
    /// file, and those a caller reads with
    /// [`CameraFile::read_value`](crate::CameraFile::read_value). It is the
    /// most one read takes into memory; decoding a value of numbers holds up to
    /// twice as much again while it runs. A value of four bytes or fewer,
    /// which its entry holds itself, is read under any cap. A MakerNote is
    /// not read whole: raw-files reads its directory in place, and each of
    /// that directory's values under the cap.
    ///
    /// A longer value costs what raw-files would have read from it:
    ///
    /// - [`CameraFile::read_value`](crate::CameraFile::read_value) fails with
    ///   [`ErrorKind::LimitExceeded`](crate::ErrorKind::LimitExceeded).
    /// - A value of the metadata is `None`, as a damaged one is.
    /// - A preview or a directory that the file locates by such a value is
    ///   left out, with all it holds, such as the SubIFDs of a list longer
    ///   than the cap.
    /// - A RAW file, or any TIFF file but a DNG, whose Make is longer
    ///   fails to open with `LimitExceeded`: its format is told by its
    ///   Make. A JPEG file still opens, without what its MakerNote gives.
    ///
    /// A tag that names one number or fraction, such as the ISO or the
    /// exposure time, is never refused for its length: raw-files reads its
    /// first value alone, or, where the tag's rule allows exactly one
    /// value, such as the offset of a directory or the orientation, gives
    /// none when the file gives it more.
    ///
    /// The default holds every value measured on camera files, the largest
    /// a whole MakerNote of under 1,000,000 bytes.
    pub fn max_value_bytes(self) -> u64 {
        self.max_value_bytes
    }

    /// How many markers one walk over a JPEG stream reads at most.
    ///
    /// Each marker the walk reads counts as one, and so does each fill
    /// byte, an `FF` byte before a marker. The cap holds for each walk on
    /// its own, counted from zero:
    ///
    /// - the walk that finds a JPEG file's segments when it opens;
    /// - the walk that finds the frame header of a preview, in a JPEG file
    ///   or a RAW file, once for each preview the file declares. A place
    ///   declared twice is walked twice until it gives a preview.
    ///
    /// The cap bounds time, not memory. It keeps a stream of tiny segments
    /// or fill bytes from making raw-files read it one marker at a time.
    /// Since each walk counts on its own, opening a file takes at most the
    /// cap times the number of walks: a raised cap raises the whole
    /// product.
    ///
    /// A walk that reaches the cap stops, and what follows is lost; it is
    /// not an error:
    ///
    /// - A JPEG file whose frame header lies past the cap fails to open
    ///   with [`ErrorKind::UnknownFormat`](crate::ErrorKind::UnknownFormat).
    /// - A preview whose frame header lies past the cap is left out.
    ///
    /// The JPEG standard allows any number of fill bytes before a marker.
    /// A stream with more of them than the default allows is read under a
    /// raised cap, of up to `u32::MAX` markers under
    /// [`Limits::unlimited()`]. Camera previews reach their frame header in
    /// a few dozen markers, and camera JPEG files their scan in fewer.
    pub fn max_jpeg_markers(self) -> u32 {
        self.max_jpeg_markers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let limits = Limits::default();
        assert_eq!(limits.max_ifds(), 256);
        assert_eq!(limits.max_ifd_entries(), 1_000);
        assert_eq!(limits.max_ifd_depth(), 5);
        assert_eq!(limits.max_preview_bytes(), 128 * 1024 * 1024);
        assert_eq!(limits.max_value_bytes(), 1024 * 1024);
        assert_eq!(limits.max_jpeg_markers(), 1_024);
    }

    #[test]
    fn with_max_ifds_changes_only_its_cap() {
        let limits = Limits::default().with_max_ifds(7);
        assert_eq!(limits.max_ifds(), 7);
        assert_eq!(
            limits.max_ifd_entries(),
            Limits::default().max_ifd_entries()
        );
        assert_eq!(limits.max_ifd_depth(), Limits::default().max_ifd_depth());
        assert_eq!(
            limits.max_preview_bytes(),
            Limits::default().max_preview_bytes()
        );
        assert_eq!(
            limits.max_value_bytes(),
            Limits::default().max_value_bytes()
        );
        assert_eq!(
            limits.max_jpeg_markers(),
            Limits::default().max_jpeg_markers()
        );
        assert_ne!(limits, Limits::default());
    }

    #[test]
    fn with_max_ifd_entries_changes_only_its_cap() {
        let limits = Limits::default().with_max_ifd_entries(7);
        assert_eq!(limits.max_ifd_entries(), 7);
        assert_eq!(limits.max_ifds(), Limits::default().max_ifds());
        assert_eq!(limits.max_ifd_depth(), Limits::default().max_ifd_depth());
        assert_eq!(
            limits.max_preview_bytes(),
            Limits::default().max_preview_bytes()
        );
        assert_eq!(
            limits.max_value_bytes(),
            Limits::default().max_value_bytes()
        );
        assert_eq!(
            limits.max_jpeg_markers(),
            Limits::default().max_jpeg_markers()
        );
        assert_ne!(limits, Limits::default());
    }

    #[test]
    fn with_max_ifd_depth_changes_only_its_cap() {
        let limits = Limits::default().with_max_ifd_depth(7);
        assert_eq!(limits.max_ifd_depth(), 7);
        assert_eq!(limits.max_ifds(), Limits::default().max_ifds());
        assert_eq!(
            limits.max_ifd_entries(),
            Limits::default().max_ifd_entries()
        );
        assert_eq!(
            limits.max_preview_bytes(),
            Limits::default().max_preview_bytes()
        );
        assert_eq!(
            limits.max_value_bytes(),
            Limits::default().max_value_bytes()
        );
        assert_eq!(
            limits.max_jpeg_markers(),
            Limits::default().max_jpeg_markers()
        );
        assert_ne!(limits, Limits::default());
    }

    #[test]
    fn with_max_preview_bytes_changes_only_its_cap() {
        let limits = Limits::default().with_max_preview_bytes(7);
        assert_eq!(limits.max_preview_bytes(), 7);
        assert_eq!(limits.max_ifds(), Limits::default().max_ifds());
        assert_eq!(
            limits.max_ifd_entries(),
            Limits::default().max_ifd_entries()
        );
        assert_eq!(limits.max_ifd_depth(), Limits::default().max_ifd_depth());
        assert_eq!(
            limits.max_value_bytes(),
            Limits::default().max_value_bytes()
        );
        assert_eq!(
            limits.max_jpeg_markers(),
            Limits::default().max_jpeg_markers()
        );
        assert_ne!(limits, Limits::default());
    }

    #[test]
    fn with_max_value_bytes_changes_only_its_cap() {
        let limits = Limits::default().with_max_value_bytes(7);
        assert_eq!(limits.max_value_bytes(), 7);
        assert_eq!(limits.max_ifds(), Limits::default().max_ifds());
        assert_eq!(
            limits.max_ifd_entries(),
            Limits::default().max_ifd_entries()
        );
        assert_eq!(limits.max_ifd_depth(), Limits::default().max_ifd_depth());
        assert_eq!(
            limits.max_preview_bytes(),
            Limits::default().max_preview_bytes()
        );
        assert_eq!(
            limits.max_jpeg_markers(),
            Limits::default().max_jpeg_markers()
        );
        assert_ne!(limits, Limits::default());
    }

    #[test]
    fn with_max_jpeg_markers_changes_only_its_cap() {
        let limits = Limits::default().with_max_jpeg_markers(7);
        assert_eq!(limits.max_jpeg_markers(), 7);
        assert_eq!(limits.max_ifds(), Limits::default().max_ifds());
        assert_eq!(
            limits.max_ifd_entries(),
            Limits::default().max_ifd_entries()
        );
        assert_eq!(limits.max_ifd_depth(), Limits::default().max_ifd_depth());
        assert_eq!(
            limits.max_preview_bytes(),
            Limits::default().max_preview_bytes()
        );
        assert_eq!(
            limits.max_value_bytes(),
            Limits::default().max_value_bytes()
        );
        assert_ne!(limits, Limits::default());
    }

    #[test]
    fn setters_chain_and_a_setter_restores_the_default() {
        let limits = Limits::default().with_max_ifds(1).with_max_ifds(256);
        assert_eq!(limits, Limits::default());
        let limits = Limits::default()
            .with_max_ifds(1)
            .with_max_ifd_entries(2)
            .with_max_ifd_depth(3)
            .with_max_preview_bytes(4)
            .with_max_value_bytes(5)
            .with_max_jpeg_markers(6);
        assert_eq!(
            (
                limits.max_ifds(),
                limits.max_ifd_entries(),
                limits.max_ifd_depth(),
                limits.max_preview_bytes(),
                limits.max_value_bytes(),
                limits.max_jpeg_markers()
            ),
            (1, 2, 3, 4, 5, 6)
        );
    }

    #[test]
    fn limits_is_copy() {
        fn copied<T: Copy>(value: &T) -> T {
            *value
        }
        let limits = Limits::default().with_max_ifds(7);
        assert_eq!(copied(&limits), limits);
    }

    #[test]
    fn unlimited_is_the_maximum_of_each_type() {
        let limits = Limits::unlimited();
        assert_eq!(limits.max_ifds(), u32::MAX);
        assert_eq!(limits.max_ifd_entries(), u32::MAX);
        assert_eq!(limits.max_ifd_depth(), u32::MAX);
        assert_eq!(limits.max_preview_bytes(), u64::MAX);
        assert_eq!(limits.max_value_bytes(), u64::MAX);
        assert_eq!(limits.max_jpeg_markers(), u32::MAX);
    }
}
