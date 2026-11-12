// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Nikon MakerNote: where it lies in the file, where it keeps its small
//! preview JPEG, and what it records of the picture that the standard tags
//! lack.
//!
//! The MakerNote is a block of Nikon's own metadata, the value of the
//! MakerNote tag of the Exif IFD. Among much else, it points at a
//! PreviewIFD, a small IFD whose JPEGInterchangeFormat and
//! JPEGInterchangeFormatLength declare a small JPEG, 640 by 424 pixels or
//! smaller. Every Nikon body since the D70 writes one, and on the oldest
//! bodies it is the file's only JPEG.
//!
//! It also records the visible image of a raw file, which no standard tag
//! of a NEF records: the part of the stored raw data that a viewer shows,
//! without the border pixels around it. Its CropArea, which the D850, the
//! D780 and the Z bodies write in their raw files, is that rectangle: on the
//! Z 9, the 8256 by 5504 pixels that start 12 columns and 8 rows into the
//! 8280 by 5520 raw, as large as the camera's full-size preview (public
//! samples 5146 to 5148). It is four numbers, the left column, the top
//! row, the width and the height, in stored raw pixels. It gives the image
//! size only when it holds exactly four SHORTs or LONGs, its width and
//! height are above zero, and it lies inside the raw size the standard tags
//! give; and never when the raw IFD records a default crop, a standard tag,
//! which comes first (ADR011). So it never gives a JPEG's image size: a
//! JPEG holds no raw data, so there is no raw size for CropArea to lie
//! inside, and its picture is its own pixels, of the size its frame header
//! gives (ADR014). The Z 9's own JPEGs hold no CropArea; a MakerNote
//! copied from a raw file into an exported JPEG does.
//!
//! Older bodies record three more values only here, in plain form: Nikon
//! has written them in the standard Exif tags only since the D780 and the
//! Z 6 (public samples 3827 and 3587). SerialNumber is the body's serial
//! number, ASCII text, seven digits on every checked body from the D300 on
//! (sample 928) but the Nikon 1 J1, which writes eight (sample 2956); the
//! D200 writes seven spaces, which are none (sample 8079). ISO holds two
//! SHORTs, of which the second is the ISO, and gives the ISO of the D100
//! and the D70, which write none in the Exif tags (samples 2996 and 2060);
//! the D700 writes four zero bytes there instead, which are none (sample
//! 2255). Lens is the lens's range, four RATIONALs in the order of the
//! Exif LensSpecification, on every checked body with interchangeable
//! lenses. Each is used only when the standard tags give no value for it:
//! an Exif value always wins, even where the MakerNote records another. A
//! lens the Exif tags name but give no range for, such as the Nikon 1 J1's
//! (sample 2956), takes the MakerNote's range and keeps its name; an Exif
//! range with any value known is kept whole.
//!
//! WorldTime is the time zone and daylight-saving setting of the camera's
//! clock, and so gives the capture time's offset from UTC on bodies whose
//! Exif tags record none. It is four bytes: the zone in minutes east of
//! UTC, a signed 16-bit number in the MakerNote's byte order; a byte that
//! is 1 when daylight saving time is on and 0 when it is off; and one for
//! how the camera shows dates, which is not read. The offset is the zone,
//! plus 60 minutes when daylight saving time is on. On every public sample
//! that also records the Exif OffsetTimeOriginal, the two agree, with
//! daylight saving time off (samples 1835, 3827, 3587 and 5146 to 5148)
//! and on (samples 6887 and 7808 to 7810). The bodies before the D850
//! write no Exif offset, and most of them from the D300 on write
//! WorldTime: the D300, D700, D7000, D750, D810, Nikon 1 J1 and Coolpix A
//! (samples 928, 2255, 961, 895, 2036, 2956 and 2238); the D1H, D100, D70,
//! D200, Coolpix E5700 and P7800 do not. WorldTime gives an offset only
//! when it is of type UNDEFINED and exactly four bytes long, its
//! daylight-saving byte is 0 or 1, and the offset lies within 14 hours of
//! UTC, the bound an Exif offset is held to: a zone that editing software
//! has rewritten in the wrong byte order reads as hundreds of hours, 15360
//! minutes for 60. It is used only for a capture time the standard tags
//! give, when they give no offset for it: OffsetTimeOriginal is missing,
//! or cannot be read. An Exif offset always wins, even where WorldTime
//! records another.
//!
//! AFInfo2 records where the camera's autofocus aimed, which no standard
//! tag records. Mirrorless bodies, and DSLRs in live view, fill in a
//! position in pixels: the size of a frame, the center of the focus area
//! in that frame, and the area's width and height. A DSLR shot through the
//! viewfinder records a zero frame and area instead, and names its points
//! by number, which only a table of the body's points, carried by no file,
//! places in pixels (public samples 928 and 1835): a body with a table
//! gives them placed from it, as a later paragraph says, and any other
//! body no focus point. The layout of the body's points, which every file
//! of a body whose table is in raw-files ([`points`]), the D600, has
//! whatever the shot
//! ([`Metadata::focus_layout`](crate::Metadata::focus_layout)), is not
//! this reader's to give: detection gives it from the Model alone,
//! whatever the MakerNote holds, even when it cannot be read.
//! The position is the center of the area, not its corner: the Coolpix A,
//! focusing at the middle of its 4928 by 3264 frame, records (2464, 1632)
//! (sample 2238). The frame is the camera's full-size output, not the
//! stored raw: 8256 by 5504 on the Z 9, inside an 8280 by 5520 raw
//! (samples 5146 to 5148), and the full 6048 by 4024 on the Z 6's small
//! raw, whose stored image is 4548 by 3030 (sample 3587); the point is
//! reported in that frame, as recorded. AFInfo2 is a value of type
//! UNDEFINED that starts with a version, four ASCII digits, and each
//! version keeps its fields at offsets of its own, as ExifTool's tables
//! publish them. It is read at the exact versions 0100, 0101, 0300, 0301,
//! 0400, 0401 and 0402; any other, such as the Nikon 1's 0200, gives no
//! focus point, since its fields may lie elsewhere (ADR012). The frame,
//! the center and the area are SHORTs in the MakerNote's byte order. On
//! every version read but 0300 and 0301, a byte says whether the camera
//! confirmed focus: 1 for yes, 0 for a miss, and any other value says
//! nothing. On every version read, byte 5 records the area mode, how the
//! camera chose its points, by a code read with the detection method at
//! byte 4: the codes that frames shot in a known mode confirmed, under the
//! method recorded, name a [`FocusAreaMode`] ([`area_mode`]), and any other
//! names none. A recorded position is given only when the value is at
//! least as long as its version's layout, the frame and the area have no
//! zero side, the center lies on the frame, and the area fits inside the
//! frame around it ([`Focus::new`], [`Focus::with_size`]); it is given as
//! a list of one point, the primary, with the area mode ([`FocusPoints`]).
//! FocusMode, ASCII text, records how the camera focused: `MANUAL`, or an
//! autofocus mode padded with spaces, such as `AF-S  `. When it reads
//! exactly `MANUAL`, by the text rule, the file gives no focus point,
//! whatever AFInfo2 holds: the point of a manual-focus shot is not one the
//! camera focused on (ADR012).
//!
//! On a crop mode, DX, 1:1 or 16:9, the point is not reported as recorded,
//! for the frame AFInfo2 records stays the camera's full output while the
//! picture is cut from it: the Z6III's 1:1 file records a 6048 by 4032
//! frame for a picture of 4032 by 4032 (public sample 7808), and its 16:9
//! and DX files and the Z50II's 1:1 and 16:9 files do the same (samples
//! 7809, 7810, 7767 and 7770). What the values mean on the cropped picture
//! depends on what the crop cut. A DX crop, which cuts both sides, records
//! fractions of the cropped picture; a crop that keeps one side, 1:1 or
//! 16:9, records positions in the full frame. Shots made with my Z 9, with
//! the point in the top-left corner of the picture, showed it, with the
//! same single point of 291 by 323 at (502, 424) in FX. The DX shot records
//! (636, 572) and an area of 420 by 468, which, scaled onto its 5392 by
//! 3592 picture, are (415, 373) and 274 by 305, near the corner and about
//! the FX area. The 1:1 shot records (1797, 424), which is (421, 424) in
//! its 5504 by 5504 window of the frame, and the 16:9 shot (502, 715),
//! which is (502, 283) in its 8256 by 4640 window, both with the FX area.
//! CropHiSpeed tells a crop apart: its cropped size differs from its
//! original size on a crop, and equals it on every other file checked but
//! the D810's, below. The record is checked first as recorded, in the full
//! frame, as on any file: one that does not hold there gives no focus point
//! on a crop either. On a crop, the frame becomes CropArea's size. A DX
//! crop's point is scaled onto it as [`Focus::scaled_to`] scales: the
//! center and the area are each rounded to the nearest pixel, a half
//! rounding up. An area that rounding leaves a pixel too wide for its
//! center is trimmed to fit, and an area side that rounds to nothing is
//! kept at one pixel. A center that rounds onto an edge of the picture
//! keeps the point without an area. A 1:1 or 16:9 crop's center moves
//! back by the window's origin, half the difference between the frame and
//! CropArea, keeping the area. That window holds only when CropArea's
//! kept side equals the frame's: a small raw's CropArea counts stored
//! pixels, smaller than the full output, so a 1:1 or 16:9 crop on one
//! gives no focus point, while a DX crop on one is scaled onto its
//! CropArea. A raw file whose CropHiSpeed records a crop but that has no
//! CropArea is read as recorded: the D810 writes a cropped width four
//! pixels short of its original on an uncropped file, and no CropArea
//! (sample 2036); one whose CropArea cannot be used, or cannot be checked
//! against a raw size, gives no focus point. A CropHiSpeed that fits
//! neither shape, cutting both sides or keeping exactly one, such as one
//! with a cropped side larger than its original, gives none too.
//! CropHiSpeed is read only when it holds exactly seven SHORTs; otherwise
//! the file is read as no crop (ADR012).
//!
//! A JPEG shot in a crop mode records the same AFInfo2 and CropHiSpeed as
//! the raw file of the same shot, with the frame still the full output at
//! every image size, and no CropArea: shots made with my Z 9 showed
//! it, in each crop mode and at each of its three image sizes. Its
//! picture is the JPEG's own pixels, of the size its frame header gives,
//! which the metadata's image size holds, and its point is placed on them
//! as a raw file's is placed on CropArea (ADR015). A DX crop's point is
//! scaled onto the JPEG's size, which becomes the frame. A 1:1 or 16:9
//! crop's picture is the centered window of the recorded frame that has the
//! frame's length on the side the crop keeps and the JPEG's shape: its cut
//! side is the kept length times the JPEG's cut side over its kept side,
//! rounded to the nearest pixel, a half rounding up. The center moves back
//! by that window's origin, the area is kept, and the window becomes the
//! frame, as with CropArea; a window longer than the frame on the cut side
//! gives no focus point. A JPEG whose MakerNote holds a CropArea is read by
//! the raw file's rule: with no raw size to check CropArea against, a crop
//! gives no focus point, and a file that is no crop gives its point as
//! recorded. The format's reader says which of the two a file's picture
//! is, a raw file's or a JPEG's ([`Picture`]): a crop without a CropArea
//! is placed on the JPEG's pixels, and a raw file, which has none of its
//! own, gives its point as recorded. Whether the point placed so fits the
//! JPEG's picture is for the JPEG reader to judge (ADR014).
//!
//! A record whose frame or area has a zero side records no position, and
//! the points it names are placed from the table of the body the file's
//! Model names, by the decision to place a point that the camera names by
//! number from the body's table (ADR018). They are read on version 0100
//! alone, where the D600's frames checked them: the layout code
//! (FocusPointSchema) at byte 6, the primary point's number at byte 7 and
//! the mask of the points used in the seven bytes 8 to 14, its bit k being
//! bit k % 8 of byte 8 + k / 8; the other versions keep them at offsets of
//! their own, which are not read. Points are placed only when FocusMode is
//! not `MANUAL`, the version is 0100, the Model has a table, the layout
//! code is the table's, 3 on the D600, and the primary's number is a point
//! of the table, 1 to 39 on the D600; any other such record gives no focus
//! point. The focus point is then the primary's center and box, as the
//! table gives them, in the table's frame, from a table
//! ([`FocusOrigin::Table`]), with nothing said of whether focus was
//! confirmed: version 0100's in-focus byte is the contrast-detect result,
//! 0 on every viewfinder frame of the D600's, in focus or not. The list
//! holds every point of the table whose bit the mask sets, by the table's
//! bit order, number − 1 on the D600, and the primary, whether the mask
//! sets its bit or not, in number order, each with its number; a set bit
//! the table lacks is skipped, and an empty mask gives the primary alone.
//! The area mode is read as for a position. On a crop, whose CropHiSpeed
//! records a cropped size that differs from its original size, points are
//! placed only when the table's record confirms the crop's setting: the
//! DX crop, 2, on the D600. Then the frame is the crop's window, of the
//! size the record measured, 3936 by 2624 on a D600 DX frame, centered in
//! the table's frame, so that its origin is (1040, 696); every center
//! moves back by that origin and every box is kept, by the decision to
//! measure a placed point on a crop in the crop's window (ADR018).
//! CropHiSpeed's origin, its sixth and seventh values, must be the
//! window's, as on every D600 DX frame; a file whose origin is not
//! records the crop elsewhere in the frame and gives no focus point, by
//! the decision to keep each crop's window in the table (ADR018). A point
//! whose center or box the window leaves out is dropped, and when the
//! primary is, the file gives no focus point. In a JPEG the window must be
//! the JPEG's own size, as it is on the D600 at full image size, so that a
//! crop-mode JPEG at a smaller size gives none. An uncropped JPEG takes the
//! table's frame, which the JPEG reader holds to the picture's shape,
//! unless the table lists its size as one whose picture shows a window of
//! the frame: the D600's Medium JPEG, 4512 by 3008, shows the 6010 by 4006
//! window centered in the 6016 by 4016 frame, as I measured it by fitting
//! the Medium JPEGs of RAW+JPEG pairs onto their NEF's full-size preview.
//! Its points are then placed as on a crop: the frame is that window, every
//! center moves back by its origin, (3, 5), and a point the window leaves
//! out is dropped, by the decision to keep a JPEG size's window in the
//! table (ADR018). Its Large and Small JPEGs, 6016 by 4016 and 3008 by
//! 2008, show the whole frame, and take it. CropArea plays no part in it.
//! A record of a position is read by the rules above, whatever it names.
//!
//! The MakerNote also records how the camera was set for the picture,
//! beyond what the Exif tags record (ADR011): the white balance setting
//! itself, where Exif says only auto or the kind of light; the color
//! temperature of the white balance the camera applied; the
//! photographer's fine-tuning of it; the shutter used; the part of the
//! sensor the picture uses; the raw size the camera was set to; and the
//! color space of its JPEG output. Each comes from one tag, of the exact
//! type and count its rule names, and a code or a text gives a value only
//! where a checked file, or a standard, shows what it means; anything else
//! costs that value alone. The meaning of each code is ExifTool's, read
//! in its published tag documentation and confirmed on the public samples
//! and on frames I shot for these values with a Z 9 and a D600, as
//! the NEF notes record them (`docs/formats/nef/`, section 8.7 of the
//! notes on the MakerNote).
//!
//! WhiteBalance is ASCII text, an upper-case word padded with spaces, read
//! by the text rule. Its words cannot be guessed from one another, `COOL
//! WHT FL` for a cool white fluorescent lamp, so each is looked up whole
//! in a table of the texts checked files hold. The public samples show
//! `AUTO`, `AUTO0`, `AUTO1`, `SUNNY`, `FLASH` and `SHADE`; my
//! own files add `CLOUDY`, `INCANDESCENT`, `PRESET` and `COOL WHT
//! FL`; and the frames shot on the Z 9 and the D600 add `AUTO2`,
//! `NATURAL AUTO` and `PRESET1` to `PRESET4`, with Exif's white balance
//! agreeing with each. A color temperature the photographer chose is
//! written as its number, `3230K` on both bodies, which no table could
//! list, so it is read by its form: one to five digits and an upper-case
//! `K`. A text the table lacks, such as one of the Z 9's preset slots 5
//! and 6, which no checked file shows, gives way to the white balance the
//! Exif tags record, never to a guess from its letters.
//!
//! ColorTemperatureAuto is one SHORT, in kelvin: the color temperature of
//! the white balance the camera applied. The D5, the D850, the D780 and
//! the Z bodies write it, 3050 to 7000 under auto on 17 public samples;
//! my Z 9 writes it under every setting, 5210 under daylight, 5200
//! under a preset and 3230 under a chosen 3230 K; the D850 and the Z
//! fc write 0 under daylight (samples 1835 and 8559), which records
//! none. The bodies before them write none, and the WhiteBalance text is
//! all they record of a temperature: the number of a chosen one, `3230K`
//! on my D600. So when ColorTemperatureAuto gives none, the color
//! temperature is that number, when the text records one; on the Z 9,
//! which writes both, they agree. A color temperature is given only
//! from 1 to 99,999 K, the largest number the text's five digits hold.
//!
//! The fine-tuning is a shift of the white balance along two axes, amber
//! to blue and green to magenta, which Nikon writes in two tags. 0x003F,
//! two SRATIONALs, from the D810 on, is `0/4, 0/4` on every public sample;
//! 0x000B, SSHORTs, holds two values on the bodies from the D300 to the
//! Z fc and one on the D1 family, the D70, the D100 and the D200. On the
//! Z 9, 0x003F holds `-12/4, 8/4` on a frame whose picture is warmer and
//! greener than the one shot without a shift, and `8/4, -8/4` on a cooler
//! and more magenta one; on the D600, 0x000B holds `3, 0` on a frame
//! cooler than the one at `-3, 0`, and `0, 3` on one greener than the one
//! at `0, -3`. So the first value runs from amber, negative, to blue,
//! positive, and the second from magenta, negative, to green, positive.
//! [`FineTune`] signs its green to magenta shift positive towards magenta,
//! so the second value is turned around, its numerator negated. 0x003F
//! gives the fine-tuning, as stored, and 0x000B, read as whole steps, only
//! when 0x003F gives none. A 0x000B of one value gives none, since no file
//! shows its axis or its sign; and a fraction over zero, which no file
//! holds, gives none, since it records no shift, a choice of this reader's.
//!
//! ShutterMode is one SHORT, a code for the shutter used: 0 mechanical, on
//! the D850, the D780 and the Z f; 16 electronic, on the Nikon 1 J1 and
//! the Z 9; and the auto setting's 80, which chose the mechanical shutter,
//! on the Z5II, and 81, which chose the electronic front curtain, on the
//! Z 6, the Z 30, the Z50II, the Z6III and the Z fc. 64, a movie's
//! electronic shutter, gives none, and so do 48 and 96, which ExifTool's
//! documentation lists but no checked file holds.
//!
//! The image area is the crop setting, CropHiSpeed's first value, read
//! only when CropHiSpeed holds exactly seven SHORTs: 11, FX uncropped, on
//! the D750, the D810, the D5, the D850, the D780, the Z 6, the Z 9 and
//! the Z f; 12, DX uncropped, on the D5600, the Z 30 and the Z fc; 2, a DX
//! crop, on the Z6III's DX file and the D600's DX frame; 17 and 18, the
//! 1:1 crops of FX and DX; 6, the 16:9 crop; 9, which ExifTool's
//! documentation names a DX movie's 16:9 crop, on the Z50II's 16:9 still
//! and a D7100's; and 1 and 10, the 1.3x crop and the 1.3x movie crop, on
//! my D7200 files, stills all of them. 0, "off", gives none: the D700,
//! FX, and the D300, DX, both write it, so it does not say which
//! sensor the body has.
//!
//! ImageSizeRAW is one SHORT, the raw size the camera was set to: 1 large,
//! on 19 of the 22 public samples that write it, and 3 small, on the other
//! three, small raws (samples 1835, 2036 and 3587). No checked raw file is
//! a medium one, so 2 gives none. In a JPEG, ImageSizeRAW follows the
//! JPEG's own size, as my Z 9 shots taken as a NEF and a JPEG
//! together show, so a JPEG's picture ([`Picture::Jpeg`]) gives none:
//! a JPEG holds no raw data (ADR014).
//!
//! ColorSpace is one SHORT, the color space of the camera's JPEG output:
//! 1 sRGB and 2 Adobe RGB, which the D200 and every later body but the
//! Coolpix P7800 write; 4, BT.2100, gives none. It is read only when the
//! Exif IFD holds no ColorSpace, as no checked NEF file does: a program
//! that rewrites a JPEG may set Exif's to "uncalibrated" and leave the
//! camera's MakerNote behind, as on 10 of my rewritten JPEGs, so
//! where Exif records one, it alone decides. An Exif entry the TIFF
//! reader skips as damaged, of an unknown type, of a length past 32
//! bits or with its value outside its block, the file or a JPEG's Exif
//! segment, counts as none, as any damaged value does (ADR011).
//!
//! Nikon records no electronic front curtain setting apart from the
//! shutter used, so that value stays `None`.
//!
//! Nikon has used three layouts for the block, told apart by their first
//! bytes, and two of them are read. The one every body since the D70 and
//! the D100 writes, the embedded layout, starts with the signature `Nikon`
//! and a NUL, two version bytes and two more; then, 10 bytes in, comes a
//! TIFF header of its own: a byte-order mark, 42 and the offset of the
//! MakerNote IFD. That inner header is the base of every offset inside the
//! block, the MakerNote IFD's, the PreviewIFD's and the JPEG's, never the
//! start of the file, and its byte order, which may differ from the
//! file's, is the order of every number inside the block. So the offsets
//! are the same wherever the block lies, in a NEF file or in a JPEG file's
//! Exif segment: the header of the TIFF structure that holds the MakerNote
//! is not their base, as it is for Sony's.
//!
//! The D1 family's block, the headerless layout, has no header at all: it
//! is a bare IFD, the MakerNote IFD, at its first byte. Its offsets, the
//! PreviewIFD's and the JPEG's among them, count from the header of the
//! TIFF structure that holds the MakerNote, as Sony's do: the start of the
//! file in a NEF file, the TIFF header inside the Exif segment in a JPEG
//! file. Its numbers are in that structure's byte order. So, unlike the
//! embedded layout's, its offsets depend on where the block lies, and the
//! block is read with the base and the byte order of the Exif IFD, which
//! the walk read in that structure ([`MakerNoteBlock::ifd_at`]). The D1H
//! writes it, big-endian, file and block alike: its MakerNote IFD starts
//! at the block's first byte, and its PreviewIFD and that IFD's JPEG, the
//! file's only one, 570 by 375 pixels, lie inside the block where offsets
//! counted from the start of the file put them (public sample 5364). Early
//! Coolpix bodies, the E990 and others, write it into their JPEGs,
//! little-endian like the Exif segment, with offsets counted from the
//! segment's TIFF header, as camera JPEGs from Wikimedia Commons, checked
//! by hand, show; where they hold a PreviewIFD, its JPEG lies past the end
//! of the block, so it is not a preview.
//!
//! The third layout, an IFD after the 8 bytes `Nikon\0\1\0`, which the
//! Coolpix E700 to E950 wrote into their JPEGs, is not read: the sources
//! disagree on the byte order of its numbers, and the public files that
//! have it, little-endian inside the block and out, cannot settle it. A
//! file that has one gets nothing from its MakerNote.
//!
//! A block that starts with the signature is of the embedded layout or of
//! none: it is never read as a bare IFD. Any other block is taken for the
//! headerless layout. The file's Make is not looked at here: the format's
//! reader has already decided by it that the file is Nikon's, the NEF
//! reader in its claim and the JPEG reader by IFD0's Make, both by the
//! name [`NIKON`]. So a block of another maker's under a Nikon Make, or
//! one that is no IFD at all, is read as a bare IFD, under the same
//! checks: its first two bytes, taken for the number of entries, are in
//! the thousands for a block that starts with text, more than the limits
//! or the block allow, and whatever passes is held to the block and to
//! each tag's rule, as below.
//!
//! Nothing in the block is trusted. Every offset found inside it is checked
//! against the block, not only against the file: the MakerNote IFD and the
//! PreviewIFD must lie inside the block, with their whole entry tables; the
//! three numbers that lead to the JPEG, the PreviewIFD's offset and the
//! JPEG's offset and length, must each be one LONG, as Nikon writes them,
//! which the entry holds itself ([`Ifd::held`]); and the whole JPEG, as
//! declared, must lie inside the block, whichever header the offsets count
//! from. The block's IFDs are read inside the block ([`ifd_in`]): every
//! entry whose value lies outside the block is dropped, as an entry whose
//! value lies outside the file is, so every value read from them lies
//! inside the block: the ISO's two numbers, WorldTime's four bytes, the
//! settings of one SHORT and 0x000B's two SSHORTs, which the entry holds,
//! and CropArea's four numbers, CropHiSpeed's seven, the serial number, the
//! lens's range, FocusMode's and WhiteBalance's texts, 0x003F's two
//! fractions and AFInfo2's bytes, which take more than the four bytes an
//! entry holds and so are stored elsewhere. A value is read only in the
//! type and count its tag's rule names, which the reader declares once,
//! beside the tag's number ([`Rule`](crate::container::tiff::Rule)); a
//! value of another type or count is never read ([`Ifd::get`]). A
//! block that does not hold together, whether it is too short for its
//! layout, its inner header is not one, or an IFD in it is cut short or has
//! more entries than the limits allow, costs what the MakerNote would have
//! given, the small preview, the crop area, the camera settings and the
//! values above, never the file: the NEF security notes ask that bad
//! entries be skipped rather than fail the file
//! (`docs/formats/nef/security.md`, item 4 of the checklist).
//! Only a failure of the reader itself is an error: that is not the file's
//! fault, and says nothing about the MakerNote.
//!
//! The MakerNote is read once per file, into a [`MakerNote`]: its block
//! found and checked, its MakerNote IFD and its PreviewIFD parsed. Every
//! use of the MakerNote then reads from that same structure, so that none
//! of them finds, checks or parses the block again: the preview, the
//! metadata, and the raw view, which lists both IFDs' entries as the
//! directories `MakerNote` and `MakerNoteSub(0x0011)`
//! ([`Directory`]).
//!
//! [`FocusAreaMode`]: crate::focus::FocusAreaMode
//! [`area_mode`]: af_info2::area_mode
//! [`Focus::new`]: crate::focus::Focus::new
//! [`Focus::with_size`]: crate::focus::Focus::with_size
//! [`Focus::scaled_to`]: crate::focus::Focus::scaled_to
//! [`FocusOrigin::Table`]: crate::focus::FocusOrigin::Table
//! [`FineTune`]: crate::camera_settings::FineTune
//! [`MakerNoteBlock::ifd_at`]: super::MakerNoteBlock::ifd_at
//! [`Ifd::held`]: crate::container::tiff::Ifd::held
//! [`ifd_in`]: crate::container::tiff::ifd_in
//! [`Ifd::get`]: crate::container::tiff::Ifd::get
//! [`Directory`]: crate::entries::Directory

