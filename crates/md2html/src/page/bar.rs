// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The reading panel in the bar: the text size, the theme, and the ways
//! to keep the page.

use super::{Export, Meta, Switch};
use crate::config::Ui;
use crate::render::Article;
use crate::text::{PathUrl, esc, put};

/// The reading settings (text size, theme) and, beside them, the ways to
/// keep the page: two buttons, each with its own small panel.
pub(super) fn reading_panel(meta: &Meta, article: &Article) -> String {
    let ui = meta.ui;
    format!(
        "<button id=\"aa\" class=\"btn icon\" type=\"button\" popovertarget=\"prefs\" title=\"{prefs}\" aria-label=\"{prefs}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M2.5 18 7.5 5h1l5 13M4.4 13.5h7.2\"/><path d=\"M14.5 18l3-7.5h1l3 7.5M15.6 15.5h4.8\"/></svg></button>\
<div id=\"prefs\" class=\"langs prefs\" popover><p class=\"lh\">{size}</p>{size_buttons}\
<p class=\"lh\">{theme_name}</p><div class=\"themes\" role=\"group\" aria-label=\"{theme_name}\">{system}{light}{dark}</div></div>\
<button id=\"save\" class=\"btn icon\" type=\"button\" popovertarget=\"keep\" title=\"{keep_name}\" aria-label=\"{keep_name}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 4v11M7 10.5l5 5 5-5\"/><path d=\"M5 19h14\"/></svg></button>\
<div id=\"keep\" class=\"langs prefs\" popover><p class=\"lh\">{keep_name}</p><ul class=\"keep\">{keep}</ul></div>",
        prefs = esc(ui.prefs),
        size = esc(ui.size.group),
        size_buttons = size_buttons(ui),
        theme_name = esc(ui.theme.button),
        system = theme_button("", ui.theme.system),
        light = theme_button("light", ui.theme.light),
        dark = theme_button("dark", ui.theme.dark),
        keep_name = esc(ui.export.label),
        keep = keep_list(meta, article),
    )
}

/// One of the theme buttons; `set` is the theme it sets, empty for the
/// system's.
fn theme_button(set: &str, name: &str) -> String {
    format!(
        "<button type=\"button\" data-set=\"{set}\" aria-pressed=\"false\">{}</button>",
        esc(name)
    )
}

/// The ways to keep the page: the page itself, print, and the copies
/// made beside it.
fn keep_list(meta: &Meta, article: &Article) -> String {
    let ui = meta.ui;
    let mut keep = String::new();
    // The page alone reads offline when nothing it shows lives beside it.
    if !meta.file.is_empty()
        && (meta.switches.has(Switch::Embedded) || !article.out.contains("<img"))
    {
        download(&mut keep, &meta.file, ui.export.html);
    }
    put!(
        keep,
        "<li><button type=\"button\" id=\"print\">{}</button></li>",
        esc(ui.export.print)
    );
    for export in &meta.exports {
        match export {
            Export::Pdf(file) => download(&mut keep, file, ui.export.pdf),
            Export::Epub(file) => download(&mut keep, file, ui.export.epub),
        }
    }
    keep
}

/// A link that downloads `file`, a name in the page's folder, named
/// `label`: the link escapes the name as a URL, the download keeps it as
/// the file's name.
fn download(out: &mut String, file: &str, label: &str) {
    put!(
        out,
        "<li><a href=\"{}\" download=\"{}\">{}</a></li>",
        PathUrl::name(file).attr(),
        esc(file),
        esc(label)
    );
}

/// The text-size group: smaller, the size as it is, larger.
fn size_buttons(ui: &Ui) -> String {
    let group = esc(ui.size.group);
    format!(
        "<span class=\"size\" role=\"group\" aria-label=\"{group}\">\
<button id=\"size-down\" class=\"btn icon\" type=\"button\" title=\"{}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M5 12h14\"/></svg></button>\
<button id=\"size\" class=\"btn\" type=\"button\" title=\"{group}\">100%</button>\
<button id=\"size-up\" class=\"btn icon\" type=\"button\" title=\"{}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 5v14M5 12h14\"/></svg></button></span>",
        esc(ui.size.smaller.words),
        esc(ui.size.larger.words)
    )
}
