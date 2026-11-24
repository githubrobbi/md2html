// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The author's elements, balanced: whatever raw HTML a text holds, what
//! it opens is closed inside the element of md2html's it was opened in,
//! and it closes nothing it did not open.
//!
//! md2html writes its own elements around the author's text: the article,
//! a list item, a quote, a callout, a footnote, each run of inline text
//! and the emphasis and links inside it. Each is a scope
//! ([`Balance::enter`]): what the author opened in it is closed before
//! md2html closes it ([`Balance::leave`]). An HTML block is no scope, so
//! `<details>` in one HTML block and `</details>` in a later one hold the
//! Markdown between them, as the author meant.
//!
//! An end tag closes the latest element of its name open in the scope,
//! and every element opened after it, as `</div>` does in HTML; one that
//! matches nothing open there is left out. A start tag ends what HTML lets
//! an author leave open ([`IMPLIED`]): a paragraph before a block, a list
//! item before the next. A void element is never open; `<div/>` is an
//! empty `div`, in HTML as in the XHTML of a book. What had to be mended
//! is told ([`Mend`]), except an end tag HTML lets the author leave out.

use alloc::collections::BTreeMap;
use core::mem;

use super::lists::{BREAKS_OUT, ELEMENTS, LETS_HTML_IN, VOID};
use crate::scan::Scan;
use crate::text::put;

/// Elements whose end tag HTML lets an author leave out: closing one of
/// them for the author mends nothing worth telling.
const OPTIONAL_END: &[&str] = &[
    "p", "li", "dt", "dd", "td", "th", "tr", "thead", "tbody", "tfoot", "caption", "colgroup",
    "rp", "rt",
];

/// The roots of drawings and formulas, where `<x/>` closes itself.
const FOREIGN: &[&str] = &["svg", "math"];

/// The start tags of blocks: each ends an open paragraph, as in HTML.
const BLOCK_STARTS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dd",
    "details",
    "dir",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "ul",
];

/// Elements an open list item or definition does not reach out of.
const LIST_FENCES: &[&str] = &[
    "ul", "ol", "dl", "table", "td", "th", "caption", "svg", "math",
];

/// What a start tag ends that the author may leave open.
struct Implied {
    /// The elements it ends: the latest one open.
    ends: &'static [&'static str],
    /// The start tags that end them.
    by: &'static [&'static str],
    /// Elements it does not reach out of: one of [`Self::ends`] opened
    /// before the latest of these stays open.
    within: &'static [&'static str],
}

/// A block, the author's or md2html's, ends an open paragraph.
const PARAGRAPH: Implied = Implied {
    ends: &["p"],
    by: BLOCK_STARTS,
    within: &["caption", "table", "td", "th", "svg", "math"],
};

/// Everything a start tag may end, as HTML's parser ends it.
const IMPLIED: [Implied; 6] = [
    PARAGRAPH,
    Implied {
        ends: &["li"],
        by: &["li"],
        within: LIST_FENCES,
    },
    Implied {
        ends: &["dt", "dd"],
        by: &["dt", "dd"],
        within: LIST_FENCES,
    },
    Implied {
        ends: &["td", "th"],
        by: &["td", "th", "tr", "thead", "tbody", "tfoot"],
        within: &["table"],
    },
    Implied {
        ends: &["tr"],
        by: &["tr", "thead", "tbody", "tfoot"],
        within: &["table"],
    },
    Implied {
        ends: &["thead", "tbody", "tfoot"],
        by: &["thead", "tbody", "tfoot"],
        within: &["table"],
    },
];

/// What md2html changed in the author's markup to balance it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    /// An end tag that closed nothing open was left out.
    LeftOut,
    /// An element the author did not close was closed by md2html.
    Closed,
}

/// One change to the author's markup: what, to which element, and the line
/// of the Markdown the tag stands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mend {
    /// What was changed.
    change: Change,
    /// The element.
    name: &'static str,
    /// The line of the end tag left out, or of the start tag of the element
    /// closed; zero when unknown.
    line: usize,
}

impl Mend {
    /// The line of the Markdown it is about, from one; zero when unknown.
    pub(crate) const fn line(self) -> usize {
        self.line
    }

