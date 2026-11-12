// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the Nikon MakerNote records of the picture beside the
//! standard tags: CropArea, the serial number, the ISO, the lens's
//! range and WorldTime.

use std::io::{Read, Seek};

use super::block::MakerNote;
use super::{CROP_AREA, rules};
use crate::error::Result;
use crate::formats::tiff_metadata;
use crate::geometry::Size;
use crate::io::Source;
use crate::rational::Rational;

/// What `makernote`'s CropArea gives, as [`crop_area`] reads it: the size
/// of the visible image, or why there is none. The image size takes only
/// a size; on a crop, the focus point tells the other two apart
/// ([`in_crop`]).
///
/// [`in_crop`]: super::crop::in_crop
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CropArea {
    /// There is no CropArea tag, as on the D810 (public sample 2036).
    Absent,
    /// There is a CropArea tag, but its value cannot be used, or there is
    /// no raw size to check it against.
    Unusable,
    /// The size of the visible image that CropArea records.
    Size(Size),
}

/// The size of the visible image that `makernote`'s CropArea records, in
/// raw data of size `raw`, as [`CropArea::Size`].
///
/// [`CropArea::Absent`] when the tag is missing, or stores its value
/// outside the block, where the reading of the block drops it. Otherwise
/// [`CropArea::Unusable`] when there is no raw size, whatever the tag's
/// type, or when the tag is not of type SHORT or LONG, does not hold
/// exactly four values, or describes a rectangle that
/// [`tiff_metadata::crop`] does not accept: one of zero width or height,
/// or not inside the raw data.
///
/// See the [module documentation](super) for what CropArea is.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn crop_area<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    raw: Option<Size>,
) -> Result<CropArea> {
    let ifd = &makernote.ifd;
    if ifd.entry(CROP_AREA).is_none() {
        return Ok(CropArea::Absent);
    }
    // Without a raw size, the rectangle has nothing to lie inside.
    let Some(raw) = raw else {
        return Ok(CropArea::Unusable);
    };
    let Some([left, top, width, height]) = ifd.get(src, rules::CROP_AREA)? else {
        return Ok(CropArea::Unusable);
    };
    Ok(tiff_metadata::crop([left, top], [width, height], raw)
        .map_or(CropArea::Unusable, CropArea::Size))
}

/// The camera body's serial number that `makernote`'s SerialNumber
/// records, read by the rule every text value of the metadata is read by
/// ([`tiff_metadata::text`]); `None` when the tag is missing, is not of
/// type ASCII, stores its value outside the block, or holds text that rule
/// gives none for, such as the seven spaces the D200 writes (public sample
/// 8079).
///
/// A serial number that is not all digits is returned as written.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn serial_number<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<Option<String>> {
    tiff_metadata::text(src, &makernote.ifd, rules::SERIAL_NUMBER)
}

/// The ISO that `makernote`'s ISO tag records: the second of its values,
/// when it holds exactly two of type SHORT and that second one is above
/// zero; `None` otherwise.
///
/// Most bodies that write the tag write `[0, ISO]`, such as `[0, 200]` on
/// the D300 (public sample 928). Any other shape records no ISO: the D700
/// writes four zero bytes of type UNDEFINED (sample 2255), the Coolpix
/// E5700 `[0, 0]` (sample 4456). Two SHORTs are held in the entry, so this
/// reads nothing from the file, and a tag of another count is not read.
pub(super) fn iso(makernote: &MakerNote) -> Option<u32> {
    let [_, iso] = makernote.ifd.held(rules::ISO)?;
    (iso > 0).then_some(iso)
}

/// The lens's range that `makernote`'s Lens tag records: its shortest and
/// longest focal length, then the f-number at each, in the order of the
/// Exif LensSpecification, as
/// [`Lens::set_range`](crate::metadata::Lens::set_range) takes them.
///
/// The tag must be of type RATIONAL, hold exactly four values and store
/// them inside the block; otherwise all four are `None`. Each value is
/// `None` when its numerator or its denominator is zero, as in the Exif
/// tag ([`Rational::known`]).
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(super) fn lens_range<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
) -> Result<[Option<Rational>; 4]> {
    let Some(values) = makernote.ifd.get(src, rules::LENS)? else {
        return Ok([None; 4]);
    };
    Ok(values.map(|value| Rational::known(value.numerator, value.denominator)))
}

/// The offset of the camera's clock from UTC, in minutes east of UTC, that
/// `makernote`'s WorldTime records: its time zone, plus 60 minutes when
/// its daylight-saving byte is 1; `None` when the tag is missing, is not of
/// type UNDEFINED, does not hold exactly four bytes, or has a
/// daylight-saving byte other than 0 or 1.
///
/// The zone is read in the MakerNote's byte order. The offset is not
/// bounded here:
/// [`DateTime::with_offset_minutes`](crate::datetime::DateTime::with_offset_minutes),
/// which takes it, refuses one beyond 14 hours, as a zone rewritten in the
/// wrong byte order is.
///
/// Four bytes are held in the entry, so this reads nothing from the file,
/// and a tag of another count is not read. See the
/// [module documentation](super) for what WorldTime is.
pub(super) fn world_time(makernote: &MakerNote) -> Option<i16> {
    let ifd = &makernote.ifd;
    let [zone0, zone1, daylight_saving, _date_format] = ifd.held(rules::WORLD_TIME)?;
    let zone = i32::from(ifd.order().u16([zone0, zone1]).cast_signed());
    let offset = match daylight_saving {
        0 => zone,
        1 => zone + 60,
        _ => return None,
    };
    // A zone near the end of the 16-bit range, plus the daylight hour, may
    // not fit; it is far beyond any offset in use anyway.
    i16::try_from(offset).ok()
}

