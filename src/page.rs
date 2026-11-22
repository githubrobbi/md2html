// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The page around the article: the stylesheet and scripts it carries,
//! the bar, the outline, the glossary at the foot, the search index, the
//! meta tags, and on request a Content-Security-Policy that lets nothing
//! but the page's own code run.

use core::fmt::Write as _;

use crate::config::Ui;
use crate::glossary::Term;
use crate::render::{Article, Section};
use crate::text::{base64, esc, sha256, url_attr};

/// A string as a JSON literal, `<` escaped so it cannot close a script.
pub(crate) fn json_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' | '\t' => out.push(' '),
            '<' => out.push_str("\\u003c"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other if (other as u32) < 0x20 => {}
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// The search index as JSON.
pub(crate) fn index_json(sections: &[Section]) -> String {
    let mut out = String::from("[");
    for (index, section) in sections.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
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

/// What the page says about itself, beyond its article.
pub(crate) struct Meta {
    /// The title, plain.
    pub(crate) title: String,
    /// The Markdown file's name.
    pub(crate) source: String,
    /// Larger type.
    pub(crate) large: bool,
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
    /// Let search engines index it.
    pub(crate) index: bool,
    /// Show the outline.
    pub(crate) toc: bool,
    /// Add a Content-Security-Policy.
    pub(crate) csp: bool,
    /// More stylesheet, from the author.
    pub(crate) css: String,
    /// More head, from the author.
    pub(crate) head: String,
    /// The links to the run's other pages, HTML.
    pub(crate) nav: String,
}

/// The whole page.
pub(crate) fn page(article: &Article, terms: &[Term], meta: &Meta) -> String {
    let ui = meta.ui;
    // Shown as a dictionary is: by the alphabet. The page's own words are
    // all listed; a shared word only when the prose uses it.
    let mut shown: Vec<&Term> = terms
        .iter()
        .filter(|term| !term.shared || article.minted.contains_key(&term.word))
        .collect();
    shown.sort_by_key(|term| term.word.to_lowercase());
    let url = meta.glossary_url.as_deref().unwrap_or_default();
    let glossary = glossary(article, &shown, url, ui);
    let title = esc(&meta.title);
    let source = esc(&meta.source);
    let large = if meta.large { LARGE } else { "" };
    let (figure_style, figure_script) = if article.has_figure {
        (FIGURE_STYLE, format!("<script>{FIGURE_SCRIPT}</script>"))
    } else {
        ("", String::new())
    };
    let mut sections: Vec<Section> = shown
        .iter()
        .map(|term| Section {
            id: format!("g-{}", term.slug),
            title: term.word.clone(),
            chapter: ui.glossary.to_owned(),
            text: term.plain.clone(),
        })
        .collect();
    sections.extend(article.sections.iter().cloned());
    let index = index_json(&sections);
    let mut contents = article.contents.clone();
    if !shown.is_empty() {
        let _ = write!(
            contents,
            "<li><a href=\"#glossary\">{}</a></li>",
            esc(ui.glossary)
        );
    }
    let no_toc = !meta.toc || contents.is_empty();
    let html_class = if no_toc { " class=\"no-toc\"" } else { "" };
    let robots = if meta.index {
        ""
    } else {
        "<meta name=\"robots\" content=\"noindex,nofollow\">"
    };
    let description = esc(&meta.description);
    let author = meta
        .author
        .as_ref()
        .map(|a| format!("<meta name=\"author\" content=\"{}\">", esc(a)))
        .unwrap_or_default();
    let version = env!("CARGO_PKG_VERSION");
    let ui_json = ui.json();
    let brand_link = url_attr(&meta.brand_link);
    let theme_button = THEME_BUTTON.replace(
        "title=\"Theme\"",
        &format!("title=\"{}\"", esc(ui.theme[0])),
    );
    let size_buttons = SIZE_BUTTONS
        .replace(
            "aria-label=\"Text size\"",
            &format!("aria-label=\"{}\"", esc(ui.size[0])),
        )
        .replace(
            "title=\"Smaller text\"",
            &format!(
                "title=\"{}\"",
                esc(&ui.size[1]
                    .replace(" {n}%", "")
                    .replace(", {n} %", "")
                    .replace(", {n}%", ""))
            ),
        )
        .replace(
            "title=\"Larger text\"",
            &format!(
                "title=\"{}\"",
                esc(&ui.size[2]
                    .replace(" {n}%", "")
                    .replace(", {n} %", "")
                    .replace(", {n}%", ""))
            ),
        )
        .replace(
            "title=\"Text size\"",
            &format!("title=\"{}\"", esc(ui.size[0])),
        );
    let [key_move, key_open, key_close] = ui.keys;
    let page = format!(
        "<!doctype html><html lang=\"{lang}\"{html_class}><head><meta charset=\"utf-8\">\
\n<!-- Generated from {source} by md2html; edit the Markdown, not this page. -->\n\
<!-- md2html's styles and scripts in this page: MIT OR Apache-2.0. -->\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
{robots}<meta name=\"generator\" content=\"md2html {version}\">\
<title>{title}</title><meta name=\"description\" content=\"{description}\">{author}\
<meta property=\"og:type\" content=\"article\"><meta property=\"og:title\" content=\"{title}\">\
<meta property=\"og:description\" content=\"{description}\">\
<meta name=\"color-scheme\" content=\"light dark\">\
<meta name=\"theme-color\" media=\"(prefers-color-scheme: light)\" content=\"#F6F1EA\">\
<meta name=\"theme-color\" media=\"(prefers-color-scheme: dark)\" content=\"#0F0D0B\">\
<link rel=\"icon\" href=\"{FAVICON}\">\
<style>{BASE_STYLE}{LAYOUT}{CONTENT}{figure_style}{large}{article_css}{user_css}</style>\
<script id=\"md2html-ui\" type=\"application/json\">{ui_json}</script>\
<script>{THEME}</script>{head}</head><body>\
<a class=\"skip\" href=\"#main\">{skip}</a><div id=\"bar\"></div>\
<header class=\"bar\"><div class=\"bar-in\">\
{TOGGLE_BUTTON}<a class=\"brand\" href=\"{brand_link}\">{GLYPH}{brand}</a><span class=\"kicker\">{kicker}</span>\
<span class=\"crumb\" id=\"crumb\"></span><span class=\"grow\"></span>\
<button id=\"open-find\" class=\"btn\" type=\"button\" aria-haspopup=\"dialog\" aria-label=\"{search}\" aria-keyshortcuts=\"Meta+K Control+K /\"><svg viewBox=\"0 0 24 24\" aria-hidden=\"true\">\
<circle cx=\"11\" cy=\"11\" r=\"7\"/><path d=\"m20 20-3.5-3.5\"/></svg><span class=\"lbl\">{search}</span><kbd>&#8984;K</kbd></button>\
{size_buttons}{theme_button}</div></header>\
<div class=\"wrap\" id=\"top\">\
<nav class=\"toc\" id=\"toc\" aria-label=\"{outline}\">\
<a class=\"dock\" id=\"dock\" href=\"#top\">{keep}</a><ol>{contents}</ol></nav>{EDGE_BUTTON}\
<main class=\"main\"><article id=\"main\">{body}{glossary}{nav}\
<footer class=\"fine\">{footer}</footer></article></main></div>\
<dialog id=\"find\" aria-label=\"{search}\"><div class=\"find-in\" role=\"search\"><input id=\"q\" type=\"search\" autocomplete=\"off\" \
placeholder=\"{search_page}\" aria-label=\"{search_page}\" role=\"combobox\" aria-expanded=\"false\" aria-controls=\"hits\" aria-autocomplete=\"list\">\
<ol id=\"hits\" role=\"listbox\" aria-label=\"{search}\"></ol><div class=\"find-foot\"><kbd>&uarr;</kbd><kbd>&darr;</kbd> {key_move} \
<kbd>&#9166;</kbd> {key_open} <kbd>esc</kbd> {key_close}<span class=\"grow\"></span><span id=\"hits-n\" role=\"status\" aria-live=\"polite\"></span></div></div></dialog>\
<script id=\"idx\" type=\"application/json\">{index}</script>\
<script>{BEHAVE}</script><script>{SCRIPT}</script>{figure_script}</body></html>",
        lang = esc(&meta.lang),
        brand = meta.brand,
        kicker = meta.kicker,
        skip = esc(ui.skip),
        search = esc(ui.search),
        search_page = esc(ui.search_page),
        outline = esc(ui.outline),
        keep = ui.keep_outline,
        body = article.out,
        nav = meta.nav,
        footer = meta.footer,
        article_css = article.css,
        user_css = meta.css,
        head = meta.head,
    );
    if meta.csp { with_csp(&page) } else { page }
}

/// The page with a Content-Security-Policy that allows exactly its own
/// inline styles and scripts, by their hashes, and no connection out.
pub(crate) fn with_csp(page: &str) -> String {
    let mut scripts = Vec::new();
    let mut styles = Vec::new();
    let mut rest = page;
    while let Some(at) = rest.find('<') {
        rest = &rest[at..];
        let (list, close) = if rest.starts_with("<script") {
            (&mut scripts, "</script>")
        } else if rest.starts_with("<style") {
            (&mut styles, "</style>")
        } else {
            rest = &rest[1..];
            continue;
        };
        let Some(open_end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..open_end];
        let body_start = open_end + 1;
        let Some(len) = rest[body_start..].find(close) else {
            break;
        };
        let body = &rest[body_start..body_start + len];
        let data_only = tag.contains("application/json") || tag.contains("application/ld+json");
        if !data_only {
            list.push(format!("'sha256-{}'", base64(&sha256(body.as_bytes()))));
        }
        rest = &rest[body_start + len + close.len()..];
    }
    scripts.dedup();
    styles.dedup();
    let policy = format!(
        "default-src 'none'; script-src {}; style-src {}; img-src * data: blob:; media-src * data: blob:; font-src data:; \
connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'",
        if scripts.is_empty() {
            String::from("'none'")
        } else {
            scripts.join(" ")
        },
        if styles.is_empty() {
            String::from("'none'")
        } else {
            styles.join(" ")
        }
    );
    let meta = format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\">");
    page.replacen(
        "<meta charset=\"utf-8\">",
        &format!("<meta charset=\"utf-8\">{meta}"),
        1,
    )
}

/// The glossary at the foot of the article.
fn glossary(article: &Article, shown: &[&Term], url: &str, ui: &Ui) -> String {
    if shown.is_empty() {
        return String::new();
    }
    let entries = shown.iter().fold(String::new(), |mut out, term| {
        let back = if article.minted.contains_key(&term.word) {
            format!(
                "<a class=\"back\" data-back=\"{}\" href=\"#t-{}-1\">{}</a>",
                term.slug,
                term.slug,
                esc(ui.back)
            )
        } else {
            String::new()
        };
        let more = if term.shared && !url.is_empty() {
            format!(
                "<a class=\"back\" href=\"{}#g-{}\">{}</a>",
                esc(url),
                term.slug,
                esc(ui.all_terms)
            )
        } else {
            String::new()
        };
        let _ = write!(
            out,
            "<dt id=\"g-{}\">{}{back}{more}</dt><dd>{}</dd>",
            term.slug,
            esc(&term.word),
            term.said
        );
        out
    });
    let from_shared = if shown.iter().any(|term| term.shared) && !url.is_empty() {
        ui.glossary_shared.replace("{url}", &esc(url))
    } else {
        String::new()
    };
    format!(
        "<h2 id=\"glossary\">{}<a class=\"anchor\" href=\"#glossary\" aria-label=\"{}\">#</a></h2>\
<p>{}{from_shared}</p><dl>{entries}</dl>",
        esc(ui.glossary),
        esc(ui.link_section),
        ui.glossary_intro
    )
}

/// The previous and next page of a run, as links.
pub(crate) fn pages_nav(prev: Option<(&str, &str)>, next: Option<(&str, &str)>, ui: &Ui) -> String {
    if prev.is_none() && next.is_none() {
        return String::new();
    }
    let mut out = format!("<nav class=\"pages\" aria-label=\"{}\">", esc(ui.pages[2]));
    if let Some((href, title)) = prev {
        let _ = write!(
            out,
            "<a class=\"prev\" rel=\"prev\" href=\"{}\"><small>&larr; {}</small>{}</a>",
            url_attr(href),
            esc(ui.pages[0]),
            esc(title)
        );
    }
    if let Some((href, title)) = next {
        let _ = write!(
            out,
            "<a class=\"next\" rel=\"next\" href=\"{}\"><small>{} &rarr;</small>{}</a>",
            url_attr(href),
            esc(ui.pages[1]),
            esc(title)
        );
    }
    out.push_str("</nav>");
    out
}

/// The tokens and chrome layer, then the document layer.
pub(crate) const BASE_STYLE: &str = concat!(
    include_str!("page/tokens.css"),
    include_str!("page/style.css")
);
/// The page's layout: outline, hero, progress line, search.
const LAYOUT: &str = include_str!("page/layout.css");
/// Quotes, tasks, footnotes, code colour, maths, print.
const CONTENT: &str = include_str!("page/content.css");
/// The theme switch and the text size.
pub(crate) const THEME: &str = include_str!("page/theme.js");
/// The glossary's way back, the copy buttons, the landing glow.
const BEHAVE: &str = include_str!("page/behave.js");
/// The outline, the crumb, the search.
const SCRIPT: &str = include_str!("page/handbook.js");
/// The brand mark.
pub(crate) const GLYPH: &str = include_str!("page/glyph.svg");
/// The tab's icon: the mark in the accent colour.
const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' \
stroke='%23CE422B' stroke-width='2.6' stroke-linecap='round'%3E%3Cpath d='M3 12h1.5M7 8.5v7M11 5v14M15 8v8M19 10.5v3'/%3E%3C/svg%3E";
/// A figure on the page and enlarged over it.
const FIGURE_STYLE: &str = include_str!("page/figure.css");
/// The enlargement.
const FIGURE_SCRIPT: &str = include_str!("page/figure.js");
/// The larger type (`--large`).
const LARGE: &str = include_str!("page/large.css");

/// The switch button.
pub(crate) const THEME_BUTTON: &str = "<button id=\"theme\" class=\"btn icon\" type=\"button\" \
title=\"Theme\">\
<svg class=\"sun\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"4\"/>\
<path d=\"M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4\"/></svg>\
<svg class=\"moon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z\"/></svg>\
<svg class=\"auto\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"12\" rx=\"2\"/>\
<path d=\"M8 20h8M12 16v4\"/></svg></button>";
/// The outline switch, first in the bar.
pub(crate) const TOGGLE_BUTTON: &str = "<button id=\"toggle\" class=\"btn icon\" type=\"button\" \
aria-label=\"Outline\" aria-controls=\"toc\" aria-expanded=\"true\">\
<svg class=\"docked\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/>\
<path d=\"M9 4v16\"/><path class=\"pane\" d=\"M4 5h5v14H4z\"/></svg>\
<svg class=\"rail\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/>\
<path d=\"M9 4v16\"/></svg></button>";
/// The rail on the left edge while the outline is folded.
pub(crate) const EDGE_BUTTON: &str = "<button id=\"edge\" type=\"button\" tabindex=\"-1\" \
aria-hidden=\"true\" title=\"Outline\"><svg viewBox=\"0 0 24 24\"><path d=\"m9 6 6 6-6 6\"/></svg></button>";
/// The text-size group.
pub(crate) const SIZE_BUTTONS: &str = "<span class=\"size\" role=\"group\" aria-label=\"Text size\">\
<button id=\"size-down\" class=\"btn icon\" type=\"button\" title=\"Smaller text\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M5 12h14\"/></svg></button>\
<button id=\"size\" class=\"btn\" type=\"button\" title=\"Text size\">100%</button>\
<button id=\"size-up\" class=\"btn icon\" type=\"button\" title=\"Larger text\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 5v14M5 12h14\"/></svg></button></span>";
