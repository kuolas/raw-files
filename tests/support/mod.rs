// SPDX-License-Identifier: MIT OR Apache-2.0

//! Helpers shared by the integration tests.
//!
//! Cargo compiles this module only into an integration test that declares
//! `mod support;`; `tests/tiff_builder.rs` does, so it is built and linted
//! on every test run. Each test uses only part of the helpers, so unused
//! ones are not warnings.

#![allow(dead_code)]

pub mod jpeg;
pub mod nef;
pub mod tiff_builder;
