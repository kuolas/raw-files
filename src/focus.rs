// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the camera's autofocus aimed.
//!
//! A viewer can mark on a preview the place where the camera focused. The
//! camera records it as the centre of the focus area, the area's size when
//! it records one, the frame the two are measured in, which has the
//! previews' framing, and, when the file says, whether focus was confirmed.
//! [`Focus`] holds them, with where the point came from, a
//! [`FocusOrigin`], and a caller's tests build one through its checked
//! constructors.
//!
//! The camera may use several autofocus points for one picture, one of
//! them the primary. [`FocusPoints`] holds them all, each a
//! [`FocusPoint`], in one frame, with the primary marked, the area mode
//! the camera was set to, an [`AfAreaMode`], and where they came from.
//!
//! [`Metadata::focus`](crate::Metadata::focus) gives the focus point of a
//! file, if it records one,
//! [`Metadata::focus_points`](crate::Metadata::focus_points) the points
//! it used, and [`Metadata::focus_layout`](crate::Metadata::focus_layout)
//! every point of the body, a [`FocusLayout`] of [`LayoutPoint`]s, where
//! raw-files has its table of them.

use std::fmt;

use crate::geometry::{Point, Size};

/// Where the camera's autofocus aimed, as the camera recorded it: the
/// centre of the focus area, the area's size when the file records one,
/// the frame the two are measured in, whether the camera confirmed focus,
/// when the file says, and where the point came from.
///
/// It is the camera's record, never an estimate from the pixels, and not
/// proof of where the picture is sharpest: a body that focuses in front of
/// or behind the subject, or a subject that moved, puts the sharpest area
/// elsewhere.
///
/// # Coordinates
///
/// The centre is a [`Point`], and the frame and the area are [`Size`]s,
/// all in pixels and in the orientation of the stored image, like every
/// size and preview the library reports: a viewer that rotates a preview
/// to show it upright rotates the point with it.
///
/// The centre is a distance from the frame's left and top edges, so the
/// middle of the frame is at (width / 2, height / 2), as the cameras
/// record it: a Nikon Coolpix A focusing at the middle of its 4928 × 3264
/// frame records (2464, 1632), as on the raw.pixls.us sample 2238. An area
/// `w` × `h` centred at (x, y) spans from x - w / 2 to x + w / 2 across
/// and from y - h / 2 to y + h / 2 down. The values are whole numbers, as
/// the cameras store them; to place the focus in fractions of the frame,
/// divide x and the area's width by the [`frame`](Self::frame)'s width,
/// and y and the area's height by its height.
///
/// The frame has the framing of the previews, at whatever scale the file
/// records. To place the focus on a preview, scale x and the width by the
/// preview's width over the frame's, and y and the height by the heights;
/// [`scaled_to`](Self::scaled_to) does it. Do not assume that the frame
/// equals [`Metadata::image_size`](crate::Metadata::image_size). They
/// differ where the camera's framing differs from the default crop, as on a
/// Sony picture taken at another aspect ratio than the sensor's; where the
/// stored raw is a smaller copy of the camera's output, as on a small raw;
/// and on the older Nikon bodies that record no crop area, whose image size
/// includes the border pixels.
///
/// # Which files record one
///
/// Nikon mirrorless bodies, Nikon DSLRs in live view, and Sony bodies from
/// the ILCE-7RM2 on record a position. A DSLR shot through the viewfinder
/// and older bodies record only the number of an autofocus point, and a
/// Nikon mirrorless body in some auto-area modes only a mask of the points
/// it used, which the library does not map to pixels; a shot in manual
/// focus records none that means anything. Those files give no `Focus`.
///
/// On Nikon files, [`Metadata::focus`](crate::Metadata::focus) is read from
/// the MakerNote's AFInfo2 block, as the camera recorded it but placed
/// inside the cropped picture on a crop, and is `None` for a manual-focus
/// shot.
/// On Sony files it is read from the MakerNote's FocusLocation, on the
/// bodies from the ILCE-7RM2 on, in the frame the camera framed the
/// picture in: the picture at an aspect-ratio setting, and, on a medium or
/// small raw, the full size or the stored size, as the body chooses. It
/// is `None` for a manual-focus shot and for the bodies that record only
/// a point's number, and its [`in_focus`](Self::in_focus) is always
/// `None`. The area's size is read from the MakerNote's FocusFrameSize,
/// which the bodies record only from the ILCE-7SM3 on, so on the bodies
/// before it [`size`](Self::size) is `None`.
///
/// The JPEG files ([`Format::Jpeg`](crate::Format::Jpeg)) of those cameras
/// record one too, given under the shape rule that
/// [`Metadata::focus`](crate::Metadata::focus) states.
///
/// # Equality
///
/// Two values are equal when every value is the same as stored, the
/// [`origin`](Self::origin) included: a point the camera recorded and one
/// placed from a table at the same place are not equal. There is no
/// order: no order of focus records means anything, and sorting them
/// would suggest one. There is no `Default` either, since there is no
/// empty focus: a file that records none gives `None`.
///
/// # Examples
///
/// A Nikon Z 9's record, built a step at a time, the centre first, then
/// the area and whether focus was confirmed, and placed on a 1920 × 1280
/// preview:
///
/// ```
/// use raw_files::{Focus, FocusOrigin, Point, Size};
///
/// # fn main() {
/// #     example().expect("every step accepts the Z 9's values");
/// # }
/// # fn example() -> Option<()> {
/// let frame = Size { width: 8256, height: 5504 };
/// let focus = Focus::new(frame, Point { x: 4905, y: 2461 })?
///     .with_size(Size { width: 291, height: 323 })?
///     .with_in_focus(true);
/// let on_preview = focus.scaled_to(Size { width: 1920, height: 1280 })?;
/// assert_eq!(on_preview.center(), Point { x: 1141, y: 572 });
/// assert_eq!(on_preview.size(), Some(Size { width: 68, height: 75 }));
/// assert_eq!(on_preview.frame(), Size { width: 1920, height: 1280 });
/// assert_eq!(on_preview.in_focus(), Some(true));
/// assert_eq!(on_preview.origin(), FocusOrigin::Recorded);
/// # Some(())
/// # }
/// ```
///
/// Where the camera focused on a file's largest preview:
///
/// ```no_run
/// use raw_files::RawFile;
///
/// # fn main() -> raw_files::Result<()> {
/// let file = RawFile::open("photo.nef")?;
/// let focus = file.metadata().focus();
/// if let (Some(focus), Some(preview)) = (focus, file.largest_preview()) {
///     if let Some(focus) = focus.scaled_to(preview.size()) {
///         let centre = focus.center();
///         println!("focused at {centre} on the preview");
///         if let Some(size) = focus.size() {
///             println!("on an area of {size} pixels");
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Focus {
    frame: Size,
    center: Point,
    size: Option<Size>,
    in_focus: Option<bool>,
    origin: FocusOrigin,
}

impl Focus {
    /// The focus centred at `center`, measured in `frame`, with no area,
    /// nothing said about whether the camera confirmed focus, and the
    /// origin [`Recorded`](FocusOrigin::Recorded); `None` when the centre
    /// cannot lie on the frame. [`with_size`](Self::with_size) adds the
    /// area, [`with_in_focus`](Self::with_in_focus) whether focus was
    /// confirmed, and [`with_origin`](Self::with_origin) sets another
    /// origin.
    ///
    /// This is how the readers start every focus they return, and how a
    /// caller starts one for its tests. It gives `None` when:
    ///
    /// - either side of `frame` is zero;
    /// - `center` lies outside the frame: its x is more than the frame's
    ///   width, or its y more than its height. The edges are on the frame,
    ///   so a centre at x = width, on its right edge, is accepted.
    #[must_use]
    pub fn new(frame: Size, center: Point) -> Option<Self> {
        if frame.width == 0 || frame.height == 0 {
            return None;
        }
        let center_on_frame = center.x <= frame.width && center.y <= frame.height;
        center_on_frame.then_some(Self {
            frame,
            center,
            size: None,
            in_focus: None,
            origin: FocusOrigin::Recorded,
        })
    }

    /// This focus with an area of `size` around its centre, in place of
    /// any area it had; `None` when the area cannot lie on the frame
    /// around the centre, which is when:
    ///
    /// - either side of `size` is zero;
    /// - the area does not fit inside the frame around its centre. An area
    ///   `w` pixels wide centred at x spans from x - w / 2 to x + w / 2,
    ///   which must lie between 0 and the frame's width; in whole numbers,
    ///   `w` is at most 2 × x and at most 2 × (width - x). The same holds
    ///   down, for the height, y and the frame's height. So an area 10
    ///   pixels wide fits a centre from x = 5 to x = width - 5, and one 11
    ///   pixels wide, whose half is 5.5, from x = 6 to x = width - 6.
    ///
    /// A focus is `Copy`, and a refused area leaves the one it was offered
    /// to as it was, so whoever builds a focus chooses what an area that
    /// cannot be used costs. `Focus::new(frame, center)?.with_size(size)?`
    /// gives no focus at all, and `focus.with_size(size).unwrap_or(focus)`
    /// keeps the point without an area. The readers choose by what the
    /// file means by an area with a zero side: no focus where the format
    /// writes it to say that it recorded no position, and the point alone
    /// where it writes it to say only that it recorded no size.
    #[must_use]
    pub fn with_size(self, size: Size) -> Option<Self> {
        let size_fits = size.width != 0
            && size.height != 0
            && fits(self.center.x, size.width, self.frame.width)
            && fits(self.center.y, size.height, self.frame.height);
        size_fits.then_some(Self {
            size: Some(size),
            ..self
        })
    }

    /// This focus with whether the camera confirmed focus, in place of
    /// what it had: `true` when the file records that the camera found
    /// focus, and `false` when it records a miss.
    ///
    /// A focus built without it says nothing, and its
    /// [`in_focus`](Self::in_focus) is `None`, as for a file that does not
    /// say. Once set, it can be replaced but not taken back to `None`.
    #[must_use]
    pub fn with_in_focus(self, in_focus: bool) -> Self {
        Self {
            in_focus: Some(in_focus),
            ..self
        }
    }

    /// This focus with where it came from, in place of what it had:
    /// [`Table`](FocusOrigin::Table) for a point placed from a table of the
    /// body's autofocus points, and [`Recorded`](FocusOrigin::Recorded)
    /// for a position the camera measured and wrote.
    ///
    /// A focus built without it is `Recorded`, as [`new`](Self::new)
    /// gives it.
    #[must_use]
    pub fn with_origin(self, origin: FocusOrigin) -> Self {
        Self { origin, ..self }
    }

    /// The size of the frame the centre and the area are measured in. It
    /// never has a zero side.
    pub fn frame(&self) -> Size {
        self.frame
    }

    /// The centre of the focus area, as its distances from the left and top
    /// edges of the frame. It always lies within the frame, from 0 to the
    /// frame's width across and from 0 to its height down, the edges
    /// included.
    ///
    /// A caller reading the pixel at the centre takes the pixel at column x
    /// and row y; for a centre on the right edge, x = width, that is the
    /// last column, x - 1, and likewise the last row, y - 1, for a centre
    /// on the bottom edge, y = height.
    pub fn center(&self) -> Point {
        self.center
    }

    /// The width and height of the focus area, or `None` when the file
    /// records no size.
    ///
    /// It never has a zero side, and it fits inside the frame around the
    /// centre: across, from x - width / 2 to x + width / 2 lies between 0
    /// and the frame's width, so the width is at most 2 × x and at most
    /// 2 × (frame width - x); down, the same holds for the height, y and
    /// the frame's height.
    pub fn size(&self) -> Option<Size> {
        self.size
    }

    /// Whether the camera confirmed focus: `Some(true)` when it records that
    /// it found focus, `Some(false)` when it records a miss, and `None` when
    /// the file does not say.
    pub fn in_focus(&self) -> Option<bool> {
        self.in_focus
    }

