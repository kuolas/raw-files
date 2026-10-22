// SPDX-License-Identifier: MIT OR Apache-2.0

//! A builder for small JPEG files, for tests.
//!
//! A JPEG file, as a camera writes it, is one JPEG stream: a picture, with
//! the camera's metadata in application segments before the picture's
//! frame header, and sometimes more images after the end of the first.
//! Tests describe the segments they need, and the builder writes the bytes
//! and says where the parts a reader looks for landed. It writes what it is
//! told, in the order it is told, so a test can also write a file no camera
//! would: two Exif segments, fill bytes or a stand-alone marker anywhere, a
//! segment with any marker, bytes that are no segment at all.
//!
//! The layout, in order:
//!
//! - SOI, the start of the image: `FF D8`;
//! - the parts added with these methods, in the order they were added:
//!   - [`JpegBuilder::jfif`]: an APP0 segment, `FF E0`, holding the
//!     smallest JFIF header: `JFIF` and a zero byte, version 1.02, no
//!     units, a density of 1 by 1, and no thumbnail;
//!   - [`JpegBuilder::exif`]: an APP1 segment, `FF E1`, holding `Exif`, two
//!     zero bytes, and then the given TIFF structure, whose first byte is
//!     its TIFF header;
//!   - [`JpegBuilder::xmp`]: an APP1 segment holding the XMP namespace,
//!     `http://ns.adobe.com/xap/1.0/` and a zero byte, then the given text;
//!   - [`JpegBuilder::app2`]: an APP2 segment, `FF E2`, holding the given
//!     payload as it is;
//!   - [`JpegBuilder::mpf`]: an APP2 segment holding the MPF index of the
//!     file's images, described [below](#the-mpf-index);
//!   - [`JpegBuilder::segment`]: a segment with any marker and payload;
//!   - [`JpegBuilder::fill`]: `FF` fill bytes, which the next marker
//!     follows;
//!   - [`JpegBuilder::marker`]: a marker alone, `FF` and a code, with no
//!     length;
//!   - [`JpegBuilder::raw`]: any bytes, as they are;
//! - the main image: the stub JPEG of `tests/support/jpeg_stub.rs` without its
//!   SOI, that is its two tables, its frame header with the size and marker
//!   set by [`JpegBuilder::frame`] (640 by 480 and baseline, `C0`, by
//!   default), its SOS segment, 4 bytes of data and EOI, `FF D9`;
//! - the blocks added with [`JpegBuilder::after_eoi`], such as other
//!   images, and the images added with [`JpegBuilder::mpf_image`], each
//!   after the bytes that come before it, in the order they were added;
//! - the zeros added with [`JpegBuilder::pad`], as some cameras pad their
//!   files after the last image.
//!
//! A segment is written by the stub's `segment`: `FF`, the marker, the
//! 2-byte length, which counts itself and the payload, then the payload.
//! Every multi-byte number of the JPEG structure is big-endian; the TIFF
//! structure inside an Exif segment is written as the test gives it.
//!
//! # The MPF index
//!
//! A file in the Multi-Picture Format lists, in an APP2 segment, every
//! image it holds: the main image, then the images after its EOI, such as
//! the large thumbnails a camera writes for screens. The payload of the
//! segment [`JpegBuilder::mpf`] adds is, in order, every number in the byte
//! order it is given:
//!
//! - [`MPF_ID`], `MPF` and a zero byte;
//! - the MP header, at [`Built::mpf_header`], in the shape of a TIFF header:
//!   the byte-order mark, 42, and the offset of the MP Index IFD, 8. Every
//!   offset in the segment counts from the MP header's first byte;
//! - the MP Index IFD: 3 entries, MPFVersion ([`MPF_VERSION`], UNDEFINED,
//!   the 4 bytes `0100`, in the entry), NumberOfImages
//!   ([`NUMBER_OF_IMAGES`], one LONG, in the entry) and MPEntry
//!   ([`MP_ENTRY`], UNDEFINED, 16 bytes per image, after the IFD at offset
//!   [`MP_ENTRY_AT`]), then a next-IFD pointer of 0;
//! - MPEntry's value: one entry of 16 bytes per image, the main image
//!   first, then each image added with [`JpegBuilder::mpf_image`] in the
//!   order it was added. An entry is the image's attribute (4 bytes), its
//!   size (4), from its SOI to the end of its EOI, its offset (4), where its
//!   SOI lies counted from the MP header, and the entry numbers of two
//!   images that depend on it (2 and 2), both 0. The main image's entry
//!   has offset 0, a size that ends after its EOI, and the attribute
//!   `A0030000`: a parent of dependent images, the representative image,
//!   and of the type of the baseline primary image. Every other entry's
//!   attribute is that of a dependent child, of the type of a large
//!   thumbnail of class 1 (`40010001`) when the image fits in 640 by 480,
//!   and of class 2 (`40010002`) otherwise.
//!
//! Every entry has a fixed 16 bytes, so the segment's length is known
//! before the images are placed, and the builder fills in the offsets once
//! it has.
//!
//! The index can be damaged on purpose: [`JpegBuilder::mpf_header`] writes
//! other bytes in place of the MP header, [`JpegBuilder::mpf_number_of_images`]
//! another NumberOfImages, [`JpegBuilder::mpf_entry_offset`] and
//! [`JpegBuilder::mpf_entry_size`] another offset or size in one entry,
//! [`JpegBuilder::mpf_entry_len`] an MPEntry value of another length, cut
//! or extended with zeros, [`JpegBuilder::mpf_entry_type`] another field
//! type and [`JpegBuilder::mpf_entry_tag`] another tag in MPEntry's place.
//!
//! [`Built`] holds the bytes and where things landed: each Exif segment's
//! TIFF structure, each APP2 segment's payload, the main image's frame
//! header and end, each block after its EOI, the MP header and each image
//! the MPF index lists after the main one.
//!
//! The stub and `segment` come from `tests/support/jpeg_stub.rs`, and the
//! byte order of the MP header, [`Order`], from
//! `tests/support/tiff_builder.rs`, this module's siblings under one name in
//! both compilations: in `tests/support/` for the integration tests, and at
//! the root of the crate for the unit tests.
//!
//! This file uses only `std`, because it is compiled twice: into the
//! library's unit tests and, through `tests/support/`, into the integration
//! tests.

