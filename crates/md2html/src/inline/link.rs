// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What follows a link's `]`: `(destination "title")`, a reference label,
//! and an image's `{attributes}`. Each reader reads what it recognises and
//! leaves the text unread when it does not.

use super::unescape;
use crate::model::{Attrs, Reference};
use crate::scan::Scan;

/// Where a link or image points.
pub(super) struct Target {
    /// The destination, unescaped.
    pub(super) href: String,
    /// The title, if any.
    pub(super) title: Option<String>,
}

impl From<&Reference> for Target {
    fn from(reference: &Reference) -> Self {
        Self {
            href: reference.href.clone(),
            title: reference.title.clone(),
        }
    }
}

/// Reads the spaces, tabs and at most one line break between the parts
/// of a link; says whether there were any.
pub(crate) fn link_space(scan: &mut Scan<'_>) -> bool {
    let mut breaks = 0_u8;
    let gap = scan.take_while(|ch| match ch {
        '\n' => {
            breaks = breaks.saturating_add(1);
            breaks == 1
        }
        ' ' | '\t' => true,
        _ => false,
    });
    !gap.is_empty()
}

/// `(destination "title")`: what it says.
pub(super) fn inline_target(scan: &mut Scan<'_>) -> Option<Target> {
    let mut ahead = scan.clone();
    if !ahead.eat('(') {
        return None;
    }
    link_space(&mut ahead);
    let href = if ahead.peek() == Some(')') {
        None
    } else {
        Some(link_destination(&mut ahead)?)
    };
    // A title stands apart from the destination.
    let mut title = None;
    if link_space(&mut ahead) || href.is_none() {
        title = link_title(&mut ahead);
        if title.is_some() {
            link_space(&mut ahead);
        }
    }
    if !ahead.eat(')') {
        return None;
    }
    *scan = ahead;
    Some(Target {
        href: href.unwrap_or_default(),
        title,
    })
}

/// Reads a link destination, `<...>` or a run without spaces whose
/// parentheses balance, and gives it unescaped.
pub(crate) fn link_destination(scan: &mut Scan<'_>) -> Option<String> {
    let mut ahead = scan.clone();
    let raw = if ahead.eat('<') {
        angled(&mut ahead)?
    } else {
        bare(&mut ahead)?
    };
    *scan = ahead;
    Some(unescape(raw))
}

/// Reads a backslash and the ASCII punctuation it escapes; before any
/// other character the backslash is itself, and only it is read.
fn escape(scan: &mut Scan<'_>) {
    scan.bump();
    scan.eat_if(|next| next.is_ascii_punctuation());
}

/// The inside of `<...>`, the `<` read; reads the `>` too.
fn angled<'text>(scan: &mut Scan<'text>) -> Option<&'text str> {
    let start = scan.mark();
    loop {
        match scan.peek()? {
            '\\' => escape(scan),
            '\n' | '<' => return None,
            '>' => {
                let inside = scan.since(start);
                scan.bump();
                return Some(inside);
            }
            _ => {
                scan.bump();
            }
        }
    }
}

/// A destination without angle brackets: up to a space, a control
/// character or a `)` that closes nothing; not empty, and its
/// parentheses balanced, at most 32 deep.
fn bare<'text>(scan: &mut Scan<'text>) -> Option<&'text str> {
    let start = scan.mark();
    let mut open = 0_u8;
    while let Some(ch) = scan.peek() {
        match ch {
            '\\' => {
                escape(scan);
                continue;
            }
            '(' => {
                open = open.saturating_add(1);
                if open > 32 {
                    return None;
                }
            }
            ')' if open == 0 => break,
            ')' => open = open.saturating_sub(1),
            ' ' => break,
            _ if ch.is_ascii_control() => break,
            _ => {}
        }
        scan.bump();
    }
    let raw = scan.since(start);
    (!raw.is_empty() && open == 0).then_some(raw)
}

/// Reads a link title, `"..."`, `'...'` or `(...)`, which may span lines,
/// and gives it unescaped.
pub(crate) fn link_title(scan: &mut Scan<'_>) -> Option<String> {
    let mut ahead = scan.clone();
    let close = match ahead.bump()? {
        '"' => '"',
        '\'' => '\'',
        '(' => ')',
        _ => return None,
    };
    let start = ahead.mark();
    loop {
        let ch = ahead.peek()?;
        if ch == '\\' {
            ahead.bump();
        } else if ch == close {
            let inside = ahead.since(start);
            ahead.bump();
            *scan = ahead;
            return Some(unescape(inside));
        } else if close == ')' && ch == '(' {
            return None;
        }
        ahead.bump();
    }
}

/// How many characters a link label may hold between its brackets.
/// Longer bracket text is no label, so no definition can match it.
pub(crate) const LABEL_LIMIT: usize = 999;

/// Whether `label` is short enough to be a link label (see
/// [`LABEL_LIMIT`]). Text whose bytes could not all be characters of a
/// label is answered without counting.
pub(crate) fn fits_label(label: &str) -> bool {
    label.len() <= LABEL_LIMIT
        || (label.len() <= LABEL_LIMIT.saturating_mul(4)
            && label.chars().nth(LABEL_LIMIT).is_none())
}

/// Reads a reference label, `[...]`, and gives what is inside: no
/// unescaped bracket, at most [`LABEL_LIMIT`] characters.
pub(super) fn label<'text>(scan: &mut Scan<'text>) -> Option<&'text str> {
    let mut ahead = scan.clone();
    if !ahead.eat('[') {
        return None;
    }
    let start = ahead.mark();
    loop {
        match ahead.peek()? {
            '\\' => {
                ahead.bump();
            }
            '[' => return None,
            ']' => break,
            _ => {}
        }
        ahead.bump();
    }
    let inside = ahead.since(start);
    if !fits_label(inside) {
        return None;
    }
    ahead.bump();
    *scan = ahead;
    Some(inside)
}

/// A size an image may be given: letters, digits, `.` and `%`, quotes
/// around it dropped.
fn size(value: &str) -> Option<String> {
    let bare_value = value.trim_matches('"');
    (!bare_value.is_empty()
        && bare_value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '%')))
    .then(|| bare_value.to_owned())
}

/// Whether `name` may be an id or a class: letters, digits, `-` and `_`.
fn is_ident(name: &str) -> bool {
    name.chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_'))
}

/// Reads `{width=50% .wide #map}` after an image: what it says.
pub(super) fn attributes(scan: &mut Scan<'_>) -> Option<Attrs> {
    let mut ahead = scan.clone();
    if !ahead.eat('{') {
        return None;
    }
    let inside = ahead.take_until("}")?;
    ahead.bump();
    if inside.contains('\n') || inside.trim().is_empty() {
        return None;
    }
    let mut attrs = Attrs::default();
    for piece in inside.split_whitespace() {
        if let Some(id) = piece.strip_prefix('#') {
            attrs.id = Some(id.to_owned());
        } else if let Some(class) = piece.strip_prefix('.') {
            attrs.classes.push(class.to_owned());
        } else if let Some(width) = piece.strip_prefix("width=") {
            attrs.width = size(width);
        } else {
            attrs.height = size(piece.strip_prefix("height=")?);
        }
    }
    if attrs.id.as_deref().is_some_and(|id| !is_ident(id))
        || attrs.classes.iter().any(|class| !is_ident(class))
    {
        return None;
    }
    *scan = ahead;
    Some(attrs)
}
