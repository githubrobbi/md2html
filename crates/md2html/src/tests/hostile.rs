// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Hostile documents: no script from the text, no way out of the page,
//! no input that takes long or breaks the tool.

use core::time::Duration;
use std::time::Instant;

use super::{article, index, own_glossary_only, page, page_with, running_code, tags, warnings};
use crate::config::Settings;
use crate::text::put;

/// No link or image from the text points at a script or a data page.
///
/// # Panics
///
/// When a `javascript:` link in any case, with leading spaces or hidden
/// behind a character reference, a `vbscript:` or `data:` link, an
/// autolink, an image or a reference definition reaches the article as an
/// `href` or `src` that runs, or when the page has no article.
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
            !art.contains("href=\"javascript") && !art.contains("src=\"javascript"),
            "{evil}: {art}"
        );
        assert!(
            !art.contains("href=\"vbscript") && !art.contains("href=\"data:"),
            "{evil}"
        );
    }
}

/// Raw HTML in the text builds no tag that runs anything.
///
/// # Panics
///
/// When a `<script>`, `<iframe>`, `<style>` or `<svg>` element, an image
/// with an event handler, any `on...` attribute, or a `javascript:` URL from
/// the text reaches the article as a real tag, or when the page has no
/// article.
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
        "<style>body { display: none }</style>",
    ] {
        let html = page(&format!("# T\n\n{evil}\n"));
        // The tags the browser will build: no handler, no script, no frame.
        for tag in tags(&article(&html)) {
            let name = tag
                .trim_start_matches('<')
                .split(|ch: char| ch.is_whitespace() || ch == '>')
                .next()
                .unwrap_or_default();
            assert!(
                !(["script", "iframe", "style", "svg"].contains(&name)
                    || (name == "img" && tag.contains(" on"))),
                "{evil}: {tag}"
            );
            assert!(
                !tag.split_whitespace()
                    .skip(1)
                    .any(|attr| attr.starts_with("on")),
                "{evil}: {tag}"
            );
            assert!(!tag.contains("javascript:"), "{evil}: {tag}");
        }
    }
}

/// The harmless inline and block HTML of a text is kept as HTML.
///
/// # Panics
///
/// When `<kbd>`, `<sub>`, `<br>` or an open `<details>` with its
/// `<summary>` is escaped or dropped instead of kept, or when the page has
/// no article.
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

/// Text cannot close the page's title or scripts.
///
/// # Panics
///
/// When a `</title><script>` in the heading or the glossary reaches the
/// page as a running script, or when a `</script` reaches the search
/// index, where it would close the index's script element.
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

/// A setting or a link that cannot be kept is named to the author.
///
/// # Panics
///
/// When the author is not told of the unknown setting `colour`, of the link
/// to `#nowhere` that no heading has, or of the `javascript:` link left out.
#[test]
fn settings_and_links_that_cannot_be_kept_are_named() {
    let said =
        warnings("---\ncolour: red\n---\n# T\n\n[gone](#nowhere)\n\n[x](javascript:alert(1))\n");
    assert!(
        said.iter()
            .any(|message| message.contains("unknown setting `colour`")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|message| message.contains("#nowhere")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|message| message.contains("`javascript:`")),
        "{said:?}"
    );
}

/// The Content-Security-Policy lets the page's own code run and nothing else.
///
/// # Panics
///
/// When the policy does not start from `default-src 'none'` with scripts
/// allowed by hash only, when it lets the page connect anywhere, or when it
/// does not hold exactly five hashes: one for each of the page's four
/// scripts and one for its style.
#[test]
fn the_csp_lets_exactly_the_page_run() {
    let settings = Settings {
        csp: Some(true),
        ..own_glossary_only()
    };
    let html = page_with("# T\n\n![f](a.png)\n", &settings);
    let policy = html
        .split_once("Content-Security-Policy\" content=\"")
        .and_then(|(_, after)| after.split('"').next())
        .unwrap_or_default();
    assert!(
        policy.starts_with("default-src 'none'; script-src 'sha256-"),
        "{policy}"
    );
    assert!(policy.contains("connect-src 'none'"));
    // One hash per executable script (theme, behave, page, figure) and one
    // for the style.
    assert_eq!(policy.matches("'sha256-").count(), 5, "{policy}");
}

