// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Files around a page: the EPUB made beside it, what a document may read
//! into its page, the limits on what is embedded.

use super::Scratch;
use crate::config::Settings;
use crate::render_doc;

/// A ZIP file's first local header is 30 bytes; its first file's name
/// follows.
const ZIP_HEADER: usize = 30;

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
        false,
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
        false,
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
        false,
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

#[test]
fn limits_are_lifted_only_on_the_command_line() {
    let mut settings = Settings::default();
    assert!(
        settings.set("max_embed", "999").is_err(),
        "front matter cannot lift a limit"
    );
    assert_eq!(crate::megabytes("50"), Ok(50 << 20_i32));
    assert!(crate::megabytes("0").is_err() && crate::megabytes("x").is_err());
}
