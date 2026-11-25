// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The walk over the article: the title and its lead in the hero, the
//! chapters and their outline, the chapters the page shows elsewhere
//! (contents, glossary) left out, the reading guide as cards.

use super::{Renderer, Section};
use crate::glossary::{is_glossary_document, is_glossary_heading};
use crate::model::{Block, Heading, HeadingStyle, Item, Level, Node, Spacing};
use crate::sanitize::Own;
use crate::text::{esc, plain, put};

/// A block of the article, with the anchor planned for it if it is a
/// heading.
enum Planned<'blocks> {
    /// A heading and its anchor.
    Heading(&'blocks Heading, String),
    /// Any other block.
    Other(&'blocks Block),
}

/// A chapter of the reading guide that a card points at: `## 3. Tables`.
struct Numbered {
    /// The number the heading starts with.
    number: u32,
    /// The heading's anchor.
    id: String,
}

/// What the chapter being walked is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Chapter {
    /// A chapter like any other.
    Plain,
    /// The document's own contents or glossary: the page has both already.
    Skipped,
    /// The reading guide: its first list becomes cards.
    Guide,
}

impl Chapter {
    /// The kind of chapter a level-two heading's words open.
    fn of(shown: &str) -> Self {
        if is_glossary_heading(shown) || is_contents(shown) {
            Self::Skipped
        } else if is_guide(shown) {
            Self::Guide
        } else {
            Self::Plain
        }
    }
}

/// Where the walk is.
struct Walk {
    /// Levels every heading moves down, to make room for the title.
    shift: u8,
    /// The hero is still open: the next paragraph may be its lead.
    hero_open: bool,
    /// The chapter being walked.
    chapter: Chapter,
}

impl Renderer<'_, '_> {
    /// Renders the whole article.
    pub(super) fn run(&mut self, blocks: &[Block]) {
        if is_glossary_document(blocks) {
            let terms = self.inp.terms;
            self.entries = terms
                .iter()
                .filter(|term| !term.shared)
                .map(|term| (term.word.as_str(), term))
                .collect();
        }
        let (title, body) = split_title(blocks);
        let chapters = body
            .clone()
            .filter(|block| matches!(block, Block::Heading(heading) if heading.level == Level::ONE))
            .count();
        // The first heading is the title; any other level-one heading is a
        // chapter, and every heading moves down one level to make room.
        let shift = u8::from(chapters > 0);
        // The anchors of the chapters first, so the reading guide can point
        // at a chapter further down.
        let (planned, numbered) = self.plan(body, shift);
        let top = self.balance.enter(Own::ARTICLE);
        self.hero(title);
        let mut walk = Walk {
            shift,
            hero_open: true,
            chapter: Chapter::Plain,
        };
        for part in planned {
            match part {
                Planned::Heading(heading, id) => self.walk_heading(heading, &id, &mut walk),
                Planned::Other(block) => self.walk_block(block, &numbered, &mut walk),
            }
        }
        self.balance.leave(top, &mut self.art.out);
        self.close_hero(&mut walk);
        if self.sub_open {
            self.art.contents.push_str("</ol></li>");
            self.sub_open = false;
        }
    }

