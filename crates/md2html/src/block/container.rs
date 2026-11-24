// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Containers: block quotes, list items and footnotes. Each reads the
//! lines it holds, its own marks taken off, one level deeper, as they
//! come ([`super::lines`]).

use super::Parser;
use super::line::{Line, is_rule};
use super::lines::{Container, Lines};
use super::opening::{Marker, Note};
use crate::inline::normalize_label;
use crate::model::{Block, Item, List, Quote, Spacing, Task};

/// One list item, read.
struct ItemRead {
    /// The item.
    item: Item,
    /// A blank line stands between two of its own blocks.
    loose: bool,
    /// Blank lines closed it.
    blank_after: bool,
}

/// What a list item's marker line opens.
struct ItemStart<'src> {
    /// Its first line: the text after the marker, in the column it starts
    /// at.
    first: Line<'src>,
    /// The column its content starts at.
    col: usize,
    /// Nothing follows the marker.
    empty: bool,
}

impl<'src> ItemStart<'src> {
    /// The item `marker` opens on `head`.
    fn of(head: &Line<'src>, marker: Marker<'_>) -> Self {
        Self {
            first: marker.first_line(head),
            col: marker.col,
            empty: marker.is_empty(),
        }
    }
}

impl Parser<'_> {
    /// The block quote `head` opens, whose text after the `>` is
    /// `content`, with the lines of `rest` that continue it.
    pub(super) fn quote<'src>(
        &mut self,
        head: &Line<'src>,
        content: &str,
        rest: &mut Lines<'src>,
    ) -> Block {
        let inside = rest.inside(head.with(content), Container::Quote, |lines| {
            self.nested(lines)
        });
        Block::Quote(Quote {
            blocks: inside.read.blocks,
            line: head.no,
        })
    }

    /// The footnote definition `note` opens on `head`, with the lines of
    /// `rest` that continue it, into the definitions. The first
    /// definition of a label wins.
    pub(super) fn footnote<'src>(
        &mut self,
        head: &Line<'src>,
        note: Note<'_>,
        rest: &mut Lines<'src>,
    ) {
        let first = head.with(note.text.trim_start());
        let blocks = rest
            .inside(first, Container::Note, |lines| self.nested(lines))
            .read
            .blocks;
        self.defs
            .notes
            .entry(normalize_label(note.label))
            .or_insert(blocks);
    }

    /// The list whose first item `first` opens on `head`, with the items
    /// of `rest` that continue it.
    pub(super) fn list<'src>(
        &mut self,
        head: &Line<'src>,
        first: Marker<'_>,
        rest: &mut Lines<'src>,
    ) -> Block {
        let mut items = Vec::new();
        let mut loose = false;
        let mut start = ItemStart::of(head, first);
        loop {
            let read = self.item(start, rest);
            loose |= read.loose;
            items.push(read.item);
            let Some(next) = next_item(rest, first.mark) else {
                break;
            };
            loose |= read.blank_after;
            rest.bump();
            start = next;
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

    /// The list item `start` opens, with the lines of `rest` it holds:
    /// those indented to its content, blank lines, lazy lines.
    fn item<'src>(&mut self, start: ItemStart<'src>, rest: &mut Lines<'src>) -> ItemRead {
        // An item that opens empty and is followed by a blank line ends
        // there.
        let alone = start.empty && rest.peek().is_none_or(Line::is_blank);
        let holds = if alone {
            Container::Nothing
        } else {
            Container::Item { col: start.col }
        };
        let inside = rest.inside(start.first, holds, |lines| self.nested(lines));
        let mut trailing = inside.trailing;
        if alone {
            trailing += rest.skip_blank();
        }
        let mut blocks = inside.read.blocks;
        let task = take_task(&mut blocks);
        ItemRead {
            item: Item { task, blocks },
            // Loose when a blank line stands between two of the item's
            // own blocks, a definition among them; a blank inside a nested
            // list or a code block is not one.
            loose: inside.read.spaced,
            blank_after: trailing > 0,
        }
    }
}

/// The item the next line of `rest` opens, when it opens another item of
/// a list whose items are marked `mark`.
fn next_item<'src>(rest: &mut Lines<'src>, mark: char) -> Option<ItemStart<'src>> {
    let line = rest.peek()?;
    let marker = Marker::read(&line.text, line.col)?;
    (marker.mark == mark && !is_rule(&line.text)).then(|| ItemStart::of(line, marker))
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