mod af_info2;
mod block;
mod crop;
mod metadata;
mod settings;
#[cfg(test)]
mod testing;

pub(in crate::formats) use block::{MakerNote, preview_candidate, read};

use std::io::{Read, Seek};

use crate::error::Result;
use crate::focus::FocusPoints;
use crate::formats::makernote::points::{self, PointTable};
use crate::formats::tiff_metadata::{self, Standard};
use crate::geometry::Size;
use crate::io::Source;
use crate::metadata::Metadata;
use af_info2::af_info2_record;
use crop::{CropHiSpeed, crop_hi_speed, in_crop, placed};
use metadata::{CropArea, crop_area, iso, lens_range, serial_number, world_time};
use settings::camera_settings;

/// The Nikon MakerNote tag that records the ISO the picture was taken at
/// (ISO): two SHORTs, the second of them the ISO, on the bodies whose Exif
/// tags may lack one.
const ISO: u16 = 0x0002;

/// The Nikon MakerNote tag that records the white balance setting
/// (WhiteBalance), as ASCII text: an upper-case word, such as `AUTO1` or
/// `SUNNY`, padded with spaces, or a chosen color temperature as its
/// number, such as `3230K`.
const WHITE_BALANCE: u16 = 0x0005;

/// The Nikon MakerNote tag that records how the camera focused
/// (FocusMode), as ASCII text: `MANUAL` for manual focus, or an autofocus
/// mode padded with spaces to six characters, such as `AF-S  `.
const FOCUS_MODE: u16 = 0x0007;

