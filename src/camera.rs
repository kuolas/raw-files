// SPDX-License-Identifier: MIT OR Apache-2.0

//! The camera a picture was taken with.
//!
//! A camera names itself in the file it writes: its maker and its model,
//! each in a text tag of the file's first directory, IFD0, and on many
//! bodies its serial number. [`Camera`] holds the three as the file
//! records them, and [`Metadata::camera`](crate::Metadata::camera) gives
//! them for a file. From the maker's text, raw-files recognises the maker
//! as a [`Make`], such as [`Make::Nikon`] from `NIKON CORPORATION`, which
//! [`Camera::make`] gives.
//!
//! Every value is optional. A file may not record it, or may record it in
//! a way that cannot be read, such as a tag of the wrong type; either way
//! the value is `None`, and the others are unaffected.
//!
//! A program that uses raw-files can build a [`Camera`] for its own tests,
//! without a camera file, through its `with_…` methods. [`Camera`]'s docs
//! show how.

use std::fmt;

/// The camera body a picture was taken with, as the file names it: its
/// maker and its model as the file records them, and its serial number;
/// and its maker as raw-files recognises it from the recorded text.
///
/// The recorded maker and model are the file's text, cleaned as every text
/// value is and nothing more: cut at the first NUL character, trimmed of
/// spaces at both ends, with any bytes that are not UTF-8 replaced by
/// U+FFFD, and `None` when nothing is left or when the text before the NUL
/// is longer than 255 bytes. So a program gets the text that another reader
/// of the same file shows, such as `NIKON CORPORATION` and `NIKON Z 9` from
/// a Nikon Z 9, or `SONY` and `ILCE-7M4` from a Sony ILCE-7M4.
///
/// From the recorded maker, raw-files recognises the camera's maker, a
/// [`Make`] that [`make`](Self::make) gives, such as [`Make::Nikon`] from
/// `NIKON CORPORATION`. It is read from the recorded text each time it is
/// asked for, so it has no `with_…` method of its own: a test sets the
/// recorded text, and gets the maker that a file with that text gives.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read; a damaged value costs only itself.
///
/// `Camera::default()` is an empty one, with every value `None`. A caller's
/// tests start from it and set the values they need with the `with_…`
/// methods, such as [`with_recorded_model`](Self::with_recorded_model),
/// each named after the method that reads the value back and each replacing
/// any value set before, and set the camera on a [`Metadata`] with
/// [`Metadata::with_camera`]. As for [`Lens`](crate::Lens), every value is
/// optional, so the empty `Camera` is the start of every chain and there is
/// no separate builder type.
///
/// The `with_…` methods store each value as given. A text is not trimmed or
/// otherwise changed, though the readers clean a file's text before they
/// store it: a test gets back what it wrote. A text may be a `&str`, which
/// is copied, or a `String`, which is moved. `Camera` is not `Copy`, since
/// each of its values is a text of its own.
///
/// # Examples
///
/// The camera's maker and model as the file records them, such as
/// `NIKON CORPORATION` and `NIKON Z 9`, and its serial number, each when
/// the file records it:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// let camera = file.metadata().camera();
/// if let (Some(make), Some(model)) = (camera.recorded_make(), camera.recorded_model()) {
///     println!("camera: {make} {model}");
/// }
/// println!(
///     "serial number: {}",
///     camera.serial_number().unwrap_or("not recorded")
/// );
/// # Ok(())
/// # }
/// ```
///
/// A program's tests build the camera of a Nikon Z 9 file and set it on
/// the metadata, and build a camera whose file records its model alone:
///
/// ```
/// use raw_files::{Camera, Make, Metadata};
///
/// let z9 = Camera::default()
///     .with_recorded_make("NIKON CORPORATION")
///     .with_recorded_model("NIKON Z 9")
///     .with_serial_number("3001234");
/// assert_eq!(z9.make(), Some(Make::Nikon));
/// assert_eq!(z9.recorded_make(), Some("NIKON CORPORATION"));
/// assert_eq!(z9.recorded_model(), Some("NIKON Z 9"));
/// assert_eq!(z9.serial_number(), Some("3001234"));
/// let metadata = Metadata::default().with_camera(z9.clone());
/// assert_eq!(metadata.camera(), &z9);
///
/// let model_alone = Camera::default().with_recorded_model("NIKON D600");
/// assert_eq!(model_alone.make(), None);
/// assert_eq!(model_alone.recorded_make(), None);
/// assert_eq!(model_alone.recorded_model(), Some("NIKON D600"));
/// assert_eq!(model_alone.serial_number(), None);
/// ```
///
/// [`Metadata`]: crate::Metadata
/// [`Metadata::with_camera`]: crate::Metadata::with_camera
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Camera {
    pub(crate) recorded_make: Option<String>,
    pub(crate) recorded_model: Option<String>,
    pub(crate) serial_number: Option<String>,
}