use std::ops::Range;

use super::jpeg_stub::{FRAME_HEADER_AT, jpeg_stub, segment};
use super::tiff_builder::Order;

/// The marker of APP0, the JFIF segment.
pub const APP0: u8 = 0xE0;
/// The marker of APP1, the Exif and XMP segments.
pub const APP1: u8 = 0xE1;
/// The marker of APP2, the MPF segment among others.
pub const APP2: u8 = 0xE2;

/// What a JFIF segment holds after its marker and length: `JFIF` and a
/// zero byte, version 1.02, units 0 (none), a density of 1 by 1, and a
/// thumbnail of 0 by 0 pixels.
pub const JFIF: [u8; 14] = *b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0";

/// What an Exif segment's payload starts with: `Exif` and two zero bytes.
pub const EXIF_ID: [u8; 6] = *b"Exif\0\0";

/// What an XMP segment's payload starts with: its namespace and a zero
/// byte.
pub const XMP_ID: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

/// What an MPF segment's payload starts with: `MPF` and a zero byte.
pub const MPF_ID: [u8; 4] = *b"MPF\0";

/// The MP tag of the version of the Multi-Picture Format (MPFVersion):
/// the 4 bytes `0100`.
pub const MPF_VERSION: u16 = 0xB000;

/// The MP tag of the number of images the file holds (NumberOfImages): one
/// LONG.
pub const NUMBER_OF_IMAGES: u16 = 0xB001;

/// The MP tag of the list of the file's images (MPEntry): 16 bytes per
/// image.
pub const MP_ENTRY: u16 = 0xB002;

