// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the camera recorded about a picture.
//!
//! Besides the image, a camera records how the picture was taken: the
//! exposure it chose, and the lens it was taken with. [`Exposure`] and
//! [`Lens`] hold those values as the file records them, fractions with the
//! numerator and denominator it stored.
//!
//! Every value is optional. A file may not record it, or may record it in a
//! way that cannot be read, such as a tag of the wrong type; either way the
//! value is `None`, and the others are unaffected.

use crate::rational::{Rational, SignedRational};

/// The exposure of a picture: its sensitivity, how long and how wide the
/// lens was open, at what focal length, and the compensation chosen.
///
/// Every value is `None` when the file does not record it or records it in
/// a way that cannot be read. A zero ISO, exposure time, f-number or
/// focal length means the camera did not know it, and is `None` too; a
/// zero exposure bias is a real value.
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
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing reads this yet"))]
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

    /// The lens's model name, such as `NIKKOR Z 14-30mm f/4 S`.
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
}
