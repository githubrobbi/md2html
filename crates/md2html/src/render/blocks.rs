// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! One block of the article, wherever it stands: paragraphs, code and
//! diagrams, raw HTML, quotes and callouts, lists, tables, maths.

use core::iter;

use super::Renderer;
use super::nodes::Links;
use crate::config::Diagrams;
use crate::inline::HtmlMode;
use crate::model::{
    Align, Block, CodeBlock, Heading, List, ListKind, MathMode, Paragraph, Spacing, Table, Task,
};
use crate::sanitize::{Own, html_tag, safe_tag};
use crate::text::{esc, put, slug};

impl Renderer<'_, '_> {
    /// One block, anywhere: at the top, in a list, in a quote, in a note.
    /// A block of md2html's own ends the paragraph the author left open
    /// before it, as HTML does, and the author's drawing or table, which
    /// cannot hold it, before anything of it is written.
    pub(super) fn block(&mut self, block: &Block, spacing: Spacing) -> String {
        let mut out = String::new();
        self.write_block(block, spacing, &mut out);
        out
    }

    /// [`Self::block`], written at the end of `out`. A quote or a list
    /// writes the blocks it holds where they stand, so a block nested deep
    /// is written once, not made apart and copied again at every level
    /// around it.
    pub(super) fn write_block(&mut self, block: &Block, spacing: Spacing, out: &mut String) {
        if let Some(line) = block.line() {
            self.line = line;
        }
        if let Some(name) = opens(block) {
            self.balance.own_start(name, out);
        }
        let leaf = match block {
            Block::Quote(quote) => {
                self.quote(&quote.blocks, out);
                None
            }
            Block::List(list) => {
                self.list(list, out);
                None
            }
            Block::Paragraph(paragraph) => Some(self.paragraph(paragraph, spacing)),
            Block::Heading(heading) => Some(self.heading_html(heading)),
            Block::Code(code) => Some(self.code(code)),
            Block::Html(html) => Some(self.html_block(&html.text)),
            Block::Rule => Some(String::from("<hr>")),
            Block::Table(table) => Some(self.table(table)),
            Block::Math(math) => Some(format!(
                "<div class=\"math\">{}</div>",
                math_block_html(&math.tex)
            )),
        };
        if let Some(html) = leaf {
            out.push_str(&html);
        }
        self.tell_mends();
    }

    /// A heading of a quote, a list or a note, with its anchor.
    fn heading_html(&mut self, heading: &Heading) -> String {
        let nodes = self.parse(&heading.text);
        let shown = crate::inline::plain(&nodes);
        let wanted = heading.id.clone().unwrap_or_else(|| slug(&shown));
        let id = self.unique(&wanted);
        let html = self.inline(&nodes, Links::Own, Own::heading(heading.level));
        format!(
            "<h{level} id=\"{id}\">{html}</h{level}>",
            level = heading.level
        )
    }

    /// A paragraph; an image alone in one is a figure. A tight list's
    /// paragraphs are not wrapped: they are their item's text.
    fn paragraph(&mut self, paragraph: &Paragraph, spacing: Spacing) -> String {
        if paragraph.text.starts_with("SPDX-") {
            return String::new();
        }
        let nodes = self.parse(&paragraph.text);
        let mut out = String::new();
        if let Some(image) = super::figure::lone_image(&nodes) {
            self.balance.own_start("figure", &mut out);
            out.push_str(&self.figure(image));
            return out;
        }
        let own = match spacing {
            Spacing::Tight => Own::NONE,
            Spacing::Loose => {
                self.balance.own_start("p", &mut out);
                Own::PARAGRAPH
            }
        };
        let html = self.inline(&nodes, Links::Glossary, own);
        if html.contains("[^") {
            self.warn("a footnote reference [^...] has no definition; it is shown as written");
        }
        match spacing {
            Spacing::Tight => out.push_str(&html),
            Spacing::Loose => put!(out, "<p>{html}</p>"),
        }
        out
    }

    /// A code block, highlighted where the language is known.
    fn code(&mut self, code: &CodeBlock) -> String {
        let lang = code.info.split_whitespace().next().unwrap_or_default();
        if lang.eq_ignore_ascii_case("mermaid") {
            if let Some(html) = self.diagram(&code.text) {
                return html;
            }
        }
        let body = crate::highlight::highlight(lang, &code.text).unwrap_or_else(|| esc(&code.text));
        let (label, named, class) = if lang.is_empty() {
            (String::new(), "", String::new())
        } else {
            (
                format!("<span class=\"lang\">{}</span>", esc(lang)),
                " named",
                format!(" class=\"language-{}\"", esc(&lang.to_ascii_lowercase())),
            )
        };
        let id = self.block_id("code");
        format!(
            "<div class=\"code{named}\" id=\"{id}\">{label}<button class=\"copy\" type=\"button\">{}</button><pre><code{class}>{body}</code></pre></div>",
            esc(self.inp.ui.copy.label)
        )
    }

