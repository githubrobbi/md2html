// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html`: one Markdown file to one self-contained HTML page.
//!
//! Forked from iv's `iv-handbook` (same owner, MIT OR Apache-2.0), which
//! itself came from docenta's `handbook-html`. Standard library only: no
//! Markdown crate, no network, nothing to vendor, so it builds anywhere
//! a Rust toolchain does (macOS, Ubuntu).
//!
//! It renders a deliberate subset - headings, paragraphs, tables, fenced
//! code, lists, rules, links, bold, italic and inline code. A construct it
//! does not know appears verbatim, which is visible rather than silent.
//! Known gaps: block quotes inside list items, and bold-italic.
//!
//! A chapter called "Glossary" (a table of term | meaning) becomes live:
//! every term's first mention per chapter is a dotted link that explains
//! itself on hover and jumps to the entry.
//!
//! A shared glossary joins every page: `~/.config/md2html/glossary.md`
//! (or `MD2HTML_GLOSSARY`), a Markdown page with its own "Glossary"
//! chapter. Its terms link in the prose like the page's own; the page
//! lists only the shared terms it uses, each with a way to the full
//! list (the shared page's HTML twin, or `MD2HTML_GLOSSARY_URL`). A term
//! the page defines itself wins over the shared one. `--no-glossary`
//! renders without it.
//!
//! Usage: `md2html <in.md> [out.html] [--large] [--no-glossary]`. Without
//! `out.html` the page is written beside the Markdown with the extension
//! swapped. `--large` adds the larger type used for cards read at half a
//! screen.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use std::path::PathBuf;

mod markdown;
mod page;

use markdown::{Heading, Term, callout_kind, cells, esc, glossary, heading, inline, list_item};
use page::{FIGURE_MARK, Meta, page};

fn main() {
    let mut large = false;
    let mut no_glossary = false;
    let mut paths: Vec<PathBuf> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--large" => large = true,
            "--no-glossary" => no_glossary = true,
            "-h" | "--help" => {
                println!("usage: md2html <in.md> [out.html] [--large] [--no-glossary]");
                return;
            }
            "-V" | "--version" => {
                println!("md2html {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let Some(source) = paths.first().cloned() else {
        eprintln!("usage: md2html <in.md> [out.html] [--large] [--no-glossary]");
        std::process::exit(64);
    };
    if paths.len() > 2 {
        eprintln!(
            "md2html: one page per run; the second path is the output, so a third is a mistake"
        );
        std::process::exit(64);
    }
    let out = paths
        .get(1)
        .cloned()
        .unwrap_or_else(|| source.with_extension("html"));
    if let Some(reason) = unsafe_output(&source, &out) {
        eprintln!("md2html: refusing to write {}: {reason}", out.display());
        std::process::exit(64);
    }
    let text = std::fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("reading {}: {error}", source.display()));
    let own = glossary(&text);
    let shared = if no_glossary {
        None
    } else {
        shared_glossary(&source)
    };
    let glossary_url = shared.as_ref().map(|(path, _terms)| glossary_url(path));
    let terms = join(own, shared.map(|(_path, terms)| terms).unwrap_or_default());
    let shared_count = terms.iter().filter(|term| term.shared).count();
    let name = source
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let title = text
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map_or_else(|| name.clone(), |title| title.trim().replace('`', ""));
    let meta = Meta {
        title,
        source: name,
        large,
        glossary_url,
    };
    let html = render(&text, &terms, &meta);
    std::fs::write(&out, &html)
        .unwrap_or_else(|error| panic!("writing {}: {error}", out.display()));
    println!(
        "md2html: {} -> {} ({} bytes, {} glossary terms, {} of them shared)",
        source.display(),
        out.display(),
        html.len(),
        terms.len(),
        shared_count
    );
}

/// Why `out` must not be written, if it must not: it is the source
/// itself, or it is Markdown (`md2html a.md b.md` once overwrote b.md).
fn unsafe_output(source: &std::path::Path, out: &std::path::Path) -> Option<&'static str> {
    let same = match (std::fs::canonicalize(source), std::fs::canonicalize(out)) {
        (Ok(a), Ok(b)) => a == b,
        _ => source == out,
    };
    if same {
        return Some("it is the input file");
    }
    let markdown = out
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["md", "markdown"]
                .iter()
                .any(|md| ext.eq_ignore_ascii_case(md))
        });
    markdown.then_some("the output would be a Markdown file; run md2html once per page")
}

