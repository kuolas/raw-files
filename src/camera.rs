// SPDX-License-Identifier: MIT OR Apache-2.0

//! The camera a picture was taken with.
//!
//! A camera names itself in the file it writes: its maker and its model,
//! each in a text tag of the file's first directory, IFD0, on many bodies
//! its serial number, and on some its owner's name. [`Camera`] holds the
//! four as the file records them, and [`Metadata::camera`](crate::Metadata::camera) gives
//! them for a file. From the maker's text, raw-files recognizes the maker
//! as a [`Make`], such as [`Make::Nikon`] from `NIKON CORPORATION`, which
//! [`Camera::make`] gives; and from the maker and the model's text
//! together, the name the maker sells the body under, such as `Z6III` from
//! `NIKON Z6_3`, which [`Camera::model`] gives for the bodies raw-files'
//! table of names holds. [`Camera::display_name`] gives a text to show a
//! person: the two names joined, or else the model's text.
//!
//! Every value is optional. A file may not record it, or may record it in
//! a way that cannot be read, such as a tag of the wrong type; either way
//! the value is `None`, and the others are unaffected.
//!
//! A program that uses raw-files can build a [`Camera`] for its own tests,
//! without a camera file, through its `with_…` methods. [`Camera`]'s docs
//! show how.

use std::fmt;

use crate::camera_settings::ImageArea;

/// The camera body a picture was taken with, as the file names it: its
/// maker and its model as the file records them, its serial number and its
/// owner's name;
/// and its maker and the name it sells the body under, as raw-files reads
/// them from the recorded texts.
///
/// The recorded maker and model are the file's text, cleaned as every text
/// value is and nothing more: cut at the first NUL character, trimmed of
/// spaces at both ends, with any bytes that are not UTF-8 replaced by
/// U+FFFD, and `None` when nothing is left or when the text before the NUL
/// is longer than 255 bytes. So a program gets the text that another reader
/// of the same file shows, such as `NIKON CORPORATION` and `NIKON Z 9` from
/// a Nikon Z 9, or `SONY` and `ILCE-7M4` from a Sony ILCE-7M4.
///
/// From the recorded maker, raw-files recognizes the camera's maker, a
/// [`Make`] that [`make`](Self::make) gives, such as [`Make::Nikon`] from
/// `NIKON CORPORATION`; and from the maker and the recorded model
/// together, the name the maker sells the body under, which
/// [`model`](Self::model) gives, such as `Z6III` from `NIKON Z6_3`, for
/// the bodies raw-files' table of names holds and for no other.
/// [`display_name`](Self::display_name) gives a text to show a person for
/// every file that records a Model: the two names joined, such as
/// `Nikon Z6III`, or else the recorded model. Each is read from the
/// recorded texts each time it is asked for, so none has a `with_…` method
/// of its own: a test sets the recorded texts, and gets what a file with
/// those texts gives.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read; a damaged value costs only itself.
///
/// `Camera::default()` is an empty one, with every value `None`. A caller's
/// tests start from it and set the values they need with the `with_…`
/// methods, such as [`with_recorded_model`](Self::with_recorded_model),
/// each named after the method that reads the value back and each replacing
/// any value set before, and set the camera on a [`Metadata`] with
/// [`Metadata::with_camera`]. The empty `Camera` is the start of every
/// chain.
///
/// The `with_…` methods store each value as given. A text is not trimmed or
/// otherwise changed, though the readers clean a file's text before they
/// store it: a test gets back what it wrote. A text may be a `&str`, which
/// is copied, or a `String`, which is moved.
///
/// # Examples
///
/// The camera's maker and model as the file records them, such as
/// `NIKON CORPORATION` and `NIKON Z 9`, and its serial number. Each is
/// printed on a line of its own, since Nikon's Model repeats the brand:
///
/// ```no_run
/// use raw_files::CameraFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = CameraFile::open("photo.nef")?;
/// let camera = file.metadata().camera();
/// println!("make: {}", camera.recorded_make().unwrap_or("not recorded"));
/// println!("model: {}", camera.recorded_model().unwrap_or("not recorded"));
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
/// assert_eq!(z9.model(), Some("Z 9"));
/// assert_eq!(z9.recorded_make(), Some("NIKON CORPORATION"));
/// assert_eq!(z9.recorded_model(), Some("NIKON Z 9"));
/// assert_eq!(z9.serial_number(), Some("3001234"));
/// let metadata = Metadata::default().with_camera(z9.clone());
/// assert_eq!(metadata.camera(), &z9);
///
/// let model_alone = Camera::default().with_recorded_model("NIKON D600");
/// assert_eq!(model_alone.make(), None);
/// // The name needs the maker too.
/// assert_eq!(model_alone.model(), None);
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
    pub(crate) owner_name: Option<String>,
}

