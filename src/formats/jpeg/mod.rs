// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG reader: how the crate recognises a JPEG file, and how it finds
//! the file's previews and reads its metadata.
//!
//! A JPEG file is a picture as a camera writes it beside its raw file or
//! instead of it, or as an editor exports it: one JPEG stream, with the
//! camera's metadata in an Exif segment before the picture's frame header,
//! and often smaller copies of the picture stored after its end and listed
//! in an MPF segment. raw-files opens it as a format of its own, through
//! the same handle and into the same types as a raw file (ADR014). It holds
//! no raw data.
//!
//! The JPEG container has walked the file's segments up to its scan and
//! kept its first frame header, its first Exif segment and its first MPF
//! segment ([`Segments`]). For this reader a file is a JPEG when that frame
//! header gives a size, neither side zero, and is not lossless
//! ([`claims`]): a lossless frame keeps every sample value exactly, which is
//! how some raw files store their raw data, not a picture to show (T.81
//! Table B.1, p. 32). Nothing else decides: a file without an Exif segment,
//! or whose Exif segment is damaged, is a JPEG all the same, and the file's
//! name is never looked at.
//!
//! # The previews
//!
//! - **The file itself**: its whole byte range, exactly as stored, with the
//!   size its frame header gives. A viewer opens the whole file, and a
//!   decoder stops at the main image's end marker, so the MPF images and
//!   any padding after it, such as the zeros some Sony bodies write, do no
//!   harm (CIPA DC-008 4.7, p. 99). It is built from the frame header the
//!   claim checked, not probed again. No other preview starts at the
//!   file's first byte: the thumbnail lies inside the Exif segment, and an
//!   MPF image after the main image.
//! - **The thumbnail in IFD1** of the Exif segment's TIFF structure, which
//!   the shared TIFF preview finder (`tiff_previews`) finds as it finds a
//!   raw file's: counted from that structure's header, and inside the
//!   segment (DC-008 4.5.8, pp. 25 to 26).
//! - **Each image the MPF list holds after the first**, at the MP header's
//!   position plus the offset its entry gives, over the size its entry
//!   gives (CIPA DC-007 5.2.3.3.3, p. 16), whatever the type of image the
//!   entry names. These images lie after the main image, outside the
//!   segment that lists them, so they are bounded by the file alone
//!   ([`mpf::images`]).
//! - **The JPEG of the Nikon MakerNote's PreviewIFD**, in a file whose
//!   MakerNote the reader reads ([below](#the-metadata)), as the D70's
//!   JPEGs hold one of 564 by 372 pixels. The MakerNote reader finds it as
//!   in a NEF ([`nikon::preview_candidate`]): counted from the MakerNote's
//!   own inner header, and wholly inside the length the MakerNote
//!   declares, which lies inside the Exif segment. A JPEG that the
//!   PreviewIFD places outside that length is not a preview.
//!
//! The thumbnail, the MPF images and the MakerNote's preview are checked
//! as every preview is, by the shared finder's
//! [`check`](tiff_previews::check): each must lie inside the file, start
//! with SOI and hold, within its own bytes, a frame header that gives a
//! size and is not lossless, and is measured from that frame header. Every
//! preview, the file included, takes the orientation of IFD0 of the Exif
//! segment, or none without one.
//!
//! # Damage
//!
//! The Exif segment is read as a bounded block, whose TIFF header is the
//! base of every offset. A TIFF header or an IFD0 that cannot be read, or
//! a limit of the caller's [`Limits`] crossed while walking the segment's
//! structure, gives no tree: it costs the segment's values and its
//! thumbnail, never the file, which opens with its own preview and its MPF
//! images, listed in a segment of their own. The image size remains, since
//! it comes from the frame header, not from the segment. Past IFD0,
//! damage costs only what it touches, as in a raw file: an IFD or a value
//! that lies outside the segment is damage, and costs what it holds. A
//! MakerNote that does not hold together, or in which a limit is crossed,
//! costs what it would have given, as in a raw file: a Nikon MakerNote its
//! values, its focus point and its preview, as in a NEF, and a Sony
//! MakerNote its focus point, as in an ARW. An MPF list that does not hold
//! together costs only its images. Only a failure of the reader is an
//! error.
//!
//! # The metadata
//!
//! The metadata is read from the Exif segment's tree by the reader the
//! TIFF-based formats share (`tiff_metadata`), by the same rules as in a
//! raw file, the standard tags first (ADR011): the camera's make and model
//! and the orientation from IFD0, and the serial number, the capture time,
//! the exposure and the lens from the Exif IFD.
//!
//! Then the MakerNote is read, by IFD0's Make, as in a raw file (ADR014),
//! once, by the reader of the maker the Make names ([`Maker`]); the Make
//! is compared trimmed of spaces at either end and in any case, as the
//! claims of the raw formats compare it.
//!
//! - **When the Make starts with `NIKON`**, as the NEF claim tests it, the
//!   Nikon MakerNote reader (`makernote/nikon.rs`) reads it as it reads a
//!   NEF's. Its offsets count from its own inner header, so it reads the
//!   same inside the Exif segment. It gives what it gives a NEF: the
//!   serial number, the ISO, the lens's range and the capture time's
//!   offset from UTC, each only when the standard tags gave none, so that
//!   an Exif value always wins; and the focus point, below.
//! - **When the Make starts with `SONY`**, as the ARW claim first tests it,
//!   the Sony MakerNote reader (`makernote/sony.rs`) reads it as it reads
//!   an ARW's, for the focus point alone, below. Its offsets count from
//!   the TIFF header of the structure that holds it, which in a JPEG is
//!   the one inside the Exif segment, the base of the Exif IFD's offsets,
//!   as the Sony camera JPEGs that ADR014 describes count them; its IFD
//!   follows Sony's 12-byte `SONY DSC` header, which every one of them
//!   writes. The ARW claim's second test, a Hasselblad Make with the
//!   pointer to Sony's private directory, looks for a raw file's
//!   structure, and a JPEG under Hasselblad's Make is not read as Sony's.
//!
//! A MakerNote of one maker's layout under another maker's Make is not
//! read: Nikon's software writes Nikon-style MakerNotes into other makers'
//! files. No other maker's MakerNote is read.
//!
//! The image size is the size in the frame header, the picture's own (T.81
//! B.2.2 and Figure B.3, p. 35), never a size the tree gives: the Exif
//! pixel dimensions are not read, since an editor may leave them
//! unchanged, and an IFD that the shared reader would take for raw data
//! gives no size either. A JPEG holds no raw data, so it has no raw size.
//! Without a tree, the image size is the only value. Both are set before
//! the MakerNote's values are read, so that its reader takes them for a
//! JPEG's: with no raw size, CropArea, which a MakerNote copied from a raw
//! file holds, gives no image size (ADR014).
//!
//! # The focus point
//!
//! The Nikon MakerNote reader places the focus point by the rules it
//! follows in a NEF (ADR012), with the JPEG's own pixels as the picture
//! ([`Picture::Jpeg`]). A JPEG shot in a crop mode, as the Z 9 writes it,
//! records its point in the camera's full output and holds no CropArea,
//! and its point is placed on the JPEG's pixels (ADR015): a DX crop's
//! scaled onto them, a 1:1 or 16:9 crop's moved into the centred window of
//! the recorded frame that has the JPEG's shape. A JPEG whose MakerNote
//! holds a CropArea gives no point on a crop, since it has no raw size to
//! check CropArea against.
//!
//! The Sony MakerNote reader gives the point as it gives an ARW's (ADR012):
//! the frame and the centre from FocusLocation, or FocusLocation2 without
//! it, the area from FocusFrameSize where it records one, and none in
//! manual focus. Sony's frame is the picture as framed in the camera, so
//! the point is not placed: the ILCE-7M3's JPEG at the camera's M size
//! records its point at (1783, 1804) in a frame of its own 3936 by 2624
//! pixels. The DSLR-A100's MakerNote, of an older layout, holds no
//! FocusLocation, and gives none.
//!
//! Then the point is kept only when its frame has the picture's aspect
//! ratio, to within one pixel of rounding in either size ([`same_shape`]),
//! in the stored orientation (ADR014). A camera's own JPEG passes, and so
//! does an export scaled from it; an export cut to another shape, or turned
//! upright in its pixels, which may keep the camera's MakerNote, gives no
//! point. The rule applies to whatever point the MakerNote reader gave, so
//! it cannot refuse a Nikon crop-mode JPEG cut again: its point is placed
//! on the export's own pixels, a window of their shape, and is misplaced
//! if the cut changed the shape (ADR015).

