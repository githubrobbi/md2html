// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! md2html's HTML rewritten as the XHTML an EPUB is made of.

use crate::scan::Scan;
use crate::text::put;

/// Elements HTML writes without an end tag.
const VOID: [&str; 13] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// XML's own named references, kept by name.
const XML_NAMED: [&str; 5] = ["amp", "lt", "gt", "quot", "apos"];

/// The other named references HTML pages use, as code points.
const NAMED: [(&str, u32); 36] = [
    ("nbsp", 0xA0),
    ("middot", 0xB7),
    ("mdash", 0x2014),
    ("ndash", 0x2013),
    ("hellip", 0x2026),
    ("lsaquo", 0x2039),
    ("rsaquo", 0x203A),
    ("laquo", 0xAB),
    ("raquo", 0xBB),
    ("ldquo", 0x201C),
    ("rdquo", 0x201D),
    ("lsquo", 0x2018),
    ("rsquo", 0x2019),
    ("copy", 0xA9),
    ("reg", 0xAE),
    ("trade", 0x2122),
    ("times", 0xD7),
    ("divide", 0xF7),
    ("larr", 0x2190),
    ("uarr", 0x2191),
    ("rarr", 0x2192),
    ("darr", 0x2193),
    ("harr", 0x2194),
    ("bull", 0x2022),
    ("deg", 0xB0),
    ("plusmn", 0xB1),
    ("sect", 0xA7),
    ("para", 0xB6),
    ("euro", 0x20AC),
    ("pound", 0xA3),
    ("shy", 0xAD),
    ("zwj", 0x200D),
    ("zwnj", 0x200C),
    ("thinsp", 0x2009),
    ("ensp", 0x2002),
    ("emsp", 0x2003),
];

/// A reference's name is shorter than this; a longer run after `&` is text.
const NAME_LIMIT: usize = 32;

/// md2html's HTML as well-formed XHTML: void elements closed, every
/// attribute quoted and given a value, named character references other
/// than XML's five written as numbers, comments and scripts dropped.
pub(super) fn xhtml(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + html.len() / 16);
    let mut scan = Scan::new(html);
    while let Some(before) = scan.take_until("<") {
        escape(before, Place::Text, &mut out);
        if scan.eat_str("<!--") {
            skip_past(&mut scan, "-->");
        } else if scan.starts_with("<script") {
            skip_past(&mut scan, "</script>");
        } else {
            scan.eat('<');
            tag(take_tag(&mut scan), &mut out);
        }
    }
    escape(scan.take_rest(), Place::Text, &mut out);
    out
}

/// Reads past the next `end`, or to the end of the text when none comes.
fn skip_past(scan: &mut Scan<'_>, end: &str) {
    if scan.take_until(end).is_some() {
        scan.eat_str(end);
    } else {
        scan.take_rest();
    }
}

/// Reads a tag after its `<`, past its `>`, and gives what is between;
/// a `>` inside quotes does not end it. A tag that never ends runs to the
/// end of the text.
fn take_tag<'text>(scan: &mut Scan<'text>) -> &'text str {
    let start = scan.mark();
    let mut quote = None;
    while let Some(ch) = scan.peek() {
        match quote {
            None if ch == '>' => {
                let inside = scan.since(start);
                scan.eat('>');
                return inside;
            }
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            Some(open) if ch == open => quote = None,
            None | Some(_) => {}
        }
        scan.bump();
    }
    scan.since(start)
}

/// Writes one tag, given what stands between its `<` and `>`. A tag
/// without a name XML would take is written as text.
fn tag(inside: &str, out: &mut String) {
    if let Some(name) = inside.strip_prefix('/') {
        put!(out, "</{}>", name.trim());
        return;
    }
    let (open, closed) = inside
        .strip_suffix('/')
        .map_or((inside, false), |open| (open, true));
    let mut scan = Scan::new(open);
    let name = scan.take_while(|ch| !ch.is_whitespace());
    let named = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == ':');
    if !named {
        escape(&format!("<{open}>"), Place::Text, out);
        return;
    }
    out.push('<');
    out.push_str(name);
    for attribute in attributes(scan.rest()) {
        attribute.write(out);
    }
    let void = closed || VOID.iter().any(|void| name.eq_ignore_ascii_case(void));
    out.push_str(if void { "/>" } else { ">" });
}