/// The TIFF field type UNDEFINED, MPFVersion's and MPEntry's.
pub const UNDEFINED: u16 = 7;

/// The TIFF field type LONG, NumberOfImages'.
pub const LONG: u16 = 4;

/// Where the MP Index IFD lies, counted from the MP header: right after the
/// header's 8 bytes.
pub const MP_INDEX_AT: u32 = 8;

/// Where MPEntry's value lies, counted from the MP header: after the MP
/// Index IFD, its count, 3 entries of 12 bytes and its next-IFD pointer.
pub const MP_ENTRY_AT: u32 = MP_INDEX_AT + 2 + 3 * 12 + 4;

/// The length of one MP Entry.
pub const MP_ENTRY_LEN: usize = 16;

/// Which position of a segment the builder reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Report {
    /// None.
    Nothing,
    /// Where its TIFF structure lies, after the Exif identifier.
    Exif,
    /// Where its payload lies.
    App2,
}

/// One part added before the main image.
#[derive(Clone, Debug)]
enum Part {
    /// A segment: its marker, its payload, and what to report of it.
    Segment(u8, Vec<u8>, Report),
    /// Bytes written as they are.
    Bytes(Vec<u8>),
    /// The MPF segment, whose payload is written once the images are
    /// placed.
    Mpf,
}

/// One part added after the main image's EOI.
#[derive(Clone, Debug)]
enum After {
    /// Bytes written as they are.
    Bytes(Vec<u8>),
    /// An image the MPF index lists.
    Image(Image),
}

/// An image added with [`JpegBuilder::mpf_image`].
#[derive(Clone, Debug)]
struct Image {
    /// The bytes written right before the image.
    before: Vec<u8>,
    /// The multiple of which the image's position in the file is.
    align: usize,
    width: u16,
    height: u16,
}

/// The MPF index, as [`JpegBuilder::mpf`] and the methods that damage it
/// set it.
#[derive(Clone, Debug)]
struct Mpf {
    order: Order,
    header: Option<[u8; 8]>,
    number_of_images: Option<u32>,
    entry_tag: u16,
    entry_type: u16,
    entry_len: Option<usize>,
    /// The entries whose offset is set, by index.
    offsets: Vec<(usize, u32)>,
    /// The entries whose size is set, by index.
    sizes: Vec<(usize, u32)>,
}

/// Builds a JPEG file; see the [module documentation](self) for the
/// layout.
///
/// The methods that add to the file return `&mut Self`, so calls can be
/// chained. A method that writes a segment panics on a payload longer than
/// a segment can hold, 65,533 bytes, as `segment` does: a mistake in the
/// test.
#[derive(Clone, Debug)]
pub struct JpegBuilder {
    parts: Vec<Part>,
    width: u16,
    height: u16,
    marker: u8,
    after_eoi: Vec<After>,
    pad: usize,
    mpf: Option<Mpf>,
}

