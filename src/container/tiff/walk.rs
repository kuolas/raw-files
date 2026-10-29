// SPDX-License-Identifier: MIT OR Apache-2.0

//! The IFD walk: finding every directory of a TIFF file that the format
//! modules may need.
//!
//! A TIFF-based RAW file holds several images, such as a thumbnail, one or
//! more JPEG previews and the raw sensor data, and each is described by its
//! own IFD. The IFDs reach each other through four kinds of link: the
//! next-IFD pointer at the end of an IFD, which chains IFD0 to IFD1, IFD1 to
//! IFD2, and so on; the SubIFDs tag (0x014A), an array of offsets of child
//! IFDs, which may have SubIFDs of their own; IFD0's Exif IFD tag (0x8769),
//! the offset of the IFD that holds the capture metadata; and the Exif
//! IFD's Interoperability IFD tag (0xA005), the offset of the IFD that
//! names the rule the file follows, and so whether it is in Adobe RGB.
//! [`walk`] follows these links from IFD0 and returns every IFD it finds as
//! a [`Tree`], each with the [`Location`] it was found at.
//!
//! The Interoperability IFD is tried last, after IFD0's whole chain. The
//! Nikon D70 writes one IFD that is both its IFD1, which holds the
//! thumbnail, and its Interoperability IFD. Tried last, the
//! Interoperability IFD tag finds that IFD already in the tree as IFD1,
//! where it stays, so the preview finder, which never asks the
//! Interoperability IFD, still finds the thumbnail there. The tree also
//! keeps where the tag led, and [`Tree::interop`] gives that IFD wherever
//! it lies, so an InteropIndex in an IFD shared with the chain is not lost.
//!
//! The walk reads a TIFF structure inside a [`Block`]: its base, where the
//! TIFF header lies and every link counts from, and its bounds, the range
//! of the file that the structure's IFDs and values must lie inside. For a
//! TIFF file's own structure the base is the start of the file and the
//! bounds are the whole file; for a TIFF structure inside a file, such as
//! the one in a JPEG file's Exif segment, the base is that header's
//! position and the bounds are the segment. IFD0 lies at the base plus the
//! offset the header gives, and every other IFD at the base plus the offset
//! its link gives. The walk keeps the links as written and works out each
//! IFD's position in the file from them; an IFD whose position would pass
//! the largest offset cannot be read, and one outside the bounds is
//! treated as one outside the file.
//!
//! The links are offsets read from the file, so a damaged or hostile file
//! can make them loop, nest without end or multiply. Three things keep the
//! walk bounded on any input:
//!
//! - a **visited set** of the positions in the file already tried, so no
//!   position is parsed twice: an IFD that points to itself, a cycle of
//!   SubIFDs, or two links to one IFD cost one parse and give one entry in
//!   the tree, at the location where the walk first found it; a chain that
//!   reaches it goes on from its next pointer, which the set keeps, and
//!   passes each IFD once, so a chain that loops ends. The set keeps a next
//!   pointer only for an IFD in the tree: a SubIFD left out (see below)
//!   leaves 0, like an IFD that cannot be parsed, so a chain that reaches it
//!   ends there;
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
//! tree it touches. An IFD that cannot be parsed, one outside the bounds
//! among them, ends the chain if it is a chain IFD, and is left out if it is
//! a SubIFD, the Exif IFD or the Interoperability IFD; a link whose tag
//! value cannot be read is not followed. No IFD can start inside the 8-byte
//! header, so a link into it, one whose offset is below 8, is not tried: it
//! is neither parsed nor counted against the total cap, and a chain that
//! reaches it ends; nor is a link whose position would pass the largest
//! offset. A SubIFD must also be wholly sound, with every entry of a known
//! type and inside the bounds and the file, or it is left out: the Sony
//! DSLR-A100 writes the offset of its raw data in the SubIFDs tag, and the
//! bytes there are not an IFD. Only a failure of the reader itself stops the
//! walk, as does the total cap.
//!
//! The walk follows nothing else: not the next-IFD pointers of SubIFDs or of
//! the Exif IFD, unless IFD0's chain itself reaches that IFD, nor that of
//! the Interoperability IFD; not the SubIFDs of the Exif IFD or of the
//! Interoperability IFD; not the Exif IFD tag of any IFD but IFD0; and not
//! the Interoperability IFD tag of any IFD but the Exif IFD. The walk runs
//! in a loop over an explicit stack, not by recursion, so a file cannot make
//! it use more call stack however it nests its IFDs.

use std::collections::BTreeMap;
use std::io::{Read, Seek};
use std::mem;
use std::vec;

use super::header::HEADER_LEN;
use super::tags::{EXIF_IFD, INTEROP_IFD, SUB_IFDS};
use super::{Block, Ifd, parse_header, parse_ifd};
use crate::error::{Error, ErrorKind, Result};
use crate::io::{ByteOrder, Source};
use crate::limits::Limits;

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
    /// The Exif IFD's Interoperability IFD, through its Interoperability
    /// IFD tag (0xA005), when the Exif IFD is in the tree at
    /// [`Exif`](Self::Exif). An IFD the walk found first through another
    /// link keeps that location, where [`Tree::interop`] finds it.
    Interop,
}

impl Location {
    /// Whether the IFD at this location never holds an image: the Exif IFD
    /// holds the capture metadata, and the Interoperability IFD names the
    /// rule the file follows. The preview finder and the search for the raw
    /// size skip both.
    ///
    /// An IFD found first at another location stays there, and is asked: the
    /// Nikon D70's IFD1, which is also its Interoperability IFD, lies at
    /// `Chain(1)` and holds the thumbnail.
    pub(crate) fn holds_no_image(self) -> bool {
        match self {
            Self::Exif | Self::Interop => true,
            Self::Ifd0 | Self::Chain(_) | Self::Sub { .. } => false,
        }
    }
}

/// The IFDs of a TIFF file that [`walk`] found, each with its
/// [`Location`].
///
/// The IFDs are in the order the walk found them, which is deterministic:
/// IFD0; then IFD0's SubIFDs, depth first, each followed by its own
/// SubIFDs, in the order of the SubIFDs arrays; then IFD0's Exif IFD; then
/// IFD1 and its SubIFDs in the same way, IFD2 and its SubIFDs, and so on
/// along the chain; and last the Exif IFD's Interoperability IFD. IFD0 is
/// always first, and an IFD appears once, at the first location it was
/// found at.
#[derive(Clone, Debug)]
pub(crate) struct Tree {
    /// Never empty: IFD0 is always the first.
    ifds: Vec<(Location, Ifd)>,
    /// The position in `ifds` of the IFD that the Exif IFD's
    /// Interoperability IFD tag leads to, wherever the walk placed it;
    /// `None` when the tag leads to no IFD of the tree.
    interop: Option<usize>,
}

