// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests of the whole TIFF reader on damaged files; this module holds no
//! code of the reader itself.
//!
//! A RAW parser meets truncated and corrupted files as a matter of course,
//! and must answer each with a result, never a panic (ADR008). The tests
//! here hold the reader to that on a few small files built in code, in both
//! byte orders: they cut each file at every length, overwrite each byte in
//! turn with 0x00, 0xFF and its complement, and set the fields that steer
//! the reader (the offset of IFD0, the entry counts, each entry's tag, type,
//! count and value field, the next-IFD pointers) to extreme values. Each
//! damaged file is read as a caller would read it: the header, the walk,
//! then every value of every IFD the walk found. Tests run with debug
//! assertions, so an arithmetic overflow panics too, and is caught.
//!
//! Reading a damaged file may fail; when it does not, what it returns must
//! still keep the promises of the reader: no more IFDs than
//! [`Limits::max_ifds`], no IFD with more entries than
//! [`Limits::max_ifd_entries`], no SubIFD deeper than
//! [`Limits::max_ifd_depth`], and every entry kept can be read. Each damaged
//! file is read with the default limits, with small ones that the undamaged
//! file fits, and with each limit set just below what the undamaged file
//! needs, so that the reader meets every limit.
//!
//! How much the reader allocates is not measured here. Counting allocations
//! takes a global allocator, which takes `unsafe` code, and ADR008 forbids
//! it in this crate.

use std::io::Cursor;
use std::panic;

use super::header::parse_header;
use super::walk::{Location, Tree, walk};
use crate::error::{Error, Result};
use crate::io::Source;
use crate::limits::Limits;
use crate::tiff_builder::{Built, EXIF_IFD, IfdId, Order, SUB_IFDS, TiffBuilder};

const ORDERS: [Order; 2] = [Order::Little, Order::Big];

// Tags that camera files carry, for fixtures that look like theirs. The
// reader gives none of them a meaning.
const IMAGE_WIDTH: u16 = 0x0100;
const MAKE: u16 = 0x010F;
const MODEL: u16 = 0x0110;
const STRIP_OFFSETS: u16 = 0x0111;
const ORIENTATION: u16 = 0x0112;
const STRIP_BYTE_COUNTS: u16 = 0x0117;
const SOFTWARE: u16 = 0x0131;
const JPEG_OFFSET: u16 = 0x0201;
const JPEG_LENGTH: u16 = 0x0202;
const DATE_TIME_ORIGINAL: u16 = 0x9003;

/// The longest fixture, in bytes: each sweep reads every fixture a few
/// times per byte, and this keeps them fast.
const MAX_FIXTURE_LEN: usize = 400;

/// A small valid file to damage, and the IFDs a walk of it finds.
struct Fixture {
    /// What the file is, for failure messages.
    name: String,
    order: Order,
    built: Built,
    /// Every IFD the builder wrote. The walk finds each of them.
    ifds: Vec<IfdId>,
    /// How deep its SubIFDs nest: 0 for none.
    depth: u32,
}

impl Fixture {
    fn new(name: &str, order: Order, builder: &TiffBuilder, ifds: Vec<IfdId>, depth: u32) -> Self {
        let built = builder.build();
        let name = format!("{name} ({order:?})");
        assert!(
            built.bytes.len() <= MAX_FIXTURE_LEN,
            "{name} is {} bytes long",
            built.bytes.len()
        );
        Self {
            name,
            order,
            built,
            ifds,
            depth,
        }
    }

    fn bytes(&self) -> &[u8] {
        &self.built.bytes
    }

    /// The offsets of its IFDs, in ascending order.
    fn ifd_offsets(&self) -> Vec<u64> {
        let mut offsets: Vec<u64> = self
            .ifds
            .iter()
            .map(|&ifd| self.built.ifd_offset(ifd))
            .collect();
        offsets.sort_unstable();
        offsets
    }

    /// The entry count of each of its IFDs, in the order of
    /// [`ifd_offsets`](Self::ifd_offsets).
    fn entry_counts(&self) -> Vec<u16> {
        self.ifd_offsets()
            .into_iter()
            .map(|at| u16_at(self.bytes(), self.order, usize::try_from(at).unwrap()))
            .collect()
    }
}

