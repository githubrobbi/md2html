// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Blocks and inline nodes as plain HTML, with nothing of the page around
//! them: what `CommonMark`'s tests expect, what a page of someone else's
//! takes (`--fragment`), a glossary entry's meaning, a footer.

use super::blocks::{columns, math_html, sanitize};
use super::nodes::title_attr;
use crate::inline::{Env, HtmlMode};
use crate::model::{
    Align, Block, CodeBlock, Defs, Item, List, ListKind, MathMode, Node, Table, Task,
};
use crate::text::{esc, put, safe_url, url_attr};

/// Inline nodes as HTML with nothing of the page's around them: no glossary,
/// no footnote numbers. A glossary entry's meaning, a footer.
pub(crate) fn bare_inline(nodes: &[Node]) -> String {
    let mut out = String::new();
    plain_inline(nodes, &mut out, false);
    out
}

/// The `CommonMark` rendering of inline nodes; inside a link, links are
/// only their text.
fn plain_inline(nodes: &[Node], out: &mut String, in_link: bool) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(&esc(text)),
            Node::Code(code) => {
                put!(out, "<code>{}</code>", esc(code));
            }
            Node::Html(html) => out.push_str(html),
            Node::SoftBreak => out.push('\n'),
            Node::HardBreak => out.push_str("<br />\n"),
            Node::Em(kids) => plain_wrap("em", kids, out, in_link),
            Node::Strong(kids) => plain_wrap("strong", kids, out, in_link),
            Node::Del(kids) => plain_wrap("del", kids, out, in_link),
            Node::Link(link) => {
                if in_link || !safe_url(&link.href, false) {
                    plain_inline(&link.kids, out, in_link);
                    continue;
                }
                put!(
                    out,
                    "<a href=\"{}\"{}>",
                    url_attr(&link.href),
                    title_attr(link.title.as_deref())
                );
                plain_inline(&link.kids, out, true);
                out.push_str("</a>");
            }
            Node::Image(image) => {
                let alt = esc(&crate::inline::plain(&image.alt));
                if !safe_url(&image.src, true) {
                    out.push_str(&alt);
                    continue;
                }
                put!(
                    out,
                    "<img src=\"{}\" alt=\"{alt}\"{} />",
                    url_attr(&image.src),
                    title_attr(image.title.as_deref())
                );
            }
            Node::Note(label) => {
                let label_text = esc(label);
                put!(
                    out,
                    "<sup class=\"fn\"><a href=\"#fn-{label_text}\">{label_text}</a></sup>"
                );
            }
            Node::Math(math) => {
                out.push_str(&math_html(&math.tex, math.mode));
            }
        }
    }
}

/// Nodes inside an element of their own: `<em>`, `<strong>`, `<del>`.
fn plain_wrap(tag: &str, kids: &[Node], out: &mut String, in_link: bool) {
    put!(out, "<{tag}>");
    plain_inline(kids, out, in_link);
    put!(out, "</{tag}>");
}

/// Blocks as plain HTML: what `CommonMark`'s tests expect, and what a page
/// of someone else's takes (`--fragment`).
pub(crate) fn fragment(blocks: &[Block], defs: &Defs, html: HtmlMode, math: bool) -> String {
    let mut out = String::new();
    let env = Env { defs, html, math };
    for block in blocks {
        fragment_block(block, &env, false, &mut out);
    }
    if !defs.notes.is_empty() && html != HtmlMode::Spec {
        out.push_str("<section class=\"footnotes\"><ol>\n");
        for (label, note) in &defs.notes {
            put!(out, "<li id=\"fn-{}\">\n", esc(label));
            for block in note {
                fragment_block(block, &env, false, &mut out);
            }
            out.push_str("</li>\n");
        }
        out.push_str("</ol></section>\n");
    }
    out
}

