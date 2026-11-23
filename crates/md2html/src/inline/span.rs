// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Spans whose content is taken as written: code between backticks and
//! maths between dollar signs. Each reader reads the span when it is one
//! and leaves the text unread when it is not.

use alloc::collections::BTreeMap;

use crate::model::{Math, MathMode};
use crate::scan::{Mark, Scan};

/// What the searches for a closing backtick run have learnt about the
/// text, so that no stretch of it is searched in vain twice.
#[derive(Default)]
pub(super) struct Ticks {
    /// Known once a search has read to the end of the text in vain: for
    /// each length, where the last run of that many backticks starts.
    last: Option<BTreeMap<usize, Mark>>,
}

impl Ticks {
    /// Whether a run of `len` backticks may follow the opening run that
    /// starts at `opener`. Until a search has failed, nothing is known
    /// and any may.
    fn may_close(&self, len: usize, opener: Mark) -> bool {
        self.last
            .as_ref()
            .is_none_or(|last| last.get(&len).is_some_and(|start| *start > opener))
    }
}

/// Reads a code span whose opening run of backticks was just read.
///
/// The opening run of `ticks` backticks starts at `opener`, and the span
/// runs up to a run exactly as long. It gives the span's content: line
/// breaks as spaces, and one space each side dropped when both are there
/// and the content is not all spaces.
///
/// The search that first reads to the end in vain records every run it
/// passed in `known`; every later opener whose length has no run after
/// it then fails at once. A search that succeeds reads only what becomes
/// the span, so all searches together read the text about twice.
pub(super) fn code_span(
    scan: &mut Scan<'_>,
    ticks: usize,
    opener: Mark,
    known: &mut Ticks,
) -> Option<String> {
    if !known.may_close(ticks, opener) {
        return None;
    }
    let mut ahead = scan.clone();
    let start = ahead.mark();
    let mut runs = BTreeMap::new();
    loop {
        ahead.take_while(|ch| ch != '`');
        let inside = ahead.since(start);
        let run_start = ahead.mark();
        let closing = ahead.take_while(|ch| ch == '`');
        if closing.is_empty() {
            known.last = Some(runs);
            return None;
        }
        if closing.len() == ticks {
            *scan = ahead;
            return Some(code_content(inside));
        }
        runs.insert(closing.len(), run_start);
    }
}

/// A code span's content as it is shown.
fn code_content(inside: &str) -> String {
    let code = inside.replace('\n', " ");
    if !code.trim().is_empty() {
        if let Some(padded) = code
            .strip_prefix(' ')
            .and_then(|rest| rest.strip_suffix(' '))
        {
            return padded.to_owned();
        }
    }
    code
}

/// Reads `$tex$` or `$$tex$$`, pandoc's rules: no space just inside the
/// marks, and no digit right after the closing one, so "$5 and $10" is
/// money.
pub(super) fn math_span(scan: &mut Scan<'_>) -> Option<Math> {
    let mut ahead = scan.clone();
    let math = if ahead.eat_str("$$") {
        display(&mut ahead)?
    } else if ahead.eat('$') {
        inline(&mut ahead)?
    } else {
        return None;
    };
    *scan = ahead;
    Some(math)
}

/// `$$tex$$`, the opening marks read.
fn display(scan: &mut Scan<'_>) -> Option<Math> {
    let tex = scan.take_until("$$")?.trim();
    scan.eat_str("$$");
    (!tex.is_empty()).then(|| Math {
        tex: tex.to_owned(),
        mode: MathMode::Display,
    })
}

/// `$tex$`, the opening mark read. A blank line ends the search.
///
/// The search stops at the first `$` it does not skip as escaped, so a
/// search that fails costs only the stretch up to the next `$`, provided
/// the inline pass never opens maths at a `$` a search skipped. It does
/// not: from the `$` both start at, they pair backslashes alike (a
/// backslash before punctuation escapes it for both; before anything
/// else, that character is plain text for both, a line break a break for
/// both). They differ only inside what the pass reads whole: code, tags,
/// autolinks, entities, link targets, bare URLs; and the pass opens
/// nothing inside those. Each of them ends on a character that is not an
/// unpaired backslash, except a bare URL, which is followed by space,
/// `<` or punctuation other than `$`. So the searches read disjoint
/// stretches (`tests/hostile.rs` times the shapes that come closest).
fn inline(scan: &mut Scan<'_>) -> Option<Math> {
    if scan.peek().is_some_and(char::is_whitespace) {
        return None;
    }
    let start = scan.mark();
    while let Some(ch) = scan.peek() {
        match ch {
            '\\' => {
                scan.bump();
            }
            '\n' if scan.prev() == Some('\n') => return None,
            '$' => {
                let tex = scan.since(start);
                scan.bump();
                let last = tex.chars().next_back()?;
                if last.is_whitespace() || scan.peek().is_some_and(|next| next.is_ascii_digit()) {
                    return None;
                }
                return Some(Math {
                    tex: tex.to_owned(),
                    mode: MathMode::Inline,
                });
            }
            _ => {}
        }
        scan.bump();
    }
    None
}