    /// What the author is told.
    pub(crate) fn message(self) -> String {
        let name = self.name;
        match self.change {
            Change::LeftOut => {
                format!("`</{name}>` closes nothing that is open here; it is left out")
            }
            Change::Closed => format!("`<{name}>` is not closed; md2html closes it"),
        }
    }
}

/// How the author's tags are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    /// Balanced, as every page and fragment is.
    Balanced,
    /// As written: what the `CommonMark` examples expect.
    AsWritten,
}

/// An element the author opened and has not closed.
#[derive(Clone, Copy, Debug)]
struct Open {
    /// Its name, from the sanitizer's lists.
    name: &'static str,
    /// The line it was opened on.
    line: usize,
    /// It is an element of a drawing or a formula (SVG or `MathML`), as
    /// HTML's parser reads it: `svg` and `math`, and what opens inside one
    /// of their elements that does not let HTML in.
    foreign: bool,
}

/// Where a scope began: what was open before it stays open after it.
#[must_use = "a scope entered must be left, or what the author opened in it stays open"]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Floor(usize);

/// The author's open elements, across the whole of an article.
#[derive(Debug)]
pub(crate) struct Balance {
    /// Balanced or as written.
    policy: Policy,
    /// The open elements, the latest last.
    open: Vec<Open>,
    /// For each name, the places in [`Self::open`] where it is open, so an
    /// end tag finds its element without a walk down the stack.
    at: BTreeMap<&'static str, Vec<usize>>,
    /// The first place of the current scope.
    floor: usize,
    /// What was mended, not told yet.
    mends: Vec<Mend>,
}

/// One of the author's tags as the sanitizer wrote it.
#[derive(Clone, Copy, Debug)]
struct Tag {
    /// The element.
    name: &'static str,
    /// Start, end or self-closing.
    kind: TagKind,
}

/// What a tag does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TagKind {
    /// `<x ...>`.
    Start,
    /// `</x>`.
    End,
    /// `<x .../>`.
    SelfClosing,
}

impl Tag {
    /// The tag `whole` (`<` to `>`), or None when it names no element of
    /// the sanitizer's lists.
    fn read(whole: &str) -> Option<Self> {
        let inside = whole.strip_prefix('<')?.strip_suffix('>')?;
        let (kind, body) = match inside.strip_prefix('/') {
            Some(body) => (TagKind::End, body),
            None if inside.ends_with('/') => (TagKind::SelfClosing, inside),
            None => (TagKind::Start, inside),
        };
        let written = Scan::new(body).take_while(|ch| ch.is_ascii_alphanumeric());
        let name = ELEMENTS.iter().copied().find(|known| *known == written)?;
        Some(Self { name, kind })
    }
}

impl Balance {
    /// A record that balances the author's tags.
    pub(crate) const fn new() -> Self {
        Self::with(Policy::Balanced)
    }

    /// A record that writes the author's tags as written, for the
    /// `CommonMark` examples.
    pub(crate) const fn as_written() -> Self {
        Self::with(Policy::AsWritten)
    }

    /// An empty record with this policy.
    const fn with(policy: Policy) -> Self {
        Self {
            policy,
            open: Vec::new(),
            at: BTreeMap::new(),
            floor: 0,
            mends: Vec::new(),
        }
    }

    /// Enters one of md2html's elements: what the author opens until
    /// [`Self::leave`] is closed there.
    pub(crate) fn enter(&mut self) -> Floor {
        let below = Floor(self.floor);
        self.floor = self.open.len();
        below
    }

    /// Leaves the element entered last: closes, into `out`, what the author
    /// opened in it.
    pub(crate) fn leave(&mut self, below: Floor, out: &mut String) {
        self.unwind(self.floor, out);
        self.floor = below.0;
    }

    /// Writes the author's HTML, cleaned by the sanitizer (tags as it
    /// writes them, text with every `<` escaped), from `line` of the
    /// Markdown.
    pub(crate) fn write(&mut self, html: &str, line: usize, out: &mut String) {
        if self.policy == Policy::AsWritten {
            out.push_str(html);
            return;
        }
        let mut scan = Scan::new(html);
        while let Some(text) = scan.take_until("<") {
            out.push_str(text);
            let start = scan.mark();
            let whole = scan.take_until(">").is_some();
            scan.bump();
            if whole {
                self.tag(scan.since(start), line, out);
            } else {
                // A `<` that ends no tag is text.
                out.push_str("&lt;");
            }
        }
        out.push_str(scan.take_rest());
    }

