// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The blocks as the article of a page.
//!
//! The article holds the title and its lead, the chapters in the outline,
//! the reading guide as cards, callouts, figures, code, tables, footnotes,
//! glossary links, and the search index beside it. [`fragment()`] writes the
//! same blocks as plain HTML, for a page of someone else's and for the
//! `CommonMark` tests.

mod article;
mod blocks;
mod callout;
mod embed;
mod figure;
mod fragment;
mod nodes;
mod notes;

use alloc::collections::{BTreeMap, BTreeSet};
use core::num::NonZeroUsize;
use std::path::Path;

pub(crate) use self::embed::{MAX_EMBED, NoImage, image_kind, read_image};
pub(crate) use self::fragment::{bare_inline, fragment};
use self::nodes::Links;
use self::notes::Footnotes;
use crate::config::{Diagrams, Ui};
use crate::glossary::{Linker, Term};
use crate::inline::{Env, HtmlMode};
use crate::model::{Block, Defs, Node};
use crate::sanitize::Balance;
use crate::text::{Html, plain};

/// The share glyph beside a heading (a box with an arrow out of it).
pub(crate) const SHARE_ICON: &str = "<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 3v12M7.5 7.5 12 3l4.5 4.5\"/><path d=\"M8 11H6a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-8a1 1 0 0 0-1-1h-2\"/></svg>";

/// One searchable section.
#[derive(Clone, Debug)]
pub(crate) struct Section {
    /// The anchor.
    pub(crate) id: String,
    /// The heading.
    pub(crate) title: String,
    /// The chapter's heading.
    pub(crate) chapter: String,
    /// The words.
    pub(crate) text: String,
}

/// A problem worth telling the author, at a line of the Markdown.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Warning {
    /// The line, one-based; None when the whole page is meant.
    pub(crate) line: Option<NonZeroUsize>,
    /// What and what to do.
    pub(crate) message: String,
}

/// What the article needs to know.
pub(crate) struct Inputs<'a> {
    /// The document's definitions.
    pub(crate) defs: &'a Defs,
    /// The glossary terms, longest first.
    pub(crate) terms: &'a [Term],
    /// The interface words.
    pub(crate) ui: &'static Ui,
    /// Raw HTML policy.
    pub(crate) html: HtmlMode,
    /// `$...$` is maths.
    pub(crate) math: bool,
    /// The title when the document does not open with one.
    pub(crate) title: String,
    /// The line above the title.
    pub(crate) eyebrow: Html,
    /// Where relative links are looked for: the page's own folder first,
    /// then the Markdown's.
    pub(crate) bases: Vec<&'a Path>,
    /// The Markdown's folder: the only one an image is put inside the page
    /// from (with the folder md2html runs in), as SECURITY.md promises; the
    /// page's own folder, which the command line chose, is for links only.
    pub(crate) reads_from: Option<&'a Path>,
    /// Images go inside the page.
    pub(crate) embed: bool,
    /// The largest image embedded, in bytes.
    pub(crate) max_embed: u64,
    /// How Mermaid diagrams are drawn.
    pub(crate) mermaid: Diagrams,
    /// A mermaid.js was given, for the kinds drawn in the browser.
    pub(crate) mermaid_js: bool,
}

impl Inputs<'_> {
    /// What the inline parser needs of them.
    const fn env(&self) -> Env<'_> {
        Env {
            defs: self.defs,
            html: self.html,
            math: self.math,
        }
    }
}

/// The article and what came of it.
#[derive(Default)]
pub(crate) struct Article {
    /// The article's HTML: the hero, the chapters, the notes.
    pub(crate) out: String,
    /// The outline's items.
    pub(crate) contents: String,
    /// The searchable sections.
    pub(crate) sections: Vec<Section>,
    /// Every glossary mention linked, by term.
    pub(crate) minted: BTreeMap<String, usize>,
    /// What the author should know.
    pub(crate) warnings: Vec<Warning>,
    /// A figure is on the page.
    pub(crate) has_figure: bool,
    /// A diagram on the page needs mermaid.js.
    pub(crate) mermaid_js: bool,
    /// Per-figure size rules.
    pub(crate) css: String,
    /// The lead's words, for the description.
    pub(crate) lead: String,
}

/// Page anchors the page itself uses: a heading never takes them.
const RESERVED: &[&str] = &[
    "top",
    "main",
    "opening",
    "toc",
    "dock",
    "edge",
    "toggle",
    "bar",
    "crumb",
    "find",
    "q",
    "hits",
    "idx",
    "theme",
    "size",
    "size-up",
    "size-down",
    "open-find",
    "glossary",
    "notes",
    "md2html-ui",
];

