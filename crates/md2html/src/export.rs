// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Copies of a page for reading elsewhere: a PDF, printed by the Chrome
//! already on the machine, and an EPUB for e-readers, written here.
//!
//! Both start from the finished page, so they say exactly what it says.
//! md2html takes on no dependency for either: the PDF needs a Chrome (or
//! Chromium) and says so plainly when there is none; the EPUB is XHTML,
//! a stylesheet and the page's images in a zip written with the standard
//! library.

mod book;
mod conform;
mod images;
mod legacy;
mod print;
mod xhtml;
mod zip;

pub(crate) use book::{Book, epub};
pub(crate) use print::pdf;

use crate::scan::Scan;

/// The text between two marks, the first of each.
fn between<'text>(text: &'text str, open: &str, close: &str) -> Option<&'text str> {
    let (_, after) = text.split_once(open)?;
    after.split_once(close).map(|(inside, _)| inside)
}

/// What the element that the start tag `open` begins holds, up to its own
/// end tag: elements of the same `name` inside it, which balancing keeps
/// in pairs (`sanitize::Balance`), are passed over with their ends.
fn element<'text>(text: &'text str, open: &str, name: &str) -> Option<&'text str> {
    let (_, after) = text.split_once(open)?;
    let start_tag = format!("<{name}");
    let end_tag = format!("</{name}>");
    let mut scan = Scan::new(after);
    let start = scan.mark();
    let mut inner = 0_usize;
    loop {
        scan.take_until("<")?;
        if scan.starts_with(&end_tag) {
            if inner == 0 {
                return Some(scan.since(start));
            }
            inner -= 1;
        } else if scan
            .rest()
            .strip_prefix(&start_tag)
            .is_some_and(|rest| rest.starts_with(['>', ' ']))
        {
            inner += 1;
        }
        scan.bump();
    }
}

#[cfg(test)]
mod tests {
    use super::element;

    /// An element's contents end at its own end tag, past elements of its
    /// name inside it.
    ///
    /// # Panics
    ///
    /// When the article's contents stop at the end tag of an article inside
    /// it, take in what follows the article, ignore an element whose name
    /// only begins like it (`<articles>`), or are found when the article is
    /// missing or never ends.
    #[test]
    fn an_element_ends_at_its_own_end_tag() {
        let page = "<main><article id=\"main\">a<article>b<article class=\"x\">c</article></article><articles>d</article></main><p>after</p>";
        assert_eq!(
            element(page, "<article id=\"main\">", "article"),
            Some("a<article>b<article class=\"x\">c</article></article><articles>d")
        );
        assert_eq!(
            element("<p>x</p>", "<article id=\"main\">", "article"),
            None
        );
        assert_eq!(
            element(
                "<article id=\"main\">a<article>b</article>",
                "<article id=\"main\">",
                "article"
            ),
            None
        );
    }
}
