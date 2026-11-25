// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The heading anchors' contract.
//!
//! Written in `docs/policies/release_policy.md` ("What a version number
//! promises") and `docs/command-line.md` ("What is stable").
//!
//! A heading's anchor is the one GitHub gives it, so a link written for a
//! document on GitHub (`concepts.md#1--size-vs-size-on-disk`) lands on
//! the same heading in md2html's page, and the same heading text gives the
//! same id in every version. The rule (`crate::text::heading_slug`): the
//! heading's words lowercased (an image's description is none of them;
//! GitHub counts a footnote's number, md2html does not), every
//! character removed that is not a letter, a mark, a decimal digit, a connector
//! punctuation, a joiner, a space or a hyphen, each space a hyphen, nothing
//! collapsed or trimmed; a repeated anchor numbered `-1`, `-2` as GitHub
//! numbers it. Every row of [`AS_GITHUB_WRITES`] was checked against GitHub's
//! own renderer (`gh api markdown -f mode=markdown`, 2026-11-25). md2html
//! departs from it three ways, in [`MD2HTML_OWN`]: an explicit `{#id}` is taken
//! as written, a heading that leaves no character is named `section` (GitHub
//! gives it an empty id, which HTML does not allow), and an id the page
//! uses itself is numbered as a repeat is. A change to any row is a change
//! to the rule: a breaking change, which raises the minor number before
//! 1.0 and the major from 1.0. md2html's own ids (the bar, the outline, a
//! code block's or a diagram's address) are not part of it, but a heading
//! never takes one.

use alloc::collections::BTreeMap;
use std::path::Path;

use super::{article, own_glossary_only, page, page_with};
use crate::text::{heading_slug, put};

/// The ids of the article's headings below the title, in order.
fn heading_ids(markdown: &str) -> Vec<String> {
    article(&page(markdown))
        .split("<h")
        .filter_map(|tag| {
            let after = ["2", "3", "4", "5", "6"]
                .into_iter()
                .find_map(|level| tag.strip_prefix(level))?;
            let (id, _) = after.strip_prefix(" id=\"")?.split_once('"')?;
            Some(id.to_owned())
        })
        .collect()
}

/// Heading texts and the anchor GitHub gives each, as its API returned
/// them on 2026-11-25.
const AS_GITHUB_WRITES: [(&str, &str); 48] = [
    ("Installation", "installation"),
    ("Getting Started", "getting-started"),
    ("What's new?", "whats-new"),
    ("C++ & Rust: a guide", "c--rust-a-guide"),
    ("--Dashes  and   spaces--", "--dashes--and---spaces--"),
    ("Version 1.2.3", "version-123"),
    ("snake_case_name", "snake_case_name"),
    ("3. Über die Lage", "3-über-die-lage"),
    ("ÉCOLE Straße", "école-straße"),
    ("日本語の見出し", "日本語の見出し"),
    ("中文标题：测试", "中文标题测试"),
    ("한국어 제목", "한국어-제목"),
    ("שלום עולם", "שלום-עולם"),
    ("Привет, мир!", "привет-мир"),
    ("Ελληνικά ΟΔΟΣ", "ελληνικά-οδοσ"),
    ("हिन्दी भाषा", "हिन्दी-भाषा"),
    ("ภาษาไทย ก่อน", "ภาษาไทย-ก่อน"),
    ("Cafe\u{301} NFD", "cafe\u{301}-nfd"),
    ("İstanbul", "i\u{307}stanbul"),
    ("Rocket 🚀 launch", "rocket--launch"),
    ("I ❤\u{fe0f} Rust", "i-\u{fe0f}-rust"),
    (
        "Family 👨\u{200d}👩\u{200d}👧 here",
        "family-\u{200d}\u{200d}-here",
    ),
    (
        "Keycap 1\u{fe0f}\u{20e3} one",
        "keycap-1\u{fe0f}\u{20e3}-one",
    ),
    ("x² and ½ and Ⅻ", "x-and--and-ⅻ"),
    ("Ⓐ circled", "ⓐ-circled"),
    ("1 · Size vs Size on Disk", "1--size-vs-size-on-disk"),
    (
        "The Watchdog — surviving crashes and installers",
        "the-watchdog--surviving-crashes-and-installers",
    ),
    ("Tom &amp; Jerry", "tom--jerry"),
    ("A&nbsp;B", "ab"),
    (
        "\"Quoted\" and ‘single’ «guillemets»",
        "quoted-and-single-guillemets",
    ),
    ("100% (done) [ok] {x}", "100-done-ok-x"),
    ("$price #tag @user ~tilde", "price-tag-user-tilde"),
    ("Hello__World", "hello__world"),
    ("dash-at-end-", "dash-at-end-"),
    ("→ arrow", "-arrow"),
    ("*Emphasis* and **strong**", "emphasis-and-strong"),
    ("The `slug()` function", "the-slug-function"),
    ("A [link](https://example.com) here", "a-link-here"),
    (
        "Link <https://example.com/a> end",
        "link-httpsexamplecoma-end",
    ),
    ("Logo ![alt text](x.png) here", "logo--here"),
    ("Press <kbd>Ctrl</kbd> now", "press-ctrl-now"),
    ("Note <!-- hidden --> here", "note--here"),
    ("~~old~~ new", "old-new"),
    ("Escaped \\*star\\* and \\_u\\_", "escaped-star-and-_u_"),
    ("Launch :rocket:", "launch-rocket"),
    ("Title{#kept}", "titlekept"),
    ("Top-level", "top-level"),
    ("FAQ: Why?", "faq-why"),
];

