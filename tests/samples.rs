// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the helpers of `tests/support/samples.rs` that need no sample
//! file, from outside the crate, as the integration tests use them.
//!
//! These are the helpers' only tests. The support module is compiled into
//! every integration test that declares `mod support`, so a test kept in it
//! would run once in each of those test builds.

mod support;

use std::panic;

use support::samples::check_every;

/// The message a call panicked with, or `None` when it returned.
fn panic_of(call: impl FnOnce() + panic::UnwindSafe) -> Option<String> {
    let payload = panic::catch_unwind(call).err()?;
    Some(support::samples::panic_message(payload.as_ref()).to_string())
}

#[test]
fn check_every_passes_when_every_check_passes() {
    let items = ["a.nef", "b.nef"];
    let failure = panic_of(|| check_every(&items, |item| item, |_| {}));
    assert_eq!(failure, None);
}

#[test]
fn check_every_names_every_failing_file_with_its_message_in_order() {
    let items = ["a.nef", "b.nef", "c.nef", "d.nef"];
    let failure = panic_of(|| {
        check_every(
            &items,
            |item| item,
            |item| match *item {
                "b.nef" => panic!("the wrong model"),
                "d.nef" => panic!("the wrong ISO\n  read: {}\nlisted: {}", 100, 200),
                _ => {}
            },
        )
    })
    .expect("a panic naming the failing files");
    let mut lines = failure.lines();
    assert_eq!(lines.next(), Some("2 of 4 sample files failed:"));
    assert_eq!(lines.next(), Some("b.nef: the wrong model"));
    // A message of several lines, as an assert's is, keeps its lines, each
    // indented under the file's name.
    assert_eq!(lines.next(), Some("d.nef: the wrong ISO"));
    assert_eq!(lines.next(), Some("      read: 100"));
    assert_eq!(lines.next(), Some("    listed: 200"));
    assert_eq!(lines.next(), None);
}

#[test]
fn check_every_runs_the_checks_after_a_failing_one() {
    let items = ["a.nef", "b.nef", "c.nef"];
    let checked = std::sync::Mutex::new(Vec::new());
    let failure = panic_of(panic::AssertUnwindSafe(|| {
        check_every(
            &items,
            |item| item,
            |item| {
                checked.lock().unwrap().push(*item);
                assert_ne!(*item, "a.nef");
            },
        )
    }));
    assert!(failure.is_some());
    assert_eq!(*checked.lock().unwrap(), ["a.nef", "b.nef", "c.nef"]);
}
