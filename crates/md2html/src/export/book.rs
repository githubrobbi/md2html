// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The EPUB: the page's article as one chapter, its outline as the table
//! of contents, its stylesheet and its images, in a zip.

use std::path::Path;

use super::images::{self, Packed, Shelf};
use super::xhtml::xhtml;
use super::zip::Zip;
use super::{between, conform, element, legacy};
use crate::config::Direction;
use crate::scan::Scan;
use crate::text::{esc, hex, put, sha256};

/// What an EPUB says about itself.
pub(crate) struct Book<'book> {
    /// The title, plain.
    pub(crate) title: &'book str,
    /// The language code.
    pub(crate) lang: &'book str,
    /// Which way the lines run.
    pub(crate) direction: Direction,
    /// The author, plain.
    pub(crate) author: Option<&'book str>,
    /// The article's own stylesheet (its diagrams).
    pub(crate) css: &'book str,
    /// Where the page's images are found.
    pub(crate) bases: &'book [&'book Path],
    /// The largest image packed, in bytes (`--max-embed`).
    pub(crate) max_image: u64,
    /// When it is made, in seconds since 1970: the page's moment
    /// (`SOURCE_DATE_EPOCH` holds it still).
    pub(crate) now: u64,
}

/// The page's chrome taken off for an e-reader, and what the reader's own
/// layout replaces.
const EPUB_STYLE: &str = "body{margin:0 4%;background:var(--bg);color:var(--text);font:1em/1.6 var(--sans)}\
article{padding:0}h2{margin-top:2em;padding-top:0;border-top:0;page-break-before:always}\
.anchor,.copy,.back,nav.pages{display:none}\
pre,table,figure,.callout,.diagram{page-break-inside:avoid}\
img,svg{max-width:100%;height:auto}figure{margin:1em 0}\
.term{text-decoration:underline dotted}.credit{margin-top:2em}";

/// Where an EPUB reader looks first: the package's place.
const CONTAINER: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\
<rootfiles><rootfile full-path=\"OEBPS/package.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>";

/// The page as an EPUB 3.
///
/// One chapter file with the article, the outline as the table of
/// contents, the stylesheet, the images. The second value lists what
/// could not be packed, and what the book leaves out or rewrites away.
pub(crate) fn epub(page: &str, book: &Book<'_>) -> (Vec<u8>, Vec<String>) {
    // A page without them gives an empty chapter, an empty outline.
    let article = element(page, "<article id=\"main\">", "article").unwrap_or_default();
    let outline = between(page, "<ol>", "</ol></nav>").unwrap_or_default();
    let packed = images::pack(&strip_chrome(article), Shelf {
        bases: book.bases,
        max: book.max_image,
    });
    let labels = Labels::new(book);
    let (chapter, unlinked) = chapter(&packed);
    let content = labels.document(&format!("<article id=\"main\">{chapter}</article>"));
    let nav = labels.document(&contents(&labels, outline));
    let opf = package(book, &labels, &content, &packed);
    let style = conform::style(&format!(
        "{}{}{}{EPUB_STYLE}{}",
        crate::page::BASE_STYLE,
        crate::page::CONTENT,
        book.css,
        legacy::STYLE
    ));
    let mut problems = packed.problems;
    problems.extend(unlinked);
    let mut archive = Archive {
        zip: Zip::default(),
        problems,
    };
    archive.add("mimetype", b"application/epub+zip");
    archive.add("META-INF/container.xml", CONTAINER.as_bytes());
    archive.add("OEBPS/package.opf", opf.as_bytes());
    archive.add("OEBPS/nav.xhtml", nav.as_bytes());
    archive.add("OEBPS/content.xhtml", content.as_bytes());
    archive.add("OEBPS/style.css", style.as_bytes());
    for image in &packed.images {
        archive.add(&format!("OEBPS/{}", image.name), &image.picture.bytes);
    }
    (archive.zip.finish(), archive.problems)
}

/// The article without what only works on the web page: heading anchors,
/// copy buttons, the previous/next links; other buttons give up their
/// contents.
fn strip_chrome(html: &str) -> String {
    let anchors = remove_all(html, "<a class=\"anchor\"", "</a>");
    let copies = remove_all(&anchors, "<button class=\"copy\"", "</button>");
    let pages = remove_all(&copies, "<nav class=\"pages\"", "</nav>");
    // Any other button: keep what it holds.
    let mut kept = String::with_capacity(pages.len());
    let mut scan = Scan::new(&pages);
    while let Some(before) = scan.take_until("<button") {
        kept.push_str(before);
        // A tag that never ends stays, as text.
        if scan.take_until(">").is_none() {
            break;
        }
        scan.eat('>');
    }
    kept.push_str(scan.take_rest());
    kept.replace("</button>", "")
}

