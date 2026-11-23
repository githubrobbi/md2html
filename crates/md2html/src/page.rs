// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The page around the article.
//!
//! The stylesheet and scripts it carries, the bar, the outline, the
//! glossary at the foot, the search index, the meta tags, and on request a
//! Content-Security-Policy that lets nothing but the page's own code run.

mod bar;
mod csp;
mod foot;
mod shell;
mod switch;

use alloc::collections::BTreeSet;

pub(crate) use self::foot::pages_nav;
use crate::config::{Direction, Ui};
use crate::glossary::Term;
use crate::render::{Article, Section};
use crate::text::{json_str, put};

/// The search index as JSON.
fn index_json(sections: &[Section]) -> String {
    let mut out = String::from("[");
    for (index, section) in sections.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        put!(
            out,
            "{{\"id\":{},\"t\":{},\"c\":{},\"x\":{}}}",
            json_str(&section.id),
            json_str(&section.title),
            json_str(&section.chapter),
            json_str(&section.text)
        );
    }
    out.push(']');
    out
}

/// A setting of the page that is on or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Switch {
    /// Larger type.
    LargeType,
    /// Search engines may index the page.
    Indexed,
    /// The outline is shown.
    Outline,
    /// A Content-Security-Policy lets only the page's own code run.
    Csp,
    /// Translation on the reader's device is offered where the browser can.
    DeviceTranslation,
    /// The page's images are inside it: the saved page reads offline.
    Embedded,
}

/// The switches that are on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Switches(BTreeSet<Switch>);

impl Switches {
    /// These switches with `switch` turned on or off.
    #[must_use]
    pub(crate) fn with(mut self, switch: Switch, on: bool) -> Self {
        if on {
            self.0.insert(switch);
        } else {
            self.0.remove(&switch);
        }
        self
    }

    /// Whether `switch` is on.
    #[must_use]
    fn has(&self, switch: Switch) -> bool {
        self.0.contains(&switch)
    }
}

/// How a live page learns that it changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Live {
    /// The local server (`--serve`) says so.
    Serve,
    /// The page asks its own server (`--live`).
    Check,
}

impl Live {
    /// The page's `data-live` value, which its script reads.
    const fn code(self) -> &'static str {
        match self {
            Self::Serve => "serve",
            Self::Check => "check",
        }
    }
}

/// A copy made beside the page for reading elsewhere, by its file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Export {
    /// A PDF.
    Pdf(String),
    /// An EPUB.
    Epub(String),
}

/// What the page says about itself, beyond its article.
pub(crate) struct Meta {
    /// The title, plain.
    pub(crate) title: String,
    /// The Markdown file's name.
    pub(crate) source: String,
    /// The settings that are on or off.
    pub(crate) switches: Switches,
    /// Where the full shared glossary is, when one joined this page.
    pub(crate) glossary_url: Option<String>,
    /// The interface words.
    pub(crate) ui: &'static Ui,
    /// The `lang` attribute.
    pub(crate) lang: String,
    /// The name in the bar, escaped.
    pub(crate) brand: String,
    /// Where it leads.
    pub(crate) brand_link: String,
    /// The tag beside it, escaped.
    pub(crate) kicker: String,
    /// The fine print, HTML.
    pub(crate) footer: String,
    /// The meta description, plain.
    pub(crate) description: String,
    /// The author, plain.
    pub(crate) author: Option<String>,
    /// More stylesheet, from the author.
    pub(crate) css: String,
    /// More head, from the author.
    pub(crate) head: String,
    /// The links to the run's other pages, HTML.
    pub(crate) nav: String,
    /// mermaid.js, when a diagram on the page needs it.
    pub(crate) mermaid_js: Option<String>,
    /// Which way the page's language runs.
    pub(crate) direction: Direction,
    /// The page's language versions, itself included; empty for no switch.
    pub(crate) versions: Vec<Version>,
    /// For a translation: the notice that names the original.
    pub(crate) notice: String,
    /// The page's own file name, for "save this page".
    pub(crate) file: String,
    /// A live page, and how it learns it changed; None for a page that
    /// stays as it is.
    pub(crate) live: Option<Live>,
    /// A live page's time to live, in seconds.
    pub(crate) ttl: u32,
    /// Copies made beside the page for reading elsewhere.
    pub(crate) exports: Vec<Export>,
}

