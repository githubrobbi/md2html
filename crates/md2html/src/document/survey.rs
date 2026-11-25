// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A first look at a page, before any page of the run is rendered.
//!
//! The title it will show, what its front matter says of handbooks, its
//! links in the order they stand, and the terms it lends as a glossary: a
//! handbook is put together from it.

use std::path::Path;

use super::glossary::handbook_terms;
use super::stem;
use crate::config::{Glance, Settings, front_matter};
use crate::glossary::{Term, is_glossary_document};
use crate::inline::{Env, HtmlMode};
use crate::model::{Block, Defs, Level, Node};
use crate::{block, inline};

/// A page, read but not rendered.
pub(crate) struct Survey {
    /// What its settings say of it.
    glance: Glance,
    /// Its blocks.
    blocks: Vec<Block>,
    /// Its link references and footnotes.
    defs: Defs,
    /// Its file's name without the extension: the title of a page without
    /// one.
    stem: String,
}

/// What a page holds that orders a handbook, in the order it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// A heading, as plain text.
    Heading {
        /// Its level.
        level: Level,
        /// Its words.
        text: String,
    },
    /// A link (not an image), wherever it stands but in code.
    Link {
        /// Where it leads, as written.
        href: String,
        /// How many lists it stands in: 0 outside any.
        depth: u8,
    },
}

impl Survey {
    /// The page `text` of `source`, with the command line's settings
    /// `cli` over its front matter.
    #[must_use]
    pub(crate) fn of(text: &str, source: Option<&Path>, cli: &Settings) -> Self {
        let front = front_matter(text);
        let glance = Glance::of(&front, cli);
        let math = cli.math.unwrap_or(true);
        let (blocks, defs) =
            block::parse(front.body, front.first_line, math, block::Reading::Author);
        Self {
            glance,
            blocks,
            defs,
            stem: stem(source),
        }
    }

    /// The title the page shows: its setting, else its first heading of
    /// level one, else its file's name.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        if let Some(title) = &self.glance.title {
            return title.clone();
        }
        let first = self.blocks.iter().find_map(|block| match block {
            Block::Heading(heading) if heading.level == Level::ONE => Some(heading),
            Block::Heading(_)
            | Block::Paragraph(_)
            | Block::Code(_)
            | Block::Html(_)
            | Block::Rule
            | Block::Quote(_)
            | Block::List(_)
            | Block::Table(_)
            | Block::Math(_) => None,
        });
        first.map_or_else(|| self.stem.clone(), |heading| self.plain(&heading.text))
    }

    /// What the page's settings say of handbooks.
    #[must_use]
    pub(crate) const fn glance(&self) -> &Glance {
        &self.glance
    }

    /// Whether the page is a glossary as a whole: titled Glossary.
    #[must_use]
    pub(crate) fn is_glossary(&self) -> bool {
        is_glossary_document(&self.blocks)
    }

    /// The terms the page defines, lent to the other chapters of its
    /// handbook; `up` leads from the chapter that shows them to this
    /// page's folder.
    #[must_use]
    pub(crate) fn lent_terms(&self, up: &str) -> Vec<Term> {
        handbook_terms(&self.blocks, &self.defs, up)
    }

    /// Its headings and links in the order they stand; a link in code, in
    /// raw HTML or in maths is no link.
    #[must_use]
    pub(crate) fn marks(&self) -> Vec<Mark> {
        let mut out = Vec::new();
        self.walk(&self.blocks, 0, &mut out);
        out
    }

    /// The marks of `blocks`, `depth` lists deep.
    fn walk(&self, blocks: &[Block], depth: u8, out: &mut Vec<Mark>) {
        for block in blocks {
            match block {
                Block::Heading(heading) => {
                    let nodes = self.inline(&heading.text);
                    out.push(Mark::Heading {
                        level: heading.level,
                        text: inline::plain(&nodes),
                    });
                    links(&nodes, depth, out);
                }
                Block::Paragraph(paragraph) => links(&self.inline(&paragraph.text), depth, out),
                Block::Quote(quote) => self.walk(&quote.blocks, depth, out),
                Block::List(list) => {
                    for item in &list.items {
                        self.walk(&item.blocks, depth.saturating_add(1), out);
                    }
                }
                Block::Table(table) => {
                    for cell in table.head.iter().chain(table.rows.iter().flatten()) {
                        links(&self.inline(cell), depth, out);
                    }
                }
                Block::Code(_) | Block::Html(_) | Block::Rule | Block::Math(_) => {}
            }
        }
    }

    /// Inline Markdown read as the page reads it, raw HTML as text.
    fn inline(&self, text: &str) -> Vec<Node> {
        let env = Env {
            defs: &self.defs,
            html: HtmlMode::Safe,
            math: false,
        };
        inline::parse(text, &env)
    }

    /// Inline Markdown as plain text.
    fn plain(&self, text: &str) -> String {
        inline::plain(&self.inline(text))
    }
}

