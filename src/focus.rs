// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the camera's autofocus aimed.
//!
//! A viewer can mark on a preview the place where the camera focused. The
//! camera records it as the centre of the focus area, the area's size when
//! it records one, the frame the two are measured in, which has the
//! previews' framing, and, when the file says, whether focus was confirmed.
//! [`Focus`] holds them, and a caller's tests build one through its checked
//! constructors.
//!
//! [`Metadata::focus`](crate::Metadata::focus) gives the focus point of a
//! file, if it records one.

use crate::geometry::{Point, Size};

/// Where the camera's autofocus aimed, as the camera recorded it: the
/// centre of the focus area, the area's size when the file records one,
/// the frame the two are measured in, and whether the camera confirmed
/// focus, when the file says.
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
/// Two values are equal when every value is the same as stored. There is
/// no order: no order of focus records means anything, and sorting them
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
/// use raw_files::{Focus, Point, Size};
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
}

impl Focus {
    /// The focus centred at `center`, measured in `frame`, with no area and
    /// nothing said about whether the camera confirmed focus; `None` when
    /// the centre cannot lie on the frame. [`with_size`](Self::with_size)
    /// adds the area and [`with_in_focus`](Self::with_in_focus) whether
    /// focus was confirmed.
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

    /// The same focus measured in `frame`, such as a preview's size: `None`
    /// when either side of `frame` is zero, and otherwise always a focus,
    /// whose frame is `frame` and whose [`in_focus`](Self::in_focus) is
    /// this one's.
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
        let center = Point {
            x: scale(self.center.x, self.frame.width, frame.width),
            y: scale(self.center.y, self.frame.height, frame.height),
        };
        // The old centre is at most the old side, so the scaled one is at
        // most the new side: rounding cannot pass it, as the old side
        // scales to exactly the new one. The frame has no zero side either,
        // so this never gives `None`.
        let mut focus = Self::new(frame, center)?;
        if let Some(size) = self.size {
            let width = scale(size.width, self.frame.width, frame.width).max(1);
            let height = scale(size.height, self.frame.height, frame.height).max(1);
            let area = Size {
                width: trim(width, center.x, frame.width),
                height: trim(height, center.y, frame.height),
            };
            // The trimmed area fits, so `with_size` refuses it only for the
            // zero side a centre on an edge leaves, and the focus then keeps
            // its point without an area.
            focus = focus.with_size(area).unwrap_or(focus);
        }
        if let Some(in_focus) = self.in_focus {
            focus = focus.with_in_focus(in_focus);
        }
        Some(focus)
    }
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
/// `u32`s. `old` must not be zero, for it divides: here it is a side of a
/// [`Focus`]'s frame, which never is, and the Nikon MakerNote's reader,
/// which sizes a crop's window to a JPEG's shape with it, refuses a JPEG
/// side of zero first. A result too large for a `u32`, which a value on
/// the old frame cannot give but a window's side can, saturates, at
/// `u32::MAX`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{point, size, z9_focus};

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
        // `with_in_focus` never called is `None`.
        let all = [
            (unconfirmed.with_in_focus(true), Some(true)),
            (unconfirmed.with_in_focus(false), Some(false)),
            (unconfirmed, None),
        ];
        for (focus, in_focus) in all {
            assert_eq!(focus.frame(), size(8_256, 5_504));
            assert_eq!(focus.center(), point(4_905, 2_461));
            assert_eq!(focus.size(), Some(size(291, 323)));
            assert_eq!(focus.in_focus(), in_focus);
        }
        let without_size = Focus::new(size(6_000, 4_000), point(3_000, 2_000)).unwrap();
        assert_eq!(without_size.size(), None);
        assert_eq!(without_size.in_focus(), None);
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
}
