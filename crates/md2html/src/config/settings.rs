// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The settings a page is rendered with, and how a key and its value, from
//! the front matter or the command line, set one of them.

use std::path::PathBuf;

use super::front::FrontMatter;
use super::money::Currency;
use super::reach::Root;
use super::values::{Diagrams, Fingerprint, Translations, Ttl};

/// Why `mermaid_js` is refused in front matter.
const SCRIPT_FROM_COMMAND_LINE: &str = "`mermaid_js` names a script that would run in the page; only the command line gives one (--mermaid-js)";

/// Why `root` is refused in front matter and `--set`.
const ROOT_FROM_COMMAND_LINE: &str = "`root` would widen the folders a document may read from; only the command line gives one (--root)";

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
    /// Join a glossary from outside the page: the handbook's and the
    /// personal one.
    pub(crate) glossary: Option<bool>,
    /// Join the personal shared glossary (`MD2HTML_GLOSSARY`, else the one
    /// in the home folder); no leaves it out and keeps the handbook's.
    pub(crate) personal_glossary: Option<bool>,
    /// A folder whose start page links its pages is a handbook; no on the
    /// start page or the command line renders its pages one by one.
    pub(crate) handbook: Option<bool>,
    /// A page of a handbook no start page links, and meant to be so: no
    /// warning for it.
    pub(crate) orphan: Option<bool>,
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
    /// A folder every document may read from besides its own: command line
    /// only (`--root`; a document cannot widen what it may read).
    pub(crate) root: Option<Root>,
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
    /// The currency the page's amounts are in: with `rates`, the page
    /// offers its reader to see them in another.
    pub(crate) currency: Option<Currency>,
    /// The exchange rates the currency switch converts with, relative to
    /// the Markdown file.
    pub(crate) rates: Option<PathBuf>,
    /// Stylesheets to add, relative to the Markdown file.
    pub(crate) css: Vec<PathBuf>,
    /// HTML files to add to the head, relative to the Markdown file.
    pub(crate) head: Vec<PathBuf>,
}

impl Settings {
    /// Sets one key; an unknown key is reported. Whether the setting had
    /// a value already, from another key that sets it (`lang` and
    /// `language`), is said.
    ///
    /// # Errors
    ///
    /// ``unknown setting `<key>` `` for a key that is neither a setting nor
    /// common front matter md2html passes over (`tags`, `layout`, ...), with
    /// the key as it is looked up (`Brand-Link` reads `brand_link`); a
    /// `mermaid` value that is not native, js or code; a `live` value that
    /// is neither yes, no nor a number of seconds.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> Result<Given, String> {
        let name = key_name(key);
        let text = || Some(value.to_owned());
        let flag = || Some(truthy(value));
        let given = match name.as_str() {
            "title" => give(&mut self.title, text()),
            "lang" | "language" => give(&mut self.lang, text()),
            "brand" | "site" => give(&mut self.brand, text()),
            "brand_link" | "brand_href" | "home" => give(&mut self.brand_link, text()),
            "kicker" => give(&mut self.kicker, text()),
            "eyebrow" => give(&mut self.eyebrow, text()),
            "date" => give(&mut self.date, text()),
            "version" => give(&mut self.version, text()),
            "description" | "summary" => give(&mut self.description, text()),
            "author" | "authors" => give(&mut self.author, text()),
            "footer" => give(&mut self.footer, text()),
            "large" => give(&mut self.large, flag()),
            "glossary" | "shared_glossary" => give(&mut self.glossary, flag()),
            "personal_glossary" => give(&mut self.personal_glossary, flag()),
            "handbook" => give(&mut self.handbook, flag()),
            "orphan" => give(&mut self.orphan, flag()),
            "html" | "raw_html" => give(&mut self.html, flag()),
            "embed" | "embed_images" | "self_contained" => give(&mut self.embed, flag()),
            "csp" => give(&mut self.csp, flag()),
            "pdf" => give(&mut self.pdf, flag()),
            "live" => give(&mut self.live, Ttl::parse(value)?),
            "epub" => give(&mut self.epub, flag()),
            "index" => give(&mut self.index, flag()),
            "noindex" => give(&mut self.index, Some(!truthy(value))),
            "toc" | "outline" => give(&mut self.toc, flag()),
            "math" => give(&mut self.math, flag()),
            "mermaid" | "diagrams" => give(&mut self.mermaid, Some(Diagrams::parse(value)?)),
            // A script runs in the page: only the person running md2html
            // may give one, on the command line (`--mermaid-js`).
            "mermaid_js" => return Err(String::from(SCRIPT_FROM_COMMAND_LINE)),
            // A document may not widen the folders it reads from: only the
            // person running md2html may, with `--root`.
            "root" => return Err(String::from(ROOT_FROM_COMMAND_LINE)),
            "translated_from" | "translation_of" => give(&mut self.translated_from, text()),
            "source_sha256" => give(&mut self.source_sha256, Some(Fingerprint::parse(value))),
            "translations" | "language_switch" => {
                give(&mut self.translations, Some(Translations::parse(value)))
            }
            "currency" => give(&mut self.currency, Some(Currency::parse(value)?)),
            "rates" | "exchange_rates" => give(&mut self.rates, Some(PathBuf::from(value.trim()))),
            // Every stylesheet and head named is used.
            "css" | "stylesheet" => {
                self.css.extend(paths(value));
                Given::First
            }
            "head" => {
                self.head.extend(paths(value));
                Given::First
            }
            // Common front matter that is not the page's business.
            "tags" | "categories" | "layout" | "slug" | "permalink" | "draft" | "keywords"
            | "status" | "updated" | "created" | "translated_by" | "translator" => Given::First,
            other => return Err(format!("unknown setting `{other}`")),
        };
        Ok(given)
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
            personal_glossary: pick(
                self.personal_glossary.as_ref(),
                over.personal_glossary.as_ref(),
            ),
            handbook: pick(self.handbook.as_ref(), over.handbook.as_ref()),
            orphan: pick(self.orphan.as_ref(), over.orphan.as_ref()),
            html: pick(self.html.as_ref(), over.html.as_ref()),
            embed: pick(self.embed.as_ref(), over.embed.as_ref()),
            csp: pick(self.csp.as_ref(), over.csp.as_ref()),
            live: pick(self.live.as_ref(), over.live.as_ref()),
            serve: pick(self.serve.as_ref(), over.serve.as_ref()),
            pdf: pick(self.pdf.as_ref(), over.pdf.as_ref()),
            epub: pick(self.epub.as_ref(), over.epub.as_ref()),
            max_embed: pick(self.max_embed.as_ref(), over.max_embed.as_ref()),
            root: pick(self.root.as_ref(), over.root.as_ref()),
            index: pick(self.index.as_ref(), over.index.as_ref()),
            toc: pick(self.toc.as_ref(), over.toc.as_ref()),
            math: pick(self.math.as_ref(), over.math.as_ref()),
            mermaid: pick(self.mermaid.as_ref(), over.mermaid.as_ref()),
            mermaid_js: pick(self.mermaid_js.as_ref(), over.mermaid_js.as_ref()),
            translated_from: pick(self.translated_from.as_ref(), over.translated_from.as_ref()),
            source_sha256: pick(self.source_sha256.as_ref(), over.source_sha256.as_ref()),
            translations: pick(self.translations.as_ref(), over.translations.as_ref()),
            currency: pick(self.currency.as_ref(), over.currency.as_ref()),
            rates: pick(self.rates.as_ref(), over.rates.as_ref()),
            css,
            head,
        }
    }
}

