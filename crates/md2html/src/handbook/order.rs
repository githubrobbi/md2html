// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A handbook's order, from its start page's links.
//!
//! Which page starts it (`SUMMARY.md`, else `README.md`, else `index.md`),
//! the chapters it links in the order they stand, the parts a subfolder's start
//! page opens, the folders the links enter, and the pages left over. Path
//! arithmetic over the run's pages: it reads no file.

use alloc::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use super::{PageNo, Surveys};
use crate::document::Mark;
use crate::text::{is_relative, percent_decode_path};

/// A page of a folder named on the command line, by its path under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Listed {
    /// Its path under the folder.
    pub(super) under: PathBuf,
    /// The page.
    pub(super) page: PageNo,
}

/// What kind of start page a handbook has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    /// mdBook's `SUMMARY.md`: it gives the order and is no chapter.
    Summary,
    /// `README.md`, which GitHub shows for the folder: the first chapter.
    Readme,
    /// `index.md`: the first chapter.
    Index,
}

impl Kind {
    /// The kind of start page a file's name makes, in any case, with any
    /// extension: `summary`, `readme` and `index`; `SUMMARY.md` only when
    /// `top`, at the top of the handbook.
    fn of(under: &Path, top: bool) -> Option<Self> {
        let stem = under.file_stem()?.to_str()?.to_ascii_lowercase();
        match stem.as_str() {
            "summary" if top => Some(Self::Summary),
            "readme" => Some(Self::Readme),
            "index" => Some(Self::Index),
            _ => None,
        }
    }
}

/// The page a handbook starts from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Start {
    /// The page, by its path under the folder.
    pub(super) listed: Listed,
    /// Its kind.
    pub(super) kind: Kind,
}

/// The start page among a folder's pages: of those at its top, a
/// `SUMMARY.md`, else a `README.md`, else an `index.md`, the first in the
/// walk's order of each kind.
pub(super) fn start(pages: &[Listed]) -> Option<Start> {
    pages
        .iter()
        .filter(|listed| listed.under.parent() == Some(Path::new("")))
        .filter_map(|listed| {
            Kind::of(&listed.under, true).map(|kind| Start {
                listed: listed.clone(),
                kind,
            })
        })
        .min_by_key(|start| start.kind)
}

/// A chapter in the handbook's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Placed {
    /// The page.
    pub(super) page: PageNo,
    /// How many parts it stands in.
    pub(super) depth: u8,
    /// The title of the part it opens, from `SUMMARY.md`'s headings.
    pub(super) part: Option<String>,
    /// No start page links it: it comes after the linked ones.
    pub(super) orphan: bool,
}

/// A folder of the run that no link of the handbook enters, and how many
/// of its pages are therefore left out of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LeftOut {
    /// The folder, under the folder named on the command line.
    pub(super) folder: PathBuf,
    /// How many pages under it are left out.
    pub(super) pages: usize,
}

/// A handbook's order.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Order {
    /// The chapters, linked ones first, then the pages of the folders the
    /// links enter that no start page links.
    pub(super) chapters: Vec<Placed>,
    /// The folders no link enters.
    pub(super) left_out: Vec<LeftOut>,
}

/// The order of the handbook that `start` begins, among `pages`: the
/// folder's pages in the walk's order, translations left out.
pub(super) fn order(start: &Start, pages: &[Listed], surveys: &mut Surveys<'_>) -> Order {
    let by_under: BTreeMap<&Path, PageNo> = pages
        .iter()
        .map(|listed| (listed.under.as_path(), listed.page))
        .collect();
    let mut walk = Walk {
        by_under: &by_under,
        pages,
        seen: BTreeSet::from([start.listed.page]),
        entered: BTreeSet::from([PathBuf::new()]),
        chapters: Vec::new(),
    };
    let summary = start.kind == Kind::Summary;
    if !summary {
        walk.chapters.push(Placed {
            page: start.listed.page,
            depth: 0,
            part: None,
            orphan: false,
        });
    }
    walk.follow(&start.listed, 0, summary, surveys);
    walk.finish()
}

/// A walk through the start pages' links.
struct Walk<'walk> {
    /// The pages, by their path under the folder.
    by_under: &'walk BTreeMap<&'walk Path, PageNo>,
    /// The pages, in the walk's order.
    pages: &'walk [Listed],
    /// The pages placed already, and the start page.
    seen: BTreeSet<PageNo>,
    /// The folders a link enters, under the folder: its top among them.
    entered: BTreeSet<PathBuf>,
    /// The chapters so far.
    chapters: Vec<Placed>,
}

