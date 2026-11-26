// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The ids the page keeps for itself, pinned.
//!
//! A heading whose text would give one of them is numbered instead
//! (`top-1`), so adding a name here changes the id of a heading somewhere:
//! a change to the heading anchors' rule (`crate::tests::anchors`,
//! `docs/policies/release_policy.md`).

use super::RESERVED;

/// The page's own ids, as they are today.
///
/// # Panics
///
/// When [`RESERVED`] gains, loses or reorders a name.
#[test]
fn the_ids_the_page_keeps_for_itself_are_pinned() {
    assert_eq!(RESERVED, [
        "top",
        "main",
        "opening",
        "toc",
        "dock",
        "edge",
        "toggle",
        "bar",
        "crumb",
        "find",
        "q",
        "hits",
        "idx",
        "theme",
        "size",
        "size-up",
        "size-down",
        "open-find",
        "glossary",
        "notes",
        "md2html-ui",
        "md2html-money",
        "money",
        "moneys",
        "hits-n",
        "aa",
        "prefs",
        "save",
        "keep",
        "print",
        "lang",
        "langs",
        "toast",
        "rowshare",
        "passage",
    ]);
}
