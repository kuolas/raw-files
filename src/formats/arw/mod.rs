// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Sony ARW reader: how the crate recognizes a Sony RAW file, and how
//! it finds the file's previews and reads its metadata.
//!
//! Sony's RAW files are one family:
//!
//! - ARW, which every Sony body since 2006 writes;
//! - SRF and SR2, which the earliest bodies wrote;
//! - ARQ, which Sony's software makes from several pixel-shift frames.
//!
//! Each is one TIFF container with Sony's additions inside: a MakerNote, a
//! private directory, compressed or encrypted raw data. A few cameras that
//! Hasselblad sold under its own name write ARW files too.
//!
//! The parts, in the order a file is read:
//!
//! - [`claims`] says whether a file is a Sony RAW file, by IFD0's Make,
//!   and for a Hasselblad Make by Sony's private pointer.
//! - [`parse`] reads Sony's MakerNote once, with [`sony`], the reader every
//!   format whose files carry that MakerNote shares, and Sony's private
//!   directory ([`private_ifd`]). Then it reads the previews, the metadata
//!   and the entries.
//! - `previews` finds the previews.
//!
//! The MakerNote's FileFormat tag gives a version, but this reader never
//! needs it. The version follows the firmware rather than the body, and
//! the previews are found the same way whatever it says.
//!
//! Nothing the file holds is an error of this reader's. Damage costs only
//! what it touches ([`skip_damage`](crate::error::skip_damage)), and only
//! a failure of the reader is an error.
//!
//! The ARW notes say what each kind of file holds
//! (`docs/formats/arw/container-and-previews.md`, section 1.3). A sample
//! cited here by number is the file of that number on raw.pixls.us.

mod previews;

use std::io::{Read, Seek};

use super::{FormatSpec, Parsed, is_dng, make_starts_with, tiff_metadata};
use crate::container::tiff::{Ifd, Tree, ifd_in};
use crate::entries::{Directory, Entries};
use crate::error::Result;
use crate::format::Format;
use crate::formats::makernote::sony::{self, SONY};
use crate::io::Source;
use crate::limits::Limits;

/// The manufacturer's name that a Hasselblad camera's Make starts with,
/// compared without regard to case.
const HASSELBLAD: &[u8; 10] = b"HASSELBLAD";

/// The ARW format's entry in the TIFF container's list of formats.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R, Tree> {
    FormatSpec {
        format: Format::Arw,
        claims,
        parse,
    }
}

/// Whether the file whose TIFF structure is `tree` is a Sony RAW file:
/// whether it is not a DNG ([`is_dng`]), and its IFD0's Make starts with
/// `SONY` ([`make_starts_with`]). A Make that starts with `HASSELBLAD`
/// passes too, in an IFD0 that holds Sony's private pointer
/// ([`private_pointer`]).
///
/// For a file that is not a DNG, a Sony Make decides alone. Every Sony body
/// writes `SONY`, some padded with spaces, which [`make_starts_with`] trims
/// (`docs/formats/arw/makernote-and-metadata.md`, section 9.3). The SRF
/// files of the earliest bodies carry no private pointer, so the pointer is
/// not asked of a Sony Make.
///
/// A Hasselblad Make needs the pointer. A few Sony cameras that Hasselblad
/// sold under its own name, such as the Lunar, a NEX-7 inside, write
/// ordinary ARW files (sample 4051). Their Make is `HASSELBLAD`, and their
/// IFD0 holds the pointer among Sony's structures. Hasselblad's own cameras
/// write 3FR, FFF and DNG files under the same Make. In exiv2's metadata
/// dumps of the 24 that raw.pixls.us lists, read in place of the files, no
/// 3FR or FFF file holds the pointer, and both DNG files hold DNGVersion
/// (`docs/formats/arw/test-data.md`, section 3.2). The first test, for
/// every Make, has ruled DNGVersion out, so this branch asks only for the
/// pointer.
///
/// The rule rests on those 24 dumps and the one Lunar sample. A 3FR or FFF
/// file with the pointer, a Hasselblad DNG without DNGVersion, or a
/// Sony-built Hasselblad without the pointer would call for checking it
/// again.
///
/// # Errors
///
/// As for [`make_starts_with`].
fn claims<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<bool> {
    if is_dng(tree) {
        return Ok(false);
    }
    if make_starts_with(src, tree, SONY)? {
        return Ok(true);
    }
    // The pointer is tested first because it costs no read, and the Make
    // was read once already.
    let sony_pointer = private_pointer(tree).is_some();
    Ok(sony_pointer && make_starts_with(src, tree, HASSELBLAD)?)
}

