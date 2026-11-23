// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The page around the article: the hero, the outline, the search index,
//! the bar, the foot, the strings other tools find their place by.

use super::{article, assert_nested, index, page};

#[test]
fn a_callout_after_the_title_closes_the_hero_first() {
    let html = page("# Title\n\n> **Note.** Read this first.\n\nThe text.\n\n## One\n\nBody.\n");
    let art = article(&html);
    let hero_end = art.find("</header>").expect("hero closed");
    let aside = art
        .find("<div class=\"callout note\" role=\"note\">")
        .expect("callout");
    assert!(hero_end < aside);
    assert_eq!(art.matches("</header>").count(), 1);
    assert_nested(&art, "callout first");
}

#[test]
fn the_reading_column_is_centred_when_the_outline_is_hidden() {
    let html = page("# T\n\nLead.\n\n## One\n\nBody.\n");
    assert!(html.contains("html[data-toc=off] article>:not(.scroll),html[data-toc=off] article .scroll{margin-left:auto;margin-right:auto}"));
    assert!(html.contains("@media (max-width:980px){.wrap{grid-template-columns:minmax(0,1fr);gap:0}article>:not(.scroll),article .scroll{margin-left:auto;margin-right:auto}"));
}

#[test]
fn whatever_follows_the_title_the_page_structure_balances() {
    let openings = [
        "Plain lead paragraph.",
        "> **Note.** A callout first.",
        "- a list first\n- second item",
        "1. a numbered list first",
        "| a | b |\n|---|---|\n| 1 | 2 |",
        "```\ncode first\n```",
        "---",
        "## A chapter straight away",
        "- a\n  - nested\n    - deeper\n- b",
        "> a quote\n> > inside a quote",
        "![A figure](a.png)",
        "<details><summary>More</summary>\n\nHidden text.\n\n</details>",
    ];
    for opening in openings {
        let html = page(&format!(
            "# Title\n\n{opening}\n\nThe text.\n\n## One\n\nBody.\n"
        ));
        let art = article(&html);
        assert_eq!(
            art.matches("</header>").count(),
            1,
            "hero closed once after {opening:?}"
        );
        assert_nested(&art, opening);
    }
}

#[test]
fn the_lead_is_searchable() {
    let html = page(
        "# Title here\n\nOpened straight from a folder.\n\nMore opening words.\n\n## One\n\nBody.\n",
    );
    let index = index(&html);
    assert!(index.contains("folder"), "{index}");
    assert!(index.contains("More opening words"), "{index}");
    assert!(
        index.contains("{\"id\":\"opening\",\"t\":\"Title here\""),
        "{index}"
    );
    assert!(html.contains("<header class=\"hero\" id=\"opening\">"));
}

#[test]
fn a_page_without_chapters_or_title_still_has_both() {
    let html = page("Just a note about pelicans.\n\n- and a list\n");
    let index = index(&html);
    assert!(index.contains("pelicans"), "{index}");
    assert!(index.contains("and a list"), "{index}");
    let art = article(&html);
    assert!(
        art.contains("<h1>stdin</h1>"),
        "a title from the name: {art}"
    );
    assert_nested(&art, "untitled");
    assert!(
        html.contains("class=\"no-toc\""),
        "an empty outline is not shown"
    );
}

#[test]
fn table_cells_do_not_run_together() {
    let html =
        page("# T\n\nLead.\n\n## One\n\n| A | B |\n|---|---|\n| caption below | Note here |\n");
    assert!(index(&html).contains("caption below Note here"));
}

#[test]
fn a_reading_guide_card_is_searchable_and_points_at_its_chapter() {
    let html = page(
        "# T\n\nLead.\n\n## How to read this\n\n- **Chapter 1** takes two minutes.\n\n## 1. First\n\nBody.\n",
    );
    assert!(index(&html).contains("takes two minutes"));
    assert!(html.contains("class=\"card\" href=\"#1-first\""));
}

#[test]
fn only_a_page_with_a_figure_carries_the_enlargement() {
    let with = page("# T\n\n## One\n\n![A picture](docs/a.png)\n");
    assert!(with.contains("<figure class=\"fig\"><a href=\"docs/a.png\" target=\"_blank\""));
    assert!(with.contains("dialog.zoom"));
    let without = page("# T\n\n## One\n\nNo picture, only prose.\n");
    assert!(!without.contains("dialog.zoom"));
}

