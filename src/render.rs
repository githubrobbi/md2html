// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The blocks as the article of a page: the title and its lead, the
//! chapters in the outline, the reading guide as cards, callouts, figures,
//! code, tables, footnotes, glossary links, and the search index beside
//! it. `fragment` writes the same blocks as plain HTML, for a page of
//! someone else's and for the `CommonMark` tests.

use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use std::path::Path;

use crate::block::{Align, Block, Defs, Item};
use crate::config::Ui;
use crate::glossary::{Linker, Term, is_glossary_heading};
use crate::inline::{Attrs, Env, HtmlMode, Node, html_tag, safe_tag};
use crate::text::{esc, is_relative, plain, safe_url, slug, url_attr};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Warning {
    /// One-based; 0 when the whole page is meant.
    pub(crate) line: usize,
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
    /// The line above the title, escaped.
    pub(crate) eyebrow: String,
    /// Where relative links are looked for: the page's own folder first,
    /// then the Markdown's.
    pub(crate) bases: Vec<&'a Path>,
    /// Images go inside the page.
    pub(crate) embed: bool,
    /// The largest image embedded, in bytes.
    pub(crate) max_embed: u64,
    /// How Mermaid diagrams are drawn.
    pub(crate) mermaid: Diagrams,
    /// A mermaid.js was given, for the kinds drawn in the browser.
    pub(crate) mermaid_js: bool,
}

/// How a Mermaid block becomes a picture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Diagrams {
    /// Drawn here as SVG; what cannot be is left to mermaid.js, if given.
    Native,
    /// Every diagram left to mermaid.js.
    Js,
    /// Shown as code.
    Code,
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

/// The callout kinds, as class names, in the interface's order.
const KINDS: [&str; 5] = ["note", "tip", "important", "warning", "caution"];

/// The words that open a callout in English and German, by kind.
const CALLOUT_WORDS: [&[&str]; 5] = [
    &["note", "hinweis", "info", "anmerkung"],
    &["tip", "tipp", "hint"],
    &["important", "wichtig"],
    &["warning", "achtung", "warnung"],
    &["caution", "vorsicht", "danger", "gefahr"],
];

/// The article of a page.
pub(crate) fn article(blocks: &[Block], inputs: &Inputs<'_>) -> Article {
    let mut r = Renderer::new(inputs);
    r.run(blocks);
    r.finish()
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
    linker: Linker,
    /// Anchors used, for unique ones.
    ids: BTreeMap<String, usize>,
    /// Footnotes in the order they are first referenced.
    notes: Vec<String>,
    /// References made to each footnote.
    note_refs: BTreeMap<String, usize>,
    /// Each footnote as plain text, for its hover.
    note_plain: BTreeMap<String, String>,
    /// Figures so far.
    figures: usize,
    /// Diagrams so far.
    diagrams: usize,
    /// The line being rendered.
    line: usize,
}