impl Camera {
    /// The camera's maker, such as [`Make::Nikon`], as raw-files recognises
    /// it from the Make text the file records, which
    /// [`recorded_make`](Self::recorded_make) gives. `None` when the file
    /// records no Make, or a text raw-files does not recognise; [`Make`]
    /// says which texts it does.
    ///
    /// It is read each time it is asked for, from what
    /// [`with_recorded_make`](Self::with_recorded_make) sets, and has no
    /// `with_…` method of its own: a test sets the recorded text, and gets
    /// the maker that a file with that text gives.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::{Camera, Make};
    ///
    /// let z9 = Camera::default().with_recorded_make("NIKON CORPORATION");
    /// assert_eq!(z9.make(), Some(Make::Nikon));
    /// if let Some(make) = z9.make() {
    ///     assert_eq!(format!("{make}"), "Nikon");
    /// }
    ///
    /// // The maker's name is not a text a file records.
    /// let unknown = Camera::default().with_recorded_make("Nikon");
    /// assert_eq!(unknown.make(), None);
    /// ```
    pub fn make(&self) -> Option<Make> {
        self.recorded_make.as_deref().and_then(Make::from_recorded)
    }

    /// The camera's maker as the file records it, such as
    /// `NIKON CORPORATION`: IFD0's Make, cleaned of padding by the reader
    /// as every text value is, or as given to
    /// [`with_recorded_make`](Self::with_recorded_make).
    pub fn recorded_make(&self) -> Option<&str> {
        self.recorded_make.as_deref()
    }

    /// The camera's model as the file records it, such as `NIKON Z 9`:
    /// IFD0's Model, cleaned of padding by the reader as every text value
    /// is, or as given to [`with_recorded_model`](Self::with_recorded_model).
    pub fn recorded_model(&self) -> Option<&str> {
        self.recorded_model.as_deref()
    }

    /// The camera body's serial number, as text: the Exif BodySerialNumber
    /// or, on a Nikon file that records none there, the SerialNumber of
    /// Nikon's MakerNote, cleaned of padding by the reader as every text
    /// value is, or as given to
    /// [`with_serial_number`](Self::with_serial_number). Many bodies do not
    /// record one in a form the library reads, and then it is `None`.
    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }

    /// Sets the camera's maker as the file records it, in place of any set
    /// before; [`recorded_make`](Self::recorded_make) reads it, and
    /// [`make`](Self::make) reads the maker from it too.
    #[must_use]
    pub fn with_recorded_make(mut self, make: impl Into<String>) -> Self {
        self.recorded_make = Some(make.into());
        self
    }

    /// Sets the camera's model as the file records it, in place of any set
    /// before; [`recorded_model`](Self::recorded_model) reads it.
    #[must_use]
    pub fn with_recorded_model(mut self, model: impl Into<String>) -> Self {
        self.recorded_model = Some(model.into());
        self
    }

    /// Sets the camera body's serial number, in place of any set before;
    /// [`serial_number`](Self::serial_number) reads it.
    #[must_use]
    pub fn with_serial_number(mut self, serial_number: impl Into<String>) -> Self {
        self.serial_number = Some(serial_number.into());
        self
    }
}

