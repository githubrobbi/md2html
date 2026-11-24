// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Tests of the filter: what of the author's HTML a page keeps.

use super::lists::ELEMENTS;
use super::tag::SAFE_TAGS;
use super::{Attr, Reader, Verdict, attribute, defined, head, rich_block, rich_tag, safe_tag};
use crate::model::Undefined;

/// The markup the page keeps of an author's tag, or None.
fn tag_html(tag: &str) -> Option<String> {
    rich_tag(tag).map(|kept| kept.html)
}

/// The markup the page keeps of a block of the author's HTML.
fn block_html(text: &str) -> String {
    rich_block(text).html
}

/// No event handler, script or script URL in a page's HTML survives the
/// sanitizer.
///
/// # Panics
///
/// Panics when any of these is left in the cleaned block: an `on...`
/// handler, a `<script>` in either case or what it ran, a `javascript:`
/// address plain or written with a character reference (or behind a
/// prefix the author declared, `x:href`), an `<iframe>`,
/// `<object>`, `<form>` or `<input>`, a `url(` in a style, a `data:`
/// address that is not an image, or an `id` and `name` that would shadow
/// the page's own objects. Panics too when the harmless parts are lost: the
/// paragraph, the image with its `alt`, `rel="noopener noreferrer"` on a
/// link that opens a new window, `<details open>`, and a `data:` image.
#[test]
fn handlers_scripts_and_script_urls_never_pass() {
    let dirty = "<p onclick=\"x()\">a</p><script>alert(1)</script><img src=x onerror=alert(1) alt=y>\
<a href=\"javascript:alert(1)\" target=_blank>b</a><a href=\"jav&#x61;script:alert(1)\">c</a>\
<iframe src=//e></iframe><svg onload=alert(1)><a xlink:href=\"javascript:alert(1)\">d</a></svg>\
<svg xmlns:x=\"http://www.w3.org/1999/xlink\"><a x:href=\"javascript:alert(3)\">h</a></svg>\
<div style=\"background:url(javascript:alert(1))\">e</div><details open ontoggle=alert(1)>f</details>\
<object data=x>g</object><form action=//e><input name=q></form><SCRIPT>alert(2)</SCRIPT>\
<img src=\"data:text/html,x\"><img src=\"data:image/png;base64,AA==\" id=idx name=getElementById>";
    let clean = block_html(dirty);
    for bad in [
        "onclick",
        "onerror",
        "onload",
        "ontoggle",
        "<script",
        "<SCRIPT",
        "alert(2)",
        "javascript",
        "jav&",
        "<iframe",
        "<object",
        "<form",
        "<input",
        "url(",
        "xmlns",
        "x:href",
        "data:text",
        "id=\"idx\"",
        "name=",
    ] {
        assert!(!clean.contains(bad), "{bad} in {clean}");
    }
    for good in [
        "<p>a</p>",
        "<img src=\"x\" alt=\"y\">",
        "rel=\"noopener noreferrer\"",
        "<details open>",
        "<img src=\"data:image/png;base64,AA==\">",
    ] {
        assert!(clean.contains(good), "{good} missing from {clean}");
    }
}

/// Harmless rich elements, video and SVG among them, pass with their
/// attributes quoted.
///
/// # Panics
///
/// Panics when `<video>` loses `src`, `controls` or `width`, when an SVG
/// `<circle/>` loses its attributes or its closing slash, or when
/// `<marquee>`, which is not allowed, is kept as a tag.
#[test]
fn rich_html_stays_rich() {
    assert_eq!(
        tag_html("<video src=\"clip.mp4\" controls width=640>").as_deref(),
        Some("<video src=\"clip.mp4\" controls width=\"640\">")
    );
    assert_eq!(
        tag_html("<circle cx=1 cy=1 r=1/>").as_deref(),
        Some("<circle cx=\"1\" cy=\"1\" r=\"1\"/>")
    );
    assert_eq!(tag_html("<marquee>"), None);
}

