// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Raw HTML in the text: where a tag, comment, processing instruction,
//! declaration or CDATA section ends, and the tags an untrusted page may
//! keep.

use super::decode_entities;
use crate::scan::Scan;

/// The complete HTML tag, comment, processing instruction, declaration or
/// CDATA section at the start of `text`, if one is there.
pub(crate) fn html_tag(text: &str) -> Option<&str> {
    read_tag(&mut Scan::new(text))
}

/// Reads a complete tag, comment, processing instruction, declaration or
/// CDATA section, and gives it.
pub(super) fn read_tag<'text>(scan: &mut Scan<'text>) -> Option<&'text str> {
    let mut ahead = scan.clone();
    let start = ahead.mark();
    if !(ahead.eat('<') && tag_body(&mut ahead)) {
        return None;
    }
    *scan = ahead;
    Some(scan.since(start))
}

/// Reads up to and past `end`; says whether it was there.
fn through(scan: &mut Scan<'_>, end: &str) -> bool {
    scan.take_until(end).is_some() && scan.eat_str(end)
}

/// Reads spaces, tabs and line breaks; says whether there were any.
fn skip_space(scan: &mut Scan<'_>) -> bool {
    !scan
        .take_while(|ch| matches!(ch, ' ' | '\t' | '\n'))
        .is_empty()
}

/// Reads what follows a `<`; says whether it makes a whole tag.
fn tag_body(scan: &mut Scan<'_>) -> bool {
    if scan.eat_str("!--") {
        scan.eat('>') || scan.eat_str("->") || through(scan, "-->")
    } else if scan.eat('?') {
        through(scan, "?>")
    } else if scan.eat_str("![CDATA[") {
        through(scan, "]]>")
    } else if scan.eat('!') {
        scan.peek().is_some_and(|ch| ch.is_ascii_alphabetic()) && through(scan, ">")
    } else if scan.eat('/') {
        tag_name(scan) && {
            skip_space(scan);
            scan.eat('>')
        }
    } else {
        tag_name(scan) && opening_rest(scan)
    }
}

/// Reads a tag's name: a letter, then letters, digits and `-`.
fn tag_name(scan: &mut Scan<'_>) -> bool {
    scan.peek().is_some_and(|ch| ch.is_ascii_alphabetic())
        && !scan
            .take_while(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            .is_empty()
}

/// Reads an opening tag's attributes and its `>` or `/>`.
fn opening_rest(scan: &mut Scan<'_>) -> bool {
    loop {
        let spaced = skip_space(scan);
        match scan.peek() {
            Some('>') => return scan.eat('>'),
            Some('/') => return scan.eat_str("/>"),
            Some(ch) if spaced && (ch.is_ascii_alphabetic() || matches!(ch, '_' | ':')) => {
                if !attribute(scan) {
                    return false;
                }
            }
            Some(_) | None => return false,
        }
    }
}

/// Reads one attribute: a name, and `=` with a value if one follows.
fn attribute(scan: &mut Scan<'_>) -> bool {
    scan.take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '-'));
    let after_name = scan.mark();
    skip_space(scan);
    if !scan.eat('=') {
        scan.rewind(after_name);
        return true;
    }
    skip_space(scan);
    match scan.peek() {
        Some(quote @ ('"' | '\'')) => {
            scan.bump();
            scan.take_while(|ch| ch != quote);
            scan.eat(quote)
        }
        Some(_) => !scan
            .take_while(|ch| !matches!(ch, ' ' | '\t' | '\n' | '"' | '\'' | '=' | '<' | '>' | '`'))
            .is_empty(),
        None => false,
    }
}

/// The tags safe to pass through from an untrusted page, and what each
/// may carry. `<details>`, `<kbd>`, `<sub>`, `<br>` and the like are what
/// Markdown writers reach for; scripts, styles, frames, forms and event
/// handlers are not among them.
const SAFE_TAGS: &[(&str, &[&str])] = &[
    ("abbr", &["title"]),
    ("b", &[]),
    ("br", &[]),
    ("cite", &[]),
    ("code", &[]),
    ("dd", &[]),
    ("del", &[]),
    ("details", &["open"]),
    ("dfn", &[]),
    ("div", &[]),
    ("dl", &[]),
    ("dt", &[]),
    ("em", &[]),
    ("hr", &[]),
    ("i", &[]),
    ("ins", &[]),
    ("kbd", &[]),
    ("li", &[]),
    ("mark", &[]),
    ("ol", &["start"]),
    ("p", &[]),
    ("q", &[]),
    ("s", &[]),
    ("samp", &[]),
    ("small", &[]),
    ("span", &[]),
    ("strong", &[]),
    ("sub", &[]),
    ("summary", &[]),
    ("sup", &[]),
    ("table", &[]),
    ("tbody", &[]),
    ("td", &["colspan", "rowspan"]),
    ("tfoot", &[]),
    ("th", &["colspan", "rowspan"]),
    ("thead", &[]),
    ("tr", &[]),
    ("u", &[]),
    ("ul", &[]),
    ("var", &[]),
    ("wbr", &[]),
];

/// A tag rewritten with only its allowed attributes, or None when the tag
/// is not on the safe list (it is then shown as written).
pub(crate) fn safe_tag(tag: &str) -> Option<String> {
    let inside = tag.strip_prefix('<')?.strip_suffix('>')?;
    let (closing, body) = inside
        .strip_prefix('/')
        .map_or((false, inside), |rest| (true, rest));
    let mut scan = Scan::new(body.strip_suffix('/').unwrap_or(body));
    let name = scan
        .take_while(|ch| ch.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    let (_, allowed) = SAFE_TAGS.iter().find(|(known, _)| *known == name)?;
    if closing {
        return Some(format!("</{name}>"));
    }
    let mut out = format!("<{name}");
    let mut attrs = Scan::new(scan.rest().trim());
    while !attrs.is_done() {
        let key = attrs
            .take_while(|ch| ch != '=' && !ch.is_whitespace())
            .to_ascii_lowercase();
        attrs.take_while(char::is_whitespace);
        let value = attrs.eat('=').then(|| attribute_value(&mut attrs));
        if allowed.contains(&key.as_str()) {
            out.push(' ');
            out.push_str(&key);
            if let Some(written) = value {
                out.push_str("=\"");
                out.push_str(&crate::esc(&decode_entities(written)));
                out.push('"');
            }
        }
    }
    out.push('>');
    Some(out)
}

/// Reads an attribute's value after its `=`: quoted (to the closing quote
/// or the end), or up to a space; and the space after it.
fn attribute_value<'text>(scan: &mut Scan<'text>) -> &'text str {
    scan.take_while(char::is_whitespace);
    let value = match scan.eat_if(|ch| ch == '"' || ch == '\'') {
        Some(quote) => {
            let quoted = scan.take_while(|ch| ch != quote);
            scan.eat(quote);
            quoted
        }
        None => scan.take_while(|ch| !ch.is_whitespace()),
    };
    scan.take_while(char::is_whitespace);
    value
}
