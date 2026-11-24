// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Whole pages, rendered from Markdown and read back.

mod anchors;
mod balanced;
mod determinism;
mod escaping;
mod files;
mod generated;
mod glossary;
mod hostile;
mod markdown;
mod measure;
mod names;
mod reading;
mod spans;
mod structure;
mod translations;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::document::{Shape, render_doc};
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
    render_doc(text, None, None, settings, Shape::Page, None).html
}

/// The HTML of the Markdown alone (`--fragment`), with no shared glossary.
fn fragment(text: &str) -> String {
    fragment_with(text, &own_glossary_only())
}

/// The HTML of the Markdown alone (`--fragment`) with `settings`.
fn fragment_with(text: &str, settings: &Settings) -> String {
    render_doc(text, None, None, settings, Shape::Fragment, None).html
}

/// What the tool tells the author about a document.
fn warnings(text: &str) -> Vec<String> {
    render_doc(text, None, None, &own_glossary_only(), Shape::Page, None)
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
///
/// # Panics
///
/// When `html` has no `<article` or the article is not closed.
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
///
/// # Panics
///
/// When a block element of [`BLOCKS`] closes while another opened after it
/// is still open, closes without being open, or is left open at the end;
/// the message names the element and `case`.
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

/// Elements HTML writes without an end tag.
const VOID: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// The frame of a page: every element outside md2html's article.
///
/// Each is `depth name#id`, the article itself among them and what it
/// holds left out. On the way, every element of the page, the article's
/// included, is held to nest: an XHTML book's too, and a fragment's.
///
/// # Panics
///
/// When an end tag does not close the element opened last, or an element is
/// left open at the end; the message names it and `case`.
fn frame(html: &str, case: &str) -> Vec<String> {
    let mut open: Vec<&str> = Vec::new();
    let mut frame = Vec::new();
    let mut article: Option<usize> = None;
    let mut scan = Scan::new(html);
    while scan.take_until("<").is_some() {
        if scan.eat_str("<!--") {
            scan.take_until("-->");
            continue;
        }
        if scan.eat_str("<!") || scan.eat_str("<?") {
            scan.take_until(">");
            continue;
        }
        scan.bump();
        let closing = scan.eat('/');
        let name = scan.take_while(|ch| ch.is_ascii_alphanumeric());
        let rest = scan.take_until(">").expect("a tag ends");
        if closing {
            assert_eq!(
                open.pop(),
                Some(name),
                "</{name}> closes out of order in {case:?}"
            );
            if article == Some(open.len()) {
                article = None;
            }
            continue;
        }
        let id = rest
            .split_once(" id=\"")
            .and_then(|(_, after)| after.split_once('"'))
            .map_or("", |(id, _)| id);
        if article.is_none() {
            frame.push(format!("{} {name}#{id}", open.len()));
        }
        if rest.ends_with('/') || VOID.contains(&name) {
            continue;
        }
        if name == "article" && id == "main" && article.is_none() {
            article = Some(open.len());
        }
        open.push(name);
        if matches!(name, "script" | "style") {
            scan.take_until(&format!("</{name}"));
        }
    }
    assert!(open.is_empty(), "left open {open:?} in {case:?}");
    frame
}

/// A name that is not Unicode: `before`, one unit no text holds, `after`.
///
/// On Linux and macOS a name is bytes, and the unit is the byte `0xE9`
/// alone; on Windows a name is UTF-16, and the unit is a surrogate without
/// its pair. Either is shown as U+FFFD.
#[cfg(unix)]
pub(crate) fn not_unicode(before: &str, after: &str) -> OsString {
    use std::os::unix::ffi::OsStringExt as _;
    OsString::from_vec([before.as_bytes(), b"\xe9", after.as_bytes()].concat())
}

/// A name that is not Unicode: `before`, one unit no text holds, `after`.
///
/// On Linux and macOS a name is bytes, and the unit is the byte `0xE9`
/// alone; on Windows a name is UTF-16, and the unit is a surrogate without
/// its pair. Either is shown as U+FFFD.
#[cfg(windows)]
pub(crate) fn not_unicode(before: &str, after: &str) -> OsString {
    use std::os::windows::ffi::OsStringExt as _;
    let units: Vec<u16> = before
        .encode_utf16()
        .chain([0xD800])
        .chain(after.encode_utf16())
        .collect();
    OsString::from_wide(&units)
}

/// A name a URL would read as something else, and how a URL writes it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Awkward {
    /// The name, without an extension.
    pub(crate) name: &'static str,
    /// The name as one segment of a URL's path.
    pub(crate) url: &'static str,
}

