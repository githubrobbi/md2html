// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The ids the page gives elements of its own, which a heading never
//! takes: a heading whose anchor would be one is numbered instead
//! (`top-1`), as a repeated heading is.

use alloc::collections::BTreeSet;

/// The page's own ids by name: its controls, its parts, and the elements
/// its scripts make. The list is part of the heading anchors' rule, and
/// `tests` pins it.
pub(super) const RESERVED: &[&str] = &[
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
];

/// The parts of a drawn diagram, after its number: its title, its
/// description and its arrowheads (`dg2-t`), and each in the copy an
/// enlarged diagram shows (`dg2-t-z`).
const DIAGRAM_PARTS: [&str; 12] = [
    "t", "d", "a", "o", "x", "v", "t-z", "d-z", "a-z", "o-z", "x-z", "v-z",
];

/// Whether `id` is one the page gives by a pattern.
///
/// A footnote's (`fn-3`), a reference's to it (`fnref-3-1`), a part of a
/// drawn diagram (`dg2-t`), a search hit (`hit-4`), a glossary entry
/// (`g-ledger`) or a mention of a term (`t-ledger-2`), for the terms of
/// the page.
pub(super) fn page_owns(id: &str, terms: &BTreeSet<&str>) -> bool {
    if let Some(number) = id.strip_prefix("fn-").or_else(|| id.strip_prefix("hit-")) {
        return is_number(number);
    }
    if let Some(rest) = id.strip_prefix("fnref-") {
        return rest
            .split_once('-')
            .is_some_and(|(note, nth)| is_number(note) && is_number(nth));
    }
    if let Some(rest) = id.strip_prefix("dg") {
        return rest
            .split_once('-')
            .is_some_and(|(number, part)| is_number(number) && DIAGRAM_PARTS.contains(&part));
    }
    if let Some(slug) = id.strip_prefix("g-") {
        if terms.contains(slug) {
            return true;
        }
    }
    id.strip_prefix("t-")
        .and_then(|rest| rest.rsplit_once('-'))
        .is_some_and(|(slug, nth)| is_number(nth) && terms.contains(slug))
}

/// Whether `text` is a number: one digit or more, and nothing else.
fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}
