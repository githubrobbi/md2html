// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Footnotes: numbered in the order they are first referenced, each
//! reference linked to its note and showing the note's words on hover,
//! the notes listed at the end of the article.

use alloc::collections::BTreeMap;

use super::{Article, Renderer, Section};
use crate::inline::Env;
use crate::model::{Defs, Spacing};
use crate::sanitize::Own;
use crate::text::{esc, plain, put};

/// The footnotes referenced so far.
pub(super) struct Footnotes {
    /// The labels in the order they were first referenced: a note's number
    /// is its place here, from one.
    order: Vec<String>,
    /// What each referenced label has had.
    cited: BTreeMap<String, Cited>,
    /// Each defined footnote as plain text, for its hover.
    hover: BTreeMap<String, String>,
}

/// A footnote that has been referenced.
#[derive(Clone, Copy)]
struct Cited {
    /// Its number.
    number: usize,
    /// The references to it so far.
    times: usize,
}

impl Footnotes {
    /// None referenced yet, the hover text of every defined one ready.
    pub(super) fn new(defs: &Defs, env: &Env<'_>) -> Self {
        let hover = defs
            .notes
            .iter()
            .map(|(label, blocks)| {
                let text = blocks
                    .iter()
                    .filter_map(super::blocks::paragraph)
                    .map(|paragraph| {
                        crate::inline::plain(&crate::inline::parse(&paragraph.text, env))
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                (label.clone(), text)
            })
            .collect();
        Self {
            order: Vec::new(),
            cited: BTreeMap::new(),
            hover,
        }
    }

    /// One more reference to `label`: the note's number, and which
    /// reference to it this is, both from one.
    fn cite(&mut self, label: &str) -> Cited {
        let next = self.order.len() + 1;
        let cited = self.cited.entry(label.to_owned()).or_insert(Cited {
            number: next,
            times: 0,
        });
        cited.times += 1;
        let now = *cited;
        if now.number == next {
            self.order.push(label.to_owned());
        }
        now
    }
}

impl Renderer<'_, '_> {
    /// A footnote reference: its number, linked to the note when it may
    /// be (`linked`: not inside the author's link), the note's words on
    /// hover.
    pub(super) fn note_ref(&mut self, label: &str, linked: bool) -> String {
        let Cited { number, times } = self.notes.cite(label);
        if !linked {
            // The note's link back still finds the reference.
            return format!("<sup class=\"fn\" id=\"fnref-{number}-{times}\">{number}</sup>");
        }
        let def = esc(self.notes.hover.get(label).map_or("", String::as_str));
        let aria = esc(&self.inp.ui.note_n.fill([&number.to_string()]));
        format!(
            "<sup class=\"fn\"><a class=\"fnref\" id=\"fnref-{number}-{times}\" href=\"#fn-{number}\" data-def=\"{def}\" aria-label=\"{aria}\">{number}</a></sup>"
        )
    }

    /// The footnotes, then the article as finished.
    pub(super) fn finish(mut self) -> Article {
        self.tell_mends();
        if !self.notes.order.is_empty() {
            let list = self.note_list();
            let title = esc(self.inp.ui.notes);
            put!(
                self.art.out,
                "<section class=\"notes\"><h2 id=\"notes\">{title}<a class=\"anchor\" href=\"#notes\" aria-label=\"{}\">{share}</a></h2><ol>{list}</ol></section>",
                esc(self.inp.ui.link_section),
                share = super::SHARE_ICON
            );
            put!(self.art.contents, "<li><a href=\"#notes\">{title}</a></li>");
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
            .filter(|label| !self.notes.cited.contains_key(*label))
            .cloned()
            .collect();
        for label in unused {
            self.line = 0;
            self.warn(format!(
                "the footnote [^{label}] is defined but never referenced"
            ));
        }
        self.art.minted = self.linker.into_minted();
        self.art
    }

    /// The notes' items, in the order of their numbers. A note may refer to
    /// a note not referenced before, which joins the end of the list: the
    /// list is written in rounds until a round adds none.
    fn note_list(&mut self) -> String {
        let mut list = String::new();
        let mut written = 0;
        loop {
            let round: Vec<String> = self.notes.order.iter().skip(written).cloned().collect();
            if round.is_empty() {
                return list;
            }
            for label in round {
                written += 1;
                self.note_item(&label, written, &mut list);
            }
        }
    }

    /// One note's item: its blocks, and a link back to its first reference.
    fn note_item(&mut self, label: &str, number: usize, list: &mut String) {
        let Some(blocks) = self.inp.defs.notes.get(label) else {
            return;
        };
        let mut body = String::new();
        self.scoped(Own::ITEM, &mut body, |this, inner| {
            for block in blocks {
                this.write_block(block, Spacing::Loose, inner);
            }
        });
        let back = format!(
            " <a class=\"back\" href=\"#fnref-{number}-1\" aria-label=\"{} {number}\">{}</a>",
            esc(self.inp.ui.back),
            esc(self.inp.ui.back)
        );
        // The link back goes inside the note's last paragraph.
        if let Some(open) = body.strip_suffix("</p>") {
            put!(list, "<li id=\"fn-{number}\">{open}{back}</p></li>");
        } else {
            put!(list, "<li id=\"fn-{number}\">{body}{back}</li>");
        }
    }
}
