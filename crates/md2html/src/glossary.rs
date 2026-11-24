// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Glossary terms, and the first mention of each term in every chapter
//! linked to its entry.
//!
//! What a term is, which heading names the glossary, and how a mention
//! is found and linked: what the renderer and the page need. Reading a
//! glossary chapter into terms is the document's (`document/glossary.rs`),
//! where a meaning is read as Markdown and written as HTML.

use alloc::borrow::Cow;
use alloc::collections::{BTreeMap, BTreeSet};

use self::index::{Case, Key, TermIndex};
use crate::text::{esc, put};

mod index;

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
            .any(|ui| crate::config::lowercase_is(ui.glossary_heading, &bare))
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

/// One spelling of a term, made ready once for every text of the page
/// that is searched for it.
struct Spelling {
    /// As the glossary writes it: what a code span is compared with.
    written: String,
    /// It has a lowercase letter: a code span may match it in any case.
    has_lowercase: bool,
    /// Two letters or more, none of them lowercase (PATH): matched as
    /// written, not in any case.
    acronym: bool,
    /// As written, escaped like the text it is looked for in.
    escaped: String,
    /// Lowercased, then escaped: what lowercased text is searched for.
    folded: String,
}

impl Spelling {
    /// The spelling `written`, made ready.
    fn new(written: String) -> Self {
        let has_lowercase = written.chars().any(char::is_lowercase);
        let acronym = !has_lowercase && written.chars().nth(1).is_some();
        let escaped = esc(&written);
        let folded = esc(&written.to_lowercase());
        Self {
            written,
            has_lowercase,
            acronym,
            escaped,
            folded,
        }
    }

    /// The first whole-word occurrence of the spelling in escaped text
    /// that is not inside a tag or a link. An acronym matches as written
    /// (PATH, not path); anything else regardless of case, plural
    /// included. `lower` is the text lowercased.
    fn find<'text>(&self, text: &'text str, lower: &str) -> Option<Found<'text>> {
        if self.acronym {
            return first_word(text, text, &self.escaped, &PLURALS);
        }
        if self.folded.is_empty() {
            return None;
        }
        if lower.len() == text.len() {
            first_word(text, lower, &self.folded, &PLURALS)
        } else {
            // Lowercasing changed the byte length (a rare letter): match
            // exactly.
            first_word(text, text, &self.escaped, &[])
        }
    }

    /// How the spelling is looked up in a [`TermIndex`]: as an acronym, in
    /// the text as written; otherwise in the text lowercased. None when it
    /// is never found.
    fn key(&self) -> Option<(Case, Key<'_>)> {
        if self.acronym {
            Key::of(&self.escaped).map(|key| (Case::AsWritten, key))
        } else {
            Key::of(&self.folded).map(|key| (Case::Lowercased, key))
        }
    }

    /// Whether a code span is exactly this spelling: in any case when the
    /// spelling has a lowercase letter, as written when it has none.
    fn names(&self, code: &str) -> bool {
        if self.has_lowercase {
            self.written.eq_ignore_ascii_case(code)
        } else {
            self.written == code
        }
    }
}

/// A term of the page with what linking it needs, made once per page.
struct Entry<'terms> {
    /// The term.
    term: &'terms Term,
    /// Its spellings, in the order they are tried.
    spellings: Vec<Spelling>,
    /// Its definition as plain text, escaped for the hover's attribute.
    hover: String,
}

/// The page's terms, and the first unlinked mention of each in every
/// chapter linked to its entry.
pub(crate) struct Linker<'terms> {
    /// The terms, each with its spellings, in the order they are tried.
    entries: Vec<Entry<'terms>>,
    /// Which of them a text may mention, by the words it has.
    index: TermIndex,
    /// The mentions linked so far.
    mentions: Mentions,
}

