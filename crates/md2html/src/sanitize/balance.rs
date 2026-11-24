// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The author's elements, balanced inside md2html's.
//!
//! Whatever raw HTML a text holds, what it opens is closed inside the
//! element of md2html's it was opened in, and it closes nothing it did not
//! open, neither by an end tag nor by a start tag HTML's parser would end
//! md2html's element at.
//!
//! md2html writes its own elements around the author's text: the article,
//! a list item, a quote, a callout, a footnote, each run of inline text
//! and the emphasis and links inside it. Each is a scope
//! ([`Balance::enter`], given the element as [`Own`]): what the author
//! opened in it is closed before md2html closes it ([`Balance::leave`]).
//! An HTML block is no scope, so `<details>` in one HTML block and
//! `</details>` in a later one hold the Markdown between them, as the
//! author meant.
//!
//! The record holds md2html's elements beside the author's, as the
//! browser's parser holds them all. An end tag closes the latest element
//! of its name open in the scope, and every element opened after it, as
//! `</div>` does in HTML; one that matches nothing open there is left out.
//! A start tag ends what HTML lets an author leave open ([`rules`]): a
//! paragraph before a block, a list item before the next; but a start tag
//! HTML would end one of md2html's elements at (a `<div>` in md2html's
//! paragraph, a `<li>` in its list item, a `<tr>` in its cell, an `<a>` in
//! its link) is left out, its end tag with it, and its contents kept.
//! md2html's own start tags go through the record too ([`Balance::own_start`]):
//! the author's drawing or table is closed before them, and the author's
//! paragraph ended before md2html's block. A void element is never open;
//! `<div/>` is an empty `div`, in HTML as in the XHTML of a book. What had
//! to be mended is told ([`Mend`]), except an end tag HTML lets the author
//! leave out.

mod rules;
#[cfg(test)]
mod tests;

use alloc::collections::BTreeMap;
use core::mem;

use self::rules::{
    FOREIGN, HEADINGS, IMPLIED_ENDS, OPTIONAL_END, RUBY, RUBY_TEXT, RULES, Rule, TABLE_CONTEXT,
    TABLE_PARTS, TABLE_STARTS,
};
use super::lists::{BREAKS_OUT, ELEMENTS, LETS_HTML_IN, VOID};
use crate::model::Level;
use crate::scan::Scan;
use crate::text::put;

/// What md2html changed in the author's markup to balance it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    /// An end tag that closed nothing open was left out.
    LeftOut,
    /// An element the author did not close was closed by md2html.
    Closed,
    /// A start tag that would end an element it stands in, which the
    /// author did not open there (md2html's own, or one around it), was
    /// left out, with its end tag.
    WouldEnd(&'static str),
    /// A table's part outside a table was left out, with its end tag, as
    /// HTML's parser drops it.
    NoTable,
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
            Change::NoTable => {
                format!("`<{name}>` stands in no table; it is left out, its contents kept")
            }
            Change::WouldEnd(around) => format!(
                "`<{name}>` would end the `<{around}>` it stands in, which HTML does not let hold it; it is left out, its contents kept"
            ),
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

/// One of md2html's own elements that holds the author's text, or none
/// when the text stands in the element around it (a tight list's
/// paragraph is its item's text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Own(Option<&'static str>);

impl Own {
    /// The article.
    pub(crate) const ARTICLE: Self = Self(Some("article"));
    /// A list item or a footnote.
    pub(crate) const ITEM: Self = Self(Some("li"));
    /// A block quote.
    pub(crate) const QUOTE: Self = Self(Some("blockquote"));
    /// A callout's body.
    pub(crate) const DIV: Self = Self(Some("div"));
    /// A paragraph.
    pub(crate) const PARAGRAPH: Self = Self(Some("p"));
    /// A table's head cell.
    pub(crate) const HEAD_CELL: Self = Self(Some("th"));
    /// A table's cell.
    pub(crate) const CELL: Self = Self(Some("td"));
    /// A figure's caption.
    pub(crate) const FIGCAPTION: Self = Self(Some("figcaption"));
    /// A glossary term's meaning.
    pub(crate) const DEFINITION: Self = Self(Some("dd"));
    /// The page's fine print.
    pub(crate) const FOOTER: Self = Self(Some("footer"));
    /// Emphasis.
    pub(crate) const EM: Self = Self(Some("em"));
    /// Strong emphasis.
    pub(crate) const STRONG: Self = Self(Some("strong"));
    /// Struck-through text.
    pub(crate) const DEL: Self = Self(Some("del"));
    /// A link.
    pub(crate) const LINK: Self = Self(Some("a"));
    /// A span of text.
    pub(crate) const SPAN: Self = Self(Some("span"));
    /// No element of its own.
    pub(crate) const NONE: Self = Self(None);

    /// A heading of `level`.
    pub(crate) fn heading(level: Level) -> Self {
        let index = usize::from(level.number()).saturating_sub(1);
        Self(HEADINGS.get(index).copied())
    }

    /// The element's name, from the sanitizer's lists or md2html's own.
    pub(crate) const fn name(self) -> Option<&'static str> {
        self.0
    }
}