impl Default for JpegBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JpegBuilder {
    /// A file of SOI and the main image, a baseline frame of 640 by 480.
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            width: 640,
            height: 480,
            marker: 0xC0,
            after_eoi: Vec::new(),
            pad: 0,
            mpf: None,
        }
    }

    /// Gives the main image's frame header marker `marker` and the size
    /// `width` by `height`, written as given: a zero side, or a marker that
    /// is no frame marker, included.
    pub fn frame(&mut self, width: u16, height: u16, marker: u8) -> &mut Self {
        self.width = width;
        self.height = height;
        self.marker = marker;
        self
    }

    /// Adds an APP0 segment holding the smallest JFIF header, [`JFIF`].
    pub fn jfif(&mut self) -> &mut Self {
        self.add(APP0, JFIF.to_vec(), Report::Nothing)
    }

    /// Adds an APP1 segment holding [`EXIF_ID`] and then `tiff`, a TIFF
    /// structure; [`Built::exif_tiff`] says where `tiff` landed.
    pub fn exif(&mut self, tiff: &[u8]) -> &mut Self {
        let mut payload = EXIF_ID.to_vec();
        payload.extend_from_slice(tiff);
        self.add(APP1, payload, Report::Exif)
    }

    /// Adds an APP1 segment holding [`XMP_ID`] and then `text`.
    pub fn xmp(&mut self, text: &str) -> &mut Self {
        let mut payload = XMP_ID.to_vec();
        payload.extend_from_slice(text.as_bytes());
        self.add(APP1, payload, Report::Nothing)
    }

    /// Adds an APP2 segment holding `payload` as it is;
    /// [`Built::app2_payload`] says where it landed.
    pub fn app2(&mut self, payload: &[u8]) -> &mut Self {
        self.add(APP2, payload.to_vec(), Report::App2)
    }

    /// Adds the MPF segment, an APP2 segment holding the MPF index of the
    /// file's images, with the MP header and its numbers in byte order
    /// `order`; see [the MPF index](self#the-mpf-index). It lists the main
    /// image and every image added with [`mpf_image`](Self::mpf_image),
    /// before or after this call; [`Built::mpf_header`] says where its MP
    /// header landed.
    ///
    /// Panics when called twice: a file has one MPF index.
    pub fn mpf(&mut self, order: Order) -> &mut Self {
        assert!(self.mpf.is_none(), "one MPF segment per file");
        self.mpf = Some(Mpf {
            order,
            header: None,
            number_of_images: None,
            entry_tag: MP_ENTRY,
            entry_type: UNDEFINED,
            entry_len: None,
            offsets: Vec::new(),
            sizes: Vec::new(),
        });
        self.parts.push(Part::Mpf);
        self
    }

    /// Adds an image after the main image's EOI, after the blocks and
    /// images added before, which the MPF index lists: zeros, then
    /// `before`, so that the image starts at a multiple of `align` bytes
    /// from the start of the file, then the stub JPEG of `width` by
    /// `height`. An `align` of 1 puts the image right after `before`.
    /// [`Built::mpf_image`] says where the image landed, from its SOI to the
    /// end of its EOI, which is what its entry's offset and size say.
    ///
    /// Panics when `align` is 0: a mistake in the test.
    pub fn mpf_image(&mut self, width: u16, height: u16, before: &[u8], align: usize) -> &mut Self {
        assert!(align > 0, "an alignment of at least 1");
        self.after_eoi.push(After::Image(Image {
            before: before.to_vec(),
            align,
            width,
            height,
        }));
        self
    }

    /// Writes `header` in place of the MP header's 8 bytes.
    ///
    /// Panics without [`mpf`](Self::mpf) first, as each method that damages
    /// the MPF index does.
    pub fn mpf_header(&mut self, header: [u8; 8]) -> &mut Self {
        self.mpf_index().header = Some(header);
        self
    }

    /// Writes `count` as NumberOfImages, in place of the number of entries.
    pub fn mpf_number_of_images(&mut self, count: u32) -> &mut Self {
        self.mpf_index().number_of_images = Some(count);
        self
    }

    /// Writes `offset` as the offset of the `n`th entry, from 0 for the
    /// main image's.
    pub fn mpf_entry_offset(&mut self, n: usize, offset: u32) -> &mut Self {
        self.mpf_index().offsets.push((n, offset));
        self
    }

    /// Writes `size` as the size of the `n`th entry, from 0 for the main
    /// image's.
    pub fn mpf_entry_size(&mut self, n: usize, size: u32) -> &mut Self {
        self.mpf_index().sizes.push((n, size));
        self
    }

    /// Writes MPEntry's value with `len` bytes, the entries cut or followed
    /// by zeros, and `len` as its count. A value of 4 bytes or fewer is
    /// written in the entry, as TIFF has it.
    pub fn mpf_entry_len(&mut self, len: usize) -> &mut Self {
        self.mpf_index().entry_len = Some(len);
        self
    }

    /// Writes `typ` as MPEntry's field type; its count stays the value's
    /// length in bytes.
    pub fn mpf_entry_type(&mut self, typ: u16) -> &mut Self {
        self.mpf_index().entry_type = typ;
        self
    }

    /// Writes MPEntry's value under the tag `tag` in place of
    /// [`MP_ENTRY`].
    pub fn mpf_entry_tag(&mut self, tag: u16) -> &mut Self {
        self.mpf_index().entry_tag = tag;
        self
    }

    /// Adds a segment with marker `marker` holding `payload`.
    pub fn segment(&mut self, marker: u8, payload: &[u8]) -> &mut Self {
        self.add(marker, payload.to_vec(), Report::Nothing)
    }

    /// Adds `count` fill bytes, `FF` each, before what comes next.
    pub fn fill(&mut self, count: usize) -> &mut Self {
        self.raw(&vec![0xFF; count])
    }

    /// Adds the marker `code` alone: `FF` and `code`, with no length.
    pub fn marker(&mut self, code: u8) -> &mut Self {
        self.raw(&[0xFF, code])
    }

    /// Adds `bytes` as they are.
    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.parts.push(Part::Bytes(bytes.to_vec()));
        self
    }

    /// Adds `bytes` after the main image's EOI, after the blocks and images
    /// added before; [`Built::after_eoi`] says where they landed.
    pub fn after_eoi(&mut self, bytes: &[u8]) -> &mut Self {
        self.after_eoi.push(After::Bytes(bytes.to_vec()));
        self
    }

    /// Adds `count` zero bytes at the end of the file.
    pub fn pad(&mut self, count: usize) -> &mut Self {
        self.pad = self
            .pad
            .checked_add(count)
            .expect("the padding fits in memory");
        self
    }

    /// Writes the file.
    pub fn build(&self) -> Built {
        let mut bytes = vec![0xFF, 0xD8];
        let mut exif = Vec::new();
        let mut app2 = Vec::new();
        // Where the MPF segment's payload starts, written as zeros first.
        let mut mpf_payload_at = None;
        for part in &self.parts {
            match part {
                Part::Bytes(raw) => bytes.extend_from_slice(raw),
                Part::Segment(marker, payload, report) => {
                    // After the marker and the length.
                    let payload_at = to_u64(bytes.len()) + 4;
                    bytes.extend(segment(*marker, payload));
                    let end = to_u64(bytes.len());
                    match report {
                        Report::Nothing => {}
                        Report::Exif => exif.push(payload_at + to_u64(EXIF_ID.len())..end),
                        Report::App2 => app2.push(payload_at..end),
                    }
                }
                Part::Mpf => {
                    mpf_payload_at = Some(bytes.len() + 4);
                    bytes.extend(segment(APP2, &vec![0; self.mpf_payload_len()]));
                }
            }
        }
        // The stub's frame header is FRAME_HEADER_AT bytes into it, SOI
        // included, and its SOI is left out here.
        let frame_header = to_u64(bytes.len() + FRAME_HEADER_AT - 2);
        bytes.extend_from_slice(&jpeg_stub(self.width, self.height, self.marker)[2..]);
        let image_end = to_u64(bytes.len());
        let mut after_eoi = Vec::new();
        let mut mpf_images = Vec::new();
        for after in &self.after_eoi {
            match after {
                After::Bytes(block) => {
                    after_eoi.push(to_u64(bytes.len()));
                    bytes.extend_from_slice(block);
                }
                After::Image(image) => {
                    let unaligned = (bytes.len() + image.before.len()) % image.align;
                    let zeros = (image.align - unaligned) % image.align;
                    bytes.resize(bytes.len() + zeros, 0);
                    bytes.extend_from_slice(&image.before);
                    let start = to_u64(bytes.len());
                    bytes.extend(jpeg_stub(image.width, image.height, 0xC0));
                    mpf_images.push((start..to_u64(bytes.len()), image));
                }
            }
        }
        bytes.resize(bytes.len() + self.pad, 0);
        let mpf_header = match (&self.mpf, mpf_payload_at) {
            (Some(mpf), Some(payload_at)) => {
                let header = payload_at + MPF_ID.len();
                let payload = mpf.payload(to_u64(header), image_end, &mpf_images);
                assert_eq!(
                    payload.len(),
                    self.mpf_payload_len(),
                    "the MPF payload's length"
                );
                bytes[payload_at..payload_at + payload.len()].copy_from_slice(&payload);
                Some(to_u64(header))
            }
            _ => None,
        };
        Built {
            bytes,
            exif,
            app2,
            frame_header,
            image_end,
            after_eoi,
            mpf_header,
            mpf_images: mpf_images.into_iter().map(|(range, _)| range).collect(),
        }
    }

    /// Adds a segment.
    fn add(&mut self, marker: u8, payload: Vec<u8>, report: Report) -> &mut Self {
        self.parts.push(Part::Segment(marker, payload, report));
        self
    }

    /// The MPF index, to damage it.
    fn mpf_index(&mut self) -> &mut Mpf {
        self.mpf
            .as_mut()
            .expect("the MPF segment is added with mpf() before it is damaged")
    }

    /// The length of the MPF segment's payload, which the images' places
    /// do not change.
    fn mpf_payload_len(&self) -> usize {
        let Some(mpf) = &self.mpf else {
            return 0;
        };
        let images = self
            .after_eoi
            .iter()
            .filter(|after| matches!(after, After::Image(_)))
            .count();
        let entries = mpf.entry_len.unwrap_or(MP_ENTRY_LEN * (1 + images));
        let out_of_line = if entries > 4 { entries } else { 0 };
        MPF_ID.len() + to_usize(MP_ENTRY_AT) + out_of_line
    }
}

