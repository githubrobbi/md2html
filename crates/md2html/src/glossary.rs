// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::min_ident_chars,
    clippy::shadow_reuse,
    clippy::string_slice,
    clippy::wildcard_enum_match_arm,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]

//! The glossary: a chapter of terms and meanings, and the first mention
//! of each term in every chapter linked to its entry.

use alloc::collections::BTreeMap;

use crate::block::{Block, Defs};
use crate::inline::{Env, HtmlMode};
use crate::text::{esc, slug};

/// One glossary entry.
#[derive(Clone, Debug)]
pub(crate) struct Term {
    /// The word as the glossary spells it.
    pub(crate) word: String,
    /// The definition, rendered.
    pub(crate) said: String,
    /// The definition as plain text, for the hover.
    pub(crate) plain: String,
    /// The anchor slug.
    pub(crate) slug: String,
    /// From the shared glossary rather than this page's own chapter.
    pub(crate) shared: bool,
    /// Other spellings the prose uses for it (a third column, "Also").
    pub(crate) also: Vec<String>,
}

/// Whether a chapter heading names the glossary: "Glossary", "9. Glossary",
/// "Glossar". The capital matters: a chapter about "the shared glossary"
/// is prose, not the glossary itself.
pub(crate) fn is_glossary_heading(text: &str) -> bool {
    let bare = text
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
        .trim()
        .to_lowercase();
    ["glossary", "glossar"].contains(&bare.as_str())
        || crate::config::all_ui()
            .iter()
            .any(|ui| ui.glossary_heading.to_lowercase() == bare)
}

