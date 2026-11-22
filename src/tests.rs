// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Whole pages, rendered from Markdown and read back.

use crate::config::Settings;
use crate::render_doc;

/// A page as the tool writes it, with no shared glossary.
fn page(text: &str) -> String {
    page_with(
        text,
        &Settings {
            glossary: Some(false),
            ..Settings::default()
        },
    )
}

fn page_with(text: &str, settings: &Settings) -> String {
    render_doc(text, None, None, settings, false, None).html
}

fn warnings(text: &str) -> Vec<String> {
    let settings = Settings {
        glossary: Some(false),
        ..Settings::default()
    };
    render_doc(text, None, None, &settings, false, None)
        .warnings
        .into_iter()
        .map(|w| w.message)
        .collect()
}

/// The search index of a page.
fn index(html: &str) -> &str {
    let open = "<script id=\"idx\" type=\"application/json\">";
    let start = html.find(open).map_or(0, |at| at + open.len());
    html[start..].split("</script>").next().unwrap_or_default()
}

/// The article of a page.
fn article(html: &str) -> &str {
    let start = html.find("<article").expect("article");
    let end = html.find("</article>").expect("article closed");
    &html[start..end + "</article>".len()]
}

/// Every block element closes in the order it opened.
fn assert_nested(html: &str, case: &str) {
    const BLOCKS: [&str; 16] = [
        "article",
        "header",
        "aside",
        "div",
        "ul",
        "ol",
        "li",
        "table",
        "pre",
        "nav",
        "section",
        "blockquote",
        "figure",
        "p",
        "dl",
        "details",
    ];
    let mut open: Vec<&str> = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        let closing = rest.starts_with('/');
        let name_start = usize::from(closing);
        let end = rest[name_start..]
            .find(|c: char| !c.is_ascii_alphanumeric())
            .map_or(rest.len(), |e| e + name_start);
        let name = &rest[name_start..end];
        if !BLOCKS.contains(&name) {
            continue;
        }
        if closing {
            assert_eq!(
                open.pop(),
                Some(name),
                "</{name}> closes out of order in {case:?}"
            );
        } else {
            open.push(name);
        }
    }
    assert!(open.is_empty(), "left open {open:?} in {case:?}");
}

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
    assert_nested(art, "callout first");
}

#[test]
fn the_reading_column_is_centred_when_the_outline_is_hidden() {
    let html = page("# T\n\nLead.\n\n## One\n\nBody.\n");
    assert!(html.contains("html[data-toc=off] article>:not(.scroll),html[data-toc=off] article .scroll{margin-left:auto;margin-right:auto}"));
    assert!(html.contains("@media (max-width:980px){.wrap{grid-template-columns:minmax(0,1fr);gap:0}article>:not(.scroll),article .scroll{margin-left:auto;margin-right:auto}"));
}

