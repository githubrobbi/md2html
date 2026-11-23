// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Lines of the source, and what their indentation says.

/// A line of the source with its number.
#[derive(Clone, Debug)]
pub(super) struct Line {
    /// One-based.
    pub(super) no: usize,
    /// The text, tabs in the indentation expanded.
    pub(super) text: String,
}

impl Line {
    /// The lines of `text`, the first numbered `first`.
    pub(super) fn split(text: &str, first: usize) -> Vec<Self> {
        text.split('\n')
            .enumerate()
            .map(|(at, raw)| Self {
                no: at.saturating_add(first),
                text: expand_tabs(raw.strip_suffix('\r').unwrap_or(raw)).replace('\0', "\u{FFFD}"),
            })
            .collect()
    }

    /// Another text on the same source line: what a container leaves of it.
    pub(super) fn with(&self, text: &str) -> Self {
        Self {
            no: self.no,
            text: text.to_owned(),
        }
    }

    /// Whether it holds nothing but spaces.
    pub(super) fn is_blank(&self) -> bool {
        is_blank(&self.text)
    }
}

/// Tabs in a line's indentation as spaces to the next stop of four.
fn expand_tabs(line: &str) -> String {
    if !line.contains('\t') {
        return line.to_owned();
    }
    let mut out = String::with_capacity(line.len().saturating_add(8));
    let mut col = 0_usize;
    let mut leading = true;
    for ch in line.chars() {
        if leading && ch == '\t' {
            let fill = 4 - col % 4;
            for _ in 0..fill {
                out.push(' ');
            }
            col = col.saturating_add(fill);
            continue;
        }
        if leading && !matches!(ch, ' ' | '>') {
            leading = false;
        }
        out.push(ch);
        col = col.saturating_add(1);
    }
    out
}

/// Whether a line holds nothing but spaces.
pub(super) fn is_blank(line: &str) -> bool {
    line.chars().all(|ch| ch == ' ' || ch == '\t')
}

/// A line split at the end of its leading spaces.
#[derive(Clone, Copy, Debug)]
pub(super) struct Indented<'line> {
    /// How many spaces lead it.
    pub(super) width: usize,
    /// What follows them.
    pub(super) body: &'line str,
}

/// `line` split at the end of its leading spaces.
pub(super) fn indented(line: &str) -> Indented<'_> {
    let body = line.trim_start_matches(' ');
    Indented {
        width: line.len() - body.len(),
        body,
    }
}

/// The number of leading spaces.
pub(super) fn indent(line: &str) -> usize {
    indented(line).width
}

/// The line with up to `cols` leading spaces removed.
pub(super) fn strip_cols(line: &str, cols: usize) -> &str {
    let mut rest = line;
    for _ in 0..cols {
        let Some(after) = rest.strip_prefix(' ') else {
            break;
        };
        rest = after;
    }
    rest
}

/// `---`, `***`, `___`: three or more of one mark, spaces between allowed.
pub(super) fn is_rule(line: &str) -> bool {
    let mut mark = None;
    let mut count = 0_usize;
    for ch in line.trim_end().chars() {
        match ch {
            ' ' | '\t' => {}
            '-' | '*' | '_' => {
                if mark.is_some_and(|seen| seen != ch) {
                    return false;
                }
                mark = Some(ch);
                count = count.saturating_add(1);
            }
            _ => return false,
        }
    }
    count >= 3
}
