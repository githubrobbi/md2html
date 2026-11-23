// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Inline text, the way `CommonMark` reads it.
//!
//! Code spans come first, then a delimiter stack for emphasis and brackets,
//! so `***both***` nests, `[x]` stays text unless it is a link, and a URL
//! may hold parentheses.
//! GitHub's additions come along: `~~strike~~`, bare URLs, `[^note]`
//! references; and `$maths$`.
//!
//! One pass reads the text front to back into tokens: finished nodes,
//! and runs of `*`, `_` or `~` whose part is not known yet. A `]` that
//! makes a link resolves the emphasis inside it there and then; what is
//! left at the end is resolved in one more pass over the tokens.

mod autolink;
mod brackets;
mod emphasis;
mod entity;
mod html;
mod link;
mod span;
#[cfg(test)]
mod tests;

use alloc::collections::VecDeque;

use self::brackets::{Bracket, Frame};
use self::emphasis::{Flank, Run, Tok, emphasis};
use self::entity::Entity;
pub(crate) use self::entity::{decode_entities, unescape};
pub(crate) use self::html::{html_tag, safe_tag};
pub(crate) use self::link::{fits_label, link_destination, link_title};
use crate::model::{Defs, Node};
use crate::scan::Scan;

/// How raw HTML in the text is treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HtmlMode {
    /// A short list of harmless tags passes; anything else is shown as written.
    Safe,
    /// The author's HTML, rich but never executable (`--html`): see `sanitize`.
    Rich,
    /// Everything passes, as the `CommonMark` tests expect.
    Spec,
}

/// What the inline pass needs to know about the document.
pub(crate) struct Env<'defs> {
    /// The link references and footnotes.
    pub(crate) defs: &'defs Defs,
    /// Raw HTML policy.
    pub(crate) html: HtmlMode,
    /// `$...$` is maths.
    pub(crate) math: bool,
}

/// How deep emphasis, links and images may nest inside one another.
const MAX_NEST: usize = 64;

/// How deep a node's emphasis, links and images nest (a leaf is 0).
/// Never deeper than [`MAX_NEST`] plus one, by construction.
fn depth(node: &Node) -> usize {
    node.kids()
        .map_or(0, |kids| 1 + kids.iter().map(depth).fold(0, usize::max))
}

/// The inline nodes of `src`.
pub(crate) fn parse(src: &str, env: &Env<'_>) -> Vec<Node> {
    Inline {
        scan: Scan::new(src),
        env,
        toks: VecDeque::new(),
        text: String::new(),
        frames: Vec::new(),
        dead: 0,
        ticks: span::Ticks::default(),
    }
    .run()
}

/// One inline parse.
struct Inline<'src, 'env> {
    /// The text, read front to back.
    scan: Scan<'src>,
    /// The document's definitions and policies.
    env: &'env Env<'env>,
    /// The tokens since the innermost open bracket (all of them when no
    /// bracket is open).
    toks: VecDeque<Tok>,
    /// Plain text not yet pushed as a token.
    text: String,
    /// The open brackets, innermost last, each with the tokens before it.
    frames: Vec<Frame>,
    /// The open brackets below this depth in `frames` were opened before
    /// a link was made, so they can no longer make one (no links in
    /// links): one number instead of a walk over them all.
    dead: usize,
    /// What is known about the backtick runs ahead.
    ticks: span::Ticks,
}

impl Inline<'_, '_> {
    /// Reads the whole text into nodes.
    fn run(mut self) -> Vec<Node> {
        while let Some(ch) = self.scan.peek() {
            match ch {
                '\\' => self.backslash(),
                '`' => self.code(),
                '*' | '_' | '~' => self.delimiters(ch),
                '[' => self.open_bracket(Bracket::Link),
                '!' if self.scan.starts_with("![") => self.open_bracket(Bracket::Image),
                ']' => self.close_bracket(),
                '<' => self.angle(),
                '&' => self.ampersand(),
                '\n' => self.line_break(),
                '$' if self.env.math => self.maths(),
                'h' | 'w' | 'H' | 'W' => self.bare_url(),
                _ => self.keep_char(),
            }
        }
        self.finish()
    }

    /// The tokens as nodes once the text is read: brackets still open
    /// are text, and a trailing break is not one.
    fn finish(mut self) -> Vec<Node> {
        self.flush();
        let mut toks = core::mem::take(&mut self.toks);
        while let Some(frame) = self.frames.pop() {
            toks = frame.unclosed(toks);
        }
        while matches!(
            toks.back(),
            Some(Tok::Node(Node::HardBreak | Node::SoftBreak))
        ) {
            toks.pop_back();
        }
        merge(emphasis(toks))
    }

    /// Pushes the pending text as a node.
    fn flush(&mut self) {
        if !self.text.is_empty() {
            let text = core::mem::take(&mut self.text);
            self.toks.push_back(Tok::Node(Node::Text(text)));
        }
    }

    /// Pushes a finished node.
    fn node(&mut self, node: Node) {
        self.flush();
        self.toks.push_back(Tok::Node(node));
    }

    /// Reads the next character as plain text.
    fn keep_char(&mut self) {
        if let Some(ch) = self.scan.bump() {
            self.text.push(ch);
        }
    }