/// A file for the page's head keeps descriptions, stylesheets and styles,
/// nothing that runs or redirects.
///
/// # Panics
///
/// Panics when the output differs at all from the plain `<meta>`, the
/// stylesheet link and the style: a refresh `<meta>`, a script, a preload
/// of a script or a `<base>` that moves every address is kept, or a
/// harmless tag is lost.
#[test]
fn a_head_file_keeps_only_harmless_tags() {
    let clean = head(
        "<meta name=\"x\" content=\"y\"><meta http-equiv=refresh content=\"0;url=//e\"><script src=//e></script>\
<link rel=stylesheet href=a.css><link rel=preload as=script href=e.js><style>p { color: red }</style><base href=//e>",
    );
    assert_eq!(
        clean.to_string(),
        "<meta name=\"x\" content=\"y\"><link rel=\"stylesheet\" href=\"a.css\"><style>p { color: red }</style>"
    );
}

/// What [`attribute`] makes of `name=value` (or of a bare `name`) on
/// an `element`.
fn verdict(element: &str, name: &str, value: Option<&str>) -> Verdict {
    attribute(element, &Attr {
        name: name.to_owned(),
        value,
    })
}

/// Whether `name="value"` on an `element` is left out of the page, as
/// able to run or fetch something or as undefined there.
fn dropped(element: &str, name: &str, value: &str) -> bool {
    !matches!(
        verdict(element, name, Some(value)),
        Verdict::Value(_) | Verdict::Bare
    )
}

/// An attribute passes only with a well-formed name that is not on the
/// refused list, and on an HTML element only where HTML defines it.
///
/// # Panics
///
/// Panics when a name of letters, digits, `-`, `_`, `.` or `:` that does
/// not start with a digit or a sign, such as `data-x` or `xml:lang`, is
/// dropped from a drawing's element with or without a value, or a defined
/// one (`data-x`, `aria-label`, `xml:lang`, `dir`) from a `div`; when an
/// undefined one (`a`, `x1`, `a_b`, `a.b`) is not left out of a `div` as
/// undefined; or when an empty or ill-formed name, a name beyond ASCII, an
/// `on` handler, or a refused name (`rel`, `srcdoc`, `formaction`, `is`,
/// `xlink:actuate`, `autofocus`, `popovertarget`), a namespace declaration
/// (`xmlns`, `xmlns:x`), or a prefixed name that is not one of the four
/// kept (`x:href`, `xlink:show`) is kept, or taken for merely undefined,
/// on either.
#[test]
fn attribute_names_pass_by_their_spelling() {
    for name in [
        "a",
        "x1",
        "data-x",
        "aria-label",
        "xml:lang",
        "a_b",
        "a.b",
        "dir",
    ] {
        assert_eq!(
            verdict("svg", name, Some("v")),
            Verdict::Value("v".to_owned()),
            "{name}"
        );
        assert_eq!(verdict("svg", name, None), Verdict::Bare, "{name}");
        let on_div = if matches!(name, "data-x" | "aria-label" | "xml:lang" | "dir") {
            Verdict::Bare
        } else {
            Verdict::Undefined
        };
        assert_eq!(verdict("div", name, None), on_div, "{name}");
    }
    for name in [
        "",
        "1a",
        "-a",
        "_a",
        ":a",
        "a b",
        "a@b",
        "a/b",
        "é",
        "onclick",
        "on",
        "rel",
        "srcdoc",
        "formaction",
        "is",
        "xlink:actuate",
        "autofocus",
        "popovertarget",
        "xmlns",
        "xmlns:x",
        "xmlns:xlink",
        "x:href",
        "y:href",
        "xlink:show",
        "svg:a",
    ] {
        for element in ["div", "svg"] {
            assert_eq!(verdict(element, name, Some("v")), Verdict::Drop, "{name}");
            assert_eq!(verdict(element, name, None), Verdict::Drop, "{name}");
        }
    }
}

