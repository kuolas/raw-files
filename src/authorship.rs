// SPDX-License-Identifier: MIT OR Apache-2.0

//! Who made a picture and what they wrote about it.
//!
//! Exif gives a picture's authorship seven texts: the artist and the
//! copyright notice, the photographer, the editor and the title, a
//! description and a comment. [`Authorship`] holds them as the file
//! records them, and [`Metadata::authorship`](crate::Metadata::authorship)
//! gives them for a file.
//!
//! Every value is optional. A camera writes most of them only when its
//! owner has set them, and leaves them blank otherwise; a file may also
//! record one in a way that cannot be read, such as a tag of the wrong
//! type. Either way the value is `None`, and the others are unaffected.
//!
//! A program that uses raw-files can build an [`Authorship`] for its own
//! tests, without a camera file, through its `with_…` methods.
//! [`Authorship`]'s docs show how.

/// Who made a picture and what they wrote about it: the artist, the
/// copyright notice, the photographer, the editor, the title, the
/// description and the comment, each as the file records it.
///
/// Each text is read from its standard Exif tag, cleaned as every text
/// value of the metadata is: cut at the first NUL, trimmed of spaces at
/// both ends, with bytes that are not UTF-8 replaced by U+FFFD, and `None`
/// when nothing is left. A camera writes a field its owner has not set as
/// NULs or spaces, so such a field is `None`. The artist, the copyright
/// notice, the photographer, the editor and the title are names and
/// notices, refused as damage when longer than 255 bytes; the description
/// and the comment are free text of any length, line breaks included, up
/// to the cap on any one value. Each getter names its tag and says what
/// else gives `None`.
///
/// The texts name people, and raw-files gives them as the file records
/// them: nothing is hidden or shortened, and there is no option to hide
/// them. A program that shows or shares them decides what to keep.
///
/// `Authorship::default()` is an empty one, with every value `None`. A
/// caller's tests start from it and set the values they need with the
/// `with_…` methods, such as [`with_artist`](Self::with_artist), each named
/// after the method that reads the value back and each replacing any value
/// set before, and set the authorship on a [`Metadata`] with
/// [`Metadata::with_authorship`]. As for [`Lens`](crate::Lens), every value
/// is optional, so the empty `Authorship` is the start of every chain and
/// there is no separate builder type.
///
/// The `with_…` methods store each text as given: it is not trimmed or
/// otherwise changed, though the readers clean a file's text before they
/// store it, and a test gets back what it wrote. A text may be a `&str`,
/// which is copied, or a `String`, which is moved.
///
/// # Examples
///
/// Who took a picture and who holds its copyright, each when the file
/// records it:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// let authorship = file.metadata().authorship();
/// if let Some(artist) = authorship.artist() {
///     println!("by {artist}");
/// }
/// if let Some(copyright) = authorship.copyright() {
///     println!("{copyright}");
/// }
/// # Ok(())
/// # }
/// ```
///
/// A program's tests build the authorship of a picture whose camera
/// records its artist and copyright notice, and set it on the metadata:
///
/// ```
/// use raw_files::{Authorship, Metadata};
///
/// let authorship = Authorship::default()
///     .with_artist("Claude Monet")
///     .with_copyright("Copyright Claude Monet")
///     .with_title("Impression, soleil levant");
/// assert_eq!(authorship.artist(), Some("Claude Monet"));
/// assert_eq!(authorship.copyright(), Some("Copyright Claude Monet"));
/// assert_eq!(authorship.title(), Some("Impression, soleil levant"));
/// assert_eq!(authorship.description(), None);
/// let metadata = Metadata::default().with_authorship(authorship.clone());
/// assert_eq!(metadata.authorship(), &authorship);
/// ```
///
/// [`Metadata`]: crate::Metadata
/// [`Metadata::with_authorship`]: crate::Metadata::with_authorship
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Authorship {
    pub(crate) artist: Option<String>,
    pub(crate) copyright: Option<String>,
    pub(crate) photographer: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) comment: Option<String>,
}

impl Authorship {
    /// The person who made the picture, as the file records them: IFD0's
    /// Artist, of type ASCII or UTF-8, or as given to
    /// [`with_artist`](Self::with_artist).
    ///
    /// A camera writes the name its owner set in its menus, or a blank
    /// field, which is `None`.
    pub fn artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    /// The copyright notice, as the file records it: IFD0's Copyright, of
    /// type ASCII or UTF-8, or as given to
    /// [`with_copyright`](Self::with_copyright).
    ///
    /// A camera writes the notice its owner set in its menus, or a blank
    /// field, which is `None`.
    pub fn copyright(&self) -> Option<&str> {
        self.copyright.as_deref()
    }

