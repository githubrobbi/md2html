// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a line of Markdown becomes.
//!
//! Autolinks, bare links, strikethrough, code spans, maths, footnote
//! references, link labels and destinations, and an image's `{...}`
//! attributes: each case is a rule a reader relies on that the
//! specification's examples leave open.

use super::{fragment, fragment_with, page};
use crate::config::Settings;

/// An address in `<...>` is a mail link only when its host is a name: no
/// label empty, longer than 63 characters, or starting or ending with `-`.
///
/// # Panics
///
/// When `<a@b.c>` or a host label of 63 characters is not a mail link, or
/// one of the addresses with a host that is not a name is.
#[test]
fn a_mail_autolink_needs_a_host_name() {
    let longest = format!("a@{}.c", "b".repeat(63));
    let mail = fragment(&format!("<a@b.c> <{longest}>"));
    assert!(
        mail.contains("<a href=\"mailto:a@b.c\">a@b.c</a>"),
        "{mail}"
    );
    assert!(
        mail.contains(&format!("href=\"mailto:{longest}\"")),
        "{mail}"
    );
    let long = format!("<a@{}.c>", "b".repeat(64));
    for not_mail in [
        "<a@-b.c>",
        "<a@b-.c>",
        "<a@b..c>",
        "<a@b_c.d>",
        long.as_str(),
    ] {
        let html = fragment(not_mail);
        assert!(!html.contains("mailto:"), "{not_mail}: {html}");
    }
}

/// A bare link needs a host of letters, digits, `-`, `_`, `.` and `:`;
/// one that starts with `www.` needs a dot after it, and one with a
/// scheme does not.
///
/// # Panics
///
/// When `www.a.b` or `https://localhost/x` is not a link, or
/// `https://a!b.com` or `www.example` is.
#[test]
fn a_bare_link_needs_a_host() {
    let links = fragment("www.a.b and https://localhost/x");
    assert!(
        links.contains("<a href=\"http://www.a.b\">www.a.b</a>"),
        "{links}"
    );
    assert!(
        links.contains("<a href=\"https://localhost/x\">https://localhost/x</a>"),
        "{links}"
    );
    for not_link in ["https://a!b.com", "www.example"] {
        let html = fragment(not_link);
        assert!(!html.contains("<a "), "{not_link}: {html}");
    }
}

/// A bare link leaves out what ends the sentence around it: a `)` that
/// opens nowhere in it and an entity reference, but keeps a `)` it opened.
///
/// # Panics
///
/// When `https://a.b/c_(d)` loses its `)`, `(https://a.b/e)` keeps the
/// outer one, or `&hl;` is taken into the link.
#[test]
fn a_bare_link_ends_before_the_sentence_does() {
    let html = fragment("see https://a.b/c_(d) and (https://a.b/e) and www.g.com/s?q=c&hl; ok");
    assert!(
        html.contains("<a href=\"https://a.b/c_(d)\">https://a.b/c_(d)</a>"),
        "{html}"
    );
    assert!(
        html.contains("(<a href=\"https://a.b/e\">https://a.b/e</a>)"),
        "{html}"
    );
    assert!(
        html.contains("<a href=\"http://www.g.com/s?q=c\">www.g.com/s?q=c</a>&amp;hl;"),
        "{html}"
    );
}

/// Strikethrough is two tildes on each side; one tilde, or two on one side
/// only, is text.
///
/// # Panics
///
/// When `~a~`, `~~a~` or `~a~~` strikes anything, or `~~b~~` does not.
#[test]
fn strikethrough_takes_two_tildes_each_side() {
    assert_eq!(fragment("~~b~~"), "<p><del>b</del></p>\n");
    for text in ["~a~", "~~a~", "~a~~"] {
        assert_eq!(fragment(text), format!("<p>{text}</p>\n"));
    }
}

