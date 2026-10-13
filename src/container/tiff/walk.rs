// SPDX-License-Identifier: MIT OR Apache-2.0

//! The IFD walk: finding every directory of a TIFF file that the format
//! modules may need.
//!
//! A TIFF-based RAW file holds several images, such as a thumbnail, one or
//! more JPEG previews and the raw sensor data, and each is described by its
//! own IFD. The IFDs reach each other through three kinds of link: the
//! next-IFD pointer at the end of an IFD, which chains IFD0 to IFD1, IFD1 to
//! IFD2, and so on; the SubIFDs tag (0x014A), an array of offsets of child
//! IFDs, which may have SubIFDs of their own; and IFD0's Exif IFD tag
//! (0x8769), the offset of the IFD that holds the capture metadata.
//! [`walk`] follows these links from IFD0 and returns every IFD it finds as
//! a [`Tree`], each with the [`Location`] it was found at.
//!
//! The links are offsets read from the file, so a damaged or hostile file
//! can make them loop, nest without end or multiply. Three things keep the
//! walk bounded on any input:
//!
//! - a **visited set** of the offsets already tried, so no offset is parsed
//!   twice: an IFD that points to itself, a cycle of SubIFDs, or two links to
//!   one IFD cost one parse and give one entry in the tree, at the location
//!   where the walk first found it; a chain that reaches it goes on from its
//!   next pointer, which the set keeps, and passes each IFD once, so a
//!   chain that loops ends. The set keeps a next pointer only for an IFD in
//!   the tree: a SubIFD left out (see below) leaves 0, like an IFD that cannot
//!   be parsed, so a chain that reaches it ends there;
//! - a **depth cap**, [`Limits::max_ifd_depth`]: IFD0 and the chain IFDs are
//!   at depth 0, their SubIFDs at depth 1, and so on, and the SubIFDs of an
//!   IFD at the cap are not followed. A real file never nests that deep, so
//!   this is not an error: what lies deeper is ignored;
//! - a **total cap**, [`Limits::max_ifds`], on the IFDs the walk tries to
//!   parse, whether or not they turn out to be IFDs: a file full of bogus
//!   links costs as much as a file full of IFDs. The attempt that would
//!   cross it is a `LimitExceeded` error.
//!
//! IFD0 must be readable: a file whose IFD0 cannot be parsed has nothing to
//! offer, and the walk fails. Past IFD0, damage costs only the part of the
//! tree it touches. A chain IFD that cannot be parsed ends the chain; a
//! SubIFD or Exif IFD that cannot be parsed is left out; a link whose tag
//! value cannot be read is not followed. No IFD can start inside the 8-byte
//! header, so a link into it is not tried: it is neither parsed nor counted
//! against the total cap, and a chain that reaches it ends. A SubIFD must
//! also be wholly sound, with every entry of a known type and inside the
//! file, or it is left out: the Sony DSLR-A100 writes the offset of its raw
//! data in the SubIFDs tag, and the bytes there are not an IFD. Only a
//! failure of the reader itself stops the walk, as does the total cap.
//!
//! The walk follows nothing else: not the next-IFD pointers of SubIFDs or of
//! the Exif IFD, unless IFD0's chain itself reaches that IFD; not the
//! SubIFDs of the Exif IFD; and not the Exif IFD tag of any IFD but IFD0.
//! The walk runs in a loop over an explicit stack, not by recursion, so a
//! file cannot make it use more call stack however it nests its IFDs.

use std::collections::BTreeMap;
use std::io::{Read, Seek};
use std::mem;
use std::vec;

use super::header::HEADER_LEN;
use super::tags::{EXIF_IFD, SUB_IFDS};
use super::{Header, Ifd, parse_ifd};
use crate::error::{Error, ErrorKind, Result};
use crate::io::{ByteOrder, Source};
use crate::limits::Limits;

/// The base of the offsets inside the file's own IFDs: the start of the
/// file. Every link the walk follows is relative to it, so the offset as
/// written is the absolute offset.
const BASE: u64 = 0;

/// Where in a TIFF file the walk found an IFD: which link led to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Location {
    /// The first IFD, the one the header points to.
    Ifd0,
    /// The IFD this many steps along IFD0's chain of next-IFD pointers:
    /// `Chain(1)` is IFD1, the IFD that IFD0's next-IFD pointer leads to,
    /// and `Chain(2)` is IFD2. A step that reaches an IFD the walk found
    /// first through another link leaves that IFD at its first location,
    /// but still counts: the IFD after it is one step further.
    Chain(u32),
    /// A SubIFD: a child of another IFD of the tree, through its SubIFDs tag
    /// (0x014A).
    Sub {
        /// The parent's position in the [`Tree`], for [`Tree::get`].
        parent: usize,
        /// The position of this IFD's offset in the parent's SubIFDs array,
        /// from 0.
        index: u32,
    },
    /// IFD0's Exif IFD, through its Exif IFD tag (0x8769).
    Exif,
}

