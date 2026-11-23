// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::cognitive_complexity,
    clippy::indexing_slicing,
    clippy::min_ident_chars,
    clippy::shadow_reuse,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]

//! The block structure of a document, the way `CommonMark` reads it:
//! containers (block quotes, list items, footnotes) hold lines that are
//! parsed again on their own, so a list nests, a quote holds a list and
//! a list item holds code. Leaves keep their inline text raw; the inline
//! pass (`inline.rs`) runs later, once every link reference and footnote
//! is known. GitHub's additions come along: tables, task items,
//! footnotes; and `$$` maths blocks.
//!
//! Exception: the `CommonMark` block parser is one state machine over the
//! container stack; its rules are read together
//! (`scripts/ci/file_size_exceptions.txt`).

use alloc::collections::BTreeMap;

/// A table column's alignment, from its delimiter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    /// `---`
    None,
    /// `:--`
    Left,
    /// `:-:`
    Center,
    /// `--:`
    Right,
}

/// One block of the document.
#[derive(Debug)]
pub(crate) enum Block {
    /// `#` to `######`, or a paragraph underlined with `===` / `---`.
    Heading {
        /// One to six.
        level: u8,
        /// The raw inline text.
        text: String,
        /// An explicit `{#id}`.
        id: Option<String>,
        /// The source line.
        line: usize,
        /// Underlined rather than hashed.
        setext: bool,
    },
    /// Running text.
    Paragraph {
        /// The raw inline text, lines joined by `\n`.
        text: String,
        /// The source line.
        line: usize,
    },
    /// Fenced or indented code.
    Code {
        /// The info string after the fence.
        info: String,
        /// The code, each line ending in `\n`.
        text: String,
        /// The source line.
        line: usize,
    },
    /// Raw HTML, as written.
    Html {
        /// The lines.
        text: String,
        /// The source line.
        line: usize,
    },
    /// `---`, `***` or `___`.
    Rule,
    /// `>` lines.
    Quote {
        /// What the quote holds.
        blocks: Vec<Self>,
        /// The source line.
        line: usize,
    },
    /// Bulleted or numbered items.
    List {
        /// Numbered.
        ordered: bool,
        /// The first number.
        start: u64,
        /// No blank line between the items: their paragraphs are not wrapped.
        tight: bool,
        /// The items.
        items: Vec<Item>,
    },
    /// A pipe table with a delimiter row.
    Table {
        /// One per column.
        align: Vec<Align>,
        /// The header cells, raw.
        head: Vec<String>,
        /// The body rows, raw, as many cells as the header.
        rows: Vec<Vec<String>>,
        /// The source line.
        line: usize,
    },
    /// A `$$` block.
    Math {
        /// The TeX.
        text: String,
        /// The source line.
        line: usize,
    },
}

/// One list item.
#[derive(Debug)]
pub(crate) struct Item {
    /// `[ ]` or `[x]` at its start: a task, done or not.
    pub(crate) task: Option<bool>,
    /// What the item holds.
    pub(crate) blocks: Vec<Block>,
}

/// What the document defines for later use: link references and
/// footnotes, by normalized label.
#[derive(Default, Debug)]
pub(crate) struct Defs {
    /// `[label]: destination "title"`.
    pub(crate) links: BTreeMap<String, (String, Option<String>)>,
    /// `[^label]: text`.
    pub(crate) notes: BTreeMap<String, Vec<Block>>,
}

/// A line of the source with its number.
#[derive(Clone, Debug)]
struct Line {
    /// One-based.
    no: usize,
    /// The text, tabs in the indentation expanded.
    text: String,
}

/// The blocks of `text` and what it defines. `first_line` is the number
/// of its first line in the file (front matter shifts it).
///
/// `strict` reads `---` under a paragraph as `CommonMark` does, always a
/// heading. Otherwise only a single line that does not end like a sentence
/// becomes one; a paragraph with a rule straight under it, which is what
/// authors mean far more often, keeps its rule.
pub(crate) fn parse(text: &str, first_line: usize, math: bool, strict: bool) -> (Vec<Block>, Defs) {
    let lines: Vec<Line> = text
        .split('\n')
        .enumerate()
        .map(|(at, raw)| Line {
            no: at.saturating_add(first_line),
            text: expand_tabs(raw.strip_suffix('\r').unwrap_or(raw)).replace('\0', "\u{FFFD}"),
        })
        .collect();
    let mut defs = Defs::default();
    let blocks = Parser {
        defs: &mut defs,
        math,
        depth: 0,
        strict,
    }
    .blocks(&lines);
    (blocks, defs)
}

