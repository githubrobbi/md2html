// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The lexer the languages of [`super::lang`] share.
//!
//! It is a list of token kinds, each a recogniser that reads one token
//! where it stands, tried in order at every place; a character no kind
//! takes is written as it is.

use super::lang::{Feature, Lang};
use super::{Class, read_through, span};
use crate::scan::{Mark, Scan};
use crate::text::esc_into;

/// The code as highlighted HTML, read with `lang`'s tables.
pub(super) fn lex(code: &str, lang: &Lang) -> String {
    let mut lexer = Lexer {
        scan: Scan::new(code),
        lang,
        out: String::with_capacity(code.len().saturating_mul(2)),
    };
    while !lexer.scan.is_done() {
        if !Lexer::RECOGNISERS
            .iter()
            .any(|recognise| recognise(&mut lexer))
        {
            lexer.other();
        }
    }
    lexer.out
}

/// Whether `ch` can be part of a word.
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// The lexer's state: the code, the language and the HTML so far.
struct Lexer<'code> {
    /// The code, read up to where the lexer stands.
    scan: Scan<'code>,
    /// The language's tables.
    lang: &'code Lang,
    /// The HTML written so far.
    out: String,
}

impl Lexer<'_> {
    /// The token kinds, in the order they are tried: the first that takes
    /// a token wins. Each reads one token of its kind where the lexer
    /// stands and writes it, and says whether there was one; one that
    /// finds none reads nothing.
    const RECOGNISERS: [fn(&mut Self) -> bool; 10] = [
        Self::directive,
        Self::block_comment,
        Self::line_comment,
        Self::string,
        Self::variable,
        Self::annotation,
        Self::attribute,
        Self::colour,
        Self::number,
        Self::word,
    ];

    /// Whether the language has `feature`.
    fn has(&self, feature: Feature) -> bool {
        self.lang.has(feature)
    }

    /// Whether the next character is `ch`.
    fn next_is(&self, ch: char) -> bool {
        self.scan.peek() == Some(ch)
    }

    /// Writes what was read since `from` as a span of `class`.
    fn emit(&mut self, class: Class, from: Mark) {
        span(&mut self.out, class, self.scan.since(from));
    }

    /// Reads to the end of the line, which stays unread, and writes what
    /// was read since `from` as a span of `class`.
    fn rest_of_line(&mut self, class: Class, from: Mark) {
        self.scan.take_while(|ch| ch != '\n');
        self.emit(class, from);
    }

    /// A character no token kind takes, as it is.
    fn other(&mut self) {
        let text = self.scan.take_chars(1);
        esc_into(&mut self.out, text);
    }

    /// `#include` and the like: a `#` that only white space precedes on
    /// its line, to the line's end.
    fn directive(&mut self) -> bool {
        if !self.has(Feature::HashDirective) || !self.next_is('#') {
            return false;
        }
        let seen = self.scan.seen();
        let line = seen.rsplit_once('\n').map_or(seen, |(_, line)| line);
        if !line.chars().all(char::is_whitespace) {
            return false;
        }
        let from = self.scan.mark();
        self.rest_of_line(Class::Meta, from);
        true
    }

    /// A block comment, to its close or to the end of the code.
    fn block_comment(&mut self) -> bool {
        let Some(block) = &self.lang.block else {
            return false;
        };
        let from = self.scan.mark();
        if !self.scan.eat_str(block.open) {
            return false;
        }
        read_through(&mut self.scan, block.close);
        self.emit(Class::Comment, from);
        true
    }

    /// A line comment, to the line's end.
    fn line_comment(&mut self) -> bool {
        let scan = &self.scan;
        let Some(marker) = self
            .lang
            .line
            .iter()
            .find(|marker| scan.starts_with(marker))
        else {
            return false;
        };
        if self.has(Feature::HashAfterSpace)
            && *marker == "#"
            && !self.scan.prev().is_none_or(char::is_whitespace)
        {
            return false;
        }
        let from = self.scan.mark();
        self.rest_of_line(Class::Comment, from);
        true
    }

    /// A string, with its quotes: a key where the language has string
    /// keys and a `:` follows it.
    fn string(&mut self) -> bool {
        let lang = self.lang;
        let Some(quote) = self.scan.peek().filter(|ch| lang.quotes.contains(*ch)) else {
            return false;
        };
        let from = self.scan.mark();
        let scan = &self.scan;
        let opening = triple(quote)
            .filter(|fence| lang.has(Feature::TripleQuotes) && scan.starts_with(fence));
        if let Some(fence) = opening {
            self.scan.eat_str(fence);
            long_string(&mut self.scan, fence);
        } else {
            self.scan.bump();
            short_string(&mut self.scan, quote);
        }
        let key = self.has(Feature::StringKeys)
            && self
                .scan
                .rest()
                .trim_start_matches([' ', '\t'])
                .starts_with(':');
        self.emit(if key { Class::Attr } else { Class::Str }, from);
        true
    }

    /// `$name` or `${name}`.
    fn variable(&mut self) -> bool {
        if !self.has(Feature::DollarVars)
            || !self.next_is('$')
            || !self
                .scan
                .peek_nth(1)
                .is_some_and(|ch| is_word_char(ch) || ch == '{')
        {
            return false;
        }
        let from = self.scan.mark();
        self.scan.bump();
        if self.scan.eat('{') {
            self.scan.take_while(|ch| ch != '}');
            self.scan.eat('}');
        } else {
            self.scan.take_while(is_word_char);
        }
        self.emit(Class::Type, from);
        true
    }

    /// `@name` where no word runs into it: an annotation, or a rule in a
    /// style sheet.
    fn annotation(&mut self) -> bool {
        if !self.has(Feature::AtMeta)
            || !self.next_is('@')
            || !self.scan.peek_nth(1).is_some_and(char::is_alphabetic)
            || self.scan.prev().is_some_and(is_word_char)
        {
            return false;
        }
        let from = self.scan.mark();
        self.scan.bump();
        self.scan
            .take_while(|ch| is_word_char(ch) || ch == '.' || ch == '-');
        let class = if self.has(Feature::StyleSheet) {
            Class::Keyword
        } else {
            Class::Meta
        };
        self.emit(class, from);
        true
    }

    /// `#[..]` or `#![..]`, to the bracket that closes the first one or to
    /// the line's end.
    fn attribute(&mut self) -> bool {
        if !self.has(Feature::Attributes)
            || !self.next_is('#')
            || !matches!(self.scan.peek_nth(1), Some('[' | '!'))
        {
            return false;
        }
        let from = self.scan.mark();
        let mut depth = 0_i32;
        while let Some(ch) = self.scan.eat_if(|ch| ch != '\n') {
            if ch == '[' {
                depth = depth.saturating_add(1);
            } else if ch == ']' {
                depth = depth.saturating_sub(1);
                if depth == 0_i32 {
                    break;
                }
            }
        }
        self.emit(Class::Meta, from);
        true
    }

    /// `#abc` in a style sheet.
    fn colour(&mut self) -> bool {
        if !self.has(Feature::StyleSheet)
            || !self.next_is('#')
            || !self
                .scan
                .peek_nth(1)
                .is_some_and(|ch| ch.is_ascii_hexdigit())
        {
            return false;
        }
        let from = self.scan.mark();
        self.scan.bump();
        self.scan.take_while(|ch| ch.is_ascii_alphanumeric());
        self.emit(Class::Number, from);
        true
    }

    /// A number where no word runs into it: digits, letters, `_` and a `.`
    /// before a digit; in a style sheet, the `%` after it too.
    fn number(&mut self) -> bool {
        if !self.scan.peek().is_some_and(|ch| ch.is_ascii_digit())
            || self.scan.prev().is_some_and(is_word_char)
        {
            return false;
        }
        let from = self.scan.mark();
        while continues_number(&self.scan) {
            self.scan.bump();
        }
        if self.has(Feature::StyleSheet) {
            self.scan.take_while(|ch| ch == '%');
        }
        self.emit(Class::Number, from);
        true
    }

    /// A word: a keyword, a type, a property in a style sheet, or plain.
    fn word(&mut self) -> bool {
        let style = self.has(Feature::StyleSheet);
        let starts = |ch: char| ch.is_alphabetic() || ch == '_' || (style && ch == '-');
        if !self.scan.peek().is_some_and(starts) {
            return false;
        }
        let before = self.scan.seen();
        let word = self
            .scan
            .take_while(|ch| is_word_char(ch) || (style && ch == '-'));
        let property = style
            && self.scan.rest().trim_start_matches(' ').starts_with(':')
            && before.trim_end().ends_with(['{', ';']);
        let marked = if property {
            Some(Class::Attr)
        } else {
            self.lang.class_of(word)
        };
        match marked {
            Some(class) => span(&mut self.out, class, word),
            None => esc_into(&mut self.out, word),
        }
        true
    }
}

