// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Containers: block quotes, list items and footnotes. Each gathers the
//! lines it holds, its own marks taken off, and parses them again one
//! level deeper.

use super::Parser;
use super::line::{Line, indent, is_rule, strip_cols};
use super::opening::{Fence, Marker, Note, Opening, quoted};
use crate::inline::normalize_label;
use crate::model::{Block, Item, List, Quote, Spacing, Task};
use crate::scan::Cursor;

/// The lines a quote or a list item holds, gathered to be parsed on
/// their own.
struct Inner<'src> {
    /// The lines, the container's marks taken off.
    lines: Vec<Line<'src>>,
    /// The code fence the lines are inside, if any: no lazy line there.
    fence: Option<Fence>,
}

impl<'src> Inner<'src> {
    /// The lines of a container whose first line, its marks taken off, is
    /// `first`.
    fn new(first: Line<'src>) -> Self {
        let fence = Fence::track(None, &first.text);
        Self {
            lines: vec![first],
            fence,
        }
    }

    /// Adds `text`, the container's part of `line`.
    fn push(&mut self, line: &Line<'src>, text: &str) {
        self.fence = Fence::track(self.fence, text);
        self.lines.push(line.with(text));
    }

    /// Whether `line`, without the container's mark, still belongs to it:
    /// the lazy continuation of the paragraph its last line leaves open.
    ///
    /// A line lazy in an outer container is lazy here too.
    fn takes_lazy(&self, line: &Line<'_>, math: bool) -> bool {
        self.fence.is_none()
            && self
                .lines
                .last()
                .is_some_and(|last| last.lazy || Opening::of(last, math).continues_paragraph())
            && (line.lazy || Opening::of(line, math).is_lazy())
    }

    /// Adds a lazy line.
    fn push_lazy(&mut self, line: &Line<'src>) {
        self.lines.push(line.to_lazy());
    }

    /// Whether the line numbered `no` comes straight after a blank line.
    fn blank_before(&self, no: usize) -> bool {
        self.lines
            .windows(2)
            .any(|pair| matches!(pair, [before, line] if line.no == no && before.is_blank()))
    }
}

/// One list item, read.
struct ItemRead {
    /// The item.
    item: Item,
    /// A blank line stands between two of its own blocks.
    loose: bool,
    /// Blank lines closed it.
    blank_after: bool,
}