/// IFD0 alone, as a thumbnail IFD looks: Make and Model, out of line;
/// Orientation, inline; and a JPEG of a few bytes, through its offset and
/// length.
fn small(order: Order) -> Fixture {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let jpeg = builder.blob(&[0xFF, 0xD8, 0xFF, 0xD9]);
    builder
        .ascii(ifd0, MAKE, "NIKON")
        .ascii(ifd0, MODEL, "NIKON Z 9")
        .short(ifd0, ORIENTATION, &[1])
        .long_blob(ifd0, JPEG_OFFSET, jpeg)
        .long_blob_len(ifd0, JPEG_LENGTH, jpeg);
    Fixture::new("small", order, &builder, vec![ifd0], 0)
}

/// IFD0 with two SubIFDs, the first with a SubIFD of its own, and an Exif
/// IFD; each IFD with an out-of-line text and an array of LONGs.
fn with_subifds(order: Order) -> Fixture {
    let mut builder = TiffBuilder::new(order);
    let ifd0 = builder.ifd();
    let [sub, nested, sub2, exif] = [(); 4].map(|()| builder.ifd());
    builder
        .ascii(ifd0, MAKE, "NIKON")
        .long(ifd0, STRIP_OFFSETS, &[0, 0])
        .short(ifd0, ORIENTATION, &[1])
        .subifds(ifd0, &[sub, sub2])
        .exif(ifd0, exif);
    for (ifd, width) in [(sub, 160), (nested, 640), (sub2, 8256)] {
        builder
            .short(ifd, IMAGE_WIDTH, &[width])
            .long(ifd, STRIP_OFFSETS, &[0x1000, 0x2000])
            .ascii(ifd, SOFTWARE, "Ver.1.00");
    }
    builder.subifds(sub, &[nested]);
    builder
        .short(exif, IMAGE_WIDTH, &[1])
        .long(exif, STRIP_BYTE_COUNTS, &[1, 2])
        .ascii(exif, DATE_TIME_ORIGINAL, "2026:10:12 10:00:00");
    Fixture::new(
        "with SubIFDs",
        order,
        &builder,
        vec![ifd0, sub, nested, sub2, exif],
        2,
    )
}

/// IFD0, IFD1 and IFD2 along the chain, IFD1 with one SubIFD.
fn with_chain(order: Order) -> Fixture {
    let mut builder = TiffBuilder::new(order);
    let [ifd0, ifd1, sub, ifd2] = [(); 4].map(|()| builder.ifd());
    builder.next(ifd0, Some(ifd1)).next(ifd1, Some(ifd2));
    builder
        .ascii(ifd0, MAKE, "NIKON")
        .long(ifd0, STRIP_OFFSETS, &[8, 16])
        .short(ifd0, ORIENTATION, &[1]);
    builder
        .short(ifd1, IMAGE_WIDTH, &[160])
        .long(ifd1, STRIP_OFFSETS, &[0x1000, 0x2000])
        .subifds(ifd1, &[sub]);
    builder
        .short(sub, IMAGE_WIDTH, &[8256])
        .ascii(sub, SOFTWARE, "Ver.1.00");
    builder
        .short(ifd2, IMAGE_WIDTH, &[640])
        .long(ifd2, STRIP_BYTE_COUNTS, &[1, 2, 3]);
    Fixture::new(
        "with a chain",
        order,
        &builder,
        vec![ifd0, ifd1, sub, ifd2],
        1,
    )
}

fn fixtures() -> Vec<Fixture> {
    ORDERS
        .into_iter()
        .flat_map(|order| [small(order), with_subifds(order), with_chain(order)])
        .collect()
}

/// Limits far below the defaults, which the fixtures still fit.
fn small_limits() -> Limits {
    Limits::default()
        .max_ifds(8)
        .max_ifd_entries(16)
        .max_ifd_depth(2)
}

