// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A diagram's title read from its front matter or its `title` statement,
//! told apart from the name md2html builds, and written as text.

use super::{Caption, Name, front};
use crate::mermaid::{Outcome, render};

/// The title front matter gives, if any.
///
/// # Panics
///
/// Panics when `src` is front matter that never closes.
fn titled(src: &str) -> Option<String> {
    front(src)
        .expect("front matter that closes")
        .caption
        .map(|caption| caption.as_str().to_owned())
}

/// A drawn diagram's HTML.
///
/// # Panics
///
/// Panics when `src` is not drawn, naming the kind or the reason.
fn drawn(src: &str) -> String {
    match render(src, 1) {
        Outcome::Drawn(drawing) => drawing.html,
        Outcome::Unsupported(kind) => panic!("unsupported {kind}"),
        Outcome::Unreadable(why) => panic!("unreadable: {why}"),
    }
}

/// The text of the SVG's `<title>`: the diagram's name for readers who
/// cannot see it.
///
/// # Panics
///
/// Panics when the drawing has no `<title>`.
fn name(html: &str) -> &str {
    html.split_once("<title id=\"dg1-t\">")
        .and_then(|(_, after)| after.split_once("</title>"))
        .map(|(words, _)| words)
        .expect("a title")
}

/// The caption under a drawing, if it has one.
fn caption(html: &str) -> Option<&str> {
    html.split_once("<figcaption dir=\"auto\">")
        .and_then(|(_, after)| after.split_once("</figcaption>"))
        .map(|(words, _)| words)
}

/// Front matter's `title` is the diagram's title, and the diagram is
/// what follows the front matter.
///
/// # Panics
///
/// When the title is not read, when the diagram after the front matter is
/// not what is left, or when a block without front matter loses anything.
#[test]
fn front_matter_gives_the_title_and_the_diagram_follows_it() {
    let src = "---\ntitle: Order flow\n---\nflowchart LR\n  A --> B\n";
    let opened = front(src).expect("closes");
    assert_eq!(
        opened.caption.as_ref().map(Caption::as_str),
        Some("Order flow")
    );
    assert_eq!(opened.body, "flowchart LR\n  A --> B\n");
    // Blank lines before it, an indented block, spaces after the dashes,
    // Windows line ends: Mermaid reads them all.
    assert_eq!(
        titled(
            "\n\n  ---  \r\n  title: Indented\r\n  config:\r\n    theme: dark\r\n  ---\r\nflowchart LR\r\n A-->B\r\n"
        ),
        Some(String::from("Indented"))
    );
    let plain = "flowchart LR\n  A --> B\n";
    let whole = front(plain).expect("no front matter");
    assert!(whole.caption.is_none());
    assert_eq!(whole.body, plain);
}

/// A title is read as YAML writes it: quoted, folded, after a comment;
/// only at the top level; and front matter that never closes is refused.
///
/// # Panics
///
/// When a quoted, block or plain title is read otherwise, when a nested
/// `title` is taken for the diagram's, or when unclosed front matter is
/// read.
#[test]
fn a_title_is_read_as_yaml_writes_it() {
    let one = |yaml: &str| titled(&format!("---\n{yaml}\n---\nflowchart LR\n A-->B\n"));
    assert_eq!(
        one("title: \"Say \\\"hi\\\"\""),
        Some(String::from("Say \"hi\""))
    );
    assert_eq!(
        one("title: 'It''s # here'"),
        Some(String::from("It's # here"))
    );
    assert_eq!(
        one("title: Plain words # a comment"),
        Some(String::from("Plain words"))
    );
    assert_eq!(
        one("title: >-\n  Folded over\n  two lines"),
        Some(String::from("Folded over two lines"))
    );
    assert_eq!(
        one("title: Runs on\n  under it\nconfig: {}"),
        Some(String::from("Runs on under it"))
    );
    assert_eq!(one("title: 42"), Some(String::from("42")));
    assert_eq!(one("config:\n  title: Nested\ndisplayMode: compact"), None);
    assert_eq!(one("titles: Not the key"), None);
    assert_eq!(one("title:Not YAML's"), None);
    let refused = front("---\ntitle: Open\nflowchart LR\n A-->B\n").unwrap_err();
    assert!(refused.contains("closing"), "{refused}");
    assert!(matches!(
        render("---\ntitle: Open\nflowchart LR\n A-->B\n", 1),
        Outcome::Unreadable(why) if why.contains("closing")
    ));
}

/// A title of nothing, or of spaces, is no title: no caption, and the
/// name is the one built from the nodes.
///
/// # Panics
///
/// When a blank title gives a caption, or the drawing's name is not built
/// from its nodes.
#[test]
fn a_blank_title_is_no_title() {
    for yaml in ["title:", "title:    ", "title: \"\"", "title: '  '"] {
        assert_eq!(
            titled(&format!("---\n{yaml}\n---\nflowchart LR\n A-->B\n")),
            None,
            "{yaml}"
        );
        let html = drawn(&format!(
            "---\n{yaml}\n---\nflowchart LR\n  A[Start] --> B[End]\n"
        ));
        assert_eq!(caption(&html), None, "{yaml}");
        assert_eq!(name(&html), "Start, End", "{yaml}");
    }
    let html = drawn("sequenceDiagram\n  title   \n  A->>B: hi\n");
    assert_eq!(caption(&html), None);
    assert_eq!(name(&html), "A, B");
}

