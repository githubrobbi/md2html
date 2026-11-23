// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Inline nodes in the article: text with its glossary links, emphasis,
//! links sent where the page can follow them, images, footnote numbers,
//! maths.

use std::path::Path;

use super::Renderer;
use super::blocks::math_html;
use crate::model::{Link, Node};
use crate::text::{UrlFor, esc, is_relative, put, safe_url, url_attr};

/// What inline text may link to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Links {
    /// Its own links, and the glossary's words to their entries.
    Glossary,
    /// Its own links only.
    Own,
    /// Nothing: the text is inside a link already (a card is one).
    None,
}

impl Renderer<'_, '_> {
    /// Inline nodes as HTML.
    pub(super) fn inline(&mut self, nodes: &[Node], links: Links) -> String {
        let mut out = String::new();
        self.inline_into(nodes, links, &mut out);
        out
    }

    /// Writes inline nodes as HTML.
    fn inline_into(&mut self, nodes: &[Node], links: Links, out: &mut String) {
        let glossary = links == Links::Glossary && !self.inp.terms.is_empty();
        for node in nodes {
            match node {
                Node::Text(text) => {
                    let escaped = esc(text);
                    if glossary {
                        out.push_str(&self.linker.link(&escaped, self.inp.terms));
                    } else {
                        out.push_str(&escaped);
                    }
                }
                Node::Code(code) => {
                    let rendered = format!("<code>{}</code>", esc(code));
                    if glossary {
                        out.push_str(&self.linker.code(code, &rendered, self.inp.terms));
                    } else {
                        out.push_str(&rendered);
                    }
                }
                Node::Html(html) => out.push_str(html),
                Node::SoftBreak => out.push('\n'),
                Node::HardBreak => out.push_str("<br>\n"),
                Node::Em(kids) => self.wrap("em", kids, links, out),
                Node::Strong(kids) => self.wrap("strong", kids, links, out),
                Node::Del(kids) => self.wrap("del", kids, links, out),
                Node::Link(link) => self.link(link, links, out),
                Node::Image(image) => {
                    let html = self.image(image);
                    out.push_str(&html);
                }
                Node::Note(label) => {
                    let html = self.note_ref(label);
                    out.push_str(&html);
                }
                Node::Math(math) => {
                    out.push_str(&math_html(&math.tex, math.mode));
                }
            }
        }
    }

    /// Nodes inside an element of their own: `<em>`, `<strong>`, `<del>`.
    fn wrap(&mut self, tag: &str, kids: &[Node], links: Links, out: &mut String) {
        put!(out, "<{tag}>");
        self.inline_into(kids, links, out);
        put!(out, "</{tag}>");
    }

    /// A link: an `<a>`, or only its text when it is inside another or its
    /// destination must not be followed.
    fn link(&mut self, link: &Link, links: Links, out: &mut String) {
        if links == Links::None {
            self.inline_into(&link.kids, Links::None, out);
            return;
        }
        let Some(url) = self.href(&link.href, UrlFor::Link) else {
            self.inline_into(&link.kids, links, out);
            return;
        };
        let title = title_attr(link.title.as_deref());
        // A link that shows its own address needs it no second time in
        // print.
        let shown = crate::inline::plain(&link.kids);
        let bare = shown == link.href
            || format!("http://{shown}") == link.href
            || format!("mailto:{shown}") == link.href;
        let class = if bare { " class=\"url\"" } else { "" };
        put!(out, "<a{class} href=\"{url}\"{title}>");
        self.inline_into(&link.kids, Links::None, out);
        out.push_str("</a>");
    }

    /// A destination fit for the page: script URLs refused, a link to a
    /// Markdown file next to this one sent to its HTML twin, an image put
    /// inside the page when asked. None when it must not be a link.
    pub(super) fn href(&mut self, url: &str, url_for: UrlFor) -> Option<String> {
        if !safe_url(url, url_for) {
            let scheme = url.split(':').next().unwrap_or_default().trim().to_owned();
            self.warn(format!(
                "a link to a `{scheme}:` address was removed; only its text is shown"
            ));
            return None;
        }
        if !is_relative(url) || url.is_empty() || self.inp.bases.is_empty() {
            return Some(url_attr(url));
        }
        let (path, rest) = split_path(url);
        // A path that decodes to bytes which are not UTF-8 names no file.
        let Some(decoded) = crate::text::percent_decode_path(path) else {
            self.warn(format!("`{path}` does not exist next to this page"));
            return Some(url_attr(url));
        };
        let found = self
            .inp
            .bases
            .iter()
            .map(|base| base.join(&decoded))
            .find(|target| target.exists());
        if !path.is_empty() && found.is_none() {
            self.warn(format!("`{decoded}` does not exist next to this page"));
        }
        let Some(target) = found.filter(|target| target.is_file()) else {
            return Some(url_attr(url));
        };
        let markdown = Path::new(&decoded).extension().is_some_and(|ext| {
            ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
        });
        if url_for == UrlFor::Link && markdown {
            let stem = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
            return Some(url_attr(&format!("{stem}.html{rest}")));
        }
        if url_for == UrlFor::Image && self.inp.embed {
            match super::embed::data_uri(&target, &self.inp.bases, self.inp.max_embed) {
                Ok(data) => return Some(data),
                Err(why) => self.warn(format!("`{decoded}` is not embedded: {why}")),
            }
        }
        Some(url_attr(url))
    }
}

/// A relative URL's path, and the `?query` or `#fragment` after it.
fn split_path(url: &str) -> (&str, &str) {
    url.find(['#', '?'])
        .map_or((url, ""), |cut| url.split_at(cut))
}

/// A `title` attribute, or nothing without a title.
pub(super) fn title_attr(title: Option<&str>) -> String {
    title
        .map(|text| format!(" title=\"{}\"", esc(text)))
        .unwrap_or_default()
}
