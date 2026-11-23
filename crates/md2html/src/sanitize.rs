// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The author's own HTML, with `--html`: rich (images, links, video,
//! SVG, layout), never executable. A page md2html writes runs only
//! md2html's own code, whatever the Markdown holds.
//!
//! Elements pass by an allowlist; scripts, frames, plug-ins, forms and
//! the elements that change how the page loads are dropped, the first
//! kinds with everything inside them. Attributes pass unless they are
//! event handlers or otherwise able to run or fetch code; every URL is
//! checked as a Markdown link's is; a style that could fetch or run
//! anything is dropped; ids that the page's own script uses are not
//! taken. The same filter cleans the `head` file an author adds.

mod lists;
mod markup;

use self::lists::{
    DENIED, DROP_WITH_CONTENT, ELEMENTS, HEAD_ATTRS, LINK_RELS, RESERVED_IDS, STYLE_DENIED,
    URL_ATTRS,
};
use self::markup::{Attr, Markup, Opening, Piece, attributes, before_close, opens};
use crate::inline::decode_entities;
use crate::scan::Scan;
use crate::text::{esc, put, safe_url};

/// What becomes of one attribute of an author's tag.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Verdict {
    /// It stays, with this value (entities decoded, not yet escaped).
    Value(String),
    /// It stays, bare, as it was written.
    Bare,
    /// It is dropped.
    Drop,
}

/// An author's tag as it may stand in the page, or None to drop it.
pub(crate) fn rich_tag(tag: &str) -> Option<String> {
    let body = tag.strip_prefix('<')?.strip_suffix('>')?;
    let (closing, open) = body
        .strip_prefix('/')
        .map_or((false, body), |after| (true, after));
    let trimmed = open.trim_end();
    let self_closing = trimmed.ends_with('/');
    let inner = trimmed.strip_suffix('/').unwrap_or(open);
    let mut scan = Scan::new(inner);
    let written = scan
        .take_while(|ch| !ch.is_whitespace() && ch != '/')
        .to_ascii_lowercase();
    let name = list_of(&written).unwrap_or(&written);
    if !ELEMENTS.contains(&name) {
        return None;
    }
    if closing {
        return Some(format!("</{name}>"));
    }
    let mut out = format!("<{name}");
    let mut target = false;
    for attr in attributes(scan.rest()) {
        match attribute(name, &attr) {
            Verdict::Value(value) => put!(out, " {}=\"{}\"", attr.name, esc(&value)),
            Verdict::Bare => put!(out, " {}", attr.name),
            Verdict::Drop => continue,
        }
        target |= attr.name == "target";
    }
    if target && name == "a" {
        out.push_str(" rel=\"noopener noreferrer\"");
    }
    out.push_str(if self_closing { "/>" } else { ">" });
    Some(out)
}

/// The list element that an obsolete list element stands for: `<dir>`
/// (HTML 4) and `<menu>` hold `<li>` items like `<ul>`, and dropping them
/// would leave their items outside any list.
fn list_of(name: &str) -> Option<&'static str> {
    matches!(name, "dir" | "menu").then_some("ul")
}

/// What becomes of `attr` on an `element`.
fn attribute(element: &str, attr: &Attr<'_>) -> Verdict {
    if !allowed_name(&attr.name) {
        return Verdict::Drop;
    }
    let Some(written) = attr.value else {
        return Verdict::Bare;
    };
    let value = decode_entities(written);
    if allowed_value(element, &attr.name, &value) {
        Verdict::Value(value)
    } else {
        Verdict::Drop
    }
}

/// Whether an attribute of this name may stand at all: a plain
/// lowercase name, no event handler, nothing denied, no `rel` (the
/// filter writes that itself).
fn allowed_name(name: &str) -> bool {
    let plain = name.starts_with(|ch: char| ch.is_ascii_lowercase())
        && name.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | ':' | '.')
        });
    plain && !name.starts_with("on") && !DENIED.contains(&name) && name != "rel"
}

/// Whether an attribute `key` with this (decoded) value may stand on an
/// `element`: `name` only on a link, no id the page keeps, a style that
/// is only layout, a URL that is safe.
fn allowed_value(element: &str, key: &str, value: &str) -> bool {
    match key {
        "name" => element == "a",
        "id" => !RESERVED_IDS.contains(&value.trim().to_ascii_lowercase().as_str()),
        "style" => safe_style(value),
        url if URL_ATTRS.contains(&url) => safe_url_attr(element, url, value),
        _ => true,
    }
}