/// The IFDs of a TIFF file that [`walk`] found, each with its
/// [`Location`].
///
/// The IFDs are in the order the walk found them, which is deterministic:
/// IFD0; then IFD0's SubIFDs, depth first, each followed by its own
/// SubIFDs, in the order of the SubIFDs arrays; then IFD0's Exif IFD; then
/// IFD1 and its SubIFDs in the same way, IFD2 and its SubIFDs, and so on
/// along the chain. IFD0 is always first, and an IFD appears once, at the
/// first location it was found at.
#[derive(Clone, Debug)]
pub(crate) struct Tree {
    /// Never empty: IFD0 is always the first.
    ifds: Vec<(Location, Ifd)>,
}

// The NEF reader's claim reads IFD0; no format reads the rest of the tree
// yet. Each `expect` fails the build once its method has a caller, so it
// is then removed rather than left behind. `iter` has none because the
// preview finder, which carries the `expect` for its path, calls it, and
// `exif` calls `iter`: some compilers count an item under
// `expect(dead_code)` as used code, and so what it calls, which would
// leave an `expect` on `iter` unfulfilled there.
impl Tree {
    /// IFD0, the IFD the header points to.
    pub(crate) fn ifd0(&self) -> &Ifd {
        // `walk` returns a tree only once IFD0 is in it.
        &self.ifds[0].1
    }

    /// Every IFD with its location, in the order of the tree.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Location, &Ifd)> {
        self.ifds.iter().map(|(location, ifd)| (*location, ifd))
    }

    /// IFD0's Exif IFD, if the walk found it there.
    ///
    /// `None` also when the Exif IFD tag points to an IFD the walk had
    /// already found at another location.
    #[cfg_attr(not(test), expect(dead_code, reason = "no format reads the tree yet"))]
    pub(crate) fn exif(&self) -> Option<&Ifd> {
        self.iter()
            .find(|&(location, _)| location == Location::Exif)
            .map(|(_, ifd)| ifd)
    }

    /// The number of IFDs in the tree, at least 1.
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing reads this method yet"))]
    pub(crate) fn len(&self) -> usize {
        self.ifds.len()
    }

    /// The IFD at position `i` of the tree, with its location; `None` past
    /// the end. Position 0 is IFD0.
    #[cfg_attr(not(test), expect(dead_code, reason = "nothing reads this method yet"))]
    pub(crate) fn get(&self, i: usize) -> Option<(Location, &Ifd)> {
        self.ifds.get(i).map(|(location, ifd)| (*location, ifd))
    }
}

/// Walks the IFDs of the TIFF file in `src` whose header is `header`: IFD0,
/// its chain of next IFDs, the SubIFDs of each of those, recursively, and
/// IFD0's Exif IFD.
///
/// See the [module documentation](self) for what is followed, what is left
/// out and the order of the tree.
///
/// # Errors
///
/// `Malformed` if IFD0 lies inside the header, at IFD0's offset; whatever
/// [`parse_ifd`] returns for IFD0 if it cannot be read; `LimitExceeded` if
/// the walk would try to parse more IFDs than [`Limits::max_ifds`] allows,
/// at the offset of the IFD that would cross it; `Io` if the reader fails.
pub(crate) fn walk<R: Read + Seek>(
    src: &mut Source<R>,
    header: Header,
    limits: &Limits,
) -> Result<Tree> {
    let at = u64::from(header.ifd0);
    if at < HEADER_LEN {
        return Err(Error::malformed("IFD0 lies inside the TIFF header", at));
    }
    let mut walker = Walker {
        src,
        order: header.order,
        limits,
        visited: BTreeMap::new(),
        attempts: 0,
        ifds: Vec::new(),
    };

    // IFD0 is the one IFD whose failure is the walk's.
    walker.claim(at)?;
    let (ifd0, mut next) = parse_ifd(walker.src, BASE, at, header.order, limits)?;
    walker.place(Location::Ifd0, ifd0)?;

    let exif = skip_damage(walker.ifds[0].1.u32(walker.src, EXIF_IFD))?.flatten();
    if let Some(offset) = exif
        && let Some((ifd, next)) = walker.candidate(u64::from(offset))?
    {
        walker.visited.insert(u64::from(offset), next);
        walker.ifds.push((Location::Exif, ifd));
    }

    let mut step: u32 = 1;
    while next != 0 {
        let at = u64::from(next);
        if !walker.visited.contains_key(&at) {
            // A new IFD. One that cannot be parsed, or a link into the
            // header, ends the chain.
            let Some((ifd, after)) = walker.candidate(at)? else {
                break;
            };
            walker.visited.insert(at, after);
            walker.place(Location::Chain(step), ifd)?;
        }
        // New or found before, the IFD takes this step; one found before,
        // through another link or by this chain on a loop, keeps its first
        // location. The chain goes on from the pointer recorded for it and
        // takes it, leaving 0, so the chain goes on from each IFD at most
        // once and a loop ends.
        next = walker.visited.get_mut(&at).map_or(0, mem::take);
        // Each step either claims a new offset or takes a pointer recorded
        // when an earlier claim was parsed, so the total cap bounds the
        // steps to twice its value; saturating keeps the largest caps from
        // overflowing.
        step = step.saturating_add(1);
    }

    Ok(Tree { ifds: walker.ifds })
}

