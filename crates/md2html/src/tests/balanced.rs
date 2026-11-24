// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The author's tags cannot close or leave open anything of md2html's.
//!
//! Whatever raw HTML a text holds, the page around the article keeps its
//! shape, the article's elements nest, and so do a fragment's and a
//! book's.

use std::path::Path;

use super::{fragment_with, frame, own_glossary_only, page_with, running_code, warnings};
use crate::config::Settings;
use crate::document::{Shape, render_doc};

/// Raw HTML that does not balance, in every place md2html writes an
/// element of its own around the author's text.
const CASES: [&str; 22] = [
    "</div></div></main>",
    "x </div> y </details> z </p>",
    "- item </div>\n- <div>open\n- <details><summary>s</summary>",
    "> </div>\n> <div>open",
    "> a\n> > <details>deeper",
    "> [!NOTE]\n> </div> <details>",
    "> **Note.** </p> <div>",
    "| a | b |\n|---|---|\n| </div> | <div>x |",
    "Note[^1].\n\n[^1]: </div> <div>open",
    "*a </div> <span>b* c</span>",
    "[link <div>](https://e.org) </a>",
    "<div><p>a</div></p>",
    "<details>\n<summary>S</summary>\n\nInside, never closed.",
    "<div>\n\nnever closed",
    "</details>\n\n<details open>",
    "a<br>b<hr/>c<div/>d<wbr>",
    "<p>a<p>b\n\n- c",
    "<table><tr><td>a<td>b</table>",
    "<ul><li>a<li>b</ul>",
    "</article></main></body></html>",
    "<main><article>inner</article></main>\n\n<section><header>open",
    "</div><script>alert(1)</script><div onclick=\"alert(1)\"><svg onload=alert(1)><circle r=\"1\"/>",
];

/// A text with `case` in its first chapter, between a lead and a second
/// chapter with a footnote (so the outline is the same whether the case
/// adds a footnote or not).
fn text(case: &str) -> String {
    format!("# T\n\nLead.\n\n## One\n\n{case}\n\n## Two\n\nThe words after.[^z]\n\n[^z]: A note.\n")
}

/// The settings of a page, with `--html` or without.
fn settings(rich: bool) -> Settings {
    Settings {
        html: rich.then_some(true),
        ..own_glossary_only()
    }
}

/// The XHTML of the chapter of the book of `markdown`.
///
/// # Panics
///
/// When no book is made, or it holds no `OEBPS/content.xhtml` that is a
/// whole XML document.
fn chapter(markdown: &str, settings: &Settings) -> String {
    let book = render_doc(
        &format!("---\nepub: true\n---\n{markdown}"),
        None,
        Some(Path::new("book.html")),
        settings,
        Shape::Page,
        None,
    )
    .epub
    .expect("a book");
    let raw = String::from_utf8_lossy(&book);
    let (_, file) = raw
        .split_once("OEBPS/content.xhtml")
        .expect("the chapter in the book");
    let (_, xml) = file.split_once("<?xml").expect("an XML document");
    let (body, _) = xml.split_once("</html>").expect("the whole document");
    format!("<?xml{body}</html>")
}

/// However the author's tags fail to balance, the page around the article
/// is the page of a text that has none, and every element nests: the
/// page's, the fragment's, the book's.
///
/// # Panics
///
/// When, by default or with `--html`, a case changes the frame of the page
/// (an element of md2html's closed early, left open, or added outside the
/// article); when an element of the page, the fragment or the book does not
/// nest; when the words after the case are lost from any of them; or when
/// running code reaches the page or the fragment.
#[test]
fn the_page_around_the_article_keeps_its_shape() {
    for rich in [false, true] {
        let settings = settings(rich);
        let clean = frame(&page_with(&text("clean"), &settings), "clean");
        let clean_book = frame(&chapter(&text("clean"), &settings), "clean book");
        for case in CASES {
            let shown = format!("{case} (--html: {rich})");
            let page = page_with(&text(case), &settings);
            assert_eq!(frame(&page, &shown), clean, "the frame of {shown}");
            let fragment = fragment_with(&text(case), &settings);
            assert!(
                !frame(&fragment, &shown).is_empty(),
                "the fragment's blocks of {shown}"
            );
            assert!(fragment.contains("The words after."), "{shown}: {fragment}");
            for written in [&page, &fragment] {
                let found = running_code(written);
                assert!(found.is_none(), "{found:?} in {shown}");
            }
            let xhtml = chapter(&text(case), &settings);
            assert_eq!(frame(&xhtml, &shown), clean_book, "the book of {shown}");
            assert!(xhtml.contains("The words after."), "{shown}: {xhtml}");
        }
    }
}

/// What balancing changed is told to the author, at the line of the tag.
///
/// # Panics
///
/// When a stray `</div>` is not named on its line as left out, an unclosed
/// `<details>` not named on its line as closed by md2html, or when a
/// paragraph or list item left open (which HTML allows) is told.
#[test]
fn the_author_is_told_what_was_mended() {
    let said = warnings("# T\n\nLead.\n\n</div>\n\n<details>\n\n<p>a<p>b\n\n<ul><li>c<li>d</ul>\n");
    assert!(
        said.iter()
            .any(|message| message == "`</div>` closes nothing that is open here; it is left out"),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|message| message == "`<details>` is not closed; md2html closes it"),
        "{said:?}"
    );
    assert_eq!(said.len(), 2, "{said:?}");
    let lines: Vec<_> = render_doc(
        "# T\n\nLead.\n\n</div>\n\n<details>\n",
        None,
        None,
        &own_glossary_only(),
        Shape::Page,
        None,
    )
    .warnings
    .into_iter()
    .filter_map(|warning| warning.line)
    .map(core::num::NonZeroUsize::get)
    .collect();
    assert_eq!(lines, [5, 7], "each on its line");
}

/// A disclosure the author writes across blocks, with Markdown inside it,
/// is kept exactly as written.
///
/// # Panics
///
/// When `<details>` opened in one HTML block and closed in a later one, a
/// list and a paragraph between them, is closed early, mended, or told.
#[test]
fn a_disclosure_across_blocks_is_kept() {
    let markdown = "# T\n\nLead.\n\n<details>\n<summary>More</summary>\n\n- one\n- two\n\nText.\n\n</details>\n\nAfter.\n";
    let page = page_with(markdown, &own_glossary_only());
    assert!(
        page.contains(
            "<details>\n<summary>More</summary><ul><li>one</li><li>two</li></ul><p>Text.</p></details><p>After.</p>"
        ),
        "{page}"
    );
    assert!(warnings(markdown).is_empty(), "nothing to tell");
}