impl<'terms> Linker<'terms> {
    /// A linker for `terms`: their spellings are worked out here, once,
    /// not again for every text of the page.
    pub(crate) fn new(terms: &'terms [Term]) -> Self {
        let entries: Vec<Entry<'terms>> = terms
            .iter()
            .map(|term| Entry {
                term,
                spellings: needles(term).into_iter().map(Spelling::new).collect(),
                hover: esc(&term.plain),
            })
            .collect();
        let index = TermIndex::new(&entries);
        Self {
            entries,
            index,
            mentions: Mentions::default(),
        }
    }

    /// A new chapter: each term may be linked once more.
    pub(crate) fn next_chapter(&mut self) {
        self.mentions.seen.clear();
    }

    /// How many mentions of each term the page links, by term; the foot's
    /// glossary links back to each of them.
    #[must_use = "the glossary's way back to each mention is lost without the count"]
    pub(crate) fn into_minted(self) -> BTreeMap<String, usize> {
        self.mentions.minted
    }

    /// Writes escaped text to `out` with the first unlinked mention of
    /// each term linked. Each term is looked for in the text as the terms
    /// before it left it; the text is copied only when a term is found in
    /// it.
    ///
    /// Only the terms the text has a key of are looked for (see
    /// [`TermIndex`]): a text costs in proportion to its words, not to the
    /// size of the glossary.
    pub(crate) fn link(&mut self, escaped: &str, out: &mut String) {
        if self.entries.is_empty() {
            out.push_str(escaped);
            return;
        }
        let mut text = Cow::Borrowed(escaped);
        let mut lower = text.to_lowercase();
        let mut places = self.places(&text, &lower);
        let mut next = 0;
        while let Some(&place) = places.range(next..).next() {
            next = place.saturating_add(1);
            let Some(entry) = self.entries.get(place) else {
                break;
            };
            if self.mentions.seen.contains(&entry.term.word) {
                continue;
            }
            let Some(found) = entry
                .spellings
                .iter()
                .filter_map(|spelling| spelling.find(&text, &lower))
                .min_by_key(|found| found.before.len())
            else {
                continue;
            };
            let mut linked = String::with_capacity(text.len().saturating_add(MINT_LEN));
            linked.push_str(found.before);
            self.mentions.mint(entry, found.word, &mut linked);
            linked.push_str(found.after);
            text = Cow::Owned(linked);
            // The text has changed: its words are read again for the terms
            // after this one.
            lower = text.to_lowercase();
            places = self.places(&text, &lower);
        }
        out.push_str(&text);
    }

    /// The places in [`Linker::entries`] of the terms that may be found
    /// in `text`, lowercased `lower`: every term when lowercasing changed
    /// the text's length, as a spelling then is matched exactly.
    fn places(&self, text: &str, lower: &str) -> BTreeSet<usize> {
        if lower.len() == text.len() {
            self.index.places(text, lower)
        } else {
            (0..self.entries.len()).collect()
        }
    }

    /// Writes a code span, `rendered`, to `out`: linked when it is exactly
    /// a term (`PATH`, `WAL`), which explains itself like any other
    /// mention.
    pub(crate) fn code(&mut self, code: &str, rendered: &str, out: &mut String) {
        // Only a term with a spelling equal to the code but for the case
        // of its ASCII letters can be named by it.
        let known = self
            .index
            .spelled(code)
            .iter()
            .filter_map(|&place| self.entries.get(place))
            .find(|entry| {
                !self.mentions.seen.contains(&entry.term.word)
                    && entry.spellings.iter().any(|spelling| spelling.names(code))
            });
        match known {
            Some(entry) => self.mentions.mint(entry, rendered, out),
            None => out.push_str(rendered),
        }
    }
}

/// Room for the link a mention is wrapped in, beyond the words of its
/// term's slug and definition.
const MINT_LEN: usize = 96;

/// Which terms have been linked: per chapter (one link per chapter) and
/// in the whole page (each link its own anchor, for the way back).
///
/// The two counts move together: a term is seen in a chapter exactly when
/// a link to it was minted there, and the mentions are numbered from 1 in
/// the order they were linked. The fields are private to the linker for
/// that reason; it starts a chapter with [`Linker::next_chapter`] and gives
/// the count at the end with [`Linker::into_minted`].
#[derive(Default)]
struct Mentions {
    /// Terms already linked in this chapter.
    seen: BTreeSet<String>,
    /// Every mention linked in the page, by term.
    minted: BTreeMap<String, usize>,
}