    /// md2html's own block, `own`, is about to be written: if it starts
    /// with a block element, the author's open paragraph ends first, into
    /// `out`.
    pub(crate) fn before_own(&mut self, own: &str, out: &mut String) {
        let starts_block = own.strip_prefix('<').is_some_and(|rest| {
            let name = Scan::new(rest).take_while(|ch| ch.is_ascii_alphanumeric());
            BLOCK_STARTS.contains(&name)
        });
        if starts_block {
            self.end_implied(&PARAGRAPH, out);
        }
    }

    /// What was mended since the last call, in order.
    pub(crate) fn take_mends(&mut self) -> Vec<Mend> {
        mem::take(&mut self.mends)
    }

    /// One tag: what its start ends is closed, then it is written, and
    /// opened unless it is void or closes itself; left out when it names no
    /// element md2html knows (the sanitizer writes none such).
    fn tag(&mut self, whole: &str, line: usize, out: &mut String) {
        let Some(tag) = Tag::read(whole) else {
            return;
        };
        if tag.kind == TagKind::End {
            self.end(tag.name, line, out);
            return;
        }
        for implied in IMPLIED
            .iter()
            .filter(|implied| implied.by.contains(&tag.name))
        {
            self.end_implied(implied, out);
        }
        // HTML's parser leaves a drawing or a formula at an HTML start tag
        // (`<p>`, `<b>`, `<div>`) unless that tag stands where SVG or
        // MathML lets HTML in: it closes what is open of the drawing, and
        // so does the balance, so the two agree on what stays open.
        if BREAKS_OUT.contains(&tag.name) {
            while self.open.len() > self.floor && self.in_foreign() {
                self.unwind(self.open.len().saturating_sub(1), out);
            }
        }
        let void = VOID.contains(&tag.name);
        if tag.kind == TagKind::Start {
            out.push_str(whole);
            if !void {
                self.push(tag.name, line);
            }
        } else if void || self.in_foreign() {
            // `<rect/>` in a drawing closes itself.
            out.push_str(whole);
        } else {
            // `<div/>` is an empty `div`: HTML would leave it open.
            let start = whole.strip_suffix("/>").unwrap_or(whole);
            put!(out, "{}></{}>", start.trim_end(), tag.name);
        }
    }

    /// An end tag: closes its element and what was opened after it, or is
    /// left out when none of its name is open in the scope.
    fn end(&mut self, name: &'static str, line: usize, out: &mut String) {
        match self.latest(name) {
            Some(at) => {
                self.unwind(at.saturating_add(1), out);
                self.pop(out);
            }
            None => self.mends.push(Mend {
                change: Change::LeftOut,
                name,
                line,
            }),
        }
    }

    /// Ends the latest open element `implied` ends, if one is open in the
    /// scope and none of the elements it does not reach out of was opened
    /// after it.
    fn end_implied(&mut self, implied: &Implied, out: &mut String) {
        let nearest = implied
            .ends
            .iter()
            .chain(implied.within)
            .filter_map(|name| self.latest(name).map(|at| (at, *name)))
            .max();
        if let Some((at, name)) = nearest {
            if implied.ends.contains(&name) {
                self.unwind(at.saturating_add(1), out);
                self.pop(out);
            }
        }
    }

    /// Whether what opens next opens inside a drawing or a formula, as
    /// HTML's parser reads it: the latest open element is one of theirs,
    /// and not one that lets HTML in (an SVG `title`, a `MathML` `mi`).
    /// There `<x/>` closes itself, and an HTML start tag leaves it.
    fn in_foreign(&self) -> bool {
        self.open
            .last()
            .is_some_and(|open| open.foreign && !LETS_HTML_IN.contains(&open.name))
    }

    /// The place of the latest element named `name` open in the scope.
    fn latest(&self, name: &str) -> Option<usize> {
        self.at
            .get(name)
            .and_then(|places| places.last().copied())
            .filter(|&at| at >= self.floor)
    }