/// Who opened an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Who {
    /// The author, in the text.
    Author,
    /// md2html, around the author's text.
    Md2html,
}

/// An element open: the author's or md2html's.
#[derive(Clone, Copy, Debug)]
struct Open {
    /// Its name, from the sanitizer's lists or md2html's own.
    name: &'static str,
    /// The line it was opened on.
    line: usize,
    /// It is an element of a drawing or a formula (SVG or `MathML`), as
    /// HTML's parser reads it: `svg` and `math`, and what opens inside one
    /// of their elements that does not let HTML in.
    foreign: bool,
    /// Whose it is.
    who: Who,
    /// Where the run of elements HTML ends together ([`IMPLIED_ENDS`]) it
    /// belongs to begins: its own place, or that of the element below it
    /// in the run. So `rt` finds how far its implied ends reach without a
    /// walk down the stack.
    run: usize,
}

/// A start tag of the author's that was left out: its end tag, when it
/// comes, closes it and is left out too.
#[derive(Clone, Copy, Debug)]
struct LeftOut {
    /// Its name.
    name: &'static str,
    /// How many elements were open below it: those opened later are inside
    /// it.
    at: usize,
}

/// Where a scope began: what was open before it stays open after it.
#[must_use = "a scope entered must be left, or what the author opened in it stays open"]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Floor {
    /// The first place of the scope around it.
    below: usize,
    /// The first place in the left-out start tags of the scope around it.
    left_below: usize,
    /// Whether md2html's own element was opened for it.
    own: bool,
}