#[cfg(test)]
mod tests {
    use super::super::block::read;
    use super::super::testing::{
        CFA, MAKER_NOTE_VERSION, ORDERS, RAW, SERIAL, Tags, entry_at, exif_file, exif_metadata,
        makernote_metadata, metadata_of, nikon_file_with, raw_file, retyped, tags_block, whole,
        with_makernote,
    };
    use super::super::{CROP_HI_SPEED, ISO, LENS, Picture, SERIAL_NUMBER, WORLD_TIME, metadata};
    use super::*;
    use crate::container::tiff::tags::{
        BODY_SERIAL_NUMBER, DATE_TIME_ORIGINAL, DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE,
        IMAGE_LENGTH, IMAGE_WIDTH, LENS_MAKE, LENS_MODEL, LENS_SERIAL_NUMBER, LENS_SPECIFICATION,
        MAKER_NOTE, OFFSET_TIME_ORIGINAL, PHOTOGRAPHIC_SENSITIVITY, PHOTOMETRIC_INTERPRETATION,
        SUB_SEC_TIME_ORIGINAL,
    };
    use crate::container::tiff::{self, walk};
    use crate::datetime::DateTime;
    use crate::limits::Limits;
    use crate::metadata::{Lens, Metadata};
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{
        BYTE, IFD, IfdId, LONG, Order, RATIONAL, SHORT, SRATIONAL, TiffBuilder, UNDEFINED,
    };
    use std::io::Cursor;

    /// The Z 9's CropArea: the 8256 by 5504 pixels that start 12 columns
    /// and 8 rows into its raw (public samples 5146 to 5148).
    const Z9_CROP_AREA: [u16; 4] = [12, 8, 8256, 5504];

    /// The image size [`Z9_CROP_AREA`] gives.
    const Z9_IMAGE: Size = Size {
        width: 8256,
        height: 5504,
    };

    /// A MakerNote block in byte order `order` whose CropArea is `values`,
    /// as SHORTs.
    fn shorts_block(order: Order, values: &[u16]) -> Vec<u8> {
        tags_block(order, |inner, ifd| {
            inner.short(ifd, CROP_AREA, values);
        })
        .0
    }

    /// A MakerNote block in byte order `order` whose CropArea is `values`,
    /// as LONGs.
    fn longs_block(order: Order, values: &[u32]) -> Vec<u8> {
        tags_block(order, |inner, ifd| {
            inner.long(ifd, CROP_AREA, values);
        })
        .0
    }

    /// The image size of the file [`raw_file`] builds in byte order `order`
    /// with the MakerNote `block`, declared whole, and nothing more in
    /// IFD0; the raw size must be read.
    fn image_size(order: Order, block: &[u8]) -> Option<Size> {
        let count = u64::try_from(block.len()).unwrap();
        let metadata = metadata_of(raw_file(order, block, count, |_, _| {}));
        assert_eq!(metadata.raw_size, Some(RAW), "{order:?}");
        metadata.image_size
    }

