// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The page from its doctype to its last script, written front to back.

use alloc::borrow::Cow;

use super::bar::reading_panel;
use super::csp::{Images, policy_tag};
use super::foot::{Fuller, credit, glossary};
use super::switch::LanguageSwitch;
use super::{
    BASE_STYLE, BEHAVE, CONTENT, CONTENTS, EDGE_BUTTON, FAVICON, FIGURE_SCRIPT, FIGURE_STYLE,
    Found, GLYPH, LARGE, LAYOUT, LIVE, MERMAID_INIT, MONEY, Meta, Money, SCRIPT, Switch, THEME,
    TOGGLE_BUTTON, put_index,
};
use crate::config::{Direction, SearchKeys, Ui};
use crate::glossary::Term;
use crate::render::{Article, SHARE_ICON};
use crate::text::{PathUrl, esc, hex, put, sha256};

/// How many bytes of the page's digest make its fingerprint.
const FINGERPRINT_BYTES: usize = 12;

/// Room in a page for what is neither the article nor its search index:
/// the page's own style and scripts (about 100 kB), the head, the bar, the
/// outline and the glossary.
const SHELL_LEN: usize = 192 * 1024;

/// The version of md2html that writes the page.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One page being written: what it is made of.
pub(super) struct Shell<'page> {
    /// The rendered document.
    article: &'page Article,
    /// What the page says about itself.
    meta: &'page Meta,
    /// The interface words.
    ui: &'static Ui,
    /// The glossary terms the page lists, in the order of the alphabet.
    shown: Vec<&'page Term>,
}

impl<'page> Shell<'page> {
    /// The page for an article. The glossary is shown as a dictionary is:
    /// by the alphabet. The page's own words are all listed; a shared word
    /// only when the prose uses it; none whose entry the article shows
    /// already (a glossary document's).
    pub(super) fn new(article: &'page Article, terms: &'page [Term], meta: &'page Meta) -> Self {
        let mut shown: Vec<&Term> = terms
            .iter()
            .filter(|term| !term.origin.is_shared() || article.minted.contains_key(&term.word))
            .filter(|term| !article.listed.contains(&term.word))
            .collect();
        shown.sort_by_key(|term| term.word.to_lowercase());
        Self {
            article,
            meta,
            ui: meta.ui,
            shown,
        }
    }

    /// The page, whole. Its fingerprint comes first in its head: a live
    /// page compares its own with the first kilobyte of the copy on its
    /// server. The policy, when asked for, follows it and covers what
    /// comes after.
    ///
    /// The page is written into one string: the policy and then the
    /// fingerprint are put in after the opening once what they cover is
    /// there.
    pub(super) fn html(&self) -> String {
        let mut page = self.opening();
        let opening = page.len();
        // The article, its words again in the search index, and the
        // page's own style, scripts and parts around them.
        let words: usize = self
            .article
            .sections
            .iter()
            .map(|section| section.text.len())
            .sum();
        page.reserve(
            self.article
                .out
                .len()
                .saturating_add(words)
                .saturating_add(SHELL_LEN),
        );
        self.rest(&mut page);
        if self.meta.switches.has(Switch::Csp) {
            // mermaid.js writes its own <style> into each drawing: styles
            // may then be inline.
            let policy = policy_tag(
                page.get(opening..).unwrap_or_default(),
                self.meta.mermaid_js.is_some(),
                self.meta.live.is_some(),
                if self.meta.switches.has(Switch::Printed) {
                    Images::Inside
                } else {
                    Images::Anywhere
                },
            );
            page.insert_str(opening, &policy);
        }
        let digest = sha256(page.as_bytes());
        let print: Vec<u8> = digest.into_iter().take(FINGERPRINT_BYTES).collect();
        page.insert_str(
            opening,
            &format!("<meta name=\"md2html-page\" content=\"{}\">", hex(&print)),
        );
        page
    }

    /// The page up to its character set: what the fingerprint and the
    /// policy follow.
    fn opening(&self) -> String {
        let meta = self.meta;
        let dir = match meta.direction {
            Direction::RightToLeft => " dir=\"rtl\"",
            Direction::LeftToRight => "",
        };
        let html_class = if self.has_outline() {
            ""
        } else {
            " class=\"no-toc\""
        };
        let live = meta
            .live
            .map(|live| {
                format!(
                    " data-live=\"{}\" data-ttl=\"{}\"",
                    live.reload.code(),
                    live.ttl
                )
            })
            .unwrap_or_default();
        format!(
            "<!doctype html><html lang=\"{}\"{dir}{html_class}{live}><head><meta charset=\"utf-8\">",
            esc(&meta.lang)
        )
    }

    /// Writes everything after the character set.
    fn rest(&self, out: &mut String) {
        let switch = LanguageSwitch::new(self.meta);
        self.head(out, &switch);
        self.bar(out, &switch);
        self.wrap(out);
        self.find(out);
        self.scripts(out);
    }

