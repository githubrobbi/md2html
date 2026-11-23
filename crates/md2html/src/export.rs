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
mod images;
mod print;
mod xhtml;
mod zip;

pub(crate) use book::{Book, epub};
pub(crate) use print::pdf;

/// The text between two marks, the first of each.
fn between<'text>(text: &'text str, open: &str, close: &str) -> Option<&'text str> {
    let (_, after) = text.split_once(open)?;
    after.split_once(close).map(|(inside, _)| inside)
}