use std::io::{Read, Seek};

use super::makernote::nikon::{self, NIKON, Picture};
use super::makernote::sony::{self, SONY};
use super::tiff_metadata::{self, Standard};
use super::tiff_previews::{self, Candidate};
use super::{FormatSpec, JPEG_UNCLAIMED, Parsed, make_starts_with};
use crate::container::jpeg::{Frame, Segments, mpf};
use crate::container::tiff::{Tree, skip_damage, walk};
use crate::error::{Error, Result};
use crate::format::Format;
use crate::geometry::Size;
use crate::io::Source;
use crate::limits::Limits;
use crate::metadata::Metadata;
use crate::preview::{Preview, PreviewFormat};

/// The JPEG format's entry in the JPEG container's list of formats.
pub(crate) fn spec<R: Read + Seek>() -> FormatSpec<R, Segments> {
    FormatSpec {
        format: Format::Jpeg,
        claims,
        parse,
    }
}

/// The frame header of the picture the file holds: the first frame header
/// of the file, when it gives a size and is not lossless; `None` otherwise.
///
/// [`segments`](crate::container::jpeg::segments) keeps a frame header
/// only when neither side is zero, so a frame header that gives no size
/// never reaches here.
fn picture(segments: &Segments) -> Option<Frame> {
    segments.frame.filter(|frame| !frame.is_lossless())
}

