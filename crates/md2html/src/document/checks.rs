// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading a finished page back: its links that lead nowhere, and a lead
//! cut short for the description.

use alloc::collections::BTreeSet;

use crate::{inline, text};

/// The in-page links of a page that point at no anchor on it.
pub(super) fn broken_anchors(html: &str) -> Vec<String> {
    let mut ids = BTreeSet::new();
    let mut unread = html;
    while let Some((_, after)) = unread.split_once(" id=\"") {
        if let Some((id, _)) = after.split_once('"') {
            ids.insert(id);
        }
        unread = after;
    }
    let mut missing: Vec<String> = Vec::new();
    let mut links = html
        .split_once("<article")
        .map_or(html, |(_, article)| article);
    while let Some((_, after)) = links.split_once("href=\"#") {
        let Some((target, _)) = after.split_once('"') else {
            break;
        };
        let id = inline::decode_entities(target);
        if !id.is_empty()
            && !ids.contains(id.as_str())
            && !ids.contains(text::url_attr(&id).as_str())
            && !missing.contains(&id)
        {
            missing.push(id);
        }
        links = after;
    }
    missing
}

/// Text cut at a word to about `max` characters.
pub(super) fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    let words = cut
        .rsplit_once(' ')
        .map_or(cut.as_str(), |(words, _)| words);
    format!("{}…", words.trim_end_matches([',', ';', ':', '.']))
}

#[cfg(test)]
mod tests {
    use super::{broken_anchors, shorten};

    /// A link to an anchor no element has is found, and reported once.
    ///
    /// # Panics
    ///
    /// When a link to an id the page has, or the empty `#`, is reported; when
    /// two links to the missing `#gone` are not reported once; or when an
    /// attribute left unclosed at the end of the page is reported.
    #[test]
    fn links_to_nowhere_are_found_once() {
        let html = "<p id=\"top\"></p><article><a href=\"#top\"></a><a href=\"#gone\"></a><a href=\"#gone\"></a><a href=\"#\"></a></article>";
        assert_eq!(broken_anchors(html), ["gone"]);
        assert_eq!(
            broken_anchors("<article><a href=\"#open"),
            Vec::<String>::new()
        );
    }

    /// A lead is cut after a whole word, never inside a character.
    ///
    /// # Panics
    ///
    /// When a text short enough is changed, when a longer one is not cut after
    /// its last whole word with an ellipsis (its comma dropped), or when a cut
    /// falls inside a two-byte character.
    #[test]
    fn a_lead_is_cut_at_a_word() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("one two, three four", 12), "one two…");
        assert_eq!(shorten("ääääää", 3), "äää…");
    }
}
