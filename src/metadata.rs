// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the camera recorded about a picture.
//!
//! Besides the image, a camera records how the picture was taken: by which
//! camera and when, the exposure it chose, the lens it was taken with, and
//! how large the image is. [`Metadata`] holds all of it, read when the file
//! is opened; [`Exposure`] and [`Lens`] hold the exposure and the lens as
//! the file records them, fractions with the numerator and denominator it
//! stored.
//!
//! Every value is optional. A file may not record it, or may record it in a
//! way that cannot be read, such as a tag of the wrong type; either way the
//! value is `None`, and the others are unaffected.

use crate::datetime::DateTime;
use crate::geometry::Size;
use crate::orientation::Orientation;
use crate::rational::{Rational, SignedRational};

/// What the camera recorded about a picture: the camera, when the picture
/// was taken, how to turn it upright, its exposure and lens, and the sizes
/// of its raw data and of its visible image.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read. A damaged value costs only itself: the other
/// values are still read, and the file still opens with its previews.
///
/// `Metadata::default()` is an empty one, with every value `None`, for a
/// caller's tests. `Metadata` is not `Copy`, so that a later value that
/// cannot be copied does not break callers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Metadata {
    pub(crate) make: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) serial_number: Option<String>,
    pub(crate) capture_time: Option<DateTime>,
    pub(crate) orientation: Orientation,
    pub(crate) exposure: Exposure,
    pub(crate) lens: Option<Lens>,
    pub(crate) raw_size: Option<Size>,
    pub(crate) image_size: Option<Size>,
}

impl Metadata {
    /// The camera's maker as the file records it, such as
    /// `NIKON CORPORATION`, cleaned of padding.
    pub fn make(&self) -> Option<&str> {
        self.make.as_deref()
    }

    /// The camera's model as the file records it, such as `NIKON Z 9`,
    /// cleaned of padding.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The camera body's serial number, as text. Many bodies do not record
    /// one in a form the library reads, and then it is `None`.
    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }

    /// When the picture was taken, as the camera's clock showed it: the
    /// Exif DateTimeOriginal, with its fraction of a second and its offset
    /// from UTC when the file records them (see [`DateTime`]).
    pub fn capture_time(&self) -> Option<DateTime> {
        self.capture_time
    }

    /// What a viewer must do to the stored pixels to show the picture
    /// upright: the same value the previews carry
    /// ([`Preview::orientation`](crate::Preview::orientation)), and
    /// [`NoTransforms`](Orientation::NoTransforms) when the file records
    /// none.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// The exposure the picture was taken with. Always present, though each
    /// of its values is optional.
    pub fn exposure(&self) -> &Exposure {
        &self.exposure
    }

    /// The lens the picture was taken with, or `None` when the file
    /// describes no lens.
    pub fn lens(&self) -> Option<&Lens> {
        self.lens.as_ref()
    }

    /// The size of the raw data as stored, before any orientation is
    /// applied.
    ///
    /// It can include masked border pixels and padding beyond the visible
    /// image. On a camera's medium or small raw it is that smaller stored
    /// size, not the sensor's.
    pub fn raw_size(&self) -> Option<Size> {
        self.raw_size
    }

    /// The size of the visible image as stored, before any orientation is
    /// applied.
    ///
    /// It is the default crop the file records, when that lies inside the
    /// raw data: DNG's DefaultCropSize, which Sony writes, or Nikon's crop
    /// area. Otherwise it is the [raw size](Self::raw_size), which then
    /// includes the border pixels: older Nikon bodies, before the D850,
    /// record no crop, and on the public samples their stored raw is up to
    /// 64 pixels wider and 24 taller than the camera's own full-size
    /// preview.
    ///
    /// It is the default crop, not an aspect ratio chosen in the camera. On
    /// a Nikon small raw it counts the stored pixels, which can differ from
    /// the size the camera's menu names. `None` when there is no raw size.
    pub fn image_size(&self) -> Option<Size> {
        self.image_size
    }
}

/// The exposure of a picture: its sensitivity, how long and how wide the
/// lens was open, at what focal length, and the compensation chosen.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read. A zero ISO, exposure time, f-number or
/// focal length means the camera did not know it, and is never returned:
/// the value is `None`, unless, for the ISO, another of the file's
/// sensitivity tags gives it. A zero exposure bias is a real value.
///
/// `Exposure::default()` is an empty one, with every value `None`, for a
/// caller's tests. `Exposure` is not `Copy`, so that a later value that
/// cannot be copied does not break callers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Exposure {
    pub(crate) iso: Option<u32>,
    pub(crate) exposure_time: Option<Rational>,
    pub(crate) f_number: Option<Rational>,
    pub(crate) focal_length: Option<Rational>,
    pub(crate) exposure_bias: Option<SignedRational>,
}