    /// The outline's entries: the article's chapters, then the glossary.
    fn contents(&self) -> String {
        let mut contents = self.article.contents.clone();
        if !self.shown.is_empty() {
            put!(
                contents,
                "<li><a href=\"#glossary\">{}</a></li>",
                esc(self.ui.glossary)
            );
        }
        contents
    }

    /// Whether the outline is shown: asked for, and with something in it.
    fn has_outline(&self) -> bool {
        self.meta.switches.has(Switch::Outline)
            && !(self.article.contents.is_empty()
                && self.shown.is_empty()
                && self.meta.contents.is_none())
    }

    /// The handbook's contents, when the outline shows them.
    fn book(&self) -> Option<&super::Contents> {
        self.meta
            .contents
            .as_ref()
            .filter(|_| self.meta.switches.has(Switch::Outline))
    }

    /// The rest of the head: the meta tags, the style, the words the
    /// scripts need, the theme script, the author's head.
    fn head(&self, out: &mut String, switch: &LanguageSwitch<'_>) {
        let meta = self.meta;
        let title = esc(&meta.title);
        let description = esc(&meta.description);
        let robots = if meta.switches.has(Switch::Indexed) {
            ""
        } else {
            "<meta name=\"robots\" content=\"noindex,nofollow\">"
        };
        let author = meta
            .author
            .as_ref()
            .map(|name| format!("<meta name=\"author\" content=\"{}\">", esc(name)))
            .unwrap_or_default();
        put!(
            out,
            "\n<!-- Generated from {} by md2html; edit the Markdown, not this page. -->\n\
<!-- md2html's styles and scripts in this page: MIT OR Apache-2.0. -->\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
{robots}<meta name=\"generator\" content=\"md2html {VERSION}\">\
<title>{title}</title><meta name=\"description\" content=\"{description}\">{author}\
<meta property=\"og:type\" content=\"article\"><meta property=\"og:title\" content=\"{title}\">\
<meta property=\"og:description\" content=\"{description}\">\
<meta name=\"color-scheme\" content=\"light dark\">\
<meta name=\"theme-color\" media=\"(prefers-color-scheme: light)\" content=\"#F6F1EA\">\
<meta name=\"theme-color\" media=\"(prefers-color-scheme: dark)\" content=\"#0F0D0B\">\
<link rel=\"icon\" href=\"{FAVICON}\">{}",
            esc(&meta.source),
            switch.alternates()
        );
        let figure = if self.article.has_figure {
            FIGURE_STYLE
        } else {
            ""
        };
        let large = if meta.switches.has(Switch::LargeType) {
            LARGE
        } else {
            ""
        };
        let contents = if self.book().is_some() { CONTENTS } else { "" };
        put!(
            out,
            "<style>{BASE_STYLE}{LAYOUT}{CONTENT}{figure}{large}{contents}{}{}{}</style>\
<script id=\"md2html-ui\" type=\"application/json\">{}</script>\
<script>{THEME}</script>{}</head>",
            self.article.css,
            meta.css,
            paged(&meta.title),
            self.ui.json(),
            meta.head
        );
    }

    /// The bar: the outline switch, the brand, the search button, the
    /// language switch, the reading panel.
    fn bar(&self, out: &mut String, switch: &LanguageSwitch<'_>) {
        let meta = self.meta;
        let ui = self.ui;
        put!(
            out,
            "<body><a class=\"skip\" href=\"#main\">{}</a><div id=\"bar\"></div>\
<header class=\"bar\"><div class=\"bar-in\">\
{TOGGLE_BUTTON}<a class=\"brand\" href=\"{}\">{GLYPH}{}</a><span class=\"kicker\">{}</span>\
<span class=\"crumb\" id=\"crumb\"></span><span class=\"grow\"></span>\
<button id=\"open-find\" class=\"btn\" type=\"button\" aria-haspopup=\"dialog\" aria-keyshortcuts=\"Meta+K Control+K /\"><svg viewBox=\"0 0 24 24\" aria-hidden=\"true\">\
<circle cx=\"11\" cy=\"11\" r=\"7\"/><path d=\"m20 20-3.5-3.5\"/></svg><span class=\"lbl\">{}</span><kbd aria-hidden=\"true\">&#8984;K</kbd></button>\
{}{}{}</div></header>",
            esc(ui.skip),
            meta.brand_link.attr(),
            meta.brand,
            meta.kicker,
            esc(ui.search),
            meta.money
                .as_ref()
                .map_or_else(String::new, |_| Money::button(ui)),
            switch.button(),
            reading_panel(meta, self.article)
        );
    }