    /// Opens `name`, from `line`.
    fn push(&mut self, name: &'static str, line: usize) {
        self.at.entry(name).or_default().push(self.open.len());
        let foreign = FOREIGN.contains(&name) || self.in_foreign();
        self.open.push(Open {
            name,
            line,
            foreign,
        });
    }

    /// Closes the latest open element, into `out`.
    fn pop(&mut self, out: &mut String) -> Option<Open> {
        let open = self.open.pop()?;
        if let Some(places) = self.at.get_mut(open.name) {
            places.pop();
        }
        put!(out, "</{}>", open.name);
        Some(open)
    }

    /// Closes, into `out`, every element open from place `to` up, each
    /// one the author left open told unless HTML lets its end be left out.
    fn unwind(&mut self, to: usize, out: &mut String) {
        while self.open.len() > to {
            let Some(open) = self.pop(out) else {
                return;
            };
            if !OPTIONAL_END.contains(&open.name) {
                self.mends.push(Mend {
                    change: Change::Closed,
                    name: open.name,
                    line: open.line,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Balance, Change, Mend};

    /// The author's HTML `pieces`, each in a scope of its own when it is a
    /// list (`[...]`) and written as it stands otherwise, balanced; and what
    /// was mended.
    fn balanced(pieces: &[&str]) -> (String, Vec<Mend>) {
        let mut balance = Balance::new();
        let mut out = String::new();
        let top = balance.enter();
        for (line, piece) in (1..).zip(pieces) {
            match piece
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                Some(inside) => {
                    let floor = balance.enter();
                    out.push('[');
                    balance.write(inside, line, &mut out);
                    balance.leave(floor, &mut out);
                    out.push(']');
                }
                None => balance.write(piece, line, &mut out),
            }
        }
        balance.leave(top, &mut out);
        (out, balance.take_mends())
    }

    /// The balanced HTML alone.
    fn html(pieces: &[&str]) -> String {
        balanced(pieces).0
    }

    /// Balanced markup is written as it stands, across pieces too.
    ///
    /// # Panics
    ///
    /// When anything of markup that is already balanced is changed or
    /// mended: nested elements, a disclosure whose ends are pieces apart,
    /// void elements, a drawing's self-closing shapes, text with `&lt;`.
    #[test]
    fn balanced_markup_is_kept_as_written() {
        for pieces in [
            &["<div class=\"a\"><p>x</p></div>"][..],
            &[
                "<details open><summary>More</summary>",
                "<p>inside</p>",
                "</details>",
            ],
            &["a<br>b<hr><img src=\"x\" alt=\"y\"><wbr>c"],
            &["<svg viewBox=\"0 0 1 1\"><circle r=\"1\"/><path d=\"M0\"></path></svg>"],
            &["1 &lt; 2 <b>bold</b> 3 > 2"],
        ] {
            let (out, mends) = balanced(pieces);
            assert_eq!(out, pieces.concat(), "{pieces:?}");
            assert!(mends.is_empty(), "{mends:?} for {pieces:?}");
        }
    }

    /// An end tag that closes nothing is left out, wherever it stands, and
    /// the author is told on its line.
    ///
    /// # Panics
    ///
    /// When a stray `</div>`, `</details>` or `</p>` is written, at the top
    /// or in a scope whose element was opened outside it, or when the
    /// author is not told of each, on its line.
    #[test]
    fn an_end_tag_that_closes_nothing_is_left_out() {
        assert_eq!(html(&["a</div>b</details>c</p>"]), "abc", "at the top");
        assert_eq!(
            html(&["<div>", "[x</div>y]", "</div>"]),
            "<div>[xy]</div>",
            "the scope cannot close what was opened outside it"
        );
        let (_, mends) = balanced(&["</div>", "x", "</p>"]);
        assert_eq!(
            mends,
            [
                Mend {
                    change: Change::LeftOut,
                    name: "div",
                    line: 1
                },
                Mend {
                    change: Change::LeftOut,
                    name: "p",
                    line: 3
                }
            ],
            "told on its line"
        );
    }

    /// What the author leaves open is closed at the end of its scope, and
    /// the author told of it, except where HTML allows the end to be left
    /// out.
    ///
    /// # Panics
    ///
    /// When an open `div` or `details` is not closed at the end, or at the
    /// end of the scope it was opened in, innermost first; when the author
    /// is not told of them on the line they were opened on; or when a
    /// paragraph or a list item left open is told.
    #[test]
    fn what_is_left_open_is_closed_at_the_end_of_its_scope() {
        assert_eq!(
            html(&["<div>a", "<details><summary>s</summary>b"]),
            "<div>a<details><summary>s</summary>b</details></div>"
        );
        assert_eq!(html(&["[<div>a]", "b"]), "[<div>a</div>]b");
        let (_, mends) = balanced(&["<div>", "<details>", "<p>", "<ul><li>"]);
        assert_eq!(mends, [
            Mend {
                change: Change::Closed,
                name: "ul",
                line: 4
            },
            Mend {
                change: Change::Closed,
                name: "details",
                line: 2
            },
            Mend {
                change: Change::Closed,
                name: "div",
                line: 1
            },
        ]);
    }

    /// An end tag closes its element and everything opened after it; the
    /// end tags after that close nothing.
    ///
    /// # Panics
    ///
    /// When `<div><p></div></p>`, `<div><span></div>` or `<b><i></b></i>`
    /// does not become well nested, closing the later element first, or
    /// when the `span` the author did not close is not told.
    #[test]
    fn crossed_tags_are_nested() {
        assert_eq!(html(&["<div><p>a</div>b</p>"]), "<div><p>a</p></div>b");
        assert_eq!(html(&["<b><i>a</b>b</i>"]), "<b><i>a</i></b>b");
        let (out, mends) = balanced(&["<div><span>a</div>"]);
        assert_eq!(out, "<div><span>a</span></div>");
        assert_eq!(mends, [Mend {
            change: Change::Closed,
            name: "span",
            line: 1
        }]);
    }

    /// A start tag ends what HTML lets the author leave open: a paragraph
    /// before a block, a list item before the next, a cell before the next.
    ///
    /// # Panics
    ///
    /// When `<p>a<p>b`, `<p>a<div>`, `<li>` after `<li>`, `<dt>`/`<dd>`, or
    /// cells and rows without their end tags nest inside one another instead
    /// of following one another; when a list item in a nested list ends the
    /// outer one, or a paragraph in a cell ends one outside the table; or
    /// when any of it is told to the author.
    #[test]
    fn a_start_tag_ends_what_html_lets_be_left_open() {
        for (pieces, out) in [
            (&["<p>a<p>b"][..], "<p>a</p><p>b</p>"),
            (&["<p>a<div>c</div>"], "<p>a</p><div>c</div>"),
            (&["<ul><li>a<li>b</ul>"], "<ul><li>a</li><li>b</li></ul>"),
            (
                &["<ul><li>a<ul><li>b</ul></ul>"],
                "<ul><li>a<ul><li>b</li></ul></li></ul>",
            ),
            (
                &["<dl><dt>a<dd>b<dt>c</dl>"],
                "<dl><dt>a</dt><dd>b</dd><dt>c</dt></dl>",
            ),
            (
                &["<table><tr><td>a<td>b<tr><td>c</table>"],
                "<table><tr><td>a</td><td>b</td></tr><tr><td>c</td></tr></table>",
            ),
            (
                &["<p>a<table><tr><td><p>b<div>c</div></table>"],
                "<p>a</p><table><tr><td><p>b</p><div>c</div></td></tr></table>",
            ),
        ] {
            let (balanced_out, mends) = balanced(pieces);
            assert_eq!(balanced_out, out, "{pieces:?}");
            assert!(mends.is_empty(), "{mends:?} for {pieces:?}");
        }
    }

    /// md2html's own block ends an open paragraph of the author's, and
    /// nothing else.
    ///
    /// # Panics
    ///
    /// When a `<p>` the author left open is not ended before md2html's
    /// `<ul>` or `<p>`, is ended before its inline text, or when anything
    /// but a paragraph is ended.
    #[test]
    fn an_own_block_ends_the_authors_paragraph() {
        let mut balance = Balance::new();
        let mut out = String::new();
        balance.write("<div><p>a", 1, &mut out);
        balance.before_own("text", &mut out);
        assert_eq!(out, "<div><p>a", "inline text ends nothing");
        balance.before_own("<ul><li>x</li></ul>", &mut out);
        assert_eq!(out, "<div><p>a</p>", "a block ends the paragraph");
        balance.before_own("<p>y</p>", &mut out);
        assert_eq!(out, "<div><p>a</p>", "and nothing more");
    }

    /// A void element and a drawing's shape close themselves; any other
    /// element written to close itself is an empty one.
    ///
    /// # Panics
    ///
    /// When `<br>`, `<br/>` or `<img .../>` opens anything, when `<div/>` or
    /// `<span class="x"/>` is not written as an empty element, or when a
    /// shape inside `svg` or `math` is.
    #[test]
    fn void_and_self_closing_tags_open_nothing() {
        assert_eq!(
            html(&["<br><br/><img src=\"a\"/>x</br>"]),
            "<br><br/><img src=\"a\"/>x"
        );
        assert_eq!(
            html(&["<div/>a<span class=\"x\"/>b"]),
            "<div></div>a<span class=\"x\"></span>b"
        );
        assert_eq!(
            html(&["<svg><rect/></svg><math><mspace/></math>"]),
            "<svg><rect/></svg><math><mspace/></math>"
        );
    }

    /// An HTML start tag leaves a drawing or a formula as HTML's parser
    /// leaves it, so what the author writes after it is balanced as HTML.
    ///
    /// # Panics
    ///
    /// When `<b/>`, `<p>` or `<div/>` inside an `svg` does not close the
    /// drawing before it (the parser would have closed it there and kept the
    /// self-closed HTML element open over the rest of the page), when an
    /// element written to close itself after it is not written as an empty
    /// one, or when HTML inside an SVG `title` or a `MathML` `mi`, where HTML
    /// is let in, closes the drawing or the formula.
    #[test]
    fn an_html_tag_leaves_a_drawing_as_the_parser_does() {
        assert_eq!(
            html(&["<svg><b class=\"leak\"/> after"]),
            "<svg></svg><b class=\"leak\"></b> after"
        );
        assert_eq!(
            html(&["<svg><g><p></p><a href=\"https://e.org/\"/>x"]),
            "<svg><g></g></svg><p></p><a href=\"https://e.org/\"></a>x"
        );
        assert_eq!(
            html(&["<svg><p>x</p><div class=\"hide\"/>rest"]),
            "<svg></svg><p>x</p><div class=\"hide\"></div>rest"
        );
        assert_eq!(
            html(&["<svg><title><b>t</b></title><rect/></svg><math><mi><b>i</b></mi></math>"]),
            "<svg><title><b>t</b></title><rect/></svg><math><mi><b>i</b></mi></math>"
        );
    }

    /// A tag no list of the sanitizer names is not written; written as
    /// written, nothing is balanced.
    ///
    /// # Panics
    ///
    /// When `<script>` or a comment passes the balanced record, or when the
    /// record for the `CommonMark` examples changes anything.
    #[test]
    fn only_known_elements_pass_and_as_written_changes_nothing() {
        assert_eq!(html(&["1<script>2</script><!-- 3 -->4"]), "124");
        let mut balance = Balance::as_written();
        let mut out = String::new();
        let floor = balance.enter();
        balance.write("</div><div><p>", 1, &mut out);
        balance.leave(floor, &mut out);
        assert_eq!(out, "</div><div><p>");
    }

    /// Finding the element an end tag closes takes no walk down the stack.
    ///
    /// # Panics
    ///
    /// When 50,000 open `div`s and 50,000 stray `</span>`s, then as many
    /// `</div>`s, do not come out balanced: every `</span>` left out,
    /// every `div` closed once.
    #[test]
    fn many_open_elements_stay_cheap() {
        let text = format!(
            "{}{}{}",
            "<div>".repeat(50_000),
            "</span>".repeat(50_000),
            "</div>".repeat(50_000)
        );
        let (out, mends) = balanced(&[&text]);
        assert_eq!(
            out,
            format!("{}{}", "<div>".repeat(50_000), "</div>".repeat(50_000))
        );
        assert_eq!(mends.len(), 50_000, "each stray end tag told");
    }
}