/// A code span loses one space on each side only when it holds more than
/// space; a line break inside it is a space.
///
/// # Panics
///
/// When a span holding two backticks between spaces does not show the two
/// backticks, a span of two spaces is shortened, or a span of `a`, a line
/// break and `b` between spaces keeps its spaces or its line break.
#[test]
fn a_code_span_drops_one_space_each_side() {
    assert_eq!(
        fragment("` `` ` and `  ` and ` a\nb `"),
        "<p><code>``</code> and <code>  </code> and <code>a b</code></p>\n"
    );
}

/// `$$...$$` inside a line is display maths, unless it holds nothing.
///
/// # Panics
///
/// When `$$x^2$$` is not a block formula, or `$$ $$` becomes one.
#[test]
fn display_maths_inside_a_line() {
    let html = fragment("a $$x^2$$ b $$ $$ c");
    assert!(html.starts_with("<p>a <math display=\"block\">"), "{html}");
    assert!(html.ends_with("</math> b $$ $$ c</p>\n"), "{html}");
}

/// A formula md2html cannot read is shown as its TeX where it stands: as
/// code in the line inside a paragraph, displayed or not, and as a block of
/// code only when it is a block of its own.
///
/// # Panics
///
/// When an unreadable `$$...$$` or `$...$` in a line is not code inside its
/// paragraph (a `pre` there ends the paragraph in a browser and makes the
/// book invalid), in the page or the fragment, or an unreadable `$$` block
/// is not a block of code.
#[test]
fn unreadable_maths_stays_where_it_stands() {
    let text = "a $$\\frac{x$$ b $\\frac{y$ c";
    let shown = "a <code class=\"tex\">\\frac{x</code> b <code class=\"tex\">\\frac{y</code> c</p>";
    assert_eq!(fragment(text), format!("<p>{shown}\n"));
    let whole = page(text);
    assert!(
        whole.contains(shown) && !whole.contains("<pre class=\"tex\">"),
        "the page's paragraph holds both"
    );
    assert_eq!(
        fragment("$$\n\\frac{x\n$$\n"),
        "<pre class=\"tex\">\\frac{x</pre>\n"
    );
}

/// Inline maths runs to the next `$` that is not escaped, across a line
/// break but not a blank line; it may not end on a space, nor be followed
/// by a digit; and `--no-math` leaves dollars alone.
///
/// # Panics
///
/// When `$a\$b$` is not one formula, `$a` and `b$` on two lines are not, a
/// blank line between them does not end it, `$x $` or `$5 and $10` is a
/// formula, or `$x$` is one with maths turned off.
#[test]
fn inline_maths_ends_where_it_should() {
    let escaped = fragment("p $a\\$b$ q");
    assert!(
        escaped.contains(
            "<annotation encoding=\"application/x-tex\">a\\$b</annotation></semantics></math> q"
        ),
        "{escaped}"
    );
    let two_lines = fragment("p $a\nb$ q");
    assert!(
        two_lines.contains("<annotation encoding=\"application/x-tex\">a\nb</annotation>"),
        "{two_lines}"
    );
    assert_eq!(fragment("p $a\n\nb$ q"), "<p>p $a</p>\n<p>b$ q</p>\n");
    assert_eq!(
        fragment("p $x $ q $5 and $10"),
        "<p>p $x $ q $5 and $10</p>\n"
    );
    let off = Settings {
        glossary: Some(false),
        math: Some(false),
        ..Settings::default()
    };
    assert_eq!(fragment_with("p $x$ q", &off), "<p>p $x$ q</p>\n");
}

/// `[^label]` is a footnote reference only when the note is defined and
/// no `(` follows; otherwise it is text, or a link.
///
/// # Panics
///
/// When `[^nope]` becomes a reference, or `[^1](/u)` is not a link to
/// `/u`.
#[test]
fn a_footnote_reference_needs_its_note() {
    let html = fragment("[^nope] and [^1](/u)\n\n[^1]: note\n");
    assert!(
        html.starts_with("<p>[^nope] and <a href=\"/u\">^1</a></p>"),
        "{html}"
    );
}

