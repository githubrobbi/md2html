// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Which terms a text may mention: the page's terms by the word or the
//! character their spellings begin with.
//!
//! A text is searched only for the terms it names, so a glossary of
//! thousands of terms costs a text no more than its words. A key only ever
//! lets a term through that might be there: every term a text mentions is
//! among the ones its keys name, and the search itself decides, as it did
//! when every term was tried.

use alloc::collections::{BTreeMap, BTreeSet};

use super::{Entry, PLURALS, is_word_char, unspaced};

/// Which text a spelling is searched in, and so looked up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Case {
    /// The text as written: an acronym matches only so (PATH, not path).
    AsWritten,
    /// The text lowercased: any other spelling matches in any case.
    Lowercased,
}

/// What a spelling is looked up by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Key<'spelling> {
    /// The word it begins with. Where the spelling stands as a word, the
    /// text has that word, or (when the spelling is one word) that word
    /// with an ending a mention may add.
    Word(&'spelling str),
    /// The character it begins with: a spelling that begins with no word,
    /// or has a letter of a script written without spaces between words,
    /// may stand inside a longer word of the text.
    Char(char),
}

impl<'spelling> Key<'spelling> {
    /// The key of `needle`, a spelling as it is searched for. None when it
    /// is empty: an empty spelling is never found.
    pub(super) fn of(needle: &'spelling str) -> Option<Self> {
        let first = needle.chars().next()?;
        if needle.chars().any(unspaced) {
            return Some(Self::Char(first));
        }
        let word = needle
            .split(|ch: char| !is_word_char(ch))
            .next()
            .filter(|word| !word.is_empty());
        Some(word.map_or(Self::Char(first), Self::Word))
    }
}

/// The terms by the keys of their spellings, in one [`Case`].
#[derive(Default)]
struct Lookup {
    /// By a word: the places of the terms, in order.
    words: BTreeMap<String, Vec<usize>>,
    /// By a character.
    chars: BTreeMap<char, Vec<usize>>,
}

impl Lookup {
    /// Files the term at `place` under `key`.
    fn add(&mut self, key: Key<'_>, place: usize) {
        let places = match key {
            Key::Word(word) => self.words.entry(word.to_owned()).or_default(),
            Key::Char(ch) => self.chars.entry(ch).or_default(),
        };
        add_place(places, place);
    }

    /// Adds to `places` the terms `text` has a key of: one of its words,
    /// one of its characters.
    fn places_into(&self, text: &str, places: &mut BTreeSet<usize>) {
        if !self.words.is_empty() {
            for word in words(text) {
                places.extend(self.words.get(word).into_iter().flatten());
            }
        }
        if !self.chars.is_empty() {
            for ch in text.chars() {
                places.extend(self.chars.get(&ch).into_iter().flatten());
            }
        }
    }
}

/// The page's terms by what a text must have for each to be found in it,
/// and by what a code span must be to name each.
#[derive(Default)]
pub(super) struct TermIndex {
    /// The spellings searched in the text as written.
    as_written: Lookup,
    /// The spellings searched in the text lowercased.
    lowercased: Lookup,
    /// By a whole spelling as written, its ASCII letters lowercased: what
    /// a code span is compared with.
    spellings: BTreeMap<String, Vec<usize>>,
}

impl TermIndex {
    /// The index of `entries`, by their places in the list.
    pub(super) fn new(entries: &[Entry<'_>]) -> Self {
        let mut index = Self::default();
        for (place, entry) in entries.iter().enumerate() {
            for spelling in &entry.spellings {
                match spelling.key() {
                    Some((Case::AsWritten, key)) => index.as_written.add(key, place),
                    Some((Case::Lowercased, key)) => index.lowercased.add(key, place),
                    None => {}
                }
                let whole = spelling.written.to_ascii_lowercase();
                add_place(index.spellings.entry(whole).or_default(), place);
            }
        }
        index
    }

    /// The places of the terms `text` may mention, in order; `lower` is
    /// the text lowercased.
    pub(super) fn places(&self, text: &str, lower: &str) -> BTreeSet<usize> {
        let mut places = BTreeSet::new();
        self.as_written.places_into(text, &mut places);
        self.lowercased.places_into(lower, &mut places);
        places
    }

    /// The places of the terms with a spelling `code` may name: equal to
    /// it but for the case of its ASCII letters.
    pub(super) fn spelled(&self, code: &str) -> &[usize] {
        self.spellings
            .get(&code.to_ascii_lowercase())
            .map_or(&[], Vec::as_slice)
    }
}

/// Adds `place` to the end of `places`, once: two spellings of one term
/// may have one key.
fn add_place(places: &mut Vec<usize>, place: usize) {
    if places.last() != Some(&place) {
        places.push(place);
    }
}

/// The words of a text, each as written and without each ending a
/// mention may add to a term: `terms` is also `term`, `boxes` also `box`.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|ch: char| !is_word_char(ch))
        .filter(|word| !word.is_empty())
        .flat_map(|word| {
            core::iter::once(word).chain(
                PLURALS
                    .iter()
                    .filter_map(|ending| word.strip_suffix(ending)),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::Key;

    /// A spelling is keyed by its first word, or by its first character
    /// when it may stand inside a word.
    ///
    /// # Panics
    ///
    /// When a spelling of words is not keyed by its first word (up to a
    /// space, a sign or an escaped character), or one that begins with a
    /// sign, has a letter of a script without spaces or is empty is not
    /// keyed by its first character, or by nothing when empty.
    #[test]
    fn a_spelling_is_keyed_by_its_first_word_or_character() {
        assert_eq!(Key::of("entity"), Some(Key::Word("entity")));
        assert_eq!(Key::of("reading guide"), Some(Key::Word("reading")));
        assert_eq!(Key::of("c++"), Some(Key::Word("c")));
        assert_eq!(Key::of("at&amp;t"), Some(Key::Word("at")));
        assert_eq!(Key::of("_private"), Some(Key::Word("_private")));
        assert_eq!(Key::of(".net"), Some(Key::Char('.')));
        assert_eq!(Key::of("目次"), Some(Key::Char('目')));
        assert_eq!(Key::of("rust言語"), Some(Key::Char('r')));
        assert_eq!(Key::of(""), None);
    }
}
