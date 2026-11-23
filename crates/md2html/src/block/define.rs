// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Link reference definitions, `[label]: destination "title"`, read off
//! the front of a paragraph.

use super::line::is_blank;
use super::normalize_label;
use crate::inline::{fits_label, link_destination, link_title};
use crate::model::{Defs, Reference};
use crate::scan::{Cursor, Scan};

/// Reads the definitions that open a paragraph's `lines` into `defs`;
/// gives the lines after them. The first definition of a label wins.
pub(super) fn take_definitions<'lines>(
    lines: &'lines [String],
    defs: &mut Defs,
) -> &'lines [String] {
    let mut reader = Cursor::new(lines);
    while let Some((label, reference)) = definition(&mut reader) {
        defs.links
            .entry(normalize_label(label))
            .or_insert(reference);
    }
    reader.rest()
}

/// The definition at the front of `lines`, which reads the one to three
/// lines it uses: the label and where it points.
fn definition<'lines>(lines: &mut Cursor<'lines, String>) -> Option<(&'lines str, Reference)> {
    let (label, after) = split_label(lines.peek()?.strip_prefix('[')?)?;
    // A label longer than a label may be is no label: the line is text.
    if label.trim().is_empty() || label.starts_with('^') || !fits_label(label) {
        return None;
    }
    let same_line = after.strip_prefix(':')?.trim_start();
    let (target, mut used) = if same_line.is_empty() {
        (lines.peek_nth(1)?.trim(), 2)
    } else {
        (same_line, 1)
    };
    let (href, remainder) = link_destination(target)?;
    let title = if remainder.trim().is_empty() {
        // The title may stand on the next line.
        let next = lines
            .peek_nth(used)
            .and_then(|next| whole_title(next.trim()));
        if next.is_some() {
            used += 1;
        }
        next
    } else {
        if !remainder.starts_with([' ', '\t']) {
            return None;
        }
        Some(whole_title(remainder.trim())?)
    };
    lines.take(used);
    Some((label, Reference { href, title }))
}

/// The title that is all of `text`, but for spaces after it.
fn whole_title(text: &str) -> Option<String> {
    let (title, past_title) = link_title(text)?;
    is_blank(past_title).then_some(title)
}

/// A label that began just before `text`, up to its unescaped `]`, and
/// the text after that `]`.
fn split_label(text: &str) -> Option<(&str, &str)> {
    let mut scan = Scan::new(text);
    loop {
        match scan.peek()? {
            '\\' => {
                scan.bump();
                scan.bump();
            }
            '[' => return None,
            ']' => {
                let label = scan.seen();
                scan.bump();
                return Some((label, scan.rest()));
            }
            _ => {
                scan.bump();
            }
        }
    }
}