/// The Nikon MakerNote tag that records the white balance fine-tuning in
/// whole steps (WhiteBalanceFineTune): SSHORTs, two of them, amber to blue
/// then magenta to green, on the bodies from the D300 to the Z fc, beside
/// 0x003F ([`FINE_TUNE`]) from the D810 on; one, whose axis and sign no
/// file shows, on the D1 family, the D70, the D100 and the D200.
const FINE_TUNE_STEPS: u16 = 0x000B;

/// The Nikon MakerNote tag that points at the PreviewIFD, the sub-IFD
/// holding the small preview JPEG.
const PREVIEW_IFD: u16 = 0x0011;

/// The Nikon MakerNote tag that records the crop mode the picture was
/// taken in, and how large the picture was before and after the crop
/// (CropHiSpeed): seven SHORTs, the mode, the original width and height,
/// the cropped width and height, and the left column and top row where
/// the crop starts.
const CROP_HI_SPEED: u16 = 0x001B;

/// The Nikon MakerNote tag that records the camera body's serial number
/// (SerialNumber), as ASCII text.
const SERIAL_NUMBER: u16 = 0x001D;

/// The Nikon MakerNote tag that records the color space the camera was
/// set to for its JPEG output (ColorSpace): one SHORT, 1 sRGB, 2 Adobe RGB.
const COLOR_SPACE: u16 = 0x001E;