    /// Every block of the body with the anchors of its headings, and the
    /// numbered chapters among them.
    fn plan<'blocks>(
        &mut self,
        body: impl Iterator<Item = &'blocks Block>,
        shift: u8,
    ) -> (Vec<Planned<'blocks>>, Vec<Numbered>) {
        let mut numbered = Vec::new();
        let planned = body
            .map(|block| {
                let Block::Heading(heading) = block else {
                    return Planned::Other(block);
                };
                let nodes = self.parse(&heading.text);
                let shown = crate::inline::plain(&nodes);
                let id = self.heading_id(heading, &nodes);
                if heading.level.down(shift) == Level::TWO {
                    if let Some(number) = shown
                        .split_once('.')
                        .and_then(|(head, _)| head.trim().parse::<u32>().ok())
                    {
                        numbered.push(Numbered {
                            number,
                            id: id.clone(),
                        });
                    }
                }
                Planned::Heading(heading, id)
            })
            .collect();
        (planned, numbered)
    }

    /// Opens the hero: the eyebrow and the title, the first section.
    fn hero(&mut self, title: Option<&Heading>) {
        let title_html = match title {
            Some(heading) => {
                let nodes = self.parse(&heading.text);
                self.inline(&nodes, super::Links::Own, Own::heading(Level::ONE))
            }
            None => esc(&self.inp.title),
        };
        let title_plain = plain(&title_html);
        put!(
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
    }

    /// Closes the hero if it is still open.
    fn close_hero(&mut self, walk: &mut Walk) {
        if walk.hero_open {
            self.art.out.push_str("</header>");
            walk.hero_open = false;
        }
    }

    /// A heading of the body: a new chapter, unless the chapter is skipped.
    fn walk_heading(&mut self, heading: &Heading, id: &str, walk: &mut Walk) {
        self.line = heading.line;
        let level = heading.level.down(walk.shift);
        let nodes = self.parse(&heading.text);
        let shown = crate::inline::plain(&nodes);
        if level == Level::TWO {
            walk.chapter = Chapter::of(&shown);
        } else if level < Level::TWO {
            walk.chapter = Chapter::Plain;
        }
        if heading.style == HeadingStyle::Setext
            && level == Level::TWO
            && shown.split_whitespace().count() > 3
        {
            self.warn("a line of --- under this text makes it a heading; leave a blank line above --- for a rule");
        }
        if walk.chapter == Chapter::Skipped {
            return;
        }
        self.close_hero(walk);
        self.heading(level, &nodes, &shown, id);
    }

    /// A block of the body other than a heading.
    fn walk_block(&mut self, block: &Block, numbered: &[Numbered], walk: &mut Walk) {
        if let Some(line) = block.line() {
            self.line = line;
        }
        if walk.chapter == Chapter::Skipped || is_colophon(block) || block.is_comment() {
            return;
        }
        if walk.hero_open {
            if let Block::Paragraph(paragraph) = block {
                let nodes = self.parse(&paragraph.text);
                if super::figure::lone_image(&nodes).is_none() {
                    self.lead(&nodes);
                    walk.hero_open = false;
                    return;
                }
            }
            self.close_hero(walk);
        }
        if walk.chapter == Chapter::Guide {
            if let Block::List(list) = block {
                self.balance.own_start("div", &mut self.art.out);
                let cards = self.cards(&list.items, numbered);
                self.art.out.push_str(&cards);
                walk.chapter = Chapter::Plain;
                return;
            }
        }
        let html = self.block(block, Spacing::Loose);
        self.remember(&html);
        self.art.out.push_str(&html);
    }

    /// The paragraph under the title, which closes the hero.
    fn lead(&mut self, nodes: &[Node]) {
        self.balance.own_start("p", &mut self.art.out);
        let lead = self.inline(nodes, super::Links::Glossary, Own::PARAGRAPH);
        self.remember(&lead);
        self.art.lead = plain(&lead);
        put!(self.art.out, "<p class=\"lead\">{lead}</p></header>");
    }

    /// A heading: into the article with its anchor, into the outline, and a
    /// new section for the search.
    fn heading(&mut self, level: Level, nodes: &[Node], shown: &str, id: &str) {
        let own = Own::heading(level);
        if let Some(name) = own.name() {
            self.balance.own_start(name, &mut self.art.out);
        }
        let html = self.inline(nodes, super::Links::Own, own);
        let text = esc(shown);
        if level == Level::TWO {
            self.linker.next_chapter();
            shown.clone_into(&mut self.chapter);
            if self.sub_open {
                self.art.contents.push_str("</ol></li>");
            }
            put!(
                self.art.contents,
                "<li><a href=\"#{id}\">{text}</a><ol class=\"sub\">"
            );
            self.sub_open = true;
        } else if level == Level::THREE {
            put!(self.art.contents, "<li><a href=\"#{id}\">{text}</a></li>");
        }
        if level <= Level::THREE {
            self.art.sections.push(Section {
                id: id.to_owned(),
                title: shown.to_owned(),
                chapter: self.chapter.clone(),
                text: String::new(),
            });
        }
        put!(
            self.art.out,
            "<h{level} id=\"{id}\">{html}<a class=\"anchor\" href=\"#{id}\" aria-label=\"{}\">{share}</a></h{level}>",
            esc(self.inp.ui.link_section),
            share = super::SHARE_ICON
        );
    }

    /// The reading guide's bullets as cards: the bold lead names the
    /// chapters, the rest says what they are for. No glossary links: the
    /// card is a link, and a link inside a link is not HTML.
    fn cards(&mut self, items: &[Item], numbered: &[Numbered]) -> String {
        let mut out = String::from("<div class=\"cards\">");
        for item in items {
            let raw = item
                .blocks
                .iter()
                .filter_map(super::blocks::paragraph)
                .map(|paragraph| paragraph.text.replace('\n', " "))
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
                .and_then(|number| numbered.iter().find(|chapter| chapter.number == number))
                .map(|chapter| chapter.id.as_str())
                .unwrap_or_default();
            if href.is_empty() && !lead.is_empty() {
                self.warn(format!(
                    "the reading-guide card \"{lead}\" names no numbered chapter"
                ));
            }
            let lead_nodes = self.parse(lead);
            let rest_nodes = self.parse(rest.trim());
            // The card is a link: the author's markup in it may not end
            // it, and md2html makes no link inside it.
            let card = self.balance.enter(Own::LINK);
            let lead_html = self.inline(&lead_nodes, super::Links::Own, Own::SPAN);
            let said = self.inline(&rest_nodes, super::Links::None, Own::SPAN);
            let mut closed = String::new();
            self.balance.leave(card, &mut closed);
            self.remember(&lead_html);
            self.remember(&said);
            put!(
                out,
                "<a class=\"card\" href=\"#{href}\"><span class=\"k\">{lead_html}</span><span>{said}</span>{closed}</a>"
            );
        }
        out.push_str("</div>");
        out
    }
}

