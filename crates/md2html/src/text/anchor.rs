// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A heading's anchor as GitHub writes it, and the ids of one page, each
//! given once.
//!
//! GitHub's rule is the one github-slugger 2.0.0 implements
//! (<https://github.com/Flet/github-slugger>, `index.js` and the regular
//! expression `regex.js` that `script/index.js` generates from Unicode's
//! general categories): the text lowercased, every character removed that
//! is not a letter, a mark, a decimal digit, a connector punctuation, a
//! space or a hyphen, each space made a hyphen and nothing collapsed or
//! trimmed; a repeated anchor numbered `-1`, `-2` in order. GitHub's own
//! renderer, asked through its API (`POST /markdown`, 2026-11-25), differs
//! from github-slugger in two places, and md2html writes what GitHub
//! writes: a letter is lowercased alone (`ΟΔΟΣ` gives `οδοσ`, not
//! `οδος`), and the joiners U+200C and U+200D are kept (Unicode's word
//! characters: `Alphabetic`, the marks, the decimal digits, the connector
//! punctuation and `Join_Control`).

use alloc::collections::BTreeMap;
use core::cmp::Ordering;

use super::word::KEPT;

/// Whether an anchor keeps `ch`: a letter (Unicode's `Alphabetic`), a
/// mark, a decimal digit, a connector punctuation or a joiner.
fn is_word(ch: char) -> bool {
    let point = u32::from(ch);
    ch.is_alphabetic()
        || KEPT
            .binary_search_by(|&(first, last)| {
                if last < point {
                    Ordering::Less
                } else if first > point {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .is_ok()
}

/// The anchor GitHub gives a heading whose words are `text`: lowercased,
/// a space a hyphen, every character but a word's, a hyphen and a space
/// left out. `1 · Size vs Size on Disk` is `1--size-vs-size-on-disk`.
pub(crate) fn heading_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch == ' ' {
            out.push('-');
        } else if ch == '-' || is_word(ch) {
            out.push(ch);
        }
    }
    out
}

/// The ids of one page, each given once: a repeated one is numbered as
/// GitHub numbers a repeated heading's anchor.
#[derive(Debug, Default)]
pub(crate) struct Ids {
    /// Every id given, with how many times another asked for it.
    seen: BTreeMap<String, usize>,
}

impl Ids {
    /// An id the page has already, which a later one is numbered around.
    pub(crate) fn hold(&mut self, id: &str) {
        self.seen.entry(id.to_owned()).or_insert(0);
    }

    /// `wanted` when the page has no such id yet, else the first of
    /// `wanted-1`, `wanted-2`... it has not; `taken` says of an id the page
    /// has without holding it. As github-slugger counts: each base counts
    /// its own repeats, so `same`, `same`, `same 1` give `same`, `same-1`,
    /// `same-1-1`.
    pub(crate) fn take(&mut self, wanted: &str, taken: impl Fn(&str) -> bool) -> String {
        let mut id = wanted.to_owned();
        while self.seen.contains_key(&id) || taken(&id) {
            let count = self.seen.entry(wanted.to_owned()).or_insert(0);
            *count = count.saturating_add(1);
            id = format!("{wanted}-{count}");
        }
        self.seen.insert(id.clone(), 0);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::super::word::KEPT;
    use super::{Ids, heading_slug};

    /// The table of kept characters can be searched: its ranges are sorted,
    /// each first no later than its last, and none touches the next.
    ///
    /// # Panics
    ///
    /// When a range is backwards, or does not end before the next begins
    /// with a character left out between them.
    #[test]
    fn the_kept_characters_are_sorted_and_apart() {
        for (first, last) in KEPT {
            assert!(first <= last, "{first:X}..{last:X}");
        }
        for pair in KEPT.windows(2) {
            let [(_, end), (start, _)] = pair else {
                continue;
            };
            assert!(end + 1 < *start, "{end:X} and {start:X} touch");
        }
    }

    /// A heading's anchor is its words as GitHub writes them: nothing
    /// collapsed, nothing trimmed, a joiner and a mark kept.
    ///
    /// # Panics
    ///
    /// When one of the texts gives another anchor than GitHub gives it.
    #[test]
    fn a_slug_is_written_as_github_writes_it() {
        for (text, slug) in [
            ("1 · Size vs Size on Disk", "1--size-vs-size-on-disk"),
            ("--A  b--", "--a--b--"),
            ("snake_case", "snake_case"),
            ("x² ½ Ⅻ", "x--ⅻ"),
            ("ΟΔΟΣ", "οδοσ"),
            ("İ", "i\u{307}"),
            ("a\u{200d}b\u{200c}c\u{200b}d", "a\u{200d}b\u{200c}cd"),
            ("हिन्दी", "हिन्दी"),
            (
                "I ❤\u{fe0f} 1\u{fe0f}\u{20e3}",
                "i-\u{fe0f}-1\u{fe0f}\u{20e3}",
            ),
            ("tab\there\u{a0}nbsp", "tabherenbsp"),
        ] {
            assert_eq!(heading_slug(text), slug, "{text:?}");
        }
    }

    /// Repeats are numbered per base, as github-slugger numbers them, and
    /// an id the page has otherwise is passed over.
    ///
    /// # Panics
    ///
    /// When the ids differ from GitHub's for the same headings, or an id
    /// held or taken otherwise is given again.
    #[test]
    fn repeats_are_numbered_per_base() {
        let mut ids = Ids::default();
        ids.hold("top");
        let given: Vec<String> = ["same", "same", "same-1", "same", "same-1", "top", "fn-1"]
            .into_iter()
            .map(|wanted| ids.take(wanted, |id| id == "fn-1"))
            .collect();
        assert_eq!(given, [
            "same", "same-1", "same-1-1", "same-2", "same-1-2", "top-1", "fn-1-1"
        ]);
    }
}