impl Mpf {
    /// The MPF segment's payload, for an MP header at `header` in the
    /// file, a main image that ends at `image_end`, and `images` after it,
    /// each with where it landed.
    fn payload(&self, header: u64, image_end: u64, images: &[(Range<u64>, &Image)]) -> Vec<u8> {
        let order = self.order;
        let offset_of = |at: u64| to_u32(at - header);

        // The entries, the main image's first.
        let mut entries = Vec::new();
        let mut entry = |attribute: u32, size: u32, offset: u32| {
            entries.extend_from_slice(&order.u32(attribute));
            entries.extend_from_slice(&order.u32(size));
            entries.extend_from_slice(&order.u32(offset));
            entries.extend_from_slice(&[0; 4]);
        };
        entry(0xA003_0000, to_u32(image_end), 0);
        for (range, image) in images {
            let attribute = if image.width <= 640 && image.height <= 480 {
                0x4001_0001
            } else {
                0x4001_0002
            };
            entry(
                attribute,
                to_u32(range.end - range.start),
                offset_of(range.start),
            );
        }
        let count = 1 + images.len();
        for &(n, offset) in &self.offsets {
            assert!(n < count, "entry {n} of {count}");
            entries[n * MP_ENTRY_LEN + 8..n * MP_ENTRY_LEN + 12]
                .copy_from_slice(&order.u32(offset));
        }
        for &(n, size) in &self.sizes {
            assert!(n < count, "entry {n} of {count}");
            entries[n * MP_ENTRY_LEN + 4..n * MP_ENTRY_LEN + 8].copy_from_slice(&order.u32(size));
        }
        if let Some(len) = self.entry_len {
            entries.resize(len, 0);
        }

        let mut payload = MPF_ID.to_vec();
        match self.header {
            Some(header) => payload.extend_from_slice(&header),
            None => {
                payload.extend_from_slice(&order.mark());
                payload.extend_from_slice(&order.u16(42));
                payload.extend_from_slice(&order.u32(MP_INDEX_AT));
            }
        }
        // The MP Index IFD: its count, its 3 entries and its next-IFD
        // pointer.
        payload.extend_from_slice(&order.u16(3));
        let mut ifd_entry = |tag: u16, typ: u16, count: u32, field: [u8; 4]| {
            payload.extend_from_slice(&order.u16(tag));
            payload.extend_from_slice(&order.u16(typ));
            payload.extend_from_slice(&order.u32(count));
            payload.extend_from_slice(&field);
        };
        ifd_entry(MPF_VERSION, UNDEFINED, 4, *b"0100");
        let number = self.number_of_images.unwrap_or(to_u32(to_u64(count)));
        ifd_entry(NUMBER_OF_IMAGES, LONG, 1, order.u32(number));
        let len = to_u32(to_u64(entries.len()));
        if entries.len() > 4 {
            ifd_entry(self.entry_tag, self.entry_type, len, order.u32(MP_ENTRY_AT));
        } else {
            let mut field = [0; 4];
            field[..entries.len()].copy_from_slice(&entries);
            ifd_entry(self.entry_tag, self.entry_type, len, field);
            entries.clear();
        }
        payload.extend_from_slice(&[0; 4]);
        payload.extend_from_slice(&entries);
        payload
    }
}