// The NEF reader's claim reads IFD0, the preview finder goes through every
// IFD of the tree, the metadata and MakerNote readers read the Exif IFD,
// and the metadata reader reads the Interoperability IFD; nothing reads
// the number of IFDs or an IFD by its position yet. Each `expect` fails
// the build once its method has a caller, so it is then removed rather
// than left behind.
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
    /// already found at another location: unlike [`interop`](Self::interop),
    /// this gives only the IFD at [`Location::Exif`].
    pub(crate) fn exif(&self) -> Option<&Ifd> {
        self.iter()
            .find(|&(location, _)| location == Location::Exif)
            .map(|(_, ifd)| ifd)
    }

    /// The IFD that the Exif IFD's Interoperability IFD tag leads to,
    /// wherever the walk placed it: at [`Location::Interop`] when the walk
    /// found it first through that tag, and otherwise at the location where
    /// it found it before, as on the Nikon D70, whose Interoperability IFD
    /// is its IFD1, at `Chain(1)`. A tag that leads back to the Exif IFD
    /// itself, or to IFD0, gives that IFD too, as the file says.
    ///
    /// `None` when there is no Exif IFD at [`Location::Exif`], when it has
    /// no Interoperability IFD tag or one that cannot be read or followed,
    /// and when the IFD the tag leads to is not in the tree: one that
    /// cannot be parsed, or a SubIFD the walk left out.
    pub(crate) fn interop(&self) -> Option<&Ifd> {
        self.interop
            .and_then(|i| self.ifds.get(i))
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

/// Walks the IFDs of the TIFF structure in `src` that `block` holds: reads
/// its header at the block's base, then IFD0, its chain of next IFDs, the
/// SubIFDs of each of those, recursively, IFD0's Exif IFD and its
/// Interoperability IFD, counting every link from the base and reading
/// every IFD inside the bounds.
///
/// See the [module documentation](self) for what is followed, what is left
/// out and the order of the tree.
///
/// # Errors
///
/// Whatever [`parse_header`] returns if the header cannot be read, which
/// includes `UnknownFormat` when the block holds no TIFF header. Every other
/// error offset is a position in the file. `Malformed` if IFD0 lies inside
/// the header, at IFD0's position (the base plus IFD0's offset), or if that
/// position would pass the largest offset, at the base; whatever
/// [`parse_ifd`] returns for IFD0 if it cannot be read, which includes an
/// IFD0 outside the bounds; `LimitExceeded` if the walk would try to parse
/// more IFDs than [`Limits::max_ifds`] allows, at the position of the IFD
/// that would cross it; `Io` if the reader fails.
pub(crate) fn walk<R: Read + Seek>(
    src: &mut Source<R>,
    block: &Block,
    limits: &Limits,
) -> Result<Tree> {
    let header = parse_header(src, block)?;
    let Some(at) = block.at(header.ifd0) else {
        return Err(Error::malformed(
            "IFD0 lies past the largest offset",
            block.base(),
        ));
    };
    if u64::from(header.ifd0) < HEADER_LEN {
        return Err(Error::malformed("IFD0 lies inside the TIFF header", at));
    }
    let mut walker = Walker {
        src,
        block,
        order: header.order,
        limits,
        visited: BTreeMap::new(),
        attempts: 0,
        ifds: Vec::new(),
    };

    // IFD0 is the one IFD whose failure is the walk's.
    walker.claim(at)?;
    let (ifd0, mut next) = parse_ifd(walker.src, block, at, header.order, limits)?;
    walker.place(Location::Ifd0, ifd0)?;

    let exif = skip_damage(walker.ifds[0].1.u32(walker.src, EXIF_IFD))?.flatten();
    if let Some(at) = exif.and_then(|link| walker.target(link))
        && let Some((ifd, next)) = walker.candidate(at)?
    {
        walker.visited.insert(at, next);
        walker.ifds.push((Location::Exif, ifd));
    }

    let mut step: u32 = 1;
    while next != 0 {
        // A link into the header, or one whose position would pass the
        // largest offset, ends the chain.
        let Some(at) = walker.target(next) else {
            break;
        };
        if !walker.visited.contains_key(&at) {
            // A new IFD. One that cannot be parsed ends the chain.
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
        // Each step either claims a new position or takes a pointer recorded
        // when an earlier claim was parsed, so the total cap bounds the
        // steps to twice its value; saturating keeps the largest caps from
        // overflowing.
        step = step.saturating_add(1);
    }

    // The Interoperability IFD comes last, so that an IFD it shares with
    // the chain, as the D70's IFD1, keeps its place in the chain.
    let link = match walker
        .ifds
        .iter()
        .find(|(location, _)| *location == Location::Exif)
    {
        Some((_, exif)) => skip_damage(exif.u32(walker.src, INTEROP_IFD))?.flatten(),
        None => None,
    };
    let interop = match link.and_then(|link| walker.target(link)) {
        Some(at) => match walker.candidate(at)? {
            Some((ifd, _)) => {
                // Nothing is walked after it, so its next-IFD pointer is
                // not needed: `visited` keeps the 0 that `claim` recorded.
                walker.ifds.push((Location::Interop, ifd));
                Some(walker.ifds.len() - 1)
            }
            // Tried before, or not an IFD. One tried before and kept in the
            // tree is found by its position, where it lies once; one left
            // out, or that cannot be parsed, is not in the tree.
            None => walker.ifds.iter().position(|(_, ifd)| ifd.offset() == at),
        },
        None => None,
    };

    Ok(Tree {
        ifds: walker.ifds,
        interop,
    })
}

/// The state of one walk.
struct Walker<'a, R> {
    src: &'a mut Source<R>,
    /// The block the structure is read in: every link counts from its base.
    block: &'a Block,
    order: ByteOrder,
    limits: &'a Limits,
    /// The position in the file of every IFD the walk has tried to parse,
    /// whether or not it could, with the next-IFD pointer read there that
    /// the chain has yet to follow, as written: an offset from the base. The
    /// pointer is kept only for an IFD in the tree: 0 when the IFD could not
    /// be parsed, when it is a SubIFD the walk left out, when it is IFD0,
    /// whose pointer the chain starts from, when it is the Interoperability
    /// IFD, tried once the chain has ended, and once the chain has gone on
    /// from it.
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
    /// The children's offsets, as written, from the base, each with its
    /// position in the parent's SubIFDs array.
    offsets: std::iter::Zip<vec::IntoIter<u32>, std::ops::RangeFrom<u32>>,
}

impl<R: Read + Seek> Walker<'_, R> {
    /// The position in the file of the IFD a link past IFD0 leads to, given
    /// the link as written, an offset from the base: `None` if the link
    /// cannot be followed, because it lies inside the header or its position
    /// would pass the largest offset.
    ///
    /// No IFD can start inside the header, so a link there is not tried: it
    /// is neither parsed nor counted. (An IFD0 there is an error, which
    /// [`walk`] reports before it claims IFD0.)
    fn target(&self, link: u32) -> Option<u64> {
        if u64::from(link) < HEADER_LEN {
            return None;
        }
        self.block.at(link)
    }

    /// Marks the IFD at position `at` in the file as tried, with no
    /// next-IFD pointer recorded yet, and counts the attempt; `false`, with
    /// nothing counted, if it was tried before.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` at `at` if the walk has already tried as many IFDs as
    /// the total cap allows.
    fn claim(&mut self, at: u64) -> Result<bool> {
        if self.visited.contains_key(&at) {
            return Ok(false);
        }
        if self.attempts >= self.limits.max_ifds() {
            return Err(Error::limit("more IFDs than the limit allows", at));
        }
        self.attempts += 1;
        self.visited.insert(at, 0);
        Ok(true)
    }

    /// Parses the IFD at position `at` in the file, reached by a link past
    /// IFD0 and found by [`target`](Self::target): `None` if it was tried
    /// before or cannot be parsed. Returns the IFD and its next-IFD pointer
    /// as written, an offset from the base. It does not record the pointer:
    /// the caller does, in `visited`, only once it keeps the IFD in the tree,
    /// so that an IFD it rejects cannot feed a chain.
    ///
    /// # Errors
    ///
    /// As for [`claim`](Self::claim); `Io` if the reader fails.
    fn candidate(&mut self, at: u64) -> Result<Option<(Ifd, u32)>> {
        if !self.claim(at)? {
            return Ok(None);
        }
        skip_damage(parse_ifd(self.src, self.block, at, self.order, self.limits))
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
            if let Some(at) = self.target(offset)
                && let Some((child, next)) = self.candidate(at)?
                && child.invalid_entries() == 0
            {
                self.visited.insert(at, next);
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
        if depth >= self.limits.max_ifd_depth() {
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
/// cannot be read costs the walk only what it would have led to, the
/// preview finder skips an IFD whose tags it cannot read, and a NEF
/// MakerNote that does not hold together costs only its own preview. An
/// `Io` error is not damage: it says nothing about the file, and the
/// reader may fail again on the next read.
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
    use crate::testing::FailsFar;
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

    /// Walks the TIFF file `bytes`, its own structure, with `limits`.
    fn try_walk(bytes: &[u8], limits: &Limits) -> Result<(Source<Cursor<Vec<u8>>>, Tree)> {
        let len = u64::try_from(bytes.len()).unwrap();
        try_walk_in(bytes, &Block::file(len), limits)
    }

    /// Walks the TIFF structure that `block` holds in `bytes`, with
    /// `limits`.
    fn try_walk_in(
        bytes: &[u8],
        block: &Block,
        limits: &Limits,
    ) -> Result<(Source<Cursor<Vec<u8>>>, Tree)> {
        let mut src = open(bytes);
        let tree = walk(&mut src, block, limits)?;
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
            let len = u64::try_from(built.bytes.len()).unwrap();
            assert_eq!(tree.ifd0().block(), &Block::file(len));
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
        assert!(tree.interop().is_none());
    }

    #[test]
    fn an_exif_ifd_without_an_interoperability_ifd_has_none() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = ifd(&mut builder, 0);
        let exif = ifd(&mut builder, 1);
        builder.exif(ifd0, exif);
        let (_, tree) = walked(&builder.build());
        assert_eq!(locations(&tree), [Location::Ifd0, Location::Exif]);
        assert!(tree.exif().is_some());
        assert!(tree.interop().is_none());
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
                let (_, tree) = try_walk(&built.bytes, &Limits::default().with_max_ifds(1))
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
            let limits = Limits::default().with_max_ifd_depth(cap);
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

            let (mut src, tree) =
                try_walk(&built.bytes, &Limits::default().with_max_ifds(10)).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
            assert_error(
                try_walk(&built.bytes, &Limits::default().with_max_ifds(9)),
                ErrorKind::LimitExceeded,
                built.ifd_offset(ifd2),
            );
            assert_error(
                try_walk(&built.bytes, &Limits::default().with_max_ifds(0)),
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

            let (_, tree) = try_walk(&built.bytes, &Limits::default().with_max_ifds(4)).unwrap();
            assert_eq!(locations(&tree), [Location::Ifd0]);
            assert_error(
                try_walk(&built.bytes, &Limits::default().with_max_ifds(3)),
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
                    &Limits::default().with_max_ifd_entries(1),
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
            let [sub, exif, ifd1, interop] = [1, 2, 3, 4].map(|id| ifd(&mut builder, id));
            let [after_sub, exif_sub, after_exif, exif_of_ifd1, exif_of_sub] =
                [10, 11, 12, 13, 14].map(|id| ifd(&mut builder, id));
            let [after_interop, interop_sub, interop_of_interop] =
                [15, 16, 17].map(|id| ifd(&mut builder, id));
            let [interop_of_ifd0, interop_of_sub, interop_of_ifd1] =
                [18, 19, 20].map(|id| ifd(&mut builder, id));
            builder
                .subifds(ifd0, &[sub])
                .exif(ifd0, exif)
                .next(ifd0, Some(ifd1))
                .interop(exif, interop);
            // A SubIFD's next IFD; the Exif IFD's SubIFDs and next IFD; the
            // Exif IFDs of IFD1 and of a SubIFD.
            builder.next(sub, Some(after_sub));
            builder
                .subifds(exif, &[exif_sub])
                .next(exif, Some(after_exif));
            builder.exif(ifd1, exif_of_ifd1).exif(sub, exif_of_sub);
            // The Interoperability IFD's next IFD, SubIFDs and own
            // Interoperability IFD; the Interoperability IFDs of IFD0, of a
            // SubIFD and of IFD1.
            builder
                .next(interop, Some(after_interop))
                .subifds(interop, &[interop_sub])
                .interop(interop, interop_of_interop);
            builder
                .interop(ifd0, interop_of_ifd0)
                .interop(sub, interop_of_sub)
                .interop(ifd1, interop_of_ifd1);
            let (mut src, tree) = walked(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3, 4]);
        }
    }

    #[test]
    fn the_interoperability_ifd_comes_last() {
        for order in ORDERS {
            // IFD0 with a SubIFD a and its Exif IFD, whose Interoperability
            // IFD is I; IFD1 along the chain, with a SubIFD d.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let [interop, a, exif, ifd1, d] = [5, 1, 2, 3, 4].map(|id| ifd(&mut builder, id));
            builder.subifds(ifd0, &[a]).exif(ifd0, exif);
            builder.next(ifd0, Some(ifd1)).subifds(ifd1, &[d]);
            builder.interop(exif, interop);
            let built = builder.build();
            let (mut src, tree) = walked(&built);

            // I was written first after IFD0, and is still found last.
            assert!(built.ifd_offset(interop) < built.ifd_offset(a));
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3, 4, 5], "{order:?}");
            let sub = |parent, index| Location::Sub { parent, index };
            assert_eq!(
                locations(&tree),
                [
                    Location::Ifd0,
                    sub(0, 0),
                    Location::Exif,
                    Location::Chain(1),
                    sub(3, 0),
                    Location::Interop,
                ],
                "{order:?}"
            );
            assert_eq!(tree.exif().unwrap().offset(), built.ifd_offset(exif));
            let found = tree.interop().unwrap();
            assert_eq!(found.offset(), built.ifd_offset(interop));
            assert_eq!(found.u32(&mut src, ID).unwrap(), Some(5));
        }
    }

    #[test]
    fn an_interoperability_ifd_that_cannot_be_read_is_left_out() {
        for order in ORDERS {
            for link in [0xFFFF_0000, 1024] {
                // IFD0, with its Exif IFD, → IFD1; the Exif IFD's
                // Interoperability IFD tag points past the end of the file,
                // or at bytes that are not an IFD.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let (exif, ifd1) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
                builder.exif(ifd0, exif).next(ifd0, Some(ifd1));
                builder.long(exif, INTEROP_IFD, &[link]);
                builder.blob_at(1024, &[0xFF; 32]);
                let (mut src, tree) = walked(&builder.build());
                assert_eq!(ids(&mut src, &tree), [0, 1, 2], "{link:#X} ({order:?})");
                assert!(tree.interop().is_none());
            }
        }
    }

    #[test]
    fn an_interoperability_link_into_the_header_is_not_tried() {
        for order in ORDERS {
            for link in [0, 1, 4, 7] {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let exif = ifd(&mut builder, 1);
                builder.exif(ifd0, exif).long(exif, INTEROP_IFD, &[link]);
                // The cap allows IFD0 and the Exif IFD alone: trying the
                // link would cross it.
                let (_, tree) =
                    try_walk(&builder.build().bytes, &Limits::default().with_max_ifds(2))
                        .unwrap_or_else(|err| panic!("{link} ({order:?}): {err:?}"));
                assert_eq!(
                    locations(&tree),
                    [Location::Ifd0, Location::Exif],
                    "{link} ({order:?})"
                );
            }
        }
    }

    #[test]
    fn an_interoperability_ifd_outside_the_bounds_is_left_out() {
        for order in ORDERS {
            // I, the Exif IFD's Interoperability IFD, is the last IFD of the
            // structure and holds one entry; the file goes on past the
            // bounds, so that the bounds, not the file, are what leave I out.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (exif, interop) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder.exif(ifd0, exif).interop(exif, interop);
            let built = builder.build();
            let interop_at = built.ifd_offset(interop);
            let table_end = interop_at + 2 + 12;
            let len = u64::try_from(built.bytes.len()).unwrap();
            assert_eq!(len, table_end + 4, "I ends the structure");

            for (end, expected, what) in [
                (
                    table_end,
                    &[0, 1, 2][..],
                    "I's entries ending at the bounds",
                ),
                (table_end - 1, &[0, 1], "I's entries one byte past them"),
                (interop_at, &[0, 1], "I wholly past them"),
            ] {
                let (mut src, tree) = try_walk_to(&built, end)
                    .unwrap_or_else(|err| panic!("{what} ({order:?}): {err:?}"));
                assert_eq!(ids(&mut src, &tree), expected, "{what} ({order:?})");
            }
        }
    }

    #[test]
    fn an_interoperability_link_to_an_ifd_found_before_is_walked_once() {
        for order in ORDERS {
            // The Exif IFD's Interoperability IFD tag points to the Exif
            // IFD itself, then to IFD0: each stays where it was found, and
            // is the IFD the tag leads to.
            for to_exif in [true, false] {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let exif = ifd(&mut builder, 1);
                let to = if to_exif { exif } else { ifd0 };
                builder.exif(ifd0, exif).interop(exif, to);
                let built = builder.build();
                let (mut src, tree) = walked(&built);
                assert_eq!(ids(&mut src, &tree), [0, 1], "{to_exif} ({order:?})");
                assert_eq!(
                    locations(&tree),
                    [Location::Ifd0, Location::Exif],
                    "{to_exif} ({order:?})"
                );
                let found = tree.interop().expect("the IFD the tag leads to");
                assert_eq!(
                    found.offset(),
                    built.ifd_offset(to),
                    "{to_exif} ({order:?})"
                );
            }
        }
    }

    #[test]
    fn an_ifd_both_ifd1_and_the_interoperability_ifd_stays_ifd1() {
        for order in ORDERS {
            // The Nikon D70's layout: IFD0's next-IFD pointer and the Exif
            // IFD's Interoperability IFD tag both lead to IFD1, which holds
            // the thumbnail. Here IFD1 also leads on to IFD2. IFD1 stays in
            // the chain, once, and is still the Interoperability IFD.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let [exif, ifd1, ifd2] = [1, 2, 3].map(|id| ifd(&mut builder, id));
            builder.exif(ifd0, exif).interop(exif, ifd1);
            builder.next(ifd0, Some(ifd1)).next(ifd1, Some(ifd2));
            let built = builder.build();
            let (mut src, tree) = walked(&built);
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3], "{order:?}");
            assert_eq!(
                locations(&tree),
                [
                    Location::Ifd0,
                    Location::Exif,
                    Location::Chain(1),
                    Location::Chain(2)
                ],
                "{order:?}"
            );
            assert_eq!(tree.get(2).unwrap().1.offset(), built.ifd_offset(ifd1));
            let found = tree.interop().expect("IFD1, as the D70's");
            assert_eq!(found.offset(), built.ifd_offset(ifd1), "{order:?}");
            assert_eq!(found.u32(&mut src, ID).unwrap(), Some(2), "{order:?}");
        }
    }

    #[test]
    fn an_interoperability_link_to_a_subifd_or_a_chain_ifd_finds_it_there() {
        for order in ORDERS {
            // IFD0 with a SubIFD a and its Exif IFD; IFD1 with a SubIFD d;
            // IFD2. The Exif IFD's Interoperability IFD tag leads to a, IFD1,
            // d and IFD2 in turn, each found before the tag is followed.
            let sub = |parent, index| Location::Sub { parent, index };
            for (target, id) in [(0, 1), (1, 3), (2, 4), (3, 5)] {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let [a, exif, ifd1, d, ifd2] = [1, 2, 3, 4, 5].map(|id| ifd(&mut builder, id));
                builder.subifds(ifd0, &[a]).exif(ifd0, exif);
                builder.next(ifd0, Some(ifd1)).subifds(ifd1, &[d]);
                builder.next(ifd1, Some(ifd2));
                let to = [a, ifd1, d, ifd2][target];
                builder.interop(exif, to);
                let built = builder.build();
                let (mut src, tree) = walked(&built);

                // Each IFD once, at the location where it was found first.
                let what = format!("to {id} ({order:?})");
                assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3, 4, 5], "{what}");
                assert_eq!(
                    locations(&tree),
                    [
                        Location::Ifd0,
                        sub(0, 0),
                        Location::Exif,
                        Location::Chain(1),
                        sub(3, 0),
                        Location::Chain(2),
                    ],
                    "{what}"
                );
                let found = tree.interop().expect("the IFD the tag leads to");
                assert_eq!(found.offset(), built.ifd_offset(to), "{what}");
                assert_eq!(found.u32(&mut src, ID).unwrap(), Some(id), "{what}");
            }
        }
    }

    #[test]
    fn an_interoperability_link_to_a_position_left_out_of_the_tree_gives_none() {
        for order in ORDERS {
            // The Exif IFD's Interoperability IFD tag leads to a position
            // the walk tried before and left out of the tree: a SubIFD with
            // an entry of unknown type, and bytes that are not an IFD, tried
            // as a SubIFD and as IFD1.
            for case in [
                "a SubIFD left out",
                "not an IFD as a SubIFD",
                "not an IFD as IFD1",
            ] {
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let exif = ifd(&mut builder, 1);
                builder.exif(ifd0, exif);
                if case == "a SubIFD left out" {
                    let x = ifd(&mut builder, 2);
                    builder.raw(x, 0x0101, 99, 1, RawValue::Inline([0; 4]));
                    builder.subifds(ifd0, &[x]).interop(exif, x);
                } else {
                    builder.blob_at(1024, &[0xFF; 32]);
                    builder.long(exif, INTEROP_IFD, &[1024]);
                    if case == "not an IFD as a SubIFD" {
                        builder.subifds_raw(ifd0, &[1024]);
                    } else {
                        builder.next_raw(ifd0, 1024);
                    }
                }
                // The cap allows IFD0, the position and the Exif IFD: the
                // tag does not try the position again.
                let limits = Limits::default().with_max_ifds(3);
                let (_, tree) = try_walk(&builder.build().bytes, &limits)
                    .unwrap_or_else(|err| panic!("{case} ({order:?}): {err:?}"));
                assert_eq!(
                    locations(&tree),
                    [Location::Ifd0, Location::Exif],
                    "{case} ({order:?})"
                );
                assert!(tree.interop().is_none(), "{case} ({order:?})");
            }
        }
    }

    #[test]
    fn only_the_exif_and_interoperability_ifds_hold_no_image() {
        assert!(Location::Exif.holds_no_image());
        assert!(Location::Interop.holds_no_image());
        let sub = Location::Sub {
            parent: 0,
            index: 0,
        };
        for location in [Location::Ifd0, Location::Chain(1), sub] {
            assert!(!location.holds_no_image(), "{location:?}");
        }
    }

    #[test]
    fn the_interoperability_ifd_counts_towards_the_total_cap() {
        for order in ORDERS {
            // IFD0, its Exif IFD, IFD1, and the Exif IFD's Interoperability
            // IFD, tried last: four IFDs.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let [exif, ifd1, interop] = [1, 2, 3].map(|id| ifd(&mut builder, id));
            builder.exif(ifd0, exif).next(ifd0, Some(ifd1));
            builder.interop(exif, interop);
            let built = builder.build();
            let bytes = placed(&built);

            let (mut src, tree) =
                try_walk(&built.bytes, &Limits::default().with_max_ifds(4)).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3], "{order:?}");
            assert_error(
                try_walk(&built.bytes, &Limits::default().with_max_ifds(3)),
                ErrorKind::LimitExceeded,
                built.ifd_offset(interop),
            );

            // Inside a file, the error is at the IFD's position in it.
            let (mut src, tree) =
                try_walk_at(&bytes, POS, &Limits::default().with_max_ifds(4)).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1, 2, 3], "{order:?}");
            assert_error(
                try_walk_at(&bytes, POS, &Limits::default().with_max_ifds(3)),
                ErrorKind::LimitExceeded,
                POS + built.ifd_offset(interop),
            );
        }
    }

    #[test]
    fn an_interoperability_link_whose_value_is_too_long_to_read_is_not_followed() {
        // Over the 1 MiB that one tag value may take, and inside the file.
        let count = 256 * 1024 + 1;
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = ifd(&mut builder, 0);
        let (exif, ifd1) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
        builder.exif(ifd0, exif).next(ifd0, Some(ifd1));
        builder.raw(exif, INTEROP_IFD, LONG, count, RawValue::Offset(1024));
        builder.pad_to(1024 + 4 * u64::from(count));
        let (mut src, tree) = walked(&builder.build());
        assert_eq!(ids(&mut src, &tree), [0, 1, 2]);
        assert_eq!(tree.exif().unwrap().invalid_entries(), 0);
        assert!(tree.interop().is_none());
    }

    #[test]
    fn an_interoperability_link_inside_a_file_counts_from_its_header() {
        for order in ORDERS {
            // The Exif IFD's Interoperability IFD is I; counted from the
            // start of the file, the link lands on a decoy.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (exif, interop) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder.exif(ifd0, exif).interop(exif, interop);
            let built = builder.build();
            let mut bytes = placed(&built);
            decoy(&mut bytes, built.ifd_offset(interop), order);
            let (mut src, tree) = try_walk_at(&bytes, POS, &Limits::default()).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1, 2], "{order:?}");
            assert_eq!(
                tree.interop().unwrap().offset(),
                POS + built.ifd_offset(interop)
            );

            // The link is 500, where the file holds a decoy; counted from
            // the header, it lands past the end of the file.
            let far = 500;
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let exif = ifd(&mut builder, 1);
            builder.exif(ifd0, exif).long(exif, INTEROP_IFD, &[far]);
            let mut bytes = placed(&builder.build());
            assert!(POS + u64::from(far) > u64::try_from(bytes.len()).unwrap());
            decoy(&mut bytes, u64::from(far), order);
            let (mut src, tree) = try_walk_at(&bytes, POS, &Limits::default()).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1], "{order:?}");
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
    /// lies past the first 64 KiB, at [`FAR_IFD`], through `link`.
    fn far_link(link: Link) -> Vec<u8> {
        let order = Order::Little;
        let mut inner = TiffBuilder::new(order);
        let far = ifd(&mut inner, 1);
        let inner = inner.build();
        assert_eq!(inner.ifd_offset(far), FAR_IFD - FAR_BLOB);

        let mut builder = TiffBuilder::new(order);
        let ifd0 = ifd(&mut builder, 0);
        let far = u32::try_from(FAR_IFD).unwrap();
        match link {
            Link::SubIfds => builder.subifds_raw(ifd0, &[far]),
            Link::Exif => builder.long(ifd0, EXIF_IFD, &[far]),
            Link::Next => builder.next_raw(ifd0, far),
        };
        builder.blob_at(FAR_BLOB, &inner.bytes);
        builder.build().bytes
    }

    const FAR_BLOB: u64 = 100_000;
    /// Where the far IFD starts: past the first cached block, which holds
    /// IFD0, and so past the position from which a [`FailsFar`] fails.
    const FAR_IFD: u64 = FAR_BLOB + 8;
    const _: () = assert!(FAR_IFD > crate::testing::FAR);

    #[test]
    fn a_reader_failure_past_ifd0_fails_the_walk() {
        for link in [Link::SubIfds, Link::Exif, Link::Next] {
            let bytes = far_link(link);
            // The far IFD is there, for a reader that does not fail.
            let (mut src, tree) = try_walk(&bytes, &Limits::default()).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1], "{link:?}");

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let block = Block::file(src.len());
            let err = walk(&mut src, &block, &Limits::default()).expect_err("the read fails");
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

    /// Where the tests of a TIFF structure inside a larger file place its
    /// header: far enough in that a link counted from the start of the file
    /// lands somewhere else.
    const POS: u64 = 1000;

    /// The bytes of a file that holds `built` at [`POS`], after as many
    /// bytes of filler, as a JPEG file holds the TIFF structure of its Exif
    /// segment.
    fn placed(built: &Built) -> Vec<u8> {
        let mut bytes = vec![0; usize::try_from(POS).unwrap()];
        bytes.extend_from_slice(&built.bytes);
        bytes
    }

    /// The block of the TIFF structure at `pos` in `bytes`, with the whole
    /// file as bounds, so that only the base is at stake.
    fn at_pos(bytes: &[u8], pos: u64) -> Block {
        Block::new(pos, 0..u64::try_from(bytes.len()).unwrap())
    }

    /// Walks the TIFF structure at `pos` in `bytes`, with the whole file as
    /// bounds, with `limits`.
    fn try_walk_at(
        bytes: &[u8],
        pos: u64,
        limits: &Limits,
    ) -> Result<(Source<Cursor<Vec<u8>>>, Tree)> {
        try_walk_in(bytes, &at_pos(bytes, pos), limits)
    }

    /// Walks `built`, placed at [`POS`], with the default limits, which must
    /// succeed.
    fn walked_at(built: &Built) -> (Source<Cursor<Vec<u8>>>, Tree) {
        try_walk_at(&placed(built), POS, &Limits::default()).unwrap()
    }

    /// Writes into `bytes`, at `at`, an IFD that holds id 99 and nothing
    /// else: what a walk that counted a link from the wrong place would
    /// find.
    fn decoy(bytes: &mut [u8], at: u64, order: Order) {
        let mut builder = TiffBuilder::new(order);
        let decoy = ifd(&mut builder, 99);
        let built = builder.build();
        let block = built.ifd_block(decoy);
        let block = &built.bytes[usize::try_from(block.start).unwrap()..]
            [..usize::try_from(block.end - block.start).unwrap()];
        bytes[usize::try_from(at).unwrap()..][..block.len()].copy_from_slice(block);
    }

    #[test]
    fn a_tree_inside_a_file_counts_every_link_from_its_header() {
        for order in ORDERS {
            // IFD0 with SubIFDs a, which has its own SubIFD c, and b; IFD0's
            // Exif IFD; IFD1 along the chain, with a SubIFD d. c holds a
            // value stored out of line, whose offset counts from the header
            // too.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let [a, b, c, exif, ifd1, d] = [1, 2, 3, 4, 5, 6].map(|id| ifd(&mut builder, id));
            builder
                .subifds(ifd0, &[a, b])
                .subifds(a, &[c])
                .exif(ifd0, exif);
            builder.next(ifd0, Some(ifd1)).subifds(ifd1, &[d]);
            builder.long(c, 0x0101, &[0x0A0B_0C0D, 0x0102_0304]);
            let built = builder.build();

            let (mut src, at_zero) = walked(&built);
            let at_zero = (ids(&mut src, &at_zero), locations(&at_zero));
            let (mut src, tree) = walked_at(&built);
            assert_eq!(
                tree.ifd0().block(),
                &at_pos(&placed(&built), POS),
                "{order:?}"
            );
            assert_eq!(ids(&mut src, &tree), at_zero.0, "{order:?}");
            assert_eq!(ids(&mut src, &tree), [0, 1, 3, 2, 4, 5, 6], "{order:?}");
            assert_eq!(locations(&tree), at_zero.1, "{order:?}");
            for (i, id) in [ifd0, a, c, b, exif, ifd1, d].into_iter().enumerate() {
                let (_, found) = tree.get(i).unwrap();
                assert_eq!(found.offset(), POS + built.ifd_offset(id), "{i}");
            }
            let (_, c) = tree.get(2).unwrap();
            assert_eq!(
                c.u32s(&mut src, 0x0101).unwrap(),
                Some(vec![0x0A0B_0C0D, 0x0102_0304]),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_link_inside_a_file_counts_from_its_header_not_from_the_start() {
        for order in ORDERS {
            for link in [Link::SubIfds, Link::Exif, Link::Next] {
                // IFD0 links to X; counted from the start of the file, the
                // link lands on a decoy.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let x = ifd(&mut builder, 1);
                match link {
                    Link::SubIfds => builder.subifds(ifd0, &[x]),
                    Link::Exif => builder.exif(ifd0, x),
                    Link::Next => builder.next(ifd0, Some(x)),
                };
                let built = builder.build();
                let mut bytes = placed(&built);
                decoy(&mut bytes, built.ifd_offset(x), order);
                let (mut src, tree) = try_walk_at(&bytes, POS, &Limits::default()).unwrap();
                assert_eq!(ids(&mut src, &tree), [0, 1], "{link:?} ({order:?})");

                // IFD0 links to 500, where the file holds a decoy; counted
                // from the header, the link lands past the end of the file.
                let far = 500;
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                match link {
                    Link::SubIfds => builder.subifds_raw(ifd0, &[far]),
                    Link::Exif => builder.long(ifd0, EXIF_IFD, &[far]),
                    Link::Next => builder.next_raw(ifd0, far),
                };
                let mut bytes = placed(&builder.build());
                assert!(POS + u64::from(far) > u64::try_from(bytes.len()).unwrap());
                decoy(&mut bytes, u64::from(far), order);
                let (mut src, tree) = try_walk_at(&bytes, POS, &Limits::default()).unwrap();
                assert_eq!(ids(&mut src, &tree), [0], "{link:?} ({order:?})");
            }
        }
    }

    #[test]
    fn loops_inside_a_file_are_walked_once() {
        for order in ORDERS {
            // An IFD that points to itself.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            builder.next(ifd0, Some(ifd0));
            let (_, tree) = walked_at(&builder.build());
            assert_eq!(locations(&tree), [Location::Ifd0], "{order:?}");

            // IFD0 → IFD1 → IFD0.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let ifd1 = ifd(&mut builder, 1);
            builder.next(ifd0, Some(ifd1)).next(ifd1, Some(ifd0));
            let (mut src, tree) = walked_at(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1], "{order:?}");

            // A cycle of SubIFDs.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (a, b) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder
                .subifds(ifd0, &[a])
                .subifds(a, &[b])
                .subifds(b, &[a, ifd0]);
            let (mut src, tree) = walked_at(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1, 2], "{order:?}");

            // IFD0 → X → Y, X being IFD0's Exif IFD: the chain goes on from
            // the pointer the walk kept for X.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (x, y) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder.exif(ifd0, x).next(ifd0, Some(x)).next(x, Some(y));
            let (mut src, tree) = walked_at(&builder.build());
            assert_eq!(ids(&mut src, &tree), [0, 1, 2], "{order:?}");
            assert_eq!(
                locations(&tree),
                [Location::Ifd0, Location::Exif, Location::Chain(2)],
                "{order:?}"
            );
        }
    }

    #[test]
    fn links_into_a_header_inside_a_file_are_not_tried() {
        for order in ORDERS {
            for link in [Link::SubIfds, Link::Exif, Link::Next] {
                for at in 0..8 {
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
                    // The cap allows IFD0 alone: trying any link would
                    // cross it.
                    let bytes = placed(&builder.build());
                    let (_, tree) = try_walk_at(&bytes, POS, &Limits::default().with_max_ifds(1))
                        .unwrap_or_else(|err| panic!("{link:?} at {at} ({order:?}): {err:?}"));
                    assert_eq!(
                        locations(&tree),
                        [Location::Ifd0],
                        "{link:?} at {at} ({order:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn an_ifd0_inside_a_header_inside_a_file_fails_at_its_position() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            ifd(&mut builder, 0);
            for at in [0, 4, 7] {
                builder.ifd0_offset(Some(at));
                assert_error(
                    try_walk_at(&placed(&builder.build()), POS, &Limits::default()),
                    ErrorKind::Malformed,
                    POS + u64::from(at),
                );
            }
        }
    }

    #[test]
    fn the_total_cap_inside_a_file_fails_at_the_position_of_the_ifd_that_crosses_it() {
        for order in ORDERS {
            // IFD0, a SubIFD and IFD1.
            let mut builder = TiffBuilder::new(order);
            let ifd0 = ifd(&mut builder, 0);
            let (sub, ifd1) = (ifd(&mut builder, 1), ifd(&mut builder, 2));
            builder.subifds(ifd0, &[sub]).next(ifd0, Some(ifd1));
            let built = builder.build();
            let bytes = placed(&built);

            let (mut src, tree) =
                try_walk_at(&bytes, POS, &Limits::default().with_max_ifds(3)).unwrap();
            assert_eq!(ids(&mut src, &tree), [0, 1, 2]);
            for (cap, crosses) in [(2, ifd1), (1, sub), (0, ifd0)] {
                assert_error(
                    try_walk_at(&bytes, POS, &Limits::default().with_max_ifds(cap)),
                    ErrorKind::LimitExceeded,
                    POS + built.ifd_offset(crosses),
                );
            }
        }
    }

    /// Where [`AtTheEnd`] holds its bytes: so near the largest offset that
    /// an offset of 2^31 or more from there would pass it.
    const NEAR_THE_END: u64 = u64::MAX - 1023;

    /// A reader as long as the largest offset allows, which holds its bytes
    /// at [`NEAR_THE_END`] and zeros everywhere else.
    struct AtTheEnd {
        bytes: Vec<u8>,
        pos: u64,
    }

    impl Read for AtTheEnd {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            for (byte, pos) in buf.iter_mut().zip(self.pos..) {
                *byte = pos
                    .checked_sub(NEAR_THE_END)
                    .and_then(|i| self.bytes.get(usize::try_from(i).ok()?))
                    .copied()
                    .unwrap_or(0);
            }
            let n = u64::try_from(buf.len()).unwrap();
            self.pos = self.pos.saturating_add(n);
            Ok(buf.len())
        }
    }

    impl Seek for AtTheEnd {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.pos = match pos {
                SeekFrom::Start(pos) => pos,
                SeekFrom::End(0) => u64::MAX,
                _ => return Err(io::Error::from(io::ErrorKind::Unsupported)),
            };
            Ok(self.pos)
        }
    }

    /// Walks the TIFF structure at [`NEAR_THE_END`] in an [`AtTheEnd`]
    /// reader that holds `bytes`, with `limits`; and the reader.
    fn try_walk_at_the_end(bytes: Vec<u8>, limits: &Limits) -> Result<(Source<AtTheEnd>, Tree)> {
        let mut src = Source::new(AtTheEnd { bytes, pos: 0 }).unwrap();
        let block = Block::new(NEAR_THE_END, 0..src.len());
        let tree = walk(&mut src, &block, limits)?;
        Ok((src, tree))
    }

    #[test]
    fn links_whose_position_would_pass_the_largest_offset_are_not_tried() {
        for order in ORDERS {
            for link in [Link::SubIfds, Link::Exif, Link::Next] {
                for at in [1 << 31, u32::MAX] {
                    let mut builder = TiffBuilder::new(order);
                    let ifd0 = ifd(&mut builder, 0);
                    match link {
                        Link::SubIfds => builder.subifds_raw(ifd0, &[at]),
                        Link::Exif => builder.long(ifd0, EXIF_IFD, &[at]),
                        Link::Next => builder.next_raw(ifd0, at),
                    };
                    // The cap allows IFD0 alone: trying any link would
                    // cross it.
                    let limits = Limits::default().with_max_ifds(1);
                    let (mut src, tree) = try_walk_at_the_end(builder.build().bytes, &limits)
                        .unwrap_or_else(|err| panic!("{link:?} at {at} ({order:?}): {err:?}"));
                    assert_eq!(tree.ifd0().block().base(), NEAR_THE_END);
                    assert_eq!(ids(&mut src, &tree), [0], "{link:?} at {at} ({order:?})");
                }
            }
        }
    }

    #[test]
    fn an_ifd0_past_the_largest_offset_fails_at_the_base() {
        for order in ORDERS {
            let mut builder = TiffBuilder::new(order);
            ifd(&mut builder, 0);
            // The structure itself is sound where it is.
            try_walk_at_the_end(builder.build().bytes, &Limits::default()).unwrap();
            builder.ifd0_offset(Some(u32::MAX));
            assert_error(
                try_walk_at_the_end(builder.build().bytes, &Limits::default()),
                ErrorKind::Malformed,
                NEAR_THE_END,
            );
        }
    }

    /// Walks `built`, placed at [`POS`] in a file that goes on 100 bytes
    /// past it, with bounds from its header to `end`, an offset from the
    /// header, and the default limits.
    fn try_walk_to(built: &Built, end: u64) -> Result<(Source<Cursor<Vec<u8>>>, Tree)> {
        let mut bytes = placed(built);
        bytes.resize(bytes.len() + 100, 0);
        try_walk_in(&bytes, &Block::new(POS, POS..POS + end), &Limits::default())
    }

    #[test]
    fn an_ifd_outside_the_bounds_is_left_out() {
        for order in ORDERS {
            for link in [Link::SubIfds, Link::Exif, Link::Next] {
                // IFD0 links to X, the last IFD of the structure, which holds
                // one entry; the file goes on past the bounds, so that the
                // bounds, not the file, are what leave X out.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let x = ifd(&mut builder, 1);
                match link {
                    Link::SubIfds => builder.subifds(ifd0, &[x]),
                    Link::Exif => builder.exif(ifd0, x),
                    Link::Next => builder.next(ifd0, Some(x)),
                };
                let built = builder.build();
                let x_at = built.ifd_offset(x);
                let table_end = x_at + 2 + 12;
                let len = u64::try_from(built.bytes.len()).unwrap();
                assert_eq!(len, table_end + 4, "X ends the structure");

                for (end, expected, what) in [
                    (len, &[0, 1][..], "the whole structure"),
                    (table_end, &[0, 1], "X's entries ending at the bounds"),
                    (table_end - 1, &[0], "X's entries one byte past them"),
                    (x_at, &[0], "X wholly past them"),
                ] {
                    let (mut src, tree) = try_walk_to(&built, end)
                        .unwrap_or_else(|err| panic!("{what}, {link:?} ({order:?}): {err:?}"));
                    assert_eq!(
                        ids(&mut src, &tree),
                        expected,
                        "{what}, {link:?} ({order:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn an_ifd0_outside_the_bounds_fails_the_walk() {
        for order in ORDERS {
            // IFD0 at 8 from the header, with one entry.
            let mut builder = TiffBuilder::new(order);
            ifd(&mut builder, 0);
            let built = builder.build();
            let table_end = 8 + 2 + 12;
            let (_, tree) = try_walk_to(&built, table_end).unwrap();
            assert_eq!(locations(&tree), [Location::Ifd0], "{order:?}");
            // Its entries one byte past the bounds' end, its count past
            // them, and IFD0 wholly past them.
            for end in [table_end - 1, 9, 8] {
                assert_error(try_walk_to(&built, end), ErrorKind::Malformed, POS + 8);
            }
            // The header itself one byte past them: not a TIFF structure.
            let Err(err) = try_walk_to(&built, 7) else {
                panic!("no room for the header ({order:?})");
            };
            assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{err:?}");
        }
    }

    #[test]
    fn a_value_outside_the_bounds_leaves_out_a_subifd_but_not_a_chain_or_exif_ifd() {
        for order in ORDERS {
            for link in [Link::SubIfds, Link::Exif, Link::Next] {
                // IFD0 links to X, the last IFD of the structure, which holds
                // a value stored out of line, the last bytes of the
                // structure.
                let mut builder = TiffBuilder::new(order);
                let ifd0 = ifd(&mut builder, 0);
                let x = ifd(&mut builder, 1);
                builder.long(x, 0x0101, &[1, 2]);
                match link {
                    Link::SubIfds => builder.subifds(ifd0, &[x]),
                    Link::Exif => builder.exif(ifd0, x),
                    Link::Next => builder.next(ifd0, Some(x)),
                };
                let built = builder.build();
                let len = u64::try_from(built.bytes.len()).unwrap();
                assert_eq!(built.ifd_block(x).end, len, "X's value ends the structure");

                // The value ending exactly at the bounds' end: kept.
                let (mut src, tree) = try_walk_to(&built, len).unwrap();
                assert_eq!(ids(&mut src, &tree), [0, 1], "{link:?} ({order:?})");
                assert_eq!(tree.get(1).unwrap().1.invalid_entries(), 0);

                // One byte past it: X's entry is invalid, which leaves out
                // a SubIFD only.
                let (mut src, tree) = try_walk_to(&built, len - 1).unwrap();
                if matches!(link, Link::SubIfds) {
                    assert_eq!(ids(&mut src, &tree), [0], "{order:?}");
                } else {
                    assert_eq!(ids(&mut src, &tree), [0, 1], "{link:?} ({order:?})");
                    let (_, found) = tree.get(1).unwrap();
                    assert_eq!(found.invalid_entries(), 1, "{link:?} ({order:?})");
                    assert_eq!(found.entry(0x0101), None, "{link:?} ({order:?})");
                }
            }
        }
    }
}
