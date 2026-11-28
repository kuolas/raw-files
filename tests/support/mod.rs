// SPDX-License-Identifier: MIT OR Apache-2.0

//! Helpers shared by the integration tests: the modules below, and the
//! small conversions that tests of every format use.
//!
//! Cargo compiles this module only into an integration test that declares
//! `mod support;`, as most of them do. Each test uses only part of the
//! helpers, so unused ones are not warnings.

#![allow(dead_code)]

pub mod arw;
pub mod fixtures;
pub mod jpeg_builder;
pub mod jpeg_files;
pub mod jpeg_stub;
pub mod nef;
pub mod samples;
pub mod tiff_builder;
pub mod xmp_packets;

use raw_files::{Rational, Size};

/// A `(width, height)` pair of `u16`s, as the builders take a size, as a
/// [`Size`].
pub fn size_of((width, height): (u16, u16)) -> Size {
    Size {
        width: u32::from(width),
        height: u32::from(height),
    }
}

/// The fraction `numerator`/`denominator`, as built for an expected
/// value.
///
/// Panics on a zero denominator, which no expected value of the
/// integration tests holds: a mistake in the test.
pub fn fraction(numerator: u32, denominator: u32) -> Rational {
    Rational::new(numerator, denominator).expect("a nonzero denominator")
}