/// Whether the file whose segments are `segments` is a JPEG file: whether
/// its first frame header gives a size and is not lossless ([`picture`]).
///
/// It reads nothing: the container read the frame header already.
///
/// # Errors
///
/// None. It returns a `Result` because every format's claim does.
fn claims<R: Read + Seek>(_: &mut Source<R>, segments: &Segments) -> Result<bool> {
    Ok(picture(segments).is_some())
}

/// Reads the previews and the metadata of a JPEG file that [`claims`]
/// accepted.
///
/// The Exif segment's TIFF structure, when there is one, is walked within
/// `limits`, and gives the standard metadata, the orientation among it,
/// and the thumbnail's candidate. When IFD0's Make names Nikon, the Nikon
/// MakerNote is read, within `limits`, and gives the values the standard
/// tags lack, the focus point among them, placed on the picture, and the
/// candidate of its preview; when it names Sony, the Sony MakerNote is
/// read, within `limits`, and gives the focus point alone, as recorded
/// ([`MakerNote::read`]). The MPF list, read within `limits` as well,
/// gives a candidate for each image after the first. The shared finder
/// checks them all, and the file itself joins them as a preview of its
/// own, with its frame header's size, which is the metadata's image size.
/// A focus point is kept only when its frame has the picture's shape
/// ([`same_shape`]); see the [module documentation](self).
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the file holds is an error here.
/// `UnknownFormat`, with the message detection gives a JPEG file no format
/// claims, for segments that [`claims`] refuses, which detection never
/// passes here.
fn parse<R: Read + Seek>(
    src: &mut Source<R>,
    segments: &Segments,
    limits: &Limits,
) -> Result<Parsed> {
    let Some(frame) = picture(segments) else {
        return Err(Error::unknown_format(JPEG_UNCLAIMED));
    };
    let size = Size {
        width: u32::from(frame.width),
        height: u32::from(frame.height),
    };
    // A TIFF header or an IFD0 that cannot be read, or a limit crossed
    // while walking, costs the tree, not the file.
    let tree = match &segments.exif {
        Some(exif) => skip_damage(walk(src, exif, limits))?,
        None => None,
    };
    let (standard, mut candidates, makernote) = match &tree {
        Some(tree) => (
            tiff_metadata::read(src, tree)?.metadata,
            tiff_previews::tree_candidates(src, tree)?,
            MakerNote::read(src, tree, limits)?,
        ),
        None => (Metadata::default(), Vec::new(), None),
    };
    // The picture's own size, whatever sizes the tree gives, and no raw
    // size: a JPEG holds no raw data. Set before the MakerNote's values
    // are read, so that its reader takes them for the JPEG's: with no raw
    // size, CropArea gives no image size, and a crop is placed on the
    // picture's pixels.
    let standard = Standard {
        metadata: Metadata {
            raw_size: None,
            image_size: Some(size),
            ..standard
        },
        from_default_crop: false,
    };
    let mut metadata = match &makernote {
        Some(MakerNote::Nikon(makernote)) => {
            candidates.extend(nikon::preview_candidate(src, makernote, limits)?);
            nikon::metadata(src, makernote, standard, Picture::Jpeg)?
        }
        // Sony's gives the focus point alone, as in an ARW.
        Some(MakerNote::Sony(makernote)) => Metadata {
            focus: sony::focus(src, makernote)?,
            ..standard.metadata
        },
        None => standard.metadata,
    };
    metadata.focus = metadata
        .focus
        .filter(|focus| same_shape(focus.frame(), size));
    if let Some(index) = &segments.mpf {
        let images = mpf::images(src, index, limits)?;
        candidates.extend(images.into_iter().filter_map(|image| {
            Some(Candidate {
                offset: image.start,
                length: image.end.checked_sub(image.start)?,
            })
        }));
    }
    // Every preview takes the metadata's orientation, IFD0's.
    let mut previews = tiff_previews::check(src, &candidates, metadata.orientation)?;
    previews.push(Preview::new(
        0,
        src.len(),
        size.width,
        size.height,
        PreviewFormat::Jpeg,
        metadata.orientation,
    ));
    Ok(Parsed { previews, metadata })
}