/// Limits that `fixture` does not fit, one at a time: one IFD fewer than
/// it holds, one entry fewer than its largest IFD holds, and, when it has
/// SubIFDs, one level fewer than they nest. Read with these, a damaged file
/// meets each limit, so a tree returned past one would show.
fn tight_limits(fixture: &Fixture) -> Vec<(&'static str, Limits)> {
    let ifds = u32::try_from(fixture.ifds.len()).unwrap();
    let entries = fixture.entry_counts().into_iter().max().unwrap();
    let mut sets = vec![
        ("one IFD fewer", Limits::default().max_ifds(ifds - 1)),
        (
            "one entry fewer",
            Limits::default().max_ifd_entries(u32::from(entries) - 1),
        ),
    ];
    if fixture.depth > 0 {
        sets.push((
            "one level fewer",
            Limits::default().max_ifd_depth(fixture.depth - 1),
        ));
    }
    sets
}

/// The limits every damaged copy of `fixture` is read with, each with its
/// name: the default limits, small limits, which the fixture fits, and the
/// [`tight_limits`], which it does not.
fn limit_sets(fixture: &Fixture) -> Vec<(&'static str, Limits)> {
    let mut sets = vec![
        ("the default limits", Limits::default()),
        ("small limits", small_limits()),
    ];
    sets.extend(tight_limits(fixture));
    sets
}

/// Reads `bytes` as a caller would: the header, the walk with `limits`,
/// then every value of every IFD the walk found, through each accessor.
/// Returns the tree with the errors of the value reads, or the error that
/// stopped the walk.
fn read(bytes: &[u8], limits: &Limits) -> Result<(Tree, Vec<Error>)> {
    let mut src = Source::new(Cursor::new(bytes))?;
    let header = parse_header(&mut src)?;
    let tree = walk(&mut src, header, limits)?;
    let mut errors = Vec::new();
    for (_, ifd) in tree.iter() {
        for tag in ifd.tags() {
            errors.extend(ifd.u32s(&mut src, tag).err());
            errors.extend(ifd.ascii(&mut src, tag).err());
            errors.extend(ifd.bytes(&mut src, tag).err());
        }
    }
    Ok((tree, errors))
}

/// Reads `bytes` with `limits` and checks the outcome: an error, or a tree
/// within the limits whose every value can be read. Returns the number of
/// IFDs in the tree, `None` for an error. `what` names the file and the
/// limits in a failure message.
fn check(bytes: &[u8], limits: &Limits, what: impl Fn() -> String) -> Option<usize> {
    let Ok(result) = panic::catch_unwind(|| read(bytes, limits)) else {
        panic!("reading {} panicked", what());
    };
    let (tree, errors) = result.ok()?;
    // Every entry the walk keeps lies inside the file, and the fixtures are
    // far shorter than the 1 MiB an accessor reads at most.
    assert!(
        errors.is_empty(),
        "{}: values cannot be read: {errors:?}",
        what()
    );
    assert_within(&tree, limits, &what);
    Some(tree.len())
}

/// Asserts that `tree` is within `limits`: in the number of IFDs, the
/// entries of each and the depth of each SubIFD.
fn assert_within(tree: &Tree, limits: &Limits, what: &impl Fn() -> String) {
    let max_ifds = usize::try_from(limits.ifds()).unwrap();
    let max_entries = usize::try_from(limits.ifd_entries()).unwrap();
    assert!(
        tree.len() <= max_ifds,
        "{}: a tree of length {}",
        what(),
        tree.len()
    );
    let mut depths: Vec<u32> = Vec::new();
    for (i, (location, ifd)) in tree.iter().enumerate() {
        assert_eq!(
            i == 0,
            location == Location::Ifd0,
            "{}: {location:?} at {i}",
            what()
        );
        assert!(
            ifd.len() <= max_entries,
            "{}: {} entries at {i}",
            what(),
            ifd.len()
        );
        let depth = match location {
            Location::Ifd0 | Location::Chain(_) | Location::Exif => 0,
            Location::Sub { parent, .. } => {
                assert!(parent < i, "{}: parent {parent} of {i}", what());
                depths[parent] + 1
            }
        };
        assert!(
            depth <= limits.ifd_depth(),
            "{}: depth {depth} at {i}",
            what()
        );
        depths.push(depth);
    }
}

