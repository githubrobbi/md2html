// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Glossary terms, and the first mention of each term in every chapter
//! linked to its entry.
//!
//! What a term is, which heading names the glossary, and how a mention
//! is found and linked: what the renderer and the page need. Reading a
//! glossary chapter into terms is the document's (`document/glossary.rs`),
//! where a meaning is read as Markdown and written as HTML.

use alloc::collections::{BTreeMap, BTreeSet};

use crate::text::esc;

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
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.')
        .trim()
        .to_lowercase();
    ["glossary", "glossar"].contains(&bare.as_str())
        || crate::config::all_ui()
            .iter()
            .any(|ui| ui.glossary_heading.to_lowercase() == bare)
}

/// The spellings a glossary head offers the prose: `Sidecar / blob` is
/// two words for one thing, `Unit (passage)` is a word and its gloss, and
/// the "Also" column names more.
fn needles(term: &Term) -> Vec<String> {
    let mut out: Vec<String> = term
        .word
        .split(" / ")
        .map(|piece| {
            piece
                .split_once(" (")
                .map_or(piece, |(word, _)| word)
                .trim()
                .to_owned()
        })
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
    pub(crate) seen: BTreeSet<String>,
    /// Every mention linked in the page, by term.
    pub(crate) minted: BTreeMap<String, usize>,
}

impl Linker {
    /// Escaped text with the first unlinked mention of each term linked.
    pub(crate) fn link(&mut self, escaped: &str, terms: &[Term]) -> String {
        let mut out = escaped.to_owned();
        for term in terms {
            if self.seen.contains(&term.word) {
                continue;
            }
            let Some(found) = needles(term)
                .iter()
                .filter_map(|needle| find_word(&out, needle))
                .min_by_key(|found| found.before.len())
            else {
                continue;
            };
            let linked = format!(
                "{}{}{}",
                found.before,
                self.mint(term, found.word),
                found.after
            );
            out = linked;
        }
        out
    }

    /// A code span that is exactly a term (`PATH`, `WAL`) explains itself
    /// like any other mention.
    pub(crate) fn code(&mut self, code: &str, rendered: &str, terms: &[Term]) -> String {
        let known = terms.iter().find(|term| {
            !self.seen.contains(&term.word)
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
        self.seen.insert(term.word.clone());
        let nth = self
            .minted
            .entry(term.word.clone())
            .and_modify(|count| *count = count.saturating_add(1))
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

/// Text cut in three around a word found in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Found<'text> {
    /// What comes before the word.
    before: &'text str,
    /// The word as the text writes it.
    word: &'text str,
    /// What comes after it.
    after: &'text str,
}

impl<'text> Found<'text> {
    /// `text` cut where a search in it found `len` bytes at `start`; None
    /// when either cut is not between two characters.
    fn cut(text: &'text str, start: usize, len: usize) -> Option<Self> {
        let (before, rest) = text.split_at_checked(start)?;
        let (word, after) = rest.split_at_checked(len)?;
        Some(Self {
            before,
            word,
            after,
        })
    }

    /// Whether the word stands as a word of the prose: not joined to
    /// letters or underscores (a file name), not in a tag, not in a link.
    fn stands_alone(&self) -> bool {
        let word_char = |ch: char| ch.is_alphanumeric() || ch == '_';
        // Chinese, Japanese and Thai put no spaces between words: a term
        // written in them stands as a word wherever it stands.
        let open_start = self.word.chars().next().is_some_and(unspaced);
        let open_end = self.word.chars().next_back().is_some_and(unspaced);
        let joined_before = !open_start && self.before.chars().next_back().is_some_and(word_char);
        let joined_after = !open_end && self.after.chars().next().is_some_and(word_char);
        let inside_tag = self.before.rfind('<') > self.before.rfind('>');
        let inside_link = self.before.rfind("<a ") > self.before.rfind("</a>");
        !(joined_before || joined_after || inside_tag || inside_link)
    }
}

/// The endings a mention may add to a term: "entity" is also "entities".
const PLURALS: [&str; 2] = ["es", "s"];

/// The first whole-word occurrence of `word` in escaped text that is not
/// inside a tag or a link. An acronym matches as written (PATH, not path);
/// anything else regardless of case, plural included.
fn find_word<'text>(haystack: &'text str, word: &str) -> Option<Found<'text>> {
    let exact = word.chars().count() >= 2 && !word.chars().any(char::is_lowercase);
    if exact {
        return first_word(haystack, haystack, &esc(word), &PLURALS);
    }
    let needle = esc(&word.to_lowercase());
    if needle.is_empty() {
        return None;
    }
    let lower = haystack.to_lowercase();
    if lower.len() == haystack.len() {
        first_word(haystack, &lower, &needle, &PLURALS)
    } else {
        // Lowercasing changed the byte length (a rare letter): match exactly.
        first_word(haystack, haystack, &esc(word), &[])
    }
}

/// The first place `needle` stands as a word in `folded` (the haystack,
/// or the haystack lowercased to the same length), with one of `endings`
/// when one follows, cut from the haystack at the same place.
fn first_word<'text>(
    haystack: &'text str,
    folded: &str,
    needle: &str,
    endings: &[&str],
) -> Option<Found<'text>> {
    if needle.is_empty() {
        return None;
    }
    folded.match_indices(needle).find_map(|(start, bare)| {
        let tail = Found::cut(folded, start, bare.len())?.after;
        let ending = endings
            .iter()
            .find(|ending| {
                tail.strip_prefix(**ending)
                    .is_some_and(|rest| rest.chars().next().is_none_or(|ch| !ch.is_alphanumeric()))
            })
            .map_or(0, |ending| ending.len());
        let len = bare.len().checked_add(ending)?;
        Found::cut(folded, start, len)?
            .stands_alone()
            .then(|| Found::cut(haystack, start, len))
            .flatten()
    })
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
