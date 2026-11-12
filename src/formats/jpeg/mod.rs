// SPDX-License-Identifier: MIT OR Apache-2.0

//! The JPEG reader: how the crate recognizes a JPEG file, and how it finds
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
//!   ([`mpf::index`]).
//! - **The JPEG of the Nikon MakerNote's PreviewIFD**, in a file whose
//!   MakerNote the reader reads ([below](#the-metadata)), as the D70's
//!   JPEGs hold one of 564 by 372 pixels. The MakerNote reader finds it as
//!   in a NEF ([`nikon::preview_candidate`]): counted from the MakerNote's
//!   own inner header, or, in the headerless layout of the D1 family,
//!   from the Exif segment's TIFF header, and wholly inside the length the
//!   MakerNote declares, which lies inside the Exif segment. A JPEG that
//!   the PreviewIFD places outside that length is not a preview.
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
//! MakerNote its camera settings and its focus point, as in an ARW. An MPF
//! list that does not hold together costs only its images. Only a failure
//! of the reader is an error.
//!
//! # The metadata
//!
//! The metadata is read from the Exif segment's tree by the reader the
//! TIFF-based formats share (`tiff_metadata`), by the same rules as in a
//! raw file, the standard tags first (ADR011): the camera's make and model
//! and the orientation from IFD0, and the serial number, the capture time,
//! the exposure, the camera's settings and the lens from the Exif IFD, with
//! the Interoperability IFD for the color space.
//!
//! Then the MakerNote is read, by IFD0's Make, as in a raw file (ADR014),
//! once, by the reader of the maker the Make names ([`Maker`]); the Make
//! is compared trimmed of spaces at either end and in any case, as the
//! claims of the raw formats compare it.
//!
//! - **When the Make starts with `NIKON`**, as the NEF claim tests it, the
//!   Nikon MakerNote reader (`makernote/nikon/`) reads it as it reads a
//!   NEF's. In the layout every body since the D70 writes, the offsets
//!   count from the MakerNote's own inner header, so it reads the same
//!   inside the Exif segment. In the headerless layout of the D1 family,
//!   they count from the TIFF header of the structure that holds the
//!   MakerNote, as Sony's do, which in a JPEG is the one inside the Exif
//!   segment, as the JPEGs of early Coolpix bodies that write it show,
//!   and the numbers are in that structure's byte order. It gives what it
//!   gives a NEF: the serial number, the ISO, the lens's range and the
//!   capture time's offset from UTC, each only when the standard tags
//!   gave none, so that an Exif value always wins; the camera settings
//!   that only it records, but the raw image size, since a JPEG holds no
//!   raw data, and its white balance setting, finer than Exif's, in place
//!   of Exif's, and its color space only when the Exif IFD holds no
//!   ColorSpace; and the focus point, below.
//! - **When the Make starts with `SONY`**, as the ARW claim first tests it,
//!   the Sony MakerNote reader (`makernote/sony.rs`) reads it as it reads
//!   an ARW's. It gives what it gives an ARW: the camera settings that only
//!   it records, its white balance setting, finer than Exif's, in place of
//!   Exif's, and the focus point, below. Its offsets count from
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
//! scaled onto them, a 1:1 or 16:9 crop's moved into the centered window of
//! the recorded frame that has the JPEG's shape. A JPEG whose MakerNote
//! holds a CropArea gives no point on a crop, since it has no raw size to
//! check CropArea against. A JPEG shot through a DSLR's viewfinder, of a
//! body whose table of points raw-files has, the D600, gives the points its
//! AFInfo2 names placed from the table, as its raw file does (ADR018): in
//! the table's frame, or on a crop in the crop's window, and then only
//! when the JPEG's own size is that window's, so that a crop-mode JPEG at
//! a smaller size gives none. An uncropped JPEG of a size that the table
//! lists as showing a window of the frame, the D600's Medium JPEG, 4512
//! by 3008, gives them in that window, 6010 by 4006, which has the JPEG's
//! shape where the whole frame has not (ADR018).
//!
//! The Sony MakerNote reader gives the point as it gives an ARW's (ADR012):
//! the frame and the center from FocusLocation, or FocusLocation2 without
//! it, the area from FocusFrameSize where it records one, and none in
//! manual focus. Sony's frame is the picture as framed in the camera, so
//! the point is not placed: the ILCE-7M3's JPEG at the camera's M size
//! records its point at (1783, 1804) in a frame of its own 3936 by 2624
//! pixels. The DSLR-A100's MakerNote, of an older layout, holds no
//! FocusLocation, and gives none.
//!
//! Each reader gives its point as a list of one, the primary, or, placed
//! from a table, the points the file names, which the metadata holds as
//! its focus points, with the area mode on a Nikon file. Then the list is
//! kept only when its frame has the picture's aspect ratio, to within one
//! pixel of rounding in either size ([`Size::same_shape`]), in the stored
//! orientation (ADR014). A camera's own JPEG passes, and so does an
//! export scaled from it, except from a JPEG whose points the table
//! places in the window of its size, such as the D600's Medium JPEG: the
//! window is kept for that size alone, so a scaled export takes the whole
//! frame, of another shape, and gives no point, unless it is about 750
//! pixels tall or less, where the rule's rounding passes the whole frame
//! and the point lands under a pixel off. An export cut to another
//! shape, or turned upright in its pixels, which may keep the camera's
//! MakerNote, gives no point. The rule applies to whatever point the
//! MakerNote reader gave, so it cannot refuse a Nikon crop-mode JPEG cut
//! again: its point is placed on the export's own pixels, a window of
//! their shape, and is misplaced if the cut changed the shape (ADR015).
//! Nor can it refuse an export of a D600 JPEG cut or scaled to exactly
//! the Medium size: it is read as a Medium JPEG, its points placed in the
//! Medium window, by far misplaced on a cut (ADR018).

