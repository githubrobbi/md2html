// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the Markdown becomes inside the article: brackets, lists,
//! emphasis, callouts, footnotes, tables, code, maths, diagrams and the
//! glossary.

use super::{article, assert_nested, index, own_glossary_only, page, page_with, warnings};
use crate::config::Settings;

/// Brackets that make no link stay text.
///
/// # Panics
///
/// When `[V]`, `[M]` and `[H]` with no reference behind them are not kept
/// as written, or when the link `[see](#t)` beside them is not a link.
#[test]
fn bare_brackets_are_text() {
    let html = page("# T\n\nMarks [V], [M] and [H] are evidence grades; [see](#t) is a link.\n");
    let art = article(&html);
    assert!(art.contains("[V], [M] and [H]"), "{art}");
    assert!(art.contains("<a href=\"#t\">see</a>"), "{art}");
}

/// An indented list item opens a list inside the item above it.
///
/// # Panics
///
/// When the two indented items are not a `<ul>` inside the first item, or
/// the item after them is not back in the outer list.
#[test]
fn nested_lists_nest() {
    let html = page("# T\n\n- one\n  - one a\n  - one b\n- two\n");
    let art = article(&html);
    assert!(
        art.contains("<ul><li>one\n<ul><li>one a</li><li>one b</li></ul></li><li>two</li></ul>"),
        "{art}"
    );
}

/// Triple emphasis nests, and an image inside a line stays inline.
///
/// # Panics
///
/// When `***both***` is not `<em><strong>`, or the image in the line is not
/// an `<img>` with its alt text, loaded lazily.
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

/// GitHub alerts become callouts, and their `[!KIND]` marker is not shown.
///
/// # Panics
///
/// When `> [!WARNING]` is not a warning callout with its mark and body, when
/// `> [!TIP]` is not a tip callout, or when a `[!` marker is left in the
/// article.
#[test]
fn github_alerts_become_callouts_without_their_marker() {
    let html = page("# T\n\n> [!WARNING]\n> Hot surface.\n\n> [!TIP]\n> Use gloves.\n");
    let art = article(&html);
    assert!(art.contains("<div class=\"callout warning\" role=\"note\"><span class=\"mark\">Warning</span><div class=\"body\"><p>Hot surface.</p>"), "{art}");
    assert!(art.contains("callout tip"));
    assert!(!art.contains("[!"), "{art}");
}

/// A quote with no alert marker is a block quote.
///
/// # Panics
///
/// When the quote is not a `<blockquote>` holding its paragraph.
#[test]
fn a_plain_quote_is_a_quote() {
    let html = page("# T\n\n> To be or not to be.\n");
    assert!(article(&html).contains("<blockquote><p>To be or not to be.</p></blockquote>"));
}

/// A footnote reference carries its note, and the note links back to it.
///
/// # Panics
///
/// When the reference is not a superscript link to the note carrying the
/// note's text in `data-def`, or when the note is not a list item with a
/// link back to the reference.
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

/// Task lists, aligned tables and strikethrough render as GitHub's do.
///
/// # Panics
///
/// When a done task is not a disabled, checked, labelled checkbox in a
/// task list; when the centred and right-aligned table cells do not carry
/// their alignment; or when `~~gone~~` is not `<del>`.
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

/// Code is coloured, maths becomes `MathML`, and prices stay prices.
///
/// # Panics
///
/// When the Rust keyword `fn` is not marked in its code block, when
/// `$e^{i\pi} + 1 = 0$` is not `MathML` with its superscript, or when the
/// prices `$5 to $10` are read as maths.
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

/// Mermaid flowcharts and sequence diagrams are drawn as SVG, no script.
///
/// # Panics
///
/// When the two diagrams are not two SVG drawings with a role and a
/// description, when the page loads mermaid.js for them, when their words
/// are not in the search index, or when their elements do not nest.
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

/// A mermaid kind md2html cannot draw stays code, with a note to the author.
///
/// # Panics
///
/// When a `pie` chart is not left as a mermaid code block, when the author
/// is not told that `pie` needs `--mermaid-js`, or when `mermaid: code`
/// still draws a flowchart.
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

/// A glossary term inside a file name is not linked.
///
/// # Panics
///
/// When the term is not linked exactly once, or when its first link is in
/// the file name `2026-01-05_LIMESTONE-SURVEY.json` instead of on the word
/// `limestone` after it.
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

/// A chapter whose heading names the glossary is not taken for it.
///
/// # Panics
///
/// When the chapter `5. The shared glossary` is not kept as a chapter, or
/// the real glossary after it does not list its term.
#[test]
fn a_chapter_about_the_glossary_is_not_the_glossary() {
    let html = page(
        "# T\n\n## 5. The shared glossary\n\nWords explained once.\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| once | One time. |\n",
    );
    assert!(html.contains("<h2 id=\"5-the-shared-glossary\">"));
    assert!(html.contains("<dt id=\"g-once\">"));
}

/// Glossary terms are found in a script written without spaces.
///
/// # Panics
///
/// When the term `目录` is not linked inside the run of Han characters
/// around it, or the page does not say its language is `zh-Hans`.
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

/// A chapter whose heading mentions the glossary is still a chapter.
///
/// # Panics
///
/// When the chapter `5. Das Glossar und die Fußnoten` is dropped from the
/// page, taken for the glossary that follows it.
#[test]
fn a_chapter_that_mentions_the_glossary_is_still_a_chapter() {
    let html = page(
        "# T\n\n## 5. Das Glossar und die Fußnoten\n\nText.\n\n## Glossar\n\n| Begriff | Bedeutung |\n|:--|:--|\n| Text | Wörter. |\n",
    );
    assert!(html.contains("Das Glossar und die Fußnoten"));
}

/// A glossary term is linked once per chapter, with a link back to it.
///
/// # Panics
///
/// When `ledger` is not linked once in each of the two chapters (a plural
/// or another case counting as a mention), when its entry has no link back
/// to the first mention, or when a bold lead after the table is not an
/// entry of its own.
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
