// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The contents of the handbook a page belongs to, at the head of its
//! outline: the chapters in the handbook's order, a part's chapters a step
//! in, the page itself marked, and its own sections under it.

use crate::text::{PathUrl, esc, put};

/// How far in an entry stands at most: deeper parts stand as deep.
const DEEPEST: u8 = 3;

/// A handbook's contents as one of its pages shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Contents {
    /// The entries, in the handbook's order.
    entries: Vec<Entry>,
}

/// One entry of the contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    /// A title over the chapters after it (a part of mdBook's
    /// `SUMMARY.md`), not a link.
    Part(String),
    /// A chapter.
    Chapter(Chapter),
}

/// A chapter in the contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Chapter {
    /// Its page, relative to the page that shows the contents.
    pub(crate) href: PathUrl,
    /// Its title.
    pub(crate) title: String,
    /// How many parts it stands in: 0 for a chapter of the start page.
    pub(crate) depth: u8,
    /// It is the page that shows the contents.
    pub(crate) current: bool,
}

impl Contents {
    /// The contents of `entries`, in order.
    #[must_use]
    pub(crate) const fn new(entries: Vec<Entry>) -> Self {
        Self { entries }
    }

    /// The lists around the page's own outline: the entries up to the
    /// page itself, and those after it (empty when there are none).
    pub(super) fn around(&self) -> (String, String) {
        let at = self
            .entries
            .iter()
            .position(|entry| matches!(entry, Entry::Chapter(chapter) if chapter.current))
            .map_or(self.entries.len(), |here| here.saturating_add(1));
        let (before, after) = self
            .entries
            .split_at_checked(at)
            .unwrap_or((&self.entries, &[]));
        (list(before), list(after))
    }
}

/// The entries as one list; nothing for none.
fn list(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = String::from("<ol class=\"book\">");
    for entry in entries {
        match entry {
            Entry::Part(title) => put!(out, "<li class=\"part\">{}</li>", esc(title)),
            Entry::Chapter(chapter) => {
                let depth = chapter.depth.min(DEEPEST);
                let class = match (depth, chapter.current) {
                    (0, false) => String::new(),
                    (0, true) => String::from(" class=\"here\""),
                    (_, false) => format!(" class=\"d{depth}\""),
                    (_, true) => format!(" class=\"d{depth} here\""),
                };
                let current = if chapter.current {
                    " aria-current=\"page\""
                } else {
                    ""
                };
                put!(
                    out,
                    "<li{class}><a href=\"{}\"{current}>{}</a></li>",
                    chapter.href.attr(),
                    esc(&chapter.title)
                );
            }
        }
    }
    out.push_str("</ol>");
    out
}

#[cfg(test)]
mod tests {
    use super::{Chapter, Contents, Entry};
    use crate::system::relative_url;

    /// A chapter entry.
    ///
    /// # Panics
    ///
    /// When `href` is not a relative path.
    fn chapter(href: &str, title: &str, depth: u8, current: bool) -> Entry {
        Entry::Chapter(Chapter {
            href: relative_url(std::path::Path::new(href)).unwrap(),
            title: title.to_owned(),
            depth,
            current,
        })
    }

    /// The entries up to the page itself stand before its outline, the
    /// rest after it; a part's title is no link; a title is escaped.
    ///
    /// # Panics
    ///
    /// When the lists are split elsewhere, the page is not marked, a depth
    /// is not its class, or a title's `<` is not escaped.
    #[test]
    fn the_page_splits_the_contents_around_its_outline() {
        let contents = Contents::new(vec![
            chapter("README.html", "Book", 0, false),
            Entry::Part(String::from("Part <1>")),
            chapter("a.html", "A", 1, true),
            chapter("b.html", "B", 0, false),
        ]);
        let (before, after) = contents.around();
        assert_eq!(
            before,
            "<ol class=\"book\"><li><a href=\"README.html\">Book</a></li><li class=\"part\">Part &lt;1&gt;</li>\
<li class=\"d1 here\"><a href=\"a.html\" aria-current=\"page\">A</a></li></ol>"
        );
        assert_eq!(
            after,
            "<ol class=\"book\"><li><a href=\"b.html\">B</a></li></ol>"
        );
        let last = Contents::new(vec![chapter("b.html", "B", 0, true)]);
        assert_eq!(last.around().1, "");
    }
}