/// An attribute's value is decoded before it is checked, and the checks
/// depend on the attribute.
///
/// # Panics
///
/// Panics when `&amp;`, `&lt;` and `&gt;` in a title are not decoded; when
/// `name` is dropped on a link, kept with a value on an image, or not kept
/// bare; when an `id` the page uses itself (`toc`, `md2html-ui`, in any
/// case and with spaces) is kept or one merely like it (`tocs`) is dropped;
/// when a style of plain layout is dropped; or when a style that could
/// fetch or run anything (`url`, `expression`, `behavior`, `-moz-binding`,
/// `@import`, `image-set`, `src`, a script address, and an escape or
/// comment that could hide one) is kept.
#[test]
fn attribute_values_are_decoded_and_checked() {
    assert_eq!(
        verdict("a", "title", Some("a &amp; b &lt;c&gt;")),
        Verdict::Value("a & b <c>".to_owned())
    );
    // `name` only on a link, even where HTML defines it (`details`); a
    // bare one names nothing.
    assert!(!dropped("a", "name", "x"));
    assert!(dropped("img", "name", "getElementById"));
    assert_eq!(verdict("details", "name", Some("x")), Verdict::Drop);
    assert_eq!(verdict("details", "name", None), Verdict::Bare);
    // Ids the page keeps, in any case and with space around.
    assert!(dropped("div", "id", " TOC "));
    assert!(dropped("div", "id", "md2html-ui"));
    assert!(!dropped("div", "id", "tocs"));
    // Styles: layout passes, anything that could fetch or run does not.
    assert!(!dropped("div", "style", "color: red; width: 3em"));
    for style in [
        "background: u r l (x)",
        "BACKGROUND:URL(x)",
        "width: expression(alert(1))",
        "behavior: x",
        "-moz-binding: x",
        "a\\:b",
        "a/**/b",
        "@import 'x'",
        "x: image-set(a)",
        "x: src(a)",
        "x: javascript:1",
        "x: vbscript:1",
    ] {
        assert!(dropped("div", "style", style), "{style}");
    }
}

/// An attribute that holds an address keeps it only when the address is
/// safe there.
///
/// # Panics
///
/// Panics when a `javascript:` or `vbscript:` address, plain or written
/// with a reference, is kept in `href`, `cite`, `longdesc`, `codebase`,
/// `data`, `xlink:href` or one candidate of `srcset`; when a `data:`
/// address is kept where it is not an image on an element that shows one;
/// or when a `usemap` that is not `#name` is kept. Panics too when a web
/// address, a `data:` image on `img`, `video`'s `poster`, SVG `image` or
/// `use`, a `srcset` with an empty candidate, a `#map` or a `width` that
/// only looks like a script address is dropped.
#[test]
fn url_attributes_carry_only_safe_urls() {
    for (element, name, value) in [
        ("a", "href", "javascript:alert(1)"),
        ("a", "href", "jav&#x61;script:x"),
        ("a", "href", "javascript&colon;x"),
        ("a", "href", "java&Tab;script&colon;x"),
        ("a", "href", "data:image/png;base64,AA=="),
        ("q", "cite", "vbscript:x"),
        ("img", "longdesc", "javascript:x"),
        ("object", "codebase", "javascript:x"),
        ("object", "data", "javascript:x"),
        ("img", "src", "data:text/html,x"),
        ("svg", "xlink:href", "data:image/png,x"),
        ("img", "srcset", "a.png 1x, javascript:x 2x"),
        ("img", "usemap", "map"),
    ] {
        assert!(dropped(element, name, value), "{element} {name}={value}");
    }
    for (element, name, value) in [
        ("a", "href", "https://example.com/a?b#c"),
        ("img", "src", "data:image/png;base64,AA=="),
        ("img", "srcset", "data:image/png;base64,AA== 1x, b.png 2x"),
        ("img", "srcset", "a.png 1x, , b.png"),
        ("video", "poster", "data:image/jpeg;base64,AA=="),
        ("image", "href", "data:image/png,x"),
        ("use", "xlink:href", "data:image/svg+xml,x"),
        ("img", "usemap", "#map"),
        ("img", "width", "javascript:x"),
    ] {
        assert!(!dropped(element, name, value), "{element} {name}={value}");
    }
}

