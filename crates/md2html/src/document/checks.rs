// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading a finished page back: its links that lead nowhere, and a lead
//! cut short for the description.

use alloc::collections::BTreeSet;

use crate::text;

/// The in-page links of a page that point at no anchor on it, each once,
/// in the order the page first links to it.
///
/// Each link is looked up in the page's ids and in those already reported,
/// both sets: a page linking to many anchors costs in proportion to its
/// links, not to their square.
pub(super) fn broken_anchors(html: &str) -> Vec<String> {
    let ids: BTreeSet<&str> = after_each(html, " id=\"")
        .filter_map(|after| after.split_once('"'))
        .map(|(id, _)| id)
        .collect();
    let article = html
        .split_once("<article")
        .map_or(html, |(_, article)| article);
    let mut missing: Vec<String> = Vec::new();
    let mut reported: BTreeSet<String> = BTreeSet::new();
    for after in after_each(article, "href=\"#") {
        let Some((target, _)) = after.split_once('"') else {
            break;
        };
        let id = text::decode_entities(target);
        if !id.is_empty()
            && !ids.contains(id.as_str())
            && !ids.contains(text::url_attr(&id).as_str())
            && !reported.contains(&id)
        {
            reported.insert(id.clone());
            missing.push(id);
        }
    }
    missing
}

/// What follows each place `marker` is in `text`: one search through the
/// text for all of them.
fn after_each<'text>(text: &'text str, marker: &str) -> impl Iterator<Item = &'text str> {
    text.match_indices(marker)
        .filter_map(move |(at, found)| text.get(at.saturating_add(found.len())..))
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
    use crate::text::put;

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

    /// Sixty thousand links to anchors that are nowhere, each made twice,
    /// are each reported once, in the order the page first makes them.
    ///
    /// The reported ones were a list searched again for every link: a
    /// page like this one compared 3.6 billion pairs of anchors.
    ///
    /// # Panics
    ///
    /// When any of the anchors is not reported, is reported twice or out
    /// of order, or an anchor the page has is reported.
    #[test]
    fn many_links_to_nowhere_are_each_reported_once() {
        const LINKS: usize = 60_000;
        let mut html = String::from("<p id=\"here\"></p><article>");
        for round in 0_u8..2 {
            for number in 0..LINKS {
                put!(
                    html,
                    "<a href=\"#gone-{number}\"></a><a href=\"#here\"></a>"
                );
            }
            put!(html, "<p>round {round}</p>");
        }
        let missing = broken_anchors(&html);
        assert_eq!(missing.len(), LINKS);
        for (number, id) in missing.iter().enumerate() {
            assert_eq!(id, &format!("gone-{number}"));
        }
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