    /// Where this point came from; [`Recorded`](FocusOrigin::Recorded)
    /// unless [`with_origin`](Self::with_origin) set another. The readers
    /// give `Recorded` for every position a camera wrote.
    pub fn origin(&self) -> FocusOrigin {
        self.origin
    }

    /// The same focus measured in `frame`, such as a preview's size: `None`
    /// when either side of `frame` is zero, and otherwise always a focus,
    /// whose frame is `frame` and whose [`in_focus`](Self::in_focus) is
    /// this one's. It keeps this one's [`origin`](Self::origin) too.
    ///
    /// The centre's x and the area's width are multiplied by the new width
    /// over the old, and y and the height by the heights, each rounded to
    /// the nearest pixel, a half rounding up, so the centre stays within the
    /// new frame, from 0 to its width and height. An area that rounds to
    /// nothing keeps a side of one pixel.
    ///
    /// Rounding can leave the area too large to fit around the centre.
    /// The centre is kept where it rounded to, and the area is trimmed on
    /// that axis, by as little as it takes, to fit by the rule of
    /// [`with_size`](Self::with_size). A centre on an edge of the frame
    /// leaves no room for an area on that axis, and the size becomes
    /// `None`; on a frame only a few pixels across, rounding often puts the
    /// centre there.
    ///
    /// Each call rounds to whole pixels. To place the focus on several
    /// frames, scale it from the focus as read each time, not from one
    /// already scaled, so that it is not rounded twice.
    #[must_use]
    pub fn scaled_to(&self, frame: Size) -> Option<Self> {
        if frame.width == 0 || frame.height == 0 {
            return None;
        }
        let (center, size) = scaled_point(self.center, self.size, self.frame, frame);
        // The scaled centre lies on the frame, which has no zero side, so
        // this never gives `None`.
        let mut focus = Self::new(frame, center)?.with_origin(self.origin);
        if let Some(area) = size {
            // The scaled area fits around the centre, so `with_size` never
            // refuses it; were it to, the focus would keep its point
            // without an area.
            focus = focus.with_size(area).unwrap_or(focus);
        }
        if let Some(in_focus) = self.in_focus {
            focus = focus.with_in_focus(in_focus);
        }
        Some(focus)
    }
}

/// Where a focus point came from: a position the camera measured, or a
/// point the camera named by number that raw-files placed from a table.
///
/// A viewer or a culler reads it to know what a [`Focus`] means and how
/// far it may lie from where the camera focused. A recorded position is
/// the camera's own measurement. A point placed from a table is where the
/// viewfinder's point lies on the picture, measured by the project on
/// frames shot with that body, to a few pixels. The two differ in error
/// and in meaning, so every point says which it is, as the
/// [decision to place focus points from a table](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)
/// records.
///
/// `FocusOrigin` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Recorded" and
/// "Table".
///
/// # Examples
///
/// ```
/// use raw_files::FocusOrigin;
///
/// assert_eq!(FocusOrigin::Recorded.name(), "Recorded");
/// assert_eq!(FocusOrigin::Table.to_string(), "Table");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FocusOrigin {
    /// A position the camera measured and wrote: a Nikon mirrorless
    /// body's, a Nikon DSLR's in live view, a Sony body's with a
    /// FocusLocation.
    Recorded,
    /// A point the camera named by number, placed by raw-files from its
    /// table of the body's autofocus points, measured on frames shot with
    /// that body and read in the maker's software: an estimate of where
    /// the viewfinder's point lies on the picture, not a position the
    /// camera measured. The
    /// [format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/autofocus-points.md)
    /// record each body's table and its tolerance, a few pixels.
    Table,
}

impl FocusOrigin {
    /// The origin's name, for an information panel, such as "Table"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            FocusOrigin::Recorded => "Recorded",
            FocusOrigin::Table => "Table",
        }
    }
}

impl fmt::Display for FocusOrigin {
    /// Writes [`name`](FocusOrigin::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the camera chose its autofocus points for a picture: the area mode
/// the photographer set, in words that fit any maker.
///
/// A viewer labels the points of [`FocusPoints`] with it: one point the
/// photographer placed, a placed point that its neighbours stood in for
/// while the subject left it, points the camera chose itself, or a subject
/// the camera followed across the frame. A maker's code for a mode is named
/// only once a file shot in a known mode shows what it means, as the codes
/// of the camera settings are under the
/// [decision to read the metadata from the standard tags](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr011-metadata-from-standard-tags.md),
/// and any other code gives `None`;
/// [`Metadata::focus_points`](crate::Metadata::focus_points) says which
/// codes each reader names.
///
/// `AfAreaMode` is `#[non_exhaustive]`, so match on it with a wildcard
/// arm. It orders as its variants are listed, and a new variant is added
/// after the existing ones, so an order once observed holds. Its
/// [`name`](Self::name)s, which `Display` writes too, are "Single point",
/// "Dynamic area", "Auto area" and "Tracking".
///
/// # Examples
///
/// ```
/// use raw_files::AfAreaMode;
///
/// assert_eq!(AfAreaMode::Single.name(), "Single point");
/// assert_eq!(AfAreaMode::Auto.to_string(), "Auto area");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AfAreaMode {
    /// One point, which the photographer placed: Nikon's single-point AF.
    Single,
    /// A point the photographer placed, with the points around it standing
    /// in for it when the subject briefly leaves it: Nikon's dynamic-area
    /// AF.
    Dynamic,
    /// Points the camera chose itself: Nikon's auto-area AF.
    Auto,
    /// A subject the camera followed from the point where focus started:
    /// Nikon's 3D-tracking.
    Tracking,
}

impl AfAreaMode {
    /// The mode's name, for an information panel, such as "Auto area"; the
    /// type's docs list them all.
    pub fn name(self) -> &'static str {
        match self {
            AfAreaMode::Single => "Single point",
            AfAreaMode::Dynamic => "Dynamic area",
            AfAreaMode::Auto => "Auto area",
            AfAreaMode::Tracking => "Tracking",
        }
    }
}

impl fmt::Display for AfAreaMode {
    /// Writes [`name`](AfAreaMode::name).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One autofocus point of a picture, as a [`FocusPoints`] holds it: its
/// number, when the file gives one, the centre of its area, the area's size
/// when the file records one, and whether the camera confirmed focus
/// there, when the file says.
///
/// The number is the camera's own name for the point, such as one of 1 to
/// 39 on a body with 39 points; a position the camera measured and wrote
/// has none. The centre and the size are in pixels of the frame of the
/// list that holds the point, by the rules of [`Focus`]'s coordinates. A
/// point alone has no frame, so it is checked against one when it is
/// added to a list, by [`FocusPoints::with_point`], not when it is built.
///
/// Two points are equal when every value is the same as stored. There is
/// no order, as there is none for [`Focus`], and no `Default`, since a
/// point has a centre.
///
/// # Examples
///
/// A point built a step at a time, and read back:
///
/// ```
/// use raw_files::{FocusPoint, Point, Size};
///
/// let point = FocusPoint::new(Point { x: 3008, y: 2008 })
///     .with_number(21)
///     .with_size(Size { width: 164, height: 164 })
///     .with_in_focus(true);
/// assert_eq!(point.number(), Some(21));
/// assert_eq!(point.center(), Point { x: 3008, y: 2008 });
/// assert_eq!(point.size(), Some(Size { width: 164, height: 164 }));
/// assert_eq!(point.in_focus(), Some(true));
///
/// let bare = FocusPoint::new(Point { x: 3008, y: 2008 });
/// assert_eq!((bare.number(), bare.size(), bare.in_focus()), (None, None, None));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FocusPoint {
    number: Option<u8>,
    center: Point,
    size: Option<Size>,
    in_focus: Option<bool>,
}

impl FocusPoint {
    /// The point centred at `center`, with no number, no area and nothing
    /// said about whether the camera confirmed focus there.
    /// [`with_number`](Self::with_number),
    /// [`with_size`](Self::with_size) and
    /// [`with_in_focus`](Self::with_in_focus) add them.
    ///
    /// Any centre is accepted: [`FocusPoints::with_point`] checks it
    /// against the list's frame.
    pub fn new(center: Point) -> Self {
        Self {
            number: None,
            center,
            size: None,
            in_focus: None,
        }
    }

    /// This point with the camera's number for it, in place of any it had;
    /// [`number`](Self::number) reads it.
    #[must_use]
    pub fn with_number(self, number: u8) -> Self {
        Self {
            number: Some(number),
            ..self
        }
    }

    /// This point with an area of `size` around its centre, in place of any
    /// it had; [`size`](Self::size) reads it.
    ///
    /// The size is stored as given: [`FocusPoints::with_point`] refuses a
    /// point whose area has a zero side or does not fit around its centre
    /// in the list's frame, by the rule of [`Focus::with_size`].
    #[must_use]
    pub fn with_size(self, size: Size) -> Self {
        Self {
            size: Some(size),
            ..self
        }
    }

    /// This point with whether the camera confirmed focus there, in place
    /// of what it had: `true` when the file records that the camera found
    /// focus, and `false` when it records a miss;
    /// [`in_focus`](Self::in_focus) reads it.
    #[must_use]
    pub fn with_in_focus(self, in_focus: bool) -> Self {
        Self {
            in_focus: Some(in_focus),
            ..self
        }
    }

    /// The camera's number for the point, or `None` when the file gives
    /// none, as for a position the camera measured.
    pub fn number(&self) -> Option<u8> {
        self.number
    }

    /// The centre of the point's area, as its distances from the left and
    /// top edges of the frame of the list that holds it.
    pub fn center(&self) -> Point {
        self.center
    }

    /// The width and height of the point's area, or `None` when the file
    /// records no size.
    pub fn size(&self) -> Option<Size> {
        self.size
    }

    /// Whether the camera confirmed focus at this point: `Some(true)` when
    /// it records that it found focus, `Some(false)` when it records a
    /// miss, and `None` when the file does not say.
    pub fn in_focus(&self) -> Option<bool> {
        self.in_focus
    }
}

/// The autofocus points the camera used for a picture: every point, each a
/// [`FocusPoint`], in the one frame they are all measured in; which of them
/// is the primary, the point [`Metadata::focus`](crate::Metadata::focus)
/// reports; the area mode the camera was set to, when the file names it;
/// and where the points came from.
///
/// A viewer draws the points over a preview, marks the primary and labels
/// them with the [`AfAreaMode`]; a caller that wants only one point reads
/// [`Metadata::focus`](crate::Metadata::focus) instead. The frame and the
/// points follow the rules of [`Focus`]: the frame has the framing of the
/// previews and no zero side, every centre lies on the frame, its edges
/// included, and every area fits inside the frame around its centre.
/// [`scaled_to`](Self::scaled_to) places the points on a preview. They are
/// in the order they were added, and the [`origin`](Self::origin) is the
/// whole list's.
///
/// A list has one point for a recorded position: the readers give the
/// position the camera recorded as a list of that one point, its primary,
/// with no number and the origin [`Recorded`](FocusOrigin::Recorded).
///
/// # Equality
///
/// Two lists are equal when every value is the same as stored: the frame,
/// the points in order, the primary, the area mode and the origin. There
/// is no order, as there is none for [`Focus`]. There is no `Default`
/// either, since a list has a frame: a file that records no point gives
/// `None`, and a list built for a test starts from its frame and its
/// origin ([`new`](Self::new)).
///
/// # Examples
///
/// Two points in a 6016 × 4016 frame, the second the primary, in auto
/// area, placed on a 1504 × 1004 preview:
///
/// ```
/// use raw_files::{AfAreaMode, FocusOrigin, FocusPoint, FocusPoints, Point, Size};
///
/// # fn main() {
/// #     example().expect("every step accepts these values");
/// # }
/// # fn example() -> Option<()> {
/// let frame = Size { width: 6016, height: 4016 };
/// let area = Size { width: 164, height: 164 };
/// let points = FocusPoints::new(frame, FocusOrigin::Recorded)?
///     .with_point(FocusPoint::new(Point { x: 2008, y: 1340 }).with_size(area))?
///     .with_point(
///         FocusPoint::new(Point { x: 3008, y: 2008 })
///             .with_size(area)
///             .with_in_focus(true),
///     )?
///     .with_primary(1)?
///     .with_area_mode(AfAreaMode::Auto);
///
/// let on_preview = points.scaled_to(Size { width: 1504, height: 1004 })?;
/// let centres: Vec<Point> = on_preview.points().iter().map(FocusPoint::center).collect();
/// assert_eq!(centres, [Point { x: 502, y: 335 }, Point { x: 752, y: 502 }]);
/// assert_eq!(
///     on_preview.primary().map(FocusPoint::center),
///     Some(Point { x: 752, y: 502 })
/// );
/// assert_eq!(on_preview.area_mode(), Some(AfAreaMode::Auto));
/// assert_eq!(on_preview.origin(), FocusOrigin::Recorded);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FocusPoints {
    frame: Size,
    points: Vec<FocusPoint>,
    primary: Option<usize>,
    area_mode: Option<AfAreaMode>,
    origin: FocusOrigin,
}