/// The maker of a camera, as raw-files recognises it from the Make text a
/// file records, such as [`Nikon`](Self::Nikon) from `NIKON CORPORATION`;
/// [`Camera::make`] gives it.
///
/// Each maker's name is spelled as the maker writes its own name in its
/// own published documents, as the
/// [decision to take names as their makers write them](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr022-names-as-their-makers-write-them.md)
/// sets out. Its [`name`](Self::name)s, which `Display` writes too, are
/// "Nikon", "Sony", "Hasselblad" and "Canon".
///
/// A maker is recognised only from a Make text a file has been seen to
/// write, matched whole and with its case, so any other text, such as
/// `Sony` or `nikon`, gives none. Each variant names its texts; the
/// [notes on camera names](https://github.com/kuolas/raw-files/blob/master/docs/formats/camera-names.md)
/// list the files that show each text and the maker's documents behind
/// each name.
///
/// Which maker's reader opens a file is decided otherwise, by a looser
/// test of the same text: a Make that starts with `NIKON` or `SONY`,
/// whatever its case, once trimmed, or, in a raw file that holds Sony's
/// private pointer, `HASSELBLAD`. So a file whose Make is `Nikon` is read
/// as a Nikon file, with the values Nikon's MakerNote records, and still
/// gives no maker here.
///
/// `Make` is `#[non_exhaustive]`, so match on it with a wildcard arm: more
/// makers are added as files show their Make texts. It orders as its
/// variants are listed, and a new variant is added after the existing ones,
/// so an order once observed holds.
///
/// # Examples
///
/// ```
/// use raw_files::{Camera, Make};
///
/// let camera = Camera::default().with_recorded_make("SONY");
/// let label = match camera.make() {
///     Some(Make::Nikon) => "a Nikon".to_owned(),
///     Some(make) => format!("a camera by {make}"),
///     None => "a camera by a maker raw-files does not recognise".to_owned(),
/// };
/// assert_eq!(label, "a camera by Sony");
/// assert_eq!(Make::Hasselblad.name(), "Hasselblad");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Make {
    /// Nikon: the Make text `NIKON CORPORATION`, which every Nikon body
    /// checked writes but two Coolpix compacts, the E5700 and the P7800,
    /// which write `NIKON`.
    Nikon,
    /// Sony: the Make text `SONY`, which every Sony body checked writes.
    Sony,
    /// Hasselblad: the Make text `HASSELBLAD`, which its Lunar writes.
    Hasselblad,
    /// Canon: the Make text `Canon`, which its PowerShot G7 X writes.
    Canon,
}

impl Make {
    /// The maker's name, for an information panel: "Nikon", "Sony",
    /// "Hasselblad" or "Canon".
    pub fn name(self) -> &'static str {
        match self {
            Make::Nikon => "Nikon",
            Make::Sony => "Sony",
            Make::Hasselblad => "Hasselblad",
            Make::Canon => "Canon",
        }
    }

    /// The maker whose Make text `text` is, matched whole and with its
    /// case, or `None` for a text no maker is recognised from.
    pub(crate) fn from_recorded(text: &str) -> Option<Make> {
        MAKE_TEXTS
            .iter()
            .find(|&&(recorded, _)| recorded == text)
            .map(|&(_, make)| make)
    }
}

/// Every Make text a maker is recognised from, with its maker, in the
/// order the notes on camera names (`docs/formats/camera-names.md`,
/// section 2) list them, each one a text a file the notes name records.
static MAKE_TEXTS: [(&str, Make); 5] = [
    ("NIKON CORPORATION", Make::Nikon),
    ("NIKON", Make::Nikon),
    ("SONY", Make::Sony),
    ("HASSELBLAD", Make::Hasselblad),
    ("Canon", Make::Canon),
];

