// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading the author's HTML: tags out of the text, a name and
//! attributes out of a tag, the end of an element that goes with its
//! contents. Nothing here decides what passes.

use alloc::collections::BTreeSet;

use crate::inline::html_tag;
use crate::scan::Scan;

/// One piece of the author's HTML.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Piece<'text> {
    /// Text between tags.
    Text(&'text str),
    /// A whole opening or closing tag.
    Tag(&'text str),
    /// A comment, declaration, processing instruction or CDATA section:
    /// never part of the page.
    Comment,
    /// A `<` that starts no tag.
    Stray,
}

/// The author's HTML, read piece by piece.
#[derive(Debug)]
pub(super) struct Markup<'text> {
    /// What has not been read yet.
    rest: &'text str,
}

impl<'text> Markup<'text> {
    /// A reader at the start of `text`.
    pub(super) const fn new(text: &'text str) -> Self {
        Self { rest: text }
    }

    /// What has not been read yet.
    pub(super) const fn rest(&self) -> &'text str {
        self.rest
    }

    /// Reads the next piece.
    pub(super) fn next_piece(&mut self) -> Option<Piece<'text>> {
        if self.rest.is_empty() {
            return None;
        }
        let mut scan = Scan::new(self.rest);
        let piece = match scan.take_until("<") {
            None => Piece::Text(scan.take_rest()),
            Some(text) if !text.is_empty() => Piece::Text(text),
            Some(_) => {
                if let Some(tag) = tag_at(scan.rest()) {
                    scan.eat_str(tag);
                    if tag.starts_with("<!") || tag.starts_with("<?") {
                        Piece::Comment
                    } else {
                        Piece::Tag(tag)
                    }
                } else {
                    scan.bump();
                    Piece::Stray
                }
            }
        };
        self.rest = scan.rest();
        Some(piece)
    }

    /// Reads past the end tag of `name`: an element that never closes
    /// takes the rest with it.
    pub(super) fn skip_element(&mut self, name: &str) {
        self.rest = after_close(self.rest, name);
    }
}

/// The tag, comment or declaration `text` starts with, as
/// [`html_tag`] reads it; None when there is none.
fn tag_at(text: &str) -> Option<&str> {
    html_tag(text)
}

/// What an opening tag holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Opening<'tag> {
    /// The element's name, lowercased.
    pub(super) name: String,
    /// The text after the name, where the attributes stand.
    pub(super) after_name: &'tag str,
}

/// What a whole tag (`<` to `>`) opens, or None for a closing tag, a
/// comment or a declaration.
pub(super) fn opens(tag: &str) -> Option<Opening<'_>> {
    let inner = tag.strip_prefix('<')?.strip_suffix('>')?;
    if inner.starts_with(['/', '!', '?']) {
        return None;
    }
    let mut scan = Scan::new(inner);
    let name = scan
        .take_while(|ch| !ch.is_whitespace() && ch != '/')
        .to_ascii_lowercase();
    Some(Opening {
        name,
        after_name: scan.rest(),
    })
}

/// One attribute of a tag, as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Attr<'tag> {
    /// Its name, lowercased.
    pub(super) name: String,
    /// Its value as written, entities not yet decoded; None when the
    /// attribute stands bare.
    pub(super) value: Option<&'tag str>,
}

/// A tag's attributes in the order they stand. The first of a repeated
/// name wins, as in the browser.
pub(super) fn attributes(text: &str) -> Vec<Attr<'_>> {
    let mut list = Vec::new();
    let mut names = BTreeSet::new();
    let mut scan = Scan::new(text);
    while !scan.is_done() {
        let written =
            scan.take_while(|ch| !ch.is_ascii_whitespace() && !matches!(ch, '=' | '/' | '>'));
        if written.is_empty() {
            // White space, a `/`, or an `=` or `>` that follows no name.
            scan.bump();
            continue;
        }
        scan.take_while(|ch| ch.is_ascii_whitespace());
        let value = scan.eat('=').then(|| attribute_value(&mut scan));
        let name = written.to_ascii_lowercase();
        if names.insert(name.clone()) {
            list.push(Attr { name, value });
        }
    }
    list
}

