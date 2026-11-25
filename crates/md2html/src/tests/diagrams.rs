// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A diagram's title as its caption: on the page, in the book, and what
//! each way of showing diagrams does with it.

use std::path::Path;

use super::{Scratch, article, assert_nested, fragment, own_glossary_only, page, page_with};
use crate::config::{Diagrams, Settings};
use crate::document::{Shape, render_doc};

/// A page with a titled flowchart, a sequence diagram titled by its own
/// statement, and a flowchart without a title.
const TITLED: &str = "# T\n\n```mermaid\n---\ntitle: Order flow\n---\nflowchart LR\n  A[Start] --> B[End]\n```\n\n```mermaid\nsequenceDiagram\n  title Paying\n  Alice->>Bob: Pay\n```\n\n```mermaid\nflowchart LR\n  C[Left] --> D[Right]\n```\n";

/// The rule that styles a diagram's caption.
const RULE: &str = "figure.diagram>figcaption{";

/// The page's `<style>`.
///
/// # Panics
///
/// When the page has no `<style>`.
fn style(html: &str) -> &str {
    html.split_once("<style>")
        .and_then(|(_, after)| after.split_once("</style>"))
        .map(|(css, _)| css)
        .expect("a style")
}

/// Each titled diagram shows its title under it, in its figure, styled as
/// a picture's caption; the diagram without one has none.
///
/// # Panics
///
/// When a caption is missing, out of its figure or given to the untitled
/// diagram; when a drawing is not named by its title; when the rule for
/// captions is not in the page once; when the page does not enlarge its
/// diagrams; or when the elements do not nest.
#[test]
fn a_titled_diagram_has_its_caption_on_the_page() {
    let html = page(TITLED);
    let art = article(&html);
    assert!(
        art.contains("</svg><figcaption dir=\"auto\">Order flow</figcaption></figure>"),
        "{art}"
    );
    assert!(art.contains("</svg><figcaption dir=\"auto\">Paying</figcaption></figure>"));
    assert_eq!(art.matches("<figcaption").count(), 2, "the third has none");
    assert!(art.contains("<title id=\"dg1-t\">Order flow</title>"));
    assert!(art.contains("<title id=\"dg2-t\">Paying</title>"));
    assert!(art.contains("<title id=\"dg3-t\">Left, Right</title>"));
    assert_eq!(style(&html).matches(RULE).count(), 1, "the rule, once");
    assert!(
        html.contains("figure.diagram>svg"),
        "the enlargement's script"
    );
    assert_nested(&art, "titled diagrams");
}

/// A page whose diagrams have no title carries no caption and no rule for
/// one.
///
/// # Panics
///
/// When an untitled diagram's page has a caption or the caption's rule.
#[test]
fn an_untitled_diagram_has_no_caption() {
    let html = page("# T\n\n```mermaid\nflowchart LR\n  A --> B\n```\n");
    assert!(!html.contains("<figcaption"));
    assert!(!html.contains(RULE));
}

/// Left to mermaid.js, the diagram keeps its front matter for the script,
/// which draws the title itself; shown as code, or in a fragment, it is
/// code and has no caption.
///
/// # Panics
///
/// When the code given to mermaid.js loses its front matter, when md2html
/// adds a caption the script would draw again, or when code mode or a
/// fragment gives a caption.
#[test]
fn mermaid_js_code_and_fragments_keep_the_front_matter_as_written() {
    let scratch = Scratch::new("caption-js");
    let script = scratch.write("mermaid.js", "/* mermaid */");
    let js = Settings {
        mermaid: Some(Diagrams::Js),
        mermaid_js: Some(script),
        ..own_glossary_only()
    };
    let left = page_with(TITLED, &js);
    assert!(
        left.contains("<pre class=\"mermaid\">---\ntitle: Order flow\n---\nflowchart LR\n  A[Start] --&gt; B[End]\n</pre>"),
        "{}",
        article(&left)
    );
    assert!(!left.contains("<figcaption") && !left.contains(RULE));
    let code = Settings {
        mermaid: Some(Diagrams::Code),
        ..own_glossary_only()
    };
    let shown = page_with(TITLED, &code);
    assert!(shown.contains("---\ntitle: Order flow\n---"));
    assert!(!shown.contains("<figcaption") && !shown.contains(RULE));
    let bare = fragment(TITLED);
    assert!(bare.contains("---\ntitle: Order flow\n---") && !bare.contains("<figcaption"));
}

/// The book shows the caption under the diagram too, in XHTML, and its
/// stylesheet styles it.
///
/// # Panics
///
/// When no book is made, or its chapter lacks the caption or its
/// stylesheet the rule.
#[test]
fn a_book_shows_the_caption() {
    let book = render_doc(
        &format!("---\nepub: true\n---\n{TITLED}"),
        None,
        Some(Path::new("book.html")),
        &own_glossary_only(),
        Shape::Page,
        None,
    )
    .epub
    .expect("a book");
    let raw = String::from_utf8_lossy(&book);
    let (_, chapter) = raw
        .split_once("OEBPS/content.xhtml")
        .expect("the chapter in the book");
    assert!(chapter.contains("</svg><figcaption dir=\"auto\">Order flow</figcaption></figure>"));
    let (_, css) = raw.split_once("OEBPS/style.css").expect("the stylesheet");
    assert!(css.contains(RULE));
}
