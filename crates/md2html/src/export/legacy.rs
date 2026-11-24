// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! HTML 4's presentational markup, said the way EPUB's XHTML says it.
//!
//! README files still centre with `<p align="center">`, float an image with
//! `<img align="right">` and size a table with `width`. A browser takes
//! them; EPUB's XHTML refuses them. In the book each becomes its
//! equivalent: a class of the book's style sheet ([`STYLE`]) for an
//! alignment, an inline style for a size, a margin or a colour, a current
//! element for `<center>` and `<strike>`. What has no equivalent a book
//! could use is left out and named once per kind. Inline SVG and `MathML`
//! keep their own attributes, which are theirs to have.

use super::conform::{each_tag, tag_name};
use crate::scan::Scan;
use crate::text::put;

/// The book's classes for HTML 4's alignments.
pub(super) const STYLE: &str = ".align-left{text-align:left}.align-center{text-align:center}\
.align-right{text-align:right}.align-justify{text-align:justify}\
.box-left{margin-left:0;margin-right:auto}.box-center,.align-center>table{margin-left:auto;margin-right:auto}\
.box-right{margin-left:auto;margin-right:0}\
.float-left{float:left;margin:0 1em .5em 0}.float-right{float:right;margin:0 0 .5em 1em}\
.valign-top{vertical-align:top}.valign-middle{vertical-align:middle}\
.valign-bottom{vertical-align:bottom}.valign-baseline{vertical-align:baseline}";

/// Elements whose `width` and `height` XHTML keeps, as whole pixels.
const SIZED: [&str; 8] = [
    "img", "video", "canvas", "iframe", "embed", "object", "source", "input",
];

/// Attributes HTML 5 dropped on every element, with nothing a book could
/// put in their place.
const OBSOLETE: [&str; 10] = [
    "cellpadding",
    "summary",
    "compact",
    "noshade",
    "frame",
    "rules",
    "char",
    "charoff",
    "axis",
    "longdesc",
];

