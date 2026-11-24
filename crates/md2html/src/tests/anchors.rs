// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The heading anchors' contract.
//!
//! Written in `docs/policies/release_policy.md` ("What a version number
//! promises") and `docs/command-line.md` ("What is stable").
//!
//! The same heading text gives the same id in every version, so a link to
//! `page.html#installation` keeps working. These tests pin the rule with a
//! table of heading texts and the ids md2html gives them: letters and
//! digits lowercased, letters beyond ASCII kept, any other run one dash,
//! the words of emphasis, code and links counted as text, an explicit
//! `{#id}` taken as written, a repeated id numbered `-2`, `-3`, an id the
//! page itself uses numbered the same way, and a heading with no letter or
//! digit named `section`. A change to any row is a change to the rule: a
//! breaking change, which raises the minor number before 1.0 and the major
//! from 1.0. md2html's own ids (the bar, the outline, a code block's or a
//! diagram's address) are not part of it.

use super::{article, page};

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

/// Each heading text, alone under a title, and the id it gets.
const ONE_HEADING: [(&str, &str); 22] = [
    ("Installation", "installation"),
    ("Getting Started", "getting-started"),
    ("What's new?", "what-s-new"),
    ("C++ & Rust: a guide", "c-rust-a-guide"),
    ("--Dashes  and   spaces--", "dashes-and-spaces"),
    ("Version 1.2.3", "version-1-2-3"),
    ("snake_case_name", "snake-case-name"),
    ("3. Über die Lage", "3-über-die-lage"),
    ("ÉCOLE Straße", "école-straße"),
    ("日本語の見出し", "日本語の見出し"),
    ("Rocket 🚀 launch", "rocket-launch"),
    ("*Emphasis* and **strong**", "emphasis-and-strong"),
    ("The `slug()` function", "the-slug-function"),
    ("A [link](https://example.com) here", "a-link-here"),
    ("Press <kbd>Ctrl</kbd> now", "press-ctrl-now"),
    ("Title {#custom-id}", "custom-id"),
    ("Title {#Mixed.Case:id}", "Mixed.Case:id"),
    ("Title{#kept}", "title-kept"),
    ("!!!", "section"),
    ("Top", "top-2"),
    ("Find", "find-2"),
    ("Main", "main-2"),
];

/// Each heading text gives the id its row names.
///
/// # Panics
///
/// When a heading of [`ONE_HEADING`] gets another id, or none.
#[test]
fn a_heading_text_gives_the_same_id_in_every_version() {
    for (text, id) in ONE_HEADING {
        let ids = heading_ids(&format!("# T\n\n## {text}\n"));
        assert_eq!(ids, [id], "the heading `{text}`");
    }
}

/// Headings that would share an id are numbered in the order they come,
/// whatever made the clash; one at another level counts too.
///
/// # Panics
///
/// When one of the documents' headings gets other ids than its row names.
#[test]
fn a_repeated_id_is_numbered_in_order() {
    let documents: [(&str, &[&str]); 4] = [
        ("## Same\n\n## Same\n\n### Same\n", &[
            "same", "same-2", "same-3",
        ]),
        ("## Same\n\n## Same\n\n## Same 2\n", &[
            "same", "same-2", "same-2-2",
        ]),
        ("## !!!\n\n## ???\n", &["section", "section-2"]),
        ("## Notes {#x}\n\n## X\n", &["x", "x-2"]),
    ];
    for (body, expected) in documents {
        let ids = heading_ids(&format!("# T\n\n{body}"));
        assert_eq!(ids, expected, "the document {body:?}");
    }
}