/// The rule of the one tag this reader reads itself: IFD0's
/// DNGPrivateData, as Sony writes it (ADR011).
mod rules {
    use crate::container::tiff::tags;
    use crate::container::tiff::{FieldType, Held};

    pub(super) const DNG_PRIVATE_DATA: Held<[u8; 4]> =
        Held::new(tags::DNG_PRIVATE_DATA, &[FieldType::Byte]);
}

/// IFD0's pointer to Sony's private directory, SR2Private, in the shape
/// Sony writes it: the four bytes of the DNGPrivateData entry, of type
/// BYTE, which hold the directory's offset. A DNG's DNGPrivateData is a
/// block of a maker's own data, of any length, and is not one. Asking for
/// this shape keeps such a block out, even in a file without DNGVersion.
fn private_pointer(tree: &Tree) -> Option<[u8; 4]> {
    tree.ifd0().held(rules::DNG_PRIVATE_DATA)
}

/// Sony's private directory, SR2Private, of the file in `src` whose IFDs
/// are `tree`, read within `limits`. It is an ordinary IFD, at the offset
/// IFD0's pointer holds in the file's byte order ([`private_pointer`]). Its
/// offsets count from the start of the file, as IFD0's do.
///
/// `None` when the file has no such pointer, or when the pointer leads:
///
/// - to no IFD that holds together inside the file;
/// - to an IFD with an entry that cannot be read, as the walk takes a
///   SubIFD. The DSLR-A100's leads to a Minolta block that is no IFD
///   (`docs/formats/arw/container-and-previews.md`, section 1.3). Its
///   first bytes count 19,712 entries, more than the default limits allow.
///   Under limits that allow them, they read as entries of types TIFF does
///   not define.
///
/// No SR2Private measured has an entry that cannot be read. Only its table
/// is read. Its entries locate the SR2SubIFD, which is encrypted, and hold
/// the key; that directory is not parsed.
///
/// # Errors
///
/// `Io` when the reader fails.
fn private_ifd<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    limits: &Limits,
) -> Result<Option<Ifd>> {
    let Some(pointer) = private_pointer(tree) else {
        return Ok(None);
    };
    let ifd0 = tree.ifd0();
    let order = ifd0.order();
    let private = ifd_in(src, ifd0.block(), order.u32(pointer), order, limits)?;
    Ok(private.filter(|ifd| ifd.invalid_entries() == 0))
}

