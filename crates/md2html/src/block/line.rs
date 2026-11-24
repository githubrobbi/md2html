// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Lines of the source, and what their indentation says.

use alloc::borrow::Cow;

/// A line of the source with its number.
///
/// Its text is a slice of the source wherever it can be: a container's
/// lines are tails of the lines around it, so a text of many short lines,
/// or of quotes nested deep, costs no copy of the text a line or a level.
/// Only a line whose indentation has a tab, or that holds a NUL, or the
/// first line of a list item, owns its text.
#[derive(Clone, Debug)]
pub(super) struct Line<'src> {
    /// One-based.
    pub(super) no: usize,
    /// The text, tabs in the indentation expanded.
    pub(super) text: Cow<'src, str>,
    /// The column of the source line the text starts at: what the
    /// containers around it took off, so a tab after a list marker counts
    /// to the right stop.
    pub(super) col: usize,
    /// A lazy continuation line: it continues the paragraph open above it
    /// without its container's marks, so it is that paragraph's text,
    /// whatever it looks like.
    pub(super) lazy: bool,
}

impl<'src> Line<'src> {
    /// The source line `raw`, numbered `no`, its line ending off.
    pub(super) fn source(raw: &'src str, no: usize) -> Self {
        Self {
            no,
            text: readable(expand_tabs(raw.strip_suffix('\r').unwrap_or(raw))),
            col: 0,
            lazy: false,
        }
    }

    /// A blank line of a container, numbered `no`: what is left of a line
    /// that holds nothing but the container's marks or spaces.
    pub(super) const fn blank(no: usize) -> Self {
        Self {
            no,
            text: Cow::Borrowed(""),
            col: 0,
            lazy: false,
        }
    }

    /// What a container leaves of the line: `text`, a tail of its text
    /// after marks one column each. A slice of the source when this line
    /// is one.
    pub(super) fn with(&self, text: &str) -> Self {
        let skip = self.text.len().saturating_sub(text.len());
        let tail = match &self.text {
            Cow::Borrowed(whole) => whole
                .get(skip..)
                .filter(|tail| core::ptr::eq(*tail, text))
                .map(Cow::Borrowed),
            Cow::Owned(_) => None,
        };
        self.at(tail.unwrap_or_else(|| Cow::Owned(text.to_owned())), skip)
    }

    /// Another text on the same source line, starting `skip` columns into
    /// this one's.
    pub(super) const fn at(&self, text: Cow<'src, str>, skip: usize) -> Self {
        Self {
            no: self.no,
            text,
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

/// A line with a NUL as U+FFFD, as `CommonMark` reads it.
fn readable(line: Cow<'_, str>) -> Cow<'_, str> {
    if line.contains('\0') {
        Cow::Owned(line.replace('\0', "\u{FFFD}"))
    } else {
        line
    }
}

/// Tabs in a line's indentation as spaces to the next stop of four.
fn expand_tabs(line: &str) -> Cow<'_, str> {
    if !line.contains('\t') {
        return Cow::Borrowed(line);
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
    Cow::Owned(out)
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

#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;

    use super::Line;

    /// A line is a slice of the source, and so is what a container leaves
    /// of it, at any depth; only a tab in the indentation makes a copy.
    ///
    /// # Panics
    ///
    /// When a line without a tab, or the tail of one a quote or a list
    /// leaves at three levels, owns its text; when a line with a tab in its
    /// indentation does not have it expanded; or when a text that is not a
    /// tail of the line is not copied.
    #[test]
    fn lines_borrow_the_source() {
        let quoted = Line::source("> > > a", 1);
        let tabbed = Line::source("\tcode\r", 2);
        assert!(matches!(quoted.text, Cow::Borrowed(_)));
        let mut inner = quoted.clone();
        for _ in 0_u8..3 {
            let tail = inner.text.strip_prefix("> ").unwrap_or_default();
            let next = inner.with(tail);
            assert!(matches!(next.text, Cow::Borrowed(_)), "{tail}");
            assert_eq!(next.text, tail);
            inner = next;
        }
        assert_eq!(inner.text, "a");
        assert_eq!(tabbed.text, "    code");
        assert!(matches!(tabbed.text, Cow::Owned(_)));
        let elsewhere = quoted.with("a b");
        assert!(matches!(elsewhere.text, Cow::Owned(_)));
        assert_eq!(elsewhere.text, "a b");
    }
}