/// `html` with every stretch from `open` to the next `close` removed; an
/// `open` with no `close` after it removes the rest.
fn remove_all(html: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut scan = Scan::new(html);
    while let Some(before) = scan.take_until(open) {
        out.push_str(before);
        if scan.take_until(close).is_none() {
            return out;
        }
        scan.eat_str(close);
    }
    out.push_str(scan.take_rest());
    out
}

/// The packed article as the chapter's XHTML, in EPUB's terms.
///
/// Its footnotes are marked as EPUB's, its code, maths and diagrams run
/// left to right, its links that lead out of the book are taken off their
/// text, its sources that are not in it are left out, and HTML 4's
/// presentational markup becomes what XHTML says instead. The second
/// value names what is left out.
fn chapter(packed: &Packed) -> (String, Vec<String>) {
    let marked = xhtml(&packed.html)
        .replace("class=\"fnref\"", "class=\"fnref\" epub:type=\"noteref\"")
        .replace("<li id=\"fn-", "<li epub:type=\"footnote\" id=\"fn-");
    let held: Vec<&str> = packed
        .images
        .iter()
        .map(|image| image.name.as_str())
        .collect();
    // The book's images are shown, not linked to; links to the images left
    // out are said with them.
    let mut quiet = held.clone();
    quiet.extend(packed.left_out.iter().map(String::as_str));
    let (linked, mut problems) = conform::unlink(&marked, &quiet);
    let (sourced, unsourced) = conform::unsourced(&linked, &held);
    problems.extend(unsourced);
    let (modern, obsolete) = legacy::modernize(&sourced);
    problems.extend(obsolete);
    (namespaces(&conform::left_to_right(&modern)), problems)
}

/// Inline SVG and `MathML` with the namespaces XML needs (HTML implies them).
///
/// An author's `xlink:href` or `xlink:title` needs the `xlink:` namespace too;
/// the author cannot declare one (the filter refuses `xmlns`), so every
/// SVG gets it when the book uses the prefix.
fn namespaces(xhtml: &str) -> String {
    let svg = if xhtml.contains(" xlink:") {
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\""
    } else {
        "<svg xmlns=\"http://www.w3.org/2000/svg\""
    };
    namespaced(xhtml).replace("<svg xmlns=\"http://www.w3.org/2000/svg\"", svg)
}

/// Inline SVG and `MathML` with their own namespaces, each declared once.
fn namespaced(xhtml: &str) -> String {
    let mut out = xhtml.replace("<svg ", "<svg xmlns=\"http://www.w3.org/2000/svg\" ");
    out = out.replace("<svg>", "<svg xmlns=\"http://www.w3.org/2000/svg\">");
    out = out.replace(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns=\"http://www.w3.org/2000/svg\"",
        "<svg xmlns=\"http://www.w3.org/2000/svg\"",
    );
    out = out.replace(
        "<math ",
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\" ",
    );
    out = out.replace(
        "<math>",
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\">",
    );
    out.replace(
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\" xmlns=\"http://www.w3.org/1998/Math/MathML\"",
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"",
    )
}

/// What every file of the book says about its language and title,
/// escaped once.
struct Labels {
    /// The language code.
    lang: String,
    /// The title.
    title: String,
    /// The writing direction, `ltr` or `rtl`.
    dir: &'static str,
}

impl Labels {
    /// The labels of `book`.
    fn new(book: &Book<'_>) -> Self {
        Self {
            lang: esc(book.lang),
            title: esc(book.title),
            dir: book.direction.code(),
        }
    }

    /// An XHTML file of the book, `body` the inside of its `<body>`.
    fn document(&self, body: &str) -> String {
        let Self { lang, title, dir } = self;
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\" dir=\"{dir}\">\
<head><meta charset=\"utf-8\"/><title>{title}</title><link rel=\"stylesheet\" href=\"style.css\"/></head>\
<body>{body}</body></html>"
        )
    }
}