/// The shared glossary's terms and where they came from: the file named
/// by `MD2HTML_GLOSSARY`, else `~/.config/md2html/glossary.md`. None when
/// there is no such file, or when the page being rendered IS that file
/// (its own chapter already covers it).
fn shared_glossary(source: &PathBuf) -> Option<(PathBuf, Vec<Term>)> {
    let path = std::env::var_os("MD2HTML_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/md2html/glossary.md"))
        })?;
    let text = std::fs::read_to_string(&path).ok()?;
    let same = match (std::fs::canonicalize(&path), std::fs::canonicalize(source)) {
        (Ok(shared), Ok(own)) => shared == own,
        _ => false,
    };
    if same {
        return None;
    }
    let terms = glossary(&text)
        .into_iter()
        .map(|term| Term {
            shared: true,
            ..term
        })
        .collect();
    Some((path, terms))
}

/// Where a page sends the reader for the full shared glossary:
/// `MD2HTML_GLOSSARY_URL` if set, else the shared file's HTML twin as a
/// `file:` URL.
fn glossary_url(path: &PathBuf) -> String {
    if let Some(url) = std::env::var_os("MD2HTML_GLOSSARY_URL") {
        return url.to_string_lossy().into_owned();
    }
    let twin = std::fs::canonicalize(path)
        .unwrap_or_else(|_error| path.clone())
        .with_extension("html");
    format!("file://{}", twin.display())
}

/// The page's own terms, then the shared ones it does not define itself,
/// longest word first as the matcher wants them.
fn join(own: Vec<Term>, shared: Vec<Term>) -> Vec<Term> {
    let mut terms = own;
    for term in shared {
        let taken = terms
            .iter()
            .any(|have| have.word.eq_ignore_ascii_case(&term.word));
        if !taken {
            terms.push(term);
        }
    }
    terms.sort_by_key(|term| core::cmp::Reverse(term.word.len()));
    terms
}

/// One searchable section: where it is, what it is called, which
/// chapter holds it, and its words.
pub(crate) struct Section {
    /// The anchor.
    pub(crate) id: String,
    /// The heading.
    pub(crate) title: String,
    /// The chapter's heading.
    pub(crate) chapter: String,
    /// The plain words, tags stripped.
    pub(crate) text: String,
}

/// A chapter's number and anchor, from a pass over the headings before
/// rendering, so the reading guide can point at chapters it names.
pub(crate) fn chapters(text: &str) -> Vec<(u32, String)> {
    text.lines()
        .filter_map(heading)
        .filter(|title| title.level == 2)
        .filter_map(|title| {
            let number = title
                .text
                .split_once('.')
                .and_then(|(head, _rest)| head.trim().parse::<u32>().ok())?;
            Some((number, title.id))
        })
        .collect()
}

