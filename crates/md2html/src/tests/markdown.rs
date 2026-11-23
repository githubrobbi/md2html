// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the Markdown becomes inside the article: brackets, lists,
//! emphasis, callouts, footnotes, tables, code, maths, diagrams and the
//! glossary.

use super::{article, assert_nested, index, own_glossary_only, page, page_with, warnings};
use crate::config::Settings;

#[test]
fn bare_brackets_are_text() {
    let html = page("# T\n\nMarks [V], [M] and [H] are evidence grades; [see](#t) is a link.\n");
    let art = article(&html);
    assert!(art.contains("[V], [M] and [H]"), "{art}");
    assert!(art.contains("<a href=\"#t\">see</a>"), "{art}");
}

#[test]
fn nested_lists_nest() {
    let html = page("# T\n\n- one\n  - one a\n  - one b\n- two\n");
    let art = article(&html);
    assert!(
        art.contains("<ul><li>one\n<ul><li>one a</li><li>one b</li></ul></li><li>two</li></ul>"),
        "{art}"
    );
}

#[test]
fn emphasis_nests_and_inline_images_render() {
    let html = page("# T\n\n***both***, and an icon ![i](i.png) inline.\n");
    let art = article(&html);
    assert!(art.contains("<em><strong>both</strong></em>"), "{art}");
    assert!(
        art.contains("<img src=\"i.png\" alt=\"i\" loading=\"lazy\">"),
        "{art}"
    );
}

#[test]
fn github_alerts_become_callouts_without_their_marker() {
    let html = page("# T\n\n> [!WARNING]\n> Hot surface.\n\n> [!TIP]\n> Use gloves.\n");
    let art = article(&html);
    assert!(art.contains("<div class=\"callout warning\" role=\"note\"><span class=\"mark\">Warning</span><div class=\"body\"><p>Hot surface.</p>"), "{art}");
    assert!(art.contains("callout tip"));
    assert!(!art.contains("[!"), "{art}");
}

#[test]
fn a_plain_quote_is_a_quote() {
    let html = page("# T\n\n> To be or not to be.\n");
    assert!(article(&html).contains("<blockquote><p>To be or not to be.</p></blockquote>"));
}

#[test]
fn footnotes_explain_themselves_and_lead_back() {
    let html = page("# T\n\nA claim.[^src]\n\n[^src]: The source, page 4.\n");
    let art = article(&html);
    assert!(art.contains("A claim.<sup class=\"fn\"><a class=\"fnref\" id=\"fnref-1-1\" href=\"#fn-1\" data-def=\"The source, page 4.\""), "{art}");
    assert!(
        art.contains(
            "<li id=\"fn-1\"><p>The source, page 4. <a class=\"back\" href=\"#fnref-1-1\""
        ),
        "{art}"
    );
}

#[test]
fn tasks_tables_and_strikethrough() {
    let html = page(
        "# T\n\n- [x] done\n- [ ] open\n\n| L | C | R |\n|:--|:-:|--:|\n| a | b | 1 |\n\n~~gone~~\n",
    );
    let art = article(&html);
    assert!(art.contains("<ul class=\"tasks\"><li class=\"task\"><input type=\"checkbox\" disabled checked aria-label=\"Done\"> done</li>"), "{art}");
    assert!(
        art.contains("<td class=\"c\">b</td><td class=\"r\">1</td>"),
        "{art}"
    );
    assert!(art.contains("<del>gone</del>"), "{art}");
}

#[test]
fn code_is_coloured_and_maths_is_mathml() {
    let html =
        page("# T\n\n```rust\nfn main() {}\n```\n\nEuler: $e^{i\\pi} + 1 = 0$, and $5 to $10.\n");
    let art = article(&html);
    assert!(
        art.contains("<code class=\"language-rust\"><span class=\"k\">fn</span>"),
        "{art}"
    );
    assert!(
        art.contains("<math>") && art.contains("<msup><mi>e</mi>"),
        "{art}"
    );
    assert!(art.contains("$5 to $10"), "{art}");
}

