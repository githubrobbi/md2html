// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Entity references (`&amp;`, `&#233;`, `&#x1F600;`) and backslash
//! escapes.

use crate::scan::Scan;

/// What an entity reference stands for.
pub(super) enum Entity<'text> {
    /// A character this tool knows.
    Char(char),
    /// The name of a well-formed named reference, left for the browser to
    /// decode.
    Raw(&'text str),
}

/// The common named entities, so the search index and the alt texts see
/// the character rather than its name.
const ENTITIES: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("copy", '©'),
    ("reg", '®'),
    ("trade", '™'),
    ("hellip", '…'),
    ("mdash", '—'),
    ("ndash", '–'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("laquo", '«'),
    ("raquo", '»'),
    ("bdquo", '„'),
    ("sbquo", '‚'),
    ("times", '×'),
    ("divide", '÷'),
    ("deg", '°'),
    ("plusmn", '±'),
    ("middot", '·'),
    ("bull", '•'),
    ("euro", '€'),
    ("pound", '£'),
    ("yen", '¥'),
    ("cent", '¢'),
    ("sect", '§'),
    ("para", '¶'),
    ("frac12", '½'),
    ("frac14", '¼'),
    ("frac34", '¾'),
    ("larr", '←'),
    ("rarr", '→'),
    ("uarr", '↑'),
    ("darr", '↓'),
    ("harr", '↔'),
    ("rArr", '⇒'),
    ("lArr", '⇐'),
    ("hArr", '⇔'),
    ("le", '≤'),
    ("ge", '≥'),
    ("ne", '≠'),
    ("asymp", '≈'),
    ("infin", '∞'),
    ("micro", 'µ'),
    ("auml", 'ä'),
    ("ouml", 'ö'),
    ("uuml", 'ü'),
    ("Auml", 'Ä'),
    ("Ouml", 'Ö'),
    ("Uuml", 'Ü'),
    ("szlig", 'ß'),
    ("eacute", 'é'),
    ("egrave", 'è'),
    ("aacute", 'á'),
    ("agrave", 'à'),
    ("ccedil", 'ç'),
    ("ntilde", 'ñ'),
    ("thinsp", '\u{2009}'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("shy", '\u{ad}'),
    ("zwj", '\u{200d}'),
    ("zwnj", '\u{200c}'),
    ("check", '✓'),
    ("minus", '−'),
    ("prime", '′'),
    ("Prime", '″'),
];

/// Reads an entity reference, `&name;`, `&#digits;` or `&#xhex;`, and
/// gives what it stands for.
pub(super) fn entity<'text>(scan: &mut Scan<'text>) -> Option<Entity<'text>> {
    let mut ahead = scan.clone();
    if !ahead.eat('&') {
        return None;
    }
    let name = ahead.take_while(|ch| ch.is_ascii_alphanumeric() || ch == '#');
    if !ahead.eat(';') {
        return None;
    }
    let found = match name.strip_prefix('#') {
        Some(number) => Entity::Char(numeric(number)?),
        None => named(name)?,
    };
    *scan = ahead;
    Some(found)
}

/// The character `&#number;` stands for: decimal, or hexadecimal after
/// `x`. A code that is no character, or zero, is the replacement
/// character.
fn numeric(number: &str) -> Option<char> {
    let (digits, radix, most) = number
        .strip_prefix(['x', 'X'])
        .map_or((number, 10, 7), |hex| (hex, 16, 6));
    if digits.is_empty() || digits.len() > most || !digits.chars().all(|ch| ch.is_digit(radix)) {
        return None;
    }
    let code = u32::from_str_radix(digits, radix).ok()?;
    Some(
        char::from_u32(code)
            .filter(|ch| *ch != '\0')
            .unwrap_or(char::REPLACEMENT_CHARACTER),
    )
}

/// What `&name;` stands for: a character when the name is a known one,
/// the reference itself when it is well formed.
fn named(name: &str) -> Option<Entity<'_>> {
    if name.is_empty()
        || name.len() > 31
        || !name.starts_with(|ch: char| ch.is_ascii_alphabetic())
        || !name.chars().all(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(
        ENTITIES
            .iter()
            .find(|(known, _)| *known == name)
            .map_or(Entity::Raw(name), |(_, ch)| Entity::Char(*ch)),
    )
}

/// Reads an entity reference that stands for a character this tool
/// knows, and gives the character.
fn known_char(scan: &mut Scan<'_>) -> Option<char> {
    let mut ahead = scan.clone();
    match entity(&mut ahead)? {
        Entity::Char(ch) => {
            *scan = ahead;
            Some(ch)
        }
        Entity::Raw(_) => None,
    }
}

/// Text with its known entity references decoded.
pub(crate) fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut scan = Scan::new(text);
    while !scan.is_done() {
        out.push_str(scan.take_while(|ch| ch != '&'));
        if let Some(ch) = known_char(&mut scan).or_else(|| scan.bump()) {
            out.push(ch);
        }
    }
    out
}

/// Backslash escapes and entity references resolved, as in a link's
/// destination or a fence's info string.
pub(crate) fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut scan = Scan::new(text);
    while let Some(ch) = scan.peek() {
        match ch {
            '\\' => {
                scan.bump();
                out.push(
                    scan.eat_if(|next| next.is_ascii_punctuation())
                        .unwrap_or('\\'),
                );
            }
            '&' => {
                if let Some(decoded) = known_char(&mut scan).or_else(|| scan.bump()) {
                    out.push(decoded);
                }
            }
            _ => out.push_str(scan.take_while(|next| next != '\\' && next != '&')),
        }
    }
    out
}