/// Rendered HTML back to words, for the search index.
///
/// A tag that ends a cell, a row or an item ends a word too: without the
/// space, the last word of one table cell ran into the first of the next
/// ("below> Note"), in the snippet and for the matcher.
pub(crate) fn plain(html: &str) -> String {
    const BREAKS: [&str; 7] = ["/td", "/th", "/tr", "/li", "/p", "/dd", "/dt"];
    let mut out = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for ch in html.chars() {
        match (ch, tag.as_mut()) {
            ('<', None) => tag = Some(String::new()),
            ('>', Some(name)) => {
                if BREAKS.contains(&name.as_str()) {
                    out.push(' ');
                }
                tag = None;
            }
            (other, Some(name)) => name.push(other),
            (other, None) => out.push(other),
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Today as "19 September 2026", from the clock alone (the civil
/// date from days since the epoch, the algorithm every calendar
/// library uses).
pub(crate) fn today() -> String {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let shifted = days.saturating_add(719_468);
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    let name = usize::try_from(month - 1)
        .ok()
        .and_then(|index| NAMES.get(index))
        .copied()
        .unwrap_or("");
    format!("{day} {name} {year}")
}

/// The anchor of the page's opening: its header, where a search hit in the
/// title, the lead or anything else before the first chapter lands.
pub(crate) const OPENING: &str = "opening";

/// The page being built: what is written, what is still open, which
/// terms have been linked, and the search index growing beside it.
#[derive(Default)]
pub(crate) struct Body {
    /// The HTML so far.
    pub(crate) out: String,
    /// The contents rail, nested one level.
    pub(crate) contents: String,
    /// Whether a chapter's sub-list is open in the rail.
    sub_open: bool,
    /// The paragraph still being gathered.
    paragraph: String,
    /// The list still open, if any.
    list: Option<&'static str>,
    /// The callout still open: its kind.
    callout: Option<String>,
    /// Terms already linked in THIS chapter.
    seen: BTreeMap<String, usize>,
    /// Every term mention in the book, for unique anchors.
    pub(crate) minted: BTreeMap<String, usize>,
    /// The searchable sections.
    pub(crate) sections: Vec<Section>,
    /// The chapter being rendered.
    pub(crate) chapter: String,
    /// Inside "How to read this": its list becomes cards.
    guide: bool,
    /// The guide bullet still being gathered, continuation lines and all.
    guide_item: String,
    /// The chapter anchors, by number, for the guide's cards.
    numbered: Vec<(u32, String)>,
    /// The first paragraph after the title is the lead.
    lead_pending: bool,
}

impl Body {
    /// Closes whatever block is open.
    fn flush(&mut self, terms: &[Term]) {
        if !self.guide_item.is_empty() {
            let item = self.guide_item.clone();
            self.guide_item.clear();
            self.guide_card(&item);
        }
        if self.guide && !self.paragraph.is_empty() {
            // The first paragraph after the guide's bullets ends the grid.
            self.end_guide();
        }
        if let Some(kind) = self.list.take() {
            let _written = write!(self.out, "</{kind}>");
        }
        if !self.paragraph.is_empty() {
            let rendered = self.inline(&self.paragraph.clone(), terms);
            self.remember(&rendered);
            if self.lead_pending {
                // The lead closes the hero the title opened.
                self.lead_pending = false;
                let _written = write!(self.out, "<p class=\"lead\">{rendered}</p></header>");
            } else {
                let _written = write!(self.out, "<p>{rendered}</p>");
            }
            self.paragraph.clear();
        }
        if self.callout.take().is_some() {
            self.out.push_str("</div></aside>");
        }
    }

    /// Closes the hero when something other than a paragraph follows the
    /// title: a callout, list, table, code block, rule or heading. Without
    /// this the lead was looked for inside that block and the hero's
    /// closing tag landed in it, which broke the page's grid.
    fn end_hero(&mut self) {
        if self.lead_pending {
            self.lead_pending = false;
            self.out.push_str("</header>");
        }
    }

    /// Closes the reading guide's card grid, once.
    fn end_guide(&mut self) {
        if self.guide {
            self.out.push_str("</div>");
            self.guide = false;
        }
    }

    /// Inline markup with this page's term bookkeeping.
    fn inline(&mut self, text: &str, terms: &[Term]) -> String {
        inline(text, terms, &mut self.seen, &mut self.minted)
    }

    /// Adds rendered words to the current section's search text.
    ///
    /// Words before the first chapter have a section too: the opening. They
    /// had none, so the lead - the paragraph most likely to be remembered
    /// and searched for - was the one part of a page the search could not
    /// find, and a page with no chapters at all had an empty index.
    fn remember(&mut self, rendered: &str) {
        if self.sections.is_empty() {
            self.open_with("Top of the page");
        }
        if let Some(section) = self.sections.last_mut() {
            section.text.push(' ');
            section.text.push_str(&plain(rendered));
        }
    }

    /// The opening's search section, under `title`: everything from the top
    /// of the page to its first chapter. It lands on the page's header.
    fn open_with(&mut self, title: &str) {
        self.sections.push(Section {
            id: String::from(OPENING),
            title: title.to_owned(),
            chapter: String::new(),
            text: String::new(),
        });
    }

    /// A heading: into the article with its anchor, into the rail,
    /// and a new section for the search index.
    fn heading(&mut self, title: &Heading<'_>) {
        let level = title.level;
        if level > 1 {
            self.end_hero();
        }
        let bare = title.text.replace('`', "");
        let text = esc(&bare);
        let shown = inline(title.text, &[], &mut BTreeMap::new(), &mut BTreeMap::new());
        if level == 2 {
            self.seen.clear();
            title.text.clone_into(&mut self.chapter);
            if self.sub_open {
                self.contents.push_str("</ol></li>");
            }
            let _written = write!(
                self.contents,
                "<li><a href=\"#{}\">{text}</a><ol class=\"sub\">",
                title.id
            );
            self.sub_open = true;
        } else if level == 3 && self.sub_open {
            let _written = write!(
                self.contents,
                "<li><a href=\"#{}\">{text}</a></li>",
                title.id
            );
        }
        if level == 1 {
            let _written = write!(self.out, "<h1>{shown}</h1>");
            self.lead_pending = true;
            // The title names the opening, unless words before it already
            // opened one.
            if self.sections.is_empty() {
                self.open_with(&bare);
            }
            return;
        }
        if level <= 3 {
            self.sections.push(Section {
                id: title.id.clone(),
                title: bare,
                chapter: self.chapter.clone(),
                text: String::new(),
            });
        }
        let _written = write!(
            self.out,
            "<h{level} id=\"{}\">{shown}<a class=\"anchor\" href=\"#{}\" aria-label=\"Link to this section\">#</a></h{level}>",
            title.id, title.id
        );
    }

    /// A reading-guide bullet as a card: the bold lead names the
    /// chapters, the rest says what they are for. No glossary links inside:
    /// the card is a link, and a link inside a link is not HTML (the browser
    /// closed the card at the first one, 2026-10-08).
    fn guide_card(&mut self, item: &str) {
        let (lead, rest) = item
            .strip_prefix("**")
            .and_then(|tail| tail.split_once("**"))
            .unwrap_or(("", item));
        let first = lead
            .split(|ch: char| !ch.is_ascii_digit())
            .find_map(|piece| piece.parse::<u32>().ok());
        let href = first
            .and_then(|number| {
                self.numbered
                    .iter()
                    .find(|(candidate, _id)| *candidate == number)
                    .map(|(_number, id)| id.clone())
            })
            .unwrap_or_default();
        let said = self.inline(rest.trim(), &[]);
        // A card's words are the page's words: searchable like any other.
        self.remember(&esc(lead));
        self.remember(&said);
        let _written = write!(
            self.out,
            "<a class=\"card\" href=\"#{href}\"><span class=\"k\">{}</span><span>{said}</span></a>",
            esc(lead)
        );
    }
}

impl Body {
    /// A fence line opens or closes a code block; returns whether the
    /// line was one.
    fn fence(&mut self, line: &str, terms: &[Term], in_code: &mut bool) -> bool {
        let Some(rest) = line.trim_start().strip_prefix("```") else {
            return false;
        };
        if *in_code {
            self.out.push_str("</code></pre></div>");
        } else {
            self.flush(terms);
            self.end_guide();
            self.end_hero();
            let lang = rest.trim();
            let label = if lang.is_empty() {
                String::new()
            } else {
                format!("<span class=\"lang\">{}</span>", esc(lang))
            };
            let named = if lang.is_empty() { "" } else { " named" };
            let _written = write!(
                self.out,
                "<div class=\"code{named}\">{label}<button class=\"copy\" type=\"button\">copy</button><pre><code>"
            );
        }
        *in_code = !*in_code;
        true
    }

    /// A quoted line opens or continues a callout; returns whether the
    /// line was one.
    fn quote(&mut self, line: &str, terms: &[Term]) -> bool {
        let Some(quoted) = line.strip_prefix("> ") else {
            return false;
        };
        if self.callout.is_none() {
            self.flush(terms);
            self.end_hero();
            let (kind, said) = callout_kind(quoted);
            self.callout = Some(kind.to_owned());
            let _written = write!(
                self.out,
                "<aside class=\"callout {}\"><span class=\"mark\">{kind}</span><div class=\"body\">",
                kind.to_lowercase()
            );
            self.paragraph.push_str(said.trim());
        } else {
            if !self.paragraph.is_empty() {
                self.paragraph.push(' ');
            }
            self.paragraph.push_str(quoted.trim());
        }
        true
    }

    /// A list line opens or continues a list, or gathers a reading-guide
    /// card; returns whether the line was one.
    fn list(&mut self, line: &str, terms: &[Term]) -> bool {
        if let Some((kind, item)) = list_item(line) {
            if self.guide {
                self.flush(terms);
                item.trim().clone_into(&mut self.guide_item);
                return true;
            }
            if self.list != Some(kind) {
                self.flush(terms);
                self.end_hero();
                let _written = write!(self.out, "<{kind}>");
                self.list = Some(kind);
            }
            let rendered = self.inline(item.trim(), terms);
            self.remember(&rendered);
            let _written = write!(self.out, "<li>{rendered}</li>");
            return true;
        }
        if !self.guide_item.is_empty() && line.starts_with("  ") {
            self.guide_item.push(' ');
            self.guide_item.push_str(line.trim());
            return true;
        }
        if self.list.is_some() && line.starts_with("  ") {
            let rendered = self.inline(line.trim(), terms);
            self.remember(&rendered);
            let _written = write!(self.out, " {rendered}");
            return true;
        }
        false
    }

    /// An image on a line of its own, `![caption](path)`, as a figure
    /// with its caption under it; returns whether the line was one. The
    /// image is a link to its file, which the page's script
    /// (`page/figure.js`) turns into an enlargement over the page, so
    /// the reader does not leave their place. Without the script, or
    /// with a modifier key, the link opens the file itself.
    fn figure(&mut self, line: &str, terms: &[Term]) -> bool {
        let Some((caption, rest)) = line
            .trim()
            .strip_prefix("![")
            .and_then(|tail| tail.split_once("]("))
        else {
            return false;
        };
        let Some(src) = rest.strip_suffix(')') else {
            return false;
        };
        if src.contains(')') || src.contains('"') {
            return false;
        }
        self.flush(terms);
        self.end_guide();
        let said = self.inline(caption.trim(), terms);
        self.remember(&said);
        let src = esc(src.trim());
        let alt = esc(caption.trim());
        let _written = write!(
            self.out,
            "{FIGURE_MARK}<a href=\"{src}\" target=\"_blank\" rel=\"noopener\">\
<img src=\"{src}\" alt=\"{alt}\" loading=\"lazy\"></a><figcaption>{said}</figcaption></figure>"
        );
        true
    }

    /// A line of prose, joined into the open paragraph.
    fn prose(&mut self, line: &str) {
        if !self.paragraph.is_empty() {
            self.paragraph.push(' ');
        }
        self.paragraph.push_str(line.trim());
    }
}

/// The article, the rail and the index, from the text.
pub(crate) fn render(text: &str, terms: &[Term], meta: &Meta) -> String {
    let mut body = Body {
        numbered: chapters(text),
        ..Body::default()
    };
    let mut lines = text.lines().peekable();
    let mut in_code = false;
    let mut skipping = false;
    while let Some(line) = lines.next() {
        if line.starts_with("<!--") || line.starts_with("-->") || line.starts_with("SPDX-") {
            continue;
        }
        if body.fence(line, terms, &mut in_code) {
            continue;
        }
        if in_code {
            let code = line.strip_prefix("   ").unwrap_or(line);
            let _written = writeln!(body.out, "{}", esc(code));
            body.remember(&esc(code));
            continue;
        }
        if let Some(title) = heading(line) {
            body.flush(terms);
            body.end_guide();
            // The glossary is rendered live at the end, and the book's own
            // contents list is the rail: neither is repeated in the article.
            skipping = title.text.contains("Glossary") || title.text == "Contents";
            if !skipping {
                body.heading(&title);
                if title.level == 2 && title.text == "How to read this" {
                    body.guide = true;
                    body.out.push_str("<div class=\"cards\">");
                }
            }
            continue;
        }
        if skipping || body.quote(line, terms) {
            continue;
        }
        if line.trim().is_empty() {
            body.flush(terms);
            continue;
        }
        if line.trim() == "---" {
            body.flush(terms);
            body.end_guide();
            body.end_hero();
            body.out.push_str("<hr>");
            continue;
        }
        if line.starts_with('|') {
            body.flush(terms);
            body.end_guide();
            body.end_hero();
            let mut rows = vec![line];
            while lines.peek().is_some_and(|next| next.starts_with('|')) {
                if let Some(next) = lines.next() {
                    rows.push(next);
                }
            }
            let rendered = table(&rows, terms, &mut body);
            body.remember(&rendered);
            body.out.push_str(&rendered);
            continue;
        }
        if body.figure(line, terms) {
            continue;
        }
        if body.list(line, terms) || line.starts_with("Version of this handbook") {
            // The eyebrow carries the version and the date on the page.
            continue;
        }
        body.prose(line);
    }
    body.flush(terms);
    if in_code {
        body.out.push_str("</code></pre></div>");
    }
    if body.sub_open {
        body.contents.push_str("</ol></li>");
    }
    page(&body, terms, meta)
}

/// One table, header row first, its rule dropped.
pub(crate) fn table(rows: &[&str], terms: &[Term], body: &mut Body) -> String {
    let mut out = String::from("<div class=\"scroll\"><table>");
    for (index, row) in rows.iter().enumerate() {
        if index == 1 && row.contains("---") {
            continue;
        }
        let tag = if index == 0 { "th" } else { "td" };
        out.push_str("<tr>");
        for cell in cells(row) {
            let rendered = body.inline(cell.trim(), terms);
            let _written = write!(out, "<{tag}>{rendered}</{tag}>");
        }
        out.push_str("</tr>");
    }
    out.push_str("</table></div>");
    out
}

#[cfg(test)]
mod tests {
    use super::{Meta, glossary, render};

    /// A page as the tool writes it, from Markdown, with no shared glossary.
    fn page(text: &str) -> String {
        let meta = Meta {
            title: String::from("T"),
            source: String::from("t.md"),
            large: false,
            glossary_url: None,
        };
        render(text, &glossary(text), &meta)
    }

    /// The search index of a page: the JSON the page's script reads.
    fn index(html: &str) -> &str {
        let open = "<script id=\"idx\" type=\"application/json\">";
        let start = html.find(open).map_or(0, |at| at + open.len());
        let rest = html.get(start..).unwrap_or_default();
        rest.split("</script>").next().unwrap_or_default()
    }

    /// The live fault: "folder" stood in the lead and nowhere else, and the
    /// search said nothing matched. Everything before the first chapter is
    /// in the index, under the title, and lands on the page's header.
    #[test]
    fn a_callout_after_the_title_closes_the_hero_first() {
        let html = page("# Title\n\n> **Note.** Read this first.\n\nThe text.\n\n## One\n\nBody.\n");
        let article = &html[html.find("<article").expect("article")..];
        let hero_end = article.find("</header>").expect("hero closed");
        let aside = article.find("<aside").expect("callout");
        assert!(hero_end < aside, "the hero must close before the callout opens");
        assert_eq!(article.matches("</header>").count(), 1, "one hero, closed once");
        let body = &article[..article.find("</article>").expect("article closed")];
        assert_eq!(body.matches("<aside").count(), body.matches("</aside>").count());
        assert_eq!(body.matches("<div").count(), body.matches("</div>").count(), "divs balance inside the article");
    }

    /// Every block element closes in the order it opened: a closing tag
    /// that does not match the innermost open one would make the browser
    /// close containers early and spill the text out of the column.
    fn assert_nested(html: &str, case: &str) {
        const BLOCKS: [&str; 10] = ["article", "header", "aside", "div", "ul", "ol", "table", "pre", "nav", "section"];
        let mut open: Vec<&str> = Vec::new();
        let mut rest = html;
        while let Some(at) = rest.find('<') {
            rest = &rest[at + 1..];
            let closing = rest.starts_with('/');
            let name_start = usize::from(closing);
            let end = rest[name_start..]
                .find(|c: char| !c.is_ascii_alphanumeric())
                .map_or(rest.len(), |e| e + name_start);
            let name = &rest[name_start..end];
            if !BLOCKS.contains(&name) {
                continue;
            }
            if closing {
                assert_eq!(open.pop(), Some(name), "</{name}> closes out of order after {case:?}");
            } else {
                open.push(name);
            }
        }
        assert!(open.is_empty(), "left open {open:?} after {case:?}");
    }

    #[test]
    fn whatever_follows_the_title_the_page_structure_balances() {
        let openings = [
            "Plain lead paragraph.",
            "> **Note.** A callout first.",
            "- a list first\n- second item",
            "1. a numbered list first",
            "| a | b |\n|---|---|\n| 1 | 2 |",
            "```\ncode first\n```",
            "---",
            "## A chapter straight away",
        ];
        for opening in openings {
            let html = page(&format!("# Title\n\n{opening}\n\nThe text.\n\n## One\n\nBody.\n"));
            let article = &html[html.find("<article").expect("article")..];
            let body = &article[..article.find("</article>").expect("article closed")];
            assert_eq!(body.matches("</header>").count(), 1, "hero closed once after {opening:?}");
            let whole = &article[..body.len() + "</article>".len()];
            assert_nested(whole, opening);
        }
    }

    #[test]
    fn the_lead_is_searchable() {
        let html = page(
            "# Title here\n\nOpened straight from a folder.\n\nMore opening words.\n\n## One\n\nBody.\n",
        );
        let index = index(&html);
        assert!(index.contains("folder"), "{index}");
        assert!(index.contains("More opening words"), "{index}");
        assert!(
            index.contains("{\"id\":\"opening\",\"t\":\"Title here\""),
            "{index}"
        );
        assert!(html.contains("<header class=\"hero\" id=\"opening\">"));
    }

    /// A page with no chapters at all, and one with no title, still have
    /// something to search.
    #[test]
    fn a_page_without_chapters_is_searchable() {
        let untitled = page("Just a note about pelicans.\n\n- and a list\n");
        let index = index(&untitled);
        assert!(index.contains("pelicans"), "{index}");
        assert!(index.contains("and a list"), "{index}");
        assert!(index.contains("\"t\":\"Top of the page\""), "{index}");
    }

    /// Words in neighbouring table cells stay two words in the index.
    #[test]
    fn table_cells_do_not_run_together() {
        let html =
            page("# T\n\nLead.\n\n## One\n\n| A | B |\n|---|---|\n| caption below | Note here |\n");
        let index = index(&html);
        assert!(index.contains("caption below Note here"), "{index}");
        assert_eq!(super::plain("<b>bo</b>ld <td>x</td><td>y</td>"), "bold x y");
    }

    /// The reading guide's cards are words on the page like any other.
    #[test]
    fn a_reading_guide_card_is_searchable() {
        let html = page(
            "# T\n\nLead.\n\n## How to read this\n\n- **Chapter 1** takes two minutes.\n\n## 1. First\n\nBody.\n",
        );
        let index = index(&html);
        assert!(index.contains("takes two minutes"), "{index}");
        assert!(html.contains("class=\"card\" href=\"#1-first\""));
    }

    /// A figure brings its enlargement with it; a page without one carries
    /// none of it. The image stays a plain link to its file, which is what
    /// a reader without the script, or with a modifier key, gets.
    #[test]
    fn only_a_page_with_a_figure_carries_the_enlargement() {
        let with = page("# T\n\n## One\n\n![A picture](docs/a.png)\n");
        assert!(with.contains("<a href=\"docs/a.png\" target=\"_blank\""));
        assert!(with.contains("dialog.zoom"));
        assert!(with.contains("figure.fig>a"));
        let without = page("# T\n\n## One\n\nNo picture, only prose.\n");
        assert!(!without.contains("dialog.zoom"));
        assert!(!without.contains("figure.fig"));
    }
}