#[test]
fn mermaid_flowcharts_and_sequences_are_drawn_without_a_script() {
    let html = page(
        "# T\n\n```mermaid\nflowchart TD\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]\n```\n\n```mermaid\nsequenceDiagram\n  Alice->>Bob: Hello\n```\n",
    );
    let art = article(&html);
    assert_eq!(art.matches("class=\"diagram\"><svg").count(), 2, "{art}");
    assert!(art.contains("role=\"img\"") && art.contains("<desc id=\"dg1-d\">"));
    assert!(
        !html.contains("mermaid.initialize"),
        "no script for a native drawing"
    );
    assert!(
        index(&html).contains("Start"),
        "a diagram's words are searchable"
    );
    assert_nested(&art, "diagrams");
}

#[test]
fn other_mermaid_kinds_stay_code_with_a_note_and_code_mode_keeps_all_as_code() {
    let src = "# T\n\n```mermaid\npie title Pets\n  \"Dogs\" : 3\n```\n";
    assert!(article(&page(src)).contains("<code class=\"language-mermaid\">"));
    let said = warnings(src);
    assert!(
        said.iter()
            .any(|warning| warning.contains("`pie`") && warning.contains("--mermaid-js")),
        "{said:?}"
    );
    let settings = Settings {
        mermaid: Some(String::from("code")),
        ..own_glossary_only()
    };
    let html = page_with(
        "# T\n\n```mermaid\nflowchart TD\n  A --> B\n```\n",
        &settings,
    );
    assert!(!html.contains("figure class=\"diagram"));
}

#[test]
fn a_term_inside_a_file_name_is_not_linked() {
    let html = page(
        "# T\n\nLead.\n\n## One\n\nSee details/2026-01-05_LIMESTONE-SURVEY.json, then the limestone value.\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| limestone | A sedimentary rock. |\n",
    );
    let (before, at) = html
        .split_once("t-limestone-1")
        .expect("the term is linked once");
    assert!(before.contains("05_LIMESTONE"));
    assert!(at.starts_with("\" data-key"));
}

#[test]
fn a_chapter_about_the_glossary_is_not_the_glossary() {
    let html = page(
        "# T\n\n## 5. The shared glossary\n\nWords explained once.\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| once | One time. |\n",
    );
    assert!(html.contains("<h2 id=\"5-the-shared-glossary\">"));
    assert!(html.contains("<dt id=\"g-once\">"));
}

#[test]
fn glossary_terms_are_found_in_unspaced_scripts() {
    let md = "---\nlang: zh-Hans\n---\n# 标题\n\n本页的目录在左边。\n\n## 术语表\n\n| 术语 | 释义 |\n|:--|:--|\n| 目录 | 章节的列表。 |\n";
    let html = page(md);
    assert!(
        html.contains("class=\"term\""),
        "the term is linked inside a run of Han"
    );
    assert!(html.contains("<html lang=\"zh-Hans\""));
}

#[test]
fn a_chapter_that_mentions_the_glossary_is_still_a_chapter() {
    let html = page(
        "# T\n\n## 5. Das Glossar und die Fußnoten\n\nText.\n\n## Glossar\n\n| Begriff | Bedeutung |\n|:--|:--|\n| Text | Wörter. |\n",
    );
    assert!(html.contains("Das Glossar und die Fußnoten"));
}

#[test]
fn a_glossary_links_each_term_once_per_chapter_with_its_way_back() {
    let html = page(
        "# T\n\n## One\n\nA ledger, and the ledger again; LEDGERS too.\n\n## Two\n\nThe ledgers.\n\n## Glossary\n\n| Term | Meaning | Also |\n|---|---|---|\n| ledger | A book of accounts. | journal |\n\n- **Entity** - a thing.\n",
    );
    let art = article(&html);
    assert_eq!(art.matches("class=\"term\"").count(), 2, "{art}");
    assert!(
        art.contains("id=\"t-ledger-1\"") && art.contains("id=\"t-ledger-2\""),
        "{art}"
    );
    assert!(art.contains(">ledgers</a>"), "a plural is a mention: {art}");
    assert!(
        art.contains("<a class=\"back\" data-back=\"ledger\" href=\"#t-ledger-1\">"),
        "{art}"
    );
    assert!(
        art.contains("<dt id=\"g-entity\">Entity"),
        "a bold lead is an entry: {art}"
    );
}
