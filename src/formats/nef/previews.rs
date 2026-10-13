// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where a NEF file keeps its previews, and how the reader finds them.
//!
//! A Nikon camera writes several JPEG previews next to the raw data. Some
//! recent bodies, the Z 9 among them, put a small one in IFD0; the
//! full-size one, and on bodies from about 2012 on a medium one of about
//! 1620 by 1080 pixels, sit in SubIFDs, in an order that varies between
//! bodies. The MakerNote's PreviewIFD holds a small one too, which on the
//! oldest bodies is the only JPEG; reading it is still to come, and it will
//! join as the one candidate the reader adds outside the tree.
//!
//! The shared TIFF preview finder does the finding, over the whole tree:
//! it tells the previews from the raw data and measures each one.

use std::io::{Read, Seek};

use crate::container::tiff::Tree;
use crate::error::Result;
use crate::formats::tiff_previews;
use crate::io::Source;
use crate::preview::Preview;

/// The previews of the NEF file in `src`, whose IFDs are `tree`, in the
/// order they were found.
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
        JPEG_INTERCHANGE_FORMAT, JPEG_INTERCHANGE_FORMAT_LENGTH, MAKE,
    };
    use crate::container::tiff::{parse_header, walk};
    use crate::io::Source;
    use crate::jpeg_stub::jpeg_stub;
    use crate::limits::Limits;
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
        let header = parse_header(&mut src).unwrap();
        let tree = walk(&mut src, header, &Limits::default()).unwrap();

        let parsed = super::super::parse(&mut src, &tree, &Limits::default()).unwrap();
        let sizes: Vec<(u32, u32)> = parsed
            .previews
            .iter()
            .map(|p| (p.width(), p.height()))
            .collect();
        assert_eq!(sizes, [(160, 120), (1620, 1080)]);
    }
}