/// The state of one walk.
struct Walker<'a, R> {
    src: &'a mut Source<R>,
    order: ByteOrder,
    limits: &'a Limits,
    /// The absolute offset of every IFD the walk has tried to parse, whether
    /// or not it could, with the next-IFD pointer read there that the chain
    /// has yet to follow. The pointer is kept only for an IFD in the tree:
    /// 0 when the IFD could not be parsed, when it is a SubIFD the walk left
    /// out, when it is IFD0, whose pointer the chain starts from, and once
    /// the chain has gone on from it.
    visited: BTreeMap<u64, u32>,
    /// How many IFDs the walk has tried to parse: the size of `visited`, at
    /// most [`Limits::max_ifds`].
    attempts: u32,
    /// The tree so far.
    ifds: Vec<(Location, Ifd)>,
}

/// The SubIFDs of one IFD of the tree that the walk has yet to try.
struct Children {
    /// The parent's position in the tree.
    parent: usize,
    /// The depth of the children: one more than the parent's.
    depth: u32,
    /// The children's offsets, as written, each with its position in the
    /// parent's SubIFDs array.
    offsets: std::iter::Zip<vec::IntoIter<u32>, std::ops::RangeFrom<u32>>,
}

impl<R: Read + Seek> Walker<'_, R> {
    /// Marks the IFD at `at` as tried, with no next-IFD pointer recorded
    /// yet, and counts the attempt; `false`, with nothing counted, if it was
    /// tried before or lies inside the header.
    ///
    /// No IFD can start inside the header, so a link there is not tried: it
    /// is neither parsed nor counted. (An IFD0 there is an error, which
    /// [`walk`] reports before it claims IFD0.)
    ///
    /// # Errors
    ///
    /// `LimitExceeded` at `at` if the walk has already tried as many IFDs as
    /// the total cap allows.
    fn claim(&mut self, at: u64) -> Result<bool> {
        if at < HEADER_LEN || self.visited.contains_key(&at) {
            return Ok(false);
        }
        if self.attempts >= self.limits.ifds() {
            return Err(Error::limit("more IFDs than the limit allows", at));
        }
        self.attempts += 1;
        self.visited.insert(at, 0);
        Ok(true)
    }

    /// Parses the IFD at `at`, reached by a link past IFD0: `None` if it was
    /// tried before, lies inside the header or cannot be parsed. Returns the
    /// IFD and its next-IFD pointer as written. It does not record the
    /// pointer: the caller does, in `visited`, only once it keeps the IFD in
    /// the tree, so that an IFD it rejects cannot feed a chain.
    ///
    /// # Errors
    ///
    /// As for [`claim`](Self::claim); `Io` if the reader fails.
    fn candidate(&mut self, at: u64) -> Result<Option<(Ifd, u32)>> {
        if !self.claim(at)? {
            return Ok(None);
        }
        skip_damage(parse_ifd(self.src, BASE, at, self.order, self.limits))
    }

    /// Adds `ifd`, at depth 0, to the tree at `location`, then its SubIFDs,
    /// depth first, each followed by its own SubIFDs.
    ///
    /// # Errors
    ///
    /// As for [`candidate`](Self::candidate).
    fn place(&mut self, location: Location, ifd: Ifd) -> Result<()> {
        self.ifds.push((location, ifd));
        let mut stack: Vec<Children> = Vec::new();
        stack.extend(self.children(self.ifds.len() - 1, 0)?);
        // One element per IFD on the path from `ifd` down to the IFD being
        // tried, so the depth cap bounds the stack.
        while let Some(children) = stack.last_mut() {
            let Some((offset, index)) = children.offsets.next() else {
                stack.pop();
                continue;
            };
            let (parent, depth) = (children.parent, children.depth);
            // A SubIFD's own next-IFD pointer is not followed, but is kept for
            // a chain that reaches the SubIFD, if the SubIFD is kept.
            if let Some((child, next)) = self.candidate(u64::from(offset))?
                && child.invalid_entries() == 0
            {
                self.visited.insert(u64::from(offset), next);
                self.ifds.push((Location::Sub { parent, index }, child));
                stack.extend(self.children(self.ifds.len() - 1, depth)?);
            }
        }
        Ok(())
    }

    /// The SubIFDs of the IFD at position `parent` of the tree, which lies
    /// at `depth`: `None` if it has no SubIFDs tag that can be read, or lies
    /// at the depth cap.
    ///
    /// # Errors
    ///
    /// `Io` if the reader fails.
    fn children(&mut self, parent: usize, depth: u32) -> Result<Option<Children>> {
        if depth >= self.limits.ifd_depth() {
            return Ok(None);
        }
        let Some((_, ifd)) = self.ifds.get(parent) else {
            return Ok(None);
        };
        let offsets = skip_damage(ifd.u32s(self.src, SUB_IFDS))?.flatten();
        Ok(offsets.map(|offsets| Children {
            parent,
            // Below the cap, which is a `u32`: this cannot overflow.
            depth: depth + 1,
            // The array is at most 1 MiB, far fewer than 2^32 offsets, so
            // the positions cannot run out.
            offsets: offsets.into_iter().zip(0..),
        }))
    }
}