    #[test]
    fn a_crop_area_gives_the_image_size() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            assert_eq!(image_size(order, &block), Some(Z9_IMAGE), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_of_longs_is_read() {
        for order in ORDERS {
            let block = longs_block(order, &Z9_CROP_AREA.map(u32::from));
            assert_eq!(image_size(order, &block), Some(Z9_IMAGE), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = shorts_block(inner, &Z9_CROP_AREA);
            assert_eq!(image_size(file, &block), Some(Z9_IMAGE), "{file:?}");
        }
    }

    #[test]
    fn a_makernote_without_a_crop_area_gives_the_raw_size() {
        // As the bodies before the D850 write it.
        for order in ORDERS {
            let block = tags_block(order, |_, _| {}).0;
            assert_eq!(image_size(order, &block), Some(RAW), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_whose_values_run_past_the_block_gives_the_raw_size() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            // The values are the block's last 8 bytes.
            let values: Vec<u8> = Z9_CROP_AREA.iter().flat_map(|&v| order.u16(v)).collect();
            assert_eq!(block[block.len() - 8..], values, "{order:?}");

            // Declared one byte short: the values' last byte lies in the
            // file, after the block.
            let whole = u64::try_from(block.len()).unwrap();
            let file = raw_file(order, &block, whole - 1, |_, _| {});
            assert_eq!(metadata_of(file).image_size, Some(RAW), "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = raw_file(order, &block, whole, |_, _| {});
            assert_eq!(metadata_of(file).image_size, Some(Z9_IMAGE), "{order:?}");
        }
    }

    #[test]
    fn a_crop_area_outside_the_raw_gives_the_raw_size() {
        for order in ORDERS {
            for values in [
                // Wider or taller than the raw by itself.
                [0, 0, 8281, 5520],
                [0, 0, 8280, 5521],
                // One pixel past the raw's edge, with the left column or
                // the top row.
                [12, 8, 8269, 5504],
                [12, 17, 8256, 5504],
            ] {
                let block = shorts_block(order, &values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
            // Sums that would wrap round to 1, inside the raw.
            for values in [[u32::MAX, 8, 2, 5504], [12, u32::MAX, 8256, 2]] {
                let block = longs_block(order, &values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
        }
    }

    #[test]
    fn a_crop_area_of_zero_width_or_height_gives_the_raw_size() {
        for order in ORDERS {
            for values in [[12, 8, 0, 5504], [12, 8, 8256, 0]] {
                let block = shorts_block(order, &values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
        }
    }

    #[test]
    fn a_crop_area_of_other_than_four_values_gives_the_raw_size() {
        for order in ORDERS {
            // Three and five values, stored out of line; and two, which the
            // entry holds itself.
            for values in [&[12, 8, 8256][..], &[12, 8, 8256, 5504, 0], &[12, 8]] {
                let block = shorts_block(order, values);
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, {values:?}"
                );
            }
        }
    }

    #[test]
    fn a_crop_area_of_another_type_gives_the_raw_size() {
        for order in ORDERS {
            // The 16 bytes of four LONGs that would give the Z 9's image,
            // declared as two RATIONALs, as 16 UNDEFINED bytes, or as four
            // IFD offsets, which are numbers too.
            for (typ, count) in [(RATIONAL, 2), (UNDEFINED, 16), (IFD, 4)] {
                let (mut block, entry) = tags_block(order, |inner, ifd| {
                    inner.long(ifd, CROP_AREA, &Z9_CROP_AREA.map(u32::from));
                });
                let entry = usize::try_from(entry).unwrap();
                block[entry + 2..entry + 4].copy_from_slice(&order.u16(typ));
                block[entry + 4..entry + 8].copy_from_slice(&order.u32(count));
                assert_eq!(
                    image_size(order, &block),
                    Some(RAW),
                    "{order:?}, type {typ}"
                );
            }
        }
    }

    #[test]
    fn without_a_raw_size_a_crop_area_gives_no_image_size() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            let count = u64::try_from(block.len()).unwrap();
            let metadata = metadata_of(with_makernote(order, UNDEFINED, &block, count));
            assert_eq!(metadata.raw_size, None, "{order:?}");
            assert_eq!(metadata.image_size, None, "{order:?}");
        }
    }

    #[test]
    fn a_default_crop_comes_before_the_crop_area() {
        for order in ORDERS {
            let block = shorts_block(order, &Z9_CROP_AREA);
            let count = u64::try_from(block.len()).unwrap();
            // One smaller than the crop area, and one of the whole raw,
            // which the image size alone cannot tell from no crop at all.
            for (origin, size) in [([4, 4], [8000, 5000]), ([0, 0], [RAW.width, RAW.height])] {
                let file = raw_file(order, &block, count, |builder, ifd0| {
                    builder.long(ifd0, DEFAULT_CROP_ORIGIN, &origin).long(
                        ifd0,
                        DEFAULT_CROP_SIZE,
                        &size,
                    );
                });
                let expected = Size {
                    width: size[0],
                    height: size[1],
                };
                assert_eq!(
                    metadata_of(file).image_size,
                    Some(expected),
                    "{order:?}, {size:?}"
                );
            }
        }
    }

    /// Another made-up serial number, for the Exif tag, which must win.
    const EXIF_SERIAL: &str = "1234567";

    /// The D300's Lens tag: an 18-70 mm lens of f/3.5 to f/4.5 (public
    /// sample 928).
    const D300_LENS: [(u32, u32); 4] = [(180, 10), (700, 10), (35, 10), (45, 10)];

    /// Another lens's range, a 24-120 mm f/4, for the Exif tag, which must
    /// win.
    const EXIF_LENS: [(u32, u32); 4] = [(240, 10), (1200, 10), (400, 100), (400, 100)];

    /// `values` as the reader returns a lens's range, each fraction as
    /// stored, and `None` where either of its numbers is zero, by the
    /// readers' own check ([`Rational::known`]).
    fn known(values: [(u32, u32); 4]) -> [Option<Rational>; 4] {
        values.map(|(numerator, denominator)| Rational::known(numerator, denominator))
    }

    /// The four fractions of `lens`'s range, in the order of the Exif
    /// LensSpecification.
    fn range_of(lens: &Lens) -> [Option<Rational>; 4] {
        [
            lens.min_focal_length(),
            lens.max_focal_length(),
            lens.f_number_at_min_focal_length(),
            lens.f_number_at_max_focal_length(),
        ]
    }

    #[test]
    fn without_an_exif_serial_number_the_makernotes_is_used() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            });
            assert_eq!(
                metadata.camera.serial_number.as_deref(),
                Some(SERIAL),
                "{order:?}"
            );
        }
    }

    #[test]
    fn without_a_serial_number_in_either_there_is_none() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |_, _| {});
            assert_eq!(metadata.camera.serial_number, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_serial_number_of_another_type_is_not_read() {
        for order in ORDERS {
            // The bytes of the serial number and its NUL, as UNDEFINED and
            // as BYTE.
            let mut bytes = SERIAL.as_bytes().to_vec();
            bytes.push(0);
            for typ in [UNDEFINED, BYTE] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
                });
                let count = u32::try_from(bytes.len()).unwrap();
                let block = retyped(order, block, entry, typ, count);
                let metadata = exif_metadata(order, &block, |_, _| {});
                assert_eq!(metadata.camera.serial_number, None, "{order:?}, type {typ}");
            }
        }
    }

