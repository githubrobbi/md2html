// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Raw HTML in the text, and what of it a page may keep. A page md2html
//! writes runs only md2html's own code, whatever the Markdown holds.
//!
//! Where a tag ends is read as `CommonMark` reads it ([`html_tag`]), for
//! the Markdown readers and for this filter alike. Without `--html`, a
//! short list of harmless tags passes ([`safe_tag`]) and anything else is
//! shown as written. With `--html`, the author's own HTML is rich
//! (images, links, video, SVG, layout), never executable.
//!
//! Elements pass by an allowlist; scripts, frames, plug-ins, forms and
//! the elements that change how the page loads are dropped, the first
//! kinds with everything inside them. An attribute that could run or
//! fetch code (an event handler, `srcdoc`, `formaction`, ...) is dropped
//! wherever it stands; any other passes only where HTML defines it
//! ([`defined`], one table with the book's), and the author is told of
//! the rest. Every URL is checked as a Markdown link's is; a style that
//! could fetch or run anything is dropped; ids that the page's own script
//! uses are not taken. The same filter cleans the `head` file an author
//! adds.
//!
//! Whatever passes is balanced ([`Balance`]): the author's tags can
//! neither close nor leave open an element of the page around them.

mod attributes;
mod balance;
mod lists;
mod markup;
mod tag;

pub(crate) use self::attributes::{Reader, defined};
pub(crate) use self::balance::{Balance, Own};
use self::lists::{
    DENIED, DROP_WITH_CONTENT, ELEMENTS, HEAD_ATTRS, LINK_RELS, PREFIXED, RESERVED_IDS,
    STYLE_DENIED, URL_ATTRS,
};
use self::markup::{Attr, Markup, Opening, Piece, attributes, before_close, opens};
pub(crate) use self::tag::{html_tag, read_tag, safe_tag};
use crate::model::Undefined;
use crate::scan::Scan;
use crate::text::{Html, UrlFor, decode_entities, esc, put, safe_url};

/// The author's HTML as the page keeps it, and what was left out of it
/// because HTML does not define it, for the author to be told.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Cleaned {
    /// The markup the page keeps.
    pub(crate) html: String,
    /// Each attribute left out as undefined, in the order it stood.
    pub(crate) undefined: Vec<Undefined>,
}

/// What becomes of one attribute of an author's tag.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Verdict {
    /// It stays, with this value (entities decoded, not yet escaped).
    Value(String),
    /// It stays, bare, as it was written.
    Bare,
    /// It is left out, and the author told: HTML does not define it on
    /// the element.
    Undefined,
    /// It is dropped: it could run or fetch something, or its value
    /// could.
    Drop,
}

/// An author's tag as it may stand in the page, or None to drop it.
pub(crate) fn rich_tag(tag: &str) -> Option<Cleaned> {
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
    let wanted = list_of(&written).unwrap_or(&written);
    let name = ELEMENTS.iter().copied().find(|known| *known == wanted)?;
    let mut kept = Cleaned::default();
    if closing {
        put!(kept.html, "</{name}>");
        return Some(kept);
    }
    put!(kept.html, "<{name}");
    let mut target = false;
    for attr in attributes(scan.rest()) {
        match attribute(name, &attr) {
            Verdict::Value(value) => put!(kept.html, " {}=\"{}\"", attr.name, esc(&value)),
            Verdict::Bare => put!(kept.html, " {}", attr.name),
            Verdict::Undefined => {
                kept.undefined.push(Undefined {
                    element: name,
                    attr: attr.name,
                });
                continue;
            }
            Verdict::Drop => continue,
        }
        target |= attr.name == "target";
    }
    if target && name == "a" {
        kept.html.push_str(" rel=\"noopener noreferrer\"");
    }
    kept.html.push_str(if self_closing { "/>" } else { ">" });
    Some(kept)
}

/// The list element that an obsolete list element stands for: `<dir>`
/// (HTML 4) and `<menu>` hold `<li>` items like `<ul>`, and dropping them
/// would leave their items outside any list.
fn list_of(name: &str) -> Option<&'static str> {
    matches!(name, "dir" | "menu").then_some("ul")
}

/// What becomes of `attr` on an `element`: what could run or fetch
/// anything is dropped first, defined or not; then what HTML does not
/// define there is left out.
fn attribute(element: &str, attr: &Attr<'_>) -> Verdict {
    if !allowed_name(&attr.name) {
        return Verdict::Drop;
    }
    if !defined(Reader::Browser, element, &attr.name) {
        return Verdict::Undefined;
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

/// Whether an attribute of this name may stand at all.
///
/// A plain lowercase name, no event handler, nothing denied, no `rel` (the
/// filter writes that itself), and a name with a prefix only when it is
/// one of [`PREFIXED`].
///
/// An XML reader (the EPUB's) gives a prefix the meaning its `xmlns:`
/// declaration names, so `x:href` with `xmlns:x` set to the `xlink:` namespace
/// is a link the URL check, which knows the name `xlink:href`, would not
/// see. No namespace is the author's to declare: `xmlns` is refused too.
fn allowed_name(name: &str) -> bool {
    let plain = name.starts_with(|ch: char| ch.is_ascii_lowercase())
        && name.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | ':' | '.')
        });
    let prefix_known = !name.contains(':') || PREFIXED.contains(&name);
    plain
        && prefix_known
        && name != "xmlns"
        && !name.starts_with("on")
        && !DENIED.contains(&name)
        && name != "rel"
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
    let url_for = if matches!(key, "src" | "srcset" | "poster")
        || matches!(element, "image" | "use") && key.ends_with("href")
    {
        UrlFor::Image
    } else {
        UrlFor::Link
    };
    match key {
        "srcset" => value.split(',').all(|candidate| {
            candidate
                .split_whitespace()
                .next()
                .is_none_or(|url| safe_url(url, url_for))
        }),
        "usemap" => value.starts_with('#'),
        _ => safe_url(value, url_for),
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
pub(crate) fn rich_block(text: &str) -> Cleaned {
    let mut out = Cleaned {
        html: String::with_capacity(text.len()),
        undefined: Vec::new(),
    };
    let mut markup = Markup::new(text);
    while let Some(piece) = markup.next_piece() {
        match piece {
            Piece::Text(words) => out.html.push_str(words),
            Piece::Stray => out.html.push_str("&lt;"),
            Piece::Comment => {}
            Piece::Tag(tag) => match opens(tag) {
                Some(opening) if DROP_WITH_CONTENT.contains(&opening.name.as_str()) => {
                    markup.skip_element(&opening.name);
                }
                Some(_) | None => {
                    if let Some(clean) = rich_tag(tag) {
                        out.html.push_str(&clean.html);
                        out.undefined.extend(clean.undefined);
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
pub(crate) fn head(text: &str) -> Html {
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
    Html::markup(out)
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
    }) && lowered(attrs, "href").is_none_or(|href| safe_url(&href, UrlFor::Link))
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
mod tests;
