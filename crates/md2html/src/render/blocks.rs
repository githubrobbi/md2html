// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! One block of the article, wherever it stands: paragraphs, code and
//! diagrams, raw HTML, quotes and callouts, lists, tables, maths.

use core::iter;

use super::nodes::Links;
use super::{Diagrams, Renderer};
use crate::inline::HtmlMode;
use crate::model::{Align, Block, CodeBlock, List, ListKind, MathMode, Paragraph, Table, Task};
use crate::sanitize::{html_tag, safe_tag};
use crate::text::{esc, put, slug};

impl Renderer<'_, '_> {
    /// One block, anywhere: at the top, in a list, in a quote, in a note.
    pub(super) fn block(&mut self, block: &Block, tight: bool) -> String {
        if let Some(line) = block.line() {
            self.line = line;
        }
        match block {
            Block::Paragraph(paragraph) => self.paragraph(paragraph, tight),
            Block::Heading(heading) => {
                let nodes = self.parse(&heading.text);
                let shown = crate::inline::plain(&nodes);
                let wanted = heading.id.clone().unwrap_or_else(|| slug(&shown));
                let id = self.unique(&wanted);
                let html = self.inline(&nodes, Links::Own);
                format!(
                    "<h{level} id=\"{id}\">{html}</h{level}>",
                    level = heading.level
                )
            }
            Block::Code(code) => self.code(code),
            Block::Html(html) => self.html_block(&html.text),
            Block::Rule => String::from("<hr>"),
            Block::Quote(quote) => self.quote(&quote.blocks),
            Block::List(list) => self.list(list),
            Block::Table(table) => self.table(table),
            Block::Math(math) => {
                format!(
                    "<div class=\"math\">{}</div>",
                    math_html(&math.tex, MathMode::Display)
                )
            }
        }
    }

    /// A paragraph; an image alone in one is a figure. A tight list's
    /// paragraphs are not wrapped.
    fn paragraph(&mut self, paragraph: &Paragraph, tight: bool) -> String {
        if paragraph.text.starts_with("SPDX-") {
            return String::new();
        }
        let nodes = self.parse(&paragraph.text);
        if let Some(image) = super::figure::lone_image(&nodes) {
            return self.figure(image);
        }
        let html = self.inline(&nodes, Links::Glossary);
        if html.contains("[^") {
            self.warn("a footnote reference [^...] has no definition; it is shown as written");
        }
        if tight {
            html
        } else {
            format!("<p>{html}</p>")
        }
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

    /// Raw HTML: as written when trusted; otherwise only the safe tags pass,
    /// and a block holding any other is shown as text.
    fn html_block(&mut self, text: &str) -> String {
        match self.inp.html {
            HtmlMode::Spec => format!("{text}\n"),
            HtmlMode::Rich => format!("{}\n", crate::sanitize::rich_block(text)),
            HtmlMode::Safe => {
                let trimmed = text.trim();
                if trimmed.starts_with("<!--")
                    && trimmed.ends_with("-->")
                    && trimmed.matches("-->").count() == 1
                {
                    return String::new();
                }
                sanitize(text).unwrap_or_else(|| {
                    self.warn("raw HTML is shown as written; render with --html to let it through");
                    format!("<pre class=\"verbatim\">{}</pre>", esc(text))
                })
            }
        }
    }

    /// A block quote: a callout when its first words name one, a quote
    /// otherwise.
    fn quote(&mut self, blocks: &[Block]) -> String {
        if let Some(callout) = self.callout(blocks) {
            return callout;
        }
        let mut inner = String::new();
        for block in blocks {
            inner.push_str(&self.block(block, false));
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
        if bold_lead {
            return format!(
                "<div class=\"callout note\" role=\"note\"><span class=\"mark\">{}</span><div class=\"body\">{inner}</div></div>",
                esc(self.inp.ui.callouts.note)
            );
        }
        format!("<blockquote>{inner}</blockquote>")
    }

    /// A list; a tight one's paragraphs are not wrapped.
    fn list(&mut self, list: &List) -> String {
        let tasks = list.items.iter().any(|item| item.task.is_some());
        let mut out = match list.kind {
            ListKind::Ordered(1) => String::from("<ol>"),
            ListKind::Ordered(start) => format!("<ol start=\"{start}\">"),
            ListKind::Bullet if tasks => String::from("<ul class=\"tasks\">"),
            ListKind::Bullet => String::from("<ul>"),
        };
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
            for (at, block) in item.blocks.iter().enumerate() {
                if at > 0 && list.tight {
                    out.push('\n');
                }
                out.push_str(&self.block(block, list.tight));
            }
            out.push_str("</li>");
        }
        out.push_str(match list.kind {
            ListKind::Ordered(_) => "</ol>",
            ListKind::Bullet => "</ul>",
        });
        out
    }

    /// A table in its scrolling frame.
    fn table(&mut self, table: &Table) -> String {
        let mut out = String::from("<div class=\"scroll\"><table><thead><tr>");
        for (cell, align) in table.head.iter().zip(columns(&table.align)) {
            let nodes = self.parse(cell);
            let html = self.inline(&nodes, Links::Glossary);
            if html.is_empty() {
                out.push_str("<td></td>");
            } else {
                put!(out, "<th scope=\"col\"{}>{html}</th>", class(align));
            }
        }
        out.push_str("</tr></thead><tbody>");
        // Each row has an address of its own, for a link to exactly it: its
        // section and its first cell (`7-settings-lang`), so it holds when
        // rows or tables are added elsewhere.
        let section = self.section_id().unwrap_or("row").to_owned();
        for (number, row) in (1_usize..).zip(&table.rows) {
            let first = row
                .first()
                .map(|cell| crate::inline::plain(&self.parse(cell)))
                .unwrap_or_default();
            let mut key: String = slug(&first).chars().take(40).collect();
            if key.is_empty() {
                key = format!("r{number}");
            }
            let id = self.unique(&format!("{section}-{key}"));
            put!(out, "<tr id=\"{}\">", esc(&id));
            for (cell, align) in row.iter().zip(columns(&table.align)) {
                let nodes = self.parse(cell);
                let html = self.inline(&nodes, Links::Glossary);
                put!(out, "<td{}>{html}</td>", class(align));
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table></div>");
        out
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

/// Maths as `MathML`, or the TeX itself when this tool cannot read it.
pub(super) fn math_html(tex: &str, mode: MathMode) -> String {
    match (crate::math::mathml(tex, mode), mode) {
        (Some(mathml), _) => mathml,
        (None, MathMode::Display) => format!("<pre class=\"tex\">{}</pre>", esc(tex)),
        (None, MathMode::Inline) => format!("<code class=\"tex\">{}</code>", esc(tex)),
    }
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