/// What becomes of one attribute in the book.
#[derive(Debug, PartialEq, Eq)]
enum Effect {
    /// It stays as it is.
    Keep,
    /// It stays, with this value.
    Value(&'static str),
    /// It becomes a class of the book's style sheet.
    Class(&'static str),
    /// It becomes declarations of the element's inline style.
    Style(String),
    /// It goes, and nothing is lost with it.
    Gone,
    /// It goes with no equivalent, and the author is told.
    Dropped,
}

/// The chapter with HTML 4's presentational markup rewritten for EPUB.
///
/// Inline SVG and `MathML` stay as they are. The second value names each
/// element and attribute left out without an equivalent, once per kind.
pub(super) fn modernize(xhtml: &str) -> (String, Vec<String>) {
    let mut chapter = Chapter::default();
    let out = each_tag(xhtml, |inside, out| chapter.tag(inside, out));
    (out, chapter.problems)
}

/// Where the rewrite stands in the chapter, and what it has left out.
#[derive(Default)]
struct Chapter {
    /// How many elements of inline SVG or `MathML` are open around the tag.
    embedded: usize,
    /// How many elements that hold text alone (a paragraph, a heading, a
    /// link ...) are open around the tag.
    phrasing: usize,
    /// What each open `<center>` and `<div>` became: `div`, or `span`
    /// inside text.
    became: Vec<&'static str>,
    /// Each kind of element and attribute left out, named once.
    problems: Vec<String>,
}

/// What a tag does to the element it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TagKind {
    /// `<p>`: opens it.
    Opening,
    /// `</p>`: closes it.
    Closing,
    /// `<br/>`: is all of it.
    Empty,
}

impl TagKind {
    /// The kind of the tag `inside` is the inside of.
    pub(super) fn of(inside: &str) -> Self {
        if inside.starts_with('/') {
            Self::Closing
        } else if inside.ends_with('/') {
            Self::Empty
        } else {
            Self::Opening
        }
    }
}

impl Chapter {
    /// Writes one tag, given what stands between its `<` and `>`.
    fn tag(&mut self, inside: &str, out: &mut String) {
        let name = tag_name(inside);
        let element = name.to_ascii_lowercase();
        let kind = TagKind::of(inside);
        if matches!(element.as_str(), "svg" | "math") || self.embedded > 0 {
            self.embedded = depth(self.embedded, kind);
            put!(out, "<{inside}>");
            return;
        }
        if PHRASING.contains(&element.as_str()) {
            self.phrasing = depth(self.phrasing, kind);
        }
        let renamed = match element.as_str() {
            "center" | "div" if kind == TagKind::Closing => {
                put!(out, "</{}>", self.became.pop().unwrap_or("div"));
                return;
            }
            "center" | "div" => {
                let tag = if self.phrasing > 0 { "span" } else { "div" };
                if kind == TagKind::Opening {
                    self.became.push(tag);
                }
                match element.as_str() {
                    "center" => Some((tag, Some("align-center"))),
                    _ => (tag == "span").then_some((tag, None)),
                }
            }
            "strike" => Some(("s", None)),
            _ => None,
        };
        if kind == TagKind::Closing {
            put!(out, "</{}>", renamed.map_or(name, |(tag, _)| tag));
        } else {
            rewrite(inside, &element, renamed, out, &mut self.problems);
        }
    }
}

/// Elements that hold text alone: a `<center>` or a `<div>` inside one
/// becomes a `span`, since a `div` may not stand there (HTML's parser puts
/// an author's `<div>` in a heading; XHTML does not take it).
const PHRASING: [&str; 33] = [
    "p", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "summary", "a", "abbr", "b", "bdi", "bdo",
    "cite", "code", "dfn", "em", "i", "kbd", "label", "mark", "q", "s", "samp", "small", "span",
    "strike", "strong", "sub", "sup", "u", "var",
];

/// The count of open elements after one more tag.
///
/// An opening tag adds one, a closing tag takes one away, and a tag that
/// closes itself leaves the count as it was.
const fn depth(count: usize, kind: TagKind) -> usize {
    match kind {
        TagKind::Closing => count.saturating_sub(1),
        TagKind::Opening => count.saturating_add(1),
        TagKind::Empty => count,
    }
}

/// Writes one start tag of the chapter as EPUB's XHTML has it.
///
/// `inside` is what stands between its `<` and `>`, `element` its name in
/// lower case and `renamed` the current element (and class) for an element
/// HTML 5 dropped. A tag with nothing to change is written as it was.
fn rewrite(
    inside: &str,
    element: &str,
    renamed: Option<(&'static str, Option<&'static str>)>,
    out: &mut String,
    problems: &mut Vec<String>,
) {
    let name = tag_name(inside);
    let Some(list) = inside.strip_prefix(name).and_then(attributes) else {
        put!(out, "<{inside}>");
        return;
    };
    let mut tag = Tag {
        name: renamed.map_or(name, |(tag, _)| tag),
        changed: renamed.is_some(),
        kept: String::new(),
        class: None,
        classes: renamed.and_then(|(_, class)| class).into_iter().collect(),
        style: None,
        styles: Vec::new(),
    };
    for (attr, value) in list {
        tag.take(element, attr, value, problems);
    }
    if tag.changed {
        tag.write(TagKind::of(inside), out);
    } else {
        put!(out, "<{inside}>");
    }
}

/// A start tag being rewritten.
struct Tag<'tag> {
    /// The element's name, as it is written in the book.
    name: &'tag str,
    /// Whether anything differs from the tag as the page has it.
    changed: bool,
    /// The attributes kept, each written ` name="value"`.
    kept: String,
    /// The tag's own classes, as written.
    class: Option<&'tag str>,
    /// The classes the book's style sheet gives it.
    classes: Vec<&'static str>,
    /// The tag's own inline style, as written.
    style: Option<&'tag str>,
    /// The declarations added to its inline style.
    styles: Vec<String>,
}

impl<'tag> Tag<'tag> {
    /// Takes one attribute of the tag of `element` in, rewritten.
    fn take(
        &mut self,
        element: &str,
        attr: &'tag str,
        value: &'tag str,
        problems: &mut Vec<String>,
    ) {
        let lower = attr.to_ascii_lowercase();
        if lower == "class" {
            self.class = Some(value);
            return;
        }
        if lower == "style" {
            self.style = Some(value);
            return;
        }
        let effect = effect(element, &lower, value);
        self.changed |= effect != Effect::Keep;
        match effect {
            Effect::Keep => put!(self.kept, " {attr}=\"{value}\""),
            Effect::Value(fixed) => put!(self.kept, " {attr}=\"{fixed}\""),
            Effect::Class(class) => self.classes.push(class),
            Effect::Style(style) => self.styles.push(style),
            Effect::Gone => {}
            Effect::Dropped => {
                let problem = format!(
                    "epub: <{element} {lower}> is left out: EPUB's XHTML has neither it nor an equivalent"
                );
                if !problems.contains(&problem) {
                    problems.push(problem);
                }
            }
        }
    }