    /// The outline beside the article, and the article with the glossary,
    /// the run's links, the fine print and the credit at its foot.
    fn wrap(&self, out: &mut String) {
        let meta = self.meta;
        let ui = self.ui;
        let fuller = Fuller {
            personal: meta.glossary_url.as_deref().filter(|url| !url.is_empty()),
            handbook: meta.handbook_glossary.as_ref().map(PathUrl::as_str),
        };
        // A page of a handbook shows the handbook's chapters around its own
        // outline: those up to itself above it, the rest below.
        let (class, before, after) = self.book().map_or_else(
            || ("toc", String::new(), String::new()),
            |book| {
                let (before, after) = book.around();
                ("toc hb", before, after)
            },
        );
        put!(
            out,
            "<div class=\"wrap\" id=\"top\">\
<nav class=\"{class}\" id=\"toc\" aria-label=\"{}\">\
<a class=\"dock\" id=\"dock\" href=\"#top\">{}</a>{before}<ol>{}</ol>{after}</nav>{EDGE_BUTTON}\
<main class=\"main\"><article id=\"main\">",
            esc(ui.outline),
            ui.keep_outline,
            self.contents()
        );
        // A translation names its original right under its title.
        match self.article.out.split_once("</header>") {
            Some((hero, body)) if !meta.notice.is_empty() => {
                put!(out, "{hero}</header>{}{body}", meta.notice);
            }
            Some(_) | None => out.push_str(&self.article.out),
        }
        put!(
            out,
            "{}{}<footer class=\"fine\">{}</footer>{}</article></main></div>",
            glossary(self.article, &self.shown, &fuller, ui),
            meta.nav,
            meta.footer,
            credit(ui, VERSION)
        );
    }

    /// The search dialog.
    fn find(&self, out: &mut String) {
        let ui = self.ui;
        let SearchKeys { step, open, close } = &ui.keys;
        let search = esc(ui.search);
        let search_page = esc(ui.search_page);
        put!(
            out,
            "<dialog id=\"find\" aria-label=\"{search}\"><div class=\"find-in\" role=\"search\"><input id=\"q\" type=\"search\" autocomplete=\"off\" \
placeholder=\"{search_page}\" aria-label=\"{search_page}\" role=\"combobox\" aria-expanded=\"false\" aria-controls=\"hits\" aria-autocomplete=\"list\">\
<ol id=\"hits\" role=\"listbox\" aria-label=\"{search}\"></ol><div class=\"find-foot\"><kbd>&uarr;</kbd><kbd>&darr;</kbd> {step} \
<kbd>&#9166;</kbd> {open} <kbd>esc</kbd> {close}<span class=\"grow\"></span><span id=\"hits-n\" role=\"status\" aria-live=\"polite\"></span></div></div></dialog>"
        );
    }

    /// The search index and the scripts, to the end of the page.
    fn scripts(&self, out: &mut String) {
        let meta = self.meta;
        // The share symbol a table row, a figure or a passage shows while
        // the pointer is on it: the page carries it, as a page with no
        // heading has no heading's symbol to borrow.
        put!(
            out,
            "<button id=\"rowshare\" class=\"btn icon\" type=\"button\" hidden>{SHARE_ICON}</button>"
        );
        out.push_str("<script id=\"idx\" type=\"application/json\">");
        put_index(out, self.found());
        out.push_str("</script>");
        if let Some(money) = &meta.money {
            put!(
                out,
                "<script id=\"md2html-money\" type=\"application/json\">{}</script>",
                money.data(self.ui)
            );
        }
        put!(out, "<script>{BEHAVE}</script>");
        if meta.money.is_some() {
            put!(out, "<script>{MONEY}</script>");
        }
        put!(out, "<script>{SCRIPT}</script>");
        if self.article.has_figure {
            put!(out, "<script>{FIGURE_SCRIPT}</script>");
        }
        if let Some(js) = &meta.mermaid_js {
            put!(out, "<script>{js}</script><script>{MERMAID_INIT}</script>");
        }
        if meta.live.is_some() {
            put!(out, "<script>{LIVE}</script>");
        }
        out.push_str("</body></html>");
    }

    /// What the search finds: the glossary's entries, then the article's
    /// sections.
    fn found(&self) -> impl Iterator<Item = Found<'_>> {
        self.shown
            .iter()
            .map(|term| Found {
                id: Cow::Owned(format!("g-{}", term.slug)),
                title: &term.word,
                chapter: self.ui.glossary,
                text: &term.plain,
            })
            .chain(self.article.sections.iter().map(Found::section))
    }
}

/// Printed pages: the title at the head (from the second page on), the
/// page number at the foot.
fn paged(title: &str) -> String {
    format!(
        "@page{{margin:16mm 15mm 18mm;@top-center{{content:\"{}\";font:8.5pt system-ui,sans-serif;color:#7a7066}}\
@bottom-center{{direction:ltr;content:counter(page) \" / \" counter(pages);font:8.5pt system-ui,sans-serif;color:#7a7066}}}}\
@page:first{{@top-center{{content:none}}}}",
        css_string(title)
    )
}

/// Text as the inside of a CSS string: quotes and backslashes escaped,
/// `<` written as its code so it cannot close the style, control
/// characters made spaces.
fn css_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '<' => out.push_str("\\3c "),
            control if control.is_control() => out.push(' '),
            other => out.push(other),
        }
    }
    out
}