/// The Nikon MakerNote tag that records the time zone and daylight-saving
/// setting of the camera's clock (WorldTime): four UNDEFINED bytes, the
/// zone in minutes east of UTC as a signed 16-bit number in the
/// MakerNote's byte order, then 1 when daylight saving time is on and 0
/// when it is off, then the date display format.
const WORLD_TIME: u16 = 0x0024;

/// The Nikon MakerNote tag that records which shutter took the picture
/// (ShutterMode): one SHORT, a code for the shutter, and on an auto
/// setting for the shutter the camera chose.
const SHUTTER_MODE: u16 = 0x0034;

/// The Nikon MakerNote tag that records the raw size the camera was set to
/// (ImageSizeRAW): one SHORT, 1 large, 2 medium, 3 small. In a JPEG it
/// follows the JPEG's own size instead.
const IMAGE_SIZE_RAW: u16 = 0x003E;

/// The Nikon MakerNote tag that records the white balance fine-tuning in
/// the camera's own steps, from the D810 on (WhiteBalanceFineTune): two
/// SRATIONALs, amber to blue then magenta to green, fractions of 4 on
/// every file checked.
const FINE_TUNE: u16 = 0x003F;

/// The Nikon MakerNote tag that records the visible image, the rectangle
/// of the stored raw data inside its border (CropArea): its left column,
/// top row, width and height.
const CROP_AREA: u16 = 0x0045;

