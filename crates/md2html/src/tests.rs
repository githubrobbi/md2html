// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Whole pages, rendered from Markdown and read back.

mod determinism;
mod files;
mod hostile;
mod markdown;
mod structure;
mod translations;

use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::render_doc;
use crate::scan::Scan;

/// The settings of a page with no shared glossary.
fn own_glossary_only() -> Settings {
    Settings {
        glossary: Some(false),
        ..Settings::default()
    }
}

/// A page as the tool writes it, with no shared glossary.
fn page(text: &str) -> String {
    page_with(text, &own_glossary_only())
}

/// A page as the tool writes it with `settings`.
fn page_with(text: &str, settings: &Settings) -> String {
    render_doc(text, None, None, settings, false, None).html
}

/// What the tool tells the author about a document.
fn warnings(text: &str) -> Vec<String> {
    render_doc(text, None, None, &own_glossary_only(), false, None)
        .warnings
        .into_iter()
        .map(|warning| warning.message)
        .collect()
}

/// The search index of a page.
fn index(html: &str) -> &str {
    let from = html
        .split_once("<script id=\"idx\" type=\"application/json\">")
        .map_or(html, |(_, after)| after);
    from.split_once("</script>")
        .map_or(from, |(index, _)| index)
}

/// The article of a page, its tags included.
fn article(html: &str) -> String {
    let (_, opened) = html.split_once("<article").expect("article");
    let (inside, _) = opened.split_once("</article>").expect("article closed");
    format!("<article{inside}</article>")
}

/// The block elements whose nesting [`assert_nested`] checks.
const BLOCKS: [&str; 16] = [
    "article",
    "header",
    "aside",
    "div",
    "ul",
    "ol",
    "li",
    "table",
    "pre",
    "nav",
    "section",
    "blockquote",
    "figure",
    "p",
    "dl",
    "details",
];

/// Every block element closes in the order it opened.
fn assert_nested(html: &str, case: &str) {
    let mut open: Vec<&str> = Vec::new();
    let mut scan = Scan::new(html);
    while scan.take_until("<").is_some() {
        scan.bump();
        let closing = scan.eat('/');
        let name = scan.take_while(|ch| ch.is_ascii_alphanumeric());
        if !BLOCKS.contains(&name) {
            continue;
        }
        if closing {
            assert_eq!(
                open.pop(),
                Some(name),
                "</{name}> closes out of order in {case:?}"
            );
        } else {
            open.push(name);
        }
    }
    assert!(open.is_empty(), "left open {open:?} in {case:?}");
}

/// A folder of its own for one test, removed when the test is over.
struct Scratch {
    /// The folder.
    dir: PathBuf,
}

impl Scratch {
    /// A new, empty folder named for the test.
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("md2html-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        Self { dir }
    }

    /// Where it is.
    fn path(&self) -> &Path {
        &self.dir
    }

    /// Writes a file into it, the folders on its way made.
    fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a folder in the scratch folder");
        }
        std::fs::write(&path, contents).expect("a file in the scratch folder");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.dir) {
            // A test that is failing already has its own message to show.
            assert!(
                std::thread::panicking(),
                "the scratch folder {} stays: {error}",
                self.dir.display()
            );
        }
    }
}
