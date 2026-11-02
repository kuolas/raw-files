// SPDX-License-Identifier: MIT OR Apache-2.0

//! The camera a picture was taken with.
//!
//! A camera names itself in the file it writes: its maker and its model,
//! each in a text tag of the file's first directory, IFD0, and on many
//! bodies its serial number. [`Camera`] holds the three as the file
//! records them, and [`Metadata::camera`](crate::Metadata::camera) gives
//! them for a file.
//!
//! Every value is optional. A file may not record it, or may record it in
//! a way that cannot be read, such as a tag of the wrong type; either way
//! the value is `None`, and the others are unaffected.
//!
//! A program that uses raw-files can build a [`Camera`] for its own tests,
//! without a camera file, through its `with_…` methods. [`Camera`]'s docs
//! show how.

/// The camera body a picture was taken with, as the file names it: its
/// maker and its model as the file records them, and its serial number.
///
/// The maker and the model are the file's text, cleaned as every text value
/// is and nothing more: cut at the first NUL character, trimmed of spaces
/// at both ends, with any bytes that are not UTF-8 replaced by U+FFFD, and
/// `None` when nothing is left or when the text before the NUL is longer
/// than 255 bytes. So a program gets the text that another reader of the
/// same file shows, such as `NIKON CORPORATION` and `NIKON Z 9` from a
/// Nikon Z 9, or `SONY` and `ILCE-7M4` from a Sony ILCE-7M4.
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
/// use raw_files::{Camera, Metadata};
///
/// let z9 = Camera::default()
///     .with_recorded_make("NIKON CORPORATION")
///     .with_recorded_model("NIKON Z 9")
///     .with_serial_number("3001234");
/// assert_eq!(z9.recorded_make(), Some("NIKON CORPORATION"));
/// assert_eq!(z9.recorded_model(), Some("NIKON Z 9"));
/// assert_eq!(z9.serial_number(), Some("3001234"));
/// let metadata = Metadata::default().with_camera(z9.clone());
/// assert_eq!(metadata.camera(), &z9);
///
/// let model_alone = Camera::default().with_recorded_model("NIKON D600");
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
    /// before; [`recorded_make`](Self::recorded_make) reads it.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
