// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Leaf blocks: code, maths, and paragraphs with the setext headings that
//! underline them. Each reads the lines it holds and leaves the next.

use super::define::{only_definitions, take_definitions};
use super::line::{Indented, Line, indent, indented, is_blank, is_rule, strip_cols};
use super::lines::Lines;
use super::opening::{FenceOpen, Marker, Title};
use super::table::TableHead;
use super::{Parser, Reading};
use crate::model::{Block, CodeBlock, Heading, HeadingStyle, Level, MathBlock, Paragraph};

/// The text of indented code, its lines added one at a time: blank lines
/// count only once a line of code follows them.
#[derive(Default)]
struct Code {
    /// The lines up to the last line of code, each with its line ending.
    text: String,
    /// The blank lines since, which the code keeps only when more follows.
    gap: String,
}

impl Code {
    /// Adds a line of the code, four columns of its indentation off.
    fn push(&mut self, line: &str) {
        let target = if is_blank(line) {
            &mut self.gap
        } else {
            self.text.push_str(&self.gap);
            self.gap.clear();
            &mut self.text
        };
        target.push_str(line);
        target.push('\n');
    }

    /// The code, without the blank lines after it.
    fn into_text(self) -> String {
        if self.text.is_empty() {
            String::from("\n")
        } else {
            self.text
        }
    }
}

/// Indented code from `head` on: the lines indented four or more, and the
/// blank lines among them.
pub(super) fn indented_code(head: &Line<'_>, rest: &mut Lines<'_>) -> Block {
    let mut code = Code::default();
    code.push(strip_cols(&head.text, 4));
    while let Some(line) = rest.bump_if(|line| line.is_blank() || indent(&line.text) >= 4) {
        code.push(strip_cols(&line.text, 4));
    }
    Block::Code(Box::new(CodeBlock {
        info: String::new(),
        text: code.into_text(),
        line: head.no,
    }))
}

/// A `$$` block from `head`, whose text after the `$$` is `after`: up to
/// a line that ends in `$$`. Never closed, it is a paragraph.
pub(super) fn math_block(head: &Line<'_>, after: &str, rest: &mut Lines<'_>) -> Block {
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
        head: &Line<'_>,
        open: FenceOpen<'_>,
        rest: &mut Lines<'_>,
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
            Block::Code(Box::new(CodeBlock {
                info,
                text: code,
                line: head.no,
            }))
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
        head: &Line<'_>,
        body: &str,
        rest: &mut Lines<'_>,
    ) -> Option<Block> {
        let mut para = Running::new(body);
        let underline = loop {
            let (next, under) = rest.peek_two();
            let Some(line) = next else {
                break None;
            };
            match self.follow(line, under, &para) {
                Follow::Text => {
                    para.push(line.text.trim_start());
                    rest.bump();
                }
                Follow::Underline(underline) => {
                    rest.bump();
                    break Some(underline);
                }
                Follow::End => break None,
            }
        };
        let after = take_definitions(&para.text, self.defs);
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
                Block::Heading(Box::new(Heading {
                    level: match mark {
                        Underline::Equals => Level::ONE,
                        Underline::Dashes => Level::TWO,
                    },
                    text: title.text,
                    id: title.id,
                    style: HeadingStyle::Setext,
                    line: head.no,
                }))
            },
        ))
    }

    /// What `line`, with `under` below it, does to the open paragraph
    /// `para`.
    ///
    /// A lazy line continues it, whatever it looks like. Under a paragraph
    /// of nothing but link reference definitions, which is no paragraph, a
    /// line of `=` or `-` underlines nothing: it is text, or a rule.
    fn follow(&self, line: &Line<'_>, under: Option<&Line<'_>>, para: &Running) -> Follow {
        if line.is_blank() {
            return Follow::End;
        }
        if line.lazy {
            return Follow::Text;
        }
        let Indented { width, body } = indented(&line.text);
        let mark = body.trim_end();
        let underlines = width <= 3 && (is_run_of(mark, '=') || is_run_of(mark, '-'));
        if underlines && only_definitions(&para.text) {
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
            // Read as an author means it, a long paragraph with `---` under
            // it keeps its rule.
            return if self.reading == Reading::CommonMark
                || para.is_heading_like()
                || mark.len() < 3
            {
                Follow::Underline(Underline::Dashes)
            } else {
                Follow::End
            };
        }
        if TableHead::opens(line, under) || self.interrupts(line) {
            return Follow::End;
        }
        // "Phase B, with the doctor:" then "5. A test": a numbered list
        // after a line that announces it, whatever its first number. (An
        // item of a bullet list with text interrupts a paragraph anyway.)
        if self.reading == Reading::Author
            && para.announces
            && Marker::read(&line.text, line.col).is_some_and(|marker| !marker.is_empty())
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

/// A paragraph while its lines are read: they are copied once, joined.
struct Running {
    /// Its lines so far, joined.
    text: String,
    /// It has more than one line.
    several: bool,
    /// Its last line ends with a colon, emphasis aside: it announces a
    /// list.
    announces: bool,
}

impl Running {
    /// A paragraph whose first line is `first`.
    fn new(first: &str) -> Self {
        Self {
            text: first.to_owned(),
            several: false,
            announces: announces_list(first),
        }
    }

    /// Adds a line.
    fn push(&mut self, line: &str) {
        self.text.push('\n');
        self.text.push_str(line);
        self.several = true;
        self.announces = announces_list(line);
    }

    /// Whether it reads like a heading: one short line that does not end
    /// like a sentence.
    fn is_heading_like(&self) -> bool {
        !self.several
            && !self.text.trim_end().ends_with(['.', ',', ';', ':', ')'])
            && self.text.chars().count() <= 120
    }
}

/// Whether a paragraph's line ends with a colon, emphasis aside.
fn announces_list(line: &str) -> bool {
    line.trim_end().trim_end_matches('*').ends_with(':')
}