impl Camera {
    /// The camera's maker, such as [`Make::Nikon`], as raw-files recognizes
    /// it from the Make text the file records, which
    /// [`recorded_make`](Self::recorded_make) gives. `None` when the file
    /// records no Make, or a text raw-files does not recognize; [`Make`]
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

    /// The name the camera's maker sells the body under, such as `Z6III`
    /// for a file whose Model is `NIKON Z6_3`, or `α7 II` for `ILCE-7M2`.
    /// `None` for a body raw-files' table of names does not hold. For a text
    /// to show a person for every body, use
    /// [`display_name`](Self::display_name).
    ///
    /// The brand is not part of the name: [`make`](Self::make) gives it.
    ///
    /// The name is spelled as its maker writes it in its own documents,
    /// with its spaces and its case, such as `Z 9` with a space and `Z6III`
    /// without one, and with Sony's Greek letter α, so it is not always
    /// ASCII; a program that needs ASCII uses
    /// [`recorded_model`](Self::recorded_model).
    ///
    /// It comes from raw-files' table of names, looked up by the maker and
    /// the whole recorded Model text together, case included. The table
    /// holds the bodies that the
    /// [notes on camera names](https://github.com/kuolas/raw-files/blob/master/docs/formats/camera-names.md)
    /// list, each with a file that shows its Model text and the maker's
    /// document that gives its name. It is `None` too when the file records
    /// no Model, or no Make that raw-files recognizes. It is never the
    /// recorded text in a name's place, so that a program can tell a name
    /// from a code.
    ///
    /// It is read each time it is asked for, from what
    /// [`with_recorded_make`](Self::with_recorded_make) and
    /// [`with_recorded_model`](Self::with_recorded_model) set, and has no
    /// `with_…` method of its own: a test sets the recorded texts of a body
    /// the table holds, and gets the name that a file with those texts
    /// gives.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::Camera;
    ///
    /// let z6iii = Camera::default()
    ///     .with_recorded_make("NIKON CORPORATION")
    ///     .with_recorded_model("NIKON Z6_3");
    /// assert_eq!(z6iii.model(), Some("Z6III"));
    ///
    /// // A body the table does not hold has no name. Its text to show is
    /// // the Model as recorded.
    /// let d7200 = Camera::default()
    ///     .with_recorded_make("NIKON CORPORATION")
    ///     .with_recorded_model("NIKON D7200");
    /// assert_eq!(d7200.model(), None);
    /// assert_eq!(d7200.display_name().as_deref(), Some("NIKON D7200"));
    /// ```
    pub fn model(&self) -> Option<&'static str> {
        marketing_name(self.make()?, self.recorded_model.as_deref()?)
    }

    /// The published facts of the body, its sensor format and whether it
    /// offers raw sizes, from raw-files' table, looked up as
    /// [`model`](Self::model) looks up the name: by the maker and the whole
    /// recorded Model text together. `None` for a body the table holds no
    /// facts for, and when the file records no Model or no Make that
    /// raw-files recognizes.
    pub(crate) fn body_facts(&self) -> Option<BodyFacts> {
        entry(self.make()?, self.recorded_model.as_deref()?)?.facts
    }

    /// The camera's name to show a person, for every file that records a
    /// Model.
    ///
    /// - Where raw-files has the maker's name and the body's, it is the
    ///   two joined by a space: [`make`](Self::make) and
    ///   [`model`](Self::model), such as `Nikon Z6III`.
    /// - For any other body, it is the Model as the file records it, such
    ///   as `NIKON D7200`: [`recorded_model`](Self::recorded_model).
    /// - It is `None` when the file records no Model.
    ///
    /// It is a text to show, not to match on: it may be a name or a code.
    /// A program that compares cameras uses `make`, `model` or
    /// `recorded_model`, which say which they are.
    ///
    /// It is read each time it is asked for, from what
    /// [`with_recorded_make`](Self::with_recorded_make) and
    /// [`with_recorded_model`](Self::with_recorded_model) set. It has no
    /// `with_…` method of its own.
    ///
    /// # Examples
    ///
    /// ```
    /// use raw_files::Camera;
    ///
    /// let z6iii = Camera::default()
    ///     .with_recorded_make("NIKON CORPORATION")
    ///     .with_recorded_model("NIKON Z6_3");
    /// assert_eq!(z6iii.display_name().as_deref(), Some("Nikon Z6III"));
    ///
    /// // A body the table does not hold: the Model as recorded, with no
    /// // maker's name added before it.
    /// let d7200 = Camera::default()
    ///     .with_recorded_make("NIKON CORPORATION")
    ///     .with_recorded_model("NIKON D7200");
    /// assert_eq!(d7200.display_name().as_deref(), Some("NIKON D7200"));
    ///
    /// // A Make alone: nothing to show.
    /// let make_alone = Camera::default().with_recorded_make("SONY");
    /// assert_eq!(make_alone.display_name(), None);
    /// ```
    ///
    /// A file's camera:
    ///
    /// ```no_run
    /// use raw_files::CameraFile;
    ///
    /// # fn main() -> raw_files::Result<()> {
    /// let file = CameraFile::open("photo.arw")?;
    /// if let Some(name) = file.metadata().camera().display_name() {
    ///     println!("camera: {name}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn display_name(&self) -> Option<String> {
        let recorded_model = self.recorded_model.as_deref()?;
        let named = self.make().and_then(|make| {
            marketing_name(make, recorded_model).map(|name| format!("{make} {name}"))
        });
        Some(named.unwrap_or_else(|| recorded_model.to_owned()))
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

    /// The name of the camera's owner, as the file records it: the Exif
    /// CameraOwnerName, of type ASCII or UTF-8, cleaned of padding by the
    /// reader as every text value is, or as given to
    /// [`with_owner_name`](Self::with_owner_name).
    ///
    /// It names whoever owns the body, set once in the camera, not
    /// whoever took the picture, which
    /// [`Authorship::artist`](crate::Authorship::artist) and
    /// [`Authorship::photographer`](crate::Authorship::photographer) give.
    /// Exif says it shall in principle not be rewritten. Few bodies record
    /// one, and a blank field, as a camera writes one its owner has not
    /// set, is `None`. raw-files gives it as the file records it, with no
    /// option to hide it.
    pub fn owner_name(&self) -> Option<&str> {
        self.owner_name.as_deref()
    }

    /// Sets the camera's maker as the file records it, in place of any set
    /// before. [`recorded_make`](Self::recorded_make) reads it, and
    /// [`make`](Self::make) reads the maker from it.
    /// [`model`](Self::model) and [`display_name`](Self::display_name)
    /// read from it and the recorded model together.
    #[must_use]
    pub fn with_recorded_make(mut self, make: impl Into<String>) -> Self {
        self.recorded_make = Some(make.into());
        self
    }

    /// Sets the camera's model as the file records it, in place of any set
    /// before. [`recorded_model`](Self::recorded_model) reads it.
    /// [`model`](Self::model) and [`display_name`](Self::display_name)
    /// read from it and the recorded make together.
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

    /// Sets the name of the camera's owner, in place of any set before;
    /// [`owner_name`](Self::owner_name) reads it.
    #[must_use]
    pub fn with_owner_name(mut self, owner_name: impl Into<String>) -> Self {
        self.owner_name = Some(owner_name.into());
        self
    }
}

