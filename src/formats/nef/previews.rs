// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where a NEF file keeps its previews, and how the reader finds them.
//!
//! A Nikon camera writes several JPEG previews beside the raw data: in the
//! SubIFDs, in IFD0 on some recent bodies, and in the MakerNote's
//! PreviewIFD. The shared TIFF preview finder finds them over the whole
//! tree. The PreviewIFD's JPEG is the one candidate this reader adds from
//! outside the tree ([`nikon::preview_candidate`]), out of the MakerNote
//! the NEF reader read once for the whole file.
//!
//! Which body puts which preview where, and at what size, is in the NEF
//! notes (`docs/formats/nef/container-and-previews.md`, sections 3 and 4).

use std::io::{Read, Seek};

use crate::container::tiff::Tree;
use crate::error::Result;
use crate::formats::makernote::nikon::{self, MakerNote};
use crate::formats::tiff_previews;
use crate::io::Source;
use crate::preview::Preview;

/// The previews of the NEF file in `src`, whose IFDs are `tree` and whose
/// MakerNote, already read, is `makernote`, in the order they were found:
/// the tree's, then the MakerNote's.
///
/// An IFD whose tags cannot be read costs only its own preview, and a
/// MakerNote that does not hold together gives no candidate.
///
/// # Errors
///
/// `Io` when the reader fails.
pub(super) fn previews<R: Read + Seek>(
    src: &mut Source<R>,
    tree: &Tree,
    makernote: Option<&MakerNote>,
) -> Result<Vec<Preview>> {
    let extra = makernote.and_then(nikon::preview_candidate);
    tiff_previews::find(src, tree, extra.as_slice())
}

#[cfg(test)]
mod tests {
    use crate::container::tiff::tags::{
        JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKE,
    };
    use crate::container::tiff::{Block, walk};
    use crate::geometry::Size;
    use crate::io::Source;
    use crate::jpeg_stub::jpeg_stub;
    use crate::limits::Limits;
    use crate::preview::Preview;
    use crate::testing::size;
    use crate::tiff_builder::{IfdId, Order, TiffBuilder};
    use std::io::Cursor;

    /// Adds a stub JPEG of `width` by `height` to the file, and points
    /// `ifd`'s JPEGInterchangeFormat and JPEGInterchangeFormatLength at it.
    fn jpeg(builder: &mut TiffBuilder, ifd: IfdId, width: u16, height: u16) {
        let blob = builder.blob(&jpeg_stub(width, height, 0xC0));
        builder.long_blob(ifd, JPEG_INTERCHANGE_FORMAT, blob);
        builder.long_blob_len(ifd, JPEG_INTERCHANGE_FORMAT_LENGTH, blob);
    }

    #[test]
    fn parse_finds_the_jpegs_of_ifd0_and_a_subifd() {
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let sub = builder.ifd();
        builder.ascii(ifd0, MAKE, "NIKON CORPORATION");
        jpeg(&mut builder, ifd0, 160, 120);
        jpeg(&mut builder, sub, 1620, 1080);
        builder.subifds(ifd0, &[sub]);
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();

        let parsed = super::super::parse(&mut src, &tree, &Limits::default()).unwrap();
        let sizes: Vec<Size> = parsed.previews.iter().map(Preview::size).collect();
        assert_eq!(sizes, [size(160, 120), size(1620, 1080)]);
    }

    #[test]
    fn a_file_without_a_makernote_still_gives_its_tree_previews() {
        // An Exif IFD, as every Nikon file has, with no MakerNote in it.
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let exif = builder.ifd();
        builder
            .ascii(ifd0, MAKE, "NIKON CORPORATION")
            .exif(ifd0, exif);
        // ExifVersion.
        builder.undefined(exif, 0x9000, b"0232");
        jpeg(&mut builder, ifd0, 160, 120);
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();

        let makernote = super::nikon::read(&mut src, &tree, &Limits::default()).unwrap();
        let previews = super::previews(&mut src, &tree, makernote.as_ref()).unwrap();
        let sizes: Vec<Size> = previews.iter().map(Preview::size).collect();
        assert_eq!(sizes, [size(160, 120)]);
    }
}