impl FocusPoints {
    /// An empty list of points measured in `frame`, which came from
    /// `origin`, with no primary and no area mode; `None` when either side
    /// of `frame` is zero. [`with_point`](Self::with_point) adds the
    /// points, [`with_primary`](Self::with_primary) marks the primary and
    /// [`with_area_mode`](Self::with_area_mode) sets the area mode.
    ///
    /// This is how a caller starts a list for its tests. A file whose
    /// list would be empty gives none instead.
    #[must_use]
    pub fn new(frame: Size, origin: FocusOrigin) -> Option<Self> {
        if frame.width == 0 || frame.height == 0 {
            return None;
        }
        Some(Self {
            frame,
            points: Vec::new(),
            primary: None,
            area_mode: None,
            origin,
        })
    }

    /// This list with `point` added after the others; `None` when the
    /// point cannot lie on the frame, which is when:
    ///
    /// - its centre lies outside the frame, by the rule of [`Focus::new`]:
    ///   its x is more than the frame's width, or its y more than its
    ///   height;
    /// - it has an area, and the area has a zero side or does not fit
    ///   inside the frame around the centre, by the rule of
    ///   [`Focus::with_size`].
    ///
    /// The list is taken by value, so a refused point costs the whole list:
    /// `None` is the whole answer, and a caller that wants to keep the list
    /// without the point clones it first.
    #[must_use]
    pub fn with_point(mut self, point: FocusPoint) -> Option<Self> {
        if !lies_on(self.frame, point.center, point.size) {
            return None;
        }
        self.points.push(point);
        Some(self)
    }

    /// This list with the point at `index`, counted from 0 in the order the
    /// points were added, marked as the primary, the one
    /// [`Metadata::focus`](crate::Metadata::focus) reports, in place of any
    /// marked before; `None` when `index` is not that of a point added so far.
    #[must_use]
    pub fn with_primary(mut self, index: usize) -> Option<Self> {
        if index >= self.points.len() {
            return None;
        }
        self.primary = Some(index);
        Some(self)
    }

    /// This list with the area mode `mode`, in place of any it had;
    /// [`area_mode`](Self::area_mode) reads it.
    #[must_use]
    pub fn with_area_mode(mut self, mode: AfAreaMode) -> Self {
        self.area_mode = Some(mode);
        self
    }

    /// The size of the frame every point is measured in. It never has a
    /// zero side.
    pub fn frame(&self) -> Size {
        self.frame
    }

    /// Where the points came from, the same for the whole list: a
    /// position the camera measured, or points placed from a table of the
    /// body's points.
    pub fn origin(&self) -> FocusOrigin {
        self.origin
    }

    /// Every point, in the order they were added. Each centre lies on the
    /// frame, and each area fits inside it around its centre.
    pub fn points(&self) -> &[FocusPoint] {
        &self.points
    }

    /// The primary point, the one [`Metadata::focus`](crate::Metadata::focus)
    /// reports, or `None` when no point is marked as the primary.
    pub fn primary(&self) -> Option<&FocusPoint> {
        self.points.get(self.primary?)
    }

    /// How the camera chose its points, or `None` when the file does not
    /// name the mode in a code raw-files knows.
    pub fn area_mode(&self) -> Option<AfAreaMode> {
        self.area_mode
    }

    /// The same list measured in `frame`, such as a preview's size: `None`
    /// when either side of `frame` is zero, and otherwise always a list,
    /// whose frame is `frame`, with the same points in the same order, the
    /// same primary, the same area mode and the same origin.
    ///
    /// Each point is scaled as [`Focus::scaled_to`] scales one, by the same
    /// rounding and the same trimming: the centre lands where it rounds to,
    /// on the new frame, an area that rounds to nothing keeps a side of one
    /// pixel, an area too large to fit around the centre is trimmed to fit,
    /// and an area that cannot fit at all, around a centre on an edge,
    /// becomes `None`. Its number and whether focus was confirmed are kept.
    ///
    /// Each call rounds to whole pixels. To place the points on several
    /// frames, scale them from the list as read each time, not from one
    /// already scaled, so that they are not rounded twice.
    #[must_use]
    pub fn scaled_to(&self, frame: Size) -> Option<Self> {
        let mut scaled = Self::new(frame, self.origin)?;
        scaled.points.reserve(self.points.len());
        for point in &self.points {
            let (center, size) = scaled_point(point.center, point.size, self.frame, frame);
            // Every scaled centre lies on the frame, and every area kept
            // fits around it, so `with_point` refuses none: the `?` never
            // returns.
            scaled = scaled.with_point(FocusPoint {
                center,
                size,
                ..*point
            })?;
        }
        scaled.primary = self.primary;
        scaled.area_mode = self.area_mode;
        Some(scaled)
    }

    /// The primary point as a [`Focus`], in this list's frame and with its
    /// origin: what [`Metadata::focus`](crate::Metadata::focus) gives. `None`
    /// when no point is the primary.
    pub(crate) fn primary_focus(&self) -> Option<Focus> {
        let point = self.primary()?;
        // `with_point` checked the point by the rules `Focus::new` and
        // `Focus::with_size` apply, in this frame, which has no zero side,
        // so neither refuses it: the two `?`s below never return.
        let mut focus = Focus::new(self.frame, point.center)?.with_origin(self.origin);
        if let Some(size) = point.size {
            focus = focus.with_size(size)?;
        }
        if let Some(in_focus) = point.in_focus {
            focus = focus.with_in_focus(in_focus);
        }
        Some(focus)
    }
}

impl From<Focus> for FocusPoints {
    /// A list of one point, `focus`'s: in its frame and with its origin,
    /// the point with no number, `focus`'s centre, area and whether focus
    /// was confirmed, as the primary, and no area mode. A single point is
    /// a list of one.
    fn from(focus: Focus) -> Self {
        Self {
            frame: focus.frame,
            points: vec![FocusPoint {
                number: None,
                center: focus.center,
                size: focus.size,
                in_focus: focus.in_focus,
            }],
            primary: Some(0),
            area_mode: None,
            origin: focus.origin,
        }
    }
}

/// One autofocus point of a body's layout, as a [`FocusLayout`] holds it:
/// the camera's number for the point, the centre of its box and the box's
/// size, in pixels of the layout's frame, by the rules of [`Focus`]'s
/// coordinates.
///
/// The number is the one the camera writes for the point, such as one of 1
/// to 39 on a body with 39 points, so a point of
/// [`Metadata::focus_points`](crate::Metadata::focus_points) is found in
/// the layout by its [`number`](FocusPoint::number). A point alone has no
/// frame, so it is checked against one, and against the layout's other
/// points, by [`FocusLayout::new`], not when it is built.
///
/// Two points are equal when the number, the centre and the size are the
/// same. There is no order, as there is none for [`FocusPoint`], and no
/// `Default`, since a point has a number and a box.
///
/// # Examples
///
/// The centre point of the Nikon D600's layout, from the record of the
/// maintainer's D600 in the NEF format notes:
///
/// ```
/// use raw_files::{LayoutPoint, Point, Size};
///
/// let centre = LayoutPoint::new(1, Point { x: 3008, y: 2007 }, Size { width: 164, height: 164 });
/// assert_eq!(centre.number(), 1);
/// assert_eq!(centre.center(), Point { x: 3008, y: 2007 });
/// assert_eq!(centre.size(), Size { width: 164, height: 164 });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutPoint {
    number: u8,
    center: Point,
    size: Size,
}

impl LayoutPoint {
    /// The point numbered `number`, with a box of `size` centred at
    /// `center`, each stored as given: [`FocusLayout::new`] checks them
    /// against the layout's frame and its other points.
    ///
    /// It is `const`, so that a layout's points can be written as a
    /// `static` array, which [`FocusLayout::new`] borrows.
    pub const fn new(number: u8, center: Point, size: Size) -> Self {
        Self {
            number,
            center,
            size,
        }
    }

    /// The camera's number for the point, as it writes it in a file that
    /// names the point.
    pub fn number(&self) -> u8 {
        self.number
    }

    /// The centre of the point's box, as its distances from the left and
    /// top edges of the layout's frame.
    pub fn center(&self) -> Point {
        self.center
    }

    /// The width and height of the point's box.
    pub fn size(&self) -> Size {
        self.size
    }
}

/// Where every autofocus point of a body lies on the picture, as
/// raw-files' table of that body holds it: each point's number, as the
/// camera writes it, the centre of its box and the box's size, each a
/// [`LayoutPoint`], all in one frame, the body's full-size output. The
/// table is measured by the project on frames shot with that body and
/// read in the maker's software, as the
/// [decision to measure per-body tables in the project](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr017-per-body-tables-measured-by-the-project.md)
/// sets out.
///
/// A viewer draws from it the grid of points the photographer saw in the
/// viewfinder, and marks on that grid the points of
/// [`Metadata::focus_points`](crate::Metadata::focus_points), each found by
/// its number with [`point`](Self::point). The centres and the boxes follow
/// the rules of [`Focus`]'s coordinates: every centre lies on the frame,
/// its edges included, and every box fits inside the frame around its
/// centre.
///
/// The frame is the table's, whatever the shot. On a shot in a crop mode,
/// a point of [`Metadata::focus_points`](crate::Metadata::focus_points)
/// is in the crop's frame, while the layout stays in the body's full
/// frame, as the
/// [decision to report the points the camera used](https://github.com/kuolas/raw-files/blob/master/docs/architecture-decisions/adr018-focus-points-placed-from-a-table-and-the-points-used.md)
/// says.
///
/// The readers give a layout only for a body whose table is in
/// raw-files, and `None` for every other body. The
/// [NEF format notes](https://github.com/kuolas/raw-files/blob/master/docs/formats/nef/autofocus-points.md)
/// record each body measured, and the tables in raw-files are written
/// from those records, so a body the notes record has a layout only when
/// its table is in raw-files.
///
/// # Equality
///
/// Two layouts are equal when every value is the same: the model text,
/// the frame and the points, compared by their values and in order, not by
/// where they are stored, so two `static` arrays of the same points give
/// equal layouts. There is no order, as there is none for [`Focus`]. There
/// is no `Default` either, since a layout has a body and at least one
/// point: a body without a table gives `None`.
///
/// # Examples
///
/// Three points of the Nikon D600's layout, from the record of the
/// maintainer's D600 in the NEF format notes: the centre point, 1, the top
/// point of the left one of the three middle columns, 13, and the top
/// point of the leftmost column, 38, each with its 164 × 164 box, in the
/// body's 6016 × 4016 frame. [`LayoutPoint::new`] is `const`, so the
/// points can be a `static` array, which the layout borrows:
///
/// ```
/// use raw_files::{FocusLayout, LayoutPoint, Point, Size};
///
/// const BOX: Size = Size { width: 164, height: 164 };
/// static POINTS: [LayoutPoint; 3] = [
///     LayoutPoint::new(1, Point { x: 3008, y: 2007 }, BOX),
///     LayoutPoint::new(13, Point { x: 2768, y: 1503 }, BOX),
///     LayoutPoint::new(38, Point { x: 1812, y: 1705 }, BOX),
/// ];
///
/// # fn main() {
/// #     example().expect("the layout accepts the D600's points");
/// # }
/// # fn example() -> Option<()> {
/// let frame = Size { width: 6016, height: 4016 };
/// let layout = FocusLayout::new("NIKON D600", frame, &POINTS)?;
/// assert_eq!(layout.model(), "NIKON D600");
/// assert_eq!(layout.frame(), frame);
/// assert_eq!(layout.points().len(), 3);
/// assert_eq!(
///     layout.point(38).map(|point| point.center()),
///     Some(Point { x: 1812, y: 1705 })
/// );
/// // The D600 has a point 2, but these three points leave it out.
/// assert_eq!(layout.point(2), None);
/// # Some(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FocusLayout {
    model: &'static str,
    frame: Size,
    points: &'static [LayoutPoint],
}

