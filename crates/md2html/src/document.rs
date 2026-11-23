// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! One Markdown text as a page: its front matter over the command line's
//! settings, its glossary joined with the shared one, its language and
//! translations, the files it names beside it, and the page and book made
//! of it all.

mod beside;
mod checks;
mod glossary;
mod language;
mod parts;

use std::path::{Path, PathBuf};

pub(crate) use beside::MARKDOWN;
pub(crate) use glossary::shared_glossary_path;

use crate::config::{Settings, front_matter};
use crate::inline::HtmlMode;
use crate::model::{Block, Defs};
use crate::render::Warning;
use crate::{block, export, page, render};

/// A rendered page and what came of it.
pub(crate) struct Rendered {
    /// The HTML.
    pub(crate) html: String,
    /// What the author should know.
    pub(crate) warnings: Vec<Warning>,
    /// Glossary terms on the page.
    pub(crate) terms: usize,
    /// Of them, from the shared glossary.
    pub(crate) shared: usize,
    /// The page as an EPUB, when one was asked for.
    pub(crate) epub: Option<Vec<u8>>,
    /// A PDF of the page was asked for.
    pub(crate) pdf: bool,
}

/// A link to another page of the same run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Link {
    /// Where it leads, relative to this page.
    pub(crate) href: String,
    /// That page's title.
    pub(crate) title: String,
}

/// A page's neighbours in a run, for `--nav`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Neighbours {
    /// The page before it.
    pub(crate) prev: Option<Link>,
    /// The page after it.
    pub(crate) next: Option<Link>,
}

/// One Markdown text as a page: the whole page, or with `fragment` only
/// the HTML of the Markdown.
pub(crate) fn render_doc(
    text: &str,
    source: Option<&Path>,
    out: Option<&Path>,
    cli: &Settings,
    fragment: bool,
    nav: Option<Neighbours>,
) -> Rendered {
    let (map, body, first_line) = front_matter(text);
    let (own, warnings) = beside::own_settings(&map, source);
    let settings = own.overlay(cli);
    let math = settings.math.unwrap_or(true);
    let html = if settings.html == Some(true) {
        HtmlMode::Rich
    } else {
        HtmlMode::Safe
    };
    let (blocks, defs) = block::parse(body, first_line, math, false);
    if fragment {
        return Rendered {
            html: render::fragment(&blocks, &defs, html, math),
            warnings,
            terms: 0,
            shared: 0,
            epub: None,
            pdf: false,
        };
    }
    Doc {
        settings,
        source,
        out,
        places: Places::new(source, out),
        blocks,
        defs,
        html,
        math,
        warnings,
    }
    .page(nav)
}

/// What every part of one page is made from.
struct Doc<'doc> {
    /// The front matter over the command line.
    settings: Settings,
    /// The Markdown file, or `None` for standard input.
    source: Option<&'doc Path>,
    /// The page's file, or `None` for standard output.
    out: Option<&'doc Path>,
    /// The folders it looks in.
    places: Places,
    /// The Markdown's blocks.
    blocks: Vec<Block>,
    /// Its link references and footnotes.
    defs: Defs,
    /// What raw HTML may do.
    html: HtmlMode,
    /// `$...$` is maths.
    math: bool,
    /// What the author should know, in the order it was found.
    warnings: Vec<Warning>,
}

/// The folders a page looks in for the files it names.
struct Places {
    /// The page's folder.
    out_dir: Option<PathBuf>,
    /// The Markdown's folder.
    src_dir: Option<PathBuf>,
}

impl Places {
    /// The folders of `source` and `out`.
    fn new(source: Option<&Path>, out: Option<&Path>) -> Self {
        Self {
            out_dir: out.map(folder),
            src_dir: source.map(folder),
        }
    }

    /// Where relative links are looked for: the page's folder first, then
    /// the Markdown's, each once.
    fn bases(&self) -> Vec<&Path> {
        let mut bases: Vec<&Path> = Vec::new();
        for dir in self.both() {
            if !bases.contains(&dir) {
                bases.push(dir);
            }
        }
        bases
    }

    /// The page's folder and the Markdown's, as they are.
    fn both(&self) -> impl Iterator<Item = &Path> {
        [self.out_dir.as_deref(), self.src_dir.as_deref()]
            .into_iter()
            .flatten()
    }

    /// The folder the files the Markdown names are read from.
    fn base(&self) -> Option<&Path> {
        self.src_dir.as_deref()
    }
}

/// The folder a file is in: `.` for a bare name.
fn folder(path: &Path) -> PathBuf {
    path.parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// A file's name without its extension; `stdin` without a file.
pub(crate) fn stem(source: Option<&Path>) -> String {
    source.and_then(Path::file_stem).map_or_else(
        || String::from("stdin"),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl Doc<'_> {
    /// The page, its book, and what the author should know.
    fn page(mut self, nav: Option<Neighbours>) -> Rendered {
        let (terms, glossary_url) =
            glossary::page_terms(&self.blocks, &self.defs, &self.settings, self.source);
        let language = language::Language::of(&self.settings, self.source);
        let notice = language.notice(&self.settings, self.source, &mut self.warnings);
        let heading = self.heading(&language);
        let article = render::article(&self.blocks, &self.inputs(&terms, &language, &heading));
        self.warnings.extend(article.warnings.iter().cloned());
        let added = self.added(&article);
        let footer = self.footer(&language, &heading);
        let description = self.description(&article);
        let nav_html = nav
            .map(|links| parts::nav(&links, language.ui))
            .unwrap_or_default();
        let meta = self.meta(parts::Parts {
            heading,
            glossary_url,
            footer,
            description,
            added,
            nav: nav_html,
            notice,
            language,
        });
        let html = page::page(&article, &terms, &meta);
        self.warnings.extend(
            checks::broken_anchors(&html)
                .into_iter()
                .map(|missing| Warning {
                    line: 0,
                    message: format!("a link points at #{missing}, which is nowhere on the page"),
                }),
        );
        let epub = self.book(&html, &meta, &article);
        Rendered {
            html,
            warnings: self.warnings,
            terms: terms.len(),
            shared: terms.iter().filter(|term| term.shared).count(),
            epub,
            pdf: self.settings.pdf == Some(true),
        }
    }

    /// The page as an EPUB, when one is asked for and the page has a file.
    fn book(
        &mut self,
        html: &str,
        meta: &page::Meta,
        article: &render::Article,
    ) -> Option<Vec<u8>> {
        if self.settings.epub != Some(true) || self.out.is_none() {
            return None;
        }
        let bases: Vec<&Path> = self.places.both().collect();
        let book = export::Book {
            title: &meta.title,
            lang: &meta.lang,
            direction: meta.direction,
            author: self.settings.author.as_deref(),
            css: &article.css,
            bases: &bases,
        };
        let (bytes, problems) = export::epub(html, &book);
        self.warnings.extend(
            problems
                .into_iter()
                .map(|message| Warning { line: 0, message }),
        );
        Some(bytes)
    }
}
