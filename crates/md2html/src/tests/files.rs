// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Files around a page: the EPUB made beside it and what a document may
//! read into its page.

use super::{Scratch, not_unicode};
use crate::config::Settings;
use crate::document::{Shape, render_doc};

/// A ZIP file's first local header is 30 bytes; its first file's name
/// follows.
const ZIP_HEADER: usize = 30;

/// A document that asks for an EPUB gets a valid one, offered by its page.
///
/// # Panics
///
/// When no EPUB is made; when it is not a ZIP whose first file is the
/// uncompressed `mimetype` of an EPUB; when the container, package,
/// navigation, content, stylesheet or image file is missing from it; when
/// it does not say its language, mark the footnote and its reference, or
/// point at its image; when an HTML entity or a script reaches it; when the
/// page does not offer `book.epub` for download; or when the scratch files
/// cannot be written.
#[test]
fn an_epub_holds_the_page_as_a_book() {
    let scratch = Scratch::new("epub");
    let text = "---\nepub: true\nlang: de\n---\n# Ein Buch\n\nText&nbsp;mit[^1] einem Bild:\n\n![Ein Punkt](dot.svg)\n\n## 1. Kapitel\n\n```rust\nlet x = 1 < 2;\n```\n\n[^1]: Die Fußnote.\n";
    let path = scratch.write("book.md", text);
    scratch.write(
        "dot.svg",
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 2 2\"><circle cx=\"1\" cy=\"1\" r=\"1\"/></svg>",
    );
    let out = scratch.path().join("book.html");
    let page = render_doc(
        text,
        Some(&path),
        Some(&out),
        &Settings::default(),
        Shape::Page,
        None,
    );
    let book = page.epub.expect("an epub was asked for");
    let (header, first_file) = book.split_at_checked(ZIP_HEADER).expect("a zip header");
    assert!(header.starts_with(b"PK\x03\x04"), "a zip file");
    assert!(
        first_file.starts_with(b"mimetypeapplication/epub+zip"),
        "its first file says what it is, unpacked"
    );
    let raw = String::from_utf8_lossy(&book);
    for name in [
        "META-INF/container.xml",
        "OEBPS/package.opf",
        "OEBPS/nav.xhtml",
        "OEBPS/content.xhtml",
        "OEBPS/style.css",
        "OEBPS/images/img-1.svg",
    ] {
        assert!(raw.contains(name), "{name}");
    }
    assert!(raw.contains("<dc:language>de</dc:language>"));
    assert!(raw.contains("epub:type=\"noteref\"") && raw.contains("epub:type=\"footnote\""));
    assert!(raw.contains("src=\"images/img-1.svg\""));
    assert!(
        !raw.contains("&nbsp;") && !raw.contains("<script"),
        "XML's references only, no scripts"
    );
    assert!(
        page.html.contains("download=\"book.epub\""),
        "the page offers its EPUB"
    );
}

/// A stylesheet beside the document is read whatever the case of its
/// extension; a file of another kind is refused and named.
///
/// # Panics
///
/// When `Look.CSS` is not read into the page, when `notes.txt` is, or when
/// the author is not told it is not a `.css` file; or when the scratch files
/// cannot be written.
#[test]
fn a_stylesheet_is_known_by_its_extension_in_any_case() {
    let scratch = Scratch::new("css-case");
    scratch.write("Look.CSS", "body{--look:1}");
    scratch.write("notes.txt", "body{--notes:1}");
    for (css, read) in [("Look.CSS", true), ("notes.txt", false)] {
        let md = format!("---\ncss: {css}\n---\n# L\n");
        let path = scratch.write("doc.md", &md);
        let page = render_doc(
            &md,
            Some(&path),
            Some(&scratch.path().join("doc.html")),
            &Settings::default(),
            Shape::Page,
            None,
        );
        assert_eq!(
            page.html.contains("--look") || page.html.contains("--notes"),
            read,
            "{css}"
        );
        let refused = page.warnings.iter().any(|warning| {
            warning
                .message
                .contains(&format!("{css} is not a .css file"))
        });
        assert_eq!(refused, !read, "{css}");
    }
}