    /// A Mermaid block as a picture: drawn here, or left to mermaid.js, or
    /// None to show it as code.
    fn diagram(&mut self, source: &str) -> Option<String> {
        match self.inp.mermaid {
            Diagrams::Code => None,
            Diagrams::Js => Some(self.diagram_for_js(source)),
            Diagrams::Native => {
                self.diagrams += 1;
                match crate::mermaid::render(source, self.diagrams) {
                    crate::mermaid::Outcome::Drawn(svg) => {
                        let id = self.block_id("diagram");
                        Some(svg.replacen(
                            "<figure class=\"diagram",
                            &format!("<figure id=\"{id}\" class=\"diagram"),
                            1,
                        ))
                    }
                    crate::mermaid::Outcome::Unsupported(_)
                    | crate::mermaid::Outcome::Unreadable(_)
                        if self.inp.mermaid_js =>
                    {
                        Some(self.diagram_for_js(source))
                    }
                    crate::mermaid::Outcome::Unsupported(kind) => {
                        self.warn(format!(
                            "a Mermaid `{kind}` diagram is shown as code: md2html draws flowcharts and sequence diagrams, and --mermaid-js FILE draws the rest"
                        ));
                        None
                    }
                    crate::mermaid::Outcome::Unreadable(why) => {
                        self.warn(format!(
                            "a Mermaid diagram could not be read ({why}); it is shown as code"
                        ));
                        None
                    }
                }
            }
        }
    }

    /// A Mermaid block left to mermaid.js in the browser.
    fn diagram_for_js(&mut self, source: &str) -> String {
        self.art.mermaid_js = true;
        let id = self.block_id("diagram");
        format!(
            "<div class=\"diagram-js\" id=\"{id}\"><pre class=\"mermaid\">{}</pre></div>",
            esc(source)
        )
    }

    /// Raw HTML: with `--html` rich but never executable; otherwise only
    /// the safe tags pass, and a block holding any other is shown as text.
    /// The author's tags are balanced either way (`sanitize::Balance`).
    fn html_block(&mut self, text: &str) -> String {
        let mut out = String::new();
        match self.inp.html {
            HtmlMode::Spec => put!(out, "{text}\n"),
            HtmlMode::Rich => {
                let clean = crate::sanitize::rich_block(text);
                self.balance.write(&clean, self.line, &mut out);
                out.push('\n');
            }
            HtmlMode::Safe => {
                let trimmed = text.trim();
                if trimmed.starts_with("<!--")
                    && trimmed.ends_with("-->")
                    && trimmed.matches("-->").count() == 1
                {
                    return out;
                }
                if let Some(clean) = sanitize(text) {
                    self.balance.write(&clean, self.line, &mut out);
                } else {
                    self.warn("raw HTML is shown as written; render with --html to let it through");
                    put!(out, "<pre class=\"verbatim\">{}</pre>", esc(text));
                }
            }
        }
        out
    }

    /// A block quote: a callout when its first words name one, a quote
    /// otherwise.
    fn quote(&mut self, blocks: &[Block], out: &mut String) {
        if self.callout(blocks, out) {
            return;
        }
        // A quote that opens with a bold lead ("**For the night shift.**
        // ...") is an aside the author set apart, as every quote was on
        // md2html's first pages: a note, its lead kept.
        let bold_lead = blocks.first().and_then(paragraph).is_some_and(|first| {
            first
                .text
                .strip_prefix("**")
                .and_then(|rest| rest.find("**"))
                .is_some_and(|end| end > 0 && end <= 80)
        });
        let (own, end) = if bold_lead {
            put!(
                out,
                "<div class=\"callout note\" role=\"note\"><span class=\"mark\">{}</span><div class=\"body\">",
                esc(self.inp.ui.callouts.note)
            );
            (Own::DIV, "</div></div>")
        } else {
            out.push_str("<blockquote>");
            (Own::QUOTE, "</blockquote>")
        };
        self.scoped(own, out, |this, inner| {
            for block in blocks {
                this.write_block(block, Spacing::Loose, inner);
            }
        });
        out.push_str(end);
    }

    /// A list; a tight one's paragraphs are not wrapped.
    fn list(&mut self, list: &List, out: &mut String) {
        let tasks = list.items.iter().any(|item| item.task.is_some());
        match list.kind {
            ListKind::Ordered(1) => out.push_str("<ol>"),
            ListKind::Ordered(start) => put!(out, "<ol start=\"{start}\">"),
            ListKind::Bullet if tasks => out.push_str("<ul class=\"tasks\">"),
            ListKind::Bullet => out.push_str("<ul>"),
        }
        for item in &list.items {
            match item.task {
                Some(task) => {
                    put!(
                        out,
                        "<li class=\"task\"><input type=\"checkbox\" disabled{} aria-label=\"{}\"> ",
                        if task == Task::Done { " checked" } else { "" },
                        esc(self.inp.ui.task.word(task))
                    );
                }
                None => out.push_str("<li>"),
            }
            self.scoped(Own::ITEM, out, |this, body| {
                for (at, block) in item.blocks.iter().enumerate() {
                    if at > 0 && list.spacing == Spacing::Tight {
                        body.push('\n');
                    }
                    this.write_block(block, list.spacing, body);
                }
            });
            out.push_str("</li>");
        }
        out.push_str(match list.kind {
            ListKind::Ordered(_) => "</ol>",
            ListKind::Bullet => "</ul>",
        });
    }