/// The table of contents: the book, then the page's outline, with a
/// chapter's list of sections only when it has some.
fn contents(labels: &Labels, outline: &str) -> String {
    let title = &labels.title;
    let toc = conform::outline(&xhtml(
        &outline.replace("href=\"#", "href=\"content.xhtml#"),
    ));
    format!(
        "<nav epub:type=\"toc\" id=\"toc\"><h1>{title}</h1><ol><li><a href=\"content.xhtml\">{title}</a></li>{toc}</ol></nav>"
    )
}

/// The package document: what the book is, what it holds, in what order.
fn package(book: &Book<'_>, labels: &Labels, content: &str, packed: &Packed) -> String {
    let Labels { lang, title, dir } = labels;
    let uuid = identifier(book, content);
    let modified = modified(book.now);
    let creator = book
        .author
        .map(|author| format!("<dc:creator>{}</dc:creator>", esc(author)))
        .unwrap_or_default();
    let props = properties(content);
    let mut manifest = format!(
        "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\
<item id=\"content\" href=\"content.xhtml\" media-type=\"application/xhtml+xml\"{props}/>\
<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>"
    );
    for (number, image) in packed.images.iter().enumerate() {
        put!(
            manifest,
            "<item id=\"img{number}\" href=\"{}\" media-type=\"{}\"/>",
            image.name,
            image.picture.media.media_type()
        );
    }
    let progression = match book.direction {
        Direction::RightToLeft => " page-progression-direction=\"rtl\"",
        Direction::LeftToRight => "",
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\" xml:lang=\"{lang}\" dir=\"{dir}\">\
<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
<dc:identifier id=\"uid\">urn:uuid:{uuid}</dc:identifier><dc:title>{title}</dc:title><dc:language>{lang}</dc:language>{creator}\
<meta property=\"dcterms:modified\">{modified}</meta></metadata>\
<manifest>{manifest}</manifest><spine{progression}><itemref idref=\"content\"/></spine></package>"
    )
}

/// The chapter's EPUB properties: what a reader must be able to show.
fn properties(content: &str) -> String {
    let mut properties = Vec::new();
    if content.contains("<svg") {
        properties.push("svg");
    }
    if content.contains("<math") {
        properties.push("mathml");
    }
    if properties.is_empty() {
        String::new()
    } else {
        format!(" properties=\"{}\"", properties.join(" "))
    }
}

/// The book's identifier: a version-5-style UUID from the title, the
/// language and the chapter, so the same page makes the same book.
fn identifier(book: &Book<'_>, content: &str) -> String {
    let digest = sha256(format!("{}\n{}\n{content}", book.title, book.lang).as_bytes());
    let digits = hex(&digest);
    let mut scan = Scan::new(&digits);
    let time_low = scan.take_chars(8);
    let time_mid = scan.take_chars(4);
    // The version digit, written as 5.
    scan.bump();
    let time_high = scan.take_chars(3);
    // The variant digit, written as `a`.
    scan.bump();
    let clock_seq = scan.take_chars(3);
    let node = scan.take_chars(12);
    format!("{time_low}-{time_mid}-5{time_high}-a{clock_seq}-{node}")
}

