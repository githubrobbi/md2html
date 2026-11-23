// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Text that enters the page around the article is escaped exactly once.
//!
//! The settings, the file's name, the date and version, the links to the
//! run's other pages, the notice of a translation and the names of the
//! page's languages: never twice (`&amp;lt;`), never not at all (`<b>`).

use std::path::Path;

use super::{Scratch, own_glossary_only};
use crate::document::{Link, Neighbours, render_doc};
use crate::text::put;

/// Text with every character HTML escapes, after a word that names it.
fn hostile(name: &str) -> String {
    format!("{name} <b>\"&")
}

/// [`hostile`] text escaped once.
fn once(name: &str) -> String {
    format!("{name} &lt;b&gt;&quot;&amp;")
}

/// Each of `names`, written as [`hostile`] text, is in `html` escaped once
/// and never as it is or escaped twice.
///
/// # Panics
///
/// When a name's text is missing escaped once, or is found unescaped or
/// escaped twice; the message names `case` and the name.
fn assert_escaped_once(html: &str, names: &[&str], case: &str) {
    for name in names {
        assert!(
            html.contains(&once(name)),
            "{case}: {name} is not escaped once"
        );
        assert!(
            !html.contains(&format!("{name} <b>")),
            "{case}: {name} is not escaped"
        );
        assert!(
            !html.contains(&format!("{name} &amp;lt;")),
            "{case}: {name} is escaped twice"
        );
    }
}

/// A page of `text` read from `source`, written beside it.
fn page_of(text: &str, source: &Path) -> String {
    let out = source.with_extension("html");
    render_doc(
        text,
        Some(source),
        Some(&out),
        &own_glossary_only(),
        false,
        None,
    )
    .html
}

/// The settings that are words are escaped once wherever the page shows them.
///
/// The title in the head, the hero and the meta tags; the brand and the
/// kicker in the bar; the line above the title; the description; the
/// author; the language.
///
/// # Panics
///
/// When one of them is missing escaped once, or is found unescaped or
/// escaped twice.
#[test]
fn settings_are_escaped_once() {
    let names = [
        "title",
        "brand",
        "kicker",
        "eyebrow",
        "description",
        "author",
        "lang",
    ];
    let front = names.iter().fold(String::new(), |mut front, name| {
        put!(front, "{name}: {}\n", hostile(name));
        front
    });
    let html = page_of(
        &format!("---\n{front}---\nSome text.\n"),
        Path::new("notes.md"),
    );
    assert_escaped_once(&html, &names, "settings");
}

/// The line above the title made of the file's name, the date as written
/// and the version: each part escaped once.
///
/// # Panics
///
/// When the date or the version is missing escaped once, or is found
/// unescaped or escaped twice.
#[test]
fn the_date_and_version_are_escaped_once() {
    let html = page_of(
        &format!(
            "---\ndate: {}\nversion: {}\n---\n# Title\n\nText.\n",
            hostile("date"),
            hostile("version")
        ),
        Path::new("notes.md"),
    );
    assert_escaped_once(&html, &["date", "version"], "eyebrow");
}

/// The Markdown file's name: in the kicker, the line above the title, the
/// fine print, the comment at the top and, with no heading, the title.
///
/// # Panics
///
/// When the name is missing escaped once, or is found unescaped or escaped
/// twice.
#[test]
fn the_file_name_is_escaped_once() {
    let source = format!("/nowhere/{}.md", hostile("file"));
    let html = page_of("Text without a heading.\n", Path::new(&source));
    assert_escaped_once(&html, &["file"], "file name");
}

/// The fine print is Markdown: its text escaped once, its emphasis markup.
///
/// # Panics
///
/// When the footer's `&` and quotes are not escaped once, or its emphasis
/// is not an `em`.
#[test]
fn the_footer_is_markdown_escaped_once() {
    let html = page_of(
        "---\nfooter: Fine & \"print\" *here*\n---\nText.\n",
        Path::new("notes.md"),
    );
    assert!(
        html.contains("<footer class=\"fine\">Fine &amp; &quot;print&quot; <em>here</em></footer>"),
        "{html}"
    );
}

/// The links to the run's other pages: their titles escaped once, their
/// addresses written once as attributes.
///
/// # Panics
///
/// When a title is missing escaped once, or is found unescaped or escaped
/// twice; or when an address's `&` is not escaped exactly once.
#[test]
fn the_links_to_other_pages_are_escaped_once() {
    let nav = Neighbours {
        prev: Some(Link {
            href: String::from("a.html?x=1&y=2"),
            title: hostile("previous"),
        }),
        next: Some(Link {
            href: String::from("b.html"),
            title: hostile("next"),
        }),
    };
    let html = render_doc(
        "# Here\n\nText.\n",
        None,
        None,
        &own_glossary_only(),
        false,
        Some(nav),
    )
    .html;
    assert_escaped_once(&html, &["previous", "next"], "nav");
    assert!(html.contains("href=\"a.html?x=1&amp;y=2\""), "{html}");
    assert!(!html.contains("&amp;amp;"), "{html}");
}

/// Where the name in the bar leads: its `&` escaped once.
///
/// # Panics
///
/// When the link's `&` is not escaped exactly once.
#[test]
fn the_brand_link_is_escaped_once() {
    let html = page_of(
        "---\nbrand_link: /home?a=1&b=2\n---\nText.\n",
        Path::new("notes.md"),
    );
    assert!(html.contains("href=\"/home?a=1&amp;b=2\""), "{html}");
    assert!(!html.contains("&amp;amp;"), "{html}");
}

/// A translation's notice and the language switch: the original's
/// language, as its front matter names it, escaped once in the notice,
/// the alternates and the switch.
///
/// # Panics
///
/// When the original's language is missing escaped once, or is found
/// unescaped or escaped twice; or when the scratch files cannot be written.
#[test]
fn a_translation_notice_is_escaped_once() {
    let scratch = Scratch::new("escaping");
    let original = format!("---\nlang: {}\n---\n# Original\n\nText.\n", hostile("code"));
    scratch.write("page.md", &original);
    let digest = crate::text::hex(&crate::text::sha256(original.as_bytes()));
    let translation =
        format!("---\ntranslated_from: page.md\nsource_sha256: {digest}\n---\n# Seite\n\nText.\n");
    let path = scratch.write("page.de.md", &translation);
    let html = page_of(&translation, &path);
    assert!(html.contains("<p class=\"translated\">"), "{html}");
    assert_escaped_once(&html, &["code"], "translation");
}
