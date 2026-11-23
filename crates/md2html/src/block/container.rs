// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Containers: block quotes, list items and footnotes. Each gathers the
//! lines it holds, its own marks taken off, and parses them again one
//! level deeper.

use super::line::{Line, indent, is_rule, strip_cols};
use super::opening::{Fence, Marker, Note, Opening, quoted};
use super::{Parser, normalize_label};
use crate::model::{Block, Item, List, Quote, Task};
use crate::scan::Cursor;

/// The lines a quote or a list item holds, gathered to be parsed on
/// their own.
struct Inner {
    /// The lines, the container's marks taken off.
    lines: Vec<Line>,
    /// The code fence the lines are inside, if any: no lazy line there.
    fence: Option<Fence>,
}

impl Inner {
    /// The lines of a container whose first line `head` holds `content`.
    fn new(head: &Line, content: &str) -> Self {
        Self {
            lines: vec![head.with(content)],
            fence: Fence::track(None, content),
        }
    }

    /// Adds `text`, the container's part of `line`.
    fn push(&mut self, line: &Line, text: &str) {
        self.fence = Fence::track(self.fence, text);
        self.lines.push(line.with(text));
    }

    /// Whether `line`, without the container's mark, still belongs to it:
    /// the lazy continuation of the paragraph its last line leaves open.
    fn takes_lazy(&self, line: &Line, math: bool) -> bool {
        self.fence.is_none()
            && self
                .lines
                .last()
                .is_some_and(|last| Opening::of(&last.text, math).continues_paragraph())
            && Opening::of(&line.text, math).is_lazy()
    }

    /// Adds a lazy line.
    fn push_lazy(&mut self, line: &Line) {
        self.lines.push(line.with(line.text.trim_start()));
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
        head: &Line,
        content: &str,
        rest: &mut Cursor<'_, Line>,
    ) -> Block {
        let mut inner = Inner::new(head, content);
        while let Some(line) = rest.peek() {
            if let Some(text) = quoted(&line.text) {
                inner.push(line, text);
            } else if inner.takes_lazy(line, self.math) {
                inner.push_lazy(line);
            } else {
                break;
            }
            rest.bump();
        }
        Block::Quote(Quote {
            blocks: self.nested(&inner.lines),
            line: head.no,
        })
    }

    /// The footnote definition `note` opens on `head`, with the lines of
    /// `rest` that continue it, into the definitions. The first
    /// definition of a label wins.
    pub(super) fn footnote(&mut self, head: &Line, note: Note<'_>, rest: &mut Cursor<'_, Line>) {
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
                && !self.interrupts(&line.text)
                && Note::read(line.text.trim_start()).is_none()
            {
                inner.push(line.with(line.text.trim_start()));
            } else {
                break;
            }
            rest.bump();
        }
        let blocks = self.nested(&inner);
        self.defs
            .notes
            .entry(normalize_label(note.label))
            .or_insert(blocks);
    }

    /// The list whose first item `first` opens on `head`, with the items
    /// of `rest` that continue it.
    pub(super) fn list(
        &mut self,
        head: &Line,
        first: Marker<'_>,
        rest: &mut Cursor<'_, Line>,
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
        Block::List(List {
            kind: first.kind,
            tight: !loose,
            items,
        })
    }

    /// The list item `marker` opens on `head`, with the lines of `rest` it
    /// holds: those indented to its content, blank lines, lazy lines.
    fn item(&mut self, head: &Line, marker: Marker<'_>, rest: &mut Cursor<'_, Line>) -> ItemRead {
        let mut inner = Inner::new(head, marker.content);
        let mut last_blank = marker.is_empty();
        let mut blank_inside = false;
        // An item that opens empty and is followed by a blank line ends
        // there.
        let starts_empty = marker.is_empty() && rest.peek().is_none_or(Line::is_blank);
        while let Some(line) = rest.peek().filter(|_| !starts_empty) {
            if line.is_blank() {
                inner.lines.push(line.with(""));
                last_blank = true;
            } else if indent(&line.text) >= marker.col {
                blank_inside |= last_blank
                    && inner.fence.is_none()
                    && inner.lines.iter().any(|kept| !kept.is_blank());
                inner.push(line, strip_cols(&line.text, marker.col));
                last_blank = false;
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
        let mut blocks = self.nested(&inner.lines);
        // Loose when a blank line stands between two of the item's own
        // blocks; a blank inside a nested list or a code block is not one.
        let loose = blocks
            .iter()
            .skip(1)
            .any(|block| first_line(block).map_or(blank_inside, |start| inner.blank_before(start)));
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
fn next_item<'lines>(
    rest: &Cursor<'lines, Line>,
    mark: char,
) -> Option<(&'lines Line, Marker<'lines>)> {
    let line = rest.peek()?;
    let marker = Marker::read(&line.text)?;
    (marker.mark == mark && !is_rule(&line.text)).then_some((line, marker))
}

/// The line a block starts on, looking into a list for its first item.
fn first_line(block: &Block) -> Option<usize> {
    if let Block::List(list) = block {
        return list
            .items
            .first()
            .and_then(|item| item.blocks.first())
            .and_then(first_line);
    }
    block.line()
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
