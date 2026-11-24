// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The settings a page is rendered with, and how a key and its value, from
//! the front matter or the command line, set one of them.

use std::path::PathBuf;

use super::values::{Diagrams, Fingerprint, Translations, Ttl};

/// Everything a page may be told, from front matter and the command line.
#[derive(Clone, Debug, Default)]
pub(crate) struct Settings {
    /// The page's title, instead of its first heading.
    pub(crate) title: Option<String>,
    /// The language: `lang` attribute, interface words, date.
    pub(crate) lang: Option<String>,
    /// The name in the bar instead of "md2html".
    pub(crate) brand: Option<String>,
    /// Where the name in the bar leads.
    pub(crate) brand_link: Option<String>,
    /// The tag beside the name, instead of the file name.
    pub(crate) kicker: Option<String>,
    /// The line above the title, instead of file name and date.
    pub(crate) eyebrow: Option<String>,
    /// The date shown, instead of today.
    pub(crate) date: Option<String>,
    /// A version shown with the date.
    pub(crate) version: Option<String>,
    /// The meta description, instead of the lead.
    pub(crate) description: Option<String>,
    /// The author, for the meta tags.
    pub(crate) author: Option<String>,
    /// The fine print at the foot, Markdown allowed.
    pub(crate) footer: Option<String>,
    /// Larger type.
    pub(crate) large: Option<bool>,
    /// Join the shared glossary.
    pub(crate) glossary: Option<bool>,
    /// Trust raw HTML.
    pub(crate) html: Option<bool>,
    /// Put the images inside the page.
    pub(crate) embed: Option<bool>,
    /// Add a Content-Security-Policy.
    pub(crate) csp: Option<bool>,
    /// The page's time to live, in seconds: when it runs out the page checks
    /// whether it changed (on a server) or simply reloads (from a file),
    /// keeping the reader's place.
    pub(crate) live: Option<Ttl>,
    /// Served by `--serve`: the local server says when to reload. Command
    /// line only.
    pub(crate) serve: Option<bool>,
    /// Print a PDF beside the page (with Chrome).
    pub(crate) pdf: Option<bool>,
    /// Write an EPUB beside the page.
    pub(crate) epub: Option<bool>,
    /// The largest image embedded, in bytes: command line only (a document
    /// cannot lift its own limits).
    pub(crate) max_embed: Option<u64>,
    /// Let search engines index the page.
    pub(crate) index: Option<bool>,
    /// Show the outline.
    pub(crate) toc: Option<bool>,
    /// Read `$...$` as maths.
    pub(crate) math: Option<bool>,
    /// How Mermaid diagrams are drawn.
    pub(crate) mermaid: Option<Diagrams>,
    /// A local copy of mermaid.js, for the diagrams drawn in the browser.
    pub(crate) mermaid_js: Option<PathBuf>,
    /// The file this page translates (for a translation).
    pub(crate) translated_from: Option<String>,
    /// The SHA-256 of that file when it was translated.
    pub(crate) source_sha256: Option<Fingerprint>,
    /// What the language switch offers.
    pub(crate) translations: Option<Translations>,
    /// Stylesheets to add, relative to the Markdown file.
    pub(crate) css: Vec<PathBuf>,
    /// HTML files to add to the head, relative to the Markdown file.
    pub(crate) head: Vec<PathBuf>,
}