/// The terms a document's glossary chapter defines, longest first so
/// "content object" wins over "content".
pub(crate) fn terms(blocks: &[Block], defs: &Defs) -> Vec<Term> {
    let mut out = Vec::new();
    let mut inside = false;
    for block in blocks {
        if let Block::Heading {
            level,
            text,
            id: _,
            line: _,
            setext: _,
        } = block
        {
            if *level <= 2 {
                inside = *level == 2 && is_glossary_heading(text);
            }
            continue;
        }
        if !inside {
            continue;
        }
        match block {
            Block::Table {
                head,
                rows,
                align: _,
                line: _,
            } => {
                let also_col = head.iter().position(|cell| {
                    let c = cell.to_lowercase();
                    [
                        "also",
                        "alias",
                        "auch",
                        "ook",
                        "juga",
                        "uga",
                        "aussi",
                        "anche",
                        "también",
                        "別名",
                        "べつめい",
                        "betsumei",
                        "又称",
                        "又稱",
                        "亦稱",
                        "亦称",
                        "tambien",
                        "גם",
                        "אויך",
                    ]
                    .iter()
                    .any(|w| c.contains(w))
                });
                for row in rows {
                    let (Some(word), Some(said)) = (row.first(), row.get(1)) else {
                        continue;
                    };
                    let word = word.trim().trim_matches('*').trim_matches('`').trim();
                    if word.is_empty() {
                        continue;
                    }
                    let also = also_col
                        .and_then(|at| row.get(at))
                        .map(|cell| {
                            cell.split([',', ';'])
                                .map(|alias| alias.trim().trim_matches('`').to_owned())
                                .filter(|alias| !alias.is_empty())
                                .collect()
                        })
                        .unwrap_or_default();
                    out.push(finish(word, said, also, defs));
                }
            }
            Block::Paragraph { text, line: _ } => {
                if let Some((word, said)) = bold_lead(text) {
                    out.push(finish(&word, &said, Vec::new(), defs));
                }
            }
            Block::List {
                items,
                ordered: _,
                start: _,
                tight: _,
            } => {
                for item in items {
                    if let Some(Block::Paragraph { text, line: _ }) = item.blocks.first() {
                        if let Some((word, said)) = bold_lead(text) {
                            out.push(finish(&word, &said, Vec::new(), defs));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out.sort_by_key(|term| core::cmp::Reverse(term.word.len()));
    out
}

/// `**Term** - meaning`, `**Term**: meaning` or `**Term.** meaning`.
fn bold_lead(text: &str) -> Option<(String, String)> {
    let rest = text.strip_prefix("**")?;
    let (word, tail) = rest.split_once("**")?;
    let word = word.trim().trim_end_matches([':', '.']).trim();
    if word.is_empty() {
        return None;
    }
    let said = tail
        .trim_start()
        .trim_start_matches([':', '-', '–', '—', '.'])
        .trim()
        .replace('\n', " ");
    Some((word.to_owned(), said))
}

/// One entry, its definition rendered and as plain text.
fn finish(word: &str, said: &str, also: Vec<String>, defs: &Defs) -> Term {
    let env = Env {
        defs,
        html: HtmlMode::Safe,
        math: false,
    };
    let nodes = crate::inline::parse(said, &env);
    let rendered = crate::render::bare_inline(&nodes);
    Term {
        word: word.to_owned(),
        said: rendered,
        plain: crate::inline::plain(&nodes),
        slug: slug(word),
        shared: false,
        also,
    }
}

/// The spellings a glossary head offers the prose: `Sidecar / blob` is
/// two words for one thing, `Unit (passage)` is a word and its gloss, and
/// the "Also" column names more.
pub(crate) fn needles(term: &Term) -> Vec<String> {
    let mut out: Vec<String> = term
        .word
        .split(" / ")
        .map(|piece| piece.split(" (").next().unwrap_or(piece).trim().to_owned())
        .filter(|piece| !piece.is_empty())
        .collect();
    out.extend(term.also.iter().cloned());
    // "entity" is also "entities".
    let plurals: Vec<String> = out
        .iter()
        .filter_map(|needle| needle.strip_suffix('y').map(|stem| format!("{stem}ies")))
        .collect();
    out.extend(plurals);
    out
}

/// Which terms have been linked: per chapter (one link per chapter) and
/// in the whole page (each link its own anchor, for the way back).
#[derive(Default)]
pub(crate) struct Linker {
    /// Terms already linked in this chapter.
    pub(crate) seen: BTreeMap<String, usize>,
    /// Every mention linked in the page, by term.
    pub(crate) minted: BTreeMap<String, usize>,
}

impl Linker {
    /// Escaped text with the first unlinked mention of each term linked.
    pub(crate) fn link(&mut self, escaped: &str, terms: &[Term]) -> String {
        let mut out = escaped.to_owned();
        for term in terms {
            if self.seen.get(&term.word).copied().unwrap_or(0) > 0 {
                continue;
            }
            let Some((at, len)) = needles(term)
                .iter()
                .filter_map(|needle| find_word(&out, needle))
                .min_by_key(|(start, _len)| *start)
            else {
                continue;
            };
            let hit = out.get(at..at + len).unwrap_or_default().to_owned();
            let link = self.mint(term, &hit);
            out.replace_range(at..at + len, &link);
        }
        out
    }

    /// A code span that is exactly a term (`PATH`, `WAL`) explains itself
    /// like any other mention.
    pub(crate) fn code(&mut self, code: &str, rendered: &str, terms: &[Term]) -> String {
        let known = terms.iter().find(|term| {
            self.seen.get(&term.word).copied().unwrap_or(0) == 0
                && needles(term).iter().any(|needle| {
                    if needle.chars().any(char::is_lowercase) {
                        needle.eq_ignore_ascii_case(code)
                    } else {
                        needle == code
                    }
                })
        });
        known.map_or_else(|| rendered.to_owned(), |term| self.mint(term, rendered))
    }

    /// One link to a glossary entry around `shown`, with an anchor of its
    /// own so the entry's "back" returns to exactly this mention.
    fn mint(&mut self, term: &Term, shown: &str) -> String {
        self.seen.insert(term.word.clone(), 1);
        let nth = self
            .minted
            .entry(term.word.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        format!(
            "<a class=\"term\" id=\"t-{}-{nth}\" data-key=\"{}\" href=\"#g-{}\" data-def=\"{}\">{shown}</a>",
            term.slug,
            term.slug,
            term.slug,
            esc(&term.plain)
        )
    }
}

/// The first whole-word occurrence of `word` in escaped text that is not
/// inside a tag or a link, as `(start, length)`. An acronym matches as
/// written (PATH, not path); anything else regardless of case, plural
/// included.
pub(crate) fn find_word(haystack: &str, word: &str) -> Option<(usize, usize)> {
    let exact = word.chars().count() >= 2 && !word.chars().any(char::is_lowercase);
    let lower = if exact {
        haystack.to_owned()
    } else {
        haystack.to_lowercase()
    };
    let needle = if exact {
        esc(word)
    } else {
        esc(&word.to_lowercase())
    };
    if needle.is_empty() || lower.len() != haystack.len() {
        // Lowercasing changed the byte length (a rare letter): match exactly.
        return (!needle.is_empty() && lower.len() != haystack.len())
            .then(|| find_word_exact(haystack, &esc(word)))
            .flatten();
    }
    let mut from = 0;
    while let Some(at) = lower.get(from..)?.find(&needle) {
        let start = from + at;
        let bare_end = start + needle.len();
        let tail = lower.get(bare_end..)?;
        let plural = ["es", "s"]
            .iter()
            .find(|suffix| {
                tail.strip_prefix(**suffix)
                    .is_some_and(|rest| rest.chars().next().is_none_or(|ch| !ch.is_alphanumeric()))
            })
            .map_or(0, |suffix| suffix.len());
        let end = bare_end + plural;
        if whole_word(&lower, start, end) {
            return Some((start, end - start));
        }
        from = bare_end;
    }
    None
}

/// `find_word` without folding case.
fn find_word_exact(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    let mut from = 0;
    while let Some(at) = haystack.get(from..)?.find(needle) {
        let start = from + at;
        let end = start + needle.len();
        if whole_word(haystack, start, end) {
            return Some((start, end - start));
        }
        from = end;
    }
    None
}

/// Whether `text[start..end]` stands as a word of the prose: not joined to
/// letters or underscores (a file name), not in a tag, not in a link.
fn whole_word(text: &str, start: usize, end: usize) -> bool {
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return false;
    }
    let word_char = |ch: char| ch.is_alphanumeric() || ch == '_';
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    // Chinese, Japanese and Thai put no spaces between words: a term
    // written in them stands as a word wherever it stands.
    let first = text[start..end].chars().next().is_some_and(unspaced);
    let last = text[start..end].chars().next_back().is_some_and(unspaced);
    let before = before.filter(|_| !first);
    let after = after.filter(|_| !last);
    let head = &text[..start];
    let inside_tag = head.rfind('<') > head.rfind('>');
    let inside_link = head.rfind("<a ") > head.rfind("</a>");
    before.is_none_or(|ch| !word_char(ch))
        && after.is_none_or(|ch| !word_char(ch))
        && !inside_tag
        && !inside_link
}

/// A letter of a script written without spaces between words.
fn unspaced(ch: char) -> bool {
    matches!(u32::from(ch),
        0x0E00..=0x0E7F // Thai
        | 0x3040..=0x30FF // hiragana, katakana
        | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF // Han
        | 0xFF66..=0xFF9F // half-width katakana
        | 0x20000..=0x3134F) // Han, the rarer
}