impl Mentions {
    /// Writes one link to a glossary entry around `shown`, with an anchor
    /// of its own so the entry's "back" returns to exactly this mention.
    fn mint(&mut self, entry: &Entry<'_>, shown: &str, out: &mut String) {
        let term = entry.term;
        self.seen.insert(term.word.clone());
        let nth = self
            .minted
            .entry(term.word.clone())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
        put!(
            out,
            "<a class=\"term\" id=\"t-{}-{nth}\" data-key=\"{}\" href=\"#g-{}\" data-def=\"{}\">{shown}</a>",
            term.slug,
            term.slug,
            term.slug,
            entry.hover
        );
    }
}

/// A character of a word: a letter, a digit or `_`.
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
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
        // Chinese, Japanese and Thai put no spaces between words: a term
        // written in them stands as a word wherever it stands.
        let open_start = self.word.chars().next().is_some_and(unspaced);
        let open_end = self.word.chars().next_back().is_some_and(unspaced);
        let joined_before =
            !open_start && self.before.chars().next_back().is_some_and(is_word_char);
        let joined_after = !open_end && self.after.chars().next().is_some_and(is_word_char);
        let inside_tag = self.before.rfind('<') > self.before.rfind('>');
        let inside_link = self.before.rfind("<a ") > self.before.rfind("</a>");
        !(joined_before || joined_after || inside_tag || inside_link)
    }
}

/// The endings a mention may add to a term: "entity" is also "entities".
const PLURALS: [&str; 2] = ["es", "s"];

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

#[cfg(test)]
mod tests {
    use super::{Linker, MINT_LEN, Term};
    use crate::text::{esc, put};

    /// A term spelled `word`, also `also`.
    fn term(word: &str, also: &[&str]) -> Term {
        Term {
            word: word.to_owned(),
            said: String::from("<p>Meant.</p>"),
            plain: String::from("Meant."),
            slug: word
                .to_lowercase()
                .replace(|ch: char| !ch.is_alphanumeric(), "-"),
            shared: false,
            also: also.iter().map(|spelling| (*spelling).to_owned()).collect(),
        }
    }

    /// `text` linked as before the index: every term not yet linked in the
    /// chapter is looked for, in order.
    fn link_trying_every_term(every: &mut Linker<'_>, escaped: &str) -> String {
        let mut text = escaped.to_owned();
        for entry in &every.entries {
            if every.mentions.seen.contains(&entry.term.word) {
                continue;
            }
            let lower = text.to_lowercase();
            let Some(found) = entry
                .spellings
                .iter()
                .filter_map(|spelling| spelling.find(&text, &lower))
                .min_by_key(|found| found.before.len())
            else {
                continue;
            };
            let mut linked = String::with_capacity(text.len() + MINT_LEN);
            linked.push_str(found.before);
            every.mentions.mint(entry, found.word, &mut linked);
            linked.push_str(found.after);
            text = linked;
        }
        text
    }

    /// A code span written as before the index: compared with every term
    /// not yet linked in the chapter, in order.
    fn code_trying_every_term(every: &mut Linker<'_>, code: &str) -> String {
        let rendered = format!("<code>{}</code>", esc(code));
        let known = every.entries.iter().find(|entry| {
            !every.mentions.seen.contains(&entry.term.word)
                && entry.spellings.iter().any(|spelling| spelling.names(code))
        });
        let mut out = String::new();
        match known {
            Some(entry) => every.mentions.mint(entry, &rendered, &mut out),
            None => out.push_str(&rendered),
        }
        out
    }