impl Exposure {
    /// The ISO sensitivity, such as 64.
    ///
    /// 65535 means 65535 or more: Exif records that value for any higher
    /// sensitivity, and it is kept when the file records no exact one.
    pub fn iso(&self) -> Option<u32> {
        self.iso
    }

    /// How long the shutter was open, in seconds, as stored, such as
    /// 10/3200 for 1/320 s.
    pub fn exposure_time(&self) -> Option<Rational> {
        self.exposure_time
    }

    /// The f-number the picture was taken at, as stored, such as 900/100
    /// for f/9.
    pub fn f_number(&self) -> Option<Rational> {
        self.f_number
    }

    /// The focal length the picture was taken at, in millimetres, as
    /// stored: the lens's actual focal length, not its 35 mm equivalent.
    pub fn focal_length(&self) -> Option<Rational> {
        self.focal_length
    }

    /// The exposure compensation chosen, in EV, as stored, such as -2/3.
    /// A zero, such as 0/6, means no compensation, not an unknown one.
    pub fn exposure_bias(&self) -> Option<SignedRational> {
        self.exposure_bias
    }
}

/// The lens a picture was taken with, as the file describes it: its name
/// and serial number, and its range of focal lengths with the widest
/// aperture at each end.
///
/// A prime lens has the same shortest and longest focal length. A camera
/// with a fixed lens may describe it by its ranges alone, with no model.
/// At least one value is always present: a file that describes no lens has
/// no `Lens`.
///
/// Focal lengths are in millimetres, the lens's actual ones, not their 35 mm
/// equivalents; apertures are f-numbers. Both are fractions as stored, and
/// a zero, which a file writes for a value it does not know, is `None`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lens {
    make: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    min_focal_length: Option<Rational>,
    max_focal_length: Option<Rational>,
    f_number_at_min_focal_length: Option<Rational>,
    f_number_at_max_focal_length: Option<Rational>,
}

impl Lens {
    /// The lens with these values; `None` when every one of them is `None`,
    /// since a lens is described by at least one.
    ///
    /// The text values come cleaned, as the metadata reader gives them;
    /// the four fractions are in the order the Exif LensSpecification tag
    /// stores them.
    pub(crate) fn new(
        make: Option<String>,
        model: Option<String>,
        serial_number: Option<String>,
        min_focal_length: Option<Rational>,
        max_focal_length: Option<Rational>,
        f_number_at_min_focal_length: Option<Rational>,
        f_number_at_max_focal_length: Option<Rational>,
    ) -> Option<Self> {
        let lens = Self {
            make,
            model,
            serial_number,
            min_focal_length,
            max_focal_length,
            f_number_at_min_focal_length,
            f_number_at_max_focal_length,
        };
        let empty = lens.make.is_none()
            && lens.model.is_none()
            && lens.serial_number.is_none()
            && lens.min_focal_length.is_none()
            && lens.max_focal_length.is_none()
            && lens.f_number_at_min_focal_length.is_none()
            && lens.f_number_at_max_focal_length.is_none();
        (!empty).then_some(lens)
    }

    /// The name of the lens's maker.
    pub fn make(&self) -> Option<&str> {
        self.make.as_deref()
    }