/// `file` with the `width` bytes at `at` set to `value` in byte order
/// `order`.
fn with_field(file: &[u8], order: Order, at: usize, width: usize, value: u32) -> Vec<u8> {
    let mut file = file.to_vec();
    let bytes = match width {
        2 => order.u16(u16::try_from(value).unwrap()).to_vec(),
        _ => order.u32(value).to_vec(),
    };
    file[at..at + width].copy_from_slice(&bytes);
    file
}

/// The 16-bit number at `at` of `file`, in byte order `order`.
fn u16_at(file: &[u8], order: Order, at: usize) -> u16 {
    let bytes = [file[at], file[at + 1]];
    match order {
        Order::Little => u16::from_le_bytes(bytes),
        Order::Big => u16::from_be_bytes(bytes),
    }
}

/// A field of a fixture that tells the reader where to go or how much to
/// read, and the values to try in it.
struct Field {
    /// What the field is, for failure messages.
    name: String,
    at: usize,
    /// 2 or 4 bytes.
    width: usize,
    values: Vec<u32>,
}

/// The fields of `fixture` that steer the reader: the offset of IFD0 in
/// the header; and in each IFD the entry count, each entry's tag, type,
/// count and value field, and the next-IFD pointer.
fn structural_fields(fixture: &Fixture) -> Vec<Field> {
    let file = fixture.bytes();
    let len = u32::try_from(file.len()).unwrap();
    // Offsets: at, into and past the header and the end of the file, at
    // the limits of the types, and every IFD of the file, so links can
    // repeat and loop.
    let mut offsets = vec![
        0,
        1,
        7,
        8,
        0x7FFF,
        0xFFFF,
        len - 2,
        len - 1,
        len,
        len + 1,
        u32::MAX,
    ];
    offsets.extend(
        fixture
            .ifd_offsets()
            .into_iter()
            .map(|at| u32::try_from(at).unwrap()),
    );
    // Counts: none, one, many, and those whose length overflows 32 bits for
    // a wider type.
    let counts = vec![0, 1, 0x7FFF, 0xFFFF, 0x4000_0001, 0x8000_0000, u32::MAX];
    // Types: every one TIFF defines, and some it does not.
    let mut types: Vec<u32> = (0..=14).collect();
    types.push(0xFFFF);
    // Tags: the links the walk follows, and two it ignores.
    let tags = vec![u32::from(SUB_IFDS), u32::from(EXIF_IFD), 0, 0xFFFF];

    let mut fields = vec![Field {
        name: "the offset of IFD0".to_owned(),
        at: 4,
        width: 4,
        values: offsets.clone(),
    }];
    for at in fixture.ifd_offsets() {
        let at = usize::try_from(at).unwrap();
        let count = u16_at(file, fixture.order, at);
        fields.push(Field {
            name: format!("the entry count of the IFD at {at}"),
            at,
            width: 2,
            values: vec![0, 1, u32::from(count) + 1, 0x7FFF, 0xFFFF],
        });
        for i in 0..usize::from(count) {
            let entry = at + 2 + 12 * i;
            for (what, offset, width, values) in [
                ("tag", 0, 2, &tags),
                ("type", 2, 2, &types),
                ("count", 4, 4, &counts),
                ("value field", 8, 4, &offsets),
            ] {
                fields.push(Field {
                    name: format!("the {what} of entry {i} of the IFD at {at}"),
                    at: entry + offset,
                    width,
                    values: values.clone(),
                });
            }
        }
        fields.push(Field {
            name: format!("the next-IFD pointer of the IFD at {at}"),
            at: at + 2 + 12 * usize::from(count),
            width: 4,
            values: offsets.clone(),
        });
    }
    fields
}

