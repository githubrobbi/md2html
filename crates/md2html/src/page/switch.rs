// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The language switch: the versions there are, then, for the reader's own
//! device to translate, the family's languages and the rest.

use super::{Meta, Switch, Version};
use crate::config::{FAMILY, LANGS, Lang, Ui, lang_info};
use crate::text::{esc, put, url_attr};

/// Languages a browser's translator does not offer: they are not listed
/// for translation on the device.
const NOT_ON_DEVICE: [&str; 3] = ["ja-Hira", "ja-Latn", "yue"];

/// The language switch of one page; empty for a page with no versions.
pub(super) struct LanguageSwitch<'page> {
    /// The page.
    meta: &'page Meta,
}

impl<'page> LanguageSwitch<'page> {
    /// The switch of a page.
    pub(super) const fn new(meta: &'page Meta) -> Self {
        Self { meta }
    }

    /// The links in the head that name each version.
    pub(super) fn alternates(&self) -> String {
        self.meta
            .versions
            .iter()
            .fold(String::new(), |mut out, version| {
                put!(
                    out,
                    "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">",
                    esc(&version.code),
                    url_attr(&version.href)
                );
                out
            })
    }

    /// The button in the bar and its panel.
    pub(super) fn button(&self) -> String {
        let meta = self.meta;
        if meta.versions.is_empty() {
            return String::new();
        }
        let ui = meta.ui;
        let items = meta
            .versions
            .iter()
            .fold(String::new(), |mut out, version| {
                version_item(&mut out, version, ui);
                out
            });
        let device = if meta.switches.has(Switch::DeviceTranslation) {
            self.device_panel()
        } else {
            String::new()
        };
        let label = esc(ui.lang.label);
        format!(
            "<button id=\"lang\" class=\"btn\" type=\"button\" popovertarget=\"langs\" title=\"{label}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M3 12h18M12 3c3 3.2 3 14.8 0 18M12 3c-3 3.2-3 14.8 0 18\"/></svg>\
<span class=\"lc\">{}</span><span class=\"sr\"> {label}</span></button>\
<div id=\"langs\" class=\"langs\" popover><p class=\"lh\">{label}</p><ul class=\"authored\">{items}</ul>{device}</div>",
            esc(&meta.lang.to_ascii_uppercase())
        )
    }

    /// The languages the reader's device may translate into: the family's
    /// first, the rest under "more".
    fn device_panel(&self) -> String {
        let ui = self.meta.ui;
        format!(
            "<div class=\"device\" hidden><p class=\"lh\">{}</p><ul>{}</ul><details><summary>{}</summary><ul>{}</ul></details><p class=\"note\">{}</p></div>",
            esc(ui.lang.on_device),
            self.device_items(LANGS.iter().take(FAMILY)),
            esc(ui.lang.more),
            self.device_items(LANGS.iter().skip(FAMILY)),
            esc(ui.lang.machine)
        )
    }

    /// A button for each of `langs` the page has no version in and the
    /// device can translate into.
    fn device_items(&self, langs: impl Iterator<Item = &'static Lang>) -> String {
        langs
            .filter(|lang| {
                !self
                    .meta
                    .versions
                    .iter()
                    .any(|version| version.code == lang.code)
            })
            .filter(|lang| !NOT_ON_DEVICE.contains(&lang.code))
            .fold(String::new(), |mut out, lang| {
                put!(
                    out,
                    "<li><button type=\"button\" data-lang=\"{}\" lang=\"{}\">{}</button></li>",
                    translator_code(lang.code),
                    lang.code,
                    esc(lang.name)
                );
                out
            })
    }
}

/// The code a browser's translator knows a language by: Chrome's names
/// Simplified Chinese plain `zh`.
fn translator_code(code: &str) -> &str {
    if code == "zh-Hans" { "zh" } else { code }
}

/// One version in the switch's list, by its own name.
fn version_item(out: &mut String, version: &Version, ui: &Ui) {
    let name = lang_info(&version.code).map_or(version.code.as_str(), |lang| lang.name);
    let current = if version.current {
        " aria-current=\"page\""
    } else {
        ""
    };
    let original = if version.original {
        format!(" <small>{}</small>", esc(ui.lang.original))
    } else {
        String::new()
    };
    put!(
        out,
        "<li><a href=\"{}\" hreflang=\"{}\" lang=\"{}\"{current}>{}</a>{original}</li>",
        url_attr(&version.href),
        esc(&version.code),
        esc(&version.code),
        esc(name)
    );
}