/// Whether the URL attribute `key` on an `element` may carry `value`.
/// An image's pixels may come from a `data:` URL; a `srcset` is checked
/// candidate by candidate; a `usemap` points into the page only.
fn safe_url_attr(element: &str, key: &str, value: &str) -> bool {
    let image = matches!(key, "src" | "srcset" | "poster")
        || matches!(element, "image" | "use") && key.ends_with("href");
    match key {
        "srcset" => value.split(',').all(|candidate| {
            candidate
                .split_whitespace()
                .next()
                .is_none_or(|url| safe_url(url, image))
        }),
        "usemap" => value.starts_with('#'),
        _ => safe_url(value, image),
    }
}

/// Whether an inline style is only layout: nothing it holds could fetch
/// or run anything.
fn safe_style(style: &str) -> bool {
    let squeezed = style
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    !STYLE_DENIED.iter().any(|bad| squeezed.contains(bad))
}

/// A block of the author's HTML, cleaned: comments and dropped elements
/// gone (the dangerous kinds with their contents), the rest as
/// [`rich_tag`] allows; a `<` that starts no tag is written as text.
pub(crate) fn rich_block(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut markup = Markup::new(text);
    while let Some(piece) = markup.next_piece() {
        match piece {
            Piece::Text(words) => out.push_str(words),
            Piece::Stray => out.push_str("&lt;"),
            Piece::Comment => {}
            Piece::Tag(tag) => match opens(tag) {
                Some(opening) if DROP_WITH_CONTENT.contains(&opening.name.as_str()) => {
                    markup.skip_element(&opening.name);
                }
                Some(_) | None => {
                    if let Some(clean) = rich_tag(tag) {
                        out.push_str(&clean);
                    }
                }
            },
        }
    }
    out
}

/// An author's `head` file, cleaned: `meta` (but not `http-equiv`),
/// `link` (stylesheets, icons, alternates and the like, never one that
/// loads a script) and `style` pass; everything else, scripts first, is
/// dropped.
pub(crate) fn head(text: &str) -> String {
    let mut out = String::new();
    let mut markup = Markup::new(text);
    while let Some(piece) = markup.next_piece() {
        let Piece::Tag(tag) = piece else {
            continue;
        };
        let Some(Opening { name, after_name }) = opens(tag) else {
            continue;
        };
        let attrs = attributes(after_name);
        match name.as_str() {
            "meta" if harmless_meta(&attrs) => out.push_str(&clean_void(&name, &attrs)),
            "link" if harmless_link(&attrs) => out.push_str(&clean_void(&name, &attrs)),
            "style" => {
                out.push_str(&clean_style(before_close(markup.rest(), "style")));
                markup.skip_element("style");
            }
            other if DROP_WITH_CONTENT.contains(&other) => markup.skip_element(other),
            _ => {}
        }
    }
    out
}

