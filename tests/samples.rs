// SPDX-License-Identifier: MIT OR Apache-2.0

//! Checks the helpers of `tests/support/samples.rs` that need no sample
//! file, from outside the crate, as the integration tests use them.
//!
//! These are the helpers' only tests. The support module is compiled into
//! every integration test that declares `mod support`, so a test kept in it
//! would run once in each of those test builds.

mod support;

use std::ffi::OsStr;
use std::panic;

use support::samples::{Tier, check_every, missing_fails, require_switch, tier};

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

#[test]
fn a_missing_file_fails_only_when_the_samples_are_required() {
    // (tier, RAW_FILES_REQUIRE_SAMPLES=1, RAW_FILES_LARGE=1, fails)
    let table = [
        (Tier::Normal, false, false, false),
        (Tier::Normal, false, true, false),
        (Tier::Normal, true, false, true),
        (Tier::Normal, true, true, true),
        (Tier::Large, false, false, false),
        (Tier::Large, false, true, false),
        (Tier::Large, true, false, false),
        (Tier::Large, true, true, true),
    ];
    for (tier, require, large, fails) in table {
        assert_eq!(
            missing_fails(tier, require, large),
            fails,
            "{tier:?}, require {require}, large {large}"
        );
    }
}

#[test]
fn the_switch_is_on_at_1_off_unset_or_empty_and_refuses_any_other_value() {
    assert!(!require_switch(None));
    assert!(!require_switch(Some(OsStr::new(""))));
    assert!(require_switch(Some(OsStr::new("1"))));
    for value in ["0", "true", "yes", " 1"] {
        let failure = panic_of(|| {
            require_switch(Some(OsStr::new(value)));
        });
        let expected = format!("RAW_FILES_REQUIRE_SAMPLES is {value:?}; set it to 1");
        assert!(
            failure.is_some_and(|message| message.starts_with(&expected)),
            "{value:?}"
        );
    }
}

#[test]
fn a_files_tier_is_the_one_the_list_gives() {
    assert_eq!(tier("sony-ilce-7rm3.arq"), Tier::Large);
    assert_eq!(tier("nikon-z9-he.nef"), Tier::Normal);
    // A name the list does not hold counts as normal.
    assert_eq!(tier("not-listed.nef"), Tier::Normal);
}