/// The open elements, the author's and md2html's, across the whole of an
/// article.
#[derive(Debug)]
pub(crate) struct Balance {
    /// Balanced or as written.
    policy: Policy,
    /// The open elements, the latest last.
    open: Vec<Open>,
    /// For each name, the places in [`Self::open`] where it is open, so an
    /// end tag finds its element without a walk down the stack.
    at: BTreeMap<&'static str, Vec<usize>>,
    /// The author's start tags left out and not ended yet, in order.
    left_out: Vec<LeftOut>,
    /// For each name, the places in [`Self::left_out`] where it is.
    left_at: BTreeMap<&'static str, Vec<usize>>,
    /// The first place of the current scope: what is open below it is
    /// md2html's, or opened outside the scope.
    floor: usize,
    /// The first place in [`Self::left_out`] of the current scope: the
    /// start tags left out below it were left out outside it.
    left_floor: usize,
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
            left_out: Vec::new(),
            left_at: BTreeMap::new(),
            floor: 0,
            left_floor: 0,
            mends: Vec::new(),
        }
    }

    /// Enters md2html's element `own`, written just before: what the
    /// author opens until [`Self::leave`] is closed there, and nothing of
    /// the author's may end it.
    pub(crate) fn enter(&mut self, own: Own) -> Floor {
        let below = Floor {
            below: self.floor,
            left_below: self.left_floor,
            own: own.name().is_some(),
        };
        if let Some(name) = own.name() {
            self.push(name, 0, Who::Md2html);
        }
        self.floor = self.open.len();
        self.left_floor = self.left_out.len();
        below
    }

    /// Leaves the element entered last: closes, into `out`, what the author
    /// opened in it, before md2html writes its end tag.
    pub(crate) fn leave(&mut self, floor: Floor, out: &mut String) {
        self.unwind(self.floor, out);
        self.forget_left_out(self.left_floor);
        if floor.own {
            self.pop();
        }
        self.floor = floor.below;
        self.left_floor = floor.left_below;
    }

    /// md2html is about to write its own start tag `name` (a block, a
    /// heading, emphasis, a link...) where the author's text is: what of
    /// the author's cannot hold it is closed, into `out`, first. A
    /// drawing or a formula (HTML's parser would leave it at md2html's tag,
    /// or take the tag for a part of it), a table outside a cell (its
    /// parser would move md2html's element out), a paragraph before a
    /// block and a heading before a heading (HTML ends them there).
    pub(crate) fn own_start(&mut self, name: &'static str, out: &mut String) {
        if self.policy == Policy::AsWritten {
            return;
        }
        while let Some(top) = self.top_in_scope() {
            let holds_own = !self.in_foreign() && !TABLE_PARTS.contains(&top.name);
            if holds_own {
                break;
            }
            self.unwind(self.open.len().saturating_sub(1), out);
        }
        self.end_implied(name, out);
    }

    /// Whether md2html may write markup of its own here: not inside the
    /// author's drawing or formula, a table outside its cells, or a link
    /// (a link inside a link is no HTML, and the browser would end the
    /// author's link and what md2html opened in it). Where it may not, a
    /// link of md2html's is its text.
    pub(crate) fn takes_markup(&self) -> bool {
        if self.policy == Policy::AsWritten {
            return true;
        }
        let in_table = self
            .top_in_scope()
            .is_some_and(|top| TABLE_PARTS.contains(&top.name));
        let in_link = self
            .nearest(&["a", "td", "th", "caption"])
            .is_some_and(|(_, name)| name == "a");
        !self.in_foreign() && !in_table && !in_link
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

    /// What was mended since the last call, in order.
    pub(crate) fn take_mends(&mut self) -> Vec<Mend> {
        mem::take(&mut self.mends)
    }

    /// One tag: what its start ends is closed, then it is written, and
    /// opened unless it is void or closes itself; left out when it names no
    /// element md2html knows (the sanitizer writes none such), or when it
    /// would end an element of md2html's or one opened outside the scope.
    fn tag(&mut self, whole: &str, line: usize, out: &mut String) {
        let Some(tag) = Tag::read(whole) else {
            return;
        };
        if tag.kind == TagKind::End {
            self.end(tag.name, line, out);
            return;
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
        // In a drawing, a tag is the drawing's: HTML's rules do not apply.
        if !self.in_foreign() {
            let refused = self.would_end(tag.name).map(Change::WouldEnd).or_else(|| {
                let tableless =
                    TABLE_STARTS.contains(&tag.name) && self.nearest(TABLE_CONTEXT).is_none();
                tableless.then_some(Change::NoTable)
            });
            if let Some(change) = refused {
                self.mends.push(Mend {
                    change,
                    name: tag.name,
                    line,
                });
                if tag.kind == TagKind::Start && !void {
                    self.leave_out(tag.name);
                }
                return;
            }
            self.end_implied(tag.name, out);
        }
        if tag.kind == TagKind::Start {
            out.push_str(whole);
            if !void {
                self.push(tag.name, line, Who::Author);
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

    /// The element the start tag `name` would end that is md2html's or
    /// was opened outside the scope, if any: the browser would end it
    /// there.
    fn would_end(&self, name: &str) -> Option<&'static str> {
        let outside = |found: Option<(usize, &'static str)>, rule: &Rule| {
            found.filter(|&(at, found_name)| at < self.floor && rule.ends.contains(&found_name))
        };
        let by_rule = RULES
            .iter()
            .filter(|rule| rule.by.contains(&name))
            .find_map(|rule| outside(self.nearest_of(rule), rule));
        if let Some((_, around)) = by_rule {
            return Some(around);
        }
        let top = self.open.len().checked_sub(1)?;
        let current = self.open.last()?;
        if HEADINGS.contains(&name) && HEADINGS.contains(&current.name) && top < self.floor {
            return Some(current.name);
        }
        if RUBY_TEXT.contains(&name) && self.ruby_in_scope() {
            let run = IMPLIED_ENDS
                .contains(&current.name)
                .then_some(current.run)?;
            if run < self.floor {
                return self.open.get(run).map(|open| open.name);
            }
        }
        None
    }

    /// Ends, into `out`, what the start tag `name` ends of the author's
    /// in the scope, as HTML does: the paragraph before a block, the list
    /// item before the next, the heading before a heading, the ruby text
    /// before the next. Each is told unless HTML lets its end be left out.
    fn end_implied(&mut self, name: &str, out: &mut String) {
        if RUBY_TEXT.contains(&name) && self.ruby_in_scope() {
            if let Some(current) = self.open.last() {
                let run = current.run;
                if IMPLIED_ENDS.contains(&current.name) && run >= self.floor {
                    self.unwind(run, out);
                }
            }
        }
        for rule in RULES.iter().filter(|rule| rule.by.contains(&name)) {
            if let Some((at, found)) = self.nearest_of(rule) {
                if at >= self.floor && rule.ends.contains(&found) {
                    self.unwind(at, out);
                }
            }
        }
        if HEADINGS.contains(&name) {
            let top = self.open.len().saturating_sub(1);
            let heading_on_top = self
                .open
                .last()
                .is_some_and(|current| HEADINGS.contains(&current.name));
            if heading_on_top && top >= self.floor {
                self.unwind(top, out);
            }
        }
    }

    /// Whether a `ruby` is open in scope, as HTML's parser looks for one.
    fn ruby_in_scope(&self) -> bool {
        self.nearest_of(&RUBY)
            .is_some_and(|(_, name)| RUBY.ends.contains(&name))
    }

    /// The nearest open element `rule` ends or does not pass, the author's
    /// or md2html's, with its place.
    fn nearest_of(&self, rule: &Rule) -> Option<(usize, &'static str)> {
        let ends = self.nearest(rule.ends);
        let within = self.nearest(rule.within);
        // Of the two, the nearer; an element in both lists is one it ends.
        match (ends, within) {
            (Some(end), Some(wall)) if wall.0 > end.0 => Some(wall),
            (Some(end), _) => Some(end),
            (None, wall) => wall,
        }
    }

    /// The latest open element named one of `names`, with its place.
    fn nearest(&self, names: &[&'static str]) -> Option<(usize, &'static str)> {
        names
            .iter()
            .filter_map(|name| {
                let (key, places) = self.at.get_key_value(name)?;
                places.last().map(|&at| (at, *key))
            })
            .max()
    }

    /// An end tag: closes its element and what was opened after it, or is
    /// left out when none of its name is open in the scope. One whose
    /// start tag was left out closes that, and is left out too.
    fn end(&mut self, name: &'static str, line: usize, out: &mut String) {
        let opened = self.latest(name);
        let left = self
            .left_at
            .get(name)
            .and_then(|places| places.last().copied())
            .filter(|&place| place >= self.left_floor);
        let left_at = left.and_then(|place| self.left_out.get(place).map(|one| (place, one.at)));
        match (opened, left_at) {
            (Some(at), Some((_, below))) if at >= below => self.close(at, out),
            (Some(at), None) => self.close(at, out),
            (_, Some((place, below))) => {
                self.unwind(below, out);
                self.forget_left_out(place);
            }
            (None, None) => self.mends.push(Mend {
                change: Change::LeftOut,
                name,
                line,
            }),
        }
    }

    /// Closes the open element at `at`, the author's, at its end tag, and
    /// what was opened after it, into `out`: those are told unless HTML
    /// lets their end be left out.
    fn close(&mut self, at: usize, out: &mut String) {
        self.unwind(at.saturating_add(1), out);
        if let Some(open) = self.pop() {
            put!(out, "</{}>", open.name);
        }
    }

    /// Leaves out a start tag of the author's: its end tag will be left out
    /// too.
    fn leave_out(&mut self, name: &'static str) {
        self.left_at
            .entry(name)
            .or_default()
            .push(self.left_out.len());
        self.left_out.push(LeftOut {
            name,
            at: self.open.len(),
        });
    }

    /// Forgets the start tags left out from place `from` in
    /// [`Self::left_out`] on.
    fn forget_left_out(&mut self, from: usize) {
        while self.left_out.len() > from {
            let Some(one) = self.left_out.pop() else {
                return;
            };
            if let Some(places) = self.left_at.get_mut(one.name) {
                places.pop();
            }
        }
    }

    /// The current element, when it is in the scope.
    fn top_in_scope(&self) -> Option<&Open> {
        self.open.last().filter(|_| self.open.len() > self.floor)
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

    /// The place of the latest element named `name` open in the scope: the
    /// author's, as md2html's own stand below the scope.
    fn latest(&self, name: &str) -> Option<usize> {
        self.at
            .get(name)
            .and_then(|places| places.last().copied())
            .filter(|&at| at >= self.floor)
    }

    /// Opens `name`, from `line`, for `who`; its place.
    fn push(&mut self, name: &'static str, line: usize, who: Who) -> usize {
        let place = self.open.len();
        self.at.entry(name).or_default().push(place);
        let foreign = who == Who::Author && (FOREIGN.contains(&name) || self.in_foreign());
        let run = match self.open.last() {
            Some(below) if IMPLIED_ENDS.contains(&below.name) => below.run,
            _ => place,
        };
        self.open.push(Open {
            name,
            line,
            foreign,
            who,
            run,
        });
        place
    }

    /// Takes the latest open element off the record.
    fn pop(&mut self) -> Option<Open> {
        let open = self.open.pop()?;
        if let Some(places) = self.at.get_mut(open.name) {
            places.pop();
        }
        // A start tag left out inside it ends with it.
        let len = self.open.len();
        let inside = self.left_out.iter().rposition(|one| one.at <= len);
        self.forget_left_out(inside.map_or(0, |place| place.saturating_add(1)));
        Some(open)
    }

    /// Closes, into `out`, every element open from place `to` up, the
    /// author's, each one told unless HTML lets its end be left out.
    fn unwind(&mut self, to: usize, out: &mut String) {
        while self.open.len() > to {
            let Some(open) = self.pop() else {
                return;
            };
            put!(out, "</{}>", open.name);
            if open.who == Who::Author && !OPTIONAL_END.contains(&open.name) {
                self.mends.push(Mend {
                    change: Change::Closed,
                    name: open.name,
                    line: open.line,
                });
            }
        }
    }
}