/// The Nikon MakerNote tag that records the color temperature of the
/// white balance the camera applied, in kelvin (ColorTemperatureAuto): one
/// SHORT, 0 when none was recorded.
const COLOR_TEMPERATURE: u16 = 0x004F;

/// The Nikon MakerNote tag that records the lens's range (Lens): its
/// shortest and longest focal length and the f-number at each, as four
/// RATIONALs in the order of the Exif LensSpecification.
const LENS: u16 = 0x0084;

/// The Nikon MakerNote tag that records where the camera's autofocus
/// aimed (AFInfo2): an UNDEFINED value that starts with its version, four
/// ASCII digits, and holds, at offsets each version sets, the size of a
/// frame, the center of the focus area and the area's width and height,
/// as SHORTs in the MakerNote's byte order, and on most versions a byte
/// saying whether focus was confirmed.
const AF_INFO2: u16 = 0x00B7;

/// The rules of the tags this reader reads: each tag's number, from the
/// constants above, and the type and count Nikon writes its value in on
/// every file checked, which a value must have to be read (ADR011).
/// [`Ifd::get`] and [`Ifd::held`] read them. Two are tags of the
/// PreviewIFD, which Nikon writes as one LONG each.
///
/// [`Ifd::get`]: crate::container::tiff::Ifd::get
/// [`Ifd::held`]: crate::container::tiff::Ifd::held
mod rules {
    use crate::container::tiff::tags;
    use crate::container::tiff::{FieldType, Held, Rule, Text};
    use crate::rational::{Rational, SignedRational};