/// Where md2html departs from GitHub: an explicit `{#id}`, a heading that
/// leaves no character, and the ids the page uses itself.
const MD2HTML_OWN: [(&str, &str); 6] = [
    ("Title {#custom-id}", "custom-id"),
    ("Title {#Mixed.Case:id}", "Mixed.Case:id"),
    ("!!!", "section"),
    ("Top", "top-1"),
    ("Find", "find-1"),
    ("Print", "print-1"),
];

/// Each heading text gives the anchor GitHub gives it, or md2html's own
/// where the rule departs from GitHub's.
///
/// # Panics
///
/// When a heading of [`AS_GITHUB_WRITES`] or [`MD2HTML_OWN`] gets another
/// id, or none.
#[test]
fn a_heading_text_gives_the_same_id_in_every_version() {
    for (text, id) in AS_GITHUB_WRITES.into_iter().chain(MD2HTML_OWN) {
        let ids = heading_ids(&format!("# T\n\n## {text}\n"));
        assert_eq!(ids, [id], "the heading `{text}`");
    }
}

/// Headings that would share an id are numbered as GitHub numbers them:
/// in the order they come, each base counting its own repeats, whatever
/// made the clash; one at another level counts too.
///
/// # Panics
///
/// When one of the documents' headings gets other ids than its row names.
#[test]
fn a_repeated_id_is_numbered_in_order() {
    let documents: [(&str, &[&str]); 4] = [
        ("## Same\n\n## Same\n\n### Same\n", &[
            "same", "same-1", "same-2",
        ]),
        (
            "## Same\n\n## Same\n\n## Same 1\n\n## Same\n\n## Same-1\n\n### Same\n\n## Same 2\n",
            &[
                "same", "same-1", "same-1-1", "same-2", "same-1-2", "same-3", "same-2-1",
            ],
        ),
        ("## !!!\n\n## ???\n", &["section", "section-1"]),
        ("## Notes {#x}\n\n## X\n", &["x", "x-1"]),
    ];
    for (body, expected) in documents {
        let ids = heading_ids(&format!("# T\n\n{body}"));
        assert_eq!(ids, expected, "the document {body:?}");
    }
}