/// The value after an attribute's `=`: in quotes up to the matching
/// quote (or the end, when it never comes), or bare up to white space.
fn attribute_value<'tag>(scan: &mut Scan<'tag>) -> &'tag str {
    scan.take_while(|ch| ch.is_ascii_whitespace());
    match scan.eat_if(|ch| matches!(ch, '"' | '\'')) {
        Some(quote) => {
            let value = scan.take_while(|ch| ch != quote);
            scan.eat(quote);
            value
        }
        None => scan.take_while(|ch| !ch.is_ascii_whitespace()),
    }
}

/// What stands before the end tag of `name` (case aside), and the end
/// tag onward; None when the element never closes. As in the browser,
/// the name ends at `>`, `/` or white space: `</scripts>` does not end a
/// `script`.
fn split_at_close<'text>(text: &'text str, name: &str) -> Option<(&'text str, &'text str)> {
    let mut scan = Scan::new(text);
    while scan.take_until("</").is_some() {
        let close = scan.rest();
        if close
            .strip_prefix("</")
            .and_then(|tag| strip_prefix_ignoring_case(tag, name))
            .is_some_and(|after| {
                after
                    .chars()
                    .next()
                    .is_none_or(|ch| ch == '>' || ch == '/' || ch.is_ascii_whitespace())
            })
        {
            return Some((scan.seen(), close));
        }
        scan.bump();
    }
    None
}

/// `text` after `word`, when it starts with `word`, ASCII case aside.
fn strip_prefix_ignoring_case<'text>(text: &'text str, word: &str) -> Option<&'text str> {
    let mut chars = text.chars();
    word.chars()
        .all(|want| {
            chars
                .next()
                .is_some_and(|ch| ch.eq_ignore_ascii_case(&want))
        })
        .then_some(chars.as_str())
}

/// The contents of an element of `name` up to its end tag; all of
/// `text` when it never closes.
pub(super) fn before_close<'text>(text: &'text str, name: &str) -> &'text str {
    split_at_close(text, name).map_or(text, |(inside, _)| inside)
}

/// The text after the end tag of `name`, or nothing when it never
/// closes: an unclosed script takes the rest with it.
pub(super) fn after_close<'text>(text: &'text str, name: &str) -> &'text str {
    split_at_close(text, name)
        .and_then(|(_, close)| close.split_once('>'))
        .map_or("", |(_, after)| after)
}

#[cfg(test)]
mod tests {
    use super::{Attr, Markup, Opening, Piece, after_close, attributes, before_close, opens};
    use crate::text::put;

    /// The attributes as (name, value) pairs, for comparing.
    fn pairs(text: &str) -> Vec<(String, Option<&str>)> {
        attributes(text)
            .into_iter()
            .map(|Attr { name, value }| (name, value))
            .collect()
    }

    #[test]
    fn attributes_read_as_the_browser_reads_them() {
        let got = pairs(" A=1 b = 'two words' c=\"x>y\" bare d= e=\"open");
        let want = [
            ("a", Some("1")),
            ("b", Some("two words")),
            ("c", Some("x>y")),
            ("bare", None),
            ("d", Some("e=\"open")),
        ];
        assert_eq!(got.len(), want.len(), "{got:?}");
        for ((name, value), (want_name, want_value)) in got.iter().zip(want) {
            assert_eq!((name.as_str(), *value), (want_name, want_value));
        }
        // The first of a repeated name wins, whatever its case.
        assert_eq!(pairs("id=one ID=two id=three"), [(
            "id".to_owned(),
            Some("one")
        )]);
        // Separators and stray `=` or `>` start no attribute.
        assert_eq!(pairs("/ = > / x=/y"), [("x".to_owned(), Some("/y"))]);
        // A quote that never closes takes the rest; a bare `=` at the end
        // is an empty value.
        assert_eq!(pairs("a='b c"), [("a".to_owned(), Some("b c"))]);
        assert_eq!(pairs("a="), [("a".to_owned(), Some(""))]);
        assert_eq!(pairs(""), Vec::new());
    }