impl Parser<'_> {
    /// The block quote `head` opens, whose text after the `>` is
    /// `content`, with the lines of `rest` that continue it.
    pub(super) fn quote(
        &mut self,
        head: &Line<'_>,
        content: &str,
        rest: &mut Cursor<'_, Line<'_>>,
    ) -> Block {
        let mut inner = Inner::new(head.with(content));
        while let Some(line) = rest.peek() {
            if let Some(text) = quoted(&line.text).filter(|_| !line.lazy) {
                inner.push(line, text);
            } else if inner.takes_lazy(line, self.math) {
                inner.push_lazy(line);
            } else {
                break;
            }
            rest.bump();
        }
        Block::Quote(Quote {
            blocks: self.nested(&inner.lines).blocks,
            line: head.no,
        })
    }

    /// The footnote definition `note` opens on `head`, with the lines of
    /// `rest` that continue it, into the definitions. The first
    /// definition of a label wins.
    pub(super) fn footnote(
        &mut self,
        head: &Line<'_>,
        note: Note<'_>,
        rest: &mut Cursor<'_, Line<'_>>,
    ) {
        let mut inner = vec![head.with(note.text.trim_start())];
        let mut last_blank = false;
        while let Some(line) = rest.peek() {
            if line.is_blank() {
                inner.push(line.with(""));
                last_blank = true;
            } else if indent(&line.text) >= 4 {
                inner.push(line.with(strip_cols(&line.text, 4)));
                last_blank = false;
            } else if !last_blank
                && !self.interrupts(line)
                && Note::read(line.text.trim_start()).is_none()
            {
                inner.push(line.with(line.text.trim_start()));
            } else {
                break;
            }
            rest.bump();
        }
        let blocks = self.nested(&inner).blocks;
        self.defs
            .notes
            .entry(normalize_label(note.label))
            .or_insert(blocks);
    }

    /// The list whose first item `first` opens on `head`, with the items
    /// of `rest` that continue it.
    pub(super) fn list(
        &mut self,
        head: &Line<'_>,
        first: Marker<'_>,
        rest: &mut Cursor<'_, Line<'_>>,
    ) -> Block {
        let mut items = Vec::new();
        let mut loose = false;
        let mut item_line = head;
        let mut marker = first;
        loop {
            let read = self.item(item_line, marker, rest);
            loose |= read.loose;
            items.push(read.item);
            let Some((next_line, next_marker)) = next_item(rest, first.mark) else {
                break;
            };
            loose |= read.blank_after;
            rest.bump();
            item_line = next_line;
            marker = next_marker;
        }
        // The room left over in a list's items, and in a container's
        // blocks, is given back: a list nested in every line keeps no
        // empty slots by the hundred.
        items.shrink_to_fit();
        Block::List(Box::new(List {
            kind: first.kind,
            spacing: if loose {
                Spacing::Loose
            } else {
                Spacing::Tight
            },
            items,
        }))
    }

    /// The list item `marker` opens on `head`, with the lines of `rest` it
    /// holds: those indented to its content, blank lines, lazy lines.
    fn item(
        &mut self,
        head: &Line<'_>,
        marker: Marker<'_>,
        rest: &mut Cursor<'_, Line<'_>>,
    ) -> ItemRead {
        let mut inner = Inner::new(marker.first_line(head));
        // An item that opens empty and is followed by a blank line ends
        // there.
        let starts_empty = marker.is_empty() && rest.peek().is_none_or(Line::is_blank);
        while let Some(line) = rest.peek().filter(|_| !starts_empty) {
            if line.is_blank() {
                inner.lines.push(line.with(""));
            } else if !line.lazy && indent(&line.text) >= marker.col {
                inner.push(line, strip_cols(&line.text, marker.col));
            } else if inner.takes_lazy(line, self.math) {
                inner.push_lazy(line);
            } else {
                break;
            }
            rest.bump();
        }
        let mut trailing = 0_usize;
        while inner.lines.len() > 1 && inner.lines.last().is_some_and(Line::is_blank) {
            inner.lines.pop();
            trailing += 1;
        }
        if starts_empty {
            trailing += rest.take_while(Line::is_blank).len();
        }
        let read = self.nested(&inner.lines);
        // Loose when a blank line stands between two of the item's own
        // blocks, a definition among them; a blank inside a nested list or
        // a code block is not one.
        let loose = read
            .starts
            .iter()
            .skip(1)
            .any(|start| inner.blank_before(*start));
        let mut blocks = read.blocks;
        let task = take_task(&mut blocks);
        ItemRead {
            item: Item { task, blocks },
            loose,
            blank_after: trailing > 0,
        }
    }
}

/// The next line of `rest` and its marker, when it opens another item of
/// a list whose items are marked `mark`.
fn next_item<'lines, 'src>(
    rest: &Cursor<'lines, Line<'src>>,
    mark: char,
) -> Option<(&'lines Line<'src>, Marker<'lines>)> {
    let line = rest.peek()?;
    let marker = Marker::read(&line.text, line.col)?;
    (marker.mark == mark && !is_rule(&line.text)).then_some((line, marker))
}

/// A task item's box, `[ ]` or `[x]`, taken off the item's first paragraph.
fn take_task(blocks: &mut [Block]) -> Option<Task> {
    let Some(Block::Paragraph(paragraph)) = blocks.first_mut() else {
        return None;
    };
    let (task, rest) = [
        ("[ ]", Task::Open),
        ("[x]", Task::Done),
        ("[X]", Task::Done),
    ]
    .into_iter()
    .find_map(|(mark, task)| Some((task, paragraph.text.strip_prefix(mark)?)))?;
    if !rest.starts_with([' ', '\n']) {
        return None;
    }
    paragraph.text = rest.trim_start().to_owned();
    Some(task)
}