/// Tabs in a line's indentation as spaces to the next stop of four.
fn expand_tabs(line: &str) -> String {
    if !line.contains('\t') {
        return line.to_owned();
    }
    let mut out = String::with_capacity(line.len().saturating_add(8));
    let mut col = 0_usize;
    let mut leading = true;
    for ch in line.chars() {
        if leading && ch == '\t' {
            let fill = 4 - col % 4;
            for _ in 0..fill {
                out.push(' ');
            }
            col = col.saturating_add(fill);
            continue;
        }
        if leading && !matches!(ch, ' ' | '>') {
            leading = false;
        }
        out.push(ch);
        col = col.saturating_add(1);
    }
    out
}

/// Whether a line holds nothing but spaces.
pub(crate) fn is_blank(line: &str) -> bool {
    line.chars().all(|ch| ch == ' ' || ch == '\t')
}

/// The number of leading spaces.
pub(crate) fn indent(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

/// The line with up to `cols` leading spaces removed.
fn strip_cols(line: &str, cols: usize) -> String {
    let n = indent(line).min(cols);
    line.get(n..).unwrap_or_default().to_owned()
}

/// `---`, `***`, `___`: three or more of one mark, spaces between allowed.
pub(crate) fn is_rule(t: &str) -> bool {
    let mut mark = None;
    let mut count = 0_usize;
    for ch in t.trim_end().chars() {
        match ch {
            ' ' | '\t' => {}
            '-' | '*' | '_' => {
                if mark.is_some_and(|m| m != ch) {
                    return false;
                }
                mark = Some(ch);
                count = count.saturating_add(1);
            }
            _ => return false,
        }
    }
    count >= 3
}

/// An ATX heading: level, text, explicit id.
fn atx(t: &str) -> Option<(u8, String, Option<String>)> {
    let level = t.bytes().take_while(|byte| *byte == b'#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = t.get(level..)?;
    if !(rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    let mut text = rest.trim();
    // The closing hashes, when a space stands before them.
    let bare = text.trim_end_matches('#');
    if bare.len() < text.len() && (bare.is_empty() || bare.ends_with(' ') || bare.ends_with('\t')) {
        text = bare.trim_end();
    }
    let (text, id) = heading_id(text);
    Some((u8::try_from(level).unwrap_or(6), text, id))
}

/// A trailing `{#id}` on a heading, as pandoc and kramdown write it.
fn heading_id(text: &str) -> (String, Option<String>) {
    if let Some(head) = text.strip_suffix('}') {
        if let Some(at) = head.rfind("{#") {
            let id = head.get(at.saturating_add(2)..).unwrap_or_default();
            let before = head.get(..at).unwrap_or_default();
            if !id.is_empty()
                && id
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
                && (before.is_empty() || before.ends_with(' '))
            {
                return (before.trim_end().to_owned(), Some(id.to_owned()));
            }
        }
    }
    (text.to_owned(), None)
}

/// A code fence: its mark, its length, its info string.
fn fence_open(t: &str) -> Option<(char, usize, String)> {
    let mark = t.chars().next().filter(|ch| *ch == '`' || *ch == '~')?;
    let n = t.chars().take_while(|ch| *ch == mark).count();
    if n < 3 {
        return None;
    }
    let info = t.get(n..).unwrap_or_default().trim();
    if mark == '`' && info.contains('`') {
        return None;
    }
    Some((mark, n, crate::inline::unescape(info)))
}

/// Whether `t` closes a fence of `mark` and length `n`.
fn fence_close(line: &str, mark: char, n: usize) -> bool {
    let ind = indent(line);
    if ind > 3 {
        return false;
    }
    let t = line.get(ind..).unwrap_or_default();
    let run = t.chars().take_while(|ch| *ch == mark).count();
    run >= n && is_blank(t.get(run..).unwrap_or_default())
}

/// The tags whose blocks end at a blank line (`CommonMark`'s sixth kind).
const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

/// Which kind of HTML block `t` opens, if any (`CommonMark`'s seven).
fn html_start(t: &str, in_paragraph: bool) -> Option<u8> {
    if !t.starts_with('<') {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    for tag in ["script", "pre", "style", "textarea"] {
        if let Some(rest) = lower.strip_prefix('<').and_then(|r| r.strip_prefix(tag)) {
            if rest.is_empty() || rest.starts_with([' ', '\t', '>']) {
                return Some(1);
            }
        }
    }
    if lower.starts_with("<!--") {
        return Some(2);
    }
    if lower.starts_with("<?") {
        return Some(3);
    }
    if lower.starts_with("<![cdata[") {
        return Some(5);
    }
    if lower
        .strip_prefix("<!")
        .and_then(|r| r.chars().next())
        .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        return Some(4);
    }
    let name_at = if lower.starts_with("</") { 2 } else { 1 };
    let name: String = lower
        .get(name_at..)
        .unwrap_or_default()
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    let after = lower
        .get(name_at.saturating_add(name.len())..)
        .unwrap_or_default();
    if BLOCK_TAGS.contains(&name.as_str())
        && (after.is_empty() || after.starts_with([' ', '\t', '>']) || after.starts_with("/>"))
    {
        return Some(6);
    }
    if !in_paragraph
        && !name.is_empty()
        && !["script", "style", "pre", "textarea"].contains(&name.as_str())
    {
        if let Some(len) = crate::inline::html_tag(t) {
            let tag = t.get(..len).unwrap_or_default();
            let is_tag = !tag.starts_with("<!") && !tag.starts_with("<?");
            if is_tag && is_blank(t.get(len..).unwrap_or_default()) {
                return Some(7);
            }
        }
    }
    None
}

/// Whether a line of kind `kind` ends the HTML block.
fn html_end(kind: u8, line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    match kind {
        1 => ["</script>", "</pre>", "</style>", "</textarea>"]
            .iter()
            .any(|end| lower.contains(end)),
        2 => line.contains("-->"),
        3 => line.contains("?>"),
        4 => line.contains('>'),
        5 => line.contains("]]>"),
        _ => false,
    }
}

/// A list marker at the start of a line.
#[derive(Clone, Copy, Debug)]
struct Marker {
    /// Numbered.
    ordered: bool,
    /// `-`, `+`, `*`, or the delimiter `.` / `)` of a number.
    mark: char,
    /// The number of a numbered item.
    start: u64,
    /// The column the item's content starts at.
    col: usize,
    /// Nothing after the marker on its line.
    empty: bool,
}

/// The list marker a line opens with, if it opens with one.
fn list_marker(line: &str) -> Option<Marker> {
    let ind = indent(line);
    if ind > 3 {
        return None;
    }
    let t = line.get(ind..)?;
    let (ordered, mark, start, len) = match t.chars().next()? {
        ch @ ('-' | '+' | '*') => (false, ch, 0, 1),
        '0'..='9' => {
            let digits = t.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 9 {
                return None;
            }
            let delim = t.get(digits..)?.chars().next()?;
            if delim != '.' && delim != ')' {
                return None;
            }
            let start = t.get(..digits)?.parse::<u64>().ok()?;
            (true, delim, start, digits.saturating_add(1))
        }
        _ => return None,
    };
    let rest = t.get(len..)?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let empty = is_blank(rest);
    let spaces = indent(rest);
    let col = if empty || spaces >= 5 {
        ind.saturating_add(len).saturating_add(1)
    } else {
        ind.saturating_add(len).saturating_add(spaces)
    };
    Some(Marker {
        ordered,
        mark,
        start,
        col,
        empty,
    })
}

/// Whether a line, met while a paragraph is open, ends it by starting a
/// block of its own.
fn interrupts(line: &str, math: bool) -> bool {
    let ind = indent(line);
    if ind > 3 || is_blank(line) {
        return false;
    }
    let t = line.get(ind..).unwrap_or_default();
    if atx(t).is_some()
        || fence_open(t).is_some()
        || t.starts_with('>')
        || is_rule(t)
        || html_start(t, true).is_some()
        || (math && t.starts_with("$$"))
    {
        return true;
    }
    list_marker(line).is_some_and(|marker| !marker.empty && (!marker.ordered || marker.start == 1))
}

/// A table's delimiter row: its alignments.
fn delimiter_row(line: &str) -> Option<Vec<Align>> {
    let t = line.trim();
    if !t.contains('-') || indent(line) > 3 {
        return None;
    }
    let inner = t.strip_prefix('|').unwrap_or(t);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut out = Vec::new();
    for cell in inner.split('|') {
        let cell = cell.trim();
        let left = cell.starts_with(':');
        let right = cell.ends_with(':') && cell.len() > 1;
        let dashes = cell.trim_start_matches(':').trim_end_matches(':');
        if dashes.is_empty() || !dashes.chars().all(|ch| ch == '-') {
            return None;
        }
        out.push(match (left, right) {
            (true, true) => Align::Center,
            (true, false) => Align::Left,
            (false, true) => Align::Right,
            (false, false) => Align::None,
        });
    }
    if out.is_empty() || (!t.contains('|') && out.len() < 2) {
        return None;
    }
    Some(out)
}

/// One table row split on unescaped pipes. A `\|` stays in the cell as
/// a pipe, which is how a code span inside a cell writes one.
pub(crate) fn cells(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = if trimmed.ends_with('|') && !trimmed.ends_with("\\|") {
        trimmed.strip_suffix('|').unwrap_or(trimmed)
    } else {
        trimmed
    };
    let mut out = vec![String::new()];
    let mut escaped = false;
    for ch in trimmed.chars() {
        if escaped {
            if let Some(last) = out.last_mut() {
                if ch != '|' {
                    last.push('\\');
                }
                last.push(ch);
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '|' => out.push(String::new()),
            other => {
                if let Some(last) = out.last_mut() {
                    last.push(other);
                }
            }
        }
    }
    if escaped {
        if let Some(last) = out.last_mut() {
            last.push('\\');
        }
    }
    out.into_iter().map(|cell| cell.trim().to_owned()).collect()
}

/// Whether the lines at `at` open a table: a header row with pipes, then
/// a delimiter row with as many cells.
fn table_at(lines: &[Line], at: usize) -> Option<Vec<Align>> {
    let head = &lines.get(at)?.text;
    if !head.contains('|') || indent(head) > 3 {
        return None;
    }
    let align = delimiter_row(&lines.get(at.saturating_add(1))?.text)?;
    (cells(head).len() == align.len()).then_some(align)
}

/// A link reference definition, `[label]: destination "title"`, at the
/// start of a paragraph's text: the label, destination, title and how many
/// of the lines it used.
fn ref_def(lines: &[String]) -> Option<(String, String, Option<String>, usize)> {
    let first = lines.first()?;
    let rest = first.strip_prefix('[')?;
    let close = find_label_end(rest)?;
    let label = rest.get(..close)?;
    if label.trim().is_empty() || label.starts_with('^') {
        return None;
    }
    let after = rest.get(close.saturating_add(1)..)?.strip_prefix(':')?;
    let mut used = 1_usize;
    let mut tail = after.trim_start().to_owned();
    if tail.is_empty() {
        lines.get(1)?.trim().clone_into(&mut tail);
        used = 2;
    }
    let (dest, rest_len) = crate::inline::link_destination(&tail)?;
    let remainder = tail.get(rest_len..).unwrap_or_default();
    let title_src = remainder.trim();
    if title_src.is_empty() {
        // The title may stand on the next line.
        if let Some(next) = lines.get(used) {
            if let Some((title, len)) = crate::inline::link_title(next.trim()) {
                if next.trim().get(len..).is_some_and(is_blank) {
                    return Some((label.to_owned(), dest, Some(title), used.saturating_add(1)));
                }
            }
        }
        return Some((label.to_owned(), dest, None, used));
    }
    if !remainder.starts_with([' ', '\t']) {
        return None;
    }
    let (title, len) = crate::inline::link_title(title_src)?;
    if !is_blank(title_src.get(len..).unwrap_or_default()) {
        return None;
    }
    Some((label.to_owned(), dest, Some(title), used))
}

/// Where an unescaped `]` closes a label that began just before `text`.
fn find_label_end(text: &str) -> Option<usize> {
    let mut escaped = false;
    for (at, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' => return None,
            ']' => return Some(at),
            _ => {}
        }
    }
    None
}

/// A label as it is looked up: case folded, inner space collapsed.
pub(crate) fn normalize_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace('ß', "ss")
}

/// The block parser over one container's lines.
struct Parser<'d> {
    /// Where definitions go.
    defs: &'d mut Defs,
    /// `$$` blocks are maths.
    math: bool,
    /// Container depth, bounded so hostile nesting cannot overflow the stack.
    depth: usize,
    /// `CommonMark`'s reading of `---` under a paragraph.
    strict: bool,
}

impl Parser<'_> {
    /// The blocks of a run of lines.
    #[expect(
        clippy::many_single_char_names,
        reason = "geometry and scanning: x, y, w, h, i, j are the domain's names"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch over the kinds it handles, read top to bottom"
    )]
    fn blocks(&mut self, lines: &[Line]) -> Vec<Block> {
        let mut out = Vec::new();
        let mut i = 0_usize;
        let n = lines.len();
        while i < n {
            let line = &lines[i];
            let text = &line.text;
            if is_blank(text) {
                i += 1;
                continue;
            }
            let ind = indent(text);
            if ind >= 4 {
                let start = line.no;
                let mut code: Vec<String> = Vec::new();
                while i < n && (is_blank(&lines[i].text) || indent(&lines[i].text) >= 4) {
                    code.push(strip_cols(&lines[i].text, 4));
                    i += 1;
                }
                while code.last().is_some_and(|l| is_blank(l)) {
                    code.pop();
                }
                let mut body = code.join("\n");
                body.push('\n');
                out.push(Block::Code {
                    info: String::new(),
                    text: body,
                    line: start,
                });
                continue;
            }
            let t = text.get(ind..).unwrap_or_default();
            if let Some((mark, len, info)) = fence_open(t) {
                let start = line.no;
                let mut code = String::new();
                i += 1;
                while i < n {
                    if fence_close(&lines[i].text, mark, len) {
                        i += 1;
                        break;
                    }
                    code.push_str(&strip_cols(&lines[i].text, ind));
                    code.push('\n');
                    i += 1;
                }
                let lang = info
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if self.math && lang == "math" {
                    out.push(Block::Math {
                        text: code,
                        line: start,
                    });
                } else {
                    out.push(Block::Code {
                        info,
                        text: code,
                        line: start,
                    });
                }
                continue;
            }
            if self.math && t.starts_with("$$") {
                let start = line.no;
                let first = t.get(2..).unwrap_or_default().trim_end();
                if let Some(inner) = first.strip_suffix("$$") {
                    if !inner.trim().is_empty() {
                        out.push(Block::Math {
                            text: inner.trim().to_owned(),
                            line: start,
                        });
                        i += 1;
                        continue;
                    }
                }
                let mut tex = String::from(first);
                i += 1;
                let mut closed = false;
                while i < n {
                    let l = lines[i].text.trim_end();
                    i += 1;
                    if let Some(head) = l.strip_suffix("$$") {
                        tex.push('\n');
                        tex.push_str(head);
                        closed = true;
                        break;
                    }
                    tex.push('\n');
                    tex.push_str(l);
                }
                if closed {
                    out.push(Block::Math {
                        text: tex.trim().to_owned(),
                        line: start,
                    });
                } else {
                    out.push(Block::Paragraph {
                        text: format!("$${tex}"),
                        line: start,
                    });
                }
                continue;
            }
            if let Some((level, title, id)) = atx(t) {
                out.push(Block::Heading {
                    level,
                    text: title,
                    id,
                    line: line.no,
                    setext: false,
                });
                i += 1;
                continue;
            }
            if is_rule(t) {
                out.push(Block::Rule);
                i += 1;
                continue;
            }
            if let Some(kind) = html_start(t, false) {
                let start = line.no;
                let mut html = Vec::new();
                if kind <= 5 {
                    while i < n {
                        let l = &lines[i].text;
                        html.push(l.clone());
                        i += 1;
                        if html_end(kind, l) {
                            break;
                        }
                    }
                } else {
                    while i < n && !is_blank(&lines[i].text) {
                        html.push(lines[i].text.clone());
                        i += 1;
                    }
                }
                out.push(Block::Html {
                    text: html.join("\n"),
                    line: start,
                });
                continue;
            }
            if t.starts_with('>') {
                let start = line.no;
                let mut inner: Vec<Line> = Vec::new();
                let mut fence: Option<(char, usize)> = None;
                while i < n {
                    let l = &lines[i].text;
                    let li = indent(l);
                    if li <= 3 && l.get(li..).is_some_and(|x| x.starts_with('>')) {
                        let rest = l.get(li.saturating_add(1)..).unwrap_or_default();
                        let rest = rest.strip_prefix(' ').unwrap_or(rest);
                        fence = track_fence(fence, rest);
                        inner.push(Line {
                            no: lines[i].no,
                            text: rest.to_owned(),
                        });
                        i += 1;
                        continue;
                    }
                    let lazy = !is_blank(l)
                        && fence.is_none()
                        && inner
                            .last()
                            .is_some_and(|last| continues_paragraph(&last.text))
                        && !interrupts(l, self.math)
                        && list_marker(l).is_none();
                    if lazy {
                        inner.push(Line {
                            no: lines[i].no,
                            text: l.trim_start().to_owned(),
                        });
                        i += 1;
                        continue;
                    }
                    break;
                }
                let blocks = self.nested(&inner);
                out.push(Block::Quote {
                    blocks,
                    line: start,
                });
                continue;
            }
            if let Some(first) = list_marker(text) {
                let (list, next) = self.list(lines, i, first);
                out.push(list);
                i = next;
                continue;
            }
            if let Some(label) = note_label(t) {
                let start_line = lines[i].no;
                let after = t.get(label.len().saturating_add(4)..).unwrap_or_default();
                let mut inner = vec![Line {
                    no: start_line,
                    text: after.trim_start().to_owned(),
                }];
                i += 1;
                let mut last_blank = false;
                while i < n {
                    let l = &lines[i].text;
                    if is_blank(l) {
                        inner.push(Line {
                            no: lines[i].no,
                            text: String::new(),
                        });
                        last_blank = true;
                        i += 1;
                        continue;
                    }
                    if indent(l) >= 4 {
                        inner.push(Line {
                            no: lines[i].no,
                            text: strip_cols(l, 4),
                        });
                        last_blank = false;
                        i += 1;
                        continue;
                    }
                    if !last_blank
                        && !interrupts(l, self.math)
                        && note_label(l.trim_start()).is_none()
                    {
                        inner.push(Line {
                            no: lines[i].no,
                            text: l.trim_start().to_owned(),
                        });
                        i += 1;
                        continue;
                    }
                    break;
                }
                let blocks = self.nested(&inner);
                self.defs
                    .notes
                    .entry(normalize_label(&label))
                    .or_insert(blocks);
                continue;
            }
            if let Some(align) = table_at(lines, i) {
                let start = line.no;
                let head = cells(text);
                let width = head.len();
                i += 2;
                let mut rows = Vec::new();
                while i < n {
                    let l = &lines[i].text;
                    if is_blank(l) || interrupts(l, self.math) {
                        break;
                    }
                    let mut row = cells(l);
                    row.resize(width, String::new());
                    rows.push(row);
                    i += 1;
                }
                out.push(Block::Table {
                    align,
                    head,
                    rows,
                    line: start,
                });
                continue;
            }
            // A paragraph, or a setext heading when underlined.
            let start = line.no;
            let mut para: Vec<String> = vec![t.to_owned()];
            i += 1;
            let mut setext = None;
            while i < n {
                let l = &lines[i].text;
                if is_blank(l) {
                    break;
                }
                let li = indent(l);
                if li <= 3 {
                    let u = l.get(li..).unwrap_or_default().trim_end();
                    if !u.is_empty() && u.chars().all(|ch| ch == '=') {
                        setext = Some(1_u8);
                        i += 1;
                        break;
                    }
                    if !u.is_empty() && u.chars().all(|ch| ch == '-') {
                        let heading_like = para.len() == 1
                            && !para[0].trim_end().ends_with(['.', ',', ';', ':', ')'])
                            && para[0].chars().count() <= 120;
                        if self.strict || heading_like || u.len() < 3 {
                            setext = Some(2_u8);
                            i += 1;
                        }
                        break;
                    }
                }
                if table_at(lines, i).is_some() || interrupts(l, self.math) {
                    break;
                }
                // "Phase B, with the doctor:" then "5. A test": a numbered list
                // after a line that announces it, whatever its first number.
                if !self.strict
                    && para
                        .last()
                        .is_some_and(|last| last.trim_end().trim_end_matches('*').ends_with(':'))
                    && list_marker(l).is_some_and(|m| m.ordered && !m.empty)
                {
                    break;
                }
                para.push(l.trim_start().to_owned());
                i += 1;
            }
            // Link reference definitions open a paragraph; they are taken out.
            // (An offset moves past them, and the lines go in one drain: not
            // one shift of the whole paragraph per definition.)
            let mut taken = 0;
            while let Some((label, dest, title, used)) = ref_def(&para[taken..]) {
                self.defs
                    .links
                    .entry(normalize_label(&label))
                    .or_insert((dest, title));
                taken = (taken + used).min(para.len());
            }
            para.drain(..taken);
            if para.is_empty() {
                if let Some(level) = setext {
                    // An underline with nothing left above it.
                    if level == 2 {
                        out.push(Block::Rule);
                    } else {
                        out.push(Block::Paragraph {
                            text: String::from("==="),
                            line: start,
                        });
                    }
                }
                continue;
            }
            let mut body = para.join("\n");
            let trimmed = body.trim_end().len();
            body.truncate(trimmed);
            match setext {
                Some(level) => {
                    let (title, id) = heading_id(body.trim());
                    out.push(Block::Heading {
                        level,
                        text: title,
                        id,
                        line: start,
                        setext: true,
                    });
                }
                None => out.push(Block::Paragraph {
                    text: body,
                    line: start,
                }),
            }
        }
        out
    }

    /// The blocks of a container's lines, one level deeper.
    fn nested(&mut self, lines: &[Line]) -> Vec<Block> {
        if self.depth >= 64 {
            // Hostile nesting: the rest is text.
            let text = lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            return vec![Block::Paragraph {
                text,
                line: lines.first().map_or(0, |l| l.no),
            }];
        }
        self.depth += 1;
        let blocks = self.blocks(lines);
        self.depth -= 1;
        blocks
    }

    /// A list starting at `at` with marker `first`; the list and the line
    /// after it.
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch over the kinds it handles, read top to bottom"
    )]
    fn list(&mut self, lines: &[Line], at: usize, first: Marker) -> (Block, usize) {
        let n = lines.len();
        let mut i = at;
        let mut marker = first;
        let mut items = Vec::new();
        let mut loose = false;
        loop {
            let col = marker.col;
            let head = &lines[i].text;
            let mut inner = vec![Line {
                no: lines[i].no,
                text: if marker.empty {
                    String::new()
                } else {
                    head.get(col.min(head.len())..)
                        .unwrap_or_default()
                        .to_owned()
                },
            }];
            i += 1;
            let mut last_blank = marker.empty;
            let mut blank_inside = false;
            let mut fence: Option<(char, usize)> = track_fence(None, &inner[0].text);
            // An item that opens empty and is followed by a blank line ends
            // there.
            let starts_empty = marker.empty && (i >= n || is_blank(&lines[i].text));
            if !starts_empty {
                while i < n {
                    let l = &lines[i].text;
                    if is_blank(l) {
                        inner.push(Line {
                            no: lines[i].no,
                            text: String::new(),
                        });
                        last_blank = true;
                        i += 1;
                        continue;
                    }
                    if indent(l) >= col {
                        if last_blank && inner.iter().any(|x| !is_blank(&x.text)) && fence.is_none()
                        {
                            blank_inside = true;
                        }
                        let stripped = strip_cols(l, col);
                        fence = track_fence(fence, &stripped);
                        inner.push(Line {
                            no: lines[i].no,
                            text: stripped,
                        });
                        last_blank = false;
                        i += 1;
                        continue;
                    }
                    let lazy = !last_blank
                        && fence.is_none()
                        && inner.last().is_some_and(|x| continues_paragraph(&x.text))
                        && !interrupts(l, self.math)
                        && list_marker(l).is_none();
                    if lazy {
                        inner.push(Line {
                            no: lines[i].no,
                            text: l.trim_start().to_owned(),
                        });
                        i += 1;
                        continue;
                    }
                    break;
                }
            }
            let mut trailing = 0_usize;
            while inner.len() > 1 && inner.last().is_some_and(|x| is_blank(&x.text)) {
                inner.pop();
                trailing += 1;
            }
            if starts_empty {
                while i < n && is_blank(&lines[i].text) {
                    i += 1;
                    trailing += 1;
                }
            }
            let mut blocks = self.nested(&inner);
            // Loose when a blank line stands between two of the item's own
            // blocks; a blank inside a nested list or a code block is not one.
            for pair in blocks.windows(2) {
                match first_line(&pair[1]) {
                    Some(start) => {
                        if let Some(pos) = inner.iter().position(|l| l.no == start) {
                            if pos > 0 && is_blank(&inner[pos - 1].text) {
                                loose = true;
                            }
                        }
                    }
                    None => loose |= blank_inside,
                }
            }
            let task = take_task(&mut blocks);
            items.push(Item { task, blocks });
            if i < n {
                let next = &lines[i].text;
                if let Some(m) = list_marker(next) {
                    let ind = indent(next);
                    let same = m.ordered == first.ordered && m.mark == first.mark;
                    if same && !is_rule(next.get(ind..).unwrap_or_default()) {
                        if trailing > 0 {
                            loose = true;
                        }
                        marker = m;
                        continue;
                    }
                }
            }
            break;
        }
        (
            Block::List {
                ordered: first.ordered,
                start: first.start,
                tight: !loose,
                items,
            },
            i,
        )
    }
}

