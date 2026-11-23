// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::allow_attributes,
    clippy::indexing_slicing,
    clippy::min_ident_chars,
    clippy::mixed_read_write_in_expression,
    clippy::redundant_type_annotations,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::string_slice,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]

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

use crate::inline::{decode_entities, html_tag};
use crate::text::{esc, safe_url};

/// Elements that may stand in the article.
const ELEMENTS: &[&str] = &[
    // text and structure
    "a",
    "abbr",
    "address",
    "article",
    "aside",
    "b",
    "bdi",
    "bdo",
    "blockquote",
    "br",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "details",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "ins",
    "kbd",
    "li",
    "main",
    "mark",
    "nav",
    "ol",
    "p",
    "pre",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "section",
    "small",
    "span",
    "strike",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "u",
    "ul",
    "var",
    "wbr",
    // media
    "audio",
    "img",
    "picture",
    "source",
    "track",
    "video",
    // SVG drawing (no script, no foreignObject, no animation that could set a
    // link)
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
    // MathML
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

/// Elements dropped together with everything inside them.
const DROP_WITH_CONTENT: &[&str] = &[
    "script",
    "style",
    "iframe",
    "frame",
    "frameset",
    "object",
    "embed",
    "applet",
    "template",
    "noscript",
    "noembed",
    "noframes",
    "textarea",
    "xmp",
    "plaintext",
    "select",
    "option",
    "foreignobject",
    "head",
];

/// Attributes that never pass, whatever the element.
const DENIED: &[&str] = &[
    "srcdoc",
    "formaction",
    "action",
    "ping",
    "background",
    "lowsrc",
    "dynsrc",
    "is",
    "nonce",
    "http-equiv",
    "popover",
    "popovertarget",
    "popovertargetaction",
    "command",
    "commandfor",
    "autofocus",
    "attributename",
    "xlink:actuate",
    "xml:base",
];

/// Attributes that carry a URL.
const URL_ATTRS: &[&str] = &[
    "href",
    "src",
    "xlink:href",
    "poster",
    "cite",
    "longdesc",
    "codebase",
    "data",
    "srcset",
    "usemap",
];

/// The ids the page's own script and layout use: an author's element
/// must not take one.
const RESERVED_IDS: &[&str] = &[
    "top",
    "main",
    "toc",
    "find",
    "q",
    "hits",
    "hits-n",
    "idx",
    "md2html-ui",
    "bar",
    "lang",
    "langs",
    "prefs",
    "aa",
    "print",
    "size",
    "size-up",
    "size-down",
    "toggle",
    "edge",
    "dock",
    "crumb",
    "open-find",
    "theme",
    "glossary",
    "notes",
    "zoom",
    "save",
    "keep",
    "toast",
    "passage",
];

/// An author's tag as it may stand in the page, or None to drop it.
pub(crate) fn rich_tag(tag: &str) -> Option<String> {
    let inner = tag.strip_prefix('<')?.strip_suffix('>')?;
    let (closing, inner) = inner
        .strip_prefix('/')
        .map_or((false, inner), |rest| (true, rest));
    let self_closing = inner.trim_end().ends_with('/');
    let inner = inner.trim_end().strip_suffix('/').unwrap_or(inner);
    let name_len = inner
        .find(|c: char| c.is_whitespace() || c == '/')
        .unwrap_or(inner.len());
    let name = inner[..name_len].to_ascii_lowercase();
    if !ELEMENTS.contains(&name.as_str()) {
        return None;
    }
    if closing {
        return Some(format!("</{name}>"));
    }
    let mut out = format!("<{name}");
    let mut target_blank = false;
    for (key, value) in attributes(&inner[name_len..]) {
        let Some(clean) = attribute(&name, &key, value.as_deref()) else {
            continue;
        };
        if key == "target" {
            target_blank = true;
        }
        out.push(' ');
        out.push_str(&key);
        if let Some(v) = clean {
            out.push_str("=\"");
            out.push_str(&esc(&v));
            out.push('"');
        }
    }
    if target_blank && name == "a" {
        out.push_str(" rel=\"noopener noreferrer\"");
    }
    out.push_str(if self_closing { "/>" } else { ">" });
    Some(out)
}

/// One attribute as it may stand: Some(value) to keep (None for a bare
/// attribute), or None to drop it.
#[allow(
    clippy::option_option,
    reason = "keep with a value, keep bare, or drop"
)]
fn attribute(element: &str, key: &str, value: Option<&str>) -> Option<Option<String>> {
    let valid = key.starts_with(|c: char| c.is_ascii_lowercase())
        && key.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | ':' | '.')
        });
    if !valid || key.starts_with("on") || DENIED.contains(&key) || key == "rel" {
        return None;
    }
    let Some(raw) = value else {
        return Some(None);
    };
    let value = decode_entities(raw);
    if key == "name" && element != "a" {
        return None;
    }
    if key == "id" && RESERVED_IDS.contains(&value.trim().to_ascii_lowercase().as_str()) {
        return None;
    }
    if key == "style" && !safe_style(&value) {
        return None;
    }
    if URL_ATTRS.contains(&key) {
        let image = matches!(key, "src" | "srcset" | "poster")
            || (element == "image" || element == "use") && key.ends_with("href");
        let ok = if key == "srcset" {
            value
                .split(',')
                .all(|part| safe_url(part.split_whitespace().next().unwrap_or_default(), image))
        } else if key == "usemap" {
            value.starts_with('#')
        } else {
            safe_url(&value, image)
        };
        if !ok {
            return None;
        }
    }
    Some(Some(value))
}