/// The value of the attribute `name`, decoded and lowercased; None when
/// it is missing or bare.
fn lowered(attrs: &[Attr<'_>], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|attr| attr.name == name)
        .and_then(|attr| attr.value)
        .map(|value| decode_entities(value).to_ascii_lowercase())
}

/// Whether a `meta` only describes the page: no `http-equiv` with a
/// value, no event handler.
fn harmless_meta(attrs: &[Attr<'_>]) -> bool {
    lowered(attrs, "http-equiv").is_none_or(|equiv| equiv.is_empty())
        && !attrs.iter().any(|attr| attr.name.starts_with("on"))
}

/// Whether a `link` names only relations that load no script, and a
/// safe address.
fn harmless_link(attrs: &[Attr<'_>]) -> bool {
    lowered(attrs, "rel").is_some_and(|rel| {
        !rel.is_empty()
            && rel
                .split_whitespace()
                .all(|relation| LINK_RELS.contains(&relation))
    }) && lowered(attrs, "href").is_none_or(|href| safe_url(&href, false))
}

/// A `style` element of the `head` file, or nothing when its CSS could
/// run something; a `<` in it cannot end the element.
fn clean_style(css: &str) -> String {
    let lowered = css.to_ascii_lowercase();
    if lowered.contains("expression") || lowered.contains("-moz-binding") {
        return String::new();
    }
    format!("<style>{}</style>", css.replace('<', "\\3c "))
}

/// A `meta` or `link` written again with its harmless attributes only.
fn clean_void(name: &str, attrs: &[Attr<'_>]) -> String {
    let mut out = format!("<{name}");
    for attr in attrs
        .iter()
        .filter(|attr| HEAD_ATTRS.contains(&attr.name.as_str()))
    {
        put!(out, " {}", attr.name);
        if let Some(value) = attr.value {
            put!(out, "=\"{}\"", esc(&decode_entities(value)));
        }
    }
    out.push('>');
    out
}

#[cfg(test)]
mod tests {
    use super::{Attr, Verdict, attribute, head, rich_block, rich_tag};

    /// No event handler, script or script URL in a page's HTML survives the
    /// sanitizer.
    ///
    /// # Panics
    ///
    /// Panics when any of these is left in the cleaned block: an `on...`
    /// handler, a `<script>` in either case or what it ran, a `javascript:`
    /// address plain or written with a character reference, an `<iframe>`,
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
<div style=\"background:url(javascript:alert(1))\">e</div><details open ontoggle=alert(1)>f</details>\
<object data=x>g</object><form action=//e><input name=q></form><SCRIPT>alert(2)</SCRIPT>\
<img src=\"data:text/html,x\"><img src=\"data:image/png;base64,AA==\" id=idx name=getElementById>";
        let clean = rich_block(dirty);
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
            rich_tag("<video src=\"clip.mp4\" controls width=640>").as_deref(),
            Some("<video src=\"clip.mp4\" controls width=\"640\">")
        );
        assert_eq!(
            rich_tag("<circle cx=1 cy=1 r=1/>").as_deref(),
            Some("<circle cx=\"1\" cy=\"1\" r=\"1\"/>")
        );
        assert_eq!(rich_tag("<marquee>"), None);
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
            clean,
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

    /// Whether `name="value"` on an `element` is dropped.
    fn dropped(element: &str, name: &str, value: &str) -> bool {
        verdict(element, name, Some(value)) == Verdict::Drop
    }

    /// An attribute passes only with a well-formed name that is not on the
    /// refused list.
    ///
    /// # Panics
    ///
    /// Panics when a name of letters, digits, `-`, `_`, `.` or `:` that does
    /// not start with a digit or a sign, such as `data-x` or `xml:lang`, is
    /// dropped with or without a value; or when an empty or ill-formed name, a
    /// name beyond ASCII, an `on` handler, or a refused name (`rel`, `srcdoc`,
    /// `formaction`, `is`, `xlink:actuate`, `autofocus`, `popovertarget`) is
    /// kept.
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
                verdict("div", name, Some("v")),
                Verdict::Value("v".to_owned()),
                "{name}"
            );
            assert_eq!(verdict("div", name, None), Verdict::Bare, "{name}");
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
        ] {
            assert_eq!(verdict("div", name, Some("v")), Verdict::Drop, "{name}");
            assert_eq!(verdict("div", name, None), Verdict::Drop, "{name}");
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
        // `name` only on a link; a bare one names nothing.
        assert!(!dropped("a", "name", "x"));
        assert!(dropped("img", "name", "getElementById"));
        assert_eq!(verdict("img", "name", None), Verdict::Bare);
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
    /// `target` passes on `area`, or when a script, a tag that never closes, or
    /// text without `<` gives a tag.
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
            ("<span target=x>", Some("<span target=\"x\">")),
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
            assert_eq!(rich_tag(tag).as_deref(), clean, "{tag}");
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
    /// allowed but harmless, `marquee` or `form`, loses its text or its allowed
    /// children; when a stray `</script>` is kept; or when `é` is changed.
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
            ("<form action=x><b>bold</b></form>", "<b>bold</b>"),
            ("x</script>y", "xy"),
            ("<em>\u{e9}</em>", "<em>\u{e9}</em>"),
        ] {
            assert_eq!(rich_block(dirty), clean, "{dirty}");
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
            assert_eq!(head(dirty), clean, "{dirty}");
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
        assert_eq!(rich_tag("<dir>").as_deref(), Some("<ul>"));
        assert_eq!(rich_tag("</DIR>").as_deref(), Some("</ul>"));
        assert_eq!(rich_tag("<Menu>").as_deref(), Some("<ul>"));
        assert_eq!(rich_tag("</menu>").as_deref(), Some("</ul>"));
        assert_eq!(
            rich_block("<dir>\n<li>one</li>\n<li>two</li>\n</dir>"),
            "<ul>\n<li>one</li>\n<li>two</li>\n</ul>"
        );
    }
}