/// The fixtures are read whole, each IFD at its offset, with every limit
/// they fit, and not whole with the limits they do not: the sweeps damage
/// files that the reader reads, under limits that bind.
#[test]
fn the_fixtures_are_read_whole_only_within_their_limits() {
    for fixture in fixtures() {
        for (name, limits) in [
            ("the default limits", Limits::default()),
            ("small limits", small_limits()),
            ("no limits", Limits::unlimited()),
        ] {
            let (tree, errors) = read(fixture.bytes(), &limits).unwrap();
            assert_eq!(errors.len(), 0, "{} with {name}", fixture.name);
            let mut offsets: Vec<u64> = tree.iter().map(|(_, ifd)| ifd.offset()).collect();
            offsets.sort_unstable();
            assert_eq!(
                offsets,
                fixture.ifd_offsets(),
                "{} with {name}",
                fixture.name
            );
            for (_, ifd) in tree.iter() {
                assert_eq!(ifd.invalid_entries(), 0, "{} with {name}", fixture.name);
            }
        }
        for (name, limits) in tight_limits(&fixture) {
            let found = check(fixture.bytes(), &limits, || {
                format!("{} with {name}", fixture.name)
            });
            assert_ne!(
                found,
                Some(fixture.ifds.len()),
                "{} with {name}",
                fixture.name
            );
        }
    }
}

/// A file cut short anywhere, inside the header, the count of an IFD, an
/// entry, a next-IFD pointer or a value, gives an error or a tree within
/// the limits.
#[test]
fn a_file_cut_short_gives_an_error_or_a_tree_within_the_limits() {
    for fixture in fixtures() {
        let file = fixture.bytes();
        let (mut trees, mut errors) = (0, 0);
        for len in 0..=file.len() {
            for (name, limits) in limit_sets(&fixture) {
                let found = check(&file[..len], &limits, || {
                    format!("{} cut to {len} bytes, with {name}", fixture.name)
                });
                match found {
                    Some(_) => trees += 1,
                    None => errors += 1,
                }
            }
        }
        // Both outcomes occur: the sweep reaches past the header.
        assert!(trees > 0 && errors > 0, "{}", fixture.name);
        // Uncut, the file is read whole, by the same check as its cuts.
        for (name, limits) in [
            ("the default limits", Limits::default()),
            ("small limits", small_limits()),
        ] {
            let found = check(file, &limits, || format!("{} with {name}", fixture.name));
            assert_eq!(
                found,
                Some(fixture.ifds.len()),
                "{} with {name}",
                fixture.name
            );
        }
    }
}

/// A file with any one byte set to 0x00, to 0xFF or to its complement gives
/// an error or a tree within the limits.
#[test]
fn a_file_with_a_byte_overwritten_gives_an_error_or_a_tree_within_the_limits() {
    for fixture in fixtures() {
        let file = fixture.bytes();
        let (mut trees, mut errors) = (0, 0);
        for at in 0..file.len() {
            for byte in [0x00, 0xFF, !file[at]] {
                if byte == file[at] {
                    continue;
                }
                let mut damaged = file.to_vec();
                damaged[at] = byte;
                for (name, limits) in limit_sets(&fixture) {
                    let found = check(&damaged, &limits, || {
                        format!(
                            "{} with byte {at} set to {byte:#04X}, with {name}",
                            fixture.name
                        )
                    });
                    match found {
                        Some(_) => trees += 1,
                        None => errors += 1,
                    }
                }
            }
        }
        assert!(trees > 0 && errors > 0, "{}", fixture.name);
    }
}

/// A file whose offsets, counts, types or tags are set to extreme values,
/// one field at a time, gives an error or a tree within the limits. Such
/// values send the reader past the end of the file, into the header, round
/// a loop, or to a length that overflows.
#[test]
fn extreme_structural_fields_give_an_error_or_a_tree_within_the_limits() {
    for fixture in fixtures() {
        let file = fixture.bytes();
        let fields = structural_fields(&fixture);
        // The header's field, then per IFD its count, four per entry and its
        // next-IFD pointer: a check that the fields were found.
        let entries: usize = fixture.entry_counts().into_iter().map(usize::from).sum();
        assert_eq!(
            fields.len(),
            1 + 2 * fixture.ifds.len() + 4 * entries,
            "{}",
            fixture.name
        );
        for field in fields {
            for &value in &field.values {
                let damaged = with_field(file, fixture.order, field.at, field.width, value);
                for (name, limits) in limit_sets(&fixture) {
                    check(&damaged, &limits, || {
                        format!(
                            "{} with {} set to {value:#X}, with {name}",
                            fixture.name, field.name
                        )
                    });
                }
            }
        }
    }
}
