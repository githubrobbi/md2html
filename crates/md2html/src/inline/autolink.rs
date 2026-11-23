// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Links written as addresses: `<scheme:...>`, `<user@host>`, and GitHub's
//! bare `https://`, `http://` and `www.` links.

use crate::model::{Link, Node};
use crate::scan::Scan;

/// A link whose text is what was written.
fn link_to(href: String, shown: &str) -> Link {
    Link {
        href,
        title: None,
        kids: vec![Node::Text(shown.to_owned())],
    }
}

/// Reads `<scheme:...>` or `<user@host>`, and gives the link.
pub(super) fn autolink(scan: &mut Scan<'_>) -> Option<Link> {
    let mut ahead = scan.clone();
    if !ahead.eat('<') {
        return None;
    }
    // No space, `<` or control character inside.
    let inside =
        ahead.take_while(|ch| ch != '>' && ch != '<' && !ch.is_whitespace() && !ch.is_control());
    if inside.is_empty() || !ahead.eat('>') {
        return None;
    }
    let href = if is_uri(inside) {
        inside.to_owned()
    } else if is_email(inside) {
        format!("mailto:{inside}")
    } else {
        return None;
    };
    *scan = ahead;
    Some(link_to(href, inside))
}

/// Whether `text` is an absolute URI: a scheme of 2 to 32 letters,
/// digits, `+`, `.` and `-`, starting with a letter, then `:`.
fn is_uri(text: &str) -> bool {
    text.split_once(':').is_some_and(|(scheme, _)| {
        (2..=32).contains(&scheme.len())
            && scheme.starts_with(|ch: char| ch.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'))
    })
}

/// Whether `text` is an e-mail address as `CommonMark` has it.
fn is_email(text: &str) -> bool {
    let Some((user, host)) = text.split_once('@') else {
        return false;
    };
    let user_ok = !user.is_empty()
        && user
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(ch));
    let host_ok = !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        });
    user_ok && host_ok
}

/// `text` without `prefix`, which it starts with in any case of ASCII
/// letters.
fn strip_prefix_any_case<'text>(text: &'text str, prefix: &str) -> Option<&'text str> {
    let (head, tail) = text.split_at_checked(prefix.len())?;
    head.eq_ignore_ascii_case(prefix).then_some(tail)
}

/// The kind of bare link at the start of `text`, and what follows its
/// prefix.
fn bare_prefix(text: &str) -> Option<(Bare, &str)> {
    [
        ("https://", Bare::Web),
        ("http://", Bare::Web),
        ("www.", Bare::Www),
    ]
    .into_iter()
    .find_map(|(prefix, kind)| strip_prefix_any_case(text, prefix).map(|rest| (kind, rest)))
}

/// How a bare link starts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bare {
    /// `https://` or `http://`: the link is what is written.
    Web,
    /// `www.`: the link is that, after `http://`.
    Www,
}

/// Reads a bare `https://`, `http://` or `www.` link, without the
/// punctuation that ends the sentence around it, and gives the link.
pub(super) fn bare_url(scan: &mut Scan<'_>) -> Option<Link> {
    let written = scan
        .clone()
        .take_while(|ch| !ch.is_whitespace() && ch != '<');
    let (kind, after_prefix) = bare_prefix(written)?;
    let host = Scan::new(after_prefix).take_while(|ch| !matches!(ch, '/' | '?' | '#'));
    if host.is_empty()
        || !host
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        || (kind == Bare::Www && !host.contains('.'))
    {
        return None;
    }
    let shown = trim_trailing(written);
    let prefix_len = written.len().saturating_sub(after_prefix.len());
    if shown.len() <= prefix_len {
        return None;
    }
    scan.eat_str(shown);
    let href = match kind {
        Bare::Web => shown.to_owned(),
        Bare::Www => format!("http://{shown}"),
    };
    Some(link_to(href, shown))
}

/// A bare link without what ends the sentence around it: trailing
/// punctuation, a `)` that opens nowhere in it, an entity reference.
fn trim_trailing(written: &str) -> &str {
    let mut shown = written;
    while let Some(last) = shown.chars().next_back() {
        let trimmed = if "?!.,:*_~'\"".contains(last) {
            shown.strip_suffix(last)
        } else if last == ')' && shown.matches(')').count() > shown.matches('(').count() {
            shown.strip_suffix(')')
        } else if last == ';' {
            shown.rsplit_once('&').and_then(|(head, reference)| {
                let name = reference.strip_suffix(';')?;
                name.chars()
                    .all(|ch| ch.is_ascii_alphanumeric())
                    .then_some(head)
            })
        } else {
            None
        };
        match trimmed {
            Some(rest) => shown = rest,
            None => break,
        }
    }
    shown
}