/// What a first look at a page needs of its settings.
///
/// Before the page is rendered: its title, and what it says of handbooks.
/// Everything else of its front matter, and what is wrong with it, is read
/// when the page is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Glance {
    /// The title it gives, instead of its first heading.
    pub(crate) title: Option<String>,
    /// `handbook`: no renders the folder of this start page one page at a
    /// time.
    pub(crate) handbook: Option<bool>,
    /// `orphan`: yes says that no start page links this page, and that is
    /// meant.
    pub(crate) orphan: Option<bool>,
}

impl Glance {
    /// What `front` says, with the command line's `cli` over it.
    #[must_use]
    pub(crate) fn of(front: &FrontMatter<'_>, cli: &Settings) -> Self {
        Self {
            title: cli
                .title
                .clone()
                .or_else(|| front.get("title").map(str::to_owned)),
            handbook: cli.handbook.or_else(|| front.get("handbook").map(truthy)),
            orphan: cli.orphan.or_else(|| front.get("orphan").map(truthy)),
        }
    }
}

/// Whether a setting had a value before a key set it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Given {
    /// It had none: this is its first.
    First,
    /// It had one, which this replaced.
    Again,
}

/// Puts `value` in `slot`, saying whether it held one.
fn give<Value>(slot: &mut Option<Value>, value: Option<Value>) -> Given {
    let given = if slot.is_some() {
        Given::Again
    } else {
        Given::First
    };
    *slot = value;
    given
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

    /// `root` is not a setting a document gives: it would widen what the
    /// document may read, and only `--root` on the command line does that.
    ///
    /// # Panics
    ///
    /// When `root` is taken, in any spelling, or refused with a message that
    /// does not name `--root`.
    #[test]
    fn root_is_for_the_command_line_only() {
        let mut settings = Settings::default();
        for key in ["root", "Root", " ROOT "] {
            let refused = settings.set(key, "..").unwrap_err();
            assert!(refused.contains("(--root)"), "{key}: {refused}");
        }
        assert_eq!(settings.root, None);
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
