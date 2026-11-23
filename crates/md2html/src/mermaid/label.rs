// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The words on a diagram: a label read from the source, broken to fit,
//! measured in the page's sans font, and written as SVG text.

use super::geom::{Len, count, px};
use super::plane::{Point, Size};
use crate::text::{esc, put};

/// The height of one line of a label.
const LINE: Len = px(18.0_f64);

/// A label's lines: always at least one.
#[derive(Clone, Debug)]
pub(super) struct Label {
    /// The lines, top to bottom.
    lines: Vec<String>,
}

impl Label {
    /// A label as written in the source: `<br>` and `\n` break it,
    /// `#quot;`-style and HTML entities are decoded, Markdown marks and
    /// enclosing quotes dropped.
    pub(super) fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        let unquoted = trimmed
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
            .or_else(|| {
                trimmed
                    .strip_prefix('`')
                    .and_then(|inner| inner.strip_suffix('`'))
            })
            .unwrap_or(trimmed);
        let mut text = unquoted.to_owned();
        for br in ["<br/>", "<br />", "<br>", "<BR>", "<BR/>", "\\n"] {
            text = text.replace(br, "\n");
        }
        let decoded = crate::text::decode_entities(&mermaid_escapes(&text))
            .replace("**", "")
            .replace("__", "");
        Self {
            lines: decoded
                .split('\n')
                .map(|line| line.trim().to_owned())
                .collect(),
        }
    }

    /// The label with its lines broken at word boundaries to fit `max` at
    /// `font`.
    pub(super) fn wrapped(self, max: Len, font: Len) -> Self {
        let mut lines = Vec::new();
        for line in self.lines {
            if text_width(&line, font) <= max {
                lines.push(line);
                continue;
            }
            let mut current = String::new();
            for word in line.split(' ') {
                let candidate = if current.is_empty() {
                    word.to_owned()
                } else {
                    format!("{current} {word}")
                };
                if !current.is_empty() && text_width(&candidate, font) > max {
                    lines.push(core::mem::take(&mut current));
                    word.clone_into(&mut current);
                } else {
                    current = candidate;
                }
            }
            lines.push(current);
        }
        Self { lines }
    }

    /// The lines joined by spaces: the label read aloud.
    pub(super) fn joined(&self) -> String {
        self.lines.join(" ")
    }

    /// Whether every line is empty.
    pub(super) fn is_blank(&self) -> bool {
        self.lines.iter().all(String::is_empty)
    }

    /// Whether it is one empty line: no label at all.
    pub(super) fn is_empty(&self) -> bool {
        matches!(self.lines.as_slice(), [only] if only.is_empty())
    }

    /// The room its lines take at `font`.
    pub(super) fn size(&self, font: Len) -> Size {
        let width = self
            .lines
            .iter()
            .map(|line| text_width(line, font))
            .fold(px(0.0_f64), Len::max);
        Size::new(width, LINE * count(self.lines.len().max(1)))
    }

    /// The lines as SVG text, centred on `centre`.
    pub(super) fn draw(&self, out: &mut String, centre: Point, class: &str) {
        let Point { x, y } = centre;
        let extra = count(self.lines.len().saturating_sub(1));
        let top = y - LINE * extra / 2.0_f64;
        put!(
            out,
            "<text class=\"{class}\" x=\"{x:.1}\" y=\"{top:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\">"
        );
        if let Some((first, rest)) = self.lines.split_first() {
            put!(out, "<tspan x=\"{x:.1}\">{}</tspan>", esc(first));
            for line in rest {
                put!(
                    out,
                    "<tspan x=\"{x:.1}\" dy=\"{LINE}\">{}</tspan>",
                    esc(line)
                );
            }
        }
        out.push_str("</text>");
    }
}

/// Mermaid's own escapes decoded: `#quot;`, `#amp;`, `#35;`, ... A `#`
/// that starts none of them stays.
fn mermaid_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, tail)) = rest.split_once('#') {
        out.push_str(before);
        if let Some((name, after)) = tail.split_once(';').filter(|(name, _)| name.len() <= 8) {
            let decoded = match name {
                "quot" => Some('"'),
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "nbsp" => Some('\u{a0}'),
                _ => name.parse::<u32>().ok().and_then(char::from_u32),
            };
            if let Some(ch) = decoded {
                out.push(ch);
            } else {
                out.push('#');
                out.push_str(name);
                out.push(';');
            }
            rest = after;
        } else {
            out.push('#');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// The width of text in the page's sans font at `font` pixels, estimated
/// from the shapes of the letters (no font is measured; the boxes are made
/// a little roomy for it).
pub(super) fn text_width(text: &str, font: Len) -> Len {
    let ems: f64 = text
        .chars()
        .map(|ch| match ch {
            'i' | 'l' | 'j' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | 'I' | '`' => 0.28_f64,
            ' ' => 0.29_f64,
            'f' | 't' | 'r' | '(' | ')' | '[' | ']' | '/' | '-' => 0.37_f64,
            'm' | 'w' => 0.84_f64,
            'M' | 'W' | '@' | '%' => 0.92_f64,
            'A'..='Z' => 0.67_f64,
            '0'..='9' => 0.6_f64,
            'a'..='z' => 0.55_f64,
            other if is_wide(other) => 1.0_f64,
            _ => 0.62_f64,
        })
        .sum();
    font * ems
}

/// A character that takes a full em: CJK, emoji.
fn is_wide(ch: char) -> bool {
    matches!(u32::from(ch), 0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x1F300..=0x1FAFF)
}
