// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's glossary: its own terms, and the shared glossary's that it
//! does not define itself.

use std::path::{Path, PathBuf};

use crate::block;
use crate::config::{Settings, front_matter};
use crate::glossary::{self, Term};
use crate::input::{MAX_BESIDE, read_file};
use crate::model::{Block, Defs};

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
    let own = glossary::terms(blocks, defs);
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
    let terms = glossary::terms(&blocks, &defs)
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