/// A page whose file name is not Unicode is still made, shows its name
/// with U+FFFD, and says that no link can name it: no download links, and
/// no language versions looked for under an empty name.
///
/// # Panics
///
/// When the page does not say so, offers a download, links `.de.md` or
/// `.fr.md` (what an empty stem finds) as its versions, or does not show its
/// name; or when the scratch files cannot be written.
#[test]
fn a_page_whose_name_is_not_unicode_says_no_link_can_name_it() {
    let scratch = Scratch::new("odd-name");
    scratch.write(".de.md", "# De\n");
    scratch.write(".fr.md", "# Fr\n");
    let source = scratch.path().join(not_unicode("", ".md"));
    let out = scratch.path().join(not_unicode("", ".html"));
    let md = "---\nepub: true\npdf: true\n---\n# Odd\n";
    let page = render_doc(
        md,
        Some(&source),
        Some(&out),
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert!(
        page.warnings.iter().any(|warning| warning
            .message
            .starts_with("the file name \u{fffd}.md is not valid Unicode, so no link can name it")),
        "{:?}",
        page.warnings
    );
    assert!(!page.html.contains("download="), "no download link");
    assert!(
        !page.html.contains(".de.html") && !page.html.contains(".fr.html"),
        "no versions under an empty name"
    );
    assert!(page.html.contains("\u{fffd}.md"), "the name is shown");
}

/// A document cannot read a file outside its folder into its page.
///
/// # Panics
///
/// When a stylesheet from the parent folder, an absolute head file, or a
/// text file posing as an image is read into the page (as itself or in
/// base64); when the author is not told that a file is outside the
/// document's folder, is not a `.html` file, or is not an image; or when the
/// scratch files cannot be written.
#[test]
fn a_document_cannot_read_files_outside_its_folder() {
    let scratch = Scratch::new("leak");
    scratch.write("secret.css", "body{--secret:1}");
    scratch.write("doc/not-an-image.png", "TOP-SECRET");
    let md = "---\nembed: true\ncss: ../secret.css\nhead: /etc/hosts\n---\n# L\n\n![x](not-an-image.png)\n";
    let path = scratch.write("doc/leak.md", md);
    let out = scratch.path().join("doc/leak.html");
    let page = render_doc(
        md,
        Some(&path),
        Some(&out),
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert!(
        !page.html.contains("--secret") && !page.html.contains("VE9QLVNFQ1JFVA"),
        "nothing read in"
    );
    let said: Vec<&str> = page
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(
        said.iter()
            .any(|message| message.contains("outside the document's folder")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|message| message.contains("not a .html")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|message| message.contains("not an image")),
        "{said:?}"
    );
}

/// An image is put inside the page, or packed into its book, only from
/// the document's folder: not from the page's own folder, which the
/// command line chose and may be any folder at all.
///
/// # Panics
///
/// When a PNG found only beside the page is embedded or packed, or the
/// author is not told it is not embedded; or when the scratch files cannot
/// be written.
#[test]
fn an_image_beside_the_page_only_is_not_read_into_it() {
    let scratch = Scratch::new("beside-page");
    scratch.write(
        "out/pic.png",
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR-PRIVATE-PHOTO",
    );
    let md = "---\nembed: true\nepub: true\n---\n# P\n\n![a](pic.png)\n";
    let path = scratch.write("doc/p.md", md);
    let page = render_doc(
        md,
        Some(&path),
        Some(&scratch.path().join("out/p.html")),
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert!(!page.html.contains("data:image/png"), "not embedded");
    let book = page.epub.expect("a book was asked for");
    assert!(
        !book
            .windows(b"PRIVATE-PHOTO".len())
            .any(|window| window == b"PRIVATE-PHOTO"),
        "not packed"
    );
    assert!(
        page.warnings
            .iter()
            .any(|warning| warning.message.contains("outside the document's folder")),
        "{:?}",
        page.warnings
    );
}

/// A stylesheet named many times is read into the page once, so a few
/// bytes of front matter cannot make a page many times the size of its
/// files.
///
/// # Panics
///
/// When the stylesheet, named three ways, stands in the page other than
/// once; or when the scratch files cannot be written.
#[test]
fn a_stylesheet_named_many_times_is_read_once() {
    let scratch = Scratch::new("css-once");
    scratch.write("a.css", "body{--once:1}");
    let md = "---\ncss: a.css, a.css, ./a.css\n---\n# P\n";
    let path = scratch.write("p.md", md);
    let page = render_doc(
        md,
        Some(&path),
        None,
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert_eq!(
        page.html.matches("--once:1").count(),
        1,
        "{:?}",
        page.warnings
    );
}

/// A translation's original, found by the translation's name, is read only
/// when it lies inside the folder: one that links out of it is not, so
/// nothing of it (its language) reaches the page.
///
/// # Panics
///
/// When the outside file's language reaches the translation's page, or
/// the author is not told the original is outside the folder; or when the
/// scratch files or the link cannot be written.
#[cfg(unix)]
#[test]
fn an_original_that_links_out_of_the_folder_is_not_read() {
    let scratch = Scratch::new("original-out");
    let secret = scratch.write("outside/secret.md", "---\nlang: secretlang\n---\n# S\n");
    scratch.write("doc/n.de.md", "# Seite\n");
    std::os::unix::fs::symlink(&secret, scratch.path().join("doc/n.md")).unwrap();
    let md = "# Seite\n";
    let path = scratch.path().join("doc/n.de.md");
    let page = render_doc(
        md,
        Some(&path),
        Some(&scratch.path().join("doc/n.de.html")),
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert!(!page.html.contains("secretlang"), "nothing of it read");
    assert!(
        page.warnings
            .iter()
            .any(|warning| warning.message.contains("outside the document's folder")),
        "{:?}",
        page.warnings
    );
}

/// An image or a link whose path ends in a separator names a folder, on
/// every system: Windows would open `pic.png/` as the file `pic.png`, so
/// md2html's own rule decides, not the file system.
///
/// # Panics
///
/// When `pic.png/` is embedded, when the author is not told it does not
/// exist next to the page, when `notes.md/` is turned into a link to a page,
/// or when the plain `pic.png` is not embedded.
#[test]
fn a_path_ending_in_a_separator_names_a_folder() {
    let scratch = Scratch::new("trailing");
    std::fs::write(scratch.path().join("pic.png"), b"\x89PNG\r\n\x1a\n....").unwrap();
    scratch.write("notes.md", "# Notes\n");
    let md = "---\nembed: true\n---\n# T\n\n![a](pic.png/) ![b](pic.png) [n](notes.md/)\n";
    let path = scratch.write("page.md", md);
    let out = scratch.path().join("page.html");
    let page = render_doc(
        md,
        Some(&path),
        Some(&out),
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert_eq!(
        page.html.matches("data:image/png;base64,").count(),
        1,
        "only pic.png"
    );
    assert!(page.html.contains("src=\"pic.png/\""), "left as written");
    assert!(
        page.html.contains("href=\"notes.md/\""),
        "not a page's link"
    );
    assert!(
        page.warnings
            .iter()
            .any(|warning| warning.message == "`pic.png/` does not exist next to this page"),
        "{:?}",
        page.warnings
    );
}

/// An image path whose escapes are not UTF-8 names no file.
///
/// # Panics
///
/// When `%ff.png` is changed or embedded instead of left as written, when
/// the file `\u{fffd}.png` is not embedded by its own escaped name or is
/// embedded twice, when the author is not told that `%ff.png` does not
/// exist, or when the scratch files cannot be written.
#[test]
fn an_image_path_that_is_not_utf8_names_no_file() {
    let scratch = Scratch::new("not-utf8");
    // The name that replacing the bytes with U+FFFD would make.
    scratch.write("\u{fffd}.png", b"\x89PNG\r\n\x1a\n");
    let md = "---\nembed: true\n---\n# I\n\n![a](%ff.png) ![b](%EF%BF%BD.png)\n";
    let path = scratch.write("page.md", md);
    let out = scratch.path().join("page.html");
    let page = render_doc(
        md,
        Some(&path),
        Some(&out),
        &Settings::default(),
        Shape::Page,
        None,
    );
    assert!(page.html.contains("src=\"%ff.png\""), "left as written");
    assert_eq!(
        page.html.matches("src=\"data:image/png").count(),
        1,
        "only the file's own name finds it"
    );
    let said: Vec<&str> = page
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(
        said.contains(&"`%ff.png` does not exist next to this page"),
        "{said:?}"
    );
}