    /// A table in its scrolling frame.
    fn table(&mut self, table: &Table) -> String {
        let mut out = String::from("<div class=\"scroll\"><table><thead><tr>");
        for (cell, align) in table.head.iter().zip(columns(&table.align)) {
            let nodes = self.parse(cell);
            let start = out.len();
            put!(out, "<th scope=\"col\"{}>", class(align));
            let inside = out.len();
            self.inline_into(&nodes, Links::Glossary, Own::HEAD_CELL, &mut out);
            if out.len() == inside {
                // An empty head cell heads nothing.
                out.truncate(start);
                out.push_str("<td></td>");
            } else {
                out.push_str("</th>");
            }
        }
        out.push_str("</tr></thead><tbody>");
        // Each row has an address of its own, for a link to exactly it: its
        // section and its first cell (`7-settings-lang`), so it holds when
        // rows or tables are added elsewhere.
        let section = self.section_id().unwrap_or("row").to_owned();
        for (number, row) in (1_usize..).zip(&table.rows) {
            // The first cell is read once: for the row's address and as
            // the cell it is.
            let mut first_cell = row.first().map(|cell| self.parse(cell));
            let first = first_cell
                .as_deref()
                .map(crate::inline::plain)
                .unwrap_or_default();
            let mut key: String = slug(&first).chars().take(40).collect();
            if key.is_empty() {
                key = format!("r{number}");
            }
            let id = self.unique(&format!("{section}-{key}"));
            put!(out, "<tr id=\"{}\">", esc(&id));
            for (cell, align) in row.iter().zip(columns(&table.align)) {
                let nodes = first_cell.take().unwrap_or_else(|| self.parse(cell));
                put!(out, "<td{}>", class(align));
                self.inline_into(&nodes, Links::Glossary, Own::CELL, &mut out);
                out.push_str("</td>");
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table></div>");
        out
    }
}

/// The start tag md2html writes a block with, before it is written; None
/// for raw HTML, which is the author's, and a paragraph, which says itself
/// (it may be a figure, or a tight list's bare text).
pub(super) fn opens(block: &Block) -> Option<&'static str> {
    match block {
        Block::Heading(heading) => Own::heading(heading.level).name(),
        Block::Paragraph(_) | Block::Html(_) => None,
        Block::Rule => Some("hr"),
        Block::Quote(_) => Some("blockquote"),
        Block::List(_) => Some("ul"),
        Block::Code(_) | Block::Table(_) | Block::Math(_) => Some("div"),
    }
}

/// The paragraph a block is, if it is one.
pub(super) const fn paragraph(block: &Block) -> Option<&Paragraph> {
    match block {
        Block::Paragraph(paragraph) => Some(paragraph),
        Block::Heading(_)
        | Block::Code(_)
        | Block::Html(_)
        | Block::Rule
        | Block::Quote(_)
        | Block::List(_)
        | Block::Table(_)
        | Block::Math(_) => None,
    }
}

/// A table's alignment for every column: as many as there are cells, the
/// ones its delimiter row does not reach unaligned.
pub(super) fn columns(align: &[Align]) -> impl Iterator<Item = Align> + '_ {
    align.iter().copied().chain(iter::repeat(Align::None))
}

/// A cell's class for its column's alignment.
const fn class(align: Align) -> &'static str {
    match align {
        Align::Center => " class=\"c\"",
        Align::Right => " class=\"r\"",
        Align::None | Align::Left => "",
    }
}

/// A formula in running text as `MathML`, or its TeX as code in the line
/// when this tool cannot read it, displayed (`$$`) or not: a block there
/// would end the paragraph it stands in.
pub(super) fn math_html(tex: &str, mode: MathMode) -> String {
    crate::math::mathml(tex, mode)
        .unwrap_or_else(|| format!("<code class=\"tex\">{}</code>", esc(tex)))
}

/// A formula that is a block of its own as `MathML`, or its TeX as a block
/// of code when this tool cannot read it.
pub(super) fn math_block_html(tex: &str) -> String {
    crate::math::mathml(tex, MathMode::Display)
        .unwrap_or_else(|| format!("<pre class=\"tex\">{}</pre>", esc(tex)))
}

/// HTML with only the safe tags, or None if it holds any other. A `<`
/// that opens no tag is text.
pub(super) fn sanitize(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        if let Some((tag, after)) = split_tag(rest) {
            if !tag.starts_with("<!--") {
                out.push_str(&safe_tag(tag)?);
            }
            rest = after;
            continue;
        }
        let mut chars = rest.chars();
        match chars.next() {
            None => return Some(out),
            Some('<') => out.push_str("&lt;"),
            Some(ch) => out.push(ch),
        }
        rest = chars.as_str();
    }
}

/// The tag `text` opens with, and what follows it.
fn split_tag(text: &str) -> Option<(&str, &str)> {
    if !text.starts_with('<') {
        return None;
    }
    let tag = html_tag(text)?;
    Some((tag, text.strip_prefix(tag)?))
}