/// Every tag that passes is written again in one form: lower case, quoted,
/// each attribute once.
///
/// # Panics
///
/// Panics when a tag's name or attribute names keep their case, when values
/// are not quoted (a `"` inside escaped), when a repeated or invalid
/// attribute is kept, when an end tag keeps attributes or space, when `<br
/// />` and `<img .../>` lose their closing slash, when a link that can open
/// a window lacks `rel="noopener noreferrer"` or keeps `rel=opener`, when
/// `target` passes on `area` or on a `span`, which HTML does not give it,
/// when an undefined attribute is kept beside defined ones, or when a
/// script, a tag that never closes, or text without `<` gives a tag.
#[test]
fn tags_are_written_again_in_one_form() {
    for (tag, clean) in [
        ("<DIV Class=a  ID=b>", Some("<div class=\"a\" id=\"b\">")),
        ("</DIV >", Some("</div>")),
        ("</div x=1>", Some("</div>")),
        ("<br/>", Some("<br/>")),
        ("<br />", Some("<br/>")),
        ("<img src=a/>", Some("<img src=\"a\"/>")),
        ("<p title='\"x\"'>", Some("<p title=\"&quot;x&quot;\">")),
        (
            "<p hidden title=x hidden=y>",
            Some("<p hidden title=\"x\">"),
        ),
        ("<p x\u{a0}>", Some("<p>")),
        (
            "<a target=_blank href=x>",
            Some("<a target=\"_blank\" href=\"x\" rel=\"noopener noreferrer\">"),
        ),
        (
            "<a rel=opener target=x>",
            Some("<a target=\"x\" rel=\"noopener noreferrer\">"),
        ),
        ("<a target>", Some("<a target rel=\"noopener noreferrer\">")),
        ("<area target=x>", None),
        ("<span target=x>", Some("<span>")),
        (
            "<p x=1 align=center data-a=b>",
            Some("<p align=\"center\" data-a=\"b\">"),
        ),
        (
            "<a onclick=x target=y>",
            Some("<a target=\"y\" rel=\"noopener noreferrer\">"),
        ),
        ("<a onclick=x>", Some("<a>")),
        ("<script>", None),
        ("<p", None),
        ("p>", None),
        ("</script>", None),
    ] {
        assert_eq!(tag_html(tag).as_deref(), clean, "{tag}");
    }
}

/// A block of HTML loses comments and dangerous elements with all they
/// hold, and keeps the rest.
///
/// # Panics
///
/// Panics when a comment, a processing instruction, a doctype or a CDATA
/// section is kept; when a lone `<` is not escaped; when a `script`,
/// `style`, `noscript`, `template`, `iframe` or `object` keeps any of its
/// content, whatever the case of its end tag; when an element that is not
/// allowed but harmless, `marquee`, `form` or a second `main`, loses its
/// text or its allowed children; when a stray `</script>` is kept; or when
/// `é` is changed.
#[test]
fn a_block_loses_comments_and_dangerous_elements_whole() {
    for (dirty, clean) in [
        ("a<!-- <b>x</b> -->b", "ab"),
        ("a<?php x ?>b<!DOCTYPE html>c<![CDATA[<b>]]>d", "abcd"),
        ("1 < 2 <3", "1 &lt; 2 &lt;3"),
        ("a<script>x<p>y</p></script>b", "ab"),
        ("a<STYLE>p{}</style>b", "ab"),
        ("a<noscript><b>x</b></NoScript >b", "ab"),
        ("a<template>x", "a"),
        ("a<iframe src=x></iframe>b<object>x</object>c", "abc"),
        ("<marquee>moving</marquee>", "moving"),
        ("<main><p>x</p></MAIN>", "<p>x</p>"),
        ("<form action=x><b>bold</b></form>", "<b>bold</b>"),
        ("x</script>y", "xy"),
        ("<em>\u{e9}</em>", "<em>\u{e9}</em>"),
    ] {
        assert_eq!(block_html(dirty), clean, "{dirty}");
    }
}