/// A link destination may nest 32 parentheses, not more.
///
/// # Panics
///
/// When the destination with 32 nested parentheses is not a link, or the
/// one with 33 is.
#[test]
fn a_destination_nests_32_parentheses() {
    let nested = |depth: usize| format!("[a]({}{})", "(".repeat(depth), ")".repeat(depth));
    let deepest = fragment(&nested(32));
    assert!(deepest.starts_with("<p><a href=\"(("), "{deepest}");
    let too_deep = fragment(&nested(33));
    assert!(!too_deep.contains("<a "), "{too_deep}");
}

/// A link label holds at most 999 characters, however many bytes they
/// take, and no unescaped `[`.
///
/// # Panics
///
/// When a label of 999 two-byte characters does not find its definition,
/// one of 1000 characters (one byte or two each) does or its definition is
/// not left as text, or a label with an unescaped `[` is read as one.
#[test]
fn a_link_label_holds_999_characters() {
    let label = |ch: &str, count: usize| ch.repeat(count);
    for (inside, found) in [
        (label("é", 999), true),
        (label("a", 1000), false),
        (label("é", 1000), false),
    ] {
        let html = fragment(&format!("[x][{inside}]\n\n[{inside}]: /there\n"));
        assert_eq!(
            html.contains("<a href=\"/there\">x</a>"),
            found,
            "{}",
            inside.len()
        );
        assert_eq!(
            html.contains("]: /there</p>"),
            !found,
            "the definition is text"
        );
    }
    let bracket = fragment("[x][a[b]\n\n[a[b]: /one\n");
    assert!(!bracket.contains("<a "), "{bracket}");
}

/// Emphasis nests 64 deep, not deeper: the marks around the 64 innermost
/// stay text.
///
/// # Panics
///
/// When 64 nested pairs are not 64 emphases, or 65 make more than 64 or
/// lose the outer marks.
#[test]
fn emphasis_nests_64_deep() {
    let nested = |depth: usize| format!("{}b{}", "*a ".repeat(depth), " c*".repeat(depth));
    let deepest = fragment(&nested(64));
    assert_eq!(deepest.matches("<em>").count(), 64);
    let too_deep = fragment(&nested(65));
    assert_eq!(too_deep.matches("<em>").count(), 64);
    assert!(too_deep.starts_with("<p>*a <em>"), "the outer marks stay");
    assert!(too_deep.ends_with("</em> c*</p>\n"), "the outer marks stay");
}

/// A full reference whose label holds an unescaped `[` is no reference:
/// the bracket before it is a shortcut reference of its own.
///
/// # Panics
///
/// When `[x][a[b]` does not link `x` by its own definition and keep
/// `[a[b]` as text.
#[test]
fn a_label_holds_no_bracket() {
    assert_eq!(
        fragment("[x][a[b]\n\n[x]: /x\n"),
        "<p><a href=\"/x\">x</a>[a[b]</p>\n"
    );
}

/// An image's `{...}` sets its size, class and id; with a piece it does
/// not know, or a name that is not one, the braces stay text.
///
/// # Panics
///
/// When the figure does not get the class and id, its width and height
/// are not in the page's style, a size with a `;` is kept, or one of the
/// unreadable attribute lists is not left as text.
#[test]
fn an_images_attributes_set_its_size_class_and_id() {
    let sized = page("# T\n\n![a](p.png){width=50% height=\"20\" .wide #map}\n");
    assert!(
        sized.contains("<figure class=\"fig wide\" id=\"map\">"),
        "{sized}"
    );
    assert!(
        sized.contains("#map img{width:calc((100% - 22px)*0.5000 + 22px)}"),
        "the width"
    );
    assert!(
        sized.contains("#map img{height:20px;width:auto}"),
        "the height"
    );
    let unsizable = page("# T\n\nx ![d](p.png){width=5;x} y\n");
    assert!(!unsizable.contains("{width:5"), "a size with ; is not kept");
    for (attributes, kept) in [
        ("{#a.b}", "{#a.b}"),
        ("{.x y}", "{.x y}"),
        ("{ }", "{ }"),
        ("{.a!b}", "{.a!b}"),
    ] {
        let html = fragment(&format!("x ![c](p.png){attributes} y"));
        assert!(
            html.contains(&format!("/>{kept} y")),
            "{attributes}: {html}"
        );
    }
}
