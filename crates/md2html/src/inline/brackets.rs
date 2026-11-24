// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Brackets: `[` and `![` open a frame of their own, and a `]` closes the
//! innermost one, into a link, an image or a footnote reference when what
//! follows makes one, back into text otherwise.

use alloc::collections::VecDeque;

use super::emphasis::{Tok, emphasis};
use super::link::{self, Target, normalize_label};
use super::{Inline, MAX_NEST, depth, merge, unlink};
use crate::model::{Image, Link, Node};
use crate::scan::Mark;

/// What an opening bracket may become.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Bracket {
    /// `[`: a link or a footnote reference.
    Link,
    /// `![`: an image.
    Image,
}

impl Bracket {
    /// The bracket as written.
    const fn written(self) -> &'static str {
        match self {
            Self::Link => "[",
            Self::Image => "![",
        }
    }

    /// The bracket as a text token, for when it makes nothing.
    fn to_text(self) -> Tok {
        Tok::Node(Node::Text(self.written().to_owned()))
    }
}

/// An open bracket.
pub(super) struct Frame {
    /// The tokens before it.
    before: VecDeque<Tok>,
    /// Which bracket it is.
    bracket: Bracket,
    /// Where its text starts.
    label: Mark,
}

impl Frame {
    /// The tokens before the bracket, the bracket as text, and `inside`,
    /// in that order: a bracket that never closed.
    pub(super) fn unclosed(self, inside: VecDeque<Tok>) -> VecDeque<Tok> {
        join(self.before, self.bracket.to_text(), inside)
    }
}

/// `before`, then `middle`, then `after`, as one sequence. The shorter
/// side moves, so brackets that never make anything cost each token at
/// most a logarithmic number of moves, however deep they nest.
fn join(mut before: VecDeque<Tok>, middle: Tok, mut after: VecDeque<Tok>) -> VecDeque<Tok> {
    if before.len() >= after.len() {
        before.push_back(middle);
        before.append(&mut after);
        before
    } else {
        after.push_front(middle);
        while let Some(tok) = before.pop_back() {
            after.push_front(tok);
        }
        after
    }
}

impl Inline<'_, '_> {
    /// `[` or `![`: a frame of its own for what follows.
    pub(super) fn open_bracket(&mut self, bracket: Bracket) {
        self.flush();
        self.scan.eat_str(bracket.written());
        let before = core::mem::take(&mut self.toks);
        self.frames.push(Frame {
            before,
            bracket,
            label: self.scan.mark(),
        });
    }

    /// A `]`: a link, image or footnote reference if the bracket before it
    /// and what follows make one; text otherwise.
    pub(super) fn close_bracket(&mut self) {
        let Some(frame) = self.frames.pop() else {
            self.keep_char();
            return;
        };
        let label = self.scan.since(frame.label);
        self.scan.bump();
        let after = self.scan.mark();
        let may_link = frame.bracket == Bracket::Image || self.frames.len() >= self.dead;
        self.dead = self.dead.min(self.frames.len());
        if !may_link {
            self.stays_text(frame);
            return;
        }
        if frame.bracket == Bracket::Link {
            if let Some(key) = self.note(label) {
                // The label's text is still pending; it is the reference,
                // not prose.
                self.text.clear();
                self.toks = frame.before;
                self.node(Node::Note(key));
                return;
            }
        }
        match self.target(label) {
            Some(target) => self.make(frame, target, after),
            None => self.stays_text(frame),
        }
    }

    /// The bracket made nothing: it and its `]` are text, and what came
    /// between belongs to the frame around it again.
    fn stays_text(&mut self, frame: Frame) {
        let inside = core::mem::take(&mut self.toks);
        self.toks = frame.unclosed(inside);
        self.text.push(']');
    }

    /// The footnote a `[^label]` refers to, when the document has it and
    /// no `(` follows. A label too long to be one refers to nothing.
    fn note(&self, label: &str) -> Option<String> {
        let name = label
            .strip_prefix('^')
            .filter(|name| link::fits_label(name))?;
        let key = normalize_label(name);
        (!key.is_empty() && self.env.defs.notes.contains_key(&key) && self.scan.peek() != Some('('))
            .then_some(key)
    }

    /// Where the bracket whose text is `label` points: `(destination
    /// "title")`, a reference `[label]` or `[]`, or the label itself as a
    /// reference. Reads what it used. Bracket text too long to be a label
    /// is not looked up, so no `]` costs more than a label's length.
    fn target(&mut self, label: &str) -> Option<Target> {
        if let Some(target) = link::inline_target(&mut self.scan) {
            return Some(target);
        }
        let defs = self.env.defs;
        let mut ahead = self.scan.clone();
        if let Some(second) = link::label(&mut ahead) {
            // A label that follows decides, even when nothing is defined
            // for it; an empty one stands for the first.
            let named = if second.trim().is_empty() {
                label
            } else {
                second
            };
            if !link::fits_label(named) {
                return None;
            }
            let target = defs.reference(&normalize_label(named)).map(Target::from)?;
            self.scan = ahead;
            return Some(target);
        }
        if !link::fits_label(label) || label.trim().is_empty() {
            return None;
        }
        defs.reference(&normalize_label(label)).map(Target::from)
    }

    /// The link or image of `frame`, pointing at `target`. Emphasis
    /// inside it is settled here. Nested too deep to be a document, the
    /// brackets stay text and reading goes on just after the `]`.
    fn make(&mut self, frame: Frame, target: Target, after: Mark) {
        self.flush();
        let inside = core::mem::replace(&mut self.toks, frame.before);
        let kids = merge(emphasis(inside));
        if kids.iter().any(|kid| depth(kid) >= MAX_NEST) {
            self.scan.rewind(after);
            self.toks.push_back(frame.bracket.to_text());
            self.toks.extend(kids.into_iter().map(Tok::Node));
            self.text.push(']');
            return;
        }
        let Target { href, title } = target;
        let node = match frame.bracket {
            Bracket::Image => Node::Image(Image {
                src: href,
                title,
                alt: kids,
                attrs: link::attributes(&mut self.scan).unwrap_or_default(),
            }),
            Bracket::Link => {
                // Every `[` still open can no longer make a link.
                self.dead = self.frames.len();
                Node::Link(Link {
                    href,
                    title,
                    kids: unlink(kids),
                })
            }
        };
        self.toks.push_back(Tok::Node(node));
    }
}
