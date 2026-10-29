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
/// costs what that segment holds, not the file, which still opens.
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
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Limits {
    max_ifds: u32,
    max_ifd_entries: u32,
    max_ifd_depth: u32,
    max_preview_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_ifds: 256,
            max_ifd_entries: 1_000,
            max_ifd_depth: 5,
            max_preview_bytes: 128 * 1024 * 1024,
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

    /// How many IFDs, the directories of a TIFF-based file, are visited
    /// per file at most.
    pub fn max_ifds(&self) -> u32 {
        self.max_ifds
    }

    /// How many entries one IFD may hold at most.
    pub fn max_ifd_entries(&self) -> u32 {
        self.max_ifd_entries
    }

    /// How deeply SubIFDs may nest at most.
    pub fn max_ifd_depth(&self) -> u32 {
        self.max_ifd_depth
    }

    /// How many bytes one preview may take at most, which is the most that
    /// `read_preview` will allocate.
    pub fn max_preview_bytes(&self) -> u64 {
        self.max_preview_bytes
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
            .with_max_preview_bytes(4);
        assert_eq!(
            (
                limits.max_ifds(),
                limits.max_ifd_entries(),
                limits.max_ifd_depth(),
                limits.max_preview_bytes()
            ),
            (1, 2, 3, 4)
        );
        assert_eq!(limits.clone(), limits);
    }

    #[test]
    fn unlimited_is_the_maximum_of_each_type() {
        let limits = Limits::unlimited();
        assert_eq!(limits.max_ifds(), u32::MAX);
        assert_eq!(limits.max_ifd_entries(), u32::MAX);
        assert_eq!(limits.max_ifd_depth(), u32::MAX);
        assert_eq!(limits.max_preview_bytes(), u64::MAX);
    }
}