/// Names that a link must escape to name their file, each one a file
/// name this system allows.
///
/// A space, `#` and `?` (which would end the path), `%` (which would start
/// an escape), `&` and `'` (which a URL may hold as they are), `"` and `<`,
/// letters beyond ASCII, a name that already looks escaped, and one whose
/// `:` would make the link a `javascript:` URL. Windows allows no `?`,
/// `"`, `<` or `:` in a name, so there those four are left out.
pub(crate) fn awkward_names() -> Vec<Awkward> {
    let all = [
        ("a b", "a%20b"),
        ("a#b", "a%23b"),
        ("a?b", "a%3Fb"),
        ("a%b", "a%25b"),
        ("a&b", "a&b"),
        ("a'b", "a'b"),
        ("a\"b", "a%22b"),
        ("a<b", "a%3Cb"),
        ("\u{e9}t\u{e9} \u{65e5}", "%C3%A9t%C3%A9%20%E6%97%A5"),
        ("a%20b", "a%2520b"),
        ("javascript:alert(1)", "javascript%3Aalert(1)"),
    ];
    all.into_iter()
        .filter(|(name, _)| !cfg!(windows) || !name.contains(['?', '"', '<', ':']))
        .map(|(name, url)| Awkward { name, url })
        .collect()
}

/// The real tags of `html`, lowercased, from `<` to `>`: escaped text
/// (`&lt;img onerror=...`) is harmless words and is not among them.
fn tags(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut scan = Scan::new(html);
    while scan.take_until("<").is_some() {
        let tag = match scan.take_until(">") {
            Some(tag) => {
                scan.bump();
                format!("{tag}>")
            }
            None => scan.take_rest().to_owned(),
        };
        out.push(tag.to_ascii_lowercase());
    }
    out
}

/// The head and the article of a page, without md2html's own scripts.
fn authored_part(html: &str) -> String {
    let head = html.split_once("<style>").map_or("", |(head, _)| head);
    let hero = "<header class=\"hero\"";
    let body = html
        .split_once(hero)
        .map_or_else(|| String::from(html), |(_, after)| format!("{hero}{after}"));
    let article = body
        .split_once("<script id=\"idx\"")
        .map_or(body.as_str(), |(article, _)| article);
    format!("{head}{article}")
}

/// The first real tag of a page's head or article that would run code.
///
/// That is a `<script>` or `<iframe>`, an event handler, or an attribute
/// whose value is a `javascript:` URL or holds one in a `url()`; `None`
/// when there is none. md2html's own scripts are not looked at, and text
/// in an attribute's quotes is text, whatever words it holds.
fn running_code(html: &str) -> Option<String> {
    tags(&authored_part(html)).into_iter().find(|tag| {
        let mut scan = Scan::new(tag);
        scan.eat('<');
        let name = scan.take_while(|ch| !ch.is_whitespace() && ch != '>');
        if matches!(name, "script" | "iframe") {
            return true;
        }
        attributes(&mut scan).into_iter().any(|(attr, value)| {
            let bare = value.trim_start();
            attr.starts_with("on")
                || bare.starts_with("javascript:")
                || bare.contains("url(javascript")
        })
    })
}

/// The attributes of a tag read from after its name: each name with its
/// value, quoted or bare (empty without one).
fn attributes<'tag>(scan: &mut Scan<'tag>) -> Vec<(&'tag str, &'tag str)> {
    let mut found = Vec::new();
    loop {
        scan.take_while(char::is_whitespace);
        let name = scan.take_while(|ch| !ch.is_whitespace() && !matches!(ch, '=' | '>' | '/'));
        if name.is_empty() {
            if scan.bump().is_none() {
                return found;
            }
            continue;
        }
        let value = if scan.eat('=') {
            match scan.peek() {
                Some(quote @ ('"' | '\'')) => {
                    scan.bump();
                    let inside = scan.take_while(|ch| ch != quote);
                    scan.bump();
                    inside
                }
                _ => scan.take_while(|ch| !ch.is_whitespace() && ch != '>'),
            }
        } else {
            ""
        };
        found.push((name, value));
    }
}

/// A folder of its own for one test, removed when the test is over.
struct Scratch {
    /// The folder.
    dir: PathBuf,
}

impl Scratch {
    /// A new, empty folder named for the test.
    ///
    /// # Panics
    ///
    /// When the folder cannot be created in the temporary directory.
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
    ///
    /// # Panics
    ///
    /// When the folders on the way to the file, or the file itself, cannot be
    /// written.
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
