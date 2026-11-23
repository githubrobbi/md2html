// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The `MathML` elements, built from what the parser has read.

use crate::text::{esc, put};

/// Elements in reading order: one cell of a table, or a whole formula.
pub(super) type Cell = Vec<String>;

/// The cells of one table row.
pub(super) type Row = Vec<Cell>;

/// Elements as one: a single one alone, any other number in an `<mrow>`.
pub(super) fn mrow(items: &[String]) -> String {
    if items.len() == 1 {
        items.concat()
    } else {
        format!("<mrow>{}</mrow>", items.concat())
    }
}

/// Rows of cells as an `<mtable>`, with `attrs` on it.
pub(super) fn table(rows: &[Row], attrs: &str) -> String {
    let mut out = format!("<mtable{attrs}>");
    for row in rows {
        out.push_str("<mtr>");
        for cell in row {
            put!(out, "<mtd>{}</mtd>", mrow(cell));
        }
        out.push_str("</mtr>");
    }
    out.push_str("</mtable>");
    out
}

/// A base with what was written below (`_`) and above (`^`) it: as
/// limits under and over it, or as scripts beside it.
pub(super) fn scripts(
    base: String,
    sub: Option<String>,
    sup: Option<String>,
    limits: bool,
) -> String {
    let (under, over, both) = if limits {
        ("munder", "mover", "munderover")
    } else {
        ("msub", "msup", "msubsup")
    };
    match (sub, sup) {
        (None, None) => base,
        (Some(below), None) => format!("<{under}>{base}{below}</{under}>"),
        (None, Some(above)) => format!("<{over}>{base}{above}</{over}>"),
        (Some(below), Some(above)) => format!("<{both}>{base}{below}{above}</{both}>"),
    }
}

/// An accent `mark` over `body`.
pub(super) fn accent(body: &str, mark: &str) -> String {
    format!("<mover accent=\"true\">{body}<mo>{mark}</mo></mover>")
}

/// A delimiter that grows with what it encloses; nothing for an empty one.
pub(super) fn fence(shown: &str) -> String {
    if shown.is_empty() {
        String::new()
    } else {
        format!("<mo stretchy=\"true\">{shown}</mo>")
    }
}

/// A run of digits and points.
pub(super) fn number(digits: &str) -> String {
    if digits == "." {
        String::from("<mo>.</mo>")
    } else {
        format!("<mn>{digits}</mn>")
    }
}

/// A character that stands for itself: a letter, an operator or a
/// delimiter. None for one that has a meaning of its own in TeX and is out
/// of place here.
pub(super) fn character(ch: char) -> Option<String> {
    Some(match ch {
        '(' | ')' | '[' | ']' | '|' => format!("<mo stretchy=\"false\">{ch}</mo>"),
        '*' => String::from("<mo>∗</mo>"),
        '-' => String::from("<mo>−</mo>"),
        '~' => String::from("<mspace width=\"0.33em\"></mspace>"),
        '}' | '&' | '^' | '_' | '#' | '%' | '$' => return None,
        letter if letter.is_alphabetic() => format!("<mi>{letter}</mi>"),
        other => format!("<mo>{}</mo>", esc(other.encode_utf8(&mut [0; 4]))),
    })
}

/// The alphabets of `\mathbf`, `\mathbb`, `\mathcal`.
#[derive(Clone, Copy)]
pub(super) enum Style {
    /// Bold.
    Bold,
    /// Double-struck.
    Double,
    /// Script.
    Script,
}

impl Style {
    /// Where the alphabet sits in Unicode.
    const fn alphabet(self) -> &'static Alphabet {
        match self {
            Self::Bold => &BOLD,
            Self::Double => &DOUBLE,
            Self::Script => &SCRIPT,
        }
    }
}

/// Where a mathematical alphabet sits in Unicode.
struct Alphabet {
    /// The code of its `A`; `a` follows 26 places on.
    letters: u32,
    /// The code of its `0`, when it has digits.
    digits: Option<u32>,
    /// Letters Unicode had before the run and that the run leaves out.
    older: &'static [(char, char)],
}

/// Bold, with digits.
const BOLD: Alphabet = Alphabet {
    letters: 0x1_D400,
    digits: Some(0x1_D7CE),
    older: &[],
};

/// Double-struck, with digits.
const DOUBLE: Alphabet = Alphabet {
    letters: 0x1_D538,
    digits: Some(0x1_D7D8),
    older: &[
        ('C', 'ℂ'),
        ('H', 'ℍ'),
        ('N', 'ℕ'),
        ('P', 'ℙ'),
        ('Q', 'ℚ'),
        ('R', 'ℝ'),
        ('Z', 'ℤ'),
    ],
};

/// Script, letters only.
const SCRIPT: Alphabet = Alphabet {
    letters: 0x1_D49C,
    digits: None,
    older: &[
        ('B', 'ℬ'),
        ('E', 'ℰ'),
        ('F', 'ℱ'),
        ('H', 'ℋ'),
        ('I', 'ℐ'),
        ('L', 'ℒ'),
        ('M', 'ℳ'),
        ('R', 'ℛ'),
    ],
};

impl Alphabet {
    /// `ch` in this alphabet, or None when it has no such character.
    fn letter(&self, ch: char) -> Option<char> {
        let older = self
            .older
            .iter()
            .find_map(|&(plain, styled)| (plain == ch).then_some(styled));
        if older.is_some() {
            return older;
        }
        let code = u32::from(ch);
        let place = if ch.is_ascii_uppercase() {
            self.letters + code - u32::from('A')
        } else if ch.is_ascii_lowercase() {
            self.letters + 26 + code - u32::from('a')
        } else if ch.is_ascii_digit() {
            self.digits? + code - u32::from('0')
        } else {
            return None;
        };
        char::from_u32(place)
    }
}

/// Letters in a mathematical alphabet, as the Unicode characters for it;
/// None when one of them has no such character.
pub(super) fn styled(text: &str, style: Style) -> Option<String> {
    let alphabet = style.alphabet();
    let letters = text
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '{' && *ch != '}')
        .map(|ch| alphabet.letter(ch))
        .collect::<Option<String>>()?;
    Some(format!("<mi>{letters}</mi>"))
}
