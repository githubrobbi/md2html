// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Handbooks: a folder named on the command line whose start page links
//! its pages is read as one book.
//!
//! Its start page (`SUMMARY.md`, else `README.md`, else `index.md`) gives
//! the order by the links it holds; every chapter shows the handbook's
//! contents in its outline and links the chapters before and after it, and
//! every chapter may show images from anywhere in the handbook's folder. A page
//! in a folder the links enter that no start page links is said to be
//! there; a folder no link enters stays out. Each language of the
//! translations is a handbook of its own. Without a start page that links
//! the run's pages there is no handbook, and nothing of this changes a
//! page, unless `--nav` asks for one of the whole run.

mod languages;
mod order;
#[cfg(test)]
mod tests;

use alloc::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use self::order::{LeftOut, Listed, Placed};
use crate::config::{BookFolder, Glance, Settings};
use crate::document::{Link, Mark, Neighbours, Place, Survey, translation_suffix};
use crate::page::{Chapter as Entered, Contents, Entry};
use crate::system::relative_page;

/// A page of the run, by its place in the run's list of pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PageNo(usize);

/// A page of the run, as a handbook sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Page<'run> {
    /// The Markdown file; none for standard input, which is in no handbook.
    pub(crate) file: Option<&'run Path>,
    /// The folder named on the command line it was found in; none for a
    /// file named itself.
    pub(crate) folder: Option<&'run Path>,
    /// Its page; none for standard output.
    pub(crate) out: Option<&'run Path>,
    /// Its Markdown; none when it could not be read.
    pub(crate) text: Option<&'run str>,
}

/// What the run asks of its handbooks.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Asked<'run> {
    /// The command line's settings: `--no-handbook` among them.
    pub(crate) cli: &'run Settings,
    /// `--nav`: the pages in no handbook make one, in the run's order.
    pub(crate) nav: bool,
}

/// The handbooks of a run.
#[derive(Debug, Default)]
pub(crate) struct Handbooks {
    /// The handbooks.
    books: Vec<Book>,
    /// What the person should know of them that is no page's: the folders
    /// left out.
    notes: Vec<String>,
    /// The pages of the handbooks, and the start pages that are no chapter
    /// (`SUMMARY.md`): `--nav` leaves them to their handbook.
    claimed: BTreeSet<PageNo>,
}

/// One handbook.
#[derive(Debug)]
struct Book {
    /// Its chapters, in its order.
    chapters: Vec<Chapter>,
    /// Its folder, which every chapter may read images from.
    images: Option<BookFolder>,
}

/// A chapter of a handbook.
#[derive(Debug)]
struct Chapter {
    /// The page.
    page: PageNo,
    /// Its title, as the page shows it.
    title: String,
    /// How many parts it stands in.
    depth: u8,
    /// The title of the part it opens.
    part: Option<String>,
    /// What the author should know of its place: no start page links it.
    warning: Option<String>,
}

impl Handbooks {
    /// The handbooks among `pages`.
    #[must_use]
    pub(crate) fn find(pages: &[Page<'_>], asked: Asked<'_>) -> Self {
        let mut surveys = Surveys {
            pages,
            cli: asked.cli,
            done: BTreeMap::new(),
        };
        let mut found = Self::default();
        if asked.cli.handbook == Some(false) {
            return found;
        }
        let mut folders: Vec<&Path> = Vec::new();
        for folder in pages.iter().filter_map(|page| page.folder) {
            if !folders.contains(&folder) {
                folders.push(folder);
            }
        }
        for folder in folders {
            found.folder(folder, &mut surveys);
        }
        if asked.nav {
            found.whole_run(&mut surveys);
        }
        found
    }

    /// The handbooks of `folder`, named on the command line: the original
    /// language's, when its start page links its pages, and each
    /// translation's.
    fn folder(&mut self, folder: &Path, surveys: &mut Surveys<'_>) {
        let (originals, translations): (Vec<Listed>, Vec<Listed>) = surveys
            .pages
            .iter()
            .enumerate()
            .filter(|(_, page)| page.folder == Some(folder))
            .filter_map(|(at, page)| {
                let under = page.file?.strip_prefix(folder).ok()?;
                Some(Listed {
                    under: under.to_path_buf(),
                    page: PageNo(at),
                })
            })
            .partition(|listed| translation_suffix(&listed.under).is_none());
        let Some(start) = order::start(&originals) else {
            return;
        };
        if surveys.glance(start.listed.page).handbook == Some(false) {
            return;
        }
        let order = order::order(&start, &originals, surveys);
        // The start page must link a page of the run: the pages it does not
        // link make no handbook of a folder.
        let linked = order
            .chapters
            .iter()
            .any(|placed| !placed.orphan && placed.page != start.listed.page);
        if !linked || order.chapters.len() < 2 {
            return;
        }
        let start_name = start.listed.under.display().to_string();
        let named = folder.display().to_string();
        let shown = named.trim_end_matches(['/', '\\']);
        for LeftOut { folder: out, pages } in &order.left_out {
            let (count, verb) = if *pages == 1 {
                (String::from("1 page"), "is")
            } else {
                (format!("{pages} pages"), "are")
            };
            self.notes.push(format!(
                "md2html: {count} under {} {verb} not in the handbook (no link from {} enters it)",
                Path::new(shown).join(out).display(),
                Path::new(shown).join(&start.listed.under).display()
            ));
        }
        let warn = |placed: &Placed, looked: &mut Surveys<'_>| {
            (placed.orphan && looked.glance(placed.page).orphan != Some(true)).then(|| {
                format!(
                    "not linked from {start_name}; it comes after the pages {start_name} links (orphan: true says that is meant)"
                )
            })
        };
        self.claimed.insert(start.listed.page);
        let original = Book::new(&order.chapters, folder, surveys, warn);
        let chapters: Vec<PageNo> = original
            .chapters
            .iter()
            .map(|chapter| chapter.page)
            .collect();
        self.books.push(original);
        self.books
            .extend(languages::books(folder, &translations, &chapters, surveys));
        let pages = self
            .books
            .iter()
            .flat_map(|book| book.chapters.iter().map(|chapter| chapter.page));
        self.claimed.extend(pages);
    }

