// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Blocks and inline nodes as plain HTML, with nothing of the page around
//! them: what `CommonMark`'s tests expect, what a page of someone else's
//! takes (`--fragment`), a glossary entry's meaning, a footer.

use super::blocks::{columns, math_html, sanitize};
use super::nodes::title_attr;
use crate::inline::{Env, HtmlMode};
use crate::model::{
    Align, Block, CodeBlock, Defs, Item, List, ListKind, MathMode, Node, Spacing, Table, Task,
};
use crate::sanitize::Balance;
use crate::text::{UrlFor, esc, esc_into, put, safe_url, url_attr};

/// Where inline nodes stand: HTML has no link inside a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Within {
    /// In running text: a link is written as a link.
    Text,
    /// Inside a link: a link is written as its text.
    Link,
}

/// Inline nodes as HTML with nothing of the page's around them: no glossary,
/// no footnote numbers. A glossary entry's meaning, a footer: the author's
/// tags balanced inside it.
pub(crate) fn bare_inline(nodes: &[Node]) -> String {
    let mut out = String::new();
    plain_inline(nodes, &mut out, Within::Text, &mut Balance::new());
    out
}

/// The `CommonMark` rendering of inline nodes; inside a link, links are
/// only their text. A scope of the author's tags of its own.
fn plain_inline(nodes: &[Node], out: &mut String, within: Within, balance: &mut Balance) {
    let floor = balance.enter();
    for node in nodes {
        match node {
            Node::Text(text) => esc_into(out, text),
            Node::Code(code) => {
                put!(out, "<code>{}</code>", esc(code));
            }
            Node::Html(html) => balance.write(html, 0, out),
            Node::SoftBreak => out.push('\n'),
            Node::HardBreak => out.push_str("<br />\n"),
            Node::Em(kids) => plain_wrap("em", kids, out, within, balance),
            Node::Strong(kids) => plain_wrap("strong", kids, out, within, balance),
            Node::Del(kids) => plain_wrap("del", kids, out, within, balance),
            Node::Link(link) => {
                if within == Within::Link || !safe_url(&link.href, UrlFor::Link) {
                    plain_inline(&link.kids, out, within, balance);
                    continue;
                }
                put!(
                    out,
                    "<a href=\"{}\"{}>",
                    url_attr(&link.href),
                    title_attr(link.title.as_deref())
                );
                plain_inline(&link.kids, out, Within::Link, balance);
                out.push_str("</a>");
            }
            Node::Image(image) => {
                let alt = esc(&crate::inline::plain(&image.alt));
                if !safe_url(&image.src, UrlFor::Image) {
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
    balance.leave(floor, out);
}

/// Nodes inside an element of their own: `<em>`, `<strong>`, `<del>`.
fn plain_wrap(tag: &str, kids: &[Node], out: &mut String, within: Within, balance: &mut Balance) {
    put!(out, "<{tag}>");
    plain_inline(kids, out, within, balance);
    put!(out, "</{tag}>");
}

/// Blocks as plain HTML: what `CommonMark`'s tests expect, and what a page
/// of someone else's takes (`--fragment`). The author's tags are balanced
/// within it (as written for the `CommonMark` examples).
pub(crate) fn fragment(blocks: &[Block], defs: &Defs, html: HtmlMode, math: bool) -> String {
    let mut out = String::new();
    let env = Env { defs, html, math };
    let mut balance = if html == HtmlMode::Spec {
        Balance::as_written()
    } else {
        Balance::new()
    };
    let top = balance.enter();
    for block in blocks {
        fragment_block(block, &env, Spacing::Loose, &mut balance, &mut out);
    }
    balance.leave(top, &mut out);
    if !defs.notes.is_empty() && html != HtmlMode::Spec {
        out.push_str("<section class=\"footnotes\"><ol>\n");
        for (label, note) in &defs.notes {
            put!(out, "<li id=\"fn-{}\">\n", esc(label));
            fragment_blocks(note, &env, Spacing::Loose, &mut balance, &mut out);
            out.push_str("</li>\n");
        }
        out.push_str("</ol></section>\n");
    }
    out
}

/// Blocks inside an element of the fragment's own, a scope of the author's
/// tags.
fn fragment_blocks(
    blocks: &[Block],
    env: &Env<'_>,
    spacing: Spacing,
    balance: &mut Balance,
    out: &mut String,
) {
    let floor = balance.enter();
    for block in blocks {
        fragment_block(block, env, spacing, balance, out);
    }
    balance.leave(floor, out);
}

/// One block of a fragment: the Markdown's HTML with nothing of the page
/// around it. A block of the fragment's own ends the paragraph the author
/// left open before it.
fn fragment_block(
    block: &Block,
    env: &Env<'_>,
    spacing: Spacing,
    balance: &mut Balance,
    out: &mut String,
) {
    let mut html = String::new();
    block_html(block, env, spacing, balance, &mut html);
    if !matches!(block, Block::Html(_)) {
        balance.before_own(&html, out);
    }
    out.push_str(&html);
}

/// One block's HTML. A tight list's paragraphs are not wrapped.
fn block_html(
    block: &Block,
    env: &Env<'_>,
    spacing: Spacing,
    balance: &mut Balance,
    out: &mut String,
) {
    match block {
        Block::Paragraph(paragraph) => {
            let nodes = crate::inline::parse(&paragraph.text, env);
            match spacing {
                Spacing::Tight => plain_inline(&nodes, out, Within::Text, balance),
                Spacing::Loose => {
                    out.push_str("<p>");
                    plain_inline(&nodes, out, Within::Text, balance);
                    out.push_str("</p>\n");
                }
            }
        }
        Block::Heading(heading) => {
            put!(out, "<h{}>", heading.level);
            plain_inline(
                &crate::inline::parse(&heading.text, env),
                out,
                Within::Text,
                balance,
            );
            put!(out, "</h{}>\n", heading.level);
        }
        Block::Code(code) => fragment_code(code, out),
        Block::Html(html) => match env.html {
            HtmlMode::Safe => {
                if let Some(clean) = sanitize(&html.text) {
                    balance.write(&clean, 0, out);
                    out.push('\n');
                } else {
                    put!(out, "<pre>{}</pre>\n", esc(&html.text));
                }
            }
            HtmlMode::Rich => {
                balance.write(&crate::sanitize::rich_block(&html.text), 0, out);
                out.push('\n');
            }
            HtmlMode::Spec => put!(out, "{}\n", html.text),
        },
        Block::Rule => out.push_str("<hr />\n"),
        Block::Quote(quote) => {
            out.push_str("<blockquote>\n");
            fragment_blocks(&quote.blocks, env, Spacing::Loose, balance, out);
            out.push_str("</blockquote>\n");
        }
        Block::List(list) => fragment_list(list, env, balance, out),
        Block::Table(table) => fragment_table(table, env, balance, out),
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
fn fragment_list(list: &List, env: &Env<'_>, balance: &mut Balance, out: &mut String) {
    match list.kind {
        ListKind::Ordered(1) => out.push_str("<ol>\n"),
        ListKind::Ordered(start) => put!(out, "<ol start=\"{start}\">\n"),
        ListKind::Bullet => out.push_str("<ul>\n"),
    }
    for item in &list.items {
        fragment_item(item, list.spacing, env, balance, out);
    }
    out.push_str(match list.kind {
        ListKind::Ordered(_) => "</ol>\n",
        ListKind::Bullet => "</ul>\n",
    });
}

/// A list item of a fragment: in a tight list a paragraph is its bare
/// text, any other block starts on a line of its own. A scope of the
/// author's tags.
fn fragment_item(
    item: &Item,
    spacing: Spacing,
    env: &Env<'_>,
    balance: &mut Balance,
    out: &mut String,
) {
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
    let floor = balance.enter();
    for (at, block) in item.blocks.iter().enumerate() {
        let bare = spacing == Spacing::Tight && matches!(block, Block::Paragraph(_));
        if at == 0 && !bare {
            out.push('\n');
        }
        fragment_block(block, env, spacing, balance, out);
        if bare && several {
            out.push('\n');
        }
    }
    balance.leave(floor, out);
    out.push_str("</li>\n");
}

/// A table of a fragment, its columns' alignment as attributes.
fn fragment_table(table: &Table, env: &Env<'_>, balance: &mut Balance, out: &mut String) {
    out.push_str("<table>\n<thead>\n<tr>\n");
    for (cell, align) in table.head.iter().zip(columns(&table.align)) {
        put!(out, "<th{}>", align_attr(align));
        plain_inline(&crate::inline::parse(cell, env), out, Within::Text, balance);
        out.push_str("</th>\n");
    }
    out.push_str("</tr>\n</thead>\n");
    if !table.rows.is_empty() {
        out.push_str("<tbody>\n");
        for row in &table.rows {
            out.push_str("<tr>\n");
            for (cell, align) in row.iter().zip(columns(&table.align)) {
                put!(out, "<td{}>", align_attr(align));
                plain_inline(&crate::inline::parse(cell, env), out, Within::Text, balance);
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