    /// The lens's model name, such as `NIKKOR Z 14-30mm f/4 S`. A name
    /// made only of `-` characters, which Sony writes for a manual lens,
    /// names no lens and is `None`.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The lens's serial number, as the text the file records.
    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }

    /// The shortest focal length of the lens, in millimetres, as stored,
    /// such as 140/10 for 14 mm.
    pub fn min_focal_length(&self) -> Option<Rational> {
        self.min_focal_length
    }

    /// The longest focal length of the lens, in millimetres, as stored;
    /// equal to the shortest on a prime lens.
    pub fn max_focal_length(&self) -> Option<Rational> {
        self.max_focal_length
    }

    /// The widest aperture at the shortest focal length, as an f-number
    /// stored as a fraction, such as 400/100 for f/4.
    pub fn f_number_at_min_focal_length(&self) -> Option<Rational> {
        self.f_number_at_min_focal_length
    }

    /// The widest aperture at the longest focal length, as an f-number
    /// stored as a fraction; on a lens whose aperture does not change as
    /// it zooms, the same as at the shortest.
    pub fn f_number_at_max_focal_length(&self) -> Option<Rational> {
        self.f_number_at_max_focal_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fraction(numerator: u32, denominator: u32) -> Rational {
        Rational {
            numerator,
            denominator,
        }
    }

    /// An exposure of 1/320 s at f/9, 14 mm and ISO 64, with -2/3 EV of
    /// compensation, as a camera stores it.
    fn z9_exposure() -> Exposure {
        Exposure {
            iso: Some(64),
            exposure_time: Some(fraction(10, 3200)),
            f_number: Some(fraction(900, 100)),
            focal_length: Some(fraction(140, 10)),
            exposure_bias: Some(SignedRational {
                numerator: -4,
                denominator: 6,
            }),
        }
    }

    #[test]
    fn default_exposure_is_empty() {
        let exposure = Exposure::default();
        assert_eq!(exposure.iso(), None);
        assert_eq!(exposure.exposure_time(), None);
        assert_eq!(exposure.f_number(), None);
        assert_eq!(exposure.focal_length(), None);
        assert_eq!(exposure.exposure_bias(), None);
    }

    #[test]
    fn exposure_getters_return_what_was_set() {
        let exposure = z9_exposure();
        assert_eq!(exposure.iso(), Some(64));
        assert_eq!(exposure.exposure_time(), Some(fraction(10, 3200)));
        assert_eq!(exposure.f_number(), Some(fraction(900, 100)));
        assert_eq!(exposure.focal_length(), Some(fraction(140, 10)));
        assert_eq!(
            exposure.exposure_bias(),
            Some(SignedRational {
                numerator: -4,
                denominator: 6,
            })
        );
    }

    #[test]
    fn exposure_equality_is_as_stored() {
        let reduced = Exposure {
            exposure_time: Some(fraction(1, 320)),
            ..z9_exposure()
        };
        assert_ne!(z9_exposure(), reduced);
        assert_eq!(z9_exposure(), z9_exposure());
    }

    #[test]
    fn a_lens_with_no_value_is_none() {
        assert_eq!(Lens::new(None, None, None, None, None, None, None), None);
    }

    #[test]
    fn a_lens_with_any_one_value_is_some() {
        let text = || Some("x".to_owned());
        let value = Some(fraction(1, 1));
        let lenses = [
            Lens::new(text(), None, None, None, None, None, None),
            Lens::new(None, text(), None, None, None, None, None),
            Lens::new(None, None, text(), None, None, None, None),
            Lens::new(None, None, None, value, None, None, None),
            Lens::new(None, None, None, None, value, None, None),
            Lens::new(None, None, None, None, None, value, None),
            Lens::new(None, None, None, None, None, None, value),
        ];
        for (place, lens) in lenses.into_iter().enumerate() {
            let lens = lens.unwrap_or_else(|| panic!("value {place} alone gave no lens"));
            let values = [
                lens.make().is_some(),
                lens.model().is_some(),
                lens.serial_number().is_some(),
                lens.min_focal_length().is_some(),
                lens.max_focal_length().is_some(),
                lens.f_number_at_min_focal_length().is_some(),
                lens.f_number_at_max_focal_length().is_some(),
            ];
            // Only the value given is there, so each getter reads its own.
            let expected: Vec<bool> = (0..values.len()).map(|i| i == place).collect();
            assert_eq!(values.to_vec(), expected, "value {place}");
        }
    }

    #[test]
    fn a_fixed_lens_described_by_its_ranges_alone() {
        let lens = Lens::new(
            None,
            None,
            None,
            Some(fraction(90, 10)),
            Some(fraction(720, 10)),
            Some(fraction(28, 10)),
            Some(fraction(45, 10)),
        )
        .unwrap();
        assert_eq!(lens.model(), None);
        assert_eq!(lens.min_focal_length(), Some(fraction(90, 10)));
        assert_eq!(lens.max_focal_length(), Some(fraction(720, 10)));
        assert_eq!(lens.f_number_at_min_focal_length(), Some(fraction(28, 10)));
        assert_eq!(lens.f_number_at_max_focal_length(), Some(fraction(45, 10)));
    }

    #[test]
    fn a_z_lens_reads_back_exactly() {
        let lens = Lens::new(
            Some("Nikon".to_owned()),
            Some("NIKKOR Z 14-30mm f/4 S".to_owned()),
            Some("20012345".to_owned()),
            Some(fraction(140, 10)),
            Some(fraction(300, 10)),
            Some(fraction(400, 100)),
            Some(fraction(400, 100)),
        )
        .unwrap();
        assert_eq!(lens.make(), Some("Nikon"));
        assert_eq!(lens.model(), Some("NIKKOR Z 14-30mm f/4 S"));
        assert_eq!(lens.serial_number(), Some("20012345"));
        assert_eq!(lens.min_focal_length(), Some(fraction(140, 10)));
        assert_eq!(lens.max_focal_length(), Some(fraction(300, 10)));
        assert_eq!(
            lens.f_number_at_min_focal_length(),
            Some(fraction(400, 100))
        );
        assert_eq!(
            lens.f_number_at_max_focal_length(),
            Some(fraction(400, 100))
        );
        // As stored, not reduced: 140/10 is not 14/1.
        assert_ne!(lens.min_focal_length(), Some(fraction(14, 1)));
    }

    #[test]
    fn lenses_and_exposures_collect_in_a_set() {
        // A culler listing the lenses of a folder puts them in a set.
        let lens = |model: &str| {
            Lens::new(None, Some(model.to_owned()), None, None, None, None, None).unwrap()
        };
        let lenses: std::collections::HashSet<Lens> =
            [lens("A"), lens("B"), lens("A")].into_iter().collect();
        assert_eq!(lenses.len(), 2);

        let exposure = |iso| Exposure {
            iso: Some(iso),
            ..Exposure::default()
        };
        let exposures: std::collections::HashSet<Exposure> =
            [exposure(64), exposure(64), exposure(100)]
                .into_iter()
                .collect();
        assert_eq!(exposures.len(), 2);
    }

    /// The metadata of a Z 9 picture, every value set and each one
    /// different from the others, so that a getter reading the wrong value
    /// is caught.
    fn z9_metadata() -> Metadata {
        Metadata {
            make: Some("NIKON CORPORATION".to_owned()),
            model: Some("NIKON Z 9".to_owned()),
            serial_number: Some("3001234".to_owned()),
            capture_time: DateTime::from_exif(b"2024:05:01 10:20:30", Some(b"85"), Some(b"+02:00")),
            orientation: Orientation::Rotate90,
            exposure: z9_exposure(),
            lens: Lens::new(
                None,
                Some("NIKKOR Z 14-30mm f/4 S".to_owned()),
                None,
                None,
                None,
                None,
                None,
            ),
            raw_size: Some(Size {
                width: 8_280,
                height: 5_520,
            }),
            image_size: Some(Size {
                width: 8_256,
                height: 5_504,
            }),
        }
    }

    #[test]
    fn default_metadata_is_empty() {
        let metadata = Metadata::default();
        assert_eq!(metadata.make(), None);
        assert_eq!(metadata.model(), None);
        assert_eq!(metadata.serial_number(), None);
        assert_eq!(metadata.capture_time(), None);
        assert_eq!(metadata.orientation(), Orientation::NoTransforms);
        assert_eq!(metadata.exposure(), &Exposure::default());
        assert_eq!(metadata.lens(), None);
        assert_eq!(metadata.raw_size(), None);
        assert_eq!(metadata.image_size(), None);
    }

    #[test]
    fn metadata_getters_return_what_was_set() {
        let metadata = z9_metadata();
        assert_eq!(metadata.make(), Some("NIKON CORPORATION"));
        assert_eq!(metadata.model(), Some("NIKON Z 9"));
        assert_eq!(metadata.serial_number(), Some("3001234"));
        let time = metadata.capture_time().unwrap();
        assert_eq!(time.to_string(), "2024-05-01T10:20:30.85+02:00");
        assert_eq!(metadata.orientation(), Orientation::Rotate90);
        assert_eq!(metadata.exposure(), &z9_exposure());
        assert_eq!(
            metadata.lens().and_then(Lens::model),
            Some("NIKKOR Z 14-30mm f/4 S")
        );
        assert_eq!(
            metadata.raw_size(),
            Some(Size {
                width: 8_280,
                height: 5_520,
            })
        );
        assert_eq!(
            metadata.image_size(),
            Some(Size {
                width: 8_256,
                height: 5_504,
            })
        );
    }

    #[test]
    fn metadata_collects_in_a_set() {
        // A renamer grouping a folder's pictures by what they record puts
        // their metadata in a set.
        let metadata = |model: &str| Metadata {
            model: Some(model.to_owned()),
            ..Metadata::default()
        };
        let all: std::collections::HashSet<Metadata> =
            [metadata("A"), metadata("B"), metadata("A")]
                .into_iter()
                .collect();
        assert_eq!(all.len(), 2);
    }
}
