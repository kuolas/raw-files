// SPDX-License-Identifier: MIT OR Apache-2.0

//! Helpers shared by the integration tests.
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