    /// The photographer, as the file records them: the Exif IFD's
    /// Photographer, of type ASCII or UTF-8, or as given to
    /// [`with_photographer`](Self::with_photographer).
    pub fn photographer(&self) -> Option<&str> {
        self.photographer.as_deref()
    }

    /// The person who edited the picture, as the file records them: the
    /// Exif IFD's ImageEditor, of type ASCII or UTF-8, or as given to
    /// [`with_editor`](Self::with_editor).
    pub fn editor(&self) -> Option<&str> {
        self.editor.as_deref()
    }

    /// The picture's title, as the file records it: the Exif IFD's
    /// ImageTitle, of type ASCII or UTF-8, or as given to
    /// [`with_title`](Self::with_title).
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// What the picture shows, as the file records it: IFD0's
    /// ImageDescription, of type ASCII or UTF-8, or as given to
    /// [`with_description`](Self::with_description).
    ///
    /// It is free text, of any length up to the cap on any one value, and
    /// keeps its line breaks. Some cameras write a text of their own there
    /// whatever the picture shows, such as `SONY DSC` on older Sony bodies;
    /// a text the files checked show a camera writes by itself is no
    /// description, and is `None`, matched as the whole text once cleaned.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// A comment on the picture, as the file records it: the text of the
    /// Exif IFD's UserComment, or as given to
    /// [`with_comment`](Self::with_comment).
    ///
    /// UserComment is of type UNDEFINED and starts with 8 bytes naming the
    /// characters that follow. The comment is read only when they name
    /// ASCII; under any other code, such as Unicode, JIS or the undefined
    /// code, it is `None`, as it is when the value holds no more than its
    /// code. It is free text, of any length up to the cap on any one value,
    /// and keeps its line breaks.
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Sets the artist, in place of any set before;
    /// [`artist`](Self::artist) reads it.
    #[must_use]
    pub fn with_artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = Some(artist.into());
        self
    }

    /// Sets the copyright notice, in place of any set before;
    /// [`copyright`](Self::copyright) reads it.
    #[must_use]
    pub fn with_copyright(mut self, copyright: impl Into<String>) -> Self {
        self.copyright = Some(copyright.into());
        self
    }

    /// Sets the photographer, in place of any set before;
    /// [`photographer`](Self::photographer) reads it.
    #[must_use]
    pub fn with_photographer(mut self, photographer: impl Into<String>) -> Self {
        self.photographer = Some(photographer.into());
        self
    }

    /// Sets the editor, in place of any set before;
    /// [`editor`](Self::editor) reads it.
    #[must_use]
    pub fn with_editor(mut self, editor: impl Into<String>) -> Self {
        self.editor = Some(editor.into());
        self
    }

    /// Sets the title, in place of any set before; [`title`](Self::title)
    /// reads it.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the description, in place of any set before;
    /// [`description`](Self::description) reads it. A camera's placeholder
    /// is stored as given too.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the comment, in place of any set before;
    /// [`comment`](Self::comment) reads it.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_authorship_has_no_value() {
        let authorship = Authorship::default();
        assert_eq!(authorship.artist(), None);
        assert_eq!(authorship.copyright(), None);
        assert_eq!(authorship.photographer(), None);
        assert_eq!(authorship.editor(), None);
        assert_eq!(authorship.title(), None);
        assert_eq!(authorship.description(), None);
        assert_eq!(authorship.comment(), None);
    }

    #[test]
    fn each_setter_sets_its_own_value_as_given() {
        let authorship = Authorship::default()
            .with_artist(" Berthe Morisot ")
            .with_copyright("Copyright Berthe Morisot")
            .with_photographer("Édouard Manet")
            .with_editor("Edgar Degas")
            .with_title("Le Berceau")
            .with_description("A mother\nwatching her child")
            .with_comment("SONY DSC");
        // Stored as given: neither trimmed nor refused as a placeholder.
        assert_eq!(authorship.artist(), Some(" Berthe Morisot "));
        assert_eq!(authorship.copyright(), Some("Copyright Berthe Morisot"));
        assert_eq!(authorship.photographer(), Some("Édouard Manet"));
        assert_eq!(authorship.editor(), Some("Edgar Degas"));
        assert_eq!(authorship.title(), Some("Le Berceau"));
        assert_eq!(
            authorship.description(),
            Some("A mother\nwatching her child")
        );
        assert_eq!(authorship.comment(), Some("SONY DSC"));
    }

    #[test]
    fn a_setter_replaces_the_value_set_before() {
        let authorship = Authorship::default()
            .with_artist("Claude Monet")
            .with_artist("Camille Pissarro");
        assert_eq!(authorship.artist(), Some("Camille Pissarro"));
    }
}
