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
    /// mermaid.js, when a diagram on the page needs it.
    pub(crate) mermaid_js: Option<String>,
    /// The page's language is written right to left.
    pub(crate) rtl: bool,
    /// The page's language versions, itself included; empty for no switch.
    pub(crate) versions: Vec<Version>,
    /// For a translation: the notice that names the original.
    pub(crate) notice: String,
    /// Offer translation on the reader's device where the browser can.
    pub(crate) device: bool,
    /// The page's own file name, for "save this page".
    pub(crate) file: String,
    /// Its images are inside it: the saved page reads offline.
    pub(crate) embed: bool,
    /// Live: `serve` (the local server says when it changed) or `check`
    /// (the page asks its own server); None for a page that stays as it is.
    pub(crate) live: Option<&'static str>,
    /// A live page's time to live, in seconds.
    pub(crate) ttl: u32,
    /// Copies made beside the page for reading elsewhere: (kind, file), kind `pdf` or `epub`.
    pub(crate) exports: Vec<(&'static str, String)>,
}

/// The share glyph beside a heading (a box with an arrow out of it).
pub(crate) const SHARE_ICON: &str = "<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 3v12M7.5 7.5 12 3l4.5 4.5\"/><path d=\"M8 11H6a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-8a1 1 0 0 0-1-1h-2\"/></svg>";

/// Where md2html lives, for the credit at the foot.
pub(crate) const HOME: &str = "https://github.com/githubrobbi/md2html";

