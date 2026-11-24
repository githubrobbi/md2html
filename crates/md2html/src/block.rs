// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The block structure of a document, the way `CommonMark` reads it.
//!
//! Containers (block quotes, list items, footnotes) hold lines that are
//! parsed again on their own, so a list nests, a quote holds a list and
//! a list item holds code. Leaves keep their inline text raw; the inline
//! pass ([`crate::inline`]) runs later, once every link reference and
//! footnote is known. GitHub's additions come along: tables, task items,
//! footnotes; and `$$` maths blocks.
//!
//! Each line is read once for what it opens ([`opening`]); the parser
//! hands it to the reader of that kind, which takes the lines the block
//! holds from a [`Cursor`] and leaves the next.

mod container;
mod define;
mod html;
mod leaf;
mod line;
mod opening;
mod table;
#[cfg(test)]
mod tests;

use self::html::html_block;
use self::leaf::{indented_code, math_block};
use self::line::Line;
use self::opening::Opening;
use self::table::TableHead;
use crate::model::{Block, Copies, Defs, Paragraph};
use crate::scan::Cursor;

/// How deep containers may nest before the rest is read as text, so
/// hostile nesting cannot overflow the stack.
const MOST_DEPTH: usize = 64;

/// How a text is read where `CommonMark` and what authors mean part ways.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// As `CommonMark` reads it: `---` under a paragraph always makes it a
    /// heading, and a numbered line always continues a paragraph.
    CommonMark,
    /// As authors mean it far more often: only a single line that does not
    /// end like a sentence becomes a heading over `---` (a paragraph with a
    /// rule straight under it keeps its rule), and a numbered list may
    /// follow the line that announces it ("Phase B, with the doctor:").
    Author,
}

/// The blocks of `text` and what it defines. `first_line` is the number
/// of its first line in the file (front matter shifts it).
pub(crate) fn parse(
    text: &str,
    first_line: usize,
    math: bool,
    reading: Reading,
) -> (Vec<Block>, Defs) {
    let lines = Line::split(text, first_line);
    let mut defs = Defs {
        copies: Copies::of(text.len()),
        ..Defs::default()
    };
    let blocks = Parser {
        defs: &mut defs,
        math,
        depth: 0,
        reading,
    }
    .blocks(&lines)
    .blocks;
    (blocks, defs)
}

/// What a run of lines holds.
struct Read {
    /// Its blocks.
    blocks: Vec<Block>,
    /// The line each block it read starts on, those that show nothing
    /// (link and footnote definitions) too.
    starts: Vec<usize>,
}

/// The block parser over one container's lines.
struct Parser<'defs> {
    /// Where definitions go.
    defs: &'defs mut Defs,
    /// `$$` blocks are maths.
    math: bool,
    /// Container depth, bounded by [`MOST_DEPTH`].
    depth: usize,
    /// How the text is read where `CommonMark` and authors part ways.
    reading: Reading,
}

impl Parser<'_> {
    /// The blocks of a run of lines.
    fn blocks(&mut self, lines: &[Line<'_>]) -> Read {
        let mut rest = Cursor::new(lines);
        let mut read = Read {
            blocks: Vec::new(),
            starts: Vec::new(),
        };
        while let Some(line) = rest.bump() {
            if !line.is_blank() {
                read.starts.push(line.no);
            }
            if let Some(block) = self.block(line, &mut rest) {
                read.blocks.push(block);
            }
        }
        read.blocks.shrink_to_fit();
        read
    }

    /// The block `line` opens, with the lines of `rest` it holds. A blank
    /// line, a footnote definition and a paragraph of nothing but link
    /// definitions give none.
    fn block(&mut self, line: &Line<'_>, rest: &mut Cursor<'_, Line<'_>>) -> Option<Block> {
        Some(match Opening::of(line, self.math) {
            Opening::Blank => return None,
            Opening::Code => indented_code(line, rest),
            Opening::Fence(open) => self.fenced(line, open, rest),
            Opening::Math(after) => math_block(line, after, rest),
            Opening::Heading(atx) => Block::Heading(Box::new(atx.heading(line.no))),
            Opening::Rule => Block::Rule,
            Opening::Html(kind) => html_block(line, kind, rest),
            Opening::Quote(content) => self.quote(line, content, rest),
            Opening::Item(marker) => self.list(line, marker, rest),
            Opening::Note(note) => {
                self.footnote(line, note, rest);
                return None;
            }
            Opening::Text(body) => match TableHead::take(line, rest) {
                Some(head) => self.table(line.no, head, rest),
                None => return self.paragraph(line, body, rest),
            },
        })
    }

    /// Whether `line`, met while a paragraph is open, ends it by starting
    /// a block of its own.
    fn interrupts(&self, line: &Line<'_>) -> bool {
        Opening::of(line, self.math).interrupts()
    }

    /// The blocks of a container's lines, one level deeper.
    fn nested(&mut self, lines: &[Line<'_>]) -> Read {
        if self.depth >= MOST_DEPTH {
            // Hostile nesting: the rest is text.
            let text = lines
                .iter()
                .map(|line| line.text.as_ref())
                .collect::<Vec<_>>()
                .join("\n");
            let line = lines.first().map_or(0, |line| line.no);
            return Read {
                blocks: vec![Block::Paragraph(Paragraph { text, line })],
                starts: vec![line],
            };
        }
        self.depth += 1;
        let blocks = self.blocks(lines);
        self.depth -= 1;
        blocks
    }
}