/// No document, with or without `--html`, puts running code in its page.
///
/// # Panics
///
/// When an event handler, a `<script>` or `<iframe>`, or a `javascript:`
/// URL (in a link, an attribute or a style's `url()`) from the front matter,
/// the heading, the text, raw HTML or a mermaid diagram reaches a real tag
/// of the head or the article, by default or with `--html`.
#[test]
fn no_document_puts_running_code_in_its_page() {
    let attack = "---\ntitle: \"</title><script>alert(1)</script>\"\nbrand_link: \"javascript:alert(1)\"\nauthor: \"\\\"><script>alert(1)</script>\"\n\
footer: \"[f](javascript:alert(1)) <img src=x onerror=alert(1)>\"\n---\n# H <img src=x onerror=alert(1)>\n\n\
[a](javascript:alert(1)) [b](JaVaScRiPt:alert(1)) [c](java&#115;cript:alert(1)) [d](data:text/html,x)\n\n\
<a href=\"javascript:alert(1)\">x</a> <svg onload=alert(1)> <iframe src=//e></iframe> <details ontoggle=alert(1)>z</details>\n\n\
<script>alert(1)</script>\n\n<div onclick=\"alert(1)\" style=\"background:url(javascript:alert(1))\">d</div>\n\n\
```mermaid\nflowchart LR\n  A[\"<img src=x onerror=alert(1)>\"] --> B\n  click A \"javascript:alert(1)\"\n```\n";
    for html_flag in [None, Some(true)] {
        let settings = Settings {
            html: html_flag,
            ..own_glossary_only()
        };
        let mode = if html_flag.is_some() {
            "--html"
        } else {
            "default"
        };
        let found = running_code(&page_with(attack, &settings));
        assert!(found.is_none(), "{found:?} ({mode})");
    }
}

/// With `--html`, harmless HTML is kept and a new tab cannot reach back.
///
/// # Panics
///
/// When a figure with its image (and its width) and caption is not kept
/// as written, or when a link that opens a new tab does not get
/// `rel="noopener noreferrer"`.
#[test]
fn rich_html_keeps_what_is_harmless() {
    let settings = Settings {
        html: Some(true),
        ..own_glossary_only()
    };
    let html = page_with(
        "# T\n\n<figure><img src=\"a.png\" alt=\"A\" width=\"300\"><figcaption>Cap</figcaption></figure>\n\n<a href=\"https://e.org\" target=\"_blank\">e</a>\n",
        &settings,
    );
    assert!(
        html.contains("<img src=\"a.png\" alt=\"A\" width=\"300\">")
            && html.contains("<figcaption>Cap</figcaption>")
    );
    assert!(
        html.contains(
            "<a href=\"https://e.org\" target=\"_blank\" rel=\"noopener noreferrer\">e</a>"
        )
    );
}

/// Thousands of brackets and emphasis openers render in time, in proportion.
///
/// # Panics
///
/// When a line of thousands of `[V]`, `[`, `](` and `*a` takes 10 seconds
/// or more to render, or makes a page of 2 MB or more.
#[test]
fn pathological_brackets_finish_quickly() {
    let evil = [
        "[V] ".repeat(4000),
        "[".repeat(5000),
        "](".repeat(2000),
        "*a ".repeat(3000),
    ]
    .concat();
    let started = Instant::now();
    let html = page(&format!("# T\n\n{evil}\n"));
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(10),
        "took {} ms",
        took.as_millis()
    );
    assert!(
        html.len() < 2_000_000,
        "the output stays proportionate: {} bytes",
        html.len()
    );
}

