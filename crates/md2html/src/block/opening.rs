// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a line opens. One reading of a line's start serves the dispatch,
//! a paragraph's interruptions and a container's lazy lines alike.

use super::html::HtmlKind;
use super::line::{Indented, indented, is_blank, is_rule, strip_cols};
use crate::model::{Heading, HeadingStyle, ListKind};
use crate::scan::Scan;

/// What a line opens, in the order the kinds are tried.
#[derive(Debug)]
pub(super) enum Opening<'line> {
    /// Nothing but spaces.
    Blank,
    /// Four or more spaces of indentation: a line of indented code.
    Code,
    /// A code fence.
    Fence(FenceOpen<'line>),
    /// `$$` when maths is on, with the text after it.
    Math(&'line str),
    /// `#` to `######`.
    Heading(Atx<'line>),
    /// `---`, `***` or `___`.
    Rule,
    /// Raw HTML of one of `CommonMark`'s seven kinds.
    Html(HtmlKind),
    /// `>`, with the text after the mark.
    Quote(&'line str),
    /// A list marker.
    Item(Marker<'line>),
    /// `[^label]:`, a footnote definition.
    Note(Note<'line>),
    /// Anything else, its indentation taken off.
    Text(&'line str),
}

impl<'line> Opening<'line> {
    /// What `line` opens; `$$` is maths only when `math` is on.
    pub(super) fn of(line: &'line str, math: bool) -> Self {
        let Indented { width, body } = indented(line);
        if is_blank(line) {
            return Self::Blank;
        }
        if width > 3 {
            return Self::Code;
        }
        if let Some(open) = FenceOpen::read(body, width) {
            return Self::Fence(open);
        }
        if let Some(after) = body.strip_prefix("$$").filter(|_| math) {
            return Self::Math(after);
        }
        if let Some(atx) = Atx::read(body) {
            return Self::Heading(atx);
        }
        if is_rule(body) {
            return Self::Rule;
        }
        if let Some(kind) = HtmlKind::opening(body) {
            return Self::Html(kind);
        }
        if let Some(content) = quoted(line) {
            return Self::Quote(content);
        }
        if let Some(marker) = Marker::read(line) {
            return Self::Item(marker);
        }
        Note::read(body).map_or(Self::Text(body), Self::Note)
    }

    /// Whether the line, met while a paragraph is open, ends it by
    /// starting a block of its own.
    pub(super) const fn interrupts(&self) -> bool {
        match self {
            Self::Fence(_) | Self::Math(_) | Self::Heading(_) | Self::Rule | Self::Quote(_) => true,
            Self::Html(kind) => kind.interrupts(),
            Self::Item(marker) => marker.interrupts(),
            Self::Blank | Self::Code | Self::Note(_) | Self::Text(_) => false,
        }
    }

    /// Whether the line, standing last in a container, leaves a paragraph
    /// open that a lazy line may continue.
    pub(super) const fn continues_paragraph(&self) -> bool {
        !matches!(
            self,
            Self::Blank
                | Self::Code
                | Self::Fence(_)
                | Self::Heading(_)
                | Self::Rule
                | Self::Html(_)
        )
    }

    /// Whether the line may continue a container's open paragraph without
    /// the container's mark: text that neither starts a block nor a list.
    pub(super) const fn is_lazy(&self) -> bool {
        !matches!(self, Self::Blank | Self::Item(_)) && !self.interrupts()
    }
}

/// The text of a `>` line after its mark and one space; `None` for any
/// other line.
pub(super) fn quoted(line: &str) -> Option<&str> {
    let Indented { width, body } = indented(line);
    if width > 3 {
        return None;
    }
    body.strip_prefix('>').map(|content| strip_cols(content, 1))
}

/// What closes a code fence: its mark and how many of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fence {
    /// `` ` `` or `~`.
    mark: char,
    /// Three or more.
    len: usize,
}

impl Fence {
    /// Whether `line` closes the fence.
    pub(super) fn closed_by(self, line: &str) -> bool {
        let Indented { width, body } = indented(line);
        let after = body.trim_start_matches(self.mark);
        width <= 3 && body.len() - after.len() >= self.len && is_blank(after)
    }

    /// The fence a container is inside after `line`: opened, closed or
    /// unchanged.
    pub(super) fn track(open: Option<Self>, line: &str) -> Option<Self> {
        if let Some(fence) = open {
            return (!fence.closed_by(line)).then_some(fence);
        }
        let Indented { width, body } = indented(line);
        if width > 3 {
            return None;
        }
        FenceOpen::read(body, width).map(|opening| opening.fence)
    }
}

/// A line that opens a code fence.
#[derive(Clone, Copy, Debug)]
pub(super) struct FenceOpen<'line> {
    /// What closes it.
    pub(super) fence: Fence,
    /// The fence's indentation, taken off the code's lines too.
    pub(super) indent: usize,
    /// The info string, as written.
    pub(super) info: &'line str,
}

impl<'line> FenceOpen<'line> {
    /// The fence `body`, indented by `indent`, opens.
    fn read(body: &'line str, indent: usize) -> Option<Self> {
        let mark = body.chars().next().filter(|ch| matches!(ch, '`' | '~'))?;
        let after = body.trim_start_matches(mark);
        let len = body.len() - after.len();
        let info = after.trim();
        if len < 3 || (mark == '`' && info.contains('`')) {
            return None;
        }
        Some(Self {
            fence: Fence { mark, len },
            indent,
            info,
        })
    }
}

/// An ATX heading's line.
#[derive(Clone, Copy, Debug)]
pub(super) struct Atx<'line> {
    /// One to six.
    level: u8,
    /// The text, closing hashes taken off.
    text: &'line str,
}

impl<'line> Atx<'line> {
    /// The heading `body` opens.
    fn read(body: &'line str) -> Option<Self> {
        let after = body.trim_start_matches('#');
        let level = u8::try_from(body.len() - after.len())
            .ok()
            .filter(|level| (1..=6).contains(level))?;
        if !(after.is_empty() || after.starts_with([' ', '\t'])) {
            return None;
        }
        let text = after.trim();
        // The closing hashes, when a space stands before them.
        let bare = text.trim_end_matches('#');
        let closed = bare.len() < text.len() && (bare.is_empty() || bare.ends_with([' ', '\t']));
        Some(Self {
            level,
            text: if closed { bare.trim_end() } else { text },
        })
    }

    /// The heading, on source line `line`.
    pub(super) fn heading(self, line: usize) -> Heading {
        let title = Title::read(self.text);
        Heading {
            level: self.level,
            text: title.text,
            id: title.id,
            style: HeadingStyle::Atx,
            line,
        }
    }
}

/// A heading's text and the `{#id}` written after it.
pub(super) struct Title {
    /// The text.
    pub(super) text: String,
    /// The explicit id.
    pub(super) id: Option<String>,
}

impl Title {
    /// `text` with a trailing `{#id}`, as pandoc and kramdown write it,
    /// taken apart.
    pub(super) fn read(text: &str) -> Self {
        text.strip_suffix('}')
            .and_then(|head| head.rsplit_once("{#"))
            .filter(|(before, id)| {
                !id.is_empty()
                    && id
                        .chars()
                        .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
                    && (before.is_empty() || before.ends_with(' '))
            })
            .map_or_else(
                || Self {
                    text: text.to_owned(),
                    id: None,
                },
                |(before, id)| Self {
                    text: before.trim_end().to_owned(),
                    id: Some(id.to_owned()),
                },
            )
    }
}

/// A list marker at the start of a line.
#[derive(Clone, Copy, Debug)]
pub(super) struct Marker<'line> {
    /// `-`, `+` or `*`, or the `.` or `)` after a number: items with the
    /// same mark make one list.
    pub(super) mark: char,
    /// Bulleted, or numbered from this item's number.
    pub(super) kind: ListKind,
    /// The column the item's content starts at.
    pub(super) col: usize,
    /// The item's text on the marker's line; empty when nothing follows
    /// the marker.
    pub(super) content: &'line str,
}

impl<'line> Marker<'line> {
    /// The list marker `line` opens with, if it opens with one.
    pub(super) fn read(line: &'line str) -> Option<Self> {
        let Indented { width, body } = indented(line);
        if width > 3 {
            return None;
        }
        let mut scan = Scan::new(body);
        let (mark, kind) = if scan.peek()?.is_ascii_digit() {
            let digits = scan.take_while(|ch| ch.is_ascii_digit());
            if digits.len() > 9 {
                return None;
            }
            let delim = scan.eat_if(|ch| matches!(ch, '.' | ')'))?;
            (delim, ListKind::Ordered(digits.parse().ok()?))
        } else {
            let bullet = scan.eat_if(|ch| matches!(ch, '-' | '+' | '*'))?;
            (bullet, ListKind::Bullet)
        };
        let rest = scan.rest();
        if !(rest.is_empty() || rest.starts_with(' ')) {
            return None;
        }
        let after_mark = width + scan.seen().len();
        let text = rest.trim_start_matches(' ');
        let spaces = rest.len() - text.len();
        Some(if is_blank(rest) {
            Self {
                mark,
                kind,
                col: after_mark + 1,
                content: "",
            }
        } else if spaces >= 5 {
            // Indented code inside the item: one space belongs to the marker.
            Self {
                mark,
                kind,
                col: after_mark + 1,
                content: strip_cols(rest, 1),
            }
        } else {
            Self {
                mark,
                kind,
                col: after_mark + spaces,
                content: text,
            }
        })
    }

    /// Whether nothing follows the marker on its line.
    pub(super) const fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Whether the marker is a numbered one.
    pub(super) const fn is_ordered(&self) -> bool {
        matches!(self.kind, ListKind::Ordered(_))
    }

    /// Whether the item may interrupt a paragraph: it has text, and a
    /// numbered one starts at one.
    const fn interrupts(&self) -> bool {
        !self.is_empty()
            && match self.kind {
                ListKind::Bullet => true,
                ListKind::Ordered(start) => start == 1,
            }
    }
}

/// The opening line of a footnote definition.
#[derive(Clone, Copy, Debug)]
pub(super) struct Note<'line> {
    /// The label between `[^` and `]:`.
    pub(super) label: &'line str,
    /// What follows `]:`.
    pub(super) text: &'line str,
}

impl<'line> Note<'line> {
    /// `[^label]:` opening `body`. A label longer than a link label may
    /// be (`crate::inline::fits_label`) opens nothing, as for links: no
    /// reference could reach it.
    pub(super) fn read(body: &'line str) -> Option<Self> {
        let (label, text) = body.strip_prefix("[^")?.split_once("]:")?;
        (!label.is_empty()
            && !label.contains(char::is_whitespace)
            && !label.contains(['[', ']'])
            && crate::inline::fits_label(label))
        .then_some(Self { label, text })
    }
}