#[test]
fn a_term_inside_a_file_name_is_not_linked() {
    let html = page(
        "# T\n\nLead.\n\n## One\n\nSee details/2026-11-17_METANEPHRINE-FRACTIONATED.json, then the metanephrine value.\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| metanephrine | A breakdown product. |\n",
    );
    let at = html
        .find("t-metanephrine-1")
        .expect("the term is linked once");
    assert!(html[..at].contains("17_METANEPHRINE"));
    assert!(html[at..].starts_with("t-metanephrine-1\" data-key"));
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
        assert_nested(art, opening);
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
    assert_nested(art, "untitled");
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

// Security.

#[test]
fn script_links_are_never_written() {
    for evil in [
        "[x](javascript:alert(1))",
        "[x](JAVASCRIPT:alert(1))",
        "[x](  javascript:alert(1))",
        "[x](java&#x73;cript:alert(1))",
        "[x](vbscript:msgbox)",
        "[x](data:text/html;base64,PHNjcmlwdD4=)",
        "<javascript:alert(1)>",
        "![x](javascript:alert(1))",
        "[x][r]\n\n[r]: javascript:alert(1)",
    ] {
        let html = page(&format!("# T\n\n{evil}\n"));
        let art = article(&html).to_lowercase();
        assert!(
            !art.contains("javascript:") || !art.contains("href=\"javascript"),
            "{evil}: {art}"
        );
        assert!(
            !art.contains("href=\"javascript") && !art.contains("src=\"javascript"),
            "{evil}"
        );
        assert!(
            !art.contains("href=\"vbscript") && !art.contains("href=\"data:"),
            "{evil}"
        );
    }
}

#[test]
fn raw_html_cannot_run() {
    for evil in [
        "<script>alert(1)</script>",
        "<img src=x onerror=alert(1)>",
        "<div onclick=\"alert(1)\">x</div>",
        "<iframe src=\"https://example.com\"></iframe>",
        "Text <b onmouseover=alert(1)>bold</b> text",
        "<a href=\"javascript:alert(1)\">x</a>",
        "<svg onload=alert(1)>",
        "<style>body{display:none}</style>",
    ] {
        let html = page(&format!("# T\n\n{evil}\n"));
        let art = article(&html).to_lowercase();
        // The tags the browser will build: no handler, no script, no frame.
        for tag in art.split('<').skip(1).filter_map(|t| t.split('>').next()) {
            let name = tag.split_whitespace().next().unwrap_or_default();
            assert!(
                !["script", "iframe", "style", "svg", "img"].contains(&name)
                    || (name == "img" && !tag.contains(" on")),
                "{evil}: <{tag}>"
            );
            assert!(
                !tag.split_whitespace()
                    .skip(1)
                    .any(|attr| attr.starts_with("on")),
                "{evil}: <{tag}>"
            );
            assert!(!tag.contains("javascript:"), "{evil}: <{tag}>");
        }
    }
}

#[test]
fn harmless_html_passes() {
    let html = page(
        "# T\n\nPress <kbd>Ctrl</kbd>+<kbd>K</kbd>, H<sub>2</sub>O.<br>Next line.\n\n<details open><summary>More</summary>\n\nInside.\n\n</details>\n",
    );
    let art = article(&html);
    assert!(
        art.contains("<kbd>Ctrl</kbd>") && art.contains("<sub>2</sub>") && art.contains("<br>"),
        "{art}"
    );
    assert!(
        art.contains("<details open><summary>More</summary>"),
        "{art}"
    );
}

#[test]
fn the_page_cannot_be_broken_out_of() {
    let html = page(
        "# T</title><script>x()</script>\n\nA \"quote\" & </script> in text.\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| quote | Said \"</script><script>x()</script>\" |\n",
    );
    assert!(!html.contains("<script>x()"), "no script from the text");
    assert!(
        !index(&html).contains("</script"),
        "the index cannot close its script"
    );
}

// What the earlier renderer got wrong.

#[test]
fn bare_brackets_are_text() {
    let html = page("# T\n\nMarks [V], [M] and [H] are evidence grades; [see](#t) is a link.\n");
    let art = article(&html);
    assert!(art.contains("[V], [M] and [H]"), "{art}");
    assert!(art.contains("<a href=\"#t\">see</a>"), "{art}");
}

#[test]
fn pathological_brackets_finish_quickly() {
    let evil = "[V] ".repeat(4000) + &"[".repeat(5000) + &"](".repeat(2000) + &"*a ".repeat(3000);
    let started = std::time::Instant::now();
    let html = page(&format!("# T\n\n{evil}\n"));
    assert!(
        started.elapsed().as_secs() < 10,
        "took {:?}",
        started.elapsed()
    );
    assert!(
        html.len() < 2_000_000,
        "the output stays proportionate: {} bytes",
        html.len()
    );
}

#[test]
fn nested_lists_nest() {
    let html = page("# T\n\n- one\n  - one a\n  - one b\n- two\n");
    assert!(
        article(&html)
            .contains("<ul><li>one\n<ul><li>one a</li><li>one b</li></ul></li><li>two</li></ul>"),
        "{}",
        article(&html)
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
fn front_matter_sets_the_page() {
    let html = page(
        "---\ntitle: Die Pläne\nlang: de\nbrand: Maplewood\nkicker: Pläne\nfooter: Nur für die *Familie*.\ndescription: Die Zeichnungen.\n---\n\n# Überschrift\n\n> **Hinweis:** Schematisch.\n\n## Glossar\n\n| Begriff | Bedeutung |\n|---|---|\n| ADU | Eine Einliegerwohnung. |\n\n## Eins\n\nDie ADU.\n",
    );
    assert!(html.contains("<html lang=\"de\""));
    assert!(html.contains("<title>Die Pläne</title>"));
    assert!(html.contains("</svg>\nMaplewood</a><span class=\"kicker\">Pläne</span>"));
    assert!(html.contains("<footer class=\"fine\">Nur für die <em>Familie</em>.</footer>"));
    assert!(html.contains("<span class=\"mark\">Hinweis</span>"));
    assert!(html.contains(">Begriffe<a class=\"anchor\""));
    assert!(html.contains("content=\"Die Zeichnungen.\""));
    assert!(html.contains("Diese Seite durchsuchen"));
}

#[test]
fn the_build_scripts_on_this_machine_still_find_their_marks() {
    // ~/ADU/web/build.py, ~/reports/pet-web/build.sh and the medical pages
    // rewrite these exact strings after rendering.
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
fn links_to_markdown_files_and_settings_warnings() {
    let w =
        warnings("---\ncolour: red\n---\n# T\n\n[gone](#nowhere)\n\n[x](javascript:alert(1))\n");
    assert!(
        w.iter().any(|m| m.contains("unknown setting `colour`")),
        "{w:?}"
    );
    assert!(w.iter().any(|m| m.contains("#nowhere")), "{w:?}");
    assert!(w.iter().any(|m| m.contains("`javascript:`")), "{w:?}");
}

#[test]
fn the_csp_lets_exactly_the_page_run() {
    let settings = Settings {
        glossary: Some(false),
        csp: Some(true),
        ..Settings::default()
    };
    let html = page_with("# T\n\n![f](a.png)\n", &settings);
    let policy = html
        .split("Content-Security-Policy\" content=\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .unwrap_or_default();
    assert!(
        policy.starts_with("default-src 'none'; script-src 'sha256-"),
        "{policy}"
    );
    assert!(policy.contains("connect-src 'none'"));
    // One hash per executable script: theme, behave, page, figure.
    assert_eq!(
        policy
            .split("; ")
            .next()
            .map(|_| policy.matches("'sha256-").count()),
        Some(5),
        "{policy}"
    );
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
fn a_chapter_about_the_glossary_is_not_the_glossary() {
    let html = page(
        "# T\n\n## 5. The shared glossary\n\nWords explained once.\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| once | One time. |\n",
    );
    assert!(html.contains("<h2 id=\"5-the-shared-glossary\">"));
    assert!(html.contains("<dt id=\"g-once\">"));
}
