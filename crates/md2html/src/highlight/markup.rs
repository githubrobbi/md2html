// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! HTML and XML: tags, attribute names, quoted values and comments.

use super::{Class, read_through, span};
use crate::scan::Scan;
use crate::text::esc_into;

/// HTML and XML as highlighted HTML.
pub(super) fn markup(code: &str) -> String {
    let mut out = String::with_capacity(code.len().saturating_mul(2));
    let mut scan = Scan::new(code);
    while !scan.is_done() {
        let from = scan.mark();
        if scan.starts_with("<!--") {
            // The search for the close starts at the `<`, so `<!-->` is a
            // whole comment.
            read_through(&mut scan, "-->");
            span(&mut out, Class::Comment, scan.since(from));
        } else if scan.starts_with("<") {
            read_through(&mut scan, ">");
            tag(&mut out, scan.since(from));
        } else {
            esc_into(&mut out, scan.take_while(|ch| ch != '<'));
        }
    }
    out
}

/// A tag, from its `<` through its `>` (or the end of the code): its name,
/// then its attributes.
fn tag(out: &mut String, text: &str) {
    let mut scan = Scan::new(text);
    scan.bump();
    scan.take_while(|ch| !ch.is_whitespace() && ch != '>');
    span(out, Class::Keyword, scan.seen());
    let mut attributes = Scan::new(scan.rest());
    while !attributes.is_done() {
        let before = attributes.take_while(|ch| ch != '"' && ch != '\'');
        let Some(quote) = attributes.peek() else {
            esc_into(out, before);
            return;
        };
        name(out, before);
        let from = attributes.mark();
        attributes.bump();
        attributes.take_while(|ch| ch != quote);
        attributes.eat(quote);
        span(out, Class::Str, attributes.since(from));
    }
}

/// What comes before a quoted value: the attribute's name is the word
/// before the last `=`.
fn name(out: &mut String, text: &str) {
    let Some((head, after)) = text.rsplit_once('=') else {
        esc_into(out, text);
        return;
    };
    let mut scan = Scan::new(head);
    let space = head.trim_end_matches(|ch: char| !ch.is_whitespace());
    esc_into(out, scan.take_chars(space.chars().count()));
    span(out, Class::Attr, scan.rest());
    out.push('=');
    esc_into(out, after);
}
