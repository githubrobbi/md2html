// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The edge between two nodes as written: `-->`, `---`, `-.->`, `==>`,
//! `--o`, `<-->`, `-- text -->`, `-->|text|`.

use super::model::{Head, Stroke};
use crate::mermaid::label::Label;
use crate::scan::Scan;

/// An edge operator read.
pub(super) struct Op {
    /// The label on the edge.
    pub(super) label: Option<Label>,
    /// How the line is drawn.
    pub(super) stroke: Stroke,
    /// The mark at its start.
    pub(super) tail: Head,
    /// The mark at its end.
    pub(super) head: Head,
    /// How many ranks it spans at least.
    pub(super) minlen: usize,
}

/// Whether `rest` starts with an edge.
pub(super) fn at_edge(rest: &str) -> bool {
    ["--", "-.", "==", "~~", "<-", "<=", "o--", "x--"]
        .iter()
        .any(|start| rest.starts_with(start))
}

/// The edge `scan` is at, read; nothing is read when there is none.
pub(super) fn edge_op(scan: &mut Scan<'_>) -> Option<Op> {
    let mut probe = scan.clone();
    let tail = tail(&mut probe);
    let (stroke, dashes) = line(&mut probe);
    if dashes < 2 {
        return None;
    }
    let (head, inside) = head(&mut probe, stroke, dashes);
    let label = inside.or_else(|| piped_label(&mut probe));
    *scan = probe;
    Some(Op {
        label,
        stroke,
        tail,
        head,
        minlen: if dashes > 3 { (dashes - 2).min(4) } else { 1 },
    })
}

/// The mark before the line: `<`, or `o` or `x` right before a dash.
fn tail(scan: &mut Scan<'_>) -> Head {
    let head = match (scan.peek(), scan.peek_nth(1)) {
        (Some('<'), _) => Head::Arrow,
        (Some('o'), Some('-')) => Head::Circle,
        (Some('x'), Some('-')) => Head::Cross,
        _ => return Head::None,
    };
    scan.bump();
    head
}

/// The line itself: how it is drawn, and how many marks it is long.
fn line(scan: &mut Scan<'_>) -> (Stroke, usize) {
    let mut stroke = Stroke::Solid;
    let mut dashes = 0_usize;
    while let Some(ch) = scan.eat_if(|ch| matches!(ch, '-' | '=' | '.' | '~')) {
        stroke = match ch {
            '=' => Stroke::Thick,
            '.' => Stroke::Dotted,
            '~' => Stroke::Invisible,
            _ => stroke,
        };
        dashes += 1;
    }
    (stroke, dashes)
}

/// Whether what follows a mark ends it as a head: nothing, a space or `|`.
fn ends_head(next: Option<char>) -> bool {
    next.is_none_or(|ch| ch.is_whitespace() || ch == '|')
}

/// The mark after the line, or a label inside the edge with the rest of the
/// edge after it.
fn head(scan: &mut Scan<'_>, stroke: Stroke, dashes: usize) -> (Head, Option<Label>) {
    let mark = match scan.peek() {
        Some('>') => Head::Arrow,
        Some('o') if ends_head(scan.peek_nth(1)) => Head::Circle,
        Some('x') if ends_head(scan.peek_nth(1)) => Head::Cross,
        Some(ch)
            if (ch.is_whitespace() || (stroke == Stroke::Dotted && ch.is_alphanumeric()))
                && dashes <= 2
                && stroke != Stroke::Invisible =>
        {
            return inside_label(scan, stroke).unwrap_or((Head::None, None));
        }
        _ => return (Head::None, None),
    };
    scan.bump();
    (mark, None)
}

/// `-- text -->`, `-. text .->`, `== text ==>`: the label written inside the
/// edge, up to the nearest closer, and the head that closer ends with.
fn inside_label(scan: &mut Scan<'_>, stroke: Stroke) -> Option<(Head, Option<Label>)> {
    let closers: &[&str] = match stroke {
        Stroke::Dotted => &[".->", ".-"],
        Stroke::Thick => &["==>", "==="],
        Stroke::Solid | Stroke::Invisible => &["-->", "---", "--o", "--x"],
    };
    let (text, closer, mut ahead) = closers
        .iter()
        .filter_map(|closer| {
            let mut ahead = scan.clone();
            let text = ahead.take_until(closer)?;
            Some((text, *closer, ahead))
        })
        .min_by_key(|(text, _, _)| text.len())?;
    let words = text.trim();
    if words.is_empty() || words.contains("--") {
        return None;
    }
    ahead.eat_str(closer);
    let mut head = match closer.chars().last() {
        Some('>') => Head::Arrow,
        Some('o') => Head::Circle,
        Some('x') => Head::Cross,
        _ => Head::None,
    };
    // A longer closer (`--->`) stretches the edge.
    ahead.take_while(|ch| matches!(ch, '-' | '=' | '.'));
    if ahead.eat('>') {
        head = Head::Arrow;
    }
    *scan = ahead;
    Some((head, Some(Label::parse(words))))
}

/// `|text|` after the edge, spaces before it allowed.
fn piped_label(scan: &mut Scan<'_>) -> Option<Label> {
    let mut ahead = scan.clone();
    ahead.take_while(|ch| ch == ' ');
    if !ahead.eat('|') {
        return None;
    }
    let text = ahead.take_until("|")?;
    ahead.bump();
    *scan = ahead;
    Some(Label::parse(text))
}
