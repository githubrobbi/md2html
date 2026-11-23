// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page in its languages: the versions beside it, the switch between
//! them, the interface words and the direction of each language.

use std::path::Path;

use super::{Scratch, own_glossary_only, page, page_with};
use crate::config::{Direction, Settings, direction, ui};
use crate::{Rendered, render_doc};

/// A page written to a scratch folder, rendered from there.
///
/// # Panics
///
/// When the Markdown file `name` cannot be read from `dir`.
fn render_file(dir: &Path, name: &str) -> Rendered {
    let path = dir.join(name);
    let text = std::fs::read_to_string(&path).expect("written");
    let out = path.with_extension("html");
    render_doc(
        &text,
        Some(&path),
        Some(&out),
        &own_glossary_only(),
        false,
        None,
    )
}

/// Translations beside a page are linked from it and checked against it.
///
/// # Panics
///
/// When the original does not name each version (`en`, `he`, `fr`) as an
/// alternate, has no language switch in its bar, does not mark itself in
/// the switch, or offers the device's translation for a language that has a
/// version; when the Hebrew page is not right to left, has no notice that
/// it is a translation, or lacks its Hebrew callout word and reading guide;
/// when the Hebrew version, whose digest matches, is warned about; when the
/// French version, whose digest does not, is not called older than its
/// original or has no French search field; or when the scratch files cannot
/// be written or read.
#[test]
fn translations_beside_a_page_are_linked_and_checked() {
    let scratch = Scratch::new("i18n");
    let original = "# Gardens\n\nA page.\n\n## 1. Roses\n\nSun.\n";
    scratch.write("page.md", original);
    let digest = crate::text::hex(&crate::text::sha256(original.as_bytes()));
    scratch.write(
        "page.he.md",
        format!("---\ntranslated_from: page.md\nsource_sha256: {digest}\n---\n# גינות\n\nדף.\n\n## איך לקרוא את זה\n\n- **פרק 1** ורדים.\n\n## 1. ורדים\n\n> **הערה:** שמש.\n"),
    );
    scratch.write(
        "page.fr.md",
        "---\ntranslated_from: page.md\nsource_sha256: 00\n---\n# Jardins\n\nUne page.\n\n## 1. Roses\n\nSoleil.\n",
    );
    let en = render_file(scratch.path(), "page.md");
    for code in ["en", "he", "fr"] {
        assert!(
            en.html
                .contains(&format!("<link rel=\"alternate\" hreflang=\"{code}\"")),
            "{code}"
        );
    }
    assert!(
        en.html.contains("id=\"lang\"") && en.html.contains("popovertarget=\"langs\""),
        "the switch is in the bar"
    );
    assert!(
        en.html.contains("<a href=\"page.html\" hreflang=\"en\" lang=\"en\" aria-current=\"page\">English</a> <small>"),
        "the page itself, marked, and the original"
    );
    assert!(
        !en.html.contains("data-lang=\"he\"")
            && en.html.contains("data-lang=\"zh\" lang=\"zh-Hans\""),
        "the device offers only languages without a version"
    );
    let he = render_file(scratch.path(), "page.he.md");
    assert!(he.html.contains("<html lang=\"he\" dir=\"rtl\""));
    assert!(
        he.html.contains("<p class=\"translated\">זהו תרגום."),
        "the notice names the original"
    );
    assert!(
        he.html.contains("<span class=\"mark\">הערה</span>"),
        "a Hebrew callout word"
    );
    assert!(
        he.html.contains("class=\"card\" href=\"#1-ורדים\""),
        "a Hebrew reading guide"
    );
    let he_warnings: Vec<&str> = he
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(he_warnings.is_empty(), "{he_warnings:?}");
    let fr = render_file(scratch.path(), "page.fr.md");
    let fr_warnings: Vec<&str> = fr
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(
        fr_warnings
            .iter()
            .any(|message| message.contains("older than")),
        "{fr_warnings:?}"
    );
    assert!(fr.html.contains("Rechercher dans cette page"));
}

/// A page with no translation beside it has no language switch.
///
/// # Panics
///
/// When the page shows the switch or names an alternate language.
#[test]
fn a_page_without_translations_has_no_switch() {
    let html = page("# Alone\n\nOnly English.\n");
    assert!(!html.contains("id=\"lang\""));
    assert!(!html.contains("rel=\"alternate\" hreflang"));
}

/// Every interface language writes its dates, words and direction.
///
/// # Panics
///
/// When a language's date of 22 November 2026 lacks the day or the year,
/// when its page does not say its language or lacks its word for a note or
/// its search field, or when Hebrew and Yiddish are not right to left and
/// Low German left to right.
#[test]
fn every_interface_language_has_its_dates_and_words() {
    for code in [
        "en", "de", "he", "id", "nl", "jv", "yi", "nds", "fr", "it", "es",
    ] {
        let words = ui(code);
        let date = words.date(crate::text::iso_date("2026-11-22").unwrap());
        assert!(
            date.contains("22") && date.contains("2026"),
            "{code}: {date}"
        );
        let settings = Settings {
            lang: Some(code.to_owned()),
            ..own_glossary_only()
        };
        let html = page_with("# T\n\n> [!NOTE]\n> x\n", &settings);
        assert!(html.contains(&format!("<html lang=\"{code}\"")), "{code}");
        assert!(
            html.contains(words.callouts.note) && html.contains(words.search_page),
            "{code}"
        );
    }
    assert_eq!(["he", "yi"].map(direction), [Direction::RightToLeft; 2]);
    assert_eq!(direction("nds"), Direction::LeftToRight);
}

/// The text-size buttons are named in the page's language.
///
/// # Panics
///
/// When a German page's size group is not labelled `Textgröße`, or its
/// smaller-text button does not say `Kleinere Schrift` (without a
/// percentage).
#[test]
fn the_size_buttons_speak_the_pages_language() {
    let settings = Settings {
        lang: Some(String::from("de")),
        ..own_glossary_only()
    };
    let html = page_with("# T\n\nx\n", &settings);
    assert!(
        html.contains("<span class=\"size\" role=\"group\" aria-label=\"Textgröße\">"),
        "the group's name"
    );
    assert!(
        html.contains(
            "id=\"size-down\" class=\"btn icon\" type=\"button\" title=\"Kleinere Schrift\""
        ),
        "a tip without the percentage"
    );
}