#[test]
fn headings_get_unique_anchors_and_shift_when_the_document_uses_several_h1() {
    let html = page("# Title\n\n# Part\n\n## Same\n\n## Same\n");
    let art = article(&html);
    assert!(art.contains("<h2 id=\"part\">"), "{art}");
    assert!(
        art.contains("<h3 id=\"same\">") && art.contains("<h3 id=\"same-2\">"),
        "{art}"
    );
}

#[test]
fn front_matter_sets_the_page() {
    let html = page(
        "---\ntitle: Die Pläne\nlang: de\nbrand: Beispielhaus\nkicker: Pläne\nfooter: Nur für die *Familie*.\ndescription: Die Zeichnungen.\n---\n\n# Überschrift\n\n> **Hinweis:** Schematisch.\n\n## Glossar\n\n| Begriff | Bedeutung |\n|---|---|\n| ADU | Eine Einliegerwohnung. |\n\n## Eins\n\nDie ADU.\n",
    );
    assert!(html.contains("<html lang=\"de\""));
    assert!(html.contains("<title>Die Pläne</title>"));
    assert!(html.contains("</svg>\nBeispielhaus</a><span class=\"kicker\">Pläne</span>"));
    assert!(html.contains("<footer class=\"fine\">Nur für die <em>Familie</em>.</footer>"));
    assert!(html.contains("<span class=\"mark\">Hinweis</span>"));
    assert!(html.contains(">Begriffe<a class=\"anchor\""));
    assert!(html.contains("content=\"Die Zeichnungen.\""));
    assert!(html.contains("Diese Seite durchsuchen"));
}

#[test]
fn the_strings_later_tools_rewrite_stay_as_they_are() {
    // Tools that change a page after it is rendered find their place by
    // these exact strings; a page that changes one of them breaks them.
    let html = page(
        "<!-- c -->\n# T\n\nLead.\n\n## 1. One\n\n![A drawing](plans/a.svg)\n\n⟦cards⟧\n\n> **Note:** schematic.\n\n> **Warning:** careful.\n",
    );
    for mark in [
        "<html lang=\"en\"",
        "<head>",
        "</head>",
        "</style>",
        "<!-- Generated from stdin by md2html; edit the Markdown, not this page. -->\n",
        "<a class=\"brand\" href=\"#top\">",
        "\nmd2html</a><span class=\"kicker\">stdin</span>",
        "<div class=\"eyebrow\">stdin &middot; ",
        "<header class=\"hero\"",
        "<footer class=\"fine\">",
        "<figure class=\"fig\">",
        "<p>⟦cards⟧</p>",
        "<span class=\"mark\">Note</span>",
        "<span class=\"mark\">Warning</span>",
        "</body>",
    ] {
        assert!(html.contains(mark), "missing {mark:?}");
    }
}

#[test]
fn every_page_credits_md2html_and_offers_the_reading_panel() {
    let html = page("# T\n\nx\n");
    assert!(html.contains(
        "<p class=\"credit\">Made with <a href=\"https://github.com/githubrobbi/md2html\""
    ));
    assert!(html.contains(env!("CARGO_PKG_VERSION")));
    assert!(html.contains("popovertarget=\"prefs\"") && html.contains("id=\"print\""));
    assert!(
        html.contains("@page"),
        "printed pages carry title and number"
    );
}

#[test]
fn the_fingerprint_opens_the_head_and_names_the_rest() {
    let html = page("# T\n\nx\n");
    let (opening, after) = html
        .split_once("<meta name=\"md2html-page\" content=\"")
        .expect("a fingerprint");
    assert!(
        opening.ends_with("<head><meta charset=\"utf-8\">"),
        "{opening}"
    );
    let (print, _) = after.split_once('"').expect("the fingerprint ends");
    assert_eq!(print.len(), 24, "twelve bytes, in hex: {print}");
    assert!(print.chars().all(|ch| ch.is_ascii_hexdigit()), "{print}");
    let unprinted = html.replacen(
        &format!("<meta name=\"md2html-page\" content=\"{print}\">"),
        "",
        1,
    );
    let digest = crate::text::hex(&crate::text::sha256(unprinted.as_bytes()));
    assert!(
        digest.starts_with(print),
        "the page without it hashes to it"
    );
}