/// Turns damage in the file into `None`, keeping only a failure of the
/// reader as an error.
///
/// The rule wherever damage should cost only the part of the file it
/// touches: past IFD0, an IFD that cannot be parsed or a tag value that
/// cannot be read costs the walk only what it would have led to, and the
/// preview finder skips an IFD whose tags it cannot read. An `Io` error is
/// not damage: it says nothing about the file, and the reader may fail
/// again on the next read.
pub(crate) fn skip_damage<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.kind() == ErrorKind::Io => Err(err),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::tiff::header::parse_header;
    use crate::tiff_builder::{Built, IfdId, LONG, Order, RawValue, TiffBuilder};
    use std::io::{self, Cursor, SeekFrom};

    const ORDERS: [Order; 2] = [Order::Little, Order::Big];

    /// The tag each test IFD carries, as a SHORT, to tell it apart: its id.
    const ID: u16 = 0x0100;

    /// Creates an IFD holding `id` in tag [`ID`].
    fn ifd(builder: &mut TiffBuilder, id: u16) -> IfdId {
        let ifd = builder.ifd();
        builder.short(ifd, ID, &[id]);
        ifd
    }

    fn open(bytes: &[u8]) -> Source<Cursor<Vec<u8>>> {
        Source::new(Cursor::new(bytes.to_vec())).unwrap()
    }

    /// Reads the header of `bytes` and walks the file with `limits`.
    fn try_walk(bytes: &[u8], limits: &Limits) -> Result<(Source<Cursor<Vec<u8>>>, Tree)> {
        let mut src = open(bytes);
        let header = parse_header(&mut src)?;
        let tree = walk(&mut src, header, limits)?;
        Ok((src, tree))
    }

    /// Walks `built` with the default limits, which must succeed.
    fn walked(built: &Built) -> (Source<Cursor<Vec<u8>>>, Tree) {
        try_walk(&built.bytes, &Limits::default()).unwrap()
    }

    /// The id of each IFD of the tree, read through the tree, in its order.
    fn ids<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Vec<u32> {
        tree.iter()
            .map(|(_, ifd)| ifd.u32(src, ID).unwrap().expect("every IFD has an id"))
            .collect()
    }

    fn locations(tree: &Tree) -> Vec<Location> {
        tree.iter().map(|(location, _)| location).collect()
    }

    fn assert_error<T>(result: Result<T>, kind: ErrorKind, offset: u64) {
        let Err(err) = result else {
            panic!("the walk should fail");
        };
        assert_eq!(err.kind(), kind, "{err:?}");
        assert_eq!(err.offset(), Some(offset), "{err:?}");
    }

    #[test]
    fn ifd0_its_subifds_and_its_exif_ifd() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (a, b, exif) = (
                ifd(&mut builder, 1),
                ifd(&mut builder, 2),
                ifd(&mut builder, 3),
            );
            builder.subifds(ifd0, &[a, b]).exif(ifd0, exif);
            let built = builder.build();
            let (mut src, tree) = walked(&built);

            assert_eq!(tree.len(), 4);
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3]);
            assert_eq!(
                locations(&tree),
                [
                    Location::Ifd0,
                    Location::Sub {
                        parent: 0,
                        index: 0
                    },
                    Location::Sub {
                        parent: 0,
                        index: 1
                    },
                    Location::Exif,
                ]
            );
            assert_eq!(tree.ifd0().offset(), built.ifd_offset(ifd0));
            assert_eq!(tree.exif().unwrap().offset(), built.ifd_offset(exif));
            let (location, second) = tree.get(2).unwrap();
            assert_eq!(
                location,
                Location::Sub {
                    parent: 0,
                    index: 1
                }
            );
            assert_eq!(second.offset(), built.ifd_offset(b));
            // A SubIFD's entries are read through the tree like any IFD's.
            assert_eq!(second.u32(&mut src, ID).unwrap(), Some(2));
            assert!(tree.get(4).is_none());
        }
    }

    #[test]
    fn a_file_without_exif_ifd_has_none() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = ifd(&mut builder, 0);
        let a = ifd(&mut builder, 1);
        builder.subifds(ifd0, &[a]);
        let (_, tree) = walked(&builder.build());
        assert_eq!(tree.len(), 2);
        assert!(tree.exif().is_none());
    }

    #[test]
    fn subifds_of_type_ifd_are_followed() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let a = ifd(&mut builder, 1);
            // Type 13, IFD (TIFF Technical Note 1), inline. The child's
            // offset is known before the build: IFD0 at 8 holds two entries.
            builder.raw(ifd0, SUB_IFDS, 13, 1, RawValue::Inline(order.u32(8 + 30)));
            let built = builder.build();
            assert_eq!(built.ifd_offset(a), 8 + 30);
            let (mut src, tree) = walked(&built);
            assert_eq!(ids(&mut src, &tree), [0, 1]);
        }
    }

    #[test]
    fn the_chain_is_followed() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (ifd1, ifd2, sub) = (
                ifd(&mut builder, 1),
                ifd(&mut builder, 2),
                ifd(&mut builder, 3),
            );
            builder.next(ifd0, Some(ifd1)).next(ifd1, Some(ifd2));
            builder.subifds(ifd1, &[sub]);
            let built = builder.build();
            let (mut src, tree) = walked(&built);

            assert_eq!(ids(&mut src, &tree), [0, 1, 3, 2]);
            assert_eq!(
                locations(&tree),
                [
                    Location::Ifd0,
                    Location::Chain(1),
                    Location::Sub {
                        parent: 1,
                        index: 0
                    },
                    Location::Chain(2),
                ]
            );
        }
    }

    #[test]
    fn the_order_is_depth_first_then_exif_then_the_chain() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = ifd(&mut builder, 0);
        let [a, b, c, exif, ifd1, d] = [1, 2, 3, 4, 5, 6].map(|id| ifd(&mut builder, id));
        builder
            .subifds(ifd0, &[a, b])
            .subifds(a, &[c])
            .exif(ifd0, exif);
        builder.next(ifd0, Some(ifd1)).subifds(ifd1, &[d]);
        let (mut src, tree) = walked(&builder.build());

        // IFD0, a, a's c, b, the Exif IFD, IFD1, IFD1's d.
        assert_eq!(ids(&mut src, &tree), [0, 1, 3, 2, 4, 5, 6]);
        let sub = |parent, index| Location::Sub { parent, index };
        assert_eq!(
            locations(&tree),
            [
                Location::Ifd0,
                sub(0, 0),
                sub(1, 0),
                sub(0, 1),
                Location::Exif,
                Location::Chain(1),
                sub(5, 0),
            ]
        );
    }

    #[test]
    fn an_ifd_that_points_to_itself_is_walked_once() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            builder.next(ifd0, Some(ifd0));
            let (_, tree) = walked(&builder.build());
            assert_eq!(locations(&tree), [Location::Ifd0]);

            // IFD0 → IFD1 → IFD0.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let ifd1 = ifd(&mut builder, 1);
            builder.next(ifd0, Some(ifd1)).next(ifd1, Some(ifd0));
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1]);
        }
    }

    #[test]
    fn a_subifd_cycle_is_walked_once() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (a, b) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder
                .subifds(ifd0, &[a])
                .subifds(a, &[b])
                .subifds(b, &[a, ifd0]);
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1, 2]);
        }
    }

    #[test]
    fn an_ifd_found_twice_is_kept_at_its_first_location() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let a = ifd(&mut builder, 1);
            builder
                .subifds(ifd0, &[a, a])
                .exif(ifd0, a)
                .next(ifd0, Some(a));
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1]);
            assert_eq!(
                tree.get(1).unwrap().0,
                Location::Sub {
                    parent: 0,
                    index: 0
                }
            );
            assert!(tree.exif().is_none(), "found first as a SubIFD");
        }
    }

    #[test]
    fn the_chain_goes_on_through_an_ifd_found_through_another_link() {
        for order in ORDERS {
            for link in [Link::Exif, Link::SubIfds] {
                // IFD0 → X → Y along the chain, where IFD0 also links to X
                // as its Exif IFD or as a SubIFD, which the walk follows
                // first.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let (x, y) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
                match link {
                    Link::Exif => builder.exif(ifd0, x),
                    _ => builder.subifds(ifd0, &[x]),
                };
                builder.next(ifd0, Some(x)).next(x, Some(y));
                let (mut src, tree) = walked(&builder.build());

                // X keeps its first location, and still takes step 1 of the
                // chain: Y is IFD2.
                let first = match link {
                    Link::Exif => Location::Exif,
                    _ => Location::Sub {
                        parent: 0,
                        index: 0,
                    },
                };
                assert_eq!(ids(&mut src, &tree), [0, 1, 2], "{link:?}");
                assert_eq!(
                    locations(&tree),
                    [Location::Ifd0, first, Location::Chain(2)],
                    "{link:?}"
                );
            }
        }
    }

    #[test]
    fn a_chain_through_ifds_found_before_still_ends_on_a_loop() {
        for order in ORDERS {
            // X, IFD0's Exif IFD, points to itself, and SubIFDs A and B
            // point to each other; the chain runs from IFD0 into X, then
            // into A.
            for start in [1, 2] {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let [x, a, b] = [1, 2, 3].map(|id| ifd(&mut builder, id));
                builder.exif(ifd0, x).subifds(ifd0, &[a, b]);
                builder.next(x, Some(x)).next(a, Some(b)).next(b, Some(a));
                builder.next(ifd0, Some(if start == 1 { x } else { a }));
                let (mut src, tree) = walked(&builder.build());
                assert_eq!(ids(&mut src, &tree), [0, 2, 3, 1], "from {start}");
            }

            // IFD0 → X → IFD0, X being IFD0's Exif IFD.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let x = ifd(&mut builder, 1);
            builder
                .exif(ifd0, x)
                .next(ifd0, Some(x))
                .next(x, Some(ifd0));
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1]);
        }
    }

    #[test]
    fn links_inside_the_header_are_not_tried() {
        for order in ORDERS {
            // A SubIFD at 0 and an Exif IFD at 4 in one file, then each kind
            // of link alone at offsets inside the header.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            builder.subifds_raw(ifd0, &[0]).long(ifd0, EXIF_IFD, &[4]);
            let mut files = vec![("SubIFD 0 and Exif 4".to_owned(), builder.build())];
            for link in [Link::SubIfds, Link::Exif, Link::Next] {
                for at in [0, 1, 4, 7] {
                    if matches!(link, Link::Next) && at == 0 {
                        // 0 is no next IFD, not a link.
                        continue;
                    }
                    let mut builder = TiffBuilder::new(order);
                    let ifd0 = ifd(&mut builder, 0);
                    match link {
                        Link::SubIfds => builder.subifds_raw(ifd0, &[at]),
                        Link::Exif => builder.long(ifd0, EXIF_IFD, &[at]),
                        Link::Next => builder.next_raw(ifd0, at),
                    };
                    files.push((format!("{link:?} at {at}"), builder.build()));
                }
            }
            for (what, built) in files {
                // The cap allows IFD0 alone: trying any link would cross it.
                let (_, tree) = try_walk(&built.bytes, &Limits::default().max_ifds(1))
                    .unwrap_or_else(|err| panic!("{what} ({order:?}): {err:?}"));
                assert_eq!(locations(&tree), [Location::Ifd0], "{what} ({order:?})");
            }
        }
    }

    /// A file whose IFD0 has one SubIFD, which has one, and so on: IFD `n`
    /// lies at depth `n` and has id `n`, down to depth `depth`; IFD0 also
    /// chains to an IFD1 (id 100) with SubIFDs as deep.
    fn nested(depth: u16) -> Built {
        let mut builder = TiffBuilder::new(Order::Big);
        let ifd0 = ifd(&mut builder, 0);
        let ifd1 = ifd(&mut builder, 100);
        builder.next(ifd0, Some(ifd1));
        for (root, first) in [(ifd0, 0), (ifd1, 100)] {
            let mut parent = root;
            for n in 1..=depth {
                let child = ifd(&mut builder, first + n);
                builder.subifds(parent, &[child]);
                parent = child;
            }
        }
        builder.build()
    }

    #[test]
    fn subifds_nest_down_to_the_depth_cap() {
        let built = nested(7);
        for (cap, expected) in [
            (5, vec![0, 1, 2, 3, 4, 5, 100, 101, 102, 103, 104, 105]),
            (1, vec![0, 1, 100, 101]),
            (0, vec![0, 100]),
            (
                7,
                vec![
                    0, 1, 2, 3, 4, 5, 6, 7, 100, 101, 102, 103, 104, 105, 106, 107,
                ],
            ),
        ] {
            let limits = Limits::default().max_ifd_depth(cap);
            let (mut src, tree) = try_walk(&built.bytes, &limits).unwrap();
            assert_eq!(ids(&mut src, &tree), expected, "cap {cap}");
        }
        let (mut src, tree) = walked(&built);
        assert_eq!(tree.len(), 12, "the default cap is 5");
        assert_eq!(ids(&mut src, &tree)[5], 5);
    }

    #[test]
    fn the_total_number_of_ifds_is_capped() {
        for order in ORDERS {
            // Ten IFDs: IFD0, four SubIFDs, the Exif IFD, IFD1, two SubIFDs
            // and IFD2.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let subs = [1, 2, 3, 4].map(|id| ifd(&mut builder, id));
            let exif = ifd(&mut builder, 5);
            let ifd1 = ifd(&mut builder, 6);
            let subs1 = [7, 8].map(|id| ifd(&mut builder, id));
            let ifd2 = ifd(&mut builder, 9);
            builder.subifds(ifd0, &subs).exif(ifd0, exif);
            builder.next(ifd0, Some(ifd1)).subifds(ifd1, &subs1);
            builder.next(ifd1, Some(ifd2));
            let built = builder.build();

            let (mut src, tree) = try_walk(&built.bytes, &Limits::default().max_ifds(10)).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
            assert_error(
                try_walk(&built.bytes, &Limits::default().max_ifds(9)),
                ErrorKind::LimitExceeded,
                built.ifd_offset(ifd2),
            );
            assert_error(
                try_walk(&built.bytes, &Limits::default().max_ifds(0)),
                ErrorKind::LimitExceeded,
                built.ifd_offset(ifd0),
            );
        }
    }

    #[test]
    fn attempts_count_towards_the_total_cap() {
        for order in ORDERS {
            // Three SubIFD pointers into bytes that are not IFDs: their
            // count, 0xFFFF, is over the entry cap.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            builder.blob_at(1024, &[0xFF; 64]);
            builder.subifds_raw(ifd0, &[1024, 1040, 1056]);
            let built = builder.build();

            let (_, tree) = try_walk(&built.bytes, &Limits::default().max_ifds(4)).unwrap();
            assert_eq!(locations(&tree), [Location::Ifd0]);
            assert_error(
                try_walk(&built.bytes, &Limits::default().max_ifds(3)),
                ErrorKind::LimitExceeded,
                1056,
            );
        }
    }

    #[test]
    fn a_subifd_pointer_into_pixel_data_is_not_followed() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            // The start of a JPEG, as the DSLR-A100's raw-data pointer might
            // find; read as an IFD, its count is far over the entry cap.
            builder.blob_at(
                1024,
                &[
                    0xFF, 0xD8, 0xFF, 0xE1, 0x12, 0x34, b'E', b'x', b'i', b'f', 0, 0,
                ],
            );
            // Bytes whose count, 2, is fine but whose entries are noise, with
            // field types no TIFF defines: the A100's own target reads so.
            #[rustfmt::skip]
            let noise = [
                0xE5, 0xC2, 0x52, 0xEC, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C,
                0, 0, 0, 0,
            ];
            builder.blob_at(2048, &[&order.u16(2)[..], &noise].concat());
            builder.subifds_raw(ifd0, &[1024, 2048]);
            let built = builder.build();
            let (_, tree) = walked(&built);
            assert_eq!(locations(&tree), [Location::Ifd0]);
        }
    }

    #[test]
    fn a_subifd_must_have_no_invalid_entries() {
        for order in ORDERS {
            // A child with an entry of unknown type, one whose value lies
            // past the end of the file, and a sound one.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (unknown, outside, sound) = (
                ifd(&mut builder, 1),
                ifd(&mut builder, 2),
                ifd(&mut builder, 3),
            );
            builder.raw(unknown, 0x0101, 99, 1, RawValue::Inline([0; 4]));
            builder.raw(outside, 0x0101, LONG, 2, RawValue::Offset(u32::MAX - 4));
            builder.short(sound, 0x0101, &[1]);
            builder.subifds(ifd0, &[unknown, outside, sound]);
            let built = builder.build();
            let (mut src, tree) = walked(&built);
            assert_eq!(ids(&mut src, &tree), [0, 3]);
            assert_eq!(
                tree.get(1).unwrap().0,
                Location::Sub {
                    parent: 0,
                    index: 2
                }
            );

            // The rule is for SubIFDs only: the same IFDs along the chain
            // and as the Exif IFD are kept.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (ifd1, exif) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder.raw(ifd1, 0x0101, 99, 1, RawValue::Inline([0; 4]));
            builder.raw(exif, 0x0101, 99, 1, RawValue::Inline([0; 4]));
            builder.next(ifd0, Some(ifd1)).exif(ifd0, exif);
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 2, 1]);
        }
    }

    #[test]
    fn a_rejected_subifd_does_not_feed_the_chain() {
        for order in ORDERS {
            // X has an entry of unknown type: as a SubIFD it is left out, and
            // a chain that reaches it ends there, not on a pointer read
            // inside something the walk judged not to be an IFD.
            let build = |as_subifd: bool| {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let (x, y) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
                builder.raw(x, 0x0101, 99, 1, RawValue::Inline([0; 4]));
                if as_subifd {
                    builder.subifds(ifd0, &[x]);
                }
                builder.next(ifd0, Some(x)).next(x, Some(y));
                builder.build()
            };

            let (mut src, tree) = walked(&build(true));
            assert_eq!(ids(&mut src, &tree), [0], "{order:?}");
            assert_eq!(locations(&tree), [Location::Ifd0], "{order:?}");

            // The control: chain IFDs may carry invalid entries.
            let (mut src, tree) = walked(&build(false));
            assert_eq!(ids(&mut src, &tree), [0, 1, 2], "{order:?}");
            assert_eq!(
                locations(&tree),
                [Location::Ifd0, Location::Chain(1), Location::Chain(2)],
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_broken_chain_ifd_ends_the_chain() {
        for order in ORDERS {
            for broken in [0xFFFF_0000, 1024] {
                // IFD0, with a SubIFD, → IFD1 → something that is not an IFD.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let (sub, ifd1) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
                builder.subifds(ifd0, &[sub]).next(ifd0, Some(ifd1));
                builder.next_raw(ifd1, broken);
                builder.blob_at(1024, &[0xFF; 32]);
                let (mut src, tree) = walked(&builder.build());
                assert_eq!(ids(&mut src, &tree), [0, 1, 2], "next {broken:#X}");
            }
        }
    }

    #[test]
    fn an_ifd0_that_cannot_be_read_fails_the_walk() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            ifd(&mut builder, 0);
            let len = u32::try_from(builder.build().bytes.len()).unwrap();
            for at in [0, 4, 7, len, len - 1, u32::MAX] {
                builder.ifd0_offset(Some(at));
                assert_error(
                    try_walk(&builder.build().bytes, &Limits::default()),
                    ErrorKind::Malformed,
                    u64::from(at),
                );
            }

            // An IFD0 with no entries, and one with more than the cap.
            let mut builder = TiffBuilder::new(order);
            builder.ifd();
            assert_error(
                try_walk(&builder.build().bytes, &Limits::default()),
                ErrorKind::Malformed,
                8,
            );
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            builder.short(ifd0, 0x0101, &[1]);
            assert_error(
                try_walk(
                    &builder.build().bytes,
                    &Limits::default().max_ifd_entries(1),
                ),
                ErrorKind::LimitExceeded,
                8,
            );
        }
    }

    #[test]
    fn an_exif_ifd_that_cannot_be_read_is_left_out() {
        for order in ORDERS {
            for exif in [0xFFFF_0000, 1024] {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let sub = ifd(&mut builder, 1);
                builder.subifds(ifd0, &[sub]).long(ifd0, EXIF_IFD, &[exif]);
                builder.blob_at(1024, &[0xFF; 32]);
                let (mut src, tree) = walked(&builder.build());
                assert_eq!(ids(&mut src, &tree), [0, 1], "exif {exif:#X}");
                assert!(tree.exif().is_none());
            }
        }
    }

    #[test]
    fn links_the_walk_does_not_follow() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let [sub, exif, ifd1] = [1, 2, 3].map(|id| ifd(&mut builder, id));
            let [after_sub, exif_sub, after_exif, exif_of_ifd1, exif_of_sub] =
                [10, 11, 12, 13, 14].map(|id| ifd(&mut builder, id));
            builder
                .subifds(ifd0, &[sub])
                .exif(ifd0, exif)
                .next(ifd0, Some(ifd1));
            // A SubIFD's next IFD; the Exif IFD's SubIFDs and next IFD; the
            // Exif IFDs of IFD1 and of a SubIFD.
            builder.next(sub, Some(after_sub));
            builder
                .subifds(exif, &[exif_sub])
                .next(exif, Some(after_exif));
            builder.exif(ifd1, exif_of_ifd1).exif(sub, exif_of_sub);
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3]);
        }
    }

    /// A link from IFD0 to another IFD.
    #[derive(Clone, Copy, Debug)]
    enum Link {
        SubIfds,
        Exif,
        Next,
    }

    /// The bytes of a little-endian file whose IFD0 links to an IFD that
    /// lies past the first 64 KiB, at [`FAR`], through `link`.
    fn far_link(link: Link) -> Vec<u8> {
        let order = Order::Little;
        let mut inner = TiffBuilder::new(order);
        let far = ifd(&mut inner, 1);
        let inner = inner.build();
        assert_eq!(inner.ifd_offset(far), FAR - FAR_BLOB);

        let mut builder = TiffBuilder::new(order);
        let ifd0 = ifd(&mut builder, 0);
        let far = u32::try_from(FAR).unwrap();
        match link {
            Link::SubIfds => builder.subifds_raw(ifd0, &[far]),
            Link::Exif => builder.long(ifd0, EXIF_IFD, &[far]),
            Link::Next => builder.next_raw(ifd0, far),
        };
        builder.blob_at(FAR_BLOB, &inner.bytes);
        builder.build().bytes
    }

    const FAR_BLOB: u64 = 100_000;
    const FAR: u64 = FAR_BLOB + 8;

    /// A reader that fails every read from 64 KiB on: past the first cached
    /// block, which holds IFD0, and before the far IFD.
    struct FailsFar(Cursor<Vec<u8>>);

    impl Read for FailsFar {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0.position() >= 64 * 1024 {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            self.0.read(buf)
        }
    }

    impl Seek for FailsFar {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.0.seek(pos)
        }
    }

    #[test]
    fn a_reader_failure_past_ifd0_fails_the_walk() {
        for link in [Link::SubIfds, Link::Exif, Link::Next] {
            let bytes = far_link(link);
            // The far IFD is there, for a reader that does not fail.
            let (mut src, tree) = try_walk(&bytes, &Limits::default()).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1], "{link:?}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let header = parse_header(&mut src).unwrap();
            let err = walk(&mut src, header, &Limits::default()).expect_err("the read fails");
            assert_eq!(err.kind(), ErrorKind::Io, "{link:?}: {err:?}");
        }
    }

    #[test]
    fn a_link_whose_value_is_too_long_to_read_is_not_followed() {
        // Over the 1 MiB that one tag value may take, and inside the file.
        let count = 256 * 1024 + 1;
        for tag in [SUB_IFDS, EXIF_IFD] {
            let mut builder = TiffBuilder::new(Order::Little);
            let ifd0 = ifd(&mut builder, 0);
            let ifd1 = ifd(&mut builder, 1);
            builder.raw(ifd0, tag, LONG, count, RawValue::Offset(1024));
            builder.next(ifd0, Some(ifd1));
            builder.pad_to(1024 + 4 * u64::from(count));
            let built = builder.build();
            let (mut src, tree) = walked(&built);
            assert_eq!(ids(&mut src, &tree), [0, 1], "tag {tag:#06X}");
            assert_eq!(tree.ifd0().invalid_entries(), 0);
        }
    }
}