/// A head file keeps `meta`, `link` and `style` only where they cannot run,
/// redirect or fetch scripts.
///
/// # Panics
///
/// Panics when a `<meta>` with `charset`, `name` or `property` loses what
/// it says or keeps an attribute beyond them (`data-x`, an empty
/// `http-equiv`); when one with a non-empty `http-equiv` or a handler is
/// kept; when a link with a stylesheet, icon or canonical `rel` is
/// dropped or keeps its handler; when one whose address is a script
/// address, whose `rel` is missing or empty, or with a relation that
/// loads a script (`preload`, `modulepreload`) is kept; when anything
/// inside `script`, `noscript`, `title` or a comment, or a `<base>` or
/// `<body>`, is kept; when a `<` inside a style is not escaped as `\3c `;
/// when a style with `expression` or `-moz-binding` in any case is kept;
/// or when a style that never closes is not closed.
#[test]
fn a_head_file_drops_what_could_run_or_redirect() {
    for (dirty, clean) in [
        ("<meta charset=utf-8>", "<meta charset=\"utf-8\">"),
        ("<meta http-equiv=\"\" name=a>", "<meta name=\"a\">"),
        ("<meta HTTP-EQUIV=Refresh content=0>", ""),
        ("<meta name=a onload=x>", ""),
        (
            "<meta property=og:title content=\"A &amp; B\" data-x=1>",
            "<meta property=\"og:title\" content=\"A &amp; B\">",
        ),
        (
            "<link rel=\"icon stylesheet\" href=a.ico sizes=16x16 onload=x crossorigin>",
            "<link rel=\"icon stylesheet\" href=\"a.ico\" sizes=\"16x16\" crossorigin>",
        ),
        (
            "<link rel=Canonical href=https://e.org/>",
            "<link rel=\"Canonical\" href=\"https://e.org/\">",
        ),
        ("<link rel=stylesheet href=javascript:x>", ""),
        ("<link rel=stylesheet>", "<link rel=\"stylesheet\">"),
        ("<link href=a.css>", ""),
        ("<link rel href=a.css>", ""),
        ("<link rel=\"\" href=a.css>", ""),
        ("<link rel=\"icon preload\" href=a.js>", ""),
        ("<link rel=modulepreload href=a.js>", ""),
        (
            "<script><meta name=a></script><meta name=b>",
            "<meta name=\"b\">",
        ),
        ("<noscript><meta name=a></noscript>", ""),
        ("<title>T</title><base href=//e><body>x", ""),
        ("<!-- <meta name=a> --><meta name=b>", "<meta name=\"b\">"),
        (
            "<style>a::after { content: \"<\" }</STYLE><meta name=b>",
            "<style>a::after { content: \"\\3c \" }</style><meta name=\"b\">",
        ),
        (
            "<style>p { width: EXPRESSION(1) }</style><meta name=b>",
            "<meta name=\"b\">",
        ),
        ("<style>p { -moz-binding: x }</style>", ""),
        ("<style>p { -MOZ-Binding: x }</style>", ""),
        ("<style>p { color: red }", "<style>p { color: red }</style>"),
    ] {
        assert_eq!(head(dirty).to_string(), clean, "{dirty}");
    }
}

/// Without `--html`, a harmless tag keeps only its allowed attributes,
/// each value read whole, decoded and quoted again.
///
/// # Panics
///
/// When an allowed attribute loses its value, unquoted, in single or
/// double quotes, with spaces around its `=` or inside its quotes; when a
/// reference in a value is not decoded and escaped again; when a name keeps
/// its case or a handler is kept; when a bare `open` gains a value; when
/// `<br/>` is not written `<br>`, or a `<div/>` loses the slash that makes
/// it an empty element; or when a tag off the list is kept.
#[test]
fn a_safe_tag_keeps_its_allowed_attributes_read_whole() {
    for (tag, clean) in [
        ("<ol start=3>", Some("<ol start=\"3\">")),
        (
            "<td colspan=2 rowspan=3>",
            Some("<td colspan=\"2\" rowspan=\"3\">"),
        ),
        (
            "<td colspan = '2' rowspan=\"3\" >",
            Some("<td colspan=\"2\" rowspan=\"3\">"),
        ),
        (
            "<abbr title=\"a b &amp; c\">",
            Some("<abbr title=\"a b &amp; c\">"),
        ),
        (
            "<abbr title='say \"hi\"'>",
            Some("<abbr title=\"say &quot;hi&quot;\">"),
        ),
        ("<ABBR TITLE=x onclick=y>", Some("<abbr title=\"x\">")),
        ("<details open>", Some("<details open>")),
        ("<br/>", Some("<br>")),
        ("<div/>", Some("<div/>")),
        ("</DIV>", Some("</div>")),
        ("<script>", None),
    ] {
        assert_eq!(safe_tag(tag).as_deref(), clean, "{tag}");
    }
}