/// The maker of a camera, as raw-files recognizes it from the Make text a
/// file records, such as [`Nikon`](Self::Nikon) from `NIKON CORPORATION`;
/// [`Camera::make`] gives it.
///
/// Each maker's name is spelled as the maker writes its own name in its
/// own published documents, as the
/// [decision to take names as their makers write them](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr022-names-as-their-makers-write-them.md)
/// sets out. Its [`name`](Self::name)s, which `Display` writes too, are
/// "Nikon", "Sony", "Hasselblad" and "Canon".
///
/// A maker is recognized only from a Make text a file has been seen to
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
///     None => "a camera by a maker raw-files does not recognize".to_owned(),
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
    /// case, or `None` for a text no maker is recognized from.
    pub(crate) fn from_recorded(text: &str) -> Option<Make> {
        MAKE_TEXTS
            .iter()
            .find(|&&(recorded, _)| recorded == text)
            .map(|&(_, make)| make)
    }
}

/// Every Make text a maker is recognized from, with its maker, in the
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

/// One body's name, as its maker sells it, by the maker and the Model text
/// the body's files record, with the facts its maker publishes about it
/// where the notes give them.
struct Name {
    /// The maker, as [`Make::from_recorded`] recognizes it from the Make
    /// text the body's files record.
    make: Make,
    /// The whole Model text the body's files record, cleaned of padding as
    /// [`Camera::recorded_model`] gives it, such as `NIKON Z6_3`.
    recorded_model: &'static str,
    /// The name, as its maker writes it, without the brand, such as
    /// `Z6III`.
    name: &'static str,
    /// The body's sensor format and whether it offers raw sizes, for a body
    /// whose row in section 5 of the notes on camera names gives them;
    /// `None` for any other. A body may have a name and no facts.
    facts: Option<BodyFacts>,
}

