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
    /// The column of the source line the text starts at: what the
    /// containers around it took off, so a tab after a list marker counts
    /// to the right stop.
    pub(super) col: usize,
    /// A lazy continuation line: it continues the paragraph open above it
    /// without its container's marks, so it is that paragraph's text,
    /// whatever it looks like.
    pub(super) lazy: bool,
}

impl Line {
    /// The lines of `text`, the first numbered `first`.
    pub(super) fn split(text: &str, first: usize) -> Vec<Self> {
        text.split('\n')
            .enumerate()
            .map(|(at, raw)| Self {
                no: at.saturating_add(first),
                text: expand_tabs(raw.strip_suffix('\r').unwrap_or(raw)).replace('\0', "\u{FFFD}"),
                col: 0,
                lazy: false,
            })
            .collect()
    }

    /// What a container leaves of the line: `text`, a tail of its text
    /// after marks one column each.
    pub(super) fn with(&self, text: &str) -> Self {
        self.at(text, self.text.len().saturating_sub(text.len()))
    }

    /// Another text on the same source line, starting `skip` columns into
    /// this one's.
    pub(super) fn at(&self, text: &str, skip: usize) -> Self {
        Self {
            no: self.no,
            text: text.to_owned(),
            col: self.col.saturating_add(skip),
            lazy: self.lazy,
        }
    }

    /// The line as a lazy continuation, its indentation taken off.
    pub(super) fn to_lazy(&self) -> Self {
        Self {
            lazy: true,
            ..self.with(self.text.trim_start())
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

/// The columns the spaces and tabs that start `text` fill when it starts
/// at column `start` of its source line, a tab reaching to the next stop
/// of four.
pub(super) fn gap_width(text: &str, start: usize) -> usize {
    let mut col = start;
    for ch in text.chars() {
        col = match ch {
            ' ' => col.saturating_add(1),
            '\t' => col.saturating_add(4 - col % 4),
            _ => break,
        };
    }
    col.saturating_sub(start)
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