/// The line a block starts on, looking into a list for its first item.
fn first_line(block: &Block) -> Option<usize> {
    match block {
        Block::Heading {
            line,
            level: _,
            text: _,
            id: _,
            setext: _,
        }
        | Block::Paragraph { line, text: _ }
        | Block::Code {
            line,
            info: _,
            text: _,
        }
        | Block::Html { line, text: _ }
        | Block::Quote { line, blocks: _ }
        | Block::Table {
            line,
            align: _,
            head: _,
            rows: _,
        }
        | Block::Math { line, text: _ } => Some(*line),
        Block::List {
            items,
            ordered: _,
            start: _,
            tight: _,
        } => items
            .first()
            .and_then(|item| item.blocks.first())
            .and_then(first_line),
        Block::Rule => None,
    }
}

/// Whether a container line, seen as the last one so far, leaves a
/// paragraph open that a lazy line may continue.
fn continues_paragraph(text: &str) -> bool {
    if is_blank(text) || indent(text) >= 4 {
        return false;
    }
    let t = text.get(indent(text)..).unwrap_or_default();
    atx(t).is_none() && fence_open(t).is_none() && !is_rule(t) && html_start(t, false).is_none()
}

/// The fence state after a line: opened, closed or unchanged.
fn track_fence(state: Option<(char, usize)>, line: &str) -> Option<(char, usize)> {
    if let Some((mark, len)) = state {
        (!fence_close(line, mark, len)).then_some((mark, len))
    } else {
        let ind = indent(line);
        if ind > 3 {
            return None;
        }
        fence_open(line.get(ind..).unwrap_or_default()).map(|(mark, len, _info)| (mark, len))
    }
}

