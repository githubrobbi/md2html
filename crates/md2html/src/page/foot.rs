// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What closes the article: the glossary, the links to the run's other
//! pages, the credit.

use super::HOME;
use crate::config::Ui;
use crate::glossary::{Origin, Term};
use crate::render::{Article, SHARE_ICON};
use crate::text::{Html, PathUrl, esc, put};

/// Where the full glossaries a page's terms come from are: the personal
/// shared one (`MD2HTML_GLOSSARY_URL`) and its handbook's glossary page.
pub(super) struct Fuller<'page> {
    /// The personal shared glossary's address, when one joined the page.
    pub(super) personal: Option<&'page str>,
    /// The handbook's glossary page, relative to the page.
    pub(super) handbook: Option<&'page str>,
}

impl Fuller<'_> {
    /// Where the glossary of terms from `origin` is: none for the page's
    /// own.
    const fn of(&self, origin: Origin) -> Option<&str> {
        match origin {
            Origin::Page => None,
            Origin::Handbook => self.handbook,
            Origin::Personal => self.personal,
        }
    }
}

/// The glossary at the foot of the article; `fuller` says where the full
/// glossaries are that its shared terms come from.
pub(super) fn glossary(article: &Article, shown: &[&Term], fuller: &Fuller<'_>, ui: &Ui) -> String {
    if shown.is_empty() {
        return String::new();
    }
    let entries = shown.iter().fold(String::new(), |mut out, term| {
        entry(&mut out, term, article, fuller, ui);
        out
    });
    // The intro names one of them: the handbook's when the page shows one
    // of its terms.
    let from_shared = [Origin::Handbook, Origin::Personal]
        .into_iter()
        .filter(|origin| shown.iter().any(|term| term.origin == *origin))
        .find_map(|origin| fuller.of(origin))
        .map(|url| ui.glossary_shared.fill([&esc(url)]))
        .unwrap_or_default();
    format!(
        "<h2 id=\"glossary\">{}<a class=\"anchor\" href=\"#glossary\" aria-label=\"{}\">{SHARE_ICON}</a></h2>\
<p>{}{from_shared}</p><dl>{entries}</dl>",
        esc(ui.glossary),
        esc(ui.link_section),
        ui.glossary_intro,
    )
}

/// One entry: the term, the way back to its first mention, the way on to
/// the full glossary it comes from, and what it means.
fn entry(out: &mut String, term: &Term, article: &Article, fuller: &Fuller<'_>, ui: &Ui) {
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
    let more = fuller
        .of(term.origin)
        .map(|url| {
            format!(
                "<a class=\"back\" href=\"{}#g-{}\">{}</a>",
                esc(url),
                term.slug,
                esc(ui.all_terms)
            )
        })
        .unwrap_or_default();
    put!(
        out,
        "<dt id=\"g-{}\">{}{back}{more}</dt><dd>{}</dd>",
        term.slug,
        esc(&term.word),
        term.said
    );
}

/// The credit every page carries: what made it, linked to md2html's home.
pub(super) fn credit(ui: &Ui, version: &str) -> String {
    let link = format!(
        "<a href=\"{HOME}\" rel=\"noopener\" title=\"md2html {version} &middot; {}\">md2html</a> <span class=\"ver\">{version}</span>",
        HOME.trim_start_matches("https://")
    );
    format!(
        "<p class=\"credit\">{}</p>",
        ui.credit.fill_around([&link], esc)
    )
}

/// The previous and next page of a run, as links.
pub(crate) fn pages_nav(
    prev: Option<(&PathUrl, &str)>,
    next: Option<(&PathUrl, &str)>,
    ui: &Ui,
) -> Html {
    if prev.is_none() && next.is_none() {
        return Html::default();
    }
    let mut out = format!(
        "<nav class=\"pages\" aria-label=\"{}\">",
        esc(ui.pages.label)
    );
    if let Some((href, title)) = prev {
        put!(
            out,
            "<a class=\"prev\" rel=\"prev\" href=\"{}\"><small>&larr; {}</small>{}</a>",
            href.attr(),
            esc(ui.pages.previous),
            esc(title)
        );
    }
    if let Some((href, title)) = next {
        put!(
            out,
            "<a class=\"next\" rel=\"next\" href=\"{}\"><small>{} &rarr;</small>{}</a>",
            href.attr(),
            esc(ui.pages.next),
            esc(title)
        );
    }
    out.push_str("</nav>");
    Html::markup(out)
}