/// A file written by [`JpegBuilder::build`], with where its parts landed.
///
/// The methods that take `n` count from 0, in the order the parts were
/// added, and panic when there are not that many: a mistake in the test.
#[derive(Clone, Debug)]
pub struct Built {
    /// The file.
    pub bytes: Vec<u8>,
    exif: Vec<Range<u64>>,
    app2: Vec<Range<u64>>,
    frame_header: u64,
    image_end: u64,
    after_eoi: Vec<u64>,
    mpf_header: Option<u64>,
    mpf_images: Vec<Range<u64>>,
}

impl Built {
    /// Where the TIFF structure of the `n`th Exif segment lies: from its
    /// TIFF header, 6 bytes into the segment's payload, to the end of the
    /// segment.
    pub fn exif_tiff(&self, n: usize) -> Range<u64> {
        self.exif[n].clone()
    }

    /// Where the payload of the `n`th segment added with
    /// [`JpegBuilder::app2`] lies: from the byte after its length to the end
    /// of the segment.
    pub fn app2_payload(&self, n: usize) -> Range<u64> {
        self.app2[n].clone()
    }

    /// Where the main image's frame header starts: the `FF` of its marker.
    pub fn frame_header(&self) -> u64 {
        self.frame_header
    }

    /// Where the main image ends: the byte after its EOI, which is the size
    /// of the first image in the MPF index.
    pub fn image_end(&self) -> u64 {
        self.image_end
    }