impl Settings {
    /// Sets one key; an unknown key is reported.
    ///
    /// # Errors
    ///
    /// ``unknown setting `<key>` `` for a key that is neither a setting nor
    /// common front matter md2html passes over (`tags`, `layout`, ...), with
    /// the key as it is looked up (`Brand-Link` reads `brand_link`); a
    /// `mermaid` value that is not native, js or code; a `live` value that
    /// is neither yes, no nor a number of seconds.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let name = key_name(key);
        let text = || Some(value.to_owned());
        let flag = || Some(truthy(value));
        match name.as_str() {
            "title" => self.title = text(),
            "lang" | "language" => self.lang = text(),
            "brand" | "site" => self.brand = text(),
            "brand_link" | "brand_href" | "home" => self.brand_link = text(),
            "kicker" => self.kicker = text(),
            "eyebrow" => self.eyebrow = text(),
            "date" => self.date = text(),
            "version" => self.version = text(),
            "description" | "summary" => self.description = text(),
            "author" | "authors" => self.author = text(),
            "footer" => self.footer = text(),
            "large" => self.large = flag(),
            "glossary" | "shared_glossary" => self.glossary = flag(),
            "html" | "raw_html" => self.html = flag(),
            "embed" | "embed_images" | "self_contained" => self.embed = flag(),
            "csp" => self.csp = flag(),
            "pdf" => self.pdf = flag(),
            "live" => self.live = Ttl::parse(value)?,
            "epub" => self.epub = flag(),
            "index" => self.index = flag(),
            "noindex" => self.index = Some(!truthy(value)),
            "toc" | "outline" => self.toc = flag(),
            "math" => self.math = flag(),
            "mermaid" | "diagrams" => self.mermaid = Some(Diagrams::parse(value)?),
            "mermaid_js" => self.mermaid_js = Some(PathBuf::from(value.trim())),
            "translated_from" | "translation_of" => self.translated_from = text(),
            "source_sha256" => self.source_sha256 = Some(Fingerprint::parse(value)),
            "translations" | "language_switch" => {
                self.translations = Some(Translations::parse(value));
            }
            "css" | "stylesheet" => self.css.extend(paths(value)),
            "head" => self.head.extend(paths(value)),
            // Common front matter that is not the page's business.
            "tags" | "categories" | "layout" | "slug" | "permalink" | "draft" | "keywords"
            | "status" | "updated" | "created" | "translated_by" | "translator" => {}
            other => return Err(format!("unknown setting `{other}`")),
        }
        Ok(())
    }

    /// `over` on top of `self`: what `over` says wins.
    pub(crate) fn overlay(&self, over: &Self) -> Self {
        fn pick<Value: Clone>(base: Option<&Value>, top: Option<&Value>) -> Option<Value> {
            top.or(base).cloned()
        }
        let mut css = self.css.clone();
        css.extend(over.css.iter().cloned());
        let mut head = self.head.clone();
        head.extend(over.head.iter().cloned());
        Self {
            title: pick(self.title.as_ref(), over.title.as_ref()),
            lang: pick(self.lang.as_ref(), over.lang.as_ref()),
            brand: pick(self.brand.as_ref(), over.brand.as_ref()),
            brand_link: pick(self.brand_link.as_ref(), over.brand_link.as_ref()),
            kicker: pick(self.kicker.as_ref(), over.kicker.as_ref()),
            eyebrow: pick(self.eyebrow.as_ref(), over.eyebrow.as_ref()),
            date: pick(self.date.as_ref(), over.date.as_ref()),
            version: pick(self.version.as_ref(), over.version.as_ref()),
            description: pick(self.description.as_ref(), over.description.as_ref()),
            author: pick(self.author.as_ref(), over.author.as_ref()),
            footer: pick(self.footer.as_ref(), over.footer.as_ref()),
            large: pick(self.large.as_ref(), over.large.as_ref()),
            glossary: pick(self.glossary.as_ref(), over.glossary.as_ref()),
            html: pick(self.html.as_ref(), over.html.as_ref()),
            embed: pick(self.embed.as_ref(), over.embed.as_ref()),
            csp: pick(self.csp.as_ref(), over.csp.as_ref()),
            live: pick(self.live.as_ref(), over.live.as_ref()),
            serve: pick(self.serve.as_ref(), over.serve.as_ref()),
            pdf: pick(self.pdf.as_ref(), over.pdf.as_ref()),
            epub: pick(self.epub.as_ref(), over.epub.as_ref()),
            max_embed: pick(self.max_embed.as_ref(), over.max_embed.as_ref()),
            index: pick(self.index.as_ref(), over.index.as_ref()),
            toc: pick(self.toc.as_ref(), over.toc.as_ref()),
            math: pick(self.math.as_ref(), over.math.as_ref()),
            mermaid: pick(self.mermaid.as_ref(), over.mermaid.as_ref()),
            mermaid_js: pick(self.mermaid_js.as_ref(), over.mermaid_js.as_ref()),
            translated_from: pick(self.translated_from.as_ref(), over.translated_from.as_ref()),
            source_sha256: pick(self.source_sha256.as_ref(), over.source_sha256.as_ref()),
            translations: pick(self.translations.as_ref(), over.translations.as_ref()),
            css,
            head,
        }
    }
}

/// A key as it is looked up: trimmed, lowercased, dashes as underscores
/// (`Brand-Link` is `brand_link`).
pub(super) fn key_name(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('-', "_")
}

/// Whether a value says yes.
fn truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "on" | "1"
    )
}

/// A comma-separated list of paths, the empty ones left out.
fn paths(value: &str) -> impl Iterator<Item = PathBuf> {
    value
        .split(',')
        .map(|part| PathBuf::from(part.trim()))
        .filter(|path| !path.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::Settings;
    use crate::config::Ttl;

    /// A known setting is set; an unknown one is named in its error.
    ///
    /// # Panics
    ///
    /// When `brand` is not set, or when `colour` is accepted or refused with
    /// another message than ``unknown setting `colour` ``.
    #[test]
    fn unknown_settings_are_named() {
        let mut settings = Settings::default();
        settings.set("brand", "Hillside").unwrap();
        assert_eq!(settings.brand.as_deref(), Some("Hillside"));
        assert_eq!(
            settings.set("colour", "red"),
            Err("unknown setting `colour`".to_owned())
        );
    }

    /// `live` takes yes, no or a number of seconds, held to at least five.
    ///
    /// # Panics
    ///
    /// When `yes` is not two minutes, when `2s` is not held to five seconds,
    /// when `off` does not switch it off, or when `soon` is accepted or refused
    /// with another message than the one that says what `live` takes.
    #[test]
    fn live_takes_a_word_or_seconds() {
        let mut settings = Settings::default();
        settings.set("live", "yes").unwrap();
        assert_eq!(settings.live, Some(Ttl::DEFAULT));
        settings.set("Live", "2s").unwrap();
        assert_eq!(settings.live, Some(Ttl::seconds(5)));
        settings.set("live", "off").unwrap();
        assert_eq!(settings.live, None);
        let message = settings.set("live", "soon").unwrap_err();
        assert!(message.starts_with("live: soon: true, false or a number of seconds ("));
    }
}