/// One block of a fragment: the Markdown's HTML with nothing of the page
/// around it. A tight list's paragraphs are not wrapped.
fn fragment_block(block: &Block, env: &Env<'_>, tight: bool, out: &mut String) {
    match block {
        Block::Paragraph(paragraph) => {
            let nodes = crate::inline::parse(&paragraph.text, env);
            if tight {
                plain_inline(&nodes, out, false);
            } else {
                out.push_str("<p>");
                plain_inline(&nodes, out, false);
                out.push_str("</p>\n");
            }
        }
        Block::Heading(heading) => {
            put!(out, "<h{}>", heading.level);
            plain_inline(&crate::inline::parse(&heading.text, env), out, false);
            put!(out, "</h{}>\n", heading.level);
        }
        Block::Code(code) => fragment_code(code, out),
        Block::Html(html) => {
            if env.html == HtmlMode::Safe {
                if let Some(clean) = sanitize(&html.text) {
                    put!(out, "{clean}\n");
                } else {
                    put!(out, "<pre>{}</pre>\n", esc(&html.text));
                }
            } else {
                put!(out, "{}\n", html.text);
            }
        }
        Block::Rule => out.push_str("<hr />\n"),
        Block::Quote(quote) => {
            out.push_str("<blockquote>\n");
            for inner in &quote.blocks {
                fragment_block(inner, env, false, out);
            }
            out.push_str("</blockquote>\n");
        }
        Block::List(list) => fragment_list(list, env, out),
        Block::Table(table) => fragment_table(table, env, out),
        Block::Math(math) => {
            put!(out, "{}\n", math_html(&math.tex, MathMode::Display));
        }
    }
}

/// A code block of a fragment, its language as a class.
fn fragment_code(code: &CodeBlock, out: &mut String) {
    let lang = code.info.split_whitespace().next().unwrap_or_default();
    if lang.is_empty() {
        out.push_str("<pre><code>");
    } else {
        put!(out, "<pre><code class=\"language-{}\">", esc(lang));
    }
    put!(out, "{}</code></pre>\n", esc(&code.text));
}

/// A list of a fragment.
fn fragment_list(list: &List, env: &Env<'_>, out: &mut String) {
    match list.kind {
        ListKind::Ordered(1) => out.push_str("<ol>\n"),
        ListKind::Ordered(start) => put!(out, "<ol start=\"{start}\">\n"),
        ListKind::Bullet => out.push_str("<ul>\n"),
    }
    for item in &list.items {
        fragment_item(item, list.tight, env, out);
    }
    out.push_str(match list.kind {
        ListKind::Ordered(_) => "</ol>\n",
        ListKind::Bullet => "</ul>\n",
    });
}

/// A list item of a fragment: in a tight list a paragraph is its bare
/// text, any other block starts on a line of its own.
fn fragment_item(item: &Item, tight: bool, env: &Env<'_>, out: &mut String) {
    out.push_str("<li>");
    if let Some(task) = item.task {
        let checked = if task == Task::Done {
            " checked=\"\""
        } else {
            ""
        };
        put!(out, "<input type=\"checkbox\" disabled=\"\"{checked} /> ");
    }
    let several = item.blocks.len() > 1;
    for (at, block) in item.blocks.iter().enumerate() {
        let bare = tight && matches!(block, Block::Paragraph(_));
        if at == 0 && !bare {
            out.push('\n');
        }
        fragment_block(block, env, tight, out);
        if bare && several {
            out.push('\n');
        }
    }
    out.push_str("</li>\n");
}

/// A table of a fragment, its columns' alignment as attributes.
fn fragment_table(table: &Table, env: &Env<'_>, out: &mut String) {
    out.push_str("<table>\n<thead>\n<tr>\n");
    for (cell, align) in table.head.iter().zip(columns(&table.align)) {
        put!(out, "<th{}>", align_attr(align));
        plain_inline(&crate::inline::parse(cell, env), out, false);
        out.push_str("</th>\n");
    }
    out.push_str("</tr>\n</thead>\n");
    if !table.rows.is_empty() {
        out.push_str("<tbody>\n");
        for row in &table.rows {
            out.push_str("<tr>\n");
            for (cell, align) in row.iter().zip(columns(&table.align)) {
                put!(out, "<td{}>", align_attr(align));
                plain_inline(&crate::inline::parse(cell, env), out, false);
                out.push_str("</td>\n");
            }
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody>\n");
    }
    out.push_str("</table>\n");
}

/// A cell's `align` attribute for its column.
const fn align_attr(align: Align) -> &'static str {
    match align {
        Align::None => "",
        Align::Left => " align=\"left\"",
        Align::Center => " align=\"center\"",
        Align::Right => " align=\"right\"",
    }
}