/// One attribute of a tag.
struct Attribute<'tag> {
    /// Its name, as written.
    name: &'tag str,
    /// Its value, unquoted; a bare attribute has none.
    value: Option<&'tag str>,
}

impl Attribute<'_> {
    /// Writes ` name="value"`; a bare attribute takes its name as value.
    fn write(&self, out: &mut String) {
        put!(out, " {}=\"", self.name);
        match self.value {
            Some(value) => escape(value, Place::Attribute, out),
            None => out.push_str(self.name),
        }
        out.push('"');
    }
}

/// A tag's attributes, after its name: the first of each name, and only
/// those whose name XML takes.
fn attributes(text: &str) -> Vec<Attribute<'_>> {
    let mut list: Vec<Attribute<'_>> = Vec::new();
    let mut scan = Scan::new(text);
    loop {
        scan.take_while(|ch| ch.is_whitespace() || ch == '/');
        if scan.is_done() {
            return list;
        }
        let name = scan.take_while(|ch| !ch.is_whitespace() && ch != '=' && ch != '/');
        if name.is_empty() {
            // A `=` with no name before it.
            scan.bump();
            continue;
        }
        scan.take_while(char::is_whitespace);
        let value = scan.eat('=').then(|| attribute_value(&mut scan));
        let valid = name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'));
        if valid && !list.iter().any(|seen| seen.name == name) {
            list.push(Attribute { name, value });
        }
    }
}

/// Reads an attribute's value after its `=`: quoted, to the same quote,
/// or else to the next white space.
fn attribute_value<'tag>(scan: &mut Scan<'tag>) -> &'tag str {
    scan.take_while(char::is_whitespace);
    match scan.eat_if(|ch| ch == '"' || ch == '\'') {
        Some(quote) => {
            let value = scan.take_while(|ch| ch != quote);
            scan.eat(quote);
            value
        }
        None => scan.take_while(|ch| !ch.is_whitespace()),
    }
}

/// Where text is written, which decides what it must escape.
#[derive(Clone, Copy)]
enum Place {
    /// Between tags.
    Text,
    /// Inside a quoted attribute value.
    Attribute,
}

impl Place {
    /// What `ch` is written as here when it cannot stand as itself; `&`
    /// starts a reference and is not one of these.
    const fn entity(self, ch: char) -> Option<&'static str> {
        match (self, ch) {
            (Self::Text | Self::Attribute, '<') => Some("&lt;"),
            (Self::Text, '>') => Some("&gt;"),
            (Self::Attribute, '"') => Some("&quot;"),
            (Self::Text | Self::Attribute, _) => None,
        }
    }
}

/// Writes `raw` with its character references made XML's and the
/// characters `place` cannot hold escaped.
fn escape(raw: &str, place: Place, out: &mut String) {
    let mut scan = Scan::new(raw);
    loop {
        out.push_str(scan.take_while(|ch| ch != '&' && place.entity(ch).is_none()));
        let Some(ch) = scan.bump() else {
            return;
        };
        // What stopped the text above: an escaped character, or a `&`.
        match place.entity(ch) {
            Some(entity) => out.push_str(entity),
            None => reference(&mut scan, out),
        }
    }
}

/// What a character reference becomes in XML.
enum Reference {
    /// One of XML's five: kept as written.
    Kept,
    /// A character XML allows, written as its number.
    Code(u32),
    /// A number that names no character XML allows.
    Invalid,
    /// A name XML would not know: the `&` is text.
    Unknown,
}

impl Reference {
    /// What the reference `&name;` is.
    fn of(name: &str) -> Self {
        if let Some(number) = name.strip_prefix('#') {
            return number
                .strip_prefix(['x', 'X'])
                .map_or_else(|| number.parse::<u32>(), |hex| u32::from_str_radix(hex, 16))
                .ok()
                .and_then(char::from_u32)
                .filter(|ch| xml_char(*ch))
                .map_or(Self::Invalid, |ch| Self::Code(u32::from(ch)));
        }
        if XML_NAMED.contains(&name) {
            return Self::Kept;
        }
        NAMED
            .iter()
            .find(|(named, _)| *named == name)
            .map_or(Self::Unknown, |(_, code)| Self::Code(*code))
    }
}