/// No heading takes an id the page gives an element of its own, by name
/// or by pattern: every id on the page stands once, whatever the headings
/// are called.
///
/// # Panics
///
/// When an id stands twice on the page, or a heading named after one of
/// the page's own ids keeps it.
#[test]
fn a_heading_never_takes_an_id_of_the_page() {
    let headings = [
        "Print",
        "Save",
        "Keep",
        "Lang",
        "Toast",
        "Notes",
        "Fn 1",
        "Fnref 1 1",
        "Fnref 1 2",
        "G Ledger",
        "T Ledger 1",
        "T Ledger 2",
        "DG1 T",
        "Hit 0",
        "Fig 1",
    ];
    let mut markdown = String::from(
        "# T\n\nA ledger, the ledger again.[^1] Once more.[^1]\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n",
    );
    for heading in headings {
        put!(markdown, "## {heading}\n\nText.\n\n");
    }
    markdown.push_str("## Glossary\n\n**ledger**: a book of accounts.\n\n[^1]: A note.\n");
    let html = page_with(&markdown, &own_glossary_only());
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for (at, marker) in html.match_indices(" id=\"") {
        let after = html.get(at + marker.len()..).unwrap();
        let id = after.split_once('"').map_or(after, |(id, _)| id);
        *count.entry(id).or_insert(0) += 1;
    }
    let twice: Vec<_> = count.iter().filter(|(_, times)| **times > 1).collect();
    assert!(twice.is_empty(), "ids that stand twice: {twice:?}");
    for own in [
        "print",
        "fn-1",
        "fnref-1-1",
        "fnref-1-2",
        "g-ledger",
        "t-ledger-1",
        "dg1-t",
    ] {
        assert!(count.contains_key(own), "the page's own `{own}` is there");
    }
    let ids = heading_ids(&markdown);
    assert!(
        ids.contains(&String::from("print-1")) && ids.contains(&String::from("g-ledger-1")),
        "{ids:?}"
    );
}

/// The characters an anchor keeps are Unicode's word characters, as the
/// vendored Unicode Character Database says, code point by code point.
///
/// The letters, and the marks, decimal digits and connector punctuation
/// the file's general categories name, with the two joiners; the hyphen
/// too, and a space as a hyphen. Every Unicode scalar value is checked,
/// so the generated table (`crate::text`'s `word.rs`) cannot keep a
/// character the file does not name, or drop one it does.
///
/// # Panics
///
/// When the vendored file is not Unicode 16.0.0's, or a character is kept
/// that is none of these, or one of these is dropped.
#[test]
fn an_anchor_keeps_the_word_characters_of_unicode() {
    const KEPT: [&str; 5] = [
        "Nonspacing_Mark",
        "Spacing_Mark",
        "Enclosing_Mark",
        "Decimal_Number",
        "Connector_Punctuation",
    ];
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/unicode/DerivedGeneralCategory.txt");
    let data = std::fs::read_to_string(&file).unwrap();
    assert!(data.starts_with("# DerivedGeneralCategory-16.0.0.txt\n"));
    let mut word = vec![false; 0x11_0000];
    let mut category = "";
    for line in data.lines() {
        if let Some(name) = line.strip_prefix("# General_Category=") {
            category = name.trim();
            continue;
        }
        let fields = line.split_once('#').map_or(line, |(fields, _)| fields);
        let Some((written, _)) = fields.split_once(';') else {
            continue;
        };
        if !KEPT.contains(&category) {
            continue;
        }
        let span = written.trim();
        let (first_hex, last_hex) = span.split_once("..").unwrap_or((span, span));
        let first = usize::from_str_radix(first_hex, 16).unwrap();
        let last = usize::from_str_radix(last_hex, 16).unwrap();
        let Some(points) = word.get_mut(first..=last) else {
            panic!("{span} lies beyond Unicode");
        };
        points.fill(true);
    }
    for (point, named) in word.iter().enumerate() {
        let Some(ch) = char::from_u32(u32::try_from(point).unwrap()) else {
            continue;
        };
        let expected =
            *named || ch.is_alphabetic() || matches!(ch, '\u{200c}' | '\u{200d}' | '-' | ' ');
        let slug = heading_slug(ch.encode_utf8(&mut [0; 4]));
        assert_eq!(!slug.is_empty(), expected, "U+{point:04X}");
    }
}
