// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The parts of a page around its article: the heading, the footer, the
//! files added to it, and the description of it all that the page is
//! written from.

use super::language::Language;
use super::{Doc, Neighbours, beside, checks, stem};
use crate::config::{self, Ui};
use crate::model::Block;
use crate::render::{self, Warning};
use crate::text::esc;
use crate::{inline, page, text};

/// What the page is called, above its article.
pub(super) struct Heading {
    /// The Markdown file's name, `stdin` without one.
    name: String,
    /// The title.
    title: String,
    /// The line above the title, escaped.
    eyebrow: String,
}

/// What the files a page names add to it.
pub(super) struct Added {
    /// The stylesheets, one after the other.
    css: String,
    /// The head files, sanitized.
    head: String,
    /// mermaid.js, when a diagram needs it.
    mermaid_js: Option<String>,
}

/// Everything the page's description is made from besides the settings.
pub(super) struct Parts {
    /// The title and what is above it.
    pub(super) heading: Heading,
    /// Where the full shared glossary is.
    pub(super) glossary_url: Option<String>,
    /// The fine print, as HTML.
    pub(super) footer: String,
    /// The meta description.
    pub(super) description: String,
    /// What the named files add.
    pub(super) added: Added,
    /// The links to the run's other pages, as HTML.
    pub(super) nav: String,
    /// The notice of a translation, as HTML.
    pub(super) notice: String,
    /// The page's language and versions.
    pub(super) language: Language,
}

/// The links to a page's neighbours.
pub(super) fn nav(links: &Neighbours, ui: &Ui) -> String {
    page::pages_nav(
        links.prev.as_ref().map(pair),
        links.next.as_ref().map(pair),
        ui,
    )
}

/// A link as the page's navigation takes it: where, and what.
fn pair(link: &super::Link) -> (&str, &str) {
    (link.href.as_str(), link.title.as_str())
}