/// The three quotes that open a string running across lines, for a quote
/// that has them.
const fn triple(quote: char) -> Option<&'static str> {
    match quote {
        '"' => Some("\"\"\""),
        '\'' => Some("'''"),
        _ => None,
    }
}

/// Reads the rest of a string that `quote` opened.
///
/// It reads to the closing quote (read), to the line's end (unread) unless
/// the quote is a backtick, or to the end of the code. A backslash escapes
/// the character after it.
fn short_string(scan: &mut Scan<'_>, quote: char) {
    while let Some(ch) = scan.eat_if(|ch| ch != '\n' || quote == '`') {
        if ch == '\\' {
            scan.bump();
        } else if ch == quote {
            break;
        }
    }
}

/// Reads the rest of a string that `fence` opened, through the `fence`
/// that closes it or to the end of the code. A backslash escapes the
/// character after it.
fn long_string(scan: &mut Scan<'_>, fence: &str) {
    while !scan.is_done() && !scan.eat_str(fence) {
        if scan.bump() == Some('\\') {
            scan.bump();
        }
    }
}

/// Whether the next character continues a number.
fn continues_number(scan: &Scan<'_>) -> bool {
    match scan.peek() {
        Some('.') => scan.peek_nth(1).is_some_and(|ch| ch.is_ascii_digit()),
        Some(ch) => ch.is_ascii_alphanumeric() || ch == '_',
        None => false,
    }
}
