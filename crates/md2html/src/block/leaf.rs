// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Leaf blocks: code, maths, and paragraphs with the setext headings that
//! underline them. Each reads the lines it holds and leaves the next.

use super::Parser;
use super::define::{only_definitions, take_definitions};
use super::line::{Indented, Line, indent, indented, is_blank, is_rule, strip_cols};
use super::opening::{FenceOpen, Marker, Title};
use super::table::TableHead;
use crate::model::{Block, CodeBlock, Heading, HeadingStyle, MathBlock, Paragraph};
use crate::scan::Cursor;

/// Indented code from `head` on: the lines indented four or more, and the
/// blank lines among them.
pub(super) fn indented_code(head: &Line, rest: &mut Cursor<'_, Line>) -> Block {
    let more = rest.take_while(|line| line.is_blank() || indent(&line.text) >= 4);
    let mut code: Vec<&str> = core::iter::once(head)
        .chain(more)
        .map(|line| strip_cols(&line.text, 4))
        .collect();
    while code.last().is_some_and(|line| is_blank(line)) {
        code.pop();
    }
    let mut text = code.join("\n");
    text.push('\n');
    Block::Code(CodeBlock {
        info: String::new(),
        text,
        line: head.no,
    })
}

/// A `$$` block from `head`, whose text after the `$$` is `after`: up to
/// a line that ends in `$$`. Never closed, it is a paragraph.
pub(super) fn math_block(head: &Line, after: &str, rest: &mut Cursor<'_, Line>) -> Block {
    let first = after.trim_end();
    if let Some(inner) = first
        .strip_suffix("$$")
        .map(str::trim)
        .filter(|inner| !inner.is_empty())
    {
        return Block::Math(MathBlock {
            tex: inner.to_owned(),
            line: head.no,
        });
    }
    let mut tex = String::from(first);
    while let Some(line) = rest.bump() {
        let text = line.text.trim_end();
        tex.push('\n');
        if let Some(last) = text.strip_suffix("$$") {
            tex.push_str(last);
            return Block::Math(MathBlock {
                tex: tex.trim().to_owned(),
                line: head.no,
            });
        }
        tex.push_str(text);
    }
    Block::Paragraph(Paragraph {
        text: format!("$${tex}"),
        line: head.no,
    })
}

/// The line under a paragraph that makes it a setext heading.
#[derive(Clone, Copy, Debug)]
enum Underline {
    /// `===`: a first-level heading.
    Equals,
    /// `---`: a second-level heading.
    Dashes,
}

/// What a line does to the paragraph above it.
enum Follow {
    /// Continues it.
    Text,
    /// Underlines it, and ends it.
    Underline(Underline),
    /// Ends it, and starts what comes next.
    End,
}

impl Parser<'_> {
    /// The fenced code block `open` opens on `head`: up to its closing
    /// fence, or to the end. Fenced as `math`, it is a maths block.
    pub(super) fn fenced(
        &self,
        head: &Line,
        open: FenceOpen<'_>,
        rest: &mut Cursor<'_, Line>,
    ) -> Block {
        let mut code = String::new();
        while let Some(line) = rest.bump() {
            if open.fence.closed_by(&line.text) {
                break;
            }
            code.push_str(strip_cols(&line.text, open.indent));
            code.push('\n');
        }
        let info = crate::text::unescape(open.info);
        let is_math = info
            .split_whitespace()
            .next()
            .is_some_and(|lang| lang.eq_ignore_ascii_case("math"));
        if self.math && is_math {
            Block::Math(MathBlock {
                tex: code,
                line: head.no,
            })
        } else {
            Block::Code(CodeBlock {
                info,
                text: code,
                line: head.no,
            })
        }
    }

    /// The paragraph that opens on `head` with `body`, or the setext
    /// heading it is when underlined.
    ///
    /// The link reference definitions at its start go into the definitions;
    /// when nothing else is left there is no block (nor an underline: see
    /// [`Parser::follow`]).
    pub(super) fn paragraph(
        &mut self,
        head: &Line,
        body: &str,
        rest: &mut Cursor<'_, Line>,
    ) -> Option<Block> {
        let mut lines = vec![body.to_owned()];
        let underline = loop {
            let Some(line) = rest.peek() else {
                break None;
            };
            match self.follow(line, rest.peek_nth(1), &lines) {
                Follow::Text => {
                    lines.push(line.text.trim_start().to_owned());
                    rest.bump();
                }
                Follow::Underline(underline) => {
                    rest.bump();
                    break Some(underline);
                }
                Follow::End => break None,
            }
        };
        let joined = lines.join("\n");
        let after = take_definitions(&joined, self.defs);
        if after.is_empty() {
            return None;
        }
        let text = after.trim_end();
        Some(underline.map_or_else(
            || {
                Block::Paragraph(Paragraph {
                    text: text.to_owned(),
                    line: head.no,
                })
            },
            |mark| {
                let title = Title::read(text.trim());
                Block::Heading(Heading {
                    level: match mark {
                        Underline::Equals => 1,
                        Underline::Dashes => 2,
                    },
                    text: title.text,
                    id: title.id,
                    style: HeadingStyle::Setext,
                    line: head.no,
                })
            },
        ))
    }

    /// What `line`, with `under` below it, does to the open paragraph
    /// `para`.
    ///
    /// A lazy line continues it, whatever it looks like. Under a paragraph
    /// of nothing but link reference definitions, which is no paragraph, a
    /// line of `=` or `-` underlines nothing: it is text, or a rule.
    fn follow(&self, line: &Line, under: Option<&Line>, para: &[String]) -> Follow {
        if line.is_blank() {
            return Follow::End;
        }
        if line.lazy {
            return Follow::Text;
        }
        let Indented { width, body } = indented(&line.text);
        let mark = body.trim_end();
        let underlines = width <= 3 && (is_run_of(mark, '=') || is_run_of(mark, '-'));
        if underlines && only_definitions(&para.join("\n")) {
            return if is_rule(body) {
                Follow::End
            } else {
                Follow::Text
            };
        }
        if width <= 3 && is_run_of(mark, '=') {
            return Follow::Underline(Underline::Equals);
        }
        if width <= 3 && is_run_of(mark, '-') {
            // Outside strict reading, a long paragraph with `---` under it
            // keeps its rule.
            return if self.strict || is_heading_like(para) || mark.len() < 3 {
                Follow::Underline(Underline::Dashes)
            } else {
                Follow::End
            };
        }
        if TableHead::opens(line, under) || self.interrupts(line) {
            return Follow::End;
        }
        // "Phase B, with the doctor:" then "5. A test": a numbered list
        // after a line that announces it, whatever its first number.
        if !self.strict
            && announces_list(para)
            && Marker::read(&line.text, line.col)
                .is_some_and(|marker| marker.is_ordered() && !marker.is_empty())
        {
            return Follow::End;
        }
        Follow::Text
    }
}

/// Whether `text` is one or more of `mark` and nothing else.
fn is_run_of(text: &str, mark: char) -> bool {
    !text.is_empty() && text.chars().all(|ch| ch == mark)
}

/// Whether a paragraph reads like a heading: one short line that does
/// not end like a sentence.
fn is_heading_like(para: &[String]) -> bool {
    match para {
        [only] => {
            !only.trim_end().ends_with(['.', ',', ';', ':', ')']) && only.chars().count() <= 120
        }
        _ => false,
    }
}

/// Whether a paragraph's last line ends with a colon, emphasis aside.
fn announces_list(para: &[String]) -> bool {
    para.last()
        .is_some_and(|last| last.trim_end().trim_end_matches('*').ends_with(':'))
}
