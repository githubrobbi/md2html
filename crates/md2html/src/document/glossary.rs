// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's glossary: its own terms, and the shared glossary's that it
//! does not define itself.
//!
//! Both are read from a glossary chapter here, where the readers and the
//! renderer meet: each meaning is read as inline Markdown and written as
//! HTML.

use std::path::{Path, PathBuf};

use crate::config::{Settings, front_matter};
use crate::glossary::{Term, is_glossary_heading};
use crate::inline::{Env, HtmlMode};
use crate::input::{MAX_BESIDE, read_file};
use crate::model::{Block, Defs, Table};
use crate::text::slug;
use crate::{block, inline, render};

/// Where the shared glossary is: `MD2HTML_GLOSSARY`, else
/// `~/.config/md2html/glossary.md`.
pub(crate) fn shared_glossary_path() -> Option<PathBuf> {
    std::env::var_os("MD2HTML_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/md2html/glossary.md"))
        })
}

/// The page's terms, longest first, and where the full shared glossary
/// is when the page uses it.
pub(super) fn page_terms(
    blocks: &[Block],
    defs: &Defs,
    settings: &Settings,
    source: Option<&Path>,
) -> (Vec<Term>, Option<String>) {
    let own = chapter_terms(blocks, defs);
    let shared = if settings.glossary == Some(false) {
        None
    } else {
        source.and_then(shared_glossary)
    };
    match shared {
        Some((path, terms)) => (join(own, terms), Some(glossary_url(&path))),
        None => (join(own, Vec::new()), None),
    }
}

/// The shared glossary's terms, unless the page is the shared glossary.
fn shared_glossary(source: &Path) -> Option<(PathBuf, Vec<Term>)> {
    let path = shared_glossary_path()?;
    let text = read_file(&path, MAX_BESIDE).ok()?;
    let same = match (std::fs::canonicalize(&path), std::fs::canonicalize(source)) {
        (Ok(shared), Ok(own)) => shared == own,
        _ => false,
    };
    if same {
        return None;
    }
    let (_, body, first) = front_matter(&text);
    let (blocks, defs) = block::parse(body, first, false, false);
    let terms = chapter_terms(&blocks, &defs)
        .into_iter()
        .map(|term| Term {
            shared: true,
            ..term
        })
        .collect();
    Some((path, terms))
}

/// Where a page sends the reader for the full shared glossary.
fn glossary_url(path: &Path) -> String {
    if let Some(url) = std::env::var_os("MD2HTML_GLOSSARY_URL") {
        return url.to_string_lossy().into_owned();
    }
    let twin = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .with_extension("html");
    format!("file://{}", twin.display())
}

/// The page's own terms, then the shared ones it does not define itself,
/// longest first as the matcher wants them.
fn join(own: Vec<Term>, shared: Vec<Term>) -> Vec<Term> {
    let mut terms = own;
    for term in shared {
        if !terms
            .iter()
            .any(|have| have.word.eq_ignore_ascii_case(&term.word))
        {
            terms.push(term);
        }
    }
    terms.sort_by_key(|term| core::cmp::Reverse(term.word.len()));
    terms
}

/// The words a glossary table's "Also" column is headed with, in the
/// languages md2html speaks.
const ALSO_HEADS: [&str; 19] = [
    "also",
    "alias",
    "auch",
    "ook",
    "juga",
    "uga",
    "aussi",
    "anche",
    "también",
    "別名",
    "べつめい",
    "betsumei",
    "又称",
    "又稱",
    "亦稱",
    "亦称",
    "tambien",
    "גם",
    "אויך",
];

/// Whether a table's header cell names the "Also" column.
fn is_also_head(cell: &str) -> bool {
    let lower = cell.to_lowercase();
    ALSO_HEADS.iter().any(|head| lower.contains(head))
}

/// The terms a document's glossary chapter defines, longest first so
/// "content object" wins over "content".
fn chapter_terms(blocks: &[Block], defs: &Defs) -> Vec<Term> {
    let mut out = Vec::new();
    let mut inside = false;
    for block in blocks {
        match block {
            Block::Heading(heading) => {
                if heading.level <= 2 {
                    inside = heading.level == 2 && is_glossary_heading(&heading.text);
                }
            }
            Block::Table(table) if inside => out.extend(table_terms(table, defs)),
            Block::Paragraph(paragraph) if inside => {
                out.extend(bold_lead(&paragraph.text).map(|lead| lead.finish(Vec::new(), defs)));
            }
            Block::List(list) if inside => {
                out.extend(list.items.iter().filter_map(|item| {
                    let Some(Block::Paragraph(paragraph)) = item.blocks.first() else {
                        return None;
                    };
                    bold_lead(&paragraph.text).map(|lead| lead.finish(Vec::new(), defs))
                }));
            }
            Block::Table(_)
            | Block::Paragraph(_)
            | Block::List(_)
            | Block::Code(_)
            | Block::Html(_)
            | Block::Rule
            | Block::Quote(_)
            | Block::Math(_) => {}
        }
    }
    out.sort_by_key(|term| core::cmp::Reverse(term.word.len()));
    out
}

/// The terms of a glossary table: the term in its first column, the
/// meaning in its second, other spellings in a column headed "Also".
fn table_terms(table: &Table, defs: &Defs) -> Vec<Term> {
    table
        .rows
        .iter()
        .filter_map(|row| {
            let [word_cell, said, ..] = row.as_slice() else {
                return None;
            };
            let word = word_cell.trim().trim_matches('*').trim_matches('`').trim();
            if word.is_empty() {
                return None;
            }
            let also = table
                .head
                .iter()
                .zip(row)
                .find(|(head, _)| is_also_head(head))
                .map(|(_, cell)| aliases(cell))
                .unwrap_or_default();
            Some(
                Lead {
                    word: word.to_owned(),
                    said: said.clone(),
                }
                .finish(also, defs),
            )
        })
        .collect()
}

/// The spellings an "Also" cell lists, split at commas and semicolons.
fn aliases(cell: &str) -> Vec<String> {
    cell.split([',', ';'])
        .map(|alias| alias.trim().trim_matches('`').to_owned())
        .filter(|alias| !alias.is_empty())
        .collect()
}

/// A glossary entry as written: the term and what it means, not rendered.
struct Lead {
    /// The term.
    word: String,
    /// Its meaning, Markdown.
    said: String,
}

impl Lead {
    /// The entry, its definition rendered and as plain text.
    fn finish(self, also: Vec<String>, defs: &Defs) -> Term {
        let env = Env {
            defs,
            html: HtmlMode::Safe,
            math: false,
        };
        let nodes = inline::parse(&self.said, &env);
        let anchor = slug(&self.word);
        Term {
            word: self.word,
            said: render::bare_inline(&nodes),
            plain: inline::plain(&nodes),
            slug: anchor,
            shared: false,
            also,
        }
    }
}

/// `**Term** - meaning`, `**Term**: meaning` or `**Term.** meaning`, the
/// meaning's line breaks made spaces.
fn bold_lead(text: &str) -> Option<Lead> {
    let rest = text.strip_prefix("**")?;
    let (bold, tail) = rest.split_once("**")?;
    let word = bold.trim().trim_end_matches([':', '.']).trim();
    if word.is_empty() {
        return None;
    }
    let said = tail
        .trim_start()
        .trim_start_matches([':', '-', '–', '—', '.'])
        .trim()
        .replace('\n', " ");
    Some(Lead {
        word: word.to_owned(),
        said,
    })
}
