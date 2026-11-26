// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The currency switch: there only when the author asks for it, with the
//! currency and the rates beside the page, and nothing but data from them.

use super::Scratch;
use crate::config::Settings;
use crate::document::{Rendered, Shape, render_doc};

/// The rates of one day, in yen, as the European Central Bank's are served.
const RATES: &str = "{\"amount\":1.0,\"base\":\"JPY\",\"date\":\"2026-11-26\",\"rates\":{\"EUR\":0.0057212,\"USD\":0.006279}}";

/// `md` written as `budget.md` beside `rates.json` holding `rates`, and
/// rendered.
fn budget(name: &str, md: &str, rates: &str) -> Rendered {
    let scratch = Scratch::new(name);
    scratch.write("rates.json", rates);
    let path = scratch.write("budget.md", md);
    render_doc(
        md,
        Some(&path),
        Some(&scratch.path().join("budget.html")),
        &Settings::default(),
        Shape::Page,
        None,
    )
}

/// Whether one of the page's warnings says `words`.
fn warned(page: &Rendered, words: &str) -> bool {
    page.warnings
        .iter()
        .any(|warning| warning.message.contains(words))
}

/// A page that names its currency and its rates has the switch.
///
/// A button hidden until the script fills it, the rates with their day and
/// each language's currency, and the script, in that order before the
/// page's own.
///
/// # Panics
///
/// When a part of the switch is missing, out of order, shown before the
/// script runs, or the page warns; or when the scratch files cannot be
/// written.
#[test]
fn a_page_with_a_currency_and_rates_has_the_switch() {
    let page = budget(
        "money-on",
        "---\ncurrency: jpy\nrates: rates.json\n---\n# Budget\n\nThe cap is ¥215,378.\n",
        RATES,
    );
    let html = &page.html;
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    assert!(html.contains("<button id=\"money\" class=\"btn\" type=\"button\" popovertarget=\"moneys\" title=\"Currency\" hidden>"));
    assert!(html.contains("<div id=\"moneys\" class=\"langs\" popover>"));
    let data = html
        .find("<script id=\"md2html-money\" type=\"application/json\">{\"main\":\"JPY\",\"base\":\"JPY\",\"date\":\"2026-11-26\",\"rates\":{\"EUR\":0.0057212,\"USD\":0.006279},\"day\":\"26 November 2026\",\"home\":{\"en\":\"USD\",\"de\":\"EUR\",")
        .expect("the rates");
    let script = html.find("md2html-currency").expect("the script");
    let own = html.find("md2html-translate").expect("the page's script");
    assert!(data < script && script < own);
}

/// The switch is asked for, never there by default; asked for in part, or
/// with rates that cannot convert the page's amounts, it is not there and
/// the author is told why.
///
/// # Panics
///
/// When a page has the switch it did not ask for in full, or a page that
/// asked for it in part is not told why; or when the scratch files cannot
/// be written.
#[test]
fn the_switch_is_there_only_when_asked_for_in_full() {
    for (name, front, rates, why) in [
        ("money-none", "", RATES, ""),
        ("money-no-rates", "currency: JPY\n", RATES, "without rates"),
        (
            "money-no-currency",
            "rates: rates.json\n",
            RATES,
            "without the page's currency",
        ),
        (
            "money-gbp",
            "currency: GBP\nrates: rates.json\n",
            RATES,
            "do not convert from GBP",
        ),
        (
            "money-broken",
            "currency: JPY\nrates: rates.json\n",
            "{\"base\":\"JPY\"}",
            "no `rates`",
        ),
        (
            "money-missing",
            "currency: JPY\nrates: elsewhere.json\n",
            RATES,
            "rates: ",
        ),
    ] {
        let md = format!("---\ntitle: B\n{front}---\n# Budget\n\n¥1,000\n");
        let page = budget(name, &md, rates);
        assert!(!page.html.contains("id=\"money\""), "{name}");
        assert!(!page.html.contains("md2html-money"), "{name}");
        assert!(!page.html.contains("md2html-currency"), "{name}");
        assert_eq!(
            why.is_empty(),
            page.warnings.is_empty(),
            "{name}: {:?}",
            page.warnings
        );
        if !why.is_empty() {
            assert!(warned(&page, why), "{name}: {:?}", page.warnings);
        }
    }
}

/// A currency that is not a code of three letters is refused where it is
/// written.
///
/// # Panics
///
/// When `currency: yen` is taken, or refused without saying so.
#[test]
fn a_currency_is_a_code_of_three_letters() {
    let page = budget(
        "money-yen",
        "---\ncurrency: yen!\nrates: rates.json\n---\n# Budget\n",
        RATES,
    );
    assert!(!page.html.contains("id=\"money\""));
    assert!(
        warned(&page, "is not a currency's code"),
        "{:?}",
        page.warnings
    );
}