    #[test]
    fn attributes_step_over_characters_beyond_ascii_whole() {
        // Only ASCII white space separates; anything else is part of a
        // name or a value, never cut.
        assert_eq!(pairs("t\u{e9}st=\u{e9}\u{a0}x n\u{3000}"), [
            ("t\u{e9}st".to_owned(), Some("\u{e9}\u{a0}x")),
            ("n\u{3000}".to_owned(), None)
        ]);
        assert_eq!(pairs("a=\"\u{1f600}\"\u{1f600}"), [
            ("a".to_owned(), Some("\u{1f600}")),
            ("\u{1f600}".to_owned(), None)
        ]);
    }

    #[test]
    fn thousands_of_attributes_take_no_time() {
        let mut tag = String::new();
        for n in 0_u32..50_000 {
            put!(tag, " a{n}=1 a{n}=2");
        }
        assert_eq!(attributes(&tag).len(), 50_000);
    }

    #[test]
    fn opening_names_are_read_up_to_space_slash_or_end() {
        for (tag, name, after_name) in [
            ("<SCRIPT src=x>", "script", " src=x"),
            ("<br/>", "br", "/"),
            ("<Style>", "style", ""),
            ("<meta\tname=a />", "meta", "\tname=a /"),
        ] {
            let opening = Opening {
                name: name.to_owned(),
                after_name,
            };
            assert_eq!(opens(tag), Some(opening), "{tag}");
        }
        for none in [
            "</p>",
            "<!-- x -->",
            "<!DOCTYPE html>",
            "<?xml?>",
            "p>",
            "<p",
        ] {
            assert_eq!(opens(none), None, "{none}");
        }
    }

    #[test]
    fn an_element_ends_at_its_end_tag_in_any_case() {
        assert_eq!(after_close("a</b>c</ScRiPt >d", "script"), "d");
        assert_eq!(before_close("p{}</STYLE>q", "style"), "p{}");
        // A longer name is another element, as in the browser.
        assert_eq!(after_close("x</scripts>y</script>z", "script"), "z");
        assert_eq!(
            before_close("x</scripts>y</script>z", "script"),
            "x</scripts>y"
        );
        assert_eq!(after_close("x</SCRIPT >y", "script"), "y");
        assert_eq!(after_close("x</script/>y", "script"), "y");
        // Never closed: the element takes the rest.
        assert_eq!(after_close("alert(1)", "script"), "");
        assert_eq!(after_close("alert(1)</script", "script"), "");
        assert_eq!(before_close("p{}", "style"), "p{}");
        // A close that only overlaps another `</` is still found.
        assert_eq!(after_close("</</script>z", "script"), "z");
        assert_eq!(after_close("\u{e9}</script>\u{e9}", "script"), "\u{e9}");
    }

    #[test]
    fn markup_comes_apart_into_its_pieces() {
        let mut markup = Markup::new("a<b>c<!-- d --><? e ?>< f</b><");
        let mut pieces = Vec::new();
        while let Some(piece) = markup.next_piece() {
            pieces.push(piece);
        }
        assert_eq!(pieces, [
            Piece::Text("a"),
            Piece::Tag("<b>"),
            Piece::Text("c"),
            Piece::Comment,
            Piece::Comment,
            Piece::Stray,
            Piece::Text(" f"),
            Piece::Tag("</b>"),
            Piece::Stray,
        ]);
        assert_eq!(markup.rest(), "");
        let mut skipping = Markup::new("<script>x</script>y");
        assert_eq!(skipping.next_piece(), Some(Piece::Tag("<script>")));
        skipping.skip_element("script");
        assert_eq!(skipping.next_piece(), Some(Piece::Text("y")));
        assert_eq!(skipping.next_piece(), None);
    }
}