/// Whether an inline style is only layout: nothing it holds could fetch
/// or run anything.
fn safe_style(style: &str) -> bool {
    let squeezed: String = style
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    ![
        "expression",
        "javascript:",
        "vbscript:",
        "url(",
        "@import",
        "behavior",
        "-moz-binding",
        "\\",
        "/*",
        "image-set(",
        "src(",
    ]
    .iter()
    .any(|bad| squeezed.contains(bad))
}

/// A tag's attributes as (name, value); names lowercased.
fn attributes(text: &str) -> Vec<(String, Option<String>)> {
    let mut list: Vec<(String, Option<String>)> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && !matches!(bytes[i], b'=' | b'/' | b'>')
        {
            i += 1;
        }
        if start == i {
            i += 1;
            continue;
        }
        let key = text[start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = None;
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && matches!(bytes[i], b'"' | b'\'') {
                let quote = bytes[i];
                let from = i + 1;
                let end = text[from..]
                    .find(char::from(quote))
                    .map_or(text.len(), |e| from + e);
                value = Some(text[from..end].to_owned());
                i = end + 1;
            } else {
                let from = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = Some(text[from..i].to_owned());
            }
        }
        // The first of a repeated attribute wins, as in the browser.
        if !list.iter().any(|(k, _)| *k == key) {
            list.push((key, value));
        }
    }
    list
}

/// The name of the element a tag opens, lowercased, or None for a
/// closing tag, a comment or a declaration.
fn opens(tag: &str) -> Option<String> {
    let inner = tag.strip_prefix('<')?;
    if inner.starts_with(['/', '!', '?']) {
        return None;
    }
    let len = inner
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .unwrap_or(inner.len());
    Some(inner[..len].to_ascii_lowercase())
}

/// A block of the author's HTML, cleaned: comments and dropped elements
/// gone (the dangerous kinds with their contents), the rest as
/// [`rich_tag`] allows; a `<` that starts no tag is written as text.
pub(crate) fn rich_block(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(len) = html_tag(rest) else {
            out.push_str("&lt;");
            rest = &rest[1..];
            continue;
        };
        let tag = &rest[..len];
        rest = &rest[len..];
        if tag.starts_with("<!") || tag.starts_with("<?") {
            continue;
        }
        if let Some(name) = opens(tag) {
            if DROP_WITH_CONTENT.contains(&name.as_str()) {
                rest = after_close(rest, &name);
                continue;
            }
        }
        if let Some(clean) = rich_tag(tag) {
            out.push_str(&clean);
        }
    }
    out.push_str(rest);
    out
}

