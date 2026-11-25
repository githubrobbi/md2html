// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The translations of a handbook: each language is a handbook of its own.
//!
//! A reader moves, finds and looks words up among pages of one language.
//! A language's own start page (`README.de.md`) orders it when
//! it links its pages; else it follows the original's order, page for
//! page by name, and the pages no original stands for come after. A
//! language with one page is a page, not a handbook.

use std::path::{Path, PathBuf};

use super::order::{self, Listed, Placed, Start};
use super::{Book, PageNo, Surveys};
use crate::document::{original_of, translation_suffix};

/// The handbooks of the `translations` of a handbook in `folder`, whose
/// original has the `chapters` in its order.
pub(super) fn books(
    folder: &Path,
    translations: &[Listed],
    chapters: &[PageNo],
    surveys: &mut Surveys<'_>,
) -> Vec<Book> {
    let mut languages: Vec<String> = Vec::new();
    for listed in translations {
        if let Some(code) = translation_suffix(&listed.under) {
            if !languages.contains(&code) {
                languages.push(code);
            }
        }
    }
    let mut books = Vec::new();
    for code in languages {
        let group: Vec<Listed> = translations
            .iter()
            .filter(|listed| translation_suffix(&listed.under).as_deref() == Some(code.as_str()))
            .cloned()
            .collect();
        if group.len() < 2 {
            continue;
        }
        let placed = own_order(&group, surveys)
            .unwrap_or_else(|| mapped_order(folder, &group, chapters, surveys));
        books.push(Book::new(&placed, folder, surveys, |_, _| None));
    }
    books
}

/// The order a language's own start page gives, when it links two pages
/// or more of the language.
fn own_order(group: &[Listed], surveys: &mut Surveys<'_>) -> Option<Vec<Placed>> {
    // The start page is found by the original's name (`README.de.md` is a
    // `README.md`) and read from where it really is.
    let named: Vec<Listed> = group
        .iter()
        .map(|listed| Listed {
            under: original_of(&listed.under).unwrap_or_else(|| listed.under.clone()),
            page: listed.page,
        })
        .collect();
    let found = order::start(&named)?;
    let listed = group
        .iter()
        .find(|listed| listed.page == found.listed.page)?
        .clone();
    let start = Start {
        listed,
        kind: found.kind,
    };
    let chapters = order::order(&start, group, surveys).chapters;
    let linked = chapters
        .iter()
        .any(|placed| !placed.orphan && placed.page != start.listed.page);
    linked.then_some(chapters)
}

/// The original's order, page for page by name, then the language's pages
/// no original of the handbook stands for, in the walk's order.
fn mapped_order(
    folder: &Path,
    group: &[Listed],
    chapters: &[PageNo],
    surveys: &Surveys<'_>,
) -> Vec<Placed> {
    let file = |page: PageNo| -> Option<PathBuf> {
        surveys
            .pages
            .get(page.0)
            .and_then(|it| it.file)
            .map(Path::to_path_buf)
    };
    let mut placed: Vec<Placed> = Vec::new();
    for &chapter in chapters {
        let Some(original) = file(chapter) else {
            continue;
        };
        let translated = group
            .iter()
            .find(|listed| original_of(&folder.join(&listed.under)).as_ref() == Some(&original));
        if let Some(listed) = translated {
            placed.push(placement(listed.page));
        }
    }
    for listed in group {
        if !placed.iter().any(|place| place.page == listed.page) {
            placed.push(placement(listed.page));
        }
    }
    placed
}

/// A chapter at the top of its handbook, opening no part.
const fn placement(page: PageNo) -> Placed {
    Placed {
        page,
        depth: 0,
        part: None,
        orphan: false,
    }
}