    /// Writes the tag, with `/` before its `>` when it is empty.
    fn write(&self, kind: TagKind, out: &mut String) {
        put!(out, "<{}{}", self.name, self.kept);
        let classes = self.class.into_iter().chain(self.classes.iter().copied());
        let class = classes.collect::<Vec<_>>().join(" ");
        if !class.is_empty() {
            put!(out, " class=\"{class}\"");
        }
        let styles = self
            .style
            .map(|style| style.trim().trim_end_matches(';'))
            .filter(|style| !style.is_empty())
            .into_iter()
            .chain(self.styles.iter().map(String::as_str));
        let style = styles.collect::<Vec<_>>().join(";");
        if !style.is_empty() {
            put!(out, " style=\"{style}\"");
        }
        out.push_str(if kind == TagKind::Empty { "/>" } else { ">" });
    }
}

/// A tag's attributes, after its name, as XHTML writes them: ` name="value"`.
///
/// Anything else (a tag this module did not get from the XHTML writer)
/// gives none, and the tag stays as it is.
fn attributes(rest: &str) -> Option<Vec<(&str, &str)>> {
    let mut list = Vec::new();
    let mut scan = Scan::new(rest);
    loop {
        scan.take_while(|ch| ch.is_whitespace() || ch == '/');
        if scan.is_done() {
            return Some(list);
        }
        let name = scan.take_while(|ch| ch != '=' && ch != '/' && !ch.is_whitespace());
        if name.is_empty() || !scan.eat_str("=\"") {
            return None;
        }
        let value = scan.take_while(|ch| ch != '"');
        if !scan.eat('"') {
            return None;
        }
        list.push((name, value));
    }
}

/// What becomes of `attr="value"` on `element` (both names lower case).
fn effect(element: &str, attr: &str, value: &str) -> Effect {
    if obsolete(element, attr) {
        return Effect::Dropped;
    }
    let style = |declarations: Option<String>| declarations.map_or(Effect::Dropped, Effect::Style);
    match attr {
        "align" => align(element, value),
        "valign" => vertical(value).map_or(Effect::Dropped, Effect::Class),
        "width" | "height" => size(element, attr, value),
        "border" => border(element, value),
        "bgcolor" => style(color(value).map(|color| format!("background-color:{color}"))),
        "nowrap" => Effect::Style("white-space:nowrap".to_owned()),
        "hspace" => style(length(value).map(|len| format!("margin-left:{len};margin-right:{len}"))),
        "vspace" => style(length(value).map(|len| format!("margin-top:{len};margin-bottom:{len}"))),
        "cellspacing" => style(length(value).map(|len| format!("border-spacing:{len}"))),
        "clear" if element == "br" => clear(value),
        "type" if element == "ul" || element == "li" => {
            style(list_style(value).map(|kind| format!("list-style-type:{kind}")))
        }
        _ => Effect::Keep,
    }
}

/// Whether `attr` on `element` is one HTML 5 dropped with nothing to put
/// in its place.
fn obsolete(element: &str, attr: &str) -> bool {
    OBSOLETE.contains(&attr)
        || (element == "hr" && attr == "size")
        || (element == "td" && matches!(attr, "scope" | "abbr"))
}

/// `align`: a float or a vertical alignment for an image, a margin for a
/// table or a rule, the side for a caption, a text alignment elsewhere.
fn align(element: &str, value: &str) -> Effect {
    let side = value.trim().to_ascii_lowercase();
    let class = match element {
        "img" => match side.as_str() {
            "left" => Some("float-left"),
            "right" => Some("float-right"),
            other => vertical(other),
        },
        "table" | "hr" => match side.as_str() {
            "left" => Some("box-left"),
            "center" | "middle" => Some("box-center"),
            "right" => Some("box-right"),
            _ => None,
        },
        "caption" if matches!(side.as_str(), "top" | "bottom") => {
            return Effect::Style(format!("caption-side:{side}"));
        }
        _ => match side.as_str() {
            "left" => Some("align-left"),
            "center" | "middle" => Some("align-center"),
            "right" => Some("align-right"),
            "justify" => Some("align-justify"),
            _ => None,
        },
    };
    class.map_or(Effect::Dropped, Effect::Class)
}