impl fmt::Display for Make {
    /// Writes [`name`](Make::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_names, assert_ordered};

    /// The camera of a Z 9 file, every value set, each one different from
    /// the others, so that a getter reading the wrong value is caught.
    fn z9_camera() -> Camera {
        Camera {
            recorded_make: Some("NIKON CORPORATION".to_owned()),
            recorded_model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("3001234".to_owned()),
        }
    }

    #[test]
    fn default_camera_is_empty() {
        let camera = Camera::default();
        assert_eq!(camera.recorded_make(), None);
        assert_eq!(camera.recorded_model(), None);
        assert_eq!(camera.serial_number(), None);
    }

    #[test]
    fn camera_getters_return_what_was_set() {
        let camera = z9_camera();
        assert_eq!(camera.recorded_make(), Some("NIKON CORPORATION"));
        assert_eq!(camera.recorded_model(), Some("NIKON Z 9"));
        assert_eq!(camera.serial_number(), Some("3001234"));
    }

    #[test]
    fn each_camera_builder_sets_its_own_value_and_no_other() {
        // From the empty camera, and from a full one whose every value
        // differs from the one set, so that a builder writing another
        // value, or clearing one, is caught. The expected value is built
        // field by field, not through the builders.
        for start in [Camera::default(), z9_camera()] {
            let cases = [
                (
                    start.clone().with_recorded_make("Nikon"),
                    Camera {
                        recorded_make: Some("Nikon".to_owned()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_recorded_model("NIKON Z 8"),
                    Camera {
                        recorded_model: Some("NIKON Z 8".to_owned()),
                        ..start.clone()
                    },
                ),
                (
                    start.clone().with_serial_number("7654321"),
                    Camera {
                        serial_number: Some("7654321".to_owned()),
                        ..start.clone()
                    },
                ),
            ];
            for (place, (built, expected)) in cases.into_iter().enumerate() {
                assert_eq!(built, expected, "builder {place} from {start:?}");
            }
        }
    }

    #[test]
    fn every_camera_builder_in_a_chain_equals_the_value_built_field_by_field() {
        let built = Camera::default()
            .with_recorded_make("NIKON CORPORATION")
            .with_recorded_model("NIKON Z 9")
            .with_serial_number("3001234");
        assert_eq!(built, z9_camera());
    }

    #[test]
    fn a_later_camera_value_replaces_an_earlier_one() {
        let earlier = Camera::default()
            .with_recorded_make("A")
            .with_recorded_model("A")
            .with_serial_number("A");
        // The Z 9's values, each set after an earlier one.
        let camera = earlier
            .with_recorded_make("NIKON CORPORATION")
            .with_recorded_model("NIKON Z 9")
            .with_serial_number("3001234");
        assert_eq!(camera, z9_camera());
    }

    #[test]
    fn the_camera_builders_move_an_owned_text_rather_than_copy_it() {
        let make = String::from("NIKON CORPORATION");
        let model = String::from("NIKON Z 9");
        let serial_number = String::from("3001234");
        let addresses = [make.as_ptr(), model.as_ptr(), serial_number.as_ptr()];
        let camera = Camera::default()
            .with_recorded_make(make)
            .with_recorded_model(model)
            .with_serial_number(serial_number);
        let texts = [
            camera.recorded_make(),
            camera.recorded_model(),
            camera.serial_number(),
        ];
        assert_eq!(
            texts,
            [
                Some("NIKON CORPORATION"),
                Some("NIKON Z 9"),
                Some("3001234")
            ]
        );
        // The very texts given, not copies of them.
        assert_eq!(texts.map(|text| text.map(str::as_ptr)), addresses.map(Some));
    }

    #[test]
    fn a_camera_keeps_its_texts_as_given() {
        // Neither cleaned nor refused, as the readers would a file's: a
        // padded text, one with its NUL padding, and an empty one.
        let camera = Camera::default()
            .with_recorded_make("  padded  ")
            .with_recorded_model("NIKON Z 9\0\0")
            .with_serial_number("");
        assert_eq!(camera.recorded_make(), Some("  padded  "));
        assert_eq!(camera.recorded_model(), Some("NIKON Z 9\0\0"));
        assert_eq!(camera.serial_number(), Some(""));
    }

    #[test]
    fn cameras_collect_in_a_set() {
        // A culler listing the bodies of a folder puts them in a set.
        let camera = |model: &str| Camera::default().with_recorded_model(model);
        let cameras: std::collections::HashSet<Camera> = [camera("A"), camera("B"), camera("A")]
            .into_iter()
            .collect();
        assert_eq!(cameras.len(), 2);
    }

    // Every variant of `Make`, in the order of its declaration. The
    // `match` has no wildcard arm, so a variant added later does not
    // compile until it is listed here too, and the test below then fails
    // until its name is listed there.

    fn all_makes() -> [Make; 4] {
        let all = [Make::Nikon, Make::Sony, Make::Hasselblad, Make::Canon];
        for make in all {
            match make {
                Make::Nikon | Make::Sony | Make::Hasselblad | Make::Canon => {}
            }
        }
        all
    }

    #[test]
    fn every_make_name_is_the_one_documented_and_display_writes_it() {
        // The names are stable once released: a program may show or store
        // them, so a changed one fails here.
        assert_names(
            &all_makes(),
            Make::name,
            &["Nikon", "Sony", "Hasselblad", "Canon"],
        );
    }

    #[test]
    fn makes_order_as_their_variants_are_listed() {
        // A program may key a sorted map by the maker, and the order is a
        // promise: a variant moved, or added before the others, fails.
        assert_ordered(&all_makes());
    }

    #[test]
    fn every_make_text_of_the_notes_gives_its_maker() {
        // Every Make text the notes on camera names list, as the files
        // show it, set as the recorded make.
        let cases = [
            ("NIKON CORPORATION", Make::Nikon),
            ("NIKON", Make::Nikon),
            ("SONY", Make::Sony),
            ("HASSELBLAD", Make::Hasselblad),
            ("Canon", Make::Canon),
        ];
        for (text, make) in cases {
            let camera = Camera::default().with_recorded_make(text);
            assert_eq!(camera.make(), Some(make), "{text:?}");
        }
    }

    #[test]
    fn any_other_make_text_gives_no_maker() {
        // A maker is recognised from its whole text, case included: a
        // text in another case, a maker's name, a part of a text or more
        // than it, another field's text, and a text with the padding a
        // reader trims, which the setter stores as given.
        let texts = [
            "Sony",
            "sony",
            "nikon",
            "Nikon",
            "NIKON CORP",
            "NIKON CORPORATION INC",
            " SONY",
            "SONY ",
            "",
            "CANON",
            "Hasselblad",
            "NIKON Z 9",
        ];
        for text in texts {
            let camera = Camera::default().with_recorded_make(text);
            assert_eq!(camera.make(), None, "{text:?}");
        }
    }

    #[test]
    fn the_maker_is_read_from_the_recorded_make_each_time() {
        assert_eq!(Camera::default().make(), None);
        // A model alone gives no maker, even one that is a Make text.
        assert_eq!(Camera::default().with_recorded_model("NIKON").make(), None);
        let nikon = Camera::default().with_recorded_make("NIKON CORPORATION");
        assert_eq!(nikon.make(), Some(Make::Nikon));
        // No maker is kept beside the text: a new text gives its own.
        let sony = nikon.with_recorded_make("SONY");
        assert_eq!(sony.make(), Some(Make::Sony));
        let unknown = sony.with_recorded_make("Sony");
        assert_eq!(unknown.make(), None);
    }
}