/// A maker whose MakerNote the JPEG reader reads, as IFD0's Make names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Maker {
    /// Nikon: a Make that starts with [`NIKON`], as the NEF claim tests it.
    Nikon,
    /// Sony: a Make that starts with [`SONY`], as the ARW claim tests it
    /// first. The claim's second test, a Hasselblad Make with Sony's
    /// private pointer, is a raw file's structure, which a JPEG lacks.
    Sony,
}

impl Maker {
    /// Every maker whose MakerNote the JPEG reader reads. No Make starts
    /// with two of their names, so the order does not matter.
    const ALL: [Maker; 2] = [Maker::Nikon, Maker::Sony];

    /// The name the maker's Make starts with.
    fn name(self) -> &'static [u8] {
        match self {
            Maker::Nikon => NIKON,
            Maker::Sony => SONY,
        }
    }

    /// The maker that the IFD0 of `tree` names in its Make, trimmed of
    /// spaces at either end and in any case, as the claims of the raw
    /// formats test it ([`make_starts_with`]); `None` for any other Make, or
    /// none.
    ///
    /// # Errors
    ///
    /// As for [`make_starts_with`].
    fn of<R: Read + Seek>(src: &mut Source<R>, tree: &Tree) -> Result<Option<Maker>> {
        for maker in Maker::ALL {
            if make_starts_with(src, tree, maker.name())? {
                return Ok(Some(maker));
            }
        }
        Ok(None)
    }
}

/// The MakerNote of a JPEG file, read once by its maker's reader, which
/// every use of it starts from.
#[derive(Debug)]
enum MakerNote {
    /// Nikon's: its values, its focus point and its preview.
    Nikon(nikon::MakerNote),
    /// Sony's: its focus point.
    Sony(sony::MakerNote),
}