/// The moment `now` the book was made, as EPUB writes it.
fn modified(now: u64) -> String {
    let date = crate::text::civil(now);
    let (year, month, day) = (date.year(), date.month(), date.day());
    let secs = now % 86_400;
    format!(
        "{year:04}-{:02}-{day:02}T{:02}:{:02}:{:02}Z",
        month.number(),
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// The zip being written, and what could not go into the book.
struct Archive {
    /// The archive.
    zip: Zip,
    /// What is left out, and why.
    problems: Vec<String>,
}

impl Archive {
    /// Adds a file; one the archive cannot hold is a problem.
    fn add(&mut self, name: &str, data: &[u8]) {
        if let Err(why) = self.zip.add(name, data) {
            self.problems
                .push(format!("epub: {name} is left out: {why}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Book, epub, namespaces, remove_all, strip_chrome};
    use crate::config::Direction;

    /// Whether `bytes` hold `part` (the archive's entries are stored, so
    /// their text stands in it as written).
    fn holds(bytes: &[u8], part: &str) -> bool {
        bytes
            .windows(part.len())
            .any(|window| window == part.as_bytes())
    }

    /// A whole book keeps EPUB's rules where the web page does not.
    ///
    /// # Panics
    ///
    /// Panics when the outline holds an empty list, when the style sheet sets
    /// `direction` or `unicode-bidi`, when code lacks `dir="ltr"`, when a link
    /// out of the book stays or one within it goes, when the figure still links
    /// to its image, or when the left-out link is not reported.
    #[test]
    fn the_book_keeps_epub_s_rules() {
        let gif = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
        let page = format!(
            "<nav class=\"toc\"><ol><li><a href=\"#a\">A</a><ol class=\"sub\"></ol></li></ol></nav>\
<main><article id=\"main\"><h2 id=\"a\">A</h2><pre><code>x</code></pre>\
<p><a href=\"SECURITY.html\">policy</a> <a href=\"#a\">up</a></p>\
<figure class=\"fig\"><a href=\"{gif}\" target=\"_blank\"><img src=\"{gif}\" alt=\"a\"></a></figure></article></main>"
        );
        let book = Book {
            title: "T",
            lang: "he",
            direction: Direction::RightToLeft,
            author: None,
            css: "",
            bases: &[],
            max_image: crate::render::MAX_EMBED,
            now: 1_700_000_000,
        };
        let (bytes, problems) = epub(&page, &book);
        // The book records the moment it is given.
        assert!(holds(
            &bytes,
            "<meta property=\"dcterms:modified\">2023-11-14T22:13:20Z</meta>"
        ));
        // An `ol` holds at least one `li`.
        assert!(holds(&bytes, "<a href=\"content.xhtml#a\">A</a></li>"));
        assert!(!holds(&bytes, "<ol class=\"sub\"></ol>"));
        // The style sheet leaves the direction to the markup.
        assert!(!holds(&bytes, "unicode-bidi"));
        assert!(!holds(&bytes, "direction:ltr"));
        assert!(holds(
            &bytes,
            "<pre dir=\"ltr\"><code dir=\"ltr\">x</code></pre>"
        ));
        // A link out of the book keeps its text; one within it stays.
        assert!(holds(&bytes, "<p>policy <a href=\"#a\">up</a></p>"));
        // The figure shows its image and links to nothing.
        assert!(holds(
            &bytes,
            "<figure class=\"fig\"><img src=\"images/img-1.gif\" alt=\"a\"/></figure>"
        ));
        assert_eq!(problems, [
            "epub: a link is left out: SECURITY.html is not in the book"
        ]);
    }

    /// Only what works on the web page alone is taken out of the book's text.
    ///
    /// # Panics
    ///
    /// Panics when a heading keeps its anchor link, when a code block keeps its
    /// copy button, when another button keeps its tag or loses its text `keep`,
    /// or when a `<button` that never closes is changed.
    #[test]
    fn only_the_page_s_chrome_is_taken_off() {
        assert_eq!(
            strip_chrome(
                "<h2>A<a class=\"anchor\" href=\"#a\">#</a></h2><pre><button class=\"copy\">Copy</button>x</pre><button type=\"button\">keep</button>"
            ),
            "<h2>A</h2><pre>x</pre>keep"
        );
        assert_eq!(strip_chrome("a<button b"), "a<button b");
    }

    /// A stretch whose close never comes is removed to the end of the text.
    ///
    /// # Panics
    ///
    /// Panics when `a[b]c[d` does not lose both `[b]` and the open `[d`, or
    /// when text with no stretch in it is changed.
    #[test]
    fn an_open_stretch_runs_to_the_end() {
        assert_eq!(remove_all("a[b]c[d", "[", "]"), "ac");
        assert_eq!(remove_all("abc", "[", "]"), "abc");
    }

    /// SVG and `MathML` get their namespaces once each, and the `xlink:` one
    /// only when the book uses its prefix.
    ///
    /// # Panics
    ///
    /// When an SVG or a math element lacks its namespace or has it twice, or
    /// the `xlink:` one is declared without an `xlink:` attribute or missing
    /// with one.
    #[test]
    fn namespaces_are_declared_once_and_xlink_when_used() {
        assert_eq!(
            namespaces("<svg><rect/></svg><math><mi>x</mi></math>"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg><math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mi>x</mi></math>"
        );
        assert_eq!(
            namespaces(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\"><a xlink:href=\"#a\">a</a></svg>"
            ),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"1\"><a xlink:href=\"#a\">a</a></svg>"
        );
    }
}