/// `[^label]:` opening a footnote definition: the label.
fn note_label(t: &str) -> Option<String> {
    let rest = t.strip_prefix("[^")?;
    let close = rest.find("]:")?;
    let label = rest.get(..close)?;
    (!label.is_empty() && !label.contains(char::is_whitespace) && !label.contains(['[', ']']))
        .then(|| label.to_owned())
}

/// A task item's box, `[ ]` or `[x]`, taken off the item's first paragraph.
fn take_task(blocks: &mut [Block]) -> Option<bool> {
    let Some(Block::Paragraph { text, line: _ }) = blocks.first_mut() else {
        return None;
    };
    let done = match text.get(..3) {
        Some("[ ]") => false,
        Some("[x]" | "[X]") => true,
        _ => return None,
    };
    let rest = text.get(3..).unwrap_or_default();
    if !(rest.starts_with(' ') || rest.starts_with('\n')) {
        return None;
    }
    *text = rest.trim_start().to_owned();
    Some(done)
}

#[cfg(test)]
mod tests {
    use super::{Block, parse};

    fn kinds(text: &str) -> Vec<&'static str> {
        parse(text, 1, true, false)
            .0
            .iter()
            .map(|block| match block {
                Block::Heading {
                    level: _,
                    text: _,
                    id: _,
                    line: _,
                    setext: _,
                } => "h",
                Block::Paragraph { text: _, line: _ } => "p",
                Block::Code {
                    info: _,
                    text: _,
                    line: _,
                } => "code",
                Block::Html { text: _, line: _ } => "html",
                Block::Rule => "hr",
                Block::Quote { blocks: _, line: _ } => "quote",
                Block::List {
                    ordered: _,
                    start: _,
                    tight: _,
                    items: _,
                } => "list",
                Block::Table {
                    align: _,
                    head: _,
                    rows: _,
                    line: _,
                } => "table",
                Block::Math { text: _, line: _ } => "math",
            })
            .collect()
    }

    #[test]
    fn a_nested_list_nests() {
        let (blocks, _defs) = parse("- a\n  - b\n  - c\n- d\n", 1, false, false);
        let Some(Block::List {
            items,
            tight,
            ordered: _,
            start: _,
        }) = blocks.first()
        else {
            panic!("a list");
        };
        assert!(*tight);
        assert_eq!(items.len(), 2);
        assert!(
            matches!(items[0].blocks.get(1), Some(Block::List { items, ordered: _, start: _, tight: _ }) if items.len() == 2)
        );
    }

    #[test]
    fn a_table_may_follow_a_line_of_text() {
        assert_eq!(kinds("Text:\n| a | b |\n|---|---|\n| 1 | 2 |\n"), [
            "p", "table"
        ]);
    }

    #[test]
    fn task_items_and_footnotes() {
        let (blocks, defs) = parse(
            "- [x] done\n- [ ] open\n\n[^1]: The note.\n",
            1,
            false,
            false,
        );
        let Some(Block::List {
            items,
            ordered: _,
            start: _,
            tight: _,
        }) = blocks.first()
        else {
            panic!("a list");
        };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
        assert!(defs.notes.contains_key("1"));
    }

    #[test]
    fn a_comment_spanning_lines_is_one_block() {
        assert_eq!(kinds("<!--\nhidden\n-->\n\nText.\n"), ["html", "p"]);
    }

    #[test]
    fn maths_blocks() {
        assert_eq!(kinds("$$\nx^2\n$$\n\n```math\ny\n```\n"), ["math", "math"]);
    }
}
