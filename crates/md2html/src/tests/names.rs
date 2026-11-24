// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Files whose names a URL would read as something else (a space, `#`,
//! `?`, `%`, letters beyond ASCII): every link md2html makes to such a file
//! from its name escapes the name, so the link leads to the file.

use std::path::Path;

use super::{Awkward, Scratch, awkward_names, own_glossary_only};
use crate::config::Settings;
use crate::document::{Rendered, Shape, render_doc};
use crate::text::esc;

/// The page of the Markdown file `name` in `dir`, as `out` beside it.
///
/// # Panics
///
/// When the file cannot be read.
fn render_in(dir: &Path, name: &str, settings: &Settings) -> Rendered {
    let path = dir.join(name);
    let text = std::fs::read_to_string(&path).expect("written");
    let out = path.with_extension("html");
    render_doc(&text, Some(&path), Some(&out), settings, Shape::Page, None)
}

/// A name's URL as an attribute holds it: `&` written `&amp;`.
fn attr(awkward: Awkward, rest: &str) -> String {
    esc(&format!("{}{rest}", awkward.url))
}

/// The language versions of a page and a translation's notice link each
/// page by its name escaped.
///
/// # Panics
///
/// When the original does not name its translation, as an alternate and in
/// the switch, by the translation's name escaped, or itself by its own;
/// when the translation's notice does not link the original by its name
/// escaped; or when the scratch files cannot be written or read.
#[test]
fn language_versions_link_pages_by_their_names_escaped() {
    for (at, awkward) in awkward_names().into_iter().enumerate() {
        let scratch = Scratch::new(&format!("names-versions-{at}"));
        scratch.write(&format!("{}.md", awkward.name), "# Original\n\nText.\n");
        scratch.write(&format!("{}.de.md", awkward.name), "# Seite\n\nText.\n");
        let original = render_in(
            scratch.path(),
            &format!("{}.md", awkward.name),
            &own_glossary_only(),
        );
        let german = attr(awkward, ".de.html");
        assert!(
            original.html.contains(&format!(
                "<link rel=\"alternate\" hreflang=\"de\" href=\"{german}\">"
            )),
            "{}: the alternate",
            awkward.name
        );
        assert!(
            original.html.contains(&format!(
                "<a href=\"{german}\" hreflang=\"de\" lang=\"de\">"
            )),
            "{}: the switch",
            awkward.name
        );
        assert!(
            original.html.contains(&format!(
                "<a href=\"{}\" hreflang=\"en\" lang=\"en\" aria-current=\"page\">",
                attr(awkward, ".html")
            )),
            "{}: the page itself",
            awkward.name
        );
        let translation = render_in(
            scratch.path(),
            &format!("{}.de.md", awkward.name),
            &own_glossary_only(),
        );
        assert!(
            translation.html.contains("<p class=\"translated\">"),
            "{}: a notice",
            awkward.name
        );
        assert!(
            translation.html.contains(&format!(
                "<a href=\"{}\" hreflang=\"en\" lang=\"en\">",
                attr(awkward, ".html")
            )),
            "{}: the notice links the original",
            awkward.name
        );
    }
}

/// A translation's original named in its front matter, in a folder, is
/// linked by both names escaped.
///
/// # Panics
///
/// When the notice does not link `<folder>/<name>.html` with both names
/// escaped, or the scratch files cannot be written or read.
#[test]
fn a_named_original_is_linked_by_its_path_escaped() {
    for (at, awkward) in awkward_names().into_iter().enumerate() {
        let scratch = Scratch::new(&format!("names-original-{at}"));
        let original = format!("{0}/{0}.md", awkward.name);
        scratch.write(&original, "# Original\n\nText.\n");
        scratch.write(
            "seite.md",
            format!("---\nlang: de\ntranslated_from: '{original}'\n---\n# Seite\n"),
        );
        let page = render_in(scratch.path(), "seite.md", &own_glossary_only());
        let href = esc(&format!("{0}/{0}.html", awkward.url));
        assert!(
            page.html
                .contains(&format!("<a href=\"{href}\" hreflang=\"en\" lang=\"en\">")),
            "{}: {:?}",
            awkward.name,
            page.warnings
        );
    }
}

/// The page's offer to keep it, its book and its PDF link each file by its
/// name escaped, and save it under its own name.
///
/// # Panics
///
/// When a download's link is not the file's name escaped, or its
/// `download` is not the name itself, HTML-escaped.
#[test]
fn downloads_link_files_by_their_names_escaped() {
    for awkward in awkward_names() {
        let out = Path::new("never-site").join(format!("{}.html", awkward.name));
        let html = render_doc(
            "---\nepub: true\npdf: true\n---\n# Book\n\nText.\n",
            None,
            Some(&out),
            &own_glossary_only(),
            Shape::Page,
            None,
        )
        .html;
        for ext in ["html", "epub", "pdf"] {
            let link = format!(
                "<a href=\"{}\" download=\"{}\">",
                attr(awkward, &format!(".{ext}")),
                esc(&format!("{}.{ext}", awkward.name))
            );
            assert!(html.contains(&link), "{link}");
        }
    }
}

/// A book's files have names of md2html's own: the document's name
/// reaches none of its links, so no name needs escaping inside the EPUB
/// (the name shown in the book's text is text, escaped as text).
///
/// # Panics
///
/// When no book is made, it names no file of its own, or one of its
/// `href`, `src` or `full-path` values holds the document's name, as it is
/// or escaped.
#[test]
fn a_book_links_no_name_of_the_document() {
    for (at, awkward) in awkward_names().into_iter().enumerate() {
        let scratch = Scratch::new(&format!("names-book-{at}"));
        scratch.write(
            &format!("{}.md", awkward.name),
            "---\nepub: true\n---\n# Book\n\nText.\n",
        );
        let page = render_in(
            scratch.path(),
            &format!("{}.md", awkward.name),
            &Settings::default(),
        );
        let book = page.epub.expect("a book was asked for");
        let text = String::from_utf8_lossy(&book);
        let mut links = Vec::new();
        for attribute in ["href=\"", "src=\"", "full-path=\""] {
            for (_, after) in text
                .match_indices(attribute)
                .map(|(start, _)| text.split_at(start + attribute.len()))
            {
                links.extend(after.split('"').next());
            }
        }
        assert!(links.contains(&"content.xhtml"), "{links:?}");
        for link in links {
            assert!(
                !link.contains(awkward.name) && !link.contains(awkward.url),
                "{}: {link}",
                awkward.name
            );
        }
    }
}