/// A titled flowchart shows its title under it and is named by it; one
/// without a title has no caption and keeps the name built from its nodes.
///
/// # Panics
///
/// When the caption is missing, out of the figure or not after the SVG;
/// when the name is not the title; or when an untitled diagram gets a
/// caption.
#[test]
fn a_titled_flowchart_has_its_caption_and_name() {
    let html = drawn("---\ntitle: Order flow\n---\nflowchart LR\n  A[Start] --> B[End]\n");
    assert_eq!(caption(&html), Some("Order flow"));
    assert!(
        html.ends_with("</svg><figcaption dir=\"auto\">Order flow</figcaption></figure>"),
        "{html}"
    );
    assert_eq!(name(&html), "Order flow");
    let bare = drawn("flowchart LR\n  A[Start] --> B[End]\n");
    assert_eq!(caption(&bare), None);
    assert!(bare.ends_with("</svg></figure>"));
    assert_eq!(name(&bare), "Start, End");
}

/// A sequence diagram takes its title from a `title` statement, the old
/// `title:` too, or its front matter; the statement replaces the front
/// matter's.
///
/// # Panics
///
/// When any of the four ways gives no caption or another one.
#[test]
fn a_sequence_diagram_takes_its_title_statement() {
    let statement = drawn("sequenceDiagram\n  title Paying\n  Alice->>Bob: Pay\n");
    assert_eq!(caption(&statement), Some("Paying"));
    assert_eq!(name(&statement), "Paying");
    let older = drawn("sequenceDiagram\n  title: Paying, older\n  Alice->>Bob: Pay\n");
    assert_eq!(caption(&older), Some("Paying, older"));
    let fronted = drawn("---\ntitle: From the front\n---\nsequenceDiagram\n  Alice->>Bob: Pay\n");
    assert_eq!(caption(&fronted), Some("From the front"));
    let both = drawn(
        "---\ntitle: From the front\n---\nsequenceDiagram\n  title From the body\n  Alice->>Bob: Pay\n",
    );
    assert_eq!(caption(&both), Some("From the body"));
}

/// The author's accessible title names the drawing; their title is still
/// the caption. Neither is the name built from the parts.
///
/// # Panics
///
/// When `accTitle` does not name a flowchart or a sequence diagram, when
/// it becomes a caption, or when [`Name`] does not say whose a name is.
#[test]
fn the_accessible_title_names_the_drawing_and_the_title_is_its_caption() {
    let flow = drawn("---\ntitle: Seen\n---\nflowchart LR\n  accTitle: Heard\n  A --> B\n");
    assert_eq!((name(&flow), caption(&flow)), ("Heard", Some("Seen")));
    let seq = drawn("sequenceDiagram\n  title Seen\n  accTitle : Heard\n  A->>B: hi\n");
    assert_eq!((name(&seq), caption(&seq)), ("Heard", Some("Seen")));
    let heard = drawn("flowchart LR\n  accTitle: Heard\n  A --> B\n");
    assert_eq!((name(&heard), caption(&heard)), ("Heard", None));
    let built = || String::from("A, B");
    let seen = Caption::new("Seen");
    assert_eq!(
        Name::of(None, None, built),
        Name::Built(String::from("A, B"))
    );
    assert_eq!(
        Name::of(None, seen.as_ref(), built),
        Name::Author(String::from("Seen"))
    );
    assert_eq!(
        Name::of(Some(" "), seen.as_ref(), built),
        Name::Author(String::from("Seen"))
    );
    assert_eq!(
        Name::of(Some("Heard"), seen.as_ref(), built),
        Name::Author(String::from("Heard"))
    );
}

/// A title is text: markup in it is escaped in the caption and the name,
/// in either kind of diagram.
///
/// # Panics
///
/// When any `<` of the title reaches the drawing as a tag.
#[test]
fn a_title_with_markup_is_text() {
    let attack = "</figcaption><script>alert(1)</script><img src=x onerror=alert(2)> & \"q\"";
    for src in [
        format!("---\ntitle: '{attack}'\n---\nflowchart LR\n  A --> B\n"),
        format!("sequenceDiagram\n  title {attack}\n  A->>B: hi\n"),
    ] {
        let html = drawn(&src);
        assert!(
            !html.contains("<script") && !html.contains("<img"),
            "{html}"
        );
        let shown = caption(&html).expect("a caption");
        assert_eq!(
            shown,
            "&lt;/figcaption&gt;&lt;script&gt;alert(1)&lt;/script&gt;&lt;img src=x onerror=alert(2)&gt; &amp; &quot;q&quot;"
        );
        assert_eq!(name(&html), shown);
        assert_eq!(html.matches("</figcaption>").count(), 1);
    }
}

/// A very long title is drawn whole and costs no more than its words: in
/// the name and the caption, and nothing that grows with it besides.
///
/// # Panics
///
/// When the title is cut, or the drawing grows by more than twice the
/// title's length.
#[test]
fn a_very_long_title_costs_its_words() {
    let words = "word ".repeat(40_000);
    let long = words.trim_end();
    let bare = drawn("flowchart LR\n  A --> B\n").len();
    let html = drawn(&format!(
        "---\ntitle: {long}\n---\nflowchart LR\n  A --> B\n"
    ));
    assert_eq!(caption(&html), Some(long));
    assert_eq!(name(&html), long);
    assert!(
        html.len() <= bare + 2 * long.len() + 64,
        "{} against {bare}",
        html.len()
    );
}