/// Reads the previews, the metadata and the entries of a file that
/// [`claims`] accepted.
///
/// The MakerNote is read first, once for the whole file, and Sony's private
/// directory, both within `limits`. Then:
///
/// - **The previews** are the JPEGs that the IFDs of IFD0's chain declare.
///   The shared TIFF preview finder finds them over the whole tree.
/// - **The metadata** is what the shared TIFF metadata reader reads from
///   the standard tags. When there is a MakerNote, [`sony::metadata`] adds
///   what no standard tag records.
/// - **The entries** are those of every directory of the tree, then the
///   MakerNote IFD's
///   ([`MakerNote::add_entries`](sony::MakerNote::add_entries)), then those
///   of SR2Private.
///
/// # Errors
///
/// `Io` when the reader fails.
fn parse<R: Read + Seek>(src: &mut Source<R>, tree: &Tree, limits: &Limits) -> Result<Parsed> {
    let makernote = sony::read(src, tree, limits)?;
    let private = private_ifd(src, tree, limits)?;
    let previews = previews::previews(src, tree)?;
    let standard = tiff_metadata::read(src, tree)?.metadata;
    let mut entries = Entries::of_tree(tree);
    let metadata = match &makernote {
        Some(makernote) => {
            makernote.add_entries(&mut entries);
            sony::metadata(src, makernote, standard)?
        }
        None => standard,
    };
    if let Some(private) = &private {
        entries.push(Directory::Private, private);
    }
    Ok(Parsed {
        previews,
        metadata,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_settings::{CameraSettings, FineTune, WhiteBalance, WhiteBalanceMode};
    use crate::container::tiff::tags::{
        DNG_PRIVATE_DATA, DNG_VERSION, MAKE, MAKER_NOTE, WHITE_BALANCE as EXIF_WHITE_BALANCE,
    };
    use crate::container::tiff::{Block, walk};
    use crate::entries::Value;
    use crate::focus::Focus;
    use crate::formats::makernote::sony::{
        ELECTRONIC_FRONT_CURTAIN, FINE_TUNE, FOCUS_LOCATION, FOCUS_MODE, WHITE_BALANCE,
    };
    use crate::formats::testing::{Src, file};
    use crate::geometry::{Point, Size};
    use crate::rational::SignedRational;
    use crate::tiff_builder::{BYTE, IfdId, Order, RawValue, TiffBuilder, UNDEFINED};
    use std::io::Cursor;

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// Whether the ARW reader claims the file that `build` describes.
    fn claimed(build: impl FnOnce(&mut TiffBuilder, IfdId)) -> bool {
        let (mut src, tree) = file(build);
        claims(&mut src, &tree).unwrap()
    }

    /// Whether the ARW reader claims a file whose Make is `make`, as ASCII.
    fn claimed_make(make: &str) -> bool {
        claimed(|builder, ifd| {
            builder.ascii(ifd, MAKE, make);
        })
    }

    /// The pointer to Sony's private directory as Sony writes it: four
    /// bytes holding a little-endian offset.
    const PRIVATE_POINTER: [u8; 4] = [0x00, 0x10, 0x00, 0x00];

    /// Whether the ARW reader claims a file whose Make is `make`, with
    /// Sony's private pointer in IFD0 and no DNG version.
    fn claimed_make_with_pointer(make: &str) -> bool {
        claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, make)
                .byte(ifd, DNG_PRIVATE_DATA, &PRIVATE_POINTER);
        })
    }

    #[test]
    fn the_make_of_a_sony_body_is_claimed() {
        assert!(claimed_make("SONY"));
    }

    #[test]
    fn the_make_of_the_a100_padded_with_spaces_is_claimed() {
        // Eleven spaces after the name; the builder adds the NUL that makes
        // the value sixteen bytes.
        assert!(claimed_make("SONY           "));
    }

    #[test]
    fn a_make_of_type_utf8_is_claimed() {
        // Exif 3.1 gives the Make as "ASCII or UTF-8".
        assert!(claimed(|builder, ifd| {
            builder.utf8(ifd, MAKE, "SONY");
        }));
    }

    #[test]
    fn a_hasselblad_make_with_sonys_private_pointer_is_claimed() {
        assert!(claimed_make_with_pointer("HASSELBLAD"));
    }

    #[test]
    fn a_hasselblad_make_in_mixed_case_with_the_pointer_is_claimed() {
        assert!(claimed_make_with_pointer("Hasselblad"));
    }

    #[test]
    fn a_hasselblad_make_without_the_pointer_is_not_claimed() {
        assert!(!claimed_make("HASSELBLAD"));
    }

    #[test]
    fn a_hasselblad_make_with_eight_bytes_of_private_data_is_not_claimed() {
        // A DNG's private data block, of a length Sony never writes.
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "HASSELBLAD")
                .byte(ifd, DNG_PRIVATE_DATA, &[0; 8]);
        }));
    }

    #[test]
    fn a_hasselblad_make_with_a_pointer_of_type_undefined_is_not_claimed() {
        // Four bytes, as Sony's pointer, but not of Sony's type BYTE.
        assert!(!claimed(|builder, ifd| {
            builder.ascii(ifd, MAKE, "HASSELBLAD");
            builder.undefined(ifd, DNG_PRIVATE_DATA, &PRIVATE_POINTER);
        }));
    }

    #[test]
    fn a_hasselblad_make_with_the_pointer_and_a_dng_version_is_not_claimed() {
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "HASSELBLAD")
                .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
                .byte(ifd, DNG_PRIVATE_DATA, &PRIVATE_POINTER);
        }));
    }

    #[test]
    fn the_make_of_a_nikon_body_is_not_claimed() {
        assert!(!claimed_make("NIKON CORPORATION"));
    }

    #[test]
    fn the_make_of_another_manufacturer_is_not_claimed() {
        assert!(!claimed_make("Canon"));
    }

    #[test]
    fn a_dng_from_a_sony_camera_is_not_claimed() {
        // As Adobe's converter writes it: Sony's Make, a DNG version and a
        // private data block.
        assert!(!claimed(|builder, ifd| {
            builder
                .ascii(ifd, MAKE, "SONY")
                .byte(ifd, DNG_VERSION, &[1, 4, 0, 0])
                .byte(ifd, DNG_PRIVATE_DATA, &[0; 32]);
        }));
    }

    #[test]
    fn a_claimed_file_without_a_jpeg_has_no_previews() {
        let (mut src, tree) = file(|builder, ifd| {
            builder.ascii(ifd, MAKE, "SONY");
        });
        assert!(claims(&mut src, &tree).unwrap());
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        assert!(parsed.previews.is_empty());
    }

    /// The focus point that parsing a Sony file gives, when its Exif IFD
    /// holds a MakerNote whose IFD, after `prefix`, records the ILCE-9's
    /// autofocus point, FocusMode 3 (AF-C) and its FocusLocation (public
    /// sample 1541); the file must be claimed.
    fn parsed_focus(prefix: &[u8]) -> Option<Focus> {
        let (mut src, tree) = file(|builder, ifd| {
            let exif = builder.ifd();
            let makernote = builder.ifd();
            builder.ascii(ifd, MAKE, "SONY").exif(ifd, exif);
            builder
                .prefix(makernote, prefix)
                .byte(makernote, FOCUS_MODE, &[3])
                .short(makernote, FOCUS_LOCATION, &[6000, 4000, 2662, 2000]);
            builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        });
        assert!(claims(&mut src, &tree).unwrap());
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        parsed.metadata.focus()
    }

    #[test]
    fn the_focus_point_is_read_from_the_makernote() {
        let focus = parsed_focus(b"").expect("a focus point");
        let frame = Size {
            width: 6000,
            height: 4000,
        };
        assert_eq!(focus.frame(), frame);
        assert_eq!(focus.center(), Point { x: 2662, y: 2000 });
        assert_eq!(focus.size(), None);
        assert_eq!(focus.in_focus(), None);
    }

    #[test]
    fn a_makernote_that_holds_no_sony_directory_gives_no_focus_point() {
        // The same IFD after the start of an Olympus-format MakerNote.
        assert_eq!(parsed_focus(b"SONY PI\0"), None);
    }

    #[test]
    fn a_file_without_a_makernote_gives_no_focus_point() {
        let (mut src, tree) = file(|builder, ifd| {
            let exif = builder.ifd();
            builder.ascii(ifd, MAKE, "SONY").exif(ifd, exif);
            builder.undefined(exif, 0x9000, b"0232");
        });
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        assert_eq!(parsed.metadata.focus(), None);
    }

    #[test]
    fn the_camera_settings_are_read_from_the_makernote() {
        // Exif's auto white balance, and a MakerNote that records the
        // underwater setting, which Exif calls auto, a shift of two steps
        // towards amber and three towards magenta, and the electronic front
        // curtain on.
        let (mut src, tree) = file(|builder, ifd| {
            let exif = builder.ifd();
            let makernote = builder.ifd();
            builder.ascii(ifd, MAKE, "SONY").exif(ifd, exif);
            builder.short(exif, EXIF_WHITE_BALANCE, &[0]);
            builder
                .long(makernote, WHITE_BALANCE, &[128])
                .slong(makernote, FINE_TUNE, &[2, 3])
                .long(makernote, ELECTRONIC_FRONT_CURTAIN, &[1]);
            builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        });
        assert!(claims(&mut src, &tree).unwrap());
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        let step = |n| SignedRational::new(n, 1).unwrap();
        let white_balance = WhiteBalance::default()
            .with_mode(WhiteBalanceMode::Underwater)
            .with_fine_tune(FineTune::new(step(-2), step(3)));
        let expected = CameraSettings::default()
            .with_white_balance(white_balance)
            .with_electronic_front_curtain_setting(true);
        assert_eq!(parsed.metadata.camera_settings(), &expected);
    }

    #[test]
    fn the_entries_are_those_of_every_directory_of_the_tree() {
        let (mut src, tree) = file(|builder, ifd0| {
            let (exif, gps) = (builder.ifd(), builder.ifd());
            builder
                .ascii(ifd0, MAKE, "SONY")
                .exif(ifd0, exif)
                .gps(ifd0, gps)
                .short(exif, 0x9207, &[5])
                .byte(gps, 0x0000, &[2, 3, 0, 0]);
        });
        let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
        let entries = &parsed.entries;
        assert_eq!(
            entries.directories(),
            [Directory::Ifd(0), Directory::Exif, Directory::Gps]
        );
        assert!(entries.entry(Directory::Ifd(0), MAKE).is_some());
        assert!(entries.entry(Directory::Exif, 0x9207).is_some());
        assert!(entries.entry(Directory::Gps, 0x0000).is_some());
    }

    /// The SR2Private tags the tests give the private directory: where the
    /// SR2SubIFD lies, its length and its key, as Sony writes them.
    const SR2_SUB_IFD_OFFSET: u16 = 0x7200;
    const SR2_SUB_IFD_LENGTH: u16 = 0x7201;
    const SR2_SUB_IFD_KEY: u16 = 0x7221;

    /// Where the tests' pointer to Sony's private directory leads.
    #[derive(Clone, Copy, Debug)]
    enum Target {
        /// The private directory.
        Private,
        /// The start of a Minolta MRW block, as on the DSLR-A100: its first
        /// two bytes, `\0M`, read as an entry count, announce 19,712 or 77
        /// entries. Zeros follow, entries of type 0, which TIFF does not
        /// define, far enough for either table, so that only the limits,
        /// or the entries' types, refuse it.
        Mrw,
        /// This offset.
        At(u32),
    }

    /// A Sony file in byte order `order` whose IFD0 points, with Sony's
    /// four bytes, at `target`, and holds a private directory with the
    /// SR2SubIFD's offset, length and key; and whose Exif IFD holds a
    /// headerless MakerNote of one entry. A source of it, and its tree.
    fn with_private(order: Order, target: Target) -> (Src, Tree) {
        with_private_typed(order, target, BYTE)
    }

    /// As [`with_private`], with a pointer of type `typ`, four bytes long.
    fn with_private_typed(order: Order, target: Target, typ: u16) -> (Src, Tree) {
        let mut builder = TiffBuilder::new(order);
        let (ifd0, exif, makernote, private) =
            (builder.ifd(), builder.ifd(), builder.ifd(), builder.ifd());
        builder.ascii(ifd0, MAKE, "SONY").exif(ifd0, exif);
        builder.short(makernote, FOCUS_MODE, &[3]);
        builder.undefined_ifd(exif, MAKER_NOTE, makernote);
        builder
            .long(private, SR2_SUB_IFD_OFFSET, &[0x0001_0000])
            .long(private, SR2_SUB_IFD_LENGTH, &[56_958])
            .undefined(private, SR2_SUB_IFD_KEY, &[0x11, 0x22, 0x33, 0x44]);
        let mrw = builder.blob(&[b"\0MRM".as_slice(), &[0; 19_712 * 12 + 4]].concat());
        // The pointer's four bytes move nothing, so a copy with a stand-in
        // gives where the private directory and the MRW block lie.
        let mut probe = builder.clone();
        probe.byte(ifd0, DNG_PRIVATE_DATA, &[0; 4]);
        let layout = probe.build();
        let at = match target {
            Target::Private => u32::try_from(layout.ifd_offset(private)).unwrap(),
            Target::Mrw => u32::try_from(layout.blob_offset(mrw)).unwrap(),
            Target::At(at) => at,
        };
        builder.raw(
            ifd0,
            DNG_PRIVATE_DATA,
            typ,
            4,
            RawValue::Inline(order.u32(at)),
        );
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let block = Block::file(src.len());
        let tree = walk(&mut src, &block, &Limits::default()).unwrap();
        (src, tree)
    }

    #[test]
    fn the_makernote_ifd_and_the_private_directory_follow_the_trees() {
        for order in ORDERS {
            let (mut src, tree) = with_private(order, Target::Private);
            assert!(claims(&mut src, &tree).unwrap(), "{order:?}");
            let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
            let entries = &parsed.entries;
            assert_eq!(
                entries.directories(),
                [
                    Directory::Ifd(0),
                    Directory::Exif,
                    Directory::MakerNote,
                    Directory::Private
                ],
                "{order:?}"
            );
            let tags: Vec<u16> = entries
                .in_directory(Directory::Private)
                .iter()
                .map(crate::Entry::tag)
                .collect();
            assert_eq!(
                tags,
                [SR2_SUB_IFD_OFFSET, SR2_SUB_IFD_LENGTH, SR2_SUB_IFD_KEY],
                "{order:?}"
            );
            assert_eq!(entries.skipped(Directory::Private), 0, "{order:?}");
            // The key's bytes, as recorded; the SR2SubIFD is not parsed.
            let key = entries.entry(Directory::Private, SR2_SUB_IFD_KEY).unwrap();
            assert_eq!(
                key.read(&mut src, 64).unwrap(),
                Value::Undefined(vec![0x11, 0x22, 0x33, 0x44]),
                "{order:?}"
            );
            let offset = entries.entry(Directory::Private, SR2_SUB_IFD_OFFSET);
            let offset = offset.unwrap().read(&mut src, 64).unwrap();
            assert_eq!(offset, Value::Long(vec![0x0001_0000]), "{order:?}");
        }
    }

    #[test]
    fn a_pointer_that_leads_to_no_ifd_gives_no_private_directory_alone() {
        // The MRW block, under the default limits and none; an offset past
        // the end of the file. The MakerNote is listed all the same.
        for order in ORDERS {
            for (target, limits) in [
                (Target::Mrw, Limits::default()),
                (Target::Mrw, Limits::unlimited()),
                (Target::At(u32::MAX), Limits::default()),
            ] {
                let (mut src, tree) = with_private(order, target);
                let parsed = parse(&mut src, &tree, &limits).unwrap();
                assert_eq!(
                    parsed.entries.directories(),
                    [Directory::Ifd(0), Directory::Exif, Directory::MakerNote],
                    "{order:?}, {target:?}, {limits:?}"
                );
                let focus_mode = parsed.entries.entry(Directory::MakerNote, FOCUS_MODE);
                assert!(focus_mode.is_some(), "{order:?}, {target:?}");
            }
        }
    }

    #[test]
    fn a_pointer_not_in_sonys_shape_is_not_followed() {
        // Four bytes that lead to the private directory, but of type
        // UNDEFINED, not BYTE.
        for order in ORDERS {
            let (mut src, tree) = with_private_typed(order, Target::Private, UNDEFINED);
            let parsed = parse(&mut src, &tree, &Limits::default()).unwrap();
            let found = parsed.entries.directories();
            assert!(!found.contains(&Directory::Private), "{order:?}");
            assert!(found.contains(&Directory::MakerNote), "{order:?}");
        }
    }
}