    const SHORT: &[FieldType] = &[FieldType::Short];
    const LONG: &[FieldType] = &[FieldType::Long];
    const ASCII: &[FieldType] = &[FieldType::Ascii];

    pub(super) const ISO: Held<[u32; 2]> = Held::new(super::ISO, SHORT);
    pub(super) const WHITE_BALANCE: Rule<Text> = Rule::new(super::WHITE_BALANCE, ASCII);
    pub(super) const FOCUS_MODE: Rule<Text> = Rule::new(super::FOCUS_MODE, ASCII);
    pub(super) const FINE_TUNE_STEPS: Held<[i16; 2]> =
        Held::new(super::FINE_TUNE_STEPS, &[FieldType::SShort]);
    pub(super) const PREVIEW_IFD: Held<[u32; 1]> = Held::new(super::PREVIEW_IFD, LONG);
    pub(super) const CROP_HI_SPEED: Rule<[u32; 7]> = Rule::new(super::CROP_HI_SPEED, SHORT);
    pub(super) const SERIAL_NUMBER: Rule<Text> = Rule::new(super::SERIAL_NUMBER, ASCII);
    pub(super) const COLOR_SPACE: Held<[u32; 1]> = Held::new(super::COLOR_SPACE, SHORT);
    pub(super) const WORLD_TIME: Held<[u8; 4]> =
        Held::new(super::WORLD_TIME, &[FieldType::Undefined]);
    pub(super) const SHUTTER_MODE: Held<[u32; 1]> = Held::new(super::SHUTTER_MODE, SHORT);
    pub(super) const IMAGE_SIZE_RAW: Held<[u32; 1]> = Held::new(super::IMAGE_SIZE_RAW, SHORT);
    pub(super) const FINE_TUNE: Rule<[SignedRational; 2]> =
        Rule::new(super::FINE_TUNE, &[FieldType::SRational]);
    pub(super) const CROP_AREA: Rule<[u32; 4]> =
        Rule::new(super::CROP_AREA, &[FieldType::Short, FieldType::Long]);
    pub(super) const COLOR_TEMPERATURE: Held<[u32; 1]> = Held::new(super::COLOR_TEMPERATURE, SHORT);
    pub(super) const LENS: Rule<[Rational; 4]> = Rule::new(super::LENS, &[FieldType::Rational]);
    pub(super) const AF_INFO2: Rule<Vec<u8>> = Rule::new(super::AF_INFO2, &[FieldType::Undefined]);

    // The PreviewIFD.
    pub(super) const JPEG_INTERCHANGE_FORMAT: Held<[u32; 1]> =
        Held::new(tags::JPEG_INTERCHANGE_FORMAT, LONG);
    pub(super) const JPEG_INTERCHANGE_FORMAT_LENGTH: Held<[u32; 1]> =
        Held::new(tags::JPEG_INTERCHANGE_FORMAT_LENGTH, LONG);
}