/// The reading settings (text size, theme) and, beside them, the ways to
/// keep the page: two buttons, each with its own small panel.
fn reading_panel(meta: &Meta, article: &Article, size_buttons: &str) -> String {
    let ui = meta.ui;
    let theme = |set: &str, name: &str| {
        format!(
            "<button type=\"button\" data-set=\"{set}\" aria-pressed=\"false\">{}</button>",
            esc(name)
        )
    };
    let mut keep = String::new();
    // The page alone reads offline when nothing it shows lives beside it.
    if !meta.file.is_empty() && (meta.embed || !article.out.contains("<img")) {
        let _ = write!(
            keep,
            "<li><a href=\"{0}\" download=\"{0}\">{1}</a></li>",
            url_attr(&meta.file),
            esc(ui.export[1])
        );
    }
    let _ = write!(
        keep,
        "<li><button type=\"button\" id=\"print\">{}</button></li>",
        esc(ui.export[2])
    );
    for (kind, file) in &meta.exports {
        let label = if *kind == "pdf" {
            ui.export[3]
        } else {
            ui.export[4]
        };
        let _ = write!(
            keep,
            "<li><a href=\"{0}\" download=\"{0}\">{1}</a></li>",
            url_attr(file),
            esc(label)
        );
    }
    format!(
        "<button id=\"aa\" class=\"btn icon\" type=\"button\" popovertarget=\"prefs\" title=\"{prefs}\" aria-label=\"{prefs}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M2.5 18 7.5 5h1l5 13M4.4 13.5h7.2\"/><path d=\"M14.5 18l3-7.5h1l3 7.5M15.6 15.5h4.8\"/></svg></button>\
<div id=\"prefs\" class=\"langs prefs\" popover><p class=\"lh\">{size}</p>{size_buttons}\
<p class=\"lh\">{theme_name}</p><div class=\"themes\" role=\"group\" aria-label=\"{theme_name}\">{a}{b}{c}</div></div>\
<button id=\"save\" class=\"btn icon\" type=\"button\" popovertarget=\"keep\" title=\"{keep_name}\" aria-label=\"{keep_name}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 4v11M7 10.5l5 5 5-5\"/><path d=\"M5 19h14\"/></svg></button>\
<div id=\"keep\" class=\"langs prefs\" popover><p class=\"lh\">{keep_name}</p><ul class=\"keep\">{keep}</ul></div>",
        prefs = esc(ui.prefs),
        size = esc(ui.size[0]),
        theme_name = esc(ui.theme[0]),
        a = theme("", ui.theme[3]),
        b = theme("light", ui.theme[4]),
        c = theme("dark", ui.theme[5]),
        keep_name = esc(ui.export[0]),
    )
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

/// The language switch: the versions there are, then, for the reader's own
/// device to translate, the family's languages and the rest.
fn language_switch(meta: &Meta) -> (String, String) {
    if meta.versions.is_empty() {
        return (String::new(), String::new());
    }
    let ui = meta.ui;
    let code = esc(&meta.lang.to_ascii_uppercase());
    let mut head = String::new();
    let mut items = String::new();
    for v in &meta.versions {
        let name = crate::config::lang_info(&v.code).map_or(v.code.as_str(), |(_, n, _)| n);
        let _ = write!(
            head,
            "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">",
            esc(&v.code),
            url_attr(&v.href)
        );
        let current = if v.current {
            " aria-current=\"page\""
        } else {
            ""
        };
        let original = if v.original {
            format!(" <small>{}</small>", esc(ui.lang[1]))
        } else {
            String::new()
        };
        let _ = write!(
            items,
            "<li><a href=\"{}\" hreflang=\"{}\" lang=\"{}\"{current}>{}</a>{original}</li>",
            url_attr(&v.href),
            esc(&v.code),
            esc(&v.code),
            esc(name)
        );
    }
    let authored: Vec<&str> = meta.versions.iter().map(|v| v.code.as_str()).collect();
    let device = |from: usize, to: usize| {
        crate::config::LANGS[from..to]
            .iter()
            .filter(|(c, _, _)| !authored.contains(c))
            .filter(|(c, _, _)| !matches!(*c, "ja-Hira" | "ja-Latn" | "yue"))
            .fold(String::new(), |mut acc, (c, n, _)| {
                // Chrome's translator names Simplified Chinese plain `zh`.
                let target = if *c == "zh-Hans" { "zh" } else { c };
                let _ = write!(
                    acc,
                    "<li><button type=\"button\" data-lang=\"{target}\" lang=\"{c}\">{}</button></li>",
                    esc(n)
                );
                acc
            })
    };
    let device_html = if meta.device {
        format!(
            "<div class=\"device\" hidden><p class=\"lh\">{}</p><ul>{}</ul><details><summary>{}</summary><ul>{}</ul></details><p class=\"note\">{}</p></div>",
            esc(ui.lang[3]),
            device(0, crate::config::FAMILY),
            esc(ui.lang[8]),
            device(crate::config::FAMILY, crate::config::LANGS.len()),
            esc(ui.lang[4])
        )
    } else {
        String::new()
    };
    let button = format!(
        "<button id=\"lang\" class=\"btn\" type=\"button\" popovertarget=\"langs\" title=\"{}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M3 12h18M12 3c3 3.2 3 14.8 0 18M12 3c-3 3.2-3 14.8 0 18\"/></svg>\
<span class=\"lc\">{code}</span><span class=\"sr\"> {}</span></button>\
<div id=\"langs\" class=\"langs\" popover><p class=\"lh\">{}</p><ul class=\"authored\">{items}</ul>{device_html}</div>",
        esc(ui.lang[0]),
        esc(ui.lang[0]),
        esc(ui.lang[0])
    );
    (head, button)
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
                    .replace(", {n}%", "")
                    .replace("、{n}%", "")
                    .replace("，{n}%", ""))
            ),
        )
        .replace(
            "title=\"Larger text\"",
            &format!(
                "title=\"{}\"",
                esc(&ui.size[2]
                    .replace(" {n}%", "")
                    .replace(", {n} %", "")
                    .replace(", {n}%", "")
                    .replace("、{n}%", "")
                    .replace("，{n}%", ""))
            ),
        )
        .replace(
            "title=\"Text size\"",
            &format!("title=\"{}\"", esc(ui.size[0])),
        );
    let panel = reading_panel(meta, article, &size_buttons);
    // Every page says what made it, linked to md2html's home.
    let credit = format!(
        "<p class=\"credit\">{}</p>",
        esc(ui.credit).replace(
            "{link}",
            &format!(
                "<a href=\"{HOME}\" rel=\"noopener\" title=\"md2html {version} &middot; {}\">md2html</a> <span class=\"ver\">{version}</span>",
                HOME.trim_start_matches("https://")
            )
        )
    );
    // Printed pages: the title at the head (from the second page on), the
    // page number at the foot.
    let running = meta
        .title
        .chars()
        .map(|c| match c {
            '\\' => String::from("\\\\"),
            '"' => String::from("\\\""),
            '<' => String::from("\\3c "),
            c if c.is_control() => String::from(" "),
            c => c.to_string(),
        })
        .collect::<String>();
    let paged = format!(
        "@page{{margin:16mm 15mm 18mm;@top-center{{content:\"{running}\";font:8.5pt system-ui,sans-serif;color:#7a7066}}\
@bottom-center{{direction:ltr;content:counter(page) \" / \" counter(pages);font:8.5pt system-ui,sans-serif;color:#7a7066}}}}\
@page:first{{@top-center{{content:none}}}}"
    );
    let [key_move, key_open, key_close] = ui.keys;
    let (alternates, switch) = language_switch(meta);
    let dir = if meta.rtl { " dir=\"rtl\"" } else { "" };
    let body = if meta.notice.is_empty() {
        article.out.clone()
    } else {
        article
            .out
            .replacen("</header>", &format!("</header>{}", meta.notice), 1)
    };
    let page = format!(
        "<!doctype html><html lang=\"{lang}\"{dir}{html_class}{live_attr}><head><meta charset=\"utf-8\">\
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
<link rel=\"icon\" href=\"{FAVICON}\">{alternates}\
<style>{BASE_STYLE}{LAYOUT}{CONTENT}{figure_style}{large}{article_css}{user_css}{paged}</style>\
<script id=\"md2html-ui\" type=\"application/json\">{ui_json}</script>\
<script>{THEME}</script>{head}</head><body>\
<a class=\"skip\" href=\"#main\">{skip}</a><div id=\"bar\"></div>\
<header class=\"bar\"><div class=\"bar-in\">\
{TOGGLE_BUTTON}<a class=\"brand\" href=\"{brand_link}\">{GLYPH}{brand}</a><span class=\"kicker\">{kicker}</span>\
<span class=\"crumb\" id=\"crumb\"></span><span class=\"grow\"></span>\
<button id=\"open-find\" class=\"btn\" type=\"button\" aria-haspopup=\"dialog\" aria-keyshortcuts=\"Meta+K Control+K /\"><svg viewBox=\"0 0 24 24\" aria-hidden=\"true\">\
<circle cx=\"11\" cy=\"11\" r=\"7\"/><path d=\"m20 20-3.5-3.5\"/></svg><span class=\"lbl\">{search}</span><kbd aria-hidden=\"true\">&#8984;K</kbd></button>\
{switch}{panel}</div></header>\
<div class=\"wrap\" id=\"top\">\
<nav class=\"toc\" id=\"toc\" aria-label=\"{outline}\">\
<a class=\"dock\" id=\"dock\" href=\"#top\">{keep}</a><ol>{contents}</ol></nav>{EDGE_BUTTON}\
<main class=\"main\"><article id=\"main\">{body}{glossary}{nav}\
<footer class=\"fine\">{footer}</footer>{credit}</article></main></div>\
<dialog id=\"find\" aria-label=\"{search}\"><div class=\"find-in\" role=\"search\"><input id=\"q\" type=\"search\" autocomplete=\"off\" \
placeholder=\"{search_page}\" aria-label=\"{search_page}\" role=\"combobox\" aria-expanded=\"false\" aria-controls=\"hits\" aria-autocomplete=\"list\">\
<ol id=\"hits\" role=\"listbox\" aria-label=\"{search}\"></ol><div class=\"find-foot\"><kbd>&uarr;</kbd><kbd>&darr;</kbd> {key_move} \
<kbd>&#9166;</kbd> {key_open} <kbd>esc</kbd> {key_close}<span class=\"grow\"></span><span id=\"hits-n\" role=\"status\" aria-live=\"polite\"></span></div></div></dialog>\
<script id=\"idx\" type=\"application/json\">{index}</script>\
<script>{BEHAVE}</script><script>{SCRIPT}</script>{figure_script}{mermaid_script}{live_script}</body></html>",
        live_attr = meta
            .live
            .map(|m| format!(" data-live=\"{m}\" data-ttl=\"{}\"", meta.ttl))
            .unwrap_or_default(),
        live_script = if meta.live.is_some() {
            format!("<script>{LIVE}</script>")
        } else {
            String::new()
        },
        lang = esc(&meta.lang),
        brand = meta.brand,
        kicker = meta.kicker,
        skip = esc(ui.skip),
        search = esc(ui.search),
        search_page = esc(ui.search_page),
        outline = esc(ui.outline),
        keep = ui.keep_outline,
        body = body,
        nav = meta.nav,
        mermaid_script = meta
            .mermaid_js
            .as_ref()
            .map(|js| format!("<script>{js}</script><script>{MERMAID_INIT}</script>"))
            .unwrap_or_default(),
        footer = meta.footer,
        article_css = article.css,
        user_css = meta.css,
        head = meta.head,
    );
    let page = if meta.csp {
        // mermaid.js writes its own <style> into each drawing: styles may then be inline.
        with_csp(&page, meta.mermaid_js.is_some(), meta.live.is_some())
    } else {
        page
    };
    // The page's fingerprint, first in its head: a live page compares its
    // own with the first kilobyte of the copy on its server.
    let print = crate::text::hex(&sha256(page.as_bytes())[..12]);
    page.replacen(
        "<meta charset=\"utf-8\">",
        &format!("<meta charset=\"utf-8\"><meta name=\"md2html-page\" content=\"{print}\">"),
        1,
    )
}

/// The page with a Content-Security-Policy that allows exactly its own
/// inline styles and scripts, by their hashes, and no connection out.
pub(crate) fn with_csp(page: &str, inline_styles: bool, live: bool) -> String {
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
connect-src {}; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'",
        if scripts.is_empty() {
            String::from("'none'")
        } else {
            scripts.join(" ")
        },
        if inline_styles {
            String::from("'unsafe-inline'")
        } else if styles.is_empty() {
            String::from("'none'")
        } else {
            styles.join(" ")
        },
        // A live page asks only its own server whether it changed.
        if live { "'self'" } else { "'none'" }
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
        "<h2 id=\"glossary\">{}<a class=\"anchor\" href=\"#glossary\" aria-label=\"{}\">{share}</a></h2>\
<p>{}{from_shared}</p><dl>{entries}</dl>",
        esc(ui.glossary),
        esc(ui.link_section),
        ui.glossary_intro,
        share = SHARE_ICON
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