    #[test]
    fn a_makernote_serial_number_past_the_block_is_not_read() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            // The seven digits and the NUL are the block's last 8 bytes.
            assert_eq!(block[block.len() - 8..], *b"7654321\0", "{order:?}");

            // Declared one byte short: the NUL lies in the file, after the
            // block.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).camera.serial_number, None, "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            assert_eq!(
                metadata_of(file).camera.serial_number.as_deref(),
                Some(SERIAL),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_entry_whose_value_runs_past_the_block_is_dropped_when_read() {
        // As in the test above, but on the MakerNote IFD itself: no reader
        // of the IFD can reach the value, whatever it checks.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            for (count, kept) in [(whole(&block) - 1, false), (whole(&block), true)] {
                let file = exif_file(order, &block, count, |_, _| {});
                let mut src = Source::new(Cursor::new(file)).unwrap();
                let file_block = tiff::Block::file(src.len());
                let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();
                let makernote = read(&mut src, &tree, &Limits::default())
                    .unwrap()
                    .expect("a MakerNote");
                let what = format!("{order:?}, declared {count} bytes");
                assert_eq!(makernote.ifd.entry(SERIAL_NUMBER).is_some(), kept, "{what}");
                assert_eq!(makernote.ifd.invalid_entries(), u32::from(!kept), "{what}");
                // MakerNoteVersion, whose four bytes the entry holds, stays.
                assert!(makernote.ifd.entry(MAKER_NOTE_VERSION).is_some(), "{what}");
            }
        }
    }

    #[test]
    fn an_exif_serial_number_wins_over_the_makernotes() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.ascii(ifd, SERIAL_NUMBER, SERIAL);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.ascii(exif, BODY_SERIAL_NUMBER, EXIF_SERIAL);
            });
            assert_eq!(
                metadata.camera.serial_number.as_deref(),
                Some(EXIF_SERIAL),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_makernote_serial_number_of_spaces_is_none() {
        // As the D200 writes it: seven spaces and a NUL (public sample
        // 8079).
        let metadata = makernote_metadata(Order::Big, |inner, ifd| {
            inner.ascii(ifd, SERIAL_NUMBER, "       ");
        });
        assert_eq!(metadata.camera.serial_number, None);
    }

    #[test]
    fn a_makernote_serial_number_that_is_not_digits_is_returned_as_written() {
        let metadata = makernote_metadata(Order::Big, |inner, ifd| {
            inner.ascii(ifd, SERIAL_NUMBER, "ABC123");
        });
        assert_eq!(metadata.camera.serial_number.as_deref(), Some("ABC123"));
    }

    #[test]
    fn without_an_exif_iso_the_makernotes_second_value_is_used() {
        // As most bodies write it: `[0, ISO]`, here the D300's (public
        // sample 928).
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 200]);
            });
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    #[test]
    fn without_an_iso_in_either_there_is_none() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |_, _| {});
            assert_eq!(metadata.exposure.iso, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_iso_of_another_type_is_not_read() {
        for order in ORDERS {
            // `[0, 200]` as LONGs, stored out of line inside the block,
            // which the reader of numbers reads; and the same two numbers
            // declared as IFD offsets, which it reads too.
            let long = tags_block(order, |inner, ifd| {
                inner.long(ifd, ISO, &[0, 200]);
            });
            let ifds = retyped(order, long.0.clone(), long.1, IFD, 2);
            for (label, block) in [("LONG", long.0), ("IFD", ifds)] {
                let metadata = exif_metadata(order, &block, |_, _| {});
                assert_eq!(metadata.exposure.iso, None, "{order:?}, {label}");
            }
        }
    }

    #[test]
    fn a_makernote_iso_of_four_zero_bytes_is_none() {
        // As the D700 writes it: four zero bytes of type UNDEFINED (public
        // sample 2255).
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.undefined(ifd, ISO, &[0; 4]);
            });
            assert_eq!(metadata.exposure.iso, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_iso_of_zero_is_none() {
        // As the Coolpix E5700 writes it: `[0, 0]` (public sample 4456).
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 0]);
            });
            assert_eq!(metadata.exposure.iso, None, "{order:?}");
        }
    }

    #[test]
    fn a_makernote_iso_of_other_than_two_values_is_none() {
        for order in ORDERS {
            // One value, which the entry holds, and three, stored out of
            // line inside the block.
            for values in [&[200][..], &[0, 200, 400]] {
                let metadata = makernote_metadata(order, |inner, ifd| {
                    inner.short(ifd, ISO, values);
                });
                assert_eq!(metadata.exposure.iso, None, "{order:?}, {values:?}");
            }
        }
    }

    #[test]
    fn a_makernote_isos_first_value_is_not_read() {
        // Most bodies write 0 first; the ISO is the second value whatever
        // the first is.
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.short(ifd, ISO, &[1, 3200]);
            });
            assert_eq!(metadata.exposure.iso, Some(3200), "{order:?}");
        }
    }

    #[test]
    fn an_exif_iso_wins_over_the_makernotes() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 200]);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[100]);
            });
            assert_eq!(metadata.exposure.iso, Some(100), "{order:?}");
        }
    }

    #[test]
    fn an_exif_iso_of_zero_alone_gives_the_makernotes() {
        // The standard tags give no ISO for a PhotographicSensitivity of 0,
        // an unknown sensitivity, without another sensitivity tag.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.short(ifd, ISO, &[0, 200]);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.short(exif, PHOTOGRAPHIC_SENSITIVITY, &[0]);
            });
            assert_eq!(metadata.exposure.iso, Some(200), "{order:?}");
        }
    }

    #[test]
    fn without_an_exif_lens_the_makernotes_range_fills_the_lens() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            });
            let lens = metadata.lens;
            assert_eq!(range_of(&lens), known(D300_LENS), "{order:?}");
            assert_eq!(
                (
                    lens.recorded_make(),
                    lens.recorded_model(),
                    lens.serial_number()
                ),
                (None, None, None),
                "{order:?}"
            );
        }
    }

    #[test]
    fn without_a_lens_in_either_the_lens_is_empty() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |_, _| {});
            assert_eq!(metadata.lens, Lens::default(), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_lens_of_another_type_is_not_read() {
        for order in ORDERS {
            // The 32 bytes of the four fractions, declared as SRATIONALs,
            // as eight LONGs, or as 32 UNDEFINED bytes.
            for (typ, count) in [(SRATIONAL, 4), (LONG, 8), (UNDEFINED, 32)] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.rational(ifd, LENS, &D300_LENS);
                });
                let block = retyped(order, block, entry, typ, count);
                let metadata = exif_metadata(order, &block, |_, _| {});
                assert_eq!(metadata.lens, Lens::default(), "{order:?}, type {typ}");
            }
        }
    }

    #[test]
    fn a_makernote_lens_past_the_block_is_not_read() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            })
            .0;
            // The four fractions are the block's last 32 bytes.
            let values: Vec<u8> = D300_LENS
                .iter()
                .flat_map(|&(n, d)| [order.u32(n), order.u32(d)])
                .flatten()
                .collect();
            assert_eq!(block[block.len() - 32..], values, "{order:?}");

            // Declared one byte short: the last fraction's last byte lies
            // in the file, after the block.
            let file = exif_file(order, &block, whole(&block) - 1, |_, _| {});
            assert_eq!(metadata_of(file).lens, Lens::default(), "{order:?}");

            // The control: declared whole, the same bytes are used.
            let file = exif_file(order, &block, whole(&block), |_, _| {});
            let lens = metadata_of(file).lens;
            assert_eq!(range_of(&lens), known(D300_LENS), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_range_wins_over_the_makernotes() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.rational(exif, LENS_SPECIFICATION, &EXIF_LENS);
            });
            let lens = metadata.lens;
            assert_eq!(range_of(&lens), known(EXIF_LENS), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_range_of_zeros_gives_the_makernotes() {
        // Four unknown values, written as 0/0 and as a zero over a
        // denominator, give no range, so the MakerNote's is read.
        for zero in [(0, 0), (0, 10)] {
            for order in ORDERS {
                let block = tags_block(order, |inner, ifd| {
                    inner.rational(ifd, LENS, &D300_LENS);
                })
                .0;
                let metadata = exif_metadata(order, &block, |builder, exif| {
                    builder.rational(exif, LENS_SPECIFICATION, &[zero; 4]);
                });
                let lens = metadata.lens;
                assert_eq!(range_of(&lens), known(D300_LENS), "{zero:?}, {order:?}");
            }
        }
    }

    #[test]
    fn an_exif_lens_range_with_one_known_value_wins_whole() {
        // Not merged value by value: the three unknown values stay unknown.
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS);
            })
            .0;
            let exif_range = [(240, 10), (0, 0), (0, 0), (0, 0)];
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.rational(exif, LENS_SPECIFICATION, &exif_range);
            });
            let lens = metadata.lens;
            assert_eq!(range_of(&lens), known(exif_range), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_with_a_model_alone_takes_the_makernotes_range() {
        // As the Nikon 1 J1 writes it: a LensModel and no LensSpecification
        // in the Exif tags, the range in the MakerNote (public sample 2956).
        for order in ORDERS {
            let j1_range = [(100, 10), (300, 10), (35, 10), (56, 10)];
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &j1_range);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder.ascii(exif, LENS_MODEL, "1 NIKKOR VR 10-30mm f/3.5-5.6");
            });
            let lens = metadata.lens;
            assert_eq!(
                lens.recorded_model(),
                Some("1 NIKKOR VR 10-30mm f/3.5-5.6"),
                "{order:?}"
            );
            assert_eq!(range_of(&lens), known(j1_range), "{order:?}");
        }
    }

    #[test]
    fn an_exif_lens_without_a_range_keeps_its_texts_beside_the_makernotes_range() {
        // The make and serial number too, not only the model, stay as the
        // Exif tags give them when the MakerNote fills the range.
        for order in ORDERS {
            let j1_range = [(100, 10), (300, 10), (35, 10), (56, 10)];
            let block = tags_block(order, |inner, ifd| {
                inner.rational(ifd, LENS, &j1_range);
            })
            .0;
            let metadata = exif_metadata(order, &block, |builder, exif| {
                builder
                    .ascii(exif, LENS_MAKE, "Nikon")
                    .ascii(exif, LENS_MODEL, "1 NIKKOR VR 10-30mm f/3.5-5.6")
                    .ascii(exif, LENS_SERIAL_NUMBER, "20012345");
            });
            let lens = metadata.lens;
            assert_eq!(
                (
                    lens.recorded_make(),
                    lens.recorded_model(),
                    lens.serial_number()
                ),
                (
                    Some("Nikon"),
                    Some("1 NIKKOR VR 10-30mm f/3.5-5.6"),
                    Some("20012345")
                ),
                "{order:?}"
            );
            assert_eq!(range_of(&lens), known(j1_range), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_lens_of_other_than_four_values_is_not_read() {
        for order in ORDERS {
            let metadata = makernote_metadata(order, |inner, ifd| {
                inner.rational(ifd, LENS, &D300_LENS[..3]);
            });
            assert_eq!(metadata.lens, Lens::default(), "{order:?}");
        }
    }

    #[test]
    fn a_makernote_lens_value_of_zero_is_none_and_the_others_are_kept() {
        // An unknown value written as 0/0, and as a zero over a denominator.
        for zero in [(0, 0), (0, 10)] {
            for order in ORDERS {
                let values = [(180, 10), (700, 10), zero, (45, 10)];
                let metadata = makernote_metadata(order, |inner, ifd| {
                    inner.rational(ifd, LENS, &values);
                });
                let lens = metadata.lens;
                assert_eq!(
                    range_of(&lens),
                    [
                        Rational::new(180, 10).ok(),
                        Rational::new(700, 10).ok(),
                        None,
                        Rational::new(45, 10).ok()
                    ],
                    "{zero:?} {order:?}"
                );
            }
        }
    }

    #[test]
    fn the_fallbacks_in_the_other_byte_order_from_the_files_are_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner
                    .short(ifd, ISO, &[0, 200])
                    .ascii(ifd, SERIAL_NUMBER, SERIAL)
                    .rational(ifd, LENS, &D300_LENS);
            })
            .0;
            let metadata = exif_metadata(file, &block, |_, _| {});
            assert_eq!(metadata.exposure.iso, Some(200), "{file:?}");
            assert_eq!(
                metadata.camera.serial_number.as_deref(),
                Some(SERIAL),
                "{file:?}"
            );
            let lens = metadata.lens;
            assert_eq!(range_of(&lens), known(D300_LENS), "{file:?}");
        }
    }

    /// The DateTimeOriginal the WorldTime tests give the Exif IFD: the
    /// D300 fixture's, made up.
    const DATE: &str = "2008:05:17 10:22:31";

    /// [`DATE`] with the SubSecTimeOriginal `34`, as shown without an
    /// offset.
    const LOCAL: &str = "2008-05-17T10:22:31.34";

    /// WorldTime's four bytes in byte order `order`: the time zone `zone`,
    /// in minutes east of UTC, the daylight-saving byte `daylight_saving`,
    /// and the date format 0.
    fn world_time_bytes(order: Order, zone: i16, daylight_saving: u8) -> Vec<u8> {
        let mut bytes = order.u16(zone.cast_unsigned()).to_vec();
        bytes.extend_from_slice(&[daylight_saving, 0]);
        bytes
    }

    /// The capture time of a file in byte order `file` whose MakerNote is
    /// `block`, declared whole, and whose Exif IFD holds what `exif` adds.
    fn capture_time_of(
        file: Order,
        block: &[u8],
        exif: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> Option<DateTime> {
        exif_metadata(file, block, exif).capture_time
    }

    /// The capture time of a file in byte order `order`, MakerNote alike,
    /// whose Exif IFD holds [`DATE`] and its sub-seconds, with the
    /// OffsetTimeOriginal `offset` when there is one, and whose MakerNote
    /// holds what `tags` adds; the time must be read.
    fn time_with(
        order: Order,
        offset: Option<&str>,
        tags: impl FnOnce(&mut TiffBuilder, IfdId),
    ) -> DateTime {
        let block = tags_block(order, tags).0;
        capture_time_of(order, &block, |builder, exif| {
            builder
                .ascii(exif, DATE_TIME_ORIGINAL, DATE)
                .ascii(exif, SUB_SEC_TIME_ORIGINAL, "34");
            if let Some(offset) = offset {
                builder.ascii(exif, OFFSET_TIME_ORIGINAL, offset);
            }
        })
        .expect("a capture time")
    }

    /// The capture time of a file in byte order `order` with [`DATE`], no
    /// OffsetTimeOriginal, and a WorldTime of the time zone `zone` and the
    /// daylight-saving byte `daylight_saving`.
    fn zone_time(order: Order, zone: i16, daylight_saving: u8) -> DateTime {
        time_with(order, None, |inner, ifd| {
            inner.undefined(
                ifd,
                WORLD_TIME,
                &world_time_bytes(order, zone, daylight_saving),
            );
        })
    }

    #[test]
    fn without_an_exif_offset_world_time_gives_it() {
        // The D300's zone, as on public sample 928; the zone of samples
        // 6887 and 7808 to 7810 with daylight saving time on, which agrees
        // with their Exif -04:00; one of daylight saving time on west of
        // UTC, as on sample 961; and UTC itself.
        for order in ORDERS {
            for (zone, daylight_saving, minutes, display) in [
                (60, 0, 60, "+01:00"),
                (-300, 1, -240, "-04:00"),
                (-480, 1, -420, "-07:00"),
                (0, 0, 0, "+00:00"),
            ] {
                let time = zone_time(order, zone, daylight_saving);
                let what = format!("{order:?}, {zone}, {daylight_saving}");
                assert_eq!(time.offset_minutes(), Some(minutes), "{what}");
                assert_eq!(time.to_string(), format!("{LOCAL}{display}"), "{what}");
            }
        }
    }

    #[test]
    fn a_world_time_offset_of_up_to_fourteen_hours_is_taken() {
        for order in ORDERS {
            for (zone, daylight_saving, display) in
                [(840, 0, "+14:00"), (780, 1, "+14:00"), (-840, 0, "-14:00")]
            {
                let time = zone_time(order, zone, daylight_saving);
                assert_eq!(
                    time.to_string(),
                    format!("{LOCAL}{display}"),
                    "{order:?}, {zone}, {daylight_saving}"
                );
            }
        }
    }

    #[test]
    fn a_world_time_offset_beyond_fourteen_hours_gives_none() {
        // With the daylight hour too: 840 plus 60 is 15 hours. And at the
        // end of the 16-bit range, where adding the hour would overflow it.
        for order in ORDERS {
            for (zone, daylight_saving) in
                [(841, 0), (840, 1), (-841, 0), (i16::MAX, 1), (i16::MIN, 0)]
            {
                let time = zone_time(order, zone, daylight_saving);
                let what = format!("{order:?}, {zone}, {daylight_saving}");
                assert_eq!(time.offset_minutes(), None, "{what}");
                assert_eq!(time.to_string(), LOCAL, "{what}");
            }
        }
    }

    #[test]
    fn a_byte_swapped_world_time_gives_none() {
        // As editing software can rewrite it: the zone's two bytes in the
        // other order from the MakerNote's, so 60 reads as 15360, and
        // -300 as -11010.
        for (order, swapped) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            for zone in [60, -300] {
                let time = time_with(order, None, |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, &world_time_bytes(swapped, zone, 0));
                });
                assert_eq!(time.offset_minutes(), None, "{order:?}, {zone}");
                assert_eq!(time.to_string(), LOCAL, "{order:?}, {zone}");
            }
        }
    }

    #[test]
    fn a_world_time_daylight_saving_byte_of_another_value_gives_none() {
        for order in ORDERS {
            for daylight_saving in [2, 255] {
                let time = zone_time(order, 60, daylight_saving);
                assert_eq!(time.offset_minutes(), None, "{order:?}, {daylight_saving}");
            }
        }
    }

    #[test]
    fn an_exif_offset_wins_over_world_time() {
        for order in ORDERS {
            let time = time_with(order, Some("-05:00"), |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
            });
            assert_eq!(time.to_string(), format!("{LOCAL}-05:00"), "{order:?}");
        }
    }

    #[test]
    fn an_unreadable_exif_offset_gives_way_to_world_time() {
        // The standard tags keep the time and drop an offset they cannot
        // read, so they give none, and WorldTime gives it.
        for order in ORDERS {
            let time = time_with(order, Some("+5:00"), |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
            });
            assert_eq!(time.to_string(), format!("{LOCAL}+01:00"), "{order:?}");
        }
    }

    #[test]
    fn without_a_date_time_original_world_time_gives_no_capture_time() {
        for order in ORDERS {
            let block = tags_block(order, |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
            })
            .0;
            let time = capture_time_of(order, &block, |builder, exif| {
                builder.ascii(exif, SUB_SEC_TIME_ORIGINAL, "34");
            });
            assert_eq!(time, None, "{order:?}");
        }
    }

    #[test]
    fn a_world_time_of_another_type_or_length_gives_none() {
        for order in ORDERS {
            // The four bytes of a zone of 60 without daylight saving time,
            // declared as four BYTEs or as two SHORTs.
            for (typ, count) in [(BYTE, 4), (SHORT, 2)] {
                let (block, entry) = tags_block(order, |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, &world_time_bytes(order, 60, 0));
                });
                let block = retyped(order, block, entry, typ, count);
                let time = capture_time_of(order, &block, |builder, exif| {
                    builder.ascii(exif, DATE_TIME_ORIGINAL, DATE);
                })
                .expect("a capture time");
                assert_eq!(time.offset_minutes(), None, "{order:?}, type {typ}");
            }
            // Three of those bytes, held in the entry, and the four and one
            // more, stored elsewhere inside the block.
            let bytes = world_time_bytes(order, 60, 0);
            let mut five = bytes.clone();
            five.push(0);
            for value in [&bytes[..3], &five] {
                let time = time_with(order, None, |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, value);
                });
                assert_eq!(time.offset_minutes(), None, "{order:?}, {value:?}");
            }
        }
    }

    #[test]
    fn without_world_time_the_time_has_no_offset() {
        // As the D200 and older bodies write it (public sample 8079).
        for order in ORDERS {
            let time = time_with(order, None, |_, _| {});
            assert_eq!(time.offset_minutes(), None, "{order:?}");
            assert_eq!(time.to_string(), LOCAL, "{order:?}");
        }
    }

    #[test]
    fn a_world_time_in_the_other_byte_order_from_the_files_is_read() {
        for (file, inner_order) in [(Order::Little, Order::Big), (Order::Big, Order::Little)] {
            let block = tags_block(inner_order, |inner, ifd| {
                inner.undefined(ifd, WORLD_TIME, &world_time_bytes(inner_order, -300, 1));
            })
            .0;
            let time = capture_time_of(file, &block, |builder, exif| {
                builder.ascii(exif, DATE_TIME_ORIGINAL, DATE);
            })
            .expect("a capture time");
            assert_eq!(time.to_string(), "2008-05-17T10:22:31-04:00", "{file:?}");
        }
    }

    #[test]
    fn a_tag_of_another_count_than_its_rule_is_not_read() {
        // Each tag that is read only at an exact count, given one value
        // more, alone in the block, whose last bytes it is, placed from FAR
        // on, where every read fails, with the inner header and the
        // MakerNote IFD before FAR. Its count is checked before its values
        // are read, so it gives nothing, and reading the metadata succeeds.
        let order = Order::Little;
        type Check = fn(&Metadata) -> bool;
        let cases: [(&str, u64, Tags, Check); 5] = [
            (
                "CropArea",
                10,
                |inner, ifd| {
                    inner.short(ifd, CROP_AREA, &[4, 4, 5976, 3984, 0]);
                },
                |metadata| metadata.image_size == Some(RAW),
            ),
            (
                "ISO",
                6,
                |inner, ifd| {
                    inner.short(ifd, ISO, &[0, 200, 0]);
                },
                |metadata| metadata.exposure.iso.is_none(),
            ),
            (
                "Lens",
                40,
                |inner, ifd| {
                    let range = [(240, 10), (700, 10), (28, 10), (28, 10), (1, 1)];
                    inner.rational(ifd, LENS, &range);
                },
                |metadata| !metadata.lens.has_range(),
            ),
            (
                "WorldTime",
                5,
                |inner, ifd| {
                    inner.undefined(ifd, WORLD_TIME, &[0x3C, 0, 0, 0, 0]);
                },
                |metadata| {
                    let time = metadata.capture_time.expect("a capture time");
                    time.offset_minutes().is_none()
                },
            ),
            (
                "CropHiSpeed",
                16,
                |inner, ifd| {
                    let crop = [11, 8280, 5520, 8280, 5520, 0, 0, 0];
                    inner.short(ifd, CROP_HI_SPEED, &crop);
                },
                |metadata| metadata.camera_settings.image_area.is_none(),
            ),
        ];
        for (name, len, tags, check) in cases {
            let block = tags_block(order, tags).0;
            let at = FAR - (whole(&block) - len);
            let bytes = nikon_file_with(order, at, &block, 0, |builder, ifd0, exif| {
                builder
                    .short(ifd0, PHOTOMETRIC_INTERPRETATION, &[CFA])
                    .long(ifd0, IMAGE_WIDTH, &[RAW.width])
                    .long(ifd0, IMAGE_LENGTH, &[RAW.height])
                    .ascii(exif, DATE_TIME_ORIGINAL, "2024:05:01 12:00:00");
                entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, whole(&block));
            });
            // A reader that does not fail gives the same.
            assert!(check(&metadata_of(bytes.clone())), "{name}");

            let limits = Limits::default();
            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let file_block = tiff::Block::file(src.len());
            let tree = walk(&mut src, &file_block, &limits).unwrap();
            let makernote = read(&mut src, &tree, &limits)
                .unwrap()
                .expect("a MakerNote");
            let standard = tiff_metadata::read(&mut src, &tree).unwrap();
            let read = metadata(&mut src, &makernote, standard, Picture::Raw)
                .unwrap_or_else(|err| panic!("{name}: {err:?}"));
            assert!(check(&read), "{name}");
        }
    }
}
