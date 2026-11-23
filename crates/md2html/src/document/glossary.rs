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

use crate::config::{Settings, front_matter};
use crate::glossary::{Term, is_glossary_heading};
use crate::inline::{Env, HtmlMode};
use crate::input::{MAX_BESIDE, Unread, read_file};
use crate::model::{Block, Defs, Level, Table};
use crate::render::Warning;
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
        (_, Some(page), Some(path)) => shared_glossary(&path, page, warnings),
    };
    match shared {
        Some((path, terms)) => {
            let configured = std::env::var_os("MD2HTML_GLOSSARY_URL");
            let url = glossary_url(&path, configured, warnings);
            (join(own, terms), url)
        }
        None => (join(own, Vec::new()), None),
    }
}

/// The terms of the shared glossary at `path`, unless the page is the
/// shared glossary. None when there is no file there; when there is one
/// that cannot be read, `warnings` says why.
fn shared_glossary(
    path: &Path,
    source: &Path,
    warnings: &mut Vec<Warning>,
) -> Option<(PathBuf, Vec<Term>)> {
    let text = match read_file(path, MAX_BESIDE) {
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
    let same = match (std::fs::canonicalize(path), std::fs::canonicalize(source)) {
        (Ok(shared), Ok(own)) => shared == own,
        _ => false,
    };
    if same {
        return None;
    }
    let (_, body, first) = front_matter(&text);
    // A glossary's definitions are words: `$` in them is not maths.
    let math = false;
    let (blocks, defs) = block::parse(body, first, math, block::Reading::Author);
    let terms = chapter_terms(&blocks, &defs)
        .into_iter()
        .map(|term| Term {
            shared: true,
            ..term
        })
        .collect();
    Some((path.to_path_buf(), terms))
}

/// Where a page sends the reader for the full shared glossary at `path`.
///
/// That is `configured` (`MD2HTML_GLOSSARY_URL`), else the shared page's
/// own twin. A link is text: a value or a path that is not Unicode gives
/// none, and `warnings` says so.
fn glossary_url(
    path: &Path,
    configured: Option<OsString>,
    warnings: &mut Vec<Warning>,
) -> Option<String> {
    if let Some(value) = configured {
        match value.into_string() {
            Ok(url) => return Some(url),
            Err(bytes) => warnings.push(Warning {
                line: None,
                message: format!(
                    "MD2HTML_GLOSSARY_URL {} is not valid Unicode; the link goes to the shared glossary's own page",
                    Path::new(&bytes).display()
                ),
            }),
        }
    }
    let twin = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .with_extension("html");
    let url = twin.to_str().map(|text| format!("file://{text}"));
    if url.is_none() {
        warnings.push(Warning {
            line: None,
            message: format!(
                "the shared glossary's page {} has a path that is not valid Unicode, so no link can name it; MD2HTML_GLOSSARY_URL sets one",
                twin.display()
            ),
        });
    }
    url
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
                if heading.level <= Level::TWO {
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{glossary_url, shared_glossary};

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
    /// there but cannot be read is none, and the page says why.
    ///
    /// # Panics
    ///
    /// When a missing glossary gives terms or a warning, when a glossary
    /// that is not UTF-8 gives terms or no warning naming it, when a
    /// readable one gives no terms, or when the scratch folder cannot be
    /// written in or removed.
    #[test]
    fn a_shared_glossary_that_cannot_be_read_is_reported() {
        let dir = folder("unread");
        let page = dir.join("page.md");
        let mut warnings = Vec::new();
        let missing = shared_glossary(&dir.join("absent.md"), &page, &mut warnings);
        assert!(missing.is_none() && warnings.is_empty(), "{warnings:?}");
        let bad = dir.join("bad.md");
        std::fs::write(&bad, b"# Glossary\n\n**Term** \xff\n").unwrap();
        assert!(shared_glossary(&bad, &page, &mut warnings).is_none());
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
        let (_, terms) = shared_glossary(&good, &page, &mut warnings).unwrap();
        assert_eq!(terms.len(), 1, "the term is read");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The link to the full glossary is `MD2HTML_GLOSSARY_URL` when it is
    /// Unicode, else the shared page's twin, and the page is told when the
    /// variable cannot be used.
    ///
    /// # Panics
    ///
    /// When a Unicode value is not the link, or gives a warning; when a
    /// value that is not Unicode is the link, or gives no warning; or when
    /// the twin is not `file://` and the glossary's path with `.html`.
    #[test]
    fn the_glossary_link_is_text_or_none() {
        let path = Path::new("/nowhere/glossary.md");
        let mut warnings = Vec::new();
        let set = glossary_url(
            path,
            Some(OsString::from("https://x.test/g")),
            &mut warnings,
        );
        assert_eq!(set.as_deref(), Some("https://x.test/g"));
        assert!(warnings.is_empty(), "{warnings:?}");
        let twin = glossary_url(path, None, &mut warnings);
        assert_eq!(twin.as_deref(), Some("file:///nowhere/glossary.html"));
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt as _;
            let odd = OsString::from_vec(b"https://x.test/\xff".to_vec());
            let fallen = glossary_url(path, Some(odd), &mut warnings);
            assert_eq!(fallen.as_deref(), Some("file:///nowhere/glossary.html"));
            let [warning] = warnings.as_slice() else {
                panic!("one warning: {warnings:?}");
            };
            assert!(
                warning.message.starts_with(
                    "MD2HTML_GLOSSARY_URL https://x.test/\u{fffd} is not valid Unicode"
                ),
                "{}",
                warning.message
            );
        }
    }
}