impl Walk<'_> {
    /// Places the pages start page `from` links, `depth` parts deep; with
    /// `summary`, its nested lists are parts and its headings after the
    /// first link name them.
    fn follow(&mut self, from: &Listed, depth: u8, summary: bool, surveys: &mut Surveys<'_>) {
        let dir = from.under.parent().unwrap_or_else(|| Path::new(""));
        let marks = surveys.marks(from.page);
        let mut part = None;
        let mut linked = false;
        for mark in marks {
            match mark {
                // `# Summary` above the first link is the title, not a part.
                Mark::Heading { level: _, text } if summary && linked => part = Some(text),
                Mark::Heading { level: _, text: _ } => {}
                Mark::Link { href, depth: lists } => {
                    linked = true;
                    let Some((under, page)) = self.find(dir, &href) else {
                        continue;
                    };
                    if !self.seen.insert(page) {
                        continue;
                    }
                    let deep = if summary {
                        depth.saturating_add(lists.saturating_sub(1))
                    } else {
                        depth
                    };
                    self.entered.insert(folder_of(&under));
                    self.chapters.push(Placed {
                        page,
                        depth: deep,
                        part: part.take(),
                        orphan: false,
                    });
                    // A subfolder's start page opens a part: its own chapters
                    // follow it.
                    let opens_part = !summary
                        && under.parent() != Some(Path::new(""))
                        && Kind::of(&under, false).is_some();
                    if opens_part {
                        let listed = Listed { under, page };
                        self.follow(&listed, deep.saturating_add(1), false, surveys);
                    }
                }
            }
        }
    }

    /// The page a link from folder `dir` leads to, when it is one of the
    /// folder's pages: a link to a folder leads to its start page.
    fn find(&self, dir: &Path, href: &str) -> Option<(PathBuf, PageNo)> {
        let under = resolve(dir, href)?;
        if let Some(&page) = self.by_under.get(under.as_path()) {
            return Some((under, page));
        }
        // `part/` or `part`: the folder's README, else its index.
        self.pages
            .iter()
            .filter(|listed| listed.under.parent() == Some(under.as_path()))
            .filter_map(|listed| Kind::of(&listed.under, false).map(|kind| (kind, listed)))
            .min_by_key(|(kind, _)| *kind)
            .map(|(_, listed)| (listed.under.clone(), listed.page))
    }

    /// The order: the linked chapters, then the pages of the entered
    /// folders no link placed, in the walk's order; the folders left out.
    fn finish(mut self) -> Order {
        let mut left_out: Vec<LeftOut> = Vec::new();
        for listed in self.pages {
            if self.seen.contains(&listed.page) {
                continue;
            }
            let folder = folder_of(&listed.under);
            if self.entered.contains(&folder) {
                self.chapters.push(Placed {
                    page: listed.page,
                    depth: 0,
                    part: None,
                    orphan: true,
                });
                continue;
            }
            // The highest folder on its way that no link enters.
            let outermost = folder
                .ancestors()
                .take_while(|up| !up.as_os_str().is_empty() && !self.entered.contains(*up))
                .last()
                .map_or_else(|| folder.clone(), Path::to_path_buf);
            match left_out.iter_mut().find(|left| left.folder == outermost) {
                Some(left) => left.pages = left.pages.saturating_add(1),
                None => left_out.push(LeftOut {
                    folder: outermost,
                    pages: 1,
                }),
            }
        }
        Order {
            chapters: self.chapters,
            left_out,
        }
    }
}

/// The folder a page is in, under the folder named on the command line:
/// empty for its top.
fn folder_of(under: &Path) -> PathBuf {
    under.parent().map_or_else(PathBuf::new, Path::to_path_buf)
}

/// Where a relative link from folder `dir` leads, under the folder named
/// on the command line.
///
/// Its `#fragment` and `?query` cut, percent-decoded, `.` and `..` read. None
/// for a link with a scheme, one to this page alone (`#top`), and one that
/// leaves the folder.
fn resolve(dir: &Path, href: &str) -> Option<PathBuf> {
    if !is_relative(href) {
        return None;
    }
    let path = href.split(['#', '?']).next().unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let decoded = percent_decode_path(path)?;
    let mut under = PathBuf::new();
    for part in dir.join(decoded).components() {
        match part {
            Component::Normal(name) => under.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if !under.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(under)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Kind, Listed, resolve, start};
    use crate::handbook::PageNo;

    /// The pages of a folder, numbered in the order given.
    fn listed(names: &[&str]) -> Vec<Listed> {
        names
            .iter()
            .enumerate()
            .map(|(at, name)| Listed {
                under: PathBuf::from(name),
                page: PageNo(at),
            })
            .collect()
    }

    /// `SUMMARY.md` starts a handbook before `README.md`, which starts it
    /// before `index.md`, in any case and with any Markdown extension; a
    /// subfolder's is none; a folder without one has none.
    ///
    /// # Panics
    ///
    /// When another page is taken for the start page, or one is found in a
    /// folder without any.
    #[test]
    fn the_start_page_is_summary_then_readme_then_index() {
        let pages = listed(&["index.md", "Readme.markdown", "part/SUMMARY.md", "a.md"]);
        let found = start(&pages).unwrap();
        assert_eq!((found.listed.page, found.kind), (PageNo(1), Kind::Readme));
        let summary = listed(&["index.md", "README.md", "summary.MD"]);
        assert_eq!(start(&summary).unwrap().kind, Kind::Summary);
        assert_eq!(
            start(&listed(&["index.qmd", "a.md"])).unwrap().kind,
            Kind::Index
        );
        assert_eq!(start(&listed(&["a.md", "part/README.md"])), None);
    }

    /// A link is cut at `#` and `?`, percent-decoded and read from the
    /// start page's folder; one with a scheme, one to the page itself and
    /// one leaving the folder lead nowhere.
    ///
    /// # Panics
    ///
    /// When a link is read to another path, or one of the others to any.
    #[test]
    fn a_link_is_read_from_its_folder() {
        let at = |dir: &str, href: &str| resolve(Path::new(dir), href);
        assert_eq!(at("", "a.md#x"), Some(PathBuf::from("a.md")));
        assert_eq!(at("", "./b%20c.md?y"), Some(PathBuf::from("b c.md")));
        assert_eq!(at("part", "../a.md"), Some(PathBuf::from("a.md")));
        assert_eq!(at("", "https://x.test/a.md"), None);
        assert_eq!(at("", "#top"), None);
        assert_eq!(at("", "../outside.md"), None);
        assert_eq!(at("", "/etc/passwd.md"), None);
    }
}