/// The obsolete list elements become lists, so their items stay in one.
///
/// # Panics
///
/// When `<dir>` or `<menu>`, opening or closing, in any case, is not
/// written as the matching `<ul>` tag, or when a block of them keeps an
/// `<li>` outside a `<ul>`.
#[test]
fn obsolete_lists_stay_lists() {
    assert_eq!(tag_html("<dir>").as_deref(), Some("<ul>"));
    assert_eq!(tag_html("</DIR>").as_deref(), Some("</ul>"));
    assert_eq!(tag_html("<Menu>").as_deref(), Some("<ul>"));
    assert_eq!(tag_html("</menu>").as_deref(), Some("</ul>"));
    assert_eq!(
        block_html("<dir>\n<li>one</li>\n<li>two</li>\n</dir>"),
        "<ul>\n<li>one</li>\n<li>two</li>\n</ul>"
    );
}

/// An attribute HTML does not define, as written.
fn undefined(element: &'static str, attr: &str) -> Undefined {
    Undefined {
        element,
        attr: attr.to_owned(),
    }
}

/// What the filter leaves out of a tag as undefined is reported, each
/// attribute once, in order, on the element the tag becomes; what it
/// drops as able to run or fetch something is not.
///
/// # Panics
///
/// When a link keeps `b`, loses `href` or `data-k`, or reports anything
/// but `b` once (not the handler, not `popover`); when a block's undefined
/// attributes are not reported on `p` and on the `ul` a `dir` becomes, or a
/// drawing's are; or when an end tag reports any.
#[test]
fn undefined_attributes_are_reported() {
    let kept = rich_tag("<A HREF=x B=c onclick=y popover b=d Data-k=v>").expect("a link stays");
    assert_eq!(kept.html, "<a href=\"x\" data-k=\"v\">");
    assert_eq!(kept.undefined, [undefined("a", "b")]);
    let block = rich_block("<p x=1>a</p><dir y>b</dir><circle r=1 q=2/>");
    assert_eq!(block.html, "<p>a</p><ul>b</ul><circle r=\"1\" q=\"2\"/>");
    assert_eq!(block.undefined, [undefined("p", "x"), undefined("ul", "y")]);
    assert_eq!(rich_tag("</p>").map(|end| end.undefined), Some(Vec::new()));
}

/// Every HTML element the filter lets stand is judged by the table of
/// the attributes HTML defines; only a drawing's and a formula's are not.
///
/// # Panics
///
/// When an element of the filter's list that is not SVG's or `MathML`'s
/// takes an attribute HTML has never had, or one of theirs is judged.
#[test]
fn every_html_element_is_judged() {
    const FOREIGN: [&str; 48] = [
        "svg",
        "g",
        "path",
        "rect",
        "circle",
        "ellipse",
        "line",
        "polyline",
        "polygon",
        "text",
        "tspan",
        "textpath",
        "defs",
        "use",
        "symbol",
        "marker",
        "lineargradient",
        "radialgradient",
        "stop",
        "clippath",
        "mask",
        "pattern",
        "title",
        "desc",
        "image",
        "math",
        "mi",
        "mo",
        "mn",
        "ms",
        "mtext",
        "mrow",
        "msup",
        "msub",
        "msubsup",
        "mfrac",
        "msqrt",
        "mroot",
        "mtable",
        "mtr",
        "mtd",
        "mover",
        "munder",
        "munderover",
        "mspace",
        "mstyle",
        "semantics",
        "annotation",
    ];
    for element in ELEMENTS {
        assert_eq!(
            defined(Reader::Browser, element, "never-an-attribute"),
            FOREIGN.contains(element),
            "{element}"
        );
    }
}

/// Without `--html`, a tag keeps only attributes HTML defines on it.
///
/// # Panics
///
/// When an element of the short list is not judged by the table, or one
/// of the attributes it keeps is not defined there.
#[test]
fn the_safe_tags_keep_only_defined_attributes() {
    for (element, allowed) in SAFE_TAGS {
        assert!(
            !defined(Reader::Browser, element, "never-an-attribute"),
            "{element}"
        );
        for attr in *allowed {
            assert!(
                defined(Reader::Browser, element, attr),
                "{attr} on {element}"
            );
        }
    }
}