/// The links among `nodes`, in order, each `depth` lists deep.
fn links(nodes: &[Node], depth: u8, out: &mut Vec<Mark>) {
    for node in nodes {
        match node {
            Node::Link(link) => out.push(Mark::Link {
                href: link.href.clone(),
                depth,
            }),
            Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => links(kids, depth, out),
            Node::Text(_)
            | Node::Code(_)
            | Node::Html(_)
            | Node::Undefined(_)
            | Node::HardBreak
            | Node::Image(_)
            | Node::Note(_)
            | Node::Math(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Mark, Survey};
    use crate::config::Settings;
    use crate::model::Level;

    /// The links of a page, by where they lead.
    fn hrefs(text: &str) -> Vec<String> {
        Survey::of(text, None, &Settings::default())
            .marks()
            .into_iter()
            .filter_map(|mark| match mark {
                Mark::Link { href, depth: _ } => Some(href),
                Mark::Heading { level: _, text: _ } => None,
            })
            .collect()
    }

    /// Every kind of link counts in the order it stands, an image and a
    /// link in code do not.
    ///
    /// # Panics
    ///
    /// When a link is missed or out of order, or an image, a code span, a
    /// code block or raw HTML gives one.
    #[test]
    fn links_are_read_from_the_tree_in_order() {
        let text = "# Book\n\n1. [One](one.md)\n2. <https://x.test/two.md>\n\n| A | B |\n|---|---|\n| [three][3] | x |\n\n\
> [four](four.md)\n\n`[no](code.md)` ![no](image.md)\n\n```\n[no](block.md)\n```\n\n<a href=\"html.md\">no</a>\n\n\
[3]: three.md\n";
        assert_eq!(hrefs(text), [
            "one.md",
            "https://x.test/two.md",
            "three.md",
            "four.md"
        ]);
    }

    /// The title is the setting, else the first heading of level one as
    /// plain text, else the file's name; a list's depth is counted.
    ///
    /// # Panics
    ///
    /// When a title is another, or the depth of a nested item's link is
    /// not 2.
    #[test]
    fn a_title_and_the_depth_of_a_link() {
        let plain = Survey::of("## Sub\n\n# The *Book*\n", None, &Settings::default());
        assert_eq!(plain.title(), "The Book");
        let set = Survey::of(
            "---\ntitle: Set\n---\n# Heading\n",
            None,
            &Settings::default(),
        );
        assert_eq!(set.title(), "Set");
        let none = Survey::of(
            "text\n",
            Some(std::path::Path::new("a/notes.md")),
            &Settings::default(),
        );
        assert_eq!(none.title(), "notes");
        let nested = Survey::of("- [a](a.md)\n  - [b](b.md)\n", None, &Settings::default());
        assert_eq!(nested.marks(), [
            Mark::Link {
                href: String::from("a.md"),
                depth: 1
            },
            Mark::Link {
                href: String::from("b.md"),
                depth: 2
            }
        ]);
        let headed = Survey::of("# Part `one`\n", None, &Settings::default());
        assert_eq!(headed.marks(), [Mark::Heading {
            level: Level::ONE,
            text: String::from("Part one")
        }]);
    }
}
