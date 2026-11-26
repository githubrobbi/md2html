// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The currency switch: a page's amounts in another currency.
//!
//! The rates are of one day, and the author's. The switch is there only
//! when the author asks for it (`currency` and `rates`); the page shows its
//! own amounts until the reader picks another currency, the one of the
//! language they read in first.

use crate::config::{Currency, LANGS, Rates, Ui};
use crate::text::{esc, json_str_into, put};

/// What the currency switch of a page converts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Money {
    /// The currency the page's amounts are in.
    pub(crate) main: Currency,
    /// The rates to convert them with.
    pub(crate) rates: Rates,
}

impl Money {
    /// The button in the bar and its panel, hidden until the page's script
    /// fills the panel: with no script there is nothing to convert with.
    pub(super) fn button(ui: &Ui) -> String {
        let label = esc(ui.money.label);
        format!(
            "<button id=\"money\" class=\"btn\" type=\"button\" popovertarget=\"moneys\" title=\"{label}\" hidden>\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"2.5\" y=\"6\" width=\"19\" height=\"12\" rx=\"2\"/><circle cx=\"12\" cy=\"12\" r=\"2.6\"/><path d=\"M6 9.5v5M18 9.5v5\"/></svg>\
<span class=\"lc\"></span><span class=\"sr\"> {label}</span></button>\
<div id=\"moneys\" class=\"langs\" popover><p class=\"lh\">{label}</p></div>"
        )
    }

    /// What the page's script converts with, as JSON: the page's currency,
    /// the rates, their day as the page's language writes it, and each
    /// language's own currency.
    pub(super) fn data(&self, ui: &Ui) -> String {
        let mut out = String::from("{\"main\":");
        json_str_into(&mut out, self.main.as_str());
        out.push(',');
        self.rates.json_fields_into(&mut out);
        if let Some(day) = self.rates.date {
            out.push_str(",\"day\":");
            json_str_into(&mut out, &ui.date(day));
        }
        out.push_str(",\"home\":{");
        for (place, lang) in LANGS.iter().enumerate() {
            if place > 0 {
                out.push(',');
            }
            put!(out, "\"{}\":\"{}\"", lang.code, lang.currency);
        }
        out.push_str("}}");
        out
    }
}
