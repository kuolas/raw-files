// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where a NEF file keeps its previews, and how the reader finds them.
//!
//! A Nikon camera writes several JPEG previews beside the raw data: in the
//! SubIFDs, in IFD0 on some recent bodies, and in the MakerNote's
//! PreviewIFD. The shared TIFF preview finder finds them over the whole
//! tree. The PreviewIFD's JPEG is the one candidate from outside the tree,
//! out of the MakerNote that the NEF reader reads once for the whole file
//! ([`parse_tiff`](crate::formats::parse_tiff)). This module holds no code
//! of its own, only the tests of what the reader finds.
//!
//! Which body puts which preview where, and at what size, is in the NEF
//! notes (`docs/formats/nef/container-and-previews.md`, sections 3 and 4).

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

        let parsed = super::super::parse(&mut src, &tree, &Limits::default()).unwrap();
        let sizes: Vec<Size> = parsed.previews.iter().map(Preview::size).collect();
        assert_eq!(sizes, [size(160, 120)]);
    }
}
