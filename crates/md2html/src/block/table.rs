// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Pipe tables: a header row, a delimiter row with as many cells, then
//! rows up to a blank line or a line that starts a block.

use super::Parser;
use super::line::{Line, indent};
use crate::model::{Align, Block, Table};
use crate::scan::Cursor;

/// A table's first two rows: the header cells and the alignments.
pub(super) struct TableHead {
    /// One per column.
    align: Vec<Align>,
    /// The header cells, raw.
    head: Vec<String>,
}

impl TableHead {
    /// The table `head` and the line `under` it open, if they open one: a
    /// header row with pipes, then a delimiter row with as many cells.
    fn read(head: &str, under: &str) -> Option<Self> {
        if !head.contains('|') || indent(head) > 3 {
            return None;
        }
        let align = delimiter_row(under)?;
        let names = cells(head);
        (names.len() == align.len()).then_some(Self { align, head: names })
    }

    /// The table `head` opens with the next line of `rest`, that line
    /// read.
    pub(super) fn take(head: &Line<'_>, rest: &mut Cursor<'_, Line<'_>>) -> Option<Self> {
        let under = rest.peek().filter(|under| !under.lazy)?;
        let table = Self::read(&head.text, &under.text)?;
        rest.bump();
        Some(table)
    }

    /// Whether `head` and the line `under` it, if any, open a table.
    ///
    /// A lazy line is paragraph text, never a delimiter row.
    pub(super) fn opens(head: &Line<'_>, under: Option<&Line<'_>>) -> bool {
        under.is_some_and(|row| !row.lazy && Self::read(&head.text, &row.text).is_some())
    }
}

impl Parser<'_> {
    /// The table under `head`, on source line `line`, with the body rows
    /// of `rest`.
    pub(super) fn table(
        &self,
        line: usize,
        head: TableHead,
        rest: &mut Cursor<'_, Line<'_>>,
    ) -> Block {
        let width = head.head.len();
        let rows = rest
            .take_while(|row| !row.is_blank() && !self.interrupts(row))
            .iter()
            .map(|row| {
                let mut row_cells = cells(&row.text);
                row_cells.resize(width, String::new());
                row_cells
            })
            .collect();
        Block::Table(Box::new(Table {
            align: head.align,
            head: head.head,
            rows,
            line,
        }))
    }
}

/// A table's delimiter row: its alignments.
fn delimiter_row(line: &str) -> Option<Vec<Align>> {
    let row = line.trim();
    if indent(line) > 3 {
        return None;
    }
    let opened = row.strip_prefix('|').unwrap_or(row);
    let inner = opened.strip_suffix('|').unwrap_or(opened);
    let align = inner
        .split('|')
        .map(|cell| alignment(cell.trim()))
        .collect::<Option<Vec<_>>>()?;
    // Two cells or more are split at a pipe: the row holds one either way.
    row.contains('|').then_some(align)
}

/// One cell of a delimiter row, trimmed: dashes with an optional colon at
/// either end.
fn alignment(cell: &str) -> Option<Align> {
    // A lone `:` has no dashes and is refused below, so it is not both ends.
    let left = cell.starts_with(':');
    let right = cell.ends_with(':');
    let dashes = cell.trim_start_matches(':').trim_end_matches(':');
    if dashes.is_empty() || !dashes.chars().all(|ch| ch == '-') {
        return None;
    }
    Some(match (left, right) {
        (true, true) => Align::Center,
        (true, false) => Align::Left,
        (false, true) => Align::Right,
        (false, false) => Align::None,
    })
}

/// One table row split on unescaped pipes. A `\|` stays in the cell as
/// a pipe, which is how a code span inside a cell writes one.
fn cells(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let opened = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = if opened.ends_with("\\|") {
        opened
    } else {
        opened.strip_suffix('|').unwrap_or(opened)
    };
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some('|') => cell.push('|'),
                Some(escaped) => {
                    cell.push('\\');
                    cell.push(escaped);
                }
                None => cell.push('\\'),
            },
            '|' => {
                out.push(cell.trim().to_owned());
                cell.clear();
            }
            other => cell.push(other),
        }
    }
    out.push(cell.trim().to_owned());
    out
}