/// The manufacturer's name that a Nikon camera's Make starts with,
/// compared without regard to case: what a format's reader looks for in
/// IFD0's Make before it takes a file for a Nikon one, the NEF claim and
/// the JPEG reader alike.
pub(in crate::formats) const NIKON: &[u8; 5] = b"NIKON";

/// What a file's picture is, as the format that read the MakerNote knows
/// it: what the focus point of a crop mode is placed on when the
/// MakerNote holds no CropArea. [`metadata`] turns it, once, into the
/// pixels it gives [`focus`]: none for a raw file, the image size for a
/// JPEG.
///
/// [`metadata`]: fn@metadata
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::formats) enum Picture {
    /// A raw file's, such as a NEF's: the picture is the part of the stored
    /// raw data that CropArea locates inside it, and a crop's point is
    /// placed on CropArea alone. A raw file has no pixels of its own to
    /// place it on, so a crop without a CropArea is read as recorded
    /// (ADR012).
    Raw,
    /// A JPEG's: the picture is the file's own pixels, of the size its
    /// frame header gives, which the metadata's image size already holds.
    /// A crop without a CropArea is placed on those pixels (ADR015).
    Jpeg,
}

/// The metadata of the file whose MakerNote is `makernote` and whose
/// picture is of the kind `picture` names: what the standard tags gave,
/// `standard`, with what the MakerNote records that the standard tags
/// lack, the focus point among it.
///
/// That is, first, the image size, from CropArea, which replaces the raw
/// size the image size otherwise falls back on: when the raw IFD records no
/// default crop, which comes first, and when there is a raw size for the
/// crop area to lie inside ([`crop_area`]). A JPEG's reader passes no raw
/// size, so a JPEG keeps the image size `standard` gives, its picture's.
/// CropArea is read once, whether or not it gives the image size, and the
/// focus point below takes what it gives too: on a crop, a CropArea that
/// cannot be used, or that there is no raw size to check against, gives no
/// focus point. Then, each only when the standard tags gave none, the
/// serial number ([`serial_number`]), the ISO ([`iso`]), the lens's range
/// ([`lens_range`]) and the capture time's offset from UTC
/// ([`world_time`]): a range fills a lens whose standard tags gave no
/// value of its range, keeping their recorded make and model and their
/// serial number; an offset is given only to a capture time the standard
/// tags gave, one whose OffsetTimeOriginal is missing or could not be
/// read, and only within the bound every offset is held to
/// ([`DateTime::with_offset_minutes`](crate::datetime::DateTime::with_offset_minutes)).
/// Then the camera settings that only the MakerNote records, and the white
/// balance setting, finer than Exif's, in place of the mode the Exif tags
/// gave when the MakerNote names one, each by its own rule
/// ([`camera_settings`]): the color space only when the Exif IFD holds no
/// ColorSpace, and the raw image size only for a raw file's picture.
/// CropHiSpeed is read once, for the image area and the focus point.
/// The table of the body's autofocus points is found from the Model the
/// standard tags give, matched exactly as [`points::find`] matches it:
/// none for a body without a table, or a file without a Model. Then the
/// focus points, from AFInfo2 and FocusMode, with AFInfo2's
/// area mode ([`focus`]), always read: the standard tags never give one,
/// so it is not a fallback. A recorded position is placed on the cropped
/// picture by CropHiSpeed and CropArea on a crop, as a list of one point;
/// the points a record with no position names are placed from the table,
/// in the crop's window on a crop that CropHiSpeed records. `picture` says,
/// once, what a crop without a CropArea is placed on: nothing for a raw
/// file's, whose point is then read as recorded, and for a JPEG's, its own
/// pixels, of the image size `standard` gives, which a point placed on a
/// crop must have as its window, and which picks the window of a JPEG size
/// that the table lists. Every other value is the standard tags', and the
/// layout of the body's autofocus points is not given here: detection
/// gives it from the Model alone, whatever the MakerNote holds.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
pub(in crate::formats) fn metadata<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    standard: Standard,
    picture: Picture,
) -> Result<Metadata> {
    let Standard {
        mut metadata,
        from_default_crop,
        exif_records_color_space,
    } = standard;
    // Read once, for the image size and the focus point. Without a raw
    // size, as in a JPEG, CropArea is never a size, so it never replaces
    // the image size a JPEG's reader gave, the picture's own (ADR014).
    let crop = crop_area(src, makernote, metadata.raw_size)?;
    if !from_default_crop && let CropArea::Size(image) = crop {
        metadata.image_size = Some(image);
    }
    if metadata.camera.serial_number.is_none() {
        metadata.camera.serial_number = serial_number(src, makernote)?;
    }
    if metadata.exposure.iso.is_none() {
        metadata.exposure.iso = iso(makernote);
    }
    if !metadata.lens.has_range() {
        metadata.lens.set_range(lens_range(src, makernote)?);
    }
    // An offset belongs to a time: without a capture time, WorldTime gives
    // nothing.
    if let Some(time) = metadata.capture_time
        && time.offset_minutes().is_none()
        && let Some(offset) = world_time(makernote)
        && let Ok(time) = time.with_offset_minutes(offset)
    {
        metadata.capture_time = Some(time);
    }
    // Read once, for the image area and the focus point.
    let crop_hi_speed = crop_hi_speed(src, makernote)?;
    metadata.camera_settings = camera_settings(
        src,
        makernote,
        metadata.camera_settings,
        crop_hi_speed,
        exif_records_color_space,
        picture,
    )?;
    // What a crop without a CropArea is placed on: a raw file has no
    // pixels of its own for it, a JPEG has the image size its reader gave.
    let pixels = match picture {
        Picture::Raw => None,
        Picture::Jpeg => metadata.image_size,
    };
    // The table that places the points a viewfinder shot names.
    let table = metadata
        .camera
        .recorded_model
        .as_deref()
        .and_then(points::find);
    metadata.focus_points = focus(src, makernote, crop_hi_speed, crop, pixels, table)?;
    Ok(metadata)
}