/// What a body's maker publishes about it that fills two camera settings
/// its files may not record: the image area and the raw image size. The
/// Nikon reader gives each where a file does not record it
/// ([`Camera::body_facts`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BodyFacts {
    /// The format of the body's sensor, as the image area of the whole
    /// sensor: [`ImageArea::FullFrame`] for what Nikon calls FX, and
    /// [`ImageArea::ApsC`] for DX.
    pub(crate) sensor: ImageArea,
    /// Whether the body offers a raw size other than its largest. A body
    /// that offers none writes every raw file at the large size.
    pub(crate) offers_raw_sizes: bool,
}

/// The name of every body raw-files names, by its maker and its recorded
/// Model text, in the order the notes on camera names
/// (`docs/formats/camera-names.md`) list them, Nikon's bodies and then
/// Sony's. The notes record, for every entry, a file that shows its Model
/// text and the maker's document that gives its name, and an entry needs
/// both
/// ([the decision to take names and published facts as their makers write them](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr022-names-as-their-makers-write-them.md),
/// ADR022). Each name is copied from the notes as they write it, its
/// spaces plain U+0020 spaces and Sony's α the Greek small letter alpha,
/// U+03B1.
///
/// A body's facts come from the same notes, section 5, which give the
/// maker's document that states each one; each row's comment names it. A
/// fact enters only with such a document, so most bodies have none.
static NAMES: &[Name] = &[
    // Nikon's D600 User's Manual: its name on page 326 (specifications)
    // of edition 6MB15311-01, its facts on pages 326 and 94 (image size)
    // of edition 6MB15211-06.
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON D600",
        name: "D600",
        facts: Some(BodyFacts {
            sensor: ImageArea::FullFrame,
            offers_raw_sizes: false,
        }),
    },
    // Nikon's D300S User's Manual: pages 385 (specifications) and 69
    // (image size).
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON D300S",
        name: "D300S",
        facts: Some(BodyFacts {
            sensor: ImageArea::ApsC,
            offers_raw_sizes: false,
        }),
    },
    // Nikon's D3200 Reference Manual: pages 196 (specifications) and 45
    // (image size).
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON D3200",
        name: "D3200",
        facts: Some(BodyFacts {
            sensor: ImageArea::ApsC,
            offers_raw_sizes: false,
        }),
    },
    // Nikon's D3300 Reference Manual: pages 342 (specifications) and 86
    // (image size).
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON D3300",
        name: "D3300",
        facts: Some(BodyFacts {
            sensor: ImageArea::ApsC,
            offers_raw_sizes: false,
        }),
    },
    // Nikon's D7100 User's Manual: pages 336 (specifications) and 67
    // (image size).
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON D7100",
        name: "D7100",
        facts: Some(BodyFacts {
            sensor: ImageArea::ApsC,
            offers_raw_sizes: false,
        }),
    },
    // Nikon's Z 9 Reference Guide, page 951, its online manual and its
    // product pages.
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z 9",
        name: "Z 9",
        facts: None,
    },
    // The Z bodies below: each one's online manual and product page.
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z 6",
        name: "Z 6",
        facts: None,
    },
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z 30",
        name: "Z 30",
        facts: None,
    },
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z f",
        name: "Z f",
        facts: None,
    },
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z fc",
        name: "Z fc",
        facts: None,
    },
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z50_2",
        name: "Z50II",
        facts: None,
    },
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z5_2",
        name: "Z5II",
        facts: None,
    },
    Name {
        make: Make::Nikon,
        recorded_model: "NIKON Z6_3",
        name: "Z6III",
        facts: None,
    },
    // Sony's bodies: each one's page in Sony's photo gallery.
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-1",
        name: "α1",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-1M2",
        name: "α1 II",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-6000",
        name: "α6000",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-6700",
        name: "α6700",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7M2",
        name: "α7 II",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7M3",
        name: "α7 III",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7M4",
        name: "α7 IV",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7M5",
        name: "α7 V",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7RM2",
        name: "α7R II",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7RM3",
        name: "α7R III",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7S",
        name: "α7S",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-7SM3",
        name: "α7S III",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-9",
        name: "α9",
        facts: None,
    },
    Name {
        make: Make::Sony,
        recorded_model: "ILCE-9M3",
        name: "α9 III",
        facts: None,
    },
];

/// The entry of the body of `make` whose files record the Model text
/// `recorded_model`, matched whole and with its case, or `None` for a body
/// [`NAMES`] does not hold.
fn entry(make: Make, recorded_model: &str) -> Option<&'static Name> {
    NAMES
        .iter()
        .find(|entry| entry.make == make && entry.recorded_model == recorded_model)
}

