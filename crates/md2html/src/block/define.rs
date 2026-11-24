// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Link reference definitions, `[label]: destination "title"`, read off
//! the front of a paragraph.
//!
//! A definition is read from the paragraph's text, its lines joined, so
//! its label and its title may each span lines, and its destination may
//! stand on the line after the label.

use crate::inline::{link_destination, link_space, link_title, normalize_label};
use crate::model::{Defs, Reference};
use crate::scan::Scan;

/// Reads the definitions that open a paragraph's `text` into `defs`;
/// gives the text after them, which starts on a line of its own. The
/// first definition of a label wins.
pub(super) fn take_definitions<'text>(text: &'text str, defs: &mut Defs) -> &'text str {
    let mut scan = Scan::new(text);
    while let Some((label, reference)) = definition(&mut scan) {
        defs.links
            .entry(normalize_label(label))
            .or_insert(reference);
    }
    scan.rest()
}

/// Whether a paragraph's `text` is nothing but definitions, so that no
/// paragraph is left of it.
pub(super) fn only_definitions(text: &str) -> bool {
    let mut scan = Scan::new(text);
    while definition(&mut scan).is_some() {}
    scan.is_done()
}

/// Reads the definition at the front of `scan`, up to the end of the line
/// it ends on: its label and where it points.
fn definition<'text>(scan: &mut Scan<'text>) -> Option<(&'text str, Reference)> {
    let mut ahead = scan.clone();
    if !ahead.eat('[') {
        return None;
    }
    let label = label(&mut ahead)?;
    // A label longer than a label may be is no label (`label` gives none):
    // the line is text.
    if label.trim().is_empty() || label.starts_with('^') || !ahead.eat(':') {
        return None;
    }
    link_space(&mut ahead);
    let href = link_destination(&mut ahead)?;
    // The title stands apart from the destination, on its line or the
    // next, and nothing but spaces follows it; without one, the line ends
    // after the destination.
    let title = titled(&mut ahead);
    if title.is_none() && !line_ends(&mut ahead) {
        return None;
    }
    *scan = ahead;
    Some((label, Reference { href, title }))
}

/// Reads a definition's title, the white space before it and the end of
/// its line, when they are there.
fn titled(scan: &mut Scan<'_>) -> Option<String> {
    let mut ahead = scan.clone();
    if !link_space(&mut ahead) {
        return None;
    }
    let title = link_title(&mut ahead)?;
    if !line_ends(&mut ahead) {
        return None;
    }
    *scan = ahead;
    Some(title)
}

/// Reads a label's text, the `[` read, up to its unescaped `]`, which it
/// reads too; a `[` inside makes it no label.
fn label<'text>(scan: &mut Scan<'text>) -> Option<&'text str> {
    let start = scan.mark();
    loop {
        match scan.peek()? {
            '\\' => {
                scan.bump();
                scan.bump();
            }
            '[' => return None,
            ']' => {
                let label = scan.since(start);
                scan.bump();
                return Some(label);
            }
            _ => {
                scan.bump();
            }
        }
    }
}

/// Reads spaces and tabs to the end of the line and its line break, if
/// that is all there is before it; says whether it was.
fn line_ends(scan: &mut Scan<'_>) -> bool {
    let mut ahead = scan.clone();
    ahead.take_while(|ch| ch == ' ' || ch == '\t');
    if !(ahead.eat('\n') || ahead.is_done()) {
        return false;
    }
    *scan = ahead;
    true
}