impl FocusLayout {
    /// The layout of the body whose Model is `model`, its points `points`,
    /// all in `frame`; `None` when the points cannot be a layout on that
    /// frame, which is when:
    ///
    /// - either side of `frame` is zero;
    /// - `points` is empty;
    /// - a point's number is 0, which Nikon writes for no point, or is the
    ///   number of another point;
    /// - a point's centre lies outside the frame, by the rule of
    ///   [`Focus::new`]: its x is more than the frame's width, or its y
    ///   more than its height;
    /// - a point's box has a zero side or does not fit inside the frame
    ///   around its centre, by the rule of [`Focus::with_size`].
    ///
    /// The model is stored as given, and the points are borrowed, in the
    /// order given, not copied, so a layout is `Copy`: a text, a frame and
    /// a slice. This is how a caller builds a layout for its tests.
    #[must_use]
    pub fn new(model: &'static str, frame: Size, points: &'static [LayoutPoint]) -> Option<Self> {
        if frame.width == 0 || frame.height == 0 || points.is_empty() {
            return None;
        }
        // Whether a point of each number has been seen, indexed by the
        // number: any `u8` indexes it, so one pass finds a number given
        // twice without allocating.
        let mut seen = [false; 256];
        for point in points {
            let number = usize::from(point.number);
            let numbered = point.number != 0 && !seen[number];
            if !(numbered && lies_on(frame, point.center, Some(point.size))) {
                return None;
            }
            seen[number] = true;
        }
        Some(Self {
            model,
            frame,
            points,
        })
    }

    /// The Model the layout is for, as the camera writes it in the Model tag,
    /// trimmed of padding as [`Metadata::model`](crate::Metadata::model) gives
    /// it, such as `NIKON D600`.
    pub fn model(&self) -> &'static str {
        self.model
    }

    /// The size of the frame every point is measured in: the body's
    /// full-size output, whatever the shot. It never has a zero side.
    pub fn frame(&self) -> Size {
        self.frame
    }

    /// Every point, in the order given to [`new`](Self::new). There is at
    /// least one; each number is above 0 and no other point's, each centre
    /// lies on the frame, and each box fits inside it around its centre.
    pub fn points(&self) -> &'static [LayoutPoint] {
        self.points
    }

    /// The point numbered `number`, or `None` when the layout has no point of
    /// that number, as for 0, which no point has. A point of
    /// [`Metadata::focus_points`](crate::Metadata::focus_points) is found here
    /// by its [`number`](FocusPoint::number).
    pub fn point(&self, number: u8) -> Option<LayoutPoint> {
        self.points
            .iter()
            .find(|point| point.number == number)
            .copied()
    }
}

/// Whether a point centred at `center`, with an area of `size` when it has
/// one, lies on the frame `frame`: whether [`Focus::new`] takes the centre
/// and [`Focus::with_size`] the area. It asks those constructors rather
/// than repeat their checks, so that the one rule a focus is checked by is
/// the one a [`FocusPoints`] and a [`FocusLayout`] check each of their
/// points by.
fn lies_on(frame: Size, center: Point, size: Option<Size>) -> bool {
    Focus::new(frame, center)
        .and_then(|focus| match size {
            Some(size) => focus.with_size(size),
            None => Some(focus),
        })
        .is_some()
}

/// Whether an area `side` pixels across, centred at `center`, lies inside
/// a frame `frame` pixels across, on one axis: from `center - side / 2` to
/// `center + side / 2` lies between 0 and `frame`, that is, `side` is at
/// most `2 * center` and at most `2 * (frame - center)`. Half the side is
/// rounded up, which in whole numbers is the same test. Summed in 64 bits,
/// so that no `u32` can overflow it.
fn fits(center: u32, side: u32, frame: u32) -> bool {
    let half = side.div_ceil(2);
    half <= center && u64::from(center) + u64::from(half) <= u64::from(frame)
}

/// `value`, measured on an axis `old` pixels long, measured on one `new`
/// pixels long instead: multiplied by `new` over `old` and rounded to the
/// nearest whole number, a half rounding up.
///
/// The product and the half of `old` added to it fit in 64 bits for any
/// `u32`s. `old` must not be zero, for it divides: here it is a side of
/// the frame of a [`Focus`] or of a [`FocusPoints`], which never is, and
/// the Nikon MakerNote's reader, which sizes a crop's window to a JPEG's
/// shape with it, refuses a JPEG side of zero first. A result too large
/// for a `u32`, which a value on the old frame cannot give but a window's
/// side can, saturates, at `u32::MAX`.
pub(crate) fn scale(value: u32, old: u32, new: u32) -> u32 {
    let old = u64::from(old);
    let scaled = (u64::from(value) * u64::from(new) + old / 2) / old;
    u32::try_from(scaled).unwrap_or(u32::MAX)
}

/// `side`, shortened as little as it takes for an area centred at `center`
/// to fit a frame `frame` pixels across by the rule of [`fits`]: to no more
/// than twice the smaller of `center` and `frame - center`, the centre's
/// distances from the two edges. Zero when the centre is on either edge,
/// at 0 or at `frame`, where no area fits.
fn trim(side: u32, center: u32, frame: u32) -> u32 {
    let room = 2 * u64::from(center.min(frame.saturating_sub(center)));
    u32::try_from(u64::from(side).min(room)).unwrap_or(u32::MAX)
}