/// Where the camera's autofocus aimed, as `makernote`'s AFInfo2 records
/// it, with the area mode the record names, when it names one
/// ([`area_mode`]).
///
/// A record of a position, whose frame and area have no zero side
/// ([`FocusRecord::records_position`]), gives the center of the focus area,
/// the area's size and the frame the two are measured in, with whether
/// focus was confirmed on the versions that say, placed on the cropped
/// picture when `crop`, what CropHiSpeed records, already read
/// ([`crop_hi_speed`]), says the file is a crop, by `crop_area`, what
/// CropArea gives, already read ([`crop_area`]), or, when the MakerNote
/// holds no CropArea, on `pixels`, a JPEG's own pixels, when the file has
/// them ([`in_crop`]). The point is given as a list of one, the primary.
///
/// A record with a zero side, as a shot through a DSLR's viewfinder
/// writes, gives the points it names ([`NamedPoints`]), placed from
/// `table`, the table of the body the file's Model names, when there is
/// one ([`placed`]), with their frame on a crop set by `crop` and checked
/// against `pixels`, and on an uncropped JPEG set by `pixels` when the
/// table lists that size; CropArea plays no part there.
///
/// `None` when FocusMode records manual focus ([`manual_focus`]); when
/// AFInfo2 is missing, is not of type UNDEFINED, stores its value outside
/// the block, or holds a value [`af_info2_record`] gives none for; for a
/// position, when [`in_crop`] gives none, because [`FocusRecord::focus`]
/// refuses the record as recorded or the record cannot be placed on the
/// cropped picture; and with no position, when the record names no points,
/// as every version but 0100 does here, there is no table, or [`placed`]
/// gives none.
///
/// See the [module documentation](self) for what AFInfo2 and CropHiSpeed
/// are.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
///
/// [`area_mode`]: af_info2::area_mode
/// [`FocusRecord::records_position`]: af_info2::FocusRecord::records_position
/// [`NamedPoints`]: af_info2::NamedPoints
/// [`FocusRecord::focus`]: af_info2::FocusRecord::focus
fn focus<R: Read + Seek>(
    src: &mut Source<R>,
    makernote: &MakerNote,
    crop: Option<CropHiSpeed>,
    crop_area: CropArea,
    pixels: Option<Size>,
    table: Option<&PointTable>,
) -> Result<Option<FocusPoints>> {
    let ifd = &makernote.ifd;
    // Without AFInfo2 there is nothing to place, so FocusMode is not read;
    // with manual focus there is nothing to place either, so AFInfo2 is not
    // read.
    if ifd.entry(AF_INFO2).is_none() || manual_focus(src, makernote)? {
        return Ok(None);
    }
    let Some(value) = ifd.get(src, rules::AF_INFO2)? else {
        return Ok(None);
    };
    let Some(record) = af_info2_record(&value, ifd.order()) else {
        return Ok(None);
    };
    let points = if record.records_position() {
        in_crop(record, crop, crop_area, pixels).map(FocusPoints::from)
    } else {
        record
            .named
            .zip(table)
            .and_then(|(named, table)| placed(named, table, crop, pixels))
    };
    Ok(points.map(|points| match record.area_mode {
        Some(mode) => points.with_area_mode(mode),
        None => points,
    }))
}

/// Whether `makernote`'s FocusMode records manual focus: whether it is of
/// type ASCII and reads `MANUAL`, exactly, by the rule every text value of
/// the metadata is read by ([`tiff_metadata::text`]), which cuts it at its
/// first NUL and trims the spaces that pad it. A FocusMode that is
/// missing, of another type, or that reads anything else, such as `AF-S`,
/// records no manual focus.
///
/// # Errors
///
/// `Io` when the reader fails. Nothing the block holds is an error.
fn manual_focus<R: Read + Seek>(src: &mut Source<R>, makernote: &MakerNote) -> Result<bool> {
    let mode = tiff_metadata::text(src, &makernote.ifd, rules::FOCUS_MODE)?;
    Ok(mode.as_deref() == Some("MANUAL"))
}

#[cfg(test)]
mod tests {
    use super::testing::{
        Z9, af_info2, entry_at, expected, metadata_of, nikon_file, tags_block, whole,
    };
    use super::*;
    use crate::container::tiff::tags::MAKER_NOTE;
    use crate::container::tiff::{self, walk};
    use crate::error::ErrorKind;
    use crate::limits::Limits;
    use crate::testing::{FAR, FailsFar};
    use crate::tiff_builder::{Order, UNDEFINED};
    use std::io::Cursor;

    #[test]
    fn a_reader_failure_while_reading_af_info2_is_an_error() {
        // The block placed so that its inner header and MakerNote IFD lie
        // before FAR and AFInfo2's value, its last 80 bytes, from FAR on.
        let order = Order::Little;
        let block = tags_block(order, |inner, ifd| {
            inner.undefined(ifd, AF_INFO2, &af_info2(order, "0400", Z9));
        })
        .0;
        let at = FAR - (whole(&block) - 80);
        let bytes = nikon_file(order, at, &block, 0, |builder, exif| {
            entry_at(builder, exif, MAKER_NOTE, UNDEFINED, at, whole(&block));
        });
        // The file is sound, for a reader that does not fail.
        assert_eq!(
            metadata_of(bytes.clone()).focus(),
            Some(expected(Z9, Some(true)))
        );

        let limits = Limits::default();
        let mut src = Source::new(FailsFar(Cursor::new(bytes))).unwrap();
        let file_block = tiff::Block::file(src.len());
        let tree = walk(&mut src, &file_block, &limits).unwrap();
        let makernote = read(&mut src, &tree, &limits)
            .unwrap()
            .expect("a MakerNote");
        let standard = tiff_metadata::read(&mut src, &tree).unwrap();
        let err =
            metadata(&mut src, &makernote, standard, Picture::Raw).expect_err("the read fails");
        assert_eq!(err.kind(), ErrorKind::Io, "{err:?}");
    }
}