    /// Where the `n`th block added with [`JpegBuilder::after_eoi`] starts.
    pub fn after_eoi(&self, n: usize) -> u64 {
        self.after_eoi[n]
    }

    /// Where the MP header of the MPF segment lies, 4 bytes into the
    /// segment's payload: the position every offset of the index counts
    /// from. Panics when the file has no MPF segment.
    pub fn mpf_header(&self) -> u64 {
        self.mpf_header.expect("an MPF segment, added with mpf()")
    }

    /// Where the `n`th image added with [`JpegBuilder::mpf_image`] lies:
    /// from its SOI to the end of its EOI, the bytes before it left out.
    pub fn mpf_image(&self, n: usize) -> Range<u64> {
        self.mpf_images[n].clone()
    }

    /// Where every image added with [`JpegBuilder::mpf_image`] lies, in the
    /// order they were added.
    pub fn mpf_images(&self) -> Vec<Range<u64>> {
        self.mpf_images.clone()
    }

    /// The first `len` bytes of the file, which must be at most its length.
    pub fn truncated(&self, len: usize) -> Vec<u8> {
        self.bytes[..len].to_vec()
    }
}

/// `n` as a `u64`, which it always fits in where a test runs.
fn to_u64(n: usize) -> u64 {
    u64::try_from(n).expect("a length fits in a u64")
}

/// `n` as a `usize`, which a test's small numbers always fit in.
fn to_usize(n: u32) -> usize {
    usize::try_from(n).expect("a small number fits in a usize")
}

/// `n` as a `u32`, as the MPF index writes sizes and offsets: a test's file
/// is far smaller than 4 GiB.
fn to_u32(n: u64) -> u32 {
    u32::try_from(n).expect("a test file's offsets fit in 32 bits")
}