    /// A text is searched only for the terms it names: the work of a text
    /// does not grow with the glossary.
    ///
    /// # Panics
    ///
    /// When, in a glossary of 20,000 terms and a few others, a text naming
    /// four of them (one in the plural, one an acronym) and one written
    /// without spaces is to be searched for any other term, or a text
    /// naming none for any.
    #[test]
    fn a_text_is_searched_only_for_the_terms_it_names() {
        let mut terms: Vec<Term> = (0_u32..20_000)
            .map(|number| term(&format!("Gizmo{number}"), &[]))
            .collect();
        terms.extend([
            term("Entity", &[]),
            term("PATH", &[]),
            term("C++", &[]),
            term("索引", &[]),
        ]);
        let linker = Linker::new(&terms);
        let places = |text: &str| -> Vec<usize> {
            let escaped = esc(text);
            linker
                .places(&escaped, &escaped.to_lowercase())
                .into_iter()
                .collect()
        };
        assert_eq!(
            places("The gizmo7 keeps entities on the PATH, and so does C++ with gizmo19999."),
            [7, 19_999, 20_000, 20_001, 20_002]
        );
        assert_eq!(places("见索引。"), [20_003]);
        assert_eq!(places("Nothing the glossary names."), Vec::<usize>::new());
    }

    /// The terms the index finds are linked exactly as when every term was
    /// looked for in every text and compared with every code span.
    ///
    /// # Panics
    ///
    /// When any of the texts below (plurals, acronyms as written and not,
    /// spellings with an ending or a gloss, characters escaped, letters
    /// whose lowercase is longer, words in scripts without spaces, words
    /// inside other words, in tags and in links), or any of their words as
    /// a code span, is linked otherwise than by trying every term, or the
    /// count of links differs.
    #[test]
    fn the_index_links_what_trying_every_term_links() {
        let terms = [
            term("Entity", &[]),
            term("PATH", &["PATHS"]),
            term("Sidecar / blob", &[]),
            term("Unit (passage)", &[]),
            term("AT&T", &[]),
            term(".NET", &[]),
            term("C++", &[]),
            term("_private", &[]),
            term("ΑΣ", &[]),
            term("索引", &[]),
            term("Rust", &["rustacean"]),
            term("Kelvin", &[]),
            term("data", &["datum"]),
            term("a\u{3a3}", &[]),
        ];
        let texts = [
            "Entities and an entity, the PATH and the path, PATHs and PATHS.",
            "A sidecar is a blob; units are passages: unit (passage).",
            "AT&T sells .NET and C++ with _private parts, not __private_ ones.",
            "ΑΣ'Β and ΑΣ and ας and ΑΣΣ.",
            "这是索引，索引在这里; Rust是 and Rust_ and rusts and rustaceans.",
            "\u{212A}elvin and Kelvin: the Kelvin sign is longer in lowercase.",
            "a\u{3a3}'b is a word before a quote, and \u{130} is longer in lowercase.",
            "<a href=\"#x\">Entity</a> then entity, <i title=\"data\">data</i> and data_set and datums.",
            "",
            "PATH",
            "path Path entity ENTITY c++ C++ datum DATUM 索引 AT&T x _PRIVATE .net",
        ];
        for (escape, chapters) in [(true, false), (true, true), (false, false)] {
            let mut indexed = Linker::new(&terms);
            let mut every = Linker::new(&terms);
            let mut by_index = String::new();
            let mut by_every = String::new();
            for text in texts {
                if chapters {
                    indexed.next_chapter();
                    every.next_chapter();
                }
                let escaped = if escape { esc(text) } else { text.to_owned() };
                indexed.link(&escaped, &mut by_index);
                by_every.push_str(&link_trying_every_term(&mut every, &escaped));
                for code in text.split_whitespace() {
                    let rendered = format!("<code>{}</code>", esc(code));
                    indexed.code(code, &rendered, &mut by_index);
                    by_every.push_str(&code_trying_every_term(&mut every, code));
                }
                put!(by_index, "\n");
                put!(by_every, "\n");
            }
            assert_eq!(by_index, by_every);
            assert_eq!(indexed.into_minted(), every.into_minted());
        }
    }
}