impl MakerNote {
    /// The MakerNote of the JPEG file in `src`, whose Exif segment's TIFF
    /// structure is `tree`, read within `limits` by the reader of the maker
    /// that IFD0's Make names ([`Maker::of`]), as [`nikon::read`] or
    /// [`sony::read`] gives it; `None` under any other Make.
    ///
    /// Make decides, not what the MakerNote looks like, as in a raw file:
    /// Nikon's editing software writes Nikon-style MakerNotes into the files
    /// of other makes, whose Make it leaves as it was, and a Sony MakerNote
    /// without a header, as an ARW's is, holds nothing that names Sony. A
    /// Make that cannot be read is taken for another maker's, so that it
    /// costs only the MakerNote.
    ///
    /// # Errors
    ///
    /// `Io` when the reader fails. Nothing the file holds is an error here.
    fn read<R: Read + Seek>(
        src: &mut Source<R>,
        tree: &Tree,
        limits: &Limits,
    ) -> Result<Option<MakerNote>> {
        Ok(match skip_damage(Maker::of(src, tree))?.flatten() {
            Some(Maker::Nikon) => nikon::read(src, tree, limits)?.map(MakerNote::Nikon),
            Some(Maker::Sony) => sony::read(src, tree, limits)?.map(MakerNote::Sony),
            None => None,
        })
    }
}