/// One language version of a page.
#[derive(Clone, Debug)]
pub(crate) struct Version {
    /// Its language.
    pub(crate) code: String,
    /// Its page, relative to this one.
    pub(crate) href: String,
    /// It is this page.
    pub(crate) current: bool,
    /// It is the original the others translate.
    pub(crate) original: bool,
}

/// Where md2html lives, for the credit at the foot.
const HOME: &str = "https://github.com/githubrobbi/md2html";

/// The whole page.
pub(crate) fn page(article: &Article, terms: &[Term], meta: &Meta) -> String {
    shell::Shell::new(article, terms, meta).html()
}

/// mermaid.js started on the page's diagrams, in the page's theme.
const MERMAID_INIT: &str = "(function(){if(!window.mermaid)return;var r=document.documentElement,t=r.getAttribute('data-theme'),\
d=t==='dark'||(!t&&window.matchMedia&&matchMedia('(prefers-color-scheme: dark)').matches);\
mermaid.initialize({startOnLoad:false,securityLevel:'strict',theme:d?'dark':'neutral'});mermaid.run({querySelector:'pre.mermaid'});})();";

/// The tokens and chrome layer, then the document layer.
pub(crate) const BASE_STYLE: &str = concat!(
    include_str!("page/tokens.css"),
    include_str!("page/style.css")
);
/// The page's layout: outline, hero, progress line, search.
const LAYOUT: &str = include_str!("page/layout.css");
/// Quotes, tasks, footnotes, code colour, maths, print.
pub(crate) const CONTENT: &str = include_str!("page/content.css");
/// A live page: reloads when it changes, keeping the reader's place.
const LIVE: &str = include_str!("page/live.js");
/// The theme switch and the text size.
const THEME: &str = include_str!("page/theme.js");
/// The glossary's way back, the copy buttons, the landing glow.
const BEHAVE: &str = include_str!("page/behave.js");
/// The outline, the crumb, the search.
const SCRIPT: &str = include_str!("page/handbook.js");
/// The brand mark.
const GLYPH: &str = include_str!("page/glyph.svg");
/// The tab's icon: the mark in the accent colour.
const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' \
stroke='%23CE422B' stroke-width='2.6' stroke-linecap='round'%3E%3Cpath d='M3 12h1.5M7 8.5v7M11 5v14M15 8v8M19 10.5v3'/%3E%3C/svg%3E";
/// A figure on the page and enlarged over it.
const FIGURE_STYLE: &str = include_str!("page/figure.css");
/// The enlargement.
const FIGURE_SCRIPT: &str = include_str!("page/figure.js");
/// The larger type (`--large`).
const LARGE: &str = include_str!("page/large.css");

/// The outline switch, first in the bar.
const TOGGLE_BUTTON: &str = "<button id=\"toggle\" class=\"btn icon\" type=\"button\" \
aria-label=\"Outline\" aria-controls=\"toc\" aria-expanded=\"true\">\
<svg class=\"docked\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/>\
<path d=\"M9 4v16\"/><path class=\"pane\" d=\"M4 5h5v14H4z\"/></svg>\
<svg class=\"rail\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/>\
<path d=\"M9 4v16\"/></svg></button>";
/// The rail on the left edge while the outline is folded.
const EDGE_BUTTON: &str = "<button id=\"edge\" type=\"button\" tabindex=\"-1\" \
aria-hidden=\"true\" title=\"Outline\"><svg viewBox=\"0 0 24 24\"><path d=\"m9 6 6 6-6 6\"/></svg></button>";