impl Doc<'_> {
    /// The file's name, the title and the line above it.
    pub(super) fn heading(&self, language: &Language) -> Heading {
        let settings = &self.settings;
        let name = self.source.and_then(|path| path.file_name()).map_or_else(
            || String::from("stdin"),
            |file| file.to_string_lossy().into_owned(),
        );
        let title = settings
            .title
            .clone()
            .unwrap_or_else(|| self.first_title().unwrap_or_else(|| stem(self.source)));
        let date = settings.date.as_ref().map_or_else(
            || language.ui.date(text::civil(text::now_secs())),
            |date| text::iso_date(date).map_or_else(|| date.clone(), |day| language.ui.date(day)),
        );
        let eyebrow = match (&settings.eyebrow, &settings.version) {
            (Some(eyebrow), _) => esc(eyebrow),
            (None, None) => format!("{} &middot; {}", esc(&name), esc(&date)),
            (None, Some(version)) => format!(
                "{} &middot; {} &middot; {}",
                esc(&name),
                esc(version),
                esc(&date)
            ),
        };
        Heading {
            name,
            title,
            eyebrow,
        }
    }

    /// The document's own title: its first heading of level one.
    fn first_title(&self) -> Option<String> {
        let heading = self.blocks.iter().find_map(|block| {
            if let Block::Heading(heading) = block {
                (heading.level == 1).then_some(heading)
            } else {
                None
            }
        })?;
        let env = inline::Env {
            defs: &self.defs,
            html: inline::HtmlMode::Safe,
            math: false,
        };
        Some(inline::plain(&inline::parse(&heading.text, &env)))
    }

    /// What the article is rendered with.
    pub(super) fn inputs<'inputs>(
        &'inputs self,
        terms: &'inputs [crate::glossary::Term],
        language: &Language,
        heading: &Heading,
    ) -> render::Inputs<'inputs> {
        let settings = &self.settings;
        render::Inputs {
            defs: &self.defs,
            terms,
            ui: language.ui,
            html: self.html,
            math: self.math,
            title: heading.title.clone(),
            eyebrow: heading.eyebrow.clone(),
            bases: self.places.bases(),
            embed: settings.embed.unwrap_or(false),
            max_embed: settings.max_embed.unwrap_or(render::MAX_EMBED),
            mermaid: match settings.mermaid.as_deref() {
                Some("code") => render::Diagrams::Code,
                Some("js") if settings.mermaid_js.is_some() => render::Diagrams::Js,
                _ => render::Diagrams::Native,
            },
            mermaid_js: settings.mermaid_js.is_some(),
        }
    }

    /// The meta description: the setting, or the lead cut at a word.
    pub(super) fn description(&self, article: &render::Article) -> String {
        self.settings
            .description
            .clone()
            .unwrap_or_else(|| checks::shorten(&article.lead, 200))
    }

    /// The fine print: the setting as Markdown, or the page's own words.
    pub(super) fn footer(&self, language: &Language, heading: &Heading) -> String {
        self.settings.footer.as_ref().map_or_else(
            || language.ui.footer.fill([&esc(&heading.name)]),
            |footer| {
                let env = inline::Env {
                    defs: &self.defs,
                    html: self.html,
                    math: false,
                };
                render::bare_inline(&inline::parse(footer, &env))
            },
        )
    }

    /// The stylesheets, heads and mermaid.js the page adds; what cannot be
    /// read is a warning.
    pub(super) fn added(&mut self, article: &render::Article) -> Added {
        let base = self.places.base();
        let mut css = String::new();
        let mut head = String::new();
        let mut unread = Vec::new();
        for file in &self.settings.css {
            match beside::read_beside(base, file) {
                Ok(sheet) => css.push_str(&sheet),
                Err(message) => unread.push(message),
            }
        }
        for file in &self.settings.head {
            // Only meta, link and style pass: a page runs no code but
            // md2html's.
            match beside::read_beside(base, file) {
                Ok(part) => head.push_str(&crate::sanitize::head(&part)),
                Err(message) => unread.push(message),
            }
        }
        // mermaid.js goes into a page only when one of its diagrams needs it.
        let wanted = self
            .settings
            .mermaid_js
            .as_ref()
            .filter(|_| article.mermaid_js);
        let mermaid_js = wanted.and_then(|file| match beside::read_beside(base, file) {
            Ok(script) => Some(script.replace("</script", "<\\/script")),
            Err(message) => {
                unread.push(format!("mermaid.js: {message}"));
                None
            }
        });
        if self.settings.mermaid.as_deref() == Some("js") && self.settings.mermaid_js.is_none() {
            unread.push(String::from(
                "mermaid: js needs --mermaid-js FILE; the diagrams are drawn natively",
            ));
        }
        self.warnings.extend(
            unread
                .into_iter()
                .map(|message| Warning { line: 0, message }),
        );
        Added {
            css,
            head,
            mermaid_js,
        }
    }

    /// Where the name in the bar leads: the setting when a page may carry
    /// it, else the top.
    fn brand_link(&mut self) -> String {
        match &self.settings.brand_link {
            Some(link) if text::safe_url(link, false) => link.clone(),
            Some(link) => {
                let message = format!(
                    "brand_link: `{link}` is not a link a page may carry; the brand leads to the top"
                );
                self.warnings.push(Warning { line: 1, message });
                String::from("#top")
            }
            None => String::from("#top"),
        }
    }

    /// The copies offered beside the page, by kind and file name.
    fn exports(&self) -> Vec<page::Export> {
        let Some(file_stem) = self.out.and_then(|out| out.file_stem()) else {
            return Vec::new();
        };
        let page_stem = file_stem.to_string_lossy();
        let mut list = Vec::new();
        if self.settings.pdf == Some(true) {
            list.push(page::Export::Pdf(format!("{page_stem}.pdf")));
        }
        if self.settings.epub == Some(true) {
            list.push(page::Export::Epub(format!("{page_stem}.epub")));
        }
        list
    }

    /// Everything the page is written from.
    pub(super) fn meta(&mut self, parts: Parts) -> page::Meta {
        let brand_link = self.brand_link();
        let exports = self.exports();
        let settings = &self.settings;
        let Parts {
            heading,
            glossary_url,
            footer,
            description,
            added,
            nav,
            notice,
            language,
        } = parts;
        let direction = config::direction(&language.lang);
        let shows_versions =
            language.versions.len() > 1 || settings.translations.as_deref() == Some("device");
        page::Meta {
            title: heading.title,
            source: heading.name.clone(),
            switches: page::Switches::default()
                .with(page::Switch::LargeType, settings.large.unwrap_or(false))
                .with(page::Switch::Indexed, settings.index.unwrap_or(false))
                .with(page::Switch::Outline, settings.toc.unwrap_or(true))
                .with(page::Switch::Csp, settings.csp.unwrap_or(false))
                .with(
                    page::Switch::DeviceTranslation,
                    settings.translations.as_deref() != Some("false"),
                )
                .with(page::Switch::Embedded, settings.embed.unwrap_or(false)),
            glossary_url,
            ui: language.ui,
            lang: language.lang,
            brand: settings
                .brand
                .as_deref()
                .map_or_else(|| String::from("md2html"), esc),
            brand_link,
            kicker: settings
                .kicker
                .as_deref()
                .map_or_else(|| esc(&heading.name), esc),
            footer,
            description,
            author: settings.author.clone(),
            css: added.css.replace("</style", "<\\/style"),
            head: added.head,
            nav,
            mermaid_js: added.mermaid_js,
            direction,
            versions: if shows_versions {
                language.versions
            } else {
                Vec::new()
            },
            notice,
            file: self
                .out
                .and_then(|out| out.file_name())
                .map(|file| file.to_string_lossy().into_owned())
                .unwrap_or_default(),
            live: if settings.serve == Some(true) {
                Some(page::Live::Serve)
            } else {
                settings.live.map(|_| page::Live::Check)
            },
            ttl: settings.live.unwrap_or(120),
            exports,
        }
    }
}
