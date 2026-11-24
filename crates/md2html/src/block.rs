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
//! holds and leaves the next. The lines are read from the source as they
//! are asked for, through the containers open around them ([`lines`]):
//! what the parser holds grows with the blocks it has made, not with the
//! lines it has read.

mod container;
mod define;
mod html;
mod leaf;
mod line;
mod lines;
mod opening;
mod table;
#[cfg(test)]
mod tests;

use self::html::html_block;
use self::leaf::{indented_code, math_block};
use self::line::Line;
use self::lines::Lines;
use self::opening::Opening;
use self::table::TableHead;
use crate::model::{Block, Copies, Defs, Paragraph};

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
    let mut lines = Lines::new(text, first_line, math);
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
    .blocks(&mut lines)
    .blocks;
    (blocks, defs)
}

/// What a run of lines holds.
struct Read {
    /// Its blocks.
    blocks: Vec<Block>,
    /// A blank line stands right before one of the blocks it read after
    /// the first, those that show nothing (link and footnote definitions)
    /// too: in a list item, it makes the list loose.
    spaced: bool,
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
    fn blocks(&mut self, rest: &mut Lines<'_>) -> Read {
        let mut read = Read {
            blocks: Vec::new(),
            spaced: false,
        };
        let mut started = false;
        loop {
            let after_blank = rest.after_blank();
            let Some(line) = rest.bump() else {
                break;
            };
            if !line.is_blank() {
                read.spaced |= started && after_blank;
                started = true;
            }
            if let Some(block) = self.block(&line, rest) {
                read.blocks.push(block);
            }
        }
        read.blocks.shrink_to_fit();
        read
    }

    /// The block `line` opens, with the lines of `rest` it holds. A blank
    /// line, a footnote definition and a paragraph of nothing but link
    /// definitions give none.
    fn block<'src>(&mut self, line: &Line<'src>, rest: &mut Lines<'src>) -> Option<Block> {
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
    fn nested(&mut self, lines: &mut Lines<'_>) -> Read {
        if self.depth >= MOST_DEPTH {
            return Read {
                blocks: vec![Block::Paragraph(as_text(lines))],
                spaced: false,
            };
        }
        self.depth += 1;
        let blocks = self.blocks(lines);
        self.depth -= 1;
        blocks
    }
}

/// Hostile nesting: the rest of the lines, read as the text of one
/// paragraph.
fn as_text(lines: &mut Lines<'_>) -> Paragraph {
    let mut text = String::new();
    let mut first = None;
    while let Some(line) = lines.bump() {
        if first.is_some() {
            text.push('\n');
        } else {
            first = Some(line.no);
        }
        text.push_str(&line.text);
    }
    Paragraph {
        text,
        line: first.unwrap_or_default(),
    }
}