/// After a `&`: reads a reference and writes it for XML. A `&` that
/// starts none is written `&amp;`, and what follows it stays unread.
fn reference(scan: &mut Scan<'_>, out: &mut String) {
    let start = scan.mark();
    let name = scan.take_while(|ch| ch.is_ascii_alphanumeric() || ch == '#');
    let found = if name.len() < NAME_LIMIT && scan.eat(';') {
        Reference::of(name)
    } else {
        Reference::Unknown
    };
    match found {
        Reference::Kept => put!(out, "&{name};"),
        Reference::Code(code) => put!(out, "&#{code};"),
        Reference::Invalid => out.push_str("&#xFFFD;"),
        Reference::Unknown => {
            out.push_str("&amp;");
            scan.rewind(start);
        }
    }
}

/// A character XML 1.0 allows.
fn xml_char(ch: char) -> bool {
    matches!(u32::from(ch), 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x1_0000..=0x10_FFFF)
}

#[cfg(test)]
mod tests {
    use super::xhtml;

    /// md2html's HTML becomes XHTML an e-reader's XML parser accepts.
    ///
    /// # Panics
    ///
    /// Panics when the output differs at all from what is expected: `&nbsp;`
    /// and `&middot;` as numbers, a bare `&` escaped, `<br>`, `<img>` and
    /// `<input>` closed, bare and valueless attributes quoted and given values,
    /// and the comment dropped.
    #[test]
    fn html_becomes_well_formed_xhtml() {
        let out = xhtml(
            "<p>a&nbsp;b &middot; c & d<br><img src=\"x.png\" alt=a loading=lazy><input type=checkbox checked disabled></p><!-- gone -->",
        );
        assert_eq!(
            out,
            "<p>a&#160;b &#183; c &amp; d<br/><img src=\"x.png\" alt=\"a\" loading=\"lazy\"/><input type=\"checkbox\" checked=\"checked\" disabled=\"disabled\"/></p>"
        );
    }

    /// Character references become ones XML reads, and broken ones become the
    /// replacement character.
    ///
    /// # Panics
    ///
    /// Panics when `&amp;` is not kept, when `&#x41;` is not written `&#65;`,
    /// when `&#0;`, `&#xZZ;` or a number past U+10FFFF is not written as
    /// U+FFFD, or when an unknown name, or one of 40 letters, is not escaped as
    /// text.
    #[test]
    fn references_become_what_xml_reads() {
        assert_eq!(
            xhtml("&amp; &#x41; &#66; &#0; &#xZZ; &bogus; &#1114112;"),
            "&amp; &#65; &#66; &#xFFFD; &#xFFFD; &amp;bogus; &#xFFFD;"
        );
        let name = "a".repeat(40);
        assert_eq!(xhtml(&format!("&{name};")), format!("&amp;{name};"));
    }

    /// Every attribute comes out once, in double quotes, and a name that is not
    /// one is dropped.
    ///
    /// # Panics
    ///
    /// Panics when the single-quoted title loses its text or its `"` are not
    /// escaped, when the second `title` is kept, when `=y` is not made the
    /// attribute `y="y"`, when `data-a&b` is kept, or when the spaced `href =
    /// q` is not read as `href="q"`.
    #[test]
    fn attributes_are_quoted_once_each() {
        assert_eq!(
            xhtml("<a title='say \"hi\" > bye' title=x =y data-a&b=1 href = q>"),
            "<a title=\"say &quot;hi&quot; > bye\" y=\"y\" href=\"q\">"
        );
    }

    /// A `<` that opens no tag stays text, and scripts and open comments are
    /// dropped.
    ///
    /// # Panics
    ///
    /// Panics when the `<` and `>` of `1 < 2 > 0` are not escaped, when a `<`
    /// at the end is not written as the text `&lt;&gt;`, when `</b >` is not
    /// written `</b>`, or when a script or a comment that never closes is
    /// kept.
    #[test]
    fn what_is_not_a_tag_stays_text() {
        assert_eq!(xhtml("1 < 2 > 0"), "1 &lt; 2 &gt; 0");
        assert_eq!(xhtml("a <"), "a &lt;&gt;");
        assert_eq!(xhtml("<b>x</b >"), "<b>x</b>");
        assert_eq!(xhtml("<script>x</script>y<!-- z"), "y");
    }
}