impl<'a, 'b> Renderer<'a, 'b> {
    fn new(inp: &'b Inputs<'a>) -> Self {
        let mut ids = BTreeMap::new();
        for id in RESERVED {
            ids.insert((*id).to_owned(), 1);
        }
        let env = Env {
            defs: inp.defs,
            html: inp.html,
            math: inp.math,
        };
        let note_plain = inp
            .defs
            .notes
            .iter()
            .map(|(label, blocks)| {
                let text = blocks
                    .iter()
                    .filter_map(|b| match b {
                        Block::Paragraph { text, .. } => {
                            Some(crate::inline::plain(&crate::inline::parse(text, &env)))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                (label.clone(), text)
            })
            .collect();
        Self {
            inp,
            art: Article::default(),
            sub_open: false,
            chapter: String::new(),
            linker: Linker::default(),
            ids,
            notes: Vec::new(),
            note_refs: BTreeMap::new(),
            note_plain,
            figures: 0,
            diagrams: 0,
            line: 0,
        }
    }

    /// The inline nodes of raw text.
    fn parse(&self, text: &str) -> Vec<Node> {
        let env = Env {
            defs: self.inp.defs,
            html: self.inp.html,
            math: self.inp.math,
        };
        crate::inline::parse(text, &env)
    }

    /// A warning at the current line.
    fn warn(&mut self, message: impl Into<String>) {
        let warning = Warning {
            line: self.line,
            message: message.into(),
        };
        if !self.art.warnings.contains(&warning) {
            self.art.warnings.push(warning);
        }
    }

    /// A unique anchor from `wanted`.
    /// An address for a block a reader may link to (a code block, a
    /// diagram): its section and its kind, `4-code-and-maths-diagram`, a
    /// number after it for the second and on.
    fn block_id(&mut self, kind: &str) -> String {
        let section = self
            .art
            .sections
            .last()
            .map_or_else(|| String::from("top"), |s| s.id.clone());
        self.unique(&format!("{section}-{kind}"))
    }

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
        let mut n = seen + 1;
        loop {
            let candidate = format!("{base}-{n}");
            if !self.ids.contains_key(&candidate) {
                self.ids.insert(base, n);
                self.ids.insert(candidate.clone(), 1);
                return candidate;
            }
            n += 1;
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

    fn run(&mut self, blocks: &[Block]) {
        let first = blocks.iter().position(|b| !is_comment(b));
        let title_at =
            first.filter(|at| matches!(blocks.get(*at), Some(Block::Heading { level: 1, .. })));
        let h1s = blocks
            .iter()
            .filter(|b| matches!(b, Block::Heading { level: 1, .. }))
            .count();
        // The first heading is the title; any other level-one heading is a
        // chapter, and every heading moves down one level to make room.
        let others = if title_at.is_some() {
            h1s.saturating_sub(1)
        } else {
            h1s
        };
        let shift: u8 = u8::from(others > 0);
        // The anchors of the chapters first, so the reading guide can point
        // at a chapter further down.
        let mut planned: Vec<Option<String>> = Vec::with_capacity(blocks.len());
        let mut numbered: Vec<(u32, String)> = Vec::new();
        for (at, block) in blocks.iter().enumerate() {
            let id = match block {
                Block::Heading {
                    text, id, level, ..
                } if Some(at) != title_at => {
                    let shown = crate::inline::plain(&self.parse(text));
                    let wanted = id.clone().unwrap_or_else(|| slug(&shown));
                    let id = self.unique(&wanted);
                    if level.saturating_add(shift) == 2 {
                        if let Some(n) = shown
                            .split_once('.')
                            .and_then(|(head, _)| head.trim().parse::<u32>().ok())
                        {
                            numbered.push((n, id.clone()));
                        }
                    }
                    Some(id)
                }
                _ => None,
            };
            planned.push(id);
        }
        // The hero: the eyebrow, the title, and a lead if one follows.
        let title_html = match title_at.and_then(|at| blocks.get(at)) {
            Some(Block::Heading { text, .. }) => {
                let nodes = self.parse(text);
                self.inline(&nodes, false)
            }
            _ => esc(&self.inp.title),
        };
        let title_plain = plain(&title_html);
        let _ = write!(
            self.art.out,
            "<header class=\"hero\" id=\"opening\"><div class=\"eyebrow\">{}</div><h1>{title_html}</h1>",
            self.inp.eyebrow
        );
        self.art.sections.push(Section {
            id: String::from("opening"),
            title: if title_plain.is_empty() {
                self.inp.ui.top.to_owned()
            } else {
                title_plain
            },
            chapter: String::new(),
            text: String::new(),
        });
        let mut hero_open = true;
        let mut lead_pending = true;
        let mut skipping = false;
        let mut guide = false;
        for (at, block) in blocks.iter().enumerate() {
            if Some(at) == title_at {
                continue;
            }
            if let Some(line) = block_line(block) {
                self.line = line;
            }
            if let Block::Heading {
                level,
                text,
                setext,
                ..
            } = block
            {
                let level = level.saturating_add(shift).min(6);
                let nodes = self.parse(text);
                let shown_plain = crate::inline::plain(&nodes);
                if level <= 2 {
                    skipping = level == 2
                        && (is_glossary_heading(&shown_plain) || is_contents(&shown_plain));
                    guide = level == 2 && is_guide(&shown_plain);
                }
                if *setext && level == 2 && shown_plain.split_whitespace().count() > 3 {
                    self.warn("a line of --- under this text makes it a heading; leave a blank line above --- for a rule");
                }
                if skipping {
                    continue;
                }
                if hero_open {
                    self.art.out.push_str("</header>");
                    hero_open = false;
                }
                lead_pending = false;
                let id = planned.get(at).cloned().flatten().unwrap_or_default();
                self.heading(level, &nodes, &shown_plain, &id);
                continue;
            }
            if skipping {
                continue;
            }
            if let Block::Paragraph { text, .. } = block {
                if text.starts_with("Version of this handbook") || text.starts_with("SPDX-") {
                    continue;
                }
                if lead_pending && hero_open && lone_image(&self.parse(text)).is_none() {
                    let nodes = self.parse(text);
                    let lead = self.inline(&nodes, true);
                    self.remember(&lead);
                    self.art.lead = plain(&lead);
                    let _ = write!(self.art.out, "<p class=\"lead\">{lead}</p></header>");
                    hero_open = false;
                    lead_pending = false;
                    continue;
                }
            }
            if is_comment(block) {
                continue;
            }
            if hero_open {
                self.art.out.push_str("</header>");
                hero_open = false;
            }
            lead_pending = false;
            if guide {
                if let Block::List { items, .. } = block {
                    let cards = self.cards(items, &numbered);
                    self.art.out.push_str(&cards);
                    guide = false;
                    continue;
                }
            }
            let html = self.block(block, false);
            self.remember(&html);
            self.art.out.push_str(&html);
        }
        if hero_open {
            self.art.out.push_str("</header>");
        }
        if self.sub_open {
            self.art.contents.push_str("</ol></li>");
            self.sub_open = false;
        }
    }

    /// The footnotes, then the article as finished.
    fn finish(mut self) -> Article {
        if !self.notes.is_empty() {
            let mut list = String::new();
            let mut at = 0;
            while at < self.notes.len() {
                let label = self.notes[at].clone();
                let n = at + 1;
                at += 1;
                let Some(blocks) = self.inp.defs.notes.get(&label) else {
                    continue;
                };
                let mut body = String::new();
                for block in blocks {
                    body.push_str(&self.block(block, false));
                }
                let back = format!(
                    " <a class=\"back\" href=\"#fnref-{n}-1\" aria-label=\"{} {n}\">{}</a>",
                    esc(self.inp.ui.back),
                    esc(self.inp.ui.back)
                );
                if body.ends_with("</p>") {
                    body.truncate(body.len() - 4);
                    body.push_str(&back);
                    body.push_str("</p>");
                } else {
                    body.push_str(&back);
                }
                let _ = write!(list, "<li id=\"fn-{n}\">{body}</li>");
            }
            let title = esc(self.inp.ui.notes);
            let _ = write!(
                self.art.out,
                "<section class=\"notes\"><h2 id=\"notes\">{title}<a class=\"anchor\" href=\"#notes\" aria-label=\"{}\">{share}</a></h2><ol>{list}</ol></section>",
                esc(self.inp.ui.link_section),
                share = crate::page::SHARE_ICON
            );
            let _ = write!(self.art.contents, "<li><a href=\"#notes\">{title}</a></li>");
            self.art.sections.push(Section {
                id: String::from("notes"),
                title: self.inp.ui.notes.to_owned(),
                chapter: self.inp.ui.notes.to_owned(),
                text: plain(&list),
            });
        }
        let unused: Vec<String> = self
            .inp
            .defs
            .notes
            .keys()
            .filter(|label| !self.notes.contains(label))
            .cloned()
            .collect();
        for label in unused {
            self.line = 0;
            self.warn(format!(
                "the footnote [^{label}] is defined but never referenced"
            ));
        }
        self.art.minted = core::mem::take(&mut self.linker.minted);
        self.art
    }

    /// A heading: into the article with its anchor, into the outline, and a
    /// new section for the search.
    fn heading(&mut self, level: u8, nodes: &[Node], shown: &str, id: &str) {
        let html = self.inline(nodes, false);
        let text = esc(shown);
        if level == 2 {
            self.linker.seen.clear();
            shown.clone_into(&mut self.chapter);
            if self.sub_open {
                self.art.contents.push_str("</ol></li>");
            }
            let _ = write!(
                self.art.contents,
                "<li><a href=\"#{id}\">{text}</a><ol class=\"sub\">"
            );
            self.sub_open = true;
        } else if level == 3 {
            let _ = write!(self.art.contents, "<li><a href=\"#{id}\">{text}</a></li>");
        }
        if level <= 3 {
            self.art.sections.push(Section {
                id: id.to_owned(),
                title: shown.to_owned(),
                chapter: self.chapter.clone(),
                text: String::new(),
            });
        }
        let _ = write!(
            self.art.out,
            "<h{level} id=\"{id}\">{html}<a class=\"anchor\" href=\"#{id}\" aria-label=\"{}\">{share}</a></h{level}>",
            esc(self.inp.ui.link_section),
            share = crate::page::SHARE_ICON
        );
    }

    /// The reading guide's bullets as cards: the bold lead names the
    /// chapters, the rest says what they are for. No glossary links: the
    /// card is a link, and a link inside a link is not HTML.
    fn cards(&mut self, items: &[Item], numbered: &[(u32, String)]) -> String {
        let mut out = String::from("<div class=\"cards\">");
        for item in items {
            let raw = item
                .blocks
                .iter()
                .filter_map(|b| match b {
                    Block::Paragraph { text, .. } => Some(text.replace('\n', " ")),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            let (lead, rest) = raw
                .strip_prefix("**")
                .and_then(|tail| tail.split_once("**"))
                .unwrap_or(("", raw.as_str()));
            let first = lead
                .split(|ch: char| !ch.is_ascii_digit())
                .find_map(|piece| piece.parse::<u32>().ok());
            let href = first
                .and_then(|number| {
                    numbered
                        .iter()
                        .find(|(n, _)| *n == number)
                        .map(|(_, id)| id.clone())
                })
                .unwrap_or_default();
            if href.is_empty() && !lead.is_empty() {
                self.warn(format!(
                    "the reading-guide card \"{lead}\" names no numbered chapter"
                ));
            }
            let lead_nodes = self.parse(lead);
            let lead_html = self.inline(&lead_nodes, false);
            let rest_nodes = self.parse(rest.trim());
            let said = self.inline_nolinks(&rest_nodes);
            self.remember(&lead_html);
            self.remember(&said);
            let _ = write!(
                out,
                "<a class=\"card\" href=\"#{href}\"><span class=\"k\">{lead_html}</span><span>{said}</span></a>"
            );
        }
        out.push_str("</div>");
        out
    }

    /// One block, anywhere: at the top, in a list, in a quote, in a note.
    fn block(&mut self, block: &Block, tight: bool) -> String {
        if let Some(line) = block_line(block) {
            self.line = line;
        }
        match block {
            Block::Paragraph { text, .. } => {
                if text.starts_with("SPDX-") {
                    return String::new();
                }
                let nodes = self.parse(text);
                if let Some(image) = lone_image(&nodes) {
                    return self.figure(image);
                }
                let html = self.inline(&nodes, true);
                if html.contains("[^") {
                    self.warn(
                        "a footnote reference [^...] has no definition; it is shown as written",
                    );
                }
                if tight {
                    html
                } else {
                    format!("<p>{html}</p>")
                }
            }
            Block::Heading {
                level, text, id, ..
            } => {
                let nodes = self.parse(text);
                let shown = crate::inline::plain(&nodes);
                let wanted = id.clone().unwrap_or_else(|| slug(&shown));
                let id = self.unique(&wanted);
                let html = self.inline(&nodes, false);
                format!("<h{level} id=\"{id}\">{html}</h{level}>")
            }
            Block::Code { info, text, .. } => self.code(info, text),
            Block::Html { text, .. } => self.html_block(text),
            Block::Rule => String::from("<hr>"),
            Block::Quote { blocks, .. } => self.quote(blocks),
            Block::List {
                ordered,
                start,
                tight,
                items,
            } => self.list(*ordered, *start, *tight, items),
            Block::Table {
                align, head, rows, ..
            } => self.table(align, head, rows),
            Block::Math { text, .. } => {
                format!("<div class=\"math\">{}</div>", math_html(text, true))
            }
        }
    }

    /// Inline nodes as HTML; `terms` links the glossary's words.
    fn inline(&mut self, nodes: &[Node], terms: bool) -> String {
        let mut out = String::new();
        self.inline_into(nodes, terms, false, &mut out);
        out
    }

    /// Inline nodes as HTML without any links of their own (a card is one).
    fn inline_nolinks(&mut self, nodes: &[Node]) -> String {
        let mut out = String::new();
        self.inline_into(nodes, false, true, &mut out);
        out
    }

    fn inline_into(&mut self, nodes: &[Node], terms: bool, in_link: bool, out: &mut String) {
        for node in nodes {
            match node {
                Node::Text(text) => {
                    let escaped = esc(text);
                    if terms && !in_link && !self.inp.terms.is_empty() {
                        out.push_str(&self.linker.link(&escaped, self.inp.terms));
                    } else {
                        out.push_str(&escaped);
                    }
                }
                Node::Code(code) => {
                    let rendered = format!("<code>{}</code>", esc(code));
                    if terms && !in_link && !self.inp.terms.is_empty() {
                        out.push_str(&self.linker.code(code, &rendered, self.inp.terms));
                    } else {
                        out.push_str(&rendered);
                    }
                }
                Node::Html(html) => out.push_str(html),
                Node::SoftBreak => out.push('\n'),
                Node::HardBreak => out.push_str("<br>\n"),
                Node::Em(kids) => {
                    out.push_str("<em>");
                    self.inline_into(kids, terms, in_link, out);
                    out.push_str("</em>");
                }
                Node::Strong(kids) => {
                    out.push_str("<strong>");
                    self.inline_into(kids, terms, in_link, out);
                    out.push_str("</strong>");
                }
                Node::Del(kids) => {
                    out.push_str("<del>");
                    self.inline_into(kids, terms, in_link, out);
                    out.push_str("</del>");
                }
                Node::Link { href, title, kids } => {
                    if in_link {
                        self.inline_into(kids, terms, true, out);
                        continue;
                    }
                    match self.href(href, false) {
                        Some(url) => {
                            let title = title
                                .as_ref()
                                .map(|t| format!(" title=\"{}\"", esc(t)))
                                .unwrap_or_default();
                            // A link that shows its own address needs it no second time in print.
                            let shown = crate::inline::plain(kids);
                            let bare = shown == *href
                                || format!("http://{shown}") == *href
                                || format!("mailto:{shown}") == *href;
                            let class = if bare { " class=\"url\"" } else { "" };
                            let _ = write!(out, "<a{class} href=\"{url}\"{title}>");
                            self.inline_into(kids, terms, true, out);
                            out.push_str("</a>");
                        }
                        None => self.inline_into(kids, terms, in_link, out),
                    }
                }
                Node::Image {
                    src,
                    title,
                    alt,
                    attrs,
                } => {
                    let html = self.image(src, title.as_deref(), alt, attrs);
                    out.push_str(&html);
                }
                Node::Note(label) => {
                    let html = self.note_ref(label);
                    out.push_str(&html);
                }
                Node::Math { tex, display } => out.push_str(&math_html(tex, *display)),
            }
        }
    }

    /// A destination fit for the page: script URLs refused, a link to a
    /// Markdown file next to this one sent to its HTML twin, an image put
    /// inside the page when asked. None when it must not be a link.
    fn href(&mut self, url: &str, image: bool) -> Option<String> {
        if !safe_url(url, image) {
            let scheme = url.split(':').next().unwrap_or_default().trim().to_owned();
            self.warn(format!(
                "a link to a `{scheme}:` address was removed; only its text is shown"
            ));
            return None;
        }
        let mut url = url.to_owned();
        if is_relative(&url) && !url.is_empty() {
            let cut = url.find(['#', '?']).unwrap_or(url.len());
            let (path, rest) = url.split_at(cut);
            let decoded = percent_decode(path);
            if !self.inp.bases.is_empty() {
                let found = self
                    .inp
                    .bases
                    .iter()
                    .map(|base| base.join(&decoded))
                    .find(|t| t.exists());
                if !path.is_empty() && found.is_none() {
                    self.warn(format!("`{decoded}` does not exist next to this page"));
                }
                if let Some(target) = found {
                    let markdown = Path::new(&decoded).extension().is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
                    });
                    if !image && markdown && target.is_file() {
                        let stem = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
                        url = format!("{stem}.html{rest}");
                    } else if image && self.inp.embed && target.is_file() {
                        match data_uri(&target, &self.inp.bases, self.inp.max_embed) {
                            Ok(data) => return Some(data),
                            Err(why) => self.warn(format!("`{decoded}` is not embedded: {why}")),
                        }
                    }
                }
            }
        }
        Some(url_attr(&url))
    }

    /// An image inside a line of text.
    fn image(&mut self, src: &str, title: Option<&str>, alt: &[Node], attrs: &Attrs) -> String {
        let alt_text = esc(&crate::inline::plain(alt));
        let Some(url) = self.href(src, true) else {
            return alt_text;
        };
        let title = title
            .map(|t| format!(" title=\"{}\"", esc(t)))
            .unwrap_or_default();
        let id = self.size_rule(attrs, false);
        let classes = if attrs.classes.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", esc(&attrs.classes.join(" ")))
        };
        format!("<img src=\"{url}\" alt=\"{alt_text}\"{title}{id}{classes} loading=\"lazy\">")
    }

    /// A rule for an image's `{width=...}`, and the id attribute it hangs on.
    fn size_rule(&mut self, attrs: &Attrs, figure: bool) -> String {
        if attrs.width.is_none() && attrs.height.is_none() && attrs.id.is_none() {
            return String::new();
        }
        self.figures += usize::from(!figure);
        let id = attrs
            .id
            .clone()
            .unwrap_or_else(|| format!("fig-{}", self.figures));
        let id = self.unique(&id);
        let target = if figure {
            format!("#{id} img")
        } else {
            format!("#{id}")
        };
        if let Some(width) = &attrs.width {
            let width = if width.chars().all(|c| c.is_ascii_digit()) {
                format!("{width}px")
            } else {
                width.clone()
            };
            match width.strip_suffix('%').and_then(|n| n.parse::<f64>().ok()) {
                Some(pct) if figure => {
                    let _ = write!(
                        self.art.css,
                        "{target}{{width:calc((100% - 22px)*{:.4} + 22px)}}",
                        pct / 100.0
                    );
                }
                _ => {
                    let _ = write!(self.art.css, "{target}{{width:{width};max-width:100%}}");
                }
            }
        }
        if let Some(height) = &attrs.height {
            let height = if height.chars().all(|c| c.is_ascii_digit()) {
                format!("{height}px")
            } else {
                height.clone()
            };
            let _ = write!(self.art.css, "{target}{{height:{height};width:auto}}");
        }
        format!(" id=\"{id}\"")
    }

    /// An image on a line of its own: a figure with its caption under it,
    /// enlarged over the page on a click (`page/figure.js`).
    fn figure(&mut self, image: &Node) -> String {
        let Node::Image {
            src,
            title,
            alt,
            attrs,
        } = image
        else {
            return String::new();
        };
        self.figures += 1;
        let caption = self.inline(alt, true);
        let alt_text = esc(&crate::inline::plain(alt));
        let Some(url) = self.href(src, true) else {
            return format!("<p>{caption}</p>");
        };
        self.art.has_figure = true;
        let embedded = url.starts_with("data:");
        let mut attrs = attrs.clone();
        if embedded && attrs.id.is_none() {
            attrs.id = Some(format!("fig-{}", self.figures));
        }
        let id = self.size_rule(&attrs, true);
        let link = if embedded {
            id.trim()
                .strip_prefix("id=\"")
                .and_then(|v| v.strip_suffix('"'))
                .map(|v| format!("#{v}"))
                .unwrap_or_default()
        } else {
            url.clone()
        };
        let title = title
            .as_ref()
            .map(|t| format!(" title=\"{}\"", esc(t)))
            .unwrap_or_default();
        let classes = if attrs.classes.is_empty() {
            String::new()
        } else {
            format!(" {}", esc(&attrs.classes.join(" ")))
        };
        let figcaption = if caption.is_empty() {
            String::new()
        } else {
            format!("<figcaption>{caption}</figcaption>")
        };
        format!(
            "<figure class=\"fig{classes}\"{id}><a href=\"{link}\" target=\"_blank\" rel=\"noopener\">\
<img src=\"{url}\" alt=\"{alt_text}\"{title} loading=\"lazy\"></a>{figcaption}</figure>"
        )
    }

    /// A footnote reference: its number, linked to the note, the note's
    /// words on hover.
    fn note_ref(&mut self, label: &str) -> String {
        let n = if let Some(at) = self.notes.iter().position(|l| l == label) {
            at + 1
        } else {
            self.notes.push(label.to_owned());
            self.notes.len()
        };
        let k = {
            let count = self.note_refs.entry(label.to_owned()).or_insert(0);
            *count += 1;
            *count
        };
        let def = esc(self.note_plain.get(label).map_or("", String::as_str));
        let aria = esc(&self.inp.ui.note_n.replace("{n}", &n.to_string()));
        format!(
            "<sup class=\"fn\"><a class=\"fnref\" id=\"fnref-{n}-{k}\" href=\"#fn-{n}\" data-def=\"{def}\" aria-label=\"{aria}\">{n}</a></sup>"
        )
    }

    /// A code block, highlighted where the language is known.
    fn code(&mut self, info: &str, text: &str) -> String {
        let lang = info.split_whitespace().next().unwrap_or_default();
        if lang.eq_ignore_ascii_case("mermaid") {
            if let Some(html) = self.diagram(text) {
                return html;
            }
        }
        let body = crate::highlight::highlight(lang, text).unwrap_or_else(|| esc(text));
        let (label, named, class) = if lang.is_empty() {
            (String::new(), "", String::new())
        } else {
            (
                format!("<span class=\"lang\">{}</span>", esc(lang)),
                " named",
                format!(" class=\"language-{}\"", esc(&lang.to_ascii_lowercase())),
            )
        };
        let id = self.block_id("code");
        format!(
            "<div class=\"code{named}\" id=\"{id}\">{label}<button class=\"copy\" type=\"button\">{}</button><pre><code{class}>{body}</code></pre></div>",
            esc(self.inp.ui.copy[0])
        )
    }

    /// A Mermaid block as a picture: drawn here, or left to mermaid.js, or
    /// None to show it as code.
    fn diagram(&mut self, source: &str) -> Option<String> {
        let for_js = |me: &mut Self| {
            me.art.mermaid_js = true;
            let id = me.block_id("diagram");
            Some(format!(
                "<div class=\"diagram-js\" id=\"{id}\"><pre class=\"mermaid\">{}</pre></div>",
                esc(source)
            ))
        };
        match self.inp.mermaid {
            Diagrams::Code => None,
            Diagrams::Js => for_js(self),
            Diagrams::Native => {
                self.diagrams += 1;
                match crate::mermaid::render(source, self.diagrams) {
                    crate::mermaid::Outcome::Drawn(svg) => {
                        let id = self.block_id("diagram");
                        Some(svg.replacen(
                            "<figure class=\"diagram",
                            &format!("<figure id=\"{id}\" class=\"diagram"),
                            1,
                        ))
                    }
                    _ if self.inp.mermaid_js => for_js(self),
                    crate::mermaid::Outcome::Unsupported(kind) => {
                        self.warn(format!(
                            "a Mermaid `{kind}` diagram is shown as code: md2html draws flowcharts and sequence diagrams, and --mermaid-js FILE draws the rest"
                        ));
                        None
                    }
                    crate::mermaid::Outcome::Unreadable(why) => {
                        self.warn(format!(
                            "a Mermaid diagram could not be read ({why}); it is shown as code"
                        ));
                        None
                    }
                }
            }
        }
    }

    /// Raw HTML: as written when trusted; otherwise only the safe tags pass,
    /// and a block holding any other is shown as text.
    fn html_block(&mut self, text: &str) -> String {
        match self.inp.html {
            HtmlMode::Spec => format!("{text}\n"),
            HtmlMode::Rich => format!("{}\n", crate::sanitize::rich_block(text)),
            HtmlMode::Safe => {
                let t = text.trim();
                if t.starts_with("<!--") && t.ends_with("-->") && t.matches("-->").count() == 1 {
                    return String::new();
                }
                if let Some(clean) = sanitize(text) {
                    clean
                } else {
                    self.warn("raw HTML is shown as written; render with --html to let it through");
                    format!("<pre class=\"verbatim\">{}</pre>", esc(text))
                }
            }
        }
    }

    /// A block quote: a callout when its first words name one, a quote
    /// otherwise.
    fn quote(&mut self, blocks: &[Block]) -> String {
        if let Some(Block::Paragraph { text, line }) = blocks.first() {
            if let Some((kind, rest)) = callout_kind(text) {
                let mut inner = String::new();
                if !rest.trim().is_empty() {
                    let first = Block::Paragraph {
                        text: rest,
                        line: *line,
                    };
                    inner.push_str(&self.block(&first, false));
                }
                for block in blocks.iter().skip(1) {
                    inner.push_str(&self.block(block, false));
                }
                return format!(
                    "<div class=\"callout {}\" role=\"note\"><span class=\"mark\">{}</span><div class=\"body\">{inner}</div></div>",
                    KINDS[kind],
                    esc(self.inp.ui.callouts[kind])
                );
            }
        }
        let mut inner = String::new();
        for block in blocks {
            inner.push_str(&self.block(block, false));
        }
        // A quote that opens with a bold lead ("**For Esther.** ...") is an
        // aside the author set apart, as every quote was on md2html's first
        // pages: a note, its lead kept.
        let bold_lead = matches!(blocks.first(), Some(Block::Paragraph { text, .. })
            if text.starts_with("**") && text[2..].find("**").is_some_and(|end| end > 0 && end <= 80));
        if bold_lead {
            return format!(
                "<div class=\"callout note\" role=\"note\"><span class=\"mark\">{}</span><div class=\"body\">{inner}</div></div>",
                esc(self.inp.ui.callouts[0])
            );
        }
        format!("<blockquote>{inner}</blockquote>")
    }

    /// A list; a tight one's paragraphs are not wrapped.
    fn list(&mut self, ordered: bool, start: u64, tight: bool, items: &[Item]) -> String {
        let tasks = items.iter().any(|item| item.task.is_some());
        let mut out = if ordered {
            if start == 1 {
                String::from("<ol>")
            } else {
                format!("<ol start=\"{start}\">")
            }
        } else if tasks {
            String::from("<ul class=\"tasks\">")
        } else {
            String::from("<ul>")
        };
        for item in items {
            match item.task {
                Some(done) => {
                    let _ = write!(
                        out,
                        "<li class=\"task\"><input type=\"checkbox\" disabled{} aria-label=\"{}\"> ",
                        if done { " checked" } else { "" },
                        esc(self.inp.ui.task[usize::from(!done)])
                    );
                }
                None => out.push_str("<li>"),
            }
            for (at, block) in item.blocks.iter().enumerate() {
                if at > 0 && tight {
                    out.push('\n');
                }
                out.push_str(&self.block(block, tight));
            }
            out.push_str("</li>");
        }
        out.push_str(if ordered { "</ol>" } else { "</ul>" });
        out
    }

    /// A table in its scrolling frame.
    fn table(&mut self, align: &[Align], head: &[String], rows: &[Vec<String>]) -> String {
        let class = |at: usize| match align.get(at) {
            Some(Align::Center) => " class=\"c\"",
            Some(Align::Right) => " class=\"r\"",
            _ => "",
        };
        let mut out = String::from("<div class=\"scroll\"><table><thead><tr>");
        for (at, cell) in head.iter().enumerate() {
            let nodes = self.parse(cell);
            let html = self.inline(&nodes, true);
            if html.is_empty() {
                out.push_str("<td></td>");
            } else {
                let _ = write!(out, "<th scope=\"col\"{}>{html}</th>", class(at));
            }
        }
        out.push_str("</tr></thead><tbody>");
        // Each row has an address of its own, for a link to exactly it: its
        // section and its first cell (`7-settings-lang`), so it holds when
        // rows or tables are added elsewhere.
        let section = self
            .art
            .sections
            .last()
            .map_or_else(|| String::from("row"), |s| s.id.clone());
        for (n, row) in rows.iter().enumerate() {
            let first = row
                .first()
                .map(|cell| crate::inline::plain(&self.parse(cell)))
                .unwrap_or_default();
            let mut key: String = slug(&first).chars().take(40).collect();
            if key.is_empty() {
                key = format!("r{}", n + 1);
            }
            let id = self.unique(&format!("{section}-{key}"));
            let _ = write!(out, "<tr id=\"{}\">", esc(&id));
            for (at, cell) in row.iter().enumerate() {
                let nodes = self.parse(cell);
                let html = self.inline(&nodes, true);
                let _ = write!(out, "<td{}>{html}</td>", class(at));
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table></div>");
        out
    }
}

/// Maths as `MathML`, or the TeX itself when this tool cannot read it.
fn math_html(tex: &str, display: bool) -> String {
    match crate::math::mathml(tex, display) {
        Some(mathml) => mathml,
        None if display => format!("<pre class=\"tex\">{}</pre>", esc(tex)),
        None => format!("<code class=\"tex\">{}</code>", esc(tex)),
    }
}

/// HTML with only the safe tags, or None if it holds any other.
fn sanitize(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if rest.starts_with('<') {
            if let Some(len) = html_tag(rest) {
                let tag = &rest[..len];
                if !tag.starts_with("<!--") {
                    out.push_str(&safe_tag(tag)?);
                }
                i += len;
                continue;
            }
            out.push_str("&lt;");
            i += 1;
            continue;
        }
        let ch = rest.chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    Some(out)
}

/// A callout's kind from its opening: GitHub's `[!NOTE]`, or a bold or
/// colon-ended word (`**Note:**`, `**Tip.**`, `Warning:`); and the text
/// after it.
pub(crate) fn callout_kind(text: &str) -> Option<(usize, String)> {
    let t = text.trim_start();
    let kind_of = |word: &str| {
        let w = word.to_lowercase();
        CALLOUT_WORDS
            .iter()
            .position(|words| words.contains(&w.as_str()))
            .or_else(|| {
                // The callout words of every language md2html speaks.
                crate::config::all_ui()
                    .iter()
                    .find_map(|ui| ui.callouts.iter().position(|c| c.to_lowercase() == w))
            })
    };
    if let Some(rest) = t.strip_prefix("[!") {
        let close = rest.find(']')?;
        let kind = kind_of(&rest[..close])?;
        return Some((kind, rest[close + 1..].trim_start().to_owned()));
    }
    let (bold, body) = match t.strip_prefix("**") {
        Some(rest) => (true, rest),
        None => (false, t),
    };
    let word_len = body
        .chars()
        .take_while(|ch| ch.is_alphabetic())
        .map(char::len_utf8)
        .sum::<usize>();
    let kind = kind_of(&body[..word_len])?;
    let mut rest = &body[word_len..];
    if bold {
        rest = rest.strip_prefix([':', '.']).unwrap_or(rest);
        rest = rest.strip_prefix("**")?;
        rest = rest.strip_prefix([':', '.']).unwrap_or(rest);
    } else {
        rest = rest.strip_prefix(':')?;
    }
    Some((kind, rest.trim_start().to_owned()))
}

/// The image of a paragraph that holds nothing else.
fn lone_image(nodes: &[Node]) -> Option<&Node> {
    let mut found = None;
    for node in nodes {
        match node {
            Node::Image { .. } if found.is_none() => found = Some(node),
            Node::Text(text) if text.trim().is_empty() => {}
            Node::SoftBreak => {}
            _ => return None,
        }
    }
    found
}

/// An HTML comment block, which the page drops.
fn is_comment(block: &Block) -> bool {
    matches!(block, Block::Html { text, .. } if text.trim_start().starts_with("<!--") && text.trim_end().ends_with("-->"))
}

/// The line a block starts on.
fn block_line(block: &Block) -> Option<usize> {
    match block {
        Block::Heading { line, .. }
        | Block::Paragraph { line, .. }
        | Block::Code { line, .. }
        | Block::Html { line, .. }
        | Block::Quote { line, .. }
        | Block::Table { line, .. }
        | Block::Math { line, .. } => Some(*line),
        _ => None,
    }
}

/// A chapter that is the document's own table of contents: the outline
/// already is one.
fn is_contents(text: &str) -> bool {
    let t = bare_heading(text);
    matches!(
        t.as_str(),
        "contents" | "table of contents" | "inhalt" | "inhaltsverzeichnis"
    ) || crate::config::all_ui()
        .iter()
        .any(|ui| ui.contents_heading.to_lowercase() == t)
}

/// A heading's words for comparison: number dropped ("3. "), lowercased.
fn bare_heading(text: &str) -> String {
    let t = text.trim();
    let t = match t.split_once(". ") {
        Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) => rest,
        _ => t,
    };
    t.trim().to_lowercase()
}

/// The reading guide's chapter.
pub(crate) fn is_guide(text: &str) -> bool {
    let t = bare_heading(text);
    matches!(
        t.as_str(),
        "how to read this" | "so liest man diese seite" | "wie man dies liest"
    ) || crate::config::all_ui()
        .iter()
        .any(|ui| ui.guide_heading.to_lowercase() == t)
}

/// `%20` and the like back to characters, for a file name on disk.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(byte) = text
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A file as a `data:` URI, its type from its extension.
fn data_uri(path: &Path, bases: &[&Path], max: u64) -> Result<String, String> {
    if !crate::text::inside(path, bases) {
        return Err(String::from(
            "it is outside the page's folder and the folder md2html runs in",
        ));
    }
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > max {
        return Err(format!(
            "it is larger than {} MB (--max-embed raises the limit)",
            max >> 20
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // The bytes say what the file is, not its name: only an image is embedded.
    let mime = image_kind(&bytes).ok_or("its contents are not an image")?;
    Ok(format!(
        "data:{mime};base64,{}",
        crate::text::base64(&bytes)
    ))
}

/// The largest image put inside a page unless `--max-embed` says more.
pub(crate) const MAX_EMBED: u64 = 32 << 20;

/// An image's media type from its first bytes.
pub(crate) fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    let head = &bytes[..bytes.len().min(512)];
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if head.starts_with(b"\xFF\xD8\xFF") {
        Some("image/jpeg")
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else if head.len() >= 12
        && &head[4..8] == b"ftyp"
        && (&head[8..12] == b"avif" || &head[8..12] == b"avis")
    {
        Some("image/avif")
    } else if head.starts_with(b"\0\0\x01\0") {
        Some("image/x-icon")
    } else if head.starts_with(b"BM") {
        Some("image/bmp")
    } else {
        let text = String::from_utf8_lossy(head);
        let t = text.trim_start_matches('\u{feff}').trim_start();
        (t.starts_with("<svg")
            || (t.starts_with("<?xml") || t.starts_with("<!--")) && text.contains("<svg"))
        .then_some("image/svg+xml")
    }
}

/// Inline nodes as HTML with nothing of the page's around them: no glossary,
/// no footnote numbers. A glossary entry's meaning, a footer.
pub(crate) fn bare_inline(nodes: &[Node]) -> String {
    let mut out = String::new();
    plain_inline(nodes, &mut out, false);
    out
}

/// The `CommonMark` rendering of inline nodes.
fn plain_inline(nodes: &[Node], out: &mut String, in_link: bool) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(&esc(text)),
            Node::Code(code) => {
                let _ = write!(out, "<code>{}</code>", esc(code));
            }
            Node::Html(html) => out.push_str(html),
            Node::SoftBreak => out.push('\n'),
            Node::HardBreak => out.push_str("<br />\n"),
            Node::Em(kids) => {
                out.push_str("<em>");
                plain_inline(kids, out, in_link);
                out.push_str("</em>");
            }
            Node::Strong(kids) => {
                out.push_str("<strong>");
                plain_inline(kids, out, in_link);
                out.push_str("</strong>");
            }
            Node::Del(kids) => {
                out.push_str("<del>");
                plain_inline(kids, out, in_link);
                out.push_str("</del>");
            }
            Node::Link { href, title, kids } => {
                if in_link || !safe_url(href, false) {
                    plain_inline(kids, out, in_link);
                    continue;
                }
                let title = title
                    .as_ref()
                    .map(|t| format!(" title=\"{}\"", esc(t)))
                    .unwrap_or_default();
                let _ = write!(out, "<a href=\"{}\"{title}>", url_attr(href));
                plain_inline(kids, out, true);
                out.push_str("</a>");
            }
            Node::Image {
                src, title, alt, ..
            } => {
                let alt = esc(&crate::inline::plain(alt));
                if !safe_url(src, true) {
                    out.push_str(&alt);
                    continue;
                }
                let title = title
                    .as_ref()
                    .map(|t| format!(" title=\"{}\"", esc(t)))
                    .unwrap_or_default();
                let _ = write!(
                    out,
                    "<img src=\"{}\" alt=\"{alt}\"{title} />",
                    url_attr(src)
                );
            }
            Node::Note(label) => {
                let _ = write!(
                    out,
                    "<sup class=\"fn\"><a href=\"#fn-{}\">{}</a></sup>",
                    esc(label),
                    esc(label)
                );
            }
            Node::Math { tex, display } => out.push_str(&math_html(tex, *display)),
        }
    }
}

/// Blocks as plain HTML: what `CommonMark`'s tests expect, and what a page
/// of someone else's takes (`--fragment`).
pub(crate) fn fragment(blocks: &[Block], defs: &Defs, html: HtmlMode, math: bool) -> String {
    let mut out = String::new();
    let env = Env { defs, html, math };
    for block in blocks {
        fragment_block(block, &env, false, &mut out);
    }
    if !defs.notes.is_empty() && html != HtmlMode::Spec {
        out.push_str("<section class=\"footnotes\"><ol>\n");
        for (label, blocks) in &defs.notes {
            let _ = writeln!(out, "<li id=\"fn-{}\">", esc(label));
            for block in blocks {
                fragment_block(block, &env, false, &mut out);
            }
            out.push_str("</li>\n");
        }
        out.push_str("</ol></section>\n");
    }
    out
}

fn fragment_block(block: &Block, env: &Env<'_>, tight: bool, out: &mut String) {
    match block {
        Block::Paragraph { text, .. } => {
            let nodes = crate::inline::parse(text, env);
            if tight {
                plain_inline(&nodes, out, false);
            } else {
                out.push_str("<p>");
                plain_inline(&nodes, out, false);
                out.push_str("</p>\n");
            }
        }
        Block::Heading { level, text, .. } => {
            let _ = write!(out, "<h{level}>");
            plain_inline(&crate::inline::parse(text, env), out, false);
            let _ = writeln!(out, "</h{level}>");
        }
        Block::Code { info, text, .. } => {
            let lang = info.split_whitespace().next().unwrap_or_default();
            if lang.is_empty() {
                out.push_str("<pre><code>");
            } else {
                let _ = write!(out, "<pre><code class=\"language-{}\">", esc(lang));
            }
            out.push_str(&esc(text));
            out.push_str("</code></pre>\n");
        }
        Block::Html { text, .. } => {
            if env.html == HtmlMode::Safe {
                match sanitize(text) {
                    Some(clean) => {
                        out.push_str(&clean);
                        out.push('\n');
                    }
                    None => {
                        let _ = writeln!(out, "<pre>{}</pre>", esc(text));
                    }
                }
            } else {
                out.push_str(text);
                out.push('\n');
            }
        }
        Block::Rule => out.push_str("<hr />\n"),
        Block::Quote { blocks, .. } => {
            out.push_str("<blockquote>\n");
            for b in blocks {
                fragment_block(b, env, false, out);
            }
            out.push_str("</blockquote>\n");
        }
        Block::List {
            ordered,
            start,
            tight,
            items,
        } => {
            if *ordered {
                if *start == 1 {
                    out.push_str("<ol>\n");
                } else {
                    let _ = writeln!(out, "<ol start=\"{start}\">");
                }
            } else {
                out.push_str("<ul>\n");
            }
            for item in items {
                out.push_str("<li>");
                if let Some(done) = item.task {
                    let _ = write!(
                        out,
                        "<input type=\"checkbox\" disabled=\"\"{} /> ",
                        if done { " checked=\"\"" } else { "" }
                    );
                }
                let mut first = true;
                for b in &item.blocks {
                    let para = matches!(b, Block::Paragraph { .. });
                    if !(*tight && para) && first {
                        out.push('\n');
                    }
                    fragment_block(b, env, *tight, out);
                    if *tight && para && item.blocks.len() > 1 {
                        out.push('\n');
                    }
                    first = false;
                }
                out.push_str("</li>\n");
            }
            out.push_str(if *ordered { "</ol>\n" } else { "</ul>\n" });
        }
        Block::Table {
            align, head, rows, ..
        } => {
            let attr = |at: usize| match align.get(at) {
                Some(Align::Left) => " align=\"left\"",
                Some(Align::Center) => " align=\"center\"",
                Some(Align::Right) => " align=\"right\"",
                _ => "",
            };
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (at, cell) in head.iter().enumerate() {
                let _ = write!(out, "<th{}>", attr(at));
                plain_inline(&crate::inline::parse(cell, env), out, false);
                out.push_str("</th>\n");
            }
            out.push_str("</tr>\n</thead>\n");
            if !rows.is_empty() {
                out.push_str("<tbody>\n");
                for row in rows {
                    out.push_str("<tr>\n");
                    for (at, cell) in row.iter().enumerate() {
                        let _ = write!(out, "<td{}>", attr(at));
                        plain_inline(&crate::inline::parse(cell, env), out, false);
                        out.push_str("</td>\n");
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n");
            }
            out.push_str("</table>\n");
        }
        Block::Math { text, .. } => {
            out.push_str(&math_html(text, true));
            out.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::callout_kind;

    #[test]
    fn callouts_are_named_by_their_first_words() {
        assert_eq!(callout_kind("**Note:** read this").map(|k| k.0), Some(0));
        assert_eq!(callout_kind("**Note.** read this").map(|k| k.0), Some(0));
        assert_eq!(callout_kind("[!WARNING]\nCareful").map(|k| k.0), Some(3));
        assert_eq!(callout_kind("[!CAUTION]").map(|k| k.0), Some(4));
        assert_eq!(callout_kind("Tip: try it").map(|k| k.0), Some(1));
        assert_eq!(callout_kind("**Achtung:** heiß").map(|k| k.0), Some(3));
        assert_eq!(callout_kind("Note that this is a plain quote."), None);
        assert_eq!(callout_kind("Notebooks: plain"), None);
        assert_eq!(
            callout_kind("[!NOTE]\nThe text").map(|k| k.1),
            Some(String::from("The text"))
        );
    }
}