/// A point centred at `center`, with an area of `size` when it has one,
/// measured in the frame `old`, measured in the frame `new` instead, by
/// the rule of [`Focus::scaled_to`], which [`FocusPoints::scaled_to`]
/// shares: the centre's x and the area's width multiplied by the new width
/// over the old, and y and the height by the heights, each rounded to the
/// nearest pixel, a half rounding up ([`scale`]); an area side that rounds
/// to nothing kept at one pixel; and the area then trimmed on each axis,
/// by as little as it takes, to fit around the centre ([`trim`]). The area
/// is `None` when a side is left with no room, as for a centre on an edge
/// of `new`.
///
/// The centre lies on `new`: the old centre is at most the old side, so
/// the scaled one is at most the new side, since rounding cannot pass it,
/// the old side scaling to exactly the new one. An area kept fits around
/// it by the rule of [`fits`].
///
/// `old` must have no zero side, for [`scale`] divides by its sides: it is
/// the frame of a [`Focus`] or of a [`FocusPoints`], which never has one.
/// The callers refuse a `new` with a zero side before they call it.
fn scaled_point(center: Point, size: Option<Size>, old: Size, new: Size) -> (Point, Option<Size>) {
    let center = Point {
        x: scale(center.x, old.width, new.width),
        y: scale(center.y, old.height, new.height),
    };
    let size = size.and_then(|size| {
        let width = scale(size.width, old.width, new.width).max(1);
        let height = scale(size.height, old.height, new.height).max(1);
        let area = Size {
            width: trim(width, center.x, new.width),
            height: trim(height, center.y, new.height),
        };
        (area.width != 0 && area.height != 0).then_some(area)
    });
    (center, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_names, assert_ordered, point, size, z9_focus};

    /// The focus a reader builds from these values, a step at a time: the
    /// centre in the frame, then the area when there is one, then whether
    /// focus was confirmed when that is known. `None` when a step refuses
    /// its value.
    fn built(
        frame: Size,
        center: Point,
        area: Option<Size>,
        in_focus: Option<bool>,
    ) -> Option<Focus> {
        let mut focus = Focus::new(frame, center)?;
        if let Some(area) = area {
            focus = focus.with_size(area)?;
        }
        if let Some(in_focus) = in_focus {
            focus = focus.with_in_focus(in_focus);
        }
        Some(focus)
    }

    #[test]
    fn a_focus_needs_a_frame_with_no_zero_side() {
        // Refused even with the centre at 0, which lies on both edges of a
        // zero side.
        assert_eq!(Focus::new(size(0, 100), point(0, 0)), None);
        assert_eq!(Focus::new(size(100, 0), point(0, 0)), None);
        assert_eq!(Focus::new(size(0, 0), point(0, 0)), None);
        assert!(Focus::new(size(1, 1), point(0, 0)).is_some());
    }

    #[test]
    fn a_focus_needs_its_centre_on_the_frame() {
        let frame = size(100, 50);
        assert_eq!(Focus::new(frame, point(101, 0)), None);
        assert_eq!(Focus::new(frame, point(0, 51)), None);
        assert_eq!(Focus::new(frame, point(101, 51)), None);
        // The far edges are on the frame.
        assert!(Focus::new(frame, point(100, 50)).is_some());
    }

    #[test]
    fn a_focus_without_a_size_may_be_anywhere_on_the_frame() {
        let frame = size(100, 50);
        let corners = [point(0, 0), point(100, 50), point(0, 50), point(100, 0)];
        for center in corners.into_iter().chain([point(99, 49)]) {
            let focus = Focus::new(frame, center);
            assert_eq!(focus.map(|focus| focus.center()), Some(center));
        }
    }

    #[test]
    fn no_area_fits_around_a_centre_on_an_edge() {
        let frame = size(100, 100);
        for center in [point(0, 50), point(100, 50), point(50, 0), point(50, 100)] {
            let focus = Focus::new(frame, center).unwrap();
            assert_eq!(focus.with_size(size(1, 1)), None, "{center:?}");
            // One pixel in from the edge, an area of 2 fits.
            let inward = point(center.x.clamp(1, 99), center.y.clamp(1, 99));
            let focus = Focus::new(frame, inward).unwrap();
            assert!(focus.with_size(size(2, 2)).is_some(), "{inward:?}");
        }
    }

    #[test]
    fn a_focus_area_needs_no_zero_side() {
        let focus = Focus::new(size(100, 100), point(50, 50)).unwrap();
        assert_eq!(focus.with_size(size(0, 10)), None);
        assert_eq!(focus.with_size(size(10, 0)), None);
        assert!(focus.with_size(size(10, 10)).is_some());
    }

    #[test]
    fn a_focus_area_fits_around_its_centre_by_half_its_side_rounded_up() {
        // In a frame of 100, an area of 10 (half 5) fits a centre from 5 to
        // 95, and one of 11 (half 6) from 6 to 94: (side, centre, accepted).
        let cases = [
            (10, 5, true),
            (10, 4, false),
            (10, 95, true),
            (10, 96, false),
            (11, 6, true),
            (11, 5, false),
            (11, 94, true),
            (11, 95, false),
        ];
        let frame = size(100, 100);
        for (side, at, accepted) in cases {
            let across = Focus::new(frame, point(at, 50)).unwrap();
            let across = across.with_size(size(side, 1));
            assert_eq!(across.is_some(), accepted, "width {side} at x = {at}");
            let down = Focus::new(frame, point(50, at)).unwrap();
            let down = down.with_size(size(1, side));
            assert_eq!(down.is_some(), accepted, "height {side} at y = {at}");
        }
    }

    #[test]
    fn a_refused_area_costs_the_focus_or_only_the_area_as_the_builder_chooses() {
        let (frame, center) = (size(100, 100), point(50, 50));
        let empty = size(0, 10);
        // With `?` on every step, the whole focus is refused.
        let whole = || Focus::new(frame, center)?.with_size(empty);
        assert_eq!(whole(), None);
        // Falling back to the focus before the area keeps the point alone.
        let focus = Focus::new(frame, center).unwrap().with_in_focus(true);
        let point_only = focus.with_size(empty).unwrap_or(focus);
        assert_eq!(point_only.center(), center);
        assert_eq!(point_only.size(), None);
        assert_eq!(point_only.in_focus(), Some(true));
    }

    #[test]
    fn a_focus_near_the_largest_u32_follows_the_same_rules() {
        const MAX: u32 = u32::MAX;
        let frame = size(MAX, MAX);
        let new = |x, y, area: Option<u32>| {
            built(frame, point(x, y), area.map(|side| size(side, side)), None).is_some()
        };
        // No `u32` lies past a frame of MAX, so every centre is on it, the
        // far edges included; no area fits around a centre there.
        assert!(new(MAX - 1, MAX - 1, None));
        assert!(new(MAX, 0, None));
        assert!(new(0, MAX, None));
        assert!(new(MAX, MAX, None));
        assert!(!new(MAX, MAX, Some(1)));

        // An area of MAX - 1, half 2^31 - 1, fits a centre from 2^31 - 1 to
        // 2^31; past that, the centre plus the half overflows a u32.
        let half = (MAX - 1) / 2;
        assert!(new(half, half, Some(MAX - 1)));
        assert!(new(half + 1, half + 1, Some(MAX - 1)));
        assert!(!new(half - 1, half, Some(MAX - 1)));
        assert!(!new(half + 2, half, Some(MAX - 1)));
        assert!(!new(half, half + 2, Some(MAX - 1)));

        // An area of MAX, half 2^31, fits no centre.
        assert!(!new(half, half, Some(MAX)));
        assert!(!new(half + 1, half + 1, Some(MAX)));

        // One pixel from the far edges, an area of 2 (half 1) reaches them,
        // and one of 3 (half 2) would sum past MAX.
        assert!(new(MAX - 1, MAX - 1, Some(2)));
        assert!(!new(MAX - 1, MAX - 1, Some(3)));
        assert!(!new(MAX - 1, MAX - 1, Some(MAX)));
    }

    #[test]
    fn focus_getters_return_what_was_built() {
        let unconfirmed = Focus::new(size(8_256, 5_504), point(4_905, 2_461))
            .unwrap()
            .with_size(size(291, 323))
            .unwrap();
        // `with_in_focus` never called is `None`, and `with_origin` never
        // called is `Recorded`.
        let (recorded, table) = (FocusOrigin::Recorded, FocusOrigin::Table);
        let all = [
            (unconfirmed.with_in_focus(true), Some(true), recorded),
            (unconfirmed.with_in_focus(false), Some(false), recorded),
            (unconfirmed, None, recorded),
            (unconfirmed.with_origin(table), None, table),
            (unconfirmed.with_origin(recorded), None, recorded),
        ];
        for (focus, in_focus, origin) in all {
            assert_eq!(focus.frame(), size(8_256, 5_504));
            assert_eq!(focus.center(), point(4_905, 2_461));
            assert_eq!(focus.size(), Some(size(291, 323)));
            assert_eq!(focus.in_focus(), in_focus);
            assert_eq!(focus.origin(), origin);
        }
        let without_size = Focus::new(size(6_000, 4_000), point(3_000, 2_000)).unwrap();
        assert_eq!(without_size.size(), None);
        assert_eq!(without_size.in_focus(), None);
        assert_eq!(without_size.origin(), recorded);
    }

    #[test]
    fn a_focus_is_recorded_unless_told_otherwise() {
        let focus = Focus::new(size(100, 100), point(50, 50)).unwrap();
        assert_eq!(focus.origin(), FocusOrigin::Recorded);

        let placed = focus.with_origin(FocusOrigin::Table);
        assert_eq!(placed.origin(), FocusOrigin::Table);
        assert_eq!(placed.frame(), size(100, 100));
        assert_eq!(placed.center(), point(50, 50));
        // The other builders keep it.
        let placed = placed.with_size(size(10, 10)).unwrap().with_in_focus(true);
        assert_eq!(placed.origin(), FocusOrigin::Table);

        // A later origin replaces the earlier one.
        let recorded = placed.with_origin(FocusOrigin::Recorded);
        assert_eq!(recorded.origin(), FocusOrigin::Recorded);
        assert_eq!(recorded.with_origin(FocusOrigin::Table), placed);

        assert_eq!(z9_focus().origin(), FocusOrigin::Recorded);
    }

    #[test]
    fn a_later_area_or_confirmation_replaces_the_earlier_one() {
        let focus = Focus::new(size(100, 100), point(50, 50))
            .unwrap()
            .with_size(size(10, 10))
            .unwrap()
            .with_in_focus(true);

        let resized = focus.with_size(size(20, 30)).unwrap();
        assert_eq!(resized.size(), Some(size(20, 30)));
        assert_eq!(resized.frame(), size(100, 100));
        assert_eq!(resized.center(), point(50, 50));
        assert_eq!(resized.in_focus(), Some(true));

        let missed = focus.with_in_focus(false);
        assert_eq!(missed.in_focus(), Some(false));
        assert_eq!(missed.frame(), size(100, 100));
        assert_eq!(missed.center(), point(50, 50));
        assert_eq!(missed.size(), Some(size(10, 10)));
        assert_eq!(missed.with_in_focus(true), focus);
    }

    #[test]
    fn a_z9_focus_scales_onto_its_preview() {
        let preview = z9_focus().scaled_to(size(1_920, 1_280)).unwrap();
        // 4905 × 1920 / 8256 = 1140.70 and 2461 × 1280 / 5504 = 572.33;
        // 291 × 1920 / 8256 = 67.67 and 323 × 1280 / 5504 = 75.12.
        assert_eq!(preview.center(), point(1_141, 572));
        assert_eq!(preview.size(), Some(size(68, 75)));
        assert_eq!(preview.frame(), size(1_920, 1_280));
        assert_eq!(preview.in_focus(), Some(true));
    }

    #[test]
    fn a_focus_scaled_to_its_own_frame_is_unchanged() {
        let z9 = z9_focus();
        assert_eq!(z9.scaled_to(z9.frame()), Some(z9));
        let odd = Focus::new(size(7, 3), point(6, 0))
            .unwrap()
            .with_in_focus(false);
        assert_eq!(odd.scaled_to(odd.frame()), Some(odd));
        let corner = Focus::new(size(7, 3), point(7, 3)).unwrap();
        assert_eq!(corner.scaled_to(corner.frame()), Some(corner));
    }

    #[test]
    fn a_focus_scaled_to_one_pixel_is_on_it_without_an_area() {
        let pixel = z9_focus().scaled_to(size(1, 1)).unwrap();
        assert_eq!(pixel.frame(), size(1, 1));
        // (4905 + 4128) / 8256 rounds to 1, on the right edge, and
        // (2461 + 2752) / 5504 to 0, on the top edge: no room for an area.
        assert_eq!(pixel.center(), point(1, 0));
        assert_eq!(pixel.size(), None);
        assert_eq!(pixel.in_focus(), Some(true));
    }

    #[test]
    fn a_scaled_focus_keeps_its_origin() {
        for origin in all_origins() {
            let focus = z9_focus().with_origin(origin);
            let preview = focus.scaled_to(size(1_920, 1_280)).unwrap();
            assert_eq!(preview.origin(), origin, "{origin:?}");
            assert_eq!(focus.scaled_to(focus.frame()), Some(focus));
            // A point without an area or a confirmation keeps it too.
            let bare = Focus::new(size(7, 3), point(6, 0))
                .unwrap()
                .with_origin(origin);
            assert_eq!(bare.scaled_to(size(14, 6)).unwrap().origin(), origin);
        }
    }

    #[test]
    fn a_focus_scales_to_no_frame_with_a_zero_side() {
        let z9 = z9_focus();
        assert_eq!(z9.scaled_to(size(0, 1_280)), None);
        assert_eq!(z9.scaled_to(size(1_920, 0)), None);
        assert_eq!(z9.scaled_to(size(0, 0)), None);
    }

    #[test]
    fn a_scaled_focus_rounds_to_the_nearest_pixel_a_half_up() {
        let scaled = |frame, center, area, to| {
            let focus = built(frame, center, area, None).unwrap();
            let scaled = focus.scaled_to(to).unwrap();
            (scaled.center(), scaled.size())
        };
        // 1 × 2 / 4 = 0.5 rounds up to 1, on both axes.
        assert_eq!(
            scaled(size(4, 4), point(1, 1), None, size(2, 2)),
            (point(1, 1), None)
        );
        // 2 × 4 / 3 = 2.67 rounds up to 3, where cutting off the fraction
        // would give 2.
        assert_eq!(
            scaled(size(3, 3), point(2, 2), None, size(4, 4)),
            (point(3, 3), None)
        );
        // 1 × 4 / 10 = 0.4 rounds down to 0.
        assert_eq!(
            scaled(size(10, 10), point(1, 1), None, size(4, 4)),
            (point(0, 0), None)
        );
        // An area of 3 × 4 / 8 = 1.5 rounds up to 2.
        assert_eq!(
            scaled(size(8, 8), point(4, 4), Some(size(3, 3)), size(4, 4)),
            (point(2, 2), Some(size(2, 2)))
        );
    }

    #[test]
    fn a_scaled_focus_keeps_its_centre_and_trims_its_area_to_fit() {
        // The area rounds to 3, and the centre to 2, which an area of 3
        // (half 2) would overflow in a frame of 3: the area is trimmed to 2.
        let focus = built(size(4, 4), point(2, 2), Some(size(4, 4)), None).unwrap();
        let scaled = focus.scaled_to(size(3, 3)).unwrap();
        assert_eq!(scaled.center(), point(2, 2));
        assert_eq!(scaled.size(), Some(size(2, 2)));

        // Only the axis that needs it is trimmed.
        let focus = built(size(4, 8), point(2, 4), Some(size(4, 4)), None).unwrap();
        let scaled = focus.scaled_to(size(3, 8)).unwrap();
        assert_eq!(scaled.center(), point(2, 4));
        assert_eq!(scaled.size(), Some(size(2, 4)));
    }

    #[test]
    fn a_focus_scales_up_and_between_the_largest_frames() {
        const MAX: u32 = u32::MAX;
        let small = built(size(2, 2), point(1, 1), Some(size(2, 2)), None).unwrap();
        let large = small.scaled_to(size(2_000, 2_000)).unwrap();
        assert_eq!(large.center(), point(1_000, 1_000));
        assert_eq!(large.size(), Some(size(2_000, 2_000)));

        // 1 × MAX / 2 rounds up to 2^31; the area of MAX that 2 × MAX / 2
        // gives is trimmed to MAX - 1 to fit around it.
        let largest = small.scaled_to(size(MAX, MAX)).unwrap();
        assert_eq!(largest.center(), point(1 << 31, 1 << 31));
        assert_eq!(largest.size(), Some(size(MAX - 1, MAX - 1)));

        let corner = built(
            size(MAX, MAX),
            point(MAX - 1, MAX - 1),
            Some(size(2, 2)),
            None,
        )
        .unwrap();
        assert_eq!(corner.scaled_to(size(MAX, MAX)), Some(corner));
        // (MAX - 1)² / MAX is MAX - 2 and a sliver; 2 × (MAX - 1) / MAX
        // is just under 2.
        let shrunk = corner.scaled_to(size(MAX - 1, MAX - 1)).unwrap();
        assert_eq!(shrunk.center(), point(MAX - 2, MAX - 2));
        assert_eq!(shrunk.size(), Some(size(2, 2)));

        let half = (MAX - 1) / 2;
        let widest = built(
            size(MAX, MAX),
            point(half, half),
            Some(size(MAX - 1, MAX - 1)),
            None,
        )
        .unwrap();
        let down = widest.scaled_to(size(1_000, 1_000)).unwrap();
        assert_eq!(down.center(), point(500, 500));
        assert_eq!(down.size(), Some(size(1_000, 1_000)));
    }

    #[test]
    fn a_scaled_focus_always_satisfies_the_rules_of_its_builders() {
        // On one axis, every frame from 1 to 12 pixels, every centre on it,
        // edges included, no area and every area `with_size` accepts around
        // it, scaled to every frame from 1 to 12 pixels. The other axis
        // stays a frame of 3 with the centre at 1 and an area of 1.
        let mut checked = 0;
        for vertical in [false, true] {
            let on_axis = |frame: u32, center: u32, area: Option<u32>| {
                if vertical {
                    (
                        size(3, frame),
                        point(1, center),
                        area.map(|side| size(1, side)),
                    )
                } else {
                    (
                        size(frame, 3),
                        point(center, 1),
                        area.map(|side| size(side, 1)),
                    )
                }
            };
            let axis = |focus: Focus| {
                let size = focus.size();
                if vertical {
                    (
                        focus.center().y,
                        size.map(|size| size.height),
                        focus.center().x,
                    )
                } else {
                    (
                        focus.center().x,
                        size.map(|size| size.width),
                        focus.center().y,
                    )
                }
            };
            for old in 1..=12_u32 {
                for center in 0..=old {
                    for area in std::iter::once(None).chain((1..=old).map(Some)) {
                        let (frame, at, area_size) = on_axis(old, center, area);
                        let Some(focus) = built(frame, at, area_size, Some(false)) else {
                            continue;
                        };
                        for new in 1..=12_u32 {
                            let (target, _, _) = on_axis(new, 0, None);
                            let context = format!("{focus:?} to {target:?}");
                            let scaled = focus.scaled_to(target).expect(&context);
                            assert_eq!(scaled.frame(), target, "{context}");
                            assert_eq!(scaled.in_focus(), Some(false), "{context}");
                            // Built again a step at a time, it is the same.
                            assert_eq!(
                                built(
                                    scaled.frame(),
                                    scaled.center(),
                                    scaled.size(),
                                    scaled.in_focus()
                                ),
                                Some(scaled),
                                "{context}"
                            );

                            // The nearest whole number, a half up, which is
                            // on the new frame: never moved by the trimming
                            // of the area.
                            let (got_center, got_area, other) = axis(scaled);
                            let nearest = |value: u32| (2 * value * new + old) / (2 * old);
                            assert_eq!(got_center, nearest(center), "{context}");
                            assert!(got_center <= new, "{context}");
                            assert_eq!(other, 1, "{context}");

                            // The area, at least 1, trimmed only as far as
                            // it must be to fit, and dropped only for a
                            // centre on an edge.
                            match (area, got_area) {
                                (None, got) => assert_eq!(got, None, "{context}"),
                                (Some(side), Some(got)) => {
                                    let rounded = nearest(side).max(1);
                                    assert!(got <= rounded, "{context}");
                                    let one_more = fits(got_center, got + 1, new);
                                    assert!(got == rounded || !one_more, "{context}");
                                }
                                (Some(_), None) => {
                                    assert!(got_center == 0 || got_center == new, "{context}");
                                }
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
        // Every case was reached: 412 starting focuses on each axis, each
        // scaled 12 ways.
        assert_eq!(checked, 9_888);
    }

    #[test]
    fn focus_equality_is_by_every_value_and_collects_in_a_set() {
        let z9 = z9_focus();
        assert_eq!(z9, z9_focus());
        let frame = size(8_256, 5_504);
        let center = point(4_905, 2_461);
        let area = Some(size(291, 323));
        let others = [
            built(size(8_257, 5_504), center, area, Some(true)),
            built(size(8_256, 5_505), center, area, Some(true)),
            built(frame, point(4_906, 2_461), area, Some(true)),
            built(frame, point(4_905, 2_462), area, Some(true)),
            built(frame, center, Some(size(292, 323)), Some(true)),
            built(frame, center, Some(size(291, 324)), Some(true)),
            built(frame, center, None, Some(true)),
            built(frame, center, area, Some(false)),
            built(frame, center, area, None),
            // The same point, placed from a table.
            built(frame, center, area, Some(true))
                .map(|focus| focus.with_origin(FocusOrigin::Table)),
        ];
        let others: Vec<Focus> = others.into_iter().map(Option::unwrap).collect();
        for (place, other) in others.iter().enumerate() {
            assert_ne!(&z9, other, "value {place}");
        }

        let mut all = others.clone();
        all.extend([z9, z9_focus()]);
        all.extend(others.iter().copied());
        let set: std::collections::HashSet<Focus> = all.into_iter().collect();
        assert_eq!(set.len(), others.len() + 1);
    }

    // Every origin, in the order of its declaration. The `match` has no
    // wildcard arm, so a variant added later does not compile until it is
    // listed here too, and the test below then fails until its name is
    // listed there.
    fn all_origins() -> [FocusOrigin; 2] {
        let all = [FocusOrigin::Recorded, FocusOrigin::Table];
        for origin in all {
            match origin {
                FocusOrigin::Recorded | FocusOrigin::Table => {}
            }
        }
        all
    }

    #[test]
    fn every_name_is_the_one_documented_and_display_writes_it() {
        // The names are stable once released: a program may show or store
        // them, so a changed one fails here.
        assert_names(&all_origins(), FocusOrigin::name, &["Recorded", "Table"]);
    }

    #[test]
    fn origins_order_as_their_variants_are_listed() {
        // A program may key a sorted map by the origin, and the order is a
        // promise: a variant moved, or added before the others, fails.
        assert_ordered(&all_origins());
    }

    /// The list in `frame`, which came from `origin`, of `points`, each
    /// added in turn; `None` when a step refuses one.
    fn list_of(frame: Size, origin: FocusOrigin, points: &[FocusPoint]) -> Option<FocusPoints> {
        points
            .iter()
            .try_fold(FocusPoints::new(frame, origin)?, |list, &point| {
                list.with_point(point)
            })
    }

    /// The point `focus` describes: its centre, its area and whether focus
    /// was confirmed, with no number. Built field by field, apart from the
    /// `From` that turns a focus into a list.
    fn point_of(focus: Focus) -> FocusPoint {
        FocusPoint {
            number: None,
            center: focus.center(),
            size: focus.size(),
            in_focus: focus.in_focus(),
        }
    }

    #[test]
    fn a_list_needs_a_frame_with_no_zero_side() {
        for origin in all_origins() {
            assert_eq!(FocusPoints::new(size(0, 100), origin), None);
            assert_eq!(FocusPoints::new(size(100, 0), origin), None);
            assert_eq!(FocusPoints::new(size(0, 0), origin), None);
            // A new list is empty, with no primary and no area mode.
            let list = FocusPoints::new(size(1, 1), origin).unwrap();
            assert_eq!(list.frame(), size(1, 1));
            assert_eq!(list.origin(), origin);
            assert!(list.points().is_empty());
            assert_eq!(list.primary(), None);
            assert_eq!(list.area_mode(), None);
        }
    }

    #[test]
    fn a_point_off_the_frame_or_with_an_area_that_does_not_fit_is_refused() {
        let list = || FocusPoints::new(size(100, 50), FocusOrigin::Recorded).unwrap();
        // The far edges are on the frame; one pixel past them is not.
        assert!(list().with_point(FocusPoint::new(point(100, 50))).is_some());
        assert_eq!(list().with_point(FocusPoint::new(point(101, 0))), None);
        assert_eq!(list().with_point(FocusPoint::new(point(0, 51))), None);
        // An area needs no zero side, and must fit around its centre: at
        // (50, 25), 100 by 50 just fits, and a pixel more on either side
        // does not.
        let middle = FocusPoint::new(point(50, 25));
        assert_eq!(list().with_point(middle.with_size(size(0, 10))), None);
        assert_eq!(list().with_point(middle.with_size(size(10, 0))), None);
        assert!(list().with_point(middle.with_size(size(100, 50))).is_some());
        assert_eq!(list().with_point(middle.with_size(size(101, 50))), None);
        assert_eq!(list().with_point(middle.with_size(size(100, 51))), None);
        // A refused point costs the list it was offered to; a clone made
        // before keeps the points added so far.
        let one = list().with_point(middle).unwrap();
        assert_eq!(one.clone().with_point(FocusPoint::new(point(101, 0))), None);
        assert_eq!(one.points(), [middle]);
        // Near the largest `u32`, as for a focus: every centre is on the
        // frame, and no area fits around one on its far edges.
        const MAX: u32 = u32::MAX;
        let largest = FocusPoints::new(size(MAX, MAX), FocusOrigin::Recorded).unwrap();
        let corner = FocusPoint::new(point(MAX, MAX));
        assert!(largest.clone().with_point(corner).is_some());
        assert_eq!(largest.with_point(corner.with_size(size(1, 1))), None);
    }

    #[test]
    fn a_point_is_added_exactly_when_a_focus_of_it_could_be_built() {
        // On one axis, every frame from 1 to 8 pixels, every centre from 0
        // to one past the frame, and no area or every area from 0 to two
        // more than the frame; the other axis a frame of 3, the centre at
        // 1 and an area of 1. A point is added exactly when `Focus::new`
        // and `Focus::with_size` accept the same values.
        let (mut checked, mut added) = (0, 0);
        for vertical in [false, true] {
            for side in 1..=8_u32 {
                for at in 0..=side + 1 {
                    for area in std::iter::once(None).chain((0..=side + 1).map(Some)) {
                        let (frame, center, area) = if vertical {
                            (size(3, side), point(1, at), area.map(|area| size(1, area)))
                        } else {
                            (size(side, 3), point(at, 1), area.map(|area| size(area, 1)))
                        };
                        let mut offered = FocusPoint::new(center).with_in_focus(true);
                        if let Some(area) = area {
                            offered = offered.with_size(area);
                        }
                        let list = list_of(frame, FocusOrigin::Recorded, &[offered]);
                        let focus = built(frame, center, area, Some(true));
                        let context = format!("{frame:?}, {center:?}, {area:?}");
                        assert_eq!(list.is_some(), focus.is_some(), "{context}");
                        if let Some(list) = list {
                            assert_eq!(list.points(), [offered], "{context}");
                            added += 1;
                        }
                        checked += 1;
                    }
                }
            }
        }
        // Every case was reached, on both axes, and some of each kind.
        assert_eq!(checked, 864);
        assert!(added > 0 && added < checked, "{added} of {checked}");
    }

    #[test]
    fn the_primary_is_the_index_of_a_point_added_so_far() {
        let first = FocusPoint::new(point(10, 10));
        let second = FocusPoint::new(point(20, 20)).with_number(2);
        let empty = FocusPoints::new(size(100, 100), FocusOrigin::Recorded).unwrap();
        assert_eq!(empty.clone().with_primary(0), None);
        let list = list_of(size(100, 100), FocusOrigin::Recorded, &[first, second]).unwrap();
        assert_eq!(list.primary(), None);
        assert_eq!(list.clone().with_primary(2), None);
        assert_eq!(list.clone().with_primary(usize::MAX), None);
        assert_eq!(
            list.clone().with_primary(0).unwrap().primary(),
            Some(&first)
        );
        let marked = list.with_primary(1).unwrap();
        assert_eq!(marked.primary(), Some(&second));
        // A later primary replaces the earlier one, and a point added
        // after it leaves it as it was.
        assert_eq!(
            marked.clone().with_primary(0).unwrap().primary(),
            Some(&first)
        );
        let longer = marked.with_point(FocusPoint::new(point(30, 30))).unwrap();
        assert_eq!(longer.primary(), Some(&second));
        assert_eq!(longer.points().len(), 3);
    }

    #[test]
    fn a_later_area_mode_replaces_the_earlier_one() {
        let list = list_of(
            size(100, 100),
            FocusOrigin::Table,
            &[FocusPoint::new(point(50, 50))],
        )
        .and_then(|list| list.with_primary(0))
        .unwrap();
        for mode in all_area_modes() {
            let set = list
                .clone()
                .with_area_mode(AfAreaMode::Tracking)
                .with_area_mode(mode);
            assert_eq!(set.area_mode(), Some(mode), "{mode:?}");
            // Nothing else changes.
            assert_eq!(set.frame(), list.frame(), "{mode:?}");
            assert_eq!(set.points(), list.points(), "{mode:?}");
            assert_eq!(set.primary(), list.primary(), "{mode:?}");
            assert_eq!(set.origin(), FocusOrigin::Table, "{mode:?}");
        }
    }

    #[test]
    fn a_point_reads_back_what_was_built_and_a_later_value_replaces_an_earlier_one() {
        let bare = FocusPoint::new(point(3_008, 2_008));
        assert_eq!(bare.number(), None);
        assert_eq!(bare.center(), point(3_008, 2_008));
        assert_eq!(bare.size(), None);
        assert_eq!(bare.in_focus(), None);
        let full = bare
            .with_number(1)
            .with_size(size(1, 1))
            .with_in_focus(false)
            .with_number(21)
            .with_size(size(164, 164))
            .with_in_focus(true);
        assert_eq!(full.number(), Some(21));
        assert_eq!(full.center(), point(3_008, 2_008));
        assert_eq!(full.size(), Some(size(164, 164)));
        assert_eq!(full.in_focus(), Some(true));
        // A size is stored as given, even one no frame takes: a list
        // checks it when the point is added.
        assert_eq!(
            bare.with_size(size(0, u32::MAX)).size(),
            Some(size(0, u32::MAX))
        );
    }

    #[test]
    fn a_focus_is_a_list_of_one_point_its_primary() {
        let bare = Focus::new(size(7, 3), point(6, 0)).unwrap();
        for origin in all_origins() {
            for focus in [z9_focus(), z9_focus().with_in_focus(false), bare] {
                let focus = focus.with_origin(origin);
                let list = FocusPoints::from(focus);
                let only = point_of(focus);
                assert_eq!(list.frame(), focus.frame(), "{focus:?}");
                assert_eq!(list.origin(), origin, "{focus:?}");
                assert_eq!(list.points(), [only], "{focus:?}");
                assert_eq!(list.primary(), Some(&only), "{focus:?}");
                assert_eq!(list.area_mode(), None, "{focus:?}");
                // The same list as the builders make, and its primary is
                // the focus given.
                let built =
                    list_of(focus.frame(), origin, &[only]).and_then(|built| built.with_primary(0));
                assert_eq!(built.as_ref(), Some(&list), "{focus:?}");
                assert_eq!(list.primary_focus(), Some(focus), "{focus:?}");
            }
        }
    }

    #[test]
    fn the_primary_focus_is_the_primary_point_in_the_lists_frame_with_its_origin() {
        let first = FocusPoint::new(point(100, 100)).with_number(1);
        let second = FocusPoint::new(point(4_905, 2_461))
            .with_number(2)
            .with_size(size(291, 323))
            .with_in_focus(true);
        for origin in all_origins() {
            let list = list_of(size(8_256, 5_504), origin, &[first, second])
                .unwrap()
                .with_area_mode(AfAreaMode::Auto);
            // No primary, no focus.
            assert_eq!(list.primary_focus(), None, "{origin:?}");
            let on_second = list.clone().with_primary(1).unwrap().primary_focus();
            assert_eq!(
                on_second,
                Some(z9_focus().with_origin(origin)),
                "{origin:?}"
            );
            let on_first = list.with_primary(0).unwrap().primary_focus().unwrap();
            assert_eq!(on_first.frame(), size(8_256, 5_504), "{origin:?}");
            assert_eq!(on_first.center(), point(100, 100), "{origin:?}");
            assert_eq!(on_first.size(), None, "{origin:?}");
            assert_eq!(on_first.in_focus(), None, "{origin:?}");
            assert_eq!(on_first.origin(), origin, "{origin:?}");
        }
    }

    #[test]
    fn a_list_scales_onto_a_preview() {
        // The Z 9's point, as `a_z9_focus_scales_onto_its_preview` scales
        // it, and a second point at the frame's middle with no area.
        let list = list_of(
            size(8_256, 5_504),
            FocusOrigin::Recorded,
            &[
                FocusPoint::new(point(4_128, 2_752)).with_number(5),
                point_of(z9_focus()),
            ],
        )
        .and_then(|list| list.with_primary(1))
        .unwrap()
        .with_area_mode(AfAreaMode::Single);
        let preview = list.scaled_to(size(1_920, 1_280)).unwrap();
        assert_eq!(preview.frame(), size(1_920, 1_280));
        assert_eq!(
            preview.points(),
            [
                FocusPoint::new(point(960, 640)).with_number(5),
                FocusPoint::new(point(1_141, 572))
                    .with_size(size(68, 75))
                    .with_in_focus(true),
            ]
        );
        assert_eq!(preview.primary(), preview.points().get(1));
        assert_eq!(preview.area_mode(), Some(AfAreaMode::Single));
        assert_eq!(preview.origin(), FocusOrigin::Recorded);
    }

    #[test]
    fn a_list_scaled_to_its_own_frame_is_unchanged_and_to_a_zero_side_is_none() {
        for origin in all_origins() {
            let list = FocusPoints::from(z9_focus().with_origin(origin))
                .with_area_mode(AfAreaMode::Dynamic)
                .with_point(FocusPoint::new(point(8_256, 0)).with_number(3))
                .unwrap();
            assert_eq!(
                list.scaled_to(list.frame()),
                Some(list.clone()),
                "{origin:?}"
            );
            assert_eq!(list.scaled_to(size(0, 1_280)), None, "{origin:?}");
            assert_eq!(list.scaled_to(size(1_920, 0)), None, "{origin:?}");
            assert_eq!(list.scaled_to(size(0, 0)), None, "{origin:?}");
        }
        // A list without points or a primary scales too.
        let empty = FocusPoints::new(size(7, 3), FocusOrigin::Recorded).unwrap();
        let scaled = empty.scaled_to(size(14, 6)).unwrap();
        assert!(scaled.points().is_empty());
        assert_eq!(scaled.primary(), None);
        assert_eq!(scaled.frame(), size(14, 6));
    }

    #[test]
    fn a_list_scales_each_point_as_a_focus_is_scaled() {
        // Across, every frame from 1 to 8 pixels and every focus that can be
        // built on it, every centre and no area or every area that fits,
        // down a frame of 3 with the centre at 1 and an area of 1. Each is
        // the primary of a list beside a numbered point without an area,
        // and the list is scaled to frames from 1 by 8 to 8 by 1: each of
        // its points is the focus it describes, scaled.
        let mut checked = 0;
        for old in 1..=8_u32 {
            for center in 0..=old {
                for area in std::iter::once(None).chain((1..=old).map(Some)) {
                    let frame = size(old, 3);
                    let area = area.map(|side| size(side, 1));
                    let Some(focus) = built(frame, point(center, 1), area, Some(true)) else {
                        continue;
                    };
                    let points = [
                        FocusPoint::new(point(old - center, 2)).with_number(9),
                        point_of(focus).with_number(7),
                    ];
                    let list = list_of(frame, FocusOrigin::Table, &points)
                        .and_then(|list| list.with_primary(1))
                        .unwrap()
                        .with_area_mode(AfAreaMode::Dynamic);
                    for new in 1..=8_u32 {
                        let target = size(new, 9 - new);
                        let context = format!("{list:?} to {target:?}");
                        let scaled = list.scaled_to(target).expect(&context);
                        assert_eq!(scaled.frame(), target, "{context}");
                        assert_eq!(scaled.origin(), FocusOrigin::Table, "{context}");
                        assert_eq!(scaled.area_mode(), Some(AfAreaMode::Dynamic), "{context}");
                        assert_eq!(scaled.points().len(), 2, "{context}");
                        assert_eq!(scaled.primary(), scaled.points().get(1), "{context}");
                        for (got, was) in scaled.points().iter().zip(&points) {
                            let as_focus = built(frame, was.center(), was.size(), was.in_focus())
                                .and_then(|focus| focus.scaled_to(target))
                                .expect(&context);
                            let number = was.number().expect("numbered");
                            assert_eq!(*got, point_of(as_focus).with_number(number), "{context}");
                        }
                        assert_eq!(
                            scaled.primary_focus(),
                            focus.with_origin(FocusOrigin::Table).scaled_to(target),
                            "{context}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        // Every case was reached: 144 starting focuses, each scaled 8
        // ways.
        assert_eq!(checked, 1_152);
    }

    #[test]
    fn focus_points_equality_is_by_every_value_and_collects_in_a_set() {
        use AfAreaMode::{Auto, Single};
        use FocusOrigin::{Recorded, Table};
        let (a, b) = (
            FocusPoint::new(point(10, 10)),
            FocusPoint::new(point(20, 20)),
        );
        let list = |frame, origin, points: &[FocusPoint], primary, mode| {
            let mut list = list_of(frame, origin, points).unwrap();
            if let Some(primary) = primary {
                list = list.with_primary(primary).unwrap();
            }
            if let Some(mode) = mode {
                list = list.with_area_mode(mode);
            }
            list
        };
        let frame = size(100, 100);
        let base = list(frame, Recorded, &[a, b], Some(0), Some(Single));
        assert_eq!(base, list(frame, Recorded, &[a, b], Some(0), Some(Single)));
        let others = [
            list(size(101, 100), Recorded, &[a, b], Some(0), Some(Single)),
            list(size(100, 101), Recorded, &[a, b], Some(0), Some(Single)),
            list(frame, Table, &[a, b], Some(0), Some(Single)),
            // The same points in the other order.
            list(frame, Recorded, &[b, a], Some(0), Some(Single)),
            list(frame, Recorded, &[a], Some(0), Some(Single)),
            list(frame, Recorded, &[a, b, a], Some(0), Some(Single)),
            list(frame, Recorded, &[a, b], Some(1), Some(Single)),
            list(frame, Recorded, &[a, b], None, Some(Single)),
            list(frame, Recorded, &[a, b], Some(0), Some(Auto)),
            list(frame, Recorded, &[a, b], Some(0), None),
            // A point that differs in one value.
            list(
                frame,
                Recorded,
                &[a.with_number(1), b],
                Some(0),
                Some(Single),
            ),
            list(
                frame,
                Recorded,
                &[a, FocusPoint::new(point(20, 21))],
                Some(0),
                Some(Single),
            ),
            list(
                frame,
                Recorded,
                &[a.with_size(size(2, 2)), b],
                Some(0),
                Some(Single),
            ),
            list(
                frame,
                Recorded,
                &[a.with_in_focus(false), b],
                Some(0),
                Some(Single),
            ),
        ];
        for (place, other) in others.iter().enumerate() {
            assert_ne!(&base, other, "value {place}");
        }

        let mut all = others.to_vec();
        all.extend([base.clone(), base.clone()]);
        all.extend(others.iter().cloned());
        let set: std::collections::HashSet<FocusPoints> = all.into_iter().collect();
        assert_eq!(set.len(), others.len() + 1);
    }

    // Every area mode, in the order of its declaration. The `match` has no
    // wildcard arm, so a variant added later does not compile until it is
    // listed here too, and the tests below then fail until its name is
    // listed there.
    fn all_area_modes() -> [AfAreaMode; 4] {
        let all = [
            AfAreaMode::Single,
            AfAreaMode::Dynamic,
            AfAreaMode::Auto,
            AfAreaMode::Tracking,
        ];
        for mode in all {
            match mode {
                AfAreaMode::Single
                | AfAreaMode::Dynamic
                | AfAreaMode::Auto
                | AfAreaMode::Tracking => {}
            }
        }
        all
    }

    #[test]
    fn every_area_mode_name_is_the_one_documented_and_display_writes_it() {
        // The names are stable once released: a program may show or store
        // them, so a changed one fails here.
        assert_names(
            &all_area_modes(),
            AfAreaMode::name,
            &["Single point", "Dynamic area", "Auto area", "Tracking"],
        );
    }

    #[test]
    fn area_modes_order_as_their_variants_are_listed() {
        // A program may key a sorted map by the mode, and the order is a
        // promise: a variant moved, or added before the others, fails.
        assert_ordered(&all_area_modes());
    }

    /// The D600's frame, its full-size output, and the box of every one of
    /// its points, as the record of the maintainer's D600 in the NEF format
    /// notes gives them.
    const D600_FRAME: Size = Size {
        width: 6_016,
        height: 4_016,
    };
    const D600_BOX: Size = Size {
        width: 164,
        height: 164,
    };

    /// The D600's point `number`, centred at (x, y), with the D600's box.
    const fn d600(number: u8, x: u32, y: u32) -> LayoutPoint {
        LayoutPoint::new(number, Point { x, y }, D600_BOX)
    }

    /// Point 1, centred at (x, y), with a box `width` by `height`.
    const fn first(x: u32, y: u32, width: u32, height: u32) -> LayoutPoint {
        LayoutPoint::new(1, Point { x, y }, Size { width, height })
    }

    /// Three points of the D600's layout, from the same record: the centre
    /// point, 1; the top point of the left one of the three middle columns,
    /// 13; and the top point of the leftmost column, 38.
    static D600_THREE: [LayoutPoint; 3] = [
        d600(1, 3_008, 2_007),
        d600(13, 2_768, 1_503),
        d600(38, 1_812, 1_705),
    ];

    /// The layout of [`D600_THREE`] in the D600's frame.
    fn d600_three() -> FocusLayout {
        FocusLayout::new("NIKON D600", D600_FRAME, &D600_THREE).unwrap()
    }

    #[test]
    fn a_layout_needs_a_frame_with_no_zero_side() {
        // A point in the middle of a 2 by 2 frame, with a box of 2 by 2,
        // fits it. No box fits a frame with a zero side either, so the
        // frame's rule and the box's both refuse these.
        static MIDDLE: [LayoutPoint; 1] = [first(1, 1, 2, 2)];
        assert_eq!(FocusLayout::new("A", size(0, 2), &MIDDLE), None);
        assert_eq!(FocusLayout::new("A", size(2, 0), &MIDDLE), None);
        assert_eq!(FocusLayout::new("A", size(0, 0), &MIDDLE), None);
        assert!(FocusLayout::new("A", size(2, 2), &MIDDLE).is_some());
        for frame in [size(0, 4_016), size(6_016, 0)] {
            let layout = FocusLayout::new("NIKON D600", frame, &D600_THREE);
            assert_eq!(layout, None, "{frame:?}");
        }
    }

    #[test]
    fn a_layout_needs_at_least_one_point() {
        assert_eq!(FocusLayout::new("NIKON D600", D600_FRAME, &[]), None);
        static CENTRE: [LayoutPoint; 1] = [d600(1, 3_008, 2_007)];
        let one = FocusLayout::new("NIKON D600", D600_FRAME, &CENTRE).unwrap();
        assert_eq!(one.points(), CENTRE);
    }

    #[test]
    fn a_layout_point_needs_a_number_above_zero_that_no_other_has() {
        let new =
            |points: &'static [LayoutPoint]| FocusLayout::new("NIKON D600", D600_FRAME, points);
        // Nikon writes 0 for no point: refused alone, or after a good
        // point.
        static ZERO: [LayoutPoint; 1] = [d600(0, 3_008, 2_007)];
        static ZERO_LAST: [LayoutPoint; 2] = [d600(1, 3_008, 2_007), d600(0, 2_768, 1_503)];
        assert_eq!(new(&ZERO), None);
        assert_eq!(new(&ZERO_LAST), None);
        // A number given twice, at two places apart in the list, or as the
        // same point twice in a row.
        static APART: [LayoutPoint; 3] = [
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
            d600(1, 1_812, 1_705),
        ];
        static TWICE: [LayoutPoint; 2] = [d600(38, 1_812, 1_705), d600(38, 1_812, 1_705)];
        assert_eq!(new(&APART), None);
        assert_eq!(new(&TWICE), None);
        // Any other numbers, in any order, the largest included.
        static ANY: [LayoutPoint; 3] = [
            d600(255, 3_008, 2_007),
            d600(2, 2_768, 1_503),
            d600(1, 1_812, 1_705),
        ];
        assert_eq!(new(&ANY).map(|layout| layout.points()), Some(&ANY[..]));
        assert!(new(&D600_THREE).is_some());
    }

    #[test]
    fn a_layout_point_lies_inside_the_frame_by_the_rules_of_a_focus() {
        // In a frame of 100 by 50: a centre off the frame, a box with a
        // zero side, and a box that does not fit around its centre, each
        // refused; at (50, 25), a box of 100 by 50 just fits, and one a
        // pixel larger either way does not.
        static OFF_ACROSS: [LayoutPoint; 1] = [first(101, 25, 2, 2)];
        static OFF_DOWN: [LayoutPoint; 1] = [first(50, 51, 2, 2)];
        static NO_WIDTH: [LayoutPoint; 1] = [first(50, 25, 0, 10)];
        static NO_HEIGHT: [LayoutPoint; 1] = [first(50, 25, 10, 0)];
        static TOO_WIDE: [LayoutPoint; 1] = [first(50, 25, 101, 50)];
        static TOO_TALL: [LayoutPoint; 1] = [first(50, 25, 100, 51)];
        static WHOLE: [LayoutPoint; 1] = [first(50, 25, 100, 50)];
        let frame = size(100, 50);
        for refused in [
            &OFF_ACROSS,
            &OFF_DOWN,
            &NO_WIDTH,
            &NO_HEIGHT,
            &TOO_WIDE,
            &TOO_TALL,
        ] {
            assert_eq!(FocusLayout::new("A", frame, refused), None, "{refused:?}");
        }
        assert!(FocusLayout::new("A", frame, &WHOLE).is_some());

        // A point that does not fit costs the layout wherever it is in the
        // list: here the last, one pixel past the D600's frame.
        static LAST_OFF: [LayoutPoint; 3] = [
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
            d600(38, 6_017, 1_705),
        ];
        assert_eq!(FocusLayout::new("NIKON D600", D600_FRAME, &LAST_OFF), None);

        // On one axis, every frame from 1 to 8 pixels, every centre from 0
        // to one past the frame and every box from 0 to two more than the
        // frame; the other axis a frame of 3, the centre at 1 and a box of
        // 2. A one-point layout is built exactly when `Focus::new` and
        // `Focus::with_size` accept the same values: the edges are on the
        // frame, and a box fits by half its side rounded up. Each point is
        // leaked, to make the `'static` slice a layout borrows.
        let (mut checked, mut built) = (0, 0);
        for vertical in [false, true] {
            for side in 1..=8_u32 {
                for at in 0..=side + 1 {
                    for area in 0..=side + 1 {
                        let (frame, point) = if vertical {
                            (size(3, side), first(1, at, 2, area))
                        } else {
                            (size(side, 3), first(at, 1, area, 2))
                        };
                        let points: &'static [LayoutPoint] = vec![point].leak();
                        let layout = FocusLayout::new("A", frame, points);
                        let focus = Focus::new(frame, point.center())
                            .and_then(|focus| focus.with_size(point.size()));
                        let context = format!("{frame:?}, {point:?}");
                        assert_eq!(layout.is_some(), focus.is_some(), "{context}");
                        built += usize::from(layout.is_some());
                        checked += 1;
                    }
                }
            }
        }
        // Every case was reached, on both axes, and some of each kind:
        // (side + 2)² cases for each side, 380 on each axis.
        assert_eq!(checked, 760);
        assert!(built > 0 && built < checked, "{built} of {checked}");
    }

    #[test]
    fn layout_getters_return_what_was_built() {
        let layout = d600_three();
        assert_eq!(layout.model(), "NIKON D600");
        assert_eq!(layout.frame(), D600_FRAME);
        assert_eq!(layout.points(), D600_THREE);
        // The points are borrowed as given, in their order, not copied.
        assert_eq!(layout.points().as_ptr(), D600_THREE.as_ptr());
        let read: Vec<(u8, Point, Size)> = layout
            .points()
            .iter()
            .map(|point| (point.number(), point.center(), point.size()))
            .collect();
        assert_eq!(
            read,
            [
                (1, point(3_008, 2_007), size(164, 164)),
                (13, point(2_768, 1_503), size(164, 164)),
                (38, point(1_812, 1_705), size(164, 164)),
            ]
        );
        // A point keeps what it was built with, even values no layout
        // takes: a layout checks them.
        let any = LayoutPoint::new(0, point(u32::MAX, 0), size(0, u32::MAX));
        assert_eq!(
            (any.number(), any.center(), any.size()),
            (0, point(u32::MAX, 0), size(0, u32::MAX))
        );
    }

    #[test]
    fn a_layout_point_is_found_by_its_number() {
        // The points out of number order too, so that the first point is
        // not always the one asked for.
        static SHUFFLED: [LayoutPoint; 3] = [
            d600(38, 1_812, 1_705),
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
        ];
        let shuffled = FocusLayout::new("NIKON D600", D600_FRAME, &SHUFFLED).unwrap();
        for layout in [d600_three(), shuffled] {
            for (number, x, y) in [(1, 3_008, 2_007), (13, 2_768, 1_503), (38, 1_812, 1_705)] {
                let found = layout.point(number);
                assert_eq!(found, Some(d600(number, x, y)), "{number} in {layout:?}");
            }
            // No point is numbered 0, and these three leave out 2, 39 and
            // every number past the D600's.
            for number in [0, 2, 39, 255] {
                assert_eq!(layout.point(number), None, "{number} in {layout:?}");
            }
        }
    }

    #[test]
    fn layout_equality_is_by_every_value() {
        // The same three points in another array: equal, for the points
        // are compared by their values, not by where they are stored.
        static AGAIN: [LayoutPoint; 3] = [
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
            d600(38, 1_812, 1_705),
        ];
        let base = d600_three();
        let again = FocusLayout::new("NIKON D600", D600_FRAME, &AGAIN).unwrap();
        assert_ne!(AGAIN.as_ptr(), D600_THREE.as_ptr());
        assert_eq!(base, again);

        // One point's centre moved by a pixel, its number changed, its box
        // changed; the points in another order; one point fewer.
        static MOVED: [LayoutPoint; 3] = [
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
            d600(38, 1_813, 1_705),
        ];
        static RENUMBERED: [LayoutPoint; 3] = [
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
            d600(39, 1_812, 1_705),
        ];
        static RESIZED: [LayoutPoint; 3] = [
            d600(1, 3_008, 2_007),
            d600(13, 2_768, 1_503),
            LayoutPoint::new(
                38,
                Point { x: 1_812, y: 1_705 },
                Size {
                    width: 164,
                    height: 166,
                },
            ),
        ];
        static REORDERED: [LayoutPoint; 3] = [
            d600(13, 2_768, 1_503),
            d600(1, 3_008, 2_007),
            d600(38, 1_812, 1_705),
        ];
        let others = [
            FocusLayout::new("NIKON D610", D600_FRAME, &D600_THREE),
            FocusLayout::new("NIKON D600", size(6_017, 4_016), &D600_THREE),
            FocusLayout::new("NIKON D600", size(6_016, 4_017), &D600_THREE),
            FocusLayout::new("NIKON D600", D600_FRAME, &MOVED),
            FocusLayout::new("NIKON D600", D600_FRAME, &RENUMBERED),
            FocusLayout::new("NIKON D600", D600_FRAME, &RESIZED),
            FocusLayout::new("NIKON D600", D600_FRAME, &REORDERED),
            FocusLayout::new("NIKON D600", D600_FRAME, &D600_THREE[..2]),
        ];
        let others: Vec<FocusLayout> = others.into_iter().map(Option::unwrap).collect();
        for (place, other) in others.iter().enumerate() {
            assert_ne!(&base, other, "value {place}");
        }

        let mut all = others.clone();
        all.extend([base, again]);
        all.extend(others.iter().copied());
        let set: std::collections::HashSet<FocusLayout> = all.into_iter().collect();
        assert_eq!(set.len(), others.len() + 1);
    }
}