/// Inputs built to be slow render within seconds, and whole.
///
/// # Panics
///
/// When deep runs of `*`, unmatched emphasis, nested images, `[`, quotes
/// `>` a hundred thousand deep, or forty thousand reference definitions
/// take 5 seconds or more to render, or the page is not written from its
/// doctype to its end.
#[test]
fn pathological_inputs_render_quickly_and_whole() {
    let references = (0_i32..40_000_i32).fold(String::new(), |mut text, number| {
        put!(text, "[r{number}]: /u{number}\n");
        text
    });
    let cases = [
        ["*".repeat(100_000), String::from("a"), "*".repeat(100_000)].concat(),
        ["*a ".repeat(30_000), "a* ".repeat(30_000)].concat(),
        "*a* ".repeat(100_000),
        [
            "![".repeat(20_000),
            String::from("x"),
            "](a)".repeat(20_000),
        ]
        .concat(),
        "[".repeat(100_000),
        [">".repeat(100_000), String::from(" deep")].concat(),
        references,
    ];
    for case in &cases {
        let started = Instant::now();
        let html = page(case);
        let took = started.elapsed();
        let opening: String = case.chars().take(40).collect();
        assert!(
            took < Duration::from_secs(5),
            "{opening}: took {} ms",
            took.as_millis()
        );
        assert!(
            html.starts_with("<!doctype html>") && html.ends_with("</body></html>"),
            "{opening}: the page is written to its end"
        );
    }
}

/// Renders `case`, and says how long it took.
///
/// # Panics
///
/// When the page of `case` is not written to its end.
fn render_time(case: &str) -> Duration {
    let started = Instant::now();
    let html = page(case);
    let took = started.elapsed();
    let opening: String = case.chars().take(40).collect();
    assert!(
        html.ends_with("</body></html>"),
        "{opening}: the page is written to its end"
    );
    took
}

/// An opener that never closes is not searched for again.
///
/// # Panics
///
/// When forty thousand `[` before as many `]`, thirty thousand footnote
/// openers, or a thousand backtick runs of new lengths before 4 MB of text
/// take 5 seconds or more, or the page is not written to its end.
#[test]
fn openers_that_never_close_cost_no_second_search() {
    // Every `]` used to fold the whole bracket text before it into a
    // label to look up; every backtick run of a new length used to search
    // to the end of the text for its match.
    let backtick_runs: String = (1..1_000_usize)
        .map(|length| ["`".repeat(length), String::from(" a ")].concat())
        .collect();
    let cases = [
        ["[".repeat(40_000), "]".repeat(40_000)].concat(),
        ["[^".repeat(30_000), "]".repeat(30_000)].concat(),
        [backtick_runs, "b".repeat(4_000_000)].concat(),
    ];
    for case in &cases {
        let took = render_time(case);
        let opening: String = case.chars().take(12).collect();
        assert!(
            took < Duration::from_secs(5),
            "{opening}: took {} ms",
            took.as_millis()
        );
    }
}

/// Dollar signs that open no maths are read in linear time.
///
/// # Panics
///
/// When one of the shapes of `$` that come nearest to a second search
/// (escaped dollars, dollars in code spans and URLs, a `$$` before many
/// single ones) takes 5 seconds or more, or the page is not written to its
/// end.
#[test]
fn dollar_signs_that_open_no_maths_stay_cheap() {
    // A search for the closing `$` stops at the next `$` it does not skip
    // as escaped, and the tokens never open maths at one it skipped: the
    // searches read disjoint stretches. These are the shapes that come
    // nearest to breaking that.
    let cases = [
        "$a ".repeat(100_000),
        ["$$ ", &"$ a ".repeat(100_000)].concat(),
        "\\$a $".repeat(60_000),
        ["x $", &"a \\$ ".repeat(80_000)].concat(),
        "$x `$` ".repeat(50_000),
        "$x http://a.b\\ $".repeat(40_000),
        "$x\\\\$5 ".repeat(50_000),
    ];
    for case in &cases {
        let took = render_time(case);
        let opening: String = case.chars().take(12).collect();
        assert!(
            took < Duration::from_secs(5),
            "{opening}: took {} ms",
            took.as_millis()
        );
    }
}