use std::io::{Read, Seek};

use super::makernote::nikon::{self, NIKON, Picture};
use super::makernote::sony::{self, SONY};
use super::tiff_metadata::{self, Standard};
use super::tiff_previews::{self, Candidate};
use super::{FormatSpec, JPEG_UNCLAIMED, Parsed, make_starts_with};
use crate::container::jpeg::{Frame, Segments, mpf};
use crate::container::tiff::{Tree, walk};
use crate::entries::{Directory, Entries};
use crate::error::{Error, Result, skip_damage};
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
/// tags lack, the camera settings and the focus points among them, a
/// recorded position, placed on the picture, or the points a shot through
/// the viewfinder names, placed from the table of the body's points, and
/// the candidate of its preview; when it names Sony, the Sony MakerNote
/// is read, within `limits`, and gives the camera settings that only it
/// records and the focus point, as recorded ([`MakerNote::read`]). The
/// MPF list, read within `limits` as well, gives a candidate for each
/// image after the first. The shared finder checks them all, and the
/// file itself joins them as a preview of its own, with its frame
/// header's size, which is the metadata's image size.
/// The focus points, the primary among them, are kept only when their
/// frame has the picture's shape ([`Size::same_shape`]); see the
/// [module documentation](self). The entries are those of the Exif
/// segment's tree, then those of the MakerNote's directories, Nikon's
/// MakerNote IFD and PreviewIFD or Sony's MakerNote IFD
/// ([`MakerNote::add_entries`]), then the MP Index IFD's.
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
            tiff_metadata::read(src, tree)?,
            tiff_previews::tree_candidates(src, tree)?,
            MakerNote::read(src, tree, limits)?,
        ),
        None => (Standard::default(), Vec::new(), None),
    };
    // The picture's own size, whatever sizes the tree gives, and no raw
    // size: a JPEG holds no raw data. Set before the MakerNote's values
    // are read, so that its reader takes them for the JPEG's: with no raw
    // size, CropArea gives no image size, and a crop is placed on the
    // picture's pixels. Whether the Exif IFD holds a ColorSpace is kept as
    // read, so that the MakerNote's color space never overrules it.
    let standard = Standard {
        metadata: Metadata {
            raw_size: None,
            image_size: Some(size),
            ..standard.metadata
        },
        from_default_crop: false,
        ..standard
    };
    let mut metadata = match &makernote {
        Some(MakerNote::Nikon(makernote)) => {
            candidates.extend(nikon::preview_candidate(makernote));
            nikon::metadata(src, makernote, standard, Picture::Jpeg)?
        }
        Some(MakerNote::Sony(makernote)) => sony::metadata(src, makernote, standard.metadata)?,
        None => standard.metadata,
    };
    metadata.focus_points = metadata
        .focus_points
        .filter(|points| points.frame().same_shape(size));
    // The entries of the Exif segment's tree, walked once, none when it
    // does not walk, as it then gives no metadata; then the MakerNote's;
    // then the MP Index IFD's, which the MPF segment holds apart from it.
    let mut entries = tree.as_ref().map_or_else(Entries::new, Entries::of_tree);
    if let Some(makernote) = &makernote {
        makernote.add_entries(&mut entries);
    }
    let index = match &segments.mpf {
        Some(block) => mpf::index(src, block, limits)?,
        None => None,
    };
    if let Some(index) = index {
        entries.push(Directory::MpIndex, &index.ifd);
        candidates.extend(index.images.into_iter().filter_map(|image| {
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
        size,
        PreviewFormat::Jpeg,
        metadata.orientation,
    ));
    Ok(Parsed {
        previews,
        metadata,
        entries,
    })
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
    /// Sony's: its camera settings and its focus point.
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

    /// Adds the MakerNote's directories to `entries`, as its maker's reader
    /// lists them: Nikon's MakerNote IFD and PreviewIFD, Sony's MakerNote
    /// IFD.
    fn add_entries(&self, entries: &mut Entries) {
        match self {
            MakerNote::Nikon(makernote) => makernote.add_entries(entries),
            MakerNote::Sony(makernote) => makernote.add_entries(entries),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_settings::{CameraSettings, ColorSpace, MeteringMode};
    use crate::container::jpeg::segments;
    use crate::container::tiff::tags::{
        COLOR_SPACE, INTEROP_INDEX, MAKE, MAKER_NOTE, METERING_MODE,
    };
    use crate::entries::Directory;
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
        assert_eq!(
            parsed.metadata.camera.recorded_make.as_deref(),
            Some("Canon")
        );
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

    #[test]
    fn a_jpeg_keeps_the_camera_settings_its_exif_segment_records() {
        // Under each Make, through each way the MakerNote is read: Nikon's,
        // whose IFD holds one entry, its version; Sony's, as the test above
        // builds it; and none, under another maker's Make.
        let mut nikon = b"Nikon\0\x02\x11\0\0II\x2A\0\x08\0\0\0".to_vec();
        nikon.extend_from_slice(&[0x01, 0, 0x01, 0, 0x07, 0, 0, 0, 0x04, 0, 0, 0]);
        nikon.extend_from_slice(b"0211");
        nikon.extend_from_slice(&[0; 4]);
        let mut sony = b"SONY DSC \0\0\0\x01\0".to_vec();
        sony.extend_from_slice(&[0x1B, 0x20, 0x01, 0, 0x01, 0, 0, 0, 0x03, 0, 0, 0]);
        sony.extend_from_slice(&[0; 4]);
        for (make, block) in [
            ("NIKON CORPORATION", &nikon[..]),
            ("SONY", &sony[..]),
            ("Canon", &[][..]),
        ] {
            let mut tiff = TiffBuilder::new(Order::Little);
            let (ifd0, exif, interop) = (tiff.ifd(), tiff.ifd(), tiff.ifd());
            tiff.ascii(ifd0, MAKE, make)
                .exif(ifd0, exif)
                .interop(exif, interop)
                .short(exif, METERING_MODE, &[5])
                .short(exif, COLOR_SPACE, &[65535])
                .ascii(interop, INTEROP_INDEX, "R03");
            if !block.is_empty() {
                let makernote = tiff.blob(block);
                tiff.undefined_blob(exif, MAKER_NOTE, makernote);
            }
            let mut builder = JpegBuilder::new();
            builder.exif(&tiff.build().bytes).frame(640, 480, 0xC0);
            let (bytes, found) = read(&builder);
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let parsed = parse(&mut src, &found, &Limits::default()).unwrap();
            assert_eq!(
                parsed.metadata.camera_settings,
                CameraSettings::default()
                    .with_metering_mode(MeteringMode::MultiSegment)
                    .with_color_space(ColorSpace::AdobeRgb),
                "{make}"
            );
        }
    }

    #[test]
    fn an_exif_color_space_keeps_the_nikon_makernotes_out() {
        // A Nikon MakerNote whose IFD holds one entry, its ColorSpace, one
        // SHORT 2, Adobe RGB. Beside an Exif ColorSpace that is
        // "uncalibrated" with no index, as a program that rewrote the JPEG
        // may leave it, the color space is Exif's: none. Without an Exif
        // ColorSpace, it is the MakerNote's.
        let mut nikon = b"Nikon\0\x02\x11\0\0II\x2A\0\x08\0\0\0".to_vec();
        nikon.extend_from_slice(&[0x01, 0, 0x1E, 0, 0x03, 0, 0x01, 0, 0, 0, 0x02, 0, 0, 0]);
        nikon.extend_from_slice(&[0; 4]);
        for (exif_color_space, expected) in [(true, None), (false, Some(ColorSpace::AdobeRgb))] {
            let mut tiff = TiffBuilder::new(Order::Little);
            let (ifd0, exif) = (tiff.ifd(), tiff.ifd());
            tiff.ascii(ifd0, MAKE, "NIKON CORPORATION").exif(ifd0, exif);
            if exif_color_space {
                tiff.short(exif, COLOR_SPACE, &[65535]);
            }
            let makernote = tiff.blob(&nikon);
            tiff.undefined_blob(exif, MAKER_NOTE, makernote);
            let mut builder = JpegBuilder::new();
            builder.exif(&tiff.build().bytes).frame(640, 480, 0xC0);
            let (bytes, found) = read(&builder);
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let parsed = parse(&mut src, &found, &Limits::default()).unwrap();
            assert_eq!(
                parsed.metadata.camera_settings.color_space, expected,
                "Exif ColorSpace {exif_color_space}"
            );
        }
    }

    #[test]
    fn the_entries_are_those_of_the_exif_tree_or_none() {
        let mut tiff = TiffBuilder::new(Order::Big);
        let (ifd0, exif, interop, gps) = (tiff.ifd(), tiff.ifd(), tiff.ifd(), tiff.ifd());
        tiff.ascii(ifd0, MAKE, "Canon")
            .exif(ifd0, exif)
            .gps(ifd0, gps)
            .interop(exif, interop)
            .short(exif, METERING_MODE, &[5])
            .ascii(interop, INTEROP_INDEX, "R98")
            .byte(gps, 0x0000, &[2, 3, 0, 0]);
        let exif_bytes = tiff.build().bytes;
        // Bytes that are not a TIFF header: the segment does not walk.
        let not_tiff = b"no TIFF header here".to_vec();
        for (segment, expected) in [
            (
                Some(&exif_bytes),
                &[
                    Directory::Ifd(0),
                    Directory::Exif,
                    Directory::Interop,
                    Directory::Gps,
                ][..],
            ),
            (Some(&not_tiff), &[]),
            (None, &[]),
        ] {
            let mut builder = JpegBuilder::new();
            if let Some(segment) = segment {
                builder.exif(segment);
            }
            builder.frame(640, 480, 0xC0);
            let (bytes, found) = read(&builder);
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let parsed = parse(&mut src, &found, &Limits::default()).unwrap();
            let directories: Vec<Directory> = parsed.entries.directories().to_vec();
            assert_eq!(directories, expected, "{segment:?}");
            assert_eq!(parsed.entries.is_empty(), expected.is_empty());
        }
    }

    #[test]
    fn the_mp_index_is_listed_after_the_exif_tree_with_or_without_it() {
        let mut tiff = TiffBuilder::new(Order::Little);
        let (ifd0, exif) = (tiff.ifd(), tiff.ifd());
        tiff.ascii(ifd0, MAKE, "Canon").exif(ifd0, exif);
        tiff.short(exif, METERING_MODE, &[5]);
        let exif_bytes = tiff.build().bytes;
        for (with_exif, expected) in [
            (
                true,
                &[Directory::Ifd(0), Directory::Exif, Directory::MpIndex][..],
            ),
            (false, &[Directory::MpIndex]),
        ] {
            let mut builder = JpegBuilder::new();
            if with_exif {
                builder.exif(&exif_bytes);
            }
            builder.mpf(Order::Big).mpf_image(640, 424, &[], 1);
            let (bytes, found) = read(&builder);
            let mut src = Source::new(Cursor::new(bytes)).unwrap();
            let parsed = parse(&mut src, &found, &Limits::default()).unwrap();
            let directories: Vec<Directory> = parsed.entries.directories().to_vec();
            assert_eq!(directories, expected, "Exif {with_exif}");
            // MPFVersion, NumberOfImages and MPEntry.
            assert_eq!(parsed.entries.in_directory(Directory::MpIndex).len(), 3);
            assert!(parsed.entries.entry(Directory::MpIndex, 0xB002).is_some());
            // The large thumbnail is still a preview.
            assert_eq!(parsed.previews.len(), 2, "Exif {with_exif}");
        }
    }

    /// The MakerNote layouts the JPEG tests build into an Exif segment.
    #[derive(Clone, Copy, Debug)]
    enum Layout {
        /// Nikon's, with a TIFF header of its own after `Nikon\0`, its
        /// version and two NULs: a MakerNote IFD whose 0x0011 leads to a
        /// PreviewIFD.
        NikonEmbedded,
        /// Nikon's of the early Coolpix bodies: a bare MakerNote IFD whose
        /// offsets count from the segment's TIFF header, its block running
        /// through its PreviewIFD.
        NikonHeaderless,
        /// Sony's, its IFD after the `SONY DSC` header, its offsets counted
        /// from the segment's TIFF header.
        Sony,
    }

    /// The four SHORTs the tests' MakerNote IFDs store out of line under
    /// tag 0x0102, so that reading them back takes the right base.
    const SHORTS: [u16; 4] = [10, 20, 30, 40];

    /// The bytes of an Exif segment's TIFF structure in byte order
    /// `order`, under IFD0's Make `make`, whose Exif IFD holds a MakerNote
    /// of layout `layout`: an IFD of a SHORT under 0x0101 and [`SHORTS`]
    /// under 0x0102, with, for Nikon, a PreviewIFD of one SHORT.
    fn exif_with_makernote(order: Order, make: &str, layout: Layout) -> Vec<u8> {
        let mut tiff = TiffBuilder::new(order);
        let (ifd0, exif) = (tiff.ifd(), tiff.ifd());
        tiff.ascii(ifd0, MAKE, make).exif(ifd0, exif);
        tiff.short(exif, METERING_MODE, &[5]);
        match layout {
            Layout::NikonEmbedded => {
                let mut inner = TiffBuilder::new(order);
                let (makernote, preview) = (inner.ifd(), inner.ifd());
                inner
                    .short(makernote, 0x0101, &[7])
                    .short(makernote, 0x0102, &SHORTS)
                    .ifd_pointer(makernote, 0x0011, preview)
                    .short(preview, 0x0103, &[6]);
                let mut block = b"Nikon\0\x02\x11\0\0".to_vec();
                block.extend_from_slice(&inner.build().bytes);
                let blob = tiff.blob(&block);
                tiff.undefined_blob(exif, MAKER_NOTE, blob);
            }
            Layout::NikonHeaderless => {
                let (makernote, preview) = (tiff.ifd(), tiff.ifd());
                tiff.short(makernote, 0x0101, &[7])
                    .short(makernote, 0x0102, &SHORTS)
                    .ifd_pointer(makernote, 0x0011, preview)
                    .short(preview, 0x0103, &[6]);
                let end = tiff.blob(&[0; 4]);
                tiff.block_through(makernote, end)
                    .undefined_ifd(exif, MAKER_NOTE, makernote);
            }
            Layout::Sony => {
                let makernote = tiff.ifd();
                tiff.prefix(makernote, b"SONY DSC \0\0\0")
                    .short(makernote, 0x0101, &[7])
                    .short(makernote, 0x0102, &SHORTS);
                tiff.undefined_ifd(exif, MAKER_NOTE, makernote);
            }
        }
        tiff.build().bytes
    }

    #[test]
    fn a_makernotes_directories_come_after_the_exif_tree_and_before_the_mp_index() {
        let nikon = [Directory::MakerNote, Directory::MakerNoteSub(0x0011)];
        for order in [Order::Little, Order::Big] {
            for (make, layout, makers) in [
                ("NIKON CORPORATION", Layout::NikonEmbedded, &nikon[..]),
                ("NIKON", Layout::NikonHeaderless, &nikon[..]),
                ("SONY", Layout::Sony, &[Directory::MakerNote][..]),
                // Another maker's Make: the MakerNote is the Exif entry's
                // bytes alone.
                ("Canon", Layout::Sony, &[][..]),
            ] {
                let mut builder = JpegBuilder::new();
                // A comment first, so that the segment's TIFF header lies
                // well into the file.
                builder
                    .segment(0xFE, &[0x20; 300])
                    .exif(&exif_with_makernote(order, make, layout))
                    .mpf(Order::Big)
                    .mpf_image(640, 424, &[], 1);
                let (bytes, found) = read(&builder);
                let mut src = Source::new(Cursor::new(bytes)).unwrap();
                let parsed = parse(&mut src, &found, &Limits::default()).unwrap();
                let entries = &parsed.entries;
                let mut expected = vec![Directory::Ifd(0), Directory::Exif];
                expected.extend_from_slice(makers);
                expected.push(Directory::MpIndex);
                let what = format!("{order:?}, {make}, {layout:?}");
                assert_eq!(entries.directories(), expected, "{what}");
                if makers.is_empty() {
                    continue;
                }
                let tags: Vec<u16> = entries
                    .in_directory(Directory::MakerNote)
                    .iter()
                    .map(crate::Entry::tag)
                    .collect();
                let expected_tags: &[u16] = match layout {
                    Layout::Sony => &[0x0101, 0x0102],
                    _ => &[0x0011, 0x0101, 0x0102],
                };
                assert_eq!(tags, expected_tags, "{what}");
                // Read from the base its layout counts from: the inner
                // header, or the segment's TIFF header.
                let shorts = entries.entry(Directory::MakerNote, 0x0102).unwrap();
                assert_eq!(
                    shorts.read(&mut src, 64).unwrap(),
                    crate::Value::Short(SHORTS.to_vec()),
                    "{what}"
                );
                if makers.len() == 2 {
                    let preview = entries.entry(Directory::MakerNoteSub(0x0011), 0x0103);
                    let value = preview.unwrap().read(&mut src, 64).unwrap();
                    assert_eq!(value, crate::Value::Short(vec![6]), "{what}");
                }
            }
        }
    }
}