    /// With `--nav`: the pages in no handbook, when there are two or more,
    /// as one in the run's order.
    fn whole_run(&mut self, surveys: &mut Surveys<'_>) {
        let rest: Vec<Placed> = surveys
            .pages
            .iter()
            .enumerate()
            .filter(|(at, page)| page.file.is_some() && !self.claimed.contains(&PageNo(*at)))
            .map(|(at, _)| Placed {
                page: PageNo(at),
                depth: 0,
                part: None,
                orphan: false,
            })
            .collect();
        if rest.len() < 2 {
            return;
        }
        let chapters = rest
            .iter()
            .map(|placed| Chapter {
                page: placed.page,
                title: surveys.title(placed.page),
                depth: 0,
                part: None,
                warning: None,
            })
            .collect();
        self.books.push(Book {
            chapters,
            images: None,
        });
    }

    /// What the person should know of the handbooks that is no page's.
    #[must_use]
    pub(crate) fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Where each of `pages` stands, in their order: a chapter of a
    /// handbook its place in it, any other page none.
    #[must_use]
    pub(crate) fn places<'books>(&'books self, pages: &[Page<'_>]) -> Vec<Option<Place<'books>>> {
        let mut places: Vec<Option<Place<'books>>> = pages.iter().map(|_| None).collect();
        for book in &self.books {
            for (at, chapter) in book.chapters.iter().enumerate() {
                if let Some(slot) = places.get_mut(chapter.page.0) {
                    *slot = Some(book.place(at, chapter, pages));
                }
            }
        }
        places
    }
}

impl Book {
    /// A handbook of `placed` in `folder`; `warn` says what a chapter's
    /// place needs said.
    fn new(
        placed: &[Placed],
        folder: &Path,
        surveys: &mut Surveys<'_>,
        warn: impl Fn(&Placed, &mut Surveys<'_>) -> Option<String>,
    ) -> Self {
        let chapters: Vec<Chapter> = placed
            .iter()
            .map(|place| Chapter {
                page: place.page,
                title: surveys.title(place.page),
                depth: place.depth,
                part: place.part.clone(),
                warning: warn(place, surveys),
            })
            .collect();
        Self {
            chapters,
            images: BookFolder::named(folder),
        }
    }

    /// The place of chapter number `at`.
    fn place<'books>(
        &'books self,
        at: usize,
        chapter: &Chapter,
        pages: &[Page<'_>],
    ) -> Place<'books> {
        let here = pages.get(chapter.page.0).and_then(|page| page.out);
        let href = |page: PageNo| {
            let there = pages.get(page.0).and_then(|other| other.out)?;
            relative_page(here?, there)
        };
        let link = |beside: Option<&Chapter>| {
            let other = beside?;
            Some(Link {
                href: href(other.page)?,
                title: other.title.clone(),
            })
        };
        let neighbours = Neighbours {
            prev: link(
                at.checked_sub(1)
                    .and_then(|before| self.chapters.get(before)),
            ),
            next: link(at.checked_add(1).and_then(|after| self.chapters.get(after))),
        };
        let mut entries = Vec::new();
        for other in &self.chapters {
            let Some(to) = href(other.page) else {
                continue;
            };
            if let Some(part) = &other.part {
                entries.push(Entry::Part(part.clone()));
            }
            entries.push(Entry::Chapter(Entered {
                href: to,
                title: other.title.clone(),
                depth: other.depth,
                current: other.page == chapter.page,
            }));
        }
        Place {
            neighbours,
            contents: here.map(|_| Contents::new(entries)),
            images: self.images.as_ref(),
            warnings: chapter.warning.iter().cloned().collect(),
        }
    }
}

/// The run's pages, each surveyed once when a handbook first needs it.
struct Surveys<'run> {
    /// The run's pages.
    pages: &'run [Page<'run>],
    /// The command line's settings.
    cli: &'run Settings,
    /// The pages surveyed so far.
    done: BTreeMap<PageNo, Survey>,
}

impl Surveys<'_> {
    /// The survey of `page`; none when it has no Markdown.
    fn get(&mut self, page: PageNo) -> Option<&Survey> {
        if !self.done.contains_key(&page) {
            let it = self.pages.get(page.0)?;
            let survey = Survey::of(it.text?, it.file, self.cli);
            self.done.insert(page, survey);
        }
        self.done.get(&page)
    }

    /// The headings and links of `page`.
    fn marks(&mut self, page: PageNo) -> Vec<Mark> {
        self.get(page).map(Survey::marks).unwrap_or_default()
    }

    /// The title `page` shows; its file's name when it has no Markdown.
    fn title(&mut self, page: PageNo) -> String {
        if let Some(survey) = self.get(page) {
            return survey.title();
        }
        crate::document::stem(self.pages.get(page.0).and_then(|it| it.file))
    }

    /// What the settings of `page` say of handbooks.
    fn glance(&mut self, page: PageNo) -> Glance {
        self.get(page)
            .map(|survey| survey.glance().clone())
            .unwrap_or_default()
    }
}