/// The name of the body of `make` whose files record the Model text
/// `recorded_model`, or `None` for a body [`NAMES`] does not hold.
fn marketing_name(make: Make, recorded_model: &str) -> Option<&'static str> {
    entry(make, recorded_model).map(|entry| entry.name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;
    use crate::testing::{assert_names, assert_ordered};

    /// The camera of a Z 9 file, every value set, each one different from
    /// the others, so that a getter reading the wrong value is caught.
    fn z9_camera() -> Camera {
        Camera {
            recorded_make: Some("NIKON CORPORATION".to_owned()),
            recorded_model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("3001234".to_owned()),
            owner_name: Some("Claude Monet".to_owned()),
        }
    }

    #[test]
    fn default_camera_is_empty() {
        let camera = Camera::default();
        assert_eq!(camera.recorded_make(), None);
        assert_eq!(camera.recorded_model(), None);
        assert_eq!(camera.serial_number(), None);
        assert_eq!(camera.owner_name(), None);
    }

    #[test]
    fn camera_getters_return_what_was_set() {
        let camera = z9_camera();
        assert_eq!(camera.recorded_make(), Some("NIKON CORPORATION"));
        assert_eq!(camera.recorded_model(), Some("NIKON Z 9"));
        assert_eq!(camera.serial_number(), Some("3001234"));
        assert_eq!(camera.owner_name(), Some("Claude Monet"));
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
                (
                    start.clone().with_owner_name("Berthe Morisot"),
                    Camera {
                        owner_name: Some("Berthe Morisot".to_owned()),
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
            .with_serial_number("3001234")
            .with_owner_name("Claude Monet");
        assert_eq!(built, z9_camera());
    }

    #[test]
    fn a_later_camera_value_replaces_an_earlier_one() {
        let earlier = Camera::default()
            .with_recorded_make("A")
            .with_recorded_model("A")
            .with_serial_number("A")
            .with_owner_name("A");
        // The Z 9's values, each set after an earlier one.
        let camera = earlier
            .with_recorded_make("NIKON CORPORATION")
            .with_recorded_model("NIKON Z 9")
            .with_serial_number("3001234")
            .with_owner_name("Claude Monet");
        assert_eq!(camera, z9_camera());
    }

    #[test]
    fn the_camera_builders_move_an_owned_text_rather_than_copy_it() {
        let make = String::from("NIKON CORPORATION");
        let model = String::from("NIKON Z 9");
        let serial_number = String::from("3001234");
        let owner_name = String::from("Claude Monet");
        let addresses = [
            make.as_ptr(),
            model.as_ptr(),
            serial_number.as_ptr(),
            owner_name.as_ptr(),
        ];
        let camera = Camera::default()
            .with_recorded_make(make)
            .with_recorded_model(model)
            .with_serial_number(serial_number)
            .with_owner_name(owner_name);
        let texts = [
            camera.recorded_make(),
            camera.recorded_model(),
            camera.serial_number(),
            camera.owner_name(),
        ];
        assert_eq!(
            texts,
            [
                Some("NIKON CORPORATION"),
                Some("NIKON Z 9"),
                Some("3001234"),
                Some("Claude Monet")
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
        // A maker is recognized from its whole text, case included: a
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

    #[test]
    fn every_recorded_model_is_named_once_for_its_maker() {
        // A Model text listed twice for one maker would give the first
        // entry's name and hide the second's.
        let mut seen = BTreeSet::new();
        for entry in NAMES {
            assert!(
                seen.insert((entry.make, entry.recorded_model)),
                "{} {:?} is listed twice",
                entry.make,
                entry.recorded_model
            );
        }
    }

    #[test]
    fn every_name_is_written_as_a_name_without_its_brand() {
        // Not empty, with no space at either end and no two together, each
        // space a plain one, and without its maker's name before it in any
        // case: the brand is `make()`'s, and a name that began with it
        // would show twice when `display_name` joins the two.
        for entry in NAMES {
            let (name, model) = (entry.name, entry.recorded_model);
            assert!(!name.is_empty(), "{model:?}: an empty name");
            assert_eq!(name, name.trim(), "{model:?}: spaces at an end");
            assert!(!name.contains("  "), "{model:?}: two spaces together");
            assert!(
                name.chars().filter(|c| c.is_whitespace()).all(|c| c == ' '),
                "{model:?}: a space other than U+0020"
            );
            assert!(
                !name
                    .to_lowercase()
                    .starts_with(&entry.make.name().to_lowercase()),
                "{model:?}: the name {name:?} starts with its maker's"
            );
        }
    }

    #[test]
    fn the_name_needs_the_maker_and_the_model() {
        // A Model text another maker might write gives no name under it,
        // and neither text gives one alone.
        let model_alone = Camera::default().with_recorded_model("ILCE-7M2");
        assert_eq!(model_alone.model(), None);
        let other_maker = model_alone.clone().with_recorded_make("NIKON CORPORATION");
        assert_eq!(other_maker.model(), None);
        assert_eq!(Camera::default().with_recorded_make("SONY").model(), None);
        let sony = model_alone.with_recorded_make("SONY");
        assert_eq!(sony.model(), Some("α7 II"));
    }

    #[test]
    fn the_name_needs_the_whole_model_text_case_included() {
        // As the maker is recognized: a text in another case, part of a
        // text or more than it, and a text with the padding a reader trims,
        // which the setter stores as given.
        let cases = [
            ("SONY", "ILCE-7M2 "),
            ("SONY", " ILCE-7M2"),
            ("SONY", "ilce-7m2"),
            ("SONY", "ILCE-7M"),
            ("SONY", "ILCE-7M2V"),
            ("NIKON CORPORATION", "NIKON Z 9 "),
            ("NIKON CORPORATION", "NIKON Z9"),
            ("NIKON CORPORATION", "nikon z 9"),
            ("NIKON CORPORATION", "Z 9"),
        ];
        for (make, model) in cases {
            let camera = Camera::default()
                .with_recorded_make(make)
                .with_recorded_model(model);
            assert_eq!(camera.model(), None, "{make:?} {model:?}");
        }
    }

    #[test]
    fn the_name_is_read_from_the_recorded_texts_each_time() {
        assert_eq!(Camera::default().model(), None);
        let z9 = Camera::default()
            .with_recorded_make("NIKON CORPORATION")
            .with_recorded_model("NIKON Z 9");
        assert_eq!(z9.model(), Some("Z 9"));
        // No name is kept beside the texts: a new text gives its own.
        let z6iii = z9.with_recorded_model("NIKON Z6_3");
        assert_eq!(z6iii.model(), Some("Z6III"));
        let unlisted = z6iii.with_recorded_model("NIKON D7200");
        assert_eq!(unlisted.model(), None);
        let sony = unlisted
            .with_recorded_make("SONY")
            .with_recorded_model("ILCE-9M3");
        assert_eq!(sony.model(), Some("α9 III"));
        let unknown = sony.with_recorded_make("Sony");
        assert_eq!(unknown.model(), None);
    }

    /// The text to show of a camera whose file records `make` and `model`.
    fn display_name(make: &str, model: &str) -> Option<String> {
        Camera::default()
            .with_recorded_make(make)
            .with_recorded_model(model)
            .display_name()
    }

    #[test]
    fn a_body_of_the_table_shows_its_makers_names() {
        let cases = [
            ("NIKON CORPORATION", "NIKON Z6_3", "Nikon Z6III"),
            ("NIKON CORPORATION", "NIKON Z 9", "Nikon Z 9"),
            ("SONY", "ILCE-7M2", "Sony α7 II"),
        ];
        for (make, model, shown) in cases {
            assert_eq!(
                display_name(make, model).as_deref(),
                Some(shown),
                "{make:?} {model:?}"
            );
        }
    }

    #[test]
    fn every_body_of_the_table_shows_its_maker_and_its_name() {
        // Each entry as a file of the body records it: one of its maker's
        // Make texts and its Model text.
        for entry in NAMES {
            let make = MAKE_TEXTS
                .iter()
                .find(|&&(_, make)| make == entry.make)
                .map(|&(text, _)| text)
                .unwrap_or_else(|| panic!("no Make text for {}", entry.make));
            assert_eq!(
                display_name(make, entry.recorded_model),
                Some(format!("{} {}", entry.make, entry.name)),
                "{make:?} {:?}",
                entry.recorded_model
            );
        }
    }

    #[test]
    fn a_body_outside_the_table_shows_its_model_as_recorded() {
        // Never the maker's name before it: "Nikon NIKON D7200" would show
        // the brand twice. The text keeps what the setter was given.
        let cases = [
            ("NIKON CORPORATION", "NIKON D7200"),
            ("SONY", "ILCE-7RM4"),
            ("NIKON CORPORATION", "NIKON Z 9 "),
        ];
        for (make, model) in cases {
            assert_eq!(
                display_name(make, model).as_deref(),
                Some(model),
                "{make:?} {model:?}"
            );
        }
    }

    #[test]
    fn a_make_raw_files_does_not_recognize_shows_the_model_alone() {
        // A Model of the table under a Make no maker is recognized from.
        for (make, model) in [("Sony", "ILCE-7M2"), ("Nikon", "NIKON Z 9")] {
            assert_eq!(
                display_name(make, model).as_deref(),
                Some(model),
                "{make:?} {model:?}"
            );
        }
    }

    #[test]
    fn the_text_to_show_needs_a_recorded_model() {
        let model_alone = Camera::default().with_recorded_model("NIKON Z 9");
        assert_eq!(model_alone.display_name().as_deref(), Some("NIKON Z 9"));
        let make_alone = Camera::default().with_recorded_make("NIKON CORPORATION");
        assert_eq!(make_alone.display_name(), None);
        assert_eq!(Camera::default().display_name(), None);
    }

    #[test]
    fn the_text_to_show_is_read_from_the_recorded_texts_each_time() {
        // No text is kept beside the recorded ones: a new text gives its
        // own.
        let z9 = Camera::default()
            .with_recorded_make("NIKON CORPORATION")
            .with_recorded_model("NIKON Z 9");
        assert_eq!(z9.display_name().as_deref(), Some("Nikon Z 9"));
        let d7200 = z9.with_recorded_model("NIKON D7200");
        assert_eq!(d7200.display_name().as_deref(), Some("NIKON D7200"));
        let unknown = d7200
            .with_recorded_model("NIKON Z 9")
            .with_recorded_make("Nikon");
        assert_eq!(unknown.display_name().as_deref(), Some("NIKON Z 9"));
    }

    /// The rows of the table under the heading `## {heading}` of the notes
    /// on camera names, in their order: each row's first cell, a text in
    /// backticks, and its third, the name. No cell before the name holds a
    /// `|`, so a row splits on it.
    fn rows_under(notes: &str, heading: &str) -> Vec<(String, String)> {
        cells_under(notes, heading)
            .into_iter()
            .map(|(text, cells)| {
                let Some(name) = cells.get(1).cloned() else {
                    panic!("the row of {text:?} has fewer than three cells");
                };
                (text, name)
            })
            .collect()
    }

    /// The rows of the tables under the heading `## {heading}` of the notes
    /// on camera names, in their order: each row's first cell, a text in
    /// backticks, and its other cells, trimmed. A cell that holds a `|`
    /// splits, so only the cells before the first such one are sound.
    fn cells_under(notes: &str, heading: &str) -> Vec<(String, Vec<String>)> {
        let mut rows = Vec::new();
        let mut inside = false;
        for line in notes.lines() {
            if let Some(title) = line.strip_prefix("## ") {
                inside = title == heading;
                continue;
            }
            if !inside || !line.starts_with('|') {
                continue;
            }
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // The header, whose first cell names the column, and the line
            // under it; every other line of the table is a row.
            if line.starts_with("|---") || cells.get(1).is_some_and(|cell| cell.ends_with(" text"))
            {
                continue;
            }
            let (Some(text), true) = (cells.get(1), line.ends_with('|')) else {
                panic!("a row with no cell, or not closed by a `|`: {line}");
            };
            let text = text
                .strip_prefix('`')
                .and_then(|cell| cell.strip_suffix('`'))
                .unwrap_or_else(|| panic!("a row whose first cell is not in backticks: {line}"));
            // The cells between the first and the line's closing `|`.
            let rest = &cells[2..cells.len() - 1];
            rows.push((
                text.to_owned(),
                rest.iter().map(|&cell| cell.to_owned()).collect(),
            ));
        }
        assert!(!rows.is_empty(), "no rows under \"## {heading}\"");
        rows
    }

    /// The notes on camera names, or `None` in the packaged crate, which
    /// leaves `docs/` out. The skip is keyed on `fuzz/Cargo.toml`, which
    /// the repository has and the packaged crate leaves out too, as the
    /// tables of autofocus points key theirs, so that a renamed or deleted
    /// notes page fails instead of silently skipping.
    fn notes() -> Option<String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fuzz_manifest = root.join("fuzz/Cargo.toml");
        if !fuzz_manifest.is_file() {
            eprintln!(
                "notes skipped: {} does not exist, as in the packaged crate",
                fuzz_manifest.display()
            );
            return None;
        }
        let path = root.join("docs/formats/camera-names.md");
        let notes = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        Some(notes)
    }

    #[test]
    fn the_table_holds_the_bodies_the_notes_list_in_their_order() {
        let Some(notes) = notes() else {
            return;
        };
        let mut listed = Vec::new();
        for (heading, make) in [
            ("3. Nikon bodies", Make::Nikon),
            ("4. Sony bodies", Make::Sony),
        ] {
            for (model, name) in rows_under(&notes, heading) {
                listed.push((make, model, name));
            }
        }
        let table: Vec<(Make, String, String)> = NAMES
            .iter()
            .map(|entry| {
                (
                    entry.make,
                    entry.recorded_model.to_owned(),
                    entry.name.to_owned(),
                )
            })
            .collect();
        // Both ways, and in order: every row is an entry with its name, and
        // every entry a row.
        assert_eq!(table, listed);
    }

    #[test]
    fn the_facts_are_the_ones_the_notes_list_in_their_order() {
        let Some(notes) = notes() else {
            return;
        };
        // Both ways, and in order: every row of section 5 is a body of the
        // table with those facts, and every body with facts a row. Each
        // fact is written as the notes write it: the sensor format by
        // Nikon's name, and "none" for a body that offers no raw sizes.
        let table: Vec<(String, Vec<String>)> = NAMES
            .iter()
            .filter_map(|entry| {
                let facts = entry.facts?;
                let sensor = match facts.sensor {
                    ImageArea::FullFrame => "FX",
                    ImageArea::ApsC => "DX",
                    other => panic!("{:?}: {other:?} is no sensor format", entry.recorded_model),
                };
                let raw_sizes = if facts.offers_raw_sizes {
                    "offered"
                } else {
                    "none"
                };
                Some((
                    entry.recorded_model.to_owned(),
                    vec![sensor.to_owned(), raw_sizes.to_owned()],
                ))
            })
            .collect();
        let listed: Vec<(String, Vec<String>)> =
            cells_under(&notes, "5. Sensor format and raw sizes")
                .into_iter()
                .map(|(model, cells)| {
                    let Some(facts) = cells.get(..2) else {
                        panic!("the row of {model:?} has fewer than three cells");
                    };
                    (model, facts.to_vec())
                })
                .collect();
        assert_eq!(table, listed);
    }

    #[test]
    fn every_body_of_the_table_gives_its_facts_by_its_texts() {
        // Each entry as a file of the body records it, as for its name: a
        // body with facts gives them, and any other none.
        for entry in NAMES {
            let make = MAKE_TEXTS
                .iter()
                .find(|&&(_, make)| make == entry.make)
                .map(|&(text, _)| text)
                .unwrap_or_else(|| panic!("no Make text for {}", entry.make));
            let camera = Camera::default()
                .with_recorded_make(make)
                .with_recorded_model(entry.recorded_model);
            assert_eq!(camera.body_facts(), entry.facts, "{}", entry.recorded_model);
        }
    }

    #[test]
    fn the_facts_need_the_maker_and_the_whole_model_text() {
        let d600 = BodyFacts {
            sensor: ImageArea::FullFrame,
            offers_raw_sizes: false,
        };
        let camera = |make: Option<&str>, model: Option<&str>| {
            let mut camera = Camera::default();
            if let Some(make) = make {
                camera = camera.with_recorded_make(make);
            }
            if let Some(model) = model {
                camera = camera.with_recorded_model(model);
            }
            camera.body_facts()
        };
        assert_eq!(
            camera(Some("NIKON CORPORATION"), Some("NIKON D600")),
            Some(d600)
        );
        assert_eq!(camera(Some("NIKON"), Some("NIKON D600")), Some(d600));
        // Under another maker, under a Make no maker is recognized from,
        // without a Make or a Model, and for a text in another case, part
        // of the text or more than it: none.
        let none = [
            (Some("SONY"), Some("NIKON D600")),
            (Some("Nikon"), Some("NIKON D600")),
            (None, Some("NIKON D600")),
            (Some("NIKON CORPORATION"), None),
            (Some("NIKON CORPORATION"), Some("nikon d600")),
            (Some("NIKON CORPORATION"), Some("NIKON D60")),
            (Some("NIKON CORPORATION"), Some("NIKON D600 ")),
        ];
        for (make, model) in none {
            assert_eq!(camera(make, model), None, "{make:?} {model:?}");
        }
        // A body with a name and no facts, and one with neither.
        assert_eq!(camera(Some("NIKON CORPORATION"), Some("NIKON Z 9")), None);
        assert_eq!(camera(Some("SONY"), Some("ILCE-7M2")), None);
        assert_eq!(camera(Some("NIKON CORPORATION"), Some("NIKON D7200")), None);
    }

    #[test]
    fn the_make_texts_are_the_ones_the_notes_list_in_their_order() {
        let Some(notes) = notes() else {
            return;
        };
        // Both ways, and in order: every row of the makers' table is a text
        // a maker is recognized from, with that maker's name, and every
        // such text a row.
        let texts: Vec<(String, String)> = MAKE_TEXTS
            .iter()
            .map(|&(text, make)| (text.to_owned(), make.name().to_owned()))
            .collect();
        assert_eq!(texts, rows_under(&notes, "2. Makers"));
    }
}