/// The title, if the document opens with a level-one heading (comments
/// before it aside), and every other block.
fn split_title(blocks: &[Block]) -> (Option<&Heading>, impl Iterator<Item = &Block> + Clone) {
    let comments = blocks.iter().take_while(|block| block.is_comment()).count();
    let (before, from_title) = blocks.split_at_checked(comments).unwrap_or((blocks, &[]));
    match from_title {
        [Block::Heading(heading), after @ ..] if heading.level == Level::ONE => {
            (Some(heading), before.iter().chain(after))
        }
        _ => (None, blocks.iter().chain(&[])),
    }
}

/// A paragraph that is about the file, not part of the article: the
/// handbook's version line, a licence tag.
fn is_colophon(block: &Block) -> bool {
    matches!(block, Block::Paragraph(paragraph)
        if paragraph.text.starts_with("Version of this handbook") || paragraph.text.starts_with("SPDX-"))
}

/// A chapter that is the document's own table of contents: the outline
/// already is one.
fn is_contents(text: &str) -> bool {
    let words = bare_heading(text);
    matches!(
        words.as_str(),
        "contents" | "table of contents" | "inhalt" | "inhaltsverzeichnis"
    ) || crate::config::all_ui()
        .iter()
        .any(|ui| crate::config::lowercase_is(ui.contents_heading, &words))
}

/// A heading's words for comparison: number dropped ("3. "), lowercased.
fn bare_heading(text: &str) -> String {
    let trimmed = text.trim();
    let words = match trimmed.split_once(". ") {
        Some((number, rest)) if number.chars().all(|ch| ch.is_ascii_digit()) => rest,
        _ => trimmed,
    };
    words.trim().to_lowercase()
}

/// The reading guide's chapter.
fn is_guide(text: &str) -> bool {
    let words = bare_heading(text);
    matches!(
        words.as_str(),
        "how to read this" | "so liest man diese seite" | "wie man dies liest"
    ) || crate::config::all_ui()
        .iter()
        .any(|ui| crate::config::lowercase_is(ui.guide_heading, &words))
}