/// The class for a vertical alignment (`valign`, or `align` on an image).
fn vertical(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "top" | "texttop" => Some("valign-top"),
        "middle" | "absmiddle" | "center" => Some("valign-middle"),
        "bottom" | "absbottom" => Some("valign-bottom"),
        "baseline" => Some("valign-baseline"),
        _ => None,
    }
}

/// `width` or `height`: kept in whole pixels where XHTML has them, an
/// inline style elsewhere; a preformatted block's width counts characters
/// and has no equivalent.
fn size(element: &str, attr: &str, value: &str) -> Effect {
    if element == "pre" {
        return Effect::Dropped;
    }
    let pixels = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    if SIZED.contains(&element) && pixels {
        return Effect::Keep;
    }
    length(value).map_or(Effect::Dropped, |len| {
        Effect::Style(format!("{attr}:{len}"))
    })
}

/// `border`: on a table, `1` for any border and nothing for none; on an
/// image, `0` stays and a width becomes a style.
fn border(element: &str, value: &str) -> Effect {
    let width = value.trim();
    let whole = !width.is_empty() && width.bytes().all(|byte| byte.is_ascii_digit());
    match element {
        "table" if width.is_empty() || width == "1" => Effect::Keep,
        "table" if whole && width.bytes().all(|byte| byte == b'0') => Effect::Gone,
        "table" if whole => Effect::Value("1"),
        "img" if width == "0" => Effect::Keep,
        "img" if whole => Effect::Style(format!("border:{width}px solid")),
        "table" | "img" => Effect::Dropped,
        _ => Effect::Keep,
    }
}

/// `clear` on a line break: the side to clear, or nothing for `none`.
fn clear(value: &str) -> Effect {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => Effect::Style("clear:left".to_owned()),
        "right" => Effect::Style("clear:right".to_owned()),
        "all" | "both" => Effect::Style("clear:both".to_owned()),
        "none" => Effect::Gone,
        _ => Effect::Dropped,
    }
}

/// The list style a list's or an item's `type` names; a letter or a
/// numeral is case-sensitive, as HTML 4 had it.
fn list_style(value: &str) -> Option<&'static str> {
    match value.trim() {
        "1" => Some("decimal"),
        "a" => Some("lower-alpha"),
        "A" => Some("upper-alpha"),
        "i" => Some("lower-roman"),
        "I" => Some("upper-roman"),
        other => match other.to_ascii_lowercase().as_str() {
            "disc" => Some("disc"),
            "circle" => Some("circle"),
            "square" => Some("square"),
            _ => None,
        },
    }
}

/// A length as CSS writes it: whole or decimal pixels (`120`, `120px`) or a
/// percentage (`50%`). Anything else, which could carry more CSS, is none.
fn length(value: &str) -> Option<String> {
    let written = value.trim();
    let (number, unit) = written.strip_suffix('%').map_or_else(
        || (written.strip_suffix("px").unwrap_or(written), "px"),
        |number| (number, "%"),
    );
    let mut parts = number.split('.');
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let whole = parts.next().is_some_and(digits);
    let fraction = parts.next().is_none_or(digits);
    (whole && fraction && parts.next().is_none()).then(|| format!("{number}{unit}"))
}

/// A colour as HTML 4 wrote it: `#` and three to eight hexadecimal digits,
/// or a name of letters only. Anything else, which could carry more CSS, is
/// none.
fn color(value: &str) -> Option<&str> {
    let written = value.trim();
    let valid = written.strip_prefix('#').map_or_else(
        || {
            (1..=32).contains(&written.len())
                && written.bytes().all(|byte| byte.is_ascii_alphabetic())
        },
        |hex| (3..=8).contains(&hex.len()) && hex.bytes().all(|byte| byte.is_ascii_hexdigit()),
    );
    valid.then_some(written)
}

#[cfg(test)]
mod tests;
