// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Entity references (`&amp;`, `&#233;`, `&#x1F600;`) and backslash
//! escapes.

use crate::scan::Scan;

mod names;

/// What an entity reference stands for.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Entity {
    /// A numeric reference's character.
    Char(char),
    /// A named reference's characters: one, or two for a few names.
    Named(&'static str),
}

impl Entity {
    /// Adds the characters to `out`.
    pub(crate) fn push_to(self, out: &mut String) {
        match self {
            Self::Char(ch) => out.push(ch),
            Self::Named(chars) => out.push_str(chars),
        }
    }
}

/// Reads an entity reference, `&name;`, `&#digits;` or `&#xhex;`, and
/// gives what it stands for.
///
/// A name that is not one of HTML's is no reference: nothing is read, and
/// the `&` is text.
pub(crate) fn entity(scan: &mut Scan<'_>) -> Option<Entity> {
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
        None => Entity::Named(named(name)?),
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

/// The characters `&name;` stands for, when `name` is one of HTML's named
/// character references: a binary search in the one part of the sorted
/// table that could hold it.
fn named(name: &str) -> Option<&'static str> {
    let part = names::PARTS
        .iter()
        .find(|part| part.last().is_some_and(|(last, _)| name <= *last))?;
    let at = part
        .binary_search_by(|(known, _)| (*known).cmp(name))
        .ok()?;
    part.get(at).map(|(_, chars)| *chars)
}

/// Text with its entity references decoded; an `&` that starts none
/// stays as it is.
pub(crate) fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut scan = Scan::new(text);
    while !scan.is_done() {
        out.push_str(scan.take_while(|ch| ch != '&'));
        match entity(&mut scan) {
            Some(found) => found.push_to(&mut out),
            None => out.extend(scan.bump()),
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
            '&' => match entity(&mut scan) {
                Some(found) => found.push_to(&mut out),
                None => out.extend(scan.bump()),
            },
            _ => out.push_str(scan.take_while(|next| next != '\\' && next != '&')),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decode_entities, named, names};

    /// The table holds every one of HTML's names that end in `;`, in the
    /// order a binary search needs.
    ///
    /// # Panics
    ///
    /// Panics when the table does not hold 2,125 names, when a part is not
    /// sorted or does not lie wholly before the next, when a name is not found
    /// by the lookup, or when a name HTML does not know is.
    #[test]
    fn the_table_is_whole_and_sorted() {
        let all: Vec<&(&str, &str)> = names::PARTS.iter().flat_map(|part| part.iter()).collect();
        assert_eq!(all.len(), 2125);
        assert!(
            all.windows(2)
                .all(|pair| matches!(pair, [one, two] if one.0 < two.0))
        );
        for (name, chars) in all {
            assert_eq!(named(name), Some(*chars), "{name}");
        }
        for unknown in ["", "x", "MadeUpEntity", "amp;", "zzzz", "AAAA"] {
            assert_eq!(named(unknown), None, "{unknown}");
        }
    }

    /// Named references decode to their characters; a name HTML does not
    /// know stays as written.
    ///
    /// # Panics
    ///
    /// Panics when `&ngE;` is not its two characters, when `&AElig;`,
    /// `&HilbertSpace;` or `&#x1F600;` is not its character, or when
    /// `&bogus;` or `&amp` without its `;` is changed.
    #[test]
    fn references_decode_to_their_characters() {
        assert_eq!(
            decode_entities("&ngE; &AElig; &HilbertSpace; &#x1F600; &bogus; &amp"),
            "\u{2267}\u{338} \u{c6} \u{210b} \u{1f600} &bogus; &amp"
        );
    }
}