/// The text after the end tag of `name` (case aside), or nothing when it
/// never closes: an unclosed script takes the rest with it.
fn after_close<'a>(text: &'a str, name: &str) -> &'a str {
    let lower = text.to_ascii_lowercase();
    let close = format!("</{name}");
    lower
        .find(&close)
        .and_then(|at| text[at..].find('>').map(|end| &text[at + end + 1..]))
        .unwrap_or("")
}

/// An author's `head` file, cleaned: `meta` (but not `http-equiv`),
/// `link` (stylesheets, icons, alternates and the like, never one that
/// loads a script) and `style` pass; everything else, scripts first, is
/// dropped.
pub(crate) fn head(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at..];
        let Some(len) = html_tag(rest) else {
            rest = &rest[1..];
            continue;
        };
        let tag = &rest[..len];
        rest = &rest[len..];
        let Some(name) = opens(tag) else {
            continue;
        };
        let inner = tag.trim_start_matches('<').trim_end_matches('>');
        let attrs = attributes(
            inner
                .find(char::is_whitespace)
                .map_or("", |at| &inner[at..]),
        );
        let get = |k: &str| {
            attrs
                .iter()
                .find(|(key, _)| key == k)
                .and_then(|(_, v)| v.clone())
                .map(|v| decode_entities(&v).to_ascii_lowercase())
                .unwrap_or_default()
        };
        match name.as_str() {
            "meta"
                if get("http-equiv").is_empty()
                    && !attrs.iter().any(|(k, _)| k.starts_with("on")) =>
            {
                out.push_str(&clean_void(&name, &attrs));
            }
            "link" => {
                let rel = get("rel");
                let fine = rel.split_whitespace().all(|r| {
                    matches!(
                        r,
                        "stylesheet"
                            | "icon"
                            | "apple-touch-icon"
                            | "alternate"
                            | "canonical"
                            | "author"
                            | "license"
                            | "preconnect"
                            | "dns-prefetch"
                    )
                }) && !rel.is_empty()
                    && safe_url(&get("href"), false);
                if fine {
                    out.push_str(&clean_void(&name, &attrs));
                }
            }
            "style" => {
                let lower = rest.to_ascii_lowercase();
                let end = lower.find("</style").unwrap_or(rest.len());
                let css = &rest[..end];
                if !css.to_ascii_lowercase().contains("expression") && !css.contains("-moz-binding")
                {
                    out.push_str("<style>");
                    out.push_str(&css.replace('<', "\\3c "));
                    out.push_str("</style>");
                }
                rest = after_close(rest, "style");
            }
            other if DROP_WITH_CONTENT.contains(&other) => rest = after_close(rest, other),
            _ => {}
        }
    }
    out
}

/// A `meta` or `link` written again with its harmless attributes only.
fn clean_void(name: &str, attrs: &[(String, Option<String>)]) -> String {
    let mut out = format!("<{name}");
    for (key, value) in attrs {
        let fine = matches!(
            key.as_str(),
            "name"
                | "content"
                | "property"
                | "charset"
                | "rel"
                | "href"
                | "type"
                | "media"
                | "sizes"
                | "hreflang"
                | "title"
                | "crossorigin"
                | "itemprop"
        );
        if fine {
            out.push(' ');
            out.push_str(key);
            if let Some(v) = value {
                out.push_str("=\"");
                out.push_str(&esc(&decode_entities(v)));
                out.push('"');
            }
        }
    }
    out.push('>');
    out
}

#[cfg(test)]
mod tests {
    use super::{head, rich_block, rich_tag};

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

    #[test]
    #[expect(
        clippy::literal_string_with_formatting_args,
        reason = "a placeholder in the page's own words ({n}), filled in elsewhere"
    )]
    fn a_head_file_keeps_only_harmless_tags() {
        let clean = head(
            "<meta name=\"x\" content=\"y\"><meta http-equiv=refresh content=\"0;url=//e\"><script src=//e></script>\
<link rel=stylesheet href=a.css><link rel=preload as=script href=e.js><style>p{color:red}</style><base href=//e>",
        );
        assert_eq!(
            clean,
            "<meta name=\"x\" content=\"y\"><link rel=\"stylesheet\" href=\"a.css\"><style>p{color:red}</style>"
        );
    }
}
