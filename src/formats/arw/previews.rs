// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where a Sony RAW file keeps its previews, and how the reader finds them.
//!
//! A Sony camera writes its JPEG previews into the IFDs of IFD0's chain,
//! never into a SubIFD. IFD0 holds a medium one: 1616 by 1080 pixels on
//! most bodies since 2008, 640 by 480 or 640 by 424 on the earliest, and
//! cut to the picture's shape when the camera was set to another aspect
//! ratio. IFD1, the next IFD of the chain, holds a thumbnail of 160 by 120
//! pixels. Bodies that write version 4.0 or later of Sony's file format
//! (the FileFormat tag of the MakerNote) add IFD2, after IFD1, with a JPEG
//! of the picture's full size, whose Compression is 7, the newer TIFF code
//! for JPEG. The SRF files of the earliest bodies are the exception: their
//! IFD0 describes the raw data, and their only JPEG is in IFD1.
//!
//! The raw data is in IFD0's one SubIFD, and on the newest bodies it can
//! look like a preview. A lossless raw is stored as tiles that are each a
//! JPEG stream, starting with the start-of-image marker and a lossless
//! frame header; a medium or small raw is a YCbCr image, as IFD2's JPEG
//! is. The shared TIFF preview finder keeps them out twice over: its rule
//! that an IFD describing raw data (by its colour filter array, its samples
//! wider than 8 bits, its tiles or its full-resolution role) points at no
//! preview, and its rejection of lossless JPEG streams behind that rule.
//!
//! So the finder does all the work, over the whole tree, and this module
//! adds no candidate of its own: Sony's MakerNote points at no preview that
//! the IFDs do not already list. A file with no preview the finder can
//! take, such as a damaged one, opens with an empty list.

use std::io::{Read, Seek};

use crate::container::tiff::Tree;
use crate::error::Result;
use crate::formats::tiff_previews;
use crate::io::Source;
use crate::preview::Preview;

/// The previews of the Sony file in `src`, whose IFDs are `tree`, in the
/// order the shared finder found them.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the file holds is an error here: an
/// IFD whose tags cannot be read costs only its own preview.
pub(super) fn previews<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Vec<Preview>> {
    tiff_previews::find(src, tree, &[])
}

#[cfg(test)]
mod tests {
    use crate::container::tiff::tags::{
        JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKE, NEW_SUBFILE_TYPE,
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
    fn parse_finds_the_jpegs_of_ifd0_and_of_ifd2_along_the_chain() {
        // IFD0 → IFD1 → IFD2, as Sony's newer bodies write them; IFD1
        // holds no JPEG here, so only the chain reaches IFD2's.
        let mut builder = TiffBuilder::new(Order::Little);
        let ifd0 = builder.ifd();
        let (ifd1, ifd2) = (builder.ifd(), builder.ifd());
        builder.ascii(ifd0, MAKE, "SONY");
        jpeg(&mut builder, ifd0, 1616, 1080);
        builder.long(ifd1, NEW_SUBFILE_TYPE, &[1]);
        jpeg(&mut builder, ifd2, 7008, 4672);
        builder.next(ifd0, Some(ifd1)).next(ifd1, Some(ifd2));
        let mut src = Source::new(Cursor::new(builder.build().bytes)).unwrap();
        let file_block = Block::file(src.len());
        let tree = walk(&mut src, &file_block, &Limits::default()).unwrap();

        let parsed = super::super::parse(&mut src, &tree, &Limits::default()).unwrap();
        let sizes: Vec<Size> = parsed.previews.iter().map(Preview::size).collect();
        assert_eq!(sizes, [size(1616, 1080), size(7008, 4672)]);
    }
}