    /// `\`: an escaped punctuation mark, a hard break before a line's end,
    /// or a backslash.
    fn backslash(&mut self) {
        self.scan.bump();
        if let Some(escaped) = self.scan.eat_if(|ch| ch.is_ascii_punctuation()) {
            self.text.push(escaped);
        } else if self.scan.eat('\n') {
            self.node(Node::HardBreak);
            self.scan.take_while(|ch| ch == ' ');
        } else {
            self.text.push('\\');
        }
    }

    /// A run of backticks: a code span when a run as long closes it, text
    /// otherwise.
    fn code(&mut self) {
        let opener = self.scan.mark();
        let ticks = self.scan.take_while(|ch| ch == '`');
        match span::code_span(&mut self.scan, ticks.len(), opener, &mut self.ticks) {
            Some(code) => self.node(Node::Code(code)),
            None => self.text.push_str(ticks),
        }
    }

    /// A run of `*`, `_` or `~`: a delimiter run for the emphasis pass, or
    /// text (`~` makes strike-through only in pairs).
    fn delimiters(&mut self, mark: char) {
        // The start and the end of the text count as white space.
        let before = self.scan.prev().unwrap_or(' ');
        let marks = self.scan.take_while(|ch| ch == mark);
        let after = self.scan.peek().unwrap_or(' ');
        if mark == '~' && marks.len() != 2 {
            self.text.push_str(marks);
            return;
        }
        self.flush();
        self.toks.push_back(Tok::Run(Run {
            mark,
            len: marks.len(),
            flank: Flank::of(mark, before, after),
        }));
    }

    /// `<`: an autolink, a tag (treated as the HTML policy says), or text.
    fn angle(&mut self) {
        if let Some(link) = autolink::autolink(&mut self.scan) {
            self.node(Node::Link(link));
        } else if let Some(tag) = html::read_tag(&mut self.scan) {
            self.raw_html(tag);
        } else {
            self.keep_char();
        }
    }

    /// A tag from the text, as the HTML policy says.
    fn raw_html(&mut self, tag: &str) {
        match self.env.html {
            HtmlMode::Spec => self.node(Node::Html(tag.to_owned())),
            HtmlMode::Rich => {
                // A tag the filter does not allow is dropped, not shown.
                if let Some(clean) = crate::sanitize::rich_tag(tag) {
                    self.node(Node::Html(clean));
                }
            }
            HtmlMode::Safe => {
                // A comment is the author's note to self: never shown.
                if !tag.starts_with("<!--") {
                    match safe_tag(tag) {
                        Some(clean) => self.node(Node::Html(clean)),
                        None => self.text.push_str(tag),
                    }
                }
            }
        }
    }

    /// `&`: an entity reference, or text.
    fn ampersand(&mut self) {
        match entity::entity(&mut self.scan) {
            Some(Entity::Char(ch)) => self.text.push(ch),
            Some(Entity::Raw(name)) => self.node(Node::Html(format!("&{name};"))),
            None => self.keep_char(),
        }
    }

    /// A line's end: a hard break after two spaces, a soft one otherwise;
    /// the spaces around it go.
    fn line_break(&mut self) {
        self.scan.bump();
        let hard = self.text.ends_with("  ");
        let kept = self.text.trim_end_matches(' ').len();
        self.text.truncate(kept);
        self.node(if hard {
            Node::HardBreak
        } else {
            Node::SoftBreak
        });
        self.scan.take_while(|ch| ch == ' ');
    }

    /// `$`: maths, or text.
    fn maths(&mut self) {
        match span::math_span(&mut self.scan) {
            Some(math) => self.node(Node::Math(math)),
            None => self.keep_char(),
        }
    }

    /// A letter that may start a bare URL: one after a space, an opening
    /// mark or a quote, outside the `CommonMark` tests.
    fn bare_url(&mut self) {
        let before = self.scan.prev().unwrap_or(' ');
        let boundary = self.env.html != HtmlMode::Spec
            && (before.is_whitespace() || matches!(before, '*' | '_' | '~' | '(' | '"' | '\''));
        match boundary
            .then(|| autolink::bare_url(&mut self.scan))
            .flatten()
        {
            Some(link) => self.node(Node::Link(link)),
            None => self.keep_char(),
        }
    }
}

/// A link's text never holds another link: a bare URL inside it is text.
fn unlink(kids: Vec<Node>) -> Vec<Node> {
    kids.into_iter()
        .map(|node| {
            if let Node::Link(link) = node {
                Node::Text(plain(&link.kids))
            } else {
                node
            }
        })
        .collect()
}

/// Neighbouring text nodes as one.
fn merge(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if let Node::Text(text) = &node {
            if let Some(Node::Text(last)) = out.last_mut() {
                last.push_str(text);
                continue;
            }
        }
        out.push(node);
    }
    out
}

/// The words of nodes, markup dropped: an image's alt text, a title for
/// the search index.
pub(crate) fn plain(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(text) | Node::Code(text) => out.push_str(text),
            Node::Html(_) | Node::Note(_) => {}
            Node::SoftBreak | Node::HardBreak => out.push(' '),
            Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => {
                out.push_str(&plain(kids));
            }
            Node::Link(link) => out.push_str(&plain(&link.kids)),
            Node::Image(image) => out.push_str(&plain(&image.alt)),
            Node::Math(math) => out.push_str(&math.tex),
        }
    }
    out
}