/// Whether a focus point measured in `frame` fits a JPEG whose picture is
/// `picture` pixels: whether `frame` has the picture's aspect ratio, to
/// within one pixel of rounding in either of the two sizes (ADR014).
///
/// That is when the frame's width times the picture's height and the
/// frame's height times the picture's width differ by at most the longest
/// of the four sides. When one of the two sizes is the other scaled, each
/// side rounded to the nearest pixel, the products differ by at most half
/// the sum of the other size's two sides, which is within the longest
/// side: a 640 by 427 frame, 3:2 rounded, against a 6000 by 4000 picture
/// differs by 2,000, within 6,000. A picture cut to another shape, or
/// turned upright in its pixels, does not pass. Both sizes are in the
/// stored orientation: Orientation turns neither.
///
/// The products are of two `u32`s, which a `u64` always holds, so nothing
/// here can overflow.
fn same_shape(frame: Size, picture: Size) -> bool {
    let [fw, fh, pw, ph] =
        [frame.width, frame.height, picture.width, picture.height].map(u64::from);
    let longest = fw.max(fh).max(pw).max(ph);
    (fw * ph).abs_diff(fh * pw) <= longest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::jpeg::segments;
    use crate::container::tiff::tags::{MAKE, MAKER_NOTE};
    use crate::error::ErrorKind;
    use crate::jpeg_builder::JpegBuilder;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{Order, TiffBuilder};
    use std::io::Cursor;

    /// The bytes of the file `builder` describes, with its segments as the
    /// JPEG container reads them.
    fn read(builder: &JpegBuilder) -> (Vec<u8>, Segments) {
        let bytes = builder.build().bytes;
        let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
        let found = segments(&mut src).unwrap().expect("a JPEG file");
        (bytes, found)
    }

    /// Whether the JPEG reader claims the file `builder` describes.
    fn claimed(builder: &JpegBuilder) -> bool {
        let (bytes, found) = read(builder);
        claims(&mut Source::new(Cursor::new(bytes)).unwrap(), &found).unwrap()
    }

    /// A file whose main image has the frame header marker `marker`.
    fn with_marker(marker: u8) -> JpegBuilder {
        let mut builder = JpegBuilder::new();
        builder.frame(640, 480, marker);
        builder
    }

    #[test]
    fn a_frame_header_that_gives_a_size_is_claimed() {
        for marker in [0xC0, 0xC1, 0xC2] {
            assert!(claimed(&with_marker(marker)), "{marker:02X}");
        }
    }

    #[test]
    fn a_lossless_frame_header_is_not_claimed() {
        for marker in [0xC3, 0xC7, 0xCB, 0xCF] {
            assert!(!claimed(&with_marker(marker)), "{marker:02X}");
        }
    }

    #[test]
    fn a_file_without_a_frame_header_before_its_scan_is_not_claimed() {
        // SOS before the main image's frame header: the walk ends there.
        let mut builder = JpegBuilder::new();
        builder.segment(0xDA, &[1, 1, 0x00, 0, 63, 0]);
        assert!(!claimed(&builder));
    }

    #[test]
    fn parse_refuses_what_the_claim_refuses() {
        let (bytes, lossless) = read(&with_marker(0xC3));
        for found in [Segments::default(), lossless] {
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            let Err(err) = parse(&mut src, &found, &Limits::default()) else {
                panic!("parsed segments the claim refuses: {found:?}");
            };
            assert_eq!(err.kind(), ErrorKind::UnknownFormat, "{err:?}");
            assert_eq!(err.to_string(), JPEG_UNCLAIMED);
        }
    }

    #[test]
    fn a_failing_reader_is_an_io_error() {
        // The Exif segment past `FAR`, behind a comment: walking its TIFF
        // structure fails.
        let tiff = [b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut exif = JpegBuilder::new();
        exif.segment(0xFE, &[0x20; 60_000])
            .segment(0xFE, &[0x20; 60_000])
            .exif(&tiff);
        // An MPF image past `FAR`, behind zeros: probing it fails.
        let mut mpf = JpegBuilder::new();
        mpf.mpf(Order::Little)
            .after_eoi(&vec![0; usize::try_from(FAR).unwrap()])
            .mpf_image(640, 424, &[], 1);
        for (case, builder) in [("Exif", exif), ("MPF", mpf)] {
            let (bytes, found) = read(&builder);
            assert!(u64::try_from(bytes.len()).unwrap() > FAR, "{case}");
            // Read without the failure, the same bytes give their previews.
            let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
            parse(&mut src, &found, &Limits::default()).unwrap();

            let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
            let Err(err) = parse(&mut src, &found, &Limits::default()) else {
                panic!("{case}: the reader fails");
            };
            assert_eq!(err.kind(), ErrorKind::Io, "{case}: {err:?}");
        }
    }

    /// What parsing a JPEG file gives when its reader fails past `FAR`,
    /// where the file holds the MakerNote `block`, under IFD0's Make
    /// `make`. The Exif segment's TIFF structure starts 1,024 bytes before
    /// `FAR`, behind a comment, with IFD0 and the Exif IFD before `FAR` and
    /// the MakerNote 2,048 bytes in, after it: reading the MakerNote fails.
    /// The same bytes, read without the failure, are checked to open.
    fn parsed_when_the_makernote_fails(make: &str, block: &[u8]) -> Result<Parsed> {
        const MAKER_NOTE_AT: u64 = 2048;
        let mut tiff = TiffBuilder::new(Order::Little);
        let (ifd0, exif) = (tiff.ifd(), tiff.ifd());
        tiff.ascii(ifd0, MAKE, make).exif(ifd0, exif);
        let makernote = tiff.blob_at(MAKER_NOTE_AT, block);
        tiff.undefined_blob(exif, MAKER_NOTE, makernote);
        // SOI, the comment's marker and length, then the Exif segment's
        // marker, length and identifier come before the TIFF header: 16
        // bytes besides the comment's payload.
        let comment = usize::try_from(FAR - 1024 - 16).unwrap();
        let mut builder = JpegBuilder::new();
        builder
            .segment(0xFE, &vec![0x20; comment])
            .exif(&tiff.build().bytes);
        let built = builder.build();
        let header = built.exif_tiff(0).start;
        assert_eq!(header, FAR - 1024, "{make}");
        assert!(header + MAKER_NOTE_AT >= FAR, "{make}");
        let (bytes, found) = read(&builder);
        // Read without the failure, the same bytes open.
        let mut src = Source::new(Cursor::new(bytes.clone())).unwrap();
        parse(&mut src, &found, &Limits::default()).unwrap();

        let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
        parse(&mut src, &found, &Limits::default())
    }

    /// Checks that the file of [`parsed_when_the_makernote_fails`] with the
    /// MakerNote `block` fails with an `Io` error under IFD0's Make `make`,
    /// which reads the MakerNote, and opens under another maker's, which
    /// does not.
    fn assert_only_its_make_reads(make: &str, block: &[u8]) {
        let Err(err) = parsed_when_the_makernote_fails(make, block) else {
            panic!("{make}: the reader fails");
        };
        assert_eq!(err.kind(), ErrorKind::Io, "{make}: {err:?}");
        let parsed = parsed_when_the_makernote_fails("Canon", block)
            .expect("another make's MakerNote is not read");
        assert_eq!(parsed.metadata.make.as_deref(), Some("Canon"));
    }

    #[test]
    fn a_failing_reader_inside_a_nikon_makernote_is_an_io_error() {
        // The signature, the version, two zero bytes and an inner TIFF
        // header whose MakerNote IFD has no entries.
        let mut block = b"Nikon\0\x02\x11\0\0II\x2A\0\x08\0\0\0".to_vec();
        block.extend_from_slice(&[0; 6]);
        assert_only_its_make_reads("NIKON CORPORATION", &block);
    }

    #[test]
    fn a_failing_reader_inside_a_sony_makernote_is_an_io_error() {
        // The `SONY DSC` header, then an IFD of one entry, FocusMode, one
        // BYTE 3, with no next IFD, little-endian as the file is.
        let mut block = b"SONY DSC \0\0\0\x01\0".to_vec();
        block.extend_from_slice(&[0x1B, 0x20, 0x01, 0, 0x01, 0, 0, 0, 0x03, 0, 0, 0]);
        block.extend_from_slice(&[0; 4]);
        assert_only_its_make_reads("SONY", &block);
    }

    /// `width` by `height`, as a size.
    fn size(width: u32, height: u32) -> Size {
        Size { width, height }
    }

    #[test]
    fn a_frame_of_the_pictures_shape_has_its_shape() {
        for (frame, picture) in [
            // The same size, and a frame twice the picture's.
            (size(8256, 5504), size(8256, 5504)),
            (size(8256, 5504), size(4128, 2752)),
            // A 3:2 frame rounded to 640 by 427: 640 x 4000 and 427 x 6000
            // differ by 2,000, within the longest side, 6,000, which a
            // tolerance of the frame's longer side, 640, would refuse; and
            // the other way round.
            (size(640, 427), size(6000, 4000)),
            (size(6000, 4000), size(640, 427)),
            // A 1:1 window of a Z 9's frame, on a JPEG of its middle size.
            (size(5504, 5504), size(4128, 4128)),
            // 3 x 3 and 2 x 3 differ by 3, exactly the longest side.
            (size(3, 2), size(3, 3)),
        ] {
            assert!(same_shape(frame, picture), "{frame:?}, {picture:?}");
        }
    }

    #[test]
    fn a_frame_of_another_shape_has_not_the_pictures() {
        for (frame, picture) in [
            // A landscape frame on a portrait picture, as on an export
            // turned upright in its pixels or cut to portrait.
            (size(8256, 5504), size(5504, 8256)),
            (size(8256, 5504), size(3648, 5472)),
            // A landscape frame on a square export.
            (size(8256, 5504), size(1742, 1742)),
            // 7 x 4 and 5 x 4 differ by 8, one more than the longest side.
            (size(7, 5), size(4, 4)),
        ] {
            assert!(!same_shape(frame, picture), "{frame:?}, {picture:?}");
        }
    }

    #[test]
    fn the_largest_sides_do_not_overflow() {
        let largest = size(u32::MAX, u32::MAX);
        assert!(same_shape(largest, largest));
        assert!(!same_shape(largest, size(1, u32::MAX)));
    }
}
