// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's glossary: its own terms, and the shared glossary's that it
//! does not define itself.
//!
//! Both are read from a glossary chapter here, where the readers and the
//! renderer meet: each meaning is read as inline Markdown and written as
//! HTML.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::config::{Settings, all_ui, front_matter, lowercase_is};
use crate::glossary::{Term, bold_word, is_glossary_document, is_glossary_heading, table_word};
use crate::inline::{Env, HtmlMode};
use crate::input::{MAX_BESIDE, Unread, is_markdown, read_named};
use crate::model::{Block, Defs, Level, Node, Table};
use crate::render::Warning;
use crate::sanitize::Own;
use crate::text::{is_relative, slug};
use crate::{block, inline, render, system};

/// Where the shared glossary is: `MD2HTML_GLOSSARY`, else
/// `.config/md2html/glossary.md` in the home folder ([`system::home`]).
pub(crate) fn shared_glossary_path() -> Option<PathBuf> {
    std::env::var_os("MD2HTML_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| system::home().map(|home| home.join(".config/md2html/glossary.md")))
}

/// The page's terms, longest first, and where the full shared glossary
/// is when the page uses it; a shared glossary that is there but cannot be
/// used goes to `warnings`.
pub(super) fn page_terms(
    blocks: &[Block],
    defs: &Defs,
    settings: &Settings,
    source: Option<&Path>,
    warnings: &mut Vec<Warning>,
) -> (Vec<Term>, Option<String>) {
    let own = chapter_terms(blocks, defs);
    let shared = match (settings.glossary, source, shared_glossary_path()) {
        (Some(false), _, _) | (_, None, _) | (_, _, None) => None,
        (_, Some(_), Some(path)) => shared_glossary(&path, warnings),
    };
    match shared {
        Some(terms) => {
            let configured = std::env::var_os("MD2HTML_GLOSSARY_URL");
            let url = glossary_url(configured, warnings);
            (join(own, terms), url)
        }
        None => (join(own, Vec::new()), None),
    }
}

/// The terms of the shared glossary at `path`.
///
/// None when there is no file there; when there is one that cannot be
/// read, `warnings` says why. The shared glossary's own page needs no
/// exception: each of its terms is its own as well, and a page's own term
/// wins ([`join`]).
fn shared_glossary(path: &Path, warnings: &mut Vec<Warning>) -> Option<Vec<Term>> {
    let text = match read_named(path, MAX_BESIDE) {
        Ok(text) => text,
        Err(Unread::Missing(_)) => return None,
        Err(unread) => {
            warnings.push(Warning {
                line: None,
                message: format!(
                    "the shared glossary {} cannot be read: {}; the page is made without it",
                    path.display(),
                    unread.describe("")
                ),
            });
            return None;
        }
    };
    let read = front_matter(&text);
    // A glossary's definitions are words: `$` in them is not maths.
    let math = false;
    let (blocks, defs) = block::parse(read.body, read.first_line, math, block::Reading::Author);
    let terms = chapter_terms(&blocks, &defs)
        .into_iter()
        .map(|term| Term {
            shared: true,
            ..term
        })
        .collect();
    Some(terms)
}

/// Where a page sends the reader for the full shared glossary: the
/// address `configured` (`MD2HTML_GLOSSARY_URL`) names, or nowhere.
///
/// Never the shared page's own file: its `file:` URL would carry the home
/// folder's path, and the name in it, into every page made with the
/// glossary, and lead a reader of a published page nowhere. A value that
/// is not Unicode gives no link either, and `warnings` says so.
fn glossary_url(configured: Option<OsString>, warnings: &mut Vec<Warning>) -> Option<String> {
    match configured?.into_string() {
        Ok(url) => Some(url),
        Err(bytes) => {
            warnings.push(Warning {
                line: None,
                message: format!(
                    "MD2HTML_GLOSSARY_URL {} is not valid Unicode; the page links to no full glossary",
                    Path::new(&bytes).display()
                ),
            });
            None
        }
    }
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

/// Whether a table's header cell names the meaning column.
///
/// The whole cell is `Meaning`, `Definition` or their words in a language
/// md2html speaks (each interface's `meaning_heads`). Without one, the
/// meaning is the second column, whatever it is headed.
fn is_meaning_head(cell: &str) -> bool {
    let word = table_word(cell).to_lowercase();
    all_ui().iter().any(|ui| {
        ui.meaning_heads
            .iter()
            .any(|head| lowercase_is(head, &word))
    })
}

/// Whether a table's header cell names the "Also" column.
fn is_also_head(cell: &str) -> bool {
    let lower = cell.to_lowercase();
    ALSO_HEADS.iter().any(|head| lower.contains(head))
}

/// The terms a document's glossary chapter defines, or the whole document
/// when it is a glossary ([`is_glossary_document`]), longest first so
/// "content object" wins over "content".
fn chapter_terms(blocks: &[Block], defs: &Defs) -> Vec<Term> {
    let mut out = Vec::new();
    // A glossary document is a glossary from its title to its end.
    let whole = is_glossary_document(blocks);
    let mut inside = whole;
    for block in blocks {
        match block {
            Block::Heading(heading) => {
                if !whole && heading.level <= Level::TWO {
                    inside = heading.level == Level::TWO && is_glossary_heading(&heading.text);
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

/// The terms of a glossary table.
///
/// The term in its first column, the meaning in the column headed for it
/// ([`is_meaning_head`]: `Meaning`, `Definition`...) or else the second,
/// other spellings in a column headed "Also".
fn table_terms(table: &Table, defs: &Defs) -> Vec<Term> {
    let meaning = table
        .head
        .iter()
        .skip(1)
        .position(|head| is_meaning_head(head))
        .map_or(1, |at| at.saturating_add(1));
    let also_column = table.head.iter().position(|head| is_also_head(head));
    table
        .rows
        .iter()
        .filter_map(|row| {
            let word = table_word(row.first()?);
            let said = row.get(meaning)?;
            if word.is_empty() {
                return None;
            }
            let also = also_column
                .and_then(|at| row.get(at))
                .map(|cell| aliases(cell))
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
        let mut nodes = inline::parse(&self.said, &env);
        to_pages(&mut nodes);
        let anchor = slug(&self.word);
        Term {
            word: self.word,
            said: render::bare_inline(&nodes, Own::DEFINITION),
            plain: inline::plain(&nodes),
            slug: anchor,
            shared: false,
            also,
        }
    }
}

/// A meaning's links to a Markdown file sent to the page md2html makes of
/// it, as a link in a page's own text is.
///
/// `concepts.md#1--size` is `concepts.html#1--size`, so a handbook's
/// glossary written for GitHub leads from every page to the pages, not to
/// their Markdown.
fn to_pages(nodes: &mut [Node]) {
    for node in nodes {
        match node {
            Node::Link(link) => {
                if let Some(page) = page_link(&link.href) {
                    link.href = page;
                }
                to_pages(&mut link.kids);
            }
            Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => to_pages(kids),
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

/// A relative link to a Markdown file as a link to its page; None for any
/// other link.
fn page_link(href: &str) -> Option<String> {
    if !is_relative(href) {
        return None;
    }
    let (path, rest) = href
        .find(['#', '?'])
        .and_then(|cut| href.split_at_checked(cut))
        .unwrap_or((href, ""));
    if !is_markdown(Path::new(path)) {
        return None;
    }
    let (stem, _) = path.rsplit_once('.')?;
    Some(format!("{stem}.html{rest}"))
}

/// `**Term** - meaning`, `**Term**: meaning` or `**Term.** meaning`, the
/// meaning's line breaks made spaces.
fn bold_lead(text: &str) -> Option<Lead> {
    let (word, tail) = bold_word(text)?;
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{glossary_url, shared_glossary};
    use crate::tests::not_unicode;

    /// A fresh, empty folder for one test.
    ///
    /// # Panics
    ///
    /// When the folder of an earlier run cannot be removed, or the new one
    /// cannot be created in the temporary directory.
    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("md2html-glossary-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A shared glossary that is not there is none, quietly; one that is
    /// there but cannot be read is none, and the page says why; a glossary
    /// file (`# Glossary` and a table) gives its table's terms.
    ///
    /// # Panics
    ///
    /// When a missing glossary gives terms or a warning, when a glossary
    /// that is not UTF-8 gives terms or no warning naming it, when a
    /// readable one gives no terms, when a glossary file's table does not
    /// give its two terms, shared, their links to Markdown pages leading to
    /// the pages, or when the scratch folder cannot be written in or
    /// removed.
    #[test]
    fn a_shared_glossary_that_cannot_be_read_is_reported() {
        let dir = folder("unread");
        let mut warnings = Vec::new();
        let missing = shared_glossary(&dir.join("absent.md"), &mut warnings);
        assert!(missing.is_none() && warnings.is_empty(), "{warnings:?}");
        let bad = dir.join("bad.md");
        std::fs::write(&bad, b"# Glossary\n\n**Term** \xff\n").unwrap();
        assert!(shared_glossary(&bad, &mut warnings).is_none());
        let [warning] = warnings.as_slice() else {
            panic!("one warning: {warnings:?}");
        };
        assert_eq!(
            warning.message,
            format!(
                "the shared glossary {} cannot be read: stream did not contain valid UTF-8; the page is made without it",
                bad.display()
            )
        );
        let good = dir.join("good.md");
        std::fs::write(&good, "# Words\n\n## Glossary\n\n**Term** - a word.\n").unwrap();
        let terms = shared_glossary(&good, &mut warnings).unwrap();
        assert_eq!(terms.len(), 1, "the term is read");
        let file = dir.join("glossary.md");
        std::fs::write(
            &file,
            "# Glossary\n\n| Term | Definition |\n|---|---|\n| **ADS** | A stream. See [Concepts](concepts.md#1--size). |\n| **FRS** | A record. |\n",
        )
        .unwrap();
        let filed = shared_glossary(&file, &mut warnings).unwrap();
        let words: Vec<&str> = filed.iter().map(|term| term.word.as_str()).collect();
        assert_eq!(words, ["ADS", "FRS"], "a glossary file's table is read");
        assert!(filed.iter().all(|term| term.shared), "its terms are shared");
        assert!(
            filed
                .first()
                .is_some_and(|term| term.said.contains("href=\"concepts.html#1--size\"")),
            "a meaning's link to a Markdown page leads to the page"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The link to the full glossary is `MD2HTML_GLOSSARY_URL` when it is
    /// Unicode, and there is none otherwise: never a `file:` URL of the
    /// shared page, which would put the home folder's path in the page.
    ///
    /// # Panics
    ///
    /// When a Unicode value is not the link, or gives a warning; when no
    /// value gives a link or a warning; or when a value that is not Unicode
    /// gives a link, or no warning that says the page links nowhere.
    #[test]
    fn the_glossary_link_is_the_url_or_none() {
        let mut warnings = Vec::new();
        let set = glossary_url(Some(OsString::from("https://x.test/g")), &mut warnings);
        assert_eq!(set.as_deref(), Some("https://x.test/g"));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(glossary_url(None, &mut warnings), None);
        assert!(warnings.is_empty(), "{warnings:?}");
        let odd = not_unicode("https://x.test/", "");
        assert_eq!(glossary_url(Some(odd), &mut warnings), None);
        let [warning] = warnings.as_slice() else {
            panic!("one warning: {warnings:?}");
        };
        assert_eq!(
            warning.message,
            "MD2HTML_GLOSSARY_URL https://x.test/\u{fffd} is not valid Unicode; the page links to no full glossary"
        );
    }
}