/// The article of a page.
pub(crate) fn article(blocks: &[Block], inputs: &Inputs<'_>) -> Article {
    let mut renderer = Renderer::new(inputs);
    renderer.run(blocks);
    renderer.finish()
}

/// The renderer's state.
struct Renderer<'a, 'b> {
    /// What it was given.
    inp: &'b Inputs<'a>,
    /// What it made.
    art: Article,
    /// A chapter's sub-list is open in the outline.
    sub_open: bool,
    /// The chapter being rendered.
    chapter: String,
    /// Glossary bookkeeping.
    linker: Linker<'a>,
    /// Anchors used, for unique ones.
    ids: BTreeMap<String, usize>,
    /// The footnotes referenced so far.
    notes: Footnotes,
    /// Figures so far.
    figures: usize,
    /// Diagrams so far.
    diagrams: usize,
    /// The line being rendered.
    line: usize,
    /// The author's open elements.
    balance: Balance,
    /// The warnings told so far, to tell each once: looked up in a set,
    /// so a text with many warnings costs no more than their number.
    told: BTreeSet<Warning>,
}

impl<'a, 'b> Renderer<'a, 'b> {
    /// A renderer for one article.
    fn new(inp: &'b Inputs<'a>) -> Self {
        let ids = RESERVED.iter().map(|id| ((*id).to_owned(), 1)).collect();
        Self {
            inp,
            art: Article::default(),
            sub_open: false,
            chapter: String::new(),
            linker: Linker::new(inp.terms),
            ids,
            notes: Footnotes::new(inp.defs, &inp.env()),
            figures: 0,
            diagrams: 0,
            line: 0,
            balance: if inp.html == HtmlMode::Spec {
                Balance::as_written()
            } else {
                Balance::new()
            },
            told: BTreeSet::new(),
        }
    }

    /// What `write` writes inside one of md2html's elements: whatever the
    /// author opens there is closed before it ends.
    fn scoped<Write>(&mut self, write: Write) -> String
    where
        Write: FnOnce(&mut Self, &mut String),
    {
        let floor = self.balance.enter();
        let mut out = String::new();
        write(self, &mut out);
        self.balance.leave(floor, &mut out);
        out
    }

    /// Tells the author what balancing their tags changed.
    fn tell_mends(&mut self) {
        for mend in self.balance.take_mends() {
            self.warn_at(mend.line(), mend.message());
        }
    }

    /// The inline nodes of raw text.
    fn parse(&self, text: &str) -> Vec<Node> {
        crate::inline::parse(text, &self.inp.env())
    }

    /// A warning at the current line.
    fn warn(&mut self, message: impl Into<String>) {
        self.warn_at(self.line, message);
    }

    /// A warning at `line` (zero: the whole page).
    fn warn_at(&mut self, line: usize, message: impl Into<String>) {
        let warning = Warning {
            line: NonZeroUsize::new(line),
            message: message.into(),
        };
        if self.told.insert(warning.clone()) {
            self.art.warnings.push(warning);
        }
    }

    /// The anchor of the section being rendered, if one has begun.
    fn section_id(&self) -> Option<&str> {
        self.art.sections.last().map(|section| section.id.as_str())
    }

    /// An address for a block a reader may link to (a code block, a
    /// diagram): its section and its kind, `4-code-and-maths-diagram`, a
    /// number after it for the second and on.
    fn block_id(&mut self, kind: &str) -> String {
        let wanted = format!("{}-{kind}", self.section_id().unwrap_or("top"));
        self.unique(&wanted)
    }

    /// An id no other element on the page has: `wanted`, or `wanted-2`, `-3`...
    fn unique(&mut self, wanted: &str) -> String {
        let base = if wanted.is_empty() {
            String::from("section")
        } else {
            wanted.to_owned()
        };
        let Some(seen) = self.ids.get(&base).copied() else {
            self.ids.insert(base.clone(), 1);
            return base;
        };
        let mut suffix = seen + 1;
        loop {
            let candidate = format!("{base}-{suffix}");
            if !self.ids.contains_key(&candidate) {
                self.ids.insert(base, suffix);
                self.ids.insert(candidate.clone(), 1);
                return candidate;
            }
            suffix += 1;
        }
    }

    /// Adds rendered words to the current section.
    fn remember(&mut self, html: &str) {
        let words = plain(html);
        if words.is_empty() {
            return;
        }
        if let Some(section) = self.art.sections.last_mut() {
            if !section.text.is_empty() {
                section.text.push(' ');
            }
            section.text.push_str(&words);
        }
    }
}
